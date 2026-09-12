//! Source-authenticated creation-site claims, separate from live closure inputs.
//! Structural verification binds complete IR/prototypes and lexical identities;
//! it does not authenticate an actual Lua host or confer numerical admission.
use super::*;
use serde::de::{SeqAccess, Visitor};
use std::{fmt, marker::PhantomData};

pub const SOURCE_PROGRAM_CLOSURE_CREATIONS_SCHEMA_VERSION: u32 = 1;
const MAX_SITES: usize = 100_000;
const MAX_CAPTURES: usize = 128;
const MAX_RECORDS: usize = 1_000_000;
const MAX_BYTES: usize = 8 * 1024 * 1024;
const FNEW: u32 = 51;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceProgramClosureLocalKind {
    Parameter,
    Declare,
    NumericFor,
    GenericFor,
}
/// Exact semantic declaration binding supplied by the source observer/lowerer.
/// `local` is the typed IR identity; `register` is the source VM slot. Parameter
/// ranges select their token (or the colon-method name that introduces implicit
/// `self`); other ranges match the complete declaring statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceProgramClosureLocalBinding {
    pub local: u16,
    pub register: u8,
    pub declaration: SourceProgramLocation,
    pub kind: SourceProgramClosureLocalKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceProgramClosureCreation {
    pub callback: SourceCallbackId,
    pub provenance: SourceProgramProvenance,
    pub expression: SourceProgramLocation,
    pub prototype: SourceClosurePrototypeId,
    pub child_provenance: SourceProgramProvenance,
    /// Canonical observed instruction digests; their encoding and handling of
    /// patched JIT instructions are the importing adapter's proof obligation.
    pub bytecode_sha256: String,
    pub bytecode_pc: u32,
    pub instruction: u32,
    pub child_bytecode_sha256: String,
    #[serde(deserialize_with = "capture_vec")]
    pub captures: Vec<SourceProgramCaptureOrigin>,
    /// Actual ordered uint16 source descriptors, retaining the immutable hint.
    /// That hint never permits copying a value instead of retaining its cell.
    #[serde(deserialize_with = "capture_vec")]
    pub capture_descriptors: Vec<u16>,
    /// Exactly one declaration proof per distinct captured parent local.
    #[serde(deserialize_with = "capture_vec")]
    pub local_bindings: Vec<SourceProgramClosureLocalBinding>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceProgramClosureCreations {
    pub schema_version: u32,
    pub profile: SourceTableRuntimeProfile,
    #[serde(deserialize_with = "site_vec")]
    pub sites: Vec<SourceProgramClosureCreation>,
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
        "closure creation metadata bound",
    )
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn bounded_vec<'de, D, T, const N: usize>(de: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Bounded<T, const N: usize>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const N: usize> Visitor<'de> for Bounded<T, N> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("bounded closure creation metadata list")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::new();
            while let Some(value) = seq.next_element()? {
                if out.len() >= N {
                    return Err(serde::de::Error::custom(
                        "closure creation metadata list bound",
                    ));
                }
                out.push(value);
            }
            Ok(out)
        }
    }
    de.deserialize_seq(Bounded::<T, N>(PhantomData))
}
fn capture_vec<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    de: D,
) -> Result<Vec<T>, D::Error> {
    bounded_vec::<D, T, MAX_CAPTURES>(de)
}
fn site_vec<'de, D: serde::Deserializer<'de>>(
    de: D,
) -> Result<Vec<SourceProgramClosureCreation>, D::Error> {
    bounded_vec::<D, SourceProgramClosureCreation, MAX_SITES>(de)
}
impl SourceProgramClosureCreations {
    pub fn from_bytes(bytes: &[u8]) -> SourceProgramResult<Self> {
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(resource());
        }
        let data: Self = serde_json::from_slice(bytes)
            .map_err(|e| invalid(&format!("closure creation JSON: {e}")))?;
        data.validate_shape()?;
        Ok(data)
    }
    fn validate_shape(&self) -> SourceProgramResult<()> {
        if self.schema_version != SOURCE_PROGRAM_CLOSURE_CREATIONS_SCHEMA_VERSION {
            return Err(invalid("unsupported closure creation schema"));
        }
        self.profile.validate_shape()?;
        if self.sites.len() > MAX_SITES {
            return Err(resource());
        }
        let mut records = self.sites.len();
        let mut bytes = self.profile.source_revision.len();
        // Complete scalar/count preflight precedes auxiliary map allocations.
        for site in &self.sites {
            for n in [
                site.captures.len(),
                site.capture_descriptors.len(),
                site.local_bindings.len(),
            ] {
                records = records.checked_add(n).ok_or_else(resource)?;
                if n > MAX_CAPTURES || records > MAX_RECORDS {
                    return Err(resource());
                }
            }
            for value in [
                &site.provenance.source.path,
                &site.provenance.source.sha256,
                &site.provenance.function_sha256,
                &site.child_provenance.source.path,
                &site.child_provenance.source.sha256,
                &site.child_provenance.function_sha256,
                &site.bytecode_sha256,
                &site.child_bytecode_sha256,
            ] {
                bytes = bytes.checked_add(value.len()).ok_or_else(resource)?;
                if value.len() > 4096 || bytes > MAX_BYTES {
                    return Err(resource());
                }
            }
            if !digest(&site.bytecode_sha256)
                || !digest(&site.child_bytecode_sha256)
                || site.bytecode_pc == 0
                || site.bytecode_pc > 1_000_000
                || site.expression.start >= site.expression.end
            {
                return Err(invalid("invalid closure creation bytecode/range evidence"));
            }
            if site.instruction & 255 != FNEW
                || site.captures.len() != site.capture_descriptors.len()
            {
                return Err(binding(
                    "closure creation lacks exact FNEW/capture descriptors",
                ));
            }
            for (origin, raw) in site.captures.iter().zip(&site.capture_descriptors) {
                match origin {
                    SourceProgramCaptureOrigin::Local { .. }
                        if raw & 0x8000 != 0 && raw & 0x3f00 == 0 => {}
                    SourceProgramCaptureOrigin::ParentCapture { upvalue }
                        if raw == upvalue && *upvalue < 128 => {}
                    _ => {
                        return Err(binding(
                            "closure capture origin differs from source descriptor",
                        ));
                    }
                }
            }
            if site
                .local_bindings
                .iter()
                .any(|b| b.declaration.start >= b.declaration.end)
            {
                return Err(invalid("empty closure lexical declaration range"));
            }
        }
        Ok(())
    }
    fn validate_catalog(
        &self,
        data: &SourceProgramData,
        owner: &SourceProgramOwner,
    ) -> SourceProgramResult<()> {
        if owner.parser().is_some() {
            return Err(failure(
                SourceProgramErrorKind::UnsupportedCapability,
                "parser owners cannot bind source closure creation metadata",
            ));
        }
        self.validate_shape()?;
        let mut sites = BTreeMap::new();
        let mut pcs = BTreeSet::new();
        let mut constants = BTreeMap::new();
        let mut hashes = BTreeMap::new();
        let mut child_captures = BTreeMap::new();
        let mut declarations = BTreeMap::new();
        let mut local_proofs = BTreeMap::new();
        let mut edges: BTreeMap<SourceCallbackId, BTreeSet<SourceCallbackId>> = BTreeMap::new();
        for site in &self.sites {
            let parent = program(data, site.callback)?;
            let child_callback = owner
                .closure_prototype(site.prototype)
                .ok_or_else(|| binding("created child prototype is foreign or missing"))?
                .callback;
            let child = program(data, child_callback)?;
            if let Some(prior) = child_captures.insert(child_callback, &site.capture_descriptors)
                && prior != &site.capture_descriptors
            {
                return Err(binding(
                    "creation sites disagree on immutable child capture descriptors",
                ));
            }
            if parent.provenance != site.provenance || child.provenance != site.child_provenance {
                return Err(binding(
                    "closure creation provenance differs from complete parent/child programs",
                ));
            }
            if child_callback == site.callback
                || child.provenance.source.path != parent.provenance.source.path
                || child.provenance.source.line < parent.provenance.source.line
                || child.provenance.source.end_line > parent.provenance.source.end_line
            {
                return Err(binding(
                    "closure creation child is not nested in its parent source",
                ));
            }
            let key = (site.callback, site.expression.start, site.expression.end);
            if sites.insert(key, site).is_some() || !pcs.insert((site.callback, site.bytecode_pc)) {
                return Err(binding(
                    "duplicate closure creation expression or instruction",
                ));
            }
            if let Some(prior) =
                constants.insert((site.callback, site.instruction >> 16), site.prototype)
                && prior != site.prototype
            {
                return Err(binding(
                    "source child constant has conflicting prototype bindings",
                ));
            }
            for (callback, hash) in [
                (site.callback, &site.bytecode_sha256),
                (child_callback, &site.child_bytecode_sha256),
            ] {
                if let Some(prior) = hashes.insert(callback, hash)
                    && prior != hash
                {
                    return Err(binding(
                        "closure creation bytecode hashes disagree for one callback",
                    ));
                }
            }
            let index = declarations
                .entry(site.callback)
                .or_insert_with(|| declaration_index(parent));
            validate_locals(parent, site, index)?;
            for local in &site.local_bindings {
                if let Some(prior) = local_proofs.insert((site.callback, local.local), local)
                    && prior != local
                {
                    return Err(binding(
                        "creation sites disagree on one lexical declaration proof",
                    ));
                }
            }
            edges
                .entry(site.callback)
                .or_default()
                .insert(child_callback);
        }
        acyclic(&edges)?;
        let mut matched = BTreeSet::new();
        let mut mismatch = false;
        for parent in &data.programs {
            constructors::walk::expressions(parent, |expr| {
                if let SourceProgramExprKind::CreateClosure {
                    prototype,
                    captures,
                } = &expr.operation
                {
                    let key = (parent.callback, expr.location.start, expr.location.end);
                    match sites.get(&key) {
                        Some(site)
                            if site.prototype == *prototype && site.captures == *captures =>
                        {
                            if !matched.insert(key) {
                                mismatch = true;
                            }
                        }
                        _ => mismatch = true,
                    }
                }
            });
        }
        if mismatch || matched.len() != sites.len() {
            return Err(binding(
                "closure creation metadata must match every exact CreateClosure expression once",
            ));
        }
        Ok(())
    }
}
fn program(
    data: &SourceProgramData,
    callback: SourceCallbackId,
) -> SourceProgramResult<&SourceProgram> {
    data.callbacks
        .get(&callback)
        .and_then(|id| id.0.checked_sub(1))
        .and_then(|i| data.programs.get(i as usize))
        .filter(|p| p.callback == callback)
        .ok_or_else(|| binding("closure creation callback lacks its complete program"))
}
type DeclarationIndex = BTreeMap<u16, (SourceProgramClosureLocalKind, SourceProgramLocation)>;
fn declaration_index(parent: &SourceProgram) -> DeclarationIndex {
    let mut index = BTreeMap::new();
    let mut stack = vec![parent.body.as_slice()];
    while let Some(body) = stack.pop() {
        for statement in body {
            use SourceProgramStatementKind as S;
            let mut add = |local, kind| {
                index.insert(local, (kind, statement.location));
            };
            match &statement.operation {
                S::Declare { locals, .. } => {
                    for local in locals {
                        add(*local, SourceProgramClosureLocalKind::Declare);
                    }
                }
                S::ForNumeric { local, body, .. } => {
                    add(*local, SourceProgramClosureLocalKind::NumericFor);
                    stack.push(body);
                }
                S::ForEach { locals, body, .. } => {
                    for local in locals {
                        add(*local, SourceProgramClosureLocalKind::GenericFor);
                    }
                    stack.push(body);
                }
                S::If {
                    branches,
                    otherwise,
                } => {
                    stack.push(otherwise);
                    for branch in branches {
                        stack.push(&branch.body);
                    }
                }
                _ => {}
            }
        }
    }
    index
}
fn acyclic(
    edges: &BTreeMap<SourceCallbackId, BTreeSet<SourceCallbackId>>,
) -> SourceProgramResult<()> {
    let mut state = BTreeMap::new();
    for root in edges.keys() {
        if state.get(root) == Some(&2u8) {
            continue;
        }
        let mut work = vec![(*root, false)];
        while let Some((node, exit)) = work.pop() {
            if exit {
                state.insert(node, 2);
                continue;
            }
            match state.get(&node) {
                Some(1) => return Err(binding("cyclic source child-prototype creation graph")),
                Some(2) => continue,
                _ => {}
            }
            state.insert(node, 1);
            work.push((node, true));
            if let Some(children) = edges.get(&node) {
                for child in children.iter().rev() {
                    work.push((*child, false));
                }
            }
        }
    }
    Ok(())
}
fn validate_locals(
    parent: &SourceProgram,
    site: &SourceProgramClosureCreation,
    index: &DeclarationIndex,
) -> SourceProgramResult<()> {
    let mut bindings = BTreeMap::new();
    let mut registers = BTreeSet::new();
    for local in &site.local_bindings {
        if !registers.insert(local.register) {
            return Err(binding(
                "distinct captured local declarations share one source register",
            ));
        }
        if local.declaration.end > parent.provenance.function_end - parent.provenance.function_start
            || bindings.insert(local.local, local).is_some()
        {
            return Err(binding(
                "invalid or duplicate closure local declaration binding",
            ));
        }
        if local.kind == SourceProgramClosureLocalKind::Parameter {
            if local.local >= parent.parameter_count {
                return Err(binding(
                    "closure parameter identity is not a parent parameter",
                ));
            }
        } else if index.get(&local.local) != Some(&(local.kind, local.declaration)) {
            return Err(binding(
                "closure local range/kind differs from its lexical declaration",
            ));
        }
    }
    let mut used = BTreeSet::new();
    for (origin, raw) in site.captures.iter().zip(&site.capture_descriptors) {
        if let SourceProgramCaptureOrigin::Local { local } = origin {
            let local_binding = bindings
                .get(local)
                .ok_or_else(|| binding("captured local has no source declaration proof"))?;
            if u16::from(local_binding.register) != raw & 255 {
                return Err(binding(
                    "captured local source register differs from upvalue descriptor",
                ));
            }
            used.insert(*local);
        }
    }
    if used.len() != bindings.len() {
        return Err(binding("unused closure local declaration proof"));
    }
    Ok(())
}

