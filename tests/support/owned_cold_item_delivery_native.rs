//! Actual published Cold/Sapphire bodies and selected Original05 inputs in an
//! unpublished finite topology. Life, other items, placement feasibility, reflected
//! copies, Focus scaling and final resistance aggregation are outside this proof.
#[allow(dead_code)]
#[path = "owned_sapphire_native_fixture.rs"]
mod shared;

use super::{family, release};
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_readiness::*, owned_rules::*,
    owned_schema::*, owned_stages::*, owned_support_inputs::*, owned_support_receiving::*,
    owned_supports::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting, owned_rules::OwnedRulePackage,
    owned_schema::OwnedDefinitionSchemaPackage, owned_stages::OwnedEvaluationStages,
    owned_support_inputs::OwnedSupportInputBindings,
    owned_support_receiving::OwnedSupportReceiving, owned_supports::OwnedSupportPreparation,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use poe_optimizer_import::owned_recipe::OwnedRecipeInput;
use rayon::prelude::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

const DIRECT: &str = "contribute-player-cold-resistance";
const APPLICABLE: &str = "ordinary-unscaled-numeric-item-delivery-applicability";

fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(ns(), format!("def.{n:016x}")).unwrap()
}
fn decode<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn known(value: &Value) -> &Value {
    assert_eq!(value["kind"], "known");
    &value["value"]
}
fn subject<I: SchemaDefinitionId>(id: I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
fn quantity(n: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, def(2)).unwrap())
}
fn assignments(value: &Value) -> Vec<ParameterAssignment> {
    assert_eq!(value["completion"], json!({"kind":"complete"}));
    value["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| ParameterAssignment {
            slot: decode(known(&p["slot"])),
            value: decode(known(&p["value"])),
        })
        .collect()
}
fn quality(value: &Value) -> Option<QualitySelection> {
    let value = known(value);
    if value.is_null() {
        None
    } else {
        Some(QualitySelection {
            kind: decode(known(&value["kind"])),
            amount: decode(known(&value["amount"])),
        })
    }
}

struct Source {
    recipe: OwnedRecipeInput,
    draft: Value,
    selected: Value,
    vectors: Value,
    inputs: shared::Inputs,
}
fn source() -> &'static Source {
    static SOURCE: OnceLock<Source> = OnceLock::new();
    SOURCE.get_or_init(|| {
        let path = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_COLD_ITEM_DELIVERY_RELEASE")
                .expect("set the verified cold item delivery package"),
        );
        let before = release::inventory(&path);
        let inputs = shared::Inputs::load_output(path.parent().unwrap());
        family::assert_endpoint(&inputs.endpoint);
        let read = |name: &str| -> Value {
            serde_json::from_slice(&fs::read(path.parent().unwrap().join(name)).unwrap()).unwrap()
        };
        let draft = read("original-05/draft.json")["draft"].clone();
        let selected = read("selected-05.json");
        assert_eq!(before, release::inventory(&path));
        Source {
            recipe: inputs.endpoint.input().recipe.clone(),
            draft,
            selected,
            vectors: family::read("source-vectors.json"),
            inputs,
        }
    })
}

