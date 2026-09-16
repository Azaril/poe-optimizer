//! Offline, finite base identities and explicit attack-profile source presence.
//!
//! A template capability describes source presence only. It proves neither
//! attack compatibility nor activation, numeric coverage, or selected lifecycle.
//! Source acquisition authenticates upstream separately; this converter binds
//! exact supplied catalog bytes and never loads legacy metadata or source code.
use crate::{
    owned_item_lines::*,
    owned_item_source::*,
    owned_mapping::*,
    owned_recipe::*,
    owned_successor::{CatalogAppend, CatalogItemPolicyMode},
};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::ParameterValue,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const OWNED_ITEM_BASE_VERSION: u32 = 1;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemBaseCatalog {
    pub schema_version: u32,
    pub source: SourcePin,
    pub bases: Vec<ItemBaseRow>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemBaseRow {
    pub name: String,
    pub item_type: String,
    pub source_module: String,
    pub weapon_field: ItemBaseWeaponField,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemBaseWeaponField {
    Absent,
    Table,
    /// No Boolean inference is permitted for an unreviewed source shape.
    Unsupported,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemBasePolicy {
    pub schema_version: u32,
    pub version: OwnedDefinitionKey,
    /// SHA-256 of exact catalog bytes, including any final newline.
    pub catalog_sha256: String,
    pub capability: CapabilityDefId,
    /// Exact complete membership of this supplied catalog; IDs are allocated
    /// and descriptors reviewed by the caller before this conversion.
    pub templates: Vec<ItemBaseTemplateBinding>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemBaseTemplateBinding {
    pub source_base: String,
    pub template: ItemTemplateDefId,
    pub header_rule: OwnedDefinitionKey,
    /// New/unreviewed generated buff prefixes must remain Unresolved.
    pub load_index_prefix: ItemLoadIndexPrefix,
}
#[derive(Clone, Copy, Debug)]
pub struct ItemBaseLimits {
    pub max_catalog_bytes: usize,
    pub max_policy_bytes: usize,
    pub max_bases: usize,
    pub max_work: usize,
    pub recipe: OwnedRecipeLimits,
    pub mapping: OwnedMappingLimits,
    pub items: ItemLineLimits,
    pub item_source: ItemSourceLimits,
}
impl Default for ItemBaseLimits {
    fn default() -> Self {
        Self {
            max_catalog_bytes: 8 * 1024 * 1024,
            max_policy_bytes: 8 * 1024 * 1024,
            max_bases: 8192,
            max_work: 64 * 1024 * 1024,
            recipe: OwnedRecipeLimits::default(),
            mapping: OwnedMappingLimits::default(),
            items: ItemLineLimits::default(),
            item_source: ItemSourceLimits::default(),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ItemBaseError {
    #[error("item base exceeds or has invalid limit: {0}")]
    Limit(&'static str),
    #[error("invalid item base catalog or policy: {0}")]
    Invalid(&'static str),
    #[error("item base artifact, provenance or predecessor binding differs")]
    Binding,
    #[error("item base conversion cannot replace prior coverage, identity or writers")]
    Preservation,
    #[error(transparent)]
    Mapping(#[from] OwnedMappingError),
    #[error(transparent)]
    Recipe(#[from] OwnedRecipeError),
    #[error(transparent)]
    Items(#[from] ItemLineError),
    #[error(transparent)]
    Source(#[from] ItemSourceError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, ItemBaseError>;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemBaseReceipt {
    pub catalog_sha256: String,
    pub policy: OwnedContentDigest,
    pub mapping: OwnedContentDigest,
    pub before_definitions: DataIdentity,
    pub after_definitions: DataIdentity,
    pub bases: usize,
    pub present: usize,
    pub absent: usize,
    pub unsupported: usize,
    pub added_mappings: usize,
    pub added_header_rules: usize,
    pub changed_program_owners: usize,
    pub work_used: usize,
}
#[derive(Clone, Debug)]
pub struct StagedItemBaseRecipe {
    pub successor: OwnedRecipeInput,
    pub append: CatalogAppend,
    pub items: ItemLinePolicyInput,
    pub item_source: ItemSourceLayoutPolicyInput,
    pub receipt: ItemBaseReceipt,
}
fn key(text: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(text).expect("compiler-owned key")
}
fn valid_text(text: &str) -> bool {
    !text.is_empty() && text.trim_ascii() == text && !text.chars().any(char::is_control)
}
fn selector(base: &str) -> ExternalSelector {
    // This identifies an exact row of the base catalog. Missing is not a wildcard
    // for an authored unique prototype, variant, or unresolved item lifecycle.
    ExternalSelector::Definition(ExternalOwnerSelector::ItemTemplate {
        base: SourceComponent::Text(base.to_owned()),
        prototype: SourceComponent::Missing,
        variant: SourceComponent::Missing,
    })
}
fn capability_program(capability: &CapabilityDefId, enabled: bool) -> RuleProgram {
    RuleProgram {
        id: key("template-supplies-base-attack-profile"),
        context: RuleEntityKind::EquipmentUse,
        reads: vec![],
        nodes: vec![RuleNode {
            id: key("present"),
            expression: RuleExpression::Literal {
                value: ParameterValue::Boolean(enabled),
            },
        }],
        effects: vec![RuleEffect {
            id: key("base-attack-profile-source"),
            when: None,
            effect: RuleEffectKind::Capability {
                entity: RuleEntity::Current,
                capability: capability.clone(),
                enabled: key("present"),
            },
        }],
    }
}
fn merge_pins(target: &mut SourcePin, source: &SourcePin) -> Result<()> {
    if target.system != source.system || target.revision != source.revision {
        return Err(ItemBaseError::Binding);
    }
    let mut pins: BTreeMap<_, _> = target.files.iter().map(|p| (&p.path, &p.sha256)).collect();
    if pins.len() != target.files.len() {
        return Err(ItemBaseError::Binding);
    }
    for p in &source.files {
        if let Some(old) = pins.insert(&p.path, &p.sha256)
            && old != &p.sha256
        {
            return Err(ItemBaseError::Binding);
        }
    }
    target.files = pins
        .into_iter()
        .map(|(path, sha256)| SourceFilePin {
            path: path.clone(),
            sha256: sha256.clone(),
        })
        .collect();
    Ok(())
}

/// Append only identity headers and one scoped source-presence capability.
/// Existing descriptors, source defaults, declaration membership, rule coverage,
/// versions and unrelated semantics are preserved. Final host publication must
/// pass `append` and these policies through the checked successor transition.
pub fn compile_owned_item_bases(
    base: &StagedOwnedRecipe,
    mapping: &OwnedMappingIndex,
    items: &OwnedItemLinePolicy,
    item_source: &ItemSourceLayoutPolicy,
    catalog_bytes: &[u8],
    policy: &ItemBasePolicy,
    limits: ItemBaseLimits,
) -> Result<StagedItemBaseRecipe> {
    let hard = ItemBaseLimits::default();
    for (name, value, ceiling) in [
        (
            "catalog bytes",
            limits.max_catalog_bytes,
            hard.max_catalog_bytes,
        ),
        (
            "policy bytes",
            limits.max_policy_bytes,
            hard.max_policy_bytes,
        ),
        ("bases", limits.max_bases, hard.max_bases),
        ("work", limits.max_work, hard.max_work),
    ] {
        if value == 0 || value > ceiling {
            return Err(ItemBaseError::Limit(name));
        }
    }
    if catalog_bytes.len() > limits.max_catalog_bytes {
        return Err(ItemBaseError::Limit("catalog bytes"));
    }
    let policy_digest = digest_owned("owned-item-base-policy-v1", policy, limits.max_policy_bytes)?;
    let catalog_sha256 = format!("{:x}", Sha256::digest(catalog_bytes));
    if catalog_sha256 != policy.catalog_sha256 {
        return Err(ItemBaseError::Binding);
    }
    let catalog: ItemBaseCatalog = serde_json::from_slice(catalog_bytes)?;
    if catalog.schema_version != OWNED_ITEM_BASE_VERSION
        || policy.schema_version != OWNED_ITEM_BASE_VERSION
    {
        return Err(ItemBaseError::Invalid("version"));
    }
    if catalog.bases.is_empty()
        || catalog.bases.len() > limits.max_bases
        || policy.templates.len() > limits.max_bases
    {
        return Err(ItemBaseError::Limit("bases"));
    }
    mapping.validate_limits(limits.mapping)?;
    items.verify_bindings(base.schema())?;
    let checked_source = ItemSourceLayoutPolicy::new(
        item_source.input().clone(),
        items,
        base.schema(),
        limits.item_source,
    )?;
    if checked_source.identity() != item_source.identity()
        || mapping.input().registry != base.registry().identity()?
        || mapping.input().definitions != *base.schema().identity()
        || catalog.source.system != ExternalSourceSystem::PathOfBuilding2
        || catalog.source.files.is_empty()
    {
        return Err(ItemBaseError::Binding);
    }
    let SchemaLookup::Known(capability) = base.schema().definition(&policy.capability) else {
        return Err(ItemBaseError::Invalid("unknown capability"));
    };
    if capability.targets != [RuleEntityKind::EquipmentUse] {
        return Err(ItemBaseError::Invalid(
            "capability must target only EquipmentUse",
        ));
    }
    let mut work = 0usize;
    let mut charge = |n: usize| -> Result<()> {
        work = work
            .checked_add(n)
            .filter(|n| *n <= limits.max_work)
            .ok_or(ItemBaseError::Limit("work"))?;
        Ok(())
    };
    charge(catalog.bases.len() + policy.templates.len() + catalog.source.files.len())?;
    let mut source_paths = BTreeSet::new();
    for p in &catalog.source.files {
        if !source_paths.insert(p.path.as_str()) {
            return Err(ItemBaseError::Invalid("duplicate source file"));
        }
    }
    let mut next_mapping = mapping.input().clone();
    merge_pins(&mut next_mapping.source, &catalog.source)?;
    // Reconstruct once to validate all new path/hash syntax, even if no mapping
    // changes are needed. Carried hashes alone do not authenticate derivation.
    OwnedMappingIndex::new(
        next_mapping.clone(),
        base.registry(),
        base.schema(),
        limits.mapping,
    )?;
    let mut bindings = BTreeMap::new();
    let mut targets = BTreeSet::new();
    let mut header_ids = BTreeSet::new();
    for b in &policy.templates {
        if !valid_text(&b.source_base)
            || !targets.insert(b.template.clone())
            || !header_ids.insert(b.header_rule.clone())
            || bindings.insert(b.source_base.as_str(), b).is_some()
            || !matches!(
                base.schema().definition(&b.template),
                SchemaLookup::Known(_)
            )
        {
            return Err(ItemBaseError::Invalid(
                "duplicate, invalid or unknown template binding",
            ));
        }
    }
    let mut rows = BTreeMap::new();
    for row in &catalog.bases {
        if !valid_text(&row.name)
            || !valid_text(&row.item_type)
            || !source_paths.contains(row.source_module.as_str())
            || rows.insert(row.name.as_str(), row).is_some()
        {
            return Err(ItemBaseError::Invalid(
                "duplicate, invalid or unpinned base",
            ));
        }
    }
    if rows.keys().copied().collect::<Vec<_>>() != bindings.keys().copied().collect::<Vec<_>>() {
        return Err(ItemBaseError::Invalid(
            "complete supplied base membership differs",
        ));
    }
    let mut added_mappings = vec![];
    let mut desired = BTreeMap::new();
    let (mut present, mut absent, mut unsupported) = (0, 0, 0);
    let mut next_items = items.input().clone();
    let mut next_source = item_source.input().clone();
    merge_pins(&mut next_source.source, &catalog.source)?;
    charge(
        next_items.rules.len()
            + next_source.rule_layouts.len()
            + next_source.template_layouts.len(),
    )?;
    let mut rule_index: BTreeMap<_, _> = next_items
        .rules
        .iter()
        .enumerate()
        .map(|(i, r)| (r.id.clone(), i))
        .collect();
    let mut role_index: BTreeMap<_, _> = next_source
        .rule_layouts
        .iter()
        .enumerate()
        .map(|(i, r)| (r.rule.clone(), i))
        .collect();
    let mut prefix_index: BTreeMap<_, _> = next_source
        .template_layouts
        .iter()
        .enumerate()
        .map(|(i, r)| (r.template.clone(), i))
        .collect();
    let mut added_header_rules = 0;
    for (name, row) in &rows {
        let binding = bindings[name];
        let target = SchemaSubject::Definition(binding.template.address());
        let entry = MappingEntry {
            source: selector(name),
            outcome: MappingOutcome::Mapped {
                target: target.clone(),
                basis: MappingBasis::Exact,
            },
        };
        match mapping.lookup(&entry.source) {
            None => added_mappings.push(entry.clone()),
            Some(prior) if prior == &entry.outcome => {}
            _ => return Err(ItemBaseError::Preservation),
        }
        let enabled = match row.weapon_field {
            ItemBaseWeaponField::Table => {
                present += 1;
                Some(true)
            }
            ItemBaseWeaponField::Absent => {
                absent += 1;
                Some(false)
            }
            ItemBaseWeaponField::Unsupported => {
                unsupported += 1;
                None
            }
        };
        desired.insert(
            binding.template.clone(),
            enabled.map(|enabled| capability_program(&policy.capability, enabled)),
        );
        let rule = ItemLineRule {
            id: binding.header_rule.clone(),
            pattern: vec![ItemPatternPart::Literal(row.name.clone())],
            captures: vec![],
            emissions: vec![ItemEmission::Template {
                definition: binding.template.clone(),
            }],
        };
        charge(4)?;
        if let Some(i) = rule_index.get(&rule.id) {
            if next_items.rules[*i] != rule {
                return Err(ItemBaseError::Preservation);
            }
        } else {
            rule_index.insert(rule.id.clone(), next_items.rules.len());
            next_items.rules.push(rule);
            added_header_rules += 1;
        }
        let layout = ItemRuleSourceLayout {
            rule: binding.header_rule.clone(),
            role: ItemRuleSourceRole::Header,
        };
        if let Some(i) = role_index.get(&layout.rule) {
            if next_source.rule_layouts[*i] != layout {
                return Err(ItemBaseError::Preservation);
            }
        } else {
            role_index.insert(layout.rule.clone(), next_source.rule_layouts.len());
            next_source.rule_layouts.push(layout);
        }
        let layout = ItemTemplateSourceLayout {
            template: binding.template.clone(),
            load_index_prefix: binding.load_index_prefix,
        };
        if let Some(i) = prefix_index.get(&layout.template) {
            if next_source.template_layouts[*i] != layout {
                return Err(ItemBaseError::Preservation);
            }
        } else {
            prefix_index.insert(layout.template.clone(), next_source.template_layouts.len());
            next_source.template_layouts.push(layout);
        }
    }
    next_mapping.entries.extend(added_mappings.clone());
    OwnedMappingIndex::new(next_mapping, base.registry(), base.schema(), limits.mapping)?;
    let mut rules = base.rules().input().clone();
    let mut seen = BTreeSet::new();
    let mut changed_program_owners = 0;
    for owner in &mut rules.owners {
        let wanted = match &owner.owner {
            SchemaSubject::Definition(DefinitionAddress::ItemTemplate(id)) => desired.get(id),
            _ => None,
        };
        for p in &owner.programs.members {
            charge(p.effects.len() + 1)?;
            if wanted.and_then(Option::as_ref) == Some(p) {
                continue;
            }
            if wanted.is_some() && p.id == key("template-supplies-base-attack-profile")
                || p.effects.iter().any(|e| matches!(&e.effect, RuleEffectKind::Capability { capability, .. } if capability == &policy.capability))
            { return Err(ItemBaseError::Preservation); }
        }
        if let Some(wanted) = wanted {
            if owner.programs.is_complete() {
                return Err(ItemBaseError::Preservation);
            }
            if let Some(p) = wanted
                && !owner.programs.members.contains(p)
            {
                owner.programs.members.push(p.clone());
                changed_program_owners += 1;
            }
            if let SchemaSubject::Definition(DefinitionAddress::ItemTemplate(id)) = &owner.owner {
                seen.insert(id.clone());
            }
        }
    }
    for (id, wanted) in desired {
        if seen.contains(&id) {
            continue;
        }
        let owner = SchemaSubject::Definition(id.address());
        rules.owners.push(DefinitionRules {
            owner: owner.clone(),
            programs: DeclaredSet::partial(
                wanted.into_iter().collect(),
                vec![SchemaGap {
                    subject: owner,
                    facet: SchemaFacet::GameRules,
                    code: key("item-template-rules-unconverted"),
                }],
            ),
        });
        changed_program_owners += 1;
    }
    let successor = OwnedRecipeInput {
        schema_version: OWNED_RECIPE_VERSION,
        registry: base.registry().input().clone(),
        schema: base.schema().input().clone(),
        rules,
        routing: base.routing().input().clone(),
    };
    let checked = assemble_owned_recipe(successor.clone(), limits.recipe)?;
    let checked_items = OwnedItemLinePolicy::new(next_items, checked.schema(), limits.items)?;
    // This also catches overlapping old patterns (including opaque captures),
    // not just duplicate exact literals or IDs. Ambiguity never becomes identity.
    for (name, binding) in bindings {
        charge(
            checked_items
                .input()
                .rules
                .len()
                .saturating_mul(name.len().saturating_add(1)),
        )?;
        let line = checked_items.convert_line(1, name, None)?;
        if !matches!(line.outcome, ItemLineOutcome::Known { ref rule, ref emissions }
            if rule == &binding.header_rule && emissions == &[ConvertedItemEmission::Template { definition: binding.template.clone() }])
        {
            return Err(ItemBaseError::Preservation);
        }
    }
    next_source.item_lines = *checked_items.identity();
    let checked_source = ItemSourceLayoutPolicy::new(
        next_source,
        &checked_items,
        checked.schema(),
        limits.item_source,
    )?;
    let receipt = ItemBaseReceipt {
        catalog_sha256,
        policy: policy_digest,
        mapping: *mapping.identity(),
        before_definitions: base.schema().identity().clone(),
        after_definitions: checked.schema().identity().clone(),
        bases: catalog.bases.len(),
        present,
        absent,
        unsupported,
        added_mappings: added_mappings.len(),
        added_header_rules,
        changed_program_owners,
        work_used: work,
    };
    Ok(StagedItemBaseRecipe {
        successor,
        append: CatalogAppend {
            mappings: added_mappings,
            source: catalog.source,
            item_policies: CatalogItemPolicyMode::SuppliedSuccessor,
        },
        items: checked_items.input().clone(),
        item_source: checked_source.input().clone(),
        receipt,
    })
}
