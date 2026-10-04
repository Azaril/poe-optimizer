//! Data-only intrinsic assembly with an explicitly missing final-level producer.
#[path = "owned_release_migration_preservation.rs"]
mod migration_preservation;
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_routing::ActionRoutingInput,
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

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/ice-nova-intrinsics")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn bindings() -> Value {
    read("bindings.json")
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn quantity(value: f64, unit: &Value) -> Value {
    json!({"kind":"quantity","value":{"value":value,"unit":unit}})
}
fn names() -> [(&'static str, &'static str); 3] {
    [
        ("cold_min", "spell_minimum_base_cold_damage"),
        ("cold_max", "spell_maximum_base_cold_damage"),
        ("radius", "active_skill_base_area_of_effect_radius"),
    ]
}
fn radius_constants(set: &Value) -> Vec<f64> {
    let name = names()[2].1;
    assert!(
        set["stats"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["stat"] != name)
    );
    set["constants"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["stat"] == name)
        .map(|r| {
            assert_eq!(r["value_present"], true);
            r["value"].as_f64().unwrap()
        })
        .collect()
}
fn expected_program(b: &Value, v: &Value) -> Value {
    let mut nodes =
        vec![json!({"id":"final-level","expression":{"kind":"read","input":"final-level"}})];
    let mut effects = vec![];
    for (index, row) in b["stat_sets"].as_array().unwrap().iter().enumerate() {
        let tag = if index == 0 {
            "ordinary"
        } else {
            "cold-infused"
        };
        for channel in ["cold_min", "cold_max"] {
            let id = format!("{tag}-{}", channel.replace('_', "-"));
            nodes.push(json!({"id":id,"expression":{"kind":"lookup_integer_table","table":row["tables"][channel],"key":"final-level"}}));
            effects.push(json!({"id":id,"when":null,"effect":{"kind":"derive","entity":"current","stat":row["internal"][channel],"value":id}}));
        }
        let mut previous = None;
        let constants = radius_constants(&v["constructed"]["stat_sets"][index]);
        assert_eq!(constants.len(), index + 1);
        for (term, value) in constants.into_iter().enumerate() {
            let mut id = format!("{tag}-radius-term-{}", term + 1);
            nodes.push(json!({"id":id,"expression":{"kind":"literal","value":quantity(value,&b["units"]["distance"])}}));
            if let Some(left) = previous {
                let sum = format!("{tag}-radius-sum-{}", term + 1);
                nodes.push(json!({"id":sum,"expression":{"kind":"add","left":left,"right":id}}));
                id = sum;
            }
            previous = Some(id);
        }
        effects.push(json!({"id":format!("{tag}-radius"),"when":null,"effect":{"kind":"derive","entity":"current","stat":row["internal"]["radius"],"value":previous.unwrap()}}));
    }
    json!({"id":b["program"],"context":"action","reads":[{"id":"final-level","value_type":{"kind":"integer"},"source":{"kind":"parameter","value":{"slot":b["final_level"]}}}],"nodes":nodes,"effects":effects})
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b = bindings();
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let route: ActionRoutingInput = read("routing.json");
    assert_eq!(m.schema_version, 3);
    assert_eq!(m.contract.schema_version, 5);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v17"
    );
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(m.schema.len(), 11);
    assert_eq!(m.tables.len(), 4);
    assert_eq!(m.owners.len(), 1);
    assert!(m.receivers.is_empty() && m.query_targets.is_empty() && m.evaluation.is_none());
    assert_eq!(a["registry_last_issued_before"], 0x32d5);
    assert_eq!(a["registry_last_issued_after"], 0x32df);
    assert_eq!(a["allocated_definitions"], 10);
    for field in ["before", "definitions", "registry", "roles"] {
        assert_eq!(b[field], a[field]);
    }
    assert_eq!(d["source"]["input"], a["before"]);
    assert_eq!(d["source"]["definitions"], a["definitions"]);
    let artifacts = a["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        artifacts.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "bindings",
            "dependencies",
            "migration",
            "routing",
            "source-vectors"
        ]
    );
    for (name, hash) in artifacts {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r'));
        assert_eq!(*hash, json!(format!("{:x}", Sha256::digest(bytes))));
    }
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        a["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&manifest_bytes))
    );
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    let pins = a["source_files"].as_array().unwrap();
    assert_eq!(pins.len(), 14);
    let mut paths = BTreeSet::new();
    for pin in pins {
        assert!(paths.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    let schema = json!(m.schema);
    let mut expected = d["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["value"]["id"] == b["primary_skill"])
        .unwrap()
        .clone();
    let declarations = &mut expected["value"]["schema"]["value"]["declarations"];
    assert!(
        declarations["parameters"]["members"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(declarations["parameters"]["closure"]["kind"], "partial");
    declarations["parameters"]["members"] = json!([b["final_level"]]);
    assert_eq!(schema[0], json!({"kind":"definition","value":expected}));
    let slot = &schema[1]["value"]["value"];
    assert_eq!(slot["id"], b["final_level"]);
    assert_eq!(
        slot["schema"]["value"],
        json!({"value":{"kind":"integer","value":{"minimum":1,"maximum":40}},"presence":"required_once","sites":[],"skill_input":"projected"})
    );
    assert!(m.owners[0].programs.members.len() == 1 && !m.owners[0].programs.is_complete());
    assert_eq!(
        json!(m.owners[0].owner),
        json!({"kind":"definition","value":{"kind":"skill","value":b["primary_skill"]}})
    );
    assert_eq!(
        json!(m.owners[0].programs.members[0]),
        expected_program(&b, &v)
    );
    let sets = b["stat_sets"].as_array().unwrap();
    assert_eq!(sets.len(), 2);
    assert_eq!(v["admitted"].as_array().unwrap().len(), 80);
    let mut expected_tables = BTreeSet::new();
    for (index, row) in sets.iter().enumerate() {
        assert_eq!(row["source_index"], index + 1);
        let set = &v["constructed"]["stat_sets"][index];
        assert_eq!(set["index"], index + 1);
        assert_eq!(set["levels"].as_array().unwrap().len(), 40);
        for (channel, source_name) in names() {
            let unit = &b["units"][if channel == "radius" {
                "distance"
            } else {
                "damage"
            }];
            for stat in [&row["internal"][channel], &b["channels"][channel]] {
                let definition = schema
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|d| d["kind"] == "definition" && d["value"]["value"]["id"] == *stat)
                    .unwrap();
                assert_eq!(
                    definition["value"]["value"]["schema"]["value"],
                    json!({"value":{"kind":"quantity","value":{"unit":unit}},"targets":["action"]})
                );
            }
            if channel == "radius" {
                let sum: f64 = radius_constants(set).iter().sum();
                for vector in v["admitted"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|x| x["stat_set"] == index + 1)
                {
                    assert_eq!(vector[channel].as_f64(), Some(sum));
                }
                continue;
            }
            let id = row["tables"][channel].as_str().unwrap();
            assert!(expected_tables.insert(id));
            let table = m.tables.iter().find(|t| t.id.as_str() == id).unwrap();
            assert_eq!(table.minimum.get(), 1);
            assert_eq!(table.maximum.get(), 40);
            assert_eq!(table.rows.len(), 40);
            assert_eq!(
                json!(table.value_type),
                json!({"kind":"quantity","value":{"unit":unit}})
            );
            let positions: Vec<_> = set["stats"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|x| x["stat"] == source_name)
                .map(|x| x["index"].as_u64().unwrap())
                .collect();
            assert_eq!(positions.len(), index + 1);
            for (level, source_row) in set["levels"].as_array().unwrap().iter().enumerate() {
                assert_eq!(source_row["level"], level + 1);
                let mut total = 0.;
                for position in &positions {
                    let values: Vec<_> = source_row["row"]["positions"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|x| x["index"] == *position)
                        .collect();
                    assert_eq!(values.len(), 1);
                    assert_eq!(
                        source_row["row"]["interpolation"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter(|x| x["index"] == *position && x["value"] == 1)
                            .count(),
                        1
                    );
                    total += values[0]["value"].as_f64().unwrap();
                }
                for constant in set["constants"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|x| x["stat"] == source_name)
                {
                    assert_eq!(constant["value_present"], true);
                    total += constant["value"].as_f64().unwrap();
                }
                assert_eq!(
                    json!(table.rows[level]),
                    quantity(total, unit),
                    "ordered source contributors at set{} level{}",
                    index + 1,
                    level + 1
                );
                let observed = &v["admitted"][index * 40 + level];
                assert_eq!(observed["stat_set"], index + 1);
                assert_eq!(observed["level"], level + 1);
                assert_eq!(observed[channel].as_f64(), Some(total));
            }
        }
    }
    assert_eq!(expected_tables.len(), m.tables.len());
    assert_eq!(route.schema_version, 2);
    assert_eq!(
        route.definitions,
        serde_json::from_value::<poe_optimizer_core::data::DataIdentity>(b["definitions"].clone())
            .unwrap()
    );
    assert_eq!(route.outputs.len(), 5);
    let output = route
        .outputs
        .iter()
        .find(|r| json!(r.output) == b["output"])
        .unwrap();
    assert!(
        !output.routes.is_complete() && !output.source_selectors.as_ref().unwrap().is_complete()
    );
    assert_eq!(output.routes.members.len(), 6);
    assert_eq!(output.source_selectors.as_ref().unwrap().members.len(), 2);
    for (index, row) in sets.iter().enumerate() {
        let selector = &output
            .source_selectors
            .as_ref()
            .unwrap()
            .members
            .iter()
            .find(|s| json!(s.selection)["value"]["stat_set"] == row["stat_set"])
            .unwrap();
        let tag = if index == 0 {
            "ordinary"
        } else {
            "cold-infused"
        };
        let selection = json!({"kind":"exact","value":{"part":b["part"],"mode":b["mode"],"stat_set":row["stat_set"]}});
        assert_eq!(
            json!(selector),
            json!({"id":format!("ice-nova-{tag}-intrinsic"),"selection":selection,"sources":[{"id":"intrinsic","origin":{"kind":"current_action"}}],"policy":{"kind":"fixed","value":{"source":"intrinsic"}}})
        );
        for channel in ["cold_min", "cold_max", "radius"] {
            let route = output
                .routes
                .members
                .iter()
                .find(|r| {
                    json!(r.selection) == selection && json!(r.target) == b["channels"][channel]
                })
                .unwrap();
            assert_eq!(
                json!(route.source),
                json!({"kind":"selected","value":{"selector":selector.id,"stats":[{"source":"intrinsic","stat":row["internal"][channel]}]}})
            );
        }
    }
}

fn source_proof() {
    let a: Value = read("authoring.json");
    let v: Value = read("source-vectors.json");
    let p = &a["source_validation"];
    assert_eq!(p["status"], "passed");
    let mut reports = vec![];
    for (path, bytes, hash) in [
        ("evidence_json", "evidence_bytes", "evidence_sha256"),
        (
            "evidence_on_json",
            "evidence_on_bytes",
            "evidence_on_sha256",
        ),
    ] {
        let raw = fs::read(root().join(p[path].as_str().unwrap())).unwrap();
        assert_eq!(raw.len() as u64, p[bytes].as_u64().unwrap());
        assert_eq!(format!("{:x}", Sha256::digest(&raw)), p[hash]);
        reports.push(raw);
    }
    assert!(
        reports[0] == reports[1],
        "complete source JIT reports agree"
    );
    assert_eq!(v["source_report_sha256"], p["evidence_sha256"]);
    let report: Value = serde_json::from_slice(&reports[0]).unwrap();
    for (field, authored) in [
        ("source_revision", "source_revision"),
        ("manifest_sha256", "source_manifest_sha256"),
        ("files", "source_files"),
        ("original_sources", "original_sources"),
    ] {
        assert_eq!(report[field], a[authored]);
    }
    for field in [
        "business_wrappers",
        "source_tables_mutated",
        "native_build_parity",
        "native_inventory_authority",
        "final_input_authority",
        "canonical_parity_lifecycle_selected",
    ] {
        assert_eq!(report[field], false);
    }
    assert_eq!(
        report["admitted_helper_domain"],
        json!({"minimum_level":1,"maximum_level":40,"stat_sets":2,"quality":0,"alt_quality":false})
    );
    let stages = ["fresh", "rebuilt_once", "rebuilt_twice"];
    assert_eq!(report["lifecycle_stages"], json!(stages));
    let names_expected = [
        "original-01",
        "original-02",
        "original-03",
        "original-04",
        "original-05",
        "main-two-calcs-one",
        "main-one-calcs-two",
        "raw-level-one",
        "raw-level-forty",
        "raw-quality-fractional",
        "independent-copy-level-one",
        "repeat-original-05",
    ];
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), names_expected.len());
    for original in a["original_sources"].as_array().unwrap() {
        let bytes = fs::read(root().join(original["path"].as_str().unwrap())).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), original["sha256"]);
    }
    for (case, name) in cases.iter().zip(names_expected) {
        assert_eq!(case["name"], name);
        for stage in stages {
            let intrinsic = &case["states"][stage]["intrinsic"];
            assert_eq!(intrinsic["constructed"], v["constructed"]);
            assert_eq!(intrinsic["admitted"].as_array().unwrap().len(), 80);
            assert_eq!(intrinsic["boundaries"].as_array().unwrap().len(), 36);
            for flag in [
                "original_function_preserved",
                "constructed_inputs_preserved",
                "caller_inputs_preserved",
                "requested_jit_verified",
            ] {
                assert_eq!(intrinsic[flag], true);
            }
            for (index, probe) in intrinsic["admitted"].as_array().unwrap().iter().enumerate() {
                let expected = &v["admitted"][index];
                assert_eq!(probe["success"], true);
                assert_eq!(probe["include_alt_quality"], false);
                assert_eq!(probe["instance"]["quality"], 0);
                assert_eq!(probe["stat_set"], expected["stat_set"]);
                assert_eq!(probe["instance"]["level"], expected["level"]);
                for (channel, name) in names() {
                    assert_eq!(probe["stats"][name], expected[channel]);
                }
            }
        }
    }
}

