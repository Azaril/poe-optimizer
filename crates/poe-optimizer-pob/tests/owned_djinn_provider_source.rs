//! Exact manual and allocated Djinn occurrence/support joins through complete source loads.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Function, Lua, LuaSerdeExt, Value};
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    ops::Range,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "complete_djinn_sources_keep_manual_and_allocated_supports_distinct";
const CHILD: &str = "POE_DJINN_PROVIDER_SOURCE_CHILD";
const OBSERVE: &str = include_str!("support/djinn_provider_source.lua");
const SAND: &str = "SummonSandDjinnPlayer";
const WATER: &str = "SummonWaterDjinnPlayer";

#[test]
fn complete_djinn_sources_keep_manual_and_allocated_supports_distinct() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-djinn-provider-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        run_child(&root, &out, mode == "on");
        return;
    }
    for mode in ["off", "on"] {
        let log_path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&log_path).unwrap();
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
                assert!(
                    status.success(),
                    "source child failed; log {} evidence {}\n{}",
                    log_path.display(),
                    out.join(format!("source-jit-{mode}.json")).display(),
                    tail(&log_path)
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(240) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "source deadline: {}\n{}",
                    log_path.display(),
                    tail(&log_path)
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        read(&out.join("source-jit-off.json")),
        read(&out.join("source-jit-on.json"))
    );
}

