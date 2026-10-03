//! Optional complete-source evidence for saved minion usage/action settings.
//! Observed actors and child abilities never stand in for unresolved Commands.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/json_evidence.rs"]
mod json_evidence;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_core::owned_content::digest_owned;
use poe_optimizer_data::skill_identities::{GemIdentity, SkillIdentityCatalog};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "complete_source_minion_occurrence_settings_keep_exact_actor_actions";
const CHILD: &str = "POE_MINION_OCCURRENCE_CHILD";
const ORIGINAL_HASH: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";
const CATALOG: &str = "data/owned/poe2/3887ae68/import/skill-identities.json";
const CATALOG_DIGEST: &str = "b22849f6afaef20b49a578c2ed88314e014b893a71b7919c7b83ca95c6faa7ea";
const BASE_OBSERVER: &str = include_str!("support/minion_physical_gem_inputs.lua");
const OBSERVER: &str = include_str!("support/minion_occurrence_inputs.lua");

#[test]
#[ignore = "optional complete pinned PoB runtime; writes authenticated minion occurrence evidence"]
fn complete_source_minion_occurrence_settings_keep_exact_actor_actions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-minion-occurrence-inputs-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let catalog_bytes = fs::read(root.join(CATALOG)).unwrap();
        let catalog =
            SkillIdentityCatalog::new(serde_json::from_slice(&catalog_bytes).unwrap()).unwrap();
        let reviewed = reviewed(&catalog);
        let path = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
        let xml = fs::read_to_string(&path).unwrap();
        assert_eq!(sha256(xml.as_bytes()), ORIGINAL_HASH);
        println!("original05");
        let original = run(&root, &xml, None, enabled, &reviewed);
        let mut controls = Vec::new();
        for gem in &reviewed {
            let observed = selected(&original["minions"], &gem.key);
            let summon = &rows(&observed["MAIN"])[0];
            let choices = rows(&summon["minion_choices"]);
            assert_eq!(
                choices.len(),
                1,
                "finite actor alternatives must be observed"
            );
            let actor = choices[0].as_str().unwrap();
            let children = rows(&summon["actor"]["children"]);
            assert!(children.len() > 1);
            let last = children.len().to_string();
            // Prefer a genuine alternate stat set. A singleton still gets an
            // explicit correctly scoped child map; no alternative is invented.
            let map_child = children
                .iter()
                .find(|c| rows(&c["stat_sets"]).len() > 1)
                .unwrap_or(&children[0]);
            let map_index = map_child["index"].as_u64().unwrap().to_string();
            let map_last = rows(&map_child["stat_sets"]).len().to_string();
            let maps_main = child_maps(&gem.primary_effect_id, &map_index, &map_last, "1");
            let maps_calcs = child_maps(&gem.primary_effect_id, &map_index, "1", &map_last);
            let focused = focus(&xml, &gem.primary_effect_id);
            for (name, attrs, group, children) in [
                ("focused", vec![], vec![], ""),
                ("count-three", vec![("count", Some("3"))], vec![], ""),
                (
                    "group-four",
                    vec![("count", Some("3"))],
                    vec![("groupCount", Some("4"))],
                    "",
                ),
                (
                    "group-zero",
                    vec![("count", Some("3"))],
                    vec![("groupCount", Some("0"))],
                    "",
                ),
                (
                    "global-one-false",
                    vec![
                        ("enableGlobal1", Some("false")),
                        ("enableGlobal2", Some("true")),
                    ],
                    vec![],
                    "",
                ),
                (
                    "global-two-false",
                    vec![
                        ("enableGlobal1", Some("true")),
                        ("enableGlobal2", Some("false")),
                    ],
                    vec![],
                    "",
                ),
                (
                    "names-valid",
                    vec![
                        ("skillMinion", Some(actor)),
                        ("skillMinionCalcs", Some(actor)),
                    ],
                    vec![],
                    "",
                ),
                (
                    "names-invalid",
                    vec![
                        ("skillMinion", Some("unknown-main-actor")),
                        ("skillMinionCalcs", Some("unknown-calcs-actor")),
                    ],
                    vec![],
                    "",
                ),
                (
                    "names-absent",
                    vec![("skillMinion", None), ("skillMinionCalcs", None)],
                    vec![],
                    "",
                ),
                (
                    "action-main-last",
                    vec![
                        ("skillMinionSkill", Some(last.as_str())),
                        ("skillMinionSkillCalcs", Some("1")),
                    ],
                    vec![],
                    "",
                ),
                (
                    "action-calcs-last",
                    vec![
                        ("skillMinionSkill", Some("1")),
                        ("skillMinionSkillCalcs", Some(last.as_str())),
                    ],
                    vec![],
                    "",
                ),
                (
                    "actions-absent",
                    vec![("skillMinionSkill", None), ("skillMinionSkillCalcs", None)],
                    vec![],
                    "",
                ),
                (
                    "actions-clamped",
                    vec![
                        ("skillMinionSkill", Some("0")),
                        ("skillMinionSkillCalcs", Some("999")),
                    ],
                    vec![],
                    "",
                ),
                (
                    "maps-main-last",
                    vec![
                        ("skillMinionSkill", Some(map_index.as_str())),
                        ("skillMinionSkillCalcs", Some(map_index.as_str())),
                    ],
                    vec![],
                    maps_main.as_str(),
                ),
                (
                    "maps-calcs-last",
                    vec![
                        ("skillMinionSkill", Some(map_index.as_str())),
                        ("skillMinionSkillCalcs", Some(map_index.as_str())),
                    ],
                    vec![],
                    maps_calcs.as_str(),
                ),
                (
                    "full-dps-one",
                    vec![("count", Some("1"))],
                    vec![("includeInFullDPS", Some("true"))],
                    "",
                ),
                (
                    "full-dps-three",
                    vec![("count", Some("3"))],
                    vec![("includeInFullDPS", Some("true"))],
                    "",
                ),
            ] {
                println!("{}: {name}", gem.key);
                let changed = mutate(&focused, &gem.primary_effect_id, &attrs, &group, children);
                controls.push(json!({
                    "physical_id":gem.key,"name":name,"xml_sha256":sha256(changed.as_bytes()),
                    "state":run(&root,&changed,None,enabled,&reviewed),
                }));
            }
        }
        println!("original05-repeat");
        let repeated_original = run(&root, &xml, None, enabled, &reviewed);
        let warm = mutate(
            &focus(&xml, &reviewed[0].primary_effect_id),
            &reviewed[0].primary_effect_id,
            &[
                ("count", Some("3")),
                ("skillMinionSkill", Some("2")),
                ("skillMinionSkillCalcs", Some("2")),
            ],
            &[],
            "",
        );
        println!("original05-warm-restoration");
        let warm_original = run(&root, &xml, Some(&warm), enabled, &reviewed);
        let result = json!({
            "manifest_sha256":pinned::manifest_sha256(),
            "files":([
                "src/Classes/SkillsTab.lua","src/Classes/CalcsTab.lua","src/Modules/Data.lua",
                "src/Modules/CalcTools.lua","src/Modules/CalcSetup.lua","src/Modules/CalcActiveSkill.lua",
                "src/Modules/CalcDefence.lua","src/Modules/CalcPerform.lua","src/Modules/Calcs.lua",
                "src/Data/Gems.lua","src/Data/Skills/act_int.lua","src/Data/Minions.lua","src/Data/Skills/minion.lua",
            ].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
            "catalog_sha256":sha256(&catalog_bytes),"catalog_digest":CATALOG_DIGEST,
            "reviewed_gems":reviewed,
            "original_xml_sha256":ORIGINAL_HASH,"warm_xml_sha256":sha256(warm.as_bytes()),
            "original":original,"controls":controls,"repeated_original":repeated_original,"warm_original":warm_original,
            "native_inventory_authority":false,"native_build_parity":false,"missing_commands_resolved":false,
        });
        fs::write(
            out.join(format!(
                "source-jit-{}.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        assert_eq!(fs::read_to_string(path).unwrap(), xml);
        check(&result);
        return;
    }
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env(CHILD, mode)
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
            if started.elapsed() > Duration::from_secs(300) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline: {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    json_evidence::assert_files_equal(
        &out.join("source-jit-off.json"),
        &out.join("source-jit-on.json"),
        "minion occurrence JIT evidence",
    );
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn reviewed(catalog: &SkillIdentityCatalog) -> Vec<GemIdentity> {
    assert_eq!(
        serde_json::to_value(
            digest_owned(
                "owned-skill-source-catalog-v1",
                catalog.data(),
                64 * 1024 * 1024
            )
            .unwrap()
        )
        .unwrap(),
        json!(CATALOG_DIGEST)
    );
    [
        "Metadata/Items/Gems/SkillGemSkeletalArsonist",
        "Metadata/Items/Gems/SkillGemSkeletalSniper",
        "Metadata/Items/Gems/SkillGemSkeletalFrostMage",
        "Metadata/Items/Gems/SkillGemSkeletalReaver",
    ]
    .map(|key| {
        let gem = catalog.gem_by_key(key).unwrap();
        assert_eq!(
            gem.effect_list.as_slice(),
            std::slice::from_ref(&gem.primary_effect_id)
        );
        assert!(gem.additional_effects.is_empty());
        assert_eq!(
            gem.declared_additional_effects,
            gem.constructed_additional_effects
        );
        assert_eq!(gem.declared_additional_effects.len(), 1);
        assert!(
            catalog
                .skill_by_id(&gem.declared_additional_effects[0].id)
                .is_none()
        );
        gem.clone()
    })
    .into()
}

fn run(
    root: &Path,
    xml: &str,
    warm: Option<&str>,
    enabled: bool,
    reviewed: &[GemIdentity],
) -> Json {
    let before = |lua: &Lua| {
        lua.globals().set("minionPhysicalJit", enabled)?;
        lua.globals().set("minionPhysicalXml", xml)?;
        lua.globals()
            .set("minionPhysicalReviewed", lua.to_value(reviewed)?)?;
        lua.globals()
            .set("minionPhysicalCases", lua.create_table()?)?;
        lua.load("if minionPhysicalJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
        let value: Value = lua
            .load(BASE_OBSERVER)
            .set_name("@minion-occurrence-base-observer")
            .eval()?;
        let baseline: Json = lua.from_value(value.clone())?;
        lua.globals().set("minionOccurrenceBase", value)?;
        let minions: Value = lua
            .load(OBSERVER)
            .set_name("@minion-occurrence-observer")
            .eval()?;
        let minions: Json = lua.from_value(minions)?;
        let after: Value = lua
            .load(BASE_OBSERVER)
            .set_name("@minion-occurrence-preservation")
            .eval()?;
        let after: Json = lua.from_value(after)?;
        assert_same(&baseline, &after, "minion observer preservation");
        Ok(json!({"occurrence":baseline,"minions":minions}))
    };
    let scratch = tempfile::tempdir().unwrap();
    let result = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        xml,
        warm,
        false,
        Some(&before),
        None,
        Some(&observe),
    )
    .unwrap();
    assert_eq!(result["configuration_method_wrappers"], false);
    assert_eq!(result["original_build_output_available"], true);
    let mut observed = result["additional_observation"].clone();
    observed["source_hash"] = result["source_hash"].clone();
    observed
}

fn target<'a>(doc: &'a roxmltree::Document<'a>, effect: &str) -> roxmltree::Node<'a, 'a> {
    let skills = doc
        .descendants()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let set = skills.attribute("activeSkillSet").unwrap();
    let targets: Vec<_> = skills
        .descendants()
        .filter(|n| {
            n.has_tag_name("Gem")
                && n.attribute("skillId") == Some(effect)
                && n.ancestors()
                    .any(|p| p.has_tag_name("SkillSet") && p.attribute("id") == Some(set))
        })
        .collect();
    assert_eq!(targets.len(), 1, "exact selected physical occurrence");
    targets[0]
}

fn edit(
    xml: &str,
    node: roxmltree::Node<'_, '_>,
    attrs: &[(&str, Option<&str>)],
    children: &str,
) -> String {
    fn escape(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;")
    }
    let range = node.range();
    let body = &xml[range.clone()];
    let end = body.find('>').unwrap();
    let mut start = format!("<{}", node.tag_name().name());
    for attr in node.attributes() {
        if !attrs.iter().any(|(name, _)| *name == attr.name()) {
            start.push_str(&format!(" {}=\"{}\"", attr.name(), escape(attr.value())));
        }
    }
    for (name, value) in attrs {
        if let Some(value) = value {
            start.push_str(&format!(" {name}=\"{}\"", escape(value)));
        }
    }
    start.push('>');
    start.push_str(children);
    if body[..end].ends_with('/') {
        start.push_str(&format!("</{}>", node.tag_name().name()));
    } else {
        start.push_str(&body[end + 1..]);
    }
    let mut result = xml.to_owned();
    result.replace_range(range, &start);
    result
}

fn mutate(
    xml: &str,
    effect: &str,
    attrs: &[(&str, Option<&str>)],
    group: &[(&str, Option<&str>)],
    children: &str,
) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let updated = edit(xml, target(&doc, effect), attrs, children);
    let doc = roxmltree::Document::parse(&updated).unwrap();
    edit(&updated, target(&doc, effect).parent().unwrap(), group, "")
}

fn focus(xml: &str, effect: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let group = target(&doc, effect).parent().unwrap();
    // roxmltree's sibling axis includes the node itself, giving the source's
    // one-based socket-group position without adding another offset.
    let index = group
        .prev_siblings()
        .filter(|node| node.has_tag_name("Skill"))
        .count();
    let index = index.to_string();
    let build = doc.descendants().find(|n| n.has_tag_name("Build")).unwrap();
    let updated = edit(xml, build, &[("mainSocketGroup", Some(index.as_str()))], "");
    let doc = roxmltree::Document::parse(&updated).unwrap();
    let input = doc
        .descendants()
        .find(|n| {
            n.has_tag_name("Input")
                && n.attribute("name") == Some("skill_number")
                && n.parent().is_some_and(|p| p.has_tag_name("Calcs"))
        })
        .unwrap();
    edit(&updated, input, &[("number", Some(index.as_str()))], "")
}

fn child_maps(effect: &str, index: &str, main: &str, calcs: &str) -> String {
    format!(
        "<MinionSkillIndexLookup grantedEffect=\"{effect}\"><MinionSkillIndexMap skillIndex=\"{index}\" statSetIndex=\"{main}\"/></MinionSkillIndexLookup><MinionSkillIndexLookupCalcs grantedEffect=\"{effect}\"><MinionSkillIndexMap skillIndex=\"{index}\" statSetIndex=\"{calcs}\"/></MinionSkillIndexLookupCalcs>"
    )
}

fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(
            value.as_object().is_some_and(|v| v.is_empty()),
            "expected source rows"
        );
        &[]
    }
}
fn selected<'a>(state: &'a Json, key: &str) -> &'a Json {
    rows(&state["selected"])
        .iter()
        .find(|r| r["physical_id"] == key)
        .unwrap()
}
fn assert_same(left: &Json, right: &Json, label: &str) {
    if let Some(difference) = json_evidence::first_difference(left, right, "$") {
        panic!("{label}: {difference}; full evidence in runs/owned-minion-occurrence-inputs-01");
    }
}

