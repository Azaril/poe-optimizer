//! Exact file comparison with bounded diagnostics; full evidence stays on disk.
use serde_json::Value;
use std::{fs, path::Path};

pub fn assert_files_equal(left: &Path, right: &Path, label: &str) {
    let left_bytes = fs::read(left).unwrap();
    let right_bytes = fs::read(right).unwrap();
    if left_bytes == right_bytes {
        return;
    }
    let difference = match (
        serde_json::from_slice::<Value>(&left_bytes),
        serde_json::from_slice::<Value>(&right_bytes),
    ) {
        (Ok(left), Ok(right)) => first_difference(&left, &right, "$")
            .unwrap_or_else(|| "JSON values agree but serialized bytes differ".into()),
        _ => "serialized evidence differs and is not valid JSON".into(),
    };
    panic!(
        "{label}: {difference}; complete evidence: {} ({} bytes), {} ({} bytes)",
        left.display(),
        left_bytes.len(),
        right.display(),
        right_bytes.len()
    );
}

pub fn first_difference(left: &Value, right: &Value, path: &str) -> Option<String> {
    match (left, right) {
        (Value::Array(left), Value::Array(right)) => {
            if left.len() != right.len() {
                return Some(format!("{path}.length: {} != {}", left.len(), right.len()));
            }
            left.iter()
                .zip(right)
                .enumerate()
                .find_map(|(index, (left, right))| {
                    first_difference(left, right, &format!("{}[{index}]", bounded(path, 500)))
                })
        }
        (Value::Object(left), Value::Object(right)) => {
            for (key, left) in left {
                let path = format!(
                    "{}[{}]",
                    bounded(path, 500),
                    Value::String(bounded(key, 120))
                );
                let Some(right) = right.get(key) else {
                    return Some(format!("{path}: field is missing on the right"));
                };
                if let Some(difference) = first_difference(left, right, &path) {
                    return Some(difference);
                }
            }
            right
                .keys()
                .find(|key| !left.contains_key(*key))
                .map(|key| {
                    format!(
                        "{}[{}]: field is missing on the left",
                        bounded(path, 500),
                        Value::String(bounded(key, 120))
                    )
                })
        }
        _ if left == right => None,
        _ => Some(format!("{path}: {} != {}", describe(left), describe(right))),
    }
}

fn bounded(text: &str, limit: usize) -> String {
    let mut chars = text.chars();
    let mut result: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        result.push_str("...");
    }
    result
}

fn describe(value: &Value) -> String {
    match value {
        Value::Array(rows) => format!("array with {} entries", rows.len()),
        Value::Object(fields) => format!("object with {} fields", fields.len()),
        Value::String(text) => Value::String(bounded(text, 120)).to_string(),
        _ => value.to_string(),
    }
}

#[test]
fn large_evidence_failure_is_bounded_and_keeps_both_complete_files() {
    let temp = tempfile::tempdir().unwrap();
    let left = temp.path().join("left.json");
    let right = temp.path().join("right.json");
    let value = serde_json::json!({"long field": "x".repeat(1_000_000), "ordinal": 60});
    let mut changed = value.clone();
    changed["long field"] = Value::String("y".repeat(1_000_000));
    let left_bytes = serde_json::to_vec(&value).unwrap();
    let right_bytes = serde_json::to_vec(&changed).unwrap();
    fs::write(&left, &left_bytes).unwrap();
    fs::write(&right, &right_bytes).unwrap();
    let failure = std::panic::catch_unwind(|| assert_files_equal(&left, &right, "JIT evidence"))
        .expect_err("different evidence must fail");
    let message = failure.downcast_ref::<String>().unwrap();
    assert!(message.len() < 2000);
    assert!(message.contains("long field"));
    assert_eq!(fs::read(left).unwrap(), left_bytes);
    assert_eq!(fs::read(right).unwrap(), right_bytes);
}

#[test]
fn serialized_differences_are_not_hidden_by_json_equivalence() {
    let temp = tempfile::tempdir().unwrap();
    let left = temp.path().join("left.json");
    let right = temp.path().join("right.json");
    fs::write(&left, b"{\"value\":1}").unwrap();
    fs::write(&right, b"{ \"value\": 1 }\n").unwrap();
    assert!(std::panic::catch_unwind(|| assert_files_equal(&left, &right, "bytes")).is_err());
}
