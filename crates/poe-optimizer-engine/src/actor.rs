//! Shared actor attribute, inherent-bonus and maximum-resource preparation.
//!
//! Source inputs are normalized, ordered global records. Preparation reuses the
//! modifier/condition primitives and executes exactly two attribute passes. The
//! immutable result is bound to one compiled dataset and retains no modifier DB.
//! Optional receiving preparation shares the exact same source queries and final
//! attribute conditions. Reservation and conversion receivers remain unsupported.
use crate::{
    armour::ArmourSlots,
    character::{CharacterAttributes, CharacterInput},
    conditions::{
        ConditionActor, ConditionEnvironment, ConditionEnvironmentInput, ConditionVariables,
        ModifierTag,
    },
    data::CompiledGameData,
    defence::round_to_integer,
    modifiers::{
        ModifierDatabase, ModifierInput, ModifierKind, ModifierValue, MorePrecision, NumericKind,
        QueryContext, SumKind, TaggedModifierInput,
    },
    spark::SparkQuestRewards,
};
use poe_optimizer_data::game_data::{
    ActorCondition, ActorModifierEffect, ActorModifierRecord, ActorModifierTag,
    ActorNumericOperation, ActorStat,
};
use std::{error::Error, fmt, sync::Arc};

#[path = "actor_program.rs"]
mod program;
pub use program::{ActorModifierLayer, ActorScratch, CompiledActorModifiers};
#[path = "actor_receiving.rs"]
mod receiving;
pub use crate::action_speed::ActionSpeedOutput;
pub use crate::movement::MovementOutput;
pub use receiving::{ReceivingOutput, ReceivingScenario};

