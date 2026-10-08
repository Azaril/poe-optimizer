//! Actual published contribution programs in the existing finite two-Sniper
//! world. Parent final levels and unrelated rule inventories remain explicit
//! component boundaries; no final Damage metric or original-build closure. The
//! separately published population partition supplies checked V20 readiness;
//! the fixture never removes or rewrites its execution requirement.
#[path = "../../crates/poe-optimizer-engine/tests/support/plain_minion_damage_fixture.rs"]
mod fixture;
use super::{family, readiness_family, release};
use fixture::{Node, World, def, intrinsic, known, value};
use poe_optimizer_core::{
    owned_binding::*, owned_build::*, owned_definitions::*, owned_readiness::*, owned_routing::*,
    owned_rules::*, owned_schema::*, owned_stages::*, owned_support_inputs::*,
    owned_support_receiving::*, owned_supports::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting, owned_rules::OwnedRulePackage,
    owned_schema::OwnedDefinitionSchemaPackage, owned_stages::OwnedEvaluationStages,
    owned_support_inputs::OwnedSupportInputBindings,
    owned_support_receiving::OwnedSupportReceiving, owned_supports::OwnedSupportPreparation,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release_migration::OwnedReleaseMigrationInput,
};
use rayon::prelude::*;
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::PathBuf, sync::Arc};

