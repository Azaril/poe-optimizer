//! Ordered source local-data producers; names and constants come from the owner policy.
use super::{Context, ItemLoadProvider, Result, TableId, Value, finite, number, table};
use poe_optimizer_data::item_assembly::{
    ItemAssemblyArmourRole as Role, ItemAssemblyChargePolicy, ItemAssemblyOverridePolicy,
    ItemAssemblyQueryList,
};

impl<P: ItemLoadProvider + ?Sized> Context<'_, '_, P> {
    pub(super) fn local_armour(&mut self, list: TableId) -> Result<()> {
        self.stage = "local armour";
        let policy = &self.definitions.policy().armour;
        let base = self.base_field(&policy.base_field)?;
        let output = self.get(&policy.output_field)?;
        // A fixed stack array keeps the source's named locals without per-call policy clones.
        let mut values: [Value; 18] = std::array::from_fn(|_| Value::Nil);
        for query in &policy.queries {
            let value = self.local(list, &query.query)?;
            let value = if let Some(field) = &query.base {
                let base = self.arena.get_field(local_table(&base)?, &field.field)?;
                let base = if base.truthy() {
                    base
                } else {
                    Value::Number(field.default)
                };
                Value::Number(number(&value)? + number(&base)?)
            } else {
                value
            };
            values[role_index(query.role)] = value;
        }
        let mut quality = self.get("quality")?;
        let alternate = self.local(list, &policy.alternate_quality)?;
        let Value::Number(alternate) = alternate else {
            return Err(super::AssemblyError::source(
                "attempt to compare item value with number",
            ));
        };
        if alternate > policy.quality_threshold {
            quality = Value::Number(policy.suppressed_quality);
        }
        for rule in &policy.defences {
            let raw = self
                .arena
                .get_field(local_table(&base)?, &rule.base.field)?;
            let raw = if raw.truthy() {
                raw
            } else {
                Value::Number(rule.base.default)
            };
            // The original base output is written before the arithmetic/quality checks.
            self.write_local(&output, &rule.base_output, raw)?;
            let total = sum_roles(&values, &rule.base_roles)?
                * (policy.fraction_base
                    + sum_roles(&values, &rule.increased_roles)? / policy.percent_divisor)
                * (policy.fraction_base + number(&quality)? / policy.percent_divisor);
            self.write_local(
                &output,
                &rule.output,
                finite(super::query::round(total, policy.round_places))?,
            )?;
        }
        for rule in &policy.per_level {
            let value = number(&values[role_index(rule.base_role)])?
                * (policy.fraction_base
                    + sum_roles(&values, &rule.increased_roles)? / policy.percent_divisor)
                * (policy.fraction_base + number(&quality)? / policy.percent_divisor);
            self.write_local(&output, &rule.output, finite(value)?)?;
        }
        let block = self
            .arena
            .get_field(local_table(&base)?, &policy.block.base_field)?;
        if block.truthy() {
            let added = self.local(list, &policy.block.base)?;
            let total = number(&block)? + number(&added)?;
            let increased = number(&self.local(list, &policy.block.increased)?)?;
            let value =
                (total * (policy.fraction_base + increased / policy.percent_divisor)).floor();
            self.write_local(&output, &policy.block.output, finite(value)?)?;
        }
        let penalty = self
            .arena
            .get_field(local_table(&base)?, &policy.movement.base_field)?;
        if penalty.truthy() {
            let value = finite(number(&penalty)? * policy.movement.multiplier)?;
            let source = self.get("modSource")?;
            // The tag and complete record precede AddMod publication.
            let tag = self.arena.new_table()?;
            let kind = self.arena.text(&policy.movement.tag_type)?;
            self.arena.set_field(tag, "type", kind)?;
            let condition = self.arena.text(&policy.movement.condition)?;
            self.arena.set_field(tag, "var", condition)?;
            self.arena
                .set_field(tag, "neg", Value::Boolean(policy.movement.negated))?;
            let record = self.arena.new_table()?;
            let name = self.arena.text(&policy.movement.modifier_name)?;
            self.arena.set_field(record, "name", name)?;
            let kind = self.arena.text(&policy.movement.mod_type)?;
            self.arena.set_field(record, "type", kind)?;
            self.arena.set_field(record, "value", value)?;
            self.arena.set_field(record, "source", source)?;
            self.arena.set_field(record, "flags", Value::Number(0.0))?;
            self.arena
                .set_field(record, "keywordFlags", Value::Number(0.0))?;
            self.arena.set_index(record, 1, Value::Table(tag))?;
            self.arena.append(list, Value::Table(record))?;
        }
        self.local_overrides(list, &output, &policy.overrides)
    }

