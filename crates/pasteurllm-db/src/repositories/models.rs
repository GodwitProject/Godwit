use crate::models::Model;
use pasteurllm_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct ModelRepository {
    pool: PgPool,
}

impl ModelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        organization_id: Uuid,
        public_id: &str,
        provider: &str,
        provider_model_id: &str,
    ) -> Result<Model, PasteurError> {
        sqlx::query_as::<_, Model>(
            "INSERT INTO models (organization_id, public_id, provider, provider_model_id) VALUES ($1, $2, $3, $4) RETURNING *"
        )
        .bind(organization_id)
        .bind(public_id)
        .bind(provider)
        .bind(provider_model_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn list_for_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<Model>, PasteurError> {
        sqlx::query_as::<_, Model>("SELECT * FROM models WHERE organization_id = $1 ORDER BY public_id")
            .bind(organization_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get_by_public_id(
        &self,
        organization_id: Uuid,
        public_id: &str,
    ) -> Result<Model, PasteurError> {
        sqlx::query_as::<_, Model>("SELECT * FROM models WHERE organization_id = $1 AND public_id = $2")
            .bind(organization_id)
            .bind(public_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }
}
