//! Complete finite weapon-local producer over owner-injected names and operands.
use super::{
    AssemblyError, Context, ItemLoadProvider, Result, TableId, Value, finite, number, table,
};
use poe_optimizer_data::item_assembly::{
    ItemAssemblyWeaponDamageKind as DamageKind, ItemAssemblyWeaponFlagComparison as FlagComparison,
    ItemAssemblyWeaponNameFlags, ItemAssemblyWeaponResidualPolicy,
};

impl<P: ItemLoadProvider + ?Sized> Context<'_, '_, P> {
    pub(super) fn local_weapon(&mut self, list: TableId, slot: Option<u16>) -> Result<()> {
        self.stage = "local weapon";
        let p = &self.definitions.policy().weapon;
        let output = self.arena.new_table()?;
        let slots = self.root_table(&p.output_field)?;
        let key = slot.map_or(Value::Nil, |n| Value::Number(f64::from(n)));
        self.set_raw_key(slots, &key, Value::Table(output))?;
        let kind = self.base_field(&p.type_base_field)?;
        self.arena.set_field(output, &p.type_output, kind)?;
        let name = self.get(&p.name_item_field)?;
        self.arena.set_field(output, &p.name_output, name)?;

        let speed = self.local(list, &p.attack_speed.query)?;
        // Quality division precedes the alternate query; do not eagerly coerce
        // the first query's return or move this division after the next call.
        let quality = number(&self.get("quality")?)? / p.attack_speed.quality_divisor;
        let alternate = self.local(list, &p.attack_speed.alternate)?;
        let alternate = (quality * number(&alternate)?).floor();
        self.arena.set_field(
            output,
            &p.attack_speed.output,
            finite(number(&speed)? + alternate)?,
        )?;
        let base = self.weapon_field(&p.base_field, &p.attack_rate.base_field)?;
        let speed = self.arena.get_field(output, &p.attack_speed.output)?;
        let rate = number(&base)? * (p.fraction_base + number(&speed)? / p.percent_divisor);
        self.arena.set_field(
            output,
            &p.attack_rate.output,
            finite(super::query::round(rate, p.attack_rate.round_places))?,
        )?;

        let flat = self.local(list, &p.range.flat)?;
        let metres = self.local(list, &p.range.metres)?;
        let metres = p.range.metre_multiplier * number(&metres)?;
        let range = number(&flat)? + metres;
        let quality = number(&self.get("quality")?)? / p.range.quality_divisor;
        let alternate = self.local(list, &p.range.alternate)?;
        let range = range + (quality * number(&alternate)?).floor();
        self.arena
            .set_field(output, &p.range.bonus_output, finite(range)?)?;
        let base = self.weapon_field(&p.base_field, &p.range.base_field)?;
        let bonus = self.arena.get_field(output, &p.range.bonus_output)?;
        self.arena.set_field(
            output,
            &p.range.output,
            finite(number(&base)? + number(&bonus)?)?,
        )?;
        if self
            .weapon_field(&p.base_field, &p.reload.base_field)?
            .truthy()
        {
            let increased = self.local(list, &p.reload.query)?;
            let speed = self.arena.get_field(output, &p.attack_speed.output)?;
            self.arena.set_field(
                output,
                &p.reload.increased_output,
                finite(number(&increased)? + number(&speed)?)?,
            )?;
            let base = self.weapon_field(&p.base_field, &p.reload.base_field)?;
            let increased = self.arena.get_field(output, &p.reload.increased_output)?;
            let time = number(&base)? / (p.fraction_base + number(&increased)? / p.percent_divisor);
            self.arena.set_field(
                output,
                &p.reload.output,
                finite(super::query::round(time, p.reload.round_places))?,
            )?;
        }

        let elemental = self.local(list, &p.damage.elemental_increased)?;
        for channel in &p.damage.channels {
            let raw = self.weapon_field(&p.base_field, &channel.minimum.base_field)?;
            let raw = if raw.truthy() {
                raw
            } else {
                Value::Number(p.damage.base_default)
            };
            let added = self.local(list, &channel.minimum.query)?;
            let mut minimum = number(&raw)? + number(&added)?;
            let raw = self.weapon_field(&p.base_field, &channel.maximum.base_field)?;
            let raw = if raw.truthy() {
                raw
            } else {
                Value::Number(p.damage.base_default)
            };
            let added = self.local(list, &channel.maximum.query)?;
            let mut maximum = number(&raw)? + number(&added)?;
            match channel.kind {
                DamageKind::Physical => {
                    let increased = self.local(list, &p.damage.physical_increased)?;
                    let mut quality = self.get("quality")?;
                    let alternate = self.local(list, &p.damage.alternate_quality)?;
                    let Value::Number(alternate) = alternate else {
                        return Err(AssemblyError::source(
                            "attempt to compare item value with number",
                        ));
                    };
                    if alternate > p.damage.quality_threshold {
                        quality = Value::Number(p.damage.suppressed_quality);
                    }
                    minimum = super::query::round(
                        minimum
                            * (p.fraction_base + number(&increased)? / p.percent_divisor)
                            * (p.fraction_base + number(&quality)? / p.percent_divisor),
                        p.damage.round_places,
                    );
                    maximum = super::query::round(
                        maximum
                            * (p.fraction_base + number(&increased)? / p.percent_divisor)
                            * (p.fraction_base + number(&quality)? / p.percent_divisor),
                        p.damage.round_places,
                    );
                }
                DamageKind::Elemental => {
                    let query = channel
                        .increased
                        .as_ref()
                        .expect("validated elemental query");
                    let increased = self.local(list, query)?;
                    let increased = number(&increased)? + number(&elemental)?;
                    minimum = super::query::round(
                        minimum * (p.fraction_base + increased / p.percent_divisor),
                        p.damage.round_places,
                    );
                    maximum = super::query::round(
                        maximum * (p.fraction_base + increased / p.percent_divisor),
                        p.damage.round_places,
                    );
                }
                DamageKind::Unscaled => {}
            }
            if minimum > p.damage.positive_threshold && maximum > p.damage.positive_threshold {
                self.arena
                    .set_field(output, &channel.minimum.output, finite(minimum)?)?;
                self.arena
                    .set_field(output, &channel.maximum.output, finite(maximum)?)?;
                let rate = self.arena.get_field(output, &p.attack_rate.output)?;
                let dps = (minimum + maximum) / p.damage.average_divisor * number(&rate)?;
                self.arena
                    .set_field(output, &channel.dps_output, finite(dps)?)?;
                if channel.kind == DamageKind::Elemental {
                    let prior = self.arena.get_field(output, &p.damage.elemental_output)?;
                    let prior = if prior.truthy() {
                        prior
                    } else {
                        Value::Number(p.damage.elemental_default)
                    };
                    self.arena.set_field(
                        output,
                        &p.damage.elemental_output,
                        finite(number(&prior)? + dps)?,
                    )?;
                }
            }
        }

        let base = self.weapon_field(&p.base_field, &p.critical.base_field)?;
        let added = self.local(list, &p.critical.base)?;
        let critical = number(&base)? + number(&added)?;
        let increased = self.local(list, &p.critical.increased)?;
        let quality = number(&self.get("quality")?)? / p.critical.quality_divisor;
        let alternate = self.local(list, &p.critical.alternate)?;
        let alternate = (quality * number(&alternate)?).floor();
        let increased = number(&increased)? + alternate;
        let critical = critical * (p.fraction_base + increased / p.percent_divisor);
        self.arena.set_field(
            output,
            &p.critical.output,
            finite(super::query::round(critical, p.critical.round_places))?,
        )?;
        self.local_overrides(list, &Value::Table(output), &p.overrides)?;
        self.weapon_tags(list, slot, &p.residual)?;
        self.arena
            .set_field(output, &p.total_output, finite(p.total_initial)?)?;
        for channel in &p.damage.channels {
            let total = self.arena.get_field(output, &p.total_output)?;
            let value = self.arena.get_field(output, &channel.dps_output)?;
            let value = if value.truthy() {
                value
            } else {
                Value::Number(p.total_default)
            };
            self.arena.set_field(
                output,
                &p.total_output,
                finite(number(&total)? + number(&value)?)?,
            )?;
        }
        Ok(())
    }

