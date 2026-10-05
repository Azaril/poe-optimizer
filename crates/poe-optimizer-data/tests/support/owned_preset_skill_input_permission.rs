//! Supply permission reuses the shared Skill-input schema fixture.
use super::*;

fn raw_slot() -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(id("selected-skill")),
        slot: id("raw-skill-input"),
    }
}
fn supply() -> DeclaredSlot<SkillGrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Modifier(id("provider")),
        slot: id("supply"),
    }
}
fn permission_input() -> SchemaPackageInput {
    let mut raw = input(
        Some(SkillInputAuthority::Projected),
        vec![],
        OWNED_SCHEMA_PACKAGE_V6,
    );
    let mut ports = declarations(raw_slot());
    ports.parameters.members.clear();
    ports.skill_grants.members.push(supply());
    raw.definitions
        .push(DefinitionDescriptor::Modifier(DefinitionEntry {
            id: id("provider"),
            schema: SchemaState::Known(ModifierSchema {
                declarations: ports,
            }),
        }));
    raw.slots.push(SlotDescriptor::SkillGrant(DefinitionEntry {
        id: supply(),
        schema: SchemaState::Known(SkillGrantSlotSchema {
            skill: id("selected-skill"),
            outputs: DeclaredSet::complete(vec![]),
            preset_inputs: Some(PresetSkillInputPermission {
                schema_version: PRESET_SKILL_INPUT_PERMISSION_V1,
                parameters: DeclaredSet::complete(vec![raw_slot()]),
            }),
        }),
    }));
    raw
}
fn grant(raw: &mut SchemaPackageInput) -> &mut SkillGrantSlotSchema {
    raw.slots
        .iter_mut()
        .find_map(|row| match row {
            SlotDescriptor::SkillGrant(DefinitionEntry {
                schema: SchemaState::Known(schema),
                ..
            }) => Some(schema),
            _ => None,
        })
        .unwrap()
}
fn permission(raw: &mut SchemaPackageInput) -> &mut PresetSkillInputPermission {
    grant(raw).preset_inputs.as_mut().unwrap()
}
fn slot(raw: &mut SchemaPackageInput) -> &mut ParameterSlotSchema {
    let SlotDescriptor::Parameter(DefinitionEntry {
        schema: SchemaState::Known(schema),
        ..
    }) = &mut raw.slots[0]
    else {
        unreachable!()
    };
    schema
}
fn partial() -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: SchemaSubject::Definition(DefinitionAddress::Skill(id("selected-skill"))),
            facet: SchemaFacet::StaticLinks,
            code: key("unknown-inputs"),
        }],
    }
}

#[test]
fn permission_is_v6_only_and_omission_preserves_legacy_supply_bytes() {
    assert_eq!(OWNED_SCHEMA_PACKAGE_VERSION, OWNED_SCHEMA_PACKAGE_V4);
    let mut old = permission_input();
    let supply = grant(&mut old).clone();
    let legacy = json!({"skill":supply.skill,"outputs":supply.outputs});
    let decoded: SkillGrantSlotSchema = serde_json::from_value(legacy.clone()).unwrap();
    assert!(decoded.preset_inputs.is_none());
    assert_eq!(serde_json::to_value(&decoded).unwrap(), legacy);
    grant(&mut old).preset_inputs = None;
    for version in [
        OWNED_SCHEMA_PACKAGE_V2,
        OWNED_SCHEMA_PACKAGE_V3,
        OWNED_SCHEMA_PACKAGE_V4,
        OWNED_SCHEMA_PACKAGE_V5,
    ] {
        let mut legacy = old.clone();
        legacy.schema_version = version;
        // Legacy projected slots use omission before V5; this does not grant the
        // newly authored supply permission at any historical version.
        if version < OWNED_SCHEMA_PACKAGE_V5 {
            slot(&mut legacy).skill_input = None;
        }
        let stored = package(legacy.clone());
        let bytes = encode_schema_package(&stored, Default::default()).unwrap();
        assert!(
            !String::from_utf8(bytes.clone())
                .unwrap()
                .contains("preset_inputs")
        );
        assert_eq!(
            decode_schema_package(&bytes, Default::default())
                .unwrap()
                .identity(),
            stored.identity()
        );
        grant(&mut legacy).preset_inputs = supply.preset_inputs.clone();
        reject(legacy, SchemaPackageErrorKind::UnsupportedSchemaFeature);
    }
    let allowed = package(permission_input());
    assert_ne!(allowed.identity(), package(old.clone()).identity());
    grant(&mut old).preset_inputs = Some(PresetSkillInputPermission {
        schema_version: 1,
        parameters: DeclaredSet::complete(vec![]),
    });
    let explicitly_empty = package(old);
    assert_ne!(allowed.identity(), explicitly_empty.identity());
}

