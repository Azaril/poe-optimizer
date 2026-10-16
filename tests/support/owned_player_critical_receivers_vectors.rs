//! Execute the authored receiver programs in an explicitly finite synthetic world.
//! Measured dependencies are operands, never claims that their production suppliers close.
use super::*;
use poe_optimizer_import::owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{fs, path::PathBuf};

fn game<K: DefinitionDomain>(id: u64) -> DefId<K> {
    def(&format!("def.{id:016x}"))
}
fn quantity(n: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, game(2)).unwrap())
}
// Only namespace is rebound into the existing isolated fixture; semantic IDs,
// units, expressions, relative recipients and all program declarations are exact.
fn isolated<T: DeserializeOwned>(mut value: Value) -> T {
    fn visit(v: &mut Value) {
        match v {
            Value::Object(o) => {
                if let Some(namespace) = o.get_mut("namespace") {
                    assert_eq!(
                        *namespace,
                        serde_json::json!({"game":"poe2","version":"owned-mechanics-v1"})
                    );
                    *namespace = serde_json::to_value(ns()).unwrap();
                }
                for child in o.values_mut() {
                    visit(child);
                }
            }
            Value::Array(a) => a.iter_mut().for_each(visit),
            _ => {}
        }
    }
    visit(&mut value);
    serde_json::from_value(value).unwrap()
}
fn constructor_default(directory: &str) -> f64 {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68")
        .join(directory)
        .join("input.json");
    let input: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    input["constructor_default"]["value"]["value"]
        .as_f64()
        .unwrap()
}
fn finite(row: &Value) -> World {
    finite_flags(row, 0)
}
fn plan(world: &World) -> OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage> {
    world
        .plan_with(|stages| {
            stages.frozen_channels.push(FrozenStageChannel {
                channel: StageChannel::Contributions {
                    scope: RuleEntityKind::Actor,
                    stat: game(0x337f),
                    contribution: ContributionKind::Override,
                },
                stage: key("execute"),
            });
        })
        .unwrap()
}
fn finite_flags(row: &Value, unobserved_mask: u8) -> World {
    let mut world = World::new(80, false);
    for owner in &mut world.f.owners {
        owner.programs.members.clear();
    }
    world.f.receivers.members.clear();
    let dependency_data: Value = family::read("dependencies.json");
    let definitions: Vec<DefinitionDescriptor> = isolated(dependency_data["definitions"].clone());
    world.f.schema.definitions.extend(definitions);
    let entry = world
        .f
        .schema
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::Encounter(entry) => Some(entry),
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(encounter_schema) = &mut entry.schema else {
        panic!()
    };
    encounter_schema.external_inputs =
        DeclaredSet::complete(vec![game(0x3251), game(0x3371), game(0x3372)]);
    // The injected category schema declares all seven options, even though the
    // retained source vectors exercise Average and DamageOverTime only.
    for id in 0x3252..=0x3258 {
        if id != 0x3254 {
            world
                .f
                .schema
                .definitions
                .push(DefinitionDescriptor::Option(known(
                    game(id),
                    OptionSchema {},
                )));
        }
    }
    let extension: OwnedRecipeExtension = isolated(family::read("extension.json"));
    for entry in extension.schema {
        let SchemaExtensionEntry::Definition(definition) = entry else {
            panic!("stat-only receiver extension")
        };
        world.f.schema.definitions.push(definition);
    }
    let consumer: family::Consumer = isolated(family::read("consumer.json"));
    world.f.owners.extend(consumer.owners);
    world.f.receivers.members = consumer.receivers;
    world.queries = DeclaredSet::complete(consumer.queries);
    let dot = row["category"] == "DamageOverTime";
    assert!(dot || row["category"] == "Average");
    world.f.scenario.assumptions.push(ExternalAssumption {
        input: game(0x3251),
        target: AssumptionTarget::Actor(poe_optimizer_core::owned_build::ActorKey::ScenarioEnemy),
        value: ParameterValue::Option(game(if dot { 0x3254 } else { 0x3252 })),
    });
    for (index, id, directory) in [
        (0, 0x3371, "configuration-enemy-critical-chance-inputs"),
        (1, 0x3372, "configuration-enemy-critical-damage-inputs"),
    ] {
        // Null is the measured absent saved value. The previously published
        // constructor adapter supplies its default; the evaluator does not.
        let amount = row["raw"][index]
            .as_f64()
            .unwrap_or_else(|| constructor_default(directory));
        world.f.scenario.assumptions.push(ExternalAssumption {
            input: game(id),
            target: AssumptionTarget::Actor(
                poe_optimizer_core::owned_build::ActorKey::ScenarioEnemy,
            ),
            value: quantity(amount),
        });
    }
    for (index, id) in (0x3376..=0x337b).enumerate() {
        let Some(amount) = row["dependencies"][index].as_f64() else {
            continue;
        };
        let enemy = matches!(index, 1 | 3 | 4);
        world
            .f
            .owner_mut(&if enemy { encounter() } else { class_owner() })
            .programs
            .members
            .push(RuleProgram {
                id: key(&format!("fixture-dependency-{index}")),
                context: RuleEntityKind::Actor,
                reads: vec![],
                nodes: vec![node(
                    "value",
                    RuleExpression::Literal {
                        value: quantity(amount),
                    },
                )],
                effects: vec![effect(
                    "value",
                    RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: game(id),
                        value: key("value"),
                    },
                )],
            });
    }
    for (index, name) in ["never", "always", "unlucky"].into_iter().enumerate() {
        let query = world
            .queries
            .members
            .iter_mut()
            .find(|q| q.id == key(&format!("player-critical-{name}")))
            .unwrap();
        assert!(!query.groups[0].members.is_complete());
        assert!(query.groups[0].members.members.is_empty());
        // A skipped observation proves no Boolean value. The finite test suite
        // separately enumerates every completion of these unconstrained slots;
        // each must leave the observed outputs unchanged. No completion enters data.
        let flag = row["flags"][index]
            .as_bool()
            .unwrap_or(unobserved_mask & (1 << index) != 0);
        let id = key(&format!("fixture-{name}"));
        query.groups[0].members = DeclaredSet::complete(vec![ContributionMember {
            producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                owner: class_owner(),
                program: id.clone(),
                effect: key("flag"),
                origin: ContributionOrigin::Character,
            }),
            order: None,
        }]);
        world
            .f
            .owner_mut(&class_owner())
            .programs
            .members
            .push(RuleProgram {
                id,
                context: RuleEntityKind::Actor,
                reads: vec![],
                nodes: vec![bool_node("value", flag)],
                effects: vec![effect(
                    "flag",
                    RuleEffectKind::Contribute {
                        entity: if index < 2 {
                            RuleEntity::Enemy
                        } else {
                            RuleEntity::Current
                        },
                        stat: game(0x3373 + index as u64),
                        contribution: ContributionKind::Flag,
                        value: key("value"),
                    },
                )],
            });
    }
    // This component has no authored override producer. An empty known inventory
    // is an explicit fixture assertion, never a change to the published Partial.
    let query = world
        .queries
        .members
        .iter_mut()
        .find(|q| q.id == key("player-critical-override"))
        .unwrap();
    assert!(!query.groups[0].members.is_complete() && query.groups[0].members.members.is_empty());
    query.groups[0].members.closure = SchemaClosure::Complete;
    world
}
fn result(report: &OwnedEffectsReport, id: u64) -> &EffectValue {
    &report
        .values
        .iter()
        .find(|row| {
            row.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: game(id),
                }
        })
        .unwrap()
        .value
}
fn overrides(world: &mut World, values: &[Option<f64>]) {
    if values.iter().any(Option::is_none) {
        world
            .f
            .schema
            .definitions
            .push(DefinitionDescriptor::Stat(known(
                def("fixture-unknown-override"),
                StatSchema {
                    value: ComputedValueType::Quantity { unit: game(2) },
                    targets: vec![RuleEntityKind::Actor],
                },
            )));
    }
    let mut members = Vec::new();
    for (index, value) in values.iter().enumerate() {
        let id = key(&format!("fixture-override-{index}"));
        let (reads, node) = match value {
            Some(v) => (
                vec![],
                node(
                    "value",
                    RuleExpression::Literal {
                        value: quantity(*v),
                    },
                ),
            ),
            None => (
                vec![RuleRead {
                    id: key("missing"),
                    value_type: ComputedValueType::Quantity { unit: game(2) },
                    source: RuleReadSource::Stat {
                        entity: RuleEntity::Current,
                        stat: def("fixture-unknown-override"),
                    },
                }],
                read_node("value", "missing"),
            ),
        };
        world
            .f
            .owner_mut(&class_owner())
            .programs
            .members
            .push(RuleProgram {
                id: id.clone(),
                context: RuleEntityKind::Actor,
                reads,
                nodes: vec![node],
                effects: vec![effect(
                    "override",
                    RuleEffectKind::Contribute {
                        entity: RuleEntity::Current,
                        stat: game(0x337f),
                        contribution: ContributionKind::Override,
                        value: key("value"),
                    },
                )],
            });
        members.push(ContributionMember {
            producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                owner: class_owner(),
                program: id,
                effect: key("override"),
                origin: ContributionOrigin::Character,
            }),
            order: None,
        });
    }
    world
        .queries
        .members
        .iter_mut()
        .find(|q| q.id == key("player-critical-override"))
        .unwrap()
        .groups[0]
        .members = DeclaredSet::complete(members);
}
fn compare(row: &Value, report: &SupportEffectsReport) {
    let output = effects(report);
    for (index, id) in [0x337c, 0x337d, 0x337e].into_iter().enumerate() {
        let actual = result(output, id);
        match row["outputs"][index].as_f64() {
            Some(expected) => {
                let EffectValue::Known {
                    value: ParameterValue::Quantity(actual),
                } = actual
                else {
                    panic!("{} {} {actual:?}", row["name"], row["mode"])
                };
                assert_eq!(actual.unit(), &game(if index == 2 { 1 } else { 2 }));
                assert!(
                    (actual.value() - expected).abs() <= 1e-12,
                    "{} {} output{index}: {} != {expected}",
                    row["name"],
                    row["mode"],
                    actual.value()
                );
            }
            None => assert_eq!(
                actual,
                &EffectValue::Inactive,
                "{} output{index}",
                row["name"]
            ),
        }
    }
    assert!(
        output
            .effects
            .iter()
            .filter(|e| matches!(e.key.invocation.origin, RuleOrigin::Receiver { .. }))
            .all(|e| e.key.invocation.entity == ConcreteEntity::Actor(ActorKey::Player))
    );
}

