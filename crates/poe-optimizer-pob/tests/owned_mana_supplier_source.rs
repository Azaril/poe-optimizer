//! Optional complete-source constructor witness, never a native coverage grant.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/json_evidence.rs"]
mod json_evidence;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Function, Lua, LuaSerdeExt, Table, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "unchanged_mana_supplier_construction_is_original_and_repeatable";
const CHILD: &str = "POE_MANA_SUPPLIER_SOURCE_CHILD";
const OUTPUT: &str = "POE_MANA_SUPPLIER_SOURCE_OUT";
const COMMON: &str = include_str!("support/owned_minion_physical_damage_source.lua");
const OBSERVER: &str = include_str!("support/mana_supplier_observer.lua");
const XML: &str = "tests/fixtures/builds/breadth-20260908/build-05.xml";
const FAMILIES: [&str; 7] = [
    "baseline",
    "configuration",
    "callback_invocations",
    "node_construction",
    "node_lists",
    "skill_construction",
    "environments",
];

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn helpers(lua: &Lua) -> Result<Table, RuntimeError> {
    let marker = "local methods={";
    assert_eq!(COMMON.matches(marker).count(), 1);
    let prefix = COMMON.split_once(marker).unwrap().0;
    let code = format!(
        "{prefix}\nreturn {{original=original,upvalue=upvalue,scalar=scalar,scalars=scalars,sourceOccurrence=sourceOccurrence,offeringOutputSnapshot=offeringOutputSnapshot}}"
    );
    Ok(lua.load(code).set_name("@existing-source-helpers").eval()?)
}
fn equal(a: &Json, b: &Json, label: &str) {
    assert_eq!(json_evidence::first_difference(a, b, "$"), None, "{label}");
}

// These are source inventories, not a numeric reduction order. Keep every raw
// report intact; compare counted construction visits and full modifier bodies.
// Only cross-stat list placement is ignored. Same-stat order and duplicates
// remain exact, including callback before/after and routing phases.
fn inventory_view(v: &mut Json) {
    match v {
        Json::Array(rows) => {
            for row in rows.iter_mut() {
                inventory_view(row);
            }
            if !rows.is_empty()
                && rows.iter().all(|r| {
                    r.as_object().is_some_and(|o| {
                        o.len() == 2
                            && o.contains_key("position")
                            && o.get("body").is_some_and(|b| b["name"].is_string())
                    })
                })
            {
                let mut buckets: BTreeMap<String, Vec<Json>> = BTreeMap::new();
                for row in std::mem::take(rows) {
                    buckets
                        .entry(row["body"]["name"].as_str().unwrap().to_owned())
                        .or_default()
                        .push(row["body"].clone());
                }
                *v = json!({"counted_source_inventory_by_stat":buckets});
            }
        }
        Json::Object(map) => {
            for child in map.values_mut() {
                inventory_view(child);
            }
        }
        _ => {}
    }
}
fn comparison(v: &Json) -> Json {
    let mut copy = v.clone();
    inventory_view(&mut copy);
    for row in copy["skill_construction"].as_array_mut().unwrap() {
        skill_data_assignment_inventory(row);
        minion_modifier_destination_inventory(row);
    }
    for family in FAMILIES {
        copy[family]
            .as_array_mut()
            .unwrap()
            .sort_by_cached_key(|r| serde_json::to_string(r).unwrap());
    }
    copy
}

// CalcPerform:1161-1167 passes the unchanged nested mod to ModDB:AddMod,
// whose 31-37 implementation appends it to its own name bucket. Preserve
// order within each destination name and every wrapper/filter/body field.
fn minion_modifier_destination_inventory(row: &mut Json) {
    let Some(records) = row
        .get_mut("records")
        .and_then(|r| r.get_mut("counted_source_inventory_by_stat"))
        .and_then(|r| r.get_mut("MinionModifier"))
    else {
        return;
    };
    let Some(values) = records.as_array() else {
        return;
    };
    if !values.iter().all(|r| {
        r["name"] == "MinionModifier"
            && r["type"] == "LIST"
            && r["value"]["mod"]["name"].is_string()
    }) {
        return;
    }
    let mut names: BTreeMap<String, Vec<Json>> = BTreeMap::new();
    for r in values {
        names
            .entry(r["value"]["mod"]["name"].as_str().unwrap().to_owned())
            .or_default()
            .push(r.clone());
    }
    *records = json!({"unchanged_child_records_by_destination_name":names});
}

