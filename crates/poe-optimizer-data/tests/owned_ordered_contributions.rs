//! Definition-side laws. Concrete candidates, ordering and refusal run in Engine tests.
#[allow(dead_code)]
#[path = "support/owned_rule_receiver_fixture.rs"]
mod fixture;
use fixture::{id, key, value};
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_content::digest_owned, owned_definitions::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::{owned_rules::*, owned_schema::*};

fn empty_slots() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn subject(name: &str) -> SchemaSubject {
    SchemaSubject::Definition(match name {
        "class" => DefinitionAddress::Class(id(name)),
        "passive" => DefinitionAddress::PassiveNode(id(name)),
        "item" => DefinitionAddress::ItemTemplate(id(name)),
        "modifier" => DefinitionAddress::Modifier(id(name)),
        _ => DefinitionAddress::Stat(id(name)),
    })
}
fn slots() -> Vec<ContributionSlotRank> {
    vec![
        ContributionSlotRank {
            slot: id("left"),
            rank: 0,
        },
        ContributionSlotRank {
            slot: id("right"),
            rank: 1,
        },
    ]
}
fn schema() -> OwnedDefinitionSchemaPackage {
    let mut raw = fixture::schema_input();
    raw.definitions.extend([
        DefinitionDescriptor::Class(known(
            id("class"),
            ClassSchema {
                level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                },
                ascendancies: DeclaredSet::complete(vec![]),
                implicit_passives: DeclaredSet::complete(vec![]),
                declarations: empty_slots(),
            },
        )),
        DefinitionDescriptor::PassiveNode(known(
            id("passive"),
            PassiveNodeSchema {
                pools: DeclaredSet::complete(vec![]),
                adjacent: DeclaredSet::complete(vec![]),
                declarations: empty_slots(),
            },
        )),
        DefinitionDescriptor::ItemTemplate(known(
            id("item"),
            ItemTemplateSchema {
                item_level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                },
                equipment_slots: DeclaredSet::complete(vec![id("left"), id("right")]),
                socket_destinations: DeclaredSet::complete(vec![]),
                modifiers: DeclaredSet::complete(vec![id("modifier")]),
                quality: QualityUseSchema {
                    presence: QualityPresence::Optional,
                    allowed_kinds: DeclaredSet::complete(vec![]),
                },
                declarations: empty_slots(),
            },
        )),
        DefinitionDescriptor::Modifier(known(
            id("modifier"),
            ModifierSchema {
                declarations: empty_slots(),
            },
        )),
        DefinitionDescriptor::EquipmentSlot(known(
            id("left"),
            EquipmentSlotSchema {
                scope: ScopePolicy::Either,
            },
        )),
        DefinitionDescriptor::EquipmentSlot(known(
            id("right"),
            EquipmentSlotSchema {
                scope: ScopePolicy::Either,
            },
        )),
    ]);
    OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap()
}
fn input(schema: &OwnedDefinitionSchemaPackage) -> RulePackageInput {
    let mut raw = fixture::input(schema);
    raw.operations_version = key(OWNED_RULE_OPERATIONS_V21);
    raw.effect_applications = Some(DeclaredSet::complete(vec![]));
    raw.receivers.members.truncate(1);
    raw.owners[0].programs.members.truncate(1);
    raw.owners[0].programs.members[0].reads.push(RuleRead {
        id: key("sum"),
        value_type: ComputedValueType::Quantity { unit: id("points") },
        source: RuleReadSource::ContributionQuery {
            entity: RuleEntity::Current,
            query: key("query"),
            group: key("base"),
        },
    });
    let mut members = Vec::new();
    for (rank, name, origin) in [
        (0, "class", ContributionOrigin::Character),
        (1, "passive", ContributionOrigin::Allocation),
        (
            2,
            "item",
            ContributionOrigin::EquipmentUse {
                slots: slots().into_iter().map(|s| s.slot).collect(),
            },
        ),
        (
            3,
            "modifier",
            ContributionOrigin::ItemModifier {
                slots: slots().into_iter().map(|s| s.slot).collect(),
            },
        ),
    ] {
        let context = if rank < 2 {
            RuleEntityKind::Actor
        } else {
            RuleEntityKind::EquipmentUse
        };
        raw.owners.push(DefinitionRules {
            owner: subject(name),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key("supply"),
                context,
                reads: vec![],
                nodes: vec![RuleNode {
                    id: key("value"),
                    expression: RuleExpression::Literal {
                        value: value(f64::from(rank + 1)),
                    },
                }],
                effects: vec![RuleEffect {
                    id: key("add"),
                    when: None,
                    effect: RuleEffectKind::Contribute {
                        entity: RuleEntity::Player,
                        stat: id("final"),
                        contribution: ContributionKind::Add,
                        value: key("value"),
                    },
                }],
            }]),
        });
        members.push(ContributionMember {
            producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                owner: subject(name),
                program: key("supply"),
                effect: key("add"),
                origin,
            }),
            order: Some(ContributionOrder {
                source_rank: rank,
                program_rank: 0,
                effect_rank: 0,
                slot_ranks: if rank < 2 { vec![] } else { slots() },
            }),
        });
    }
    raw.contribution_queries = Some(DeclaredSet::complete(vec![ContributionQuery {
        id: key("query"),
        stat: id("final"),
        contribution: ContributionKind::Add,
        groups: vec![ContributionGroup {
            id: key("base"),
            reduction: ContributionReduction::Sum,
            ordering: ContributionOrdering::Ordered,
            empty: Some(value(0.0)),
            members: DeclaredSet::complete(members),
        }],
    }]));
    raw
}
fn query(raw: &mut RulePackageInput) -> &mut ContributionQuery {
    &mut raw.contribution_queries.as_mut().unwrap().members[0]
}
fn group(raw: &mut RulePackageInput) -> &mut ContributionGroup {
    &mut query(raw).groups[0]
}
fn check(raw: RulePackageInput, schema: &OwnedDefinitionSchemaPackage) -> OwnedRulePackage {
    validate_contribution_queries(&raw, schema, RuleStorageLimits::default()).unwrap();
    OwnedRulePackage::new(raw, schema, RuleStorageLimits::default()).unwrap()
}
fn reject(raw: RulePackageInput, schema: &OwnedDefinitionSchemaPackage, message: &str) {
    for result in [
        validate_contribution_queries(&raw, schema, RuleStorageLimits::default()).map(|_| ()),
        OwnedRulePackage::new(raw.clone(), schema, RuleStorageLimits::default()).map(|_| ()),
        decode_rule_package(
            &serde_json::to_vec(&raw).unwrap(),
            schema,
            RuleStorageLimits::default(),
        )
        .map(|_| ()),
    ] {
        assert!(
            matches!(result, Err(RuleStorageError::Structure(found)) if found == message),
            "expected {message}: {result:?}"
        );
    }
}

