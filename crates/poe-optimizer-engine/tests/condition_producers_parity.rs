//! FLAG/GetCondition parity against complete original ModStore/ModDB methods.
#![cfg(not(target_arch = "wasm32"))]

#[path = "support/condition_producers_corpus.rs"]
mod corpus;
#[path = "support/condition_producers_native.rs"]
mod native;
#[path = "support/condition_producers_oracle.rs"]
mod oracle;

use oracle::{Observed, Oracle};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"root":1,"stores":[{"actor":1,"conditions":{},"mods":[]}],
        "actors":[{"store":1,"links":{},"output":{}}],"cfg":{},
        "query":{"kind":"condition","variable":"Enabled","no_mod":false}})
}
fn flag(name: &str, value: Value, tags: Value) -> Value {
    json!({"name":name,"type":"FLAG","value":value,"flags":0,
        "keywordFlags":0,"source":"Item:caller","tags":tags})
}
fn condition(variable: &str, negated: bool) -> Value {
    json!({"type":"Condition","var":variable,"neg":negated})
}

#[test]
fn original_scalar_truthiness_and_no_mod_results_are_not_boolean_coercions() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for (value, expected) in [
            (Value::Null, Observed::Nil),
            (json!(false), Observed::Nil),
            (json!(true), Observed::Boolean(true)),
            (json!(0), Observed::Boolean(true)),
            (json!(-2), Observed::Boolean(true)),
            (json!(""), Observed::Boolean(true)),
            (json!("caller"), Observed::Boolean(true)),
            (json!({}), Observed::Boolean(true)),
        ] {
            let mut input = fixture();
            input["stores"][0]["mods"] = json!([flag("Enabled", value, json!([]))]);
            input["query"] = json!({"kind":"flag","names":["Enabled"]});
            assert_eq!(oracle.query(&input).unwrap(), expected, "{input}");
        }
        for (value, expected) in [
            (json!(false), Observed::Boolean(false)),
            (json!(true), Observed::Boolean(true)),
            (json!(0), Observed::Number(0.0)),
            (json!(-2), Observed::Number(-2.0)),
            (json!(""), Observed::Text(String::new())),
            (json!("caller"), Observed::Text("caller".into())),
        ] {
            let mut input = fixture();
            input["cfg"]["overrideCond"]["Enabled"] = value;
            input["stores"][0]["conditions"]["Enabled"] = json!(true);
            assert_eq!(oracle.query(&input).unwrap(), expected);
        }
        let mut absent = fixture();
        assert_eq!(oracle.query(&absent).unwrap(), Observed::Nil);
        absent["query"]["no_mod"] = json!(true);
        assert_eq!(oracle.query(&absent).unwrap(), Observed::Boolean(false));
        absent["stores"][0]["mods"] = json!([flag("Condition:Enabled", json!(true), json!([]))]);
        assert_eq!(oracle.query(&absent).unwrap(), Observed::Boolean(false));
    }
}

#[test]
fn original_parent_producers_use_child_context_and_skill_fallback_is_tag_specific() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        let mut input = fixture();
        input["stores"][0]["parent"] = json!(2);
        input["stores"][0]["conditions"]["ChildOnly"] = json!(true);
        input["stores"]
            .as_array_mut()
            .unwrap()
            .push(json!({"actor":1,"conditions":{},"mods":[
                flag("Condition:Enabled", json!(0), json!([condition("ChildOnly", false)]))
            ]}));
        assert_eq!(oracle.query(&input).unwrap(), Observed::Boolean(true));
        input["stores"][0]["conditions"]["ChildOnly"] = json!(false);
        assert_eq!(oracle.query(&input).unwrap(), Observed::Nil);
        input["stores"][1]["conditions"]["Enabled"] = json!(0);
        input["stores"][0]["conditions"]["Enabled"] = json!(false);
        assert_eq!(oracle.query(&input).unwrap(), Observed::Number(0.0));
        input["cfg"]["overrideCond"]["Enabled"] = json!(false);
        assert_eq!(oracle.query(&input).unwrap(), Observed::Boolean(false));

        let mut input = fixture();
        input["cfg"] = json!({"overrideCond":{"Gate":false},"skillCond":{"Gate":true}});
        input["stores"][0]["mods"] = json!([flag(
            "Condition:Enabled",
            json!(true),
            json!([condition("Gate", false)])
        )]);
        assert_eq!(oracle.query(&input).unwrap(), Observed::Boolean(true));
        input["stores"][0]["mods"][0]["tags"] = json!([{"type":"ActorCondition","var":"Gate"}]);
        assert_eq!(oracle.query(&input).unwrap(), Observed::Nil);
    }
}

