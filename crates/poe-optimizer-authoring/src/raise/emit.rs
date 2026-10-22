//! Source construction for the existing Core variants, without evaluation.
use super::{
    Prepared, RaiseError, RaiseErrorKind, RaiseLimit, RaiseLimits, RaiseLocation, error, limit,
};
use crate::SymbolRegistry;
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::{FiniteQuantity, OwnedDefinitionKey},
    owned_rules::*,
    owned_schema::{
        ComputedValueType, DefinitionAddress, RuleEntityKind, SchemaDefinitionId, SchemaSlotId,
        SlotAddress,
    },
};
use std::{
    collections::BTreeMap,
    fmt::{self, Write},
};

struct Aliases<'symbols> {
    definitions: BTreeMap<&'symbols DefinitionAddress, &'symbols str>,
    slots: BTreeMap<&'symbols SlotAddress, &'symbols str>,
}

impl<'symbols> Aliases<'symbols> {
    fn new(symbols: &'symbols SymbolRegistry, maximum: usize) -> Result<Self, RaiseError> {
        let mut result = Self {
            definitions: BTreeMap::new(),
            slots: BTreeMap::new(),
        };
        let mut count = 0usize;
        // Registry iteration is lexical name order, so the first alias wins.
        // Maps borrow both keys and names; bounded input strings are not copied.
        for (name, address) in symbols.definitions() {
            if count == maximum {
                return Err(limit(RaiseLimit::Symbols, RaiseLocation::Symbol(count)));
            }
            result.definitions.entry(address).or_insert(name);
            count += 1;
        }
        for (name, address) in symbols.slots() {
            if count == maximum {
                return Err(limit(RaiseLimit::Symbols, RaiseLocation::Symbol(count)));
            }
            result.slots.entry(address).or_insert(name);
            count += 1;
        }
        Ok(result)
    }
}

/// Formatting primitive numbers directly into this writer avoids constructing
/// an unbounded intermediate string. Every append checks the byte cap first.
struct Output {
    source: String,
    maximum: usize,
    location: RaiseLocation,
}

impl Write for Output {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if self
            .source
            .len()
            .checked_add(text.len())
            .is_none_or(|end| end > self.maximum)
        {
            return Err(fmt::Error);
        }
        self.source.push_str(text);
        Ok(())
    }
}

impl Output {
    fn text(&mut self, text: &str) -> Result<(), RaiseError> {
        self.write_str(text)
            .map_err(|_| limit(RaiseLimit::SourceBytes, self.location))
    }

    fn number(&mut self, value: impl fmt::Display) -> Result<(), RaiseError> {
        write!(self, "{value}").map_err(|_| limit(RaiseLimit::SourceBytes, self.location))
    }

    fn key(&mut self, key: &OwnedDefinitionKey) -> Result<(), RaiseError> {
        self.text("\"")?;
        self.text(key.as_str())?;
        self.text("\"")
    }
}

struct Emitter<'prepared, 'input, 'symbols> {
    output: Output,
    graph: &'prepared Prepared<'input>,
    aliases: Aliases<'symbols>,
}