#[test]
fn exact_registry_roundtrip_preserves_authored_membership_and_semantic_ranks() {
    let schema = schema();
    let raw = input(&schema);
    let package = check(raw.clone(), &schema);
    let bytes = encode_rule_package(&package, RuleStorageLimits::default()).unwrap();
    let restored = decode_rule_package(&bytes, &schema, RuleStorageLimits::default()).unwrap();
    assert_eq!(restored.identity(), package.identity());
    assert_eq!(restored.input(), &raw);
    assert_eq!(package.resources().ordered_queries, 1);
    assert_eq!(package.resources().ordered_groups, 1);
    assert_eq!(package.resources().ordered_members, 4);
    assert_eq!(package.resources().ordered_slots, 4);
    let mut changed = raw.clone();
    group(&mut changed).members.members[0]
        .order
        .as_mut()
        .unwrap()
        .source_rank = 8;
    assert_ne!(check(changed, &schema).identity(), package.identity());
    let mut bad: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    bad["contribution_queries"]["members"][0]["groups"][0]["members"]["members"][0]["order"]["discovery_index"] =
        0.into();
    assert!(
        decode_rule_package(
            &serde_json::to_vec(&bad).unwrap(),
            &schema,
            RuleStorageLimits::default()
        )
        .is_err()
    );
}

