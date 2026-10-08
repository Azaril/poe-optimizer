//! Real published INC bodies, memberships and scalar receivers in an explicit
//! finite component domain. BASE values are authenticated call projections, not
//! imported game rules; equipment, class rules, topology and other providers are
//! excluded. This fixture cannot certify a complete build or BASE ordering.
use super::{empty_support, family, release, source as witness};
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::owned_recipe::OwnedRecipeInput;
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, path::PathBuf, sync::OnceLock};

fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(ns(), format!("def.{n:016x}")).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    value.parse().unwrap()
}
fn subject<I: SchemaDefinitionId>(id: I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
fn decode<T: DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}
fn known(v: &Value) -> &Value {
    assert_eq!(v["kind"], "known");
    &v["value"]
}
fn referenced_keys(v: &Value, keys: &mut BTreeSet<String>) {
    match v {
        Value::Object(o) => {
            if o.contains_key("namespace")
                && let Some(k) = o.get("key").and_then(Value::as_str)
            {
                keys.insert(k.to_owned());
            }
            for v in o.values() {
                referenced_keys(v, keys);
            }
        }
        Value::Array(a) => {
            for v in a {
                referenced_keys(v, keys);
            }
        }
        _ => {}
    }
}
fn ports() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
struct Source {
    recipe: OwnedRecipeInput,
    owners: Vec<DefinitionRules>,
    builds: Vec<BuildInput>,
    cases: Vec<Value>,
    contributors: Vec<Value>,
}
fn source() -> &'static Source {
    static SOURCE: OnceLock<Source> = OnceLock::new();
    SOURCE.get_or_init(|| {
        let path = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ATTRIBUTE_INCREASE_RELEASE")
                .expect("verified current increased-attribute successor"),
        );
        let inventory = release::inventory(&path);
        let endpoint = release::load(&path);
        family::assert_endpoint(&endpoint);
        let dependencies: family::Dependencies = family::read("dependencies.json");
        let bindings: Value = family::read("bindings.json");
        let contributors = bindings["contributors"].as_array().unwrap().clone();
        let owners = dependencies.owners;
        let read = |name: &str| -> Value {
            serde_json::from_slice(&fs::read(path.parent().unwrap().join(name)).unwrap()).unwrap()
        };
        let builds = (1..=5)
            .map(|i| {
                let document = read(&format!("original-{i:02}/draft.json"));
                let d = &document["draft"];
                let selection = read(&format!("selected-{i:02}.json"));
                let selected = &selection["build"];
                let character = d["character_presets"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|p| p["id"] == selected["character"])
                    .unwrap();
                let preset = d["allocation_presets"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|p| p["id"] == selected["allocations"])
                    .unwrap();
                assert_eq!(preset["allocations"]["completion"]["kind"], "complete");
                let selected_ids = preset["allocations"]["members"].as_array().unwrap();
                let allocations = d["allocations"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|a| selected_ids.contains(&a["id"]))
                    .filter_map(|a| {
                        let node: PassiveNodeDefId = decode(known(&a["node"]));
                        if !owners.iter().any(|o| o.owner == subject(node.clone())) {
                            return None;
                        }
                        assert_eq!(a["choices"]["completion"]["kind"], "complete");
                        Some(Allocation {
                            id: decode(&a["id"]),
                            node,
                            pool: decode(known(&a["pool"])),
                            scope: decode(known(&a["scope"])),
                            access: decode(&a["access"]),
                            choices: a["choices"]["members"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|c| ChoiceSelection {
                                    slot: decode(known(&c["slot"])),
                                    value: decode(known(&c["value"])),
                                })
                                .collect(),
                        })
                    })
                    .collect();
                BuildInput {
                    allocator: decode(&d["allocator"]),
                    revision: decode(&d["revision"]),
                    game_version: ns(),
                    character: CharacterSpec {
                        class: decode(known(&character["class"])),
                        ascendancy: None,
                        level: decode(known(&character["level"])),
                        rewards: vec![],
                    },
                    weapon_loadouts: decode(&d["weapon_loadouts"]["members"]),
                    active_weapon_loadout: decode(&selected["active_weapon_loadout"]),
                    allocations,
                    items: vec![],
                    gems: vec![],
                    equipment: vec![],
                    skills: vec![],
                    supports: vec![],
                    authored_support_order: Some(vec![]),
                    generated_inputs: Some(GeneratedSkillInputsV1 {
                        schema_version: 1,
                        bindings: vec![],
                    }),
                    payload_links: vec![],
                    choices: vec![],
                }
            })
            .collect();
        assert_eq!(inventory, release::inventory(&path));
        Source {
            recipe: endpoint.input().recipe.clone(),
            owners,
            builds,
            cases: witness::native_cases(),
            contributors,
        }
    })
}
fn actual_owner(subject: &SchemaSubject) -> &'static DefinitionRules {
    source()
        .recipe
        .rules
        .owners
        .iter()
        .find(|o| &o.owner == subject)
        .unwrap()
}
type Plan = OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>;
#[derive(Clone)]
struct World {
    recipe: OwnedRecipeInput,
    build: BuildInput,
}
impl World {
    fn new(index: usize) -> Self {
        let s = source();
        let build = s.builds[index].clone();
        let mut owners = s.owners.clone();
        // These are the actual twelve scalar-producing owners and receiver
        // bodies, including the real empty-MORE coverage guard at every stage.
        for n in (0x3321..=0x3329).chain(0x1d2e..=0x1d30) {
            let owner = actual_owner(&subject(def::<StatDefinition>(n)));
            assert!(owner.programs.is_complete());
            assert_eq!(owner.programs.members.len(), 1);
            owners.push(owner.clone());
        }
        let stages = s.cases[index]["stages"].as_array().unwrap();
        assert_eq!(stages.len(), 6);
        let class_owner = subject(build.character.class.clone());
        let mut base = RuleProgram {
            id: key("fixture-projected-base"),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![],
            effects: vec![],
        };
        for (i, stage) in stages.iter().enumerate() {
            let id = key(&format!("base-{i}"));
            base.nodes.push(RuleNode {
                id: id.clone(),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Quantity(
                        FiniteQuantity::new(stage["base"].as_f64().unwrap(), def(0x295a)).unwrap(),
                    ),
                },
            });
            base.effects.push(RuleEffect {
                id: id.clone(),
                when: None,
                effect: RuleEffectKind::Contribute {
                    entity: RuleEntity::Player,
                    stat: def(0x331b + i as u64),
                    contribution: ContributionKind::Add,
                    value: id,
                },
            });
        }
        owners.push(DefinitionRules {
            owner: class_owner.clone(),
            programs: DeclaredSet::complete(vec![base]),
        });
        let mut recipe = s.recipe.clone();
        let receivers: Vec<_> = recipe
            .rules
            .receivers
            .members
            .iter()
            .filter(|r| owners.iter().any(|o| o.owner == subject(r.stat.clone())))
            .cloned()
            .collect();
        assert_eq!(receivers.len(), 12);
        recipe.rules.receivers = DeclaredSet::complete(receivers);
        recipe.rules.effect_applications = Some(DeclaredSet::complete(vec![]));
        recipe.rules.existing_actor_rules = Some(DeclaredSet::complete(vec![]));
        recipe.rules.tables.clear();
        recipe.routing.outputs.clear();
        let registry = recipe.rules.contribution_queries.as_mut().unwrap();
        assert!(!registry.is_complete());
        assert_eq!(registry.members.len(), 18);
        registry.closure = SchemaClosure::Complete;
        let mut base_count = 0;
        for q in &mut registry.members {
            if q.contribution != ContributionKind::Add {
                continue;
            }
            assert!(!q.groups[0].members.is_complete());
            let index = (0..6).find(|i| q.stat == def(0x331b + i)).unwrap();
            q.groups[0].members = DeclaredSet::complete(vec![ContributionMember {
                owner: class_owner.clone(),
                program: key("fixture-projected-base"),
                effect: key(&format!("base-{index}")),
                origin: ContributionOrigin::Character,
                order: Some(ContributionOrder {
                    source_rank: 0,
                    program_rank: 0,
                    effect_rank: index as u32,
                    slot_ranks: vec![],
                }),
            }]);
            base_count += 1;
        }
        assert_eq!(base_count, 6);
        let mut keys = BTreeSet::new();
        referenced_keys(&json!(owners), &mut keys);
        referenced_keys(&json!(build), &mut keys);
        referenced_keys(&json!(recipe.rules.receivers), &mut keys);
        referenced_keys(&json!(recipe.rules.contribution_queries), &mut keys);
        for n in [1, 2, 0x295a, 0x31d1] {
            keys.insert(format!("def.{n:016x}"));
        }
        // The fixture retains complete actual passive ports and contribution
        // bodies. Class, encounter and topology exclusions are explicit here.
        for d in &mut recipe.schema.definitions {
            if !keys.contains(d.address().key().as_str()) {
                continue;
            }
            match d {
                DefinitionDescriptor::Class(e) => {
                    let SchemaState::Known(v) = &mut e.schema else {
                        panic!("known class")
                    };
                    v.ascendancies = DeclaredSet::complete(vec![]);
                    v.implicit_passives = DeclaredSet::complete(vec![]);
                    v.declarations = ports();
                }
                DefinitionDescriptor::PassiveNode(e) => {
                    let SchemaState::Known(v) = &mut e.schema else {
                        panic!("known passive")
                    };
                    v.adjacent = DeclaredSet::complete(vec![]);
                }
                DefinitionDescriptor::Encounter(e) => {
                    let SchemaState::Known(v) = &mut e.schema else {
                        panic!("known encounter")
                    };
                    v.external_inputs = DeclaredSet::complete(vec![]);
                }
                _ => {}
            }
        }
        loop {
            let before = keys.len();
            for d in &recipe.schema.definitions {
                if keys.contains(d.address().key().as_str()) {
                    referenced_keys(&json!(d), &mut keys);
                }
            }
            for slot in &recipe.schema.slots {
                if keys.contains(slot.address().key().as_str()) {
                    referenced_keys(&json!(slot), &mut keys);
                }
            }
            if keys.len() == before {
                break;
            }
        }
        recipe
            .schema
            .definitions
            .retain(|d| keys.contains(d.address().key().as_str()));
        recipe
            .schema
            .slots
            .retain(|d| keys.contains(d.address().key().as_str()));
        recipe.rules.owners = recipe
            .schema
            .definitions
            .iter()
            .map(|d| {
                let owner = SchemaSubject::Definition(d.address());
                owners
                    .iter()
                    .find(|o| o.owner == owner)
                    .cloned()
                    .unwrap_or(DefinitionRules {
                        owner,
                        programs: DeclaredSet::complete(vec![]),
                    })
            })
            .collect();
        Self { recipe, build }
    }
    fn plan(&self) -> std::result::Result<Plan, PlanError> {
        empty_support::compile(
            &self.recipe,
            &self.build,
            &ScenarioInput {
                game_version: ns(),
                enemy: EnemySpec {
                    encounter: def(0x31d1),
                    level: 20,
                },
                assumptions: vec![],
                usage: vec![],
            },
            def(2),
        )
    }
    fn report(&self) -> SupportEffectsReport {
        let plan = self.plan().unwrap();
        plan.evaluate(&mut plan.new_scratch()).unwrap()
    }
    fn owner_mut(&mut self, owner: &SchemaSubject) -> &mut DefinitionRules {
        self.recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| &o.owner == owner)
            .unwrap()
    }
    fn all_members(&mut self) {
        let lineage = self.build.allocator.lineage();
        self.build.allocator = InstanceAllocatorState::from_parts(lineage, 100_000);
        self.build.allocations = source()
            .contributors
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let node: PassiveNodeDefId = decode(&c["owner"]["value"]["value"]);
                let descriptor = self
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .find(|d| d.address() == node.address())
                    .unwrap();
                let DefinitionDescriptor::PassiveNode(entry) = descriptor else {
                    panic!()
                };
                let SchemaState::Known(schema) = &entry.schema else {
                    panic!()
                };
                assert!(schema.pools.is_complete());
                assert_eq!(schema.pools.members.len(), 1);
                Allocation {
                    id: AllocationId::from_instance_id(
                        InstanceId::from_parts(lineage, 90_000 + i as u64).unwrap(),
                    ),
                    node,
                    pool: schema.pools.members[0].clone(),
                    scope: LoadoutScope::Shared,
                    access: AllocationAccess::Ordinary,
                    choices: vec![],
                }
            })
            .collect();
    }
    fn potential(&mut self, stage: u64, value: Option<f64>, enabled: bool, kind: ContributionKind) {
        let mut reads = vec![];
        let expression = if let Some(value) = value {
            RuleExpression::Literal {
                value: ParameterValue::Quantity(
                    FiniteQuantity::new(
                        value,
                        if kind == ContributionKind::Multiply {
                            def(1)
                        } else {
                            def(2)
                        },
                    )
                    .unwrap(),
                ),
            }
        } else {
            // A declared, unproduced percentage channel is introduced only for
            // this binder control. It must not be evaluated before membership.
            let missing: StatDefId = DefId::parse(ns(), "fixture.missing-increase-input").unwrap();
            self.recipe
                .schema
                .definitions
                .push(DefinitionDescriptor::Stat(DefinitionEntry {
                    id: missing.clone(),
                    schema: SchemaState::Known(StatSchema {
                        value: ComputedValueType::Quantity { unit: def(2) },
                        targets: vec![RuleEntityKind::Actor],
                    }),
                }));
            self.recipe.rules.owners.push(DefinitionRules {
                owner: subject(missing.clone()),
                programs: DeclaredSet::complete(vec![]),
            });
            reads.push(RuleRead {
                id: key("missing"),
                value_type: ComputedValueType::Quantity { unit: def(2) },
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: missing,
                },
            });
            RuleExpression::Read {
                input: key("missing"),
            }
        };
        self.owner_mut(&subject(self.build.character.class.clone()))
            .programs
            .members
            .push(RuleProgram {
                id: key("fixture-potential-extra"),
                context: RuleEntityKind::Actor,
                reads,
                nodes: vec![
                    RuleNode {
                        id: key("amount"),
                        expression,
                    },
                    RuleNode {
                        id: key("enabled"),
                        expression: RuleExpression::Literal {
                            value: ParameterValue::Boolean(enabled),
                        },
                    },
                ],
                effects: vec![RuleEffect {
                    id: key("extra"),
                    when: Some(key("enabled")),
                    effect: RuleEffectKind::Contribute {
                        entity: RuleEntity::Player,
                        stat: def(0x331b + stage),
                        contribution: kind,
                        value: key("amount"),
                    },
                }],
            });
    }
}
fn effects(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("{report:?}")
    };
    assert!(effects.gaps.is_empty(), "{:?}", effects.gaps);
    effects
}
fn value(report: &OwnedEffectsReport, stat: StatDefId) -> &EffectValue {
    &report
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: stat.clone(),
                }
        })
        .unwrap()
        .value
}
fn assert_outputs(report: &SupportEffectsReport, expected: [i64; 6]) {
    let r = effects(report);
    for i in 0..6 {
        let output = if i < 3 { 0x3321 + i } else { 0x1d2e + i - 3 };
        assert_eq!(
            value(r, def(output)),
            &EffectValue::Known {
                value: ParameterValue::Integer(BoundedInteger::new(expected[i as usize]).unwrap())
            }
        );
        assert_eq!(
            value(r, def(0x3324 + i)),
            &EffectValue::Known {
                value: ParameterValue::Quantity(FiniteQuantity::new(1., def(1)).unwrap())
            }
        );
    }
}
fn source_id(node: &PassiveNodeDefId) -> String {
    source()
        .contributors
        .iter()
        .find(|c| c["owner"]["value"]["value"] == json!(node))
        .unwrap()["source_node_id"]
        .as_u64()
        .unwrap()
        .to_string()
}
fn assert_increase_occurrences(
    world: &World,
    report: &SupportEffectsReport,
    stage: usize,
    expected: &[Value],
) {
    let r = effects(report);
    let stat = def(0x331b + stage as u64);
    let mut names = vec![];
    for effect in &r.effects {
        let BoundEffectTarget::Contribution { key: channel } = &effect.target else {
            continue;
        };
        if channel.stat != stat || channel.kind != ContributionKind::Increase {
            continue;
        }
        let RuleOrigin::Provider { provider } = &effect.key.invocation.origin else {
            panic!("direct allocation origin")
        };
        let ProviderRoot::Allocation(id) = provider.root else {
            panic!("allocation")
        };
        assert!(provider.grant_path.is_empty());
        let allocation = world.build.allocations.iter().find(|a| a.id == id).unwrap();
        assert_eq!(
            effect.key.invocation.owner,
            subject(allocation.node.clone())
        );
        assert_eq!(effect.key.invocation.program.as_str(), "passive-view");
        assert!(matches!(effect.value, EffectValue::Known { .. }));
        names.push(source_id(&allocation.node));
    }
    names.sort();
    let mut expected: Vec<_> = expected
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    expected.sort();
    assert_eq!(names, expected);
}

