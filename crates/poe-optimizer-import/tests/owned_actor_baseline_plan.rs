//! Compiler-generated programs exercised by the native occurrence resolver.
//! Closure certification below belongs solely to this complete synthetic world;
//! no original or source-derived actor coverage is closed by production code.
#[allow(dead_code)]
#[path = "../../poe-optimizer-engine/tests/support/owned_plan_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::{
    owned_actor_baseline_recipe::*, owned_actor_baselines::*, owned_mapping::*, owned_recipe::*,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf, sync::Arc};
use support::*;
fn rename<T: Serialize + DeserializeOwned>(value: &T, names: &BTreeMap<String, String>) -> T {
    fn walk(value: &mut Value, names: &BTreeMap<String, String>) {
        match value {
            Value::Object(map) => {
                if map.contains_key("namespace")
                    && let Some(old) = map.get("key").and_then(Value::as_str)
                {
                    map.insert(
                        "key".into(),
                        Value::String(
                            names
                                .get(old)
                                .expect("fixture definition is registered")
                                .clone(),
                        ),
                    );
                }
                for value in map.values_mut() {
                    walk(value, names);
                }
            }
            Value::Array(rows) => {
                for value in rows {
                    walk(value, names);
                }
            }
            _ => {}
        }
    }
    let mut value = serde_json::to_value(value).unwrap();
    walk(&mut value, names);
    serde_json::from_value(value).unwrap()
}
fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn actor_value<'a>(
    report: &'a OwnedEffectsReport,
    actor: &ActorKey,
    stat: &StatDefId,
) -> &'a EffectValue {
    &report
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(actor.clone()),
                    stat: stat.clone(),
                }
        })
        .unwrap()
        .value
}
struct World {
    fixture: Fixture,
    actors: [ActorKey; 2],
    level: StatDefId,
    damage: StatDefId,
    time: StatDefId,
    parameter: DeclaredSlot<ParameterSlotDefId>,
    catalog: ActorBaselineCatalog,
}
fn world() -> World {
    let mut fixture = Fixture::new();
    fixture.add_generated_actors();
    let parameter = parameter(SlotOwnerDefId::Gem(def("summoner")), "creating-level");
    for d in &mut fixture.schema.definitions {
        if let DefinitionDescriptor::Gem(row) = d
            && row.id == def("summoner")
            && let SchemaState::Known(schema) = &mut row.schema
        {
            schema
                .declarations
                .parameters
                .members
                .push(parameter.clone());
        }
    }
    fixture.schema.slots.push(SlotDescriptor::Parameter(entry(
        parameter.clone(),
        ParameterSlotSchema {
            value: ValueSchema::Integer(IntegerRange {
                minimum: BoundedInteger::new(1).unwrap(),
                maximum: BoundedInteger::new(40).unwrap(),
            }),
            presence: SlotPresence::RequiredOnce,
            sites: vec![ParameterSite::GemParameter],
        },
    )));
    for (index, gem) in fixture.build.gems.iter_mut().enumerate() {
        gem.level = 1;
        gem.parameters.push(ParameterAssignment {
            slot: parameter.clone(),
            value: integer(if index == 0 { 20 } else { 1 }),
        });
    }
    let summoner = fixture.owner_mut(&summoner_owner());
    summoner.programs.members[0]
        .effects
        .retain(|e| !matches!(e.effect, RuleEffectKind::ProjectActorStat { .. }));
    summoner.programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: summoner_owner(),
            facet: SchemaFacet::GameRules,
            code: key("synthetic-before-compiler"),
        }],
    };
    let owner = SchemaSubject::Slot(SlotAddress::Actor(child_slot()));
    let actor = fixture.owner_mut(&owner);
    actor.programs.members.clear();
    actor.programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner,
            facet: SchemaFacet::GameRules,
            code: key("synthetic-before-compiler"),
        }],
    };
    for (name, dimension) in [
        ("seconds", UnitDimension::Time),
        ("damage-points", UnitDimension::Damage),
    ] {
        fixture
            .schema
            .definitions
            .push(DefinitionDescriptor::Unit(entry(
                def(name),
                UnitSchema { dimension },
            )));
    }
    for (name, value) in [
        ("native-level", ComputedValueType::Integer),
        (
            "native-time",
            ComputedValueType::Quantity {
                unit: def("seconds"),
            },
        ),
        (
            "native-damage",
            ComputedValueType::Quantity {
                unit: def("damage-points"),
            },
        ),
    ] {
        fixture
            .schema
            .definitions
            .push(DefinitionDescriptor::Stat(entry(
                def(name),
                StatSchema {
                    value,
                    targets: vec![RuleEntityKind::Actor],
                },
            )));
    }
    // Convert the shared synthetic fixture to strict append-only registry IDs.
    // Only typed IDs are renamed; program/table names and build occurrences stay.
    let mut names = BTreeMap::new();
    for name in fixture
        .schema
        .definitions
        .iter()
        .map(|d| d.address().key().as_str().to_owned())
        .chain(
            fixture
                .schema
                .slots
                .iter()
                .map(|s| s.address().key().as_str().to_owned()),
        )
    {
        let next = format!("def.{:016x}", names.len() + 1);
        assert!(names.insert(name, next).is_none());
    }
    fixture.schema = rename(&fixture.schema, &names);
    fixture.build = rename(&fixture.build, &names);
    fixture.scenario = rename(&fixture.scenario, &names);
    fixture.queries = rename(&fixture.queries, &names);
    fixture.owners = rename(&fixture.owners, &names);
    fixture.tables = rename(&fixture.tables, &names);
    fixture.receivers = rename(&fixture.receivers, &names);
    fixture.routes = rename(&fixture.routes, &names);
    let count = fixture.schema.definitions.len() + fixture.schema.slots.len();
    let entries = fixture
        .schema
        .definitions
        .iter()
        .map(|d| SchemaSubject::Definition(d.address()))
        .chain(
            fixture
                .schema
                .slots
                .iter()
                .map(|s| SchemaSubject::Slot(s.address())),
        )
        .enumerate()
        .map(|(i, target)| RegistryEntry {
            sequence: BoundedInteger::new(i as i64 + 1).unwrap(),
            target,
            state: RegistryState::Active,
        })
        .collect();
    let registry = RegistryInput {
        schema_version: OWNED_ID_REGISTRY_VERSION,
        namespace: ns(),
        revision: BoundedInteger::new(count as i64).unwrap(),
        last_issued: BoundedInteger::new(count as i64).unwrap(),
        entries,
    };
    let schema =
        OwnedDefinitionSchemaPackage::new(fixture.schema.clone(), Default::default()).unwrap();
    let base = assemble_owned_recipe(
        OwnedRecipeInput {
            schema_version: 1,
            registry,
            schema: schema.input().clone(),
            rules: RulePackageInput {
                schema_version: OWNED_RULE_PACKAGE_VERSION,
                namespace: ns(),
                release: key("test"),
                semantics_version: key("test"),
                operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
                definitions: schema.identity().clone(),
                tables: fixture.tables.clone(),
                owners: fixture.owners.clone(),
                receivers: fixture.receivers.clone(),
            },
            routing: ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("test"),
                definitions: schema.identity().clone(),
                outputs: fixture.routes.clone(),
            },
        },
        Default::default(),
    )
    .unwrap();
    let bytes = fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/owned/poe2/3887ae68/actor-baselines/catalog.json"),
    )
    .unwrap();
    let catalog: ActorBaselineCatalog = serde_json::from_slice(&bytes).unwrap();
    let actor = rename(&child_slot(), &names);
    let parameter = rename(&parameter, &names);
    let level = rename(&def::<StatDefinition>("native-level"), &names);
    let time = rename(&def::<StatDefinition>("native-time"), &names);
    let damage = rename(&def::<StatDefinition>("native-damage"), &names);
    let policy = ActorBaselinePolicy {
        schema_version: 1,
        version: key("actual-occurrences"),
        catalog_sha256: format!("{:x}", Sha256::digest(&bytes)),
        source: catalog.source.clone(),
        bindings: vec![ActorBaselineBinding {
            profile: "SandDjinn".into(),
            actor: actor.clone(),
            program: key("baseline"),
            fields: vec![ActorScalarBinding {
                field: ActorBaselineField::AttackTime,
                target: ActorScalarTarget::Quantity {
                    stat: time.clone(),
                    unit: rename(&def::<UnitDefinition>("seconds"), &names),
                },
                when_absent: ActorFactAbsence::Reject,
            }],
            curves: vec![ActorCurveBinding {
                curve: ActorDamageCurve::Allied,
                actor_level: level.clone(),
                table: key("allied"),
                stat: damage.clone(),
                unit: rename(&def::<UnitDefinition>("damage-points"), &names),
            }],
            summon_level: Some(ActorSummonLevelBinding {
                parameter: parameter.clone(),
                stat: level.clone(),
                table: key("summon-levels"),
                program: key("project-native-level"),
            }),
        }],
    };
    let result = compile_owned_actor_baselines(&base, &bytes, &policy, Default::default()).unwrap();
    assert_eq!(result.successor.schema, *base.schema().input());
    assert!(
        result
            .successor
            .rules
            .owners
            .iter()
            .filter(|o| !o.programs.is_complete())
            .count()
            == 2
    );
    fixture.owners = result.successor.rules.owners;
    fixture.tables = result.successor.rules.tables;
    // This finite test world declares exactly the listed programs. The compiler
    // preserved both Partial states; synthetic certification is explicit here.
    for owner in &mut fixture.owners {
        owner.programs.closure = SchemaClosure::Complete;
    }
    World {
        fixture,
        actors: [
            rename(&child_actor(30), &names),
            rename(&child_actor(31), &names),
        ],
        level,
        damage,
        time,
        parameter,
        catalog,
    }
}
#[test]
fn generated_rules_bind_distinct_creating_occurrences_in_native_parallel_plans() {
    let mut w = world();
    let plan = Arc::new(w.fixture.compile().unwrap());
    let mut scratch = plan.new_scratch();
    let first = plan.evaluate(&mut scratch).unwrap();
    assert_eq!(first, plan.evaluate(&mut scratch).unwrap());
    for (i, level) in [(0, 40), (1, 2)] {
        assert_eq!(
            actor_value(&first, &w.actors[i], &w.level),
            &EffectValue::Known {
                value: integer(level)
            }
        );
        assert!(
            matches!(actor_value(&first,&w.actors[i],&w.damage),EffectValue::Known{value:ParameterValue::Quantity(v)} if v.value()==w.catalog.allied_damage.rows[level as usize-1])
        );
        assert!(
            matches!(actor_value(&first,&w.actors[i],&w.time),EffectValue::Known{value:ParameterValue::Quantity(v)} if v.value()==0.0)
        );
    }
    let workers = (0..4)
        .map(|_| {
            let plan = plan.clone();
            std::thread::spawn(move || plan.evaluate(&mut plan.new_scratch()).unwrap())
        })
        .collect::<Vec<_>>();
    for worker in workers {
        assert_eq!(first, worker.join().unwrap());
    }
    w.fixture.build.gems[0]
        .parameters
        .iter_mut()
        .find(|p| p.slot == w.parameter)
        .unwrap()
        .value = integer(21);
    let changed = w.fixture.compile().unwrap();
    let changed = changed.evaluate(&mut changed.new_scratch()).unwrap();
    assert_eq!(
        actor_value(&changed, &w.actors[0], &w.level),
        &EffectValue::Known { value: integer(42) }
    );
    assert_eq!(
        actor_value(&changed, &w.actors[1], &w.level),
        &EffectValue::Known { value: integer(2) }
    );
    w.fixture.build.gems[0]
        .parameters
        .iter_mut()
        .find(|p| matches!(p.value, ParameterValue::Boolean(_)))
        .unwrap()
        .value = ParameterValue::Boolean(false);
    let disabled = w.fixture.compile().unwrap();
    let disabled = disabled.evaluate(&mut disabled.new_scratch()).unwrap();
    assert_eq!(
        actor_value(&disabled, &w.actors[0], &w.time),
        &EffectValue::Inactive
    );
    assert!(matches!(
        actor_value(&disabled, &w.actors[1], &w.time),
        EffectValue::Known { .. }
    ));
}
