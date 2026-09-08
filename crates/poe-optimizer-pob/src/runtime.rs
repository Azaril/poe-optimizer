//! Embedded evaluator implementation. Call only from a dedicated worker process.
//! Upstream source is trusted, revision-pinned code; the worker is not a security sandbox.

use mlua::{Function, Lua, LuaSerdeExt, MultiValue, Table, Value};
use poe_optimizer_core::{EvaluationSnapshot, RuntimeIdentity, options::EvaluationOptions};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
use thiserror::Error;

pub const UPSTREAM_REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const HOST: &str = include_str!("host.lua");
const SNAPSHOT: &str = include_str!("snapshot.lua");

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("{0}")]
    Setup(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Lua(#[from] mlua::Error),
    #[error(transparent)]
    Import(#[from] crate::import::ImportError),
    #[error(transparent)]
    Preflight(#[from] crate::preflight::PreflightError),
}

#[derive(Deserialize)]
struct LuaSnapshot {
    build: poe_optimizer_core::BuildSummary,
    player: poe_optimizer_core::ActorOutput,
    minion: Option<poe_optimizer_core::ActorOutput>,
    warnings: Vec<String>,
    export_xml: String,
}

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
/// Evaluates one imported build and returns diagnostic raw outputs and PoB's export.
/// The caller must supervise this process; Lua hooks are not a hard deadline.
pub fn evaluate(
    pob_root: &Path,
    scratch: &Path,
    xml: &str,
) -> Result<EvaluationSnapshot, RuntimeError> {
    evaluate_with_options(pob_root, scratch, xml, &EvaluationOptions::default())
}

pub fn evaluate_with_options(
    pob_root: &Path,
    scratch: &Path,
    xml: &str,
    options: &EvaluationOptions,
) -> Result<EvaluationSnapshot, RuntimeError> {
    let start = Instant::now();
    options.validate().map_err(RuntimeError::Setup)?;
    crate::import::decode_build(xml.as_bytes())?;
    crate::preflight::validate(xml)?;
    let root = pob_root.canonicalize()?;
    let source_hash =
        crate::source::verify(&root).map_err(|error| RuntimeError::Setup(error.to_string()))?;
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
    poe_optimizer_lua_utf8::register(&lua)?;
    let globals = lua.globals();
    globals.set("arg", lua.create_table()?)?;
    globals.set(
        "_optimizer_options",
        lua.to_value_with(
            options,
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
    let load: Function = globals.get("loadBuildFromXML")?;
    load.call::<()>((xml, "optimizer-input"))?;
    check_prompt(&lua)?;
    // Normal UI popups/early returns do not necessarily set launch.promptMsg.
    // Require a fully loaded build and a completed, newly allocated output environment.
    lua.load(r#"
        assert(main.mode == "BUILD" and not main.newMode, "PoB did not finish loading the requested build")
        assert(build.targetVersion == liveTargetVersion, "Unsupported build targetVersion; conversion required")
        assert(not build.abortSave, "PoB build initialization is incomplete")
        assert(#main.popups == 0, "PoB requires an interactive import decision")
        assert(build.calcsTab and build.spec and build.skillsTab, "Incomplete imported build")
        assert(build.spec.curClass.classes[build.spec.curAscendClassId], "Invalid imported ascendancy")
    "#).exec()?;
    lua.load(include_str!("options.lua"))
        .set_name("@optimizer-options.lua")
        .exec()?;
    check_prompt(&lua)?;
    lua.load(r#"
        local previous = build.calcsTab.mainEnv
        local revision = assert(build.outputRevision, "Missing calculation revision")
        build.buildFlag = true
        runCallback("OnFrame")
        assert(not build.buildFlag and build.outputRevision > revision, "PoB did not complete fresh calculation")
        assert(build.calcsTab.mainEnv ~= previous, "PoB retained an old calculation environment")
    "#).exec()?;
    check_prompt(&lua)?;
    globals
        .get::<Function>("_optimizer_validate_options")?
        .call::<()>(())?;
    let value: Value = lua
        .load(SNAPSHOT)
        .set_name("@optimizer-snapshot.lua")
        .eval()?;
    let mut snapshot: LuaSnapshot = lua.from_value(value)?;
    check_prompt(&lua)?;
    crate::import::decode_build(snapshot.export_xml.as_bytes())?;
    let mut coverage: poe_optimizer_core::coverage::BuildCoverage = lua.from_value(
        lua.load(include_str!("coverage.lua"))
            .set_name("@optimizer-coverage.lua")
            .eval()?,
    )?;
    let passives: poe_optimizer_core::coverage::PassiveCoverage = lua.from_value(
        lua.load(include_str!("passive_coverage.lua"))
            .set_name("@optimizer-passive-coverage.lua")
            .eval()?,
    )?;
    passives.validate().map_err(RuntimeError::Setup)?;
    coverage.passives = Some(passives);
    let context = lua.from_value(
        lua.load(include_str!("context.lua"))
            .set_name("@optimizer-context.lua")
            .eval()?,
    )?;
    if !coverage.tree_connections.is_empty() {
        snapshot.warnings.push(format!("{} passive-tree links reference node IDs missing from the pinned data; these are separate from valid ascendancy-tree components. Complete game-tree coverage is unverified", coverage.tree_connections.len()));
    }
    let jit: Table = globals.get("jit")?;
    let mut hash = Sha256::new();
    hash.update(include_str!("runtime.rs"));
    hash.update(poe_optimizer_data::implementation_fingerprint());
    hash.update(HOST);
    hash.update(SNAPSHOT);
    hash.update(include_str!("options.lua"));
    hash.update(include_str!("context.lua"));
    hash.update(include_str!("coverage.lua"));
    hash.update(include_str!("passive_coverage.lua"));
    hash.update(include_str!("metrics.rs"));
    hash.update(include_str!("backend.rs"));
    hash.update(include_str!("supervisor.rs"));
    hash.update(include_str!("../../poe-optimizer-core/src/evaluation.rs"));
    hash.update(include_str!("../../poe-optimizer-core/src/data.rs"));
    hash.update(include_str!("../../poe-optimizer-core/src/options.rs"));
    hash.update(include_str!("../../poe-optimizer-core/src/metrics.rs"));
    hash.update(include_str!("../../poe-optimizer-core/src/coverage.rs"));
    hash.update(include_str!("../../../Cargo.lock"));
    hash.update(include_str!("../../../Cargo.toml"));
    hash.update(include_str!("../Cargo.toml"));
    hash.update(include_str!("import.rs"));
    hash.update(include_str!("../../poe-optimizer-import/src/lib.rs"));
    hash.update(include_str!("../../poe-optimizer-import/Cargo.toml"));
    hash.update(include_str!("preflight.rs"));
    hash.update(include_str!("../../poe-optimizer-import/src/preflight.rs"));
    hash.update(include_str!("../../poe-optimizer-import/src/xml_compat.rs"));
    hash.update(include_str!("../../poe-optimizer-core/src/lib.rs"));
    hash.update(include_str!("../../poe-optimizer-core/Cargo.toml"));
    hash.update(include_str!("source.rs"));
    hash.update(include_str!("../../poe-optimizer-lua-utf8/src/lib.rs"));
    hash.update(include_str!("../../poe-optimizer-lua-utf8/build.rs"));
    hash.update(include_str!("../../poe-optimizer-lua-utf8/Cargo.toml"));
    for bytes in [
        include_bytes!("../../poe-optimizer-lua-utf8/vendor/luautf8/lutf8lib.c").as_slice(),
        include_bytes!("../../poe-optimizer-lua-utf8/vendor/luautf8/unidata.h").as_slice(),
        include_bytes!("../../poe-optimizer-lua-utf8/vendor/luajit-headers/lua.h").as_slice(),
        include_bytes!("../../poe-optimizer-lua-utf8/vendor/luajit-headers/lauxlib.h").as_slice(),
        include_bytes!("../../poe-optimizer-lua-utf8/vendor/luajit-headers/luaconf.h").as_slice(),
        include_bytes!("../../poe-optimizer-lua-utf8/vendor/luajit-headers/lualib.h").as_slice(),
    ] {
        hash.update(bytes);
    }
    Ok(EvaluationSnapshot {
        runtime: RuntimeIdentity {
            upstream_revision: UPSTREAM_REVISION.into(),
            source_hash,
            mlua_version: "0.12.1".into(),
            lua_version: jit.get("version")?,
            lua_arch: jit.get("arch")?,
            operating_system: std::env::consts::OS.into(),
            luajit_source: "210.7.3+1ee778a".into(),
            utf8_version: "0.1.6".into(),
            adapter_hash: format!("{:x}", hash.finalize()),
        },
        build: snapshot.build,
        coverage,
        context,
        player: snapshot.player,
        minion: snapshot.minion,
        warnings: snapshot.warnings,
        export_xml: snapshot.export_xml,
        elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
        diagnostics: String::new(),
        diagnostics_truncated: false,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_preserve_unc_and_reject_writes_outside_scratch() {
        assert_eq!(
            lua_path(Path::new(r"\\?\UNC\server\share\src")),
            "//server/share/src"
        );
        assert_eq!(lua_path(Path::new(r"\\?\C:\code\src")), "C:/code/src");
        let scratch = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let root = scratch.path().canonicalize().unwrap();
        assert!(checked_destination(&scratch.path().join("nested/file.xml"), &root).is_ok());
        assert!(checked_destination(&outside.path().join("file.xml"), &root).is_err());
        assert!(checked_destination(&scratch.path().join("../file.xml"), &root).is_err());
    }

    #[test]
    fn pinned_entrypoints_parse_without_source_overlays() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2/src");
        let lua = Lua::new();
        for file in ["HeadlessWrapper.lua", "Launch.lua", "Modules/Main.lua"] {
            lua.load(fs::read_to_string(root.join(file)).unwrap())
                .set_name(file)
                .into_function()
                .unwrap();
        }
        let count: u32 = lua
            .load("local count = 0; count += 1; return count")
            .eval()
            .unwrap();
        assert_eq!(count, 1);
    }
}
