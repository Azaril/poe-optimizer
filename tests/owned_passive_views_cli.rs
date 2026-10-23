//! Retained isolated whole-plan receiver law. Seven-view source/component and
//! Original05 normalization laws now live in Authoring view_support_domains.
//! This predecessor fixture remains until a current checked readiness bundle
//! permits equivalent disabled/exact-actor/Partial execution without coercion.
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::{OwnedActionRouting, RoutingLimits},
    owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits, SchemaPackageInput},
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use poe_optimizer_import::{
    owned_passive_views::ViewRecipePolicy,
    owned_recipe::{StagedOwnedRecipe, assemble_owned_recipe},
    owned_tree_policy::{TreeNormalizationPackageInput, TreeTokenRole},
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::Arc,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68")
}
fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn compile(
    command: &str,
    cwd: &Path,
    prior: &Path,
    catalog: &Path,
    policy: &Path,
    statistics: &Path,
    out: &Path,
) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg(command)
        .arg(prior)
        .arg("--catalog")
        .arg(catalog)
        .arg("--policy")
        .arg(policy)
        .arg("--statistics")
        .arg(statistics)
        .arg("--output")
        .arg(out)
        .output()
        .unwrap()
}
struct Fixture {
    temp: tempfile::TempDir,
    prior: PathBuf,
    catalog: PathBuf,
    policy: PathBuf,
    statistics: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let prior = temp.path().join("attributes");
        let catalog = temp.path().join("catalog.json");
        fs::copy(data().join("tree/tree-catalog.json"), &catalog).unwrap();
        success(compile(
            "compile-owned-attributes",
            temp.path(),
            &data().join("current"),
            &catalog,
            &data().join("attributes/policy.json"),
            &data().join("attributes/statistics.json"),
            &prior,
        ));
        let policy = temp.path().join("policy.json");
        let statistics = temp.path().join("statistics.json");
        fs::copy(data().join("passive-views/policy.json"), &policy).unwrap();
        fs::copy(data().join("passive-views/statistics.json"), &statistics).unwrap();
        Self {
            temp,
            prior,
            catalog,
            policy,
            statistics,
        }
    }
    fn run(&self, prior: &Path, out: &Path) -> Output {
        compile(
            "compile-owned-passive-views",
            self.temp.path(),
            prior,
            &self.catalog,
            &self.policy,
            &self.statistics,
            out,
        )
    }
}
fn node(tree: &TreeNormalizationPackageInput, token: &str) -> PassiveNodeDefId {
    let TreeTokenRole::Allocation { node, .. } = &tree
        .content
        .tokens
        .iter()
        .find(|n| n.token == token)
        .unwrap()
        .role
    else {
        panic!("allocation token")
    };
    node.clone()
}
fn quantity(v: &ParameterValue) -> f64 {
    let ParameterValue::Quantity(v) = v else {
        panic!("quantity")
    };
    v.value()
}

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn ports() -> DeclaredSlots {
    DeclaredSlots {
        parameters: empty(),
        choices: empty(),
        grants: empty(),
        actors: empty(),
        skill_grants: empty(),
        outputs: empty(),
        sockets: empty(),
    }
}
fn entry<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn occurrence<I: BuildInstanceId>(n: u64) -> I {
    I::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([79; 16]), n).unwrap())
}
struct ReceiverFixture {
    plan: OwnedEffectPlan<OwnedDefinitionSchemaPackage>,
    player_stat: StatDefId,
    actor_stat: StatDefId,
    sniper: ActorKey,
    unrelated: ActorKey,
}
// A separate synthetic closed world supplies activation and class/point topology.
// Production descriptors/coverage are not changed or claimed complete. The
// generated passive program and both persisted receiver programs/registrations,
// including actual Sniper actor/stat IDs, are used without edits.
fn receiver_fixture(
    recipe: &StagedOwnedRecipe,
    policy: &ViewRecipePolicy,
    tree: &TreeNormalizationPackageInput,
    witch: bool,
    enabled: bool,
    partial: bool,
) -> ReceiverFixture {
    let namespace = recipe.schema().input().namespace.clone();
    let named = |s: &str| OwnedDefinitionKey::new(s).unwrap();
    let class = tree
        .content
        .classes
        .iter()
        .find(|c| c.key == if witch { "1" } else { "7" })
        .unwrap()
        .class
        .clone();
    let classes: Vec<_> = tree
        .content
        .classes
        .iter()
        .filter(|c| c.key == "1" || c.key == "7")
        .map(|c| c.class.clone())
        .collect();
    let node_id = node(tree, "1755");
    let owner = SchemaSubject::Definition(node_id.address());
    let mut passive = recipe
        .rules()
        .input()
        .owners
        .iter()
        .find(|r| r.owner == owner)
        .unwrap()
        .clone();
    let SchemaLookup::Known(passive_schema) = recipe.schema().definition(&node_id) else {
        panic!("node schema")
    };
    let pool = passive_schema.pools.members[0].clone();
    let owned_receiver = policy
        .receivers
        .iter()
        .find(|r| {
            matches!(
                r.targets.as_slice(),
                [ActorReceiverTarget::OwnedSlot { .. }]
            )
        })
        .unwrap();
    let ActorReceiverTarget::OwnedSlot { slot: sniper_slot } = &owned_receiver.targets[0] else {
        unreachable!()
    };
    let SlotOwnerDefId::Skill(skill) = &sniper_slot.declaration else {
        panic!("Sniper skill owner")
    };
    let grant = recipe
        .schema()
        .input()
        .slots
        .iter()
        .find_map(|d| match d {
            SlotDescriptor::Grant(e) if e.id.declaration == sniper_slot.declaration => match &e
                .schema
            {
                SchemaState::Known(s) if s.target == GrantTarget::Actor(sniper_slot.clone()) => {
                    Some(e.id.clone())
                }
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    let unrelated_slot = DeclaredSlot {
        declaration: sniper_slot.declaration.clone(),
        slot: ActorSlotDefId::new(namespace.clone(), named("fixture.other-actor")),
    };
    let unrelated_grant = DeclaredSlot {
        declaration: sniper_slot.declaration.clone(),
        slot: GrantSlotDefId::new(namespace.clone(), named("fixture.other-grant")),
    };
    let encounter = EncounterDefId::new(namespace.clone(), named("fixture.encounter"));
    let range = IntegerRange {
        minimum: BoundedInteger::new(1).unwrap(),
        maximum: BoundedInteger::new(100).unwrap(),
    };
    let mut definitions = vec![];
    for c in &classes {
        definitions.push(DefinitionDescriptor::Class(entry(
            c.clone(),
            ClassSchema {
                level: range.clone(),
                ascendancies: empty(),
                implicit_passives: empty(),
                declarations: ports(),
            },
        )));
    }
    definitions.extend([
        DefinitionDescriptor::PassiveNode(entry(
            node_id.clone(),
            PassiveNodeSchema {
                pools: DeclaredSet::complete(vec![pool.clone()]),
                adjacent: empty(),
                declarations: ports(),
            },
        )),
        DefinitionDescriptor::PointPool(entry(
            pool.clone(),
            PointPoolSchema {
                scope: PointPoolScope::Shared,
            },
        )),
        DefinitionDescriptor::Encounter(entry(
            encounter.clone(),
            EncounterSchema {
                enemy_level: range,
                external_inputs: empty(),
            },
        )),
    ]);
    let statistics: Vec<DefinitionEntry<StatDefId, StatSchema>> =
        serde_json::from_value(read(data().join("passive-views/statistics.json"))).unwrap();
    for s in statistics {
        definitions.push(DefinitionDescriptor::Stat(s));
    }
    let unit = match &policy.nodes[0].default.contributions[0].value {
        ParameterValue::Quantity(q) => q.unit().clone(),
        _ => panic!("percentage contribution"),
    };
    definitions.push(
        recipe
            .schema()
            .lookup_definition(&unit.address())
            .unwrap()
            .clone(),
    );
    let mut skill_ports = ports();
    skill_ports.actors = DeclaredSet::complete(vec![sniper_slot.clone(), unrelated_slot.clone()]);
    skill_ports.grants = DeclaredSet::complete(vec![grant.clone(), unrelated_grant.clone()]);
    definitions.push(DefinitionDescriptor::Skill(entry(
        skill.clone(),
        SkillSchema {
            directly_selectable: true,
            declarations: skill_ports,
        },
    )));
    let mut slots = vec![];
    for (actor, activation) in [
        (sniper_slot.clone(), grant.clone()),
        (unrelated_slot.clone(), unrelated_grant.clone()),
    ] {
        slots.push(SlotDescriptor::Actor(entry(
            actor.clone(),
            ActorSlotSchema {
                provider_definition: None,
                skills: empty(),
                outputs: empty(),
            },
        )));
        slots.push(SlotDescriptor::Grant(entry(
            activation,
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Actor(actor),
            },
        )));
    }
    let schema = Arc::new(
        OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: recipe.schema().input().schema_version,
                namespace: namespace.clone(),
                release: key("synthetic-receiver-fixture"),
                semantics_version: key("synthetic-v1"),
                definitions,
                slots,
            },
            OwnedSchemaLimits::default(),
        )
        .unwrap(),
    );
    if partial {
        passive.programs.closure = SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: owner,
                facet: SchemaFacet::GameRules,
                code: key("synthetic-missing-contributor"),
            }],
        };
    }
    let mut owners = policy
        .receiver_rules
        .iter()
        .map(|expected| {
            let actual = recipe
                .rules()
                .input()
                .owners
                .iter()
                .find(|row| row.owner == expected.owner)
                .unwrap();
            assert_eq!(
                actual, expected,
                "published receiver rule differs from its policy"
            );
            actual.clone()
        })
        .collect::<Vec<_>>();
    let receivers = policy
        .receivers
        .iter()
        .map(|expected| {
            let actual = recipe
                .rules()
                .input()
                .receivers
                .members
                .iter()
                .find(|row| row.id == expected.id)
                .unwrap();
            assert_eq!(
                actual, expected,
                "published receiver applicability differs from its policy"
            );
            actual.clone()
        })
        .collect::<Vec<_>>();
    owners.push(passive);
    for subject in classes
        .iter()
        .map(|c| SchemaSubject::Definition(c.address()))
        .chain([
            SchemaSubject::Definition(encounter.address()),
            SchemaSubject::Slot(SlotAddress::Actor(sniper_slot.clone())),
            SchemaSubject::Slot(SlotAddress::Actor(unrelated_slot.clone())),
            SchemaSubject::Slot(SlotAddress::Grant(grant.clone())),
            SchemaSubject::Slot(SlotAddress::Grant(unrelated_grant.clone())),
        ])
    {
        owners.push(DefinitionRules {
            owner: subject,
            programs: empty(),
        });
    }
    owners
        .iter_mut()
        .find(|row| row.owner == SchemaSubject::Slot(SlotAddress::Actor(unrelated_slot.clone())))
        .unwrap()
        .programs = DeclaredSet::complete(vec![RuleProgram {
        id: key("synthetic-presence"),
        context: RuleEntityKind::Actor,
        reads: vec![],
        nodes: vec![RuleNode {
            id: key("present"),
            expression: RuleExpression::Literal {
                value: ParameterValue::Boolean(true),
            },
        }],
        effects: vec![RuleEffect {
            id: key("observed"),
            when: None,
            effect: RuleEffectKind::Requirement {
                satisfied: key("present"),
                code: key("synthetic-present"),
            },
        }],
    }]);
    owners.push(DefinitionRules {
        owner: SchemaSubject::Definition(skill.address()),
        programs: DeclaredSet::complete(vec![RuleProgram {
            id: key("synthetic-explicit-activation"),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![RuleNode {
                id: key("enabled"),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Boolean(true),
                },
            }],
            effects: [grant, unrelated_grant]
                .into_iter()
                .enumerate()
                .map(|(i, slot)| RuleEffect {
                    id: key(&format!("activation-{i}")),
                    when: None,
                    effect: RuleEffectKind::ActivateGrant {
                        slot,
                        enabled: key("enabled"),
                    },
                })
                .collect(),
        }]),
    });
    let rules = Arc::new(
        CompiledRulePackage::compile(
            &RulePackageInput {
                support_discovery: Some(SupportDiscoveryInput {
                    providers: owners
                        .iter()
                        .map(|row| SupportSourceDomainDeclaration {
                            owner: row.owner.clone(),
                            domain: SchemaState::Known(
                                SupportSourceDomain::AuthoredAssignmentsOnly,
                            ),
                        })
                        .collect(),
                }),
                existing_actor_rules: None,
                contribution_queries: None,
                effect_applications: None,
                schema_version: OWNED_RULE_PACKAGE_VERSION,
                namespace: namespace.clone(),
                release: key("synthetic-receiver-rules"),
                semantics_version: key("synthetic-v1"),
                operations_version: key(OWNED_RULE_OPERATIONS_V14),
                definitions: schema.identity().clone(),
                tables: vec![],
                owners,
                receivers: DeclaredSet::complete(receivers),
            },
            schema.as_ref(),
            Default::default(),
        )
        .unwrap(),
    );
    let routing = Arc::new(
        OwnedActionRouting::new(
            ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: namespace.clone(),
                release: key("synthetic-empty-routing"),
                definitions: schema.identity().clone(),
                outputs: vec![],
            },
            schema.as_ref(),
            RoutingLimits::default(),
        )
        .unwrap(),
    );
    let limits = OwnedInputLimits::default();
    let request = OwnedEvaluationRequest::new(
        BuildSpec::new(
            BuildInput {
                usage_preferences: vec![],
                generated_inputs: None,
                authored_support_order: None,
                allocator: InstanceAllocatorState::from_parts(
                    BuildLineage::from_bytes([79; 16]),
                    100,
                ),
                revision: BuildRevision::from_u64(1),
                game_version: namespace.clone(),
                character: CharacterSpec {
                    class,
                    ascendancy: None,
                    level: 20,
                    rewards: vec![],
                },
                weapon_loadouts: vec![occurrence(1)],
                active_weapon_loadout: occurrence(1),
                items: vec![],
                gems: vec![],
                equipment: vec![],
                allocations: vec![Allocation {
                    id: occurrence(2),
                    node: node_id,
                    pool,
                    scope: LoadoutScope::Shared,
                    access: AllocationAccess::Ordinary,
                    choices: vec![],
                }],
                skills: vec![SkillUse {
                    parameters: None,
                    id: occurrence(3),
                    source: AuthoredSkillSource::Direct(skill.clone()),
                    enabled,
                    scope: LoadoutScope::Shared,
                }],
                supports: vec![],
                payload_links: vec![],
                choices: vec![],
            },
            limits,
        )
        .unwrap(),
        ScenarioSpec::new(
            ScenarioInput {
                usage_intents: vec![],
                game_version: namespace.clone(),
                enemy: EnemySpec {
                    encounter,
                    level: 20,
                },
                assumptions: vec![],
                usage: vec![],
            },
            limits,
        )
        .unwrap(),
        QuerySpec::new(
            QueryInput {
                game_version: namespace,
                requests: vec![],
            },
            limits,
        )
        .unwrap(),
        limits,
    )
    .unwrap();
    let actor = |slot| {
        ActorKey::Owned(Box::new(OwnedActorKey {
            provider: ProviderKey {
                root: ProviderRoot::SkillUse(occurrence(3)),
                grant_path: vec![],
            },
            slot,
        }))
    };
    ReceiverFixture {
        plan: OwnedEffectPlan::compile(
            Arc::new(request),
            schema,
            rules,
            routing,
            PlanLimits::default(),
        )
        .unwrap(),
        player_stat: policy
            .receivers
            .iter()
            .find(|r| r.targets == [ActorReceiverTarget::Player])
            .unwrap()
            .stat
            .clone(),
        actor_stat: owned_receiver.stat.clone(),
        sniper: actor(sniper_slot.clone()),
        unrelated: actor(unrelated_slot),
    }
}
fn receiver_value<'a>(
    report: &'a OwnedEffectsReport,
    actor: ActorKey,
    stat: &StatDefId,
) -> Option<&'a EffectValue> {
    report
        .values
        .iter()
        .find(|r| {
            r.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(actor.clone()),
                    stat: stat.clone(),
                }
        })
        .map(|r| &r.value)
}

