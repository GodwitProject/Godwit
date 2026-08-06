use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use std::sync::Arc;

use crate::state::AppState;

#[derive(serde::Serialize)]
pub struct CircuitBreakerStatus {
    pub provider_id: String,
    pub state: String,
}

#[derive(serde::Serialize)]
pub struct CircuitBreakersResponse {
    pub breakers: Vec<CircuitBreakerStatus>,
}

pub async fn list_circuit_breakers(
    State(state): State<Arc<AppState>>,
) -> Json<CircuitBreakersResponse> {
    let breakers = state
        .circuit_breaker_registry
        .all_states()
        .into_iter()
        .map(|(provider_id, state)| CircuitBreakerStatus {
            provider_id,
            state: match state {
                crate::circuit_breaker::CircuitState::Closed => "closed".to_string(),
                crate::circuit_breaker::CircuitState::Open => "open".to_string(),
                crate::circuit_breaker::CircuitState::HalfOpen => "half_open".to_string(),
            },
        })
        .collect();

    Json(CircuitBreakersResponse { breakers })
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/circuit-breakers", get(list_circuit_breakers))
}
