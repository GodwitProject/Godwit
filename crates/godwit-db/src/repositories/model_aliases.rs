use crate::models::ModelAlias;
use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct ModelAliasRepository {
    pool: PgPool,
}

impl ModelAliasRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, alias: &str, target_model_id: Uuid) -> Result<ModelAlias, PasteurError> {
        sqlx::query_as::<_, ModelAlias>(
            "INSERT INTO model_aliases (alias, target_model_id) VALUES ($1, $2) RETURNING *",
        )
        .bind(alias)
        .bind(target_model_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get_by_alias(&self, alias: &str) -> Result<ModelAlias, PasteurError> {
        sqlx::query_as::<_, ModelAlias>("SELECT * FROM model_aliases WHERE alias = $1")
            .bind(alias)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn list(&self) -> Result<Vec<ModelAlias>, PasteurError> {
        sqlx::query_as::<_, ModelAlias>("SELECT * FROM model_aliases ORDER BY alias")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), PasteurError> {
        sqlx::query("DELETE FROM model_aliases WHERE id = $1")
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
    use crate::repositories::{models::ModelRepository, provider_profiles::ProviderProfileRepository};
    use sqlx::PgPool;

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn create_and_get_alias(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create("openai", "openai", None, false)
            .await
            .expect("create profile");

        let models = ModelRepository::new(pool.clone());
        let model = models
            .create("gpt-4o", "openai", profile.id, "gpt-4o", "chat", serde_json::json!({"input_price_per_million": 0, "output_price_per_million": 0}))
            .await
            .expect("create model");

        let aliases = ModelAliasRepository::new(pool);
        let created = aliases
            .create("gpt-4-turbo", model.id)
            .await
            .expect("create alias");

        assert_eq!(created.alias, "gpt-4-turbo");
        assert_eq!(created.target_model_id, model.id);

        let fetched = aliases
            .get_by_alias("gpt-4-turbo")
            .await
            .expect("get alias");
        assert_eq!(fetched.id, created.id);
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn list_aliases(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create("openai", "openai", None, false)
            .await
            .expect("create profile");

        let models = ModelRepository::new(pool.clone());
        let model = models
            .create("gpt-4o", "openai", profile.id, "gpt-4o", "chat", serde_json::json!({"input_price_per_million": 0, "output_price_per_million": 0}))
            .await
            .expect("create model");

        let aliases = ModelAliasRepository::new(pool);
        aliases.create("alias-a", model.id).await.expect("create alias a");
        aliases.create("alias-b", model.id).await.expect("create alias b");

        let listed = aliases.list().await.expect("list aliases");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].alias, "alias-a");
        assert_eq!(listed[1].alias, "alias-b");
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn delete_alias(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create("openai", "openai", None, false)
            .await
            .expect("create profile");

        let models = ModelRepository::new(pool.clone());
        let model = models
            .create("gpt-4o", "openai", profile.id, "gpt-4o", "chat", serde_json::json!({"input_price_per_million": 0, "output_price_per_million": 0}))
            .await
            .expect("create model");

        let aliases = ModelAliasRepository::new(pool);
        let created = aliases
            .create("to-delete", model.id)
            .await
            .expect("create alias");

        aliases.delete(created.id).await.expect("delete alias");

        let err = aliases.get_by_alias("to-delete").await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound));
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn get_nonexistent_alias_returns_not_found(pool: PgPool) {
        let aliases = ModelAliasRepository::new(pool);
        let err = aliases.get_by_alias("does-not-exist").await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound));
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn alias_with_nonexistent_target_returns_error(pool: PgPool) {
        let aliases = ModelAliasRepository::new(pool);
        let fake_model_id = Uuid::new_v4();
        let err = aliases.create("bad-alias", fake_model_id).await.unwrap_err();
        assert!(matches!(err, PasteurError::Database(_)));
    }
}
