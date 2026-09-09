//! Affix loading against complete unchanged Item/ParseRaw; ordinary load never calls Craft.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_loading_native.rs"]
mod reference;
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
#[allow(dead_code)]
#[path = "support/base_buff_source.rs"]
mod source;
use mlua::{Function, Table, Value};
use poe_optimizer_data::game_data::{
    GameDataLoader, GameDataSnapshot, TrustPolicy, bundled_snapshot,
};
use poe_optimizer_import::item_loading::*;

fn canonical(value: Value) -> serde_json::Value {
    match value {
        Value::Nil => serde_json::json!(["nil"]),
        Value::Boolean(v) => serde_json::json!(["boolean", v]),
        Value::Integer(v) => {
            serde_json::json!(["number", format!("{:016x}", (v as f64).to_bits())])
        }
        Value::Number(v) => serde_json::json!(["number", format!("{:016x}", v.to_bits())]),
        Value::String(v) => serde_json::json!(["string", v.to_str().unwrap().to_owned()]),
        Value::Table(t) => {
            let mut rows = t
                .pairs::<Value, Value>()
                .map(|r| {
                    let (k, v) = r.unwrap();
                    (canonical(k), canonical(v))
                })
                .collect::<Vec<_>>();
            rows.sort_by_key(|(k, _)| k.to_string());
            serde_json::json!(["table", rows])
        }
        other => panic!("unsupported snapshot {other:?}"),
    }
}

