//! Actual published physical Offering and Ice paths with finite component
//! boundaries. Admission inputs and final Offering inputs are test-owned;
//! published support bodies and grant/projection bodies remain unchanged.
//! These tests prove contribution factors, not final duration/cost or buff uptime.
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
    owned_release::StagedOwnedRelease, owned_release_migration::OwnedReleaseMigrationInput,
};
use rayon::prelude::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

const I: &str = "ProlongedDurationSupportPlayer";
const II: &str = "ProlongedDurationSupportPlayerTwo";
const SOURCES: [usize; 3] = [2, 3, 4];
const OFFERINGS: [usize; 2] = [3, 4];
const FINAL_BOUNDARY: &str = "fixture.physical-final-inputs";

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
    physical_owner: DefinitionRules,
    receiving: bidding_fixture::Receiving,
}
impl World {
    fn load() -> Self {
        static WORLD: OnceLock<World> = OnceLock::new();
        WORLD.get_or_init(Self::load_once).clone()
    }
    fn load_once() -> Self {
        let path = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_PROLONGED_RELEASE")
                .expect("checked Prolonged Duration publication"),
        );
        let before = crate::release::inventory(&path);
        let endpoint = crate::release::load(&path);
        crate::family::assert_endpoint(&endpoint);
        let bindings: Value = crate::family::read("bindings.json");
        let migration: OwnedReleaseMigrationInput = crate::family::read("migration.json");
        let preparation: SupportPreparationInput = crate::family::read("preparation.json");
        let receiving: bidding_fixture::Receiving = crate::family::read("receiving.json");
        assert_eq!(bindings["supports"].as_array().unwrap().len(), 2);
        assert_eq!(migration.owners.len(), 2);
        let mut base = fixture::World::load_release(&path, false);
        base.set_area_fact(None);
        for field in [
            "physical_gem",
            "primary_skill",
            "primary_supply",
            "entering_grant",
            "output",
            "part",
            "mode",
            "stat_sets",
        ] {
            assert_eq!(
                bindings["contrast"][field], base.ice[field],
                "exact Ice contrast {field}"
            );
        }
        base.add_support_fragment(
            &endpoint,
            &bindings,
            &migration.owners,
            &preparation,
            &receiving,
        );
        let prior_supports: Vec<GemDefId> = base.bindings["supports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| decode(&row["gem"]))
            .collect();
        base.inner
            .owners
            .retain(|o| !prior_supports.iter().any(|g| o.owner == subject(g.clone())));
        base.inner.preparation = preparation;
        base.inner.bindings = bindings.clone();
        base.inner.channels = bindings["channels"].clone();
        base.inner.receiving = bidding_fixture::Receiving {
            roles: receiving.roles.clone(),
            targets: finite(&receiving.targets),
            supports: finite(&receiving.supports),
        };
        // The existing fixture intentionally supplies finite effective inputs for
        // preparation. Actual published 30ae/30af outputs are asserted separately;
        // they are not misrepresented as a complete preparation assembly here.
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
            set_fact(facts, "type.type.duration", true);
            set_fact(facts, "type.type.spell", true);
        }
        let physical_owner = install_physical_source(&mut base, &endpoint, &bindings["target"]);
        base.inner.build.supports.clear();
        base.inner.build.support_origins = Some(vec![]);
        base.inner.build.gems.retain(|g| g.id == id(900));
        base.inner.build.skills.retain(|s| s.id == id(22));
        for source in OFFERINGS {
            add_physical_occurrence(&mut base, &bindings["target"], source);
        }
        for source in SOURCES {
            base.inner.add_support(source, 0);
        }
        assert_eq!(before, crate::release::inventory(&path));
        Self {
            base,
            bindings,
            originals: migration.owners,
            physical_owner,
            receiving,
        }
    }
    fn target(&self, source: usize) -> &Value {
        if source == self.base.ice_source {
            &self.bindings["contrast"]
        } else {
            assert!(OFFERINGS.contains(&source));
            &self.bindings["target"]
        }
    }
    fn actions(&self, source: usize) -> Vec<ActionSelection> {
        let target = self.target(source);
        target["stat_sets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|set| ActionSelection {
                action: ActionKey {
                    actor: ActorKey::Player,
                    provider: ProviderKey {
                        root: ProviderRoot::SkillUse(id(20 + source as u64)),
                        grant_path: vec![decode(&target["entering_grant"])],
                    },
                    output: decode(&target["output"]),
                },
                part: decode(&target["part"]),
                mode: decode(&target["mode"]),
                stat_set: decode(&set["stat_set"]),
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
                            id: QueryId::new(format!("prolonged-{i}")).unwrap(),
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
        let target = &self.bindings["target"];
        self.base.checked_plan_with_readiness_inputs(
            self.request(),
            &[
                decode(&target["final_level_parameter"]),
                decode(&target["final_quality_parameter"]),
            ],
            &[(
                self.physical_owner.owner.clone(),
                key(target["primary_supply_program"].as_str().unwrap()),
            )],
        )
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

fn set_fact(program: &mut RuleProgram, name: &str, value: bool) {
    program
        .nodes
        .iter_mut()
        .find(|n| n.id == key(name))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(value),
    };
}

/// Copy exact checked physical topology, then explicitly close only its finite
/// component copy. No Actor is manufactured from SkillType.Minion metadata.
fn install_physical_source(
    base: &mut fixture::World,
    endpoint: &StagedOwnedRelease,
    target: &Value,
) -> DefinitionRules {
    let gem: GemDefId = decode(&target["physical_gem"]);
    let skill: SkillDefId = decode(&target["primary_skill"]);
    let mut wanted = BTreeSet::from([
        gem.address(),
        skill.address(),
        decode::<ActionPartDefId>(&target["part"]).address(),
        decode::<ActionModeDefId>(&target["mode"]).address(),
    ]);
    for set in target["stat_sets"].as_array().unwrap() {
        wanted.insert(decode::<ActionStatSetDefId>(&set["stat_set"]).address());
    }
    assert_eq!(target["stat_sets"].as_array().unwrap().len(), 1);
    let schema = &endpoint.input().recipe.schema;
    for address in wanted {
        let actual = schema
            .definitions
            .iter()
            .find(|d| d.address() == address)
            .unwrap();
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
    let declarations = [
        SlotOwnerDefId::Gem(gem.clone()),
        SlotOwnerDefId::Skill(skill.clone()),
    ];
    for slot in &schema.slots {
        let value = serde_json::to_value(slot).unwrap();
        let declaration: SlotOwnerDefId = decode(&value["value"]["id"]["declaration"]);
        if declarations.contains(&declaration) {
            assert!(
                !matches!(slot, SlotDescriptor::Actor(_)),
                "source has no attached minion"
            );
            assert!(!base.inner.schema.slots.iter().any(|s| s == slot));
            base.inner.schema.slots.push(finite(slot));
        }
    }
    let physical_owner = endpoint
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == subject(gem.clone()))
        .unwrap()
        .clone();
    assert!(!physical_owner.programs.is_complete());
    assert_eq!(
        physical_owner
            .programs
            .members
            .iter()
            .filter(|p| p.id.as_str() == target["primary_supply_program"].as_str().unwrap())
            .count(),
        1
    );
    for owner in [subject(gem.clone()), subject(skill.clone())] {
        let actual = endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == owner)
            .unwrap();
        assert!(!base.inner.owners.iter().any(|o| o.owner == owner));
        base.inner.owners.push(finite(actual));
    }
    for slot in [
        SchemaSubject::Slot(SlotAddress::Grant(decode(&target["entering_grant"]))),
        SchemaSubject::Slot(SlotAddress::SkillGrant(decode(&target["primary_supply"]))),
        SchemaSubject::Slot(SlotAddress::ActionOutput(decode(&target["output"]))),
    ] {
        base.inner.owner_mut(slot);
    }
    let mut facts = base
        .inner
        .owner_mut(subject(decode::<GemDefId>(&base.ice["physical_gem"])))
        .programs
        .members
        .iter()
        .find(|p| p.id == key("fixture.initial-facts"))
        .unwrap()
        .clone();
    // Finite relevant source vocabulary. The optional minion context stays absent:
    // source types Minion/CreatesMinion do not create an owned recipient actor.
    set_fact(&mut facts, "type.type.spell", false);
    for name in [
        "type.type.duration",
        "type.type.minion",
        "type.type.buff",
        "type.type.creates-minion",
        "type.type.usable-while-moving",
        "type.type.triggerable",
    ] {
        set_fact(&mut facts, name, true);
    }
    for owner in [subject(gem.clone()), subject(skill)] {
        base.inner
            .owner_mut(owner)
            .programs
            .members
            .push(facts.clone());
    }
    let quality = base.inner.quality_unit.clone();
    base.inner
        .owner_mut(subject(gem))
        .programs
        .members
        .push(RuleProgram {
            id: key(FINAL_BOUNDARY),
            context: RuleEntityKind::Skill,
            reads: vec![],
            nodes: vec![
                RuleNode {
                    id: key("level"),
                    expression: RuleExpression::Literal {
                        value: ParameterValue::Integer(BoundedInteger::new(22).unwrap()),
                    },
                },
                RuleNode {
                    id: key("quality"),
                    expression: RuleExpression::Literal {
                        value: quantity(0.0, &quality),
                    },
                },
            ],
            effects: [
                ("level", "physical_final_level"),
                ("quality", "physical_final_quality"),
            ]
            .into_iter()
            .map(|(name, field)| RuleEffect {
                id: key(name),
                when: None,
                effect: RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat: decode(&target[field]),
                    value: key(name),
                },
            })
            .collect(),
        });
    physical_owner
}

