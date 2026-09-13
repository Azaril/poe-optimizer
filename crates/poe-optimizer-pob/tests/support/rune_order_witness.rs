//! Bounded interpreted-only observation of the retained original ParseRaw.
//! The source methods and candidate tables are never replaced or mutated.
use mlua::{
    Function, HookTriggers, Lua, LuaString, MultiValue, Table, Thread, Value, VmState,
    debug::DebugEvent, ffi,
};
use serde::Serialize;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    ffi::CStr,
    rc::Rc,
    time::{Duration, Instant},
};

fn error(message: impl Into<String>) -> mlua::Error {
    mlua::Error::RuntimeError(format!("rune order observer: {}", message.into()))
}
pub fn original_parse(lua: &Lua, entry: &Function) -> Function {
    let mut current = entry.clone();
    for _ in 0..8 {
        let info = current.info();
        if info.source.as_deref() == Some("@src/Classes/Item.lua") {
            assert_eq!(info.line_defined, Some(468));
            assert_eq!(info.last_line_defined, Some(1803));
            return current;
        }
        let mut found = None;
        for slot in 1..=i32::from(current.info().num_upvalues) {
            // SAFETY: read existing captures only, retaining the actual function.
            let (name, value): (String, Value) = unsafe {
                lua.exec_raw(current.clone(), |state| {
                    let name = ffi::lua_getupvalue(state, 1, slot);
                    ffi::lua_pushstring(state, name);
                    ffi::lua_insert(state, -2);
                    ffi::lua_remove(state, 1);
                })
            }
            .unwrap();
            if matches!(name.as_str(), "parseRaw" | "originalParse") {
                assert!(found.is_none(), "ambiguous delegating ParseRaw capture");
                found = Some(
                    value
                        .as_function()
                        .expect("original ParseRaw capture")
                        .clone(),
                );
            }
        }
        current = found.expect("retained delegating ParseRaw capture");
    }
    panic!("ParseRaw capture depth bound");
}

