//! The bounded union uses actual contribution bodies, never source zero inference.
#[allow(dead_code)]
#[path = "support/owned_life_base_domain.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::{BoundedInteger, FiniteQuantity},
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, RuleFact, RuleScratch, UnsupportedDomainCause,
};
use rayon::prelude::*;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn authenticated_union_has_distinct_exact_writers_and_one_guard() {
    family::check_authored();
}

#[test]
fn future_component_changes_cannot_escape_the_finite_union_census() {
    let d = family::dependencies();
    let mut schema = d.native_schema.clone();
    schema.definitions.extend(d.classes.clone());
    let mut rules = d.native_rules.clone();
    rules.owners.clear();
    for w in &d.writers {
        if let Some(owner) = rules.owners.iter_mut().find(|o| o.owner == w.owner) {
            owner.programs.members.push(w.program.clone());
        } else {
            rules.owners.push(DefinitionRules {
                owner: w.owner.clone(),
                programs: DeclaredSet::complete(vec![w.program.clone()]),
            });
        }
    }
    for complete in [
        family::shared_after(),
        family::final_after(),
        d.inherent_owner.clone(),
        d.equipment_owner.clone(),
    ] {
        if let Some(owner) = rules.owners.iter_mut().find(|o| o.owner == complete.owner) {
            *owner = complete;
        } else {
            rules.owners.push(complete);
        }
    }
    rules.existing_actor_rules = d.existing_actor_rules.clone();
    rules.receivers = DeclaredSet {
        members: d.inherent_receivers.clone(),
        closure: d.receiver_registry_closure.clone(),
    };
    rules.contribution_queries = Some(DeclaredSet {
        members: d.queries.clone(),
        closure: d.query_registry_closure.clone(),
    });
    family::check_component_rules(&schema, &rules);
    for mutation in 0..6 {
        let mut s = schema.clone();
        let mut r = rules.clone();
        match mutation {
            0 => {
                let mut extra = d.classes[0].clone();
                let DefinitionDescriptor::Class(row) = &mut extra else {
                    panic!()
                };
                row.id = poe_optimizer_core::owned_definitions::ClassDefId::parse(
                    row.id.namespace().clone(),
                    "fixture.additional-class",
                )
                .unwrap();
                let SchemaState::Known(schema) = &mut row.schema else {
                    panic!()
                };
                schema.level.maximum = BoundedInteger::new(101).unwrap();
                s.definitions.push(extra);
            }
            1 => {
                let owner = r
                    .owners
                    .iter_mut()
                    .find(|o| o.owner == d.equipment_owner.owner)
                    .unwrap();
                let p = owner
                    .programs
                    .members
                    .iter_mut()
                    .find(|p| p.id.as_str() == "effective-amount")
                    .unwrap();
                p.nodes.pop();
            }
            2 => {
                let mut extra = d.inherent_owner.clone();
                extra.owner = SchemaSubject::Definition(d.classes[0].address());
                r.owners.push(extra);
            }
            3 => {
                let mut extra = d.inherent_receivers[0].clone();
                extra.id = "fixture.extra-inherent-receiver".parse().unwrap();
                r.receivers.members.push(extra);
            }
            4 => {
                let q = &mut r.contribution_queries.as_mut().unwrap().members[0];
                let duplicate = q.groups[0].members.members[0].clone();
                q.groups[1].members.members.push(duplicate);
            }
            5 => r.existing_actor_rules.as_mut().unwrap().members[0]
                .targets
                .clear(),
            _ => unreachable!(),
        }
        assert!(
            std::panic::catch_unwind(|| family::check_component_rules(&s, &r)).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn guard_inverse_refuses_removed_activation_changed_recipient_and_coverage() {
    for mutation in 0..6 {
        let mut p = family::program();
        let mut a = family::shared_after();
        let mut f = family::final_after();
        match mutation {
            0 => p.effects[0].when = None,
            1 => {
                let RuleEffectKind::Contribute { value, .. } = &mut p.effects[0].effect else {
                    panic!()
                };
                *value = "amount".parse().unwrap();
            }
            2 => {
                let RuleEffectKind::Contribute { entity, .. } = &mut p.effects[0].effect else {
                    panic!()
                };
                *entity = RuleEntity::Enemy;
            }
            3 => a.programs.closure = SchemaClosure::Complete,
            4 => f.programs.closure = SchemaClosure::Complete,
            5 => {
                let RuleExpression::CheckNumericDomain { domain, .. } =
                    &mut p.nodes.last_mut().unwrap().expression
                else {
                    panic!()
                };
                let NumericDomain::Quantity(range) = domain else {
                    panic!()
                };
                range.maximum =
                    FiniteQuantity::new(family::BOUND * 2., range.maximum.unit().clone()).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(
            std::panic::catch_unwind(|| family::check_transition(&p, &a, &f)).is_err(),
            "mutation {mutation}"
        );
    }
}
fn component() -> (
    OwnedDefinitionSchemaPackage,
    CompiledRulePackage,
    SchemaSubject,
) {
    let d = family::dependencies();
    let schema = OwnedDefinitionSchemaPackage::new(d.native_schema, Default::default()).unwrap();
    let mut rules = d.native_rules;
    rules.definitions = schema.identity().clone();
    let compiled = CompiledRulePackage::compile(&rules, &schema, Default::default()).unwrap();
    (schema, compiled, d.shared_player.owner)
}
fn quantity(n: f64) -> ParameterValue {
    let d = family::dependencies();
    let ComputedValueType::Quantity { unit } = d
        .shared_player
        .programs
        .members
        .iter()
        .find(|p| p.id == family::program().id)
        .unwrap()
        .reads[0]
        .value_type
        .clone()
    else {
        panic!()
    };
    ParameterValue::Quantity(FiniteQuantity::new(n, unit).unwrap())
}
fn evaluate(
    c: &CompiledRulePackage,
    schema: &OwnedDefinitionSchemaPackage,
    owner: &SchemaSubject,
    n: Option<f64>,
    flags: [Option<bool>; 3],
    scratch: &mut RuleScratch,
) -> EffectDisposition {
    let mut facts = Vec::new();
    if let Some(n) = n {
        facts.push(RuleFact {
            read: "amount".parse().unwrap(),
            value: quantity(n),
        });
    }
    for (name, value) in ["all-disabled", "strength-disabled", "life-disabled"]
        .into_iter()
        .zip(flags)
    {
        if let Some(value) = value {
            facts.push(RuleFact {
                read: name.parse().unwrap(),
                value: ParameterValue::Boolean(value),
            });
        }
    }
    let r = c
        .evaluate(owner, &family::program().id, &facts, schema, scratch)
        .unwrap();
    assert_eq!(r.effects.len(), 1);
    r.effects[0].disposition.clone()
}
#[test]
fn actual_shared_bridge_preserves_bounds_zero_inactivity_and_diagnostic_causes() {
    let (schema, c, owner) = component();
    let mut scratch = c.new_scratch();
    for n in [-family::BOUND, -1., -0., 0., 54., family::BOUND] {
        assert_eq!(
            evaluate(&c, &schema, &owner, Some(n), [Some(false); 3], &mut scratch),
            EffectDisposition::Applied { value: quantity(n) }
        );
    }
    for n in [
        -family::BOUND - 1.,
        family::BOUND + 1.,
        (-family::BOUND).next_down(),
        family::BOUND.next_up(),
    ] {
        let domain = serde_json::from_value(
            family::read::<serde_json::Value>("proof.json")["life_domain"].clone(),
        )
        .unwrap();
        assert_eq!(
            evaluate(&c, &schema, &owner, Some(n), [Some(false); 3], &mut scratch),
            EffectDisposition::UnsupportedDomain {
                node: "bounded-inherent-life".parse().unwrap(),
                domain: UnsupportedDomainCause::NumericRange {
                    value: quantity(n),
                    domain: Box::new(domain)
                }
            }
        );
    }
    for index in 0..3 {
        let mut flags = [Some(false); 3];
        flags[index] = Some(true);
        assert_eq!(
            evaluate(&c, &schema, &owner, Some(f64::MAX), flags, &mut scratch),
            EffectDisposition::Inactive
        );
        assert_eq!(
            evaluate(&c, &schema, &owner, None, flags, &mut scratch),
            EffectDisposition::Inactive
        );
        flags[index] = None;
        assert!(matches!(
            evaluate(&c, &schema, &owner, Some(54.), flags, &mut scratch),
            EffectDisposition::Unresolved { .. }
        ));
    }
    assert_eq!(
        evaluate(&c, &schema, &owner, None, [Some(false); 3], &mut scratch),
        EffectDisposition::Unresolved {
            input: "amount".parse().unwrap()
        }
    );
    let expected: Vec<_> = [0., 54., family::BOUND, family::BOUND + 1.]
        .into_iter()
        .map(|n| {
            evaluate(
                &c,
                &schema,
                &owner,
                Some(n),
                [Some(false); 3],
                &mut c.new_scratch(),
            )
        })
        .collect();
    let actual: Vec<_> = (0..16)
        .into_par_iter()
        .map_init(
            || c.new_scratch(),
            |scratch, n| {
                evaluate(
                    &c,
                    &schema,
                    &owner,
                    Some([0., 54., family::BOUND, family::BOUND + 1.][n % 4]),
                    [Some(false); 3],
                    scratch,
                )
            },
        )
        .collect();
    for (n, result) in actual.iter().enumerate() {
        assert_eq!(result, &expected[n % 4]);
    }
}

#[test]
fn actual_integer_strength_producer_reaches_guard_before_any_base_regrouping() {
    let (schema, c, owner) = component();
    let d = family::dependencies();
    let mut scratch = c.new_scratch();
    let source = &d.inherent_owner.programs.members[0];
    for (half, double, factor) in [
        (false, false, 2i64),
        (true, false, 1),
        (false, true, 4),
        (true, true, 2),
    ] {
        for strength in [
            -(1i64 << 37) / factor,
            -27,
            0,
            27,
            (1i64 << 37) / factor,
            (1i64 << 37) / factor + 1,
        ] {
            let facts: Vec<_> = source
                .reads
                .iter()
                .map(|read| RuleFact {
                    read: read.id.clone(),
                    value: match read.id.as_str() {
                        "strength" => {
                            ParameterValue::Integer(BoundedInteger::new(strength).unwrap())
                        }
                        "inherent-doubled" => ParameterValue::Boolean(double),
                        "strength-life-halved" => ParameterValue::Boolean(half),
                        _ => ParameterValue::Boolean(false),
                    },
                })
                .collect();
            let result = c
                .evaluate(
                    &d.inherent_owner.owner,
                    &source.id,
                    &facts,
                    &schema,
                    &mut scratch,
                )
                .unwrap();
            let EffectDisposition::Applied {
                value: ParameterValue::Quantity(q),
            } = &result.effects[0].disposition
            else {
                panic!()
            };
            assert_eq!(q.value(), (strength * factor) as f64);
            let actual = evaluate(
                &c,
                &schema,
                &owner,
                Some(q.value()),
                [Some(false); 3],
                &mut scratch,
            );
            if q.value().abs() <= family::BOUND {
                assert_eq!(
                    actual,
                    EffectDisposition::Applied {
                        value: quantity(q.value())
                    }
                );
            } else {
                assert!(matches!(
                    actual,
                    EffectDisposition::UnsupportedDomain { .. }
                ));
            }
        }
    }
}

#[test]
fn global_union_bound_covers_cancellation_and_all_four_partitions() {
    // Independent exact-integer oracle at the hard effect ceiling. This proves
    // the grouping law; native graph tests separately enforce membership/limit.
    let n = poe_optimizer_engine::owned_plan::PlanLimits::default().max_effects;
    let bound = 1i128 << 37;
    for mode in 0..4 {
        let values: Vec<i128> = (0..n)
            .map(|i| match mode {
                0 => bound,
                1 => -bound,
                2 => {
                    if i < n / 2 {
                        bound
                    } else {
                        -bound
                    }
                }
                _ => {
                    if i % 2 == 0 {
                        bound
                    } else {
                        -bound + 1
                    }
                }
            })
            .collect();
        let expected: i128 = values.iter().sum();
        let mut direct = 0.;
        let mut exact = 0i128;
        for value in &values {
            direct += *value as f64;
            exact += value;
            assert_eq!(direct, exact as f64);
        }
        let mut groups = [0.; 4];
        for (i, value) in values.iter().enumerate() {
            groups[i % 4] += *value as f64;
        }
        assert_eq!(groups.iter().sum::<f64>(), expected as f64);
        assert!(exact.abs() <= 1i128 << 53);
    }
    // An unbounded cross-bucket floating sum is deliberately NOT proved.
    let source = (1e16_f64 + -1e16_f64) + 1.;
    let grouped = 1e16_f64 + (-1e16_f64 + 1.);
    assert_eq!((source, grouped), (1., 0.));
}

#[test]
#[ignore = "requires LIFE_BASE_DOMAIN_PRIOR/OUTPUT; all five imports remain incomplete"]
fn publish_bounded_life_base_preserving_all_five() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_LIFE_BASE_DOMAIN_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_LIFE_BASE_DOMAIN_OUTPUT").unwrap()),
        &family::data(),
        &[],
        &["authoring.json", "dependencies.json", "proof.json"],
        family::stage,
        json!({"changed_programs":1,"numeric_guards":1,"refined_order_gaps":1,"new_definitions":0,"supplier_admission":false,"conversion_order_proved":false,"whole_build_parity":false}),
        [106, 117, 109, 123, 4],
    );
}
