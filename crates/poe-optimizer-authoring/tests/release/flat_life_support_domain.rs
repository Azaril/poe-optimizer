//! One intrinsic support-source declaration does not close numerical rules,
//! item routing, incoming contributions or the selected build.
use super::{
    flat_life_support_source as source, selected_passive_accounting::with_original,
    tracked_release, tracked_sources,
};
use poe_optimizer_authoring::{SymbolRegistry, load_names};
use poe_optimizer_core::{
    owned_build::{EquipmentDestination, ParameterValue},
    owned_definitions::*,
    owned_draft::{DraftFinalization, DraftListCompletion},
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{EffectDisposition, RuleFact, RuleScratch};
use poe_optimizer_import::owned_normalize::OwnedOriginTarget;
use std::collections::{BTreeMap, BTreeSet};

const PROGRAMS: [(&str, &str); 6] = [
    ("catalyst-scalar", "catalyst-scalar"),
    ("ordered-magnitude-scalar", "magnitude-scalar"),
    ("corrupted-base-factor", "corrupted-base-factor"),
    ("contribute-player-flat-life", "direct-flat-life"),
    ("effective-amount", "effective-value"),
    ("amulet-copy-flat-life", "copied-flat-life"),
];

pub(super) fn declarations() -> Vec<SupportSourceDomainDeclaration> {
    vec![SupportSourceDomainDeclaration {
        owner: SchemaSubject::Definition(source::modifier().address()),
        domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
    }]
}
fn owner() -> &'static DefinitionRules {
    let expected = declarations().remove(0).owner;
    let owners: Vec<_> = tracked_release()
        .release()
        .assembled()
        .checked()
        .rules()
        .input()
        .owners
        .iter()
        .filter(|row| row.owner == expected)
        .collect();
    assert_eq!(owners.len(), 1);
    owners[0]
}
fn program(name: &str) -> &'static RuleProgram {
    owner()
        .programs
        .members
        .iter()
        .find(|program| program.id.as_str() == name)
        .unwrap()
}
fn names(file: &str) -> SymbolRegistry {
    load_names(
        tracked_sources().text(file),
        tracked_release().release().assembled().schema(),
        Default::default(),
    )
    .unwrap()
}
fn quantity_fact(program: &RuleProgram, name: &str, value: f64) -> RuleFact {
    let read = program
        .reads
        .iter()
        .find(|read| read.id.as_str() == name)
        .unwrap();
    let ComputedValueType::Quantity { unit } = &read.value_type else {
        panic!("quantity read {name}")
    };
    RuleFact {
        read: read.id.clone(),
        value: ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap()),
    }
}
fn boolean_fact(program: &RuleProgram, name: &str, value: bool) -> RuleFact {
    let read = program
        .reads
        .iter()
        .find(|read| read.id.as_str() == name)
        .unwrap();
    assert_eq!(read.value_type, ComputedValueType::Boolean);
    RuleFact {
        read: read.id.clone(),
        value: ParameterValue::Boolean(value),
    }
}
fn evaluate(
    program: &RuleProgram,
    facts: &[RuleFact],
    scratch: &mut RuleScratch,
) -> EffectDisposition {
    let recipe = tracked_release().release().assembled().checked();
    let actual = recipe
        .compiled_rules()
        .evaluate(
            &owner().owner,
            &program.id,
            facts,
            recipe.schema().as_ref(),
            scratch,
        )
        .unwrap();
    let fresh = recipe
        .compiled_rules()
        .evaluate(
            &owner().owner,
            &program.id,
            facts,
            recipe.schema().as_ref(),
            &mut recipe.compiled_rules().new_scratch(),
        )
        .unwrap();
    assert_eq!(actual, fresh);
    assert_eq!(actual.owner, owner().owner);
    assert_eq!(actual.owner_programs_closure, owner().programs.closure);
    assert_eq!(actual.effects.len(), 1);
    assert_eq!(actual.effects[0].effect, program.effects[0].effect);
    actual.effects[0].disposition.clone()
}
fn applied(actual: EffectDisposition, expected: f64, unit: &UnitDefId) {
    assert_eq!(
        actual,
        EffectDisposition::Applied {
            value: ParameterValue::Quantity(FiniteQuantity::new(expected, unit.clone()).unwrap()),
        }
    );
}

