use crate::models::PasswordResetToken;
use godwit_core::PasteurError;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

pub struct PasswordResetTokenRepository {
    pool: PgPool,
}

impl PasswordResetTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        token_hash: &str,
        ttl: Duration,
    ) -> Result<PasswordResetToken, PasteurError> {
        let expires_at = chrono::Utc::now() + chrono::Duration::from_std(ttl).unwrap();
        sqlx::query_as::<_, PasswordResetToken>(
            "INSERT INTO password_reset_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))
    }

    pub async fn get_by_hash(&self, token_hash: &str) -> Result<PasswordResetToken, PasteurError> {
        sqlx::query_as::<_, PasswordResetToken>(
            "SELECT * FROM password_reset_tokens WHERE token_hash = $1",
        )
        .bind(token_hash)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => PasteurError::NotFound,
            _ => PasteurError::Database(e.to_string()),
        })
    }

    pub async fn mark_used(&self, id: Uuid) -> Result<(), PasteurError> {
        sqlx::query("UPDATE password_reset_tokens SET used_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn delete_for_user(&self, user_id: Uuid) -> Result<(), PasteurError> {
        sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }
}