#[test]
fn version_gate_requires_explicit_inventory_and_checks_unused_reads() {
    let schema = schema();
    let mut raw = input(&schema);
    raw.contribution_queries = None;
    reject(
        raw.clone(),
        &schema,
        "ordered contributions require an explicit V21 inventory",
    );
    raw.operations_version = key(OWNED_RULE_OPERATIONS_V20);
    reject(
        raw,
        &schema,
        "ordered contributions require owned-domain-operations-v21",
    );
    let mut raw = input(&schema);
    raw.operations_version = key(OWNED_RULE_OPERATIONS_V20);
    reject(
        raw,
        &schema,
        "ordered contributions require owned-domain-operations-v21",
    );
    let mut raw = input(&schema);
    raw.owners.clear();
    raw.receivers.members.clear();
    raw.contribution_queries = Some(DeclaredSet::complete(vec![]));
    check(raw, &schema);
}

#[test]
fn every_group_and_exact_producer_reference_is_validated_even_when_unread() {
    let schema = schema();
    for mutation in 0..8 {
        let mut raw = input(&schema);
        let expected = match mutation {
            0 => {
                let q = query(&mut raw).clone();
                raw.contribution_queries.as_mut().unwrap().members.push(q);
                "duplicate ordered query"
            }
            1 => {
                let g = group(&mut raw).clone();
                query(&mut raw).groups.push(g);
                "duplicate ordered group"
            }
            2 => {
                let mut g = group(&mut raw).clone();
                g.id = key("unread");
                query(&mut raw).groups.push(g);
                "ordered producer occurs more than once across query groups"
            }
            3 => {
                group(&mut raw).members.members[0]
                    .producer
                    .as_program_effect_mut()
                    .unwrap()
                    .program = key("missing");
                "ordered member references an unknown producer program"
            }
            4 => {
                group(&mut raw).members.members[0]
                    .producer
                    .as_program_effect_mut()
                    .unwrap()
                    .effect = key("missing");
                "ordered member references an unknown or duplicate effect"
            }
            5 => {
                raw.owners[1].programs.members[0].effects[0].effect = RuleEffectKind::Derive {
                    entity: RuleEntity::Player,
                    stat: id("final"),
                    value: key("value"),
                };
                "ordered member must select a contribution effect"
            }
            6 => {
                query(&mut raw).contribution = ContributionKind::Increase;
                "ordered member contribution channel differs from query"
            }
            _ => {
                query(&mut raw).stat = id("missing");
                "ordered query stat must be known"
            }
        };
        reject(raw, &schema, expected);
    }
}

#[test]
fn explicit_origin_context_and_slot_policies_cannot_be_replaced_by_ids() {
    let schema = schema();
    for mutation in 0..5 {
        let mut raw = input(&schema);
        let expected = match mutation {
            0 => {
                group(&mut raw).members.members[0]
                    .producer
                    .as_program_effect_mut()
                    .unwrap()
                    .origin = ContributionOrigin::Allocation;
                "ordered origin policy does not match its owner and program context"
            }
            1 => {
                raw.owners[1].programs.members[0].context = RuleEntityKind::EquipmentUse;
                "ordered origin policy does not match its owner and program context"
            }
            2 => {
                group(&mut raw).members.members[2]
                    .producer
                    .as_program_effect_mut()
                    .unwrap()
                    .origin = ContributionOrigin::EquipmentUse { slots: vec![] };
                "contribution equipment origin needs explicit slot membership"
            }
            3 => {
                let mut s = slots();
                s[1].rank = 0;
                group(&mut raw).members.members[2]
                    .order
                    .as_mut()
                    .unwrap()
                    .slot_ranks = s;
                "duplicate, unknown or ambiguously ranked ordered equipment slot"
            }
            _ => {
                let mut s = slots();
                s[1].slot = id("missing");
                group(&mut raw).members.members[2]
                    .order
                    .as_mut()
                    .unwrap()
                    .slot_ranks = s;
                "duplicate, unknown or ambiguously ranked ordered equipment slot"
            }
        };
        reject(raw, &schema, expected);
    }
    let mut raw = input(&schema);
    let mut second = raw.owners[4].programs.members[0].clone();
    second.id = key("second");
    raw.owners[4].programs.members.push(second);
    let mut member = group(&mut raw).members.members[3].clone();
    member.producer.as_program_effect_mut().unwrap().program = key("second");
    member.order.as_mut().unwrap().program_rank = 1;
    member.order.as_mut().unwrap().source_rank = 99;
    group(&mut raw).members.members.push(member);
    reject(
        raw,
        &schema,
        "ordered equipment origin lane must share source and slot ranks",
    );
    let mut raw = input(&schema);
    let mut second = raw.owners[1].programs.members[0].clone();
    second.id = key("second");
    raw.owners[1].programs.members.push(second);
    let mut member = group(&mut raw).members.members[0].clone();
    member.producer.as_program_effect_mut().unwrap().program = key("second");
    member.order.as_mut().unwrap().program_rank = 1;
    member.order.as_mut().unwrap().source_rank = 99;
    group(&mut raw).members.members.push(member);
    reject(
        raw,
        &schema,
        "ordered programs of one owner must share source rank",
    );
}