#[test]
#[ignore = "requires ATTRIBUTE_INCREASE_RELEASE"]
fn all_five_selected_inc_occurrences_match_source_with_actual_scalar_and_empty_more_receivers() {
    for (i, case) in source().cases.iter().enumerate() {
        let world = World::new(i);
        assert_eq!(world.build.allocations.len(), [1, 1, 1, 0, 0][i]);
        assert_eq!(world.build.allocations, source().builds[i].allocations);
        let expected = std::array::from_fn(|n| {
            let value = case["stages"][n]["result"].as_f64().unwrap();
            assert_eq!(value.fract(), 0.);
            value as i64
        });
        let report = world.report();
        assert_outputs(&report, expected);
        for stage in 0..6 {
            assert_increase_occurrences(
                &world,
                &report,
                stage,
                case["stages"][stage]["increase_sources"]
                    .as_array()
                    .unwrap(),
            );
        }
    }
}

#[test]
#[ignore = "requires ATTRIBUTE_INCREASE_RELEASE"]
fn entire_published_family_is_exact_under_order_permutations_and_opaque_id_rebasing() {
    let mut world = World::new(4);
    world.all_members();
    let expected = [35, 9, 128, 35, 9, 128];
    let baseline = world.report();
    assert_outputs(&baseline, expected);
    for i in 0..6 {
        let mut total = 0.;
        let mut count = 0;
        for e in &effects(&baseline).effects {
            if matches!(&e.target, BoundEffectTarget::Contribution { key } if key.stat == def(0x331b + i) && key.kind == ContributionKind::Increase)
            {
                let EffectValue::Known {
                    value: ParameterValue::Quantity(amount),
                } = &e.value
                else {
                    panic!("known INC")
                };
                total += amount.value();
                count += 1;
            }
        }
        assert_eq!(total, [29., 24., 22., 29., 24., 22.][i as usize]);
        assert_eq!(count, [7, 5, 5, 7, 5, 5][i as usize]);
    }
    world.build.allocations.reverse();
    world.recipe.rules.owners.reverse();
    world.recipe.rules.receivers.members.reverse();
    let registry = world.recipe.rules.contribution_queries.as_mut().unwrap();
    registry.members.reverse();
    for q in &mut registry.members {
        q.groups[0].members.members.reverse();
        if q.contribution == ContributionKind::Increase {
            for m in &mut q.groups[0].members.members {
                let order = m.order.as_mut().unwrap();
                order.source_rank = 10 - order.source_rank;
            }
        }
    }
    assert_outputs(&world.report(), expected);
    // IDs remain opaque: preserving all relations while changing lineage and
    // reversing allocation ID order cannot affect any numeric component.
    let lineage = BuildLineage::from_bytes([0x49; 16]);
    let old_loadouts = world.build.weapon_loadouts.clone();
    let replacement: Vec<_> = (0..old_loadouts.len())
        .map(|i| {
            WeaponLoadoutId::from_instance_id(
                InstanceId::from_parts(lineage, 10 + i as u64).unwrap(),
            )
        })
        .collect();
    let active = old_loadouts
        .iter()
        .position(|id| *id == world.build.active_weapon_loadout)
        .unwrap();
    world.build.weapon_loadouts = replacement.clone();
    world.build.active_weapon_loadout = replacement[active];
    for (i, a) in world.build.allocations.iter_mut().enumerate() {
        a.id = AllocationId::from_instance_id(
            InstanceId::from_parts(lineage, 100 - i as u64).unwrap(),
        );
        if let LoadoutScope::Selected { loadouts } = &mut a.scope {
            for id in loadouts {
                *id = replacement[old_loadouts.iter().position(|old| old == id).unwrap()];
            }
        }
    }
    world.build.allocator = InstanceAllocatorState::from_parts(lineage, 1000);
    assert_outputs(&world.report(), expected);
}

