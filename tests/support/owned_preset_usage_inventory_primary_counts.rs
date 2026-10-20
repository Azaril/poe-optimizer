//! Existing-primary count projections and exact source-linked output inverses.
//! This is test authoring/accounting, not a native count or eligibility evaluator.
use crate::{family, preservation};
use family::{Output, data, read, root, sha};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_schema::{DefinitionDescriptor, SlotDescriptor},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{
        NormalizationPolicy, PrimarySkillUsageInput, UsageInputPolicy, UsageInventoryPolicy,
    },
    owned_release::StagedOwnedRelease,
    owned_skill_catalog::OwnedGemRoleRow,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, io::Read};

#[path = "owned_preset_usage_inventory_primary_count_controls.rs"]
mod controls;
pub use controls::run as run_controls;
pub const COUNT: &str = "def.000000000000326a";
pub const PARAMETER: &str = "def.000000000000326b";

#[derive(Deserialize)]
struct Packet {
    kind: String,
    transitions: Vec<Value>,
}
fn packet() -> Packet {
    serde_json::from_value(read(data().join("primary-count-inputs.json"))).unwrap()
}
fn rows(value: &Value) -> &[Value] {
    value.as_array().unwrap()
}
pub(super) fn transitions() -> Vec<Value> {
    packet().transitions
}
fn pin(path: &std::path::Path, expected: &Value) {
    let bytes = fs::read(path).unwrap();
    assert_eq!(json!(sha(&bytes)), expected["sha256"]);
    assert_eq!(json!(bytes.len()), expected["bytes"]);
}
fn singleton(role: &Value, expected_role: &str) {
    assert_eq!(role["materialization"]["kind"], "physical");
    assert_eq!(role["role"]["kind"], "known");
    assert_eq!(role["role"]["value"], expected_role);
    assert_eq!(role["primary"]["kind"], "known");
    assert_eq!(role["construction"]["kind"], "observed");
    let c = &role["construction"]["value"];
    assert_eq!(c["primary"]["resolution"]["kind"], "mapped");
    assert_eq!(
        c["primary"]["resolution"]["value"],
        role["primary"]["value"]
    );
    assert_eq!(c["resolved_effects"], json!([c["primary"]]));
    assert!(c["display_order"].is_null());
    for field in [
        "declared_additional_effects",
        "constructed_additional_effects",
        "resolved_additional_effects",
        "missing_references",
        "additional_stat_sets",
    ] {
        assert_eq!(c[field], json!([]), "{field}");
    }
}
/// Semantic delta checks deliberately separate from the outer byte seal.
fn check_transition(t: &Value, template: &Value) {
    let before: PrimarySkillUsageInput =
        serde_json::from_value(t["physical_before"].clone()).unwrap();
    let after: PrimarySkillUsageInput =
        serde_json::from_value(t["physical_after"].clone()).unwrap();
    assert_eq!(after.policies.len(), before.policies.len() + 1);
    let mut inverse = after.clone();
    let count = inverse.policies.pop().unwrap();
    assert_eq!(
        inverse, before,
        "no change to guards, source frame or prior consumers"
    );
    assert_eq!(count.policy.key().as_str(), COUNT);
    assert_eq!(count.parameters.len(), 1);
    assert_eq!(count.parameters[0].slot.slot.key().as_str(), PARAMETER);
    let mut expected = template.clone();
    expected["parameters"][0]["source"]["group"]["id"] = json!("requested-group-count");
    expected["parameters"][0]["source"]["occurrence"]["id"] = json!("requested-skill-count");
    expected["parameters"][0]["source"]["fallback_admission"]["companions"] = json!(
        rows(&t["companions"])
            .iter()
            .map(|c| &c["companion"])
            .collect::<Vec<_>>()
    );
    assert_eq!(serde_json::to_value(&count).unwrap(), expected);
    singleton(&t["primary_role"], "skill_use");
    assert_eq!(t["primary_role"]["gem"], t["gem"]);
    assert_eq!(
        t["primary_role"]["primary"]["value"],
        t["physical_before"]["primary"]
    );
    let mut seen = BTreeSet::new();
    for companion in rows(&t["companions"]) {
        singleton(&companion["role"], "support_assignment");
        let tuple = &companion["companion"];
        assert!(seen.insert(tuple["gem"]["key"].as_str().unwrap()));
        assert_eq!(tuple["gem"], companion["role"]["gem"]);
        let c = &companion["role"]["construction"]["value"];
        assert_eq!(tuple["game_id"], c["game_id"]);
        assert_eq!(tuple["variant_id"], c["variant_id"]);
        assert_eq!(tuple["skill_id"], c["primary"]["effect_id"]);
        assert_ne!(
            companion["role"]["primary"]["value"],
            t["physical_before"]["primary"]
        );
        assert_ne!(tuple["skill_id"], t["physical_before"]["skill_id"]);
    }
    let mut expected_inventory = t["inventory_before"].clone();
    assert_eq!(rows(&expected_inventory["fields"]).len(), 18);
    let destination = json!({"kind":"usage_parameter","policy":count.policy,"parameter":count.parameters[0].slot});
    for field in expected_inventory["fields"].as_array_mut().unwrap() {
        let typed: Vec<_> = rows(&field["destinations"])
            .iter()
            .filter(|d| d["kind"] == "usage_parameter")
            .cloned()
            .collect();
        field["destinations"] = if field["field"]["kind"] == "occurrence_attribute"
            && field["field"]["name"] == "count"
        {
            json!([destination])
        } else if field["field"]["kind"] == "group_attribute"
            && field["field"]["name"] == "includeInFullDPS"
        {
            json!(typed)
        } else {
            let mut kept = vec![json!({"kind":"input_disposition"})];
            kept.extend(typed);
            json!(kept)
        };
        assert!(!rows(&field["destinations"]).is_empty());
    }
    let fields = expected_inventory["fields"].as_array_mut().unwrap();
    let index = fields
        .iter()
        .position(|f| {
            f["field"]["kind"] == "group_attribute" && f["field"]["name"] == "includeInFullDPS"
        })
        .unwrap();
    fields.insert(index, json!({"field":{"kind":"group_attribute","name":"groupCount"},"destinations":[destination]}));
    assert_eq!(expected_inventory, t["inventory_after"]);
}
pub fn check_data() {
    let proof = read(data().join("primary-count-source.json"));
    let p = packet();
    assert_eq!(p.kind, "reviewed-existing-primary-count-projections");
    assert_eq!(p.transitions.len(), 2);
    assert_eq!(
        p.transitions
            .iter()
            .map(|t| t["gem"]["key"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["def.000000000000086b", "def.0000000000000797"]
    );
    for t in &p.transitions {
        check_transition(t, &proof["shared_count_template"]);
    }
    let inventory = read(data().join("inventory.json"));
    assert_eq!(rows(&inventory["rows"]).len(), 25);
    assert_eq!(
        rows(&inventory["rows"])
            .iter()
            .map(|r| rows(&r["fields"]).len())
            .sum::<usize>(),
        456
    );
    for t in &p.transitions {
        let found: Vec<_> = rows(&inventory["rows"])
            .iter()
            .filter(|r| r["source"]["kind"] == "physical" && r["source"]["gem"] == t["gem"])
            .collect();
        assert_eq!(found, vec![&t["inventory_after"]]);
    }
    let evidence = read(data().join("evidence.json"));
    for key in ["inputs", "source"] {
        let p = &evidence["primary_requested_counts"][key];
        pin(&data().join(p["file"].as_str().unwrap()), p);
    }
    let census = read(data().join("source-census.json"));
    let mut all = BTreeSet::new();
    for t in &p.transitions {
        for occurrence in rows(&t["occurrences"]) {
            let case = occurrence["case"].as_u64().unwrap() as usize;
            let source = &census[case - 1];
            pin(&root().join(source["path"].as_str().unwrap()), source);
            let saved: Vec<_> = rows(&source["gem_rows"])
                .iter()
                .filter(|r| r["ordinal"] == occurrence["row"]["ordinal"])
                .collect();
            assert_eq!(saved, vec![&occurrence["row"]]);
            assert!(all.insert((case, occurrence["row"]["ordinal"].as_u64().unwrap())));
            let siblings: Vec<_> = rows(&source["gem_rows"])
                .iter()
                .filter(|r| {
                    r["group"] == occurrence["row"]["group"]
                        && r["ordinal"] != occurrence["row"]["ordinal"]
                })
                .collect();
            assert_eq!(siblings.len(), rows(&occurrence["companions"]).len());
            for (sibling, known) in siblings.into_iter().zip(rows(&occurrence["companions"])) {
                assert_eq!(sibling["ordinal"], known["ordinal"]);
                let companions: Vec<_> = rows(&t["companions"])
                    .iter()
                    .filter(|c| c["companion"]["gem"] == known["gem"])
                    .collect();
                assert_eq!(companions.len(), 1);
                for (attribute, member) in [
                    ("gemId", "game_id"),
                    ("variantId", "variant_id"),
                    ("skillId", "skill_id"),
                    ("nameSpec", "name_spec"),
                ] {
                    let values: Vec<_> = rows(&sibling["attributes"])
                        .iter()
                        .filter(|a| a["name"] == attribute)
                        .collect();
                    assert_eq!(values.len(), 1);
                    assert_eq!(values[0]["decoded"], companions[0]["companion"][member]);
                }
            }
        }
    }
    assert_eq!(
        all,
        BTreeSet::from([
            (4, 119),
            (5, 202),
            (5, 221),
            (5, 234),
            (5, 264),
            (5, 283),
            (5, 331),
            (5, 396)
        ])
    );
    pin(
        &root().join(proof["manifest"]["path"].as_str().unwrap()),
        &proof["manifest"],
    );
    let manifest = read(root().join(proof["manifest"]["path"].as_str().unwrap()));
    assert_eq!(proof["source_revision"], manifest["upstream_revision"]);
    assert_eq!(rows(&proof["archives"]).len(), 3);
    for archive in rows(&proof["archives"]) {
        assert!(rows(&manifest["files"]).contains(&archive["source"]));
        let compressed = fs::read(root().join(archive["file"].as_str().unwrap())).unwrap();
        assert_eq!(json!(sha(&compressed)), archive["compressed_sha256"]);
        assert_eq!(json!(compressed.len()), archive["compressed_bytes"]);
        let mut raw = Vec::new();
        flate2::read::GzDecoder::new(compressed.as_slice())
            .take(1024 * 1024 + 1)
            .read_to_end(&mut raw)
            .unwrap();
        assert!(raw.len() <= 1024 * 1024);
        assert_eq!(json!(raw.len()), archive["raw_bytes"]);
        assert_eq!(json!(sha(&raw)), archive["source"]["sha256"]);
        let text = std::str::from_utf8(&raw).unwrap();
        for span in rows(&proof["spans"])
            .iter()
            .filter(|s| s["file"] == archive["source"]["path"])
        {
            let first = span["first"].as_u64().unwrap() as usize;
            let last = span["last"].as_u64().unwrap() as usize;
            let selected = text
                .split('\n')
                .skip(first - 1)
                .take(last - first + 1)
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            assert_eq!(json!(selected), span["text"]);
            assert_eq!(json!(sha(selected.as_bytes())), span["sha256"]);
        }
    }
    // Historical run paths are labels, never dependencies of ordinary tests.
    assert_eq!(
        proof["historical_observations"]["frost"]["declared_count_domain"],
        json!([1])
    );
    assert_eq!(
        proof["count_parameter"]["value"]["schema"]["value"]["value"],
        json!({"kind":"integer","value":{"minimum":0,"maximum":4}})
    );
    let original_source =
        read(root().join("data/owned/poe2/3887ae68/ice-nova-count-inputs/source-evidence.json"));
    let license = &original_source["license"];
    let bytes = fs::read(root().join(license["file"].as_str().unwrap())).unwrap();
    assert_eq!(json!(sha(&bytes)), license["sha256"]);
    assert_eq!(json!(bytes.len()), license["bytes"]);
}
pub fn mutation_controls() {
    let source = read(data().join("primary-count-source.json"));
    let original = transitions().remove(0);
    for changed in [
        {
            let mut v = original.clone();
            v["physical_after"]["guards"] = json!([]);
            v
        },
        {
            let mut v = original.clone();
            v["inventory_after"]["fields"][0]["destinations"] =
                json!([{"kind":"input_disposition"}]);
            v
        },
        {
            let mut v = original.clone();
            v["companions"][0]["role"]["construction"]["value"]["missing_references"] =
                json!(["missing"]);
            v
        },
        {
            let mut v = original.clone();
            v["companions"][0]["role"]["role"]["value"] = json!("skill_use");
            v
        },
    ] {
        assert!(
            std::panic::catch_unwind(|| check_transition(
                &changed,
                &source["shared_count_template"]
            ))
            .is_err()
        );
    }
}
pub fn authenticate(prior: &StagedOwnedRelease) {
    let proof = read(data().join("primary-count-source.json"));
    let definition: DefinitionDescriptor =
        serde_json::from_value(proof["count_schema"].clone()).unwrap();
    let parameter: SlotDescriptor =
        serde_json::from_value(proof["count_parameter"].clone()).unwrap();
    assert_eq!(
        prior
            .input()
            .recipe
            .schema
            .definitions
            .iter()
            .filter(|d| **d == definition)
            .count(),
        1
    );
    assert_eq!(
        prior
            .input()
            .recipe
            .schema
            .slots
            .iter()
            .filter(|d| **d == parameter)
            .count(),
        1
    );
    for t in transitions() {
        for value in std::iter::once(&t["primary_role"])
            .chain(rows(&t["companions"]).iter().map(|c| &c["role"]))
        {
            let role: OwnedGemRoleRow = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(
                prior
                    .input()
                    .roles
                    .roles
                    .iter()
                    .filter(|r| **r == role)
                    .count(),
                1
            );
        }
    }
}
pub fn apply(normalization: &mut NormalizationPolicy, inverse: bool) {
    let Some(UsageInputPolicy::PobOccurrenceUsageV3 { physical, .. }) =
        &mut normalization.usage_inputs
    else {
        panic!("current usage policy")
    };
    for t in transitions() {
        let before: PrimarySkillUsageInput = serde_json::from_value(
            t[if inverse {
                "physical_after"
            } else {
                "physical_before"
            }]
            .clone(),
        )
        .unwrap();
        let after: PrimarySkillUsageInput = serde_json::from_value(
            t[if inverse {
                "physical_before"
            } else {
                "physical_after"
            }]
            .clone(),
        )
        .unwrap();
        let matches: Vec<_> = physical
            .iter()
            .enumerate()
            .filter(|(_, r)| r.gem == before.gem)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(physical[matches[0]], before);
        physical[matches[0]] = after;
    }
}
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryControl {
    Deferred,
    MissingGroupCount,
    MissingOccurrenceCount,
}
pub fn controlled_inventory(control: InventoryControl) -> UsageInventoryPolicy {
    let mut inventory = read(data().join("inventory.json"));
    for t in transitions() {
        let row = inventory["rows"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["source"]["kind"] == "physical" && r["source"]["gem"] == t["gem"])
            .unwrap();
        match control {
            InventoryControl::Deferred => *row = t["inventory_before"].clone(),
            InventoryControl::MissingGroupCount | InventoryControl::MissingOccurrenceCount => {
                let (kind, name) = match control {
                    InventoryControl::MissingGroupCount => ("group_attribute", "groupCount"),
                    InventoryControl::MissingOccurrenceCount => ("occurrence_attribute", "count"),
                    _ => unreachable!(),
                };
                let fields = row["fields"].as_array_mut().unwrap();
                let n = fields.len();
                fields.retain(|f| !(f["field"]["kind"] == kind && f["field"]["name"] == name));
                assert_eq!(fields.len() + 1, n);
            }
        }
    }
    serde_json::from_value(inventory).unwrap()
}
pub(super) fn instance(xml: &[u8]) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([73; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
pub(super) fn target(output: &Output, source: usize, transition: &Value) -> Value {
    let skill = preservation::link(&output.sidecar, source, "skill");
    let gem = preservation::link(&output.sidecar, source, "gem");
    assert_eq!(
        preservation::member(&output.draft, "gems", &gem)["definition"],
        json!({"kind":"known","value":transition["gem"]})
    );
    json!({"kind":"skill","value":{"kind":"generated","value":{
        "provider":{"root":{"kind":"skill_use","value":{"kind":"known","value":skill}},"grant_path":{"members":[],"completion":{"kind":"complete"}}},
        "slot":{"kind":"known","value":transition["physical_after"]["supply"]}
    }}})
}
pub(super) fn assert_known_count(binding: &Value, value: i64) {
    assert_eq!(binding["applicability"], "required");
    assert_eq!(binding["selection"]["policy"]["kind"], "known");
    assert_eq!(binding["selection"]["policy"]["value"]["key"], COUNT);
    assert_eq!(
        binding["selection"]["parameters"]["completion"],
        json!({"kind":"complete"})
    );
    let parameters = rows(&binding["selection"]["parameters"]["members"]);
    assert_eq!(parameters.len(), 1);
    assert_eq!(parameters[0]["slot"]["kind"], "known");
    assert_eq!(parameters[0]["slot"]["value"]["slot"]["key"], PARAMETER);
    assert_eq!(
        parameters[0]["value"],
        json!({"kind":"known","value":{"kind":"integer","value":value}})
    );
}
/// Known parameters allocate no IDs. After exact additions are removed, require
/// identical whole drafts, origins, diagnostics and allocator watermarks.
pub fn compare_additions(before: &Output, after: &Output, xml: &[u8], case: usize) -> Value {
    let imported = instance(xml);
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let sources = evidence.rows();
    let mut draft = after.draft.clone();
    let mut matched = Vec::new();
    let mut selected_count = 0;
    for t in transitions() {
        for source in sources.iter().filter(|r| {
            r.occurrence().name() == "Gem"
                && r.attribute("gemId").and_then(|a| a.decoded().ok())
                    == t["physical_before"]["game_id"].as_str()
                && r.attribute("variantId").and_then(|a| a.decoded().ok())
                    == t["physical_before"]["variant_id"].as_str()
        }) {
            let ordinal = source.occurrence().id().ordinal() as usize;
            assert_eq!(source.attribute("count").unwrap().decoded().unwrap(), "1");
            let group = &sources[source.occurrence().parent().unwrap().ordinal() as usize];
            assert!(group.attribute("groupCount").is_none());
            let set = &sources[group.occurrence().parent().unwrap().ordinal() as usize];
            let skills = &sources[set.occurrence().parent().unwrap().ordinal() as usize];
            let is_selected = skills
                .attribute("activeSkillSet")
                .unwrap()
                .decoded()
                .unwrap()
                == set.attribute("id").unwrap().decoded().unwrap();
            let preset = preservation::link(
                &after.sidecar,
                set.occurrence().id().ordinal() as usize,
                "skill_preset",
            );
            assert_eq!(
                preset,
                preservation::link(
                    &before.sidecar,
                    set.occurrence().id().ordinal() as usize,
                    "skill_preset"
                )
            );
            let expected = target(after, ordinal, &t);
            assert_eq!(expected, target(before, ordinal, &t));
            let old = preservation::member(&before.draft, "skill_presets", &preset);
            let old_rows = rows(&old["intent"]["usage"]["members"]);
            assert!(
                !old_rows
                    .iter()
                    .any(|r| r["selection"]["policy"]["value"]["key"] == COUNT
                        && r["selection"]["target"] == expected)
            );
            assert!(
                old_rows
                    .iter()
                    .any(|r| r["selection"]["target"] == expected),
                "the exact source already owns independent preferences"
            );
            let p = draft["draft"]["skill_presets"]["members"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|p| p["id"] == preset)
                .unwrap();
            let values = p["intent"]["usage"]["members"].as_array_mut().unwrap();
            let positions: Vec<_> = values
                .iter()
                .enumerate()
                .filter(|(_, r)| {
                    r["selection"]["policy"]["value"]["key"] == COUNT
                        && r["selection"]["target"] == expected
                })
                .map(|(i, _)| i)
                .collect();
            assert_eq!(positions.len(), 1);
            let count = values.remove(positions[0]);
            assert_known_count(&count, 1);
            matched.push(json!({"source_ordinal":ordinal,"group":group.occurrence().id().ordinal(),"preset":preset,"selected":is_selected,"target":expected}));
            selected_count += usize::from(is_selected);
        }
    }
    assert_eq!(matched.len(), [0, 0, 0, 1, 7][case - 1]);
    assert_eq!(selected_count, [0, 0, 0, 1, 2][case - 1]);
    assert_eq!(
        draft, before.draft,
        "complete draft inverse after exactly the source-linked count additions"
    );
    assert_eq!(
        after.sidecar, before.sidecar,
        "all origins and allocator state remain exact"
    );
    assert_eq!(
        after.report, before.report,
        "all unrelated diagnostics and counts remain exact"
    );
    json!({"all_presets":matched.len(),"selected":selected_count,"sources":matched,"exact_whole_output_inverse":true,"new_local_ids":0})
}
