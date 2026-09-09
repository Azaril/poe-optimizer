use poe_optimizer_data::item_loading::*;
use poe_optimizer_import::item_loading::*;
use std::collections::{BTreeMap, BTreeSet};
fn table(fields: impl IntoIterator<Item = (&'static str, ItemMetadataValue)>) -> ItemMetadataTable {
    ItemMetadataTable {
        fields: fields.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        indexed: BTreeMap::new(),
    }
}
fn text(s: &str) -> ItemMetadataValue {
    ItemMetadataValue::Text(s.into())
}
fn catalog() -> ItemLoadingCatalog {
    let source_file = "src/Item.lua".to_owned();
    let assignment = |field, kind| {
        ItemMetadataValue::Table(table([("field", text(field)), ("kind", text(kind))]))
    };
    let policy = ItemLoadingPolicy {
        default_affix_quality: 0.5,
        default_item_quality: 17.0,
        catalysts: vec![],
        line_flags: [
            "implicit", "enchant", "rune", "disabled", "crafted", "custom",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        rarities: ["NORMAL", "MAGIC", "RARE", "UNIQUE", "RELIC"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        header_names: BTreeSet::new(),
        defence_header_keys: BTreeMap::new(),
        compatibility: [
            (
                "base_aliases".into(),
                ItemMetadataValue::Table(table([(
                    "armour_header_rewrites",
                    ItemMetadataValue::Table(ItemMetadataTable::default()),
                )])),
            ),
            (
                "hidden_specs".into(),
                ItemMetadataValue::Table(ItemMetadataTable::default()),
            ),
            (
                "fallback_jewel_socket_counts".into(),
                ItemMetadataValue::Table(ItemMetadataTable::default()),
            ),
            (
                "mod_magnitude_patterns".into(),
                ItemMetadataValue::Array(vec![]),
            ),
            (
                "rarity_roles".into(),
                ItemMetadataValue::Table(table([
                    ("default", text("UNIQUE")),
                    ("normal", text("NORMAL")),
                    ("magic", text("MAGIC")),
                    ("unique", text("UNIQUE")),
                    ("relic", text("RELIC")),
                ])),
            ),
            ("superior_prefix".into(), text("Superior ")),
            (
                "literal_state_flags".into(),
                ItemMetadataValue::Table(table([(
                    "Corrupted",
                    ItemMetadataValue::Table(table([(
                        "corrupted",
                        ItemMetadataValue::Boolean(true),
                    )])),
                )])),
            ),
            (
                "postparse_line_effects".into(),
                ItemMetadataValue::Table(ItemMetadataTable::default()),
            ),
            (
                "header_assignments".into(),
                ItemMetadataValue::Table(table([
                    ("Item Level", assignment("itemLevel", "number")),
                    ("Authored Level", assignment("itemLevel", "number")),
                    ("Quality", assignment("quality", "number")),
                    ("LevelReq", assignment("requirements.level", "number")),
                    ("Note", assignment("note", "text")),
                ])),
            ),
            (
                "selection_headers".into(),
                ItemMetadataValue::Table(table([
                    ("Version", ItemMetadataValue::Boolean(true)),
                    ("Variant", ItemMetadataValue::Boolean(true)),
                    ("Selected Version", ItemMetadataValue::Boolean(true)),
                    ("Selected Variant", ItemMetadataValue::Boolean(true)),
                    ("Selected Variant Group", ItemMetadataValue::Boolean(true)),
                ])),
            ),
            (
                "noncorruptible_types".into(),
                ItemMetadataValue::Array(vec![]),
            ),
            ("fallback_modifier_table".into(), text("Item")),
        ]
        .into_iter()
        .collect(),
    };
    ItemLoadingCatalog::new(ItemLoadingData {
        schema_version: ITEM_LOADING_SCHEMA_VERSION,
        capability: ItemLoadingCapability::DefinitionsOnly,
        source: ItemLoadingSource {
            upstream_revision: "a".repeat(40),
            files: [(source_file.clone(), "b".repeat(64))]
                .into_iter()
                .collect(),
            construction_spans: [(
                "all".into(),
                ItemSourceSpan {
                    path: source_file.clone(),
                    line: 1,
                    end_line: 2,
                    sha256: "c".repeat(64),
                },
            )]
            .into_iter()
            .collect(),
            module_order: vec![source_file.clone()],
        },
        policy,
        bases: vec![ItemBaseDefinition {
            name: "Caller Base".into(),
            item_type: "Caller Type".into(),
            source_module: source_file,
            fields: table([
                ("type", text("Caller Type")),
                ("quality", ItemMetadataValue::Number(20.0)),
                (
                    "req",
                    ItemMetadataValue::Table(table([
                        ("level", ItemMetadataValue::Number(12.0)),
                        ("str", ItemMetadataValue::Number(7.0)),
                    ])),
                ),
            ]),
        }],
        modifier_tables: [("Item".into(), ItemMetadataTable::default())]
            .into_iter()
            .collect(),
        unique_groups: BTreeMap::new(),
        jewel_radii: ItemMetadataTable::default(),
    })
    .unwrap()
}
#[derive(Default)]
struct CompleteProvider;
impl ItemLoadProvider for CompleteProvider {
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
            armour_data: Default::default(),
            modifier_payloads: None,
            requirements: None,
            state_updates: BTreeMap::new(),
            evidence: ItemMetadataTable::default(),
        })
    }
}
#[test]
fn constructor_no_base_and_source_reset_preserve_only_retained_fields() {
    let data = catalog();
    let mut machine = ItemLoadMachine::new(&data);
    assert_eq!(machine.status(), ItemLoadStatus::NoBase);
    assert_eq!(machine.state().assembly_calls, 1);
    assert!(machine.state().raw_lines.is_empty());
    let mut p = CompleteProvider;
    machine.set_xml_attributes(&[("id".into(), "7".into())].into_iter().collect());
    machine.apply_text("Rarity: NORMAL\nCaller Base\nAuthored Level: +60e2\nQuality: 13\nCorrupted\nImplicits: 0\nFirst",&mut p).unwrap();
    assert_eq!(machine.status(), ItemLoadStatus::Complete);
    assert_eq!(
        machine.state().retained_fields["itemLevel"],
        ItemScalar::Number(ItemNumber::Finite(60.0))
    );
    assert_eq!(machine.state().explicit_mod_lines.len(), 1);
    machine
        .apply_text("Rarity: NORMAL\nCaller Base\nImplicits: 0\nSecond", &mut p)
        .unwrap();
    assert_eq!(
        machine.state().retained_fields["itemLevel"],
        ItemScalar::Number(ItemNumber::Finite(60.0))
    );
    assert_eq!(
        machine.state().retained_fields["corrupted"],
        ItemScalar::Boolean(true)
    );
    assert_eq!(
        machine.state().retained_fields["quality"],
        ItemScalar::Number(ItemNumber::Finite(0.0))
    );
    assert_eq!(machine.state().explicit_mod_lines[0].line, "Second");
    assert_eq!(
        machine.state().requirements["level"],
        ItemNumber::Finite(12.0)
    );
    machine.finish_load(&mut p).unwrap();
    assert_eq!(machine.state().assembly_calls, 4);
}
#[test]
fn parser_feedback_preserves_combined_fallback_text_and_unconsumed_next_line() {
    struct Feedback {
        calls: usize,
    }
    impl ItemLoadProvider for Feedback {
        fn format_line(&mut self, r: &FormatRequest) -> DependencyResult<String> {
            DependencyResult::Available(r.text.clone())
        }
        fn parse_modifier(&mut self, _: &ParseRequest) -> DependencyResult<ParseOutcome> {
            self.calls += 1;
            DependencyResult::Available(if self.calls <= 2 {
                ParseOutcome {
                    modifiers: None,
                    extra: None,
                }
            } else {
                ParseOutcome {
                    modifiers: Some(vec![]),
                    extra: None,
                }
            })
        }
        fn assemble(&mut self, _: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
            DependencyResult::Available(AssemblyOutcome {
                armour_data: Default::default(),
                modifier_payloads: None,
                requirements: None,
                state_updates: BTreeMap::new(),
                evidence: ItemMetadataTable::default(),
            })
        }
    }
    let data = catalog();
    let mut m = ItemLoadMachine::new(&data);
    m.apply_text(
        "Rarity: NORMAL\nCaller Base\nImplicits: 0\nFirst\n{crafted}Second (implicit)",
        &mut Feedback { calls: 0 },
    )
    .unwrap();
    assert_eq!(
        m.state()
            .parser_calls
            .iter()
            .map(|r| (r.text.as_str(), r.combined))
            .collect::<Vec<_>>(),
        vec![
            ("First", false),
            ("First Second", true),
            ("First Second", false),
            ("Second", false)
        ]
    );
    assert_eq!(m.state().explicit_mod_lines[0].line, "First");
    assert_eq!(m.state().implicit_mod_lines[0].line, "Second");
    assert_eq!(
        m.state()
            .parser_calls
            .iter()
            .map(|r| r.sequence)
            .collect::<Vec<_>>(),
        vec![1, 3, 4, 6]
    );
}
#[test]
fn successful_combined_feedback_consumes_exactly_one_next_line() {
    struct Combined;
    impl ItemLoadProvider for Combined {
        fn format_line(&mut self, r: &FormatRequest) -> DependencyResult<String> {
            DependencyResult::Available(r.text.clone())
        }
        fn parse_modifier(&mut self, r: &ParseRequest) -> DependencyResult<ParseOutcome> {
            DependencyResult::Available(ParseOutcome {
                modifiers: if r.combined { Some(vec![]) } else { None },
                extra: None,
            })
        }
    }
    let data = catalog();
    let mut m = ItemLoadMachine::new(&data);
    m.apply_text(
        "Rarity: NORMAL\nCaller Base\nImplicits: 0\nFirst\nSecond",
        &mut Combined,
    )
    .unwrap();
    assert_eq!(m.state().parser_calls.len(), 2);
    assert_eq!(m.state().explicit_mod_lines[0].line, "First\nSecond");
    assert_eq!(m.pending().unwrap().kind, DependencyKind::Assembly);
}
#[test]
fn unavailable_dependency_stops_state_and_following_text() {
    let data = catalog();
    let mut m = ItemLoadMachine::new(&data);
    let mut p = UnavailableItemLoadProvider;
    m.apply_text(
        "Rarity: NORMAL\nCaller Base\nImplicits: 0\nFirst\nItem Level: 99",
        &mut p,
    )
    .unwrap();
    assert_eq!(m.status(), ItemLoadStatus::Pending);
    assert_eq!(m.pending().unwrap().kind, DependencyKind::RangeFormatting);
    assert!(!m.state().retained_fields.contains_key("itemLevel"));
    let raw = m.state().raw.clone();
    m.apply_text("Rarity: NORMAL\nCaller Base", &mut p).unwrap();
    assert_eq!(m.state().raw, raw);
    assert!(m.state().explicit_mod_lines.is_empty());
}
#[test]
fn legacy_range_uses_category_order_and_source_nil_index_errors() {
    let data = catalog();
    let mut m = ItemLoadMachine::new(&data);
    m.apply_text(
        "Rarity: NORMAL\nCaller Base\nImplicits: 0\n{enchant}A\n{implicit}B\nC",
        &mut CompleteProvider,
    )
    .unwrap();
    m.apply_mod_range(Some("2"), Some("0x1p-2")).unwrap();
    assert_eq!(
        m.state().implicit_mod_lines[0].range,
        ItemNumber::Finite(0.25)
    );
    assert_eq!(
        m.state().enchant_mod_lines[0].range,
        ItemNumber::Finite(0.5)
    );
    m.apply_mod_range(Some("99"), Some("nan")).unwrap();
    assert_eq!(m.status(), ItemLoadStatus::Complete);
    assert!(m.apply_mod_range(Some("garbage"), Some("1")).is_err());
    assert_eq!(m.status(), ItemLoadStatus::SourceError);
}
#[test]
fn lua_numeric_header_and_xml_identity_have_distinct_grammars() {
    assert_eq!(spec_to_number("+12e3"), ItemNumber::Finite(12.0));
    assert_eq!(spec_to_number("1.2.3"), ItemNumber::Nil);
    assert_eq!(spec_to_number(" 12"), ItemNumber::Nil);
    let data = catalog();
    for (input, expected) in [
        ("0x1.8p1", ItemNumber::Finite(3.0)),
        ("0X.8P-1", ItemNumber::Finite(0.25)),
        ("-0x1p-1074", ItemNumber::Finite(-f64::from_bits(1))),
        ("0x1p1024", ItemNumber::PositiveInfinity),
        ("+infinity", ItemNumber::PositiveInfinity),
        ("-INF", ItemNumber::NegativeInfinity),
    ] {
        let mut m = ItemLoadMachine::new(&data);
        m.set_xml_attributes(&[("id".into(), input.into())].into_iter().collect());
        assert_eq!(
            m.state().retained_fields["id"],
            ItemScalar::Number(expected),
            "{input}"
        );
    }
}
#[test]
fn variant_group_prepass_is_ordered_distinct_and_not_range_expansion() {
    let data = catalog();
    let mut m = ItemLoadMachine::new(&data);
    m.apply_text("Rarity: RARE\nCaller Name\nVariant: A\nVariant: B\nVariant: C\nVersion: Old\nVersion: New\nSelected Variant Group: 1 = 1\nSelected Variant Group: 2 = 1\n{variant:1-3}{version:2}{group:1,2}Caller Base\nImplicits: 0",&mut CompleteProvider).unwrap();
    assert_eq!(
        m.state().variants.group_selections[&1],
        ItemNumber::Finite(1.0)
    );
    assert_eq!(
        m.state().variants.group_selections[&2],
        ItemNumber::Finite(3.0)
    );
    assert!(!m.state().variants.groups[&1].contains_key(&2));
    assert!(m.state().base_present);
    assert_eq!(m.state().name, "Caller Name, Caller Base");
}
#[test]
fn ascii_line_whitespace_and_greedy_ggg_replacements_are_preserved() {
    let data = catalog();
    let mut m = ItemLoadMachine::new(&data);
    m.apply_text(
        "Rarity: NORMAL\nCaller Base\nImplicits: 0\n  [a|b]tail]  \n\u{a0}x\u{a0}",
        &mut CompleteProvider,
    )
    .unwrap();
    assert_eq!(m.state().raw_lines[3], "b]tail");
    assert_eq!(m.state().raw_lines[4], "\u{a0}x\u{a0}");
}

