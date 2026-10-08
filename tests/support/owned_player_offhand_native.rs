//! Actual published structural off-hand programs in an explicit finite domain.
//! Other Class/Item/Actor mechanics, physical stock, legality, source item
//! preparation and effective modifier conditions are not completed by this test.
use super::{empty_support, family, release};
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::owned_recipe::OwnedRecipeInput;
use rayon::prelude::*;
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

#[derive(Clone, Deserialize)]
struct Template {
    source_base: String,
    item_type: String,
    template: ItemTemplateDefId,
}
#[derive(Deserialize)]
struct Capabilities {
    shield: CapabilityDefId,
    focus: CapabilityDefId,
}
#[derive(Deserialize)]
struct Stats {
    empty: StatDefId,
    shield: StatDefId,
    focus: StatDefId,
}
#[derive(Deserialize)]
struct Bindings {
    actor: ActorDefId,
    slot: EquipmentSlotDefId,
    capabilities: Capabilities,
    stats: Stats,
    template_program: String,
    player_program: String,
    templates: Vec<Template>,
}
struct Source {
    recipe: OwnedRecipeInput,
    bindings: Bindings,
    class: ClassDefId,
}
fn source() -> &'static Source {
    static SOURCE: OnceLock<Source> = OnceLock::new();
    SOURCE.get_or_init(|| {
        let path = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_OFFHAND_RELEASE")
                .expect("verified off-hand release"),
        );
        let before = release::inventory(&path);
        let staged = release::load(&path);
        family::assert_endpoint(&staged);
        let recipe = staged.input().recipe.clone();
        let bindings: Bindings = family::read("bindings.json");
        assert_eq!(bindings.templates.len(), 1756);
        assert_eq!(
            bindings
                .templates
                .iter()
                .filter(|t| t.item_type == "Shield")
                .count(),
            193
        );
        assert_eq!(
            bindings
                .templates
                .iter()
                .filter(|t| t.item_type == "Focus")
                .count(),
            51
        );
        let class = recipe
            .schema
            .definitions
            .iter()
            .find_map(|d| match d {
                DefinitionDescriptor::Class(row) => Some(row.id.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(before, release::inventory(&path));
        Source {
            recipe,
            bindings,
            class,
        }
    })
}
fn ns() -> GameVersionNamespace {
    source().recipe.schema.namespace.clone()
}
fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(ns(), format!("def.{n:016x}")).unwrap()
}
fn subject<I: SchemaDefinitionId>(id: I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
fn key(name: &str) -> OwnedDefinitionKey {
    name.parse().unwrap()
}
fn occurrence<T: BuildInstanceId>(n: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([0x6f; 16]), n).unwrap())
}
fn both_loadouts() -> LoadoutScope {
    LoadoutScope::Selected {
        loadouts: vec![occurrence(1), occurrence(2)],
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
fn actual_owner(owner: &SchemaSubject) -> &DefinitionRules {
    source()
        .recipe
        .rules
        .owners
        .iter()
        .find(|o| &o.owner == owner)
        .unwrap()
}
fn template(base: &str) -> &Template {
    source()
        .bindings
        .templates
        .iter()
        .find(|t| t.source_base == base)
        .unwrap()
}
fn sample(item_type: &str) -> &'static str {
    &source()
        .bindings
        .templates
        .iter()
        .find(|t| t.item_type == item_type)
        .unwrap()
        .source_base
}
type Plan = OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>;
#[derive(Clone)]
struct World {
    recipe: OwnedRecipeInput,
    build: BuildInput,
    scenario: ScenarioInput,
}
impl World {
    fn new(bases: &[&str]) -> Self {
        let s = source();
        let templates: BTreeSet<_> = bases.iter().map(|b| template(b).template.clone()).collect();
        let ids = [
            s.class.address(),
            s.bindings.actor.address(),
            s.bindings.slot.address(),
            def::<EquipmentSlotDefinition>(0x64).address(),
            s.bindings.capabilities.shield.address(),
            s.bindings.capabilities.focus.address(),
            s.bindings.stats.empty.address(),
            s.bindings.stats.shield.address(),
            s.bindings.stats.focus.address(),
            def::<EncounterDefinition>(0x31d1).address(),
            def::<UnitDefinition>(2).address(),
            def::<UnitDefinition>(3).address(),
            def::<UnitDefinition>(0x3119).address(),
            def::<MetricDefinition>(0x312d).address(),
            def::<MetricDefinition>(0x312e).address(),
        ];
        let mut recipe = s.recipe.clone();
        recipe.schema.definitions.retain(|d| {
            ids.contains(&d.address())
                || matches!(d, DefinitionDescriptor::ItemTemplate(e) if templates.contains(&e.id))
        });
        for d in &mut recipe.schema.definitions {
            match d {
                DefinitionDescriptor::Class(e) => {
                    let SchemaState::Known(v) = &mut e.schema else {
                        panic!("known class")
                    };
                    v.ascendancies = DeclaredSet::complete(vec![]);
                    v.implicit_passives = DeclaredSet::complete(vec![]);
                    v.declarations = ports();
                }
                DefinitionDescriptor::ItemTemplate(e) => {
                    let SchemaState::Known(v) = &mut e.schema else {
                        panic!("known template")
                    };
                    // These selected descriptors are a test-only relation domain:
                    // no assertion of game placement, quality, rolls or stock.
                    v.equipment_slots =
                        DeclaredSet::complete(vec![def(0x64), s.bindings.slot.clone()]);
                    v.socket_destinations = DeclaredSet::complete(vec![]);
                    v.modifiers = DeclaredSet::complete(vec![]);
                    v.quality = QualityUseSchema {
                        presence: QualityPresence::Optional,
                        allowed_kinds: DeclaredSet::complete(vec![]),
                    };
                    v.declarations = ports();
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
        recipe.schema.slots.clear();
        recipe.rules.owners = recipe
            .schema
            .definitions
            .iter()
            .map(|d| {
                let owner = SchemaSubject::Definition(d.address());
                let program = match d {
                    DefinitionDescriptor::Actor(_) => Some(s.bindings.player_program.as_str()),
                    DefinitionDescriptor::ItemTemplate(_) => {
                        Some(s.bindings.template_program.as_str())
                    }
                    _ => None,
                };
                let programs = program.map_or_else(Vec::new, |name| {
                    let original = actual_owner(&owner);
                    assert!(!original.programs.is_complete());
                    let matches: Vec<_> = original
                        .programs
                        .members
                        .iter()
                        .filter(|p| p.id.as_str() == name)
                        .cloned()
                        .collect();
                    assert_eq!(matches.len(), 1);
                    matches
                });
                DefinitionRules {
                    owner,
                    programs: DeclaredSet::complete(programs),
                }
            })
            .collect();
        recipe.rules.tables.clear();
        recipe.rules.receivers = DeclaredSet::complete(vec![]);
        recipe.rules.effect_applications = Some(DeclaredSet::complete(vec![]));
        recipe.rules.contribution_queries = Some(DeclaredSet::complete(vec![]));
        let registry = recipe.rules.existing_actor_rules.as_ref().unwrap();
        assert!(registry.is_complete());
        assert_eq!(registry.members.len(), 1);
        assert_eq!(registry.members[0].owner, s.bindings.actor);
        assert_eq!(
            registry.members[0].targets,
            [ExistingActorRuleTarget::Player]
        );
        recipe.routing.outputs.clear();
        let mut world = Self {
            recipe,
            build: BuildInput {
                allocator: InstanceAllocatorState::from_parts(
                    BuildLineage::from_bytes([0x6f; 16]),
                    100,
                ),
                revision: BuildRevision::from_u64(1),
                game_version: ns(),
                character: CharacterSpec {
                    class: s.class.clone(),
                    ascendancy: None,
                    level: 80,
                    rewards: vec![],
                },
                weapon_loadouts: vec![occurrence(1), occurrence(2)],
                active_weapon_loadout: occurrence(1),
                items: vec![],
                gems: vec![],
                equipment: vec![],
                allocations: vec![],
                skills: vec![],
                supports: vec![],
                authored_support_order: Some(vec![]),
                generated_inputs: Some(GeneratedSkillInputsV1 {
                    schema_version: 1,
                    bindings: vec![],
                }),
                payload_links: vec![],
                choices: vec![],
            },
            scenario: ScenarioInput {
                game_version: ns(),
                enemy: EnemySpec {
                    encounter: def(0x31d1),
                    level: 20,
                },
                assumptions: vec![],
                usage: vec![],
            },
        };
        for (i, base) in bases.iter().enumerate() {
            world.build.items.push(ItemRecord {
                id: occurrence(10 + i as u64),
                template: template(base).template.clone(),
                parameters: vec![],
                item_level: None,
                quality: None,
                modifiers: vec![],
                modifier_order: vec![],
            });
        }
        world
    }
    fn equip(&mut self, item: usize, use_id: u64, offhand: bool, scope: LoadoutScope) {
        self.build.equipment.push(EquipmentUse {
            id: occurrence(use_id),
            item: self.build.items[item].id,
            destination: EquipmentDestination::CharacterSlot(if offhand {
                source().bindings.slot.clone()
            } else {
                def(0x64)
            }),
            scope,
        });
    }
    fn owner_mut(&mut self, subject: &SchemaSubject) -> &mut DefinitionRules {
        self.recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| &o.owner == subject)
            .unwrap()
    }
    fn plan(&self) -> std::result::Result<Plan, PlanError> {
        empty_support::compile(&self.recipe, &self.build, &self.scenario, def(2))
    }
    fn plan_with_queries(&self, queries: &QueryInput) -> Plan {
        empty_support::compile_with_queries(
            &self.recipe,
            &self.build,
            &self.scenario,
            def(2),
            queries,
        )
        .unwrap()
    }
    fn report(&self) -> SupportEffectsReport {
        let plan = self.plan().unwrap();
        plan.evaluate(&mut plan.new_scratch()).unwrap()
    }
}
fn effects(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects, .. } = &report.outcome else {
        panic!("expected finite evaluated component: {:?}", report.gaps)
    };
    effects
}
fn player_value<'a>(report: &'a OwnedEffectsReport, stat: &StatDefId) -> &'a EffectValue {
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
fn boolean(value: bool) -> EffectValue {
    EffectValue::Known {
        value: ParameterValue::Boolean(value),
    }
}
fn assert_facts(report: &SupportEffectsReport, expected: [bool; 3]) {
    let r = effects(report);
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    let s = &source().bindings;
    for (stat, expected) in [&s.stats.empty, &s.stats.shield, &s.stats.focus]
        .into_iter()
        .zip(expected)
    {
        assert_eq!(player_value(r, stat), &boolean(expected));
    }
    let actual: Vec<_> = r
        .effects
        .iter()
        .filter(|e| e.key.invocation.program.as_str() == s.player_program)
        .collect();
    assert_eq!(actual.len(), 3, "one shared Player application");
    let application = &source()
        .recipe
        .rules
        .existing_actor_rules
        .as_ref()
        .unwrap()
        .members[0];
    for e in actual {
        assert_eq!(e.key.invocation.owner, subject(s.actor.clone()));
        assert_eq!(
            e.key.invocation.entity,
            ConcreteEntity::Actor(ActorKey::Player)
        );
        assert_eq!(
            e.key.invocation.origin,
            RuleOrigin::ExistingActor {
                application: application.id.clone(),
                actor: ActorKey::Player,
            }
        );
    }
}
fn assert_unresolved(report: &SupportEffectsReport, names: &[&StatDefId], reason: PlanGapReason) {
    for stat in names {
        assert!(
            matches!(player_value(effects(report), stat), EffectValue::Unresolved {
            reason: actual, ..
        } if *actual == reason)
        );
    }
}

#[test]
#[ignore = "requires PLAYER_OFFHAND_RELEASE"]
fn published_empty_shield_focus_and_other_are_selected_equipment_facts() {
    assert_facts(&World::new(&[]).report(), [true, false, false]);
    for (kind, expected) in [
        ("Shield", [false, true, false]),
        ("Focus", [false, false, true]),
        ("Sceptre", [false, false, false]),
    ] {
        let mut world = World::new(&[sample(kind)]);
        world.equip(0, 30, true, both_loadouts());
        assert_facts(&world.report(), expected);
        // An unchanged template in the opposite hand does not classify off-hand.
        world.build.equipment.clear();
        world.equip(0, 30, false, both_loadouts());
        assert_facts(&world.report(), [true, false, false]);
    }
}

#[test]
#[ignore = "requires PLAYER_OFFHAND_RELEASE"]
fn exact_uses_and_active_loadouts_bind_without_item_or_query_order_authority() {
    let mut world = World::new(&[sample("Focus"), sample("Shield")]);
    world.equip(0, 30, false, both_loadouts());
    // The same rolled record supports independent exact uses in this finite test.
    world.equip(
        0,
        31,
        true,
        LoadoutScope::Selected {
            loadouts: vec![occurrence(1)],
        },
    );
    world.equip(
        1,
        32,
        true,
        LoadoutScope::Selected {
            loadouts: vec![occurrence(2)],
        },
    );
    let a = world.report();
    assert_facts(&a, [false, false, true]);
    let focus = &source().bindings.capabilities.focus;
    for id in [30, 31] {
        assert!(effects(&a).values.iter().any(|v| v.key
            == PlanValueKey::Capability {
                entity: ConcreteEntity::EquipmentUse(occurrence(id)),
                capability: focus.clone()
            }
            && v.value == boolean(true)));
    }
    assert!(
        !effects(&a)
            .effects
            .iter()
            .any(|e| e.key.invocation.entity == ConcreteEntity::EquipmentUse(occurrence(32)))
    );
    world.build.active_weapon_loadout = occurrence(2);
    assert_facts(&world.report(), [false, true, false]);
    world.build.equipment.reverse();
    world.build.items.reverse();
    assert_facts(&world.report(), [false, true, false]);
}

#[test]
#[ignore = "requires PLAYER_OFFHAND_RELEASE"]
fn absent_capability_ambiguity_unknown_provider_and_real_partial_coverage_do_not_default() {
    let s = &source().bindings;
    let mut missing = World::new(&[sample("Focus")]);
    missing.equip(0, 30, true, both_loadouts());
    let owner = subject(missing.build.items[0].template.clone());
    missing.owner_mut(&owner).programs.members[0].effects.retain(|e|
        !matches!(&e.effect, RuleEffectKind::Capability { capability, .. } if capability == &s.capabilities.focus));
    let report = missing.report();
    assert_eq!(
        player_value(effects(&report), &s.stats.empty),
        &boolean(false)
    );
    assert_eq!(
        player_value(effects(&report), &s.stats.shield),
        &boolean(false)
    );
    assert_unresolved(&report, &[&s.stats.focus], PlanGapReason::MissingProducer);
    missing.build.equipment.clear();
    assert_facts(&missing.report(), [true, false, false]);

    let mut ambiguous = World::new(&[sample("Focus")]);
    ambiguous.equip(0, 30, true, both_loadouts());
    ambiguous.equip(0, 31, true, both_loadouts());
    assert_unresolved(
        &ambiguous.report(),
        &[&s.stats.empty, &s.stats.shield, &s.stats.focus],
        PlanGapReason::UnsupportedRelation,
    );

    let mut unknown = World::new(&[sample("Focus")]);
    unknown.equip(0, 30, true, both_loadouts());
    let owner = subject(unknown.build.items[0].template.clone());
    for d in &mut unknown.recipe.schema.definitions {
        if let DefinitionDescriptor::ItemTemplate(e) = d {
            e.schema = SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: owner.clone(),
                    facet: SchemaFacet::InputSchema,
                    code: key("fixture-unknown-template"),
                }],
            };
        }
    }
    unknown.recipe.rules.owners.retain(|o| o.owner != owner);
    let report = unknown.report();
    assert!(matches!(
        report.outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::SchemaUnresolved)
    );

    // Restore exact actual production closure independently of finite program bodies.
    for actor in [false, true] {
        let mut world = World::new(&[sample("Focus")]);
        world.equip(0, 30, true, both_loadouts());
        let owner = if actor {
            subject(s.actor.clone())
        } else {
            subject(world.build.items[0].template.clone())
        };
        let actual = actual_owner(&owner).programs.closure.clone();
        assert!(!actual_owner(&owner).programs.is_complete());
        world.owner_mut(&owner).programs.closure = actual;
        let report = world.report();
        assert!(matches!(
            report.outcome,
            SupportEffectsOutcome::Unavailable { .. }
        ));
        assert!(
            report
                .gaps
                .iter()
                .any(|g| g.reason == PlanGapReason::PartialPrograms
                    && g.subject.as_ref() == Some(&owner))
        );
    }
}

