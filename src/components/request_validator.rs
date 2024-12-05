use serde_json::Value;
use serde_json_diff::{Difference, EntryDifference};

use super::composite_objects::RequestObject;

pub struct ValidationError {
    pub error_message: String,
}

pub fn validate_request(
    request_object: &RequestObject,
    body: String,
) -> Result<bool, ValidationError> {
    let parsed: Value = serde_json::from_str(&body).unwrap();
    let obj = serde_json::to_value(parsed.as_object().unwrap().clone()).unwrap();

    let parsed2: Value = serde_json::from_str(&request_object.clone().body.unwrap()).unwrap();
    let obj2 = serde_json::to_value(parsed2.as_object().unwrap().clone()).unwrap();

    if let Some(diff) = serde_json_diff::values(obj, obj2) {
        let mut missing_keys: Vec<String> = vec![];
        check_difference(&diff, &mut missing_keys);
        if missing_keys.len() > 0 {
            let error_message =
                "Missing keys: ".to_string() + missing_keys.join(", ").to_owned().as_str();
            return Err(ValidationError {
                error_message: String::from(error_message),
            });
        }
    }
    Ok(true)
}

fn check_difference(diff: &Difference, missing_keys: &mut Vec<String>) {
    match diff {
        Difference::Object { different_entries } => {
            for (key, entry) in &different_entries.0 {
                match entry {
                    EntryDifference::Missing { value } => {
                        missing_keys.push(key.clone());
                    }
                    EntryDifference::Value { value_diff } => match value_diff {
                        Difference::Object { different_entries } => {
                            check_difference(value_diff, missing_keys)
                        }
                        _ => (),
                    },
                    _ => (),
                }
            }
        }
        _ => (),
    }
}
