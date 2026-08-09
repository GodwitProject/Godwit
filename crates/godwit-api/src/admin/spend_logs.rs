use axum::{
    extract::{Extension, Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use godwit_auth::{jwt::Claims, rbac::Role};
use rust_decimal::Decimal;
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/spend/logs", get(get_spend_logs))
}

#[derive(Debug, Deserialize)]
struct SpendLogsQuery {
    api_key_id: Option<Uuid>,
    model: Option<String>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct SpendLogEntry {
    id: Uuid,
    api_key_id: Option<Uuid>,
    model: String,
    provider: String,
    capability: String,
    tokens_in: Option<i32>,
    tokens_out: Option<i32>,
    duration_ms: i32,
    streamed: bool,
    cost_usd: Option<Decimal>,
    status: String,
    created_at: chrono::DateTime<Utc>,
}

async fn fetch_spend_logs(
    pool: &PgPool,
    api_key_id: Option<Uuid>,
    model: Option<String>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: i64,
    offset: i64,
) -> Result<Vec<SpendLogEntry>, sqlx::Error> {
    sqlx::query_as::<_, SpendLogEntry>(
        "SELECT id, api_key_id, model, provider, capability, tokens_in, tokens_out, duration_ms, streamed, cost_usd, status, created_at
         FROM request_logs
         WHERE ($1::uuid IS NULL OR api_key_id = $1)
           AND ($2::text IS NULL OR model = $2)
           AND ($3::timestamptz IS NULL OR created_at >= $3)
           AND ($4::timestamptz IS NULL OR created_at <= $4)
         ORDER BY created_at DESC
         LIMIT $5 OFFSET $6",
    )
    .bind(api_key_id)
    .bind(model)
    .bind(from)
    .bind(to)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

async fn get_spend_logs(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<SpendLogsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let role = Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    
    // RBAC: non-super-admins can only see their own logs
    let api_key_id = match role {
        Role::SuperAdmin => query.api_key_id,
        _ => None,
    };
    
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);
    
    let logs = fetch_spend_logs(
        &state.pool,
        api_key_id,
        query.model,
        query.from,
        query.to,
        limit,
        offset,
    )
    .await
    .map_err(|e| ApiError::Core(godwit_core::PasteurError::Database(e.to_string())))?;
    
    Ok(Json(serde_json::json!({
        "data": logs,
        "limit": limit,
        "offset": offset
    })))
}
