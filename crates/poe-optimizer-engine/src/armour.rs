//! Fixed local armour assembly and borrowed per-slot receiving inputs.
//!
//! Item-local values remain separate from ordered global actor modifiers. Local
//! consumption follows literal Item.calcLocal; the admitted normalized IR has
//! no InSlot tags, while the raw primitive retains that exact source predicate.
use crate::{
    CompiledGameData,
    actor::{ActorError, CompiledActorModifiers},
    defence::round_to_integer,
    modifiers::{ModifierInput, ModifierKind, ModifierValue, NumericKind},
    weapon::consume_local_numeric,
};
use poe_optimizer_data::game_data::{
    ActorModifierEffect, ActorModifierRecord, ActorNumericOperation, ActorStat, EquipmentSlot,
};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ArmourBaseValues {
    pub armour: f64,
    pub evasion: f64,
    pub energy_shield: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmourStats {
    pub base_armour: f64,
    pub base_evasion: f64,
    pub base_energy_shield: f64,
    pub armour: f64,
    pub evasion: f64,
    pub energy_shield: f64,
}
/// Literal GetArmourDataValue arithmetic. Prepared items admit fixed values only;
/// this raw primitive does not authorize per-level item modifiers in profiles.
pub fn armour_data_value(fixed: f64, per_level: f64, level: u32) -> Result<f64, ActorError> {
    let value = fixed + round_to_integer(per_level * f64::from(level));
    if !fixed.is_finite() || !per_level.is_finite() || !value.is_finite() {
        return Err(ActorError("Armour data value must remain finite"));
    }
    Ok(value)
}
impl ArmourStats {
    fn value(self, stat: ActorStat) -> f64 {
        let fixed = match stat {
            ActorStat::Armour => self.armour,
            ActorStat::Evasion => self.evasion,
            ActorStat::EnergyShield => self.energy_shield,
            _ => unreachable!("validated receiving defence target"),
        };
        // GetArmourDataValue adds the separately rounded per-level term. That
        // term is zero in fixed-only prepared items; preserve even this addition.
        fixed + round_to_integer(0.0)
    }
}
/// The normalized source-neutral item's local/global partition. Validation of
/// numeric ranges, effects and tags remains mandatory before item preparation.
pub fn is_local_modifier(record: &ActorModifierRecord) -> bool {
    let ActorModifierEffect::Numeric { operation, .. } = record.effect else {
        return false;
    };
    let supported = match record.stat {
        ActorStat::Armour
        | ActorStat::Evasion
        | ActorStat::EnergyShield
        | ActorStat::ArmourAndEvasion
        | ActorStat::ArmourAndEnergyShield
        | ActorStat::EvasionAndEnergyShield => matches!(
            operation,
            ActorNumericOperation::Base | ActorNumericOperation::Increased
        ),
        ActorStat::Defences => operation == ActorNumericOperation::Increased,
        _ => false,
    };
    supported && record.flags == 0 && record.keyword_flags == 0 && record.tags.is_empty()
}
/// Original fixed local Item.lua calculations. Unconsumed records remain in
/// source order for caller inspection; this does not certify item legality.
pub fn assemble_local_armour(
    base: ArmourBaseValues,
    quality: u32,
    modifiers: &mut Vec<ModifierInput>,
) -> Result<ArmourStats, ActorError> {
    if quality > 20
        || [base.armour, base.evasion, base.energy_shield]
            .into_iter()
            .any(|v| !v.is_finite())
    {
        return Err(ActorError(
            "Local armour bases must be finite and quality in 0..20",
        ));
    }
    let mut query = |name, kind| {
        consume_local_numeric(modifiers, name, kind, 0)
            .map(|result| result.value)
            .map_err(|error| ActorError(error.0))
    };
    // Exact source query and addition order, including the asymmetric ES sums.
    let armour_base = query("Armour", NumericKind::Base)? + base.armour;
    let armour_evasion_base = query("ArmourAndEvasion", NumericKind::Base)?;
    let evasion_base = query("Evasion", NumericKind::Base)? + base.evasion;
    let evasion_es_base = query("EvasionAndEnergyShield", NumericKind::Base)?;
    let es_base = query("EnergyShield", NumericKind::Base)? + base.energy_shield;
    let armour_es_base = query("ArmourAndEnergyShield", NumericKind::Base)?;
    let armour_inc = query("Armour", NumericKind::Increased)?;
    let armour_evasion_inc = query("ArmourAndEvasion", NumericKind::Increased)?;
    let evasion_inc = query("Evasion", NumericKind::Increased)?;
    let evasion_es_inc = query("EvasionAndEnergyShield", NumericKind::Increased)?;
    let es_inc = query("EnergyShield", NumericKind::Increased)?;
    let armour_es_inc = query("ArmourAndEnergyShield", NumericKind::Increased)?;
    let defences_inc = query("Defences", NumericKind::Increased)?;
    let quality = 1.0 + f64::from(quality) / 100.0;
    let armour = round_to_integer(
        (armour_base + armour_evasion_base + armour_es_base)
            * (1.0 + (armour_inc + armour_evasion_inc + armour_es_inc + defences_inc) / 100.0)
            * quality,
    );
    let evasion = round_to_integer(
        (evasion_base + armour_evasion_base + evasion_es_base)
            * (1.0 + (evasion_inc + armour_evasion_inc + evasion_es_inc + defences_inc) / 100.0)
            * quality,
    );
    let energy_shield = round_to_integer(
        (es_base + evasion_es_base + armour_es_base)
            * (1.0 + (es_inc + armour_es_inc + evasion_es_inc + defences_inc) / 100.0)
            * quality,
    );
    if [armour, evasion, energy_shield]
        .into_iter()
        .any(|v| !v.is_finite())
    {
        return Err(ActorError("Local armour assembly must remain finite"));
    }
    Ok(ArmourStats {
        base_armour: base.armour,
        base_evasion: base.evasion,
        base_energy_shield: base.energy_shield,
        armour,
        evasion,
        energy_shield,
    })
}
/// Immutable local values and surviving source-ordered global contribution.
/// Owner identity is the exact CompiledGameData instance, not content equality.
#[derive(Debug, Clone)]
pub struct PreparedArmour {
    binding: Arc<()>,
    base_key: String,
    slot: EquipmentSlot,
    quality: u32,
    item_level: u32,
    stats: ArmourStats,
    consumed_modifier_count: usize,
    source_global_count: usize,
    global_records: Vec<ActorModifierRecord>,
    global_program: CompiledActorModifiers,
}
impl PreparedArmour {
    pub fn base_key(&self) -> &str {
        &self.base_key
    }
    pub fn slot(&self) -> EquipmentSlot {
        self.slot
    }
    pub fn quality(&self) -> u32 {
        self.quality
    }
    pub fn item_level(&self) -> u32 {
        self.item_level
    }
    pub fn stats(&self) -> ArmourStats {
        self.stats
    }
    pub fn consumed_modifier_count(&self) -> usize {
        self.consumed_modifier_count
    }
    /// Surviving authored records before source-generated base effects.
    pub fn source_global_records(&self) -> &[ActorModifierRecord] {
        &self.global_records[..self.source_global_count]
    }
    pub fn generated_global_records(&self) -> &[ActorModifierRecord] {
        &self.global_records[self.source_global_count..]
    }
    pub fn global_records(&self) -> &[ActorModifierRecord] {
        &self.global_records
    }
    pub fn global_program(&self) -> &CompiledActorModifiers {
        &self.global_program
    }
    /// Owned capacities including normalized source/tag allocations; excludes
    /// allocator metadata and shared data-owner Arc bookkeeping.
    pub fn owned_heap_bytes(&self) -> usize {
        self.base_key.capacity()
            + self.global_program.owned_heap_bytes()
            + self.global_records.capacity() * std::mem::size_of::<ActorModifierRecord>()
            + self
                .global_records
                .iter()
                .map(|record| {
                    record.source.as_ref().map_or(0, String::capacity)
                        + record.tags.capacity()
                            * std::mem::size_of::<poe_optimizer_data::game_data::ActorModifierTag>()
                        + record
                            .tags
                            .iter()
                            .map(|tag| match tag {
                                poe_optimizer_data::game_data::ActorModifierTag::Global => 0,
                                poe_optimizer_data::game_data::ActorModifierTag::Condition {
                                    variables,
                                    ..
                                } => {
                                    variables.capacity()
                                        * std::mem::size_of::<
                                            poe_optimizer_data::game_data::ActorCondition,
                                        >()
                                }
                            })
                            .sum::<usize>()
                })
                .sum::<usize>()
    }
    pub fn footprint(&self) -> usize {
        std::mem::size_of::<Self>() + self.owned_heap_bytes()
    }
}
/// Fixed supported equipment slots. Fields borrow prepared immutable components;
/// the complete actor path validates owner and slot before any calculation.
#[derive(Debug, Clone, Copy, Default)]
pub struct ArmourSlots<'a> {
    pub helmet: Option<&'a PreparedArmour>,
    pub gloves: Option<&'a PreparedArmour>,
    pub boots: Option<&'a PreparedArmour>,
    pub body_armour: Option<&'a PreparedArmour>,
}
impl ArmourSlots<'_> {
    pub(crate) fn validate(self, data: &CompiledGameData) -> Result<(), ActorError> {
        for (item, slot) in [
            (self.helmet, EquipmentSlot::Helmet),
            (self.gloves, EquipmentSlot::Gloves),
            (self.boots, EquipmentSlot::Boots),
            (self.body_armour, EquipmentSlot::BodyArmour),
        ] {
            if let Some(item) = item {
                if !Arc::ptr_eq(&item.binding, &data.actor_binding) {
                    return Err(ActorError(
                        "Armour component belongs to a different compiled dataset",
                    ));
                }
                if item.slot != slot {
                    return Err(ActorError(
                        "Armour component does not match its receiving slot",
                    ));
                }
            }
        }
        Ok(())
    }
    pub(crate) fn ordered_values(self, stat: ActorStat) -> [f64; 6] {
        [
            self.helmet.map_or(0.0, |item| item.stats.value(stat)),
            self.gloves.map_or(0.0, |item| item.stats.value(stat)),
            self.boots.map_or(0.0, |item| item.stats.value(stat)),
            self.body_armour.map_or(0.0, |item| item.stats.value(stat)),
            0.0,
            0.0,
        ]
    }
}
impl CompiledGameData {
    /// Prepare one item's local outputs and leftover actor contribution once.
    /// Explicit LevelReq and other structural item legality belong to the import
    /// boundary; item level never substitutes for actor level or requirements.
    pub fn prepare_armour(
        &self,
        base_key: &str,
        quality: u32,
        item_level: u32,
        records: &[ActorModifierRecord],
    ) -> Result<PreparedArmour, ActorError> {
        self.prepare_armour_internal(base_key, quality, item_level, None, records)
    }
    /// Source-aware item assembly. Generated base movement penalties require the
    /// exact Item modSource even when the item has no authored modifiers.
    pub fn prepare_armour_with_source(
        &self,
        base_key: &str,
        quality: u32,
        item_level: u32,
        source: &str,
        records: &[ActorModifierRecord],
    ) -> Result<PreparedArmour, ActorError> {
        self.prepare_armour_internal(base_key, quality, item_level, Some(source), records)
    }
    fn prepare_armour_internal(
        &self,
        base_key: &str,
        quality: u32,
        item_level: u32,
        source: Option<&str>,
        records: &[ActorModifierRecord],
    ) -> Result<PreparedArmour, ActorError> {
        if quality > 20 || !(1..=100).contains(&item_level) || records.len() > 512 {
            return Err(ActorError(
                "Armour requires quality 0..20, item level 1..100 and at most 512 records",
            ));
        }
        let base = self
            .snapshot()
            .package()
            .armour_base(base_key)
            .ok_or(ActorError("Unknown selected armour base"))?;
        let mut local = Vec::new();
        let mut globals = Vec::new();
        for record in records {
            record
                .validate()
                .map_err(|_| ActorError("Invalid normalized local armour record"))?;
            if is_local_modifier(record) {
                let ActorModifierEffect::Numeric { operation, value } = record.effect else {
                    unreachable!("local numeric predicate")
                };
                local.push(ModifierInput {
                    name: record.stat.upstream_name().into(),
                    kind: ModifierKind::Numeric(match operation {
                        ActorNumericOperation::Base => NumericKind::Base,
                        ActorNumericOperation::Increased => NumericKind::Increased,
                        _ => unreachable!("local numeric predicate"),
                    }),
                    value: ModifierValue::Number(value),
                    flags: record.flags,
                    keyword_flags: record.keyword_flags,
                    source: record.source.clone(),
                    tag_kinds: vec![],
                });
            } else {
                globals.push(record.clone());
            }
        }
        let consumed_modifier_count = local.len();
        let stats = assemble_local_armour(
            ArmourBaseValues {
                armour: base.armour,
                evasion: base.evasion,
                energy_shield: base.energy_shield,
            },
            quality,
            &mut local,
        )?;
        if !local.is_empty() {
            return Err(ActorError("Unconsumed local armour modifiers"));
        }
        let source_global_count = globals.len();
        if let Some(penalty) = base.movement_penalty {
            use poe_optimizer_data::game_data::{ActorRuleEffect, ActorRuleValue};
            let source = source.filter(|value| !value.is_empty()).ok_or(ActorError(
                "Generated armour movement penalties require an explicit item modifier source",
            ))?;
            let mapping = &self.snapshot().package().movement.penalty_modifier;
            let ActorRuleEffect::Numeric {
                operation,
                value:
                    ActorRuleValue::Capture {
                        index: 0,
                        multiplier,
                    },
            } = mapping.effect
            else {
                return Err(ActorError("Unsupported generated movement penalty mapping"));
            };
            let generated = ActorModifierRecord {
                stat: mapping.stat,
                effect: ActorModifierEffect::Numeric {
                    operation,
                    value: penalty * multiplier,
                },
                source: Some(source.into()),
                flags: mapping.flags,
                keyword_flags: mapping.keyword_flags,
                tags: mapping.tags.clone(),
            };
            generated
                .validate()
                .map_err(|_| ActorError("Invalid generated armour movement penalty"))?;
            globals.push(generated);
        }
        let global_program = self.compile_actor_modifiers(&globals)?;
        Ok(PreparedArmour {
            binding: self.actor_binding.clone(),
            base_key: base_key.into(),
            slot: base.slot,
            quality,
            item_level,
            stats,
            consumed_modifier_count,
            source_global_count,
            global_records: globals,
            global_program,
        })
    }
}