#[test]
fn authored_critical_receivers_match_all_retained_source_vectors_and_worker_reuse() {
    let packet = family::source_vectors();
    let rows = packet["vectors"].as_array().unwrap();
    assert_eq!(rows.len(), 52);
    let plans: Vec<_> = rows.iter().map(|r| plan(&finite(r))).collect();
    let expected: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    let mut scratch = plans[0].new_scratch();
    for (index, (row, plan)) in rows.iter().zip(&plans).enumerate() {
        compare(row, &expected[index]);
        assert_eq!(plan.evaluate(&mut scratch).unwrap(), expected[index]);
        let unknown = row["flags"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .filter(|(_, v)| v.is_null())
            .fold(0u8, |mask, (i, _)| mask | (1 << i));
        for mask in 1..=7 {
            if mask & !unknown == 0 {
                let alternative = self::plan(&finite_flags(row, mask));
                compare(
                    row,
                    &alternative
                        .evaluate(&mut alternative.new_scratch())
                        .unwrap(),
                );
            }
        }
    }
    let actual = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            plans
                .par_iter()
                .map_init(|| plans[0].new_scratch(), |s, p| p.evaluate(s).unwrap())
                .collect::<Vec<_>>()
        });
    assert_eq!(
        actual, expected,
        "parallel workers preserve full values and diagnostics"
    );
    assert_eq!(plans[0].evaluate(&mut scratch).unwrap(), expected[0]);
}

