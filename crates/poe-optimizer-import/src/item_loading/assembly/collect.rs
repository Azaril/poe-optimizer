use super::hydrate::rows;
use super::{
    AssemblyError, Context, ItemLoadProvider, Result, TableId, Value, dependency, finite, number,
    table,
};
use crate::item_loading::{FormatRequest, LoadedModLine, MAX_ITEM_LOADING_CALLS, ParseRequest};
use poe_optimizer_data::item_assembly::{
    ItemAssemblyAttribute, ItemAssemblyRows, ItemAssemblyRuneEffect,
};

impl<P: ItemLoadProvider + ?Sized> Context<'_, '_, P> {
    pub(super) fn collect(&mut self) -> Result<()> {
        self.stage = "collection";
        if !self.get("base")?.truthy() {
            return Ok(());
        }
        let policy = self.definitions.policy();
        let base_list = self.mod_list()?;
        if self.base_field("weapon")?.truthy() {
            self.fresh_field("weaponData")?;
        } else if self.base_field(&policy.armour.base_field)?.truthy() {
            if !self.get(&policy.armour.output_field)?.truthy() {
                self.fresh_field(&policy.armour.output_field)?;
            }
        } else if self.base_field(&policy.flask.base_field)?.truthy() {
            self.fresh_field(&policy.flask.output_field)?;
            self.fresh_field("buffModList")?;
        } else if self.base_field(&policy.charm.base_field)?.truthy() {
            self.fresh_field(&policy.charm.output_field)?;
            self.fresh_field("buffModList")?;
        } else if self.get("type")?.as_str() == Some(&policy.jewel_item_type) {
            self.fresh_field("jewelData")?;
        }
        self.set("baseModList", Value::Table(base_list))?;
        self.fresh_field("rangeLineList")?;
        let id = self.get("id")?;
        let id = if !id.truthy() {
            Value::Number(policy.collection.missing_item_id)
        } else {
            id
        };
        let id = match id {
            Value::Number(n) => poe_optimizer_engine::item_tools::lua_number_text(n),
            Value::Text(s) => s,
            _ => return Err(AssemblyError::source("attempt to concatenate item id")),
        };
        let name = self.get("name")?;
        let name = name
            .as_str()
            .ok_or_else(|| AssemblyError::source("attempt to concatenate item name"))?;
        self.arena.reserve_bytes(
            policy.collection.source_prefix.len()
                + policy.collection.source_separator.len()
                + id.len()
                + name.len(),
        )?;
        let source = format!(
            "{}{}{}{}",
            policy.collection.source_prefix, id, policy.collection.source_separator, name
        );
        let source_value = self.arena.text(&source)?;
        self.set("modSource", source_value)?;
        let buff_lines = self.root_table("buffModLines")?;
        for (i, row) in self.request.state.buff_mod_lines.iter().enumerate() {
            if row.extra.is_none() && self.request.state.variants.matches(&row.selection) {
                let line = table(self.arena.get_index(buff_lines, i as i64 + 1)?)?;
                let mods = table(self.arena.get_field(line, "modList")?)?;
                for j in 1..=self.arena.dense_len(mods)? {
                    let record = table(self.arena.get_index(mods, j as i64)?)?;
                    let value = self.arena.text(&source)?;
                    self.arena.set_field(record, "source", value)?;
                    let target = self.root_table("buffModList")?;
                    self.arena.append(target, Value::Table(record))?;
                }
            }
        }
        self.set("socketedAugmentTypeOverride", Value::Nil)?;
        self.fresh_field("socketedSoulCoreTypes")?;
        for group in policy.collection.rows {
            let group = group_name(group);
            let list = self.root_table(group)?;
            for (i, row) in rows(&self.request.state, group).iter().enumerate() {
                let line = table(self.arena.get_index(list, i as i64 + 1)?)?;
                self.process_line(base_list, line, row, &source)?;
            }
        }
        self.stage = "rune effects";
        let value = self.local(base_list, &policy.rune.bonded_unlock)?;
        self.set("socketedIdolsUseBondedModifiers", value)?;
        for (role, field) in [
            (
                ItemAssemblyRuneEffect::SoulCore,
                "socketedSoulCoreEffectModifier",
            ),
            (ItemAssemblyRuneEffect::Rune, "socketedRuneEffectModifier"),
            (
                ItemAssemblyRuneEffect::Global,
                "socketedAugmentItemEffectModifier",
            ),
        ] {
            let value = self.local_query(base_list, self.definitions.rune_effect_query(role))?;
            self.set(
                field,
                finite(number(&value)? / self.definitions.rune_policy().effect_divisor)?,
            )?;
        }
        let rune_lines = self.root_table("runeModLines")?;
        if self.arena.get_index(rune_lines, 1)?.truthy() {
            for i in 1..=self.arena.dense_len(rune_lines)? {
                let line = table(self.arena.get_index(rune_lines, i as i64)?)?;
                let effect = self.rune_effect(line)?;
                let value = if effect != 0.0
                    && !self
                        .arena
                        .get_field(line, "socketedRuneEffectAlreadyApplied")?
                        .truthy()
                {
                    finite(1.0 + effect)?
                } else {
                    Value::Nil
                };
                self.arena.set_field(line, "displayValueScalar", value)?;
            }
        }
        for (i, row) in self.request.state.rune_mod_lines.iter().enumerate() {
            let line = table(self.arena.get_index(rune_lines, i as i64 + 1)?)?;
            let effect = self.rune_effect(line)?;
            let bonded = if self.arena.get_field(line, "bonded")?.truthy() {
                self.arena.get_field(line, "bondedModList")?
            } else {
                Value::Nil
            };
            let target = if bonded.truthy() {
                table(bonded)?
            } else {
                base_list
            };
            if effect != 0.0
                && self.request.state.variants.matches(&row.selection)
                && !self.arena.get_field(line, "extra")?.truthy()
                && !self
                    .arena
                    .get_field(line, "socketedRuneEffectAlreadyApplied")?
                    .truthy()
            {
                let mods = table(self.arena.get_field(line, "modList")?)?;
                for j in 1..=self.arena.dense_len(mods)? {
                    let record = table(self.arena.get_index(mods, j as i64)?)?;
                    self.scale_add(target, record, effect)?;
                }
            }
        }
        self.stage = "grants";
        let grants = self.fresh_field("grantedSkills")?;
        let skills = table(self.nil_query(base_list, &policy.grants.query_name, false)?)?;
        for i in 1..=self.arena.dense_len(skills)? {
            let skill = table(self.arena.get_index(skills, i as i64)?)?;
            if self.arena.get_field(skill, "name")?.as_str() != Some(&policy.grants.unknown_name) {
                let grant = self.arena.new_table()?;
                for key in ["skillId", "level", "noSupports"] {
                    let v = self.arena.get_field(skill, key)?;
                    self.arena.set_field(grant, key, v)?;
                }
                let no_reservation = self.base_field("grantedSkillsHaveNoReservation")?;
                self.arena.set_field(
                    grant,
                    "noReservation",
                    if no_reservation.truthy() {
                        no_reservation
                    } else {
                        Value::Nil
                    },
                )?;
                let v = self.arena.text(&source)?;
                self.arena.set_field(grant, "source", v)?;
                for key in ["triggered", "triggerChance"] {
                    let v = self.arena.get_field(skill, key)?;
                    self.arena.set_field(grant, key, v)?;
                }
                self.arena.append(grants, Value::Table(grant))?;
            }
        }
        if self
            .nil_query(base_list, &policy.jewel_restrictions.query_name, true)?
            .truthy()
        {
            let allowed = self.fresh_field("canSocketJewelBase")?;
            for entry in &policy.jewel_restrictions.entries {
                let value = self.local(base_list, &entry.query)?;
                self.arena.set_field(allowed, &entry.base_name, value)?;
            }
        }
        self.stage = "requirements";
        for rule in &policy.named_compatibility {
            if rule
                .names
                .iter()
                .any(|name| name == &self.request.state.name)
            {
                let value = self.arena.new_table()?;
                let key = self.arena.text(&rule.modifier.key)?;
                self.arena.set_field(value, "key", key)?;
                self.arena
                    .set_field(value, "value", Value::Number(rule.modifier.value))?;
                self.add_new(
                    base_list,
                    &rule.modifier.name,
                    &rule.modifier.mod_type,
                    Value::Table(value),
                    None,
                )?;
                if let Some(replacement) = &rule.requirement_override {
                    let requirements = self.root_table("requirements")?;
                    self.arena.set_field(
                        requirements,
                        attribute(replacement.attribute),
                        Value::Number(replacement.value),
                    )?;
                }
            }
        }
        self.requirements(base_list)?;
        if self.request.state.item_socket_count > 0 {
            let sockets = self.arena.new_table()?;
            for i in 1..=self.request.state.item_socket_count {
                let socket = self.arena.new_table()?;
                self.arena
                    .set_field(socket, "group", Value::Number(i as f64))?;
                self.arena.append(sockets, Value::Table(socket))?;
            }
            self.set("sockets", Value::Table(sockets))?;
        }
        let value = self.local(base_list, &policy.slots.socketed_jewel_effect)?;
        self.set(
            "socketedJewelEffectModifier",
            finite(1.0 + number(&value)? / policy.slots.socketed_jewel_percent_divisor)?,
        )?;
        self.stage = "slots";
        self.slots(base_list)?;
        self.set("activeBondedState", Value::Nil)?;
        self.stage = "complete";
        Ok(())
    }
    fn variant_count(&self, row: &LoadedModLine) -> Result<usize> {
        let variants = &self.request.state.variants;
        if variants.uses_versioned_or_grouped()
            || !variants.allow_duplicates
            || row.selection.variants.is_none()
        {
            return Ok(usize::from(variants.matches(&row.selection)));
        }
        let count = usize::from(
            self.definitions
                .policy()
                .collection
                .duplicate_alternate_count,
        );
        if count > variants.alternate.len() {
            return Err(AssemblyError::unsupported(
                "duplicate variant policy exceeds represented alternate slots",
            ));
        }
        let selected = row
            .selection
            .variants
            .as_ref()
            .expect("checked variant set");
        let contains = |value: Option<crate::item_loading::ItemNumber>| {
            value
                .and_then(|v| v.value())
                .is_some_and(|value| selected.iter().any(|n| f64::from(*n) == value))
        };
        Ok(usize::from(contains(variants.selected))
            + variants
                .alternate
                .iter()
                .enumerate()
                .take(count)
                .filter(|(i, value)| variants.has_alternate[*i] && contains(**value))
                .count())
    }
    fn process_line(
        &mut self,
        base_list: TableId,
        line: TableId,
        row: &LoadedModLine,
        source: &str,
    ) -> Result<()> {
        self.arena.set_field(line, "bondedModList", Value::Nil)?;
        if self.arena.get_field(line, "disabled")?.truthy() {
            return Ok(());
        }
        let count = self.variant_count(row)?;
        if count == 0 {
            return Ok(());
        }
        let policy = self.definitions.policy();
        if self.patterns.find(
            &mut self.arena,
            &row.line,
            &policy.collection.class_find_pattern,
        )? {
            let rewrite = &policy.collection.class_variant_rewrite;
            let text = self.patterns.replace(
                &mut self.arena,
                &row.line,
                &rewrite.pattern,
                &rewrite.replacement,
            )?;
            let value = self
                .patterns
                .capture(
                    &mut self.arena,
                    &text,
                    &policy.collection.class_capture_pattern,
                )?
                .map_or(Value::Nil, Value::Text);
            self.set("classRestriction", value)?;
        }
        let override_type = self.arena.get_field(line, "socketedAugmentTypeOverride")?;
        if override_type.truthy() {
            self.set("socketedAugmentTypeOverride", override_type)?;
        } else {
            let soul = self.arena.get_field(line, "socketedSoulCoreType")?;
            if soul.truthy() {
                let key = soul
                    .as_str()
                    .ok_or_else(|| AssemblyError::unsupported("soul core type key is not text"))?;
                let types = self.root_table("socketedSoulCoreTypes")?;
                self.arena.set_field(types, key, Value::Boolean(true))?;
            }
        }
        if self.arena.get_field(line, "extra")?.truthy() {
            return Ok(());
        }
        let target = if self.arena.get_field(line, "bonded")?.truthy() {
            let target = self.mod_list()?;
            self.arena
                .set_field(line, "bondedModList", Value::Table(target))?;
            target
        } else {
            base_list
        };
        if let Some(ranged) = self.ranged(row)? {
            self.arena
                .set_field(line, "modList", Value::Table(ranged))?;
            let ranges = self.root_table("rangeLineList")?;
            self.arena.append(ranges, Value::Table(line))?;
        }
        let mods = table(self.arena.get_field(line, "modList")?)?;
        for i in 1..=self.arena.dense_len(mods)? {
            let record = table(self.arena.get_index(mods, i as i64)?)?;
            for _ in 0..count {
                let value = self.arena.text(source)?;
                self.arena.set_field(record, "source", value)?;
                if let Some(value) = self.arena.get_field(record, "value")?.as_table() {
                    let nested = self.arena.get_field(value, "mod")?;
                    if nested.truthy() {
                        let nested = table(nested)?;
                        let source = self.arena.text(source)?;
                        self.arena.set_field(nested, "source", source)?;
                    }
                }
                self.arena.append(target, Value::Table(record))?;
            }
        }
        let tags = self.arena.get_field(line, "modTags")?;
        if tags.truthy() && self.arena.dense_len(table(tags)?)? > 0 {
            self.set("hasModTags", Value::Boolean(true))?;
        }
        Ok(())
    }
    fn ranged(&mut self, row: &LoadedModLine) -> Result<Option<TableId>> {
        if row.range.value().is_none()
            || !self.patterns.find(
                &mut self.arena,
                &row.line,
                &self.definitions.policy().range.range_find_pattern,
            )?
        {
            return Ok(None);
        }
        let rewrite = &self.definitions.policy().range.newline_rewrite;
        let text = self.patterns.replace(
            &mut self.arena,
            &row.line,
            &rewrite.pattern,
            &rewrite.replacement,
        )?;
        if self.sequence >= MAX_ITEM_LOADING_CALLS {
            return Err(AssemblyError::resource(
                "item assembly dependency call bound",
            ));
        }
        let request = FormatRequest {
            sequence: self.sequence,
            line_index: row.source_line,
            text,
            range: row.range,
            scalar: row.value_scalar,
            corrupted_range: row.corrupted_range,
        };
        self.sequence += 1;
        self.arena.work(1)?;
        let formatted = self.dependencies.format_with_trace(&request);
        self.sequence = self
            .sequence
            .checked_add(formatted.precision_parser_calls.len())
            .ok_or_else(|| AssemblyError::resource("item parser sequence"))?;
        let text = dependency(formatted.result)?;
        if self.sequence >= MAX_ITEM_LOADING_CALLS {
            return Err(AssemblyError::resource(
                "item assembly dependency call bound",
            ));
        }
        self.arena.reserve_bytes(text.len())?;
        let request = ParseRequest {
            sequence: self.sequence,
            line_index: row.source_line,
            origin: row.rune_origins.first().cloned(),
            text,
            combined: false,
        };
        self.sequence += 1;
        self.arena.work(1)?;
        let parsed = dependency(self.dependencies.parse_modifier(&request))?;
        if self.patterns.zero_line(&mut self.arena, &request.text)? {
            return self.arena.new_table().map(Some);
        }
        if parsed.extra.is_some() {
            return Ok(None);
        }
        let Some(records) = parsed.modifiers else {
            return Ok(None);
        };
        let out = self.arena.new_table()?;
        for record in &records {
            let record = self.arena.import_metadata(record)?;
            self.arena.append(out, Value::Table(record))?;
        }
        Ok(Some(out))
    }
    fn rune_effect(&mut self, line: TableId) -> Result<f64> {
        let global = self.get("socketedAugmentItemEffectModifier")?;
        let mut effect = if global.truthy() {
            number(&global)?
        } else {
            0.0
        };
        let kind = self.arena.get_field(line, "augmentType")?;
        let policy = self.definitions.rune_policy();
        if kind.as_str() == Some(&policy.extra_slot_augment_type) {
            effect += number(&self.get("socketedSoulCoreEffectModifier")?)?;
        } else if kind.as_str() == Some(&policy.rune_augment_type) {
            effect += number(&self.get("socketedRuneEffectModifier")?)?;
        }
        if effect.is_finite() {
            Ok(effect)
        } else {
            Err(AssemblyError::unsupported("nonfinite item rune effect"))
        }
    }
    fn requirements(&mut self, list: TableId) -> Result<()> {
        let policy = &self.definitions.policy().requirements;
        let requirements = self.root_table("requirements")?;
        if self.local(list, &policy.no_attributes)?.truthy() {
            for a in &policy.attributes {
                self.arena
                    .set_field(requirements, attr_mod(a.attribute), Value::Number(0.0))?;
            }
        } else if self.local(list, &policy.converted)?.truthy() {
            let mut conversions = [0.0; 3];
            for attr in policy.conversion_read_order {
                let rule = policy
                    .attributes
                    .iter()
                    .find(|r| r.attribute == attr)
                    .expect("validated attribute rule");
                conversions[attr_index(attr)] =
                    number(&self.local(list, &rule.conversion)?)? / policy.percent_divisor;
            }
            for attr in policy.converted_write_order {
                let rule = policy
                    .attributes
                    .iter()
                    .find(|r| r.attribute == attr)
                    .expect("validated attribute rule");
                let first = number(
                    &self
                        .arena
                        .get_field(requirements, attribute(rule.source_addends[0]))?,
                )?;
                let second = number(
                    &self
                        .arena
                        .get_field(requirements, attribute(rule.source_addends[1]))?,
                )?;
                let original = number(&self.arena.get_field(requirements, attribute(attr))?)?;
                let added = number(&self.local(list, &rule.base)?)?;
                let base = conversions[attr_index(attr)] * (first + second) + (original + added)
                    - original
                        * (conversions[attr_index(rule.conversion_subtrahends[0])]
                            + conversions[attr_index(rule.conversion_subtrahends[1])]);
                self.arena
                    .set_field(requirements, attr_base(attr), finite(base)?)?;
                let increased = number(&self.local(list, &rule.increased)?)?;
                self.arena.set_field(
                    requirements,
                    attr_mod(attr),
                    finite((base * (1.0 + increased / policy.percent_divisor)).floor())?,
                )?;
            }
        } else {
            for attr in policy.ordinary_write_order {
                let rule = policy
                    .attributes
                    .iter()
                    .find(|r| r.attribute == attr)
                    .expect("validated attribute rule");
                let original = number(&self.arena.get_field(requirements, attribute(attr))?)?;
                let added = number(&self.local(list, &rule.base)?)?;
                let increased = number(&self.local(list, &rule.increased)?)?;
                self.arena.set_field(
                    requirements,
                    attr_mod(attr),
                    finite(
                        ((original + added) * (1.0 + increased / policy.percent_divisor)).floor(),
                    )?,
                )?;
            }
        }
        Ok(())
    }
}
fn group_name(group: ItemAssemblyRows) -> &'static str {
    match group {
        ItemAssemblyRows::Enchant => "enchantModLines",
        ItemAssemblyRows::Rune => "runeModLines",
        ItemAssemblyRows::ClassRequirement => "classRequirementModLines",
        ItemAssemblyRows::Implicit => "implicitModLines",
        ItemAssemblyRows::Explicit => "explicitModLines",
    }
}
fn attribute(a: ItemAssemblyAttribute) -> &'static str {
    match a {
        ItemAssemblyAttribute::Strength => "str",
        ItemAssemblyAttribute::Dexterity => "dex",
        ItemAssemblyAttribute::Intelligence => "int",
    }
}
fn attr_mod(a: ItemAssemblyAttribute) -> &'static str {
    match a {
        ItemAssemblyAttribute::Strength => "strMod",
        ItemAssemblyAttribute::Dexterity => "dexMod",
        ItemAssemblyAttribute::Intelligence => "intMod",
    }
}
fn attr_base(a: ItemAssemblyAttribute) -> &'static str {
    match a {
        ItemAssemblyAttribute::Strength => "strBase",
        ItemAssemblyAttribute::Dexterity => "dexBase",
        ItemAssemblyAttribute::Intelligence => "intBase",
    }
}
fn attr_index(a: ItemAssemblyAttribute) -> usize {
    match a {
        ItemAssemblyAttribute::Strength => 0,
        ItemAssemblyAttribute::Dexterity => 1,
        ItemAssemblyAttribute::Intelligence => 2,
    }
}