fn action(index: usize, set: Option<ActionStatSetDefId>) -> ActionSelection {
    let mut a = intrinsic::selected(index);
    if let Some(set) = set {
        a.action.provider.grant_path[2] =
            intrinsic::slot(SlotOwnerDefId::Actor(def(0x3091)), 0x3095);
        a.action.output = intrinsic::slot(SlotOwnerDefId::Skill(def(0x24)), 0x25);
        a.stat_set = set;
    }
    a
}
fn actions(index: usize) -> Vec<ActionSelection> {
    std::iter::once(action(index, None))
        .chain([0x32ec, 0x32ed, 0x32ee].map(|n| action(index, Some(def(n)))))
        .collect()
}
fn world() -> World {
    let path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_COMMAND_DAMAGE_RELEASE")
            .expect("actual published Command Damage endpoint"),
    );
    let endpoint = release::load(&path);
    family::assert_endpoint(&endpoint);
    let readiness_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_COMMAND_DAMAGE_READINESS_RELEASE")
            .expect("actual published Sniper readiness successor"),
    );
    let readiness_endpoint = release::load(&readiness_path);
    readiness_family::assert_endpoint(&readiness_endpoint);
    let readiness_bindings: Value = readiness_family::read("bindings.json");
    assert_eq!(
        readiness_bindings["before"],
        json!(endpoint.receipt().input)
    );
    let b: Value = family::read("bindings.json");
    let d: Value = family::read("dependencies.json");
    let c: Value = family::read("closure.json");
    let declarations: Vec<DefinitionDescriptor> =
        serde_json::from_value(c["definitions"].clone()).unwrap();
    let nodes: Vec<Node> = b["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .zip(&declarations)
        .map(|(n, row)| {
            let DefinitionDescriptor::PassiveNode(row) = row else {
                panic!("passive")
            };
            let SchemaState::Known(schema) = &row.schema else {
                panic!("known passive")
            };
            assert_eq!(schema.pools.members.len(), 1);
            Node {
                node: row.id.clone(),
                source_id: n["source_id"].as_str().unwrap().into(),
                value: 0.0,
                pool: schema.pools.members[0].clone(),
            }
        })
        .collect();
    let mut world = World::new(&nodes);
    let f = &mut world.intrinsic.f;
    f.schema.schema_version = endpoint.input().recipe.schema.schema_version;
    // Consume the independently published rule partition, not a fixture rewrite.
    // Only the finite owner closure is test-owned; both split bodies are exact.
    let population_owner = SchemaSubject::Definition(DefinitionAddress::Skill(def(0x12)));
    let old_owner = endpoint
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|owner| owner.owner == population_owner)
        .unwrap();
    let new_owner = readiness_endpoint
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|owner| owner.owner == population_owner)
        .unwrap();
    assert!(!new_owner.programs.is_complete());
    assert_eq!(new_owner.programs.closure, old_owner.programs.closure);
    let partition: Value = readiness_family::read("partition.json");
    let original: RuleProgram = serde_json::from_value(partition["original"].clone()).unwrap();
    let owner = f.owner_mut(&population_owner);
    let prior_program = owner
        .programs
        .members
        .iter_mut()
        .find(|program| program.id.as_str() == "ordinary-population-inputs")
        .unwrap();
    assert_eq!(&*prior_program, &original);
    assert_eq!(
        old_owner
            .programs
            .members
            .iter()
            .find(|program| program.id == original.id),
        Some(&original)
    );
    *prior_program = new_owner
        .programs
        .members
        .iter()
        .find(|program| program.id == original.id)
        .unwrap()
        .clone();
    assert!(
        !owner
            .programs
            .members
            .iter()
            .any(|program| program.id.as_str() == "ordinary-population-requirements")
    );
    owner.programs.members.push(
        new_owner
            .programs
            .members
            .iter()
            .find(|program| program.id.as_str() == "ordinary-population-requirements")
            .unwrap()
            .clone(),
    );
    for row in declarations {
        assert!(endpoint.input().recipe.schema.definitions.contains(&row));
        // This only removes graph adjacency in the inherited finite test world;
        // publication checks retain every real topology/declaration field.
        let row = fixture::finite_node(&row);
        assert!(
            !f.schema
                .definitions
                .iter()
                .any(|r| r.address() == row.address())
        );
        f.schema.definitions.push(row);
    }
    let m: OwnedReleaseMigrationInput = family::read("migration.json");
    for entry in m.schema {
        let SchemaExtensionEntry::Definition(row) = entry else {
            panic!("only new stats")
        };
        assert!(
            !f.schema
                .definitions
                .iter()
                .any(|r| r.address() == row.address())
        );
        f.schema.definitions.push(row);
    }
    let supporting: Vec<DefinitionDescriptor> =
        serde_json::from_value(d["supporting_definitions"].clone()).unwrap();
    for row in supporting.into_iter().filter(|r| {
        matches!(
            r,
            DefinitionDescriptor::ActionStatSet(_) | DefinitionDescriptor::Stat(_)
        )
    }) {
        assert!(endpoint.input().recipe.schema.definitions.contains(&row));
        if let Some(old) = f
            .schema
            .definitions
            .iter()
            .find(|r| r.address() == row.address())
        {
            assert_eq!(old, &row)
        } else {
            f.schema.definitions.push(row)
        }
    }
    let slots: Vec<SlotDescriptor> = serde_json::from_value(d["slots"].clone()).unwrap();
    let gas = slots
        .into_iter()
        .find(|s| {
            s.address() == SlotAddress::ActionOutput(action(0, Some(def(0x32ec))).action.output)
        })
        .unwrap();
    assert!(endpoint.input().recipe.schema.slots.contains(&gas));
    let gas_address = gas.address();
    *f.schema
        .slots
        .iter_mut()
        .find(|s| s.address() == gas_address)
        .unwrap() = gas;
    let closed: Vec<DefinitionRules> = serde_json::from_value(c["owners"].clone()).unwrap();
    for row in closed {
        assert!(row.programs.is_complete());
        assert!(!f.owners.iter().any(|o| o.owner == row.owner));
        f.owners.push(row);
    }
    for row in m.owners {
        if matches!(row.owner, SchemaSubject::Slot(_)) {
            let destination = f.owner_mut(&row.owner);
            // Only this family's exact appended body and the already-published
            // eligibility fact join the finite intrinsic Action inventory.
            // The actual Action owners remain Partial in the release.
            assert!(!row.programs.is_complete());
            for id in ["commandable-action-fact", "conditional-command-damage"] {
                let program = row
                    .programs
                    .members
                    .iter()
                    .find(|p| p.id.as_str() == id)
                    .unwrap()
                    .clone();
                assert!(
                    !destination
                        .programs
                        .members
                        .iter()
                        .any(|p| p.id == program.id)
                );
                destination.programs.members.push(program);
            }
        } else {
            assert!(row.programs.is_complete());
            f.owners.push(row);
        }
    }
    f.receivers.members.extend(m.receivers);
    let metric = f.queries.requests[0].metric.clone();
    for index in 0..2 {
        for (i, set) in [0x32ec, 0x32ed, 0x32ee].into_iter().enumerate() {
            f.queries.requests.push(MetricRequest {
                id: QueryId::new(format!("command-damage-{index}-{i}")).unwrap(),
                metric: metric.clone(),
                target: MetricTarget::Action(Box::new(action(index, Some(def(set))))),
            });
        }
    }
    assert_eq!(f.build.skills.len(), 2);
    assert!(
        f.build
            .skills
            .iter()
            .all(|s| matches!(s.source, AuthoredSkillSource::Gem(_)))
    );
    assert_eq!(f.queries.requests.len(), 10);
    world
}
type Plan = OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>;

