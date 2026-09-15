//! Offline conversion of reviewed attribute-choice passives to owned effect recipes.
//!
//! Source text is matched exactly against injected policy. No stat parser, Lua,
//! build fixture, class default, or new definition allocation belongs here.
use crate::{owned_mapping::*, owned_recipe::*, owned_tree_catalog::*};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::{DeclaredSlot, ParameterValue},
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const OWNED_ATTRIBUTE_RECIPE_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeRecipePolicy {
    pub schema_version: u32,
    pub version: OwnedDefinitionKey,
    /// Exact reviewed text of a selectable attribute node, not a parsing pattern.
    pub expected_node_stats: Vec<String>,
    pub lanes: Vec<AttributeLanePolicy>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeLanePolicy {
    pub key: String,
    pub expected_stats: Vec<String>,
    /// Must already have a numeric Actor schema in the supplied recipe.
    pub stat: StatDefId,
    pub value: ParameterValue,
}
#[derive(Clone, Copy, Debug)]
pub struct AttributeRecipeLimits {
    pub max_wire_bytes: usize,
    pub max_nodes: usize,
    pub max_lanes: usize,
    pub max_work: usize,
    pub recipe: OwnedRecipeLimits,
    pub mapping: OwnedMappingLimits,
}
impl Default for AttributeRecipeLimits {
    fn default() -> Self {
        Self {
            max_wire_bytes: 8 * 1024 * 1024,
            max_nodes: 20_000,
            max_lanes: 32,
            max_work: 1_000_000,
            recipe: OwnedRecipeLimits::default(),
            mapping: OwnedMappingLimits::default(),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum AttributeRecipeError {
    #[error("attribute recipe exceeds or has invalid limit: {0}")]
    Limit(&'static str),
    #[error("invalid attribute recipe: {0}")]
    Invalid(&'static str),
    #[error("attribute recipe source, mapping, registry or schema binding differs")]
    Binding,
    #[error("attribute recipe cannot replace existing declarations or effects")]
    Preservation,
    #[error(transparent)]
    Mapping(#[from] OwnedMappingError),
    #[error(transparent)]
    Recipe(#[from] OwnedRecipeError),
    #[error(transparent)]
    Schema(#[from] SchemaPackageError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, AttributeRecipeError>;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeRecipeReceipt {
    pub catalog: OwnedContentDigest,
    pub policy: OwnedContentDigest,
    pub mapping: OwnedContentDigest,
    pub before_definitions: DataIdentity,
    pub after_definitions: DataIdentity,
    pub converted_nodes: usize,
    pub refined_nodes: usize,
    pub changed_program_owners: usize,
    pub work_used: usize,
}
#[derive(Clone, Debug)]
pub struct StagedAttributeRecipe {
    pub successor: OwnedRecipeInput,
    /// Only descriptors whose declaration closure actually changed. The caller
    /// must authorize these through the checked successor refinement policy.
    pub refined: Vec<DefinitionAddress>,
    pub receipt: AttributeRecipeReceipt,
}

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).expect("compiler-owned key")
}
fn text(s: &str) -> SourceComponent {
    SourceComponent::Text(s.to_owned())
}
fn source_node(version: &str, node: &str) -> ExternalOwnerSelector {
    ExternalOwnerSelector::PassiveNode {
        tree_version: text(version),
        node_id: text(node),
        view: SourceComponent::Missing,
    }
}
fn catalog_selector(
    version: &str,
    kind: ExternalCatalogKind,
    name: &str,
    variant: &str,
) -> ExternalSelector {
    ExternalSelector::Catalog {
        kind,
        key: text(name),
        version: text(version),
        variant: text(variant),
    }
}
fn mapped<'a>(
    mapping: &'a OwnedMappingIndex,
    selector: &ExternalSelector,
) -> Result<&'a SchemaSubject> {
    match mapping.lookup(selector) {
        Some(MappingOutcome::Mapped { target, .. }) => Ok(target),
        _ => Err(AttributeRecipeError::Invalid(
            "missing or unresolved exact selector",
        )),
    }
}
fn close<T>(set: &mut DeclaredSet<T>, owner: &SchemaSubject) -> Result<bool> {
    match &set.closure {
        SchemaClosure::Complete => Ok(false),
        SchemaClosure::Partial { gaps } => {
            if gaps.is_empty()
                || gaps.iter().any(|g| {
                    g.subject != *owner
                        || g.facet != SchemaFacet::InputSchema
                        || g.code != key("tree-declarations-not-converted")
                })
            {
                return Err(AttributeRecipeError::Preservation);
            }
            set.closure = SchemaClosure::Complete;
            Ok(true)
        }
    }
}
struct Budget {
    work: usize,
    maximum: usize,
}
impl Budget {
    fn charge(&mut self, n: usize) -> Result<()> {
        self.work = self
            .work
            .checked_add(n)
            .filter(|v| *v <= self.maximum)
            .ok_or(AttributeRecipeError::Limit("work"))?;
        Ok(())
    }
}
struct Lane<'a> {
    policy: &'a AttributeLanePolicy,
    option: OptionDefId,
}
fn program(slot: &DeclaredSlot<ChoiceSlotDefId>, lanes: &[Lane<'_>]) -> RuleProgram {
    let mut nodes = vec![RuleNode {
        id: key("selected"),
        expression: RuleExpression::Read {
            input: key("choice"),
        },
    }];
    let mut effects = Vec::with_capacity(lanes.len());
    for (i, lane) in lanes.iter().enumerate() {
        let option = key(&format!("option-{i}"));
        let selected = key(&format!("selected-{i}"));
        let value = key(&format!("value-{i}"));
        nodes.extend([
            RuleNode {
                id: option.clone(),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Option(lane.option.clone()),
                },
            },
            RuleNode {
                id: selected.clone(),
                expression: RuleExpression::Compare {
                    operation: RuleComparison::Equal,
                    left: key("selected"),
                    right: option,
                },
            },
            RuleNode {
                id: value.clone(),
                expression: RuleExpression::Literal {
                    value: lane.policy.value.clone(),
                },
            },
        ]);
        effects.push(RuleEffect {
            id: key(&format!("contribution-{i}")),
            when: Some(selected),
            effect: RuleEffectKind::Contribute {
                entity: RuleEntity::Player,
                stat: lane.policy.stat.clone(),
                contribution: ContributionKind::Add,
                value,
            },
        });
    }
    RuleProgram {
        id: key("attribute-choice"),
        context: RuleEntityKind::Actor,
        reads: vec![RuleRead {
            id: key("choice"),
            value_type: ComputedValueType::Option,
            source: RuleReadSource::Choice { slot: slot.clone() },
        }],
        nodes,
        effects,
    }
}

/// Convert the complete reviewed attribute family, preserving all physical IDs.
/// A prior nonempty, nonidentical rule program is never silently replaced. This
/// makes reruns idempotent while changed prior mechanics require another explicit
/// migration rather than treating the current policy as universal authority.
pub fn compile_owned_attribute_recipe(
    base: &StagedOwnedRecipe,
    mapping: &OwnedMappingIndex,
    catalog: &TreeCatalogInput,
    policy: &AttributeRecipePolicy,
    limits: AttributeRecipeLimits,
) -> Result<StagedAttributeRecipe> {
    let hard = AttributeRecipeLimits::default();
    for (name, value, ceiling) in [
        ("wire bytes", limits.max_wire_bytes, hard.max_wire_bytes),
        ("nodes", limits.max_nodes, hard.max_nodes),
        ("lanes", limits.max_lanes, hard.max_lanes),
        ("work", limits.max_work, hard.max_work),
    ] {
        if value == 0 || value > ceiling {
            return Err(AttributeRecipeError::Limit(name));
        }
    }
    let catalog_digest =
        digest_owned("owned-attribute-catalog-v1", catalog, limits.max_wire_bytes)?;
    let policy_digest = digest_owned("owned-attribute-policy-v1", policy, limits.max_wire_bytes)?;
    if catalog.schema_version != OWNED_TREE_CATALOG_VERSION
        || policy.schema_version != OWNED_ATTRIBUTE_RECIPE_VERSION
        || catalog.tree_version.is_empty()
        || policy.expected_node_stats.is_empty()
        || policy.expected_node_stats.iter().any(String::is_empty)
    {
        return Err(AttributeRecipeError::Invalid(
            "version, tree or reviewed node text",
        ));
    }
    if catalog.nodes.len() > limits.max_nodes {
        return Err(AttributeRecipeError::Limit("nodes"));
    }
    if policy.lanes.is_empty()
        || policy.lanes.len() > limits.max_lanes
        || catalog.attribute_options.len() != policy.lanes.len()
    {
        return Err(AttributeRecipeError::Invalid("attribute lane membership"));
    }
    mapping.validate_limits(limits.mapping)?;
    if mapping.input().registry != base.registry().identity()?
        || mapping.input().definitions != *base.schema().identity()
        || catalog.source.system != mapping.input().source.system
        || catalog.source.revision != mapping.input().source.revision
        || catalog.source.files.is_empty()
    {
        return Err(AttributeRecipeError::Binding);
    }
    let mut budget = Budget {
        work: 0,
        maximum: limits.max_work,
    };
    budget.charge(catalog.nodes.len() + catalog.attribute_options.len() + policy.lanes.len())?;
    let mut pins = BTreeSet::new();
    for pin in &catalog.source.files {
        budget.charge(mapping.input().source.files.len() + 1)?;
        if !pins.insert(&pin.path) || !mapping.input().source.files.contains(pin) {
            return Err(AttributeRecipeError::Binding);
        }
    }
    let mut sources = BTreeMap::new();
    for lane in &catalog.attribute_options {
        if sources.insert(lane.key.as_str(), lane).is_some() {
            return Err(AttributeRecipeError::Invalid("duplicate source lane"));
        }
    }
    let mut selected = BTreeMap::new();
    for lane in &policy.lanes {
        if lane.key.is_empty()
            || lane.expected_stats.is_empty()
            || lane.expected_stats.iter().any(String::is_empty)
            || selected.insert(lane.key.as_str(), lane).is_some()
        {
            return Err(AttributeRecipeError::Invalid(
                "duplicate or empty reviewed lane",
            ));
        }
    }
    let mut lanes = Vec::with_capacity(selected.len());
    let mut options = BTreeSet::new();
    for (name, lane) in selected {
        if sources
            .get(name)
            .is_none_or(|row| row.stats != lane.expected_stats)
        {
            return Err(AttributeRecipeError::Invalid("unreviewed source lane text"));
        }
        let SchemaSubject::Definition(DefinitionAddress::Option(option)) = mapped(
            mapping,
            &catalog_selector(
                &catalog.tree_version,
                ExternalCatalogKind::Option,
                name,
                "attribute",
            ),
        )?
        else {
            return Err(AttributeRecipeError::Invalid("attribute lane target kind"));
        };
        if !options.insert(option.clone()) {
            return Err(AttributeRecipeError::Invalid("aliased attribute options"));
        }
        let SchemaLookup::Known(stat) = base.schema().definition(&lane.stat) else {
            return Err(AttributeRecipeError::Invalid("unknown target stat"));
        };
        let ty = match &lane.value {
            ParameterValue::Integer(_) => ComputedValueType::Integer,
            ParameterValue::Quantity(v) => ComputedValueType::Quantity {
                unit: v.unit().clone(),
            },
            _ => {
                return Err(AttributeRecipeError::Invalid(
                    "attribute contribution must be numeric",
                ));
            }
        };
        if stat.value != ty || !stat.targets.contains(&RuleEntityKind::Actor) {
            return Err(AttributeRecipeError::Invalid(
                "attribute target stat type or scope",
            ));
        }
        lanes.push(Lane {
            policy: lane,
            option: option.clone(),
        });
    }
    let SchemaSubject::Definition(DefinitionAddress::PointPool(pool)) = mapped(
        mapping,
        &catalog_selector(
            &catalog.tree_version,
            ExternalCatalogKind::PointPool,
            "ordinary",
            "tree-pool",
        ),
    )?
    else {
        return Err(AttributeRecipeError::Invalid("ordinary pool target kind"));
    };
    let mut nodes = BTreeMap::new();
    let mut keys = BTreeSet::new();
    for node in &catalog.nodes {
        if !keys.insert(&node.key) {
            return Err(AttributeRecipeError::Invalid("duplicate physical node key"));
        }
        let TreeNodeKind::Attribute { pool: source_pool } = node.kind else {
            continue;
        };
        if source_pool != TreePoolKind::Ordinary
            || node.stats != policy.expected_node_stats
            || !node.views.is_empty()
            || !node.unlock.is_empty()
        {
            return Err(AttributeRecipeError::Invalid(
                "unreviewed attribute node semantics",
            ));
        }
        let source = source_node(&catalog.tree_version, &node.key);
        let SchemaSubject::Definition(DefinitionAddress::PassiveNode(id)) =
            mapped(mapping, &ExternalSelector::Definition(source.clone()))?
        else {
            return Err(AttributeRecipeError::Invalid("attribute node target kind"));
        };
        let SchemaSubject::Slot(SlotAddress::Choice(slot)) = mapped(
            mapping,
            &ExternalSelector::Slot {
                owner: source,
                kind: ExternalSlotKind::Choice,
                key: text("options"),
            },
        )?
        else {
            return Err(AttributeRecipeError::Invalid(
                "attribute choice target kind",
            ));
        };
        if slot.declaration != SlotOwnerDefId::PassiveNode(id.clone()) {
            return Err(AttributeRecipeError::Invalid(
                "attribute choice owner differs",
            ));
        }
        let SchemaLookup::Known(schema) = base.schema().slot(slot) else {
            return Err(AttributeRecipeError::Invalid("unknown choice slot"));
        };
        let ValueSchema::Option { allowed } = &schema.value else {
            return Err(AttributeRecipeError::Invalid("attribute choice type"));
        };
        if !allowed.is_complete()
            || allowed.members.iter().cloned().collect::<BTreeSet<_>>() != options
            || schema.presence != SlotPresence::RequiredOnce
            || schema.owners != [ChoiceOwnerScope::Allocation]
        {
            return Err(AttributeRecipeError::Invalid(
                "attribute choice domain or scope differs",
            ));
        }
        if nodes.insert(id.clone(), slot.clone()).is_some() {
            return Err(AttributeRecipeError::Invalid(
                "aliased physical attribute nodes",
            ));
        }
    }
    if nodes.is_empty() {
        return Err(AttributeRecipeError::Invalid("no reviewed attribute nodes"));
    }
    budget.charge(base.schema().input().definitions.len() + base.rules().input().owners.len())?;
    let mut schema = base.schema().input().clone();
    let mut refined = Vec::new();
    let mut seen = BTreeSet::new();
    for descriptor in &mut schema.definitions {
        let DefinitionDescriptor::PassiveNode(entry) = descriptor else {
            continue;
        };
        let Some(slot) = nodes.get(&entry.id) else {
            continue;
        };
        let SchemaState::Known(passive) = &mut entry.schema else {
            return Err(AttributeRecipeError::Invalid(
                "attribute schema is unmapped",
            ));
        };
        if !passive.pools.is_complete() || passive.pools.members != [pool.clone()] {
            return Err(AttributeRecipeError::Invalid("attribute pool differs"));
        }
        let declarations = &mut passive.declarations;
        if !declarations.parameters.members.is_empty()
            || declarations.choices.members != [slot.clone()]
            || !declarations.grants.members.is_empty()
            || !declarations.actors.members.is_empty()
            || !declarations.skill_grants.members.is_empty()
            || !declarations.outputs.members.is_empty()
            || !declarations.sockets.members.is_empty()
        {
            return Err(AttributeRecipeError::Preservation);
        }
        let owner = SchemaSubject::Definition(entry.id.address());
        budget.charge(7 + lanes.len() * 5)?;
        let changed = [
            close(&mut declarations.parameters, &owner)?,
            close(&mut declarations.choices, &owner)?,
            close(&mut declarations.grants, &owner)?,
            close(&mut declarations.actors, &owner)?,
            close(&mut declarations.skill_grants, &owner)?,
            close(&mut declarations.outputs, &owner)?,
            close(&mut declarations.sockets, &owner)?,
        ]
        .into_iter()
        .any(|v| v);
        if changed {
            refined.push(entry.id.address());
        }
        seen.insert(entry.id.clone());
    }
    if seen.len() != nodes.len() {
        return Err(AttributeRecipeError::Invalid(
            "missing attribute descriptors",
        ));
    }
    let schema = OwnedDefinitionSchemaPackage::new(schema, limits.recipe.schema)?;
    let mut rules = base.rules().input().clone();
    let mut owners = BTreeSet::new();
    let mut changed_program_owners = 0;
    for row in &mut rules.owners {
        let SchemaSubject::Definition(DefinitionAddress::PassiveNode(id)) = &row.owner else {
            continue;
        };
        let Some(slot) = nodes.get(id) else {
            continue;
        };
        let desired = DeclaredSet::complete(vec![program(slot, &lanes)]);
        if row.programs != desired {
            if !row.programs.members.is_empty()
                || !matches!(&row.programs.closure, SchemaClosure::Partial { gaps } if !gaps.is_empty() && gaps.iter().all(|g| g.subject == row.owner && g.facet == SchemaFacet::GameRules && g.code == key("tree-game-rules-not-converted")))
            {
                return Err(AttributeRecipeError::Preservation);
            }
            row.programs = desired;
            changed_program_owners += 1;
        }
        owners.insert(id.clone());
    }
    if owners.len() != nodes.len() {
        return Err(AttributeRecipeError::Invalid(
            "missing attribute rule owners",
        ));
    }
    rules.definitions = schema.identity().clone();
    let mut routing = base.routing().input().clone();
    routing.definitions = schema.identity().clone();
    let successor = OwnedRecipeInput {
        schema_version: OWNED_RECIPE_VERSION,
        registry: base.registry().input().clone(),
        schema: schema.input().clone(),
        rules,
        routing,
    };
    let checked = assemble_owned_recipe(successor, limits.recipe)?;
    refined.sort();
    let receipt = AttributeRecipeReceipt {
        catalog: catalog_digest,
        policy: policy_digest,
        mapping: *mapping.identity(),
        before_definitions: base.schema().identity().clone(),
        after_definitions: checked.schema().identity().clone(),
        converted_nodes: nodes.len(),
        refined_nodes: refined.len(),
        changed_program_owners,
        work_used: budget.work,
    };
    Ok(StagedAttributeRecipe {
        successor: OwnedRecipeInput {
            schema_version: OWNED_RECIPE_VERSION,
            registry: checked.registry().input().clone(),
            schema: checked.schema().input().clone(),
            rules: checked.rules().input().clone(),
            routing: checked.routing().input().clone(),
        },
        refined,
        receipt,
    })
}
