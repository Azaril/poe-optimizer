use super::*;
use crate::{SymbolLimits, program_tests};
use poe_optimizer_core::{owned_build::DeclaredSlot, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_rules::CompiledRulePackage;

fn id<K: DefinitionDomain>(value: &str) -> DefId<K> {
    DefId::parse(
        GameVersionNamespace::new("authored-rule-tests", "v1").unwrap(),
        value,
    )
    .unwrap()
}
fn document(body: &str) -> String {
    format!(
        "queries game=\"authored-rule-tests\" version=\"v1\" coverage=\"complete\" {{\n{body}\n}}\n"
    )
}
fn query(body: &str) -> String {
    document(&format!(
        "query \"q\" stat=\"stat.damage\" contribution=\"add\" {{ group \"g\" reduction=\"sum\" ordering=\"ordered\" coverage=\"complete\" {{\n{body}\n}}; }};"
    ))
}
fn parse(source: &str) -> Result<DeclaredSet<ContributionQuery>> {
    let f = program_tests::enriched_fixture();
    parse_contribution_queries(
        source,
        &f.schema,
        &program_tests::symbols(&f),
        Default::default(),
    )
}
fn program_member(origin: &str) -> String {
    format!(
        "member \"program_effect\" owner=\"item.template\" program=\"local-item\" effect=\"amount\" {{ {origin}; }}"
    )
}
fn quantity(value: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, id("unit.factor")).unwrap())
}

#[test]
fn all_exact_origin_variants_and_both_producer_addresses_preserve_membership_and_order() {
    let actor = DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(id("item.template")),
        slot: id("actor.supplied"),
    };
    let skill = DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(id("item.template")),
        slot: id("supply.skill"),
    };
    let origins = [
        ("origin kind=\"character\"", ContributionOrigin::Character),
        ("origin kind=\"encounter\"", ContributionOrigin::Encounter),
        ("origin kind=\"allocation\"", ContributionOrigin::Allocation),
        ("origin kind=\"reward\"", ContributionOrigin::Reward),
        (
            "origin kind=\"existing_actor\" application=\"player\"",
            ContributionOrigin::ExistingActor {
                application: key("player", 0).unwrap(),
            },
        ),
        (
            "origin kind=\"equipment_use\" { slot \"equipment.ring\"; }",
            ContributionOrigin::EquipmentUse {
                slots: vec![id("equipment.ring")],
            },
        ),
        (
            "origin kind=\"item_modifier\" { slot \"equipment.ring\"; slot \"equipment.ring\"; }",
            ContributionOrigin::ItemModifier {
                slots: vec![id("equipment.ring"), id("equipment.ring")],
            },
        ),
        (
            "origin kind=\"supplied_actor\" { supply \"actor.supplied\"; }",
            ContributionOrigin::SuppliedActor { slots: vec![actor] },
        ),
        (
            "origin kind=\"skill\" authored=\"false\" { supply \"supply.skill\"; }",
            ContributionOrigin::Skill {
                authored: false,
                supplies: vec![skill.clone()],
            },
        ),
        (
            "origin kind=\"action\" authored=\"true\" { supply \"supply.skill\"; }",
            ContributionOrigin::Action {
                authored: true,
                supplies: vec![skill],
            },
        ),
    ];
    let mut body = origins
        .iter()
        .map(|(text, _)| program_member(text))
        .collect::<Vec<_>>()
        .join("\n");
    body.push_str(
        r#"
member "application_group" family="buff" modifier="speed" {
    declaration application="first" effect="speed";
    declaration application="second" effect="speed";
    order source="4" program="2" effect="3" {
        slot-rank "equipment.ring" rank="7";
    }
}
"#,
    );
    let result = parse(&query(&body)).unwrap();
    assert!(result.is_complete());
    let members = &result.members[0].groups[0].members;
    assert!(members.is_complete());
    assert_eq!(members.members.len(), 11);
    for (actual, (_, expected)) in members.members.iter().zip(origins) {
        assert_eq!(actual.order, None);
        assert_eq!(
            actual.producer,
            ContributionProducer::ProgramEffect(ProgramContributionProducer {
                owner: SchemaSubject::Definition(DefinitionAddress::ItemTemplate(id(
                    "item.template"
                ))),
                program: key("local-item", 0).unwrap(),
                effect: key("amount", 0).unwrap(),
                origin: expected,
            })
        );
    }
    assert_eq!(
        members.members[10],
        ContributionMember {
            producer: ContributionProducer::ApplicationGroup(
                ApplicationGroupContributionProducer {
                    family: key("buff", 0).unwrap(),
                    modifier: key("speed", 0).unwrap(),
                    declarations: ["first", "second"]
                        .map(|name| ApplicationContributionDeclaration {
                            application: key(name, 0).unwrap(),
                            effect: key("speed", 0).unwrap()
                        })
                        .to_vec(),
                }
            ),
            order: Some(ContributionOrder {
                source_rank: 4,
                program_rank: 2,
                effect_rank: 3,
                slot_ranks: vec![ContributionSlotRank {
                    slot: id("equipment.ring"),
                    rank: 7
                }],
            }),
        }
    );
    // This is structural DTO preservation, not a grant of semantic authority.
    // Repeated origin slots survive for the existing compiler to assess.
}

