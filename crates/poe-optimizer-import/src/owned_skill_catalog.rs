//! Offline conversion of injected skill identity facts into owned IDs.
//!
//! Fresh compilation and exact same-pin extension share catalog interpretation.
//! New schemas remain Unmapped; extension preserves existing schemas unchanged.
//! Import role evidence establishes neither activation nor numerical coverage.
use crate::owned_mapping::*;
use poe_optimizer_core::{
    data::DataIdentity,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_schema::*,
};
use poe_optimizer_data::{
    owned_schema::{
        OwnedDefinitionSchemaPackage, OwnedSchemaLimits, SchemaPackageError, encode_schema_package,
    },
    skill_identities::SkillIdentityCatalog,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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
/// retain an Unmapped Gem identity placeholder until provider conversion; they
/// must not be materialized as physical GemInstance or authored SkillUse rows.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedGemRoleRow {
    pub gem: GemDefId,
    pub primary: OwnedPrimarySkill,
    pub role: OwnedGemRole,
    pub materialization: OwnedGemMaterialization,
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
/// Staged values; assemble them through the existing schema/mapping constructors.
/// The caller publishes the registry only after the entire intended artifact set
/// succeeds and the host's registry compare-and-swap succeeds.
#[derive(Clone, Debug)]
pub struct FreshOwnedSkillCatalog {
    pub registry: OwnedIdRegistry,
    pub definitions: Vec<DefinitionDescriptor>,
    pub mappings: Vec<MappingEntry>,
    pub roles: Vec<OwnedGemRoleRow>,
    pub receipt: SkillCatalogReceipt,
}

/// Full staged same-pin extension; existing declarations and unrelated mappings
/// are preserved. No successful identity mapping establishes input/rule coverage.
#[derive(Clone, Debug)]
pub struct OwnedSkillCatalogExtension {
    pub registry: OwnedIdRegistry,
    pub definitions: Vec<DefinitionDescriptor>,
    pub slots: Vec<SlotDescriptor>,
    pub mappings: Vec<MappingEntry>,
    pub roles: Vec<OwnedGemRoleRow>,
    pub receipt: SkillCatalogReceipt,
    pub counts: SkillCatalogExtensionCounts,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillCatalogExtensionCounts {
    pub reused_gems: usize,
    pub reused_skills: usize,
    pub allocated_gems: usize,
    pub allocated_skills: usize,
}
struct ExtensionBase<'a> {
    definitions: &'a OwnedDefinitionSchemaPackage,
    mappings: &'a OwnedMappingIndex,
}
struct CatalogCompilation {
    output: FreshOwnedSkillCatalog,
    counts: SkillCatalogExtensionCounts,
}

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).expect("fixed owned compiler symbol")
}
fn subject<I: SchemaDefinitionId>(id: &I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
fn unknown<I: SchemaDefinitionId, T>(id: I) -> DefinitionEntry<I, T> {
    let subject = subject(&id);
    DefinitionEntry {
        id,
        schema: SchemaState::Unmapped {
            gaps: vec![SchemaGap {
                subject,
                facet: SchemaFacet::InputSchema,
                code: key("skill-input-schema-not-converted"),
            }],
        },
    }
}
fn exact(source: ExternalSelector, target: SchemaSubject) -> MappingEntry {
    MappingEntry {
        source,
        outcome: MappingOutcome::Mapped {
            target,
            basis: MappingBasis::Exact,
        },
    }
}
struct Budget {
    limits: OwnedMappingLimits,
    remaining: usize,
    text_remaining: usize,
}
impl Budget {
    fn new(limits: SkillCatalogLimits) -> Result<Self> {
        limits.mapping.validate()?;
        Ok(Self {
            limits: limits.mapping,
            remaining: limits.mapping.max_entries,
            text_remaining: limits.mapping.max_total_string_bytes,
        })
    }
    fn collection(&mut self, name: &'static str, size: usize) -> Result<()> {
        if size > self.limits.max_collection_entries || size > self.remaining {
            return Err(SkillCatalogError::LimitExceeded(name));
        }
        self.remaining -= size;
        Ok(())
    }
    fn text(&mut self, value: &str) -> Result<()> {
        if value.len() > self.limits.max_string_bytes || value.len() > self.text_remaining {
            return Err(SkillCatalogError::LimitExceeded("source text"));
        }
        self.text_remaining -= value.len();
        Ok(())
    }
    fn pin(&mut self, pin: &SourcePin) -> Result<()> {
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

/// Append fresh Gem/Skill definitions without changing `base`. Repeating this
/// operation against the same base is deterministic; applying it to its previous
/// output allocates new IDs and is not a refresh or an identity migration.
pub fn compile_fresh_owned_skill_catalog(
    catalog: &SkillIdentityCatalog,
    base: &OwnedIdRegistry,
    source: &SourcePin,
    policy: &SkillCatalogPolicy,
    limits: SkillCatalogLimits,
) -> Result<FreshOwnedSkillCatalog> {
    Ok(compile_catalog(catalog, base, source, policy, limits, None)?.output)
}

/// Extend an exact prior registry/schema/mapping under the same source pin and
/// policy version. Positive mappings reuse active owned IDs; only absent unique
/// selectors allocate. Ambiguous/unmapped reuse needs an explicit later migration.
///
/// The prior mapping authenticates a policy version, not historical policy
/// settings. This operation records the complete current injected policy in its
/// receipt and checks it against known schema metadata. It makes no stronger
/// unchanged-settings claim. Hosts publish by compare-and-swap on prior identity.
pub fn compile_owned_skill_catalog_extension(
    catalog: &SkillIdentityCatalog,
    base: &OwnedIdRegistry,
    definitions: &OwnedDefinitionSchemaPackage,
    mappings: &OwnedMappingIndex,
    source: &SourcePin,
    policy: &SkillCatalogPolicy,
    limits: SkillCatalogLimits,
) -> Result<OwnedSkillCatalogExtension> {
    base.validate_limits(limits.mapping)?;
    mappings.verify_bindings(base, definitions, source, &policy.version, limits.mapping)?;
    // Bound the whole existing seed before cloning indexes, descriptors or slots.
    digest_owned(
        "owned-skill-extension-base-v1",
        &(base.input(), definitions.input(), mappings.input()),
        limits.mapping.max_wire_bytes,
    )?;
    let schema_limits = OwnedSchemaLimits {
        max_entries: limits.mapping.max_entries,
        max_collection_entries: limits.mapping.max_collection_entries,
        max_wire_bytes: limits.mapping.max_wire_bytes,
    };
    // The immutable schema can have been built under looser caller limits.
    encode_schema_package(definitions, schema_limits)?;
    for descriptor in &definitions.input().definitions {
        base.require_active(&SchemaSubject::Definition(descriptor.address()))?;
    }
    for descriptor in &definitions.input().slots {
        base.require_active(&SchemaSubject::Slot(descriptor.address()))?;
    }
    let CatalogCompilation { mut output, counts } = compile_catalog(
        catalog,
        base,
        source,
        policy,
        limits,
        Some(ExtensionBase {
            definitions,
            mappings,
        }),
    )?;
    base.validate_successor(&output.registry)?;
    // Return the same canonical descriptor order used by the assembled schema,
    // so a no-allocation second pass also preserves staged descriptor ordering.
    output
        .definitions
        .sort_by_cached_key(DefinitionDescriptor::address);
    let mut schema_input = definitions.input().clone();
    schema_input.definitions = output.definitions.clone();
    let schema = OwnedDefinitionSchemaPackage::new(schema_input, schema_limits)?;
    let mut mapping_input = mappings.input().clone();
    mapping_input.registry = output.registry.identity()?;
    mapping_input.definitions = schema.identity().clone();
    mapping_input.entries = output.mappings.clone();
    let final_mappings =
        OwnedMappingIndex::new(mapping_input, &output.registry, &schema, limits.mapping)?;
    // Reuse the established role/schema consistency check, including provider-only,
    // known primary membership, role contradictions and duplicate owned Gem roles.
    OwnedSkillRoleIndex::new(
        OwnedSkillRolePackageInput {
            schema_version: OWNED_SKILL_ROLE_VERSION,
            namespace: schema.input().namespace.clone(),
            definitions: schema.identity().clone(),
            mapping: *final_mappings.identity(),
            compilation: output.receipt.clone(),
            roles: output.roles.clone(),
        },
        &final_mappings,
        &schema,
        limits,
    )?;
    let slots = definitions.input().slots.clone();
    digest_owned(
        "owned-skill-extension-output-v1",
        &(
            output.registry.input(),
            &output.definitions,
            &slots,
            &output.mappings,
            &output.roles,
            &output.receipt,
            counts,
        ),
        limits.mapping.max_wire_bytes,
    )?;
    Ok(OwnedSkillCatalogExtension {
        registry: output.registry,
        definitions: output.definitions,
        slots,
        mappings: output.mappings,
        roles: output.roles,
        receipt: output.receipt,
        counts,
    })
}

fn compile_catalog(
    catalog: &SkillIdentityCatalog,
    base: &OwnedIdRegistry,
    source: &SourcePin,
    policy: &SkillCatalogPolicy,
    limits: SkillCatalogLimits,
    extension: Option<ExtensionBase<'_>>,
) -> Result<CatalogCompilation> {
    let mut budget = Budget::new(limits)?;
    base.validate_limits(limits.mapping)?;
    budget.pin(source)?;
    let data = catalog.data();
    if source.system != ExternalSourceSystem::PathOfBuilding2
        || source.revision != data.source.upstream_revision
    {
        return Err(SkillCatalogError::SourceMismatch);
    }
    let files: BTreeMap<_, _> = source
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.sha256.as_str()))
        .collect();
    if data
        .source
        .files
        .iter()
        .any(|(path, hash)| files.get(path.as_str()).copied() != Some(hash.as_str()))
    {
        return Err(SkillCatalogError::SourceMismatch);
    }
    // Full catalog hashing preserves declaration/constructed evidence exactly;
    // allocation below uses sorted final rows and never recomputes source winners.
    let catalog_digest = digest_owned(
        "owned-skill-source-catalog-v1",
        data,
        limits.mapping.max_wire_bytes,
    )?;
    for (name, count) in [
        ("gem declarations", data.gem_declarations.len()),
        ("skill declarations", data.skill_declarations.len()),
        ("gems", data.gems.len()),
        ("skills", data.skills.len()),
        ("missing references", data.missing_references.len()),
        ("source module order", data.source.skill_module_order.len()),
    ] {
        budget.collection(name, count)?;
    }
    for gem in &data.gems {
        for value in [
            &gem.key,
            &gem.game_id,
            &gem.variant_id,
            &gem.primary_effect_id,
        ] {
            budget.text(value)?;
        }
        for size in [
            gem.declared_additional_effects.len(),
            gem.declared_additional_stat_sets.len(),
            gem.constructed_additional_effects.len(),
            gem.additional_effects.len(),
            gem.effect_list.len(),
            gem.display_order.as_ref().map_or(0, Vec::len),
        ] {
            budget.collection("source effect references", size)?;
        }
    }
    for skill in &data.skills {
        budget.text(&skill.id)?;
    }
    // Output expansion is explicit and bounded independently of source rows.
    budget.collection(
        "owned definitions",
        data.skills
            .len()
            .checked_add(data.gems.len())
            .ok_or(SkillCatalogError::LimitExceeded("owned definitions"))?,
    )?;
    budget.collection("owned role rows", data.gems.len())?;
    let mut counts = SkillCatalogExtensionCounts::default();
    if let Some(previous) = &extension {
        budget.collection("preserved registry entries", base.input().entries.len())?;
        budget.collection(
            "preserved schema definitions",
            previous.definitions.input().definitions.len(),
        )?;
        budget.collection(
            "preserved schema slots",
            previous.definitions.input().slots.len(),
        )?;
        budget.collection(
            "preserved mapping rows",
            previous.mappings.input().entries.len(),
        )?;
    }
    let mut registry = base.clone();
    let mut definitions = extension
        .as_ref()
        .map_or_else(Vec::new, |v| v.definitions.input().definitions.clone());
    let mut mappings = extension
        .as_ref()
        .map_or_else(Vec::new, |v| v.mappings.input().entries.clone());
    let mut skills = BTreeMap::new();
    let mut source_skills: Vec<_> = data.skills.iter().collect();
    source_skills.sort_by(|a, b| a.id.cmp(&b.id));
    for skill in source_skills {
        let source = ExternalSelector::Definition(ExternalOwnerSelector::Skill {
            effect_id: SourceComponent::Text(skill.id.clone()),
        });
        let previous = extension.as_ref().and_then(|e| e.mappings.lookup(&source));
        let id = match previous {
            Some(MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Skill(id)),
                ..
            }) => {
                registry.require_active(&subject(id))?;
                counts.reused_skills += 1;
                id.clone()
            }
            Some(MappingOutcome::Mapped { .. }) => return Err(SkillCatalogError::UnknownTarget),
            Some(_) => return Err(SkillCatalogError::ReuseUnresolved(Box::new(source))),
            None => {
                let id = registry.allocate_definition::<SkillDefinition>()?;
                counts.allocated_skills += 1;
                definitions.push(DefinitionDescriptor::Skill(unknown(id.clone())));
                mappings.push(exact(source, subject(&id)));
                id
            }
        };
        skills.insert(skill.id.as_str(), (id, skill.support, skill.from_tree));
    }
    let mut source_gems: Vec<_> = data.gems.iter().collect();
    source_gems.sort_by(|a, b| a.key.cmp(&b.key));
    let mut gem_selectors: BTreeMap<ExternalSelector, Vec<SchemaSubject>> = BTreeMap::new();
    let mut roles = Vec::new();
    for gem in source_gems {
        let source = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text(gem.game_id.clone()),
            variant_id: SourceComponent::Text(gem.variant_id.clone()),
        });
        if extension.is_some() && gem_selectors.contains_key(&source) {
            return Err(SkillCatalogError::DuplicateSourceSelector(Box::new(source)));
        }
        let previous = extension.as_ref().and_then(|e| e.mappings.lookup(&source));
        let id = match previous {
            Some(MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Gem(id)),
                ..
            }) => {
                registry.require_active(&subject(id))?;
                counts.reused_gems += 1;
                id.clone()
            }
            Some(MappingOutcome::Mapped { .. }) => return Err(SkillCatalogError::UnknownTarget),
            Some(_) => return Err(SkillCatalogError::ReuseUnresolved(Box::new(source))),
            None => {
                let id = registry.allocate_definition::<GemDefinition>()?;
                counts.allocated_gems += 1;
                definitions.push(DefinitionDescriptor::Gem(unknown(id.clone())));
                id
            }
        };
        gem_selectors.entry(source).or_default().push(subject(&id));
        let (primary, role, materialization) = match skills.get(gem.primary_effect_id.as_str()) {
            Some((primary, support, from_tree)) => {
                let role = match (support, policy.absent_support) {
                    (Some(true), _) => OwnedGemRole::Known(AuthoredGemRole::SupportAssignment),
                    (Some(false), _) | (None, AbsentSupportPolicy::NonSupport) => {
                        OwnedGemRole::Known(AuthoredGemRole::SkillUse)
                    }
                    (None, AbsentSupportPolicy::Pending) => OwnedGemRole::Unmapped {
                        issue: key("support-role-unmapped"),
                    },
                };
                let materialization = match (from_tree, policy.absent_from_tree) {
                    (Some(true), _) => OwnedGemMaterialization::ProviderOnly,
                    (Some(false), _) | (None, AbsentFromTreePolicy::Physical) => {
                        OwnedGemMaterialization::Physical
                    }
                    (None, AbsentFromTreePolicy::Pending) => OwnedGemMaterialization::Unmapped {
                        issue: key("gem-materialization-unmapped"),
                    },
                };
                (
                    OwnedPrimarySkill::Known(primary.clone()),
                    role,
                    materialization,
                )
            }
            None => (
                OwnedPrimarySkill::Unmapped {
                    issue: key("primary-effect-unmapped"),
                },
                OwnedGemRole::Unmapped {
                    issue: key("support-role-unmapped"),
                },
                OwnedGemMaterialization::Unmapped {
                    issue: key("gem-materialization-unmapped"),
                },
            ),
        };
        roles.push(OwnedGemRoleRow {
            gem: id,
            primary,
            role,
            materialization,
        });
    }
    budget.collection(
        "mapping rows",
        mappings
            .len()
            .checked_add(gem_selectors.len())
            .ok_or(SkillCatalogError::LimitExceeded("mapping rows"))?,
    )?;
    for (source, mut targets) in gem_selectors {
        if extension
            .as_ref()
            .is_some_and(|e| e.mappings.lookup(&source).is_some())
        {
            continue; // Preserve the exact prior mapping and its reviewed alias basis.
        }
        let outcome = if targets.len() == 1 {
            MappingOutcome::Mapped {
                target: targets.remove(0),
                basis: MappingBasis::Exact,
            }
        } else {
            if targets.len() > limits.mapping.max_candidates {
                return Err(SkillCatalogError::LimitExceeded("selector candidates"));
            }
            budget.collection("selector candidates", targets.len())?;
            MappingOutcome::Ambiguous {
                candidates: targets,
                issue: key("ambiguous-external-gem-variant"),
            }
        };
        mappings.push(MappingEntry { source, outcome });
    }
    // A cloned registry retains its construction limits; the caller may have
    // supplied tighter limits for this append operation. Recheck the full result.
    registry.validate_limits(limits.mapping)?;
    mappings.sort_by(|a, b| a.source.cmp(&b.source));
    let mut source = source.clone();
    source.files.sort_by(|a, b| a.path.cmp(&b.path));
    let receipt = SkillCatalogReceipt {
        source,
        catalog_digest,
        policy: policy.clone(),
        base_registry: base.identity()?,
        staged_registry: registry.identity()?,
        gem_count: data.gems.len(),
        skill_count: data.skills.len(),
    };
    // Bound the combined staged output before returning any allocation authority.
    digest_owned(
        "owned-fresh-skill-output-v1",
        &(&definitions, &mappings, &roles, &receipt),
        limits.mapping.max_wire_bytes,
    )?;
    Ok(CatalogCompilation {
        output: FreshOwnedSkillCatalog {
            registry,
            definitions,
            mappings,
            roles,
            receipt,
        },
        counts,
    })
}

