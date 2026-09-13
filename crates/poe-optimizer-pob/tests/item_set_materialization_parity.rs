//! Source/native ordered set materialization; whole Load is a separate boundary.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_set_materialization.rs"]
mod original;
#[test]
fn derived_set_cases_preserve_item_bytes_and_unrelated_root_sections() {
    let xml = r#"<PathOfBuilding2><Build/><Items activeItemSet="1"><Item id="1">A&#10;B</Item><Item id="2"><![CDATA[raw <&>]]></Item><ItemSet id="1"/></Items><Tree activeSpec="1"/><Config/></PathOfBuilding2>"#;
    let cases = original::cases(xml, true);
    assert_eq!(cases.len(), 12);
    assert_eq!(cases[0].xml, xml);
    for case in &cases {
        let doc = roxmltree::Document::parse(&case.xml).unwrap();
        let items = doc
            .descendants()
            .filter(|n| n.has_tag_name("Item"))
            .map(|n| &case.xml[n.range()])
            .collect::<Vec<_>>();
        assert_eq!(
            items,
            vec![
                r#"<Item id="1">A&#10;B</Item>"#,
                r#"<Item id="2"><![CDATA[raw <&>]]></Item>"#
            ]
        );
        assert!(case.xml.starts_with("<PathOfBuilding2><Build/>"));
        assert!(
            case.xml
                .ends_with("<Tree activeSpec=\"1\"/><Config/></PathOfBuilding2>")
        );
        assert_eq!(
            doc.root_element()
                .children()
                .filter(|n| n.has_tag_name("Items"))
                .count(),
            case.loads
        );
    }
}
#[test]
fn derived_numeric_winners_and_error_cases_remain_ordered_source_inputs() {
    let xml = "<PathOfBuilding2><Items><Item id=\"1\">raw</Item><ItemSet id=\"1\"/></Items></PathOfBuilding2>";
    let cases = original::cases(xml, true);
    let mixed = cases
        .iter()
        .find(|c| c.label == "missing_and_duplicate_ids")
        .unwrap();
    let doc = roxmltree::Document::parse(&mixed.xml).unwrap();
    assert_eq!(
        doc.descendants()
            .filter(|n| n.has_tag_name("ItemSet"))
            .map(|n| n.attribute("id"))
            .collect::<Vec<_>>(),
        vec![
            Some("3"),
            None,
            Some("1.0"),
            Some("nonnumeric"),
            Some("2.5")
        ]
    );
    assert_eq!(cases.iter().filter(|c| c.source_error).count(), 6);
    assert!(
        cases
            .iter()
            .filter(|c| c.source_error)
            .all(|c| c.structural)
    );
}

#[test]
fn all_five_original_set_materialization_and_derived_histories() {
    original::run();
}

