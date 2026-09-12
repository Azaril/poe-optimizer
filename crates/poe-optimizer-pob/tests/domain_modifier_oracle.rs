//! Original modifier observations across unchanged complete builds, independent of execution-model choice.
//! Source-only conformance evidence; this test grants no native or alternative-model admission.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Function, LuaSerdeExt, Value};
use poe_optimizer_pob::runtime::RuntimeError;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "original_domain_modifiers_are_observed_on_all_five_builds";
const MANIFEST: &str = "tests/fixtures/builds/breadth-20260908/index.json";
const OBSERVER_PATH: &str = "tests/support/domain_modifier_observer.lua";
const OBSERVER: &str = include_str!("support/domain_modifier_observer.lua");
const OUTPUT_ENV: &str = "POE_DOMAIN_MODIFIER_OUTPUT";
const CHILD_ENV: &str = "POE_DOMAIN_MODIFIER_CHILD";
const TIMEOUT_ENV: &str = "POE_DOMAIN_MODIFIER_TIMEOUT_SECONDS";

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn specification() -> Json {
    // Probe inputs only: no expected parser records, sums or outcomes are supplied to Lua.
    json!({
        "stat":"ColdResist", "spelling":"Cold Resistance",
        "condition":"DexHigherThanInt",
        "condition_suffix":"if Dexterity is higher than Intelligence",
        "condition_left":"Dex", "condition_right":"Int",
        "per_stat":"Dex", "per_stat_suffix":"per 10 Dexterity",
        "per_stat_divisor":10, "amount":7
    })
}

fn saved_context(xml: &str) -> Json {
    let document = roxmltree::Document::parse(xml).unwrap();
    let root = document.root_element();
    let section = |name| {
        root.children()
            .find(|node| node.has_tag_name(name))
            .unwrap_or_else(|| panic!("missing original {name}"))
    };
    let id = |section_name, attribute| {
        section(section_name)
            .attribute(attribute)
            .unwrap()
            .parse::<u64>()
            .unwrap()
    };
    let selected = json!({
        "skills":id("Skills","activeSkillSet"),
        "items":id("Items","activeItemSet"),
        "config":id("Config","activeConfigSet"),
        "passives":id("Tree","activeSpec")
    });
    let items = section("Items");
    let active = selected["items"].as_u64().unwrap();
    let sets: Vec<_> = items
        .children()
        .filter(|node| node.has_tag_name("ItemSet"))
        .filter(|node| node.attribute("id").unwrap().parse::<u64>().unwrap() == active)
        .collect();
    assert_eq!(sets.len(), 1, "one actual saved active item set");
    let mut slots = BTreeMap::new();
    for node in sets[0].children().filter(|node| node.has_tag_name("Slot")) {
        let name = node.attribute("name").unwrap().to_owned();
        let item_id = node.attribute("itemId").unwrap().parse::<u64>().unwrap();
        assert!(
            slots
                .insert(
                    name,
                    json!({"item_id":item_id,"active":node.attribute("active")})
                )
                .is_none()
        );
    }
    assert!(!slots.is_empty());
    json!({"selected":selected,"slots":slots,"item_set_attributes":{
        "useSecondWeaponSet":sets[0].attribute("useSecondWeaponSet")
    }})
}

// mlua serializes an empty unmarked Lua sequence as an empty object.
// Accept that one representation without treating missing or nonempty maps as arrays.
fn sequence(value: &Json) -> &[Json] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(
            value.as_object().is_some_and(|map| map.is_empty()),
            "expected sequence: {value}"
        );
        &[]
    }
}

fn true_fields(value: &Json, names: &[&str]) {
    for name in names {
        assert_eq!(value[*name], true, "required {name}: {value}");
    }
}

fn number(value: &Json) -> f64 {
    assert_eq!(
        value["kind"], "number",
        "expected actual numeric result: {value}"
    );
    let number = value["value"].as_f64().unwrap();
    assert!(number.is_finite());
    number
}

fn records(value: &Json, stat: Option<&str>) {
    let rows = sequence(value);
    assert!(rows.len() <= 4096);
    for row in rows {
        let fields = row["fields"].as_object().unwrap();
        assert!(fields.len() <= 64);
        assert!(
            fields
                .values()
                .all(|field| field.is_string() || field.is_boolean() || field.is_number())
        );
        assert!(fields["name"].is_string());
        assert!(fields["type"].is_string());
        if let Some(stat) = stat {
            assert_eq!(fields["name"], stat);
        }
        let tags = sequence(&row["tags"]);
        assert!(tags.len() <= 16);
        for tag in tags {
            let tag = tag.as_object().unwrap();
            assert!(tag.len() <= 16);
            assert!(
                tag.values()
                    .all(|field| field.is_string() || field.is_boolean() || field.is_number())
            );
        }
    }
}

