//! Optional exact saved Warrior correspondence; this grants no native mechanics.
use super::*;
const OBSERVER: &str = include_str!("warrior_generated_source.lua");
const TEST: &str =
    "warrior_generated_source::actual_warrior_saved_sources_preserve_exact_selector_correspondence";
const OUTPUT: &str = "POE_OPTIMIZER_TEST_WARRIOR_GENERATED_SOURCE_OUT";
const CHILD: &str = "POE_WARRIOR_GENERATED_SOURCE_CHILD";

fn original(root: &Path, number: usize) -> String {
    let directory = root.join("tests/fixtures/builds/breadth-20260908");
    let name = format!("build-{number:02}.xml");
    let index: Json =
        serde_json::from_slice(&fs::read(directory.join("index.json")).unwrap()).unwrap();
    let xml = fs::read_to_string(directory.join(&name)).unwrap();
    let pin = index["builds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["xml"] == name)
        .unwrap();
    assert_eq!(pin["xml_sha256"], digest(xml.as_bytes()));
    xml
}
fn plain_observe(root: &Path, xml: &str, jit: bool) -> Json {
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
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
        let value: Value = lua
            .load(OBSERVER)
            .set_name("@warrior_generated_source.lua")
            .eval()?;
        assert_eq!(lua.load("return jit.status()").eval::<bool>()?, jit);
        Ok(lua.from_value(value)?)
    };
    let scratch = tempfile::tempdir().unwrap();
    let report = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        xml,
        None,
        false,
        Some(&before),
        None,
        Some(&after),
    )
    .unwrap();
    assert_eq!(report["configuration_method_wrappers"], false);
    assert_eq!(report["original_build_output_available"], true);
    report["additional_observation"].clone()
}

