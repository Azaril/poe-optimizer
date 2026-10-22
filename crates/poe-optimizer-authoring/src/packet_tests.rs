//! Packet assembly preserves Core inventories; Engine remains the authority.
use crate::{lowering_tests::fixture, packet::*, program_tests, *};
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, RuleFact, RuleLimits,
};

const SOURCE: &str = r#"program "added" context actor {
    read "level" as level: integer = character_level();
    let positive = level > 0;
    effect "check" = requirement(positive, "positive");
}"#;

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}

fn manifest(f: &fixture::Fixture, owners: &[usize]) -> PacketManifest {
    PacketManifest {
        id: key("packet"),
        appends: owners
            .iter()
            .map(|owner| PacketAppend {
                owner: f.rules.owners[*owner].owner.clone(),
                programs: vec![PacketProgram {
                    id: key("added"),
                    source: "source.rule".into(),
                }],
            })
            .collect(),
    }
}

fn lower(
    f: &fixture::Fixture,
    manifest: &PacketManifest,
    sources: &[RuleSource<'_>],
    limits: PacketLimits,
) -> Result<LoweredPacket, PacketError> {
    lower_packet(
        manifest,
        sources,
        &f.schema,
        &program_tests::symbols(f),
        &f.rules.tables,
        limits,
    )
}

fn sources() -> [RuleSource<'static>; 1] {
    [RuleSource {
        name: "source.rule",
        text: SOURCE,
    }]
}

#[test]
fn shared_source_is_lowered_once_and_appends_preserve_every_other_package_field() {
    let f = fixture::fixture();
    let mut target = f.rules.clone();
    target.owners[0].programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: target.owners[0].owner.clone(),
            facet: SchemaFacet::GameRules,
            code: key("remaining-rule"),
        }],
    };
    let before = target.clone();
    let packet = lower(
        &f,
        &manifest(&f, &[0, 1]),
        &sources(),
        PacketLimits::default(),
    )
    .unwrap();
    assert_eq!(packet.id(), &key("packet"));
    assert_eq!(packet.definitions(), &f.rules.definitions);
    assert_eq!(packet.namespace(), &f.rules.namespace);
    assert_eq!(packet.programs().len(), 1);
    assert_eq!(packet.programs()[0].source, "source.rule");
    assert_eq!(packet.appends()[0].programs, [0]);
    assert_eq!(packet.appends()[1].programs, [0]);
    let mut expected = before.clone();
    let program = packet.programs()[0].lowered.program.clone();
    expected.owners[0].programs.members.push(program.clone());
    expected.owners[1].programs.members.push(program);
    packet.apply_to(&mut target).unwrap();
    // Includes unrelated inventories, releases, operation version, ordering and gaps.
    assert_eq!(target, expected);
    assert_eq!(
        target.owners[0].programs.closure,
        before.owners[0].programs.closure
    );
}

#[test]
fn owner_local_ids_allow_reuse_but_duplicate_exact_members_are_rejected() {
    let f = fixture::fixture();
    assert!(
        lower(
            &f,
            &manifest(&f, &[0, 1]),
            &sources(),
            PacketLimits::default()
        )
        .is_ok()
    );
    let mut duplicate = manifest(&f, &[0, 0]);
    let error = lower(&f, &duplicate, &sources(), PacketLimits::default()).unwrap_err();
    assert_eq!(error.kind, PacketErrorKind::DuplicateProgram);
    assert_eq!(
        error.location,
        PacketLocation::Member {
            append: 1,
            program: 0
        }
    );
    duplicate.appends.truncate(1);
    let repeated = duplicate.appends[0].programs[0].clone();
    duplicate.appends[0].programs.push(repeated);
    assert_eq!(
        lower(&f, &duplicate, &sources(), PacketLimits::default())
            .unwrap_err()
            .kind,
        PacketErrorKind::DuplicateProgram
    );
}

#[test]
fn repeated_owner_blocks_with_distinct_members_preserve_manifest_order() {
    let f = fixture::fixture();
    let mut input = manifest(&f, &[0, 0]);
    input.appends[1].programs[0] = PacketProgram {
        id: key("second"),
        source: "second.rule".into(),
    };
    let supplied = [
        sources()[0],
        RuleSource {
            name: "second.rule",
            text: "program \"second\" context actor {}",
        },
    ];
    let packet = lower(&f, &input, &supplied, PacketLimits::default()).unwrap();
    let mut target = f.rules.clone();
    let start = target.owners[0].programs.members.len();
    packet.apply_to(&mut target).unwrap();
    let ids: Vec<_> = target.owners[0].programs.members[start..]
        .iter()
        .map(|p| p.id.as_str())
        .collect();
    assert_eq!(ids, ["added", "second"]);
}

