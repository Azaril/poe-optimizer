//! Injective relocation preserves identities and repeated references in source-corresponding test fixtures.
use serde_json::Value;
use std::collections::BTreeMap;
// Compare an exact structural occurrence correspondence with one injective ID
// relocation map. Repeated references must agree; IDs are never discarded.
pub fn correspond(a: &Value, b: &mut Value, ids: &mut BTreeMap<String, Value>, path: &str) {
    if a.get("local").is_some() && a.get("lineage").is_some() {
        assert!(
            b.get("local").is_some() && b.get("lineage").is_some(),
            "{path}"
        );
        let k = serde_json::to_string(b).unwrap();
        if let Some(old) = ids.get(&k) {
            assert_eq!(old, a, "ID reference changed: {path}");
        } else {
            assert!(!ids.values().any(|v| v == a), "ID coalescence at {path}");
            ids.insert(k, a.clone());
        }
        *b = a.clone();
        return;
    }
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            assert_eq!(x.len(), y.len(), "{path}");
            for (k, v) in x {
                correspond(v, y.get_mut(k).unwrap(), ids, &format!("{path}.{k}"));
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            assert_eq!(x.len(), y.len(), "{path}");
            for (i, (a, b)) in x.iter().zip(y).enumerate() {
                correspond(a, b, ids, &format!("{path}[{i}]"));
            }
        }
        (x, y) => assert_eq!(x, y, "{path}"),
    }
}
pub fn relocate(v: &mut Value, ids: &BTreeMap<String, Value>) {
    if v.get("local").is_some() && v.get("lineage").is_some() {
        *v = ids
            .get(&serde_json::to_string(v).unwrap())
            .expect("old source occurrence must have a correspondence")
            .clone();
        return;
    }
    match v {
        Value::Object(o) => {
            for v in o.values_mut() {
                relocate(v, ids)
            }
        }
        Value::Array(a) => {
            for v in a {
                relocate(v, ids)
            }
        }
        _ => {}
    }
}
