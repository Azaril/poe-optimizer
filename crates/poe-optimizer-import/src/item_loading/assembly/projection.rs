//! Bounded diagnostic/loading projection of the authoritative owned graph.
//! Metadata copies do not preserve graph aliases; executable consumers retain
//! AssembledItem. Input hydration failures must not be sent to this adapter.
use super::{AssembledItem, AssemblyError, AssemblyTableId, AssemblyValue};
use crate::item_loading::{
    ArmourDataUpdate, AssemblyModifierPayloads, AssemblyOutcome, ItemNumber, ItemScalar, ItemState,
    LoadedModLine, MAX_ITEM_LOADING_EVIDENCE_BYTES,
};
use poe_optimizer_data::item_loading::{ItemMetadataTable, ItemMetadataValue};
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, AssemblyError>;
const MAX_VALUES: usize = 65_536;
const MAX_DEPTH: usize = 64;

#[derive(Default)]
struct Budget {
    values: usize,
    bytes: usize,
}
impl Budget {
    fn charge(&mut self, bytes: usize) -> Result<()> {
        let values = self
            .values
            .checked_add(1)
            .ok_or_else(|| AssemblyError::resource("item projection value overflow"))?;
        let bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or_else(|| AssemblyError::resource("item projection byte overflow"))?;
        if values > MAX_VALUES || bytes > MAX_ITEM_LOADING_EVIDENCE_BYTES {
            return Err(AssemblyError::resource("item loading projection bound"));
        }
        self.values = values;
        self.bytes = bytes;
        Ok(())
    }
    fn key(&mut self, key: &str) -> Result<()> {
        self.charge(
            key.len()
                .checked_add(64)
                .ok_or_else(|| AssemblyError::resource("item projection key bytes"))?,
        )
    }
    fn depth(&self, depth: usize) -> Result<()> {
        if depth > MAX_DEPTH {
            Err(AssemblyError::resource("item loading projection depth"))
        } else {
            Ok(())
        }
    }
}