// Rebind only the finite construction's instance lineage. Semantic item/use
// addresses and assignments below come from the actual selected source draft.
fn relineage<T: Serialize + DeserializeOwned>(value: &T, lineage: BuildLineage) -> T {
    fn walk(value: &mut Value, lineage: &Value) {
        match value {
            Value::Object(o) => {
                if o.contains_key("lineage") {
                    o.insert("lineage".into(), lineage.clone());
                }
                for v in o.values_mut() {
                    walk(v, lineage);
                }
            }
            Value::Array(a) => {
                for v in a {
                    walk(v, lineage);
                }
            }
            _ => {}
        }
    }
    let mut value = json!(value);
    walk(&mut value, &json!(lineage));
    decode(&value)
}
struct World {
    f: shared::Fixture,
    actual_owners: Vec<DefinitionRules>,
    ring_uses: Vec<ItemSlotUseId>,
    cold: ModifierInstanceId,
}
impl World {
    fn new() -> Self {
        let s = source();
        let dependencies: Value = family::read("dependencies.json");
        let prior: Vec<DefinitionRules> = decode(&dependencies["owners"]);
        let mut f = s.inputs.successor_fixture(&prior);
        let mut actual_owners = Vec::new();
        for (address, appended) in [
            (def::<ModifierDefinition>(0x2542).address(), DIRECT),
            (def::<ItemTemplateDefinition>(0x09dc).address(), APPLICABLE),
        ] {
            let actual = s
                .recipe
                .rules
                .owners
                .iter()
                .find(|o| o.owner == SchemaSubject::Definition(address.clone()))
                .unwrap();
            assert!(!actual.programs.is_complete());
            let target = f
                .recipe
                .rules
                .owners
                .iter_mut()
                .find(|o| o.owner == actual.owner)
                .unwrap();
            let added = actual
                .programs
                .members
                .iter()
                .find(|p| p.id.as_str() == appended)
                .unwrap();
            let mut expected = target.programs.members.clone();
            expected.push(added.clone());
            assert_eq!(
                actual.programs.members, expected,
                "exact additive successor body"
            );
            target.programs.members = actual.programs.members.clone();
            actual_owners.push(actual.clone());
        }
        // The component's original four programs already bring their exact leaf
        // dependencies. Add only the new delivery/gate and actual destination /
        // quality descriptors; complete leaf owners remain test-only topology.
        let required = [
            def::<StatDefinition>(0x09d4).address(),
            def::<StatDefinition>(0x3314).address(),
            def::<StatDefinition>(0x3306).address(),
            def::<QualityDefinition>(6).address(),
            def::<EquipmentSlotDefinition>(0x006b).address(),
            def::<EquipmentSlotDefinition>(0x006c).address(),
        ];
        for address in required {
            let d = s
                .recipe
                .schema
                .definitions
                .iter()
                .find(|d| d.address() == address)
                .unwrap();
            if let Some(existing) = f
                .recipe
                .schema
                .definitions
                .iter()
                .find(|x| x.address() == address)
            {
                assert_eq!(existing, d);
            } else {
                f.recipe.schema.definitions.push(d.clone());
                f.recipe.rules.owners.push(DefinitionRules {
                    owner: SchemaSubject::Definition(address),
                    programs: DeclaredSet::complete(vec![]),
                });
            }
        }
        let actual_allocator: InstanceAllocatorState = decode(&s.draft["allocator"]);
        f.build = relineage(&f.build, actual_allocator.lineage());
        f.build.allocator = InstanceAllocatorState::from_parts(
            actual_allocator.lineage(),
            actual_allocator.last_issued() + 10,
        );
        let ring = &s.inputs.ring;
        let item_id: ItemRecordId = decode(&ring["id"]);
        let actual_cold = ring["modifiers"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| {
                m["definition"].get("value") == Some(&json!(def::<ModifierDefinition>(0x2542)))
            })
            .unwrap();
        let cold_id: ModifierInstanceId = decode(&actual_cold["id"]);
        assert_eq!(f.build.items.len(), 1);
        let item = &mut f.build.items[0];
        item.id = item_id;
        item.modifiers[0].id = cold_id;
        item.modifier_order = vec![cold_id];
        item.item_level = decode(known(&ring["item_level"]));
        item.quality = quality(&ring["quality"]);
        assert_eq!(item.parameters, assignments(&ring["parameters"]));
        assert_eq!(item.modifiers[0].rolls, assignments(&actual_cold["rolls"]));
        let preset = s.draft["equipment_presets"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == s.selected["build"]["equipment"])
            .unwrap();
        assert_eq!(
            preset["equipment"]["completion"],
            json!({"kind":"complete"})
        );
        let selected_ids: BTreeSet<ItemSlotUseId> = decode(&preset["equipment"]["members"]);
        f.build.equipment.clear();
        for e in s.draft["equipment"]["members"].as_array().unwrap() {
            if selected_ids.contains(&decode::<ItemSlotUseId>(&e["id"]))
                && known(&e["item"]) == &json!(item_id)
            {
                assert_eq!(e["destination"]["kind"], "character_slot");
                f.build.equipment.push(EquipmentUse {
                    id: decode(&e["id"]),
                    item: item_id,
                    destination: EquipmentDestination::CharacterSlot(decode(known(
                        &e["destination"]["value"],
                    ))),
                    scope: decode(known(&e["scope"])),
                });
            }
        }
        f.build.equipment.sort_by_key(|e| e.id);
        assert_eq!(f.build.equipment.len(), 2);
        assert_eq!(
            f.build
                .equipment
                .iter()
                .map(|e| e.destination.clone())
                .collect::<Vec<_>>(),
            [
                EquipmentDestination::CharacterSlot(def(0x006b)),
                EquipmentDestination::CharacterSlot(def(0x006c))
            ]
        );
        let ring_uses = f.build.equipment.iter().map(|e| e.id).collect();
        let template = f
            .recipe
            .schema
            .definitions
            .iter_mut()
            .find(|d| d.address() == def::<ItemTemplateDefinition>(0x09dc).address())
            .unwrap();
        let DefinitionDescriptor::ItemTemplate(row) = template else {
            panic!()
        };
        let SchemaState::Known(schema) = &mut row.schema else {
            panic!()
        };
        schema.equipment_slots = DeclaredSet::complete(vec![def(0x006b), def(0x006c)]);
        schema.quality.presence = QualityPresence::Optional;
        schema.quality.allowed_kinds = DeclaredSet::complete(vec![def(6)]);
        f.complete_domain();
        Self {
            f,
            actual_owners,
            ring_uses,
            cold: cold_id,
        }
    }
    fn owner_mut(&mut self, subject: &SchemaSubject) -> &mut DefinitionRules {
        self.f
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| &o.owner == subject)
            .unwrap()
    }
    fn plan(
        &self,
    ) -> std::result::Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>, PlanError> {
        staged_plan(&self.f)
    }
    fn report(&self) -> SupportEffectsReport {
        let plan = self.plan().unwrap();
        plan.evaluate(&mut plan.new_scratch()).unwrap()
    }
    fn evaluate(&self) -> OwnedEffectsReport {
        effects(self.report())
    }
    fn raw(&mut self, value: f64) {
        self.f.build.items[0].modifiers[0]
            .rolls
            .iter_mut()
            .find(|p| p.slot.slot == def(0x2543))
            .unwrap()
            .value = quantity(value);
    }
    fn guard(&mut self, enabled: bool) {
        let p = self
            .owner_mut(&subject(def::<ItemTemplateDefinition>(0x09dc)))
            .programs
            .members
            .iter_mut()
            .find(|p| p.id.as_str() == APPLICABLE)
            .unwrap();
        assert_eq!(p.nodes.len(), 1);
        p.nodes[0].expression = RuleExpression::Literal {
            value: ParameterValue::Boolean(enabled),
        };
    }
    fn remove_program(&mut self, name: &str) {
        for o in &mut self.f.recipe.rules.owners {
            o.programs.members.retain(|p| p.id.as_str() != name);
        }
    }
    fn cold_effects<'a>(&self, report: &'a OwnedEffectsReport) -> Vec<&'a BoundEffectResult> {
        report
            .effects
            .iter()
            .filter(|e| e.key.invocation.program.as_str() == DIRECT)
            .collect()
    }
    fn assert_values(&self, report: &OwnedEffectsReport, expected: &[(ItemSlotUseId, f64)]) {
        assert!(report.gaps.is_empty(), "{:?}", report.gaps);
        let effects = self.cold_effects(report);
        assert_eq!(effects.len(), expected.len());
        let mut actual = Vec::new();
        for e in effects {
            assert_eq!(
                e.target,
                BoundEffectTarget::Contribution {
                    key: ContributionKey {
                        entity: ConcreteEntity::Actor(ActorKey::Player),
                        stat: def(0x09d4),
                        kind: ContributionKind::Add,
                    }
                }
            );
            let RuleOrigin::Provider { provider } = &e.key.invocation.origin else {
                panic!("exact source provider")
            };
            assert!(provider.grant_path.is_empty());
            let ProviderRoot::ItemModifier {
                equipment_use,
                modifier,
            } = provider.root
            else {
                panic!()
            };
            let use_row = self
                .f
                .build
                .equipment
                .iter()
                .find(|u| u.id == equipment_use)
                .unwrap();
            let item = self
                .f
                .build
                .items
                .iter()
                .find(|i| i.id == use_row.item)
                .unwrap();
            assert!(
                item.modifiers
                    .iter()
                    .any(|m| m.id == modifier && m.definition == def(0x2542))
            );
            assert_eq!(
                e.key.invocation.entity,
                ConcreteEntity::EquipmentUse(equipment_use)
            );
            let EffectValue::Known {
                value: ParameterValue::Quantity(q),
            } = &e.value
            else {
                panic!("{:?}", e.value)
            };
            assert_eq!(q.unit(), &def::<UnitDefinition>(2));
            actual.push((equipment_use, q.value()));
        }
        actual.sort_by_key(|r| r.0);
        let mut expected = expected.to_vec();
        expected.sort_by_key(|r| r.0);
        assert_eq!(actual, expected);
        assert!(
            !report.values.iter().any(|v| matches!(&v.key,
            PlanValueKey::Stat { entity: ConcreteEntity::Actor(_), stat } if *stat == def(0x09d4))),
            "this component emits contributions, not final cold resistance"
        );
    }
    fn both(&self, report: &OwnedEffectsReport, value: f64) {
        self.assert_values(
            report,
            &self
                .ring_uses
                .iter()
                .map(|id| (*id, value))
                .collect::<Vec<_>>(),
        );
    }
}

