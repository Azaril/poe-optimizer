//! Finite Bidding delivery authoring and checked publication, without owner closure.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::{OwnedDefinitionKey, StatDefinition},
    owned_rules::DefinitionRules,
    owned_schema::{DefinitionDescriptor, SchemaDefinitionId, SlotDescriptor},
    owned_support_receiving::{
        SupportReceivingEntry, SupportReceivingRole, SupportTargetReceivingRoles,
    },
};
use poe_optimizer_import::{
    owned_mapping::{MappingEntry, OwnedIdRegistry},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReceivingFragment {
    roles: Vec<SupportReceivingRole>,
    targets: Vec<SupportTargetReceivingRoles>,
    supports: Vec<SupportReceivingEntry>,
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/bidding-support-delivery")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn quantity(amount: &Value, unit: &Value) -> Value {
    json!({"kind":"quantity","value":{"value":amount,"unit":unit}})
}
fn owner(gem: &Value) -> Value {
    json!({"kind":"definition","value":{"kind":"gem","value":gem}})
}
fn partial(subject: &Value, code: &str) -> Value {
    json!({"kind":"partial","value":{"gaps":[{"subject":subject,"facet":"game_rules","code":code}]}})
}
fn support_owner(s: &Value, b: &Value) -> Value {
    let mut nodes = vec![
        json!({"id":"commandable","expression":{"kind":"read","input":"commandable"}}),
        json!({"id":"cooldown","expression":{"kind":"literal","value":quantity(&s["cooldown"],&b["percent_unit"])}}),
    ];
    let mut effects = vec![json!({"id":"cooldown","when":"commandable","effect":{
        "kind":"contribute","entity":"current","stat":b["channels"]["cooldown"],
        "contribution":"increase","value":"cooldown"}})];
    if !s["damage_factor"].is_null() {
        nodes.push(json!({"id":"damage-factor","expression":{"kind":"literal","value":quantity(&s["damage_factor"],&b["factor_unit"])}}));
        effects.push(json!({"id":"damage-factor","when":"commandable","effect":{
            "kind":"contribute","entity":"current","stat":b["channels"]["damage_factor"],
            "contribution":"multiply","value":"damage-factor"}}));
    }
    let subject = owner(&s["gem"]);
    json!({"owner":subject,"programs":{
    "closure":partial(&subject,"support-owner-integration-incomplete"),"members":[
        {"id":s["programs"]["applicability"],"context":"action","reads":[],
         "nodes":[{"id":"applicable","expression":{"kind":"literal","value":{"kind":"boolean","value":true}}}],
         "effects":[{"id":"applicability","when":null,"effect":{"kind":"support_applicability","applicable":"applicable"}}]},
        {"id":s["programs"]["delivery"],"context":"action",
         "reads":[{"id":"commandable","value_type":{"kind":"boolean"},"source":{"kind":"stat","value":{"entity":"current","stat":b["channels"]["commandable"]}}}],
         "nodes":nodes,"effects":effects}
    ]}})
}
fn action_program(fact: &Value, b: &Value) -> Value {
    json!({"id":fact["program"],"context":"action","reads":[],
        "nodes":[{"id":"eligible","expression":{"kind":"literal","value":{"kind":"boolean","value":fact["commandable"]}}}],
        "effects":[{"id":"fact","when":null,"effect":{"kind":"derive","entity":"current","stat":b["channels"]["commandable"],"value":"eligible"}}]})
}
fn check_source_definitions(observed: &Value, bindings: &Value) {
    let definitions = observed.as_array().unwrap();
    assert_eq!(definitions.len(), 2);
    for support in bindings["supports"].as_array().unwrap() {
        let rows: Vec<_> = definitions
            .iter()
            .filter(|d| d["effect"] == support["source_effect"])
            .collect();
        assert_eq!(rows.len(), 1);
        let definition = rows[0];
        let sets = definition["stat_sets"].as_array().unwrap();
        assert_eq!(sets.len(), 1);
        let constants = sets[0]["constants"]["positions"].as_array().unwrap();
        let mut expected = vec![];
        if !support["damage_factor"].is_null() {
            expected.push(("support_command_skill_damage_+%_final", 30.0));
            assert_eq!(
                support["damage_factor"].as_f64().unwrap(),
                1.0 + 30.0 / 100.0
            );
        }
        expected.push((
            "minion_command_skill_cooldown_speed_+%",
            support["cooldown"].as_f64().unwrap(),
        ));
        assert_eq!(constants.len(), expected.len());
        let constant_rows: Vec<_>=constants.iter().zip(expected).enumerate().map(|(index,(row,(stat,amount)))| {
            let value=&row["value"]["positions"][1]["value"];
            assert_eq!(value.as_f64().unwrap(),amount);
            // Preserve the source's JSON number encoding after checking its value.
            json!({"index":index+1,"value":{"positions":[{"index":1,"value":stat},{"index":2,"value":value}]}})
        }).collect();
        assert_eq!(
            *definition,
            json!({
                "effect":support["source_effect"],"mod_source":format!("Skill:{}",support["source_effect"].as_str().unwrap()),
                "add_types":{},"exclude_types":{},"family":{"positions":[{"index":1,"value":"Bidding"}]},
                "levels":{"positions":[{"index":1,"value":{"levelRequirement":0}}]},
                "stat_sets":[{"index":1,"label":if support["damage_factor"].is_null(){"Bidding III"}else{"Bidding II"},
                    "scope":"gem_stat_descriptions","stats":{},"constants":{"positions":constant_rows},
                    "levels":{"positions":[{"index":1,"value":{"actorLevel":1}}]}}]
            }),
            "complete observed definition; no undeclared level, quality or extra modifier family"
        );
    }
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let fragment: ReceivingFragment = read("receiving.json");
    let r = json!(fragment);
    for (field, expected) in [
        ("allocated_definitions", 1),
        ("new_programs", 12),
        ("registry_last_issued_before", 0x32f7),
        ("registry_last_issued_after", 0x32f8),
    ] {
        assert_eq!(a[field], expected);
    }
    for field in ["before", "definitions"] {
        assert_eq!(a[field], b[field]);
    }
    for field in [
        "definitions",
        "registry",
        "roles",
        "normalization",
        "mapping",
        "rules",
    ] {
        assert_eq!(a[field], d["source"][field]);
    }
    assert_eq!(a["before"], d["source"]["input"]);
    assert_eq!(json!(m.before), b["before"]);
    assert_eq!(json!(m.release), b["release"]);
    assert_eq!(m.schema_version, 4);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v19"
    );
    assert_eq!((m.schema.len(), m.owners.len()), (1, 10));
    assert!(
        m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(
        b["channels"]["damage_factor"]["key"],
        "def.00000000000032f8"
    );
    assert_eq!(
        json!(m.schema[0]),
        json!({"kind":"definition","value":{"kind":"stat","value":{
        "id":b["channels"]["damage_factor"],"schema":{"kind":"known","value":{
            "value":{"kind":"quantity","value":{"unit":b["factor_unit"]}},"targets":["action"]}}}}})
    );
    let supports = b["supports"].as_array().unwrap();
    assert_eq!(supports.len(), 2);
    for (s, (id, name, cooldown, damage)) in supports.iter().zip([
        (0x673, "SupportBiddingPlayerThree", 80.0, None),
        (0x674, "SupportBiddingPlayerTwo", 30.0, Some(1.3)),
    ]) {
        assert_eq!(s["gem"]["key"], format!("def.{id:016x}"));
        assert_eq!(s["source_effect"], name);
        assert_eq!(s["cooldown"], cooldown);
        assert_eq!(s["damage_factor"], json!(damage));
        let matches: Vec<_> = m
            .owners
            .iter()
            .filter(|o| json!(o.owner) == owner(&s["gem"]))
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(json!(matches[0]), support_owner(s, &b));
    }
    let djinn: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/djinn-actions/bindings.json")).unwrap(),
    )
    .unwrap();
    let old: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    assert_eq!(old.len(), 8);
    let mut seen = BTreeSet::new();
    let mut expected_targets = vec![];
    for family in djinn["families"].as_array().unwrap() {
        let mut endpoints = vec![];
        for child in family["actions"].as_array().unwrap() {
            let matches: Vec<_> = b["action_facts"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|f| f["output"] == child["output"])
                .collect();
            assert_eq!(matches.len(), 1);
            let fact = matches[0];
            assert!(seen.insert(fact["source_effect"].as_str().unwrap()));
            assert_eq!(fact["family"], family["key"]);
            assert_eq!(fact["source_effect"], child["skill_id"]);
            assert_eq!(fact["stat_sets"], child["stat_sets"]);
            assert_eq!(
                fact["commandable"],
                child["skill_id"] != "PassiveTriggeredManaWaveWaterDjinn"
            );
            let subject =
                json!({"kind":"slot","value":{"kind":"action_output","value":child["output"]}});
            let prior = old.iter().find(|o| json!(o.owner) == subject).unwrap();
            assert!(!prior.programs.is_complete());
            let next = m.owners.iter().find(|o| o.owner == prior.owner).unwrap();
            let mut restored = next.clone();
            assert_eq!(
                json!(restored.programs.members.pop().unwrap()),
                action_program(fact, &b)
            );
            assert_eq!(
                &restored, prior,
                "exact prior programs and Partial closure survive"
            );
            endpoints.push(json!({"kind":"action","path":[family["minion"]["entering_grant"],child["entering_grant"]],
                "output":child["output"],"selection":{"kind":"all_declared"},"admission":{"kind":"receiving_skill","summoner_path":[]}}));
        }
        let subject = json!({"kind":"definition","value":{"kind":"skill","value":family["skill"]}});
        expected_targets.push(json!({"owner":{"kind":"skill","value":family["skill"]},"roles":{
            "members":[{"role":"minion-command-action","endpoints":{"members":endpoints,"closure":{"kind":"complete"}}}],
            "closure":partial(&subject,"remaining-support-receivers-not-converted")}}));
    }
    assert_eq!(seen.len(), 8);
    assert_eq!(b["action_facts"].as_array().unwrap().len(), 8);
    assert_eq!(
        r["roles"],
        json!([{"id":"minion-command-action","kind":"action"}])
    );
    assert_eq!(r["targets"], json!(expected_targets));
    assert_eq!(r["supports"],json!(supports.iter().map(|s|json!({"gem":s["gem"],"receivers":{
        "members":[{"role":"minion-command-action","applicability":s["programs"]["applicability"],"delivery":[s["programs"]["delivery"]]}],
        "closure":partial(&owner(&s["gem"]),"remaining-support-receivers-not-converted")}})).collect::<Vec<_>>()));
    check_evidence(&a, &v, false);
}