#[test]
#[ignore = "requires ATTRIBUTE_INCREASE_RELEASE"]
fn undeclared_potentials_and_duplicate_semantic_positions_are_rejected_before_values() {
    for stage in 0..6 {
        for (amount, enabled) in [
            (Some(1.), true),
            (Some(1.), false),
            (Some(0.), true),
            (None, true),
            (None, false),
        ] {
            let mut world = World::new(4);
            world.potential(stage, amount, enabled, ContributionKind::Increase);
            assert!(matches!(world.plan(), Err(PlanError::Invalid(message))
                if message == "actual contribution has no ordered membership"));
        }
        // The actual empty-MORE receiver must still reject a new multiplier.
        let mut world = World::new(4);
        world.potential(stage, Some(1.), false, ContributionKind::Multiply);
        assert!(matches!(world.plan(), Err(PlanError::Invalid(message))
            if message == "actual contribution has no ordered membership"));
    }
    let mut duplicate = World::new(0);
    let mut allocation = duplicate.build.allocations[0].clone();
    allocation.id = AllocationId::from_instance_id(
        InstanceId::from_parts(duplicate.build.allocator.lineage(), 90_000).unwrap(),
    );
    duplicate.build.allocator =
        InstanceAllocatorState::from_parts(duplicate.build.allocator.lineage(), 100_000);
    duplicate.build.allocations.push(allocation);
    assert!(matches!(duplicate.plan(), Err(PlanError::Invalid(message))
        if message == "ordered contribution semantic positions are tied"));
}

