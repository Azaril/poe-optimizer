//! Explicit program construction. This module resolves syntax into the existing
//! Core contracts; only the Engine's package compiler establishes authority.
use crate::{
    BindingLimits, Diagnostic, DiagnosticCode, ExprId, ExprKind, ExpressionBindings, LimitKind,
    LowerLimits, LoweredBinding, ParsedProgram, Span, SymbolRegistry,
    lower::{Syntax, lower_statements, path_name, static_literal, validate_limits},
};
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::{
        ComputedValueType, DefinitionAddress, DefinitionSchemaIndex, RuleEntityKind, SlotAddress,
    },
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug)]
pub struct ProgramLowerLimits {
    pub formulas: LowerLimits,
    pub bindings: BindingLimits,
    pub max_reads: usize,
    pub max_effects: usize,
}

impl Default for ProgramLowerLimits {
    fn default() -> Self {
        Self {
            formulas: LowerLimits::default(),
            bindings: BindingLimits::default(),
            max_reads: 4096,
            max_effects: 4096,
        }
    }
}

/// Parallel to the corresponding IR vectors. These source locations do not
/// participate in content identity and confer no definition or effect authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramSpans {
    pub id: Span,
    pub context: Span,
    pub reads: Vec<Span>,
    pub nodes: Vec<Span>,
    pub effects: Vec<Span>,
}

/// An unchecked program, ready for insertion into the manifest-owned package
/// and validation by the existing Engine. No owner or coverage is inferred.
#[derive(Clone, Debug, PartialEq)]
pub struct LoweredProgram {
    pub program: RuleProgram,
    pub bindings: Vec<LoweredBinding>,
    pub spans: ProgramSpans,
}

fn error(code: DiagnosticCode, span: Span, message: &'static str) -> Diagnostic {
    Diagnostic::new(code, span, message)
}

