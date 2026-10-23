//! Immutable import-only skill-role evidence bound to exact owned dependencies.
//!
//! Structural validation preserves historical source-pin subsets and never
//! generates catalog identities, allocates IDs or reproduces source catalogs.
use crate::owned_mapping::{
    ExternalOwnerSelector, ExternalSelector, MappingOutcome, OwnedMappingError, OwnedMappingIndex,
    OwnedMappingLimits, SourceComponent, SourcePin,
};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::SchemaPackageError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) mod source_construction;
pub use source_construction::{
    ObservedGemSourceConstruction, OwnedGemSourceConstruction, OwnedIndexedSourceEffectReference,
    OwnedMissingSourceEffect, OwnedMissingSourceEffectKind, OwnedSourceEffectReference,
    OwnedSourceEffectResolution, OwnedSourceStatSetReference,
};
pub(crate) mod reverse_primary;
pub use reverse_primary::{
    OwnedReversePrimaryCandidate, OwnedReversePrimaryCensus, OwnedReversePrimaryLookup,
    OwnedReversePrimaryRoute,
};

pub const OWNED_SKILL_ROLE_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbsentSupportPolicy {
    Pending,
    NonSupport,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbsentFromTreePolicy {
    Pending,
    Physical,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillCatalogPolicy {
    pub version: OwnedDefinitionKey,
    pub absent_support: AbsentSupportPolicy,
    pub absent_from_tree: AbsentFromTreePolicy,
}
/// Independent limits for compilation/role artifacts. Source catalogs are already
/// validated; their full serialized content is also bounded before hashing.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SkillCatalogLimits {
    pub mapping: OwnedMappingLimits,
}

#[derive(Debug, thiserror::Error)]
pub enum SkillCatalogError {
    #[error("{0}: skill catalog resource limit exceeded")]
    LimitExceeded(&'static str),
    #[error("source pin does not match the injected identity catalog")]
    SourceMismatch,
    #[error("owned skill role namespace differs from its bound dependencies")]
    ForeignNamespace,
    #[error("owned skill role artifact binding mismatch")]
    BindingMismatch,
    #[error("duplicate owned gem role")]
    DuplicateRole,
    #[error("owned skill role target is absent, has the wrong domain, or is inconsistent")]
    UnknownTarget,
    #[error("owned skill role contradicts known schema metadata")]
    SchemaConflict,
    #[error("owned source construction facts are inconsistent with their bound identities")]
    ConstructionMismatch,
    #[error("owned reverse primary routes are inconsistent with their bound identities")]
    ReversePrimaryMismatch,
    #[error("owned role rows do not reproduce the authenticated source catalog")]
    ReproductionMismatch,
    #[error("existing selector has no unambiguous reusable identity: {0:?}")]
    ReuseUnresolved(Box<ExternalSelector>),
    #[error("catalog selector does not uniquely identify a reusable source row: {0:?}")]
    DuplicateSourceSelector(Box<ExternalSelector>),
    #[error(transparent)]
    Schema(#[from] SchemaPackageError),
    #[error("unsupported owned skill role version {0}")]
    UnsupportedVersion(u32),
    #[error(transparent)]
    Mapping(#[from] OwnedMappingError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, SkillCatalogError>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OwnedPrimarySkill {
    Known(SkillDefId),
    Unmapped { issue: OwnedDefinitionKey },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OwnedGemRole {
    Known(AuthoredGemRole),
    Unmapped { issue: OwnedDefinitionKey },
}
/// Whether catalog evidence permits physical materialization, independently of
/// the primary effect's skill/support role. Source group-origin policy is another
/// required check; Physical alone never proves an authored physical occurrence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum OwnedGemMaterialization {
    Physical,
    ProviderOnly,
    Unmapped { issue: OwnedDefinitionKey },
}
/// Import-only classification of a mapped catalog identity. ProviderOnly rows
/// retain an Unmapped Gem identity placeholder until provider conversion and
/// never authorize physical Gem instances. A separate reviewed manual-source
/// policy may prove an authored Direct SkillUse; the catalog alone cannot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedGemRoleRow {
    pub gem: GemDefId,
    pub primary: OwnedPrimarySkill,
    pub role: OwnedGemRole,
    pub materialization: OwnedGemMaterialization,
    /// Import evidence only; this never declares native Skill supplies or mechanics.
    pub construction: OwnedGemSourceConstruction,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillCatalogReceipt {
    pub source: SourcePin,
    pub catalog_digest: OwnedContentDigest,
    pub policy: SkillCatalogPolicy,
    pub base_registry: OwnedContentDigest,
    pub staged_registry: OwnedContentDigest,
    pub gem_count: usize,
    pub skill_count: usize,
}
pub(crate) fn subject<I: SchemaDefinitionId>(id: &I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
pub(crate) struct Budget {
    pub(crate) limits: OwnedMappingLimits,
    remaining: usize,
    text_remaining: usize,
}
impl Budget {
    pub(crate) fn new(limits: SkillCatalogLimits) -> Result<Self> {
        limits.mapping.validate()?;
        Ok(Self {
            limits: limits.mapping,
            remaining: limits.mapping.max_entries,
            text_remaining: limits.mapping.max_total_string_bytes,
        })
    }
    pub(crate) fn collection(&mut self, name: &'static str, size: usize) -> Result<()> {
        if size > self.limits.max_collection_entries || size > self.remaining {
            return Err(SkillCatalogError::LimitExceeded(name));
        }
        self.remaining -= size;
        Ok(())
    }
    pub(crate) fn text(&mut self, value: &str) -> Result<()> {
        if value.len() > self.limits.max_string_bytes || value.len() > self.text_remaining {
            return Err(SkillCatalogError::LimitExceeded("source text"));
        }
        self.text_remaining -= value.len();
        Ok(())
    }
    pub(crate) fn pin(&mut self, pin: &SourcePin) -> Result<()> {
        self.collection("source files", pin.files.len())?;
        self.text(&pin.revision)?;
        if pin.revision.is_empty() || pin.files.is_empty() {
            return Err(SkillCatalogError::SourceMismatch);
        }
        let mut paths = BTreeSet::new();
        for file in &pin.files {
            self.text(&file.path)?;
            self.text(&file.sha256)?;
            if file.path.is_empty()
                || file.path.starts_with('/')
                || file.path.contains(['\\', ':'])
                || file
                    .path
                    .split('/')
                    .any(|part| part.is_empty() || part == "." || part == "..")
                || !paths.insert(&file.path)
                || file.sha256.len() != 64
                || !file
                    .sha256
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            {
                return Err(SkillCatalogError::SourceMismatch);
            }
        }
        Ok(())
    }
}

/// Authored role evidence without current schema or mapping bindings. Receipt
/// hashes are provenance claims, not attestation or proof of registry succession.
/// A host may append other domains after skill compilation, so the final mapping
/// registry need not equal the receipt's staged registry. The host validates and
/// publishes registry history and reviewed compiler provenance separately.
/// Historical source files must remain an immutable matching subset of the final
/// mapping pins; rebinding never rewrites the original compilation receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillRoleContent {
    /// Historical compiler provenance remains unchanged when binding to a new
    /// definition space. Its registry hashes are not current dependency seals.
    pub compilation: SkillCatalogReceipt,
    pub roles: Vec<OwnedGemRoleRow>,
    pub reverse_primary: OwnedReversePrimaryCensus,
}

/// Runtime skill-role evidence bound to exact definitions and mappings. These
/// current dependency seals are checked independently of compiler provenance.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedSkillRolePackageInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub definitions: DataIdentity,
    pub mapping: OwnedContentDigest,
    pub compilation: SkillCatalogReceipt,
    pub roles: Vec<OwnedGemRoleRow>,
    /// Complete compiler observation or explicit unavailability; never inferred
    /// from the surviving role rows or from a whole-catalog digest alone.
    pub reverse_primary: OwnedReversePrimaryCensus,
}
/// Immutable import-only role evidence. No source catalog or evaluator is retained.
#[derive(Clone, Debug)]
pub struct OwnedSkillRoleIndex {
    input: OwnedSkillRolePackageInput,
    identity: OwnedContentDigest,
    positions: BTreeMap<GemDefId, usize>,
    sources: BTreeMap<ExternalSelector, MappingOutcome>,
}
/// A historical compiler receipt can cover fewer files than a later combined
/// catalog. Its original source context and every file hash must still match.
/// Final schema/mapping digests remain exact and are checked independently.
pub(crate) fn provenance_is_subset(previous: &SourcePin, current: &SourcePin) -> bool {
    previous.system == current.system
        && previous.revision == current.revision
        && previous.files.iter().all(|pin| {
            current
                .files
                .binary_search_by(|value| value.path.cmp(&pin.path))
                .ok()
                .is_some_and(|index| current.files[index].sha256 == pin.sha256)
        })
}

impl OwnedSkillRoleIndex {
    /// Bind authored role evidence to actual checked dependencies. Existing
    /// semantic validation still checks every source, member and coverage fact.
    pub fn bind_new<I: DefinitionSchemaIndex>(
        content: SkillRoleContent,
        mappings: &OwnedMappingIndex,
        definitions: &I,
        limits: SkillCatalogLimits,
    ) -> Result<Self> {
        limits.mapping.validate()?;
        Self::new(
            OwnedSkillRolePackageInput {
                schema_version: OWNED_SKILL_ROLE_VERSION,
                namespace: definitions.namespace().clone(),
                definitions: definitions.identity().clone(),
                mapping: *mappings.identity(),
                compilation: content.compilation,
                roles: content.roles,
                reverse_primary: content.reverse_primary,
            },
            mappings,
            definitions,
            limits,
        )
    }

    /// Move checked semantic evidence into authoring without cloning it,
    /// rewriting compiler provenance or granting any additional coverage.
    pub fn into_authored_content(self) -> SkillRoleContent {
        SkillRoleContent {
            compilation: self.input.compilation,
            roles: self.input.roles,
            reverse_primary: self.input.reverse_primary,
        }
    }

    pub fn new<I: DefinitionSchemaIndex>(
        mut input: OwnedSkillRolePackageInput,
        mappings: &OwnedMappingIndex,
        definitions: &I,
        limits: SkillCatalogLimits,
    ) -> Result<Self> {
        let mut budget = Budget::new(limits)?;
        mappings.validate_limits(limits.mapping)?;
        if input.schema_version != OWNED_SKILL_ROLE_VERSION {
            return Err(SkillCatalogError::UnsupportedVersion(input.schema_version));
        }
        if input.namespace != *definitions.namespace()
            || input.namespace != mappings.input().namespace
        {
            return Err(SkillCatalogError::ForeignNamespace);
        }
        budget.pin(&input.compilation.source)?;
        input
            .compilation
            .source
            .files
            .sort_by(|a, b| a.path.cmp(&b.path));
        if input.definitions != *definitions.identity()
            || input.definitions != mappings.input().definitions
            || input.mapping != *mappings.identity()
            || !provenance_is_subset(&input.compilation.source, &mappings.input().source)
            || input.compilation.policy.version != mappings.input().policy_version
            || input.roles.len() != input.compilation.gem_count
        {
            return Err(SkillCatalogError::BindingMismatch);
        }
        budget.collection("role rows", input.roles.len())?;
        for row in &input.roles {
            source_construction::preflight(&row.construction, &mut budget)?;
        }
        reverse_primary::preflight(&input.reverse_primary, &mut budget)?;
        if matches!(
            input.reverse_primary,
            OwnedReversePrimaryCensus::Observed { .. }
        ) {
            budget.collection("reverse primary role joins", input.roles.len())?;
        }
        input.roles.sort_by(|a, b| a.gem.cmp(&b.gem));
        let mut positions = BTreeMap::new();
        let mut sources = BTreeMap::new();
        let mut mapped_gems = BTreeSet::new();
        let mut construction_keys = BTreeSet::new();
        for entry in &mappings.input().entries {
            if matches!(
                entry.source,
                ExternalSelector::Definition(ExternalOwnerSelector::Gem { .. })
            ) {
                budget.collection("role source mappings", 1)?;
                let targets: &[SchemaSubject] = match &entry.outcome {
                    MappingOutcome::Mapped { target, .. } => std::slice::from_ref(target),
                    MappingOutcome::Ambiguous { candidates, .. } => candidates,
                    MappingOutcome::Unmapped { .. } => &[],
                };
                budget.collection("role source candidates", targets.len())?;
                for target in targets {
                    let SchemaSubject::Definition(DefinitionAddress::Gem(id)) = target else {
                        return Err(SkillCatalogError::UnknownTarget);
                    };
                    mapped_gems.insert(id.clone());
                }
                sources.insert(entry.source.clone(), entry.outcome.clone());
            }
        }
        for (i, row) in input.roles.iter().enumerate() {
            if positions.insert(row.gem.clone(), i).is_some() {
                return Err(SkillCatalogError::DuplicateRole);
            }
            if row.gem.namespace() != &input.namespace {
                return Err(SkillCatalogError::ForeignNamespace);
            }
            if !mapped_gems.contains(&row.gem) {
                return Err(SkillCatalogError::UnknownTarget);
            }
            let gem_schema = match definitions.definition(&row.gem) {
                SchemaLookup::Known(schema) => Some(schema),
                SchemaLookup::Unmapped(_) => None,
                _ => return Err(SkillCatalogError::UnknownTarget),
            };
            // A provider-only source identity is not a physical Gem definition.
            // The compiler deliberately leaves its placeholder schema Unmapped.
            if gem_schema.is_some()
                && matches!(row.materialization, OwnedGemMaterialization::ProviderOnly)
            {
                return Err(SkillCatalogError::SchemaConflict);
            }
            if let OwnedPrimarySkill::Known(primary) = &row.primary {
                if primary.namespace() != &input.namespace {
                    return Err(SkillCatalogError::ForeignNamespace);
                }
                match definitions.definition(primary) {
                    SchemaLookup::Known(_) | SchemaLookup::Unmapped(_) => {}
                    _ => return Err(SkillCatalogError::UnknownTarget),
                }
                if let Some(schema) = gem_schema {
                    budget.collection("known gem skill membership", schema.skills.members.len())?;
                    // `primary` is the imported catalogue association. Only an
                    // authored active Gem promises it as a potential Skill
                    // supply; a support primary identifies modifier behavior.
                    if row.role != OwnedGemRole::Known(AuthoredGemRole::SupportAssignment)
                        && schema.skills.is_complete()
                        && !schema.skills.members.contains(primary)
                    {
                        return Err(SkillCatalogError::SchemaConflict);
                    }
                }
            } else if matches!(row.role, OwnedGemRole::Known(_))
                || !matches!(
                    row.materialization,
                    OwnedGemMaterialization::Unmapped { .. }
                )
            {
                return Err(SkillCatalogError::SchemaConflict);
            }
            if let (Some(schema), OwnedGemRole::Known(role)) = (gem_schema, &row.role) {
                budget.collection("known gem roles", schema.roles.len())?;
                if !schema.roles.contains(role) {
                    return Err(SkillCatalogError::SchemaConflict);
                }
            }
            source_construction::validate(row, mappings, definitions, &mut construction_keys)?;
        }
        reverse_primary::validate(
            &mut input.reverse_primary,
            &input.roles,
            &positions,
            input.compilation.skill_count,
            mappings,
            definitions,
        )?;
        let identity = digest_owned(
            "owned-skill-role-package-v1",
            &input,
            limits.mapping.max_wire_bytes,
        )?;
        Ok(Self {
            input,
            identity,
            positions,
            sources,
        })
    }
    pub fn input(&self) -> &OwnedSkillRolePackageInput {
        &self.input
    }
    pub fn identity(&self) -> &OwnedContentDigest {
        &self.identity
    }
    pub fn role(&self, gem: &GemDefId) -> Option<&OwnedGemRoleRow> {
        self.positions.get(gem).map(|i| &self.input.roles[*i])
    }
    /// Exact source selector semantics, including ambiguity and explicit unmapped
    /// outcomes. A successful identity lookup still requires a separate role lookup.
    pub fn lookup(&self, source: &ExternalSelector) -> Option<&MappingOutcome> {
        self.sources.get(source)
    }
    /// Exact source Skill lookup. Unobserved or mismatched identities remain
    /// unavailable; only an observed empty candidate list establishes absence.
    /// Structural loading does not replace offline full-catalog reproduction.
    pub fn reverse_primary(
        &self,
        effect_id: &str,
        skill: &SkillDefId,
    ) -> OwnedReversePrimaryLookup<'_> {
        reverse_primary::lookup(&self.input.reverse_primary, effect_id, skill)
    }
}