#[test]
fn numeric_identities_units_and_relative_recipient_authority_are_explicit() {
    let schema = schema();
    for mutation in 0..8 {
        let mut raw = input(&schema);
        let expected = match mutation {
            0 => {
                group(&mut raw).empty = Some(ParameterValue::Boolean(false));
                "ordered group needs a numeric identity"
            }
            1 => {
                group(&mut raw).empty = Some(value(1.0));
                "ordered group reduction or identity differs from its contribution kind"
            }
            2 => {
                group(&mut raw).reduction = ContributionReduction::Product;
                "ordered group reduction or identity differs from its contribution kind"
            }
            3 => {
                group(&mut raw).empty = Some(ParameterValue::Quantity(
                    FiniteQuantity::new(0.0, id("factor")).unwrap(),
                ));
                "ordered group identity has the wrong numeric type or unit"
            }
            4 => {
                raw.owners[0].programs.members[0].reads[0].value_type = ComputedValueType::Integer;
                "ordered read has an unknown query/group or mismatched type"
            }
            5 => {
                raw.owners[0].programs.members[0].reads[0].source =
                    RuleReadSource::ContributionQuery {
                        entity: RuleEntity::PropertyOwner,
                        query: key("query"),
                        group: key("base"),
                    };
                "ordered read has an unsupported relative recipient scope"
            }
            6 => {
                raw.owners[0].programs.members[0].context = RuleEntityKind::EquipmentUse;
                "ordered read recipient is not admitted by its stat"
            }
            _ => {
                raw.owners[0].programs.members[0].reads[0].source =
                    RuleReadSource::ContributionQuery {
                        entity: RuleEntity::Player,
                        query: key("query"),
                        group: key("missing"),
                    };
                "ordered read has an unknown query/group or mismatched type"
            }
        };
        // Changing the receiver context is independently rejected by receiver
        // admission first; call the reusable ordered validator for that case.
        if mutation == 6 {
            assert!(
                matches!(validate_contribution_queries(&raw, &schema, RuleStorageLimits::default()), Err(RuleStorageError::Structure(found)) if found == expected)
            );
        } else {
            reject(raw, &schema, expected);
        }
    }
}

#[test]
fn partial_inventories_retain_evidence_and_never_close_existing_program_owners() {
    let schema = schema();
    let mut raw = input(&schema);
    let gap = SchemaGap {
        subject: subject("final"),
        facet: SchemaFacet::GameRules,
        code: key("unproved"),
    };
    raw.contribution_queries.as_mut().unwrap().closure = SchemaClosure::Partial {
        gaps: vec![gap.clone()],
    };
    group(&mut raw).members.closure = SchemaClosure::Partial { gaps: vec![gap] };
    raw.owners[1].programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: subject("class"),
            facet: SchemaFacet::GameRules,
            code: key("other-effects"),
        }],
    };
    let package = check(raw.clone(), &schema);
    assert_eq!(package.input(), &raw);
    assert!(
        !package
            .input()
            .contribution_queries
            .as_ref()
            .unwrap()
            .is_complete()
    );
    assert!(!package.input().owners[1].programs.is_complete());
    group(&mut raw).members.closure = SchemaClosure::Partial { gaps: vec![] };
    reject(raw, &schema, "partial ordered inventory needs gap evidence");
    let mut raw = input(&schema);
    group(&mut raw).members.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: subject("class"),
            facet: SchemaFacet::GameRules,
            code: key("wrong-channel"),
        }],
    };
    reject(raw, &schema, "invalid ordered inventory gap");
}

