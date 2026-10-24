//! Offline conversion of injected skill identity facts into owned IDs.
//!
//! Fresh compilation and exact same-pin extension share catalog interpretation.
//! New schemas remain Unmapped; extension preserves existing schemas unchanged.
//! Role packages and their runtime validation live in owned_skill_roles.
use crate::{owned_mapping::*, owned_skill_roles::*};
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::{
    owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits, encode_schema_package},
    skill_identities::SkillIdentityCatalog,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod source_construction;
pub use source_construction::verify_owned_skill_catalog_reproduction;
mod reverse_primary;

type Result<T> = std::result::Result<T, SkillCatalogError>;

/// Staged values; assemble them through the existing schema/mapping constructors.
/// The caller publishes the registry only after the entire intended artifact set
/// succeeds and the host's registry compare-and-swap succeeds.
#[derive(Clone, Debug)]
pub struct FreshOwnedSkillCatalog {
    pub registry: OwnedIdRegistry,
    pub definitions: Vec<DefinitionDescriptor>,
    pub mappings: Vec<MappingEntry>,
    pub roles: Vec<OwnedGemRoleRow>,
    pub reverse_primary: OwnedReversePrimaryCensus,
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
    pub reverse_primary: OwnedReversePrimaryCensus,
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
/// Append fresh Gem/Skill definitions without changing `base`. Repeating this
/// operation against the same base is deterministic; applying it to its previous
/// output allocates new IDs and is not a refresh or an identity migration.
pub fn compile_fresh_owned_skill_catalog(
    catalog: &SkillIdentityCatalog,
    base: &OwnedIdRegistrySnapshot,
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
    base: &OwnedIdRegistrySnapshot,
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
        max_semantic_bytes: limits.mapping.max_wire_bytes,
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
            reverse_primary: output.reverse_primary.clone(),
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
            &output.reverse_primary,
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
        reverse_primary: output.reverse_primary,
        receipt: output.receipt,
        counts,
    })
}

fn compile_catalog(
    catalog: &SkillIdentityCatalog,
    base: &OwnedIdRegistrySnapshot,
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
    let mut registry = OwnedIdRegistry::from_snapshot(base.clone());
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
        let (primary, role, materialization) =
            classify_primary(skills.get(gem.primary_effect_id.as_str()), policy);
        roles.push(OwnedGemRoleRow {
            gem: id,
            primary,
            role,
            materialization,
            construction: source_construction::derive(gem, &skills, &mut budget)?,
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
    let reverse_primary = reverse_primary::derive(catalog, &skills, &roles, &mut budget)?;
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
        &(&definitions, &mappings, &roles, &reverse_primary, &receipt),
        limits.mapping.max_wire_bytes,
    )?;
    Ok(CatalogCompilation {
        output: FreshOwnedSkillCatalog {
            registry,
            definitions,
            mappings,
            roles,
            reverse_primary,
            receipt,
        },
        counts,
    })
}

fn classify_primary(
    skill: Option<&(SkillDefId, Option<bool>, Option<bool>)>,
    policy: &SkillCatalogPolicy,
) -> (OwnedPrimarySkill, OwnedGemRole, OwnedGemMaterialization) {
    match skill {
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
    }
}
