//! Strong roots in one private Lua table, rather than one mlua auxiliary-stack
//! reference per observed value. IDs never leave this capture and every access
//! is raw; neither source tables nor their metatables are mutated or invoked.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct StoredValue(u32);

pub(super) struct ValueArena {
    table: Table,
    len: usize,
}
impl ValueArena {
    pub(super) fn new(lua: &Lua) -> Result<Self> {
        Ok(Self {
            table: lua.create_table()?,
            len: 0,
        })
    }
    pub(super) fn store(&mut self, value: Value) -> Result<StoredValue> {
        // Discovery already bounds source rows, tables and capture cells. This
        // independent ceiling also bounds private arena storage itself.
        if self.len >= MAX_VALUES {
            return Err(error("session reference arena value bound"));
        }
        let id = StoredValue(self.len as u32 + 1);
        self.table.raw_set(id.0, value)?;
        self.len += 1;
        Ok(id)
    }
    pub(super) fn value(&self, id: StoredValue) -> Result<Value> {
        Ok(self.table.raw_get(id.0)?)
    }
    pub(super) fn function(&self, id: StoredValue) -> Result<Function> {
        Ok(self.table.raw_get(id.0)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arena_is_a_strong_gc_root_only_until_capture_is_dropped() {
        let lua = Lua::new();
        let weak: Table = lua
            .load("return setmetatable({}, {__mode='v'})")
            .eval()
            .unwrap();
        let mut arena = ValueArena::new(&lua).unwrap();
        let original: Table = lua
            .load("local t={}; t.self=t; t.read=function() return t end; return t")
            .eval()
            .unwrap();
        let pointer = original.to_pointer();
        weak.raw_set(1, original.clone()).unwrap();
        let stored = arena.store(Value::Table(original)).unwrap();
        lua.gc_collect().unwrap();
        assert_eq!(weak.raw_get::<Table>(1).unwrap().to_pointer(), pointer);
        {
            let Value::Table(table) = arena.value(stored).unwrap() else {
                panic!("table")
            };
            let read: Function = table.raw_get("read").unwrap();
            assert_eq!(read.call::<Table>(()).unwrap().to_pointer(), pointer);
            assert_eq!(
                table.raw_get::<Table>("self").unwrap().to_pointer(),
                pointer
            );
        }
        drop(arena);
        lua.gc_collect().unwrap();
        assert!(matches!(weak.raw_get::<Value>(1).unwrap(), Value::Nil));
        let failure: Result<()> = (|| {
            let mut arena = ValueArena::new(&lua)?;
            let original = lua.create_table()?;
            weak.raw_set(1, original.clone())?;
            arena.store(Value::Table(original))?;
            Err(error("deliberate capture failure"))
        })();
        assert!(failure.is_err());
        lua.gc_collect().unwrap();
        assert!(matches!(weak.raw_get::<Value>(1).unwrap(), Value::Nil));
    }
}