    pub(super) fn local_flask(&mut self, list: TableId, base_list: TableId) -> Result<()> {
        self.stage = "local flask";
        let policy = &self.definitions.policy().flask;
        let base = self.base_field(&policy.base_field)?;
        let output = self.get(&policy.output_field)?;
        let duration_inc = self.local(list, &policy.duration.increased)?;
        let duration_more = self.local(list, &policy.duration.more)?;
        let mut recovery = false;
        for channel in &policy.recovery.channels {
            if self
                .arena
                .get_field(local_table(&base)?, &channel.base_field)?
                .truthy()
            {
                recovery = true;
                break;
            }
        }
        if recovery {
            let instant = self.local(list, &policy.recovery.instant.query)?;
            self.write_local(&output, &policy.recovery.instant.output, instant)?;
            let recovery = policy.fraction_base
                + number(&self.local(list, &policy.recovery.increased)?)? / policy.percent_divisor;
            let rate = policy.fraction_base
                + number(&self.local(list, &policy.recovery.rate)?)? / policy.percent_divisor;
            let duration = number(
                &self
                    .arena
                    .get_field(local_table(&base)?, &policy.duration.base_field)?,
            )? * (policy.fraction_base
                + number(&duration_inc)? / policy.percent_divisor)
                / rate
                * number(&duration_more)?;
            self.write_local(
                &output,
                &policy.duration.output,
                finite(super::query::round(duration, policy.duration.round_places))?,
            )?;
            for channel in &policy.recovery.channels {
                let raw = self
                    .arena
                    .get_field(local_table(&base)?, &channel.base_field)?;
                if !raw.truthy() {
                    continue;
                }
                let quality = self.get("quality")?;
                let value = number(&raw)?
                    * (policy.fraction_base + number(&quality)? / policy.percent_divisor)
                    * recovery;
                self.write_local(&output, &channel.base_output, finite(value)?)?;
                let value = number(
                    &self
                        .arena
                        .get_field(local_table(&output)?, &channel.base_output)?,
                )?;
                let instant = number(
                    &self
                        .arena
                        .get_field(local_table(&output)?, &policy.recovery.instant.output)?,
                )?;
                self.write_local(
                    &output,
                    &channel.instant_output,
                    finite(value * instant / policy.percent_divisor)?,
                )?;
                let value = number(
                    &self
                        .arena
                        .get_field(local_table(&output)?, &channel.base_output)?,
                )?;
                let instant = number(
                    &self
                        .arena
                        .get_field(local_table(&output)?, &policy.recovery.instant.output)?,
                )?;
                self.write_local(
                    &output,
                    &channel.gradual_output,
                    finite(value * (policy.fraction_base - instant / policy.percent_divisor))?,
                )?;
                let instant = number(
                    &self
                        .arena
                        .get_field(local_table(&output)?, &channel.instant_output)?,
                )?;
                let gradual = number(
                    &self
                        .arena
                        .get_field(local_table(&output)?, &channel.gradual_output)?,
                )?;
                self.write_local(&output, &channel.total_output, finite(instant + gradual)?)?;
                if let Some(additional) = &channel.additional {
                    let value = self.local(list, &additional.query)?;
                    self.write_local(&output, &additional.output, value)?;
                }
                let selected = match channel.effect_not_removed.list {
                    ItemAssemblyQueryList::Slot => list,
                    ItemAssemblyQueryList::Base => base_list,
                };
                let value = self.local(selected, &channel.effect_not_removed.query)?;
                self.write_local(&output, &channel.effect_not_removed.output, value)?;
            }
        }
        self.local_charges(
            list,
            &base,
            &output,
            &policy.charges,
            policy.fraction_base,
            policy.percent_divisor,
        )?;
        self.local_overrides(list, &output, &policy.overrides)
    }

