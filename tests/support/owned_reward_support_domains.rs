//! Explicit support-source certificates for already-complete owned numeric
//! rewards. This is offline authoring, not runtime inference from empty buckets.
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_content::digest_owned, owned_definitions::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_import::{
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
const KIND: &str = "reward-support-source-domains";
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/reward-support-domains")
}
pub fn read<T: DeserializeOwned>(n: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(n)).unwrap()).unwrap()
}
#[derive(Clone, Deserialize)]
pub struct Dependencies {
    pub definitions: Vec<DefinitionDescriptor>,
    pub owners: Vec<DefinitionRules>,
    operations_version: OwnedDefinitionKey,
}
pub fn extension() -> OwnedRecipeExtension {
    read("extension.json")
}
pub fn admissible(d: &DefinitionDescriptor, o: &DefinitionRules) -> bool {
    if !matches!(d, DefinitionDescriptor::Reward(_))
        || o.owner != SchemaSubject::Definition(d.address())
        || !o.programs.is_complete()
        || o.programs.members.is_empty()
    {
        return false;
    }
    let v = json!(d);
    let s = &v["value"]["schema"];
    if s["kind"] != "known" {
        return false;
    }
    let Some(declarations) = s["value"]["declarations"].as_object() else {
        return false;
    };
    if declarations.len() != 7
        || !declarations.values().all(|d| {
            d["closure"]["kind"] == "complete" && d["members"].as_array().is_some_and(Vec::is_empty)
        })
    {
        return false;
    }
    o.programs.members.iter().all(|p| {
        json!(p.context)=="actor" && p.reads.is_empty() && !p.effects.is_empty()
        && p.nodes.iter().all(|n|matches!(&n.expression,RuleExpression::Literal{value:ParameterValue::Quantity(_)}))
        && p.effects.iter().all(|e|e.when.is_none() && matches!(&e.effect,RuleEffectKind::Contribute{entity:RuleEntity::Player,contribution:ContributionKind::Add|ContributionKind::Increase|ContributionKind::Multiply,value,..} if p.nodes.iter().any(|n|n.id==*value)))
    })
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let deps: Dependencies = read("dependencies.json");
    let e = extension();
    assert_eq!(deps.operations_version.as_str(), OWNED_RULE_OPERATIONS_V25);
    assert_eq!((deps.definitions.len(), deps.owners.len()), (31, 19));
    let mut expected: Vec<_> = deps
        .owners
        .iter()
        .filter(|o| deps.definitions.iter().any(|d| admissible(d, o)))
        .map(|o| o.owner.clone())
        .collect();
    let mut actual: Vec<_> = e
        .support_source_domains
        .iter()
        .map(|d| {
            assert_eq!(
                d.domain,
                SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly)
            );
            d.owner.clone()
        })
        .collect();
    expected.sort_by_cached_key(|s| json!(s).to_string());
    actual.sort_by_cached_key(|s| json!(s).to_string());
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 18);
    assert!(
        e.schema.is_empty()
            && e.tables.is_empty()
            && e.owners.is_empty()
            && e.receivers.is_empty()
            && e.operations_version.is_none()
    );
    for n in ["dependencies.json", "extension.json"] {
        let b = fs::read(data().join(n)).unwrap();
        assert_eq!(a["artifacts"][n]["bytes"], b.len());
        assert_eq!(
            a["artifacts"][n]["sha256"],
            format!("{:x}", Sha256::digest(b))
        );
    }
    assert_eq!(a["remaining_unknown"], 13);
    assert_eq!(a["full_build"], false);
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    assert_eq!(a["before"], json!(prior.receipt().input));
    let deps: Dependencies = read("dependencies.json");
    let r = &prior.input().recipe;
    assert_eq!(r.rules.operations_version, deps.operations_version);
    assert_eq!(
        r.schema
            .definitions
            .iter()
            .filter(|d| matches!(d, DefinitionDescriptor::Reward(_)))
            .cloned()
            .collect::<Vec<_>>(),
        deps.definitions
    );
    assert_eq!(
        r.rules
            .owners
            .iter()
            .filter(|o| matches!(
                &o.owner,
                SchemaSubject::Definition(DefinitionAddress::Reward(_))
            ))
            .cloned()
            .collect::<Vec<_>>(),
        deps.owners
    );
    let e = extension();
    let extended = extend_owned_recipe(prior.assembled(), &e, Default::default()).unwrap();
    assert!(extended.refinement.is_none());
    assert_eq!(extended.receipt.appended_support_source_domains, 18);
    assert_eq!(extended.receipt.allocated_entries, 0);
    assert_eq!(extended.receipt.appended_programs, 0);
    let mut input = prior.input().clone();
    input.recipe = extended.successor;
    input.provenance.push(OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(KIND, &(a, e), 1024 * 1024).unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    inverse.recipe.rules.support_discovery = prior.input().recipe.rules.support_discovery.clone();
    inverse.provenance.pop();
    assert_eq!(
        inverse,
        *prior.input(),
        "only explicit support certificates and provenance change"
    );
    crate::migration_preservation::assert_import_rebindings_only(prior, &next);
    next
}
