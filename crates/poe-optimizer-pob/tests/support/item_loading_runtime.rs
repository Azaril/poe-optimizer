//! Read-only source runtime for Item, with explicit host and observation boundaries.
use mlua::{Function, HookTriggers, Lua, MultiValue, Table, Value, VmState};
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
fn methods(lua: &Lua, path: &str, first: &str, next: &str) {
    let source = verified(path).unwrap();
    let start = source.find(first).unwrap();
    let padding = "\n".repeat(source[..start].bytes().filter(|b| *b == b'\n').count());
    lua.load(format!("{padding}{}", section(&source, first, next)))
        .set_name(format!("@{path}"))
        .exec()
        .unwrap();
}
pub struct Oracle {
    pub lua: Lua,
}
impl Oracle {
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
        let item_source = verified("src/Classes/Item.lua").unwrap();
        lua.globals()
            .set(
                "itemSyntax",
                lua.load(format!(
                    "{}\nreturn {{specToNumber=specToNumber,parseItemSpec=parseItemSpec}}",
                    section(
                        &item_source,
                        "local function specToNumber(s)",
                        "local function parseIdSpec("
                    )
                ))
                .set_name("@test-host-original-item-syntax")
                .eval::<Table>()
                .unwrap(),
            )
            .unwrap();
        lua.globals().set("itemPolicy",lua.load(format!("{}\n{}\nreturn {{catalysts=catalystList,descriptors=catalystDescriptorList,tags=catalystTags,scalar=getCatalystScalar,line_flags=lineFlags}}",
            section(&item_source,"local dmgTypeList =","local function normaliseModLine("),
            section(&item_source,"local lineFlags =","local function baseHasImplicitLine(")))
            .set_name("@test-host-original-item-policy").eval::<Table>().unwrap()).unwrap();
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
        let data_source = verified("src/Modules/Data.lua").unwrap();
        lua.globals()
            .set(
                "rawJewelRadii",
                lua.load(format!(
                    "local data={{}}\n{}\nreturn data.jewelRadii",
                    section(
                        &data_source,
                        "data.jewelRadii = {",
                        "data.jewelRadius = data.setJewelRadiiGlobally"
                    )
                ))
                .set_name("@test-host-original-raw-jewel-radii")
                .eval::<Table>()
                .unwrap(),
            )
            .unwrap();
        lua.load("data.setJewelRadiiGlobally(latestTreeVersion)")
            .exec()
            .unwrap();
        let unique_loop = section(
            &main_source,
            "\t\tfor type, typeList in pairsYield(data.uniques) do",
            "\t\tfor _, raw in pairsYield(data.rares) do",
        );
        lua.load(format!("main.uniqueDB={{list={{}},loading=true}}\nlocal self=main\nlocal function original_unique_load()\n{unique_loop}\nend\nlocal thread=coroutine.create(original_unique_load)\nrepeat local ok,err=coroutine.resume(thread);assert(ok,err) until coroutine.status(thread)=='dead'"))
            .set_name("@test-host-original-main-unique-construction").exec().unwrap();
        lua.globals()
            .set(
                "originalXml",
                lua.load(verified("runtime/lua/xml.lua").unwrap())
                    .set_name("@runtime/lua/xml.lua")
                    .eval::<Table>()
                    .unwrap(),
            )
            .unwrap();
        lua.load("ItemsTabClass={};t_insert=table.insert;t_remove=table.remove;s_format=string.format;m_max=math.max;m_min=math.min;m_floor=math.floor;m_ceil=math.ceil;m_modf=math.modf").exec().unwrap();
        for (first, next) in [
            (
                "function ItemsTabClass:Load(xml, dbFileName)",
                "function ItemsTabClass:Draw(",
            ),
            (
                "function ItemsTabClass:CreateItemSet(itemSetId, name)",
                "function ItemsTabClass:CopyItemSet(",
            ),
            (
                "function ItemsTabClass:SetActiveItemSet(itemSetId, deferSync)",
                "-- Equips the given item",
            ),
        ] {
            methods(&lua, "src/Classes/ItemsTab.lua", first, next);
        }
        let items = verified("src/Classes/ItemsTab.lua").unwrap();
        lua.load(format!("{}\nlocal runeModLines={{{{name='None'}}}}\nfunction original_slot_setup(self)\n{}\nend",section(&items,"local baseSlots =","local runeModLines"),section(&items,"\t-- Runes that fit Martial Artist","\t-- Passive tree dropdown controls")))
            .set_name("@test-host-original-slot-construction").exec().unwrap();
        lua.load(include_str!("item_loading_observer.lua"))
            .set_name("@item-loading-test-observer")
            .exec()
            .unwrap();
        Self { lua }
    }
    pub fn load(&self, xml: &str, instrument_ranges: bool) -> Table {
        self.lua
            .globals()
            .get::<Function>("item_loading_load")
            .unwrap()
            .call((xml, instrument_ranges))
            .unwrap()
    }
    pub fn parse(&self, raw: &str) -> Table {
        self.lua
            .globals()
            .get::<Function>("item_loading_parse")
            .unwrap()
            .call(raw)
            .unwrap()
    }
}
