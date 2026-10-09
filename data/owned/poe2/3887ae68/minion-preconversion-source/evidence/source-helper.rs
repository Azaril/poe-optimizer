//! Focused original pre-conversion calls; raw inventories are not native input defaults.
use super::*;
#[path = "minion_preconversion_evidence.rs"]
mod evidence;

const TEST: &str = "preconversion::original_preconversion_inputs_are_complete_and_stable";
const CHILD: &str = "POE_MINION_PRECONVERSION_SOURCE_CHILD";
const OUTPUT: &str = "POE_MINION_PRECONVERSION_SOURCE_OUT";

fn config_input(xml: &str, value: bool) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let config = doc
        .descendants()
        .find(|n| n.has_tag_name("Config"))
        .unwrap();
    let active = config.attribute("activeConfigSet").unwrap();
    let set = config
        .children()
        .find(|n| n.has_tag_name("ConfigSet") && n.attribute("id") == Some(active))
        .unwrap();
    let mut edits: Vec<_> = set
        .children()
        .filter(|n| {
            n.has_tag_name("Input") && n.attribute("name") == Some("minionsConditionFullLife")
        })
        .map(|n| (n.range(), String::new()))
        .collect();
    let end = set.range().end - "</ConfigSet>".len();
    edits.push((
        end..end,
        format!("<Input name=\"minionsConditionFullLife\" boolean=\"{value}\"/>"),
    ));
    edits.sort_by_key(|(range, _)| range.start);
    let mut changed = xml.to_owned();
    for (range, text) in edits.into_iter().rev() {
        changed.replace_range(range, &text);
    }
    changed
}

fn cases(original: &str) -> Vec<Case> {
    let flat = custom(original, "Minions deal 3 to 7 additional Physical Damage");
    let conditional = custom(
        original,
        "Minions deal 3 to 7 additional Physical Damage while on Full Life",
    );
    vec![
        Case {
            name: "original-05".into(),
            xml: original.into(),
            warm: None,
            original: true,
        },
        Case {
            name: "repeat-original-05".into(),
            xml: original.into(),
            warm: None,
            original: true,
        },
        Case {
            name: "warm-flat-to-original".into(),
            xml: original.into(),
            warm: Some(flat.clone()),
            original: true,
        },
        Case {
            name: "sniper-calcs-effective".into(),
            xml: calcs_input(
                &calcs_input(original, "skill_number", "number", "3"),
                "misc_buffMode",
                "string",
                "EFFECTIVE",
            ),
            warm: None,
            original: false,
        },
        Case {
            name: "flat-physical".into(),
            xml: flat,
            warm: None,
            original: false,
        },
        Case {
            name: "zero-flat-physical".into(),
            xml: custom(original, "Minions deal 0 to 0 additional Physical Damage"),
            warm: None,
            original: false,
        },
        Case {
            name: "conditional-flat-disabled".into(),
            xml: config_input(&conditional, false),
            warm: None,
            original: false,
        },
        Case {
            name: "conditional-flat-enabled".into(),
            xml: config_input(&conditional, true),
            warm: None,
            original: false,
        },
    ]
}

#[test]
fn focused_controls_are_distinct_and_keep_the_saved_build_selected() {
    let original = include_str!("../../../../tests/fixtures/builds/breadth-20260908/build-05.xml");
    let cases = cases(original);
    assert_eq!(cases.len(), 8);
    for case in &cases[..3] {
        assert_eq!(case.xml, original);
    }
    assert_eq!(cases[2].warm.as_ref().unwrap(), &cases[4].xml);
    let hashes: std::collections::BTreeSet<_> = cases
        .iter()
        .skip(3)
        .map(|c| digest(c.xml.as_bytes()))
        .collect();
    assert_eq!(hashes.len(), 5);
    for (case, value) in cases[6..].iter().zip(["false", "true"]) {
        let doc = roxmltree::Document::parse(&case.xml).unwrap();
        let config = doc
            .descendants()
            .find(|n| n.has_tag_name("Config"))
            .unwrap();
        let set = config
            .children()
            .find(|n| {
                n.has_tag_name("ConfigSet")
                    && n.attribute("id") == config.attribute("activeConfigSet")
            })
            .unwrap();
        let inputs: Vec<_> = set
            .children()
            .filter(|n| {
                n.has_tag_name("Input") && n.attribute("name") == Some("minionsConditionFullLife")
            })
            .collect();
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].attribute("boolean"), Some(value));
        assert!(set.children().any(|n| n.has_tag_name("CustomModifierBlock")
            && n.text()
                == Some("Minions deal 3 to 7 additional Physical Damage while on Full Life")));
    }
}

