//! Offline reuse of the existing Talisman predicate for ordinary Player Life.
//! Other routing laws, copied records, minion receipt and owner gaps stay open.
use super::migration_preservation;
#[allow(dead_code)]
#[path = "owned_player_offhand_facts.rs"]
mod offhand;
#[path = "owned_ordinary_item_routing_evidence.rs"]
mod routing_evidence;
use poe_optimizer_core::{
    owned_content::digest_owned, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_import::owned_mapping::{
    ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, SourceComponent,
};
use poe_optimizer_import::owned_release::{
    OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const KIND: &str = "flat-life-ordinary-routing";
pub const DIRECT: &str = "contribute-player-flat-life";
pub const APPLICABILITY: &str = "ordinary-item-direct-applicability";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/flat-life-routing")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn source<T: DeserializeOwned>(packet: &str, name: &str) -> T {
    serde_json::from_slice(
        &fs::read(root().join(format!("data/owned/poe2/3887ae68/{packet}/{name}"))).unwrap(),
    )
    .unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Replacement {
    pub before: DefinitionRules,
    pub after: DefinitionRules,
}
pub fn replacements() -> Vec<Replacement> {
    read("owners.json")
}
pub fn check_authored() {
    offhand::check_authored();
    let a: Value = read("authoring.json");
    assert_eq!(a["kind"], KIND);
    assert_eq!(a["schema_version"], 1);
    assert_eq!(
        a["scope"],
        json!({"talisman_direct_guard":true,"new_templates":4,
        "replaced_programs":1,"new_definitions":0,"retired_owner_gaps":0,
        "minion_delivery":false,"copy_delivery":false,"whole_build_parity":false})
    );
    for pin in a["source_packets"].as_array().unwrap() {
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], format!("{:x}", Sha256::digest(bytes)));
    }
    let b: Value = source("ordinary-item-routing", "bindings.json");
    let original: Value = source("ordinary-item-routing", "routing-authoring.json");
    let donor: DefinitionRules = serde_json::from_value(original["owner"].clone()).unwrap();
    let donor = donor
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == b["modifier"]["direct_program"].as_str().unwrap())
        .unwrap();
    let guard_read = donor
        .reads
        .iter()
        .find(|r| r.id == key("direct-applicability"))
        .unwrap();
    let guard_node = donor
        .nodes
        .iter()
        .find(|r| r.id == key("direct-applicability"))
        .unwrap();
    let rows = replacements();
    assert_eq!(rows.len(), 5);
    let life: ModifierDefId = serde_json::from_value(json!({"kind":"modifier",
        "namespace":{"game":"poe2","version":"owned-mechanics-v1"},
        "key":"def.0000000000003100"}))
    .unwrap();
    assert_eq!(
        rows[0].before.owner,
        SchemaSubject::Definition(life.address())
    );
    let mut expected = rows[0].before.clone();
    assert!(!expected.programs.is_complete());
    let p = expected
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key(DIRECT))
        .unwrap();
    assert_eq!(p.context, RuleEntityKind::EquipmentUse);
    assert_eq!((p.reads.len(), p.nodes.len(), p.effects.len()), (1, 5, 1));
    assert!(p.effects[0].when.is_none());
    assert_eq!(p.effects[0].id, key("direct-flat-life"));
    assert!(matches!(&p.effects[0].effect, RuleEffectKind::Contribute {
        entity: RuleEntity::Player, contribution: ContributionKind::Add, stat, value
    } if stat.key().as_str() == "def.000000000000311a" && value == &key("life-points")));
    p.reads.push(guard_read.clone());
    p.nodes.push(guard_node.clone());
    p.effects[0].when = donor.effects[0].when.clone();
    assert_eq!(
        rows[0].after, expected,
        "only ordinary applicability changes"
    );
    let types: Value = offhand::read("bindings.json");
    let templates = a["templates"].as_array().unwrap();
    assert_eq!(templates.len(), 4);
    for (row, binding) in rows[1..].iter().zip(templates) {
        assert_eq!(
            types["templates"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|t| *t == binding)
                .count(),
            1
        );
        assert!(matches!(
            binding["item_type"].as_str(),
            Some("Ring" | "Belt" | "Gloves" | "Body Armour")
        ));
        let template: ItemTemplateDefId =
            serde_json::from_value(binding["template"].clone()).unwrap();
        assert_eq!(
            row.before.owner,
            SchemaSubject::Definition(template.address())
        );
        assert!(!row.before.programs.is_complete());
        let mut expected = row.before.clone();
        assert!(
            !expected
                .programs
                .members
                .iter()
                .any(|p| p.id == key(APPLICABILITY))
        );
        expected.programs.members.push(serde_json::from_value(json!({
            "id":APPLICABILITY,"context":"equipment_use","reads":[],
            "nodes":[{"id":"applicable","expression":{"kind":"literal","value":{"kind":"boolean","value":true}}}],
            "effects":[{"id":"direct-applicability","when":null,"effect":{"kind":"derive",
                "entity":"current","stat":b["channels"]["direct_applicability"],"value":"applicable"}}]
        })).unwrap());
        assert_eq!(
            row.after, expected,
            "one existing narrow predicate per exact template"
        );
    }
    // The source branch diverts every Amulet modifier, independent of its stat.
    // The retained original-call evidence is shared with the existing law.
    let v: Value = source("ordinary-item-routing", "source-vectors.json");
    routing_evidence::check(&v, false);
}
fn digest() -> poe_optimizer_core::owned_content::OwnedContentDigest {
    digest_owned(
        KIND,
        &(
            read::<Value>("authoring.json"),
            read::<Value>("owners.json"),
        ),
        1024 * 1024,
    )
    .unwrap()
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    for row in replacements() {
        let owner = endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == row.after.owner)
            .unwrap();
        assert_eq!(owner, &row.after);
    }
    assert!(
        endpoint
            .receipt()
            .provenance
            .iter()
            .any(|p| p.kind == key(KIND) && p.authoring_input == digest())
    );
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    assert_eq!(json!(prior.receipt().input), a["before"]);
    offhand::verify_source(true);
    routing_evidence::check(
        &source("ordinary-item-routing", "source-vectors.json"),
        true,
    );
    for row in a["templates"].as_array().unwrap() {
        let template: ItemTemplateDefId = serde_json::from_value(row["template"].clone()).unwrap();
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::ItemTemplate {
            base: SourceComponent::Text(row["source_base"].as_str().unwrap().into()),
            prototype: SourceComponent::Missing,
            variant: SourceComponent::Missing,
        });
        assert!(
            matches!(prior.mapping().lookup(&selector), Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::ItemTemplate(found)),
            basis: MappingBasis::Exact,
        }) if *found == template)
        );
    }
    let mut input = prior.input().clone();
    for row in replacements() {
        let owner = input
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == row.before.owner)
            .unwrap();
        assert_eq!(
            owner, &row.before,
            "exact current donor including every other program and gap"
        );
        *owner = row.after;
    }
    input.provenance.push(OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    for row in replacements() {
        *inverse
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == row.after.owner)
            .unwrap() = row.before;
    }
    inverse.provenance.pop();
    assert_eq!(
        inverse,
        *prior.input(),
        "only five exact owner edits and provenance"
    );
    assert_eq!(next.receipt().definitions, prior.receipt().definitions);
    assert_ne!(next.receipt().rules, prior.receipt().rules);
    assert_ne!(
        next.receipt().compiled_rules,
        prior.receipt().compiled_rules
    );
    next
}
