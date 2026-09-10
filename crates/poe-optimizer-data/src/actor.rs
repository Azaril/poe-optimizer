//! Portable actor modifier records and source-derived actor preparation rules.
//! These records preserve source order and unsupported downstream mechanics are
//! still rejected by native build admission. No game calculation or Lua lives here.
use crate::game_data::GameDataError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorStat {
    Str,
    Dex,
    Int,
    Life,
    Mana,
    Spirit,
    Accuracy,
    ActionSpeed,
    TemporalChainsActionSpeed,
    MinimumActionSpeed,
    MaximumActionSpeedReduction,
    UnaffectedBySlows,
    MovementSpeed,
    IgnoreMovementPenalties,
    MovementSpeedCannotBeBelowBase,
    Armour,
    Evasion,
    EnergyShield,
    ArmourAndEvasion,
    ArmourAndEnergyShield,
    EvasionAndEnergyShield,
    Defences,
    FireResist,
    ColdResist,
    LightningResist,
    ChaosResist,
    ElementalResist,
    ExtraLife,
    ExtraMana,
    ExtraSpirit,
    LifeTotal,
    ManaTotal,
    SpiritTotal,
    LifeConvertToEnergyShield,
    LifeConvertToArmour,
    LifeConvertToEvasion,
    ManaConvertToEnergyShield,
    ManaConvertToArmour,
    ManaConvertToEvasion,
    SpiritConvertToEnergyShield,
    SpiritConvertToArmour,
    SpiritConvertToEvasion,
    DexAccBonusOverride,
    LowLifePercentage,
    FullLifePercentage,
    NoAttributeBonuses,
    DoubledInherentAttributeBonuses,
    NoStrengthAttributeBonuses,
    NoStrBonusToLife,
    HalvesLifeFromStrength,
    NoDexterityAttributeBonuses,
    NoDexBonusToAccuracy,
    NoIntelligenceAttributeBonuses,
    NoIntBonusToMana,
    ChaosInoculation,
}
impl ActorStat {
    pub const fn upstream_name(self) -> &'static str {
        match self {
            Self::Str => "Str",
            Self::Dex => "Dex",
            Self::Int => "Int",
            Self::Life => "Life",
            Self::Mana => "Mana",
            Self::Spirit => "Spirit",
            Self::Accuracy => "Accuracy",
            Self::ActionSpeed => "ActionSpeed",
            Self::TemporalChainsActionSpeed => "TemporalChainsActionSpeed",
            Self::MinimumActionSpeed => "MinimumActionSpeed",
            Self::MaximumActionSpeedReduction => "MaximumActionSpeedReduction",
            Self::UnaffectedBySlows => "UnaffectedBySlows",
            Self::MovementSpeed => "MovementSpeed",
            Self::IgnoreMovementPenalties => "Condition:IgnoreMovementPenalties",
            Self::MovementSpeedCannotBeBelowBase => "MovementSpeedCannotBeBelowBase",
            Self::Armour => "Armour",
            Self::Evasion => "Evasion",
            Self::EnergyShield => "EnergyShield",
            Self::ArmourAndEvasion => "ArmourAndEvasion",
            Self::ArmourAndEnergyShield => "ArmourAndEnergyShield",
            Self::EvasionAndEnergyShield => "EvasionAndEnergyShield",
            Self::Defences => "Defences",
            Self::FireResist => "FireResist",
            Self::ColdResist => "ColdResist",
            Self::LightningResist => "LightningResist",
            Self::ChaosResist => "ChaosResist",
            Self::ElementalResist => "ElementalResist",
            Self::ExtraLife => "ExtraLife",
            Self::ExtraMana => "ExtraMana",
            Self::ExtraSpirit => "ExtraSpirit",
            Self::LifeTotal => "LifeTotal",
            Self::ManaTotal => "ManaTotal",
            Self::SpiritTotal => "SpiritTotal",
            Self::LifeConvertToEnergyShield => "LifeConvertToEnergyShield",
            Self::LifeConvertToArmour => "LifeConvertToArmour",
            Self::LifeConvertToEvasion => "LifeConvertToEvasion",
            Self::ManaConvertToEnergyShield => "ManaConvertToEnergyShield",
            Self::ManaConvertToArmour => "ManaConvertToArmour",
            Self::ManaConvertToEvasion => "ManaConvertToEvasion",
            Self::SpiritConvertToEnergyShield => "SpiritConvertToEnergyShield",
            Self::SpiritConvertToArmour => "SpiritConvertToArmour",
            Self::SpiritConvertToEvasion => "SpiritConvertToEvasion",
            Self::DexAccBonusOverride => "DexAccBonusOverride",
            Self::LowLifePercentage => "LowLifePercentage",
            Self::FullLifePercentage => "FullLifePercentage",
            Self::NoAttributeBonuses => "NoAttributeBonuses",
            Self::DoubledInherentAttributeBonuses => "DoubledInherentAttributeBonuses",
            Self::NoStrengthAttributeBonuses => "NoStrengthAttributeBonuses",
            Self::NoStrBonusToLife => "NoStrBonusToLife",
            Self::HalvesLifeFromStrength => "HalvesLifeFromStrength",
            Self::NoDexterityAttributeBonuses => "NoDexterityAttributeBonuses",
            Self::NoDexBonusToAccuracy => "NoDexBonusToAccuracy",
            Self::NoIntelligenceAttributeBonuses => "NoIntelligenceAttributeBonuses",
            Self::NoIntBonusToMana => "NoIntBonusToMana",
            Self::ChaosInoculation => "ChaosInoculation",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorCondition {
    TwoHighestAttributesEqual,
    DexHigherThanInt,
    StrHigherThanInt,
    IntHigherThanDex,
    StrHigherThanDex,
    IntHigherThanStr,
    DexHigherThanStr,
    StrHighestAttribute,
    IntHighestAttribute,
    DexHighestAttribute,
    IntSingleHighestAttribute,
    DexSingleHighestAttribute,
    IgnoreMovementPenalties,
}
impl ActorCondition {
    pub const fn upstream_name(self) -> &'static str {
        match self {
            Self::TwoHighestAttributesEqual => "TwoHighestAttributesEqual",
            Self::DexHigherThanInt => "DexHigherThanInt",
            Self::StrHigherThanInt => "StrHigherThanInt",
            Self::IntHigherThanDex => "IntHigherThanDex",
            Self::StrHigherThanDex => "StrHigherThanDex",
            Self::IntHigherThanStr => "IntHigherThanStr",
            Self::DexHigherThanStr => "DexHigherThanStr",
            Self::StrHighestAttribute => "StrHighestAttribute",
            Self::IntHighestAttribute => "IntHighestAttribute",
            Self::DexHighestAttribute => "DexHighestAttribute",
            Self::IntSingleHighestAttribute => "IntSingleHighestAttribute",
            Self::DexSingleHighestAttribute => "DexSingleHighestAttribute",
            Self::IgnoreMovementPenalties => "IgnoreMovementPenalties",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorNumericOperation {
    Base,
    Increased,
    More,
    Override,
    Max,
}
impl ActorNumericOperation {
    pub const fn upstream_name(self) -> &'static str {
        match self {
            Self::Base => "BASE",
            Self::Increased => "INC",
            Self::More => "MORE",
            Self::Override => "OVERRIDE",
            Self::Max => "MAX",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorGlobalEffectType {
    Global,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActorModifierTag {
    /// Exact source scope marker. It prevents local item consumption while
    /// remaining numerically neutral in global modifier queries.
    Global,
    /// Exact original metadata on the global minimum-action-speed aliases.
    GlobalEffect {
        effect_type: ActorGlobalEffectType,
        unscalable: bool,
    },
    /// Ordered OR variables; separate tags combine in source order.
    Condition {
        variables: Vec<ActorCondition>,
        negated: bool,
    },
}
// Serde ignores extra map fields on an internally tagged unit variant, even
// with deny_unknown_fields. An empty struct variant makes Global fail closed
// without changing the convenient public unit variant or its wire encoding.
impl<'de> Deserialize<'de> for ActorModifierTag {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
        enum WireTag {
            Global {},
            GlobalEffect {
                effect_type: ActorGlobalEffectType,
                unscalable: bool,
            },
            Condition {
                variables: Vec<ActorCondition>,
                negated: bool,
            },
        }
        Ok(match WireTag::deserialize(deserializer)? {
            WireTag::Global {} => Self::Global,
            WireTag::GlobalEffect {
                effect_type,
                unscalable,
            } => Self::GlobalEffect {
                effect_type,
                unscalable,
            },
            WireTag::Condition { variables, negated } => Self::Condition { variables, negated },
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActorModifierEffect {
    Numeric {
        operation: ActorNumericOperation,
        value: f64,
    },
    Flag {
        value: bool,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorModifierRecord {
    pub stat: ActorStat,
    pub effect: ActorModifierEffect,
    pub source: Option<String>,
    pub flags: u64,
    pub keyword_flags: u64,
    pub tags: Vec<ActorModifierTag>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorCaptureKind {
    /// Source [%+%-][%d%.]+ form; a leading sign is required.
    SignedDecimal,
    UnsignedInteger,
    UnsignedDecimal,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActorRuleValue {
    /// Zero-based capture index; negative source forms retain their multiplier.
    Capture {
        index: u32,
        multiplier: f64,
    },
    /// Preserve a literal source division rather than reciprocal multiplication.
    CaptureDivided {
        index: u32,
        divisor: f64,
    },
    Constant {
        value: f64,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActorRuleEffect {
    Numeric {
        operation: ActorNumericOperation,
        value: ActorRuleValue,
    },
    Flag {
        value: bool,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorModifierMapping {
    pub stat: ActorStat,
    pub effect: ActorRuleEffect,
    pub flags: u64,
    pub keyword_flags: u64,
    pub tags: Vec<ActorModifierTag>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorModifierRule {
    pub id: String,
    pub template: String,
    pub captures: Vec<ActorCaptureKind>,
    pub modifiers: Vec<ActorModifierMapping>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorQuestModifier {
    pub config_key: String,
    pub default_enabled: bool,
    pub modifiers: Vec<ActorModifierRecord>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorData {
    pub initial_spirit: f64,
    pub minimum_spirit: f64,
    /// Fractions as supplied to the source function, before percentage output.
    pub low_life_threshold: f64,
    pub full_life_threshold: f64,
    pub attribute_bonus_multiplier: f64,
    pub doubled_attribute_bonus_multiplier: f64,
    pub halved_life_per_strength: f64,
    pub chaos_inoculation_life: f64,
    /// Complete source highPrecisionMods table, including BASE entries. MORE
    /// consumers explicitly select More; no primitive pinned default is implied.
    pub high_precision_mods: BTreeMap<String, BTreeMap<ActorNumericOperation, u8>>,
    pub spirit_quests: Vec<ActorQuestModifier>,
    pub modifier_rules: Vec<ActorModifierRule>,
}
impl ActorData {
    pub fn modifier_rule(&self, id: &str) -> Option<&ActorModifierRule> {
        self.modifier_rules.iter().find(|rule| rule.id == id)
    }
}
fn invalid(message: &str) -> GameDataError {
    GameDataError(message.into())
}
fn bounded(value: f64) -> bool {
    value.is_finite() && value.abs() <= 1_000_000.0
}
impl ActorStat {
    /// Source local armour aliases. Global assembly must reject these unless
    /// an equipped armour item consumes them before building actor programs.
    pub const fn is_local_armour_only(self) -> bool {
        matches!(
            self,
            Self::ArmourAndEnergyShield | Self::EvasionAndEnergyShield
        )
    }
    pub const fn is_receiving_defence(self) -> bool {
        matches!(
            self,
            Self::Armour
                | Self::Evasion
                | Self::EnergyShield
                | Self::ArmourAndEvasion
                | Self::Defences
                | Self::FireResist
                | Self::ColdResist
                | Self::LightningResist
                | Self::ChaosResist
                | Self::ElementalResist
        )
    }
    pub const fn is_action_speed(self) -> bool {
        matches!(
            self,
            Self::ActionSpeed
                | Self::TemporalChainsActionSpeed
                | Self::MinimumActionSpeed
                | Self::MaximumActionSpeedReduction
                | Self::UnaffectedBySlows
        )
    }
    pub const fn is_movement(self) -> bool {
        matches!(
            self,
            Self::MovementSpeed
                | Self::IgnoreMovementPenalties
                | Self::MovementSpeedCannotBeBelowBase
        )
    }
    pub const fn is_flag(self) -> bool {
        use ActorStat::*;
        matches!(
            self,
            NoAttributeBonuses
                | DoubledInherentAttributeBonuses
                | NoStrengthAttributeBonuses
                | NoStrBonusToLife
                | HalvesLifeFromStrength
                | NoDexterityAttributeBonuses
                | NoDexBonusToAccuracy
                | NoIntelligenceAttributeBonuses
                | NoIntBonusToMana
                | ChaosInoculation
                | IgnoreMovementPenalties
                | MovementSpeedCannotBeBelowBase
                | UnaffectedBySlows
        )
    }
    pub const fn admits_operation(self, operation: ActorNumericOperation) -> bool {
        use ActorNumericOperation::*;
        use ActorStat::*;
        match self {
            Str | Dex | Int | Life | Mana | Spirit | Accuracy | MovementSpeed => {
                matches!(operation, Base | Increased | More | Override)
            }
            ActionSpeed | TemporalChainsActionSpeed => matches!(operation, Increased),
            MinimumActionSpeed | MaximumActionSpeedReduction => matches!(operation, Max),
            Armour
            | Evasion
            | EnergyShield
            | ArmourAndEvasion
            | ArmourAndEnergyShield
            | EvasionAndEnergyShield
            | FireResist
            | ColdResist
            | LightningResist
            | ChaosResist
            | ElementalResist => {
                matches!(operation, Base | Increased)
            }
            Defences => matches!(operation, Increased),
            DexAccBonusOverride => matches!(operation, Override),
            ExtraLife
            | ExtraMana
            | ExtraSpirit
            | LifeTotal
            | ManaTotal
            | SpiritTotal
            | LifeConvertToEnergyShield
            | LifeConvertToArmour
            | LifeConvertToEvasion
            | ManaConvertToEnergyShield
            | ManaConvertToArmour
            | ManaConvertToEvasion
            | SpiritConvertToEnergyShield
            | SpiritConvertToArmour
            | SpiritConvertToEvasion
            | LowLifePercentage
            | FullLifePercentage => matches!(operation, Base),
            _ => false,
        }
    }
}
fn validate_scope(
    flags: u64,
    keyword_flags: u64,
    tags: &[ActorModifierTag],
) -> Result<(), GameDataError> {
    if flags != 0 || keyword_flags != 0 || tags.len() > 8 {
        return Err(invalid(
            "actor modifiers require global flags/keywords and at most eight audited tags",
        ));
    }
    for tag in tags {
        if let ActorModifierTag::Condition { variables, .. } = tag
            && (variables.is_empty() || variables.len() > 12)
        {
            return Err(invalid(
                "actor conditions require one to twelve ordered variables",
            ));
        }
    }
    Ok(())
}
fn validate_target_tags(stat: ActorStat, tags: &[ActorModifierTag]) -> Result<(), GameDataError> {
    for tag in tags {
        if let ActorModifierTag::GlobalEffect {
            effect_type,
            unscalable,
        } = tag
            && (stat != ActorStat::MinimumActionSpeed
                || *effect_type != ActorGlobalEffectType::Global
                || !unscalable)
        {
            return Err(invalid(
                "GlobalEffect requires MinimumActionSpeed, effect_type Global and unscalable true",
            ));
        }
    }
    if stat!=ActorStat::MovementSpeed && tags.iter().any(|tag| matches!(tag,ActorModifierTag::Condition{variables,..} if variables.contains(&ActorCondition::IgnoreMovementPenalties))) {
        return Err(invalid("dynamic movement condition is admitted only on MovementSpeed numeric records, preventing cycles and actor-stage feedback"));
    }
    if !stat.is_receiving_defence()
        && tags
            .iter()
            .any(|tag| matches!(tag, ActorModifierTag::Global))
    {
        return Err(invalid(
            "Global marker is admitted only for reviewed receiving-defence targets",
        ));
    }
    Ok(())
}
impl ActorModifierRecord {
    /// Validate portable numerical/scope shape. This does not certify that a full
    /// build implements downstream effects such as donor conversions or immunity.
    pub fn validate(&self) -> Result<(), GameDataError> {
        validate_scope(self.flags, self.keyword_flags, &self.tags)?;
        validate_target_tags(self.stat, &self.tags)?;
        if self
            .source
            .as_ref()
            .is_some_and(|source| source.len() > 256 || source.chars().any(char::is_control))
        {
            return Err(invalid("actor modifier source is not bounded plain text"));
        }
        match self.effect {
            ActorModifierEffect::Numeric { operation, value }
                if self.stat.admits_operation(operation) && bounded(value) =>
            {
                Ok(())
            }
            ActorModifierEffect::Flag { .. } if self.stat.is_flag() => Ok(()),
            _ => Err(invalid(
                "actor target, operation and finite value do not match",
            )),
        }
    }
}
impl ActorModifierRule {
    /// Literal segments surrounding zero to two ordered unique captures.
    /// Concrete number syntax and exact source text belong to the importer.
    pub fn template_literals(&self) -> Result<Vec<&str>, GameDataError> {
        if self.template.trim() != self.template
            || self.template.is_empty()
            || self.template.len() > 256
            || self.captures.len() > 2
        {
            return Err(invalid(
                "actor modifier template or capture count exceeds bounds",
            ));
        }
        let mut literals = Vec::with_capacity(self.captures.len() + 1);
        let mut remaining = self.template.as_str();
        for index in 0..self.captures.len() {
            let marker = format!("{{{index}}}");
            let Some((literal, rest)) = remaining.split_once(&marker) else {
                return Err(invalid(
                    "actor captures require ordered unique contiguous indexes",
                ));
            };
            if index > 0 && literal.is_empty() {
                return Err(invalid("adjacent actor captures are ambiguous"));
            }
            literals.push(literal);
            remaining = rest;
        }
        literals.push(remaining);
        if literals.iter().any(|literal| {
            literal
                .chars()
                .any(|c| c.is_control() || c.is_ascii_digit() || matches!(c, '{' | '}' | '.'))
        }) || literals.iter().all(|literal| literal.is_empty())
        {
            return Err(invalid(
                "actor modifier literals contain ambiguous numeric syntax",
            ));
        }
        Ok(literals)
    }
}
pub(crate) fn validate_high_precision_mods(
    data: &BTreeMap<String, BTreeMap<ActorNumericOperation, u8>>,
) -> Result<(), GameDataError> {
    if data.len() > 256 {
        return Err(invalid(
            "actor precision table exceeds bounded source model",
        ));
    }
    for (name, operations) in data {
        if name.is_empty()
            || name.len() > 128
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || operations.is_empty()
            || operations.len() > 4
            || operations.values().any(|places| *places > 15)
        {
            return Err(invalid(
                "actor precision record has unsupported name, operation count or places",
            ));
        }
    }
    Ok(())
}
pub(crate) fn validate_actor(data: &ActorData) -> Result<(), GameDataError> {
    for value in [
        data.initial_spirit,
        data.minimum_spirit,
        data.attribute_bonus_multiplier,
        data.doubled_attribute_bonus_multiplier,
        data.halved_life_per_strength,
        data.chaos_inoculation_life,
    ] {
        if !bounded(value) || value < 0.0 {
            return Err(invalid("actor constants must be finite and in 0..1000000"));
        }
    }
    for value in [data.low_life_threshold, data.full_life_threshold] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(invalid("actor pool thresholds must be finite fractions"));
        }
    }
    validate_high_precision_mods(&data.high_precision_mods)?;
    let mut quest_keys = BTreeSet::new();
    if data.spirit_quests.len() != 3 {
        return Err(invalid(
            "actor Spirit quests must retain the three reviewed config records",
        ));
    }
    for quest in &data.spirit_quests {
        if quest.config_key.is_empty()
            || quest.config_key.len() > 256
            || quest.config_key.chars().any(char::is_control)
            || !quest_keys.insert(&quest.config_key)
            || quest.modifiers.len() != 1
        {
            return Err(invalid(
                "actor Spirit quest keys must be unique bounded records",
            ));
        }
        let modifier = &quest.modifiers[0];
        modifier.validate()?;
        if modifier.stat != ActorStat::Spirit
            || !matches!(modifier.effect,ActorModifierEffect::Numeric{operation:ActorNumericOperation::Base,value} if value>=0.0)
            || !modifier.tags.is_empty()
        {
            return Err(invalid(
                "actor Spirit quest requires one untagged nonnegative BASE record",
            ));
        }
    }
    if data.modifier_rules.is_empty() || data.modifier_rules.len() > 512 {
        return Err(invalid(
            "actor modifier rules require one to 512 bounded records",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut templates = BTreeSet::new();
    for rule in &data.modifier_rules {
        if rule.id.is_empty()
            || rule.id.len() > 64
            || !rule
                .id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            || !ids.insert(&rule.id)
        {
            return Err(invalid(
                "actor rule IDs must be unique bounded lower-case keys",
            ));
        }
        rule.template_literals()?;
        if !templates.insert(rule.template.to_ascii_lowercase())
            || rule.modifiers.is_empty()
            || rule.modifiers.len() > 8
        {
            return Err(invalid(
                "actor rule templates and complete mapping counts must be unambiguous",
            ));
        }
        let mut used = BTreeSet::new();
        for mapping in &rule.modifiers {
            if matches!(
                mapping.stat,
                ActorStat::TemporalChainsActionSpeed | ActorStat::MaximumActionSpeedReduction
            ) {
                return Err(invalid(
                    "actor grammar cannot produce unimplemented curse or enemy action-speed sources",
                ));
            }
            validate_scope(mapping.flags, mapping.keyword_flags, &mapping.tags)?;
            validate_target_tags(mapping.stat, &mapping.tags)?;
            match mapping.effect {
                ActorRuleEffect::Numeric { operation, value }
                    if mapping.stat.admits_operation(operation) =>
                {
                    match value {
                        ActorRuleValue::Capture { index, multiplier }
                            if (index as usize) < rule.captures.len()
                                && bounded(multiplier)
                                && multiplier != 0.0 =>
                        {
                            used.insert(index);
                        }
                        ActorRuleValue::CaptureDivided { index, divisor }
                            if (index as usize) < rule.captures.len()
                                && bounded(divisor)
                                && divisor > 0.0 =>
                        {
                            used.insert(index);
                        }
                        ActorRuleValue::Constant { value } if bounded(value) => {}
                        _ => {
                            return Err(invalid(
                                "actor numeric mapping has invalid value or capture",
                            ));
                        }
                    }
                }
                ActorRuleEffect::Flag { .. } if mapping.stat.is_flag() => {}
                _ => return Err(invalid("actor rule target and operation are incompatible")),
            }
        }
        if used.len() != rule.captures.len() {
            return Err(invalid("actor rule contains an unconsumed numeric capture"));
        }
    }
    Ok(())
}

/// Ordered query membership extracted from the pinned receiving-defence stage.
/// Membership is a reviewed semantic capability, not a new balance constant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceivingDefenceQuery {
    pub stat: ActorStat,
    pub query_stats: Vec<ActorStat>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceivingDefenceData {
    pub resources: Vec<ReceivingDefenceQuery>,
    pub resistances: Vec<ReceivingDefenceQuery>,
}
impl ReceivingDefenceData {
    pub(crate) fn validate(&self) -> Result<(), GameDataError> {
        use ActorStat::*;
        let resources: &[(ActorStat, &[ActorStat])] = &[
            (Armour, &[Armour, ArmourAndEvasion, Defences]),
            (Evasion, &[Evasion, ArmourAndEvasion, Defences]),
            (EnergyShield, &[EnergyShield, Defences]),
        ];
        let resistances: &[(ActorStat, &[ActorStat])] = &[
            (FireResist, &[FireResist, ElementalResist]),
            (ColdResist, &[ColdResist, ElementalResist]),
            (LightningResist, &[LightningResist, ElementalResist]),
            (ChaosResist, &[ChaosResist]),
        ];
        for (actual, expected) in [
            (&self.resources, resources),
            (&self.resistances, resistances),
        ] {
            if actual.len() != expected.len()
                || actual
                    .iter()
                    .zip(expected)
                    .any(|(actual, (stat, queries))| {
                        actual.stat != *stat || actual.query_stats != *queries
                    })
            {
                return Err(invalid(
                    "receiving defence query membership/order differs from source-reviewed capability",
                ));
            }
        }
        Ok(())
    }
}
