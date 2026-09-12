use super::*;
const COUNT: usize = 9_000;
const LARGE: &str = r#"local state, shared = {}, {value=0}
for i=1,9000 do
    local marker = 'marker:' .. i
    state[i] = function() return marker, shared end
end
state.alias, state.self = state[1], state
return {state=state, shared=shared}
"#;
fn entry<'a>(table: &'a SourceSessionTable, key: &SourceSessionValue) -> &'a SourceSessionValue {
    &table
        .entries
        .iter()
        .find(|(candidate, _)| candidate == key)
        .unwrap()
        .1
}
#[test]
fn full_live_graph_over_auxiliary_reference_limit_preserves_closures_cells_aliases_and_order() {
    let f = fixture(LARGE);
    let state: Table = f.exports.raw_get("state").unwrap();
    let first: Function = state.raw_get(1).unwrap();
    let first_pointer = first.to_pointer();
    let (marker, shared): (String, Table) = first.call(()).unwrap();
    assert_eq!(marker, "marker:1");
    let before_memory = {
        f.lua.gc_collect().unwrap();
        f.lua.used_memory()
    };
    let mut prior = None;
    for iteration in [false, true, true] {
        let mut req = request(&f, &[], &["state"]);
        req.definitions.capture_iteration = iteration;
        let observed = observe(&f, req).unwrap();
        let input = observed.input();
        assert_eq!(input.closures.len(), COUNT);
        assert_eq!(input.cells.len(), COUNT + 1);
        assert_eq!(input.state.tables.len(), 2);
        assert_eq!(observed.owner().callbacks().len(), 1);
        let SourceSessionValue::Table(root) = input.state.values[0] else {
            panic!("root table")
        };
        let raw = &input.state.tables[root.0 as usize - 1];
        assert_eq!(raw.entries.len(), COUNT + 2);
        assert_eq!(
            entry(raw, &SourceSessionValue::Bytes(b"self".to_vec())),
            &SourceSessionValue::Table(root)
        );
        assert_eq!(
            entry(raw, &SourceSessionValue::Bytes(b"alias".to_vec())),
            entry(raw, &SourceSessionValue::Number(1.0))
        );
        let callback = &observed.owner().callbacks()[0];
        let marker_slot = callback
            .upvalues
            .iter()
            .position(|slot| slot.name == "marker")
            .unwrap();
        let shared_slot = callback
            .upvalues
            .iter()
            .position(|slot| slot.name == "shared")
            .unwrap();
        let common = input.closures[0].captures[shared_slot];
        let mut separate = BTreeSet::new();
        for i in 1..=COUNT {
            let SourceSessionValue::Closure(id) = entry(raw, &SourceSessionValue::Number(i as f64))
            else {
                panic!("original closure")
            };
            let closure = &input.closures[id.0 as usize - 1];
            assert_eq!(closure.captures[shared_slot], common);
            let cell = closure.captures[marker_slot];
            assert!(separate.insert(cell));
            assert_eq!(
                input.cells[cell.0 as usize - 1],
                SourceSessionValue::Bytes(format!("marker:{i}").into_bytes())
            );
        }
        assert_eq!(separate.len(), COUNT);
        if iteration {
            let traversal = &input.traversal.as_ref().unwrap().tables[&root];
            assert_eq!(
                traversal.order,
                state
                    .pairs::<Value, Value>()
                    .map(|row| match row.unwrap().0 {
                        Value::Integer(n) => SourceSessionValue::Number(n as f64),
                        Value::Number(n) => SourceSessionValue::Number(n),
                        Value::String(s) => SourceSessionValue::Bytes(s.as_bytes().to_vec()),
                        _ => panic!("fixture key"),
                    })
                    .collect::<Vec<_>>()
            );
            assert_eq!(traversal.raw_length, Some(state.raw_len() as u32));
        } else {
            assert!(input.traversal.is_none());
        }
        let comparable = (
            input.state.clone(),
            input.cells.clone(),
            input
                .closures
                .iter()
                .map(|c| (c.prototype.id(), c.captures.clone()))
                .collect::<Vec<_>>(),
        );
        if let Some(previous) = &prior {
            assert_eq!(previous, &comparable);
        }
        prior = Some(comparable);
        drop(observed);
        f.lua.gc_collect().unwrap();
        // Source state stays rooted by the fixture; each completed observation
        // releases its private arena rather than accumulating Lua references.
        assert!(f.lua.used_memory() < before_memory + 256 * 1024);
        assert_eq!(
            state.raw_get::<Function>(1).unwrap().to_pointer(),
            first_pointer
        );
        assert_eq!(shared.raw_get::<i32>("value").unwrap(), 0);
        assert_eq!(
            state.raw_get::<Table>("self").unwrap().to_pointer(),
            state.to_pointer()
        );
    }
}
#[test]
fn failed_large_capture_releases_reference_arena_and_keeps_raw_row_bound() {
    let f = fixture(
        r#"local state={}
for i=1,12000 do state['key'..i]='value:'..i end
state.zzbad=coroutine.create(function() end)
return {state=state}
"#,
    );
    let state: Table = f.exports.raw_get("state").unwrap();
    f.lua.gc_collect().unwrap();
    let baseline = f.lua.used_memory();
    for _ in 0..3 {
        assert!(
            observe(&f, request(&f, &[], &["state"]))
                .unwrap_err()
                .to_string()
                .contains("value is not represented")
        );
        f.lua.gc_collect().unwrap();
        assert!(f.lua.used_memory() < baseline + 256 * 1024);
        assert!(matches!(
            state.raw_get::<Value>("zzbad").unwrap(),
            Value::Thread(_)
        ));
    }
    state.raw_set("zzbad", Value::Nil).unwrap();
    let observed = observe(&f, request(&f, &[], &["state"])).unwrap();
    assert_eq!(observed.input().state.tables[0].entries.len(), 12000);
    let bounded = fixture(
        "local state={}; for i=1,50001 do state[i]='value:'..i end; return {state=state}\n",
    );
    assert!(
        observe(&bounded, request(&bounded, &[], &["state"]))
            .unwrap_err()
            .to_string()
            .contains("raw row bound")
    );
}