#[test]
fn source_labels_are_exact_injected_names_and_every_source_is_accounted_for() {
    let f = fixture::fixture();
    let input = manifest(&f, &[0]);
    let duplicate = [sources()[0], sources()[0]];
    assert_eq!(
        lower(&f, &input, &duplicate, PacketLimits::default())
            .unwrap_err()
            .kind,
        PacketErrorKind::DuplicateSource
    );
    assert_eq!(
        lower(&f, &input, &[], PacketLimits::default())
            .unwrap_err()
            .kind,
        PacketErrorKind::MissingSource
    );
    let extra = [
        sources()[0],
        RuleSource {
            name: "unused.rule",
            text: "not even parsed",
        },
    ];
    let error = lower(&f, &input, &extra, PacketLimits::default()).unwrap_err();
    assert_eq!(error.kind, PacketErrorKind::UnusedSource);
    assert_eq!(error.location, PacketLocation::Source(1));
    let empty = [RuleSource {
        name: "",
        text: SOURCE,
    }];
    assert_eq!(
        lower(&f, &input, &empty, PacketLimits::default())
            .unwrap_err()
            .kind,
        PacketErrorKind::InvalidSourceName
    );

    // A virtual label may resemble a path; this API only matches injected text.
    let label = "../not-on-disk/⚡.rule";
    let mut input = input;
    input.appends[0].programs[0].source = label.into();
    let packet = lower(
        &f,
        &input,
        &[RuleSource {
            name: label,
            text: SOURCE,
        }],
        PacketLimits::default(),
    )
    .unwrap();
    assert_eq!(packet.programs()[0].source, label);
}

#[test]
fn manifest_program_ids_must_match_each_shared_source_header() {
    let f = fixture::fixture();
    let mut input = manifest(&f, &[0]);
    input.appends[0].programs[0].id = key("different");
    let error = lower(&f, &input, &sources(), PacketLimits::default()).unwrap_err();
    assert_eq!(error.kind, PacketErrorKind::SourceProgramMismatch);
    assert_eq!(error.location, PacketLocation::Source(0));
    let mut input = manifest(&f, &[0, 1]);
    input.appends[1].programs[0].id = key("different");
    let error = lower(&f, &input, &sources(), PacketLimits::default()).unwrap_err();
    assert_eq!(error.kind, PacketErrorKind::SourceProgramMismatch);
    assert_eq!(
        error.location,
        PacketLocation::Member {
            append: 1,
            program: 0
        }
    );
}

#[test]
fn syntax_and_lowering_diagnostics_keep_the_original_source_span() {
    let f = fixture::fixture();
    let input = manifest(&f, &[0]);
    for text in [
        "// ⚡\nprogram \"added\" context actor { let amount = ; }",
        "program \"added\" context actor { let amount = not_declared; }",
    ] {
        let error = lower(
            &f,
            &input,
            &[RuleSource {
                name: "source.rule",
                text,
            }],
            PacketLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.kind, PacketErrorKind::SourceDiagnostic);
        assert_eq!(error.location, PacketLocation::Source(0));
        assert!(error.diagnostic.unwrap().span.text(text).is_some());
    }
}

#[test]
fn typed_owners_must_exist_but_known_slots_do_not_become_definition_owners() {
    let f = fixture::fixture();
    let mut input = manifest(&f, &[0]);
    input.appends[0].owner = SchemaSubject::Definition(DefinitionAddress::Actor(
        ActorDefId::parse(f.rules.namespace.clone(), "missing-actor").unwrap(),
    ));
    assert_eq!(
        lower(&f, &input, &sources(), PacketLimits::default())
            .unwrap_err()
            .kind,
        PacketErrorKind::UnknownOwner
    );
    input.appends[0].owner = SchemaSubject::Definition(DefinitionAddress::Actor(
        ActorDefId::parse(
            GameVersionNamespace::new("other", "v1").unwrap(),
            "missing-actor",
        )
        .unwrap(),
    ));
    assert_eq!(
        lower(&f, &input, &sources(), PacketLimits::default())
            .unwrap_err()
            .kind,
        PacketErrorKind::UnknownOwner
    );

    let slot = f.schema.input().slots[0].address();
    input.appends[0].owner = SchemaSubject::Slot(slot.clone());
    let packet = lower(&f, &input, &sources(), PacketLimits::default()).unwrap();
    let mut target = f.rules.clone();
    let before = target.clone();
    assert_eq!(
        packet.apply_to(&mut target).unwrap_err().kind,
        PacketErrorKind::UnknownOwner
    );
    assert_eq!(target, before);
    target.owners.push(DefinitionRules {
        owner: SchemaSubject::Slot(slot),
        programs: DeclaredSet::complete(vec![]),
    });
    lower(&f, &input, &sources(), PacketLimits::default())
        .unwrap()
        .apply_to(&mut target)
        .unwrap();
    assert_eq!(
        target.owners.last().unwrap().programs.members[0].id,
        key("added")
    );
}