#[test]
#[ignore = "requires ATTRIBUTE_INCREASE_RELEASE"]
fn missing_members_and_actual_partial_domains_are_never_promoted_to_zero() {
    let mut missing_owner = World::new(0);
    missing_owner
        .recipe
        .rules
        .owners
        .retain(|o| o.owner != source().owners[0].owner);
    let schema =
        OwnedDefinitionSchemaPackage::new(missing_owner.recipe.schema.clone(), Default::default())
            .unwrap();
    missing_owner.recipe.rules.definitions = schema.identity().clone();
    let Err(error) = poe_optimizer_data::owned_rules::OwnedRulePackage::new(
        missing_owner.recipe.rules.clone(),
        &schema,
        Default::default(),
    ) else {
        panic!("declared member lost its actual owner");
    };
    assert!(
        error
            .to_string()
            .contains("ordered member references an unknown producer program"),
        "{error}"
    );
    let mut missing_member = World::new(0);
    let registry = missing_member
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap();
    registry
        .members
        .iter_mut()
        .find(|q| q.stat == def(0x331d) && q.contribution == ContributionKind::Increase)
        .unwrap()
        .groups[0]
        .members
        .members
        .retain(|m| m.owner != source().owners[0].owner);
    assert!(
        matches!(missing_member.plan(), Err(PlanError::Invalid(message))
        if message == "actual contribution has no ordered membership")
    );
    for domain in ["base", "global", "class", "inc"] {
        let mut world = World::new(0);
        if domain == "class" {
            let owner = subject(world.build.character.class.clone());
            let actual = actual_owner(&owner);
            assert!(!actual.programs.is_complete());
            world.owner_mut(&owner).programs.closure = actual.programs.closure.clone();
        } else {
            let prior: family::Dependencies = family::read("dependencies.json");
            let registry = world.recipe.rules.contribution_queries.as_mut().unwrap();
            if domain == "global" {
                registry.closure = source()
                    .recipe
                    .rules
                    .contribution_queries
                    .as_ref()
                    .unwrap()
                    .closure
                    .clone();
            } else {
                let kind = if domain == "base" {
                    ContributionKind::Add
                } else {
                    ContributionKind::Increase
                };
                for q in registry
                    .members
                    .iter_mut()
                    .filter(|q| q.contribution == kind)
                {
                    q.groups[0].members.closure = prior
                        .query_registry_before
                        .members
                        .iter()
                        .find(|old| old.id == q.id)
                        .unwrap()
                        .groups[0]
                        .members
                        .closure
                        .clone();
                }
            }
        }
        let report = world.report();
        assert!(
            matches!(report.outcome, SupportEffectsOutcome::Unavailable { .. }),
            "{domain}"
        );
        assert!(
            report.gaps.iter().any(|g| g.reason
                == if domain == "class" {
                    PlanGapReason::PartialPrograms
                } else {
                    PlanGapReason::IncompleteContributors
                }),
            "{domain}"
        );
    }
}