#[test]
fn inspection_binds_exact_occurrences_and_retains_stopped_instruction_suffix() {
    use poe_optimizer_data::game_data::bundled_snapshot;
    use poe_optimizer_import::item_source;
    let snapshot = bundled_snapshot().unwrap();
    let xml = "<PathOfBuilding2><Items><Item id='3'><![CDATA[Rarity: RARE\nCaller Named Item\nTribal Club\nImplicits: 0\n+17 to Strength]]><ModRange id='1' range='.2'/><![CDATA[Rarity: NORMAL\nTribal Club]]></Item><Item id='4'/></Items></PathOfBuilding2>";
    let projection = item_source::project_xml(xml).unwrap();
    let report = inspect(&projection, &snapshot, &mut UnavailableItemLoadProvider).unwrap();
    assert_eq!(report.items.len(), 2);
    assert_eq!(report.source_sha256, projection.source_sha256());
    assert_eq!(&report.data_identity, snapshot.identity());
    let first = &report.items[0];
    assert_eq!(first.status, ItemLoadStatus::Pending);
    assert_eq!(first.authored_id.as_deref(), Some("3"));
    assert_eq!(
        first
            .instructions
            .iter()
            .map(|i| i.consumed_index)
            .collect::<Vec<_>>(),
        vec![None, Some(0), Some(1), Some(2), None]
    );
    assert_eq!(
        first
            .instructions
            .iter()
            .map(|i| i.status)
            .collect::<Vec<_>>(),
        vec![
            InstructionStatus::Executed,
            InstructionStatus::Pending,
            InstructionStatus::NotExecuted,
            InstructionStatus::NotExecuted,
            InstructionStatus::NotExecuted
        ]
    );
    assert!(first.instructions[0].text_sha256.is_none());
    assert!(first.instructions[1].text_sha256.is_some());
    assert_eq!(report.items[1].status, ItemLoadStatus::NoBase);
}
#[test]
fn dependency_metadata_limits_and_source_errors_are_explicit() {
    struct Invalid;
    impl ItemLoadProvider for Invalid {
        fn format_line(&mut self, r: &FormatRequest) -> DependencyResult<String> {
            DependencyResult::Available(r.text.clone())
        }
        fn parse_modifier(&mut self, _: &ParseRequest) -> DependencyResult<ParseOutcome> {
            DependencyResult::Available(ParseOutcome {
                modifiers: Some(vec![table([("bad", ItemMetadataValue::Number(f64::NAN))])]),
                extra: None,
            })
        }
    }
    let data = catalog();
    let mut m = ItemLoadMachine::new(&data);
    assert!(
        m.apply_text("Rarity: NORMAL\nCaller Base\nImplicits: 0\nA", &mut Invalid)
            .unwrap_err()
            .to_string()
            .contains("nonfinite")
    );
    assert!(m.state().explicit_mod_lines.is_empty());
    let mut m = ItemLoadMachine::new(&data);
    assert!(m.apply_text("Item Class: ", &mut CompleteProvider).is_err());
    assert_eq!(m.status(), ItemLoadStatus::SourceError);
}

