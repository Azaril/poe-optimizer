//! Optional test-only evidence for the argument of an exact original unpack call.
//! The expected return pack is derived from the pinned primitive and its observed
//! caller argument. It is not an intercepted return or a TSETM execution snapshot.
use mlua::{Function, Lua, Table, Value};

const MAX_EVENTS: usize = 256;
const MAX_VALUES: usize = 4096;
const MAX_TEXT: usize = 65_536;
const INSPECT: &str = r#"local getinfo,getlocal = ...
return function(callee, target, sourceLine, argumentLocal, constructorSlot)
    for level=1,256 do
        local info=getinfo(level,'f')
        if not info then return nil end
        if info.func == callee then
            local caller=getinfo(level+1,'fl')
            if not caller or caller.func ~= target or caller.currentline ~= sourceLine then
                return nil
            end
            local row={caller=caller.func,sourceLine=caller.currentline}
            for slot=1,256 do
                local name,value=getlocal(level+1,slot)
                if not name then break end
                if slot == 256 then error('producer tail local bound') end
                if name == argumentLocal then
                    if row.argumentSlot then error('producer tail ambiguous argument local') end
                    row.argumentSlot=slot; row.argument=value
                end
            end
            row.constructorName,row.constructor=getlocal(level+1,constructorSlot)
            local depth=0
            for parent=level+1,256 do
                local ancestor=getinfo(parent,'f')
                if not ancestor then row.depth=depth; return row end
                if ancestor.func == target then
                    depth=depth+1
                    if depth > 32 then error('producer tail depth bound') end
                end
            end
            error('producer tail stack scan bound')
        end
    end
    error('producer tail stack scan bound')
end"#;
const INSPECT_ENTRY: &str = r#"local getinfo,getlocal = ...
return function(target, constructorSlot)
    for level=1,256 do
        local info=getinfo(level,'f')
        if not info then error('producer tail entry frame missing') end
        if info.func == target then
            local _,constructor=getlocal(level,constructorSlot)
            local depth=0
            for parent=level,256 do
                local ancestor=getinfo(parent,'f')
                if not ancestor then return constructor,depth end
                if ancestor.func == target then
                    depth=depth+1
                    if depth > 32 then error('producer tail entry depth bound') end
                end
            end
            error('producer tail entry stack scan bound')
        end
    end
    error('producer tail entry stack scan bound')
