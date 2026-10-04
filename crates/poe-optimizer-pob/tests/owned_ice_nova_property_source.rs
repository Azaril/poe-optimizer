//! Complete original property preparation, including shared manual sources and retained positions.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/json_evidence.rs"]
mod json_evidence;
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
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "complete_property_preparation_observes_original_sources_and_positions";
const CHILD: &str = "POE_ICE_PROPERTY_SOURCE_CHILD";
const OBSERVER: &str = include_str!("support/ice_nova_property_source.lua");
const LIFECYCLE: &str = include_str!("support/djinn_provider_source.lua");
const STAGES: [&str; 3] = ["fresh", "rebuilt_once", "rebuilt_twice"];
const ICE: &str = "IceNovaPlayer";
const EXODUS: &str = "SupportUhtredExodusPlayer";
const FILES: [&str; 28] = [
    "src/HeadlessWrapper.lua",
    "src/Modules/Common.lua",
    "src/Modules/Build.lua",
    "src/Classes/CalcsTab.lua",
    "src/Classes/CompareTab.lua",
    "src/Classes/SkillsTab.lua",
    "src/Classes/PassiveSpec.lua",
    "src/Classes/Item.lua",
    "src/Classes/ItemsTab.lua",
    "src/Classes/ModStore.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModList.lua",
    "src/Modules/Data.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcActiveSkill.lua",
    "src/Modules/CalcTools.lua",
    "src/Modules/Calcs.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/ItemTools.lua",
    "src/Modules/ModParser.lua",
    "src/Data/Gems.lua",
    "src/Data/Skills/act_int.lua",
    "src/Data/Skills/act_dex.lua",
    "src/Data/Skills/sup_str.lua",
    "src/Data/Skills/sup_int.lua",
    "src/Data/Skills/sup_dex.lua",
    "src/Data/Skills/other.lua",
    "src/Data/Skills/minion.lua",
];

#[test]
#[ignore = "requires complete pinned PoB runtime; source property evidence only"]
fn complete_property_preparation_observes_original_sources_and_positions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-ice-nova-property-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        child(&root, &out, mode == "on");
        return;
    }
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env(CHILD, mode)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = process.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "source failed: {}\n{}",
                    path.display(),
                    tail(&path)
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(300) {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!("source deadline: {}\n{}", path.display(), tail(&path));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    json_evidence::assert_files_equal(
        &out.join("source-jit-off.json"),
        &out.join("source-jit-on.json"),
        "property-source JIT parity",
    );
}

fn child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let dir = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json = serde_json::from_slice(&fs::read(dir.join("index.json")).unwrap()).unwrap();
    let originals: Vec<_> = (1..=5)
        .map(|i| fs::read(dir.join(format!("build-{i:02}.xml"))).unwrap())
        .collect();
    let mut cases = Vec::new();
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(
            digest(bytes),
            index["builds"][i]["xml_sha256"].as_str().unwrap()
        );
        cases.push(observe(
            root,
            &format!("original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
        ));
    }
    let original = std::str::from_utf8(&originals[4]).unwrap();
    for preset in ["5", "6", "1"] {
        let doc = roxmltree::Document::parse(original).unwrap();
        let skills = doc
            .descendants()
            .find(|n| n.has_tag_name("Skills"))
            .unwrap();
        let range = skills
            .attribute_node("activeSkillSet")
            .unwrap()
            .range_value();
        let mut changed = original.to_owned();
        changed.replace_range(range, preset);
        cases.push(observe(
            root,
            &format!("ice-preset-{preset}"),
            &changed,
            enabled,
        ));
    }
    for (name, lines) in [
        ("amulet-spell-level", "+2 to Level of all Spell Skills"),
        (
            "amulet-spell-level-bonus",
            "+2 to Level of all Spell Skills\n50% increased bonuses gained from equipped rings and amulets",
        ),
        (
            "amulet-fire-level-negative",
            "+2 to Level of all Fire Skills",
        ),
        (
            "actor-supported-ice-level",
            "+2 to Level of Ice Nova Skills",
        ),
    ] {
        let changed = item_control(original, lines);
        let changed = if name == "actor-supported-ice-level" {
            without_ice_supports(&changed)
        } else {
            changed
        };
        cases.push(observe(root, name, &changed, enabled));
    }
    let alone = exodus_control(original, false, None);
    cases.push(observe(root, "exodus-alone", &alone, enabled));
    cases.push(observe(
        root,
        "exodus-other-support",
        &exodus_control(original, true, None),
        enabled,
    ));
    cases.push(observe(
        root,
        "distinct-ice-copies",
        &copies_control(original),
        enabled,
    ));
    for (name, level, quality) in [
        ("duplicate-exodus-tie", "1", "0"),
        ("duplicate-exodus-quality", "1", "15"),
        ("duplicate-exodus-raw-level", "2", "0"),
    ] {
        cases.push(observe(
            root,
            name,
            &exodus_control(original, false, Some((level, quality))),
            enabled,
        ));
    }
    cases.push(observe(
        root,
        "twister-retained-positions",
        &twister_control(std::str::from_utf8(&originals[1]).unwrap()),
        enabled,
    ));
    cases.push(observe(root, "repeat-original-05", original, enabled));
    assert_eq!(cases.len(), 20);
    let report = json!({
        "source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4",
        "manifest_sha256":pinned::manifest_sha256(),
        "files":FILES.map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})),
        "original_sources":originals.iter().enumerate().map(|(i,b)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(b)})).collect::<Vec<_>>(),
        "business_wrappers":false,"source_tables_mutated":false,"native_build_parity":false,
        "native_inventory_authority":false,"final_input_authority":false,
        "canonical_parity_lifecycle_selected":false,"lifecycle_stages":STAGES,"cases":cases
    });
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(
        bytes.len() <= 64 * 1024 * 1024,
        "property evidence is {} bytes",
        bytes.len()
    );
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        bytes,
    )
    .unwrap();
    check(&report);
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(
            &fs::read(dir.join(format!("build-{:02}.xml", i + 1))).unwrap(),
            bytes
        );
    }
}

