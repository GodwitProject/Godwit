use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchWebhookPayload {
    pub batch_id: String,
    pub status: String,
    pub total_requests: usize,
    pub completed_requests: usize,
    pub failed_requests: usize,
    pub total_cost_usd: rust_decimal::Decimal,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub completed_at: DateTime<Utc>,
}

pub struct WebhookSender {
    client: Client,
}

impl WebhookSender {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    pub async fn send_webhook(
        &self,
        webhook_url: &str,
        payload: &BatchWebhookPayload,
    ) -> Result<(), WebhookError> {
        let response = self
            .client
            .post(webhook_url)
            .json(payload)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(WebhookError::HttpStatus(response.status()));
        }

        Ok(())
    }
}

impl Default for WebhookSender {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("HTTP status: {0}")]
    HttpStatus(reqwest::StatusCode),
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_webhook_sender_creation() {
        let _sender = WebhookSender::new();
    }

    #[test]
    fn test_webhook_payload_serialization() {
        let payload = BatchWebhookPayload {
            batch_id: "batch-123".to_string(),
            status: "completed".to_string(),
            total_requests: 10,
            completed_requests: 8,
            failed_requests: 2,
            total_cost_usd: Decimal::from(150),
            total_input_tokens: 5000,
            total_output_tokens: 3000,
            completed_at: Utc::now(),
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("batch-123"));
        assert!(json.contains("completed"));
        assert!(json.contains("total_requests"));
        assert!(json.contains("total_cost_usd"));
    }

    #[tokio::test]
    async fn test_send_webhook_invalid_url() {
        let sender = WebhookSender::new();
        let payload = BatchWebhookPayload {
            batch_id: "test".to_string(),
            status: "completed".to_string(),
            total_requests: 1,
            completed_requests: 1,
            failed_requests: 0,
            total_cost_usd: Decimal::ZERO,
            total_input_tokens: 100,
            total_output_tokens: 50,
            completed_at: Utc::now(),
        };

        let result = sender.send_webhook("http://invalid-url-that-does-not-exist.local", &payload).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_webhook_payload_all_fields() {
        let completed_at = Utc::now();
        let payload = BatchWebhookPayload {
            batch_id: "batch-456".to_string(),
            status: "completed".to_string(),
            total_requests: 100,
            completed_requests: 95,
            failed_requests: 5,
            total_cost_usd: Decimal::from(250),
            total_input_tokens: 50000,
            total_output_tokens: 30000,
            completed_at,
        };

        assert_eq!(payload.batch_id, "batch-456");
        assert_eq!(payload.status, "completed");
        assert_eq!(payload.total_requests, 100);
        assert_eq!(payload.completed_requests, 95);
        assert_eq!(payload.failed_requests, 5);
        assert_eq!(payload.total_cost_usd, Decimal::from(250));
        assert_eq!(payload.total_input_tokens, 50000);
        assert_eq!(payload.total_output_tokens, 30000);
    }
}
