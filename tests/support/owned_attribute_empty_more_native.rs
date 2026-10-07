//! Actual class/choice occurrences in the existing finite test domain. Production
//! contributor inventories remain Partial and are tested separately below.
use super::{empty_more as family, empty_support, release, step};
use family::key;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::owned_recipe::OwnedRecipeInput;
use rayon::prelude::*;
use serde_json::Value;
use std::{path::PathBuf, sync::OnceLock};

fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(ns(), format!("def.{n:016x}")).unwrap()
}
fn subject(id: StatDefId) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
fn source() -> &'static OwnedRecipeInput {
    static SOURCE: OnceLock<OwnedRecipeInput> = OnceLock::new();
    SOURCE.get_or_init(|| {
        let path = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ATTRIBUTE_EMPTY_MORE_RELEASE")
                .expect("verified empty-MORE release"),
        );
        let before = release::inventory(&path);
        let endpoint = release::load(&path);
        family::assert_endpoint(&endpoint);
        assert_eq!(before, release::inventory(&path));
        endpoint.input().recipe.clone()
    })
}
#[derive(Clone)]
struct World {
    recipe: OwnedRecipeInput,
    build: BuildInput,
}
impl World {
    fn new() -> Self {
        let (mut recipe, build) = step::finite_parts();
        let actual = source();
        let packet: family::Producers = family::read("producers.json");
        for row in &packet.owners {
            let published = actual
                .rules
                .owners
                .iter()
                .find(|o| o.owner == row.after.owner)
                .unwrap();
            assert_eq!(published, &row.after);
            let owner = recipe
                .rules
                .owners
                .iter_mut()
                .find(|o| o.owner == row.after.owner)
                .unwrap();
            assert_eq!(owner.programs.members.len(), 1);
            assert!(
                owner.programs.members[0]
                    .id
                    .as_str()
                    .starts_with("fixture-empty-more-")
            );
            *owner = published.clone();
        }
        let before = recipe.rules.receivers.members.len();
        recipe
            .rules
            .receivers
            .members
            .retain(|r| !r.id.as_str().starts_with("fixture-empty-more-"));
        assert_eq!(before - recipe.rules.receivers.members.len(), 6);
        for receiver in packet.receivers {
            assert!(actual.rules.receivers.members.contains(&receiver));
            recipe.rules.receivers.members.push(receiver);
        }
        let registry = recipe.rules.contribution_queries.as_mut().unwrap();
        assert!(registry.is_complete());
        assert_eq!(registry.members.len(), 12);
        let queries: Vec<ContributionQuery> = family::read("queries.json");
        for query in queries {
            assert!(
                actual
                    .rules
                    .contribution_queries
                    .as_ref()
                    .unwrap()
                    .members
                    .contains(&query)
            );
            registry.members.push(query);
        }
        Self { recipe, build }
    }
    fn plan(
        &self,
    ) -> std::result::Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>, PlanError> {
        let scenario = ScenarioInput {
            game_version: ns(),
            enemy: EnemySpec {
                encounter: def(0x31d1),
                level: 20,
            },
            assumptions: vec![],
            usage: vec![],
        };
        empty_support::compile(&self.recipe, &self.build, &scenario, def(2))
    }
    fn evaluate(&self) -> OwnedEffectsReport {
        let plan = self.plan().unwrap();
        effects(plan.evaluate(&mut plan.new_scratch()).unwrap())
    }
    fn edit(&mut self) {
        let allocation = self
            .build
            .allocations
            .iter_mut()
            .find(|a| a.node == crate::count_native::choice_control_node())
            .unwrap();
        assert_eq!(
            allocation.choices[0].value,
            ParameterValue::Option(def(0x1bf4))
        );
        allocation.choices[0].value = ParameterValue::Option(def(0x1bf2));
    }
    fn remove_factor(&mut self, i: u64) {
        let stat = def(0x3324 + i);
        self.recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == subject(stat.clone()))
            .unwrap()
            .programs
            .members
            .clear();
        self.recipe
            .rules
            .receivers
            .members
            .retain(|r| r.stat != stat);
    }
    fn potential(&mut self, stage: u64, value: Option<f64>, enabled: bool) {
        // Synthetic binding control, never an obtainable source claim. Missing
        // input is an absent different-stage factor, not a copied scalar default.
        let mut reads = vec![];
        let expression = if let Some(value) = value {
            RuleExpression::Literal {
                value: ParameterValue::Quantity(FiniteQuantity::new(value, def(1)).unwrap()),
            }
        } else {
            assert_ne!(stage, 5);
            self.remove_factor(5);
            reads.push(RuleRead {
                id: key("missing"),
                value_type: ComputedValueType::Quantity { unit: def(1) },
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: def(0x3329),
                },
            });
            RuleExpression::Read {
                input: key("missing"),
            }
        };
        let owner = SchemaSubject::Definition(self.build.character.class.address());
        self.recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == owner)
            .unwrap()
            .programs
            .members
            .push(RuleProgram {
                id: key("potential-more-control"),
                context: RuleEntityKind::Actor,
                reads,
                nodes: vec![
                    RuleNode {
                        id: key("amount"),
                        expression,
                    },
                    RuleNode {
                        id: key("enabled"),
                        expression: RuleExpression::Literal {
                            value: ParameterValue::Boolean(enabled),
                        },
                    },
                ],
                effects: vec![RuleEffect {
                    id: key("potential-more"),
                    when: Some(key("enabled")),
                    effect: RuleEffectKind::Contribute {
                        entity: RuleEntity::Current,
                        stat: def(0x331b + stage),
                        contribution: ContributionKind::Multiply,
                        value: key("amount"),
                    },
                }],
            });
    }
}
fn effects(report: SupportEffectsReport) -> OwnedEffectsReport {
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    let SupportEffectsOutcome::Evaluated { effects } = report.outcome else {
        panic!("{report:?}")
    };
    effects
}
fn value(report: &OwnedEffectsReport, stat: StatDefId) -> &EffectValue {
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
fn assert_outputs(report: &OwnedEffectsReport, expected: [i64; 3]) {
    for i in 0..6 {
        let result = if i < 3 { 0x3321 + i } else { 0x1d2e + i - 3 };
        assert_eq!(
            value(report, def(result)),
            &EffectValue::Known {
                value: ParameterValue::Integer(
                    BoundedInteger::new(expected[i as usize % 3]).unwrap()
                )
            }
        );
        assert_eq!(
            value(report, def(0x3324 + i)),
            &EffectValue::Known {
                value: ParameterValue::Quantity(FiniteQuantity::new(1., def(1)).unwrap())
            }
        );
    }
    assert!(!report.effects.iter().any(|e| matches!(&e.target, BoundEffectTarget::Contribution { key } if key.kind == ContributionKind::Multiply && (0..6).any(|i| key.stat == def(0x331b + i)))));
}
fn assert_unavailable(report: &SupportEffectsReport) {
    assert_eq!(
        report.outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                read: None
            },
            input: None,
        }
    );
}

