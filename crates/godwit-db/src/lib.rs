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
    use crate::repositories::{
        models::ModelRepository, organizations::OrganizationRepository,
        provider_profiles::ProviderProfileRepository,
    };
    use serde_json::json;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn migrations_run_successfully(pool: PgPool) {
        run_migrations(&pool).await.expect("migrations should run");
    }

    #[sqlx::test]
    async fn provider_profile_backfill_creates_profiles_for_existing_models(pool: PgPool) {
        // Revert the provider-profile migration so we can simulate legacy models.
        sqlx::raw_sql(
            "ALTER TABLE models DROP CONSTRAINT IF EXISTS chk_models_capability;
             ALTER TABLE models DROP COLUMN IF EXISTS provider_profile_id CASCADE;
             ALTER TABLE models DROP COLUMN IF EXISTS capability;
             ALTER TABLE models DROP COLUMN IF EXISTS pricing;
             DROP TABLE IF EXISTS provider_profiles CASCADE;"
        )
        .execute(&pool)
        .await
        .expect("revert provider profile changes");

        let orgs = OrganizationRepository::new(pool.clone());
        let org1 = orgs.create("org-1").await.expect("create org 1");
        let org2 = orgs.create("org-2").await.expect("create org 2");

        // Insert legacy models (pre-provider_profile_id schema).
        sqlx::query(
            "INSERT INTO models (organization_id, public_id, provider, provider_model_id, config)
             VALUES ($1, 'gpt-4', 'openai', 'gpt-4', '{}'),
                    ($2, 'claude-3', 'anthropic', 'claude-3', '{}'),
                    ($3, 'gpt-3.5', 'openai', 'gpt-3.5-turbo', '{}')"
        )
        .bind(org1.id)
        .bind(org1.id)
        .bind(org2.id)
        .execute(&pool)
        .await
        .expect("insert legacy models");

        // Re-run the provider profile migration.
        let migration = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/migrations/20260802000001_provider_profiles.up.sql"
        ));
        sqlx::raw_sql(migration).execute(&pool).await.expect("reapply migration");

        // Verify provider profiles were backfilled per organization/provider pair.
        let profiles: Vec<(uuid::Uuid, String, String)> = sqlx::query_as(
            "SELECT organization_id, name, protocol FROM provider_profiles"
        )
        .fetch_all(&pool)
        .await
        .expect("fetch profiles");
        assert_eq!(profiles.len(), 3);
        let profile_set: std::collections::HashSet<_> = profiles.into_iter().collect();
        assert!(profile_set.contains(&(org1.id, "anthropic".to_string(), "anthropic".to_string())));
        assert!(profile_set.contains(&(org1.id, "openai".to_string(), "openai".to_string())));
        assert!(profile_set.contains(&(org2.id, "openai".to_string(), "openai".to_string())));

        // Verify every model now has a non-null provider_profile_id.
        let unlinked: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM models WHERE provider_profile_id IS NULL"
        )
        .fetch_one(&pool)
        .await
        .expect("count unlinked models");
        assert_eq!(unlinked, 0);

        // Verify defaults were applied.
        let defaults: (String, serde_json::Value, serde_json::Value) = sqlx::query_as(
            "SELECT capability, pricing, config FROM models WHERE public_id = 'gpt-4'"
        )
        .fetch_one(&pool)
        .await
        .expect("fetch defaults");
        assert_eq!(defaults.0, "chat");
        assert_eq!(defaults.1, json!({}));
        assert_eq!(defaults.2, json!({}));
    }

    #[sqlx::test]
    async fn models_capability_check_constraint_rejects_invalid_value(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("check-org").await.expect("create org");

        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create(org.id, "openai", "openai", Some("https://api.openai.com/v1"))
            .await
            .expect("create profile");

        let result = sqlx::query(
            "INSERT INTO models (organization_id, public_id, provider, provider_profile_id, provider_model_id, capability)
             VALUES ($1, 'bad-cap', 'openai', $2, 'gpt-4', 'time_travel')"
        )
        .bind(org.id)
        .bind(profile.id)
        .execute(&pool)
        .await;

        assert!(result.is_err(), "invalid capability should violate check constraint");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("chk_models_capability") || err.contains("check constraint"),
            "expected check constraint error, got: {err}"
        );
    }

    #[sqlx::test]
    async fn model_repository_create_round_trips_new_fields(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("round-trip-org").await.expect("create org");

        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create(org.id, "openai", "openai", Some("https://api.openai.com/v1"))
            .await
            .expect("create profile");

        let models = ModelRepository::new(pool);
        let created = models
            .create(org.id, "my-model", "openai", profile.id, "gpt-4")
            .await
            .expect("create model");

        assert_eq!(created.organization_id, org.id);
        assert_eq!(created.public_id, "my-model");
        assert_eq!(created.provider, "openai");
        assert_eq!(created.provider_profile_id, profile.id);
        assert_eq!(created.provider_model_id, "gpt-4");
        assert_eq!(created.capability, "chat");
        assert_eq!(created.pricing, json!({}));
        assert_eq!(created.config, json!({}));

        let fetched = models
            .get_by_public_id(org.id, "my-model")
            .await
            .expect("fetch model");
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.provider_profile_id, profile.id);
        assert_eq!(fetched.capability, "chat");
    }
}