#[test]
#[ignore = "requires complete pinned PoB runtime; original pre-conversion source census"]
fn original_preconversion_inputs_are_complete_and_stable() {
    run_life_source_modes(
        TEST,
        CHILD,
        OUTPUT,
        "runs/owned-minion-preconversion-source-01",
        run_child,
    );
}

#[test]
#[ignore = "source-free semantic validation of an explicitly supplied retained acquisition report"]
fn retained_acquisition_passes_semantic_checks() {
    let path = PathBuf::from(
        std::env::var_os("POE_MINION_PRECONVERSION_RETAINED_REPORT")
            .expect("explicit retained report"),
    );
    let path = if path.is_absolute() {
        path
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    };
    check(&serde_json::from_slice(&fs::read(path).unwrap()).unwrap());
}

fn run_child(root: &Path, out: &Path, enabled: bool) {
    let original =
        fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let index = read(&root.join("tests/fixtures/builds/breadth-20260908/index.json"));
    assert_eq!(
        rows(&index["builds"])
            .iter()
            .find(|r| r["xml"] == "build-05.xml")
            .unwrap()["xml_sha256"],
        digest(original.as_bytes())
    );
    let mut observations = Vec::new();
    fs::create_dir_all(out.join("inputs")).unwrap();
    fs::create_dir_all(out.join("raw-observations")).unwrap();
    for case in cases(&original) {
        eprintln!("Pre-conversion source case {}", case.name);
        fs::write(
            out.join("inputs").join(format!("{}.xml", case.name)),
            &case.xml,
        )
        .unwrap();
        let setup = |lua: &Lua, jit: bool| {
            lua.globals()
                .set("physicalDamagePreconversionEvidence", true)?;
            lua.globals().set("physicalDamageJit", jit)?;
            lua.load("if physicalDamageJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let before = |lua: &Lua| setup(lua, false);
        let before_plain = |lua: &Lua| setup(lua, enabled);
        let install = |lua: &Lua| {
            lua.globals().set("physicalDamagePhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@preconversion-source-hook")
                .eval::<Function>()?)
        };
        let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
            assert!(!lua.load("return jit.status()").eval::<bool>()?);
            lua.globals().set("physicalDamagePhase", "after")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@preconversion-source-observe")
                .eval()?;
            Ok(lua.from_value(value)?)
        };
        let snapshot = |lua: &Lua| -> Result<Json, RuntimeError> {
            assert_eq!(lua.load("return jit.status()").eval::<bool>()?, enabled);
            lua.globals()
                .set("physicalDamagePhase", "preconversion_snapshot")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@preconversion-source-unhooked")
                .eval()?;
            Ok(lua.from_value(value)?)
        };
        let scratch = tempfile::tempdir().unwrap();
        let observed = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &case.xml,
            case.warm.as_deref(),
            !case.original,
            Some(&before),
            Some(&install),
            Some(&observe),
        )
        .unwrap_or_else(|e| panic!("{}: {e}", case.name));
        let scratch = tempfile::tempdir().unwrap();
        let plain = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &case.xml,
            case.warm.as_deref(),
            !case.original,
            Some(&before_plain),
            None,
            Some(&snapshot),
        )
        .unwrap_or_else(|e| panic!("{} plain: {e}", case.name));
        // Preserve both complete observations before any comparison can fail.
        // No diagnostic inventory order is normalized by this witness.
        for (kind, result) in [("observed", &observed), ("plain", &plain)] {
            fs::write(
                out.join("raw-observations").join(format!(
                    "{}-jit-{}-{kind}.json",
                    case.name,
                    if enabled { "on" } else { "off" }
                )),
                serde_json::to_vec(result).unwrap(),
            )
            .unwrap();
        }
        for result in [&observed, &plain] {
            assert_eq!(result["configuration_method_wrappers"], false);
            assert_eq!(result["original_build_output_available"], true);
            assert_eq!(result["source_hash"], pinned::manifest_sha256());
        }
        assert_eq!(
            json_evidence::first_difference(
                &observed["additional_observation"]["preconversion_snapshot"],
                &plain["additional_observation"],
                "unhooked"
            ),
            None,
            "{}",
            case.name
        );
        observations.push(json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),"warm_xml_sha256":case.warm.as_ref().map(|x|digest(x.as_bytes())),"state":observed["additional_observation"],"unhooked":plain["additional_observation"]}));
        fs::write(
            out.join(format!(
                "source-jit-{}-progress.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec(&observations).unwrap(),
        )
        .unwrap();
    }
    let mut report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":digest(OBSERVE.as_bytes()),"driver_sha256":digest(include_bytes!("../owned_minion_physical_damage_source.rs")),
        "source_helper_sha256":digest(include_bytes!("minion_preconversion_source.rs")),"bootstrap_sha256":digest(include_bytes!("configuration_preparation_source.rs")),
        "files":FILES.iter().map(|p|json!({"path":p,"sha256":pinned::expected_file_sha256(p).unwrap()})).collect::<Vec<_>>(),
        "original_xml_sha256":digest(original.as_bytes()),"complete_load_attempts_per_jit":18,
        "capture":{"original_base_locals":true,"raw_zero_and_conditional_records":true,"original_pass_assembly":true,"business_methods_replaced":false,"observer_jit_enabled":false,"unhooked_jit_mode_from_filename":true,"unhooked_controls":true,"copied_formula_as_evidence":false},
        "scope":{"native_coverage":false,"whole_build_parity":false,"conditional_game_obtainability":false,"later_damage_pipeline":false},
        "presentation_canonicalization":"Existing stable_state buff multisets only; every new preconversion inventory and original numerical/source operation remains ordered and exact. Complete raw observations are retained separately.","cases":observations});
    fs::write(
        out.join(format!(
            "source-jit-{}-raw.json",
            if enabled { "on" } else { "off" }
        )),
        serde_json::to_vec(&report).unwrap(),
    )
    .unwrap();
    for case in report["cases"].as_array_mut().unwrap() {
        case["state"] = stable_state(&case["state"]);
    }
    let path = out.join(format!(
        "source-jit-{}.json",
        if enabled { "on" } else { "off" }
    ));
    fs::write(&path, serde_json::to_vec(&report).unwrap()).unwrap();
    check(&report);
}

