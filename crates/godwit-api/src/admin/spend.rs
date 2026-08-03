use axum::{
    extract::{Extension, Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use godwit_auth::{jwt::Claims, rbac::Role};
use godwit_core::Capability;
use godwit_db::models::Model;
use godwit_providers::UsageReport;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

const PRICING_INPUT_PER_1K: &str = "input_per_1k";
const PRICING_OUTPUT_PER_1K: &str = "output_per_1k";

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/spend", get(get_spend))
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct SpendQuery {
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    organization_id: Option<Uuid>,
    team_id: Option<Uuid>,
    user_id: Option<Uuid>,
}

/// Applies the RBAC scoping model (spec §4/§9): `super_admin` gets whatever it asked
/// for; `org_admin` is always forced to its own org (but may still filter by team/user
/// within it); `team_admin`/`user` are always forced to their own usage only, with any
/// org/team/user filter the caller passed ignored.
fn scope_spend_query(
    claims: &Claims,
    query: SpendQuery,
) -> (Option<Uuid>, Option<Uuid>, Option<Uuid>) {
    let role = Role::from_str(&claims.role);
    match role {
        Some(Role::SuperAdmin) => (query.organization_id, query.team_id, query.user_id),
        Some(Role::OrgAdmin) => (Some(claims.organization_id), query.team_id, query.user_id),
        _ => (Some(claims.organization_id), None, Some(claims.user_id)),
    }
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct SpendRow {
    organization_id: Uuid,
    team_id: Option<Uuid>,
    user_id: Option<Uuid>,
    total_cost_usd: Decimal,
    request_count: i64,
    tokens_in: i64,
    tokens_out: i64,
}

async fn get_spend(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<SpendQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    let from = query.from;
    let to = query.to;
    let (organization_id, team_id, user_id) = scope_spend_query(&claims, query);

    let rows = sqlx::query_as::<_, SpendRow>(
        "SELECT organization_id, team_id, user_id,
                COALESCE(SUM(cost_usd), 0) AS total_cost_usd,
                COUNT(*) AS request_count,
                COALESCE(SUM(tokens_in), 0) AS tokens_in,
                COALESCE(SUM(tokens_out), 0) AS tokens_out
         FROM request_logs
         WHERE ($1::timestamptz IS NULL OR created_at >= $1)
           AND ($2::timestamptz IS NULL OR created_at <= $2)
           AND ($3::uuid IS NULL OR organization_id = $3)
           AND ($4::uuid IS NULL OR team_id = $4)
           AND ($5::uuid IS NULL OR user_id = $5)
         GROUP BY organization_id, team_id, user_id"
    )
    .bind(from)
    .bind(to)
    .bind(organization_id)
    .bind(team_id)
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| ApiError::Core(godwit_core::PasteurError::Database(e.to_string())))?;

    Ok(Json(serde_json::json!({ "data": rows })))
}

pub fn compute_cost(model: &Model, capability: Capability, usage: &UsageReport) -> Option<Decimal> {
    let pricing = model.pricing.as_object()?;
    match capability {
        Capability::Chat => {
            let input_price = pricing.get(PRICING_INPUT_PER_1K)?;
            let output_price = pricing.get(PRICING_OUTPUT_PER_1K)?;
            let input_rate = Decimal::from_str(input_price.as_str()?)
                .inspect_err(|e| tracing::warn!(%e, "malformed input_per_1k pricing"))
                .ok()?;
            let output_rate = Decimal::from_str(output_price.as_str()?)
                .inspect_err(|e| tracing::warn!(%e, "malformed output_per_1k pricing"))
                .ok()?;
            let input =
                Decimal::from(usage.prompt_tokens.unwrap_or(0)) * input_rate / Decimal::from(1000);
            let output = Decimal::from(usage.completion_tokens.unwrap_or(0)) * output_rate
                / Decimal::from(1000);
            Some(input + output)
        }
        _ => {
            tracing::warn!(
                capability = %capability,
                "cost computation not supported for capability"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn cost_computation() {
        let model = Model {
            id: uuid::Uuid::nil(),
            public_id: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            provider_profile_id: uuid::Uuid::nil(),
            provider_model_id: "gpt-4o".to_string(),
            capabilities: vec!["chat".to_string()],
            pricing: serde_json::json!({
                "input_per_1k": "0.005",
                "output_per_1k": "0.015"
            }),
            config: serde_json::json!({}),
            created_at: chrono::Utc::now(),
        };
        let usage = UsageReport {
            prompt_tokens: Some(1000),
            completion_tokens: Some(500),
            ..Default::default()
        };
        let cost = compute_cost(&model, Capability::Chat, &usage).expect("cost");
        assert_eq!(cost, dec!(0.0125));
    }

    #[test]
    fn spend_query_deserializes_with_all_fields_optional() {
        let query: SpendQuery = serde_json::from_str("{}").expect("empty query");
        assert_eq!(query.organization_id, None);
        assert_eq!(query.team_id, None);
        assert_eq!(query.user_id, None);
        assert_eq!(query.from, None);
        assert_eq!(query.to, None);
    }

    #[test]
    fn spend_scope_forces_org_admin_to_own_org() {
        let claims = godwit_auth::jwt::Claims::new(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), "org_admin");
        let requested = SpendQuery {
            from: None,
            to: None,
            organization_id: Some(uuid::Uuid::new_v4()), // attempt to look at a different org
            team_id: None,
            user_id: None,
        };
        let scoped = scope_spend_query(&claims, requested);
        assert_eq!(scoped.0, Some(claims.organization_id));
    }

    #[test]
    fn spend_scope_forces_user_to_self() {
        let claims = godwit_auth::jwt::Claims::new(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), "user");
        let requested = SpendQuery {
            from: None,
            to: None,
            organization_id: Some(uuid::Uuid::new_v4()),
            team_id: Some(uuid::Uuid::new_v4()),
            user_id: Some(uuid::Uuid::new_v4()), // attempt to look at someone else's usage
        };
        let scoped = scope_spend_query(&claims, requested);
        assert_eq!(scoped.0, Some(claims.organization_id));
        assert_eq!(scoped.1, None);
        assert_eq!(scoped.2, Some(claims.user_id));
    }

    #[test]
    fn spend_scope_leaves_super_admin_unscoped() {
        let claims = godwit_auth::jwt::Claims::new(uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), "super_admin");
        let org_id = uuid::Uuid::new_v4();
        let requested = SpendQuery { from: None, to: None, organization_id: Some(org_id), team_id: None, user_id: None };
        let scoped = scope_spend_query(&claims, requested);
        assert_eq!(scoped.0, Some(org_id));
    }
}