#[test]
fn bounded_registry_counts_are_checked_independently_and_replay_exactly() {
    let schema = schema();
    let mut raw = input(&schema);
    let mut q = query(&mut raw).clone();
    q.id = key("other-query");
    q.groups.push(ContributionGroup {
        id: key("empty"),
        reduction: ContributionReduction::Sum,
        ordering: ContributionOrdering::Ordered,
        empty: Some(value(0.0)),
        members: DeclaredSet::complete(vec![]),
    });
    raw.contribution_queries.as_mut().unwrap().members.push(q);
    let use_ = check(raw.clone(), &schema).resources();
    let exact = RuleStorageLimits {
        max_ordered_queries: use_.ordered_queries,
        max_ordered_groups: use_.ordered_groups,
        max_ordered_members: use_.ordered_members,
        max_ordered_slots: use_.ordered_slots,
        max_ordered_work: use_.ordered_work,
        ..Default::default()
    };
    OwnedRulePackage::new(raw.clone(), &schema, exact).unwrap();
    for (limits, expected) in [
        (
            RuleStorageLimits {
                max_ordered_queries: 1,
                ..exact
            },
            "ordered queries",
        ),
        (
            RuleStorageLimits {
                max_ordered_groups: 1,
                ..exact
            },
            "ordered groups",
        ),
        (
            RuleStorageLimits {
                max_ordered_members: 1,
                ..exact
            },
            "ordered members",
        ),
        (
            RuleStorageLimits {
                max_ordered_slots: 1,
                ..exact
            },
            "ordered slots",
        ),
        (
            RuleStorageLimits {
                max_ordered_work: 1,
                ..exact
            },
            "ordered work",
        ),
    ] {
        assert!(
            matches!(OwnedRulePackage::new(raw.clone(), &schema, limits), Err(RuleStorageError::Limit(found)) if found == expected)
        );
    }
}

/// Current wire shape for operation subsets without contribution queries.
/// This checks omitted optional fields, not compatibility with retired schemas.
#[derive(serde::Serialize)]
struct OmittedQueryWire<'a> {
    schema_version: u32,
    namespace: &'a GameVersionNamespace,
    release: &'a OwnedDefinitionKey,
    semantics_version: &'a OwnedDefinitionKey,
    operations_version: &'a OwnedDefinitionKey,
    definitions: &'a poe_optimizer_core::data::DataIdentity,
    tables: &'a [IntegerRuleTable],
    owners: &'a [DefinitionRules],
    receivers: &'a DeclaredSet<StatReceiver>,
    #[serde(skip_serializing_if = "Option::is_none")]
    effect_applications: &'a Option<DeclaredSet<EffectApplicationRule>>,
}
#[test]
fn earlier_operation_subsets_use_current_wire_domain_with_omitted_query_fields() {
    let schema = schema();
    for revision in 6..=20 {
        let mut raw = input(&schema);
        raw.operations_version = key(&format!("owned-domain-operations-v{revision}"));
        raw.contribution_queries = None;
        raw.owners.clear();
        raw.receivers.members.clear();
        if revision < 15 {
            raw.effect_applications = None;
        }
        let old = OmittedQueryWire {
            schema_version: raw.schema_version,
            namespace: &raw.namespace,
            release: &raw.release,
            semantics_version: &raw.semantics_version,
            operations_version: &raw.operations_version,
            definitions: &raw.definitions,
            tables: &raw.tables,
            owners: &raw.owners,
            receivers: &raw.receivers,
            effect_applications: &raw.effect_applications,
        };
        let old_bytes = serde_json::to_vec(&old).unwrap();
        let old_digest = digest_owned(
            "owned-rule-package-v3",
            &old,
            RuleStorageLimits::default().max_wire_bytes,
        )
        .unwrap();
        let package = check(raw.clone(), &schema);
        assert_eq!(
            encode_rule_package(&package, RuleStorageLimits::default()).unwrap(),
            old_bytes,
            "v{revision}"
        );
        assert_eq!(*package.identity(), old_digest, "v{revision}");
        let usage = serde_json::to_value(package.resources()).unwrap();
        assert!(
            usage
                .as_object()
                .unwrap()
                .keys()
                .all(|k| !k.starts_with("ordered_") || k == "ordered_work")
        );
        raw.contribution_queries = Some(DeclaredSet::complete(vec![]));
        reject(
            raw,
            &schema,
            "ordered contributions require owned-domain-operations-v21",
        );
    }
}

