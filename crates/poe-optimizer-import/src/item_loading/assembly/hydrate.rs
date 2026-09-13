use super::{AssemblyError, Context, ItemLoadProvider, Result, TableId, Value};
use crate::item_loading::{ItemNumber, ItemScalar, ItemState, LoadedModLine};

pub(super) const GROUPS: [&str; 6] = [
    "buffModLines",
    "enchantModLines",
    "runeModLines",
    "classRequirementModLines",
    "implicitModLines",
    "explicitModLines",
];
pub(super) fn rows<'a>(state: &'a ItemState, group: &str) -> &'a [LoadedModLine] {
    match group {
        "buffModLines" => &state.buff_mod_lines,
        "enchantModLines" => &state.enchant_mod_lines,
        "runeModLines" => &state.rune_mod_lines,
        "classRequirementModLines" => &state.class_requirement_mod_lines,
        "implicitModLines" => &state.implicit_mod_lines,
        "explicitModLines" => &state.explicit_mod_lines,
        _ => unreachable!("closed group"),
    }
}
pub(super) fn numeric(n: ItemNumber) -> Result<Value> {
    match n.value() {
        None => Ok(Value::Nil),
        Some(n) if n.is_finite() => Ok(Value::Number(n)),
        _ => Err(AssemblyError::unsupported("nonfinite item loading input")),
    }
}
impl<P: ItemLoadProvider + ?Sized> Context<'_, '_, P> {
    pub(super) fn hydrate(&mut self, previous: bool) -> Result<()> {
        let state = &self.request.state;
        if previous {
            // Scalar loading updates explicitly represent removals. Structural
            // graph fields retain identity until the corresponding source write.
            let old = self.arena.table(self.root)?;
            let bytes = old
                .fields
                .keys()
                .map(|k| k.len() + std::mem::size_of::<String>())
                .sum();
            self.arena.reserve_bytes(bytes)?;
            let keys: Vec<_> = self
                .arena
                .table(self.root)?
                .fields
                .iter()
                .filter(|(_, v)| !matches!(v, Value::Table(_)))
                .map(|(k, _)| k.clone())
                .collect();
            for key in keys {
                if !state.retained_fields.contains_key(&key)
                    && !matches!(key.as_str(), "name" | "type")
                {
                    self.set(&key, Value::Nil)?;
                }
            }
        }
        for (key, scalar) in &state.retained_fields {
            let value = match scalar {
                ItemScalar::Boolean(b) => Value::Boolean(*b),
                ItemScalar::Number(n) => numeric(*n)?,
                ItemScalar::Text(s) => self.arena.text(s)?,
            };
            self.set(key, value)?;
        }
        let name = self.arena.text(&state.name)?;
        self.set("name", name)?;
        for (key, value) in [
            ("namePrefix", &state.name_prefix),
            ("nameSuffix", &state.name_suffix),
            ("rarity", &state.rarity),
        ] {
            let value = self.arena.text(value)?;
            self.set(key, value)?;
        }
        self.set(
            "itemSocketCount",
            Value::Number(state.item_socket_count as f64),
        )?;
        self.set(
            "jewelSocketCount",
            Value::Number(state.jewel_socket_count as f64),
        )?;
        let item_type = match &state.item_type {
            Some(t) => self.arena.text(t)?,
            None => Value::Nil,
        };
        self.set("type", item_type)?;
        if !previous || self.request.reparsed {
            let base = if state.base_present {
                let name = state.base_name.as_deref().ok_or_else(|| {
                    AssemblyError::unsupported("present item base has no definition identity")
                })?;
                let definition = self.definitions.items().base(name).ok_or_else(|| {
                    AssemblyError::unsupported("item base definition is unavailable")
                })?;
                Value::Table(self.arena.import_metadata(&definition.fields)?)
            } else {
                Value::Nil
            };
            self.set("base", base)?;
            let requirements = self.fresh_field("requirements")?;
            for (key, value) in &state.requirements {
                self.arena.set_field(requirements, key, numeric(*value)?)?;
            }
            let sockets = self.fresh_field("sockets")?;
            for (i, group) in state.sockets.iter().enumerate() {
                let socket = self.arena.new_table()?;
                self.arena
                    .set_field(socket, "group", Value::Number(f64::from(*group)))?;
                self.arena
                    .set_index(sockets, i as i64 + 1, Value::Table(socket))?;
            }
            for group in GROUPS {
                let list = self.fresh_field(group)?;
                for (i, row) in rows(state, group).iter().enumerate() {
                    let line = self.arena.new_table()?;
                    self.hydrate_line(line, row, group, true)?;
                    self.arena
                        .set_index(list, i as i64 + 1, Value::Table(line))?;
                }
            }
        } else {
            for group in GROUPS {
                let list = self.root_table(group)?;
                if self.arena.dense_len(list)? != rows(state, group).len() {
                    return Err(AssemblyError::unsupported(
                        "item line topology changed without ParseRaw generation",
                    ));
                }
                for (i, row) in rows(state, group).iter().enumerate() {
                    let line = super::table(self.arena.get_index(list, i as i64 + 1)?)?;
                    self.hydrate_line(line, row, group, false)?;
                }
            }
        }
        if !previous {
            if let Some(fields) = &state.armour_data {
                let armour = self.fresh_field("armourData")?;
                for (key, value) in fields {
                    self.arena.set_field(armour, key, numeric(*value)?)?;
                }
            }
            let types = self.fresh_field("socketedSoulCoreTypes")?;
            for name in &state.socketed_soul_core_types {
                self.arena.set_field(types, name, Value::Boolean(true))?;
            }
        }
        Ok(())
    }
    fn hydrate_line(
        &mut self,
        line: TableId,
        row: &LoadedModLine,
        group: &str,
        fresh: bool,
    ) -> Result<()> {
        if fresh {
            let text = self.arena.text(&row.line)?;
            self.arena.set_field(line, "line", text)?;
            let mods = self.arena.new_table()?;
            for (i, record) in row.modifiers.iter().enumerate() {
                let id = self.arena.import_metadata(record)?;
                self.arena.set_index(mods, i as i64 + 1, Value::Table(id))?;
            }
            self.arena.set_field(line, "modList", Value::Table(mods))?;
            let extra = match &row.extra {
                Some(s) => self.arena.text(s)?,
                None => Value::Nil,
            };
            self.arena.set_field(line, "extra", extra)?;
            for flag in &row.flags {
                self.arena.set_field(line, flag, Value::Boolean(true))?;
            }
            // Existing loading wire carries tags as a vector. Authored ParseRaw
            // rows create an empty tag table; generated rune/buff rows omit it.
            if !row.mod_tags.is_empty() || (group != "buffModLines" && row.rune_origins.is_empty())
            {
                let tags = self.arena.new_table()?;
                for (i, tag) in row.mod_tags.iter().enumerate() {
                    let value = self.arena.text(tag)?;
                    self.arena.set_index(tags, i as i64 + 1, value)?;
                }
                self.arena.set_field(line, "modTags", Value::Table(tags))?;
            }
            for (key, selected) in [
                ("variantList", &row.selection.variants),
                ("versionList", &row.selection.versions),
                ("variantGroupList", &row.selection.groups),
            ] {
                if let Some(selected) = selected {
                    let set = self.arena.new_table()?;
                    for n in selected {
                        self.arena
                            .set_index(set, i64::from(*n), Value::Boolean(true))?;
                    }
                    self.arena.set_field(line, key, Value::Table(set))?;
                }
            }
            for (key, v) in [
                ("augmentType", &row.augment_type),
                (
                    "socketedAugmentTypeOverride",
                    &row.socketed_augment_type_override,
                ),
                ("socketedSoulCoreType", &row.socketed_soul_core_type),
            ] {
                if let Some(v) = v {
                    let value = self.arena.text(v)?;
                    self.arena.set_field(line, key, value)?;
                }
            }
            for (key, n) in [
                ("order", row.order),
                ("runeCount", row.rune_count),
                ("displayValueScalar", row.display_value_scalar),
            ] {
                if let Some(n) = n {
                    self.arena.set_field(line, key, numeric(n)?)?;
                }
            }
            if let Some(v) = row.socketed_rune_effect_already_applied {
                self.arena.set_field(
                    line,
                    "socketedRuneEffectAlreadyApplied",
                    Value::Boolean(v),
                )?;
            }
        }
        for (key, n) in [
            ("range", row.range),
            ("valueScalar", row.value_scalar),
            ("corruptedRange", row.corrupted_range),
        ] {
            self.arena.set_field(line, key, numeric(n)?)?;
        }
        Ok(())
    }
}
