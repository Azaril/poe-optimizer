//! Physical quality-kind closure is independent from numeric/game-rule coverage.
#[path = "support/owned_physical_gem_quality_release.rs"]
mod release;
use poe_optimizer_core::{owned_definitions::*, owned_schema::*};
use poe_optimizer_data::skill_identities::SkillIdentityCatalog;
use poe_optimizer_import::{owned_mapping::*, owned_release_revision::OwnedReleaseRevisionInput};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
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
fn data() -> PathBuf {
    base().join("physical-gem-quality-kinds")
}
fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: impl AsRef<Path>, v: &impl Serialize) {
    fs::write(p, serde_json::to_vec(v).unwrap()).unwrap();
}
fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}
fn sha(v: &impl Serialize) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_vec(v).unwrap()))
}
fn command(name: &str) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    c.arg(name);
    c
}
fn success(o: Output) -> serde_json::Value {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scope {
    schema_version: u32,
    before: String,
    catalog_source_revision: String,
    source_test: String,
    catalog_count: usize,
    known_count: usize,
    complete_unchanged: Vec<OwnedDefinitionKey>,
    unmapped_unchanged: Vec<OwnedDefinitionKey>,
    corrections: Vec<Correction>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Correction {
    owner: OwnedDefinitionKey,
    source_key: String,
    before_descriptor_sha256: String,
    before_closure: SchemaClosure,
}
fn catalog_join(mapping: &MappingPackageInput) -> BTreeMap<GemDefId, String> {
    let catalog =
        SkillIdentityCatalog::new(read(base().join("import/skill-identities.json"))).unwrap();
    assert_eq!(
        catalog.data().source.upstream_revision,
        mapping.source.revision
    );
    assert_eq!(catalog.data().gems.len(), 966);
    let mut joined = BTreeMap::new();
    for gem in &catalog.data().gems {
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text(gem.game_id.clone()),
            variant_id: SourceComponent::Text(gem.variant_id.clone()),
        });
        let rows: Vec<_> = mapping
            .entries
            .iter()
            .filter(|v| v.source == selector)
            .collect();
        assert_eq!(rows.len(), 1);
        let MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Gem(id)),
            basis: MappingBasis::Exact,
        } = &rows[0].outcome
        else {
            panic!("exact physical catalog mapping");
        };
        assert!(joined.insert(id.clone(), gem.key.clone()).is_none());
    }
    assert_eq!(joined.len(), 966);
    joined
}
#[test]
fn correction_covers_exact_catalog_scope_and_changes_only_frozen_quality_closures() {
    let scope: Scope = read(data().join("scope.json"));
    let revision: OwnedReleaseRevisionInput = read(data().join("revision.json"));
    assert_eq!(scope.schema_version, 1);
    assert_eq!(scope.catalog_count, 966);
    assert_eq!(scope.known_count, 604);
    assert_eq!(
        scope.catalog_source_revision,
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    assert_eq!(
        scope.source_test,
        "crates/poe-optimizer-pob/tests/owned_physical_quality_inputs.rs"
    );
    assert_eq!(serde_json::to_value(revision.before).unwrap(), scope.before);
    assert_eq!(revision.schema_version, 1);
    assert!(revision.slots.is_empty());
    assert_eq!(revision.definitions.len(), 603);
    assert_eq!(scope.complete_unchanged, vec![key("def.0000000000000011")]);
    assert_eq!(scope.unmapped_unchanged.len(), 362);
    let joined = catalog_join(&read(base().join("current/mapping.json")));
    let by_key: BTreeMap<_, _> = joined
        .iter()
        .map(|(id, source)| (id.key(), source))
        .collect();
    let mut covered: BTreeSet<_> = scope
        .complete_unchanged
        .iter()
        .chain(&scope.unmapped_unchanged)
        .cloned()
        .collect();
    assert_eq!(covered.len(), 363);
    let corrections: BTreeMap<_, _> = scope.corrections.iter().map(|v| (&v.owner, v)).collect();
    assert_eq!(corrections.len(), 603);
    for descriptor in &revision.definitions {
        let DefinitionDescriptor::Gem(entry) = descriptor else {
            panic!("only physical Gems");
        };
        let SchemaState::Known(gem) = &entry.schema else {
            panic!("only Known descriptors");
        };
        let proof = corrections[entry.id.key()];
        assert_eq!(by_key[entry.id.key()], &proof.source_key);
        assert!(covered.insert(entry.id.key().clone()));
        assert_eq!(gem.quality.allowed_kinds.closure, SchemaClosure::Complete);
        assert_eq!(
            gem.quality
                .allowed_kinds
                .members
                .iter()
                .map(|v| v.key().as_str())
                .collect::<Vec<_>>(),
            vec!["def.0000000000000006"]
        );
        assert!(matches!(
            proof.before_closure,
            SchemaClosure::Partial { .. }
        ));
        let mut before = descriptor.clone();
        let DefinitionDescriptor::Gem(entry) = &mut before else {
            unreachable!()
        };
        let SchemaState::Known(gem) = &mut entry.schema else {
            unreachable!()
        };
        gem.quality.allowed_kinds.closure = proof.before_closure.clone();
        // Hash exact serde serialization of a typed descriptor, including all
        // other declarations and finite quantity bounds, not shell JSON spelling.
        assert_eq!(sha(&before), proof.before_descriptor_sha256);
    }
    assert_eq!(covered, by_key.keys().map(|v| (*v).clone()).collect());
}
#[test]
#[ignore = "requires the explicit checked real raw-Gem-input release"]
fn real_quality_revision_preserves_release_and_pending_input_semantics() {
    let p = std::env::var_os("POE_OPTIMIZER_TEST_PHYSICAL_GEM_QUALITY_PRIOR")
        .expect("explicit predecessor required");
    release::check(&PathBuf::from(p));
}