pub(super) fn program(
    program: &RuleProgram,
    symbols: &SymbolRegistry,
    graph: &Prepared<'_>,
    limits: RaiseLimits,
) -> Result<String, RaiseError> {
    let mut emit = Emitter {
        output: Output {
            source: String::new(),
            maximum: limits.max_source_bytes,
            location: RaiseLocation::Program,
        },
        graph,
        aliases: Aliases::new(symbols, limits.max_symbols)?,
    };
    emit.output.text("program ")?;
    emit.output.key(&program.id)?;
    emit.output.text(" context ")?;
    emit.output.text(match program.context {
        RuleEntityKind::Actor => "actor",
        RuleEntityKind::Action => "action",
        RuleEntityKind::EquipmentUse => "equipment_use",
        RuleEntityKind::Environment => "environment",
        RuleEntityKind::SupportOrigin => "support_origin",
        RuleEntityKind::Skill => "skill",
        RuleEntityKind::Modifier => {
            return Err(error(
                RaiseErrorKind::InvalidContext,
                RaiseLocation::Program,
            ));
        }
    })?;
    emit.output.text(" {\n")?;
    for index in graph.reads.values().copied() {
        emit.output.location = RaiseLocation::Read(index);
        let read = &program.reads[index];
        emit.output.text("    read ")?;
        emit.output.key(&read.id)?;
        emit.output.text(" as ")?;
        emit.read_reference(&read.id)?;
        emit.output.text(": ")?;
        emit.value_type(&read.value_type)?;
        emit.output.text(" = ")?;
        emit.read(&read.source)?;
        emit.output.text(";\n")?;
    }
    for index in graph.order.iter().copied() {
        emit.output.location = RaiseLocation::Node(index);
        let node = &program.nodes[index];
        emit.output.text("    let ")?;
        emit.node_reference(&node.id)?;
        emit.output.text(" = ")?;
        emit.expression(&node.expression)?;
        emit.output.text(";\n")?;
    }
    for (index, effect) in program.effects.iter().enumerate() {
        emit.output.location = RaiseLocation::Effect(index);
        emit.output.text("    effect ")?;
        emit.output.key(&effect.id)?;
        if let Some(guard) = &effect.when {
            emit.output.text(" when ")?;
            emit.node_reference(guard)?;
        }
        emit.output.text(" = ")?;
        emit.effect(&effect.effect)?;
        emit.output.text(";\n")?;
    }
    emit.output.location = RaiseLocation::Program;
    emit.output.text("}\n")?;
    Ok(emit.output.source)
}

