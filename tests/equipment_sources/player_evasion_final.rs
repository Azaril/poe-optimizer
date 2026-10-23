//! Player Evasion source gate and finite native metric proof.
#[allow(unused_imports)]
use super::evasion_family as family;
#[allow(unused_imports)]
use super::evasion_native as native;
#[allow(unused_imports)]
use super::migration_preservation;
#[allow(unused_imports)]
use super::replay;
#[allow(unused_imports)]
use super::{preservation, publication, release, selected};
use serde_json::{Value, json};
use std::path::PathBuf;

#[test]
fn authenticated_evasion_source_and_authored_formula() {
    family::check_authored();
}

#[test]
fn evasion_suppliers_keep_zero_inactive_conditions_and_recipient_identity() {
    let mut capture = family::capture(0);
    family::review_capture(&capture, 0).unwrap();
    for name in [
        "Evasion",
        "ArmourAndEvasion",
        "EvasionAndEnergyShield",
        "Defences",
        "EvasionTotal",
        "EvasionPerLevel",
        "ArmourData",
        "LifeGainAsEvasion",
        "ArmourConvertToEvasion",
        "Multiplier:Tailwind",
    ] {
        capture["state"]["baseline"][0]["player_store"][0]["buckets"]["Synthetic"] = json!([{"body":{"name":name,"type":"BASE","value":0,"source":"Synthetic","flags":0,"keywordFlags":0,"_positions":[{"index":1,"value":{"type":"Condition","var":"Never"}}]},"position":1}]);
        assert!(family::review_capture(&capture, 0).is_err(), "{name}");
        capture["state"]["baseline"][0]["player_store"][0]["buckets"]
            .as_object_mut()
            .unwrap()
            .remove("Synthetic");
    }
    for name in ["Maimed", "Blinded", "Tailwind"] {
        capture["state"]["environments"][0]["player_store"][0]["conditions"][name] = json!(false);
        assert!(family::review_capture(&capture, 0).is_err());
        capture["state"]["environments"][0]["player_store"][0]["conditions"]
            .as_object_mut()
            .unwrap()
            .remove(name);
    }
    let b = capture["state"]["environments"][0]["player_store"][0]["buckets"]["Evasion"].clone();
    capture["state"]["environments"][0]["player_store"][0]["buckets"]["Evasion"][0]["body"]["source"] =
        json!("Enemy");
    assert!(family::review_capture(&capture, 0).is_err());
    capture["state"]["environments"][0]["player_store"][0]["buckets"]["Evasion"] = b;
    family::review_capture(&capture, 0).unwrap();
}

