use godwit_core::PasteurError;
use sqlx::{migrate::Migrator, PgPool};

pub mod models;
pub mod repositories;

pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub async fn run_migrations(pool: &PgPool) -> Result<(), PasteurError> {
    MIGRATOR
        .run(pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
}

pub async fn connect(database_url: &str) -> Result<PgPool, PasteurError> {
    PgPool::connect(database_url)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::{models::ModelRepository, provider_profiles::ProviderProfileRepository};
    use serde_json::json;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn migrations_run_successfully(pool: PgPool) {
        run_migrations(&pool).await.expect("migrations should run");
    }

    // Note: the org-scoped provider-profile backfill test that previously lived here
    // (`provider_profile_backfill_creates_profiles_for_existing_models`) exercised the
    // 20260802000001 migration's per-organization backfill by reverting to a legacy
    // `models.organization_id`-only schema. That column no longer exists at all once
    // the 20260803000002 instance-wide-catalog migration runs (it's applied
    // unconditionally by `#[sqlx::test]` before every test body), so the scenario it
    // simulated is no longer reachable and the test was removed rather than adapted.

    #[sqlx::test]
    async fn models_capability_check_constraint_rejects_invalid_value(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create("openai", "openai", Some("https://api.openai.com/v1"), false)
            .await
            .expect("create profile");

        let result = sqlx::query(
            "INSERT INTO models (public_id, provider, provider_profile_id, provider_model_id, capabilities)
             VALUES ('bad-cap', 'openai', $1, 'gpt-4', ARRAY['time_travel'])"
        )
        .bind(profile.id)
        .execute(&pool)
        .await;

        assert!(result.is_err(), "invalid capability should violate check constraint");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("chk_models_capabilities") || err.contains("check constraint"),
            "expected check constraint error, got: {err}"
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn models_capabilities_check_constraint_accepts_image_edit(pool: PgPool) {
        let profiles =
            crate::repositories::provider_profiles::ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create("openai", "openai", None, false)
            .await
            .expect("create profile");

        let result = sqlx::query(
            "INSERT INTO models (public_id, provider, provider_profile_id, provider_model_id, capabilities)
             VALUES ('edit-model', 'openai', $1, 'gpt-image-1', ARRAY['image_edit'])"
        )
        .bind(profile.id)
        .execute(&pool)
        .await;
        assert!(result.is_ok(), "image_edit should be a legal capability value, got: {:?}", result.err());
    }

    #[sqlx::test]
    async fn model_repository_create_round_trips_new_fields(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create("openai", "openai", Some("https://api.openai.com/v1"), false)
            .await
            .expect("create profile");

        let models = ModelRepository::new(pool);
        let created = models
            .create("my-model", "openai", profile.id, "gpt-4", "chat,image_generation")
            .await
            .expect("create model");

        assert_eq!(created.public_id, "my-model");
        assert_eq!(created.provider, "openai");
        assert_eq!(created.provider_profile_id, profile.id);
        assert_eq!(created.provider_model_id, "gpt-4");
        assert_eq!(created.capabilities, vec!["chat".to_string(), "image_generation".to_string()]);
        assert_eq!(created.pricing, json!({}));
        assert_eq!(created.config, json!({}));

        let fetched = models
            .get_by_public_id("my-model")
            .await
            .expect("fetch model");
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.provider_profile_id, profile.id);
        assert_eq!(fetched.capabilities, vec!["chat".to_string(), "image_generation".to_string()]);
    }
}