    fn weapon_field(&mut self, branch: &str, field: &str) -> Result<Value> {
        let base = table(self.base_field(branch)?)?;
        self.arena.get_field(base, field)
    }
    fn weapon_name_flags(
        &mut self,
        record: TableId,
        rule: &ItemAssemblyWeaponNameFlags,
    ) -> Result<bool> {
        let name = self.arena.get_field(record, "name")?;
        let mut matches = false;
        for candidate in &rule.names {
            self.arena.work(1 + candidate.len() as u64)?;
            if name.as_str() == Some(candidate) {
                matches = true;
                break;
            }
        }
        if !matches {
            return Ok(false);
        }
        let flags = self.arena.get_field(record, "flags")?;
        let equal = flags == Value::Number(rule.flags.value);
        Ok(match rule.flags.operation {
            FlagComparison::Equal => equal,
            FlagComparison::NotEqual => !equal,
        })
    }
    fn weapon_tags(
        &mut self,
        list: TableId,
        slot: Option<u16>,
        p: &ItemAssemblyWeaponResidualPolicy,
    ) -> Result<()> {
        for index in 1..=self.arena.dense_len(list)? {
            let record = table(self.arena.get_index(list, index as i64)?)?;
            let mut plain = false;
            for rule in &p.untagged {
                if self.weapon_name_flags(record, rule)? {
                    plain = true;
                    break;
                }
            }
            if plain {
                let keyword = self.arena.get_field(record, "keywordFlags")?;
                plain = (keyword == Value::Number(p.keyword_flags[0])
                    || keyword == Value::Number(p.keyword_flags[1]))
                    && !self.arena.get_index(record, 1)?.truthy();
            }
            if plain {
                let tag = self.weapon_hand_tag(slot, p)?;
                self.arena.set_index(record, 1, Value::Table(tag))?;
            } else if self.weapon_name_flags(record, &p.critical)? {
                let first = self.arena.get_index(record, 1)?;
                let eligible = !first.truthy() || {
                    let tag = table(first)?;
                    self.arena.get_field(tag, "type")?.as_str() == Some(&p.condition_tag_type)
                        && self.arena.get_field(tag, "var")?.as_str() == Some(&p.critical_condition)
                        && !self.arena.get_index(record, 2)?.truthy()
                };
                if eligible {
                    let tag = self.weapon_hand_tag(slot, p)?;
                    self.arena.append(record, Value::Table(tag))?;
                }
            }
        }
        Ok(())
    }
    fn weapon_hand_tag(
        &mut self,
        slot: Option<u16>,
        p: &ItemAssemblyWeaponResidualPolicy,
    ) -> Result<TableId> {
        let tag = self.arena.new_table()?;
        let kind = self.arena.text(&p.condition_tag_type)?;
        self.arena.set_field(tag, "type", kind)?;
        let condition = if slot == Some(p.primary_slot) {
            &p.primary_condition
        } else {
            &p.other_condition
        };
        let value = self.arena.text(condition)?;
        self.arena.set_field(tag, "var", value)?;
        Ok(tag)
    }
}
