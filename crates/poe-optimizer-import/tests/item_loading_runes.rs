use poe_optimizer_data::{
    game_data::{GameDataSnapshot, bundled_snapshot},
    item_loading::*,
};
use poe_optimizer_import::item_loading::*;
use std::{collections::BTreeMap, sync::OnceLock};
fn snapshot() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}
fn catalog(change: impl FnOnce(&mut ItemLoadingData)) -> ItemLoadingCatalog {
    let mut data = snapshot().item_loading().data().clone();
    change(&mut data);
    ItemLoadingCatalog::new(data).unwrap()
}
#[derive(Default)]
struct Complete;
impl ItemLoadProvider for Complete {
    fn format_line(&mut self, r: &FormatRequest) -> DependencyResult<String> {
        DependencyResult::Available(r.text.clone())
    }
    fn parse_modifier(&mut self, _: &ParseRequest) -> DependencyResult<ParseOutcome> {
        DependencyResult::Available(ParseOutcome {
            modifiers: Some(vec![]),
            extra: None,
        })
    }
    fn lookup_unique(&mut self, _: &UniqueRequest) -> DependencyResult<Option<UniqueOutcome>> {
        DependencyResult::Available(None)
    }
    fn assemble(&mut self, _: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
        DependencyResult::Available(AssemblyOutcome {
            assembled: None,
            armour_data: Default::default(),
            modifier_payloads: None,
            requirements: None,
            state_updates: BTreeMap::new(),
            evidence: ItemMetadataTable::default(),
        })
    }
}
#[test]
fn unused_malformed_rune_numeric_grammar_is_lazy() {
    let catalog = catalog(|d| d.policy.rune_loading.numeric_pattern = "(".into());
    let mut machine = ItemLoadMachine::new(&catalog);
    machine
        .apply_text(
            "Rarity: NORMAL\nAmber Amulet\nImplicits: 0\n+10 to maximum Life",
            &mut Complete,
        )
        .unwrap();
    assert_eq!(machine.status(), ItemLoadStatus::Complete);
    assert!(machine.state().rune_mod_lines.is_empty());
}
#[test]
fn reached_malformed_rune_numeric_grammar_is_source_error() {
    let catalog = catalog(|d| d.policy.rune_loading.numeric_pattern = "(".into());
    let mut machine = ItemLoadMachine::new(&catalog);
    let error = machine
        .apply_text(
            "Rarity: NORMAL\nCrude Bow\nSockets: S\nRune: None",
            &mut Complete,
        )
        .unwrap_err();
    assert_eq!(machine.status(), ItemLoadStatus::SourceError);
    assert!(error.to_string().contains("capture"), "{error}");
    assert!(machine.state().rune_mod_lines.is_empty());
}
#[test]
fn socket_work_bound_is_distinct_from_source_error() {
    let catalog = snapshot().item_loading();
    let mut machine = ItemLoadMachine::new(catalog);
    let raw = format!(
        "Rarity: NORMAL\nAmber Amulet\nSockets: {}",
        "S".repeat(MAX_ITEM_LOADING_LINES + 1)
    );
    let error = machine.apply_text(&raw, &mut Complete).unwrap_err();
    assert!(error.to_string().contains("socket count bound"));
    assert_ne!(machine.status(), ItemLoadStatus::SourceError);
    assert_eq!(machine.state().sockets.len(), MAX_ITEM_LOADING_LINES);
}
#[test]
fn byte_capture_inside_unicode_hint_is_pending_not_a_panic() {
    let catalog = catalog(|d| d.policy.rune_loading.augment_override_pattern = "(.)".into());
    let mut machine = ItemLoadMachine::new(&catalog);
    machine
        .apply_text(
            "Rarity: NORMAL\nAmber Amulet\nImplicits: 0\nécho",
            &mut Complete,
        )
        .unwrap();
    assert_eq!(machine.status(), ItemLoadStatus::Pending);
    assert!(machine.pending().unwrap().message.contains("UTF-8"));
}
#[test]
fn malformed_reserved_header_guard_is_not_a_reached_source_error() {
    let catalog = catalog(|d| d.policy.rune_loading.other_header_patterns = vec!["(".into()]);
    let mut machine = ItemLoadMachine::new(&catalog);
    machine
        .apply_text("Rarity: NORMAL\nAmber Amulet\nRune: None", &mut Complete)
        .unwrap();
    assert_eq!(machine.status(), ItemLoadStatus::Pending);
    assert!(
        machine
            .pending()
            .unwrap()
            .message
            .contains("reserved rune header")
    );
    assert!(machine.state().runes.is_empty());
}
#[test]
fn injected_effect_default_cannot_silently_use_unscaled_groups() {
    let catalog = catalog(|d| d.policy.rune_loading.effect_default = 0.25);
    let mut machine = ItemLoadMachine::new(&catalog);
    machine
        .apply_text(
            "Rarity: NORMAL\nCrude Bow\nSockets: S\nRune: None",
            &mut Complete,
        )
        .unwrap();
    assert_eq!(machine.status(), ItemLoadStatus::Pending);
    assert!(machine.pending().unwrap().message.contains("range scaling"));
}

