//! Test assembly only: executes injected programs through the ordinary engine.
//! Explicit finite completions below do not certify production owner coverage.
use super::{family, replay::ReplayInput};
use family::{def, key, subject};
use poe_optimizer_core::{
    build_identity::{InstanceAllocator, ItemRecordId},
    owned_build::*,
    owned_definitions::*,
    owned_readiness::*,
    owned_rules::*,
    owned_schema::*,
    owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde_json::{Value, json};
use std::{fs, sync::Arc};

fn decode<T: serde::de::DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}
fn quantity(n: f64, u: u64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, def(u)).unwrap())
}
pub fn declare(i: &mut ReplayInput, owner: SchemaSubject, p: RuleProgram, stage: &str) {
    let row = i
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == owner)
        .unwrap();
    if let Some(old) = row.programs.members.iter().find(|old| old.id == p.id) {
        assert_eq!(old, &p);
        return;
    }
    i.stages.programs.members.push(StagedRuleProgram {
        owner: owner.clone(),
        program: p.id.clone(),
        stage: key(stage),
    });
    i.stages
        .readiness
        .as_mut()
        .unwrap()
        .programs
        .members
        .push(ReadinessProgram {
            owner,
            program: p.id.clone(),
            phase: if stage == "prepare" { ReadinessPhase::Structural } else { ReadinessPhase::Execution },
            role: if stage == "prepare" { ReadinessProgramRole::PreparationFacts } else { ReadinessProgramRole::Execution },
            outputs: if stage == "prepare" { p.effects.iter().map(|e| {
                let RuleEffectKind::Derive { entity, stat, .. } = &e.effect else { panic!("pure preparation derivation") };
                decode(&json!({"kind":"stat", "scope":if *entity == RuleEntity::Modifier {"modifier"}else{"equipment_use"}, "stat":stat}))
            }).collect() } else { vec![] },
        });
    row.programs.members.push(p);
}
/// Adds exact authored component data; existing template/modifier coverage is
/// retained. Callers own selected physical inventory and all finite test closures.
pub fn install(i: &mut ReplayInput) {
    let d: Value = family::read("dependencies.json");
    let c = family::consumer();
    for value in d["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .cloned()
        .chain(c.definitions.iter().map(|d| json!(d)))
    {
        let desc: DefinitionDescriptor = decode(&value);
        if !i
            .schema
            .definitions
            .iter()
            .any(|x| x.address() == desc.address())
        {
            i.schema.definitions.push(desc.clone());
        }
        if let DefinitionDescriptor::ItemTemplate(r) = &desc
            && let SchemaState::Known(s) = &r.schema
        {
            schema_mut(i, r.id.clone()).quality = s.quality.clone();
        }
        if !i
            .rules
            .owners
            .iter()
            .any(|x| x.owner == SchemaSubject::Definition(desc.address()))
        {
            i.rules.owners.push(DefinitionRules {
                owner: SchemaSubject::Definition(desc.address()),
                programs: DeclaredSet::complete(vec![]),
            });
        }
    }
    for value in d["slots"].as_array().unwrap() {
        let desc: SlotDescriptor = decode(value);
        if let SlotDescriptor::Parameter(p) = &desc
            && let SlotOwnerDefId::ItemTemplate(template) = &p.id.declaration
        {
            let s = schema_mut(i, template.clone());
            if !s.declarations.parameters.members.contains(&p.id) {
                continue;
            }
        }
        if !i.schema.slots.iter().any(|x| x.address() == desc.address()) {
            i.schema.slots.push(desc);
        }
    }
    // This portable replay carries only its explicitly selected modifier
    // vocabulary. Retain Partial coverage while removing dangling references
    // to the rest of the production catalogue; the archived schema is unchanged.
    let available: std::collections::BTreeSet<_> =
        i.schema.definitions.iter().map(|d| d.address()).collect();
    for d in &mut i.schema.definitions {
        if let DefinitionDescriptor::ItemTemplate(r) = d
            && let SchemaState::Known(s) = &mut r.schema
        {
            s.modifiers
                .members
                .retain(|m| available.contains(&m.address()));
        }
    }
    for (id, pred) in [
        ("item-evasion-inputs", "deliver"),
        ("item-evasion-delivery", "item-evasion-inputs"),
        ("item-evasion-local", "item-evasion-delivery"),
        ("item-evasion-final", "item-evasion-local"),
    ] {
        i.stages.stages.push(EvaluationStage {
            id: key(id),
            predecessors: vec![key(pred)],
        });
    }
    for value in d["owners"].as_array().unwrap() {
        let original: DefinitionRules = decode(value);
        let modifier = matches!(
            &original.owner,
            SchemaSubject::Definition(DefinitionAddress::Modifier(_))
        );
        for p in &original.programs.members {
            if modifier
                || [
                    "raw-base-defence-profile",
                    "declared-standard-item-quality",
                    "catalyst-inputs",
                ]
                .contains(&p.id.as_str())
            {
                declare(
                    i,
                    original.owner.clone(),
                    p.clone(),
                    if modifier || p.id.as_str() == "catalyst-inputs" {
                        "prepare"
                    } else {
                        "item-evasion-inputs"
                    },
                );
            }
        }
        // Do not overwrite an existing controlled fixture's closure. Newly
        // introduced production owners retain their exact original gaps.
        if modifier {
            i.rules
                .owners
                .iter_mut()
                .find(|o| o.owner == original.owner)
                .unwrap()
                .programs
                .closure = original.programs.closure;
        }
    }
    for o in &c.owners {
        for p in &o.programs.members {
            declare(
                i,
                o.owner.clone(),
                p.clone(),
                if o.owner == subject(def::<StatDefinition>(family::LOCAL)) {
                    "item-evasion-local"
                } else {
                    "item-evasion-final"
                },
            );
        }
    }
    for delivery in c.delivery {
        declare(i, delivery.owner, delivery.program, "item-evasion-delivery");
    }
    for q in &c.queries {
        i.stages.frozen_channels.push(FrozenStageChannel {
            channel: StageChannel::Contributions {
                scope: RuleEntityKind::EquipmentUse,
                stat: q.stat.clone(),
                contribution: q.contribution,
            },
            stage: key("item-evasion-delivery"),
        });
    }
    i.rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(c.queries);
    i.rules.receivers.members.extend(c.receivers);
    let declarations: Vec<_> = i
        .rules
        .owners
        .iter()
        .filter(|o| {
            !i.rules
                .support_discovery
                .as_ref()
                .unwrap()
                .providers
                .iter()
                .any(|p| p.owner == o.owner)
        })
        .map(|o| SupportSourceDomainDeclaration {
            owner: o.owner.clone(),
            domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
        })
        .collect();
    i.rules
        .support_discovery
        .as_mut()
        .unwrap()
        .providers
        .extend(declarations);
}
fn schema_mut(i: &mut ReplayInput, template: ItemTemplateDefId) -> &mut ItemTemplateSchema {
    let d = i
        .schema
        .definitions
        .iter_mut()
        .find(|x| x.address() == template.address())
        .unwrap();
    let DefinitionDescriptor::ItemTemplate(r) = d else {
        unreachable!()
    };
    let SchemaState::Known(s) = &mut r.schema else {
        unreachable!()
    };
    s
}
pub fn fixture(template: u64) -> ReplayInput {
    let mut i = ReplayInput::decode(
        &fs::read(family::root().join("tests/fixtures/owned-sniper-replay.json.gz")).unwrap(),
    );
    install(&mut i);
    let d: Value = family::read("dependencies.json");
    let old_item = i
        .build
        .equipment
        .iter()
        .find(|e| e.destination == EquipmentDestination::CharacterSlot(def(0x66)))
        .unwrap()
        .item;
    let mut allocator = InstanceAllocator::from_state(i.build.allocator);
    let id = allocator.allocate().unwrap();
    // Preserve every unrelated fixture occurrence. Replace only the selected
    // helmet record; using another admitted template is an injected control.
    i.build
        .equipment
        .iter_mut()
        .find(|e| e.item == old_item)
        .unwrap()
        .item = id;
    let original = d["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["template"]["value"] == json!(def::<ItemTemplateDefinition>(template)));
    let mut parameters: Vec<ParameterAssignment> = original
        .map(|v| {
            v["parameters"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| ParameterAssignment {
                    slot: decode(&p["slot"]["value"]),
                    value: decode(&p["value"]["value"]),
                })
                .collect()
        })
        .unwrap_or_default();
    let declared = schema_mut(&mut i, def(template))
        .declarations
        .parameters
        .members
        .clone();
    parameters.retain(|p| declared.contains(&p.slot));
    i.build.items.push(ItemRecord {
        id,
        template: def(template),
        parameters,
        item_level: Some(80),
        quality: Some(QualitySelection {
            kind: def(6),
            amount: FiniteQuantity::new(20., def(2)).unwrap(),
        }),
        modifiers: vec![],
        modifier_order: vec![],
    });
    i.build.allocator = allocator.state();
    let modifiers: Vec<_> = d["families"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| decode(&f["canonical"]))
        .collect();
    let s = schema_mut(&mut i, def(template));
    s.equipment_slots = DeclaredSet::complete(vec![def(0x66)]);
    s.quality.allowed_kinds = DeclaredSet::complete(vec![def(6)]);
    for modifier in &modifiers {
        if !s.modifiers.members.contains(modifier) {
            s.modifiers.members.push(modifier.clone());
        }
    }
    s.modifiers.closure = SchemaClosure::Complete;
    s.declarations.parameters.closure = SchemaClosure::Complete;
    s.declarations.choices.closure = SchemaClosure::Complete;
    s.declarations.grants.closure = SchemaClosure::Complete;
    s.declarations.actors.closure = SchemaClosure::Complete;
    s.declarations.skill_grants.closure = SchemaClosure::Complete;
    s.declarations.outputs.closure = SchemaClosure::Complete;
    s.declarations.sockets.closure = SchemaClosure::Complete;
    s.socket_destinations = DeclaredSet::complete(vec![]);
    i.rules
        .owners
        .iter_mut()
        .find(|o| o.owner == subject(def::<ItemTemplateDefinition>(template)))
        .unwrap()
        .programs
        .closure = SchemaClosure::Complete;
    for m in modifiers {
        let d = i
            .schema
            .definitions
            .iter_mut()
            .find(|d| d.address() == m.address())
            .unwrap();
        let DefinitionDescriptor::Modifier(d) = d else {
            unreachable!()
        };
        let SchemaState::Known(s) = &mut d.schema else {
            unreachable!()
        };
        s.declarations.parameters.closure = SchemaClosure::Complete;
        i.rules
            .owners
            .iter_mut()
            .find(|o| o.owner == subject(m.clone()))
            .unwrap()
            .programs
            .closure = SchemaClosure::Complete;
    }
    i.rebind_test_edit().unwrap();
    i
}
fn selected_item(i: &ReplayInput) -> ItemRecordId {
    i.build
        .equipment
        .iter()
        .find(|e| e.destination == EquipmentDestination::CharacterSlot(def(0x66)))
        .unwrap()
        .item
}
pub fn add_modifier(i: &mut ReplayInput, modifier: u64, amount: f64) {
    add_modifier_to_item(i, selected_item(i), modifier, amount);
}
pub fn add_modifier_to_item(
    i: &mut ReplayInput,
    item_id: ItemRecordId,
    modifier: u64,
    amount: f64,
) {
    let d: Value = family::read("dependencies.json");
    let f = d["families"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["canonical"] == json!(def::<ModifierDefinition>(modifier)))
        .unwrap();
    let owner = SlotOwnerDefId::Modifier(def(modifier));
    let amount_slot: DeclaredSlot<ParameterSlotDefId> = decode(&f["canonical_inputs"]["amount"]);
    let rolls = i
        .schema
        .slots
        .iter()
        .filter_map(|d| {
            let SlotDescriptor::Parameter(r) = d else {
                return None;
            };
            if r.id.declaration != owner {
                return None;
            }
            let SchemaState::Known(s) = &r.schema else {
                panic!("slot")
            };
            let value = match &s.value {
                ValueSchema::Boolean => ParameterValue::Boolean(false),
                ValueSchema::Quantity(range) => ParameterValue::Quantity(
                    FiniteQuantity::new(
                        if r.id == amount_slot {
                            amount
                        } else if range.minimum.unit() == &def(1) {
                            1.
                        } else {
                            0.
                        },
                        range.minimum.unit().clone(),
                    )
                    .unwrap(),
                ),
                other => panic!("unhandled canonical input {other:?}"),
            };
            Some(ParameterAssignment {
                slot: r.id.clone(),
                value,
            })
        })
        .collect();
    let mut a = InstanceAllocator::from_state(i.build.allocator);
    let id = a.allocate().unwrap();
    i.build.allocator = a.state();
    let item = i.build.items.iter_mut().find(|x| x.id == item_id).unwrap();
    item.modifier_order.push(id);
    item.modifiers.push(RolledModifier {
        id,
        definition: def(modifier),
        rolls,
    });
}
pub fn value(report: &SupportEffectsReport, i: &ReplayInput) -> EffectValue {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("unavailable: {:?}", report.gaps)
    };
    let equipment = i
        .build
        .equipment
        .iter()
        .find(|e| e.destination == EquipmentDestination::CharacterSlot(def(0x66)))
        .unwrap()
        .id;
    effects
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::EquipmentUse(equipment),
                    stat: def(family::FINAL),
                }
        })
        .unwrap()
        .value
        .clone()
}
fn evaluate(i: &ReplayInput) -> SupportEffectsReport {
    let p = i.compile().unwrap_or_else(|e| {
        let definitions = poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage::new(
            i.schema.clone(),
            Default::default(),
        )
        .unwrap();
        let request = OwnedEvaluationRequest::new(
            BuildSpec::new(i.build.clone(), Default::default()).unwrap(),
            ScenarioSpec::new(i.scenario.clone(), Default::default()).unwrap(),
            QuerySpec::new(i.queries.clone(), Default::default()).unwrap(),
            Default::default(),
        )
        .unwrap();
        let report = poe_optimizer_core::owned_binding::bind_owned_request(
            &definitions,
            &request,
            Default::default(),
        )
        .unwrap();
        panic!(
            "{e}; first binding issues: {:?}",
            report.issues().iter().take(8).collect::<Vec<_>>()
        );
    });
    p.evaluate(&mut p.new_scratch()).unwrap()
}

