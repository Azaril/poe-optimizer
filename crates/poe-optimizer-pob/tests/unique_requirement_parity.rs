//! Independent full-source unique construction, dependency and order audit.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/unique_requirement_runtime.rs"]
mod runtime;
use mlua::{Function, LuaSerdeExt, Value as LuaValue};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn observe(mode: &str) -> Value {
    let scratch = tempfile::tempdir().unwrap();
    let output = scratch.path().join("output.json");
    let stdout = scratch.path().join("stdout.txt");
    let stderr = scratch.path().join("stderr.txt");
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "unique_construction_worker",
            "--ignored",
            "--nocapture",
        ])
        .env("POE_UNIQUE_CONSTRUCTION_MODE", mode)
        .env("POE_UNIQUE_CONSTRUCTION_OUTPUT", &output)
        .stdout(Stdio::from(fs::File::create(&stdout).unwrap()))
        .stderr(Stdio::from(fs::File::create(&stderr).unwrap()));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("unique construction worker {mode} exceeded 120 seconds");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(
        status.success(),
        "mode {mode}\nstdout:\n{}\nstderr:\n{}",
        fs::read_to_string(stdout).unwrap(),
        fs::read_to_string(stderr).unwrap()
    );
    assert!(fs::metadata(&output).unwrap().len() <= 16 * 1024 * 1024);
    let bytes = fs::read(output).unwrap();
    if let Some(directory) = std::env::var_os("POE_UNIQUE_CONSTRUCTION_EVIDENCE") {
        let directory = PathBuf::from(directory);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join(format!("{mode}.json")), &bytes).unwrap();
    }
    serde_json::from_slice(&bytes).unwrap()
}

fn bits(value: &Value) -> Value {
    match value {
        Value::Number(number) => {
            serde_json::json!({"f64_bits":format!("{:016x}",number.as_f64().unwrap().to_bits())})
        }
        Value::Array(values) => values.iter().map(bits).collect(),
        Value::Object(values) => values
            .iter()
            .map(|(key, value)| (key.clone(), bits(value)))
            .collect(),
        other => other.clone(),
    }
}
fn normalized_records(result: &Value) -> BTreeMap<(String, u64), Value> {
    let entries = result["entries"].as_object().unwrap();
    let mut records = BTreeMap::new();
    let mut inserted = BTreeSet::new();
    let mut reads = 0;
    let mut potential = 0;
    for record in result["records"].as_array().unwrap() {
        let identity = (
            record["group"].as_str().unwrap().to_owned(),
            record["index"].as_u64().unwrap(),
        );
        let raw = record["raw"].as_str().unwrap();
        let key = record["inserted_key"].as_str().unwrap();
        assert!(
            inserted.insert(key),
            "duplicate constructed canonical key {key}"
        );
        assert_eq!(record["result"]["has_base"], true);
        assert_eq!(record["result"], entries[key]);
        for lookup in record["lookups"].as_array().unwrap() {
            for target in lookup["potential"].as_array().unwrap() {
                potential += 1;
                let target = target.as_str().unwrap();
                assert!(
                    target == key || !entries.contains_key(target),
                    "cross-prototype dependency {identity:?}: {target}"
                );
            }
        }
        for read in record["reads"].as_array().unwrap() {
            reads += 1;
            assert_eq!(read["found"], false, "construction read hit: {record}");
        }
        let normalized = serde_json::json!({"raw_sha256":format!("{:x}",Sha256::digest(raw.as_bytes())),
            "inserted_key":key,"result":bits(&record["result"]),"lookups":record["lookups"]});
        assert!(records.insert(identity, normalized).is_none());
    }
    assert_eq!(records.len(), 443, "pinned-source fixture inventory");
    assert_eq!(inserted.len(), entries.len());
    assert_eq!(result["overwrites"], 0);
    assert_eq!(
        result["insertions"].as_array().unwrap().len(),
        records.len()
    );
    assert_eq!(result["complete"], true);
    eprintln!(
        "{}: {} constructors, {} actual reads, {} potential keys, {} parser calls",
        result["mode"],
        records.len(),
        reads,
        potential,
        result["parse_calls"]
    );
    records
}

