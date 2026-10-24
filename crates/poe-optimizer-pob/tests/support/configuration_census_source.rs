//! Project the existing full-source observation, without interpreting Lua.
use flate2::read::GzDecoder;
use poe_optimizer_pob::source as pinned;
use serde_json::{Value, json};
use std::{fs, io::Read, path::Path};

const PACKET: &str = "data/owned/poe2/3887ae68/configuration-census/catalogue.json.gz";
const DISPATCH_KEYS: [&str; 4] = [
    "detonateDeadCorpseLife",
    "conditionMoving",
    "plagueBearerState",
    "windDancerStacks",
];

pub(super) fn dispatcher_controls(xml: &str) -> Vec<(String, String)> {
    assert_eq!(super::remove_keys(xml, &DISPATCH_KEYS), xml);
    // Reverse catalogue order deliberately: the original dispatcher, not XML
    // insertion order, determines the four observed Config record positions.
    [
        (
            "dispatcher-false-zero-first",
            concat!(
                "<Input name=\"windDancerStacks\" number=\"0\"/>",
                "<Input name=\"plagueBearerState\" string=\"INC\"/>",
                "<Input name=\"conditionMoving\" boolean=\"false\"/>",
                "<Input name=\"detonateDeadCorpseLife\" number=\"0\"/>",
            ),
        ),
        (
            "dispatcher-true-nonzero-second",
            concat!(
                "<Input name=\"windDancerStacks\" number=\"3\"/>",
                "<Input name=\"plagueBearerState\" string=\"INF\"/>",
                "<Input name=\"conditionMoving\" boolean=\"true\"/>",
                "<Input name=\"detonateDeadCorpseLife\" number=\"17\"/>",
            ),
        ),
        (
            "dispatcher-numeric-placeholders",
            concat!(
                "<Placeholder name=\"windDancerStacks\" number=\"7\"/>",
                "<Placeholder name=\"detonateDeadCorpseLife\" number=\"11\"/>",
            ),
        ),
        (
            "dispatcher-zero-with-placeholders",
            concat!(
                "<Placeholder name=\"windDancerStacks\" number=\"7\"/>",
                "<Input name=\"windDancerStacks\" number=\"0\"/>",
                "<Placeholder name=\"detonateDeadCorpseLife\" number=\"11\"/>",
                "<Input name=\"detonateDeadCorpseLife\" number=\"0\"/>",
            ),
        ),
    ]
    .map(|(name, body)| (name.to_owned(), super::add(xml, body)))
    .into()
}

fn config_record(name: &str, kind: &str, value: Value) -> Value {
    json!({
        "name":name,"type":kind,"value":value,"source":"Config",
        "flags":0,"keyword_flags":0,"tags":{},
        "all_fields":{"name":name,"type":kind,"value":value,"source":"Config","flags":0,"keywordFlags":0}
    })
}

struct DispatcherCase {
    name: &'static str,
    input: Value,
    placeholder: Value,
    records: Vec<Value>,
    list_selection: u64,
}

fn dispatcher_cases() -> [DispatcherCase; 4] {
    let corpse = |value| {
        config_record(
            "SkillData",
            "LIST",
            json!({"key":"corpseLife","value":value}),
        )
    };
    let stacks = |value| config_record("Multiplier:WindDancerStacks", "BASE", json!(value));
    [
        DispatcherCase {
            name: "dispatcher-false-zero-first",
            input: json!({"windDancerStacks":0,"plagueBearerState":"INC","conditionMoving":false,"detonateDeadCorpseLife":0}),
            placeholder: json!({}),
            records: vec![
                config_record("Condition:PlagueBearerIncubating", "FLAG", json!(true)),
                stacks(0),
            ],
            list_selection: 1,
        },
        DispatcherCase {
            name: "dispatcher-true-nonzero-second",
            input: json!({"windDancerStacks":3,"plagueBearerState":"INF","conditionMoving":true,"detonateDeadCorpseLife":17}),
            placeholder: json!({}),
            records: vec![
                corpse(17),
                config_record("Condition:Moving", "FLAG", json!(true)),
                config_record("Condition:PlagueBearerInfecting", "FLAG", json!(true)),
                stacks(3),
            ],
            list_selection: 2,
        },
        DispatcherCase {
            name: "dispatcher-numeric-placeholders",
            input: json!({}),
            placeholder: json!({"windDancerStacks":7,"detonateDeadCorpseLife":11}),
            records: vec![corpse(11), stacks(7)],
            list_selection: 1,
        },
        DispatcherCase {
            name: "dispatcher-zero-with-placeholders",
            input: json!({"windDancerStacks":0,"detonateDeadCorpseLife":0}),
            placeholder: json!({"windDancerStacks":7,"detonateDeadCorpseLife":11}),
            records: vec![corpse(11), stacks(0)],
            list_selection: 1,
        },
    ]
}

