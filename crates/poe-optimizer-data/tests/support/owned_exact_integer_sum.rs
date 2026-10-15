//! Exact integer addition changes reduction semantics, never member authority.
use super::*;

fn integer(value: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(value).unwrap())
}

fn integer_schema() -> OwnedDefinitionSchemaPackage {
    let mut raw = schema().input().clone();
    for definition in &mut raw.definitions {
        if let DefinitionDescriptor::Stat(entry) = definition
            && entry.id == id("final")
        {
            entry.schema = SchemaState::Known(StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::Actor],
            });
        }
    }
    OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap()
}

fn exact_input(schema: &OwnedDefinitionSchemaPackage) -> RulePackageInput {
    let mut raw = input(schema);
    raw.operations_version = key(OWNED_RULE_OPERATIONS_V27);
    let group = group(&mut raw);
    group.reduction = ContributionReduction::ExactIntegerSum;
    group.ordering = ContributionOrdering::Unordered;
    group.empty = Some(integer(0));
    for member in &mut group.members.members {
        member.order = None;
    }
    for owner in &mut raw.owners {
        for program in &mut owner.programs.members {
            for read in &mut program.reads {
                read.value_type = ComputedValueType::Integer;
            }
            program.nodes[0].expression = RuleExpression::Literal { value: integer(3) };
        }
    }
    raw
}

#[test]
fn checked_exact_sum_roundtrips_with_explicit_members_and_no_semantic_ranks() {
    let schema = integer_schema();
    let raw = exact_input(&schema);
    let package = check(raw.clone(), &schema);
    let wire = encode_rule_package(&package, RuleStorageLimits::default()).unwrap();
    let restored = decode_rule_package(&wire, &schema, RuleStorageLimits::default()).unwrap();
    assert_eq!(restored.input(), &raw);
    assert_eq!(restored.identity(), package.identity());
    assert_eq!(restored.resources().ordered_members, 4);
    assert_eq!(restored.resources().ordered_slots, 4);
    assert!(
        group(&mut raw.clone())
            .members
            .members
            .iter()
            .all(|m| m.order.is_none())
    );

    let mut empty = raw.clone();
    empty.owners.truncate(1);
    group(&mut empty).members.members.clear();
    check(empty, &schema);

    // The existing ordered integer sum remains a separate, explicit contract.
    let mut ordered = raw;
    let old_ranks = group(&mut input(&schema)).members.members.clone();
    let group = group(&mut ordered);
    group.reduction = ContributionReduction::Sum;
    group.ordering = ContributionOrdering::Ordered;
    for (member, old) in group.members.members.iter_mut().zip(old_ranks) {
        member.order = old.order;
    }
    assert_ne!(check(ordered, &schema).identity(), package.identity());
}

#[test]
fn exact_sum_admits_only_integer_add_unordered_zero_and_no_ranks() {
    let schema = integer_schema();
    for mutation in 0..8 {
        let mut raw = exact_input(&schema);
        match mutation {
            0 => query(&mut raw).contribution = ContributionKind::Increase,
            1 => query(&mut raw).contribution = ContributionKind::Multiply,
            2 => query(&mut raw).contribution = ContributionKind::Flag,
            3 => query(&mut raw).contribution = ContributionKind::Override,
            4 => group(&mut raw).ordering = ContributionOrdering::Ordered,
            5 => group(&mut raw).empty = None,
            6 => group(&mut raw).empty = Some(integer(1)),
            _ => group(&mut raw).empty = Some(value(0.0)),
        }
        reject(
            raw,
            &schema,
            "exact integer sum requires Integer Add, unordered membership and integer zero",
        );
    }
    let quantity = super::schema();
    reject(
        exact_input(&quantity),
        &quantity,
        "exact integer sum requires Integer Add, unordered membership and integer zero",
    );
    let mut ranked = exact_input(&schema);
    group(&mut ranked).members.members[0].order = Some(ContributionOrder {
        source_rank: 0,
        program_rank: 0,
        effect_rank: 0,
        slot_ranks: vec![],
    });
    reject(
        ranked,
        &schema,
        "contribution member ordering differs from its group",
    );
    for reduction in [ContributionReduction::Sum, ContributionReduction::Product] {
        let mut raw = exact_input(&schema);
        group(&mut raw).reduction = reduction;
        reject(
            raw,
            &schema,
            "numeric contribution group requires semantic ordering",
        );
    }
}

