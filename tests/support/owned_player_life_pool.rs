//! Authored Player Life arithmetic; source/ordering coverage stays explicitly open.
use super::migration_preservation;

use poe_optimizer_core::{owned_content::*, owned_rules::*, owned_schema::*};
use poe_optimizer_import::{
    owned_recipe_extension::OwnedRecipeExtension,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf, sync::OnceLock};

const KIND: &str = "player-life-pool";
const FILES: [&str; 6] = [
    "authoring.json",
    "extension.json",
    "consumer.json",
    "dependencies.json",
    "bindings.json",
    "source-vectors.json",
];
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/player-life-pool")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub owners: Vec<DefinitionRules>,
    pub receivers: Vec<StatReceiver>,
    pub queries: Vec<ContributionQuery>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependencies {
    pub before: OwnedContentDigest,
    pub registry_last: i64,
    pub definitions: Vec<DefinitionDescriptor>,
    pub queries: Vec<ContributionQuery>,
    pub donor_owners: Vec<DefinitionRules>,
    pub query_registry_closure: SchemaClosure,
    pub receiver_registry_closure: SchemaClosure,
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn extension() -> OwnedRecipeExtension {
    read("extension.json")
}
pub fn dependencies() -> Dependencies {
    read("dependencies.json")
}

fn check_pin(bytes: &[u8], pin: &Value) {
    assert_eq!(pin["bytes"], bytes.len());
    assert_eq!(pin["sha256"], format!("{:x}", Sha256::digest(bytes)));
}
fn archived(proof: &Value, name: &str, expected_hash: &str) -> Value {
    let pins = proof["archives"].as_array().unwrap();
    let matches: Vec<_> = pins.iter().filter(|p| p["path"] == name).collect();
    assert_eq!(matches.len(), 1);
    let pin = matches[0];
    assert_eq!(pin["raw"]["sha256"], expected_hash);
    let compressed = fs::read(data().join(name)).unwrap();
    check_pin(&compressed, &pin["compressed"]);
    let mut raw = Vec::new();
    flate2::read::GzDecoder::new(compressed.as_slice())
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut raw)
        .unwrap();
    assert!(raw.len() <= 64 * 1024 * 1024);
    check_pin(&raw, &pin["raw"]);
    serde_json::from_slice(&raw).unwrap()
}
fn number(observation: &Value) -> Value {
    assert_eq!(observation["present"], true);
    assert_eq!(observation["kind"], "number");
    assert!(observation["value"].as_f64().unwrap().is_finite());
    observation["value"].clone()
}
fn reconstruct_source() -> Value {
    let proof: Value = read("source-vectors.json");
    assert_eq!(proof["archives"].as_array().unwrap().len(), 4);
    let acquisition = archived(
        &proof,
        "acquisition-jit-off.json.gz",
        "0bd4879b4fa8fd7d9a9f4a2f8bf1a67b12ad94aedb850613cb02e421210c1373",
    );
    let off = archived(
        &proof,
        "reference-jit-off.json.gz",
        "2697e09bb8c351c49357a02bb5db771ff2bc350793a19ba20d01616dd04e1a92",
    );
    let on = archived(
        &proof,
        "reference-jit-on.json.gz",
        "26a268b78539703cd1c31d549722067f3f3ac3585164ba388f2c6f601d189ef7",
    );
    let comparison_pin = proof["archives"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["path"] == "comparison.json.gz")
        .unwrap();
    let comparison = archived(
        &proof,
        "comparison.json.gz",
        comparison_pin["raw"]["sha256"].as_str().unwrap(),
    );
    for field in [
        "acquisition_vs_plain_off_projection_agreement",
        "exact_acquisition_repeat_agreement",
        "exact_reference_projection_agreement",
    ] {
        assert_eq!(comparison[field], true);
    }
    assert_eq!(comparison["case_count"], 11);
    assert_eq!(comparison["complete_loads"], 66);
    let manifest =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    check_pin(&manifest, &proof["source_manifest"]);
    assert_eq!(
        acquisition["metadata"]["manifest_sha256"],
        proof["source_manifest"]["sha256"]
    );
    assert_eq!(acquisition["metadata"], proof["source_metadata"]);
    let formula = &proof["formula_source"];
    assert_eq!(formula["path"], "src/Modules/CalcDefence.lua");
    assert_eq!(
        formula["file"],
        "data/owned/poe2/3887ae68/configuration-enemy-speed-inputs/CalcDefence.lua.gz"
    );
    assert_eq!(
        formula["sha256"],
        "b0f498e93dd69ac09deea875dfc186345e26a977419a6b5cec1cf9b04e1f46d2"
    );
    let compressed = fs::read(root().join(formula["file"].as_str().unwrap())).unwrap();
    assert_eq!(
        formula["artifact_sha256"],
        format!("{:x}", Sha256::digest(&compressed))
    );
    let mut raw = Vec::new();
    flate2::read::GzDecoder::new(compressed.as_slice())
        .take(1024 * 1024 + 1)
        .read_to_end(&mut raw)
        .unwrap();
    assert!(raw.len() <= 1024 * 1024);
    check_pin(&raw, formula);
    let raw = String::from_utf8(raw).unwrap();
    assert_eq!(
        (
            formula["first_line"].as_u64(),
            formula["last_line"].as_u64()
        ),
        (Some(74), Some(130))
    );
    let body = raw.lines().skip(73).take(57).collect::<Vec<_>>().join("\n") + "\n";
    assert_eq!(formula["body"], body);
    assert_eq!(acquisition["jit_enabled"], false);
    assert_eq!(off["jit_enabled"], false);
    assert_eq!(on["jit_enabled"], true);
    let cases = acquisition["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 11);
    assert_eq!(off["cases"].as_array().unwrap().len(), 11);
    assert_eq!(on["cases"].as_array().unwrap().len(), 11);
    let mut vectors = vec![];
    for (index, case) in cases.iter().enumerate() {
        assert!(
            case["first"] == case["repeat"],
            "exact repeated acquisition"
        );
        for reference in [&off, &on] {
            let c = &reference["cases"][index];
            assert_eq!(c["name"], case["name"]);
            assert!(
                c["first"] == c["repeat"],
                "exact repeated unhooked reference"
            );
            assert!(
                c["first"]["state"]["snapshot"] == case["first"]["state"]["snapshot"],
                "all retained scalar/store projections agree"
            );
            assert_eq!(c["xml_sha256"], case["xml_sha256"]);
        }
        let rows = case["first"]["state"]["invocations"].as_array().unwrap();
        assert_eq!(rows.len(), 72);
        let mut projected = vec![];
        for row in rows {
            for field in [
                "exact_actor_store",
                "exact_existing_player",
                "consumer_checkpoint_reached",
            ] {
                assert_eq!(row[field], true);
            }
            let inputs = &row["computation"]["inputs"];
            let mut values = serde_json::Map::new();
            for name in ["base", "increase", "more", "conversion", "extra", "total"] {
                assert_eq!(inputs[name]["assignment_reached"], true);
                values.insert(name.into(), number(&inputs[name]["value"]));
            }
            let raw_override = &inputs["override"]["value"];
            let mut override_value = json!({"present":raw_override["present"]});
            if raw_override["present"] == true {
                override_value["value"] = number(raw_override);
            }
            values.insert("override".into(), override_value);
            let ci = &inputs["chaos_inoculation"]["value"];
            assert_eq!(inputs["chaos_inoculation"]["assignment_reached"], true);
            if ci["present"] == true {
                assert_eq!(ci["kind"], "boolean");
                assert!(ci["value"].is_boolean());
            } else {
                assert_eq!(ci["present"], false);
                assert_eq!(ci["kind"], "nil");
            }
            values.insert(
                "chaos_inoculation".into(),
                if ci["present"] == true {
                    ci["value"].clone()
                } else {
                    json!(false)
                },
            );
            assert_eq!(row["rounding"]["exact_consumer_frame"], true);
            assert_eq!(row["rounding"]["result_observed"], false);
            projected.push(
                json!({"inputs":values,"operand":number(&row["rounding"]["argument"]),
                "checkpoint_life":row["computation"]["life"],"final_life":row["return_life"]}),
            );
        }
        assert!(
            projected.iter().all(|p| *p == projected[0]),
            "all72 original calls agree"
        );
        let mut vector = projected.remove(0);
        vector["name"] = case["name"].clone();
        vector["synthetic_source_control"] = case["synthetic_source_control"].clone();
        vector["xml_sha256"] = case["xml_sha256"].clone();
        vector["observed_invocations_per_acquisition"] = json!(72);
        vectors.push(vector);
    }
    assert_eq!(proof["vectors"], json!(vectors));
    proof
}
pub fn authenticate_source_vectors(proof: &Value) {
    static PROOF: OnceLock<Value> = OnceLock::new();
    assert_eq!(
        proof["scope"],
        json!({"aggregate_formula_operands":true,"native_supplier_proof":false,
        "positive_transform_source_proof":false,"override_source_proof":false,"internal_cross_jit_proof":false})
    );
    assert!(
        proof == PROOF.get_or_init(reconstruct_source),
        "exact source proof identity, metadata and vectors"
    );
}
pub fn source_vectors() -> Value {
    let proof: Value = read("source-vectors.json");
    authenticate_source_vectors(&proof);
    proof
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let d = dependencies();
    assert_eq!(a["kind"], KIND);
    assert_eq!(a["before"], json!(d.before));
    assert_eq!(
        a["scope"],
        json!({"new_definitions":6,"new_programs":1,"new_receivers":1,"new_queries":7,
        "closed_existing_owners":0,"supplier_admission":false,"complete_life":false,"evaluation_bundle":false,"whole_build":false})
    );
    for file in FILES.iter().skip(1) {
        check_pin(&fs::read(data().join(file)).unwrap(), &a["artifacts"][file]);
    }
    source_vectors();
    let e = extension();
    assert_eq!(e.schema.len(), 6);
    assert!(e.owners.is_empty() && e.receivers.is_empty() && e.tables.is_empty());
    assert_eq!(d.registry_last, 0x3382);
    assert_eq!(d.queries.len(), 3);
    assert_eq!(d.donor_owners.len(), 5);
    let c = consumer();
    assert_eq!(
        (c.owners.len(), c.receivers.len(), c.queries.len()),
        (1, 1, 7)
    );
    for (owner, receiver) in c.owners.iter().zip(&c.receivers) {
        assert_eq!(
            owner.owner,
            SchemaSubject::Definition(receiver.stat.address())
        );
        assert!(!owner.programs.is_complete());
        assert_eq!(owner.programs.members.len(), 1);
        let p = &owner.programs.members[0];
        assert_eq!(p.id, receiver.program);
        assert_eq!(p.context, RuleEntityKind::Actor);
        assert_eq!(receiver.targets, [StatReceiverTarget::Player]);
        assert_eq!(p.effects.len(), 1);
        assert!(
            matches!(&p.effects[0].effect,RuleEffectKind::Derive{entity:RuleEntity::Current,stat,..} if *stat==receiver.stat)
        );
    }
    for q in &c.queries {
        assert_eq!(q.groups.len(), 1);
        let g = &q.groups[0];
        assert!(!g.members.is_complete() && g.members.members.is_empty());
        match q.contribution {
            ContributionKind::Add => {
                assert_eq!(g.reduction, ContributionReduction::Sum);
                assert_eq!(g.ordering, ContributionOrdering::Ordered);
            }
            ContributionKind::Flag => {
                assert_eq!(g.reduction, ContributionReduction::Any);
                assert_eq!(g.ordering, ContributionOrdering::Unordered);
                assert_eq!(
                    g.empty,
                    Some(poe_optimizer_core::owned_build::ParameterValue::Boolean(
                        false
                    ))
                );
            }
            ContributionKind::Override => {
                assert_eq!(g.reduction, ContributionReduction::RequireAgreement);
                assert!(g.empty.is_none());
            }
            _ => panic!("unexpected contribution kind"),
        }
    }
}

/// Authenticate this component wherever it appears in a later checked release.
/// Unrelated schema/input additions need no historical whole-release rewind.
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    assert_component_with_reviewed_rules(endpoint, &endpoint.input().recipe.rules);
}
/// Caller authenticates its exact later component inverse before comparison.
pub fn assert_component_with_reviewed_rules(
    endpoint: &StagedOwnedRelease,
    rules: &RulePackageInput,
) {
    use poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry;
    check_authored();
    let recipe = &endpoint.input().recipe;
    let d = dependencies();
    for entry in extension().schema {
        let SchemaExtensionEntry::Definition(definition) = entry else {
            panic!("only six Stat definitions belong to this component")
        };
        assert!(recipe.schema.definitions.contains(&definition));
    }
    let c = consumer();
    for owner in c.owners.iter().chain(&d.donor_owners) {
        let matches: Vec<_> = rules
            .owners
            .iter()
            .filter(|o| o.owner == owner.owner)
            .collect();
        assert_eq!(matches.len(), 1);
        assert!(
            *matches[0] == *owner,
            "exact Life owner, including its open coverage"
        );
    }
    for receiver in &c.receivers {
        let matches: Vec<_> = recipe
            .rules
            .receivers
            .members
            .iter()
            .filter(|r| r.id == receiver.id)
            .collect();
        assert_eq!(matches, vec![receiver]);
    }
    let registry = rules.contribution_queries.as_ref().unwrap();
    for query in c.queries.iter().chain(&d.queries) {
        let matches: Vec<_> = registry
            .members
            .iter()
            .filter(|q| q.id == query.id)
            .collect();
        assert_eq!(matches, vec![query]);
    }
    assert_eq!(registry.closure, d.query_registry_closure);
    assert_eq!(recipe.rules.receivers.closure, d.receiver_registry_closure);
    let expected = OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: d.before,
        authoring_input: digest_owned(KIND, &FILES.map(read::<Value>), 1024 * 1024).unwrap(),
    };
    assert!(endpoint.input().provenance.contains(&expected));
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    let d = dependencies();
    assert_eq!(prior.receipt().input, d.before);
    let old = &prior.input().recipe;
    assert_eq!(old.registry.last_issued.get(), d.registry_last);
    for definition in &d.definitions {
        assert!(old.schema.definitions.contains(definition));
    }
    for owner in &d.donor_owners {
        assert!(old.rules.owners.contains(owner));
    }
    let queries = old.rules.contribution_queries.as_ref().unwrap();
    for query in &d.queries {
        assert!(queries.members.contains(query));
    }
    assert_eq!(queries.closure, d.query_registry_closure);
    assert_eq!(old.rules.receivers.closure, d.receiver_registry_closure);
    assert_eq!(
        a["contract"],
        json!({"schema_version":old.schema.schema_version,
        "schema_semantics_version":old.schema.semantics_version,"operations_version":old.rules.operations_version,
        "rule_semantics_version":old.rules.semantics_version})
    );
    let migrated = compile_owned_release_migration(
        prior,
        OwnedReleaseMigrationInput {
            schema_version: 5,
            before: d.before,
            release: serde_json::from_value(a["release"].clone()).unwrap(),
            reason: KIND.parse().unwrap(),
            contract: serde_json::from_value(a["contract"].clone()).unwrap(),
            schema: extension().schema,
            tables: vec![],
            owners: vec![],
            receivers: vec![],
            query_targets: vec![],
            evaluation: None,
        },
        Default::default(),
    )
    .unwrap();
    let c = consumer();
    let mut input = migrated.input().clone();
    for owner in &c.owners {
        assert!(
            !input
                .recipe
                .rules
                .owners
                .iter()
                .any(|o| o.owner == owner.owner)
        );
    }
    input.recipe.rules.owners.extend(c.owners);
    input.recipe.rules.receivers.members.extend(c.receivers);
    input
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(c.queries);
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(KIND, &FILES.map(read::<Value>), 1024 * 1024).unwrap(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    let c = consumer();
    for owner in c.owners {
        let rows = &mut inverse.recipe.rules.owners;
        let at = rows.iter().position(|o| o.owner == owner.owner).unwrap();
        assert!(rows.remove(at) == owner);
    }
    for receiver in c.receivers {
        let rows = &mut inverse.recipe.rules.receivers.members;
        let at = rows.iter().position(|r| r.id == receiver.id).unwrap();
        assert_eq!(rows.remove(at), receiver);
    }
    for query in c.queries {
        let rows = &mut inverse
            .recipe
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members;
        let at = rows.iter().position(|q| q.id == query.id).unwrap();
        assert_eq!(rows.remove(at), query);
    }
    inverse.provenance = migrated.input().provenance.clone();
    assert!(
        inverse == *migrated.input(),
        "only exact authored receivers/queries follow six definitions"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert!(next.evaluation().is_none());
    assert_component(&next);
    next
}
