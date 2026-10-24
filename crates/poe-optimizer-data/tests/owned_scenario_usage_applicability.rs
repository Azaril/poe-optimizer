//! Finite Scenario usage permissions retain exact supply and unknown coverage.
use poe_optimizer_core::{owned_build::DeclaredSlot, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
use serde_json::{Value, json};

fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("usage-test", "v1").unwrap()
}
fn id<K: DefinitionDomain>(name: &str) -> DefId<K> {
    DefId::parse(ns(), name).unwrap()
}
fn key(name: &str) -> OwnedDefinitionKey {
    name.parse().unwrap()
}
fn slot<K: DefinitionDomain>(owner: SlotOwnerDefId, name: &str) -> DeclaredSlot<DefId<K>> {
    DeclaredSlot {
        declaration: owner,
        slot: id(name),
    }
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn declarations() -> DeclaredSlots {
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
fn range() -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(1).unwrap(),
        maximum: BoundedInteger::new(100).unwrap(),
    }
}
fn quality() -> QualityUseSchema {
    QualityUseSchema {
        presence: QualityPresence::Optional,
        allowed_kinds: DeclaredSet::complete(vec![]),
    }
}
fn owners() -> Vec<SlotOwnerDefId> {
    vec![
        SlotOwnerDefId::ItemTemplate(id("item")),
        SlotOwnerDefId::PassiveNode(id("node")),
        SlotOwnerDefId::Gem(id("support")),
    ]
}
fn supply(owner: SlotOwnerDefId) -> UsageSkillSupply {
    UsageSkillSupply {
        skill: id("effect"),
        grant: slot(owner.clone(), "grant"),
        supply: slot(owner, "supply"),
    }
}
fn gap(subject: SchemaSubject) -> SchemaGap {
    SchemaGap {
        subject,
        facet: SchemaFacet::StaticLinks,
        code: key("not-yet-known"),
    }
}
fn input() -> SchemaPackageInput {
    let mut descriptors = vec![
        DefinitionDescriptor::Skill(known(
            id("effect"),
            SkillSchema {
                directly_selectable: true,
                declarations: declarations(),
            },
        )),
        DefinitionDescriptor::Skill(known(
            id("other"),
            SkillSchema {
                directly_selectable: false,
                declarations: declarations(),
            },
        )),
    ];
    let mut slots = vec![];
    let mut recipients = vec![UsageSkillApplicability::AuthoredDirect {
        skill: id("effect"),
    }];
    for owner in owners() {
        let relation = supply(owner.clone());
        let mut declarations = declarations();
        declarations.grants.members.push(relation.grant.clone());
        declarations
            .skill_grants
            .members
            .push(relation.supply.clone());
        let descriptor = match &owner {
            SlotOwnerDefId::ItemTemplate(id) => DefinitionDescriptor::ItemTemplate(known(
                id.clone(),
                ItemTemplateSchema {
                    item_level: range(),
                    equipment_slots: DeclaredSet::complete(vec![]),
                    socket_destinations: DeclaredSet::complete(vec![]),
                    modifiers: DeclaredSet::complete(vec![]),
                    quality: quality(),
                    declarations,
                },
            )),
            SlotOwnerDefId::PassiveNode(id) => DefinitionDescriptor::PassiveNode(known(
                id.clone(),
                PassiveNodeSchema {
                    pools: DeclaredSet::complete(vec![]),
                    adjacent: DeclaredSet::complete(vec![]),
                    declarations,
                },
            )),
            SlotOwnerDefId::Gem(id) => DefinitionDescriptor::Gem(known(
                id.clone(),
                GemSchema {
                    level: range(),
                    roles: vec![AuthoredGemRole::SupportAssignment],
                    skills: DeclaredSet::complete(vec![relation.skill.clone()]),
                    quality: quality(),
                    declarations,
                },
            )),
            _ => unreachable!(),
        };
        descriptors.push(descriptor);
        let provider_role = match owner {
            SlotOwnerDefId::ItemTemplate(_) => ProviderRole::EquipmentUse,
            SlotOwnerDefId::PassiveNode(_) => ProviderRole::Allocation,
            SlotOwnerDefId::Gem(_) => ProviderRole::SupportAssignment,
            _ => unreachable!(),
        };
        slots.extend([
            SlotDescriptor::Grant(known(
                relation.grant.clone(),
                GrantSlotSchema {
                    provider_roles: vec![provider_role],
                    target: GrantTarget::Skill(relation.supply.clone()),
                },
            )),
            SlotDescriptor::SkillGrant(known(
                relation.supply.clone(),
                SkillGrantSlotSchema {
                    skill: relation.skill.clone(),
                    outputs: DeclaredSet::complete(vec![]),
                    preset_inputs: None,
                },
            )),
        ]);
        recipients.push(UsageSkillApplicability::Supplied(Box::new(relation)));
    }
    descriptors.push(DefinitionDescriptor::UsagePolicy(known(
        id("stages"),
        UsagePolicySchema {
            targets: vec![UsageTargetKind::Skill],
            declarations: declarations(),
            scenario_skill_applicability: Some(DeclaredSet::complete(recipients)),
        },
    )));
    SchemaPackageInput {
        schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
        namespace: ns(),
        release: key("release"),
        semantics_version: key("semantics"),
        definitions: descriptors,
        slots,
    }
}
fn policy(input: &mut SchemaPackageInput) -> &mut UsagePolicySchema {
    input
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::UsagePolicy(DefinitionEntry {
                schema: SchemaState::Known(schema),
                ..
            }) => Some(schema),
            _ => None,
        })
        .unwrap()
}
fn package(input: SchemaPackageInput) -> OwnedDefinitionSchemaPackage {
    OwnedDefinitionSchemaPackage::new(input, Default::default()).unwrap()
}
fn reject(input: SchemaPackageInput, expected: SchemaPackageErrorKind) {
    match OwnedDefinitionSchemaPackage::new(input, Default::default()) {
        Err(SchemaPackageError::Invalid { kind, path }) => {
            assert_eq!(kind, expected, "{path}");
            assert!(!path.is_empty());
        }
        other => panic!("expected {expected:?}, got {other:?}"),
    }
}

