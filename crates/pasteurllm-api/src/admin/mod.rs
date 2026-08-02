pub mod api_keys;
pub mod auth;
pub mod models;
pub mod organizations;
pub mod spend;
pub mod teams;
pub mod users;

use axum::{middleware, Router};
use std::sync::Arc;
use crate::{middleware::jwt_auth, state::AppState};

pub fn router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let protected = Router::new()
        .nest("/users", users::router())
        .nest("/organizations", organizations::router())
        .nest("/teams", teams::router())
        .nest("/api-keys", api_keys::router())
        .nest("/models", models::router())
        .nest("/spend", spend::router())
        .route_layer(middleware::from_fn_with_state(state, jwt_auth));

    Router::new()
        .merge(auth::router())
        .merge(protected)
}
