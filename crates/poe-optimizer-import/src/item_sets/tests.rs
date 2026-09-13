use super::*;
use crate::item_loading::assembly::AssemblyErrorKind;
use poe_optimizer_data::item_assembly::*;
fn policy() -> ItemInventoryPolicy {
    ItemInventoryPolicy {
        layout: ItemInventoryLayout {
            base_slots: vec!["Weapon 1".into(), "Charm 1".into()],
            swap: ItemInventorySwap {
                slot_pattern: "Weapon".into(),
                suffix: " Swap".into(),
                primary_weapon_set: 1,
                alternate_weapon_set: 2,
            },
            embedded: ItemInventoryEmbedded {
                parent_slots: vec!["Weapon 1".into()],
                count: 2,
                name_infix: " Jewel ".into(),
                label_prefix: "Child ".into(),
            },
            passive: ItemInventoryPassive {
                node_type: "Socket".into(),
                contained_socket_field: "containJewelSocket".into(),
                slot_prefix: "Node ".into(),
                label: "Socket".into(),
                nodes: ItemInventoryPassiveNodes {
                    tree_version: "caller-tree".into(),
                    full_snapshot_sha256: "0".repeat(64),
                    ids: vec![7, 11],
                },
            },
            rune_slots: vec![
                ItemInventoryRuneSlot {
                    name: "Rune A".into(),
                    slot_type: "caller".into(),
                    label: "A".into(),
                },
                ItemInventoryRuneSlot {
                    name: "Rune B".into(),
                    slot_type: "caller".into(),
                    label: "B".into(),
                },
            ],
            slot_number_patterns: ["%d+$".into(), "%d+".into()],
            activation_patterns: ["Charm".into(), "Flask".into()],
        },
        defaults: ItemInventoryDefaults {
            first_set_id: 1.0,
            set_id_increment: 1.0,
            reset_active_set_id: 0.0,
            empty_item_id: 0.0,
            default_set_title: "Default".into(),
            empty_rune_name: "None".into(),
            empty_slot_name: "".into(),
            empty_url: "".into(),
            true_token: "true".into(),
            empty_item_label: "None".into(),
            show_stat_differences: true,
        },
        power_stats: ItemInventoryPowerStats {
            rows: vec![
                ItemInventoryPowerStat {
                    stat: None,
                    label: None,
                    transform: None,
                },
                ItemInventoryPowerStat {
                    stat: Some("caller".into()),
                    label: Some("first".into()),
                    transform: Some(0),
                },
                ItemInventoryPowerStat {
                    stat: Some("caller".into()),
                    label: Some("second".into()),
                    transform: Some(1),
                },
                ItemInventoryPowerStat {
                    stat: Some("shared".into()),
                    label: Some("shared".into()),
                    transform: Some(0),
                },
                ItemInventoryPowerStat {
                    stat: Some("distinct".into()),
                    label: None,
                    transform: Some(1),
                },
            ],
            transforms: vec![
                ItemInventoryPowerTransform::Negate,
                ItemInventoryPowerTransform::Negate,
            ],
        },
    }
}
fn fresh() -> ItemSetState {
    ItemSetState::new(&policy(), ItemSetLimits::default()).unwrap()
}
fn root(g: &ProgramValueGraph) -> Id {
    id(&g.values[0])
}
fn id(v: &V) -> Id {
    if let V::Table(id) = v {
        *id
    } else {
        panic!("expected table, got {v:?}")
    }
}
fn field<'a>(g: &'a ProgramValueGraph, t: Id, key: &str) -> &'a V {
    g.tables[t.0 as usize - 1]
        .entries
        .iter()
        .find_map(|(k, v)| matches!(k,V::Bytes(b) if b==key.as_bytes()).then_some(v))
        .unwrap_or(&V::Nil)
}
fn at(g: &ProgramValueGraph, t: Id, key: f64) -> &V {
    g.tables[t.0 as usize - 1]
        .entries
        .iter()
        .find_map(|(k, v)| matches!(k,V::Number(n) if *n==key).then_some(v))
        .unwrap_or(&V::Nil)
}
fn text(v: &V) -> &str {
    if let V::Bytes(b) = v {
        std::str::from_utf8(b).unwrap()
    } else {
        panic!("{v:?}")
    }
}
fn num(v: &V) -> f64 {
    if let V::Number(n) = v {
        *n
    } else {
        panic!("{v:?}")
    }
}
fn current(g: &ProgramValueGraph, key: f64) -> Id {
    id(at(g, id(field(g, root(g), "itemSets")), key))
}
fn open(s: &mut ItemSetState, key: &str) {
    s.begin_item_set(ItemSetInput {
        id: Some(key),
        ..Default::default()
    })
    .unwrap();
}