fn selected_record(record: &Value) -> bool {
    record["source"] == "Config"
        && (record["name"] == "Condition:Moving"
            || record["name"] == "Condition:PlagueBearerIncubating"
            || record["name"] == "Condition:PlagueBearerInfecting"
            || record["name"] == "Multiplier:WindDancerStacks"
            || (record["name"] == "SkillData" && record["value"]["key"] == "corpseLife"))
}

fn raw_dispatch_entries(state: &Value) -> Vec<&Value> {
    super::rows(&state["raw"])
        .iter()
        .flat_map(|config| super::rows(&config["sets"]))
        .flat_map(|set| super::rows(&set["entries"]))
        .filter(|entry| {
            entry["xml"]["attrib"]["name"]
                .as_str()
                .is_some_and(|name| DISPATCH_KEYS.contains(&name))
        })
        .collect()
}

fn list_selection(saved: &Value) -> u64 {
    let controls: Vec<_> = super::rows(&saved["controls"])
        .iter()
        .filter(|control| control["key"] == "plagueBearerState")
        .collect();
    assert_eq!(controls.len(), 1);
    controls[0]["scalars"]["selIndex"].as_u64().unwrap()
}

fn check_dispatcher(report: &Value) {
    let original = super::state(report, "original-05");
    let base = &original["saved"];
    for (index, key, kind, first, last) in [
        (3, DISPATCH_KEYS[0], "count", 114, 116),
        (6, DISPATCH_KEYS[1], "check", 133, 135),
        (155, DISPATCH_KEYS[2], "list", 514, 520),
        (253, DISPATCH_KEYS[3], "countAllowZero", 749, 751),
    ] {
        let row = &original["catalogue"][index - 1];
        let definition = &row["definition"];
        assert_eq!(row["index"], index);
        assert_eq!(definition["var"], key);
        assert_eq!(definition["type"], kind);
        assert_eq!(definition["apply"]["function_source"]["first"], first);
        assert_eq!(definition["apply"]["function_source"]["last"], last);
        for field in ["defaultState", "defaultPlaceholderState", "defaultIndex"] {
            assert!(!definition.as_object().unwrap().contains_key(field));
        }
    }
    assert!(raw_dispatch_entries(original).is_empty());
    for key in DISPATCH_KEYS {
        assert!(!base["input"].as_object().unwrap().contains_key(key));
        assert!(!base["placeholder"].as_object().unwrap().contains_key(key));
    }
    assert!(
        !super::rows(&base["player_mods"])
            .iter()
            .any(selected_record)
    );
    // UI fallback remains visible even though neither raw lane supplies a
    // value and the actual dispatcher emits none of these records.
    assert_eq!(base["control_defaults"]["detonateDeadCorpseLife"], 0);
    assert_eq!(base["control_defaults"]["conditionMoving"], false);
    assert_eq!(base["control_defaults"]["windDancerStacks"], 0);
    assert_eq!(base["control_defaults"]["plagueBearerState"], "INC");
    assert_eq!(list_selection(base), 1);
    for control in dispatcher_cases() {
        let state = super::state(report, control.name);
        let saved = &state["saved"];
        let mut raw_expected = Vec::new();
        for (lane, additions) in [
            ("input", &control.input),
            ("placeholder", &control.placeholder),
        ] {
            let mut expected = base[lane].clone();
            for (key, value) in additions.as_object().unwrap() {
                expected[key] = value.clone();
                let (field, text) = match value {
                    Value::Bool(value) => ("boolean", value.to_string()),
                    Value::Number(value) => ("number", value.to_string()),
                    Value::String(value) => ("string", value.clone()),
                    _ => panic!("finite dispatcher source scalar"),
                };
                let mut attributes = json!({"name":key});
                attributes[field] = text.into();
                raw_expected.push(json!({"elem":if lane=="input" {"Input"} else {"Placeholder"},"attrib":attributes,"empty":true}));
            }
            super::same(
                &saved[lane],
                &expected,
                &format!("{} exact {lane}", control.name),
            );
            super::same(
                &super::case(report, control.name)["load"][lane],
                &saved[lane],
                &format!("{} load {lane}", control.name),
            );
        }
        let raw: Vec<_> = raw_dispatch_entries(state)
            .into_iter()
            .map(|entry| &entry["xml"])
            .collect();
        assert_eq!(raw.len(), raw_expected.len(), "{} raw count", control.name);
        for expected in raw_expected {
            assert_eq!(
                raw.iter().filter(|actual| ***actual == expected).count(),
                1,
                "{} raw occurrence {expected}",
                control.name
            );
        }
        let records: Vec<_> = super::rows(&saved["player_mods"])
            .iter()
            .filter(|record| selected_record(record))
            .cloned()
            .collect();
        assert_eq!(
            records, control.records,
            "{} complete typed records in catalogue order",
            control.name
        );
        let remaining: Vec<_> = super::rows(&saved["player_mods"])
            .iter()
            .filter(|record| !selected_record(record))
            .cloned()
            .collect();
        super::same(
            &json!(remaining),
            &base["player_mods"],
            &format!("{} all other Player Config records/order", control.name),
        );
        super::same(
            &saved["enemy_mods"],
            &base["enemy_mods"],
            &format!("{} all Enemy Config records/order", control.name),
        );
        super::same(
            &saved["control_defaults"],
            &base["control_defaults"],
            &format!("{} UI fallback metadata", control.name),
        );
        assert_eq!(list_selection(saved), control.list_selection);
    }
}

