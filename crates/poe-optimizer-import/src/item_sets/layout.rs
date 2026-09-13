use super::*;
use poe_optimizer_engine::lua_pattern::{
    Capture, CompileLimits, LuaPattern, MatchBudget, MatchLimits, PatternError,
};
struct Patterns {
    swap: LuaPattern,
    number: [LuaPattern; 2],
    activation: [LuaPattern; 2],
    budget: MatchBudget,
}
fn error(e: PatternError) -> AssemblyError {
    match e {
        PatternError::Source(e) => AssemblyError::source(e.message()),
        PatternError::Resource(_) => AssemblyError::resource(e.to_string()),
    }
}
impl Patterns {
    fn new(policy: &ItemInventoryPolicy, limits: ItemSetLimits) -> Result<(Self, usize)> {
        let mut remaining = limits.max_compiled_bytes;
        let mut compile = |text: &str| -> Result<LuaPattern> {
            let pattern = LuaPattern::compile_with_limits(
                text.as_bytes(),
                CompileLimits {
                    max_pattern_bytes: limits.max_string_bytes,
                    max_compiled_bytes: remaining,
                },
            )
            .map_err(error)?;
            remaining = remaining
                .checked_sub(pattern.compiled_bytes())
                .ok_or_else(|| AssemblyError::resource("item-set compiled pattern bound"))?;
            Ok(pattern)
        };
        let swap = compile(&policy.layout.swap.slot_pattern)?;
        let number = [
            compile(&policy.layout.slot_number_patterns[0])?,
            compile(&policy.layout.slot_number_patterns[1])?,
        ];
        let activation = [
            compile(&policy.layout.activation_patterns[0])?,
            compile(&policy.layout.activation_patterns[1])?,
        ];
        Ok((
            Self {
                swap,
                number,
                activation,
                budget: MatchBudget::new(MatchLimits {
                    max_subject_bytes: limits.max_string_bytes,
                    max_steps: limits.max_steps,
                    max_backtrack_frames: 256,
                }),
            },
            limits.max_compiled_bytes - remaining,
        ))
    }
}
impl ItemSetState {
    pub(super) fn construct_layout(&mut self) -> Result<()> {
        let policy = Arc::clone(&self.policy);
        let (mut p, bytes) = Patterns::new(&policy, self.graph.limits)?;
        self.graph.charge(bytes, 1)?;
        for name in &policy.layout.base_slots {
            let parent = self.add_slot(name, None, &mut p)?;
            let swapped = p
                .swap
                .match_captures(name.as_bytes(), 1, &mut p.budget)
                .map_err(error)?
                .is_some();
            let swap = if swapped {
                self.graph.set_field(
                    parent,
                    "weaponSet",
                    V::Number(f64::from(policy.layout.swap.primary_weapon_set)),
                )?;
                let swap = self.add_slot(
                    &format!("{name}{}", policy.layout.swap.suffix),
                    None,
                    &mut p,
                )?;
                self.graph.set_field(
                    swap,
                    "weaponSet",
                    V::Number(f64::from(policy.layout.swap.alternate_weapon_set)),
                )?;
                Some(swap)
            } else {
                None
            };
            if policy.layout.embedded.parent_slots.contains(name) {
                self.add_children(parent, name, &mut p)?;
                if let Some(swap) = swap {
                    self.add_children(
                        swap,
                        &format!("{name}{}", policy.layout.swap.suffix),
                        &mut p,
                    )?;
                }
            }
        }
        for rune in &policy.layout.rune_slots {
            let row = self.graph.table()?;
            let name = self.graph.text(&policy.defaults.empty_rune_name)?;
            self.graph.set_field(row, "selected_name", name)?;
            self.graph
                .set_field(self.runes, &rune.name, V::Table(row))?;
        }
        for id in &policy.layout.passive.nodes.ids {
            self.add_slot(
                &format!("{}{id}", policy.layout.passive.slot_prefix),
                Some(*id),
                &mut p,
            )?;
        }
        self.graph.usage.pattern_steps = p.budget.steps_used();
        self.graph.charge(0, p.budget.steps_used())?;
        Ok(())
    }
    fn add_children(&mut self, parent: Id, name: &str, p: &mut Patterns) -> Result<()> {
        let policy = Arc::clone(&self.policy);
        let children = table(self.graph.field(parent, "jewelSocketList")?)?;
        for i in 1..=policy.layout.embedded.count {
            let child = self.add_slot(
                &format!("{name}{}{i}", policy.layout.embedded.name_infix),
                None,
                p,
            )?;
            self.graph
                .set_field(child, "parentSlot", V::Table(parent))?;
            let weapon = self.graph.field(parent, "weaponSet")?;
            self.graph.set_field(child, "weaponSet", weapon)?;
            // Fresh constructor PopulateSlots sees empty inventory and zero
            // parent selection. This is a proved initial value, not activation.
            self.graph.set_field(child, "inactive", V::Boolean(true))?;
            self.graph.append(children, V::Table(child))?;
        }
        Ok(())
    }
    fn add_slot(&mut self, name: &str, node: Option<u32>, p: &mut Patterns) -> Result<Id> {
        if self.ordered_slots.len() >= self.graph.limits.max_slots {
            return Err(AssemblyError::resource("item-set slot bound"));
        }
        let policy = Arc::clone(&self.policy);
        let slot = self.graph.table()?;
        let text = self.graph.text(name)?;
        self.graph.set_field(slot, "slotName", text)?;
        self.graph
            .set_field(slot, "selItemId", V::Number(policy.defaults.empty_item_id))?;
        if let Some(node) = node {
            self.graph
                .set_field(slot, "nodeId", V::Number(f64::from(node)))?;
        }
        let mut matched = p.number[0]
            .match_captures(name.as_bytes(), 1, &mut p.budget)
            .map_err(error)?;
        if matched.is_none() {
            matched = p.number[1]
                .match_captures(name.as_bytes(), 1, &mut p.budget)
                .map_err(error)?;
        }
        if let Some(m) = matched {
            let value = match m.captures().first() {
                Some(Capture::Position(i)) => Some(*i as f64),
                Some(Capture::Bytes { start, end }) => parse_number(&name.as_bytes()[*start..*end]),
                None => parse_number(&name.as_bytes()[m.range()]),
            };
            if let Some(value) = value {
                self.graph.set_field(slot, "slotNum", V::Number(value))?;
            }
        }
        let first = p.activation[0]
            .match_captures(name.as_bytes(), 1, &mut p.budget)
            .map_err(error)?
            .is_some();
        if first
            || p.activation[1]
                .match_captures(name.as_bytes(), 1, &mut p.budget)
                .map_err(error)?
                .is_some()
        {
            let activate = self.graph.table()?;
            self.graph.set_field(slot, "activate", V::Table(activate))?;
        }
        let children = self.graph.table()?;
        self.graph
            .set_field(slot, "jewelSocketList", V::Table(children))?;
        self.graph.charge(std::mem::size_of::<Id>(), 1)?;
        self.ordered_slots.push(slot);
        self.graph.set_field(self.slots, name, V::Table(slot))?;
        Ok(slot)
    }
}