fn before(snapshot: &GameDataSnapshot, source: &source::Source, raw: &str) -> ItemState {
    let (_, error) = source.try_parse(raw);
    assert!(
        !source.stages().is_empty(),
        "original stopped before assembly for {raw}: {error:?}"
    );
    let mut provider = reference::OriginalDependencies::new(&source.oracle);
    let mut machine = ItemLoadMachine::new(snapshot.item_loading());
    machine.apply_text(raw, &mut provider).unwrap();
    assert_eq!(
        machine.pending().map(|p| p.kind),
        Some(DependencyKind::Assembly),
        "{raw}: {:?}",
        machine.pending()
    );
    reference::compare_state(machine.state(), &source.before());
    assert_eq!(
        provider.calls,
        source.calls(),
        "complete parser sequence for {raw}"
    );
    machine.into_state()
}
fn raw(rarity: &str, base: &str, body: &str) -> String {
    let title = if matches!(rarity, "RARE" | "UNIQUE" | "RELIC") {
        "Caller Title\n"
    } else {
        ""
    };
    format!("Rarity: {rarity}\n{title}{base}\n{body}")
}
/// Test-owned data is injected identically; original source functions stay intact.
fn custom_data(
    snapshot: &GameDataSnapshot,
    source: &source::Source,
    script: &str,
) -> GameDataSnapshot {
    let definitions = source.oracle.lua.load(script).eval::<Table>().unwrap();
    let mut package = snapshot.package().clone();
    package.unique_requirements =
        poe_optimizer_data::unique_requirements::UniqueRequirementData::unavailable(
            "test-owned affix/base construction input",
        );
    if let Some(bases) = definitions.get::<Option<Table>>("bases").unwrap() {
        for row in bases.pairs::<String, Table>() {
            let (name, fields) = row.unwrap();
            let original = source.base(&name).unwrap();
            let native = package
                .item_loading
                .bases
                .iter_mut()
                .find(|b| b.name == name)
                .unwrap();
            for (key, value) in reference::metadata(fields.clone()).fields {
                if key == "type" {
                    native.item_type = value.as_str().unwrap().into();
                }
                native.fields.fields.insert(key, value);
            }
            for row in fields.pairs::<String, Value>() {
                let (key, value) = row.unwrap();
                original.set(key, value).unwrap();
            }
        }
    }
    if let Some(tables) = definitions.get::<Option<Table>>("tables").unwrap() {
        let original = source
            .oracle
            .lua
            .globals()
            .get::<Table>("data")
            .unwrap()
            .get::<Table>("itemMods")
            .unwrap();
        for row in tables.pairs::<String, Table>() {
            let (key, table) = row.unwrap();
            package
                .item_loading
                .modifier_tables
                .insert(key.clone(), reference::metadata(table.clone()));
            original.set(key, table).unwrap();
        }
    }
    if let Some(remove) = definitions.get::<Option<Table>>("remove_tables").unwrap() {
        let original = source
            .oracle
            .lua
            .globals()
            .get::<Table>("data")
            .unwrap()
            .get::<Table>("itemMods")
            .unwrap();
        for key in remove.sequence_values::<String>() {
            let key = key.unwrap();
            package.item_loading.modifier_tables.remove(&key);
            original.set(key, Value::Nil).unwrap();
        }
    }
    package.refresh_section_digests().unwrap();
    GameDataLoader::from_bytes(
        &serde_json::to_vec(&package).unwrap(),
        &TrustPolicy::AllowCustom,
        &Default::default(),
    )
    .unwrap()
}
#[test]
fn authored_ranges_fractured_flags_and_nonfinite_values_match_complete_source_tables() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    let values = [
        "None",
        "CallerId",
        "{fractured}CallerId",
        "{fractured}",
        "{fractured}{fractured}CallerId",
        "{range:0}CallerId",
        "{range:-0}CallerId",
        "{range:0x1p-1074}CallerId",
        "{range:1e400}CallerId",
        "{range:-1e400}CallerId",
        "{range:nan}CallerId",
        "{range:invalid}CallerId",
        "{range:invalid,also}CallerId",
        "{range:0,-0,1e400,nan,invalid,0x1p-1074}CallerId",
        "{range:,,}CallerId",
        "{range:,1,,2,}CallerId",
        "{range:}CallerId",
        "{range:0}",
        "before{range:0.25}CallerId",
        "{fractured}{range:0.25,0.75}CallerId",
        "{range:0}None",
        "{range:invalid}None",
        "{range:invalid,also}None",
        "{range:1}{fractured}CallerId",
    ];
    let mut count = 0;
    for header in ["Prefix", "Suffix"] {
        for value in values {
            let body = format!("{header}: {value}\nImplicits: 0\n+10 to maximum Life");
            before(&snapshot, &source, &raw("NORMAL", "Amber Amulet", &body));
            count += 1;
        }
    }
    eprintln!("Affix authored range/fracture source parity: {count} complete states");
}
#[test]
fn reconciliation_rarity_limits_and_rows_beyond_active_slots_match_original() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    let mut count = 0;
    for rarity in ["NORMAL", "MAGIC", "RARE", "UNIQUE", "RELIC"] {
        for base in ["Amber Amulet", "Ruby", "Rusted Greathelm", "Ruby Charm"] {
            for limits in [
                "",
                "+1 Prefix Modifier allowed",
                "-4 Prefix Modifiers allowed",
                "+7 Suffix Modifiers allowed",
                "+1 Prefix Modifier allowed\n-1 Suffix Modifier allowed",
                "Prefix Modifiers allowed",
            ] {
                let body = format!(
                    "Crafted: true\nPrefix: {{range:0.2}}missing\nPrefix: None\nPrefix: {{fractured}}beyond-three\nPrefix: {{range:0.7}}beyond-four\nSuffix: missing\n{limits}"
                );
                before(&snapshot, &source, &raw(rarity, base, &body));
                count += 1;
            }
        }
    }
    let custom = custom_data(
        &snapshot,
        &source,
        "return {bases={Ruby={subType='Abyss'}}}",
    );
    for corrupted in ["", "Corrupted"] {
        for limits in [
            "",
            "+1 Prefix Modifier allowed\n-1 Suffix Modifier allowed",
            "-9 Prefix Modifiers allowed\n+9 Suffix Modifiers allowed",
        ] {
            before(
                &custom,
                &source,
                &raw(
                    "RARE",
                    "Ruby",
                    &format!("Crafted: true\n{corrupted}\nPrefix: missing\n{limits}"),
                ),
            );
            count += 1;
        }
    }
    eprintln!("Affix rarity/limit/outside-slot source parity: {count} complete states");
}
#[test]
fn exact_ids_unique_legacy_missing_and_none_resolve_without_calling_craft() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    let custom = custom_data(
        &snapshot,
        &source,
        "return {tables={Item={Exact={affix='LegacyOne'},Second={affix='LegacyTwo'},SharedA={affix='Shared'},SharedB={affix='Shared'},FalseID=false,TextID='string exact',NumberID=3,None={affix='None label'}}}}",
    );
    let mut count = 0;
    for id in [
        "Exact", "Second", "SharedA", "SharedB", "None", "TextID", "NumberID",
    ] {
        before(
            &custom,
            &source,
            &raw(
                "RARE",
                "Amber Amulet",
                &format!(
                    "Crafted: true\nPrefix: {{fractured}}{{range:0.2,0.8}}{id}\n+10 to maximum Life"
                ),
            ),
        );
        count += 1;
    }
    // Remove malformed records before the legacy scan: source pairs order can
    // otherwise raise on NumberID before finding any requested label.
    let clean = custom_data(
        &snapshot,
        &source,
        "return {tables={Item={Exact={affix='LegacyOne'},Second={affix='LegacyTwo'},SharedA={affix='Shared'},SharedB={affix='Shared'},None={affix='None label'}}}}",
    );
    for id in ["LegacyOne", "LegacyTwo", "missing", "None", "None label"] {
        before(
            &clean,
            &source,
            &raw(
                "RARE",
                "Amber Amulet",
                &format!("Crafted: true\nPrefix: {{range:-0}}{id}"),
            ),
        );
        count += 1;
    }
    let raw = raw("RARE", "Amber Amulet", "Crafted: true\nPrefix: Shared");
    let (_, error) = source.try_parse(&raw);
    assert!(!source.stages().is_empty(), "{error:?}");
    let chosen = source
        .before()
        .get::<Table>("prefixes")
        .unwrap()
        .get::<Table>(1)
        .unwrap()
        .get::<String>("modId")
        .unwrap();
    assert!(["SharedA", "SharedB"].contains(&chosen.as_str()));
    let mut provider = reference::OriginalDependencies::new(&source.oracle);
    let mut machine = ItemLoadMachine::new(clean.item_loading());
    machine.apply_text(&raw, &mut provider).unwrap();
    assert_eq!(
        machine.pending().map(|p| p.kind),
        Some(DependencyKind::CraftedAffixes)
    );
    assert_eq!(machine.state().prefixes.entries[0].mod_id, "Shared");
    assert!(
        machine.state().suffixes.entries.is_empty(),
        "stop before following-list mutation"
    );
    eprintln!(
        "Affix lookup source parity: {count} complete states, one explicit ambiguous boundary"
    );
}
#[test]
fn affix_tables_reset_on_reparse_and_original_assembly_payloads_remain_intact() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    let mut item = None;
    let mut machine = ItemLoadMachine::new(snapshot.item_loading());
    for raw in [
        raw(
            "RARE",
            "Amber Amulet",
            "Crafted: true\nPrefix: missing\n+1 Prefix Modifier allowed\nSuffix: None",
        ),
        raw(
            "NORMAL",
            "Amber Amulet",
            "Prefix: {fractured}{range:invalid,also}CallerId",
        ),
        raw("MAGIC", "Ruby", "Crafted: true\nSuffix: missing"),
        raw("NORMAL", "Amber Amulet", ""),
    ] {
        match &item {
            None => item = Some(source.parse(&raw)),
            Some(item) => source.reparse(item, &raw),
        }
        let mut provider = reference::FrozenAssembly {
            dependencies: reference::OriginalDependencies::new(&source.oracle),
            stages: source.stages().into(),
        };
        machine.apply_text(&raw, &mut provider).unwrap();
        assert_eq!(machine.status(), ItemLoadStatus::Complete);
        assert!(provider.stages.is_empty());
        reference::compare_state(machine.state(), &source.snapshot(item.as_ref().unwrap()));
    }
    eprintln!("Affix original complete assembly/reparse lifecycle: four full states");
}
#[test]
fn original_lua_range_array_nil_insertions_and_nonfinite_source_witness() {
    let source = source::Source::new();
    source.parse(&raw(
        "NORMAL",
        "Amber Amulet",
        "Prefix: {range:bad,,0,-0,nan,1e400,also}CallerId\nSuffix: {range:bad,also}None",
    ));
    let before = source.before();
    let p = before
        .get::<Table>("prefixes")
        .unwrap()
        .get::<Table>(1)
        .unwrap();
    let values = p
        .get::<Table>("range")
        .unwrap()
        .sequence_values::<Value>()
        .map(|v| reference::number(v.unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(values.len(), 4);
    reference::assert_number(values[0], ItemNumber::new(0.0), "zero");
    reference::assert_number(values[1], ItemNumber::new(-0.0), "negative zero");
    assert_eq!(values[2], ItemNumber::NaN);
    assert_eq!(values[3], ItemNumber::new(f64::INFINITY));
    assert_eq!(
        before
            .get::<Table>("suffixes")
            .unwrap()
            .get::<Table>(1)
            .unwrap()
            .get::<Table>("range")
            .unwrap()
            .raw_len(),
        0
    );
    // Every test calls original ParseRaw/BuildModList. Instrumenting Craft would
    // reveal an accidental invocation without replacing its behavior.
    let count=source.oracle.lua.load("return function() local class=common.classes.Item;local old=class.Craft;affixCraftCalls=0;class.Craft=function(self,...)affixCraftCalls=affixCraftCalls+1;return old(self,...)end end").eval::<Function>().unwrap();
    count.call::<()>(()).unwrap();
    source.parse(&raw(
        "RARE",
        "Amber Amulet",
        "Crafted: true\nPrefix: missing\n+10 to maximum Life",
    ));
    let xml_raw = raw(
        "RARE",
        "Amber Amulet",
        "Crafted: true\nPrefix: missing\n+10 to maximum Life",
    );
    for instrument in [false, true] {
        let loaded = source.oracle.load(
            &format!("<Items><Item id=\"7\"><![CDATA[{xml_raw}]]></Item></Items>"),
            instrument,
        );
        assert!(
            loaded.get::<bool>("ok").unwrap(),
            "{:?}",
            loaded.get::<Value>("error").unwrap()
        );
    }
    assert_eq!(
        source
            .oracle
            .lua
            .globals()
            .get::<usize>("affixCraftCalls")
            .unwrap(),
        0
    );
}

#[test]
fn original_malformed_affix_metadata_and_numeric_legacy_key_source_witness() {
    let source = source::Source::new();
    let item_mods = source
        .oracle
        .lua
        .globals()
        .get::<Table>("data")
        .unwrap()
        .get::<Table>("itemMods")
        .unwrap();
    for (definition, id, expected, error_expected) in [
        ("{Only='text'}", "missing", Some("None"), false),
        ("{Only={}}", "missing", Some("None"), false),
        ("{Only={'array'}}", "missing", Some("None"), false),
        ("{Only=false}", "Only", None, true),
        ("{Only=false}", "missing", None, true),
        ("{Only=3}", "Only", Some("Only"), false),
        ("{Only=3}", "missing", None, true),
        ("{Only=true}", "Only", Some("Only"), false),
        ("{Only=true}", "missing", None, true),
        ("{Only='text'}", "Only", Some("Only"), false),
        ("{[3]={affix='Legacy'}}", "missing", Some("None"), false),
    ] {
        let table = source
            .oracle
            .lua
            .load(format!("return {definition}"))
            .eval::<Table>()
            .unwrap();
        item_mods.set("Item", table).unwrap();
        let raw = raw(
            "RARE",
            "Amber Amulet",
            &format!("Crafted: true\nPrefix: {{fractured}}{{range:0.2}}{id}"),
        );
        let (item, error) = source.try_parse(&raw);
        if error_expected {
            let error = error
                .expect("original indexes malformed child .affix")
                .to_string();
            assert!(error.contains("index"), "{error}");
            let state = source.snapshot(&item);
            let prefixes = state.get::<Table>("prefixes").unwrap();
            assert_eq!(prefixes.raw_len(), 1, "error precedes following slots");
            assert_eq!(
                prefixes
                    .get::<Table>(1)
                    .unwrap()
                    .get::<String>("modId")
                    .unwrap(),
                id
            );
            assert_eq!(
                state.get::<Table>("suffixes").unwrap().raw_len(),
                0,
                "error precedes following list"
            );
            assert_eq!(
                state.get::<f64>("affixLimit").unwrap(),
                6.0,
                "limit assignment precedes lookup error"
            );
        } else {
            assert!(!source.stages().is_empty(), "{definition}/{id}: {error:?}");
            assert_eq!(
                source
                    .before()
                    .get::<Table>("prefixes")
                    .unwrap()
                    .get::<Table>(1)
                    .unwrap()
                    .get::<String>("modId")
                    .unwrap(),
                expected.unwrap()
            );
        }
        eprintln!(
            "Original affix metadata witness {definition}, id={id}: source_error={error_expected}"
        );
    }
    item_mods
        .set(
            "Item",
            source
                .oracle
                .lua
                .load("return {[3]={affix='Legacy'}}")
                .eval::<Table>()
                .unwrap(),
        )
        .unwrap();
    let (_, error) = source.try_parse(&raw(
        "RARE",
        "Amber Amulet",
        "Crafted: true\nPrefix: Legacy",
    ));
    assert!(!source.stages().is_empty(), "{error:?}");
    assert_eq!(
        source
            .before()
            .get::<Table>("prefixes")
            .unwrap()
            .get::<Table>(1)
            .unwrap()
            .get::<i64>("modId")
            .unwrap(),
        3,
        "source numeric ID remains numeric"
    );
    eprintln!("Original numeric-key legacy resolution retains numeric modId 3");
}

#[test]
fn prebase_repeated_base_variant_and_section_order_preserve_all_authored_rows() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    for text in [
        "Rarity: NORMAL\nPrefix: {range:0.2}BeforeBase\nAmber Amulet\nSuffix: AfterBase",
        "Rarity: RARE\nCaller Title\nPrefix: BeforeBase\nAmber Amulet\nCrafted: true\nSuffix: AfterBase",
        "Rarity: NORMAL\nAmber Amulet\nPrefix: First\nRusted Greathelm\nSuffix: Second\nPrefix: Third",
        "Rarity: NORMAL\n{variant:1}Amber Amulet\n{variant:2}Rusted Greathelm\nSelected Variant: 2\nPrefix: {variant:1}First\nSuffix: {variant:2}Second",
        "Rarity: NORMAL\nAmber Amulet\nImplicits: 1\n+10 to maximum Life\nPrefix: {range:0.2}CallerId\n+11 to Dexterity\nSuffix: None",
        "Rarity: RARE\nCaller Title\nAmber Amulet\nCrafted: false\nPrefix: {fractured}missing\n{crafted}+10 to maximum Life\nSuffix: None",
    ] {
        before(&snapshot, &source, text);
    }
    eprintln!("Affix base/variant/section ordering: six complete source states");
}
#[test]
fn every_injected_modifier_family_accepts_its_exact_id_without_category_filter() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    let mut count = 0;
    for (family, table) in &snapshot.package().item_loading.modifier_tables {
        let id = table.fields.keys().next().unwrap();
        let script = format!("return {{bases={{['Amber Amulet']={{type={family:?}}}}}}}");
        let injected = custom_data(&snapshot, &source, &script);
        let state = before(
            &injected,
            &source,
            &raw(
                "RARE",
                "Amber Amulet",
                &format!("Crafted: true\nPrefix: {{range:0.2}}{id}"),
            ),
        );
        assert_eq!(state.prefixes.entries[0].mod_id, *id);
        count += 1;
    }
    assert_eq!(count, 9);
    eprintln!("Affix exact lookup across all nine injected table families");
}