fn unused_input(name: &str) -> StatDefId {
    DefId::parse(intrinsic::ns(), format!("fixture.command-damage.{name}")).unwrap()
}
fn readiness_role(program: &RuleProgram) -> ReadinessProgramRole {
    match program.id.as_str() {
        "primary-supply"
        | "fixture-explicit-final-level"
        | "basic-attack-supply"
        | "gas-arrow-supply" => ReadinessProgramRole::FinalInputAssembly,
        "ordinary-population-inputs" => ReadinessProgramRole::PreparationFacts,
        _ => ReadinessProgramRole::Execution,
    }
}
fn readiness_outputs(program: &RuleProgram) -> Vec<StageChannel> {
    program
        .effects
        .iter()
        .filter_map(|effect| match &effect.effect {
            RuleEffectKind::ActivateGrant { slot, .. } => {
                Some(StageChannel::Grant { slot: slot.clone() })
            }
            RuleEffectKind::ProjectActorStat { stat, .. } => Some(StageChannel::Stat {
                scope: RuleEntityKind::Actor,
                stat: stat.clone(),
            }),
            RuleEffectKind::ProjectSkillParameter { parameter, .. } => {
                Some(StageChannel::SkillParameter {
                    parameter: parameter.clone(),
                })
            }
            RuleEffectKind::Requirement { .. } => None,
            _ => panic!("unexpected early fixture output"),
        })
        .collect()
}
fn compile(world: &World) -> Plan {
    let f = &world.intrinsic.f;
    // The public V20 effect path carries checked readiness and support packages.
    // This component has no support assignments. The three unused typed channels
    // only satisfy that empty package's schema; no values/defaults are produced.
    assert!(f.build.supports.is_empty());
    let mut schema_input = f.schema.clone();
    let unused = [
        (
            unused_input("support-level"),
            RuleEntityKind::SupportOrigin,
            ComputedValueType::Integer,
        ),
        (
            unused_input("support-quality"),
            RuleEntityKind::SupportOrigin,
            ComputedValueType::Quantity { unit: def(2) },
        ),
        (
            unused_input("target-presence"),
            RuleEntityKind::Skill,
            ComputedValueType::Boolean,
        ),
    ];
    for (id, scope, value) in &unused {
        assert!(
            !schema_input
                .definitions
                .iter()
                .any(|row| row.address() == id.address())
        );
        schema_input
            .definitions
            .push(DefinitionDescriptor::Stat(DefinitionEntry {
                id: id.clone(),
                schema: SchemaState::Known(StatSchema {
                    value: value.clone(),
                    targets: vec![*scope],
                }),
            }));
    }
    let schema =
        Arc::new(OwnedDefinitionSchemaPackage::new(schema_input, Default::default()).unwrap());
    let m: OwnedReleaseMigrationInput = family::read("migration.json");
    let rules = RulePackageInput {
        support_discovery: Some(SupportDiscoveryInput {
            providers: f
                .owners
                .iter()
                .map(|row| SupportSourceDomainDeclaration {
                    owner: row.owner.clone(),
                    domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
                })
                .collect(),
        }),
        existing_actor_rules: None,
        contribution_queries: None,
        // V20 requires an explicit inventory. This finite contribution-only
        // component admits no effect applications; production stays unchanged.
        effect_applications: Some(DeclaredSet::complete(vec![])),
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: intrinsic::ns(),
        release: intrinsic::key("command-damage-component"),
        semantics_version: intrinsic::key("finite-test"),
        operations_version: m.contract.operations_version,
        definitions: schema.identity().clone(),
        owners: f.owners.clone(),
        tables: f.tables.clone(),
        receivers: f.receivers.clone(),
    };
    let stored = OwnedRulePackage::new(rules, schema.as_ref(), Default::default()).unwrap();
    let rules = Arc::new(
        CompiledRulePackage::compile_stored(&stored, schema.as_ref(), Default::default()).unwrap(),
    );
    let routes = Arc::new(
        OwnedActionRouting::new(
            ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: intrinsic::ns(),
                release: intrinsic::key("finite-routes"),
                definitions: schema.identity().clone(),
                outputs: f.routes.clone(),
            },
            schema.as_ref(),
            Default::default(),
        )
        .unwrap(),
    );
    let prepare = intrinsic::key("prepare");
    let execute = intrinsic::key("execute");
    let mut program_stages = Vec::new();
    let mut program_readiness = Vec::new();
    for owner in &stored.input().owners {
        for program in &owner.programs.members {
            let role = readiness_role(program);
            let early = role != ReadinessProgramRole::Execution;
            program_stages.push(StagedRuleProgram {
                owner: owner.owner.clone(),
                program: program.id.clone(),
                stage: if early {
                    prepare.clone()
                } else {
                    execute.clone()
                },
            });
            program_readiness.push(ReadinessProgram {
                owner: owner.owner.clone(),
                program: program.id.clone(),
                phase: if early {
                    ReadinessPhase::Preparation
                } else {
                    ReadinessPhase::Execution
                },
                role,
                outputs: if early {
                    readiness_outputs(program)
                } else {
                    vec![]
                },
            });
        }
    }
    let skills = [0x12, 0x21, 0x24]
        .into_iter()
        .map(|id| {
            let skill: SkillDefId = def(id);
            let SchemaLookup::Known(declaration) = schema.definition(&skill) else {
                panic!("fixture generated Skill must be known")
            };
            assert!(declaration.declarations.parameters.is_complete());
            let mut parameters = Vec::new();
            for parameter in &declaration.declarations.parameters.members {
                let SchemaLookup::Known(slot) = schema.slot(parameter) else {
                    panic!("fixture Skill input must be declared")
                };
                if slot.presence == SlotPresence::RequiredOnce {
                    parameters.push(ParameterReadiness {
                        parameter: parameter.clone(),
                        phase: ReadinessPhase::Preparation,
                    });
                }
            }
            SkillReadiness {
                participation: None,
                skill,
                parameters: DeclaredSet::complete(parameters),
            }
        })
        .collect();
    let stages = Arc::new(
        OwnedEvaluationStages::new(
            EvaluationStagesInput {
                schema_version: OWNED_EVALUATION_STAGES_V3,
                namespace: intrinsic::ns(),
                release: intrinsic::key("finite-command-stages"),
                definitions: schema.identity().clone(),
                rules: *stored.identity(),
                routing: *routes.identity(),
                stages: vec![
                    EvaluationStage {
                        id: prepare.clone(),
                        predecessors: vec![],
                    },
                    EvaluationStage {
                        id: execute.clone(),
                        predecessors: vec![prepare.clone()],
                    },
                ],
                programs: DeclaredSet::complete(program_stages),
                effect_applications: Some(DeclaredSet::complete(vec![])),
                routing_stage: execute,
                frozen_channels: unused
                    .iter()
                    .map(|(id, scope, _)| FrozenStageChannel {
                        channel: StageChannel::Stat {
                            scope: *scope,
                            stat: id.clone(),
                        },
                        stage: prepare.clone(),
                    })
                    .collect(),
                readiness: Some(ReadinessInput {
                    skills,
                    programs: DeclaredSet::complete(program_readiness),
                }),
            },
            schema.as_ref(),
            &stored,
            &routes,
            Default::default(),
        )
        .unwrap(),
    );
    let preparation = Arc::new(
        OwnedSupportPreparation::new(
            SupportPreparationInput {
                schema_version: OWNED_SUPPORT_PREPARATION_VERSION,
                namespace: intrinsic::ns(),
                release: intrinsic::key("finite-no-supports"),
                definitions: schema.identity().clone(),
                rules: *stored.identity(),
                policy: SupportPreparationPolicy::OrderedReplacementRetryFrontierV1,
                quality_unit: def(2),
                types: vec![],
                effects: vec![],
                families: vec![],
                supports: vec![],
            },
            schema.as_ref(),
            &stored,
            Default::default(),
        )
        .unwrap(),
    );
    let presence = unused_input("target-presence");
    let inputs = Arc::new(
        OwnedSupportInputBindings::new(
            SupportInputBindingsInput {
                schema_version: OWNED_SUPPORT_INPUT_BINDINGS_VERSION,
                namespace: intrinsic::ns(),
                release: intrinsic::key("finite-no-support-inputs"),
                definitions: schema.identity().clone(),
                rules: *stored.identity(),
                preparation: *preparation.identity(),
                stages: *stages.identity(),
                preparation_stage: prepare,
                effective_level: unused_input("support-level"),
                effective_quality: unused_input("support-quality"),
                target: SupportTargetInputBindings {
                    skill_types: vec![],
                    minion_types: OptionalTypeInputs {
                        present: presence.clone(),
                        members: vec![],
                    },
                    summoner: OptionalTypeContextInputs {
                        present: presence.clone(),
                        skill_types: vec![],
                        minion_types: OptionalTypeInputs {
                            present: presence.clone(),
                            members: vec![],
                        },
                    },
                    cannot_be_supported: presence.clone(),
                    has_gem: presence.clone(),
                    from_item: presence.clone(),
                    is_player_actor: presence,
                },
            },
            schema.as_ref(),
            &stored,
            &preparation,
            &stages,
            Default::default(),
        )
        .unwrap(),
    );
    let receiving = Arc::new(
        OwnedSupportReceiving::new(
            SupportReceivingInput {
                schema_version: OWNED_SUPPORT_RECEIVING_V2,
                namespace: intrinsic::ns(),
                release: intrinsic::key("finite-no-support-receivers"),
                definitions: schema.identity().clone(),
                rules: *stored.identity(),
                preparation: *preparation.identity(),
                inputs: *inputs.identity(),
                stages: *stages.identity(),
                roles: vec![],
                targets: vec![],
                supports: vec![],
                source_properties: None,
            },
            schema.as_ref(),
            &stored,
            &preparation,
            &inputs,
            &stages,
            Default::default(),
        )
        .unwrap(),
    );
    Plan::compile(
        SupportEffectPlanInputs {
            request: Arc::new(f.request()),
            definitions: schema,
            rules,
            routing: routes,
            stages,
            preparation,
            inputs,
            receiving,
        },
        Default::default(),
        Default::default(),
    )
    .unwrap()
}
fn evaluated(plan: &Plan, scratch: &mut OwnedPlanScratch) -> OwnedEffectsReport {
    let report = plan.evaluate(scratch).unwrap();
    match report.outcome {
        SupportEffectsOutcome::Evaluated { effects } => effects,
        other => panic!(
            "expected complete finite component: {other:?}; gaps: {:?}",
            report.gaps
        ),
    }
}
fn evaluate(world: &World) -> OwnedEffectsReport {
    let p = compile(world);
    evaluated(&p, &mut p.new_scratch())
}
fn unavailable(plan: &Plan, gap: PlanGapReason) {
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(
        report.gaps.iter().any(|g| g.reason == gap),
        "{:?}",
        report.gaps
    );
    assert!(
        matches!(
            report.outcome,
            SupportEffectsOutcome::Unavailable {
                cause: EffectValue::Unresolved {
                    reason: PlanGapReason::IncompleteContributors,
                    read: None
                },
                input: None,
            }
        ),
        "{:?}",
        report.outcome
    );
}

