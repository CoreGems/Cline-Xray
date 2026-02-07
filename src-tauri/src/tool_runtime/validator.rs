//! OpenAPI schema validation for ToolRuntime
//!
//! Validates requests and responses against the OpenAPI specification.

use super::{ToolRuntime, ValidationResult};

impl ToolRuntime {
    /// Validate a request against the OpenAPI schema
    pub fn validate_request(
        &self,
        operation_id: &str,
        args: &serde_json::Value,
    ) -> ValidationResult {
        let spec = self.openapi_spec.read();
        if spec.is_none() {
            let mut result = ValidationResult::valid();
            result.add_warning("OpenAPI spec not loaded, skipping validation".to_string());
            return result;
        }

        let spec = spec.as_ref().unwrap();
        let mut result = ValidationResult::valid();

        // Parse operation_id to get path and method
        let (method, path) = match self.parse_operation_id(operation_id) {
            Ok((m, p)) => (m, p),
            Err(_) => {
                result.add_error(format!("Invalid operation_id format: {}", operation_id));
                return result;
            }
        };

        // Find the operation in the spec
        let operation = match self.find_operation(spec, &path, &method) {
            Some(op) => op,
            None => {
                result.add_warning(format!(
                    "Operation {} {} not found in OpenAPI spec",
                    method.to_uppercase(),
                    path
                ));
                return result;
            }
        };

        // Validate parameters
        if let Some(parameters) = operation.get("parameters").and_then(|p| p.as_array()) {
            self.validate_parameters(parameters, args, &mut result, spec);
        }

        // Validate request body (for POST, PUT, PATCH)
        if ["post", "put", "patch"].contains(&method.as_str()) {
            if let Some(request_body) = operation.get("requestBody") {
                self.validate_request_body(request_body, args, &mut result, spec);
            }
        }

        result
    }

    /// Validate a response against the OpenAPI schema
    pub fn validate_response(
        &self,
        operation_id: &str,
        response: &serde_json::Value,
    ) -> ValidationResult {
        let spec = self.openapi_spec.read();
        if spec.is_none() {
            let mut result = ValidationResult::valid();
            result.add_warning("OpenAPI spec not loaded, skipping validation".to_string());
            return result;
        }

        let spec = spec.as_ref().unwrap();
        let mut result = ValidationResult::valid();

        // Parse operation_id to get path and method
        let (method, path) = match self.parse_operation_id(operation_id) {
            Ok((m, p)) => (m, p),
            Err(_) => {
                result.add_error(format!("Invalid operation_id format: {}", operation_id));
                return result;
            }
        };

        // Find the operation in the spec
        let operation = match self.find_operation(spec, &path, &method) {
            Some(op) => op,
            None => {
                result.add_warning(format!(
                    "Operation {} {} not found in OpenAPI spec",
                    method.to_uppercase(),
                    path
                ));
                return result;
            }
        };

        // Validate 200 response schema
        if let Some(responses) = operation.get("responses").and_then(|r| r.as_object()) {
            if let Some(ok_response) = responses.get("200").or_else(|| responses.get("201")) {
                if let Some(content) = ok_response.get("content") {
                    if let Some(json_content) = content.get("application/json") {
                        if let Some(schema) = json_content.get("schema") {
                            self.validate_value_against_schema(
                                response,
                                schema,
                                "response",
                                &mut result,
                                spec,
                            );
                        }
                    }
                }
            }
        }

        result
    }