end"#;
fn error(message: &str) -> mlua::Error {
    mlua::Error::RuntimeError(message.into())
}
#[derive(Debug, Clone, Copy)]
pub struct ProducerTailLimits {
    pub max_events: usize,
    pub max_values: usize,
    pub max_text_bytes: usize,
}
impl Default for ProducerTailLimits {
    fn default() -> Self {
        Self {
            max_events: 64,
            max_values: 1024,
            max_text_bytes: MAX_TEXT,
        }
    }
}
#[derive(Debug, Clone)]
pub struct ProducerTailConfig {
    pub source_line: usize,
    pub argument_local: String,
    /// One-based debug slot; the caller proves its relation to allocation A.
    pub constructor_slot: usize,
    pub limits: ProducerTailLimits,
}
#[derive(Debug, Clone)]
pub struct ProducerTailBinding {
    pub config: ProducerTailConfig,
    pub original_unpack: Function,
}
#[derive(Debug)]
pub struct ProducerTailLineEntry {
    pub ordinal: usize,
    pub observed_event: usize,
    pub activation: usize,
    pub source_line: usize,
    pub caller_depth: usize,
    pub constructor_slot: usize,
    pub constructor: Table,
    pub environment: Table,
    pub original_unpack: Function,
    pub consumed_by_tail: Option<usize>,
    pub abandoned: bool,
    pub post_call_same_line_events: usize,
}
#[derive(Debug)]
pub struct ProducerTailCall {
    pub line_entry_ordinal: usize,
    pub ordinal: usize,
    pub observed_event: usize,
    pub activation: usize,
    pub caller: Function,
    pub callee: Function,
    pub source_line: usize,
    pub caller_depth: usize,
    pub argument_local: String,
    pub argument_slot: usize,
    pub argument: Table,
    pub constructor_slot: usize,
    pub constructor_name: String,
    pub constructor: Table,
    pub raw_length: usize,
    /// Exactly raw_length values, including nil; derived from original unpack.
    pub values: Vec<Value>,
}
#[derive(Clone)]
pub struct ProducerTailProbe {
    original_unpack: Option<Function>,
    inspect: Function,
    inspect_entry: Function,
}
impl ProducerTailProbe {
    pub fn before_source(lua: &Lua, getinfo: &Function, getlocal: &Function) -> mlua::Result<Self> {
        // Retain before source without introducing a new prerequisite for
        // callers that never enable the optional tail witness.
        let original_unpack = match lua.globals().raw_get::<Value>("unpack")? {
            Value::Function(function) if function.info().what == "C" => Some(function),
            _ => None,
        };
        let inspect = lua
            .load(INSPECT)
            .set_name("@tests/support/source_program_producer_tail.rs#inspection")
            .call((getinfo.clone(), getlocal.clone()))?;
        let inspect_entry = lua
            .load(INSPECT_ENTRY)
            .set_name("@tests/support/source_program_producer_tail.rs#entry")
            .call((getinfo.clone(), getlocal.clone()))?;
        Ok(Self {
            original_unpack,
            inspect,
            inspect_entry,
        })
    }
    fn validate_config(config: &ProducerTailConfig) -> mlua::Result<()> {
        if config.source_line == 0
            || config.argument_local.is_empty()
            || config.argument_local.len() > 256
            || !(1..256).contains(&config.constructor_slot)
            || !(1..=MAX_EVENTS).contains(&config.limits.max_events)
            || config.limits.max_values > MAX_VALUES
            || config.limits.max_text_bytes > MAX_TEXT
        {
            return Err(error("producer tail invalid configuration/bounds"));
        }
        Ok(())
    }
    pub fn verify(&self, binding: &ProducerTailBinding) -> mlua::Result<()> {
        // Bindings are visible to the test reporter. Recheck borrowed fields
        // before cloning them or installing a hook, even after caller mutation.
        Self::validate_config(&binding.config)?;
        if self.original_unpack.as_ref() != Some(&binding.original_unpack) {
            return Err(error("producer tail original unpack identity changed"));
        }
        Ok(())
    }
    pub fn bind(&self, config: ProducerTailConfig) -> mlua::Result<ProducerTailBinding> {
        Self::validate_config(&config)?;
        let original_unpack = self
            .original_unpack
            .clone()
            .ok_or_else(|| error("producer tail requires retained original C unpack"))?;
        Ok(ProducerTailBinding {
            config,
            original_unpack,
        })
    }
    fn checked_environment(
        target: &Function,
        binding: &ProducerTailBinding,
    ) -> mlua::Result<Table> {
        let environment = target
            .environment()
            .ok_or_else(|| error("producer tail target environment missing"))?;
        if environment.metatable().is_some()
            || environment.raw_get::<Value>("unpack")?
                != Value::Function(binding.original_unpack.clone())
        {
            return Err(error(
                "producer tail requires plain environment and raw original unpack",
            ));
        }
        Ok(environment)
    }
    pub fn abandon_pending(state: &mut super::ProducerState) {
        if let Some(index) = state.tail_pending.take() {
            state.tail_entries[index].abandoned = true;
        }
    }
    pub fn observe_entry(
        &self,
        state: &mut super::ProducerState,
        target: &Function,
        binding: &ProducerTailBinding,
        available_events: usize,
        observed_event: usize,
    ) -> mlua::Result<()> {
        let (constructor, depth): (Table, usize) = self
            .inspect_entry
            .call((target.clone(), binding.config.constructor_slot))?;
        if depth == 0 || depth > state.active.len() || constructor.metatable().is_some() {
            return Err(error("producer tail entry constructor/activation mismatch"));
        }
        state.active.truncate(depth);
        let activation = *state
            .active
            .last()
            .ok_or_else(|| error("producer tail entry activation missing"))?;
        if let Some(old) = state
            .tail_entries
            .iter_mut()
            .find(|entry| entry.activation == activation && entry.constructor == constructor)
        {
            if old.consumed_by_tail.is_none() || old.abandoned {
                return Err(error(
                    "producer tail repeated or stale unconsumed line entry",
                ));
            }
            old.post_call_same_line_events = old
                .post_call_same_line_events
                .checked_add(1)
                .filter(|n| *n <= MAX_EVENTS)
                .ok_or_else(|| error("producer tail repeated line bound"))?;
            return Ok(());
        }
        if available_events == 0
            || state.tail_entries.len() + state.tails.len() >= binding.config.limits.max_events
        {
            return Err(error("producer tail combined event bound"));
        }
        if state.tail_pending.is_some() {
            return Err(error("producer tail overlapping pending entry"));
        }
        // The complete line/CFG proof identifies this first event as pre-GGET.
        // Read after the Lua inspection returns, and do no Lua calls afterward.
        let environment = Self::checked_environment(target, binding)?;
        let ordinal = state.tail_entries.len();
        state.tail_entries.push(ProducerTailLineEntry {
            ordinal,
            observed_event,
            activation,
            source_line: binding.config.source_line,
            caller_depth: depth,
            constructor_slot: binding.config.constructor_slot,
            constructor,
            environment,
            original_unpack: binding.original_unpack.clone(),
            consumed_by_tail: None,
            abandoned: false,
            post_call_same_line_events: 0,
        });
        state.tail_pending = Some(ordinal);
        Ok(())
    }
    pub fn observe(
        &self,
        state: &mut super::ProducerState,
        target: &Function,
        binding: &ProducerTailBinding,
        available_events: usize,
        observed_event: usize,
    ) -> mlua::Result<()> {
        let config = &binding.config;
        // Inspect only the Lua caller. mlua may temporarily shift the original
        // C frame when reserving failure storage before entering its hook.
        let row: Option<Table> = self.inspect.call((
            binding.original_unpack.clone(),
            target.clone(),
            config.source_line,
            config.argument_local.clone(),
            config.constructor_slot,
        ))?;
        let Some(row) = row else {
            return Ok(());
        };
        if available_events == 0
            || state.tail_entries.len() + state.tails.len() >= config.limits.max_events
        {
            return Err(error("producer tail combined event bound"));
        }
        let caller_depth: usize = row.raw_get("depth")?;
        if caller_depth == 0 || caller_depth > state.active.len() {
            return Err(error("producer tail missing caller activation"));
        }
        state.active.truncate(caller_depth);
        let activation = *state
            .active
            .last()
            .ok_or_else(|| error("producer tail missing activation"))?;
        let argument: Table = row.raw_get("argument")?;
        let constructor: Table = row.raw_get("constructor")?;
        if argument.metatable().is_some() || constructor.metatable().is_some() {
            return Err(error(
                "producer tail requires plain argument/constructor tables",
            ));
        }
        let line_entry_ordinal = state
            .tail_pending
            .ok_or_else(|| error("producer tail call has no fresh pre-GGET entry"))?;
        let entry = &state.tail_entries[line_entry_ordinal];
        if entry.activation != activation
            || entry.constructor != constructor
            || entry.abandoned
            || entry.consumed_by_tail.is_some()
            || entry.environment != Self::checked_environment(target, binding)?
        {
            return Err(error(
                "producer tail call does not consume its exact line entry",
            ));
        }
        let raw_length = argument.raw_len();
        let retained_values = state
            .tail_values
            .checked_add(raw_length)
            .filter(|n| *n <= config.limits.max_values)
            .ok_or_else(|| error("producer tail retained value bound"))?;
        let constructor_name: String = row.raw_get("constructorName")?;
        let mut retained_text = state
            .tail_text
            .checked_add(config.argument_local.len())
            .and_then(|n| n.checked_add(constructor_name.len()))
            .filter(|n| *n <= config.limits.max_text_bytes)
            .ok_or_else(|| error("producer tail retained text bound"))?;
        let mut values = Vec::with_capacity(raw_length);
        for index in 1..=raw_length {
            let value: Value = argument.raw_get(index)?;
            if let Value::String(value) = &value {
                retained_text = retained_text
                    .checked_add(value.as_bytes().len())
                    .filter(|n| *n <= config.limits.max_text_bytes)
                    .ok_or_else(|| error("producer tail retained text bound"))?;
            }
            values.push(value);
        }
        let event = ProducerTailCall {
            line_entry_ordinal,
            ordinal: state.tails.len(),
            observed_event,
            activation,
            caller: row.raw_get("caller")?,
            callee: binding.original_unpack.clone(),
            source_line: row.raw_get("sourceLine")?,
            caller_depth,
            argument_local: config.argument_local.clone(),
            argument_slot: row.raw_get("argumentSlot")?,
            argument,
            constructor_slot: config.constructor_slot,
            constructor_name,
            constructor,
            raw_length,
            values,
        };
        state.tail_entries[line_entry_ordinal].consumed_by_tail = Some(event.ordinal);
        state.tail_pending = None;
        state.tails.push(event);
        state.tail_values = retained_values;
        state.tail_text = retained_text;
        Ok(())
    }
}

#[cfg(test)]
#[path = "source_program_producer_tail_tests.rs"]
mod tests;