fn add_physical_occurrence(base: &mut fixture::World, target: &Value, source: usize) {
    let gem: GemDefId = decode(&target["physical_gem"]);
    let parameters = base
        .inner
        .schema
        .slots
        .iter()
        .filter_map(|slot| match slot {
            SlotDescriptor::Parameter(DefinitionEntry {
                id,
                schema: SchemaState::Known(p),
            }) if id.declaration == SlotOwnerDefId::Gem(gem.clone()) => Some(ParameterAssignment {
                slot: id.clone(),
                value: match &p.value {
                    ValueSchema::Boolean => ParameterValue::Boolean(false),
                    ValueSchema::Quantity(q) => quantity(0.0, q.minimum.unit()),
                    _ => panic!("actual physical corruption input"),
                },
            }),
            _ => None,
        })
        .collect();
    let instance = id(900 + (source - 2) as u64);
    base.inner.build.gems.push(GemInstance {
        id: instance,
        definition: gem,
        parameters,
        level: 20,
        quality: Some(QualitySelection {
            kind: def("def.0000000000000006"),
            amount: FiniteQuantity::new(0.0, base.inner.quality_unit.clone()).unwrap(),
        }),
    });
    base.inner.build.skills.push(SkillUse {
        id: id(20 + source as u64),
        source: AuthoredSkillSource::Gem(instance),
        parameters: None,
        enabled: true,
        scope: LoadoutScope::Shared,
    });
}

