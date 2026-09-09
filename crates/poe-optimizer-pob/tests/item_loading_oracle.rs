//! Independent source Item/ItemsTab load observations, separate from numerical native coverage.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Table, Value};
use sha2::{Digest, Sha256};
#[path = "support/item_loading_native.rs"]
mod native;
#[path = "support/item_loading_runtime.rs"]
mod runtime;
use poe_optimizer_data::game_data::bundled_snapshot;
use poe_optimizer_import::item_loading::*;
use std::collections::BTreeMap;
#[test]
fn original_complete_item_constructor_bootstraps_from_authenticated_modules() {
    let oracle = runtime::Oracle::new();
    let item = oracle.parse("Rarity: Normal\nRusted Greathelm\nItem Level: 1\nQuality: 0\n");
    assert_eq!(item.get::<String>("baseName").unwrap(), "Rusted Greathelm");
    assert!(item.get::<mlua::Table>("modList").is_ok());
}

#[test]
fn original_items_load_uses_complete_parser_and_rebuilds_after_ranges() {
    let oracle = runtime::Oracle::new();
    let xml = "<Items><Item id=\"7\">Rarity: Normal\nRusted Greathelm\nItem Level: 1\nQuality: 0\n+(10-20) to maximum Life<ModRange id=\"1\" range=\"0.25\"/></Item></Items>";
    let loaded = oracle.load(xml, true);
    assert!(
        loaded.get::<bool>("ok").unwrap(),
        "{:?}",
        loaded.get::<Option<String>>("error").unwrap()
    );
    let items = loaded.get::<mlua::Table>("items").unwrap();
    assert_eq!(items.raw_len(), 1);
    assert_eq!(
        items
            .get::<mlua::Table>(1)
            .unwrap()
            .get::<String>("baseName")
            .unwrap(),
        "Rusted Greathelm"
    );
}

