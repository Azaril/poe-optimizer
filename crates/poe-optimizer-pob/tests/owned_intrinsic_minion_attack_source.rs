//! Fresh complete-source actor construction and intrinsic weapon observations.
//! Source-only vectors do not grant native coverage or whole-build parity.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
#[path = "../../../tests/support/minion_attack_source_vectors.rs"]
mod vectors;

use mlua::{Function, Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "fresh_intrinsic_minion_weapons_follow_original_actor_construction";
const CHILD: &str = "POE_INTRINSIC_MINION_ATTACK_SOURCE_CHILD";
const OBSERVE: &str = include_str!("support/owned_intrinsic_minion_attack_source.lua");
const SNIPER: &str = "SummonSkeletalSnipersPlayer";
const SPECTRE: &str = "Metadata/Monsters/LeagueAbyss/Blackblood/CollectorSpectre";
const FILES: &[&str] = &[
    "src/Launch.lua",
    "src/GameVersions.lua",
    "src/Modules/Common.lua",
    "src/Modules/Main.lua",
    "src/Modules/Build.lua",
    "src/Modules/Data.lua",
    "src/Classes/SkillsTab.lua",
    "src/Classes/CalcsTab.lua",
    "src/Classes/ItemsTab.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcActiveSkill.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/CalcOffence.lua",
    "src/Modules/CalcTools.lua",
    "src/Modules/Calcs.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModList.lua",
    "src/Classes/ModStore.lua",
    "src/Data/Gems.lua",
    "src/Data/Minions.lua",
    "src/Data/Spectres.lua",
    "src/Data/Misc.lua",
    "src/Data/Skills/act_int.lua",
    "src/Data/Skills/act_dex.lua",
    "src/Data/Skills/other.lua",
    "src/Data/Skills/minion.lua",
    "src/TreeData/0_5/tree.lua",
    "runtime/lua/xml.lua",
];

#[derive(Clone)]
struct Case {
    name: String,
    xml: String,
    warm: Option<String>,
    original: bool,
}

#[test]
fn fresh_intrinsic_minion_weapons_follow_original_actor_construction() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-intrinsic-minion-attack-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "on" || mode == "off");
        run_child(&root, &out, mode == "on");
        return;
    }
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, mode)
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
                    "source child failed {}\n{}",
                    path.display(),
                    tail(&path)
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(240) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline {}\n{}", path.display(), tail(&path));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        read(&out.join("source-jit-off.json")),
        read(&out.join("source-jit-on.json")),
        "exact scoped JIT evidence"
    );
}

fn run_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
    let index = read(&fixtures.join("index.json"));
    let originals: Vec<_> = [3, 5]
        .into_iter()
        .map(|n| {
            let filename = format!("build-{n:02}.xml");
            let xml = fs::read_to_string(fixtures.join(&filename)).unwrap();
            let entry = rows(&index["builds"])
                .iter()
                .find(|v| v["xml"] == filename)
                .unwrap();
            assert_eq!(entry["xml_sha256"], digest(xml.as_bytes()));
            (filename, xml)
        })
        .collect();
    let original = &originals[1].1;
    let mut inputs = vec![Case {
        name: "original-05".into(),
        xml: original.clone(),
        warm: None,
        original: true,
    }];
    for level in [1, 7, 30, 40] {
        inputs.push(Case {
            name: format!("sniper-physical-{level}"),
            xml: sniper_level(original, level),
            warm: None,
            original: false,
        });
    }
    inputs.push(Case {
        name: "repeat-original-05".into(),
        xml: original.clone(),
        warm: None,
        original: true,
    });
    inputs.push(Case {
        name: "warm-high-to-original".into(),
        xml: original.clone(),
        warm: Some(sniper_level(original, 40)),
        original: true,
    });
    inputs.push(Case {
        name: "two-sniper-occurrences".into(),
        xml: duplicate_sniper(original),
        warm: None,
        original: false,
    });
    for level in [7, 20] {
        inputs.push(Case {
            name: format!("spectre-physical-{level}"),
            xml: one_skill(original, "SummonSpectrePlayer", level, Some(SPECTRE)),
            warm: None,
            original: false,
        });
    }
    inputs.push(Case {
        name: "manifest-equipped-weapon".into(),
        xml: one_skill(&originals[0].1, "ManifestWeaponPlayer", 20, None),
        warm: None,
        original: false,
    });
    let mut cases = Vec::new();
    for case in &inputs {
        eprintln!("complete intrinsic minion attack case {}", case.name);
        let before = |lua: &Lua| {
            lua.globals().set("intrinsicAttackJit", enabled)?;
            lua.load("if intrinsicAttackJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| {
            lua.globals().set("intrinsicAttackPhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@intrinsic-minion-attack-authentication")
                .eval::<Function>()?)
        };
        let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
            lua.globals().set("intrinsicAttackPhase", "after")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@intrinsic-minion-attack-observation")
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
        );
        let row = match observed {
            Ok(value) => {
                assert_eq!(value["configuration_method_wrappers"], false);
                assert_eq!(value["original_build_output_available"], true);
                assert_eq!(value["source_hash"], pinned::manifest_sha256());
                json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),"warm_xml_sha256":case.warm.as_ref().map(|v|digest(v.as_bytes())),"available":true,"state":value["additional_observation"]})
            }
            Err(error) => {
                json!({"name":case.name,"available":false,"source_error":error.to_string()})
            }
        };
        cases.push(row);
        fs::write(
            out.join(format!(
                "source-jit-{}-progress.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&cases).unwrap(),
        )
        .unwrap();
    }
    for (filename, xml) in &originals {
        assert_eq!(fs::read_to_string(fixtures.join(filename)).unwrap(), *xml);
    }
    let evidence = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","source_hash":pinned::manifest_sha256(),
        "evidence":{"case_count":inputs.len(),"complete_load_attempts_per_jit":inputs.len()+1,"observer_sha256":digest(OBSERVE.as_bytes()),
            "business_method_wrappers":false,"actor_level_mutation":false,"native_coverage":false,"whole_build_parity":false,
            "owned_ability_identity":owned_ability_identity(root),
            "files":FILES.iter().map(|p|json!({"path":p,"sha256":pinned::expected_file_sha256(p).unwrap()})).collect::<Vec<_>>(),
            "originals":originals.iter().map(|(n,x)|json!({"name":n,"sha256":digest(x.as_bytes())})).collect::<Vec<_>>()},"cases":cases});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
    check(&evidence);
    let projected = vectors::project(&evidence);
    let golden =
        read(&root.join("tests/fixtures/calibration/intrinsic-minion-attack-3887ae68.json"));
    assert_eq!(
        projected, golden,
        "fresh source differs from committed native parity vectors"
    );
}

