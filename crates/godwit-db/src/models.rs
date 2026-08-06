use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub rate_limit_requests_per_minute: Option<i32>,
    pub rate_limit_tokens_per_minute: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub email: String,
    pub name: Option<String>,
    pub role: String,
    pub sso_provider: Option<String>,
    pub sso_subject: Option<String>,
    // Never serialized into API responses — this is an Argon2 hash. The field is still
    // populated via `sqlx::FromRow` (password verification reads it as a Rust struct
    // field), only its presence in JSON *output* is suppressed.
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum UserRole {
    SuperAdmin,
    OrgAdmin,
    TeamAdmin,
    User,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::SuperAdmin => "super_admin",
            UserRole::OrgAdmin => "org_admin",
            UserRole::TeamAdmin => "team_admin",
            UserRole::User => "user",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "super_admin" => Some(UserRole::SuperAdmin),
            "org_admin" => Some(UserRole::OrgAdmin),
            "team_admin" => Some(UserRole::TeamAdmin),
            "user" => Some(UserRole::User),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub team_id: Option<Uuid>,
    pub organization_id: Uuid,
    pub name: String,
    pub key_prefix: String,
    // Never serialized into API responses — this is the hashed API key. Still populated
    // via `sqlx::FromRow` for lookup/verification; only JSON output is suppressed.
    #[serde(skip_serializing)]
    pub key_hash: String,
    pub scopes: Vec<String>,
    pub allowed_models: Vec<String>,
    pub budget_limit_usd: Option<rust_decimal::Decimal>,
    pub budget_spent_usd: rust_decimal::Decimal,
    pub rate_limit_requests_per_minute: Option<i32>,
    pub rate_limit_tokens_per_minute: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
    pub disabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Model {
    pub id: Uuid,
    pub public_id: String,
    // Kept for backward compatibility during the transition to provider profiles.
    // Will be removed once all code uses `provider_profile_id`.
    pub provider: String,
    pub provider_profile_id: Uuid,
    pub provider_model_id: String,
    pub capabilities: Vec<String>,
    pub pricing: serde_json::Value,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl Model {
    pub fn has_capability(&self, capability: godwit_core::Capability) -> bool {
        self.capabilities.iter().any(|c| c == capability.as_str())
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub id: Uuid,
    pub name: String,
    pub protocol: String,
    pub base_url: Option<String>,
    pub allow_wildcard: bool,
    pub auth: serde_json::Value,
    pub config: serde_json::Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Team {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub budget_usd: Option<rust_decimal::Decimal>,
    pub max_budget_usd: Option<rust_decimal::Decimal>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TeamMembership {
    pub user_id: Uuid,
    pub team_id: Uuid,
    pub role: String,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    // Not currently returned by any handler, but a `json!({"data": token})` away from
    // leaking a valid refresh-token hash — suppressed preventively, same as
    // `User::password_hash` and `ApiKey::key_hash`.
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