#[test]
fn minion_wrapper_inventory_preserves_same_destination_order_and_filters() {
    let body = |name: &str, value: i32| json!({"name":"MinionModifier","type":"LIST","value":{"type":"RaisedSkeletonSniper","mod":{"name":name,"type":"BASE","value":value}},"flags":0});
    let normalize = |rows: Vec<Json>| {
        let mut row =
            json!({"records":{"counted_source_inventory_by_stat":{"MinionModifier":rows}}});
        minion_modifier_destination_inventory(&mut row);
        row
    };
    let original = vec![
        body("Damage", 1),
        body("Damage", 2),
        body("CooldownRecovery", 3),
    ];
    assert_eq!(
        normalize(original.clone()),
        normalize(vec![
            body("CooldownRecovery", 3),
            body("Damage", 1),
            body("Damage", 2)
        ])
    );
    let mut filter = original.clone();
    filter[0]["value"]["type"] = json!("DifferentRecipient");
    for rows in [
        vec![
            body("Damage", 2),
            body("Damage", 1),
            body("CooldownRecovery", 3),
        ],
        vec![body("Damage", 1), body("CooldownRecovery", 3)],
        vec![
            body("Damage", 1),
            body("Damage", 2),
            body("CooldownRecovery", 4),
        ],
        filter,
    ] {
        assert_ne!(normalize(original.clone()), normalize(rows));
    }
}

fn source_span(root: &Path, path: &str, first: usize, last: usize, expected: &str) -> Json {
    let text = fs::read_to_string(root.join("vendor/path-of-building-poe2").join(path))
        .unwrap()
        .replace("\r\n", "\n");
    let full_hash = hash(text.as_bytes());
    assert_eq!(full_hash, pinned::expected_file_sha256(path).unwrap());
    let body = text
        .lines()
        .skip(first - 1)
        .take(last - first + 1)
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(body, expected);
    json!({"path":path,"whole_file_sha256":full_hash,"first_line":first,"last_line":last,"body":body,"body_sha256":hash(body.as_bytes()),"source_newlines":"CRLF normalized to LF; exact pinned manifest contract"})
}

