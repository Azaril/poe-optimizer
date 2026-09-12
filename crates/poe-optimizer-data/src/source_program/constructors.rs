//! Optional catalog-owned source constructor evidence; never legacy parser wire.
use super::*;
use serde::de::{SeqAccess, Visitor};
use std::fmt;
pub(super) mod walk;
pub const SOURCE_PROGRAM_CONSTRUCTORS_SCHEMA_VERSION: u32 = 1;
pub const SOURCE_TABLE_RUNTIME_REVISION: &str = "luajit-src@210.7.3+1ee778a";
const MAX_SITES: usize = 100_000;
const MAX_BYTES: usize = 8 * 1024 * 1024;
const TNEW: u32 = 52;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTableRuntimeFamily {
    LuaJit21,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRuntimeArchitecture {
    X86,
    X64,
    Arm64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceNumberMode {
    Single,
    Dual,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceEndianness {
    Little,
    Big,
}
/// A source runtime/build claim, not a statement about the Rust execution host.
/// Build-only flags require a trusted adapter attestation; reflection/version
/// strings alone cannot authenticate them. The engine explicitly admits profiles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceTableRuntimeProfile {
    pub family: SourceTableRuntimeFamily,
    pub source_revision: String,
    pub architecture: SourceRuntimeArchitecture,
    pub number_mode: SourceNumberMode,
    pub endian: SourceEndianness,
    pub gc64: bool,
    pub frame_slots: u8,
    pub table_bump: bool,
    pub bytecode_version: u8,
    pub lua52_compat: bool,
}
impl SourceTableRuntimeProfile {
    /// The initial supported semantic profile. Constructing this descriptor is
    /// not evidence that any actual Lua state was built with these settings.
    pub fn luajit21_x64_single() -> Self {
        Self {
            family: SourceTableRuntimeFamily::LuaJit21,
            source_revision: SOURCE_TABLE_RUNTIME_REVISION.into(),
            architecture: SourceRuntimeArchitecture::X64,
            number_mode: SourceNumberMode::Single,
            endian: SourceEndianness::Little,
            gc64: true,
            frame_slots: 2,
            table_bump: false,
            bytecode_version: 2,
            lua52_compat: false,
        }
    }
    /// Host-independent admission predicate: WASM may emulate this source profile.
    pub fn is_supported_array_profile(&self) -> bool {
        self.family == SourceTableRuntimeFamily::LuaJit21
            && self.source_revision == SOURCE_TABLE_RUNTIME_REVISION
            && self.architecture == SourceRuntimeArchitecture::X64
            && self.number_mode == SourceNumberMode::Single
            && self.endian == SourceEndianness::Little
            && self.gc64
            && self.frame_slots == 2
            && !self.table_bump
            && self.bytecode_version == 2
            && !self.lua52_compat
    }
    pub(super) fn validate_shape(&self) -> SourceProgramResult<()> {
        if self.source_revision.is_empty()
            || self.source_revision.len() > 128
            || !self.source_revision.is_ascii()
            || self
                .source_revision
                .bytes()
                .any(|b| b.is_ascii_control() || b.is_ascii_whitespace())
            || !(1..=2).contains(&self.frame_slots)
            || self.bytecode_version == 0
        {
            return Err(invalid("invalid source table runtime profile"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceTableAllocation {
    New { array_slots: u32, hash_bits: u8 },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceProgramConstructor {
    pub callback: SourceCallbackId,
    pub provenance: SourceProgramProvenance,
    /// Exact Table-expression offsets relative to the complete function slice.
    pub expression: SourceProgramLocation,
    /// Authenticated instruction-stream digest, using the observing adapter's
    /// fixed canonical encoding; never a freshly recompiled snippet's bytecode.
    pub bytecode_sha256: String,
    /// Original root-prototype instruction PC (PC0 is the function header).
    pub bytecode_pc: u32,
    pub instruction: u32,
    pub allocation: SourceTableAllocation,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceProgramConstructors {
    pub schema_version: u32,
    pub profile: SourceTableRuntimeProfile,
    #[serde(deserialize_with = "bounded_sites")]
    pub sites: Vec<SourceProgramConstructor>,
}
fn invalid(message: &str) -> SourceProgramError {
    failure(SourceProgramErrorKind::InvalidData, message)
}
fn binding(message: &str) -> SourceProgramError {
    failure(SourceProgramErrorKind::Binding, message)
}
fn resource() -> SourceProgramError {
    failure(
        SourceProgramErrorKind::ResourceLimit,
        "source constructor metadata bound",
    )
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl SourceProgramConstructors {
    /// Bounded structural authoring input. Source observation and catalog binding
    /// are separate; success does not grant execution/layout capability.
    pub fn from_bytes(bytes: &[u8]) -> SourceProgramResult<Self> {
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(resource());
        }
        let data: Self = serde_json::from_slice(bytes)
            .map_err(|e| invalid(&format!("source constructors JSON: {e}")))?;
        data.validate_shape()?;
        Ok(data)
    }
    fn validate_shape(&self) -> SourceProgramResult<()> {
        if self.schema_version != SOURCE_PROGRAM_CONSTRUCTORS_SCHEMA_VERSION {
            return Err(invalid("unsupported source constructor schema"));
        }
        self.profile.validate_shape()?;
        if self.sites.len() > MAX_SITES {
            return Err(resource());
        }
        let mut bytes = self.profile.source_revision.len();
        for site in &self.sites {
            for value in [
                &site.provenance.source.path,
                &site.provenance.source.sha256,
                &site.provenance.function_sha256,
                &site.bytecode_sha256,
            ] {
                bytes = bytes.checked_add(value.len()).ok_or_else(resource)?;
                if value.len() > 4096 || bytes > MAX_BYTES {
                    return Err(resource());
                }
            }
            if !digest(&site.bytecode_sha256)
                || site.bytecode_pc == 0
                || site.bytecode_pc > 1_000_000
                || site.expression.start >= site.expression.end
            {
                return Err(invalid(
                    "invalid source constructor bytecode/range evidence",
                ));
            }
            let SourceTableAllocation::New {
                array_slots,
                hash_bits,
            } = site.allocation;
            if site.instruction & 255 != TNEW
                || (site.instruction >> 16) & 2047 != array_slots
                || (site.instruction >> 27) != u32::from(hash_bits)
            {
                return Err(binding(
                    "constructor allocation differs from TNEW instruction",
                ));
            }
            if array_slots != 0 || hash_bits != 0 {
                return Err(failure(
                    SourceProgramErrorKind::UnsupportedCapability,
                    "only no-template empty TNEW allocation is represented",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn validate_catalog(
        &self,
        data: &SourceProgramData,
        owner: &SourceProgramOwner,
    ) -> SourceProgramResult<()> {
        if owner.parser().is_some() {
            return Err(failure(
                SourceProgramErrorKind::UnsupportedCapability,
                "parser owners cannot bind source constructor metadata",
            ));
        }
        self.validate_shape()?;
        let mut sites = BTreeSet::new();
        let mut pcs = BTreeSet::new();
        let mut callbacks = BTreeMap::new();
        for site in &self.sites {
            let program = data
                .callbacks
                .get(&site.callback)
                .and_then(|id| id.0.checked_sub(1))
                .and_then(|i| data.programs.get(i as usize))
                .ok_or_else(|| binding("constructor callback lacks a program in this catalog"))?;
            if program.callback != site.callback || program.provenance != site.provenance {
                return Err(binding(
                    "constructor provenance differs from this catalog's program",
                ));
            }
            let key = (site.callback, site.expression.start, site.expression.end);
            if !sites.insert(key) || !pcs.insert((site.callback, site.bytecode_pc)) {
                return Err(binding(
                    "duplicate constructor expression or instruction site",
                ));
            }
            if let Some((_, hash)) =
                callbacks.insert(site.callback, (program, &site.bytecode_sha256))
                && hash != &site.bytecode_sha256
            {
                return Err(binding(
                    "constructor sites disagree on original function bytecode",
                ));
            }
        }
        // Walk each selected already-validated program only once. Store matches
        // only for requested sites, not a duplicate index of the whole IR.
        let mut matched = BTreeMap::new();
        for (callback, (program, _)) in callbacks {
            walk::tables(program, |expr, fields| {
                let key = (callback, expr.location.start, expr.location.end);
                if sites.contains(&key) {
                    let value = matched.entry(key).or_insert((0usize, true));
                    value.0 += 1;
                    value.1 &= fields.is_empty();
                }
            });
        }
        for site in &self.sites {
            if matched.get(&(site.callback, site.expression.start, site.expression.end))
                != Some(&(1, true))
            {
                return Err(binding(
                    "constructor site does not identify one empty Table expression",
                ));
            }
        }
        Ok(())
    }
}
impl SourceProgramCatalog {
    /// Fresh program identity binds source claims to owned, fully verified IR.
    /// The same definition owner may back other catalogs with different code;
    /// constructor records cannot be attached by mutating an existing catalog.
    pub fn new_with_constructors(
        data: SourceProgramData,
        owner: SourceProgramOwner,
        constructors: SourceProgramConstructors,
    ) -> SourceProgramResult<Self> {
        Self::new_with_facets(data, owner, Some(constructors), None)
    }
    pub fn constructors(&self) -> Option<&SourceProgramConstructors> {
        self.constructors.as_deref()
    }
}
fn bounded_sites<'de, D: serde::Deserializer<'de>>(
    de: D,
) -> Result<Vec<SourceProgramConstructor>, D::Error> {
    struct Sites;
    impl<'de> Visitor<'de> for Sites {
        type Value = Vec<SourceProgramConstructor>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("bounded source constructor sites")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::new();
            while let Some(value) = seq.next_element()? {
                if out.len() >= MAX_SITES {
                    return Err(serde::de::Error::custom("source constructor count bound"));
                }
                out.push(value);
            }
            Ok(out)
        }
    }
    de.deserialize_seq(Sites)
}