#[test]
#[ignore = "requires PLAYER_OFFHAND_RELEASE"]
fn shared_facts_are_query_independent_and_deterministic_on_fresh_reused_and_rayon_workers() {
    let mut a = World::new(&[sample("Focus")]);
    a.equip(0, 30, true, both_loadouts());
    let mut b = a.clone();
    b.build.equipment.clear();
    let plans = [Arc::new(a.plan().unwrap()), Arc::new(b.plan().unwrap())];
    let expected: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    let mut scratch = plans[0].new_scratch();
    for i in [0, 1, 0] {
        assert_eq!(plans[i].evaluate(&mut scratch).unwrap(), expected[i]);
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let parallel: Vec<_> = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map_init(
                || plans[0].new_scratch(),
                |scratch, i| (i % 2, plans[i % 2].evaluate(scratch).unwrap()),
            )
            .collect()
    });
    for (i, report) in parallel {
        assert_eq!(report, expected[i]);
    }
    for requests in [
        vec![0x312d, 0x312e],
        vec![0x312e, 0x312d],
        vec![0x312e],
        vec![],
    ] {
        let query = QueryInput {
            game_version: ns(),
            requests: requests
                .into_iter()
                .map(|n| MetricRequest {
                    id: QueryId::new(format!("offhand-query-{n}")).unwrap(),
                    metric: def(n),
                    target: MetricTarget::Actor(ActorKey::Player),
                })
                .collect(),
        };
        let plan = a.plan_with_queries(&query);
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert_facts(&report, [false, false, true]);
        assert_eq!(effects(&report).effects, effects(&expected[0]).effects);
        assert_eq!(effects(&report).values, effects(&expected[0]).values);
    }
}