fn parser_result(value: &Json) {
    assert!(value["n"].as_u64().is_some_and(|n| n <= 2));
    match value["modifiers"]["kind"].as_str().unwrap() {
        "modifiers" => records(&value["modifiers"]["records"], None),
        "nil" => assert!(value["modifiers"]["value"].is_null()),
        other => panic!("unexpected original parser result kind {other}: {value}"),
    }
    match value["extra"]["kind"].as_str().unwrap() {
        "nil" => assert!(value["extra"]["value"].is_null()),
        "string" => assert!(value["extra"]["value"].is_string()),
        other => panic!("unexpected original remainder kind {other}: {value}"),
    }
}

fn query(value: &Json) -> f64 {
    records(&value["records"], None);
    let evaluated = sequence(&value["evaluated"]);
    assert_eq!(evaluated.len(), sequence(&value["records"]).len());
    for entry in evaluated {
        if entry["kind"] != "nil" {
            number(entry);
        }
    }
    number(&value["sum"])
}

fn history(value: &Json) {
    assert!(value["input"].as_str().is_some_and(|text| !text.is_empty()));
    parser_result(&value["result"]);
    assert_eq!(value["cache_row"]["kind"], "table");
    assert_eq!(value["cache_row"]["first"], value["result"]["modifiers"]);
    assert_eq!(value["cache_row"]["extra"], value["result"]["extra"]);
    true_fields(
        value,
        &[
            "hit_equal",
            "hit_row_identity",
            "independent_return_tables",
            "independent_records_and_tags",
            "return_edit_isolated",
            "eviction_equal",
            "eviction_new_row",
        ],
    );
}