#[test]
fn sole_modifier_domain_retains_all_six_programs_four_gaps_and_exact_delivery_query() {
    source::check_proof();
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    let rules = recipe.rules().input();
    let domains: Vec<_> = rules
        .support_discovery
        .as_ref()
        .unwrap()
        .providers
        .iter()
        .filter(|row| {
            matches!(
                row.owner,
                SchemaSubject::Definition(DefinitionAddress::Modifier(_))
            )
        })
        .cloned()
        .collect();
    assert_eq!(domains, declarations());
    let body = owner();
    assert_eq!(body.programs.members.len(), PROGRAMS.len());
    let actual: Vec<_> = body
        .programs
        .members
        .iter()
        .map(|p| {
            assert_eq!(p.context, RuleEntityKind::EquipmentUse);
            assert_eq!(p.effects.len(), 1);
            (p.id.as_str(), p.effects[0].id.as_str())
        })
        .collect();
    assert_eq!(actual, PROGRAMS);
    let SchemaClosure::Partial { gaps } = &body.programs.closure else {
        panic!("numerical owner remains Partial")
    };
    let expected: Vec<_> = [
        "source-encoding-and-corrupted-range-unproved",
        "remaining-ordered-magnitude-transforms-unconverted",
        "ordinary-item-routing-unconverted",
        "external-contributor-membership-unconverted",
    ]
    .map(|code| SchemaGap {
        subject: body.owner.clone(),
        facet: SchemaFacet::GameRules,
        code: OwnedDefinitionKey::new(code).unwrap(),
    })
    .into();
    assert_eq!(*gaps, expected);
    for (name, _) in PROGRAMS {
        let effect = &program(name).effects[0].effect;
        if matches!(
            name,
            "contribute-player-flat-life" | "amulet-copy-flat-life"
        ) {
            assert!(matches!(effect, RuleEffectKind::Contribute {
                entity: RuleEntity::Player, stat, contribution: ContributionKind::Add, ..
            } if stat.key().as_str() == "def.000000000000311a"));
        } else {
            assert!(matches!(
                effect,
                RuleEffectKind::Derive {
                    entity: RuleEntity::Modifier,
                    ..
                }
            ));
        }
    }
    let queries = rules.contribution_queries.as_ref().unwrap();
    assert!(!queries.is_complete());
    let query = queries
        .members
        .iter()
        .find(|q| q.id.as_str() == "life-base-contributions")
        .unwrap();
    assert_eq!(query.stat.key().as_str(), "def.000000000000311a");
    assert_eq!(query.contribution, ContributionKind::Add);
    let equipment = query
        .groups
        .iter()
        .find(|g| g.id.as_str() == "equipment")
        .unwrap();
    assert_eq!(equipment.reduction, ContributionReduction::Sum);
    assert_eq!(equipment.ordering, ContributionOrdering::Ordered);
    assert!(equipment.members.is_complete());
    assert_eq!(equipment.members.members.len(), 2);
    let aliases = names("names/contribution-queries-002.kdl");
    let slot_names = [
        "weapon_1",
        "weapon_2",
        "helmet",
        "body_armour",
        "gloves",
        "boots",
        "amulet",
        "ring_1",
        "ring_2",
        "ring_3",
        "belt",
        "flask_1",
        "flask_2",
        "charm_1",
        "charm_2",
        "charm_3",
        "arm_1",
        "arm_2",
        "leg_1",
        "leg_2",
    ];
    let slots: Vec<_> = slot_names
        .iter()
        .map(|name| {
            let DefinitionAddress::EquipmentSlot(slot) = aliases
                .definition(&format!("equipment_slot.{name}"))
                .unwrap()
            else {
                panic!("equipment slot")
            };
            slot.clone()
        })
        .collect();
    for (rank, (name, effect)) in [
        ("contribute-player-flat-life", "direct-flat-life"),
        ("amulet-copy-flat-life", "copied-flat-life"),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            equipment.members.members[rank],
            ContributionMember {
                producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                    owner: body.owner.clone(),
                    program: OwnedDefinitionKey::new(name).unwrap(),
                    effect: OwnedDefinitionKey::new(effect).unwrap(),
                    origin: ContributionOrigin::ItemModifier {
                        slots: slots.clone()
                    },
                }),
                order: Some(ContributionOrder {
                    source_rank: 0,
                    program_rank: rank as u32,
                    effect_rank: 0,
                    slot_ranks: slots
                        .iter()
                        .enumerate()
                        .map(|(rank, slot)| ContributionSlotRank {
                            slot: slot.clone(),
                            rank: rank as u32
                        })
                        .collect(),
                }),
            }
        );
    }
    // The complete equipment partition was already authored; it confers no
    // completeness on the outer query or this modifier's numerical owner.
    assert!(release.evaluation().is_none());
}