fn contributions(r: &OwnedEffectsReport, stat: u64) -> Vec<&BoundEffectResult> {
    r.effects
        .iter()
        .filter(|e| matches!(&e.target,BoundEffectTarget::Contribution{key} if key.stat==def(stat)))
        .collect()
}
fn effect<'a>(r: &'a OwnedEffectsReport, a: &ActionSelection) -> &'a BoundEffectResult {
    let found:Vec<_>=contributions(r,0x3304).into_iter().filter(|e|matches!(&e.target,BoundEffectTarget::Contribution{key} if key.entity==ConcreteEntity::Action(Box::new(a.clone())))).collect();
    assert_eq!(found.len(), 1);
    found[0]
}
fn check(world: &World, r: &OwnedEffectsReport, expected: f64) {
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    fixture::check(r, 0.0);
    assert_eq!(
        known(value(r, ConcreteEntity::Actor(ActorKey::Player), 0x3302)),
        expected
    );
    let b: Value = family::read("bindings.json");
    let rows = contributions(r, 0x3302);
    assert_eq!(rows.len(), world.intrinsic.f.build.allocations.len());
    let mut ids = BTreeSet::new();
    for e in rows {
        let RuleOrigin::Provider { provider } = &e.key.invocation.origin else {
            panic!("allocation origin")
        };
        let ProviderRoot::Allocation(id) = provider.root else {
            panic!("actual allocation")
        };
        assert!(provider.grant_path.is_empty() && ids.insert(id));
        let allocation = world
            .intrinsic
            .f
            .build
            .allocations
            .iter()
            .find(|a| a.id == id)
            .unwrap();
        let n = b["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["definition"] == json!(allocation.node))
            .unwrap();
        assert_eq!(known(&e.value), n["amount"].as_f64().unwrap());
        assert_eq!(
            e.key.invocation.owner,
            SchemaSubject::Definition(allocation.node.address())
        );
    }
    for index in 0..2 {
        assert_eq!(known(fixture::actor_value(r, index, 0x3303)), expected);
        assert_eq!(
            fixture::actor_value(r, index, 0x1c),
            &EffectValue::Known {
                value: ParameterValue::Integer(BoundedInteger::new([44, 2][index]).unwrap())
            }
        );
        for a in actions(index) {
            let e = effect(r, &a);
            let gas = a.action.output.slot == def(0x25);
            assert_eq!(known(&e.value), if gas { expected } else { 0.0 });
            assert_eq!(
                e.key.invocation.origin,
                RuleOrigin::Provider {
                    provider: a.action.provider.clone()
                }
            );
            assert_eq!(
                e.key.invocation.program.as_str(),
                "conditional-command-damage"
            );
            let BoundEffectTarget::Contribution { key } = &e.target else {
                unreachable!()
            };
            assert_eq!(key.kind, ContributionKind::Increase);
        }
    }
    assert_ne!(intrinsic::actor(0), intrinsic::actor(1));
    assert!(
        !r.values
            .iter()
            .any(|v| matches!(&v.key,PlanValueKey::Stat{stat,..} if *stat==def(0x3304))),
        "Action contribution is not a final Damage scalar"
    );
}

#[test]
#[ignore = "requires actual published COMMAND_DAMAGE_RELEASE and COMMAND_DAMAGE_READINESS_RELEASE"]
fn command_damage_basic_zero_gas_fifty_five_all_sets_and_allocation_origins() {
    let w = world();
    check(&w, &evaluate(&w), 55.0);
}

#[test]
#[ignore = "requires actual published COMMAND_DAMAGE_RELEASE and COMMAND_DAMAGE_READINESS_RELEASE"]
fn each_real_node_removal_changes_only_the_conditional_damage_family() {
    let mut w = world();
    let originals = w.intrinsic.f.build.allocations.clone();
    for allocation in &originals {
        w.intrinsic.f.build.allocations = originals
            .iter()
            .filter(|a| a.id != allocation.id)
            .cloned()
            .collect();
        check(
            &w,
            &evaluate(&w),
            if allocation.node == def(0x139d) {
                40.0
            } else {
                35.0
            },
        );
    }
    w.intrinsic.f.build.allocations.clear();
    check(&w, &evaluate(&w), 0.0);
}

#[test]
#[ignore = "requires actual published COMMAND_DAMAGE_RELEASE and COMMAND_DAMAGE_READINESS_RELEASE"]
fn exact_allocation_scope_a_b_a_and_private_parallel_scratch() {
    let mut w = world();
    let loadouts = w.intrinsic.f.build.weapon_loadouts.clone();
    let allocation = w.intrinsic.f.build.allocations[0].id;
    w.intrinsic.f.build.allocations[0].scope = LoadoutScope::Selected {
        loadouts: vec![loadouts[1]],
    };
    let without = compile(&w);
    w.intrinsic.f.build.active_weapon_loadout = loadouts[1];
    let with = Arc::new(compile(&w));
    let mut scratch = with.new_scratch();
    let a = evaluated(&with, &mut scratch);
    check(&w, &a, 55.0);
    let b = evaluated(&without, &mut scratch);
    assert_eq!(
        known(value(&b, ConcreteEntity::Actor(ActorKey::Player), 0x3302)),
        35.0
    );
    assert!(contributions(&b,0x3302).iter().all(|e| !matches!(&e.key.invocation.origin,RuleOrigin::Provider{provider} if provider.root==ProviderRoot::Allocation(allocation))));
    for i in 0..2 {
        for a in actions(i) {
            assert_eq!(
                known(&effect(&b, &a).value),
                if a.action.output.slot == def(0x25) {
                    35.0
                } else {
                    0.0
                }
            );
        }
    }
    assert_eq!(a, evaluated(&with, &mut scratch));
    let reports: Vec<_> = (0..12)
        .into_par_iter()
        .map_init(|| with.new_scratch(), |s, _| evaluated(&with, s))
        .collect();
    assert!(reports.iter().all(|r| r == &a));
}

#[test]
#[ignore = "requires actual published COMMAND_DAMAGE_RELEASE and COMMAND_DAMAGE_READINESS_RELEASE"]
fn missing_eligibility_and_parent_inputs_never_become_damage_defaults() {
    let mut w = world();
    for a in actions(0) {
        w.intrinsic
            .f
            .owner_mut(&SchemaSubject::Slot(SlotAddress::ActionOutput(
                a.action.output,
            )))
            .programs
            .members
            .retain(|p| p.id.as_str() != "commandable-action-fact");
    }
    let r = evaluate(&w);
    for i in 0..2 {
        for a in actions(i) {
            assert!(matches!(
                &effect(&r, &a).value,
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    read: Some(_)
                }
            ));
        }
    }
    let mut w = world();
    w.intrinsic.missing_final_input();
    let r = evaluate(&w);
    for i in 0..2 {
        for a in actions(i) {
            assert!(matches!(
                &effect(&r, &a).value,
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    ..
                }
            ));
        }
    }
}

