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
    use sqlx::PgPool;

    #[sqlx::test]
    async fn migrations_run_successfully(pool: PgPool) {
        run_migrations(&pool).await.expect("migrations should run");
    }
}
