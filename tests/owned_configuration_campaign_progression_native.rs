//! Actual injected progression rules in a finite synthetic world, without PoB.
#[allow(dead_code)]
#[path = "../crates/poe-optimizer-engine/tests/support/owned_plan_fixture.rs"]
mod base;
mod support {
    pub use super::base::{Fixture, key};
    pub use super::def;
}
#[allow(dead_code)]
#[path = "../crates/poe-optimizer-engine/tests/support/owned_empty_support_contribution_fixture.rs"]
mod empty_support;

use base::{integer, key, occurrence, subject};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_metrics::*, owned_routing::*, owned_rules::*,
    owned_schema::*, owned_stages::*,
};
use poe_optimizer_data::{
    owned_metrics::OwnedMetricMapping, owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{fs, path::PathBuf, sync::Arc};

const FAMILY: &str = "data/owned/poe2/3887ae68/configuration-campaign-progression";
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
pub fn def<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::parse(ns(), s).unwrap()
}
fn read(name: &str) -> Value {
    serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(FAMILY)
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn packet<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_value(read(name)).unwrap()
}
// Only the generic synthetic scaffold is rebound. Published packet bytes and
// identities are deserialized unchanged and compared below.
fn rebind<T: Serialize + DeserializeOwned>(value: &T) -> T {
    fn walk(v: &mut Value) {
        if *v == json!({"game":"owned-plan-test","version":"v1"}) {
            *v = serde_json::to_value(ns()).unwrap();
        } else {
            match v {
                Value::Array(a) => a.iter_mut().for_each(walk),
                Value::Object(o) => o.values_mut().for_each(walk),
                _ => {}
            }
        }
    }
    let mut v = serde_json::to_value(value).unwrap();
    walk(&mut v);
    serde_json::from_value(v).unwrap()
}
struct World {
    fixture: base::Fixture,
    queries: DeclaredSet<ContributionQuery>,
    domains: SupportDiscoveryInput,
}
impl World {
    fn new(values: &[i64]) -> Self {
        let mut f = base::Fixture::new();
        f.schema = rebind(&f.schema);
        f.build = rebind(&f.build);
        f.scenario = rebind(&f.scenario);
        f.queries = rebind(&f.queries);
        f.owners = rebind(&f.owners);
        f.build.items.clear();
        f.build.equipment.clear();
        f.schema
            .definitions
            .retain(|d| !matches!(d, DefinitionDescriptor::Skill(_)));
        f.schema
            .slots
            .retain(|d| !matches!(d, SlotDescriptor::ActionOutput(_)));
        f.owners.retain(|o| {
            !matches!(
                o.owner,
                SchemaSubject::Definition(DefinitionAddress::Skill(_)) | SchemaSubject::Slot(_)
            )
        });
        for owner in &mut f.owners {
            owner.programs.members.clear();
        }
        f.schema
            .definitions
            .push(DefinitionDescriptor::Unit(DefinitionEntry {
                id: def("percent"),
                schema: SchemaState::Known(UnitSchema {
                    dimension: UnitDimension::PercentagePoints,
                }),
            }));
        let mut domains = base::assignment_only_domains(&f.owners);
        f.schema
            .definitions
            .extend(packet::<Vec<DefinitionDescriptor>>("definitions.json"));
        let slots: Vec<SlotDescriptor> = packet("slots.json");
        let SlotDescriptor::Parameter(parameter) = &slots[0] else {
            panic!()
        };
        f.build.character.rewards = values
            .iter()
            .enumerate()
            .map(|(i, &count)| RewardSelection {
                id: occurrence(40 + i as u64),
                definition: def("def.00000000000033b4"),
                parameters: vec![ParameterAssignment {
                    slot: parameter.id.clone(),
                    value: integer(count),
                }],
            })
            .collect();
        f.schema.slots.extend(slots);
        let consumer = read("consumer.json");
        let owners: Vec<DefinitionRules> =
            serde_json::from_value(consumer["owners"].clone()).unwrap();
        f.owners.extend(owners.clone());
        assert_eq!(f.owners.last(), owners.last());
        domains.providers.extend(
            serde_json::from_value::<Vec<SupportSourceDomainDeclaration>>(
                consumer["support_source_domains"].clone(),
            )
            .unwrap(),
        );
        // Test-only observation of the actual optional-selection query. This is
        // not a production damage rule or an assertion of game supplier closure.
        let observer: StatDefId = def("synthetic.selected-campaign-count");
        f.schema
            .definitions
            .push(DefinitionDescriptor::Stat(DefinitionEntry {
                id: observer.clone(),
                schema: SchemaState::Known(StatSchema {
                    value: ComputedValueType::Integer,
                    targets: vec![RuleEntityKind::Actor],
                }),
            }));
        f.owners.push(DefinitionRules {
            owner: subject(observer.clone()),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key("observe-fact"),
                context: RuleEntityKind::Actor,
                reads: vec![base::read(
                    "fact",
                    RuleReadSource::ContributionSelection {
                        entity: RuleEntity::Current,
                        query: key("campaign-bosses-defeated"),
                        group: key("sources"),
                        projection: ContributionSelectionProjection::Value,
                    },
                )],
                nodes: vec![base::read_node("fact", "fact")],
                effects: vec![base::effect(
                    "selected",
                    RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: observer.clone(),
                        value: key("fact"),
                    },
                )],
            }]),
        });
        f.receivers.members.push(StatReceiver {
            id: key("observe-fact"),
            stat: observer,
            program: key("observe-fact"),
            targets: vec![StatReceiverTarget::Player],
        });
        Self {
            fixture: f,
            queries: DeclaredSet::complete(
                serde_json::from_value(consumer["queries"].clone()).unwrap(),
            ),
            domains,
        }
    }
    fn plan(&self) -> OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage> {
        let schema =
            OwnedDefinitionSchemaPackage::new(self.fixture.schema.clone(), Default::default())
                .unwrap();
        let rules = RulePackageInput {
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: ns(),
            release: key("actual-progression-fixture"),
            semantics_version: key("fixture-v1"),
            operations_version: key(OWNED_RULE_OPERATIONS_V27),
            definitions: schema.identity().clone(),
            tables: vec![],
            owners: self.fixture.owners.clone(),
            receivers: self.fixture.receivers.clone(),
            effect_applications: Some(DeclaredSet::complete(vec![])),
            contribution_queries: Some(self.queries.clone()),
            existing_actor_rules: None,
            support_discovery: Some(self.domains.clone()),
        };
        let bindings = read("bindings.json");
        let programs: Vec<StagedRuleProgram> =
            serde_json::from_value(bindings["programs"].clone()).unwrap();
        let channels: Vec<FrozenStageChannel> =
            serde_json::from_value(bindings["frozen_channels"].clone()).unwrap();
        empty_support::checked_plan_with_stages(
            &self.fixture,
            rules,
            ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("empty-progression-routing"),
                definitions: schema.identity().clone(),
                outputs: vec![],
            },
            Default::default(),
            |stages| {
                for target in &programs {
                    let existing = stages
                        .programs
                        .members
                        .iter_mut()
                        .find(|p| p.owner == target.owner && p.program == target.program)
                        .unwrap();
                    *existing = target.clone();
                }
                stages.frozen_channels.extend(channels);
            },
        )
        .unwrap()
    }
    fn metric_plan(&self) -> OwnedSupportMetricPlan<OwnedDefinitionSchemaPackage> {
        let effects = Arc::new(self.plan());
        let mapping = Arc::new(
            OwnedMetricMapping::new(
                MetricMappingInput {
                    schema_version: OWNED_METRIC_MAPPING_VERSION,
                    namespace: ns(),
                    release: key("empty-progression-metrics"),
                    definitions: effects.definitions().identity().clone(),
                    bindings: vec![],
                },
                effects.definitions(),
                Default::default(),
            )
            .unwrap(),
        );
        OwnedSupportMetricPlan::compile(effects, mapping).unwrap()
    }
}
fn selected(report: &SupportEffectsReport) -> &EffectValue {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("{:?}", report.outcome)
    };
    &effects
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def("synthetic.selected-campaign-count"),
                }
        })
        .unwrap()
        .value
}
#[test]
fn actual_packet_preserves_all_original_values_and_representable_native_boundaries() {
    for value in [0, 1, 55, 57, 57, 56, 56, BoundedInteger::MAX] {
        let world = World::new(&[value]);
        let plan = world.plan();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert_eq!(
            selected(&report),
            &EffectValue::Known {
                value: integer(value)
            }
        );
        let metrics = world.metric_plan();
        let assessment = metrics
            .evaluate(&mut metrics.new_scratch())
            .unwrap()
            .evaluation
            .requirements;
        assert!(assessment.is_satisfied());
        assert_eq!(assessment.results.len(), 1);
    }
}
#[test]
fn actual_packet_reports_distinct_equal_and_conflicting_authorities() {
    for values in [[56, 56], [56, 57]] {
        let world = World::new(&values);
        let plan = world.metric_plan();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert_eq!(
            report.evaluation.requirements.coverage,
            RequirementCoverage::Complete
        );
        assert!(report.evaluation.requirements.has_violations());
        assert_eq!(report.evaluation.requirements.results.len(), 2);
        for row in report.evaluation.requirements.results {
            assert_eq!(row.outcome, RequirementOutcome::Violated);
            assert_eq!(row.code, key("exactly-one-campaign-progression-authority"));
        }
        let effects = world.plan();
        let raw = effects.evaluate(&mut effects.new_scratch()).unwrap();
        if values[0] == values[1] {
            assert_eq!(selected(&raw), &EffectValue::Known { value: integer(56) });
        } else {
            assert!(matches!(
                selected(&raw),
                EffectValue::Unresolved {
                    reason: PlanGapReason::ConflictingContributors,
                    ..
                }
            ));
        }
    }
}
#[test]
fn actual_packet_keeps_missing_and_unknown_membership_distinct_from_zero() {
    let empty = World::new(&[]).plan();
    assert!(matches!(
        selected(&empty.evaluate(&mut empty.new_scratch()).unwrap()),
        EffectValue::Unresolved {
            reason: PlanGapReason::AbsentSelection,
            ..
        }
    ));
    for index in 0..2 {
        let mut world = World::new(&[56]);
        let query = &mut world.queries.members[index];
        query.groups[0].members.closure = SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: subject(query.stat.clone()),
                facet: SchemaFacet::GameRules,
                code: key("synthetic-unknown-membership"),
            }],
        };
        let plan = world.metric_plan();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(matches!(
            report.evaluation.requirements.coverage,
            RequirementCoverage::Incomplete { .. }
        ));
        assert!(!report.evaluation.requirements.is_satisfied());
    }
}
#[test]
fn actual_packet_is_identical_with_fresh_reused_and_parallel_scratch() {
    let a = World::new(&[55]).plan();
    let b = World::new(&[57]).plan();
    let expected_a = a.evaluate(&mut a.new_scratch()).unwrap();
    let expected_b = b.evaluate(&mut b.new_scratch()).unwrap();
    assert_eq!(
        selected(&expected_a),
        &EffectValue::Known { value: integer(55) }
    );
    assert_eq!(
        selected(&expected_b),
        &EffectValue::Known { value: integer(57) }
    );
    assert_ne!(a.identity(), b.identity());
    // One scratch crosses A -> B -> A, so the result must come from the current
    // raw parameter rather than a previous attempt's cached selected fact.
    let mut raw_scratch = a.new_scratch();
    for _ in 0..4 {
        assert_eq!(a.evaluate(&mut raw_scratch).unwrap(), expected_a);
        assert_eq!(b.evaluate(&mut raw_scratch).unwrap(), expected_b);
        assert_eq!(a.evaluate(&mut raw_scratch).unwrap(), expected_a);
    }
    let raw_reports: Vec<_> = (0..16)
        .into_par_iter()
        .map_init(
            || a.new_scratch(),
            |scratch, _| {
                let first = a.evaluate(scratch).unwrap();
                let changed = b.evaluate(scratch).unwrap();
                let restored = a.evaluate(scratch).unwrap();
                (first, changed, restored)
            },
        )
        .collect();
    for (first, changed, restored) in raw_reports {
        assert_eq!(first, expected_a);
        assert_eq!(changed, expected_b);
        assert_eq!(restored, expected_a);
    }

    let plan = World::new(&[56]).metric_plan();
    let expected = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let mut scratch = plan.new_scratch();
    for _ in 0..4 {
        assert_eq!(plan.evaluate(&mut scratch).unwrap(), expected);
    }
    let reports: Vec<_> = (0..16)
        .into_par_iter()
        .map_init(
            || plan.new_scratch(),
            |scratch, _| plan.evaluate(scratch).unwrap(),
        )
        .collect();
    assert!(reports.iter().all(|report| report == &expected));
}
