use crate::models::Model;
use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct ModelRepository {
    pool: PgPool,
}

fn parse_capabilities(capabilities: &str) -> Vec<String> {
    let mut caps: Vec<String> = capabilities
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if caps.is_empty() {
        caps.push("chat".to_string());
    }
    caps
}

impl ModelRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        public_id: &str,
        provider: &str,
        provider_profile_id: Uuid,
        provider_model_id: &str,
        capabilities: &str,
    ) -> Result<Model, PasteurError> {
        sqlx::query_as::<_, Model>(
            "INSERT INTO models (public_id, provider, provider_profile_id, provider_model_id, capabilities) VALUES ($1, $2, $3, $4, $5) RETURNING *"
        )
        .bind(public_id)
        .bind(provider)
        .bind(provider_profile_id)
        .bind(provider_model_id)
        .bind(parse_capabilities(capabilities))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn list(&self) -> Result<Vec<Model>, PasteurError> {
        sqlx::query_as::<_, Model>("SELECT * FROM models ORDER BY public_id")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get(&self, id: Uuid) -> Result<Model, PasteurError> {
        sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn get_by_public_id(&self, public_id: &str) -> Result<Model, PasteurError> {
        sqlx::query_as::<_, Model>("SELECT * FROM models WHERE public_id = $1")
            .bind(public_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn get_by_profile_and_public_id(
        &self,
        provider_profile_id: Uuid,
        public_id: &str,
    ) -> Result<Model, PasteurError> {
        sqlx::query_as::<_, Model>(
            "SELECT * FROM models WHERE provider_profile_id = $1 AND public_id = $2",
        )
        .bind(provider_profile_id)
        .bind(public_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => PasteurError::NotFound,
            _ => PasteurError::Database(e.to_string()),
        })
    }

    pub async fn update(
        &self,
        id: Uuid,
        public_id: Option<&str>,
        capabilities: Option<&str>,
    ) -> Result<Model, PasteurError> {
        let current = self.get(id).await?;
        let new_public_id = public_id.unwrap_or(&current.public_id);
        let new_capabilities = capabilities
            .map(parse_capabilities)
            .unwrap_or(current.capabilities);
        sqlx::query_as::<_, Model>(
            "UPDATE models SET public_id = $2, capabilities = $3 WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .bind(new_public_id)
        .bind(new_capabilities)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), PasteurError> {
        sqlx::query("DELETE FROM models WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::provider_profiles::ProviderProfileRepository;

    #[sqlx::test]
    async fn create_list_and_get_model(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles.create("openai", "openai", None, false).await.expect("create profile");

        let models = ModelRepository::new(pool);
        let created = models
            .create("gpt-4o", "openai", profile.id, "gpt-4o", "chat")
            .await
            .expect("create model");
        assert_eq!(created.public_id, "gpt-4o");
        assert_eq!(created.provider_profile_id, profile.id);

        let listed = models.list().await.expect("list models");
        assert_eq!(listed.len(), 1);

        let fetched = models.get_by_public_id("gpt-4o").await.expect("get by public id");
        assert_eq!(fetched.id, created.id);
    }

    #[sqlx::test]
    async fn update_and_delete_model(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles.create("openai", "openai", None, false).await.expect("create profile");
        let models = ModelRepository::new(pool);
        let created = models
            .create("gpt-4o", "openai", profile.id, "gpt-4o", "chat")
            .await
            .expect("create model");

        let updated = models
            .update(created.id, Some("gpt-4o-renamed"), Some("chat,embedding"))
            .await
            .expect("update model");
        assert_eq!(updated.public_id, "gpt-4o-renamed");
        assert_eq!(updated.capabilities, vec!["chat".to_string(), "embedding".to_string()]);

        models.delete(created.id).await.expect("delete model");
        let err = models.get_by_public_id("gpt-4o-renamed").await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound));
    }
}
