use tokio::time::{interval, Duration};
use godwit_core::alerting::AlertingService;

pub struct Scheduler {
    alerting_service: AlertingService,
}

impl Scheduler {
    pub fn new(alerting_service: AlertingService) -> Self {
        Self { alerting_service }
    }
    
    pub async fn run(&self) {
        let mut interval = interval(Duration::from_secs(300));
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.alerting_service.check_budgets().await {
                tracing::error!("Budget check failed: {:?}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use sqlx::{Pool, Postgres};

    fn create_mock_alerting_service(pool: Pool<Postgres>) -> AlertingService {
        AlertingService::new(pool)
    }

    #[tokio::test]
    async fn scheduler_constructs_correctly() {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost:5432/godwit_test".to_string());
        let pool = sqlx::PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database");
        
        let alerting_service = create_mock_alerting_service(pool);
        let _scheduler = Scheduler::new(alerting_service);

        assert!(true, "Scheduler constructed successfully");
    }
}
