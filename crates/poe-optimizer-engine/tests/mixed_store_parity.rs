//! Authentic mixed ModList/ModDB queries; captured contexts are not full-build admission.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/condition_producers_corpus.rs"]
mod corpus;
#[path = "support/mixed_store_native.rs"]
mod native;
#[path = "support/mixed_store_oracle.rs"]
mod oracle;
use oracle::{Observed, Oracle};
use serde_json::{Value, json};
use std::cell::Cell;
thread_local! { static PAIRS: Cell<usize> = const { Cell::new(0) }; }
struct CountGuard(usize);
impl CountGuard {
    fn new(expected: usize) -> Self {
        PAIRS.with(|count| count.set(0));
        Self(expected)
    }
}
impl Drop for CountGuard {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            assert_eq!(PAIRS.with(Cell::get), self.0, "paired observation count");
        }
    }
}

const CHAINS: [[&str; 2]; 4] = [
    ["ModDB", "ModDB"],
    ["ModDB", "ModList"],
    ["ModList", "ModDB"],
    ["ModList", "ModList"],
];
fn fixture(kinds: &[&str]) -> Value {
    let stores: Vec<_> = kinds.iter().enumerate().map(|(i,kind)| json!({"store_type":kind,"actor":1,"parent":(i+1<kinds.len()).then_some(i+2),"conditions":{},"mods":[]})).collect();
    json!({"root":1,"stores":stores,"actors":[{"store":kinds.len(),"links":{},"output":{}}],"cfg":{},"precision":{},"query":{"kind":"sum","operation":"BASE","names":["Target"]}})
}
fn record(name: &str, kind: &str, value: Value) -> Value {
    json!({"name":name,"type":kind,"value":value,"flags":0,"keywordFlags":0,"source":"Item:caller","tags":[]})
}
fn predicate(variable: &str) -> Value {
    json!({"type":"Condition","var":variable})
}
fn set_query(input: &mut Value, operation: &str, names: Value) {
    input["query"] = match operation {
        "BASE" | "INC" => json!({"kind":"sum","operation":operation,"names":names}),
        "positive" => json!({"kind":"positive","operation":"BASE","names":names}),
        value => json!({"kind":value,"names":names}),
    };
}
fn operation_type(operation: &str) -> &str {
    match operation {
        "BASE" | "positive" => "BASE",
        "INC" => "INC",
        "more" => "MORE",
        "override" => "OVERRIDE",
        "max" => "MAX",
        "flag" => "FLAG",
        _ => panic!("bad query"),
    }
}
fn same(actual: &Observed, expected: &Observed) {
    match (actual, expected) {
        (Observed::Number(a), Observed::Number(b)) if a.is_nan() && b.is_nan() => (),
        (Observed::Number(a), Observed::Number(b)) => {
            assert_eq!(a.to_bits(), b.to_bits(), "{a:?} vs {b:?}")
        }
        _ => assert_eq!(actual, expected),
    }
}
fn parity(oracle: &Oracle, input: &Value) -> Option<Observed> {
    PAIRS.with(|count| count.set(count.get() + 1));
    match (oracle.query(input), native::query(input)) {
        (Ok(original), Ok(native)) => {
            same(&native, &original);
            Some(original)
        }
        (Err(original), Err(native)) => {
            // Every negative cell in these paired matrices is a reached missing
            // source. Other errors must not masquerade as successful error parity.
            let (layer, position) = input["stores"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .find_map(|(layer, store)| {
                    store["mods"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .position(|row| row["source"].is_null())
                        .map(|position| (layer, position))
                })
                .expect("paired error requires an explicit absent-source row");
            let store_kind = input["stores"][layer]["store_type"].as_str().unwrap();
            let operation = input["query"]["kind"].as_str().unwrap();
            let multi = input["query"]["names"]
                .as_array()
                .is_some_and(|v| v.len() != 1);
            let line = match (store_kind, operation, multi) {
                ("ModList", "sum", false) => 129,
                ("ModList", "sum", true) => 149,
                ("ModDB", "more", false) => 223,
                ("ModDB", "more", true) => 265,
                ("ModList", "more", false) => 170,
                ("ModList", "more", true) => 203,
                ("ModDB", "override", false) => 349,
                ("ModDB", "override", true) => 373,
                ("ModList", "override", false) => 271,
                ("ModList", "override", true) => 292,
                ("ModDB", "max", true) => 473,
                ("ModList", "max", true) => 375,
                ("ModDB", "max" | "positive", _) => 444,
                ("ModList", "max" | "positive", _) => 353,
                ("ModDB", "flag", true) => 327,
                ("ModList", "flag", true) => 252,
                ("ModDB", "flag" | "condition", _) => 303,
                ("ModList", "flag" | "condition", _) => 232,
                other => {
                    panic!("unrepresented paired error origin {other:?}: {original} / {native}")
                }
            };
            let original = original.to_string();
            assert!(
                original.contains("field 'source'")
                    && original.contains(&format!("src/Classes/{store_kind}.lua:{line}:")),
                "{original}"
            );
            assert!(
                native.contains(&format!(
                    "MissingSource {{ layer: {layer}, modifier: {position} }}"
                )),
                "unexpected native error: {native}"
            );
            None
        }
        (a, b) => panic!("original/native disagreement: {a:?} / {b:?}\n{input}"),
    }
}

#[test]
fn per_matching_layer_source_filters_missing_sources_and_full_strings_match() {
    let _count = CountGuard::new(3360);
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for chain in CHAINS {
            for operation in ["BASE", "INC", "more", "override", "max", "positive", "flag"] {
                for source in [
                    Value::Null,
                    json!("Item:caller"),
                    json!("Item"),
                    json!(""),
                    json!("::Item:caller"),
                ] {
                    for query_source in [
                        Value::Null,
                        json!("Item"),
                        json!("Item:caller"),
                        json!(""),
                        json!(":"),
                        json!("Other"),
                    ] {
                        for matching_layer in 0..2 {
                            let mut input = fixture(&chain);
                            let mut row =
                                record("Target", operation_type(operation), json!(13.125));
                            row["source"] = source.clone();
                            input["stores"][matching_layer]["mods"] = json!([row]);
                            input["cfg"]["source"] = query_source.clone();
                            set_query(&mut input, operation, json!(["Target"]));
                            parity(&oracle, &input);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn name_repetition_layer_grouping_and_precision_carry_match_original_bits() {
    let _count = CountGuard::new(168);
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for chain in CHAINS {
            let mut input = fixture(&[chain[0], chain[1], chain[0]]);
            for (i, amount) in [1e16, -1e16, 1.0].into_iter().enumerate() {
                input["stores"][i]["mods"] = json!([
                    record("A", "BASE", json!(amount)),
                    record("B", "BASE", json!(3.0))
                ]);
            }
            for names in [
                json!(["A"]),
                json!(["A", "B"]),
                json!(["B", "A", "B"]),
                json!(["Missing", "A", "A"]),
            ] {
                set_query(&mut input, "BASE", names);
                parity(&oracle, &input);
            }
            for i in 0..3 {
                input["stores"][i]["mods"] = json!([
                    record("A", "MORE", json!(3.4567)),
                    record("B", "MORE", json!(-15.123)),
                    record("A", "MORE", json!(12.789)),
                    record("Missing", "MORE", json!(0.0123))
                ]);
            }
            for precision in [
                json!({}),
                json!({"A":4}),
                json!({"B":6}),
                json!({"A":4,"B":6}),
            ] {
                input["precision"] = precision;
                for names in [
                    json!(["A"]),
                    json!(["A", "B"]),
                    json!(["B", "A", "B"]),
                    json!(["Missing", "A", "A"]),
                ] {
                    set_query(&mut input, "more", names);
                    parity(&oracle, &input);
                }
            }
            input["stores"][0]["mods"] = json!([record("Later", "OVERRIDE", json!(0.0))]);
            input["stores"][1]["mods"] = json!([record("Earlier", "OVERRIDE", json!(19))]);
            set_query(&mut input, "override", json!(["Earlier", "Later"]));
            same(&parity(&oracle, &input).unwrap(), &Observed::Number(0.0));
        }
    }
}

#[test]
fn mixed_producers_keep_source_bypass_local_to_db_rows_and_child_conditions() {
    let _count = CountGuard::new(448);
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for chain in CHAINS {
            for producer_layer in 0..2 {
                for bypass in [false, true] {
                    for closed in [false, true] {
                        let mut input = fixture(&chain);
                        let mut producer = record("Condition:Enabled", "FLAG", json!(0));
                        producer["source"] = json!("Config");
                        producer["tags"] = json!([predicate("Child")]);
                        input["stores"][producer_layer]["mods"] = json!([producer]);
                        input["stores"][0]["conditions"]["Child"] = json!(true);
                        input["cfg"] =
                            json!({"source":"Item","ignoreSourceInCheckConditions":bypass});
                        if closed {
                            input["cfg"]["overrideCond"]["Enabled"] = json!(false);
                        }
                        input["query"] =
                            json!({"kind":"condition","variable":"Enabled","no_mod":false});
                        parity(&oracle, &input);
                        for operation in ["BASE", "INC", "more", "override", "max", "positive"] {
                            let mut trial = input.clone();
                            let mut row =
                                record("Target", operation_type(operation), json!(23.456));
                            row["tags"] = json!([predicate("Enabled")]);
                            trial["stores"][0]["mods"].as_array_mut().unwrap().push(row);
                            set_query(&mut trial, operation, json!(["Target", "Target"]));
                            parity(&oracle, &trial);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn flag_masks_source_presence_raw_overrides_and_name_order_match() {
    let _count = CountGuard::new(4560);
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for chain in CHAINS {
            for layer in 0..2 {
                for bypass in [false, true] {
                    for source in [Value::Null, json!("Item:caller"), json!("Other:caller")] {
                        let mut input = fixture(&chain);
                        let mut row = record("Condition:Enabled", "FLAG", json!(true));
                        row["source"] = source;
                        input["stores"][layer]["mods"] = json!([row]);
                        input["cfg"] =
                            json!({"source":"Item","ignoreSourceInCheckConditions":bypass});
                        for value in [Value::Null, json!(false), json!(0), json!(""), json!(true)] {
                            input["cfg"]["overrideCond"]["Enabled"] = value;
                            for no_mod in [false, true] {
                                input["query"] = json!({"kind":"condition","variable":"Enabled","no_mod":no_mod});
                                parity(&oracle, &input);
                            }
                        }
                    }
                }
            }
            for (flags, keyword) in [
                (0, 0),
                (1, 1),
                (1_u64 << 32, 0),
                (1_u64 << 52, 2),
                (5, 0x4000_0003),
            ] {
                let mut input = fixture(&chain);
                for layer in 0..2 {
                    let mut row = record("Target", "BASE", json!(7));
                    row["flags"] = json!(flags);
                    row["keywordFlags"] = json!(keyword);
                    input["stores"][layer]["mods"] = json!([row]);
                }
                for query_flags in [0, 1, 5, (1_u64 << 52) | 5, (1_u64 << 32) | 5] {
                    for query_keywords in [0, 1, 3] {
                        input["cfg"] = json!({"flags":query_flags,"keywordFlags":query_keywords});
                        for operation in ["BASE", "more", "override", "max", "positive", "flag"] {
                            for layer in 0..2 {
                                input["stores"][layer]["mods"][0]["type"] =
                                    json!(operation_type(operation));
                            }
                            set_query(
                                &mut input,
                                operation,
                                json!(["Target", "Missing", "Target"]),
                            );
                            parity(&oracle, &input);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn tabulate_preserves_order_nonzero_and_override_rows_before_max_and_positive_queries() {
    let _count = CountGuard::new(24);
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for chain in CHAINS {
            let mut input = fixture(&chain);
            input["stores"][0]["mods"] = json!([
                record("A", "MAX", json!(0)),
                record("B", "MAX", json!(2)),
                record("A", "MAX", json!(-3)),
                record("A", "OVERRIDE", json!(0))
            ]);
            input["stores"][1]["mods"] =
                json!([record("A", "MAX", json!(9)), record("B", "MAX", json!(7))]);
            input["query"] = json!({"kind":"tabulate","names":["A","B","A"]});
            let expected = Observed::Rows(vec![
                ("1:3".into(), Observed::Number(-3.0)),
                ("1:4".into(), Observed::Number(0.0)),
                ("1:2".into(), Observed::Number(2.0)),
                ("1:3".into(), Observed::Number(-3.0)),
                ("1:4".into(), Observed::Number(0.0)),
                ("2:1".into(), Observed::Number(9.0)),
                ("2:2".into(), Observed::Number(7.0)),
                ("2:1".into(), Observed::Number(9.0)),
            ]);
            same(&oracle.query(&input).unwrap(), &expected);
            set_query(&mut input, "max", json!(["A", "B", "A"]));
            same(&parity(&oracle, &input).unwrap(), &Observed::Number(9.0));
            for layer in 0..2 {
                for row in input["stores"][layer]["mods"].as_array_mut().unwrap() {
                    row["type"] = json!("BASE");
                }
            }
            set_query(&mut input, "positive", json!(["A", "B", "A"]));
            same(&parity(&oracle, &input).unwrap(), &Observed::Number(9.0)); // source ignores extra names
            input["stores"][1]["mods"] = json!([]);
            input["stores"][0]["mods"] =
                json!([record("A", "MAX", json!(0)), record("A", "MAX", json!(-3))]);
            set_query(&mut input, "max", json!(["A"]));
            assert_eq!(parity(&oracle, &input), Some(Observed::Nil));
        }
    }
}

#[test]
fn nonfinite_numeric_results_are_preserved_but_malformed_numeric_values_are_not_admitted() {
    let _count = CountGuard::new(144);
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for chain in CHAINS {
            for operation in ["BASE", "INC", "more", "override", "max", "positive"] {
                for nonfinite in ["nan", "positive_infinity", "negative_infinity"] {
                    let mut input = fixture(&chain);
                    let mut row = record("Target", operation_type(operation), json!(0));
                    row["nonfinite_value"] = json!(nonfinite);
                    input["stores"][1]["mods"] = json!([row]);
                    set_query(&mut input, operation, json!(["Target"]));
                    parity(&oracle, &input);
                }
            }
            for value in [Value::Null, json!(false)] {
                let mut input = fixture(&chain);
                input["stores"][0]["mods"] = json!([record("Target", "MORE", value.clone())]);
                set_query(&mut input, "more", json!(["Target"]));
                assert!(
                    native::query(&input)
                        .unwrap_err()
                        .contains("UnsupportedValue")
                );
                if chain[0] == "ModDB" {
                    assert_eq!(oracle.query(&input).unwrap(), Observed::Number(1.0));
                } else {
                    assert!(oracle.query(&input).is_err());
                }
                input["stores"][0]["mods"][0]["type"] = json!("OVERRIDE");
                input["stores"][1]["mods"] = json!([record("Target", "OVERRIDE", json!(0.0))]);
                set_query(&mut input, "override", json!(["Target"]));
                assert!(native::query(&input).is_err());
                same(&oracle.query(&input).unwrap(), &Observed::Number(0.0));
            }
        }
    }
}

#[test]
fn source_errors_are_reached_child_first_without_flattening_or_reversing_layers() {
    let _count = CountGuard::new(0);
    let oracle = Oracle::new(false);
    oracle.bound_instructions();
    let mut input = fixture(&["ModList", "ModDB"]);
    let mut missing = record("Target", "BASE", json!(1));
    missing["source"] = Value::Null;
    input["stores"][0]["mods"] = json!([missing]);
    let mut loop_flag = record("Condition:Loop", "FLAG", json!(true));
    loop_flag["tags"] = json!([predicate("Loop")]);
    let mut dependent = record("Target", "BASE", json!(1));
    dependent["tags"] = json!([predicate("Loop")]);
    input["stores"][1]["mods"] = json!([loop_flag, dependent]);
    input["cfg"] = json!({"source":"Item"});
    assert!(
        oracle
            .query(&input)
            .unwrap_err()
            .to_string()
            .contains("source")
    );
    let error = native::query(&input).unwrap_err();
    assert!(
        error.contains("MissingSource") && error.contains("layer: 0"),
        "{error}"
    );
    input["stores"][0]["mods"][0]["source"] = json!("Item:caller");
    assert!(oracle.query(&input).is_err());
    assert!(native::query(&input).unwrap_err().contains("Recursive"));
}

#[test]
fn frozen_captured_modlist_db_chains_replay_with_explicit_native_store_kinds() {
    let _count = CountGuard::new(34);
    let capture = corpus::authenticated_capture();
    assert_eq!(corpus::unresolved_names(&capture).len(), 4);
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        let mut count = 0;
        for case in capture["cases"].as_array().unwrap() {
            if let Ok(input) = corpus::replay_input(case) {
                parity(&oracle, &input);
                count += 1;
            }
        }
        assert_eq!(count, 15);
        // An isolated captured numeric row, with its complete captured condition
        // closure. This does not assert that all same-name numeric build rows were captured.
        for index in [10, 11] {
            let case = &capture["cases"][index];
            let mut input = corpus::replay_input(case).unwrap();
            let row = case["related_retained_consumers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["record"]["name"] == "Multiplier:DemonFlameStacks")
                .unwrap();
            let mut normalized = row["record"].clone();
            normalized["tags"] = json!([normalized["1"]]);
            normalized.as_object_mut().unwrap().remove("1");
            input["stores"][0]["mods"]
                .as_array_mut()
                .unwrap()
                .push(normalized);
            set_query(&mut input, "BASE", json!(["Multiplier:DemonFlameStacks"]));
            parity(&oracle, &input);
        }
    }
}

#[test]
fn warmed_claim_is_backed_by_completed_traces_in_both_original_store_kinds() {
    let _count = CountGuard::new(24);
    let oracle = Oracle::new(true);
    for chain in CHAINS {
        let mut input = fixture(&chain);
        for operation in ["BASE", "more", "override", "max", "positive", "flag"] {
            let mut row = record("Target", operation_type(operation), json!(17.125));
            row["tags"] = json!([predicate("Enabled")]);
            input["stores"][0]["mods"] = json!([row.clone()]);
            input["stores"][1]["mods"] = json!([row]);
            input["stores"][0]["conditions"]["Enabled"] = json!(true);
            set_query(&mut input, operation, json!(["Target"]));
            parity(&oracle, &input);
        }
    }
    // Retain all 24 mixed-chain operation pairs above. Independently compile
    // the six required original prototypes for explicit source inputs below;
    // this does not claim compilation of the entire mixed tagged chain. Fresh
    // VMs also avoid inheriting prototype blacklisting from prior mixed calls.
    for (kind, operation, source) in [
        ("ModDB", "BASE", "@src/Classes/ModDB.lua:137"),
        ("ModList", "BASE", "@src/Classes/ModList.lua:125"),
        ("ModDB", "more", "@src/Classes/ModDB.lua:214"),
        ("ModList", "more", "@src/Classes/ModList.lua:164"),
    ] {
        let oracle = Oracle::new(true);
        let mut input = fixture(&[kind]);
        input["stores"][0]["mods"] =
            json!([record("Target", operation_type(operation), json!(17.125))]);
        set_query(&mut input, operation, json!(["Target"]));
        let actual = oracle.warm_source(&input).unwrap();
        same(&actual, &native::query(&input).unwrap());
        require_live_source(&oracle, source);
    }
    let oracle = Oracle::new(true);
    let mut input = fixture(&["ModDB"]);
    input["stores"][0]["conditions"]["Enabled"] = json!(true);
    input["query"] = json!({"kind":"condition","variable":"Enabled","no_mod":false});
    assert_eq!(oracle.warm_source(&input).unwrap(), Observed::Boolean(true));
    same(&native::query(&input).unwrap(), &Observed::Boolean(true));
    require_live_source(&oracle, "@src/Classes/ModStore.lua:409");
    let oracle = Oracle::new(true);
    let mut tagged = record("Target", "BASE", json!(17.125));
    tagged["tags"] = json!([predicate("Enabled")]);
    input["query"] = json!({"kind":"eval","mod":tagged});
    assert_eq!(
        oracle.warm_source(&input).unwrap(),
        Observed::Number(17.125)
    );
    // The mixed native adapter's public queries do not expose raw EvalMod;
    // its source result is explicit here and existing tagged matrices stay paired.
    require_live_source(&oracle, "@src/Classes/ModStore.lua:490");
}

fn require_live_source(oracle: &Oracle, source: &str) {
    let traces = oracle.completed_trace_functions();
    let state = oracle.trace_state();
    eprintln!("independent mixed-store source {source}: {state:?}; {traces:?}");
    assert!(state.live > 0, "{state:?}");
    assert!(
        traces.contains(source),
        "missing completed original trace {source}: {traces:?}; {state:?}"
    );
    for _ in 0..32 {
        assert_eq!(oracle.completed_trace_functions(), traces);
        assert_eq!(oracle.trace_state(), state);
    }
}

#[test]
fn mixed_store_trace_observer_rejects_aborted_reused_and_flushed_provenance() {
    let oracle = Oracle::new(true);
    let mut input = fixture(&["ModDB"]);
    input["stores"][0]["conditions"]["Enabled"] = json!(true);
    input["query"] = json!({"kind":"condition","variable":"Enabled","no_mod":false});
    oracle.record_limit(1);
    assert_eq!(oracle.warm_source(&input).unwrap(), Observed::Boolean(true));
    let aborted = oracle.trace_state();
    eprintln!("mixed bounded recording: {aborted:?}");
    assert!(aborted.start > 0 && aborted.abort > 0, "{aborted:?}");
    assert_eq!(aborted.pending, 0);
    assert_eq!(aborted.live, 0);
    assert!(oracle.completed_trace_functions().is_empty());
    assert!(
        aborted
            .aborted_functions
            .contains("@src/Classes/ModStore.lua:409")
    );
    oracle.record_limit(4000); // Original luajit-src lj_jit.h:110 default.
    let mut input = fixture(&["ModList"]);
    input["stores"][0]["mods"] = json!([record("Target", "BASE", json!(17.125))]);
    assert_eq!(
        oracle.warm_source_with_flush(&input, false).unwrap(),
        Observed::Number(17.125)
    );
    let completed = oracle.trace_state();
    let traces = oracle.completed_trace_functions();
    eprintln!("mixed completed reused trace: {completed:?}; {traces:?}");
    assert!(
        completed.stop > aborted.stop && completed.live > 0,
        "{completed:?}"
    );
    assert!(
        completed.reused_after_abort > aborted.reused_after_abort,
        "{completed:?}"
    );
    assert!(
        traces.contains("@src/Classes/ModList.lua:125"),
        "{traces:?}"
    );
    assert!(
        !traces.contains("@src/Classes/ModStore.lua:409"),
        "{traces:?}"
    );
    oracle.flush_traces();
    let flushed = oracle.trace_state();
    assert_eq!(flushed.flush, completed.flush + 1);
    assert_eq!(flushed.pending, 0);
    assert_eq!(flushed.live, 0);
    assert!(oracle.completed_trace_functions().is_empty());
}
