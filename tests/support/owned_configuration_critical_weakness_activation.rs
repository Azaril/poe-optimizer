//! Configured encounter state is separate from skill capability and hit history.
use crate::{configuration_sources as sources, configuration_stage as common};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_content::digest_owned,
    owned_rules::{ContributionMember, RuleProgram},
    owned_schema::DefinitionDescriptor,
};
use poe_optimizer_import::{
    owned_normalize::{ConfigurationDefaultInput, ConfigurationDispositionRule},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, sync::OnceLock};

pub const SOURCE: &str = "conditionEnemyCriticalWeakness";
pub const INPUT: &str = "def.00000000000033b1";
pub const STAT: &str = "def.00000000000033b0";
pub const PROGRAM: &str = "configured-critical-weakness-activation";
pub const QUERY: &str = "outgoing-critical-weakness-enabled";
pub const BEFORE: &str = "051c663ef14260a0fd10b7cfedb6f253cb1dfede2efd017831f25b7237904e64";
const FILES: [&str; 7] = [
    "definitions.json",
    "extension.json",
    "input.json",
    "accounting.json",
    "consumer.json",
    "dependencies.json",
    "source-evidence.json",
];
const QUERIES: [&str; 16] = [
    "outgoing-critical-chance-add",
    "outgoing-critical-chance-increase",
    "outgoing-critical-chance-multiply",
    "outgoing-critical-chance-override",
    "outgoing-critical-bonus-add",
    "outgoing-critical-bonus-increase",
    "outgoing-critical-bonus-multiply",
    "outgoing-critical-bonus-override",
    "outgoing-critical-cap-add",
    "outgoing-critical-cap-override",
    "outgoing-critical-base-override",
    "outgoing-critical-enemy-chance-add",
    "outgoing-critical-enemy-chance-increase",
    "outgoing-critical-enemy-bonus-add",
    "outgoing-critical-enemy-bonus-increase",
    "outgoing-critical-weakness-enabled",
];
pub fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/configuration-critical-weakness-activation")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(directory().join(name)).unwrap()).unwrap()
}
fn value(v: impl Serialize) -> Value {
    serde_json::to_value(v).unwrap()
}
fn id(kind: &str, key: &str) -> Value {
    json!({"kind":kind,"namespace":{"game":"poe2","version":"owned-mechanics-v1"},"key":key})
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub program: RuleProgram,
    pub query: String,
    pub group: String,
    pub member: ContributionMember,
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn check_policy(input: &Value, definitions: &Value, accounting: &Value) {
    let typed: ConfigurationDefaultInput = serde_json::from_value(input.clone()).unwrap();
    assert_eq!(typed.source_name, SOURCE);
    assert_eq!(typed.value_input.key().as_str(), INPUT);
    assert_eq!(value(typed.target), "enemy");
    assert_eq!(value(typed.selection), "input_or_constructor");
    assert!(!typed.ignore_numeric_placeholder);
    assert_eq!(typed.constructor_default, ParameterValue::Boolean(false));
    assert_eq!(
        value(typed.recipe),
        json!({
            "id":"configured-enemy-critical-weakness-activation",
            "codec":{"namespace":{"game":"poe2","version":"owned-mechanics-v1"},"whitespace":"exact",
                "codec":{"kind":"boolean","value":{"tokens":[{"token":"true","value":true},{"token":"false","value":false}]}}},
            "tiers":[{"selectors":[{"lane":"input_boolean","name":SOURCE}],"duplicates":"reject"}],
            "missing":{"kind":"pending"}
        })
    );
    let descriptors: Vec<DefinitionDescriptor> =
        serde_json::from_value(definitions.clone()).unwrap();
    assert_eq!(
        value(descriptors),
        json!([{"kind":"external_input","value":{
        "id":id("external_input",INPUT),"schema":{"kind":"known","value":{
            "value":{"kind":"boolean"},"targets":["scenario_enemy_actor"]}}}}])
    );
    let rules: Vec<ConfigurationDispositionRule> =
        serde_json::from_value(accounting.clone()).unwrap();
    assert_eq!(
        value(rules),
        json!([{"id":"checked-critical-weakness-activation-input",
        "selector":{"lane":"input_boolean","name":SOURCE},
        "destinations":[{"kind":"adapter","control":{"kind":"scalar_input","input":id("external_input",INPUT)}}]}])
    );
}
pub fn check_consumer(candidate: &Value) {
    let actual: Consumer = serde_json::from_value(candidate.clone()).unwrap();
    let expected: Consumer = serde_json::from_value(json!({
        "program":{"id":PROGRAM,"context":"actor",
            "reads":[{"id":"configured","value_type":{"kind":"boolean"},"source":{"kind":"external","value":{"entity":"current","input":id("external_input",INPUT)}}}],
            "nodes":[{"id":"configured","expression":{"kind":"read","input":"configured"}},
                {"id":"true","expression":{"kind":"literal","value":{"kind":"boolean","value":true}}}],
            "effects":[{"id":"configured-activation","when":"configured","effect":{"kind":"contribute","entity":"current","stat":id("stat",STAT),"contribution":"flag","value":"true"}}]},
        "query":QUERY,"group":"sources",
        "member":{"producer":{"kind":"program_effect","owner":{"kind":"definition","value":{"kind":"encounter","value":id("encounter","def.00000000000031d1")}},
            "program":PROGRAM,"effect":"configured-activation","origin":{"kind":"encounter"}},"order":null}
    })).unwrap();
    assert_eq!(
        actual, expected,
        "exact configured-state producer and recipient"
    );
}
const ARCHIVES: &[&str] = &[
    "src/Modules/ConfigOptions.lua",
    "src/Classes/ConfigTab.lua",
    "src/Modules/CalcPerform.lua",
];
const SPANS: &[(&str, usize, usize, &str)] = &[
    (
        "src/Modules/ConfigOptions.lua",
        1908,
        1913,
        "separate-enemy-activation-and-stack-callbacks",
    ),
    (
        "src/Classes/ConfigTab.lua",
        712,
        732,
        "checkbox-constructor-default",
    ),
    ("src/Classes/ConfigTab.lua", 883, 906, "saved-input-loading"),
    (
        "src/Classes/ConfigTab.lua",
        1169,
        1199,
        "fresh-lists-and-checkbox-dispatch",
    ),
    (
        "src/Classes/ConfigTab.lua",
        1317,
        1334,
        "fresh-config-constructor",
    ),
    (
        "src/Modules/CalcPerform.lua",
        1193,
        1197,
        "player-and-enemy-destination-bindings",
    ),
    (
        "src/Modules/CalcPerform.lua",
        2562,
        2568,
        "capability-is-not-enemy-condition",
    ),
];
pub fn source_outputs(configured: Option<bool>) -> Option<Value> {
    configured.map(|enabled| {
        if enabled {
            json!([{
                "name":"Condition:ApplyCriticalWeakness","type":"FLAG","value":true,
                "source":"Config","flags":0,"keywordFlags":0,
                "_positions":[{"index":1,"value":{"type":"Condition","var":"Effective"}}]
            }])
        } else {
            json!([])
        }
    })
}
pub fn authenticate_source(proof: &Value) {
    sources::authenticate(proof, ARCHIVES, SPANS);
    assert_eq!(
        proof["kind"],
        "configured-enemy-critical-weakness-activation"
    );
    assert_eq!(proof["schema_version"], 1);
    assert_eq!(
        proof["scope"],
        json!({"target":"scenario_enemy_actor","configured_snapshot":true,
        "constructor":false,"true_callback_emits_condition":true,"false_callback_emits_no_record":true,
        "missing_or_malformed_proof_is_false":false,"capability_implies_activation":false,
        "participation_implies_hit":false,"global_condition_absence":false,"coverage_completed":false,"new_source_execution":false})
    );
    assert_eq!(
        proof["derivation"],
        json!({
            "source_scope":"this-config-callback-only","true":source_outputs(Some(true)).unwrap(),"false":[],
            "absent_saved_control":"reviewed-fresh-checkbox-default-false","malformed_saved_control":"pending-no-input",
            "global_condition_false_inferred":false,"effective_tag":"source-view applicability, not a native UI-mode flag",
            "sand_djinn":"Capability and Command classification do not establish a past hit or current Enemy stacks; raw on-hit application remains separately untranslated."
        })
    );
}
fn project(report: &Value) -> Value {
    let state = &report["state"];
    json!({
        "catalogue":state["configuration_catalogue"].as_array().unwrap().iter().filter(|r| r["fields"]["var"] == SOURCE).collect::<Vec<_>>(),
        "constructors":state["configuration"].as_array().unwrap().iter().map(|r| json!({
            "input":r["input"].get(SOURCE),"placeholder":r["placeholder"].get(SOURCE),"default_state":r["default_state"].get(SOURCE)
        })).collect::<Vec<_>>(),
        "calls":state["callback_invocations"].as_array().unwrap().iter().filter(|r| r["var"] == SOURCE).collect::<Vec<_>>()
    })
}
pub fn observation() -> Value {
    project(&sources::acquisition().jit_off)
}
pub fn check_observation(observed: &Value) {
    assert_eq!(
        observed["catalogue"],
        json!([{
            "apply":{"execution_not_implied":true,"first_line":1908,"last_line":1910,"kind":"callback_reference",
                "path":"@C:/code/poe-optimizer/vendor/path-of-building-poe2/src/Modules/ConfigOptions.lua","what":"Lua"},
            "fields":{"ifFlag":"ApplyCriticalWeakness","label":"Is the enemy Critically Weak",
                "tooltip":"Hits against Critically Weak enemies have up to +10% Critical Strike Chance.\nThis option will also allow you to input the effect of Critical Weakness.",
                "type":"check","var":SOURCE},"index":593
        }])
    );
    assert_eq!(
        observed["constructors"],
        json!([
            {"input":null,"placeholder":null,"default_state":false},
            {"input":null,"placeholder":null,"default_state":false}
        ])
    );
    assert_eq!(
        observed["calls"],
        json!([]),
        "absent checkbox invokes no callback"
    );
}
/// Require the complete reviewed artifact set. A missing or duplicated pin is
/// not an authenticated subset, even if every remaining digest is correct.
pub fn check_authoring(authoring: &Value) {
    let artifacts: Vec<_> = FILES
        .iter()
        .map(|file| {
            let path = directory().join(file);
            let size = fs::metadata(&path).unwrap().len();
            assert!(size <= 2 * 1024 * 1024);
            let bytes = fs::read(path).unwrap();
            json!({"file":file,"bytes":size,"sha256":format!("{:x}",Sha256::digest(bytes))})
        })
        .collect();
    assert_eq!(
        authoring,
        &json!({
            "schema_version":1,"kind":"configured-enemy-critical-weakness-activation",
            "allocated_input":id("external_input",INPUT),"condition_stat":id("stat",STAT),
            "native_programs_added":1,"predecessor_status":"published-outgoing-critical",
            "publication_ready":true,"preserve_owner_and_query_closure":true,
            "constructor_absence_is_configured_false_only":true,"source_vm_runs":0,
            "before":BEFORE,"artifacts":artifacts
        }),
        "exact publication scope and seven artifact commitments"
    );
}

fn check_partial_queries(release: &StagedOwnedRelease) {
    let queries = &release
        .input()
        .recipe
        .rules
        .contribution_queries
        .as_ref()
        .unwrap()
        .members;
    let selected: Vec<_> = queries
        .iter()
        .filter(|q| q.id.as_str().starts_with("outgoing-critical-"))
        .collect();
    assert_eq!(selected.len(), QUERIES.len());
    for name in QUERIES {
        let matching: Vec<_> = selected.iter().filter(|q| q.id.as_str() == name).collect();
        assert_eq!(matching.len(), 1, "one exact critical query: {name}");
        assert_eq!(matching[0].groups.len(), 1);
        assert_eq!(
            value(&matching[0].groups[0].members.closure)["kind"],
            "partial"
        );
    }
}

pub fn check_authored() {
    check_policy(
        &read("input.json"),
        &read("definitions.json"),
        &read("accounting.json"),
    );
    check_consumer(&read("consumer.json"));
    check_authoring(&read("authoring.json"));
    let dependencies: Value = read("dependencies.json");
    assert_eq!(dependencies["before"], BEFORE);
    assert_eq!(dependencies["registry_last_issued"], 0x33b0);
    let extension: poe_optimizer_import::owned_recipe_extension::OwnedRecipeExtension =
        read("extension.json");
    assert!(
        extension.owners.is_empty()
            && extension.tables.is_empty()
            && extension.receivers.is_empty()
            && extension.support_source_domains.is_empty()
    );
    assert!(extension.operations_version.is_none());
    assert_eq!(extension.schema.len(), 2);
    let mut encounter = dependencies["encounter"].clone();
    encounter["value"]["schema"]["value"]["external_inputs"]["members"]
        .as_array_mut()
        .unwrap()
        .push(id("external_input", INPUT));
    assert_eq!(value(&extension.schema[0])["value"], encounter);
    assert_eq!(
        value(&extension.schema[1])["value"],
        read::<Value>("definitions.json")[0]
    );
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        authenticate_source(&read("source-evidence.json"));
        let acquisition = sources::acquisition();
        for report in [&acquisition.jit_off, &acquisition.jit_on] {
            check_observation(&project(report));
        }
    });
}