fn preserve(prior: &StagedOwnedRelease, next: &StagedOwnedRelease, m: &OwnedReleaseMigrationInput) {
    let old = prior.input();
    let new = next.input();
    let mut expected = old.recipe.schema.clone();
    expected.release = m.release.clone();
    for row in &m.schema {
        match row {
            SchemaExtensionEntry::Definition(d) => {
                if let Some(entry) = expected
                    .definitions
                    .iter_mut()
                    .find(|x| x.address() == d.address())
                {
                    *entry = d.clone();
                } else {
                    expected.definitions.push(d.clone());
                }
            }
            SchemaExtensionEntry::Slot(s) => {
                assert!(!expected.slots.iter().any(|x| x.address() == s.address()));
                expected.slots.push(s.clone());
            }
        }
    }
    expected
        .definitions
        .sort_by_cached_key(DefinitionDescriptor::address);
    expected.slots.sort_by_cached_key(SlotDescriptor::address);
    assert!(
        expected == new.recipe.schema,
        "only the final-level declaration and nine action-local stats"
    );
    let mut expected = old.recipe.rules.clone();
    expected.definitions = next.receipt().definitions.clone();
    for table in &m.tables {
        assert!(!expected.tables.iter().any(|x| x.id == table.id));
        expected.tables.push(table.clone());
    }
    for row in &m.owners {
        assert!(!expected.owners.iter().any(|x| x.owner == row.owner));
        expected.owners.push(row.clone());
    }
    assert!(
        expected == new.recipe.rules,
        "all old programs, tables, receivers and application registry remain exact"
    );
    assert_eq!(new.recipe.registry.last_issued.get(), 0x32df);
    assert_eq!(
        new.recipe.registry.entries.len(),
        old.recipe.registry.entries.len() + 10
    );
    assert_eq!(
        new.recipe.registry.entries[..old.recipe.registry.entries.len()],
        old.recipe.registry.entries
    );
    assert_eq!(new.query_sets, old.query_sets);
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    migration_preservation::assert_import_rebindings_only(prior, next);
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source_proof();
    let a: Value = read("authoring.json");
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in ["definitions", "registry", "roles", "normalization"] {
        assert_eq!(receipt[field], a[field]);
    }
    let d: Value = read("dependencies.json");
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(d["definitions"].clone()).unwrap();
    let slots: Vec<SlotDescriptor> = serde_json::from_value(d["slots"].clone()).unwrap();
    for row in definitions {
        assert_eq!(
            prior
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
    for row in slots {
        assert_eq!(
            prior
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
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let migrated = compile_owned_release_migration(prior, m.clone(), Default::default()).unwrap();
    preserve(prior, &migrated, &m);
    let mut routing: ActionRoutingInput = read("routing.json");
    assert_eq!(routing.definitions, prior.receipt().definitions);
    let b = bindings();
    let old = &prior.input().recipe.routing;
    assert_eq!(old.schema_version, 1);
    assert_eq!(old.outputs.len(), 4);
    assert_eq!(routing.outputs.len(), old.outputs.len() + 1);
    let mut inverse = routing.clone();
    let position = inverse
        .outputs
        .iter()
        .position(|r| json!(r.output) == b["output"])
        .unwrap();
    inverse.outputs.remove(position);
    for (previous, adapted) in old.outputs.iter().zip(&mut inverse.outputs) {
        assert!(previous.source_selectors.is_none() && !previous.routes.is_complete());
        let selectors = adapted.source_selectors.take().unwrap();
        assert!(selectors.members.is_empty());
        assert_eq!(selectors.closure, previous.routes.closure);
        assert_eq!(
            *adapted, *previous,
            "legacy routes survive inverse V2 adaptation"
        );
    }
    inverse.schema_version = old.schema_version;
    inverse.release = old.release.clone();
    assert_eq!(
        inverse, *old,
        "only explicit V2 selector coverage and Ice output were added"
    );
    routing.definitions = migrated.receipt().definitions.clone();
    let mut input = migrated.input().clone();
    input.recipe.routing = routing.clone();
    input.provenance.push(OwnedReleaseProvenance {
        kind: key("source-bound-ice-nova-intrinsics"),
        prior_input: migrated.receipt().input,
        authoring_input: digest_owned(
            "owned-ice-nova-intrinsics-v1",
            &(
                a,
                b,
                d,
                read::<Value>("source-vectors.json"),
                read::<Value>("routing.json"),
            ),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_eq!(next.input().recipe.routing, routing);
    let mut restored = next.input().clone();
    restored.recipe.routing = migrated.input().recipe.routing.clone();
    restored.provenance.pop();
    assert!(
        restored == *migrated.input(),
        "no release mutation outside authored routing and provenance"
    );
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
