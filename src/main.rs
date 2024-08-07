use regex::Regex;
use serde_json::json;
use std::env;

mod guidance;
mod handle_types;
mod py_wrapper;
mod types;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!(
            "Usage: {} <multiple|limit|simplebool|simple|empty>",
            args[0]
        );
        return;
    }

    let schema_type = &args[1];

    let json_input = match schema_type.as_str() {
        "multiple" => json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "flag": {"type": "boolean"},
            },
        }),
        "limit" => json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "minLength": 2, "maxLength": 5},
            },
        }),
        "simple" => json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
            },
        }),
        "simplebool" => json!({
            "type": "object",
            "properties": {
                "flag": {"type": "boolean"},
            },
        }),
        "empty" => json!({
            "type": "object",
        }),
        _ => {
            println!("Invalid argument. Use 'simple' or 'empty'.");
            return;
        }
    }
    .to_string();

    let regex = guidance::build_regex_from_schema(&json_input, None).unwrap();
    let regex_pattern = Regex::new(&regex).unwrap();

    println!("{}", regex_pattern.as_str());

    match schema_type.as_str() {
        "simple" => {
            let reference_regex =
                std::fs::read_to_string("reference/regex_for_simple_json.txt").unwrap();
            println!("regex length: {}", regex_pattern.as_str().len());
            println!("reference regex length: {}", reference_regex.len());
        }
        "empty" => {
            let reference_regex =
                std::fs::read_to_string("reference/regex_for_empty_json.txt").unwrap();
            println!("regex length: {}", regex_pattern.as_str().len());
            println!("reference regex length: {}", reference_regex.len());
        }
        _ => {}
    }
}