/// The input-only common stage retains its exact zero-program inverse. The
/// second stage appends precisely one potential flag member, preserving every
/// existing owner and query closure, including the unresolved source domain.
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let authoring: Value = read("authoring.json");
    assert_eq!(authoring["publication_ready"], true);
    let dependencies: Value = read("dependencies.json");
    assert_eq!(value(prior.receipt().input), dependencies["before"]);
    assert_eq!(
        value(&prior.receipt().artifacts),
        dependencies["predecessor_artifacts"]
    );
    assert_eq!(prior.receipt().artifacts.len(), 17);
    let receipt = prior
        .artifacts()
        .find(|(file, _)| *file == "release.json")
        .unwrap()
        .1;
    assert_eq!(
        dependencies["release_authority"],
        json!({
            "file":"runs/owned-outgoing-critical-component-publication-02/package/release.json",
            "sha256":format!("{:x}",Sha256::digest(receipt))
        })
    );
    check_partial_queries(prior);
    let configured = common::stage(prior, &directory(), "configured-critical-weakness-input", 1);
    let c = consumer();
    let mut input = configured.input().clone();
    let owner = c.member.producer.as_program_effect().unwrap().owner.clone();
    let owner_rows: Vec<_> = input
        .recipe
        .rules
        .owners
        .iter()
        .enumerate()
        .filter(|(_, o)| o.owner == owner)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(owner_rows.len(), 1);
    let owner_at = owner_rows[0];
    let programs = &mut input.recipe.rules.owners[owner_at].programs;
    assert_eq!(value(&programs.closure)["kind"], "partial");
    assert!(!programs.members.iter().any(|p| p.id == c.program.id));
    programs.members.push(c.program.clone());
    let queries = &mut input
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members;
    let matching: Vec<_> = queries
        .iter()
        .enumerate()
        .filter(|(_, q)| q.id.as_str() == QUERY)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(matching.len(), 1);
    let query_at = matching[0];
    let query = &mut queries[query_at];
    assert_eq!(value(&*query), dependencies["weakness_query"]);
    assert_eq!(query.stat.key().as_str(), STAT);
    assert_eq!(value(query.contribution), "flag");
    assert_eq!(query.groups.len(), 1);
    let group = &mut query.groups[0];
    assert_eq!(group.id.as_str(), c.group);
    assert_eq!(value(group.reduction), "any");
    assert_eq!(value(group.ordering), "unordered");
    assert_eq!(group.empty, Some(ParameterValue::Boolean(false)));
    assert_eq!(value(&group.members.closure)["kind"], "partial");
    assert!(!group.members.members.contains(&c.member));
    group.members.members.push(c.member.clone());
    let provenance = OwnedReleaseProvenance {
        kind: PROGRAM.parse().unwrap(),
        prior_input: configured.receipt().input,
        authoring_input: digest_owned(
            "configured-critical-weakness-activation",
            &authoring,
            2 * 1024 * 1024,
        )
        .unwrap(),
    };
    input.provenance.push(provenance.clone());
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    assert_eq!(
        inverse.recipe.rules.owners[owner_at].programs.members.pop(),
        Some(c.program)
    );
    assert_eq!(
        inverse
            .recipe
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members[query_at]
            .groups[0]
            .members
            .members
            .pop(),
        Some(c.member)
    );
    assert_eq!(inverse.provenance.pop(), Some(provenance));
    assert!(
        inverse == *configured.input(),
        "exact flag/member append inverse"
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    check_partial_queries(&next);
    next
}
