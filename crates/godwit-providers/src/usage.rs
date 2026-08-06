use crate::adapter::UsageReport;
use godwit_core::Capability;
use rust_decimal::Decimal;
use std::str::FromStr;

const INPUT_PRICE_PER_MILLION: &str = "input_price_per_million";
const OUTPUT_PRICE_PER_MILLION: &str = "output_price_per_million";
const IMAGE_PRICE_PER_IMAGE: &str = "image_price_per_image";
const AUDIO_TTS_PRICE_PER_CHAR: &str = "tts_price_per_character";
const AUDIO_STT_PRICE_PER_SEC: &str = "stt_price_per_second";

pub fn compute_chat_cost(pricing: &serde_json::Value, usage: &UsageReport) -> Option<Decimal> {
    let input_price = decimal_field(pricing, INPUT_PRICE_PER_MILLION)?;
    let output_price = decimal_field(pricing, OUTPUT_PRICE_PER_MILLION)?;
    let input_tokens = Decimal::from(i64::from(usage.prompt_tokens?));
    let output_tokens = Decimal::from(i64::from(usage.completion_tokens?));
    let cost = (input_tokens * input_price + output_tokens * output_price)
        / Decimal::from(1_000_000);
    Some(cost)
}

pub fn compute_embedding_cost(pricing: &serde_json::Value, usage: &UsageReport) -> Option<Decimal> {
    let input_price = decimal_field(pricing, INPUT_PRICE_PER_MILLION)?;
    let tokens = Decimal::from(usage.embedding_tokens?);
    let cost = tokens * input_price / Decimal::from(1_000_000);
    Some(cost)
}

pub fn compute_image_cost(pricing: &serde_json::Value, usage: &UsageReport) -> Option<Decimal> {
    let image_price = decimal_field(pricing, IMAGE_PRICE_PER_IMAGE)?;
    let count = Decimal::from(usage.image_count?);
    let cost = count * image_price;
    Some(cost)
}

pub fn compute_audio_tts_cost(pricing: &serde_json::Value, usage: &UsageReport) -> Option<Decimal> {
    let price_per_char = decimal_field(pricing, AUDIO_TTS_PRICE_PER_CHAR)?;
    let chars = Decimal::from(usage.tts_characters?);
    let cost = chars * price_per_char;
    Some(cost)
}

pub fn compute_audio_stt_cost(pricing: &serde_json::Value, usage: &UsageReport) -> Option<Decimal> {
    let price_per_sec = decimal_field(pricing, AUDIO_STT_PRICE_PER_SEC)?;
    let seconds = Decimal::from_f64_retain(usage.audio_seconds?)?;
    let cost = seconds * price_per_sec;
    Some(cost)
}

pub fn compute_cost(pricing: &serde_json::Value, capability: Capability, usage: &UsageReport) -> Option<Decimal> {
    match capability {
        Capability::Chat => compute_chat_cost(pricing, usage),
        Capability::Embedding => compute_embedding_cost(pricing, usage),
        Capability::ImageGeneration | Capability::ImageEdit => compute_image_cost(pricing, usage),
        Capability::AudioTts => compute_audio_tts_cost(pricing, usage),
        Capability::AudioStt => compute_audio_stt_cost(pricing, usage),
        _ => None,
    }
}

fn decimal_field(pricing: &serde_json::Value, key: &str) -> Option<Decimal> {
    let value = pricing.get(key)?;
    if let Some(s) = value.as_str() {
        return Decimal::from_str_exact(s).ok();
    }
    let s = value.to_string();
    Decimal::from_str_exact(&s).ok()
}

pub fn chat_usage_report(usage: &Option<godwit_core::Usage>) -> UsageReport {
    let Some(u) = usage else {
        return UsageReport::default();
    };
    UsageReport {
        prompt_tokens: Some(u.prompt_tokens),
        completion_tokens: Some(u.completion_tokens),
        cache_read_tokens: u.prompt_tokens_details.as_ref().and_then(|d| d.cached_tokens),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn pricing() -> serde_json::Value {
        serde_json::json!({
            "input_price_per_million": 2.5,
            "output_price_per_million": 10.0,
            "image_price_per_image": 0.005,
            "tts_price_per_character": 0.00001,
            "stt_price_per_second": 0.0001,
        })
    }

    #[test]
    fn chat_cost_matches_expected() {
        let usage = UsageReport {
            prompt_tokens: Some(1_000_000),
            completion_tokens: Some(500_000),
            ..Default::default()
        };
        let cost = compute_chat_cost(&pricing(), &usage).expect("cost");
        assert_eq!(cost, dec!(7.5));
    }

    #[test]
    fn embedding_cost() {
        let usage = UsageReport {
            embedding_tokens: Some(2_000_000),
            ..Default::default()
        };
        let cost = compute_embedding_cost(&pricing(), &usage).expect("cost");
        assert_eq!(cost, dec!(5.0));
    }

    #[test]
    fn image_cost() {
        let usage = UsageReport {
            image_count: Some(4),
            ..Default::default()
        };
        let cost = compute_image_cost(&pricing(), &usage).expect("cost");
        assert_eq!(cost, dec!(0.02));
    }

    #[test]
    fn audio_tts_cost() {
        let usage = UsageReport {
            tts_characters: Some(1_000_000),
            ..Default::default()
        };
        let cost = compute_audio_tts_cost(&pricing(), &usage).expect("cost");
        assert_eq!(cost, dec!(10.0));
    }

    #[test]
    fn audio_stt_cost() {
        let usage = UsageReport {
            audio_seconds: Some(60.0),
            ..Default::default()
        };
        let cost = compute_audio_stt_cost(&pricing(), &usage).expect("cost");
        assert_eq!(cost, dec!(0.006));
    }

    #[test]
    fn dispatch_by_capability() {
        let usage = UsageReport {
            prompt_tokens: Some(1_000_000),
            completion_tokens: Some(0),
            ..Default::default()
        };
        assert_eq!(
            compute_cost(&pricing(), Capability::Chat, &usage),
            Some(dec!(2.5))
        );
        let usage = UsageReport {
            embedding_tokens: Some(1_000_000),
            ..Default::default()
        };
        assert_eq!(
            compute_cost(&pricing(), Capability::Embedding, &usage),
            Some(dec!(2.5))
        );
        let usage = UsageReport {
            image_count: Some(2),
            ..Default::default()
        };
        assert_eq!(
            compute_cost(&pricing(), Capability::ImageGeneration, &usage),
            Some(dec!(0.01))
        );
        let usage = UsageReport {
            tts_characters: Some(100),
            ..Default::default()
        };
        assert_eq!(
            compute_cost(&pricing(), Capability::AudioTts, &usage),
            Some(dec!(0.001))
        );
        let usage = UsageReport {
            audio_seconds: Some(10.0),
            ..Default::default()
        };
        assert_eq!(
            compute_cost(&pricing(), Capability::AudioStt, &usage),
            Some(dec!(0.001))
        );
    }
}
