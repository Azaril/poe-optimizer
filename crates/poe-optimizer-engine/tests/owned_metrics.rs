//! Native metric behavior from owned requests, without explicit result/fact injection.
#[path = "support/owned_metric_fixture.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::FiniteQuantity, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_metrics::*;
use poe_optimizer_engine::owned_plan::*;
use std::sync::Arc;
use support::*;

#[test]
fn preserves_query_order_exact_occurrences_units_and_changed_input() {
    let mut f = fixture();
    let plan = compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_eq!(
        report
            .results
            .iter()
            .map(|r| r.request.clone())
            .collect::<Vec<_>>(),
        f.queries.requests
    );
    assert_eq!(
        report.results.iter().map(number).collect::<Vec<_>>(),
        [11.0, 16.0, 18.0, 20.0, 11.0]
    );
    f.build
        .gems
        .iter_mut()
        .find(|gem| gem.id == occurrence(28))
        .unwrap()
        .level = 37;
    f.build.items[0].modifiers[0].rolls[0].value = integer(7);
    let changed = compile(&f);
    let report = changed.evaluate(&mut changed.new_scratch()).unwrap();
    assert_ne!(changed.identity(), plan.identity());
    assert_eq!(
        report.results.iter().map(number).collect::<Vec<_>>(),
        [37.0, 24.0, 22.0, 20.0, 37.0]
    );
}
#[test]
fn false_actor_grant_blocks_known_parent_projection_without_aliasing_sibling() {
    let mut f = fixture();
    f.build
        .gems
        .iter_mut()
        .find(|g| g.id == occurrence(28))
        .unwrap()
        .parameters[0]
        .value = ParameterValue::Boolean(false);
    let plan = compile(&f);
    let mut scratch = plan.new_scratch();
    let effects = plan.effect_plan().evaluate(&mut scratch).unwrap();
    let key = PlanValueKey::Stat {
        entity: ConcreteEntity::Actor(child_actor(30)),
        stat: def("child-quantity"),
    };
    assert!(matches!(
        &effects.values.iter().find(|r| r.key == key).unwrap().value,
        EffectValue::Known { .. }
    ));
    let report = plan.evaluate(&mut scratch).unwrap();
    assert_eq!(report.results[0].value, EffectValue::Inactive);
    assert_eq!(report.results[4].value, EffectValue::Inactive);
    assert_eq!(number(&report.results[3]), 20.0);
    assert_eq!(number(&report.results[1]), 16.0);
}
#[test]
fn missing_grant_producer_never_publishes_projected_value() {
    let mut f = fixture();
    f.owner_mut(&summoner_owner()).programs.members[0]
        .effects
        .retain(|e| e.id != key("activate"));
    let plan = compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    for i in [0, 3, 4] {
        assert!(matches!(
            report.results[i].value,
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                ..
            }
        ));
    }
    assert_eq!(number(&report.results[1]), 16.0);
}
#[test]
fn disabled_root_is_inactive_and_unknown_input_is_unresolved() {
    let mut f = fixture();
    f.build
        .skills
        .iter_mut()
        .find(|s| s.id == occurrence(30))
        .unwrap()
        .enabled = false;
    let plan = compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_eq!(report.results[0].value, EffectValue::Inactive);
    // Optional import absence can never masquerade as an explicit zero quantity.
    let mut f = fixture();
    f.build
        .gems
        .iter_mut()
        .find(|g| g.id == occurrence(28))
        .unwrap()
        .parameters
        .clear();
    // A required authoring slot rejects before numerical evaluation, as it should.
    assert!(f.compile().is_err());
}
#[test]
fn partial_rule_closure_blocks_every_final_metric() {
    let mut f = fixture();
    let owner = modifier_owner();
    f.owner_mut(&owner).programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner.clone(),
            facet: SchemaFacet::GameRules,
            code: key("unconverted"),
        }],
    };
    let plan = compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(!report.gaps.is_empty());
    assert!(report.results.iter().all(|r| matches!(
        r.value,
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    )));
}
#[test]
fn missing_mapping_and_missing_final_producer_have_distinct_typed_results() {
    let f = fixture();
    let effects = Arc::new(f.compile().unwrap());
    let mut input = mapping_input(effects.definitions());
    input.bindings.clear();
    let mapping = Arc::new(
        OwnedMetricMapping::new(input, effects.definitions(), MetricMappingLimits::default())
            .unwrap(),
    );
    let plan = OwnedMetricPlan::compile(Arc::clone(&effects), mapping).unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.results.iter().all(|r| r.stat.is_none()
        && matches!(
            r.value,
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingMetricBinding,
                ..
            }
        )));
    let mut input = mapping_input(effects.definitions());
    input.bindings[0].stat = def("absent-quantity");
    let mapping = Arc::new(
        OwnedMetricMapping::new(input, effects.definitions(), MetricMappingLimits::default())
            .unwrap(),
    );
    let other = OwnedMetricPlan::compile(effects, mapping).unwrap();
    assert_ne!(other.identity(), plan.identity());
    let report = other.evaluate(&mut other.new_scratch()).unwrap();
    assert!(matches!(
        report.results[1].value,
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingProducer,
            ..
        }
    ));
}
#[test]
fn numerical_failure_is_preserved_and_scratch_recovers_across_plans() {
    let good = compile(&fixture());
    let mut f = fixture();
    let parent = &mut f.owner_mut(&summoner_owner()).programs.members[0];
    let one = parent
        .nodes
        .iter_mut()
        .find(|n| n.id == key("one-unit"))
        .unwrap();
    one.expression = RuleExpression::Literal {
        value: ParameterValue::Quantity(FiniteQuantity::new(f64::MAX, def("count")).unwrap()),
    };
    let bad = compile(&f);
    let mut scratch = good.new_scratch();
    let expected = good.evaluate(&mut scratch).unwrap();
    let failed = bad.evaluate(&mut scratch).unwrap();
    assert!(
        matches!(&failed.results[0].value,EffectValue::NumericalError{node,..} if node==&key("measurement"))
    );
    assert_eq!(expected, good.evaluate(&mut scratch).unwrap());
    let shared = Arc::new(good);
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let plan = Arc::clone(&shared);
            std::thread::spawn(move || plan.evaluate(&mut plan.new_scratch()).unwrap())
        })
        .collect();
    for thread in threads {
        assert_eq!(expected, thread.join().unwrap());
    }
}
#[test]
fn stale_mapping_cannot_be_rebound_by_the_evaluator() {
    let f = fixture();
    let effects = Arc::new(f.compile().unwrap());
    let mapping = Arc::new(
        OwnedMetricMapping::new(
            mapping_input(effects.definitions()),
            effects.definitions(),
            MetricMappingLimits::default(),
        )
        .unwrap(),
    );
    let mut changed = f;
    changed.schema.release = key("new-schema");
    let changed = Arc::new(changed.compile().unwrap());
    assert!(OwnedMetricPlan::compile(changed, mapping).is_err());
}

#[test]
fn finite_domain_failure_retains_original_node_table_key_and_bounds() {
    let mut f = fixture();
    let bound = poe_optimizer_core::owned_definitions::BoundedInteger::new(1).unwrap();
    f.tables.push(IntegerRuleTable {
        id: key("reviewed-domain"),
        minimum: bound,
        maximum: bound,
        value_type: ComputedValueType::Quantity { unit: def("count") },
        rows: vec![ParameterValue::Quantity(
            FiniteQuantity::new(99.0, def("count")).unwrap(),
        )],
    });
    let parent = &mut f.owner_mut(&summoner_owner()).programs.members[0];
    parent
        .nodes
        .iter_mut()
        .find(|n| n.id == key("measurement"))
        .unwrap()
        .expression = RuleExpression::LookupIntegerTable {
        table: key("reviewed-domain"),
        key: key("level"),
    };
    let plan = compile(&f);
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_eq!(
        report.results[0].value,
        EffectValue::UnsupportedDomain {
            node: key("measurement"),
            table: key("reviewed-domain"),
            key: poe_optimizer_core::owned_definitions::BoundedInteger::new(11).unwrap(),
            minimum: bound,
            maximum: bound
        }
    );
    assert_eq!(number(&report.results[1]), 16.0);
}
