//! Authenticated offline acquisition of the finite socketed-augment data table.
//! Only ModRunes.lua runs, with no source loader, UI, modifier parser or globals.
//! The result carries unconverted descriptions, never executable rule coverage.
use crate::source;
use mlua::{HookTriggers, Lua, LuaOptions, StdLib, Table, Value, VmState};
use poe_optimizer_import::{
    owned_augments::*,
    owned_mapping::{ExternalSourceSystem, SourceFilePin, SourcePin},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

pub const AUGMENT_SOURCE_PATH: &str = "src/Data/ModRunes.lua";
#[derive(Clone, Copy, Debug)]
pub struct AugmentExportLimits {
    pub catalog: AugmentCatalogLimits,
    pub max_source_bytes: usize,
    pub max_lua_bytes: usize,
    pub max_instructions: usize,
    pub max_source_entries: usize,
}
impl Default for AugmentExportLimits {
    fn default() -> Self {
        Self {
            catalog: Default::default(),
            max_source_bytes: 2 * 1024 * 1024,
            max_lua_bytes: 32 * 1024 * 1024,
            max_instructions: 1_000_000,
            max_source_entries: 131_072,
        }
    }
}
impl AugmentExportLimits {
    fn validate(self) -> Result<()> {
        self.catalog.validate()?;
        let max = Self::default();
        for (label, actual, ceiling) in [
            ("source bytes", self.max_source_bytes, max.max_source_bytes),
            ("Lua bytes", self.max_lua_bytes, max.max_lua_bytes),
            ("instructions", self.max_instructions, max.max_instructions),
            (
                "source entries",
                self.max_source_entries,
                max.max_source_entries,
            ),
        ] {
            if actual == 0 || actual > ceiling {
                return Err(invalid(format!("invalid {label} limit")));
            }
        }
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
pub enum AugmentExportError {
    #[error("owned augment export: {0}")]
    Invalid(String),
    #[error(transparent)]
    Source(#[from] source::SourceError),
    #[error(transparent)]
    Lua(#[from] mlua::Error),
    #[error(transparent)]
    Catalog(#[from] AugmentCatalogError),
}
type Result<T> = std::result::Result<T, AugmentExportError>;
fn invalid(message: impl Into<String>) -> AugmentExportError {
    AugmentExportError::Invalid(message.into())
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn extractor_fingerprint() -> String {
    let mut digest = Sha256::new();
    digest.update(b"owned-augment-acquisition-extractor-v1\0");
    for (name, text) in [
        ("pob/owned_augments.rs", include_str!("owned_augments.rs")),
        (
            "import/owned_augments.rs",
            include_str!("../../poe-optimizer-import/src/owned_augments.rs"),
        ),
    ] {
        let normalized = text.replace("\r\n", "\n");
        digest.update((name.len() as u64).to_le_bytes());
        digest.update(name.as_bytes());
        digest.update((normalized.len() as u64).to_le_bytes());
        digest.update(normalized.as_bytes());
    }
    format!("{:x}", digest.finalize())
}
/// Readable evidence is not itself an authentication token.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AugmentExportEvidence {
    pub schema_version: u32,
    pub upstream_revision: String,
    pub source_manifest_sha256: String,
    pub extractor_sha256: String,
    pub acquisition_source_sha256: String,
    pub normalized_source_sha256: String,
    pub catalog_sha256: String,
    pub counts: AugmentCatalogCounts,
}
#[derive(Debug)]
pub struct AuthenticatedAugmentExport {
    acquisition: AcquiredAugmentCatalog,
    source_bytes: Vec<u8>,
    evidence: AugmentExportEvidence,
    limits: AugmentCatalogLimits,
}
impl AuthenticatedAugmentExport {
    pub fn catalog(&self) -> &AugmentCatalog {
        self.acquisition.catalog()
    }
    pub fn catalog_bytes(&self) -> &[u8] {
        self.acquisition.bytes()
    }
    pub fn source_bytes(&self) -> &[u8] {
        &self.source_bytes
    }
    pub fn evidence(&self) -> &AugmentExportEvidence {
        &self.evidence
    }
    pub fn validate_catalog(&self, candidate: &AugmentCatalog) -> Result<()> {
        // Canonical bytes preserve -0.0 and ordered lines, unlike floating PartialEq.
        let bytes = encode_owned_augments(candidate, self.limits)?;
        if bytes != self.catalog_bytes() {
            return Err(invalid("catalog differs from authenticated acquisition"));
        }
        Ok(())
    }
}

/// Authenticate the complete pinned inventory, then read and execute the exact
/// independently verified normalized ModRunes.lua bytes. Private acquisition
/// bytes are the normalized script actually executed, not unchecked input paths.
pub fn export_owned_augments(
    root: &Path,
    limits: AugmentExportLimits,
) -> Result<AuthenticatedAugmentExport> {
    limits.validate()?;
    source::verify(root)?;
    let text = source::read_verified_text(root, AUGMENT_SOURCE_PATH)?;
    let pin = SourcePin {
        system: ExternalSourceSystem::PathOfBuilding2,
        revision: source::UPSTREAM_REVISION.into(),
        files: vec![SourceFilePin {
            path: AUGMENT_SOURCE_PATH.into(),
            sha256: source::expected_file_sha256(AUGMENT_SOURCE_PATH)?,
        }],
    };
    export_pinned_bytes(text.as_bytes(), &pin, limits)
}
/// Full caller bytes and pin must independently match the embedded reviewed
/// source manifest before execution. Only CRLF normalization is permitted.
fn export_pinned_bytes(
    bytes: &[u8],
    pin: &SourcePin,
    limits: AugmentExportLimits,
) -> Result<AuthenticatedAugmentExport> {
    limits.validate()?;
    if bytes.len() > limits.max_source_bytes {
        return Err(invalid("source byte limit"));
    }
    let expected = source::expected_file_sha256(AUGMENT_SOURCE_PATH)?;
    if pin.system != ExternalSourceSystem::PathOfBuilding2
        || pin.revision != source::UPSTREAM_REVISION
        || pin.files.len() != 1
        || pin.files[0].path != AUGMENT_SOURCE_PATH
        || pin.files[0].sha256 != expected
    {
        return Err(invalid(
            "source pin differs from independently reviewed manifest",
        ));
    }
    let normalized = std::str::from_utf8(bytes)
        .map_err(|_| invalid("source is not UTF-8"))?
        .replace("\r\n", "\n");
    if hash(normalized.as_bytes()) != expected {
        return Err(invalid("source bytes differ from reviewed manifest"));
    }
    let augments = evaluate(&normalized, limits)?;
    let catalog = AugmentCatalog {
        schema_version: OWNED_AUGMENT_CATALOG_VERSION,
        source: pin.clone(),
        augments,
    };
    let acquisition = decode_owned_augments(
        &encode_owned_augments(&catalog, limits.catalog)?,
        limits.catalog,
    )?;
    let evidence = AugmentExportEvidence {
        schema_version: 1,
        upstream_revision: source::UPSTREAM_REVISION.into(),
        source_manifest_sha256: source::manifest_sha256(),
        extractor_sha256: extractor_fingerprint(),
        acquisition_source_sha256: hash(bytes),
        normalized_source_sha256: expected,
        catalog_sha256: acquisition.sha256().into(),
        counts: acquisition.counts(),
    };
    Ok(AuthenticatedAugmentExport {
        acquisition,
        source_bytes: bytes.to_vec(),
        evidence,
        limits: limits.catalog,
    })
}

fn evaluate(text: &str, limits: AugmentExportLimits) -> Result<Vec<AugmentDefinition>> {
    let lua = Lua::new_with(StdLib::JIT, LuaOptions::default())?;
    if lua.used_memory() > limits.max_lua_bytes {
        return Err(invalid("Lua memory limit below initialization"));
    }
    lua.set_memory_limit(limits.max_lua_bytes)?;
    lua.load("jit.off(); jit.flush(); jit=nil").exec()?;
    let count = Arc::new(AtomicUsize::new(0));
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(100),
        move |_, _| {
            if count.fetch_add(100, Ordering::Relaxed) + 100 >= limits.max_instructions {
                return Err(mlua::Error::RuntimeError(
                    "augment instruction limit".into(),
                ));
            }
            Ok(VmState::Continue)
        },
    )?;
    let table: Table = lua
        .load(text)
        .set_name(AUGMENT_SOURCE_PATH)
        .set_mode(mlua::chunk::ChunkMode::Text)
        .set_environment(lua.create_table()?)
        .eval()?;
    Projector {
        limits,
        left: limits.max_source_entries,
        text_left: limits.catalog.max_total_text_bytes,
    }
    .catalog(table)
}
struct Projector {
    limits: AugmentExportLimits,
    left: usize,
    text_left: usize,
}
impl Projector {
    fn charge(&mut self) -> Result<()> {
        self.left = self
            .left
            .checked_sub(1)
            .ok_or_else(|| invalid("source entry limit"))?;
        Ok(())
    }
    fn text(&mut self, value: Value) -> Result<String> {
        let Value::String(v) = value else {
            return Err(invalid("expected source string"));
        };
        if v.as_bytes().len() > self.limits.catalog.max_text_bytes {
            return Err(invalid("source text limit"));
        }
        self.text_left = self
            .text_left
            .checked_sub(v.as_bytes().len())
            .ok_or_else(|| invalid("source aggregate text limit"))?;
        Ok(v.to_str()?.to_string())
    }
    fn catalog(&mut self, table: Table) -> Result<Vec<AugmentDefinition>> {
        plain(&table)?;
        let mut out = BTreeMap::new();
        let mut row_count = 0usize;
        for entry in table.pairs::<Value, Value>() {
            self.charge()?;
            if out.len() >= self.limits.catalog.max_augments {
                return Err(invalid("augment count limit"));
            }
            let (key, value) = entry?;
            let source_name = self.text(key)?;
            let selectors_table = table_value(value)?;
            let mut selectors = BTreeMap::new();
            for entry in selectors_table.pairs::<Value, Value>() {
                self.charge()?;
                row_count += 1;
                if row_count > self.limits.catalog.max_selectors {
                    return Err(invalid("selector count limit"));
                }
                let (key, value) = entry?;
                let source_selector = self.text(key)?;
                let row = self.selector(source_selector.clone(), table_value(value)?)?;
                selectors.insert(source_selector, row);
            }
            out.insert(
                source_name.clone(),
                AugmentDefinition {
                    source_name,
                    selectors: selectors.into_values().collect(),
                },
            );
        }
        Ok(out.into_values().collect())
    }
    fn parts(&mut self, table: Table) -> Result<(BTreeMap<String, Value>, Vec<Value>)> {
        plain(&table)?;
        let mut fields = BTreeMap::new();
        let mut indexed = BTreeMap::new();
        for pair in table.pairs::<Value, Value>() {
            self.charge()?;
            let (key, value) = pair?;
            match key {
                Value::String(_) => {
                    fields.insert(self.text(key)?, value);
                }
                _ => {
                    let key = integer(key)?;
                    if key == 0 || key > self.limits.catalog.max_lines as u64 {
                        return Err(invalid("array index limit"));
                    }
                    indexed.insert(key, value);
                }
            }
        }
        if indexed.keys().copied().ne(1..=indexed.len() as u64) {
            return Err(invalid("sparse source array"));
        }
        Ok((fields, indexed.into_values().collect()))
    }
    fn array(&mut self, value: Value) -> Result<Vec<Value>> {
        let (fields, values) = self.parts(table_value(value)?)?;
        if !fields.is_empty() {
            return Err(invalid("unexpected array fields"));
        }
        Ok(values)
    }
    fn lines(&mut self, values: Vec<Value>, order: Option<Value>) -> Result<Vec<AugmentLine>> {
        let orders = order.map(|v| self.array(v)).transpose()?;
        if orders.as_ref().is_some_and(|v| v.len() != values.len()) {
            return Err(invalid("stat order membership differs from lines"));
        }
        let mut out = Vec::new();
        for (index, value) in values.into_iter().enumerate() {
            out.push(AugmentLine {
                text: self.text(value)?,
                stat_order: orders
                    .as_ref()
                    .map(|v| number(v[index].clone()))
                    .transpose()?,
            });
        }
        Ok(out)
    }
    fn selector(&mut self, source_selector: String, table: Table) -> Result<AugmentSelector> {
        let (mut fields, values) = self.parts(table)?;
        let kind = match self
            .text(
                fields
                    .remove("type")
                    .ok_or_else(|| invalid("missing augment kind"))?,
            )?
            .as_str()
        {
            "Rune" => AugmentKind::Rune,
            "SoulCore" => AugmentKind::SoulCore,
            "Idol" => AugmentKind::Idol,
            "AbyssalEye" => AugmentKind::AbyssalEye,
            "CongealedMist" => AugmentKind::CongealedMist,
            _ => return Err(invalid("unreviewed augment kind")),
        };
        let normal = self.lines(values, fields.remove("statOrder"))?;
        let bonded = if let Some(value) = fields.remove("bonded") {
            let (mut metadata, lines) = self.parts(table_value(value)?)?;
            let order = metadata.remove("statOrder");
            if !metadata.is_empty() {
                return Err(invalid("unreviewed Bonded fields"));
            }
            Some(self.lines(lines, order)?)
        } else {
            None
        };
        let metadata = AugmentMetadata {
            local_mod: fields.remove("localMod").map(boolean).transpose()?,
            level_requirement: fields.remove("levelReq").map(u32_value).transpose()?,
            limit: fields.remove("limit").map(u32_value).transpose()?,
            limit_id: fields.remove("limitId").map(|v| self.text(v)).transpose()?,
            socket_bound: fields.remove("isSocketBound").map(boolean).transpose()?,
            can_socket_in_chakra_slots: fields
                .remove("canSocketInChakraSlots")
                .map(boolean)
                .transpose()?,
            can_socket_in_unique_items: fields
                .remove("canSocketInUniqueItems")
                .map(boolean)
                .transpose()?,
            can_socket_in_jewellery: fields
                .remove("canSocketInJewellery")
                .map(boolean)
                .transpose()?,
            can_socket_in_corrupted_sanctified: fields
                .remove("canSocketInCorruptedSanctified")
                .map(boolean)
                .transpose()?,
            trade_hashes: fields
                .remove("tradeHashes")
                .map(|v| self.trades(v))
                .transpose()?,
        };
        if !fields.is_empty() {
            return Err(invalid(format!(
                "unreviewed selector fields: {:?}",
                fields.keys().collect::<Vec<_>>()
            )));
        }
        Ok(AugmentSelector {
            source_selector,
            kind,
            normal,
            bonded,
            metadata,
            semantics: AugmentSemantics::Unconverted,
        })
    }
    fn trades(&mut self, value: Value) -> Result<Vec<AugmentTradeHash>> {
        let table = table_value(value)?;
        let mut out = BTreeMap::new();
        for pair in table.pairs::<Value, Value>() {
            self.charge()?;
            if out.len() >= self.limits.catalog.max_trade_hashes {
                return Err(invalid("trade hash limit"));
            }
            let (key, value) = pair?;
            let hash = integer(key)?;
            let lines = self
                .array(value)?
                .into_iter()
                .map(|v| self.text(v))
                .collect::<Result<Vec<_>>>()?;
            out.insert(hash, AugmentTradeHash { hash, lines });
        }
        Ok(out.into_values().collect())
    }
}
fn plain(table: &Table) -> Result<()> {
    if table.metatable().is_some() {
        return Err(invalid("metatable is not a finite data record"));
    }
    Ok(())
}
fn table_value(value: Value) -> Result<Table> {
    let Value::Table(table) = value else {
        return Err(invalid("expected source table"));
    };
    plain(&table)?;
    Ok(table)
}
fn number(value: Value) -> Result<f64> {
    let n = match value {
        Value::Integer(v) => v as f64,
        Value::Number(v) => v,
        _ => return Err(invalid("expected number")),
    };
    if !n.is_finite() {
        return Err(invalid("nonfinite source number"));
    }
    Ok(n)
}
fn integer(value: Value) -> Result<u64> {
    let n = number(value)?;
    if !(0.0..=9_007_199_254_740_991.0).contains(&n) || n.fract() != 0.0 {
        return Err(invalid("expected portable nonnegative integer"));
    }
    Ok(n as u64)
}
fn u32_value(value: Value) -> Result<u32> {
    u32::try_from(integer(value)?).map_err(|_| invalid("source integer exceeds u32"))
}
fn boolean(value: Value) -> Result<bool> {
    let Value::Boolean(v) = value else {
        return Err(invalid("expected Boolean metadata"));
    };
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    const MINIMAL: &str =
        "return { Example = { weapon = {type='Rune','2 damage',statOrder={3.5}} } }";
    #[test]
    fn isolated_source_has_no_libraries_or_environment_and_preserves_fractions() {
        for name in [
            "_G",
            "getfenv",
            "setfenv",
            "require",
            "package",
            "io",
            "os",
            "ffi",
            "debug",
            "load",
            "loadstring",
            "loadfile",
            "dofile",
            "jit",
        ] {
            let text = format!(
                "local hidden={name}; return {{Example={{weapon={{type='Rune',hidden==nil and '2 damage' or 'exposed',statOrder={{3.5}}}}}}}}"
            );
            let rows = evaluate(&text, Default::default()).unwrap();
            assert_eq!(rows[0].selectors[0].normal[0].text, "2 damage");
            assert_eq!(rows[0].selectors[0].normal[0].stat_order, Some(3.5));
        }
    }
    #[test]
    fn unknown_shapes_sparse_arrays_and_nonfinite_values_fail_closed() {
        for text in [
            "return {Example={weapon={type='Future'}}}",
            "return {Example={weapon={type='Rune',newSemantics=true}}}",
            "return {Example={weapon={type='Rune',localMod=1}}}",
            "return {Example={weapon={type='Rune',[2]='sparse'}}}",
            "return {Example={weapon={type='Rune','x',statOrder={1,2}}}}",
            "return {Example={weapon={type='Rune','x',statOrder={0/0}}}}",
            "return {Example={weapon={type='Rune',bonded={unknown=1}}}}",
            "return {Example={weapon={type='Rune',tradeHashes={word={'x'}}}}}",
            "return {Example={weapon={type='Rune',limit=-1}}}",
        ] {
            assert!(evaluate(text, Default::default()).is_err(), "{text}");
        }
        assert!(evaluate(MINIMAL, Default::default()).is_ok());
    }
    #[test]
    fn execution_and_projection_are_bounded() {
        let tiny = AugmentExportLimits {
            max_instructions: 1000,
            ..Default::default()
        };
        assert!(
            evaluate("while true do end", tiny)
                .unwrap_err()
                .to_string()
                .contains("instruction limit")
        );
        let tiny = AugmentExportLimits {
            max_lua_bytes: 128 * 1024,
            ..Default::default()
        };
        assert!(
            evaluate(
                "local t={}; for i=1,100000 do t[i]={i,i,i} end; return t",
                tiny
            )
            .is_err()
        );
        let tiny = AugmentExportLimits {
            max_source_entries: 1,
            ..Default::default()
        };
        assert!(evaluate(MINIMAL, tiny).is_err());
        assert!(evaluate("\u{1b}Lua", Default::default()).is_err());
    }
    #[test]
    fn absent_false_empty_and_unconverted_states_stay_distinct() {
        let rows=evaluate("return {Example={weapon={type='Idol',localMod=false,bonded={},tradeHashes={}},armour={type='Idol'}}}",Default::default()).unwrap();
        let armour = &rows[0].selectors[0];
        let weapon = &rows[0].selectors[1];
        assert_eq!(armour.bonded, None);
        assert_eq!(weapon.bonded, Some(vec![]));
        assert_eq!(armour.metadata.local_mod, None);
        assert_eq!(weapon.metadata.local_mod, Some(false));
        assert_eq!(weapon.metadata.trade_hashes, Some(vec![]));
        assert_eq!(weapon.semantics, AugmentSemantics::Unconverted);
    }
    fn pinned() -> (Vec<u8>, SourcePin) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
        let bytes = source::read_verified_text(&root, AUGMENT_SOURCE_PATH)
            .unwrap()
            .into_bytes();
        let pin = SourcePin {
            system: ExternalSourceSystem::PathOfBuilding2,
            revision: source::UPSTREAM_REVISION.into(),
            files: vec![SourceFilePin {
                path: AUGMENT_SOURCE_PATH.into(),
                sha256: source::expected_file_sha256(AUGMENT_SOURCE_PATH).unwrap(),
            }],
        };
        (bytes, pin)
    }
    #[test]
    fn private_acquisition_rechecks_independent_manifest_and_rejects_repinned_changes() {
        let (bytes, pin) = pinned();
        let mut changed = bytes.clone();
        changed.extend_from_slice(b"\nreturn {}\n");
        let mut repinned = pin.clone();
        repinned.files[0].sha256 = hash(&changed);
        assert!(
            export_pinned_bytes(&changed, &pin, Default::default())
                .unwrap_err()
                .to_string()
                .contains("source bytes differ")
        );
        assert!(
            export_pinned_bytes(&changed, &repinned, Default::default())
                .unwrap_err()
                .to_string()
                .contains("source pin differs")
        );
        for case in 0..6 {
            let mut wrong = pin.clone();
            match case {
                0 => wrong.revision = "unreviewed".into(),
                1 => wrong.system = ExternalSourceSystem::PathOfBuilding1,
                2 => wrong.files.clear(),
                3 => wrong.files.push(wrong.files[0].clone()),
                4 => wrong.files[0].path = "src/Other.lua".into(),
                _ => wrong.files[0].sha256 = "0".repeat(64),
            }
            assert!(
                export_pinned_bytes(&bytes, &wrong, Default::default()).is_err(),
                "case {case}"
            );
        }
        assert!(export_pinned_bytes(&[0xff], &pin, Default::default()).is_err());
        let tiny = AugmentExportLimits {
            max_source_bytes: 1,
            ..Default::default()
        };
        assert!(
            export_pinned_bytes(&bytes, &pin, tiny)
                .unwrap_err()
                .to_string()
                .contains("source byte limit")
        );
    }
    #[test]
    fn only_crlf_normalization_preserves_content_while_exact_acquired_bytes_differ() {
        let (bytes, pin) = pinned();
        let lf = export_pinned_bytes(&bytes, &pin, Default::default()).unwrap();
        let crlf = String::from_utf8(bytes.clone())
            .unwrap()
            .replace('\n', "\r\n")
            .into_bytes();
        let crlf = export_pinned_bytes(&crlf, &pin, Default::default()).unwrap();
        assert_eq!(lf.catalog_bytes(), crlf.catalog_bytes());
        assert_eq!(
            lf.evidence().normalized_source_sha256,
            crlf.evidence().normalized_source_sha256
        );
        assert_ne!(
            lf.evidence().acquisition_source_sha256,
            crlf.evidence().acquisition_source_sha256
        );
        assert_ne!(lf.source_bytes(), crlf.source_bytes());
        let mut bom = vec![0xef, 0xbb, 0xbf];
        bom.extend_from_slice(&bytes);
        assert!(export_pinned_bytes(&bom, &pin, Default::default()).is_err());
    }
}
