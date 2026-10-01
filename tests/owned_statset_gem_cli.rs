//! Physical primary inputs do not establish action stat-set selection.
#[path = "support/owned_release_fixture.rs"]
mod fixture;
#[path = "support/owned_statset_gem_release.rs"]
mod release;
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::skill_identities::SkillIdentityCatalog;
use poe_optimizer_import::owned_gem_catalog::{GemEffectMembershipPolicy, PhysicalGemSchemaPolicy};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn base() -> PathBuf {
    root().join("data/owned/poe2/3887ae68")
}
fn policy_path() -> PathBuf {
    base().join("active-gem-inputs/statset-primary.json")
}
fn evidence() -> Value {
    read(base().join("active-gem-inputs/statset-publication/evidence.json"))
}
fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(p, serde_json::to_vec(value).unwrap()).unwrap();
}
fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}
fn command(name: &str) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    c.arg(name);
    c
}
fn success(o: Output) -> Value {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}
fn catalog() -> SkillIdentityCatalog {
    SkillIdentityCatalog::new(read(base().join("import/skill-identities.json"))).unwrap()
}

#[test]
fn finite_physical_policy_distinguishes_primary_effects_from_stat_set_aliases() {
    let p: PhysicalGemSchemaPolicy = read(policy_path());
    let c = catalog();
    let proof = evidence();
    assert_eq!(p.source_gems.len(), 12);
    assert!(p.source_gems.windows(2).all(|v| v[0] < v[1]));
    assert_eq!(
        p.effect_membership,
        GemEffectMembershipPolicy::SinglePrimary
    );
    assert_eq!(
        serde_json::to_value(
            digest_owned("owned-physical-gem-schema-policy-v1", &p, 16 * 1024 * 1024).unwrap()
        )
        .unwrap(),
        proof["policy"]
    );
    assert_eq!(
        p.catalog_digest,
        digest_owned("owned-skill-source-catalog-v1", c.data(), 16 * 1024 * 1024).unwrap()
    );
    assert_eq!(c.data().source.upstream_revision, proof["source_revision"]);
    assert_eq!((p.level.minimum.get(), p.level.maximum.get()), (1, 40));
    assert_eq!(p.quality_presence, QualityPresence::Optional);
    assert_eq!(
        p.quality_kinds
            .iter()
            .map(|v| v.key().as_str())
            .collect::<Vec<_>>(),
        ["def.0000000000000006"]
    );
    assert!(p.guards.is_empty());
    assert_eq!(p.parameters.len(), 2);
    assert_eq!(p.parameters[0].schema.value, ValueSchema::Boolean);
    let ValueSchema::Quantity(range) = &p.parameters[1].schema.value else {
        panic!("Count delta")
    };
    assert_eq!(range.minimum.value(), -f64::MAX);
    assert_eq!(range.maximum.value(), f64::MAX);
    assert_eq!(range.minimum.unit().key().as_str(), "def.000000000000295a");
    for k in &p.source_gems {
        let g = c.gem_by_key(k).unwrap();
        assert!(!g.declared_additional_stat_sets.is_empty());
        assert!(
            g.declared_additional_effects.is_empty()
                && g.constructed_additional_effects.is_empty()
                && g.additional_effects.is_empty()
        );
        assert_eq!(
            g.effect_list.as_slice(),
            std::slice::from_ref(&g.primary_effect_id)
        );
        let primary = c.skill_by_id(&g.primary_effect_id).unwrap();
        assert_ne!(primary.support, Some(true));
        assert_ne!(primary.from_tree, Some(true));
        for alias in &g.declared_additional_stat_sets {
            assert!(c.skill_by_id(&alias.id).is_none());
            assert!(!g.effect_list.contains(&alias.id));
        }
    }
    for source in proof["source_tests"].as_array().unwrap() {
        assert!(root().join(source.as_str().unwrap()).is_file());
    }
}

#[test]
#[ignore = "requires the explicit checked legacy corruption-flag release"]
fn real_statset_primary_inputs_preserve_full_release_and_originals() {
    let p = std::env::var_os("POE_OPTIMIZER_TEST_STATSET_GEM_PRIOR")
        .expect("explicit predecessor required");
    release::check(&PathBuf::from(p));
}