#[test]
fn fresh_definitions_create_aliases_and_load_reset_retains_detached_active_root() {
    let mut s = fresh();
    let g = s.snapshot().unwrap();
    let r = root(&g);
    let active = id(field(&g, r, "activeItemSet"));
    assert_eq!(active, current(&g, 1.0));
    assert_eq!(active, id(field(&g, r, "previousActiveItemSet")));
    let slots = id(field(&g, r, "slots"));
    assert_eq!(g.tables[slots.0 as usize - 1].entries.len(), 9);
    let parent = id(field(&g, slots, "Weapon 1"));
    let child = id(field(&g, slots, "Weapon 1 Jewel 1"));
    assert_eq!(id(field(&g, child, "parentSlot")), parent);
    assert_eq!(
        id(at(&g, id(field(&g, parent, "jewelSocketList")), 1.0)),
        child
    );
    assert_eq!(field(&g, child, "inactive"), &V::Boolean(true));
    assert_eq!(field(&g, parent, "active"), &V::Nil);
    assert_eq!(field(&g, parent, "note"), &V::Nil);
    assert_eq!(
        field(&g, active, "Node 7"),
        &V::Nil,
        "passive slots are not ItemSet rows"
    );
    s.begin_load().unwrap();
    let g = s.snapshot().unwrap();
    let r = root(&g);
    assert_eq!(num(field(&g, r, "activeItemSetId")), 0.0);
    assert!(
        g.tables[id(field(&g, r, "itemSets")).0 as usize - 1]
            .entries
            .is_empty()
    );
    assert_eq!(
        id(field(&g, r, "activeItemSet")),
        id(field(&g, r, "previousActiveItemSet"))
    );
    assert_eq!(num(field(&g, id(field(&g, r, "activeItemSet")), "id")), 1.0);
}

#[test]
fn duplicate_zero_fractional_and_infinite_keys_preserve_order_and_scalar_bits() {
    let mut s = fresh();
    s.begin_load().unwrap();
    for key in ["0", "-0", "0.5", "inf"] {
        open(&mut s, key);
        s.finish_item_set().unwrap();
    }
    let g = s.snapshot().unwrap();
    let r = root(&g);
    let sets = id(field(&g, r, "itemSets"));
    let order = id(field(&g, r, "itemSetOrderList"));
    assert_eq!(g.tables[sets.0 as usize - 1].entries.len(), 3);
    assert!(num(field(&g, current(&g, 0.0), "id")).is_sign_negative());
    assert_eq!(num(at(&g, order, 1.0)).to_bits(), 0.0f64.to_bits());
    assert_eq!(num(at(&g, order, 2.0)).to_bits(), (-0.0f64).to_bits());
    assert_eq!(num(at(&g, order, 3.0)), 0.5);
    assert_eq!(num(at(&g, order, 4.0)), f64::INFINITY);
    assert!(
        g.tables[sets.0 as usize - 1]
            .entries
            .iter()
            .any(|(k, _)| matches!(k,V::Number(n) if n.to_bits()==0.0f64.to_bits()))
    );
    assert_ne!(
        current(&g, 0.0),
        id(field(&g, r, "activeItemSet")),
        "lookup replacement never substitutes the detached prior root"
    );
}