fn evaluate(w: &World) -> SupportEffectsReport {
    let plan = w.plan();
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}
fn effects(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("finite component: {report:?}")
    };
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert!(effects.gaps.is_empty(), "{:?}", effects.gaps);
    effects
}
fn expected(w: &World, tier: &'static str) -> [Option<(&'static str, SupportAssignmentId)>; 3] {
    SOURCES.map(|s| Some((tier, w.selected(s))))
}
fn check(
    w: &World,
    report: &SupportEffectsReport,
    expected: [Option<(&str, SupportAssignmentId)>; 3],
) {
    check_admission(w, report, expected, [true; 3]);
}
fn check_admission(
    w: &World,
    report: &SupportEffectsReport,
    expected: [Option<(&str, SupportAssignmentId)>; 3],
    admitted: [bool; 3],
) {
    let report = effects(report);
    let unit: UnitDefId = decode(&w.bindings["factor_unit"]);
    let duration: StatDefId = decode(&w.bindings["channels"]["duration_factor"]);
    let cost: StatDefId = decode(&w.bindings["channels"]["cost_factor"]);
    let mut actual_applications = BTreeSet::new();
    for ((source, expected), admitted) in SOURCES.into_iter().zip(expected).zip(admitted) {
        let actions = w.actions(source);
        assert_eq!(actions.len(), if source == 2 { 2 } else { 1 });
        for action in actions {
            assert_eq!(action.action.actor, ActorKey::Player);
            assert_eq!(
                action.action.provider.grant_path,
                vec![decode(&w.target(source)["entering_grant"])]
            );
            let entity = ConcreteEntity::Action(Box::new(action.clone()));
            let rows: Vec<_> = report
                .effects
                .iter()
                .filter(|e| {
                    matches!(&e.target,
                BoundEffectTarget::Contribution { key } if key.entity == entity)
                })
                .collect();
            assert_eq!(rows.len(), if expected.is_some() { 2 } else { 0 });
            let Some((tier, winner)) = expected else {
                continue;
            };
            let support = w.bindings["supports"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["source_effect"] == tier)
                .unwrap();
            for (channel, value) in [
                (
                    &duration,
                    if tier == I {
                        1.3
                    } else {
                        assert_eq!(tier, II);
                        1.35
                    },
                ),
                (&cost, 1.2),
            ] {
                let joined: Vec<_> = rows.iter().filter(|e| matches!(&e.target, BoundEffectTarget::Contribution { key } if &key.stat == channel)).collect();
                assert_eq!(joined.len(), 1);
                let row = joined[0];
                let RuleOrigin::SupportApplication { application } = &row.key.invocation.origin
                else {
                    panic!("exact support provenance")
                };
                assert_eq!(
                    application.prepared.origin,
                    SupportOrigin::Assignment(winner)
                );
                assert_eq!(
                    application.prepared.target,
                    SkillTarget::Authored(id(20 + source as u64))
                );
                assert_eq!(application.prepared.position, 0);
                assert_eq!(
                    application.receiver,
                    SupportReceiverKey::Action(Box::new(action.clone()))
                );
                actual_applications.insert(application.clone());
                assert_eq!(
                    row.key.invocation.program,
                    key(support["programs"]["delivery"].as_str().unwrap())
                );
                assert!(
                    matches!(&row.target, BoundEffectTarget::Contribution { key } if key.kind == ContributionKind::Multiply)
                );
                assert_eq!(
                    row.value,
                    if admitted {
                        EffectValue::Known {
                            value: quantity(value, &unit),
                        }
                    } else {
                        EffectValue::Inactive
                    }
                );
            }
            let admissions: Vec<_> = report
                .effects
                .iter()
                .filter(|e| {
                    matches!(&e.target,
                BoundEffectTarget::Value { key: PlanValueKey::SupportApplicability { application } }
                    if application.prepared.origin == SupportOrigin::Assignment(winner)
                    && application.receiver == SupportReceiverKey::Action(Box::new(action.clone())))
                })
                .collect();
            assert_eq!(admissions.len(), 1);
            assert_eq!(
                admissions[0].key.invocation.program,
                key(support["programs"]["applicability"].as_str().unwrap())
            );
            assert_eq!(
                admissions[0].value,
                EffectValue::Known {
                    value: ParameterValue::Boolean(admitted)
                }
            );
        }
    }
    for row in &report.effects {
        if let BoundEffectTarget::Contribution { key } = &row.target {
            assert!([&duration, &cost].contains(&&key.stat));
            assert!(
                matches!(&key.entity, ConcreteEntity::Action(a) if SOURCES.into_iter().flat_map(|s| w.actions(s)).any(|expected| expected == **a)),
                "no Player, minion or unrelated receiver leak"
            );
        }
        if let RuleOrigin::SupportApplication { application } = &row.key.invocation.origin {
            assert!(actual_applications.contains(application));
        }
    }
    // The actual prefix's outputs are separately checked. The selector still
    // uses the existing explicitly finite preparation input boundary above.
    for assignment in w.base.inner.build.supports.iter().filter(|s| s.enabled) {
        let gem = w
            .base
            .inner
            .build
            .gems
            .iter()
            .find(|g| g.id == assignment.support)
            .unwrap();
        let binding = w.bindings["supports"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| decode::<GemDefId>(&s["gem"]) == gem.definition)
            .unwrap();
        let program = key(binding["programs"]["prepared_inputs"].as_str().unwrap());
        for (stat, value) in [
            (
                def::<StatDefinition>("def.00000000000030ae"),
                ParameterValue::Integer(BoundedInteger::new(1).unwrap()),
            ),
            (
                def::<StatDefinition>("def.00000000000030af"),
                ParameterValue::Quantity(gem.quality.as_ref().unwrap().amount.clone()),
            ),
        ] {
            let rows: Vec<_> = report.effects.iter().filter(|e| e.key.invocation.program == program && matches!(&e.target,
                BoundEffectTarget::Value { key: PlanValueKey::Stat { entity: ConcreteEntity::SupportOrigin(origin), stat: actual } }
                    if *origin == SupportOrigin::Assignment(assignment.id) && *actual == stat)).collect();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].value, EffectValue::Known { value });
        }
    }
    for source in OFFERINGS {
        let root = ProviderKey {
            root: ProviderRoot::SkillUse(id(20 + source as u64)),
            grant_path: vec![],
        };
        let target = &w.bindings["target"];
        let generated = GeneratedSkillKey {
            provider: root,
            slot: decode(&target["primary_supply"]),
        };
        let quality_unit = &w.base.inner.quality_unit;
        for (field, expected) in [
            (
                "final_level_parameter",
                ParameterValue::Integer(BoundedInteger::new(22).unwrap()),
            ),
            ("final_quality_parameter", quantity(0.0, quality_unit)),
        ] {
            let value = PlanValueKey::SkillParameter {
                skill: Box::new(generated.clone()),
                parameter: decode(&target[field]),
            };
            let rows: Vec<_> = report
                .effects
                .iter()
                .filter(|e| matches!(&e.target, BoundEffectTarget::Value { key } if *key == value))
                .collect();
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0].key.invocation.program.as_str(),
                target["primary_supply_program"].as_str().unwrap()
            );
            assert_eq!(rows[0].value, EffectValue::Known { value: expected });
        }
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_PROLONGED_RELEASE; finite component only"]
fn prolonged_real_player_offering_and_ice_contrast_deliver_both_tiers() {
    let mut w = World::load();
    assert!(w.originals.iter().all(|o| !o.programs.is_complete()));
    assert_ne!(
        w.base.inner.build.skills[1].source,
        w.base.inner.build.skills[2].source
    );
    let offering: GemDefId = decode(&w.bindings["target"]["physical_gem"]);
    let raw: Vec<_> = w
        .base
        .inner
        .build
        .gems
        .iter()
        .filter(|g| g.definition == offering)
        .collect();
    assert_eq!(raw.len(), 2);
    assert!(
        raw.iter().all(|g| g.level == 20),
        "raw physical levels differ from the explicit final22 boundary"
    );
    check(&w, &evaluate(&w), expected(&w, I));
    for source in SOURCES {
        w.base.inner.change_support(source, II);
    }
    check(&w, &evaluate(&w), expected(&w, II));
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_PROLONGED_RELEASE; finite component only"]
fn prolonged_removal_and_disable_do_not_affect_other_roots() {
    for source in SOURCES {
        for remove in [false, true] {
            let mut w = World::load();
            let mut wanted = expected(&w, I);
            let origin = wanted[source - 2].take().unwrap().1;
            if remove {
                let gem = w
                    .base
                    .inner
                    .build
                    .supports
                    .iter()
                    .find(|s| s.id == origin)
                    .unwrap()
                    .support;
                w.base.inner.build.supports.retain(|s| s.id != origin);
                w.base.inner.build.gems.retain(|g| g.id != gem);
                for row in w.base.inner.build.support_origins.as_mut().unwrap() {
                    row.origins
                        .retain(|o| *o != SupportOrigin::Assignment(origin));
                }
            } else {
                w.base
                    .inner
                    .build
                    .supports
                    .iter_mut()
                    .find(|s| s.id == origin)
                    .unwrap()
                    .enabled = false;
            }
            check(&w, &evaluate(&w), wanted);
        }
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_PROLONGED_RELEASE; finite component only"]
fn prolonged_duplicate_quality_and_family_order_keep_one_exact_origin() {
    let mut w = World::load();
    let original = w.selected(3);
    let index = w.base.inner.support_index(I);
    w.base.inner.add_support(3, index);
    let duplicate = w.base.inner.build.supports.last().unwrap().clone();
    for (quality, winner) in [(0.0, original), (15.0, duplicate.id)] {
        w.base
            .inner
            .build
            .gems
            .iter_mut()
            .find(|g| g.id == duplicate.support)
            .unwrap()
            .quality
            .as_mut()
            .unwrap()
            .amount = FiniteQuantity::new(quality, w.base.inner.quality_unit.clone()).unwrap();
        check(
            &w,
            &evaluate(&w),
            [
                Some((I, w.selected(2))),
                Some((I, winner)),
                Some((I, w.selected(4))),
            ],
        );
    }
    let index = w.base.inner.support_index(II);
    w.base.inner.add_support(3, index);
    let last = w.base.inner.build.supports.last().unwrap().id;
    check(
        &w,
        &evaluate(&w),
        [
            Some((I, w.selected(2))),
            Some((II, last)),
            Some((I, w.selected(4))),
        ],
    );
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_PROLONGED_RELEASE; finite component only"]
fn prolonged_actual_partial_owners_and_receiving_remain_blocked() {
    let baseline = World::load();
    for owner in baseline
        .originals
        .iter()
        .chain(std::iter::once(&baseline.physical_owner))
    {
        let mut w = baseline.clone();
        if let Some(support) = w.bindings["supports"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| subject(decode::<GemDefId>(&s["gem"])) == owner.owner)
        {
            let tier = support["source_effect"].as_str().unwrap().to_owned();
            w.base.inner.change_support(3, &tier);
        }
        w.base.inner.owner_mut(owner.owner.clone()).programs.closure =
            owner.programs.closure.clone();
        assert!(
            w.checked_plan().is_err(),
            "actual Partial owner cannot become complete"
        );
    }
    let mut w = baseline;
    w.base.inner.receiving = w.receiving.clone();
    if let Ok(p) = w.checked_plan() {
        assert!(!matches!(
            p.evaluate(&mut p.new_scratch()).unwrap().outcome,
            SupportEffectsOutcome::Evaluated { .. }
        ));
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_PROLONGED_RELEASE; finite component only"]
fn prolonged_duration_admission_checks_assigned_and_generated_skill_separately() {
    for assigned in [true, false] {
        let mut w = World::load();
        for target in &w.receiving.targets {
            for role in &target.roles.members {
                for endpoint in &role.endpoints.members {
                    assert!(matches!(endpoint.admission(),
                        poe_optimizer_core::owned_support_receiving::SupportAdmissionContext::ReceivingSkill {
                            summoner_path: None
                        }));
                }
            }
        }
        let owner = if assigned {
            subject(decode::<GemDefId>(&w.bindings["target"]["physical_gem"]))
        } else {
            subject(decode::<SkillDefId>(&w.bindings["target"]["primary_skill"]))
        };
        let facts = w
            .base
            .inner
            .owner_mut(owner)
            .programs
            .members
            .iter_mut()
            .find(|p| p.id == key("fixture.initial-facts"))
            .unwrap();
        set_fact(facts, "type.type.duration", false);
        let report = evaluate(&w);
        for source in OFFERINGS {
            let target = if assigned {
                SkillTarget::Authored(id(20 + source as u64))
            } else {
                SkillTarget::Generated(Box::new(GeneratedSkillKey {
                    provider: ProviderKey {
                        root: ProviderRoot::SkillUse(id(20 + source as u64)),
                        grant_path: vec![],
                    },
                    slot: decode(&w.bindings["target"]["primary_supply"]),
                }))
            };
            let fact = PlanValueKey::Stat {
                entity: ConcreteEntity::Skill(Box::new(target)),
                stat: def("fixture.type.type.duration"),
            };
            let rows: Vec<_> = effects(&report)
                .effects
                .iter()
                .filter(|e| {
                    e.key.invocation.program == key("fixture.initial-facts")
                        && matches!(&e.target, BoundEffectTarget::Value { key } if *key == fact)
                })
                .collect();
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0].value,
                EffectValue::Known {
                    value: ParameterValue::Boolean(false)
                }
            );
        }
        // ReceivingSkill with no summoner uses the generated effect's own
        // preparation. Its physical input owner still owns the selected support
        // position, but that owner's type fact is not a second admission gate.
        // A rejected receiver keeps its exact applications as false/Inactive
        // diagnostics; removal of an assignment is the separate zero-row case.
        check_admission(&w, &report, expected(&w, I), [true, assigned, assigned]);
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_PROLONGED_RELEASE; finite component only"]
fn prolonged_missing_final_input_boundary_cannot_default_to_ready() {
    let mut w = World::load();
    w.base
        .inner
        .owner_mut(w.physical_owner.owner.clone())
        .programs
        .members
        .retain(|p| p.id != key(FINAL_BOUNDARY));
    let report = evaluate(&w);
    match &report.outcome {
        SupportEffectsOutcome::Unavailable { cause, .. } => assert!(matches!(
            cause,
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                ..
            }
        )),
        SupportEffectsOutcome::Evaluated { effects } => {
            let missing: Vec<_> = effects
                .effects
                .iter()
                .filter(|e| {
                    e.key.invocation.program.as_str()
                        == w.bindings["target"]["primary_supply_program"]
                            .as_str()
                            .unwrap()
                        && matches!(
                            &e.target,
                            BoundEffectTarget::Value {
                                key: PlanValueKey::SkillParameter { .. }
                            }
                        )
                })
                .collect();
            assert_eq!(missing.len(), 4);
            assert!(missing.iter().all(|e| matches!(
                e.value,
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    ..
                }
            )));
            assert!(!effects.effects.iter().any(|e| matches!(&e.target, BoundEffectTarget::Contribution { key }
                if matches!(&key.entity, ConcreteEntity::Action(a) if OFFERINGS.into_iter().flat_map(|s| w.actions(s)).any(|expected| expected == **a)))
                && matches!(e.value, EffectValue::Known { .. })));
        }
        other => panic!("missing final input must retain its cause: {other:?}"),
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_PROLONGED_RELEASE; finite component only"]
fn prolonged_scratch_a_b_a_and_private_rayon_preserve_exact_receivers() {
    let a = World::load();
    let mut b = a.clone();
    b.base.inner.change_support(3, II);
    let pa = Arc::new(a.plan());
    let pb = b.plan();
    let mut scratch = pa.new_scratch();
    let first = pa.evaluate(&mut scratch).unwrap();
    check(&a, &first, expected(&a, I));
    check(
        &b,
        &pb.evaluate(&mut scratch).unwrap(),
        [
            Some((I, b.selected(2))),
            Some((II, b.selected(3))),
            Some((I, b.selected(4))),
        ],
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