#[derive(Debug, Serialize)]
pub struct Candidate {
    pub name: String,
    pub kind: String,
    pub values: Vec<f64>,
    pub effect_applied: Option<bool>,
}
#[derive(Debug, Serialize)]
pub struct Search {
    pub group_available: bool,
    pub group_name_aliases_verified: bool,
    pub source_line: usize,
    pub local_slots: BTreeMap<String, i32>,
    pub line: String,
    pub candidates: Vec<Candidate>,
    pub target: Vec<f64>,
    pub counts: Option<BTreeMap<usize, usize>>,
    pub count: Option<usize>,
    pub should_fix: bool,
    pub saved_runes_at_search: Vec<String>,
}
#[derive(Default, Debug, Serialize)]
pub struct Witness {
    pub parse_calls: usize,
    pub parse_returns: usize,
    pub update_calls: usize,
    pub update_returns: usize,
    pub searches: Vec<Search>,
    pub calls_complete: bool,
    pub exact_function_rechecked: bool,
    pub prior_hook_restored: bool,
    #[serde(skip)]
    active: usize,
    #[serde(skip)]
    retained_bytes: usize,
    #[serde(skip)]
    failure: Option<String>,
}
fn text(value: LuaString, budget: &mut usize) -> mlua::Result<String> {
    let bytes = value.as_bytes();
    if bytes.len() > 4096 || *budget > 1024 * 1024 - bytes.len() {
        return Err(error("text bound"));
    }
    *budget += bytes.len();
    Ok(value.to_str()?.to_owned())
}
fn dense(table: &Table, maximum: usize) -> mlua::Result<usize> {
    if table.metatable().is_some() {
        return Err(error("nonplain observed table"));
    }
    let n = table.raw_len();
    if n > maximum {
        return Err(error("array bound"));
    }
    let mut count = 0;
    for row in table.pairs::<Value, Value>() {
        let (key, _) = row?;
        count += 1;
        let index = match key {
            Value::Integer(i) if i > 0 && i as usize <= n => Some(i as usize),
            Value::Number(v) if v.is_finite() && v.fract() == 0.0 && v >= 1.0 && v <= n as f64 => {
                Some(v as usize)
            }
            _ => None,
        };
        if count > maximum || index.is_none() {
            return Err(error("nondense observed array"));
        }
    }
    if count != n {
        return Err(error("array hole"));
    }
    Ok(n)
}
// Source groupedRunes contains both the ordered numeric sequence and name
// aliases to those exact row tables. Validate both surfaces; only the sequence
// is the DFS order. Alias keys are not silently ignored.
fn group_rows(group: &Table) -> mlua::Result<Vec<Table>> {
    if group.metatable().is_some() || group.raw_len() > 256 {
        return Err(error("group bound/metatable"));
    }
    let n = group.raw_len();
    let rows = (1..=n)
        .map(|i| group.raw_get::<Table>(i))
        .collect::<mlua::Result<Vec<_>>>()?;
    let mut numeric = 0;
    let mut aliases = std::collections::BTreeSet::new();
    let mut visited = 0;
    for entry in group.pairs::<Value, Value>() {
        let (key, value) = entry?;
        visited += 1;
        if visited > 512 {
            return Err(error("group row bound"));
        }
        let value = value
            .as_table()
            .ok_or_else(|| error("group row is not a table"))?;
        if value.metatable().is_some() {
            return Err(error("candidate metatable"));
        }
        let index = match key {
            Value::Integer(i) if i > 0 && i as usize <= n => Some(i as usize),
            Value::Number(v) if v.is_finite() && v.fract() == 0.0 && v >= 1.0 && v <= n as f64 => {
                Some(v as usize)
            }
            Value::String(key) => {
                let name: LuaString = value.raw_get("name")?;
                if key.as_bytes().len() > 4096 || key.as_bytes() != name.as_bytes() {
                    return Err(error("invalid group name alias"));
                }
                let index = rows
                    .iter()
                    .position(|row| row.to_pointer() == value.to_pointer())
                    .ok_or_else(|| error("foreign group name alias"))?;
                if !aliases.insert(index) {
                    return Err(error("duplicate group name alias"));
                }
                None
            }
            _ => return Err(error("invalid group key")),
        };
        if let Some(i) = index {
            numeric += 1;
            if rows[i - 1].to_pointer() != value.to_pointer() {
                return Err(error("group sequence identity"));
            }
        }
    }
    if numeric != n || aliases.len() != n {
        return Err(error("group holes/missing name aliases"));
    }
    Ok(rows)
}
fn vector(table: Table) -> mlua::Result<Vec<f64>> {
    let n = dense(&table, 64)?;
    (1..=n)
        .map(|i| {
            let value = match table.raw_get::<Value>(i)? {
                Value::Integer(v) => v as f64,
                Value::Number(v) => v,
                _ => return Err(error("nonnumeric vector")),
            };
            if !value.is_finite() {
                return Err(error("nonfinite vector"));
            }
            Ok(value)
        })
        .collect()
}