#[test]
fn scalar_empty_values_all_channel_and_reduction_variants_and_selection_absence_are_preserved() {
    let cases = [
        (
            "flag",
            ContributionKind::Flag,
            "any",
            ContributionReduction::Any,
            "unordered",
            ContributionOrdering::Unordered,
            "empty type=\"boolean\" value=\"false\";",
            Some(ParameterValue::Boolean(false)),
        ),
        (
            "override",
            ContributionKind::Override,
            "require_agreement",
            ContributionReduction::RequireAgreement,
            "unordered",
            ContributionOrdering::Unordered,
            "",
            None,
        ),
        (
            "add",
            ContributionKind::Add,
            "exact_integer_sum",
            ContributionReduction::ExactIntegerSum,
            "unordered",
            ContributionOrdering::Unordered,
            "empty type=\"integer\" value=\"-9007199254740991\";",
            Some(ParameterValue::Integer(
                BoundedInteger::new(BoundedInteger::MIN).unwrap(),
            )),
        ),
        (
            "increase",
            ContributionKind::Increase,
            "sum",
            ContributionReduction::Sum,
            "ordered",
            ContributionOrdering::Ordered,
            "empty type=\"quantity\" value=\"-0\" unit=\"unit.factor\";",
            Some(quantity(0.0)),
        ),
        (
            "multiply",
            ContributionKind::Multiply,
            "product",
            ContributionReduction::Product,
            "ordered",
            ContributionOrdering::Ordered,
            "empty type=\"option\" value=\"option.enabled\";",
            Some(ParameterValue::Option(id("option.enabled"))),
        ),
    ];
    for (channel, contribution, fold, reduction, order, ordering, empty, expected) in cases {
        let source = document(&format!(
            "query \"q\" stat=\"stat.damage\" contribution=\"{channel}\" {{ group \"g\" reduction=\"{fold}\" ordering=\"{order}\" coverage=\"complete\" {{ {empty} }}; }};"
        ));
        let result = parse(&source).unwrap();
        let q = &result.members[0];
        assert_eq!(q.id.as_str(), "q");
        assert_eq!(q.stat, id("stat.damage"));
        assert_eq!(q.contribution, contribution);
        let group = &q.groups[0];
        assert_eq!(group.id.as_str(), "g");
        assert_eq!(group.reduction, reduction);
        assert_eq!(group.ordering, ordering);
        assert_eq!(group.empty, expected);
        assert_eq!(group.members, DeclaredSet::complete(vec![]));
    }
    let zero = parse(&query("empty type=\"integer\" value=\"0\";")).unwrap();
    assert_eq!(
        zero.members[0].groups[0].empty,
        Some(ParameterValue::Integer(BoundedInteger::new(0).unwrap()))
    );
    let none = parse(&query("")).unwrap();
    assert_eq!(none.members[0].groups[0].empty, None);
}

