//! Original support-source construction, before native discovery policy is authored.
use super::*;

const TEST_NAME: &str = "support_origin::composed_support_origins_observe_original_constructors";
const OBSERVER: &str = include_str!("support_origin_source.lua");

#[test]
#[ignore = "requires pinned PoB; complete cold support discovery census"]
fn composed_support_origins_observe_original_constructors() {
    physical_support::run_modes(
        physical_support::Witness {
            name: TEST_NAME,
            child_env: "POE_SUPPORT_ORIGIN_SOURCE_CHILD",
            output_env: "POE_SUPPORT_ORIGIN_SOURCE_OUT",
            default_output: "runs/owned-support-origin-source-01",
            label: "Composed support origins",
        },
        child,
    );
}

fn same(a: &Json, b: &Json, label: &str) {
    assert_eq!(json_evidence::first_difference(a, b, label), None);
}

fn child(root: &Path, out: &Path, enabled: bool) {
    let path = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let bytes = fs::read(&path).unwrap();
    let index: Json =
        serde_json::from_slice(&fs::read(path.parent().unwrap().join("index.json")).unwrap())
            .unwrap();
    assert_eq!(digest(&bytes), index["builds"][4]["xml_sha256"]);
    let xml = std::str::from_utf8(&bytes).unwrap();
    let (disabled, control) = disable_support(xml, SAND, SUPPORTS[1]);
    let (shared, shared_control) = slot_sharing(xml);
    let cases = vec![
        observe(root, "original-05", xml, enabled, true, None),
        observe(root, "repeat-original-05", xml, enabled, true, None),
        observe(root, "unhooked-original-05", xml, enabled, false, None),
        observe(
            root,
            "disabled-sand-magnified",
            &disabled,
            enabled,
            true,
            Some(control),
        ),
        observe(
            root,
            "slot-sharing",
            &shared,
            enabled,
            true,
            Some(shared_control.clone()),
        ),
        observe(
            root,
            "unhooked-slot-sharing",
            &shared,
            enabled,
            false,
            Some(shared_control),
        ),
    ];
    let files = [
        "src/Modules/CalcSetup.lua",
        "src/Modules/CalcActiveSkill.lua",
        "src/Modules/CalcTools.lua",
        "src/Classes/ModStore.lua",
        "src/Classes/ModDB.lua",
        "src/Classes/SkillsTab.lua",
        "src/Modules/Data.lua",
        "src/Data/Gems.lua",
        "src/Modules/ModParser.lua",
    ];
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,
        "manifest_sha256":pinned::manifest_sha256(),"observer_sha256":digest(OBSERVER.as_bytes()),
        "files":files.map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})),
        "original_xml_sha256":digest(&bytes),"fresh_vm_per_case":true,"numeric_tolerance":0,
        "business_wrappers":false,"native_discovery_authority":false,"cases":cases});
    let result = serde_json::to_vec(&report).unwrap();
    assert!(result.len() < 16 * 1024 * 1024);
    let mode = if enabled { "on" } else { "off" };
    fs::write(out.join(format!("source-jit-{mode}.raw.json")), &result).unwrap();
    check(&report);
    fs::write(out.join(format!("source-jit-{mode}.json")), &result).unwrap();
    assert_eq!(fs::read(path).unwrap(), bytes);
    eprintln!("Support origin evidence: {} bytes", result.len());
}

fn observe(
    root: &Path,
    name: &str,
    xml: &str,
    enabled: bool,
    instrumented: bool,
    control: Option<Json>,
) -> Json {
    eprintln!("Support origin case {name}; instrumented={instrumented}, JIT={enabled}");
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("supportOriginJit", enabled)?;
        lua.globals()
            .set("supportOriginInstrumented", instrumented)?;
        lua.load("if supportOriginJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let install = |lua: &Lua| -> Result<Function, RuntimeError> {
        let api: mlua::Table = lua
            .load(OBSERVER)
            .set_name("@support-origin-source")
            .eval()?;
        let install: Function = api.get("install")?;
        let cleanup: Function = install.call(())?;
        lua.globals().set("supportOriginApi", api)?;
        Ok(cleanup)
    };
    let project = |lua: &Lua| -> Result<Json, RuntimeError> {
        let api: mlua::Table = lua.globals().get("supportOriginApi")?;
        let observe: Function = api.get("observe")?;
        let a: Json = lua.from_value(observe.call::<Value>(())?)?;
        let b: Json = lua.from_value(observe.call::<Value>(())?)?;
        same(&a, &b, "read-only projection");
        Ok(a)
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
        Some(&project),
    )
    .unwrap_or_else(|e| panic!("{name}: source observation failed: {e}"));
    assert_eq!(observed["configuration_method_wrappers"], false);
    assert_eq!(observed["original_build_output_available"], true);
    json!({"name":name,"xml_sha256":digest(xml.as_bytes()),"instrumented":instrumented,
        "control":control,"state":observed["additional_observation"]})
}