    pub(super) fn local_charm(&mut self, list: TableId) -> Result<()> {
        self.stage = "local charm";
        let policy = &self.definitions.policy().charm;
        let base = self.base_field(&policy.base_field)?;
        let output = self.get(&policy.output_field)?;
        let increased = self.local(list, &policy.duration.increased)?;
        let more = self.local(list, &policy.duration.more)?;
        let duration = number(
            &self
                .arena
                .get_field(local_table(&base)?, &policy.duration.base_field)?,
        )? * (policy.fraction_base + number(&increased)? / policy.percent_divisor)
            * (policy.fraction_base + number(&self.get("quality")?)? / policy.percent_divisor)
            * number(&more)?;
        self.write_local(
            &output,
            &policy.duration.output,
            finite(super::query::round(duration, policy.duration.round_places))?,
        )?;
        self.local_charges(
            list,
            &base,
            &output,
            &policy.charges,
            policy.fraction_base,
            policy.percent_divisor,
        )?;
        self.local_overrides(list, &output, &policy.overrides)
    }

    fn local_charges(
        &mut self,
        list: TableId,
        base: &Value,
        output: &Value,
        policy: &ItemAssemblyChargePolicy,
        fraction_base: f64,
        divisor: f64,
    ) -> Result<()> {
        let raw = self
            .arena
            .get_field(local_table(base)?, &policy.maximum_base_field)?;
        let added = self.local(list, &policy.maximum_base)?;
        let maximum = number(&raw)? + number(&added)?;
        let increased = number(&self.local(list, &policy.maximum_increased)?)?;
        self.write_local(
            output,
            &policy.maximum_output,
            finite(maximum * (fraction_base + increased / divisor))?,
        )?;
        let raw = self
            .arena
            .get_field(local_table(base)?, &policy.used_base_field)?;
        let increased = number(&self.local(list, &policy.used_increased)?)?;
        self.write_local(
            output,
            &policy.used_output,
            finite((number(&raw)? * (fraction_base + increased / divisor)).floor())?,
        )?;
        let value = self.local(list, &policy.gain_base.query)?;
        self.write_local(output, &policy.gain_base.output, value)?;
        let value = self.local(list, &policy.gain_increased.query)?;
        self.write_local(output, &policy.gain_increased.output, value)?;
        let value =
            fraction_base + number(&self.local(list, &policy.gain_multiplier.query)?)? / divisor;
        self.write_local(output, &policy.gain_multiplier.output, finite(value)?)?;
        let first = self.local(list, &policy.effect_queries[0])?;
        let second = self.local(list, &policy.effect_queries[1])?;
        self.write_local(
            output,
            &policy.effect_output,
            finite(number(&first)? + number(&second)?)?,
        )?;
        Ok(())
    }

    fn write_local(&mut self, output: &Value, key: &str, value: Value) -> Result<()> {
        self.arena.set_field(local_table(output)?, key, value)
    }

    fn local_overrides(
        &mut self,
        list: TableId,
        output: &Value,
        policy: &ItemAssemblyOverridePolicy,
    ) -> Result<()> {
        let values = table(self.nil_query(list, &policy.query_name, false)?)?;
        for index in 1..=self.arena.dense_len(values)? {
            let row = table(self.arena.get_index(values, index as i64)?)?;
            let key = self.arena.get_field(row, &policy.key_field)?;
            let value = self.arena.get_field(row, &policy.value_field)?;
            self.set_raw_key(local_table(output)?, &key, value)?;
        }
        Ok(())
    }
}

fn sum_roles(values: &[Value; 18], roles: &[Role]) -> Result<f64> {
    // Bare calcLocal values remain uncoerced until this source arithmetic use.
    let mut roles = roles.iter();
    let mut total = number(&values[role_index(*roles.next().expect("validated role operands"))])?;
    for role in roles {
        total += number(&values[role_index(*role)])?;
    }
    Ok(total)
}
fn role_index(role: Role) -> usize {
    match role {
        Role::ArmourBase => 0,
        Role::ArmourEvasionBase => 1,
        Role::EvasionBase => 2,
        Role::EvasionEnergyShieldBase => 3,
        Role::EnergyShieldBase => 4,
        Role::ArmourEnergyShieldBase => 5,
        Role::WardBase => 6,
        Role::EvasionPerLevel => 7,
        Role::EnergyShieldPerLevel => 8,
        Role::WardPerLevel => 9,
        Role::ArmourIncreased => 10,
        Role::ArmourEvasionIncreased => 11,
        Role::EvasionIncreased => 12,
        Role::EvasionEnergyShieldIncreased => 13,
        Role::EnergyShieldIncreased => 14,
        Role::WardIncreased => 15,
        Role::ArmourEnergyShieldIncreased => 16,
        Role::DefencesIncreased => 17,
    }
}

fn local_table(value: &Value) -> Result<TableId> {
    value
        .as_table()
        .ok_or_else(|| super::AssemblyError::source("attempt to index a non-table item value"))
}