/// Final binding supplied after schema/mapping package assembly. Receipt hashes
/// are provenance claims, not attestation or proof of registry succession. A host
/// may append other domains after skill compilation, so the final mapping registry
/// need not equal the receipt's staged registry. The exact final mapping digest
/// and schema identity are binding authority here; the host validates/publishes
/// the registry history and reviewed compiler provenance separately. Historical
/// source files must be an immutable matching subset of the final mapping pins;
/// adding another catalog never rewrites the original compilation receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedSkillRolePackageInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub definitions: DataIdentity,
    pub mapping: OwnedContentDigest,
    pub compilation: SkillCatalogReceipt,
    pub roles: Vec<OwnedGemRoleRow>,
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
fn provenance_is_subset(previous: &SourcePin, current: &SourcePin) -> bool {
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
        input
            .compilation
            .source
            .files
            .sort_by(|a, b| a.path.cmp(&b.path));
        budget.pin(&input.compilation.source)?;
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
        input.roles.sort_by(|a, b| a.gem.cmp(&b.gem));
        let mut positions = BTreeMap::new();
        let mut sources = BTreeMap::new();
        let mut mapped_gems = BTreeSet::new();
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
                    if schema.skills.is_complete() && !schema.skills.members.contains(primary) {
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
        }
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
}