#[test]
fn malformed_finite_provider_numbers_never_serialize_as_null() {
    assert_eq!(
        serde_json::to_value(ItemNumber::Finite(f64::INFINITY)).unwrap(),
        serde_json::json!({"kind":"positive_infinity"})
    );
    struct Invalid;
    impl ItemLoadProvider for Invalid {
        fn lookup_unique(&mut self, _: &UniqueRequest) -> DependencyResult<Option<UniqueOutcome>> {
            DependencyResult::Available(Some(UniqueOutcome {
                natural_level: Some(ItemNumber::Finite(f64::NAN)),
                level: None,
            }))
        }
    }
    let data = catalog();
    let mut m = ItemLoadMachine::new(&data);
    let error = m
        .apply_text("Rarity: UNIQUE\nCaller Name\nCaller Base", &mut Invalid)
        .unwrap_err();
    assert!(error.to_string().contains("noncanonical"));
}

#[test]
fn source_item_line_sideeffects_cannot_be_bypassed_with_empty_parse_results() {
    let data = catalog();
    for (text, kind) in [
        ("+1 prefix modifier allowed", DependencyKind::CraftedAffixes),
        (
            "This Item gains bonuses from socketed items as though it was a Helmet",
            DependencyKind::RuneReconstruction,
        ),
        (
            "20% increased modifier magnitudes",
            DependencyKind::ModifierMagnitudes,
        ),
    ] {
        let mut m = ItemLoadMachine::new(&data);
        m.apply_text(
            &format!("Rarity: NORMAL\nCaller Base\nImplicits: 0\n{text}"),
            &mut CompleteProvider,
        )
        .unwrap();
        assert_eq!(m.pending().unwrap().kind, kind);
        assert!(m.state().explicit_mod_lines.is_empty());
    }
}

#[test]
fn assembly_requirement_replacement_is_explicit_and_validated() {
    struct Replace {
        invalid: bool,
    }
    impl ItemLoadProvider for Replace {
        fn assemble(&mut self, r: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
            let mut requirements = r.state.requirements.clone();
            requirements.insert(
                "strMod".into(),
                if self.invalid {
                    ItemNumber::Nil
                } else {
                    ItemNumber::Finite(42.0)
                },
            );
            DependencyResult::Available(AssemblyOutcome {
                armour_data: Default::default(),
                modifier_payloads: None,
                requirements: Some(requirements),
                state_updates: BTreeMap::new(),
                evidence: ItemMetadataTable::default(),
            })
        }
    }
    let data = catalog();
    let mut valid = ItemLoadMachine::new(&data);
    valid
        .apply_text(
            "Rarity: NORMAL\nCaller Base",
            &mut Replace { invalid: false },
        )
        .unwrap();
    assert_eq!(
        valid.state().requirements["strMod"],
        ItemNumber::Finite(42.0)
    );
    let mut invalid = ItemLoadMachine::new(&data);
    assert!(
        invalid
            .apply_text(
                "Rarity: NORMAL\nCaller Base",
                &mut Replace { invalid: true }
            )
            .unwrap_err()
            .to_string()
            .contains("requirement")
    );
    assert!(!invalid.state().requirements.contains_key("strMod"));
}

