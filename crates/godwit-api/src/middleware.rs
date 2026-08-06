use std::sync::Arc;

use axum::{
    body::{Body, Bytes},
    extract::{Extension, Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use godwit_auth::{api_keys::verify_key, jwt::verify};
use godwit_db::models::ApiKey;

use crate::{error::ApiError, state::AppState};

pub fn extract_token(header: &str) -> Option<&str> {
    header.strip_prefix("Bearer ")
}

pub async fn api_key_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(extract_token)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Fast path: cache lookup by raw key.
    if let Some(key) = state.api_key_cache.get(&auth.to_string()).await {
        if !key.disabled
            && key
                .expires_at
                .map(|e| e > chrono::Utc::now())
                .unwrap_or(true)
        {
            req.extensions_mut().insert(key);
            return Ok(next.run(req).await);
        }
    }

    // Fallback: database lookup by prefix.
    let prefix = godwit_auth::api_keys::extract_prefix(auth);
    let candidates = state
        .api_key_repo
        .get_by_prefix(&prefix)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let key = candidates
        .into_iter()
        .find(|k| verify_key(auth, &k.key_hash))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if key.disabled
        || key
            .expires_at
            .map(|e| e < chrono::Utc::now())
            .unwrap_or(false)
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    state
        .api_key_cache
        .insert(auth.to_string(), key.clone())
        .await;
    req.extensions_mut().insert(key);
    Ok(next.run(req).await)
}

pub async fn jwt_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(extract_token)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let claims =
        verify(&state.config.auth.jwt_secret, auth).map_err(|_| StatusCode::UNAUTHORIZED)?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

async fn extract_model_from_body(body: Bytes) -> Option<String> {
    serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("model")?.as_str().map(|s| s.to_string()))
}

fn is_model_allowed(api_key: &ApiKey, model: &str) -> bool {
    api_key.allowed_models.is_empty() || api_key.allowed_models.iter().any(|m| m == model)
}

pub async fn model_scope(
    Extension(api_key): Extension<ApiKey>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let (parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    if let Some(model) = extract_model_from_body(bytes.clone()).await {
        if !is_model_allowed(&api_key, &model) {
            return Err(ApiError::Forbidden);
        }
    }

    let req = Request::from_parts(parts, Body::from(bytes));
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_bearer_token() {
        assert_eq!(
            extract_token("Bearer sk-godwit-abc123"),
            Some("sk-godwit-abc123")
        );
        assert_eq!(extract_token("Basic abc"), None);
    }

    #[tokio::test]
    async fn extract_model_from_json_body() {
        let body = Bytes::from(r#"{"model":"gpt-4o","messages":[]}"#);
        assert_eq!(extract_model_from_body(body).await, Some("gpt-4o".into()));
    }

    #[tokio::test]
    async fn extract_model_returns_none_when_missing() {
        let body = Bytes::from(r#"{"messages":[]}"#);
        assert_eq!(extract_model_from_body(body).await, None);
    }

    #[tokio::test]
    async fn extract_model_returns_none_for_non_json() {
        let body = Bytes::from("not json");
        assert_eq!(extract_model_from_body(body).await, None);
    }

    fn api_key_with_allowed_models(models: &[String]) -> ApiKey {
        ApiKey {
            id: uuid::Uuid::new_v4(),
            user_id: uuid::Uuid::new_v4(),
            team_id: None,
            organization_id: uuid::Uuid::new_v4(),
            name: "test".to_string(),
            key_prefix: "prefix".to_string(),
            key_hash: "hash".to_string(),
            scopes: vec!["chat".to_string()],
            allowed_models: models.to_vec(),
            budget_limit_usd: None,
            budget_spent_usd: rust_decimal::Decimal::ZERO,
            rate_limit_requests_per_minute: None,
            rate_limit_tokens_per_minute: None,
            expires_at: None,
            disabled: false,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn empty_allowed_models_allows_anything() {
        let key = api_key_with_allowed_models(&[]);
        assert!(is_model_allowed(&key, "gpt-4o"));
        assert!(is_model_allowed(&key, "claude-sonnet"));
    }

    #[test]
    fn allowed_models_blocks_missing_model() {
        let key = api_key_with_allowed_models(&["gpt-4o".to_string()]);
        assert!(is_model_allowed(&key, "gpt-4o"));
        assert!(!is_model_allowed(&key, "claude-sonnet"));
    }
}