    /// Find an operation in the OpenAPI spec
    fn find_operation<'a>(
        &self,
        spec: &'a serde_json::Value,
        path: &str,
        method: &str,
    ) -> Option<&'a serde_json::Value> {
        spec.get("paths")?
            .get(path)?
            .get(method)
    }

    /// Validate parameters against schema
    fn validate_parameters(
        &self,
        parameters: &[serde_json::Value],
        args: &serde_json::Value,
        result: &mut ValidationResult,
        spec: &serde_json::Value,
    ) {
        let args_obj = args.as_object();

        for param in parameters {
            let name = param.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let required = param.get("required").and_then(|r| r.as_bool()).unwrap_or(false);
            let schema = param.get("schema");

            // Check if required parameter is present
            let value = args_obj.and_then(|obj| obj.get(name));
            
            if required && value.is_none() {
                result.add_error(format!("Missing required parameter: {}", name));
                continue;
            }

            // Validate value against schema
            if let (Some(value), Some(schema)) = (value, schema) {
                self.validate_value_against_schema(value, schema, name, result, spec);
            }
        }
    }

    /// Validate request body against schema
    fn validate_request_body(
        &self,
        request_body: &serde_json::Value,
        args: &serde_json::Value,
        result: &mut ValidationResult,
        spec: &serde_json::Value,
    ) {
        let schema = request_body
            .get("content")
            .and_then(|c| c.get("application/json"))
            .and_then(|j| j.get("schema"));

        if let Some(schema) = schema {
            // Resolve $ref if present
            let resolved_schema = if let Some(ref_path) = schema.get("$ref").and_then(|r| r.as_str()) {
                self.resolve_ref(spec, ref_path)
            } else {
                Some(schema)
            };

            if let Some(resolved_schema) = resolved_schema {
                // Check required fields
                if let Some(required) = resolved_schema.get("required").and_then(|r| r.as_array()) {
                    let args_obj = args.as_object();
                    for req_field in required {
                        if let Some(field_name) = req_field.as_str() {
                            if args_obj.map(|obj| !obj.contains_key(field_name)).unwrap_or(true) {
                                result.add_error(format!("Missing required field: {}", field_name));
                            }
                        }
                    }
                }

                // Validate against schema
                self.validate_value_against_schema(args, resolved_schema, "body", result, spec);
            }
        }
    }

    /// Validate a value against a JSON schema
    fn validate_value_against_schema(
        &self,
        value: &serde_json::Value,
        schema: &serde_json::Value,
        path: &str,
        result: &mut ValidationResult,
        spec: &serde_json::Value,
    ) {
        // Resolve $ref if present
        let schema = if let Some(ref_path) = schema.get("$ref").and_then(|r| r.as_str()) {
            match self.resolve_ref(spec, ref_path) {
                Some(s) => s,
                None => {
                    result.add_warning(format!("Could not resolve $ref: {}", ref_path));
                    return;
                }
            }
        } else {
            schema
        };

        // Get expected type
        let expected_type = schema.get("type").and_then(|t| t.as_str());

        match expected_type {
            Some("string") => {
                if !value.is_string() && !value.is_null() {
                    result.add_error(format!("{}: expected string, got {:?}", path, value));
                }
                // Check enum values
                if let Some(enum_values) = schema.get("enum").and_then(|e| e.as_array()) {
                    if let Some(str_val) = value.as_str() {
                        let valid = enum_values.iter().any(|v| v.as_str() == Some(str_val));
                        if !valid {
                            result.add_error(format!(
                                "{}: value '{}' not in enum {:?}",
                                path, str_val, enum_values
                            ));
                        }
                    }
                }
            }
            Some("integer") | Some("number") => {
                if !value.is_number() && !value.is_null() {
                    result.add_error(format!("{}: expected number, got {:?}", path, value));
                }
                // Check min/max
                if let Some(num) = value.as_f64() {
                    if let Some(min) = schema.get("minimum").and_then(|m| m.as_f64()) {
                        if num < min {
                            result.add_error(format!("{}: {} is less than minimum {}", path, num, min));
                        }
                    }
                    if let Some(max) = schema.get("maximum").and_then(|m| m.as_f64()) {
                        if num > max {
                            result.add_error(format!("{}: {} is greater than maximum {}", path, num, max));
                        }
                    }
                }
            }
            Some("boolean") => {
                if !value.is_boolean() && !value.is_null() {
                    result.add_error(format!("{}: expected boolean, got {:?}", path, value));
                }
            }
            Some("array") => {
                if !value.is_array() && !value.is_null() {
                    result.add_error(format!("{}: expected array, got {:?}", path, value));
                }
                // Validate array items
                if let (Some(arr), Some(items_schema)) = (value.as_array(), schema.get("items")) {
                    for (i, item) in arr.iter().enumerate() {
                        let item_path = format!("{}[{}]", path, i);
                        self.validate_value_against_schema(item, items_schema, &item_path, result, spec);
                    }
                }
            }
            Some("object") => {
                if !value.is_object() && !value.is_null() {
                    result.add_error(format!("{}: expected object, got {:?}", path, value));
                }
                // Validate object properties
                if let (Some(obj), Some(properties)) = (value.as_object(), schema.get("properties").and_then(|p| p.as_object())) {
                    for (prop_name, prop_schema) in properties {
                        if let Some(prop_value) = obj.get(prop_name) {
                            let prop_path = format!("{}.{}", path, prop_name);
                            self.validate_value_against_schema(prop_value, prop_schema, &prop_path, result, spec);
                        }
                    }
                }
            }
            None => {
                // No type specified, skip validation
                result.add_warning(format!("{}: no type specified in schema", path));
            }
            Some(t) => {
                result.add_warning(format!("{}: unknown type '{}'", path, t));
            }
        }
    }

    /// Resolve a $ref pointer in the OpenAPI spec
    fn resolve_ref<'a>(&self, spec: &'a serde_json::Value, ref_path: &str) -> Option<&'a serde_json::Value> {
        if !ref_path.starts_with("#/") {
            return None;
        }
        let parts: Vec<&str> = ref_path[2..].split('/').collect();
        let mut current = spec;
        for part in parts {
            current = current.get(part)?;
        }
        Some(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use std::sync::Arc;

    fn create_test_runtime_with_spec() -> Arc<ToolRuntime> {
        let state = AppState::new(
            "test-token".to_string(),
            "https://jira.test".to_string(),
            "test@test.com".to_string(),
            "api-token".to_string(),
            "gemini-key".to_string(),
        );
        let runtime = ToolRuntime::new(state);
        
        // Set a minimal OpenAPI spec
        runtime.set_openapi_spec(serde_json::json!({
            "paths": {
                "/jira/list": {
                    "get": {
                        "parameters": [
                            {
                                "name": "jql",
                                "in": "query",
                                "required": false,
                                "schema": {"type": "string"}
                            },
                            {
                                "name": "maxResults",
                                "in": "query",
                                "required": false,
                                "schema": {"type": "integer", "minimum": 1, "maximum": 1000}
                            }
                        ],
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "object",
                                            "properties": {
                                                "issues": {"type": "array"},
                                                "total": {"type": "integer"}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }));
        
        runtime
    }

    #[test]
    fn test_validate_request() {
        let runtime = create_test_runtime_with_spec();
        
        let args = serde_json::json!({
            "jql": "assignee = currentUser()",
            "maxResults": 50
        });
        
        let result = runtime.validate_request("get_jira_list", &args);
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_request_invalid_type() {
        let runtime = create_test_runtime_with_spec();
        
        let args = serde_json::json!({
            "maxResults": "not a number"
        });
        
        let result = runtime.validate_request("get_jira_list", &args);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("expected number")));
    }

    #[test]
    fn test_validate_request_out_of_range() {
        let runtime = create_test_runtime_with_spec();
        
        let args = serde_json::json!({
            "maxResults": 5000
        });
        
        let result = runtime.validate_request("get_jira_list", &args);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("greater than maximum")));
    }
}