#[test]
fn explicit_assembly_nil_update_removes_retained_field() {
    struct Clear;
    impl ItemLoadProvider for Clear {
        fn assemble(&mut self, _: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
            DependencyResult::Available(AssemblyOutcome {
                armour_data: Default::default(),
                modifier_payloads: None,
                requirements: None,
                state_updates: [("note".into(), ItemScalar::Number(ItemNumber::Nil))]
                    .into_iter()
                    .collect(),
                evidence: ItemMetadataTable::default(),
            })
        }
    }
    let data = catalog();
    let mut machine = ItemLoadMachine::new(&data);
    machine
        .apply_text("Rarity: NORMAL\nCaller Base\nNote: old", &mut Clear)
        .unwrap();
    assert!(!machine.state().retained_fields.contains_key("note"));
}

#[test]
fn assembly_modifier_payloads_preserve_row_identity_and_reject_count_mismatch() {
    struct Replace {
        invalid: bool,
    }
    impl ItemLoadProvider for Replace {
        fn format_line(&mut self, r: &FormatRequest) -> DependencyResult<String> {
            DependencyResult::Available(r.text.clone())
        }
        fn parse_modifier(&mut self, _: &ParseRequest) -> DependencyResult<ParseOutcome> {
            DependencyResult::Available(ParseOutcome {
                modifiers: Some(vec![]),
                extra: None,
            })
        }
        fn assemble(&mut self, _: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
            let mut payloads = AssemblyModifierPayloads::default();
            if !self.invalid {
                payloads.explicit_mod_lines.push(vec![table([(
                    "source",
                    text("caller dependency evidence"),
                )])]);
            }
            DependencyResult::Available(AssemblyOutcome {
                armour_data: Default::default(),
                modifier_payloads: Some(payloads),
                requirements: None,
                state_updates: BTreeMap::new(),
                evidence: ItemMetadataTable::default(),
            })
        }
    }
    let data = catalog();
    let mut valid = ItemLoadMachine::new(&data);
    valid
        .apply_text(
            "Rarity: NORMAL\nCaller Base\nImplicits: 0\nOriginal line",
            &mut Replace { invalid: false },
        )
        .unwrap();
    assert_eq!(valid.state().explicit_mod_lines[0].line, "Original line");
    assert_eq!(
        valid.state().explicit_mod_lines[0].modifiers[0].fields["source"].as_str(),
        Some("caller dependency evidence")
    );
    let mut invalid = ItemLoadMachine::new(&data);
    assert!(
        invalid
            .apply_text(
                "Rarity: NORMAL\nCaller Base\nImplicits: 0\nOriginal line",
                &mut Replace { invalid: true }
            )
            .unwrap_err()
            .to_string()
            .contains("row count")
    );
    assert!(invalid.state().explicit_mod_lines[0].modifiers.is_empty());
}

#[test]
fn oversized_callback_dependency_descriptor_is_rejected_before_loading() {
    struct Callback;
    impl ItemLoadProvider for Callback {
        fn format_line(&mut self, r: &FormatRequest) -> DependencyResult<String> {
            DependencyResult::Available(r.text.clone())
        }
        fn parse_modifier(&mut self, _: &ParseRequest) -> DependencyResult<ParseOutcome> {
            DependencyResult::Available(ParseOutcome {
                modifiers: Some(vec![table([(
                    "value",
                    ItemMetadataValue::Callback(ItemOpaqueFunction {
                        callback: ItemSourceSpan {
                            path: "x".repeat(4097),
                            line: 1,
                            end_line: 1,
                            sha256: "a".repeat(64),
                        },
                    }),
                )])]),
                extra: None,
            })
        }
    }
    let data = catalog();
    let mut machine = ItemLoadMachine::new(&data);
    assert!(
        machine
            .apply_text(
                "Rarity: NORMAL\nCaller Base\nImplicits: 0\nValue",
                &mut Callback
            )
            .unwrap_err()
            .to_string()
            .contains("callback descriptor bound")
    );
    assert!(machine.state().explicit_mod_lines.is_empty());
}

#[test]
fn every_unavailable_provider_message_is_bounded_and_charged_before_retention() {
    struct Pending {
        stage: DependencyKind,
        bytes: usize,
    }
    impl ItemLoadProvider for Pending {
        fn format_line(&mut self, r: &FormatRequest) -> DependencyResult<String> {
            if self.stage == DependencyKind::RangeFormatting {
                DependencyResult::Unavailable("x".repeat(self.bytes))
            } else {
                CompleteProvider.format_line(r)
            }
        }
        fn parse_modifier(&mut self, r: &ParseRequest) -> DependencyResult<ParseOutcome> {
            if self.stage == DependencyKind::ModifierParser {
                DependencyResult::Unavailable("x".repeat(self.bytes))
            } else {
                CompleteProvider.parse_modifier(r)
            }
        }
        fn lookup_unique(&mut self, r: &UniqueRequest) -> DependencyResult<Option<UniqueOutcome>> {
            if self.stage == DependencyKind::UniqueDatabase {
                DependencyResult::Unavailable("x".repeat(self.bytes))
            } else {
                CompleteProvider.lookup_unique(r)
            }
        }
        fn assemble(&mut self, r: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
            if self.stage == DependencyKind::Assembly {
                DependencyResult::Unavailable("x".repeat(self.bytes))
            } else {
                CompleteProvider.assemble(r)
            }
        }
    }
    let data = catalog();
    for stage in [
        DependencyKind::RangeFormatting,
        DependencyKind::ModifierParser,
        DependencyKind::UniqueDatabase,
        DependencyKind::Assembly,
    ] {
        let raw = if stage == DependencyKind::UniqueDatabase {
            "Rarity: UNIQUE\nCaller Name\nCaller Base"
        } else {
            "Rarity: NORMAL\nCaller Base\nImplicits: 0\nA"
        };
        let mut baseline = ItemLoadMachine::new(&data);
        baseline
            .apply_text(raw, &mut Pending { stage, bytes: 0 })
            .unwrap();
        assert_eq!(baseline.pending().unwrap().kind, stage);
        let mut bounded = ItemLoadMachine::new(&data);
        bounded
            .apply_text(
                raw,
                &mut Pending {
                    stage,
                    bytes: MAX_ITEM_LOADING_DEPENDENCY_MESSAGE,
                },
            )
            .unwrap();
        assert_eq!(
            bounded.pending().unwrap().message.len(),
            MAX_ITEM_LOADING_DEPENDENCY_MESSAGE
        );
        assert_eq!(
            bounded.evidence_bytes() - baseline.evidence_bytes(),
            MAX_ITEM_LOADING_DEPENDENCY_MESSAGE
        );
        let mut oversized = ItemLoadMachine::new(&data);
        let error = oversized
            .apply_text(
                raw,
                &mut Pending {
                    stage,
                    bytes: MAX_ITEM_LOADING_DEPENDENCY_MESSAGE + 1,
                },
            )
            .unwrap_err();
        assert!(
            error.to_string().contains("dependency message bound"),
            "{stage:?}: {error}"
        );
        assert!(oversized.pending().is_none());
    }
}