fn canonical(value: Value) -> serde_json::Value {
    canonical_inner(value, 0)
}
fn canonical_inner(value: Value, depth: usize) -> serde_json::Value {
    assert!(
        depth < 32,
        "unexpected observed nesting depth {depth}, {value:?}"
    );
    match value {
        Value::Nil => serde_json::json!(["nil"]),
        Value::Boolean(v) => serde_json::json!(["boolean", v]),
        Value::Integer(v) => serde_json::json!(["number", (v as f64).to_bits().to_string()]),
        Value::Number(v) => serde_json::json!(["number", v.to_bits().to_string()]),
        Value::String(v) => serde_json::json!(["string", v.to_str().unwrap().to_owned()]),
        Value::Table(t) => {
            let mut rows = t
                .pairs::<Value, Value>()
                .map(|r| {
                    let (k, v) = r.unwrap();
                    (canonical_inner(k, depth + 1), canonical_inner(v, depth + 1))
                })
                .collect::<Vec<_>>();
            rows.sort_by_key(|(k, _)| k.to_string());
            serde_json::json!(["table", rows])
        }
        Value::Function(f) => {
            let info = f.info();
            serde_json::json!([
                "function_descriptor_only",
                info.source,
                info.line_defined,
                info.last_line_defined
            ])
        }
        other => panic!("unsupported observed value {other:?}"),
    }
}
fn json(_oracle: &runtime::Oracle, table: Table) -> serde_json::Value {
    canonical(Value::Table(table))
}
fn events(result: &Table, kind: &str) -> Vec<Table> {
    result
        .get::<Table>("events")
        .unwrap()
        .sequence_values::<Table>()
        .map(Result::unwrap)
        .filter(|v| v.get::<String>("kind").unwrap() == kind)
        .collect()
}
#[test]
fn all_caller_items_execute_original_parse_and_range_writes_without_observer_changes() {
    let oracle = runtime::Oracle::new();
    let directory = runtime::repository().join("tests/fixtures/builds/breadth-20260908");
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("index.json")).unwrap()).unwrap();
    let mut counts = (0, 0, 0, 0);
    for row in index["builds"].as_array().unwrap() {
        let path = directory.join(row["xml"].as_str().unwrap());
        let before = std::fs::read(&path).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&before)),
            row["xml_sha256"].as_str().unwrap()
        );
        let xml = std::str::from_utf8(&before).unwrap();
        let plain = oracle.load(xml, false);
        let observed = oracle.load(xml, true);
        assert!(
            plain.get::<bool>("ok").unwrap(),
            "{}: {:?}",
            path.display(),
            plain.get::<Option<String>>("error").unwrap()
        );
        assert!(
            observed.get::<bool>("ok").unwrap(),
            "{}: {:?}",
            path.display(),
            observed.get::<Option<String>>("error").unwrap()
        );
        for key in ["items", "order", "sets", "setOrder"] {
            assert_eq!(
                json(&oracle, plain.get::<Table>(key).unwrap()),
                json(&oracle, observed.get::<Table>(key).unwrap()),
                "{}: {key}",
                path.display()
            );
        }
        assert_eq!(
            plain.get::<usize>("activeSet").unwrap(),
            observed.get::<usize>("activeSet").unwrap()
        );
        let projected = poe_optimizer_import::item_source::project_xml(xml).unwrap();
        assert_eq!(projected.source_xml(), xml);
        let item_count = observed.get::<Table>("items").unwrap().raw_len();
        let parses = events(&observed, "parse_raw");
        assert_eq!(
            parses.len(),
            2 * item_count,
            "empty constructor plus one authored text per corpus item"
        );
        assert_eq!(events(&observed, "build_mod_list").len(), 3 * item_count);
        counts.0 += item_count;
        counts.1 += observed.get::<Table>("setOrder").unwrap().raw_len();
        counts.2 += events(&observed, "range").len();
        counts.3 += roxmltree::Document::parse(xml)
            .unwrap()
            .descendants()
            .filter(|n| n.has_tag_name("ModRange"))
            .count();
        assert!(
            events(&observed, "range")
                .iter()
                .all(|r| r.get::<String>("list").unwrap() != "runeModLines")
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    assert_eq!(counts, (116, 15, 402, 486));
}