#[test]
fn permission_wire_is_strict_and_complete_including_empty_permissions() {
    let mut raw = permission_input();
    let encoded = serde_json::to_value(grant(&mut raw)).unwrap();
    for pointer in [
        "/preset_inputs",
        "/preset_inputs/schema_version",
        "/preset_inputs/parameters",
    ] {
        let mut bad = encoded.clone();
        *bad.pointer_mut(pointer).unwrap() = Value::Null;
        assert!(serde_json::from_value::<SkillGrantSlotSchema>(bad).is_err());
    }
    for pointer in ["", "/preset_inputs", "/preset_inputs/parameters"] {
        let mut bad = encoded.clone();
        bad.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown".into(), true.into());
        assert!(serde_json::from_value::<SkillGrantSlotSchema>(bad).is_err());
    }
    let text = serde_json::to_string(&encoded).unwrap();
    let duplicate = text.replacen(
        "\"schema_version\":1",
        "\"schema_version\":1,\"schema_version\":1",
        1,
    );
    assert_ne!(duplicate, text);
    assert!(serde_json::from_str::<SkillGrantSlotSchema>(&duplicate).is_err());
    for version in [0, 2] {
        let mut bad = permission_input();
        permission(&mut bad).schema_version = version;
        reject(bad, SchemaPackageErrorKind::InvalidPresetInputPermission);
    }
    for empty in [false, true] {
        let mut bad = permission_input();
        let p = permission(&mut bad);
        if empty {
            p.parameters.members.clear();
        }
        p.parameters.closure = partial();
        reject(bad, SchemaPackageErrorKind::InvalidPresetInputPermission);
    }
}

#[test]
fn permission_requires_explicit_projected_authority_and_keeps_other_required_inputs() {
    for authority in [
        SkillInputAuthority::Projected,
        SkillInputAuthority::AuthoredOrProjected,
    ] {
        let mut raw = permission_input();
        slot(&mut raw).skill_input = Some(authority);
        slot(&mut raw).sites = if authority == SkillInputAuthority::AuthoredOrProjected {
            vec![ParameterSite::SkillParameter]
        } else {
            vec![]
        };
        let other = DeclaredSlot {
            declaration: raw_slot().declaration,
            slot: id("provider-level"),
        };
        let DefinitionDescriptor::Skill(DefinitionEntry {
            schema: SchemaState::Known(skill),
            ..
        }) = &mut raw.definitions[0]
        else {
            unreachable!()
        };
        skill.declarations.parameters.members.push(other.clone());
        raw.slots.push(SlotDescriptor::Parameter(DefinitionEntry {
            id: other.clone(),
            schema: SchemaState::Known(parameter(Some(SkillInputAuthority::Projected), vec![])),
        }));
        let stored = package(raw);
        let SchemaLookup::Known(other_schema) = stored.slot(&other) else {
            panic!("other required input")
        };
        assert_eq!(other_schema.presence, SlotPresence::RequiredOnce);
        let SchemaLookup::Known(grant) = stored.slot(&supply()) else {
            panic!("supply")
        };
        assert_eq!(
            grant.preset_inputs.as_ref().unwrap().parameters.members,
            vec![raw_slot()]
        );
        let bytes = encode_schema_package(&stored, Default::default()).unwrap();
        assert_eq!(
            decode_schema_package(&bytes, Default::default())
                .unwrap()
                .identity(),
            stored.identity()
        );
    }
    for authority in [None, Some(SkillInputAuthority::Authored)] {
        let mut raw = permission_input();
        slot(&mut raw).skill_input = authority;
        slot(&mut raw).sites = if authority.is_some() {
            vec![ParameterSite::SkillParameter]
        } else {
            vec![]
        };
        reject(raw, SchemaPackageErrorKind::InvalidPresetInputPermission);
    }
}

#[test]
fn permission_cannot_join_unknown_wrong_or_undeclared_targets() {
    let mut duplicate = permission_input();
    permission(&mut duplicate)
        .parameters
        .members
        .push(raw_slot());
    reject(duplicate, SchemaPackageErrorKind::DuplicateMember);
    let mut missing = permission_input();
    permission(&mut missing).parameters.members[0].slot = id("missing");
    reject(missing, SchemaPackageErrorKind::MissingSlot);
    let mut foreign = permission_input();
    permission(&mut foreign).parameters.members[0].slot = ParameterSlotDefId::parse(
        GameVersionNamespace::new("foreign", "v1").unwrap(),
        "raw-skill-input",
    )
    .unwrap();
    reject(foreign, SchemaPackageErrorKind::ForeignNamespace);
    let mut wrong = permission_input();
    let other = DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(id("other-skill")),
        slot: id("other-input"),
    };
    wrong
        .definitions
        .push(DefinitionDescriptor::Skill(DefinitionEntry {
            id: id("other-skill"),
            schema: SchemaState::Known(SkillSchema {
                directly_selectable: false,
                declarations: declarations(other.clone()),
            }),
        }));
    wrong.slots.push(SlotDescriptor::Parameter(DefinitionEntry {
        id: other.clone(),
        schema: SchemaState::Known(parameter(Some(SkillInputAuthority::Projected), vec![])),
    }));
    permission(&mut wrong).parameters.members = vec![other];
    reject(wrong, SchemaPackageErrorKind::WrongDeclaration);
    for remove_supply in [false, true] {
        let mut raw = permission_input();
        if remove_supply {
            let DefinitionDescriptor::Modifier(DefinitionEntry {
                schema: SchemaState::Known(owner),
                ..
            }) = &mut raw.definitions[1]
            else {
                unreachable!()
            };
            owner.declarations.skill_grants.members.clear();
            owner.declarations.skill_grants.closure = partial();
        } else {
            let DefinitionDescriptor::Skill(DefinitionEntry {
                schema: SchemaState::Known(skill),
                ..
            }) = &mut raw.definitions[0]
            else {
                unreachable!()
            };
            skill.declarations.parameters.members.clear();
            skill.declarations.parameters.closure = partial();
        }
        reject(raw, SchemaPackageErrorKind::UndeclaredSlot);
    }
    for unknown_skill in [false, true] {
        let mut raw = permission_input();
        let SchemaClosure::Partial { gaps } = partial() else {
            unreachable!()
        };
        if unknown_skill {
            let DefinitionDescriptor::Skill(entry) = &mut raw.definitions[0] else {
                unreachable!()
            };
            entry.schema = SchemaState::Unmapped { gaps };
        } else {
            let SlotDescriptor::Parameter(entry) = &mut raw.slots[0] else {
                unreachable!()
            };
            entry.schema = SchemaState::Unmapped { gaps };
        }
        reject(raw, SchemaPackageErrorKind::InvalidPresetInputPermission);
    }
}

