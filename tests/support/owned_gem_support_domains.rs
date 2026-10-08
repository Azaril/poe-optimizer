//! Offline source-capability proof. Source names never enter the native rules.
use poe_optimizer_core::{
    owned_content::digest_owned, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_import::{
    owned_mapping::{
        ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, SourceComponent,
    },
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

const KIND: &str = "gem-support-source-domains";
const GAP: &str = "additional-gem-support-origins-not-converted";
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/gem-support-domains")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn key(name: &str) -> OwnedDefinitionKey {
    name.parse().unwrap()
}
fn rows(value: &Value) -> Option<&[Value]> {
    value
        .as_array()
        .map(Vec::as_slice)
        .or_else(|| value.as_object().filter(|o| o.is_empty()).map(|_| &[][..]))
}

/// This proves catalog construction only: the primary may be an authored
/// support; any additional support needs unimplemented origin authority.
/// Display order, active level/quality, source groups and final DPS are irrelevant.
pub fn assignment_only(record: &Value) -> Option<bool> {
    let primary = record["primary_effect"].as_str()?;
    if !record["selector_resolves_same"].as_bool()? {
        return None;
    }
    let effects = rows(&record["effects"])?;
    let mut ids = BTreeSet::new();
    let mut additional = BTreeSet::new();
    let mut only = true;
    let mut found_primary = false;
    for effect in effects {
        let id = effect["id"].as_str()?;
        if !ids.insert(id) {
            return None;
        }
        let support = effect["support"].as_bool()?;
        if id == primary {
            found_primary = support == record["primary_support"].as_bool()?;
        } else {
            additional.insert(id);
            only &= !support;
        }
    }
    if !found_primary {
        return None;
    }
    let supplied = rows(&record["additional_effects"])?;
    let actual: BTreeSet<_> = supplied.iter().map(Value::as_str).collect::<Option<_>>()?;
    if actual.len() != supplied.len() || actual != additional {
        return None;
    }
    let references = rows(&record["declared_references"])?;
    let mut fields = BTreeSet::new();
    let mut declared = BTreeSet::new();
    for reference in references {
        if reference["present_in_constructed_effect_list"] != true
            || reference["resolves_as_effect"] != true
            || !fields.insert(reference["field"].as_str()?)
            || !declared.insert(reference["id"].as_str()?)
        {
            return None;
        }
    }
    (declared == additional).then_some(only)
}

pub fn check_authored() {
    super::catalog_evidence::check_authored();
    let authoring: Value = read("authoring.json");
    assert_eq!(authoring["kind"], KIND);
    assert_eq!(authoring["schema_version"], 1);
    for field in ["artifacts", "evidence"] {
        for pin in authoring[field].as_array().unwrap() {
            let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
            assert_eq!(pin["bytes"], bytes.len());
            assert_eq!(pin["sha256"], hash(&bytes));
        }
    }
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(authoring["source_manifest_sha256"], hash(&manifest_bytes));
    assert_eq!(authoring["source_revision"], manifest["upstream_revision"]);
    for pin in authoring["source_files"].as_array().unwrap() {
        assert!(manifest["files"].as_array().unwrap().contains(pin));
    }
    let records: Vec<Value> = super::catalog_evidence::read("source-records.json");
    let bindings: Vec<Value> = super::catalog_evidence::read("bindings.json");
    let extension: OwnedRecipeExtension = read("extension.json");
    assert_eq!(extension.schema_version, 1);
    assert!(
        extension.schema.is_empty()
            && extension.tables.is_empty()
            && extension.owners.is_empty()
            && extension.receivers.is_empty()
            && extension.operations_version.is_none()
    );
    assert_eq!(extension.support_source_domains.len(), records.len());
    assert_eq!(records.len(), bindings.len());
    let mut expected = Vec::new();
    let mut known = 0;
    let mut unmapped = 0;
    let mut unresolved = 0;
    for (record, binding) in records.iter().zip(&bindings) {
        assert_eq!(record["gem_id"], binding["source_gem"]);
        let gem: GemDefId = serde_json::from_value(binding["gem"].clone()).unwrap();
        let owner = SchemaSubject::Definition(gem.address());
        let domain = match assignment_only(record) {
            Some(true) => {
                known += 1;
                SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly)
            }
            decision => {
                unmapped += 1;
                let code = if decision.is_none() {
                    unresolved += 1;
                    "gem-effect-construction-unresolved"
                } else {
                    GAP
                };
                SchemaState::Unmapped {
                    gaps: vec![SchemaGap {
                        subject: owner.clone(),
                        facet: SchemaFacet::GameRules,
                        code: key(code),
                    }],
                }
            }
        };
        expected.push(SupportSourceDomainDeclaration { owner, domain });
    }
    expected.sort_by_key(|row| match &row.owner {
        SchemaSubject::Definition(d) => d.key().clone(),
        _ => unreachable!(),
    });
    assert_eq!(extension.support_source_domains, expected);
    assert_eq!(
        authoring["domains"],
        json!({"total":966,"known":known,"unmapped":unmapped,"unresolved_construction":unresolved})
    );
    assert_eq!(known + unmapped, 966);
    assert_eq!((known, unmapped, unresolved), (818, 148, 119));
}

