//! Independent original calls for three selected Command Skill Damage passives.
use super::*;
#[allow(dead_code)]
#[path = "../../../../tests/support/owned_command_damage_evidence.rs"]
mod evidence;

const TEST_NAME: &str = "command_damage::command_damage_observes_original_consumers";
const CHILD_ENV: &str = "POE_COMMAND_DAMAGE_SOURCE_CHILD";
const LUA: &str = include_str!("command_damage_source.lua");
const NODES: [(u64, f64); 3] = [(25927, 20.0), (41511, 15.0), (32847, 20.0)];
// Removing either branch entrance requires its terminal tail to be removed as
// well. Node35560 has its own conditional damage law, outside this witness's
// three reviewed producers; its removal cannot be treated as numerically inert.
const BRANCHES: [(u64, &[u64]); 3] = [
    (25927, &[25927, 32847]),
    (41511, &[41511, 35560]),
    (32847, &[32847]),
];
const ALL_BRANCH_NODES: &[u64] = &[25927, 41511, 32847, 35560];

struct Control {
    name: String,
    xml: String,
    child: usize,
    set: usize,
    removed: Vec<u64>,
}
impl Control {
    fn reviewed_sources_removed(&self) -> Vec<u64> {
        NODES
            .iter()
            .map(|(id, _)| *id)
            .filter(|id| self.removed.contains(id))
            .collect()
    }
}

fn focus(xml: &str, child: usize, set: usize) -> String {
    let xml = calcs_input(
        &calcs_input(xml, "skill_number", "number", "3"),
        "misc_buffMode",
        "string",
        "EFFECTIVE",
    );
    let xml = gem_attribute(
        &gem_attribute(&xml, SNIPER, "skillMinionSkill", &child.to_string()),
        SNIPER,
        "skillMinionSkillCalcs",
        &child.to_string(),
    );
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let gem = selected_gem(&doc, SNIPER);
    assert!(gem.children().next().is_none());
    let range = gem.range();
    let text = &xml[range.clone()];
    assert!(text.ends_with("/>"));
    let next = format!(
        "{}><MinionSkillIndexLookup grantedEffect=\"{SNIPER}\"><MinionSkillIndexMap skillIndex=\"{child}\" statSetIndex=\"{set}\"/></MinionSkillIndexLookup><MinionSkillIndexLookupCalcs grantedEffect=\"{SNIPER}\"><MinionSkillIndexMap skillIndex=\"{child}\" statSetIndex=\"{set}\"/></MinionSkillIndexLookupCalcs></Gem>",
        &text[..text.len() - 2]
    );
    let mut out = xml.to_owned();
    out.replace_range(range, &next);
    out
}

fn cases(original: &str) -> Vec<Control> {
    let mut cases = vec![Control {
        name: "original-05".into(),
        xml: original.into(),
        child: 0,
        set: 0,
        removed: vec![],
    }];
    for (child, set) in [(1, 1), (2, 1), (2, 2), (2, 3)] {
        let xml = focus(original, child, set);
        cases.push(Control {
            name: format!("selected-{child}-{set}"),
            xml: xml.clone(),
            child,
            set,
            removed: vec![],
        });
        let mut none = xml;
        for node in ALL_BRANCH_NODES {
            none = remove_node(&none, &node.to_string());
        }
        cases.push(Control {
            name: format!("removed-command-branches-{child}-{set}"),
            xml: none,
            child,
            set,
            removed: ALL_BRANCH_NODES.to_vec(),
        });
    }
    for (id, removed) in BRANCHES {
        let mut xml = focus(original, 2, 1);
        for node in removed {
            xml = remove_node(&xml, &node.to_string());
        }
        cases.push(Control {
            name: if removed.len() == 1 {
                format!("removed-{id}")
            } else {
                format!("removed-{id}-branch")
            },
            xml,
            child: 2,
            set: 1,
            removed: removed.to_vec(),
        });
    }
    cases.push(Control {
        name: "original-05-repeat".into(),
        xml: original.into(),
        child: 0,
        set: 0,
        removed: vec![],
    });
    cases.push(Control {
        name: "selected-2-3-repeat".into(),
        xml: focus(original, 2, 3),
        child: 2,
        set: 3,
        removed: vec![],
    });
    assert_eq!(cases.len(), 14);
    cases
}

