//! Native recipe components; complete source/build coverage is deliberately absent.
#[path = "support/owned_effective_gem_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_rules::EffectDisposition;
use poe_optimizer_import::owned_effective_gem_recipe::*;
fn applied(value: ParameterValue) -> EffectDisposition {
    EffectDisposition::Applied { value }
}

#[test]
fn roles_preserve_corruption_order_without_corrupted_flag_or_final_projection() {
    let f = Fixture::new();
    let output = f.compile();
    assert_eq!(
        output.programs[0].phase,
        EffectiveGemInputPhase::ActivePreSupport
    );
    assert_eq!(output.programs[0].program.context, RuleEntityKind::Skill);
    assert_eq!(
        output.programs[1].phase,
        EffectiveGemInputPhase::SupportPreparation
    );
    assert_eq!(
        output.programs[1].program.context,
        RuleEntityKind::SupportOrigin
    );
    assert_eq!(
        f.evaluate(0, 19, -30.0, Some(5.0), Some(20.0)),
        vec![applied(qty(6.0, &f.count)), applied(qty(23.0, &f.percent))]
    );
    assert_eq!(
        f.evaluate(1, 19, -30.0, Some(5.0), Some(20.0)),
        vec![applied(int(24)), applied(qty(23.0, &f.percent))]
    );
    for row in &output.programs {
        assert!(row.program.effects.iter().all(|effect| matches!(
            effect.effect,
            RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                ..
            }
        )));
        assert!(row.program.reads.iter().all(|read| !matches!(
            read.source,
            RuleReadSource::Parameter { .. }
        ) || row.phase
            == EffectiveGemInputPhase::ActivePreSupport));
        assert!(
            row.program
                .reads
                .iter()
                .all(|read| read.value_type != ComputedValueType::Boolean
                    || matches!(read.source, RuleReadSource::HasGemQuality { .. }))
        );
    }
}
#[test]
fn dense_levels_clamp_before_integrality_and_fractional_values_use_natural_maximum() {
    let f = Fixture::new();
    for (raw, external, expected) in [
        (2, 0.5, 20),
        (40, 0.5, 40),
        (1, -0.5, 1),
        (40, 50.0, 40),
        (1, -100.0, 1),
        (19, 11.0, 30),
        (20, 2.0, 22),
    ] {
        assert_eq!(
            f.evaluate(1, raw, 0.0, Some(external), Some(0.0))[0],
            applied(int(expected))
        );
    }
    let mut single = Fixture::new();
    single.input.bindings[1].role = EffectiveGemRecipeRole::SupportPreparation {
        levels: DenseGemLevelPolicy {
            maximum: 1,
            natural_maximum: 1,
        },
    };
    assert_eq!(
        single.evaluate(1, 19, 0.0, Some(0.5), Some(0.0))[0],
        applied(int(1))
    );
}
#[test]
fn active_pre_support_preserves_fractional_high_and_post_property_negative_levels() {
    let f = Fixture::new();
    for (raw, corruption, external, expected) in [
        (12, 0.25, 0.0, 12.25),
        (12, 50.0, 0.0, 62.0),
        (12, -50.0, -3.0, -2.0),
        (12, 0.0, 0.25, 12.25),
    ] {
        assert_eq!(
            f.evaluate(0, raw, corruption, Some(external), Some(13.0))[0],
            applied(qty(expected, &f.count))
        );
    }
    // A later +0.75 supported property can turn 12.25 into valid13. No early
    // natural-maximum fallback may destroy that input before final validation.
    let output = f.compile();
    assert!(
        output.programs[0]
            .program
            .nodes
            .iter()
            .all(|node| !matches!(node.expression, RuleExpression::QuantizeInteger { .. }))
    );
}
#[test]
fn absent_known_quality_is_zero_but_unknown_inputs_remain_unresolved() {
    let f = Fixture::new();
    assert_eq!(
        f.evaluate(0, 19, 0.0, Some(0.0), None)[1],
        applied(qty(3.0, &f.percent))
    );
    assert!(matches!(
        f.evaluate(0, 19, 0.0, None, Some(20.0))[0],
        EffectDisposition::Unresolved { .. }
    ));
    let mut partial = Fixture::new();
    partial.edit_schema(|schema| {
        for row in &mut schema.definitions {
            if let DefinitionDescriptor::Gem(row) = row
                && let SchemaState::Known(gem) = &mut row.schema
            {
                gem.quality.allowed_kinds.closure = SchemaClosure::Partial {
                    gaps: vec![gap(&row.id)],
                };
            }
        }
    });
    assert!(
        compile_effective_gem_recipe(&partial.input, &partial.schema, Default::default()).is_err()
    );
    for binding in &mut partial.input.bindings {
        binding.quality_absence = QualityAbsencePolicy::RequireSelectedQuality;
    }
    let prior = partial.schema.input().clone();
    let output = partial.compile();
    assert_eq!(&prior, partial.schema.input());
    assert!(output.programs.iter().all(|row| {
        row.program
            .reads
            .iter()
            .all(|read| !matches!(read.source, RuleReadSource::HasGemQuality { .. }))
    }));
    assert_eq!(
        partial.evaluate(0, 19, 0.0, Some(0.0), Some(20.0))[1],
        applied(qty(23.0, &partial.percent))
    );
    assert!(matches!(
        partial.evaluate(0, 19, 0.0, Some(0.0), None)[1],
        EffectDisposition::Unresolved { .. }
    ));
    let compiled = partial.executable(&output);
    assert!(
        compiled
            .input()
            .owners
            .iter()
            .all(|owner| !owner.programs.is_complete())
    );
    assert!(
        !serde_json::to_value(output)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("owners")
    );
}
#[test]
fn same_output_ids_are_valid_across_exact_owners_but_duplicates_are_rejected() {
    let mut f = Fixture::new();
    f.compile();
    f.input.bindings.push(f.input.bindings[0].clone());
    assert!(compile_effective_gem_recipe(&f.input, &f.schema, Default::default()).is_err());
    f.input.bindings[2].program = key("competing");
    assert!(compile_effective_gem_recipe(&f.input, &f.schema, Default::default()).is_err());
}
#[test]
fn schema_identity_units_roles_dense_domain_and_exact_corruption_owner_are_checked() {
    let f = Fixture::new();
    for mutation in 0..7 {
        let mut input = f.input.clone();
        match mutation {
            0 => input.definitions.release = "foreign".into(),
            1 => input.bindings[0].level_unit = f.percent.clone(),
            2 => {
                input.bindings[1].role = EffectiveGemRecipeRole::SupportPreparation {
                    levels: DenseGemLevelPolicy {
                        maximum: 0,
                        natural_maximum: 20,
                    },
                }
            }
            3 => {
                input.bindings[1].role = EffectiveGemRecipeRole::SupportPreparation {
                    levels: DenseGemLevelPolicy {
                        maximum: 40,
                        natural_maximum: 41,
                    },
                }
            }
            4 => input.bindings[0].level_output = input.bindings[0].quality_output.clone(),
            5 => {
                input.bindings[0].role = EffectiveGemRecipeRole::SupportPreparation {
                    levels: DenseGemLevelPolicy {
                        maximum: 40,
                        natural_maximum: 20,
                    },
                }
            }
            6 => {
                if let EffectiveGemRecipeRole::ActivePreSupport { corruption } =
                    &mut input.bindings[0].role
                {
                    corruption.declaration = SlotOwnerDefId::Gem(id("support"));
                }
            }
            _ => unreachable!(),
        }
        assert!(
            compile_effective_gem_recipe(&input, &f.schema, Default::default()).is_err(),
            "mutation{mutation}"
        );
    }
    let mut input = f.input.clone();
    let duplicate = input.bindings[0].external_level[0].clone();
    input.bindings[0].external_level.push(duplicate);
    assert!(compile_effective_gem_recipe(&input, &f.schema, Default::default()).is_err());
}
#[test]
fn bounded_preflight_determinism_and_policy_identity() {
    let f = Fixture::new();
    let output = f.compile();
    assert_eq!(output, f.compile());
    for limits in [
        EffectiveGemRecipeLimits {
            max_policy_bytes: 10,
            ..Default::default()
        },
        EffectiveGemRecipeLimits {
            max_output_bytes: 1024,
            ..Default::default()
        },
        EffectiveGemRecipeLimits {
            max_bindings: 1,
            ..Default::default()
        },
        EffectiveGemRecipeLimits {
            max_channels_per_binding: 1,
            ..Default::default()
        },
        EffectiveGemRecipeLimits {
            max_nodes: 1,
            ..Default::default()
        },
        EffectiveGemRecipeLimits {
            max_work: 1,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            compile_effective_gem_recipe(&f.input, &f.schema, limits),
            Err(EffectiveGemRecipeError::Limit(_))
        ));
    }
    let mut changed = f.input.clone();
    changed.bindings[1].role = EffectiveGemRecipeRole::SupportPreparation {
        levels: DenseGemLevelPolicy {
            maximum: 40,
            natural_maximum: 19,
        },
    };
    assert_ne!(
        output.policy,
        compile_effective_gem_recipe(&changed, &f.schema, Default::default())
            .unwrap()
            .policy
    );
}
