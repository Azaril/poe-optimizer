//! Real source-defined Attack-flat rules in an explicitly finite equipment
//! world. The template admission and closure of other flat families are test
//! assumptions; neither is published or inferred from a measured zero.
use super::*;
use poe_optimizer_core::build_identity::{InstanceAllocator, ModifierInstanceId};

#[path = "../../../../tests/support/owned_minion_attack_flat_modifier_evidence.rs"]
mod source_evidence;

const GRANT: &str = "grant-minion-attack-flat";
const CONTRIBUTE: &str = "minion-attack-flat-contributors";
const RECEIVE: &str = "minion-attack-flat-receivers";
const MODIFIERS: [u64; 2] = [0x3377, 0x337a];

fn data(file: &str) -> Value {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/self-flat-attack-damage")
                .join(file),
        )
        .unwrap(),
    )
    .unwrap()
}
fn authored_queries() -> Vec<ContributionQuery> {
    serde_json::from_value(data("queries.json")).unwrap()
}
fn is_new_query(q: &ContributionQuery) -> bool {
    q.stat == def(0x3373) || q.stat == def(0x3374)
}
fn known(n: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, def(0x1d3a)).unwrap())
}
fn install_owner(i: &mut ReplayInput, owner: DefinitionRules, stage: &str) {
    for p in &owner.programs.members {
        i.stages.programs.members.push(StagedRuleProgram {
            owner: owner.owner.clone(),
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
                owner: owner.owner.clone(),
                program: p.id.clone(),
                phase: ReadinessPhase::Execution,
                role: ReadinessProgramRole::Execution,
                outputs: vec![],
            });
    }
    i.rules
        .support_discovery
        .as_mut()
        .unwrap()
        .providers
        .push(SupportSourceDomainDeclaration {
            owner: owner.owner.clone(),
            domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
        });
    i.rules.owners.push(owner);
}
fn fixture() -> ReplayInput {
    let mut i = composed(false);
    let selection: Value = serde_json::from_slice(&authenticated(
        &packet("bindings.json")["source_selection_prerequisite"]["evidence"],
    ))
    .unwrap();
    assert_eq!(
        selection["catalog"]["selected"]["basic_id"],
        "MinionMeleeBow"
    );
    assert_eq!(
        selection["catalog"]["selected"]["basic_stat_sets"]["1"]["baseFlags"]["attack"], true,
        "the exact routed Basic stat set is an Attack"
    );
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(data("definitions.json")).unwrap();
    let owners: Vec<DefinitionRules> = serde_json::from_value(data("programs.json")).unwrap();
    for definition in definitions {
        let subject = SchemaSubject::Definition(definition.address());
        if !owners.iter().any(|o| o.owner == subject) {
            install_owner(
                &mut i,
                DefinitionRules {
                    owner: subject,
                    programs: DeclaredSet::complete(vec![]),
                },
                CONTRIBUTE,
            );
        }
        i.schema.definitions.push(definition);
    }
    i.schema
        .slots
        .extend(serde_json::from_value::<Vec<SlotDescriptor>>(data("slots.json")).unwrap());
    i.stages.stages.extend([
        EvaluationStage {
            id: key(CONTRIBUTE),
            predecessors: vec![key("prepare")],
        },
        EvaluationStage {
            id: key(RECEIVE),
            predecessors: vec![key(CONTRIBUTE), key("population")],
        },
    ]);
    i.stages
        .stages
        .iter_mut()
        .find(|s| s.id == i.stages.routing_stage)
        .unwrap()
        .predecessors
        .push(key(RECEIVE));
    for owner in owners {
        let stage = if matches!(
            owner.owner,
            SchemaSubject::Definition(DefinitionAddress::Modifier(_))
        ) {
            CONTRIBUTE
        } else {
            RECEIVE
        };
        install_owner(&mut i, owner, stage);
    }
    let mut queries = authored_queries();
    for q in &mut queries {
        assert!(!q.groups[0].members.is_complete());
        // The compact replay omits unused equipment definitions. Restrict its
        // explicit origin census to the slots it actually declares, retaining
        // each surviving authored rank rather than deriving an iteration order.
        for member in &mut q.groups[0].members.members {
            let producer = member.producer.as_program_effect_mut().unwrap();
            let ContributionOrigin::ItemModifier { slots } = &mut producer.origin else {
                panic!()
            };
            slots.retain(|slot| {
                i.schema
                    .definitions
                    .iter()
                    .any(|d| d.address() == slot.address())
            });
            member
                .order
                .as_mut()
                .unwrap()
                .slot_ranks
                .retain(|r| slots.contains(&r.slot));
        }
        // Only this controlled finite world has the two enumerated supplier
        // definitions and no other flat damage families. Production stays Partial.
        q.groups[0].members = DeclaredSet::complete(q.groups[0].members.members.clone());
    }
    i.rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(queries);
    i.rules
        .receivers
        .members
        .extend(serde_json::from_value::<Vec<StatReceiver>>(data("receivers.json")).unwrap());
    let routes: ActionOutputRoutes = serde_json::from_value(data("routes.json")).unwrap();
    assert!(!routes.routes.is_complete());
    i.routing
        .outputs
        .iter_mut()
        .find(|r| r.output == routes.output)
        .unwrap()
        .routes
        .members
        .extend(routes.routes.members);
    for n in 0..2 {
        i.stages.frozen_channels.extend([
            FrozenStageChannel {
                channel: StageChannel::Contributions {
                    scope: RuleEntityKind::Actor,
                    stat: def(0x3373 + n),
                    contribution: ContributionKind::Add,
                },
                stage: key(CONTRIBUTE),
            },
            FrozenStageChannel {
                channel: StageChannel::Stat {
                    scope: RuleEntityKind::Actor,
                    stat: def(0x3375 + n),
                },
                stage: key(RECEIVE),
            },
        ]);
        i.stages
            .frozen_channels
            .iter_mut()
            .find(|f| {
                f.channel
                    == StageChannel::Stat {
                        scope: RuleEntityKind::Action,
                        stat: def(0x3371 + n),
                    }
            })
            .unwrap()
            .stage = i.stages.routing_stage.clone();
    }
    i.rebind_test_edit().unwrap();
    i
}
fn synthetic_item_admission(
    i: &mut ReplayInput,
    rolls: &[(usize, [f64; 2])],
) -> Vec<ModifierInstanceId> {
    // This exact existing item is a finite test carrier. No source-defined unique
    // item/template association or obtainable combination has been proved.
    let use_ = i
        .build
        .equipment
        .iter()
        .find(|e| e.destination == EquipmentDestination::CharacterSlot(def(0x66)))
        .unwrap();
    let item_id = use_.item;
    let item = i.build.items.iter_mut().find(|t| t.id == item_id).unwrap();
    let template = item.template.clone();
    let DefinitionDescriptor::ItemTemplate(entry) = i
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == template.address())
        .unwrap()
    else {
        panic!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        panic!()
    };
    schema
        .modifiers
        .members
        .extend(MODIFIERS.map(def::<ModifierDefinition>));
    let mut allocator = InstanceAllocator::from_state(i.build.allocator);
    let mut ids = vec![];
    for (family, amounts) in rolls {
        let n = MODIFIERS[*family];
        let id = allocator.allocate().unwrap();
        ids.push(id);
        item.modifiers.push(RolledModifier {
            id,
            definition: def(n),
            rolls: amounts
                .iter()
                .enumerate()
                .map(|(endpoint, amount)| ParameterAssignment {
                    slot: DeclaredSlot {
                        declaration: SlotOwnerDefId::Modifier(def(n)),
                        slot: def(n + 1 + endpoint as u64),
                    },
                    value: known(*amount),
                })
                .collect(),
        });
        item.modifier_order.push(id);
    }
    i.build.allocator = allocator.state();
    i.rebind_test_edit().unwrap();
    ids
}
fn with_rolls(rolls: &[(usize, [f64; 2])]) -> ReplayInput {
    let mut i = fixture();
    synthetic_item_admission(&mut i, rolls);
    i
}
fn synthetic_ranges(i: &mut ReplayInput) {
    for slot in &mut i.schema.slots {
        let SlotDescriptor::Parameter(entry) = slot else {
            continue;
        };
        if !MODIFIERS
            .iter()
            .any(|n| entry.id.declaration == SlotOwnerDefId::Modifier(def(*n)))
        {
            continue;
        }
        let SchemaState::Known(schema) = &mut entry.schema else {
            panic!()
        };
        schema.value = ValueSchema::Quantity(QuantityRange {
            minimum: FiniteQuantity::new(-1e17, def(0x1d3a)).unwrap(),
            maximum: FiniteQuantity::new(1e17, def(0x1d3a)).unwrap(),
        });
    }
    i.rebind_test_edit().unwrap();
}
fn contributions(r: &SupportEffectsReport) -> Vec<&BoundEffectResult> {
    effects(r).effects.iter().filter(|e| matches!(&e.target,
        BoundEffectTarget::Contribution {key} if key.stat == def(0x3373) || key.stat == def(0x3374)
    )).collect()
}
fn check_path(i: &ReplayInput, r: &SupportEffectsReport, expected: [f64; 2], count: usize) {
    assert_eq!(contributions(r).len(), count * 2);
    for row in contributions(r) {
        assert_eq!(row.key.invocation.program, key(GRANT));
        let RuleOrigin::Provider { provider } = &row.key.invocation.origin else {
            panic!()
        };
        assert!(provider.grant_path.is_empty());
        let ProviderRoot::ItemModifier {
            equipment_use,
            modifier,
        } = provider.root
        else {
            panic!()
        };
        let use_ = i
            .build
            .equipment
            .iter()
            .find(|e| e.id == equipment_use)
            .unwrap();
        let item = i.build.items.iter().find(|t| t.id == use_.item).unwrap();
        let source = item.modifiers.iter().find(|m| m.id == modifier).unwrap();
        assert_eq!(
            row.key.invocation.owner,
            SchemaSubject::Definition(source.definition.address())
        );
        let BoundEffectTarget::Contribution { key } = &row.target else {
            unreachable!()
        };
        assert_eq!(key.entity, ConcreteEntity::Actor(ActorKey::Player));
    }
    for actor in actors(i) {
        for (endpoint, amount) in expected.into_iter().enumerate() {
            assert_eq!(
                value(r, &actor, 0x3375 + endpoint as u64),
                &EffectValue::Known {
                    value: known(amount)
                }
            );
        }
    }
    let exact: Vec<_> = actions(i)
        .into_iter()
        .filter(|a| a.action.output.slot == def(0x22))
        .collect();
    assert_eq!(exact.len(), 2);
    for a in exact {
        for (endpoint, amount) in expected.into_iter().enumerate() {
            assert_eq!(number(r, a, 0x3371 + endpoint as u64), amount);
            let intrinsic = number(r, a, 0x3212 + endpoint as u64);
            assert_eq!(
                number(r, a, 0x336f + endpoint as u64).to_bits(),
                (intrinsic + amount * 1.15).to_bits()
            );
        }
    }
    // Gas and the Player never receive the minion's Attack subtotal/final result.
    for a in actions(i)
        .into_iter()
        .filter(|a| a.action.output.slot != def(0x22))
    {
        for n in [0x3371, 0x3372, 0x336f, 0x3370] {
            assert!(!effects(r).values.iter().any(|v| v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Action(Box::new(a.clone())),
                    stat: def(n),
                }));
        }
    }
    assert!(
        !i.rules
            .owners
            .iter()
            .flat_map(|o| &o.programs.members)
            .any(|p| p.id == key(OPERANDS))
    );
}