struct ExtraProvider {
    bytes: usize,
}
impl ItemLoadProvider for ExtraProvider {
    fn format_line(&mut self, r: &FormatRequest) -> DependencyResult<String> {
        CompleteProvider.format_line(r)
    }
    fn parse_modifier(&mut self, _: &ParseRequest) -> DependencyResult<ParseOutcome> {
        DependencyResult::Available(ParseOutcome {
            modifiers: Some(vec![]),
            extra: Some("x".repeat(self.bytes)),
        })
    }
    fn assemble(&mut self, r: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
        CompleteProvider.assemble(r)
    }
}

#[test]
fn parser_extra_text_is_charged_and_cannot_accumulate_beyond_machine_budget() {
    let data = catalog();
    let raw = "Rarity: NORMAL\nCaller Base\nImplicits: 0\nA";
    let mut baseline = ItemLoadMachine::new(&data);
    baseline
        .apply_text(raw, &mut ExtraProvider { bytes: 0 })
        .unwrap();
    let mut single = ItemLoadMachine::new(&data);
    single
        .apply_text(
            raw,
            &mut ExtraProvider {
                bytes: MAX_ITEM_LOADING_TEXT,
            },
        )
        .unwrap();
    assert_eq!(
        single.state().explicit_mod_lines[0]
            .extra
            .as_ref()
            .unwrap()
            .len(),
        MAX_ITEM_LOADING_TEXT
    );
    assert_eq!(
        single.evidence_bytes() - baseline.evidence_bytes(),
        MAX_ITEM_LOADING_TEXT
    );
    let mut multiple = ItemLoadMachine::new(&data);
    let error = multiple
        .apply_text(
            &format!("{raw}{}", "\nA".repeat(31)),
            &mut ExtraProvider {
                bytes: MAX_ITEM_LOADING_TEXT,
            },
        )
        .unwrap_err();
    assert!(error.to_string().contains("item evidence size bound"));
    assert!(multiple.state().parser_calls.len() <= 16);
    assert!(
        multiple
            .state()
            .explicit_mod_lines
            .iter()
            .map(|row| row.extra.as_ref().map_or(0, String::len))
            .sum::<usize>()
            < MAX_ITEM_LOADING_EVIDENCE_BYTES
    );
}

#[test]
fn report_budget_includes_parser_extra_text_from_every_occurrence() {
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let xml = format!(
        "<PathOfBuilding2><Items>{}</Items></PathOfBuilding2>",
        (1..=65)
            .map(|id| format!(
                "<Item id='{id}'><![CDATA[Rarity: NORMAL\nTribal Club\nImplicits: 0\nA]]></Item>"
            ))
            .collect::<String>()
    );
    let projection = poe_optimizer_import::item_source::project_xml(&xml).unwrap();
    let error = inspect(
        &projection,
        &snapshot,
        &mut ExtraProvider {
            bytes: MAX_ITEM_LOADING_TEXT,
        },
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("report evidence size bound"),
        "{error}"
    );
}

#[test]
fn assembly_state_bytes_are_bounded_before_any_update_is_applied() {
    struct Large;
    impl ItemLoadProvider for Large {
        fn assemble(&mut self, _: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
            DependencyResult::Available(AssemblyOutcome {
                armour_data: Default::default(),
                modifier_payloads: None,
                requirements: None,
                state_updates: (0..17)
                    .map(|i| {
                        (
                            format!("provider{i}"),
                            ItemScalar::Text("x".repeat(MAX_ITEM_LOADING_TEXT)),
                        )
                    })
                    .collect(),
                evidence: ItemMetadataTable::default(),
            })
        }
    }
    let data = catalog();
    let mut machine = ItemLoadMachine::new(&data);
    let error = machine
        .apply_text("Rarity: NORMAL\nCaller Base", &mut Large)
        .unwrap_err();
    assert!(error.to_string().contains("item evidence size bound"));
    assert!(
        !machine
            .state()
            .retained_fields
            .keys()
            .any(|key| key.starts_with("provider"))
    );
}

#[test]
fn empty_assembly_modifier_tables_count_towards_the_metadata_bound() {
    struct EmptyTables;
    impl ItemLoadProvider for EmptyTables {
        fn format_line(&mut self, r: &FormatRequest) -> DependencyResult<String> {
            CompleteProvider.format_line(r)
        }
        fn parse_modifier(&mut self, r: &ParseRequest) -> DependencyResult<ParseOutcome> {
            CompleteProvider.parse_modifier(r)
        }
        fn assemble(&mut self, r: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
            DependencyResult::Available(AssemblyOutcome {
                armour_data: Default::default(),
                modifier_payloads: Some(AssemblyModifierPayloads {
                    explicit_mod_lines: r
                        .state
                        .explicit_mod_lines
                        .iter()
                        .map(|_| vec![ItemMetadataTable::default(); 4096])
                        .collect(),
                    ..AssemblyModifierPayloads::default()
                }),
                requirements: None,
                state_updates: BTreeMap::new(),
                evidence: ItemMetadataTable::default(),
            })
        }
    }
    let data = catalog();
    let mut machine = ItemLoadMachine::new(&data);
    let error = machine
        .apply_text(
            &format!(
                "Rarity: NORMAL\nCaller Base\nImplicits: 0{}",
                "\nA".repeat(17)
            ),
            &mut EmptyTables,
        )
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("dependency metadata value bound")
    );
    assert!(
        machine
            .state()
            .explicit_mod_lines
            .iter()
            .all(|row| row.modifiers.is_empty())
    );
}

