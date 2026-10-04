//! Optional generated-only source consumer evidence, sharing the full lifecycle.
use super::*;

const TEST: &str =
    "generated_skill_usage::complete_generated_skill_usage_preserves_source_consumers";
const CHILD: &str = "POE_GENERATED_SKILL_USAGE_SOURCE_CHILD";
const GENERATED: [(&str, &str, &str); 3] = [
    ("sand", "Tree:13289", "SummonSandDjinnPlayer"),
    ("water", "Tree:32705", "SummonWaterDjinnPlayer"),
    ("firebolt", STAFF_SOURCE, FIREBOLT),
];

#[test]
#[ignore = "requires complete pinned PoB; generated usage evidence does not certify native inventory or numerical parity"]
fn complete_generated_skill_usage_preserves_source_consumers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-generated-skill-usage-source-01");
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
                assert!(status.success(), "{}\n{}", path.display(), tail(&path));
                break;
            }
            if started.elapsed() > Duration::from_secs(300) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "generated usage deadline: {}\n{}",
                    path.display(),
                    tail(&path)
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert!(
        fs::read(out.join("source-jit-off.json")).unwrap()
            == fs::read(out.join("source-jit-on.json")).unwrap(),
        "generated source evidence must be byte-identical across JIT modes"
    );
}

fn stage(lua: &Lua) -> Result<Json, RuntimeError> {
    let mut base = super::observe_stage(lua)?;
    lua.globals()
        .set("generatedUsageBase", lua.to_value(&base)?)?;
    let value: Value = lua
        .load(include_str!("generated_skill_usage.lua"))
        .set_name("@exact-generated-source-usage-observer")
        .eval()?;
    let usage: Json = lua.from_value(value)?;
    let after = super::observe_stage(lua)?;
    assert!(
        base == after,
        "usage observer changed source ownership/state"
    );
    base["generated_usage"] = usage;
    Ok(base)
}

fn observe(root: &Path, name: &str, xml: &str, enabled: bool) -> Json {
    observe_case_with_stage(root, name, xml, enabled, &stage)
}

fn run_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let directory = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json =
        serde_json::from_slice(&fs::read(directory.join("index.json")).unwrap()).unwrap();
    let originals: Vec<_> = (1..=5)
        .map(|i| fs::read(directory.join(format!("build-{i:02}.xml"))).unwrap())
        .collect();
    let mut cases = vec![];
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(
            digest(bytes),
            index["builds"][i]["xml_sha256"].as_str().unwrap()
        );
        let xml = std::str::from_utf8(bytes).unwrap();
        cases.push(observe(
            root,
            &format!("original-{:02}", i + 1),
            xml,
            enabled,
        ));
        let doc = roxmltree::Document::parse(xml).unwrap();
        let skills = doc
            .descendants()
            .find(|n| n.has_tag_name("Skills"))
            .unwrap();
        for set in skills.children().filter(|n| n.has_tag_name("SkillSet")) {
            let id = set.attribute("id").unwrap();
            if skills.attribute("activeSkillSet") == Some(id) {
                continue;
            }
            let changed = change_attributes(xml, skills, &[("activeSkillSet", Some(id))]);
            cases.push(observe(
                root,
                &format!("original-{:02}-activate-preset-{id}", i + 1),
                &changed,
                enabled,
            ));
        }
    }
    let xml = std::str::from_utf8(&originals[4]).unwrap();
    for (name, changed) in controls(xml) {
        assert_ne!(xml, changed);
        cases.push(observe(root, &name, &changed, enabled));
    }
    cases.push(observe(root, "repeat-original-05", xml, enabled));
    let files = [
        "src/HeadlessWrapper.lua",
        "src/Modules/Build.lua",
        "src/Classes/CalcsTab.lua",
        "src/Classes/SkillsTab.lua",
        "src/Classes/ItemsTab.lua",
        "src/Classes/Item.lua",
        "src/Classes/PassiveSpec.lua",
        "src/Modules/Data.lua",
        "src/Modules/CalcSetup.lua",
        "src/Modules/CalcActiveSkill.lua",
        "src/Modules/CalcDefence.lua",
        "src/Modules/Calcs.lua",
        "src/Modules/CalcPerform.lua",
        "src/Data/Gems.lua",
        "src/Data/Skills/act_int.lua",
    ];
    let report = json!({
        "source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "business_wrappers":false,"native_inventory_authority":false,"native_build_parity":false,
        "canonical_parity_lifecycle_selected":false,"call_boundary_reporting_attribution":false,
        "native_defaults_authorized":false,
        "lifecycle_stages":["fresh_complete_load","requested_original_frame_rebuild_1","requested_original_frame_rebuild_2"],
        "files":files.map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})),
        "original_sources":originals.iter().enumerate().map(|(i,bytes)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(bytes)})).collect::<Vec<_>>(),
        "reviewed_generated_sources":GENERATED.map(|(family,source,skill)|json!({"family":family,"source":source,"skill_id":skill})),
        "cases":cases,
    });
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(
        bytes.len() <= 64 * 1024 * 1024,
        "generated report {} bytes exceeds64MiB",
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
            &fs::read(directory.join(format!("build-{:02}.xml", i + 1))).unwrap(),
            bytes
        );
    }
}

