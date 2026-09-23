//! Offline lowering of canonical numeric components to ordinary owned rules.
//! Source encoding, range selection and cache history are Import admission work;
//! this compiler consumes explicit unrounded inputs and preserves coverage gaps.
use crate::{owned_recipe::*, owned_recipe_extension::*};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::{DeclaredSlot, ParameterValue},
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModifierValuePolicy {
    pub schema_version: u32,
    pub version: OwnedDefinitionKey,
    pub definitions: DataIdentity,
    pub factor_unit: UnitDefId,
    pub bindings: Vec<ModifierValueBinding>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModifierValueBinding {
    pub modifier: ModifierDefId,
    pub program: OwnedDefinitionKey,
    /// Required canonical, unrounded numeric component. Existing nominal or
    /// baked source values must not be relabelled as this input.
    pub input: DeclaredSlot<ParameterSlotDefId>,
    pub unit: UnitDefId,
    pub output: StatDefId,
    /// Positive internal multiplier, bounded to 1..=1_000_000.
    pub precision: u32,
    /// Explicit final decimal rounding, bounded to 0..=12.
    pub display_precision: u8,
    pub sign: ModifierValueSign,
    pub scaling: ModifierValueScaling,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ModifierValueSign {
    /// The numeric component retains its sign throughout all numeric stages.
    Direct,
    /// The canonical input must be nonnegative. Import normalizes textual
    /// negative prefixes and qualifier antonyms before assigning these inputs.
    Qualifier {
        negative: DeclaredSlot<ParameterSlotDefId>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ModifierValueScaling {
    /// Component-level non-scalability, distinct from a line's unscalable flag.
    Unscaled,
    Scaled {
        corrupted_base: StatDefId,
        /// Final ordered factor, not the catalyst-only intermediate. Its
        /// producer and normal graph completeness checks establish authority.
        magnitude: StatDefId,
    },
}
#[derive(Clone, Copy, Debug)]
pub struct ModifierValueRecipeLimits {
    pub max_policy_bytes: usize,
    pub max_bindings: usize,
    pub max_nodes: usize,
    pub max_work: usize,
    pub extension: RecipeExtensionLimits,
}
impl Default for ModifierValueRecipeLimits {
    fn default() -> Self {
        Self {
            max_policy_bytes: 2 * 1024 * 1024,
            max_bindings: 4096,
            max_nodes: 262_144,
            max_work: 2_000_000,
            extension: Default::default(),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ModifierValueRecipeError {
    #[error("modifier value compiler limit: {0}")]
    Limit(&'static str),
    #[error("invalid modifier value policy: {0}")]
    Invalid(&'static str),
    #[error("modifier value policy schema binding differs")]
    Binding,
    #[error(transparent)]
    Extension(#[from] RecipeExtensionError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, ModifierValueRecipeError>;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModifierValueRecipeReceipt {
    pub policy: OwnedContentDigest,
    pub bindings: usize,
    pub owners: usize,
    pub nodes: usize,
    pub work_used: usize,
    pub extension: RecipeExtensionReceipt,
}
#[derive(Debug)]
pub struct StagedModifierValueRecipe {
    pub successor: OwnedRecipeInput,
    pub extension: OwnedRecipeExtension,
    pub receipt: ModifierValueRecipeReceipt,
}
fn invalid(message: &'static str) -> ModifierValueRecipeError {
    ModifierValueRecipeError::Invalid(message)
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).expect("fixed compiler key")
}
struct Budget {
    work: usize,
    nodes: usize,
}
impl Budget {
    fn charge(&mut self, count: usize) -> Result<()> {
        self.work = self
            .work
            .checked_sub(count)
            .ok_or(ModifierValueRecipeError::Limit("work"))?;
        Ok(())
    }
    fn node(&mut self) -> Result<()> {
        self.nodes = self
            .nodes
            .checked_sub(1)
            .ok_or(ModifierValueRecipeError::Limit("nodes"))?;
        self.charge(1)
    }
}
fn checked_parameter<'a>(
    base: &'a StagedOwnedRecipe,
    owner: &ModifierDefId,
    slot: &DeclaredSlot<ParameterSlotDefId>,
    budget: &mut Budget,
) -> Result<&'a ParameterSlotSchema> {
    let SchemaLookup::Known(modifier) = base.schema().definition(owner) else {
        return Err(invalid("modifier owner must be Known"));
    };
    budget.charge(modifier.declarations.parameters.members.len() + 1)?;
    if slot.declaration != SlotOwnerDefId::Modifier(owner.clone())
        || !modifier.declarations.parameters.members.contains(slot)
    {
        return Err(invalid(
            "parameter must be declared by its exact modifier owner",
        ));
    }
    let SchemaLookup::Known(schema) = base.schema().slot(slot) else {
        return Err(invalid("parameter must be Known"));
    };
    if schema.presence != SlotPresence::RequiredOnce
        || schema.sites != [ParameterSite::ModifierRoll]
    {
        return Err(invalid("parameter must be a required ModifierRoll"));
    }
    Ok(schema)
}
fn checked_stat(base: &StagedOwnedRecipe, stat: &StatDefId, unit: &UnitDefId) -> Result<()> {
    let SchemaLookup::Known(schema) = base.schema().definition(stat) else {
        return Err(invalid("stat must be Known"));
    };
    if schema.targets != [RuleEntityKind::Modifier]
        || schema.value != (ComputedValueType::Quantity { unit: unit.clone() })
    {
        return Err(invalid(
            "stat must have exact quantity unit and Modifier scope",
        ));
    }
    Ok(())
}
fn validate_binding(
    base: &StagedOwnedRecipe,
    policy: &ModifierValuePolicy,
    binding: &ModifierValueBinding,
    budget: &mut Budget,
) -> Result<()> {
    if !(1..=1_000_000).contains(&binding.precision) || binding.display_precision > 12 {
        return Err(invalid("precision outside supported bounds"));
    }
    if !matches!(
        base.schema().definition(&binding.unit),
        SchemaLookup::Known(_)
    ) {
        return Err(invalid("component unit must be Known"));
    }
    let input = checked_parameter(base, &binding.modifier, &binding.input, budget)?;
    let ValueSchema::Quantity(range) = &input.value else {
        return Err(invalid("canonical input must be a quantity"));
    };
    if range.minimum.unit() != &binding.unit || range.maximum.unit() != &binding.unit {
        return Err(invalid("canonical input unit differs"));
    }
    if let ModifierValueSign::Qualifier { negative } = &binding.sign {
        if range.minimum.value() < 0.0 {
            return Err(invalid("qualifier input must have a nonnegative range"));
        }
        if checked_parameter(base, &binding.modifier, negative, budget)?.value
            != ValueSchema::Boolean
        {
            return Err(invalid("qualifier sign must be Boolean"));
        }
    }
    checked_stat(base, &binding.output, &binding.unit)?;
    if let ModifierValueScaling::Scaled {
        corrupted_base,
        magnitude,
    } = &binding.scaling
    {
        checked_stat(base, corrupted_base, &policy.factor_unit)?;
        checked_stat(base, magnitude, &policy.factor_unit)?;
    }
    Ok(())
}
struct Program<'a> {
    value: RuleProgram,
    unit: &'a UnitDefId,
    factor: &'a UnitDefId,
    budget: &'a mut Budget,
}
impl Program<'_> {
    fn node(&mut self, id: &str, expression: RuleExpression) -> Result<OwnedDefinitionKey> {
        self.budget.node()?;
        let id = key(id);
        self.value.nodes.push(RuleNode {
            id: id.clone(),
            expression,
        });
        Ok(id)
    }
    fn quantity(&mut self, id: &str, value: f64, factor: bool) -> Result<OwnedDefinitionKey> {
        let unit = if factor { self.factor } else { self.unit };
        self.node(
            id,
            RuleExpression::Literal {
                value: ParameterValue::Quantity(
                    FiniteQuantity::new(value, unit.clone())
                        .map_err(|_| invalid("compiler quantity"))?,
                ),
            },
        )
    }
    fn read(
        &mut self,
        id: &str,
        source: RuleReadSource,
        value_type: ComputedValueType,
    ) -> Result<OwnedDefinitionKey> {
        self.budget.charge(1)?;
        self.value.reads.push(RuleRead {
            id: key(id),
            value_type,
            source,
        });
        self.node(id, RuleExpression::Read { input: key(id) })
    }
    fn symmetric(&mut self, stage: &str, value: OwnedDefinitionKey) -> Result<OwnedDefinitionKey> {
        let negative = self.node(
            &format!("{stage}-negative"),
            RuleExpression::Compare {
                operation: RuleComparison::Less,
                left: value.clone(),
                right: key("zero"),
            },
        )?;
        let positive = self.node(
            &format!("{stage}-plus-half"),
            RuleExpression::Add {
                left: value.clone(),
                right: key("half"),
            },
        )?;
        let negative_value = self.node(
            &format!("{stage}-minus-half"),
            RuleExpression::Subtract {
                left: value,
                right: key("half"),
            },
        )?;
        let quantum =
            FiniteQuantity::new(1.0, self.unit.clone()).map_err(|_| invalid("compiler quantum"))?;
        let positive = self.node(
            &format!("{stage}-floor"),
            RuleExpression::Round {
                value: positive,
                quantum: quantum.clone(),
                mode: RuleRounding::Floor,
            },
        )?;
        let negative_value = self.node(
            &format!("{stage}-ceiling"),
            RuleExpression::Round {
                value: negative_value,
                quantum,
                mode: RuleRounding::Ceiling,
            },
        )?;
        self.node(
            stage,
            RuleExpression::Select {
                condition: negative,
                when_true: negative_value,
                when_false: positive,
            },
        )
    }
    fn scalar(
        &mut self,
        stage: &str,
        value: OwnedDefinitionKey,
        stat: &StatDefId,
        corrupt: bool,
    ) -> Result<OwnedDefinitionKey> {
        let scalar = self.read(
            &format!("{stage}-factor"),
            RuleReadSource::Stat {
                entity: RuleEntity::Modifier,
                stat: stat.clone(),
            },
            ComputedValueType::Quantity {
                unit: self.factor.clone(),
            },
        )?;
        let unity = self.node(
            &format!("{stage}-unity"),
            RuleExpression::Compare {
                operation: RuleComparison::Equal,
                left: scalar.clone(),
                right: key("one-factor"),
            },
        )?;
        let scaled = self.node(
            &format!("{stage}-scaled"),
            RuleExpression::Scale {
                value: value.clone(),
                factor: scalar,
            },
        )?;
        let scaled = if corrupt {
            self.node(
                &format!("{stage}-plus-half"),
                RuleExpression::Add {
                    left: scaled,
                    right: key("half"),
                },
            )?
        } else {
            scaled
        };
        let truncated = self.node(
            &format!("{stage}-truncated"),
            RuleExpression::Round {
                value: scaled,
                quantum: FiniteQuantity::new(1.0, self.unit.clone())
                    .map_err(|_| invalid("compiler quantum"))?,
                mode: RuleRounding::Truncate,
            },
        )?;
        self.node(
            stage,
            RuleExpression::Select {
                condition: unity,
                when_true: value,
                when_false: truncated,
            },
        )
    }
}
fn program(
    policy: &ModifierValuePolicy,
    binding: &ModifierValueBinding,
    budget: &mut Budget,
) -> Result<RuleProgram> {
    let mut p = Program {
        value: RuleProgram {
            id: binding.program.clone(),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![],
            nodes: vec![],
            effects: vec![],
        },
        unit: &binding.unit,
        factor: &policy.factor_unit,
        budget,
    };
    p.quantity("zero", 0.0, false)?;
    p.quantity("half", 0.5, false)?;
    let precision = p.quantity("precision", f64::from(binding.precision), true)?;
    let input = p.read(
        "component",
        RuleReadSource::Parameter {
            slot: binding.input.clone(),
        },
        ComputedValueType::Quantity {
            unit: binding.unit.clone(),
        },
    )?;
    let internal = p.node(
        "internal-unrounded",
        RuleExpression::Scale {
            value: input,
            factor: precision.clone(),
        },
    )?;
    let mut value = p.symmetric("internal", internal)?;
    if let ModifierValueScaling::Scaled {
        corrupted_base,
        magnitude,
    } = &binding.scaling
    {
        p.quantity("one-factor", 1.0, true)?;
        value = p.scalar("corruption", value, corrupted_base, true)?;
        value = p.scalar("magnitude", value, magnitude, false)?;
    }
    let value = p.node(
        "display-unrounded",
        RuleExpression::DivideFactor {
            value,
            divisor: precision,
        },
    )?;
    // Validated before program allocation; keep multiplication and division as
    // separate operations rather than replacing either with a reciprocal.
    let display = p.quantity(
        "display-factor",
        10.0_f64.powi(i32::from(binding.display_precision)),
        true,
    )?;
    let value = p.node(
        "display-ticks",
        RuleExpression::Scale {
            value,
            factor: display.clone(),
        },
    )?;
    let value = p.symmetric("display-rounded", value)?;
    let mut value = p.node(
        "formatted",
        RuleExpression::DivideFactor {
            value,
            divisor: display,
        },
    )?;
    if let ModifierValueSign::Qualifier { negative } = &binding.sign {
        let negative = p.read(
            "negative",
            RuleReadSource::Parameter {
                slot: negative.clone(),
            },
            ComputedValueType::Boolean,
        )?;
        let minus_one = p.quantity("minus-one-factor", -1.0, true)?;
        let negated = p.node(
            "negated",
            RuleExpression::Scale {
                value: value.clone(),
                factor: minus_one,
            },
        )?;
        value = p.node(
            "signed",
            RuleExpression::Select {
                condition: negative,
                when_true: negated,
                when_false: value,
            },
        )?;
    }
    p.budget.charge(1)?;
    p.value.effects.push(RuleEffect {
        id: key("effective-value"),
        when: None,
        effect: RuleEffectKind::Derive {
            entity: RuleEntity::Modifier,
            stat: binding.output.clone(),
            value,
        },
    });
    Ok(p.value)
}
/// Compile explicit canonical components. No schema allocation, source parsing,
/// completeness assertion or operation-contract upgrade is performed here.
pub fn compile_owned_modifier_values(
    base: &StagedOwnedRecipe,
    policy: &ModifierValuePolicy,
    limits: ModifierValueRecipeLimits,
) -> Result<StagedModifierValueRecipe> {
    let hard = ModifierValueRecipeLimits::default();
    for (name, value, maximum) in [
        (
            "policy bytes",
            limits.max_policy_bytes,
            hard.max_policy_bytes,
        ),
        ("bindings", limits.max_bindings, hard.max_bindings),
        ("nodes", limits.max_nodes, hard.max_nodes),
        ("work", limits.max_work, hard.max_work),
    ] {
        if value == 0 || value > maximum {
            return Err(ModifierValueRecipeError::Limit(name));
        }
    }
    if policy.schema_version != 1 {
        return Err(invalid("policy version"));
    }
    if &policy.definitions != base.schema().identity() {
        return Err(ModifierValueRecipeError::Binding);
    }
    if policy.bindings.is_empty() || policy.bindings.len() > limits.max_bindings {
        return Err(ModifierValueRecipeError::Limit("bindings"));
    }
    let policy_digest = digest_owned(
        "owned-modifier-value-policy-v1",
        policy,
        limits.max_policy_bytes,
    )?;
    if !matches!(base.schema().definition(&policy.factor_unit), SchemaLookup::Known(unit) if unit.dimension == UnitDimension::DimensionlessFactor)
    {
        return Err(invalid("factor unit must be Known DimensionlessFactor"));
    }
    let mut budget = Budget {
        work: limits.max_work,
        nodes: limits.max_nodes,
    };
    budget.charge(policy.bindings.len() + base.rules().input().owners.len())?;
    let prior: BTreeMap<_, _> = base
        .rules()
        .input()
        .owners
        .iter()
        .filter_map(|owner| match &owner.owner {
            SchemaSubject::Definition(DefinitionAddress::Modifier(id)) => Some((id.clone(), owner)),
            _ => None,
        })
        .collect();
    let mut outputs = BTreeSet::new();
    let mut ids = BTreeSet::new();
    // Validate every field and precision before generating any expression nodes.
    for binding in &policy.bindings {
        validate_binding(base, policy, binding, &mut budget)?;
        if !outputs.insert((binding.modifier.clone(), binding.output.clone())) {
            return Err(invalid("duplicate modifier output"));
        }
        if !ids.insert((binding.modifier.clone(), binding.program.clone())) {
            return Err(invalid("duplicate modifier program"));
        }
        if !prior.contains_key(&binding.modifier) {
            return Err(invalid("modifier requires an existing covered rule owner"));
        }
    }
    for binding in &policy.bindings {
        budget.charge(1)?;
        if let ModifierValueScaling::Scaled {
            corrupted_base,
            magnitude,
        } = &binding.scaling
            && [corrupted_base, magnitude]
                .iter()
                .any(|stat| outputs.contains(&(binding.modifier.clone(), (*stat).clone())))
        {
            return Err(invalid(
                "effective outputs cannot supply their upstream factors",
            ));
        }
    }
    let mut generated: BTreeMap<ModifierDefId, BTreeMap<OwnedDefinitionKey, RuleProgram>> =
        BTreeMap::new();
    for binding in &policy.bindings {
        let value = program(policy, binding, &mut budget)?;
        generated
            .entry(binding.modifier.clone())
            .or_default()
            .insert(binding.program.clone(), value);
    }
    let mut owners = Vec::new();
    for (modifier, programs) in generated {
        let old = prior[&modifier];
        let gaps = match &old.programs.closure {
            SchemaClosure::Complete => 0,
            SchemaClosure::Partial { gaps } => gaps.len(),
        };
        budget.charge(old.programs.members.len() + gaps + 1)?;
        for existing in &old.programs.members {
            budget
                .charge(existing.reads.len() + existing.nodes.len() + existing.effects.len() + 1)?;
            if let Some(requested) = programs.get(&existing.id) {
                if requested == existing {
                    continue;
                }
                return Err(invalid("existing program differs"));
            }
            for effect in &existing.effects {
                if let RuleEffectKind::Derive {
                    entity: RuleEntity::Modifier,
                    stat,
                    ..
                }
                | RuleEffectKind::Contribute {
                    entity: RuleEntity::Modifier,
                    stat,
                    ..
                } = &effect.effect
                    && outputs.contains(&(modifier.clone(), stat.clone()))
                {
                    return Err(invalid("existing modifier output writer conflicts"));
                }
            }
        }
        owners.push(DefinitionRules {
            owner: SchemaSubject::Definition(modifier.address()),
            programs: DeclaredSet {
                members: programs.into_values().collect(),
                closure: old.programs.closure.clone(),
            },
        });
    }
    let extension = OwnedRecipeExtension {
        schema_version: 1,
        version: policy.version.clone(),
        schema: vec![],
        operations_version: None,
        tables: vec![],
        owners,
        receivers: vec![],
    };
    let extended = extend_owned_recipe(base, &extension, limits.extension)?;
    if extended.refinement.is_some() {
        return Err(invalid("modifier values unexpectedly refined schema"));
    }
    Ok(StagedModifierValueRecipe {
        successor: extended.successor,
        receipt: ModifierValueRecipeReceipt {
            policy: policy_digest,
            bindings: policy.bindings.len(),
            owners: extension.owners.len(),
            nodes: limits.max_nodes - budget.nodes,
            work_used: limits.max_work - budget.work,
            extension: extended.receipt,
        },
        extension,
    })
}
