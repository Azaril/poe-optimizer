use super::*;
use ModifierValue as V;
use ParserDictionary as D;

impl Run<'_> {
    pub(super) fn ordinary(&mut self, source: &[u8], order: u8) -> ParserResult<ParseOutcome> {
        let mut line = source.to_vec();
        line.push(b' ');
        let pre = self.scan(&mut line, D::PreFlag, false)?;
        let pre_flag = self.callback(pre.value, "prefix callback")?;
        let mut skill = self.scan(&mut line, D::PreSkillName, false)?.value;
        let form = self.scan(&mut line, D::Form, false)?;
        if !form.value.truthy() {
            return Ok(partial(None, line));
        }
        let tag = self.scan(&mut line, D::ModTag, false)?;
        let tag = self.callback(tag.value, "modifier tag callback")?;
        let tag2 = if tag.truthy() {
            let selected = self.scan(&mut line, D::ModTag, false)?;
            self.callback(selected.value, "second modifier tag callback")?
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
            b"DOUBLED" => {
                return Err(ParserError::Deferred {
                    stage: "shared dictionary mutation in doubled form",
                    callback: None,
                });
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