fn slot(lines: impl IntoIterator<Item = String>) -> ItemMetadataValue {
    ItemMetadataValue::Table(ItemMetadataTable {
        fields: [("type".into(), ItemMetadataValue::Text("Rune".into()))].into(),
        indexed: lines
            .into_iter()
            .enumerate()
            .map(|(i, line)| ((i + 1) as i64, ItemMetadataValue::Text(line)))
            .collect(),
    })
}
fn rune(value: ItemMetadataValue) -> ItemMetadataValue {
    ItemMetadataValue::Table(ItemMetadataTable {
        fields: [("weapon".into(), value)].into(),
        indexed: BTreeMap::new(),
    })
}
#[test]
fn aggregate_grouping_storage_is_bounded_before_search() {
    let catalog = catalog(|d| {
        d.modifier_tables
            .get_mut(&d.policy.rune_loading.rune_table)
            .unwrap()
            .fields = [(
            "Custom".into(),
            rune(slot(std::iter::repeat_n(
                "x".to_owned(),
                MAX_ITEM_LOADING_CALLS + 1,
            ))),
        )]
        .into();
    });
    let mut machine = ItemLoadMachine::new(&catalog);
    let error = machine
        .apply_text("Rarity: NORMAL\nCrude Bow", &mut Complete)
        .unwrap_err();
    assert!(
        error.to_string().contains("grouping row count bound"),
        "{error}"
    );
    assert_ne!(machine.status(), ItemLoadStatus::SourceError);
}
#[test]
fn sorting_work_exhaustion_returns_an_error_without_panicking() {
    let catalog = catalog(|d| {
        d.modifier_tables
            .get_mut(&d.policy.rune_loading.rune_table)
            .unwrap()
            .fields = (1..=2500)
            .map(|i| (format!("Custom{i:04}"), rune(slot([format!("stat {i}")]))))
            .collect();
    });
    let mut machine = ItemLoadMachine::new(&catalog);
    let error = machine
        .apply_text("Rarity: NORMAL\nCrude Bow", &mut Complete)
        .unwrap_err();
    assert!(error.to_string().contains("MatchSteps"), "{error}");
    assert_ne!(machine.status(), ItemLoadStatus::SourceError);
}

#[test]
fn unselected_string_slot_is_inert_under_lua_named_indexing() {
    let catalog = catalog(|d| {
        d.modifier_tables
            .get_mut(&d.policy.rune_loading.rune_table)
            .unwrap()
            .fields
            .insert(
                "Caller".into(),
                ItemMetadataValue::Table(ItemMetadataTable {
                    fields: [(
                        "unselected".into(),
                        ItemMetadataValue::Text("metadata".into()),
                    )]
                    .into(),
                    indexed: BTreeMap::new(),
                }),
            );
    });
    let mut machine = ItemLoadMachine::new(&catalog);
    machine
        .apply_text("Rarity: NORMAL\nCrude Bow", &mut Complete)
        .unwrap();
    assert_eq!(machine.status(), ItemLoadStatus::Complete);
}
#[test]
fn unrepresented_base_tag_indexing_does_not_invent_a_source_error() {
    let catalog = catalog(|d| {
        d.bases
            .iter_mut()
            .find(|b| b.name == "Amber Amulet")
            .unwrap()
            .fields
            .fields
            .insert("tags".into(), ItemMetadataValue::Text("caller".into()));
    });
    let mut machine = ItemLoadMachine::new(&catalog);
    machine
        .apply_text("Rarity: NORMAL\nAmber Amulet", &mut Complete)
        .unwrap();
    assert_eq!(machine.status(), ItemLoadStatus::Pending);
    assert!(
        machine
            .pending()
            .unwrap()
            .message
            .contains("base-tag indexing")
    );
}

