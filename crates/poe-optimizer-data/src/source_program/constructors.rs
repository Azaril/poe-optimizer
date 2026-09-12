//! Optional catalog-owned source constructor evidence; never legacy parser wire.
use super::*;
use serde::de::{SeqAccess, Visitor};
use std::fmt;
pub(super) mod walk;
pub const SOURCE_PROGRAM_CONSTRUCTORS_SCHEMA_VERSION: u32 = 1;
pub const SOURCE_TABLE_RUNTIME_REVISION: &str = "luajit-src@210.7.3+1ee778a";
/// Maximum retained constructor site records in one catalog.
pub const SOURCE_PROGRAM_CONSTRUCTORS_MAX_SITES: usize = 100_000;
/// Maximum aggregate retained profile/provenance/digest text and key bytes in one catalog.
pub const SOURCE_PROGRAM_CONSTRUCTORS_MAX_TEXT_BYTES: usize = 8 * 1024 * 1024;
/// Maximum reserved string keys per template, matching the IR field bound.
pub const SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEYS_PER_TEMPLATE: usize = 4096;
/// Maximum reserved key records retained across all sites in one catalog.
pub const SOURCE_PROGRAM_CONSTRUCTORS_MAX_RESERVED_KEYS: usize = 500_000;
/// Maximum bytes in one reserved key, matching the IR string bound.
pub const SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEY_BYTES: usize = 4096;
const TNEW: u32 = 52;
const TDUP: u32 = 53;

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceTableAllocation {
    /// Decoded interpreter TNEW capacity, including slot zero. In the pinned
    /// TNEW encoding the lower eleven-bit hint 0x7ff means 0x801 slots;
    /// it must never be stored here as an apparent capacity of 0x7ff. This does
    /// not assert subsequent allocation history or a traced JIT specialization.
    New { array_slots: u32, hash_bits: u8 },
    /// Complete observed raw order of distinct string keys whose source TDUP
    /// template values are all exact template self markers. Duplication reserves
    /// these keys with initially nil values; this is not live-entry traversal,
    /// array capacity, a raw length, or a claim about subsequent tail results.
    /// The observing adapter authenticates the actual Function/template and its
    /// rooting lifetime; constructing/deserializing this claim does not do so.
    DuplicateReservedStrings {
        #[serde(deserialize_with = "bounded_keys")]
        keys: Vec<Vec<u8>>,
    },
}
impl SourceTableAllocation {
    /// Decode a pinned LuaJIT TNEW allocation instruction. This is a structural
    /// claim only: source observation, expression binding and runtime-profile
    /// admission remain separate. Hash allocations decode but are not admitted
    /// by the currently supported constructor family.
    pub fn from_tnew_instruction(instruction: u32) -> SourceProgramResult<Self> {
        if instruction & 255 != TNEW {
            return Err(binding("constructor instruction is not TNEW"));
        }
        let hint = (instruction >> 16) & 0x7ff;
        Ok(Self::New {
            array_slots: if hint == 0x7ff { 0x801 } else { hint },
            hash_bits: (instruction >> 27) as u8,
        })
    }
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
        if self.sites.len() > SOURCE_PROGRAM_CONSTRUCTORS_MAX_SITES {
            return Err(resource());
        }
        // Preflight the entire owned facet before allocating comparison sets.
        // All key bytes are borrowed during later normalization and matching.
        let mut budget = RetainedBudget {
            bytes: self.profile.source_revision.len(),
            keys: 0,
        };
        for site in &self.sites {
            budget.site(site)?;
        }
        for site in &self.sites {
            if !digest(&site.bytecode_sha256)
                || site.bytecode_pc == 0
                || site.bytecode_pc > 1_000_000
                || site.expression.start >= site.expression.end
            {
                return Err(invalid(
                    "invalid source constructor bytecode/range evidence",
                ));
            }
            match &site.allocation {
                SourceTableAllocation::New {
                    array_slots,
                    hash_bits,
                } => {
                    if SourceTableAllocation::from_tnew_instruction(site.instruction)?
                        != site.allocation
                    {
                        return Err(binding(
                            "constructor allocation differs from TNEW instruction",
                        ));
                    }
                    if *hash_bits != 0 || matches!(*array_slots, 1 | 2) {
                        return Err(failure(
                            SourceProgramErrorKind::UnsupportedCapability,
                            "only empty or list-only no-hash TNEW allocation is represented",
                        ));
                    }
                }
                SourceTableAllocation::DuplicateReservedStrings { keys } => {
                    if site.instruction & 255 != TDUP {
                        return Err(binding(
                            "reserved-string constructor instruction is not TDUP",
                        ));
                    }
                    if keys.is_empty() {
                        return Err(binding("reserved-string template must contain keys"));
                    }
                    let unique: BTreeSet<&[u8]> = keys.iter().map(Vec::as_slice).collect();
                    if unique.len() != keys.len() {
                        return Err(binding("reserved-string template has duplicate keys"));
                    }
                }
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
        let mut templates = BTreeMap::new();
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
            if let SourceTableAllocation::DuplicateReservedStrings { keys } = &site.allocation {
                // D selects a constant in this exact callback's prototype. The
                // destination register and PC do not change that template.
                let template = (site.callback, site.instruction >> 16);
                if let Some(previous) = templates.insert(template, keys)
                    && previous != keys
                {
                    return Err(binding(
                        "constructor sites disagree on reserved template order",
                    ));
                }
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
                    let value = matched.entry(key).or_insert((0usize, None));
                    value.0 += 1;
                    value.1 = Some(fields);
                }
            });
        }
        for site in &self.sites {
            let Some((1, Some(fields))) =
                matched.get(&(site.callback, site.expression.start, site.expression.end))
            else {
                return Err(binding(
                    "constructor site does not identify one Table expression",
                ));
            };
            let matches = match &site.allocation {
                SourceTableAllocation::New { .. } => {
                    list_allocation(fields).as_ref() == Some(&site.allocation)
                }
                SourceTableAllocation::DuplicateReservedStrings { keys } => {
                    reserved_fields_match(fields, keys)
                }
            };
            if !matches {
                return Err(binding(
                    "constructor Table fields differ from the bound allocation family or keys",
                ));
            }
        }
        Ok(())
    }
}
#[derive(Default)]
struct RetainedBudget {
    bytes: usize,
    keys: usize,
}
impl RetainedBudget {
    fn site(&mut self, site: &SourceProgramConstructor) -> SourceProgramResult<()> {
        for value in [
            &site.provenance.source.path,
            &site.provenance.source.sha256,
            &site.provenance.function_sha256,
            &site.bytecode_sha256,
        ] {
            if value.len() > 4096 {
                return Err(resource());
            }
            self.bytes = self.bytes.checked_add(value.len()).ok_or_else(resource)?;
        }
        if let SourceTableAllocation::DuplicateReservedStrings { keys } = &site.allocation {
            if keys.len() > SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEYS_PER_TEMPLATE {
                return Err(resource());
            }
            self.keys = self.keys.checked_add(keys.len()).ok_or_else(resource)?;
            for key in keys {
                if key.len() > SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEY_BYTES {
                    return Err(resource());
                }
                self.bytes = self.bytes.checked_add(key.len()).ok_or_else(resource)?;
            }
        }
        if self.bytes > SOURCE_PROGRAM_CONSTRUCTORS_MAX_TEXT_BYTES
            || self.keys > SOURCE_PROGRAM_CONSTRUCTORS_MAX_RESERVED_KEYS
        {
            return Err(resource());
        }
        Ok(())
    }
}
fn reserved_fields_match(fields: &[SourceProgramField], keys: &[Vec<u8>]) -> bool {
    let mut actual = BTreeSet::new();
    for (index, field) in fields.iter().enumerate() {
        let key = match field {
            SourceProgramField::Named { key, .. } => key.as_bytes(),
            SourceProgramField::Keyed { key, .. } => match &key.operation {
                SourceProgramExprKind::Literal {
                    value: crate::modifier_parser::ParserFactoryLiteral::Text(value),
                } => value.as_bytes(),
                SourceProgramExprKind::Bytes { value } => value.as_slice(),
                _ => return false,
            },
            SourceProgramField::Tail { .. } if index + 1 == fields.len() => continue,
            SourceProgramField::Tail { .. } | SourceProgramField::List { .. } => return false,
        };
        if !actual.insert(key) {
            return false;
        }
    }
    actual.len() == keys.len() && keys.iter().all(|key| actual.contains(key.as_slice()))
}
/// The compiler counts syntactic list fields, including the one final expanded
/// call/varargs field even when it later returns no values. Parenthesized calls
/// remain scalar List fields. Trailing separators do not add fields. The prior
/// complete IR validator has already bounded fields and made Tail final.
fn list_allocation(fields: &[SourceProgramField]) -> Option<SourceTableAllocation> {
    if fields.iter().enumerate().any(|(index, field)| match field {
        SourceProgramField::List { .. } => false,
        SourceProgramField::Tail { .. } => index + 1 != fields.len(),
        SourceProgramField::Named { .. } | SourceProgramField::Keyed { .. } => true,
    }) {
        return None;
    }
    let hint = if fields.is_empty() {
        0
    } else {
        // Bounded IR field count makes this addition and cast exact.
        (fields.len() as u32 + 1).clamp(3, 0x7ff)
    };
    Some(SourceTableAllocation::New {
        array_slots: if hint == 0x7ff { 0x801 } else { hint },
        hash_bits: 0,
    })
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
            let mut budget = RetainedBudget::default();
            while let Some(value) = seq.next_element()? {
                if out.len() >= SOURCE_PROGRAM_CONSTRUCTORS_MAX_SITES {
                    return Err(serde::de::Error::custom("source constructor count bound"));
                }
                budget.site(&value).map_err(serde::de::Error::custom)?;
                out.push(value);
            }
            Ok(out)
        }
    }
    de.deserialize_seq(Sites)
}