#[test]
fn absent_affix_family_clears_crafted_without_erasing_authored_lists() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    let injected = custom_data(
        &snapshot,
        &source,
        "return {bases={['Amber Amulet']={type='CallerTypeWithoutAffixes'}},remove_tables={'Item'}}",
    );
    let state = before(
        &injected,
        &source,
        &raw(
            "RARE",
            "Amber Amulet",
            "Crafted: true\nPrefix: {fractured}{range:0.2,0.8}CallerId\nSuffix: None",
        ),
    );
    assert_eq!(
        state.retained_fields.get("crafted"),
        Some(&ItemScalar::Boolean(false))
    );
    assert_eq!(state.prefixes.entries[0].mod_id, "CallerId");
    assert_eq!(state.suffixes.entries.len(), 1);
}
#[test]
fn original_limit_overflow_and_disabled_line_order_keep_exact_numeric_state() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    let huge = "9".repeat(400);
    for rarity in ["NORMAL", "MAGIC", "RARE"] {
        for line in [
            format!("+{huge} prefix modifiers allowed"),
            format!("-{huge} prefix modifiers allowed"),
            format!("+{huge} prefix modifiers allowed -{huge} prefix modifiers allowed"),
            "{disabled}+1 prefix modifier allowed".into(),
            "{variant:2}+1 prefix modifier allowed".into(),
            "+1 prefix modifier allowed +1 suffix modifier allowed".into(),
        ] {
            before(
                &snapshot,
                &source,
                &raw(
                    rarity,
                    "Amber Amulet",
                    &format!("Crafted: true\nPrefix: None\n{line}"),
                ),
            );
        }
    }
    eprintln!("Affix overflow/disabled/overlapping limit-rule source parity: 18 complete states");
}
#[test]
fn malformed_or_numeric_key_fallback_defers_without_inventing_a_miss() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    for definition in [
        "{Only=false}",
        "{Only=3}",
        "{Only=true}",
        "{[3]={affix='Legacy'}}",
    ] {
        let injected = custom_data(
            &snapshot,
            &source,
            &format!("return {{tables={{Item={definition}}}}}"),
        );
        let text = raw(
            "RARE",
            "Amber Amulet",
            "Crafted: true\nPrefix: {fractured}{range:0.2}Legacy",
        );
        let (item, error) = source.try_parse(&text);
        let mut provider = reference::OriginalDependencies::new(&source.oracle);
        let mut machine = ItemLoadMachine::new(injected.item_loading());
        machine.apply_text(&text, &mut provider).unwrap();
        assert_eq!(
            machine.pending().map(|p| p.kind),
            Some(DependencyKind::CraftedAffixes)
        );
        assert_eq!(machine.state().prefixes.entries[0].mod_id, "Legacy");
        assert!(machine.state().suffixes.entries.is_empty());
        assert_eq!(
            machine.state().assembly_calls,
            1,
            "only the initial empty constructor assembly"
        );
        if !definition.contains("[3]") {
            assert!(error.is_some(), "original malformed child raises");
            reference::compare_state(machine.state(), &source.snapshot(&item));
        } else {
            assert!(!source.stages().is_empty(), "{error:?}");
            assert_eq!(
                source
                    .before()
                    .get::<Table>("prefixes")
                    .unwrap()
                    .get::<Table>(1)
                    .unwrap()
                    .get::<i64>("modId")
                    .unwrap(),
                3
            );
        }
    }
}