fn check(evidence: &Json) {
    let cases = rows(&evidence["cases"]);
    assert_eq!(cases.len(), 11);
    for case in cases {
        assert_eq!(
            case["available"], true,
            "{}: {}",
            case["name"], case["source_error"]
        );
        for field in [
            "original_functions_preserved",
            "loaded_state_preserved",
            "cached_outputs_preserved",
            "fresh_actor_construction",
        ] {
            assert_eq!(case["state"][field], true);
        }
        assert_eq!(case["state"]["source_actor_level_mutated"], false);
        assert_eq!(case["state"]["business_method_wrappers"], false);
        for mode in ["main", "calcs"] {
            for actor in rows(&case["state"][mode]["actors"]) {
                assert_eq!(actor["fresh_actor"], true);
                if !actor["policy"]["uses_weapon1"].as_bool().unwrap()
                    && !actor["policy"]["minion_use_bow_and_quiver"]
                        .as_bool()
                        .unwrap()
                    && !actor["policy"]["iron_mass"].as_bool().unwrap()
                    && !actor["policy"]["weapon1_from_item"].as_bool().unwrap()
                {
                    check_intrinsic(actor);
                }
            }
        }
    }
    let original = named(cases, "original-05");
    let main = one_actor(original, "main", SNIPER);
    assert_eq!(main["physical_level"], 20);
    assert_eq!(main["effective_level"], 22);
    assert_eq!(main["actor_level"], 44);
    assert_eq!(main["profile"]["ignore_attack_speed"], true);
    assert_eq!(main["profile"]["attack_time"], 1.5);
    let basic = rows(&main["children"])
        .iter()
        .find(|v| v["effect_id"] == "MinionMeleeBow")
        .unwrap();
    assert_eq!(basic["effect_name"], "Basic Attack");
    assert_eq!(basic["selected"], true);
    let passes = rows(&basic["consumer"]["passes"]);
    assert_eq!(passes.len(), 1);
    for field in ["PhysicalMin", "PhysicalMax", "AttackRate", "CritChance"] {
        assert_eq!(passes[0]["source"][field], main["weapon1"][field]);
    }
    assert_eq!(passes[0]["copied_from_actor"], true);
    for name in ["repeat-original-05", "warm-high-to-original"] {
        assert_eq!(
            named(cases, name)["state"],
            original["state"],
            "fresh/reused {name}"
        );
    }
    let mut prior = 0.0;
    for name in [
        "sniper-physical-1",
        "sniper-physical-7",
        "original-05",
        "sniper-physical-30",
        "sniper-physical-40",
    ] {
        let actor = one_actor(named(cases, name), "main", SNIPER);
        assert_eq!(actor["actor_level"], actor["table_actor_level"]);
        let minimum = number(&actor["weapon1"]["PhysicalMin"]);
        assert!(minimum > prior, "fresh level baseline {name}");
        prior = minimum;
    }
    let repeated: Vec<_> = rows(&named(cases, "two-sniper-occurrences")["state"]["main"]["actors"])
        .iter()
        .filter(|a| a["summon_effect_id"] == SNIPER)
        .collect();
    assert_eq!(repeated.len(), 2);
    assert_ne!(repeated[0]["source_group"], repeated[1]["source_group"]);
    assert_ne!(repeated[0]["actor_level"], repeated[1]["actor_level"]);
    assert_ne!(repeated[0]["weapon1"], repeated[1]["weapon1"]);
    for name in ["spectre-physical-7", "spectre-physical-20"] {
        let actor = one_actor(named(cases, name), "main", "SummonSpectrePlayer");
        assert_eq!(actor["actor_profile"], SPECTRE);
        assert_eq!(actor["profile"]["ignore_attack_speed"], false);
        assert_eq!(actor["profile"]["ignore_attack_speed_present"], false);
        assert_eq!(actor["profile"]["attack_time"], 1.5);
        assert_eq!(actor["curve"]["kind"], "hostile");
        assert_eq!(actor["policy"]["monster_damage"], true);
        check_intrinsic(actor);
    }
    let manifest = one_actor(
        named(cases, "manifest-equipped-weapon"),
        "main",
        "ManifestWeaponPlayer",
    );
    assert_eq!(manifest["policy"]["minion_has_item_set"], true);
    assert_eq!(manifest["policy"]["uses_weapon1"], true);
    assert_eq!(manifest["policy"]["weapon1_from_item"], true);
}

