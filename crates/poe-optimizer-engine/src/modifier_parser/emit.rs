use super::ordinary::{concat, fallback, or_flags};
use super::*;
use ModifierValue as V;

impl Run<'_> {
    pub(super) fn emit(
        &mut self,
        name: V,
        kind: V,
        value: V,
        suffix: V,
        contributions: [V; 6],
    ) -> ParserResult<ModifierTable> {
        let mut flags = 0.0;
        let mut keywords = 0.0;
        let mut tags = Vec::new();
        let mut per_mod: Option<Vec<Vec<V>>> = None;
        let mut misc = ModifierTable::default();
        // Original literal array has seven source positions; pairs traverses
        // all numeric positions, including those beyond absent earlier values.
        for contribution in std::iter::once(&name).chain(contributions.iter()) {
            let Some(data) = contribution.as_table() else {
                continue;
            };
            flags = or_flags(
                &V::Number(flags),
                &fallback(data.field("flags").clone(), V::Number(0.0)),
            )?;
            keywords = or_flags(
                &V::Number(keywords),
                &fallback(data.field("keywordFlags").clone(), V::Number(0.0)),
            )?;
            let first = data.indexed_value(1);
            if data.field("tag").truthy() {
                if first.truthy() && first.field("tag")?.truthy() {
                    let mut list = Vec::new();
                    for row in data.dense() {
                        let tag = row.field("tag")?;
                        list.push(if tag.truthy() {
                            vec![self.copy_table(tag)?]
                        } else {
                            Vec::new()
                        });
                    }
                    per_mod = Some(list);
                } else {
                    tags.push(self.copy_table(data.field("tag"))?);
                }
            } else if data.field("tagList").truthy() {
                if first.truthy() && first.field("tagList")?.truthy() {
                    let mut list = Vec::new();
                    for row in data.dense() {
                        let mut row_tags = Vec::new();
                        for tag in row.field("tagList")?.table()?.dense() {
                            row_tags.push(self.copy_table(&tag)?);
                        }
                        list.push(row_tags);
                    }
                    per_mod = Some(list);
                } else {
                    for tag in data.field("tagList").table()?.dense() {
                        tags.push(self.copy_table(&tag)?);
                    }
                }
            }
            for (key, value) in &data.fields {
                self.output.charge(key.len() + scalar_bytes(value))?;
                misc.fields.insert(key.clone(), value.clone());
            }
            for (key, value) in &data.indexed {
                self.output.charge(scalar_bytes(value))?;
                misc.indexed.insert(*key, value.clone());
            }
        }
        let names = name
            .as_table()
            .map_or_else(|| vec![name.clone()], ModifierTable::dense);
        let suffix = fallback(
            suffix,
            fallback(misc.field("modSuffix").clone(), V::text("")),
        );
        let mut mods = Vec::new();
        for (i, name) in names.into_iter().enumerate() {
            self.output.charge(0)?;
            for tag in &tags {
                self.output.charge(scalar_bytes(tag))?;
            }
            let row_name = concat(&name, &suffix)?;
            let row_kind = indexed_or_whole(&kind, i as i64 + 1);
            let row_value = indexed_or_whole(&value, i as i64 + 1);
            for field in [&row_name, &row_kind, &row_value] {
                self.output.charge(scalar_bytes(field))?;
            }
            let mut row = ModifierTable::from_values(tags.clone());
            row.set("name", row_name);
            row.set("type", row_kind);
            row.set("value", row_value);
            row.set("flags", V::Number(flags));
            row.set("keywordFlags", V::Number(keywords));
            if let Some(extra) = per_mod.as_ref().and_then(|list| list.get(i)) {
                // Every entry was produced by copyTable above. With two entries
                // the first table becomes table.insert's invalid position; zero
                // or >2 entries have invalid arity in the original runtime.
                if extra.len() != 1 {
                    return Err(ParserError::SourceError(
                        "invalid table.insert arguments for modifier tags".into(),
                    ));
                }
                row.indexed
                    .insert(row.indexed.len() as i64 + 1, extra[0].clone());
            }
            mods.push(row);
        }
        // The original constructs every raw row before entering wrapper logic.
        // Preserve this ordering so later raw-row errors precede wrapper errors.
        let mods = mods
            .into_iter()
            .map(|row| self.wrap(row, &misc))
            .collect::<ParserResult<Vec<_>>>()?;
        Ok(ModifierTable::from_values(
            mods.into_iter().map(|t| V::Table(Arc::new(t))),
        ))
    }
    fn copy_table(&mut self, value: &V) -> ParserResult<V> {
        value.table()?;
        deep_copy(value, &mut self.output)
    }
    fn wrap(
        &mut self,
        mut effect: ModifierTable,
        misc: &ModifierTable,
    ) -> ParserResult<ModifierTable> {
        if misc.field("addToAura").truthy() {
            let tags = if misc.field("onlyAddToBanners").truthy() {
                let id = self.parser.catalog.data().policy.skill_types;
                let banner = self
                    .parser
                    .catalog
                    .table(id)
                    .and_then(|t| t.fields.get("Banner"))
                    .cloned()
                    .unwrap_or(ParserValue::Nil);
                let banner = self.copy(&banner)?;
                self.output.charge(0)?;
                self.output.charge("type".len() + "SkillType".len())?;
                self.output.charge("skillType".len())?;
                vec![super::ordinary::record([
                    ("type", V::text("SkillType")),
                    ("skillType", banner),
                ])]
            } else {
                Vec::new()
            };
            let value = self.wrapper_payload(effect)?;
            self.finish_wrapper("ExtraAuraEffect", value, tags)
        } else if misc.field("newAura").truthy() {
            let mut tags = Vec::new();
            self.append_dense(&mut tags, &effect)?;
            for index in 1..=tags.len() {
                effect.indexed.remove(&(index as i64));
            }
            let mut value = self.payload_table(effect)?;
            let only_allies = self.clone_wrapper_value(misc.field("newAuraOnlyAllies"))?;
            self.output.charge("onlyAllies".len())?;
            value.set("onlyAllies", only_allies);
            self.finish_wrapper("ExtraAura", V::Table(Arc::new(value)), tags)
        } else if misc.field("addToMinion").truthy() {
            let mut tags = Vec::new();
            for key in ["playerTag", "addToMinionTag"] {
                if misc.field(key).truthy() {
                    tags.push(self.clone_wrapper_value(misc.field(key))?);
                }
            }
            self.append_player_tags(&mut tags, misc)?;
            let value = self.wrapper_payload(effect)?;
            self.finish_wrapper("MinionModifier", value, tags)
        } else if misc.field("addToSkill").truthy() {
            let tag = self.clone_wrapper_value(misc.field("addToSkill"))?;
            let value = self.wrapper_payload(effect)?;
            self.finish_wrapper("ExtraSkillMod", value, vec![tag])
        } else if misc.field("applyToEnemy").truthy() {
            let mut tags = Vec::new();
            if misc.field("playerTag").truthy() {
                tags.push(self.clone_wrapper_value(misc.field("playerTag"))?);
            }
            self.append_player_tags(&mut tags, misc)?;
            if effect.indexed_value(1).truthy() && misc.field("actorEnemy").truthy() {
                let copied = self.copy_table(&V::Table(Arc::new(effect)))?;
                effect = self.clone_wrapper_table(copied.table()?)?;
                let mut first = self.clone_wrapper_table(effect.indexed_value(1).table()?)?;
                self.output.charge("actor".len() + "enemy".len())?;
                first.set("actor", V::text("enemy"));
                self.output.charge(0)?;
                effect.indexed.insert(1, V::Table(Arc::new(first)));
            }
            let value = self.wrapper_payload(effect)?;
            self.finish_wrapper("EnemyModifier", value, tags)
        } else {
            Ok(effect)
        }
    }
    /// Charge each owned scalar clone before allocating it. Arc table references
    /// cost one value here; their eventual recursive copy has its own budget.
    fn clone_wrapper_value(&mut self, value: &V) -> ParserResult<V> {
        self.output.charge(scalar_bytes(value))?;
        Ok(value.clone())
    }
    fn clone_wrapper_table(&mut self, source: &ModifierTable) -> ParserResult<ModifierTable> {
        self.output.charge(0)?;
        let mut result = ModifierTable::default();
        for (key, value) in &source.fields {
            self.output.charge(key.len())?;
            let value = self.clone_wrapper_value(value)?;
            result.fields.insert(key.clone(), value);
        }
        for (key, value) in &source.indexed {
            result
                .indexed
                .insert(*key, self.clone_wrapper_value(value)?);
        }
        Ok(result)
    }
    fn append_dense(&mut self, out: &mut Vec<V>, source: &ModifierTable) -> ParserResult<()> {
        // Do not call dense(): it clones the entire source before a bound can be
        // checked. Wrapper fanout can repeat this list once per emitted modifier.
        for index in 1.. {
            let Some(value) = source.indexed.get(&index) else {
                break;
            };
            out.push(self.clone_wrapper_value(value)?);
        }
        Ok(())
    }
    fn append_player_tags(&mut self, tags: &mut Vec<V>, misc: &ModifierTable) -> ParserResult<()> {
        if misc.field("playerTagList").truthy() {
            self.append_dense(tags, misc.field("playerTagList").table()?)?;
        }
        Ok(())
    }
    fn payload_table(&mut self, effect: ModifierTable) -> ParserResult<ModifierTable> {
        self.output.charge(0)?;
        self.output.charge("mod".len())?;
        let mut value = ModifierTable::default();
        value.set("mod", V::Table(Arc::new(effect)));
        Ok(value)
    }
    fn wrapper_payload(&mut self, effect: ModifierTable) -> ParserResult<V> {
        Ok(V::Table(Arc::new(self.payload_table(effect)?)))
    }
    fn finish_wrapper(
        &mut self,
        name: &str,
        value: V,
        args: Vec<V>,
    ) -> ParserResult<ModifierTable> {
        self.output.charge(0)?;
        for (key, text) in [
            ("name", name),
            ("type", "LIST"),
            ("value", ""),
            ("flags", ""),
            ("keywordFlags", ""),
        ] {
            self.output.charge(key.len() + text.len())?;
        }
        // createMod clones a leading string into `source` before consuming its
        // positional argument. Account for that additional owned byte copy too.
        if let Some(V::Bytes(source)) = args.first() {
            self.output.charge("source".len() + source.len())?;
        }
        Ok(make_mod(name, "LIST", value, args))
    }
}
fn indexed_or_whole(value: &V, index: i64) -> V {
    if let Some(table) = value.as_table() {
        fallback(table.indexed_value(index).clone(), value.clone())
    } else {
        value.clone()
    }
}

