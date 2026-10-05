//! Bounded actual-parser applicability evidence; never a native mechanic or waiver.
use super::*;

const TEST: &str = "generated_extra_skill_stats_source::actual_custom_mod_skill_name_controls";
const CHILD: &str = "POE_GENERATED_EXTRA_STATS_CHILD";
const OUTPUT: &str = "POE_GENERATED_EXTRA_STATS_OUT";
const OBSERVER: &str = include_str!("generated_extra_skill_stats_source.lua");
const TITLE: &str = "ExtraSkillStat witness";
const PURIFYING: &str =
    "Consecrated Ground from Purifying Flame applies 17% increased Damage taken to Enemies";
const LIGHTNING: &str = "19% increased Lightning Trap Lightning Ailment Effect";
const PURIFYING_STAT: &str = "consecrated_ground_enemy_damage_taken_+%";
const LIGHTNING_STAT: &str = "shock_effect_+%";
const EFFECTS: [&str; 5] = [
    "SummonSandDjinnPlayer",
    "CommandSandDjinnKnifeThrowPlayer",
    "SummonWaterDjinnPlayer",
    "CommandWaterDjinnBubblePlayer",
    FIREBOLT,
];
const COMMANDS: [&str; 2] = [
    "CommandSandDjinnKnifeThrowPlayer",
    "CommandWaterDjinnBubblePlayer",
];

fn observe(lua: &Lua, phase: &str) -> Result<Json, RuntimeError> {
    let observer: Function = lua
        .load(OBSERVER)
        .set_name("@actual-extra-stat-source-observer")
        .eval()?;
    let value: Value = observer.call((
        lua.to_value(&EFFECTS)?,
        lua.to_value(&[PURIFYING_STAT, LIGHTNING_STAT])?,
        lua.to_value(&["Purifying Flame", "Lightning Trap", "Firebolt"])?,
        lua.to_value(&[PURIFYING, LIGHTNING])?,
        phase,
    ))?;
    Ok(lua.from_value(value)?)
}
fn custom(xml: &str, line: &str) -> String {
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
    assert!(
        !set.children()
            .any(|n| n.has_tag_name("CustomModifierBlock"))
    );
    let insertion = set.range().end - "</ConfigSet>".len();
    assert_eq!(&xml[insertion..set.range().end], "</ConfigSet>");
    let mut result = xml.to_owned();
    result.insert_str(
        insertion,
        &format!(
            "<CustomModifierBlock title=\"{TITLE}\" enabled=\"true\">{}</CustomModifierBlock>",
            escape(line)
        ),
    );
    result
}
fn second_switch(xml: &str, value: &str) -> String {
    let mut result = xml.to_owned();
    for source in ["Tree:13289", "Tree:32705"] {
        let doc = roxmltree::Document::parse(&result).unwrap();
        let skills = doc
            .descendants()
            .find(|n| n.has_tag_name("Skills"))
            .unwrap();
        let active = skills.attribute("activeSkillSet").unwrap();
        let set = skills
            .children()
            .find(|n| n.has_tag_name("SkillSet") && n.attribute("id") == Some(active))
            .unwrap();
        let matches: Vec<_> = set
            .children()
            .filter(|n| n.has_tag_name("Skill") && n.attribute("source") == Some(source))
            .collect();
        assert_eq!(matches.len(), 1);
        let gem = matches[0]
            .children()
            .find(|n| n.has_tag_name("Gem"))
            .unwrap();
        result = change_attributes(&result, gem, &[("enableGlobal2", Some(value))]);
    }
    result
}

