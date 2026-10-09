//! Intrinsic Mana parity and checked release publication; no final pool claim.
#[path = "support/owned_empty_support_domain.rs"]
mod empty_support;
#[path = "support/owned_player_intrinsic_mana.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_plan_replay.rs"]
mod replay;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::owned_recipe::OwnedRecipeInput;
use rayon::prelude::*;
use serde_json::{Value, json};
use std::{fs, path::PathBuf, sync::OnceLock};

#[test]
fn authored_shared_player_mana_reuses_existing_channel() {
    family::check_authored();
}
#[test]
#[ignore = "requires PLAYER_INTRINSIC_MANA_PRIOR/OUTPUT and retained independent source reports"]
fn publish_mana_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_INTRINSIC_MANA_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_PLAYER_INTRINSIC_MANA_OUTPUT").unwrap()),
        &family::data(),
        &[],
        &["authoring.json", "source-vectors.json", "dependencies.json"],
        family::stage,
        json!({"new_definitions":0,"new_programs":1,"final_mana":false,"whole_build_parity":false}),
        [107, 117, 109, 123, 4],
    );
}
fn def<K: DefinitionDomain>(id: u64) -> DefId<K> {
    DefId::parse(
        GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap(),
        format!("def.{id:016x}"),
    )
    .unwrap()
}
#[derive(Clone)]
struct World {
    recipe: OwnedRecipeInput,
    build: BuildInput,
    scenario: ScenarioInput,
}
fn finite_domain() -> &'static World {
    static WORLD: OnceLock<World> = OnceLock::new();
    WORLD.get_or_init(|| {
        let bytes = fs::read(family::root().join("tests/fixtures/owned-sniper-replay.json.gz")).unwrap();
        let base = replay::ReplayInput::decode(&bytes);
        let deps: family::Dependencies = family::read("dependencies.json");
        let m = family::migration();
        let mut schema = base.schema;
        schema.definitions = deps.definitions;
        schema.slots.clear();
        // Explicit test-only domain: no equipment, gems, skills, passives or
        // ascendancies. Production descriptors and Partial owners are untouched.
        for d in &mut schema.definitions {
            match d {
                DefinitionDescriptor::Class(c) => {
                    let SchemaState::Known(c) = &mut c.schema else { panic!() };
                    c.ascendancies = DeclaredSet::complete(vec![]);
                    c.implicit_passives = DeclaredSet::complete(vec![]);
                }
                DefinitionDescriptor::Encounter(e) => {
                    let SchemaState::Known(e) = &mut e.schema else { panic!() };
                    e.external_inputs = DeclaredSet::complete(vec![]);
                }
                _ => {}
            }
        }
        let mut rules = base.rules;
        // This program uses only already-implemented V22 capabilities. The
        // release itself remains V25 and is independently validated above.
        rules.operations_version = OWNED_RULE_OPERATIONS_V22.parse().unwrap();
        rules.tables.clear();
        rules.receivers = DeclaredSet::complete(vec![]);
        rules.effect_applications = Some(DeclaredSet::complete(vec![]));
        rules.contribution_queries = Some(DeclaredSet::complete(vec![]));
        rules.support_discovery = Some(SupportDiscoveryInput {
            providers: schema.definitions.iter().map(|d| SupportSourceDomainDeclaration {
                owner: SchemaSubject::Definition(d.address()),
                domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
            }).collect(),
        });
        rules.existing_actor_rules = Some(deps.existing_actor_rules);
        rules.owners = schema.definitions.iter().map(|d| DefinitionRules {
            owner: SchemaSubject::Definition(d.address()),
            programs: DeclaredSet::complete(
                if SchemaSubject::Definition(d.address()) == m.owners[0].owner {
                    m.owners[0].programs.members.clone()
                } else {
                    vec![]
                }
            ),
        }).collect();
        let mut routing = base.routing;
        routing.outputs.clear();
        let registry = serde_json::from_value(json!({
            "schema_version":1,"namespace":schema.namespace,"revision":0,"last_issued":0,"entries":[]
        })).unwrap();
        let recipe = OwnedRecipeInput { schema_version: 1, registry, schema, rules, routing };
        let mut build = base.build;
        build.character.ascendancy = None;
        build.character.rewards.clear();
        build.items.clear();
        build.gems.clear();
        build.equipment.clear();
        build.allocations.clear();
        build.skills.clear();
        build.supports.clear();
        build.payload_links.clear();
        build.choices.clear();
        build.authored_support_order = Some(vec![]);
        build.generated_inputs = Some(GeneratedSkillInputsV1 { schema_version: 1, bindings: vec![] });
        let mut scenario = base.scenario;
        scenario.enemy.encounter = def(0x31d1);
        scenario.assumptions.clear();
        scenario.usage.clear();
        World { recipe, build, scenario }
    })
}
impl World {
    fn at(class: ClassDefId, level: u16) -> Self {
        let mut w = finite_domain().clone();
        w.build.character.class = class;
        w.build.character.level = level;
        w
    }
    fn plan(
        &self,
    ) -> std::result::Result<
        OwnedSupportEffectPlan<poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage>,
        PlanError,
    > {
        empty_support::compile(&self.recipe, &self.build, &self.scenario, def(2))
    }
    fn report(&self) -> SupportEffectsReport {
        let p = self.plan().unwrap();
        p.evaluate(&mut p.new_scratch()).unwrap()
    }
}
fn check(r: &SupportEffectsReport, amount: f64) {
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    let SupportEffectsOutcome::Evaluated { effects } = &r.outcome else {
        panic!("{r:?}")
    };
    assert!(effects.gaps.is_empty());
    assert_eq!(effects.effects.len(), 1);
    let e = &effects.effects[0];
    assert_eq!(e.key.invocation.program.as_str(), family::PROGRAM);
    assert_eq!(
        e.key.invocation.origin,
        RuleOrigin::ExistingActor {
            application: "shared-player-initialization".parse().unwrap(),
            actor: ActorKey::Player
        }
    );
    assert_eq!(
        e.target,
        BoundEffectTarget::Contribution {
            key: ContributionKey {
                entity: ConcreteEntity::Actor(ActorKey::Player),
                stat: def(0x29f9),
                kind: ContributionKind::Add
            }
        }
    );
    assert_eq!(
        e.value,
        EffectValue::Known {
            value: ParameterValue::Quantity(FiniteQuantity::new(amount, def(3)).unwrap())
        }
    );
    assert!(
        !effects
            .values
            .iter()
            .any(|v| matches!(&v.key,PlanValueKey::Stat{stat,..} if *stat==def(0x29f9))),
        "intrinsic input is not a final Mana pool"
    );
}
#[test]
fn original_source_vectors_and_every_class_execute_one_shared_contribution() {
    let v: Value = family::read("source-vectors.json");
    let d: family::Dependencies = family::read("dependencies.json");
    for row in v["vectors"].as_array().unwrap() {
        let class = d
            .classes
            .iter()
            .find(|c| c["source_id"] == row["class_id"])
            .unwrap();
        let w = World::at(
            serde_json::from_value(class["class"].clone()).unwrap(),
            row["level"].as_u64().unwrap() as u16,
        );
        check(&w.report(), row["amount"].as_f64().unwrap());
    }
    for class in d.classes {
        for (level, amount) in [(1, 34.), (92, 398.), (100, 430.)] {
            check(
                &World::at(
                    serde_json::from_value(class["class"].clone()).unwrap(),
                    level,
                )
                .report(),
                amount,
            );
        }
    }
    for level in [0, 101] {
        assert!(World::at(def(0xa1e), level).plan().is_err());
    }
}
#[test]
fn partial_shared_owner_is_unavailable_and_worker_reuse_is_deterministic() {
    let a = World::at(def(0xa1e), 92);
    let b = World::at(def(0xa20), 91);
    let mut unknown = a.clone();
    let m = family::migration();
    unknown
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == m.owners[0].owner)
        .unwrap()
        .programs
        .closure = m.owners[0].programs.closure.clone();
    let plans = [
        a.plan().unwrap(),
        unknown.plan().unwrap(),
        b.plan().unwrap(),
    ];
    let expected: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    check(&expected[0], 398.);
    check(&expected[2], 394.);
    assert!(matches!(
        expected[1].outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    assert!(
        expected[1]
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialPrograms)
    );
    let sequence = [0, 1, 2, 0];
    let mut scratch = plans[0].new_scratch();
    for i in sequence {
        assert_eq!(plans[i].evaluate(&mut scratch).unwrap(), expected[i]);
    }
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..16).into_par_iter().for_each(|_| {
                let mut s = plans[0].new_scratch();
                for i in sequence {
                    assert_eq!(plans[i].evaluate(&mut s).unwrap(), expected[i]);
                }
            });
        });
}
