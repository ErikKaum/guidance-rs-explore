use anyhow::{anyhow, Result};
use jsonschema::JSONSchema;
use regex::escape;
use serde_json::json;
use serde_json::Value;

use crate::handle_types;
use crate::types;

#[derive(Debug, Copy, Clone)]
enum SchemaKeyword {
    Properties,
    AllOf,
    AnyOf,
    OneOf,
    PrefixItems,
    Enum,
    Const,
    Ref,
    Type,
    EmptyObject,
}

pub fn build_regex_from_schema(json: &str, whitespace_pattern: Option<&str>) -> Result<String> {
    let json_value: Value = serde_json::from_str(json)?;
    let _compiled_schema = JSONSchema::compile(&json_value)
        .map_err(|e| anyhow!("Failed to compile JSON schema: {}", e))?;

    to_regex(&json_value, whitespace_pattern)
}

pub fn to_regex(json: &Value, whitespace_pattern: Option<&str>) -> Result<String> {
    let whitespace_pattern = whitespace_pattern.unwrap_or(types::WHITESPACE);

    match json {
        Value::Object(obj) => {
            let keyword = if obj.is_empty() {
                SchemaKeyword::EmptyObject
            } else {
                [
                    ("properties", SchemaKeyword::Properties),
                    ("allOf", SchemaKeyword::AllOf),
                    ("anyOf", SchemaKeyword::AnyOf),
                    ("oneOf", SchemaKeyword::OneOf),
                    ("prefixItems", SchemaKeyword::PrefixItems),
                    ("enum", SchemaKeyword::Enum),
                    ("const", SchemaKeyword::Const),
                    ("$ref", SchemaKeyword::Ref),
                    ("type", SchemaKeyword::Type),
                ]
                .iter()
                .find_map(|&(key, schema_keyword)| {
                    if obj.contains_key(key) {
                        Some(schema_keyword)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| anyhow!("Unsupported JSON Schema structure {} \nMake sure it is valid to the JSON Schema specification and check if it's supported by Outlines.\nIf it should be supported, please open an issue.", json))?
            };

            match keyword {
                SchemaKeyword::Properties => handle_properties(obj, whitespace_pattern),
                SchemaKeyword::AllOf => handle_all_of(obj, whitespace_pattern),
                SchemaKeyword::AnyOf => handle_any_of(obj, whitespace_pattern),
                SchemaKeyword::OneOf => handle_one_of(obj, whitespace_pattern),
                SchemaKeyword::PrefixItems => handle_prefix_items(obj, whitespace_pattern),
                SchemaKeyword::Enum => handle_enum(obj, whitespace_pattern),
                SchemaKeyword::Const => handle_const(obj, whitespace_pattern),
                // SchemaKeyword::Ref => handle_ref(obj, whitespace_pattern),
                SchemaKeyword::Type => handle_type(obj, whitespace_pattern),
                SchemaKeyword::EmptyObject => handle_empty_object(whitespace_pattern),
                val => Err(anyhow!("Unsupported JSON Schema keyword: {:?}", val)),
            }
        }
        _ => Err(anyhow!("Invalid JSON Schema: expected an object")),
    }
}

fn handle_properties(
    obj: &serde_json::Map<String, Value>,
    whitespace_pattern: &str,
) -> Result<String> {
    let mut regex = String::from(r"\{");

    let properties = obj
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("'properties' not found or not an object"))?;

    let required_properties = obj
        .get("required")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();

    let is_required: Vec<bool> = properties
        .keys()
        .map(|item| required_properties.contains(&item.as_str()))
        .collect();

    if is_required.iter().any(|&x| x) {
        let last_required_pos = is_required
            .iter()
            .enumerate()
            .filter(|&(_, &value)| value)
            .map(|(i, _)| i)
            .max()
            .unwrap();

        for (i, (name, value)) in properties.iter().enumerate() {
            let mut subregex = format!(
                r#"{whitespace_pattern}"{}"{}:{}"#,
                escape(name),
                whitespace_pattern,
                whitespace_pattern
            );
            subregex += &to_regex(value, Some(whitespace_pattern))?;

            if i < last_required_pos {
                subregex = format!("{}{},", subregex, whitespace_pattern);
            } else if i > last_required_pos {
                subregex = format!("{},{}", whitespace_pattern, subregex);
            }

            regex += &if is_required[i] {
                subregex
            } else {
                format!("({})?", subregex)
            };
        }
    } else {
        let mut property_subregexes = Vec::new();
        for (name, value) in properties.iter().rev() {
            let mut subregex = format!(
                r#"{whitespace_pattern}"{}"{}:{}"#,
                escape(name),
                whitespace_pattern,
                whitespace_pattern
            );

            subregex += &to_regex(value, Some(whitespace_pattern))?;
            property_subregexes.push(subregex);
        }

        let mut possible_patterns = Vec::new();
        for i in 0..property_subregexes.len() {
            let mut pattern = String::new();
            for subregex in &property_subregexes[..i] {
                pattern += &format!("({}{},)?", subregex, whitespace_pattern);
            }
            pattern += &property_subregexes[i];
            for subregex in &property_subregexes[i + 1..] {
                pattern += &format!("({},{})?", whitespace_pattern, subregex);
            }
            possible_patterns.push(pattern);
        }

        regex += &format!("({})?", possible_patterns.join("|"));
    }

    regex += &format!("{}\\}}", whitespace_pattern);

    Ok(regex)
}

fn handle_all_of(obj: &serde_json::Map<String, Value>, whitespace_pattern: &str) -> Result<String> {
    match obj.get("allOf") {
        Some(Value::Array(all_of)) => {
            let subregexes: Result<Vec<String>> = all_of
                .iter()
                .map(|t| to_regex(t, Some(whitespace_pattern)))
                .collect();

            let subregexes = subregexes?;
            let combined_regex = subregexes.join("");

            Ok(format!(r"({})", combined_regex))
        }
        _ => Err(anyhow!("'allOf' must be an array")),
    }
}

fn handle_any_of(obj: &serde_json::Map<String, Value>, whitespace_pattern: &str) -> Result<String> {
    match obj.get("anyOf") {
        Some(Value::Array(any_of)) => {
            let subregexes: Result<Vec<String>> = any_of
                .iter()
                .map(|t| to_regex(t, Some(whitespace_pattern)))
                .collect();

            let subregexes = subregexes?;

            Ok(format!(r"({})", subregexes.join("|")))
        }
        _ => Err(anyhow!("'anyOf' must be an array")),
    }
}

fn handle_one_of(obj: &serde_json::Map<String, Value>, whitespace_pattern: &str) -> Result<String> {
    match obj.get("oneOf") {
        Some(Value::Array(one_of)) => {
            let subregexes: Result<Vec<String>> = one_of
                .iter()
                .map(|t| to_regex(t, Some(whitespace_pattern)))
                .collect();

            let subregexes = subregexes?;

            let xor_patterns: Vec<String> = subregexes
                .into_iter()
                .map(|subregex| format!(r"(?:{})", subregex))
                .collect();

            Ok(format!(r"({})", xor_patterns.join("|")))
        }
        _ => Err(anyhow!("'oneOf' must be an array")),
    }
}

fn handle_prefix_items(
    obj: &serde_json::Map<String, Value>,
    whitespace_pattern: &str,
) -> Result<String> {
    match obj.get("prefixItems") {
        Some(Value::Array(prefix_items)) => {
            let element_patterns: Result<Vec<String>> = prefix_items
                .iter()
                .map(|t| to_regex(t, Some(whitespace_pattern)))
                .collect();

            let element_patterns = element_patterns?;

            let comma_split_pattern = format!("{},{}", whitespace_pattern, whitespace_pattern);
            let tuple_inner = element_patterns.join(&comma_split_pattern);

            Ok(format!(
                r"\[{whitespace_pattern}{tuple_inner}{whitespace_pattern}\]"
            ))
        }
        _ => Err(anyhow!("'prefixItems' must be an array")),
    }
}

fn handle_enum(obj: &serde_json::Map<String, Value>, _whitespace_pattern: &str) -> Result<String> {
    match obj.get("enum") {
        Some(Value::Array(enum_values)) => {
            let choices: Result<Vec<String>> = enum_values
                .iter()
                .map(|choice| match choice {
                    Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                        let json_string = serde_json::to_string(choice)?;
                        Ok(regex::escape(&json_string))
                    }
                    _ => Err(anyhow!("Unsupported data type in enum: {:?}", choice)),
                })
                .collect();

            let choices = choices?;
            Ok(format!(r"({})", choices.join("|")))
        }
        _ => Err(anyhow!("'enum' must be an array")),
    }
}