#[test]
#[ignore = "requires complete pinned PoB; bounded parser/applicability evidence only"]
fn actual_custom_mod_skill_name_controls() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join(
        std::env::var_os(OUTPUT)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("runs/owned-generated-extra-skill-stats-source-03")),
    );
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        run_child(&root, &out, mode == "on");
        return;
    }
    assert!(
        !out.exists(),
        "choose a fresh {OUTPUT}; immutable evidence exists: {}",
        out.display()
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
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "{}\n{}", path.display(), tail(&path));
                break;
            }
            if start.elapsed() > Duration::from_secs(300) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "extra-stat source deadline: {}\n{}",
                    path.display(),
                    tail(&path)
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        fs::read(out.join("source-jit-off.json")).unwrap(),
        fs::read(out.join("source-jit-on.json")).unwrap(),
        "JIT modes must agree"
    );
}
fn run_child(root: &Path, out: &Path, enabled: bool) {
    let input = root.join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let original = fs::read_to_string(&input).unwrap();
    let purifying = custom(&original, PURIFYING);
    let cases = [
        ("original-05", original.clone()),
        ("purifying-original-switches", purifying.clone()),
        ("purifying-global2-true", second_switch(&purifying, "true")),
        (
            "purifying-global2-false",
            second_switch(&purifying, "false"),
        ),
        ("lightning-original-switches", custom(&original, LIGHTNING)),
        ("repeat-original-05", original.clone()),
    ];
    let mut results = vec![];
    for (name, xml) in cases {
        let cold = std::cell::RefCell::new(None);
        let data_ready = |lua: &Lua| -> Result<(), RuntimeError> {
            let first = observe(lua, "before_build")?;
            assert!(
                first == observe(lua, "before_build")?,
                "cold observer must be read-only"
            );
            *cold.borrow_mut() = Some(first);
            Ok(())
        };
        let stage = |lua: &Lua| -> Result<Json, RuntimeError> {
            let base = super::observe_stage(lua)?;
            lua.globals().set("extraStatBase", lua.to_value(&base)?)?;
            let first = observe(lua, "stage")?;
            assert!(
                first == observe(lua, "stage")?,
                "stage observer must be read-only"
            );
            assert!(
                base == super::observe_stage(lua)?,
                "observer changed exact source objects"
            );
            let mut result = base;
            result["extra_skill_stats"] = first;
            Ok(result)
        };
        let mut case = observe_case_with_stage_and_data_hook(
            root,
            name,
            &xml,
            enabled,
            &stage,
            Some(&data_ready),
        );
        case["before_build"] = cold.into_inner().unwrap();
        results.push(case);
    }
    let files = [
        "src/Modules/Data.lua",
        "src/Data/Gems.lua",
        "src/Data/Skills/other.lua",
        "src/Data/Skills/act_int.lua",
        "src/Data/SkillStatMap.lua",
        "src/Modules/ModParser.lua",
        "src/Data/ModCache.lua",
        "src/Modules/ModTools.lua",
        "src/Classes/ConfigTab.lua",
        "src/Classes/SkillsTab.lua",
        "src/Classes/ModStore.lua",
        "src/Classes/ModList.lua",
        "src/Classes/ModDB.lua",
        "src/Modules/CalcTools.lua",
        "src/Modules/CalcActiveSkill.lua",
        "src/Modules/CalcSetup.lua",
        "src/HeadlessWrapper.lua",
        "src/Modules/Main.lua",
    ];
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,
        "manifest_sha256":pinned::manifest_sha256(),"observer_sha256":digest(OBSERVER.as_bytes()),
        "original_sha256":digest(original.as_bytes()),"native_inventory_authority":false,
        "all_producer_scope_proved":false,"whole_build_parity":false,"business_wrappers":false,
        "purpose":"finite actual-parser SkillName exclusion controls; no all-producer closure, native mechanic or reference exception",
        "effect_ids":EFFECTS,"control_lines":[PURIFYING,LIGHTNING],
        "files":files.map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})),"cases":results});
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(bytes.len() <= 64 * 1024 * 1024);
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        bytes,
    )
    .unwrap();
    assert_eq!(fs::read_to_string(input).unwrap(), original);
    check(&report);
}
fn effect<'a>(metadata: &'a Json, id: &str) -> &'a Json {
    let matches: Vec<_> = rows(metadata)
        .iter()
        .filter(|row| row["id"] == id)
        .collect();
    assert_eq!(matches.len(), 1);
    matches[0]
}
fn actions<'a>(state: &'a Json, mode: &str, id: &str) -> Vec<&'a Json> {
    rows(&state["modes"][mode]["actions"])
        .iter()
        .filter(|row| row["effect"] == id)
        .collect()
}
fn check_lookups(state: &Json) {
    let lookups = rows(&state["lookups"]);
    assert_eq!(lookups.len(), 3);
    for lookup in lookups {
        let name = lookup["name"].as_str().unwrap();
        assert!(matches!(
            name,
            "Purifying Flame" | "Lightning Trap" | "Firebolt"
        ));
        assert_eq!(lookup["game_id"]["present"], name == "Firebolt");
    }
}
fn check_control_cache(state: &Json, selected: Option<&str>) {
    assert_eq!(state["parser_original"], true);
    assert_eq!(state["parser_public_cache_identity"], true);
    let entries = rows(&state["control_parser_cache"]);
    assert_eq!(entries.len(), 2);
    for (entry, line) in entries.iter().zip([PURIFYING, LIGHTNING]) {
        assert_eq!(entry["line"], line);
        let expected = if selected == Some(line) {
            let (stat, value, skill) = if line == PURIFYING {
                (PURIFYING_STAT, 17, "Purifying Flame")
            } else {
                (LIGHTNING_STAT, 19, "Lightning Trap")
            };
            json!({"present":true,"value":[[{
                "name":"ExtraSkillStat","type":"LIST","flags":0,"keywordFlags":0,
                "value":{"key":stat,"value":value},
                "1":{"type":"SkillName","skillName":skill,"includeTransfigured":true}
            }]]})
        } else {
            json!({"present":false})
        };
        assert_eq!(entry["entry"], expected, "exact control cache for {line}");
    }
}
fn expected_sources(id: &str) -> Vec<(u64, u64, Option<&'static str>)> {
    // The unchanged original contains both generated and authored Djinn copies.
    // Keep their exact source occurrences; a definition-only count conflates them.
    match id {
        "SummonSandDjinnPlayer" | "CommandSandDjinnKnifeThrowPlayer" => {
            vec![(208, 209, Some("Tree:13289")), (215, 216, None)]
        }
        "SummonWaterDjinnPlayer" | "CommandWaterDjinnBubblePlayer" => {
            vec![(226, 227, Some("Tree:32705")), (228, 229, None)]
        }
        FIREBOLT => vec![(243, 244, Some("Item:28:New Item, Ashen Staff"))],
        _ => panic!("unexpected effect"),
    }
}
fn check(report: &Json) {
    assert_eq!(rows(&report["cases"]).len(), 6);
    assert!(case(report, "original-05")["states"] == case(report, "repeat-original-05")["states"]);
    for c in rows(&report["cases"]) {
        let name = c["name"].as_str().unwrap();
        let is_purifying = name.starts_with("purifying");
        let is_lightning = name.starts_with("lightning");
        let control = is_purifying || is_lightning;
        check_lookups(&c["before_build"]);
        // This hook runs after full Main/cache initialization, before XML load.
        // The two literal lines must take the real parser miss path.
        check_control_cache(&c["before_build"], None);
        for id in EFFECTS {
            let cold = effect(&c["before_build"]["metadata"], id);
            assert_eq!(cold["has_global_effect"]["present"], false);
            assert_eq!(
                cold["game_id"]["present"],
                !COMMANDS.contains(&id),
                "cold name resolution {id}"
            );
        }
        for stage in STAGES {
            let state = &c["states"][stage]["extra_skill_stats"];
            assert_eq!(state["native_inventory_authority"], false);
            assert_eq!(state["extra_stat_scope_proved"], false);
            assert_eq!(state["outputs_preserved"], true);
            check_lookups(state);
            check_control_cache(
                state,
                if is_purifying {
                    Some(PURIFYING)
                } else if is_lightning {
                    Some(LIGHTNING)
                } else {
                    None
                },
            );
            for id in EFFECTS {
                let row = effect(&state["metadata"], id);
                let command = COMMANDS.contains(&id);
                assert_eq!(row["game_id"]["present"], !command);
                assert_eq!(
                    row["has_global_effect"]["present"], false,
                    "{name}/{stage}/{id}"
                );
                for part in rows(&row["parts"]) {
                    for map in rows(&part["maps"]) {
                        assert_eq!(
                            map["local_entry"]["present"], false,
                            "excluded custom stat must not initialize lazy map: {name}/{stage}/{id}"
                        );
                    }
                }
            }
            for mode in ["MAIN", "CALCS"] {
                let mods = rows(&state["modes"][mode]["player_records"]);
                assert_eq!(
                    mods.len(),
                    usize::from(control),
                    "actual parser-produced player records {name}/{stage}/{mode}"
                );
                if control {
                    let (stat, amount, skill) = if is_purifying {
                        (PURIFYING_STAT, 17, "Purifying Flame")
                    } else {
                        (LIGHTNING_STAT, 19, "Lightning Trap")
                    };
                    assert_eq!(mods[0]["name"], "ExtraSkillStat");
                    assert_eq!(mods[0]["type"], "LIST");
                    assert_eq!(mods[0]["source"], format!("Custom:{TITLE}"));
                    assert_eq!(mods[0]["value"], json!({"key":stat,"value":amount}));
                    assert_eq!(
                        mods[0]["1"],
                        json!({"type":"SkillName","skillName":skill,"includeTransfigured":true})
                    );
                    assert_eq!(mods[0]["flags"], 0);
                    assert_eq!(mods[0]["keywordFlags"], 0);
                }
                for id in EFFECTS {
                    let observed = actions(state, mode, id);
                    let command = COMMANDS.contains(&id);
                    let actual_sources: Vec<_> = observed
                        .iter()
                        .map(|action| {
                            (
                                action["source_ordinal"].as_u64().unwrap(),
                                action["gem_source_ordinal"].as_u64().unwrap(),
                                action["source"].as_str(),
                            )
                        })
                        .collect();
                    assert_eq!(
                        actual_sources,
                        expected_sources(id),
                        "{name}/{stage}/{mode}/{id}"
                    );
                    for action in observed {
                        assert_eq!(action["cfg"]["game_id"]["present"], !command);
                        if command {
                            assert_eq!(action["cfg"]["include_transfigured_match_game_id"], "");
                        } else {
                            assert_eq!(
                                action["cfg"]["include_transfigured_match_game_id"],
                                action["cfg"]["game_id"]["value"]
                            );
                        }
                        let stats = rows(&action["extra_stats"]);
                        assert!(
                            stats.is_empty(),
                            "actual source excludes unrelated SkillName: {name}/{stage}/{mode}/{id}"
                        );
                    }
                }
            }
            for group in rows(&state["groups"]) {
                for row in rows(&group["effects"]) {
                    if COMMANDS.iter().any(|id| row["id"] == *id) {
                        assert_eq!(
                            row["global_value"],
                            json!({"present":true,"value":group["source"].is_null() || name=="purifying-global2-true"})
                        );
                    }
                }
            }
            let caches = rows(&state["parser_cache"]);
            assert_eq!(caches.len(), usize::from(control));
            if control {
                assert_eq!(
                    caches[0]["line"],
                    if is_purifying { PURIFYING } else { LIGHTNING }
                );
                assert_eq!(caches[0]["entry"]["present"], true);
            }
        }
    }
}
