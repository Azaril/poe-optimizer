//! Published Rapid Casting producers over two finite physical Ice occurrences.
//! Activation, admission inputs and closure of this isolated graph are fixture
//! boundaries; no complete original build or final cast-time formula is claimed.
#[allow(dead_code)]
#[path = "owned_bidding_delivery_fixture.rs"]
mod bidding_fixture;
#[allow(dead_code)]
#[path = "owned_magnified_area_fixture.rs"]
mod fixture;

use fixture::{decode, def, id, key, quantity, subject};
use poe_optimizer_core::{
    build_identity::SupportAssignmentId, owned_build::*, owned_definitions::*, owned_rules::*,
    owned_schema::*, owned_supports::*,
};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release_migration::OwnedReleaseMigrationInput,
};
use rayon::prelude::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

const I: &str = "SupportRapidCastingPlayer";
const II: &str = "SupportRapidCastingPlayerTwo";
const SOURCES: [usize; 2] = [2, 3];

// Close only the finite component copies. Every original published owner and
// receiving inventory remains separately retained and tested below.
fn finite<T: Serialize + DeserializeOwned>(value: &T) -> T {
    fn close(value: &mut Value) {
        match value {
            Value::Object(fields) => {
                if fields.contains_key("members") && fields.contains_key("closure") {
                    fields.insert("closure".into(), json!({"kind":"complete"}));
                }
                for child in fields.values_mut() {
                    close(child);
                }
            }
            Value::Array(rows) => {
                for child in rows {
                    close(child);
                }
            }
            _ => {}
        }
    }
    let mut value = serde_json::to_value(value).unwrap();
    close(&mut value);
    decode(&value)
}