fn handle_const(obj: &serde_json::Map<String, Value>, _whitespace_pattern: &str) -> Result<String> {
    match obj.get("const") {
        Some(const_value) => match const_value {
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                let json_string = serde_json::to_string(const_value)?;
                Ok(regex::escape(&json_string))
            }
            _ => Err(anyhow!("Unsupported data type in const: {:?}", const_value)),
        },
        None => Err(anyhow!("'const' key not found in object")),
    }
}

// fn handle_ref(obj: &serde_json::Map<String, Value>, whitespace_pattern: &str) -> Result<String> {
//     // Implementation for $ref case
//     todo!()
// }

fn handle_type(obj: &serde_json::Map<String, Value>, whitespace_pattern: &str) -> Result<String> {
    let instance_type = obj["type"]
        .as_str()
        .ok_or_else(|| anyhow!("'type' must be a string"))?;
    match instance_type {
        "string" => handle_types::handle_string_type(obj),
        "number" => handle_types::handle_number_type(obj),
        "integer" => handle_types::handle_integer_type(obj),
        "array" => handle_types::handle_array_type(obj, whitespace_pattern),
        "object" => handle_types::handle_object_type(obj, whitespace_pattern),
        "boolean" => handle_types::handle_boolean_type(),
        "null" => handle_types::handle_null_type(),
        _ => Err(anyhow!("Unsupported type: {}", instance_type)),
    }
}

