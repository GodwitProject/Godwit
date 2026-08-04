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
    /// When present, switches the response to a day-bucketed `{date, cost}` series (see
    /// `fetch_daily_spend_rows`) instead of the org/team/user-grouped rows below, and — if
    /// `from` wasn't also given — sets it to `days` ago. This is what the dashboard's spend
    /// graph sends; the resource-level Spend page leaves it unset to get the grouped rows.
    days: Option<i64>,
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

/// Runs the actual spend aggregation query over `request_logs`, grouped by
/// organization/team/user, with each of `from`/`to`/`organization_id`/`team_id`/`user_id`
/// applied as an optional filter (a `None` leaves that dimension unfiltered). Extracted
/// out of `get_spend` so the query itself — not just the RBAC scoping around it — is
/// directly exercisable by a DB-backed test.
async fn fetch_spend_rows(
    pool: &sqlx::PgPool,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    organization_id: Option<Uuid>,
    team_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> Result<Vec<SpendRow>, sqlx::Error> {
    sqlx::query_as::<_, SpendRow>(
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
         GROUP BY organization_id, team_id, user_id",
    )
    .bind(from)
    .bind(to)
    .bind(organization_id)
    .bind(team_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
}

async fn get_spend(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<SpendQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    let days = query.days;
    let from = query
        .from
        .or_else(|| days.map(|d| Utc::now() - chrono::Duration::days(d)));
    let to = query.to;
    let (organization_id, team_id, user_id) = scope_spend_query(&claims, query);

    if days.is_some() {
        let rows = fetch_daily_spend_rows(&state.pool, from, to, organization_id, team_id, user_id)
            .await
            .map_err(|e| ApiError::Core(godwit_core::PasteurError::Database(e.to_string())))?;
        return Ok(Json(serde_json::json!({ "data": rows })));
    }

    let rows = fetch_spend_rows(&state.pool, from, to, organization_id, team_id, user_id)
        .await
        .map_err(|e| ApiError::Core(godwit_core::PasteurError::Database(e.to_string())))?;

    Ok(Json(serde_json::json!({ "data": rows })))
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct DailySpendRow {
    date: chrono::NaiveDate,
    cost: Decimal,
}

/// Day-bucketed spend series for the dashboard's line graph, which plots a `{date, cost}`
/// point per day rather than the org/team/user breakdown `fetch_spend_rows` returns.
async fn fetch_daily_spend_rows(
    pool: &sqlx::PgPool,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    organization_id: Option<Uuid>,
    team_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> Result<Vec<DailySpendRow>, sqlx::Error> {
    sqlx::query_as::<_, DailySpendRow>(
        "SELECT date_trunc('day', created_at)::date AS date,
                COALESCE(SUM(cost_usd), 0) AS cost
         FROM request_logs
         WHERE ($1::timestamptz IS NULL OR created_at >= $1)
           AND ($2::timestamptz IS NULL OR created_at <= $2)
           AND ($3::uuid IS NULL OR organization_id = $3)
           AND ($4::uuid IS NULL OR team_id = $4)
           AND ($5::uuid IS NULL OR user_id = $5)
         GROUP BY date_trunc('day', created_at)::date
         ORDER BY date",
    )
    .bind(from)
    .bind(to)
    .bind(organization_id)
    .bind(team_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
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

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn spend_aggregation_sums_matching_rows_and_respects_filters(pool: sqlx::PgPool) {
        let org_a = Uuid::new_v4();
        let org_b = Uuid::new_v4();
        sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, 'org-a'), ($2, 'org-b')")
            .bind(org_a)
            .bind(org_b)
            .execute(&pool)
            .await
            .expect("insert organizations");

        let user_a1 = Uuid::new_v4();
        let user_a2 = Uuid::new_v4();
        let user_b1 = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO users (id, organization_id, email, role) VALUES
                ($1, $4, 'a1@test.example', 'user'),
                ($2, $4, 'a2@test.example', 'user'),
                ($3, $5, 'b1@test.example', 'user')",
        )
        .bind(user_a1)
        .bind(user_a2)
        .bind(user_b1)
        .bind(org_a)
        .bind(org_b)
        .execute(&pool)
        .await
        .expect("insert users");

        // Two rows for (org_a, user_a1) that should be summed together, plus a row for a
        // different user in the same org and a row in a different org entirely, both of
        // which the org_a/user_a1 filter below must exclude.
        sqlx::query(
            "INSERT INTO request_logs
                (organization_id, user_id, model, provider, provider_model_id,
                 tokens_in, tokens_out, cost_usd, duration_ms, status)
             VALUES
                ($1, $2, 'gpt-4o', 'openai', 'gpt-4o', 100, 50, 1.50, 10, 'success'),
                ($1, $2, 'gpt-4o', 'openai', 'gpt-4o', 200, 100, 2.50, 20, 'success'),
                ($1, $3, 'gpt-4o', 'openai', 'gpt-4o', 999, 999, 9.99, 30, 'success'),
                ($4, $5, 'gpt-4o', 'openai', 'gpt-4o', 111, 111, 5.00, 40, 'success')",
        )
        .bind(org_a)
        .bind(user_a1)
        .bind(user_a2)
        .bind(org_b)
        .bind(user_b1)
        .execute(&pool)
        .await
        .expect("insert request_logs");

        let rows = fetch_spend_rows(&pool, None, None, Some(org_a), None, Some(user_a1))
            .await
            .expect("fetch spend rows");

        assert_eq!(rows.len(), 1, "expected exactly one aggregated row: {rows:?}");
        let row = &rows[0];
        assert_eq!(row.organization_id, org_a);
        assert_eq!(row.user_id, Some(user_a1));
        assert_eq!(row.team_id, None);
        assert_eq!(row.total_cost_usd, dec!(4.00));
        assert_eq!(row.request_count, 2);
        assert_eq!(row.tokens_in, 300);
        assert_eq!(row.tokens_out, 150);
    }
}