#[test]
#[ignore = "requires PLAYER_OFFHAND_RELEASE and authenticated source vectors"]
fn all_five_originals_and_source_controls_match_selected_facts_not_prepared_condition_truth() {
    let vectors: serde_json::Value = family::read("source-vectors.json");
    let cases = vectors["native_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 10);
    let mut original_names = BTreeSet::new();
    let mut filtered = 0;
    for case in cases {
        let name = case["name"].as_str().unwrap();
        if name.len() == "original-01".len() {
            original_names.insert(name);
        }
        let snapshots = [
            &case["selected"]["weapon_one"],
            &case["selected"]["weapon_two"],
        ];
        let bases: Vec<_> = snapshots
            .iter()
            .filter(|s| s["present"] == true)
            .map(|s| {
                assert_eq!(s["exact_catalogue_base"], true, "{name}");
                s["base_name"].as_str().unwrap()
            })
            .collect();
        let mut world = World::new(&bases);
        let loadout = case["source"]["weapon_loadout"].as_u64().unwrap();
        assert!([1, 2].contains(&loadout));
        world.build.active_weapon_loadout = occurrence(loadout);
        let mut item_index = 0;
        for (hand, snapshot) in snapshots.into_iter().enumerate() {
            if snapshot["present"] == true {
                let source_hand = if hand == 0 {
                    "weapon_one"
                } else {
                    "weapon_two"
                };
                assert_eq!(
                    snapshot["source_item_id"]["value"], case["source"][source_hand]["item_id"],
                    "{name}"
                );
                assert_eq!(
                    snapshot["type"],
                    template(snapshot["base_name"].as_str().unwrap()).item_type,
                    "{name}"
                );
                world.equip(
                    item_index,
                    30 + hand as u64,
                    hand == 1,
                    LoadoutScope::Selected {
                        loadouts: vec![occurrence(loadout)],
                    },
                );
                item_index += 1;
            }
        }
        let facts = &case["selected"]["facts"];
        assert_facts(
            &world.report(),
            [
                facts["empty"].as_bool().unwrap(),
                facts["shield"].as_bool().unwrap(),
                facts["focus"].as_bool().unwrap(),
            ],
        );
        let prepared = case["prepared"].as_array().unwrap();
        assert_eq!(prepared.len(), 2);
        if case["filtering_control"] == true {
            filtered += 1;
            assert_eq!(facts["focus"], true);
            assert!(prepared.iter().all(|p| p["same_selected_item"] == false
                && p["item"]["present"] == false
                && p["facts"]["empty"] == true));
        } else {
            assert!(
                prepared
                    .iter()
                    .all(|p| p["same_selected_item"] == true && &p["facts"] == facts),
                "{name}"
            );
        }
    }
    assert_eq!(
        original_names,
        BTreeSet::from([
            "original-01",
            "original-02",
            "original-03",
            "original-04",
            "original-05"
        ])
    );
    assert_eq!(
        filtered, 1,
        "saved occupancy does not reproduce source filtering"
    );
}
