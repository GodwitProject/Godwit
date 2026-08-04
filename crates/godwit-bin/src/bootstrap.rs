use godwit_auth::credentials::encrypt_api_key;
use godwit_db::{
    models::UserRole,
    repositories::{
        organizations::OrganizationRepository, provider_profiles::ProviderProfileRepository,
        users::UserRepository,
    },
};
use sqlx::PgPool;

pub struct LegacyProviderConfig {
    pub name: &'static str,
    pub protocol: &'static str,
    pub base_url: String,
    pub api_key: String,
}

pub async fn bootstrap_provider_profiles(
    pool: &PgPool,
    master_key: &[u8; 32],
    legacy: &[LegacyProviderConfig],
) -> anyhow::Result<()> {
    let repo = ProviderProfileRepository::new(pool.clone());
    if !repo.list().await?.is_empty() {
        return Ok(());
    }
    for provider in legacy {
        let profile = repo
            .create(
                provider.name,
                provider.protocol,
                Some(&provider.base_url),
                false,
            )
            .await?;
        let secret = encrypt_api_key(master_key, &provider.api_key);
        repo.set_auth(profile.id, &secret).await?;
    }
    Ok(())
}

/// Creates the first `super_admin` account from `ADMIN_EMAIL`/`ADMIN_PASSWORD` env vars
/// when no user with that email exists yet. Without this, operators have no way to log
/// into the admin UI on a fresh database short of hand-writing SQL. Idempotent by email:
/// safe to run on every startup, including against a database that already has users.
pub async fn bootstrap_admin_user(pool: &PgPool, email: &str, password: &str) -> anyhow::Result<()> {
    let user_repo = UserRepository::new(pool.clone());
    if user_repo.get_by_email(email).await.is_ok() {
        return Ok(());
    }

    let org_repo = OrganizationRepository::new(pool.clone());
    let existing_orgs = org_repo.list().await?;
    let org = match existing_orgs.into_iter().next() {
        Some(org) => org,
        None => org_repo.create("Default Organization", None).await?,
    };

    let hash = godwit_auth::api_keys::hash_password(password);
    let user = user_repo
        .create(email, Some("Admin"), UserRole::SuperAdmin, Some(org.id))
        .await?;
    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(&hash)
        .bind(user.id)
        .execute(pool)
        .await?;
    Ok(())
}

pub fn legacy_providers_from_env() -> Vec<LegacyProviderConfig> {
    let mut providers = Vec::new();
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        providers.push(LegacyProviderConfig {
            name: "openai",
            protocol: "openai",
            base_url: std::env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            api_key: key,
        });
    }
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        providers.push(LegacyProviderConfig {
            name: "anthropic",
            protocol: "anthropic",
            base_url: std::env::var("ANTHROPIC_BASE_URL")
                .unwrap_or_else(|_| "https://api.anthropic.com".to_string()),
            api_key: key,
        });
    }
    if let Ok(key) = std::env::var("GEMINI_API_KEY") {
        providers.push(LegacyProviderConfig {
            name: "gemini",
            protocol: "gemini",
            base_url: std::env::var("GEMINI_BASE_URL")
                .unwrap_or_else(|_| "https://generativelanguage.googleapis.com".to_string()),
            api_key: key,
        });
    }
    providers
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn seeds_profiles_when_table_is_empty(pool: PgPool) {
        let legacy = vec![LegacyProviderConfig {
            name: "openai",
            protocol: "openai",
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-legacy".to_string(),
        }];
        bootstrap_provider_profiles(&pool, &[9u8; 32], &legacy)
            .await
            .expect("bootstrap");

        let repo = ProviderProfileRepository::new(pool);
        let profiles = repo.list().await.expect("list");
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "openai");
    }

    #[sqlx::test(migrations = "../godwit-db/migrations")]
    async fn does_nothing_when_profiles_already_exist(pool: PgPool) {
        let repo = ProviderProfileRepository::new(pool.clone());
        repo.create("existing", "openai", None, false)
            .await
            .expect("create profile");

        let legacy = vec![LegacyProviderConfig {
            name: "openai",
            protocol: "openai",
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-legacy".to_string(),
        }];
        bootstrap_provider_profiles(&pool, &[9u8; 32], &legacy)
            .await
            .expect("bootstrap");

        let profiles = repo.list().await.expect("list");
        assert_eq!(
            profiles.len(),
            1,
            "should not add legacy profiles when any profile already exists"
        );
        assert_eq!(profiles[0].name, "existing");
    }
}