#[test]
fn component_magnitude_and_direct_copy_receivers_still_need_their_own_inputs() {
    let recipe = tracked_release().release().assembled().checked();
    let mut scratch = recipe.compiled_rules().new_scratch();
    let aliases = names("names/group-100.kdl");
    let unit = |name| match aliases.definition(name).unwrap() {
        DefinitionAddress::Unit(unit) => unit.clone(),
        _ => panic!("unit"),
    };
    let factor = unit("unit.factor");
    let count = unit("unit.count");
    let life = unit("unit.life");
    assert_ne!(count, life);
    let p = program("catalyst-scalar");
    let DefinitionAddress::Option(catalyst) = aliases.definition("option.catalyst.life").unwrap()
    else {
        panic!("Life catalyst")
    };
    let mut facts: Vec<_> = p
        .reads
        .iter()
        .map(|read| match &read.value_type {
            ComputedValueType::Boolean => {
                boolean_fact(p, read.id.as_str(), read.id.as_str() == "property-life")
            }
            ComputedValueType::Quantity { .. } => quantity_fact(p, read.id.as_str(), 20.0),
            ComputedValueType::Option => RuleFact {
                read: read.id.clone(),
                value: ParameterValue::Option(catalyst.clone()),
            },
            _ => panic!("reviewed catalyst input"),
        })
        .collect();
    applied(evaluate(p, &facts, &mut scratch), 1.2, &factor);
    facts
        .iter_mut()
        .find(|f| f.read.as_str() == "unscalable")
        .unwrap()
        .value = ParameterValue::Boolean(true);
    applied(evaluate(p, &facts, &mut scratch), 1.0, &factor);
    facts
        .iter_mut()
        .find(|f| f.read.as_str() == "unscalable")
        .unwrap()
        .value = ParameterValue::Boolean(false);
    facts
        .iter_mut()
        .find(|f| f.read.as_str() == "property-life")
        .unwrap()
        .value = ParameterValue::Boolean(false);
    applied(evaluate(p, &facts, &mut scratch), 1.0, &factor);
    for (name, read) in [
        ("ordered-magnitude-scalar", "scalar"),
        ("corrupted-base-factor", "corrupted-base"),
    ] {
        let p = program(name);
        applied(
            evaluate(p, &[quantity_fact(p, read, 1.5)], &mut scratch),
            1.5,
            &factor,
        );
        assert!(matches!(
            evaluate(p, &[], &mut scratch),
            EffectDisposition::Unresolved { .. }
        ));
    }
    let p = program("effective-amount");
    // The source proof separately authenticates the unsigned-integer import
    // grammar. The numeric parameter schema is a quantity range, so it rejects
    // negative/out-of-range values but does not itself reject fractions.
    for (raw, expected) in [
        (0.0, 0.0),
        (1.0, 2.0),
        (10.0, 18.0),
        (11.0, 20.0),
        (1_000_000.0, 1_800_000.0),
    ] {
        let facts = [
            quantity_fact(p, "component", raw),
            quantity_fact(p, "corruption-factor", 1.5),
            quantity_fact(p, "magnitude-factor", 1.2),
        ];
        applied(evaluate(p, &facts, &mut scratch), expected, &count);
        for missing in 0..facts.len() {
            let incomplete: Vec<_> = facts
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != missing)
                .map(|(_, fact)| fact.clone())
                .collect();
            assert!(matches!(
                evaluate(p, &incomplete, &mut scratch),
                EffectDisposition::Unresolved { .. }
            ));
        }
        applied(evaluate(p, &facts, &mut scratch), expected, &count);
    }
    for raw in [-10.5, -1.0, 1_000_000.5, 1_000_001.0] {
        let facts = [
            quantity_fact(p, "component", raw),
            quantity_fact(p, "corruption-factor", 1.5),
            quantity_fact(p, "magnitude-factor", 1.2),
        ];
        let error = recipe
            .compiled_rules()
            .evaluate(
                &owner().owner,
                &p.id,
                &facts,
                recipe.schema().as_ref(),
                &mut scratch,
            )
            .unwrap_err();
        assert_eq!(error.path, "facts.component");
        assert_eq!(
            error.message,
            "fact violates declared value schema/membership"
        );
        // A rejected fact cannot poison the reused worker's next legal attempt.
        applied(
            evaluate(
                p,
                &[
                    quantity_fact(p, "component", 10.0),
                    quantity_fact(p, "corruption-factor", 1.5),
                    quantity_fact(p, "magnitude-factor", 1.2),
                ],
                &mut scratch,
            ),
            18.0,
            &count,
        );
    }
    let p = program("contribute-player-flat-life");
    for amount in [17.0, 16.0, 10.0, -10.0, 0.0] {
        applied(
            evaluate(
                p,
                &[
                    boolean_fact(p, "direct-applicability", true),
                    quantity_fact(p, "effective", amount),
                ],
                &mut scratch,
            ),
            amount,
            &life,
        );
    }
    assert_eq!(
        evaluate(
            p,
            &[boolean_fact(p, "direct-applicability", false)],
            &mut scratch
        ),
        EffectDisposition::Inactive
    );
    assert!(matches!(
        evaluate(
            p,
            &[boolean_fact(p, "direct-applicability", true)],
            &mut scratch
        ),
        EffectDisposition::Unresolved { .. }
    ));
    assert!(matches!(
        evaluate(
            p,
            &[
                boolean_fact(p, "direct-applicability", true),
                quantity_fact(p, "effective", 137438953473.0)
            ],
            &mut scratch
        ),
        EffectDisposition::UnsupportedDomain { .. }
    ));
    let p = program("amulet-copy-flat-life");
    for (amount, percent, unscalable, expected) in [
        (17.0, 50.0, false, 8.0),
        (-17.0, 50.0, false, -8.0),
        (17.5, 50.0, false, 8.7),
        (17.5, 100.0, false, 17.5),
        (17.5, 50.0, true, 17.5),
    ] {
        let facts = [
            quantity_fact(p, "effective", amount),
            boolean_fact(p, "eligible", true),
            quantity_fact(p, "pre-amulet-percent", percent),
            boolean_fact(p, "unscalable", unscalable),
        ];
        applied(evaluate(p, &facts, &mut scratch), expected, &life);
    }
    assert_eq!(
        evaluate(p, &[boolean_fact(p, "eligible", false)], &mut scratch),
        EffectDisposition::Inactive
    );
    assert!(matches!(
        evaluate(p, &[boolean_fact(p, "eligible", true)], &mut scratch),
        EffectDisposition::Unresolved { .. }
    ));
}

