//! Buff loading against complete, unchanged original Item/parser methods.
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
use poe_optimizer_data::item_loading::ItemMetadataValue;
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

/// Mutate only test-owned data, identically in the injected catalog and Lua.
/// The original Item/parser functions and their grammar remain unchanged.
fn custom(
    snapshot: &GameDataSnapshot,
    source: &source::Source,
    definitions: &str,
) -> GameDataSnapshot {
    let definitions = source.oracle.lua.load(definitions).eval::<Table>().unwrap();
    let mut package = snapshot.package().clone();
    // Test-owned item definitions invalidate the source-constructed projection.
    package.unique_requirements =
        poe_optimizer_data::unique_requirements::UniqueRequirementData::unavailable(
            "test fixture changes item construction inputs",
        );
    for entry in definitions.pairs::<String, Table>() {
        let (name, parts) = entry.unwrap();
        let base = source.base(&name).unwrap();
        let native = package
            .item_loading
            .bases
            .iter_mut()
            .find(|b| b.name == name)
            .unwrap();
        for family in ["flask", "charm"] {
            let value = parts.get::<Value>(family).unwrap();
            base.set(family, value.clone()).unwrap();
            let wrapper = source.oracle.lua.create_table().unwrap();
            wrapper.set("value", value).unwrap();
            if let Some(value) = reference::metadata(wrapper).fields.remove("value") {
                native.fields.fields.insert(family.into(), value);
            } else {
                native.fields.fields.remove(family);
            }
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

fn before(snapshot: &GameDataSnapshot, source: &source::Source, raw: &str) -> ItemState {
    let (_, error) = source.try_parse(raw);
    assert!(
        !source.stages().is_empty(),
        "loading failed before assembly: {error:?}"
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
        "complete loading calls for {raw}"
    );
    machine.into_state()
}

#[test]
fn all_shipped_charm_buffs_match_original_with_native_parser_and_exact_empty_row() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    let mut count = 0;
    for base in &snapshot.package().item_loading.bases {
        let Some(lines) = base
            .fields
            .fields
            .get("charm")
            .and_then(ItemMetadataValue::as_table)
            .and_then(|t| t.fields.get("buff"))
            .and_then(ItemMetadataValue::as_array)
        else {
            continue;
        };
        let text = lines[0].as_str().unwrap();
        for suffix in [
            String::new(),
            format!("\n{text}\n{text}\nImplicits: 0\n+7 to Strength"),
        ] {
            let raw = format!("Rarity: NORMAL\n{}{}", base.name, suffix);
            source.parse(&raw);
            let mut provider = BuiltinItemLoadProvider::new(&snapshot);
            let mut machine = ItemLoadMachine::new(snapshot.item_loading());
            machine.apply_text(&raw, &mut provider).unwrap();
            assert_eq!(
                machine.pending().map(|p| p.kind),
                Some(DependencyKind::Assembly),
                "{}: {:?}",
                base.name,
                machine.pending()
            );
            reference::compare_state(machine.state(), &source.before());
            assert_eq!(
                machine
                    .state()
                    .parser_calls
                    .iter()
                    .map(|r| (r.text.clone(), r.combined))
                    .collect::<Vec<_>>(),
                source.calls()
            );
            let row = &machine.state().buff_mod_lines[0];
            assert_eq!(row.source_line, 2);
            assert_eq!(row.range, ItemNumber::Nil);
            assert_eq!(row.corrupted_range, ItemNumber::Nil);
            assert_eq!(row.value_scalar, ItemNumber::Nil);
            assert!(row.flags.is_empty() && row.mod_tags.is_empty());
            let first = &machine.state().parser_calls[0];
            assert_eq!(
                (first.line_index, first.sequence, first.combined),
                (2, 0, false)
            );
            assert_eq!(first.text, text);
            if text.is_empty() {
                assert!(row.modifiers.is_empty());
                assert_eq!(row.extra.as_deref(), Some(" "));
            }
            count += 1;
        }
    }
    assert_eq!(count, 26);
    eprintln!("Shipped charm native parser/source parity: {count} complete preassembly states");
}

#[test]
fn independent_family_order_duplicates_and_suppression_precedence_match_original() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    let injected = custom(
        &snapshot,
        &source,
        r#"return {['Ruby Charm']={
      flask={buff={'+10 to Strength','+10 to Strength','--------'},duration=1,chargesUsed=1,chargesMax=1},
      charm={buff={'+10 to Strength','+11 to Dexterity'},duration=1,chargesUsed=1,chargesMax=1}}}"#,
    );
    let mut count = 0;
    for body in [
        "",
        "+10 to Strength",
        "+10 to Strength\n+10 to Strength",
        "+10 to Strength\n+10 to Strength\n+10 to Strength",
        "--------\n+12 to Intelligence",
        "--------\n--------\n+12 to Intelligence",
        "  +10 to Strength  \n+10 to Strength",
        "+10 to strength\n+11 to Dexterity\n+11 to Dexterity",
    ] {
        let raw = format!("Rarity: NORMAL\nRuby Charm\n{body}");
        let state = before(&injected, &source, &raw);
        assert_eq!(
            state
                .buff_mod_lines
                .iter()
                .map(|r| r.line.as_str())
                .collect::<Vec<_>>(),
            [
                "+10 to Strength",
                "+10 to Strength",
                "--------",
                "+10 to Strength",
                "+11 to Dexterity"
            ]
        );
        for (i, request) in state.parser_calls.iter().take(5).enumerate() {
            assert_eq!(
                (request.sequence, request.line_index, request.combined),
                (i, 2, false)
            );
        }
        count += 1;
    }
    eprintln!("Original dual-family/suppression matrix: {count} complete states");
}

