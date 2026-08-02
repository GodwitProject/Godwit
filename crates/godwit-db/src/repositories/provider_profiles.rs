use crate::models::ProviderProfile;
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
        organization_id: Uuid,
        name: &str,
        protocol: &str,
        base_url: Option<&str>,
    ) -> Result<ProviderProfile, PasteurError> {
        sqlx::query_as::<_, ProviderProfile>(
            "INSERT INTO provider_profiles (organization_id, name, protocol, base_url) VALUES ($1, $2, $3, $4) RETURNING *"
        )
        .bind(organization_id)
        .bind(name)
        .bind(protocol)
        .bind(base_url)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn list_for_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<ProviderProfile>, PasteurError> {
        sqlx::query_as::<_, ProviderProfile>(
            "SELECT * FROM provider_profiles WHERE organization_id = $1 ORDER BY name"
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::organizations::OrganizationRepository;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn create_and_list_provider_profiles(pool: PgPool) {
        let orgs = OrganizationRepository::new(pool.clone());
        let org = orgs.create("test-org").await.expect("create org");

        let repo = ProviderProfileRepository::new(pool);
        let profile = repo
            .create(org.id, "openai-default", "openai", Some("https://api.openai.com/v1"))
            .await
            .expect("create profile");
        assert_eq!(profile.organization_id, org.id);
        assert_eq!(profile.name, "openai-default");
        assert_eq!(profile.protocol, "openai");
        assert_eq!(profile.base_url.as_deref(), Some("https://api.openai.com/v1"));

        let listed = repo
            .list_for_organization(org.id)
            .await
            .expect("list profiles");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, profile.id);
    }
}