#[test]
fn partial_root_and_group_gaps_keep_exact_subjects_facets_and_source_order() {
    let source = r#"
queries game="authored-rule-tests" version="v1" coverage="partial" {
    gap subject="item.template" facet="identity" code="unconverted-identity";
    gap subject="actor.supplied" facet="static_links" code="unconverted-links";
    query "q" stat="stat.damage" contribution="add" {
        group "second" reduction="sum" ordering="ordered" coverage="partial" {
            gap subject="item.template" facet="input_schema" code="unconverted-input";
            gap subject="actor.supplied" facet="game_rules" code="unconverted-rules";
        }
        group "first" reduction="sum" ordering="ordered" coverage="complete" {}
    }
}
"#;
    let result = parse(source).unwrap();
    let definition =
        SchemaSubject::Definition(DefinitionAddress::ItemTemplate(id("item.template")));
    let slot = SchemaSubject::Slot(SlotAddress::Actor(DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(id("item.template")),
        slot: id("actor.supplied"),
    }));
    assert_eq!(
        result.closure,
        SchemaClosure::Partial {
            gaps: vec![
                SchemaGap {
                    subject: definition.clone(),
                    facet: SchemaFacet::Identity,
                    code: key("unconverted-identity", 0).unwrap()
                },
                SchemaGap {
                    subject: slot.clone(),
                    facet: SchemaFacet::StaticLinks,
                    code: key("unconverted-links", 0).unwrap()
                },
            ]
        }
    );
    assert_eq!(
        result.members[0]
            .groups
            .iter()
            .map(|group| group.id.as_str())
            .collect::<Vec<_>>(),
        ["second", "first"]
    );
    assert_eq!(
        result.members[0].groups[0].members.closure,
        SchemaClosure::Partial {
            gaps: vec![
                SchemaGap {
                    subject: definition,
                    facet: SchemaFacet::InputSchema,
                    code: key("unconverted-input", 0).unwrap()
                },
                SchemaGap {
                    subject: slot,
                    facet: SchemaFacet::GameRules,
                    code: key("unconverted-rules", 0).unwrap()
                },
            ]
        }
    );
    let subject = program_member("origin kind=\"allocation\"")
        .replace("owner=\"item.template\"", "owner=\"actor.supplied\"");
    assert!(matches!(
        parse(&query(&subject)).unwrap().members[0].groups[0]
            .members
            .members[0]
            .producer,
        ContributionProducer::ProgramEffect(ProgramContributionProducer {
            owner: SchemaSubject::Slot(_),
            ..
        })
    ));
}