fn check_evidence(a: &Value, v: &Value, full_reports: bool) {
    assert_eq!(a["source_validation"]["status"], "passed");
    assert_eq!(
        a["scope"],
        json!({"complete_original_builds":0,"retired_input_issues":0,
        "closed_rule_owners":0,"receiving_fragment_only":true,"evaluation_bundle_added":false,
        "final_damage_or_cooldown_claimed":false})
    );
    assert_eq!(
        v["scope"],
        json!({"numerical_component":true,"complete_build_parity":false,
        "complete_supplier_scope":false})
    );
    for field in ["source_revision", "source_manifest_sha256"] {
        assert_eq!(v[field], a[field]);
    }
    assert_eq!(v["evidence_view"], "bidding_distinct_channel_projection_v1");
    assert_eq!(a["source_validation"]["evidence_view"], v["evidence_view"]);
    assert_eq!(
        v["determinism_contract"],
        json!({
        "deterministic_view":"bidding_distinct_channel_projection_v1",
        "raw_parent_outer_cross_channel_order":"unspecified",
        "source_reason":"CalcActiveSkill.mergeStatSet iterates pairs(stats); MinionModifier transfers exact nested objects into separate named ModDB channels",
        "projected_fields":["delivery.contexts[].parent_minion_modifiers","delivery.contexts[].children[].queries.producer_joins[].parent_outer_indices"],
        "same_channel_arithmetic_order":"preserved","all_other_fields":"exact"})
    );
    let bindings: Value = read("bindings.json");
    let cases = v["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 22);
    let focus_cases: BTreeSet<_> = cases
        .iter()
        .enumerate()
        .filter(|(_, c)| c["control"]["kind"] == "focus")
        .map(|(index, _)| index)
        .collect();
    assert_eq!(focus_cases.len(), 8);
    let artifacts = a["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        artifacts.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "bindings",
            "dependencies",
            "migration",
            "receiving",
            "source-vectors"
        ]
    );
    for (name, expected) in artifacts {
        let bytes = fs::read(
            root()
                .join("data/owned/poe2/3887ae68/bidding-support-delivery")
                .join(format!("{name}.json")),
        )
        .unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(*expected, hash(&bytes));
    }
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(a["source_manifest_sha256"], hash(&bytes));
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    let mut pins = BTreeSet::new();
    for pin in a["source_files"].as_array().unwrap() {
        assert_eq!(
            pin.as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["path", "sha256"]
        );
        let path = pin["path"].as_str().unwrap();
        assert!(!path.is_empty() && pins.insert(path));
        let matched: Vec<_> = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["path"] == pin["path"])
            .collect();
        assert_eq!(matched.len(), 1);
        assert!(matched[0]["bytes"].as_u64().is_some());
        // Source receipts deliberately project the authenticated manifest's
        // path and digest; file lengths remain covered by the manifest hash.
        assert_eq!(
            *pin,
            json!({"path":matched[0]["path"],"sha256":matched[0]["sha256"]})
        );
    }
    assert!(!pins.is_empty());
    let reports = v["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    assert_ne!(reports[0]["path"], reports[1]["path"]);
    for field in ["bytes", "sha256", "observations"] {
        assert_eq!(reports[0][field], reports[1][field]);
    }
    let receipts = a["source_validation"]["reports"].as_array().unwrap();
    assert_eq!(receipts.len(), 2);
    let raw_receipts = v["raw_reports"].as_array().unwrap();
    assert_eq!(raw_receipts.len(), 2);
    assert_eq!(v["raw_reports"], a["source_validation"]["raw_reports"]);
    assert_ne!(raw_receipts[0]["path"], raw_receipts[1]["path"]);
    for ((report, receipt), raw_receipt) in reports.iter().zip(receipts).zip(raw_receipts) {
        for field in ["path", "bytes", "sha256"] {
            assert_eq!(report[field], receipt[field]);
        }
        assert_eq!(
            raw_receipt["path"],
            format!(
                "{}.raw.json",
                report["path"]
                    .as_str()
                    .unwrap()
                    .strip_suffix(".json")
                    .unwrap()
            )
        );
        let observations = report["observations"].as_array().unwrap();
        assert_eq!(observations.len(), 49);
        let mut pointers = BTreeSet::new();
        let mut coverage = BTreeSet::new();
        let mut effects = BTreeSet::new();
        for row in observations {
            let pointer = row["pointer"].as_str().unwrap();
            assert!(pointers.insert(pointer));
            if pointer == "/cases/0/states/fresh/delivery/definitions" {
                continue;
            }
            let parts: Vec<_> = pointer.split('/').collect();
            assert_eq!(parts.len(), 8);
            assert_eq!(
                (parts[0], parts[1], parts[3], parts[5], parts[6]),
                ("", "cases", "states", "delivery", "contexts")
            );
            let case_index = parts[2].parse::<usize>().unwrap();
            assert!(focus_cases.contains(&case_index));
            assert!(parts[7].parse::<usize>().is_ok());
            assert!(["fresh", "rebuilt_once", "rebuilt_twice"].contains(&parts[4]));
            let context = &row["value"];
            let mode = context["mode"].as_str().unwrap();
            assert!(["MAIN", "CALCS"].contains(&mode));
            assert!(coverage.insert((case_index, parts[4], mode)));
            let control = &cases[case_index]["control"];
            assert_eq!(context["selected"], true);
            assert_eq!(context["effect"], control["effect"]);
            let selected: Vec<_> = context["children"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|child| child["selected"] == true)
                .collect();
            assert_eq!(selected.len(), 1);
            let child = selected[0];
            assert_eq!(child["index"], control["child"]);
            assert_eq!(
                child["stat_set_index"],
                control[if mode == "MAIN" {
                    "main_set"
                } else {
                    "calcs_set"
                }]
            );
            let facts: Vec<_> = bindings["action_facts"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|fact| fact["source_effect"] == child["effect"])
                .collect();
            assert_eq!(facts.len(), 1);
            assert_eq!(child["queries"]["commandable"], facts[0]["commandable"]);
            assert_eq!(child["queries"]["original_cfg_unchanged"], true);
            assert_eq!(child["queries"]["original_store_unchanged"], true);
            effects.insert(child["effect"].as_str().unwrap());
        }
        assert_eq!(coverage.len(), 48);
        assert_eq!(effects.len(), 8);
        assert!(pointers.contains("/cases/0/states/fresh/delivery/definitions"));
        assert!(
            pointers.len() > 1,
            "definitions alone are not delivery evidence"
        );
        let definitions = observations
            .iter()
            .find(|row| row["pointer"] == "/cases/0/states/fresh/delivery/definitions")
            .unwrap();
        check_source_definitions(&definitions["value"], &bindings);
        if full_reports {
            let bytes = fs::read(root().join(report["path"].as_str().unwrap())).unwrap();
            assert_eq!(bytes.len() as u64, report["bytes"].as_u64().unwrap());
            assert_eq!(hash(&bytes), report["sha256"]);
            let source: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(source["evidence_view"], v["evidence_view"]);
            assert_eq!(source["determinism_contract"], v["determinism_contract"]);
            assert_eq!(source["source_revision"], a["source_revision"]);
            assert_eq!(source["manifest_sha256"], a["source_manifest_sha256"]);
            assert_eq!(source["files"], a["source_files"]);
            assert_eq!(source["original_sources"], v["original_sources"]);
            assert_eq!(
                source["lifecycle_stages"],
                json!(["fresh", "rebuilt_once", "rebuilt_twice"])
            );
            assert_eq!(
                source["cases"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|case| json!({"name":case["name"],"control":case["control"]}))
                    .collect::<Vec<_>>(),
                *cases
            );
            for row in observations {
                let pointer = row["pointer"].as_str().unwrap();
                assert_eq!(
                    source.pointer(pointer).unwrap(),
                    &row["value"],
                    "exact source observation {pointer}"
                );
            }
            // Raw order is deliberately unspecified across distinct channels;
            // authenticate each preserved diagnostic report independently.
            let raw_bytes = fs::read(root().join(raw_receipt["path"].as_str().unwrap())).unwrap();
            assert_eq!(
                raw_bytes.len() as u64,
                raw_receipt["bytes"].as_u64().unwrap()
            );
            assert_eq!(hash(&raw_bytes), raw_receipt["sha256"]);
            let raw: Value = serde_json::from_slice(&raw_bytes).unwrap();
            assert_eq!(raw["evidence_view"], "raw_source_observation");
            for field in [
                "determinism_contract",
                "source_revision",
                "manifest_sha256",
                "files",
                "original_sources",
                "lifecycle_stages",
            ] {
                assert_eq!(raw[field], source[field]);
            }
            assert_eq!(
                raw["cases"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|case| json!({"name":case["name"],"control":case["control"]}))
                    .collect::<Vec<_>>(),
                *cases
            );
        }
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let receiving: ReceivingFragment = read("receiving.json");
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    check_evidence(&a, &v, true);
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "roles",
        "normalization",
        "mapping",
        "rules",
    ] {
        assert_eq!(receipt[field], a[field]);
    }
    assert!(prior.evaluation().is_none());
    for definition in
        serde_json::from_value::<Vec<DefinitionDescriptor>>(d["supporting_definitions"].clone())
            .unwrap()
    {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| **x == definition)
                .count(),
            1
        );
    }
    let old_owners: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    for row in &old_owners {
        assert_eq!(
            prior
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|x| *x == row)
                .count(),
            1
        );
    }
    for slot in serde_json::from_value::<Vec<SlotDescriptor>>(d["slots"].clone()).unwrap() {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .slots
                .iter()
                .filter(|x| **x == slot)
                .count(),
            1
        );
    }
    for row in serde_json::from_value::<Vec<MappingEntry>>(d["mapping_rows"].clone()).unwrap() {
        assert_eq!(
            prior
                .input()
                .mapping
                .entries
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    let migrated =
        compile_owned_release_migration(prior, migration.clone(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    assert_eq!(input.provenance.len(), prior.input().provenance.len() + 1);
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("bidding-support-delivery-v1").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-bidding-support-delivery-v1",
            &(a, b.clone(), d, v, migration, receiving),
            8 * 1024 * 1024,
        )
        .unwrap(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut registry =
        OwnedIdRegistry::new(prior.input().recipe.registry.clone(), Default::default()).unwrap();
    let added = registry
        .allocate_definition::<StatDefinition>()
        .unwrap()
        .address();
    assert_eq!(
        added.key().as_str(),
        b["channels"]["damage_factor"]["key"].as_str().unwrap()
    );
    assert_eq!(next.input().recipe.registry, *registry.input());
    let mut restored = next.input().recipe.clone();
    restored.registry = prior.input().recipe.registry.clone();
    assert_eq!(
        restored.schema.definitions.len(),
        prior.input().recipe.schema.definitions.len() + 1
    );
    restored
        .schema
        .definitions
        .retain(|row| row.address() != added);
    restored.schema.release = prior.input().recipe.schema.release.clone();
    for support in b["supports"].as_array().unwrap() {
        let before = restored.rules.owners.len();
        restored
            .rules
            .owners
            .retain(|row| json!(row.owner) != owner(&support["gem"]));
        assert_eq!(restored.rules.owners.len() + 1, before);
    }
    for old in old_owners {
        let owner = old.owner.clone();
        *restored
            .rules
            .owners
            .iter_mut()
            .find(|row| row.owner == owner)
            .unwrap() = old;
    }
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "only exact new Stat/support owners and eight Action fact appends"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