#[derive(Clone)]
struct World {
    base: fixture::World,
    bindings: Value,
    originals: Vec<DefinitionRules>,
    receiving: bidding_fixture::Receiving,
}
impl World {
    fn load() -> Self {
        static WORLD: OnceLock<World> = OnceLock::new();
        WORLD.get_or_init(Self::load_once).clone()
    }
    fn load_once() -> Self {
        let path = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_RAPID_RELEASE")
                .expect("checked Rapid Casting publication"),
        );
        let before = crate::release::inventory(&path);
        let endpoint = crate::release::load(&path);
        crate::family::assert_endpoint(&endpoint);
        let migration: OwnedReleaseMigrationInput = crate::family::read("migration.json");
        let bindings: Value = crate::family::read("bindings.json");
        let receiving: bidding_fixture::Receiving = crate::family::read("receiving.json");
        let preparation: SupportPreparationInput = crate::family::read("preparation.json");
        assert_eq!(bindings["supports"].as_array().unwrap().len(), 2);
        assert_eq!(preparation.supports.len(), 2);
        assert_eq!(receiving.supports.len(), 2);
        assert_eq!(receiving.roles.len(), 1);
        assert_eq!(receiving.roles[0].id, key("rapid-casting-action"));
        assert_eq!(migration.schema.len(), 1);
        for entry in &migration.schema {
            let SchemaExtensionEntry::Definition(d) = entry else {
                panic!("one actual Stat")
            };
            assert_eq!(
                endpoint
                    .input()
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .filter(|a| *a == d)
                    .count(),
                1
            );
        }
        assert_eq!(migration.owners.len(), 2);
        for owner in &migration.owners {
            assert_eq!(
                endpoint
                    .input()
                    .recipe
                    .rules
                    .owners
                    .iter()
                    .filter(|r| *r == owner)
                    .count(),
                1
            );
            assert!(!owner.programs.is_complete());
        }
        let mut base = fixture::World::load_release(&path, false);
        base.set_area_fact(None);
        assert!(base.area_usage.is_empty());
        for field in [
            "physical_gem",
            "primary_skill",
            "entering_grant",
            "output",
            "part",
            "mode",
            "stat_sets",
        ] {
            assert_eq!(
                bindings["target"][field], base.ice[field],
                "exact physical Ice {field}"
            );
        }
        assert_eq!(
            bindings["target"]["stat_sets"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["source_index"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        let origin = base
            .inner
            .owners
            .iter()
            .flat_map(|o| &o.programs.members)
            .find(|p| p.id == key("fixture.origin-facts"))
            .unwrap()
            .clone();
        let old_gems: Vec<GemDefId> = base.bindings["supports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| decode(&s["gem"]))
            .collect();
        base.inner
            .owners
            .retain(|o| !old_gems.iter().any(|g| o.owner == subject(g.clone())));
        let gems: Vec<GemDefId> = bindings["supports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| decode(&s["gem"]))
            .collect();
        for row in bindings["supports"].as_array().unwrap() {
            let gem: GemDefId = decode(&row["gem"]);
            let skill: SkillDefId = decode(&row["skill"]);
            let association = endpoint
                .input()
                .roles
                .roles
                .iter()
                .find(|r| r.gem == gem)
                .unwrap();
            let poe_optimizer_import::owned_skill_catalog::OwnedPrimarySkill::Known(primary) =
                &association.primary
            else {
                panic!("actual support primary association")
            };
            assert_eq!(primary, &skill);
            for address in [gem.address(), skill.address()] {
                let actual = endpoint
                    .input()
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .find(|d| d.address() == address)
                    .unwrap();
                if let DefinitionDescriptor::Gem(DefinitionEntry {
                    schema: SchemaState::Known(s),
                    ..
                }) = actual
                {
                    assert!(s.skills.members.is_empty());
                    assert!(!s.skills.is_complete());
                }
                assert!(
                    !base
                        .inner
                        .schema
                        .definitions
                        .iter()
                        .any(|d| d.address() == address)
                );
                base.inner.schema.definitions.push(finite(actual));
            }
        }
        for slot in &endpoint.input().recipe.schema.slots {
            let value = serde_json::to_value(slot).unwrap();
            let owner: SlotOwnerDefId = decode(&value["value"]["id"]["declaration"]);
            if gems.iter().any(|g| owner == SlotOwnerDefId::Gem(g.clone())) {
                assert!(!base.inner.schema.slots.iter().any(|s| s == slot));
                base.inner.schema.slots.push(finite(slot));
            }
        }
        for owner in &migration.owners {
            let mut component: DefinitionRules = finite(owner);
            component.programs.members.push(origin.clone());
            base.inner.owners.push(component);
        }
        // Reuse the already-declared finite type vocabulary, while installing
        // the exact two real support predicates from the authored publication.
        for t in &preparation.types {
            assert!(base.inner.skill_types.contains(t));
        }
        base.inner.preparation.supports = preparation.supports;
        base.inner.preparation.effects = preparation.effects;
        base.inner.preparation.families = preparation.families;
        base.inner.bindings = bindings.clone();
        base.inner.channels = bindings["channels"].clone();
        base.inner.receiving = bidding_fixture::Receiving {
            roles: receiving.roles.clone(),
            targets: finite(&receiving.targets),
            supports: finite(&receiving.supports),
        };
        // The source-backed admission boundary is Spell and none of the three
        // excluded timing types. This is not a general production classifier.
        for owner in [
            subject(decode::<GemDefId>(&base.ice["physical_gem"])),
            subject(decode::<SkillDefId>(&base.ice["primary_skill"])),
        ] {
            let facts = base
                .inner
                .owner_mut(owner)
                .programs
                .members
                .iter_mut()
                .find(|p| p.id == key("fixture.initial-facts"))
                .unwrap();
            for (name, expected) in [
                ("type.type.spell", true),
                ("type.type.instant", false),
                ("type.type.fixed-cast-time", false),
                ("type.type.no-attack-or-cast-time", false),
            ] {
                let node = facts.nodes.iter_mut().find(|n| n.id == key(name)).unwrap();
                node.expression = RuleExpression::Literal {
                    value: ParameterValue::Boolean(expected),
                };
            }
        }
        base.inner.build.supports.clear();
        base.inner.build.support_origins = Some(vec![]);
        base.inner.build.gems.retain(|g| g.id == id(900));
        base.inner.build.skills.retain(|s| s.id == id(22));
        let mut gem = base.inner.build.gems[0].clone();
        gem.id = id(901);
        base.inner.build.gems.push(gem);
        let mut skill = base.inner.build.skills[0].clone();
        skill.id = id(23);
        skill.source = AuthoredSkillSource::Gem(id(901));
        base.inner.build.skills.push(skill);
        for source in SOURCES {
            base.inner.add_support(source, 0);
        }
        assert_eq!(before, crate::release::inventory(&path));
        Self {
            base,
            bindings,
            originals: migration.owners,
            receiving,
        }
    }
    fn actions(&self, source: usize) -> Vec<ActionSelection> {
        self.base
            .actions(self.base.ice_source)
            .into_iter()
            .map(|mut a| {
                a.action.provider.root = ProviderRoot::SkillUse(id(20 + source as u64));
                a
            })
            .collect()
    }
    fn request(&self) -> OwnedEvaluationRequest {
        OwnedEvaluationRequest::new(
            BuildSpec::new(self.base.inner.build.clone(), Default::default()).unwrap(),
            ScenarioSpec::new(
                ScenarioInput {
                    game_version: bidding_fixture::ns(),
                    enemy: EnemySpec {
                        encounter: def("fixture.encounter"),
                        level: 20,
                    },
                    assumptions: vec![],
                    usage: vec![],
                },
                Default::default(),
            )
            .unwrap(),
            QuerySpec::new(
                QueryInput {
                    game_version: bidding_fixture::ns(),
                    requests: SOURCES
                        .into_iter()
                        .flat_map(|s| self.actions(s))
                        .enumerate()
                        .map(|(i, a)| MetricRequest {
                            id: QueryId::new(format!("rapid-{i}")).unwrap(),
                            metric: def("fixture.observe"),
                            target: MetricTarget::Action(Box::new(a)),
                        })
                        .collect(),
                },
                Default::default(),
            )
            .unwrap(),
            Default::default(),
        )
        .unwrap()
    }
    fn checked_plan(&self) -> std::result::Result<bidding_fixture::Plan, String> {
        self.base.checked_plan_with_request(self.request())
    }
    fn plan(&self) -> bidding_fixture::Plan {
        self.checked_plan().unwrap()
    }
    fn selected(&self, source: usize) -> SupportAssignmentId {
        let rows: Vec<_> = self
            .base
            .inner
            .build
            .supports
            .iter()
            .filter(|s| s.target == SkillTarget::Authored(id(20 + source as u64)))
            .collect();
        assert_eq!(rows.len(), 1);
        rows[0].id
    }
}

fn evaluate(w: &World) -> SupportEffectsReport {
    let p = w.plan();
    p.evaluate(&mut p.new_scratch()).unwrap()
}
fn effects(r: &SupportEffectsReport) -> &OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = &r.outcome else {
        panic!("finite component: {r:?}")
    };
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    effects
}
fn application(e: &BoundEffectResult) -> Option<&SupportApplicationKey> {
    if let RuleOrigin::SupportApplication { application } = &e.key.invocation.origin {
        Some(application)
    } else {
        None
    }
}
fn check(
    w: &World,
    report: &SupportEffectsReport,
    expected: [Option<(&str, SupportAssignmentId)>; 2],
) {
    let r = effects(report);
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    let stat: StatDefId = decode(&w.bindings["channels"]["cast_speed"]);
    let percent: UnitDefId = decode(&w.bindings["percent_unit"]);
    let mut applications = BTreeSet::new();
    for (source, expected) in SOURCES.into_iter().zip(expected) {
        let actions = w.actions(source);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].action, actions[1].action);
        assert_ne!(actions[0].stat_set, actions[1].stat_set);
        for action in actions {
            assert_eq!(action.action.actor, ActorKey::Player);
            assert_eq!(
                action.action.provider.grant_path,
                vec![decode(&w.bindings["target"]["entering_grant"])]
            );
            let rows: Vec<_> = r.effects.iter().filter(|e| matches!(&e.target,
                BoundEffectTarget::Contribution { key } if key.entity == ConcreteEntity::Action(Box::new(action.clone())) && key.stat == stat)).collect();
            assert_eq!(rows.len(), usize::from(expected.is_some()));
            let Some((tier, winner)) = expected else {
                continue;
            };
            let row = rows[0];
            let app = application(row).unwrap();
            assert_eq!(app.prepared.origin, SupportOrigin::Assignment(winner));
            assert_eq!(
                app.prepared.target,
                SkillTarget::Authored(id(20 + source as u64))
            );
            assert_eq!(app.prepared.position, 0);
            assert_eq!(app.receiver, SupportReceiverKey::Action(Box::new(action)));
            assert!(applications.insert(app.clone()));
            assert_eq!(row.key.invocation.program, key("rapid-casting-cast-speed"));
            let BoundEffectTarget::Contribution { key: contribution } = &row.target else {
                unreachable!()
            };
            assert_eq!(contribution.kind, ContributionKind::Increase);
            assert_eq!(
                row.value,
                EffectValue::Known {
                    value: quantity(
                        if tier == I {
                            15.0
                        } else {
                            assert_eq!(tier, II);
                            20.0
                        },
                        &percent
                    )
                }
            );
            let admitted: Vec<_> = r.effects.iter().filter(|e| matches!(&e.target,
                BoundEffectTarget::Value { key: PlanValueKey::SupportApplicability { application } } if application.as_ref() == app)).collect();
            assert_eq!(admitted.len(), 1);
            assert_eq!(
                admitted[0].key.invocation.program,
                key("rapid-casting-applicability")
            );
            assert_eq!(
                admitted[0].value,
                EffectValue::Known {
                    value: ParameterValue::Boolean(true)
                }
            );
        }
    }
    for row in &r.effects {
        if let BoundEffectTarget::Contribution { key: contribution } = &row.target {
            assert_eq!(
                contribution.stat, stat,
                "no cost/reservation delivery from another origin"
            );
        }
        if let Some(app) = application(row) {
            assert!(applications.contains(app));
            match &row.target {
                BoundEffectTarget::Contribution { key: contribution } => assert_eq!(
                    contribution.stat, stat,
                    "Rapid has no cost, reservation or other numerical delivery lane"
                ),
                BoundEffectTarget::Value {
                    key: PlanValueKey::SupportApplicability { .. },
                } => {}
                _ => panic!("unreviewed Rapid application effect"),
            }
        }
    }
}

#[test]
#[ignore = "requires checked Rapid Casting publication; finite component only"]
fn rapid_actual_tiers_reach_both_stat_sets_of_independent_physical_roots() {
    let mut w = World::load();
    assert!(w.originals.iter().all(|o| !o.programs.is_complete()));
    assert_ne!(
        w.base.inner.build.skills[0].source,
        w.base.inner.build.skills[1].source
    );
    for source in SOURCES {
        assert_eq!(w.actions(source).len(), 2);
    }
    check(
        &w,
        &evaluate(&w),
        [Some((I, w.selected(2))), Some((I, w.selected(3)))],
    );
    w.base.inner.change_support(3, II);
    check(
        &w,
        &evaluate(&w),
        [Some((I, w.selected(2))), Some((II, w.selected(3)))],
    );
    w.base.inner.change_support(2, II);
    check(
        &w,
        &evaluate(&w),
        [Some((II, w.selected(2))), Some((II, w.selected(3)))],
    );
}

#[test]
#[ignore = "requires checked Rapid Casting publication; finite component only"]
fn rapid_disable_and_removal_do_not_affect_the_other_physical_root() {
    for (remove, source) in [false, true]
        .into_iter()
        .flat_map(|remove| SOURCES.map(|s| (remove, s)))
    {
        let mut w = World::load();
        w.base.inner.change_support(3, II);
        let mut expected = [Some((I, w.selected(2))), Some((II, w.selected(3)))];
        let first = w.selected(source);
        expected[source - SOURCES[0]] = None;
        if remove {
            let gem = w
                .base
                .inner
                .build
                .supports
                .iter()
                .find(|s| s.id == first)
                .unwrap()
                .support;
            w.base.inner.build.supports.retain(|s| s.id != first);
            w.base.inner.build.gems.retain(|g| g.id != gem);
            w.base
                .inner
                .build
                .support_origins
                .as_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r.target == SkillTarget::Authored(id(20 + source as u64)))
                .unwrap()
                .origins
                .clear();
        } else {
            w.base
                .inner
                .build
                .supports
                .iter_mut()
                .find(|s| s.id == first)
                .unwrap()
                .enabled = false;
        }
        check(&w, &evaluate(&w), expected);
    }
}

