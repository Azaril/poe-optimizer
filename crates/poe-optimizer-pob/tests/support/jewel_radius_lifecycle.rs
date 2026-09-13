//! Source-only ordered radius lifecycle receipts; no native admission is inferred.
#[allow(dead_code)]
#[path = "configuration_preparation_source.rs"]
mod source;
use mlua::{Function, Lua, LuaSerdeExt, Table, Value};
use poe_optimizer_pob::runtime::RuntimeError;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};
const OBSERVER: &str = include_str!("jewel_radius_lifecycle.lua");
const TEST: &str = "all_five_original_jewel_radius_import_lifecycles";
const CHILD: &str = "POE_JEWEL_LIFECYCLE_CHILD";
const OUTPUT: &str = "POE_JEWEL_LIFECYCLE_OUTPUT";
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn host(
    repo: &Path,
    directory: &Path,
    xml: &str,
    warm: Option<&str>,
    observed: bool,
    structural: bool,
) -> Json {
    fs::create_dir_all(directory).unwrap();
    let module = Rc::new(RefCell::new(None::<Table>));
    let capture = Rc::new(RefCell::new(None::<Table>));
    let before_source = |lua: &Lua| -> Result<(), RuntimeError> {
        *module.borrow_mut() = Some(
            lua.load(OBSERVER)
                .set_name("@jewel_radius_lifecycle.lua")
                .eval()?,
        );
        Ok(())
    };
    let before_build = |_lua: &Lua| -> Result<Function, RuntimeError> {
        let active: Table = module
            .borrow()
            .as_ref()
            .unwrap()
            .raw_get::<Function>("start")?
            .call(observed)?;
        let finish = active.raw_get("finish")?;
        *capture.borrow_mut() = Some(active);
        Ok(finish)
    };
    let after = |lua: &Lua| -> Result<Json, RuntimeError> {
        let report: Table = capture.borrow().as_ref().unwrap().raw_get("report")?;
        assert_eq!(
            report.raw_get::<Table>("incomplete_calls")?.raw_len(),
            0,
            "completed import has incomplete exact call records"
        );
        let mut value: Json = lua.from_value(Value::Table(report))?;
        value["finite_post_import"]["items"]
            .as_array_mut()
            .unwrap()
            .sort_by_key(|row| serde_json::to_string(&row["id"]).unwrap());
        Ok(value)
    };
    source::observe_with_build_hook_unwrapped(
        &repo.join("vendor/path-of-building-poe2"),
        directory,
        xml,
        warm,
        structural,
        Some(&before_source),
        Some(&before_build),
        Some(&after),
    )
    .unwrap()
}
fn inspect(report: &Json, xml: &str) -> Json {
    let observation = &report["additional_observation"];
    let events = observation["events"].as_array().unwrap();
    assert!(!events.is_empty());
    let document = roxmltree::Document::parse(xml).unwrap();
    let root = document.root_element();
    let expected_items = root.children().filter(|n| n.has_tag_name("Items")).count();
    let expected_trees = root
        .children()
        .filter(|n| n.has_tag_name("Tree") || n.has_tag_name("Spec"))
        .count();
    let calls = |name: &str| {
        events
            .iter()
            .filter(|e| e["event"] == "call" && e["name"] == name)
            .collect::<Vec<_>>()
    };
    let items = calls("items_load");
    let trees = calls("tree_load");
    let setters = calls("radius_set");
    let parses = calls("parse_raw");
    assert_eq!(items.len(), expected_items);
    assert_eq!(trees.len(), expected_trees);
    assert!(!items.is_empty());
    let first_items = items[0]["ordinal"].as_u64().unwrap();
    let first_setter = setters
        .first()
        .expect("original build installs startup radius");
    assert!(first_setter["ordinal"].as_u64().unwrap() < first_items);
    // Source initialization is named by its actual observed argument, not final SelectedView.
    let startup_version = first_setter["requested_version"].clone();
    assert_eq!(
        startup_version, observation["configured_latest_tree_version"],
        "initial setter must use actual configured startup version"
    );
    let mut attributed = 0;
    for parse in &parses {
        if parse.get("items_xml_token").is_some() {
            attributed += 1;
            let ordinal = parse["ordinal"].as_u64().unwrap();
            assert!(
                trees
                    .iter()
                    .all(|tree| tree["ordinal"].as_u64().unwrap() > ordinal),
                "saved tree selected before an original Item parse"
            );
            let latest = setters
                .iter()
                .rev()
                .find(|setter| setter["ordinal"].as_u64().unwrap() < ordinal)
                .unwrap();
            assert_eq!(
                latest["requested_version"], startup_version,
                "Item parse used a different requested context"
            );
        }
    }
    assert!(attributed > 0);
    let last_items_return = events
        .iter()
        .filter(|e| e["event"] == "return" && e["name"] == "items_load")
        .map(|e| e["ordinal"].as_u64().unwrap())
        .max()
        .unwrap();
    assert!(
        trees
            .iter()
            .all(|tree| tree["ordinal"].as_u64().unwrap() > last_items_return)
    );
    json!({"events":events.len(),"items_load_calls":items.len(),"saved_tree_load_calls":trees.len(),"radius_set_calls":setters.len(),"parse_raw_calls":parses.len(),"parse_calls_in_actual_items_load":attributed,
        "startup_requested_version":startup_version,"all_items_before_saved_trees":true,"all_attributed_item_parses_use_startup_request":true,"source_only":true})
}
fn rearranged(xml: &str, mode: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let root = doc.root_element();
    let children = root
        .children()
        .filter(|n| n.is_element())
        .collect::<Vec<_>>();
    assert!(!children.is_empty());
    let prefix = &xml[..children[0].range().start];
    let suffix = &xml[children.last().unwrap().range().end..];
    let mut ordinary = Vec::new();
    let mut trees = Vec::new();
    let mut items = Vec::new();
    for node in &children {
        let text = &xml[node.range()];
        if node.has_tag_name("Tree") || node.has_tag_name("Spec") {
            trees.push(text)
        } else {
            ordinary.push(text)
        }
        if node.has_tag_name("Items") {
            items.push(text)
        }
    }
    assert!(!trees.is_empty() && !items.is_empty());
    let mut out = String::from(prefix);
    if mode == "tree_first" {
        for text in &trees {
            out.push_str(text);
            out.push('\n');
        }
    }
    for text in &ordinary {
        out.push_str(text);
        out.push('\n');
    }
    if mode == "repeated_items" {
        for text in &items {
            out.push_str(text);
            out.push('\n');
        }
    }
    if mode != "tree_first" {
        for text in &trees {
            out.push_str(text);
            out.push('\n');
        }
    }
    if mode == "repeated_tree" {
        for text in &trees {
            out.push_str(text);
            out.push('\n');
        }
    }
    out.push_str(suffix);
    out
}
fn child(repo: &Path, output: &Path, entry: &Json) {
    let name = entry["xml"].as_str().unwrap();
    let xml = fs::read_to_string(
        repo.join("tests/fixtures/builds/breadth-20260908")
            .join(name),
    )
    .unwrap();
    assert_eq!(hash(xml.as_bytes()), entry["xml_sha256"].as_str().unwrap());
    let mut cases = vec![("original".to_owned(), xml.clone(), false, false)];
    if name == "build-01.xml" {
        for mode in ["tree_first", "tree_last", "repeated_tree", "repeated_items"] {
            cases.push((mode.into(), rearranged(&xml, mode), false, true));
        }
        cases.push(("reused_host".into(), xml.clone(), true, false));
    }
    let mut receipts = Vec::new();
    for (label, input, reused, structural) in cases {
        let directory = output.join(name).join(&label);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("input.xml"), &input).unwrap();
        let warm = reused.then_some(xml.as_str());
        let control = host(
            repo,
            &directory.join("control-host"),
            &input,
            warm,
            false,
            structural,
        );
        let observed = host(
            repo,
            &directory.join("observed-host"),
            &input,
            warm,
            true,
            structural,
        );
        assert_eq!(
            control["additional_observation"]["finite_post_import"],
            observed["additional_observation"]["finite_post_import"],
            "{name} {label}: source observer changed declared finite post-import state"
        );
        assert_eq!(control["selected"], observed["selected"]);
        let proof = inspect(&observed, &input);
        fs::write(
            directory.join("control.json"),
            serde_json::to_vec_pretty(&control).unwrap(),
        )
        .unwrap();
        fs::write(
            directory.join("observed.json"),
            serde_json::to_vec_pretty(&observed).unwrap(),
        )
        .unwrap();
        receipts.push(json!({"label":label,"input_sha256":hash(input.as_bytes()),"reused_host":reused,"structural_root_order_probe":structural,"control_equal":true,"proof":proof}));
    }
    fs::write(output.join(format!("{name}.json")),serde_json::to_vec_pretty(&json!({"xml":name,"cases":receipts,"scope":"source-only interpreted call ordering and fixed finite control observables; no native or universal noninterference claim"})).unwrap()).unwrap();
}
pub fn run() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let index: Json = serde_json::from_slice(
        &fs::read(repo.join("tests/fixtures/builds/breadth-20260908/index.json")).unwrap(),
    )
    .unwrap();
    let output = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("runs/r2ab-jewel-01/lifecycle"));
    fs::create_dir_all(&output).unwrap();
    if let Ok(name) = std::env::var(CHILD) {
        child(
            &repo,
            &output,
            index["builds"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["xml"] == name)
                .unwrap(),
        );
        return;
    }
    let mut children = Vec::new();
    for entry in index["builds"].as_array().unwrap() {
        let name = entry["xml"].as_str().unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, name)
            .env(OUTPUT, &output)
            .current_dir(repo.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(
                fs::File::create(output.join(format!("{name}.stdout.log"))).unwrap(),
            ))
            .stderr(Stdio::from(
                fs::File::create(output.join(format!("{name}.stderr.log"))).unwrap(),
            ))
            .spawn()
            .unwrap();
        let start = Instant::now();
        let status = loop {
            if let Some(status) = process.try_wait().unwrap() {
                break status;
            }
            if start.elapsed() > Duration::from_secs(300) {
                process.kill().unwrap();
                let _ = process.wait();
                panic!("jewel lifecycle child timeout: {name}")
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        assert!(
            status.success(),
            "jewel lifecycle child failed {name}; inspect {}",
            output.display()
        );
        children.push(json!({"xml":name,"exit":status.code()}));
    }
    fs::write(output.join("summary.json"),serde_json::to_vec_pretty(&json!({"children":children,"source_only":true,"cases":10,"fresh_hosts":20,"reused_host_preloads":2,"native_parity_claim":false})).unwrap()).unwrap();
}
