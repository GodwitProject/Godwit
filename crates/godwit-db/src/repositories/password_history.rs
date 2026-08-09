use godwit_core::PasteurError;
use sqlx::PgPool;
use uuid::Uuid;

pub struct PasswordHistoryRepository {
    pool: PgPool,
}

impl PasswordHistoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn push(&self, user_id: Uuid, hash: &str) -> Result<(), PasteurError> {
        sqlx::query("INSERT INTO password_history (user_id, password_hash) VALUES ($1, $2)")
            .bind(user_id)
            .bind(hash)
            .execute(&self.pool)
            .await
            .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn get_last_n(&self, user_id: Uuid, n: i64) -> Result<Vec<String>, PasteurError> {
        let rows = sqlx::query_as::<_, (String,)>(
            "SELECT password_hash FROM password_history WHERE user_id = $1 ORDER BY created_at DESC, id DESC LIMIT $2",
        )
        .bind(user_id)
        .bind(n)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(rows.into_iter().map(|(h,)| h).collect())
    }

    pub async fn purge_older_than(&self, user_id: Uuid, keep_n: i64) -> Result<(), PasteurError> {
        sqlx::query(
            "DELETE FROM password_history WHERE user_id = $1 AND id NOT IN (
                SELECT id FROM password_history WHERE user_id = $1 ORDER BY created_at DESC, id DESC LIMIT $2
            )",
        )
        .bind(user_id)
        .bind(keep_n)
        .execute(&self.pool)
        .await
        .map_err(|e| PasteurError::Database(e.to_string()))?;
        Ok(())
    }
}