fn validate(root: &Path, report: &Json) {
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["native_admission"], false);
    assert!(report["architecture_decision"].is_null());
    assert_eq!(
        report["source"]["upstream_revision"],
        poe_optimizer_pob::source::UPSTREAM_REVISION
    );
    assert_eq!(
        report["source"]["manifest_sha256"],
        poe_optimizer_pob::source::manifest_sha256()
    );
    assert_eq!(
        report["trace"]["source_hash"],
        report["source"]["manifest_sha256"]
    );
    assert_eq!(report["trace"]["selected"], report["saved"]["selected"]);
    let observed = &report["trace"]["additional_observation"];
    assert_eq!(observed["schema_version"], 1);
    assert_eq!(observed["spec"], specification());
    assert_eq!(observed["selected"], report["saved"]["selected"]);
    assert_eq!(observed["scope"]["source_only"], true);
    for field in [
        "native_alternative",
        "full_actor_effect",
        "import_parser_trace",
        "cache_layout_restored",
    ] {
        assert_eq!(observed["scope"][field], false, "scope {field}");
    }
    assert!(
        observed["scope"]["query_context"]
            .as_str()
            .unwrap()
            .contains("isolated ModList fragment")
    );
    assert!(
        observed["scope"]["cache_restoration"]
            .as_str()
            .unwrap()
            .contains("raw")
    );
    assert!(
        sequence(&observed["frontiers"]).is_empty(),
        "source observer frontier: {}",
        observed["frontiers"]
    );
    for (name, path) in [
        ("parser", "Modules/ModParser.lua"),
        ("set_source", "Modules/ModTools.lua"),
        ("ranged_helper", "Classes/Item.lua"),
        ("build_mod_list", "Classes/Item.lua"),
        ("formatter", "Modules/ItemTools.lua"),
        ("copy", "Modules/Common.lua"),
        ("sum", "Classes/ModStore.lua"),
        ("eval_mod", "Classes/ModStore.lua"),
        ("add_mod", "Classes/ModList.lua"),
    ] {
        let proof = &observed["functions"][name];
        assert_eq!(proof["what"], "Lua", "original {name}");
        assert_eq!(proof["source"], format!("@{path}"), "original {name}");
        let source = poe_optimizer_pob::source::read_verified_text(
            &root.join("vendor/path-of-building-poe2"),
            &format!("src/{path}"),
        )
        .unwrap();
        let first = proof["first_line"].as_u64().unwrap();
        let last = proof["last_line"].as_u64().unwrap();
        assert!(
            first > 0 && last >= first && last <= source.lines().count() as u64,
            "original {name} span"
        );
        assert!(
            source
                .lines()
                .nth(first as usize - 1)
                .unwrap()
                .contains("function"),
            "original {name} declaration"
        );
    }
    let stat = observed["spec"]["stat"].as_str().unwrap();
    let saved = report["saved"]["slots"].as_object().unwrap();
    let slots = sequence(&observed["slots"]);
    assert!(slots.len() >= saved.len() && slots.len() <= 128);
    // The original source fills default slots (for example jewel sockets) that
    // the XML does not list. Preserve them while checking every saved occurrence.
    let mut seen = BTreeMap::new();
    let mut lines = 0;
    for slot in slots {
        let name = slot["slot"].as_str().unwrap();
        assert!(seen.insert(name, slot).is_none(), "duplicate slot {name}");
        if let Some(saved_slot) = saved.get(name) {
            assert_eq!(
                slot["item_id"], saved_slot["item_id"],
                "saved item for {name}"
            );
            if let Some(active) = saved_slot["active"].as_str() {
                assert_eq!(
                    slot["saved_active"],
                    json!({"kind":"boolean", "value":active.parse::<bool>().unwrap()})
                );
            }
        } else {
            assert_eq!(slot["item_id"], 0, "unsaved default slot {name}");
            assert_eq!(slot["effective_item_present"], false);
            assert_eq!(slot["effective_equipped"], false);
            assert!(sequence(&slot["family_records"]).is_empty());
            assert!(sequence(&slot["lines"]).is_empty());
        }
        assert_eq!(
            slot["saved_active"], slot["live_active"],
            "active state for {name}"
        );
        assert_eq!(slot["live_slot_present"], true);
        assert_eq!(
            slot["live_item_id"], slot["item_id"],
            "live item for {name}"
        );
        assert_eq!(slot["saved_live_binding_equal"], true);
        assert_eq!(slot["empty"], slot["item_id"] == 0);
        assert!(slot["inactive"].is_boolean());
        assert!(slot["effective_equipped"].is_boolean());
        assert!(slot["effective_item_present"].is_boolean());
        records(&slot["family_records"], Some(stat));
        for line in sequence(&slot["lines"]) {
            lines += 1;
            assert!(line["text"].is_string());
            assert!(line["index"].as_u64().is_some_and(|n| n > 0));
            records(&line["retained_records"], Some(stat));
            assert!(!sequence(&line["retained_records"]).is_empty());
            assert_eq!(line["replayed_matches_retained"], true);
            assert!(line["frontier"].is_null());
            let calls = sequence(&line["ranged_parser_calls"]);
            if let Some(call) = calls.last() {
                assert!(calls.len() <= 64);
                assert_eq!(
                    line["origin"],
                    "actual original getRangedModList parser argument on component replay"
                );
                assert_eq!(line["hooked_unhooked_equal"], true);
                assert_eq!(line["formatted_input"], call["line"]);
                records(&line["replayed_records"], None);
            } else {
                assert_eq!(
                    line["origin"],
                    "retained cleaned line replay; historical import argument not observed"
                );
                assert_eq!(line["formatted_input"]["value"], line["text"]);
                parser_result(&line["replay_result"]);
            }
            assert_eq!(line["formatted_input"]["kind"], "string");
            query(&line["isolated_fragment"]);
        }
    }
    for name in saved.keys() {
        assert!(
            seen.contains_key(name.as_str()),
            "missing saved slot {name}"
        );
    }
    // Selected witnesses in the pinned, unmodified XML corpus. These identify
    // source item occurrences; they do not supply parser records or range values.
    let ordinal = report["input"]["ordinal"].as_u64().unwrap();
    let exemplars: &[(&str, u64)] = match ordinal {
        1 => &[("Gloves", 4)],
        2 => &[("Helmet", 20)],
        3 => &[("Boots", 11)],
        4 => &[("Helmet", 21)],
        5 => &[("Ring 1", 26), ("Ring 2", 26)],
        _ => panic!("unknown corpus build {ordinal}"),
    };
    for &(name, item_id) in exemplars {
        let slot = seen[name];
        assert_eq!(slot["item_id"], item_id, "selected exemplar {name}");
        assert_eq!(slot["effective_equipped"], true);
        assert_eq!(slot["effective_item_id"], item_id);
        assert!(
            !sequence(&slot["family_records"]).is_empty(),
            "family exemplar {name}"
        );
        assert!(!sequence(&slot["lines"]).is_empty(), "line exemplar {name}");
        if ordinal == 5 {
            assert_eq!(slot["list_identity"], "per-slot source-built list");
            assert_eq!(slot["slot_num"], if name == "Ring 1" { 1 } else { 2 });
            for record in sequence(&slot["family_records"]) {
                assert_eq!(record["fields"]["sourceSlot"], name);
            }
            assert!(
                sequence(&slot["lines"])
                    .iter()
                    .any(|line| !sequence(&line["ranged_parser_calls"]).is_empty()),
                "actual ranged parser call for {name}"
            );
        }
    }
    assert!(
        lines > 0,
        "each complete build must contribute actual selected-family item lines"
    );
    let histories = &observed["histories"];
    history(&histories["positive"]);
    let positive = &histories["positive"]["result"];
    assert_eq!(positive["modifiers"]["kind"], "modifiers");
    assert_eq!(positive["extra"]["kind"], "nil");
    let positive_rows = sequence(&positive["modifiers"]["records"]);
    assert!(!positive_rows.is_empty());
    records(&positive["modifiers"]["records"], Some(stat));
    let base: f64 = positive_rows
        .iter()
        .map(|row| row["fields"]["value"].as_f64().unwrap())
        .sum();
    assert!(base > 0.0);
    let no_match = sequence(&histories["no_match"]);
    assert_eq!(no_match.len(), 2);
    assert_eq!(no_match[0]["result"]["modifiers"]["kind"], "nil");
    assert_eq!(no_match[1]["result"]["modifiers"]["kind"], "modifiers");
    for entry in no_match {
        history(entry);
        assert_eq!(entry["result"]["n"], 2);
        assert_eq!(entry["result"]["extra"]["kind"], "string");
        assert!(
            entry["result"]["extra"]["value"]
                .as_str()
                .is_some_and(|text| !text.is_empty())
        );
        if entry["result"]["modifiers"]["kind"] == "modifiers" {
            assert!(sequence(&entry["result"]["modifiers"]["records"]).is_empty());
        }
    }
    let errors = sequence(&histories["errors"]);
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0]["input"]["kind"], "nil");
    assert_eq!(errors[1]["input"], json!({"kind":"boolean","value":false}));
    for entry in errors {
        true_fields(entry, &["failed", "existing_row_unchanged"]);
        assert!(
            entry["message"]
                .as_str()
                .is_some_and(|text| !text.is_empty())
        );
    }
    let derived = &observed["derived"];
    parser_result(&derived["conditional_result"]);
    parser_result(&derived["per_stat_result"]);
    let conditional = sequence(&derived["conditional"]);
    assert_eq!(conditional.len(), 3);
    for (entry, relation) in conditional.iter().zip(["below", "tie", "above"]) {
        assert_eq!(entry["relation"], relation);
        let condition = entry["left"].as_f64().unwrap() > entry["right"].as_f64().unwrap();
        assert_eq!(entry["condition"], condition);
        assert_eq!(
            entry["context_origin"],
            "controlled resolved-condition input; original attribute stage not rerun"
        );
        assert_eq!(query(&entry["result"]), if condition { base } else { 0.0 });
    }
    let per_stat = sequence(&derived["per_stat"]);
    assert_eq!(per_stat.len(), 5);
    let divisor = observed["spec"]["per_stat_divisor"].as_f64().unwrap();
    for (entry, input) in per_stat.iter().zip([
        divisor - 1.0,
        divisor,
        2.0 * divisor - 1.0,
        2.0 * divisor,
        divisor - 1.0,
    ]) {
        assert_eq!(entry["stat"], observed["spec"]["per_stat"]);
        assert_eq!(entry["value"].as_f64().unwrap(), input);
        assert_eq!(entry["same_record"], true);
        assert_eq!(query(&entry["result"]), (input / divisor).floor() * base);
    }
    assert_eq!(per_stat[0]["result"], per_stat[4]["result"]);
    assert_eq!(derived["per_stat_restored_result"], true);
    let restoration = &observed["restoration"];
    assert!(restoration["cache_entries"].as_u64().is_some_and(|n| n > 0));
    true_fields(
        restoration,
        &[
            "cache_identity",
            "cache_entries_restored",
            "observed_cache_rows_unchanged",
            "selected_records_unchanged",
            "source_functions",
            "modlist_methods",
            "build_bindings",
            "selected",
            "active_set",
            "main_output",
            "watched_item_fields",
            "hook",
        ],
    );
}

