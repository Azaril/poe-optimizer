use super::*;
use crate::owned_rules::test_fixture as fixture;
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn integer(value: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(value).unwrap())
}
fn flag_fixture() -> fixture::Fixture {
    let mut f = fixture::fixture();
    let mut schema = f.schema.input().clone();
    let stat = StatDefId::parse(schema.namespace.clone(), "stat.local-damage").unwrap();
    for definition in &mut schema.definitions {
        if let DefinitionDescriptor::Stat(entry) = definition
            && entry.id == stat
        {
            let SchemaState::Known(value) = &mut entry.schema else {
                unreachable!()
            };
            value.value = ComputedValueType::Boolean;
        }
    }
    f.schema = OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
    f.rules.definitions = f.schema.identity().clone();
    f.rules.operations_version = key(OWNED_RULE_OPERATIONS_V22);
    f.rules.effect_applications = Some(DeclaredSet::complete(vec![]));
    f.rules.existing_actor_rules = Some(DeclaredSet::complete(vec![]));
    f.rules.contribution_queries = Some(DeclaredSet::complete(vec![]));
    f.rules.owners.truncate(1);
    f.rules.owners[0].programs = DeclaredSet::complete(vec![RuleProgram {
        id: key("flag"),
        context: RuleEntityKind::EquipmentUse,
        reads: vec![],
        nodes: vec![RuleNode {
            id: key("value"),
            expression: RuleExpression::Literal {
                value: ParameterValue::Boolean(true),
            },
        }],
        effects: vec![RuleEffect {
            id: key("flag"),
            when: None,
            effect: RuleEffectKind::Contribute {
                entity: RuleEntity::Current,
                stat,
                contribution: ContributionKind::Flag,
                value: key("value"),
            },
        }],
    }]);
    f
}

#[test]
fn boolean_contribution_types_and_reduction_identity_are_exact() {
    let f = fixture::fixture();
    for stat in [ComputedValueType::Boolean, ComputedValueType::Integer] {
        for value in [ComputedValueType::Boolean, ComputedValueType::Integer] {
            for kind in [ContributionKind::Flag, ContributionKind::Add] {
                let accepted = contribution(kind, &stat, &value, &f.schema, "test").is_ok();
                assert_eq!(
                    accepted,
                    match kind {
                        ContributionKind::Flag =>
                            stat == ComputedValueType::Boolean
                                && value == ComputedValueType::Boolean,
                        ContributionKind::Add =>
                            stat == ComputedValueType::Integer
                                && value == ComputedValueType::Integer,
                        _ => unreachable!(),
                    }
                );
            }
        }
    }
    for reduction in [
        ContributionReduction::Any,
        ContributionReduction::Sum,
        ContributionReduction::Product,
    ] {
        for empty in [
            ParameterValue::Boolean(false),
            ParameterValue::Boolean(true),
            integer(0),
            integer(1),
        ] {
            assert_eq!(
                reduction_identity(
                    ContributionKind::Flag,
                    reduction,
                    &empty,
                    &ComputedValueType::Boolean,
                    "test"
                )
                .is_ok(),
                reduction == ContributionReduction::Any && empty == ParameterValue::Boolean(false)
            );
            assert!(
                reduction_identity(
                    ContributionKind::Flag,
                    reduction,
                    &empty,
                    &ComputedValueType::Integer,
                    "test"
                )
                .is_err()
            );
        }
    }
    for kind in [
        ContributionKind::Add,
        ContributionKind::Increase,
        ContributionKind::Multiply,
    ] {
        assert!(
            contribution(
                kind,
                &ComputedValueType::Boolean,
                &ComputedValueType::Boolean,
                &f.schema,
                "test"
            )
            .is_err()
        );
        assert!(
            reduction_identity(
                kind,
                ContributionReduction::Any,
                &integer(0),
                &ComputedValueType::Integer,
                "test"
            )
            .is_err()
        );
    }
}

#[test]
fn flag_effect_compiles_only_under_boolean_operations_and_boolean_stat() {
    let f = flag_fixture();
    compile(&f.rules, &f.schema, RuleLimits::default()).unwrap();
    let mut old = f.rules.clone();
    old.operations_version = key(OWNED_RULE_OPERATIONS_V21);
    assert!(compile(&old, &f.schema, RuleLimits::default()).is_err());
    let mut numeric = f.rules.clone();
    numeric.owners[0].programs.members[0].nodes[0].expression =
        RuleExpression::Literal { value: integer(1) };
    assert!(compile(&numeric, &f.schema, RuleLimits::default()).is_err());
}

#[test]
fn boolean_read_requires_query_proof_and_preserves_the_public_query() {
    let mut f = flag_fixture();
    let stat = StatDefId::parse(f.rules.namespace.clone(), "stat.local-damage").unwrap();
    f.rules.contribution_queries = Some(DeclaredSet::complete(vec![ContributionQuery {
        id: key("flags"),
        stat: stat.clone(),
        contribution: ContributionKind::Flag,
        groups: vec![ContributionGroup {
            id: key("all"),
            reduction: ContributionReduction::Any,
            ordering: ContributionOrdering::Unordered,
            empty: Some(ParameterValue::Boolean(false)),
            members: DeclaredSet::complete(vec![]),
        }],
    }]));
    let program = &mut f.rules.owners[0].programs.members[0];
    program.reads = vec![RuleRead {
        id: key("flags"),
        value_type: ComputedValueType::Boolean,
        source: RuleReadSource::ContributionQuery {
            entity: RuleEntity::Current,
            query: key("flags"),
            group: key("all"),
        },
    }];
    program.nodes[0].expression = RuleExpression::Read {
        input: key("flags"),
    };
    program.effects[0].effect = RuleEffectKind::Derive {
        entity: RuleEntity::Current,
        stat: stat.clone(),
        value: key("value"),
    };
    let compiled = compile(&f.rules, &f.schema, RuleLimits::default()).unwrap();
    assert!(matches!(
        compiled.input.owners[0].programs.members[0].reads[0].source,
        RuleReadSource::ContributionQuery { .. }
    ));
    let program = &mut f.rules.owners[0].programs.members[0];
    program.reads[0].source = RuleReadSource::Contributions {
        entity: RuleEntity::Current,
        stat,
        contribution: ContributionKind::Flag,
        reduction: ContributionReduction::Any,
        empty: ParameterValue::Boolean(false),
    };
    // Test the private admission too, independently of shared storage checks.
    let owner = &f.rules.owners[0];
    let ports = owner_ports(&owner.owner, &f.schema, "test").unwrap();
    let error = applications::read(
        &owner.programs.members[0].reads[0],
        &owner.programs.members[0],
        (&owner.owner, &ports),
        &f.schema,
        "test",
        RuleLimits::default(),
        &mut Budget::default(),
        None,
        ContributionReadAuthority::Direct,
    )
    .unwrap_err();
    assert!(error.message.contains("checked contribution query"));
    assert!(compile(&f.rules, &f.schema, RuleLimits::default()).is_err());
}
