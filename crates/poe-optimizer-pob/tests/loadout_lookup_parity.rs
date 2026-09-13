//! Complete unchanged GetSpecList/GetLoadoutByName component differentials.
//! Supplied plain live-state fixtures, not original import closure or activation parity.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/loadout_lookup_source.rs"]
mod original;

use mlua::{MultiValue, Table, Value};
use original::{BUILD, Case, Context, Domain, Original, TREE};
use poe_optimizer_import::loadouts::{LoadoutErrorKind, LoadoutLimits, LoadoutProgram};
use serde_json::{Value as Json, json};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

#[derive(Clone, Debug)]
enum Expected {
    Returned(Option<[Option<f64>; 4]>),
    Source {
        path: &'static str,
        line: usize,
        getter: bool,
    },
}
fn returned(ids: [Option<f64>; 4]) -> Expected {
    Expected::Returned(Some(ids))
}
fn expected_ids(ids: [Option<f64>; 4]) -> BTreeMap<String, u64> {
    ["specId", "itemSetId", "skillSetId", "configSetId"]
        .into_iter()
        .zip(ids)
        .filter_map(|(key, value)| value.map(|value| (key.to_owned(), value.to_bits())))
        .collect()
}
fn native_ids(ids: poe_optimizer_import::loadouts::LoadoutIds) -> BTreeMap<String, u64> {
    expected_ids(
        [ids.spec, ids.items, ids.skills, ids.configuration].map(|value| value.map(|v| v.value())),
    )
}
fn source_failure(error: &mlua::Error, path: &str, line: usize) {
    assert!(
        original::is_source_error(error),
        "host/unclassified source failure: {error}"
    );
    assert!(
        error.to_string().contains(&format!("{path}:{line}:")),
        "wrong source error site: {error}"
    );
}
fn strings(pack: &MultiValue) -> Vec<String> {
    assert_eq!(pack.len(), 1);
    let Value::Table(table) = pack.front().unwrap() else {
        panic!("GetSpecList did not return one table")
    };
    assert!(table.metatable().is_none());
    assert!(table.raw_len() < 128);
    let mut keys = 0;
    for entry in table.pairs::<Value, Value>() {
        let (key, value) = entry.unwrap();
        keys += 1;
        let number = match key {
            Value::Integer(n) => n as f64,
            Value::Number(n) => n,
            other => panic!("unexpected list key {}", other.type_name()),
        };
        assert!(
            number.is_finite()
                && number.fract() == 0.0
                && number >= 1.0
                && number <= table.raw_len() as f64
        );
        assert!(matches!(value, Value::String(_)));
    }
    assert_eq!(keys, table.raw_len());
    (1..=keys)
        .map(|index| table.raw_get::<String>(index).unwrap())
        .collect()
}
fn run_case(
    original: &Original,
    program: &LoadoutProgram,
    case: &Case,
    expected: &Expected,
) -> Json {
    original.verify();
    let (build, read_set) = original.state(case);
    let context = Context::new(case);
    let before = original::snapshot(Value::Table(read_set.clone()));
    let tree: Table = build.raw_get("treeTab").unwrap();
    assert_eq!(
        tree.raw_get::<mlua::Function>("GetSpecList").unwrap(),
        original.spec_list
    );
    let source_specs = original.spec_list.call::<MultiValue>(tree.clone());
    let native_specs = program.spec_list(&context);
    let getter_report = match (source_specs, native_specs) {
        (Ok(source), Ok(native)) => {
            assert!(!matches!(expected, Expected::Source { getter: true, .. }));
            let actual = strings(&source);
            assert_eq!(actual, native, "{} GetSpecList", case.name);
            let again: MultiValue = original.spec_list.call(tree.clone()).unwrap();
            assert_eq!(strings(&again), actual);
            let (Value::Table(first), Value::Table(second)) =
                (source.front().unwrap(), again.front().unwrap())
            else {
                unreachable!()
            };
            assert_ne!(first, second, "source GetSpecList allocates a fresh list");
            json!({"kind":"returned","arity":source.len(),"values":actual,"fresh_list":true})
        }
        (Err(source), Err(native)) => {
            let Expected::Source {
                path,
                line,
                getter: true,
            } = expected
            else {
                panic!("unexpected getter error {}: {source}; {native}", case.name)
            };
            source_failure(&source, path, *line);
            assert_eq!(native.kind, LoadoutErrorKind::Source);
            json!({"kind":"source_error","original":source.to_string(),"native":native.message})
        }
        (source, native) => panic!(
            "{} getter outcome mismatch: {source:?} / {native:?}",
            case.name
        ),
    };
    context.reads.borrow_mut().clear();
    let source = original
        .lookup
        .call::<MultiValue>((build.clone(), case.query.as_str()));
    let native = program.lookup(&context, &case.query);
    let lookup_report = match (source, native) {
        (Ok(source), Ok(native)) => {
            let Expected::Returned(expected) = expected else {
                panic!("{} expected Source", case.name)
            };
            let actual = original::ids(&source);
            let native = native.map(|selection| {
                assert!(selection.belongs_to(&context, program));
                native_ids(selection.ids())
            });
            assert_eq!(actual, native, "{} original/native raw result", case.name);
            assert_eq!(
                actual,
                expected.map(expected_ids),
                "{} declared semantic witness",
                case.name
            );
            let again: MultiValue = original
                .lookup
                .call((build.clone(), case.query.as_str()))
                .unwrap();
            assert_eq!(original::ids(&again), actual);
            if let (Some(Value::Table(first)), Some(Value::Table(second))) =
                (source.front(), again.front())
            {
                assert_ne!(first, second, "lookup result has fresh table identity");
            }
            json!({"kind":"returned","arity":source.len(),"ids_bits":actual,"repeat_same_values":true,"table_results_fresh":actual.is_some()})
        }
        (Err(source), Err(native)) => {
            let Expected::Source { path, line, .. } = expected else {
                panic!("{} unexpected Source: {source} / {native}", case.name)
            };
            source_failure(&source, path, *line);
            assert_eq!(native.kind, LoadoutErrorKind::Source);
            json!({"kind":"source_error","original":source.to_string(),"native":native.message})
        }
        (Ok(source), Err(native)) => panic!(
            "{} source returned {:?}, native {native}",
            case.name,
            original::ids(&source)
        ),
        (Err(source), Ok(_)) => panic!("{} source failed {source}, native returned", case.name),
    };
    assert_eq!(
        before,
        original::snapshot(Value::Table(read_set)),
        "{} source read-set mutation/identity change",
        case.name
    );
    assert_eq!(
        tree.raw_get::<mlua::Function>("GetSpecList").unwrap(),
        original.spec_list
    );
    original.verify();
    let reads = context.reads.borrow();
    assert_eq!(
        &reads[..3],
        [
            "singleton:Skills",
            "singleton:Items",
            "singleton:Configuration"
        ]
    );
    json!({"case":case,"get_spec_list":getter_report,"lookup":lookup_report,
        "unchanged_read_set_sha256":before.digest(),"native_reads":*reads,
        "source_access_trace_observed":false,"source_input_length_proofs_checked":true})
}
fn cases(latest: &str, old: &str) -> Vec<(Case, Expected)> {
    let mut cases = vec![(
        Case::empty("no_matches_one_nil", "nothing"),
        Expected::Returned(None),
    )];
    let mut c = Case::empty("all_four_ids", "Match");
    c.spec(Some("Match"), Some(latest));
    c.items = Domain::named(&[(7.0, Some("Match")), (8.0, Some("Other"))]);
    c.skills = Domain::named(&[(13.0, Some("Other")), (17.0, Some("Match"))]);
    c.configuration = Domain::named(&[(21.0, Some("Match")), (22.0, Some("Other"))]);
    cases.push((c, returned([Some(1.0), Some(7.0), Some(17.0), Some(21.0)])));
    let mut c = Case::empty("fallback_title", "Default");
    c.spec(None, Some(latest));
    c.items = Domain::named(&[(5.0, None), (6.0, Some("Other"))]);
    cases.push((c, returned([Some(1.0), Some(5.0), None, None])));
    let mut c = Case::empty("empty_title_is_not_fallback", "");
    c.spec(Some(""), Some(latest));
    c.items = Domain::named(&[(3.0, Some("")), (4.0, None)]);
    cases.push((c, returned([Some(1.0), Some(3.0), None, None])));
    let mut c = Case::empty("utf8_and_nul_titles", "世界\0é");
    c.spec(Some("Other"), Some(latest));
    c.spec(Some("世界\0é"), Some(latest));
    c.items = Domain::named(&[(0.0, Some("世界\0é")), (-4.5, Some("Other"))]);
    cases.push((c, returned([Some(2.0), Some(0.0), None, None])));
    let mut c = Case::empty("singleton_ignores_missing_rows_and_link_maps", "No Match");
    c.items = Domain::named(&[(12.25, None)]);
    c.items.rows.clear();
    c.items.links = None;
    c.skills = Domain::named(&[(-4.5, None)]);
    c.skills.rows.clear();
    c.skills.links = None;
    c.configuration = Domain::named(&[(0.0, None)]);
    c.configuration.rows.clear();
    cases.push((c, returned([None, Some(12.25), Some(-4.5), Some(0.0)])));
    let mut c = Case::empty("tree_first_link_precedes_later_exact", "Later {pair}");
    c.spec(Some("First"), Some(latest));
    c.spec(Some("Later {pair}"), Some(latest));
    c.tree_links = Some(vec![("pair".into(), Some(41.5))]);
    cases.push((c, returned([Some(41.5), None, None, None])));
    let mut c = Case::empty("set_first_link_precedes_later_exact", "Later {pair}");
    c.items = Domain::named(&[(4.0, Some("First")), (9.0, Some("Later {pair}"))]);
    c.items.links = Some(vec![("pair".into(), Some(57.0))]);
    cases.push((c, returned([None, Some(57.0), None, None])));
    let mut c = Case::empty("first_exact_hides_missing_links", "{A}");
    c.spec(Some("{A}"), Some(latest));
    c.tree_links = None;
    c.items = Domain::named(&[(2.0, Some("{A}")), (3.0, Some("Other"))]);
    c.items.links = None;
    cases.push((c, returned([Some(1.0), Some(2.0), None, None])));
    let mut c = Case::empty("missing_tree_link_map", "Missing {A}");
    c.spec(Some("Other"), Some(latest));
    c.tree_links = None;
    cases.push((
        c,
        Expected::Source {
            path: BUILD,
            line: 925,
            getter: false,
        },
    ));
    let mut c = Case::empty("missing_tree_link_row", "Missing {A}");
    c.spec(Some("Other"), Some(latest));
    cases.push((
        c,
        Expected::Source {
            path: BUILD,
            line: 925,
            getter: false,
        },
    ));
    let mut c = Case::empty("present_link_without_id_returns_nil_early", "Later {A}");
    c.spec(Some("Other"), Some(latest));
    c.spec(Some("Later {A}"), Some(latest));
    c.tree_links = Some(vec![("A".into(), None)]);
    cases.push((c, Expected::Returned(None)));
    let mut c = Case::empty("missing_set_row_precedes_link", "Missing {A}");
    c.items = Domain::named(&[(4.0, None), (5.0, None)]);
    c.items.rows.clear();
    c.items.links = Some(vec![("A".into(), Some(90.0))]);
    cases.push((
        c,
        Expected::Source {
            path: BUILD,
            line: 904,
            getter: false,
        },
    ));
    let mut c = Case::empty(
        "missing_item_link_precedes_later_skill_failure",
        "Missing {A}",
    );
    c.items = Domain::named(&[(4.0, Some("Other")), (5.0, None)]);
    c.skills.absent = true;
    cases.push((
        c,
        Expected::Source {
            path: BUILD,
            line: 909,
            getter: false,
        },
    ));
    let mut c = Case::empty("missing_version_before_any_lookup", "Match");
    c.spec(Some("Match"), Some("unknown-version"));
    c.items.absent = true;
    cases.push((
        c,
        Expected::Source {
            path: TREE,
            line: 487,
            getter: true,
        },
    ));
    let mut c = Case::empty(
        "full_getter_reaches_later_bad_version_before_first_match",
        "Match",
    );
    c.spec(Some("Match"), Some(latest));
    c.spec(Some("Later"), None);
    cases.push((
        c,
        Expected::Source {
            path: TREE,
            line: 487,
            getter: true,
        },
    ));
    let mut c = Case::empty("duplicate_order_id_uses_last_map_winner", "Winner");
    c.items = Domain::named(&[(7.0, Some("Old")), (7.0, Some("Winner"))]);
    cases.push((c, returned([None, Some(7.0), None, None])));
    let mut c = Case::empty("duplicate_link_key_last_winner", "{A}");
    c.spec(Some("Other"), Some(latest));
    c.tree_links = Some(vec![("A".into(), Some(4.0)), ("A".into(), Some(8.0))]);
    cases.push((c, returned([Some(8.0), None, None, None])));
    let mut c = Case::empty("sparse_order_stops_ipairs_not_vector_count", "Match");
    c.items = Domain::named(&[(2.0, Some("Match")), (3.0, Some("Match"))]);
    c.items.order = vec![None, Some(2.0), Some(3.0)];
    c.items.singleton = false;
    cases.push((c, Expected::Returned(None)));
    let mut c = Case::empty("sparse_specs_hide_unreached_bad_version", "Match");
    c.spec(Some("Match"), Some(latest));
    c.specs.push(None);
    c.spec(Some("Later"), Some("missing"));
    cases.push((c, returned([Some(1.0), None, None, None])));
    for (name, query) in [
        ("empty_link_capture_does_not_match", "{}"),
        ("multi_link_is_not_single_link", "{A,B}"),
        ("nonascii_link_is_not_word_capture", "{é}"),
    ] {
        let mut c = Case::empty(name, query);
        c.spec(Some("Other"), Some(latest));
        c.tree_links = None;
        cases.push((c, Expected::Returned(None)));
    }
    let mut c = Case::empty("legacy_version_display", "placeholder");
    c.spec(Some("Title"), Some(old));
    // The caller fills this query from actual GameVersions display data, below.
    c.query = format!("[{old}] Title");
    cases.push((c, returned([Some(1.0), None, None, None])));
    let mut c = Case::empty("missing_items_after_getter", "Match");
    c.spec(Some("Match"), Some(latest));
    c.items.absent = true;
    cases.push((
        c,
        Expected::Source {
            path: BUILD,
            line: 937,
            getter: false,
        },
    ));
    let mut c = Case::empty("missing_skills_after_item_resolution", "Match");
    c.spec(Some("Match"), Some(latest));
    c.items = Domain::named(&[(5.0, None)]);
    c.skills.absent = true;
    cases.push((
        c,
        Expected::Source {
            path: BUILD,
            line: 939,
            getter: false,
        },
    ));
    cases
}

