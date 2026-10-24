//! Exact outside-Config omissions under the unchanged normal build loader.
use super::*;
use std::ops::Range;

const TEST: &str = "outside_config_defaults::fresh_outside_configuration_defaults_and_contrasts";
const CHILD: &str = "POE_OUTSIDE_CONFIG_DEFAULTS_CHILD";
const OUTPUT: &str = "POE_OUTSIDE_CONFIG_DEFAULTS_OUT";
const LOADOUT_TEST: &str =
    "outside_config_defaults::fresh_loadout_titles_preserve_actual_loader_selection_evidence";
const LOADOUT_CHILD: &str = "POE_LOADOUT_TITLES_CHILD";
const LOADOUT_OUTPUT: &str = "POE_LOADOUT_TITLES_OUT";
const OBSERVER: &str = include_str!("outside_config_defaults_source.lua");
const ORIGINAL: &str =
    include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-05.xml");

#[derive(Clone)]
struct Edit {
    range: Range<usize>,
    replacement: String,
}
struct Control {
    name: &'static str,
    xml: String,
    edits: Vec<Edit>,
}
fn controls(original: &str) -> Vec<Control> {
    let doc = roxmltree::Document::parse(original).unwrap();
    let build = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Build"))
        .unwrap();
    let timeless: Vec<_> = build
        .children()
        .filter(|n| n.has_tag_name("TimelessData"))
        .collect();
    assert_eq!(timeless.len(), 1);
    assert_eq!(timeless[0].attributes().len(), 4);
    assert_eq!(timeless[0].attribute("devotionVariant1"), Some("1"));
    assert_eq!(timeless[0].attribute("devotionVariant2"), Some("1"));
    assert_eq!(timeless[0].attribute("searchList"), Some(""));
    assert_eq!(timeless[0].attribute("searchListFallback"), Some(""));
    let calcs = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Calcs"))
        .unwrap();
    assert_eq!(
        calcs
            .children()
            .filter(|n| n.has_tag_name("Section"))
            .count(),
        49
    );
    let inputs: Vec<_> = calcs
        .children()
        .filter(|n| n.has_tag_name("Input"))
        .collect();
    assert_eq!(inputs.len(), 2);
    let skill = *inputs
        .iter()
        .find(|n| n.attribute("name") == Some("skill_number"))
        .unwrap();
    let buff = *inputs
        .iter()
        .find(|n| n.attribute("name") == Some("misc_buffMode"))
        .unwrap();
    assert_eq!(skill.attribute("number"), Some("1"));
    assert_eq!(buff.attribute("string"), Some("EFFECTIVE"));
    let omitted_timeless = Edit {
        range: timeless[0].range(),
        replacement: String::new(),
    };
    let omitted_calcs: Vec<_> = inputs
        .iter()
        .map(|n| Edit {
            range: n.range(),
            replacement: String::new(),
        })
        .collect();
    let mut both = omitted_calcs.clone();
    both.push(omitted_timeless.clone());
    let specifications = [
        ("original-05", vec![]),
        ("omit-timeless", vec![omitted_timeless]),
        ("omit-calcs-inputs", omitted_calcs),
        ("omit-both", both),
        ("calcs-skill-three", vec![Edit { range: skill.range(), replacement: r#"<Input number="3" name="skill_number"/>"#.into() }]),
        ("calcs-unbuffed", vec![Edit { range: buff.range(), replacement: r#"<Input string="UNBUFFED" name="misc_buffMode"/>"#.into() }]),
        ("nondefault-timeless", vec![Edit { range: timeless[0].range(), replacement: r#"<TimelessData devotionVariant1="2" devotionVariant2="3" searchList="source-witness" searchListFallback="fallback-witness" socketFilter="true" socketFilterDistance="7"/>"#.into() }]),
        ("original-05-repeat", vec![]),
    ];
    specifications
        .into_iter()
        .map(|(name, mut edits)| {
            edits.sort_by_key(|e| e.range.start);
            assert!(edits.windows(2).all(|w| w[0].range.end <= w[1].range.start));
            let mut xml = original.to_owned();
            for edit in edits.iter().rev() {
                xml.replace_range(edit.range.clone(), &edit.replacement);
            }
            Control { name, xml, edits }
        })
        .collect()
}

fn loadout_controls(original: &str) -> Vec<Control> {
    let doc = roxmltree::Document::parse(original).unwrap();
    let items: Vec<_> = doc
        .root_element()
        .children()
        .filter(|node| node.has_tag_name("Items"))
        .collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].attribute("activeItemSet"), Some("2"));
    let selected: Vec<_> = items[0]
        .children()
        .filter(|node| node.has_tag_name("ItemSet") && node.attribute("id") == Some("2"))
        .collect();
    assert_eq!(selected.len(), 1);
    let title = selected[0]
        .attributes()
        .find(|attribute| attribute.name() == "title")
        .unwrap();
    assert_eq!(title.value(), "Act 1 & 2 (Pre-Ascend)");
    let range = title.range();
    assert_eq!(
        &original[range.clone()],
        r#"title="Act 1 &amp; 2 (Pre-Ascend)""#
    );
    [
        ("original-05", None),
        ("title-nonmatching", Some("No shared loadout")),
        ("title-substring", Some("Prefix Act 3")),
        ("title-exact", Some("Act 3")),
        ("title-substring-repeat", Some("Prefix Act 3")),
        ("original-05-repeat", None),
    ]
    .into_iter()
    .map(|(name, replacement)| {
        let edits = replacement
            .map(|title| Edit {
                range: range.clone(),
                replacement: format!(r#"title="{title}""#),
            })
            .into_iter()
            .collect::<Vec<_>>();
        let mut xml = original.to_owned();
        for edit in &edits {
            xml.replace_range(edit.range.clone(), &edit.replacement);
        }
        Control { name, xml, edits }
    })
    .collect()
}

fn saved_axes(xml: &str) -> Json {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let header = |name: &str| {
        let found = doc
            .root_element()
            .children()
            .filter(|node| node.has_tag_name(name))
            .collect::<Vec<_>>();
        assert_eq!(found.len(), 1);
        found[0]
    };
    json!({
        "spec": header("Tree").attribute("activeSpec").unwrap(),
        "skills": header("Skills").attribute("activeSkillSet").unwrap(),
        "items": header("Items").attribute("activeItemSet").unwrap(),
        "config": header("Config").attribute("activeConfigSet").unwrap(),
        "main_group": header("Build").attribute("mainSocketGroup").unwrap(),
    })
}

fn assert_unchanged_surroundings(case: &Control) {
    let mut source_offset = 0;
    let mut target_offset = 0;
    for edit in &case.edits {
        let unchanged = &ORIGINAL[source_offset..edit.range.start];
        assert_eq!(
            &case.xml[target_offset..target_offset + unchanged.len()],
            unchanged,
            "{} surrounding bytes",
            case.name
        );
        target_offset += unchanged.len();
        assert_eq!(
            &case.xml[target_offset..target_offset + edit.replacement.len()],
            edit.replacement
        );
        target_offset += edit.replacement.len();
        source_offset = edit.range.end;
    }
    assert_eq!(
        &case.xml[target_offset..],
        &ORIGINAL[source_offset..],
        "{} trailing bytes",
        case.name
    );
}

#[test]
fn loadout_title_controls_change_only_the_exact_selected_items_title() {
    let cases = loadout_controls(ORIGINAL);
    assert_eq!(cases.len(), 6);
    for case in &cases {
        assert_unchanged_surroundings(case);
        assert_eq!(
            case.edits.len(),
            usize::from(case.name.starts_with("title-"))
        );
        let doc = roxmltree::Document::parse(&case.xml).unwrap();
        let items = doc
            .root_element()
            .children()
            .find(|node| node.has_tag_name("Items"))
            .unwrap();
        assert_eq!(items.attribute("activeItemSet"), Some("2"));
        assert_eq!(saved_axes(&case.xml), saved_axes(ORIGINAL));
        for (id, expected_title) in [("2", None), ("4", Some("Act 3"))] {
            let set = items
                .children()
                .find(|node| node.has_tag_name("ItemSet") && node.attribute("id") == Some(id))
                .unwrap();
            if let Some(title) = expected_title {
                assert_eq!(set.attribute("title"), Some(title));
            }
            let weapon = set
                .children()
                .find(|node| {
                    node.has_tag_name("Slot") && node.attribute("name") == Some("Weapon 1")
                })
                .unwrap();
            assert_eq!(
                weapon.attribute("itemId"),
                Some(if id == "2" { "28" } else { "24" })
            );
        }
        assert_eq!(
            doc.descendants()
                .filter(|node| node.has_tag_name("Section"))
                .count(),
            49
        );
    }
    assert_eq!(cases[2].xml, cases[4].xml);
    assert_eq!(cases[0].xml, cases[5].xml);
}

#[test]
fn outside_configuration_controls_preserve_every_unedited_xml_byte_and_all_sections() {
    let cases = controls(ORIGINAL);
    assert_eq!(cases.len(), 8);
    let before = roxmltree::Document::parse(ORIGINAL).unwrap();
    let sections = |doc: &roxmltree::Document<'_>| {
        doc.descendants()
            .filter(|n| n.has_tag_name("Section"))
            .map(|n| doc.input_text()[n.range()].to_owned())
            .collect::<Vec<_>>()
    };
    for case in &cases {
        let after = roxmltree::Document::parse(&case.xml).unwrap();
        assert_eq!(
            sections(&after),
            sections(&before),
            "{} Section preservation",
            case.name
        );
        assert_unchanged_surroundings(case);
        let calcs = after
            .root_element()
            .children()
            .find(|n| n.has_tag_name("Calcs"))
            .unwrap();
        assert_eq!(
            calcs.children().filter(|n| n.has_tag_name("Input")).count(),
            if matches!(case.name, "omit-calcs-inputs" | "omit-both") {
                0
            } else {
                2
            }
        );
        assert_eq!(
            after
                .descendants()
                .filter(|n| n.has_tag_name("TimelessData"))
                .count(),
            usize::from(!matches!(case.name, "omit-timeless" | "omit-both"))
        );
    }
}

#[test]
#[ignore = "optional complete pinned PoB source witness; dedicated fresh JIT processes"]
fn fresh_outside_configuration_defaults_and_contrasts() {
    supervise(TEST, CHILD, OUTPUT, false);
}

#[test]
#[ignore = "optional complete pinned PoB source witness; dedicated fresh JIT processes"]
fn fresh_loadout_titles_preserve_actual_loader_selection_evidence() {
    supervise(LOADOUT_TEST, LOADOUT_CHILD, LOADOUT_OUTPUT, true);
}

fn supervise(test: &str, child_env: &str, output_env: &str, loadouts: bool) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    if let Some(mode) = std::env::var_os(child_env) {
        assert!(mode == "off" || mode == "on");
        let out = root.join(std::env::var_os(output_env).expect("supervisor output"));
        assert!(out.is_dir());
        run_child(&root, &out, mode == "on", loadouts);
        return;
    }
    let out = if let Some(path) = std::env::var_os(output_env) {
        let out = root.join(path);
        assert!(
            !out.exists(),
            "refusing evidence overwrite {}",
            out.display()
        );
        fs::create_dir_all(&out).unwrap();
        out
    } else {
        let runs = root.join("runs");
        fs::create_dir_all(&runs).unwrap();
        tempfile::Builder::new()
            .prefix("outside-config-defaults-")
            .tempdir_in(runs)
            .unwrap()
            .keep()
    };
    eprintln!("outside-Config evidence: {}", out.display());
    for mode in ["off", "on"] {
        let log_path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&log_path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--ignored", "--nocapture"])
            .env(child_env, mode)
            .env(output_env, &out)
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
                    "source child failed: {}\n{}",
                    log_path.display(),
                    tail(&log_path)
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(300) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "source timeout: {}\n{}",
                    log_path.display(),
                    tail(&log_path)
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    same(
        &read(&out.join("source-jit-off.json")),
        &read(&out.join("source-jit-on.json")),
        "fresh JIT source witness differs; full reports retained",
    );
}

fn run_child(root: &Path, out: &Path, enabled: bool, loadouts: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let fixture = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let original = fs::read_to_string(&fixture).unwrap();
    let index = read(&fixture.parent().unwrap().join("index.json"));
    assert_eq!(
        rows(&index["builds"])
            .iter()
            .find(|row| row["xml"] == "build-05.xml")
            .unwrap()["xml_sha256"],
        digest(original.as_bytes())
    );
    assert_eq!(original, ORIGINAL);
    let mut cases = Vec::new();
    let report_path = out.join(format!(
        "source-jit-{}.json",
        if enabled { "on" } else { "off" }
    ));
    let controls = if loadouts {
        loadout_controls(&original)
    } else {
        controls(&original)
    };
    let fresh_loads = controls.len();
    for case in controls {
        assert_unchanged_surroundings(&case);
        eprintln!("outside-Config source case {}", case.name);
        let before = |lua: &Lua| {
            lua.globals().set("outsideConfigJit", enabled)?;
            lua.globals().set("outsideConfigLoadouts", loadouts)?;
            lua.load("if outsideConfigJit then jit.on() else jit.off(); jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| {
            lua.globals().set("outsideConfigPhase", "before")?;
            Ok(lua
                .load(OBSERVER)
                .set_name("@outside-config-defaults-source")
                .eval::<Function>()?)
        };
        let observe = |lua: &Lua| {
            lua.globals().set("outsideConfigPhase", "observe")?;
            let value: Value = lua
                .load(OBSERVER)
                .set_name("@outside-config-defaults-source")
                .eval()?;
            Ok(lua.from_value::<Json>(value)?)
        };
        let scratch = tempfile::tempdir().unwrap();
        let result = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &case.xml,
            None,
            !loadouts && !case.name.starts_with("original-"),
            Some(&before),
            Some(&install),
            Some(&observe),
        );
        let edits: Vec<_> = case.edits.iter().map(|e| json!({"start":e.range.start,"end":e.range.end,"before":&original[e.range.clone()],"after":e.replacement})).collect();
        let row = match result {
            Ok(value) => {
                assert_eq!(value["source_hash"], pinned::manifest_sha256());
                assert_eq!(value["configuration_method_wrappers"], false);
                assert_eq!(value["original_build_output_available"], true);
                json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),"saved_axes":saved_axes(&case.xml),"edits":edits,"available":true,"observed":value["additional_observation"]})
            }
            Err(error) => {
                json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),"saved_axes":saved_axes(&case.xml),"edits":edits,"available":false,"source_error":error.to_string()})
            }
        };
        cases.push(row);
        fs::write(
            &report_path,
            serde_json::to_vec_pretty(&json!({"cases":cases})).unwrap(),
        )
        .unwrap();
    }
    assert_eq!(fs::read_to_string(fixture).unwrap(), original);
    let files = [
        "src/Modules/Build.lua",
        "src/Classes/CalcsTab.lua",
        "src/Modules/CalcSetup.lua",
        "src/Modules/Calcs.lua",
        "src/Modules/CalcPerform.lua",
        "src/Modules/Common.lua",
        "src/Classes/ConfigTab.lua",
        "src/Classes/SkillsTab.lua",
        "src/Classes/ItemsTab.lua",
        "src/Classes/TreeTab.lua",
        "src/Classes/DropDownControl.lua",
    ]
    .map(|path| json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}));
    let result = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","source_hash":pinned::manifest_sha256(),
        "observer_sha256":digest(OBSERVER.as_bytes()),"bootstrap_sha256":digest(include_bytes!("configuration_preparation_source.rs")),
        "fresh_loads":fresh_loads,"original_xml_sha256":digest(original.as_bytes()),"native_effect_coverage":false,"source_inventory_complete":false,
        "loadout_title_probe":loadouts,"selection_correspondence_proven":false,
        "files":files,"cases":cases});
    fs::write(&report_path, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    if loadouts {
        check_loadouts(&result);
    } else {
        check(&result);
    }
}

