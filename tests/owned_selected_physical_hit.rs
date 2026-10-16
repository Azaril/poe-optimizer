//! Published native damage endpoint, with a finite joined component replay.
#[allow(dead_code)]
#[path = "support/owned_minion_attack_selection.rs"]
mod attack_selection_family;
#[allow(dead_code)]
#[path = "support/owned_combined_added_attack_damage.rs"]
mod combined_added_damage_family;
#[allow(dead_code)]
#[path = "support/owned_selected_physical_hit.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_selected_physical_hit_replay.rs"]
mod numerical;
#[allow(dead_code)]
#[path = "support/owned_selected_physical_base.rs"]
mod physical_base_family;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_plan_replay.rs"]
mod replay;
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

#[test]
fn source_domain_retains_full_bodies_and_actual_returns_without_native_source_execution() {
    family::check_authored();
}

#[test]
fn source_census_rejects_zero_inactive_flags_conversion_and_endpoint_suppliers() {
    let domain: Value = family::read("source-domain.json");
    let original = domain["selected"][0]["inputs"].clone();
    family::check_selected_physical_inputs(&original).unwrap();
    for name in [
        "Condition:Intimidated",
        "Condition:Party:Intimidated",
        "Condition:Debilitated",
        "PhysicalDamage",
        "MinPhysicalDamage",
        "MaxPhysicalDamage",
        "SkillPhysicalDamageConvertToFire",
        "PhysicalDamageConvertToRandom",
        "DamageGainAsPhysical",
        "DamageAsPhysical",
        "FireDamageAsPhysical",
        "DealNoDamage",
        "DealNoPhysical",
    ] {
        let mut value = original.clone();
        value["raw_skill_modifiers"].as_array_mut().unwrap().push(json!({"ancestor_depth":0,"mod":{"name":name,"type":"BASE","value":0,"tags":[{"type":"Condition","var":"SyntheticNever"}]}}));
        assert!(
            family::check_selected_physical_inputs(&value).is_err(),
            "{name}"
        );
    }
    let mut suppressed = original.clone();
    suppressed["bases"]["Physical"]["can_deal"] = json!(false);
    assert!(family::check_selected_physical_inputs(&suppressed).is_err());
    for matrix in ["conversion_table", "gain_table"] {
        let mut missing = original.clone();
        missing[matrix]["Physical"]
            .as_object_mut()
            .unwrap()
            .remove("Physical");
        assert!(family::check_selected_physical_inputs(&missing).is_err());
    }
    for field in [
        "conditions",
        "overrideCond",
        "skillCond",
        "skill_conditions",
    ] {
        let mut value = original.clone();
        value[field] = json!({"Intimidated":false});
        assert!(family::check_selected_physical_inputs(&value).is_err());
    }
}

fn native_fixture() -> (Value, Value, Value) {
    let base: Value = physical_base_family::read("dependencies.json");
    let hit: Value = family::read("dependencies.json");
    let mut definitions = base["selected_native"]["definitions"]
        .as_array()
        .unwrap()
        .clone();
    for d in base["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .chain(hit["definitions"].as_array().unwrap())
    {
        if !definitions.contains(d) {
            definitions.push(d.clone());
        }
    }
    let mut owners = base["selected_native"]["owners"]
        .as_array()
        .unwrap()
        .clone();
    let owner = owners
        .iter_mut()
        .find(|o| o["owner"] == hit["owner_before"]["owner"])
        .unwrap();
    *owner = hit["owner_before"].clone();
    // The all-package writer census also contains unselected Action owners,
    // such as Gas Arrow sharing the INC result channel. Retain those writers;
    // the selected occurrence subset alone is intentionally insufficient.
    for writer in hit["input_writers"].as_array().unwrap() {
        if writer["kind"] != "owner" {
            continue;
        }
        if let Some(row) = owners.iter_mut().find(|o| o["owner"] == writer["owner"]) {
            if !row["programs"]["members"]
                .as_array()
                .unwrap()
                .contains(&writer["program"])
            {
                row["programs"]["members"]
                    .as_array_mut()
                    .unwrap()
                    .push(writer["program"].clone());
            }
        } else {
            owners.push(json!({"owner":writer["owner"],"programs":{"members":[writer["program"]],"closure":{"kind":"complete"}}}));
        }
    }
    (
        json!({"definitions":definitions,"slots":base["selected_native"]["slots"]}),
        json!({"owners":owners,"effect_applications":hit["effect_applications"],"existing_actor_rules":hit["existing_actor_rules"],"contribution_queries":{"members":[hit["more_query"]],"closure":hit["remaining_query_registry_closure"]},"receivers":{"closure":hit["remaining_receiver_registry_closure"]}}),
        json!([base["routes"]]),
    )
}

#[test]
fn native_census_includes_every_owner_and_application_even_when_zero_or_inactive() {
    let (s, original, routes) = native_fixture();
    family::check_native(&s, &original, &routes, false);
    for (stat, application) in [0x339e_u64, 0x339f, 0x33a0, 0x339c, 0x335c, 0x1d]
        .into_iter()
        .flat_map(|n| [(n, false), (n, true)])
    {
        let mut r = original.clone();
        let p = json!({"id":"unlisted-inactive-source","context":"actor","reads":[],"nodes":[{"id":"false","expression":{"kind":"literal","value":{"kind":"boolean","value":false}}}],"effects":[{"id":"flag","when":"false","effect":{"kind":"contribute","entity":"current","stat":{"key":format!("def.{stat:016x}")},"contribution":"flag","value":"false"}}]});
        if application {
            r["effect_applications"]["members"][0]["program"] = p;
        } else {
            r["owners"][0]["programs"]["members"]
                .as_array_mut()
                .unwrap()
                .push(p);
        }
        assert!(
            std::panic::catch_unwind(|| family::check_native(&s, &r, &routes, false)).is_err(),
            "{stat:x}/{application}"
        );
    }
}

#[test]
#[ignore = "explicit SELECTED_PHYSICAL_HIT_PRIOR/OUTPUT; preserves complete-build gaps"]
fn publish_selected_physical_hit_preserving_all_five_originals() {
    let out =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_SELECTED_PHYSICAL_HIT_OUTPUT").unwrap());
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_SELECTED_PHYSICAL_HIT_PRIOR").unwrap()),
        out.clone(),
        &family::data(),
        &[],
        &[
            "authoring.json",
            "dependencies.json",
            "bindings.json",
            "source-evidence.json",
            "source-domain.json",
        ],
        family::stage,
        json!({"new_definitions":4,"new_programs":1,"new_queries":2,"new_receivers":0,"complete_damage":false}),
        [106, 117, 109, 123, 4],
    );
    let xml = fs::read(family::root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
        .unwrap();
    for name in ["prior-original-05", "original-05"] {
        let dir = out.join(name);
        let selection = selected::selection(&xml, &dir);
        let draft: Value =
            serde_json::from_slice(&fs::read(dir.join("draft.json")).unwrap()).unwrap();
        physical_base_family::check_selected_import(&draft, &selection);
    }
}