#[test]
fn real_receiver_formula_retains_published_partial_authority_and_missing_numeric_inputs() {
    let packet = family::source_vectors();
    let row = &packet["vectors"][0];
    let mut world = finite(row);
    assert!(
        world
            .plan_with(|_| {})
            .err()
            .unwrap()
            .to_string()
            .contains("numeric selection requires a declared frozen channel")
    );
    let consumer: family::Consumer = isolated(family::read("consumer.json"));
    // Preserve every finite producer but put the exact published open membership
    // back: observed values cannot certify an untranslated supplier inventory.
    for query in &mut world.queries.members {
        query.groups[0].members.closure = consumer
            .queries
            .iter()
            .find(|q| q.id == query.id)
            .unwrap()
            .groups[0]
            .members
            .closure
            .clone();
    }
    let plan = plan(&world);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    incomplete(&report);
    let mut missing = finite(row);
    missing
        .f
        .owner_mut(&encounter())
        .programs
        .members
        .retain(|p| p.id != key("fixture-dependency-1"));
    let plan = self::plan(&missing);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(matches!(
        result(effects(&report), 0x337c),
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingProducer,
            ..
        }
    ));
    assert!(!matches!(
        result(effects(&report), 0x337e),
        EffectValue::Known { .. }
    ));
}

#[test]
fn authored_override_hook_preserves_zero_agreement_conflict_and_lazy_branch_contracts() {
    // Positive overrides are synthetic contract controls: the retained PoB
    // source domain observed no lowercase override producer.
    let packet = family::source_vectors();
    let original = &packet["vectors"][0];
    for (values, expected) in [
        (vec![], 5.0),
        (vec![Some(0.0)], 0.0),
        (vec![Some(12.5)], 12.5),
        (vec![Some(0.0), Some(0.0)], 0.0),
    ] {
        let mut world = finite(original);
        overrides(&mut world, &values);
        let plan = plan(&world);
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert_eq!(
            result(effects(&report), 0x337c),
            &EffectValue::Known {
                value: quantity(expected)
            }
        );
    }
    for values in [
        vec![Some(0.0), Some(10.0)],
        vec![None],
        vec![Some(0.0), None],
    ] {
        let mut world = finite(original);
        overrides(&mut world, &values);
        let plan = plan(&world);
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(
            !matches!(result(effects(&report), 0x337c), EffectValue::Known { .. }),
            "{values:?}"
        );
        assert!(
            !matches!(result(effects(&report), 0x337e), EffectValue::Known { .. }),
            "{values:?}"
        );
    }
    let never = packet["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "never")
        .unwrap();
    let mut world = finite(never);
    overrides(&mut world, &[None]);
    let plan = plan(&world);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    compare(never, &report);
}