#[test]
fn original_flag_source_filter_errors_and_query_bounds_are_explicit() {
    let oracle = Oracle::new(false);
    let mut input = fixture();
    input["stores"][0]["mods"] = json!([flag("Enabled", json!(0), json!([]))]);
    input["query"] = json!({"kind":"flag","names":["Enabled"]});
    input["cfg"]["source"] = json!("Item:caller");
    assert_eq!(oracle.query(&input).unwrap(), Observed::Nil);
    input["cfg"]["source"] = json!("Item");
    assert_eq!(oracle.query(&input).unwrap(), Observed::Boolean(true));
    input["stores"][0]["mods"][0]["source"] = Value::Null;
    assert!(oracle.query(&input).is_err());
    input["cfg"]["ignoreSourceInCheckConditions"] = json!(true);
    assert_eq!(oracle.query(&input).unwrap(), Observed::Boolean(true));
    input["query"]["names"] = json!(["A", "B", "C", "D", "E", "F", "G", "H", "I"]);
    assert!(
        oracle
            .query(&input)
            .unwrap_err()
            .to_string()
            .contains("at most 8 names")
    );
    input["query"]["names"] = json!([]);
    assert_eq!(oracle.query(&input).unwrap(), Observed::Nil);
}

#[test]
fn original_dependency_cycles_fail_under_an_explicit_interpreted_budget() {
    let oracle = Oracle::new(false);
    oracle.bound_instructions();
    let mut input = fixture();
    input["stores"][0]["mods"] = json!([flag(
        "Condition:Enabled",
        json!(true),
        json!([condition("Enabled", false)])
    )]);
    let error = oracle.query(&input).unwrap_err().to_string();
    assert!(
        error.contains("instruction budget") || error.contains("stack overflow"),
        "{error}"
    );
}

#[test]
fn warmed_source_claim_requires_completed_traces_containing_the_real_consumers() {
    let mut input = fixture();
    input["stores"][0]["conditions"]["Gate"] = json!(true);
    let modifier = flag(
        "Condition:Enabled",
        json!(0),
        json!([condition("Gate", false)]),
    );
    input["stores"][0]["mods"] = json!([modifier]);
    // Preserve the actual GetCondition -> Flag -> FlagInternal -> EvalMod query.
    let oracle = Oracle::new(true);
    assert_eq!(oracle.query(&input).unwrap(), Observed::Boolean(true));
    for (query, expected_value, expected_functions) in [
        (
            json!({"kind":"condition","variable":"Enabled","no_mod":false}),
            Observed::Boolean(true),
            vec!["@src/Classes/ModStore.lua:409"],
        ),
        (
            json!({"kind":"flag","names":["Condition:Enabled"]}),
            Observed::Boolean(true),
            vec![
                "@src/Classes/ModStore.lua:281",
                "@src/Classes/ModDB.lua:297",
            ],
        ),
        (
            json!({"kind":"eval","mod":modifier}),
            Observed::Number(0.0),
            vec!["@src/Classes/ModStore.lua:490"],
        ),
    ] {
        input["query"] = query;
        if input["query"]["kind"] == "flag" {
            input["stores"][0]["mods"][0]["tags"] = json!([]);
        }
        assert_eq!(oracle.warm_source(&input).unwrap(), expected_value);
        // Inspect while the completed traces for this control are still live,
        // before the following control flushes its independent warm-up state.
        let traced = oracle.completed_trace_functions();
        let state = oracle.trace_state();
        eprintln!(
            "source warm control {}: {state:?}; {traced:?}",
            input["query"]["kind"]
        );
        assert!(state.live > 0, "{state:?}");
        for expected in expected_functions {
            assert!(
                traced.contains(expected),
                "missing completed source trace {expected}; observed {traced:?}; {state:?}"
            );
        }
        // Repeated readout itself must be invisible to JIT activity.
        for _ in 0..32 {
            assert_eq!(oracle.completed_trace_functions(), traced);
            assert_eq!(oracle.trace_state(), state);
        }
    }
}

