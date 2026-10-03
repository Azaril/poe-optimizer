//! Finite physical-occurrence proof using the published Frost supply/usage programs.
//! Usage records the requested global-effect setting, not source action presence.
//! The conditional reader is synthetic; no action, exposure, damage or build parity.
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod base;

use base::{key, occurrence};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_project::*, owned_routing::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting, owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{fs, path::PathBuf, sync::Arc};

type Plan = OwnedEffectPlan<OwnedDefinitionSchemaPackage>;

fn asset(name: &str) -> Value {
    serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/owned/poe2/3887ae68/frost-bomb-inputs")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Inputs {
    schema_version: u32,
    policy: UsagePolicyDefId,
    parameter: DeclaredSlot<ParameterSlotDefId>,
    requested_global_effect: StatDefId,
    gem: GemDefId,
    skill: SkillDefId,
    primary_supply: DeclaredSlot<SkillGrantSlotDefId>,
    entering_grant: DeclaredSlot<GrantSlotDefId>,
    supply_program: OwnedDefinitionKey,
    supply_effect: OwnedDefinitionKey,
    program: OwnedDefinitionKey,
    effect: OwnedDefinitionKey,
}

fn inputs() -> Inputs {
    serde_json::from_value(asset("native-inputs.json")).unwrap()
}

fn named<K: DefinitionDomain>(name: &str) -> DefId<K> {
    DefId::parse(
        inputs().gem.namespace().clone(),
        format!("fixture.frost.{name}"),
    )
    .unwrap()
}

fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}

// Reuse the generic fixture's unrelated character/encounter skeleton in the
// authored namespace. This never alters game-data closures or program bodies.
fn namespace<T: Serialize + DeserializeOwned>(source: &T, ns: &GameVersionNamespace) -> T {
    fn walk(value: &mut Value, ns: &GameVersionNamespace) {
        match value {
            Value::Object(fields) if fields.get("game") == Some(&json!("owned-plan-test")) => {
                *value = serde_json::to_value(ns).unwrap();
            }
            Value::Object(fields) => fields.values_mut().for_each(|v| walk(v, ns)),
            Value::Array(values) => values.iter_mut().for_each(|v| walk(v, ns)),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(source).unwrap();
    walk(&mut value, ns);
    serde_json::from_value(value).unwrap()
}

fn finite_ports(ports: &mut DeclaredSlots) {
    // Only the two named component definitions call this helper. Their actual
    // members remain unchanged; the finite test excludes all other mechanics.
    ports.parameters.closure = SchemaClosure::Complete;
    ports.choices.closure = SchemaClosure::Complete;
    ports.grants.closure = SchemaClosure::Complete;
    ports.actors.closure = SchemaClosure::Complete;
    ports.skill_grants.closure = SchemaClosure::Complete;
    ports.outputs.closure = SchemaClosure::Complete;
    ports.sockets.closure = SchemaClosure::Complete;
}

fn provider(index: usize) -> ProviderKey {
    ProviderKey {
        root: ProviderRoot::SkillUse(occurrence(30 + index as u64)),
        grant_path: vec![],
    }
}

fn target(index: usize) -> SkillTarget {
    SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: provider(index),
        slot: inputs().primary_supply,
    }))
}

fn usage(index: usize, enabled: bool) -> UsagePolicySelection {
    let input = inputs();
    UsagePolicySelection {
        policy: input.policy,
        target: UsageTarget::Skill(target(index)),
        parameters: vec![ParameterAssignment {
            slot: input.parameter,
            value: ParameterValue::Boolean(enabled),
        }],
    }
}

struct World {
    f: base::Fixture,
    preferences: Vec<UsagePolicySelection>,
}

