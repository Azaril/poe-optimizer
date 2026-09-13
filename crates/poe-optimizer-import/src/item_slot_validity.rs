//! Source-ordered finite `IsItemValidForSlot`, over borrowed item owners and
//! explicit caller context. This does not activate equipment or derive actor flags.
mod value;
use crate::item_loading::assembly::AssemblyError;
use poe_optimizer_data::item_assembly::ItemSlotValidityPolicy;
use poe_optimizer_engine::{
    lua_number::parse_number,
    lua_pattern::{
        Capture, CompileLimits, GsubLimits, LuaPattern, MatchBudget, MatchLimits, PatternError,
    },
};
use std::sync::Arc;
pub use value::{Entries, Key, Table, Value};
pub type Result<T> = std::result::Result<T, AssemblyError>;

#[derive(Clone, Copy, Debug)]
pub struct SlotValidityRequest<'a> {
    pub item: Value<'a>,
    pub slot_name: &'a str,
    /// Nil/false selects the context's active set, before any pattern matching.
    pub item_set: Value<'a>,
    pub flag_state: Value<'a>,
}
#[derive(Clone, Copy, Debug)]
pub enum SlotValidityResult<'a> {
    NoValues,
    Value(Value<'a>),
}
impl SlotValidityResult<'_> {
    pub fn truthy(self) -> bool {
        match self {
            Self::NoValues => false,
            Self::Value(v) => v.truthy(),
        }
    }
    pub fn arity(self) -> usize {
        usize::from(matches!(self, Self::Value(_)))
    }
}
/// Dependencies are requested in source order, only when reached. Missing
/// context is an explicit frontier, never inferred absence or an invented flag.
pub trait SlotValidityContext<'a> {
    fn active_item_set(&mut self) -> Result<Value<'a>> {
        unavailable("active item set")
    }
    fn tree_node(&mut self, _key: Value<'_>) -> Result<Value<'a>> {
        unavailable("tree nodes")
    }
    fn effective_node(&mut self, _key: Value<'_>) -> Result<Value<'a>> {
        unavailable("effective nodes")
    }
    fn inventory_item(&mut self, _key: Value<'_>) -> Result<Value<'a>> {
        unavailable("item inventory")
    }
    fn has_calculation_environment(&mut self) -> Result<bool> {
        unavailable("calculation environment presence")
    }
    fn flag(&mut self, _query_name: &str) -> Result<Value<'a>> {
        unavailable("actor flag query")
    }
}
fn unavailable<T>(name: &str) -> Result<T> {
    Err(AssemblyError::unsupported(format!(
        "slot validity requires {name}"
    )))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotValidityLimits {
    pub max_text_bytes: usize,
    pub max_compiled_bytes: usize,
    pub max_steps: u64,
    pub max_backtrack_frames: usize,
}
impl Default for SlotValidityLimits {
    fn default() -> Self {
        Self {
            max_text_bytes: 64 * 1024,
            max_compiled_bytes: 1024 * 1024,
            max_steps: 2_000_000,
            max_backtrack_frames: 256,
        }
    }
}
#[derive(Debug)]
struct Compiled {
    policy: ItemSlotValidityPolicy,
    slot: LuaPattern,
    flask: [(LuaPattern, LuaPattern); 2],
    embedded: LuaPattern,
    parent: LuaPattern,
    limits: SlotValidityLimits,
}
/// Immutable reusable policy and pattern storage; no mutable query state is shared.
#[derive(Clone, Debug)]
pub struct SlotValidityProgram(Arc<Compiled>);
impl SlotValidityProgram {
    pub fn new(policy: &ItemSlotValidityPolicy, limits: SlotValidityLimits) -> Result<Self> {
        policy
            .validate()
            .map_err(|error| AssemblyError::unsupported(error.to_string()))?;
        let mut remaining = limits.max_compiled_bytes;
        let mut compile = |text: &str| {
            let pattern = LuaPattern::compile_with_limits(
                text.as_bytes(),
                CompileLimits {
                    max_pattern_bytes: limits.max_text_bytes,
                    max_compiled_bytes: remaining,
                },
            )
            .map_err(pattern_error)?;
            remaining = remaining
                .checked_sub(pattern.compiled_bytes())
                .ok_or_else(|| AssemblyError::resource("slot-validity aggregate compiled bytes"))?;
            Ok(pattern)
        };
        // Validation bounds the policy clone; all seven retained patterns share
        // one storage limit. Source syntax traps are evaluated only if reached.
        let slot = compile(&policy.slot_pattern)?;
        let flask = [
            (
                compile(&policy.flask.routes[0].base_name_pattern)?,
                compile(&policy.flask.routes[0].slot_name_pattern)?,
            ),
            (
                compile(&policy.flask.routes[1].base_name_pattern)?,
                compile(&policy.flask.routes[1].slot_name_pattern)?,
            ),
        ];
        let embedded = compile(&policy.embedded.slot_pattern)?;
        let parent = compile(&policy.embedded.parent_rewrite.pattern)?;
        Ok(Self(Arc::new(Compiled {
            policy: policy.clone(),
            slot,
            flask,
            embedded,
            parent,
            limits,
        })))
    }
    pub fn policy(&self) -> &ItemSlotValidityPolicy {
        &self.0.policy
    }
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub fn check<'a, C: SlotValidityContext<'a> + ?Sized>(
        &self,
        request: SlotValidityRequest<'a>,
        context: &mut C,
    ) -> Result<SlotValidityResult<'a>> {
        let mut kernel = Kernel {
            compiled: &self.0,
            context,
            budget: MatchBudget::new(MatchLimits {
                max_subject_bytes: self.0.limits.max_text_bytes,
                max_steps: self.0.limits.max_steps,
                max_backtrack_frames: self.0.limits.max_backtrack_frames,
            }),
        };
        kernel.run(request)
    }
}
pub fn is_item_valid_for_slot<'a, C: SlotValidityContext<'a> + ?Sized>(
    policy: &ItemSlotValidityPolicy,
    request: SlotValidityRequest<'a>,
    context: &mut C,
    limits: SlotValidityLimits,
) -> Result<SlotValidityResult<'a>> {
    SlotValidityProgram::new(policy, limits)?.check(request, context)
}
pub fn implementation_sources() -> &'static [&'static str] {
    &[
        include_str!("item_slot_validity.rs"),
        include_str!("item_slot_validity/value.rs"),
    ]
}
struct Kernel<'p, 'c, C: ?Sized> {
    compiled: &'p Compiled,
    context: &'c mut C,
    budget: MatchBudget,
}
impl<'a, C: SlotValidityContext<'a> + ?Sized> Kernel<'_, '_, C> {
    fn charge(&mut self, n: u64) -> Result<()> {
        self.budget.charge(n).map_err(pattern_error)
    }
    fn field(&mut self, value: Value<'a>, key: &str) -> Result<Value<'a>> {
        if key.len() > self.compiled.limits.max_text_bytes {
            return Err(AssemblyError::resource("slot-validity field bytes"));
        }
        let count = match value {
            Value::Table(t) => t.len(),
            _ => 1,
        };
        self.charge(
            (count as u64)
                .saturating_mul((key.len() as u64).saturating_add(1))
                .saturating_add(1),
        )?;
        value.field(key)
    }
    fn tag(&mut self, value: Value<'a>, key: &str) -> Result<Value<'a>> {
        let tags = self.field(value, "tags")?;
        self.field(tags, key)
    }
    fn matches(&mut self, pattern: &LuaPattern, subject: Value<'_>) -> Result<bool> {
        let Value::Text(subject) = subject else {
            return Err(AssemblyError::source(
                "slot validity attempted a string method on a non-string value",
            ));
        };
        Ok(pattern
            .match_captures(subject.as_bytes(), 1, &mut self.budget)
            .map_err(pattern_error)?
            .is_some())
    }
    fn raw_index(&mut self, value: Value<'a>, key: Value<'_>) -> Result<Value<'a>> {
        if let Value::Text(key) = key {
            return self.field(value, key);
        }
        self.charge(match value {
            Value::Table(t) => t.len() as u64 + 1,
            _ => 1,
        })?;
        value.index(key)
    }
    fn run(&mut self, r: SlotValidityRequest<'a>) -> Result<SlotValidityResult<'a>> {
        let p = &self.compiled.policy;
        self.charge(1)?;
        let set = if r.item_set.truthy() {
            r.item_set
        } else {
            self.context.active_item_set()?
        };
        let captures = self
            .compiled
            .slot
            .match_captures(r.slot_name.as_bytes(), 1, &mut self.budget)
            .map_err(pattern_error)?;
        let capture = |index: usize| -> Result<Value<'a>> {
            Ok(
                match captures.as_ref().and_then(|m| m.captures().get(index)) {
                    None => Value::Nil,
                    Some(Capture::Position(n)) => Value::Number(*n as f64),
                    Some(Capture::Bytes { start, end }) => {
                        Value::Text(r.slot_name.get(*start..*end).ok_or_else(|| {
                            AssemblyError::unsupported("slot pattern produced non-UTF8 capture")
                        })?)
                    }
                },
            )
        };
        let slot_type = capture(0)?;
        let slot_type = if slot_type.truthy() {
            slot_type
        } else {
            Value::Text(r.slot_name)
        };
        let slot_id = capture(1)?;
        let item = r.item;
        if eq_text(slot_type, &p.jewel.slot_type) {
            let id = match slot_id {
                Value::Text(s) => {
                    self.charge(s.len() as u64)?;
                    parse_number(s.as_bytes()).map_or(Value::Nil, Value::Number)
                }
                Value::Number(v) => Value::Number(v),
                _ => Value::Nil,
            };
            self.charge(1)?;
            let node = self.context.tree_node(id)?;
            let node = if node.truthy() {
                node
            } else {
                self.charge(1)?;
                self.context.effective_node(id)?
            };
            return self.jewel(item, node).map(ret);
        }
        if eq_text(self.field(item, "type")?, &p.flask.item_type)
            && eq_text(slot_type, &p.flask.slot_type)
        {
            for index in 0..2 {
                let name = self.field(item, "baseName")?;
                if self.matches(&self.compiled.flask[index].0, name)?
                    && self.matches(&self.compiled.flask[index].1, Value::Text(r.slot_name))?
                {
                    return Ok(boolean(true));
                }
            }
            return Ok(SlotValidityResult::NoValues);
        }
        for rule in &p.subtypes {
            let base = self.field(item, "base")?;
            if eq_text(self.field(base, "subType")?, &rule.base_subtype)
                && eq_text(slot_type, &rule.slot_type)
            {
                return Ok(boolean(true));
            }
        }
        if self.field(item, "type")?.same_identity(slot_type) {
            return Ok(boolean(true));
        }
        if eq_text(self.field(item, "type")?, &p.embedded.item_type)
            && self.matches(&self.compiled.embedded, Value::Text(r.slot_name))?
            && !eq_text(self.field(item, "rarity")?, &p.embedded.excluded_rarity)
        {
            let bytes = self
                .compiled
                .parent
                .gsub(
                    r.slot_name.as_bytes(),
                    p.embedded.parent_rewrite.replacement.as_bytes(),
                    None,
                    &mut self.budget,
                    GsubLimits {
                        max_replacement_bytes: self.compiled.limits.max_text_bytes,
                        max_output_bytes: self.compiled.limits.max_text_bytes,
                    },
                )
                .map_err(pattern_error)?
                .bytes;
            let parent = std::str::from_utf8(&bytes).map_err(|_| {
                AssemblyError::unsupported("parent slot rewrite produced non-UTF8 text")
            })?;
            let slot = self.field(set, parent)?;
            if slot.truthy() {
                let selected = self.field(slot, "selItemId")?;
                self.charge(1)?;
                let parent_item = self.context.inventory_item(selected)?;
                if parent_item.truthy() {
                    let restriction = self.field(parent_item, &p.embedded.restriction_field)?;
                    if !restriction.truthy() {
                        return Ok(boolean(true));
                    }
                    let name = self.field(item, "baseName")?;
                    if self.raw_index(restriction, name)?.truthy() {
                        return Ok(boolean(true));
                    }
                }
            }
            return Ok(SlotValidityResult::NoValues);
        }
        for slot in &p.weapon.primary_slots {
            self.charge(1)?;
            if r.slot_name == slot {
                let base = self.field(item, "base")?;
                let first = self.tag(base, &p.weapon.primary_tags[0])?;
                return Ok(ret(if first.truthy() {
                    first
                } else {
                    self.tag(base, &p.weapon.primary_tags[1])?
                }));
            }
        }
        for link in &p.weapon.offhand_slots {
            self.charge(1)?;
            if r.slot_name == link.offhand {
                return self.offhand(r, set, &link.primary);
            }
        }
        Ok(SlotValidityResult::NoValues)
    }
    fn jewel(&mut self, item: Value<'a>, node: Value<'a>) -> Result<Value<'a>> {
        let p = &self.compiled.policy.jewel;
        if !node.truthy() || !eq_text(self.field(item, "type")?, &p.item_type) {
            return Ok(Value::Boolean(false));
        }
        if self.field(node, &p.sinister_field)?.truthy() && self.unique(item, &p.unique_rarities)? {
            return Ok(Value::Boolean(false));
        }
        if self.field(node, &p.contained_socket_field)?.truthy() {
            if self.unique(item, &p.unique_rarities)? {
                return Ok(Value::Boolean(false));
            }
            let base = self.field(item, "base")?;
            if base.truthy() && !matches!(self.field(base, "subType")?, Value::Nil) {
                return Ok(Value::Boolean(false));
            }
            return Ok(Value::Boolean(true));
        }
        let charm = self.field(node, &p.charm_socket_field)?;
        let is_charm = if charm.truthy() {
            true
        } else {
            let base = self.field(item, "base")?;
            eq_text(self.field(base, "subType")?, &p.charm_subtype)
        };
        if is_charm {
            let result = if charm.truthy() {
                let base = self.field(item, "base")?;
                eq_text(self.field(base, "subType")?, &p.charm_subtype)
            } else {
                false
            };
            return Ok(Value::Boolean(result));
        }
        let cluster = self.field(item, &p.cluster_field)?;
        if cluster.truthy() && !self.field(node, &p.expansion_field)?.truthy() {
            return Ok(Value::Boolean(false));
        }
        let expansion = self.field(node, &p.expansion_field)?;
        if !expansion.truthy()
            || eq_number(
                self.field(expansion, &p.expansion_size_field)?,
                p.outer_size,
            )
        {
            return Ok(Value::Boolean(true));
        }
        let cluster = self.field(item, &p.cluster_field)?;
        if !cluster.truthy() {
            return Ok(Value::Boolean(true));
        }
        let size = self.field(cluster, &p.cluster_size_field)?;
        let node_size = self.field(expansion, &p.expansion_size_field)?;
        Ok(Value::Boolean(match (size, node_size) {
            (Value::Number(a), Value::Number(b)) => a <= b,
            (Value::Text(a), Value::Text(b)) => a.as_bytes() <= b.as_bytes(),
            _ => {
                return Err(AssemblyError::source(
                    "slot validity compared incompatible cluster sizes",
                ));
            }
        }))
    }
    fn unique(&mut self, item: Value<'a>, rarities: &[String; 2]) -> Result<bool> {
        Ok(eq_text(self.field(item, "rarity")?, &rarities[0])
            || eq_text(self.field(item, "rarity")?, &rarities[1]))
    }
    fn any_tag(&mut self, base: Value<'a>, keys: &[String]) -> Result<Value<'a>> {
        let mut result = Value::Nil;
        for key in keys {
            result = self.tag(base, key)?;
            if result.truthy() {
                break;
            }
        }
        Ok(result)
    }
    fn offhand(
        &mut self,
        r: SlotValidityRequest<'a>,
        set: Value<'a>,
        primary: &str,
    ) -> Result<SlotValidityResult<'a>> {
        let p = &self.compiled.policy.weapon;
        let selected = self.field(set, primary)?;
        let id = self.field(selected, "selItemId")?;
        let id = if id.truthy() {
            id
        } else {
            Value::Number(p.empty_selection)
        };
        self.charge(1)?;
        let weapon = self.context.inventory_item(id)?;
        let base = if weapon.truthy() {
            self.charge(1)?;
            let weapon = self.context.inventory_item(id)?;
            Some(self.field(weapon, "base")?).filter(|v| v.truthy())
        } else {
            None
        };
        // Missing/false base uses the source string sentinel. Its fixed `.type`
        // lookup is absent in the pinned string library; no policy string escapes.
        let flags = [
            &p.giants_blood,
            &p.instruments_of_power,
            &p.lord_of_the_wilds,
        ];
        let mut values = [
            Value::Boolean(flags[0].default),
            Value::Boolean(flags[1].default),
            Value::Boolean(flags[2].default),
        ];
        if r.flag_state.truthy() {
            for (value, flag) in values.iter_mut().zip(flags) {
                *value = self.field(r.flag_state, &flag.state_field)?;
            }
        } else {
            self.charge(1)?;
            if self.context.has_calculation_environment()? {
                for (value, flag) in values.iter_mut().zip(flags) {
                    self.charge(1)?;
                    *value = self.context.flag(&flag.query_name)?;
                }
            }
        }
        let [giants, instruments, lord] = values;
        let base_type = match base {
            Some(v) => self.field(v, "type")?,
            None => Value::Nil,
        };
        if eq_text(base_type, &p.bow_type) {
            return Ok(boolean(eq_text(
                self.field(r.item, "type")?,
                &p.quiver_type,
            )));
        }
        if eq_text(base_type, &p.talisman_type) && lord.truthy() {
            return Ok(boolean(
                eq_text(self.field(r.item, "type")?, &p.sceptre_type)
                    && !self.unique(r.item, &p.talisman_excluded_rarities)?,
            ));
        }
        if eq_text(base_type, &p.staff_type) && instruments.truthy() {
            return Ok(boolean(eq_text(self.field(r.item, "type")?, &p.focus_type)));
        }
        let unarmed = base.is_none_or(|v| eq_text(v, &p.unarmed_sentinel));
        let eligible = if unarmed {
            true
        } else {
            let b = base.expect("non-sentinel base");
            self.tag(b, &p.onehand_tag)?.truthy()
                || (giants.truthy() && self.any_tag(b, &p.giant_tags)?.truthy())
        };
        if !eligible {
            return Ok(SlotValidityResult::NoValues);
        }
        for kind in &p.ordinary_offhand_types {
            if eq_text(self.field(r.item, "type")?, kind) {
                return Ok(boolean(true));
            }
        }
        let item_base = self.field(r.item, "base")?;
        let dual = self.tag(item_base, &p.dual_wield_tag)?;
        if dual.truthy() {
            let mut allowed = true;
            for kind in &p.excluded_primary_types {
                self.charge(1)?;
                if eq_text(base_type, kind) {
                    allowed = false;
                    break;
                }
            }
            if allowed && !eq_text(self.field(r.item, "type")?, &p.excluded_offhand_type) {
                return Ok(boolean(true));
            }
        }
        Ok(ret(if giants.truthy() {
            self.any_tag(item_base, &p.giant_tags)?
        } else {
            giants
        }))
    }
}
fn ret(value: Value<'_>) -> SlotValidityResult<'_> {
    SlotValidityResult::Value(value)
}
fn boolean(value: bool) -> SlotValidityResult<'static> {
    ret(Value::Boolean(value))
}
fn eq_text(value: Value<'_>, text: &str) -> bool {
    matches!(value,Value::Text(v) if v==text)
}
fn eq_number(value: Value<'_>, number: f64) -> bool {
    matches!(value,Value::Number(v) if v==number)
}
fn pattern_error(error: PatternError) -> AssemblyError {
    match error {
        PatternError::Source(_) => AssemblyError::source(error.to_string()),
        PatternError::Resource(_) => AssemblyError::resource(error.to_string()),
    }
}