#[test]
fn no_base_retains_authored_affix_tables_and_original_call_count() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    for text in [
        "Rarity: NORMAL\nCaller Missing Base\nPrefix: {fractured}{range:0,-0}CallerId",
        "Rarity: RARE\nCaller Title\nCaller Missing Base\nCrafted: true\nPrefix: CallerId\nSuffix: None",
    ] {
        let (item, error) = source.try_parse(text);
        assert!(error.is_none(), "{error:?}");
        let mut provider = reference::OriginalDependencies::new(&source.oracle);
        let mut machine = ItemLoadMachine::new(snapshot.item_loading());
        machine.apply_text(text, &mut provider).unwrap();
        assert_eq!(machine.status(), ItemLoadStatus::NoBase);
        assert_eq!(
            machine.state().assembly_calls,
            1 + source.stages().len(),
            "empty constructor plus original no-base BuildModList"
        );
        reference::compare_state(machine.state(), &source.snapshot(&item));
    }
}

#[test]
fn original_no_base_reparse_retains_selected_affix_family_source_witness() {
    let source = source::Source::new();
    source
        .oracle
        .lua
        .globals()
        .get::<Table>("data")
        .unwrap()
        .get::<Table>("itemMods")
        .unwrap()
        .set(
            "Item",
            source
                .oracle
                .lua
                .load("return {Exact={affix='LegacyOne'}}")
                .eval::<Table>()
                .unwrap(),
        )
        .unwrap();
    let item = source.parse(&raw("RARE", "Amber Amulet", "Crafted: true\nPrefix: Exact"));
    source.reparse(
        &item,
        &raw("RARE", "Caller Missing Base", "Prefix: LegacyOne"),
    );
    let state = source.snapshot(&item);
    assert!(!state.get::<bool>("hasBase").unwrap());
    assert_eq!(
        state.get::<String>("selectedAffixesTableKey").unwrap(),
        "Item"
    );
    assert!(state.get::<bool>("crafted").unwrap());
    assert_eq!(
        state
            .get::<Table>("prefixes")
            .unwrap()
            .get::<Table>(1)
            .unwrap()
            .get::<String>("modId")
            .unwrap(),
        "Exact"
    );
    assert_eq!(state.get::<Table>("prefixes").unwrap().raw_len(), 3);
    assert_eq!(state.get::<Table>("suffixes").unwrap().raw_len(), 3);
    eprintln!(
        "Original no-base reparse retains prior affix family/crafted flag and reconciles fresh rows"
    );
}