#[test]
#[ignore = "offline validation of already acquired complete raw reports; executes no Lua"]
fn reconcile_retained_mana_supplier_pair_with_exact_acquisition() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let directory =
        PathBuf::from(std::env::var_os("POE_MANA_SUPPLIER_RECHECK").expect("retained directory"));
    let directory = if directory.is_absolute() {
        directory
    } else {
        root.join(directory)
    };
    let read = |name: &str| -> Json {
        serde_json::from_slice(&fs::read(directory.join(name)).unwrap()).unwrap()
    };
    let reports = [read("source-jit-off.json"), read("source-jit-on.json")];
    let mut files = Vec::new();
    for (mode, report) in ["off", "on"].into_iter().zip(&reports) {
        assert_eq!(report["source_revision"], pinned::UPSTREAM_REVISION);
        assert_eq!(report["manifest_sha256"], pinned::manifest_sha256());
        assert_eq!(
            report["xml"]["sha256"],
            hash(&fs::read(root.join(XML)).unwrap())
        );
        assert_eq!(report["complete_loads_per_mode"], 2);
        assert_eq!(report["instrumented_jit_enabled"], false);
        assert_eq!(report["plain_jit_enabled"], mode == "on");
        assert_eq!(report["state"]["original_functions_preserved"], true);
        assert_eq!(report["state"]["observer_removed"], true);
        assert_eq!(
            report["state"]["scope"]["native_coverage_authorized"],
            false
        );
        for family in FAMILIES {
            assert!(!report["state"][family].as_array().unwrap().is_empty());
        }
        for (label, projection) in [
            ("observed", &report["state"]),
            ("plain", &report["plain_snapshot"]),
        ] {
            let name = format!("jit-{mode}-{label}-raw.json");
            let raw = read(&name);
            assert_eq!(raw["source_hash"], pinned::manifest_sha256());
            assert_eq!(raw["configuration_method_wrappers"], false);
            assert_eq!(raw["original_build_output_available"], true);
            equal(
                &raw["additional_observation"],
                projection,
                "exact raw acquisition projection",
            );
        }
        equal(
            &report["state"]["snapshot"],
            &report["plain_snapshot"],
            "observed/plain full scalar output",
        );
        for label in ["observed-raw", "plain-raw"] {
            let name = format!("jit-{mode}-{label}.json");
            let bytes = fs::read(directory.join(&name)).unwrap();
            files.push(json!({"path":name,"bytes":bytes.len(),"sha256":hash(&bytes)}));
        }
        let name = format!("source-jit-{mode}.json");
        let bytes = fs::read(directory.join(&name)).unwrap();
        files.push(json!({"path":name,"bytes":bytes.len(),"sha256":hash(&bytes)}));
    }
    let mut acquisition = Vec::new();
    for (name, field) in [
        ("acquisition-driver.rs", "driver_sha256"),
        ("acquisition-observer.lua", "observer_sha256"),
        ("acquisition-helpers.lua", "helper_sha256"),
        ("acquisition-bootstrap.rs", "bootstrap_sha256"),
    ] {
        let bytes = fs::read(directory.join(name)).unwrap();
        for report in &reports {
            assert_eq!(report[field], hash(&bytes));
        }
        acquisition.push(json!({"path":name,"bytes":bytes.len(),"sha256":hash(&bytes)}));
    }
    // The observer is unchanged; only the subsequent comparison policy evolved.
    assert_eq!(reports[0]["observer_sha256"], hash(OBSERVER.as_bytes()));
    equal(
        &comparison(&reports[0]["state"]),
        &comparison(&reports[1]["state"]),
        "all retained constructor inventories",
    );
    equal(
        &reports[0]["plain_snapshot"],
        &reports[1]["plain_snapshot"],
        "plain JIT scalar outputs",
    );
    equal(
        &reports[0]["comparison"],
        &reports[1]["comparison"],
        "acquired comparison metadata",
    );
    let bodies = vec![
        source_span(
            &root,
            "src/Modules/CalcActiveSkill.lua",
            896,
            901,
            "\tfor _, value in ipairs(env.modDB:List(activeSkill.skillCfg, \"SkillData\")) do\n\t\tactiveSkill.skillData[value.key] = value.value\n\tend\n\tfor _, value in ipairs(skillModList:List(activeSkill.skillCfg, \"SkillData\")) do\n\t\tactiveSkill.skillData[value.key] = value.value\n\tend",
        ),
        source_span(
            &root,
            "src/Modules/CalcPerform.lua",
            1161,
            1167,
            "local function addMinionModifiers(modList, skillCfg, minion)\n\tfor _, value in ipairs(modList:List(skillCfg, \"MinionModifier\")) do\n\t\tif not value.type or minion.type == value.type then\n\t\t\tminion.modDB:AddMod(value.mod)\n\t\tend\n\tend\nend",
        ),
        source_span(
            &root,
            "src/Classes/ModDB.lua",
            31,
            37,
            "function ModDBClass:AddMod(mod)\n\tlocal name = mod.name\n\tif not self.mods[name] then\n\t\tself.mods[name] = { }\n\tend\n\tt_insert(self.mods[name], mod)\nend",
        ),
    ];
    let proof = json!({"schema_version":1,"kind":"retained-mana-supplier-construction-comparison",
        "acquisition":acquisition,"files":files,"comparison_driver_sha256":hash(include_bytes!("owned_mana_supplier_source.rs")),
        "source_consumers":bodies,"counts":FAMILIES.into_iter().map(|f|(f,reports[0]["state"][f].as_array().unwrap().len())).collect::<BTreeMap<_,_>>(),
        "scope":{"new_source_loads":0,"raw_reports_unchanged":true,"all_scalar_outputs_exact":true,"all_record_bodies_and_multiplicities_exact":true,
            "native_coverage_authorized":false,"unresolved_branches_preserved":true,"native_arithmetic_order_authority":false},
        "comparison_policy":{"constructed_visits":"counted inventories","record_lists":"stat buckets preserving same-stat order",
            "constructed_skill_SkillData":"distinct assignment keys; same-key order preserved",
            "constructed_skill_MinionModifier":"distinct nested destination names; same-name order preserved"}});
    let output = directory.join("retained-comparison.json");
    assert!(!output.exists(), "fresh proof path required");
    fs::write(output, serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
}