#[test]
fn unsupported_capture_diagnostic_retains_actual_function_span_and_slot_name() {
    let f = fixture(
        "local unsafe = collectgarbage\nlocal function run() return unsafe end\nreturn {run=run}\n",
    );
    let message = observe(&f, request(&f, &["run"], &[]))
        .unwrap_err()
        .to_string();
    assert!(
        message.contains("tests/session.lua:2-2 capture unsafe"),
        "{message}"
    );
    assert!(
        message.contains("builtin has no observed primitive identity"),
        "{message}"
    );
}

#[test]
fn thousands_of_live_table_identities_do_not_consume_auxiliary_reference_slots() {
    let f = fixture(
        "local state={}\nfor i=1,9000 do local t={value=i}; t.self=t; state[i]=t end\nstate.alias=state[1]\nreturn {state=state}\n",
    );
    let mut req = request(&f, &[], &["state"]);
    req.definitions.capture_iteration = true;
    let observed = observe(&f, req).unwrap();
    let input = observed.input();
    assert_eq!(input.state.tables.len(), COUNT + 1);
    assert_eq!(input.traversal.as_ref().unwrap().tables.len(), COUNT + 1);
    let SourceSessionValue::Table(root) = input.state.values[0] else {
        panic!("root table")
    };
    let raw = &input.state.tables[root.0 as usize - 1];
    assert_eq!(
        entry(raw, &SourceSessionValue::Bytes(b"alias".to_vec())),
        entry(raw, &SourceSessionValue::Number(1.0))
    );
    let mut identities = BTreeSet::new();
    for i in 1..=COUNT {
        let SourceSessionValue::Table(id) = entry(raw, &SourceSessionValue::Number(i as f64))
        else {
            panic!("child table")
        };
        assert!(identities.insert(*id));
        let child = &input.state.tables[id.0 as usize - 1];
        assert_eq!(
            entry(child, &SourceSessionValue::Bytes(b"self".to_vec())),
            &SourceSessionValue::Table(*id)
        );
        assert_eq!(
            entry(child, &SourceSessionValue::Bytes(b"value".to_vec())),
            &SourceSessionValue::Number(i as f64)
        );
    }
    assert!(observed.owner().definitions().unwrap().tables.is_empty());
}