#[test]
fn four_physical_records_keep_five_exact_selected_uses_and_pending_build_obligations() {
    with_original(|evidence, normalized, selection| {
        let draft = normalized.draft();
        let input = draft.input();
        let preset = input
            .equipment_presets
            .members
            .iter()
            .find(|p| p.id == selection.build.equipment)
            .unwrap();
        assert_eq!(preset.equipment.completion, DraftListCompletion::Complete);
        let selected: BTreeSet<_> = preset.equipment.members.iter().copied().collect();
        let uses: Vec<_> = input
            .equipment
            .members
            .iter()
            .filter(|row| selected.contains(&row.id))
            .map(|row| row.to_resolved().unwrap())
            .collect();
        let aliases = names("names/contribution-queries-002.kdl");
        let mut found = BTreeMap::new();
        let mut modifier_ids = BTreeSet::new();
        let mut occurrences = BTreeSet::new();
        for item in &input.items.members {
            let item_uses: Vec<_> = uses.iter().filter(|use_| use_.item == item.id).collect();
            if item_uses.is_empty() {
                continue;
            }
            let modifiers: Vec<_> = item
                .modifiers
                .members
                .iter()
                .filter(|modifier| modifier.definition.to_resolved() == Some(source::modifier()))
                .collect();
            if modifiers.is_empty() {
                continue;
            }
            assert_eq!(modifiers.len(), 1);
            let modifier = modifiers[0];
            assert!(modifier_ids.insert(modifier.id));
            let origins: Vec<_> = normalized
                .sidecar()
                .origins
                .iter()
                .filter(|origin| {
                    origin.links.contains(&OwnedOriginTarget::Item(item.id))
                        && evidence.row(origin.source).unwrap().occurrence().name() == "Item"
                })
                .collect();
            assert_eq!(origins.len(), 1);
            let origin = origins[0];
            assert!(
                origin
                    .links
                    .contains(&OwnedOriginTarget::Modifier(modifier.id))
            );
            let row = evidence.row(origin.source).unwrap();
            let id: u64 = row
                .attribute("id")
                .unwrap()
                .decoded()
                .unwrap()
                .parse()
                .unwrap();
            let (amount, expected_slots): (f64, &[&str]) = match id {
                19 => (17.0, &["body_armour"]),
                20 => (16.0, &["gloves"]),
                26 => (10.0, &["ring_1", "ring_2"]),
                27 => (10.0, &["belt"]),
                _ => panic!("unreviewed selected flat-Life item {id}"),
            };
            assert!(
                evidence
                    .source_fragment(origin.source)
                    .unwrap()
                    .contains(&format!("+{} to maximum Life", amount as u32))
            );
            let rolls: Vec<_> = modifier
                .rolls
                .members
                .iter()
                .filter_map(|roll| {
                    let slot = roll.slot.to_resolved()?;
                    (slot.slot.key().as_str() == "def.0000000000003101")
                        .then(|| (slot, roll.value.to_resolved().unwrap()))
                })
                .collect();
            assert_eq!(rolls.len(), 1);
            assert_eq!(
                rolls[0].0.declaration,
                SlotOwnerDefId::Modifier(source::modifier())
            );
            let ParameterValue::Quantity(value) = &rolls[0].1 else {
                panic!("flat-Life raw Count")
            };
            assert_eq!(value.value(), amount);
            let expected: BTreeSet<_> = expected_slots
                .iter()
                .map(|name| {
                    let DefinitionAddress::EquipmentSlot(slot) = aliases
                        .definition(&format!("equipment_slot.{name}"))
                        .unwrap()
                    else {
                        panic!("character slot")
                    };
                    slot.clone()
                })
                .collect();
            let actual: BTreeSet<_> = item_uses
                .iter()
                .map(|use_| {
                    assert!(occurrences.insert((use_.id, modifier.id)));
                    let EquipmentDestination::CharacterSlot(slot) = &use_.destination else {
                        panic!("selected character equipment")
                    };
                    slot.clone()
                })
                .collect();
            assert_eq!(actual, expected);
            assert_eq!(item_uses.len(), expected_slots.len());
            assert!(found.insert(id, item_uses.len()).is_none());
            for projection in normalized.sidecar().origins.iter().filter(|row| {
                row.source != origin.source
                    && row
                        .links
                        .contains(&OwnedOriginTarget::Modifier(modifier.id))
            }) {
                let row = evidence.row(projection.source).unwrap();
                assert_eq!(row.occurrence().name(), "ModRange");
                assert_eq!(row.occurrence().parent(), Some(origin.source));
            }
        }
        assert_eq!(found, BTreeMap::from([(19, 1), (20, 1), (26, 2), (27, 1)]));
        assert_eq!(modifier_ids.len(), 4);
        assert_eq!(occurrences.len(), 5);
        // This is an exact source/use census, not five evaluated Life results.
        // Magnitude and delivery still require their separate checked facts.
        let recipe = tracked_release().release().assembled().checked();
        let proof = draft
            .prove_intent(
                recipe.schema().as_ref(),
                Default::default(),
                Default::default(),
            )
            .unwrap();
        let checked = draft
            .finalize_selection_checked(
                recipe.schema().as_ref(),
                selection,
                &proof,
                Default::default(),
                Default::default(),
            )
            .unwrap();
        assert!(
            matches!(checked.finalization(), DraftFinalization::Pending { issues, .. } if issues.len() == 5)
        );
        assert_eq!(checked.draft_digest(), normalized.sidecar().draft);
        assert!(tracked_release().release().evaluation().is_none());
    });
}