#[test]
#[ignore = "requires actual published COMMAND_DAMAGE_RELEASE and COMMAND_DAMAGE_READINESS_RELEASE"]
fn missing_actor_transport_and_entering_grant_keep_distinct_unavailable_causes() {
    let mut w = world();
    w.intrinsic
        .f
        .receivers
        .members
        .retain(|r| r.stat != def(0x3303));
    let r = evaluate(&w);
    assert_eq!(
        known(value(&r, ConcreteEntity::Actor(ActorKey::Player), 0x3302)),
        55.0
    );
    for i in 0..2 {
        assert_eq!(
            known(&effect(&r, &action(i, None)).value),
            0.0,
            "Basic selects the explicit false branch independently of the missing transport"
        );
        for a in actions(i).into_iter().skip(1) {
            assert!(matches!(
                &effect(&r, &a).value,
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    read: Some(_)
                }
            ));
        }
    }
    let mut w = world();
    let actor = w
        .intrinsic
        .f
        .owner_mut(&SchemaSubject::Definition(DefinitionAddress::Actor(def(
            0x3091,
        ))));
    let supply = actor
        .programs
        .members
        .iter_mut()
        .find(|p| p.id.as_str() == "gas-arrow-supply")
        .unwrap();
    let before = supply.effects.len();
    supply.effects.retain(
        |e| !matches!(&e.effect,RuleEffectKind::ActivateGrant{slot,..} if slot.slot==def(0x3095)),
    );
    assert_eq!(supply.effects.len() + 1, before);
    // V20 refuses the complete attempt; it does not fabricate effect rows for
    // an unknown selected supply.
    unavailable(&compile(&w), PlanGapReason::UnresolvedActivation);
}