fn wire_policy(wire: &mut Value) -> &mut Value {
    &mut wire["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["kind"] == "usage_policy")
        .unwrap()["value"]["schema"]["value"]
}

fn wire_supplied(wire: &mut Value) -> &mut Value {
    &mut wire_policy(wire)["scenario_skill_applicability"]["members"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| {
            row["kind"] == "supplied"
                && row["value"]["grant"]["declaration"]["definition"] == "item_template:item"
        })
        .unwrap()["value"]
}

#[test]
fn compact_artifact_decode_retains_the_ordinary_usage_semantic_rejections() {
    let checked = package(input());
    let bytes = encode_schema_package(&checked, Default::default()).unwrap();
    assert_eq!(
        decode_schema_package(&bytes, Default::default())
            .unwrap()
            .input(),
        checked.input()
    );
    let baseline: Value = serde_json::from_slice(&bytes).unwrap();
    for (case, expected) in [
        ("target", SchemaPackageErrorKind::InvalidUsageApplicability),
        ("duplicate", SchemaPackageErrorKind::DuplicateMember),
        ("missing-direct", SchemaPackageErrorKind::MissingDefinition),
        (
            "not-direct",
            SchemaPackageErrorKind::InvalidUsageApplicability,
        ),
        ("missing-grant", SchemaPackageErrorKind::MissingSlot),
        ("missing-supply", SchemaPackageErrorKind::MissingSlot),
        (
            "wrong-skill",
            SchemaPackageErrorKind::InvalidUsageApplicability,
        ),
        (
            "crossed-supply",
            SchemaPackageErrorKind::InvalidUsageApplicability,
        ),
        (
            "wrong-grant",
            SchemaPackageErrorKind::InvalidUsageApplicability,
        ),
        ("undeclared-grant", SchemaPackageErrorKind::UndeclaredSlot),
        ("undeclared-supply", SchemaPackageErrorKind::UndeclaredSlot),
        ("empty-gaps", SchemaPackageErrorKind::EmptyGapEvidence),
    ] {
        let mut bad = baseline.clone();
        match case {
            "target" => wire_policy(&mut bad)["targets"] = json!(["actor"]),
            "duplicate" => {
                let rows = wire_policy(&mut bad)["scenario_skill_applicability"]["members"]
                    .as_array_mut()
                    .unwrap();
                rows.push(rows[0].clone());
            }
            "missing-direct" | "not-direct" => {
                let row = wire_policy(&mut bad)["scenario_skill_applicability"]["members"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|row| row["kind"] == "authored_direct")
                    .unwrap();
                row["value"]["skill"] = json!(if case == "missing-direct" {
                    "skill:absent"
                } else {
                    "skill:other"
                });
            }
            "missing-grant" => {
                wire_supplied(&mut bad)["grant"]["slot"] = json!("grant_slot:missing-grant")
            }
            "missing-supply" => {
                wire_supplied(&mut bad)["supply"]["slot"] = json!("skill_grant_slot:missing-supply")
            }
            "wrong-skill" => wire_supplied(&mut bad)["skill"] = json!("skill:other"),
            "crossed-supply" => {
                wire_supplied(&mut bad)["supply"] = json!({
                    "declaration":{"kind":"passive_node","definition":"passive_node:node"}, "slot":"skill_grant_slot:supply"
                })
            }
            "wrong-grant" => {
                let row = bad["slots"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|row| {
                        row["kind"] == "grant"
                            && row["value"]["id"]["declaration"]["definition"]
                                == "item_template:item"
                    })
                    .unwrap();
                row["value"]["schema"]["value"]["target"] = json!({"kind":"allocation_access","value":{"pools":{"members":[],"closure":{"kind":"complete"}}}});
            }
            "undeclared-grant" | "undeclared-supply" => {
                let row = bad["definitions"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|row| row["kind"] == "item_template")
                    .unwrap();
                let field = if case == "undeclared-grant" {
                    "grants"
                } else {
                    "skill_grants"
                };
                row["value"]["schema"]["value"]["declarations"][field]["members"] = json!([]);
            }
            "empty-gaps" => {
                wire_policy(&mut bad)["scenario_skill_applicability"] =
                    json!({"members":[],"closure":{"kind":"partial","value":{"gaps":[]}}})
            }
            _ => unreachable!(),
        }
        assert_ne!(bad, baseline, "{case}");
        let bytes = serde_json::to_vec(&bad).unwrap();
        // The compact syntax is valid; only the ordinary final validator certifies these links.
        let raw = decode_schema_input(&bytes, Default::default()).unwrap();
        reject(raw, expected.clone());
        match decode_schema_package(&bytes, Default::default()) {
            Err(SchemaPackageError::Invalid { kind, path }) => {
                assert_eq!(kind, expected, "{case}: {path}");
                assert!(!path.is_empty());
            }
            other => panic!("{case}: expected {expected:?}, got {other:?}"),
        }
    }
}

#[test]
fn required_nullable_authority_and_strict_recipient_wire() {
    let mut raw = input();
    policy(&mut raw).scenario_skill_applicability = None;
    let baseline = package(raw.clone());
    let bytes = encode_schema_package(&baseline, Default::default()).unwrap();
    assert_eq!(
        decode_schema_package(&bytes, Default::default())
            .unwrap()
            .identity(),
        baseline.identity()
    );
    let mut value = serde_json::to_value(policy(&mut raw)).unwrap();
    assert_eq!(value["scenario_skill_applicability"], Value::Null);
    value
        .as_object_mut()
        .unwrap()
        .remove("scenario_skill_applicability");
    assert!(serde_json::from_value::<UsagePolicySchema>(value).is_err());
    let mut raw = input();
    let rows = &policy(&mut raw)
        .scenario_skill_applicability
        .as_ref()
        .unwrap()
        .members;
    for row in rows {
        let mut value = serde_json::to_value(row).unwrap();
        value["value"]["unrecognized"] = json!(true);
        assert!(serde_json::from_value::<UsageSkillApplicability>(value).is_err());
    }
    assert_eq!(raw.schema_version, OWNED_SCHEMA_PACKAGE_V7);
    raw.schema_version = 6;
    assert!(matches!(
        OwnedDefinitionSchemaPackage::new(raw, Default::default()),
        Err(SchemaPackageError::UnsupportedVersion(6))
    ));
}

#[test]
fn canonical_rows_roundtrip_and_each_relation_changes_identity() {
    let original = input();
    let baseline = package(original.clone());
    let mut reordered = original.clone();
    reordered.definitions.reverse();
    reordered.slots.reverse();
    policy(&mut reordered)
        .scenario_skill_applicability
        .as_mut()
        .unwrap()
        .members
        .reverse();
    let reordered = package(reordered);
    assert_eq!(baseline.identity(), reordered.identity());
    assert_eq!(
        encode_schema_package(&baseline, Default::default()).unwrap(),
        encode_schema_package(&reordered, Default::default()).unwrap()
    );
    let bytes = encode_schema_package(&baseline, Default::default()).unwrap();
    assert_eq!(
        decode_schema_package(&bytes, Default::default())
            .unwrap()
            .input(),
        baseline.input()
    );
    for row in 0..4 {
        let mut changed = original.clone();
        policy(&mut changed)
            .scenario_skill_applicability
            .as_mut()
            .unwrap()
            .members
            .remove(row);
        assert_ne!(package(changed).identity(), baseline.identity());
    }
}

#[test]
fn applicability_needs_skill_target_and_exact_registered_addresses() {
    let mut bad = input();
    policy(&mut bad).targets = vec![UsageTargetKind::Actor];
    reject(bad, SchemaPackageErrorKind::InvalidUsageApplicability);
    let mut duplicate = input();
    let rows = &mut policy(&mut duplicate)
        .scenario_skill_applicability
        .as_mut()
        .unwrap()
        .members;
    rows.push(rows[0].clone());
    reject(duplicate, SchemaPackageErrorKind::DuplicateMember);
    let mut missing = input();
    policy(&mut missing)
        .scenario_skill_applicability
        .as_mut()
        .unwrap()
        .members[0] = UsageSkillApplicability::AuthoredDirect {
        skill: id("absent"),
    };
    reject(missing, SchemaPackageErrorKind::MissingDefinition);
    let mut foreign = input();
    policy(&mut foreign)
        .scenario_skill_applicability
        .as_mut()
        .unwrap()
        .members[0] = UsageSkillApplicability::AuthoredDirect {
        skill: SkillDefId::parse(
            GameVersionNamespace::new("foreign", "v1").unwrap(),
            "effect",
        )
        .unwrap(),
    };
    reject(foreign, SchemaPackageErrorKind::ForeignNamespace);
    let mut missing = input();
    let UsageSkillApplicability::Supplied(row) = &mut policy(&mut missing)
        .scenario_skill_applicability
        .as_mut()
        .unwrap()
        .members[1]
    else {
        panic!()
    };
    row.grant.slot = id("missing-grant");
    reject(missing, SchemaPackageErrorKind::MissingSlot);
    let mut missing = input();
    let UsageSkillApplicability::Supplied(row) = &mut policy(&mut missing)
        .scenario_skill_applicability
        .as_mut()
        .unwrap()
        .members[1]
    else {
        panic!()
    };
    row.supply.slot = id("missing-supply");
    reject(missing, SchemaPackageErrorKind::MissingSlot);
}

#[test]
fn known_direct_and_supply_contradictions_are_rejected() {
    let mut direct = input();
    policy(&mut direct)
        .scenario_skill_applicability
        .as_mut()
        .unwrap()
        .members[0] = UsageSkillApplicability::AuthoredDirect { skill: id("other") };
    reject(direct, SchemaPackageErrorKind::InvalidUsageApplicability);
    let mut wrong = input();
    let UsageSkillApplicability::Supplied(row) = &mut policy(&mut wrong)
        .scenario_skill_applicability
        .as_mut()
        .unwrap()
        .members[1]
    else {
        panic!()
    };
    row.skill = id("other");
    reject(wrong, SchemaPackageErrorKind::InvalidUsageApplicability);
    let mut wrong = input();
    let UsageSkillApplicability::Supplied(row) = &mut policy(&mut wrong)
        .scenario_skill_applicability
        .as_mut()
        .unwrap()
        .members[1]
    else {
        panic!()
    };
    row.supply = supply(owners()[1].clone()).supply;
    reject(wrong, SchemaPackageErrorKind::InvalidUsageApplicability);
    let mut wrong = input();
    let SlotDescriptor::Grant(DefinitionEntry {
        schema: SchemaState::Known(grant),
        ..
    }) = &mut wrong.slots[0]
    else {
        panic!()
    };
    grant.target = GrantTarget::AllocationAccess {
        pools: DeclaredSet::complete(vec![]),
    };
    reject(wrong, SchemaPackageErrorKind::InvalidUsageApplicability);
    let mut absent = input();
    let DefinitionDescriptor::ItemTemplate(DefinitionEntry {
        schema: SchemaState::Known(item),
        ..
    }) = &mut absent.definitions[2]
    else {
        panic!()
    };
    item.declarations.grants.members.clear();
    reject(absent, SchemaPackageErrorKind::UndeclaredSlot);
    let mut absent = input();
    let DefinitionDescriptor::ItemTemplate(DefinitionEntry {
        schema: SchemaState::Known(item),
        ..
    }) = &mut absent.definitions[2]
    else {
        panic!()
    };
    item.declarations.skill_grants.members.clear();
    reject(absent, SchemaPackageErrorKind::UndeclaredSlot);
}

#[test]
fn explicit_cross_owner_grant_links_remain_valid() {
    let mut raw = input();
    let parent = supply(owners()[2].clone());
    let cross_supply = slot(SlotOwnerDefId::Skill(id("effect")), "cross-supply");
    let DefinitionDescriptor::Skill(DefinitionEntry {
        schema: SchemaState::Known(skill),
        ..
    }) = &mut raw.definitions[0]
    else {
        panic!()
    };
    skill
        .declarations
        .skill_grants
        .members
        .push(cross_supply.clone());
    raw.slots.push(SlotDescriptor::SkillGrant(known(
        cross_supply.clone(),
        SkillGrantSlotSchema {
            skill: id("effect"),
            outputs: DeclaredSet::complete(vec![]),
            preset_inputs: None,
        },
    )));
    let SlotDescriptor::Grant(DefinitionEntry {
        schema: SchemaState::Known(grant),
        ..
    }) = &mut raw.slots[4]
    else {
        panic!()
    };
    grant.target = GrantTarget::Skill(cross_supply.clone());
    let UsageSkillApplicability::Supplied(row) = &mut policy(&mut raw)
        .scenario_skill_applicability
        .as_mut()
        .unwrap()
        .members[3]
    else {
        panic!()
    };
    assert_eq!(row.grant, parent.grant);
    row.supply = cross_supply;
    assert_ne!(row.grant.declaration, row.supply.declaration);
    package(raw);
}

#[test]
fn complete_empty_partial_and_unmapped_knowledge_stay_distinct() {
    let mut empty = input();
    policy(&mut empty).scenario_skill_applicability = Some(DeclaredSet::complete(vec![]));
    let complete = package(empty.clone());
    policy(&mut empty).scenario_skill_applicability = Some(DeclaredSet::partial(
        vec![],
        vec![gap(SchemaSubject::Definition(
            id::<SkillDefinition>("effect").address(),
        ))],
    ));
    let partial = package(empty);
    assert_ne!(partial.identity(), complete.identity());
    let SchemaLookup::Known(usage) = partial.definition(&id::<UsagePolicyDefinition>("stages"))
    else {
        panic!()
    };
    assert!(
        !usage
            .scenario_skill_applicability
            .as_ref()
            .unwrap()
            .is_complete()
    );
    let mut raw = input();
    let SlotDescriptor::Grant(grant) = &mut raw.slots[0] else {
        panic!()
    };
    grant.schema = SchemaState::Unmapped {
        gaps: vec![gap(SchemaSubject::Slot(GrantSlotDefId::address(&grant.id)))],
    };
    let unknown = package(raw);
    assert!(matches!(
        unknown.slot(&supply(owners()[0].clone()).grant),
        SchemaLookup::Unmapped(_)
    ));
    let mut raw = input();
    let SlotDescriptor::SkillGrant(supplied) = &mut raw.slots[1] else {
        panic!()
    };
    supplied.schema = SchemaState::Unmapped {
        gaps: vec![gap(SchemaSubject::Slot(SkillGrantSlotDefId::address(
            &supplied.id,
        )))],
    };
    let unknown = package(raw);
    assert!(matches!(
        unknown.slot(&supply(owners()[0].clone()).supply),
        SchemaLookup::Unmapped(_)
    ));
    let mut raw = input();
    let DefinitionDescriptor::Skill(skill) = &mut raw.definitions[0] else {
        panic!()
    };
    skill.schema = SchemaState::Unmapped {
        gaps: vec![gap(SchemaSubject::Definition(skill.id.address()))],
    };
    let unknown = package(raw);
    assert!(matches!(
        unknown.definition(&id::<SkillDefinition>("effect")),
        SchemaLookup::Unmapped(_)
    ));
    let mut raw = input();
    let DefinitionDescriptor::ItemTemplate(DefinitionEntry {
        schema: SchemaState::Known(item),
        ..
    }) = &mut raw.definitions[2]
    else {
        panic!()
    };
    item.declarations.grants = DeclaredSet::partial(
        vec![],
        vec![gap(SchemaSubject::Slot(GrantSlotDefId::address(
            &supply(owners()[0].clone()).grant,
        )))],
    );
    let partial_owner = package(raw);
    let SchemaLookup::Known(item) = partial_owner.definition(&id::<ItemTemplateDefinition>("item"))
    else {
        panic!()
    };
    assert!(!item.declarations.grants.is_complete());
    let mut raw = input();
    policy(&mut raw).scenario_skill_applicability = Some(DeclaredSet::partial(vec![], vec![]));
    reject(raw, SchemaPackageErrorKind::EmptyGapEvidence);
}

fn arrays(value: &Value) -> (usize, usize) {
    let mut total = 0;
    let mut largest = 0;
    match value {
        Value::Array(rows) => {
            total += rows.len();
            largest = rows.len();
            for row in rows {
                let (n, m) = arrays(row);
                total += n;
                largest = largest.max(m);
            }
        }
        Value::Object(fields) => {
            for field in fields.values() {
                let (n, m) = arrays(field);
                total += n;
                largest = largest.max(m);
            }
        }
        _ => {}
    }
    (total, largest)
}

#[test]
fn semantic_link_work_and_collection_wire_budgets_apply_to_decode_and_encode() {
    let mut raw = input();
    let usage = policy(&mut raw).clone();
    for n in 0..30 {
        raw.definitions
            .push(DefinitionDescriptor::UsagePolicy(known(
                id(&format!("policy-{n}")),
                usage.clone(),
            )));
    }
    let baseline = package(raw.clone());
    let bytes = encode_schema_package(&baseline, Default::default()).unwrap();
    let (entries, largest) = arrays(&serde_json::to_value(&raw).unwrap());
    let links = 31 * (1 + 3 * 3);
    assert!(links > entries);
    let exact = OwnedSchemaLimits {
        max_entries: links,
        max_collection_entries: largest,
        max_wire_bytes: bytes.len(),
        max_semantic_bytes: serde_json::to_vec(baseline.input()).unwrap().len(),
    };
    assert_eq!(
        OwnedDefinitionSchemaPackage::new(raw.clone(), exact)
            .unwrap()
            .identity(),
        baseline.identity()
    );
    assert_eq!(
        decode_schema_package(&bytes, exact).unwrap().identity(),
        baseline.identity()
    );
    assert_eq!(encode_schema_package(&baseline, exact).unwrap(), bytes);
    for limits in [
        OwnedSchemaLimits {
            max_entries: links - 1,
            ..exact
        },
        OwnedSchemaLimits {
            max_collection_entries: largest - 1,
            ..exact
        },
    ] {
        assert!(OwnedDefinitionSchemaPackage::new(raw.clone(), limits).is_err());
        assert!(decode_schema_package(&bytes, limits).is_err());
        assert!(encode_schema_package(&baseline, limits).is_err());
    }
    let tight = OwnedSchemaLimits {
        max_wire_bytes: bytes.len() - 1,
        ..exact
    };
    assert!(OwnedDefinitionSchemaPackage::new(raw, tight).is_err());
    assert!(decode_schema_package(&bytes, tight).is_err());
    assert!(encode_schema_package(&baseline, tight).is_err());
}