#[test]
#[ignore = "requires ATTRIBUTE_EMPTY_MORE_RELEASE, ATTRIBUTE_STEP_RELEASE and ATTRIBUTE_COUNT_RELEASE"]
fn native_empty_more_retains_actual_original05_occurrences_and_six_consumers() {
    let mut world = World::new();
    assert_eq!(world.build.allocations.len(), 22);
    let packet: Value = family::read("source-vectors.json");
    for (case, expected) in [[27, 7, 105], [22, 12, 105]].into_iter().enumerate() {
        let report = world.evaluate();
        assert_outputs(&report, expected);
        for (i, s) in packet["cases"][case]["modes"]["MAIN"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            assert_eq!(s["value"], expected[i % 3]);
            assert_eq!(s["actual_query"]["result"], 1);
            assert!(s["multiply_records"].as_array().unwrap().is_empty());
        }
        // Compare exact occurrence keys, targets, values and inactive choice
        // effects with the already authenticated real class/choice fixture.
        let (recipe, build) = step::finite_parts();
        let mut before = World { recipe, build };
        if case == 1 {
            before.edit();
        }
        let old = before.evaluate();
        let contributions = |r: &OwnedEffectsReport| {
            r.effects
                .iter()
                .filter(|e| matches!(e.target, BoundEffectTarget::Contribution { .. }))
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(contributions(&old), contributions(&report));
        for i in 0..6 {
            let stat = if i < 3 { 0x3321 + i } else { 0x1d2e + i - 3 };
            assert_eq!(value(&old, def(stat)), value(&report, def(stat)));
        }
        if case == 0 {
            world.edit();
        }
    }
}

#[test]
#[ignore = "requires ATTRIBUTE_EMPTY_MORE_RELEASE, ATTRIBUTE_STEP_RELEASE and ATTRIBUTE_COUNT_RELEASE"]
fn native_empty_domain_rejects_all_potential_more_before_activation_or_values() {
    for stage in 0..6 {
        for enabled in [true, false] {
            let mut world = World::new();
            world.potential(stage, Some(1.2), enabled);
            let error = world
                .plan()
                .err()
                .expect("potential MORE must reject the empty domain");
            assert!(
                matches!(error, PlanError::Invalid(message) if message == "actual contribution has no ordered membership"),
                "stage {stage}, enabled {enabled}"
            );
        }
    }
    for (value, enabled) in [
        (Some(0.), true),
        (Some(1.), true),
        (None, true),
        (None, false),
    ] {
        let mut world = World::new();
        world.potential(0, value, enabled);
        let error = world
            .plan()
            .err()
            .expect("zero, identity or missing-valued potential MORE must reject");
        assert!(
            matches!(error, PlanError::Invalid(message) if message == "actual contribution has no ordered membership"),
            "value {value:?}, enabled {enabled}"
        );
    }
}

#[test]
#[ignore = "requires ATTRIBUTE_EMPTY_MORE_RELEASE, ATTRIBUTE_STEP_RELEASE and ATTRIBUTE_COUNT_RELEASE"]
fn native_missing_factor_and_actual_partial_coverage_never_receive_identity_defaults() {
    let mut missing = World::new();
    missing.remove_factor(3);
    let report = missing.evaluate();
    assert!(matches!(
        value(&report, def(0x1d2e)),
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingProducer,
            ..
        }
    ));
    assert_eq!(
        value(&report, def(0x3321)),
        &EffectValue::Known {
            value: ParameterValue::Integer(BoundedInteger::new(27).unwrap())
        }
    );

    let mut registry = World::new();
    registry
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .closure = source()
        .rules
        .contribution_queries
        .as_ref()
        .unwrap()
        .closure
        .clone();
    let plan = registry.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::IncompleteContributors)
    );
    assert_unavailable(&report);

    let mut owner = World::new();
    let subject = SchemaSubject::Definition(owner.build.character.class.address());
    let actual = source()
        .rules
        .owners
        .iter()
        .find(|o| o.owner == subject)
        .unwrap();
    assert!(!actual.programs.is_complete());
    owner
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == subject)
        .unwrap()
        .programs
        .closure = actual.programs.closure.clone();
    let plan = owner.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.iter().any(
        |g| g.reason == PlanGapReason::PartialPrograms && g.subject.as_ref() == Some(&subject)
    ));
    assert_unavailable(&report);
}

#[test]
#[ignore = "requires ATTRIBUTE_EMPTY_MORE_RELEASE, ATTRIBUTE_STEP_RELEASE and ATTRIBUTE_COUNT_RELEASE"]
fn native_empty_more_is_deterministic_across_edits_restoration_and_rayon() {
    let a = World::new();
    let mut b = a.clone();
    b.edit();
    let pa = a.plan().unwrap();
    let pb = b.plan().unwrap();
    let va = a.evaluate();
    let vb = b.evaluate();
    assert_ne!(pa.identity(), pb.identity());
    let mut scratch = pa.new_scratch();
    for (p, v) in [(&pa, &va), (&pb, &vb), (&pa, &va)] {
        assert_eq!(&effects(p.evaluate(&mut scratch).unwrap()), v);
    }
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
    let expected: Vec<_> = (0..16)
        .map(|i| if i % 2 == 0 { va.clone() } else { vb.clone() })
        .collect();
    assert_eq!(parallel, expected);
    assert_eq!(World::new().evaluate(), va);
}
