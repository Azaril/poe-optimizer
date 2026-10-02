//! Checked addition of partial passive producers; reference outputs are never inputs.
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::{
        OwnedDefinitionKey, PassiveNodeDefId, PointPoolDefId, StatDefId, UnitDefId,
    },
    owned_rules::{
        ContributionKind, DefinitionRules, RuleEffectKind, RuleEntity, RuleExpression, StatReceiver,
    },
    owned_schema::{DefinitionDescriptor, SchemaDefinitionId, SchemaSubject},
};
use poe_optimizer_import::{
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_tree_policy::TreeTokenRole,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeBinding {
    pub node: PassiveNodeDefId,
    pub source_id: String,
    pub value: f64,
    pub pool: PointPoolDefId,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bindings {
    pub player_stat: StatDefId,
    pub actor_stat: StatDefId,
    pub unit: UnitDefId,
    pub nodes: Vec<NodeBinding>,
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn read<T: DeserializeOwned>(file: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/plain-minion-damage-passives")
                .join(file),
        )
        .unwrap(),
    )
    .unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(
            value.as_object().is_some_and(|o| o.is_empty()),
            "source list: {value}"
        );
        &[]
    }
}
fn damage(record: &Value) -> f64 {
    let matches: Vec<_> = rows(&record["modifiers"])
        .iter()
        .filter(|m| {
            m["name"] == "MinionModifier"
                && m["type"] == "LIST"
                && m["value"]["mod"]["name"] == "Damage"
                && m["value"]["mod"]["type"] == "INC"
        })
        .collect();
    assert_eq!(matches.len(), 1);
    let outer = matches[0];
    assert_eq!(outer["flags"], 0);
    assert_eq!(outer["keyword_flags"], 0);
    assert!(rows(&outer["tags"]).is_empty());
    let inner = &outer["value"]["mod"];
    assert_eq!(inner["flags"], 0);
    assert_eq!(inner["keywordFlags"], 0);
    assert!(inner.as_object().unwrap().keys().all(|k| matches!(
        k.as_str(),
        "name" | "type" | "value" | "flags" | "keywordFlags" | "source"
    )));
    assert_eq!(
        inner["source"],
        format!("Tree:{}", record["id"].as_u64().unwrap())
    );
    let value = inner["value"].as_f64().unwrap();
    assert!(value.is_finite() && value > 0.0);
    value
}
pub fn check_authored() {
    let extension: OwnedRecipeExtension = read("extension.json");
    let bindings: Bindings = read("bindings.json");
    let records: Vec<Value> = read("source-records.json");
    let _: Vec<DefinitionDescriptor> = read("dependencies.json");
    let _: Vec<DefinitionRules> = read("dependency-rules.json");
    let receivers: Vec<StatReceiver> = read("receivers.json");
    assert!(
        extension.schema.is_empty()
            && extension.tables.is_empty()
            && extension.receivers.is_empty()
    );
    assert!(extension.operations_version.is_none());
    assert_eq!(extension.owners.len(), 41);
    assert_eq!(bindings.nodes.len(), 41);
    assert_eq!(records.len(), 41);
    assert_eq!(receivers.len(), 2);
    assert_ne!(bindings.player_stat, bindings.actor_stat);
    let mut seen = BTreeSet::new();
    let mut source_ids = BTreeSet::new();
    for binding in &bindings.nodes {
        assert!(seen.insert(binding.node.clone()));
        assert!(source_ids.insert(binding.source_id.clone()));
        let record: Vec<_> = records
            .iter()
            .filter(|r| r["id"].as_u64().unwrap().to_string() == binding.source_id)
            .collect();
        assert_eq!(record.len(), 1);
        assert_eq!(damage(record[0]), binding.value);
        let owners: Vec<_> = extension
            .owners
            .iter()
            .filter(|o| o.owner == SchemaSubject::Definition(binding.node.address()))
            .collect();
        assert_eq!(owners.len(), 1);
        let owner = owners[0];
        assert!(!owner.programs.is_complete());
        assert_eq!(owner.programs.members.len(), 1);
        let program = &owner.programs.members[0];
        assert_eq!(json!(program.context), "actor");
        assert!(program.reads.is_empty());
        assert_eq!(program.nodes.len(), 1);
        assert_eq!(program.effects.len(), 1);
        let RuleExpression::Literal { value } = &program.nodes[0].expression else {
            panic!("reviewed constant passive modifier")
        };
        assert_eq!(
            json!(value),
            json!({"kind":"quantity","value":{"value":binding.value,"unit":bindings.unit}})
        );
        assert!(program.effects[0].when.is_none());
        let RuleEffectKind::Contribute {
            entity,
            stat,
            contribution,
            value,
        } = &program.effects[0].effect
        else {
            panic!("passive contribution")
        };
        assert_eq!(*entity, RuleEntity::Player);
        assert_eq!(*contribution, ContributionKind::Increase);
        assert_eq!(stat, &bindings.player_stat);
        assert_eq!(value, &program.nodes[0].id);
    }
}
fn source_proof(authoring: &Value) {
    let proof = &authoring["source_validation"];
    assert_eq!(proof["status"], "passed");
    assert_eq!(proof["evidence_bytes"], 19_612_949_u64);
    assert_eq!(
        proof["evidence_sha256"],
        "030f14d71a01a2c54862eb858249b2abca1ba1caa9337dbffb3458118777e935"
    );
    let off = fs::read(root().join(proof["evidence_json"].as_str().unwrap())).unwrap();
    let on = fs::read(root().join(proof["evidence_on_json"].as_str().unwrap())).unwrap();
    assert!(off == on, "both original JIT modes agree");
    assert_eq!(off.len() as u64, proof["evidence_bytes"].as_u64().unwrap());
    assert_eq!(
        format!("{:x}", Sha256::digest(&off)),
        proof["evidence_sha256"]
    );
    let evidence: Value = serde_json::from_slice(&off).unwrap();
    assert_eq!(evidence["source_revision"], authoring["source_revision"]);
    assert_eq!(evidence["source_hash"], authoring["source_manifest_sha256"]);
    for pin in authoring["source_files"].as_array().unwrap() {
        assert!(
            rows(&evidence["evidence"]["files"])
                .iter()
                .any(|r| r["path"] == pin["path"] && r["sha256"] == pin["sha256"])
        );
    }
    assert_eq!(
        rows(&evidence["cases"]).len() as u64,
        proof["cases_per_jit"].as_u64().unwrap()
    );
    let originals: Vec<_> = rows(&evidence["cases"])
        .iter()
        .filter(|r| r["name"] == "original-05")
        .collect();
    assert_eq!(originals.len(), 1);
    let original = originals[0];
    assert_eq!(original["available"], true);
    let observed = rows(&original["state"]["plain_minion_damage_family"]);
    let records: Vec<Value> = read("source-records.json");
    assert_eq!(observed.len(), records.len());
    for record in &records {
        let matching: Vec<_> = observed
            .iter()
            .filter(|r| r["id"] == record["id"])
            .collect();
        assert_eq!(
            matching,
            vec![record],
            "exact source record, independent of authoring order"
        );
    }
    for field in [
        "cached_outputs_preserved",
        "loaded_state_preserved",
        "original_functions_preserved",
        "query_state_preserved",
        "saved_specs_preserved",
    ] {
        assert_eq!(original["state"][field], true, "{field}");
    }
    let actors: Vec<_> = rows(&original["state"]["main"]["actors"])
        .iter()
        .filter(|r| r["summon_effect_id"] == "SummonSkeletalSnipersPlayer")
        .collect();
    assert_eq!(actors.len(), 1);
    let skills: Vec<_> = rows(&actors[0]["children"])
        .iter()
        .filter(|r| r["effect_id"] == "MinionMeleeBow")
        .collect();
    assert_eq!(skills.len(), 1);
    let calls: Vec<_> = rows(&skills[0]["damage_calls"])
        .iter()
        .filter(|r| r["damage_type"] == "Physical" && r["critical"] == false)
        .collect();
    assert_eq!(calls.len(), 1);
    assert_eq!(rows(&calls[0]["increased_records"]).len(), 11);
    assert_eq!(
        rows(&calls[0]["increased_records"])
            .iter()
            .filter(|r| r["mod"]["source"].as_str().unwrap().starts_with("Tree:"))
            .count(),
        10
    );
    let actual: BTreeMap<_, _> = rows(&calls[0]["increased_records"])
        .iter()
        .filter_map(|r| {
            r["mod"]["source"]
                .as_str()
                .unwrap()
                .strip_prefix("Tree:")
                .map(|id| (id.to_owned(), r["value"].as_f64().unwrap()))
        })
        .collect();
    let expected = selected_values();
    assert_eq!(
        actual, expected,
        "exact filtered source contribution membership"
    );
    assert_eq!(actual.len(), 10);
    assert_eq!(actual.values().sum::<f64>(), 68.0);
}
pub fn selected_values() -> BTreeMap<String, f64> {
    read::<Vec<Value>>("source-records.json")
        .iter()
        .filter(|r| r["allocated"] == true)
        .map(|r| (r["id"].as_u64().unwrap().to_string(), damage(r)))
        .collect()
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let authoring: Value = read("authoring.json");
    assert_eq!(authoring["before"], json!(prior.receipt().input));
    source_proof(&authoring);
    assert_eq!(
        authoring["catalog_source"],
        json!(prior.tree().unwrap().input().content.source)
    );
    for definition in read::<Vec<DefinitionDescriptor>>("dependencies.json") {
        assert!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .contains(&definition),
            "exact dependency descriptor"
        );
    }
    for owner in read::<Vec<DefinitionRules>>("dependency-rules.json") {
        assert!(
            prior.input().recipe.rules.owners.contains(&owner),
            "existing channel producer remains exact"
        );
    }
    for receiver in read::<Vec<StatReceiver>>("receivers.json") {
        assert!(
            prior
                .input()
                .recipe
                .rules
                .receivers
                .members
                .contains(&receiver),
            "existing receiving route remains exact"
        );
    }
    let bindings: Bindings = read("bindings.json");
    for binding in &bindings.nodes {
        let token = prior
            .tree()
            .unwrap()
            .input()
            .content
            .tokens
            .iter()
            .find(|t| t.token == binding.source_id)
            .unwrap();
        let TreeTokenRole::Allocation { node, pool, .. } = &token.role else {
            panic!("exact source node allocation mapping")
        };
        assert_eq!(node, &binding.node);
        assert_eq!(pool, &binding.pool);
    }
    let extension: OwnedRecipeExtension = read("extension.json");
    let extended = extend_owned_recipe(prior.assembled(), &extension, Default::default()).unwrap();
    assert!(extended.refinement.is_none());
    assert_eq!(extended.receipt.allocated_entries, 0);
    assert_eq!(extended.receipt.refined_subjects, 0);
    assert_eq!(extended.receipt.appended_tables, 0);
    assert_eq!(extended.receipt.appended_receivers, 0);
    assert_eq!(extended.receipt.appended_programs, 41);
    assert_eq!(
        extended.receipt.before_definitions,
        extended.receipt.after_definitions
    );
    let mut input = prior.input().clone();
    input.recipe = extended.successor;
    input.provenance.push(OwnedReleaseProvenance {
        kind: key("ordinary-passive-minion-damage"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-plain-minion-damage-publication-v1",
            &(
                authoring,
                &extension,
                read::<Value>("bindings.json"),
                read::<Value>("source-records.json"),
            ),
            16 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    preserve(prior, &next, &extension);
    next
}
fn preserve(
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    extension: &OwnedRecipeExtension,
) {
    let before = prior.input();
    let mut restored = next.input().clone();
    assert_eq!(
        (next.receipt().query_sets, next.receipt().query_rows),
        (5, 110)
    );
    assert_eq!(next.receipt().source, prior.receipt().source);
    for added in &extension.owners {
        let old = before
            .recipe
            .rules
            .owners
            .iter()
            .find(|r| r.owner == added.owner);
        let index = restored
            .recipe
            .rules
            .owners
            .iter()
            .position(|r| r.owner == added.owner)
            .unwrap();
        let actual = &mut restored.recipe.rules.owners[index];
        assert_eq!(actual.programs.closure, added.programs.closure);
        for program in &added.programs.members {
            assert!(old.is_none_or(|o| !o.programs.members.iter().any(|p| p.id == program.id)));
            let p = actual
                .programs
                .members
                .iter()
                .position(|p| p.id == program.id)
                .unwrap();
            assert_eq!(actual.programs.members.remove(p), *program);
        }
        if let Some(old) = old {
            assert_eq!(&*actual, old);
        } else {
            assert!(actual.programs.members.is_empty());
            restored.recipe.rules.owners.remove(index);
        }
    }
    assert_eq!(restored.provenance.len(), before.provenance.len() + 1);
    restored.provenance.pop();
    assert!(
        restored == *before,
        "only forty-one exact producer programs and authenticated provenance change"
    );
}