pub fn handle_empty_object(whitespace_pattern: &str) -> Result<String> {
    // JSON Schema Spec: Empty object means unconstrained, any json type is legal
    let types = vec![
        json!({"type": "boolean"}),
        json!({"type": "null"}),
        json!({"type": "number"}),
        json!({"type": "integer"}),
        json!({"type": "string"}),
        json!({"type": "array"}),
        json!({"type": "object"}),
    ];

    let regexes: Result<Vec<String>> = types
        .iter()
        .map(|t| to_regex(t, Some(whitespace_pattern)))
        .collect();

    let regexes = regexes?;

    let wrapped_regexes: Vec<String> = regexes.into_iter().map(|r| format!("({})", r)).collect();

    Ok(wrapped_regexes.join("|"))
}

#[cfg(test)]
mod tests {
    use crate::py_wrapper::python_build_regex_from_schema;
    use regex::Regex;
    use serde_json::json;

    use super::*;

    fn compare_with_outlines(schema: &serde_json::Value) {
        let schema_str = schema.to_string();
        let rust_regex = build_regex_from_schema(&schema_str, None).unwrap();
        let outlines_regex = python_build_regex_from_schema(&schema_str).unwrap();
        
        assert!(
            regex_equivalent(&rust_regex, &outlines_regex),
            "Rust and Python outputs are not equivalent for schema: {}\nRust:    {}\nPython:  {}",
            schema_str, rust_regex, outlines_regex
        );
    }

    fn regex_equivalent(a: &str, b: &str) -> bool {
        if a == b {
            return true;
        }
    
        // If direct comparison fails, compile and compare as regexes
        let re_a = Regex::new(a).unwrap();
        let re_b = Regex::new(b).unwrap();
        
        if re_a.to_string() == re_b.to_string() {            
            return true;
        } else {
            return false;
        }
    }
    


    #[test]
    fn test_handle_object_type() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "minLength": 2,
                    "maxLength": 5
                },
            },
        });
        compare_with_outlines(&schema);
    }

    #[test]
    fn test_string_type() {
        let schema = json!({"type": "string", "minLength": 2, "maxLength": 5});
        compare_with_outlines(&schema);
    }

    #[test]
    fn test_number_type() {
        let schema = json!({"type": "number", "minimum": 0, "maximum": 100});
        compare_with_outlines(&schema);
    }

    #[test]
    fn test_integer_type() {
        let schema = json!({"type": "integer", "minimum": 1, "maximum": 10});
        compare_with_outlines(&schema);
    }

    #[test]
    fn test_array_type() {
        let schema = json!({"type": "array", "items": {"type": "string"}});
        compare_with_outlines(&schema);
    }

    #[test]
    fn test_object_type() {
        let schema = json!({"type": "object", "properties": {"name": {"type": "string"}}});
        compare_with_outlines(&schema);
    }

    #[test]
    fn test_boolean_type() {
        let schema = json!({"type": "boolean"});
        compare_with_outlines(&schema);
    }

    #[test]
    fn test_null_type() {
        let schema = json!({"type": "null"});
        compare_with_outlines(&schema);
    }

    #[test]
    fn test_prefix_items() {
        let schema = json!({
            "prefixItems": [
                { "type": "integer" },
                { "type": "string" },
                { "type": "boolean" }
            ]
        });
        compare_with_outlines(&schema);
    }

    #[test]
    fn test_enum() {
        let schema = json!({
            "enum": ["red", "green", "blue", 42, true, null]
        });
        compare_with_outlines(&schema);
    }

    #[test]
    fn test_const() {
        let schema = json!({
            "const": "hello world"
        });
        compare_with_outlines(&schema);
    }

    #[test]
    fn test_expected_regex() {
        let schema1 = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "minLength": 2,
                    "maxLength": 5
                },
            },
        });
        compare_with_outlines(&schema1);

        let schema2 = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string"
                },
            },
        });
        compare_with_outlines(&schema2);

        let schema3 = json!({
            "type": "object",
            "properties": {
                "flag": {"type": "boolean"},
            },
        });
        compare_with_outlines(&schema3);

        let schema4 = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "flag": {"type": "boolean"},
            },
        });
        compare_with_outlines(&schema4);
    }
}
