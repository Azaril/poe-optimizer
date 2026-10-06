//! Actual derived receiver with finite final-input producers; no native final
//! Strength/flag calculation or real contributor completeness is implied.
#[path = "owned_empty_support_domain.rs"]
mod empty_support;
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
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(ns(), format!("def.{n:016x}")).unwrap()
}
fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
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
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([0x5a; 16]), n).unwrap())
}
fn references(v: &Value, out: &mut BTreeSet<String>) {
    match v {
        Value::Object(o) => {
            if o.contains_key("namespace")
                && let Some(k) = o.get("key").and_then(Value::as_str)
            {
                out.insert(k.into());
            }
            for v in o.values() {
                references(v, out);
            }
        }
        Value::Array(a) => {
            for v in a {
                references(v, out);
            }
        }
        _ => {}
    }
}
struct Source {
    recipe: OwnedRecipeInput,
    class: ClassDefId,
    level: u16,
    actual_class: DefinitionRules,
    bindings: Value,
    vectors: Value,
}
fn source() -> &'static Source {
    static SOURCE: OnceLock<Source> = OnceLock::new();
    SOURCE.get_or_init(|| {
        let path = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_STRENGTH_LIFE_RELEASE")
                .expect("verified Strength Life endpoint"),
        );
        let before = release::inventory(&path);
        let endpoint = release::load(&path);
        family::assert_endpoint(&endpoint);
        let b: Value = family::read("bindings.json");
        let v = family::read("source-vectors.json");
        let read = |p: &str| -> Value {
            serde_json::from_slice(&fs::read(path.parent().unwrap().join(p)).unwrap()).unwrap()
        };
        let draft = read("original-05/draft.json");
        let selected = read("selected-05.json");
        let character = draft["draft"]["character_presets"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == selected["build"]["character"])
            .unwrap();
        let class: ClassDefId = decode(known(&character["class"]));
        let level = decode(known(&character["level"]));
        let actual_class = endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == subject(class.clone()))
            .unwrap()
            .clone();
        assert!(!actual_class.programs.is_complete());
        let amount: StatDefId = decode(&b["amount"]);
        let receiver = endpoint
            .input()
            .recipe
            .rules
            .receivers
            .members
            .iter()
            .find(|r| r.stat == amount)
            .unwrap()
            .clone();
        let owner = endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == subject(amount.clone()))
            .unwrap()
            .clone();
        assert_eq!(owner.programs.members.len(), 1);
        assert_eq!(owner.programs.members[0], family::program());
        assert_eq!(json!(receiver.targets), json!([{"kind":"player"}]));
        let mut keys = BTreeSet::new();
        references(&json!(actual_class), &mut keys);
        references(&json!(owner), &mut keys);
        for n in [0x31d1, 2] {
            keys.insert(format!("def.{n:016x}"));
        }
        let mut recipe = endpoint.input().recipe.clone();
        recipe
            .schema
            .definitions
            .retain(|d| keys.contains(d.address().key().as_str()));
        recipe.schema.slots.clear();
        for d in &mut recipe.schema.definitions {
            match d {
                DefinitionDescriptor::Class(row) => {
                    let SchemaState::Known(s) = &mut row.schema else {
                        panic!()
                    };
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
                _ => panic!("unexpected finite dependency"),
            }
        }
        recipe.rules.tables.clear();
        recipe.rules.receivers = DeclaredSet::complete(vec![receiver]);
        recipe.rules.effect_applications = Some(DeclaredSet::complete(vec![]));
        recipe.routing.outputs.clear();
        recipe.rules.owners = recipe
            .schema
            .definitions
            .iter()
            .map(|d| {
                let target = SchemaSubject::Definition(d.address());
                if target == owner.owner {
                    owner.clone()
                } else {
                    DefinitionRules {
                        owner: target,
                        programs: DeclaredSet::complete(vec![]),
                    }
                }
            })
            .collect();
        assert_eq!(before, release::inventory(&path));
        Source {
            recipe,
            class,
            level,
            actual_class,
            bindings: b,
            vectors: v,
        }
    })
}
fn producer(name: &str, stat: StatDefId, value: ParameterValue) -> RuleProgram {
    decode(
        &json!({"id":name,"context":"actor","reads":[],"nodes":[{"id":"value","expression":{"kind":"literal","value":value}}],"effects":[{"id":"value","when":null,"effect":{"kind":"derive","entity":"player","stat":stat,"value":"value"}}]}),
    )
}
struct World {
    recipe: OwnedRecipeInput,
    build: BuildInput,
    scenario: ScenarioInput,
}
impl World {
    fn new(strength: i64, flags: [bool; 5]) -> Self {
        let s = source();
        let mut recipe = s.recipe.clone();
        let owner = recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == subject(s.class.clone()))
            .unwrap();
        owner.programs.members.push(producer(
            "fixture-final-strength",
            decode(&s.bindings["strength"]),
            ParameterValue::Integer(BoundedInteger::new(strength).unwrap()),
        ));
        for (field, value) in family::FIELDS.iter().zip(flags) {
            owner.programs.members.push(producer(
                &format!("fixture-{field}"),
                decode(&s.bindings["inputs"][field]),
                ParameterValue::Boolean(value),
            ));
        }
        Self {
            recipe,
            build: BuildInput {
                allocator: InstanceAllocatorState::from_parts(
                    BuildLineage::from_bytes([0x5a; 16]),
                    2,
                ),
                revision: BuildRevision::from_u64(1),
                game_version: ns(),
                character: CharacterSpec {
                    class: s.class.clone(),
                    ascendancy: None,
                    level: s.level,
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
                support_origins: Some(vec![]),
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
        }
    }
    fn class_mut(&mut self) -> &mut DefinitionRules {
        let class = subject(self.build.character.class.clone());
        self.recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == class)
            .unwrap()
    }
    fn remove(&mut self, name: &str) {
        let rows = &mut self.class_mut().programs.members;
        let old = rows.len();
        rows.retain(|p| p.id.as_str() != name);
        assert_eq!(rows.len() + 1, old);
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
    fn result<'a>(&self, r: &'a OwnedEffectsReport) -> &'a BoundEffectResult {
        let rows: Vec<_> = r
            .effects
            .iter()
            .filter(|e| e.key.invocation.program.as_str() == "inherent-strength-life")
            .collect();
        assert_eq!(rows.len(), 1);
        let e = rows[0];
        assert_eq!(
            e.key.invocation.owner,
            subject(def::<StatDefinition>(0x331a))
        );
        assert_eq!(
            e.key.invocation.entity,
            ConcreteEntity::Actor(ActorKey::Player)
        );
        assert_eq!(
            e.key.invocation.origin,
            RuleOrigin::Receiver {
                receiver: key("player-inherent-strength-life"),
                actor: ActorKey::Player
            }
        );
        assert_eq!(
            e.target,
            BoundEffectTarget::Value {
                key: PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def(0x331a)
                }
            }
        );
        e
    }
    fn assert_value(&self, r: &OwnedEffectsReport, amount: f64) {
        assert!(r.gaps.is_empty(), "{:?}", r.gaps);
        assert_eq!(
            self.result(r).value,
            EffectValue::Known {
                value: ParameterValue::Quantity(FiniteQuantity::new(amount, def(0x3119)).unwrap())
            }
        );
        assert!(
            r.effects
                .iter()
                .all(|e| !matches!(e.target, BoundEffectTarget::Contribution { .. })),
            "derived receiver emits no contributions"
        );
        assert!(
            !r.values
                .iter()
                .any(|v| matches!(&v.key,PlanValueKey::Stat{stat,..}if *stat==def(0x311a))),
            "no final Life value"
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
#[ignore = "requires verified STRENGTH_LIFE_RELEASE"]
fn original_final_strength_and_parsed_controls_feed_the_actual_published_receiver() {
    let s = source();
    let cases = s.vectors["native_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 13);
    for c in cases {
        let flags = std::array::from_fn(|i| {
            let name = s.vectors["source_inputs"][i]["source_flag"]
                .as_str()
                .unwrap();
            c["flags"][name].as_bool().unwrap()
        });
        let w = World::new(c["strength"].as_i64().unwrap(), flags);
        w.assert_value(&w.evaluate(), c["inherent_life"].as_f64().unwrap());
    }
    let zero = cases
        .iter()
        .find(|c| c["name"] == "original-05-zero-strength")
        .unwrap();
    assert_eq!(zero["emitted_count"], 1);
    assert_eq!(zero["inherent_life"], 0);
    for name in [
        "original-05-disable-all",
        "original-05-disable-strength",
        "original-05-disable-strength-life",
    ] {
        let c = cases.iter().find(|c| c["name"] == name).unwrap();
        assert_eq!(c["emitted_count"], 0);
        assert_eq!(c["inherent_life"], 0);
    }
}
#[test]
#[ignore = "requires verified STRENGTH_LIFE_RELEASE"]
fn missing_final_strength_or_active_controls_never_default_and_flat_contributions_are_not_final_values()
 {
    let names = std::iter::once("fixture-final-strength".to_owned())
        .chain(family::FIELDS.map(|v| format!("fixture-{v}")));
    for name in names {
        let mut w = World::new(27, [false; 5]);
        w.remove(&name);
        let r = w.evaluate();
        assert!(r.gaps.is_empty());
        assert!(
            matches!(
                w.result(&r).value,
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    ..
                }
            ),
            "{name}"
        );
    }
    let mut w = World::new(27, [false; 5]);
    let p = w
        .class_mut()
        .programs
        .members
        .iter_mut()
        .find(|p| p.id.as_str() == "fixture-final-strength")
        .unwrap();
    p.effects[0] = decode(
        &json!({"id":"value","when":null,"effect":{"kind":"contribute","entity":"player","stat":def::<StatDefinition>(0x1d2e),"contribution":"add","value":"value"}}),
    );
    let r = w.evaluate();
    assert!(matches!(
        w.result(&r).value,
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingProducer,
            ..
        }
    ));
}
#[test]
#[ignore = "requires verified STRENGTH_LIFE_RELEASE"]
fn explicit_disabling_is_lazy_but_zero_strength_does_not_imply_missing_controls() {
    for index in 0..3 {
        let mut flags = [false; 5];
        flags[index] = true;
        let mut w = World::new(27, flags);
        w.remove("fixture-final-strength");
        for field in &family::FIELDS[index + 1..] {
            w.remove(&format!("fixture-{field}"));
        }
        w.assert_value(&w.evaluate(), 0.);
    }
    let mut zero = World::new(0, [false; 5]);
    zero.remove("fixture-inherent_doubled");
    let r = zero.evaluate();
    assert!(matches!(
        zero.result(&r).value,
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingProducer,
            ..
        }
    ));
}
#[test]
#[ignore = "requires verified STRENGTH_LIFE_RELEASE"]
fn missing_receiver_and_actual_partial_coverage_do_not_certify_real_builds() {
    let mut missing = World::new(27, [false; 5]);
    missing.recipe.rules.receivers.members.clear();
    let r = missing.evaluate();
    assert!(
        !r.effects
            .iter()
            .any(|e| e.key.invocation.program.as_str() == "inherent-strength-life")
    );
    assert!(
        !r.values
            .iter()
            .any(|v| matches!(&v.key,PlanValueKey::Stat{stat,..}if *stat==def(0x331a)))
    );
    for partial_receiver in [false, true] {
        let mut w = World::new(27, [false; 5]);
        let reason = if partial_receiver {
            w.recipe.rules.receivers.closure = SchemaClosure::Partial {
                gaps: vec![SchemaGap {
                    subject: subject(def::<StatDefinition>(0x331a)),
                    facet: SchemaFacet::GameRules,
                    code: key("fixture-unreviewed-receiver"),
                }],
            };
            PlanGapReason::PartialReceivers
        } else {
            *w.class_mut() = source().actual_class.clone();
            PlanGapReason::PartialPrograms
        };
        let report = w.report();
        assert!(report.gaps.iter().any(|g| g.reason == reason));
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
#[ignore = "requires verified STRENGTH_LIFE_RELEASE"]
fn derived_amount_is_deterministic_with_reused_and_parallel_scratch() {
    let a = World::new(27, [false; 5]);
    let b = World::new(27, [false, false, false, true, false]);
    let pa = a.plan().unwrap();
    let pb = b.plan().unwrap();
    let va = a.evaluate();
    let vb = b.evaluate();
    a.assert_value(&va, 54.);
    b.assert_value(&vb, 108.);
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