impl World {
    fn new(requested: [Option<bool>; 2], finite: bool) -> Self {
        let input = inputs();
        assert_eq!(input.schema_version, 1);
        let migration = asset("migration.json");
        let ns = input.gem.namespace().clone();
        let mut f = base::Fixture::new();
        f.schema = namespace(&f.schema, &ns);
        f.schema.schema_version = migration["contract"]["schema_version"].as_u64().unwrap() as u32;
        f.build = namespace(&f.build, &ns);
        f.scenario = namespace(&f.scenario, &ns);
        f.queries = namespace(&f.queries, &ns);
        f.build.items.clear();
        f.build.equipment.clear();
        f.owners.clear();
        f.routes.clear();
        f.tables.clear();
        f.receivers = DeclaredSet::complete(vec![]);
        for row in migration["schema"].as_array().unwrap() {
            match row["kind"].as_str().unwrap() {
                "definition" => f
                    .schema
                    .definitions
                    .push(serde_json::from_value(row["value"].clone()).unwrap()),
                "slot" => f
                    .schema
                    .slots
                    .push(serde_json::from_value(row["value"].clone()).unwrap()),
                other => panic!("unexpected schema row {other}"),
            }
        }
        let dependencies: Value = serde_json::from_str(include_str!(
            "support/frost_bomb_physical_dependencies.json"
        ))
        .unwrap();
        assert_eq!(
            dependencies["source"]["release"],
            "pob-3887ae68-direct-skill-inputs-v1"
        );
        f.schema.definitions.extend(
            serde_json::from_value::<Vec<DefinitionDescriptor>>(
                dependencies["definitions"].clone(),
            )
            .unwrap(),
        );
        f.schema.slots.extend(
            serde_json::from_value::<Vec<SlotDescriptor>>(dependencies["slots"].clone()).unwrap(),
        );
        f.owners = serde_json::from_value(migration["owners"].clone()).unwrap();
        assert!(migration["tables"].as_array().unwrap().is_empty());
        assert!(migration["receivers"].as_array().unwrap().is_empty());
        if finite {
            for descriptor in &mut f.schema.definitions {
                match descriptor {
                    DefinitionDescriptor::Gem(e) if e.id == input.gem => {
                        let SchemaState::Known(gem) = &mut e.schema else {
                            panic!("known physical Gem")
                        };
                        gem.skills.closure = SchemaClosure::Complete;
                        finite_ports(&mut gem.declarations);
                    }
                    DefinitionDescriptor::Skill(e) if e.id == input.skill => {
                        let SchemaState::Known(skill) = &mut e.schema else {
                            panic!("known primary Skill")
                        };
                        finite_ports(&mut skill.declarations);
                    }
                    _ => {}
                }
            }
            for descriptor in &mut f.schema.slots {
                if let SlotDescriptor::SkillGrant(e) = descriptor
                    && e.id == input.primary_supply
                {
                    let SchemaState::Known(supply) = &mut e.schema else {
                        panic!("known primary supply")
                    };
                    supply.outputs.closure = SchemaClosure::Complete;
                }
            }
            for owner in &mut f.owners {
                if owner.owner == SchemaSubject::Definition(input.gem.address()) {
                    owner.programs.closure = SchemaClosure::Complete;
                }
            }
        }
        for owner in [
            SchemaSubject::Definition(f.build.character.class.address()),
            SchemaSubject::Definition(f.scenario.enemy.encounter.address()),
            SchemaSubject::Slot(GrantSlotDefId::address(&input.entering_grant)),
        ] {
            f.owners.push(DefinitionRules {
                owner,
                programs: DeclaredSet::complete(vec![]),
            });
        }
        f.schema.definitions.push(DefinitionDescriptor::Stat(entry(
            named("conditional-marker"),
            StatSchema {
                value: ComputedValueType::Boolean,
                targets: vec![RuleEntityKind::Skill],
            },
        )));
        // This synthetic reader conditionally emits a marker for the requested
        // setting. It makes no assertion about real Frost action presence or
        // exposure, including cold MAIN/CALCS lifecycle differences in PoB.
        f.owners.push(DefinitionRules {
            owner: SchemaSubject::Definition(input.skill.address()),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key("synthetic-conditional-consumer"),
                context: RuleEntityKind::Skill,
                reads: vec![RuleRead {
                    id: key("requested"),
                    value_type: ComputedValueType::Boolean,
                    source: RuleReadSource::Stat {
                        entity: RuleEntity::Current,
                        stat: input.requested_global_effect.clone(),
                    },
                }],
                nodes: vec![
                    RuleNode {
                        id: key("requested"),
                        expression: RuleExpression::Read {
                            input: key("requested"),
                        },
                    },
                    RuleNode {
                        id: key("marker"),
                        expression: RuleExpression::Literal {
                            value: ParameterValue::Boolean(true),
                        },
                    },
                ],
                effects: vec![RuleEffect {
                    id: key("conditional-marker"),
                    when: Some(key("requested")),
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: named("conditional-marker"),
                        value: key("marker"),
                    },
                }],
            }]),
        });
        let physical = asset("inventory.json")["physical"].clone();
        let corrupt: DeclaredSlot<ParameterSlotDefId> =
            serde_json::from_value(physical["corrupted"].clone()).unwrap();
        let delta: DeclaredSlot<ParameterSlotDefId> =
            serde_json::from_value(physical["corruption_level"].clone()).unwrap();
        let count_unit: UnitDefId = serde_json::from_value(dependencies["slots"][1]["value"]["schema"]["value"]["value"]["value"]["minimum"]["unit"].clone()).unwrap();
        for index in 0..2 {
            let gem_id = occurrence(20 + index);
            f.build.gems.push(GemInstance {
                id: gem_id,
                definition: input.gem.clone(),
                level: 20,
                quality: None,
                parameters: vec![
                    ParameterAssignment {
                        slot: corrupt.clone(),
                        value: ParameterValue::Boolean(false),
                    },
                    ParameterAssignment {
                        slot: delta.clone(),
                        value: ParameterValue::Quantity(
                            FiniteQuantity::new(0., count_unit.clone()).unwrap(),
                        ),
                    },
                ],
            });
            f.build.skills.push(SkillUse {
                id: occurrence(30 + index),
                source: AuthoredSkillSource::Gem(gem_id),
                enabled: true,
                scope: LoadoutScope::Shared,
                parameters: None,
            });
        }
        Self {
            f,
            preferences: requested
                .into_iter()
                .enumerate()
                .filter_map(|(i, enabled)| enabled.map(|enabled| usage(i, enabled)))
                .collect(),
        }
    }

    fn request(&self) -> OwnedEvaluationRequest {
        let b = &self.f.build;
        let limits = OwnedInputLimits::default();
        let selection = VariantSelection {
            character: occurrence(50),
            equipment: occurrence(51),
            allocations: occurrence(52),
            skills: occurrence(53),
            choices: occurrence(54),
            active_weapon_loadout: b.active_weapon_loadout,
        };
        let project = BuildProject::new(
            ProjectInput {
                allocator: b.allocator,
                revision: b.revision,
                game_version: b.game_version.clone(),
                weapon_loadouts: b.weapon_loadouts.clone(),
                items: vec![],
                gems: b.gems.clone(),
                rewards: vec![],
                equipment: vec![],
                allocations: vec![],
                skills: b.skills.clone(),
                supports: vec![],
                payload_links: vec![],
                character_presets: vec![CharacterPreset {
                    id: selection.character,
                    class: b.character.class.clone(),
                    ascendancy: None,
                    level: b.character.level,
                    rewards: vec![],
                }],
                equipment_presets: vec![EquipmentPreset {
                    id: selection.equipment,
                    equipment: vec![],
                }],
                allocation_presets: vec![AllocationPreset {
                    id: selection.allocations,
                    allocations: vec![],
                    equipment: vec![],
                }],
                skill_presets: vec![SkillPreset {
                    id: selection.skills,
                    skills: b.skills.iter().map(|s| s.id).collect(),
                    supports: vec![],
                    support_origins: None,
                    payload_links: vec![],
                    usage_preferences: Some(self.preferences.clone()),
                }],
                choice_presets: vec![ChoicePreset {
                    id: selection.choices,
                    choices: vec![],
                    rewards: vec![],
                }],
                saved_variants: vec![],
            },
            limits,
        )
        .unwrap();
        compose_request(
            &project,
            &selection,
            None,
            ScenarioSpec::new(self.f.scenario.clone(), limits).unwrap(),
            QuerySpec::new(self.f.queries.clone(), limits).unwrap(),
            limits,
        )
        .unwrap()
    }

    fn compile(&self, operations: &str) -> Result<Plan> {
        let schema = Arc::new(
            OwnedDefinitionSchemaPackage::new(self.f.schema.clone(), Default::default()).unwrap(),
        );
        let rules = Arc::new(
            CompiledRulePackage::compile(
                &RulePackageInput {
                    schema_version: OWNED_RULE_PACKAGE_VERSION,
                    namespace: schema.input().namespace.clone(),
                    release: key("finite-frost-inputs"),
                    semantics_version: key("finite-component-only"),
                    operations_version: key(operations),
                    definitions: schema.identity().clone(),
                    owners: self.f.owners.clone(),
                    tables: vec![],
                    receivers: DeclaredSet::complete(vec![]),
                    effect_applications: Some(DeclaredSet::complete(vec![])),
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
                    namespace: schema.input().namespace.clone(),
                    release: key("no-real-action-output"),
                    definitions: schema.identity().clone(),
                    outputs: vec![],
                },
                schema.as_ref(),
                Default::default(),
            )
            .unwrap(),
        );
        OwnedEffectPlan::compile(
            Arc::new(self.request()),
            schema,
            rules,
            routing,
            Default::default(),
        )
    }

    fn plan(&self) -> Plan {
        self.compile(OWNED_RULE_OPERATIONS_V15).unwrap()
    }
    fn evaluate(&self) -> OwnedEffectsReport {
        let plan = self.plan();
        plan.evaluate(&mut plan.new_scratch()).unwrap()
    }
}