// Avoid Vec's untrusted size-hint reservation at either nesting level. Whole
// JSON input, total site/key records and aggregate bytes have separate bounds.
fn bounded_keys<'de, D: serde::Deserializer<'de>>(de: D) -> Result<Vec<Vec<u8>>, D::Error> {
    struct Key(Vec<u8>);
    impl<'de> Deserialize<'de> for Key {
        fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
            struct Bytes;
            impl<'de> Visitor<'de> for Bytes {
                type Value = Key;
                fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    f.write_str("bounded reserved string bytes")
                }
                fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Key, A::Error> {
                    let mut bytes = Vec::new();
                    while let Some(byte) = seq.next_element()? {
                        if bytes.len() >= SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEY_BYTES {
                            return Err(serde::de::Error::custom("reserved key byte bound"));
                        }
                        bytes.push(byte);
                    }
                    Ok(Key(bytes))
                }
            }
            de.deserialize_seq(Bytes)
        }
    }
    struct Keys;
    impl<'de> Visitor<'de> for Keys {
        type Value = Vec<Vec<u8>>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("bounded reserved string keys")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut keys = Vec::new();
            let mut bytes = 0usize;
            while let Some(Key(key)) = seq.next_element()? {
                bytes = bytes
                    .checked_add(key.len())
                    .ok_or_else(|| serde::de::Error::custom("reserved key byte bound"))?;
                if keys.len() >= SOURCE_PROGRAM_CONSTRUCTORS_MAX_KEYS_PER_TEMPLATE
                    || bytes > SOURCE_PROGRAM_CONSTRUCTORS_MAX_TEXT_BYTES
                {
                    return Err(serde::de::Error::custom("reserved key count/byte bound"));
                }
                keys.push(key);
            }
            Ok(keys)
        }
    }
    de.deserialize_seq(Keys)
}
