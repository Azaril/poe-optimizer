use super::*;

fn slot(name: &str) -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def("supplied")),
        slot: def(name),
    }
}
fn fixture() -> Fixture {
    let mut f = Fixture::new();
    let mut declarations = declarations();
    declarations.parameters.members = vec![slot("quality"), slot("level")];
    f.index.put(DefinitionDescriptor::Skill(known(
        def("supplied"),
        SkillSchema {
            directly_selectable: false,
            declarations,
        },
    )));
    for name in ["quality", "level"] {
        let row = SlotDescriptor::Parameter(known(
            slot(name),
            ParameterSlotSchema {
                value: ValueSchema::Integer(range(0, 40)),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
                skill_input: Some(SkillInputAuthority::Projected),
            },
        ));
        f.index.slots.insert(row.address(), row);
    }
    let supply = DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(def("item")),
        slot: def("supplied-skill"),
    };
    f.index
        .slots(&supply.declaration)
        .skill_grants
        .members
        .push(supply.clone());
    let row = SlotDescriptor::SkillGrant(known(
        supply.clone(),
        SkillGrantSlotSchema {
            skill: def("supplied"),
            outputs: empty(),
            preset_inputs: Some(PresetSkillInputPermission {
                schema_version: 1,
                parameters: DeclaredSet::complete(vec![slot("quality")]),
            }),
        },
    ));
    f.index.slots.insert(row.address(), row);
    f.build.generated_inputs = Some(GeneratedSkillInputsV1 {
        schema_version: 1,
        bindings: vec![SelectedGeneratedSkillInput {
            target: GeneratedSkillKey {
                provider: ProviderKey {
                    root: ProviderRoot::EquipmentUse(id(6)),
                    grant_path: vec![],
                },
                slot: supply,
            },
            parameters: vec![ParameterAssignment {
                slot: slot("quality"),
                value: ParameterValue::Integer(BoundedInteger::new(12).unwrap()),
            }],
            origin: GeneratedSkillInputOrigin {
                skill_preset: id(50),
            },
        }],
    });
    f
}
fn row(f: &mut Fixture) -> &mut SelectedGeneratedSkillInput {
    &mut f.build.generated_inputs.as_mut().unwrap().bindings[0]
}
fn permission(f: &mut Fixture) -> &mut Option<PresetSkillInputPermission> {
    let target = row(f).target.slot.clone();
    let SlotDescriptor::SkillGrant(DefinitionEntry {
        schema: SchemaState::Known(schema),
        ..
    }) = f
        .index
        .slots
        .get_mut(&SkillGrantSlotDefId::address(&target))
        .unwrap()
    else {
        panic!()
    };
    &mut schema.preset_inputs
}

#[test]
fn exact_selected_inputs_do_not_author_other_required_projected_slots() {
    let mut f = fixture();
    valid(&f.bind());
    let mut repeated = f.build.equipment[0].clone();
    repeated.id = id(7);
    f.build.equipment.push(repeated);
    let mut second = row(&mut f).clone();
    second.target.provider.root = ProviderRoot::EquipmentUse(id(7));
    second.parameters[0].value = ParameterValue::Integer(BoundedInteger::new(24).unwrap());
    f.build
        .generated_inputs
        .as_mut()
        .unwrap()
        .bindings
        .push(second);
    valid(&f.bind());
    assert_ne!(
        f.owned()
            .build()
            .input()
            .generated_inputs
            .as_ref()
            .unwrap()
            .bindings[0]
            .parameters,
        f.owned()
            .build()
            .input()
            .generated_inputs
            .as_ref()
            .unwrap()
            .bindings[1]
            .parameters
    );
}

#[test]
fn permission_and_values_cannot_be_hidden_by_inactive_loadouts() {
    let base = fixture();
    let mut f = base.clone();
    f.build.weapon_loadouts.push(id(9));
    f.build.equipment[0].scope = LoadoutScope::Selected {
        loadouts: vec![id(9)],
    };
    *permission(&mut f) = None;
    assert!(has(&f.bind(), BindingIssueCode::ValueForbidden));
    let mut f = base.clone();
    row(&mut f).parameters.clear();
    assert!(has(&f.bind(), BindingIssueCode::RequiredValueMissing));
    let mut f = base.clone();
    row(&mut f).parameters[0].value = ParameterValue::Boolean(false);
    assert!(has(&f.bind(), BindingIssueCode::ValueKindMismatch));
    let mut f = base.clone();
    row(&mut f).parameters[0].value = ParameterValue::Integer(BoundedInteger::new(41).unwrap());
    assert!(has(&f.bind(), BindingIssueCode::OutOfRange));
    let mut f = base;
    row(&mut f).parameters[0].slot = slot("level");
    assert!(has(&f.bind(), BindingIssueCode::NotDeclared));
}