#[test]
#[ignore = "requires checked Rapid Casting publication; finite component only"]
fn rapid_duplicate_quality_and_family_order_retain_exact_assignment() {
    let mut w = World::load();
    let first = w.selected(2);
    let other = w.selected(3);
    w.base.inner.add_support(2, w.base.inner.support_index(I));
    let duplicate = w.base.inner.build.supports.last().unwrap().id;
    let gem = w.base.inner.build.supports.last().unwrap().support;
    for (quality, winner) in [(0.0, first), (15.0, duplicate)] {
        w.base
            .inner
            .build
            .gems
            .iter_mut()
            .find(|g| g.id == gem)
            .unwrap()
            .quality
            .as_mut()
            .unwrap()
            .amount = FiniteQuantity::new(quality, w.base.inner.quality_unit.clone()).unwrap();
        check(&w, &evaluate(&w), [Some((I, winner)), Some((I, other))]);
    }
    for (first, last) in [(I, II), (II, I)] {
        let mut w = World::load();
        w.base.inner.change_support(2, first);
        w.base
            .inner
            .add_support(2, w.base.inner.support_index(last));
        let winner = w.base.inner.build.supports.last().unwrap().id;
        check(
            &w,
            &evaluate(&w),
            [Some((last, winner)), Some((I, w.selected(3)))],
        );
    }
}