#[test]
fn command_damage_controls_preserve_source_inputs_outside_declared_edits() {
    let original = include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-05.xml");
    let before = roxmltree::Document::parse(original).unwrap();
    let original_nodes = selected_nodes(&before);
    for c in cases(original) {
        let (removed, reviewed): (&[u64], &[u64]) = match c.name.as_str() {
            "removed-command-branches-1-1"
            | "removed-command-branches-2-1"
            | "removed-command-branches-2-2"
            | "removed-command-branches-2-3" => {
                (&[25927, 41511, 32847, 35560], &[25927, 41511, 32847])
            }
            "removed-25927-branch" => (&[25927, 32847], &[25927, 32847]),
            "removed-41511-branch" => (&[41511, 35560], &[41511]),
            "removed-32847" => (&[32847], &[32847]),
            _ => (&[], &[]),
        };
        assert_eq!(c.removed, removed, "{} explicit allocation closure", c.name);
        assert_eq!(
            c.reviewed_sources_removed(),
            reviewed,
            "{} reviewed source subset",
            c.name
        );
        let after = roxmltree::Document::parse(&c.xml).unwrap();
        for tag in ["Build", "Items", "Config"] {
            let a = before
                .root_element()
                .children()
                .find(|n| n.has_tag_name(tag))
                .unwrap();
            let b = after
                .root_element()
                .children()
                .find(|n| n.has_tag_name(tag))
                .unwrap();
            assert_eq!(&original[a.range()], &c.xml[b.range()], "{} {tag}", c.name);
        }
        let expected: Vec<_> = original_nodes
            .iter()
            .filter(|id| !c.removed.contains(id))
            .copied()
            .collect();
        assert_eq!(selected_nodes(&after), expected, "{}", c.name);
        let gems_before: Vec<_> = selected_skill_set(&before)
            .descendants()
            .filter(|n| n.has_tag_name("Gem") && n.attribute("skillId") != Some(SNIPER))
            .collect();
        let gems_after: Vec<_> = selected_skill_set(&after)
            .descendants()
            .filter(|n| n.has_tag_name("Gem") && n.attribute("skillId") != Some(SNIPER))
            .collect();
        assert_eq!(gems_before.len(), gems_after.len());
        for (a, b) in gems_before.into_iter().zip(gems_after) {
            assert_eq!(&original[a.range()], &c.xml[b.range()]);
        }
        if c.child != 0 {
            let gem = selected_gem(&after, SNIPER);
            for kind in ["MinionSkillIndexLookup", "MinionSkillIndexLookupCalcs"] {
                let lookup = gem.children().find(|n| n.has_tag_name(kind)).unwrap();
                assert_eq!(lookup.attribute("grantedEffect"), Some(SNIPER));
                let map = lookup.children().find(|n| n.is_element()).unwrap();
                assert_eq!(
                    map.attribute("skillIndex")
                        .unwrap()
                        .parse::<usize>()
                        .unwrap(),
                    c.child
                );
                assert_eq!(
                    map.attribute("statSetIndex")
                        .unwrap()
                        .parse::<usize>()
                        .unwrap(),
                    c.set
                );
            }
        }
    }
}

fn selected_nodes(doc: &roxmltree::Document<'_>) -> Vec<u64> {
    let tree = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Tree"))
        .unwrap();
    let active: usize = tree.attribute("activeSpec").unwrap().parse().unwrap();
    tree.children()
        .filter(|n| n.has_tag_name("Spec"))
        .nth(active - 1)
        .unwrap()
        .attribute("nodes")
        .unwrap()
        .split(',')
        .map(|v| v.parse().unwrap())
        .collect()
}

#[test]
fn command_damage_receipt_projection_keeps_consumer_and_source_fields() {
    let context = json!({"output":{"TotalDPS":17},"allocated_nodes":[25927],
        "recipients":[{"source":{"group":3,"gem":1},"original_offence_calls":[{
            "output":{"TotalDPS":17},"stat_set_index":2,"cfg":{"flags":1},
            "parent_records":[{"record":{"value":20,"source":"Tree:25927"}}],
            "original_damage_calls":[{"inc":1.55,"diagnostic":{"sum":55}}]}]}]});
    let report = json!({"cases":[{"state":{"main":context,"calcs":context}}]});
    let projected = evidence::project_report(&report);
    assert!(projected.pointer("/cases/0/state/main/output").is_none());
    assert!(
        projected
            .pointer("/cases/0/state/main/recipients/0/original_offence_calls/0/output")
            .is_none()
    );
    for path in [
        "/cases/0/state/main/allocated_nodes/0",
        "/cases/0/state/main/recipients/0/source/group",
        "/cases/0/state/main/recipients/0/original_offence_calls/0/stat_set_index",
        "/cases/0/state/main/recipients/0/original_offence_calls/0/cfg/flags",
        "/cases/0/state/main/recipients/0/original_offence_calls/0/parent_records/0/record/value",
        "/cases/0/state/main/recipients/0/original_offence_calls/0/original_damage_calls/0/inc",
        "/cases/0/state/main/recipients/0/original_offence_calls/0/original_damage_calls/0/diagnostic/sum",
    ] {
        let mut changed = report.clone();
        *changed.pointer_mut(path).unwrap() = json!(999);
        assert_ne!(evidence::project_report(&changed), projected, "lost {path}");
    }
}