fn run_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
    let manifest = read(&fixtures.join("index.json"));
    let mut originals = Vec::new();
    for index in 1..=5 {
        let name = format!("build-{index:02}.xml");
        let text = fs::read_to_string(fixtures.join(&name)).unwrap();
        let entry = rows(&manifest["builds"])
            .iter()
            .find(|r| r["xml"] == name)
            .unwrap();
        assert_eq!(entry["xml_sha256"], digest(text.as_bytes()));
        originals.push((name, text));
    }
    let xml = &originals[4].1;
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([73; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let mut source_joins = Vec::new();
    for (ordinal, effect) in [
        (216, SAND),
        (217, "SupportBiddingPlayerTwo"),
        (218, "SupportMagnifiedAreaPlayer"),
        (219, "SupportMusterPlayer"),
        (229, WATER),
        (230, "SupportBiddingPlayerTwo"),
        (231, "SupportMusterPlayer"),
        (232, "SupportChillingIcePlayer"),
    ] {
        let row = evidence
            .rows()
            .iter()
            .find(|r| r.occurrence().id().ordinal() == ordinal)
            .unwrap();
        assert_eq!(row.occurrence().name(), "Gem");
        assert_eq!(
            row.attribute("skillId").and_then(|a| a.decoded().ok()),
            Some(effect)
        );
        source_joins.push(json!({"source":ordinal,"effect":effect}));
    }
    let mut cases = Vec::new();
    let mut source_hash = None;
    for (name, text) in controls(xml) {
        let before = |lua: &Lua| {
            lua.globals().set("djinnXml", text.as_str())?;
            lua.globals().set("djinnJit", enabled)?;
            lua.load("if djinnJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| -> Result<Function, RuntimeError> {
            lua.globals().set("djinnPhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@djinn-source-authentication")
                .eval()?)
        };
        let temp = tempfile::tempdir().unwrap();
        let observed = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            temp.path(),
            &text,
            None,
            false,
            Some(&before),
            Some(&install),
            Some(&observe),
        )
        .unwrap_or_else(|error| panic!("complete source case {name}: {error}"));
        assert_eq!(observed["configuration_method_wrappers"], false);
        assert_eq!(observed["original_build_output_available"], true);
        if let Some(hash) = &source_hash {
            assert_eq!(&observed["source_hash"], hash);
        } else {
            source_hash = Some(observed["source_hash"].clone());
        }
        cases.push(json!({"name":name,"xml_sha256":digest(text.as_bytes()),"state":observed["additional_observation"]}));
    }
    for (name, text) in &originals {
        assert_eq!(&fs::read_to_string(fixtures.join(name)).unwrap(), text);
    }
    let tree_bytes =
        fs::read_to_string(root.join("vendor/path-of-building-poe2/src/TreeData/0_5/tree.json"))
            .unwrap()
            .replace("\r\n", "\n")
            .into_bytes();
    assert_eq!(
        digest(&tree_bytes),
        "6449fe534c0265b21f59f8213254bd3f37a445887582c40aab04ad11cede3e95"
    );
    let tree: Json = serde_json::from_slice(&tree_bytes).unwrap();
    for (node, line) in [
        ("13289", "Grants Skill: Kelari, the Tainted Sands"),
        ("32705", "Grants Skill: Navira, the Last Mirage"),
    ] {
        assert!(
            rows(&tree["nodes"][node]["stats"])
                .iter()
                .any(|s| s == line)
        );
    }
    let result = json!({"source_hash":source_hash,"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4",
        "evidence":{"manifest_sha256":pinned::manifest_sha256(),"complete_loads_per_jit":13,"controls":12,"originals":originals.iter().map(|(name,text)|json!({"name":name,"sha256":digest(text.as_bytes())})).collect::<Vec<_>>(),"source_joins":source_joins,"tree_sha256":digest(&tree_bytes),"no_business_wrappers":true,"native_parity":false,
        "files":(["src/Modules/Common.lua","src/Classes/SkillsTab.lua","src/Classes/PassiveSpec.lua","src/Classes/TreeTab.lua","src/Classes/CalcsTab.lua","src/Classes/CompareTab.lua","src/Modules/Build.lua","src/Modules/CalcSetup.lua","src/Modules/CalcActiveSkill.lua","src/Modules/CalcPerform.lua","src/Modules/Calcs.lua","src/Modules/CalcTools.lua","src/Data/Skills/other.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})))},"cases":cases});
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

fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    lua.globals().set("djinnPhase", "observe")?;
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@djinn-source-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn controls(xml: &str) -> Vec<(String, String)> {
    let mut cases = vec![("original".into(), xml.to_owned())];
    for (label, effect, node) in [("sand", SAND, "13289"), ("water", WATER, "32705")] {
        cases.push((
            format!("remove-{label}-allocation"),
            edit_spec(xml, |s| {
                let nodes = attribute(s, "nodes");
                let saved: Vec<_> = nodes.split(',').collect();
                assert_eq!(saved.iter().filter(|v| **v == node).count(), 1);
                set_attribute(
                    s,
                    "nodes",
                    &saved
                        .into_iter()
                        .filter(|v| *v != node)
                        .collect::<Vec<_>>()
                        .join(","),
                )
            }),
        ));
        cases.push((
            format!("remove-{label}-saved-tree-group"),
            edit_group(xml, 4, effect, true, |_| String::new()),
        ));
        cases.push((
            format!("disable-{label}-manual"),
            edit_group(xml, 4, effect, false, |g| {
                set_attribute(g, "enabled", "false")
            }),
        ));
        cases.push((
            format!("level-one-{label}-manual"),
            edit_group(xml, 4, effect, false, |g| {
                edit_gem(g, 0, |v| set_attribute(v, "level", "1"))
            }),
        ));
        cases.push((
            format!("disable-{label}-bidding"),
            edit_group(xml, 4, effect, false, |g| {
                edit_gem(g, 1, |v| {
                    assert_eq!(attribute(v, "skillId"), "SupportBiddingPlayerTwo");
                    set_attribute(v, "enabled", "false")
                })
            }),
        ));
    }
    cases.push((
        "sand-main-minion-two".into(),
        edit_group(xml, 4, SAND, false, |g| {
            edit_gem(g, 0, |v| set_attribute(v, "skillMinionSkill", "2"))
        }),
    ));
    let mut archive = xml.to_owned();
    let mut changed = 0;
    for range in elements(xml, "SkillSet").into_iter().rev() {
        let set = &xml[range.clone()];
        if attribute(set, "id") == "4" {
            continue;
        }
        let mut next = set.to_owned();
        for gr in elements(set, "Skill").into_iter().rev() {
            let group = &set[gr.clone()];
            let header = &group[..=group.find('>').unwrap()];
            if !header.contains(" source=")
                && [SAND, WATER]
                    .iter()
                    .any(|e| group.contains(&format!("skillId=\"{e}\"")))
            {
                let value = set_attribute(
                    &edit_gem(group, 0, |g| set_attribute(g, "level", "1")),
                    "enabled",
                    "false",
                );
                next.replace_range(gr, &value);
                changed += 1;
            }
        }
        archive.replace_range(range, &next);
    }
    assert!(changed >= 2);
    cases.push(("archived-manual-changes".into(), archive));
    assert_eq!(cases.len(), 13);
    for (_, text) in cases.iter().skip(1) {
        assert_ne!(text, xml);
    }
    cases
}
fn elements(text: &str, name: &str) -> Vec<Range<usize>> {
    let open = format!("<{name} ");
    let close = format!("</{name}>");
    let mut from = 0;
    let mut out = Vec::new();
    while let Some(i) = text[from..].find(&open) {
        let begin = from + i;
        let end = begin + text[begin..].find(&close).unwrap() + close.len();
        out.push(begin..end);
        from = end;
    }
    out
}
fn attribute(text: &str, name: &str) -> String {
    let head = &text[..=text.find('>').unwrap()];
    let start = head.find(&format!(" {name}=\"")).unwrap() + name.len() + 3;
    head[start..start + head[start..].find('"').unwrap()].into()
}
fn set_attribute(text: &str, name: &str, value: &str) -> String {
    let old = attribute(text, name);
    let before = format!(" {name}=\"{old}\"");
    let mut out = text.to_owned();
    let at = text[..=text.find('>').unwrap()].find(&before).unwrap();
    out.replace_range(at..at + before.len(), &format!(" {name}=\"{value}\""));
    out
}
fn edit_spec(xml: &str, edit: impl FnOnce(&str) -> String) -> String {
    let range = elements(xml, "Spec")[2].clone();
    let mut out = xml.to_owned();
    out.replace_range(range.clone(), &edit(&xml[range]));
    out
}
fn edit_group(
    xml: &str,
    sid: u32,
    effect: &str,
    generated: bool,
    edit: impl FnOnce(&str) -> String,
) -> String {
    let set = elements(xml, "SkillSet")
        .into_iter()
        .find(|r| attribute(&xml[r.clone()], "id") == sid.to_string())
        .unwrap();
    let text = &xml[set.clone()];
    let groups: Vec<_> = elements(text, "Skill")
        .into_iter()
        .filter(|r| {
            let group = &text[r.clone()];
            let head = &group[..=group.find('>').unwrap()];
            head.contains(" source=\"Tree:") == generated
                && group.contains(&format!("skillId=\"{effect}\""))
        })
        .collect();
    assert_eq!(groups.len(), 1);
    let range = (set.start + groups[0].start)..(set.start + groups[0].end);
    let mut out = xml.to_owned();
    out.replace_range(range.clone(), &edit(&xml[range]));
    out
}
fn edit_gem(group: &str, index: usize, edit: impl FnOnce(&str) -> String) -> String {
    let begin = group.match_indices("<Gem ").nth(index).unwrap().0;
    let end = begin + group[begin..].find("/>").unwrap() + 2;
    let mut out = group.to_owned();
    out.replace_range(begin..end, &edit(&group[begin..end]));
    out
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn tail(path: &Path) -> String {
    let text = fs::read_to_string(path).unwrap_or_else(|e| e.to_string());
    let mut lines: Vec<_> = text.lines().rev().take(50).collect();
    lines.reverse();
    lines.join("\n")
}
fn rows(value: &Json) -> &[Json] {
    if let Some(v) = value.as_array() {
        v
    } else {
        assert!(
            value.as_object().is_some_and(|v| v.is_empty()),
            "expected rows: {value}"
        );
        &[]
    }
}
fn state<'a>(result: &'a Json, name: &str) -> &'a Json {
    &rows(&result["cases"])
        .iter()
        .find(|r| r["name"] == name)
        .unwrap()["state"]
}
fn group<'a>(s: &'a Json, effect: &str, generated: bool) -> Option<&'a Json> {
    let key = format!(
        "4/{effect}/{}",
        if generated {
            format!("Tree:{}", if effect == SAND { 13289 } else { 32705 })
        } else {
            "manual".into()
        }
    );
    let found: Vec<_> = rows(&s["runtime_groups"])
        .iter()
        .filter(|g| g["key"] == key)
        .collect();
    assert!(found.len() <= 1);
    found.first().copied()
}
fn action<'a>(g: &'a Json, mode: &str) -> &'a Json {
    let list = rows(&g["actions"][mode]);
    assert_eq!(list.len(), 2, "{} {mode}", g["key"]);
    let primary: Vec<_> = list
        .iter()
        .filter(|a| a["effect"] == g["state"]["gems"][0]["effect"])
        .collect();
    assert_eq!(primary.len(), 1, "{} {mode}", g["key"]);
    primary[0]
}
fn support_keys(a: &Json) -> Vec<Json> {
    rows(&a["supports"]["candidates"])
        .iter()
        .map(|r| json!([r["effect"], r["origin"]["key"], r["origin"]["index"]]))
        .collect()
}
fn support_applications(g: &Json, mode: &str) -> Vec<Json> {
    rows(&g["actions"][mode])
        .iter()
        .map(|a| {
            let children: Vec<_> = if a["minion_available"] == true {
                rows(&a["minion"]["children"])
                    .iter()
                    .map(|child| json!({"effect":child["effect"],"supports":child["supports"]}))
                    .collect()
            } else {
                Vec::new()
            };
            json!({"effect":a["effect"],"supports":a["supports"],"children":children})
        })
        .collect()
}
fn contextual(context: impl std::fmt::Display, check: impl FnOnce() + std::panic::UnwindSafe) {
    if let Err(error) = std::panic::catch_unwind(check) {
        let message = error
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied())
            .unwrap_or("non-string assertion failure");
        panic!("source assertion [{context}]: {message}");
    }
}
fn check(result: &Json) {
    assert_eq!(rows(&result["cases"]).len(), 13);
    for case in rows(&result["cases"]) {
        let s = &case["state"];
        assert_eq!(
            s["output_lifecycle"],
            json!({
                "module_reloads": 2, "recreated": true, "bytecode_equal": true,
                "calcs_upvalue_exact": true, "tab_calcs_exact": true, "compare_calcs_exact": true
            }),
            "{} output lifecycle",
            case["name"]
        );
        for flag in [
            "original_functions_preserved",
            "saved_instances_preserved",
            "selected_state_preserved",
            "outputs_preserved",
        ] {
            assert_eq!(s[flag], true, "{} {flag}", case["name"]);
        }
        for (key, value) in [("skills", 4), ("spec", 3), ("items", 2), ("config", 1)] {
            assert_eq!(
                s["selection"][key], value,
                "{} selection {key}",
                case["name"]
            );
        }
        for g in rows(&s["runtime_groups"]) {
            for mode in ["MAIN", "CALCS"] {
                contextual(format!("{} / {} / {mode}", case["name"], g["key"]), || {
                    let list = rows(&g["actions"][mode]);
                    if !list.is_empty() {
                        assert_eq!(list.len(), 2, "{} {} {mode}", case["name"], g["key"]);
                        let primary = action(g, mode);
                        assert_eq!(primary["granted_effect_index"], 1);
                        let command = list
                            .iter()
                            .find(|a| a["granted_effect_index"] == 2)
                            .unwrap();
                        assert_eq!(
                            command["effect"],
                            if primary["effect"] == SAND {
                                "CommandSandDjinnKnifeThrowPlayer"
                            } else {
                                assert_eq!(primary["effect"], WATER);
                                "CommandWaterDjinnBubblePlayer"
                            }
                        );
                        assert_eq!(command["level"], primary["level"]);
                        assert_eq!(command["quality"], primary["quality"]);
                        assert_eq!(command["source_instance"], primary["source_instance"]);
                    }
                    for a in rows(&g["actions"][mode]) {
                        assert_eq!(a["actor_is_player"], true);
                        assert_eq!(a["group_exact"], true);
                        assert_eq!(a["source_instance"]["key"], g["key"]);
                        let summon = a["effect"] == SAND || a["effect"] == WATER;
                        assert_eq!(
                            a["minion_available"], summon,
                            "{} {}",
                            case["name"], a["effect"]
                        );
                        if summon {
                            assert_eq!(a["minion"]["parent_is_player"], true);
                            assert_eq!(a["minion"]["enemy_exact"], true);
                            assert_eq!(
                                rows(&a["minion"]["children"]).len(),
                                if a["effect"] == SAND { 3 } else { 5 }
                            );
                        } else {
                            assert!(a["minion"].is_null());
                        }
                        for r in rows(&a["supports"]["candidates"]) {
                            assert_eq!(r["origin_known"], true);
                            assert_eq!(r["origin"]["key"], g["key"]);
                            assert!(r["origin"]["index"].as_u64().unwrap() > 1);
                        }
                        if summon {
                            for child in rows(&a["minion"]["children"]) {
                                assert_eq!(child["actor_is_parent_minion"], true);
                                assert_eq!(child["summon_skill_exact"], true);
                                assert_eq!(child["support_list_same_parent"], true);
                                assert_eq!(support_keys(child), support_keys(a));
                            }
                        }
                    }
                });
            }
        }
    }
    let base = state(result, "original");
    assert_eq!(base["selection"]["main_group"], 3);
    assert_eq!(base["selection"]["calcs_group"], 1);
    assert_eq!(
        base["modes"]["MAIN"]["main_effect"],
        "SummonSkeletalSnipersPlayer"
    );
    for (effect, label, source) in [(SAND, "sand", 216), (WATER, "water", 229)] {
        let manual = group(base, effect, false).unwrap();
        let generated = group(base, effect, true).unwrap();
        assert_eq!(manual["state"]["gems"][0]["fields"]["level"], 20);
        assert_eq!(manual["state"]["gems"][0]["fields"]["quality"], 0);
        assert_eq!(manual["state"]["gems"][0]["from_tree"], true);
        assert!(manual["state"]["source_node_id"].is_null());
        assert_eq!(generated["state"]["fields"]["noSupports"], true);
        for mode in ["MAIN", "CALCS"] {
            contextual(
                format!("{label} selected manual/generated contrasts / {mode}"),
                || {
                    let a = action(manual, mode);
                    assert_eq!(a["level"], 22);
                    assert_eq!(a["quality"], 0);
                    assert_eq!(a["minion"]["level"], 44);
                    assert_eq!(a["source_instance"]["source"], source);
                    assert_eq!(rows(&a["supports"]["candidates"]).len(), 3);
                    assert_eq!(a["supports"]["accepted_indices"], json!([1, 2, 3]));
                    assert!(rows(&a["supports"]["rejected_indices"]).is_empty());
                    for child in rows(&a["minion"]["children"]) {
                        assert_eq!(child["supports"]["accepted_indices"], json!([1, 2, 3]));
                        assert!(rows(&child["supports"]["rejected_indices"]).is_empty());
                    }
                    let command = rows(&manual["actions"][mode])
                        .iter()
                        .find(|a| a["granted_effect_index"] == 2)
                        .unwrap();
                    assert_eq!(command["supports"]["accepted_indices"], json!([1]));
                    assert_eq!(command["supports"]["rejected_indices"], json!([2, 3]));
                    assert_eq!(action(generated, mode)["level"], 3);
                    assert_eq!(action(generated, mode)["minion"]["level"], 6);
                    assert!(rows(&action(generated, mode)["supports"]["candidates"]).is_empty());
                    assert_eq!(generated["source_nodes"][mode]["exact_allocated"], true);
                    let removed = state(result, &format!("remove-{label}-allocation"));
                    assert!(group(removed, effect, true).is_none());
                    let node = if effect == SAND { 13289 } else { 32705 };
                    assert!(
                        !rows(&removed["modes"][mode]["allocated_ids"])
                            .iter()
                            .any(|id| *id == node),
                        "removed provider {node} remains allocated"
                    );
                    assert!(
                        !rows(&removed["modes"][mode]["grants"])
                            .iter()
                            .any(|grant| grant["source_node_id"] == node
                                || grant["fields"]["skillId"] == effect),
                        "removed provider {node} still grants {effect}"
                    );
                    let surviving_group = group(removed, effect, false).unwrap();
                    assert_eq!(surviving_group["source_nodes"][mode]["present"], false);
                    assert_eq!(
                        support_applications(surviving_group, mode),
                        support_applications(manual, mode),
                        "provider removal changed manual support application"
                    );
                    let surviving = action(surviving_group, mode);
                    assert_eq!(support_keys(surviving), support_keys(a));
                    assert_eq!(surviving["source_instance"]["source"], source);
                    let rebuilt = state(result, &format!("remove-{label}-saved-tree-group"));
                    let g = group(rebuilt, effect, true).unwrap();
                    assert_eq!(g["raw_present"], false);
                    assert!(rows(&action(g, mode)["supports"]["candidates"]).is_empty());
                    assert_eq!(
                        support_keys(action(group(rebuilt, effect, false).unwrap(), mode)),
                        support_keys(a)
                    );
                    let disabled = state(result, &format!("disable-{label}-manual"));
                    assert!(
                        rows(&group(disabled, effect, false).unwrap()["actions"][mode]).is_empty()
                    );
                    assert!(
                rows(
                    &action(group(disabled, effect, true).unwrap(), mode)["supports"]["candidates"]
                )
                .is_empty()
            );
                    let low = group(
                        state(result, &format!("level-one-{label}-manual")),
                        effect,
                        false,
                    )
                    .unwrap();
                    assert_eq!(low["state"]["gems"][0]["fields"]["level"], 1);
                    assert_eq!(action(low, mode)["level"], 3);
                    assert_eq!(action(low, mode)["minion"]["level"], 6);
                    let less_group = group(
                        state(result, &format!("disable-{label}-bidding")),
                        effect,
                        false,
                    )
                    .unwrap();
                    let less = action(less_group, mode);
                    assert_eq!(rows(&less["supports"]["candidates"]).len(), 2);
                    assert_eq!(support_keys(less), support_keys(a)[1..]);
                    assert_eq!(less["supports"]["accepted_indices"], json!([1, 2]));
                    assert!(rows(&less["supports"]["rejected_indices"]).is_empty());
                    for child in rows(&less["minion"]["children"]) {
                        assert_eq!(child["supports"]["accepted_indices"], json!([1, 2]));
                        assert!(rows(&child["supports"]["rejected_indices"]).is_empty());
                    }
                    let less_command = rows(&less_group["actions"][mode])
                        .iter()
                        .find(|a| a["granted_effect_index"] == 2)
                        .unwrap();
                    assert!(rows(&less_command["supports"]["accepted_indices"]).is_empty());
                    assert_eq!(less_command["supports"]["rejected_indices"], json!([1, 2]));
                    assert!(
                        !rows(&less["supports"]["candidates"])
                            .iter()
                            .any(|r| r["effect"] == "SupportBiddingPlayerTwo")
                    );
                },
            );
        }
    }
    // Saved numeric selectors are untouched; deleting/reconstructing the Sand
    // group legitimately changes which action occupies MAIN group three.
    for name in ["remove-sand-allocation", "remove-sand-saved-tree-group"] {
        let changed = state(result, name);
        assert_eq!(changed["selection"]["main_group"], 3);
        assert_eq!(
            changed["modes"]["MAIN"]["main_effect"],
            "SummonSkeletalFrostMagesPlayer"
        );
    }
    let changed = group(state(result, "sand-main-minion-two"), SAND, false).unwrap();
    assert_eq!(action(changed, "MAIN")["minion"]["selected_index"], 2);
    assert_eq!(action(changed, "CALCS")["minion"]["selected_index"], 1);
    let archived = state(result, "archived-manual-changes");
    for effect in [SAND, WATER] {
        for generated in [false, true] {
            assert_eq!(
                group(archived, effect, generated),
                group(base, effect, generated)
            );
        }
    }
    assert_eq!(archived["modes"], base["modes"]);
    assert_eq!(archived["outputs"], base["outputs"]);
}
