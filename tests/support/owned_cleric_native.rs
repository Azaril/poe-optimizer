//! Execute the published Cleric data without modifying its coverage declarations.
//! Explicit component inputs below are test probes, not imported effective values.
use super::support::{data, json};
use poe_optimizer_core::{
    build_identity::*, owned_binding::*, owned_build::*, owned_definitions::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting, owned_rules::OwnedRulePackage,
    owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::*};
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{fs, path::Path, sync::Arc};

fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn field<T: DeserializeOwned>(value: &Value, name: &str) -> T {
    serde_json::from_value(value[name].clone()).unwrap()
}
fn integer(value: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(value).unwrap())
}
fn occurrence<T: BuildInstanceId>(id: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([115; 16]), id).unwrap())
}

struct World {
    schema: Arc<OwnedDefinitionSchemaPackage>,
    rules: Arc<CompiledRulePackage>,
    routing: Arc<OwnedActionRouting>,
    ids: Value,
    bindings: Value,
    sniper: Value,
}
impl World {
    fn load(package: &Path) -> Self {
        let schema = Arc::new(
            OwnedDefinitionSchemaPackage::new(
                read(package.join("schema.json")),
                Default::default(),
            )
            .unwrap(),
        );
        let stored = OwnedRulePackage::new(
            read(package.join("rules.json")),
            schema.as_ref(),
            Default::default(),
        )
        .unwrap();
        let rules = Arc::new(
            CompiledRulePackage::compile_stored(&stored, schema.as_ref(), Default::default())
                .unwrap(),
        );
        let routing = Arc::new(
            OwnedActionRouting::new(
                read(package.join("routing.json")),
                schema.as_ref(),
                Default::default(),
            )
            .unwrap(),
        );
        Self {
            schema,
            rules,
            routing,
            ids: json(data().join("ids.json"))["allocations"].clone(),
            bindings: json(data().join("cleric-ability-supply/bindings.json")),
            sniper: json(data().join("actor-ability-supply/bindings.json")),
        }
    }
    fn quantity(&self, value: f64) -> ParameterValue {
        ParameterValue::Quantity(
            FiniteQuantity::new(value, field(&self.ids, "percentage-points")).unwrap(),
        )
    }
    fn summoner(&self, use_id: u64) -> ProviderKey {
        ProviderKey {
            root: ProviderRoot::SkillUse(occurrence(use_id)),
            grant_path: vec![field(&self.bindings, "primary_grant")],
        }
    }
    fn actor(&self, use_id: u64) -> ActorKey {
        ActorKey::Owned(Box::new(OwnedActorKey {
            provider: self.summoner(use_id),
            slot: field(&self.bindings, "population"),
        }))
    }
    fn actor_provider(&self, use_id: u64) -> ProviderKey {
        let mut provider = self.summoner(use_id);
        provider
            .grant_path
            .push(field(&self.bindings, "population_grant"));
        provider
    }
    fn action(&self, use_id: u64) -> ActionSelection {
        let mut provider = self.actor_provider(use_id);
        provider
            .grant_path
            .push(field(&self.bindings["heal"], "grant"));
        ActionSelection {
            action: ActionKey {
                actor: self.actor(use_id),
                provider,
                output: field(&self.bindings["heal"], "output"),
            },
            part: field(&self.ids, "ordinary-part"),
            mode: field(&self.ids, "ordinary-mode"),
            stat_set: field(&self.ids, "primary-stat-set"),
        }
    }
    fn request(&self, first_enabled: bool) -> OwnedEvaluationRequest {
        let ns = self.schema.namespace().clone();
        let limits = OwnedInputLimits::default();
        let gem: GemDefId = field(&self.bindings, "gem");
        let SchemaLookup::Known(gem_schema) = self.schema.definition(&gem) else {
            panic!("published Cleric Gem schema")
        };
        // Explicit uncorrupted physical test Gems; these are not effective levels.
        let intrinsic = gem_schema
            .declarations
            .parameters
            .members
            .iter()
            .map(|slot| {
                let SchemaLookup::Known(parameter) = self.schema.slot(slot) else {
                    panic!("published intrinsic parameter")
                };
                let value = match &parameter.value {
                    ValueSchema::Boolean => ParameterValue::Boolean(false),
                    ValueSchema::Quantity(range) => ParameterValue::Quantity(
                        FiniteQuantity::new(0.0, range.minimum.unit().clone()).unwrap(),
                    ),
                    _ => panic!("unreviewed Cleric intrinsic input"),
                };
                ParameterAssignment {
                    slot: slot.clone(),
                    value,
                }
            })
            .collect::<Vec<_>>();
        let build = BuildSpec::new(
            BuildInput {
                support_origins: None,
                allocator: InstanceAllocatorState::from_parts(
                    BuildLineage::from_bytes([115; 16]),
                    30,
                ),
                revision: BuildRevision::from_u64(1),
                game_version: ns.clone(),
                character: CharacterSpec {
                    class: ClassDefId::parse(ns.clone(), "unmapped-component-class").unwrap(),
                    ascendancy: None,
                    level: 100,
                    rewards: vec![],
                },
                weapon_loadouts: vec![occurrence(1)],
                active_weapon_loadout: occurrence(1),
                items: vec![],
                equipment: vec![],
                allocations: vec![],
                gems: [(2, 19), (3, 25)]
                    .into_iter()
                    .map(|(id, level)| GemInstance {
                        id: occurrence(id),
                        definition: gem.clone(),
                        parameters: intrinsic.clone(),
                        level,
                        quality: Some(QualitySelection {
                            kind: field(&self.ids, "standard-quality"),
                            amount: FiniteQuantity::new(
                                20.0,
                                field(&self.ids, "percentage-points"),
                            )
                            .unwrap(),
                        }),
                    })
                    .collect(),
                skills: [(12, 2, first_enabled), (13, 3, true)]
                    .into_iter()
                    .map(|(id, gem, enabled)| SkillUse {
                        id: occurrence(id),
                        source: AuthoredSkillSource::Gem(occurrence(gem)),
                        enabled,
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
                    encounter: EncounterDefId::parse(ns.clone(), "unmapped-component-encounter")
                        .unwrap(),
                    level: 1,
                },
                assumptions: vec![],
                usage: vec![],
            },
            limits,
        )
        .unwrap();
        let queries = QuerySpec::new(
            QueryInput {
                game_version: ns.clone(),
                requests: [12, 13]
                    .into_iter()
                    .map(|id| MetricRequest {
                        id: QueryId::new(format!("cleric-{id}")).unwrap(),
                        metric: MetricDefId::parse(ns.clone(), "unmapped-component-metric")
                            .unwrap(),
                        target: MetricTarget::Action(Box::new(self.action(id))),
                    })
                    .collect(),
            },
            limits,
        )
        .unwrap();
        OwnedEvaluationRequest::new(build, scenario, queries, limits).unwrap()
    }
}

fn check_component_inputs(w: &World) {
    let summon_level: DeclaredSlot<ParameterSlotDefId> = field(&w.bindings, "summon_level");
    let summon_quality: DeclaredSlot<ParameterSlotDefId> = field(&w.bindings, "summon_quality");
    let population: DeclaredSlot<ActorSlotDefId> = field(&w.bindings, "population");
    // Prove the specific absent producers as well as the whole-plan coverage
    // result below. Unrelated Partial owners must not conceal a raw-value fallback.
    for effect in w
        .rules
        .input()
        .owners
        .iter()
        .flat_map(|owner| &owner.programs.members)
        .flat_map(|program| &program.effects)
    {
        match &effect.effect {
            RuleEffectKind::ProjectSkillParameter { parameter, .. } => {
                assert!(parameter != &summon_level && parameter != &summon_quality);
            }
            RuleEffectKind::ProjectActorStat { actor, .. } => assert_ne!(actor, &population),
            _ => {}
        }
    }
    let actor: ActorDefId = field(&w.bindings, "actor");
    let owner = SchemaSubject::Definition(actor.address());
    let actor_rules = w
        .rules
        .input()
        .owners
        .iter()
        .find(|row| row.owner == owner)
        .unwrap();
    assert!(!actor_rules.programs.is_complete());
    let heal = &w.bindings["heal"];
    let ability: SkillDefId = field(heal, "skill");
    let SchemaLookup::Known(schema) = w.schema.definition(&ability) else {
        panic!()
    };
    assert!(!schema.directly_selectable);
    assert!(!schema.declarations.parameters.is_complete());
    let supply: DeclaredSlot<SkillGrantSlotDefId> = field(heal, "supply");
    let grant: DeclaredSlot<GrantSlotDefId> = field(heal, "grant");
    let actor_level: StatDefId = field(&w.bindings, "actor_level");
    let mut scratch = w.rules.new_scratch();
    for supplied_level in [Some(1), Some(38), Some(100), None] {
        let mut effects = vec![];
        for program in &actor_rules.programs.members {
            let facts = program.reads.iter().filter_map(|read| {
                assert!(matches!(&read.source, RuleReadSource::Stat { entity: RuleEntity::Current | RuleEntity::Actor, stat } if stat == &actor_level));
                supplied_level.map(|value| RuleFact { read: read.id.clone(), value: integer(value) })
            }).collect::<Vec<_>>();
            effects.extend(
                w.rules
                    .evaluate(&owner, &program.id, &facts, w.schema.as_ref(), &mut scratch)
                    .unwrap()
                    .effects,
            );
        }
        for (name, expected) in [
            ("level", Some(integer(1))),
            ("quality", Some(w.quantity(0.0))),
            ("actor_level", supplied_level.map(integer)),
        ] {
            let parameter: DeclaredSlot<ParameterSlotDefId> = field(heal, name);
            let matching = effects.iter().filter(|effect| matches!(&effect.effect, RuleEffectKind::ProjectSkillParameter { skill, parameter: target, .. } if skill == &supply && target == &parameter)).collect::<Vec<_>>();
            assert_eq!(matching.len(), 1, "{name}");
            if let Some(value) = expected {
                assert_eq!(
                    matching[0].disposition,
                    EffectDisposition::Applied { value }
                );
            } else {
                assert!(matches!(
                    matching[0].disposition,
                    EffectDisposition::Unresolved { .. }
                ));
            }
        }
        let activations = effects.iter().filter(|effect| matches!(&effect.effect, RuleEffectKind::ActivateGrant { slot, .. } if slot == &grant)).collect::<Vec<_>>();
        assert_eq!(activations.len(), 1);
        assert_eq!(
            activations[0].disposition,
            EffectDisposition::Applied {
                value: ParameterValue::Boolean(true)
            }
        );
    }
}

fn check_bound_topology(w: &World, request: &OwnedEvaluationRequest) {
    let resolver =
        OwnedOccurrenceResolver::new(w.schema.as_ref(), request, Default::default()).unwrap();
    assert_ne!(w.actor(12), w.actor(13));
    assert_ne!(
        field::<ActorDefId>(&w.bindings, "actor"),
        field::<ActorDefId>(&w.sniper, "actor")
    );
    let heal = &w.bindings["heal"];
    for id in [12, 13] {
        let generated = GeneratedSkillKey {
            provider: w.actor_provider(id),
            slot: field(heal, "supply"),
        };
        let resolved = resolver
            .skill(&SkillTarget::Generated(Box::new(generated.clone())))
            .unwrap();
        assert!(
            resolved.value().is_some(),
            "Cleric skill binding: {:?}",
            resolved.issues()
        );
        let skill = resolved.into_value().unwrap();
        assert_eq!(
            skill.definition(),
            Some(&field::<SkillDefId>(heal, "skill"))
        );
        assert_eq!(skill.provider().actor(), &w.actor(id));
        // The generated Skill path is known. Its full Action still requires the
        // deliberately unconverted Heal choice declaration; keep that distinction.
        let action = resolver.action(&w.action(id)).unwrap();
        assert!(action.value().is_none());
        assert_eq!(action.schema(), SchemaBindingStatus::Unresolved);
        assert!(action.issues().iter().any(|issue| {
            issue.code == BindingIssueCode::PartialMembership
                && issue.subject
                    == Some(SchemaSubject::Definition(
                        field::<SkillDefId>(heal, "skill").address(),
                    ))
        }));
        let mut sibling = w.action(id);
        sibling.action.actor = w.actor(if id == 12 { 13 } else { 12 });
        let mismatch = resolver.action(&sibling).unwrap();
        assert_eq!(mismatch.schema(), SchemaBindingStatus::Invalid);
        assert!(
            mismatch
                .issues()
                .iter()
                .any(|issue| issue.code == BindingIssueCode::ActorMismatch)
        );
    }
    let disabled = w.request(false);
    let resolver =
        OwnedOccurrenceResolver::new(w.schema.as_ref(), &disabled, Default::default()).unwrap();
    let inactive = resolver.action(&w.action(12)).unwrap();
    assert!(inactive.value().is_none());
    assert!(
        inactive
            .issues()
            .iter()
            .any(|issue| issue.code == BindingIssueCode::DisabledProvider)
    );
    assert!(
        resolver
            .skill(&SkillTarget::Generated(Box::new(GeneratedSkillKey {
                provider: w.actor_provider(13),
                slot: field(heal, "supply"),
            })))
            .unwrap()
            .value()
            .is_some()
    );
}

pub(super) fn check_native_cleric_supply(package: &Path) {
    let w = World::load(package);
    check_component_inputs(&w);
    let request = Arc::new(w.request(true));
    check_bound_topology(&w, &request);
    let plan = OwnedEffectPlan::compile(
        request,
        w.schema.clone(),
        w.rules.clone(),
        w.routing.clone(),
        Default::default(),
    )
    .unwrap();
    assert!(!plan.gaps().is_empty());
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    for id in [12, 13] {
        for parameter in ["level", "quality", "actor_level"] {
            let expected = PlanValueKey::SkillParameter {
                skill: Box::new(GeneratedSkillKey {
                    provider: w.actor_provider(id),
                    slot: field(&w.bindings["heal"], "supply"),
                }),
                parameter: field(&w.bindings["heal"], parameter),
            };
            let values = report
                .values
                .iter()
                .filter(|row| row.key == expected)
                .collect::<Vec<_>>();
            assert_eq!(values.len(), 1);
            assert!(matches!(values[0].value, EffectValue::Unresolved { .. }));
        }
    }
    // Data-derived literals cannot escape incomplete parent inputs/owner coverage.
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(3)
        .build()
        .unwrap();
    let parallel = pool.install(|| {
        (0..6)
            .into_par_iter()
            .map(|_| {
                let mut scratch = plan.new_scratch();
                let first = plan.evaluate(&mut scratch).unwrap();
                assert_eq!(first, plan.evaluate(&mut scratch).unwrap());
                first
            })
            .collect::<Vec<_>>()
    });
    for actual in parallel {
        assert_eq!(report, actual);
    }
}

/// Fast local reproduction against an explicitly published immutable release.
/// The ordinary CLI-chain test above also calls the same assertions in CI.
#[test]
#[ignore = "requires an explicitly published Cleric release in POE_OPTIMIZER_TEST_CLERIC_RELEASE"]
fn published_cleric_release_native_regression() {
    let path = std::env::var_os("POE_OPTIMIZER_TEST_CLERIC_RELEASE")
        .expect("set POE_OPTIMIZER_TEST_CLERIC_RELEASE to the generated release directory");
    check_native_cleric_supply(Path::new(&path));
}
