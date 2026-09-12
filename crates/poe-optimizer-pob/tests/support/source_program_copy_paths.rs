//! Bounded graph identity diagnostics. Sorting chooses a display path, never iteration order.
use serde_json::{Value as Json, json};
use std::collections::VecDeque;

pub(super) fn describe(graph: &Json) -> Result<Json, String> {
    let roots = graph["values"].as_array().ok_or("missing roots")?;
    if roots.len() != 3 {
        return Err("expected cache row, traversal table and control roots".into());
    }
    let tables = graph["tables"].as_array().ok_or("missing tables")?;
    if tables.len() > 512 {
        return Err("copy witness table bound exceeded".into());
    }
    let table_id = |value: &Json| -> Result<usize, String> {
        let id = value["table"].as_u64().ok_or("expected table identity")? as usize;
        if id >= tables.len() {
            return Err("table reference outside graph".into());
        }
        Ok(id)
    };
    let root = table_id(&roots[0])?;
    let target = table_id(&roots[1])?;
    let mut paths: Vec<Option<Vec<Json>>> = vec![None; tables.len()];
    paths[root] = Some(Vec::new());
    let mut queue = VecDeque::from([root]);
    let mut incoming = Vec::new();
    let mut edges = 0usize;
    while let Some(parent) = queue.pop_front() {
        for entry in tables[parent].as_array().ok_or("invalid table rows")? {
            edges += 1;
            if edges > 8192 {
                return Err("copy witness edge bound exceeded".into());
            }
            let row = entry
                .as_array()
                .filter(|row| row.len() == 2)
                .ok_or("invalid table entry")?;
            let key = &row[0];
            if key.get("number_bits").is_none() && key.get("bytes").is_none() {
                return Err("unrepresented diagnostic key".into());
            }
            if row[1].get("table").is_none() {
                continue;
            }
            let child = table_id(&row[1])?;
            let mut path = paths[parent].as_ref().unwrap().clone();
            if path.len() >= 64 {
                return Err("copy witness path bound exceeded".into());
            }
            path.push(key.clone());
            if child == target {
                incoming.push(json!({"parent_path":paths[parent],"key":key}));
            }
            if paths[child].is_none() {
                paths[child] = Some(path);
                queue.push_back(child);
            }
        }
    }
    Ok(
        json!({"reachable_from_cache":paths[target].is_some(),"shortest_path":paths[target],
        "incoming_alias_edges":incoming,"target_is_cache_root":target==root,
        "reachable_tables":paths.iter().filter(|path|path.is_some()).count(),
        "traversal_control":roots[2],"target_entries":tables[target],
        "scope":"joint-root identity and bounded raw paths; canonical IDs are graph-local; no allocation or iteration-order inference"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn index(value: f64) -> Json {
        json!({"number_bits":format!("{:016x}",value.to_bits())})
    }
    #[test]
    fn shared_and_equal_distinct_children_have_different_alias_evidence() {
        let shared = json!({"values":[{"table":0},{"table":1},{"nil":true}],
            "tables":[[[index(1.0),{"table":1}],[index(2.0),{"table":1}]],[]]});
        let equal_distinct = json!({"values":[{"table":0},{"table":1},{"nil":true}],
            "tables":[[[index(1.0),{"table":1}],[index(2.0),{"table":2}]],[],[]]});
        let first = describe(&shared).unwrap();
        let second = describe(&equal_distinct).unwrap();
        assert_eq!(first["target_entries"], second["target_entries"]);
        assert_eq!(first["incoming_alias_edges"].as_array().unwrap().len(), 2);
        assert_eq!(second["incoming_alias_edges"].as_array().unwrap().len(), 1);
        assert_eq!(first["shortest_path"], json!([index(1.0)]));
    }
    #[test]
    fn cycles_are_bounded_and_control_is_not_assumed_nil() {
        let graph = json!({"values":[{"table":0},{"table":1},{"bytes":[110,97,109,101]}],
            "tables":[[[index(1.0),{"table":1}]],[[index(2.0),{"table":0}],[index(3.0),{"table":1}]]]});
        let result = describe(&graph).unwrap();
        assert_eq!(result["reachable_tables"], 2);
        assert_eq!(result["incoming_alias_edges"].as_array().unwrap().len(), 2);
        assert_eq!(
            result["traversal_control"],
            json!({"bytes":[110,97,109,101]})
        );
    }
    #[test]
    fn detached_equal_tables_and_invalid_graphs_do_not_fake_a_cache_path() {
        let graph = json!({"values":[{"table":0},{"table":2},{"nil":true}],
            "tables":[[[index(1.0),{"table":1}]],[],[]]});
        let result = describe(&graph).unwrap();
        assert_eq!(result["reachable_from_cache"], false);
        assert_eq!(result["shortest_path"], Json::Null);
        let invalid = json!({"values":[{"table":0},{"table":9},{"nil":true}],"tables":[[]]});
        assert!(describe(&invalid).is_err());
        let excessive =
            json!({"values":[{"table":0},{"table":0},{"nil":true}],"tables":vec![json!([]);513]});
        assert!(describe(&excessive).is_err());
    }
}
