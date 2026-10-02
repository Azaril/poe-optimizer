//! Complete source-only evidence for saved occupied item augment lifecycles.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Function, Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const TEST: &str = "occupied_item_augments_preserve_complete_source_state";
const CHILD: &str = "POE_OCCUPIED_ITEM_AUGMENTS_SOURCE_CHILD";
const OBSERVE: &str = include_str!("support/owned_occupied_item_augments_source.lua");
const PINNED_FILES: &[&str] = &[
    "src/Launch.lua",
    "src/GameVersions.lua",
    "src/Modules/Common.lua",
    "src/Modules/Main.lua",
    "src/Modules/Build.lua",
    "src/Modules/Data.lua",
    "src/Classes/Item.lua",
    "src/Classes/ItemsTab.lua",
    "src/Classes/ItemSlotControl.lua",
    "src/Classes/CalcsTab.lua",
    "src/Classes/ConfigTab.lua",
    "src/Classes/PassiveSpec.lua",
    "src/Classes/TreeTab.lua",
    "src/Modules/ItemTools.lua",
    "src/Modules/ModTools.lua",
    "src/Modules/ModParser.lua",
    "src/Modules/CalcSetup.lua",
    "src/Classes/ModList.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModStore.lua",
    "src/Data/ModRunes.lua",
    "src/TreeData/0_5/tree.lua",
    "runtime/lua/xml.lua",
];
#[derive(Clone)]
struct Case {
    name: String,
    xml: String,
    warm: Option<String>,
    controls: bool,
    family_controls: bool,
}
#[test]
fn occupied_item_augments_preserve_complete_source_state() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-occupied-item-augments-source-01");
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
    let originals: Vec<_> = (1..=5)
        .map(|n| {
            let filename = format!("build-{n:02}.xml");
            let xml = fs::read_to_string(fixtures.join(&filename)).unwrap();
            let row = rows(&index["builds"])
                .iter()
                .find(|r| r["xml"] == filename)
                .unwrap();
            assert_eq!(row["xml_sha256"], digest(xml.as_bytes()));
            Case {
                name: format!("original-{n:02}"),
                xml,
                warm: None,
                controls: n == 3,
                family_controls: n == 4,
            }
        })
        .collect();
    let mut inputs = originals.clone();
    let original = &originals[2].xml;
    let normal = "{enchant}{rune}36% increased Armour, Evasion and Energy Shield";
    let bonded = "{enchant}{rune}Bonded: +40 to maximum Life";
    for (name, before, after) in [
        (
            "changed-saved-value",
            normal,
            "{enchant}{rune}99% increased Armour, Evasion and Energy Shield".to_owned(),
        ),
        ("disabled-normal", normal, format!("{{disabled}}{normal}")),
        ("disabled-bonded", bonded, format!("{{disabled}}{bonded}")),
        (
            "extra-effect-thirteen",
            "+44 to maximum Life",
            "+44 to maximum Life\n13% increased Effect of Socketed Runes".to_owned(),
        ),
        (
            "unknown-saved-retained",
            "Rune: Greater Iron Rune\nRune: Greater Iron Rune",
            "Rune: Owned Unknown Rune\nRune: Greater Iron Rune".to_owned(),
        ),
        (
            "missing-inferred",
            "Rune: Greater Iron Rune\nRune: Greater Iron Rune\n",
            String::new(),
        ),
    ] {
        inputs.push(Case {
            name: name.into(),
            xml: edit_item(original, 14, |raw| {
                assert_eq!(raw.matches(before).count(), 1);
                raw.replace(before, &after)
            }),
            warm: None,
            controls: false,
            family_controls: false,
        });
    }
    for (name, unknown) in [("known-surplus", false), ("unknown-surplus", true)] {
        inputs.push(Case {
            name: name.into(),
            xml: edit_item(original, 14, |raw| {
                let before = "Rune: Greater Iron Rune\nRune: Greater Iron Rune";
                assert_eq!(raw.matches(before).count(), 1);
                let after = format!(
                    "{before}\nRune: {}",
                    if unknown {
                        "Owned Unknown Rune"
                    } else {
                        "Greater Iron Rune"
                    }
                );
                let changed = raw.replace(before, &after);
                if unknown {
                    changed.replace(
                        normal,
                        "{enchant}{rune}99% increased Armour, Evasion and Energy Shield",
                    )
                } else {
                    changed
                }
            }),
            warm: None,
            controls: false,
            family_controls: false,
        });
    }
    // A visibly stale saved value distinguishes retention from reconstruction.
    let unknown = inputs
        .iter_mut()
        .find(|c| c.name == "unknown-saved-retained")
        .unwrap();
    unknown.xml = edit_item(&unknown.xml, 14, |raw| {
        raw.replace(
            normal,
            "{enchant}{rune}99% increased Armour, Evasion and Energy Shield",
        )
    });
    inputs.push(Case {
        name: "warm-scaled-to-original".into(),
        xml: original.clone(),
        warm: Some(
            inputs
                .iter()
                .find(|c| c.name == "extra-effect-thirteen")
                .unwrap()
                .xml
                .clone(),
        ),
        controls: false,
        family_controls: false,
    });
    let mut cases = Vec::new();
    for case in &inputs {
        eprintln!("complete occupied augment source case {}", case.name);
        let before = |lua: &Lua| {
            lua.globals().set("occupiedAugmentXml", case.xml.as_str())?;
            lua.globals()
                .set("occupiedAugmentControls", case.controls)?;
            lua.globals()
                .set("occupiedAugmentFamilyControls", case.family_controls)?;
            lua.globals().set("occupiedAugmentJit", enabled)?;
            lua.load("if occupiedAugmentJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| {
            lua.globals().set("occupiedAugmentPhase", "before")?;
            Ok(lua
                .load(OBSERVE)
                .set_name("@occupied-augment-authentication")
                .eval::<Function>()?)
        };
        let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
            lua.globals().set("occupiedAugmentPhase", "after")?;
            let value: Value = lua
                .load(OBSERVE)
                .set_name("@occupied-augment-observation")
                .eval()?;
            Ok(lua.from_value(value)?)
        };
        let temp = tempfile::tempdir().unwrap();
        let value = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            temp.path(),
            &case.xml,
            case.warm.as_deref(),
            !case.name.starts_with("original-"),
            Some(&before),
            Some(&install),
            Some(&observe),
        );
        let row = match value {
            Ok(value) => {
                assert_eq!(value["configuration_method_wrappers"], false);
                assert_eq!(value["original_build_output_available"], true);
                assert_eq!(value["source_hash"], pinned::manifest_sha256());
                json!({"name":case.name,"xml_sha256":digest(case.xml.as_bytes()),"warm_xml_sha256":case.warm.as_ref().map(|s|digest(s.as_bytes())),"expected_selections":selections(&case.xml),"available":true,"state":value["additional_observation"]})
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
    for (n, original) in originals.iter().enumerate() {
        assert_eq!(
            fs::read_to_string(fixtures.join(format!("build-{:02}.xml", n + 1))).unwrap(),
            original.xml
        );
    }
    let result = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","source_hash":pinned::manifest_sha256(),"evidence":{
        "complete_load_attempts_per_jit":15,"complete_cases":14,"unchanged_originals":5,"full_controls":9,"isolated_item_controls":24,"isolated_family_controls":6,
        "native_effect_coverage":false,"whole_build_parity":false,"business_method_wrappers":false,"prepared_binding_authority":false,
        "observer_sha256":digest(OBSERVE.as_bytes()),"originals":originals.iter().map(|c|json!({"name":c.name,"sha256":digest(c.xml.as_bytes())})).collect::<Vec<_>>(),
        "files":PINNED_FILES.iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>()},"cases":cases});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    check(&result);
}
fn check(result: &Json) {
    let cases = rows(&result["cases"]);
    assert_eq!(cases.len(), 14);
    let case = |name: &str| unique(cases, name);
    for row in cases {
        let name = row["name"].as_str().unwrap();
        assert_eq!(row["available"], true, "{name}: {}", row["source_error"]);
        for flag in [
            "saved_items_preserved",
            "saved_item_sets_preserved",
            "saved_specs_preserved",
            "saved_selections_preserved",
            "main_output_preserved",
            "calcs_output_preserved",
            "original_functions_preserved",
            "fresh_loaded_items",
        ] {
            assert_eq!(row["state"][flag], true, "{name}: {flag}");
        }
        assert_eq!(
            row["state"]["selected"], row["expected_selections"],
            "{name}"
        );
        assert!(!row["state"]["main_output"].as_object().unwrap().is_empty());
        assert!(!row["state"]["calcs_output"].as_object().unwrap().is_empty());
        for host in rows(&row["state"]["hosts"]) {
            assert_eq!(host["fresh_reconstruction_equal"], true, "{name}");
            assert_eq!(host["loaded"]["runes"], host["fresh"]["runes"], "{name}");
            assert_eq!(
                host["loaded"]["socket_count"], host["fresh"]["socket_count"],
                "{name}"
            );
            let expected: Vec<_> = host["raw"]
                .as_str()
                .unwrap()
                .lines()
                .filter_map(|line| line.trim().strip_prefix("Rune: "))
                .map(|name| json!(name.trim()))
                .collect();
            if name == "missing-inferred" && host["id"] == 14 {
                assert!(
                    expected.is_empty(),
                    "inference control has no saved headers"
                );
                assert_eq!(
                    host["loaded"]["runes"],
                    json!(["Perfect Iron Rune", "Iron Rune"])
                );
            } else {
                assert_eq!(
                    rows(&host["loaded"]["runes"]),
                    expected,
                    "{name}: exact saved header order"
                );
            }
            let branches = rows(&host["branches"]);
            assert_eq!(branches.len(), 3);
            assert_eq!(branches[0], branches[2], "{name}: cache restoration");
            assert_eq!(branches[1]["active_state"], "all");
        }
    }
    for (row, (host_count, headers)) in
        cases[..5]
            .iter()
            .zip([(6, 7), (6, 7), (5, 7), (6, 9), (0, 0)])
    {
        let actual = rows(&row["state"]["hosts"]);
        assert_eq!(actual.len(), host_count, "{}", row["name"]);
        assert_eq!(
            actual
                .iter()
                .map(|h| rows(&h["loaded"]["runes"]).len())
                .sum::<usize>(),
            headers
        );
        for host in actual {
            let runes = rows(&host["loaded"]["runes"]);
            assert!(
                runes
                    .iter()
                    .all(|name| name.as_str().is_some_and(|s| !s.is_empty() && s != "None"))
            );
            assert_eq!(
                host["loaded"]["socket_count"].as_u64().unwrap() as usize,
                runes.len()
            );
        }
    }
    let host = |name: &str, id: u64| host(case(name), id);
    assert_eq!(
        host("original-02", 2)["loaded"]["unknown_headers"],
        json!([{ "ordinal":1,"name":"Purity of Lightning" }])
    );
    assert_eq!(
        host("original-04", 16)["loaded"]["unknown_headers"],
        json!([{ "ordinal":1,"name":"Jiquani's Soul Core of Rippling" }])
    );
    // Unknown headers retain occupied counts but do not invent effects. The
    // sceptre has only an ordinary implicit skill grant; the staff actually
    // carries two saved rune lines whose parsed effects survive the fallback.
    let sceptre = host("original-02", 2);
    assert!(rows(&sceptre["loaded"]["lists"]["rune"]).is_empty());
    assert!(!sceptre["raw"].as_str().unwrap().contains("{rune}"));
    let grant = line(
        &sceptre["loaded"]["lists"]["implicit"],
        "Grants Skill: Level 16 Purity of Lightning",
    );
    assert_eq!(rows(&grant["mods"]).len(), 1);
    assert_eq!(grant["mods"][0]["name"], "ExtraSkill");
    assert_eq!(
        grant["mods"][0]["value"],
        json!({"level":16,"skillId":"PurityOfLightningPlayer"})
    );
    let staff = host("original-04", 16);
    assert_eq!(
        texts(&staff["loaded"]["lists"]["rune"]),
        vec![
            json!("+1 to Level of all Spell Skills"),
            json!("+1 to Level of all Nova Skill Gems")
        ]
    );
    for (text, keyword) in [
        ("+1 to Level of all Spell Skills", "spell"),
        ("+1 to Level of all Nova Skill Gems", "nova"),
    ] {
        let parsed = &line(&staff["loaded"]["lists"]["rune"], text)["mods"];
        assert_eq!(rows(parsed).len(), 1);
        assert_eq!(parsed[0]["name"], "GemProperty");
        assert_eq!(
            parsed[0]["value"],
            json!({"key":"level","keyOfScaledMod":"value","keyword":keyword,"value":1})
        );
        for mode in ["main", "calcs"] {
            assert_eq!(
                rows(&staff["delivery"][mode]["delivered"])
                    .iter()
                    .filter(|m| **m == parsed[0])
                    .count(),
                1,
                "{mode}: exact saved staff effect delivered once"
            );
        }
    }
    for mode in ["main", "calcs"] {
        assert_eq!(
            rows(&sceptre["delivery"][mode]["delivered"])
                .iter()
                .filter(|m| **m == grant["mods"][0])
                .count(),
            1
        );
        assert_eq!(
            sceptre["delivery"][mode]["multipliers"]["RunesSocketedInWeapon 2"],
            1
        );
        assert_eq!(
            staff["delivery"][mode]["multipliers"]["RunesSocketedInWeapon 1"],
            2
        );
    }
    let baseline = case("original-03");
    for name in [
        "changed-saved-value",
        "known-surplus",
        "warm-scaled-to-original",
    ] {
        for output in ["main_output", "calcs_output"] {
            assert_eq!(
                case(name)["state"][output],
                baseline["state"][output],
                "{name}: {output}"
            );
        }
        assert_eq!(
            host(name, 14)["delivery"],
            host("original-03", 14)["delivery"],
            "{name}: actual consumers"
        );
        assert_eq!(
            host(name, 14)["loaded"]["lists"]["rune"],
            host("original-03", 14)["loaded"]["lists"]["rune"],
            "{name}: rebuilt once"
        );
    }
    let normal = "36% increased Armour, Evasion and Energy Shield";
    let stale = "99% increased Armour, Evasion and Energy Shield";
    for name in ["unknown-saved-retained", "unknown-surplus"] {
        let loaded = &host(name, 14)["loaded"];
        assert_eq!(line(&loaded["lists"]["rune"], stale)["line"], stale);
        assert!(!texts(&loaded["lists"]["rune"]).contains(&json!(normal)));
        assert_ne!(
            loaded["armour_data"],
            host("original-03", 14)["loaded"]["armour_data"],
            "{name}: stale modifier reaches item calculation"
        );
    }
    for name in ["disabled-normal", "extra-effect-thirteen"] {
        assert_ne!(
            host(name, 14)["loaded"]["armour_data"],
            host("original-03", 14)["loaded"]["armour_data"],
            "{name}: real local effect"
        );
        assert_ne!(
            case(name)["state"]["main_output"],
            baseline["state"]["main_output"],
            "{name}: full main consumer"
        );
        assert_ne!(
            case(name)["state"]["calcs_output"],
            baseline["state"]["calcs_output"],
            "{name}: full calcs consumer"
        );
    }
    let probes = rows(&baseline["state"]["probes"]);
    assert_eq!(probes.len(), 24);
    for probe in probes {
        assert_eq!(
            probe["available"], true,
            "{}: {}",
            probe["name"], probe["error"]
        );
        assert_eq!(rows(&probe["branches"]).len(), 3);
        assert_eq!(probe["branches"][0], probe["branches"][2]);
    }
    let probe = |name: &str| unique(probes, name);
    let state = |name: &str| &probe(name)["state"];
    let rune_lines = |name: &str| &state(name)["lists"]["rune"];
    let base = texts(rune_lines("original"));
    assert_eq!(
        base,
        vec![
            json!(normal),
            json!("Bonded: +40 to maximum Life"),
            json!("Bonded: +40 to maximum Mana")
        ]
    );
    for name in [
        "changed-saved-value",
        "removed-saved-normal",
        "known-surplus",
        "duplicate-socket-header",
        "socket-markers",
    ] {
        assert_eq!(texts(rune_lines(name)), base, "{name}");
    }
    let inferred_names = json!(["Perfect Iron Rune", "Iron Rune"]);
    for name in ["missing-all-headers", "lowercase-header"] {
        assert_eq!(state(name)["runes"], inferred_names, "{name}");
        assert_eq!(texts(rune_lines(name)), base, "{name}");
    }
    let inferred = host("missing-inferred", 14);
    assert_eq!(rows(&case("missing-inferred")["state"]["hosts"]).len(), 5);
    assert_eq!(inferred["loaded"]["runes"], inferred_names);
    assert_eq!(texts(&inferred["loaded"]["lists"]["rune"]), base);
    assert_eq!(inferred["delivery"], host("original-03", 14)["delivery"]);
    for output in ["main_output", "calcs_output"] {
        assert_eq!(
            case("missing-inferred")["state"][output],
            baseline["state"][output]
        );
    }
    for name in ["unknown-active", "unknown-surplus"] {
        assert!(texts(rune_lines(name)).contains(&json!(stale)), "{name}");
        assert!(!texts(rune_lines(name)).contains(&json!(normal)), "{name}");
        assert_eq!(rows(&state(name)["unknown_headers"]).len(), 1);
        assert_eq!(
            state(name)["unknown_headers"][0]["ordinal"],
            if name == "unknown-active" { 1 } else { 3 }
        );
    }
    for name in [
        "disabled-normal",
        "disabled-normal-other-value",
        "duplicate-disabled-normal",
    ] {
        assert_eq!(line(rune_lines(name), normal)["disabled"], true, "{name}");
        for stat in ["Armour", "Evasion", "EnergyShield"] {
            delta(
                &state("original")["base_mods"],
                &state(name)["base_mods"],
                stat,
                "INC",
                &[36.0],
                &[],
                name,
            );
        }
    }
    assert_eq!(
        line(rune_lines("disabled-bonded"), "Bonded: +40 to maximum Life")["disabled"],
        true
    );
    assert!(
        rows(&line(rune_lines("disabled-bonded"), "Bonded: +40 to maximum Life")["bonded_mods"])
            .is_empty()
    );
    delta(
        &probe("original")["branches"][0]["active"],
        &probe("original")["branches"][1]["active"],
        "Life",
        "BASE",
        &[],
        &[40.0],
        "Bonded Life",
    );
    delta(
        &probe("original")["branches"][0]["active"],
        &probe("original")["branches"][1]["active"],
        "Mana",
        "BASE",
        &[],
        &[40.0],
        "Bonded Mana",
    );
    delta(
        &probe("original")["branches"][1]["active"],
        &probe("disabled-bonded")["branches"][1]["active"],
        "Life",
        "BASE",
        &[40.0],
        &[],
        "disabled Bonded Life",
    );
    assert!(rows(rune_lines("empty-selection")).is_empty());
    assert_eq!(state("empty-selection")["runes"], json!(["None", "None"]));
    assert_eq!(
        state("known-surplus")["runes"],
        json!([
            "Greater Iron Rune",
            "Greater Iron Rune",
            "Greater Iron Rune"
        ])
    );
    assert_eq!(state("known-surplus")["socket_count"], 2);
    assert_eq!(state("socket-markers")["socket_count"], 2);
    assert_eq!(state("socket-markers")["jewel_count"], 1);
    assert_eq!(state("duplicate-socket-header")["socket_count"], 2);
    assert_eq!(
        state("missing-one-header")["runes"],
        json!(["Greater Iron Rune"])
    );
    assert_eq!(
        line(
            rune_lines("missing-one-header"),
            "18% increased Armour, Evasion and Energy Shield"
        )["line"],
        "18% increased Armour, Evasion and Energy Shield"
    );
    let reused = &baseline["state"]["reused_item"];
    assert_eq!(reused["headers_equal"], true);
    assert_eq!(reused["rune_state_equal"], true);
    assert_eq!(
        reused["before"]["socketedRuneEffectModifier"].as_f64(),
        Some(0.13)
    );
    assert_eq!(
        reused["after"]["socketedRuneEffectModifier"].as_f64(),
        Some(0.0)
    );
    assert_eq!(reused["after"]["base_mods"], reused["fresh"]["base_mods"]);
    // Extra effect is a separately truncated contribution after grouping.
    // Disabled lines are skipped by ordinary processing, but the pinned extra
    // effect loop still contributes its scaled remainder: preserve that quirk.
    for (name, removed, added) in [
        ("extra-effect-thirteen", vec![], vec![4.0]),
        ("disabled-extra-effect-thirteen", vec![36.0], vec![4.0]),
        ("single-extra-effect-thirteen", vec![36.0], vec![18.0, 2.0]),
    ] {
        assert_eq!(
            state(name)["socketedRuneEffectModifier"].as_f64(),
            Some(0.13)
        );
        for stat in ["Armour", "Evasion", "EnergyShield"] {
            delta(
                &state("original")["base_mods"],
                &state(name)["base_mods"],
                stat,
                "INC",
                &removed,
                &added,
                name,
            );
        }
    }
    for (name, values) in [
        ("extra-effect-thirteen", vec![40.0, 5.0]),
        ("single-extra-effect-thirteen", vec![20.0, 2.0]),
    ] {
        for stat in ["Life", "Mana"] {
            delta(
                &probe(name)["branches"][0]["active"],
                &probe(name)["branches"][1]["active"],
                stat,
                "BASE",
                &[],
                &values,
                name,
            );
        }
    }
    for (name, added) in [
        ("weapon-single-extra-effect-twenty-five", vec![18.0, 4.0]),
        ("weapon-double-extra-effect-twenty-five", vec![36.0, 9.0]),
    ] {
        assert_eq!(
            state(name)["socketedRuneEffectModifier"].as_f64(),
            Some(0.25)
        );
        delta(
            &state("weapon-no-runes")["base_mods"],
            &state(name)["base_mods"],
            "PhysicalDamage",
            "INC",
            &[],
            &added,
            name,
        );
    }
    check_families(case("original-04"));
}
fn check_families(original: &Json) {
    let idol = host(original, 11);
    assert_eq!(idol["loaded"]["socketedIdolsUseBondedModifiers"], true);
    assert_eq!(idol["branches"][0]["active_state"], "idol");
    assert_eq!(quality_values(&idol["branches"][0]["active"]), vec![5.0]);
    assert_eq!(quality_values(&idol["branches"][1]["active"]), vec![5.0]);
    for stat in ["Life", "Mana"] {
        delta(
            &idol["branches"][0]["active"],
            &idol["branches"][1]["active"],
            stat,
            "BASE",
            &[],
            &[20.0],
            "Idol-only excludes Rune Bonded",
        );
    }
    for mode in ["main", "calcs"] {
        let delivery = &idol["delivery"][mode];
        assert_eq!(
            rows(&delivery["actual_item_slots"]),
            &[json!("Body Armour")]
        );
        assert!(!rows(&delivery["delivered"]).is_empty());
        assert_eq!(
            quality_values(&delivery["delivered"]),
            vec![5.0],
            "{mode}: actual Idol Bonded consumer"
        );
    }
    let probes = rows(&original["state"]["family_probes"]);
    assert_eq!(probes.len(), 6);
    for probe in probes {
        assert_eq!(
            probe["available"], true,
            "{}: {}",
            probe["name"], probe["error"]
        );
        assert_eq!(rows(&probe["branches"]).len(), 3);
        assert_eq!(probe["branches"][0], probe["branches"][2]);
    }
    let probe = |name: &str| unique(probes, name);
    assert!(quality_values(&probe("idol-removed")["branches"][0]["active"]).is_empty());
    assert_eq!(
        probe("idol-removed")["state"]["socketedIdolsUseBondedModifiers"],
        false
    );
    assert_eq!(
        quality_values(&probe("idol-rune-effect25")["branches"][0]["active"]),
        vec![5.0]
    );
    assert_eq!(
        quality_values(&probe("idol-all-effect25")["branches"][0]["active"]),
        // Nested GemProperty values retain fractional precision, unlike the
        // ordinary integer Life/Spirit contributions checked above.
        vec![1.25, 5.0]
    );
    for name in ["idol-rune-effect25", "idol-all-effect25"] {
        for stat in ["Life", "Mana"] {
            delta(
                &probe(name)["branches"][0]["active"],
                &probe(name)["branches"][1]["active"],
                stat,
                "BASE",
                &[],
                &[20.0, 5.0],
                name,
            );
        }
    }
    let mixed = &host(original, 13)["fresh"]["base_mods"];
    for (name, spirit, life) in [
        ("mixed-soulcore-effect25", vec![3.0], vec![]),
        ("mixed-rune-effect25", vec![], vec![8.0]),
        ("mixed-all-and-family-effect25", vec![7.0], vec![17.0]),
    ] {
        let actual = &probe(name)["state"]["base_mods"];
        delta(mixed, actual, "Spirit", "BASE", &[], &spirit, name);
        delta(mixed, actual, "LifeOnKill", "BASE", &[], &life, name);
    }
}
fn unique<'a>(rows: &'a [Json], name: &str) -> &'a Json {
    let matches: Vec<_> = rows.iter().filter(|row| row["name"] == name).collect();
    assert_eq!(matches.len(), 1, "unique case {name}");
    matches[0]
}
fn host(case: &Json, id: u64) -> &Json {
    let matches: Vec<_> = rows(&case["state"]["hosts"])
        .iter()
        .filter(|host| host["id"] == id)
        .collect();
    assert_eq!(matches.len(), 1, "unique selected host {id}");
    matches[0]
}
fn line<'a>(lines: &'a Json, text: &str) -> &'a Json {
    let matches: Vec<_> = rows(lines)
        .iter()
        .filter(|line| line["line"] == text)
        .collect();
    assert_eq!(matches.len(), 1, "unique rebuilt line {text}");
    matches[0]
}
fn texts(lines: &Json) -> Vec<Json> {
    rows(lines)
        .iter()
        .map(|line| line["line"].clone())
        .collect()
}
fn values(records: &Json, name: &str, kind: &str) -> Vec<f64> {
    let mut result: Vec<_> = rows(records)
        .iter()
        .filter(|m| m["name"] == name && m["type"] == kind)
        .map(|m| m["value"].as_f64().unwrap())
        .collect();
    result.sort_by(f64::total_cmp);
    result
}
fn quality_values(records: &Json) -> Vec<f64> {
    let mut result: Vec<_> = rows(records)
        .iter()
        .filter(|m| {
            m["name"] == "GemProperty" && m["type"] == "LIST" && m["value"]["key"] == "quality"
        })
        .map(|m| m["value"]["value"].as_f64().unwrap())
        .collect();
    result.sort_by(f64::total_cmp);
    result
}
fn delta(
    before: &Json,
    after: &Json,
    name: &str,
    kind: &str,
    removed: &[f64],
    added: &[f64],
    context: &str,
) {
    let mut expected = values(before, name, kind);
    for value in removed {
        let index = expected
            .iter()
            .position(|v| v == value)
            .unwrap_or_else(|| panic!("{context}: missing original {name}={value}: {expected:?}"));
        expected.remove(index);
    }
    expected.extend_from_slice(added);
    expected.sort_by(f64::total_cmp);
    assert_eq!(
        values(after, name, kind),
        expected,
        "{context}: exact {name} {kind} multiset"
    );
}
fn edit_item(xml: &str, id: usize, edit: impl FnOnce(&str) -> String) -> String {
    let document = roxmltree::Document::parse(xml).unwrap();
    let id = id.to_string();
    let items = document
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Items"))
        .unwrap();
    let item = items
        .children()
        .find(|n| n.has_tag_name("Item") && n.attribute("id") == Some(id.as_str()))
        .unwrap();
    let text = item.children().find(|n| n.is_text()).unwrap().range();
    let mut result = xml.to_owned();
    result.replace_range(text.clone(), &edit(&xml[text]));
    result
}
fn selections(xml: &str) -> Json {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let mut out = serde_json::Map::new();
    for (key, element, attribute) in [
        ("config", "Config", "activeConfigSet"),
        ("items", "Items", "activeItemSet"),
        ("skills", "Skills", "activeSkillSet"),
        ("spec", "Tree", "activeSpec"),
        ("group", "Build", "mainSocketGroup"),
    ] {
        let node = doc
            .root_element()
            .children()
            .find(|n| n.has_tag_name(element))
            .unwrap();
        out.insert(
            key.into(),
            json!(node.attribute(attribute).unwrap().parse::<usize>().unwrap()),
        );
    }
    Json::Object(out)
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Json) -> &[Json] {
    if let Some(v) = value.as_array() {
        v
    } else {
        assert!(
            value.as_object().is_some_and(|v| v.is_empty()),
            "not source list: {value}"
        );
        &[]
    }
}
fn tail(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .rev()
        .take(30)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|s| s.chars().take(600).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