#[test]
fn mixed_rows_and_nil_deletions_follow_raw_field_truthiness() {
    let mut s = fresh();
    s.begin_load().unwrap();
    open(&mut s, "2");
    s.set_slot(SetSlotInput {
        name: Some("Rune A"),
        item_id: Some("3"),
        active: Some("true"),
        item_pb_url: Some("a"),
        note: Some("first"),
    })
    .unwrap();
    s.set_slot(SetSlotInput {
        name: Some("Rune A"),
        note: None,
        ..Default::default()
    })
    .unwrap();
    s.set_rune(SetRuneInput {
        slot_name: Some("Weapon 1"),
        rune_name: Some("caller rune"),
    })
    .unwrap();
    s.set_slot(SetSlotInput {
        name: Some("useSecondWeaponSet"),
        item_id: Some("8"),
        ..Default::default()
    })
    .unwrap();
    s.socket_url(SocketUrlInput {
        node_id: Some("7"),
        item_pb_url: Some("first"),
    })
    .unwrap();
    s.socket_url(SocketUrlInput {
        node_id: Some("7"),
        item_pb_url: Some("second"),
    })
    .unwrap();
    s.finish_item_set().unwrap();
    let g = s.snapshot().unwrap();
    let set = current(&g, 2.0);
    let rune = id(field(&g, set, "Rune A"));
    assert_eq!(field(&g, rune, "selItemId"), &V::Nil);
    assert_eq!(field(&g, rune, "note"), &V::Nil);
    assert_eq!(field(&g, rune, "active"), &V::Boolean(false));
    assert_eq!(text(field(&g, rune, "runeName")), "None");
    assert_eq!(
        text(field(&g, id(field(&g, set, "Weapon 1")), "runeName")),
        "caller rune"
    );
    assert_eq!(text(field(&g, id(at(&g, set, 7.0)), "pbURL")), "second");
}

#[test]
fn scalar_target_errors_preserve_reached_rows_before_order_append() {
    for (target, second) in [
        ("id", false),
        ("title", false),
        ("useSecondWeaponSet", true),
    ] {
        let mut s = fresh();
        s.begin_load().unwrap();
        s.begin_item_set(ItemSetInput {
            id: Some("2"),
            use_second_weapon_set: second.then_some("true"),
            ..Default::default()
        })
        .unwrap();
        s.set_slot(SetSlotInput {
            name: Some("Weapon 1"),
            item_id: Some("9"),
            ..Default::default()
        })
        .unwrap();
        let error = s
            .set_rune(SetRuneInput {
                slot_name: Some(target),
                rune_name: Some("bad"),
            })
            .unwrap_err();
        assert_eq!(error.kind, AssemblyErrorKind::Source);
        assert_eq!(s.phase(), ItemSetPhase::Failed);
        let g = s.snapshot().unwrap();
        let set = current(&g, 2.0);
        assert_eq!(
            num(field(&g, id(field(&g, set, "Weapon 1")), "selItemId")),
            9.0
        );
        assert!(
            g.tables[id(field(&g, root(&g), "itemSetOrderList")).0 as usize - 1]
                .entries
                .is_empty()
        );
        assert!(s.finish_item_set().is_err());
    }
}

#[test]
fn invalid_socket_keys_stop_before_append_and_failed_load_can_explicitly_restart() {
    for invalid in [None, Some("nan"), Some("not-a-number")] {
        let mut s = fresh();
        s.begin_load().unwrap();
        open(&mut s, "2");
        s.legacy_slot(LegacySlotInput::default()).unwrap_err(); // wrong phase does not execute source work
        let error = s
            .socket_url(SocketUrlInput {
                node_id: invalid,
                item_pb_url: Some("retained before invalid key"),
            })
            .unwrap_err();
        assert_eq!(error.kind, AssemblyErrorKind::Source);
        let g = s.snapshot().unwrap();
        assert!(
            g.tables[id(field(&g, root(&g), "itemSetOrderList")).0 as usize - 1]
                .entries
                .is_empty()
        );
        assert_eq!(num(field(&g, current(&g, 2.0), "id")), 2.0);
        s.begin_load().unwrap();
        assert_eq!(s.phase(), ItemSetPhase::Loading);
        let g = s.snapshot().unwrap();
        assert!(
            g.tables[id(field(&g, root(&g), "itemSets")).0 as usize - 1]
                .entries
                .is_empty()
        );
    }
}