fn check(report: &Json) {
    let cases = report["cases"].as_array().unwrap();
    same(
        &cases[0]["state"],
        &cases[1]["state"],
        "independent fresh repeat",
    );
    let mut baseline = cases[0]["state"].clone();
    for env in baseline["environments"].as_array_mut().unwrap() {
        env.as_object_mut().unwrap().remove("calls");
    }
    same(&baseline, &cases[2]["state"], "unhooked cold control");
    let mut shared = cases[4]["state"].clone();
    for env in shared["environments"].as_array_mut().unwrap() {
        env.as_object_mut().unwrap().remove("calls");
    }
    same(&shared, &cases[5]["state"], "unhooked slot-sharing control");
    for case in cases {
        let state = &case["state"];
        assert_eq!(state["selected"]["skills"], 4);
        assert_eq!(state["original_methods_preserved"], true);
        assert_eq!(state["hook_removed"], true);
        assert_eq!(state["source_tables_mutated"], false);
        if case["instrumented"] != true {
            continue;
        }
        for env in state["environments"].as_array().unwrap() {
            let calls = &env["calls"];
            let queries = calls["queries"].as_array().unwrap();
            assert_eq!(
                queries
                    .iter()
                    .filter(|q| q["name"] == "LinkedSupport")
                    .count(),
                1
            );
            assert!(queries.iter().any(|q| q["name"] == "ExtraSupport"));
            for q in queries {
                assert_eq!(q["original_call"], true);
                assert_eq!(q["original_return"], true);
                assert_eq!(q["exact_actor_store"], true);
                assert_eq!(q["result_count"], 0);
                for store in rows(&q["before"]) {
                    assert!(rows(&store["rows"]).is_empty());
                }
            }
            assert!(!calls["constructors"].as_array().unwrap().is_empty());
            let processed = calls["processed"].as_array().unwrap();
            assert!(processed.iter().any(|p| p["branch"] == "primary"));
            assert!(processed.iter().any(|p| p["branch"] == "additional"));
            // PoB attaches gemData even to the manual Djinn rows. It is source
            // metadata, not authority to classify a native physical Gem.
            assert!(!processed.iter().any(|p| p["branch"] == "without_gem_data"));
            for row in processed {
                assert_eq!(row["exact_environment"], true);
                if row["branch"] == "additional" {
                    assert_eq!(row["effect"]["support"], false);
                }
            }
            let constructors = calls["constructors"].as_array().unwrap();
            assert_eq!(constructors.len(), 16);
            let denied: Vec<_> = constructors
                .iter()
                .filter(|c| c["no_supports"] == true)
                .collect();
            assert_eq!(denied.len(), 4);
            for c in denied {
                assert!(rows(&c["candidates"]).is_empty());
            }
            let manual_sand: Vec<_> = constructors
                .iter()
                .filter(|c| c["effect"]["id"] == SAND && c["group_source"]["kind"] == "absent")
                .collect();
            assert_eq!(manual_sand.len(), 1);
            assert_eq!(manual_sand[0]["effect"]["from_tree"], true);
            assert_eq!(
                rows(&manual_sand[0]["candidates"]).len(),
                if case["name"] == "disabled-sand-magnified" {
                    2
                } else {
                    3
                }
            );
            let firebolt: Vec<_> = constructors
                .iter()
                .filter(|c| c["effect"]["id"] == "FireboltPlayer")
                .collect();
            assert_eq!(firebolt.len(), 1);
            let borrowed = rows(&firebolt[0]["candidates"]);
            if case["name"] == "slot-sharing" {
                assert_eq!(borrowed.len(), 3);
                for candidate in borrowed {
                    assert_eq!(candidate["source"]["group"], manual_sand[0]["group"]);
                    assert_ne!(candidate["source"]["group"], firebolt[0]["group"]);
                }
            } else {
                assert!(borrowed.is_empty());
            }
        }
    }
    assert_ne!(cases[0]["xml_sha256"], cases[3]["xml_sha256"]);
    assert_ne!(cases[0]["state"]["groups"], cases[3]["state"]["groups"]);
}

fn slot_sharing(xml: &str) -> (String, Json) {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let skills = doc
        .descendants()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let set = skills
        .children()
        .find(|n| {
            n.has_tag_name("SkillSet") && n.attribute("id") == skills.attribute("activeSkillSet")
        })
        .unwrap();
    let matching: Vec<_> = set
        .children()
        .filter(|n| {
            n.has_tag_name("Skill")
                && n.attribute("source").is_none()
                && n.children()
                    .any(|g| g.has_tag_name("Gem") && g.attribute("skillId") == Some(SAND))
        })
        .collect();
    assert_eq!(matching.len(), 1);
    let node = matching[0];
    assert!(node.attribute("slot").is_none());
    let range = node.range();
    let before = &xml[range.clone()];
    let after = magnified_area_support::set_attr(before, "slot", "Weapon 1");
    let mut changed = xml.to_owned();
    changed.replace_range(range, &after);
    let before_tag = &before[..before.find('>').unwrap() + 1];
    let after_tag = &after[..after.find('>').unwrap() + 1];
    assert_eq!(&before[before_tag.len()..], &after[after_tag.len()..]);
    (
        changed,
        json!({"kind":"source_slot_sharing_control","source_group":SAND,"slot":"Weapon 1",
        "only_opening_tag_changed":true,"before":before_tag,"after":after_tag,"game_legality_authority":false}),
    )
}