#[test]
fn trace_observer_drops_actual_aborts_and_flushed_completed_traces() {
    let oracle = Oracle::new(true);
    let mut input = fixture();
    input["stores"][0]["conditions"]["Enabled"] = json!(true);
    // Force real LuaJIT recording failures without modifying any source method.
    oracle.record_limit(1);
    assert_eq!(oracle.warm_source(&input).unwrap(), Observed::Boolean(true));
    let aborted = oracle.trace_state();
    eprintln!("bounded recording: {aborted:?}");
    assert!(aborted.start > 0 && aborted.abort > 0, "{aborted:?}");
    assert_eq!(aborted.pending, 0);
    assert_eq!(aborted.live, 0);
    assert!(oracle.completed_trace_functions().is_empty());
    assert!(
        aborted
            .aborted_functions
            .contains("@src/Classes/ModStore.lua:409")
    );
    oracle.record_limit(4000); // Original luajit-src lj_jit.h:110 default, not a larger budget.
    input["query"] = json!({"kind":"flag","names":["Plain"]});
    input["stores"][0]["mods"] = json!([flag("Plain", json!(true), json!([]))]);
    // Reuse an actual aborted ID without a flush; the previous GetCondition
    // recording must not be attributed to this completed plain Flag trace.
    assert_eq!(
        oracle.warm_source_with_flush(&input, false).unwrap(),
        Observed::Boolean(true)
    );
    let completed = oracle.trace_state();
    assert!(
        completed.stop > aborted.stop && completed.live > 0,
        "{completed:?}"
    );
    assert!(
        completed.reused_after_abort > aborted.reused_after_abort,
        "{completed:?}"
    );
    let traced = oracle.completed_trace_functions();
    assert!(
        traced.contains("@src/Classes/ModStore.lua:281"),
        "{traced:?}"
    );
    assert!(
        !traced.contains("@src/Classes/ModStore.lua:409"),
        "{traced:?}"
    );
    eprintln!("completed reused trace: {completed:?}; {traced:?}");
    oracle.flush_traces();
    let flushed = oracle.trace_state();
    assert_eq!(flushed.flush, completed.flush + 1);
    assert_eq!(flushed.live, 0);
    assert_eq!(flushed.pending, 0);
    assert!(oracle.completed_trace_functions().is_empty());
}