fn child(root: &Path, destination: &Path, ordinal: usize, manifest: &Json) {
    let entry = &manifest["builds"][ordinal - 1];
    let filename = format!("build-{ordinal:02}.xml");
    assert_eq!(entry["xml"], filename);
    let xml_path = root
        .join("tests/fixtures/builds/breadth-20260908")
        .join(filename);
    let bytes = fs::read(&xml_path).unwrap();
    assert_eq!(bytes.len() as u64, entry["xml_bytes"].as_u64().unwrap());
    assert_eq!(hash(&bytes), entry["xml_sha256"]);
    let xml = std::str::from_utf8(&bytes).unwrap();
    let saved = saved_context(xml);
    let spec = specification();
    let scratch = tempfile::tempdir().unwrap();
    let pob_root = root.join("vendor/path-of-building-poe2");
    let trace = source::observe_with_hook(
        &pob_root,
        scratch.path(),
        xml,
        None,
        false,
        Some(&|lua| -> Result<Json, RuntimeError> {
            let observer: Function = lua
                .load(OBSERVER)
                .set_name(format!("@{OBSERVER_PATH}"))
                .eval()?;
            let input = lua.to_value_with(
                &spec,
                mlua::serde::SerializeOptions::new().serialize_none_to_null(false),
            )?;
            let observed: Value = observer.call(input)?;
            Ok(lua.from_value(observed)?)
        }),
    )
    .unwrap();
    assert_eq!(fs::read(&xml_path).unwrap(), bytes, "original XML changed");
    assert_eq!(
        poe_optimizer_pob::source::verify(&pob_root).unwrap(),
        poe_optimizer_pob::source::manifest_sha256()
    );
    let report = json!({
        "schema_version":1,"status":"source_observation",
        "scope":"complete_original_build_then_original_modifier_observations",
        "native_admission":false,"architecture_decision":null,
        "input":{"id":entry["id"],"ordinal":ordinal,"xml":entry["xml"],"xml_sha256":hash(&bytes),
            "manifest":MANIFEST,"manifest_sha256":hash(&fs::read(root.join(MANIFEST)).unwrap())},
        "source":{"upstream_revision":poe_optimizer_pob::source::UPSTREAM_REVISION,
            "manifest_sha256":poe_optimizer_pob::source::manifest_sha256(),
            "observer":OBSERVER_PATH,"observer_sha256":hash(OBSERVER.as_bytes())},
        "specification":spec,"saved":saved,"trace":trace
    });
    let output = destination.join(format!("build-{ordinal:02}.json"));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .unwrap();
    file.write_all(&serde_json::to_vec_pretty(&report).unwrap())
        .unwrap();
    validate(root, &report);
}