// The V20 public evaluator requires checked readiness/support packages even
// for an item-only component. This deliberately empty support domain mirrors the
// existing finite command-damage fixture: these three typed channels have no
// producer, consumer occurrence, value or implied game-data coverage.
fn unused_input(name: &str) -> StatDefId {
    DefId::parse(ns(), format!("fixture.cold-item-delivery.{name}")).unwrap()
}
fn key(name: &str) -> OwnedDefinitionKey {
    name.parse().unwrap()
}
fn staged_plan(
    f: &shared::Fixture,
) -> std::result::Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>, PlanError> {
    assert_eq!(
        f.recipe.rules.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V20
    );
    assert!(f.build.gems.is_empty());
    assert!(f.build.skills.is_empty());
    assert!(f.build.supports.is_empty());
    assert!(
        f.build
            .generated_inputs
            .as_ref()
            .is_none_or(|v| v.schema_version == 1 && v.bindings.is_empty())
    );
    assert!(f.build.support_origins.as_ref().is_none_or(Vec::is_empty));
    assert_eq!(
        f.recipe.rules.effect_applications,
        Some(DeclaredSet::complete(vec![]))
    );
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
    let mut schema_input = f.recipe.schema.clone();
    assert!(!schema_input.definitions.iter().any(|d| matches!(
        d,
        DefinitionDescriptor::Gem(_) | DefinitionDescriptor::Skill(_)
    )));
    for (id, scope, value) in &unused {
        assert!(
            !schema_input
                .definitions
                .iter()
                .any(|d| d.address() == id.address())
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
    let mut rules_input = f.recipe.rules.clone();
    rules_input.definitions = schema.identity().clone();
    let stored = OwnedRulePackage::new(rules_input, schema.as_ref(), Default::default()).unwrap();
    let rules = Arc::new(
        CompiledRulePackage::compile_stored(&stored, schema.as_ref(), Default::default()).unwrap(),
    );
    let mut routing_input = f.recipe.routing.clone();
    routing_input.definitions = schema.identity().clone();
    let routing = Arc::new(
        OwnedActionRouting::new(routing_input, schema.as_ref(), Default::default()).unwrap(),
    );
    let programs: Vec<_> = stored
        .input()
        .owners
        .iter()
        .flat_map(|o| o.programs.members.iter().map(move |p| (o, p)))
        .collect();
    let stages = Arc::new(
        OwnedEvaluationStages::new(
            EvaluationStagesInput {
                schema_version: OWNED_EVALUATION_STAGES_V3,
                namespace: ns(),
                release: key("finite-cold-stages"),
                definitions: schema.identity().clone(),
                rules: *stored.identity(),
                routing: *routing.identity(),
                stages: vec![
                    EvaluationStage {
                        id: key("prepare"),
                        predecessors: vec![],
                    },
                    EvaluationStage {
                        id: key("execute"),
                        predecessors: vec![key("prepare")],
                    },
                ],
                programs: DeclaredSet::complete(
                    programs
                        .iter()
                        .map(|(o, p)| StagedRuleProgram {
                            owner: o.owner.clone(),
                            program: p.id.clone(),
                            stage: key("execute"),
                        })
                        .collect(),
                ),
                effect_applications: Some(DeclaredSet::complete(vec![])),
                routing_stage: key("execute"),
                frozen_channels: unused
                    .iter()
                    .map(|(id, scope, _)| FrozenStageChannel {
                        channel: StageChannel::Stat {
                            scope: *scope,
                            stat: id.clone(),
                        },
                        stage: key("prepare"),
                    })
                    .collect(),
                readiness: Some(ReadinessInput {
                    skills: vec![],
                    programs: DeclaredSet::complete(
                        programs
                            .iter()
                            .map(|(o, p)| ReadinessProgram {
                                owner: o.owner.clone(),
                                program: p.id.clone(),
                                phase: ReadinessPhase::Execution,
                                role: ReadinessProgramRole::Execution,
                                outputs: vec![],
                            })
                            .collect(),
                    ),
                }),
            },
            schema.as_ref(),
            &stored,
            &routing,
            Default::default(),
        )
        .unwrap(),
    );
    let preparation = Arc::new(
        OwnedSupportPreparation::new(
            SupportPreparationInput {
                schema_version: OWNED_SUPPORT_PREPARATION_VERSION,
                namespace: ns(),
                release: key("finite-cold-no-supports"),
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
                namespace: ns(),
                release: key("finite-cold-no-support-inputs"),
                definitions: schema.identity().clone(),
                rules: *stored.identity(),
                preparation: *preparation.identity(),
                stages: *stages.identity(),
                preparation_stage: key("prepare"),
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
                namespace: ns(),
                release: key("finite-cold-no-support-receivers"),
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
    let mut build = f.build.clone();
    build.support_origins = Some(vec![]);
    build.generated_inputs = Some(GeneratedSkillInputsV1 {
        schema_version: 1,
        bindings: vec![],
    });
    let limits = OwnedInputLimits::default();
    let request = OwnedEvaluationRequest::new(
        BuildSpec::new(build, limits).unwrap(),
        ScenarioSpec::new(f.scenario.clone(), limits).unwrap(),
        QuerySpec::new(
            QueryInput {
                game_version: ns(),
                requests: vec![],
            },
            limits,
        )
        .unwrap(),
        limits,
    )
    .unwrap();
    OwnedSupportEffectPlan::compile(
        SupportEffectPlanInputs {
            request: Arc::new(request),
            definitions: schema,
            rules,
            routing,
            stages,
            preparation,
            inputs,
            receiving,
        },
        Default::default(),
        Default::default(),
    )
}
fn effects(report: SupportEffectsReport) -> OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = report.outcome else {
        panic!("{report:?}")
    };
    assert_eq!(report.gaps, effects.gaps);
    effects
}

#[test]
#[ignore = "requires verified COLD_ITEM_DELIVERY_RELEASE"]
fn original_ring_inputs_and_source_range_controls_deliver_once_per_selected_use() {
    let w = World::new();
    assert_eq!(
        w.f.build.items[0].quality, None,
        "preserve actual missing raw quality"
    );
    assert_eq!(w.f.build.items[0].modifiers[0].id, w.cold);
    w.both(&w.evaluate(), 25.);
    let cases = source().vectors["native_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 4);
    for case in cases {
        let mut w = World::new();
        w.raw(case["raw_amount"].as_f64().unwrap());
        if case["name"] == "quality-twenty" {
            w.f.build.items[0].quality = Some(QualitySelection {
                kind: def(6),
                amount: FiniteQuantity::new(20., def(2)).unwrap(),
            });
        }
        assert_eq!(case["source_slots"], json!(["Ring 1", "Ring 2"]));
        w.both(&w.evaluate(), case["expected_per_use"].as_f64().unwrap());
    }
    let mut removed = World::new();
    let baseline = removed.evaluate();
    let item = &mut removed.f.build.items[0];
    let saved = item.modifiers.pop().unwrap();
    item.modifier_order.clear();
    removed.assert_values(&removed.evaluate(), &[]);
    removed.f.build.items[0].modifier_order.push(saved.id);
    removed.f.build.items[0].modifiers.push(saved);
    assert_eq!(removed.evaluate(), baseline);
}

#[test]
#[ignore = "requires verified COLD_ITEM_DELIVERY_RELEASE"]
fn source_use_removal_and_independent_item_inputs_preserve_delivery_identity() {
    let mut w = World::new();
    let baseline = w.evaluate();
    let second = w.f.build.equipment.pop().unwrap();
    w.assert_values(&w.evaluate(), &[(w.ring_uses[0], 25.)]);
    w.f.build.equipment.push(second);
    assert_eq!(w.evaluate(), baseline);

    // Independent descriptor is a synthetic control, not a second physical copy
    // inferred from the repeated source item in the unchanged original.
    let allocator = w.f.build.allocator;
    let next =
        |n| InstanceId::from_parts(allocator.lineage(), allocator.last_issued() - n).unwrap();
    let mut other = w.f.build.items[0].clone();
    other.id = ItemRecordId::from_instance_id(next(0));
    other.modifiers[0].id = ModifierInstanceId::from_instance_id(next(1));
    other.modifier_order = vec![other.modifiers[0].id];
    other.modifiers[0]
        .rolls
        .iter_mut()
        .find(|p| p.slot.slot == def(0x2543))
        .unwrap()
        .value = quantity(30.);
    w.f.build.equipment[1].item = other.id;
    w.f.build.items.push(other);
    w.assert_values(
        &w.evaluate(),
        &[(w.ring_uses[0], 25.), (w.ring_uses[1], 30.)],
    );
    w.f.build.items[0]
        .parameters
        .iter_mut()
        .find(|p| p.slot.slot == def(0x09f9))
        .unwrap()
        .value = ParameterValue::Option(def(0x09f1));
    w.assert_values(
        &w.evaluate(),
        &[(w.ring_uses[0], 30.), (w.ring_uses[1], 30.)],
    );
    w.f.build.items[1].modifiers[0]
        .rolls
        .iter_mut()
        .find(|p| p.slot.slot == def(0x2543))
        .unwrap()
        .value = quantity(20.);
    w.assert_values(
        &w.evaluate(),
        &[(w.ring_uses[0], 30.), (w.ring_uses[1], 20.)],
    );
}

#[test]
#[ignore = "requires verified COLD_ITEM_DELIVERY_RELEASE"]
fn false_missing_and_talisman_only_guards_never_authorize_numeric_delivery() {
    let mut w = World::new();
    w.guard(false);
    let report = w.evaluate();
    assert_eq!(w.cold_effects(&report).len(), 2);
    assert!(
        w.cold_effects(&report)
            .iter()
            .all(|e| e.value == EffectValue::Inactive)
    );
    let mut missing = World::new();
    missing.remove_program(APPLICABLE);
    let report = missing.evaluate();
    assert_eq!(missing.cold_effects(&report).len(), 2);
    // Missing final producers are precise effect diagnostics, not topology gaps.
    assert!(report.gaps.is_empty());
    assert!(missing.cold_effects(&report).iter().all(|e| e.value
        == EffectValue::Unresolved {
            reason: PlanGapReason::MissingProducer,
            read: Some(key("applicable")),
        }));

    // The existing Focus predicate proves only Talisman non-Amulet handling. Its
    // actual published true body must not substitute for the new numeric gate.
    let focus = source()
        .recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == subject(def::<ItemTemplateDefinition>(0x1fe3)))
        .unwrap();
    let prior_guard = focus
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == "ordinary-item-direct-applicability")
        .unwrap()
        .clone();
    assert!(prior_guard.reads.is_empty());
    missing
        .owner_mut(&subject(def::<ItemTemplateDefinition>(0x09dc)))
        .programs
        .members
        .push(prior_guard);
    let report = missing.evaluate();
    for id in &missing.ring_uses {
        assert!(report.values.iter().any(|v| v.key
            == PlanValueKey::Stat {
                entity: ConcreteEntity::EquipmentUse(*id),
                stat: def(0x3306)
            }
            && v.value
                == EffectValue::Known {
                    value: ParameterValue::Boolean(true)
                }));
    }
    assert!(
        missing
            .cold_effects(&report)
            .iter()
            .all(|e| matches!(e.value, EffectValue::Unresolved { .. }))
    );
}

#[test]
#[ignore = "requires verified COLD_ITEM_DELIVERY_RELEASE"]
fn missing_numeric_inputs_producers_and_real_partial_owners_remain_unavailable() {
    for suffix in [0x2543, 0x2674, 0x316d] {
        let mut w = World::new();
        w.f.build.items[0].modifiers[0]
            .rolls
            .retain(|p| p.slot.slot != def(suffix));
        assert!(
            matches!(w.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
        );
    }
    for suffix in [0x09f9, 0x09fa] {
        let mut w = World::new();
        w.f.build.items[0]
            .parameters
            .retain(|p| p.slot.slot != def(suffix));
        assert!(
            matches!(w.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
        );
    }
    for name in [
        "catalyst-inputs",
        "ordered-magnitude-scalar",
        "corrupted-base-factor",
        "effective-amount",
    ] {
        let mut w = World::new();
        w.remove_program(name);
        let report = w.evaluate();
        assert_eq!(w.cold_effects(&report).len(), 2);
        assert!(
            w.cold_effects(&report).iter().all(|e| matches!(
                e.value,
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    ..
                }
            )),
            "{name}"
        );
        assert!(
            report.gaps.is_empty(),
            "missing final values do not fabricate topology gaps"
        );
    }
    for index in 0..2 {
        let mut w = World::new();
        let actual = w.actual_owners[index].clone();
        if index == 0 {
            let SchemaClosure::Partial { gaps } = &actual.programs.closure else {
                panic!()
            };
            assert_eq!(
                gaps.len(),
                7,
                "restore current delivery successor residuals"
            );
        }
        let owner = actual.owner.clone();
        *w.owner_mut(&owner) = actual;
        let report = w.report();
        assert!(
            report
                .gaps
                .iter()
                .any(|g| g.reason == PlanGapReason::PartialPrograms
                    && g.subject.as_ref() == Some(&owner)),
            "the actual production owner remains Partial"
        );
        assert_eq!(
            report.outcome,
            SupportEffectsOutcome::Unavailable {
                cause: EffectValue::Unresolved {
                    reason: PlanGapReason::IncompleteContributors,
                    read: None,
                },
                input: None,
            },
            "the public staged evaluator refuses all numerical effects under actual Partial coverage"
        );
    }
}

#[test]
#[ignore = "requires verified COLD_ITEM_DELIVERY_RELEASE"]
fn fresh_reused_and_rayon_reports_match_for_actual_cold_delivery_plans() {
    let a = World::new();
    let pa = a.plan().unwrap();
    let before = a.evaluate();
    let mut b = World::new();
    b.raw(30.);
    let pb = b.plan().unwrap();
    let after = b.evaluate();
    a.both(&before, 25.);
    b.both(&after, 30.);
    assert_ne!(pa.identity(), pb.identity());
    let mut scratch = pa.new_scratch();
    assert_eq!(effects(pa.evaluate(&mut scratch).unwrap()), before);
    assert_eq!(effects(pb.evaluate(&mut scratch).unwrap()), after);
    assert_eq!(effects(pa.evaluate(&mut scratch).unwrap()), before);
    let serial: Vec<_> = (0..16)
        .map(|i| {
            if i % 2 == 0 {
                before.clone()
            } else {
                after.clone()
            }
        })
        .collect();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let parallel: Vec<_> = pool.install(|| {
        (0..16)
            .into_par_iter()
            .map(|i| {
                let plan = if i % 2 == 0 { &pa } else { &pb };
                effects(plan.evaluate(&mut plan.new_scratch()).unwrap())
            })
            .collect()
    });
    assert_eq!(parallel, serial);
}