fn entity(index: usize) -> ConcreteEntity {
    ConcreteEntity::Skill(Box::new(target(index)))
}
fn value(report: &OwnedEffectsReport, index: usize, stat: StatDefId) -> &EffectValue {
    &report
        .values
        .iter()
        .find(|row| {
            row.key
                == PlanValueKey::Stat {
                    entity: entity(index),
                    stat: stat.clone(),
                }
        })
        .expect("exact occurrence stat")
        .value
}
fn requested_setting(report: &OwnedEffectsReport, index: usize) -> Option<&BoundEffectResult> {
    let input = inputs();
    let rows: Vec<_> = report
        .effects
        .iter()
        .filter(|row| {
            row.key.invocation.owner == SchemaSubject::Definition(input.policy.address())
                && row.key.invocation.program == input.program
                && row.key.effect == input.effect
                && row.key.invocation.entity == entity(index)
        })
        .collect();
    assert!(rows.len() <= 1);
    rows.first().copied()
}
fn check(report: &OwnedEffectsReport, expected: [bool; 2]) {
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert!(report.application_groups.is_empty());
    let input = inputs();
    for (index, enabled) in expected.into_iter().enumerate() {
        let requested = requested_setting(report, index).unwrap();
        assert!(matches!(
            requested.key.invocation.origin,
            RuleOrigin::Usage { .. }
        ));
        assert_eq!(
            requested.value,
            EffectValue::Known {
                value: ParameterValue::Boolean(enabled)
            }
        );
        assert_eq!(
            value(report, index, input.requested_global_effect.clone()),
            &requested.value
        );
        assert_eq!(
            value(report, index, named("conditional-marker")),
            &if enabled {
                EffectValue::Known {
                    value: ParameterValue::Boolean(true),
                }
            } else {
                EffectValue::Inactive
            }
        );
        let supplies: Vec<_> = report
            .effects
            .iter()
            .filter(|row| {
                row.key.invocation.program == input.supply_program
                    && row.key.effect == input.supply_effect
                    && row.key.invocation.origin
                        == RuleOrigin::Provider {
                            provider: provider(index),
                        }
            })
            .collect();
        assert_eq!(supplies.len(), 1);
        assert_eq!(
            supplies[0].key.invocation.owner,
            SchemaSubject::Definition(input.gem.address())
        );
        assert_eq!(
            supplies[0].value,
            EffectValue::Known {
                value: ParameterValue::Boolean(true)
            },
            "a requested global-effect setting does not invent or revoke structural supply"
        );
    }
}

