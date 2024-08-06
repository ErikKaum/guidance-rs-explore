use regex::Regex;
use serde_json::json;

mod guidance;
mod types;

mod handle_types;

fn main() {
    let json_input = json!(
        {
            "type": "object",
                "properties": {
                    "name": {"type": "string"},
            }
        }
    ).to_string();

    let regex = guidance::build_regex_from_schema(&json_input, None).unwrap();

    let regex_pattern = Regex::new(&regex).unwrap();

    println!("regex: {:?}", regex_pattern.as_str());

    let reference_regex = std::fs::read_to_string(
        "/Users/erikkaum/Documents/guidance-rs/reference/regex_for_empty_json.txt",
    )
    .unwrap();

    println!("regex length: {}", regex_pattern.as_str().len());
    println!("reference regex length: {}", reference_regex.len());
}