fn scalar_bytes(value: &V) -> usize {
    if let V::Bytes(bytes) = value {
        bytes.len()
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lua_pattern::MatchBudget;
    use std::sync::OnceLock;

    fn parser() -> &'static CompiledModifierParser {
        static PARSER: OnceLock<CompiledModifierParser> = OnceLock::new();
        PARSER.get_or_init(|| {
            let data = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
            CompiledModifierParser::new(data.modifier_parser()).unwrap()
        })
    }
    fn run(budget: &mut MatchBudget, remaining_bytes: usize) -> Run<'_> {
        let mut output = OutputBudget::default();
        output
            .charge(super::super::value::MAX_OUTPUT_BYTES - remaining_bytes)
            .unwrap();
        Run {
            parser: parser(),
            budget,
            output,
            source_tables: BTreeMap::new(),
        }
    }
    fn fields(values: impl IntoIterator<Item = (&'static str, V)>) -> ModifierTable {
        let mut table = ModifierTable::default();
        for (key, value) in values {
            table.set(key, value);
        }
        table
    }
    fn list(values: impl IntoIterator<Item = V>) -> V {
        V::Table(Arc::new(ModifierTable::from_values(values)))
    }
    fn effect() -> ModifierTable {
        make_mod("Life", "BASE", V::Number(1.0), vec![])
    }
    fn resource(result: ParserResult<ModifierTable>) {
        assert!(
            matches!(result, Err(ParserError::ResourceBound(_))),
            "{result:?}"
        );
    }
    #[test]
    fn player_tag_list_stops_inside_wrapper_before_public_copy() {
        for wrapper in ["addToMinion", "applyToEnemy"] {
            let misc = fields([
                (wrapper, V::Boolean(true)),
                (
                    "playerTagList",
                    list([V::text("1234567890123456"), V::text("1234567890123456")]),
                ),
            ]);
            let mut work = MatchBudget::default();
            // The two existing strings fit in the catalog; only 24 bytes remain
            // for this wrapper. No public recursive-copy step is invoked here.
            resource(run(&mut work, 24).wrap(effect(), &misc));
        }
    }
    #[test]
    fn standalone_wrapper_tags_are_charged_before_a_later_source_error() {
        for (wrapper, key) in [
            ("addToMinion", "playerTag"),
            ("addToMinion", "addToMinionTag"),
            ("applyToEnemy", "playerTag"),
        ] {
            let misc = fields([
                (wrapper, V::Boolean(true)),
                (key, V::text("123456789")),
                ("playerTagList", V::Boolean(true)),
            ]);
            let mut work = MatchBudget::default();
            // An unchecked scalar clone would reach the malformed playerTagList
            // and return SourceError. Reject its allocation before that read.
            resource(run(&mut work, 8).wrap(effect(), &misc));
        }
        let mut work = MatchBudget::default();
        resource(run(&mut work, 8).wrap(effect(), &fields([("addToSkill", V::text("123456789"))])));
    }
    #[test]
    fn new_aura_tag_transfer_and_allies_value_have_early_bounds() {
        let mut effect = effect();
        effect.indexed.insert(1, V::text("1234567890123456"));
        effect.indexed.insert(2, V::text("1234567890123456"));
        let mut work = MatchBudget::default();
        resource(run(&mut work, 24).wrap(effect, &fields([("newAura", V::Boolean(true))])));
        let mut work = MatchBudget::default();
        resource(run(&mut work, 32).wrap(
            self::effect(),
            &fields([
                ("newAura", V::Boolean(true)),
                ("newAuraOnlyAllies", V::Bytes(vec![b'x'; 64])),
            ]),
        ));
    }
    #[test]
    fn enemy_tag_shallow_copies_are_bounded_after_the_source_deep_copy() {
        let mut effect = effect();
        effect.indexed.insert(
            1,
            V::Table(Arc::new(fields([("var", V::Bytes(vec![b'x'; 64]))]))),
        );
        let misc = fields([
            ("applyToEnemy", V::Boolean(true)),
            ("actorEnemy", V::Boolean(true)),
        ]);
        let mut work = MatchBudget::default();
        // Source deep copying uses 105 bytes; the following Rust-owned shallow
        // copies would exceed 150. The old wrapper returned Ok until public copy.
        resource(run(&mut work, 150).wrap(effect, &misc));
    }
    #[test]
    fn wrapper_metadata_and_source_reclone_are_bounded_inside_wrapper() {
        let mut work = MatchBudget::default();
        resource(run(&mut work, 170).wrap(
            effect(),
            &fields([
                ("addToMinion", V::Boolean(true)),
                ("playerTag", V::Bytes(vec![b'x'; 64])),
            ]),
        ));
        let mut work = MatchBudget::default();
        resource(run(&mut work, 1).wrap(effect(), &fields([("addToAura", V::Boolean(true))])));
    }
    #[test]
    fn bounded_wrappers_keep_source_fields_and_tag_order() {
        let misc = fields([
            ("addToMinion", V::Boolean(true)),
            ("playerTag", V::text("source")),
            ("addToMinionTag", V::Number(17.0)),
            ("playerTagList", list([V::Number(23.0), V::text("tail")])),
        ]);
        let mut work = MatchBudget::default();
        let result = run(&mut work, 4096).wrap(effect(), &misc).unwrap();
        assert_eq!(result.field("name"), &V::text("MinionModifier"));
        assert_eq!(result.field("source"), &V::text("source"));
        assert_eq!(result.field("flags"), &V::Number(17.0));
        assert_eq!(result.field("keywordFlags"), &V::Number(23.0));
        assert_eq!(result.indexed_value(1), &V::text("tail"));
    }
}