fn parity(oracle: &Oracle, input: &Value) {
    let expected = oracle
        .query(input)
        .unwrap_or_else(|e| panic!("original source: {e}; {input}"));
    let actual = native::query(input).unwrap_or_else(|e| panic!("native: {e}; {input}"));
    match (&actual, &expected) {
        (Observed::Number(a), Observed::Number(b)) => {
            assert_eq!(a.to_bits(), b.to_bits(), "{input}")
        }
        _ => assert_eq!(actual, expected, "{input}"),
    }
}
#[test]
fn native_raw_values_parent_precedence_and_flag_mask_matrix_match_original() {
    use poe_optimizer_engine::modifiers::KEYWORD_MATCH_ALL;
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for value in [
            Value::Null,
            json!(false),
            json!(true),
            json!(0),
            json!(-0.0),
            json!(-17.5),
            json!(""),
            json!("caller-value"),
        ] {
            for location in ["flag", "local", "parent", "override"] {
                for no_mod in [false, true] {
                    let mut input = fixture();
                    input["query"]["no_mod"] = json!(no_mod);
                    match location {
                        "flag" => {
                            input["stores"][0]["mods"] =
                                json!([flag("Condition:Enabled", value.clone(), json!([]))])
                        }
                        "local" => input["stores"][0]["conditions"]["Enabled"] = value.clone(),
                        "override" => {
                            input["cfg"]["overrideCond"]["Enabled"] = value.clone();
                            input["stores"][0]["conditions"]["Enabled"] = json!(true);
                        }
                        _ => {
                            input["stores"][0]["conditions"]["Enabled"] = json!(false);
                            input["stores"][0]["parent"] = json!(2);
                            input["stores"]
                                .as_array_mut()
                                .unwrap()
                                .push(json!({"actor":1,"conditions":{"Enabled":value},"mods":[]}));
                        }
                    }
                    parity(&oracle, &input);
                }
            }
        }
        for flag_bits in [0, 1, 1_u64 << 32, 1_u64 << 52, (1_u64 << 32) | 1] {
            for query_bits in [0, 1, 1_u64 << 32, (1_u64 << 52) | (1_u64 << 32) | 1] {
                for keyword_bits in [0, 1, 3, KEYWORD_MATCH_ALL | 3] {
                    for query_keywords in [0, 1, 3] {
                        let mut input = fixture();
                        let mut record = flag("Enabled", json!(0), json!([]));
                        record["flags"] = json!(flag_bits);
                        record["keywordFlags"] = json!(keyword_bits);
                        input["stores"][0]["mods"] = json!([record]);
                        input["cfg"] = json!({"flags":query_bits,"keywordFlags":query_keywords});
                        input["query"] = json!({"kind":"flag","names":["Missing","Enabled"]});
                        parity(&oracle, &input);
                    }
                }
            }
        }
    }
}
#[test]
fn native_recursive_actor_skill_and_weapon_predicates_match_original() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for gate in [Value::Null, json!(false), json!(true), json!(0), json!("")] {
            for negated in [false, true] {
                for actor_tag in [false, true] {
                    let mut input = fixture();
                    input["stores"][0]["parent"] = json!(2);
                    input["stores"][0]["conditions"]["Gate"] = gate.clone();
                    input["cfg"]["skillCond"]["Gate"] = json!(true);
                    let tag = if actor_tag {
                        json!({"type":"ActorCondition","var":"Gate","neg":negated})
                    } else {
                        condition("Gate", negated)
                    };
                    input["stores"].as_array_mut().unwrap().push(json!({"actor":1,"conditions":{},"mods":[flag("Condition:Enabled",json!(0),json!([tag]))]}));
                    parity(&oracle, &input);
                }
            }
        }
        for role in ["parent", "enemy", "player", "absent"] {
            for link in ["direct", "through-parent", "through-enemy", "none"] {
                for negated in [false, true] {
                    let mut input = fixture();
                    input["stores"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"actor":2,"conditions":{"Remote":0},"mods":[]}));
                    input["stores"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"actor":3,"conditions":{},"mods":[]}));
                    input["actors"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"store":2,"links":{},"output":{}}));
                    input["actors"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"store":3,"links":{"player":2},"output":{}}));
                    match link {
                        "direct" => input["actors"][0]["links"][role] = json!(2),
                        "through-parent" => input["actors"][0]["links"]["parent"] = json!(3),
                        "through-enemy" => input["actors"][0]["links"]["enemy"] = json!(3),
                        _ => {}
                    }
                    input["cfg"]["actor"] = json!(role);
                    input["stores"][0]["mods"] = json!([flag(
                        "Condition:Enabled",
                        json!(true),
                        json!([{"type":"ActorCondition","actor":role,"var":"Remote","neg":negated}])
                    )]);
                    parity(&oracle, &input);
                    input["stores"][0]["mods"][0]["tags"][0]
                        .as_object_mut()
                        .unwrap()
                        .remove("var");
                    parity(&oracle, &input);
                }
            }
        }
        for one in [false, true] {
            for two in [false, true] {
                for added in [Value::Null, json!(false), json!(true)] {
                    for list in [false, true] {
                        let mut input = fixture();
                        input["actors"][0]["weapon_one"] =
                            json!({"countsAsAll1H":one,"AddedGate":added});
                        input["actors"][0]["weapon_two"] =
                            json!({"countsAsAll1H":two,"AddedGate":true});
                        input["stores"][0]["conditions"]["Gate"] = json!(true);
                        let tag = if list {
                            json!({"type":"Condition","varList":["Gate","Missing"],"neg":true})
                        } else {
                            condition("Gate", true)
                        };
                        input["stores"][0]["mods"] =
                            json!([flag("Condition:Enabled", json!(true), json!([tag]))]);
                        parity(&oracle, &input);
                    }
                }
            }
        }
    }
}
#[test]
fn native_threshold_producers_use_selected_store_stats_and_source_order() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for life in [0.0, 1.5, 2.0, 2.5] {
            for upper in [false, true] {
                for parent in [false, true] {
                    let mut input = fixture();
                    input["actors"][0]["output"]["Life"] = json!(life);
                    input["cfg"]["skillStats"]["Life"] = json!(100);
                    let record = flag(
                        "Condition:Enabled",
                        json!(0),
                        json!([{"type":"StatThreshold","stat":"Life","threshold":2,"upper":upper}]),
                    );
                    if parent {
                        input["stores"][0]["parent"] = json!(2);
                        input["stores"]
                            .as_array_mut()
                            .unwrap()
                            .push(json!({"actor":2,"conditions":{},"mods":[record]}));
                        input["actors"]
                            .as_array_mut()
                            .unwrap()
                            .push(json!({"store":2,"output":{"Life":100},"links":{}}));
                    } else {
                        input["stores"][0]["mods"] = json!([record]);
                    }
                    parity(&oracle, &input);
                }
            }
        }
        for percent in [Value::Null, json!(0), json!(50), json!(100)] {
            let mut input = fixture();
            input["actors"][0]["output"] = json!({"Life":5,"Mana":3,"Limit":16});
            input["stores"][0]["mods"] = json!([flag(
                "Condition:Enabled",
                json!(true),
                json!([{"type":"StatThreshold","statList":["Life","Mana"],"thresholdStat":"Limit","thresholdPercent":percent}])
            )]);
            parity(&oracle, &input);
        }
    }
}
#[test]
fn native_producer_errors_remain_explicit_and_do_not_become_false_conditions() {
    let oracle = Oracle::new(false);
    let mut input = fixture();
    input["stores"][0]["mods"] = json!([flag(
        "Condition:Enabled",
        json!(true),
        json!([condition("Enabled", false)])
    )]);
    oracle.bound_instructions();
    assert!(oracle.query(&input).is_err());
    assert!(native::query(&input).unwrap_err().contains("Recursive"));
    input["cfg"]["overrideCond"]["Enabled"] = json!(false);
    assert_eq!(native::query(&input).unwrap(), Observed::Boolean(false));
    input["cfg"] = json!({});
    input["stores"][0]["mods"][0]["tags"] =
        json!([condition("Closed", false), condition("Enabled", false)]);
    assert_eq!(native::query(&input).unwrap(), Observed::Nil);
    for tags in [
        json!([{"type":"PerStat","stat":"Strength","div":5}]),
        json!([{"type":"StatThreshold","stat":"ManaReservedPercent","threshold":1}]),
        json!([{"type":"StatThreshold","stat":"Life","threshold":2,"thresholdPercentVar":"External"}]),
    ] {
        input["stores"][0]["mods"][0]["tags"] = tags;
        assert!(native::query(&input).is_err());
    }
    input["stores"][0]["mods"][0]["tags"] = json!([]);
    input["stores"][0]["mods"][0]["value"] = json!({"unsupported":"table"});
    assert!(native::query(&input).is_err());
    input["stores"][0]["mods"][0]["value"] = json!(true);
    input["stores"][0]["mods"][0]["flags"] = json!(1_u64 << 31);
    assert!(native::query(&input).is_err());
}