fn group<'a, 'input>(
    doc: &'a roxmltree::Document<'input>,
    source: &str,
) -> roxmltree::Node<'a, 'input> {
    let skills = doc
        .descendants()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let selected = skills.attribute("activeSkillSet").unwrap();
    skills
        .children()
        .find(|n| n.has_tag_name("SkillSet") && n.attribute("id") == Some(selected))
        .unwrap()
        .children()
        .find(|n| n.has_tag_name("Skill") && n.attribute("source") == Some(source))
        .unwrap()
}

fn controls(xml: &str) -> Vec<(String, String)> {
    let mut result = vec![];
    for (family, source, _) in GENERATED {
        for (name, field, value, is_group) in [
            ("disabled-source", "enabled", "false", false),
            ("disabled-group", "enabled", "false", true),
            ("count-three", "count", "3", false),
            ("global-one-false", "enableGlobal1", "false", false),
            ("global-two-true", "enableGlobal2", "true", false),
            ("quality-fraction", "quality", "12.5", false),
        ] {
            let doc = roxmltree::Document::parse(xml).unwrap();
            let g = group(&doc, source);
            let node = if is_group {
                g
            } else {
                g.children().find(|n| n.has_tag_name("Gem")).unwrap()
            };
            result.push((
                format!("{family}-{name}"),
                change_attributes(xml, node, &[(field, Some(value))]),
            ));
        }
        for (name, count, group_field, value) in [
            ("group-zero", "3", "groupCount", "0"),
            ("group-four", "3", "groupCount", "4"),
            ("full-dps-one", "1", "includeInFullDPS", "true"),
            ("full-dps-three", "3", "includeInFullDPS", "true"),
        ] {
            let doc = roxmltree::Document::parse(xml).unwrap();
            let gem = group(&doc, source)
                .children()
                .find(|n| n.has_tag_name("Gem"))
                .unwrap();
            let changed = change_attributes(xml, gem, &[("count", Some(count))]);
            let doc = roxmltree::Document::parse(&changed).unwrap();
            let changed =
                change_attributes(&changed, group(&doc, source), &[(group_field, Some(value))]);
            result.push((format!("{family}-{name}"), changed));
        }
    }
    result
}

fn selected<'a>(state: &'a Json, source: &str) -> &'a Json {
    let matched: Vec<_> = rows(&state["generated_usage"]["groups"])
        .iter()
        .filter(|g| g["selected"] == true && g["source"] == source)
        .collect();
    assert_eq!(matched.len(), 1, "exact selected generated source {source}");
    matched[0]
}

