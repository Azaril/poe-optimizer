//! Complete original ParseRaw versus native loading through advanced-copy ordering.
//! Assembly remains an explicit dependency in this component lane; source runs it.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/advanced_copy_order_source.rs"]
mod original;
#[allow(dead_code)]
#[path = "support/item_loading_native.rs"]
mod reference;
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
#[allow(dead_code)]
#[path = "support/base_buff_source.rs"]
mod source;
use mlua::{Table, Value};
use original::{Oracle, canonical};
use poe_optimizer_data::{game_data::bundled_snapshot, item_loading::ItemLoadingCatalog};
use poe_optimizer_import::item_loading::*;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};

const EXCLUSIVE: &str = r#"return {
 Exact={'+10 to maximum Life',statOrder={80}},
 ExactDuplicate={'+10 to maximum Life',statOrder={60}},
 Normalized={'+20 to maximum Life',statOrder={10}},
 Strength={'+5 to Strength',statOrder={20}},
 Mana={'+5 to\nmaximum Mana',statOrder={12}},
 Armour={'(10-20)% increased Armour',statOrder={30}},
 StringOrder={'+9 to Dexterity',statOrder={' 7e0 '}},
 HexOrder={'+9 to Intelligence',statOrder={'0x8'}},
}"#;
fn raw(rarity: &str, headers: &str, lines: &str) -> String {
    let title = if matches!(rarity, "UNIQUE" | "RELIC" | "RARE") {
        "Caller ordering fixture\n"
    } else {
        ""
    };
    format!(
        "Rarity: {rarity}\n{title}Amber Amulet\nItem Level: 80\nQuality: 0\n{headers}Implicits: 0\n{lines}"
    )
}
fn write(name: &str, report: Json) {
    let output = std::env::var_os("POE_ADVANCED_COPY_ORDER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            runtime::repository().join("runs/r2ag-advanced-copy-order-01/source-order")
        });
    std::fs::create_dir_all(&output).unwrap();
    std::fs::write(
        output.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
/// The source has finished full ParseRaw before this comparison. Native stops at
/// its independent assembly dependency, without receiving source poststate.
fn compare(
    oracle: &Oracle,
    catalog: &ItemLoadingCatalog,
    input: &str,
    reuse: Option<&Table>,
) -> (Table, Json) {
    let item = if let Some(item) = reuse {
        oracle.source.reparse(item, input);
        item.clone()
    } else {
        let (item, error) = oracle.source.try_parse(input);
        assert!(
            error.is_none(),
            "complete original ParseRaw: {error:?}; {input}"
        );
        item
    };
    let before = oracle.source.before();
    let after = oracle.source.snapshot(&item);
    let calls = oracle.source.calls();
    let mut provider = reference::OriginalDependencies::new(&oracle.source.oracle);
    let mut machine = ItemLoadMachine::new(catalog);
    machine.apply_text(input, &mut provider).unwrap();
    assert_eq!(
        machine.pending().map(|p| p.kind),
        Some(DependencyKind::Assembly),
        "{:?}",
        machine.pending()
    );
    reference::compare_state(machine.state(), &before);
    assert_eq!(provider.calls, calls, "ordered original parser parameters");
    let rows = original::rows(&before);
    assert_eq!(
        original::native_rows(&machine.state().explicit_mod_lines),
        rows
    );
    oracle.verify();
    let report = json!({
        "scope":"complete unchanged original ParseRaw; native complete preassembly loading projection through ordering, explicit Assembly dependency",
        "source":oracle.provenance(), "raw":input, "raw_sha256":format!("{:x}",Sha256::digest(input.as_bytes())),
        "original_item_reused":reuse.is_some(),"native_machine":"fresh; no observed poststate supplied",
        "native_reparse_or_all_retained_field_parity_claim":false,
        "rows":rows,"parser_calls":calls,
        "parser_calls_scope":"ordered text and combined boolean parameters, not observed actual arity",
        "preassembly_equal":true,"source_full_parse_succeeded":true,
        "source_before_assembly":canonical(Value::Table(before)),
        "source_after_parse":canonical(Value::Table(after)),
        "projection_limits":"existing declared ItemState fields; callback/shared-cycle sentinels retained; no complete Item alias-graph or final Load/inventory claim",
        "native_owned_assembly_claim":false
    });
    (item, report)
}
fn control(oracle: &original::Control, input: &str, report: &mut Json) {
    let reuse = report["original_item_reused"].as_bool().unwrap();
    let state = oracle.snapshot(input, reuse);
    assert!(
        report["source_after_parse"] == state,
        "inactive-observer control finite projection: {}",
        original::first_difference(&report["source_after_parse"], &state)
    );
    report["control_item_reused"] = json!(reuse);
    report["inactive_observer_control_finite_projection_equal"] = json!(true);
    report["control_scope"] = json!(
        "independent control host, matching fresh/reused Item history, existing base observer inactive and no buff observer; complete original methods, finite projection only"
    );
}
#[test]
fn actual_build02_item34_single_line_receives_order_before_assembly() {
    let snapshot = bundled_snapshot().unwrap();
    let oracle = Oracle::new();
    let relative = "tests/fixtures/builds/breadth-20260908/build-02.xml";
    let bytes = std::fs::read(runtime::repository().join(relative)).unwrap();
    let document = roxmltree::Document::parse(std::str::from_utf8(&bytes).unwrap()).unwrap();
    let items = document
        .descendants()
        .filter(|n| n.has_tag_name("Item") && n.attribute("id") == Some("34"))
        .collect::<Vec<_>>();
    assert_eq!(items.len(), 1);
    let node = items[0];
    let input = node
        .children()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .collect::<String>();
    let (_, mut report) = compare(&oracle, snapshot.item_loading(), &input, None);
    assert_eq!(
        report["rows"],
        json!([["(70-80)% reduced Amount Recovered", 930.0]])
    );
    control(&original::Control::new(None), &input, &mut report);
    report["input"] = json!({"kind":"unchanged actual XML item text","path":relative,"item_id":34,"file_sha256":format!("{:x}",Sha256::digest(&bytes)),"item_range":[node.range().start,node.range().end],"XML_child_processing_claim":false});
    report["single_line_sort_skipped_but_order_written"] = json!(true);
    write("build-02-item-34", report);
}
#[test]
fn custom_exact_normalized_minima_groups_metadata_and_source_reentry() {
    let snapshot = bundled_snapshot().unwrap();
    let oracle = Oracle::new();
    let catalog = oracle.install(snapshot.item_loading(), EXCLUSIVE);
    let controls = original::Control::new(Some(EXCLUSIVE));
    let cases = [
        (
            "single-exact",
            raw("UNIQUE", "", "{range:0.5}+10 to maximum Life"),
            json!([["+10 to maximum Life", 60.0]]),
        ),
        (
            "single-normalized",
            raw("UNIQUE", "", "{range:0.5}+15 to maximum Life"),
            json!([["+15 to maximum Life", 10.0]]),
        ),
        (
            "single-unmatched",
            raw("UNIQUE", "", "{range:0.5}+7 to Accuracy Rating"),
            json!([["+7 to Accuracy Rating", null]]),
        ),
        (
            "numeric-string",
            raw(
                "UNIQUE",
                "",
                "{range:0.5}+9 to Intelligence\n+9 to Dexterity",
            ),
            json!([["+9 to Dexterity", 7.0], ["+9 to Intelligence", 8.0]]),
        ),
        (
            "range-normalization",
            raw(
                "RELIC",
                "",
                "{range:0.25}(30-40)% increased Armour\n+5 to maximum Mana",
            ),
            json!([
                ["+5 to maximum Mana", 12.0],
                ["(30-40)% increased Armour", 30.0]
            ]),
        ),
        (
            "groups-and-metadata",
            raw(
                "UNIQUE",
                "",
                "{range:0.5}{tags:alpha}+99 to maximum Life\n{crafted}+10 to maximum Life\n{fractured}{corruptedRange:0.25}+10 to maximum Life\n{range:0.75}{tags:beta}+15 to maximum Life\n{fractured}+5 to Strength\n{custom}{fractured}+20 to maximum Life\n{crafted}+20 to maximum Life\n+5 to maximum Mana\n+7 to Accuracy Rating",
            ),
            json!([
                ["+5 to Strength", 20.0],
                ["+10 to maximum Life", 60.0],
                ["+99 to maximum Life", 10.0],
                ["+15 to maximum Life", 10.0],
                ["+5 to maximum Mana", 12.0],
                ["+7 to Accuracy Rating", null],
                ["+10 to maximum Life", 60.0],
                ["+20 to maximum Life", 10.0],
                ["+20 to maximum Life", 10.0]
            ]),
        ),
        (
            "duplicate-lines",
            raw(
                "UNIQUE",
                "",
                "{range:0.5}{tags:first}+10 to maximum Life\n{tags:second}+10 to maximum Life\n+15 to maximum Life",
            ),
            json!([
                ["+15 to maximum Life", 10.0],
                ["+10 to maximum Life", 60.0],
                ["+10 to maximum Life", 60.0]
            ]),
        ),
    ];
    let mut reports = Vec::new();
    let mut reused = None;
    let mut cache = None;
    for (name, input, expected) in cases {
        let (item, mut report) = compare(&oracle, &catalog, &input, reused.as_ref());
        assert_eq!(report["rows"], expected, "{name}");
        let current = oracle.cache().as_table().unwrap().clone();
        if let Some(previous) = &cache {
            assert_eq!(previous, &current, "original lazy cache identity persists");
        }
        cache = Some(current);
        reused = Some(item);
        control(&controls, &input, &mut report);
        if name == "duplicate-lines" {
            assert!(!input.contains("{fractured}"));
            assert!(
                reused
                    .as_ref()
                    .unwrap()
                    .raw_get::<bool>("fractured")
                    .unwrap()
            );
            assert_eq!(controls.field("fractured"), Value::Boolean(true));
            report["retained_fractured_from_prior_parse"] = json!(true);
        }
        report["name"] = json!(name);
        reports.push(report);
    }
    write(
        "custom-minima-groups-reentry",
        json!({"catalog_script":EXCLUSIVE,"input_kind":"identical test-owned raw Exclusive definitions injected before original first lookup","source_cache_identity_retained":true,"cases":reports}),
    );
}
#[test]
fn versioned_and_grouped_unique_bypass_lookup_but_still_sort() {
    let snapshot = bundled_snapshot().unwrap();
    let script = "return {Unusable={'+10 to maximum Life'}}";
    let oracle = Oracle::new();
    let catalog = oracle.install(snapshot.item_loading(), script);
    let controls = original::Control::new(Some(script));
    let cases = [
        (
            "versioned",
            "Version: Old\nVersion: New\nVariant: First\nVariant: Second\nSelected Version: 2\nSelected Variant: 2\n",
            "{range:0.5}{version:1}{variant:1}{crafted}+10 to maximum Life\n{version:2}{variant:2}{fractured}+20 to maximum Life\n+5 to Strength",
        ),
        (
            "grouped",
            "Version: Old\nVersion: New\nVariant: First\nVariant: Second\nSelected Version: 2\nSelected Variant Group: 1=2\nSelected Variant Group: 2=2\n",
            "{range:0.5}{version:1}{variant:1}{group:1}{custom}+10 to maximum Life\n{version:2}{variant:2}{group:1,2}{fractured}+20 to maximum Life\n+5 to Strength",
        ),
    ];
    let mut reports = Vec::new();
    for (name, headers, lines) in cases {
        let input = raw("UNIQUE", headers, lines);
        let (_, mut report) = compare(&oracle, &catalog, &input, None);
        assert_eq!(
            report["rows"],
            json!([
                ["+20 to maximum Life", null],
                ["+5 to Strength", null],
                ["+10 to maximum Life", null]
            ])
        );
        assert!(
            oracle.cache().is_nil(),
            "unreached invalid Exclusive family"
        );
        control(&controls, &input, &mut report);
        report["name"] = json!(name);
        reports.push(report);
    }
    write(
        "grouped-versioned-bypass",
        json!({"raw_catalog":script,"cache_remains_absent":true,"cases":reports}),
    );
}
#[test]
fn fresh_catalogs_do_not_share_original_or_native_lookup_results() {
    let snapshot = bundled_snapshot().unwrap();
    let mut reports = Vec::new();
    for order in [17, 43] {
        let script = format!("return {{Only={{'+10 to maximum Life',statOrder={{{order}}}}}}}");
        let oracle = Oracle::new();
        let catalog = oracle.install(snapshot.item_loading(), &script);
        let input = raw("UNIQUE", "", "{range:0.5}+10 to maximum Life");
        let (_, report) = compare(&oracle, &catalog, &input, None);
        assert_eq!(
            report["rows"],
            json!([["+10 to maximum Life", order as f64]])
        );
        reports.push(report);
    }
    write(
        "catalog-isolation",
        json!({"scope":"fresh original module/host and native immutable catalog per input; no same-host data rebinding claim","cases":reports}),
    );
}
fn genuine_source(error: &mlua::Error) -> bool {
    match error {
        mlua::Error::RuntimeError(text) => !text.contains("oracle deadline"),
        mlua::Error::CallbackError { cause, .. } => genuine_source(cause),
        _ => false,
    }
}
#[test]
fn malformed_raw_catalogs_remain_explicit_frontiers_not_fabricated_source_prefixes() {
    let snapshot = bundled_snapshot().unwrap();
    let mut reports = Vec::new();
    for (name, script) in [
        ("missing-order", "return {Bad={'+10 to maximum Life'}}"),
        (
            "invalid-order",
            "return {Bad={'+10 to maximum Life',statOrder={'not numeric'}}}",
        ),
        ("scalar-record", "return {Bad=42}"),
    ] {
        let oracle = Oracle::new();
        let catalog = oracle.install(snapshot.item_loading(), script);
        let input = raw("UNIQUE", "", "{range:0.5}+10 to maximum Life");
        let (item, error) = oracle.source.try_parse(&input);
        let error = error.expect("original malformed raw definition must fail");
        assert!(
            genuine_source(&error),
            "host/resource failure is not source evidence: {error:?}"
        );
        assert!(
            oracle.source.stages().is_empty(),
            "source fails before assembly"
        );
        let mut provider = reference::OriginalDependencies::new(&oracle.source.oracle);
        let mut machine = ItemLoadMachine::new(&catalog);
        machine.apply_text(&input, &mut provider).unwrap();
        assert_eq!(machine.status(), ItemLoadStatus::Pending);
        assert_eq!(
            machine.pending().map(|p| p.kind),
            Some(DependencyKind::AdvancedCopyAffixes)
        );
        assert!(
            machine
                .state()
                .explicit_mod_lines
                .iter()
                .all(|r| r.order.is_none())
        );
        reports.push(json!({"name":name,"raw_catalog":script,"source_error":error.to_string(),"source_partial_projection":canonical(Value::Table(oracle.source.snapshot(&item))),"native_frontier":machine.pending(),"source_partial_cache":canonical(oracle.cache()),"prefix_or_error_parity_claim":false,"reason":"raw pairs traversal and partial lookup-cache effects are not represented; native refusal is explicit"}));
        oracle.verify();
    }
    write("malformed-catalog-frontiers", json!({"cases":reports}));
}
#[test]
fn retained_original_sort_preserves_rows_metadata_and_duplicate_alias_last_index() {
    let oracle = Oracle::new();
    let lua = &oracle.source.oracle.lua;
    let fixture: Table = lua
        .load(
            r#"
      local shared={marker='retained'}
      local a={line='a',order=20,modTags={'x'},metadata=shared}
      local b={line='b',order=10,metadata=shared}
      local c={line='c',order=1,crafted=true,metadata={nested=true}}
      local d={line='d',order=90,fractured=true}
      return {rows={a,c,b,d}, original={a,b,c,d},shared=shared}
    "#,
        )
        .eval()
        .unwrap();
    let rows: Table = fixture.get("rows").unwrap();
    let original: Table = fixture.get("original").unwrap();
    let handles = (1..=4)
        .map(|i| original.raw_get::<Table>(i).unwrap())
        .collect::<Vec<_>>();
    let snapshots = handles
        .iter()
        .map(|t| canonical(Value::Table(t.clone())))
        .collect::<Vec<_>>();
    oracle.sort.call::<()>(rows.clone()).unwrap();
    for (index, expected) in [3, 1, 0, 2].into_iter().enumerate() {
        assert_eq!(rows.raw_get::<Table>(index + 1).unwrap(), handles[expected]);
    }
    for (handle, snapshot) in handles.iter().zip(&snapshots) {
        assert_eq!(&canonical(Value::Table(handle.clone())), snapshot);
    }
    assert_eq!(
        handles[0].get::<Table>("metadata").unwrap(),
        handles[1].get::<Table>("metadata").unwrap()
    );
    oracle.sort.call::<()>(rows.clone()).unwrap();
    for (index, expected) in [3, 1, 0, 2].into_iter().enumerate() {
        assert_eq!(rows.raw_get::<Table>(index + 1).unwrap(), handles[expected]);
    }
    let duplicates: Table = lua
        .load(
            "local a={line='a',order=10};local b={line='b',order=10};return {rows={a,b,a},a=a,b=b}",
        )
        .eval()
        .unwrap();
    let duplicate_rows: Table = duplicates.get("rows").unwrap();
    oracle.sort.call::<()>(duplicate_rows.clone()).unwrap();
    assert_eq!(
        duplicate_rows.raw_get::<Table>(1).unwrap(),
        duplicates.get::<Table>("b").unwrap()
    );
    assert_eq!(
        duplicate_rows.raw_get::<Table>(2).unwrap(),
        duplicates.get::<Table>("a").unwrap()
    );
    assert_eq!(
        duplicate_rows.raw_get::<Table>(3).unwrap(),
        duplicates.get::<Table>("a").unwrap()
    );
    let normalization = [
        ("(-10-20)% Increased Armour", "#% increased armour"),
        ("+10.5 to\nMAXIMUM Life", "+# to maximum life"),
        ("-12 to Life", "-# to life"),
    ];
    for (input, expected) in normalization {
        assert_eq!(oracle.normalize.call::<String>(input).unwrap(), expected);
    }
    oracle.verify();
    write(
        "original-helper-identities",
        json!({"source":oracle.provenance(),"scope":"retained original helper on test-owned rows; no native alias comparison for unrepresentable repeated row inputs","same_row_tables_after_sort":true,"shared_nested_metadata_retained":true,"metadata_fields_unchanged":true,"repeat_call_equal":true,"duplicate_alias_source_order_last_index":true,"normalization_cases":normalization}),
    );
}
