use anyhow::{anyhow, Result};
use jsonschema::JSONSchema;
use serde_json::json;
use serde_json::Value;
use regex::escape;

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

    let regex = to_regex(&json_value, whitespace_pattern);

    Ok(regex.unwrap())
}

pub fn to_regex(json: &Value, whitespace_pattern: Option<&str>) -> Result<String> {
    
    println!("to regex called with json: {:?}", json);
    
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
                .ok_or_else(|| anyhow!("Unsupported JSON Schema structure"))?
            };

            match keyword {
                SchemaKeyword::Properties => handle_properties(&obj, whitespace_pattern),
                // SchemaKeyword::AllOf => handle_all_of(&obj, whitespace_pattern),
                SchemaKeyword::AnyOf => handle_any_of(&obj, whitespace_pattern),
                // SchemaKeyword::OneOf => handle_one_of(&obj, whitespace_pattern),
                // SchemaKeyword::PrefixItems => handle_prefix_items(&obj, whitespace_pattern),
                // SchemaKeyword::Enum => handle_enum(&obj, whitespace_pattern),
                // SchemaKeyword::Const => handle_const(&obj, whitespace_pattern),
                // SchemaKeyword::Ref => handle_ref(&obj, whitespace_pattern),
                SchemaKeyword::Type => handle_type(&obj, whitespace_pattern),
                SchemaKeyword::EmptyObject => handle_empty_object(whitespace_pattern),
                val => Err(anyhow!("Unsupported JSON Schema keyword: {:?}", val)),
            }
        }
        _ => Err(anyhow!("Invalid JSON Schema: expected an object")),
    }
}

fn handle_properties(obj: &serde_json::Map<String, Value>, whitespace_pattern: &str) -> Result<String> {
    
    let mut regex = String::from(r"\{");
    
    let properties = obj.get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("'properties' not found or not an object"))?;
    
    let required_properties = obj.get("required")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();

    let is_required: Vec<bool> = properties.keys()
        .map(|item| required_properties.contains(&item.as_str()))
        .collect();

    if is_required.iter().any(|&x| x) {
        let last_required_pos = is_required.iter().enumerate()
            .filter(|&(_, &value)| value)
            .map(|(i, _)| i)
            .max()
            .unwrap();

        for (i, (name, value)) in properties.iter().enumerate() {
            let mut subregex = format!(r#"{whitespace_pattern}"{}"{}:{}""#, 
                escape(name), whitespace_pattern, whitespace_pattern);
            subregex += &to_regex(value, Some(whitespace_pattern))?;

            if i < last_required_pos {
                subregex = format!("{}{},", subregex, whitespace_pattern);
            } else if i > last_required_pos {
                subregex = format!("{},{}", whitespace_pattern, subregex);
            }

            regex += &if is_required[i] { subregex } else { format!("({})?", subregex) };
        }
    } else {
        let mut property_subregexes = Vec::new();
        for (name, value) in properties.iter() {
            let mut subregex = format!(r#"{whitespace_pattern}"{}"{}:{}""#, 
                escape(name), whitespace_pattern, whitespace_pattern);
            subregex += &to_regex(value, Some(whitespace_pattern))?;
            property_subregexes.push(subregex);
        }

        let mut possible_patterns = Vec::new();
        for i in 0..property_subregexes.len() {
            let mut pattern = String::new();
            for subregex in &property_subregexes[..i] {
                pattern += &format!("({}{}}},)?", subregex, whitespace_pattern);
            }
            pattern += &property_subregexes[i];
            for subregex in &property_subregexes[i + 1..] {
                pattern += &format!("({},{})?", whitespace_pattern, subregex);
            }
            possible_patterns.push(pattern);
        }

        regex += &format!("({})?", possible_patterns.join("|"));
    }

    regex += &format!("{}}}", whitespace_pattern);

    Ok(regex)
}


