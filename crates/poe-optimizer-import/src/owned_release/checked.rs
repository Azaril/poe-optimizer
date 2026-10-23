//! Shared constituent validation, without authoring or publication buffers.
use super::constituents::CheckedImportConstituents;
use super::*;
use crate::owned_recipe::{CheckedOwnedRecipe, OwnedRecipeView};

/// Borrowed serialization view used only for existing typed commitments and
/// aggregate validation. It does not reconstruct an authoring input or allocate.
#[derive(Serialize)]
pub(crate) struct ReleaseView<'a> {
    pub schema_version: u32,
    pub recipe: OwnedRecipeView<'a>,
    pub mapping: &'a MappingPackageInput,
    pub roles: &'a OwnedSkillRolePackageInput,
    pub normalization: &'a NormalizationPolicy,
    pub rewards: &'a RewardPolicyInput,
    pub items: &'a ItemLinePolicyInput,
    pub item_source: &'a ItemSourceLayoutPolicyInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree: Option<&'a TreeNormalizationPackageInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluation: Option<&'a OwnedReleaseEvaluationInput>,
    pub query_sets: &'a [NamedQuerySet],
    #[serde(skip_serializing_if = "<[OwnedReleaseProvenance]>::is_empty")]
    pub provenance: &'a [OwnedReleaseProvenance],
}

impl<'a> From<&'a OwnedReleaseInput> for ReleaseView<'a> {
    fn from(input: &'a OwnedReleaseInput) -> Self {
        Self {
            schema_version: input.schema_version,
            recipe: (&input.recipe).into(),
            mapping: &input.mapping,
            roles: &input.roles,
            normalization: &input.normalization,
            rewards: &input.rewards,
            items: &input.items,
            item_source: &input.item_source,
            tree: input.tree.as_ref(),
            evaluation: input.evaluation.as_ref(),
            query_sets: &input.query_sets,
            provenance: &input.provenance,
        }
    }
}

/// Validated immutable release components. Runtime consumers retain these
/// packages; they do not need offline assembly or encoded publication buffers.
pub struct CheckedOwnedRelease {
    pub(super) limits: OwnedReleaseLimits,
    recipe: CheckedOwnedRecipe,
    mapping: OwnedMappingIndex,
    roles: OwnedSkillRoleIndex,
    normalization: NormalizationPolicy,
    rewards: OwnedRewardPolicy,
    items: OwnedItemLinePolicy,
    item_source: ItemSourceLayoutPolicy,
    tree: Option<OwnedTreeNormalizationPolicy>,
    evaluation: Option<CheckedOwnedReleaseEvaluation>,
    query_sets: Vec<NamedQuerySet>,
    pub(super) receipt: OwnedReleaseReceipt,
}

impl CheckedOwnedRelease {
    pub(crate) fn check(input: ReleaseView<'_>, limits: OwnedReleaseLimits) -> Result<Self> {
        limits.validate()?;
        let domain = input_domain(input.schema_version, input.evaluation.is_some())?;
        let query_policy_bytes = preflight_view(&input, limits)?;
        // Bound nested data before constituent clones or indexes are allocated.
        digest_owned(domain, &input, limits.max_input_bytes)?;
        let mut recipe = CheckedOwnedRecipe::new(input.recipe.to_input(), limits.recipe)?;
        recipe.canonicalize_input_identity(limits.recipe.max_wire_bytes)?;
        let CheckedImportConstituents {
            mapping,
            roles,
            normalization,
            rewards,
            items,
            item_source,
            tree,
        } = CheckedImportConstituents::check(
            &input,
            recipe.registry(),
            recipe.schema().as_ref(),
            limits,
        )?;
        let evaluation = input
            .evaluation
            .map(|value| {
                CheckedOwnedReleaseEvaluation::new(value.clone(), &recipe, limits.evaluation)
            })
            .transpose()?;
        let source = source_union(&mapping, &roles, &item_source, tree.as_ref())?;
        let canonical = ReleaseView {
            schema_version: input.schema_version,
            recipe: recipe.view(),
            mapping: mapping.input(),
            roles: roles.input(),
            normalization: input.normalization,
            rewards: rewards.input(),
            items: items.input(),
            item_source: item_source.input(),
            tree: tree.as_ref().map(|v| v.input()),
            evaluation: evaluation.as_ref().map(|v| v.input()),
            query_sets: input.query_sets,
            provenance: input.provenance,
        };
        let receipt = OwnedReleaseReceipt {
            schema_version: input.schema_version,
            document_kind: "owned_data_release".into(),
            input: digest_owned(domain, &canonical, limits.max_input_bytes)?,
            registry: recipe.registry().identity()?,
            definitions: recipe.schema().identity().clone(),
            rules: *recipe.rules().identity(),
            compiled_rules: recipe.compiled_rules().identity(),
            routing: *recipe.routing().identity(),
            mapping: *mapping.identity(),
            roles: *roles.identity(),
            normalization,
            rewards: *rewards.identity(),
            items: *items.identity(),
            item_source: *item_source.identity(),
            tree: tree.as_ref().map(|v| *v.identity()),
            evaluation: evaluation
                .as_ref()
                .map(CheckedOwnedReleaseEvaluation::receipt),
            source,
            provenance: input.provenance.to_vec(),
            query_sets: input.query_sets.len(),
            query_rows: input.query_sets.iter().map(|v| v.queries.len()).sum(),
            query_policy_bytes,
            artifacts: vec![],
        };
        Ok(Self {
            limits,
            recipe,
            mapping,
            roles,
            normalization: input.normalization.clone(),
            rewards,
            items,
            item_source,
            tree,
            evaluation,
            query_sets: input.query_sets.to_vec(),
            receipt,
        })
    }

