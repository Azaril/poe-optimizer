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
fn slots() -> Vec<OrderedEquipmentSlot> {
    vec![
        OrderedEquipmentSlot {
            slot: id("left"),
            rank: 0,
        },
        OrderedEquipmentSlot {
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
        source: RuleReadSource::OrderedContributions {
            entity: RuleEntity::Current,
            query: key("query"),
            group: key("base"),
        },
    });
    let mut members = Vec::new();
    for (rank, name, origin) in [
        (0, "class", OrderedContributionOrigin::Character),
        (1, "passive", OrderedContributionOrigin::Allocation),
        (
            2,
            "item",
            OrderedContributionOrigin::EquipmentUse { slots: slots() },
        ),
        (
            3,
            "modifier",
            OrderedContributionOrigin::ItemModifier { slots: slots() },
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
        members.push(OrderedContributionMember {
            owner: subject(name),
            program: key("supply"),
            effect: key("add"),
            order: OrderedContributionOrder {
                source_rank: rank,
                program_rank: 0,
                effect_rank: 0,
                origin,
            },
        });
    }
    raw.ordered_contributions = Some(DeclaredSet::complete(vec![OrderedContributionQuery {
        id: key("query"),
        stat: id("final"),
        contribution: ContributionKind::Add,
        groups: vec![OrderedContributionGroup {
            id: key("base"),
            reduction: ContributionReduction::Sum,
            empty: value(0.0),
            members: DeclaredSet::complete(members),
        }],
    }]));
    raw
}
fn query(raw: &mut RulePackageInput) -> &mut OrderedContributionQuery {
    &mut raw.ordered_contributions.as_mut().unwrap().members[0]
}
fn group(raw: &mut RulePackageInput) -> &mut OrderedContributionGroup {
    &mut query(raw).groups[0]
}
fn check(raw: RulePackageInput, schema: &OwnedDefinitionSchemaPackage) -> OwnedRulePackage {
    validate_ordered_contributions(&raw, schema, RuleStorageLimits::default()).unwrap();
    OwnedRulePackage::new(raw, schema, RuleStorageLimits::default()).unwrap()
}
fn reject(raw: RulePackageInput, schema: &OwnedDefinitionSchemaPackage, message: &str) {
    for result in [
        validate_ordered_contributions(&raw, schema, RuleStorageLimits::default()).map(|_| ()),
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
    group(&mut changed).members.members[0].order.source_rank = 8;
    assert_ne!(check(changed, &schema).identity(), package.identity());
    let mut bad: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    bad["ordered_contributions"]["members"][0]["groups"][0]["members"]["members"][0]["order"]["discovery_index"] =
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
    raw.ordered_contributions = None;
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
    raw.ordered_contributions = Some(DeclaredSet::complete(vec![]));
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
                raw.ordered_contributions.as_mut().unwrap().members.push(q);
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
                group(&mut raw).members.members[0].program = key("missing");
                "ordered member references an unknown producer program"
            }
            4 => {
                group(&mut raw).members.members[0].effect = key("missing");
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
                group(&mut raw).members.members[0].order.origin =
                    OrderedContributionOrigin::Allocation;
                "ordered origin policy does not match its owner and program context"
            }
            1 => {
                raw.owners[1].programs.members[0].context = RuleEntityKind::EquipmentUse;
                "ordered origin policy does not match its owner and program context"
            }
            2 => {
                group(&mut raw).members.members[2].order.origin =
                    OrderedContributionOrigin::EquipmentUse { slots: vec![] };
                "ordered equipment policy needs explicit slot ranks"
            }
            3 => {
                let mut s = slots();
                s[1].rank = 0;
                group(&mut raw).members.members[2].order.origin =
                    OrderedContributionOrigin::EquipmentUse { slots: s };
                "duplicate, unknown or ambiguously ranked ordered equipment slot"
            }
            _ => {
                let mut s = slots();
                s[1].slot = id("missing");
                group(&mut raw).members.members[2].order.origin =
                    OrderedContributionOrigin::EquipmentUse { slots: s };
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
    member.program = key("second");
    member.order.program_rank = 1;
    member.order.source_rank = 99;
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
    member.program = key("second");
    member.order.program_rank = 1;
    member.order.source_rank = 99;
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
                group(&mut raw).empty = ParameterValue::Boolean(false);
                "ordered group needs a numeric identity"
            }
            1 => {
                group(&mut raw).empty = value(1.0);
                "ordered group reduction or identity differs from its contribution kind"
            }
            2 => {
                group(&mut raw).reduction = ContributionReduction::Product;
                "ordered group reduction or identity differs from its contribution kind"
            }
            3 => {
                group(&mut raw).empty =
                    ParameterValue::Quantity(FiniteQuantity::new(0.0, id("factor")).unwrap());
                "ordered group identity has the wrong numeric type or unit"
            }
            4 => {
                raw.owners[0].programs.members[0].reads[0].value_type = ComputedValueType::Integer;
                "ordered read has an unknown query/group or mismatched type"
            }
            5 => {
                raw.owners[0].programs.members[0].reads[0].source =
                    RuleReadSource::OrderedContributions {
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
                    RuleReadSource::OrderedContributions {
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
                matches!(validate_ordered_contributions(&raw, &schema, RuleStorageLimits::default()), Err(RuleStorageError::Structure(found)) if found == expected)
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
    raw.ordered_contributions.as_mut().unwrap().closure = SchemaClosure::Partial {
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
            .ordered_contributions
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
    q.groups.push(OrderedContributionGroup {
        id: key("empty"),
        reduction: ContributionReduction::Sum,
        empty: value(0.0),
        members: DeclaredSet::complete(vec![]),
    });
    raw.ordered_contributions.as_mut().unwrap().members.push(q);
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

/// Frozen pre-V21 wire shape, in its original struct field order. This comparison
/// catches an accidentally serialized None or newly added historical counter.
#[derive(serde::Serialize)]
struct HistoricalWire<'a> {
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
fn v6_through_v20_keep_exact_omitted_field_bytes_and_digest() {
    let schema = schema();
    for revision in 6..=20 {
        let mut raw = input(&schema);
        raw.operations_version = key(&format!("owned-domain-operations-v{revision}"));
        raw.ordered_contributions = None;
        raw.owners.clear();
        raw.receivers.members.clear();
        if revision < 15 {
            raw.effect_applications = None;
        }
        let old = HistoricalWire {
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
            "owned-rule-package-v2",
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
                .all(|k| !k.starts_with("ordered_"))
        );
        raw.ordered_contributions = Some(DeclaredSet::complete(vec![]));
        reject(
            raw,
            &schema,
            "ordered contributions require owned-domain-operations-v21",
        );
    }
}