#[test]
fn all_pinned_unique_constructors_have_no_cross_entry_requirement_dependency() {
    let actual = observe("actual");
    let baseline = normalized_records(&actual);
    let control = observe("control");
    assert_eq!(
        bits(&actual["entries"]),
        bits(&control["entries"]),
        "observation-only control"
    );
    for mode in ["reverse-groups", "reverse-all", "rotate", "stored-cache"] {
        let result = observe(mode);
        assert_eq!(
            normalized_records(&result),
            baseline,
            "construction schedule/cache: {mode}"
        );
    }
    let warm = observe("warm");
    assert_eq!(
        normalized_records(&warm[0]),
        baseline,
        "cold half of warm witness"
    );
    assert_eq!(
        normalized_records(&warm[1]),
        baseline,
        "retained original parser cache"
    );
}

#[test]
#[ignore = "isolated full-source construction worker invoked by parent"]
fn unique_construction_worker() {
    let mode = std::env::var("POE_UNIQUE_CONSTRUCTION_MODE").unwrap();
    let output = PathBuf::from(std::env::var_os("POE_UNIQUE_CONSTRUCTION_OUTPUT").unwrap());
    let runtime = runtime::UniqueRuntime::new();
    let run: Function = runtime.lua.globals().get("runUniqueConstruction").unwrap();
    let observe = |mode: &str| -> Value {
        runtime
            .lua
            .from_value(run.call::<LuaValue>(mode).unwrap())
            .unwrap()
    };
    let result = match mode.as_str() {
        "warm" => serde_json::json!([observe("actual"), observe("reverse-all")]),
        "stored-cache" => {
            runtime
                .lua
                .globals()
                .get::<Function>("loadOriginalStoredCache")
                .unwrap()
                .call::<()>(())
                .unwrap();
            observe("actual")
        }
        "witness-duplicate" => {
            runtime
                .lua
                .load(
                    r#"data.uniques={Witness={
                "Oracle duplicate\nIron Ring\nRequires Level 10",
                "Oracle duplicate\nIron Ring\nRequires Level 20"}}"#,
                )
                .exec()
                .unwrap();
            serde_json::json!([observe("actual"), observe("reverse-all")])
        }
        "witness-runic" => {
            runtime
                .lua
                .load(
                    r#"data.itemBases["Runeforged Iron Ring"]=copyTable(data.itemBases["Iron Ring"])
                data.uniques={Witness={
                "Oracle linked\nIron Ring\nRequires Level 40",
                "Oracle linked\nRuneforged Iron Ring\nRequires Level 10"}}"#,
                )
                .exec()
                .unwrap();
            serde_json::json!([observe("actual"), observe("reverse-all")])
        }
        "witness-missing" => {
            runtime
                .lua
                .load(r#"data.uniques={Witness={"Oracle missing\nNot a source base"}}"#)
                .exec()
                .unwrap();
            observe("actual")
        }
        "witness-error" => {
            runtime.lua.load(r#"data.itemBases["Oracle broken ring"]=copyTable(data.itemBases["Iron Ring"])
                data.itemBases["Oracle broken ring"].req.level="invalid requirement type"
                data.uniques={Witness={"Oracle valid\nIron Ring","Oracle failed\nOracle broken ring"}}"#).exec().unwrap();
            runtime
                .lua
                .from_value(
                    runtime
                        .lua
                        .load(
                            r#"local ok,err=pcall(runUniqueConstruction,'actual')
                return {ok=ok,error=not ok and err or nil,loading=main.uniqueDB.loading}"#,
                        )
                        .eval::<LuaValue>()
                        .unwrap(),
                )
                .unwrap()
        }
        "lookups" => runtime
            .lua
            .from_value(
                runtime
                    .lua
                    .load(include_str!("support/unique_requirement_lookup.lua"))
                    .eval::<LuaValue>()
                    .unwrap(),
            )
            .unwrap(),
        "numbers" => {
            runtime
                .lua
                .globals()
                .set(
                    "oracleNumberBits",
                    runtime
                        .lua
                        .create_function(|_, value: LuaValue| {
                            Ok(match value {
                                LuaValue::Nil => "nil".to_owned(),
                                LuaValue::Integer(number) => {
                                    format!("{:016x}", (number as f64).to_bits())
                                }
                                LuaValue::Number(number) => format!("{:016x}", number.to_bits()),
                                _ => {
                                    return Err(mlua::Error::RuntimeError(
                                        "unexpected requirement type".into(),
                                    ));
                                }
                            })
                        })
                        .unwrap(),
                )
                .unwrap();
            runtime
                .lua
                .from_value(
                    runtime
                        .lua
                        .load(include_str!("support/unique_requirement_numbers.lua"))
                        .eval::<LuaValue>()
                        .unwrap(),
                )
                .unwrap()
        }
        other => observe(other),
    };
    fs::write(output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
}

#[test]
fn original_construction_observer_detects_order_dependencies_and_missing_bases() {
    let duplicates = observe("witness-duplicate");
    for pass in duplicates.as_array().unwrap() {
        assert_eq!(pass["overwrites"], 1);
        assert_eq!(pass["records"].as_array().unwrap().len(), 2);
        assert_eq!(pass["insertions"].as_array().unwrap().len(), 2);
    }
    assert_ne!(
        duplicates[0]["entries"], duplicates[1]["entries"],
        "duplicate natural requirement depends on source order"
    );
    let runic = observe("witness-runic");
    for pass in runic.as_array().unwrap() {
        assert_eq!(
            pass["overwrites"], 0,
            "runic dependency requires no canonical collision"
        );
        assert_eq!(pass["entries"].as_object().unwrap().len(), 2);
        let record = pass["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|record| record["inserted_key"] == "Oracle linked, Runeforged Iron Ring")
            .unwrap();
        assert!(record["lookups"].as_array().unwrap().iter().any(|lookup| {
            lookup["potential"]
                .as_array()
                .unwrap()
                .iter()
                .any(|key| key == "Oracle linked, Iron Ring")
        }));
    }
    assert_ne!(
        runic[0]["entries"], runic[1]["entries"],
        "fallback dependency changes requirement without a collision"
    );
    let missing = observe("witness-missing");
    assert_eq!(missing["complete"], true);
    assert_eq!(missing["records"].as_array().unwrap().len(), 1);
    assert_eq!(missing["records"][0]["result"]["has_base"], false);
    assert!(missing["records"][0].get("inserted_key").is_none());
    assert!(missing["insertions"].as_object().unwrap().is_empty());
    let failed = observe("witness-error");
    assert_eq!(failed["ok"], false);
    assert_eq!(
        failed["loading"], true,
        "failed construction never clears original readiness"
    );
    assert!(
        failed["error"]
            .as_str()
            .unwrap()
            .contains("number expected, got string")
    );
}

#[test]
fn original_unique_requirement_numeric_witnesses_are_explicit() {
    let result = observe("numbers");
    let expected = [
        Some((42.0_f64, 42.0_f64)),
        Some((0.0, 0.0)),
        Some((0.0, 0.0)),
        Some((42.0, 42.0)),
        None,
        Some((0.0, 0.0)),
        Some((42.0, 42.0)),
        Some((f64::INFINITY, f64::INFINITY)),
        Some((0.0, 0.0)),
        Some((5.0, 6.0)),
        Some((0.0, 0.0)),
        Some((0.0, 0.0)),
        Some((5.0, f64::INFINITY)),
        Some((5.0, 5.0)),
    ];
    let cases = result["cases"].as_array().unwrap();
    assert_eq!(cases.len(), expected.len());
    for (actual, expected) in cases.iter().zip(expected) {
        if let Some((natural, level)) = expected {
            assert_eq!(actual["ok"], true, "{actual}");
            assert_eq!(
                actual["natural"],
                format!("{:016x}", natural.to_bits()),
                "{actual}"
            );
            assert_eq!(
                actual["level"],
                format!("{:016x}", level.to_bits()),
                "{actual}"
            );
        } else {
            assert_eq!(actual["ok"], false);
            assert!(
                actual["error"]
                    .as_str()
                    .unwrap()
                    .contains("number expected, got nil")
            );
        }
    }
    let number =
        |value: &Value| f64::from_bits(u64::from_str_radix(value.as_str().unwrap(), 16).unwrap());
    assert_eq!(result["rune_operands"][0]["ok"], true);
    assert!(number(&result["rune_operands"][0]["level"]).is_nan());
    assert_eq!(result["rune_operands"][1]["level"], "8000000000000000");
    assert_eq!(result["rune_operands"][2]["ok"], false);
    assert!(
        result["rune_operands"][2]["error"]
            .as_str()
            .unwrap()
            .contains("number expected, got nil")
    );
    let pairs = result["max"].as_array().unwrap();
    assert_eq!(pairs.len(), 36);
    for pair in pairs {
        let (a, b) = (number(&pair["a"]), number(&pair["b"]));
        // The source primitive selects its right argument on ties/unordered input.
        let expected = if a > b { a } else { b };
        assert_eq!(
            pair["result"],
            format!("{:016x}", expected.to_bits()),
            "{pair}"
        );
    }
    assert_eq!(
        result["triples"][0]["result"],
        format!("{:016x}", 42.0_f64.to_bits())
    );
    assert_eq!(result["triples"][1]["result"], "0000000000000000");
    assert!(number(&result["triples"][2]["result"]).is_nan());
    assert_eq!(result["triples"][3]["result"], "8000000000000000");
}

#[test]
fn original_requirement_numbers_match_the_native_item_machine_boundary() {
    use poe_optimizer_import::item_loading::*;
    struct FixedUnique(UniqueOutcome);
    impl ItemLoadProvider for FixedUnique {
        fn lookup_unique(&mut self, _: &UniqueRequest) -> DependencyResult<Option<UniqueOutcome>> {
            DependencyResult::Available(Some(self.0.clone()))
        }
    }
    fn input(value: &Value) -> ItemNumber {
        match value.as_str().unwrap() {
            "nil" => ItemNumber::Nil,
            bits => ItemNumber::new(f64::from_bits(u64::from_str_radix(bits, 16).unwrap())),
        }
    }
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let observed = observe("numbers");
    for row in observed["cases"].as_array().unwrap() {
        let mut provider = FixedUnique(UniqueOutcome {
            // Explicit Nil is a supported provider transport spelling; source
            // nil remains absent in its own database table.
            natural_level: Some(input(&row["db_natural"])),
            level: Some(input(&row["db_level"])),
        });
        let mut raw = "Rarity: UNIQUE\nOracle numeric\nIron Ring".to_owned();
        if let Some(authored) = row["authored"].as_str() {
            raw.push_str("\nLevelReq: ");
            raw.push_str(authored);
        }
        let mut machine = ItemLoadMachine::new(snapshot.item_loading());
        let result = machine.apply_text(&raw, &mut provider);
        if row["ok"] == false {
            assert!(result.is_err(), "{row}");
            assert_eq!(machine.status(), ItemLoadStatus::SourceError);
        } else {
            result.unwrap();
            assert_eq!(
                machine.pending().map(|pending| pending.kind),
                Some(DependencyKind::Assembly),
                "{row}"
            );
            for (native, source) in [("naturalLevel", "natural"), ("level", "level")] {
                let actual = machine.state().requirements[native].value().unwrap();
                assert_eq!(
                    format!("{:016x}", actual.to_bits()),
                    row[source],
                    "{native}: {row}"
                );
            }
        }
    }
}

#[test]
fn all_constructed_unique_entries_match_the_injected_catalog_and_native_provider() {
    use poe_optimizer_data::unique_requirements::UniqueRequirementLookup;
    use poe_optimizer_import::item_loading::*;
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let data = snapshot.package().unique_requirements.complete().unwrap();
    let source = observe("stored-cache");
    let source_records = normalized_records(&source);
    assert_eq!(data.entries.len(), source_records.len());
    assert_eq!(data.prototypes.len(), source_records.len());
    let mut provider = BuiltinItemLoadProvider::new(&snapshot);
    for entry in &data.entries {
        let record = &source_records[&(
            entry.prototype.group.clone(),
            u64::from(entry.prototype.index),
        )];
        assert_eq!(record["raw_sha256"], entry.prototype.raw_sha256);
        assert_eq!(record["inserted_key"], entry.canonical_key);
        let observed = &source["entries"][&entry.canonical_key];
        assert_eq!(observed["base_name"], entry.base_name);
        for (expected, field) in [(entry.natural_level, "natural"), (entry.level, "level")] {
            match expected {
                Some(expected) => {
                    assert_eq!(observed[field]["kind"], "number");
                    assert_eq!(
                        expected.to_bits(),
                        observed[field]["value"].as_f64().unwrap().to_bits(),
                        "{} {field}",
                        entry.canonical_key
                    );
                }
                None => assert_eq!(observed[field]["kind"], "nil"),
            }
        }
        assert_eq!(
            snapshot
                .unique_requirements()
                .lookup(&entry.canonical_key, None, None),
            UniqueRequirementLookup::Ready(Some(entry))
        );
        let request = UniqueRequest {
            name: entry.canonical_key.clone(),
            title: None,
            base_name: None,
        };
        let DependencyResult::Available(Some(actual)) = provider.lookup_unique(&request) else {
            panic!("missing native lookup {}", entry.canonical_key)
        };
        for (actual, expected) in [
            (actual.natural_level, entry.natural_level),
            (actual.level, entry.level),
        ] {
            assert_eq!(actual.is_some(), expected.is_some());
            if let (Some(actual), Some(expected)) = (actual, expected) {
                assert_eq!(actual.value().unwrap().to_bits(), expected.to_bits());
            }
        }
    }
}

#[test]
fn original_lookup_matches_native_exact_runic_and_injected_edge_cases() {
    use poe_optimizer_data::unique_requirements::{
        UniquePrototypeDisposition, UniqueRequirementCatalog, UniqueRequirementLookup,
    };
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let source = observe("lookups");
    let split = source["original_count"].as_u64().unwrap() as usize;
    let rows = source["rows"].as_array().unwrap();
    assert_eq!(split, 443 * 4 + 1);
    let compare = |catalog: &UniqueRequirementCatalog, row: &Value| {
        let UniqueRequirementLookup::Ready(actual) = catalog.lookup(
            row["name"].as_str().unwrap(),
            row["title"].as_str(),
            row["base_name"].as_str(),
        ) else {
            panic!("unexpected unavailable lookup")
        };
        assert_eq!(
            actual.map(|entry| entry.canonical_key.as_str()),
            row["found_key"].as_str(),
            "{row}"
        );
    };
    for row in &rows[..split] {
        compare(snapshot.unique_requirements(), row);
    }
    // Authored test catalog updates use the same explicit input records injected
    // into the source DB. They are not claimed as source-extracted provenance.
    let mut authored = snapshot.package().unique_requirements.clone();
    let complete = authored.complete_mut().unwrap();
    for injection in source["injected"].as_array().unwrap() {
        let old = injection["old_key"].as_str().unwrap();
        let new = injection["new_key"].as_str().unwrap();
        let entry = complete
            .entries
            .iter_mut()
            .find(|entry| entry.canonical_key == old)
            .unwrap();
        entry.canonical_key = new.into();
        entry.natural_level = Some(
            injection["natural"]
                .as_str()
                .unwrap()
                .parse::<f64>()
                .unwrap(),
        );
        entry.level = injection["level"].as_f64();
        for prototype in &mut complete.prototypes {
            if let UniquePrototypeDisposition::Inserted { canonical_key } =
                &mut prototype.disposition
                && canonical_key == old
            {
                *canonical_key = new.into();
            }
            for key in &mut prototype.lookup_keys {
                if key == old {
                    *key = new.into();
                }
            }
            prototype.lookup_keys.sort();
        }
    }
    complete
        .entries
        .sort_by(|a, b| a.canonical_key.cmp(&b.canonical_key));
    let catalog = UniqueRequirementCatalog::new(authored).unwrap();
    for row in &rows[split..] {
        compare(&catalog, row);
    }
    assert_eq!(rows.len() - split, 15);
    let UniqueRequirementLookup::Ready(Some(zero)) = catalog.lookup("Oracle key, ", None, None)
    else {
        panic!("missing injected zero")
    };
    assert_eq!(zero.natural_level.unwrap().to_bits(), (-0.0_f64).to_bits());
}
