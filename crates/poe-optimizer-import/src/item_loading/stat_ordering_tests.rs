use super::*;
use poe_optimizer_data::item_loading::ItemStatOrderingGroups;
use poe_optimizer_engine::lua_pattern::SourcePatternError;
use std::sync::OnceLock;

fn policy() -> ItemStatOrderingPolicy {
    ItemStatOrderingPolicy {
        modifier_table: "CallerExclusive".into(),
        stat_order_field: "callerOrder".into(),
        unique_rarity: "UNIQUE".into(),
        relic_rarity: "RELIC".into(),
        normalize_numbers: ItemStatOrderingSubstitution {
            pattern: "%d+%.?%d*".into(),
            replacement: "#".into(),
        },
        normalize_ranges: ItemStatOrderingSubstitution {
            pattern: "%(%-?#%-#%)".into(),
            replacement: "#".into(),
        },
        flatten_newlines: ItemStatOrderingSubstitution {
            pattern: "\n".into(),
            replacement: " ".into(),
        },
        groups: ItemStatOrderingGroups {
            crafted_custom: 3.0,
            fractured: 1.0,
            ordinary: 2.0,
            compare_order_below: 3.0,
        },
    }
}
fn text(s: &str) -> ItemMetadataValue {
    ItemMetadataValue::Text(s.into())
}
fn record(lines: &[(&str, ItemMetadataValue)]) -> ItemMetadataValue {
    ItemMetadataValue::Table(ItemMetadataTable {
        fields: [(
            "callerOrder".into(),
            ItemMetadataValue::Array(lines.iter().map(|(_, n)| n.clone()).collect()),
        )]
        .into(),
        indexed: lines
            .iter()
            .enumerate()
            .map(|(i, (s, _))| ((i + 1) as i64, text(s)))
            .collect(),
    })
}
fn definitions(rows: Vec<ItemMetadataValue>) -> ItemMetadataTable {
    ItemMetadataTable {
        fields: rows
            .into_iter()
            .enumerate()
            .map(|(i, v)| (format!("record{i}"), v))
            .collect(),
        indexed: BTreeMap::new(),
    }
}
fn number(n: f64) -> ItemMetadataValue {
    ItemMetadataValue::Number(n)
}
fn work<T>(f: impl FnOnce(&mut Work<'_>) -> Result<T>) -> Result<T> {
    let mut matching = budget();
    f(&mut Work {
        matching: &mut matching,
        bytes: 0,
        maximum: MAX_ITEM_LOADING_EVIDENCE_BYTES,
    })
}
fn line(name: &str, flags: &[&str], order: Option<f64>) -> LoadedModLine {
    LoadedModLine {
        line: name.into(),
        source_line: Some(17),
        rune_origins: vec![],
        order: order.map(ItemNumber::new),
        augment_type: None,
        rune_count: None,
        socketed_rune_effect_already_applied: None,
        display_value_scalar: None,
        socketed_augment_type_override: None,
        socketed_soul_core_type: None,
        selection: LineSelection::default(),
        flags: flags.iter().map(|s| (*s).to_owned()).collect(),
        mod_tags: vec![format!("tag {name}")],
        range: ItemNumber::Finite(0.5),
        corrupted_range: ItemNumber::Nil,
        value_scalar: ItemNumber::Nil,
        modifiers: vec![ItemMetadataTable {
            fields: [("identity".into(), text(name))].into(),
            indexed: BTreeMap::new(),
        }],
        extra: Some(format!("extra {name}")),
    }
}
fn names(lines: &[LoadedModLine]) -> Vec<&str> {
    lines.iter().map(|l| l.line.as_str()).collect()
}
fn specimen() -> Vec<LoadedModLine> {
    vec![
        line("unknown", &[], None),
        line("crafted", &["crafted", "fractured"], Some(-100.0)),
        line("fracture9", &["fractured"], Some(9.0)),
        line("ordinary2", &[], Some(2.0)),
        line("custom", &["custom"], Some(-200.0)),
        line("fracture1", &["fractured"], Some(1.0)),
        line("ordinary2again", &[], Some(2.0)),
    ]
}
#[test]
fn exact_lookup_precedes_normalised_minimum_and_preserves_newline_and_numeric_rules() {
    let p = policy();
    let d = definitions(vec![
        record(&[
            ("(10-20) Fire\nPower", number(80.0)),
            ("15 Fire Power", number(7.0)),
        ]),
        record(&[
            ("15 fire power", text(" 5e0 ")),
            ("(30-40) Fire Power", number(12.0)),
            ("Loss -7.25", text("0x10")),
        ]),
    ]);
    work(|w| {
        let cache = StatOrderingPrograms::prepare(&p, Some(&d), w)?;
        assert_eq!(
            cache.lookup("(10-20) FIRE POWER", &p, w)?,
            Some(ItemNumber::Finite(80.0))
        );
        assert_eq!(
            cache.lookup("15 Fire Power", &p, w)?,
            Some(ItemNumber::Finite(5.0))
        );
        assert_eq!(
            cache.lookup("99 Fire Power", &p, w)?,
            Some(ItemNumber::Finite(5.0))
        );
        assert_eq!(
            cache.lookup("Loss -8.5", &p, w)?,
            Some(ItemNumber::Finite(16.0))
        );
        assert_eq!(cache.lookup("unrelated", &p, w)?, None);
        Ok(())
    })
    .unwrap();
}
#[test]
fn raw_ipairs_prefix_ignores_later_holes_and_unconsumed_metadata() {
    let p = policy();
    let mut row = match record(&[("First", number(9.0))]) {
        ItemMetadataValue::Table(t) => t,
        _ => unreachable!(),
    };
    row.indexed.insert(3, ItemMetadataValue::Boolean(false));
    let mut d = definitions(vec![
        ItemMetadataValue::Table(row),
        ItemMetadataValue::Table(ItemMetadataTable {
            fields: [("callerOrder".into(), ItemMetadataValue::Boolean(false))].into(),
            indexed: BTreeMap::new(),
        }),
    ]);
    d.indexed.insert(1, record(&[("First", number(4.0))]));
    work(|w| {
        let cache = StatOrderingPrograms::prepare(&p, Some(&d), w)?;
        assert_eq!(cache.lookup("First", &p, w)?, Some(ItemNumber::Finite(4.0)));
        Ok(())
    })
    .unwrap();
}
#[test]
fn signed_zero_minimum_requires_order_independence_but_smaller_value_can_dominate_it() {
    let p = policy();
    for (a, b) in [(0.0, -0.0), (-0.0, 0.0)] {
        let mut d = definitions(vec![
            record(&[("1 Life", number(a))]),
            record(&[("2 Life", number(b))]),
        ]);
        assert!(
            matches!(work(|w| StatOrderingPrograms::prepare(&p,Some(&d),w)),Err(Failure::Unsupported(m)) if m.contains("signed zero"))
        );
        d.fields
            .insert("last".into(), record(&[("3 Life", number(-1.0))]));
        work(|w| {
            let cache = StatOrderingPrograms::prepare(&p, Some(&d), w)?;
            assert_eq!(
                cache.lookup("4 Life", &p, w)?,
                Some(ItemNumber::Finite(-1.0))
            );
            Ok(())
        })
        .unwrap();
    }
    let d = definitions(vec![
        record(&[("1 Life", number(-0.0))]),
        record(&[("2 Life", number(-0.0))]),
    ]);
    work(|w| {
        let cache = StatOrderingPrograms::prepare(&p, Some(&d), w)?;
        assert_eq!(
            cache
                .lookup("3 Life", &p, w)?
                .unwrap()
                .value()
                .unwrap()
                .to_bits(),
            (-0.0f64).to_bits()
        );
        Ok(())
    })
    .unwrap();
}
#[test]
fn malformed_raw_consumed_rows_are_explicit_not_a_guessed_pairs_error() {
    let p = policy();
    let malformed = [
        ItemMetadataValue::Boolean(false),
        ItemMetadataValue::Array(vec![text("has no order")]),
        record(&[("nonfinite", text("inf"))]),
        record(&[("boolean", ItemMetadataValue::Boolean(false))]),
    ];
    for row in malformed {
        let d = definitions(vec![row]);
        assert!(
            matches!(work(|w| StatOrderingPrograms::prepare(&p,Some(&d),w)),Err(Failure::Unsupported(m)) if m.contains("pairs/error-order"))
        );
    }
    let mut lazy = policy();
    // Gsub permits an unfinished capture when a literal replacement never reads
    // it. A reached unclosed class errors independently of replacement captures.
    lazy.normalize_numbers.pattern = "[".into();
    work(|w| {
        let empty = ItemMetadataTable::default();
        let cache = StatOrderingPrograms::prepare(&lazy, Some(&empty), w)?;
        assert!(matches!(
            cache.lookup("reached", &lazy, w),
            Err(Failure::Pattern(PatternError::Source(
                SourcePatternError::MissingBracket
            )))
        ));
        Ok(())
    })
    .unwrap();
}
#[test]
fn crafted_precedence_original_position_ties_and_row_payload_identity_are_preserved() {
    let mut lines = specimen();
    let pointers: BTreeMap<_, _> = lines
        .iter()
        .map(|l| {
            (
                l.line.clone(),
                (l.line.as_ptr(), l.modifiers.as_ptr(), l.mod_tags.as_ptr()),
            )
        })
        .collect();
    work(|w| sort_lines(&mut lines, &policy(), w)).unwrap();
    assert_eq!(
        names(&lines),
        [
            "fracture1",
            "fracture9",
            "ordinary2",
            "ordinary2again",
            "unknown",
            "crafted",
            "custom"
        ]
    );
    for row in &lines {
        assert_eq!(
            pointers[&row.line],
            (
                row.line.as_ptr(),
                row.modifiers.as_ptr(),
                row.mod_tags.as_ptr()
            )
        );
    }
}
#[test]
fn caller_group_values_and_compare_threshold_control_the_same_algorithm() {
    let mut p = policy();
    p.groups.crafted_custom = 0.0;
    p.groups.fractured = -1.0;
    p.groups.ordinary = -3.0;
    p.groups.compare_order_below = 1.0;
    let mut lines = specimen();
    work(|w| sort_lines(&mut lines, &p, w)).unwrap();
    assert_eq!(
        names(&lines),
        [
            "ordinary2",
            "ordinary2again",
            "unknown",
            "fracture1",
            "fracture9",
            "custom",
            "crafted"
        ]
    );
    p.groups.fractured = 0.0;
    p.groups.ordinary = 0.0;
    p.groups.compare_order_below = 0.0;
    let mut lines = specimen();
    let original = names(&lines)
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    work(|w| sort_lines(&mut lines, &p, w)).unwrap();
    assert_eq!(names(&lines), original);
}
#[test]
fn exhausted_sort_work_and_bytes_do_not_move_or_clone_source_rows() {
    let mut lines = specimen();
    let before: Vec<_> = lines
        .iter()
        .map(|l| (l.line.clone(), l.line.as_ptr(), l.modifiers.as_ptr()))
        .collect();
    let mut matching = MatchBudget::new(MatchLimits {
        max_steps: lines.len() as u64 + 1,
        ..MatchLimits::default()
    });
    let mut w = Work {
        matching: &mut matching,
        bytes: 0,
        maximum: MAX_ITEM_LOADING_EVIDENCE_BYTES,
    };
    assert!(matches!(
        sort_lines(&mut lines, &policy(), &mut w),
        Err(Failure::Pattern(PatternError::Resource(_)))
    ));
    assert_eq!(
        before,
        lines
            .iter()
            .map(|l| (l.line.clone(), l.line.as_ptr(), l.modifiers.as_ptr()))
            .collect::<Vec<_>>()
    );
    let mut matching = budget();
    let mut w = Work {
        matching: &mut matching,
        bytes: 0,
        maximum: 1,
    };
    assert!(matches!(
        sort_lines(&mut lines, &policy(), &mut w),
        Err(Failure::Resource(_))
    ));
    assert_eq!(
        before,
        lines
            .iter()
            .map(|l| (l.line.clone(), l.line.as_ptr(), l.modifiers.as_ptr()))
            .collect::<Vec<_>>()
    );
}
fn catalog(
    defs: ItemMetadataTable,
    change: impl FnOnce(&mut ItemStatOrderingPolicy),
) -> ItemLoadingCatalog {
    static BASE: OnceLock<poe_optimizer_data::game_data::GameDataSnapshot> = OnceLock::new();
    let mut data = BASE
        .get_or_init(|| poe_optimizer_data::game_data::bundled_snapshot().unwrap())
        .item_loading()
        .data()
        .clone();
    data.policy.stat_ordering = policy();
    change(&mut data.policy.stat_ordering);
    data.modifier_tables
        .insert(data.policy.stat_ordering.modifier_table.clone(), defs);
    ItemLoadingCatalog::new(data).unwrap()
}
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
fn one_line_order_write_and_successful_cache_survive_reparse_without_cross_catalog_reuse() {
    let d = catalog(
        definitions(vec![record(&[("(1-9) Life", number(42.0))])]),
        |_| {},
    );
    let raw = "Rarity: UNIQUE\nCaller name\nGold Ring\nImplicits: 0\n{range:0.5}50 Life";
    let mut machine = ItemLoadMachine::new(&d);
    machine.apply_text(raw, &mut Complete).unwrap();
    assert_eq!(
        machine.status(),
        ItemLoadStatus::Complete,
        "{:?}",
        machine.pending()
    );
    assert_eq!(
        machine.state.explicit_mod_lines[0].order,
        Some(ItemNumber::Finite(42.0))
    );
    let key = machine
        .stat_order_programs
        .as_ref()
        .unwrap()
        .exact
        .first_key_value()
        .unwrap()
        .0
        .as_ptr();
    let first_steps = machine.stat_order_budget.steps_used();
    let first_bytes = machine.work_bytes;
    machine.apply_text(raw, &mut Complete).unwrap();
    assert_eq!(
        machine.state.explicit_mod_lines[0].order,
        Some(ItemNumber::Finite(42.0))
    );
    assert_eq!(
        key,
        machine
            .stat_order_programs
            .as_ref()
            .unwrap()
            .exact
            .first_key_value()
            .unwrap()
            .0
            .as_ptr()
    );
    let replay_steps = machine.stat_order_budget.steps_used() - first_steps;
    assert!(replay_steps > 0 && replay_steps < first_steps);
    assert!(machine.work_bytes > first_bytes);
    let mut independent = ItemLoadMachine::new(&d);
    independent.apply_text(raw, &mut Complete).unwrap();
    assert_eq!(
        independent.stat_order_budget.steps_used(),
        first_steps,
        "new machine rebuilds; no shared-cache claim"
    );
    let changed = catalog(
        definitions(vec![record(&[("(1-9) Life", number(13.0))])]),
        |_| {},
    );
    let mut other = ItemLoadMachine::new(&changed);
    other.apply_text(raw, &mut Complete).unwrap();
    assert_eq!(
        other.state.explicit_mod_lines[0].order,
        Some(ItemNumber::Finite(13.0))
    );
}
#[test]
fn later_lookup_error_preserves_earlier_nil_order_write_and_cached_success() {
    let d = catalog(ItemMetadataTable::default(), |p| {
        p.flatten_newlines.pattern = "^x$".into();
        p.flatten_newlines.replacement = "%2".into();
    });
    let mut machine = ItemLoadMachine::new(&d);
    machine.boolean("advancedCopy", true);
    machine.state.rarity = "UNIQUE".into();
    machine.state.explicit_mod_lines =
        vec![line("safe", &[], Some(5.0)), line("x", &[], Some(6.0))];
    assert!(machine.finish_stat_ordering().is_err());
    assert_eq!(machine.status(), ItemLoadStatus::SourceError);
    assert!(machine.stat_order_programs.is_some());
    assert_eq!(machine.state.explicit_mod_lines[0].order, None);
    assert_eq!(
        machine.state.explicit_mod_lines[1].order,
        Some(ItemNumber::Finite(6.0))
    );
    assert_eq!(names(&machine.state.explicit_mod_lines), ["safe", "x"]);
}
#[test]
fn versioned_and_grouped_bypass_lookup_but_sort_while_earlier_frontiers_remain() {
    let d = catalog(definitions(vec![ItemMetadataValue::Boolean(false)]), |_| {});
    for grouped in [false, true] {
        let mut machine = ItemLoadMachine::new(&d);
        machine.boolean("advancedCopy", true);
        machine.state.rarity = "UNIQUE".into();
        if grouped {
            machine.state.variants.groups.insert(1, BTreeMap::new());
        } else {
            machine.state.variants.has_version_list = true;
        }
        machine.state.explicit_mod_lines = vec![
            line("crafted", &["crafted"], Some(1.0)),
            line("normal", &[], Some(2.0)),
        ];
        assert!(machine.finish_stat_ordering().unwrap());
        assert!(machine.stat_order_programs.is_none());
        assert_eq!(
            names(&machine.state.explicit_mod_lines),
            ["normal", "crafted"]
        );
        assert_eq!(
            machine.state.explicit_mod_lines[0].order,
            Some(ItemNumber::Finite(2.0))
        );
    }
    for (raw, kind) in [
        (
            "Rarity: RARE\nCaller\nGold Ring\n{ Prefix Modifier",
            DependencyKind::AdvancedCopyAffixes,
        ),
        (
            "Rarity: UNIQUE\nCaller\nGold Ring\nImplicits: 0\n10% increased modifier magnitudes",
            DependencyKind::ModifierMagnitudes,
        ),
    ] {
        let mut machine = ItemLoadMachine::new(&d);
        machine.apply_text(raw, &mut Complete).unwrap();
        assert_eq!(machine.pending().unwrap().kind, kind);
        assert!(machine.stat_order_programs.is_none());
    }
}
