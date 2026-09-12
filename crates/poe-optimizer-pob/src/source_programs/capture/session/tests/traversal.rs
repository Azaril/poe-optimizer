use super::*;
fn table_id(observed: &ObservedSourceSession, name: &str) -> SourceSessionTableId {
    let SourceSessionValue::Table(id) =
        observed.input().state.values[observed.root_index(name).unwrap()]
    else {
        panic!("table root")
    };
    id
}
fn source_order(table: &Table) -> Vec<SourceSessionValue> {
    let mut result = vec![];
    for row in table.pairs::<Value, Value>() {
        let (key, _) = row.unwrap();
        result.push(match key {
            Value::Integer(n) => SourceSessionValue::Number(n as f64),
            Value::Number(n) => SourceSessionValue::Number(n),
            Value::String(s) => SourceSessionValue::Bytes(s.as_bytes().to_vec()),
            _ => panic!("fixture key"),
        });
    }
    result
}
#[test]
fn live_order_and_sparse_raw_length_are_opt_in_actual_observations() {
    let f =
        fixture("local state = {z=false,[2]='sparse',a=4,[9]=5}; return {state=state,empty={}}\n");
    let ordinary = observe(&f, request(&f, &[], &["state", "empty"])).unwrap();
    assert!(ordinary.input().traversal.is_none());
    let table: Table = f.exports.raw_get("state").unwrap();
    let expected = source_order(&table);
    // Rebinding public next must not affect the retained original primitive.
    f.lua
        .globals()
        .raw_set(
            "next",
            f.lua
                .create_function(|_, (): ()| Err::<(), _>(mlua::Error::runtime("rebound next")))
                .unwrap(),
        )
        .unwrap();
    let mut req = request(&f, &[], &["state", "empty"]);
    req.definitions.capture_iteration = true;
    let observed = observe(&f, req).unwrap();
    let facet = observed.input().traversal.as_ref().unwrap();
    let actual = &facet.tables[&table_id(&observed, "state")];
    assert_eq!(actual.order, expected);
    assert_eq!(actual.raw_length, Some(table.raw_len() as u32));
    let empty = &facet.tables[&table_id(&observed, "empty")];
    assert!(empty.order.is_empty());
    assert_eq!(empty.raw_length, Some(0));
    let input = observed.input();
    input
        .validate_traversal(4096, 1_000_000, 16 * 1024 * 1024)
        .unwrap();
    assert!(
        observed.owner().definitions().unwrap().tables.is_empty(),
        "live facts stay outside shared definitions"
    );
}
#[test]
fn full_selection_keeps_raw_facts_but_omitted_keys_do_not() {
    let f = fixture("return {state={a=1,b=2}}\n");
    let table: Table = f.exports.raw_get("state").unwrap();
    for (fields, complete) in [(&["a"][..], false), (&["a", "b"][..], true)] {
        let mut req = request(&f, &[], &["state"]);
        req.definitions.capture_iteration = true;
        req.state_projections.push(selection(table.clone(), fields));
        let observed = observe(&f, req).unwrap();
        let id = table_id(&observed, "state");
        assert_eq!(
            observed
                .input()
                .traversal
                .as_ref()
                .unwrap()
                .tables
                .contains_key(&id),
            complete
        );
    }
}
#[test]
fn streaming_partial_projection_does_not_retain_omitted_lua_references() {
    let f = fixture(
        "local state={}; for i=1,12000 do state['key'..i]={value=i} end; return {state=state}\n",
    );
    let table: Table = f.exports.raw_get("state").unwrap();
    let mut req = request(&f, &[], &["state"]);
    req.definitions.capture_iteration = true;
    req.state_projections.push(selection(table, &["key1"]));
    let observed = observe(&f, req).unwrap();
    let id = table_id(&observed, "state");
    assert_eq!(
        observed.input().state.tables[id.0 as usize - 1]
            .entries
            .len(),
        1
    );
    assert_eq!(observed.input().coverage[&id].unavailable.len(), 11999);
    assert!(
        !observed
            .input()
            .traversal
            .as_ref()
            .unwrap()
            .tables
            .contains_key(&id)
    );
}
#[test]
fn observation_explicitly_rejects_unrepresented_raw_key_domains() {
    for key in ["true", "{}", "function() end", "1.5"] {
        let f = fixture(&format!("return {{state={{[{key}]=1}}}}\n"));
        let mut req = request(&f, &[], &["state"]);
        req.definitions.capture_iteration = true;
        assert!(
            observe(&f, req).is_err(),
            "key domain {key} needs actual capture support"
        );
    }
}
