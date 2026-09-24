//! Offline full-list passive effects, with optional class/ascendancy views.
//! Source selectors/text stop here; runtime rules contain only owned identities.
use crate::{owned_mapping::*, owned_recipe::*, owned_tree_catalog::*};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::ParameterValue,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const OWNED_PASSIVE_VIEWS_VERSION: u32 = 1;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewRecipePolicy {
    pub schema_version: u32,
    pub version: OwnedDefinitionKey,
    pub nodes: Vec<ViewNodePolicy>,
    pub receiver_rules: Vec<DefinitionRules>,
    pub receivers: Vec<ActorStatReceiver>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewNodePolicy {
    pub node: String,
    pub pool: TreePoolKind,
    pub default: ViewEffectPolicy,
    pub views: Vec<ViewBranchPolicy>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewEffectPolicy {
    pub expected_stats: Vec<String>,
    pub contributions: Vec<ViewContribution>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewContribution {
    pub stat: StatDefId,
    pub contribution: ContributionKind,
    pub value: ParameterValue,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewBranchPolicy {
    pub selector: String,
    pub when: ViewSelector,
    pub effects: ViewEffectPolicy,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ViewSelector {
    Class { key: String },
    Ascendancy { class: String, key: String },
}
#[derive(Clone, Copy, Debug)]
pub struct ViewRecipeLimits {
    pub max_wire_bytes: usize,
    pub max_nodes: usize,
    pub max_views: usize,
    pub max_contributions: usize,
    pub max_work: usize,
    pub recipe: OwnedRecipeLimits,
    pub mapping: OwnedMappingLimits,
}
impl Default for ViewRecipeLimits {
    fn default() -> Self {
        Self {
            max_wire_bytes: 8 * 1024 * 1024,
            max_nodes: 20_000,
            max_views: 128,
            max_contributions: 65_536,
            max_work: 1_000_000,
            recipe: OwnedRecipeLimits::default(),
            mapping: OwnedMappingLimits::default(),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ViewRecipeError {
    #[error("passive view recipe exceeds or has invalid limit: {0}")]
    Limit(&'static str),
    #[error("invalid passive view recipe: {0}")]
    Invalid(&'static str),
    #[error("passive view source, mapping, registry or schema binding differs")]
    Binding,
    #[error("passive view recipe cannot replace prior declarations or programs")]
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
type Result<T> = std::result::Result<T, ViewRecipeError>;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewRecipeReceipt {
    pub catalog: OwnedContentDigest,
    pub policy: OwnedContentDigest,
    pub mapping: OwnedContentDigest,
    pub before_definitions: DataIdentity,
    pub after_definitions: DataIdentity,
    pub converted_nodes: usize,
    pub refined_nodes: usize,
    pub changed_program_owners: usize,
    pub added_receiver_owners: usize,
    pub added_receivers: usize,
    pub work_used: usize,
}
#[derive(Clone, Debug)]
pub struct StagedViewRecipe {
    pub successor: OwnedRecipeInput,
    pub refined: Vec<DefinitionAddress>,
    pub receipt: ViewRecipeReceipt,
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).expect("compiler-owned key")
}
fn text(s: &str) -> SourceComponent {
    SourceComponent::Text(s.to_owned())
}
fn mapped<'a>(
    mapping: &'a OwnedMappingIndex,
    selector: &ExternalSelector,
) -> Result<&'a SchemaSubject> {
    match mapping.lookup(selector) {
        Some(MappingOutcome::Mapped { target, .. }) => Ok(target),
        _ => Err(ViewRecipeError::Invalid(
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
                return Err(ViewRecipeError::Preservation);
            }
            set.closure = SchemaClosure::Complete;
            Ok(true)
        }
    }
}
struct Budget {
    work: usize,
    contributions: usize,
    limits: ViewRecipeLimits,
}
impl Budget {
    fn charge(&mut self, n: usize) -> Result<()> {
        self.work = self
            .work
            .checked_add(n)
            .filter(|v| *v <= self.limits.max_work)
            .ok_or(ViewRecipeError::Limit("work"))?;
        Ok(())
    }
    fn effects(
        &mut self,
        branch: &ViewEffectPolicy,
        source: &[String],
        base: &StagedOwnedRecipe,
    ) -> Result<()> {
        self.charge(source.len() + branch.contributions.len() + 1)?;
        if branch.expected_stats != source
            || branch.expected_stats.iter().any(String::is_empty)
            || branch.expected_stats.is_empty() != branch.contributions.is_empty()
        {
            return Err(ViewRecipeError::Invalid(
                "unreviewed or omitted full stat list",
            ));
        }
        self.contributions = self
            .contributions
            .checked_add(branch.contributions.len())
            .filter(|v| *v <= self.limits.max_contributions)
            .ok_or(ViewRecipeError::Limit("contributions"))?;
        for contribution in &branch.contributions {
            let SchemaLookup::Known(stat) = base.schema().definition(&contribution.stat) else {
                return Err(ViewRecipeError::Invalid("unknown contribution stat"));
            };
            let ty = match &contribution.value {
                ParameterValue::Integer(_) => ComputedValueType::Integer,
                ParameterValue::Quantity(v) => ComputedValueType::Quantity {
                    unit: v.unit().clone(),
                },
                _ => return Err(ViewRecipeError::Invalid("contribution must be numeric")),
            };
            if !matches!(
                &stat.value,
                ComputedValueType::Integer | ComputedValueType::Quantity { .. }
            ) || !stat.targets.contains(&RuleEntityKind::Actor)
            {
                return Err(ViewRecipeError::Invalid("contribution type or target"));
            }
            match contribution.contribution {
                ContributionKind::Add if stat.value != ty => {
                    return Err(ViewRecipeError::Invalid("Add contribution type differs"));
                }
                ContributionKind::Increase | ContributionKind::Multiply => {
                    let ComputedValueType::Quantity { unit } = &ty else {
                        return Err(ViewRecipeError::Invalid(
                            "percentage or multiplier quantity required",
                        ));
                    };
                    let SchemaLookup::Known(unit) = base.schema().definition(unit) else {
                        return Err(ViewRecipeError::Invalid("unknown contribution unit"));
                    };
                    let expected = if contribution.contribution == ContributionKind::Increase {
                        UnitDimension::PercentagePoints
                    } else {
                        UnitDimension::DimensionlessFactor
                    };
                    if unit.dimension != expected {
                        return Err(ViewRecipeError::Invalid(
                            "contribution unit dimension differs",
                        ));
                    }
                }
                ContributionKind::Add => {}
            }
        }
        Ok(())
    }
}
struct Branch<'a> {
    policy: &'a ViewBranchPolicy,
    read: RuleReadSource,
    class: bool,
}
fn emit(
    program: &mut RuleProgram,
    branch: &ViewEffectPolicy,
    guard: OwnedDefinitionKey,
    prefix: &str,
) {
    for (i, c) in branch.contributions.iter().enumerate() {
        let id = key(&format!("{prefix}-value-{i}"));
        program.nodes.push(RuleNode {
            id: id.clone(),
            expression: RuleExpression::Literal {
                value: c.value.clone(),
            },
        });
        program.effects.push(RuleEffect {
            id: key(&format!("{prefix}-effect-{i}")),
            when: Some(guard.clone()),
            effect: RuleEffectKind::Contribute {
                entity: RuleEntity::Player,
                stat: c.stat.clone(),
                contribution: c.contribution,
                value: id,
            },
        });
    }
}
fn program(default: &ViewEffectPolicy, branches: &[Branch<'_>]) -> RuleProgram {
    let mut p = RuleProgram {
        id: key("passive-view"),
        context: RuleEntityKind::Actor,
        reads: vec![],
        nodes: vec![],
        effects: vec![],
    };
    let mut classes = vec![];
    let mut asc = vec![];
    for (i, b) in branches.iter().enumerate() {
        let id = key(&format!("match-{i}"));
        p.reads.push(RuleRead {
            id: id.clone(),
            value_type: ComputedValueType::Boolean,
            source: b.read.clone(),
        });
        p.nodes.push(RuleNode {
            id: id.clone(),
            expression: RuleExpression::Read { input: id.clone() },
        });
        if b.class {
            classes.push(id);
        } else {
            asc.push(id);
        }
    }
    p.nodes.extend([
        RuleNode {
            id: key("false"),
            expression: RuleExpression::Literal {
                value: ParameterValue::Boolean(false),
            },
        },
        RuleNode {
            id: key("any-class"),
            expression: RuleExpression::Any { values: classes },
        },
        RuleNode {
            id: key("any-ascendancy"),
            expression: RuleExpression::Any { values: asc },
        },
        RuleNode {
            id: key("no-ascendancy"),
            expression: RuleExpression::Not {
                value: key("any-ascendancy"),
            },
        },
        RuleNode {
            id: key("default"),
            expression: RuleExpression::Select {
                condition: key("any-class"),
                when_true: key("false"),
                when_false: key("no-ascendancy"),
            },
        },
    ]);
    emit(&mut p, default, key("default"), "default");
    for (i, b) in branches.iter().enumerate() {
        let matched = key(&format!("match-{i}"));
        let guard = if b.class {
            matched
        } else {
            let guard = key(&format!("selected-{i}"));
            p.nodes.push(RuleNode {
                id: guard.clone(),
                expression: RuleExpression::Select {
                    condition: key("any-class"),
                    when_true: key("false"),
                    when_false: matched,
                },
            });
            guard
        };
        emit(&mut p, &b.policy.effects, guard, &format!("view-{i}"));
    }
    p
}
fn selector(
    mapping: &OwnedMappingIndex,
    policy: &ViewSelector,
) -> Result<(RuleReadSource, bool, DefinitionAddress)> {
    match policy {
        ViewSelector::Class { key: k } => {
            if k.is_empty() {
                return Err(ViewRecipeError::Invalid("empty class selector"));
            }
            let SchemaSubject::Definition(DefinitionAddress::Class(id)) = mapped(
                mapping,
                &ExternalSelector::Definition(ExternalOwnerSelector::Class { key: text(k) }),
            )?
            else {
                return Err(ViewRecipeError::Invalid("class selector kind"));
            };
            Ok((
                RuleReadSource::CharacterClassIs { class: id.clone() },
                true,
                id.address(),
            ))
        }
        ViewSelector::Ascendancy { class, key: k } => {
            if class.is_empty() || k.is_empty() {
                return Err(ViewRecipeError::Invalid("empty ascendancy selector"));
            }
            let SchemaSubject::Definition(DefinitionAddress::Ascendancy(id)) = mapped(
                mapping,
                &ExternalSelector::Definition(ExternalOwnerSelector::Ascendancy {
                    class: text(class),
                    key: text(k),
                }),
            )?
            else {
                return Err(ViewRecipeError::Invalid("ascendancy selector kind"));
            };
            Ok((
                RuleReadSource::CharacterAscendancyIs {
                    ascendancy: id.clone(),
                },
                false,
                id.address(),
            ))
        }
    }
}

/// Lower fully reviewed default and optional view lists. No unlisted passive is closed.
/// Source provenance and publication stay with the host's checked tree successor.
pub fn compile_owned_passive_views(
    base: &StagedOwnedRecipe,
    mapping: &OwnedMappingIndex,
    catalog: &TreeCatalogInput,
    policy: &ViewRecipePolicy,
    limits: ViewRecipeLimits,
) -> Result<StagedViewRecipe> {
    let h = ViewRecipeLimits::default();
    for (name, v, max) in [
        ("wire bytes", limits.max_wire_bytes, h.max_wire_bytes),
        ("nodes", limits.max_nodes, h.max_nodes),
        ("views", limits.max_views, h.max_views),
        (
            "contributions",
            limits.max_contributions,
            h.max_contributions,
        ),
        ("work", limits.max_work, h.max_work),
    ] {
        if v == 0 || v > max {
            return Err(ViewRecipeError::Limit(name));
        }
    }
    let catalog_digest = digest_owned(
        "owned-passive-view-catalog-v1",
        catalog,
        limits.max_wire_bytes,
    )?;
    let policy_digest = digest_owned(
        "owned-passive-view-policy-v1",
        policy,
        limits.max_wire_bytes,
    )?;
    if catalog.schema_version != OWNED_TREE_CATALOG_VERSION
        || policy.schema_version != OWNED_PASSIVE_VIEWS_VERSION
        || catalog.tree_version.is_empty()
        || policy.nodes.is_empty()
    {
        return Err(ViewRecipeError::Invalid("version or empty conversion"));
    }
    if catalog.nodes.len() > limits.max_nodes || policy.nodes.len() > limits.max_nodes {
        return Err(ViewRecipeError::Limit("nodes"));
    }
    mapping.validate_limits(limits.mapping)?;
    if mapping.input().registry != base.registry().identity()?
        || mapping.input().definitions != *base.schema().identity()
        || catalog.source.system != mapping.input().source.system
        || catalog.source.revision != mapping.input().source.revision
        || catalog.source.files.is_empty()
    {
        return Err(ViewRecipeError::Binding);
    }
    let mut budget = Budget {
        work: 0,
        contributions: 0,
        limits,
    };
    budget.charge(
        catalog.nodes.len()
            + policy.nodes.len()
            + base.schema().input().definitions.len()
            + base.rules().input().owners.len(),
    )?;
    let mut pins = BTreeSet::new();
    for pin in &catalog.source.files {
        budget.charge(mapping.input().source.files.len() + 1)?;
        if !pins.insert(&pin.path) || !mapping.input().source.files.contains(pin) {
            return Err(ViewRecipeError::Binding);
        }
    }
    let mut source_nodes = BTreeMap::new();
    for n in &catalog.nodes {
        if source_nodes.insert(n.key.as_str(), n).is_some() {
            return Err(ViewRecipeError::Invalid("duplicate source node"));
        }
    }
    let mut requested = BTreeSet::new();
    let mut programs = BTreeMap::new();
    let mut pools = BTreeMap::new();
    for node in &policy.nodes {
        if !requested.insert(&node.node) {
            return Err(ViewRecipeError::Invalid("duplicate policy node"));
        }
        let Some(source) = source_nodes.get(node.node.as_str()) else {
            return Err(ViewRecipeError::Invalid("missing source node"));
        };
        if !matches!(source.kind,TreeNodeKind::Allocation{pool} if pool==node.pool)
            || !source.unlock.is_empty()
            // An empty ordinary stat list does not certify a no-effect node:
            // sockets and other deferred semantics may have no display lines.
            || (source.views.is_empty() && source.stats.is_empty())
        {
            return Err(ViewRecipeError::Invalid(
                "unreviewed passive shape or unlock",
            ));
        }
        if source.views.len() > limits.max_views || node.views.len() > limits.max_views {
            return Err(ViewRecipeError::Limit("views"));
        }
        if source.views.len() != node.views.len() {
            return Err(ViewRecipeError::Invalid("view membership differs"));
        }
        budget.charge(source.views.len() + node.views.len())?;
        let mut views = BTreeMap::new();
        for view in &source.views {
            if views.insert(view.selector.as_str(), view).is_some() {
                return Err(ViewRecipeError::Invalid("duplicate source view"));
            }
        }
        let mut view_policies = BTreeMap::new();
        for view in &node.views {
            if view_policies.insert(view.selector.as_str(), view).is_some() {
                return Err(ViewRecipeError::Invalid("duplicate policy view"));
            }
        }
        budget.effects(&node.default, &source.stats, base)?;
        let mut branches = Vec::new();
        let mut predicates = BTreeSet::new();
        for (label, view) in view_policies {
            let Some(raw) = views.get(label) else {
                return Err(ViewRecipeError::Invalid("unknown source view selector"));
            };
            budget.effects(&view.effects, &raw.stats, base)?;
            let (read, class, address) = selector(mapping, &view.when)?;
            if !predicates.insert(address) {
                return Err(ViewRecipeError::Invalid("aliased view predicate"));
            }
            branches.push(Branch {
                policy: view,
                read,
                class,
            });
        }
        let SchemaSubject::Definition(DefinitionAddress::PassiveNode(id)) = mapped(
            mapping,
            &ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode {
                tree_version: text(&catalog.tree_version),
                node_id: text(&node.node),
                view: SourceComponent::Missing,
            }),
        )?
        else {
            return Err(ViewRecipeError::Invalid("passive selector kind"));
        };
        let pool_name = match node.pool {
            TreePoolKind::Ordinary => "ordinary",
            TreePoolKind::Ascendancy => "ascendancy",
        };
        let SchemaSubject::Definition(DefinitionAddress::PointPool(pool)) = mapped(
            mapping,
            &ExternalSelector::Catalog {
                kind: ExternalCatalogKind::PointPool,
                key: text(pool_name),
                version: text(&catalog.tree_version),
                variant: text("tree-pool"),
            },
        )?
        else {
            return Err(ViewRecipeError::Invalid("pool selector kind"));
        };
        if programs
            .insert(id.clone(), program(&node.default, &branches))
            .is_some()
        {
            return Err(ViewRecipeError::Invalid("aliased physical node"));
        }
        pools.insert(id.clone(), pool.clone());
    }
    let mut schema = base.schema().input().clone();
    let mut refined = Vec::new();
    let mut seen = BTreeSet::new();
    for d in &mut schema.definitions {
        let DefinitionDescriptor::PassiveNode(entry) = d else {
            continue;
        };
        let Some(pool) = pools.get(&entry.id) else {
            continue;
        };
        let SchemaState::Known(passive) = &mut entry.schema else {
            return Err(ViewRecipeError::Invalid("unmapped passive schema"));
        };
        if !passive.pools.is_complete() || passive.pools.members != [pool.clone()] {
            return Err(ViewRecipeError::Invalid("passive pool differs"));
        }
        let ports = &mut passive.declarations;
        if !ports.parameters.members.is_empty()
            || !ports.choices.members.is_empty()
            || !ports.grants.members.is_empty()
            || !ports.actors.members.is_empty()
            || !ports.skill_grants.members.is_empty()
            || !ports.outputs.members.is_empty()
            || !ports.sockets.members.is_empty()
        {
            return Err(ViewRecipeError::Preservation);
        }
        let owner = SchemaSubject::Definition(entry.id.address());
        budget.charge(7)?;
        let changed = [
            close(&mut ports.parameters, &owner)?,
            close(&mut ports.choices, &owner)?,
            close(&mut ports.grants, &owner)?,
            close(&mut ports.actors, &owner)?,
            close(&mut ports.skill_grants, &owner)?,
            close(&mut ports.outputs, &owner)?,
            close(&mut ports.sockets, &owner)?,
        ]
        .into_iter()
        .any(|v| v);
        if changed {
            refined.push(entry.id.address());
        }
        seen.insert(entry.id.clone());
    }
    if seen.len() != programs.len() {
        return Err(ViewRecipeError::Invalid("missing passive schemas"));
    }
    let schema = OwnedDefinitionSchemaPackage::new(schema, limits.recipe.schema)?;
    let mut rules = base.rules().input().clone();
    let mut changed_program_owners = 0;
    let mut seen = BTreeSet::new();
    for row in &mut rules.owners {
        let SchemaSubject::Definition(DefinitionAddress::PassiveNode(id)) = &row.owner else {
            continue;
        };
        let Some(p) = programs.get(id) else {
            continue;
        };
        let desired = DeclaredSet::complete(vec![p.clone()]);
        if row.programs != desired {
            if !row.programs.members.is_empty()
                || !matches!(&row.programs.closure,SchemaClosure::Partial{gaps} if !gaps.is_empty()&&gaps.iter().all(|g|g.subject==row.owner&&g.facet==SchemaFacet::GameRules&&g.code==key("tree-game-rules-not-converted")))
            {
                return Err(ViewRecipeError::Preservation);
            }
            row.programs = desired;
            changed_program_owners += 1;
        }
        seen.insert(id.clone());
    }
    if seen.len() != programs.len() {
        return Err(ViewRecipeError::Invalid("missing passive rule owners"));
    }
    let mut added_receiver_owners = 0;
    let mut extra_owners = BTreeSet::new();
    for row in &policy.receiver_rules {
        let SchemaSubject::Definition(DefinitionAddress::Stat(stat)) = &row.owner else {
            return Err(ViewRecipeError::Invalid("receiver rule owner must be Stat"));
        };
        if !extra_owners.insert(stat) {
            return Err(ViewRecipeError::Invalid("duplicate receiver rule owner"));
        }
        budget.charge(rules.owners.len() + 1)?;
        match rules.owners.iter().find(|r| r.owner == row.owner) {
            Some(prior) if prior != row => return Err(ViewRecipeError::Preservation),
            Some(_) => {}
            None => {
                rules.owners.push(row.clone());
                added_receiver_owners += 1;
            }
        }
    }
    let mut added_receivers = 0;
    let mut receiver_ids = BTreeSet::new();
    for receiver in &policy.receivers {
        if !receiver_ids.insert(&receiver.id) {
            return Err(ViewRecipeError::Invalid("duplicate receiver id"));
        }
        budget.charge(rules.receivers.members.len() + 1)?;
        match rules.receivers.members.iter().find(|r| r.id == receiver.id) {
            Some(prior) if prior != receiver => return Err(ViewRecipeError::Preservation),
            Some(_) => {}
            None => {
                rules.receivers.members.push(receiver.clone());
                added_receivers += 1;
            }
        }
    }
    rules.definitions = schema.identity().clone();
    // This converter needs v7 predicates only. Preserve later compatible inputs
    // and unchanged v7 output when newer operation sets become available.
    if rules.operations_version.as_str() == OWNED_RULE_OPERATIONS_V6 {
        rules.operations_version = key(OWNED_RULE_OPERATIONS_V7);
    }
    let mut routing = base.routing().input().clone();
    routing.definitions = schema.identity().clone();
    let checked = assemble_owned_recipe(
        OwnedRecipeInput {
            schema_version: OWNED_RECIPE_VERSION,
            registry: base.registry().input().clone(),
            schema: schema.input().clone(),
            rules,
            routing,
        },
        limits.recipe,
    )?;
    refined.sort();
    let receipt = ViewRecipeReceipt {
        catalog: catalog_digest,
        policy: policy_digest,
        mapping: *mapping.identity(),
        before_definitions: base.schema().identity().clone(),
        after_definitions: checked.schema().identity().clone(),
        converted_nodes: programs.len(),
        refined_nodes: refined.len(),
        changed_program_owners,
        added_receiver_owners,
        added_receivers,
        work_used: budget.work,
    };
    Ok(StagedViewRecipe {
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