#[test]
fn native_flag_source_filter_and_presence_errors_match_original() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for source in [
            Value::Null,
            json!("Item:caller"),
            json!(":Item:caller"),
            json!("Other:caller"),
            json!(""),
        ] {
            for requested in [Value::Null, json!("Item"), json!("Item:caller"), json!("")] {
                for bypass in [false, true] {
                    let mut input = fixture();
                    let mut record = flag("Enabled", json!(0), json!([]));
                    record["source"] = source.clone();
                    input["stores"][0]["mods"] = json!([record]);
                    input["cfg"] =
                        json!({"source":requested,"ignoreSourceInCheckConditions":bypass});
                    input["query"] = json!({"kind":"flag","names":["Enabled"]});
                    match oracle.query(&input) {
                        Ok(expected) => {
                            assert_eq!(native::query(&input).unwrap(), expected, "{input}")
                        }
                        Err(_) => assert!(native::query(&input).is_err(), "{input}"),
                    }
                }
            }
        }
    }
}
#[test]
fn native_flag_chains_drive_numeric_queries_without_flattening_action_context() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for family in [
            "CompanionInPresence",
            "Empowered",
            "caller-renamed-condition",
        ] {
            for active in [false, true] {
                for local_override in [Value::Null, json!(false), json!(0)] {
                    let mut input = fixture();
                    input["stores"][0]["parent"] = json!(2);
                    input["stores"][0]["conditions"][family] = json!(active);
                    input["cfg"]["overrideCond"]["Enabled"] = local_override;
                    let mut numeric = flag(
                        "CallerStat",
                        json!(17.5),
                        json!([condition("Enabled", false)]),
                    );
                    numeric["type"] = json!("BASE");
                    input["stores"][0]["mods"] = json!([numeric.clone()]);
                    input["stores"].as_array_mut().unwrap().push(json!({"actor":1,"conditions":{},"mods":[
                        flag("Condition:Enabled",json!(3000),json!([condition(family,false)])),numeric
                    ]}));
                    input["query"] =
                        json!({"kind":"sum","operation":"BASE","names":["CallerStat"]});
                    parity(&oracle, &input);
                }
            }
        }
        // A target actor's flag must resolve its own condition store, not the
        // originating player's store, even when all queried names coincide.
        let mut input = fixture();
        input["actors"][0]["links"]["enemy"] = json!(2);
        input["actors"]
            .as_array_mut()
            .unwrap()
            .push(json!({"store":2,"links":{},"output":{}}));
        input["stores"].as_array_mut().unwrap().push(json!({"actor":2,"conditions":{"RareOrUnique":true},"mods":[flag("Condition:CanCull",json!(true),json!([condition("RareOrUnique",false)]))]}));
        let mut numeric = flag(
            "CallerStat",
            json!(25),
            json!([{"type":"ActorCondition","actor":"enemy","var":"CanCull"}]),
        );
        numeric["type"] = json!("INC");
        input["stores"][0]["mods"] = json!([numeric]);
        input["query"] = json!({"kind":"sum","operation":"INC","names":["CallerStat"]});
        parity(&oracle, &input);
        input["stores"][1]["conditions"]["RareOrUnique"] = json!(false);
        parity(&oracle, &input);
    }
}

