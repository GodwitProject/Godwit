use crate::models::Organization;
use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct OrganizationRepository {
    pool: PgPool,
}

impl OrganizationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        name: &str,
        rate_limit_requests_per_minute: Option<i32>,
    ) -> Result<Organization, PasteurError> {
        self.create_with_limits(name, rate_limit_requests_per_minute, None)
            .await
    }

    pub async fn create_with_limits(
        &self,
        name: &str,
        rate_limit_requests_per_minute: Option<i32>,
        rate_limit_tokens_per_minute: Option<i32>,
    ) -> Result<Organization, PasteurError> {
        sqlx::query_as::<_, Organization>(
            "INSERT INTO organizations (name, rate_limit_requests_per_minute, rate_limit_tokens_per_minute) VALUES ($1, $2, $3) RETURNING *"
        )
        .bind(name)
        .bind(rate_limit_requests_per_minute)
        .bind(rate_limit_tokens_per_minute)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Organization, PasteurError> {
        sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn list(&self) -> Result<Vec<Organization>, PasteurError> {
        sqlx::query_as::<_, Organization>("SELECT * FROM organizations ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn update(
        &self,
        id: Uuid,
        name: Option<&str>,
        rate_limit_requests_per_minute: Option<i32>,
    ) -> Result<Organization, PasteurError> {
        self.update_with_limits(id, name, rate_limit_requests_per_minute, None)
            .await
    }

    pub async fn update_with_limits(
        &self,
        id: Uuid,
        name: Option<&str>,
        rate_limit_requests_per_minute: Option<i32>,
        rate_limit_tokens_per_minute: Option<i32>,
    ) -> Result<Organization, PasteurError> {
        let current = self.get_by_id(id).await?;
        sqlx::query_as::<_, Organization>(
            "UPDATE organizations SET name = $2, rate_limit_requests_per_minute = $3, rate_limit_tokens_per_minute = $4 WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .bind(name.unwrap_or(current.name.as_str()))
        .bind(rate_limit_requests_per_minute.or(current.rate_limit_requests_per_minute))
        .bind(rate_limit_tokens_per_minute.or(current.rate_limit_tokens_per_minute))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), PasteurError> {
        sqlx::query("DELETE FROM organizations WHERE id = $1")
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
    use sqlx::PgPool;

    #[sqlx::test]
    async fn create_with_rate_limit(pool: PgPool) {
        let repo = OrganizationRepository::new(pool);
        let org = repo
            .create_with_limits("acme", Some(100), Some(10000))
            .await
            .expect("create org");
        assert_eq!(org.name, "acme");
        assert_eq!(org.rate_limit_requests_per_minute, Some(100));
        assert_eq!(org.rate_limit_tokens_per_minute, Some(10000));
    }

    #[sqlx::test]
    async fn create_without_rate_limit(pool: PgPool) {
        let repo = OrganizationRepository::new(pool);
        let org = repo.create("acme", None).await.expect("create org");
        assert_eq!(org.rate_limit_requests_per_minute, None);
        assert_eq!(org.rate_limit_tokens_per_minute, None);
    }

    #[sqlx::test]
    async fn update_changes_only_provided_fields(pool: PgPool) {
        let repo = OrganizationRepository::new(pool);
        let org = repo
            .create_with_limits("acme", Some(100), Some(10000))
            .await
            .expect("create org");

        let renamed = repo
            .update(org.id, Some("acme-corp"), None)
            .await
            .expect("update name only");
        assert_eq!(renamed.name, "acme-corp");
        assert_eq!(renamed.rate_limit_requests_per_minute, Some(100));
        assert_eq!(renamed.rate_limit_tokens_per_minute, Some(10000));

        let rate_limited = repo
            .update_with_limits(org.id, None, Some(50), None)
            .await
            .expect("update rpm only");
        assert_eq!(rate_limited.name, "acme-corp");
        assert_eq!(rate_limited.rate_limit_requests_per_minute, Some(50));
        assert_eq!(rate_limited.rate_limit_tokens_per_minute, Some(10000));

        let token_limited = repo
            .update_with_limits(org.id, None, None, Some(5000))
            .await
            .expect("update tpm only");
        assert_eq!(token_limited.name, "acme-corp");
        assert_eq!(token_limited.rate_limit_requests_per_minute, Some(50));
        assert_eq!(token_limited.rate_limit_tokens_per_minute, Some(5000));
    }
}
