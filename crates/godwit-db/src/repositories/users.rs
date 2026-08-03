use crate::models::{User, UserRole};
use godwit_core::PasteurError;
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

    pub async fn list_all(&self) -> Result<Vec<User>, PasteurError> {
        sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY organization_id, email")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn update(
        &self,
        id: Uuid,
        name: Option<&str>,
        role: Option<&str>,
        organization_id: Option<Uuid>,
    ) -> Result<User, PasteurError> {
        let current = self.get_by_id(id).await?;
        sqlx::query_as::<_, User>(
            "UPDATE users SET name = $2, role = $3, organization_id = $4 WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(name.or(current.name.as_deref()))
        .bind(role.unwrap_or(current.role.as_str()))
        .bind(organization_id.or(current.organization_id))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), PasteurError> {
        sqlx::query("DELETE FROM users WHERE id = $1")
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

    #[sqlx::test]
    async fn list_all_spans_organizations(pool: PgPool) {
        let repo = UserRepository::new(pool);
        repo.create("a@example.com", None, UserRole::User, None).await.expect("create a");
        repo.create("b@example.com", None, UserRole::User, None).await.expect("create b");
        let all = repo.list_all().await.expect("list all");
        assert_eq!(all.len(), 2);
    }

    #[sqlx::test]
    async fn update_changes_only_provided_fields(pool: PgPool) {
        let repo = UserRepository::new(pool);
        let user = repo
            .create("carol@example.com", Some("Carol"), UserRole::User, None)
            .await
            .expect("create user");

        let renamed = repo
            .update(user.id, Some("Carol R."), None, None)
            .await
            .expect("update name only");
        assert_eq!(renamed.name.as_deref(), Some("Carol R."));
        assert_eq!(renamed.role, "user");

        let promoted = repo
            .update(user.id, None, Some("org_admin"), None)
            .await
            .expect("update role only");
        assert_eq!(promoted.name.as_deref(), Some("Carol R."));
        assert_eq!(promoted.role, "org_admin");
    }

    #[sqlx::test]
    async fn delete_removes_the_row(pool: PgPool) {
        let repo = UserRepository::new(pool);
        let user = repo
            .create("dan@example.com", None, UserRole::User, None)
            .await
            .expect("create user");
        repo.delete(user.id).await.expect("delete user");
        let err = repo.get_by_id(user.id).await.unwrap_err();
        assert!(matches!(err, godwit_core::PasteurError::NotFound));
    }
}
