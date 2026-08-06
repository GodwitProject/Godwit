use reqwest::Client;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use thiserror::Error;

#[derive(Debug, Clone, Serialize)]
pub struct BudgetAlertPayload {
    pub event_type: String,
    pub org_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub api_key_id: Option<Uuid>,
    pub current_spend: f64,
    pub budget: f64,
    pub threshold_percent: u32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum AlertingError {
    #[error("database error: {0}")]
    Database(String),
    #[error("http error: {0}")]
    Http(String),
    #[error("webhook delivery failed: {0}")]
    WebhookFailed(String),
}

impl From<sqlx::Error> for AlertingError {
    fn from(err: sqlx::Error) -> Self {
        AlertingError::Database(err.to_string())
    }
}

impl From<reqwest::Error> for AlertingError {
    fn from(err: reqwest::Error) -> Self {
        AlertingError::Http(err.to_string())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct AlertingConfig {
    id: i64,
    org_id: Option<Uuid>,
    team_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
    budget_threshold_percent: i32,
    webhook_url: String,
    enabled: bool,
}

pub struct AlertingService {
    http_client: Client,
    db_pool: PgPool,
}

impl AlertingService {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            http_client: Client::new(),
            db_pool,
        }
    }

    async fn get_alerting_configs(&self) -> Result<Vec<AlertingConfig>, AlertingError> {
        let configs = sqlx::query_as::<_, AlertingConfig>(
            "SELECT id, org_id, team_id, api_key_id, budget_threshold_percent, webhook_url, enabled
             FROM alerting_config
             WHERE enabled = true"
        )
        .fetch_all(&self.db_pool)
        .await?;
        Ok(configs)
    }

    async fn get_current_spend(
        &self,
        org_id: Option<Uuid>,
        team_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<f64, AlertingError> {
        let result = sqlx::query_scalar::<_, Option<rust_decimal::Decimal>>(
            "SELECT COALESCE(SUM(cost_usd), 0)
             FROM request_logs
             WHERE ($1::uuid IS NULL OR org_id = $1)
               AND ($2::uuid IS NULL OR team_id = $2)
               AND ($3::uuid IS NULL OR api_key_id = $3)"
        )
        .bind(org_id)
        .bind(team_id)
        .bind(api_key_id)
        .fetch_one(&self.db_pool)
        .await?;
        
        Ok(result.unwrap_or_default().to_string().parse::<f64>().unwrap_or(0.0))
    }

    async fn get_budget(
        &self,
        org_id: Option<Uuid>,
        team_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<f64, AlertingError> {
        let budget = if let Some(key_id) = api_key_id {
            sqlx::query_scalar::<_, Option<rust_decimal::Decimal>>(
                "SELECT budget_limit_usd FROM api_keys WHERE id = $1"
            )
            .bind(key_id)
            .fetch_one(&self.db_pool)
            .await?
        } else if let Some(tid) = team_id {
            sqlx::query_scalar::<_, Option<rust_decimal::Decimal>>(
                "SELECT budget_usd FROM teams WHERE id = $1"
            )
            .bind(tid)
            .fetch_one(&self.db_pool)
            .await?
        } else if let Some(oid) = org_id {
            sqlx::query_scalar::<_, Option<rust_decimal::Decimal>>(
                "SELECT budget_usd FROM organizations WHERE id = $1"
            )
            .bind(oid)
            .fetch_one(&self.db_pool)
            .await?
        } else {
            None
        };
        
        Ok(budget.unwrap_or_default().to_string().parse::<f64>().unwrap_or(0.0))
    }

    pub async fn check_budgets(&self) -> Result<(), AlertingError> {
        let configs = self.get_alerting_configs().await?;
        
        for config in configs {
            let current_spend = self.get_current_spend(
                config.org_id,
                config.team_id,
                config.api_key_id,
            ).await?;
            
            let budget = self.get_budget(
                config.org_id,
                config.team_id,
                config.api_key_id,
            ).await?;
            
            if budget <= 0.0 {
                continue;
            }
            
            let threshold = budget * (config.budget_threshold_percent as f64 / 100.0);
            
            if current_spend >= threshold {
                let event_type = if current_spend >= budget {
                    "budget_100"
                } else {
                    "budget_80"
                };
                
                self.send_webhook(
                    event_type,
                    &config.webhook_url,
                    BudgetAlertPayload {
                        event_type: event_type.to_string(),
                        org_id: config.org_id,
                        team_id: config.team_id,
                        api_key_id: config.api_key_id,
                        current_spend,
                        budget,
                        threshold_percent: config.budget_threshold_percent as u32,
                        timestamp: Utc::now(),
                    },
                ).await?;
            }
        }
        
        Ok(())
    }

    async fn send_webhook(
        &self,
        _event_type: &str,
        _url: &str,
        _payload: BudgetAlertPayload,
    ) -> Result<(), AlertingError> {
        Ok(())
    }
}