#[test]
fn declared_item_ranges_compose_through_exact_origins_and_basic_attack_only() {
    let binding = data("bindings.json");
    let proof: Value =
        serde_json::from_slice(&authenticated(&binding["source_proof"]["evidence"])).unwrap();
    authenticated(&binding["source_proof"]["review"]);
    source_evidence::check(&proof);
    let mut compared = 0;
    for row in proof["cases"].as_array().unwrap() {
        let input = &row["input"];
        let id = input["id"].as_str().unwrap();
        let rolls = if input["admitted_roll"] == true {
            let family = binding["families"]
                .as_array()
                .unwrap()
                .iter()
                .position(|f| f["source"] == input["family"])
                .unwrap();
            vec![(
                family,
                [
                    input["minimum"].as_f64().unwrap(),
                    input["maximum"].as_f64().unwrap(),
                ],
            )]
        } else if id == "empty" {
            vec![]
        } else if id == "duplicate" {
            vec![(0, [7., 11.]), (0, [7., 11.])]
        } else {
            continue; // Conditions and out-of-range zero are not this native family.
        };
        let original = &row["result"]["inherited"];
        let expected = [
            original["minimum"].as_f64().unwrap(),
            original["maximum"].as_f64().unwrap(),
        ];
        assert_eq!(
            row["result"]["non_attack_inherited"]["minimum"].as_f64(),
            Some(0.)
        );
        assert_eq!(
            row["result"]["non_attack_inherited"]["maximum"].as_f64(),
            Some(0.)
        );
        let i = with_rolls(&rolls);
        check_path(&i, &run(&i), expected, rolls.len());
        compared += 1;
    }
    assert_eq!(compared, 12, "ten source range cases, empty and duplicate");
    let mixed = with_rolls(&[(0, [5., 10.]), (1, [8., 16.])]);
    check_path(&mixed, &run(&mixed), [13., 26.], 2);
    for (family, amounts) in [
        (0, [4., 11.]),
        (0, [7., 13.]),
        (1, [9., 14.]),
        (1, [7., 17.]),
    ] {
        let mut i = fixture();
        synthetic_item_admission(&mut i, &[(family, amounts)]);
        assert!(
            i.compile().is_err(),
            "the declared bounds remain admission requirements"
        );
    }
}