impl SourceProgramCatalog {
    pub fn new_with_closure_creations(
        data: SourceProgramData,
        owner: SourceProgramOwner,
        constructors: Option<SourceProgramConstructors>,
        creations: SourceProgramClosureCreations,
    ) -> SourceProgramResult<Self> {
        Self::new_with_facets(data, owner, constructors, Some(creations))
    }
    pub fn closure_creations(&self) -> Option<&SourceProgramClosureCreations> {
        self.closure_creations.as_deref()
    }
    pub(super) fn new_with_facets(
        data: SourceProgramData,
        owner: SourceProgramOwner,
        constructors: Option<SourceProgramConstructors>,
        creations: Option<SourceProgramClosureCreations>,
    ) -> SourceProgramResult<Self> {
        let required = crate::modifier_parser::programs::validate::validate(&data, &owner)?;
        if let Some(facet) = &constructors {
            facet.validate_catalog(&data, &owner)?;
        }
        if let Some(facet) = &creations {
            facet.validate_catalog(&data, &owner)?;
            if let Some(tables) = &constructors
                && tables.profile != facet.profile
            {
                return Err(binding(
                    "constructor and closure creation source profiles differ",
                ));
            }
        } else if required.contains(&SourceProgramCapability::ClosureCreation) {
            return Err(binding(
                "source closure creation requires catalog-bound creation metadata",
            ));
        }
        Ok(Self {
            data: ProgramStorage::Authored(Arc::new(data)),
            constructors: constructors.map(Arc::new),
            closure_creations: creations.map(Arc::new),
            owner,
            required,
        })
    }
}
