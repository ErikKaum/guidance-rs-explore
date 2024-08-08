use std::num::NonZeroU64;

use anyhow::{anyhow, Ok, Result};
use serde_json::json;
use serde_json::Value;

use crate::guidance::to_regex;
use crate::types;

pub fn handle_boolean_type() -> Result<String> {
    let format_type = types::JsonType::Boolean;
    Ok(format_type.to_regex().to_string())
}

pub fn handle_null_type() -> Result<String> {
    let format_type = types::JsonType::Null;
    Ok(format_type.to_regex().to_string())
}

pub fn handle_string_type(obj: &serde_json::Map<String, Value>) -> Result<String> {
    if obj.contains_key("maxLength") || obj.contains_key("minLength") {
        let max_items = obj.get("maxLength");
        let min_items = obj.get("minLength");

        match (min_items, max_items) {
            (Some(min), Some(max)) if min.as_f64() > max.as_f64() => {
                return Err(anyhow::anyhow!(
                    "maxLength must be greater than or equal to minLength"
                ));
            }
            _ => {}
        }

        let formatted_max = max_items
            .and_then(Value::as_u64)
            .map_or("".to_string(), |n| format!("{}", n));
        let formatted_min = min_items
            .and_then(Value::as_u64)
            .map_or("".to_string(), |n| format!("{}", n));

        Ok(format!(
            r#""{}{{{},{}}}""#,
            types::STRING_INNER,
            formatted_min,
            formatted_max,
        ))
    } else if let Some(pattern) = obj.get("pattern").and_then(Value::as_str) {
        if pattern.starts_with('^') && pattern.ends_with('$') {
            Ok(format!(r#"("{}")"#, &pattern[1..pattern.len() - 1]))
        } else {
            Ok(format!(r#"("{}")"#, pattern))
        }
    } else if let Some(format) = obj.get("format").and_then(Value::as_str) {
        match types::FormatType::from_str(format) {
            Some(format_type) => Ok(format_type.to_regex().to_string()),
            None => Err(anyhow::anyhow!(
                "Format {} is not supported by Outlines",
                format
            )),
        }
    } else {
        Ok(types::JsonType::String.to_regex().to_string())
    }
}

pub fn handle_number_type(obj: &serde_json::Map<String, Value>) -> Result<String> {
    let bounds = [
        "minDigitsInteger",
        "maxDigitsInteger",
        "minDigitsFraction",
        "maxDigitsFraction",
        "minDigitsExponent",
        "maxDigitsExponent",
    ];

    let has_bounds = bounds.iter().any(|&key| obj.contains_key(key));

    if has_bounds {
        let (min_digits_integer, max_digits_integer) = validate_quantifiers(
            obj.get("minDigitsInteger").and_then(Value::as_u64),
            obj.get("maxDigitsInteger").and_then(Value::as_u64),
            1,
        )?;

        let (min_digits_fraction, max_digits_fraction) = validate_quantifiers(
            obj.get("minDigitsFraction").and_then(Value::as_u64),
            obj.get("maxDigitsFraction").and_then(Value::as_u64),
            0,
        )?;

        let (min_digits_exponent, max_digits_exponent) = validate_quantifiers(
            obj.get("minDigitsExponent").and_then(Value::as_u64),
            obj.get("maxDigitsExponent").and_then(Value::as_u64),
            0,
        )?;

        let integers_quantifier = match (min_digits_integer, max_digits_integer) {
            (Some(min), Some(max)) => format!("{{{},{}}}", min, max),
            (Some(min), None) => format!("{{{},}}", min),
            (None, Some(max)) => format!("{{1,{}}}", max),
            (None, None) => "*".to_string(),
        };
        let fraction_quantifier = match (min_digits_fraction, max_digits_fraction) {
            (Some(min), Some(max)) => format!("{{{},{}}}", min, max),
            (Some(min), None) => format!("{{{},}}", min),
            (None, Some(max)) => format!("{{0,{}}}", max),
            (None, None) => "+".to_string(),
        };

        let exponent_quantifier = match (min_digits_exponent, max_digits_exponent) {
            (Some(min), Some(max)) => format!("{{{},{}}}", min, max),
            (Some(min), None) => format!("{{{},}}", min),
            (None, Some(max)) => format!("{{0,{}}}", max),
            (None, None) => "+".to_string(),
        };

        Ok(format!(
            r"((-)?(0|[1-9][0-9]{}))(\.[0-9]{})?([eE][+-][0-9]{})?",
            integers_quantifier, fraction_quantifier, exponent_quantifier
        ))
    } else {
        let format_type = types::JsonType::Number;
        Ok(format_type.to_regex().to_string())
    }
}
pub fn handle_integer_type(obj: &serde_json::Map<String, Value>) -> Result<String> {
    if obj.contains_key("minDigits") || obj.contains_key("maxDigits") {
        let (min_digits, max_digits) = validate_quantifiers(
            obj.get("minDigits").and_then(Value::as_u64),
            obj.get("maxDigits").and_then(Value::as_u64),
            1,
        )?;

        let quantifier = match (min_digits, max_digits) {
            (Some(min), Some(max)) => format!("{{{},{}}}", min, max),
            (Some(min), None) => format!("{{{},}}", min),
            (None, Some(max)) => format!("{{1,{}}}", max),
            (None, None) => "*".to_string(),
        };

        Ok(format!(r"(-)?(0|[1-9][0-9]{})", quantifier))
    } else {
        let format_type = types::JsonType::Integer;
        Ok(format_type.to_regex().to_string())
    }
}
pub fn handle_object_type(
    obj: &serde_json::Map<String, Value>,
    whitespace_pattern: &str,
) -> Result<String> {
    let min_properties = obj.get("minProperties").and_then(|v| v.as_u64());
    let max_properties = obj.get("maxProperties").and_then(|v| v.as_u64());

    let num_repeats = get_num_items_pattern(min_properties, max_properties);

    if num_repeats.is_none() {
        return Ok(format!(r"\{{{}}}", whitespace_pattern));
    }

    let num_repeats = num_repeats.unwrap();
    let allow_empty = if min_properties.unwrap_or(0) == 0 {
        "?"
    } else {
        ""
    };

    let additional_properties = obj.get("additionalProperties");

    let value_pattern =
        if additional_properties.is_none() || additional_properties == Some(&Value::Bool(true)) {
            // Handle unconstrained object case
            let mut legal_types = vec![
                json!({"type": "string"}),
                json!({"type": "number"}),
                json!({"type": "boolean"}),
                json!({"type": "null"}),
            ];

            let depth = obj.get("depth").and_then(|v| v.as_u64()).unwrap_or(2);
            if depth > 0 {
                legal_types.push(json!({"type": "object", "depth": depth - 1}));
                legal_types.push(json!({"type": "array", "depth": depth - 1}));
            }

            let any_of = json!({"anyOf": legal_types});
            to_regex(&any_of, Some(whitespace_pattern))
        } else {
            to_regex(additional_properties.unwrap(), Some(whitespace_pattern))
        };

    // TODO handle the unwrap
    let value_pattern = value_pattern.unwrap();

    let key_value_pattern = format!(
        "{}{whitespace_pattern}:{whitespace_pattern}{value_pattern}",
        types::STRING
    );
    let key_value_successor_pattern =
        format!("{whitespace_pattern},{whitespace_pattern}{key_value_pattern}");
    let multiple_key_value_pattern = format!(
        "({key_value_pattern}({key_value_successor_pattern}){{{num_repeats}}}){allow_empty}"
    );

    let res = format!(
        r"\{{{}{}{}}}",
        whitespace_pattern, multiple_key_value_pattern, whitespace_pattern
    );
    Ok(res)
}

pub fn handle_array_type(
    obj: &serde_json::Map<String, Value>,
    whitespace_pattern: &str,
) -> Result<String> {
    let num_repeats = get_num_items_pattern(
        obj.get("minItems").and_then(Value::as_u64),
        obj.get("maxItems").and_then(Value::as_u64),
    )
    .unwrap_or_else(|| String::from(""));

    if num_repeats.is_empty() {
        return Ok(format!(r"\[{0}{0}\]", whitespace_pattern));
    }

    let allow_empty = if obj.get("minItems").and_then(Value::as_u64).unwrap_or(0) == 0 {
        "?"
    } else {
        ""
    };

    if let Some(items) = obj.get("items") {
        let items_regex = to_regex(items, Some(whitespace_pattern))?;
        Ok(format!(
            r"\[{0}(({1})(,{0}({1})){2}){3}{0}\]",
            whitespace_pattern, items_regex, num_repeats, allow_empty
        ))
    } else {
        let mut legal_types = vec![
            json!({"type": "boolean"}),
            json!({"type": "null"}),
            json!({"type": "number"}),
            json!({"type": "integer"}),
            json!({"type": "string"}),
        ];

        let depth = obj.get("depth").and_then(Value::as_u64).unwrap_or(2);
        if depth > 0 {
            legal_types.push(json!({"type": "object", "depth": depth - 1}));
            legal_types.push(json!({"type": "array", "depth": depth - 1}));
        }

        let regexes: Result<Vec<String>> = legal_types
            .iter()
            .map(|t| to_regex(t, Some(whitespace_pattern)))
            .collect();

        let regexes = regexes?;
        let regexes_joined = regexes.join("|");

        Ok(format!(
            r"\[{0}(({1})(,{0}({1})){2}){3}{0}\]",
            whitespace_pattern, regexes_joined, num_repeats, allow_empty
        ))
    }
}

/// HELPER FUNCTIONS

fn validate_quantifiers(
    min_bound: Option<u64>,
    max_bound: Option<u64>,
    start_offset: u64,
) -> Result<(Option<NonZeroU64>, Option<NonZeroU64>)> {
    let min_bound = min_bound.map(|n| NonZeroU64::new(n.saturating_sub(start_offset)));
    let max_bound = max_bound.map(|n| NonZeroU64::new(n.saturating_sub(start_offset)));

    if let (Some(min), Some(max)) = (min_bound, max_bound) {
        if max < min {
            return Err(anyhow!(
                "max bound must be greater than or equal to min bound"
            ));
        }
    }

    Ok((min_bound.flatten(), max_bound.flatten()))
}

fn get_num_items_pattern(min_items: Option<u64>, max_items: Option<u64>) -> Option<String> {
    let min_items = min_items.unwrap_or(0);

    match max_items {
        None => Some(format!("{{{},}}", min_items.saturating_sub(1))),
        Some(max_items) => {
            if max_items < 1 {
                None
            } else {
                Some(format!(
                    "{{{},{}}}",
                    min_items.saturating_sub(1),
                    max_items.saturating_sub(1)
                ))
            }
        }
    }
}
