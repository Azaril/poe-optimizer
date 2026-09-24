//! Explicit offline corrections create a new data release, never a monotonic
//! successor. The prior release is checked before any dependency is rebound.
use crate::{
    owned_item_lines::OwnedItemLinePolicy,
    owned_mapping::OwnedMappingIndex,
    owned_normalize::GemQualityPolicy,
    owned_recipe::{OwnedRecipeError, assemble_owned_recipe},
    owned_release::{
        OwnedReleaseError, OwnedReleaseLimits, OwnedReleaseProvenance, StagedOwnedRelease,
        assemble_owned_release, preflight,
    },
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_schema::{DefinitionDescriptor, SlotDescriptor},
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Full replacement descriptors for existing addresses, bound to the complete
/// prior authoring input. No identities are allocated, retired or reused.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedReleaseRevisionInput {
    pub schema_version: u32,
    pub before: OwnedContentDigest,
    pub release: OwnedDefinitionKey,
    pub reason: OwnedDefinitionKey,
    pub definitions: Vec<DefinitionDescriptor>,
    pub slots: Vec<SlotDescriptor>,
}

/// Correct explicitly named schemas in a new immutable release. All other
/// semantic content is preserved; dependent identities are rebuilt in order.
/// This function makes no assertion that builds authored for the old release
/// remain admissible, or that the supplied correction has numerical parity.
pub fn compile_owned_release_revision(
    prior: &StagedOwnedRelease,
    revision: OwnedReleaseRevisionInput,
    limits: OwnedReleaseLimits,
) -> Result<StagedOwnedRelease, OwnedReleaseError> {
    limits.validate()?;
    let count = revision
        .definitions
        .len()
        .checked_add(revision.slots.len())
        .ok_or(OwnedReleaseError::Invalid(
            "release revision entry overflow",
        ))?;
    if count == 0 || count > limits.max_validation_entries {
        return Err(OwnedReleaseError::Invalid("release revision entry limit"));
    }
    let authoring = digest_owned(
        "owned-release-schema-revision-v1",
        &revision,
        limits.max_artifact_bytes,
    )?;
    if revision.schema_version != 1
        || revision.before != prior.receipt().input
        || revision.release == prior.assembled().schema().input().release
    {
        return Err(OwnedReleaseError::Invalid(
            "release revision version or endpoint",
        ));
    }
    if revision
        .definitions
        .windows(2)
        .any(|rows| rows[0].address() >= rows[1].address())
        || revision
            .slots
            .windows(2)
            .any(|rows| rows[0].address() >= rows[1].address())
    {
        return Err(OwnedReleaseError::Invalid(
            "release revision needs canonical unique targets",
        ));
    }

    // The prior constructor already bounded this state. Recheck its complete
    // commitment under the caller's potentially tighter budget before cloning.
    digest_owned(
        "owned-release-revision-prior-budget-v1",
        prior.input(),
        limits.max_input_bytes,
    )?;
    if prior.input().provenance.len() >= limits.max_provenance_entries {
        return Err(OwnedReleaseError::Limit("provenance entries"));
    }
    // The revision adds one authoring row; charge it before copying prior data.
    let mut prior_limits = limits;
    prior_limits.max_validation_entries = limits
        .max_validation_entries
        .checked_sub(1)
        .ok_or(OwnedReleaseError::Limit("validation entries"))?;
    preflight(prior.input(), prior_limits)?;
    let mut input = prior.input().clone();
    let definitions: BTreeMap<_, _> = input
        .recipe
        .schema
        .definitions
        .iter()
        .enumerate()
        .map(|(index, row)| (row.address(), index))
        .collect();
    let slots: BTreeMap<_, _> = input
        .recipe
        .schema
        .slots
        .iter()
        .enumerate()
        .map(|(index, row)| (row.address(), index))
        .collect();
    for row in revision.definitions {
        let position = definitions
            .get(&row.address())
            .ok_or(OwnedReleaseError::Invalid(
                "release revision cannot allocate definitions",
            ))?;
        if input.recipe.schema.definitions[*position] == row {
            return Err(OwnedReleaseError::Invalid(
                "release revision contains an unchanged definition",
            ));
        }
        input.recipe.schema.definitions[*position] = row;
    }
    for row in revision.slots {
        let position = slots.get(&row.address()).ok_or(OwnedReleaseError::Invalid(
            "release revision cannot allocate slots",
        ))?;
        if input.recipe.schema.slots[*position] == row {
            return Err(OwnedReleaseError::Invalid(
                "release revision contains an unchanged slot",
            ));
        }
        input.recipe.schema.slots[*position] = row;
    }
    input.recipe.schema.release = revision.release;
    let schema =
        OwnedDefinitionSchemaPackage::new(input.recipe.schema.clone(), limits.recipe.schema)
            .map_err(OwnedRecipeError::from)?;
    input.recipe.rules.definitions = schema.identity().clone();
    input.recipe.routing.definitions = schema.identity().clone();
    let runtime = assemble_owned_recipe(input.recipe.clone(), limits.recipe)?;

    input.mapping.definitions = runtime.schema().identity().clone();
    input.mapping.registry = runtime.registry().identity()?;
    let mapping = OwnedMappingIndex::new(
        input.mapping.clone(),
        runtime.registry(),
        runtime.schema(),
        limits.catalog.mapping,
    )?;
    input.roles.mapping = *mapping.identity();
    input.roles.definitions = runtime.schema().identity().clone();
    if let GemQualityPolicy::Attributes(quality) = &mut input.normalization.gem_quality {
        quality.definitions = runtime.schema().identity().clone();
    }
    if let Some(gems) = &mut input.normalization.gem_inputs {
        gems.definitions = runtime.schema().identity().clone();
    }
    input.rewards.mapping = *mapping.identity();
    input.rewards.definitions = runtime.schema().identity().clone();
    input.items.definitions = runtime.schema().identity().clone();
    let items = OwnedItemLinePolicy::new(input.items.clone(), runtime.schema(), limits.items)?;
    input.item_source.item_lines = *items.identity();
    input.tree = input
        .tree
        .map(|tree| {
            OwnedTreeNormalizationPolicy::bind_new(
                tree.content,
                runtime.registry(),
                runtime.schema(),
                &mapping,
                &input.normalization,
                limits.tree,
            )
            .map(|tree| tree.input().clone())
        })
        .transpose()?;
    input.provenance.push(OwnedReleaseProvenance {
        kind: revision.reason,
        prior_input: prior.receipt().input,
        authoring_input: authoring,
    });
    assemble_owned_release(input, limits)
}