/// Parameterized primitive oracle. Only literal patterns/default references in
/// the exact original header block are replaced; this is explicitly separate
/// from the unchanged full-Item source parity above.
fn parameterized_header(source: &source::Source) -> Function {
    let original = runtime::verified("src/Classes/Item.lua").unwrap();
    let begin = "\t\t\t\t\tlocal fractured = specVal:match";
    let end = "\t\t\t\telseif specName == \"Implicits\" then";
    assert_eq!(original.matches(begin).count(), 1);
    let start = original.find(begin).unwrap();
    let mut body = original[start..start + original[start..].find(end).unwrap()].to_owned();
    for (original, replacement) in [
        (
            "specVal:match(\"^{fractured}\")",
            "specVal:match(policy.fractured_pattern)",
        ),
        (
            "specVal:gsub(\"^{fractured}\", \"\")",
            "specVal:gsub(policy.fractured_remove_pattern, \"\")",
        ),
        (
            "specVal:match(\"{range:([^}]+)}(.+)\")",
            "specVal:match(policy.range_pattern)",
        ),
        (
            "range:find(\",\", 1, true)",
            "range:find(policy.range_separator, 1, true)",
        ),
        (
            "range:gmatch(\"[^,]+\")",
            "range:gmatch(policy.range_value_pattern)",
        ),
        ("~= \"None\"", "~= policy.none_mod_id"),
        ("main.defaultItemAffixQuality", "quality"),
    ] {
        assert_eq!(body.matches(original).count(), 1, "{original}");
        body = body.replace(original, replacement);
    }
    source.oracle.lua.load(format!("return function(specVal,policy,quality) local affixes={{}};local t_insert=table.insert;{body}\nreturn affixes[1] end")).set_name("@parameterized-original-affix-header-block").eval().unwrap()
}
#[test]
fn parameterized_original_header_zero_length_caret_and_position_capture_primitives() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    let header = parameterized_header(&source);
    let mut count = 0;
    for (field, pattern, value) in [
        ("fractured_pattern", "^()", "CallerId"),
        ("fractured_remove_pattern", "a*", "banana"),
        ("fractured_remove_pattern", "^a*", "aaab"),
        ("fractured_remove_pattern", "a-", "banana"),
        ("fractured_remove_pattern", "()", "CallerId"),
        ("fractured_remove_pattern", "$", "CallerId"),
        ("fractured_remove_pattern", ".*", "CallerId"),
        ("range_value_pattern", "^%d", "{range:1,2}CallerId"),
        ("range_value_pattern", "^(%d)", "{range:^1,^2}CallerId"),
        ("range_value_pattern", "()", "{range:1,2}CallerId"),
        ("range_value_pattern", "%d*", "{range:1,2}CallerId"),
        ("range_value_pattern", ".-", "{range:1,2}CallerId"),
        ("range_value_pattern", "(%d)()", "{range:1,2}CallerId"),
        ("range_separator", "", "{range:12}CallerId"),
        ("range_pattern", "CallerId", "CallerId"),
        ("range_value_pattern", "()%d*", "{range:1,2}CallerId"),
        ("range_pattern", "()", "CallerId"),
        ("range_pattern", "(%a+)()", "CallerId"),
    ] {
        // Only the directly validated catalog is consumed by this primitive
        // fixture. Whole-package file injection remains covered by CLI tests.
        let mut data = snapshot.package().item_loading.clone();
        let policy = &mut data.policy.affix_loading;
        match field {
            "fractured_pattern" => policy.fractured_pattern = pattern.into(),
            "fractured_remove_pattern" => policy.fractured_remove_pattern = pattern.into(),
            "range_value_pattern" => policy.range_value_pattern = pattern.into(),
            "range_separator" => policy.range_separator = pattern.into(),
            "range_pattern" => policy.range_pattern = pattern.into(),
            _ => unreachable!(),
        }
        let params = source.oracle.lua.create_table().unwrap();
        for (key, value) in [
            ("fractured_pattern", policy.fractured_pattern.as_str()),
            (
                "fractured_remove_pattern",
                policy.fractured_remove_pattern.as_str(),
            ),
            ("range_pattern", policy.range_pattern.as_str()),
            ("range_separator", policy.range_separator.as_str()),
            ("range_value_pattern", policy.range_value_pattern.as_str()),
            ("none_mod_id", policy.none_mod_id.as_str()),
        ] {
            params.set(key, value).unwrap();
        }
        let original = header.call::<Table>((value, params, data.policy.default_affix_quality));
        let injected = poe_optimizer_data::item_loading::ItemLoadingCatalog::new(data).unwrap();
        let mut machine = ItemLoadMachine::new(&injected);
        let mut provider = reference::OriginalDependencies::new(&source.oracle);
        let result = machine.apply_text(
            &raw("NORMAL", "Amber Amulet", &format!("Prefix: {value}")),
            &mut provider,
        );
        match original {
            Err(error) => {
                assert!(error.to_string().contains("index"), "{error}");
                assert!(result.is_err(), "{field}={pattern}: original raises");
                assert_eq!(machine.status(), ItemLoadStatus::SourceError);
            }
            Ok(row) if !matches!(row.get::<Value>("modId").unwrap(), Value::String(_)) => {
                result.unwrap();
                assert_eq!(
                    machine.pending().map(|p| p.kind),
                    Some(DependencyKind::CraftedAffixes),
                    "numeric modId is not an original source error"
                );
            }
            Ok(row) => {
                result.unwrap();
                assert_eq!(
                    machine.pending().map(|p| p.kind),
                    Some(DependencyKind::Assembly),
                    "{field}={pattern}: {:?}",
                    machine.pending()
                );
                let list = source.oracle.lua.create_table().unwrap();
                list.set(1, row).unwrap();
                reference::compare_affixes(
                    &machine.state().prefixes,
                    list,
                    &format!("parameterized {field}={pattern}"),
                );
            }
        }
        count += 1;
    }
    eprintln!("Parameterized exact original header block: {count} zero-length/caret/capture cases");
}