#[test]
fn configured_copy_mode_selects_the_ranged_effect_dependency() {
    struct Effects {
        name: String,
        kind: String,
    }
    impl ItemLoadProvider for Effects {
        fn format_line(&mut self, r: &FormatRequest) -> DependencyResult<String> {
            DependencyResult::Available(r.text.clone())
        }
        fn parse_modifier(&mut self, _: &ParseRequest) -> DependencyResult<ParseOutcome> {
            DependencyResult::Available(ParseOutcome {
                modifiers: Some(vec![ItemMetadataTable {
                    fields: [
                        ("name".into(), ItemMetadataValue::Text(self.name.clone())),
                        ("type".into(), ItemMetadataValue::Text(self.kind.clone())),
                        ("value".into(), ItemMetadataValue::Number(10.0)),
                    ]
                    .into(),
                    indexed: BTreeMap::new(),
                }]),
                extra: None,
            })
        }
    }
    let catalog = catalog(|d| d.policy.rune_loading.game_mode = "WIKI".into());
    let mut provider = Effects {
        name: catalog.policy().rune_loading.effect_global_name.clone(),
        kind: catalog.policy().rune_loading.effect_mod_type.clone(),
    };
    let mut machine = ItemLoadMachine::new(&catalog);
    machine
        .apply_text(
            "Caller name\nCrude Bow\nImplicits: 0\nRune effect",
            &mut provider,
        )
        .unwrap();
    assert_eq!(machine.status(), ItemLoadStatus::Pending);
    assert_eq!(
        machine.pending().unwrap().kind,
        DependencyKind::RuneReconstruction
    );
    assert!(
        machine
            .pending()
            .unwrap()
            .message
            .contains("ranged augment-effect")
    );
}

fn ordered_value_catalog(names: [&str; 4]) -> ItemLoadingCatalog {
    catalog(|d| {
        let definitions = names
            .into_iter()
            .zip([20, 18, 16, 14])
            .enumerate()
            .map(|(rank, (name, value))| {
                let mut record = match slot([format!("Caller stat {value}")]) {
                    ItemMetadataValue::Table(table) => table,
                    _ => unreachable!(),
                };
                record.fields.insert(
                    "levelReq".into(),
                    ItemMetadataValue::Number(((rank + 1) * 10) as f64),
                );
                record.fields.insert(
                    "bonded".into(),
                    ItemMetadataValue::Table(ItemMetadataTable {
                        fields: BTreeMap::new(),
                        indexed: [(1, ItemMetadataValue::Text("Caller bond 20".into()))].into(),
                    }),
                );
                (name.to_owned(), rune(ItemMetadataValue::Table(record)))
            })
            .collect();
        d.modifier_tables
            .get_mut(&d.policy.rune_loading.rune_table)
            .unwrap()
            .fields = definitions;
    })
}

#[test]
fn strict_group_order_admits_ambiguous_counts_without_rewriting_saved_headers() {
    for names in [
        ["A", "B", "C", "D"],
        ["Z", "Y", "X", "W"],
        ["R", "A", "Z", "B"],
    ] {
        let catalog = ordered_value_catalog(names);
        let mut machine = ItemLoadMachine::new(&catalog);
        let raw = format!(
            "Rarity: NORMAL\nCrude Bow\nSockets: S S\nRune: {}\nRune: {}",
            names[1], names[1]
        );
        machine.apply_text(&raw, &mut Complete).unwrap();
        assert_eq!(
            machine.status(),
            ItemLoadStatus::Complete,
            "{:?}",
            machine.pending()
        );
        assert_eq!(
            machine.state().runes,
            [names[1], names[1]],
            "inferred 20+16 must not replace saved 18+18"
        );
        let normal = machine
            .state()
            .rune_mod_lines
            .iter()
            .find(|r| !r.flags.contains("bonded"))
            .unwrap();
        assert_eq!(normal.line, "Caller stat 36");
        assert_eq!(normal.rune_count, Some(ItemNumber::Finite(2.0)));
        assert_eq!(normal.augment_type.as_deref(), Some("Rune"));
        assert!(
            machine
                .state()
                .rune_mod_lines
                .iter()
                .any(|r| r.flags.contains("bonded")
                    && r.line == "Bonded: Caller bond 40"
                    && r.augment_type.as_deref() == Some("Rune"))
        );
    }
}

