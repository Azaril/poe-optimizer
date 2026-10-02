//! Pure test-only projection of authenticated complete-source observations.
//! Expected outputs remain comparison data; this helper never writes fixtures.
use serde_json::{Value, json};

pub fn project(source: &Value) -> Value {
    let cases = rows(&source["cases"]);
    assert_eq!(cases.len(), 31);
    assert_eq!(source["evidence"]["complete_load_attempts_per_jit"], 32);
    for case in cases {
        assert_eq!(case["available"], true, "{}", case["name"]);
        for field in [
            "original_functions_preserved",
            "loaded_state_preserved",
            "cached_outputs_preserved",
            "fresh_actor_construction",
            "query_state_preserved",
            "saved_specs_preserved",
        ] {
            assert_eq!(case["state"][field], true);
        }
        assert_eq!(case["state"]["source_actor_level_mutated"], false);
        assert_eq!(case["state"]["business_method_wrappers"], false);
    }
    let original = named(cases, "original-05");
    for name in ["repeat-original-05", "warm-block-high-to-original"] {
        assert_eq!(named(cases, name)["state"], original["state"]);
    }
    let sniper: Vec<_> = [
        ("original-05", "main"),
        ("block-placeholder-37", "main"),
        ("block-input-zero-placeholder-37", "main"),
        ("block-input-25-placeholder-37", "main"),
        ("block-input-negative", "main"),
        ("block-input-fractional", "main"),
        ("block-input-100", "main"),
        ("block-input-125", "main"),
        ("sniper-physical-1", "main"),
        ("sniper-physical-40", "main"),
        ("block-cannot-block-custom", "main"),
        ("sniper-calcs-effective", "calcs"),
        ("sniper-calcs-combat", "calcs"),
    ]
    .into_iter()
    .map(|(name, mode)| sniper_row(named(cases, name), mode))
    .collect();
    let mut distance = Vec::new();
    for case in cases
        .iter()
        .filter(|case| case["name"].as_str().unwrap().starts_with("distance-"))
    {
        distance.push(distance_row(case, "main"));
        if case["name"]
            .as_str()
            .unwrap()
            .starts_with("distance-calcs-")
        {
            distance.push(distance_row(case, "calcs"));
        }
    }
    assert_eq!(distance.len(), 14);
    json!({
        "schema_version": 1, "source_revision": source["source_revision"], "source_hash": source["source_hash"],
        "observer_sha256": source["evidence"]["observer_sha256"], "source_files": source["evidence"]["files"],
        "originals": source["evidence"]["originals"],
        "observed_counts": {"cases":31,"complete_load_attempts_per_jit":32,"sniper_consumers":sniper.len(),"player_distance_reference_consumers":distance.len()},
        "sniper": sniper, "player_distance_reference": distance,
    })
}

fn sniper_row(case: &Value, mode: &str) -> Value {
    let actors: Vec<_> = rows(&case["state"][mode]["actors"])
        .iter()
        .filter(|actor| actor["summon_effect_id"] == "SummonSkeletalSnipersPlayer")
        .collect();
    assert_eq!(actors.len(), 1);
    let actor = actors[0];
    assert_eq!(actor["fresh_actor"], true);
    let children: Vec<_> = rows(&actor["children"])
        .iter()
        .filter(|child| child["effect_id"] == "MinionMeleeBow")
        .collect();
    assert_eq!(children.len(), 1);
    let child = children[0];
    assert_eq!(child["effect_name"], "Basic Attack");
    assert_eq!(child["selected"], true);
    let passes = rows(&child["passes"]);
    assert_eq!(passes.len(), 1);
    let pass = &passes[0];
    assert_eq!(pass["mode"], mode.to_uppercase());
    assert_eq!(pass["flags"]["attack"], true);
    assert_eq!(
        pass["flags"]["player_minion_accuracy_equals_accuracy"],
        false
    );
    assert_eq!(pass["flags"]["skill_cannot_be_evaded"], true);
    assert_eq!(pass["skill_inherits_actor_modifiers"], true);
    assert!(rows(&pass["actor_flag_records"]).iter().any(|entry| {
        let m = &entry["mod"];
        m["source"] == "Minion Attacks always hit"
            && m["name"] == "CannotBeEvaded"
            && m["type"] == "FLAG"
            && m["value"] == 1
    }));
    json!({
        "case":case["name"], "xml_sha256":case["xml_sha256"], "mode":pass["mode"],
        "physical_level":actor["physical_level"], "effective_level":actor["effective_level"],
        "actor_level":actor["actor_level"], "actor_profile":actor["actor_profile"],
        "summon_effect_id":actor["summon_effect_id"],
        "config":{"enemy_block":case["state"]["config"]["enemy_block"],"block_records":mod_records(&case["state"]["config"]["block_records"])},
        "consumer":{"effect_id":child["effect_id"],"effect_name":child["effect_name"],"selected":child["selected"],"passes":[pass_row(pass)]},
    })
}

fn distance_row(case: &Value, mode: &str) -> Value {
    let child = &case["state"][mode]["player"];
    assert_eq!(child["effect_id"], "TwisterPlayer");
    let passes = rows(&child["passes"]);
    assert_eq!(passes.len(), 1);
    json!({
        "case":case["name"],"xml_sha256":case["xml_sha256"],"mode":mode.to_uppercase(),
        "config":{"enemy_distance":case["state"]["config"]["enemy_distance"],"distance_records":mod_records(&case["state"]["config"]["distance_records"])},
        "constants":case["state"]["constants"],"native_coverage":false,
        "consumer":{"effect_id":child["effect_id"],"effect_name":child["effect_name"],"selected":child["selected"],"passes":[pass_row(&passes[0])]},
    })
}

fn pass_row(pass: &Value) -> Value {
    assert_eq!(pass["query_state_preserved"], true);
    json!({
        "label":pass["label"], "mode":pass["mode"], "effective":pass["effective"],
        "flags":pass["flags"], "accuracy":pass["accuracy"],
        "block":{"base":pass["block"]["base"],"reduction":pass["block"]["reduction"],
            "base_records":mod_records(&pass["block"]["base_records"]),
            "reduction_records":mod_records(&pass["block"]["reduction_records"]),
            "cannot_block_records":mod_records(&pass["block"]["cannot_block_records"])},
        "flag_records":mod_records(&pass["flag_records"]),"actor_flag_records":mod_records(&pass["actor_flag_records"]),
        "inheritance_records":mod_records(&pass["inheritance_records"]),
        "skill_inherits_actor_modifiers":pass["skill_inherits_actor_modifiers"],
        "output":{"Accuracy":pass["output"]["Accuracy"],"AccuracyHitChance":pass["output"]["AccuracyHitChance"],
            "enemyBlockChance":pass["output"]["enemyBlockChance"],"HitChance":pass["output"]["HitChance"]},
    })
}

fn mod_records(value: &Value) -> Value {
    Value::Array(
        rows(value)
            .iter()
            .map(|entry| {
                let mut entry = entry.clone();
                entry["mod"]["tags"] = Value::Array(rows(&entry["mod"]["tags"]).to_vec());
                entry
            })
            .collect(),
    )
}
fn named<'a>(cases: &'a [Value], name: &str) -> &'a Value {
    let selected: Vec<_> = cases.iter().filter(|case| case["name"] == name).collect();
    assert_eq!(selected.len(), 1);
    selected[0]
}
fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(
            value.as_object().is_some_and(|map| map.is_empty()),
            "not a source list: {value}"
        );
        &[]
    }
}
