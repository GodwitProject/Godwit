use axum::{
    extract::{Extension, State},
    Json,
};
use chrono::{Duration, Utc};
use godwit_auth::{
    api_keys::{hash_password, verify_password},
    jwt::Claims,
    refresh_tokens::{generate_refresh_token, hash_refresh_token},
    rbac::Role,
};
use godwit_core::PasswordPolicy;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{admin::users, error::ApiError, mail, state::AppState};

mod common {
    use std::collections::HashSet;
    use std::sync::OnceLock;
    static LIST: &str = include_str!("../../assets/common_passwords.txt");
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    pub fn lookup(pw: &str) -> bool {
        SET.get_or_init(|| LIST.lines().map(|l| l.trim().to_string()).collect())
            .contains(&pw.to_ascii_lowercase())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PolicyError {
    TooShort,
    NeedsUpper,
    NeedsLower,
    NeedsDigit,
    NeedsSymbol,
    CommonPassword,
    Reused,
}

pub fn validate_password(
    policy: &PasswordPolicy,
    password: &str,
    history: &[String],
) -> Result<(), PolicyError> {
    if policy.block_common && common::lookup(password) {
        return Err(PolicyError::CommonPassword);
    }
    if (password.chars().count() as u32) < policy.min_length {
        return Err(PolicyError::TooShort);
    }
    if policy.require_upper && !password.chars().any(|c| c.is_uppercase()) {
        return Err(PolicyError::NeedsUpper);
    }
    if policy.require_lower && !password.chars().any(|c| c.is_lowercase()) {
        return Err(PolicyError::NeedsLower);
    }
    if policy.require_digit && !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(PolicyError::NeedsDigit);
    }
    if policy.require_symbol && !password.chars().any(|c| !c.is_alphanumeric()) {
        return Err(PolicyError::NeedsSymbol);
    }
    for h in history {
        if verify_password(password, h) {
            return Err(PolicyError::Reused);
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub enum PasswordState {
    Valid,
    Expired,
    ForcedChange,
}

pub fn password_state(
    must_change: bool,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> PasswordState {
    if must_change {
        return PasswordState::ForcedChange;
    }
    match expires_at {
        Some(exp) if exp < chrono::Utc::now() => PasswordState::Expired,
        _ => PasswordState::Valid,
    }
}

async fn do_password_change(
    state: &AppState,
    user_id: Uuid,
    new_password: &str,
) -> Result<(), ApiError> {
    let policy = &state.config.auth.password_policy;
    let history = state
        .password_history_repo
        .get_last_n(user_id, policy.max_reuse as i64)
        .await
        .map_err(ApiError::Core)?;
    validate_password(policy, new_password, &history)
        .map_err(|e| ApiError::BadRequest(format!("{e:?}")))?;
    let hash = hash_password(new_password);
    let expires_at = if policy.days_to_expire > 0 {
        Some(Utc::now() + Duration::days(policy.days_to_expire as i64))
    } else {
        None
    };
    state
        .user_repo
        .update_password(user_id, &hash, expires_at)
        .await
        .map_err(ApiError::Core)?;
    state
        .password_history_repo
        .push(user_id, &hash)
        .await
        .map_err(ApiError::Core)?;
    state
        .password_history_repo
        .purge_older_than(user_id, policy.max_reuse as i64)
        .await
        .map_err(ApiError::Core)?;
    state
        .refresh_token_repo
        .delete_all_for_user(user_id)
        .await
        .map_err(ApiError::Core)?;
    Ok(())
}

#[derive(Deserialize)]
pub struct ChangePasswordReq {
    current_password: String,
    new_password: String,
}

#[derive(Deserialize)]
pub struct ChangeRequiredReq {
    new_password: String,
}

#[derive(Deserialize)]
pub struct AdminResetReq {
    user_id: Uuid,
    new_password: String,
}

#[derive(Deserialize)]
pub struct ForgotPasswordReq {
    email: String,
}

#[derive(Deserialize)]
pub struct ResetPasswordReq {
    token: String,
    new_password: String,
}

pub async fn change_password(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<ChangePasswordReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = claims.user_id;
    let user = state.user_repo.get_by_id(user_id).await.map_err(ApiError::Core)?;
    let current_hash = user
        .password_hash
        .as_deref()
        .ok_or(ApiError::BadRequest("account has no password set".to_string()))?;
    if !verify_password(&req.current_password, current_hash) {
        return Err(ApiError::Unauthorized);
    }
    do_password_change(&state, user_id, &req.new_password).await?;
    Ok(Json(serde_json::json!({ "changed": true })))
}

pub async fn change_required(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<ChangeRequiredReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id = claims.user_id;
    let user = state.user_repo.get_by_id(user_id).await.map_err(ApiError::Core)?;
    let expired = user
        .password_expires_at
        .map(|exp| exp < Utc::now())
        .unwrap_or(false);
    if !user.must_change_password && !expired {
        return Err(ApiError::Forbidden);
    }
    do_password_change(&state, user_id, &req.new_password).await?;
    Ok(Json(serde_json::json!({ "changed": true })))
}

pub async fn admin_reset_password(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<AdminResetReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller_role = users::require_role(&claims, &[Role::SuperAdmin, Role::OrgAdmin])?;
    if claims.user_id == req.user_id {
        return Err(ApiError::BadRequest("cannot reset your own password".to_string()));
    }
    let target = state.user_repo.get_by_id(req.user_id).await.map_err(ApiError::Core)?;
    users::check_same_org(caller_role, &claims, target.organization_id)?;
    users::check_not_acting_on_super_admin(caller_role, &target.role)?;
    do_password_change(&state, target.id, &req.new_password).await?;
    state
        .user_repo
        .set_must_change(target.id, true)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "reset": true })))
}

pub async fn forgot_password(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ForgotPasswordReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if let Ok(user) = state.user_repo.get_by_email(&req.email).await {
        let (plaintext, hash) = generate_refresh_token();
        let _ = state
            .password_reset_token_repo
            .create(user.id, &hash, std::time::Duration::from_secs(1800))
            .await;
        if let Some(mailer) = &state.mailer {
            if let Some(mail_config) = &state.config.auth.mail {
                let (html, text) = mail::render_reset_email(&mail_config.app_url, &plaintext);
                let _ = mailer
                    .send(&req.email, "Reset your password", &html, &text)
                    .await;
            }
        }
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn reset_password(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResetPasswordReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let hash = hash_refresh_token(&req.token);
    let token = state
        .password_reset_token_repo
        .get_by_hash(&hash)
        .await
        .map_err(|_| ApiError::BadRequest("invalid or expired reset token".to_string()))?;
    if token.used_at.is_some() {
        return Err(ApiError::BadRequest("reset token already used".to_string()));
    }
    if token.expires_at < Utc::now() {
        return Err(ApiError::BadRequest("reset token expired".to_string()));
    }
    do_password_change(&state, token.user_id, &req.new_password).await?;
    state
        .password_reset_token_repo
        .mark_used(token.id)
        .await
        .map_err(ApiError::Core)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pol() -> PasswordPolicy {
        PasswordPolicy {
            min_length: 8,
            require_upper: true,
            require_lower: true,
            require_digit: true,
            require_symbol: true,
            max_reuse: 3,
            days_to_expire: 90,
            block_common: true,
        }
    }

    #[test]
    fn too_short() {
        assert_eq!(validate_password(&pol(), "Ab1!", &[]), Err(PolicyError::TooShort));
    }

    #[test]
    fn missing_symbol() {
        assert_eq!(validate_password(&pol(), "Abcdefg1", &[]), Err(PolicyError::NeedsSymbol));
    }

    #[test]
    fn common_blocked() {
        assert_eq!(validate_password(&pol(), "password", &[]), Err(PolicyError::CommonPassword));
    }

    #[test]
    fn reused_blocked() {
        let h = hash_password("CorrectHorse1!");
        assert_eq!(validate_password(&pol(), "CorrectHorse1!", &[h]), Err(PolicyError::Reused));
    }

    #[test]
    fn valid_pass() {
        assert_eq!(validate_password(&pol(), "CorrectHorse1!", &[]), Ok(()));
    }

    #[test]
    fn state_transitions() {
        use chrono::Utc;
        assert_eq!(
            password_state(false, Some(Utc::now() - chrono::Duration::days(1))),
            PasswordState::Expired
        );
        assert_eq!(
            password_state(true, Some(Utc::now() + chrono::Duration::days(1))),
            PasswordState::ForcedChange
        );
        assert_eq!(password_state(false, None), PasswordState::Valid);
    }
}