fn stage(lua: &Lua) -> Result<Json, RuntimeError> {
    let mut base = super::observe_stage(lua)?;
    lua.globals()
        .set("warriorMembershipBase", lua.to_value(&base)?)?;
    let value: Value = lua
        .load(OBSERVER)
        .set_name("@warrior_generated_source.lua")
        .eval()?;
    let observation: Json = lua.from_value(value)?;
    assert_eq!(base, super::observe_stage(lua)?);
    base["warrior"] = observation;
    Ok(base)
}
fn edit(xml: &str, tag: &str, attribute: &str, value: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let node = doc.descendants().find(|n| n.has_tag_name(tag)).unwrap();
    change_attributes(xml, node, &[(attribute, Some(value))])
}
fn change_warrior(xml: &str, attribute: &str, value: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let set = doc
        .descendants()
        .find(|n| n.has_tag_name("SkillSet") && n.attribute("id") == Some("3"))
        .unwrap();
    let node = set
        .descendants()
        .find(|n| {
            n.has_tag_name("Gem") && n.attribute("skillId") == Some("SummonSkeletalWarriorsPlayer")
        })
        .unwrap();
    change_attributes(xml, node, &[(attribute, Some(value))])
}
fn cases(xml: &str) -> Vec<(&'static str, String)> {
    let selected = edit(
        &edit(xml, "Skills", "activeSkillSet", "3"),
        "Items",
        "activeItemSet",
        "3",
    );
    vec![
        ("original-05", xml.into()),
        ("original-repeat", xml.into()),
        ("selected-warrior", selected.clone()),
        (
            "selected-fractional-quality",
            change_warrior(&selected, "quality", "12.5"),
        ),
        (
            "selected-foreign-minion",
            change_warrior(&selected, "skillMinion", "SandDjinn"),
        ),
        (
            "selected-child-two",
            change_warrior(&selected, "skillMinionSkill", "2"),
        ),
        ("archived-changed-level", change_warrior(xml, "level", "10")),
    ]
}
fn snapshot(mut v: Json) -> Json {
    v.as_object_mut().unwrap().remove("saved");
    v
}
fn child(root: &Path, out: &Path, jit: bool) {
    let original_xml = original(root, 5);
    let mut observations = Vec::new();
    let mut cases = cases(&original_xml);
    cases.push(("original-01", original(root, 1)));
    for (name, xml) in cases {
        let observed = observe_case_with_stage(root, name, &xml, jit, &stage);
        let uninstrumented = plain_observe(root, &xml, jit);
        let fresh = &observed["states"]["fresh"]["warrior"];
        assert_eq!(
            snapshot(fresh.clone()),
            uninstrumented,
            "observer equality {name}"
        );
        for phase in STAGES {
            let state = &observed["states"][phase];
            assert_eq!(state["exact_loader_capture"], true);
            assert_eq!(state["observer_removed_before_evaluation"], true);
            assert_eq!(state["business_wrappers"], false);
            let saved = rows(&state["warrior"]["saved"]);
            assert_eq!(saved.len(), if name == "original-01" { 2 } else { 4 });
            assert!(saved.iter().all(|r| r["loaded_object_exact"] == true));
            for group in rows(&state["warrior"]["groups"]) {
                assert_eq!(group["from_item"], true);
                assert_eq!(group["minion_list"], json!(["RaisedSkeletonWarriors"]));
            }
        }
        // Saved topology is stable across fixed normal rebuilds. Player numerical
        // output is retained, but is not a native numerical admission certificate.
        assert_eq!(
            observed["states"]["rebuilt_once"]["warrior"]["saved"],
            observed["states"]["rebuilt_twice"]["warrior"]["saved"]
        );
        observations.push(json!({"name":name,"xml_sha256":digest(xml.as_bytes()),"observation":observed,"uninstrumented_equal":true}));
    }
    assert_eq!(
        observations[0]["observation"]["states"],
        observations[1]["observation"]["states"]
    );
    let manifest: Json = serde_json::from_slice(
        &fs::read(root.join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap(),
    )
    .unwrap();
    let wanted = [
        "src/Data/Bases/sceptre.lua",
        "src/Data/Skills/act_int.lua",
        "src/Data/Skills/minion.lua",
        "src/Data/Minions.lua",
        "src/Classes/SkillsTab.lua",
        "src/Modules/CalcSetup.lua",
        "src/Modules/CalcActiveSkill.lua",
    ];
    let files: Vec<_> = manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| wanted.contains(&r["path"].as_str().unwrap()))
        .cloned()
        .collect();
    let report = json!({"schema_version":1,"source_revision":manifest["upstream_revision"],"manifest_sha256":pinned::manifest_sha256(),"files":files,"observer_sha256":digest(OBSERVER.as_bytes()),"test_sha256":digest(include_bytes!("warrior_generated_source.rs")),"attempted_full_loads":16,"cases":observations,"scope":{"native_numerical_authority":false,"physical_assignment_authority":false,"complete_usage_authority":false}});
    fs::write(
        out.join(if jit {
            "source-jit-on.json"
        } else {
            "source-jit-off.json"
        }),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "complete optional pinned PoB; exact source correspondence, no native coverage"]
fn actual_warrior_saved_sources_preserve_exact_selector_correspondence() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let output = PathBuf::from(std::env::var_os(OUTPUT).expect("fresh source output"));
    let output = if output.is_absolute() {
        output
    } else {
        root.join(output)
    };
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "on" || mode == "off");
        child(&root, &output, mode == "on");
        return;
    }
    assert!(!output.exists());
    fs::create_dir_all(&output).unwrap();
    for mode in ["off", "on"] {
        let log = fs::File::create(output.join(format!("source-jit-{mode}.log"))).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", TEST, "--nocapture"])
            .env(CHILD, mode)
            .env(OUTPUT, &output)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let started = Instant::now();
        loop {
            if let Some(status) = process.try_wait().unwrap() {
                assert!(status.success(), "source-jit-{mode}.log");
                break;
            }
            if started.elapsed() > Duration::from_secs(600) {
                process.kill().unwrap();
                process.wait().unwrap();
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
#[test]
fn warrior_controls_change_exact_saved_sources_without_rewriting_original() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let original = original(&root, 5);
    let rows = cases(&original);
    assert_eq!(rows.len(), 7);
    assert_eq!(rows[0].1, rows[1].1);
    let unique: std::collections::BTreeSet<_> = rows
        .iter()
        .skip(1)
        .map(|(_, xml)| digest(xml.as_bytes()))
        .collect();
    assert_eq!(unique.len(), 6);
    for (i, (_, xml)) in rows.iter().enumerate() {
        let doc = roxmltree::Document::parse(xml).unwrap();
        assert_eq!(
            doc.descendants()
                .filter(|n| n.has_tag_name("Gem")
                    && n.attribute("skillId") == Some("SummonSkeletalWarriorsPlayer"))
                .count(),
            4
        );
        if (2..=5).contains(&i) {
            assert_eq!(
                doc.descendants()
                    .find(|n| n.has_tag_name("Skills"))
                    .unwrap()
                    .attribute("activeSkillSet"),
                Some("3")
            );
            assert_eq!(
                doc.descendants()
                    .find(|n| n.has_tag_name("Items"))
                    .unwrap()
                    .attribute("activeItemSet"),
                Some("3")
            );
        }
    }
}
