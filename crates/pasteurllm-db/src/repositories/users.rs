use crate::models::{User, UserRole};
use pasteurllm_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        email: &str,
        name: Option<&str>,
        role: UserRole,
        organization_id: Option<Uuid>,
    ) -> Result<User, PasteurError> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (email, name, role, organization_id) VALUES ($1, $2, $3, $4) RETURNING *"
        )
        .bind(email)
        .bind(name)
        .bind(role.as_str())
        .bind(organization_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<User, PasteurError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn get_by_email(&self, email: &str) -> Result<User, PasteurError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn list_for_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<User>, PasteurError> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE organization_id = $1")
            .bind(organization_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn create_and_fetch_user(pool: PgPool) {
        let repo = UserRepository::new(pool);
        let user = repo
            .create("alice@example.com", Some("Alice"), UserRole::OrgAdmin, None)
            .await
            .expect("create user");
        assert_eq!(user.email, "alice@example.com");

        let fetched = repo.get_by_id(user.id).await.expect("fetch user");
        assert_eq!(fetched.email, "alice@example.com");
    }
}
