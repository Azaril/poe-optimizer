//! Optional source-observed definition iteration and retained primitive identity.
use super::*;
use mlua::MultiValue;

pub(super) struct Primitives {
    pairs: Function,
    next: Function,
}
impl Primitives {
    pub(super) fn capture(
        lua: &Lua,
        globals: &Table,
        opaque: &[(String, Function)],
    ) -> Result<Self> {
        let pairs = opaque
            .iter()
            .find(|(name, _)| name == "pairs")
            .expect("original pairs")
            .1
            .clone();
        let next: Function = globals.raw_get("next")?;
        if next.info().what != "C" {
            return Err(error("source observer next is not an original C function"));
        }
        let result = Self { pairs, next };
        result.verify(lua)?;
        Ok(result)
    }
    pub(super) fn verify(&self, lua: &Lua) -> Result<()> {
        // LuaJIT lib_base's LJLIB_PUSH(lastcl) retains the original next as the
        // sole C upvalue of pairs. Never consult a later global next binding.
        if !matches!(builtin_upvalue(lua,&self.pairs,1)?,Some(Value::Function(ref next)) if next.to_pointer()==self.next.to_pointer())
            || builtin_upvalue(lua, &self.pairs, 2)?.is_some()
        {
            return Err(error("source observer pairs retained next capture changed"));
        }
        Ok(())
    }
}
fn builtin_upvalue(lua: &Lua, function: &Function, slot: i32) -> Result<Option<Value>> {
    if function.info().what != "C" || !(1..=2).contains(&slot) {
        return Err(error(
            "source iterator primitive capture request is invalid",
        ));
    }
    // SAFETY: only same-host primitives retained from this observer's original
    // globals reach this private helper. mlua roots the function and protects
    // stack restoration; public lua_getupvalue only reads the existing C slot.
    let (present, value): (bool, Value) = unsafe {
        lua.exec_raw(function.clone(), |state| {
            let name = mlua::ffi::lua_getupvalue(state, 1, slot);
            if name.is_null() {
                mlua::ffi::lua_settop(state, 0);
                mlua::ffi::lua_pushboolean(state, 0);
                mlua::ffi::lua_pushnil(state);
            } else {
                mlua::ffi::lua_remove(state, 1);
                mlua::ffi::lua_pushboolean(state, 1);
                mlua::ffi::lua_insert(state, 1);
            }
        })?
    };
    Ok(present.then_some(value))
}
impl SourceClosureObserver {
    /// Stream one original raw step without retaining omitted projection values.
    pub(super) fn raw_next(
        &self,
        table: &Table,
        previous: Value,
    ) -> Result<Option<(Value, Value)>> {
        let mut values: MultiValue = self
            .iterator_primitives
            .next
            .call((table.clone(), previous))?;
        if values.len() == 1 && matches!(values.front(), Some(Value::Nil)) {
            return Ok(None);
        }
        if values.len() != 2 || matches!(values.front(), Some(Value::Nil)) {
            return Err(error(
                "original next returned an invalid raw traversal result",
            ));
        }
        Ok(Some((
            values.pop_front().expect("next key"),
            values.pop_front().expect("next value"),
        )))
    }
    pub(super) fn verify_iteration(&self, enabled: bool) -> Result<()> {
        if enabled {
            self.iterator_primitives.verify(&self.lua)?;
        }
        Ok(())
    }
}
impl Graph<'_> {
    pub(super) fn iterator_intrinsic(&self, function: &Function) -> Option<SourceProgramIntrinsic> {
        self.context.iteration.as_ref()?;
        if function.to_pointer() == self.observer.iterator_primitives.pairs.to_pointer() {
            Some(SourceProgramIntrinsic::Pairs)
        } else if function.to_pointer() == self.observer.iterator_primitives.next.to_pointer() {
            Some(SourceProgramIntrinsic::Next)
        } else {
            None
        }
    }
    pub(super) fn capture_pairs_next(
        &mut self,
        callback: SourceCallbackId,
        depth: usize,
    ) -> Result<()> {
        if self.intrinsics.get(&callback) != Some(&SourceProgramIntrinsic::Pairs) {
            return Ok(());
        }
        let SourceValue::Callback(next) = self.value(
            Value::Function(self.observer.iterator_primitives.next.clone()),
            depth + 1,
        )?
        else {
            unreachable!()
        };
        self.context
            .iteration
            .as_mut()
            .expect("enabled iteration")
            .pairs_next
            .insert(callback, next);
        Ok(())
    }
    pub(super) fn raw_table_entries(&mut self, table: &Table) -> Result<Vec<(Value, Value)>> {
        let mut rows = Vec::new();
        if self.context.iteration.is_none() {
            for row in table.pairs::<Value, Value>() {
                self.projection_row()?;
                if rows.len() >= 50_000 {
                    return Err(error("source raw table row bound"));
                }
                rows.push(row?);
            }
            return Ok(rows);
        }
        let mut previous = Value::Nil;
        loop {
            self.projection_row()?;
            let mut values: MultiValue = self
                .observer
                .iterator_primitives
                .next
                .call((table.clone(), previous))?;
            if values.len() == 1 && matches!(values.front(), Some(Value::Nil)) {
                break;
            }
            if values.len() != 2 || matches!(values.front(), Some(Value::Nil)) {
                return Err(error(
                    "original next returned an invalid raw traversal result",
                ));
            }
            if rows.len() >= 50_000 {
                return Err(error("source raw table row bound"));
            }
            let key = values.pop_front().expect("next key");
            let value = values.pop_front().expect("next value");
            previous = key.clone();
            rows.push((key, value));
        }
        Ok(rows)
    }
    pub(super) fn store_iteration_order(
        &mut self,
        table: &Table,
        id: SourceTableId,
        rows: &[(Value, Value)],
        complete: bool,
    ) -> Result<()> {
        if self.context.iteration.is_none() || !complete || table.metatable().is_some() {
            return Ok(());
        }
        let keys = rows
            .iter()
            .map(|(key, _)| self.projection_key(key.clone()))
            .collect::<Result<Vec<_>>>()?;
        self.context
            .iteration
            .as_mut()
            .expect("enabled iteration")
            .table_order
            .insert(id, keys);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
