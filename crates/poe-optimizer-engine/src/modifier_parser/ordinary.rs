use super::*;
use ModifierValue as V;
use ParserDictionary as D;

impl Run<'_> {
    pub(super) fn ordinary(&mut self, source: &[u8], order: u8) -> ParserResult<ParseOutcome> {
        let mut line = source.to_vec();
        line.push(b' ');
        let pre = self.scan(&mut line, D::PreFlag, false)?;
        let pre_flag = self.prefix_factory(pre)?;
        let mut skill = self.scan(&mut line, D::PreSkillName, false)?.value;
        let form = self.scan(&mut line, D::Form, false)?;
        if !form.value.truthy() {
            return Ok(partial(None, line));
        }
        let tag = self.scan(&mut line, D::ModTag, false)?;
        let tag = self.tag_factory(tag, "modifier tag callback")?;
        let tag2 = if tag.truthy() {
            let selected = self.scan(&mut line, D::ModTag, false)?;
            self.tag_factory(selected, "second modifier tag callback")?
        } else {
            V::Nil
        };
        if order == 2 && !skill.truthy() {
            skill = self.scan(&mut line, D::SkillName, false)?.value;
        }
        let mut captures = form.captures;
        let opcode = form.value.as_bytes().unwrap_or_default();
        let mut name = match opcode {
            b"PEN" | b"BASECOST" | b"TOTALCOST" => {
                let dictionary = match opcode {
                    b"PEN" => D::Penetration,
                    b"BASECOST" => D::BaseCost,
                    _ => D::Cost,
                };
                let name = self.scan(&mut line, dictionary, true)?.value;
                if !name.truthy() {
                    return Ok(partial(Some(ModifierTable::default()), line));
                }
                self.scan(&mut line, D::ModName, true)?;
                name
            }
            b"FLAG" => {
                let selected = self.scan(&mut line, D::Flag, false)?.value;
                if !selected.truthy() {
                    return Ok(partial(None, line));
                }
                if captures.is_empty() {
                    captures.push(selected);
                } else {
                    captures[0] = selected;
                }
                self.scan(&mut line, D::ModName, true)?.value
            }
            _ => self.scan(&mut line, D::ModName, true)?.value,
        };
        if order == 1 && !skill.truthy() {
            skill = self.scan(&mut line, D::SkillName, false)?.value;
        }
        let mut flag = self.scan(&mut line, D::ModFlag, true)?.value;
        let first = captures.first().cloned().unwrap_or(V::Nil);
        let mut value = first.number().map(V::Number).unwrap_or(first);
        let mut kind = V::text("BASE");
        let mut suffix = V::Nil;
        let mut extra = V::Nil;
        let capture = |index: usize| captures.get(index).cloned().unwrap_or(V::Nil);
        match opcode {
            b"INC" => kind = V::text("INC"),
            b"RED" => {
                kind = V::text("INC");
                value = value.negated()?;
            }
            b"MORE" => kind = V::text("MORE"),
            b"LESS" => {
                kind = V::text("MORE");
                value = value.negated()?;
            }
            b"BASE" | b"GAIN" | b"LOSE" | b"GRANTS" | b"GRANTS_GLOBAL" | b"REMOVES" => {
                if matches!(opcode, b"LOSE" | b"REMOVES") {
                    value = value.negated()?;
                }
                if matches!(opcode, b"GRANTS" | b"REMOVES") {
                    extra = self.copy(&ParserValue::Table(
                        self.parser.catalog.data().policy.local_hand_tag,
                    ))?;
                }
                suffix = self.scan(&mut line, D::Suffix, true)?.value;
            }
            b"REGENPERCENT" | b"REGENFLAT" | b"DEGENPERCENT" | b"DEGENFLAT" => {
                name = self.lookup_capture(
                    if opcode.starts_with(b"REGEN") {
                        D::Regeneration
                    } else {
                        D::Degeneration
                    },
                    &capture(1),
                )?;
                if opcode.ends_with(b"PERCENT") {
                    suffix = V::text("Percent");
                }
            }
            b"DEGEN" => {
                let damage = self.lookup_capture(D::Damage, &capture(1))?;
                if !damage.truthy() {
                    return Ok(partial(Some(ModifierTable::default()), line));
                }
                name = concat(&damage, &V::text("Degen"))?;
                suffix = V::text("");
            }
            b"DMG" | b"DMGATTACKS" | b"DMGSPELLS" | b"DMGTHORNS" | b"DMGBOTH"
            | b"DMGTHORNSBASE" => {
                let thorns_base = opcode == b"DMGTHORNSBASE";
                let damage =
                    self.lookup_capture(D::Damage, &capture(if thorns_base { 0 } else { 2 }))?;
                if !damage.truthy() {
                    return Ok(partial(Some(ModifierTable::default()), line));
                }
                let base = self.parser.catalog.data().policy.thorns_base_damage;
                value = array([
                    if thorns_base {
                        V::Number(base)
                    } else {
                        capture(0).number().map(V::Number).unwrap_or(V::Nil)
                    },
                    if thorns_base {
                        V::Number(base)
                    } else {
                        capture(1).number().map(V::Number).unwrap_or(V::Nil)
                    },
                ]);
                let mut names = ModifierTable::from_values([
                    concat(&damage, &V::text("Min"))?,
                    concat(&damage, &V::text("Max"))?,
                ]);
                if thorns_base {
                    names.set("flags", self.policy_flag(false, "Thorns")?);
                }
                name = V::Table(Arc::new(names));
                if !flag.truthy() {
                    flag = match opcode {
                        b"DMGATTACKS" => {
                            record([("keywordFlags", self.policy_flag(true, "Attack")?)])
                        }
                        b"DMGSPELLS" => {
                            record([("keywordFlags", self.policy_flag(true, "Spell")?)])
                        }
                        b"DMGTHORNS" | b"DMGTHORNSBASE" => {
                            record([("flags", self.policy_flag(false, "Thorns")?)])
                        }
                        b"DMGBOTH" => record([(
                            "keywordFlags",
                            V::Number(or_flags(
                                &self.policy_flag(true, "Attack")?,
                                &self.policy_flag(true, "Spell")?,
                            )?),
                        )]),
                        _ => V::Nil,
                    };
                }
            }
            b"FLAG" => {
                name = if value.as_table().is_some() {
                    fallback(value.field("name")?.clone(), value.clone())
                } else {
                    value.clone()
                };
                kind = if value.as_table().is_some() {
                    fallback(value.field("type")?.clone(), V::text("FLAG"))
                } else {
                    V::text("FLAG")
                };
                value = if value.as_table().is_some() {
                    fallback(value.field("value")?.clone(), V::Boolean(true))
                } else {
                    V::Boolean(true)
                };
            }
            b"IMMUNE" => {
                let Some(names) = self.immune(&line)? else {
                    return Ok(partial(Some(ModifierTable::default()), line));
                };
                let flag_type = if value.as_table().is_some() {
                    fallback(value.field("type")?.clone(), V::text("FLAG"))
                } else {
                    V::text("FLAG")
                };
                let flag_value = if value.as_table().is_some() {
                    fallback(value.field("value")?.clone(), V::Boolean(true))
                } else {
                    V::Boolean(true)
                };
                if names.len() == 2 {
                    name = array(names);
                    kind = array([flag_type.clone(), flag_type]);
                    value = array([flag_value.clone(), flag_value]);
                } else {
                    name = names.into_iter().next().unwrap_or(V::Nil);
                    kind = flag_type;
                    value = flag_value;
                }
                line.clear();
            }
            b"OVERRIDE" => kind = V::text("OVERRIDE"),
            // The scalar branch allocates fresh tables. A table-valued
            // dictionary entry instead mutates shared parser state.
            b"DOUBLED" if name.truthy() => {
                [name, kind, value, extra] = scalar_doubled(
                    name,
                    &self.parser.catalog.data().policy,
                    self.budget,
                    &mut self.output,
                )?;
            }
            _ => {}
        }
        if !name.truthy() {
            return Ok(partial(Some(ModifierTable::default()), line));
        }
        let mods = self.emit(
            name,
            kind,
            value,
            suffix,
            [pre_flag, flag, tag, tag2, skill, extra],
        )?;
        Ok(ParseOutcome {
            modifiers: Some(mods),
            extra: line.iter().any(|&b| !space(b)).then_some(line),
        })
    }
    fn policy_flag(&mut self, keyword: bool, name: &str) -> ParserResult<V> {
        let policy = &self.parser.catalog.data().policy;
        let id = if keyword {
            policy.keyword_flags
        } else {
            policy.mod_flags
        };
        let raw = self
            .parser
            .catalog
            .table(id)
            .and_then(|t| t.fields.get(name))
            .cloned()
            .unwrap_or(ParserValue::Nil);
        self.copy(&raw)
    }
    fn lookup_capture(&mut self, dictionary: D, value: &V) -> ParserResult<V> {
        if let Some(key) = value.as_bytes() {
            self.exact(dictionary, key)
        } else if let V::Number(n) = value {
            let raw = if n.is_finite()
                && n.fract() == 0.0
                && *n >= i64::MIN as f64
                && *n < i64::MAX as f64
            {
                self.parser
                    .catalog
                    .dictionary(dictionary)
                    .indexed
                    .get(&(*n as i64))
                    .cloned()
                    .unwrap_or(ParserValue::Nil)
            } else {
                ParserValue::Nil
            };
            self.copy(&raw)
        } else {
            Ok(V::Nil)
        }
    }
    fn immune(&mut self, line: &[u8]) -> ParserResult<Option<Vec<V>>> {
        let mut effect = line.to_ascii_lowercase();
        while effect.last().is_some_and(|&b| space(b)) {
            effect.pop();
        }
        let policy = &self.parser.catalog.data().policy;
        let words = word_count(&effect);
        let join = effect.windows(5).position(|w| w == b" and ");
        let parts = if let Some(join) = join {
            let (before, after) = (&effect[..join], &effect[join + 5..]);
            if words > policy.immune_combined_min_words_exclusive as usize
                && (word_count(before) > policy.immune_max_part_words as usize
                    || word_count(after) > policy.immune_max_part_words as usize)
            {
                return Ok(None);
            }
            vec![before, after]
        } else {
            if words < 1
                || words > policy.immune_max_single_words as usize
                || policy
                    .immune_effect_blacklist
                    .iter()
                    .any(|s| s.as_bytes() == effect)
            {
                return Ok(None);
            }
            vec![effect.as_slice()]
        };
        let mut names = Vec::new();
        for part in parts {
            let replacement = self.exact(D::StatusToEffect, part)?;
            let raw = if replacement.truthy() {
                replacement.string()?
            } else {
                part.to_vec()
            };
            let mut name = Vec::new();
            for word in raw.split(|&b| space(b)).filter(|w| !w.is_empty()) {
                let start = name.len();
                name.extend_from_slice(word);
                name[start] = name[start].to_ascii_uppercase();
            }
            name.extend_from_slice(b"Immune");
            names.push(V::Bytes(name));
        }
        Ok(Some(names))
    }
}
/// Closed scalar arm of the original DOUBLED form. No source table is mutated.
/// In particular, do not convert a table-valued name into a copied scalar arm:
/// its second entry is a persistent shared-dictionary write in the source.
fn scalar_doubled(
    name: V,
    policy: &poe_optimizer_data::modifier_parser::ParserPolicy,
    budget: &mut MatchBudget,
    output: &mut OutputBudget,
) -> ParserResult<[V; 4]> {
    if matches!(name, V::Table(_)) {
        return Err(ParserError::Deferred {
            stage: "shared dictionary mutation in doubled form",
            callback: None,
        });
    }
    fn text(bytes: &[u8], output: &mut OutputBudget) -> ParserResult<V> {
        output.charge(bytes.len())?;
        Ok(V::text(bytes))
    }
    fn pair(values: [V; 2], output: &mut OutputBudget) -> ParserResult<V> {
        output.charge(0)?;
        output.charge(0)?;
        output.charge(0)?;
        Ok(array(values))
    }
    fn fields<const N: usize>(
        values: [(&str, V); N],
        output: &mut OutputBudget,
    ) -> ParserResult<V> {
        output.charge(0)?;
        for (key, _) in &values {
            output.charge(key.len())?;
        }
        Ok(record(values))
    }
    // Lua concatenation is right-associative and admits numbers, but does not
    // stringify booleans/functions. Preserve the scalar's original type in [1].
    let prefix = text(policy.doubled_multiplier_prefix.as_bytes(), output)?;
    let suffix = text(policy.doubled_name_suffix.as_bytes(), output)?;
    let tail = super::strings::concat(&name, &suffix, budget, output)?;
    let multiplier = super::strings::concat(&prefix, &tail, budget, output)?;
    let original = deep_copy(&name, output)?;
    let names = pair([original, multiplier], output)?;
    let kind = pair([text(b"MORE", output)?, text(b"OVERRIDE", output)?], output)?;
    let value = pair(
        [
            V::Number(policy.doubled_more),
            V::Number(policy.doubled_override),
        ],
        output,
    )?;
    output.charge(0)?;
    output.charge("tag".len())?;
    let mut extra = ModifierTable::default();
    extra.set("tag", V::Boolean(true));
    let tag_type = text(b"Multiplier", output)?;
    let var = super::strings::concat(&name, &suffix, budget, output)?;
    let limit_suffix = text(policy.doubled_limit_suffix.as_bytes(), output)?;
    let key = super::strings::concat(&name, &limit_suffix, budget, output)?;
    let tag = fields(
        [
            ("type", tag_type),
            ("var", var),
            ("globalLimit", V::Number(policy.doubled_global_limit)),
            ("globalLimitKey", key),
        ],
        output,
    )?;
    let row = fields([("tag", tag)], output)?;
    output.charge(0)?;
    extra.indexed.insert(1, row);
    Ok([names, kind, value, V::Table(Arc::new(extra))])
}
fn partial(modifiers: Option<ModifierTable>, line: Vec<u8>) -> ParseOutcome {
    ParseOutcome {
        modifiers,
        extra: Some(line),
    }
}
pub(super) fn space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 11 | 12)
}
fn word_count(line: &[u8]) -> usize {
    line.split(|&b| space(b)).filter(|s| !s.is_empty()).count()
}
pub(super) fn fallback(value: V, other: V) -> V {
    if value.truthy() { value } else { other }
}
pub(super) fn array(values: impl IntoIterator<Item = V>) -> V {
    V::Table(Arc::new(ModifierTable::from_values(values)))
}
pub(super) fn record<const N: usize>(fields: [(&str, V); N]) -> V {
    let mut table = ModifierTable::default();
    for (key, value) in fields {
        table.set(key, value);
    }
    V::Table(Arc::new(table))
}
pub(super) fn concat(left: &V, right: &V) -> ParserResult<V> {
    let mut left = left.string()?;
    let right = right.string()?;
    if left
        .len()
        .checked_add(right.len())
        .is_none_or(|n| n > super::value::MAX_OUTPUT_BYTES)
    {
        return Err(ParserError::ResourceBound("concatenated bytes"));
    }
    left.extend_from_slice(&right);
    Ok(V::Bytes(left))
}
pub(super) fn or_flags(left: &V, right: &V) -> ParserResult<f64> {
    let a = left
        .number()
        .ok_or_else(|| ParserError::SourceError("non-number flag operand".into()))?;
    let b = right
        .number()
        .ok_or_else(|| ParserError::SourceError("non-number flag operand".into()))?;
    Ok(crate::lua_bits::or53(a, b))
}