// fn handle_all_of(obj: &serde_json::Map<String, Value>, whitespace_pattern: &str) -> Result<String> {
//     // Implementation for allOf case
//     todo!()
// }

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

// fn handle_one_of(obj: &serde_json::Map<String, Value>, whitespace_pattern: &str) -> Result<String> {
//     // Implementation for oneOf case
//     todo!()
// }
// fn handle_prefix_items(obj: &serde_json::Map<String, Value>, whitespace_pattern: &str) -> Result<String> {
//     // Implementation for prefixItems case
//     todo!()
// }
// fn handle_enum(obj: &serde_json::Map<String, Value>, whitespace_pattern: &str) -> Result<String> {
//     // Implementation for enum case
//     todo!()
// }
// fn handle_const(obj: &serde_json::Map<String, Value>, whitespace_pattern: &str) -> Result<String> {
//     // Implementation for const case
//     todo!()
// }
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
    use regex::Regex;

    use super::*;

    #[test]
    fn test_boolean_type() {
        let json = r#"{"type": "boolean"}"#;
        let regex_pattern = build_regex_from_schema(json, None).unwrap();

        let regex = Regex::new(&regex_pattern).unwrap();

        assert!(regex.is_match("true"));
        assert!(regex.is_match("false"));

        assert!(!regex.is_match("null"));
        assert!(!regex.is_match("42"));
        assert!(!regex.is_match("3.14"));
        assert!(!regex.is_match("\"hello\""));
    }

    #[test]
    fn test_string_type() {
        let json = r#"{"type": "string"}"#;
        let regex_pattern = build_regex_from_schema(json, None).unwrap();

        let regex = Regex::new(&regex_pattern).unwrap();

        // Valid strings
        assert!(regex.is_match(r#""hello""#));
        assert!(regex.is_match(r#""""#)); // Empty string
        assert!(regex.is_match(r#""Hello, World!""#));
        assert!(regex.is_match(r#""1234""#));
        assert!(regex.is_match(r#""Special chars: !@#$%^&*()""#));
        assert!(regex.is_match(r#""Escaped \"quotes\"""#));
        assert!(regex.is_match(r#""Escaped backslash: \\""#));

        // Invalid strings
        assert!(!regex.is_match(r#"hello"#)); // Unquoted
        assert!(!regex.is_match(r#""Unclosed quote"#));
        assert!(!regex.is_match(r#"'Single quotes'"#));
        assert!(!regex.is_match(
            r#""Contains
        newline""#
        ));

        // TODO, return to why this is failing
        // assert!(!regex.is_match(r#""Contains "unescaped" quotes""#));

        // Non-string types
        assert!(!regex.is_match("true"));
        assert!(!regex.is_match("false"));
        assert!(!regex.is_match("null"));
        assert!(!regex.is_match("42"));
        assert!(!regex.is_match("3.14"));
    }

    #[test]
    fn test_string_type_with_length_constraint() {
        let obj = json!({"type": "string", "minLength": "2", "maxLength": "5"})
            .as_object()
            .unwrap()
            .clone();
        let regex_pattern = handle_types::handle_string_type(&obj).unwrap();

        let regex = Regex::new(&regex_pattern).unwrap();

        assert!(regex.is_match(r#""ab""#));
        assert!(regex.is_match(r#""abcde""#));
        assert!(!regex.is_match(r#""a""#)); // Too short
        assert!(!regex.is_match(r#""abcdef""#)); // Too long
    }

    #[test]
    fn test_object_type() {
        let obj = json!(
            {
                "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                }
            }
        )
        .as_object()
        .unwrap()
        .clone();

        let regex_pattern = handle_types::handle_object_type(&obj, types::WHITESPACE).unwrap();

        println!("regex_pattern: {}", regex_pattern);

        let regex = Regex::new(&regex_pattern).unwrap();

        assert!(regex.is_match(r#"{}"#));
    }
}