fn boolean_schema(targets: Vec<RuleEntityKind>) -> OwnedDefinitionSchemaPackage {
    let mut raw = schema().input().clone();
    for definition in &mut raw.definitions {
        if let DefinitionDescriptor::Stat(entry) = definition
            && entry.id == id("final")
        {
            entry.schema = SchemaState::Known(StatSchema {
                value: ComputedValueType::Boolean,
                targets: targets.clone(),
            });
        }
    }
    OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap()
}
fn boolean_input(schema: &OwnedDefinitionSchemaPackage) -> RulePackageInput {
    let mut raw = input(schema);
    raw.operations_version = key(OWNED_RULE_OPERATIONS_V22);
    query(&mut raw).contribution = ContributionKind::Flag;
    let group = group(&mut raw);
    group.reduction = ContributionReduction::Any;
    group.ordering = ContributionOrdering::Unordered;
    group.empty = Some(ParameterValue::Boolean(false));
    for member in &mut group.members.members {
        member.order = None;
    }
    raw.owners[0].programs.members[0].reads[0].value_type = ComputedValueType::Boolean;
    for owner in &mut raw.owners {
        for program in &mut owner.programs.members {
            program.nodes[0].expression = RuleExpression::Literal {
                value: ParameterValue::Boolean(true),
            };
            for effect in &mut program.effects {
                if let RuleEffectKind::Contribute { contribution, .. } = &mut effect.effect {
                    *contribution = ContributionKind::Flag;
                }
            }
        }
    }
    raw
}

#[test]
fn boolean_membership_roundtrips_without_invented_order_and_counts_slots() {
    let schema = boolean_schema(vec![RuleEntityKind::Actor]);
    let raw = boolean_input(&schema);
    let package = check(raw.clone(), &schema);
    let encoded = encode_rule_package(&package, RuleStorageLimits::default()).unwrap();
    let restored = decode_rule_package(&encoded, &schema, RuleStorageLimits::default()).unwrap();
    assert_eq!(restored.input(), &raw);
    assert_eq!(restored.identity(), package.identity());
    assert_eq!(restored.resources().ordered_slots, 4);
    assert!(
        query(&mut raw.clone()).groups[0]
            .members
            .members
            .iter()
            .all(|m| m.order.is_none())
    );
    let mut empty = raw;
    empty.owners.truncate(1);
    group(&mut empty).members.members.clear();
    check(empty, &schema);
}

#[test]
fn boolean_type_identity_reduction_and_order_are_a_single_contract() {
    let schema = boolean_schema(vec![RuleEntityKind::Actor]);
    for mutation in 0..4 {
        let mut raw = boolean_input(&schema);
        match mutation {
            0 => group(&mut raw).empty = Some(ParameterValue::Boolean(true)),
            1 => group(&mut raw).empty = Some(value(0.0)),
            2 => group(&mut raw).reduction = ContributionReduction::Sum,
            _ => group(&mut raw).ordering = ContributionOrdering::Ordered,
        }
        reject(
            raw,
            &schema,
            "flag group requires Boolean stat, false identity and unordered Any",
        );
    }
    let numeric_schema = self::schema();
    let mut numeric = input(&numeric_schema);
    numeric.operations_version = key(OWNED_RULE_OPERATIONS_V22);
    query(&mut numeric).contribution = ContributionKind::Flag;
    reject(
        numeric,
        &numeric_schema,
        "flag group requires Boolean stat, false identity and unordered Any",
    );
    let mut unordered_numeric = input(&numeric_schema);
    group(&mut unordered_numeric).ordering = ContributionOrdering::Unordered;
    reject(
        unordered_numeric,
        &numeric_schema,
        "numeric contribution group requires semantic ordering",
    );
}