#[test]
#[ignore = "requires actual published COMMAND_DAMAGE_RELEASE and COMMAND_DAMAGE_READINESS_RELEASE"]
fn actual_partial_action_or_passive_inventory_keeps_reduction_unresolved() {
    let d: Value = family::read("dependencies.json");
    let owners: Vec<DefinitionRules> =
        serde_json::from_value(json!([d["owners"][0], d["action_owners"][1]])).unwrap();
    for owner in owners {
        let mut w = world();
        assert!(!owner.programs.is_complete());
        w.intrinsic.f.owner_mut(&owner.owner).programs.closure = owner.programs.closure;
        unavailable(&compile(&w), PlanGapReason::PartialPrograms);
    }
}

#[test]
#[ignore = "requires actual published COMMAND_DAMAGE_RELEASE and COMMAND_DAMAGE_READINESS_RELEASE"]
fn disabled_root_does_not_create_an_independent_generated_actor() {
    let mut w = world();
    w.intrinsic.f.build.skills[1].enabled = false;
    let root = ProviderRoot::SkillUse(w.intrinsic.f.build.skills[1].id);
    let p = compile(&w);
    assert!(
        p.binding_report()
            .issues()
            .iter()
            .any(|i| i.code == BindingIssueCode::DisabledProvider)
    );
    // The selected disabled query makes topology incomplete. The public staged
    // API returns no evaluated actor/action rows for this unavailable request.
    unavailable(&p, PlanGapReason::UnresolvedTopology);
    w.intrinsic
        .f
        .queries
        .requests
        .retain(|q| !matches!(&q.target,MetricTarget::Action(a) if a.action.provider.root==root));
    let r = evaluate(&w);
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    for a in actions(0) {
        assert_eq!(
            known(&effect(&r, &a).value),
            if a.action.output.slot == def(0x25) {
                55.0
            } else {
                0.0
            }
        );
    }
}

