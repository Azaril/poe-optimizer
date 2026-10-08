//! Original Player Life consumer evidence; no native reducer or copied oracle.
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
    cell::RefCell,
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};

const OBSERVE: &str = include_str!("support/player_life_source.lua");
const ORIGINAL: &str = include_str!("../../../tests/fixtures/builds/breadth-20260908/build-05.xml");
const ORIGINAL_SHA256: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";
const TEST: &str = "original_player_life_consumer_retains_inputs_branches_and_outputs";
const CHILD: &str = "POE_PLAYER_LIFE_SOURCE_CHILD";
const OUTPUT: &str = "POE_OPTIMIZER_TEST_PLAYER_LIFE_SOURCE_OUT";
const FILES: &[&str] = &[
    "src/Modules/Common.lua",
    "src/Modules/CalcDefence.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/ModParser.lua",
    "src/Modules/ModTools.lua",
    "src/Classes/ModStore.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModList.lua",
    "src/Classes/ConfigTab.lua",
    "src/Classes/CalcsTab.lua",
    "src/Classes/ItemsTab.lua",
    "src/Classes/Item.lua",
    "src/Modules/Build.lua",
    "src/Modules/Data.lua",
    "src/Data/Global.lua",
    "src/Data/Misc.lua",
];
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(v: &Json) -> &[Json] {
    if let Some(a) = v.as_array() {
        a
    } else {
        assert!(v.as_object().is_some_and(|o| o.is_empty()));
        &[]
    }
}
struct Case {
    name: &'static str,
    xml: String,
    custom: Option<&'static str>,
}
fn with_level(xml: &str, level: u16) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let build = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Build"))
        .unwrap();
    assert_eq!(build.attribute("level"), Some("92"));
    let start = build.range().start;
    let end = start + xml[start..].find('>').unwrap() + 1;
    assert_eq!(xml[start..end].matches("level=\"92\"").count(), 1);
    let replacement = xml[start..end].replace("level=\"92\"", &format!("level=\"{level}\""));
    let mut result = xml.to_owned();
    result.replace_range(start..end, &replacement);
    result
}
fn with_robe_life(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let matches: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name("Item") && n.attribute("id") == Some("19"))
        .collect();
    assert_eq!(matches.len(), 1);
    let range = matches[0].range();
    assert_eq!(xml[range.clone()].matches("+17 to maximum Life").count(), 1);
    let changed = xml[range.clone()].replace("+17 to maximum Life", "+18 to maximum Life");
    let mut result = xml.to_owned();
    result.replace_range(range, &changed);
    result
}
fn with_custom(xml: &str, line: &str) -> String {
    assert!(!line.contains(['<', '>', '&']));
    let doc = roxmltree::Document::parse(xml).unwrap();
    let config = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Config"))
        .unwrap();
    let selected = config.attribute("activeConfigSet").unwrap();
    let sets: Vec<_> = config
        .children()
        .filter(|n| n.has_tag_name("ConfigSet") && n.attribute("id") == Some(selected))
        .collect();
    assert_eq!(sets.len(), 1);
    let range = sets[0].range();
    let insertion = range.start + xml[range].rfind("</ConfigSet>").unwrap();
    let mut result = xml.to_owned();
    result.insert_str(insertion, &format!("<CustomModifierBlock title=\"Player Life source control\" enabled=\"true\">{line}</CustomModifierBlock>"));
    result
}
fn cases() -> Vec<Case> {
    let mut out = vec![
        Case {
            name: "original-05",
            xml: ORIGINAL.to_owned(),
            custom: None,
        },
        Case {
            name: "character-level-91",
            xml: with_level(ORIGINAL, 91),
            custom: None,
        },
        Case {
            name: "selected-robe-life-18",
            xml: with_robe_life(ORIGINAL),
            custom: None,
        },
    ];
    for (name, line) in [
        ("custom-increased-life", "10% increased maximum Life"),
        ("custom-more-life", "10% more maximum Life"),
        (
            "custom-chaos-inoculation",
            "Maximum Life becomes 1, immune to Chaos Damage",
        ),
    ] {
        out.push(Case {
            name,
            xml: with_custom(ORIGINAL, line),
            custom: Some(line),
        });
    }
    out
}
#[test]
fn evidence_encoder_keeps_nonfinite_numbers_distinct_from_source_strings_and_tables() {
    let lua = Lua::new();
    let observer: Table = lua.load(OBSERVE).eval().unwrap();
    let encode: Function = observer.get("encode_evidence_value").unwrap();
    let encoded: Table = lua
        .load(
            r#"
        local encode=...
        local marker="__poe_optimizer_nonfinite_number"
        local ok=pcall(encode,{[marker]="positive_infinity"})
        local nested_ok=pcall(encode,{nested={[marker]=false}})
        return {finite=encode(3.5),positive=encode(math.huge),negative=encode(-math.huge),
            nan=encode(0/0),text=encode("inf"),negative_text=encode("-inf"),nan_text=encode("nan"),
            nested=encode({math.huge,"inf"}),ordinary=encode({value=3.5}),
            marker_rejected=not ok,nested_marker_rejected=not nested_ok}
    "#,
        )
        .call(encode)
        .unwrap();
    let encoded: Json = lua.from_value(Value::Table(encoded)).unwrap();
    assert_eq!(encoded["finite"], 3.5);
    assert_eq!(
        encoded["positive"],
        json!({"__poe_optimizer_nonfinite_number":"positive_infinity"})
    );
    assert_eq!(
        encoded["negative"],
        json!({"__poe_optimizer_nonfinite_number":"negative_infinity"})
    );
    assert_eq!(
        encoded["nan"],
        json!({"__poe_optimizer_nonfinite_number":"nan"})
    );
    assert_eq!(encoded["text"], "inf");
    assert_eq!(encoded["negative_text"], "-inf");
    assert_eq!(encoded["nan_text"], "nan");
    assert_eq!(encoded["nested"][0], encoded["positive"]);
    assert_eq!(encoded["nested"][1], encoded["text"]);
    assert_eq!(encoded["ordinary"], json!({"value":3.5}));
    assert_eq!(encoded["marker_rejected"], true);
    assert_eq!(encoded["nested_marker_rejected"], true);
}
#[test]
fn player_life_controls_are_distinct_and_preserve_unrelated_saved_inputs() {
    assert_eq!(hash(ORIGINAL.as_bytes()), ORIGINAL_SHA256);
    let original = roxmltree::Document::parse(ORIGINAL).unwrap();
    let cases = cases();
    assert_eq!(cases.len(), 6);
    assert_eq!(
        cases
            .iter()
            .map(|c| hash(c.xml.as_bytes()))
            .collect::<BTreeSet<_>>()
            .len(),
        6
    );
    for case in &cases {
        let changed = roxmltree::Document::parse(&case.xml).unwrap();
        for name in ["Tree", "Skills", "Items", "Build", "Config"] {
            if (name == "Build" && case.name == "character-level-91")
                || (name == "Items" && case.name == "selected-robe-life-18")
                || (name == "Config" && case.custom.is_some())
            {
                continue;
            }
            let a = original
                .root_element()
                .children()
                .find(|n| n.has_tag_name(name))
                .unwrap();
            let b = changed
                .root_element()
                .children()
                .find(|n| n.has_tag_name(name))
                .unwrap();
            assert_eq!(
                &ORIGINAL[a.range()],
                &case.xml[b.range()],
                "{} {name}",
                case.name
            );
        }
    }
    assert!(!OBSERVE.contains(":Sum("));
    assert!(!OBSERVE.contains(":More("));
    assert!(!OBSERVE.contains(":Flag("));
    assert!(!OBSERVE.contains(":Override("));
    assert!(!OBSERVE.contains("NewMod("));
}
fn observed(root: &Path, xml: &str, jit: bool, hooked: bool) -> Json {
    assert!(!hooked || !jit, "consumer acquisition is fixed to JIT-off");
    let module = Rc::new(RefCell::new(None::<Table>));
    let before_source = |lua: &Lua| {
        lua.load(if jit {
            "jit.on()"
        } else {
            "jit.off();jit.flush()"
        })
        .exec()?;
        Ok(())
    };
    let before_build = |lua: &Lua| -> Result<Function, RuntimeError> {
        let observer: Table = lua
            .load(OBSERVE)
            .set_name("@player_life_source.lua")
            .eval()?;
        let cleanup = observer.raw_get::<Function>("begin")?.call(jit)?;
        *module.borrow_mut() = Some(observer);
        Ok(cleanup)
    };
    let after = |lua: &Lua| -> Result<Json, RuntimeError> {
        let observer = if hooked {
            module.borrow().as_ref().unwrap().clone()
        } else {
            lua.load(OBSERVE)
                .set_name("@player_life_source.lua")
                .eval::<Table>()?
        };
        let value: Table = observer.raw_get::<Function>("observe")?.call(jit)?;
        Ok(lua.from_value(Value::Table(value))?)
    };
    let scratch = tempfile::tempdir().unwrap();
    let report = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        xml,
        None,
        false,
        Some(&before_source),
        if hooked { Some(&before_build) } else { None },
        Some(&after),
    )
    .unwrap();
    assert_eq!(report["configuration_method_wrappers"], false);
    assert_eq!(report["original_build_output_available"], true);
    assert_eq!(report["source_hash"], pinned::manifest_sha256());
    json!({"source_hash":report["source_hash"],"selected":report["selected"],"state":report["additional_observation"]})
}
fn input_value<'a>(row: &'a Json, name: &str) -> &'a Json {
    assert_eq!(row["consumer_checkpoint_reached"], true);
    assert_eq!(row["computation"]["line"], 97);
    let input = &row["computation"]["inputs"][name];
    assert_eq!(input["assignment_reached"], true);
    &input["value"]
}
fn number(row: &Json, name: &str) -> f64 {
    let v = input_value(row, name);
    assert_eq!(v["present"], true);
    assert_eq!(v["kind"], "number");
    v["value"].as_f64().unwrap()
}
fn check(host: &Json, case: &Case) {
    let state = &host["state"];
    assert_eq!(
        state["instrumentation"]["consumer_observer_installed"],
        true
    );
    assert_eq!(state["instrumentation"]["business_methods_wrapped"], false);
    assert_eq!(state["instrumentation"]["original_methods_preserved"], true);
    let calls = rows(&state["invocations"]);
    assert!(!calls.is_empty());
    for call in calls {
        assert_eq!(call["exact_existing_player"], true);
        assert_eq!(call["exact_actor_store"], true);
        let c = &call["computation"];
        assert_eq!(c["line"], 97);
        for (name, local, line) in [
            ("low_life_percentage", "lowLifePerc", 80),
            ("full_life_percentage", "fullLifePerc", 82),
            ("base", "base", 88),
            ("extra", "extra", 89),
            ("total", "total", 90),
            ("increase", "inc", 91),
            ("more", "more", 92),
            ("conversion", "conv", 93),
            ("override", "override", 94),
        ] {
            let input = &c["inputs"][name];
            assert_eq!(input["local_name"], local);
            assert_eq!(input["assignment_line"], line);
            let observed = input_value(call, name);
            if name != "override" {
                assert_eq!(observed["kind"], "number");
                assert_eq!(observed["present"], true);
            }
        }
        assert_eq!(c["inputs"]["chaos_inoculation"]["assignment_line"], 85);
        assert_eq!(
            c["inputs"]["chaos_inoculation"]["output_field"],
            "ChaosInoculation"
        );
        assert_eq!(c["has_override"], input_value(call, "override")["present"]);
        assert_eq!(
            call["return_chaos_inoculation"],
            *input_value(call, "chaos_inoculation")
        );
        assert!(!rows(&c["stores"]).is_empty());
        // This is the original post-clamp local. Neither raw conversion Sum nor
        // the original rounding operand is independently observed or inferred.
        assert_eq!(number(call, "conversion"), 0.);
        assert_eq!(number(call, "extra"), 0.);
        assert_eq!(number(call, "total"), 0.);
        assert_eq!(
            input_value(call, "override"),
            &json!({"present":false,"kind":"nil"})
        );
    }
    for mode in ["MAIN", "CALCS"] {
        let selection = &state["current_modes"][mode];
        assert_eq!(selection["exact_environment"], true);
        let ids = rows(&selection["invocations"]);
        assert!(!ids.is_empty());
        let last = selection["last"].as_u64().unwrap() as usize - 1;
        assert_eq!(ids.last().unwrap().as_u64().unwrap() as usize - 1, last);
        let row = &calls[last];
        assert_eq!(row["mode"], mode);
        let snapshot = &state["snapshot"]["modes"][mode];
        assert_eq!(row["return_life"], snapshot["output"]["Life"]);
        assert_eq!(
            row["return_chaos_inoculation"],
            snapshot["availability"]["chaos_inoculation"]
        );
        assert_eq!(row["caller"]["path"], "Modules/CalcDefence.lua");
        assert_eq!(row["caller"]["line"], 1631);
        let expected_base = match case.name {
            "character-level-91" => 1245.,
            "selected-robe-life-18" => 1258.,
            _ => 1257.,
        };
        assert_eq!(number(row, "base"), expected_base);
        assert_eq!(
            number(row, "increase"),
            if case.name == "custom-increased-life" {
                15.
            } else {
                5.
            }
        );
        assert_eq!(
            number(row, "more"),
            if case.name == "custom-more-life" {
                1.1
            } else {
                1.
            }
        );
        if case.name == "custom-chaos-inoculation" {
            assert_eq!(
                input_value(row, "chaos_inoculation"),
                &json!({"present":true,"kind":"boolean","value":true})
            );
            assert_eq!(row["return_life"], 1);
            assert_eq!(
                row["return_full_life"],
                json!({"present":true,"kind":"boolean","value":true})
            );
        } else {
            assert_eq!(
                input_value(row, "chaos_inoculation"),
                &json!({"present":false,"kind":"nil"})
            );
            assert_eq!(row["return_life"], row["computation"]["life"]);
        }
        if case.name == "original-05" {
            assert_eq!(row["return_life"], 1320);
        }
    }
}
fn child(root: &Path, out: &Path, jit: bool) {
    assert_eq!(hash(ORIGINAL.as_bytes()), ORIGINAL_SHA256);
    let files: Vec<_> = FILES
        .iter()
        .map(|path| {
            let text = pinned::read_verified_text(&root.join("vendor/path-of-building-poe2"), path)
                .unwrap();
            let expected = pinned::expected_file_sha256(path).unwrap();
            assert_eq!(hash(text.as_bytes()), expected);
            json!({"path":path,"sha256":expected})
        })
        .collect();
    let metadata = json!({"source_revision":pinned::UPSTREAM_REVISION,
        "manifest_sha256":pinned::manifest_sha256(),"observer_sha256":hash(OBSERVE.as_bytes()),
        "harness_sha256":hash(include_bytes!("owned_player_life_source.rs")),
        "bootstrap_sha256":hash(include_bytes!("support/configuration_preparation_source.rs")),
        "files":files,"original_xml_sha256":ORIGINAL_SHA256});
    let projection = json!({"recipient":"exact existing Player","modes":["MAIN","CALCS"],
        "all_scalar_outputs":true,"selected_input_identity":true,
        "stores":"explicit Life-related buckets and scalar conditions/multipliers across parent chain",
        "nested_outputs_included":false,"other_modifier_buckets_included":false,
        "complete_player_state_claim":false,"source_methods_preserved":true});
    let mode = if jit { "on" } else { "off" };
    let cases = cases();
    let mut acquisitions = Vec::new();
    let mut references = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        eprintln!(
            "Player Life {}/{} {} JIT {mode}",
            index + 1,
            cases.len(),
            case.name
        );
        // A fixed protocol, never a failed JIT acquisition followed by fallback.
        let acquired = if jit {
            None
        } else {
            Some((
                observed(root, &case.xml, false, true),
                observed(root, &case.xml, false, true),
            ))
        };
        let plain = observed(root, &case.xml, jit, false);
        let repeated_plain = observed(root, &case.xml, jit, false);
        let reference = json!({"name":case.name,"xml_sha256":hash(case.xml.as_bytes()),
            "custom_modifier":case.custom,"synthetic_source_control":case.custom.is_some(),
            "first":plain,"repeat":repeated_plain});
        fs::write(
            out.join(format!(
                "reference-jit-{mode}-case-{:02}.raw.json",
                index + 1
            )),
            serde_json::to_vec_pretty(&reference).unwrap(),
        )
        .unwrap();
        for state in [&plain, &repeated_plain] {
            assert_eq!(
                state["state"]["instrumentation"]["consumer_observer_installed"],
                false
            );
            assert_eq!(
                state["state"]["instrumentation"]["business_methods_wrapped"],
                false
            );
            assert_eq!(
                state["state"]["instrumentation"]["original_methods_preserved"],
                true
            );
        }
        assert_eq!(
            json_evidence::first_difference(&plain, &repeated_plain, "$"),
            None,
            "fresh uninstrumented repeat {}",
            case.name
        );
        if let Some((first, repeat)) = acquired {
            let acquisition = json!({"name":case.name,"xml_sha256":hash(case.xml.as_bytes()),
                "custom_modifier":case.custom,"synthetic_source_control":case.custom.is_some(),
                "first":first,"repeat":repeat});
            fs::write(
                out.join(format!(
                    "acquisition-jit-off-case-{:02}.raw.json",
                    index + 1
                )),
                serde_json::to_vec_pretty(&acquisition).unwrap(),
            )
            .unwrap();
            check(&first, case);
            check(&repeat, case);
            assert_eq!(
                json_evidence::first_difference(&first, &repeat, "$"),
                None,
                "fresh JIT-off acquisition repeat {}",
                case.name
            );
            assert_eq!(
                json_evidence::first_difference(
                    &first["state"]["snapshot"],
                    &plain["state"]["snapshot"],
                    "$"
                ),
                None,
                "uninstrumented JIT-off projection {}",
                case.name
            );
            assert_eq!(first["source_hash"], plain["source_hash"]);
            assert_eq!(first["selected"], plain["selected"]);
            assert_eq!(first["state"]["methods"], plain["state"]["methods"]);
            acquisitions.push(acquisition);
        }
        references.push(reference);
    }
    let reference = json!({"schema_version":1,"role":"uninstrumented_reference",
        "jit_enabled":jit,"metadata":metadata,"projection":projection,
        "case_count":cases.len(),"complete_loads":cases.len()*2,"cases":references});
    fs::write(
        out.join(format!("reference-jit-{mode}.json")),
        serde_json::to_vec_pretty(&reference).unwrap(),
    )
    .unwrap();
    if !jit {
        let scope = json!({"recipient":"exact existing Player","original_consumer":"calcs.doActorLifeManaSpirit",
            "fixed_acquisition_jit_enabled":false,"internal_cross_jit_proof":false,
            "original_consumer_locals":true,"consumer_checkpoint_line":97,
            "original_query_return_observation":false,"raw_conversion_sum_observed":false,
            "original_round_input_observed":false,"post_clamp_conversion_local":true,
            "assigned_nil_distinct_from_checkpoint_not_reached":true,
            "internal_query_debug_event_tracking":false,"raw_notification_counts_are_semantic":false,
            "fresh_repeat":true,"no_retry_fallback_or_settling":true,"warm_rebuild_law":false,
            "copied_formula":false,"business_methods_wrapped":false,"added_modifier_queries":false,
            "native_final_life_parity":false,"whole_build_parity":false,"producer_coverage_closed":false,
            "synthetic_controls_obtainable":false,
            "extra_total_conversion_override_positive_producers_exercised":false,
            "observed_absence_is_global_absence":false,"rounding_entire_float_domain_proved":false});
        let acquisition = json!({"schema_version":1,"role":"jit_off_consumer_acquisition",
            "jit_enabled":false,"metadata":metadata,"projection":projection,"scope":scope,
            "case_count":cases.len(),"complete_loads":cases.len()*2,"cases":acquisitions});
        fs::write(
            out.join("acquisition-jit-off.json"),
            serde_json::to_vec_pretty(&acquisition).unwrap(),
        )
        .unwrap();
    }
}
fn compare_references(off: &Json, on: &Json) {
    assert_eq!(off["jit_enabled"], false);
    assert_eq!(on["jit_enabled"], true);
    // Execution-mode metadata is intentionally distinct. Every other captured
    // field must match exactly; values, ordering, nil and availability stay raw.
    let fields = [
        "schema_version",
        "role",
        "metadata",
        "projection",
        "case_count",
        "complete_loads",
        "cases",
    ];
    let expected_keys: BTreeSet<_> = fields.iter().copied().chain(["jit_enabled"]).collect();
    for report in [off, on] {
        assert_eq!(
            report
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            expected_keys
        );
    }
    for field in fields {
        assert_eq!(
            json_evidence::first_difference(&off[field], &on[field], field),
            None,
            "cross-JIT uninstrumented reference {field}"
        );
    }
}
#[test]
fn reference_comparison_excludes_only_execution_mode_metadata() {
    let off = json!({"schema_version":1,"role":"uninstrumented_reference","jit_enabled":false,
        "metadata":{"source":"fixed"},"projection":{"nil_is_distinct":true},"case_count":1,"complete_loads":2,
        "cases":[{"life":1320,"optional":{"present":false,"kind":"nil"},"ordered_sources":["a","b"]}]});
    let mut on = off.clone();
    on["jit_enabled"] = json!(true);
    compare_references(&off, &on);
    for changed in [
        {
            let mut v = on.clone();
            v["cases"][0]["life"] = json!(1321);
            v
        },
        {
            let mut v = on.clone();
            v["cases"][0]["optional"] = json!({"present":true,"kind":"boolean","value":false});
            v
        },
        {
            let mut v = on.clone();
            v["cases"][0]["ordered_sources"] = json!(["b", "a"]);
            v
        },
        {
            let mut v = on.clone();
            v.as_object_mut().unwrap().remove("projection");
            v["unreviewed"] = json!({});
            v
        },
    ] {
        assert!(std::panic::catch_unwind(|| compare_references(&off, &changed)).is_err());
    }
}
#[test]
#[ignore = "complete pinned PoB loads; independent original Player Life consumer evidence"]
fn original_player_life_consumer_retains_inputs_branches_and_outputs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join("runs/owned-player-life-source-01"));
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        child(&root, &out, mode == "on");
        return;
    }
    assert!(!out.exists(), "fresh immutable output directory required");
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", TEST, "--nocapture"])
            .env(CHILD, mode)
            .env(OUTPUT, &out)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = process.try_wait().unwrap() {
                assert!(status.success(), "source child failed: {}", path.display());
                break;
            }
            if start.elapsed() > Duration::from_secs(600) {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!("source deadline: {}", path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    let off = fs::read(out.join("reference-jit-off.json")).unwrap();
    let on = fs::read(out.join("reference-jit-on.json")).unwrap();
    compare_references(
        &serde_json::from_slice(&off).unwrap(),
        &serde_json::from_slice(&on).unwrap(),
    );
    let acquisition = fs::read(out.join("acquisition-jit-off.json")).unwrap();
    let receipt = json!({"schema_version":1,"case_count":6,"complete_loads":36,
        "protocol":"two JIT-off observed acquisitions; two uninstrumented references in each JIT mode",
        "acquisition_jit_off_sha256":hash(&acquisition),
        "reference_jit_off_sha256":hash(&off),"reference_jit_on_sha256":hash(&on),
        "reference_metadata_difference":"jit_enabled only","exact_reference_projection_agreement":true,
        "exact_acquisition_repeat_agreement":true,"acquisition_vs_plain_off_projection_agreement":true,
        "internal_cross_jit_proof":false,"warm_rebuild_law":false});
    fs::write(
        out.join("comparison.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
}