#[test]
#[ignore = "requires complete pinned PoB; actual Command Damage consumer evidence"]
fn command_damage_observes_original_consumers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = std::env::var_os("POE_COMMAND_DAMAGE_SOURCE_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("runs/owned-command-damage-source-01"));
    // Child execution uses PoB's src directory; resolve caller paths before the
    // spawn so both processes write to the same fresh repository artifact path.
    let out = if out.is_absolute() {
        out
    } else {
        root.join(out)
    };
    if let Some(mode) = std::env::var_os(CHILD_ENV) {
        assert!(mode == "off" || mode == "on");
        run(&root, &out, mode == "on");
        return;
    }
    assert!(!out.exists(), "fresh output required: {}", out.display());
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST_NAME, "--ignored", "--nocapture"])
            .env(CHILD_ENV, mode)
            .env("POE_COMMAND_DAMAGE_SOURCE_OUT", &out)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "{}\n{}", path.display(), tail(&path));
                break;
            }
            if start.elapsed() > Duration::from_secs(900) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("deadline {}\n{}", path.display(), tail(&path));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    json_evidence::assert_files_equal(
        &out.join("source-jit-off.json"),
        &out.join("source-jit-on.json"),
        "Command Damage JIT evidence",
    );
}

fn run(root: &Path, out: &Path, jit: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let fixture = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let original = fs::read_to_string(&fixture).unwrap();
    let hash = digest(original.as_bytes());
    assert_eq!(
        hash,
        "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089"
    );
    assert_eq!(
        read(&fixture.parent().unwrap().join("index.json"))["builds"][4]["xml_sha256"],
        hash
    );
    let mut observations = vec![];
    for c in cases(&original) {
        eprintln!("Command Damage source {}", c.name);
        let before = |lua: &Lua| {
            lua.globals().set("commandDamageJit", jit)?;
            lua.load("if commandDamageJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| {
            lua.globals().set("commandDamagePhase", "before")?;
            Ok(lua
                .load(LUA)
                .set_name("@command-damage-observer")
                .eval::<Function>()?)
        };
        let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
            lua.globals().set("commandDamagePhase", "after")?;
            let value: Value = lua.load(LUA).set_name("@command-damage-observer").eval()?;
            Ok(lua.from_value(value)?)
        };
        let temp = tempfile::tempdir().unwrap();
        let result = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            temp.path(),
            &c.xml,
            None,
            c.child != 0,
            Some(&before),
            Some(&install),
            Some(&observe),
        )
        .unwrap_or_else(|e| panic!("{}: {e}", c.name));
        assert_eq!(result["configuration_method_wrappers"], false);
        assert_eq!(result["original_build_output_available"], true);
        let reviewed_sources_removed = c.reviewed_sources_removed();
        observations.push(json!({"name":c.name,"xml_sha256":digest(c.xml.as_bytes()),"removed":c.removed,
            "reviewed_sources_removed":reviewed_sources_removed,"child":c.child,"stat_set":c.set,"state":result["additional_observation"]}));
    }
    let files: Vec<_> = FILES
        .iter()
        .copied()
        .chain(["src/Classes/PassiveTree.lua"])
        .map(|path| json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))
        .collect();
    let report = json!({"schema_version":1,"manifest_sha256":pinned::manifest_sha256(),"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4",
        "files":files,"observer_sha256":digest(LUA.as_bytes()),"original_xml_sha256":hash,"case_count":observations.len(),
        "lifecycle_sha256":digest(include_bytes!("configuration_preparation_source.rs")),"lifecycle":"one-normal-load-per-fresh-vm","numeric_tolerance":0,
        "native_owner_closure":false,"native_build_parity":false,"business_wrappers":false,
        "cases":observations});
    let bytes = serde_json::to_vec(&report).unwrap();
    let mode = if jit { "on" } else { "off" };
    fs::write(out.join(format!("source-jit-{mode}.raw.json")), &bytes).unwrap();
    assert!(
        bytes.len() <= 64 * 1024 * 1024,
        "raw report {} bytes",
        bytes.len()
    );
    fs::write(out.join(format!("source-jit-{mode}.json")), &bytes).unwrap();
    evidence::check_report(&report);
    assert_eq!(fs::read_to_string(fixture).unwrap(), original);
}