#[test]
fn structural_resolver_preserves_inactive_supply_without_certifying_activation() {
    let mut f = fixture();
    f.build.weapon_loadouts.push(id(9));
    f.build.equipment[0].scope = LoadoutScope::Selected {
        loadouts: vec![id(9)],
    };
    let target = SkillTarget::Generated(Box::new(row(&mut f).target.clone()));
    let request = f.owned();
    let resolver =
        OwnedOccurrenceResolver::new(&f.index, &request, BindingLimits::default()).unwrap();
    assert!(resolver.skill(&target).unwrap().value().is_none());
    assert!(
        resolver
            .structural_skill(&target)
            .unwrap()
            .value()
            .is_some()
    );
}

#[test]
fn new_input_wire_is_opt_in_bounded_and_part_of_content_identity() {
    let base = fixture();
    let original = base.bind().request_digest();
    let mut f = base.clone();
    row(&mut f).parameters[0].value = ParameterValue::Integer(BoundedInteger::new(13).unwrap());
    assert_ne!(f.bind().request_digest(), original);
    let mut f = base.clone();
    let duplicate = row(&mut f).clone();
    f.build
        .generated_inputs
        .as_mut()
        .unwrap()
        .bindings
        .push(duplicate);
    assert_eq!(
        BuildSpec::new(f.build, OwnedInputLimits::default())
            .unwrap_err()
            .kind,
        StructuralErrorKind::DuplicateAssignment
    );
    let mut f = base.clone();
    f.build.generated_inputs.as_mut().unwrap().schema_version = 2;
    assert!(BuildSpec::new(f.build, OwnedInputLimits::default()).is_err());
    let mut null = serde_json::to_value(&base.build).unwrap();
    null["generated_inputs"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<BuildInput>(null).is_err());
    let mut unknown = serde_json::to_value(&base.build).unwrap();
    unknown["generated_inputs"]["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<BuildInput>(unknown).is_err());
    let mut f = base;
    f.build.generated_inputs = None;
    let bytes = serde_json::to_vec(&f.build).unwrap();
    assert!(
        !String::from_utf8(bytes.clone())
            .unwrap()
            .contains("generated_inputs")
    );
    assert_eq!(
        bytes,
        serde_json::to_vec(&serde_json::from_slice::<BuildInput>(&bytes).unwrap()).unwrap()
    );
}

#[test]
fn custom_index_cannot_grant_legacy_or_foreign_parameter_authority() {
    for authority in [None, Some(SkillInputAuthority::Authored)] {
        let mut f = fixture();
        let SlotDescriptor::Parameter(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) = f
            .index
            .slots
            .get_mut(&ParameterSlotDefId::address(&slot("quality")))
            .unwrap()
        else {
            panic!()
        };
        schema.skill_input = authority;
        assert!(bind_owned_request(&f.index, &f.owned(), BindingLimits::default()).is_err());
    }
    let mut f = fixture();
    permission(&mut f).as_mut().unwrap().parameters.members[0].declaration =
        SlotOwnerDefId::Skill(def("other"));
    assert!(bind_owned_request(&f.index, &f.owned(), BindingLimits::default()).is_err());
}

#[test]
fn shared_quantity_units_fractional_values_and_bounded_retry_remain_exact() {
    let mut f = fixture();
    for unit in ["quality-unit", "wrong-unit"] {
        f.index.put(DefinitionDescriptor::Unit(known(
            def(unit),
            UnitSchema {
                dimension: UnitDimension::Count,
            },
        )));
    }
    let SlotDescriptor::Parameter(DefinitionEntry {
        schema: SchemaState::Known(schema),
        ..
    }) = f
        .index
        .slots
        .get_mut(&ParameterSlotDefId::address(&slot("quality")))
        .unwrap()
    else {
        panic!()
    };
    schema.value = ValueSchema::Quantity(QuantityRange {
        minimum: FiniteQuantity::new(0.0, def("quality-unit")).unwrap(),
        maximum: FiniteQuantity::new(40.0, def("quality-unit")).unwrap(),
    });
    row(&mut f).parameters[0].value =
        ParameterValue::Quantity(FiniteQuantity::new(12.5, def("quality-unit")).unwrap());
    valid(&f.bind());
    assert!(matches!(
        bind_owned_request(
            &f.index,
            &f.owned(),
            BindingLimits {
                max_work: 1,
                ..Default::default()
            }
        ),
        Err(BindingError::WorkLimit)
    ));
    valid(&f.bind());
    row(&mut f).parameters[0].value =
        ParameterValue::Quantity(FiniteQuantity::new(12.5, def("wrong-unit")).unwrap());
    assert!(has(&f.bind(), BindingIssueCode::UnitMismatch));
}