#[test]
#[ignore = "requires checked Rapid Casting publication; finite component only"]
fn rapid_actual_partial_owner_and_incomplete_receiving_are_not_promoted() {
    let mut w = World::load();
    let gem = w.base.inner.support_gem(I);
    let closure = w
        .originals
        .iter()
        .find(|o| o.owner == subject(gem.clone()))
        .unwrap()
        .programs
        .closure
        .clone();
    w.base.inner.owner_mut(subject(gem)).programs.closure = closure;
    assert!(w.checked_plan().is_err());
    let mut w = World::load();
    w.base.inner.receiving = w.receiving.clone();
    if let Ok(p) = w.checked_plan() {
        assert!(!matches!(
            p.evaluate(&mut p.new_scratch()).unwrap().outcome,
            SupportEffectsOutcome::Evaluated { .. }
        ));
    }
}

#[test]
#[ignore = "requires checked Rapid Casting publication; finite component only"]
fn rapid_scratch_a_b_a_and_rayon_preserve_exact_provenance() {
    let a = World::load();
    let mut b = a.clone();
    b.base.inner.change_support(3, II);
    let pa = Arc::new(a.plan());
    let pb = b.plan();
    let mut scratch = pa.new_scratch();
    let first = pa.evaluate(&mut scratch).unwrap();
    check(
        &a,
        &first,
        [Some((I, a.selected(2))), Some((I, a.selected(3)))],
    );
    check(
        &b,
        &pb.evaluate(&mut scratch).unwrap(),
        [Some((I, b.selected(2))), Some((II, b.selected(3)))],
    );
    assert_eq!(first, pa.evaluate(&mut scratch).unwrap());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map_init(|| pa.new_scratch(), |s, _| pa.evaluate(s).unwrap())
            .collect::<Vec<_>>()
    });
    assert!(reports.iter().all(|r| r == &first));
}