impl Emitter<'_, '_, '_> {
    fn comma(&mut self) -> Result<(), RaiseError> {
        self.output.text(", ")
    }

    fn definition<I: SchemaDefinitionId>(&mut self, id: &I) -> Result<(), RaiseError> {
        let address = id.address();
        let name = self
            .aliases
            .definitions
            .get(&address)
            .copied()
            .ok_or_else(|| {
                error(
                    RaiseErrorKind::MissingDefinitionSymbol,
                    self.output.location,
                )
            })?;
        self.output.text(name)
    }

    fn slot<I: SchemaSlotId>(&mut self, id: &DeclaredSlot<I>) -> Result<(), RaiseError> {
        let address = I::address(id);
        let name = self
            .aliases
            .slots
            .get(&address)
            .copied()
            .ok_or_else(|| error(RaiseErrorKind::MissingSlotSymbol, self.output.location))?;
        self.output.text(name)
    }

    fn node_reference(&mut self, id: &OwnedDefinitionKey) -> Result<(), RaiseError> {
        let index = self
            .graph
            .nodes
            .get(id)
            .copied()
            .ok_or_else(|| error(RaiseErrorKind::MissingNode, self.output.location))?;
        self.output.text("n_")?;
        self.output.number(self.graph.node_names[index])
    }

    fn read_reference(&mut self, id: &OwnedDefinitionKey) -> Result<(), RaiseError> {
        let index = self
            .graph
            .reads
            .get(id)
            .copied()
            .ok_or_else(|| error(RaiseErrorKind::MissingRead, self.output.location))?;
        self.output.text("r_")?;
        self.output.number(self.graph.read_names[index])
    }

    fn entity(&mut self, entity: RuleEntity) -> Result<(), RaiseError> {
        self.output.text(match entity {
            RuleEntity::Current => "current",
            RuleEntity::Modifier => "modifier",
            RuleEntity::Actor => "actor",
            RuleEntity::Player => "player",
            RuleEntity::Enemy => "enemy",
            RuleEntity::Environment => "environment",
            RuleEntity::SupportOrigin => "support_origin",
            RuleEntity::Skill => "skill",
            RuleEntity::AssignedSkill => "assigned_skill",
            RuleEntity::EffectSource => "effect_source",
            RuleEntity::PropertyOwner => "property_owner",
        })
    }

    fn value_type(&mut self, value: &ComputedValueType) -> Result<(), RaiseError> {
        match value {
            ComputedValueType::Boolean => self.output.text("boolean"),
            ComputedValueType::Integer => self.output.text("integer"),
            ComputedValueType::Option => self.output.text("option"),
            ComputedValueType::Quantity { unit } => {
                self.output.text("quantity(")?;
                self.definition(unit)?;
                self.output.text(")")
            }
        }
    }

    fn quantity(&mut self, value: &FiniteQuantity) -> Result<(), RaiseError> {
        if !value.value().is_finite() {
            return Err(error(RaiseErrorKind::InvalidLiteral, self.output.location));
        }
        self.output.text("quantity(")?;
        // Rust's Display spelling round-trips the finite f64 exactly; the
        // explicit constructor avoids numeric suffix and integer ambiguity.
        self.output.number(if value.value() == 0.0 {
            0.0
        } else {
            value.value()
        })?;
        self.comma()?;
        self.definition(value.unit())?;
        self.output.text(")")
    }

    fn literal(&mut self, value: &ParameterValue) -> Result<(), RaiseError> {
        match value {
            ParameterValue::Boolean(value) => {
                self.output.text(if *value { "true" } else { "false" })
            }
            ParameterValue::Integer(value) => self.output.number(value.get()),
            ParameterValue::Quantity(value) => self.quantity(value),
            ParameterValue::Option(value) => {
                self.output.text("option(")?;
                self.definition(value)?;
                self.output.text(")")
            }
        }
    }

    fn one_definition<I: SchemaDefinitionId>(
        &mut self,
        name: &str,
        value: &I,
    ) -> Result<(), RaiseError> {
        self.output.text(name)?;
        self.output.text("(")?;
        self.definition(value)?;
        self.output.text(")")
    }

    fn one_slot<I: SchemaSlotId>(
        &mut self,
        name: &str,
        value: &DeclaredSlot<I>,
    ) -> Result<(), RaiseError> {
        self.output.text(name)?;
        self.output.text("(")?;
        self.slot(value)?;
        self.output.text(")")
    }

    fn entity_definition<I: SchemaDefinitionId>(
        &mut self,
        name: &str,
        entity: RuleEntity,
        value: &I,
    ) -> Result<(), RaiseError> {
        self.output.text(name)?;
        self.output.text("(")?;
        self.entity(entity)?;
        self.comma()?;
        self.definition(value)?;
        self.output.text(")")
    }

    fn read(&mut self, read: &RuleReadSource) -> Result<(), RaiseError> {
        match read {
            RuleReadSource::EffectSourceParameter { slot } => {
                self.one_slot("effect_source_parameter", slot)
            }
            RuleReadSource::EffectSourceChoice { slot } => {
                self.one_slot("effect_source_choice", slot)
            }
            RuleReadSource::Parameter { slot } => self.one_slot("parameter", slot),
            RuleReadSource::Choice { slot } => self.one_slot("choice", slot),
            RuleReadSource::CharacterLevel => self.output.text("character_level()"),
            RuleReadSource::EnemyLevel => self.output.text("enemy_level()"),
            RuleReadSource::GemLevel => self.output.text("gem_level()"),
            RuleReadSource::ItemLevel => self.output.text("item_level()"),
            RuleReadSource::CharacterClassIs { class } => {
                self.one_definition("character_class_is", class)
            }
            RuleReadSource::CharacterAscendancyIs { ascendancy } => {
                self.one_definition("character_ascendancy_is", ascendancy)
            }
            RuleReadSource::ActionPartIs { part } => self.one_definition("action_part_is", part),
            RuleReadSource::ActionModeIs { mode } => self.one_definition("action_mode_is", mode),
            RuleReadSource::ActionStatSetIs { stat_set } => {
                self.one_definition("action_stat_set_is", stat_set)
            }
            RuleReadSource::ItemQualityAmount { quality } => {
                self.one_definition("item_quality_amount", quality)
            }
            RuleReadSource::HasItemQuality { quality } => {
                self.one_definition("has_item_quality", quality)
            }
            RuleReadSource::GemQualityAmount { quality } => {
                self.one_definition("gem_quality_amount", quality)
            }
            RuleReadSource::HasGemQuality { quality } => {
                self.one_definition("has_gem_quality", quality)
            }
            RuleReadSource::PlayerEquipmentSlot { slot, read } => {
                self.output.text("player_equipment_slot(")?;
                self.definition(slot)?;
                self.comma()?;
                match read {
                    PlayerEquipmentSlotRead::Occupied => self.output.text("occupied()")?,
                    PlayerEquipmentSlotRead::Stat { stat } => self.one_definition("stat", stat)?,
                    PlayerEquipmentSlotRead::Capability { capability } => {
                        self.one_definition("capability", capability)?
                    }
                }
                self.output.text(")")
            }
            RuleReadSource::Stat { entity, stat } => self.entity_definition("stat", *entity, stat),
            RuleReadSource::Capability { entity, capability } => {
                self.entity_definition("capability", *entity, capability)
            }
            RuleReadSource::External { entity, input } => {
                self.entity_definition("external", *entity, input)
            }
            RuleReadSource::ModifierTransforms { stat, initial } => {
                self.output.text("modifier_transforms(")?;
                self.definition(stat)?;
                self.comma()?;
                self.definition(initial)?;
                self.output.text(")")
            }
            RuleReadSource::Contributions {
                entity,
                stat,
                contribution,
                reduction,
                empty,
            } => {
                self.output.text("contributions(")?;
                self.entity(*entity)?;
                self.comma()?;
                self.definition(stat)?;
                self.comma()?;
                self.output.text(contribution_name(*contribution))?;
                self.comma()?;
                self.output.text(reduction_name(*reduction))?;
                self.comma()?;
                self.literal(empty)?;
                self.output.text(")")
            }
            RuleReadSource::ContributionQuery {
                entity,
                query,
                group,
            } => {
                self.query("contribution_query", *entity, query, group)?;
                self.output.text(")")
            }
            RuleReadSource::ContributionSelection {
                entity,
                query,
                group,
                projection,
            } => {
                self.query("contribution_selection", *entity, query, group)?;
                self.comma()?;
                self.output.text(match projection {
                    ContributionSelectionProjection::Present => "present",
                    ContributionSelectionProjection::Value => "value",
                })?;
                self.output.text(")")
            }
        }
    }

    fn query(
        &mut self,
        name: &str,
        entity: RuleEntity,
        query: &OwnedDefinitionKey,
        group: &OwnedDefinitionKey,
    ) -> Result<(), RaiseError> {
        self.output.text(name)?;
        self.output.text("(")?;
        self.entity(entity)?;
        self.comma()?;
        self.output.key(query)?;
        self.comma()?;
        self.output.key(group)
    }

    fn binary_call(
        &mut self,
        name: &str,
        left: &OwnedDefinitionKey,
        right: &OwnedDefinitionKey,
    ) -> Result<(), RaiseError> {
        self.output.text(name)?;
        self.output.text("(")?;
        self.node_reference(left)?;
        self.comma()?;
        self.node_reference(right)?;
        self.output.text(")")
    }

    fn infix(
        &mut self,
        operator: &str,
        left: &OwnedDefinitionKey,
        right: &OwnedDefinitionKey,
    ) -> Result<(), RaiseError> {
        self.node_reference(left)?;
        self.output.text(operator)?;
        self.node_reference(right)
    }

    fn expression(&mut self, expression: &RuleExpression) -> Result<(), RaiseError> {
        match expression {
            RuleExpression::Literal { value } => self.literal(value),
            RuleExpression::Read { input } => self.read_reference(input),
            RuleExpression::Add { left, right } => self.infix(" + ", left, right),
            RuleExpression::Subtract { left, right } => self.infix(" - ", left, right),
            RuleExpression::Minimum { left, right } => self.binary_call("min", left, right),
            RuleExpression::Maximum { left, right } => self.binary_call("max", left, right),
            RuleExpression::Scale { value, factor } => self.binary_call("scale", value, factor),
            RuleExpression::ScaleInteger { value, count } => {
                self.binary_call("scale_integer", value, count)
            }
            RuleExpression::DivideFactor { value, divisor } => {
                self.binary_call("divide_factor", value, divisor)
            }
            RuleExpression::Compare {
                operation,
                left,
                right,
            } => self.infix(
                match operation {
                    RuleComparison::Equal => " == ",
                    RuleComparison::Less => " < ",
                    RuleComparison::LessOrEqual => " <= ",
                    RuleComparison::Greater => " > ",
                    RuleComparison::GreaterOrEqual => " >= ",
                },
                left,
                right,
            ),
            RuleExpression::Not { value } => {
                self.output.text("not ")?;
                self.node_reference(value)
            }
            RuleExpression::All { values } | RuleExpression::Any { values } => {
                self.output
                    .text(if matches!(expression, RuleExpression::All { .. }) {
                        "all("
                    } else {
                        "any("
                    })?;
                for (position, value) in values.iter().enumerate() {
                    if position != 0 {
                        self.comma()?;
                    }
                    self.node_reference(value)?;
                }
                self.output.text(")")
            }
            RuleExpression::Select {
                condition,
                when_true,
                when_false,
            } => {
                self.output.text("if ")?;
                self.node_reference(condition)?;
                self.output.text(" { ")?;
                self.node_reference(when_true)?;
                self.output.text(" } else { ")?;
                self.node_reference(when_false)?;
                self.output.text(" }")
            }
            RuleExpression::Ratio {
                numerator,
                denominator,
                unit,
            } => {
                self.output.text("ratio(")?;
                self.node_reference(numerator)?;
                self.comma()?;
                self.node_reference(denominator)?;
                self.comma()?;
                self.definition(unit)?;
                self.output.text(")")
            }
            RuleExpression::PercentAsFactor { percent, unit } => {
                self.output.text("pct(")?;
                self.node_reference(percent)?;
                self.comma()?;
                self.definition(unit)?;
                self.output.text(")")
            }
            RuleExpression::Round {
                value,
                quantum,
                mode,
            }
            | RuleExpression::QuantizeInteger {
                value,
                quantum,
                mode,
            } => {
                self.output
                    .text(if matches!(expression, RuleExpression::Round { .. }) {
                        "round("
                    } else {
                        "quantize_integer("
                    })?;
                self.node_reference(value)?;
                self.comma()?;
                self.quantity(quantum)?;
                self.comma()?;
                self.output.text(rounding_name(*mode))?;
                self.output.text(")")
            }
            RuleExpression::CheckNumericDomain { value, domain } => {
                self.output.text("check_domain(")?;
                self.node_reference(value)?;
                self.comma()?;
                match domain {
                    NumericDomain::Integer(range) => {
                        self.output.number(range.minimum.get())?;
                        self.comma()?;
                        self.output.number(range.maximum.get())?;
                    }
                    NumericDomain::Quantity(range) => {
                        self.quantity(&range.minimum)?;
                        self.comma()?;
                        self.quantity(&range.maximum)?;
                    }
                }
                self.output.text(")")
            }
            RuleExpression::LookupIntegerTable { table, key } => {
                self.output.text("lookup(")?;
                self.node_reference(key)?;
                self.comma()?;
                self.output.key(table)?;
                self.output.text(")")
            }
            RuleExpression::OrdinaryTiming { recipe } => {
                self.output.text("ordinary_timing(")?;
                for input in recipe.inputs() {
                    self.node_reference(input)?;
                    self.comma()?;
                }
                self.output
                    .number(recipe.speed_multiplier_rounding_precision)?;
                self.comma()?;
                self.definition(&recipe.units.time)?;
                self.comma()?;
                self.definition(&recipe.units.rate)?;
                self.comma()?;
                self.output.text(match recipe.output {
                    OrdinaryTimingChannel::SpeedMultiplier => "speed_multiplier",
                    OrdinaryTimingChannel::PreCapRate => "pre_cap_rate",
                    OrdinaryTimingChannel::ActionRate => "action_rate",
                    OrdinaryTimingChannel::ActionTime => "action_time",
                })?;
                self.output.text(")")
            }
        }
    }

    fn effect(&mut self, effect: &RuleEffectKind) -> Result<(), RaiseError> {
        match effect {
            RuleEffectKind::Contribute {
                entity,
                stat,
                contribution,
                value,
            } => {
                self.output.text("contribute(")?;
                self.entity(*entity)?;
                self.comma()?;
                self.definition(stat)?;
                self.comma()?;
                self.output.text(contribution_name(*contribution))?;
                self.comma()?;
                self.node_reference(value)?;
            }
            RuleEffectKind::Derive {
                entity,
                stat,
                value,
            } => {
                self.output.text("derive(")?;
                self.entity(*entity)?;
                self.comma()?;
                self.definition(stat)?;
                self.comma()?;
                self.node_reference(value)?;
            }
            RuleEffectKind::Capability {
                entity,
                capability,
                enabled,
            } => {
                self.output.text("capability(")?;
                self.entity(*entity)?;
                self.comma()?;
                self.definition(capability)?;
                self.comma()?;
                self.node_reference(enabled)?;
            }
            RuleEffectKind::SupportApplicability { applicable } => {
                self.output.text("support_applicability(")?;
                self.node_reference(applicable)?;
            }
            RuleEffectKind::ActivateGrant { slot, enabled } => {
                self.output.text("activate_grant(")?;
                self.slot(slot)?;
                self.comma()?;
                self.node_reference(enabled)?;
            }
            RuleEffectKind::ProjectSkillParameter {
                skill,
                parameter,
                value,
            } => {
                self.output.text("project_skill_parameter(")?;
                self.slot(skill)?;
                self.comma()?;
                self.slot(parameter)?;
                self.comma()?;
                self.node_reference(value)?;
            }
            RuleEffectKind::ProjectActorStat { actor, stat, value } => {
                self.output.text("project_actor_stat(")?;
                self.slot(actor)?;
                self.comma()?;
                self.definition(stat)?;
                self.comma()?;
                self.node_reference(value)?;
            }
            RuleEffectKind::Requirement { satisfied, code } => {
                self.output.text("requirement(")?;
                self.node_reference(satisfied)?;
                self.comma()?;
                self.output.key(code)?;
            }
            RuleEffectKind::ProjectModifierTransform {
                stat,
                targets,
                order,
                operation,
                value,
            } => {
                self.output.text("project_modifier_transform(")?;
                self.definition(stat)?;
                self.output.text(", targets(")?;
                for (position, target) in targets.iter().enumerate() {
                    if position != 0 {
                        self.comma()?;
                    }
                    self.output.text("target(")?;
                    self.definition(&target.definition)?;
                    if let Some(when) = &target.when {
                        self.comma()?;
                        self.definition(when)?;
                    }
                    self.output.text(")")?;
                }
                self.output.text("), ")?;
                self.output.number(order.get())?;
                self.comma()?;
                self.output.text(match operation {
                    ModifierTransformOperation::Add => "add",
                    ModifierTransformOperation::Multiply => "multiply",
                })?;
                self.comma()?;
                self.node_reference(value)?;
            }
        }
        self.output.text(")")
    }
}

fn contribution_name(kind: ContributionKind) -> &'static str {
    match kind {
        ContributionKind::Flag => "flag",
        ContributionKind::Override => "override",
        ContributionKind::Add => "add",
        ContributionKind::Increase => "increase",
        ContributionKind::Multiply => "multiply",
    }
}

fn reduction_name(kind: ContributionReduction) -> &'static str {
    match kind {
        ContributionReduction::Any => "any",
        ContributionReduction::RequireAgreement => "require_agreement",
        ContributionReduction::ExactIntegerSum => "exact_integer_sum",
        ContributionReduction::Sum => "sum",
        ContributionReduction::Product => "product",
    }
}

fn rounding_name(mode: RuleRounding) -> &'static str {
    match mode {
        RuleRounding::Floor => "floor",
        RuleRounding::Ceiling => "ceiling",
        RuleRounding::Truncate => "truncate",
        RuleRounding::NearestTiesPositive => "nearest_ties_positive",
    }
}
