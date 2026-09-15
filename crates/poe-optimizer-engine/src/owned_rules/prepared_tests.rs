use super::*;
use poe_optimizer_core::{
    owned_build::DeclaredSlot, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
#[path = "../../tests/support/owned_rule_fixture.rs"]
mod fixture;

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn integer(value: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(value).unwrap())
}
fn facts(
    prepared: &PreparedRuleProgram,
    supplied: &[(OwnedDefinitionKey, ParameterValue)],
) -> Vec<Option<ParameterValue>> {
    prepared
        .read_ids()
        .map(|id| {
            supplied
                .iter()
                .find(|(read, _)| read == id)
                .map(|(_, value)| value.clone())
        })
        .collect()
}
fn compile(f: &fixture::Fixture) -> CompiledRulePackage {
    CompiledRulePackage::compile(&f.rules, &f.schema, RuleLimits::default()).unwrap()
}
// Test-only composition; production graph scheduling invokes one effect.
fn evaluate_all(
    prepared: &PreparedRuleProgram,
    inputs: &[Option<ParameterValue>],
    scratch: &mut RuleScratch,
    max_work: usize,
) -> Result<Vec<EffectDisposition>, RuleError> {
    (0..prepared.program.effects.len())
        .map(|i| prepared.evaluate_effect_indexed(i, inputs, scratch, max_work))
        .collect()
}
fn empty_caches(s: &RuleScratch) {
    assert!(s.values.is_empty());
    assert!(s.facts.is_empty());
    assert!(s.stack.is_empty());
}
#[test]
fn prepared_and_public_paths_match_all_existing_component_cases() {
    let f = fixture::fixture();
    let compiled = compile(&f);
    let mut scratch = compiled.new_scratch();
    for case in &f.cases {
        let prepared = compiled
            .prepare_program(&case.owner, &case.program)
            .unwrap();
        let inputs = facts(&prepared, &case.facts);
        let supplied = case
            .facts
            .iter()
            .map(|(read, value)| RuleFact {
                read: read.clone(),
                value: value.clone(),
            })
            .collect::<Vec<_>>();
        let checked = compiled
            .evaluate(
                &case.owner,
                &case.program,
                &supplied,
                &f.schema,
                &mut scratch,
            )
            .unwrap();
        let expected = checked
            .effects
            .iter()
            .map(|e| e.disposition.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            evaluate_all(
                &prepared,
                &inputs,
                &mut scratch,
                RuleLimits::default().max_work
            )
            .unwrap(),
            expected,
            "{}",
            case.name
        );
        for (i, (actual, declared)) in checked.effects.iter().zip(&case.expected).enumerate() {
            assert_eq!(actual.id, declared.id);
            if let Some(value) = &declared.value {
                assert_eq!(
                    actual.disposition,
                    EffectDisposition::Applied {
                        value: value.clone()
                    }
                );
            } else {
                assert_eq!(actual.disposition, EffectDisposition::Inactive);
            }
            assert_eq!(
                prepared
                    .evaluate_effect_indexed(
                        i,
                        &inputs,
                        &mut scratch,
                        RuleLimits::default().max_work
                    )
                    .unwrap(),
                expected[i]
            );
            assert!(scratch.work_used() > 0);
        }
    }
}
#[test]
fn prepared_programs_share_immutable_storage_and_outlive_package() {
    let f = fixture::fixture();
    let compiled = compile(&f);
    let case = &f.cases[0];
    let prepared = compiled
        .prepare_program(&case.owner, &case.program)
        .unwrap();
    let again = compiled
        .prepare_program(&case.owner, &case.program)
        .unwrap();
    assert!(Arc::ptr_eq(&prepared.program, &again.program));
    assert!(
        compiled
            .prepare_program(&case.owner, &key("missing"))
            .is_err()
    );
    let inputs = facts(&prepared, &case.facts);
    drop(compiled);
    assert!(
        evaluate_all(
            &prepared,
            &inputs,
            &mut RuleScratch::default(),
            RuleLimits::default().max_work
        )
        .is_ok()
    );
}
#[test]
fn indexed_count_type_unit_schema_and_work_failures_clear_caches_and_allow_reuse() {
    let f = fixture::fixture();
    let compiled = compile(&f);
    let case = &f.cases[0];
    let prepared = compiled
        .prepare_program(&case.owner, &case.program)
        .unwrap();
    let inputs = facts(&prepared, &case.facts);
    let mut scratch = compiled.new_scratch();
    let cap = RuleLimits::default().max_work;
    let expected = evaluate_all(&prepared, &inputs, &mut scratch, cap).unwrap();
    let required_work = scratch.work_used();
    assert!(required_work > 1);
    for max_work in [0, 1, required_work - 1] {
        assert!(evaluate_all(&prepared, &inputs, &mut scratch, max_work).is_err());
        empty_caches(&scratch);
        assert!(scratch.work_used() <= max_work);
    }
    assert_eq!(
        evaluate_all(&prepared, &inputs, &mut scratch, required_work).unwrap(),
        expected
    );
    assert_eq!(scratch.work_used(), required_work);
    assert_eq!(
        evaluate_all(&prepared, &inputs, &mut scratch, usize::MAX).unwrap(),
        expected
    );
    assert!(scratch.work_used() <= cap);

    assert!(evaluate_all(&prepared, &inputs[..inputs.len() - 1], &mut scratch, cap).is_err());
    empty_caches(&scratch);
    assert!(
        prepared
            .evaluate_effect_indexed(usize::MAX, &inputs, &mut scratch, cap)
            .is_err()
    );
    empty_caches(&scratch);
    assert!(prepared.effect_read_indices(usize::MAX).is_err());
    let quality = prepared
        .read_ids()
        .position(|id| id.as_str() == "quality")
        .unwrap();
    let mut bad = inputs.clone();
    bad[quality] = Some(ParameterValue::Boolean(true));
    assert!(
        evaluate_all(&prepared, &bad, &mut scratch, cap)
            .unwrap_err()
            .message
            .contains("type/unit")
    );
    empty_caches(&scratch);
    let ParameterValue::Quantity(value) = inputs[quality].as_ref().unwrap() else {
        panic!()
    };
    bad[quality] = Some(ParameterValue::Quantity(
        FiniteQuantity::new(101.0, value.unit().clone()).unwrap(),
    ));
    assert!(
        evaluate_all(&prepared, &bad, &mut scratch, cap)
            .unwrap_err()
            .message
            .contains("schema/membership")
    );
    empty_caches(&scratch);
    let unit = DefId::parse(f.rules.namespace.clone(), "unit.damage").unwrap();
    bad[quality] = Some(ParameterValue::Quantity(
        FiniteQuantity::new(1.0, unit).unwrap(),
    ));
    assert!(
        evaluate_all(&prepared, &bad, &mut scratch, cap)
            .unwrap_err()
            .message
            .contains("type/unit")
    );
    empty_caches(&scratch);
    assert_eq!(
        evaluate_all(&prepared, &inputs, &mut scratch, cap).unwrap(),
        expected
    );
    // An inactive quality branch does not excuse a malformed present value.
    let has_quality = prepared
        .read_ids()
        .position(|id| id.as_str() == "has-quality")
        .unwrap();
    bad[has_quality] = Some(ParameterValue::Boolean(false));
    assert!(evaluate_all(&prepared, &bad, &mut scratch, cap).is_err());
    empty_caches(&scratch);
}
fn split_effect_fixture() -> fixture::Fixture {
    let mut f = fixture::fixture();
    let owner = &mut f.rules.owners[1]; // Modifier with Actor integer stat available.
    let program = &mut owner.programs.members[0];
    let RuleReadSource::Stat { stat, .. } = &program.reads[0].source else {
        panic!()
    };
    let stat = stat.clone();
    program.reads = ["right", "left", "gate"]
        .into_iter()
        .map(|name| RuleRead {
            id: key(name),
            value_type: ComputedValueType::Integer,
            source: RuleReadSource::CharacterLevel,
        })
        .collect();
    program.nodes = vec![
        RuleNode {
            id: key("constant"),
            expression: RuleExpression::Literal { value: integer(5) },
        },
        RuleNode {
            id: key("zero"),
            expression: RuleExpression::Literal { value: integer(0) },
        },
        RuleNode {
            id: key("condition"),
            expression: RuleExpression::Compare {
                operation: RuleComparison::Greater,
                left: key("gate"),
                right: key("zero"),
            },
        },
        RuleNode {
            id: key("selected"),
            expression: RuleExpression::Select {
                condition: key("condition"),
                when_true: key("left"),
                when_false: key("right"),
            },
        },
    ];
    for name in ["right", "left", "gate"] {
        program.nodes.push(RuleNode {
            id: key(name),
            expression: RuleExpression::Read { input: key(name) },
        });
    }
    program.effects = vec![
        RuleEffect {
            id: key("z-constant"),
            when: None,
            effect: RuleEffectKind::Contribute {
                entity: RuleEntity::Actor,
                stat: stat.clone(),
                contribution: ContributionKind::Add,
                value: key("constant"),
            },
        },
        RuleEffect {
            id: key("a-selected"),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Actor,
                stat,
                value: key("selected"),
            },
        },
    ];
    f
}
#[test]
fn static_effect_dependencies_are_separate_conservative_and_execution_is_lazy() {
    let f = split_effect_fixture();
    let compiled = compile(&f);
    let owner = &f.rules.owners[1];
    let prepared = compiled
        .prepare_program(&owner.owner, &owner.programs.members[0].id)
        .unwrap();
    assert_eq!(
        prepared
            .read_ids()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        ["gate", "left", "right"]
    );
    assert!(prepared.effect_read_indices(0).unwrap().is_empty());
    assert_eq!(prepared.effect_read_indices(1).unwrap(), [0, 1, 2]);
    let cap = RuleLimits::default().max_work;
    let mut scratch = compiled.new_scratch();
    assert_eq!(
        prepared
            .evaluate_effect_indexed(0, &[None, None, None], &mut scratch, cap)
            .unwrap(),
        EffectDisposition::Applied { value: integer(5) }
    );
    assert_eq!(
        prepared
            .evaluate_effect_indexed(
                1,
                &[Some(integer(0)), None, Some(integer(20))],
                &mut scratch,
                cap
            )
            .unwrap(),
        EffectDisposition::Applied { value: integer(20) }
    );
    assert_eq!(
        prepared
            .evaluate_effect_indexed(
                1,
                &[Some(integer(1)), Some(integer(11)), None],
                &mut scratch,
                cap
            )
            .unwrap(),
        EffectDisposition::Applied { value: integer(11) }
    );
    assert_eq!(
        prepared
            .evaluate_effect_indexed(
                1,
                &[Some(integer(1)), None, Some(integer(20))],
                &mut scratch,
                cap
            )
            .unwrap(),
        EffectDisposition::Unresolved { input: key("left") }
    );
    let checked = compiled
        .evaluate(
            &owner.owner,
            &owner.programs.members[0].id,
            &[],
            &f.schema,
            &mut scratch,
        )
        .unwrap();
    assert_eq!(checked.effects[0].id.as_str(), "z-constant");
    assert_eq!(checked.effects[1].id.as_str(), "a-selected");
}
#[test]
fn public_validation_errors_also_clear_active_scratch_caches() {
    let f = fixture::fixture();
    let compiled = compile(&f);
    let case = &f.cases[0];
    let supplied = case
        .facts
        .iter()
        .map(|(read, value)| RuleFact {
            read: read.clone(),
            value: value.clone(),
        })
        .collect::<Vec<_>>();
    let mut scratch = compiled.new_scratch();
    compiled
        .evaluate(
            &case.owner,
            &case.program,
            &supplied,
            &f.schema,
            &mut scratch,
        )
        .unwrap();
    let mut duplicate = supplied.clone();
    duplicate.push(supplied[0].clone());
    assert!(
        compiled
            .evaluate(
                &case.owner,
                &case.program,
                &duplicate,
                &f.schema,
                &mut scratch
            )
            .is_err()
    );
    empty_caches(&scratch);
    let mut bad = supplied;
    bad.last_mut().unwrap().value = ParameterValue::Boolean(true);
    assert!(
        compiled
            .evaluate(&case.owner, &case.program, &bad, &f.schema, &mut scratch)
            .is_err()
    );
    empty_caches(&scratch);
}
#[test]
fn static_dependency_expansion_is_charged_to_compile_budget() {
    let f = split_effect_fixture();
    let generous = RuleLimits::default();
    let mut low = 1;
    let mut high = generous.max_work;
    while low < high {
        let middle = low + (high - low) / 2;
        if CompiledRulePackage::compile(
            &f.rules,
            &f.schema,
            RuleLimits {
                max_work: middle,
                ..generous
            },
        )
        .is_ok()
        {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    assert!(low > 1);
    assert!(
        CompiledRulePackage::compile(
            &f.rules,
            &f.schema,
            RuleLimits {
                max_work: low - 1,
                ..generous
            }
        )
        .is_err()
    );
    let mut more = f.rules.clone();
    let program = &mut more.owners[1].programs.members[0];
    let mut repeated = program.effects[1].clone();
    repeated.id = key("another-selection");
    program.effects.push(repeated);
    assert!(
        CompiledRulePackage::compile(
            &more,
            &f.schema,
            RuleLimits {
                max_work: low,
                ..generous
            }
        )
        .is_err()
    );
}
#[test]
fn indexed_options_check_declared_membership_and_namespace_without_global_clamp() {
    let mut f = fixture::fixture();
    let namespace = f.rules.namespace.clone();
    let option = |name: &str| DefId::<OptionDefinition>::parse(namespace.clone(), name).unwrap();
    let target = DefId::<StatDefinition>::parse(namespace.clone(), "stat.option").unwrap();
    let mut input = f.schema.input().clone();
    for name in ["option.a", "option.b", "option.c"] {
        input
            .definitions
            .push(DefinitionDescriptor::Option(DefinitionEntry {
                id: option(name),
                schema: SchemaState::Known(OptionSchema {}),
            }));
    }
    input
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: target.clone(),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Option,
                targets: vec![RuleEntityKind::Action],
            }),
        }));
    let SchemaSubject::Definition(DefinitionAddress::Gem(gem)) = &f.rules.owners[2].owner else {
        panic!()
    };
    let parameter = DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(gem.clone()),
        slot: DefId::<ParameterSlotDefinition>::parse(namespace.clone(), "option.input").unwrap(),
    };
    for d in &mut input.definitions {
        if let DefinitionDescriptor::Gem(e) = d
            && &e.id == gem
        {
            let SchemaState::Known(s) = &mut e.schema else {
                panic!()
            };
            s.declarations.parameters.members.push(parameter.clone());
        }
    }
    input.slots.push(SlotDescriptor::Parameter(DefinitionEntry {
        id: parameter.clone(),
        schema: SchemaState::Known(ParameterSlotSchema {
            value: ValueSchema::Option {
                allowed: DeclaredSet::complete(vec![option("option.a"), option("option.b")]),
            },
            presence: SlotPresence::RequiredOnce,
            sites: vec![ParameterSite::GemParameter],
        }),
    }));
    f.schema = OwnedDefinitionSchemaPackage::new(input, OwnedSchemaLimits::default()).unwrap();
    f.rules.definitions = f.schema.identity().clone();
    let owner = &mut f.rules.owners[2];
    let program = &mut owner.programs.members[0];
    program.reads = vec![
        RuleRead {
            id: key("bounded"),
            value_type: ComputedValueType::Option,
            source: RuleReadSource::Parameter { slot: parameter },
        },
        RuleRead {
            id: key("stat"),
            value_type: ComputedValueType::Option,
            source: RuleReadSource::Stat {
                entity: RuleEntity::Current,
                stat: target.clone(),
            },
        },
    ];
    program.nodes = vec![RuleNode {
        id: key("value"),
        expression: RuleExpression::Read { input: key("stat") },
    }];
    program.effects = vec![RuleEffect {
        id: key("option"),
        when: None,
        effect: RuleEffectKind::Derive {
            entity: RuleEntity::Current,
            stat: target,
            value: key("value"),
        },
    }];
    let compiled = compile(&f);
    let owner = &f.rules.owners[2];
    let prepared = compiled
        .prepare_program(&owner.owner, &owner.programs.members[0].id)
        .unwrap();
    let cap = RuleLimits::default().max_work;
    let mut scratch = compiled.new_scratch();
    let bounded = Some(ParameterValue::Option(option("option.a")));
    for name in ["option.a", "option.b", "option.c"] {
        let value = ParameterValue::Option(option(name));
        assert_eq!(
            evaluate_all(
                &prepared,
                &[bounded.clone(), Some(value.clone())],
                &mut scratch,
                cap
            )
            .unwrap(),
            [EffectDisposition::Applied { value }]
        );
    }
    assert!(
        evaluate_all(
            &prepared,
            &[Some(ParameterValue::Option(option("option.c"))), None],
            &mut scratch,
            cap
        )
        .is_err()
    );
    empty_caches(&scratch);
    let foreign = DefId::parse(
        GameVersionNamespace::new("foreign", "v1").unwrap(),
        "option.a",
    )
    .unwrap();
    assert!(
        evaluate_all(
            &prepared,
            &[bounded, Some(ParameterValue::Option(foreign))],
            &mut scratch,
            cap
        )
        .is_err()
    );
    empty_caches(&scratch);
}