#[test]
fn a_later_collision_rolls_back_the_entire_append_transaction() {
    let f = fixture::fixture();
    let existing = &f.rules.owners[1].programs.members[0].id;
    let second = format!("program \"{existing}\" context actor {{}}");
    let mut input = manifest(&f, &[0, 1]);
    input.appends[1].programs[0] = PacketProgram {
        id: existing.clone(),
        source: "collision.rule".into(),
    };
    let packet = lower(
        &f,
        &input,
        &[
            sources()[0],
            RuleSource {
                name: "collision.rule",
                text: &second,
            },
        ],
        PacketLimits::default(),
    )
    .unwrap();
    let mut target = f.rules.clone();
    let before = target.clone();
    let error = packet.apply_to(&mut target).unwrap_err();
    assert_eq!(error.kind, PacketErrorKind::DuplicateProgram);
    assert_eq!(
        error.location,
        PacketLocation::Member {
            append: 1,
            program: 0
        }
    );
    assert_eq!(target, before);
}

#[test]
fn application_rejects_changed_identity_missing_owners_and_ambiguous_base_members() {
    let f = fixture::fixture();
    for variant in 0..5 {
        let packet = lower(&f, &manifest(&f, &[0]), &sources(), PacketLimits::default()).unwrap();
        let mut target = f.rules.clone();
        let expected = match variant {
            0 => {
                target.namespace = GameVersionNamespace::new("other", "v1").unwrap();
                PacketErrorKind::NamespaceMismatch
            }
            1 => {
                target.definitions.release.push_str("-changed");
                PacketErrorKind::IdentityMismatch
            }
            2 => {
                target.owners.remove(0);
                PacketErrorKind::UnknownOwner
            }
            3 => {
                target.owners.push(target.owners[0].clone());
                PacketErrorKind::DuplicateOwner
            }
            _ => {
                let repeated = target.owners[1].programs.members[0].clone();
                target.owners[1].programs.members.push(repeated);
                PacketErrorKind::DuplicateProgram
            }
        };
        let before = target.clone();
        assert_eq!(packet.apply_to(&mut target).unwrap_err().kind, expected);
        assert_eq!(target, before);
    }
}

