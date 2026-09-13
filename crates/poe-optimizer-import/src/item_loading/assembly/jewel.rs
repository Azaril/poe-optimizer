use super::{AssemblyError, Context, ItemLoadProvider, Result, TableId, Value, number, table};
use poe_optimizer_data::item_assembly::{ItemAssemblyJewelCluster, ItemAssemblyJewelListField};

impl<P: ItemLoadProvider + ?Sized> Context<'_, '_, P> {
    pub(super) fn local_jewel(&mut self, list: TableId) -> Result<()> {
        let policy = &self.definitions.policy().jewel;
        let spectrum = &policy.grand_spectrum;
        let name = self.get(&spectrum.name_item_field)?;
        let name = name
            .as_str()
            .ok_or_else(|| AssemblyError::source("item jewel name is not text"))?;
        if self
            .patterns
            .find(&mut self.arena, name, &spectrum.name_pattern)?
        {
            self.add_new(
                list,
                &spectrum.modifier_name,
                &spectrum.modifier_type,
                Value::Number(spectrum.modifier_value),
                Some(name),
            )?;
            let last = self.arena.dense_len(list)?;
            let modifier = self.arena.get_index(list, last as i64)?;
            let nested = self.arena.new_table()?;
            // The wrapper and the list refer to the same newly created modifier.
            self.arena
                .set_field(nested, &spectrum.nested_mod_field, modifier)?;
            self.add_new(
                list,
                &spectrum.minion_name,
                &spectrum.minion_type,
                Value::Table(nested),
                Some(name),
            )?;
        }
        // Bind now, but do not index until the corresponding source operation.
        let output = self.get(&policy.output_field)?;
        let functions = table(self.nil_query(list, &policy.functions.query_name, false)?)?;
        for i in 1..=self.arena.dense_len(functions)? {
            let function = self.arena.get_index(functions, i as i64)?;
            let current = self.jewel_field(&output, &policy.functions.output_field)?;
            let current = if current.truthy() {
                current
            } else {
                Value::Table(self.arena.new_table()?)
            };
            self.arena.set_field(
                jewel_table(&output)?,
                &policy.functions.output_field,
                current,
            )?;
            let target = table(self.jewel_field(&output, &policy.functions.output_field)?)?;
            // Finite values are retained verbatim; callable descriptors are refused
            // at metadata ingress, never silently dropped or treated as executable.
            self.arena.append(target, function)?;
        }
        self.local_overrides(list, &output, &policy.overrides)?;
        let classes =
            table(self.nil_query(list, &policy.alternate_class_start.query_name, false)?)?;
        for i in 1..=self.arena.dense_len(classes)? {
            let class = self.arena.get_index(classes, i as i64)?;
            self.arena.set_field(
                jewel_table(&output)?,
                &policy.alternate_class_start.output_field,
                class,
            )?;
        }
        // List returns a truthy table even when empty. Keep the first query and
        // the fresh publication before the independent query used for stores.
        if self
            .nil_query(list, &policy.from_nothing.guard_query_name, false)?
            .truthy()
        {
            let entries = self.arena.new_table()?;
            self.arena.set_field(
                jewel_table(&output)?,
                &policy.from_nothing.output_field,
                Value::Table(entries),
            )?;
            self.local_overrides(list, &Value::Table(entries), &policy.from_nothing.entries)?;
        }
        let cluster = self.get(&policy.cluster.item_field)?;
        if cluster.truthy() {
            self.jewel_cluster(list, &output, &cluster, &policy.cluster)?;
        }
        Ok(())
    }

    fn jewel_field(&mut self, value: &Value, key: &str) -> Result<Value> {
        self.arena.get_field(jewel_table(value)?, key)
    }

    fn jewel_fresh_list(
        &mut self,
        list: TableId,
        output: &Value,
        policy: &ItemAssemblyJewelListField,
    ) -> Result<()> {
        let target = self.arena.new_table()?;
        self.arena.set_field(
            jewel_table(output)?,
            &policy.output_field,
            Value::Table(target),
        )?;
        let values = table(self.nil_query(list, &policy.query_name, false)?)?;
        for i in 1..=self.arena.dense_len(values)? {
            let value = self.arena.get_index(values, i as i64)?;
            let target = table(self.jewel_field(output, &policy.output_field)?)?;
            self.arena.append(target, value)?;
        }
        Ok(())
    }