fn escaped_xml(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
#[test]
fn native_single_combined_and_failed_retry_match_original_preassembly_state() {
    let oracle = runtime::Oracle::new();
    let snapshot = bundled_snapshot().unwrap();
    let mut combined_calls = 0;
    for body in [
        "",
        "+10 to maximum Life",
        "10% increased\nArmour",
        "unknown unrelated text\n+10 to maximum Life",
        "Foo:: value\nClass:: Warrior\n+10 to maximum Life",
        "{enchant}+10 to Strength\n{implicit}+11 to Dexterity\n+12 to Intelligence",
    ] {
        for newline in ["\n", "\r\n"] {
            let raw=format!("Rarity: Rare\nCaller name\nRusted Greathelm\nItem Level: 60\nQuality: 0\nImplicits: 0\n{body}").replace('\n',newline);
            let source = oracle.load(
                &format!("<Items><Item id=\"7\">{}</Item></Items>", escaped_xml(&raw)),
                false,
            );
            assert!(
                source.get::<bool>("ok").unwrap(),
                "{:?}",
                source.get::<Option<String>>("error").unwrap()
            );
            let parse = events(&source, "parse_raw").remove(1);
            // Original XML removes boundary whitespace before direct ParseRaw.
            let consumed = parse.get::<String>("raw").unwrap();
            let mut machine = ItemLoadMachine::new(snapshot.item_loading());
            machine.set_xml_attributes(&BTreeMap::from([("id".into(), "7".into())]));
            let mut provider = native::OriginalDependencies::new(&oracle);
            machine.apply_text(&consumed, &mut provider).unwrap();
            assert_eq!(
                machine.pending().map(|p| p.kind),
                Some(DependencyKind::Assembly),
                "{body}: {:?}",
                machine.pending()
            );
            let expected = events(&source, "build_mod_list")
                .remove(1)
                .get::<Table>("before")
                .unwrap();
            native::compare_state(machine.state(), &expected);
            let calls = parse
                .get::<Table>("calls")
                .unwrap()
                .sequence_values::<Table>()
                .map(Result::unwrap)
                .filter(|r| r.get::<String>("phase").unwrap() == "parse_raw")
                .map(|r| {
                    (
                        r.get::<String>("text").unwrap(),
                        r.get::<bool>("combined").unwrap(),
                    )
                })
                .collect::<Vec<_>>();
            combined_calls += calls.iter().filter(|(_, c)| *c).count();
            assert_eq!(provider.calls, calls, "{body}");
        }
    }
    assert!(combined_calls >= 4);
}
#[test]
fn actual_lua_numbers_and_ggg_strings_match_native_xml_and_text_inputs() {
    let oracle = runtime::Oracle::new();
    let snapshot = bundled_snapshot().unwrap();
    for (expression, expected) in [
        ("math.min(0/0,1)", ItemNumber::Finite(1.0)),
        ("math.min(1,0/0)", ItemNumber::NaN),
        ("math.max(0/0,1)", ItemNumber::Finite(1.0)),
        ("math.max(1,0/0)", ItemNumber::NaN),
    ] {
        let v: Value = oracle
            .lua
            .load(format!("return {expression}"))
            .eval()
            .unwrap();
        native::assert_number(native::number(v), expected, expression);
    }
    let tonumber = oracle
        .lua
        .globals()
        .get::<mlua::Function>("tonumber")
        .unwrap();
    for text in [
        "inf",
        "+inf",
        "-inf",
        "infinity",
        "+infinity",
        "-infinity",
        "INF",
        "NaN",
        "nan",
        "+nan",
        "-nan",
        "nan(1)",
        "0x1p2",
        "0x1.8p1",
        "-0x1.8p1",
        "0X.8P-1",
        "0x1p1024",
        "  2	",
        "1e309",
        "-1e309",
        "-0",
        "0x0p-9",
        "0x1p-1074",
        "0x1p-1075",
    ] {
        let mut machine = ItemLoadMachine::new(snapshot.item_loading());
        machine.set_xml_attributes(&BTreeMap::from([
            ("id".into(), text.into()),
            ("variant".into(), text.into()),
        ]));
        let expected = native::number(tonumber.call(text).unwrap());
        let actual = match machine.state().retained_fields.get("id") {
            Some(ItemScalar::Number(n)) => *n,
            None => ItemNumber::Nil,
            v => panic!("{v:?}"),
        };
        native::assert_number(actual, expected, text);
        native::assert_number(
            machine.state().variants.selected.unwrap_or(ItemNumber::Nil),
            expected,
            text,
        );
    }
    let escape = oracle
        .lua
        .globals()
        .get::<mlua::Function>("escapeGGGString")
        .unwrap();
    for text in [
        "[a|b]tail]",
        "[a]x|b]",
        "[[a|b]c]",
        "[a|b|c]",
        "[a|b][c|d]",
        "[a[b|c]d]",
        "<tag>{text}",
        "[Alpha]",
        "plain",
    ] {
        let mut machine = ItemLoadMachine::new(snapshot.item_loading());
        machine
            .apply_text(text, &mut UnavailableItemLoadProvider)
            .unwrap();
        assert_eq!(
            machine.state().raw_lines,
            vec![escape.call::<String>(text).unwrap()]
        );
    }
    let syntax = oracle
        .lua
        .globals()
        .get::<Table>("itemSyntax")
        .unwrap()
        .get::<mlua::Function>("specToNumber")
        .unwrap();
    for text in [
        "-1.5",
        "+.5",
        "1e3",
        "0x10",
        "1.2.3",
        " +1",
        "+1.25suffix",
        ".",
        "+",
        "inf",
        "-0",
    ] {
        native::assert_number(
            spec_to_number(text),
            native::number(syntax.call(text).unwrap()),
            text,
        );
    }
}

#[test]
fn complete_constructed_source_base_catalog_matches_injected_definitions() {
    let oracle = runtime::Oracle::new();
    let snapshot = bundled_snapshot().unwrap();
    let bases = oracle
        .lua
        .globals()
        .get::<Table>("data")
        .unwrap()
        .get::<Table>("itemBases")
        .unwrap();
    let catalog = snapshot.item_loading();
    assert_eq!(bases.clone().pairs::<String, Table>().count(), 1756);
    assert_eq!(catalog.bases().len(), 1756);
    for base in catalog.bases() {
        let original = bases.get::<Table>(base.name.as_str()).unwrap();
        assert_eq!(native::metadata(original), base.fields, "{}", base.name);
    }
    let data = oracle.lua.globals().get::<Table>("data").unwrap();
    let original_mods = data.get::<Table>("itemMods").unwrap();
    assert_eq!(
        original_mods.clone().pairs::<String, Table>().count(),
        catalog.data().modifier_tables.len()
    );
    for (key, value) in &catalog.data().modifier_tables {
        assert_eq!(
            native::metadata(original_mods.get::<Table>(key.as_str()).unwrap()),
            *value,
            "modifier group {key}"
        );
    }
    let uniques = data.get::<Table>("uniques").unwrap();
    assert_eq!(
        uniques.clone().pairs::<String, Table>().count(),
        catalog.unique_groups().len()
    );
    for (key, values) in catalog.unique_groups() {
        assert_eq!(
            uniques
                .get::<Table>(key.as_str())
                .unwrap()
                .sequence_values::<String>()
                .map(Result::unwrap)
                .collect::<Vec<_>>(),
            *values,
            "unique group {key}"
        );
    }
    assert_eq!(
        native::metadata(oracle.lua.globals().get::<Table>("rawJewelRadii").unwrap()),
        catalog.data().jewel_radii
    );
    let policy = oracle.lua.globals().get::<Table>("itemPolicy").unwrap();
    let names = policy.get::<Table>("catalysts").unwrap();
    let descriptors = policy.get::<Table>("descriptors").unwrap();
    let tags = policy.get::<Table>("tags").unwrap();
    assert_eq!(names.raw_len(), catalog.policy().catalysts.len());
    for (i, c) in catalog.policy().catalysts.iter().enumerate() {
        assert_eq!(names.get::<String>(i + 1).unwrap(), c.name);
        assert_eq!(descriptors.get::<String>(i + 1).unwrap(), c.descriptor);
        assert_eq!(
            tags.get::<Table>(i + 1)
                .unwrap()
                .sequence_values::<String>()
                .map(Result::unwrap)
                .collect::<Vec<_>>(),
            c.tags
        );
    }
    let flags = policy
        .get::<Table>("line_flags")
        .unwrap()
        .pairs::<String, bool>()
        .map(|r| {
            let (k, v) = r.unwrap();
            assert!(v);
            k
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(flags, catalog.policy().line_flags);
    let main = oracle.lua.globals().get::<Table>("main").unwrap();
    assert_eq!(
        main.get::<f64>("defaultItemQuality").unwrap().to_bits(),
        catalog.policy().default_item_quality.to_bits()
    );
    assert_eq!(
        main.get::<f64>("defaultItemAffixQuality")
            .unwrap()
            .to_bits(),
        catalog.policy().default_affix_quality.to_bits()
    );
    assert_eq!(
        poe_optimizer_pob::source::UPSTREAM_REVISION,
        catalog.data().source.upstream_revision
    );
    for (path, hash) in &catalog.data().source.files {
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(runtime::verified(path).unwrap().as_bytes())
            ),
            *hash,
            "{path}"
        );
    }
    let methods = oracle
        .lua
        .globals()
        .get::<Table>("item_loading_functions")
        .unwrap();
    for (name, first, last) in [
        ("constructor", 167, 183),
        ("parse_raw", 468, 1803),
        ("load", 1193, 1320),
    ] {
        let info = methods.get::<mlua::Function>(name).unwrap().info();
        assert_eq!(info.line_defined, Some(first), "{name}");
        assert_eq!(info.last_line_defined, Some(last), "{name}");
    }
}

fn replay_frozen(
    oracle: &runtime::Oracle,
    xml: &str,
    snapshot: &poe_optimizer_data::game_data::GameDataSnapshot,
) -> ItemState {
    let source = oracle.load(xml, false);
    assert!(
        source.get::<bool>("ok").unwrap(),
        "{:?}",
        source.get::<Option<String>>("error").unwrap()
    );
    let mut provider = native::FrozenAssembly {
        dependencies: native::OriginalDependencies::new(oracle),
        stages: events(&source, "build_mod_list")
            .into_iter()
            .skip(1)
            .collect(),
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
                if child.element().name() == "ModRange" {
                    machine
                        .apply_mod_range(
                            child.element().attribute("id").map(|a| a.decoded()),
                            child.element().attribute("range").map(|a| a.decoded()),
                        )
                        .unwrap();
                }
            }
        }
    }
    machine.finish_load(&mut provider).unwrap();
    assert_eq!(
        machine.status(),
        ItemLoadStatus::Complete,
        "{:?}",
        machine.pending()
    );
    assert!(provider.stages.is_empty());
    let expected = source
        .get::<Table>("items")
        .unwrap()
        .get::<Table>(1)
        .unwrap();
    native::compare_state(machine.state(), &expected);
    machine.into_state()
}
#[test]
fn repeated_text_and_interleaved_ranges_match_with_explicit_frozen_assembly_results() {
    let oracle = runtime::Oracle::new();
    let snapshot = bundled_snapshot().unwrap();
    for newline in ["\n", "\r\n"] {
        let first="Rarity: Rare\nEarlier title\nRusted Greathelm\nItem Level: 60\nQuality: 17\nCorrupted\nImplicits: 0\n+10 to maximum Life".replace('\n',newline);
        let second = "Rarity: Normal\nRusted Greathelm\nImplicits: 0\n+20 to maximum Life"
            .replace('\n', newline);
        let xml = format!(
            "<Items><Item id=\"9\" variantAlt=\"2\">{}<ModRange id=\"1\" range=\"0.125\"/><!-- boundary --> <![CDATA[{second}]]><ModRange id=\"1\" range=\"0.875\"/><ModRange id=\"99\" range=\"0.25\"/></Item></Items>",
            escaped_xml(&first)
        );
        let state = replay_frozen(&oracle, &xml, &snapshot);
        assert_eq!(state.explicit_mod_lines[0].range, ItemNumber::Finite(0.875));
        assert_eq!(
            state.retained_fields.get("corrupted"),
            Some(&ItemScalar::Boolean(true))
        );
        assert_eq!(
            state.retained_fields.get("itemLevel"),
            Some(&ItemScalar::Number(ItemNumber::Finite(60.0)))
        );
    }
}
#[test]
fn original_and_native_variant_version_group_selection_and_duplicate_counts_agree() {
    let oracle = runtime::Oracle::new();
    let snapshot = bundled_snapshot().unwrap();
    for headers in [
        "Variant: First\nVariant: Second\nSelected Variant: 0",
        "Variant: First\nVariant: Second\nSelected Variant: -2",
        "Variant: First\nVariant: Second\nSelected Variant: 99",
        "Variant: First\nVariant: Second\nSelected Variant: 1\nHas Alt Variant: true\nSelected Alt Variant: 1\nAllow Duplicate Variants: true",
        "Version: Old\nVersion: New\nVariant: First\nVariant: Second\nSelected Version: 0\nSelected Variant: 99",
        "Version: Old\nVersion: New\nVariant: First\nVariant: Second\nSelected Version: 2\nSelected Variant Group: 1=2\nSelected Variant Group: 2=2",
    ] {
        let modifiers = if headers.contains("Selected Variant Group") {
            "{version:1}{variant:1}{group:1}+10 to maximum Life\n{version:2}{variant:2}{group:1,2}+20 to maximum Life\n{variant:1}+5 to Strength"
        } else if headers.contains("Version:") {
            "{version:1}{variant:1}+10 to maximum Life\n{version:2}{variant:2}+20 to maximum Life\n{variant:1}+5 to Strength"
        } else {
            "{variant:1}+10 to maximum Life\n{variant:2}+20 to maximum Life\n{variant:1}+5 to Strength"
        };
        let raw = format!(
            "Rarity: Unique\nTest title\nRusted Greathelm\n{headers}\nImplicits: 0\n{modifiers}"
        );
        let xml = format!("<Items><Item id=\"5\">{}</Item></Items>", escaped_xml(&raw));
        let source = oracle.load(&xml, false);
        assert!(
            source.get::<bool>("ok").unwrap(),
            "{headers}: {:?}",
            source.get::<Option<String>>("error").unwrap()
        );
        let actual = replay_frozen(&oracle, &xml, &snapshot);
        let item = source
            .get::<Table>("items")
            .unwrap()
            .get::<Table>(1)
            .unwrap();
        native::assert_number(
            actual.variants.selected.unwrap_or(ItemNumber::Nil),
            native::number(item.get("variant").unwrap()),
            "variant",
        );
        native::assert_number(
            actual.variants.selected_version.unwrap_or(ItemNumber::Nil),
            native::number(item.get("selectedVersion").unwrap()),
            "selectedVersion",
        );
        // Compare actual original methods on the live source item, not a second
        // Rust implementation of selection semantics.
        let live = oracle.parse(&raw);
        for (index, line) in actual.explicit_mod_lines.iter().enumerate() {
            let original_line = live
                .get::<Table>("explicitModLines")
                .unwrap()
                .get::<Table>(index + 1)
                .unwrap();
            let matched = live
                .get::<mlua::Function>("CheckModLineVariant")
                .unwrap()
                .call::<bool>((live.clone(), original_line.clone()))
                .unwrap();
            let count = live
                .get::<mlua::Function>("GetModLineVariantCount")
                .unwrap()
                .call::<usize>((live.clone(), original_line))
                .unwrap();
            assert_eq!(
                actual.variants.matches(&line.selection),
                matched,
                "{headers}"
            );
            assert_eq!(actual.variants.count(&line.selection), count, "{headers}");
        }
    }
}

