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
        models::ModelRepository, provider_profiles::ProviderProfileRepository,
    };
    use serde_json::json;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn migrations_run_successfully(pool: PgPool) {
        run_migrations(&pool).await.expect("migrations should run");
    }

    /// Exercises the de-duplication logic in
    /// `migrations/20260803000002_instance_wide_catalog.up.sql` against actual
    /// duplicate data, simulating a real upgrade from the pre-instance-wide-catalog
    /// schema. `#[sqlx::test(migrations = false)]` gives us an empty database with no
    /// migrations applied, so we can apply every migration up through (but not
    /// including) the one under test, seed rows that collide the way two different
    /// organizations' data would have collided before `organization_id` is dropped,
    /// then run the migration under test directly and assert it collapses the
    /// duplicates without violating the `models.provider_profile_id` FK.
    #[sqlx::test(migrations = false)]
    async fn instance_wide_catalog_migration_collapses_duplicates(pool: PgPool) {
        let migrations_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        let target_migration = "20260803000002_instance_wide_catalog.up.sql";

        // 1. Apply every migration up through (but not including) the one under test,
        //    in filename order, by executing each file's raw SQL directly. Down-only
        //    counterparts (`*.down.sql`) are skipped; the lone irreversible migration
        //    (`20260801000001_initial.sql`) has no down file and is included.
        let mut up_files: Vec<_> = std::fs::read_dir(&migrations_dir)
            .expect("read migrations dir")
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                let name = p.file_name().unwrap().to_str().unwrap();
                name.ends_with(".sql") && !name.ends_with(".down.sql")
            })
            .collect();
        up_files.sort();

        for path in &up_files {
            let file_name = path.file_name().unwrap().to_str().unwrap();
            if file_name.starts_with("20260803000002") {
                break; // stop right before the migration under test
            }
            let sql =
                std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {path:?}: {e}"));
            sqlx::raw_sql(&sql)
                .execute(&pool)
                .await
                .unwrap_or_else(|e| panic!("applying {path:?}: {e}"));
        }

        // 2. Seed duplicate data as it existed pre-migration: two organizations, two
        //    provider_profiles rows both named 'openai' (simulating two orgs' profiles
        //    for the same protocol before organization_id is dropped and a
        //    cross-organization `name` uniqueness constraint is introduced), and two
        //    models rows that will collide on (provider_profile_id, public_id) once
        //    the profiles collapse to a single survivor. A third model, unique to the
        //    losing profile, confirms repointing (not just deletion) happens.
        let org_a: uuid::Uuid =
            sqlx::query_scalar("INSERT INTO organizations (name) VALUES ('org-a') RETURNING id")
                .fetch_one(&pool)
                .await
                .expect("insert org a");
        let org_b: uuid::Uuid =
            sqlx::query_scalar("INSERT INTO organizations (name) VALUES ('org-b') RETURNING id")
                .fetch_one(&pool)
                .await
                .expect("insert org b");

        let profile_a: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO provider_profiles (organization_id, name, protocol) VALUES ($1, 'openai', 'openai') RETURNING id"
        )
        .bind(org_a)
        .fetch_one(&pool)
        .await
        .expect("insert profile a");
        let profile_b: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO provider_profiles (organization_id, name, protocol) VALUES ($1, 'openai', 'openai') RETURNING id"
        )
        .bind(org_b)
        .fetch_one(&pool)
        .await
        .expect("insert profile b");

        // Collides with the model below on (public_id) once profile_b is repointed
        // to survivor profile_a.
        sqlx::query(
            "INSERT INTO models (organization_id, public_id, provider, provider_profile_id, provider_model_id) VALUES ($1, 'gpt-4o', 'openai', $2, 'gpt-4o')"
        )
        .bind(org_a)
        .bind(profile_a)
        .execute(&pool)
        .await
        .expect("insert model under profile a");
        sqlx::query(
            "INSERT INTO models (organization_id, public_id, provider, provider_profile_id, provider_model_id) VALUES ($1, 'gpt-4o', 'openai', $2, 'gpt-4o')"
        )
        .bind(org_b)
        .bind(profile_b)
        .execute(&pool)
        .await
        .expect("insert colliding model under profile b");
        // Unique to profile_b: should survive, repointed onto profile_a.
        sqlx::query(
            "INSERT INTO models (organization_id, public_id, provider, provider_profile_id, provider_model_id) VALUES ($1, 'gpt-3.5', 'openai', $2, 'gpt-3.5-turbo')"
        )
        .bind(org_b)
        .bind(profile_b)
        .execute(&pool)
        .await
        .expect("insert non-colliding model under profile b");

        // 3. Apply the migration under test and confirm it doesn't error (in
        //    particular, no FK violation from deleting a still-referenced profile).
        let up_sql = std::fs::read_to_string(migrations_dir.join(target_migration))
            .expect("read migration under test");
        sqlx::raw_sql(&up_sql)
            .execute(&pool)
            .await
            .expect("migration should collapse duplicates without an FK violation");

        // 4. Assert de-dup and repointing actually happened.
        let profile_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM provider_profiles WHERE name = 'openai'")
                .fetch_one(&pool)
                .await
                .expect("count openai profiles");
        assert_eq!(
            profile_count, 1,
            "duplicate provider_profiles rows sharing a name should collapse to one survivor"
        );

        let surviving_profile_id: uuid::Uuid =
            sqlx::query_scalar("SELECT id FROM provider_profiles WHERE name = 'openai'")
                .fetch_one(&pool)
                .await
                .expect("fetch surviving profile id");

        let model_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM models WHERE provider_profile_id = $1 AND public_id = 'gpt-4o'",
        )
        .bind(surviving_profile_id)
        .fetch_one(&pool)
        .await
        .expect("count gpt-4o models");
        assert_eq!(
            model_count, 1,
            "models colliding on (provider_profile_id, public_id) after repointing should collapse to one row"
        );

        let repointed_survivor_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM models WHERE provider_profile_id = $1 AND public_id = 'gpt-3.5'",
        )
        .bind(surviving_profile_id)
        .fetch_one(&pool)
        .await
        .expect("count gpt-3.5 models");
        assert_eq!(
            repointed_survivor_count, 1,
            "the non-colliding model from the losing profile should be repointed at the surviving profile, not lost"
        );

        // The unique constraints the migration adds should now hold (this would fail
        // with a constraint violation if de-dup had left any duplicates behind).
        let dup_name_check: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM (SELECT name FROM provider_profiles GROUP BY name HAVING COUNT(*) > 1) t",
        )
        .fetch_one(&pool)
        .await
        .expect("check for duplicate profile names");
        assert_eq!(dup_name_check, 0);

        let dup_model_check: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM (SELECT provider_profile_id, public_id FROM models GROUP BY provider_profile_id, public_id HAVING COUNT(*) > 1) t",
        )
        .fetch_one(&pool)
        .await
        .expect("check for duplicate models");
        assert_eq!(dup_model_check, 0);
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

        assert!(
            result.is_err(),
            "invalid capability should violate check constraint"
        );
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
        assert!(
            result.is_ok(),
            "image_edit should be a legal capability value, got: {:?}",
            result.err()
        );
    }

    /// `models.provider` was constrained to ('openai','anthropic') by the initial
    /// migration, which made catalog rows for the five other protocols that now have real
    /// adapters impossible to insert. 20260803000003 relaxes it; this exercises the
    /// relaxed constraint through the repository for every one of the seven values.
    #[sqlx::test]
    async fn models_provider_check_constraint_accepts_all_protocols(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create(
                "local-vllm",
                "vllm",
                Some("http://localhost:8000/v1"),
                false,
            )
            .await
            .expect("create profile");

        let models = ModelRepository::new(pool);
        let created = models
            .create(
                "llama-3-70b",
                "vllm",
                profile.id,
                "meta-llama/Llama-3-70B-Instruct",
                "chat",
            )
            .await
            .expect("a vllm-provider model row must be insertable");
        assert_eq!(created.provider, "vllm");
        assert_eq!(created.provider_model_id, "meta-llama/Llama-3-70B-Instruct");

        // The remaining protocols must be accepted too.
        for provider in [
            "openai",
            "anthropic",
            "gemini",
            "sglang",
            "llama_cpp",
            "ollama",
        ] {
            models
                .create(
                    &format!("model-{provider}"),
                    provider,
                    profile.id,
                    &format!("upstream-{provider}"),
                    "chat",
                )
                .await
                .unwrap_or_else(|e| panic!("provider '{provider}' should be accepted, got: {e:?}"));
        }
    }

    #[sqlx::test]
    async fn models_provider_check_constraint_still_rejects_unknown_protocol(pool: PgPool) {
        let profiles = ProviderProfileRepository::new(pool.clone());
        let profile = profiles
            .create("openai", "openai", None, false)
            .await
            .expect("create profile");

        let models = ModelRepository::new(pool);
        let err = models
            .create("bogus", "not_a_protocol", profile.id, "x", "chat")
            .await
            .expect_err("an unknown provider must still violate the check constraint");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("models_provider_check") || msg.contains("check constraint"),
            "expected check constraint violation, got: {msg}"
        );
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
            .create(
                "my-model",
                "openai",
                profile.id,
                "gpt-4",
                "chat,image_generation",
            )
            .await
            .expect("create model");

        assert_eq!(created.public_id, "my-model");
        assert_eq!(created.provider, "openai");
        assert_eq!(created.provider_profile_id, profile.id);
        assert_eq!(created.provider_model_id, "gpt-4");
        assert_eq!(
            created.capabilities,
            vec!["chat".to_string(), "image_generation".to_string()]
        );
        assert_eq!(created.pricing, json!({}));
        assert_eq!(created.config, json!({}));

        let fetched = models
            .get_by_public_id("my-model")
            .await
            .expect("fetch model");
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.provider_profile_id, profile.id);
        assert_eq!(
            fetched.capabilities,
            vec!["chat".to_string(), "image_generation".to_string()]
        );
    }
}