const CONDITIONS: [ActorCondition; 12] = [
    ActorCondition::TwoHighestAttributesEqual,
    ActorCondition::DexHigherThanInt,
    ActorCondition::StrHigherThanInt,
    ActorCondition::IntHigherThanDex,
    ActorCondition::StrHigherThanDex,
    ActorCondition::IntHigherThanStr,
    ActorCondition::DexHigherThanStr,
    ActorCondition::StrHighestAttribute,
    ActorCondition::IntHighestAttribute,
    ActorCondition::DexHighestAttribute,
    ActorCondition::IntSingleHighestAttribute,
    ActorCondition::DexSingleHighestAttribute,
];
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorQuestSelection {
    pub candlemass: bool,
    pub molten_shrine: bool,
    pub silent_hall: bool,
    /// Ordered selected-data quest slots, never inferred from character level.
    pub spirit: [bool; 3],
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorResourceOutput {
    pub attributes: CharacterAttributes,
    pub lowest_attribute: f64,
    pub total_attributes: f64,
    pub life: f64,
    pub mana: f64,
    pub spirit: f64,
    /// Global accuracy prerequisite: source floor/nonnegative order, before enemy distance.
    pub accuracy: f64,
    pub low_life_percentage: f64,
    pub full_life_percentage: f64,
    pub lowest_of_maximum_life_and_maximum_mana: f64,
    pub life_has_override: bool,
    pub mana_has_override: bool,
    pub spirit_has_override: bool,
    pub chaos_inoculation: bool,
    pub full_life_from_chaos_inoculation: bool,
    conditions: [bool; 12],
}
impl ActorResourceOutput {
    pub fn conditions(&self) -> impl Iterator<Item = (&'static str, bool)> {
        CONDITIONS
            .into_iter()
            .zip(self.conditions)
            .map(|(condition, value)| (condition.upstream_name(), value))
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorError(pub &'static str);
impl fmt::Display for ActorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl Error for ActorError {}
/// Numeric result and bounded input metadata only; no retained heap strings,
/// record collections, parsed source, query caches or foreign actor references.
#[derive(Debug, Clone)]
pub struct PreparedActorResources {
    binding: Arc<()>,
    level: u32,
    quests: ActorQuestSelection,
    character: CharacterInput,
    output: ActorResourceOutput,
    requires_downstream_defences: bool,
    requires_receiving_stage: bool,
    receiving: Option<(ReceivingScenario, ReceivingOutput)>,
    movement: MovementOutput,
    action_speed: ActionSpeedOutput,
}
impl PreparedActorResources {
    pub fn action_speed(&self) -> ActionSpeedOutput {
        self.action_speed
    }
    pub fn movement(&self) -> MovementOutput {
        self.movement
    }
    pub fn receiving(&self) -> Option<ReceivingOutput> {
        self.receiving.map(|(_, output)| output)
    }
    pub(crate) fn receiving_for(
        &self,
        scenario: ReceivingScenario,
    ) -> Result<Option<ReceivingOutput>, ActorError> {
        if let Some((prepared, output)) = self.receiving {
            if prepared != scenario {
                return Err(ActorError(
                    "Prepared receiving scenario differs from resistance penalty or quests",
                ));
            }
            Ok(Some(output))
        } else if self.requires_receiving_stage {
            Err(ActorError(
                "Receiver records require complete actor preparation",
            ))
        } else {
            Ok(None)
        }
    }
    pub fn values(&self) -> ActorResourceOutput {
        self.output
    }
    pub fn character_level(&self) -> u32 {
        self.level
    }
    pub fn quests(&self) -> ActorQuestSelection {
        self.quests
    }
    pub fn character(&self) -> &CharacterInput {
        &self.character
    }
    /// Attach independently validated scalar modifiers after preparation. Base
    /// attributes always remain bound. Complete receiving preparation also binds
    /// all defensive scalar fields; only offence fields can then change. Raw
    /// resource-only callers retain their explicit legacy scalar behaviour.
    /// Owner, level, quests, outputs and downstream-coverage guards remain bound.
    pub fn with_character(&self, character: &CharacterInput) -> Result<Self, ActorError> {
        character.validate().map_err(|error| ActorError(error.0))?;
        if self.character.attributes != character.attributes {
            return Err(ActorError(
                "Prepared actor base attributes differ from the selected character",
            ));
        }
        if self.receiving.is_some()
            && receiving::legacy_values(&self.character.modifiers)
                != receiving::legacy_values(&character.modifiers)
        {
            return Err(ActorError(
                "Prepared receiving values cannot be rebound to changed defensive scalars",
            ));
        }
        let mut rebound = self.clone();
        rebound.character = *character;
        Ok(rebound)
    }
    pub(crate) fn validate_profile(
        &self,
        data: &CompiledGameData,
        level: u32,
        quests: SparkQuestRewards,
        character: &CharacterInput,
    ) -> Result<(), ActorError> {
        if !Arc::ptr_eq(&self.binding, &data.actor_binding) {
            return Err(ActorError(
                "Prepared actor belongs to a different compiled dataset",
            ));
        }
        if self.level != level
            || self.character != *character
            || self.quests.candlemass != quests.candlemass
            || self.quests.molten_shrine != quests.molten_shrine
            || self.quests.silent_hall != quests.silent_hall
        {
            return Err(ActorError(
                "Prepared actor differs from the selected character or resource quests",
            ));
        }
        if self.requires_downstream_defences {
            return Err(ActorError(
                "Actor donor conversion or immunity requires unsupported receiving defence stages",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Copy)]
struct BuiltinRecord {
    stat: ActorStat,
    operation: ActorNumericOperation,
    value: f64,
    source: &'static str,
}
const EMPTY_RECORD: BuiltinRecord = BuiltinRecord {
    stat: ActorStat::Life,
    operation: ActorNumericOperation::Base,
    value: 0.0,
    source: "Base",
};
// Thirteen actor prefix records, eight receiving prefix records, three bonuses.
struct StackRecords {
    rows: [BuiltinRecord; 24],
    len: usize,
}
impl StackRecords {
    fn new() -> Self {
        Self {
            rows: [EMPTY_RECORD; 24],
            len: 0,
        }
    }
    fn push(
        &mut self,
        stat: ActorStat,
        operation: ActorNumericOperation,
        value: f64,
        source: &'static str,
    ) {
        self.rows[self.len] = BuiltinRecord {
            stat,
            operation,
            value,
            source,
        };
        self.len += 1;
    }
    fn as_slice(&self) -> &[BuiltinRecord] {
        &self.rows[..self.len]
    }
}
trait ActorQueries {
    fn sum(&self, kind: SumKind, names: &[&str]) -> Result<f64, ActorError>;
    fn more(&self, name: &str) -> Result<f64, ActorError>;
    fn max(&self, name: &str) -> Result<Option<f64>, ActorError>;
    fn sum_positive(&self, name: &str) -> Result<f64, ActorError>;
    fn override_value(&self, name: &str) -> Result<Option<f64>, ActorError>;
    fn flag(&self, name: &str) -> bool;
    fn update_conditions(&mut self, conditions: [bool; 12]) -> Result<(), ActorError>;
    fn set_movement_condition(&mut self, ignored: bool) -> Result<(), ActorError>;
    fn add_bonus(&mut self, record: BuiltinRecord);
    fn finish_bonuses(&mut self) -> Result<(), ActorError>;
}
// The empty-record compatibility path uses the same actor execution function
// and source-order records without allocating a modifier DB or condition maps.
impl ActorQueries for StackRecords {
    fn sum(&self, kind: SumKind, names: &[&str]) -> Result<f64, ActorError> {
        let operation = match kind {
            SumKind::Base => ActorNumericOperation::Base,
            SumKind::Increased => ActorNumericOperation::Increased,
        };
        let mut value = 0.0;
        for name in names {
            for row in self.as_slice() {
                if row.stat.upstream_name() == *name && row.operation == operation {
                    value += row.value;
                }
            }
        }
        Ok(value)
    }
    fn max(&self, _: &str) -> Result<Option<f64>, ActorError> {
        Ok(None)
    }
    fn sum_positive(&self, name: &str) -> Result<f64, ActorError> {
        Ok(self
            .as_slice()
            .iter()
            .filter(|row| {
                row.stat.upstream_name() == name
                    && row.operation == ActorNumericOperation::Increased
                    && row.value > 0.0
            })
            .fold(0.0, |sum, row| sum + row.value))
    }
    fn more(&self, _: &str) -> Result<f64, ActorError> {
        Ok(1.0)
    }
    fn override_value(&self, _: &str) -> Result<Option<f64>, ActorError> {
        Ok(None)
    }
    fn flag(&self, _: &str) -> bool {
        false
    }
    fn update_conditions(&mut self, _: [bool; 12]) -> Result<(), ActorError> {
        Ok(())
    }
    fn set_movement_condition(&mut self, _: bool) -> Result<(), ActorError> {
        Ok(())
    }
    fn add_bonus(&mut self, record: BuiltinRecord) {
        self.rows[self.len] = record;
        self.len += 1;
    }
    fn finish_bonuses(&mut self) -> Result<(), ActorError> {
        Ok(())
    }
}
fn numeric_kind(operation: ActorNumericOperation) -> NumericKind {
    match operation {
        ActorNumericOperation::Base => NumericKind::Base,
        ActorNumericOperation::Increased => NumericKind::Increased,
        ActorNumericOperation::More => NumericKind::More,
        ActorNumericOperation::Override => NumericKind::Override,
        ActorNumericOperation::Max => NumericKind::Max,
    }
}
fn tags(tags: &[ActorModifierTag]) -> Vec<ModifierTag> {
    tags.iter()
        .map(|tag| match tag {
            ActorModifierTag::Global => ModifierTag::Global,
            ActorModifierTag::GlobalEffect { unscalable, .. } => ModifierTag::GlobalEffect {
                effect_type: "Global".into(),
                unscalable: *unscalable,
            },
            ActorModifierTag::Condition { variables, negated } => ModifierTag::Condition {
                variables: ConditionVariables::Any(
                    variables
                        .iter()
                        .map(|condition| condition.upstream_name().into())
                        .collect(),
                ),
                negated: *negated,
            },
        })
        .collect()
}

fn builtin(row: BuiltinRecord) -> TaggedModifierInput {
    ModifierInput {
        name: row.stat.upstream_name().into(),
        kind: ModifierKind::Numeric(numeric_kind(row.operation)),
        value: ModifierValue::Number(row.value),
        flags: 0,
        keyword_flags: 0,
        source: Some(row.source.into()),
        tag_kinds: vec![],
    }
    .into()
}
struct ActorFlag {
    name: &'static str,
    value: bool,
    tags: Vec<ModifierTag>,
}
struct DatabaseQueries<'a> {
    layers: Vec<Vec<TaggedModifierInput>>,
    database: ModifierDatabase,
    flags: Vec<ActorFlag>,
    conditions: ConditionEnvironment,
    precision: &'a MorePrecision,
    attribute_conditions: [bool; 12],
}
fn condition_environment(
    values: [bool; 12],
    layer_count: usize,
    ignored_movement: bool,
) -> Result<ConditionEnvironment, ActorError> {
    let mut layers = vec![
        CONDITIONS
            .into_iter()
            .zip(values)
            .map(|(condition, value)| (condition.upstream_name().into(), value))
            .chain(std::iter::once((
                "IgnoreMovementPenalties".into(),
                ignored_movement,
            )))
            .collect(),
    ];
    layers.resize_with(layer_count, Default::default);
    ConditionEnvironment::try_new(ConditionEnvironmentInput {
        store_conditions: layers,
        actors: vec![ConditionActor::default()],
        ..ConditionEnvironmentInput::default()
    })
    .map_err(|_| ActorError("Invalid actor condition environment"))
}
impl<'a> DatabaseQueries<'a> {
    fn new(
        base: StackRecords,
        layers: &[Vec<ActorModifierRecord>],
        precision: &'a MorePrecision,
    ) -> Result<Self, ActorError> {
        let mut numeric_layers: Vec<Vec<TaggedModifierInput>> =
            vec![base.as_slice().iter().copied().map(builtin).collect()];
        let mut flags = vec![];
        for (index, layer) in layers.iter().enumerate() {
            if index > 0 {
                numeric_layers.push(vec![]);
            }
            for record in layer {
                match record.effect {
                    ActorModifierEffect::Numeric { operation, value } => numeric_layers[index]
                        .push(TaggedModifierInput {
                            modifier: ModifierInput {
                                name: record.stat.upstream_name().into(),
                                kind: ModifierKind::Numeric(numeric_kind(operation)),
                                value: ModifierValue::Number(value),
                                flags: record.flags,
                                keyword_flags: record.keyword_flags,
                                source: record.source.clone(),
                                tag_kinds: vec![],
                            },
                            tags: tags(&record.tags),
                        }),
                    ActorModifierEffect::Flag { value } => flags.push(ActorFlag {
                        name: record.stat.upstream_name(),
                        value,
                        tags: tags(&record.tags),
                    }),
                }
            }
        }
        let database = ModifierDatabase::try_new_tagged(numeric_layers.clone())
            .map_err(|_| ActorError("Invalid actor numeric database"))?;
        let layer_count = numeric_layers.len();
        Ok(Self {
            layers: numeric_layers,
            database,
            flags,
            conditions: condition_environment([false; 12], layer_count, false)?,
            precision,
            attribute_conditions: [false; 12],
        })
    }
}
impl ActorQueries for DatabaseQueries<'_> {
    fn sum(&self, kind: SumKind, names: &[&str]) -> Result<f64, ActorError> {
        self.database
            .sum_with_conditions(kind, &QueryContext::default(), names, &self.conditions)
            .map_err(|_| ActorError("Invalid actor BASE/INC query"))
    }
    fn max(&self, name: &str) -> Result<Option<f64>, ActorError> {
        self.database
            .max_with_conditions(&QueryContext::default(), &[name], &self.conditions)
            .map_err(|_| ActorError("Invalid actor MAX query"))
    }
    fn sum_positive(&self, name: &str) -> Result<f64, ActorError> {
        self.database
            .sum_positive_with_conditions(
                SumKind::Increased,
                &QueryContext::default(),
                name,
                &self.conditions,
            )
            .map_err(|_| ActorError("Invalid actor positive INC query"))
    }
    fn more(&self, name: &str) -> Result<f64, ActorError> {
        self.database
            .more_with_conditions(
                &QueryContext::default(),
                &[name],
                self.precision,
                &self.conditions,
            )
            .map_err(|_| ActorError("Invalid actor MORE query"))
    }
    fn override_value(&self, name: &str) -> Result<Option<f64>, ActorError> {
        self.database
            .override_with_conditions(&QueryContext::default(), &[name], &self.conditions)
            .map_err(|_| ActorError("Invalid actor override query"))
    }
    fn flag(&self, name: &str) -> bool {
        self.flags
            .iter()
            .any(|flag| flag.name == name && flag.value && self.conditions.matches(&flag.tags))
    }
    fn update_conditions(&mut self, conditions: [bool; 12]) -> Result<(), ActorError> {
        self.attribute_conditions = conditions;
        self.conditions = condition_environment(conditions, self.database.layer_count(), false)?;
        Ok(())
    }
    fn set_movement_condition(&mut self, ignored: bool) -> Result<(), ActorError> {
        self.conditions = condition_environment(
            self.attribute_conditions,
            self.database.layer_count(),
            ignored,
        )?;
        Ok(())
    }
    fn add_bonus(&mut self, record: BuiltinRecord) {
        self.layers[0].push(builtin(record));
    }
    fn finish_bonuses(&mut self) -> Result<(), ActorError> {
        self.database = ModifierDatabase::try_new_tagged(self.layers.clone())
            .map_err(|_| ActorError("Invalid inherent actor modifiers"))?;
        Ok(())
    }
}
fn attribute_conditions(attributes: CharacterAttributes) -> [bool; 12] {
    let CharacterAttributes {
        strength: s,
        dexterity: d,
        intelligence: i,
    } = attributes;
    let mut sorted = [s, d, i];
    sorted.sort_by(f64::total_cmp);
    [
        sorted[1] == sorted[2],
        d > i,
        s > i,
        i > d,
        s > d,
        i > s,
        d > s,
        s >= d && s >= i,
        i >= s && i >= d,
        d >= s && d >= i,
        i > s && i > d,
        d > s && d > i,
    ]
}
fn value(queries: &impl ActorQueries, name: &str) -> Result<f64, ActorError> {
    let base = queries.sum(SumKind::Base, &[name])?;
    // CalcTools.val does not inspect OVERRIDE and does not evaluate INC/MORE if BASE is zero.
    if base == 0.0 {
        Ok(0.0)
    } else {
        Ok(
            base * (1.0 + queries.sum(SumKind::Increased, &[name])? / 100.0)
                * queries.more(name)?,
        )
    }
}
fn finite(value: f64) -> Result<f64, ActorError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ActorError("Actor calculation must remain finite"))
    }
}
fn calculate(
    queries: &mut impl ActorQueries,
    compiled: &CompiledGameData,
) -> Result<ActorResourceOutput, ActorError> {
    let package = compiled.snapshot().package();
    let rules = &package.character;
    let actor = &package.actor;
    let mut attributes = CharacterAttributes::default();
    let mut conditions = [false; 12];
    for _ in 0..2 {
        attributes = CharacterAttributes {
            strength: round_to_integer(finite(value(queries, "Str")?)?).max(0.0),
            dexterity: round_to_integer(finite(value(queries, "Dex")?)?).max(0.0),
            intelligence: round_to_integer(finite(value(queries, "Int")?)?).max(0.0),
        };
        for attribute in [
            attributes.strength,
            attributes.dexterity,
            attributes.intelligence,
        ] {
            if attribute > 1_000_000.0 {
                return Err(ActorError(
                    "Resolved actor attributes exceed the supported finite integer scope",
                ));
            }
        }
        conditions = attribute_conditions(attributes);
        queries.update_conditions(conditions)?;
    }
    if !queries.flag("NoAttributeBonuses") {
        let multiplier = if queries.flag("DoubledInherentAttributeBonuses") {
            actor.doubled_attribute_bonus_multiplier
        } else {
            actor.attribute_bonus_multiplier
        };
        if !queries.flag("NoStrengthAttributeBonuses") && !queries.flag("NoStrBonusToLife") {
            let bonus = if queries.flag("HalvesLifeFromStrength") {
                actor.halved_life_per_strength
            } else {
                rules.life_per_strength
            };
            queries.add_bonus(BuiltinRecord {
                stat: ActorStat::Life,
                operation: ActorNumericOperation::Base,
                value: attributes.strength * bonus * multiplier,
                source: "Strength",
            });
        }
        if !queries.flag("NoDexterityAttributeBonuses") && !queries.flag("NoDexBonusToAccuracy") {
            let bonus = queries
                .override_value("DexAccBonusOverride")?
                .unwrap_or(rules.accuracy_per_dexterity);
            queries.add_bonus(BuiltinRecord {
                stat: ActorStat::Accuracy,
                operation: ActorNumericOperation::Base,
                value: attributes.dexterity * bonus * multiplier,
                source: "Dexterity",
            });
        }
        if !queries.flag("NoIntelligenceAttributeBonuses") && !queries.flag("NoIntBonusToMana") {
            queries.add_bonus(BuiltinRecord {
                stat: ActorStat::Mana,
                operation: ActorNumericOperation::Base,
                value: attributes.intelligence * rules.mana_per_intelligence * multiplier,
                source: "Intelligence",
            });
        }
    }
    queries.finish_bonuses()?;
    let low = queries.sum(SumKind::Base, &["LowLifePercentage"])?;
    let full = queries.sum(SumKind::Base, &["FullLifePercentage"])?;
    let low_life_percentage = 100.0
        * if low > 0.0 {
            low
        } else {
            actor.low_life_threshold
        };
    let full_life_percentage = 100.0
        * if full > 0.0 {
            full
        } else {
            actor.full_life_threshold
        };
    let chaos_inoculation = queries.flag("ChaosInoculation");
    let mut pools = [0.0; 3];
    let mut overrides = [false; 3];
    for (index, (name, extra, total, conversions, minimum)) in [
        (
            "Life",
            "ExtraLife",
            "LifeTotal",
            [
                "LifeConvertToEnergyShield",
                "LifeConvertToArmour",
                "LifeConvertToEvasion",
            ],
            rules.minimum_life,
        ),
        (
            "Mana",
            "ExtraMana",
            "ManaTotal",
            [
                "ManaConvertToEnergyShield",
                "ManaConvertToArmour",
                "ManaConvertToEvasion",
            ],
            rules.minimum_mana,
        ),
        (
            "Spirit",
            "ExtraSpirit",
            "SpiritTotal",
            [
                "SpiritConvertToEnergyShield",
                "SpiritConvertToArmour",
                "SpiritConvertToEvasion",
            ],
            actor.minimum_spirit,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let base = queries.sum(SumKind::Base, &[name])?;
        let extra = queries.sum(SumKind::Base, &[extra])?;
        let total = queries.sum(SumKind::Base, &[total])?;
        let inc = queries.sum(SumKind::Increased, &[name])?;
        let more = queries.more(name)?;
        let conversion = queries.sum(SumKind::Base, &conversions)?.min(100.0);
        let overridden = queries.override_value(name)?;
        overrides[index] = overridden.is_some();
        pools[index] = finite(overridden.unwrap_or_else(|| {
            round_to_integer(
                (base * (1.0 - conversion / 100.0) + extra) * (1.0 + inc / 100.0) * more + total,
            )
            .max(minimum)
        }))?;
    }
    if chaos_inoculation {
        pools[0] = actor.chaos_inoculation_life;
    }
    let accuracy = finite(
        queries.sum(SumKind::Base, &["Accuracy"])?
            * (1.0 + queries.sum(SumKind::Increased, &["Accuracy"])? / 100.0)
            * queries.more("Accuracy")?,
    )?
    .floor()
    .max(0.0);
    Ok(ActorResourceOutput {
        attributes,
        lowest_attribute: attributes
            .strength
            .min(attributes.dexterity)
            .min(attributes.intelligence),
        total_attributes: attributes.strength + attributes.dexterity + attributes.intelligence,
        life: pools[0],
        mana: pools[1],
        spirit: pools[2],
        accuracy,
        low_life_percentage: finite(low_life_percentage)?,
        full_life_percentage: finite(full_life_percentage)?,
        lowest_of_maximum_life_and_maximum_mana: pools[0].min(pools[1]),
        life_has_override: overrides[0],
        mana_has_override: overrides[1],
        spirit_has_override: overrides[2],
        chaos_inoculation,
        full_life_from_chaos_inoculation: chaos_inoculation,
        conditions,
    })
}
struct ActorOutputs {
    resources: ActorResourceOutput,
    receiving: Option<(ReceivingScenario, ReceivingOutput)>,
    movement: MovementOutput,
    action_speed: ActionSpeedOutput,
}
fn calculate_complete(
    queries: &mut impl ActorQueries,
    compiled: &CompiledGameData,
    scenario: Option<ReceivingScenario>,
    armour: ArmourSlots<'_>,
) -> Result<ActorOutputs, ActorError> {
    let actor = calculate(queries, compiled)?;
    let receiving = scenario
        .map(|scenario| {
            receiving::calculate(queries, compiled, armour).map(|output| (scenario, output))
        })
        .transpose()?;
    // Query order follows actionSpeedMod after final attribute conditions.
    let action_data = &compiled.snapshot().package().action_speed;
    let names = &action_data.query_stats;
    let minimum = queries.max(names[0].upstream_name())?;
    let maximum = queries.max(names[1].upstream_name())?;
    let unaffected = queries.flag(names[2].upstream_name());
    let increased = if unaffected {
        queries.sum_positive(names[3].upstream_name())?
    } else {
        queries.sum(SumKind::Increased, &[names[3].upstream_name()])?
    };
    let temporal = if unaffected {
        queries.sum_positive(names[4].upstream_name())?
    } else {
        queries.sum(SumKind::Increased, &[names[4].upstream_name()])?
    };
    let action_speed = crate::action_speed::calculate(
        action_data,
        minimum,
        maximum,
        increased,
        temporal,
        unaffected,
    )?;
    // Source GetCondition resolves this flag from the final attribute state.
    // Its own tags cannot reference this condition, so there is no query cycle.
    let ignored = queries.flag("Condition:IgnoreMovementPenalties");
    queries.set_movement_condition(ignored)?;
    let movement = &compiled.snapshot().package().movement;
    let overridden = queries.override_value("MovementSpeed")?;
    let (base, increased, more) = if overridden.is_some() {
        (0.0, 0.0, 1.0)
    } else {
        let mut names = [""; 1];
        for (name, stat) in names.iter_mut().zip(&movement.query_stats) {
            *name = stat.upstream_name();
        }
        (
            queries.sum(SumKind::Base, &names)?,
            queries.sum(SumKind::Increased, &names)?,
            queries.more("MovementSpeed")?,
        )
    };
    let movement = crate::movement::calculate(
        movement,
        crate::movement::MovementInput {
            base,
            increased,
            more,
            override_value: overridden,
            cannot_be_below_base: queries.flag("MovementSpeedCannotBeBelowBase"),
            ignore_movement_penalties: ignored,
            action_speed_mod: action_speed.action_speed_mod,
        },
    )?;
    Ok(ActorOutputs {
        resources: actor,
        receiving,
        movement,
        action_speed,
    })
}

impl CompiledGameData {
    pub fn actor_quest_selection(&self, quests: SparkQuestRewards) -> ActorQuestSelection {
        ActorQuestSelection {
            candlemass: quests.candlemass,
            molten_shrine: quests.molten_shrine,
            silent_hall: quests.silent_hall,
            spirit: std::array::from_fn(|i| {
                self.snapshot().package().actor.spirit_quests[i].default_enabled
            }),
        }
    }
    /// Prepare a fresh actor from explicit local/parent records. Conversion donors
    /// and CI follow the raw source function, but cannot enter the skill profiles
    /// until their downstream defence mechanics are implemented.
    pub fn prepare_actor_resources(
        &self,
        level: u32,
        quests: ActorQuestSelection,
        character: &CharacterInput,
        modifier_layers: &[Vec<ActorModifierRecord>],
    ) -> Result<PreparedActorResources, ActorError> {
        self.prepare_actor_internal(
            level,
            quests,
            None,
            character,
            modifier_layers,
            ArmourSlots::default(),
        )
    }
    /// Complete source-ordered actor and receiving-defence preparation. Receiver
    /// contributions must be ordered records, never aggregated legacy scalars.
    pub fn prepare_actor(
        &self,
        level: u32,
        quests: ActorQuestSelection,
        receiving: ReceivingScenario,
        character: &CharacterInput,
        modifier_layers: &[Vec<ActorModifierRecord>],
    ) -> Result<PreparedActorResources, ActorError> {
        self.prepare_actor_with_armour(
            level,
            quests,
            receiving,
            character,
            modifier_layers,
            ArmourSlots::default(),
        )
    }
    /// Complete preparation with separately rounded local armour in fixed slots.
    /// Callers must include each selected item's global_records exactly once in
    /// the ordered equipment layer. Slot components supply local bases only.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_actor_with_armour(
        &self,
        level: u32,
        quests: ActorQuestSelection,
        receiving: ReceivingScenario,
        character: &CharacterInput,
        modifier_layers: &[Vec<ActorModifierRecord>],
        armour: ArmourSlots<'_>,
    ) -> Result<PreparedActorResources, ActorError> {
        self.prepare_actor_internal(
            level,
            quests,
            Some(receiving),
            character,
            modifier_layers,
            armour,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn prepare_actor_internal(
        &self,
        level: u32,
        quests: ActorQuestSelection,
        receiving: Option<ReceivingScenario>,
        character: &CharacterInput,
        modifier_layers: &[Vec<ActorModifierRecord>],
        armour: ArmourSlots<'_>,
    ) -> Result<PreparedActorResources, ActorError> {
        armour.validate(self)?;
        character.validate().map_err(|error| ActorError(error.0))?;
        if let Some(scenario) = receiving {
            scenario.validate()?;
            receiving::validate_source_character(character)?;
        }
        if !(1..=100).contains(&level) {
            return Err(ActorError("Actor character level must be 1..100"));
        }
        if modifier_layers.len() > 16 || modifier_layers.iter().map(Vec::len).sum::<usize>() > 512 {
            return Err(ActorError(
                "Actor input exceeds sixteen layers or 512 normalized records",
            ));
        }
        for record in modifier_layers.iter().flatten() {
            validate_global_record(record)?;
        }
        let mut base = self.actor_base_records(level, quests, character);
        if let Some(scenario) = receiving {
            self.add_receiving_base(&mut base, scenario);
        }
        let ActorOutputs {
            resources: output,
            receiving: receiving_output,
            movement,
            action_speed,
        } = if modifier_layers.iter().all(Vec::is_empty) {
            calculate_complete(&mut base, self, receiving, armour)?
        } else {
            calculate_complete(
                &mut DatabaseQueries::new(base, modifier_layers, &self.actor_precision)?,
                self,
                receiving,
                armour,
            )?
        };
        let requires_downstream_defences = modifier_layers
            .iter()
            .flatten()
            .any(|record| requires_downstream_defences(record.stat));
        Ok(PreparedActorResources {
            binding: self.actor_binding.clone(),
            level,
            quests,
            character: *character,
            output,
            requires_downstream_defences,
            requires_receiving_stage: modifier_layers
                .iter()
                .flatten()
                .any(|record| record.stat.is_receiving_defence()),
            receiving: receiving_output,
            movement,
            action_speed,
        })
    }
    fn actor_base_records(
        &self,
        level: u32,
        quests: ActorQuestSelection,
        character: &CharacterInput,
    ) -> StackRecords {
        let package = self.snapshot().package();
        let rules = &package.character;
        let mut base = StackRecords::new();
        for (stat, value) in [
            (ActorStat::Str, character.attributes.strength),
            (ActorStat::Dex, character.attributes.dexterity),
            (ActorStat::Int, character.attributes.intelligence),
        ] {
            base.push(stat, ActorNumericOperation::Base, value, "Base");
        }
        for (stat, coefficient, offset) in [
            (ActorStat::Life, rules.life_per_level, rules.initial_life),
            (ActorStat::Mana, rules.mana_per_level, rules.initial_mana),
            (
                ActorStat::Accuracy,
                rules.accuracy_per_level,
                -rules.accuracy_per_level,
            ),
        ] {
            let value = crate::multipliers::apply_resolved_multiplier(
                coefficient,
                f64::from(level),
                1.0,
                offset,
                false,
                None,
            );
            base.push(stat, ActorNumericOperation::Base, value, "Base");
        }
        base.push(
            ActorStat::Spirit,
            ActorNumericOperation::Base,
            package.actor.initial_spirit,
            "Base",
        );
        for (enabled, stat, operation, value) in [
            (
                quests.candlemass,
                ActorStat::Life,
                ActorNumericOperation::Base,
                package.quests.flat_life,
            ),
            (
                quests.molten_shrine,
                ActorStat::Life,
                ActorNumericOperation::Increased,
                package.quests.life_increased,
            ),
            (
                quests.silent_hall,
                ActorStat::Mana,
                ActorNumericOperation::Increased,
                package.quests.mana_increased,
            ),
        ] {
            if enabled {
                base.push(stat, operation, value, "Config");
            }
        }
        for (enabled, quest) in quests.spirit.into_iter().zip(&package.actor.spirit_quests) {
            if enabled {
                let ActorModifierEffect::Numeric { operation, value } = quest.modifiers[0].effect
                else {
                    unreachable!("validated Spirit quest")
                };
                base.push(ActorStat::Spirit, operation, value, "Config");
            }
        }
        base
    }
}

fn requires_downstream_defences(stat: ActorStat) -> bool {
    matches!(
        stat,
        ActorStat::LifeConvertToEnergyShield
            | ActorStat::LifeConvertToArmour
            | ActorStat::LifeConvertToEvasion
            | ActorStat::ManaConvertToEnergyShield
            | ActorStat::ManaConvertToArmour
            | ActorStat::ManaConvertToEvasion
            | ActorStat::SpiritConvertToEnergyShield
            | ActorStat::SpiritConvertToArmour
            | ActorStat::SpiritConvertToEvasion
            | ActorStat::ChaosInoculation
    )
}

fn validate_global_record(record: &ActorModifierRecord) -> Result<(), ActorError> {
    record.validate().map_err(|_| ActorError("Invalid normalized actor target, operation, numeric bound, flags, source or condition tags"))?;
    if record.stat.is_local_armour_only() {
        return Err(ActorError(
            "Local-only armour modifiers require local armour consumption",
        ));
    }
    Ok(())
}
