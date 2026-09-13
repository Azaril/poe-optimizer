//! Explicit source-fed pre-activation inputs; no source poststate becomes native state.
use mlua::{Function, MultiValue, Table as LuaTable, Value as LuaValue};
use poe_optimizer_data::{
    game_data::GameDataSnapshot,
    item_loading::{ItemMetadataTable as Metadata, ItemMetadataValue as Field},
};
use poe_optimizer_import::{
    item_loading::{
        DependencyResult, ItemLoadProvider, ItemNumber, ParseOutcome, ParseRequest,
        assembly::AssemblyError,
    },
    item_sets::{ItemActivationContext, ItemActivationRune, ItemSetLimits, RuneChoiceCatalog},
    item_slot_validity::{
        SlotValidityContext, SlotValidityLimits, SlotValidityProgram, SlotValidityRequest, Value,
    },
};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
type Result<T> = std::result::Result<T, AssemblyError>;
/// Bounded finite tree ingress. Shared source rows are not claimed reconstructed;
/// the consumer operations here inspect fields/truthiness, not input identity.
pub fn metadata(table: &LuaTable) -> Result<Metadata> {
    metadata_fields(table, None)
}
/// Copy only the fields this context reads. The complete source input graph is
/// retained in the receipt; source itemSets/active sets never become native state.
pub fn activation_metadata(table: &LuaTable) -> Result<Metadata> {
    metadata_fields(
        table,
        Some(&[
            "treeNodes",
            "specNodes",
            "items",
            "calculation_environment_present",
            "colors",
            "nodeJewels",
        ]),
    )
}
fn metadata_fields(table: &LuaTable, fields: Option<&[&str]>) -> Result<Metadata> {
    struct Budget {
        rows: usize,
        bytes: usize,
        active: BTreeSet<usize>,
    }
    fn text(s: mlua::LuaString, b: &mut Budget) -> Result<String> {
        let size = s.as_bytes().len();
        b.bytes = b
            .bytes
            .checked_add(size)
            .ok_or_else(|| AssemblyError::resource("context byte overflow"))?;
        if b.bytes > 16 * 1024 * 1024 {
            return Err(AssemblyError::resource("context text bound"));
        }
        let s = s
            .to_str()
            .map_err(|_| AssemblyError::unsupported("non-UTF8 finite context"))?
            .to_string();
        Ok(s)
    }
    fn value(v: LuaValue, b: &mut Budget, depth: usize) -> Result<Field> {
        match v {
            LuaValue::Boolean(v) => Ok(Field::Boolean(v)),
            LuaValue::Integer(v) => Ok(Field::Number(v as f64)),
            LuaValue::Number(v) if v.is_finite() => Ok(Field::Number(v)),
            LuaValue::String(v) => text(v, b).map(Field::Text),
            LuaValue::Table(t) => table_value(&t, b, depth + 1).map(Field::Table),
            _ => Err(AssemblyError::unsupported(format!(
                "unrepresented finite context {}",
                v.type_name()
            ))),
        }
    }
    fn table_value(t: &LuaTable, b: &mut Budget, depth: usize) -> Result<Metadata> {
        if depth > 64 || t.metatable().is_some() {
            return Err(AssemblyError::unsupported("context depth/metatable"));
        }
        let pointer = t.to_pointer() as usize;
        if !b.active.insert(pointer) {
            return Err(AssemblyError::unsupported("cyclic tree ingress"));
        }
        let mut out = Metadata::default();
        for row in t.clone().pairs::<LuaValue, LuaValue>() {
            b.rows += 1;
            if b.rows > 262144 {
                return Err(AssemblyError::resource("context row bound"));
            }
            let (k, v) = row.map_err(|e| AssemblyError::resource(e.to_string()))?;
            let v = value(v, b, depth)?;
            match k {
                LuaValue::String(k) => {
                    out.fields.insert(text(k, b)?, v);
                }
                LuaValue::Integer(k) => {
                    out.indexed.insert(k, v);
                }
                LuaValue::Number(k)
                    if k.is_finite()
                        && k.fract() == 0.0
                        && k >= i64::MIN as f64
                        && k < i64::MAX as f64 =>
                {
                    out.indexed.insert(k as i64, v);
                }
                _ => return Err(AssemblyError::unsupported("unrepresented context key")),
            }
        }
        assert!(b.active.remove(&pointer));
        Ok(out)
    }
    let mut budget = Budget {
        rows: 0,
        bytes: 0,
        active: BTreeSet::new(),
    };
    let Some(fields) = fields else {
        return table_value(table, &mut budget, 0);
    };
    if table.metatable().is_some() {
        return Err(AssemblyError::unsupported("context root metatable"));
    }
    let mut out = Metadata::default();
    for &name in fields {
        let input: LuaValue = table
            .raw_get(name)
            .map_err(|e| AssemblyError::resource(e.to_string()))?;
        if !matches!(input, LuaValue::Nil) {
            budget.rows += 1;
            budget.bytes += name.len();
            out.fields
                .insert(name.to_owned(), value(input, &mut budget, 0)?);
        }
    }
    Ok(out)
}
fn raw<'a>(t: &'a Metadata, key: &str) -> Result<Value<'a>> {
    t.fields.get(key).map_or(Ok(Value::Nil), Value::metadata)
}
fn source_error(mut error: &mlua::Error) -> bool {
    loop {
        match error {
            mlua::Error::RuntimeError(_) => return true,
            mlua::Error::CallbackError { cause, .. } => error = cause,
            _ => return false,
        }
    }
}
struct Parser {
    original: Function,
    calls: Vec<Json>,
    observed: BTreeMap<(String, bool), (usize, [u8; 32])>,
    retained_bytes: usize,
}
impl Parser {
    fn charge(&mut self, bytes: usize) {
        self.retained_bytes = self
            .retained_bytes
            .checked_add(bytes)
            .expect("parser receipt byte overflow");
        assert!(
            self.retained_bytes <= 32 * 1024 * 1024,
            "parser aggregate retained receipt bound"
        );
    }
}
/// Stream the complete finite outcome into a digest, without a JSON value clone.
fn outcome_digest(outcome: &ParseOutcome) -> [u8; 32] {
    struct Writer {
        hash: Sha256,
        bytes: usize,
    }
    impl std::io::Write for Writer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.bytes = self
                .bytes
                .checked_add(bytes.len())
                .ok_or_else(|| std::io::Error::other("outcome size overflow"))?;
            if self.bytes > 128 * 1024 * 1024 {
                return Err(std::io::Error::other("outcome serialization bound"));
            }
            self.hash.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Writer {
        hash: Sha256::new(),
        bytes: 0,
    };
    serde_json::to_writer(&mut writer, outcome).unwrap();
    writer.hash.finalize().into()
}
impl ItemLoadProvider for Parser {
    fn parse_modifier(&mut self, request: &ParseRequest) -> DependencyResult<ParseOutcome> {
        assert!(self.calls.len() < 32768, "source parser call bound");
        assert!(
            request.text.len() <= 262144,
            "source parser request text bound"
        );
        // Charge both retained request copies and bounded fixed record overhead
        // before inserting either the call receipt or repeated-input map key.
        self.charge(
            request
                .text
                .len()
                .checked_mul(2)
                .and_then(|n| n.checked_add(2048))
                .unwrap(),
        );
        let result = self
            .original
            .call::<MultiValue>((request.text.as_str(), request.combined));
        let values = match result {
            Ok(v) => v,
            Err(e) => {
                let message = e.to_string();
                assert!(message.len() <= 65536, "source error text bound");
                self.charge(message.len());
                self.calls.push(json!({"text":request.text,"combined":request.combined,"source_error":source_error(&e),"error":message}));
                assert!(
                    source_error(&e),
                    "host/resource failure is not a source outcome: {e}"
                );
                return DependencyResult::SourceError(e.to_string());
            }
        };
        // Keep the observed pack arity. The existing ParseOutcome interface consumes
        // only modifiers/extra and cannot authorize any broader arity contract.
        let arity = values.len();
        assert!(arity <= 2, "unexpected original parser output arity");
        let modifiers = match values.front().cloned().unwrap_or(LuaValue::Nil) {
            LuaValue::Nil => None,
            LuaValue::Table(t) => {
                let n = t.raw_len();
                assert!(n <= 4096);
                let mut indices = BTreeSet::new();
                for row in t.clone().pairs::<LuaValue, LuaValue>() {
                    let (k, _) = row.unwrap();
                    let nkey = match k {
                        LuaValue::Integer(k) => k as f64,
                        LuaValue::Number(k) => k,
                        _ => panic!("parser list key"),
                    };
                    assert!(
                        nkey.is_finite() && nkey.fract() == 0.0 && nkey >= 1.0 && nkey <= n as f64
                    );
                    assert!(indices.insert(nkey as usize));
                }
                assert_eq!(indices.len(), n);
                // Traverse the actual outer pack once: all modifier rows share
                // one metadata row/text/depth budget, including repeated aliases.
                let pack = match metadata(&t) {
                    Ok(pack) => pack,
                    Err(e) => {
                        self.calls.push(json!({"text":request.text,"combined":request.combined,"return_pack_arity":arity,"finite_ingress_error":e.to_string(),"kind":format!("{:?}",e.kind)}));
                        let message = format!("source parser finite ingress: {e}");
                        return match e.kind {
  poe_optimizer_import::item_loading::assembly::AssemblyErrorKind::Resource => DependencyResult::ResourceError(message),
  poe_optimizer_import::item_loading::assembly::AssemblyErrorKind::Unsupported => DependencyResult::Unavailable(message),
  poe_optimizer_import::item_loading::assembly::AssemblyErrorKind::Source => panic!("metadata projection cannot manufacture a source execution error"),
 };
                    }
                };
                assert!(pack.fields.is_empty());
                assert_eq!(pack.indexed.len(), n);
                let rows = pack
                    .indexed
                    .into_values()
                    .map(|v| match v {
                        Field::Table(row) => row,
                        _ => panic!("modifier row is not a table"),
                    })
                    .collect();
                Some(rows)
            }
            other => panic!("original parser first result {}", other.type_name()),
        };
        let extra = match values.get(1).cloned().unwrap_or(LuaValue::Nil) {
            LuaValue::Nil => None,
            LuaValue::String(s) => {
                assert!(s.as_bytes().len() <= 65536, "parser extra text bound");
                Some(s.to_str().unwrap().to_string())
            }
            other => panic!("original parser extra {}", other.type_name()),
        };
        let result = ParseOutcome { modifiers, extra };
        let digest = outcome_digest(&result);
        if let Some(prior) = self
            .observed
            .insert((request.text.clone(), request.combined), (arity, digest))
        {
            assert_eq!(
                prior,
                (arity, digest),
                "repeated source parser finite outcome/arity digest changed within preparation"
            );
        }
        let digest = digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        self.calls.push(json!({"text":request.text,"combined":request.combined,"return_pack_arity":arity,"finite_outcome_sha256":digest,"modifiers_present":result.modifiers.is_some(),"modifier_count":result.modifiers.as_ref().map(Vec::len),"extra_present":result.extra.is_some(),"scope":"complete finite tree outcome digest; source aliases are not reconstructed"}));
        DependencyResult::Available(result)
    }
}
struct Validity<'a> {
    root: &'a Metadata,
    active: Value<'a>,
    calcs: bool,
}
impl<'a> SlotValidityContext<'a> for Validity<'a> {
    fn active_item_set(&mut self) -> Result<Value<'a>> {
        Ok(self.active)
    }
    fn tree_node(&mut self, key: Value<'_>) -> Result<Value<'a>> {
        raw(self.root, "treeNodes")?.index(key)
    }
    fn effective_node(&mut self, key: Value<'_>) -> Result<Value<'a>> {
        raw(self.root, "specNodes")?.index(key)
    }
    fn inventory_item(&mut self, key: Value<'_>) -> Result<Value<'a>> {
        raw(self.root, "items")?.index(key)
    }
    fn has_calculation_environment(&mut self) -> Result<bool> {
        Ok(self.calcs)
    }
    fn flag(&mut self, _name: &str) -> Result<Value<'a>> {
        Err(AssemblyError::unsupported(
            "actual actor Flag context was not observed",
        ))
    }
}
pub struct Context {
    root: Metadata,
    ids: Vec<f64>,
    program: SlotValidityProgram,
    runes: DependencyResult<RuneChoiceCatalog>,
    steps: u64,
    bytes: usize,
    pub node_jewels: BTreeMap<i64, f64>,
    reservation: BTreeMap<i64, f64>,
    pub parser_calls: Vec<Json>,
    pub rune_preparation: Json,
    pub validity_calls: Vec<Json>,
}
impl Context {
    pub fn new(root: Metadata, data: &GameDataSnapshot, parser: Function) -> Result<Self> {
        let ids = match root.fields.get("items") {
            Some(Field::Table(t)) if t.fields.is_empty() => {
                t.indexed.keys().map(|n| *n as f64).collect()
            }
            _ => {
                return Err(AssemblyError::unsupported(
                    "finite numeric inventory winners",
                ));
            }
        };
        let program = SlotValidityProgram::new(
            &data.item_assembly().policy().slot_validity,
            SlotValidityLimits::default(),
        )?;
        let mut provider = Parser {
            original: parser,
            calls: Vec::new(),
            observed: BTreeMap::new(),
            retained_bytes: 0,
        };
        let prepared = RuneChoiceCatalog::prepare(
            data.item_loading(),
            &data.item_assembly().policy().inventory,
            &mut provider,
            ItemSetLimits::default(),
        );
        let outcome = match &prepared.result {
            DependencyResult::Available(_) => json!({"available":true}),
            DependencyResult::Unavailable(m) => {
                json!({"available":false,"kind":"unsupported","message":m})
            }
            DependencyResult::SourceError(m) => {
                json!({"available":false,"kind":"source","message":m})
            }
            DependencyResult::ResourceError(m) => {
                json!({"available":false,"kind":"resource","message":m})
            }
        };
        let mut node_jewels = BTreeMap::new();
        let Some(Field::Table(nodes)) = root.fields.get("nodeJewels") else {
            return Err(AssemblyError::unsupported("observed node write context"));
        };
        assert!(nodes.fields.is_empty());
        for (id, v) in &nodes.indexed {
            let Field::Number(v) = v else {
                return Err(AssemblyError::unsupported("finite selected node value"));
            };
            node_jewels.insert(*id, *v);
        }
        Ok(Self {
            root,
            ids,
            program,
            runes: prepared.result,
            steps: prepared
                .usage
                .steps
                .saturating_add(prepared.usage.pattern_steps),
            bytes: prepared.usage.bytes,
            node_jewels,
            reservation: BTreeMap::new(),
            parser_calls: provider.calls,
            rune_preparation: json!({"outcome":outcome,"usage":prepared.usage,"scope":"fresh native prepared rune rows using retained original parser calls after source import; stable repeated finite per-line outcomes, not original initialization/cache history or source aliases"}),
            validity_calls: Vec::new(),
        })
    }
    fn item(&self, id: f64) -> Result<Value<'_>> {
        raw(&self.root, "items")?.index(Value::Number(id))
    }
}
impl ItemActivationContext for Context {
    fn inventory_ids(&self) -> &[f64] {
        &self.ids
    }
    fn take_validation_steps(&mut self) -> u64 {
        std::mem::take(&mut self.steps)
    }
    fn take_preparation_bytes(&mut self) -> usize {
        std::mem::take(&mut self.bytes)
    }
    fn valid_for_slot(&mut self, id: f64, slot: &str, active: Value<'_>) -> Result<bool> {
        let calcs = raw(&self.root, "calculation_environment_present")?.truthy();
        let mut context = Validity {
            root: &self.root,
            active,
            calcs,
        };
        let (result, steps) = self.program.check_with_usage(
            SlotValidityRequest {
                item: self.item(id)?,
                slot_name: slot,
                item_set: active,
                flag_state: Value::Nil,
            },
            &mut context,
        );
        let result = result.map(|value| value.truthy());
        self.steps = self.steps.saturating_add(steps);
        assert!(self.validity_calls.len() < 131072);
        self.validity_calls.push(json!({"id":id,"slot":slot,"truthy":result.as_ref().ok(),"error":result.as_ref().err().map(ToString::to_string)}));
        result
    }
    fn item_label(&mut self, id: f64) -> Result<String> {
        let item = self.item(id)?;
        let Value::Text(rarity) = item.field("rarity")? else {
            return Err(AssemblyError::source("missing rarity color key"));
        };
        let Value::Text(color) = raw(&self.root, "colors")?.field(rarity)? else {
            return Err(AssemblyError::source("missing rarity color"));
        };
        let Value::Text(name) = item.field("name")? else {
            return Err(AssemblyError::source("non-string item name"));
        };
        Ok(format!("{color}{name}"))
    }
    fn jewel_socket_count(&mut self, id: f64) -> Result<f64> {
        match self.item(id)?.field("jewelSocketCount")? {
            Value::Nil | Value::Boolean(false) => Ok(0.0),
            Value::Number(v) => Ok(v),
            _ => Err(AssemblyError::source("nonnumeric jewel socket count")),
        }
    }
    fn selection_dependencies(&mut self, slot: &str) -> Result<Vec<String>> {
        let (result, steps) = self.program.selection_dependencies_with_usage(slot);
        self.steps = self.steps.saturating_add(steps);
        result
    }
    fn initial_rune(&mut self, slot: &str) -> DependencyResult<ItemActivationRune> {
        match &self.runes {
            DependencyResult::Available(r) => r.initial(slot),
            DependencyResult::Unavailable(m) => DependencyResult::Unavailable(m.clone()),
            DependencyResult::SourceError(m) => DependencyResult::SourceError(m.clone()),
            DependencyResult::ResourceError(m) => DependencyResult::ResourceError(m.clone()),
        }
    }
    fn select_rune(
        &mut self,
        slot: &str,
        request: Value<'_>,
        previous: &ItemActivationRune,
    ) -> DependencyResult<ItemActivationRune> {
        match &self.runes {
            DependencyResult::Available(r) => r.select(slot, request, previous),
            DependencyResult::Unavailable(m) => DependencyResult::Unavailable(m.clone()),
            DependencyResult::SourceError(m) => DependencyResult::SourceError(m.clone()),
            DependencyResult::ResourceError(m) => DependencyResult::ResourceError(m.clone()),
        }
    }
    fn node_writes_available(&mut self, nodes: &[(f64, f64)]) -> Result<bool> {
        let mut reserved = BTreeMap::new();
        for (key, value) in nodes {
            if !key.is_finite()
                || key.fract() != 0.0
                || *key < i64::MIN as f64
                || *key >= i64::MAX as f64
                || !value.is_finite()
            {
                return Ok(false);
            }
            if reserved
                .insert(*key as i64, *value)
                .is_some_and(|old| old != *value)
            {
                return Ok(false);
            }
        }
        self.reservation = reserved;
        Ok(true)
    }
    fn set_node_selection(&mut self, node: f64, new: f64, old: ItemNumber) -> DependencyResult<()> {
        if old.value() != Some(new) || self.reservation.get(&(node as i64)) != Some(&new) {
            return DependencyResult::Unavailable(
                "source-fed node write needs a represented cluster transition".into(),
            );
        }
        self.node_jewels.insert(node as i64, new);
        DependencyResult::Available(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn activation_ingress_keeps_its_read_set_without_replacing_native_sets() {
        let lua = mlua::Lua::new();
        let root = lua.create_table().unwrap();
        let sets = lua.create_table().unwrap();
        sets.raw_set(1.5, true).unwrap();
        root.raw_set("itemSets", sets.clone()).unwrap();
        root.raw_set("calculation_environment_present", false)
            .unwrap();
        assert!(metadata(&root).is_err());
        let admitted = activation_metadata(&root).unwrap();
        assert_eq!(admitted.fields.len(), 1);
        assert_eq!(
            admitted.fields["calculation_environment_present"],
            Field::Boolean(false)
        );
        // The same unsupported key on an actually consumed inventory stays an
        // explicit ingress failure; nothing is coerced to an integer or dropped.
        root.raw_set("items", sets).unwrap();
        assert!(activation_metadata(&root).is_err());
    }
    #[test]
    fn repeated_modifier_aliases_share_the_whole_pack_budget() {
        let lua = mlua::Lua::new();
        let pack = lua.create_table().unwrap();
        let row = lua.create_table().unwrap();
        row.raw_set("text", lua.create_string("x".repeat(1024 * 1024)).unwrap())
            .unwrap();
        for i in 1..=17 {
            pack.raw_set(i, row.clone()).unwrap();
        }
        assert_eq!(
            metadata(&pack).unwrap_err().kind,
            poe_optimizer_import::item_loading::assembly::AssemblyErrorKind::Resource
        );
        // The same row by itself is representable; failure comes from cumulative
        // finite-tree expansion, not alias rejection or a per-string workaround.
        assert!(metadata(&row).is_ok());
    }
}