#[test]
fn legacy_default_fallback_stops_at_activation_entry_without_clearing_live_fields() {
    let mut s = fresh();
    s.begin_load().unwrap();
    s.legacy_slot(LegacySlotInput {
        name: Some("Weapon 1"),
        item_id: Some("23"),
        active: Some("true"),
    })
    .unwrap();
    s.legacy_slot(LegacySlotInput {
        name: Some("Charm 1"),
        item_id: Some("24"),
        active: Some("true"),
    })
    .unwrap();
    s.finish_load(FinishLoadInput {
        active_item_set: Some("999"),
        use_second_weapon_set: Some("true"),
        show_stat_differences: Some("false"),
    })
    .unwrap();
    assert_eq!(s.phase(), ItemSetPhase::AwaitingActivation);
    assert_eq!(s.pending_activation(), Some(ItemNumber::Finite(999.0)));
    let g = s.snapshot().unwrap();
    let r = root(&g);
    let active = id(field(&g, r, "activeItemSet"));
    assert_eq!(active, current(&g, 1.0));
    assert_eq!(active, id(field(&g, r, "previousActiveItemSet")));
    assert_eq!(
        num(field(&g, r, "activeItemSetId")),
        0.0,
        "selection publication is still pending"
    );
    assert_eq!(
        field(&g, r, "showStatDifferences"),
        &V::Boolean(true),
        "trailing update is still pending"
    );
    assert_eq!(s.continuation().unwrap().show_stat_differences, Some(false));
    assert_eq!(field(&g, active, "useSecondWeaponSet"), &V::Boolean(true));
    let slots = id(field(&g, r, "slots"));
    let weapon = id(field(&g, slots, "Weapon 1"));
    let charm = id(field(&g, slots, "Charm 1"));
    assert_eq!(num(field(&g, weapon, "selItemId")), 23.0);
    assert_eq!(field(&g, weapon, "active"), &V::Nil);
    assert_eq!(field(&g, charm, "active"), &V::Boolean(true));
    assert_eq!(
        field(&g, id(field(&g, charm, "activate")), "state"),
        &V::Boolean(true)
    );
    assert_eq!(
        num(field(&g, id(field(&g, active, "Weapon 1")), "selItemId")),
        0.0,
        "prev/current copying has not been invented"
    );
    assert!(s.begin_load().is_err());
    assert!(s.begin_item_set(ItemSetInput::default()).is_err());
}

#[test]
fn ordered_trade_matches_preserve_nil_rows_and_catalog_transform_identity() {
    let mut s = fresh();
    s.begin_load().unwrap();
    for (stat, label) in [
        (None, Some("removed")),
        (Some("caller"), Some("stale")),
        (Some("shared"), None),
        (Some("distinct"), None),
        (Some("absent"), None),
    ] {
        s.trade_weight(TradeWeightInput {
            stat,
            label,
            weight_mult: Some("inf"),
        })
        .unwrap();
    }
    let g = s.snapshot().unwrap();
    let trade = id(field(&g, root(&g), "trade"));
    assert_eq!(g.tables[trade.0 as usize - 1].entries.len(), 4);
    assert_eq!(field(&g, id(at(&g, trade, 1.0)), "label"), &V::Nil);
    assert_eq!(text(field(&g, id(at(&g, trade, 2.0)), "label")), "first");
    assert_eq!(
        num(field(&g, id(at(&g, trade, 2.0)), "weightMult")),
        f64::INFINITY
    );
    assert!(s.trade_transform(0).is_none());
    let first = s.trade_transform(1).unwrap();
    assert!(first.same_identity(&s.trade_transform(2).unwrap()));
    assert!(!first.same_identity(&s.trade_transform(3).unwrap()));
    let mut other = fresh();
    other.begin_load().unwrap();
    other
        .trade_weight(TradeWeightInput {
            stat: Some("caller"),
            ..Default::default()
        })
        .unwrap();
    assert!(!first.same_identity(&other.trade_transform(0).unwrap()));
}

#[test]
fn changed_defaults_generation_and_lazy_patterns_are_consumed() {
    let mut p = policy();
    p.defaults.first_set_id = 5.0;
    p.defaults.set_id_increment = 2.0;
    p.defaults.reset_active_set_id = -1.0;
    p.defaults.true_token = "yes".into();
    p.layout.activation_patterns = [".".into(), "%".into()]; // second source syntax trap is never reached
    let mut s = ItemSetState::new(&p, ItemSetLimits::default()).unwrap();
    s.begin_load().unwrap();
    for _ in 0..3 {
        s.begin_item_set(ItemSetInput {
            use_second_weapon_set: Some("yes"),
            ..Default::default()
        })
        .unwrap();
        s.finish_item_set().unwrap();
    }
    let g = s.snapshot().unwrap();
    let order = id(field(&g, root(&g), "itemSetOrderList"));
    assert_eq!(
        (
            num(at(&g, order, 1.0)),
            num(at(&g, order, 2.0)),
            num(at(&g, order, 3.0))
        ),
        (5.0, 7.0, 9.0)
    );
    assert_eq!(num(field(&g, root(&g), "activeItemSetId")), -1.0);
    assert_eq!(
        field(&g, current(&g, 9.0), "useSecondWeaponSet"),
        &V::Boolean(true)
    );
}