fn defence_catalog(target: Option<&str>, include_target: bool) -> ItemLoadingCatalog {
    let mut data = catalog().data().clone();
    data.policy
        .header_names
        .extend(["Caller Guard".into(), "Caller Second".into()]);
    data.policy.defence_header_keys = [
        ("Caller Guard".into(), "ChosenGuard".into()),
        ("Caller Second".into(), "OtherGuard".into()),
    ]
    .into_iter()
    .collect();
    if let Some(target) = target {
        let rewrite =
            ItemMetadataValue::Table(table([("from", text("Caller Base")), ("to", text(target))]));
        data.policy.compatibility.insert(
            "base_aliases".into(),
            ItemMetadataValue::Table(table([(
                "armour_header_rewrites",
                ItemMetadataValue::Table(table([("Caller Guard", rewrite)])),
            )])),
        );
        if include_target {
            let mut base = data.bases[0].clone();
            base.name = target.into();
            base.item_type = "Replacement Type".into();
            base.fields
                .fields
                .insert("type".into(), text("Replacement Type"));
            base.fields.fields.insert(
                "req".into(),
                ItemMetadataValue::Table(table([("str", ItemMetadataValue::Number(999.0))])),
            );
            data.bases.push(base);
        }
    }
    ItemLoadingCatalog::new(data).unwrap()
}

#[test]
fn injected_defence_header_rebinds_only_base_reference_before_invalid_number() {
    for include_target in [false, true] {
        let catalog = defence_catalog(Some("Replacement Base"), include_target);
        let mut loader = ItemLoadMachine::new(&catalog);
        loader
            .apply_text(
                "Rarity: NORMAL\nCaller Base\nCaller Guard: invalid\nImplicits: 0",
                &mut CompleteProvider,
            )
            .unwrap();
        assert_eq!(
            loader.state().base_name.as_deref(),
            Some("Replacement Base")
        );
        assert_eq!(loader.state().base_present, include_target);
        assert_eq!(loader.state().item_type.as_deref(), Some("Caller Type"));
        assert_eq!(
            loader.state().requirements.get("str"),
            Some(&ItemNumber::new(7.0))
        );
        assert_eq!(loader.state().armour_data, Some(BTreeMap::new()));
        assert!(!loader.state().retained_fields.contains_key("hidden_specs"));
        assert_eq!(
            loader.status(),
            if include_target {
                ItemLoadStatus::Complete
            } else {
                ItemLoadStatus::NoBase
            }
        );
    }
}

#[test]
fn header_key_removal_and_reparse_retain_optional_table_and_other_values() {
    let catalog = defence_catalog(None, false);
    let mut loader = ItemLoadMachine::new(&catalog);
    assert_eq!(loader.state().armour_data, None);
    loader
        .apply_text(
            "Rarity: NORMAL\nCaller Base\nCaller Guard: -0\nCaller Second: -3.5\nImplicits: 0",
            &mut CompleteProvider,
        )
        .unwrap();
    let data = loader.state().armour_data.as_ref().unwrap();
    assert_eq!(
        data["ChosenGuard"].value().unwrap().to_bits(),
        (-0.0f64).to_bits()
    );
    loader
        .apply_text(
            "Rarity: NORMAL\nCaller Base\nCaller Guard: invalid\nImplicits: 0",
            &mut CompleteProvider,
        )
        .unwrap();
    assert_eq!(
        loader.state().armour_data,
        Some(
            [("OtherGuard".into(), ItemNumber::new(-3.5))]
                .into_iter()
                .collect()
        )
    );
    loader
        .apply_text("Rarity: NORMAL\nUnknown Base", &mut CompleteProvider)
        .unwrap();
    assert_eq!(loader.state().armour_data.as_ref().unwrap().len(), 1);
    loader
        .apply_text(
            "Rarity: NORMAL\nCaller Base\nCaller Second: invalid\nImplicits: 0",
            &mut CompleteProvider,
        )
        .unwrap();
    assert_eq!(loader.state().armour_data, Some(BTreeMap::new()));
}

struct ArmourUpdates(std::collections::VecDeque<ArmourDataUpdate>);
impl ItemLoadProvider for ArmourUpdates {
    fn assemble(&mut self, _: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
        DependencyResult::Available(AssemblyOutcome {
            armour_data: self.0.pop_front().unwrap_or_default(),
            modifier_payloads: None,
            requirements: None,
            state_updates: BTreeMap::new(),
            evidence: ItemMetadataTable::default(),
        })
    }
}
#[test]
fn assembly_armour_updates_distinguish_preserve_clear_and_empty_replacement() {
    let catalog = defence_catalog(None, false);
    let mut loader = ItemLoadMachine::new(&catalog);
    let values: BTreeMap<_, _> = [("Computed".into(), ItemNumber::PositiveInfinity)]
        .into_iter()
        .collect();
    let mut provider = ArmourUpdates(
        [
            ArmourDataUpdate::Replace(values.clone()),
            ArmourDataUpdate::Preserve,
            ArmourDataUpdate::Clear,
            ArmourDataUpdate::Replace(BTreeMap::new()),
        ]
        .into_iter()
        .collect(),
    );
    for expected in [
        Some(values.clone()),
        Some(values),
        None,
        Some(BTreeMap::new()),
    ] {
        loader
            .apply_text("Rarity: NORMAL\nCaller Base\nImplicits: 0", &mut provider)
            .unwrap();
        assert_eq!(loader.state().armour_data, expected);
    }
}
#[test]
fn invalid_assembly_armour_replacement_does_not_overwrite_loaded_values() {
    let invalid = [
        [("Bad".into(), ItemNumber::Nil)].into_iter().collect(),
        [("Bad".into(), ItemNumber::Finite(f64::NAN))]
            .into_iter()
            .collect(),
        [("bad\0key".into(), ItemNumber::new(1.0))]
            .into_iter()
            .collect(),
        [(String::new(), ItemNumber::new(1.0))]
            .into_iter()
            .collect(),
        (0..257)
            .map(|i| (format!("key{i}"), ItemNumber::new(1.0)))
            .collect(),
    ];
    for invalid in invalid {
        let catalog = defence_catalog(None, false);
        let mut loader = ItemLoadMachine::new(&catalog);
        let mut provider =
            ArmourUpdates([ArmourDataUpdate::Replace(invalid)].into_iter().collect());
        assert!(
            loader
                .apply_text(
                    "Rarity: NORMAL\nCaller Base\nCaller Guard: 9\nImplicits: 0",
                    &mut provider
                )
                .is_err()
        );
        assert_eq!(
            loader.state().armour_data,
            Some(
                [("ChosenGuard".into(), ItemNumber::new(9.0))]
                    .into_iter()
                    .collect()
            )
        );
    }
}