#[test]
fn injected_bases_follow_original_exact_longest_and_explicit_ambiguity_boundaries() {
    use poe_optimizer_data::item_loading::*;
    let oracle = runtime::Oracle::new();
    let snapshot = bundled_snapshot().unwrap();
    let mut injected = snapshot.item_loading().data().clone();
    let prototype = injected
        .bases
        .iter()
        .find(|b| b.name == "Rusted Greathelm")
        .unwrap()
        .clone();
    let bases = oracle
        .lua
        .globals()
        .get::<Table>("data")
        .unwrap()
        .get::<Table>("itemBases")
        .unwrap();
    let original = bases.get::<Table>("Rusted Greathelm").unwrap();
    for name in ["Alpha", "Omega", "Alpha Extended"] {
        let mut base = prototype.clone();
        base.name = name.into();
        let mut req = base
            .fields
            .fields
            .get("req")
            .unwrap()
            .as_table()
            .unwrap()
            .clone();
        req.fields
            .insert("level".into(), ItemMetadataValue::Number(37.0));
        req.fields
            .insert("str".into(), ItemMetadataValue::Number(81.0));
        base.fields
            .fields
            .insert("req".into(), ItemMetadataValue::Table(req));
        injected.bases.push(base);
        let copied = oracle
            .lua
            .globals()
            .get::<mlua::Function>("copyTable")
            .unwrap()
            .call::<Table>(original.clone())
            .unwrap();
        let req = copied.get::<Table>("req").unwrap();
        req.set("level", 37).unwrap();
        req.set("str", 81).unwrap();
        bases.set(name, copied).unwrap();
    }
    let catalog = ItemLoadingCatalog::new(injected).unwrap();
    for name in [
        "Alpha",
        "Caller Alpha suffix",
        "Caller Alpha Extended suffix",
        "Omega",
    ] {
        let raw = format!("Rarity: Normal\n{name}\nQuality: 0");
        let source = oracle.load(
            &format!("<Items><Item id=\"7\">{}</Item></Items>", escaped_xml(&raw)),
            false,
        );
        assert!(source.get::<bool>("ok").unwrap());
        let mut provider = native::OriginalDependencies::new(&oracle);
        let mut machine = ItemLoadMachine::new(&catalog);
        machine.set_xml_attributes(&BTreeMap::from([("id".into(), "7".into())]));
        machine.apply_text(&raw, &mut provider).unwrap();
        assert_eq!(
            machine.pending().map(|p| p.kind),
            Some(DependencyKind::Assembly),
            "{:?}",
            machine.pending()
        );
        native::compare_state(
            machine.state(),
            &events(&source, "build_mod_list")
                .remove(1)
                .get::<Table>("before")
                .unwrap(),
        );
        assert_eq!(
            machine.state().requirements["str"],
            ItemNumber::Finite(81.0)
        );
        assert_eq!(
            machine.state().requirements["level"],
            ItemNumber::Finite(37.0)
        );
    }
    let raw = "Rarity: Normal\nAlpha Omega\nQuality: 0";
    let source = oracle.parse(raw);
    assert!(["Alpha", "Omega"].contains(&source.get::<String>("baseName").unwrap().as_str()));
    let mut machine = ItemLoadMachine::new(&catalog);
    machine
        .apply_text(raw, &mut native::OriginalDependencies::new(&oracle))
        .unwrap();
    assert_eq!(
        machine.pending().map(|p| p.kind),
        Some(DependencyKind::BaseLookupAmbiguity)
    );
    assert!(
        !machine.state().base_present,
        "native must not copy an arbitrary pairs tie winner"
    );
}
#[test]
fn ordinary_loading_remains_equal_after_proven_original_item_traces_warm() {
    let oracle = runtime::Oracle::new();
    let snapshot = bundled_snapshot().unwrap();
    let traces = oracle
        .lua
        .globals()
        .get::<mlua::Function>("item_loading_warm")
        .unwrap()
        .call::<usize>(())
        .unwrap();
    assert!(
        traces > 0,
        "no surviving trace started in original ParseRaw/BuildModList/ItemsTab.Load"
    );
    let xml = "<Items><Item id=\"8\">Rarity: Normal\nRusted Greathelm\nQuality: 0\n+10 to maximum Life<ModRange id=\"1\" range=\"0.25\"/></Item></Items>";
    let native = replay_frozen(&oracle, xml, &snapshot);
    assert_eq!(native.explicit_mod_lines[0].range, ItemNumber::Finite(0.25));
}