pub fn lower_program<I: DefinitionSchemaIndex>(
    parsed: &ParsedProgram<'_>,
    index: &I,
    symbols: &SymbolRegistry,
    limits: ProgramLowerLimits,
) -> Result<LoweredProgram, Diagnostic> {
    let span = Span {
        start: 0,
        end: parsed.source().len(),
    };
    validate_limits(limits.formulas, parsed.nodes().len(), span)?;
    if parsed.statements().len() > limits.formulas.max_statements {
        return Err(error(
            DiagnosticCode::LimitExceeded(LimitKind::Statements),
            parsed.statements()[limits.formulas.max_statements].span,
            "program statement limit exceeded",
        ));
    }
    if parsed.reads().len() > limits.max_reads {
        return Err(error(
            DiagnosticCode::LimitExceeded(LimitKind::Reads),
            parsed.reads()[limits.max_reads].span,
            "program read limit exceeded",
        ));
    }
    if parsed.effects().len() > limits.max_effects {
        return Err(error(
            DiagnosticCode::LimitExceeded(LimitKind::Effects),
            parsed.effects()[limits.max_effects].span,
            "program effect limit exceeded",
        ));
    }
    if limits.bindings.max_bindings == 0 || limits.bindings.max_name_bytes == 0 {
        return Err(error(
            DiagnosticCode::InvalidLimits,
            span,
            "binding limits must be positive",
        ));
    }
    let unit_budget = limits
        .bindings
        .max_bindings
        .checked_sub(parsed.reads().len())
        .ok_or_else(|| {
            error(
                DiagnosticCode::LimitExceeded(LimitKind::Bindings),
                span,
                "program binding limit exceeded",
            )
        })?;
    let mut units = Vec::new();
    for (name, address) in symbols.definitions() {
        if let DefinitionAddress::Unit(unit) = address {
            if units.len() == unit_budget || name.len() > limits.bindings.max_name_bytes {
                return Err(error(
                    DiagnosticCode::LimitExceeded(LimitKind::Bindings),
                    span,
                    "program unit binding limit exceeded",
                ));
            }
            units.push((name, unit.clone()));
        }
    }
    for read in parsed.reads() {
        if read.name.end - read.name.start > limits.bindings.max_name_bytes {
            return Err(error(
                DiagnosticCode::LimitExceeded(LimitKind::Bindings),
                read.name,
                "program read alias exceeds the name limit",
            ));
        }
    }
    symbols.validate_against(index).map_err(|_| {
        error(
            DiagnosticCode::InvalidArguments,
            span,
            "symbol registry does not match the supplied schema",
        )
    })?;
    let syntax = Syntax {
        source: parsed.source(),
        nodes: parsed.nodes(),
    };
    let resolve = Resolve {
        syntax: &syntax,
        symbols,
    };
    let id = resolve.key_span(parsed.id())?;
    let context = match parsed.text(parsed.context()) {
        Some("actor") => RuleEntityKind::Actor,
        Some("action") => RuleEntityKind::Action,
        Some("equipment_use") => RuleEntityKind::EquipmentUse,
        Some("environment") => RuleEntityKind::Environment,
        Some("support_origin") => RuleEntityKind::SupportOrigin,
        Some("skill") => RuleEntityKind::Skill,
        _ => {
            return Err(error(
                DiagnosticCode::InvalidArguments,
                parsed.context(),
                "unknown or invalid program context",
            ));
        }
    };
    let unit_bindings = ExpressionBindings::from_reads(index, &units, &[], limits.bindings)
        .map_err(|_| {
            error(
                DiagnosticCode::InvalidArguments,
                span,
                "unit bindings do not match the supplied schema or binding limits",
            )
        })?;
    let mut reads = Vec::with_capacity(parsed.reads().len());
    let mut read_ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for declaration in parsed.reads() {
        let id = resolve.key_span(declaration.id)?;
        if !read_ids.insert(id.clone()) {
            return Err(error(
                DiagnosticCode::DuplicateSymbol,
                declaration.id,
                "duplicate read ID",
            ));
        }
        let name = parsed.text(declaration.name).ok_or_else(|| {
            error(
                DiagnosticCode::InvalidArguments,
                declaration.name,
                "invalid read alias",
            )
        })?;
        if !names.insert(name) {
            return Err(error(
                DiagnosticCode::DuplicateSymbol,
                declaration.name,
                "duplicate read alias",
            ));
        }
        reads.push((
            name,
            RuleRead {
                id,
                value_type: resolve.value_type(declaration.value_type)?,
                source: resolve.read_source(declaration.source, &unit_bindings)?,
            },
        ));
    }
    let bindings =
        ExpressionBindings::from_reads(index, &units, &reads, limits.bindings).map_err(|_| {
            error(
                DiagnosticCode::InvalidArguments,
                span,
                "read bindings do not match the supplied schema or binding limits",
            )
        })?;
    let start = parsed
        .reads()
        .last()
        .map_or(0, |read| read.source.index() + 1);
    let block = lower_statements(
        &syntax,
        parsed.statements(),
        start,
        &bindings,
        limits.formulas,
    )?;
    let locals: BTreeMap<_, _> = block
        .bindings
        .iter()
        .map(|binding| (binding.name.as_str(), binding))
        .collect();
    let mut effect_ids = BTreeSet::new();
    let mut effects = Vec::with_capacity(parsed.effects().len());
    for declaration in parsed.effects() {
        let id = resolve.key_span(declaration.id)?;
        if !effect_ids.insert(id.clone()) {
            return Err(error(
                DiagnosticCode::DuplicateSymbol,
                declaration.id,
                "duplicate effect ID",
            ));
        }
        let when = declaration
            .when
            .map(|value| resolve.boolean_value(value, &locals))
            .transpose()?;
        let effect = resolve.effect(declaration.effect, &locals, &unit_bindings)?;
        effects.push(RuleEffect { id, when, effect });
    }
    Ok(LoweredProgram {
        program: RuleProgram {
            id,
            context,
            reads: reads.into_iter().map(|(_, read)| read).collect(),
            nodes: block.nodes,
            effects,
        },
        bindings: block.bindings,
        spans: ProgramSpans {
            id: parsed.id(),
            context: parsed.context(),
            reads: parsed.reads().iter().map(|read| read.span).collect(),
            nodes: block.spans,
            effects: parsed.effects().iter().map(|effect| effect.span).collect(),
        },
    })
}

struct Resolve<'a> {
    syntax: &'a Syntax<'a>,
    symbols: &'a SymbolRegistry,
}

macro_rules! definition_resolvers {
    ($($method:ident: $variant:ident => $ty:ty),+ $(,)?) => { $(
        fn $method(&self, id: ExprId) -> Result<$ty, Diagnostic> {
            let name = path_name(self.syntax, id)?;
            match self.symbols.definition(&name) {
                Some(DefinitionAddress::$variant(value)) => Ok(value.clone()),
                Some(_) => Err(error(DiagnosticCode::TypeMismatch, self.span(id), "definition symbol has the wrong kind")),
                None => Err(error(DiagnosticCode::UnknownSymbol, self.span(id), "unknown definition symbol")),
            }
        }
    )+ };
}