fn buff_catalog(
    flask: Option<ItemMetadataValue>,
    charm: Option<ItemMetadataValue>,
) -> ItemLoadingCatalog {
    let mut data = catalog().data().clone();
    for (key, value) in [("flask", flask), ("charm", charm)] {
        if let Some(value) = value {
            data.bases[0].fields.fields.insert(key.into(), value);
        }
    }
    ItemLoadingCatalog::new(data).unwrap()
}
fn buffs(lines: &[&str]) -> ItemMetadataValue {
    ItemMetadataValue::Table(table([(
        "buff",
        ItemMetadataValue::Array(lines.iter().map(|line| text(line)).collect()),
    )]))
}
#[derive(Default)]
struct BuffProvider {
    outcomes: BTreeMap<String, DependencyResult<ParseOutcome>>,
}
impl ItemLoadProvider for BuffProvider {
    fn parse_modifier(&mut self, request: &ParseRequest) -> DependencyResult<ParseOutcome> {
        self.outcomes
            .get(&request.text)
            .cloned()
            .unwrap_or_else(|| CompleteProvider.parse_modifier(request))
    }
    fn format_line(&mut self, request: &FormatRequest) -> DependencyResult<String> {
        CompleteProvider.format_line(request)
    }
    fn assemble(&mut self, request: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
        CompleteProvider.assemble(request)
    }
}
#[test]
fn base_buffs_preserve_duplicates_empty_text_and_independent_suppression_before_separators() {
    let data = buff_catalog(
        Some(buffs(&["F", "F", "--------"])),
        Some(buffs(&["F", ""])),
    );
    let mut machine = ItemLoadMachine::new(&data);
    machine
        .apply_text(
            "Rarity: NORMAL\nCaller Base\nF\nF\nF\nImplicits: 0\nX\n--------",
            &mut BuffProvider::default(),
        )
        .unwrap();
    assert_eq!(machine.status(), ItemLoadStatus::Complete);
    let state = machine.state();
    let rows = &state.buff_mod_lines;
    assert_eq!(
        rows.iter().map(|row| row.line.as_str()).collect::<Vec<_>>(),
        ["F", "F", "--------", "F", ""]
    );
    for row in rows {
        assert_eq!(row.source_line, 2);
        assert_eq!(row.selection, LineSelection::default());
        assert!(row.flags.is_empty() && row.mod_tags.is_empty());
        assert_eq!(row.range, ItemNumber::Nil);
        assert_eq!(row.corrupted_range, ItemNumber::Nil);
        assert_eq!(row.value_scalar, ItemNumber::Nil);
    }
    assert_eq!(
        state
            .parser_calls
            .iter()
            .map(|call| call.text.as_str())
            .collect::<Vec<_>>(),
        ["F", "F", "--------", "F", "", "F", "X"]
    );
    assert!(state.parser_calls.iter().all(|call| !call.combined));
    assert_eq!(
        state
            .format_calls
            .iter()
            .map(|call| call.text.as_str())
            .collect::<Vec<_>>(),
        ["F", "X"]
    );
    assert_eq!(
        state.retained_fields["checkSection"],
        ItemScalar::Boolean(false)
    );
}
#[test]
fn base_buffs_keep_nil_parser_output_and_extra_without_external_retry_or_defaults() {
    let data = buff_catalog(None, Some(buffs(&["Unknown", "Partial"])));
    let mut provider = BuffProvider {
        outcomes: [
            (
                "Unknown".into(),
                DependencyResult::Available(ParseOutcome {
                    modifiers: None,
                    extra: Some("".into()),
                }),
            ),
            (
                "Partial".into(),
                DependencyResult::Available(ParseOutcome {
                    modifiers: Some(vec![]),
                    extra: Some(" exact tail ".into()),
                }),
            ),
        ]
        .into_iter()
        .collect(),
    };
    let mut machine = ItemLoadMachine::new(&data);
    machine
        .apply_text("Rarity: NORMAL\nCaller Base", &mut provider)
        .unwrap();
    assert_eq!(machine.state().parser_calls.len(), 2);
    assert!(machine.state().format_calls.is_empty());
    assert_eq!(machine.state().buff_mod_lines[0].extra.as_deref(), Some(""));
    assert_eq!(
        machine.state().buff_mod_lines[1].extra.as_deref(),
        Some(" exact tail ")
    );
    assert!(
        machine
            .state()
            .buff_mod_lines
            .iter()
            .all(|row| row.modifiers.is_empty())
    );
}
#[test]
fn base_buffs_reparse_regenerates_rows_and_modrange_indexes_them_before_ordinary_lines() {
    let data = buff_catalog(None, Some(buffs(&["B"])));
    let mut machine = ItemLoadMachine::new(&data);
    let raw = "Rarity: NORMAL\nCaller Base\nB\nImplicits: 0\nOrdinary";
    machine
        .apply_text(raw, &mut BuffProvider::default())
        .unwrap();
    machine.apply_mod_range(Some("1"), Some("0.25")).unwrap();
    machine.apply_mod_range(Some("2"), Some("0.75")).unwrap();
    assert_eq!(
        machine.state().buff_mod_lines[0].range,
        ItemNumber::new(0.25)
    );
    assert_eq!(
        machine.state().explicit_mod_lines[0].range,
        ItemNumber::new(0.75)
    );
    machine
        .apply_text(raw, &mut BuffProvider::default())
        .unwrap();
    assert_eq!(machine.state().buff_mod_lines.len(), 1);
    assert_eq!(machine.state().buff_mod_lines[0].range, ItemNumber::Nil);
    assert_eq!(
        machine
            .state()
            .parser_calls
            .iter()
            .filter(|call| call.text == "B")
            .count(),
        2
    );
    assert_eq!(machine.state().explicit_mod_lines.len(), 1);
}
#[test]
fn base_buffs_sparse_tables_stop_at_first_gap_and_ignore_named_or_negative_keys() {
    let data = buff_catalog(
        None,
        Some(ItemMetadataValue::Table(table([(
            "buff",
            ItemMetadataValue::Table(ItemMetadataTable {
                fields: [("1".into(), text("Named"))].into_iter().collect(),
                indexed: [
                    (-1, text("Negative")),
                    (1, text("First")),
                    (3, text("After gap")),
                ]
                .into_iter()
                .collect(),
            }),
        )]))),
    );
    let mut machine = ItemLoadMachine::new(&data);
    machine
        .apply_text("Rarity: NORMAL\nCaller Base", &mut BuffProvider::default())
        .unwrap();
    assert_eq!(machine.state().parser_calls.len(), 1);
    assert_eq!(machine.state().parser_calls[0].text, "First");
    assert_eq!(machine.state().buff_mod_lines.len(), 1);
}
#[test]
fn base_buffs_unavailable_or_failed_parser_preserves_only_the_completed_prefix() {
    let data = buff_catalog(
        Some(buffs(&["First", "Stop", "Later"])),
        Some(buffs(&["Other family"])),
    );
    for result in [
        DependencyResult::Unavailable("pending callback".into()),
        DependencyResult::SourceError("source parser error".into()),
        DependencyResult::ResourceError("parser work bound".into()),
    ] {
        let pending = matches!(result, DependencyResult::Unavailable(_));
        let source = matches!(result, DependencyResult::SourceError(_));
        let mut provider = BuffProvider {
            outcomes: [("Stop".into(), result)].into_iter().collect(),
        };
        let mut machine = ItemLoadMachine::new(&data);
        let result = machine.apply_text(
            "Rarity: NORMAL\nCaller Base\nImplicits: 0\nAuthored tail",
            &mut provider,
        );
        assert_eq!(result.is_ok(), pending);
        assert_eq!(machine.state().base_name.as_deref(), Some("Caller Base"));
        assert_eq!(machine.state().buff_mod_lines.len(), 1);
        assert_eq!(machine.state().buff_mod_lines[0].line, "First");
        assert_eq!(machine.state().parser_calls.len(), 2);
        assert!(machine.state().format_calls.is_empty());
        assert_eq!(machine.state().assembly_calls, 1); // Empty constructor only.
        if pending {
            assert_eq!(
                machine.pending().unwrap().kind,
                DependencyKind::ModifierParser
            );
        }
        if source {
            assert_eq!(machine.status(), ItemLoadStatus::SourceError);
        }
    }
}