fn potential(
    i: &mut ReplayInput,
    query: &str,
    amount: f64,
    admitted: bool,
    active: bool,
    stage: &str,
) {
    let query = i
        .rules
        .contribution_queries
        .as_ref()
        .unwrap()
        .members
        .iter()
        .find(|q| q.id.as_str() == query)
        .unwrap()
        .clone();
    let owner = subject(def::<ItemTemplateDefinition>(0x1f1c));
    let p = RuleProgram {
        id: key("synthetic.item-evasion-potential"),
        context: RuleEntityKind::EquipmentUse,
        reads: vec![],
        nodes: vec![
            RuleNode {
                id: key("value"),
                expression: RuleExpression::Literal {
                    value: quantity(
                        amount,
                        if query.stat == def(0x3313) {
                            2
                        } else {
                            family::UNIT
                        },
                    ),
                },
            },
            RuleNode {
                id: key("active"),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Boolean(active),
                },
            },
        ],
        effects: vec![RuleEffect {
            id: key("potential"),
            when: Some(key("active")),
            effect: RuleEffectKind::Contribute {
                entity: RuleEntity::Current,
                stat: query.stat.clone(),
                contribution: query.contribution,
                value: key("value"),
            },
        }],
    };
    declare(i, owner.clone(), p, stage);
    if admitted {
        let group = &mut i
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members
            .iter_mut()
            .find(|q| q.id == query.id)
            .unwrap()
            .groups[0];
        group.members.members.push(ContributionMember {
            producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                owner,
                program: key("synthetic.item-evasion-potential"),
                effect: key("potential"),
                origin: ContributionOrigin::EquipmentUse {
                    slots: vec![def(0x66)],
                },
            }),
            order: if group.ordering == ContributionOrdering::Ordered {
                Some(ContributionOrder {
                    source_rank: 1,
                    program_rank: 0,
                    effect_rank: 0,
                    slot_ranks: vec![ContributionSlotRank {
                        slot: def(0x66),
                        rank: 0,
                    }],
                })
            } else {
                None
            },
        });
    }
}
#[test]
fn zero_inactive_unknown_and_late_suppliers_cannot_escape_checked_membership() {
    for active in [false, true] {
        let mut i = fixture(0x1f1c);
        potential(
            &mut i,
            "item-evasion-single-base",
            0.,
            false,
            active,
            "item-evasion-delivery",
        );
        let error = i
            .rebind_test_edit()
            .err()
            .or_else(|| i.compile().err())
            .expect("unlisted writer must fail");
        assert!(error.contains("membership"), "{error}");
    }
    let mut i = fixture(0x1f1c);
    potential(
        &mut i,
        "item-evasion-per-level",
        0.,
        true,
        false,
        "item-evasion-final",
    );
    let error = i
        .rebind_test_edit()
        .err()
        .or_else(|| i.compile().err())
        .expect("late writer must fail");
    assert!(error.contains("frozen"), "{error}");
    let mut i = fixture(0x1f1c);
    let q = i
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id.as_str() == "item-evasion-single-base")
        .unwrap();
    q.groups[0].members.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: subject(def::<StatDefinition>(family::LOCAL)),
            facet: SchemaFacet::GameRules,
            code: key("synthetic.unknown-local-members"),
        }],
    };
    i.rebind_test_edit().unwrap();
    let report = evaluate(&i);
    assert!(matches!(
        report.outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    assert!(format!("{:?}", report.gaps).contains("IncompleteContributors"));
}
#[test]
fn present_zero_replacement_and_positive_per_level_do_not_become_final_values() {
    for (query, amount) in [
        ("item-evasion-replacement", 0.),
        ("item-evasion-per-level", 1.),
        ("item-evasion-crafted-quality", 1.),
    ] {
        let mut i = fixture(0x1f1c);
        potential(&mut i, query, amount, true, true, "item-evasion-delivery");
        i.rebind_test_edit().unwrap();
        // Demand the final property through a separate downstream rule. Inactive
        // is a nonnumeric result, never an implicit empty-slot zero.
        let downstream: StatDefId = DefId::parse(
            def::<StatDefinition>(family::FINAL).namespace().clone(),
            "synthetic.demanded-item-evasion",
        )
        .unwrap();
        i.schema.definitions.push(decode(&json!({"kind":"stat","value":{"id":downstream,"schema":{"kind":"known","value":{"value":{"kind":"quantity","value":{"unit":def::<UnitDefinition>(family::UNIT)}},"targets":["equipment_use"]}}}})));
        i.stages.stages.push(EvaluationStage {
            id: key("item-evasion-demand"),
            predecessors: vec![key("item-evasion-final")],
        });
        declare(
            &mut i,
            subject(def::<ItemTemplateDefinition>(0x1f1c)),
            RuleProgram {
                id: key("synthetic.demand-final-item-evasion"),
                context: RuleEntityKind::EquipmentUse,
                reads: vec![RuleRead {
                    id: key("final"),
                    value_type: ComputedValueType::Quantity {
                        unit: def(family::UNIT),
                    },
                    source: RuleReadSource::Stat {
                        entity: RuleEntity::Current,
                        stat: def(family::FINAL),
                    },
                }],
                nodes: vec![RuleNode {
                    id: key("final"),
                    expression: RuleExpression::Read {
                        input: key("final"),
                    },
                }],
                effects: vec![RuleEffect {
                    id: key("demand"),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: downstream.clone(),
                        value: key("final"),
                    },
                }],
            },
            "item-evasion-demand",
        );
        i.rebind_test_edit().unwrap();
        let report = evaluate(&i);
        let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
            panic!("finite demand must evaluate")
        };
        let demanded = effects
            .values
            .iter()
            .find(|v| matches!(&v.key,PlanValueKey::Stat{stat,..} if *stat==downstream))
            .unwrap();
        assert!(
            !matches!(demanded.value, EffectValue::Known { .. }),
            "demanded unsupported property became a number"
        );
        let v = value(&report, &i);
        assert!(!matches!(v, EffectValue::Known { .. }), "{query}: {v:?}");
        if query == "item-evasion-replacement" {
            assert_eq!(v, EffectValue::Inactive);
        }
    }
}
#[test]
fn source_partial_modifier_coverage_refuses_unproved_global_delivery() {
    let mut i = fixture(0x1f1c);
    add_modifier(&mut i, 0x2a63, 13.);
    let d: Value = family::read("dependencies.json");
    let original: DefinitionRules = decode(
        d["owners"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["owner"] == json!(subject(def::<ModifierDefinition>(0x2a63))))
            .unwrap(),
    );
    i.rules
        .owners
        .iter_mut()
        .find(|o| o.owner == original.owner)
        .unwrap()
        .programs
        .closure = original.programs.closure;
    let error = i
        .rebind_test_edit()
        .err()
        .or_else(|| i.compile().err())
        .expect("original Partial modifier owner must remain unavailable");
    assert!(error.contains("complete owner"), "{error}");
}
#[test]
fn local_rounding_has_explicit_signed_domain_and_does_not_clamp() {
    let original = fixture(0x1f1c);
    for (x, expected) in [
        (-7.5, Some(-7.)),
        (-0.5000000000000001, Some(-1.)),
        (-0.5, Some(0.)),
        (-0.49999999999999994, Some(0.)),
        (0.4999999999999999, Some(0.)),
        (0.49999999999999994, None),
        (0.5, Some(1.)),
        (4503599627370496., Some(4503599627370496.)),
        (-4503599627370496., Some(-4503599627370496.)),
        (4503599627370497., None),
        (-4503599627370497., None),
    ] {
        let mut i = original.clone();
        let item = selected_item(&i);
        i.build
            .items
            .iter_mut()
            .find(|r| r.id == item)
            .unwrap()
            .quality
            .as_mut()
            .unwrap()
            .amount = FiniteQuantity::new(0., def(2)).unwrap();
        let p = i
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == subject(def::<ItemTemplateDefinition>(0x1f1c)))
            .unwrap()
            .programs
            .members
            .iter_mut()
            .find(|p| p.id.as_str() == "raw-base-defence-profile")
            .unwrap();
        let output = p
            .effects
            .iter()
            .find_map(|e| {
                if let RuleEffectKind::Derive { stat, value, .. } = &e.effect {
                    (stat == &def(0x29fe)).then(|| value.clone())
                } else {
                    None
                }
            })
            .unwrap();
        p.nodes
            .iter_mut()
            .find(|n| n.id == output)
            .unwrap()
            .expression = RuleExpression::Literal {
            value: quantity(x, family::UNIT),
        };
        i.rebind_test_edit().unwrap();
        let v = value(&evaluate(&i), &i);
        if let Some(expected) = expected {
            assert_eq!(
                v,
                EffectValue::Known {
                    value: quantity(expected, family::UNIT)
                },
                "operand{x}"
            )
        } else {
            assert!(format!("{v:?}").contains("UnsupportedDomain"), "{x}: {v:?}")
        }
    }
}
fn known(i: &ReplayInput, expected: f64) {
    assert_eq!(
        value(&evaluate(i), i),
        EffectValue::Known {
            value: quantity(expected, family::UNIT)
        }
    );
}