#[test]
fn exact_selected_item_profiles_and_source_slots_are_required() {
    let d = family::domain();
    let mut p: Value = serde_json::from_slice(
        &std::fs::read(
            family::root().join(d["defence_profiles"]["file"]["path"].as_str().unwrap()),
        )
        .unwrap(),
    )
    .unwrap();
    family::review_profiles(&p).unwrap();
    let at = p["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .position(|r| r["base"] == "Rope Cuffs")
        .unwrap();
    for field in ["Evasion", "EvasionPerLevel"] {
        p["profiles"][at]["profile"]["value"]["fields"][field] = json!(0);
        assert!(family::review_profiles(&p).is_err());
        p["profiles"][at]["profile"]["value"]["fields"]
            .as_object_mut()
            .unwrap()
            .remove(field);
    }
    let mut recipes: Value = serde_json::from_slice(
        &std::fs::read(family::root().join(d["selected_inputs"]["file"]["path"].as_str().unwrap()))
            .unwrap(),
    )
    .unwrap();
    family::review_recipes(&recipes).unwrap();
    recipes["items"][1]["base_name"] = json!("Suede Bracers");
    assert!(family::review_recipes(&recipes).is_err());
}

fn fixture() -> (Value, Value) {
    let d: Value = family::read("dependencies.json");
    let mut defs = d["definitions"].as_array().unwrap().clone();
    defs.extend(d["donor_definitions"].as_array().unwrap().clone());
    let mut owners = vec![d["shared_owner"].clone()];
    for w in d["potential_writers"].as_array().unwrap() {
        if owners.iter().any(|o| o["owner"] == w["owner"]) {
            continue;
        }
        let owner=d["source_owners"].as_array().unwrap().iter().find(|o|o["owner"]==w["owner"]).cloned().unwrap_or_else(||json!({"owner":w["owner"],"programs":{"members":[w["program"]],"closure":{"kind":"complete"}}}));
        owners.push(owner);
    }
    // Preserve the exact writer census order, which is distinct from fixture owner order.
    owners.sort_by_key(|o| {
        d["potential_writers"]
            .as_array()
            .unwrap()
            .iter()
            .position(|w| w["owner"] == o["owner"])
            .unwrap_or(usize::MAX)
    });
    (
        json!({"definitions":defs}),
        json!({"owners":owners,"effect_applications":d["effect_applications"],"support_discovery":{"providers":d["source_support_domains"]},"existing_actor_rules":d["existing_actor_rules"],"contribution_queries":{"closure":d["query_registry_closure"]},"receivers":{"closure":d["receiver_registry_closure"]}}),
    )
}
#[test]
fn complete_native_writer_census_rejects_zero_inactive_and_application_additions() {
    let (schema, r) = fixture();
    family::check_native(&schema, &r, false);
    for edit in 0..4 {
        let mut changed = r.clone();
        let mut p = json!(family::consumer().intrinsic);
        p["id"] = json!("synthetic.unselected-evasion");
        p["nodes"][0]["expression"]["value"]["value"]["value"] = json!(0);
        if edit == 1 {
            p["nodes"].as_array_mut().unwrap().push(json!({"id":"inactive","expression":{"kind":"literal","value":{"kind":"boolean","value":false}}}));
            p["effects"][0]["when"] = json!("inactive");
        }
        if edit == 2 {
            p["effects"][0]["effect"] = json!({"kind":"derive","entity":"current","stat":json!(family::consumer().owner.programs.members[0].effects[0])["effect"]["stat"],"value":"base"});
        }
        if edit == 3 {
            changed["effect_applications"]["members"][0]["program"] = p;
        } else {
            changed["owners"].as_array_mut().unwrap().push(json!({"owner":{"kind":"definition","value":{"kind":"actor","value":{"kind":"actor","namespace":{"game":"poe2","version":"owned-mechanics-v1"},"key":"synthetic.other"}}},"programs":{"members":[p],"closure":{"kind":"complete"}}}));
        }
        assert!(
            std::panic::catch_unwind(|| family::check_native(&schema, &changed, false)).is_err()
        );
    }
}

#[test]
fn component_query_lookup_preserves_exact_membership_with_later_queries() {
    let (schema, mut rules) = fixture();
    let c = family::consumer();
    let d: Value = family::read("dependencies.json");
    rules["owners"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|o| o["owner"] == d["shared_owner"]["owner"])
        .unwrap()["programs"]["members"]
        .as_array_mut()
        .unwrap()
        .push(json!(c.intrinsic));
    rules["owners"].as_array_mut().unwrap().push(json!(c.owner));
    rules["contribution_queries"]["members"] = json!(c.queries);
    let mut unrelated = c.queries[2].clone();
    unrelated.id = "unrelated-later-query".parse().unwrap();
    rules["contribution_queries"]["members"]
        .as_array_mut()
        .unwrap()
        .push(json!(unrelated));
    rules["receivers"]["members"] = json!([c.receiver]);
    family::check_native(&schema, &rules, true);
    let mut duplicate = rules.clone();
    duplicate["contribution_queries"]["members"]
        .as_array_mut()
        .unwrap()
        .push(json!(c.queries[0]));
    assert!(std::panic::catch_unwind(|| family::check_native(&schema, &duplicate, true)).is_err());
    rules["contribution_queries"]["members"][0]["groups"][0]["members"]["members"] = json!([]);
    assert!(std::panic::catch_unwind(|| family::check_native(&schema, &rules, true)).is_err());
}

#[test]
#[ignore = "explicit PLAYER_EVASION_PRIOR/OUTPUT; complete-build refusal remains"]
fn publish_player_evasion_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_EVASION_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_EVASION_OUTPUT").unwrap()),
        &family::data(),
        &[],
        &[
            "authoring.json",
            "dependencies.json",
            "bindings.json",
            "source-domain.json",
        ],
        family::stage,
        json!({"new_definitions":1,"new_owner_programs":2,"new_queries":3,"new_receivers":1,"complete_request":false}),
        [106, 117, 109, 123, 4],
    );
}