#[test]
fn base_buffs_bound_generated_rows_independently_of_authored_line_count() {
    let data = buff_catalog(
        None,
        Some(ItemMetadataValue::Table(table([(
            "buff",
            ItemMetadataValue::Array(vec![text("B"); MAX_ITEM_LOADING_LINES + 1]),
        )]))),
    );
    let mut machine = ItemLoadMachine::new(&data);
    let error = machine
        .apply_text("Rarity: NORMAL\nCaller Base", &mut BuffProvider::default())
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("base buff modifier line count bound")
    );
    assert_eq!(machine.state().raw_lines.len(), 2);
    assert_eq!(machine.state().buff_mod_lines.len(), MAX_ITEM_LOADING_LINES);
    assert_eq!(machine.state().parser_calls.len(), MAX_ITEM_LOADING_LINES);
    assert_eq!(machine.state().assembly_calls, 1);
    assert_ne!(machine.status(), ItemLoadStatus::SourceError);
    assert!(machine.evidence_bytes() <= MAX_ITEM_LOADING_EVIDENCE_BYTES);
}

#[test]
fn unique_requirements_treat_nil_as_absent_but_preserve_numeric_zero_and_authored_maximum() {
    struct Requirements(UniqueOutcome);
    impl ItemLoadProvider for Requirements {
        fn lookup_unique(&mut self, _: &UniqueRequest) -> DependencyResult<Option<UniqueOutcome>> {
            DependencyResult::Available(Some(self.0.clone()))
        }
        fn assemble(&mut self, request: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
            CompleteProvider.assemble(request)
        }
    }
    let data = catalog();
    for (natural_level, level, expected_natural) in [
        (None, Some(ItemNumber::new(49.0)), 49.0),
        (Some(ItemNumber::Nil), Some(ItemNumber::new(49.0)), 49.0),
        (
            Some(ItemNumber::new(0.0)),
            Some(ItemNumber::new(49.0)),
            12.0,
        ),
    ] {
        let mut machine = ItemLoadMachine::new(&data);
        machine
            .apply_text(
                "Rarity: UNIQUE\nCaller Name\nCaller Base\nLevelReq: 60",
                &mut Requirements(UniqueOutcome {
                    natural_level,
                    level,
                }),
            )
            .unwrap();
        assert_eq!(machine.status(), ItemLoadStatus::Complete);
        assert_eq!(
            machine.state().requirements["naturalLevel"].value(),
            Some(expected_natural)
        );
        assert_eq!(machine.state().requirements["level"].value(), Some(60.0));
    }
    for (natural_level, level) in [(None, None), (Some(ItemNumber::Nil), Some(ItemNumber::Nil))] {
        let mut machine = ItemLoadMachine::new(&data);
        let error = machine
            .apply_text(
                "Rarity: UNIQUE\nCaller Name\nCaller Base",
                &mut Requirements(UniqueOutcome {
                    natural_level,
                    level,
                }),
            )
            .unwrap_err();
        assert_eq!(machine.status(), ItemLoadStatus::SourceError);
        assert!(
            error
                .to_string()
                .contains("no natural or level requirement")
        );
    }
}

#[test]
fn unique_requirement_maximum_reads_injected_rune_field_and_retains_source_error_prefix() {
    let mut data = catalog().data().clone();
    let ItemMetadataValue::Table(requirements) =
        data.bases[0].fields.fields.get_mut("req").unwrap()
    else {
        panic!()
    };
    requirements
        .fields
        .insert("level".into(), ItemMetadataValue::Number(-0.0));
    let ItemMetadataValue::Table(headers) = data
        .policy
        .compatibility
        .get_mut("header_assignments")
        .unwrap()
    else {
        panic!()
    };
    headers.fields.insert(
        "Caller Rune Requirement".into(),
        ItemMetadataValue::Table(table([
            ("field", text("requirements.runeLevel")),
            ("kind", text("number")),
        ])),
    );
    let data = ItemLoadingCatalog::new(data).unwrap();
    struct ZeroRequirement;
    impl ItemLoadProvider for ZeroRequirement {
        fn lookup_unique(&mut self, _: &UniqueRequest) -> DependencyResult<Option<UniqueOutcome>> {
            DependencyResult::Available(Some(UniqueOutcome {
                natural_level: Some(ItemNumber::new(-0.0)),
                level: None,
            }))
        }
        fn assemble(&mut self, request: &AssemblyRequest) -> DependencyResult<AssemblyOutcome> {
            CompleteProvider.assemble(request)
        }
    }
    for (value, expected) in [("-0", -0.0_f64), ("35", 35.0_f64)] {
        let mut machine = ItemLoadMachine::new(&data);
        machine
            .apply_text(
                &format!(
                    "Rarity: UNIQUE\nCaller Name\nCaller Base\nCaller Rune Requirement: {value}"
                ),
                &mut ZeroRequirement,
            )
            .unwrap();
        assert_eq!(machine.status(), ItemLoadStatus::Complete);
        assert_eq!(
            machine.state().requirements["naturalLevel"]
                .value()
                .unwrap()
                .to_bits(),
            (-0.0_f64).to_bits()
        );
        assert_eq!(
            machine.state().requirements["level"]
                .value()
                .unwrap()
                .to_bits(),
            expected.to_bits()
        );
    }
    let mut machine = ItemLoadMachine::new(&data);
    let error = machine
        .apply_text(
            "Rarity: UNIQUE\nCaller Name\nCaller Base\nCaller Rune Requirement: unavailable",
            &mut ZeroRequirement,
        )
        .unwrap_err();
    assert_eq!(machine.status(), ItemLoadStatus::SourceError);
    assert!(error.to_string().contains("rune level requirement"));
    assert!(!machine.state().requirements.contains_key("runeLevel"));
    // Original source assigns level from naturalLevel before the failing m_max.
    assert_eq!(
        machine.state().requirements["level"]
            .value()
            .unwrap()
            .to_bits(),
        (-0.0_f64).to_bits()
    );
}