#[test]
fn explicit_synthetic_zero_inactive_and_unqualified_controls_keep_provider_identity() {
    for amounts in [[0., 0.], [3., 7.]] {
        let mut i = fixture();
        synthetic_ranges(&mut i);
        synthetic_item_admission(&mut i, &[(0, amounts)]);
        check_path(&i, &run(&i), amounts, 1);
    }
    let mut i = with_rolls(&[(0, [7., 11.])]);
    let p = i
        .rules
        .owners
        .iter_mut()
        .find(|o| {
            o.owner == SchemaSubject::Definition(def::<ModifierDefinition>(MODIFIERS[0]).address())
        })
        .unwrap()
        .programs
        .members
        .first_mut()
        .unwrap();
    p.nodes.push(RuleNode {
        id: key("synthetic-inactive"),
        expression: RuleExpression::Literal {
            value: ParameterValue::Boolean(false),
        },
    });
    for e in &mut p.effects {
        e.when = Some(key("synthetic-inactive"));
    }
    i.rebind_test_edit().unwrap();
    let r = run(&i);
    check_path(&i, &r, [0., 0.], 1);
    assert!(
        contributions(&r)
            .iter()
            .all(|e| matches!(e.value, EffectValue::Inactive))
    );
    // Inactive and zero-valued producers still require exact membership.
    for mut i in [i, {
        let mut z = fixture();
        synthetic_ranges(&mut z);
        synthetic_item_admission(&mut z, &[(0, [0., 0.])]);
        z
    }] {
        let q = i
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members
            .iter_mut()
            .find(|q| is_new_query(q))
            .unwrap();
        q.groups[0].members.members.remove(0);
        assert!(
            i.rebind_test_edit()
                .and_then(|_| i.compile().map(|_| ()))
                .is_err_and(
                    |error| error.contains("actual contribution has no declared membership")
                ),
            "inactive/zero cannot excuse missing membership"
        );
    }
}