#[test]
fn packet_budgets_charge_reuse_and_accept_exact_boundaries() {
    let f = fixture::fixture();
    let input = manifest(&f, &[0, 1]);
    let parsed = parse_program(SOURCE, ProgramParseLimits::default()).unwrap();
    let first = lower(&f, &input, &sources(), PacketLimits::default()).unwrap();
    let program = &first.programs()[0].lowered.program;
    let symbols = program_tests::symbols(&f);
    let exact = PacketLimits {
        max_sources: 1,
        max_source_bytes: SOURCE.len(),
        max_appends: 2,
        max_programs: 2,
        max_syntax_nodes: parsed.nodes().len() * 2,
        max_nodes: program.nodes.len() * 2,
        max_reads: program.reads.len() * 2,
        max_effects: program.effects.len() * 2,
        max_symbols: symbols.definitions().count() + symbols.slots().count(),
        ..PacketLimits::default()
    };
    assert!(lower(&f, &input, &sources(), exact).is_ok());
    for (kind, limits) in [
        (
            PacketLimit::SourceBytes,
            PacketLimits {
                max_source_bytes: exact.max_source_bytes - 1,
                ..exact
            },
        ),
        (
            PacketLimit::Appends,
            PacketLimits {
                max_appends: 1,
                ..exact
            },
        ),
        (
            PacketLimit::Programs,
            PacketLimits {
                max_programs: 1,
                ..exact
            },
        ),
        (
            PacketLimit::SyntaxNodes,
            PacketLimits {
                max_syntax_nodes: exact.max_syntax_nodes - 1,
                ..exact
            },
        ),
        (
            PacketLimit::Nodes,
            PacketLimits {
                max_nodes: exact.max_nodes - 1,
                ..exact
            },
        ),
        (
            PacketLimit::Reads,
            PacketLimits {
                max_reads: exact.max_reads - 1,
                ..exact
            },
        ),
        (
            PacketLimit::Effects,
            PacketLimits {
                max_effects: exact.max_effects - 1,
                ..exact
            },
        ),
        (
            PacketLimit::Symbols,
            PacketLimits {
                max_symbols: exact.max_symbols - 1,
                ..exact
            },
        ),
        (
            PacketLimit::NameBytes,
            PacketLimits {
                max_name_bytes: 1,
                ..exact
            },
        ),
    ] {
        assert_eq!(
            lower(&f, &input, &sources(), limits).unwrap_err().kind,
            PacketErrorKind::LimitExceeded(kind)
        );
    }
    let supplied = [
        sources()[0],
        RuleSource {
            name: "extra",
            text: "invalid",
        },
    ];
    assert_eq!(
        lower(&f, &input, &supplied, exact).unwrap_err().kind,
        PacketErrorKind::LimitExceeded(PacketLimit::Sources)
    );
}

#[test]
fn base_scan_budgets_are_checked_before_mutation_and_nested_budgets_are_retained() {
    let f = fixture::fixture();
    let input = manifest(&f, &[0]);
    let total = f
        .rules
        .owners
        .iter()
        .map(|owner| owner.programs.members.len())
        .sum();
    let exact = PacketLimits {
        max_base_owners: f.rules.owners.len(),
        max_base_programs: total,
        ..PacketLimits::default()
    };
    lower(&f, &input, &sources(), exact)
        .unwrap()
        .apply_to(&mut f.rules.clone())
        .unwrap();
    for (kind, limits) in [
        (
            PacketLimit::BaseOwners,
            PacketLimits {
                max_base_owners: exact.max_base_owners - 1,
                ..exact
            },
        ),
        (
            PacketLimit::BasePrograms,
            PacketLimits {
                max_base_programs: total - 1,
                ..exact
            },
        ),
    ] {
        let packet = lower(&f, &input, &sources(), limits).unwrap();
        let mut target = f.rules.clone();
        let before = target.clone();
        assert_eq!(
            packet.apply_to(&mut target).unwrap_err().kind,
            PacketErrorKind::LimitExceeded(kind)
        );
        assert_eq!(target, before);
    }
    let mut limits = PacketLimits::default();
    limits.parse.expressions.max_source_bytes = SOURCE.len() - 1;
    let error = lower(&f, &input, &sources(), limits).unwrap_err();
    assert_eq!(error.kind, PacketErrorKind::SourceDiagnostic);
    assert_eq!(
        error.diagnostic.unwrap().code,
        DiagnosticCode::LimitExceeded(LimitKind::SourceBytes)
    );
    limits = PacketLimits::default();
    limits.lower.formulas.max_nodes = 1;
    assert_eq!(
        lower(&f, &input, &sources(), limits).unwrap_err().kind,
        PacketErrorKind::SourceDiagnostic
    );
}

#[test]
fn empty_membership_is_rejected_and_invalid_limits_fail_even_without_sources() {
    let f = fixture::fixture();
    let input = PacketManifest {
        id: key("empty"),
        appends: vec![],
    };
    assert_eq!(
        lower(&f, &input, &[], PacketLimits::default())
            .unwrap_err()
            .kind,
        PacketErrorKind::EmptyPacket
    );
    let mut empty_append = manifest(&f, &[0]);
    empty_append.appends[0].programs.clear();
    assert_eq!(
        lower(&f, &empty_append, &[], PacketLimits::default())
            .unwrap_err()
            .kind,
        PacketErrorKind::EmptyAppend
    );
    for limits in [
        PacketLimits {
            max_programs: 0,
            ..PacketLimits::default()
        },
        PacketLimits {
            parse: ProgramParseLimits {
                expressions: ParseLimits {
                    max_depth: MAX_PARSE_DEPTH + 1,
                    ..ParseLimits::default()
                },
                ..ProgramParseLimits::default()
            },
            ..PacketLimits::default()
        },
    ] {
        assert_eq!(
            lower(&f, &input, &[], limits).unwrap_err().kind,
            PacketErrorKind::InvalidLimits
        );
    }
}

