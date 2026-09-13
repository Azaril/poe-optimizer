use super::{
    AssemblyError, Context, ItemLoadProvider, Result, TableId, Value, finite, number, table,
};
use poe_optimizer_data::item_assembly::{
    ItemAssemblyBorrowedQuery, ItemAssemblyLocalQuery, ItemAssemblyName,
};

impl<P: ItemLoadProvider + ?Sized> Context<'_, '_, P> {
    pub(super) fn local(&mut self, list: TableId, query: &ItemAssemblyLocalQuery) -> Result<Value> {
        self.local_query(
            list,
            ItemAssemblyBorrowedQuery {
                name: ItemAssemblyName::Text(&query.name),
                mod_type: &query.mod_type,
                flags: query.flags,
            },
        )
    }
    pub(super) fn local_query(
        &mut self,
        list: TableId,
        query: ItemAssemblyBorrowedQuery<'_>,
    ) -> Result<Value> {
        let policy = self.definitions.policy();
        let mut result = if query.mod_type == policy.kinds.flag {
            Value::Boolean(false)
        } else {
            Value::Number(f64::from(query.mod_type == policy.kinds.more))
        };
        let mut i = 1;
        self.arena.dense_len(list)?;
        loop {
            let value = self.arena.get_index(list, i)?;
            if !value.truthy() {
                break;
            }
            let record = table(value)?;
            let name = self.arena.get_field(record, "name")?;
            let matches = name
                .as_str()
                .is_some_and(|n| query.name.equals(n.as_bytes()))
                && self.arena.get_field(record, "type")?.as_str() == Some(query.mod_type)
                && self.arena.get_field(record, "flags")? == Value::Number(query.flags)
                && self.arena.get_field(record, "keywordFlags")?
                    == Value::Number(policy.local.keyword_flags);
            let eligible = if matches {
                let tag = self.arena.get_index(record, 1)?;
                !tag.truthy()
                    || self.arena.get_field(table(tag)?, "type")?.as_str()
                        == Some(&policy.local.in_slot_tag_type)
            } else {
                false
            };
            if eligible {
                if query.mod_type == policy.kinds.flag {
                    if !result.truthy() {
                        result = self.arena.get_field(record, "value")?;
                    }
                } else {
                    let value = number(&self.arena.get_field(record, "value")?)?;
                    let previous = number(&result)?;
                    result = finite(if query.mod_type == policy.kinds.more {
                        previous * ((policy.local.more_offset + value) / policy.local.more_divisor)
                    } else {
                        previous + value
                    })?;
                }
                self.arena.remove(list, i as usize)?;
            } else {
                i += 1;
            }
        }
        Ok(result)
    }
    pub(super) fn nil_query(&mut self, list: TableId, name: &str, flag: bool) -> Result<Value> {
        let policy = self.definitions.policy();
        let output = if flag {
            None
        } else {
            Some(self.arena.new_table()?)
        };
        let n = self.arena.dense_len(list)?;
        for i in 1..=n {
            let record = table(self.arena.get_index(list, i as i64)?)?;
            let kind = if flag {
                &policy.kinds.flag
            } else {
                &policy.kinds.list
            };
            if self.arena.get_field(record, "name")?.as_str() != Some(name)
                || self.arena.get_field(record, "type")?.as_str() != Some(kind)
            {
                continue;
            }
            let flags = self.arena.get_field(record, "flags")?;
            let matched = if matches!(flags, Value::Nil) {
                false
            } else {
                Value::Number(and53(policy.nil_queries.flags, number(&flags)?)?) == flags
            };
            if !matched {
                continue;
            }
            let keyword = number(&self.arena.get_field(record, "keywordFlags")?)?;
            let match_all = self
                .definitions
                .keyword_flags()
                .fields
                .get(&policy.nil_queries.match_all_field)
                .and_then(|v| v.as_f64())
                .ok_or_else(|| {
                    AssemblyError::unsupported("keyword MatchAll binding is not numeric")
                })?;
            let all = and53(keyword, match_all)? != 0.0;
            let modifier = and53(keyword, policy.nil_queries.captured_match_all_mask)?;
            let query = and53(
                policy.nil_queries.keyword_flags,
                policy.nil_queries.captured_match_all_mask,
            )?;
            if if all {
                and53(query, modifier)? != modifier
            } else {
                modifier != 0.0 && and53(query, modifier)? == 0.0
            } {
                continue;
            }
            if self.arena.get_index(record, 1)?.truthy() {
                return Err(AssemblyError::unsupported(
                    "selected item List/Flag tag requires ModStore EvalMod and actor state",
                ));
            }
            let value = self.arena.get_field(record, "value")?;
            if value.truthy() {
                if let Some(output) = output {
                    self.arena.append(output, value)?;
                } else {
                    return Ok(Value::Boolean(true));
                }
            }
        }
        Ok(output.map_or(Value::Nil, Value::Table))
    }
    pub(super) fn scale_add(&mut self, list: TableId, record: TableId, scale: f64) -> Result<()> {
        let n = self.arena.dense_len(record)?;
        let mut unscalable = false;
        for i in 1..=n {
            let tag = table(self.arena.get_index(record, i as i64)?)?;
            if self.arena.get_field(tag, "unscalable")?.truthy() {
                unscalable = true;
                break;
            }
        }
        if scale == 1.0 || unscalable {
            return self.arena.append(list, Value::Table(record));
        }
        let scaled = table(self.arena.deep_copy(&Value::Table(record))?)?;
        let mut sub = scaled;
        let policy = self.definitions.policy();
        if let Some(value) = self.arena.get_field(scaled, "value")?.as_table() {
            let nested = self.arena.get_field(value, "mod")?;
            if nested.truthy() {
                sub = table(nested)?;
            } else {
                let key = self.arena.get_field(value, "keyOfScaledMod")?;
                if key.truthy() {
                    let old = self.raw_key(value, &key)?;
                    let product = number(&old)? * scale;
                    let rounded = if self.arena.get_field(value, "key")?.as_str()
                        == Some(&policy.scale.integer_scaled_key)
                    {
                        product.floor()
                    } else {
                        round(product, policy.scale.keyed_value_decimal_places)
                    };
                    self.set_raw_key(value, &key, finite(rounded)?)?;
                }
            }
        }
        if let Value::Number(value) = self.arena.get_field(sub, "value")? {
            let name = self.arena.get_field(sub, "name")?;
            let kind = self.arena.get_field(sub, "type")?;
            let precision = name
                .as_str()
                .and_then(|name| self.definitions.high_precision_mods().get(name))
                .and_then(|kinds| {
                    kinds
                        .iter()
                        .find(|(operation, _)| Some(operation.upstream_name()) == kind.as_str())
                        .map(|(_, p)| *p)
                })
                .or_else(|| {
                    (value.floor() != value)
                        .then_some(self.definitions.scalability().data().default_high_precision)
                });
            let result = if let Some(p) = precision {
                let power = 10f64.powi(i32::from(p));
                (value * scale * power).floor() / power
            } else {
                round(value * scale, policy.scale.truncation_round_decimal_places).trunc()
            };
            self.arena.set_field(sub, "value", finite(result)?)?;
        }
        self.arena.append(list, Value::Table(scaled))
    }
    fn raw_key(&mut self, id: TableId, key: &Value) -> Result<Value> {
        match key {
            Value::Text(k) => self.arena.get_field(id, k),
            Value::Number(k) if k.fract() == 0.0 && k.abs() <= 9_007_199_254_740_991.0 => {
                self.arena.get_index(id, *k as i64)
            }
            Value::Nil => Ok(Value::Nil),
            _ => Err(AssemblyError::unsupported(
                "item scaled-value key is outside finite text/integer domain",
            )),
        }
    }
    fn set_raw_key(&mut self, id: TableId, key: &Value, value: Value) -> Result<()> {
        match key {
            Value::Text(k) => self.arena.set_field(id, k, value),
            Value::Number(k) if k.fract() == 0.0 && k.abs() <= 9_007_199_254_740_991.0 => {
                self.arena.set_index(id, *k as i64, value)
            }
            Value::Nil => Err(AssemblyError::source("table index is nil")),
            _ => Err(AssemblyError::unsupported(
                "item scaled-value key is outside finite text/integer domain",
            )),
        }
    }
}
pub(super) fn round(value: f64, places: u8) -> f64 {
    let power = 10f64.powi(i32::from(places));
    (value * power + 0.5).floor() / power
}
fn bit(value: f64) -> Result<i32> {
    if !value.is_finite() {
        return Err(AssemblyError::unsupported("nonfinite item bit conversion"));
    }
    Ok((value + 6_755_399_441_055_744.0).to_bits() as i32)
}
fn and53(a: f64, b: f64) -> Result<f64> {
    if a.fract() != 0.0
        || b.fract() != 0.0
        || a.abs() > 9_007_199_254_740_991.0
        || b.abs() > 9_007_199_254_740_991.0
    {
        return Err(AssemblyError::unsupported(
            "item query flags require finite exactly represented integer masks",
        ));
    }
    let ah = (a / 4_294_967_296.0).floor();
    let bh = (b / 4_294_967_296.0).floor();
    let high = bit(ah)? & bit(bh)?;
    let low = bit(a - ah * 4_294_967_296.0)? & bit(b - bh * 4_294_967_296.0)?;
    Ok(f64::from(high & 0x1f_ffff) * 4_294_967_296.0 + f64::from(low))
}