#[test]
fn persisted_view_contributions_reach_exact_sniper_receiver_in_separate_native_fixture() {
    let f = Fixture::new();
    let output = f.temp.path().join("views");
    success(f.run(&f.prior, &output));
    let recipe = assemble_owned_recipe(
        serde_json::from_value(read(output.join("recipe.json"))).unwrap(),
        Default::default(),
    )
    .unwrap();
    let policy: ViewRecipePolicy = serde_json::from_value(read(&f.policy)).unwrap();
    let tree: TreeNormalizationPackageInput =
        serde_json::from_value(read(output.join("tree-normalization.json"))).unwrap();
    assert!(
        recipe
            .rules()
            .input()
            .owners
            .iter()
            .any(|o| !o.programs.is_complete()),
        "production coverage must remain explicit"
    );
    for (witch, expected) in [(true, 8.0), (false, 0.0)] {
        let fixture = receiver_fixture(&recipe, &policy, &tree, witch, true, false);
        let report = fixture
            .plan
            .evaluate(&mut fixture.plan.new_scratch())
            .unwrap();
        assert!(report.gaps.is_empty(), "{report:?}");
        for (actor, stat) in [
            (ActorKey::Player, &fixture.player_stat),
            (fixture.sniper.clone(), &fixture.actor_stat),
        ] {
            let Some(EffectValue::Known { value }) = receiver_value(&report, actor, stat) else {
                panic!("receiver result must be known")
            };
            assert_eq!(quantity(value), expected);
        }
        assert!(
            report.effects.iter().any(|row| row.key.invocation.entity
                == ConcreteEntity::Actor(fixture.unrelated.clone())
                && row.key.invocation.program == key("synthetic-presence")
                && row.value
                    == EffectValue::Known {
                        value: ParameterValue::Boolean(true)
                    }),
            "unrelated actor must actually be active"
        );
        assert!(
            receiver_value(&report, fixture.unrelated, &fixture.actor_stat).is_none(),
            "unlisted actor cannot inherit Sniper receiver"
        );
    }
    let disabled = receiver_fixture(&recipe, &policy, &tree, true, false, false);
    let report = disabled
        .plan
        .evaluate(&mut disabled.plan.new_scratch())
        .unwrap();
    assert!(receiver_value(&report, disabled.sniper, &disabled.actor_stat).is_none());
    let Some(EffectValue::Known { value }) =
        receiver_value(&report, ActorKey::Player, &disabled.player_stat)
    else {
        panic!("disabling Sniper must retain the player's grant")
    };
    assert_eq!(quantity(value), 8.0);
    let partial = receiver_fixture(&recipe, &policy, &tree, true, true, true);
    let report = partial
        .plan
        .evaluate(&mut partial.plan.new_scratch())
        .unwrap();
    assert!(!report.gaps.is_empty());
    assert!(matches!(
        receiver_value(&report, ActorKey::Player, &partial.player_stat),
        Some(EffectValue::Unresolved { .. })
    ));
    assert!(matches!(
        receiver_value(&report, partial.sniper, &partial.actor_stat),
        Some(EffectValue::Unresolved { .. })
    ));
}