#[test]
fn sparse_empty_and_false_metadata_preserve_original_ipairs_prefix_and_initialization() {
    let snapshot = bundled_snapshot().unwrap();
    let mut count = 0;
    for definition in [
        "{charm={buff={}}}",
        "{flask=false,charm={buff={'+11 to Dexterity'}}}",
        "{flask={buff=false},charm={buff={'+11 to Dexterity'}}}",
        "{flask='text parent',charm={buff={'+11 to Dexterity'}}}",
        "{flask={'array parent'},charm={buff={'+11 to Dexterity'}}}",
        "{charm={buff={[0]='ignored',[-1]='ignored',[1]='+10 to Strength',[3]=false,ignored='ignored'}}}",
        "{charm={buff={[2]=false}}}",
        "{charm={buff={'','unknown buff text','+10 to Strength'}}}",
    ] {
        let source = source::Source::new();
        let definitions = [
            "return {['Ruby Charm']=",
            definition,
            ",['Sapphire Charm']={charm={buff={'+25% to Cold Resistance'}}}}",
        ]
        .concat();
        let injected = custom(&snapshot, &source, &definitions);
        for ending in ["", "\nSapphire Charm\n+25% to Cold Resistance"] {
            before(
                &injected,
                &source,
                &format!("Rarity: NORMAL\nRuby Charm{ending}"),
            );
            count += 1;
        }
    }
    eprintln!("Original sparse/empty/false data matrix: {count} complete states");
}

#[test]
fn repeated_selected_bases_and_variant_selection_match_original_once_per_family() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    let injected = custom(
        &snapshot,
        &source,
        r#"return {
      ['Ruby Charm']={charm={buff={'+10 to Strength'}}},
      ['Sapphire Charm']={flask={buff={'+11 to Dexterity'}},charm={buff={'+12 to Intelligence'}}}}
    "#,
    );
    for raw in [
        "Rarity: NORMAL\nRuby Charm\nRuby Charm\nSapphire Charm",
        "Rarity: NORMAL\n{variant:1}Ruby Charm\n{variant:2}Sapphire Charm\nSelected Variant: 2",
        "Rarity: NORMAL\n{variant:1}Ruby Charm\n{variant:2}Sapphire Charm\nSelected Variant: 1",
        "Rarity: NORMAL\nRuby Charm\nSapphire Charm\nRuby Charm\n+10 to Strength\n+11 to Dexterity\n+12 to Intelligence",
    ] {
        before(&injected, &source, raw);
    }
}

#[test]
fn actual_assembly_modifier_updates_and_reparse_reset_generated_buff_rows() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    let mut machine = ItemLoadMachine::new(snapshot.item_loading());
    let mut item = None;
    for raw in [
        "Rarity: NORMAL\nRuby Charm",
        "Rarity: NORMAL\nSapphire Charm",
        "Rarity: NORMAL\nCleansing Charm",
        "Rarity: NORMAL\nRuby Charm\n+25% to Fire Resistance",
        "Rarity: NORMAL\nAmber Amulet",
        "Rarity: NORMAL\nTopaz Charm",
    ] {
        match &item {
            None => item = Some(source.parse(raw)),
            Some(item) => source.reparse(item, raw),
        }
        let mut provider = reference::FrozenAssembly {
            dependencies: reference::OriginalDependencies::new(&source.oracle),
            stages: source.stages().into(),
        };
        machine.apply_text(raw, &mut provider).unwrap();
        assert_eq!(machine.status(), ItemLoadStatus::Complete);
        assert!(provider.stages.is_empty());
        reference::compare_state(machine.state(), &source.snapshot(item.as_ref().unwrap()));
    }
    eprintln!(
        "Real original assembly/reparse: six full loading states and observed payload updates"
    );
}