#[test]
fn boolean_version_gate_checks_unread_producers_and_queries() {
    let schema = boolean_schema(vec![RuleEntityKind::Actor]);
    let mut raw = boolean_input(&schema);
    raw.operations_version = key(OWNED_RULE_OPERATIONS_V21);
    reject(
        raw.clone(),
        &schema,
        "Boolean contributions require owned-domain-operations-v22",
    );
    raw.owners.truncate(1);
    group(&mut raw).members.members.clear();
    reject(
        raw.clone(),
        &schema,
        "Boolean contributions require owned-domain-operations-v22",
    );
    raw.operations_version = key(OWNED_RULE_OPERATIONS_V22);
    check(raw, &schema);
}

#[test]
fn public_direct_boolean_reads_cannot_bypass_member_proof() {
    let schema = boolean_schema(vec![RuleEntityKind::Actor]);
    for mutation in 0..3 {
        let mut raw = boolean_input(&schema);
        let read = &mut raw.owners[0].programs.members[0].reads[0];
        read.value_type = if mutation == 2 {
            ComputedValueType::Boolean
        } else {
            ComputedValueType::Integer
        };
        read.source = RuleReadSource::Contributions {
            entity: RuleEntity::Player,
            stat: id("final"),
            contribution: if mutation == 0 {
                ContributionKind::Flag
            } else {
                ContributionKind::Add
            },
            reduction: if mutation == 1 {
                ContributionReduction::Any
            } else {
                ContributionReduction::Sum
            },
            empty: ParameterValue::Boolean(false),
        };
        reject(
            raw,
            &schema,
            "Boolean contributions require checked query membership",
        );
    }
}

#[test]
fn membership_and_rank_validation_stay_independent() {
    let schema = boolean_schema(vec![RuleEntityKind::Actor]);
    let mut raw = boolean_input(&schema);
    group(&mut raw).members.members[0].order = Some(ContributionOrder {
        source_rank: 0,
        program_rank: 0,
        effect_rank: 0,
        slot_ranks: vec![],
    });
    reject(
        raw,
        &schema,
        "contribution member ordering differs from its group",
    );
    for mutation in 0..3 {
        let mut raw = boolean_input(&schema);
        let member = &mut group(&mut raw).members.members[2];
        member.producer.as_program_effect_mut().unwrap().origin =
            ContributionOrigin::EquipmentUse {
                slots: match mutation {
                    0 => vec![],
                    1 => vec![id("left"), id("left")],
                    _ => vec![id("missing")],
                },
            };
        reject(
            raw,
            &schema,
            if mutation == 0 {
                "contribution equipment origin needs explicit slot membership"
            } else {
                "duplicate or unknown contribution equipment slot"
            },
        );
    }
    let numeric_schema = self::schema();
    for mutation in 0..3 {
        let mut raw = input(&numeric_schema);
        let expected = match mutation {
            0 => {
                group(&mut raw).members.members[0].order = None;
                "contribution member ordering differs from its group"
            }
            1 => {
                group(&mut raw).members.members[0]
                    .order
                    .as_mut()
                    .unwrap()
                    .slot_ranks = slots();
                "non-equipment contribution order forbids slot ranks"
            }
            _ => {
                group(&mut raw).members.members[2]
                    .order
                    .as_mut()
                    .unwrap()
                    .slot_ranks
                    .pop();
                "ordered slot ranks must exactly cover origin membership"
            }
        };
        reject(raw, &numeric_schema, expected);
    }
}