/// Only the exact target's named live locals are retained, with their real slots.
/// The scan is bounded and ambiguous recursive activations are rejected.
fn locals(lua: &Lua, target: &Function) -> mlua::Result<Table> {
    let pointer = target.to_pointer();
    // SAFETY: protected read-only Lua debug API access; at most 32 frames and
    // 250 locals. Lua owns all pushed values; no source stack slot is written.
    unsafe {
        lua.exec_raw((), |state| {
            if ffi::lua_checkstack(state, 6) == 0 {
                ffi::lua_pushnil(state);
                return;
            }
            ffi::lua_newtable(state);
            let out = ffi::lua_gettop(state);
            let mut matches = 0;
            for level in 0..32 {
                let mut ar = std::mem::zeroed();
                if ffi::lua_getstack(state, level, &mut ar) == 0 {
                    break;
                }
                ffi::lua_getinfo(state, c"f".as_ptr(), &mut ar);
                let matched = ffi::lua_topointer(state, -1) == pointer;
                ffi::lua_pop(state, 1);
                if !matched {
                    continue;
                }
                matches += 1;
                for slot in 1..=251 {
                    let name = ffi::lua_getlocal(state, &ar, slot);
                    if name.is_null() {
                        break;
                    }
                    if slot == 251 {
                        ffi::lua_pop(state, 1);
                        matches = -1;
                        break;
                    }
                    if matches!(
                        CStr::from_ptr(name).to_bytes(),
                        b"self"
                            | b"groupedRunes"
                            | b"targetValues"
                            | b"result"
                            | b"numRunes"
                            | b"modLine"
                            | b"shouldFixRunesOnItem"
                    ) {
                        ffi::lua_newtable(state);
                        ffi::lua_pushvalue(state, -2);
                        ffi::lua_setfield(state, -2, c"value".as_ptr());
                        ffi::lua_pushinteger(state, slot as ffi::lua_Integer);
                        ffi::lua_setfield(state, -2, c"slot".as_ptr());
                        ffi::lua_setfield(state, out, name);
                    }
                    ffi::lua_pop(state, 1);
                }
            }
            let mut beyond = std::mem::zeroed();
            if ffi::lua_getstack(state, 32, &mut beyond) != 0 {
                matches = -1;
            }
            ffi::lua_pushinteger(state, matches);
            ffi::lua_setfield(state, out, c"matched_frames".as_ptr());
        })
    }
}
fn search(
    lua: &Lua,
    target: &Function,
    item: &Table,
    line: usize,
    budget: &mut usize,
) -> mlua::Result<Search> {
    let locals = locals(lua, target)?;
    if locals.raw_get::<i64>("matched_frames")? != 1 {
        return Err(error("target frame absent/ambiguous/bounded"));
    }
    let mut slots = BTreeMap::new();
    let mut read = |name: &str| -> mlua::Result<Value> {
        let row: Table = locals.raw_get(name)?;
        slots.insert(name.to_owned(), row.raw_get::<i32>("slot")?);
        row.raw_get("value")
    };
    if read("self")?.as_table().map(Table::to_pointer) != Some(item.to_pointer()) {
        return Err(error("foreign ParseRaw receiver"));
    }
    let group = match read("groupedRunes")? {
        Value::Table(group) => Some(group),
        Value::Nil if line == 1584 => None,
        _ => return Err(error("missing/invalid regular group")),
    };
    let group_available = group.is_some();
    let mut candidates = Vec::new();
    if let Some(group) = group {
        for row in group_rows(&group)? {
            if row.metatable().is_some() {
                return Err(error("nonplain candidate"));
            }
            candidates.push(Candidate {
                name: text(row.raw_get("name")?, budget)?,
                kind: text(row.raw_get("type")?, budget)?,
                values: vector(row.raw_get("values")?)?,
                effect_applied: row.raw_get("effectApplied")?,
            });
        }
    }
    let target = vector(
        read("targetValues")?
            .as_table()
            .ok_or_else(|| error("missing target"))?
            .clone(),
    )?;
    let counts = match read("result")? {
        Value::Nil => None,
        Value::Table(counts) => {
            if counts.metatable().is_some() {
                return Err(error("nonplain counts"));
            }
            let mut out = BTreeMap::new();
            for row in counts.pairs::<usize, usize>() {
                let (index, value) = row?;
                if out.len() >= 256 || index == 0 || index > candidates.len() || value > 128 {
                    return Err(error("count bound"));
                }
                out.insert(index, value);
            }
            Some(out)
        }
        _ => return Err(error("invalid result")),
    };
    let count = if line == 1551 {
        match read("numRunes")? {
            Value::Nil => None,
            Value::Integer(v) if (0..=128).contains(&v) => Some(v as usize),
            Value::Number(v) if v.is_finite() && v.fract() == 0.0 && (0.0..=128.0).contains(&v) => {
                Some(v as usize)
            }
            _ => return Err(error("nonnumeric/bounded count")),
        }
    } else {
        None
    };
    if !group_available && (counts.is_some() || count.is_some()) {
        return Err(error("absent bonded group unexpectedly returned counts"));
    }
    let mod_line = read("modLine")?
        .as_table()
        .ok_or_else(|| error("missing row"))?
        .clone();
    let line_text = text(mod_line.raw_get("line")?, budget)?;
    let should_fix = read("shouldFixRunesOnItem")?
        .as_boolean()
        .ok_or_else(|| error("missing fix flag"))?;
    let runes: Table = item.raw_get("runes")?;
    let mut saved = Vec::new();
    for i in 1..=dense(&runes, 128)? {
        saved.push(text(runes.raw_get(i)?, budget)?);
    }
    Ok(Search {
        group_available,
        group_name_aliases_verified: group_available,
        source_line: line,
        local_slots: slots,
        line: line_text,
        candidates,
        target,
        counts,
        count,
        should_fix,
        saved_runes_at_search: saved,
    })
}