#[test]
fn malformed_unknown_ambiguous_and_wrong_typed_references_fail_without_defaults() {
    let base = query(&program_member("origin kind=\"allocation\""));
    assert!(parse(&base).is_ok());
    for source in [
        base.replace("queries ", "unknown "),
        base.replace("version=\"v1\"", "version=\"v2\""),
        base.replace("stat=\"stat.damage\"", "stat=\"unit.factor\""),
        base.replace("stat=\"stat.damage\"", "stat=\"missing\""),
        base.replace("contribution=\"add\"", "contribution=\"not-a-kind\""),
        base.replace("reduction=\"sum\"", "reduction=\"none\""),
        base.replace("ordering=\"ordered\"", "ordering=\"implicit\""),
        base.replace("coverage=\"complete\"", "coverage=\"partial\""),
        base.replace("origin kind=\"allocation\"", "origin kind=\"skill\""),
        base.replace(
            "origin kind=\"allocation\"",
            "origin kind=\"skill\" authored=\"maybe\"",
        ),
        base.replace(
            "origin kind=\"allocation\"",
            "origin kind=\"skill\" authored=#false",
        ),
        base.replace(
            "origin kind=\"allocation\"",
            "origin kind=\"supplied_actor\" { supply \"supply.skill\"; }",
        ),
        base.replace(
            "origin kind=\"allocation\"",
            "origin kind=\"action\" authored=\"true\" { supply \"actor.supplied\"; }",
        ),
        base.replace(
            "origin kind=\"allocation\"",
            "origin kind=\"item_modifier\" { slot \"unit.factor\"; }",
        ),
        base.replace(
            "origin kind=\"allocation\"",
            "origin kind=\"existing_actor\"",
        ),
        base.replace(
            "origin kind=\"allocation\"",
            "origin kind=\"allocation\" extra=\"x\"",
        ),
        base.replace(
            "origin kind=\"allocation\"",
            "origin kind=\"allocation\" { slot \"equipment.ring\"; }",
        ),
        base.replace("origin kind=\"allocation\";", ""),
        base.replace(
            "origin kind=\"allocation\";",
            "origin kind=\"allocation\"; origin kind=\"allocation\";",
        ),
        base.replace("program=\"local-item\"", "program=\"\""),
        base.replace("owner=\"item.template\"", "owner=\"missing\""),
        query("empty type=\"quantity\" value=\"NaN\" unit=\"unit.factor\";"),
        query("empty type=\"quantity\" value=\"inf\" unit=\"unit.factor\";"),
        query("empty type=\"quantity\" value=\"1\";"),
        query("empty type=\"quantity\" value=\"1\" unit=\"stat.damage\";"),
        query("empty type=\"integer\" value=\"9007199254740992\";"),
        query("empty type=\"integer\" value=\"1.1\";"),
        query("empty type=\"boolean\" value=\"false\" unit=\"unit.factor\";"),
        query("empty type=\"option\" value=\"stat.damage\";"),
        query("empty type=\"boolean\" value=\"true\"; empty type=\"boolean\" value=\"false\";"),
        query("gap subject=\"item.template\" facet=\"game_rules\" code=\"bad-complete\";"),
        query(
            "member \"application_group\" family=\"f\" modifier=\"m\" { origin kind=\"allocation\"; };",
        ),
        query(
            "member \"program_effect\" owner=\"item.template\" program=\"p\" effect=\"e\" { origin kind=\"allocation\"; declaration application=\"a\" effect=\"e\"; };",
        ),
        query(
            "member \"application_group\" family=\"f\" modifier=\"m\" { order source=\"-1\" program=\"0\" effect=\"0\"; };",
        ),
        query(
            "member \"application_group\" family=\"f\" modifier=\"m\" { order source=\"0\" program=\"4294967296\" effect=\"0\"; };",
        ),
    ] {
        assert!(
            parse(&source).is_err(),
            "accepted malformed source: {source}"
        );
    }
    let f = program_tests::enriched_fixture();
    let ordinary = program_tests::symbols(&f);
    let mut definitions: Vec<_> = ordinary
        .definitions()
        .map(|(name, address)| (name, address.clone()))
        .collect();
    let mut slots: Vec<_> = ordinary
        .slots()
        .map(|(name, address)| (name, address.clone()))
        .collect();
    definitions.push((
        "ambiguous",
        DefinitionAddress::ItemTemplate(id("item.template")),
    ));
    slots.push((
        "ambiguous",
        ordinary.slot("actor.supplied").unwrap().clone(),
    ));
    let ambiguous =
        SymbolRegistry::new(&f.schema, &definitions, &slots, Default::default()).unwrap();
    assert!(
        parse_contribution_queries(
            &base.replace("owner=\"item.template\"", "owner=\"ambiguous\""),
            &f.schema,
            &ambiguous,
            Default::default()
        )
        .is_err()
    );
}

