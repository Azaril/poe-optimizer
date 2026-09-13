use super::{
    AssemblyError, Context, ItemLoadProvider, Result, TableId, Value, finite, number, table,
};
use poe_optimizer_data::item_assembly::{
    ItemAssemblySlotPredicate, ItemAssemblyTagReplacementRole,
};

impl<P: ItemLoadProvider + ?Sized> Context<'_, '_, P> {
    pub(super) fn slots(&mut self, base_list: TableId) -> Result<()> {
        let policy = &self.definitions.policy().slots;
        let mut multi = false;
        for predicate in &policy.multislot_any {
            if self.slot_predicate(predicate)? {
                multi = true;
                break;
            }
        }
        if multi {
            let slots = self.fresh_field("slotModList")?;
            let kind = self.get("type")?;
            let count = policy
                .slot_count_overrides
                .iter()
                .find(|rule| Some(rule.item_type.as_str()) == kind.as_str())
                .map_or(policy.default_multislot_count, |rule| rule.count);
            for i in 1..=count {
                let list = self.slot(base_list, Some(i))?;
                self.arena
                    .set_index(slots, i64::from(i), Value::Table(list))?;
            }
        } else {
            let list = self.slot(base_list, None)?;
            self.set("modList", Value::Table(list))?;
        }
        Ok(())
    }
    fn slot_predicate(&mut self, predicate: &ItemAssemblySlotPredicate) -> Result<bool> {
        Ok(match predicate {
            ItemAssemblySlotPredicate::BaseWeaponTruthy => self.base_field("weapon")?.truthy(),
            ItemAssemblySlotPredicate::BaseTypeEquals { value } => {
                self.base_field("type")?.as_str() == Some(value)
            }
            ItemAssemblySlotPredicate::ItemTypeEquals { value } => {
                self.get("type")?.as_str() == Some(value)
            }
            ItemAssemblySlotPredicate::BaseSubtypeEquals { value } => {
                self.base_field("subType")?.as_str() == Some(value)
            }
        })
    }
    fn slot(&mut self, base_list: TableId, number_slot: Option<u16>) -> Result<TableId> {
        let policy = self.definitions.policy();
        let mut slot_name = None;
        'rules: for rule in &policy.slots.primary {
            for predicate in &rule.any {
                if self.slot_predicate(predicate)? {
                    self.arena.reserve_bytes(rule.slot_name.len())?;
                    slot_name = Some(rule.slot_name.clone());
                    break 'rules;
                }
            }
        }
        let mut slot_name = match slot_name {
            Some(s) => s,
            None => match self.get("type")? {
                Value::Text(s) => s,
                _ => return Err(AssemblyError::source("item primary slot is not text")),
            },
        };
        if number_slot == Some(policy.slots.renamed_slot) {
            let rewrite = &policy.slots.slot_name_rewrite;
            slot_name = self.patterns.replace(
                &mut self.arena,
                &slot_name,
                &rewrite.pattern,
                &rewrite.replacement,
            )?;
        }
        let list = self.mod_list()?;
        for i in 1..=self.arena.dense_len(base_list)? {
            let original = self.arena.get_index(base_list, i as i64)?;
            let record = table(self.arena.deep_copy(&original)?)?;
            let mut add = true;
            for j in 1..=self.arena.dense_len(record)? {
                let tag = table(self.arena.get_index(record, j as i64)?)?;
                let kind = self.arena.get_field(tag, "type")?;
                if kind.as_str() == Some(&policy.slots.slot_number_tag_type)
                    || kind.as_str() == Some(&policy.local.in_slot_tag_type)
                {
                    let expected = number_slot.map_or(Value::Nil, |n| Value::Number(f64::from(n)));
                    if self.arena.get_field(tag, "num")? != expected {
                        add = false;
                        break;
                    }
                }
                self.rewrite_tag(tag, &slot_name, number_slot)?;
            }
            if add {
                let value = self.arena.text(&slot_name)?;
                self.arena.set_field(record, "sourceSlot", value)?;
                self.arena.append(list, Value::Table(record))?;
            }
        }
        let crafted = self.local(list, &policy.slots.crafted_quality)?;
        let crafted = if crafted.truthy() {
            crafted
        } else {
            Value::Number(0.0)
        };
        let previous = self.get("craftedQuality")?;
        if crafted != previous {
            if previous.truthy() {
                let quality = self.get("quality")?;
                let quality = if quality.truthy() {
                    number(&quality)?
                } else {
                    0.0
                };
                self.set(
                    "quality",
                    finite(quality - number(&previous)? + number(&crafted)?)?,
                )?;
            }
            self.set("craftedQuality", crafted)?;
        }
        let quality = self.get("quality")?;
        if quality.truthy() {
            self.arena
                .reserve_bytes(policy.slots.quality_name_prefix.len() + slot_name.len())?;
            let name = format!("{}{}", policy.slots.quality_name_prefix, slot_name);
            self.add_new(
                list,
                &name,
                &policy.slots.quality_mod_type,
                quality,
                Some(&policy.slots.quality_source),
            )?;
        }
        if self.get("spiritValue")?.truthy() {
            let base = number(&self.base_field("spirit")?)?;
            let added = number(&self.local(list, &policy.slots.spirit_base)?)?;
            let increased = number(&self.local(list, &policy.slots.spirit_increased)?)?;
            self.set(
                "spiritValue",
                finite(super::query::round(
                    (base + added) * (1.0 + increased / policy.slots.spirit_percent_divisor),
                    0,
                ))?,
            )?;
        }
        if self.get("charmLimit")?.truthy() {
            let base = number(&self.base_field("charmLimit")?)?;
            let added = number(&self.local(list, &policy.slots.charm_limit)?)?;
            self.set("charmLimit", finite(base + added)?)?;
        }
        // These producers require separately injected local-data algorithms.
        // Do not discard the collection, requirements or slot-prefix mutations.
        for kind in ["weapon", "armour", "flask", "charm"] {
            if self.base_field(kind)?.truthy() {
                return Err(AssemblyError::unsupported(format!(
                    "item local {kind} data assembly is unavailable"
                )));
            }
        }
        if self.get("type")?.as_str() == Some(&policy.jewel_item_type) {
            return Err(AssemblyError::unsupported(
                "item local jewel data assembly is unavailable",
            ));
        }
        let output = self.arena.new_table()?;
        for i in 1..=self.arena.dense_len(list)? {
            let value = self.arena.get_index(list, i as i64)?;
            self.arena.set_index(output, i as i64, value)?;
        }
        Ok(output)
    }
    fn rewrite_tag(&mut self, tag: TableId, slot: &str, number_slot: Option<u16>) -> Result<()> {
        let fields = self.arena.table(tag)?;
        let bytes = fields
            .fields
            .keys()
            .map(|k| k.len() + std::mem::size_of::<String>())
            .sum::<usize>()
            + fields.indexed.len() * std::mem::size_of::<i64>();
        self.arena.reserve_bytes(bytes)?;
        let keys: Vec<_> = self.arena.table(tag)?.fields.keys().cloned().collect();
        let indices: Vec<_> = self.arena.table(tag)?.indexed.keys().copied().collect();
        for key in keys {
            if let Value::Text(value) = self.arena.get_field(tag, &key)? {
                let value = self.rewrite_text(value, slot, number_slot)?;
                self.arena.set_field(tag, &key, Value::Text(value))?;
            }
        }
        for index in indices {
            if let Value::Text(value) = self.arena.get_index(tag, index)? {
                let value = self.rewrite_text(value, slot, number_slot)?;
                self.arena.set_index(tag, index, Value::Text(value))?;
            }
        }
        Ok(())
    }
    fn rewrite_text(
        &mut self,
        mut value: String,
        slot: &str,
        number_slot: Option<u16>,
    ) -> Result<String> {
        let policy = &self.definitions.policy().slots;
        let primary = number_slot == Some(policy.primary_hand_slot);
        for rewrite in &policy.tag_replacements {
            let replacement = match rewrite.role {
                ItemAssemblyTagReplacementRole::SlotName => slot,
                ItemAssemblyTagReplacementRole::Hand => {
                    if primary {
                        &policy.primary_hand_name
                    } else {
                        &policy.other_hand_name
                    }
                }
                ItemAssemblyTagReplacementRole::OtherSlotNumber => {
                    if primary {
                        &policy.other_number_for_primary
                    } else {
                        &policy.other_number_for_other
                    }
                }
            };
            value =
                self.patterns
                    .replace(&mut self.arena, &value, &rewrite.pattern, replacement)?;
        }
        Ok(value)
    }
}
