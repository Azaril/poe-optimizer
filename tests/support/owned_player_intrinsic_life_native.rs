//! Published intrinsic Player Life bodies in an unpublished finite class domain.
//! Implicit passives, ascendancies, items, skills and external effects are excluded
//! here. Actual class Partial owners remain production authority; no final Life
//! reducer or incoming-contributor completeness is authored by this fixture.
#[path = "owned_empty_support_domain.rs"]
pub(super) mod empty_support;
use super::{family, release};
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
const PROGRAM: &str = "intrinsic-player-life";
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(ns(), format!("def.{n:016x}")).unwrap()
}
fn decode<T: DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}
fn subject<I: SchemaDefinitionId>(id: I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
fn known(v: &Value) -> &Value {
    assert_eq!(v["kind"], "known");
    &v["value"]
}
fn occurrence<T: BuildInstanceId>(n: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([0x4c; 16]), n).unwrap())
}
fn referenced_keys(v: &Value, keys: &mut BTreeSet<String>) {
    match v {
        Value::Object(o) => {
            if o.contains_key("namespace")
                && let Some(k) = o.get("key").and_then(Value::as_str)
            {
                keys.insert(k.into());
            }
            for x in o.values() {
                referenced_keys(x, keys);
            }
        }
        Value::Array(a) => {
            for x in a {
                referenced_keys(x, keys);
            }
        }
        _ => {}
    }
}
struct Source {
    recipe: OwnedRecipeInput,
    actual_owners: Vec<DefinitionRules>,
    selected: Vec<CharacterSpec>,
    vectors: Value,
    bindings: Value,
}
fn source() -> &'static Source {
    static SOURCE: OnceLock<Source> = OnceLock::new();
    SOURCE.get_or_init(|| {
        let p = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_INTRINSIC_LIFE_RELEASE")
                .expect("verified Player intrinsic Life release"),
        );
        let before = release::inventory(&p);
        let endpoint = release::load(&p);
        family::assert_endpoint(&endpoint);
        let bindings: Value = family::read("bindings.json");
        let vectors = family::read("source-vectors.json");
        let actual_owners: Vec<DefinitionRules> = bindings["classes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                let owner = subject(decode::<ClassDefId>(&c["class"]));
                let actual = endpoint
                    .input()
                    .recipe
                    .rules
                    .owners
                    .iter()
                    .find(|o| o.owner == owner)
                    .unwrap()
                    .clone();
                assert!(!actual.programs.is_complete());
                assert_eq!(actual.programs.members.len(), 3);
                assert_eq!(
                    actual
                        .programs
                        .members
                        .iter()
                        .filter(|p| p.id.as_str() == PROGRAM)
                        .count(),
                    1
                );
                actual
            })
            .collect();
        let read = |name: &str| -> Value {
            serde_json::from_slice(&fs::read(p.parent().unwrap().join(name)).unwrap()).unwrap()
        };
        let selected = (1..=5)
            .map(|i| {
                let draft = read(&format!("original-{i:02}/draft.json"));
                let request = read(&format!("selected-{i:02}.json"));
                let row = draft["draft"]["character_presets"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["id"] == request["build"]["character"])
                    .unwrap();
                CharacterSpec {
                    class: decode(known(&row["class"])),
                    level: decode(known(&row["level"])),
                    ascendancy: None,
                    rewards: vec![],
                }
            })
            .collect();
        // Keep the descriptor dependencies of all actual class programs so each
        // full Partial owner can be restored in a negative control unchanged.
        let mut keys = BTreeSet::new();
        referenced_keys(&json!(actual_owners), &mut keys);
        for id in [0x311a, 0x3119, 2, 0x31d1] {
            keys.insert(format!("def.{id:016x}"));
        }
        let mut recipe = endpoint.input().recipe.clone();
        recipe
            .schema
            .definitions
            .retain(|d| keys.contains(d.address().key().as_str()));
        for d in &mut recipe.schema.definitions {
            match d {
                DefinitionDescriptor::Class(row) => {
                    let SchemaState::Known(s) = &mut row.schema else {
                        panic!()
                    };
                    assert_eq!(
                        s.level,
                        decode::<IntegerRange>(&json!({"minimum":1,"maximum":100}))
                    );
                    assert!(s.declarations.parameters.is_complete());
                    assert!(s.declarations.parameters.members.is_empty());
                    s.ascendancies = DeclaredSet::complete(vec![]);
                    s.implicit_passives = DeclaredSet::complete(vec![]);
                }
                DefinitionDescriptor::Encounter(row) => {
                    let SchemaState::Known(s) = &mut row.schema else {
                        panic!()
                    };
                    s.external_inputs = DeclaredSet::complete(vec![]);
                }
                DefinitionDescriptor::Stat(_) | DefinitionDescriptor::Unit(_) => {}
                _ => panic!("unexpected class program dependency"),
            }
        }
        recipe.schema.slots.clear();
        recipe.rules.tables.clear();
        recipe.rules.receivers = DeclaredSet::complete(vec![]);
        recipe.rules.effect_applications = Some(DeclaredSet::complete(vec![]));
        recipe.routing.outputs.clear();
        recipe.rules.owners = recipe
            .schema
            .definitions
            .iter()
            .map(|d| {
                let owner = SchemaSubject::Definition(d.address());
                let programs =
                    actual_owners
                        .iter()
                        .find(|o| o.owner == owner)
                        .map_or_else(Vec::new, |o| {
                            o.programs
                                .members
                                .iter()
                                .filter(|p| p.id.as_str() == PROGRAM)
                                .cloned()
                                .collect()
                        });
                DefinitionRules {
                    owner,
                    programs: DeclaredSet::complete(programs),
                }
            })
            .collect();
        assert_eq!(before, release::inventory(&p));
        Source {
            recipe,
            actual_owners,
            selected,
            vectors,
            bindings,
        }
    })
}
struct World {
    recipe: OwnedRecipeInput,
    build: BuildInput,
    scenario: ScenarioInput,
}
/// Finite intrinsic-only component; no production Class coverage is inferred.
pub(super) fn finite_parts(
    class: ClassDefId,
    level: u16,
) -> (OwnedRecipeInput, BuildInput, ScenarioInput) {
    let world = World::assemble(class, level);
    (world.recipe, world.build, world.scenario)
}
impl World {
    fn new(class: ClassDefId, level: u16) -> Self {
        let (recipe, build, scenario) = finite_parts(class, level);
        Self {
            recipe,
            build,
            scenario,
        }
    }
    fn assemble(class: ClassDefId, level: u16) -> Self {
        Self {
            recipe: source().recipe.clone(),
            build: BuildInput {
                allocator: InstanceAllocatorState::from_parts(
                    BuildLineage::from_bytes([0x4c; 16]),
                    2,
                ),
                revision: BuildRevision::from_u64(1),
                game_version: ns(),
                character: CharacterSpec {
                    class,
                    ascendancy: None,
                    level,
                    rewards: vec![],
                },
                weapon_loadouts: vec![occurrence(1)],
                active_weapon_loadout: occurrence(1),
                items: vec![],
                gems: vec![],
                equipment: vec![],
                allocations: vec![],
                skills: vec![],
                supports: vec![],
                payload_links: vec![],
                choices: vec![],
                authored_support_order: Some(vec![]),
                generated_inputs: Some(GeneratedSkillInputsV1 {
                    schema_version: 1,
                    bindings: vec![],
                }),
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
        }
    }
    fn owner_mut(&mut self) -> &mut DefinitionRules {
        let target = subject(self.build.character.class.clone());
        self.recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == target)
            .unwrap()
    }
    fn plan(
        &self,
    ) -> std::result::Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>, PlanError> {
        empty_support::compile(&self.recipe, &self.build, &self.scenario, def(2))
    }
    fn report(&self) -> SupportEffectsReport {
        let p = self.plan().unwrap();
        p.evaluate(&mut p.new_scratch()).unwrap()
    }
    fn evaluate(&self) -> OwnedEffectsReport {
        effects(self.report())
    }
    fn assert_value(&self, r: &OwnedEffectsReport, value: f64) {
        assert!(r.gaps.is_empty(), "{:?}", r.gaps);
        assert_eq!(r.effects.len(), 1);
        let e = &r.effects[0];
        assert_eq!(e.key.invocation.program.as_str(), PROGRAM);
        assert_eq!(
            e.key.invocation.owner,
            subject(self.build.character.class.clone())
        );
        assert_eq!(
            e.key.invocation.entity,
            ConcreteEntity::Actor(ActorKey::Player)
        );
        assert_eq!(
            e.key.invocation.origin,
            RuleOrigin::Provider {
                provider: ProviderKey {
                    root: ProviderRoot::Character,
                    grant_path: vec![]
                }
            }
        );
        assert_eq!(
            e.target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def(0x311a),
                    kind: ContributionKind::Add
                }
            }
        );
        assert_eq!(
            e.value,
            EffectValue::Known {
                value: ParameterValue::Quantity(FiniteQuantity::new(value, def(0x3119)).unwrap())
            }
        );
        assert!(
            !r.values
                .iter()
                .any(|v| matches!(&v.key,PlanValueKey::Stat{stat,..} if *stat==def(0x311a))),
            "intrinsic contribution must not appear as final Life"
        );
    }
}
fn effects(r: SupportEffectsReport) -> OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = r.outcome else {
        panic!("{r:?}")
    };
    assert_eq!(r.gaps, effects.gaps);
    effects
}
#[test]
#[ignore = "requires verified PLAYER_INTRINSIC_LIFE_RELEASE"]
fn all_five_selected_character_inputs_and_source_level_controls_match_intrinsic_contributions() {
    let s = source();
    let cases = s.vectors["native_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 10);
    for (i, c) in cases.iter().enumerate() {
        let source_class = &s.vectors["projections"][i]["state"]["modes"]["MAIN"]["class_id"];
        let binding = s.bindings["classes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["source_id"] == *source_class)
            .unwrap();
        let class: ClassDefId = decode(&binding["class"]);
        let level: u16 = decode(&c["character_level"]);
        if i < 5 {
            assert_eq!(s.selected[i].class, class);
            assert_eq!(s.selected[i].level, level);
        }
        let w = World::new(class, level);
        w.assert_value(&w.evaluate(), c["intrinsic_life"].as_f64().unwrap());
    }
}
#[test]
#[ignore = "requires verified PLAYER_INTRINSIC_LIFE_RELEASE"]
fn the_shared_intrinsic_rule_delivers_once_for_each_of_eight_class_selections() {
    // All-class applicability is pinned to the shared source initializer outside
    // its class-attribute loop. These are native selection/level boundary checks.
    for c in source().bindings["classes"].as_array().unwrap() {
        for (level, value) in [(1, 28.), (92, 1120.), (100, 1216.)] {
            let w = World::new(decode(&c["class"]), level);
            w.assert_value(&w.evaluate(), value);
        }
    }
    // Typed native input rejects values outside the actual Class schema. It does
    // not reproduce the source initializer's permissive clamp to 1..100.
    for level in [0, 101] {
        let w = World::new(source().selected[4].class.clone(), level);
        assert!(
            matches!(w.plan(),Err(PlanError::Invalid(ref message)) if message=="owned request has invalid schema bindings"),
            "level {level} must be rejected"
        );
    }
}
#[test]
#[ignore = "requires verified PLAYER_INTRINSIC_LIFE_RELEASE"]
fn missing_body_and_actual_partial_class_owners_never_supply_a_default_or_final_life() {
    for actual in &source().actual_owners {
        let SchemaSubject::Definition(DefinitionAddress::Class(class)) = &actual.owner else {
            panic!()
        };
        let mut w = World::new(class.clone(), 92);
        let exact = w.owner_mut().programs.members.clone();
        w.owner_mut().programs.members.clear();
        let missing = w.evaluate();
        assert!(missing.effects.is_empty());
        assert!(missing.gaps.is_empty());
        assert!(
            !missing
                .values
                .iter()
                .any(|v| matches!(&v.key,PlanValueKey::Stat{stat,..} if *stat==def(0x311a)))
        );
        w.owner_mut().programs.members = exact;
        w.assert_value(&w.evaluate(), 1120.);
        *w.owner_mut() = actual.clone();
        let report = w.report();
        assert!(
            report
                .gaps
                .iter()
                .any(|g| g.reason == PlanGapReason::PartialPrograms
                    && g.subject.as_ref() == Some(&actual.owner))
        );
        assert_eq!(
            report.outcome,
            SupportEffectsOutcome::Unavailable {
                cause: EffectValue::Unresolved {
                    reason: PlanGapReason::IncompleteContributors,
                    read: None
                },
                input: None
            }
        );
    }
}
#[test]
#[ignore = "requires verified PLAYER_INTRINSIC_LIFE_RELEASE"]
fn fresh_reused_and_rayon_intrinsic_life_reports_are_identical() {
    let c = source().selected[4].class.clone();
    let a = World::new(c.clone(), 92);
    let b = World::new(c, 91);
    let pa = a.plan().unwrap();
    let pb = b.plan().unwrap();
    let va = a.evaluate();
    let vb = b.evaluate();
    a.assert_value(&va, 1120.);
    b.assert_value(&vb, 1108.);
    assert_ne!(pa.identity(), pb.identity());
    let mut scratch = pa.new_scratch();
    assert_eq!(effects(pa.evaluate(&mut scratch).unwrap()), va);
    assert_eq!(effects(pb.evaluate(&mut scratch).unwrap()), vb);
    assert_eq!(effects(pa.evaluate(&mut scratch).unwrap()), va);
    let serial: Vec<_> = (0..16)
        .map(|i| if i % 2 == 0 { va.clone() } else { vb.clone() })
        .collect();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let parallel: Vec<_> = pool.install(|| {
        (0..16)
            .into_par_iter()
            .map(|i| {
                let p = if i % 2 == 0 { &pa } else { &pb };
                effects(p.evaluate(&mut p.new_scratch()).unwrap())
            })
            .collect()
    });
    assert_eq!(parallel, serial);
}