#[test]
fn authored_frost_usage_and_synthetic_consumer_keep_two_physical_copies_independent() {
    let mut world = World::new([Some(true), Some(false)], true);
    assert_eq!(world.request().scenario().input().usage, world.preferences);
    check(&world.evaluate(), [true, false]);
    world.preferences = vec![usage(0, false), usage(1, true)];
    check(&world.evaluate(), [false, true]);
    world.f.scenario.usage = vec![usage(0, true)];
    assert_eq!(
        world.request().scenario().input().usage,
        vec![usage(0, true), usage(1, true)]
    );
    check(&world.evaluate(), [true, true]);
}

#[test]
fn missing_preference_stays_unresolved_and_disabled_occurrence_is_not_reactivated() {
    let world = World::new([None, Some(true)], true);
    let missing = world.evaluate();
    assert!(requested_setting(&missing, 0).is_none());
    assert!(matches!(
        value(&missing, 0, named("conditional-marker")),
        EffectValue::Unresolved { .. }
    ));
    assert_eq!(
        requested_setting(&missing, 1).unwrap().value,
        EffectValue::Known {
            value: ParameterValue::Boolean(true)
        }
    );
    let mut world = World::new([Some(true), Some(true)], true);
    world.f.build.skills[0].enabled = false;
    let disabled = world.evaluate();
    assert!(requested_setting(&disabled, 0).is_none());
    assert!(
        !disabled
            .effects
            .iter()
            .any(|row| row.key.invocation.entity == entity(0))
    );
    assert_eq!(
        requested_setting(&disabled, 1).unwrap().value,
        EffectValue::Known {
            value: ParameterValue::Boolean(true)
        }
    );
}