#[test]
fn table_signatures_are_bounded_without_copying_or_certifying_table_rows() {
    let f = fixture::fixture();
    let input = manifest(&f, &[0]);
    let table = |id: &str| IntegerRuleTable {
        id: key(id),
        minimum: BoundedInteger::new(0).unwrap(),
        maximum: BoundedInteger::new(1).unwrap(),
        value_type: ComputedValueType::Integer,
        rows: vec![ParameterValue::Integer(BoundedInteger::new(1).unwrap())],
    };
    let mut limits = PacketLimits::default();
    limits.lower.bindings.max_bindings = 1;
    let error = lower_packet(
        &input,
        &sources(),
        &f.schema,
        &program_tests::symbols(&f),
        &[table("a"), table("b")],
        limits,
    )
    .unwrap_err();
    assert_eq!(
        error.kind,
        PacketErrorKind::LimitExceeded(PacketLimit::Tables)
    );
    // The malformed row inventory is deliberately left to the package compiler.
    assert!(
        lower_packet(
            &input,
            &sources(),
            &f.schema,
            &program_tests::symbols(&f),
            &[table("a")],
            PacketLimits::default()
        )
        .is_ok()
    );
}

#[test]
fn appended_support_program_runs_in_the_existing_engine_with_inactive_guards_preserved() {
    let f = fixture::fixture();
    let source = r#"program "support-applicability" context action {
        read "capability" as supported: boolean = capability(current, capability.supported_action);
        let applicable = supported;
        let factor = quantity(1.5, unit.factor);
        effect "applicability" = support_applicability(applicable);
        effect "supported-factor" when applicable = contribute(current, stat.damage, multiply, factor);
    }"#;
    let mut input = manifest(&f, &[2]);
    input.appends[0].programs[0].id = key("support-applicability");
    let packet = lower(
        &f,
        &input,
        &[RuleSource {
            name: "source.rule",
            text: source,
        }],
        PacketLimits::default(),
    )
    .unwrap();
    let mut target = f.rules.clone();
    target.owners[2].programs.members.clear();
    packet.apply_to(&mut target).unwrap();
    let compiled = CompiledRulePackage::compile(&target, &f.schema, RuleLimits::default()).unwrap();
    let mut cases = 0;
    let mut inactive = false;
    for case in f
        .cases
        .iter()
        .filter(|case| case.program == key("support-applicability"))
    {
        let facts: Vec<_> = case
            .facts
            .iter()
            .map(|(read, value)| RuleFact {
                read: read.clone(),
                value: value.clone(),
            })
            .collect();
        let output = compiled
            .evaluate(
                &case.owner,
                &case.program,
                &facts,
                &f.schema,
                &mut compiled.new_scratch(),
            )
            .unwrap();
        assert_eq!(output.effects.len(), case.expected.len());
        for (effect, expected) in output.effects.iter().zip(&case.expected) {
            assert_eq!(effect.id, expected.id);
            let expected = match &expected.value {
                Some(value) => EffectDisposition::Applied {
                    value: value.clone(),
                },
                None => {
                    inactive = true;
                    EffectDisposition::Inactive
                }
            };
            assert_eq!(effect.disposition, expected);
        }
        cases += 1;
    }
    assert_eq!(cases, 2);
    assert!(inactive);
}

#[test]
fn structurally_valid_packet_cannot_grant_read_or_context_authority() {
    let f = fixture::fixture();
    let source = r#"program "support-applicability" context action {
        read "level" as level: integer = item_level();
        let positive = level > 0;
        effect "applicability" = support_applicability(positive);
    }"#;
    let mut input = manifest(&f, &[2]);
    input.appends[0].programs[0].id = key("support-applicability");
    let packet = lower(
        &f,
        &input,
        &[RuleSource {
            name: "source.rule",
            text: source,
        }],
        PacketLimits::default(),
    )
    .unwrap();
    let mut target = f.rules.clone();
    target.owners[2].programs.members.clear();
    packet.apply_to(&mut target).unwrap();
    let error =
        CompiledRulePackage::compile(&target, &f.schema, RuleLimits::default()).unwrap_err();
    assert!(
        error.to_string().contains("ItemLevel requires item owner"),
        "{error}"
    );
}