    pub fn recipe(&self) -> &CheckedOwnedRecipe {
        &self.recipe
    }
    pub fn mapping(&self) -> &OwnedMappingIndex {
        &self.mapping
    }
    pub fn roles(&self) -> &OwnedSkillRoleIndex {
        &self.roles
    }
    pub fn normalization(&self) -> &NormalizationPolicy {
        &self.normalization
    }
    pub fn rewards(&self) -> &OwnedRewardPolicy {
        &self.rewards
    }
    pub fn items(&self) -> &OwnedItemLinePolicy {
        &self.items
    }
    pub fn item_source(&self) -> &ItemSourceLayoutPolicy {
        &self.item_source
    }
    pub fn tree(&self) -> Option<&OwnedTreeNormalizationPolicy> {
        self.tree.as_ref()
    }
    pub fn evaluation(&self) -> Option<&CheckedOwnedReleaseEvaluation> {
        self.evaluation.as_ref()
    }
    pub fn query_sets(&self) -> &[NamedQuerySet] {
        &self.query_sets
    }
    pub fn receipt(&self) -> &OwnedReleaseReceipt {
        &self.receipt
    }
}

/// Explicit offline reconstruction for revision/publication tools. Runtime
/// evaluation never calls this conversion.
impl OwnedReleaseInput {
    pub fn from_checked(release: &CheckedOwnedRelease) -> Self {
        Self {
            schema_version: release.receipt.schema_version,
            recipe: release.recipe.view().to_input(),
            mapping: release.mapping.input().clone(),
            roles: release.roles.input().clone(),
            normalization: release.normalization.clone(),
            rewards: release.rewards.input().clone(),
            items: release.items.input().clone(),
            item_source: release.item_source.input().clone(),
            tree: release.tree.as_ref().map(|v| v.input().clone()),
            evaluation: release.evaluation.as_ref().map(|v| v.input().clone()),
            query_sets: release.query_sets.clone(),
            provenance: release.receipt.provenance.clone(),
        }
    }
}

pub(super) fn input_domain(version: u32, evaluation: bool) -> Result<&'static str> {
    match (version, evaluation) {
        (OWNED_RELEASE_VERSION, false) => Ok("owned-data-release-input-v1"),
        (OWNED_EVALUATION_RELEASE_VERSION, true) => Ok("owned-data-release-input-v2"),
        (OWNED_RELEASE_VERSION, true) => Err(OwnedReleaseError::Invalid(
            "version 1 cannot contain evaluation artifacts",
        )),
        (OWNED_EVALUATION_RELEASE_VERSION, false) => Err(OwnedReleaseError::Invalid(
            "version 2 requires evaluation artifacts",
        )),
        (version, _) => Err(OwnedReleaseError::Version(version)),
    }
}
