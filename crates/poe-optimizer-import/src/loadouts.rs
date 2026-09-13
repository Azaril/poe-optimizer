//! Native source-format loadout lookup over a borrowed live state.
//!
//! This is the read-only GetSpecList/GetLoadoutByName boundary, not SyncLoadouts
//! or activation. Caller intent, saved selections and live owners remain distinct.
//! The context must read actual private domain state; diagnostic snapshots are not
//! a production implementation. Returned numeric IDs acquire authored-instance
//! meaning only when the subsequent activation resolves them in that same owner.
use crate::selected_view::{NumericValue, SelectionDomain};
use poe_optimizer_data::loadouts::BuildLoadoutPolicy;
use poe_optimizer_engine::lua_pattern::{
    Capture, CompileLimits, LuaPattern, MatchBudget, MatchLimits, PatternError,
};
use std::{fmt, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadoutErrorKind {
    Source,
    Resource,
    Unavailable,
    Contract,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadoutError {
    pub kind: LoadoutErrorKind,
    pub message: String,
}
impl LoadoutError {
    pub fn new(kind: LoadoutErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
impl fmt::Display for LoadoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for LoadoutError {}
pub type Result<T> = std::result::Result<T, LoadoutError>;
fn resource(message: &'static str) -> LoadoutError {
    LoadoutError::new(LoadoutErrorKind::Resource, message)
}
fn source(message: &'static str) -> LoadoutError {
    LoadoutError::new(LoadoutErrorKind::Source, message)
}
fn pattern_error(e: PatternError) -> LoadoutError {
    LoadoutError::new(
        match e {
            PatternError::Source(_) => LoadoutErrorKind::Source,
            PatternError::Resource(_) => LoadoutErrorKind::Resource,
        },
        e.to_string(),
    )
}

/// Source fields admitted by the imported-format adapter: text or absent, not
/// arbitrary mutable Lua objects. Empty text remains distinct from absence.
#[derive(Debug, Clone, Copy)]
pub struct LoadoutSpec<'a> {
    pub title: Option<&'a str>,
    pub tree_version: Option<&'a str>,
}
#[derive(Debug, Clone, Copy)]
pub enum LoadoutLink<'a> {
    Bytes(&'a [u8]),
    Position(usize),
}

/// Immutable borrow of current domain owners for one complete call. Accessors
/// may expose a reached source error or an unavailable producer. They must not
/// compute final imported state, mutate selections, or call a reference backend.
///
/// `is_singleton` proves source `#order == 1`; it is not the length of an ipairs
/// prefix or a vector containing holes. Unproved sparse-array lengths are an
/// explicit Unavailable result. `ordered_set` reads raw one-based positions;
/// None ends ipairs, independently of the singleton result. Missing reached
/// set/link rows are Source errors; a present link row without setId returns None.
/// Numeric keys are retained exactly, including zero, fractional and infinite IDs.
pub trait LiveLoadoutContext {
    fn is_singleton(&self, domain: SelectionDomain) -> Result<bool>;
    fn spec_prefix_len(&self) -> Result<usize>;
    fn spec(&self, position: usize) -> Result<LoadoutSpec<'_>>;
    fn ordered_set(&self, domain: SelectionDomain, position: usize)
    -> Result<Option<NumericValue>>;
    fn set_title(&self, domain: SelectionDomain, key: NumericValue) -> Result<Option<&str>>;
    fn linked_set(
        &self,
        domain: SelectionDomain,
        link: LoadoutLink<'_>,
    ) -> Result<Option<NumericValue>>;
}

#[derive(Debug, Clone, Copy)]
pub struct LoadoutLimits {
    pub max_specs: usize,
    pub max_set_reads: usize,
    pub max_text_bytes: usize,
    pub max_output_bytes: usize,
    pub max_steps: u64,
    pub max_compiled_bytes: usize,
}
impl Default for LoadoutLimits {
    fn default() -> Self {
        Self {
            max_specs: 32768,
            max_set_reads: 131072,
            max_text_bytes: 262144,
            max_output_bytes: 8 * 1024 * 1024,
            max_steps: 5_000_000,
            max_compiled_bytes: 1024 * 1024,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct LoadoutIds {
    pub spec: Option<NumericValue>,
    pub items: Option<NumericValue>,
    pub skills: Option<NumericValue>,
    pub configuration: Option<NumericValue>,
}
/// A numeric request bound to the exact program and borrowed live owner. It does
/// not certify that each returned ID still has a winner, or grant build legality.
pub struct LoadoutSelection<'a, C: ?Sized> {
    context: &'a C,
    program: &'a LoadoutProgram,
    ids: LoadoutIds,
}
impl<C: ?Sized> LoadoutSelection<'_, C> {
    pub fn ids(&self) -> LoadoutIds {
        self.ids
    }
    pub fn belongs_to(&self, context: &C, program: &LoadoutProgram) -> bool {
        std::ptr::eq(self.context, context) && std::ptr::eq(self.program, program)
    }
}

pub struct LoadoutProgram {
    policy: Arc<BuildLoadoutPolicy>,
    link_pattern: LuaPattern,
    limits: LoadoutLimits,
}
struct Work {
    limits: LoadoutLimits,
    pattern: MatchBudget,
    output_bytes: usize,
    set_reads: usize,
}
impl Work {
    fn new(limits: LoadoutLimits) -> Self {
        Self {
            limits,
            pattern: MatchBudget::new(MatchLimits {
                max_subject_bytes: limits.max_text_bytes,
                max_steps: limits.max_steps,
                ..MatchLimits::default()
            }),
            output_bytes: 0,
            set_reads: 0,
        }
    }
    fn step(&mut self) -> Result<()> {
        self.pattern.charge(1).map_err(pattern_error)
    }
    fn text(&mut self, text: &str) -> Result<()> {
        if text.len() > self.limits.max_text_bytes {
            return Err(resource("loadout text bound"));
        }
        self.pattern
            .charge(text.len() as u64)
            .map_err(pattern_error)
    }
    fn output(&mut self, bytes: usize) -> Result<()> {
        self.output_bytes = self
            .output_bytes
            .checked_add(bytes)
            .ok_or_else(|| resource("loadout output overflow"))?;
        if self.output_bytes > self.limits.max_output_bytes {
            return Err(resource("loadout output bound"));
        }
        Ok(())
    }
    fn order<C: LiveLoadoutContext + ?Sized>(
        &mut self,
        context: &C,
        domain: SelectionDomain,
        position: usize,
    ) -> Result<Option<NumericValue>> {
        self.step()?;
        self.set_reads = self
            .set_reads
            .checked_add(1)
            .ok_or_else(|| resource("loadout order overflow"))?;
        if self.set_reads > self.limits.max_set_reads {
            return Err(resource("loadout order bound"));
        }
        context.ordered_set(domain, position)
    }
}
impl LoadoutProgram {
    pub fn new(policy: Arc<BuildLoadoutPolicy>, limits: LoadoutLimits) -> Result<Self> {
        policy
            .validate()
            .map_err(|e| LoadoutError::new(LoadoutErrorKind::Contract, e.to_string()))?;
        // The shared pattern compiler retains malformed-source instructions;
        // syntax errors remain lazy until the original match would reach them.
        let link_pattern = LuaPattern::compile_with_limits(
            policy.single_link_pattern.as_bytes(),
            CompileLimits {
                max_pattern_bytes: limits.max_text_bytes,
                max_compiled_bytes: limits.max_compiled_bytes,
            },
        )
        .map_err(pattern_error)?;
        Ok(Self {
            policy,
            link_pattern,
            limits,
        })
    }
    pub fn policy(&self) -> &Arc<BuildLoadoutPolicy> {
        &self.policy
    }
    /// One freshly allocated display list, corresponding to one source return.
    pub fn spec_list<C: LiveLoadoutContext + ?Sized>(&self, context: &C) -> Result<Vec<String>> {
        self.specs(context, &mut Work::new(self.limits))
    }
    fn specs<C: LiveLoadoutContext + ?Sized>(
        &self,
        context: &C,
        work: &mut Work,
    ) -> Result<Vec<String>> {
        work.step()?;
        let count = context.spec_prefix_len()?;
        if count > self.limits.max_specs {
            return Err(resource("loadout spec bound"));
        }
        work.output(
            count
                .checked_mul(std::mem::size_of::<String>())
                .ok_or_else(|| resource("loadout spec storage overflow"))?,
        )?;
        let mut result = Vec::with_capacity(count);
        for position in 1..=count {
            work.step()?;
            let spec = context.spec(position)?;
            let mut display = String::new();
            // Match source evaluation order: version lookup/decorating precedes title.
            // Even a current-version equality reads the compared text.
            if let Some(version) = spec.tree_version {
                work.text(version)?;
                work.text(&self.policy.latest_tree_version)?;
            }
            if spec.tree_version != Some(self.policy.latest_tree_version.as_str()) {
                let version = spec
                    .tree_version
                    .ok_or_else(|| source("GetSpecList: missing tree version display"))?;
                let label = self
                    .policy
                    .tree_version_display
                    .get(version)
                    .ok_or_else(|| source("GetSpecList: missing tree version display"))?;
                for text in [
                    &self.policy.version_prefix,
                    label,
                    &self.policy.version_suffix,
                ] {
                    work.text(text)?;
                    work.output(text.len())?;
                    display.push_str(text);
                }
            }
            let title = spec.title.unwrap_or(&self.policy.default_title);
            work.text(title)?;
            work.output(title.len())?;
            display.push_str(title);
            result.push(display);
        }
        Ok(result)
    }
    fn link<'a>(&self, name: &'a str, work: &mut Work) -> Result<Option<LoadoutLink<'a>>> {
        let Some(found) = self
            .link_pattern
            .match_captures(name.as_bytes(), 1, &mut work.pattern)
            .map_err(pattern_error)?
        else {
            return Ok(None);
        };
        match found.captures().first() {
            Some(Capture::Bytes { start, end }) => {
                Ok(Some(LoadoutLink::Bytes(&name.as_bytes()[*start..*end])))
            }
            Some(Capture::Position(position)) => Ok(Some(LoadoutLink::Position(*position))),
            None => Err(LoadoutError::new(
                LoadoutErrorKind::Contract,
                "match returned no first capture",
            )),
        }
    }
    fn find_tree<C: LiveLoadoutContext + ?Sized>(
        &self,
        context: &C,
        names: &[String],
        name: &str,
        work: &mut Work,
    ) -> Result<Option<NumericValue>> {
        for (index, display) in names.iter().enumerate() {
            work.step()?;
            work.text(display)?;
            work.text(name)?;
            if name == display {
                return Ok(Some(NumericValue::new((index + 1) as f64)));
            }
            if let Some(link) = self.link(name, work)? {
                work.step()?;
                return context.linked_set(SelectionDomain::Passives, link);
            }
        }
        Ok(None)
    }
    fn find_set<C: LiveLoadoutContext + ?Sized>(
        &self,
        context: &C,
        domain: SelectionDomain,
        singleton: bool,
        name: &str,
        work: &mut Work,
    ) -> Result<Option<NumericValue>> {
        if singleton && let Some(key) = work.order(context, domain, 1)? {
            return Ok(Some(key));
        }
        let mut position = 1;
        while let Some(key) = work.order(context, domain, position)? {
            work.step()?;
            let title = context
                .set_title(domain, key)?
                .unwrap_or(&self.policy.default_title);
            work.text(title)?;
            work.text(name)?;
            if name == title {
                return Ok(Some(key));
            }
            if let Some(link) = self.link(name, work)? {
                work.step()?;
                return context.linked_set(domain, link);
            }
            position += 1; // bounded by the cumulative order-read limit
        }
        Ok(None)
    }
    /// One source result: None means one nil, Some means one possibly partial ID
    /// table. All four domains are resolved before testing whether the table is empty.
    pub fn lookup<'a, C: LiveLoadoutContext + ?Sized>(
        &'a self,
        context: &'a C,
        name: &str,
    ) -> Result<Option<LoadoutSelection<'a, C>>> {
        let mut work = Work::new(self.limits);
        // Source computes these probes before invoking GetSpecList, in this order.
        work.step()?;
        let one_skill = context.is_singleton(SelectionDomain::Skills)?;
        work.step()?;
        let one_item = context.is_singleton(SelectionDomain::Items)?;
        work.step()?;
        let one_config = context.is_singleton(SelectionDomain::Configuration)?;
        let names = self.specs(context, &mut work)?;
        let spec = self.find_tree(context, &names, name, &mut work)?;
        let items = self.find_set(context, SelectionDomain::Items, one_item, name, &mut work)?;
        let skills = self.find_set(context, SelectionDomain::Skills, one_skill, name, &mut work)?;
        let configuration = self.find_set(
            context,
            SelectionDomain::Configuration,
            one_config,
            name,
            &mut work,
        )?;
        if spec.is_none() && items.is_none() && skills.is_none() && configuration.is_none() {
            return Ok(None);
        }
        Ok(Some(LoadoutSelection {
            context,
            program: self,
            ids: LoadoutIds {
                spec,
                items,
                skills,
                configuration,
            },
        }))
    }
}