fn check_loadouts(result: &Json) {
    assert_eq!(rows(&result["cases"]).len(), 6);
    let original = saved(result, "original-05");
    same(
        original,
        saved(result, "original-05-repeat"),
        "independent original replay",
    );
    same(
        saved(result, "title-substring"),
        saved(result, "title-substring-repeat"),
        "independent title-substring replay",
    );
    assert_eq!(
        original["selected"],
        json!({"config":1,"skills":4,"items":2,"spec":3,"main_group":3})
    );
    assert_eq!(original["equipment_slots"]["Weapon 1"]["selItemId"], 28);
    for (name, item_set, weapon, offhand, item_source, skill, gem_id) in [
        (
            "original-05",
            2,
            28,
            0,
            "Item:28:New Item, Ashen Staff",
            "FireboltPlayer",
            "Metadata/Items/Gems/SkillGemFirebolt",
        ),
        (
            "title-nonmatching",
            2,
            28,
            0,
            "Item:28:New Item, Ashen Staff",
            "FireboltPlayer",
            "Metadata/Items/Gems/SkillGemFirebolt",
        ),
        (
            "title-substring",
            4,
            24,
            25,
            "Item:24:New Item, Rattling Sceptre",
            "SummonSkeletalWarriorsPlayer",
            "Metadata/Items/Gems/SkillGemSkeletalWarrior",
        ),
        (
            "title-exact",
            2,
            28,
            0,
            "Item:28:New Item, Ashen Staff",
            "FireboltPlayer",
            "Metadata/Items/Gems/SkillGemFirebolt",
        ),
        (
            "title-substring-repeat",
            4,
            24,
            25,
            "Item:24:New Item, Rattling Sceptre",
            "SummonSkeletalWarriorsPlayer",
            "Metadata/Items/Gems/SkillGemSkeletalWarrior",
        ),
        (
            "original-05-repeat",
            2,
            28,
            0,
            "Item:28:New Item, Ashen Staff",
            "FireboltPlayer",
            "Metadata/Items/Gems/SkillGemFirebolt",
        ),
    ] {
        let row = case(result, name);
        let snapshot = saved(result, name);
        assert_eq!(
            row["saved_axes"],
            json!({"spec":"3","skills":"4","items":"2","config":"1","main_group":"3"})
        );
        same(
            &row["observed"]["load_return"],
            snapshot,
            "unchanged state after original loader return",
        );
        assert_eq!(row["observed"]["selection_correspondence_proven"], false);
        assert_eq!(row["observed"]["source_inventory_complete"], false);
        assert_eq!(row["observed"]["native_effect_coverage"], false);
        assert_eq!(
            snapshot["selected"],
            json!({"config":1,"skills":4,"items":item_set,"spec":3,"main_group":3}),
            "{name} actual selected tuple"
        );
        assert_eq!(
            snapshot["equipment_slots"]["Weapon 1"]["selItemId"], weapon,
            "{name}"
        );
        assert_eq!(
            snapshot["equipment_slots"]["Weapon 2"]["selItemId"], offhand,
            "{name}"
        );
        let item_ids = BTreeMap::from([(weapon.to_string(), true)]);
        let expected_provider = json!({
            "item_present":true,"node_present":false,
            "item_ids":item_ids,"allocated_node_ids":{}
        });
        let item_groups = rows(&snapshot["groups"])
            .iter()
            .filter(|group| group["provider"]["item_present"] == true)
            .collect::<Vec<_>>();
        assert_eq!(item_groups.len(), 1, "{name} exact generated item group");
        let group = item_groups[0];
        assert_eq!(group["index"], 12, "{name}");
        assert_eq!(group["saved"]["source"], item_source, "{name}");
        assert_eq!(group["saved"]["slot"], "Weapon 1", "{name}");
        same(&group["provider"], &expected_provider, name);
        let gems = rows(&group["gems"]);
        assert_eq!(gems.len(), 1, "{name}");
        assert_eq!(gems[0]["skillId"], skill, "{name}");
        assert_eq!(gems[0]["gemId"], gem_id, "{name}");
        assert_eq!(gems[0]["level"], 11, "{name}");
        assert_eq!(gems[0]["quality"], 0, "{name}");
        for field in ["fromItem", "enabled", "enableGlobal1"] {
            assert_eq!(gems[0][field], true, "{name} {field}");
        }
        if weapon == 28 {
            assert_eq!(gems[0]["enableGlobal2"], false, "{name}");
        } else {
            assert!(
                gems[0].get("enableGlobal2").is_none(),
                "{name} absence remains distinct"
            );
        }
        for mode in ["MAIN", "CALCS"] {
            let observed = &snapshot["modes"][mode];
            assert_eq!(
                observed["main_group"],
                if mode == "MAIN" { 3 } else { 1 },
                "{name} {mode}"
            );
            let item_grants = rows(&observed["grants"])
                .iter()
                .filter(|grant| grant["provider"]["item_present"] == true)
                .collect::<Vec<_>>();
            assert_eq!(item_grants.len(), 1, "{name} {mode} exact item grant");
            let grant = item_grants[0];
            same(
                &grant["provider"],
                &expected_provider,
                &format!("{name} {mode}"),
            );
            assert_eq!(grant["saved"]["source"], item_source, "{name} {mode}");
            assert_eq!(grant["saved"]["slotName"], "Weapon 1", "{name} {mode}");
            assert_eq!(grant["saved"]["skillId"], skill, "{name} {mode}");
            assert_eq!(grant["saved"]["level"], 11, "{name} {mode}");
        }
    }
    // These exact contrasts were observed under the unmodified original loader
    // in both fresh JIT modes. They authorize no native selection or broad
    // source-inventory closure, and are not a general title-matching policy.
}