    fn jewel_cluster(
        &mut self,
        list: TableId,
        output: &Value,
        cluster: &Value,
        policy: &ItemAssemblyJewelCluster,
    ) -> Result<()> {
        self.jewel_fresh_list(list, output, &policy.notables)?;
        self.jewel_fresh_list(list, output, &policy.added_mods)?;
        let skill = self.jewel_field(output, &policy.skill_field)?;
        self.arena
            .work(policy.correction.matching_skill.len() as u64 + 1)?;
        if skill.as_str() == Some(&policy.correction.matching_skill) {
            let count = self.jewel_field(output, &policy.node_count_field)?;
            if count.truthy() {
                // Lua comparison does not apply arithmetic's numeric-string coercion.
                let n = match count {
                    Value::Number(n) if n.is_finite() => n,
                    Value::Number(_) => {
                        return Err(AssemblyError::unsupported(
                            "nonfinite jewel node comparison",
                        ));
                    }
                    _ => {
                        return Err(AssemblyError::source(
                            "attempt to compare jewel node count with number",
                        ));
                    }
                };
                if n < policy.correction.node_count_below {
                    let replacement = self.arena.text(&policy.correction.replacement_skill)?;
                    self.arena
                        .set_field(jewel_table(output)?, &policy.skill_field, replacement)?;
                }
            }
        }
        let count = self.jewel_field(output, &policy.node_count_field)?;
        if count.truthy() {
            // Read both arguments before math.max coercion, then read maxNodes
            // only after the inner call succeeds. Equal doubles select the second
            // operand on the pinned LuaJIT MINSD/MAXSD path, including signed zero.
            let minimum = self.jewel_field(cluster, &policy.min_nodes_field)?;
            let count = number(&count)?;
            let minimum = number(&minimum)?;
            let lower = if count > minimum { count } else { minimum };
            let maximum = self.jewel_field(cluster, &policy.max_nodes_field)?;
            let maximum = number(&maximum)?;
            let bounded = if lower < maximum { lower } else { maximum };
            self.arena.set_field(
                jewel_table(output)?,
                &policy.node_count_field,
                Value::Number(bounded),
            )?;
        }
        let skill = self.jewel_field(output, &policy.skill_field)?;
        if skill.truthy() {
            let skills = self.jewel_field(cluster, &policy.skills_field)?;
            let skills = jewel_table(&skills)?;
            let found = match &skill {
                Value::Text(key) => self.arena.get_field(skills, key)?,
                Value::Number(key)
                    if key.fract() == 0.0 && key.abs() <= 9_007_199_254_740_991.0 =>
                {
                    self.arena.get_index(skills, *key as i64)?
                }
                _ => {
                    // The finite arena contains only text/integer keys. Other
                    // legal Lua lookup keys therefore have no matching entry.
                    self.arena.work(1)?;
                    Value::Nil
                }
            };
            if !found.truthy() {
                self.arena
                    .set_field(jewel_table(output)?, &policy.skill_field, Value::Nil)?;
            }
        }
        let validity = &policy.validity;
        let mut valid = self.jewel_field(output, &validity.keystone_field)?;
        if !valid.truthy() {
            let skill = self.jewel_field(output, &policy.skill_field)?;
            let left = if skill.truthy() {
                skill
            } else {
                self.jewel_field(output, &validity.smalls_are_nothingness_field)?
            };
            valid = if left.truthy() {
                self.jewel_field(output, &policy.node_count_field)?
            } else {
                left
            };
            if !valid.truthy() {
                let sockets = self.jewel_field(output, &validity.socket_count_override_field)?;
                valid = if sockets.truthy() {
                    self.jewel_field(output, &validity.nothingness_count_field)?
                } else {
                    sockets
                };
            }
        }
        self.arena
            .set_field(jewel_table(output)?, &validity.output_field, valid)
    }
}

fn jewel_table(value: &Value) -> Result<TableId> {
    value
        .as_table()
        .ok_or_else(|| AssemblyError::source("attempt to index a non-table item value"))
}
