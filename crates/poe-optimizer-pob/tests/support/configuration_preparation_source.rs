//! Fresh complete source runtime with observation wrappers, in a dedicated test process.
use mlua::{Function, Lua, LuaSerdeExt, MultiValue, Table, Value};
use poe_optimizer_core::options::EvaluationOptions;
use poe_optimizer_pob::runtime::RuntimeError;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
const HOST: &str = include_str!("../../src/host.lua");
const INITIALIZATION: &str = include_str!("../../src/initialization.lua");
const MAX_ITEM_DATABASE_RESUMES: usize = 16_384;
fn lua_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        format!("//{}", unc.replace(char::from(92), "/"))
    } else {
        text.strip_prefix(r"\\?\")
            .unwrap_or(&text)
            .replace(char::from(92), "/")
    }
}
pub fn observe(
    pob_root: &Path,
    scratch: &Path,
    xml: &str,
    warm_xml: Option<&str>,
    structural_case: bool,
) -> Result<serde_json::Value, RuntimeError> {
    observe_with_hook(pob_root, scratch, xml, warm_xml, structural_case, None)
}

type ObservationHook<'a> = dyn Fn(&Lua) -> Result<serde_json::Value, RuntimeError> + 'a;

/// Reuse the complete source bootstrap for additional independent parity consumers.
/// The hook runs after the original build finishes and cannot replace source methods.
pub fn observe_with_hook(
    pob_root: &Path,
    scratch: &Path,
    xml: &str,
    warm_xml: Option<&str>,
    structural_case: bool,
    hook: Option<&ObservationHook<'_>>,
) -> Result<serde_json::Value, RuntimeError> {
    observe_with_hooks(
        pob_root,
        scratch,
        xml,
        warm_xml,
        structural_case,
        None,
        hook,
    )
}

type BeforeSourceHook<'a> = dyn Fn(&Lua) -> Result<(), RuntimeError> + 'a;