#[test]
fn source_and_native_preserve_layer_name_and_tag_short_circuit_order() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        let mut input = fixture();
        input["stores"][0]["parent"] = json!(2);
        input["stores"][0]["mods"] = json!([flag("LaterName", json!(true), json!([]))]);
        input["stores"].as_array_mut().unwrap().push(json!({"actor":1,"conditions":{},"mods":[flag("FirstName",json!(true),json!([condition("Loop",false)])),flag("Condition:Loop",json!(true),json!([condition("Loop",false)]))]}));
        input["query"] = json!({"kind":"flag","names":["FirstName","LaterName"]});
        // All local names are checked before any parent's names; querying each
        // name across every layer would reach the deliberate parent cycle.
        parity(&oracle, &input);
        input["query"] = json!({"kind":"condition","variable":"Loop"});
        input["stores"][0]["conditions"]["Loop"] = json!(0);
        parity(&oracle, &input);
        input["stores"][0]["conditions"] = json!({});
        input["stores"][1]["mods"][1]["tags"] =
            json!([condition("Closed", false), condition("Loop", false)]);
        parity(&oracle, &input);
    }
    for value in [Value::Null, json!(false), json!(0)] {
        let oracle = Oracle::new(false);
        oracle.bound_instructions();
        let mut input = fixture();
        input["stores"][0]["mods"] = json!([flag(
            "Condition:Enabled",
            value,
            json!([condition("Enabled", false)])
        )]);
        // EvalMod runs before FLAG tests the resulting value's truthiness.
        assert!(oracle.query(&input).is_err());
        assert!(native::query(&input).unwrap_err().contains("Recursive"));
    }
}

#[test]
fn captured_build_dependency_closures_match_original_without_claiming_full_build_parity() {
    let capture = corpus::authenticated_capture();
    assert_eq!(
        corpus::unresolved_names(&capture),
        ["CanSprint", "Burning", "CanGainRage", "Poisoned"]
            .into_iter()
            .collect()
    );
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        let mut compared = 0;
        let mut unresolved = 0;
        for (index, case) in capture["cases"].as_array().unwrap().iter().enumerate() {
            match corpus::replay_input(case) {
                Ok(input) => {
                    let original = oracle.query(&input).unwrap();
                    let native = native::query(&input).unwrap();
                    assert_eq!(
                        native, original,
                        "captured case {index}, JIT={warm}: {input}"
                    );
                    compared += 1;
                }
                Err(reason) => {
                    assert!(!reason.is_empty());
                    assert_eq!(case["closure_status"], "unresolved");
                    unresolved += 1;
                }
            }
        }
        assert_eq!((compared, unresolved), (15, 4));
        if warm {
            let traced = oracle.completed_trace_functions();
            for consumer in [
                "@src/Classes/ModList.lua:229",
                "@src/Classes/ModStore.lua:409",
                "@src/Classes/ModStore.lua:490",
            ] {
                assert!(
                    traced.contains(consumer),
                    "captured replay did not complete a source trace for {consumer}: {traced:?}"
                );
            }
        }
    }
    // The unresolved Rage closure keeps every original duplicate and transfer
    // record. It must not become admissible by selecting only a true first row.
    let rage = &capture["cases"][15]["dependency_nodes"][0]["all_same_name_rows"];
    assert_eq!(rage.as_array().unwrap().len(), 3);
    assert!(
        rage.as_array()
            .unwrap()
            .iter()
            .any(|r| r["record"]["1"]["type"] == "GlobalEffect")
    );
}
