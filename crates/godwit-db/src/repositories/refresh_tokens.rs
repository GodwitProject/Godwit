use crate::models::RefreshToken;
use chrono::{DateTime, Utc};
use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct RefreshTokenRepository {
    pool: PgPool,
}

impl RefreshTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<RefreshToken, PasteurError> {
        sqlx::query_as::<_, RefreshToken>(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3) RETURNING *"
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get_by_hash(&self, token_hash: &str) -> Result<RefreshToken, PasteurError> {
        sqlx::query_as::<_, RefreshToken>("SELECT * FROM refresh_tokens WHERE token_hash = $1")
            .bind(token_hash)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => PasteurError::NotFound,
                _ => PasteurError::Database(e.to_string()),
            })
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), PasteurError> {
        sqlx::query("DELETE FROM refresh_tokens WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn delete_by_hash(&self, token_hash: &str) -> Result<(), PasteurError> {
        sqlx::query("DELETE FROM refresh_tokens WHERE token_hash = $1")
            .bind(token_hash)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn delete_all_for_user(&self, user_id: Uuid) -> Result<u64, PasteurError> {
        let res = sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(res.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::users::UserRepository;
    use crate::models::UserRole;
    use sqlx::PgPool;

    #[sqlx::test]
    async fn create_and_get_by_hash(pool: PgPool) {
        let users = UserRepository::new(pool.clone());
        let user = users
            .create("alice@example.com", None, UserRole::User, None)
            .await
            .expect("create user");

        let repo = RefreshTokenRepository::new(pool);
        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
        let created = repo
            .create(user.id, "hash-abc", expires_at)
            .await
            .expect("create refresh token");
        assert_eq!(created.user_id, user.id);
        assert_eq!(created.token_hash, "hash-abc");

        let fetched = repo.get_by_hash("hash-abc").await.expect("get by hash");
        assert_eq!(fetched.id, created.id);
    }

    #[sqlx::test]
    async fn get_by_hash_not_found(pool: PgPool) {
        let repo = RefreshTokenRepository::new(pool);
        let err = repo.get_by_hash("missing").await.unwrap_err();
        assert!(matches!(err, godwit_core::PasteurError::NotFound));
    }

    #[sqlx::test]
    async fn delete_by_hash_removes_it(pool: PgPool) {
        let users = UserRepository::new(pool.clone());
        let user = users
            .create("bob@example.com", None, UserRole::User, None)
            .await
            .expect("create user");
        let repo = RefreshTokenRepository::new(pool);
        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
        repo.create(user.id, "hash-to-delete", expires_at)
            .await
            .expect("create refresh token");

        repo.delete_by_hash("hash-to-delete").await.expect("delete");
        let err = repo.get_by_hash("hash-to-delete").await.unwrap_err();
        assert!(matches!(err, godwit_core::PasteurError::NotFound));
    }

    #[sqlx::test]
    async fn delete_all_for_user_removes_only_that_user(pool: PgPool) {
        use crate::repositories::refresh_tokens::RefreshTokenRepository;
        let users = UserRepository::new(pool.clone());
        let user_a = users.create("aaa@example.com", None, UserRole::User, None)
            .await.expect("create user a");
        let user_b = users.create("bbb@example.com", None, UserRole::User, None)
            .await.expect("create user b");

        let repo = RefreshTokenRepository::new(pool.clone());
        let exp = chrono::Utc::now() + chrono::Duration::days(7);
        repo.create(user_a.id, "hash-a1", exp).await.expect("a1");
        repo.create(user_a.id, "hash-a2", exp).await.expect("a2");
        repo.create(user_b.id, "hash-b1", exp).await.expect("b1");

        let n = repo.delete_all_for_user(user_a.id).await.expect("delete all a");
        assert_eq!(n, 2);

        assert!(repo.get_by_hash("hash-a1").await.is_err());
        assert!(repo.get_by_hash("hash-a2").await.is_err());
        assert!(repo.get_by_hash("hash-b1").await.is_ok());
    }

    #[sqlx::test]
    async fn deleting_user_cascades_refresh_tokens(pool: PgPool) {
        let users = UserRepository::new(pool.clone());
        let user = users
            .create("carol@example.com", None, UserRole::User, None)
            .await
            .expect("create user");
        let repo = RefreshTokenRepository::new(pool.clone());
        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
        repo.create(user.id, "hash-cascade", expires_at)
            .await
            .expect("create refresh token");

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user.id)
            .execute(&pool)
            .await
            .expect("delete user");

        let err = repo.get_by_hash("hash-cascade").await.unwrap_err();
        assert!(matches!(err, godwit_core::PasteurError::NotFound));
    }
}
