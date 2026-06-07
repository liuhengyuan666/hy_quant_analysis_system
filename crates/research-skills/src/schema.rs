use serde_json::Value;

/// Lightweight JSON schema validation for skill outputs.
/// Checks required fields, types, and enum constraints.
pub struct SchemaValidator {
    schema: Value,
}

impl SchemaValidator {
    pub fn new(schema: Value) -> Self {
        Self { schema }
    }

    /// Validate a skill output JSON against the schema.
    pub fn validate(&self, output: &Value) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        self.validate_value(output, &self.schema, "", &mut errors);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_value(
        &self,
        value: &Value,
        schema: &Value,
        path: &str,
        errors: &mut Vec<String>,
    ) {
        let schema_type = schema.get("type").and_then(|v| v.as_str());

        match schema_type {
            Some("object") => {
                if let Value::Object(map) = value {
                    // Check required fields
                    if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
                        for req in required {
                            if let Some(field) = req.as_str() {
                                if !map.contains_key(field) {
                                    errors.push(format!("{}: missing required field '{}'", path, field));
                                }
                            }
                        }
                    }
                    // Validate properties
                    if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
                        for (key, prop_schema) in props {
                            if let Some(prop_value) = map.get(key) {
                                let prop_path = if path.is_empty() {
                                    key.clone()
                                } else {
                                    format!("{}.{}", path, key)
                                };
                                self.validate_value(prop_value, prop_schema, &prop_path, errors);
                            }
                        }
                    }
                } else {
                    errors.push(format!("{}: expected object, got {}", path, json_type_name(value)));
                }
            }
            Some("array") => {
                if let Value::Array(arr) = value {
                    if let Some(item_schema) = schema.get("items") {
                        for (idx, item) in arr.iter().enumerate() {
                            let item_path = format!("{}[{}]", path, idx);
                            self.validate_value(item, item_schema, &item_path, errors);
                        }
                    }
                } else {
                    errors.push(format!("{}: expected array, got {}", path, json_type_name(value)));
                }
            }
            Some("string") => {
                if let Value::String(s) = value {
                    // Check enum constraints
                    if let Some(enum_values) = schema.get("enum").and_then(|v| v.as_array()) {
                        let valid: Vec<String> = enum_values.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
                        if !valid.contains(s) {
                            errors.push(format!("{}: value '{}' not in enum {:?}", path, s, valid));
                        }
                    }
                } else {
                    errors.push(format!("{}: expected string, got {}", path, json_type_name(value)));
                }
            }
            Some("number") => {
                if !value.is_number() {
                    errors.push(format!("{}: expected number, got {}", path, json_type_name(value)));
                } else {
                    if let Some(min) = schema.get("minimum").and_then(|v| v.as_f64()) {
                        if let Some(v) = value.as_f64() {
                            if v < min {
                                errors.push(format!("{}: value {} below minimum {}", path, v, min));
                            }
                        }
                    }
                    if let Some(max) = schema.get("maximum").and_then(|v| v.as_f64()) {
                        if let Some(v) = value.as_f64() {
                            if v > max {
                                errors.push(format!("{}: value {} above maximum {}", path, v, max));
                            }
                        }
                    }
                }
            }
            Some("boolean") => {
                if !value.is_boolean() {
                    errors.push(format!("{}: expected boolean, got {}", path, json_type_name(value)));
                }
            }
            _ => {
                // Unknown or missing type: permissive
            }
        }
    }
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_regime_output() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["regime_state", "confidence"],
            "properties": {
                "regime_state": {
                    "type": "string",
                    "enum": ["risk_on", "neutral", "risk_off", "de_risk"]
                },
                "confidence": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 1
                }
            }
        });

        let validator = SchemaValidator::new(schema);

        // Valid output
        let valid = serde_json::json!({
            "regime_state": "neutral",
            "confidence": 0.8
        });
        assert!(validator.validate(&valid).is_ok());

        // Missing required field
        let missing = serde_json::json!({
            "regime_state": "neutral"
        });
        assert!(validator.validate(&missing).is_err());

        // Invalid enum value
        let invalid_enum = serde_json::json!({
            "regime_state": "bullish",
            "confidence": 0.8
        });
        assert!(validator.validate(&invalid_enum).is_err());

        // Out of range
        let out_of_range = serde_json::json!({
            "regime_state": "neutral",
            "confidence": 1.5
        });
        assert!(validator.validate(&out_of_range).is_err());
    }
}
