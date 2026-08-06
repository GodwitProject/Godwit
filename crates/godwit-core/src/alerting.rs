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
}
