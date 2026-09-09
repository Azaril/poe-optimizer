//! Independent source host stopped before any unique construction.
//! No production extractor or already-warmed Item oracle is used.
use mlua::{Function, HookTriggers, Lua, MultiValue, Value, VmState};
use poe_optimizer_pob::source;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
pub fn verified(path: &str) -> mlua::Result<String> {
    source::read_verified_text(&repository().join("vendor/path-of-building-poe2"), path)
        .map_err(mlua::Error::external)
}
fn load(
    lua: &Lua,
    path: &str,
    args: MultiValue,
    cache: &Arc<Mutex<BTreeMap<String, String>>>,
) -> mlua::Result<MultiValue> {
    let text = {
        let mut sources = cache.lock().unwrap();
        if !sources.contains_key(path) {
            sources.insert(path.into(), verified(path)?);
        }
        sources[path].clone()
    };
    lua.load(&text).set_name(format!("@{path}")).call(args)
}
fn section<'a>(source: &'a str, first: &str, next: &str) -> &'a str {
    assert_eq!(source.matches(first).count(), 1, "{first}");
    let start = source.find(first).unwrap();
    &source[start..start + source[start..].find(next).unwrap()]
}
pub struct UniqueRuntime {
    pub lua: Lua,
}
impl UniqueRuntime {
    pub fn new() -> Self {
        let lua = Lua::new();
        lua.set_memory_limit(1024 * 1024 * 1024).unwrap();
        let started = Instant::now();
        lua.set_hook(
            HookTriggers::new().every_nth_instruction(100_000),
            move |_, _| {
                if started.elapsed() > Duration::from_secs(120) {
                    Err(mlua::Error::RuntimeError(
                        "item loading oracle deadline".into(),
                    ))
                } else {
                    Ok(VmState::Continue)
                }
            },
        )
        .unwrap();
        poe_optimizer_lua_utf8::register(&lua).unwrap();
        lua.globals()
            .set(
                "GetTime",
                lua.create_function(move |_, ()| Ok(started.elapsed().as_secs_f64() * 1000.0))
                    .unwrap(),
            )
            .unwrap();
        let sources = Arc::new(Mutex::new(BTreeMap::new()));
        let modules = sources.clone();
        lua.globals()
            .set(
                "LoadModule",
                lua.create_function(move |lua, mut args: MultiValue| {
                    let name = match args.pop_front() {
                        Some(Value::String(s)) => s.to_str()?.to_owned(),
                        _ => return Err(mlua::Error::RuntimeError("missing module name".into())),
                    };
                    if name.contains("..") || name.starts_with('/') || name.contains('\\') {
                        return Err(mlua::Error::RuntimeError(
                            "invalid source module name".into(),
                        ));
                    }
                    let name = name.strip_suffix(".lua").unwrap_or(&name);
                    load(lua, &format!("src/{name}.lua"), args, &modules)
                })
                .unwrap(),
            )
            .unwrap();
        let base_require: Function = lua.globals().get("require").unwrap();
        let modules = sources.clone();
        let loaded = lua.create_table().unwrap();
        lua.globals()
            .set(
                "require",
                lua.create_function(move |lua, name: String| {
                    if matches!(name.as_str(), "bit" | "jit" | "jit.util" | "lua-utf8") {
                        return base_require.call::<Value>(name);
                    }
                    if name == "lcurl.safe" || name == "lua-profiler" {
                        return Ok(Value::Nil);
                    }
                    if name.contains("..") || name.contains('/') || name.contains('\\') {
                        return Err(mlua::Error::RuntimeError(
                            "invalid source library name".into(),
                        ));
                    }
                    if let value @ Value::Table(_) = loaded.get::<Value>(name.as_str())? {
                        return Ok(value);
                    }
                    let stem = name.replace('.', "/");
                    let root = repository().join("vendor/path-of-building-poe2");
                    let direct = format!("runtime/lua/{stem}.lua");
                    let path = if root.join(&direct).is_file() {
                        direct
                    } else {
                        format!("runtime/lua/{stem}/init.lua")
                    };
                    let result = load(lua, &path, MultiValue::new(), &modules)?
                        .pop_front()
                        .unwrap_or(Value::Nil);
                    loaded.set(name, result.clone())?;
                    Ok(result)
                })
                .unwrap(),
            )
            .unwrap();
        lua.load(
            "launch={devMode=false}; ConPrintf=function()end; main={}; jit.off(); jit.flush()",
        )
        .set_name("@item-loading-host-settings")
        .exec()
        .unwrap();
        for module in [
            "GameVersions",
            "Modules/Common",
            "Modules/CalcFormat",
            "Modules/Data",
            "Modules/ModTools",
            "Modules/ItemTools",
            "Modules/CalcTools",
            "Classes/Item",
        ] {
            lua.globals()
                .get::<Function>("LoadModule")
                .unwrap()
                .call::<MultiValue>(module)
                .unwrap();
        }
        // Original Main defaults and unique DB construction. The tree context is an
        // explicit latest-source host choice, not inferred caller allocation state.
        let main_source = verified("src/Modules/Main.lua").unwrap();
        lua.load(format!(
            "local self=main\n{}",
            section(
                &main_source,
                "\n\tself.defaultItemAffixQuality = 0.5\n",
                "\tself.showTitlebarName"
            )
        ))
        .exec()
        .unwrap();
        lua.load("data.setJewelRadiiGlobally(latestTreeVersion)")
            .exec()
            .unwrap();
        let cache_loop = section(
            &main_source,
            "\t\tfor k, v in pairs(LoadModule(\"Data/ModCache\")) do",
            "\n\tend",
        );
        lua.globals()
            .set(
                "loadOriginalStoredCache",
                lua.load(format!("return function()\n{cache_loop}\nend"))
                    .set_name("@test-original-main-cache-load")
                    .eval::<Function>()
                    .unwrap(),
            )
            .unwrap();
        let item_source = verified("src/Classes/Item.lua").unwrap();
        let requirements = section(
            &item_source,
            "\tif self.base then\n\t\tlocal dbItem = self:GetUniqueDBItem()",
            "\n\tself.affixLimit = 0",
        );
        lua.globals().set("originalRequirementFinalize", lua.load(format!(
            "local m_max=math.max;return function(self,importedLevelReq)\n{requirements}\nend"
        )).set_name("@test-original-requirement-finalization").eval::<Function>().unwrap()).unwrap();
        let unique_loop = section(
            &main_source,
            "\t\tfor type, typeList in pairsYield(data.uniques) do",
            "\t\tfor _, raw in pairsYield(data.rares) do",
        );
        lua.globals()
            .set(
                "originalUniqueLoad",
                lua.load(format!(
                    "return function() local self=main\n{unique_loop}\nend"
                ))
                .set_name("@test-original-main-unique-loop")
                .eval::<Function>()
                .unwrap(),
            )
            .unwrap();
        lua.load(include_str!("unique_requirement_observer.lua"))
            .set_name("@test-unique-requirement-observer")
            .exec()
            .unwrap();
        Self { lua }
    }
}
