use axum::{
    extract::{Extension, Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use godwit_auth::{jwt::Claims, rbac::Role};
use rust_decimal::Decimal;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/spend/tags", get(get_spend_tags))
}

#[derive(Debug, Clone, Deserialize)]
struct SpendTagsQuery {
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    organization_id: Option<Uuid>,
    tag: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
struct TeamSpend {
    team_id: Option<Uuid>,
    spend_usd: Decimal,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
struct ApiKeySpend {
    api_key_id: Option<Uuid>,
    spend_usd: Decimal,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
struct CustomTagSpend {
    tag: String,
    spend_usd: Decimal,
}

#[derive(Debug, serde::Serialize)]
struct SpendTagsResponse {
    by_team: Vec<TeamSpend>,
    by_api_key: Vec<ApiKeySpend>,
    by_custom_tag: Vec<CustomTagSpend>,
}

/// Applies RBAC scoping: super_admin gets what it asked for,
/// org_admin is forced to own org, team_admin/user see only their own data.
fn scope_spend_tags_query(
    claims: &Claims,
    query: &SpendTagsQuery,
) -> (Option<Uuid>, Option<Uuid>) {
    let role = Role::from_str(&claims.role);
    match role {
        Some(Role::SuperAdmin) => (query.organization_id, None),
        Some(Role::OrgAdmin) => (Some(claims.organization_id), None),
        _ => (Some(claims.organization_id), Some(claims.user_id)),
    }
}

async fn fetch_team_spend(
    pool: &sqlx::PgPool,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    organization_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> Result<Vec<TeamSpend>, sqlx::Error> {
    sqlx::query_as::<_, TeamSpend>(
        "SELECT team_id, COALESCE(SUM(cost_usd), 0) AS spend_usd
         FROM request_logs
         WHERE ($1::timestamptz IS NULL OR created_at >= $1)
           AND ($2::timestamptz IS NULL OR created_at <= $2)
           AND ($3::uuid IS NULL OR organization_id = $3)
           AND ($4::uuid IS NULL OR user_id = $4)
         GROUP BY team_id
         ORDER BY spend_usd DESC",
    )
    .bind(from)
    .bind(to)
    .bind(organization_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
}

async fn fetch_api_key_spend(
    pool: &sqlx::PgPool,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    organization_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> Result<Vec<ApiKeySpend>, sqlx::Error> {
    sqlx::query_as::<_, ApiKeySpend>(
        "SELECT api_key_id, COALESCE(SUM(cost_usd), 0) AS spend_usd
         FROM request_logs
         WHERE ($1::timestamptz IS NULL OR created_at >= $1)
           AND ($2::timestamptz IS NULL OR created_at <= $2)
           AND ($3::uuid IS NULL OR organization_id = $3)
           AND ($4::uuid IS NULL OR user_id = $4)
         GROUP BY api_key_id
         ORDER BY spend_usd DESC",
    )
    .bind(from)
    .bind(to)
    .bind(organization_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
}

async fn fetch_custom_tag_spend(
    pool: &sqlx::PgPool,
    tag: Option<&str>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    organization_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> Result<Vec<CustomTagSpend>, sqlx::Error> {
    let query = if tag.is_some() {
        "SELECT UNNEST(tags) AS tag, COALESCE(SUM(cost_usd), 0) AS spend_usd
         FROM request_logs
         WHERE $1 = ANY(tags)
           AND ($2::timestamptz IS NULL OR created_at >= $2)
           AND ($3::timestamptz IS NULL OR created_at <= $3)
           AND ($4::uuid IS NULL OR organization_id = $4)
           AND ($5::uuid IS NULL OR user_id = $5)
         GROUP BY tag
         ORDER BY spend_usd DESC"
    } else {
        "SELECT UNNEST(tags) AS tag, COALESCE(SUM(cost_usd), 0) AS spend_usd
         FROM request_logs
         WHERE tags IS NOT NULL AND array_length(tags, 1) > 0
           AND ($1::timestamptz IS NULL OR created_at >= $1)
           AND ($2::timestamptz IS NULL OR created_at <= $2)
           AND ($3::uuid IS NULL OR organization_id = $3)
           AND ($4::uuid IS NULL OR user_id = $4)
         GROUP BY tag
         ORDER BY spend_usd DESC"
    };

    if let Some(t) = tag {
        sqlx::query_as::<_, CustomTagSpend>(query)
            .bind(t)
            .bind(from)
            .bind(to)
            .bind(organization_id)
            .bind(user_id)
            .fetch_all(pool)
            .await
    } else {
        sqlx::query_as::<_, CustomTagSpend>(query)
            .bind(from)
            .bind(to)
            .bind(organization_id)
            .bind(user_id)
            .fetch_all(pool)
            .await
    }
}

async fn get_spend_tags(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<SpendTagsQuery>,
) -> Result<Json<SpendTagsResponse>, ApiError> {
    Role::from_str(&claims.role).ok_or(ApiError::Forbidden)?;
    
    let (organization_id, user_id) = scope_spend_tags_query(&claims, &query);
    
    let by_team = fetch_team_spend(&state.pool, query.from, query.to, organization_id, user_id)
        .await
        .map_err(|e| ApiError::Core(godwit_core::PasteurError::Database(e.to_string())))?;
    
    let by_api_key = fetch_api_key_spend(&state.pool, query.from, query.to, organization_id, user_id)
        .await
        .map_err(|e| ApiError::Core(godwit_core::PasteurError::Database(e.to_string())))?;
    
    let by_custom_tag = fetch_custom_tag_spend(
        &state.pool,
        query.tag.as_deref(),
        query.from,
        query.to,
        organization_id,
        user_id,
    )
    .await
    .map_err(|e| ApiError::Core(godwit_core::PasteurError::Database(e.to_string())))?;
    
    Ok(Json(SpendTagsResponse { by_team, by_api_key, by_custom_tag }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn spend_tags_response_serializes_correctly() {
        let response = SpendTagsResponse {
            by_team: vec![
                TeamSpend {
                    team_id: Some(Uuid::nil()),
                    spend_usd: dec!(12.34),
                },
                TeamSpend {
                    team_id: None,
                    spend_usd: dec!(5.67),
                },
            ],
            by_api_key: vec![
                ApiKeySpend {
                    api_key_id: Some(Uuid::nil()),
                    spend_usd: dec!(56.78),
                },
            ],
            by_custom_tag: vec![],
        };
        
        let json = serde_json::to_string(&response).expect("serialize");
        assert!(json.contains("by_team"));
        assert!(json.contains("by_api_key"));
        assert!(json.contains("12.34"));
        assert!(json.contains("56.78"));
    }

    #[test]
    fn spend_tags_scope_forces_org_admin_to_own_org() {
        let claims = godwit_auth::jwt::Claims::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            "org_admin"
        );
        let requested = SpendTagsQuery {
            from: None,
            to: None,
            organization_id: Some(uuid::Uuid::new_v4()),
            tag: None,
        };
        let scoped = scope_spend_tags_query(&claims, &requested);
        assert_eq!(scoped.0, Some(claims.organization_id));
    }

    #[test]
    fn spend_tags_scope_forces_user_to_self() {
        let claims = godwit_auth::jwt::Claims::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            "user"
        );
        let requested = SpendTagsQuery {
            from: None,
            to: None,
            organization_id: Some(uuid::Uuid::new_v4()),
            tag: None,
        };
        let scoped = scope_spend_tags_query(&claims, &requested);
        assert_eq!(scoped.0, Some(claims.organization_id));
        assert_eq!(scoped.1, Some(claims.user_id));
    }

    #[test]
    fn spend_tags_scope_leaves_super_admin_unscoped() {
        let claims = godwit_auth::jwt::Claims::new(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            "super_admin"
        );
        let org_id = uuid::Uuid::new_v4();
        let requested = SpendTagsQuery {
            from: None,
            to: None,
            organization_id: Some(org_id),
            tag: None,
        };
        let scoped = scope_spend_tags_query(&claims, &requested);
        assert_eq!(scoped.0, Some(org_id));
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn spend_tags_by_team_aggregates_correctly(pool: sqlx::PgPool) {
        let org = Uuid::new_v4();
        sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, 'test-org')")
            .bind(org)
            .execute(&pool)
            .await
            .expect("insert org");

        let team_a = Uuid::new_v4();
        let team_b = Uuid::new_v4();
        sqlx::query("INSERT INTO teams (id, organization_id, name) VALUES ($1, $2, 'team-a'), ($3, $2, 'team-b')")
            .bind(team_a)
            .bind(org)
            .bind(team_b)
            .execute(&pool)
            .await
            .expect("insert teams");

        let user = Uuid::new_v4();
        sqlx::query("INSERT INTO users (id, organization_id, email, role) VALUES ($1, $2, 'test@example.com', 'user')")
            .bind(user)
            .bind(org)
            .execute(&pool)
            .await
            .expect("insert user");

        sqlx::query(
            "INSERT INTO request_logs
             (organization_id, team_id, user_id, model, provider, provider_model_id,
              tokens_in, tokens_out, cost_usd, duration_ms, streamed, status)
             VALUES
             ($1, $2, $4, 'gpt-4o', 'openai', 'gpt-4o', 100, 50, 1.50, 10, false, 'success'),
             ($1, $2, $4, 'gpt-4o', 'openai', 'gpt-4o', 200, 100, 2.50, 20, false, 'success'),
             ($1, $3, $4, 'gpt-4o', 'openai', 'gpt-4o', 300, 150, 3.00, 30, false, 'success')",
        )
        .bind(org)
        .bind(team_a)
        .bind(team_b)
        .bind(user)
        .execute(&pool)
        .await
        .expect("insert request_logs");

        let result = fetch_team_spend(&pool, None, None, Some(org), None)
            .await
            .expect("fetch team spend");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].team_id, Some(team_a));
        assert_eq!(result[0].spend_usd, dec!(4.00));
        assert_eq!(result[1].team_id, Some(team_b));
        assert_eq!(result[1].spend_usd, dec!(3.00));
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn spend_tags_by_api_key_aggregates_correctly(pool: sqlx::PgPool) {
        let org = Uuid::new_v4();
        sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, 'test-org')")
            .bind(org)
            .execute(&pool)
            .await
            .expect("insert org");

        let user = Uuid::new_v4();
        sqlx::query("INSERT INTO users (id, organization_id, email, role) VALUES ($1, $2, 'test@example.com', 'user')")
            .bind(user)
            .bind(org)
            .execute(&pool)
            .await
            .expect("insert user");

        let key_a = Uuid::new_v4();
        let key_b = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO api_keys
             (id, user_id, organization_id, name, key_prefix, key_hash, scopes)
             VALUES
             ($1, $3, $2, 'key-a', 'prefix-a', 'hash-a', '{proxy:write}'),
             ($4, $3, $2, 'key-b', 'prefix-b', 'hash-b', '{proxy:write}')",
        )
        .bind(key_a)
        .bind(org)
        .bind(user)
        .bind(key_b)
        .execute(&pool)
        .await
        .expect("insert api_keys");

        sqlx::query(
            "INSERT INTO request_logs
             (organization_id, api_key_id, user_id, model, provider, provider_model_id,
              tokens_in, tokens_out, cost_usd, duration_ms, streamed, status)
             VALUES
             ($1, $2, $4, 'gpt-4o', 'openai', 'gpt-4o', 100, 50, 1.00, 10, false, 'success'),
             ($1, $2, $4, 'gpt-4o', 'openai', 'gpt-4o', 200, 100, 2.00, 20, false, 'success'),
             ($1, $3, $4, 'gpt-4o', 'openai', 'gpt-4o', 300, 150, 3.00, 30, false, 'success')",
        )
        .bind(org)
        .bind(key_a)
        .bind(key_b)
        .bind(user)
        .execute(&pool)
        .await
        .expect("insert request_logs");

        let mut result = fetch_api_key_spend(&pool, None, None, Some(org), None)
            .await
            .expect("fetch api_key spend");

        assert_eq!(result.len(), 2);
        // key_a and key_b have the same total, so ORDER BY spend_usd DESC does not pin
        // their relative order; verify both are present with correct amounts.
        let found: std::collections::HashMap<_, _> =
            result.iter().map(|r| (r.api_key_id, r.spend_usd)).collect();
        assert_eq!(found.get(&Some(key_a)), Some(&dec!(3.00)));
        assert_eq!(found.get(&Some(key_b)), Some(&dec!(3.00)));
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn spend_tags_respects_from_to_filters(pool: sqlx::PgPool) {
        let org = Uuid::new_v4();
        sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, 'test-org')")
            .bind(org)
            .execute(&pool)
            .await
            .expect("insert org");

        let user = Uuid::new_v4();
        sqlx::query("INSERT INTO users (id, organization_id, email, role) VALUES ($1, $2, 'test@example.com', 'user')")
            .bind(user)
            .bind(org)
            .execute(&pool)
            .await
            .expect("insert user");

        let now = Utc::now();
        let yesterday = now - chrono::Duration::days(1);
        let tomorrow = now + chrono::Duration::days(1);

        sqlx::query(
            "INSERT INTO request_logs
             (organization_id, user_id, model, provider, provider_model_id,
              cost_usd, duration_ms, streamed, status, created_at)
             VALUES
             ($1, $2, 'gpt-4o', 'openai', 'gpt-4o', 1.00, 10, false, 'success', $3),
             ($1, $2, 'gpt-4o', 'openai', 'gpt-4o', 2.00, 20, false, 'success', $4),
             ($1, $2, 'gpt-4o', 'openai', 'gpt-4o', 4.00, 30, false, 'success', $5)",
        )
        .bind(org)
        .bind(user)
        .bind(yesterday)
        .bind(now)
        .bind(tomorrow)
        .execute(&pool)
        .await
        .expect("insert request_logs");

        let result = fetch_team_spend(&pool, Some(now), Some(tomorrow), Some(org), Some(user))
            .await
            .expect("fetch filtered spend");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].spend_usd, dec!(6.00));
    }
}
