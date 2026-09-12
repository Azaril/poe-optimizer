//! Optional private bytecode observation for source-authenticated constructor sites.
//!
//! The caller attests the linked LuaJIT build configuration. Public reflection
//! checks observable properties but cannot prove compile-time flags such as table
//! bumping or numeric mode. No bytecode or live Function escapes into shared data.
use super::*;
use mlua::MultiValue;
use sha2::{Digest, Sha256};

pub(super) mod diagnostic;

const MAX_BYTECODES: u32 = 65_536;
const TNEW: u32 = 52;
const TDUP: u32 = 53;

pub(super) struct Reflection {
    profile: SourceTableRuntimeProfile,
    info: Function,
    bytecode: Function,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Instruction {
    pub(crate) pc: u32,
    pub(crate) word: u32,
    pub(crate) line: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallbackObservation {
    pub(crate) source: ItemSourceSpan,
    pub(crate) sha256: String,
    pub(crate) constructors: Vec<Instruction>,
    pub(crate) unsupported: Option<String>,
}
#[derive(Default)]
pub(super) struct Pending {
    pub(super) callbacks: BTreeMap<SourceCallbackId, CallbackObservation>,
    diagnostic_bindings: Vec<diagnostic::Binding>,
}
/// Owner-bound evidence produced only while observing actual original functions.
/// Source locations are mapped to lowered expressions separately. This is not
/// numerical admission and cannot be manufactured by supplying a function name.
#[derive(Debug, Clone)]
pub struct ObservedSourceConstructors {
    owner: SourceProgramOwner,
    pub(crate) profile: SourceTableRuntimeProfile,
    pub(crate) callbacks: BTreeMap<SourceCallbackId, CallbackObservation>,
    diagnostic_bindings: Vec<diagnostic::Binding>,
}
impl ObservedSourceConstructors {
    pub(crate) fn validate_owner(&self, owner: &SourceProgramOwner) -> Result<()> {
        if !self.owner.is_same_owner(owner) {
            return Err(error(
                "constructor observation belongs to another source owner",
            ));
        }
        Ok(())
    }
    pub fn profile(&self) -> &SourceTableRuntimeProfile {
        &self.profile
    }
}
impl Reflection {
    pub(super) fn capture(lua: &Lua, profile: SourceTableRuntimeProfile) -> Result<Self> {
        if !profile.is_supported_array_profile() {
            return Err(error(
                "unsupported attested source constructor runtime profile",
            ));
        }
        let jit: Table = lua.globals().raw_get("jit")?;
        plain(&jit, "original JIT library")?;
        let arch: String = jit.raw_get("arch")?;
        let version: String = jit.raw_get("version")?;
        let version_num: u32 = jit.raw_get("version_num")?;
        if arch != "x64" || version_num != 20199 || !version.starts_with("LuaJIT 2.1.") {
            return Err(error(
                "observed JIT runtime differs from constructor build attestation",
            ));
        }
        // Invoke the retained original anonymous C module loader directly. Unlike
        // require, this neither trusts a mutable package.loaded entry nor installs
        // a public debug/global/module binding. Fresh-host ownership is required.
        let preload: Table = lua.named_registry_value("_PRELOAD")?;
        let loader: Function = preload.raw_get("jit.util")?;
        if loader.info().what != "C" {
            return Err(error(
                "constructor reflection loader is not original C code",
            ));
        }
        let utility: Table = loader.call(())?;
        plain(&utility, "private constructor reflection")?;
        let info: Function = utility.raw_get("funcinfo")?;
        let bytecode: Function = utility.raw_get("funcbc")?;
        if info.info().what != "C" || bytecode.info().what != "C" {
            return Err(error(
                "constructor reflection primitives are not original C code",
            ));
        }
        Ok(Self {
            profile,
            info,
            bytecode,
        })
    }
    fn observe(
        &self,
        function: &Function,
        source: &ItemSourceSpan,
        remaining: usize,
        authenticated_children: bool,
    ) -> Result<(CallbackObservation, usize)> {
        let info: Table = self.info.call(function.clone())?;
        let count: u32 = info.raw_get("bytecodes")?;
        if count == 0 || count > MAX_BYTECODES || count as usize > remaining {
            return Err(error("constructor bytecode observation bound"));
        }
        let first: u32 = info.raw_get("linedefined")?;
        let last: u32 = info.raw_get("lastlinedefined")?;
        if first != source.line || last != source.end_line {
            return Err(error(
                "constructor bytecode source span differs from observed callback",
            ));
        }
        let children: bool = info.raw_get("children")?;
        let frame: u32 = info.raw_get("stackslots")?;
        if frame > 255 {
            return Err(error("constructor observed frame bound"));
        }
        let mut digest = Sha256::new();
        digest.update(b"poe-source-bytecode-v1\0");
        digest.update(count.to_le_bytes());
        let mut constructors = Vec::new();
        let mut unsupported = (children && !authenticated_children)
            .then(|| "nested function bytecode requires authenticated child inventory".into());
        // Header PC0 can change with JIT execution mode; source instructions are
        // PCs1..count. We hash their actual current words, never a recompiled body.
        for pc in 1..count {
            let words: MultiValue = self.bytecode.call((function.clone(), pc))?;
            if words.len() != 2 {
                return Err(error("constructor reflection returned incomplete bytecode"));
            }
            let signed = match &words[0] {
                Value::Integer(word) => i32::try_from(*word).map_err(error)?,
                Value::Number(word)
                    if word.is_finite()
                        && word.fract() == 0.0
                        && (i32::MIN as f64..=i32::MAX as f64).contains(word) =>
                {
                    *word as i32
                }
                _ => return Err(error("constructor reflection returned invalid instruction")),
            };
            let word = signed as u32;
            digest.update(word.to_le_bytes());
            if matches!(word & 255, TNEW | TDUP) {
                let info: Table = self.info.call((function.clone(), pc))?;
                let line: u32 = info.raw_get("currentline")?;
                if (word >> 8) & 255 >= frame {
                    unsupported =
                        Some("constructor destination is outside actual prototype frame".into());
                }
                if line < source.line || line > source.end_line {
                    unsupported =
                        Some("constructor instruction lacks a matching source line".into());
                }
                constructors.push(Instruction { pc, word, line });
            }
        }
        if !self
            .bytecode
            .call::<MultiValue>((function.clone(), count))?
            .is_empty()
        {
            return Err(error(
                "constructor bytecode inventory changed during observation",
            ));
        }
        Ok((
            CallbackObservation {
                source: source.clone(),
                sha256: format!("{:x}", digest.finalize()),
                constructors,
                unsupported,
            },
            count as usize,
        ))
    }
}
impl SourceClosureObserver {
    /// Opt into actual constructor bytecode observation. `build_attestation` is
    /// an explicit trusted claim about the linked host build, including flags
    /// unavailable to Lua reflection; a matching version label alone is no proof.
    /// Call before loading any untrusted or instrumented source.
    pub fn capture_before_source_with_constructors(
        lua: &Lua,
        build_attestation: SourceTableRuntimeProfile,
    ) -> Result<Self> {
        let mut observer = Self::capture_before_source(lua)?;
        observer.constructor_reflection = Some(Reflection::capture(lua, build_attestation)?);
        Ok(observer)
    }
    pub(super) fn bind_constructors(
        &self,
        owner: &SourceProgramOwner,
        pending: Pending,
    ) -> Option<ObservedSourceConstructors> {
        self.constructor_reflection
            .as_ref()
            .map(|reflection| ObservedSourceConstructors {
                owner: owner.clone(),
                profile: reflection.profile.clone(),
                callbacks: pending.callbacks,
                diagnostic_bindings: pending.diagnostic_bindings,
            })
    }
}
impl Graph<'_> {
    pub(super) fn observe_constructors(
        &mut self,
        id: SourceCallbackId,
        function: &Function,
        source: &ItemSourceSpan,
    ) -> Result<()> {
        self.observe_closure_creation(id, function, source)?;
        if self.observer.constructor_reflection.is_none() {
            return Ok(());
        }
        // Source-span clone, digest and a bounded diagnostic remain inside the
        // same aggregate text budget as the observed definition graph.
        self.text(source.path.len())?;
        self.text(source.sha256.len())?;
        self.text(64 + 128)?;
        let reflection = self
            .observer
            .constructor_reflection
            .as_ref()
            .expect("enabled");
        let (observation, count) = reflection.observe(
            function,
            source,
            MAX_VALUES.saturating_sub(self.values),
            self.observer.closure_reflection.is_some(),
        )?;
        self.values += count;
        self.bind_constructor_diagnostic_targets(id, function)?;
        if let Some(previous) = self.constructor_observations.callbacks.get(&id) {
            if previous != &observation {
                return Err(error(
                    "interned source prototype has differing actual constructor bytecode",
                ));
            }
        } else {
            self.constructor_observations
                .callbacks
                .insert(id, observation);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