#[test]
fn header_free_inference_uses_strict_vector_order_not_catalog_name_order() {
    for names in [["A", "B", "C", "D"], ["Z", "Y", "X", "W"]] {
        let catalog = ordered_value_catalog(names);
        let mut machine = ItemLoadMachine::new(&catalog);
        machine.apply_text("Rarity: NORMAL\nCrude Bow\nSockets: S S\n{rune}Caller stat 36\n{rune}Bonded: Caller bond 40", &mut Complete).unwrap();
        assert_eq!(
            machine.status(),
            ItemLoadStatus::Complete,
            "{:?}",
            machine.pending()
        );
        assert_eq!(
            machine.state().runes,
            [names[0], names[2]],
            "source first minimum is 20+16"
        );
        assert!(
            machine
                .state()
                .rune_mod_lines
                .iter()
                .any(|r| r.line == "Caller stat 36")
        );
        assert!(
            machine
                .state()
                .rune_mod_lines
                .iter()
                .any(|r| r.line == "Bonded: Caller bond 40")
        );
    }
}

#[test]
fn exact_tied_vectors_still_stop_before_annotation_despite_distinct_names() {
    let catalog = catalog(|d| {
        let definitions = ["Z", "A"]
            .into_iter()
            .map(|name| {
                let mut record = match slot(["Caller stat 10".into()]) {
                    ItemMetadataValue::Table(t) => t,
                    _ => unreachable!(),
                };
                record
                    .fields
                    .insert("levelReq".into(), ItemMetadataValue::Number(5.0));
                (name.to_owned(), rune(ItemMetadataValue::Table(record)))
            })
            .collect();
        d.modifier_tables
            .get_mut(&d.policy.rune_loading.rune_table)
            .unwrap()
            .fields = definitions;
    });
    let mut machine = ItemLoadMachine::new(&catalog);
    machine
        .apply_text(
            "Rarity: NORMAL\nCrude Bow\nSockets: S\nRune: Z",
            &mut Complete,
        )
        .unwrap();
    assert_eq!(machine.status(), ItemLoadStatus::Pending);
    assert_eq!(
        machine.pending().unwrap().kind,
        DependencyKind::RuneReconstruction
    );
    assert!(
        machine
            .pending()
            .unwrap()
            .message
            .contains("multiple minimum count vectors")
    );
    assert_eq!(machine.state().runes, ["Z"]);
    assert_eq!(machine.state().rune_mod_lines[0].rune_count, None);
}

#[test]
fn grouping_checks_exact_sum_before_floating_point_rounding_can_hide_overflow() {
    for (values, accepted) in [
        ([9_007_199_254_740_992u64, 1, 1], false),
        ([1, 1, 9_007_199_254_740_992], false),
        ([9_007_199_254_740_990, 1, 1], true),
    ] {
        let catalog = catalog(|d| {
            d.modifier_tables
                .get_mut(&d.policy.rune_loading.rune_table)
                .unwrap()
                .fields = [(
                "Caller".into(),
                rune(slot(values.map(|v| format!("Caller stat {v}")))),
            )]
            .into();
        });
        let mut machine = ItemLoadMachine::new(&catalog);
        machine
            .apply_text("Rarity: NORMAL\nCrude Bow", &mut Complete)
            .unwrap();
        if accepted {
            assert_eq!(
                machine.status(),
                ItemLoadStatus::Complete,
                "{:?}",
                machine.pending()
            );
        } else {
            assert_eq!(machine.status(), ItemLoadStatus::Pending);
            assert_eq!(
                machine.pending().unwrap().kind,
                DependencyKind::RuneReconstruction
            );
            assert!(
                machine
                    .pending()
                    .unwrap()
                    .message
                    .contains("sum exceeds exact integer range")
            );
        }
        assert!(machine.state().runes.is_empty());
        assert!(machine.state().rune_mod_lines.is_empty());
    }
}