fn raw<'a>(values: &'a Json, name: &str) -> Vec<&'a Json> {
    rows(values)
        .iter()
        .filter(|r| r["mod"]["name"] == name)
        .collect()
}

fn check_coefficient(census: &Json) {
    let coefficient = &census["coefficient"];
    assert_eq!(coefficient["level"]["present"], false);
    assert_eq!(coefficient["skill_data"]["present"], false);
    close(number(&coefficient["original_selected"]), 1.0);
    for values in [
        &census["raw"]["environment_skill_data"],
        &census["raw"]["skill"],
    ] {
        assert!(
            raw(values, "SkillData")
                .iter()
                .all(|r| r["mod"]["value"]["key"] != "baseMultiplier")
        );
    }
    for key in ["environment_eligible", "skill_eligible"] {
        assert!(
            rows(&coefficient[key])
                .iter()
                .all(|r| r["mod"]["value"]["key"] != "baseMultiplier")
        );
    }
}

#[test]
fn coefficient_guard_rejects_raw_zero_inactive_and_supplied_identity_sources() {
    let empty = json!({"coefficient":{"level":{"present":false},"skill_data":{"present":false},"original_selected":1.0,"environment_eligible":[],"skill_eligible":[]},"raw":{"environment_skill_data":[],"skill":[]}});
    check_coefficient(&empty);
    for field in ["level", "skill_data"] {
        let mut changed = empty.clone();
        changed["coefficient"][field] = json!({"present":true,"value":1.0});
        assert!(std::panic::catch_unwind(|| check_coefficient(&changed)).is_err());
    }
    for domain in ["environment_skill_data", "skill"] {
        for value in [0.0, 1.0] {
            let mut changed = empty.clone();
            changed["raw"][domain] = json!([{"ancestor_depth":0,"mod":{"name":"SkillData","type":"LIST","value":{"key":"baseMultiplier","value":value},"tags":[{"type":"Condition","var":"UnknownCondition"}]}}]);
            assert!(std::panic::catch_unwind(|| check_coefficient(&changed)).is_err());
        }
    }
    for domain in ["environment_eligible", "skill_eligible"] {
        let mut changed = empty.clone();
        changed["coefficient"][domain] =
            json!([{"mod":{"value":{"key":"baseMultiplier","value":1.0}}}]);
        assert!(std::panic::catch_unwind(|| check_coefficient(&changed)).is_err());
    }
}