#[test]
fn missing_dependencies_stop_at_actual_source_format_boundary_and_catalysts_remain_explicit() {
    let oracle = runtime::Oracle::new();
    let snapshot = bundled_snapshot().unwrap();
    for header in ["", "Catalyst: Flesh", "Quality (Life Modifiers): 30%"] {
        let raw = format!(
            "Rarity: Rare\nCaller jewel\nAmber Amulet\nItem Level: 60\n{header}\nImplicits: 0\n{{tags:life}}+10 to maximum Life"
        );
        let source = oracle.load(
            &format!("<Items><Item id=\"4\">{}</Item></Items>", escaped_xml(&raw)),
            false,
        );
        assert!(
            source.get::<bool>("ok").unwrap(),
            "{:?}",
            source.get::<Option<String>>("error").unwrap()
        );
        let mut machine = ItemLoadMachine::new(snapshot.item_loading());
        machine.set_xml_attributes(&BTreeMap::from([("id".into(), "4".into())]));
        machine
            .apply_text(&raw, &mut UnavailableItemLoadProvider)
            .unwrap();
        assert_eq!(
            machine.pending().map(|p| p.kind),
            Some(DependencyKind::RangeFormatting)
        );
        let format = events(&source, "format").remove(0);
        native::compare_state(machine.state(), &format.get::<Table>("before").unwrap());
        let expected = if header.is_empty() {
            1.0
        } else if header.starts_with("Quality") {
            1.3
        } else {
            1.2
        };
        native::assert_number(
            native::number(format.get("scalar").unwrap()),
            ItemNumber::Finite(expected),
            "actual catalyst scalar",
        );
    }
    let raw = "Rarity: Rare\nCaller rune\nRusted Greathelm\nSockets: S\nImplicits: 0\n{rune}+10 to maximum Life";
    let source = oracle.load(
        &format!("<Items><Item id=\"4\">{}</Item></Items>", escaped_xml(raw)),
        false,
    );
    assert!(source.get::<bool>("ok").unwrap());
    let mut machine = ItemLoadMachine::new(snapshot.item_loading());
    machine.set_xml_attributes(&BTreeMap::from([("id".into(), "4".into())]));
    machine
        .apply_text(raw, &mut UnavailableItemLoadProvider)
        .unwrap();
    assert_eq!(
        machine.pending().map(|p| p.kind),
        Some(DependencyKind::RangeFormatting)
    );
    assert_eq!(machine.state().format_calls.len(), 1);
    assert!(machine.state().parser_calls.is_empty());
    let format = events(&source, "format").remove(0);
    native::compare_state(machine.state(), &format.get::<Table>("before").unwrap());
}