// CalcActiveSkill.lua:896-900 assigns each SkillData payload to its own key.
// Distinct keys commute; duplicate same-key assignments remain order-sensitive.
// Apply this only to exact constructed-skill inventories, never arbitrary LIST
// modifiers, raw execution order, buff delivery, numeric folds or scalar outputs.
fn skill_data_assignment_inventory(row: &mut Json) {
    let Some(records) = row
        .get_mut("records")
        .and_then(|r| r.get_mut("counted_source_inventory_by_stat"))
        .and_then(|r| r.get_mut("SkillData"))
    else {
        return;
    };
    let Some(values) = records.as_array() else {
        return;
    };
    if !values
        .iter()
        .all(|r| r["name"] == "SkillData" && r["type"] == "LIST" && r["value"]["key"].is_string())
    {
        return;
    }
    let mut keys: BTreeMap<String, Vec<Json>> = BTreeMap::new();
    for r in values {
        keys.entry(r["value"]["key"].as_str().unwrap().to_owned())
            .or_default()
            .push(r.clone());
    }
    *records = json!({"source_assignments_by_key":keys});
}

#[test]
fn skill_data_inventory_preserves_duplicate_key_order_and_full_bodies() {
    let body = |key: &str, value: i32| json!({"name":"SkillData","type":"LIST","value":{"key":key,"value":value},"flags":0});
    let normalize = |rows: Vec<Json>| {
        let mut row = json!({"records":{"counted_source_inventory_by_stat":{"SkillData":rows}}});
        skill_data_assignment_inventory(&mut row);
        row
    };
    let original = vec![body("radius", 1), body("radius", 2), body("PhysicalMin", 3)];
    assert_eq!(
        normalize(original.clone()),
        normalize(vec![
            body("PhysicalMin", 3),
            body("radius", 1),
            body("radius", 2)
        ])
    );
    for rows in [
        vec![body("radius", 2), body("radius", 1), body("PhysicalMin", 3)],
        vec![body("radius", 1), body("PhysicalMin", 3)],
        vec![body("radius", 1), body("radius", 2), body("PhysicalMin", 4)],
    ] {
        assert_ne!(normalize(original.clone()), normalize(rows));
    }
}

#[test]
fn observer_parses_and_inventory_comparison_preserves_same_stat_order() {
    Lua::new().load(OBSERVER).into_function().unwrap();
    let row = |name: &str, value: i32, position: i32| json!({"position":position,"body":{"name":name,"type":"BASE","value":value,"source":"source","flags":0}});
    let original = json!([row("Mana", 1, 1), row("Life", 3, 2), row("Mana", 2, 3)]);
    let normalize = |mut v| {
        inventory_view(&mut v);
        v
    };
    assert_eq!(
        normalize(original.clone()),
        normalize(json!([
            row("Life", 3, 1),
            row("Mana", 1, 2),
            row("Mana", 2, 3)
        ]))
    );
    for changed in [
        json!([row("Mana", 2, 1), row("Life", 3, 2), row("Mana", 1, 3)]),
        json!([row("Mana", 1, 1), row("Life", 3, 2)]),
        json!([row("Mana", 1, 1), row("Life", 4, 2), row("Mana", 2, 3)]),
    ] {
        assert_ne!(normalize(original.clone()), normalize(changed));
    }
}