#[test]
fn boolean_queries_retain_exact_member_and_partial_inventory_evidence() {
    let schema = boolean_schema(vec![RuleEntityKind::Actor]);
    let mut raw = boolean_input(&schema);
    let duplicate = group(&mut raw).members.members[0].clone();
    group(&mut raw).members.members.push(duplicate);
    reject(
        raw,
        &schema,
        "ordered producer occurs more than once across query groups",
    );
    let mut raw = boolean_input(&schema);
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
    let mut raw = boolean_input(&schema);
    group(&mut raw).members.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: subject("final"),
            facet: SchemaFacet::GameRules,
            code: key("unknown-producers"),
        }],
    };
    assert!(
        !check(raw, &schema)
            .input()
            .contribution_queries
            .as_ref()
            .unwrap()
            .members[0]
            .groups[0]
            .members
            .is_complete()
    );
}

#[test]
fn enemy_boolean_recipient_authority_is_explicit_and_versioned() {
    let schema = boolean_schema(vec![RuleEntityKind::Actor, RuleEntityKind::Enemy]);
    let mut raw = boolean_input(&schema);
    raw.receivers.members.clear();
    let consumer = &mut raw.owners[0].programs.members[0];
    consumer.context = RuleEntityKind::Enemy;
    consumer.reads[0].source = RuleReadSource::ContributionQuery {
        entity: RuleEntity::Current,
        query: key("query"),
        group: key("base"),
    };
    for owner in raw.owners.iter_mut().skip(1) {
        if let RuleEffectKind::Contribute { entity, .. } =
            &mut owner.programs.members[0].effects[0].effect
        {
            *entity = RuleEntity::Enemy;
        }
    }
    check(raw.clone(), &schema);
    raw.owners[0].programs.members[0].reads[0].source = RuleReadSource::ContributionQuery {
        entity: RuleEntity::Enemy,
        query: key("query"),
        group: key("base"),
    };
    check(raw.clone(), &schema);
    let actor_only = boolean_schema(vec![RuleEntityKind::Actor]);
    raw.definitions = actor_only.identity().clone();
    reject(
        raw,
        &actor_only,
        "ordered producer recipient is not admitted by its stat",
    );
}

#[test]
fn unordered_membership_preserves_independent_resource_budgets() {
    let schema = boolean_schema(vec![RuleEntityKind::Actor]);
    let raw = boolean_input(&schema);
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

#[test]
fn raw_query_validation_bounds_program_scans_before_visiting_unused_reads() {
    let schema = boolean_schema(vec![RuleEntityKind::Actor]);
    for include_registry in [false, true] {
        let mut raw = boolean_input(&schema);
        if !include_registry {
            raw.operations_version = key(OWNED_RULE_OPERATIONS_V14);
            raw.contribution_queries = None;
        }
        let limits = RuleStorageLimits {
            max_ordered_work: 1,
            ..Default::default()
        };
        assert!(matches!(
            validate_contribution_queries(&raw, &schema, limits),
            Err(RuleStorageError::Limit("ordered work"))
        ));
    }
}

#[test]
fn v22_does_not_expand_numeric_query_enemy_authority() {
    let mut schema_input = schema().input().clone();
    for definition in &mut schema_input.definitions {
        if let DefinitionDescriptor::Stat(entry) = definition
            && entry.id == id("final")
            && let SchemaState::Known(stat) = &mut entry.schema
        {
            stat.targets.push(RuleEntityKind::Enemy);
        }
    }
    let schema =
        OwnedDefinitionSchemaPackage::new(schema_input, OwnedSchemaLimits::default()).unwrap();
    let mut raw = input(&schema);
    raw.operations_version = key(OWNED_RULE_OPERATIONS_V22);
    raw.owners[0].programs.members[0].reads[0].source = RuleReadSource::ContributionQuery {
        entity: RuleEntity::Enemy,
        query: key("query"),
        group: key("base"),
    };
    reject(
        raw,
        &schema,
        "ordered read has an unsupported relative recipient scope",
    );
    let mut raw = input(&schema);
    raw.operations_version = key(OWNED_RULE_OPERATIONS_V22);
    if let RuleEffectKind::Contribute { entity, .. } =
        &mut raw.owners[1].programs.members[0].effects[0].effect
    {
        *entity = RuleEntity::Enemy;
    }
    reject(
        raw,
        &schema,
        "ordered producer has unsupported recipient scope",
    );
}
