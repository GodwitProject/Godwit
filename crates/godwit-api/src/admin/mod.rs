pub mod api_keys;
pub mod auth;
pub mod models;
pub mod organizations;
pub mod provider_profiles;
pub mod spend;
pub mod spend_logs;
pub mod stats;
pub mod teams;
pub mod users;

use crate::{error::ApiError, middleware::jwt_auth, state::AppState};
use axum::{middleware, Router};
use godwit_auth::{jwt::Claims, rbac::Role};
use std::sync::Arc;

/// Only `super_admin` may manage instance-wide and cross-organization resources.
/// Shared by `admin::models`, `admin::provider_profiles`, and `admin::organizations`,
/// which all gate every handler on it.
pub(crate) fn require_super_admin(claims: &Claims) -> Result<(), ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if role != Role::SuperAdmin {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let protected = Router::new()
        // `api_keys::router()`, `models::router()`, `provider_profiles::router()`,
        // `organizations::router()`, `teams::router()`, `users::router()`, and
        // `spend::router()` already register their full intended paths ("/api-keys",
        // "/models", "/models/:id", "/provider-profiles", "/organizations", "/teams",
        // "/users", "/spend", ...), so they are merged, not nested: nesting under
        // "/organizations" produced "/api/v1/organizations/organizations".
        .merge(api_keys::router())
        .merge(models::router())
        .merge(provider_profiles::router())
        .merge(organizations::router())
        .merge(teams::router())
        .merge(users::router())
        .merge(spend::router())
        .merge(spend_logs::router())
        .merge(stats::router())
        .route_layer(middleware::from_fn_with_state(state, jwt_auth));

    Router::new().merge(auth::router()).merge(protected)
}