pub(super) fn check(report: &Json) {
    assert_eq!(report["complete_load_attempts_per_jit"], 18);
    assert_eq!(report["capture"]["observer_jit_enabled"], false);
    assert_eq!(report["capture"]["business_methods_replaced"], false);
    assert_eq!(report["capture"]["copied_formula_as_evidence"], false);
    assert_eq!(report["scope"]["native_coverage"], false);
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 8);
    assert_eq!(cases[2]["warm_xml_sha256"], cases[4]["xml_sha256"]);
    for (index, case) in cases.iter().enumerate() {
        if index < 3 {
            assert_eq!(case["xml_sha256"], report["original_xml_sha256"]);
        }
        assert_eq!(case["state"]["preconversion_snapshot"], case["unhooked"]);
        for mode in ["main", "calcs"] {
            let children: Vec<_> = rows(&case["state"][mode]["actors"])
                .iter()
                .filter(|a| a["actor_profile"] == "RaisedSkeletonSniper")
                .collect();
            assert_eq!(children.len(), 1);
            let actor = children[0];
            let skills: Vec<_> = rows(&actor["children"])
                .iter()
                .filter(|s| s["effect_id"] == "MinionMeleeBow")
                .collect();
            assert_eq!(skills.len(), 1);
            let child = skills[0];
            if mode == "calcs" && index != 3 {
                assert_eq!(actor["is_environment_minion"], false);
                for field in ["base_calls", "damage_calls", "passes"] {
                    assert!(
                        child.as_object().unwrap().get(field).is_none(),
                        "unobserved {field} remains absent"
                    );
                }
                continue;
            }
            assert_eq!(actor["is_environment_minion"], true);
            let calls: Vec<_> = rows(&child["base_calls"])
                .iter()
                .filter(|c| c["damage_type"] == "Physical")
                .collect();
            assert_eq!(calls.len(), 1);
            for call in calls {
                assert_eq!(call["observed_at"], 4137);
                assert_eq!(call["query_state_preserved"], true);
                let census = &call["preconversion"];
                assert_eq!(census["actor_profile"], "RaisedSkeletonSniper");
                for field in [
                    "exact_actor_store",
                    "exact_enemy_store",
                    "exact_parent",
                    "exact_summoner",
                    "exact_pass_source",
                    "exact_pass_cfg",
                    "channel_queries_are_supplemental",
                ] {
                    assert_eq!(census[field], true, "{field}");
                }
                assert_eq!(census["source_occurrence"], actor["source_occurrence"]);
                assert_eq!(census["pass_label"], "Main Hand");
                let selected = &census["selection"];
                assert_eq!(selected["line"], 2543);
                assert_eq!(selected["kind"], "weapon1");
                assert_eq!(selected["is_attack"], true);
                assert_eq!(selected["unarmed"], false);
                assert_eq!(selected["weapon1_attack"], true);
                assert_eq!(selected["weapon2_attack"], false);
                assert_eq!(selected["weapon_present"], true);
                assert_eq!(selected["source_is_weapon_object"], false);
                assert_eq!(selected["source_is_skill_data"], false);
                assert!(selected["events"].as_u64().unwrap() >= 1);
                assert_eq!(selected["source"], call["source"]);
                let endpoints = &census["endpoints"];
                for (name, value) in [("minimum", 208.0), ("maximum", 387.0)] {
                    assert_eq!(endpoints[name]["present"], true);
                    close(number(&endpoints[name]["value"]), value);
                }
                for name in ["bonus_minimum", "bonus_maximum"] {
                    assert_eq!(endpoints[name]["present"], false);
                }
                check_coefficient(census);
                assert!(rows(&census["raw"]["enemy"]).is_empty());
                for key in ["enemy_minimum", "enemy_maximum", "added_increased"] {
                    close(number(&call[key]["value"]), 0.0);
                    assert!(rows(&call[key]["records"]).is_empty());
                }
                close(number(&call["added_multiplier"]), 1.15);
                let more = raw(&census["raw"]["skill"], "AddedDamage");
                assert_eq!(more.len(), 1);
                assert_eq!(more[0]["mod"]["type"], "MORE");
                assert_eq!(
                    more[0]["mod"]["source"],
                    "Skeletal Sniper Damage Multiplier"
                );
                assert_eq!(number(&more[0]["mod"]["value"]), 14.999999999999991);
                assert!(raw(&census["raw"]["skill"], "AddedPhysicalDamage").is_empty());
                let supplied = index >= 4;
                let active = index == 4 || index == 7;
                for (name, key, value) in [
                    ("PhysicalMin", "minimum", 3.0),
                    ("PhysicalMax", "maximum", 7.0),
                ] {
                    let source = raw(&census["raw"]["skill"], name);
                    assert_eq!(source.len(), usize::from(supplied));
                    if supplied {
                        assert_eq!(
                            source[0]["mod"]["source"],
                            "Custom:Physical damage source control"
                        );
                        assert_eq!(source[0]["mod"]["type"], "BASE");
                        close(
                            number(&source[0]["mod"]["value"]),
                            if index == 5 { 0.0 } else { value },
                        );
                        if index >= 6 {
                            let tags = rows(&source[0]["mod"]["tags"]);
                            assert!(
                                tags.iter()
                                    .any(|t| t["type"] == "Condition" && t["var"] == "FullLife")
                            );
                        } else {
                            assert_eq!(source[0]["mod"]["tags"], json!({}));
                        }
                    }
                    assert_eq!(rows(&call[key]["records"]).len(), usize::from(active));
                    close(
                        number(&call[key]["value"]),
                        if active { value } else { 0.0 },
                    );
                }
                close(
                    number(&call["base_min"]),
                    if active { 211.45 } else { 208.0 },
                );
                close(
                    number(&call["base_max"]),
                    if active { 395.05 } else { 387.0 },
                );
            }
        }
    }
    for index in [1, 2] {
        assert_eq!(
            json_evidence::first_difference(
                &stable_state(&cases[0]["state"]),
                &stable_state(&cases[index]["state"]),
                "repeated-state"
            ),
            None,
            "{}",
            cases[index]["name"]
        );
    }
}
