//! Real authored Sniper ability inputs, independently of PoB runtime or UI state.
//! This deliberately builds a component index, not a complete release or build.
//! The CLI publication test owns the exact release/5-original/110-query contract.
use poe_optimizer_core::{
    build_identity::*, owned_binding::*, owned_build::*, owned_definitions::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting,
    owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits},
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::*};
use poe_optimizer_import::{
    owned_recipe::OwnedRecipeInput, owned_recipe_extension::SchemaExtensionEntry,
    owned_release_migration::OwnedReleaseMigrationInput,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{fs, path::PathBuf, sync::Arc};

fn key(name: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(name).unwrap()
}
fn integer(value: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(value).unwrap())
}
fn load<T: DeserializeOwned>(path: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/owned/poe2/3887ae68")
                .join(path),
        )
        .unwrap(),
    )
    .unwrap()
}
fn occurrence<T: BuildInstanceId>(id: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([114; 16]), id).unwrap())
}
struct Real {
    schema: Arc<OwnedDefinitionSchemaPackage>,
    rules: Arc<CompiledRulePackage>,
    routing: Arc<OwnedActionRouting>,
    ids: Value,
    actor: ActorDefId,
}
struct Ability {
    skill: SkillDefId,
    supply: DeclaredSlot<SkillGrantSlotDefId>,
    activation: DeclaredSlot<GrantSlotDefId>,
    output: DeclaredSlot<ActionOutputDefId>,
    level: DeclaredSlot<ParameterSlotDefId>,
    quality: DeclaredSlot<ParameterSlotDefId>,
    actor_level: DeclaredSlot<ParameterSlotDefId>,
}
impl Real {
    fn load() -> Self {
        let base: OwnedRecipeInput = load("recipe.json");
        let migration: OwnedReleaseMigrationInput = load("actor-ability-supply/migration.json");
        let mut schema = base.schema.clone();
        schema.schema_version = migration.contract.schema_version;
        schema.semantics_version = migration.contract.schema_semantics_version.clone();
        schema.release = migration.release.clone();
        for replacement in &migration.schema {
            match replacement {
                SchemaExtensionEntry::Definition(row) => {
                    if let Some(prior) = schema
                        .definitions
                        .iter_mut()
                        .find(|d| d.address() == row.address())
                    {
                        *prior = row.clone();
                    } else {
                        schema.definitions.push(row.clone());
                    }
                }
                SchemaExtensionEntry::Slot(row) => {
                    if let Some(prior) = schema
                        .slots
                        .iter_mut()
                        .find(|d| d.address() == row.address())
                    {
                        *prior = row.clone();
                    } else {
                        schema.slots.push(row.clone());
                    }
                }
            }
        }
        let schema = Arc::new(
            OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap(),
        );
        let mut rules = base.rules.clone();
        rules.operations_version = migration.contract.operations_version.clone();
        rules.semantics_version = migration.contract.rule_semantics_version.clone();
        rules.release = migration.release.clone();
        rules.definitions = schema.identity().clone();
        // The real migration adds Actor programs without weakening any prior owner.
        for owner in migration.owners {
            assert!(!rules.owners.iter().any(|prior| prior.owner == owner.owner));
            rules.owners.push(owner);
        }
        rules.tables.extend(migration.tables);
        rules.receivers.members.extend(migration.receivers);
        for owner in &base.rules.owners {
            assert_eq!(
                rules.owners.iter().find(|row| row.owner == owner.owner),
                Some(owner)
            );
        }
        assert_eq!(rules.receivers.closure, base.rules.receivers.closure);
        let rules = Arc::new(
            CompiledRulePackage::compile(&rules, schema.as_ref(), Default::default()).unwrap(),
        );
        let mut routing = base.routing;
        routing.definitions = schema.identity().clone();
        let routing = Arc::new(
            OwnedActionRouting::new(routing, schema.as_ref(), Default::default()).unwrap(),
        );
        let ids: Value = load("ids.json");
        let population: DeclaredSlot<ActorSlotDefId> =
            serde_json::from_value(ids["allocations"]["sniper-population"].clone()).unwrap();
        let SchemaLookup::Known(population_schema) = schema.slot(&population) else {
            panic!()
        };
        let actor = population_schema.provider_definition.clone().unwrap();
        assert_eq!(actor.key().as_str(), "def.0000000000003091");
        Self {
            schema,
            rules,
            routing,
            ids,
            actor,
        }
    }
    fn id<T: DeserializeOwned>(&self, name: &str) -> T {
        serde_json::from_value(self.ids["allocations"][name].clone()).unwrap()
    }
    fn quantity(&self, value: f64) -> ParameterValue {
        ParameterValue::Quantity(FiniteQuantity::new(value, self.id("percentage-points")).unwrap())
    }
    fn actor_owner(&self) -> SchemaSubject {
        SchemaSubject::Definition(self.actor.address())
    }
    fn abilities(&self) -> Vec<Ability> {
        let SchemaLookup::Known(actor) = self.schema.definition(&self.actor) else {
            panic!()
        };
        [("basic-attack", 0x3096), ("gas-arrow", 0x3099)].into_iter().map(|(name, first)| {
            let skill: SkillDefId = self.id(&format!("sniper-{name}-skill"));
            let supply = actor.declarations.skill_grants.members.iter().find(|slot| matches!(self.schema.slot(slot), SchemaLookup::Known(schema) if schema.skill == skill)).unwrap().clone();
            let activation = actor.declarations.grants.members.iter().find(|slot| matches!(self.schema.slot(slot), SchemaLookup::Known(schema) if schema.target == GrantTarget::Skill(supply.clone()))).unwrap().clone();
            let parameter = |suffix| DeclaredSlot {
                declaration: SlotOwnerDefId::Skill(skill.clone()),
                slot: ParameterSlotDefId::parse(self.schema.namespace().clone(), format!("def.{suffix:016x}")).unwrap(),
            };
            Ability { level: parameter(first), quality: parameter(first + 1), actor_level: parameter(first + 2),
                skill, supply, activation, output: self.id(&format!("sniper-{name}-output")) }
        }).collect()
    }
    fn summoner(&self, use_id: u64) -> ProviderKey {
        ProviderKey {
            root: ProviderRoot::SkillUse(occurrence(use_id)),
            grant_path: vec![self.id("sniper-primary-grant")],
        }
    }
    fn actor_key(&self, use_id: u64) -> ActorKey {
        ActorKey::Owned(Box::new(OwnedActorKey {
            provider: self.summoner(use_id),
            slot: self.id("sniper-population"),
        }))
    }
    fn actor_provider(&self, use_id: u64) -> ProviderKey {
        let mut provider = self.summoner(use_id);
        provider.grant_path.push(self.id("sniper-population-grant"));
        provider
    }
    fn action(&self, use_id: u64, ability: &Ability) -> ActionSelection {
        let mut provider = self.actor_provider(use_id);
        provider.grant_path.push(ability.activation.clone());
        ActionSelection {
            action: ActionKey {
                actor: self.actor_key(use_id),
                provider,
                output: ability.output.clone(),
            },
            part: self.id("ordinary-part"),
            mode: self.id("ordinary-mode"),
            stat_set: self.id("primary-stat-set"),
        }
    }
    fn request(&self) -> OwnedEvaluationRequest {
        let ns = self.schema.namespace().clone();
        let limits = OwnedInputLimits::default();
        let build = BuildSpec::new(
            BuildInput {
                generated_inputs: None,
                support_origins: None,
                allocator: InstanceAllocatorState::from_parts(
                    BuildLineage::from_bytes([114; 16]),
                    30,
                ),
                revision: BuildRevision::from_u64(1),
                game_version: ns.clone(),
                character: CharacterSpec {
                    class: ClassDefId::parse(ns.clone(), "unmapped-test-class").unwrap(),
                    ascendancy: None,
                    level: 100,
                    rewards: vec![],
                },
                weapon_loadouts: vec![occurrence(1)],
                active_weapon_loadout: occurrence(1),
                items: vec![],
                gems: [(2, 20), (3, 22)]
                    .into_iter()
                    .map(|(id, level)| GemInstance {
                        id: occurrence(id),
                        definition: self.id("sniper-gem"),
                        parameters: vec![],
                        level,
                        quality: Some(QualitySelection {
                            kind: self.id("standard-quality"),
                            amount: FiniteQuantity::new(20.0, self.id("percentage-points"))
                                .unwrap(),
                        }),
                    })
                    .collect(),
                equipment: vec![],
                allocations: vec![],
                skills: [(12, 2), (13, 3)]
                    .into_iter()
                    .map(|(id, gem)| SkillUse {
                        parameters: None,
                        id: occurrence(id),
                        source: AuthoredSkillSource::Gem(occurrence(gem)),
                        enabled: true,
                        scope: LoadoutScope::Shared,
                    })
                    .collect(),
                supports: vec![],
                payload_links: vec![],
                choices: vec![],
            },
            limits,
        )
        .unwrap();
        let scenario = ScenarioSpec::new(
            ScenarioInput {
                game_version: ns.clone(),
                enemy: EnemySpec {
                    encounter: EncounterDefId::parse(ns.clone(), "unmapped-test-encounter")
                        .unwrap(),
                    level: 1,
                },
                assumptions: vec![],
                usage: vec![],
            },
            limits,
        )
        .unwrap();
        let requests = [12, 13]
            .into_iter()
            .flat_map(|use_id| {
                self.abilities()
                    .into_iter()
                    .map(move |ability| MetricRequest {
                        id: QueryId::new(format!("{use_id}-{}", ability.skill.key())).unwrap(),
                        metric: MetricDefId::parse(
                            self.schema.namespace().clone(),
                            "unmapped-test-metric",
                        )
                        .unwrap(),
                        target: MetricTarget::Action(Box::new(self.action(use_id, &ability))),
                    })
            })
            .collect();
        let queries = QuerySpec::new(
            QueryInput {
                game_version: ns,
                requests,
            },
            limits,
        )
        .unwrap();
        OwnedEvaluationRequest::new(build, scenario, queries, limits).unwrap()
    }
}

