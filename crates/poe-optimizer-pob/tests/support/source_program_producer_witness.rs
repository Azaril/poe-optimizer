//! Bounded, test-only observation of an exact original parser closure's row store.
//! Retained tables are live identities. No constructor-completion snapshot or layout.
#[path = "source_program_producer_tail.rs"]
pub mod tail;
use mlua::{Function, Lua, MultiValue, Table, Value, debug::DebugEvent};
use tail::ProducerTailProbe;
pub use tail::{ProducerTailBinding, ProducerTailCall, ProducerTailConfig, ProducerTailLineEntry};
const MAX_DEPTH: usize = 32;
const MAX_TEXT: usize = 65_536;
const INSPECT: &str = r#"local getinfo,getlocal = ...
return function(target, atStore)
    local frames = {}
    for level=1,256 do
        local info=getinfo(level,'f')
        if not info then return frames end
        if info.func == target then
            if #frames == 32 then error('producer witness depth bound') end
            local row={}
            if atStore and #frames == 0 then
                for slot=1,256 do
                    local key,value=getlocal(level,slot)
                    if not key then break end
                    if slot == 256 then error('producer witness local bound') end
                    if key == 'modList' or key == 'i' or key == 'name' then
                        if row[key .. 'Slot'] then error('producer witness ambiguous local') end
                        row[key]=value; row[key .. 'Slot']=slot
                    end
                end
            end
            frames[#frames+1]=row
        end
    end
    error('producer witness stack scan bound')
end"#;
fn error(message: &str) -> mlua::Error {
    mlua::Error::RuntimeError(message.into())
}
#[derive(Clone)]
pub struct ProducerProbe {
    getupvalue: Function,
    inspect: Function,
    tail: ProducerTailProbe,
}
#[derive(Debug, Clone)]
pub struct ProducerBinding {
    parser: Function,
    pub target: Function,
    pub capture_slot: usize,
    pub source_line: usize,
    pub tail: Option<ProducerTailBinding>,
}
#[derive(Debug)]
pub struct ProducerActivation {
    pub ordinal: usize,
    pub observed_event: usize,
    pub parent: Option<usize>,
    pub depth: usize,
}
#[derive(Debug)]
pub struct ProducerStore {
    pub ordinal: usize,
    pub observed_event: usize,
    pub activation: usize,
    pub source_line: usize,
    pub list: Table,
    pub index: Value,
    pub name: Value,
    pub table: Table,
    pub list_slot: usize,
    pub index_slot: usize,
    pub name_slot: usize,
}
#[derive(Debug, Default)]
pub struct ProducerState {
    pub activations: Vec<ProducerActivation>,
    pub stores: Vec<ProducerStore>,
    pub tails: Vec<ProducerTailCall>,
    pub tail_entries: Vec<ProducerTailLineEntry>,
    tail_pending: Option<usize>,
    tail_values: usize,
    tail_text: usize,
    active: Vec<usize>,
    retained_text: usize,
}
impl ProducerProbe {
    pub fn before_source(lua: &Lua) -> mlua::Result<Self> {
        let debug: Table = lua.globals().raw_get("debug")?;
        let getupvalue: Function = debug.raw_get("getupvalue")?;
        let getinfo: Function = debug.raw_get("getinfo")?;
        let getlocal: Function = debug.raw_get("getlocal")?;
        for function in [&getupvalue, &getinfo, &getlocal] {
            if function.info().what != "C" {
                return Err(error("producer witness requires original C inspection"));
            }
        }
        let tail = ProducerTailProbe::before_source(lua, &getinfo, &getlocal)?;
        Ok(Self {
            getupvalue,
            tail,
            inspect: lua
                .load(INSPECT)
                .set_name("@tests/support/source_program_producer_witness.rs#inspection")
                .call((getinfo, getlocal))?,
        })
    }
    pub fn bind(
        &self,
        parser: &Function,
        target: &Function,
        source_line: usize,
    ) -> mlua::Result<ProducerBinding> {
        if target.info().what != "Lua" || parser.info().what != "Lua" {
            return Err(error("producer witness requires actual Lua functions"));
        }
        if !matches!((target.info().line_defined,target.info().last_line_defined),
            (Some(first),Some(last)) if first < source_line && source_line <= last)
        {
            return Err(error(
                "producer witness continuation outside actual function",
            ));
        }
        let mut capture_slot = None;
        for slot in 1..=256 {
            let values: MultiValue = self.getupvalue.call((parser.clone(), slot))?;
            if values.is_empty() {
                break;
            }
            if slot == 256 {
                return Err(error("producer witness capture bound"));
            }
            if values
                .front()
                .and_then(Value::as_string)
                .is_some_and(|s| s.as_bytes().as_ref() == b"parseMod")
            {
                if capture_slot.is_some() || values.get(1) != Some(&Value::Function(target.clone()))
                {
                    return Err(error("producer witness ambiguous/rebound parseMod capture"));
                }
                capture_slot = Some(slot);
            }
        }
        Ok(ProducerBinding {
            parser: parser.clone(),
            target: target.clone(),
            capture_slot: capture_slot
                .ok_or_else(|| error("producer witness missing parseMod capture"))?,
            source_line,
            tail: None,
        })
    }
    pub fn verify(&self, binding: &ProducerBinding) -> mlua::Result<()> {
        let current = self.bind(&binding.parser, &binding.target, binding.source_line)?;
        if current.capture_slot != binding.capture_slot {
            return Err(error("producer witness capture slot changed"));
        }
        if let Some(tail) = &binding.tail {
            self.tail.verify(tail)?;
            if !matches!((binding.target.info().line_defined, binding.target.info().last_line_defined),
                (Some(first), Some(last)) if first < tail.config.source_line && tail.config.source_line <= last)
            {
                return Err(error("producer tail line outside actual function"));
            }
        }
        Ok(())
    }
    pub fn bind_tail(
        &self,
        binding: &ProducerBinding,
        config: ProducerTailConfig,
    ) -> mlua::Result<ProducerBinding> {
        self.verify(binding)?;
        if !matches!((binding.target.info().line_defined, binding.target.info().last_line_defined),
            (Some(first), Some(last)) if first < config.source_line && config.source_line <= last)
        {
            return Err(error("producer tail line outside actual function"));
        }
        let mut result = binding.clone();
        result.tail = Some(self.tail.bind(config)?);
        Ok(result)
    }
    pub fn observe_tail(
        &self,
        state: &mut ProducerState,
        binding: &ProducerBinding,
        available_events: usize,
        observed_event: usize,
    ) -> mlua::Result<()> {
        let tail = binding
            .tail
            .as_ref()
            .ok_or_else(|| error("producer tail is not enabled"))?;
        self.tail.observe(
            state,
            &binding.target,
            tail,
            available_events,
            observed_event,
        )
    }
    pub fn observe(
        &self,
        state: &mut ProducerState,
        binding: &ProducerBinding,
        event: DebugEvent,
        line: Option<usize>,
        available_events: usize,
        observed_event: usize,
    ) -> mlua::Result<()> {
        if let Some(tail) = &binding.tail {
            if event == DebugEvent::Line && line == Some(tail.config.source_line) {
                return self.tail.observe_entry(
                    state,
                    &binding.target,
                    tail,
                    available_events,
                    observed_event,
                );
            }
            if matches!(
                event,
                DebugEvent::Call | DebugEvent::Ret | DebugEvent::TailCall | DebugEvent::Line
            ) {
                ProducerTailProbe::abandon_pending(state);
            }
        }
        match event {
            DebugEvent::Call | DebugEvent::Line => {
                let at_store = event == DebugEvent::Line;
                if at_store && line != Some(binding.source_line) {
                    return Ok(());
                }
                if available_events == 0 {
                    return Err(error("combined copy/producer witness event bound"));
                }
                let frames: Table = self.inspect.call((binding.target.clone(), at_store))?;
                let depth = frames.raw_len();
                if depth == 0 || depth > MAX_DEPTH {
                    return Err(error("producer witness exact frame/depth bound"));
                }
                let ancestors = if at_store { depth } else { depth - 1 };
                if ancestors > state.active.len() {
                    return Err(error("producer witness missed activation"));
                }
                state.active.truncate(ancestors);
                if !at_store {
                    let ordinal = state.activations.len();
                    state.activations.push(ProducerActivation {
                        ordinal,
                        observed_event,
                        parent: state.active.last().copied(),
                        depth,
                    });
                    state.active.push(ordinal);
                    return Ok(());
                }
                let row: Table = frames.raw_get(1)?;
                let list: Table = row.raw_get("modList")?;
                let index: Value = row.raw_get("i")?;
                let name: Value = row.raw_get("name")?;
                let number = match index {
                    Value::Integer(v) => v as f64,
                    Value::Number(v) => v,
                    _ => return Err(error("producer witness index type")),
                };
                if !number.is_finite() || number < 1.0 || number.fract() != 0.0 {
                    return Err(error("producer witness index is not a positive integer"));
                }
                let name_bytes = name
                    .as_string()
                    .ok_or_else(|| error("producer witness name type"))?
                    .as_bytes()
                    .len();
                let retained_text = state
                    .retained_text
                    .checked_add(name_bytes)
                    .filter(|n| *n <= MAX_TEXT)
                    .ok_or_else(|| error("producer witness retained text bound"))?;
                let table: Table = list.raw_get(index.clone())?;
                if list.metatable().is_some() || table.metatable().is_some() {
                    return Err(error("producer witness expects plain original row/list"));
                }
                let activation = *state
                    .active
                    .last()
                    .ok_or_else(|| error("producer witness missing current activation"))?;
                let store = ProducerStore {
                    ordinal: state.stores.len(),
                    observed_event,
                    activation,
                    source_line: binding.source_line,
                    list,
                    index,
                    name,
                    table,
                    list_slot: row.raw_get("modListSlot")?,
                    index_slot: row.raw_get("iSlot")?,
                    name_slot: row.raw_get("nameSlot")?,
                };
                state.stores.push(store);
                state.retained_text = retained_text;
            }
            DebugEvent::Ret => {
                state
                    .active
                    .pop()
                    .ok_or_else(|| error("producer witness unmatched return"))?;
            }
            DebugEvent::TailCall => {
                return Err(error("producer witness unexpected tail-return event"));
            }
            _ => {}
        }
        Ok(())
    }
}
impl ProducerState {
    pub fn event_count(&self) -> usize {
        self.activations.len() + self.stores.len() + self.tails.len() + self.tail_entries.len()
    }
}

impl ProducerBinding {
    pub fn is_parser(&self, parser: &Function) -> bool {
        self.parser == *parser
    }
}