#[test]
fn original_domain_modifiers_are_observed_on_all_five_builds() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let manifest_bytes = fs::read(root.join(MANIFEST)).unwrap();
    let manifest: Json = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["builds"].as_array().unwrap().len(), 5);
    let scratch = tempfile::tempdir().unwrap();
    let destination = std::env::var_os(OUTPUT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| scratch.path().to_owned());
    let destination = if destination.is_absolute() {
        destination
    } else {
        root.join(destination)
    };
    fs::create_dir_all(&destination).unwrap();
    let destination = destination.canonicalize().unwrap();
    if let Ok(ordinal) = std::env::var(CHILD_ENV) {
        let ordinal: usize = ordinal.parse().unwrap();
        assert!((1..=5).contains(&ordinal));
        child(&root, &destination, ordinal, &manifest);
        return;
    }
    let timeout = std::env::var(TIMEOUT_ENV)
        .map(|value| value.parse::<u64>().unwrap())
        .unwrap_or(300);
    assert!(
        (1..=1800).contains(&timeout),
        "finite child deadline must be 1..1800 seconds"
    );
    for ordinal in 1..=5 {
        let output = destination.join(format!("build-{ordinal:02}.json"));
        let log_path = destination.join(format!("build-{ordinal:02}.log"));
        assert!(
            !output.exists(),
            "use a fresh {OUTPUT_ENV} directory; artifact already exists: {output:?}"
        );
        let log = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&log_path)
            .unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD_ENV, ordinal.to_string())
            .env(OUTPUT_ENV, &destination)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let started = Instant::now();
        loop {
            if let Some(status) = process.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "source build {ordinal:02} failed after {:?}: {status}; log {log_path:?}",
                    started.elapsed()
                );
                break;
            }
            if started.elapsed() >= Duration::from_secs(timeout) {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!("source build {ordinal:02} exceeded {timeout} s; log {log_path:?}");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let report: Json = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
        validate(&root, &report);
        assert_eq!(report["input"]["ordinal"], ordinal);
        assert_eq!(
            report["input"]["xml_sha256"],
            manifest["builds"][ordinal - 1]["xml_sha256"]
        );
    }
    assert_eq!(
        fs::read(root.join(MANIFEST)).unwrap(),
        manifest_bytes,
        "input manifest changed"
    );
}
