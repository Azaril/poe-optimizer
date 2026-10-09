//! Shared complete-source loading and process supervision for resource witnesses.
#[allow(dead_code)]
#[path = "configuration_preparation_source.rs"]
mod source;
use mlua::{Lua, LuaSerdeExt, Table, Value};
use poe_optimizer_pob::runtime::RuntimeError;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn observe(root: &Path, xml: &str, warm: Option<&str>, jit: bool, observer: &str) -> Json {
    let before = |lua: &Lua| {
        lua.load(if jit {
            "jit.on()"
        } else {
            "jit.off();jit.flush()"
        })
        .exec()?;
        Ok(())
    };
    let after = |lua: &Lua| -> Result<Json, RuntimeError> {
        assert_eq!(lua.load("return jit.status()").eval::<bool>()?, jit);
        let t: Table = lua
            .load(observer)
            .set_name("@player_resource_source.lua")
            .eval()?;
        assert_eq!(lua.load("return jit.status()").eval::<bool>()?, jit);
        Ok(lua.from_value(Value::Table(t))?)
    };
    let scratch = tempfile::tempdir().unwrap();
    let report = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        xml,
        warm,
        false,
        Some(&before),
        None,
        Some(&after),
    )
    .unwrap();
    assert_eq!(report["configuration_method_wrappers"], false);
    assert_eq!(report["original_build_output_available"], true);
    json!({"source_hash":report["source_hash"],"selected":report["selected"],"state":report["additional_observation"]})
}
pub fn originals(root: &Path) -> Vec<String> {
    let dir = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json = serde_json::from_slice(&fs::read(dir.join("index.json")).unwrap()).unwrap();
    (1..=5)
        .map(|i| {
            let name = format!("build-{i:02}.xml");
            let xml = fs::read_to_string(dir.join(&name)).unwrap();
            let pin = index["builds"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["xml"] == name)
                .unwrap();
            assert_eq!(pin["xml_sha256"], hash(xml.as_bytes()));
            xml
        })
        .collect()
}
pub fn supervise(test: &str, child_env: &str, output_env: &str, child: fn(&Path, &Path, bool)) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let output = PathBuf::from(std::env::var_os(output_env).expect("fresh source output"));
    let output = if output.is_absolute() {
        output
    } else {
        root.join(output)
    };
    if let Some(mode) = std::env::var_os(child_env) {
        assert!(mode == "on" || mode == "off");
        child(&root, &output, mode == "on");
        return;
    }
    assert!(!output.exists());
    fs::create_dir_all(&output).unwrap();
    for mode in ["off", "on"] {
        let log = fs::File::create(output.join(format!("source-jit-{mode}.log"))).unwrap();
        let mut p = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", test, "--nocapture"])
            .env(child_env, mode)
            .env(output_env, &output)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = p.try_wait().unwrap() {
                assert!(status.success(), "see source-jit-{mode}.log");
                break;
            }
            if start.elapsed() > Duration::from_secs(600) {
                p.kill().unwrap();
                p.wait().unwrap();
                panic!("source deadline");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        fs::read(output.join("source-jit-off.json")).unwrap(),
        fs::read(output.join("source-jit-on.json")).unwrap()
    );
}