#[test]
#[ignore = "four complete original PoB loads; bounded source-construction evidence"]
fn unchanged_mana_supplier_construction_is_original_and_repeatable() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = PathBuf::from(std::env::var_os(OUTPUT).expect("fresh output directory"));
    let out = if out.is_absolute() {
        out
    } else {
        root.join(out)
    };
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        child(&root, &out, mode == "on");
        return;
    }
    assert!(!out.exists(), "fresh acquisition directory required");
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let log = fs::File::create(out.join(format!("source-jit-{mode}.log"))).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env(CHILD, mode)
            .env(OUTPUT, &out)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let started = Instant::now();
        loop {
            if let Some(status) = process.try_wait().unwrap() {
                assert!(status.success(), "inspect source-jit-{mode}.log");
                break;
            }
            if started.elapsed() > Duration::from_secs(600) {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!("source deadline; inspect source-jit-{mode}.log");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    let read = |mode| -> Json {
        serde_json::from_slice(&fs::read(out.join(format!("source-jit-{mode}.json"))).unwrap())
            .unwrap()
    };
    let off = read("off");
    let on = read("on");
    equal(
        &comparison(&off["state"]),
        &comparison(&on["state"]),
        "independent fresh constructor observations",
    );
    equal(
        &off["plain_snapshot"],
        &on["plain_snapshot"],
        "plain JIT modes",
    );
}

fn child(root: &Path, out: &Path, jit: bool) {
    let xml = fs::read_to_string(root.join(XML)).unwrap();
    assert_eq!(
        hash(xml.as_bytes()),
        "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089"
    );
    let consumer_path = "src/Modules/CalcActiveSkill.lua";
    let consumer = fs::read_to_string(
        root.join("vendor/path-of-building-poe2")
            .join(consumer_path),
    )
    .unwrap()
    .replace("\r\n", "\n");
    let consumer_hash = hash(consumer.as_bytes());
    assert_eq!(
        consumer_hash,
        pinned::expected_file_sha256(consumer_path).unwrap()
    );
    let body = consumer
        .lines()
        .skip(895)
        .take(6)
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        body,
        "\tfor _, value in ipairs(env.modDB:List(activeSkill.skillCfg, \"SkillData\")) do\n\t\tactiveSkill.skillData[value.key] = value.value\n\tend\n\tfor _, value in ipairs(skillModList:List(activeSkill.skillCfg, \"SkillData\")) do\n\t\tactiveSkill.skillData[value.key] = value.value\n\tend"
    );
    let mut observations = Vec::new();
    for instrumented in [true, false] {
        let label = if instrumented { "observed" } else { "plain" };
        eprintln!("Mana supplier {label}; plain JIT {jit}");
        let before = |lua: &Lua| -> Result<(), RuntimeError> {
            lua.globals().set("manaSupplierJit", jit && !instrumented)?;
            lua.load("if manaSupplierJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| -> Result<Function, RuntimeError> {
            assert!(!lua.load("return jit.status()").eval::<bool>()?);
            let factory: Function = lua
                .load(OBSERVER)
                .set_name("@mana-supplier-observer")
                .eval()?;
            Ok(factory.call(helpers(lua)?)?)
        };
        let after = |lua: &Lua| -> Result<Json, RuntimeError> {
            assert_eq!(
                lua.load("return jit.status()").eval::<bool>()?,
                jit && !instrumented
            );
            let value: Value = if instrumented {
                lua.globals().get("manaSupplierObservation")?
            } else {
                helpers(lua)?
                    .get::<Function>("offeringOutputSnapshot")?
                    .call(())?
            };
            Ok(lua.from_value(value)?)
        };
        let scratch = tempfile::tempdir().unwrap();
        let observed = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &xml,
            None,
            false,
            Some(&before),
            instrumented.then_some(&install),
            Some(&after),
        )
        .unwrap_or_else(|e| panic!("{label}: {e}"));
        fs::write(
            out.join(format!(
                "jit-{}-{label}-raw.json",
                if jit { "on" } else { "off" }
            )),
            serde_json::to_vec(&observed).unwrap(),
        )
        .unwrap();
        assert_eq!(observed["configuration_method_wrappers"], false);
        assert_eq!(observed["source_hash"], pinned::manifest_sha256());
        assert_eq!(observed["original_build_output_available"], true);
        observations.push(observed["additional_observation"].clone());
    }
    equal(
        &observations[0]["snapshot"],
        &observations[1],
        "observer versus unchanged original outputs",
    );
    let report = json!({"schema_version":1,"kind":"original-mana-supplier-construction-evidence",
        "source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "xml":{"path":XML,"sha256":hash(xml.as_bytes())},
        "observer_sha256":hash(OBSERVER.as_bytes()),"driver_sha256":hash(include_bytes!("owned_mana_supplier_source.rs")),
        "helper_sha256":hash(COMMON.as_bytes()),"bootstrap_sha256":hash(include_bytes!("support/configuration_preparation_source.rs")),
        "comparison":{"source_inventory_cross_stat_buckets":true,"construction_visits_are_counted_inventory":true,
            "numeric_outputs_exact":true,"raw_order_retained":true,"same_stat_order_preserved":true,
            "skill_data_distinct_assignment_keys":{"path":consumer_path,"whole_file_sha256":consumer_hash,
                "first_line":896,"last_line":901,"body_sha256":hash(body.as_bytes()),
                "duplicate_key_order_preserved":true,"native_order_authority":false}},
        "complete_loads_per_mode":2,"instrumented_jit_enabled":false,
        "plain_jit_enabled":jit,"state":observations[0],"plain_snapshot":observations[1]});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if jit { "on" } else { "off" }
        )),
        serde_json::to_vec(&report).unwrap(),
    )
    .unwrap();
    let state = &report["state"];
    assert_eq!(state["original_functions_preserved"], true);
    assert_eq!(state["observer_removed"], true);
    assert_eq!(state["scope"]["native_coverage_authorized"], false);
    for family in FAMILIES {
        assert!(
            !state[family].as_array().unwrap().is_empty(),
            "missing {family} evidence"
        );
    }
}
