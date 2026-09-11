//! Immutable environment and partial-table observations, separate from legacy graphs.
//!
//! These are structural source claims, not proof of an original runtime capture.
//! The importing domain authenticates them and rejects all unrepresented
//! metamethod behavior except explicitly unavailable `__index`/`__call` fallbacks.
use super::*;
use serde::de::{MapAccess, SeqAccess, Visitor};
use std::{fmt, marker::PhantomData};

pub const SOURCE_PROGRAM_CONTEXT_SCHEMA_VERSION: u32 = 1;
const MAX_TABLES: usize = 100_000;
const MAX_TABLE_KEYS: usize = 50_000;
const MAX_KEYS: usize = 1_000_000;
const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;
const MAX_JSON_BYTES: usize = 16 * 1024 * 1024;

/// Canonical Lua keys supported by the source graph. Text "1" and integer 1
/// remain distinct; integers must be exactly representable by the Lua number.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SourceTableKey {
    Text(String),
    Integer(i64),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTableInventory {
    /// The complete raw key inventory is represented or explicitly unavailable.
    /// An unlisted key is proven raw-absent, including other Lua key types.
    Complete,
    /// Unlisted keys have unknown presence, even when outside SourceTableKey.
    Selective,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTableIndexFallback {
    /// Source observation proves there is no effective __index fallback.
    Nil,
    /// The raw view is known but its __index behavior is unrepresented.
    /// Ordinary absent reads are unavailable; raw absent reads still yield nil.
    Unavailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTableCallFallback {
    /// Source observation proves the table has no effective __call behavior.
    NonCallable,
    /// Invocation needs unrepresented source __call behavior. Lookup still
    /// returns the table; reject only after evaluating every call argument.
    Unavailable,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceTableCoverage {
    pub inventory: SourceTableInventory,
    #[serde(deserialize_with = "bounded_keys")]
    pub known_absent: BTreeSet<SourceTableKey>,
    /// Keys observed to exist whose values cannot be represented in the graph.
    #[serde(deserialize_with = "bounded_keys")]
    pub unavailable: BTreeSet<SourceTableKey>,
    pub index_fallback: SourceTableIndexFallback,
    pub call_fallback: SourceTableCallFallback,
}
impl SourceTableCoverage {
    /// Validates metadata without constructing or cloning a definition graph.
    /// Runtime transports must additionally reject conflicts with their entries.
    pub fn validate_shape(&self) -> SourceProgramResult<()> {
        self.shape_size().map(|_| ())
    }
    fn shape_size(&self) -> SourceProgramResult<(usize, usize)> {
        let count = self
            .known_absent
            .len()
            .checked_add(self.unavailable.len())
            .ok_or_else(|| resource("coverage key count overflow"))?;
        if count > MAX_TABLE_KEYS {
            return Err(resource("coverage table key count bound"));
        }
        let mut bytes = 0usize;
        for key in self.known_absent.iter().chain(&self.unavailable) {
            match key {
                SourceTableKey::Text(value) => {
                    if value.len() > 4096 || value.contains('\0') {
                        return Err(invalid("invalid coverage text key"));
                    }
                    bytes = bytes
                        .checked_add(value.len())
                        .ok_or_else(|| resource("coverage key text overflow"))?;
                    if bytes > MAX_TEXT_BYTES {
                        return Err(resource("coverage key text bound"));
                    }
                }
                SourceTableKey::Integer(value) => {
                    if value.unsigned_abs() > 9_007_199_254_740_991 {
                        return Err(invalid("coverage integer outside exact Lua range"));
                    }
                }
            }
        }
        if !self.known_absent.is_disjoint(&self.unavailable) {
            return Err(invalid("coverage key is both absent and unavailable"));
        }
        Ok((count, bytes))
    }
    /// Validates the raw projection only. This never turns unknown __index
    /// behavior into ordinary nil, grants mutation, or admits numerical effects.
    pub fn validate_table(&self, table: &SourceTable) -> SourceProgramResult<()> {
        let (count, _) = self.shape_size()?;
        let total = count
            .checked_add(table.fields.len())
            .and_then(|count| count.checked_add(table.indexed.len()))
            .ok_or_else(|| resource("coverage and graph key count overflow"))?;
        if total > MAX_TABLE_KEYS {
            return Err(resource("coverage and graph table key count bound"));
        }
        for key in self.known_absent.iter().chain(&self.unavailable) {
            let represented = match key {
                SourceTableKey::Text(value) => table.fields.contains_key(value),
                SourceTableKey::Integer(value) => table.indexed.contains_key(value),
            };
            if represented {
                return Err(invalid(
                    "coverage key conflicts with represented graph value",
                ));
            }
        }
        Ok(())
    }
}
/// Owner-local sidecar; it does not alter SourceProgramDefinitions or parser JSON.
/// Tables without an entry retain the existing complete plain-graph contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceProgramContext {
    pub schema_version: u32,
    /// Named root identifying the actual original environment projection. It is
    /// required on the wire (null means no environment); a name is not evidence.
    #[serde(deserialize_with = "required_environment")]
    pub environment: Option<SourceProgramRootId>,
    #[serde(deserialize_with = "bounded_tables")]
    pub tables: BTreeMap<SourceTableId, SourceTableCoverage>,
}
impl Default for SourceProgramContext {
    fn default() -> Self {
        Self {
            schema_version: SOURCE_PROGRAM_CONTEXT_SCHEMA_VERSION,
            environment: None,
            tables: BTreeMap::new(),
        }
    }
}
impl SourceProgramContext {
    pub fn from_bytes(
        bytes: &[u8],
        definitions: &SourceProgramDefinitions,
        classes: Option<&SourceClassDefinitions>,
    ) -> SourceProgramResult<Self> {
        if bytes.len() > MAX_JSON_BYTES {
            return Err(resource("source context JSON byte bound"));
        }
        let context: Self = serde_json::from_slice(bytes)
            .map_err(|error| invalid(format!("source context JSON: {error}")))?;
        context.validate(definitions, classes)?;
        Ok(context)
    }
    /// Checks all counts before building owner storage. The caller owns source
    /// authentication; definitions/classes have their separate graph validation.
    pub fn validate(
        &self,
        definitions: &SourceProgramDefinitions,
        classes: Option<&SourceClassDefinitions>,
    ) -> SourceProgramResult<()> {
        if self.schema_version != SOURCE_PROGRAM_CONTEXT_SCHEMA_VERSION {
            return Err(invalid("unsupported source context schema"));
        }
        if self.tables.len() > MAX_TABLES {
            return Err(resource("source context table count bound"));
        }
        if let Some(root) = self.environment
            && root
                .0
                .checked_sub(1)
                .and_then(|index| definitions.roots.get(index as usize))
                .is_none()
        {
            return Err(failure(
                SourceProgramErrorKind::Binding,
                "environment root is not bound to source definitions",
            ));
        }
        let mut count = 0usize;
        let mut bytes = 0usize;
        for (id, coverage) in &self.tables {
            let table =
                id.0.checked_sub(1)
                    .and_then(|index| definitions.tables.get(index as usize))
                    .ok_or_else(|| invalid("coverage table is not in source definitions"))?;
            if classes.is_some_and(|classes| classes.classes.iter().any(|class| class.table == *id))
            {
                return Err(invalid(
                    "generic coverage overlaps a source class projection",
                ));
            }
            let (table_count, table_bytes) = coverage.shape_size()?;
            count = count
                .checked_add(table_count)
                .ok_or_else(|| resource("aggregate coverage key count overflow"))?;
            bytes = bytes
                .checked_add(table_bytes)
                .ok_or_else(|| resource("aggregate coverage key text overflow"))?;
            if count > MAX_KEYS || bytes > MAX_TEXT_BYTES {
                return Err(resource("aggregate coverage resource bound"));
            }
            coverage.validate_table(table)?;
        }
        Ok(())
    }
}
impl SourceProgramOwner {
    /// Creates a fresh immutable identity. Sidecars cannot be attached to an
    /// existing owner, which would invalidate its already-bound handles/programs.
    pub fn new_with_context(
        data: SourceProgramDefinitions,
        classes: Option<SourceClassDefinitions>,
        context: SourceProgramContext,
    ) -> SourceProgramResult<Self> {
        data.validate()?;
        closures::validate_declarations(&data, None)?;
        if let Some(classes) = &classes {
            classes.validate(&data)?;
        }
        context.validate(&data, classes.as_ref())?;
        Ok(Self(OwnerStorage::Standalone {
            definitions: Arc::new(data),
            classes: classes.map(Arc::new),
            context: Some(Arc::new(context)),
            closures: None,
        }))
    }
    pub fn context(&self) -> Option<&SourceProgramContext> {
        match &self.0 {
            OwnerStorage::Standalone { context, .. } => context.as_deref(),
            OwnerStorage::Parser(_) => None,
        }
    }
    pub fn table_coverage(&self, table: SourceTableId) -> Option<&SourceTableCoverage> {
        self.context()?.tables.get(&table)
    }
    pub fn environment_root(&self) -> Option<SourceProgramRootId> {
        self.context()?.environment
    }
    /// Retains owner identity; it cannot be used to rebind another owner's graph.
    pub fn bind_environment(&self) -> SourceProgramResult<Option<SourceProgramRootHandle>> {
        self.environment_root()
            .map(|root| self.bind_root(SourceProgramDefinitionRoot::Named(root)))
            .transpose()
    }
}
fn invalid(message: impl Into<String>) -> SourceProgramError {
    failure(SourceProgramErrorKind::InvalidData, message)
}
fn resource(message: impl Into<String>) -> SourceProgramError {
    failure(SourceProgramErrorKind::ResourceLimit, message)
}
fn required_environment<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<SourceProgramRootId>, D::Error> {
    Option::<SourceProgramRootId>::deserialize(deserializer)
}
fn bounded_keys<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeSet<SourceTableKey>, D::Error> {
    struct Keys;
    impl<'de> Visitor<'de> for Keys {
        type Value = BTreeSet<SourceTableKey>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("bounded unique source table keys")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
            let mut result = BTreeSet::new();
            loop {
                // Do not allocate another key beyond the admitted collection.
                if result.len() == MAX_TABLE_KEYS {
                    if sequence.next_element::<serde::de::IgnoredAny>()?.is_some() {
                        return Err(serde::de::Error::custom("coverage table key count bound"));
                    }
                    break;
                }
                let Some(key) = sequence.next_element::<SourceTableKey>()? else {
                    break;
                };
                if !result.insert(key) {
                    return Err(serde::de::Error::custom(
                        "duplicate normalized coverage key",
                    ));
                }
            }
            Ok(result)
        }
    }
    deserializer.deserialize_seq(Keys)
}
fn bounded_tables<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<SourceTableId, SourceTableCoverage>, D::Error> {
    struct Tables(PhantomData<()>);
    impl<'de> Visitor<'de> for Tables {
        type Value = BTreeMap<SourceTableId, SourceTableCoverage>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("bounded unique coverage table IDs")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut result = BTreeMap::new();
            while let Some(id) = map.next_key::<SourceTableId>()? {
                if result.len() == MAX_TABLES {
                    return Err(serde::de::Error::custom("source context table count bound"));
                }
                if result.contains_key(&id) {
                    return Err(serde::de::Error::custom(
                        "duplicate normalized coverage table ID",
                    ));
                }
                result.insert(id, map.next_value()?);
            }
            Ok(result)
        }
    }
    deserializer.deserialize_map(Tables(PhantomData))
}