#[test]
fn real_actor_programs_keep_summoning_actor_and_ability_inputs_separate() {
    let r = Real::load();
    let owner = r.actor_owner();
    let actor_programs = &r
        .rules
        .input()
        .owners
        .iter()
        .find(|row| row.owner == owner)
        .unwrap()
        .programs
        .members;
    let mut scratch = r.rules.new_scratch();
    for (summoning_level, actor_level) in [(20, 40), (22, 44)] {
        let summoning = r
            .rules
            .evaluate(
                &SchemaSubject::Definition(r.id::<SkillDefId>("sniper-skill").address()),
                &key("ordinary-population-inputs"),
                &[
                    RuleFact {
                        read: key("level"),
                        value: integer(summoning_level),
                    },
                    RuleFact {
                        read: key("quality"),
                        value: r.quantity(20.0),
                    },
                    RuleFact {
                        read: key("character-level"),
                        value: integer(100),
                    },
                ],
                r.schema.as_ref(),
                &mut scratch,
            )
            .unwrap();
        let projected = summoning.effects.iter().find(|e| matches!(&e.effect, RuleEffectKind::ProjectActorStat { stat, .. } if stat == &r.id("sniper-actor-level"))).unwrap();
        assert_eq!(
            projected.disposition,
            EffectDisposition::Applied {
                value: integer(actor_level)
            }
        );
        let mut actual = Vec::new();
        for program in actor_programs {
            let facts: Vec<_> = program.reads.iter().map(|read| {
                assert!(matches!(&read.source, RuleReadSource::Stat { entity: RuleEntity::Actor | RuleEntity::Current, stat } if stat == &r.id("sniper-actor-level")));
                assert_eq!(read.value_type, ComputedValueType::Integer);
                RuleFact { read: read.id.clone(), value: integer(actor_level) }
            }).collect();
            actual.extend(
                r.rules
                    .evaluate(&owner, &program.id, &facts, r.schema.as_ref(), &mut scratch)
                    .unwrap()
                    .effects,
            );
        }
        for ability in r.abilities() {
            for (parameter, expected) in [
                (&ability.level, integer(1)),
                (&ability.quality, r.quantity(0.0)),
                (&ability.actor_level, integer(actor_level)),
            ] {
                let effect = actual.iter().find(|e| matches!(&e.effect, RuleEffectKind::ProjectSkillParameter { skill, parameter: target, .. } if skill == &ability.supply && target == parameter)).unwrap();
                assert_eq!(
                    effect.disposition,
                    EffectDisposition::Applied { value: expected }
                );
            }
            let effect = actual.iter().find(|e| matches!(&e.effect, RuleEffectKind::ActivateGrant { slot, .. } if slot == &ability.activation)).unwrap();
            assert_eq!(
                effect.disposition,
                EffectDisposition::Applied {
                    value: ParameterValue::Boolean(true)
                }
            );
        }
    }
    // Missing actor-level evidence cannot be supplied by the summoning level or
    // positive quality: only explicitly dependent projections become unresolved.
    for program in actor_programs {
        let report = r
            .rules
            .evaluate(&owner, &program.id, &[], r.schema.as_ref(), &mut scratch)
            .unwrap();
        for ability in r.abilities() {
            for effect in &report.effects {
                if let RuleEffectKind::ProjectSkillParameter { parameter, .. } = &effect.effect {
                    if parameter == &ability.actor_level {
                        assert!(matches!(
                            effect.disposition,
                            EffectDisposition::Unresolved { .. }
                        ));
                    } else if parameter == &ability.level {
                        assert_eq!(
                            effect.disposition,
                            EffectDisposition::Applied { value: integer(1) }
                        );
                    } else if parameter == &ability.quality {
                        assert_eq!(
                            effect.disposition,
                            EffectDisposition::Applied {
                                value: r.quantity(0.0)
                            }
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn real_supplied_ability_paths_are_distinct_and_never_close_partial_build_coverage() {
    let r = Real::load();
    let request = Arc::new(r.request());
    let resolver =
        OwnedOccurrenceResolver::new(r.schema.as_ref(), &request, BindingLimits::default())
            .unwrap();
    for use_id in [12, 13] {
        for ability in r.abilities() {
            let generated = GeneratedSkillKey {
                provider: r.actor_provider(use_id),
                slot: ability.supply.clone(),
            };
            let skill = resolver
                .skill(&SkillTarget::Generated(Box::new(generated.clone())))
                .unwrap()
                .into_value()
                .unwrap();
            assert_eq!(skill.definition(), Some(&ability.skill));
            assert_eq!(skill.provider().actor(), &r.actor_key(use_id));
            let action = resolver
                .action(&r.action(use_id, &ability))
                .unwrap()
                .into_value()
                .unwrap();
            assert_eq!(action.provider().actor(), &r.actor_key(use_id));
            assert!(
                matches!(action.provider().exposure(), ProviderExposure::Skill { key, .. } if key == &generated)
            );
            let SchemaLookup::Known(schema) = r.schema.definition(&ability.skill) else {
                panic!()
            };
            assert!(!schema.declarations.parameters.is_complete());
            assert_eq!(schema.declarations.parameters.members.len(), 3);
        }
    }
    let plan = Arc::new(
        OwnedEffectPlan::compile(
            request,
            r.schema.clone(),
            r.rules.clone(),
            r.routing.clone(),
            Default::default(),
        )
        .unwrap(),
    );
    assert!(!plan.gaps().is_empty());
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    for use_id in [12, 13] {
        for ability in r.abilities() {
            for parameter in [ability.level, ability.quality, ability.actor_level] {
                let key = PlanValueKey::SkillParameter {
                    skill: Box::new(GeneratedSkillKey {
                        provider: r.actor_provider(use_id),
                        slot: ability.supply.clone(),
                    }),
                    parameter,
                };
                let values: Vec<_> = report.values.iter().filter(|row| row.key == key).collect();
                assert_eq!(values.len(), 1);
                assert!(!matches!(values[0].value, EffectValue::Known { .. }));
            }
        }
    }
    let workers: Vec<_> = (0..3)
        .map(|_| {
            let plan = Arc::clone(&plan);
            std::thread::spawn(move || plan.evaluate(&mut plan.new_scratch()).unwrap())
        })
        .collect();
    for worker in workers {
        assert_eq!(report, worker.join().unwrap());
    }
}
