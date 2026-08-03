use axum::{routing::get, Json, Router};
use godwit_core::Capability;
use godwit_db::models::Model;
use godwit_providers::UsageReport;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;

use crate::state::AppState;

const PRICING_INPUT_PER_1K: &str = "input_per_1k";
const PRICING_OUTPUT_PER_1K: &str = "output_per_1k";

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route(
        "/spend",
        get(|| async { Json(serde_json::json!({"data": []})) }),
    )
}

pub fn compute_cost(model: &Model, capability: Capability, usage: &UsageReport) -> Option<Decimal> {
    let pricing = model.pricing.as_object()?;
    match capability {
        Capability::Chat => {
            let input_price = pricing.get(PRICING_INPUT_PER_1K)?;
            let output_price = pricing.get(PRICING_OUTPUT_PER_1K)?;
            let input_rate = Decimal::from_str(input_price.as_str()?)
                .inspect_err(|e| tracing::warn!(%e, "malformed input_per_1k pricing"))
                .ok()?;
            let output_rate = Decimal::from_str(output_price.as_str()?)
                .inspect_err(|e| tracing::warn!(%e, "malformed output_per_1k pricing"))
                .ok()?;
            let input =
                Decimal::from(usage.prompt_tokens.unwrap_or(0)) * input_rate / Decimal::from(1000);
            let output = Decimal::from(usage.completion_tokens.unwrap_or(0)) * output_rate
                / Decimal::from(1000);
            Some(input + output)
        }
        _ => {
            tracing::warn!(
                capability = %capability,
                "cost computation not supported for capability"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn cost_computation() {
        let model = Model {
            id: uuid::Uuid::nil(),
            public_id: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            provider_profile_id: uuid::Uuid::nil(),
            provider_model_id: "gpt-4o".to_string(),
            capabilities: vec!["chat".to_string()],
            pricing: serde_json::json!({
                "input_per_1k": "0.005",
                "output_per_1k": "0.015"
            }),
            config: serde_json::json!({}),
            created_at: chrono::Utc::now(),
        };
        let usage = UsageReport {
            prompt_tokens: Some(1000),
            completion_tokens: Some(500),
            ..Default::default()
        };
        let cost = compute_cost(&model, Capability::Chat, &usage).expect("cost");
        assert_eq!(cost, dec!(0.0125));
    }
}