fn portable_sources(value: &mut Value) {
    match value {
        Value::Array(values) => values.iter_mut().for_each(portable_sources),
        Value::Object(fields) => {
            if let Some(source) = fields.get_mut("function_source") {
                let path = source["path"].as_str().unwrap();
                let (_, path) = path
                    .split_once("/vendor/path-of-building-poe2/")
                    .expect("original callback has a pinned source path");
                assert!(pinned::expected_file_sha256(path).is_ok(), "{path}");
                source["path"] = path.into();
            }
            fields.values_mut().for_each(portable_sources);
        }
        _ => {}
    }
}

pub(super) fn check(root: &Path, out: &Path, report: &Value, enabled: bool) {
    check_dispatcher(report);
    // The package stores source facts, not selected build outputs. Every
    // successful original/control load must reproduce the same full catalogue.
    let file = fs::File::open(root.join(PACKET)).unwrap();
    assert!(file.metadata().unwrap().len() <= 128 * 1024);
    let mut bytes = Vec::new();
    GzDecoder::new(file)
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 2 * 1024 * 1024);
    let expected: Value = serde_json::from_slice(&bytes).unwrap();
    let mut checked = 0;
    for case in report["cases"].as_array().unwrap() {
        if case["available"] != true {
            continue;
        }
        let mut rows = case["state"]["catalogue"].clone();
        portable_sources(&mut rows);
        let actual = json!({
            "source_revision": report["source_revision"],
            "source_hash": report["source_hash"],
            "rows": rows,
        });
        assert_eq!(actual, expected, "catalogue changed in {}", case["name"]);
        checked += 1;
    }
    assert!(checked >= 5, "all originals are checked by the parent law");
    let receipt = json!({
        "source_revision": report["source_revision"],
        "source_hash": report["source_hash"],
        "attempts": report["evidence"]["complete_load_attempts_per_jit"],
        "successful_catalogue_comparisons": checked,
        "catalogue_sha256": super::digest(&bytes),
        "observer_sha256": report["evidence"]["observer_sha256"],
        "native_effect_coverage": false,
    });
    fs::write(
        out.join(format!(
            "configuration-census-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
}
