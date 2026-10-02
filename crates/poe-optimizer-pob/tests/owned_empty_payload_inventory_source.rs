//! Complete original loader + all-granted-effect catalog evidence for an empty-only authored link lane.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/json_evidence.rs"]
mod json_evidence;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use json_evidence::first_difference;
use mlua::{Function, Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const TEST: &str = "complete_catalog_and_saved_groups_bound_empty_authored_payload_inventory";
const CHILD: &str = "POE_EMPTY_PAYLOAD_SOURCE_CHILD";
const OBSERVE: &str = include_str!("support/owned_empty_payload_inventory_source.lua");
const BASE_OBSERVE: &str = include_str!("support/owned_nonphysical_skill_source.lua");
const FILES: &[&str] = &[
    "src/Launch.lua",
    "src/GameVersions.lua",
    "src/Modules/Common.lua",
    "src/Modules/Main.lua",
    "src/Modules/Build.lua",
    "src/Modules/Data.lua",
    "src/Classes/SkillsTab.lua",
    "src/Classes/CalcsTab.lua",
    "src/Classes/ItemsTab.lua",
    "src/Classes/Item.lua",
    "src/Classes/PassiveSpec.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcActiveSkill.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/CalcOffence.lua",
    "src/Modules/CalcTools.lua",
    "src/Modules/CalcTriggers.lua",
    "src/Modules/Calcs.lua",
    "src/Modules/ModParser.lua",
    "src/Modules/ModTools.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModList.lua",
    "src/Classes/ModStore.lua",
    "src/Data/Gems.lua",
    "src/Data/Assets.lua",
    "src/Data/SkillStatMap.lua",
    "src/Data/Global.lua",
    "src/Data/Skills/SkillAssets.lua",
    "src/Data/Skills/act_int.lua",
    "src/Data/Skills/act_dex.lua",
    "src/Data/Skills/act_str.lua",
    "src/Data/Skills/sup_int.lua",
    "src/Data/Skills/sup_dex.lua",
    "src/Data/Skills/sup_str.lua",
    "src/Data/Skills/minion.lua",
    "src/Data/Skills/spectre.lua",
    "src/Data/Skills/other.lua",
    "src/TreeData/0_5/tree.lua",
    "runtime/lua/xml.lua",
];
struct Case {
    name: String,
    xml: String,
    warm: Option<String>,
    original: bool,
}
#[test]
fn complete_catalog_and_saved_groups_bound_empty_authored_payload_inventory() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-empty-payload-inventory-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        run_child(&root, &out, mode == "on");
        return;
    }
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, mode)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "{}: {}", path.display(), tail(&path));
                break;
            }
            if start.elapsed() > Duration::from_secs(1200) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline {}: {}", path.display(), tail(&path))
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    json_evidence::assert_files_equal(
        &out.join("source-jit-off.json"),
        &out.join("source-jit-on.json"),
        "exact payload inventory JIT evidence",
    );
}
fn run_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let dir = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json = serde_json::from_slice(&fs::read(dir.join("index.json")).unwrap()).unwrap();
    let originals: Vec<_> = (1..=5)
        .map(|n| {
            let name = format!("build-{n:02}.xml");
            let xml = fs::read_to_string(dir.join(&name)).unwrap();
            let entry = rows(&index["builds"])
                .iter()
                .find(|v| v["xml"] == name)
                .unwrap();
            assert_eq!(entry["xml_sha256"], digest(xml.as_bytes()));
            (name, xml)
        })
        .collect();
    let mut cases: Vec<_> = originals
        .iter()
        .enumerate()
        .map(|(i, (_, xml))| Case {
            name: format!("original-{:02}", i + 1),
            xml: xml.clone(),
            warm: None,
            original: true,
        })
        .collect();
    let fifth = &originals[4].1;
    let fourth = &originals[3].1;
    let disabled = change_group(fourth, "MetaCastOnDodgePlayer", "enabled", "false");
    push(&mut cases, "disabled-meta-group", disabled);
    push(
        &mut cases,
        "duplicate-active-row",
        duplicate_gem(fifth, "SummonSkeletalSnipersPlayer"),
    );
    push(
        &mut cases,
        "unknown-gem-selector",
        change_gem(
            fifth,
            "PainOfferingPlayer",
            &[
                ("gemId", Some("__unknown_payload_gem__")),
                ("nameSpec", Some("")),
            ],
        ),
    );
    push(
        &mut cases,
        "selected-contained-tornado",
        select_tornado(fourth, true),
    );
    push(
        &mut cases,
        "selected-independent-tornado",
        select_tornado(fourth, false),
    );
    push(
        &mut cases,
        "name-only-meta",
        change_gem(
            fourth,
            "MetaCastOnDodgePlayer",
            &[("gemId", None), ("variantId", None), ("skillId", None)],
        ),
    );
    cases.push(Case {
        name: "repeat-original-05".into(),
        xml: fifth.clone(),
        warm: None,
        original: true,
    });
    cases.push(Case {
        name: "warm-meta-to-original-05".into(),
        xml: fifth.clone(),
        warm: Some(fourth.clone()),
        original: true,
    });
    assert_eq!(cases.len(), 13);
    fs::create_dir_all(out.join("inputs")).unwrap();
    let mut observed = Vec::new();
    let mut catalog = None;
    let mut nonphysical = None;
    for case in &cases {
        eprintln!("complete payload inventory source case {}", case.name);
        fs::write(
            out.join("inputs").join(format!("{}.xml", case.name)),
            &case.xml,
        )
        .unwrap();
        let before = |lua: &Lua| {
            lua.globals().set("payloadJit", enabled)?;
            lua.load("if payloadJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| {
            lua.globals().set(
                "payloadInventoryBaseObserver",
                lua.load(BASE_OBSERVE)
                    .set_name("@maintained-saved-skill-observer")
                    .into_function()?,
            )?;
            lua.globals().set("payloadInventoryPhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@empty-payload-authentication")
                .eval::<Function>()?)
        };
        let after = |lua: &Lua| -> Result<Json, RuntimeError> {
            lua.globals().set("payloadInventoryPhase", "after")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@empty-payload-observation")
                .eval()?;
            Ok(lua.from_value(value)?)
        };
        let scratch = tempfile::tempdir().unwrap();
        let result = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &case.xml,
            case.warm.as_deref(),
            !case.original,
            Some(&before),
            Some(&install),
            Some(&after),
        );
        let row = match result {
            Ok(value) => {
                assert_eq!(value["configuration_method_wrappers"], false);
                assert_eq!(value["original_build_output_available"], true);
                assert_eq!(value["source_hash"], pinned::manifest_sha256());
                let mut state = value["additional_observation"].clone();
                let current = state
                    .as_object_mut()
                    .unwrap()
                    .remove("payload_catalog")
                    .unwrap();
                if let Some(prior) = &catalog {
                    check_catalog(prior, &current, out, &case.name, "catalog")
                } else {
                    catalog = Some(current)
                }
                let current = state
                    .as_object_mut()
                    .unwrap()
                    .remove("nonphysical_catalog")
                    .unwrap();
                if let Some(prior) = &nonphysical {
                    check_catalog(prior, &current, out, &case.name, "nonphysical-catalog")
                } else {
                    nonphysical = Some(current)
                }
                json!({"name":case.name,"available":true,"xml_sha256":digest(case.xml.as_bytes()),"warm_xml_sha256":case.warm.as_ref().map(|v|digest(v.as_bytes())),"xml_inventory":inventory(&case.xml),"state":state})
            }
            Err(error) => {
                json!({"name":case.name,"available":false,"source_error":error.to_string()})
            }
        };
        observed.push(row);
        fs::write(
            out.join(format!(
                "source-jit-{}-progress.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&observed).unwrap(),
        )
        .unwrap();
    }
    for (name, xml) in &originals {
        assert_eq!(fs::read_to_string(dir.join(name)).unwrap(), *xml)
    }
    let result = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","source_hash":pinned::manifest_sha256(),"catalog":catalog,"nonphysical_catalog":nonphysical,"evidence":{"case_count":cases.len(),"complete_load_attempts_per_jit":cases.len()+1,"observer_sha256":digest(OBSERVE.as_bytes()),"base_observer_sha256":digest(BASE_OBSERVE.as_bytes()),"business_method_wrappers":false,"native_coverage":false,"whole_build_parity":false,"files":FILES.iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),"originals":originals.iter().map(|(name,xml)|json!({"name":name,"sha256":digest(xml.as_bytes())})).collect::<Vec<_>>()},"cases":observed});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    check(&result);
}
fn check(source: &Json) {
    let cases = rows(&source["cases"]);
    assert_eq!(cases.len(), 13);
    for case in cases {
        assert_eq!(
            case["available"], true,
            "{}: {}",
            case["name"], case["source_error"]
        );
    }
    let catalog = rows(&source["catalog"]);
    assert!(catalog.len() > 500);
    for gem in catalog {
        assert!(!rows(&gem["effects"]).is_empty());
        assert_eq!(
            gem["non_container"],
            rows(&gem["effects"])
                .iter()
                .all(|e| e["non_container"] == true)
        );
    }
    let meta = catalog
        .iter()
        .find(|g| g["variant_id"] == "CastOnDodge")
        .unwrap();
    assert_eq!(meta["non_container"], false);
    assert!(
        rows(&meta["effects"])
            .iter()
            .any(|e| e["id"] == "SupportMetaCastOnDodgePlayer" && e["is_trigger"] == true)
    );
    for case in cases {
        for key in [
            "original_functions_preserved",
            "loaded_state_preserved",
            "saved_specs_preserved",
            "cached_outputs_preserved",
        ] {
            assert_eq!(case["state"][key], true, "{} {key}", case["name"])
        }
        assert_eq!(case["state"]["business_method_wrappers"], false);
        assert_eq!(
            rows(&case["state"]["payload_groups"]).len() as u64,
            case["xml_inventory"]["all_groups"].as_u64().unwrap()
        );
        let count: usize = rows(&case["state"]["payload_groups"])
            .iter()
            .map(|g| rows(&g["gems"]).len())
            .sum();
        assert_eq!(
            count as u64,
            case["xml_inventory"]["all_rows"].as_u64().unwrap()
        );
    }
    let original = named(cases, "original-05");
    assert!(
        selected_groups(original)
            .iter()
            .all(|g| g["loaded_shape_without_container"] == true)
    );
    let firebolt = catalog
        .iter()
        .find(|gem| gem["primary_effect"] == "FireboltPlayer")
        .unwrap();
    assert_eq!(firebolt["non_container"], true);
    let firebolt_runtime = rows(&original["state"]["runtime_effect_observations"])
        .iter()
        .find(|effect| effect["id"] == "FireboltPlayer")
        .unwrap();
    assert_eq!(firebolt_runtime["from_item"], true);
    let sniper = selected_groups(original)
        .into_iter()
        .flat_map(|g| rows(&g["gems"]))
        .find(|g| g["resolved"]["resolved_effect"] == "SummonSkeletalSnipersPlayer")
        .unwrap();
    assert_eq!(sniper["classification"], "gem");
    assert!(
        catalog.iter().any(|gem| rows(&gem["effects"]).len() > 1),
        "catalog enumerates secondary granted effects"
    );
    let fourth = named(cases, "original-04");
    let tornados: Vec<_> = rows(&fourth["state"]["payload_main"])
        .iter()
        .filter(|s| s["effect"] == "TornadoPlayer")
        .collect();
    assert_eq!(tornados.len(), 2);
    assert_ne!(tornados[0]["origin"], tornados[1]["origin"]);
    for (name, triggered) in [
        ("selected-contained-tornado", true),
        ("selected-independent-tornado", false),
    ] {
        let case = named(cases, name);
        let skill = rows(&case["state"]["payload_main"])
            .iter()
            .find(|s| s["selected"] == true)
            .unwrap();
        assert_eq!(skill["effect"], "TornadoPlayer");
        assert_eq!(skill["flags_present"], true);
        assert_eq!(
            skill["trigger_effect"] == "SupportMetaCastOnDodgePlayer",
            triggered,
            "{name}"
        );
        if triggered {
            assert_eq!(
                skill["origin"]["observer_group"],
                skill["trigger_origin"]["observer_group"]
            );
            assert_ne!(
                skill["origin"]["gem_index"],
                skill["trigger_origin"]["gem_index"]
            );
        }
    }
    assert!(
        selected_groups(named(cases, "disabled-meta-group"))
            .iter()
            .any(|g| g["enabled"] == false && g["loaded_shape_without_container"] == false)
    );
    assert!(
        selected_groups(named(cases, "duplicate-active-row"))
            .iter()
            .any(|g| g["non_support_rows"] == 2)
    );
    assert!(
        selected_groups(named(cases, "unknown-gem-selector"))
            .iter()
            .flat_map(|g| rows(&g["gems"]))
            .any(|g| g["classification"] == "unresolved")
    );
    for name in ["repeat-original-05", "warm-meta-to-original-05"] {
        let other = named(cases, name);
        for key in [
            "selected",
            "loaded_groups",
            "payload_groups",
            "payload_main",
            "main_output",
            "calcs_output",
        ] {
            assert_eq!(other["state"][key], original["state"][key], "{name}: {key}")
        }
    }
    let direct = &rows(&source["nonphysical_catalog"])[0];
    assert_eq!(direct["id"], "EnemyExplode");
    assert_eq!(direct["non_container"], true);
    assert_eq!(direct["support"], false);
    assert_eq!(direct["gem_mapping_by_object_present"], false);
    assert_eq!(direct["gem_mapping_by_id_present"], false);
}
fn selected_groups(case: &Json) -> Vec<&Json> {
    rows(&case["state"]["payload_groups"])
        .iter()
        .filter(|g| g["skill_set"] == case["state"]["selected"]["skills"])
        .collect()
}
fn rows(v: &Json) -> &[Json] {
    if let Some(v) = v.as_array() {
        v
    } else {
        assert!(v.as_object().is_some_and(|m| m.is_empty()), "not list: {v}");
        &[]
    }
}
fn named<'a>(rows: &'a [Json], name: &str) -> &'a Json {
    rows.iter().find(|r| r["name"] == name).unwrap()
}
fn push(cases: &mut Vec<Case>, name: &str, xml: String) {
    cases.push(Case {
        name: name.into(),
        xml,
        warm: None,
        original: false,
    })
}
fn digest(v: &[u8]) -> String {
    format!("{:x}", Sha256::digest(v))
}
fn check_catalog(prior: &Json, current: &Json, out: &Path, case: &str, label: &str) {
    if let Some(difference) = first_difference(prior, current, "$") {
        for (suffix, value) in [("initial", prior), (case, current)] {
            fs::write(
                out.join(format!("{label}-diagnostic-{suffix}.json")),
                serde_json::to_vec_pretty(value).unwrap(),
            )
            .unwrap();
        }
        panic!("{label} changed in {case}: {difference}; full values saved as diagnostic JSON");
    }
}
fn tail(path: &Path) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = fs::File::open(path).unwrap();
    let n = f.metadata().unwrap().len();
    f.seek(SeekFrom::Start(n.saturating_sub(12000))).unwrap();
    let mut b = Vec::new();
    f.read_to_end(&mut b).unwrap();
    String::from_utf8_lossy(&b).into()
}
fn selected_set<'a, 'i>(doc: &'a roxmltree::Document<'i>) -> roxmltree::Node<'a, 'i> {
    let skills = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    skills
        .children()
        .find(|n| {
            n.has_tag_name("SkillSet") && n.attribute("id") == skills.attribute("activeSkillSet")
        })
        .unwrap()
}
fn gem<'a, 'i>(doc: &'a roxmltree::Document<'i>, effect: &str) -> roxmltree::Node<'a, 'i> {
    selected_set(doc)
        .descendants()
        .find(|n| n.has_tag_name("Gem") && n.attribute("skillId") == Some(effect))
        .unwrap()
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn change_gem(xml: &str, effect: &str, changes: &[(&str, Option<&str>)]) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let node = gem(&doc, effect);
    let mut attrs: std::collections::BTreeMap<_, _> =
        node.attributes().map(|a| (a.name(), a.value())).collect();
    for (k, v) in changes {
        if let Some(v) = v {
            attrs.insert(k, v);
        } else {
            attrs.remove(k);
        }
    }
    let text = attrs
        .into_iter()
        .map(|(k, v)| format!("{k}=\"{}\"", escape(v)))
        .collect::<Vec<_>>()
        .join(" ");
    let mut out = xml.to_owned();
    out.replace_range(node.range(), &format!("<Gem {text}/>"));
    out
}
fn change_group(xml: &str, effect: &str, key: &str, value: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let node = gem(&doc, effect).parent().unwrap();
    let mut attrs: std::collections::BTreeMap<_, _> =
        node.attributes().map(|a| (a.name(), a.value())).collect();
    attrs.insert(key, value);
    let text = attrs
        .into_iter()
        .map(|(k, v)| format!("{k}=\"{}\"", escape(v)))
        .collect::<Vec<_>>()
        .join(" ");
    let range = node.range();
    let end = range.start + xml[range.clone()].find('>').unwrap() + 1;
    let mut out = xml.to_owned();
    out.replace_range(range.start..end, &format!("<Skill {text}>"));
    out
}
fn duplicate_gem(xml: &str, effect: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let node = gem(&doc, effect);
    let mut out = xml.to_owned();
    out.insert_str(node.range().end, &xml[node.range()]);
    out
}
fn select_tornado(xml: &str, contained: bool) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let groups: Vec<_> = selected_set(&doc)
        .children()
        .filter(|n| n.has_tag_name("Skill"))
        .collect();
    let (index, group) = groups
        .iter()
        .enumerate()
        .find(|(_, g)| {
            g.children()
                .any(|n| n.attribute("skillId") == Some("TornadoPlayer"))
                && g.children()
                    .any(|n| n.attribute("skillId") == Some("MetaCastOnDodgePlayer"))
                    == contained
        })
        .unwrap();
    let mut out = xml.to_owned();
    let raw = &xml[group.range()];
    let start = raw.find('>').unwrap() + 1;
    let mut attrs: std::collections::BTreeMap<_, _> = group
        .attributes()
        .map(|a| (a.name().to_owned(), a.value().to_owned()))
        .collect();
    attrs.insert(
        "mainActiveSkill".into(),
        if contained { "2" } else { "1" }.into(),
    );
    let opening = format!(
        "<Skill {}>",
        attrs
            .iter()
            .map(|(k, v)| format!("{k}=\"{}\"", escape(v)))
            .collect::<Vec<_>>()
            .join(" ")
    );
    out.replace_range(group.range().start..group.range().start + start, &opening);
    let current = roxmltree::Document::parse(&out).unwrap();
    let build = current
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Build"))
        .unwrap();
    let range = build.range();
    let end = range.start + out[range.clone()].find('>').unwrap() + 1;
    let mut attrs: std::collections::BTreeMap<_, _> = build
        .attributes()
        .map(|a| (a.name().to_owned(), a.value().to_owned()))
        .collect();
    attrs.insert("mainSocketGroup".into(), (index + 1).to_string());
    let text = format!(
        "<Build {}>",
        attrs
            .iter()
            .map(|(k, v)| format!("{k}=\"{}\"", escape(v)))
            .collect::<Vec<_>>()
            .join(" ")
    );
    out.replace_range(range.start..end, &text);
    out
}
fn inventory(xml: &str) -> Json {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let skills = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let groups: Vec<_> = skills
        .children()
        .filter(|n| n.has_tag_name("SkillSet"))
        .flat_map(|n| n.children().filter(|c| c.has_tag_name("Skill")))
        .collect();
    json!({"all_groups":groups.len(),"all_rows":groups.iter().map(|g|g.children().filter(|c|c.is_element()).count()).sum::<usize>()})
}