#[test]
fn exact_sum_version_and_public_raw_read_bypass_are_checked_even_when_unused() {
    let schema = integer_schema();
    let mut raw = exact_input(&schema);
    raw.operations_version = key(OWNED_RULE_OPERATIONS_V26);
    // The query is deliberately unread and empty: validation still applies.
    raw.owners.clear();
    raw.receivers.members.clear();
    group(&mut raw).members.members.clear();
    reject(
        raw,
        &schema,
        "exact integer sum requires owned-domain-operations-v27",
    );

    for revision in [OWNED_RULE_OPERATIONS_V14, OWNED_RULE_OPERATIONS_V27] {
        let mut raw = exact_input(&schema);
        raw.operations_version = key(revision);
        if revision == OWNED_RULE_OPERATIONS_V14 {
            raw.effect_applications = None;
        }
        raw.contribution_queries = None;
        let read = &mut raw.owners[0].programs.members[0].reads[0];
        read.source = RuleReadSource::Contributions {
            entity: RuleEntity::Current,
            stat: id("final"),
            contribution: ContributionKind::Add,
            reduction: ContributionReduction::ExactIntegerSum,
            empty: integer(0),
        };
        reject(
            raw,
            &schema,
            "exact integer sum requires checked query membership",
        );
    }
}

#[test]
fn exact_sum_retains_source_permissions_duplicates_and_partial_coverage() {
    let schema = integer_schema();
    let mut raw = exact_input(&schema);
    let duplicate = group(&mut raw).members.members[0].clone();
    group(&mut raw).members.members.push(duplicate);
    reject(
        raw,
        &schema,
        "ordered producer occurs more than once across query groups",
    );

    let mut raw = exact_input(&schema);
    group(&mut raw).members.members[2]
        .producer
        .as_program_effect_mut()
        .unwrap()
        .origin = ContributionOrigin::EquipmentUse { slots: vec![] };
    reject(
        raw,
        &schema,
        "contribution equipment origin needs explicit slot membership",
    );

    let mut raw = exact_input(&schema);
    group(&mut raw).members.members[0]
        .producer
        .as_program_effect_mut()
        .unwrap()
        .effect = key("missing");
    reject(
        raw,
        &schema,
        "ordered member references an unknown or duplicate effect",
    );

    let mut raw = exact_input(&schema);
    group(&mut raw).members.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: subject("final"),
            facet: SchemaFacet::GameRules,
            code: key("untranslated-integer-supplier"),
        }],
    };
    raw.owners[1].programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: subject("class"),
            facet: SchemaFacet::GameRules,
            code: key("untranslated-class-program"),
        }],
    };
    let package = check(raw, &schema);
    assert!(
        !package
            .input()
            .contribution_queries
            .as_ref()
            .unwrap()
            .members[0]
            .groups[0]
            .members
            .is_complete()
    );
    assert!(!package.input().owners[1].programs.is_complete());
}

#[test]
fn exact_sum_membership_still_consumes_existing_independent_work_budgets() {
    let schema = integer_schema();
    let raw = exact_input(&schema);
    let resources = check(raw.clone(), &schema).resources();
    for limits in [
        RuleStorageLimits {
            max_ordered_members: resources.ordered_members - 1,
            ..Default::default()
        },
        RuleStorageLimits {
            max_ordered_slots: resources.ordered_slots - 1,
            ..Default::default()
        },
        RuleStorageLimits {
            max_ordered_work: resources.ordered_work - 1,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            OwnedRulePackage::new(raw.clone(), &schema, limits),
            Err(RuleStorageError::Limit(_))
        ));
    }
}