#[test]
fn real_profiles_and_quality_reach_the_final_equipment_property() {
    for (template, expected) in [(0x1f1c, 0.), (0x1e0e, 0.), (0x1fde, 59.)] {
        let i = fixture(template);
        known(&i, expected);
    }
    let mut i = fixture(0x1fde);
    let id = selected_item(&i);
    i.build
        .items
        .iter_mut()
        .find(|x| x.id == id)
        .unwrap()
        .quality
        .as_mut()
        .unwrap()
        .amount = FiniteQuantity::new(0., def(2)).unwrap();
    i.rebind_test_edit().unwrap();
    known(&i, 49.);
}
#[test]
fn actual_canonical_modifier_occurrences_reproduce_hybrid_source_75() {
    let mut i = fixture(0x1f1c);
    for (m, n) in [
        (0x2a63, 13.),
        (0x2a97, 17.),
        (0x2b45, 31.),
        (0x2b77, 37.),
        (0x2ba9, 41.),
    ] {
        add_modifier(&mut i, m, n);
    }
    i.rebind_test_edit().unwrap();
    known(&i, 75.);
    let id = selected_item(&i);
    let item = i.build.items.iter_mut().find(|x| x.id == id).unwrap();
    item.modifiers.reverse();
    i.rebind_test_edit().unwrap();
    known(&i, 75.);
    let p = Arc::new(i.compile().unwrap());
    let expected = p.evaluate(&mut p.new_scratch()).unwrap();
    let mut scratch = p.new_scratch();
    assert_eq!(p.evaluate(&mut scratch).unwrap(), expected);
    assert_eq!(p.evaluate(&mut scratch).unwrap(), expected);
    let parallel: Vec<_> = (0..8)
        .into_par_iter()
        .map(|_| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    assert!(parallel.iter().all(|r| r == &expected));
}