#[test]
fn duplicate_query_group_and_scalar_children_reject_but_member_sequence_is_not_rewritten() {
    let member = program_member("origin kind=\"allocation\"");
    let result = parse(&query(&format!("{member}\n{member}"))).unwrap();
    assert_eq!(result.members[0].groups[0].members.members.len(), 2);
    assert_eq!(
        result.members[0].groups[0].members.members[0],
        result.members[0].groups[0].members.members[1]
    );
    let group = r#"group "g" reduction="sum" ordering="ordered" coverage="complete" {}"#;
    assert!(
        parse(&document(&format!(
            "query \"q\" stat=\"stat.damage\" contribution=\"add\" {{ {group}; {group}; }}"
        )))
        .is_err()
    );
    let q = format!("query \"q\" stat=\"stat.damage\" contribution=\"add\" {{ {group}; }}");
    assert!(parse(&document(&format!("{q}\n{q}"))).is_err());
    assert!(parse(&query(r#"member "application_group" family="f" modifier="m" { order source="0" program="0" effect="0"; order source="0" program="0" effect="0"; }"#)).is_err());
    assert!(parse(&document("")).unwrap().members.is_empty());
}

#[test]
fn exact_document_limits_and_stale_symbols_are_checked_before_dto_expansion() {
    let f = program_tests::enriched_fixture();
    let symbols = SymbolRegistry::new(
        &f.schema,
        &[
            ("channel", DefinitionAddress::Stat(id("stat.damage"))),
            ("factor", DefinitionAddress::Unit(id("unit.factor"))),
        ],
        &[],
        SymbolLimits::default(),
    )
    .unwrap();
    let source = r#"queries game="authored-rule-tests" version="v1" coverage="complete" {
query "q" stat="channel" contribution="add" {
group "g" reduction="sum" ordering="ordered" coverage="complete" {
empty type="quantity" value="1" unit="factor";
}
}
}"#;
    let text_bytes = [
        "queries",
        "game",
        "authored-rule-tests",
        "version",
        "v1",
        "coverage",
        "complete",
        "query",
        "q",
        "stat",
        "channel",
        "contribution",
        "add",
        "group",
        "g",
        "reduction",
        "sum",
        "ordering",
        "ordered",
        "coverage",
        "complete",
        "empty",
        "type",
        "quantity",
        "value",
        "1",
        "unit",
        "factor",
    ]
    .iter()
    .map(|s| s.len())
    .sum();
    let exact = ManifestLimits {
        max_source_bytes: source.len(),
        max_nodes: 4,
        max_entries: 13,
        max_depth: 4,
        max_text_bytes: text_bytes,
    };
    assert!(parse_contribution_queries(source, &f.schema, &symbols, exact).is_ok());
    for (limits, expected) in [
        (
            ManifestLimits {
                max_source_bytes: exact.max_source_bytes - 1,
                ..exact
            },
            ManifestErrorKind::SourceBytes,
        ),
        (
            ManifestLimits {
                max_nodes: 3,
                ..exact
            },
            ManifestErrorKind::Nodes,
        ),
        (
            ManifestLimits {
                max_entries: 12,
                ..exact
            },
            ManifestErrorKind::Entries,
        ),
        (
            ManifestLimits {
                max_depth: 3,
                ..exact
            },
            ManifestErrorKind::Depth,
        ),
        (
            ManifestLimits {
                max_text_bytes: text_bytes - 1,
                ..exact
            },
            ManifestErrorKind::TextBytes,
        ),
        (
            ManifestLimits {
                max_depth: 65,
                ..exact
            },
            ManifestErrorKind::InvalidLimits,
        ),
    ] {
        assert_eq!(
            parse_contribution_queries(source, &f.schema, &symbols, limits)
                .unwrap_err()
                .kind,
            expected
        );
    }
    let mut input = f.schema.input().clone();
    input
        .definitions
        .retain(|row| row.address() != DefinitionAddress::Stat(id("stat.damage")));
    let changed = OwnedDefinitionSchemaPackage::new(input, Default::default()).unwrap();
    assert_eq!(
        parse_contribution_queries(source, &changed, &symbols, Default::default())
            .unwrap_err()
            .kind,
        ManifestErrorKind::Symbols
    );
}

#[test]
fn lowering_does_not_grant_engine_channel_or_reduction_authority() {
    let mut f = program_tests::enriched_fixture();
    CompiledRulePackage::compile(&f.rules, &f.schema, Default::default()).unwrap();
    let source = document(
        r#"query "wrong-domain" stat="stat.damage" contribution="flag" {
        group "g" reduction="any" ordering="unordered" coverage="complete" { empty type="boolean" value="false"; }
    }"#,
    );
    let lowered = parse_contribution_queries(
        &source,
        &f.schema,
        &program_tests::symbols(&f),
        Default::default(),
    )
    .unwrap();
    f.rules.contribution_queries = Some(lowered);
    assert!(CompiledRulePackage::compile(&f.rules, &f.schema, Default::default()).is_err());
}