#[test]
#[ignore = "requires ATTRIBUTE_INCREASE_RELEASE"]
fn loadout_activation_and_fresh_reused_parallel_evaluation_are_deterministic() {
    let mut a = World::new(0);
    assert_eq!(a.build.weapon_loadouts.len(), 2);
    let active = a.build.active_weapon_loadout;
    let inactive = *a
        .build
        .weapon_loadouts
        .iter()
        .find(|id| **id != active)
        .unwrap();
    a.build.allocations[0].scope = LoadoutScope::Selected {
        loadouts: vec![active],
    };
    let mut b = a.clone();
    b.build.active_weapon_loadout = inactive;
    let plans = [a.plan().unwrap(), b.plan().unwrap()];
    let expected: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    assert_outputs(&expected[0], [82, 32, 156, 82, 32, 156]);
    assert_outputs(&expected[1], [82, 32, 147, 82, 32, 147]);
    assert!(
        !effects(&expected[1])
            .effects
            .iter()
            .any(|e| e.key.invocation.program.as_str() == "passive-view")
    );
    let mut scratch = plans[0].new_scratch();
    for i in [0, 1, 0] {
        assert_eq!(plans[i].evaluate(&mut scratch).unwrap(), expected[i]);
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports: Vec<_> = pool.install(|| {
        (0..16)
            .into_par_iter()
            .map_init(
                || plans[0].new_scratch(),
                |scratch, i| (i % 2, plans[i % 2].evaluate(scratch).unwrap()),
            )
            .collect()
    });
    for (i, report) in reports {
        assert_eq!(report, expected[i]);
    }
    assert_eq!(a.report(), expected[0]);
}