fn check(report: &Json) {
    assert_eq!(rows(&report["cases"]).len(), 46);
    let original = case(report, "original-05");
    assert!(
        original["states"] == case(report, "repeat-original-05")["states"],
        "repeat must restore exact source states"
    );
    for case in rows(&report["cases"]) {
        for stage in STAGES {
            let state = &case["states"][stage];
            let usage = &state["generated_usage"];
            for flag in [
                "source_methods_preserved",
                "outputs_preserved",
                "requested_jit_mode_preserved",
                "observer_removed_before_evaluation",
            ] {
                assert_eq!(usage[flag], true, "{} {stage} {flag}", case["name"]);
            }
            for flag in [
                "full_dps_rows_have_source_identity",
                "reservation_breakdown_rows_have_source_identity",
                "call_boundary_attribution",
            ] {
                assert_eq!(usage[flag], false);
            }
            for g in rows(&usage["groups"]) {
                assert_eq!(rows(&g["sources"]).len(), 1);
                for mode in ["MAIN", "CALCS"] {
                    for action in rows(&g[mode]) {
                        assert_eq!(g["selected"], true);
                        assert_eq!(action["exact_group"], true);
                        assert_eq!(action["exact_source_object"], true);
                        assert!(action["helper_count"].as_f64().is_some_and(f64::is_finite));
                        assert!(action["helper_enabled"].is_boolean());
                        let source = &rows(&g["sources"])
                            [action["source_index"].as_u64().unwrap() as usize - 1];
                        let effect = &rows(&source["effects"])
                            [action["effect_index"].as_u64().unwrap() as usize - 1];
                        assert_eq!(effect["id"], action["effect"]);
                        assert_eq!(effect["global_field"], action["global_field"]);
                        assert_eq!(effect["global_value"], action["global_value"]);
                    }
                }
            }
        }
    }
    for (family, source, skill) in GENERATED {
        let expected_effects: &[&str] = match family {
            "sand" => &["CommandSandDjinnKnifeThrowPlayer", "SummonSandDjinnPlayer"],
            "water" => &["CommandWaterDjinnBubblePlayer", "SummonWaterDjinnPlayer"],
            "firebolt" => &["FireboltPlayer"],
            _ => unreachable!(),
        };
        for stage in STAGES {
            let base = selected(&original["states"][stage], source);
            let instance = &rows(&base["sources"])[0];
            assert_eq!(instance["fields"]["skillId"], skill);
            assert_eq!(instance["saved_object_present"], true);
            assert_eq!(instance["fields"]["count"], 1);
            assert_eq!(instance["fields"]["enableGlobal1"], true);
            assert_eq!(instance["fields"]["enableGlobal2"], false);
            assert_eq!(base["group_fields"]["includeInFullDPS"], false);
            for mode in ["MAIN", "CALCS"] {
                assert!(!rows(&base[mode]).is_empty());
                for action in rows(&base[mode]) {
                    assert_eq!(action["helper_count"], 1);
                    assert_eq!(action["helper_enabled"], true);
                }
            }
            for control in [
                "disabled-source",
                "disabled-group",
                "count-three",
                "global-one-false",
                "global-two-true",
                "quality-fraction",
                "group-zero",
                "group-four",
                "full-dps-one",
                "full-dps-three",
            ] {
                let c = case(report, &format!("{family}-{control}"));
                let state = &c["states"][stage];
                let current = selected(state, source);
                let fields = &rows(&current["sources"])[0]["fields"];
                assert_eq!(
                    rows(&current["sources"])[0]["source_ordinal"],
                    instance["source_ordinal"]
                );
                assert_eq!(rows(&current["sources"])[0]["saved_object_present"], true);
                assert_eq!(current["source_ordinal"], base["source_ordinal"]);
                // Only the exact generated source/group was edited. All manual
                // source attributes retain their original evidence and ownership.
                for saved in rows(&state["saved_groups"])
                    .iter()
                    .filter(|r| r["attributes"]["source"].is_null())
                {
                    let old = rows(&original["states"][stage]["saved_groups"])
                        .iter()
                        .find(|r| r["source_ordinal"] == saved["source_ordinal"])
                        .unwrap();
                    assert!(
                        saved["attributes"] == old["attributes"],
                        "manual group changed in {family}-{control}"
                    );
                    let gems = rows(&saved["gems"]);
                    let old_gems = rows(&old["gems"]);
                    assert_eq!(gems.len(), old_gems.len());
                    for (gem, old_gem) in gems.iter().zip(old_gems) {
                        assert!(gem["source"] == old_gem["source"], "manual source changed");
                    }
                }
                match control {
                    "disabled-source" => assert_eq!(fields["enabled"], false),
                    "disabled-group" => assert_eq!(current["group_fields"]["enabled"], false),
                    "global-one-false" => assert_eq!(
                        fields["enableGlobal1"], true,
                        "generated update overwrites raw false"
                    ),
                    "global-two-true" => assert_eq!(fields["enableGlobal2"], true),
                    "quality-fraction" => assert_eq!(fields["quality"].as_f64(), Some(12.5)),
                    "group-zero" => assert_eq!(current["group_fields"]["groupCount"], 0),
                    "group-four" => assert_eq!(current["group_fields"]["groupCount"], 4),
                    "full-dps-one" | "full-dps-three" => {
                        assert_eq!(current["group_fields"]["includeInFullDPS"], true)
                    }
                    "count-three" => assert_eq!(fields["count"], 3),
                    _ => unreachable!(),
                }
                for mode in ["MAIN", "CALCS"] {
                    let actions = rows(&current[mode]);
                    if control == "disabled-source" || control == "disabled-group" {
                        assert!(
                            actions.is_empty(),
                            "nonselected disabled generated source {family} {mode}"
                        );
                    } else {
                        assert!(
                            !actions.is_empty(),
                            "source {family}-{control} {stage} {mode}"
                        );
                        if control == "quality-fraction" {
                            let mut effects: Vec<_> = actions
                                .iter()
                                .map(|action| action["effect"].as_str().unwrap())
                                .collect();
                            effects.sort_unstable();
                            assert_eq!(
                                effects, expected_effects,
                                "quality must reach every exact source effect: {family} {stage} {mode}"
                            );
                        }
                        let count = match control {
                            "count-three" | "full-dps-three" => 3,
                            "group-zero" => 0,
                            "group-four" => 4,
                            _ => 1,
                        };
                        for action in actions {
                            if control == "quality-fraction" {
                                assert_eq!(
                                    action["prepared_quality"].as_f64(),
                                    Some(12.5),
                                    "{family} {stage} {mode} {}",
                                    action["effect"]
                                );
                            }
                            assert_eq!(
                                action["helper_count"], count,
                                "{family}-{control} {stage} {mode}"
                            );
                            assert_eq!(action["helper_enabled"], true);
                        }
                    }
                }
            }
        }
    }
}