#[test]
fn retained_family_reparse_matches_no_base_and_jewel_source_error_prefixes() {
    let snapshot = bundled_snapshot().unwrap();
    for base in ["Amber Amulet", "Ruby"] {
        let source = source::Source::new();
        let first = raw("RARE", base, "Crafted: true\nPrefix: CallerMissing");
        let item = source.parse(&first);
        let mut machine = ItemLoadMachine::new(snapshot.item_loading());
        let mut assembly = reference::FrozenAssembly {
            dependencies: reference::OriginalDependencies::new(&source.oracle),
            stages: source.stages().into(),
        };
        machine.apply_text(&first, &mut assembly).unwrap();
        assert_eq!(machine.status(), ItemLoadStatus::Complete);
        assert!(assembly.stages.is_empty());
        let next = raw(
            "RARE",
            "Caller Missing Base",
            "Prefix: {fractured}{range:-0}CallerMissing",
        );
        source.clear();
        let original_error = item
            .get::<Function>("ParseRaw")
            .unwrap()
            .call::<()>((item.clone(), next.as_str()))
            .err();
        let result = machine.apply_text(
            &next,
            &mut reference::OriginalDependencies::new(&source.oracle),
        );
        if base == "Ruby" {
            assert!(original_error.unwrap().to_string().contains("base"));
            assert!(result.is_err());
            assert_eq!(machine.status(), ItemLoadStatus::SourceError);
        } else {
            assert!(original_error.is_none());
            result.unwrap();
            assert_eq!(machine.status(), ItemLoadStatus::NoBase);
        }
        reference::compare_state(machine.state(), &source.snapshot(&item));
    }
    eprintln!("Retained affix family: original NoBase and Jewel source-error prefixes paired");
}