#[test]
fn malformed_metadata_errors_preserve_original_partial_state_without_fake_string_requests() {
    let snapshot = bundled_snapshot().unwrap();
    for definition in [
        "{flask=true}",
        "{flask=7}",
        "{charm=true}",
        "{charm=7}",
        "{charm={buff=true}}",
        "{charm={buff=7}}",
        "{charm={buff='text'}}",
        "{charm={buff={'+10 to Strength',false,'+11 to Dexterity'}}}",
        "{charm={buff={'+10 to Strength',7,'+11 to Dexterity'}}}",
        "{charm={buff={'+10 to Strength',{},'+11 to Dexterity'}}}",
    ] {
        let source = source::Source::new();
        let injected = custom(
            &snapshot,
            &source,
            &format!("return {{['Ruby Charm']={definition}}}"),
        );
        let item = source.parse("");
        source.clear();
        let raw = "Rarity: NORMAL\nRuby Charm\n+12 to Intelligence";
        let error = item
            .get::<Function>("ParseRaw")
            .unwrap()
            .call::<()>((item.clone(), raw))
            .unwrap_err();
        let mut provider = reference::OriginalDependencies::new(&source.oracle);
        let mut machine = ItemLoadMachine::new(injected.item_loading());
        machine.apply_text(raw, &mut provider).unwrap_err();
        assert_eq!(
            machine.status(),
            ItemLoadStatus::SourceError,
            "{definition}: original {error}"
        );
        reference::compare_state(machine.state(), &source.snapshot(&item));
        assert_eq!(
            provider.calls,
            source.calls(),
            "successful string-call prefix {definition}"
        );
        let attempts = source.attempts();
        if definition.contains("'+10 to Strength'") {
            assert_eq!(attempts.len(), 2);
            assert_ne!(attempts[1].get::<String>("input_type").unwrap(), "string");
            assert!(
                !attempts[1]
                    .get::<Option<bool>>("success")
                    .unwrap()
                    .unwrap_or(false)
            );
            assert_eq!(
                machine.state().parser_calls.len(),
                1,
                "non-string call is an explicit seam rejection, not a fabricated request"
            );
        } else {
            assert!(attempts.is_empty());
        }
        eprintln!(
            "Original malformed boundary {definition}: {}",
            error.to_string().lines().next().unwrap()
        );
    }
}

#[test]
fn native_parser_errors_and_explicit_deferrals_preserve_original_buff_call_prefix() {
    let snapshot = bundled_snapshot().unwrap();
    for (text, source_error) in [
        ("Lose 1..5 maximum Life".to_owned(), true),
        ("Strength is doubled".to_owned(), false),
        (
            "Any number of poisons from this weapon can affect a target at the same time"
                .to_owned(),
            false,
        ),
        (
            "Dexterity from Passives in Radius is Transformed to Intelligence".to_owned(),
            false,
        ),
    ] {
        let source = source::Source::new();
        let definition = format!(
            "return {{['Ruby Charm']={{charm={{buff={{'+10 to Strength','{text}','+11 to Dexterity'}}}}}}}}"
        );
        let injected = custom(&snapshot, &source, &definition);
        let raw = "Rarity: NORMAL\nRuby Charm\n+12 to Intelligence";
        let (_, original_error) = source.try_parse(raw);
        let calls = source.attempts();
        assert!(calls.len() >= 2);
        assert_eq!(calls[1].get::<String>("text").unwrap(), text);
        let mut provider = BuiltinItemLoadProvider::new(&injected);
        let mut machine = ItemLoadMachine::new(injected.item_loading());
        let result = machine.apply_text(raw, &mut provider);
        if source_error {
            assert!(original_error.is_some());
            assert_eq!(calls.len(), 2);
            assert!(result.is_err());
            assert_eq!(machine.status(), ItemLoadStatus::SourceError);
        } else {
            result.unwrap();
            assert_eq!(
                machine.pending().map(|p| p.kind),
                Some(DependencyKind::ModifierParser)
            );
        }
        reference::compare_state(machine.state(), &calls[1].get::<Table>("before").unwrap());
        assert_eq!(machine.state().parser_calls.len(), 2);
        assert_eq!(machine.state().buff_mod_lines.len(), 1);
        assert!(machine.state().format_calls.is_empty());
    }
}