struct HookGuard {
    lua: Lua,
    thread: Thread,
    prior: Option<ffi::lua_Hook>,
    mask: i32,
    count: i32,
}
impl Drop for HookGuard {
    fn drop(&mut self) {
        self.thread.remove_hook();
        // SAFETY: restore the exact saved hook pointer/configuration. The main
        // thread's mlua registry callback was never replaced. LuaJIT shares the
        // low-level hook configuration, so restore it even for a private thread.
        unsafe {
            self.lua
                .exec_raw::<()>((), |state| {
                    ffi::lua_sethook(state, self.prior, self.mask, self.count);
                })
                .expect("restore prior deadline hook");
        }
    }
}
pub fn capture(
    lua: &Lua,
    entry: &Function,
    original: &Function,
    update: &Function,
    item: &Table,
    raw: &str,
) -> mlua::Result<(Witness, Option<mlua::Error>)> {
    if original_parse(lua, entry) != *original {
        return Err(error("changed original ParseRaw"));
    }
    let mut prior = None;
    // SAFETY: reading the hook configuration does not change its callback.
    unsafe {
        lua.exec_raw::<()>((), |state| {
            prior = Some((
                ffi::lua_gethook(state),
                ffi::lua_gethookmask(state),
                ffi::lua_gethookcount(state),
            ));
        })?;
    }
    let (hook, mask, count) = prior.expect("hook configuration");
    let thread = lua.create_thread(entry.clone())?;
    let guard = HookGuard {
        lua: lua.clone(),
        thread: thread.clone(),
        prior: hook,
        mask,
        count,
    };
    let shared = Rc::new(RefCell::new(Witness::default()));
    let output = shared.clone();
    let target = original.clone();
    let updating = update.clone();
    let receiver = item.clone();
    let started = Instant::now();
    thread.set_hook(
        HookTriggers::new()
            .on_calls()
            .on_returns()
            .every_line()
            .every_nth_instruction(100_000),
        move |lua, debug| {
            if started.elapsed() > Duration::from_secs(120) {
                return Err(error("deadline"));
            }
            let is_parse = debug.function() == target;
            let is_update = debug.function() == updating;
            if !is_parse && !is_update {
                return Ok(VmState::Continue);
            }
            let mut state = output.borrow_mut();
            if let Some(failure) = &state.failure {
                return Err(error(failure.clone()));
            }
            let result = (|| {
                if state.parse_calls + state.update_calls + state.searches.len() >= 128 {
                    return Err(error("event bound"));
                }
                match debug.event() {
                    DebugEvent::Call if is_parse => {
                        state.parse_calls += 1;
                        state.active += 1;
                        if state.active != 1 {
                            return Err(error("recursive target"));
                        }
                    }
                    DebugEvent::Ret if is_parse => {
                        state.parse_returns += 1;
                        state.active = state
                            .active
                            .checked_sub(1)
                            .ok_or_else(|| error("unpaired return"))?;
                    }
                    DebugEvent::Call if is_update => {
                        state.update_calls += 1;
                    }
                    DebugEvent::Ret if is_update => {
                        state.update_returns += 1;
                    }
                    DebugEvent::Line
                        if is_parse && matches!(debug.current_line(), Some(1551 | 1584)) =>
                    {
                        let event = search(
                            lua,
                            &target,
                            &receiver,
                            debug.current_line().unwrap(),
                            &mut state.retained_bytes,
                        )?;
                        state.searches.push(event);
                    }
                    _ => {}
                }
                Ok(())
            })();
            if let Err(failure) = result {
                state.failure = Some(failure.to_string());
                return Err(failure);
            }
            Ok(VmState::Continue)
        },
    )?;
    let source_error = thread.resume::<MultiValue>((item.clone(), raw)).err();
    let finished = thread.is_finished();
    drop(guard);
    let mut observed = std::mem::take(&mut *shared.borrow_mut());
    if let Some(failure) = observed.failure {
        return Err(error(failure));
    }
    if source_error.is_none() && !finished {
        return Err(error("unexpected source yield"));
    }
    observed.calls_complete = observed.active == 0
        && observed.parse_calls == 1
        && observed.parse_returns == 1
        && observed.update_calls == observed.update_returns;
    observed.exact_function_rechecked = original_parse(lua, entry) == *original;
    // SAFETY: read back the restored hook configuration only.
    unsafe {
        lua.exec_raw::<()>((), |state| {
            observed.prior_hook_restored = ffi::lua_gethook(state).map(|f| f as usize)
                == hook.map(|f| f as usize)
                && ffi::lua_gethookmask(state) == mask
                && ffi::lua_gethookcount(state) == count;
        })?;
    }
    if !observed.prior_hook_restored {
        return Err(error("prior hook changed"));
    }
    Ok((observed, source_error))
}

