use crate::models::ApiKey;
use godwit_core::PasteurError;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

pub struct ApiKeyRepository {
    pool: PgPool,
}

impl ApiKeyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
        name: &str,
        key_prefix: &str,
        key_hash: &str,
        scopes: &[String],
        budget_limit_usd: Option<Decimal>,
        rate_limit: Option<i32>,
    ) -> Result<ApiKey, PasteurError> {
        sqlx::query_as::<_, ApiKey>(
            "INSERT INTO api_keys (user_id, organization_id, name, key_prefix, key_hash, scopes, budget_limit_usd, rate_limit_requests_per_minute)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *"
        )
        .bind(user_id)
        .bind(organization_id)
        .bind(name)
        .bind(key_prefix)
        .bind(key_hash)
        .bind(scopes)
        .bind(budget_limit_usd)
        .bind(rate_limit)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get_by_prefix(&self, prefix: &str) -> Result<Vec<ApiKey>, PasteurError> {
        sqlx::query_as::<_, ApiKey>(
            "SELECT * FROM api_keys WHERE key_prefix = $1 AND disabled = FALSE",
        )
        .bind(prefix)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn list_for_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<ApiKey>, PasteurError> {
        sqlx::query_as::<_, ApiKey>("SELECT * FROM api_keys WHERE organization_id = $1")
            .bind(organization_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }
}