#[cfg(test)]
mod doubled_tests {
    use super::*;
    use crate::lua_pattern::{MatchLimits, PatternError, ResourceKind};
    use crate::modifier_scan::ScanError;
    use poe_optimizer_data::{game_data::bundled_snapshot, modifier_parser::ParserPolicy};

    fn policy() -> ParserPolicy {
        bundled_snapshot()
            .unwrap()
            .modifier_parser()
            .data()
            .policy
            .clone()
    }
    #[test]
    fn doubled_preserves_numeric_name_bits_before_emitter_concatenation() {
        let policy = policy();
        for name in [-0.0, 42.5, f64::INFINITY, f64::NEG_INFINITY] {
            let [names, _, _, extra] = scalar_doubled(
                V::Number(name),
                &policy,
                &mut MatchBudget::default(),
                &mut OutputBudget::default(),
            )
            .unwrap();
            let V::Number(retained) = names.as_table().unwrap().indexed_value(1) else {
                panic!()
            };
            assert_eq!(retained.to_bits(), name.to_bits());
            let rendered = crate::item_tools::lua_number_text(name);
            assert_eq!(
                names
                    .as_table()
                    .unwrap()
                    .indexed_value(2)
                    .as_bytes()
                    .unwrap(),
                format!(
                    "{}{rendered}{}",
                    policy.doubled_multiplier_prefix, policy.doubled_name_suffix
                )
                .as_bytes()
            );
            let first_tag = extra
                .as_table()
                .unwrap()
                .indexed_value(1)
                .as_table()
                .unwrap()
                .field("tag")
                .as_table()
                .unwrap();
            assert_eq!(
                first_tag.field("var").as_bytes().unwrap(),
                format!("{rendered}{}", policy.doubled_name_suffix).as_bytes()
            );
        }
    }
    #[test]
    fn doubled_concat_failure_precedes_later_output_and_match_charges() {
        let policy = policy();
        let literals = policy.doubled_multiplier_prefix.len() + policy.doubled_name_suffix.len();
        let mut output = OutputBudget::default();
        output
            .charge(super::super::value::MAX_OUTPUT_BYTES - literals)
            .unwrap();
        let mut work = MatchBudget::new(MatchLimits {
            max_steps: 0,
            ..Default::default()
        });
        assert_eq!(
            scalar_doubled(V::Boolean(true), &policy, &mut work, &mut output),
            Err(ParserError::SourceError(
                "concatenation of a non-string value".into()
            ))
        );
        assert_eq!(work.steps_used(), 0);
        let mut work = MatchBudget::new(MatchLimits {
            max_steps: 0,
            ..Default::default()
        });
        assert!(matches!(
            scalar_doubled(
                V::text("Caller"),
                &policy,
                &mut work,
                &mut OutputBudget::default()
            ),
            Err(ParserError::Scan(ScanError::Pattern(
                PatternError::Resource(ResourceKind::MatchSteps)
            )))
        ));
        let mut output = OutputBudget::default();
        output
            .charge(super::super::value::MAX_OUTPUT_BYTES - literals)
            .unwrap();
        assert!(matches!(
            scalar_doubled(
                V::text("Caller"),
                &policy,
                &mut MatchBudget::default(),
                &mut output
            ),
            Err(ParserError::ResourceBound("output values/bytes"))
        ));
    }
    #[test]
    fn doubled_table_frontier_precedes_all_scalar_allocations_and_keeps_aliases() {
        let table = Arc::new(ModifierTable::from_values([
            V::text("A"),
            V::text("B"),
            V::text("C"),
        ]));
        let original = table.clone();
        let mut output = OutputBudget::default();
        output
            .charge(super::super::value::MAX_OUTPUT_BYTES)
            .unwrap();
        let mut work = MatchBudget::new(MatchLimits {
            max_steps: 0,
            ..Default::default()
        });
        assert_eq!(
            scalar_doubled(V::Table(table.clone()), &policy(), &mut work, &mut output),
            Err(ParserError::Deferred {
                stage: "shared dictionary mutation in doubled form",
                callback: None
            })
        );
        assert_eq!(work.steps_used(), 0);
        assert!(Arc::ptr_eq(&table, &original));
        assert_eq!(table.indexed_value(2), &V::text("B"));
        assert_eq!(table.indexed_value(3), &V::text("C"));
    }
}
