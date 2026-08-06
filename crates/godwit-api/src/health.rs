use axum::{
    extract::State,
    http::StatusCode,
    Json, Router,
    routing::get,
};
use std::sync::Arc;
use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health_check))
        .route("/health/ready", get(health_ready))
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

async fn health_ready(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Check DB connectivity
    sqlx::query("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "status": "not_ready",
                    "reason": format!("database connection failed: {}", e)
                }))
            )
        })?;

    // Check at least one model exists
    let model_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM models")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "status": "not_ready",
                    "reason": format!("failed to query models: {}", e)
                }))
            )
        })?;

    if model_count == 0 {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "not_ready",
                "reason": "no models configured in catalog"
            }))
        ));
    }

    Ok(Json(serde_json::json!({"status": "ok"})))
}
