use axum::{routing::get, Json, Router};
use godwit_core::Capability;
use godwit_db::models::Model;
use godwit_providers::UsageReport;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;

use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route(
        "/spend",
        get(|| async { Json(serde_json::json!({"data": []})) }),
    )
}

pub fn compute_cost(model: &Model, usage: &UsageReport) -> Option<Decimal> {
    let pricing = model.pricing.as_object()?;
    match Capability::from_str(&model.capability).ok()? {
        Capability::Chat => {
            let input = Decimal::from(usage.prompt_tokens.unwrap_or(0)) * Decimal::from_str(pricing.get("input_per_1k")?.as_str()?).ok()? / Decimal::from(1000);
            let output = Decimal::from(usage.completion_tokens.unwrap_or(0)) * Decimal::from_str(pricing.get("output_per_1k")?.as_str()?).ok()? / Decimal::from(1000);
            Some(input + output)
        }
        _ => None,
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
            organization_id: uuid::Uuid::nil(),
            public_id: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            provider_profile_id: uuid::Uuid::nil(),
            provider_model_id: "gpt-4o".to_string(),
            capability: "chat".to_string(),
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
        let cost = compute_cost(&model, &usage).expect("cost");
        assert_eq!(cost, dec!(0.0125));
    }
}