fn check(result: &Json) {
    let original = &result["original"];
    assert_same(original, &result["repeated_original"], "fresh repeat");
    assert_same(original, &result["warm_original"], "warm restoration");
    assert_eq!(rows(&original["minions"]["selected"]).len(), 4);
    for state in
        std::iter::once(original).chain(rows(&result["controls"]).iter().map(|c| &c["state"]))
    {
        assert_eq!(state["source_hash"], original["source_hash"]);
        for flag in [
            "saved_instances_preserved",
            "selected_state_preserved",
            "main_and_calcs_outputs_preserved",
            "source_methods_preserved",
            "source_catalog_preserved",
        ] {
            assert_eq!(state["occurrence"][flag], true, "{flag}");
        }
        assert_eq!(state["minions"]["source_methods_preserved"], true);
        for catalog in rows(&state["occurrence"]["catalog"]) {
            assert_eq!(rows(&catalog["effects"]).len(), 1);
            assert!(rows(&catalog["resolved_additional"]).is_empty());
            let unresolved = rows(&catalog["unresolved_references"]);
            assert_eq!(unresolved.len(), 1);
            assert_eq!(unresolved[0]["standalone_skill"], false);
            assert_eq!(unresolved[0]["in_effect_list"], false);
        }
        for row in rows(&state["minions"]["selected"]) {
            assert_eq!(rows(&row["effects"]).len(), 1);
            for mode in ["MAIN", "CALCS"] {
                for summon in rows(&row[mode]) {
                    assert_eq!(summon["physical_source"], true);
                    assert_eq!(summon["count_enabled"], true);
                    assert_eq!(summon["reservation"]["has_reservation"], true);
                    assert_eq!(summon["reservation"]["multiple_reservation"], true);
                    assert_eq!(summon["umbral"]["flag"], false);
                    let actor = &summon["actor"];
                    assert_eq!(actor["source_data_identity"], true);
                    assert_eq!(actor["parent_is_player"], true);
                    assert_eq!(actor["enemy_identity"], true);
                    assert_eq!(actor["item_set_present"], false);
                    let children = rows(&actor["children"]);
                    assert_eq!(children.len(), 2);
                    assert_eq!(children.iter().filter(|c| c["selected"] == true).count(), 1);
                    for child in children {
                        assert_eq!(child["actor_identity"], true);
                        assert_eq!(child["summon_parent"], true);
                        assert_eq!(
                            child["stat_set"]["index"],
                            child["stat_set"]["declared_table_index"]
                        );
                    }
                }
            }
        }
    }
    for gem in rows(&result["reviewed_gems"]) {
        let key = gem["key"].as_str().unwrap();
        let control = |name: &str| -> &Json {
            &rows(&result["controls"])
                .iter()
                .find(|c| c["physical_id"] == key && c["name"] == name)
                .unwrap()["state"]
        };
        let focused = control("focused");
        let baseline = selected(&focused["minions"], key);
        for mode in ["MAIN", "CALCS"] {
            let base = &rows(&baseline[mode])[0];
            assert_eq!(base["is_main_skill"], true);
            assert_eq!(base["actor"]["is_selected_actor"], true);
            assert_eq!(base["count"], 1);
            for (name, count) in [
                ("count-three", 3),
                ("group-four", 4),
                ("group-zero", 0),
                ("full-dps-one", 1),
                ("full-dps-three", 3),
            ] {
                let changed = selected(&control(name)["minions"], key);
                let actions = rows(&changed[mode]);
                assert_eq!(actions.len(), 1, "{key}/{name}/{mode}");
                assert_eq!(actions[0]["count"], count);
                assert_eq!(actions[0]["actor"]["type"], base["actor"]["type"]);
                if name.starts_with("full-dps") {
                    assert_eq!(actions[0]["full_dps_included"], true);
                }
                if count > 1 {
                    let old = base["reservation"]["spirit"].as_f64().unwrap_or(0.0);
                    let new = actions[0]["reservation"]["spirit"].as_f64().unwrap_or(0.0);
                    assert!(
                        new > old,
                        "actual Spirit reservation must respond to count: {key}/{name}/{mode}"
                    );
                }
                if count == 0 {
                    assert!(
                        actions[0]["reservation"]["spirit"].is_null()
                            || actions[0]["reservation"]["spirit"] == 0
                    );
                }
            }
            for name in ["global-one-false", "global-two-false"] {
                let changed = selected(&control(name)["minions"], key);
                let actions = rows(&changed[mode]);
                let requires_global = rows(&baseline["effects"])[0]["has_global_effect"] == true;
                if name == "global-one-false" && requires_global {
                    assert!(actions.is_empty());
                } else {
                    assert_eq!(actions.len(), 1);
                    assert_same(
                        &actions[0],
                        base,
                        "independent global switch has no corresponding global effect",
                    );
                }
            }
            for name in [
                "names-valid",
                "names-invalid",
                "names-absent",
                "actions-absent",
            ] {
                let state = control(name);
                let changed = selected(&state["minions"], key);
                assert_same(
                    &changed[mode],
                    &baseline[mode],
                    "finite source selector fallback",
                );
                assert_same(
                    &state["occurrence"]["outputs"][mode],
                    &focused["occurrence"]["outputs"][mode],
                    "fallback output",
                );
                let physical = selected(&state["occurrence"], key);
                assert_eq!(
                    physical["loaded"][if mode == "MAIN" {
                        "minion"
                    } else {
                        "minion_calcs"
                    }],
                    base["actor"]["type"]
                );
            }
            for (name, main, calcs) in [
                ("action-main-last", 2, 1),
                ("action-calcs-last", 1, 2),
                ("actions-clamped", 1, 2),
            ] {
                let changed = selected(&control(name)["minions"], key);
                let actor = &rows(&changed[mode])[0]["actor"];
                let index = if mode == "MAIN" { main } else { calcs };
                let chosen = &rows(&actor["children"])[index - 1];
                assert_eq!(chosen["selected"], true, "{key}/{name}/{mode}");
                assert_eq!(actor["main_child"], chosen["effect"]);
            }
            for name in ["maps-main-last", "maps-calcs-last"] {
                let changed = selected(&control(name)["minions"], key);
                assert_eq!(rows(&changed["source_children"]).len(), 2);
                let actor = &rows(&changed[mode])[0]["actor"];
                let chosen = rows(&actor["children"])
                    .iter()
                    .find(|c| c["selected"] == true)
                    .unwrap();
                let last = rows(&chosen["stat_sets"]).len();
                let use_last = (name == "maps-main-last" && mode == "MAIN")
                    || (name == "maps-calcs-last" && mode == "CALCS");
                assert_eq!(
                    chosen["stat_set"]["index"],
                    if use_last { last } else { 1 },
                    "{key}/{name}/{mode}"
                );
            }
        }
    }
}
