use regex::Regex;
use std::collections::HashMap;

pub struct PiiPattern {
    pub name: String,
    pub pattern: Regex,
    pub replacement: String,
}

pub struct PiiMasker {
    patterns: Vec<PiiPattern>,
    mask_map: HashMap<String, Vec<(usize, usize, String, String)>>,
}

impl PiiMasker {
    pub fn new(patterns: Vec<PiiPattern>) -> Self {
        Self {
            patterns,
            mask_map: HashMap::new(),
        }
    }

    pub fn mask(&mut self, text: &str, request_id: &str) -> String {
        let mut masked = text.to_string();
        let mut replacements: Vec<(usize, usize, String, String)> = Vec::new();

        for pattern in &self.patterns {
            let matches: Vec<_> = pattern.pattern.find_iter(&masked).map(|m| (m.start(), m.end())).collect();
            let mut offset = 0;
            for (match_start, match_end) in matches {
                let start = match_start - offset;
                let end = match_end - offset;
                let original = masked[start..end].to_string();
                let placeholder = pattern.replacement.clone();

                masked.replace_range(start..end, &placeholder);
                offset += end - start - placeholder.len();

                replacements.push((start, end, original, placeholder));
            }
        }

        self.mask_map.insert(request_id.to_string(), replacements);
        masked
    }

    pub fn unmask(&mut self, masked_text: &str, request_id: &str) -> String {
        if let Some(replacements) = self.mask_map.remove(request_id) {
            let mut unmasked = masked_text.to_string();
            let mut offset: isize = 0;

            for (start, _end, original, placeholder) in replacements {
                let placeholder_start = (start as isize + offset) as usize;
                let placeholder_len = placeholder.len() as isize;
                
                unmasked.replace_range(placeholder_start..placeholder_start + placeholder_len as usize, &original);
                offset += original.len() as isize - placeholder_len;
            }

            unmasked
        } else {
            masked_text.to_string()
        }
    }
}

pub fn default_patterns() -> Vec<PiiPattern> {
    vec![
        PiiPattern {
            name: "phone".to_string(),
            pattern: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
            replacement: "[SSN]".to_string(),
        },
        PiiPattern {
            name: "credit_card".to_string(),
            pattern: Regex::new(r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b").unwrap(),
            replacement: "[CARD]".to_string(),
        },
        PiiPattern {
            name: "email".to_string(),
            pattern: Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
            replacement: "[EMAIL]".to_string(),
        },
        PiiPattern {
            name: "phone".to_string(),
            pattern: Regex::new(r"\b\d{3}[-.\s]?\d{3}[-.\s]?\d{4}\b").unwrap(),
            replacement: "[PHONE]".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_email() {
        let mut masker = PiiMasker::new(default_patterns());
        let masked = masker.mask("Contact me at test@example.com", "req1");
        assert_eq!(masked, "Contact me at [EMAIL]");
    }

    #[test]
    fn test_mask_phone() {
        let mut masker = PiiMasker::new(default_patterns());
        let masked = masker.mask("Call me at 555-123-4567", "req2");
        assert_eq!(masked, "Call me at [PHONE]");
    }

    #[test]
    fn test_mask_credit_card() {
        let mut masker = PiiMasker::new(default_patterns());
        let masked = masker.mask("Card: 1234-5678-9012-3456", "req3");
        assert_eq!(masked, "Card: [CARD]");
    }

    #[test]
    fn test_mask_ssn() {
        let mut masker = PiiMasker::new(default_patterns());
        let masked = masker.mask("SSN: 123-45-6789", "req4");
        assert_eq!(masked, "SSN: [SSN]");
    }

    #[test]
    fn test_mask_multiple_pii() {
        let mut masker = PiiMasker::new(default_patterns());
        let text = "Email: test@example.com, Phone: 555-123-4567, Card: 1234-5678-9012-3456";
        let masked = masker.mask(text, "req5");
        assert!(masked.contains("[EMAIL]"));
        assert!(masked.contains("[PHONE]"));
        assert!(masked.contains("[CARD]"));
    }

    #[test]
    fn test_unmask_restores_original() {
        let mut masker = PiiMasker::new(default_patterns());
        let original = "Email: test@example.com";
        let masked = masker.mask(original, "req6");
        let unmasked = masker.unmask(&masked, "req6");
        assert_eq!(unmasked, original);
    }

    #[test]
    fn test_unmask_without_request_id_returns_masked() {
        let mut masker = PiiMasker::new(default_patterns());
        let original = "Email: test@example.com";
        let masked = masker.mask(original, "req7");
        let unmasked = masker.unmask(&masked, "nonexistent");
        assert_eq!(unmasked, masked);
    }

    #[test]
    fn test_mask_multiple_same_type() {
        let mut masker = PiiMasker::new(default_patterns());
        let text = "Email: a@test.com and b@test.org";
        let masked = masker.mask(text, "req8");
        assert_eq!(masked, "Email: [EMAIL] and [EMAIL]");
    }
}
