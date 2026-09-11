//! Optional immutable raw traversal order and original builtin callback links.
//! These are bounded structural claims; the source observer authenticates them.
use super::*;
use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::fmt;

const MAX_TABLES: usize = 100_000;
const MAX_TABLE_KEYS: usize = 50_000;
const MAX_KEYS: usize = 1_000_000;
const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;
const MAX_LINKS: usize = 256;

/// A missing order is unavailable, including for an empty table. An explicit
/// empty order proves an empty raw inventory. No order is inferred from maps.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceProgramIteration {
    #[serde(deserialize_with = "order_tables")]
    pub table_order: BTreeMap<SourceTableId, Vec<SourceTableKey>>,
    /// Each exact original pairs closure retains this exact original next
    /// function. This semantic relation does not fabricate Lua upvalue records.
    #[serde(deserialize_with = "callback_links")]
    pub pairs_next: BTreeMap<SourceCallbackId, SourceCallbackId>,
}
impl SourceProgramIteration {
    pub fn validate_shape(&self) -> SourceProgramResult<()> {
        self.shape_size().map(|_| ())
    }
    pub(super) fn shape_size(&self) -> SourceProgramResult<(usize, usize)> {
        if self.table_order.len() > MAX_TABLES || self.pairs_next.len() > MAX_LINKS {
            return Err(resource("iteration table/link count bound"));
        }
        let mut budget = Budget::default();
        for order in self.table_order.values() {
            if order.len() > MAX_TABLE_KEYS {
                return Err(resource("iteration table key count bound"));
            }
            let mut seen = BTreeSet::new();
            for key in order {
                budget.key(key)?;
                if !seen.insert(key) {
                    return Err(invalid("duplicate normalized iteration key"));
                }
            }
        }
        Ok((budget.keys, budget.bytes))
    }
}
/// Run on every fresh owner construction, including constructors without context.
/// Definition validation may precede this because it has no access to sidecars.
pub(super) fn validate(
    definitions: &SourceProgramDefinitions,
    classes: Option<&SourceClassDefinitions>,
    context: Option<&SourceProgramContext>,
) -> SourceProgramResult<()> {
    let iteration = context.and_then(|context| context.iteration.as_ref());
    if let Some(iteration) = iteration {
        iteration.validate_shape()?;
        for (pairs, next) in &iteration.pairs_next {
            for (id, operation) in [
                (*pairs, SourceProgramIntrinsic::Pairs),
                (*next, SourceProgramIntrinsic::Next),
            ] {
                let callback =
                    id.0.checked_sub(1)
                        .and_then(|index| definitions.callbacks.get(index as usize))
                        .ok_or_else(|| binding("iteration builtin callback is missing"))?;
                if definitions.intrinsics.get(&id) != Some(&operation)
                    || callback.kind
                        != (SourceCallbackKind::Builtin {
                            symbol: operation.global_path().expect("builtin path").join("."),
                        })
                    || !callback.upvalues.is_empty()
                {
                    return Err(binding(
                        "iteration link differs from exact builtin identity",
                    ));
                }
            }
        }
        let class_tables = classes
            .map(|classes| {
                classes
                    .classes
                    .iter()
                    .map(|class| (class.table, class))
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default();
        for (id, order) in &iteration.table_order {
            let table =
                id.0.checked_sub(1)
                    .and_then(|index| definitions.tables.get(index as usize))
                    .ok_or_else(|| binding("iteration table is not in source definitions"))?;
            if class_tables
                .get(id)
                .is_some_and(|class| !class.unsupported_fields.is_empty())
            {
                return Err(failure(
                    SourceProgramErrorKind::UnsupportedCapability,
                    "iteration order requires a complete class raw inventory",
                ));
            }
            if let Some(coverage) = context.and_then(|context| context.tables.get(id))
                && (coverage.inventory != SourceTableInventory::Complete
                    || !coverage.unavailable.is_empty()
                    || coverage.index_fallback != SourceTableIndexFallback::Nil
                    || coverage.call_fallback != SourceTableCallFallback::NonCallable)
            {
                return Err(failure(
                    SourceProgramErrorKind::UnsupportedCapability,
                    "iteration order requires a complete plain raw table",
                ));
            }
            if table
                .fields
                .values()
                .chain(table.indexed.values())
                .any(|value| matches!(value, SourceValue::Nil))
            {
                return Err(invalid("iteration raw inventory contains a nil value"));
            }
            if order.len() != table.fields.len() + table.indexed.len()
                || order.iter().any(|key| match key {
                    SourceTableKey::Text(key) => !table.fields.contains_key(key),
                    SourceTableKey::Integer(key) => !table.indexed.contains_key(key),
                })
            {
                return Err(invalid(
                    "iteration order is not an exact raw-key permutation",
                ));
            }
        }
    }
    for (id, operation) in &definitions.intrinsics {
        if *operation == SourceProgramIntrinsic::Pairs
            && !iteration.is_some_and(|iteration| iteration.pairs_next.contains_key(id))
        {
            return Err(binding(
                "pairs intrinsic has no exact retained next callback link",
            ));
        }
    }
    Ok(())
}
impl SourceProgramOwner {
    pub fn iteration(&self) -> Option<&SourceProgramIteration> {
        self.context()?.iteration.as_ref()
    }
    /// Owner-local immutable raw order; never use it for a mutable session table.
    pub fn table_iteration_order(&self, table: SourceTableId) -> Option<&[SourceTableKey]> {
        self.iteration()?.table_order.get(&table).map(Vec::as_slice)
    }
    /// Retained callback identity belongs to this owner, not a global name lookup.
    pub fn pairs_next_callback(&self, pairs: SourceCallbackId) -> Option<SourceCallbackId> {
        self.iteration()?.pairs_next.get(&pairs).copied()
    }
}
fn invalid(message: impl Into<String>) -> SourceProgramError {
    failure(SourceProgramErrorKind::InvalidData, message)
}
fn binding(message: impl Into<String>) -> SourceProgramError {
    failure(SourceProgramErrorKind::Binding, message)
}
fn resource(message: impl Into<String>) -> SourceProgramError {
    failure(SourceProgramErrorKind::ResourceLimit, message)
}
#[derive(Default)]
struct Budget {
    keys: usize,
    bytes: usize,
}
impl Budget {
    fn key(&mut self, key: &SourceTableKey) -> SourceProgramResult<()> {
        if self.keys == MAX_KEYS {
            return Err(resource("aggregate iteration key count bound"));
        }
        self.keys += 1;
        match key {
            SourceTableKey::Text(key) => {
                if key.len() > 4096 || key.contains('\0') {
                    return Err(invalid("invalid iteration text key"));
                }
                self.bytes = self
                    .bytes
                    .checked_add(key.len())
                    .ok_or_else(|| resource("iteration key text overflow"))?;
                if self.bytes > MAX_TEXT_BYTES {
                    return Err(resource("iteration key text bound"));
                }
            }
            SourceTableKey::Integer(key) if key.unsigned_abs() > 9_007_199_254_740_991 => {
                return Err(invalid("iteration integer outside exact Lua range"));
            }
            _ => {}
        }
        Ok(())
    }
}
struct OrderSeed<'a>(&'a mut Budget);
impl<'de> DeserializeSeed<'de> for OrderSeed<'_> {
    type Value = Vec<SourceTableKey>;
    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        struct Order<'a>(&'a mut Budget);
        impl<'de> Visitor<'de> for Order<'_> {
            type Value = Vec<SourceTableKey>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("bounded source raw traversal order")
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut order = Vec::new();
                loop {
                    if order.len() == MAX_TABLE_KEYS || self.0.keys == MAX_KEYS {
                        if sequence.next_element::<serde::de::IgnoredAny>()?.is_some() {
                            return Err(serde::de::Error::custom("iteration key count bound"));
                        }
                        break;
                    }
                    let Some(key) = sequence.next_element::<SourceTableKey>()? else {
                        break;
                    };
                    self.0.key(&key).map_err(serde::de::Error::custom)?;
                    order.push(key);
                }
                let seen = order.iter().collect::<BTreeSet<_>>();
                if seen.len() != order.len() {
                    return Err(serde::de::Error::custom(
                        "duplicate normalized iteration key",
                    ));
                }
                Ok(order)
            }
        }
        deserializer.deserialize_seq(Order(self.0))
    }
}
fn order_tables<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<SourceTableId, Vec<SourceTableKey>>, D::Error> {
    struct Tables;
    impl<'de> Visitor<'de> for Tables {
        type Value = BTreeMap<SourceTableId, Vec<SourceTableKey>>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("bounded unique raw traversal table IDs")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut result = BTreeMap::new();
            let mut budget = Budget::default();
            while let Some(id) = map.next_key::<SourceTableId>()? {
                if result.len() == MAX_TABLES {
                    return Err(serde::de::Error::custom("iteration table count bound"));
                }
                if result.contains_key(&id) {
                    return Err(serde::de::Error::custom(
                        "duplicate normalized iteration table ID",
                    ));
                }
                result.insert(id, map.next_value_seed(OrderSeed(&mut budget))?);
            }
            Ok(result)
        }
    }
    deserializer.deserialize_map(Tables)
}
fn callback_links<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<SourceCallbackId, SourceCallbackId>, D::Error> {
    struct Links;
    impl<'de> Visitor<'de> for Links {
        type Value = BTreeMap<SourceCallbackId, SourceCallbackId>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("bounded exact pairs-to-next callback links")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut result = BTreeMap::new();
            while let Some(id) = map.next_key::<SourceCallbackId>()? {
                if result.len() == MAX_LINKS {
                    return Err(serde::de::Error::custom(
                        "iteration callback link count bound",
                    ));
                }
                if result.contains_key(&id) {
                    return Err(serde::de::Error::custom(
                        "duplicate normalized pairs callback ID",
                    ));
                }
                result.insert(id, map.next_value()?);
            }
            Ok(result)
        }
    }
    deserializer.deserialize_map(Links)
}
