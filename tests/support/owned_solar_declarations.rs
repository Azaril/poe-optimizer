//! Exact Solar intrinsic input-port closure. Numerical and quality coverage stay open.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_rules::DefinitionRules,
    owned_schema::{DefinitionDescriptor, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};
pub const KIND: &str = "source-bound-solar-declarations";
const DOMAIN: &str = "owned-solar-declarations-v1";
const STAGES: [&str; 3] = ["fresh", "rebuild-one", "rebuild-two"];
const CASES: [&str; 6] = [
    "original",
    "independent-repeat",
    "warm-quality-twenty-then-original",
    "source-quality-zero",
    "source-quality-twenty",
    "raw-level-seventy-seven",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/solar-declarations")
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
fn digest() -> poe_optimizer_core::owned_content::OwnedContentDigest {
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
    for (field, value) in [
        ("allocated_definitions", 0),
        ("replaced_definitions", 1),
        ("closed_declaration_inventories", 7),
        ("closed_existing_rule_owners", 0),
        ("registry_last_issued_before", 0x330b),
        ("registry_last_issued_after", 0x330b),
    ] {
        assert_eq!(a[field], value);
    }
    for field in [
        "before",
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(a[field], b[field]);
        assert_eq!(
            a[field],
            d["source"][if field == "before" { "input" } else { field }]
        );
    }
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(json!(m.release), b["release"]);
    assert_eq!(m.release.as_str(), "pob-3887ae68-solar-declarations-v1");
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v20"
    );
    assert!(
        m.tables.is_empty()
            && m.owners.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(m.schema.len(), 1);
    assert_eq!(b["template"]["key"], "def.0000000000002343");
    assert_eq!(b["base_name"], "Solar Amulet");
    assert_eq!(d["definitions"].as_array().unwrap().len(), 1);
    assert_eq!(d["slots"].as_array().unwrap().len(), 6);
    assert_eq!(d["owners"].as_array().unwrap().len(), 1);
    let old = &d["definitions"][0];
    assert_eq!(old["value"]["id"], b["template"]);
    let SchemaExtensionEntry::Definition(new) = &m.schema[0] else {
        panic!("one exact descriptor replacement")
    };
    let new = json!(new);
    let mut restored = new.clone();
    let ports = restored["value"]["schema"]["value"]["declarations"]
        .as_object_mut()
        .unwrap();
    assert_eq!(ports.len(), 7);
    for (name, after) in ports {
        let before = &old["value"]["schema"]["value"]["declarations"][name];
        assert_eq!(before["closure"]["kind"], "partial");
        assert_eq!(after["closure"], json!({"kind":"complete"}));
        assert_eq!(after["members"], before["members"]);
        assert_eq!(
            after["members"].as_array().unwrap().len(),
            if name == "parameters" { 6 } else { 0 }
        );
        after["closure"] = before["closure"].clone();
    }
    assert_eq!(
        restored, *old,
        "only seven reviewed intrinsic declaration closures"
    );
    assert_eq!(b["parameters"].as_array().unwrap().len(), 6);
    for (i, name) in [
        "rarity",
        "corrupted",
        "raw-level-requirement",
        "socket-capacity",
        "catalyst-kind",
        "catalyst-amount",
    ]
    .into_iter()
    .enumerate()
    {
        let slot = &d["slots"][i];
        let id = &slot["value"]["id"];
        assert_eq!(b["parameters"][i]["name"], name);
        assert_eq!(b["parameters"][i]["slot"], *id);
        assert_eq!(id["slot"]["key"], format!("def.{:016x}", 0x3167 + i));
        assert_eq!(id["declaration"]["definition"], b["template"]);
        assert_eq!(slot["value"]["schema"]["kind"], "known");
        assert_eq!(
            slot["value"]["schema"]["value"]["presence"],
            if i == 2 {
                "optional_once"
            } else {
                "required_once"
            }
        );
        assert_eq!(
            slot["value"]["schema"]["value"]["sites"],
            json!(["item_parameter"])
        );
        assert_eq!(
            new["value"]["schema"]["value"]["declarations"]["parameters"]["members"][i],
            *id
        );
    }
    let schema = &new["value"]["schema"]["value"];
    assert_eq!(schema["quality"]["presence"], "optional");
    assert_eq!(
        schema["quality"]["allowed_kinds"]["closure"]["kind"],
        "partial"
    );
    assert!(
        schema["quality"]["allowed_kinds"]["members"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(schema["modifiers"]["closure"]["kind"], "partial");
    assert_eq!(schema["modifiers"]["members"].as_array().unwrap().len(), 30);
    let owner = &d["owners"][0];
    assert_eq!(
        owner["owner"],
        json!({"kind":"definition","value":{"kind":"item_template","value":b["template"]}})
    );
    assert_eq!(owner["programs"]["closure"]["kind"], "partial");
    assert_eq!(
        owner["programs"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "template-supplies-base-attack-profile",
            "catalyst-inputs",
            "amulet-copy-eligibility",
            "ordinary-item-direct-applicability"
        ]
    );
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(v["scope"], b["scope"]);
    for field in [
        "whole_build_parity",
        "template_rules_closed",
        "quality_inventory_closed",
        "modifier_inventory_closed",
        "incoming_contributor_inventory_closed",
        "item_requirements_implemented",
        "derived_quality_implemented",
    ] {
        assert_eq!(b["scope"][field], false);
    }
    for field in [
        "new_definitions",
        "new_programs",
        "query_changes",
        "routing_changes",
    ] {
        assert_eq!(b["scope"][field], 0);
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
    check_vectors(&a, &v, false);
}
fn unchanged_dependencies(endpoint: &StagedOwnedRelease, d: &Value) {
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
}
pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    let a: Value = read("authoring.json");
    let d: Value = read("dependencies.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let proof = endpoint.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), a["before"]);
    assert_eq!(proof.authoring_input, digest());
    assert_eq!(endpoint.input().recipe.schema.release, m.release);
    let SchemaExtensionEntry::Definition(expected) = &m.schema[0] else {
        unreachable!()
    };
    assert_eq!(
        endpoint
            .input()
            .recipe
            .schema
            .definitions
            .iter()
            .filter(|x| *x == expected)
            .count(),
        1
    );
    unchanged_dependencies(endpoint, &d);
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    check_vectors(&a, &v, true);
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(receipt[field], a[field]);
    }
    let old: DefinitionDescriptor = serde_json::from_value(d["definitions"][0].clone()).unwrap();
    assert_eq!(
        prior
            .input()
            .recipe
            .schema
            .definitions
            .iter()
            .filter(|x| **x == old)
            .count(),
        1
    );
    unchanged_dependencies(prior, &d);
    assert!(prior.evaluation().is_none());
    let migrated = compile_owned_release_migration(prior, m, Default::default()).unwrap();
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
    let mut restored = next.input().recipe.clone();
    let target = restored
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == old.address())
        .unwrap();
    *target = old;
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "one exact schema replacement; every rule/slot/registry/routing body unchanged"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
fn project_state(state: &Value) -> Value {
    let mut item = state["item"].clone();
    for key in ["base_modifiers", "active_modifiers"] {
        item.as_object_mut().unwrap().remove(key);
    }
    let mut environments = state["environments"].clone();
    for env in environments.as_array_mut().unwrap() {
        env.as_object_mut().unwrap().remove("outputs");
    }
    json!({"selected":state["selected"],"base_catalogue_identity":state["base_catalogue_identity"],"item":item,"members":state["members"],"environments":environments,"original_functions_preserved":state["original_functions_preserved"],"method_wrappers":state["method_wrappers"],"observer_installed":state["observer_installed"]})
}
fn check_vectors(a: &Value, v: &Value, full: bool) {
    assert_eq!(v["status"], "passed");
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(hash(&manifest_bytes), a["source_manifest_sha256"]);
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    let mut pins = BTreeSet::new();
    for pin in a["source_files"].as_array().unwrap() {
        assert!(pins.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|x| *x == pin)
                .count(),
            1
        );
        if full {
            let text = fs::read_to_string(
                root()
                    .join("vendor/path-of-building-poe2")
                    .join(pin["path"].as_str().unwrap()),
            )
            .unwrap()
            .replace("\r\n", "\n");
            assert_eq!(text.len() as u64, pin["bytes"].as_u64().unwrap());
            assert_eq!(hash(text.as_bytes()), pin["sha256"]);
        }
    }
    assert_eq!(v["base"]["type"], "Amulet");
    assert_eq!(v["base"]["req"], json!({"level":30}));
    for field in [
        "quality",
        "weapon",
        "armour",
        "flask",
        "charm",
        "socketLimit",
    ] {
        assert!(v["base"].get(field).is_none());
    }
    assert_eq!(v["observations"].as_array().unwrap().len(), 18);
    let absent = json!({"kind":"absent"});
    for (i, name) in CASES.into_iter().enumerate() {
        for (j, stage) in STAGES.into_iter().enumerate() {
            let obs = &v["observations"][i * 3 + j];
            assert_eq!(obs["case_index"], i);
            assert_eq!(obs["name"], name);
            assert_eq!(obs["stage"], stage);
            assert_eq!(
                obs["pointer"],
                format!("/cases/{i}/observed/states/{stage}")
            );
            let s = &obs["value"];
            for flag in ["base_catalogue_identity", "original_functions_preserved"] {
                assert_eq!(s[flag], true);
            }
            for flag in ["method_wrappers", "observer_installed"] {
                assert_eq!(s[flag], false);
            }
            let item = &s["item"];
            assert_eq!(item["exact_registered"], true);
            assert_eq!(item["exact_catalogue_base"], true);
            assert_eq!(item["id"], 23);
            assert_eq!(item["base_name"], "Solar Amulet");
            assert_eq!(item["rarity"], "RARE");
            assert_eq!(item["socket_count"], 0);
            assert!(rows(&item["granted_skills"]).is_empty());
            for f in ["corrupted", "catalyst", "catalyst_quality"] {
                assert_eq!(item[f], absent);
            }
            assert_eq!(
                item["quality"],
                match i {
                    3 => json!(0),
                    4 => json!(20),
                    _ => absent.clone(),
                }
            );
            assert_eq!(item["crafted_quality"], 0);
            assert_eq!(item["requirements"]["level"], if i == 5 { 77 } else { 30 });
            for category in ["buff", "enchant", "rune", "classRequirement"] {
                assert!(rows(&s["members"][category]).is_empty());
            }
            for category in ["implicit", "explicit"] {
                assert_eq!(rows(&s["members"][category]).len(), 1);
            }
            for (field, n) in [("items", 2), ("spec", 3), ("skills", 4), ("config", 1)] {
                assert_eq!(s["selected"][field], n);
            }
            assert_eq!(rows(&s["environments"]).len(), 2);
            for env in rows(&s["environments"]) {
                assert_eq!(env["selected_item_exact"], true);
                assert_eq!(rows(&env["requirements_rows"]).len(), 1);
                for attr in ["Str", "Dex", "Int"] {
                    assert_eq!(env["requirements_rows"][0][attr], 0);
                    assert_eq!(env["amulet_requirement_outputs"][attr], absent);
                }
            }
        }
    }
    let metadata = &v["report_metadata"];
    assert_eq!(metadata["source_revision"], a["source_revision"]);
    assert_eq!(metadata["manifest_sha256"], a["source_manifest_sha256"]);
    assert_eq!(metadata["native_input_closure"], false);
    assert_eq!(metadata["native_owner_coverage"], false);
    for p in metadata["files"].as_array().unwrap() {
        assert!(
            a["source_files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|x| x["path"] == p["path"] && x["sha256"] == p["sha256"])
        );
    }
    assert_eq!(v["reports"].as_array().unwrap().len(), 2);
    assert_eq!(v["reports"][0]["sha256"], v["reports"][1]["sha256"]);
    assert_eq!(v["reports"][0]["bytes"], v["reports"][1]["bytes"]);
    if !full {
        return;
    }
    let source = fs::read_to_string(
        root().join("crates/poe-optimizer-pob/tests/support/solar_item_inputs_source.rs"),
    )
    .unwrap();
    let observer = source
        .split_once("pub const OWNERSHIP_OBSERVER: &str = r##\"")
        .unwrap()
        .1
        .split_once("\"##;")
        .unwrap()
        .0;
    assert_eq!(hash(observer.as_bytes()), metadata["observer_sha256"]);
    let mut first = None;
    for (i, p) in v["reports"].as_array().unwrap().iter().enumerate() {
        assert_eq!(
            p["path"],
            format!(
                "runs/owned-solar-ownership-source-02/source-jit-{}.json",
                if i == 0 { "off" } else { "on" }
            )
        );
        let bytes = fs::read(root().join(p["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, p["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), p["sha256"]);
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
        for obs in v["observations"].as_array().unwrap() {
            let s = report.pointer(obs["pointer"].as_str().unwrap()).unwrap();
            assert_eq!(s["base"], v["base"]);
            assert_eq!(project_state(s), obs["value"]);
        }
        for (i, c) in rows(&report["cases"]).iter().enumerate() {
            assert_eq!(c["name"], CASES[i]);
            assert_eq!(c["game_obtainability_authority"], false);
            assert_eq!(c["source_only_quality_control"], i == 3 || i == 4);
            for branch in ["observed", "unhooked"] {
                assert_eq!(c[branch]["source_item_id"], 23);
                assert_eq!(c[branch]["source_item_ordinal"], 580);
                assert_eq!(c[branch]["independent_source_binding_verified"], true);
                assert_eq!(c[branch]["source_hash"], a["source_manifest_sha256"]);
            }
            for stage in STAGES {
                let s = &c["observed"]["states"][stage];
                let mut original = s.clone();
                original.as_object_mut().unwrap().remove("trace");
                let mut unhooked = c["unhooked"]["states"][stage].clone();
                unhooked.as_object_mut().unwrap().remove("trace");
                assert_eq!(original, unhooked);
                check_trace(s, stage == "fresh");
            }
            assert_eq!(
                project_state(&c["observed"]["states"]["rebuild-one"]),
                project_state(&c["observed"]["states"]["rebuild-two"])
            );
        }
        for i in [1, 2] {
            for stage in STAGES {
                assert_eq!(
                    project_state(&report["cases"][0]["observed"]["states"][stage]),
                    project_state(&report["cases"][i]["observed"]["states"][stage])
                );
            }
        }
    }
}
fn check_trace(s: &Value, fresh: bool) {
    let trace = &s["trace"];
    for mode in ["MAIN", "CALCS"] {
        let envs: Vec<_> = rows(&trace["environments"])
            .iter()
            .filter(|e| e["mode"] == mode && e["exact_current_environment"] == true)
            .collect();
        assert_eq!(envs.len(), 1);
        for env in envs {
            assert_eq!(env["performed"], true);
            assert_eq!(rows(&env["deliveries"]).len(), 1);
            assert_eq!(env["deliveries"][0]["exact_merged_row"], true);
            assert!(rows(&env["positive_branches"]).is_empty());
            for attr in ["Str", "Dex", "Int"] {
                let calls: Vec<_> = rows(&env["consumers"])
                    .iter()
                    .filter(|r| r["attribute"] == attr)
                    .collect();
                assert_eq!(calls.len(), 1);
                assert_eq!(calls[0]["input"], 0);
                assert_eq!(calls[0]["exact_delivered_row"], true);
                assert_eq!(calls[0]["row"]["exact_source_item"], true);
            }
            assert_eq!(rows(&env["item_returns"]).len(), 1);
            assert_eq!(
                env["item_returns"][0]["records"],
                s["item"]["active_modifiers"]
            );
        }
    }
    let calls = rows(&trace["local_calls"]);
    for c in calls {
        assert_eq!(c["original_call"], true);
        assert_eq!(c["return_observed"], true);
        assert_eq!(c["before"], c["after"]);
        assert_eq!(
            c["result"],
            if c["type"] == "FLAG" {
                json!(false)
            } else {
                json!(0)
            }
        );
    }
    if fresh {
        assert!(!rows(&trace["item_builds"]).is_empty());
        assert!(
            calls
                .iter()
                .any(|c| c["name"] == "Quality" && c["caller_line"] == 2443)
        );
        for name in ["StrRequirement", "DexRequirement", "IntRequirement"] {
            for kind in ["BASE", "INC"] {
                assert!(calls.iter().any(|c| c["name"] == name && c["type"] == kind));
            }
        }
    }
}
