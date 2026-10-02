//! Test-only projection of complete, authenticated source observations.
//! It never updates the committed fixture and grants no production coverage.

use serde_json::{Value, json};

const SNIPER: &str = "SummonSkeletalSnipersPlayer";
const SPECTRE: &str = "SummonSpectrePlayer";

pub fn project(source: &Value) -> Value {
    let cases = source["cases"].as_array().expect("source cases");
    assert_eq!(cases.len(), 11);
    assert_eq!(source["evidence"]["complete_load_attempts_per_jit"], 12);
    let sniper: Vec<_> = [
        "sniper-physical-1",
        "sniper-physical-7",
        "original-05",
        "sniper-physical-30",
        "sniper-physical-40",
    ]
    .into_iter()
    .map(|name| canonical(cases, name, SNIPER))
    .collect();
    let spectre: Vec<_> = ["spectre-physical-7", "spectre-physical-20"]
        .into_iter()
        .map(|name| canonical(cases, name, SPECTRE))
        .collect();
    let mut counts = [0_u32; 4];
    for case in cases {
        assert_eq!(case["available"], true);
        for field in [
            "original_functions_preserved",
            "loaded_state_preserved",
            "cached_outputs_preserved",
            "fresh_actor_construction",
        ] {
            assert_eq!(case["state"][field], true);
        }
        for mode in ["main", "calcs"] {
            for actor in case["state"][mode]["actors"]
                .as_array()
                .expect("source actors")
            {
                let (canonical, offset) = match actor["summon_effect_id"].as_str() {
                    Some(SNIPER) => (&sniper, 0),
                    Some(SPECTRE) => (&spectre, 2),
                    _ => continue,
                };
                let vector = actor_vector(actor);
                let matching: Vec<_> = canonical
                    .iter()
                    .filter(|row| row["physical_level"] == vector["physical_level"])
                    .collect();
                assert_eq!(matching.len(), 1, "canonical physical level");
                let expected = matching[0];
                assert_eq!(
                    without_consumer(vector.clone()),
                    without_consumer(expected.clone()),
                    "{} {mode}",
                    case["name"]
                );
                counts[offset] += 1;
                if !vector["children"][0]["consumer"].is_null() {
                    assert_eq!(
                        &vector, expected,
                        "observed consumer {} {mode}",
                        case["name"]
                    );
                    counts[offset + 1] += 1;
                }
            }
        }
    }
    // CALCS does not evaluate the non-main Sniper actor; neither does the
    // duplicate unselected MAIN actor. Never invent a missing source consumer.
    assert_eq!(counts, [18, 8, 4, 4]);
    json!({
        "schema_version": 1,
        "source_revision": source["source_revision"],
        "source_hash": source["source_hash"],
        "observer_sha256": source["evidence"]["observer_sha256"],
        "source_files": source["evidence"]["files"],
        "owned_ability_identity": source["evidence"]["owned_ability_identity"],
        "observed_counts": {"sniper_actors":counts[0],"sniper_consumers":counts[1],"spectre_actors":counts[2],"spectre_consumers":counts[3]},
        "sniper": sniper,
        "spectre_counterexamples": spectre,
    })
}

fn canonical(cases: &[Value], name: &str, effect: &str) -> Value {
    let cases: Vec<_> = cases.iter().filter(|case| case["name"] == name).collect();
    assert_eq!(cases.len(), 1);
    let actors: Vec<_> = cases[0]["state"]["main"]["actors"]
        .as_array()
        .expect("source actors")
        .iter()
        .filter(|actor| actor["summon_effect_id"] == effect)
        .collect();
    assert_eq!(actors.len(), 1);
    let vector = actor_vector(actors[0]);
    assert!(
        !vector["children"][0]["consumer"].is_null(),
        "canonical source consumer"
    );
    vector
}

fn actor_vector(actor: &Value) -> Value {
    let mut vector = serde_json::Map::new();
    for field in [
        "summon_effect_id",
        "physical_level",
        "physical_quality",
        "effective_level",
        "actor_profile",
        "actor_level",
        "table_actor_level",
        "profile",
        "curve",
        "weapon1",
        "policy",
    ] {
        vector.insert(field.into(), actor[field].clone());
    }
    let selected: Vec<_> = actor["children"]
        .as_array()
        .expect("source children")
        .iter()
        .filter(|child| child["selected"] == true)
        .collect();
    assert_eq!(selected.len(), 1);
    let child = selected[0];
    let consumer = if child["consumer"].is_null() {
        Value::Null
    } else {
        let passes: Vec<_> = child["consumer"]["passes"].as_array().expect("source passes").iter()
            .map(|pass| json!({"label":pass["label"],"source":pass["source"],"copied_from_actor":pass["copied_from_actor"]})).collect();
        assert_eq!(passes.len(), 1);
        json!({"passes":passes})
    };
    vector.insert("children".into(), json!([{
        "effect_id":child["effect_id"],"effect_name":child["effect_name"],"effect_level":child["effect_level"],
        "actor_level":child["actor_level"],"selected":true,"consumer":consumer,
    }]));
    Value::Object(vector)
}

fn without_consumer(mut vector: Value) -> Value {
    vector["children"][0]
        .as_object_mut()
        .unwrap()
        .remove("consumer");
    vector
}
