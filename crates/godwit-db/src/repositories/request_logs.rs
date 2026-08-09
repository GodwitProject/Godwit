use crate::models::RequestLog;
use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

pub struct RequestLogsRepository {
    pool: PgPool,
}

impl RequestLogsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_with_tags(
        &self,
        api_key_id: Uuid,
        user_id: Uuid,
        organization_id: Uuid,
        team_id: Option<Uuid>,
        model: &str,
        provider: &str,
        provider_model_id: &str,
        capability: &str,
        duration_ms: i32,
        streamed: bool,
        status: &str,
        cost_usd: Option<Decimal>,
        tags: &[String],
    ) -> Result<RequestLog, PasteurError> {
        sqlx::query_as::<_, RequestLog>(
            "INSERT INTO request_logs 
             (api_key_id, user_id, organization_id, team_id, model, provider, provider_model_id, 
              capability, duration_ms, streamed, status, cost_usd, tags) 
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) 
             RETURNING *",
        )
        .bind(api_key_id)
        .bind(user_id)
        .bind(organization_id)
        .bind(team_id)
        .bind(model)
        .bind(provider)
        .bind(provider_model_id)
        .bind(capability)
        .bind(duration_ms)
        .bind(streamed)
        .bind(status)
        .bind(cost_usd)
        .bind(tags)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn find_by_tag(
        &self,
        tag: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        organization_id: Option<Uuid>,
    ) -> Result<Vec<RequestLog>, PasteurError> {
        sqlx::query_as::<_, RequestLog>(
            "SELECT * FROM request_logs 
             WHERE $1 = ANY(tags)
               AND ($2::timestamptz IS NULL OR created_at >= $2)
               AND ($3::timestamptz IS NULL OR created_at <= $3)
               AND ($4::uuid IS NULL OR organization_id = $4)
             ORDER BY created_at DESC",
        )
        .bind(tag)
        .bind(from)
        .bind(to)
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn aggregate_spend_by_tag(
        &self,
        tag: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        organization_id: Option<Uuid>,
        user_id: Option<Uuid>,
    ) -> Result<Vec<(String, Decimal)>, PasteurError> {
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

        sqlx::query_as::<_, (String, Decimal)>(query)
            .bind(tag)
            .bind(from)
            .bind(to)
            .bind(organization_id)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;
    

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn create_with_tags_round_trips_correctly(pool: PgPool) {
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

        let (_, _, prefix) = godwit_auth::api_keys::generate_api_key();
        let api_key = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO api_keys (user_id, organization_id, name, key_prefix, key_hash, scopes)
             VALUES ($1, $2, 'test', $3, 'hash', '{chat}') RETURNING id"
        )
        .bind(user)
        .bind(org)
        .bind(&prefix)
        .fetch_one(&pool)
        .await
        .expect("insert api_key");

        let repo = RequestLogsRepository::new(pool.clone());
        let tags = vec!["tag1".to_string(), "tag2".to_string()];
        
        let log = repo
            .create_with_tags(
                api_key, user, org, None, "gpt-4o", "openai", "gpt-4o",
                "chat", 100, false, "success", Some(Decimal::new(123, 2)),
                &tags,
            )
            .await
            .expect("create with tags");

        assert_eq!(log.tags, tags);
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn find_by_tag_filters_correctly(pool: PgPool) {
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

        let repo = RequestLogsRepository::new(pool.clone());
        
        sqlx::query(
            "INSERT INTO request_logs 
             (api_key_id, user_id, organization_id, model, provider, provider_model_id, 
              capability, duration_ms, streamed, status, cost_usd, tags)
             VALUES 
             (NULL, $1, $2, 'gpt-4o', 'openai', 'gpt-4o', 'chat', 100, false, 'success', 1.00, $3),
             (NULL, $1, $2, 'gpt-4o', 'openai', 'gpt-4o', 'chat', 100, false, 'success', 2.00, $4)",
        )
        .bind(user)
        .bind(org)
        .bind(vec!["production".to_string()])
        .bind(vec!["development".to_string()])
        .execute(&pool)
        .await
        .expect("insert logs");

        let prod_logs = repo
            .find_by_tag("production", None, None, Some(org))
            .await
            .expect("find by tag");

        assert_eq!(prod_logs.len(), 1);
        assert_eq!(prod_logs[0].tags, vec!["production"]);
    }
}