fn check_intrinsic(actor: &Json) {
    let p = &actor["profile"];
    let time = number(&p["attack_time"]);
    let mut damage = number(&actor["curve"]["raw_value"]).floor() * number(&p["damage_scale"]);
    if !p["ignore_attack_speed"].as_bool().unwrap() {
        damage *= time;
    }
    let spread = number(&p["damage_spread"]);
    assert_eq!(
        number(&actor["weapon1"]["PhysicalMin"]),
        (damage * (1.0 - spread)).floor(),
        "{}",
        actor["actor_profile"]
    );
    assert_eq!(
        number(&actor["weapon1"]["PhysicalMax"]),
        (damage * (1.0 + spread)).floor()
    );
    if time == 0.0 {
        // Complete originals also contain non-attacking Djinn profiles. Keep
        // their non-finite source result explicit; native admission is separate.
        assert_eq!(actor["weapon1"]["AttackRate"], "inf");
    } else {
        assert_eq!(number(&actor["weapon1"]["AttackRate"]), 1.0 / time);
    }
    assert_eq!(actor["weapon1"]["CritChance"], p["crit_chance"]);
    assert_eq!(actor["weapon1"]["range"], p["attack_range"]);
}
fn number(v: &Json) -> f64 {
    v.as_f64().unwrap()
}
fn owned_ability_identity(root: &Path) -> Json {
    let mapping_path = "data/owned/poe2/3887ae68/import/mapping-seed.json";
    let bindings_path = "data/owned/poe2/3887ae68/actor-ability-supply/bindings.json";
    let mapping = read(&root.join(mapping_path));
    assert_eq!(
        mapping["source"]["revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    let entries: Vec<_> = rows(&mapping["entries"])
        .iter()
        .filter(|entry| entry["source"]["value"]["value"]["effect_id"]["value"] == "MinionMeleeBow")
        .collect();
    assert_eq!(entries.len(), 1);
    let entry = entries[0];
    assert_eq!(entry["source"]["kind"], "definition");
    assert_eq!(entry["source"]["value"]["kind"], "skill");
    assert_eq!(entry["outcome"]["kind"], "mapped");
    assert_eq!(entry["outcome"]["value"]["basis"]["kind"], "exact");
    let skill = &entry["outcome"]["value"]["target"]["value"]["value"];
    assert_eq!(skill["kind"], "skill");
    assert_eq!(skill["key"], "def.0000000000000021");
    let bindings = read(&root.join(bindings_path));
    let abilities: Vec<_> = rows(&bindings["abilities"])
        .iter()
        .filter(|ability| ability["skill"] == *skill)
        .collect();
    assert_eq!(abilities.len(), 1);
    let output = &abilities[0]["output"];
    assert_eq!(output["declaration"]["kind"], "skill");
    assert_eq!(output["declaration"]["definition"], *skill);
    assert_eq!(output["slot"]["kind"], "action_output");
    assert_eq!(output["slot"]["key"], "def.0000000000000022");
    json!({"source_effect":"MinionMeleeBow", "source_display":"Basic Attack", "skill":skill, "output":output,
        "mapping":{"path":mapping_path,"sha256":digest(&fs::read(root.join(mapping_path)).unwrap())},
        "bindings":{"path":bindings_path,"sha256":digest(&fs::read(root.join(bindings_path)).unwrap())}})
}
fn named<'a>(rows: &'a [Json], name: &str) -> &'a Json {
    let found: Vec<_> = rows.iter().filter(|v| v["name"] == name).collect();
    assert_eq!(found.len(), 1);
    found[0]
}
fn one_actor<'a>(case: &'a Json, mode: &str, effect: &str) -> &'a Json {
    let found: Vec<_> = rows(&case["state"][mode]["actors"])
        .iter()
        .filter(|v| v["summon_effect_id"] == effect)
        .collect();
    assert_eq!(found.len(), 1, "{} {mode} {effect}", case["name"]);
    found[0]
}
fn rows(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(
            value.as_object().is_some_and(|v| v.is_empty()),
            "not source list: {value}"
        );
        &[]
    }
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn selected_set<'a, 'input>(doc: &'a roxmltree::Document<'input>) -> roxmltree::Node<'a, 'input> {
    let skills = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Skills"))
        .unwrap();
    let id = skills.attribute("activeSkillSet").unwrap();
    skills
        .children()
        .find(|n| n.has_tag_name("SkillSet") && n.attribute("id") == Some(id))
        .unwrap()
}
fn sniper_level(xml: &str, level: u32) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let set = selected_set(&doc);
    let gems: Vec<_> = set
        .descendants()
        .filter(|n| n.has_tag_name("Gem") && n.attribute("skillId") == Some(SNIPER))
        .collect();
    assert_eq!(gems.len(), 1);
    let gem = gems[0];
    let attrs = gem
        .attributes()
        .map(|a| {
            format!(
                "{}=\"{}\"",
                a.name(),
                escape(&if a.name() == "level" {
                    level.to_string()
                } else {
                    a.value().to_owned()
                })
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut out = xml.to_owned();
    out.replace_range(gem.range(), &format!("<Gem {attrs}/>"));
    out
}
fn duplicate_sniper(xml: &str) -> String {
    let changed = sniper_level(xml, 7);
    let doc = roxmltree::Document::parse(&changed).unwrap();
    let set = selected_set(&doc);
    let group = set
        .children()
        .find(|g| {
            g.has_tag_name("Skill")
                && g.children()
                    .any(|n| n.has_tag_name("Gem") && n.attribute("skillId") == Some(SNIPER))
        })
        .unwrap();
    let raw = &changed[group.range()];
    let original = roxmltree::Document::parse(xml).unwrap();
    let range = selected_set(&original).range();
    let end = range.end - "</SkillSet>".len();
    assert!(xml[end..range.end].starts_with("</SkillSet>"));
    let mut out = xml.to_owned();
    out.insert_str(end, raw);
    out
}
fn one_skill(xml: &str, effect: &str, level: u32, spectre: Option<&str>) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let set = selected_set(&doc);
    let set_id = set.attribute("id").unwrap();
    let skill = format!(
        "<SkillSet id=\"{set_id}\" title=\"Intrinsic source control\"><Skill enabled=\"true\" mainActiveSkill=\"1\" mainActiveSkillCalcs=\"1\" includeInFullDPS=\"true\"><Gem skillId=\"{effect}\" enabled=\"true\" level=\"{level}\" quality=\"0\" count=\"1\" skillMinionSkill=\"1\" skillMinionSkillCalcs=\"1\" corrupted=\"false\" corruptLevel=\"0\"/></Skill></SkillSet>"
    );
    let mut out = xml.to_owned();
    out.replace_range(set.range(), &skill);
    let doc = roxmltree::Document::parse(&out).unwrap();
    let build = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Build"))
        .unwrap();
    let attrs = build
        .attributes()
        .filter(|a| a.name() != "mainSkillIndex")
        .map(|a| {
            format!(
                "{}=\"{}\"",
                a.name(),
                escape(if a.name() == "mainSocketGroup" {
                    "1"
                } else {
                    a.value()
                })
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let body = build
        .children()
        .filter(|n| !n.has_tag_name("Spectre"))
        .map(|n| &out[n.range()])
        .collect::<String>();
    let extra = spectre
        .map(|id| format!("<Spectre id=\"{}\"/>", escape(id)))
        .unwrap_or_default();
    let range = build.range();
    let replacement = format!("<Build {attrs}>{body}{extra}</Build>");
    out.replace_range(range, &replacement);
    out
}
fn tail(path: &Path) -> String {
    let mut file = fs::File::open(path).unwrap();
    let length = file.metadata().unwrap().len();
    file.seek(SeekFrom::Start(length.saturating_sub(16 * 1024)))
        .unwrap();
    let mut bytes = Vec::new();
    file.take(16 * 1024).read_to_end(&mut bytes).unwrap();
    String::from_utf8_lossy(&bytes).into_owned()
}
