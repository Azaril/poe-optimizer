//! Source-only participation/preview controls. No native activation rule or exception.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Function, Lua, LuaSerdeExt, Table, Value};
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
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const TEST: &str = "selected_participation_preserves_exact_source_identity_and_preview_outcomes";
const CHILD: &str = "POE_SELECTED_PARTICIPATION_CHILD";
const OUTPUT: &str = "POE_OPTIMIZER_TEST_SELECTED_PARTICIPATION_SOURCE_OUT";
const ORIGINAL: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";
const AUTH: &str = include_str!("support/djinn_provider_source.lua");
const LOADER: &str = include_str!("support/sniper_actor_action_source.lua");
const MEMBERSHIP: &str = include_str!("support/authored_skill_membership_source.lua");
const OBSERVER: &str = include_str!("support/selected_participation_source.lua");
const STAGES: [&str; 3] = ["fresh", "rebuilt_once", "rebuilt_twice"];
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|o| o.is_empty()));
        &[]
    }
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn edit(xml: &str, node: roxmltree::Node<'_, '_>, name: &str, value: &str) -> String {
    assert!(
        node.attribute(name).is_some(),
        "control changes an existing attribute only"
    );
    let start = node.range().start;
    let end = start + xml[start..].find('>').unwrap() + 1;
    let mut header = format!("<{}", node.tag_name().name());
    for attr in node.attributes() {
        header.push_str(&format!(
            " {}=\"{}\"",
            attr.name(),
            escape(if attr.name() == name {
                value
            } else {
                attr.value()
            })
        ));
    }
    if xml[start..end].ends_with("/>") {
        header.push('/');
    }
    header.push('>');
    let mut result = xml.to_owned();
    result.replace_range(start..end, &header);
    result
}
fn group<'a, 'b>(doc: &'a roxmltree::Document<'b>, index: usize) -> roxmltree::Node<'a, 'b> {
    doc.descendants()
        .find(|n| n.has_tag_name("SkillSet") && n.attribute("id") == Some("4"))
        .unwrap()
        .children()
        .filter(|n| n.has_tag_name("Skill"))
        .nth(index - 1)
        .unwrap()
}
fn focus(xml: &str, index: usize) -> String {
    let text = index.to_string();
    let doc = roxmltree::Document::parse(xml).unwrap();
    let build = doc.descendants().find(|n| n.has_tag_name("Build")).unwrap();
    let xml = edit(xml, build, "mainSocketGroup", &text);
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let input = doc
        .descendants()
        .find(|n| n.has_tag_name("Input") && n.attribute("name") == Some("skill_number"))
        .unwrap();
    edit(&xml, input, "number", &text)
}
fn disable(xml: &str, index: usize, gem: bool) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let g = group(&doc, index);
    let node = if gem {
        g.children().find(|n| n.has_tag_name("Gem")).unwrap()
    } else {
        g
    };
    assert_eq!(node.attribute("enabled"), Some("true"));
    edit(xml, node, "enabled", "false")
}
fn cases(xml: &str) -> Vec<(&'static str, String)> {
    let sniper = focus(xml, 3);
    let fire = focus(xml, 12);
    let sand = focus(xml, 2);
    let doc = roxmltree::Document::parse(&fire).unwrap();
    let set = doc
        .descendants()
        .find(|n| n.has_tag_name("ItemSet") && n.attribute("id") == Some("2"))
        .unwrap();
    let slot = set
        .children()
        .find(|n| n.has_tag_name("Slot") && n.attribute("name") == Some("Weapon 1"))
        .unwrap();
    assert_eq!(slot.attribute("itemId"), Some("28"));
    let unequipped = edit(&fire, slot, "itemId", "0");
    let doc = roxmltree::Document::parse(&sand).unwrap();
    let spec = doc
        .descendants()
        .filter(|n| n.has_tag_name("Spec"))
        .nth(2)
        .unwrap();
    let nodes: Vec<_> = spec.attribute("nodes").unwrap().split(',').collect();
    assert_eq!(nodes.iter().filter(|n| **n == "13289").count(), 1);
    let removed = nodes
        .into_iter()
        .filter(|n| *n != "13289")
        .collect::<Vec<_>>()
        .join(",");
    let unallocated = edit(&sand, spec, "nodes", &removed);
    vec![
        ("original-05", xml.to_owned()),
        ("physical-sniper-focused", sniper.clone()),
        (
            "physical-sniper-selected-group-disabled",
            disable(&sniper, 3, false),
        ),
        (
            "physical-sniper-selected-gem-disabled",
            disable(&sniper, 3, true),
        ),
        (
            "physical-sniper-nonselected-arsonist-group-disabled",
            disable(&sniper, 1, false),
        ),
        ("generated-firebolt-focused", fire.clone()),
        (
            "generated-firebolt-selected-group-disabled",
            disable(&fire, 12, false),
        ),
        (
            "generated-firebolt-selected-gem-disabled",
            disable(&fire, 12, true),
        ),
        ("generated-firebolt-provider-unequipped", unequipped),
        ("generated-sand-focused", sand.clone()),
        (
            "generated-sand-selected-group-disabled",
            disable(&sand, 2, false),
        ),
        ("generated-sand-provider-unallocated", unallocated),
    ]
}
fn source_rows(xml: &str) -> Json {
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([53; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    json!(evidence.rows().iter().map(|r| json!({"ordinal":r.occurrence().id().ordinal(),"name":r.occurrence().name(),
        "attributes":r.attributes().iter().map(|a| (a.origin().name.clone(), json!(a.decoded().unwrap()))).collect::<serde_json::Map<_,_>>() })).collect::<Vec<_>>())
}
fn changed_fields(original: &Json, changed: &Json) -> Json {
    assert_eq!(rows(original).len(), rows(changed).len());
    let mut edits = vec![];
    for (a, b) in rows(original).iter().zip(rows(changed)) {
        assert_eq!(a["ordinal"], b["ordinal"]);
        assert_eq!(a["name"], b["name"]);
        let left = a["attributes"].as_object().unwrap();
        let right = b["attributes"].as_object().unwrap();
        assert_eq!(
            left.keys().collect::<Vec<_>>(),
            right.keys().collect::<Vec<_>>()
        );
        for (name, before) in left {
            if before != &right[name] {
                assert!(matches!(
                    name.as_str(),
                    "mainSocketGroup" | "number" | "enabled" | "itemId" | "nodes"
                ));
                edits.push(json!({"ordinal":a["ordinal"],"element":a["name"],"attribute":name,"before":before,"after":right[name]}));
            }
        }
    }
    json!(edits)
}
fn observe(root: &Path, xml: &str, enabled: bool, provenance: bool) -> Json {
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("membershipXml", xml)?;
        lua.globals().set("sniperActorJit", enabled)?;
        lua.globals().set("participationJit", enabled)?;
        lua.load("if participationJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let install = |lua: &Lua| -> Result<Function, RuntimeError> {
        lua.globals().set("djinnPhase", "before")?;
        let finish: Function = lua
            .load(AUTH)
            .set_name("@participation-original-lifecycle-authentication")
            .eval()?;
        lua.globals().set("sniperOriginalFinish", finish)?;
        lua.globals().set("sniperActorPhase", "before")?;
        Ok(lua
            .load(LOADER)
            .set_name("@participation-original-loader-object-observer")
            .eval()?)
    };
    let hook = |lua: &Lua| -> Result<Json, RuntimeError> {
        let observer: Table = lua
            .load(OBSERVER)
            .set_name("@selected_participation_source.lua")
            .eval()?;
        let capture: Function = observer.get("capture")?;
        let rebuild: Function = observer.get("rebuild")?;
        let mut stages = serde_json::Map::new();
        for (i, stage) in STAGES.into_iter().enumerate() {
            if i != 0 {
                rebuild.call::<()>(())?;
            }
            let base: Value = if provenance {
                lua.load(MEMBERSHIP)
                    .set_name("@participation-exact-source-membership")
                    .eval()?
            } else {
                Value::Nil
            };
            let result: Value = capture.call(base)?;
            stages.insert(stage.into(), lua.from_value(result)?);
        }
        Ok(Json::Object(stages))
    };
    let scratch = tempfile::tempdir().unwrap();
    let report = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        xml,
        None,
        false,
        Some(&before),
        if provenance { Some(&install) } else { None },
        Some(&hook),
    )
    .unwrap();
    assert_eq!(report["configuration_method_wrappers"], false);
    assert_eq!(report["original_build_output_available"], true);
    json!({"source_hash":report["source_hash"],"states":report["additional_observation"]})
}
fn check_case(name: &str, observed: &Json, sources: &Json) {
    for stage in STAGES {
        let v = &observed["states"][stage];
        let state = &v["state"];
        assert_eq!(state["selection"]["skills"], 4, "{name}/{stage}");
        assert_eq!(state["selection"]["items"], 2);
        assert_eq!(state["selection"]["spec"], 3);
        assert_eq!(v["source_methods_preserved"], true);
        assert_eq!(v["outputs_preserved"], true);
        assert_eq!(v["calculation_hook"], false);
        assert_eq!(v["business_wrappers"], false);
        assert_eq!(v["native_participation_authority"], false);
        let saved = rows(&v["provenance"]["saved"]);
        assert_eq!(saved.len(), 12);
        for group in saved {
            let ordinal = group["source_ordinal"].as_u64().unwrap() as usize;
            assert_eq!(
                group["attributes"], sources[ordinal]["attributes"],
                "{name}/{stage}/group{ordinal}"
            );
            assert_eq!(group["loaded_object_exact"], true);
            for gem in rows(&group["gems"]) {
                let ordinal = gem["source_ordinal"].as_u64().unwrap() as usize;
                assert_eq!(
                    gem["source"]["attributes"], sources[ordinal]["attributes"],
                    "{name}/{stage}/Gem{ordinal}"
                );
                assert_eq!(gem["loaded_object_exact"], true);
            }
        }
        for mode in ["MAIN", "CALCS"] {
            let m = &state["modes"][mode];
            assert_eq!(m["output_present"], true, "{name}/{stage}/{mode}");
            let active = rows(&m["active"]);
            assert_eq!(active.iter().filter(|a| a["is_main"] == true).count(), 1);
            assert!(m["main"]["identity"]["effect"].is_string());
            if m["default_unarmed_without_source"] == true {
                assert_eq!(m["main"]["identity"]["effect"], "MeleeUnarmedPlayer");
                assert_eq!(m["main"]["source_present"], false);
            }
            if name == "original-05" {
                assert_eq!(
                    m["main"]["identity"]["effect"],
                    if mode == "MAIN" {
                        "SummonSkeletalSnipersPlayer"
                    } else {
                        "SummonSkeletalArsonistsPlayer"
                    }
                );
            }
            if name.contains("physical-sniper") && !name.ends_with("selected-gem-disabled") {
                assert_eq!(
                    m["main"]["identity"]["effect"], "SummonSkeletalSnipersPlayer",
                    "{name}/{stage}/{mode}"
                );
                assert_eq!(m["main"]["group_index"], 3);
            }
            if name.ends_with("selected-gem-disabled") {
                assert_eq!(
                    m["default_unarmed_without_source"], true,
                    "{name}/{stage}/{mode}"
                );
            }
            if name == "physical-sniper-selected-group-disabled" {
                assert_eq!(m["main"]["group"]["enabled"], false);
            }
            if name == "physical-sniper-nonselected-arsonist-group-disabled" {
                assert!(
                    !active
                        .iter()
                        .any(|a| a["identity"]["effect"] == "SummonSkeletalArsonistsPlayer")
                );
            }
            if name.starts_with("generated-firebolt")
                && !name.ends_with("selected-gem-disabled")
                && !name.ends_with("provider-unequipped")
            {
                assert_eq!(
                    m["main"]["identity"]["effect"], "FireboltPlayer",
                    "{name}/{stage}/{mode}"
                );
                assert_eq!(m["main"]["source_item_id"], 28);
                assert_eq!(m["main"]["source_item_equipped"], true);
            }
            if name == "generated-firebolt-selected-group-disabled" {
                assert_eq!(m["main"]["group"]["enabled"], false);
            }
            if name == "generated-firebolt-provider-unequipped" {
                assert_ne!(m["weapon_one_item_id"]["value"], 28);
                assert!(
                    !active.iter().any(|a| a["source_item_id"] == 28),
                    "{name}/{stage}/{mode}: removed item still supplies an effect"
                );
            }
            if name == "generated-sand-provider-unallocated" {
                assert_eq!(m["node_13289_allocated"], false, "{name}/{stage}/{mode}");
                assert!(!active.iter().any(|a| a["source_node_id"] == 13289));
            }
            if name == "generated-sand-focused" || name == "generated-sand-selected-group-disabled"
            {
                assert_eq!(
                    m["main"]["identity"]["effect"], "SummonSandDjinnPlayer",
                    "{name}/{stage}/{mode}"
                );
                assert_eq!(m["main"]["source_node_id"], 13289);
                assert_eq!(m["main"]["source_node_allocated"], true);
                if name.ends_with("group-disabled") {
                    assert_eq!(m["main"]["group"]["enabled"], false);
                }
            }
            // The authored/manual same-definition Sand source remains distinct
            // even if the generated source is unavailable or preview-selected.
            if name.starts_with("generated-sand") {
                assert!(
                    active.iter().any(|a| a["group"]["source"].is_null()
                        && a["identity"]["effect"] == "SummonSandDjinnPlayer"),
                    "{name}/{stage}/{mode}: manual contrast missing"
                );
            }
        }
    }
}
#[test]
#[ignore = "requires complete pinned PoB; source evidence only, no native participation decision"]
fn selected_participation_preserves_exact_source_identity_and_preview_outcomes() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join("runs/owned-selected-participation-source-01"));
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let path = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
        let xml = fs::read_to_string(&path).unwrap();
        assert_eq!(
            pinned::manifest_sha256(),
            "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
        );
        assert_eq!(hash(xml.as_bytes()), ORIGINAL);
        let original_rows = source_rows(&xml);
        let mut reports = vec![];
        for (name, changed) in cases(&xml) {
            eprintln!("{name}: fresh provenance, independent repeat and fully unhooked control");
            let sources = source_rows(&changed);
            let edits = changed_fields(&original_rows, &sources);
            assert_eq!(
                rows(&edits).len(),
                if name == "original-05" {
                    0
                } else if name == "physical-sniper-focused" {
                    1
                } else if name.starts_with("physical-sniper") || name.ends_with("-focused") {
                    2
                } else {
                    3
                }
            );
            fs::write(out.join(format!("{name}.xml")), &changed).unwrap();
            let first = observe(&root, &changed, mode == "on", true);
            // Preserve the first raw result before checks or repeat comparisons.
            fs::write(
                out.join(format!("{name}-jit-{}.json", mode.to_string_lossy())),
                serde_json::to_vec(&first).unwrap(),
            )
            .unwrap();
            check_case(name, &first, &sources);
            let repeat = observe(&root, &changed, mode == "on", true);
            assert!(
                first == repeat,
                "{name}: independent same-protocol replay differs"
            );
            let unhooked = observe(&root, &changed, mode == "on", false);
            for stage in STAGES {
                for field in [
                    "state",
                    "methods",
                    "source_methods_preserved",
                    "outputs_preserved",
                ] {
                    assert!(
                        first["states"][stage][field] == unhooked["states"][stage][field],
                        "{name}/{stage}/{field}: independent unhooked control differs"
                    );
                }
            }
            reports.push(json!({"name":name,"xml_sha256":hash(changed.as_bytes()),"changed_fields":edits,
                "independent_fresh_repeat_equal":true,"independent_unhooked_equal":true,"observation":first}));
        }
        assert_eq!(reports.len(), 12);
        let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
            "original_xml_sha256":ORIGINAL,"stages":STAGES,"fresh_runtimes_per_case":3,
            "no_retry_or_settling":true,"native_participation_authority":false,"native_build_parity":false,"source_bug_exception":false,
            "observer_sha256":hash(OBSERVER.as_bytes()),"loader_sha256":hash(LOADER.as_bytes()),"authentication_sha256":hash(AUTH.as_bytes()),"membership_sha256":hash(MEMBERSHIP.as_bytes()),
            "files":(["src/HeadlessWrapper.lua","src/Modules/Build.lua","src/Modules/Calcs.lua","src/Modules/CalcSetup.lua",
                "src/Modules/CalcActiveSkill.lua","src/Modules/CalcPerform.lua","src/Classes/SkillsTab.lua","src/Classes/CalcsTab.lua",
                "src/Classes/ItemsTab.lua","src/Classes/PassiveSpec.lua","src/Classes/CompareTab.lua","src/Modules/Common.lua"]
                .map(|path| json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),"cases":reports});
        let bytes = serde_json::to_vec(&report).unwrap();
        assert!(
            bytes.len() <= 32 * 1024 * 1024,
            "bounded participation report"
        );
        fs::write(
            out.join(format!("source-jit-{}.json", mode.to_string_lossy())),
            bytes,
        )
        .unwrap();
        assert_eq!(fs::read_to_string(path).unwrap(), xml);
        return;
    }
    assert!(
        !out.exists(),
        "use a fresh immutable source output directory"
    );
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env(CHILD, mode)
            .env(OUTPUT, &out)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "source child failed: {}", path.display());
                break;
            }
            if started.elapsed() > Duration::from_secs(600) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline: {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert!(
        fs::read(out.join("source-jit-off.json")).unwrap()
            == fs::read(out.join("source-jit-on.json")).unwrap(),
        "JIT modes produce byte-identical observations"
    );
}