/// Construct loading updates after a complete hydration and any reached assembly
/// prefix. Missing unreached line/requirement groups in a partial artifact retain
/// their previous diagnostic values. The result never upgrades its completion bit.
pub fn loading_updates(item: &AssembledItem, before: &ItemState) -> Result<AssemblyOutcome> {
    let root = item
        .table(item.root())
        .ok_or_else(|| AssemblyError::source("item projection root is missing"))?;
    if !root.indexed.is_empty() {
        return Err(AssemblyError::unsupported(
            "indexed item root cannot become loading fields",
        ));
    }
    let mut budget = Budget::default();
    let mut updates = BTreeMap::new();
    for (key, value) in &root.fields {
        let scalar = match value {
            AssemblyValue::Nil => Some(ItemScalar::Number(ItemNumber::Nil)),
            AssemblyValue::Boolean(v) => Some(ItemScalar::Boolean(*v)),
            AssemblyValue::Number(n) => Some(ItemScalar::Number(number(*n)?)),
            AssemblyValue::Text(s) => {
                budget.charge(s.len())?;
                Some(ItemScalar::Text(s.clone()))
            }
            AssemblyValue::Table(_) => None,
        };
        if let Some(scalar) = scalar {
            budget.key(key)?;
            updates.insert(key.clone(), scalar);
        }
    }
    for key in before.retained_fields.keys() {
        if !root.fields.contains_key(key) {
            budget.key(key)?;
            updates.insert(key.clone(), ItemScalar::Number(ItemNumber::Nil));
        }
    }
    let requirements = match root.fields.get("requirements") {
        None | Some(AssemblyValue::Nil) if !item.is_complete() => None,
        Some(AssemblyValue::Table(id)) => Some(numeric_table(item, *id, &mut budget)?),
        _ => {
            return Err(AssemblyError::unsupported(
                "complete item requirements are not a numeric table",
            ));
        }
    };
    let armour_data = match root.fields.get("armourData") {
        None | Some(AssemblyValue::Nil) if !item.is_complete() => ArmourDataUpdate::Preserve,
        None | Some(AssemblyValue::Nil) => ArmourDataUpdate::Clear,
        Some(AssemblyValue::Table(id)) => armour_table(item, *id, &mut budget)?,
        _ => {
            return Err(AssemblyError::unsupported(
                "item armour data is not a numeric table",
            ));
        }
    };
    let mut convert = |name, rows| group(item, name, rows, &mut budget);
    let modifier_payloads = AssemblyModifierPayloads {
        buff_mod_lines: convert("buffModLines", &before.buff_mod_lines)?,
        enchant_mod_lines: convert("enchantModLines", &before.enchant_mod_lines)?,
        rune_mod_lines: convert("runeModLines", &before.rune_mod_lines)?,
        class_requirement_mod_lines: convert(
            "classRequirementModLines",
            &before.class_requirement_mod_lines,
        )?,
        implicit_mod_lines: convert("implicitModLines", &before.implicit_mod_lines)?,
        explicit_mod_lines: convert("explicitModLines", &before.explicit_mod_lines)?,
    };
    Ok(AssemblyOutcome {
        assembled: Some(item.clone()),
        armour_data,
        modifier_payloads: Some(modifier_payloads),
        requirements,
        state_updates: updates,
        evidence: ItemMetadataTable::default(),
    })
}
fn number(n: f64) -> Result<ItemNumber> {
    if n.is_finite() {
        Ok(ItemNumber::new(n))
    } else {
        Err(AssemblyError::unsupported(
            "nonfinite item loading projection",
        ))
    }
}
/// Only this compatibility field has an explicitly marked numeric subset.
/// The owned graph retains every skipped value, key and alias.
fn armour_table(
    item: &AssembledItem,
    id: AssemblyTableId,
    budget: &mut Budget,
) -> Result<ArmourDataUpdate> {
    let table = item
        .table(id)
        .ok_or_else(|| AssemblyError::source("item armour table reference is missing"))?;
    budget.charge(64)?;
    let mut complete = table.indexed.is_empty();
    let mut out = BTreeMap::new();
    for (key, value) in &table.fields {
        budget.key(key)?;
        if let AssemblyValue::Number(n) = value {
            out.insert(key.clone(), number(*n)?);
        } else {
            complete = false;
        }
    }
    for _ in &table.indexed {
        budget.charge(std::mem::size_of::<i64>())?;
    }
    Ok(if complete {
        ArmourDataUpdate::Replace(out)
    } else {
        ArmourDataUpdate::NumericSubset(out)
    })
}
fn numeric_table(
    item: &AssembledItem,
    id: AssemblyTableId,
    budget: &mut Budget,
) -> Result<BTreeMap<String, ItemNumber>> {
    let table = item
        .table(id)
        .ok_or_else(|| AssemblyError::source("item numeric table reference is missing"))?;
    if !table.indexed.is_empty() {
        return Err(AssemblyError::unsupported(
            "indexed numeric item fields cannot be projected",
        ));
    }
    budget.charge(64)?;
    let mut out = BTreeMap::new();
    for (key, value) in &table.fields {
        let AssemblyValue::Number(n) = value else {
            return Err(AssemblyError::unsupported("nonnumeric item numeric field"));
        };
        let n = number(*n)?;
        budget.key(key)?;
        out.insert(key.clone(), n);
    }
    Ok(out)
}
fn group(
    item: &AssembledItem,
    name: &str,
    before: &[LoadedModLine],
    budget: &mut Budget,
) -> Result<Vec<Vec<ItemMetadataTable>>> {
    budget.charge(
        before
            .len()
            .checked_mul(std::mem::size_of::<Vec<ItemMetadataTable>>())
            .ok_or_else(|| AssemblyError::resource("item line projection bytes"))?,
    )?;
    let list = match item.field(item.root(), name) {
        None | Some(AssemblyValue::Nil) if !item.is_complete() => None,
        Some(AssemblyValue::Table(id)) => Some(
            item.table(*id)
                .ok_or_else(|| AssemblyError::source("missing item line group"))?,
        ),
        _ => {
            return Err(AssemblyError::unsupported(
                "complete item line group is unavailable",
            ));
        }
    };
    if let Some(list) = list
        && (!list.fields.is_empty()
            || list
                .indexed
                .keys()
                .any(|k| *k < 1 || (*k as u128) > before.len() as u128)
            || (item.is_complete() && list.indexed.len() != before.len()))
    {
        return Err(AssemblyError::unsupported(
            "item projection line topology differs from loading state",
        ));
    }
    let mut out = Vec::with_capacity(before.len());
    for (index, row) in before.iter().enumerate() {
        let value = list.and_then(|list| list.indexed.get(&(index as i64 + 1)));
        let mods = match value {
            None | Some(AssemblyValue::Nil) if !item.is_complete() => None,
            Some(AssemblyValue::Table(id)) => match item.field(*id, "modList") {
                None | Some(AssemblyValue::Nil) if !item.is_complete() => None,
                Some(AssemblyValue::Table(id)) => Some(*id),
                _ => return Err(AssemblyError::unsupported("item modifier row has no list")),
            },
            _ => return Err(AssemblyError::unsupported("item line row is not a table")),
        };
        let projected = if let Some(mods) = mods {
            modifier_list(item, mods, budget)?
        } else {
            budget.charge(
                row.modifiers
                    .len()
                    .checked_mul(std::mem::size_of::<ItemMetadataTable>())
                    .ok_or_else(|| AssemblyError::resource("prior modifier projection bytes"))?,
            )?;
            let mut prior = Vec::with_capacity(row.modifiers.len());
            for value in &row.modifiers {
                prior.push(copy_metadata(value, 0, budget)?);
            }
            prior
        };
        out.push(projected);
    }
    Ok(out)
}
fn modifier_list(
    item: &AssembledItem,
    id: AssemblyTableId,
    budget: &mut Budget,
) -> Result<Vec<ItemMetadataTable>> {
    let list = item
        .table(id)
        .ok_or_else(|| AssemblyError::source("item modifier list reference is missing"))?;
    if !list.fields.is_empty()
        || !list
            .indexed
            .keys()
            .copied()
            .eq(1..=list.indexed.len() as i64)
        || list.indexed.len() > 4096
    {
        return Err(AssemblyError::unsupported(
            "item modifier payload is not a bounded dense list",
        ));
    }
    budget.charge(
        list.indexed
            .len()
            .checked_mul(std::mem::size_of::<ItemMetadataTable>())
            .ok_or_else(|| AssemblyError::resource("modifier projection bytes"))?,
    )?;
    let mut out = Vec::with_capacity(list.indexed.len());
    for value in list.indexed.values() {
        let AssemblyValue::Table(id) = value else {
            return Err(AssemblyError::unsupported(
                "item modifier payload contains a non-table record",
            ));
        };
        out.push(graph_table(item, *id, 0, &mut BTreeSet::new(), budget)?);
    }
    Ok(out)
}
fn graph_table(
    item: &AssembledItem,
    id: AssemblyTableId,
    depth: usize,
    path: &mut BTreeSet<AssemblyTableId>,
    budget: &mut Budget,
) -> Result<ItemMetadataTable> {
    budget.depth(depth)?;
    budget.charge(std::mem::size_of::<ItemMetadataTable>() + 64)?;
    if !path.insert(id) {
        return Err(AssemblyError::unsupported(
            "cyclic item graph cannot become diagnostic metadata",
        ));
    }
    let table = item
        .table(id)
        .ok_or_else(|| AssemblyError::source("item metadata graph reference is missing"))?;
    let mut out = ItemMetadataTable::default();
    for (key, value) in &table.fields {
        budget.key(key)?;
        let value = graph_value(item, value, depth + 1, path, budget)?;
        out.fields.insert(key.clone(), value);
    }
    for (key, value) in &table.indexed {
        budget.charge(64)?;
        out.indexed
            .insert(*key, graph_value(item, value, depth + 1, path, budget)?);
    }
    path.remove(&id);
    Ok(out)
}
fn graph_value(
    item: &AssembledItem,
    value: &AssemblyValue,
    depth: usize,
    path: &mut BTreeSet<AssemblyTableId>,
    budget: &mut Budget,
) -> Result<ItemMetadataValue> {
    budget.depth(depth)?;
    budget.charge(std::mem::size_of::<ItemMetadataValue>())?;
    Ok(match value {
        AssemblyValue::Nil => {
            return Err(AssemblyError::unsupported(
                "explicit nil cannot become metadata field",
            ));
        }
        AssemblyValue::Boolean(v) => ItemMetadataValue::Boolean(*v),
        AssemblyValue::Number(n) => {
            number(*n)?;
            ItemMetadataValue::Number(*n)
        }
        AssemblyValue::Text(s) => {
            budget.charge(s.len())?;
            ItemMetadataValue::Text(s.clone())
        }
        AssemblyValue::Table(id) => {
            let table = item
                .table(*id)
                .ok_or_else(|| AssemblyError::source("missing nested item metadata table"))?;
            if table.fields.is_empty()
                && !table.indexed.is_empty()
                && table
                    .indexed
                    .keys()
                    .copied()
                    .eq(1..=table.indexed.len() as i64)
            {
                budget.charge(
                    table
                        .indexed
                        .len()
                        .checked_mul(std::mem::size_of::<ItemMetadataValue>())
                        .ok_or_else(|| AssemblyError::resource("metadata array bytes"))?,
                )?;
                if !path.insert(*id) {
                    return Err(AssemblyError::unsupported(
                        "cyclic item graph cannot become diagnostic metadata",
                    ));
                }
                let mut out = Vec::with_capacity(table.indexed.len());
                for value in table.indexed.values() {
                    out.push(graph_value(item, value, depth + 1, path, budget)?);
                }
                path.remove(id);
                ItemMetadataValue::Array(out)
            } else {
                ItemMetadataValue::Table(graph_table(item, *id, depth, path, budget)?)
            }
        }
    })
}
fn copy_metadata(
    table: &ItemMetadataTable,
    depth: usize,
    budget: &mut Budget,
) -> Result<ItemMetadataTable> {
    budget.depth(depth)?;
    budget.charge(std::mem::size_of::<ItemMetadataTable>())?;
    let mut out = ItemMetadataTable::default();
    for (key, value) in &table.fields {
        budget.key(key)?;
        let v = copy_value(value, depth + 1, budget)?;
        out.fields.insert(key.clone(), v);
    }
    for (key, value) in &table.indexed {
        budget.charge(64)?;
        out.indexed
            .insert(*key, copy_value(value, depth + 1, budget)?);
    }
    Ok(out)
}
fn copy_value(
    value: &ItemMetadataValue,
    depth: usize,
    budget: &mut Budget,
) -> Result<ItemMetadataValue> {
    budget.depth(depth)?;
    budget.charge(std::mem::size_of::<ItemMetadataValue>())?;
    Ok(match value {
        ItemMetadataValue::Boolean(v) => ItemMetadataValue::Boolean(*v),
        ItemMetadataValue::Number(n) => {
            number(*n)?;
            ItemMetadataValue::Number(*n)
        }
        ItemMetadataValue::Text(s) => {
            budget.charge(s.len())?;
            ItemMetadataValue::Text(s.clone())
        }
        ItemMetadataValue::Array(values) => {
            budget.charge(
                values
                    .len()
                    .checked_mul(std::mem::size_of::<ItemMetadataValue>())
                    .ok_or_else(|| AssemblyError::resource("prior metadata array bytes"))?,
            )?;
            let mut out = Vec::with_capacity(values.len());
            for value in values {
                out.push(copy_value(value, depth + 1, budget)?);
            }
            ItemMetadataValue::Array(out)
        }
        ItemMetadataValue::Table(t) => ItemMetadataValue::Table(copy_metadata(t, depth, budget)?),
        ItemMetadataValue::Callback(_) => {
            return Err(AssemblyError::unsupported(
                "opaque callback cannot be projected as an assembled effect",
            ));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_loading::assembly::value::{Arena, AssemblyLimits, Value};
    #[test]
    fn partial_projection_retains_missing_groups_and_numeric_absence() {
        let mut arena = Arena::new(AssemblyLimits::default());
        let root = arena.new_table().unwrap();
        arena.set_field(root, "new", Value::Boolean(false)).unwrap();
        let item = arena.finish(root).unwrap();
        let mut before = ItemState::default();
        before
            .retained_fields
            .insert("deleted".into(), ItemScalar::Text("old".into()));
        before
            .requirements
            .insert("str".into(), ItemNumber::new(10.));
        before.armour_data = Some(BTreeMap::new());
        let updates = loading_updates(&item, &before).unwrap();
        assert!(updates.requirements.is_none());
        assert!(matches!(updates.armour_data, ArmourDataUpdate::Preserve));
        assert!(matches!(
            updates.state_updates.get("deleted"),
            Some(ItemScalar::Number(ItemNumber::Nil))
        ));
        assert!(updates.assembled.unwrap().shares_storage_with(&item));
        assert!(!item.is_complete());
    }
    #[test]
    fn diagnostic_tree_projection_rejects_cycles_and_amplification() {
        let mut arena = Arena::new(AssemblyLimits::default());
        let root = arena.new_table().unwrap();
        arena.set_field(root, "self", Value::Table(root)).unwrap();
        let item = arena.finish(root).unwrap();
        assert_eq!(
            graph_table(&item, root, 0, &mut BTreeSet::new(), &mut Budget::default())
                .unwrap_err()
                .kind,
            super::super::AssemblyErrorKind::Unsupported
        );
        let mut budget = Budget {
            values: MAX_VALUES,
            bytes: 0,
        };
        assert!(copy_metadata(&ItemMetadataTable::default(), 0, &mut budget).is_err());
    }
}