#[test]
#[ignore = "requires actual published COMMAND_DAMAGE_RELEASE and COMMAND_DAMAGE_READINESS_RELEASE"]
fn published_population_requirement_remains_an_independent_execution_result() {
    let mut world = world();
    for character_level in [100, 89] {
        world.intrinsic.f.build.character.level = character_level;
        let report = evaluate(&world);
        // A failed Requirement is an explicit result, not an invented activation
        // condition. The original factual projections and Command rows stay live.
        check(&world, &report, 55.0);
        let requirements: Vec<_> = report
            .effects
            .iter()
            .filter(|effect| {
                matches!(&effect.target, BoundEffectTarget::Requirement { code }
                if code.as_str() == "required-character-level")
            })
            .collect();
        assert_eq!(requirements.len(), 2);
        for (index, threshold) in [90, 0].into_iter().enumerate() {
            let root = ProviderRoot::SkillUse(world.intrinsic.f.build.skills[index].id);
            let rows: Vec<_> = requirements
                .iter()
                .filter(|effect| {
                    matches!(&effect.key.invocation.origin, RuleOrigin::Provider { provider }
                    if provider.root == root && provider.grant_path.len() == 1)
                })
                .collect();
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0].key.invocation.program.as_str(),
                "ordinary-population-requirements"
            );
            assert_eq!(
                rows[0].value,
                EffectValue::Known {
                    value: ParameterValue::Boolean(character_level >= threshold),
                }
            );
        }
    }
}