/// Capture primitive identities before source initialization, then observe the
/// unchanged complete runtime. Neither hook replaces game-source functions.
#[allow(clippy::too_many_arguments)]
pub fn observe_with_hooks(
    pob_root: &Path,
    scratch: &Path,
    xml: &str,
    warm_xml: Option<&str>,
    structural_case: bool,
    before_source: Option<&BeforeSourceHook<'_>>,
    hook: Option<&ObservationHook<'_>>,
) -> Result<serde_json::Value, RuntimeError> {
    let start = Instant::now();
    poe_optimizer_pob::import::decode_build(xml.as_bytes())?;
    // Structural cases deliberately exercise original Lua coercion/diagnostics
    // that the stricter external reference adapter rejects before source loading.
    // XML remains decoded/bounded; no input text is evaluated as Lua code.
    if !structural_case {
        poe_optimizer_pob::preflight::validate(xml)?;
    }
    let root = pob_root.canonicalize()?;
    let source_hash = poe_optimizer_pob::source::verify(&root)
        .map_err(|error| RuntimeError::Setup(error.to_string()))?;
    let source = root.join("src").canonicalize()?;
    if std::env::current_dir()?.canonicalize()? != source {
        return Err(RuntimeError::Setup(
            "Run the evaluator in its dedicated process with PoB src as cwd".into(),
        ));
    }
    let user_path = scratch.canonicalize()?;

    // SAFETY: only the pinned, clean upstream code and our static native module
    // execute here. PoB requires debug facilities. Input XML is validated data,
    // never a Lua chunk. Dynamic native loading is disabled below.
    let lua = unsafe { Lua::unsafe_new() };
    if let Some(hook) = before_source {
        hook(&lua)?;
    }
    lua.load(include_str!("source_module_observation.lua"))
        .set_name("@source-module-observation.lua")
        .exec()?;
    poe_optimizer_lua_utf8::register(&lua)?;
    let globals = lua.globals();
    globals.set("arg", lua.create_table()?)?;
    globals.set(
        "_optimizer_options",
        lua.to_value_with(
            &EvaluationOptions::default(),
            mlua::serde::SerializeOptions::new().serialize_none_to_null(false),
        )?,
    )?;
    globals.set("_optimizer_source_path", lua_path(&source))?;
    globals.set("_optimizer_runtime_path", lua_path(&root.join("runtime")))?;
    globals.set("_optimizer_user_path", lua_path(&user_path))?;
    globals.set(
        "_optimizer_time",
        lua.create_function(move |_, ()| Ok(start.elapsed().as_millis() as u64))?,
    )?;
    globals.set(
        "_optimizer_log",
        lua.create_function(|lua, args: MultiValue| {
            let tostring: Function = lua.globals().get("tostring")?;
            let mut parts = Vec::new();
            for value in args {
                parts.push(tostring.call::<String>(value)?);
            }
            eprintln!("{}", parts.join("\t"));
            Ok(())
        })?,
    )?;
    let write_root = user_path.clone();
    globals.set(
        "_optimizer_check_write",
        lua.create_function(move |_, path: String| {
            checked_destination(Path::new(&path), &write_root).map_err(mlua::Error::external)?;
            Ok(())
        })?,
    )?;
    let directory_root = user_path.clone();
    globals.set(
        "_optimizer_make_dir",
        lua.create_function(move |_, path: String| {
            let path = PathBuf::from(path);
            if path.is_dir() {
                return Ok(true);
            }
            let destination =
                checked_destination(&path, &directory_root).map_err(mlua::Error::external)?;
            fs::create_dir_all(destination).map_err(mlua::Error::external)?;
            Ok(true)
        })?,
    )?;
    let package: Table = globals.get("package")?;
    package.set(
        "path",
        format!(
            "{0}/?.lua;{0}/?/init.lua;{1}/lua/?.lua;{1}/lua/?/init.lua",
            lua_path(&source),
            lua_path(&root.join("runtime"))
        ),
    )?;
    package.set("cpath", "")?;
    package.set(
        "loadlib",
        lua.create_function(|_, _: MultiValue| -> mlua::Result<()> {
            Err(mlua::Error::external("Dynamic native modules are disabled"))
        })?,
    )?;

    // Interpose dofile only to install host callbacks at the upstream stub boundary.
    // lua.load handles the entry points' leading hash lines without source edits.
    globals.set(
        "dofile",
        lua.create_function(|lua, filename: String| {
            let source = fs::read_to_string(&filename).map_err(mlua::Error::external)?;
            lua.globals()
                .get::<Function>("_configuration_source_module_enter")?
                .call::<()>(format!("@{filename}"))?;
            let values = lua
                .load(&source)
                .set_name(format!("@{filename}"))
                .call::<MultiValue>(())?;
            if filename == "_SimpleGraphic.def.lua" {
                lua.load(HOST).set_name("@optimizer-host.lua").exec()?;
            } else if filename == "Launch.lua" {
                // Online update checks are outside evaluator behavior.
                lua.load("launch.CheckForUpdate = function() end").exec()?;
            }
            Ok(values)
        })?,
    )?;
    lua.load(fs::read_to_string(source.join("HeadlessWrapper.lua"))?)
        .set_name("@HeadlessWrapper.lua")
        .exec()?;
    check_prompt(&lua)?;
    lua.load(INITIALIZATION)
        .set_name("@optimizer-initialization.lua")
        .call::<()>(MAX_ITEM_DATABASE_RESUMES)?;
    check_prompt(&lua)?;
    if let Some(warm_xml) = warm_xml {
        globals
            .get::<Function>("loadBuildFromXML")?
            .call::<()>((warm_xml, "configuration-source-warmup"))?;
        check_prompt(&lua)?;
    }
    lua.load(include_str!("configuration_preparation_source.lua"))
        .set_name("@configuration-source-observation.lua")
        .exec()?;
    let load: Function = globals.get("loadBuildFromXML")?;
    load.call::<()>((xml, "configuration-source-input"))?;
    if !structural_case {
        check_prompt(&lua)?;
    }
    lua.load(
        r#"
        assert(main.mode == "BUILD" and not main.newMode)
        assert(not build.abortSave and #main.popups == 0)
        assert(build.calcsTab.mainEnv and build.calcsTab.mainOutput)
        _configuration_source_trace.prompt = launch.promptMsg
        _configuration_source_trace.final = _configuration_source_state(build.configTab, build)
        _configuration_source_trace.selected = {
            skills=build.skillsTab.activeSkillSetId, items=build.itemsTab.activeItemSetId,
            config=build.configTab.activeConfigSetId, passives=build.treeTab.activeSpec}
    "#,
    )
    .exec()?;
    let value: Value = globals.get("_configuration_source_trace")?;
    let mut result: serde_json::Value = lua.from_value(value)?;
    result["source_hash"] = source_hash.into();
    if let Some(hook) = hook {
        result["additional_observation"] = hook(&lua)?;
    }
    result["elapsed_ms"] = (start.elapsed().as_secs_f64() * 1000.0).into();
    Ok(result)
}
fn checked_destination(path: &Path, root: &Path) -> Result<PathBuf, std::io::Error> {
    use std::io::{Error, ErrorKind};
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(Error::new(
            ErrorKind::PermissionDenied,
            "Parent traversal in a write path",
        ));
    }
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut ancestor = absolute.as_path();
    while !ancestor.exists() {
        ancestor = ancestor
            .parent()
            .ok_or_else(|| Error::new(ErrorKind::PermissionDenied, "No existing write ancestor"))?;
    }
    let resolved = ancestor.canonicalize()?;
    if !resolved.starts_with(root) {
        return Err(Error::new(
            ErrorKind::PermissionDenied,
            format!(
                "PoB attempted a write outside evaluator scratch: {}",
                path.display()
            ),
        ));
    }
    Ok(resolved.join(
        absolute
            .strip_prefix(ancestor)
            .expect("ancestor of the same path"),
    ))
}
fn check_prompt(lua: &Lua) -> Result<(), RuntimeError> {
    let launch: Table = lua.globals().get("launch")?;
    if let Some(message) = launch.get::<Option<String>>("promptMsg")? {
        return Err(RuntimeError::Setup(format!(
            "PoB reported an error: {message}"
        )));
    }
    Ok(())
}
