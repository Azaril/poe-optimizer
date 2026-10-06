//! Shared local defence reducers authenticated independently of final item coverage.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::{OwnedDefinitionKey, StatDefinition},
    owned_rules::DefinitionRules,
    owned_schema::{DefinitionDescriptor, SchemaDefinitionId, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_mapping::OwnedIdRegistry,
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};
pub const KIND: &str = "source-bound-local-defence-composition";
const DOMAIN: &str = "owned-local-defence-composition-v1";
const CHANNELS: [&str; 8] = [
    "pre_override_armour",
    "pre_override_energy_shield",
    "armour_evasion",
    "armour_energy_shield",
    "evasion_energy_shield",
    "defences_increase",
    "alternate_quality",
    "crafted_quality",
];
const CASES: [&str; 6] = [
    "original",
    "independent-repeat",
    "warm-quality-thirty-then-original",
    "raw-quality-zero",
    "raw-quality-thirty",
    "synthetic-local-defences",
];
const STAGES: [&str; 3] = ["fresh", "rebuild-one", "rebuild-two"];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/local-defence-composition")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(v: &Value) -> &[Value] {
    if v.as_object().is_some_and(|o| o.is_empty()) {
        &[]
    } else {
        v.as_array().unwrap()
    }
}
fn digest() -> OwnedContentDigest {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    digest_owned(DOMAIN, &(a, b, d, v, m), 1024 * 1024).unwrap()
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    for (f, n) in [
        ("allocated_definitions", 8),
        ("new_programs", 2),
        ("new_receivers", 2),
        ("closed_existing_rule_owners", 0),
        ("registry_last_issued_before", 0x330b),
        ("registry_last_issued_after", 0x3313),
    ] {
        assert_eq!(a[f], n);
    }
    for f in [
        "before",
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(a[f], b[f]);
        assert_eq!(a[f], d["source"][if f == "before" { "input" } else { f }]);
    }
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(json!(m.release), b["release"]);
    assert_eq!(
        m.release.as_str(),
        "pob-3887ae68-local-defence-composition-v1"
    );
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v20"
    );
    assert!(m.tables.is_empty() && m.query_targets.is_empty() && m.evaluation.is_none());
    assert_eq!(
        (m.schema.len(), m.owners.len(), m.receivers.len()),
        (8, 2, 2)
    );
    assert_eq!(b["channels"].as_object().unwrap().len(), 8);
    assert_eq!(b["templates"].as_array().unwrap().len(), 2);
    for (i, (name, key)) in [("Iron Crown", 0x1f1c), ("Cryptic Leggings", 0x1e0e)]
        .into_iter()
        .enumerate()
    {
        assert_eq!(b["templates"][i]["source_base"], name);
        assert_eq!(
            b["templates"][i]["template"]["key"],
            format!("def.{key:016x}")
        );
    }
    for (i, unit) in [0x29ee, 0x29ed, 0x29ee, 0x295a, 0x295a, 2, 1, 2]
        .into_iter()
        .enumerate()
    {
        let SchemaExtensionEntry::Definition(row) = &m.schema[i] else {
            panic!("eight Stat definitions")
        };
        let row = json!(row);
        assert_eq!(row["kind"], "stat");
        assert_eq!(row["value"]["id"], b["channels"][CHANNELS[i]]);
        assert_eq!(
            row["value"]["id"]["key"],
            format!("def.{:016x}", 0x330c + i)
        );
        assert_eq!(row["value"]["schema"]["kind"], "known");
        let schema = &row["value"]["schema"]["value"];
        assert_eq!(schema["targets"], json!(["equipment_use"]));
        assert_eq!(schema["value"]["kind"], "quantity");
        assert_eq!(
            schema["value"]["value"]["unit"]["key"],
            format!("def.{unit:016x}")
        );
    }
    for (i, (name, raw, base_order, inc_order)) in [
        (
            "pre-override-local-armour",
            0x29fb,
            ["armour_evasion", "armour_energy_shield"],
            ["armour_evasion", "armour_energy_shield"],
        ),
        (
            "pre-override-local-energy-shield",
            0x29fd,
            ["evasion_energy_shield", "armour_energy_shield"],
            ["armour_energy_shield", "evasion_energy_shield"],
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let owner = json!(m.owners[i]);
        let stat = &b["channels"][CHANNELS[i]];
        assert_eq!(
            owner["owner"],
            json!({"kind":"definition","value":{"kind":"stat","value":stat}})
        );
        assert_eq!(owner["programs"]["closure"], json!({"kind":"complete"}));
        assert_eq!(owner["programs"]["members"].as_array().unwrap().len(), 1);
        let p = &owner["programs"]["members"][0];
        assert_eq!(p["id"], name);
        assert_eq!(p["context"], "equipment_use");
        assert_eq!(
            p["effects"],
            json!([{"id":"derive","when":"crafted-supported","effect":{"kind":"derive","entity":"current","stat":stat,"value":"rounded"}}])
        );
        let node = |id: &str| -> &Value {
            let matches: Vec<_> = rows(&p["nodes"]).iter().filter(|n| n["id"] == id).collect();
            assert_eq!(matches.len(), 1);
            &matches[0]["expression"]
        };
        assert_eq!(
            *node("single-and-raw"),
            json!({"kind":"add","left":"single-base","right":"raw"})
        );
        for (j, h) in base_order.into_iter().enumerate() {
            let suffix = if h == "armour_evasion" {
                "base"
            } else {
                "typed"
            };
            assert_eq!(
                *node(&format!("base-{j}")),
                json!({"kind":"add","left":if j==0{"single-and-raw"}else{"base-0"},"right":format!("{h}-{suffix}")})
            );
        }
        for (j, h) in inc_order.into_iter().enumerate() {
            assert_eq!(
                *node(&format!("increase-{j}")),
                json!({"kind":"add","left":if j==0{"single-increase"}else{"increase-0"},"right":format!("{h}-increase")})
            );
        }
        for (id, expected) in [
            (
                "all-included",
                json!({"kind":"add","left":"increase-1","right":"all-increase"}),
            ),
            (
                "increased",
                json!({"kind":"scale","value":"base-1","factor":"increase-multiplier"}),
            ),
            (
                "quality-scaled",
                json!({"kind":"scale","value":"increased","factor":"quality-multiplier"}),
            ),
            (
                "round-offset",
                json!({"kind":"add","left":"quality-scaled","right":"half-output"}),
            ),
            (
                "crafted-supported",
                json!({"kind":"compare","operation":"equal","left":"crafted-quality","right":"zero-quality"}),
            ),
            (
                "alternate",
                json!({"kind":"compare","operation":"greater","left":"alternate-quality","right":"zero-factor"}),
            ),
            (
                "effective-quality",
                json!({"kind":"select","condition":"alternate","when_true":"zero-quality","when_false":"quality"}),
            ),
        ] {
            assert_eq!(*node(id), expected);
        }
        assert_eq!(node("rounded")["mode"], "floor");
        assert_eq!(node("rounded")["value"], "round-offset");
        assert_eq!(node("half-output")["value"]["value"]["value"], 0.5);
        assert_eq!(node("rounded")["quantum"]["value"], 1.0);
        let raw_read = rows(&p["reads"]).iter().find(|r| r["id"] == "raw").unwrap();
        assert_eq!(
            raw_read["source"]["value"]["stat"]["key"],
            format!("def.{raw:016x}")
        );
        let quality = rows(&p["reads"])
            .iter()
            .find(|r| r["id"] == "quality")
            .unwrap();
        assert_eq!(
            quality["source"]["value"]["stat"],
            b["raw_fields"]["declared_standard_quality"]
        );
        assert_eq!(quality["source"]["kind"], "stat");
        assert_eq!(rows(&p["reads"]).len(), 11);
        for read in rows(&p["reads"]) {
            assert_eq!(read["source"]["value"]["entity"], "current");
            if read["source"]["kind"] == "contributions" {
                assert_eq!(read["source"]["value"]["reduction"], "sum");
                assert_eq!(read["source"]["value"]["empty"]["value"]["value"], 0.0);
                assert_eq!(
                    read["source"]["value"]["empty"]["value"]["unit"],
                    read["value_type"]["value"]["unit"]
                );
            }
        }
        assert_eq!(
            json!(m.receivers[i]),
            json!({"id":name,"stat":stat,"program":name,"targets":b["templates"].as_array().unwrap().iter().map(|t|json!({"kind":"equipment_template","value":{"template":t["template"]}})).collect::<Vec<_>>()})
        );
    }
    assert_eq!(rows(&d["definitions"]).len(), 11);
    assert_eq!(rows(&d["owners"]).len(), 2);
    assert_eq!(rows(&d["slots"]).len(), 12);
    for owner in rows(&d["owners"]) {
        assert_eq!(owner["programs"]["closure"]["kind"], "partial");
    }
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(v["scope"], b["scope"]);
    for f in [
        "whole_build_parity",
        "final_item_defences",
        "actor_defences",
        "incoming_contributor_inventory_closed",
        "template_owner_closed",
        "crafted_quality_preparation_implemented",
        "per_level_composition_implemented",
        "armour_data_overrides_implemented",
        "modifier_delivery_implemented",
    ] {
        assert_eq!(b["scope"][f], false);
    }
    let assets = a["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        assets.keys().map(String::as_str).collect::<Vec<_>>(),
        ["bindings", "dependencies", "migration", "source-vectors"]
    );
    for (name, digest) in assets {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(*digest, hash(&bytes));
    }
    check_vectors(&a, &b, &v, false);
}
fn dependencies(endpoint: &StagedOwnedRelease, d: &Value) {
    for row in
        serde_json::from_value::<Vec<DefinitionDescriptor>>(d["definitions"].clone()).unwrap()
    {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    for row in serde_json::from_value::<Vec<DefinitionRules>>(d["owners"].clone()).unwrap() {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    for row in serde_json::from_value::<Vec<SlotDescriptor>>(d["slots"].clone()).unwrap() {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .schema
                .slots
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
}
pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    let a: Value = read("authoring.json");
    let d: Value = read("dependencies.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let p = endpoint.receipt().provenance.last().unwrap();
    assert_eq!(p.kind.as_str(), KIND);
    assert_eq!(json!(p.prior_input), a["before"]);
    assert_eq!(p.authoring_input, digest());
    assert_eq!(endpoint.input().recipe.schema.release, m.release);
    dependencies(endpoint, &d);
    for entry in m.schema {
        let SchemaExtensionEntry::Definition(row) = entry else {
            unreachable!()
        };
        assert_eq!(
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    for row in m.owners {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    for mut row in m.receivers {
        // Release assembly canonicalizes target sets without changing their membership.
        row.targets.sort();
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .receivers
                .members
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    check_vectors(&a, &b, &v, true);
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for f in [
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(receipt[f], a[f]);
    }
    assert!(prior.evaluation().is_none());
    dependencies(prior, &d);
    let migrated = compile_owned_release_migration(prior, m.clone(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    assert_eq!(input.provenance.len(), prior.input().provenance.len() + 1);
    assert_eq!(
        input.provenance[..prior.input().provenance.len()],
        prior.input().provenance
    );
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_endpoint(&next);
    let mut inverse = next.input().clone();
    *inverse.provenance.last_mut().unwrap() = migrated.input().provenance.last().unwrap().clone();
    assert_eq!(inverse, *migrated.input());
    let mut registry =
        OwnedIdRegistry::new(prior.input().recipe.registry.clone(), Default::default()).unwrap();
    let added: Vec<_> = (0..8)
        .map(|_| {
            registry
                .allocate_definition::<StatDefinition>()
                .unwrap()
                .address()
        })
        .collect();
    assert_eq!(next.input().recipe.registry, *registry.input());
    let mut restored = next.input().recipe.clone();
    assert_eq!(
        restored.schema.definitions.len(),
        prior.input().recipe.schema.definitions.len() + 8
    );
    assert_eq!(
        restored.rules.owners.len(),
        prior.input().recipe.rules.owners.len() + 2
    );
    assert_eq!(
        restored.rules.receivers.members.len(),
        prior.input().recipe.rules.receivers.members.len() + 2
    );
    restored
        .schema
        .definitions
        .retain(|r| !added.contains(&r.address()));
    restored.rules.owners.retain(|r| !m.owners.contains(r));
    let mut added_receivers = m.receivers.clone();
    for receiver in &mut added_receivers {
        receiver.targets.sort();
    }
    restored
        .rules
        .receivers
        .members
        .retain(|r| !added_receivers.contains(r));
    restored.registry = prior.input().recipe.registry.clone();
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "only eight Stats, two reducer owners and two finite receivers"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
fn project_state(state: &Value) -> Value {
    let mut items = state["items"].clone();
    for item in items.as_array_mut().unwrap() {
        for field in ["base_records", "active_records"] {
            item.as_object_mut().unwrap().remove(field);
        }
    }
    let mut environments = state["environments"].clone();
    for env in environments.as_array_mut().unwrap() {
        env.as_object_mut().unwrap().remove("player_outputs");
    }
    let trace = &state["trace"];
    json!({"selected":state["selected"],"items":items,"environments":environments,"assemblies":trace["assemblies"],"rounds":trace["rounds"],"overrides":trace["overrides"],"consumers":trace["consumers"],"original_functions_preserved":state["original_functions_preserved"],"method_wrappers":state["method_wrappers"]})
}
fn without_trace(state: &Value) -> Value {
    let mut value = state.clone();
    value.as_object_mut().unwrap().remove("trace");
    value
}
fn source_input_groups(trace: &Value, item_id: i64, b: &Value) -> Value {
    let mut result = vec![];
    for (channel, kind, name, source_kind) in [
        ("pre_override_armour", "add", "Armour", "BASE"),
        ("pre_override_armour", "increase", "Armour", "INC"),
        ("pre_override_energy_shield", "add", "EnergyShield", "BASE"),
        (
            "pre_override_energy_shield",
            "increase",
            "EnergyShield",
            "INC",
        ),
        ("armour_evasion", "add", "ArmourAndEvasion", "BASE"),
        ("armour_evasion", "increase", "ArmourAndEvasion", "INC"),
        (
            "armour_energy_shield",
            "add",
            "ArmourAndEnergyShield",
            "BASE",
        ),
        (
            "armour_energy_shield",
            "increase",
            "ArmourAndEnergyShield",
            "INC",
        ),
        (
            "evasion_energy_shield",
            "add",
            "EvasionAndEnergyShield",
            "BASE",
        ),
        (
            "evasion_energy_shield",
            "increase",
            "EvasionAndEnergyShield",
            "INC",
        ),
        ("defences_increase", "increase", "Defences", "INC"),
        ("alternate_quality", "add", "AlternateQualityArmour", "BASE"),
        ("crafted_quality", "add", "Quality", "BASE"),
    ] {
        let calls: Vec<_> = rows(&trace["locals"])
            .iter()
            .filter(|r| {
                r["item_id"] == item_id
                    && r["name"] == name
                    && r["type"] == source_kind
                    && r["exact_current_item"] == true
            })
            .collect();
        assert!(!calls.is_empty());
        let value = calls[0]["result"].clone();
        for call in calls {
            assert_current_item(call);
            assert_eq!(call["original_call"], true);
            assert_eq!(call["original_return"], true);
            assert_eq!(call["flags"], 0);
            assert_eq!(call["result"], value);
        }
        result.push(json!({"stat":b["channels"][channel],"contribution":kind,"value":value}));
    }
    json!(result)
}
fn assert_current_item(row: &Value) {
    for flag in [
        "exact_registered_item",
        "exact_main_item",
        "exact_calcs_item",
        "exact_current_item",
    ] {
        assert_eq!(row[flag], true, "selected source object identity: {flag}");
    }
}
fn check_vectors(a: &Value, b: &Value, v: &Value, full: bool) {
    assert_eq!(v["status"], "passed");
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(hash(&bytes), a["source_manifest_sha256"]);
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    assert_eq!(rows(&a["source_files"]).len(), 11);
    let mut seen = BTreeSet::new();
    for pin in rows(&a["source_files"]) {
        assert!(seen.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            rows(&manifest["files"])
                .iter()
                .filter(|r| *r == pin)
                .count(),
            1
        );
        if full {
            let s = fs::read_to_string(
                root()
                    .join("vendor/path-of-building-poe2")
                    .join(pin["path"].as_str().unwrap()),
            )
            .unwrap()
            .replace("\r\n", "\n");
            assert_eq!(s.len() as u64, pin["bytes"].as_u64().unwrap());
            assert_eq!(hash(s.as_bytes()), pin["sha256"]);
        }
    }
    assert_eq!(rows(&v["observations"]).len(), 18);
    assert_eq!(rows(&v["native_cases"]).len(), 12);
    for (i, name) in CASES.into_iter().enumerate() {
        for (j, stage) in STAGES.into_iter().enumerate() {
            let o = &v["observations"][i * 3 + j];
            assert_eq!(o["case_index"], i);
            assert_eq!(o["name"], name);
            assert_eq!(o["stage"], stage);
            assert_eq!(o["pointer"], format!("/cases/{i}/observed/states/{stage}"));
            let s = &o["value"];
            assert_eq!(s["original_functions_preserved"], true);
            assert_eq!(s["method_wrappers"], false);
            for (k, n) in [("items", 2), ("spec", 3), ("skills", 4), ("config", 1)] {
                assert_eq!(s["selected"][k], n);
            }
            assert_eq!(rows(&s["items"]).len(), 2);
            for (k, id) in [21, 22].into_iter().enumerate() {
                let item = rows(&s["items"]).iter().find(|x| x["id"] == id).unwrap();
                assert_eq!(item["base_name"], b["templates"][k]["source_base"]);
                assert_eq!(item["exact_registered"], true);
                assert_eq!(item["exact_catalogue_base"], true);
                assert_eq!(
                    item["quality"],
                    match i {
                        3 => 0,
                        4 => 30,
                        _ => 20,
                    }
                );
                assert_eq!(item["crafted_quality"], 0);
                for field in ["EvasionPerLevel", "EnergyShieldPerLevel", "WardPerLevel"] {
                    assert_eq!(item["armour"][field], 0);
                }
            }
        }
    }
    let metadata = &v["report_metadata"];
    assert_eq!(metadata["source_revision"], a["source_revision"]);
    assert_eq!(metadata["manifest_sha256"], a["source_manifest_sha256"]);
    assert_eq!(metadata["native_owner_closure"], false);
    assert_eq!(
        metadata["preoverride_and_slot_consumption_are_separate"],
        true
    );
    for pin in rows(&metadata["files"]) {
        assert!(
            rows(&a["source_files"])
                .iter()
                .any(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
        );
    }
    assert_eq!(rows(&v["reports"]).len(), 2);
    assert_eq!(v["reports"][0]["sha256"], v["reports"][1]["sha256"]);
    assert_eq!(v["reports"][0]["bytes"], v["reports"][1]["bytes"]);
    if !full {
        return;
    }
    let source = fs::read_to_string(
        root().join("crates/poe-optimizer-pob/tests/support/armour_item_input_source.rs"),
    )
    .unwrap();
    let observer = source
        .split_once("pub const LOCAL_DEFENCE_OBSERVER: &str = r##\"")
        .unwrap()
        .1
        .split_once("\"##;")
        .unwrap()
        .0;
    assert_eq!(hash(observer.as_bytes()), metadata["observer_sha256"]);
    let mut first = None;
    for report_row in rows(&v["reports"]) {
        let path = report_row["path"].as_str().unwrap();
        assert!(
            path.starts_with("runs/owned-armour-local-defence-source-") && path.ends_with(".json")
        );
        let bytes = fs::read(root().join(path)).unwrap();
        assert_eq!(bytes.len() as u64, report_row["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), report_row["sha256"]);
        if let Some(previous) = &first {
            assert_eq!(&bytes, previous);
        } else {
            first = Some(bytes.clone());
        }
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        let mut m = report.clone();
        m.as_object_mut().unwrap().remove("cases");
        assert_eq!(m, *metadata);
        assert_eq!(rows(&report["cases"]).len(), 6);
        for o in rows(&v["observations"]) {
            assert_eq!(
                project_state(report.pointer(o["pointer"].as_str().unwrap()).unwrap()),
                o["value"]
            );
        }
        for (i, c) in rows(&report["cases"]).iter().enumerate() {
            assert_eq!(c["name"], CASES[i]);
            assert_eq!(c["synthetic_local_control"], i == 5);
            assert_eq!(c["game_obtainability_authority"], false);
            assert_eq!(c["native_source_admission_authority"], false);
            for branch in ["observed", "unhooked"] {
                assert_eq!(c[branch]["source_hash"], a["source_manifest_sha256"]);
                assert_eq!(c[branch]["independent_source_bindings_verified"], true);
                assert_eq!(
                    c[branch]["source_bindings"],
                    json!([{"item_id":21,"ordinal":576},{"item_id":22,"ordinal":578}])
                );
            }
            for stage in STAGES {
                let s = &c["observed"]["states"][stage];
                assert_eq!(
                    without_trace(s),
                    without_trace(&c["unhooked"]["states"][stage])
                );
                for mode in ["MAIN", "CALCS"] {
                    let consumers: Vec<_> = rows(&s["trace"]["consumers"])
                        .iter()
                        .filter(|r| r["mode"] == mode && r["exact_current_environment"] == true)
                        .collect();
                    assert_eq!(consumers.len(), 1);
                    let consumer = consumers[0];
                    assert_eq!(consumer["original_defence_return"], true);
                    assert_eq!(rows(&consumer["reads"]).len(), 4);
                    for id in [21, 22] {
                        let item = rows(&s["items"]).iter().find(|r| r["id"] == id).unwrap();
                        let output = rows(&consumer["outputs"])
                            .iter()
                            .find(|r| r["item_id"] == id)
                            .unwrap();
                        for field in ["Armour", "EnergyShield"] {
                            let calls: Vec<_> = rows(&consumer["reads"])
                                .iter()
                                .filter(|r| r["item_id"] == id && r["name"] == field)
                                .collect();
                            assert_eq!(calls.len(), 1);
                            assert_current_item(calls[0]);
                            for flag in [
                                "exact_selected_item",
                                "original_call",
                                "original_return",
                                "exact_return_to_consumer",
                            ] {
                                assert_eq!(calls[0][flag], true);
                            }
                            assert_eq!(calls[0]["returned_value"], item["armour"][field]);
                            assert_eq!(output[field], calls[0]["returned_value"]);
                        }
                    }
                }
            }
            assert_eq!(
                without_trace(&c["observed"]["states"]["rebuild-one"]),
                without_trace(&c["observed"]["states"]["rebuild-two"])
            );
            assert_eq!(
                without_trace(&c["observed"]["constructors"]),
                without_trace(&c["unhooked"]["constructors"])
            );
            assert_eq!(
                c["observed"]["constructors"]["selected_items_preserved"],
                true
            );
            for (j, id) in [21, 22].into_iter().enumerate() {
                let n = &v["native_cases"][i * 2 + j];
                assert_eq!(n["case_index"], i);
                assert_eq!(n["name"], c["name"]);
                assert_eq!(n["source_item_id"], id);
                assert_eq!(n["template"], b["templates"][j]["template"]);
                assert_eq!(n["raw_quality"], c["raw_quality"]);
                let s = &c["observed"]["states"]["fresh"];
                let item = rows(&s["items"]).iter().find(|r| r["id"] == id).unwrap();
                assert_eq!(n["raw_profile"], item["base"]["armour"]);
                assert_eq!(n["groups"], source_input_groups(&s["trace"], id, b));
                let assemblies: Vec<_> = rows(&s["trace"]["assemblies"])
                    .iter()
                    .filter(|r| r["item_id"] == id && r["exact_current_item"] == true)
                    .collect();
                assert!(!assemblies.is_empty());
                for row in assemblies {
                    assert_current_item(row);
                    assert_eq!(row["original_assignment_observed"], true);
                    assert_eq!(row["operands"]["craftedQuality"], 0);
                    assert_eq!(row["quality"], n["raw_quality"]);
                    assert_eq!(n["armour"], row["preoverride"]["Armour"]);
                    assert_eq!(n["energy_shield"], row["preoverride"]["EnergyShield"]);
                    assert_eq!(row["preoverride"]["Armour"], item["armour"]["Armour"]);
                    assert_eq!(
                        row["preoverride"]["EnergyShield"],
                        item["armour"]["EnergyShield"]
                    );
                }
                for line in [2547, 2551] {
                    let rounds: Vec<_> = rows(&s["trace"]["rounds"])
                        .iter()
                        .filter(|r| {
                            r["item_id"] == id
                                && r["call_line"] == line
                                && r["exact_current_item"] == true
                        })
                        .collect();
                    assert!(!rounds.is_empty());
                    for round in rounds {
                        assert_current_item(round);
                        assert_eq!(round["original_call"], true);
                    }
                }
                let overrides: Vec<_> = rows(&s["trace"]["overrides"])
                    .iter()
                    .filter(|r| r["item_id"] == id && r["exact_current_item"] == true)
                    .collect();
                assert!(!overrides.is_empty());
                for row in overrides {
                    assert_current_item(row);
                    assert_eq!(row["original_return"], true);
                    assert_eq!(row["exact_local_store"], true);
                    assert!(rows(&row["result"]).is_empty());
                }
            }
        }
        for i in [1, 2] {
            for stage in STAGES {
                assert_eq!(
                    without_trace(&report["cases"][0]["observed"]["states"][stage]),
                    without_trace(&report["cases"][i]["observed"]["states"][stage])
                );
            }
        }
    }
}