#[test]
fn complete_original_methods_match_native_for_directed_live_state() {
    let mut original = Original::new();
    let source = original.evidence();
    let latest = original.policy.latest_tree_version.clone();
    let old = original
        .policy
        .tree_version_display
        .keys()
        .find(|key| *key != &latest)
        .unwrap()
        .clone();
    let mut cases = cases(&latest, &old);
    let legacy = cases
        .iter_mut()
        .find(|(case, _)| case.name == "legacy_version_display")
        .unwrap();
    legacy.0.query = format!(
        "{}{}{}Title",
        original.policy.version_prefix,
        original.policy.tree_version_display[&old],
        original.policy.version_suffix
    );
    let program =
        LoadoutProgram::new(Arc::new(original.policy.clone()), LoadoutLimits::default()).unwrap();
    let mut reports = Vec::new();
    for (case, expected) in &cases {
        reports.push(run_case(&original, &program, case, expected));
    }
    assert_eq!(cases.len(), 26);
    // Inject only original data globals: methods, formatting literals and pattern are unchanged.
    original.set_versions(
        "caller-current",
        BTreeMap::from([("caller-old".into(), "Earlier Ω".into())]),
    );
    let injected_program =
        LoadoutProgram::new(Arc::new(original.policy.clone()), LoadoutLimits::default()).unwrap();
    let mut c = Case::empty("injected_version_display_and_latest", "[Earlier Ω] ");
    c.spec(Some(""), Some("caller-old"));
    reports.push(run_case(
        &original,
        &injected_program,
        &c,
        &returned([Some(1.0), None, None, None]),
    ));
    let mut c = Case::empty("current_version_does_not_require_display_entry", "Default");
    c.spec(None, Some("caller-current"));
    reports.push(run_case(
        &original,
        &injected_program,
        &c,
        &returned([Some(1.0), None, None, None]),
    ));
    let mut c = Case::empty("injected_missing_display_remains_source_error", "Default");
    c.spec(None, Some("0_1"));
    reports.push(run_case(
        &original,
        &injected_program,
        &c,
        &Expected::Source {
            path: TREE,
            line: 487,
            getter: true,
        },
    ));
    let report = json!({"source":source,"injected_policy":original.policy,"cases":reports,
        "count":reports.len(),"limits":{"lua_memory_bytes":32*1024*1024,"case_rows":128,"graph_rows":4096,"graph_bytes":256*1024},
        "scope":"source component parity over identical supplied finite text/numeric state; no imported closure, source access trace, full synchronization, activation or public native producer claim",
        "policy_variants":"Only version map/latest globals injected. Arbitrary fallback/format/link-pattern policy tests are native-only elsewhere; these original bodies are never edited."});
    if let Some(output) = std::env::var_os("POE_LOADOUT_LOOKUP_OUTPUT") {
        let output = PathBuf::from(output);
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(
            output.join("report.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
    eprintln!(
        "loadout lookup component: {} complete source/native cases",
        reports.len()
    );
}

#[test]
fn source_failure_classifier_rejects_host_resource_and_foreign_errors() {
    assert!(original::is_source_error(&mlua::Error::RuntimeError(
        "src/Modules/Build.lua:925: attempt to index a nil value".into()
    )));
    for error in [
        mlua::Error::RuntimeError("src/Modules/Build.lua:925: stack overflow".into()),
        mlua::Error::RuntimeError("harness: src/Classes/TreeTab.lua:487: deadline".into()),
        mlua::Error::RuntimeError("other.lua:1: source failure".into()),
        mlua::Error::MemoryError("not enough memory".into()),
        mlua::Error::external(std::io::Error::other("src/Modules/Build.lua:925: foreign")),
    ] {
        assert!(!original::is_source_error(&error));
    }
}
