use crate::models::ProviderProfile;
use godwit_auth::credentials::EncryptedSecret;
use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct ProviderProfileRepository {
    pool: PgPool,
}

impl ProviderProfileRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        name: &str,
        protocol: &str,
        base_url: Option<&str>,
        allow_wildcard: bool,
    ) -> Result<ProviderProfile, PasteurError> {
        sqlx::query_as::<_, ProviderProfile>(
            "INSERT INTO provider_profiles (name, protocol, base_url, allow_wildcard) VALUES ($1, $2, $3, $4) RETURNING *"
        )
        .bind(name)
        .bind(protocol)
        .bind(base_url)
        .bind(allow_wildcard)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn list(&self) -> Result<Vec<ProviderProfile>, PasteurError> {
        sqlx::query_as::<_, ProviderProfile>("SELECT * FROM provider_profiles ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get(&self, id: Uuid) -> Result<ProviderProfile, PasteurError> {
        sqlx::query_as::<_, ProviderProfile>("SELECT * FROM provider_profiles WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn get_by_name(&self, name: &str) -> Result<ProviderProfile, PasteurError> {
        sqlx::query_as::<_, ProviderProfile>("SELECT * FROM provider_profiles WHERE name = $1")
            .bind(name)
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
        base_url: Option<&str>,
        allow_wildcard: Option<bool>,
        enabled: Option<bool>,
    ) -> Result<ProviderProfile, PasteurError> {
        let current = self.get(id).await?;
        sqlx::query_as::<_, ProviderProfile>(
            "UPDATE provider_profiles SET base_url = $2, allow_wildcard = $3, enabled = $4 WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .bind(base_url.map(str::to_string).or(current.base_url))
        .bind(allow_wildcard.unwrap_or(current.allow_wildcard))
        .bind(enabled.unwrap_or(current.enabled))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn set_auth(
        &self,
        id: Uuid,
        secret: &EncryptedSecret,
    ) -> Result<ProviderProfile, PasteurError> {
        let auth =
            serde_json::to_value(secret).map_err(|e| PasteurError::Validation(e.to_string()))?;
        sqlx::query_as::<_, ProviderProfile>(
            "UPDATE provider_profiles SET auth = $2 WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(auth)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), PasteurError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM models WHERE provider_profile_id = $1"
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))?;

        if count > 0 {
            return Err(PasteurError::Validation(
                "Provider profile is referenced by existing models".to_string(),
            ));
        }

        let result = sqlx::query("DELETE FROM provider_profiles WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(PasteurError::NotFound);
        }

        tx.commit()
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn create_and_list_provider_profiles(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool);
        let profile = repo
            .create(
                "openai-default",
                "openai",
                Some("https://api.openai.com/v1"),
                false,
            )
            .await
            .expect("create profile");
        assert_eq!(profile.name, "openai-default");
        assert_eq!(profile.protocol, "openai");
        assert_eq!(
            profile.base_url.as_deref(),
            Some("https://api.openai.com/v1")
        );
        assert!(!profile.allow_wildcard);

        let listed = repo.list().await.expect("list profiles");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, profile.id);
    }

    #[sqlx::test]
    async fn get_profile_by_id(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool);
        let profile = repo
            .create("openai", "openai", None, false)
            .await
            .expect("create profile");
        let fetched = repo.get(profile.id).await.expect("get profile");
        assert_eq!(fetched.id, profile.id);
    }

    #[sqlx::test]
    async fn get_profile_by_id_not_found(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool);
        let err = repo.get(uuid::Uuid::nil()).await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound));
    }

    #[sqlx::test]
    async fn get_profile_by_name(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool);
        let profile = repo
            .create(
                "azure",
                "azure_openai",
                Some("https://azure.example.com"),
                true,
            )
            .await
            .expect("create profile");
        let fetched = repo
            .get_by_name("azure")
            .await
            .expect("get profile by name");
        assert_eq!(fetched.id, profile.id);
        assert_eq!(fetched.protocol, "azure_openai");
        assert!(fetched.allow_wildcard);
    }

    #[sqlx::test]
    async fn get_profile_by_name_not_found(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool);
        let err = repo.get_by_name("missing").await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound));
    }

    #[sqlx::test]
    async fn update_profile_fields(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool);
        let profile = repo
            .create("openai", "openai", None, false)
            .await
            .expect("create profile");
        let updated = repo
            .update(
                profile.id,
                Some("https://new.example.com"),
                Some(true),
                Some(false),
            )
            .await
            .expect("update profile");
        assert_eq!(updated.base_url.as_deref(), Some("https://new.example.com"));
        assert!(updated.allow_wildcard);
        assert!(!updated.enabled);
    }

    #[sqlx::test]
    async fn set_auth_stores_encrypted_secret(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool);
        let profile = repo
            .create("openai", "openai", None, false)
            .await
            .expect("create profile");
        let secret = godwit_auth::credentials::encrypt_api_key(&[3u8; 32], "sk-test");
        let updated = repo.set_auth(profile.id, &secret).await.expect("set auth");
        let stored: godwit_auth::credentials::EncryptedSecret =
            serde_json::from_value(updated.auth.clone()).expect("deserialize stored auth");
        assert_eq!(stored.ciphertext, secret.ciphertext);
        assert_eq!(stored.nonce, secret.nonce);
    }

    #[sqlx::test]
    async fn delete_profile_ok(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool);
        let profile = repo
            .create("openai", "openai", None, false)
            .await
            .expect("create profile");

        repo.delete(profile.id).await.expect("delete profile");

        let err = repo.get(profile.id).await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound));
    }

    #[sqlx::test]
    async fn cannot_delete_profile_with_models(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create("openai", "openai", None, false)
            .await
            .expect("create profile");

        let models = crate::repositories::models::ModelRepository::new(pool.clone());
        models
            .create(
                "gpt-4o",
                "openai",
                profile.id,
                "gpt-4o",
                "chat",
                serde_json::json!({
                    "input_price_per_million": 5.0,
                    "output_price_per_million": 15.0,
                }),
            )
            .await
            .expect("create model");

        let err = profiles.delete(profile.id).await.unwrap_err();
        assert!(matches!(err, PasteurError::Validation(_)));
    }

    #[sqlx::test]
    async fn delete_nonexistent_profile_returns_not_found(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool);
        let err = repo.delete(uuid::Uuid::nil()).await.unwrap_err();
        assert!(matches!(err, PasteurError::NotFound));
    }
}
