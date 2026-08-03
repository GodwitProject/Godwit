pub mod api_keys;
pub mod auth;
pub mod models;
pub mod organizations;
pub mod provider_profiles;
pub mod spend;
pub mod teams;
pub mod users;

use crate::{error::ApiError, middleware::jwt_auth, state::AppState};
use axum::{middleware, Router};
use godwit_auth::{jwt::Claims, rbac::Role};
use std::sync::Arc;

/// The instance-wide catalog (`models`, `provider_profiles`) is shared infrastructure:
/// only `super_admin` may manage it. Shared by `admin::models` and
/// `admin::provider_profiles`, which both gate every handler on it.
pub(crate) fn require_super_admin(claims: &Claims) -> Result<(), ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    if role != Role::SuperAdmin {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let protected = Router::new()
        .nest("/users", users::router())
        .nest("/organizations", organizations::router())
        .nest("/teams", teams::router())
        .nest("/api-keys", api_keys::router())
        // `models::router()` and `provider_profiles::router()` already register their full
        // intended paths ("/models", "/models/:id", "/provider-profiles", ...), so they are
        // merged, not nested: nesting under "/models" produced "/api/v1/models/models".
        .merge(models::router())
        .merge(provider_profiles::router())
        .nest("/spend", spend::router())
        .route_layer(middleware::from_fn_with_state(state, jwt_auth));

    Router::new().merge(auth::router()).merge(protected)
}