#[test]
fn original_modrange_numeric_edges_match_values_ignored_rows_and_error_classes() {
    let oracle = runtime::Oracle::new();
    let snapshot = bundled_snapshot().unwrap();
    let raw = "Rarity: Normal\nRusted Greathelm\nQuality: 0\n+10 to maximum Life";
    for (id, range) in [
        ("1", "nan"),
        ("1", "inf"),
        ("1", "-inf"),
        ("1", "0x1.8p-1"),
        ("1", "garbage"),
        ("2", "0"),
        ("inf", "0.5"),
        ("nan", "0.5"),
        ("0x1p0", "0.5"),
        ("0", "0.5"),
        ("-1", "0.5"),
        ("0.5", "0.5"),
        ("-inf", "0.5"),
        ("garbage", "0.5"),
    ] {
        let xml = format!(
            "<Items><Item id=\"6\">{raw}<ModRange id=\"{id}\" range=\"{range}\"/></Item></Items>"
        );
        let source = oracle.load(&xml, false);
        let mut provider = native::FrozenAssembly {
            dependencies: native::OriginalDependencies::new(&oracle),
            stages: events(&source, "build_mod_list")
                .into_iter()
                .skip(1)
                .collect(),
        };
        let mut machine = ItemLoadMachine::new(snapshot.item_loading());
        machine.set_xml_attributes(&BTreeMap::from([("id".into(), "6".into())]));
        machine.apply_text(raw, &mut provider).unwrap();
        let result = machine.apply_mod_range(Some(id), Some(range));
        if source.get::<bool>("ok").unwrap() {
            result.unwrap();
            machine.finish_load(&mut provider).unwrap();
            assert_eq!(machine.status(), ItemLoadStatus::Complete, "{id}/{range}");
            native::compare_state(
                machine.state(),
                &source
                    .get::<Table>("items")
                    .unwrap()
                    .get::<Table>(1)
                    .unwrap(),
            );
        } else {
            let error = source.get::<String>("error").unwrap();
            assert!(
                error.contains("src/Classes/ItemsTab.lua:1235:") && error.contains("nil"),
                "{id}: {error}"
            );
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("ModRange indexes an absent source modifier row")
            );
            assert_eq!(machine.status(), ItemLoadStatus::SourceError);
        }
    }
}