macro_rules! slot_resolvers {
    ($($method:ident: $variant:ident => $ty:ty),+ $(,)?) => { $(
        fn $method(&self, id: ExprId) -> Result<DeclaredSlot<$ty>, Diagnostic> {
            let name = path_name(self.syntax, id)?;
            match self.symbols.slot(&name) {
                Some(SlotAddress::$variant(value)) => Ok(value.clone()),
                Some(_) => Err(error(DiagnosticCode::TypeMismatch, self.span(id), "slot symbol has the wrong kind")),
                None => Err(error(DiagnosticCode::UnknownSymbol, self.span(id), "unknown exact slot symbol")),
            }
        }
    )+ };
}

impl Resolve<'_> {
    definition_resolvers! {
        unit: Unit => UnitDefId,
        stat: Stat => StatDefId,
        capability: Capability => CapabilityDefId,
        external: ExternalInput => ExternalInputDefId,
        class: Class => ClassDefId,
        ascendancy: Ascendancy => AscendancyDefId,
        equipment_slot: EquipmentSlot => EquipmentSlotDefId,
        part: ActionPart => ActionPartDefId,
        mode: ActionMode => ActionModeDefId,
        stat_set: ActionStatSet => ActionStatSetDefId,
        quality: Quality => QualityDefId,
        modifier: Modifier => ModifierDefId,
        option: Option => OptionDefId,
    }
    slot_resolvers! {
        parameter: Parameter => ParameterSlotDefId,
        choice: Choice => ChoiceSlotDefId,
        grant: Grant => GrantSlotDefId,
        skill: SkillGrant => SkillGrantSlotDefId,
        actor: Actor => ActorSlotDefId,
    }

    fn span(&self, id: ExprId) -> Span {
        self.syntax.nodes[id.index()].span
    }

    fn key_span(&self, span: Span) -> Result<OwnedDefinitionKey, Diagnostic> {
        self.syntax
            .text(span)
            .and_then(|s| OwnedDefinitionKey::new(s).ok())
            .ok_or_else(|| {
                error(
                    DiagnosticCode::InvalidKey,
                    span,
                    "invalid owned key literal",
                )
            })
    }

    fn key(&self, id: ExprId) -> Result<OwnedDefinitionKey, Diagnostic> {
        match self.syntax.node(id).map(|node| &node.kind) {
            Some(ExprKind::Key(span)) => self.key_span(*span),
            _ => Err(error(
                DiagnosticCode::InvalidArguments,
                self.span(id),
                "expected a quoted owned key",
            )),
        }
    }

    fn call(&self, id: ExprId) -> Result<(String, &[ExprId]), Diagnostic> {
        let ExprKind::Call { callee, arguments } = &self.syntax.nodes[id.index()].kind else {
            return Err(error(
                DiagnosticCode::InvalidArguments,
                self.span(id),
                "expected an explicit constructor",
            ));
        };
        Ok((path_name(self.syntax, *callee)?, arguments))
    }

    fn arity(&self, id: ExprId, args: &[ExprId], count: usize) -> Result<(), Diagnostic> {
        if args.len() != count {
            return Err(error(
                DiagnosticCode::InvalidArguments,
                self.span(id),
                "wrong number of constructor arguments",
            ));
        }
        Ok(())
    }

    fn keyword<T: Copy>(&self, id: ExprId, variants: &[(&str, T)]) -> Result<T, Diagnostic> {
        let name = path_name(self.syntax, id)?;
        variants
            .iter()
            .find(|(candidate, _)| *candidate == name)
            .map(|(_, value)| *value)
            .ok_or_else(|| {
                error(
                    DiagnosticCode::InvalidArguments,
                    self.span(id),
                    "unknown constructor keyword",
                )
            })
    }

    fn entity(&self, id: ExprId) -> Result<RuleEntity, Diagnostic> {
        self.keyword(
            id,
            &[
                ("current", RuleEntity::Current),
                ("modifier", RuleEntity::Modifier),
                ("actor", RuleEntity::Actor),
                ("player", RuleEntity::Player),
                ("enemy", RuleEntity::Enemy),
                ("environment", RuleEntity::Environment),
                ("support_origin", RuleEntity::SupportOrigin),
                ("skill", RuleEntity::Skill),
                ("assigned_skill", RuleEntity::AssignedSkill),
                ("effect_source", RuleEntity::EffectSource),
                ("property_owner", RuleEntity::PropertyOwner),
            ],
        )
    }

    fn contribution(&self, id: ExprId) -> Result<ContributionKind, Diagnostic> {
        self.keyword(
            id,
            &[
                ("flag", ContributionKind::Flag),
                ("override", ContributionKind::Override),
                ("add", ContributionKind::Add),
                ("increase", ContributionKind::Increase),
                ("multiply", ContributionKind::Multiply),
            ],
        )
    }

    fn reduction(&self, id: ExprId) -> Result<ContributionReduction, Diagnostic> {
        self.keyword(
            id,
            &[
                ("any", ContributionReduction::Any),
                ("require_agreement", ContributionReduction::RequireAgreement),
                ("exact_integer_sum", ContributionReduction::ExactIntegerSum),
                ("sum", ContributionReduction::Sum),
                ("product", ContributionReduction::Product),
            ],
        )
    }

    fn value_type(&self, id: ExprId) -> Result<ComputedValueType, Diagnostic> {
        if matches!(self.syntax.nodes[id.index()].kind, ExprKind::Path { .. }) {
            return match path_name(self.syntax, id)?.as_str() {
                "boolean" => Ok(ComputedValueType::Boolean),
                "integer" => Ok(ComputedValueType::Integer),
                "option" => Ok(ComputedValueType::Option),
                _ => Err(error(
                    DiagnosticCode::TypeMismatch,
                    self.span(id),
                    "unknown value type",
                )),
            };
        }
        let (name, args) = self.call(id)?;
        self.arity(id, args, 1)?;
        if name != "quantity" {
            return Err(error(
                DiagnosticCode::TypeMismatch,
                self.span(id),
                "unknown value type constructor",
            ));
        }
        Ok(ComputedValueType::Quantity {
            unit: self.unit(args[0])?,
        })
    }

    fn literal(
        &self,
        id: ExprId,
        bindings: &ExpressionBindings,
    ) -> Result<ParameterValue, Diagnostic> {
        if let ExprKind::Call { callee, arguments } = &self.syntax.nodes[id.index()].kind
            && path_name(self.syntax, *callee)? == "option"
        {
            self.arity(id, arguments, 1)?;
            return Ok(ParameterValue::Option(self.option(arguments[0])?));
        }
        static_literal(self.syntax, bindings, id)
    }

    fn equipment_read(&self, id: ExprId) -> Result<PlayerEquipmentSlotRead, Diagnostic> {
        let (name, args) = self.call(id)?;
        match name.as_str() {
            "occupied" => {
                self.arity(id, args, 0)?;
                Ok(PlayerEquipmentSlotRead::Occupied)
            }
            "stat" => {
                self.arity(id, args, 1)?;
                Ok(PlayerEquipmentSlotRead::Stat {
                    stat: self.stat(args[0])?,
                })
            }
            "capability" => {
                self.arity(id, args, 1)?;
                Ok(PlayerEquipmentSlotRead::Capability {
                    capability: self.capability(args[0])?,
                })
            }
            _ => Err(error(
                DiagnosticCode::InvalidArguments,
                self.span(id),
                "unknown equipment slot read",
            )),
        }
    }

    fn read_source(
        &self,
        id: ExprId,
        bindings: &ExpressionBindings,
    ) -> Result<RuleReadSource, Diagnostic> {
        let (name, args) = self.call(id)?;
        let count = match name.as_str() {
            "character_level" | "enemy_level" | "gem_level" | "item_level" => 0,
            "effect_source_parameter"
            | "effect_source_choice"
            | "parameter"
            | "choice"
            | "character_class_is"
            | "character_ascendancy_is"
            | "action_part_is"
            | "action_mode_is"
            | "action_stat_set_is"
            | "item_quality_amount"
            | "has_item_quality"
            | "gem_quality_amount"
            | "has_gem_quality" => 1,
            "player_equipment_slot"
            | "stat"
            | "modifier_transforms"
            | "capability"
            | "external" => 2,
            "contribution_query" => 3,
            "contribution_selection" => 4,
            "contributions" => 5,
            _ => {
                return Err(error(
                    DiagnosticCode::UnknownFunction,
                    self.span(id),
                    "unknown read source constructor",
                ));
            }
        };
        self.arity(id, args, count)?;
        Ok(match name.as_str() {
            "effect_source_parameter" => RuleReadSource::EffectSourceParameter {
                slot: self.parameter(args[0])?,
            },
            "effect_source_choice" => RuleReadSource::EffectSourceChoice {
                slot: self.choice(args[0])?,
            },
            "parameter" => RuleReadSource::Parameter {
                slot: self.parameter(args[0])?,
            },
            "choice" => RuleReadSource::Choice {
                slot: self.choice(args[0])?,
            },
            "character_level" => RuleReadSource::CharacterLevel,
            "enemy_level" => RuleReadSource::EnemyLevel,
            "gem_level" => RuleReadSource::GemLevel,
            "item_level" => RuleReadSource::ItemLevel,
            "character_class_is" => RuleReadSource::CharacterClassIs {
                class: self.class(args[0])?,
            },
            "character_ascendancy_is" => RuleReadSource::CharacterAscendancyIs {
                ascendancy: self.ascendancy(args[0])?,
            },
            "action_part_is" => RuleReadSource::ActionPartIs {
                part: self.part(args[0])?,
            },
            "action_mode_is" => RuleReadSource::ActionModeIs {
                mode: self.mode(args[0])?,
            },
            "action_stat_set_is" => RuleReadSource::ActionStatSetIs {
                stat_set: self.stat_set(args[0])?,
            },
            "item_quality_amount" => RuleReadSource::ItemQualityAmount {
                quality: self.quality(args[0])?,
            },
            "has_item_quality" => RuleReadSource::HasItemQuality {
                quality: self.quality(args[0])?,
            },
            "gem_quality_amount" => RuleReadSource::GemQualityAmount {
                quality: self.quality(args[0])?,
            },
            "has_gem_quality" => RuleReadSource::HasGemQuality {
                quality: self.quality(args[0])?,
            },
            "player_equipment_slot" => RuleReadSource::PlayerEquipmentSlot {
                slot: self.equipment_slot(args[0])?,
                read: self.equipment_read(args[1])?,
            },
            "stat" => RuleReadSource::Stat {
                entity: self.entity(args[0])?,
                stat: self.stat(args[1])?,
            },
            "capability" => RuleReadSource::Capability {
                entity: self.entity(args[0])?,
                capability: self.capability(args[1])?,
            },
            "external" => RuleReadSource::External {
                entity: self.entity(args[0])?,
                input: self.external(args[1])?,
            },
            "modifier_transforms" => RuleReadSource::ModifierTransforms {
                stat: self.stat(args[0])?,
                initial: self.stat(args[1])?,
            },
            "contributions" => RuleReadSource::Contributions {
                entity: self.entity(args[0])?,
                stat: self.stat(args[1])?,
                contribution: self.contribution(args[2])?,
                reduction: self.reduction(args[3])?,
                empty: self.literal(args[4], bindings)?,
            },
            "contribution_query" => RuleReadSource::ContributionQuery {
                entity: self.entity(args[0])?,
                query: self.key(args[1])?,
                group: self.key(args[2])?,
            },
            "contribution_selection" => RuleReadSource::ContributionSelection {
                entity: self.entity(args[0])?,
                query: self.key(args[1])?,
                group: self.key(args[2])?,
                projection: self.keyword(
                    args[3],
                    &[
                        ("present", ContributionSelectionProjection::Present),
                        ("value", ContributionSelectionProjection::Value),
                    ],
                )?,
            },
            _ => unreachable!("constructor validated above"),
        })
    }

    fn value<'a>(
        &self,
        id: ExprId,
        locals: &BTreeMap<&str, &'a LoweredBinding>,
    ) -> Result<&'a LoweredBinding, Diagnostic> {
        let name = path_name(self.syntax, id)?;
        locals.get(name.as_str()).copied().ok_or_else(|| {
            error(
                DiagnosticCode::UnknownSymbol,
                self.span(id),
                "effect values must name an earlier let declaration",
            )
        })
    }

    fn boolean_value(
        &self,
        id: ExprId,
        locals: &BTreeMap<&str, &LoweredBinding>,
    ) -> Result<OwnedDefinitionKey, Diagnostic> {
        let value = self.value(id, locals)?;
        if value.value_type != ComputedValueType::Boolean {
            return Err(error(
                DiagnosticCode::TypeMismatch,
                self.span(id),
                "effect requires a Boolean value",
            ));
        }
        Ok(value.value.clone())
    }

    fn targets(&self, id: ExprId) -> Result<Vec<ModifierTransformTarget>, Diagnostic> {
        let (name, args) = self.call(id)?;
        if name != "targets" {
            return Err(error(
                DiagnosticCode::InvalidArguments,
                self.span(id),
                "expected targets constructor",
            ));
        }
        args.iter()
            .map(|id| {
                let (name, args) = self.call(*id)?;
                if name != "target" || !(1..=2).contains(&args.len()) {
                    return Err(error(
                        DiagnosticCode::InvalidArguments,
                        self.span(*id),
                        "expected target(modifier[, eligibility_stat])",
                    ));
                }
                Ok(ModifierTransformTarget {
                    definition: self.modifier(args[0])?,
                    when: args.get(1).map(|id| self.stat(*id)).transpose()?,
                })
            })
            .collect()
    }

    fn effect(
        &self,
        id: ExprId,
        locals: &BTreeMap<&str, &LoweredBinding>,
        bindings: &ExpressionBindings,
    ) -> Result<RuleEffectKind, Diagnostic> {
        let (name, args) = self.call(id)?;
        let count = match name.as_str() {
            "contribute" => 4,
            "project_modifier_transform" => 5,
            "derive" | "capability" | "project_skill_parameter" | "project_actor_stat" => 3,
            "support_applicability" => 1,
            "activate_grant" | "requirement" => 2,
            _ => {
                return Err(error(
                    DiagnosticCode::UnknownFunction,
                    self.span(id),
                    "unknown effect constructor",
                ));
            }
        };
        self.arity(id, args, count)?;
        Ok(match name.as_str() {
            "contribute" => RuleEffectKind::Contribute {
                entity: self.entity(args[0])?,
                stat: self.stat(args[1])?,
                contribution: self.contribution(args[2])?,
                value: self.value(args[3], locals)?.value.clone(),
            },
            "derive" => RuleEffectKind::Derive {
                entity: self.entity(args[0])?,
                stat: self.stat(args[1])?,
                value: self.value(args[2], locals)?.value.clone(),
            },
            "capability" => RuleEffectKind::Capability {
                entity: self.entity(args[0])?,
                capability: self.capability(args[1])?,
                enabled: self.boolean_value(args[2], locals)?,
            },
            "support_applicability" => RuleEffectKind::SupportApplicability {
                applicable: self.boolean_value(args[0], locals)?,
            },
            "activate_grant" => RuleEffectKind::ActivateGrant {
                slot: self.grant(args[0])?,
                enabled: self.boolean_value(args[1], locals)?,
            },
            "project_skill_parameter" => RuleEffectKind::ProjectSkillParameter {
                skill: self.skill(args[0])?,
                parameter: self.parameter(args[1])?,
                value: self.value(args[2], locals)?.value.clone(),
            },
            "project_actor_stat" => RuleEffectKind::ProjectActorStat {
                actor: self.actor(args[0])?,
                stat: self.stat(args[1])?,
                value: self.value(args[2], locals)?.value.clone(),
            },
            "requirement" => RuleEffectKind::Requirement {
                satisfied: self.boolean_value(args[0], locals)?,
                code: self.key(args[1])?,
            },
            "project_modifier_transform" => {
                let ParameterValue::Integer(order) = self.literal(args[2], bindings)? else {
                    return Err(error(
                        DiagnosticCode::TypeMismatch,
                        self.span(args[2]),
                        "transform order must be an integer literal",
                    ));
                };
                if order.get() < 0 {
                    return Err(error(
                        DiagnosticCode::InvalidArguments,
                        self.span(args[2]),
                        "transform order must be nonnegative",
                    ));
                }
                RuleEffectKind::ProjectModifierTransform {
                    stat: self.stat(args[0])?,
                    targets: self.targets(args[1])?,
                    order,
                    operation: self.keyword(
                        args[3],
                        &[
                            ("add", ModifierTransformOperation::Add),
                            ("multiply", ModifierTransformOperation::Multiply),
                        ],
                    )?,
                    value: self.value(args[4], locals)?.value.clone(),
                }
            }
            _ => unreachable!("constructor validated above"),
        })
    }
}