fn array_entries(value: &Value) -> (usize, usize) {
    match value {
        Value::Array(rows) => rows.iter().map(array_entries).fold(
            (rows.len(), rows.len()),
            |(total, largest), (count, size)| (total + count, largest.max(size)),
        ),
        Value::Object(rows) => rows
            .values()
            .map(array_entries)
            .fold((0, 0), |(total, largest), (count, size)| {
                (total + count, largest.max(size))
            }),
        _ => (0, 0),
    }
}

#[test]
fn permission_does_not_bypass_the_shared_slot_value_schema() {
    let mut reversed = permission_input();
    slot(&mut reversed).value = ValueSchema::Integer(IntegerRange {
        minimum: BoundedInteger::new(2).unwrap(),
        maximum: BoundedInteger::new(1).unwrap(),
    });
    reject(reversed, SchemaPackageErrorKind::ReversedRange);
    let mut missing_unit = permission_input();
    slot(&mut missing_unit).value = ValueSchema::Quantity(QuantityRange {
        minimum: FiniteQuantity::new(-1.0, id("percent")).unwrap(),
        maximum: FiniteQuantity::new(101.5, id("percent")).unwrap(),
    });
    reject(
        missing_unit.clone(),
        SchemaPackageErrorKind::MissingDefinition,
    );
    missing_unit
        .definitions
        .push(DefinitionDescriptor::Unit(DefinitionEntry {
            id: id("percent"),
            schema: SchemaState::Known(UnitSchema {
                dimension: UnitDimension::PercentagePoints,
            }),
        }));
    let stored = package(missing_unit);
    let SchemaLookup::Known(value) = stored.slot(&raw_slot()) else {
        panic!("declared shared slot")
    };
    assert_eq!(
        value.value,
        ValueSchema::Quantity(QuantityRange {
            minimum: FiniteQuantity::new(-1.0, id("percent")).unwrap(),
            maximum: FiniteQuantity::new(101.5, id("percent")).unwrap(),
        })
    );
}

#[test]
fn permission_members_are_charged_and_revalidated_at_tighter_limits() {
    let raw = permission_input();
    let stored = package(raw.clone());
    let bytes = encode_schema_package(&stored, Default::default()).unwrap();
    let (max_entries, max_collection_entries) = array_entries(&serde_json::to_value(&raw).unwrap());
    let exact = OwnedSchemaLimits {
        max_entries,
        max_collection_entries,
        max_wire_bytes: bytes.len(),
    };
    assert_eq!(
        OwnedDefinitionSchemaPackage::new(raw.clone(), exact)
            .unwrap()
            .identity(),
        stored.identity()
    );
    assert_eq!(
        decode_schema_package(&bytes, exact).unwrap().identity(),
        stored.identity()
    );
    for tight in [
        OwnedSchemaLimits {
            max_entries: max_entries - 1,
            ..exact
        },
        OwnedSchemaLimits {
            max_collection_entries: max_collection_entries - 1,
            ..exact
        },
    ] {
        for result in [
            OwnedDefinitionSchemaPackage::new(raw.clone(), tight),
            decode_schema_package(&bytes, tight),
        ] {
            assert!(matches!(
                result,
                Err(SchemaPackageError::Invalid {
                    kind: SchemaPackageErrorKind::LimitExceeded,
                    ..
                })
            ));
        }
        assert!(matches!(
            encode_schema_package(&stored, tight),
            Err(SchemaPackageError::Invalid {
                kind: SchemaPackageErrorKind::LimitExceeded,
                ..
            })
        ));
    }
    assert_eq!(encode_schema_package(&stored, exact).unwrap(), bytes);
    assert_eq!(package(raw).identity(), stored.identity());
}