#[test]
fn observed_arrays_accept_exact_numeric_keys_and_reject_holes_or_other_keys() {
    let lua = Lua::new();
    let table = lua.create_table().unwrap();
    table.raw_set(1.0, 20.0).unwrap();
    table.raw_set(2.0, 18.0).unwrap();
    assert_eq!(dense(&table, 2).unwrap(), 2);
    assert_eq!(vector(table.clone()).unwrap(), vec![20.0, 18.0]);
    assert!(dense(&table, 1).is_err());
    table.raw_set(1.5, 16.0).unwrap();
    assert!(dense(&table, 4).is_err());
    table.raw_set(1.5, Value::Nil).unwrap();
    table.raw_set(4.0, 14.0).unwrap();
    assert!(dense(&table, 4).is_err());
    table.raw_set(4.0, Value::Nil).unwrap();
    table.raw_set("1", 20.0).unwrap();
    assert!(dense(&table, 4).is_err());
}

#[test]
fn grouped_name_aliases_must_retain_the_exact_ordered_candidate() {
    let lua = Lua::new();
    let group = lua.create_table().unwrap();
    let row = lua.create_table().unwrap();
    row.raw_set("name", "Example").unwrap();
    group.raw_set(1.0, row.clone()).unwrap();
    group.raw_set("Example", row.clone()).unwrap();
    assert_eq!(
        group_rows(&group).unwrap()[0].to_pointer(),
        row.to_pointer()
    );
    let equal_distinct = lua.create_table().unwrap();
    equal_distinct.raw_set("name", "Example").unwrap();
    group.raw_set("Example", equal_distinct).unwrap();
    assert!(group_rows(&group).is_err());
    group.raw_set("Example", Value::Nil).unwrap();
    assert!(group_rows(&group).is_err());
    group.raw_set("Other", row).unwrap();
    assert!(group_rows(&group).is_err());
}