fn saved<'a>(result: &'a Json, name: &str) -> &'a Json {
    let row = case(result, name);
    assert_eq!(row["available"], true, "{name}: {}", row["source_error"]);
    for field in ["methods_preserved", "objects_preserved"] {
        assert_eq!(row["observed"][field], true, "{name} {field}");
    }
    &row["observed"]["saved"]
}
fn check(result: &Json) {
    assert_eq!(rows(&result["cases"]).len(), 8);
    let baseline = saved(result, "original-05");
    same(
        baseline,
        saved(result, "original-05-repeat"),
        "independent original replay",
    );
    assert_eq!(baseline["calcs_input"]["skill_number"], 1);
    assert_eq!(baseline["calcs_input"]["misc_buffMode"], "EFFECTIVE");
    assert_eq!(baseline["timeless"]["socketFilter"], false);
    assert_eq!(baseline["timeless"]["socketFilterDistance"], 0);
    for name in ["omit-timeless", "omit-calcs-inputs", "omit-both"] {
        let observed = saved(result, name);
        for field in ["selected", "calcs_input", "sections", "config", "modes"] {
            same(
                &baseline[field],
                &observed[field],
                &format!("{name} {field}"),
            );
        }
        if name == "omit-calcs-inputs" {
            same(&baseline["timeless"], &observed["timeless"], name);
        } else {
            assert!(
                observed["timeless"].get("socketFilter").is_none(),
                "constructor omission stays absent"
            );
            assert!(
                observed["timeless"].get("socketFilterDistance").is_none(),
                "constructor omission stays absent"
            );
            let mut expected = baseline["timeless"].clone();
            expected.as_object_mut().unwrap().remove("socketFilter");
            expected
                .as_object_mut()
                .unwrap()
                .remove("socketFilterDistance");
            same(
                &expected,
                &observed["timeless"],
                &format!("{name} exact constructor differences"),
            );
        }
    }
    let selected = saved(result, "calcs-skill-three");
    assert_eq!(selected["calcs_input"]["skill_number"], 3);
    assert_eq!(selected["modes"]["CALCS"]["main_group"], 3);
    assert_ne!(
        selected["modes"]["CALCS"]["skill"], baseline["modes"]["CALCS"]["skill"],
        "positive query-selection contrast"
    );
    same(
        &selected["modes"]["MAIN"],
        &baseline["modes"]["MAIN"],
        "CALCS selection must not replace MAIN",
    );
    let unbuffed = saved(result, "calcs-unbuffed");
    assert_eq!(unbuffed["calcs_input"]["misc_buffMode"], "UNBUFFED");
    for flag in ["buffs", "combat", "effective"] {
        assert_eq!(baseline["modes"]["CALCS"][flag], true);
        assert_eq!(unbuffed["modes"]["CALCS"][flag], false);
        assert_eq!(unbuffed["modes"]["MAIN"][flag], true);
    }
    let timeless = saved(result, "nondefault-timeless");
    for (field, expected) in [
        ("devotionVariant1", json!(2)),
        ("devotionVariant2", json!(3)),
        ("socketFilter", json!(true)),
        ("socketFilterDistance", json!(7)),
        ("searchList", json!("source-witness")),
        ("searchListFallback", json!("fallback-witness")),
    ] {
        assert_eq!(
            timeless["timeless"][field], expected,
            "positive TimelessData consumer {field}"
        );
    }
    // Retain all nondefault contrast outputs verbatim. Their equality is not
    // assumed to prove that arbitrary saved outside-Config controls are inert.
}
