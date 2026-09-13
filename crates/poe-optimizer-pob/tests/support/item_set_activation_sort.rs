//! Opt-in startup observation; neither original sorter nor comparator is replaced.
use mlua::{Function, Lua, Result, Table};
use sha2::{Digest, Sha256};

const OBSERVER: &str = include_str!("item_set_activation_sort.lua");

pub struct StartupSort {
    capture: Table,
    finished: bool,
}
impl StartupSort {
    pub fn start(lua: &Lua, observed: bool) -> Result<Self> {
        let factory: Function = lua
            .load(OBSERVER)
            .set_name("@item_set_activation_sort.lua")
            .eval()?;
        Ok(Self {
            capture: factory.call(observed)?,
            finished: false,
        })
    }
    pub fn finish(&mut self) -> Result<Table> {
        let finish: Function = self.capture.raw_get("finish")?;
        // Lua finish removes this observer's hook before any later validation can
        // fail. If source initialization failed before before_build, Drop still
        // invokes abort through the retained private capture.
        let result: Result<Table> = finish.call(());
        self.finished = true;
        let report = result?;
        report.raw_set(
            "observer_sha256",
            format!("{:x}", Sha256::digest(OBSERVER.as_bytes())),
        )?;
        report.raw_set(
            "rust_adapter_sha256",
            format!(
                "{:x}",
                Sha256::digest(include_bytes!("item_set_activation_sort.rs"))
            ),
        )?;
        Ok(report)
    }
    pub fn after_import(&self) -> Result<Table> {
        self.capture
            .raw_get::<Function>("verify_after_import")?
            .call(())
    }
    pub fn permutations(&self) -> Result<Table> {
        self.capture.raw_get::<Function>("permutations")?.call(())
    }
}
impl Drop for StartupSort {
    fn drop(&mut self) {
        if !self.finished
            && let Ok(abort) = self.capture.raw_get::<Function>("abort")
        {
            let _ = abort.call::<()>(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Value;

    fn lua() -> Lua {
        // Isolated mechanics only: the fixture needs the original debug/JIT
        // libraries, and does not load an external chunk or shared source host.
        unsafe { Lua::unsafe_new() }
    }

    #[test]
    fn control_retains_intrinsic_without_installing_a_startup_hook() {
        let lua = lua();
        let before: bool = lua.load("return jit.status()").eval().unwrap();
        let mut witness = StartupSort::start(&lua, false).unwrap();
        let report = witness.finish().unwrap();
        witness.after_import().unwrap();
        assert!(!report.raw_get::<bool>("observed").unwrap());
        assert_eq!(report.raw_get::<u64>("hook_events").unwrap(), 0);
        assert!(matches!(
            lua.load("return debug.gethook()").eval::<Value>().unwrap(),
            Value::Nil
        ));
        assert_eq!(
            before,
            lua.load("return jit.status()").eval::<bool>().unwrap()
        );
    }

    #[test]
    fn startup_witness_rejects_sorter_rebinding() {
        let lua = lua();
        let mut witness = StartupSort::start(&lua, false).unwrap();
        // Explicitly hostile mechanics input; not an original-source wrapper.
        lua.load("table.sort = function() end").exec().unwrap();
        let error = witness.finish().unwrap_err();
        assert!(
            error
                .to_string()
                .contains("retained original sorter changed")
        );
        assert!(
            witness
                .finish()
                .unwrap_err()
                .to_string()
                .contains("retained original sorter changed")
        );
    }

    #[test]
    fn source_initialization_error_removes_only_the_owned_hook() {
        let lua = lua();
        let before: bool = lua.load("return jit.status()").eval().unwrap();
        let witness = StartupSort::start(&lua, true).unwrap();
        let error = lua
            .load("error('derived source initialization failure')")
            .exec()
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("derived source initialization failure")
        );
        drop(witness);
        assert!(matches!(
            lua.load("return debug.gethook()").eval::<Value>().unwrap(),
            Value::Nil
        ));
        assert_eq!(
            before,
            lua.load("return jit.status()").eval::<bool>().unwrap()
        );
    }

    #[test]
    fn aborted_startup_preserves_a_foreign_hook() {
        let lua = lua();
        let witness = StartupSort::start(&lua, true).unwrap();
        lua.load("foreign = function() end; debug.sethook(foreign, 'c')")
            .exec()
            .unwrap();
        drop(witness);
        let (hook, foreign): (Function, Function) =
            lua.load("return debug.gethook(), foreign").eval().unwrap();
        assert_eq!(hook, foreign);
        lua.load("debug.sethook()").exec().unwrap();
    }
    #[test]
    fn module_continuation_closes_witness_and_excludes_nested_and_later_sorters() {
        let lua = lua();
        // This is a labelled observer-mechanics chunk, not a replacement of a
        // production source function or a full-source parity case. Fixed source
        // coordinates let the actual LuaJIT hook exercise receiver/frame joins.
        let mut lines = vec![""; 2288];
        lines[0] = "local runeModLines = {{name='None',slot='None',order=-1,req=1,group=-1},{name='Second',slot='caller',order=2,req=1,group=1}}";
        lines[1] = "local ItemsTabClass = {}; common = {classes={ItemsTab=ItemsTabClass}}";
        lines[337] = "local function later_sort()";
        lines[338] = "sort_mechanics_calls=(sort_mechanics_calls or 0)+1; table.sort({2,1}, function(a,b) return a<b end)";
        lines[339] = "end";
        lines[2239] = "table.sort(runeModLines, function(a, b)";
        lines[2240] = "later_sort(); if a.order == b.order then";
        lines[2241] = "return a.req < b.req";
        lines[2242] = "elseif a.group == b.group then";
        lines[2243] = "return a.order < b.order";
        lines[2244] = "else";
        lines[2245] = "return a.group < b.group";
        lines[2246] = "end";
        lines[2247] = "end)";
        lines[2249] = "function ItemsTabClass:GetValidRunesForItem(item)";
        lines[2250] = "return runeModLines";
        lines[2285] = "end";
        lines[2287] = "later_sort()";
        let mut witness = StartupSort::start(&lua, true).unwrap();
        lua.load(lines.join("\n"))
            .set_name("@Classes/ItemsTab.lua")
            .exec()
            .unwrap();
        let report = witness.finish().unwrap();
        witness.after_import().unwrap();
        assert!(
            report
                .raw_get::<bool>("actual_sort_completion_observed")
                .unwrap()
        );
        let boundary: Table = report.raw_get("sort_completion_boundary").unwrap();
        assert_eq!(
            boundary.raw_get::<String>("kind").unwrap(),
            "next_original_module_line"
        );
        assert!([2250, 2286].contains(&boundary.raw_get::<usize>("line").unwrap()));
        assert!(boundary.raw_get::<bool>("same_original_chunk").unwrap());
        assert!(
            boundary
                .raw_get::<bool>("retained_module_local_identity")
                .unwrap()
        );
        assert!(report.raw_get::<usize>("active_line_events").unwrap() > 0);
        assert!(
            report
                .raw_get::<usize>("other_sort_calls_during_witness")
                .unwrap()
                > 0
        );
        assert!(
            lua.globals()
                .raw_get::<usize>("sort_mechanics_calls")
                .unwrap()
                >= 2
        );
        assert_eq!(report.raw_get::<usize>("rows").unwrap(), 2);
        let comparator: Table = report.raw_get("comparator").unwrap();
        assert_eq!(comparator.raw_get::<usize>("first_line").unwrap(), 2240);
        assert_eq!(comparator.raw_get::<usize>("last_line").unwrap(), 2248);
        let scope: Table = report.raw_get("scope").unwrap();
        assert!(
            !scope
                .raw_get::<bool>("c_sort_return_hook_required")
                .unwrap()
        );
        assert!(
            scope
                .raw_get::<bool>("completion_requires_original_module_continuation")
                .unwrap()
        );
        assert!(matches!(
            lua.load("return debug.gethook()").eval::<Value>().unwrap(),
            Value::Nil
        ));
    }
}