fn authenticate_source() {
    let authoring: Value = super::catalog_evidence::read("authoring.json");
    let proof = &authoring["source_validation"];
    let off = fs::read(root().join(proof["evidence_json"].as_str().unwrap())).unwrap();
    let on = fs::read(root().join(proof["evidence_on_json"].as_str().unwrap())).unwrap();
    assert_eq!(
        off.len(),
        proof["evidence_bytes"].as_u64().unwrap() as usize
    );
    assert_eq!(hash(&off), proof["evidence_sha256"]);
    assert!(off == on, "independent original JIT modes");
    let report: Value = serde_json::from_slice(&off).unwrap();
    assert_eq!(
        report["catalog"],
        super::catalog_evidence::read::<Value>("source-records.json")
    );
    assert_eq!(report["source_revision"], authoring["source_revision"]);
    assert_eq!(report["source_hash"], authoring["source_manifest_sha256"]);
    let current: Value = read("authoring.json");
    for pin in current["source_files"].as_array().unwrap() {
        let source = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(pin["path"].as_str().unwrap()),
        )
        .unwrap()
        .replace("\r\n", "\n");
        assert_eq!(pin["sha256"], hash(source.as_bytes()));
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    authenticate_source();
    let authoring: Value = read("authoring.json");
    assert_eq!(json!(prior.receipt().input), authoring["before"]);
    assert!(prior.input().recipe.rules.support_discovery.is_none());
    for binding in super::catalog_evidence::read::<Vec<Value>>("bindings.json") {
        let gem: GemDefId = serde_json::from_value(binding["gem"].clone()).unwrap();
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text(binding["game_id"].as_str().unwrap().into()),
            variant_id: SourceComponent::Text(binding["variant_id"].as_str().unwrap().into()),
        });
        assert!(
            matches!(prior.mapping().lookup(&selector), Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Gem(found)), basis: MappingBasis::Exact
        }) if *found == gem)
        );
    }
    let extension: OwnedRecipeExtension = read("extension.json");
    let extended = extend_owned_recipe(prior.assembled(), &extension, Default::default()).unwrap();
    assert!(extended.refinement.is_none());
    assert_eq!(extended.receipt.appended_support_source_domains, 966);
    assert_eq!(extended.receipt.allocated_entries, 0);
    assert_eq!(extended.receipt.appended_programs, 0);
    assert_eq!(
        extended.receipt.before_definitions,
        extended.receipt.after_definitions
    );
    let mut full = prior.input().clone();
    full.recipe = extended.successor;
    full.provenance.push(OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(KIND, &(authoring, extension), 1024 * 1024).unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut restored = next.input().clone();
    restored.recipe.rules.support_discovery = None;
    restored.provenance.pop();
    assert_eq!(
        restored,
        *prior.input(),
        "only exact domain declarations and provenance change"
    );
    assert_ne!(next.receipt().rules, prior.receipt().rules);
    assert_ne!(
        next.receipt().compiled_rules,
        prior.receipt().compiled_rules
    );
    next
}