fn diagnostic_graph() -> poe_optimizer_engine::source_program::ProgramValueGraph {
    use poe_optimizer_engine::source_program::{
        ProgramTable, ProgramTableId as Id, ProgramValue as V, ProgramValueGraph,
    };
    let text = |s: &str| V::Bytes(s.as_bytes().to_vec());
    let fields = |pairs: Vec<(&str, V)>| ProgramTable {
        entries: pairs.into_iter().map(|(k, v)| (text(k), v)).collect(),
    };
    ProgramValueGraph {
        values: vec![V::Table(Id(1))],
        tables: vec![
            fields(vec![
                ("itemSets", V::Table(Id(2))),
                ("activeItemSet", V::Table(Id(3))),
                ("previousActiveItemSet", V::Table(Id(3))),
                ("itemSetOrderList", V::Table(Id(4))),
                ("activeItemSetId", V::Number(1.0)),
                ("slots", V::Table(Id(5))),
                ("runeSlots", V::Table(Id(8))),
                ("showStatDifferences", V::Boolean(true)),
                ("trade", V::Table(Id(13))),
            ]),
            ProgramTable {
                entries: vec![(V::Number(1.0), V::Table(Id(3)))],
            },
            fields(vec![("id", V::Number(1.0)), ("title", text("Default"))]),
            ProgramTable {
                entries: vec![
                    (V::Number(1.0), V::Number(1.0)),
                    (V::Number(2.0), V::Number(1.0)),
                ],
            },
            fields(vec![
                ("Parent", V::Table(Id(6))),
                ("Child", V::Table(Id(14))),
            ]),
            fields(vec![
                ("slotName", text("Parent")),
                ("selItemId", V::Number(0.0)),
                ("jewelSocketList", V::Table(Id(7))),
            ]),
            ProgramTable {
                entries: vec![(V::Number(1.0), V::Table(Id(14)))],
            },
            fields(vec![("Rune", V::Table(Id(9)))]),
            fields(vec![
                ("selIndex", V::Number(1.0)),
                ("list", V::Table(Id(10))),
            ]),
            ProgramTable {
                entries: vec![
                    (V::Number(1.0), V::Table(Id(11))),
                    (V::Number(2.0), V::Table(Id(12))),
                ],
            },
            fields(vec![("name", text("None"))]),
            fields(vec![("name", text("Other"))]),
            ProgramTable::default(),
            fields(vec![
                ("slotName", text("Child")),
                ("selItemId", V::Number(0.0)),
                ("jewelSocketList", V::Table(Id(15))),
            ]),
            ProgramTable::default(),
        ],
    }
}
#[test]
fn projection_retains_set_and_child_aliases_duplicate_order_and_nil_presence() {
    use poe_optimizer_engine::source_program::{ProgramTableId as Id, ProgramValue as V};
    let input = diagnostic_graph();
    let base = original::common(input.clone(), true, false, false);
    let mut detached = input.clone();
    detached.tables.push(detached.tables[2].clone());
    detached.tables[0]
        .entries
        .iter_mut()
        .find(|(k, _)| *k == V::Bytes(b"previousActiveItemSet".to_vec()))
        .unwrap()
        .1 = V::Table(Id(16));
    assert_ne!(base, original::common(detached, true, false, false));
    let mut child = input.clone();
    child.tables.push(child.tables[13].clone());
    child.tables[6].entries[0].1 = V::Table(Id(16));
    assert_ne!(base, original::common(child, true, false, false));
    let mut order = input.clone();
    order.tables[3].entries.pop();
    assert_ne!(base, original::common(order, true, false, false));
    let mut note = input;
    note.tables[5]
        .entries
        .push((V::Bytes(b"note".to_vec()), V::Bytes(Vec::new())));
    assert_ne!(base, original::common(note, true, false, false));
}
#[test]
fn rune_projection_compares_selected_name_and_retains_raw_graph_separately() {
    use poe_optimizer_engine::source_program::ProgramValue as V;
    let input = diagnostic_graph();
    let base = original::common(input.clone(), true, false, false);
    let mut reordered = input.clone();
    let old = reordered.tables[9].entries[0].1.clone();
    reordered.tables[9].entries[0].1 = reordered.tables[9].entries[1].1.clone();
    reordered.tables[9].entries[1].1 = old;
    reordered.tables[8].entries[0].1 = V::Number(2.0);
    assert_eq!(
        base,
        original::common(reordered.clone(), true, false, false)
    );
    assert_ne!(input, reordered);
    let mut changed = input;
    changed.tables[8].entries[0].1 = V::Number(2.0);
    assert_ne!(base, original::common(changed, true, false, false));
}
// mlua requires Arc for CallbackError causes, including thread-local Lua errors.
#[test]
#[allow(clippy::arc_with_non_send_sync)]
fn host_and_observer_errors_cannot_be_reported_as_original_source_errors() {
    use mlua::Error;
    assert!(original::lua_source_error(&Error::RuntimeError(
        "original failure".into()
    )));
    assert!(original::lua_source_error(&Error::CallbackError {
        traceback: "trace".into(),
        cause: std::sync::Arc::new(Error::RuntimeError("original failure".into()))
    }));
    assert!(!original::lua_source_error(&Error::RuntimeError(
        "item-set lifecycle: budget".into()
    )));
    assert!(!original::lua_source_error(&Error::MemoryError(
        "allocation".into()
    )));
    assert!(!original::lua_source_error(&Error::StackError));
    assert!(!original::lua_source_error(&Error::CallbackError {
        traceback: "trace".into(),
        cause: std::sync::Arc::new(Error::MemoryError("allocation".into()))
    }));
}
