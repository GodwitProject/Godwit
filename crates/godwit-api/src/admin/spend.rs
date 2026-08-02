use axum::{routing::get, Json, Router};
use godwit_core::Usage;
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::state::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/spend", get(|| async { Json(serde_json::json!({"data": []})) }))
}

pub fn compute_cost(usage: &Usage, input_price: Decimal, output_price: Decimal) -> Decimal {
    let input = Decimal::from(usage.prompt_tokens) * input_price / Decimal::from(1000);
    let output = Decimal::from(usage.completion_tokens) * output_price / Decimal::from(1000);
    input + output
}

#[cfg(test)]
mod tests {
    use super::*;
    use godwit_core::Usage;
    use rust_decimal_macros::dec;

    #[test]
    fn cost_computation() {
        let usage = Usage {
            prompt_tokens: 1000,
            completion_tokens: 500,
            total_tokens: 1500,
        };
        let cost = compute_cost(&usage, dec!(0.005), dec!(0.015));
        assert_eq!(cost, dec!(0.0125));
    }
}
