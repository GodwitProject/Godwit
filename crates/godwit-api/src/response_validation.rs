use godwit_core::JsonSchema;
use jsonschema::JSONSchema;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("invalid json: {0}")]
    InvalidJson(String),
    #[error("schema validation failed: {0}")]
    SchemaViolation(String),
    #[error("no content to validate")]
    NoContent,
}

pub fn validate_response(
    content: &str,
    json_schema: &JsonSchema,
) -> Result<(), ValidationError> {
    let schema_value = json_schema
        .schema
        .as_ref()
        .ok_or_else(|| ValidationError::SchemaViolation("schema is required".to_string()))?;

    let parsed: Value = serde_json::from_str(content).map_err(|e| {
        ValidationError::InvalidJson(format!("failed to parse response as json: {}", e))
    })?;

    let compiled_schema = JSONSchema::compile(schema_value).map_err(|e| {
        ValidationError::SchemaViolation(format!("invalid schema definition: {}", e))
    })?;

    let result = compiled_schema.validate(&parsed);
    if let Err(errors) = result {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        return Err(ValidationError::SchemaViolation(format!(
            "response does not match schema: {}",
            error_messages.join("; ")
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_valid_json_passes() {
        let schema = JsonSchema {
            name: "test".to_string(),
            schema: Some(json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "age": { "type": "number" }
                },
                "required": ["name", "age"]
            })),
            strict: None,
        };

        let content = r#"{"name": "John", "age": 30}"#;
        let result = validate_response(content, &schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_json_fails() {
        let schema = JsonSchema {
            name: "test".to_string(),
            schema: Some(json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                },
                "required": ["name"]
            })),
            strict: None,
        };

        let content = r#"{"age": 30}"#;
        let result = validate_response(content, &schema);
        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::SchemaViolation(_) => {}
            _ => panic!("expected schema violation"),
        }
    }

    #[test]
    fn test_malformed_json_fails() {
        let schema = JsonSchema {
            name: "test".to_string(),
            schema: Some(json!({ "type": "object" })),
            strict: None,
        };

        let content = r#"{"name": "John"#;
        let result = validate_response(content, &schema);
        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::InvalidJson(_) => {}
            _ => panic!("expected invalid json error"),
        }
    }

    #[test]
    fn test_missing_schema_fails() {
        let schema = JsonSchema {
            name: "test".to_string(),
            schema: None,
            strict: None,
        };

        let content = r#"{"name": "John"}"#;
        let result = validate_response(content, &schema);
        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::SchemaViolation(_) => {}
            _ => panic!("expected schema violation"),
        }
    }
}