#[test]
fn xml_ranges_address_generated_buffs_before_authored_modifiers_and_reparse_resets_them() {
    let snapshot = bundled_snapshot().unwrap();
    let oracle = runtime::Oracle::new();
    for middle in [
        "",
        "<![CDATA[Rarity: NORMAL\nSapphire Charm\nImplicits: 1\n+10 to Strength\n+11 to Dexterity]]>",
    ] {
        let xml = format!(
            "<Items><Item id=\"7\">Rarity: NORMAL\nRuby Charm\nImplicits: 1\n+10 to Strength\n+11 to Dexterity<ModRange id=\"1\" range=\"0.25\"/><ModRange id=\"2\" range=\"0.5\"/>{middle}<ModRange id=\"3\" range=\"0.75\"/></Item></Items>"
        );
        let loaded = oracle.load(&xml, false);
        assert!(
            loaded.get::<bool>("ok").unwrap(),
            "{:?}",
            loaded.get::<Value>("error").unwrap()
        );
        let stages = loaded
            .get::<Table>("events")
            .unwrap()
            .sequence_values::<Table>()
            .map(Result::unwrap)
            .filter(|event| event.get::<String>("kind").unwrap() == "build_mod_list")
            .skip(1)
            .collect();
        let mut provider = reference::FrozenAssembly {
            dependencies: reference::OriginalDependencies::new(&oracle),
            stages,
        };
        let mut machine = ItemLoadMachine::new(snapshot.item_loading());
        let wrapped = format!("<PathOfBuilding2>{xml}</PathOfBuilding2>");
        let projected = poe_optimizer_import::item_source::project_xml(&wrapped).unwrap();
        let item = &projected.containers()[0].children()[0];
        let attributes = item
            .element()
            .attributes()
            .iter()
            .map(|a| (a.name().to_owned(), a.value().decoded().to_owned()))
            .collect();
        machine.set_xml_attributes(&attributes);
        for entry in item.ordered_content().consumed() {
            match entry {
                poe_optimizer_import::source_xml::PobContentEntry::Text { text, .. } => {
                    machine.apply_text(text, &mut provider).unwrap()
                }
                poe_optimizer_import::source_xml::PobContentEntry::Element { child_index } => {
                    let child = &item.children()[*child_index];
                    machine
                        .apply_mod_range(
                            child.element().attribute("id").map(|a| a.decoded()),
                            child.element().attribute("range").map(|a| a.decoded()),
                        )
                        .unwrap();
                }
            }
        }
        machine.finish_load(&mut provider).unwrap();
        assert_eq!(machine.status(), ItemLoadStatus::Complete);
        assert!(provider.stages.is_empty());
        reference::compare_state(
            machine.state(),
            &loaded
                .get::<Table>("items")
                .unwrap()
                .get::<Table>(1)
                .unwrap(),
        );
        assert_eq!(
            machine.state().buff_mod_lines[0].range,
            if middle.is_empty() {
                ItemNumber::new(0.25)
            } else {
                ItemNumber::Nil
            }
        );
        assert_eq!(
            machine.state().implicit_mod_lines[0].range,
            ItemNumber::new(0.5)
        );
        assert_eq!(
            machine.state().explicit_mod_lines[0].range,
            ItemNumber::new(0.75)
        );
    }
}

#[test]
fn already_initialized_family_skips_bad_buff_but_still_evaluates_later_parent() {
    let snapshot = bundled_snapshot().unwrap();
    for (first, later, fails) in [
        ("{}", "{charm={buff=true}}", false),
        ("{}", "{charm=true}", true),
        ("false", "{charm={buff=true}}", true),
    ] {
        let source = source::Source::new();
        let definitions = [
            "return {['Ruby Charm']={charm={buff=",
            first,
            "}},['Sapphire Charm']=",
            later,
            "}",
        ]
        .concat();
        let injected = custom(&snapshot, &source, &definitions);
        let raw = "Rarity: NORMAL\nRuby Charm\nSapphire Charm\n+12 to Intelligence";
        if fails {
            let (item, original_error) = source.try_parse(raw);
            assert!(original_error.is_some());
            assert!(
                source.stages().is_empty(),
                "original must fail while loading, before assembly"
            );
            let mut provider = reference::OriginalDependencies::new(&source.oracle);
            let mut machine = ItemLoadMachine::new(injected.item_loading());
            machine.apply_text(raw, &mut provider).unwrap_err();
            assert_eq!(machine.status(), ItemLoadStatus::SourceError);
            reference::compare_state(machine.state(), &source.snapshot(&item));
            assert!(provider.calls.is_empty());
        } else {
            let state = before(&injected, &source, raw);
            assert!(state.buff_mod_lines.is_empty());
        }
    }
}