#[test]
fn resource_failure_keeps_prefix_and_shared_byte_ceiling_never_refills() {
    let mut s = fresh();
    s.begin_load().unwrap();
    open(&mut s, "2");
    let error = s
        .set_slot(SetSlotInput {
            name: Some("Weapon 1"),
            item_id: Some("42"),
            active: Some("true"),
            item_pb_url: Some("written"),
            note: Some(&"x".repeat(262145)),
        })
        .unwrap_err();
    assert_eq!(error.kind, AssemblyErrorKind::Resource);
    assert_eq!(s.phase(), ItemSetPhase::Failed);
    let g = s.snapshot().unwrap();
    let row = id(field(&g, current(&g, 2.0), "Weapon 1"));
    assert_eq!(num(field(&g, row, "selItemId")), 42.0);
    assert_eq!(text(field(&g, row, "pbURL")), "written");
    let used = s.usage().bytes;
    s.tighten_byte_limit(used).unwrap();
    assert!(s.tighten_byte_limit(used + 1).is_err());
    let before = s.usage();
    assert!(
        s.snapshot().is_ok(),
        "diagnostic copy remains available after producer limit"
    );
    assert!(
        s.snapshot().is_ok(),
        "repeated diagnostics do not refill or consume producer charges"
    );
    let after = s.usage();
    assert_eq!(
        (
            before.tables,
            before.values,
            before.bytes,
            before.steps,
            before.operations,
            before.pattern_steps
        ),
        (
            after.tables,
            after.values,
            after.bytes,
            after.steps,
            after.operations,
            after.pattern_steps
        )
    );
    let mut exhausted = fresh();
    exhausted.begin_load().unwrap();
    let used = exhausted.usage().bytes;
    assert_eq!(
        exhausted.tighten_byte_limit(used - 1).unwrap_err().kind,
        AssemblyErrorKind::Resource
    );
    let error = exhausted
        .legacy_slot(LegacySlotInput {
            name: Some("Weapon 1"),
            item_id: Some("99"),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(error.kind, AssemblyErrorKind::Resource);
    let prefix = exhausted.snapshot().unwrap();
    let slots = id(field(&prefix, root(&prefix), "slots"));
    assert_eq!(
        num(field(
            &prefix,
            id(field(&prefix, slots, "Weapon 1")),
            "selItemId"
        )),
        0.0
    );
    let limits = ItemSetLimits {
        max_bytes: 0,
        ..ItemSetLimits::default()
    };
    assert_eq!(
        ItemSetState::new(&policy(), limits).err().unwrap().kind,
        AssemblyErrorKind::Resource
    );
}

#[test]
fn trade_text_preserves_earlier_weight_and_blocks_later_rows() {
    let mut s = fresh();
    s.begin_load().unwrap();
    s.trade_weight(TradeWeightInput {
        stat: Some("caller"),
        weight_mult: Some("2"),
        ..Default::default()
    })
    .unwrap();
    let error = s.trade_text().unwrap_err();
    assert_eq!(error.kind, AssemblyErrorKind::Source);
    assert_eq!(error.message, "trade weight text has no attribute table");
    assert_eq!(s.phase(), ItemSetPhase::Failed);
    assert_eq!(s.failure().unwrap().message, error.message);
    let operations = s.usage().operations;
    assert!(
        s.trade_weight(TradeWeightInput {
            stat: Some("shared"),
            ..Default::default()
        })
        .is_err()
    );
    assert_eq!(
        s.usage().operations,
        operations,
        "later entry never executes after the source error"
    );
    let g = s.snapshot().unwrap();
    let trade = id(field(&g, root(&g), "trade"));
    assert_eq!(g.tables[trade.0 as usize - 1].entries.len(), 1);
    let first = id(at(&g, trade, 1.0));
    assert_eq!(text(field(&g, first, "label")), "first");
    assert_eq!(num(field(&g, first, "weightMult")), 2.0);
    assert!(s.trade_transform(0).is_some());
    assert!(s.trade_transform(1).is_none());
}