#[test]
fn published_partial_declarations_and_operation17_readiness_are_not_upgraded_by_the_fixture() {
    let migration = asset("migration.json");
    assert_eq!(
        migration["contract"]["operations_version"],
        OWNED_RULE_OPERATIONS_V17
    );
    assert!(migration.get("evaluation").is_none());
    let world = World::new([Some(true); 2], false);
    let input = inputs();
    for descriptor in &world.f.schema.definitions {
        match descriptor {
            DefinitionDescriptor::Skill(e) if e.id == input.skill => {
                let SchemaState::Known(skill) = &e.schema else {
                    panic!("known partial skill")
                };
                assert!(!skill.declarations.parameters.is_complete());
                assert!(!skill.declarations.outputs.is_complete());
                assert!(skill.declarations.parameters.members.is_empty());
            }
            DefinitionDescriptor::Gem(e) if e.id == input.gem => {
                let SchemaState::Known(gem) = &e.schema else {
                    panic!("known partial gem")
                };
                assert!(!gem.skills.is_complete());
            }
            _ => {}
        }
    }
    assert!(
        matches!(world.compile(OWNED_RULE_OPERATIONS_V17), Err(PlanError::Invalid(message)) if message == "operation v16 requires checked readiness stages")
    );
    let partial = world.evaluate();
    assert!(!partial.gaps.is_empty());
    for index in 0..2 {
        assert!(!matches!(
            value(&partial, index, named("conditional-marker")),
            EffectValue::Known { .. }
        ));
    }
}

fn evaluate(plan: &Plan, scratch: &mut OwnedPlanScratch) -> OwnedEffectsReport {
    plan.evaluate(scratch).unwrap()
}

#[test]
fn actual_programs_reuse_scratch_across_requested_setting_changes_and_unresolved_attempts() {
    let a = World::new([Some(true), Some(false)], true).plan();
    let b = World::new([Some(false), Some(true)], true).plan();
    assert_ne!(a.identity(), b.identity());
    let expected = [
        evaluate(&a, &mut a.new_scratch()),
        evaluate(&b, &mut b.new_scratch()),
    ];
    check(&expected[0], [true, false]);
    check(&expected[1], [false, true]);
    let mut scratch = a.new_scratch();
    for (plan, expected) in [(&a, &expected[0]), (&b, &expected[1]), (&a, &expected[0])] {
        assert!(
            evaluate(plan, &mut scratch) == *expected,
            "A/B/A report changed"
        );
    }
    let missing = World::new([None, Some(true)], true).plan();
    let expected_missing = evaluate(&missing, &mut missing.new_scratch());
    assert!(matches!(
        value(&expected_missing, 0, named("conditional-marker")),
        EffectValue::Unresolved { .. }
    ));
    assert!(evaluate(&missing, &mut scratch) == expected_missing);
    assert!(
        evaluate(&b, &mut scratch) == expected[1],
        "unresolved attempt contaminated B"
    );
    assert!(
        evaluate(&a, &mut scratch) == expected[0],
        "unresolved attempt contaminated A"
    );
}

#[test]
fn rayon_workers_privately_reuse_frost_plans_and_preserve_complete_reports() {
    let plans = Arc::new([
        World::new([Some(true), Some(false)], true).plan(),
        World::new([Some(false), Some(true)], true).plan(),
    ]);
    let expected = [
        evaluate(&plans[0], &mut plans[0].new_scratch()),
        evaluate(&plans[1], &mut plans[1].new_scratch()),
    ];
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports: Vec<_> = pool.install(|| {
        (0..8)
            .into_par_iter()
            .map_init(
                || plans[0].new_scratch(),
                |scratch, batch| {
                    (0..9)
                        .map(|step| {
                            let index = (batch + step) % 2;
                            (index, evaluate(&plans[index], scratch))
                        })
                        .collect::<Vec<_>>()
                },
            )
            .collect()
    });
    for batch in reports {
        assert_eq!(batch.len(), 9);
        for (index, report) in batch {
            assert!(
                report == expected[index],
                "worker-local reuse changed values or provenance"
            );
        }
    }
}