#[test]
fn explicit_modifier_order_is_observed_and_storage_order_is_not() {
    let mut i = fixture();
    synthetic_ranges(&mut i);
    let ids = synthetic_item_admission(
        &mut i,
        &[(0, [1e16, 1e16]), (0, [-1e16, -1e16]), (0, [1., 1.])],
    );
    let first = run(&i);
    check_path(&i, &first, [1., 1.], 3);
    let mut storage = i.clone();
    storage.build.items.reverse();
    storage.build.equipment.reverse();
    for item in &mut storage.build.items {
        item.modifiers.reverse();
    }
    assert_eq!(run(&storage), first);
    let item = i
        .build
        .items
        .iter_mut()
        .find(|t| t.modifier_order.contains(&ids[0]))
        .unwrap();
    let a = item
        .modifier_order
        .iter()
        .position(|id| *id == ids[1])
        .unwrap();
    let b = item
        .modifier_order
        .iter()
        .position(|id| *id == ids[2])
        .unwrap();
    item.modifier_order.swap(a, b);
    check_path(&i, &run(&i), [0., 0.], 3);
}

#[test]
fn partial_membership_missing_rolls_and_early_receipt_are_never_neutral_defaults() {
    let mut partial = fixture();
    let authored = authored_queries();
    for q in &mut partial.rules.contribution_queries.as_mut().unwrap().members {
        if let Some(original) = authored.iter().find(|p| p.id == q.id) {
            q.groups[0].members.closure = original.groups[0].members.closure.clone();
        }
    }
    partial.rebind_test_edit().unwrap();
    let r = run(&partial);
    // Query closure is a plan admission requirement. Incomplete source
    // inventories stop evaluation rather than becoming an empty known total.
    assert!(matches!(
        r.outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    assert_eq!(r.gaps.len(), 2);
    for n in [0x3373, 0x3374] {
        assert!(r.gaps.iter().any(|gap| {
            gap.provider.is_none()
                && gap.subject
                    == Some(SchemaSubject::Definition(
                        def::<StatDefinition>(n).address(),
                    ))
                && gap.reason == PlanGapReason::IncompleteContributors
        }));
    }
    let mut missing = with_rolls(&[(0, [7., 11.])]);
    missing
        .build
        .items
        .iter_mut()
        .flat_map(|t| &mut t.modifiers)
        .find(|m| m.definition == def(MODIFIERS[0]))
        .unwrap()
        .rolls
        .pop();
    assert!(missing.compile().is_err());
    let mut early = fixture();
    early
        .stages
        .programs
        .members
        .iter_mut()
        .find(|p| p.program == key("receive-minion-attack-flat-minimum"))
        .unwrap()
        .stage = key("prepare");
    assert!(
        early
            .rebind_test_edit()
            .unwrap_err()
            .contains("frozen channel read")
    );
    let mut foreign_selection = fixture();
    let route = foreign_selection
        .routing
        .outputs
        .iter_mut()
        .find(|r| r.output.slot == def(0x22))
        .unwrap()
        .routes
        .members
        .iter_mut()
        .find(|r| r.target == def(0x3371))
        .unwrap();
    let ActionRouteSelection::Exact(selection) = &mut route.selection else {
        panic!()
    };
    selection.stat_set = def(0x3283); // Known elsewhere, not admitted by Basic.
    assert!(
        foreign_selection.rebind_test_edit().is_err(),
        "known foreign stat set is not Basic Attack eligibility"
    );
}

#[test]
fn exact_receivers_survive_repeated_actors_fresh_reused_and_rayon_plans() {
    let a = with_rolls(&[(0, [7., 11.])]);
    let mut b = with_rolls(&[(1, [8., 16.])]);
    b.build
        .gems
        .iter_mut()
        .find(|g| g.definition == def(0x11))
        .unwrap()
        .level -= 1;
    let c = fixture();
    let inputs = [&a, &b, &c];
    let plans = inputs.map(|i| i.compile().unwrap());
    let fresh = plans
        .each_ref()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap());
    for ((i, r), (amount, count)) in
        inputs
            .into_iter()
            .zip(&fresh)
            .zip([([7., 11.], 1), ([8., 16.], 1), ([0., 0.], 0)])
    {
        check_path(i, r, amount, count);
    }
    let mut scratch = plans[0].new_scratch();
    for n in [0, 1, 2, 1, 0] {
        assert_eq!(plans[n].evaluate(&mut scratch).unwrap(), fresh[n]);
    }
    let rows = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..12)
                .into_par_iter()
                .map_init(
                    || plans[0].new_scratch(),
                    |s, n| plans[n % 3].evaluate(s).unwrap(),
                )
                .collect::<Vec<_>>()
        });
    for (n, r) in rows.into_iter().enumerate() {
        assert_eq!(r, fresh[n % 3]);
    }
}
