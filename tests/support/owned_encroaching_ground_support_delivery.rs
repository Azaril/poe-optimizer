//! Exact Encroaching Ground successor; receiving/preparation remain authoring fragments.
#[path = "owned_encroaching_ground_evidence.rs"]
mod evidence;
#[path = "owned_support_delivery_migration.rs"]
mod shared;
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_content::OwnedContentDigest, owned_definitions::*,
    owned_rules::*, owned_schema::*, owned_supports::*,
};
use poe_optimizer_import::{
    owned_release::StagedOwnedRelease, owned_release_migration::OwnedReleaseMigrationInput,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use shared::ReceivingFragment;
use std::{collections::BTreeSet, fs, path::PathBuf};

pub const KIND: &str = "encroaching-ground-support-delivery-v1";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data(name: &str) -> PathBuf {
    root()
        .join("data/owned/poe2/3887ae68/encroaching-ground-support-delivery")
        .join(name)
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data(name)).unwrap()).unwrap()
}
fn decode<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn rows(value: &Value) -> &[Value] {
    value.as_array().expect("authored array")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn subject(gem: &Value) -> Value {
    json!({"kind":"definition","value":{"kind":"gem","value":gem}})
}
fn partial(owner: &Value) -> Value {
    json!({"kind":"partial","value":{"gaps":[{"subject":owner,"facet":"game_rules","code":"remaining-support-receivers-not-converted"}]}})
}
pub fn authoring_digest() -> OwnedContentDigest {
    shared::authoring_digest(&data(""), "owned-encroaching-ground-support-delivery-v1")
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let receiving = json!(read::<ReceivingFragment>("receiving.json"));
    let prep: SupportPreparationInput = read("preparation.json");
    assert_eq!(a["allocated_definitions"], 0);
    assert_eq!(a["new_programs"], 2);
    assert_eq!(a["registry_last_issued_before"], 0x32fc);
    assert_eq!(a["registry_last_issued_after"], 0x32fc);
    assert_eq!(a["before"], b["before"]);
    assert_eq!(a["before"], d["source"]["input"]);
    for field in [
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(a[field], d["source"][field]);
    }
    assert_eq!(b["definitions"], a["definitions"]);
    let scope = json!({"ordinary_cost_contribution_only":true,"final_cost_formula":false,
        "ground_growth_model":false,"reservation_formula":false,"owner_closure":"partial","receiving_fragment_only":true});
    for actual in [&a["scope"], &b["scope"], &v["scope"]] {
        assert_eq!(actual, &scope);
    }
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(json!(m.release), b["release"]);
    assert_eq!((m.schema_version, m.contract.schema_version), (5, 6));
    assert_eq!(
        m.contract.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V20
    );
    assert_eq!((m.schema.len(), m.owners.len()), (0, 1));
    assert!(
        m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(b["channels"].as_object().unwrap().len(), 1);
    assert_eq!(b["channels"]["cost_factor"]["key"], "def.00000000000032fa");
    assert_eq!(b["factor_unit"]["key"], "def.0000000000000001");
    let old: Vec<DefinitionRules> = decode(&d["owners"]);
    assert_eq!(old.len(), 1);
    let supports = rows(&b["supports"]);
    assert_eq!(supports.len(), 1);
    let mut owners = BTreeSet::new();
    for (s, (gem, skill, effect, amount)) in
        supports
            .iter()
            .zip([(0x0740, 0x03f8, "SupportEncroachingGroundPlayer", 1.1)])
    {
        assert_eq!(s["source_mana_multiplier"], 10);
        assert_eq!(s["gem"]["key"], format!("def.{gem:016x}"));
        assert_eq!(s["skill"]["key"], format!("def.{skill:016x}"));
        assert_eq!(s["source_effect"], effect);
        assert_eq!(s["cost_factor"].as_f64(), Some(amount));
        assert_eq!(
            s["programs"],
            json!({"applicability":"encroaching-ground-applicability","delivery":"encroaching-ground-cost"})
        );
        let mapping = json!({"source":{"kind":"definition","value":{"kind":"skill","value":{"effect_id":{"kind":"text","value":effect}}}},
            "outcome":{"kind":"mapped","value":{"target":{"kind":"definition","value":{"kind":"skill","value":s["skill"]}},"basis":{"kind":"exact"}}}});
        assert_eq!(
            rows(&d["mapping_rows"])
                .iter()
                .filter(|r| **r == mapping)
                .count(),
            1
        );
        let gem_rows: Vec<_> = rows(&d["supporting_definitions"])
            .iter()
            .filter(|r| r["kind"] == "gem" && r["value"]["id"] == s["gem"])
            .collect();
        assert_eq!(gem_rows.len(), 1);
        let schema = &gem_rows[0]["value"]["schema"];
        assert_eq!(schema["kind"], "known");
        assert_eq!(schema["value"]["roles"], json!(["support_assignment"]));
        assert_eq!(schema["value"]["skills"]["members"], json!([]));
        assert_eq!(schema["value"]["skills"]["closure"]["kind"], "partial");
        let owner: SchemaSubject = decode(&subject(&s["gem"]));
        assert!(owners.insert(decode::<GemDefId>(&s["gem"])));
        let new = m.owners.iter().find(|r| r.owner == owner).unwrap();
        let prior = old.iter().find(|r| r.owner == owner).unwrap();
        assert!(!new.programs.is_complete());
        assert_eq!(new.programs.closure, prior.programs.closure);
        let mut restored = new.clone();
        for (name, applicability) in [("applicability", true), ("delivery", false)] {
            let id = decode::<OwnedDefinitionKey>(&s["programs"][name]);
            let matches: Vec<_> = new.programs.members.iter().filter(|p| p.id == id).collect();
            assert_eq!(matches.len(), 1);
            let p = matches[0];
            assert_eq!(p.context, RuleEntityKind::Action);
            assert!(p.reads.is_empty());
            assert_eq!((p.nodes.len(), p.effects.len()), (1, 1));
            assert!(p.effects[0].when.is_none());
            let RuleExpression::Literal { value } = &p.nodes[0].expression else {
                panic!("source constant")
            };
            if applicability {
                assert_eq!(value, &ParameterValue::Boolean(true));
                assert_eq!(
                    p.effects[0].effect,
                    RuleEffectKind::SupportApplicability {
                        applicable: p.nodes[0].id.clone()
                    }
                );
            } else {
                assert_eq!(
                    json!(value),
                    json!({"kind":"quantity","value":{"value":amount,"unit":b["factor_unit"]}})
                );
                assert_eq!(
                    p.effects[0].effect,
                    RuleEffectKind::Contribute {
                        entity: RuleEntity::Current,
                        stat: decode(&b["channels"]["cost_factor"]),
                        contribution: ContributionKind::Multiply,
                        value: p.nodes[0].id.clone()
                    }
                );
            }
            restored.programs.members.retain(|p| p.id != id);
        }
        assert_eq!(
            restored, *prior,
            "old prepared-input programs and their full closure are exact"
        );
    }
    check_receiving(&b, &receiving);
    check_preparation(&a, &b, &prep);
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(a["source_manifest_sha256"], hash(&manifest_bytes));
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    let mut pins = BTreeSet::new();
    assert!(!rows(&a["source_files"]).is_empty());
    for pin in rows(&a["source_files"]) {
        assert!(pins.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            rows(&manifest["files"])
                .iter()
                .filter(|r| *r == pin)
                .count(),
            1
        );
    }
    evidence::check(&a, &b, &v, false);
}

fn check_receiving(b: &Value, r: &Value) {
    let ice: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/ice-nova-source-inputs/bindings.json"))
            .unwrap(),
    )
    .unwrap();
    for field in [
        "physical_gem",
        "primary_skill",
        "entering_grant",
        "output",
        "part",
        "mode",
        "stat_sets",
    ] {
        assert_eq!(b["target"][field], ice[field]);
    }
    let owner = json!({"kind":"gem","value":ice["physical_gem"]});
    assert_eq!(
        r["roles"],
        json!([{"id":"encroaching-ground-action","kind":"action"}])
    );
    assert_eq!(
        r["targets"],
        json!([{"owner":owner,"roles":{"members":[{
        "role":"encroaching-ground-action","endpoints":{"members":[{"kind":"action","path":[ice["entering_grant"]],
        "output":ice["output"],"selection":{"kind":"all_declared"},"admission":{"kind":"receiving_skill","summoner_path":null}}],
        "closure":{"kind":"complete"}}}],"closure":partial(&subject(&ice["physical_gem"]))}}])
    );
    let expected: Vec<_> = rows(&b["supports"]).iter().map(|s| json!({"gem":s["gem"],"receivers":{
        "members":[{"role":"encroaching-ground-action","applicability":s["programs"]["applicability"],"delivery":[s["programs"]["delivery"]]}],
        "closure":partial(&subject(&s["gem"]))}})).collect();
    assert_eq!(r["supports"], json!(expected));
}
fn check_preparation(a: &Value, b: &Value, p: &SupportPreparationInput) {
    assert_eq!(p.schema_version, 1);
    assert_eq!(json!(p.definitions), a["definitions"]);
    assert_eq!(json!(p.rules), a["rules"]);
    assert_eq!(json!(p.release), b["release"]);
    assert_eq!(p.quality_unit.key().as_str(), "def.0000000000000002");
    assert_eq!(
        p.policy,
        SupportPreparationPolicy::OrderedReplacementRetryFrontierV1
    );
    assert_eq!(p.supports.len(), 1);
    assert_eq!(p.effects, vec![key("support.encroaching-ground")]);
    assert_eq!(p.families, vec![key("family.encroaching-ground")]);
    for (index, s) in rows(&b["supports"]).iter().enumerate() {
        let rows: Vec<_> = p
            .supports
            .iter()
            .filter(|r| r.gem == decode::<GemDefId>(&s["gem"]))
            .collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].preparation,
            SchemaState::Known(SupportPreparationDefinition {
                effect: p.effects[index].clone(),
                families: Some(p.families.clone()),
                plus_version_of: None,
                requires: Some(SupportTypePredicate::Type(key(
                    "type.creates-ground-effect"
                ))),
                excludes: None,
                added_types: vec![],
                gems_only: false,
                from_item: false,
                is_support: true,
                is_trigger: false,
                ignore_minion_types: false,
            })
        );
    }
}

pub fn check_evidence() {
    evidence::check(
        &read("authoring.json"),
        &read("bindings.json"),
        &read("source-vectors.json"),
        true,
    );
}

pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let provenance = endpoint.input().provenance.last().unwrap();
    assert_eq!(provenance.kind, key(KIND));
    assert_eq!(provenance.prior_input, m.before);
    assert_eq!(provenance.authoring_input, authoring_digest());
    assert_eq!(endpoint.input().recipe.schema.release, m.release);
    assert_eq!(
        endpoint.input().recipe.rules.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V20
    );
    for entry in &m.schema {
        let poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry::Definition(d) =
            entry
        else {
            panic!("Stat definition only")
        };
        assert_eq!(
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|r| *r == d)
                .count(),
            1
        );
    }
    for owner in &m.owners {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|r| *r == owner)
                .count(),
            1
        );
    }
    assert!(endpoint.evaluation().is_none());
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    check_evidence();
    let next = shared::stage(prior, &data(""), KIND, authoring_digest());
    assert_endpoint(&next);
    next
}