fn install_hook(lua: &Lua) -> Result<Function, RuntimeError> {
    lua.globals().set("icePropertyPhase", "before")?;
    Ok(lua
        .load(OBSERVER)
        .set_name("@original-property-call-observer-install")
        .eval()?)
}

fn observe(root: &Path, name: &str, xml: &str, enabled: bool) -> Json {
    eprintln!(
        "Property source case {name}, JIT {}",
        if enabled { "on" } else { "off" }
    );
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("icePropertyXml", xml)?;
        lua.globals().set("icePropertyJit", enabled)?;
        lua.load("if icePropertyJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let install = |lua: &Lua| -> Result<Function, RuntimeError> {
        lua.globals().set("djinnPhase", "before")?;
        let auth: Function = lua
            .load(LIFECYCLE)
            .set_name("@property-original-lifecycle-authentication")
            .eval()?;
        let observer = install_hook(lua)?;
        let cleanup: Function = lua.load("return function(observer,auth) return function() local ok,err=pcall(observer);auth();if not ok then error(err,0)end end end").eval()?;
        Ok(cleanup.call((observer, auth))?)
    };
    let observer = |lua: &Lua| -> Result<Json, RuntimeError> {
        let fresh = stage(lua)?;
        rebuild(lua)?;
        let rebuilt_once = stage(lua)?;
        rebuild(lua)?;
        let rebuilt_twice = stage(lua)?;
        Ok(json!({"fresh":fresh,"rebuilt_once":rebuilt_once,"rebuilt_twice":rebuilt_twice}))
    };
    let scratch = tempfile::tempdir().unwrap();
    let observed = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        xml,
        None,
        false,
        Some(&before),
        Some(&install),
        Some(&observer),
    )
    .unwrap_or_else(|e| panic!("{name}: complete source failed: {e}"));
    assert_eq!(observed["configuration_method_wrappers"], false);
    assert_eq!(observed["original_build_output_available"], true);
    let states = &observed["additional_observation"];
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([107; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    for stage in STAGES {
        for row in rows(&states[stage]["saved"]) {
            let source = evidence
                .rows()
                .iter()
                .find(|source| {
                    Some(u64::from(source.occurrence().id().ordinal()))
                        == row["source_ordinal"].as_u64()
                })
                .unwrap();
            assert_eq!(source.occurrence().name(), "Gem");
            assert_eq!(
                source.attributes().len(),
                row["attributes"].as_object().unwrap().len()
            );
            for attribute in source.attributes() {
                assert_eq!(
                    row["attributes"][&attribute.origin().name],
                    attribute.decoded().unwrap()
                );
            }
        }
    }
    json!({"name":name,"xml_sha256":digest(xml.as_bytes()),"source_identity":evidence.identity(),"states":states})
}

fn snapshot(lua: &Lua) -> Result<Json, RuntimeError> {
    lua.globals().set("icePropertyPhase", "observe")?;
    let value: Value = lua
        .load(OBSERVER)
        .set_name("@original-property-observation")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn stage(lua: &Lua) -> Result<Json, RuntimeError> {
    let before = snapshot(lua)?;
    let after = snapshot(lua)?;
    assert_eq!(
        json_evidence::first_difference(&before, &after, "original-output-and-source-preservation"),
        None
    );
    Ok(before)
}
fn rebuild(lua: &Lua) -> Result<(), RuntimeError> {
    let cleanup = install_hook(lua)?;
    let result = lua
        .load(
            r#"
local function original(f,path,line)
 local i=debug.getinfo(f,"S");local p=i.source:gsub("\\","/")
 assert(i.what=="Lua"and p:sub(-#path)==path and i.linedefined==line);return f
end
local callback=original(runCallback,"HeadlessWrapper.lua",17)
local frame=original(build.OnFrame,"Modules/Build.lua",1285)
local output=original(build.calcsTab.BuildOutput,"Classes/CalcsTab.lua",486)
assert(output==djinnOriginals.refs.calcs_tab_output and build.buildFlag==false)
local revision=build.outputRevision;local main,calcs=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
build.buildFlag=true;callback("OnFrame")
assert(build.buildFlag==false and build.outputRevision==revision+1)
assert(build.calcsTab.mainEnv~=main and build.calcsTab.calcsEnv~=calcs)
assert(runCallback==callback and build.OnFrame==frame and build.calcsTab.BuildOutput==output)
"#,
        )
        .set_name("@original-property-frame-rebuild")
        .exec();
    let removed = cleanup.call::<()>(());
    result?;
    removed?;
    Ok(())
}

fn selected_gem<'a, 'input>(
    doc: &'a roxmltree::Document<'input>,
    effect: &str,
) -> roxmltree::Node<'a, 'input> {
    let skills = doc
        .descendants()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let selected = skills.attribute("activeSkillSet").unwrap();
    skills
        .descendants()
        .find(|n| {
            n.has_tag_name("Gem")
                && n.attribute("skillId") == Some(effect)
                && n.ancestors()
                    .any(|p| p.has_tag_name("SkillSet") && p.attribute("id") == Some(selected))
        })
        .unwrap()
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}
fn gem_xml(gem: roxmltree::Node<'_, '_>, changes: &[(&str, &str)], children: &str) -> String {
    let mut result = String::from("<Gem");
    for attribute in gem.attributes() {
        let value = changes
            .iter()
            .find(|(key, _)| *key == attribute.name())
            .map_or(attribute.value(), |(_, v)| *v);
        result.push_str(&format!(" {}=\"{}\"", attribute.name(), escape(value)));
    }
    if children.is_empty() {
        result.push_str("/>");
    } else {
        result.push('>');
        result.push_str(children);
        result.push_str("</Gem>");
    }
    result
}
fn item_control(xml: &str, lines: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let items = doc.descendants().find(|n| n.has_tag_name("Items")).unwrap();
    let active = items.attribute("activeItemSet").unwrap();
    let set = items
        .children()
        .find(|n| n.has_tag_name("ItemSet") && n.attribute("id") == Some(active))
        .unwrap();
    let item_id = set
        .children()
        .find(|n| n.has_tag_name("Slot") && n.attribute("name") == Some("Amulet"))
        .unwrap()
        .attribute("itemId")
        .unwrap();
    let item = items
        .children()
        .find(|n| n.has_tag_name("Item") && n.attribute("id") == Some(item_id))
        .unwrap();
    // ItemsTab:Load parses each text child independently. Preserve its single
    // complete raw item before the legacy ModRange children.
    let raw = item.children().find(|node| node.is_text()).unwrap();
    assert!(raw.text().unwrap().contains("Rarity: RARE"));
    let replacement = format!("{}\n{}\n", xml[raw.range()].trim_end(), escape(lines));
    let mut changed = xml.to_owned();
    changed.replace_range(raw.range(), &replacement);
    roxmltree::Document::parse(&changed).unwrap();
    changed
}

fn without_ice_supports(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let ice = selected_gem(&doc, ICE);
    let edits: Vec<_> = ice
        .parent()
        .unwrap()
        .children()
        .filter(|node| node.has_tag_name("Gem") && *node != ice)
        .map(|node| (node.range(), gem_xml(node, &[("enabled", "false")], "")))
        .collect();
    assert_eq!(edits.len(), 3);
    edited(xml, edits)
}

// These test-only tuples are checked against the actual constructed source catalog.
fn support_tuple(effect: &str) -> (&'static str, &'static str, &'static str) {
    match effect {
        EXODUS => (
            "Metadata/Items/Gem/SupportGemUhtredsExodus",
            "UhtredExodusSupport",
            "Uhtred's Exodus",
        ),
        "SupportMultishotPlayer" => (
            "Metadata/Items/Gems/SupportGemScattershot",
            "MultishotSupport",
            "Multishot I",
        ),
        "SupportUnleashPlayer" => (
            "Metadata/Items/Gems/SupportGemUnleash",
            "UnleashSupport",
            "Unleash",
        ),
        "SupportSalvoPlayer" => (
            "Metadata/Items/Gem/SupportGemSalvo",
            "SalvoSupport",
            "Salvo",
        ),
        _ => panic!("unreviewed support control"),
    }
}
fn support_xml(
    template: roxmltree::Node<'_, '_>,
    effect: &str,
    level: &str,
    quality: &str,
) -> String {
    let (gem, variant, name) = support_tuple(effect);
    gem_xml(
        template,
        &[
            ("gemId", gem),
            ("variantId", variant),
            ("skillId", effect),
            ("nameSpec", name),
            ("level", level),
            ("quality", quality),
            ("enabled", "true"),
        ],
        "",
    )
}
fn exodus_control(xml: &str, other: bool, duplicate: Option<(&str, &str)>) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let gem = selected_gem(&doc, ICE);
    let group = gem.parent().unwrap();
    let supports: Vec<_> = group
        .children()
        .filter(|n| n.has_tag_name("Gem") && *n != gem)
        .collect();
    assert_eq!(supports.len(), 3);
    let mut edits = Vec::new();
    let mut first = support_xml(supports[0], EXODUS, "1", "0");
    if let Some((level, quality)) = duplicate {
        first.push_str(&support_xml(supports[0], EXODUS, level, quality));
    }
    edits.push((supports[0].range(), first));
    for (i, support) in supports.iter().enumerate().skip(1) {
        edits.push((
            support.range(),
            gem_xml(
                *support,
                &[("enabled", if other && i == 1 { "true" } else { "false" })],
                "",
            ),
        ));
    }
    edited(xml, edits)
}
fn twister_control(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let gem = selected_gem(&doc, "TwisterPlayer");
    let supports: Vec<_> = gem
        .parent()
        .unwrap()
        .children()
        .filter(|n| n.has_tag_name("Gem") && *n != gem)
        .collect();
    assert!(!supports.is_empty());
    let replacement = [
        "SupportMultishotPlayer",
        "SupportUnleashPlayer",
        "SupportSalvoPlayer",
    ]
    .map(|effect| support_xml(supports[0], effect, "1", "0"))
    .join("");
    let edits = supports
        .iter()
        .enumerate()
        .map(|(i, s)| {
            (
                s.range(),
                if i == 0 {
                    replacement.clone()
                } else {
                    String::new()
                },
            )
        })
        .collect();
    edited(xml, edits)
}
fn copies_control(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let gem = selected_gem(&doc, ICE);
    assert!(gem.children().all(|n| !n.is_element()));
    let original = &xml[gem.range()];
    let copy = gem_xml(
        gem,
        &[("level", "12"), ("quality", "12.5")],
        "<StatSetIndex grantedEffect=\"IceNovaPlayer\" index=\"2\"/><StatSetCalcsIndex grantedEffect=\"IceNovaPlayer\" index=\"1\"/>",
    );
    edited(xml, vec![(gem.range(), format!("{original}{copy}"))])
}
fn edited(xml: &str, mut edits: Vec<(std::ops::Range<usize>, String)>) -> String {
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut result = xml.to_owned();
    for (range, text) in edits {
        result.replace_range(range, &text);
    }
    assert_ne!(result, xml);
    roxmltree::Document::parse(&result).unwrap();
    result
}

fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|o| o.is_empty()));
        &[]
    }
}
fn case<'a>(report: &'a Json, name: &str) -> &'a Json {
    rows(&report["cases"])
        .iter()
        .find(|case| case["name"] == name)
        .unwrap()
}
fn ice_contexts(state: &Json) -> Vec<&Json> {
    rows(&state["contexts"])
        .iter()
        .filter(|c| c["effect"] == ICE)
        .collect()
}
fn count_value(cache: &Json) -> f64 {
    let records: Vec<_> = rows(&cache["query"]["chain"])[0]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["name"] == "Multiplier:SupportCount")
        .collect();
    assert_eq!(records.len(), 1);
    records[0]["mod"]["value"].as_f64().unwrap()
}
fn property_matches(row: &Json, keyword: &str, amount: f64) -> bool {
    row["value"]["keyword"] == keyword
        && row["value"]["key"] == "level"
        && row["value"]["value"].as_f64() == Some(amount)
}
fn partition_has(external: &Json, disposition: &str, keyword: &str, amount: f64) -> bool {
    rows(&external[disposition]).iter().any(|index| {
        property_matches(
            &rows(&external["candidates"])[index.as_u64().unwrap() as usize - 1],
            keyword,
            amount,
        )
    })
}
fn actor_has_property(chain: &Json, amount: f64) -> bool {
    rows(chain)
        .iter()
        .flat_map(|store| rows(&store["rows"]))
        .any(|row| {
            row["name"] == "SupportedGemProperty"
                && row["mod"]["value"]["value"].as_f64() == Some(amount)
        })
}
fn check(report: &Json) {
    assert_eq!(rows(&report["cases"]).len(), 20);
    assert_eq!(
        json_evidence::first_difference(
            &case(report, "original-05")["states"],
            &case(report, "repeat-original-05")["states"],
            "repeat"
        ),
        None
    );
    for case in rows(&report["cases"]) {
        for stage in STAGES {
            let state = &case["states"][stage];
            let name = case["name"].as_str().unwrap();
            let expected_ice_per_mode = match name {
                "original-01"
                | "original-02"
                | "original-03"
                | "original-04"
                | "twister-retained-positions" => 0,
                "distinct-ice-copies" => 2,
                _ => 1,
            };
            let ice = ice_contexts(state);
            assert_eq!(ice.len(), expected_ice_per_mode * 2, "{name} {stage}");
            for mode in ["MAIN", "CALCS"] {
                assert_eq!(
                    ice.iter().filter(|context| context["mode"] == mode).count(),
                    expected_ice_per_mode,
                    "{name} {stage} {mode}",
                );
            }
            assert_eq!(rows(&state["catalog"]).len(), 4);
            for row in rows(&state["catalog"]) {
                assert_eq!(row["support"], true);
                let (game_id, variant, name) = support_tuple(row["effect"].as_str().unwrap());
                assert_eq!(row["game_id"], game_id);
                assert_eq!(row["variant"], variant);
                assert_eq!(row["name"], name);
            }
            for field in [
                "original_methods_preserved",
                "exact_loaded_objects_preserved",
                "hook_removed",
                "jit_mode_preserved",
            ] {
                assert_eq!(state[field], true, "{} {stage} {field}", case["name"]);
            }
            for context in rows(&state["contexts"]) {
                assert_eq!(context["actor_is_player"], true);
                assert_eq!(context["distinct_source_cache"], true);
                assert_eq!(rows(&context["constructor"]).len(), 1);
                assert_eq!(rows(&context["external"]).len(), 1);
                let external = &rows(&context["external"])[0];
                let candidates = rows(&external["candidates"]);
                let mut dispositions = BTreeSet::new();
                for field in ["matched", "rejected"] {
                    for index in rows(&external[field]) {
                        let i = index.as_u64().unwrap() as usize;
                        assert!(i >= 1 && i <= candidates.len() && dispositions.insert(i));
                    }
                }
                assert_eq!(dispositions.len(), candidates.len());
                for cache in rows(&context["cache"]) {
                    assert_eq!(cache["cache_identity_preserved"], true);
                    assert_eq!(cache["query_parent_is_actor"], true);
                    assert_eq!(
                        cache["query"]["cfg"]["skill_gem"],
                        context["source_catalog"]
                    );
                    assert_eq!(
                        cache["before"], cache["after"],
                        "the cache helper only collects properties"
                    );
                    let _ = count_value(cache);
                }
            }
            for copy in rows(&state["copies"]) {
                assert_eq!(
                    copy["before"], copy["input_after"],
                    "ScaleAddMod must preserve its original input record"
                );
                assert_eq!(rows(&copy["insertions"]).len(), 1);
                assert_eq!(rows(&copy["insertions"])[0]["exact_destination"], true);
                assert_eq!(rows(&copy["insertions"])[0]["inserted"], true);
            }
        }
    }
    for stage in STAGES {
        let original_amulets = &case(report, "original-05")["states"][stage]["equipped_amulets"];
        for name in [
            "amulet-spell-level",
            "amulet-spell-level-bonus",
            "amulet-fire-level-negative",
            "actor-supported-ice-level",
        ] {
            let state = &case(report, name)["states"][stage];
            for mode in ["MAIN", "CALCS"] {
                let amulet = &state["equipped_amulets"][mode];
                for field in ["id", "name", "title", "base", "type", "exact_registered"] {
                    assert_eq!(
                        amulet[field], original_amulets[mode][field],
                        "{name} {mode} {field}"
                    );
                }
                assert_eq!(amulet["present"], true);
                assert_eq!(amulet["exact_registered"], true);
                assert_eq!(amulet["base"], "Solar Amulet");
                assert_eq!(amulet["type"], "Amulet");
                for line in rows(&original_amulets[mode]["raw_lines"]) {
                    assert!(
                        rows(&amulet["raw_lines"]).contains(line),
                        "{name} lost an original raw item line"
                    );
                }
                assert!(
                    rows(&amulet["raw_lines"])
                        .iter()
                        .any(|line| line == "+1 to Level of all Minion Skills")
                );
            }
            for context in ice_contexts(state) {
                assert!(partition_has(
                    &rows(&context["external"])[0],
                    "rejected",
                    "minion",
                    1.0
                ));
            }
        }
        for name in [
            "exodus-alone",
            "exodus-other-support",
            "duplicate-exodus-tie",
            "duplicate-exodus-quality",
            "duplicate-exodus-raw-level",
            "twister-retained-positions",
        ] {
            let state = &case(report, name)["states"][stage];
            let target = rows(&state["contexts"])
                .iter()
                .find(|row| {
                    row["effect"]
                        == if name == "twister-retained-positions" {
                            "TwisterPlayer"
                        } else {
                            ICE
                        }
                })
                .unwrap();
            let controls: Vec<_> = rows(&state["loaded_controls"])
                .iter()
                .filter(|row| {
                    row["selected"] == true
                        && row["group_source_ordinal"] == target["source"]["group_source_ordinal"]
                })
                .collect();
            let expected = if name == "twister-retained-positions" {
                3
            } else if name.starts_with("duplicate-") {
                2
            } else {
                1
            };
            assert_eq!(
                controls.len(),
                expected,
                "{name} must load every inserted source tuple"
            );
            for row in &controls {
                for field in ["exact_saved_object", "exact_catalog", "exact_game_id"] {
                    assert_eq!(row[field], true, "{name} {field}");
                }
                assert_eq!(row["attributes"]["enabled"], "true");
                assert_eq!(row["loaded_effect"], row["attributes"]["skillId"]);
                assert_eq!(row["loaded_catalog"], row["expected_catalog"]);
                assert_eq!(row["attributes"]["gemId"], row["expected_game_id"]);
                assert_eq!(row["attributes"]["variantId"], row["expected_variant"]);
            }
            if name == "twister-retained-positions" {
                assert_eq!(
                    controls
                        .iter()
                        .map(|row| row["loaded_effect"].as_str().unwrap())
                        .collect::<Vec<_>>(),
                    vec![
                        "SupportMultishotPlayer",
                        "SupportUnleashPlayer",
                        "SupportSalvoPlayer"
                    ]
                );
            }
        }
        for name in [
            "original-05",
            "ice-preset-5",
            "ice-preset-6",
            "ice-preset-1",
        ] {
            let state = &case(report, name)["states"][stage];
            assert_eq!(
                ice_contexts(state).len(),
                2,
                "each saved Ice preset is actually evaluated"
            );
        }
        for (name, expected_pre, expected_final) in [
            ("original-05", 17.0, 17.0),
            ("amulet-spell-level", 19.0, 19.0),
            ("amulet-spell-level-bonus", 20.0, 20.0),
            ("amulet-fire-level-negative", 17.0, 17.0),
            ("actor-supported-ice-level", 17.0, 21.0),
            ("exodus-alone", 17.0, 20.0),
            ("exodus-other-support", 17.0, 17.0),
        ] {
            for context in ice_contexts(&case(report, name)["states"][stage]) {
                assert_eq!(context["raw"]["level"].as_f64(), Some(17.0), "{name}");
                assert_eq!(
                    rows(&context["external"])[0]["after"]["level"].as_f64(),
                    Some(expected_pre),
                    "{name}"
                );
                assert_eq!(
                    context["final"]["level"].as_f64(),
                    Some(expected_final),
                    "{name}"
                );
                assert_eq!(context["cache_present"], true);
                assert_eq!(context["cache_is_observed"], true);
                assert!(!rows(&context["validation"]).is_empty());
                let last = rows(&context["validation"]).last().unwrap();
                assert_eq!(last["after"]["level"], context["final"]["level"]);
            }
        }
        for (name, count) in [("exodus-alone", 1.0), ("exodus-other-support", 2.0)] {
            for context in ice_contexts(&case(report, name)["states"][stage]) {
                let cache = rows(&context["cache"]).last().unwrap();
                assert_eq!(count_value(cache), count);
                let properties = rows(&cache["properties"]);
                assert_eq!(
                    properties
                        .iter()
                        .any(|p| p["value"]["value"].as_f64() == Some(3.0)),
                    count == 1.0
                );
            }
        }
        for (name, disposition, keyword, amount) in [
            ("amulet-spell-level", "matched", "spell", 2.0),
            ("amulet-spell-level-bonus", "matched", "spell", 1.0),
            ("amulet-fire-level-negative", "rejected", "fire", 2.0),
        ] {
            for context in ice_contexts(&case(report, name)["states"][stage]) {
                assert!(
                    partition_has(&rows(&context["external"])[0], disposition, keyword, amount),
                    "{name} must exercise the original matching branch"
                );
            }
        }
        let bonus = &case(report, "amulet-spell-level-bonus")["states"][stage];
        assert!(
            rows(&bonus["copies"]).iter().any(|copy| {
                copy["scale"].as_f64() == Some(0.5)
                    && copy["before"]["source"]
                        .as_str()
                        .is_some_and(|s| s.contains("Amulet Bonus Effect"))
                    && copy["before"]["value"]["keyword"] == "spell"
                    && copy["before"]["value"]["value"].as_f64() == Some(2.0)
                    && rows(&copy["insertions"])[0]["mod"]["value"]["value"].as_f64() == Some(1.0)
                    && rows(&copy["insertions"])[0]["same_as_input"] == false
            }),
            "the genuine parsed Amulet bonus must insert its observed scaled copy"
        );
        let actor_property = &case(report, "actor-supported-ice-level")["states"][stage];
        assert!(
            rows(&actor_property["copies"]).iter().any(|copy| {
                copy["scale"].as_f64() == Some(0.0)
                    && copy["before"]["name"] == "SupportedGemProperty"
                    && copy["before"]["value"]["value"].as_f64() == Some(2.0)
                    && copy["before"]["value"]["keyOfScaledMod"].is_null()
                    && copy["before"]["source"]
                        .as_str()
                        .is_some_and(|source| source.contains("Amulet Bonus Effect"))
                    && rows(&copy["insertions"])[0]["mod"]["value"]["value"].as_f64() == Some(2.0)
                    && rows(&copy["insertions"])[0]["same_as_input"] == false
            }),
            "the original zero-scale operation retains this unkeyed nested property"
        );
        for context in ice_contexts(actor_property) {
            let cache = rows(&context["cache"]).last().unwrap();
            assert!(rows(&cache["supports"]).is_empty());
            assert!(rows(&cache["merges"]).is_empty());
            assert_eq!(count_value(cache), 0.0);
            assert!(actor_has_property(&cache["actor_chain"], 2.0));
            assert_eq!(
                rows(&cache["properties"])
                    .iter()
                    .filter(|property| property_matches(property, "grants_active_skill", 2.0))
                    .count(),
                2
            );
            assert!(rows(&cache["properties"]).iter().any(|p| {
                property_matches(p, "grants_active_skill", 2.0)
                    && p["mod"]["source"]
                        .as_str()
                        .is_some_and(|s| s.starts_with("Item:"))
            }));
        }
        let copies = ice_contexts(&case(report, "distinct-ice-copies")["states"][stage]);
        assert_eq!(copies.len(), 4);
        for mode in ["MAIN", "CALCS"] {
            let per_mode: Vec<_> = copies.iter().filter(|c| c["mode"] == mode).collect();
            assert_eq!(per_mode.len(), 2);
            assert_ne!(
                per_mode[0]["source"]["source_ordinal"],
                per_mode[1]["source"]["source_ordinal"]
            );
            assert_ne!(per_mode[0]["cache_key"], per_mode[1]["cache_key"]);
            let second = per_mode
                .iter()
                .find(|c| c["raw"]["quality"].as_f64() == Some(12.5))
                .unwrap();
            assert_eq!(second["raw"]["level"].as_f64(), Some(12.0));
            assert_eq!(second["final"]["level"].as_f64(), Some(12.0));
            assert_eq!(second["final"]["quality"].as_f64(), Some(12.5));
            assert_eq!(second["stat_set"], if mode == "MAIN" { 2 } else { 1 });
            let first = per_mode
                .iter()
                .find(|c| c["raw"]["quality"].as_f64() == Some(0.0))
                .unwrap();
            assert_eq!(first["final"]["level"].as_f64(), Some(17.0));
            assert_eq!(first["final"]["quality"].as_f64(), Some(0.0));
        }
        for name in [
            "duplicate-exodus-tie",
            "duplicate-exodus-quality",
            "duplicate-exodus-raw-level",
        ] {
            let state = &case(report, name)["states"][stage];
            assert!(!rows(&state["selections"]).is_empty());
            for context in ice_contexts(state) {
                assert_eq!(context["final"]["level"].as_f64(), Some(20.0));
                let cache = rows(&context["cache"]).last().unwrap();
                assert_eq!(count_value(cache), 1.0);
                assert_eq!(rows(&cache["supports"]).len(), 1);
                let quality = if name == "duplicate-exodus-quality" {
                    15.0
                } else {
                    0.0
                };
                assert_eq!(
                    rows(&cache["supports"])[0]["prepared"]["quality"].as_f64(),
                    Some(quality)
                );
                assert_eq!(
                    rows(&cache["supports"])[0]["prepared"]["level"].as_f64(),
                    Some(1.0)
                );
                let candidates: BTreeSet<_> = rows(&state["selections"])
                    .iter()
                    .filter(|r| r["effect"] == EXODUS && r["mode"] == context["mode"])
                    .map(|r| r["source"]["source_ordinal"].as_u64().unwrap())
                    .collect();
                assert_eq!(candidates.len(), 2);
                let expected = if name == "duplicate-exodus-quality" {
                    candidates.last()
                } else {
                    candidates.first()
                }
                .unwrap();
                assert_eq!(
                    rows(&cache["supports"])[0]["source"]["source_ordinal"].as_u64(),
                    Some(*expected)
                );
            }
        }
        let twister = &case(report, "twister-retained-positions")["states"][stage];
        let targets: Vec<_> = rows(&twister["contexts"])
            .iter()
            .filter(|c| c["effect"] == "TwisterPlayer")
            .collect();
        assert_eq!(targets.len(), 2);
        for target in targets {
            let cache = rows(&target["cache"]).last().unwrap();
            let supports = rows(&cache["supports"]);
            assert_eq!(supports.len(), 2);
            assert!(supports.iter().all(|s| s["effect"] == "SupportSalvoPlayer"));
            assert_eq!(supports[0]["source"], supports[1]["source"]);
            assert_eq!(supports[1]["same_instance_as"], 1);
            assert_eq!(rows(&cache["merges"]).len(), 2);
            assert_eq!(count_value(cache), 2.0);
        }
        let original = &case(report, "original-05")["states"][stage];
        for mode in ["MAIN", "CALCS"] {
            for (summon, command) in [
                ("SummonSandDjinnPlayer", "CommandSandDjinnKnifeThrowPlayer"),
                ("SummonWaterDjinnPlayer", "CommandWaterDjinnBubblePlayer"),
            ] {
                let first = rows(&original["contexts"])
                    .iter()
                    .find(|c| c["mode"] == mode && c["effect"] == summon)
                    .unwrap();
                let second = rows(&original["contexts"])
                    .iter()
                    .find(|c| {
                        c["mode"] == mode
                            && c["effect"] == command
                            && c["source"] == first["source"]
                    })
                    .unwrap();
                assert_eq!(
                    first["source_classification"],
                    "manual_nonphysical_correspondence"
                );
                assert_eq!(first["cache_key"], second["cache_key"]);
                assert!(!first["cache_key"].is_null());
                let mut calls: Vec<_> = rows(&first["cache"])
                    .iter()
                    .chain(rows(&second["cache"]))
                    .collect();
                calls.sort_by_key(|c| c["event_index"].as_u64().unwrap());
                assert_eq!(calls.len(), 2);
                assert_eq!(calls[0]["cache_hit"], false);
                assert_eq!(calls[1]["cache_hit"], true);
                assert_eq!(calls[0]["query"], calls[1]["query"]);
                assert_eq!(calls[0]["properties"], calls[1]["properties"]);
            }
        }
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn tail(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap()
        .chars()
        .rev()
        .take(6000)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}
