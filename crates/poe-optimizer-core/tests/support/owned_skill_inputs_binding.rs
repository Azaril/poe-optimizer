use super::*;

fn input_slot() -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def("direct")),
        slot: def("raw-level"),
    }
}
fn assignment(value: i64) -> ParameterAssignment {
    ParameterAssignment {
        slot: input_slot(),
        value: ParameterValue::Integer(BoundedInteger::new(value).unwrap()),
    }
}
fn fixture(authority: Option<SkillInputAuthority>) -> Fixture {
    let mut f = Fixture::new();
    let mut declared = declarations();
    declared.parameters.members.push(input_slot());
    f.index.put(DefinitionDescriptor::Skill(known(
        def("direct"),
        SkillSchema {
            directly_selectable: true,
            declarations: declared,
        },
    )));
    let row = SlotDescriptor::Parameter(known(
        input_slot(),
        ParameterSlotSchema {
            value: ValueSchema::Integer(range(1, 40)),
            presence: SlotPresence::RequiredOnce,
            sites: if matches!(
                authority,
                Some(SkillInputAuthority::Authored | SkillInputAuthority::AuthoredOrProjected)
            ) {
                vec![ParameterSite::SkillParameter]
            } else {
                vec![]
            },
            skill_input: authority,
        },
    ));
    f.index.slots.insert(row.address(), row);
    f.build.skills = [12, 24]
        .into_iter()
        .enumerate()
        .map(|(i, value)| SkillUse {
            id: id(20 + i as u64),
            source: AuthoredSkillSource::Direct(def("direct")),
            enabled: true,
            scope: LoadoutScope::Shared,
            parameters: Some(vec![assignment(value)]),
        })
        .collect();
    f
}
fn schema(f: &mut Fixture) -> &mut ParameterSlotSchema {
    match f
        .index
        .slots
        .get_mut(&ParameterSlotDefId::address(&input_slot()))
        .unwrap()
    {
        SlotDescriptor::Parameter(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) => schema,
        _ => panic!("fixture schema"),
    }
}
fn has(f: &Fixture, code: BindingIssueCode) -> bool {
    f.bind().issues().iter().any(|issue| issue.code == code)
}

#[test]
fn exact_direct_occurrences_bind_distinct_shared_slot_values() {
    for authority in [
        SkillInputAuthority::Authored,
        SkillInputAuthority::AuthoredOrProjected,
    ] {
        let f = fixture(Some(authority));
        assert_eq!(f.bind().schema(), SchemaBindingStatus::Valid);
        let build = f.owned();
        assert_eq!(
            build.build().input().skills[0].parameters,
            Some(vec![assignment(12)])
        );
        assert_eq!(
            build.build().input().skills[1].parameters,
            Some(vec![assignment(24)])
        );
    }
}

#[test]
fn required_authored_slots_are_checked_even_when_omitted_disabled_or_off_loadout() {
    for parameters in [None, Some(vec![])] {
        let mut f = fixture(Some(SkillInputAuthority::Authored));
        f.build.skills[0].parameters = parameters;
        f.build.skills[0].enabled = false;
        f.build.weapon_loadouts.push(id(30));
        f.build.skills[0].scope = LoadoutScope::Selected {
            loadouts: vec![id(30)],
        };
        assert!(has(&f, BindingIssueCode::RequiredValueMissing));
        assert_eq!(f.bind().schema(), SchemaBindingStatus::Invalid);
    }
}

#[test]
fn projection_only_and_legacy_slots_never_admit_authored_values() {
    for authority in [None, Some(SkillInputAuthority::Projected)] {
        let mut f = fixture(authority);
        assert!(has(&f, BindingIssueCode::ValueForbidden));
        for skill in &mut f.build.skills {
            skill.parameters = None;
        }
        assert_eq!(f.bind().schema(), SchemaBindingStatus::Valid);
        // Historical generated-only inventories were not authored input closure.
        if authority.is_none() {
            let DefinitionDescriptor::Skill(DefinitionEntry {
                schema: SchemaState::Known(value),
                ..
            }) = f
                .index
                .definitions
                .get_mut(&def::<SkillDefinition>("direct").address())
                .unwrap()
            else {
                panic!()
            };
            value.declarations.parameters.closure = SchemaClosure::Partial {
                gaps: vec![gap(SchemaSubject::Definition(
                    def::<SkillDefinition>("direct").address(),
                ))],
            };
            assert_eq!(f.bind().schema(), SchemaBindingStatus::Valid);
            f.build.skills[0].parameters = Some(vec![]);
            assert!(has(&f, BindingIssueCode::PartialMembership));
        }
    }
}

#[test]
fn skill_values_use_existing_type_bounds_unit_option_and_membership_checks() {
    let base = fixture(Some(SkillInputAuthority::AuthoredOrProjected));
    let mut f = base.clone();
    f.build.skills[0].parameters.as_mut().unwrap()[0].value = ParameterValue::Boolean(true);
    assert!(has(&f, BindingIssueCode::ValueKindMismatch));
    let mut f = base.clone();
    f.build.skills[0].parameters = Some(vec![assignment(41)]);
    assert!(has(&f, BindingIssueCode::OutOfRange));
    let mut f = base.clone();
    schema(&mut f).value = ValueSchema::Quantity(QuantityRange {
        minimum: FiniteQuantity::new(0.0, def("expected-unit")).unwrap(),
        maximum: FiniteQuantity::new(40.0, def("expected-unit")).unwrap(),
    });
    f.build.skills[0].parameters.as_mut().unwrap()[0].value =
        ParameterValue::Quantity(FiniteQuantity::new(12.0, def("wrong-unit")).unwrap());
    assert!(has(&f, BindingIssueCode::UnitMismatch));
    let mut f = base.clone();
    schema(&mut f).value = ValueSchema::Option {
        allowed: DeclaredSet::complete(vec![def("allowed")]),
    };
    f.build.skills[0].parameters.as_mut().unwrap()[0].value = ParameterValue::Option(def("other"));
    assert!(has(&f, BindingIssueCode::NotDeclared));
    let mut f = base;
    schema(&mut f).sites.clear();
    assert!(has(&f, BindingIssueCode::IncompatibleRole));
}

#[test]
fn structure_rejects_duplicate_foreign_and_physical_gem_input_layers() {
    let base = fixture(Some(SkillInputAuthority::Authored));
    let mut f = base.clone();
    f.build.skills[0].parameters = Some(vec![assignment(12), assignment(20)]);
    assert_eq!(
        BuildSpec::new(f.build, OwnedInputLimits::default())
            .unwrap_err()
            .kind,
        StructuralErrorKind::DuplicateAssignment
    );
    let mut f = base.clone();
    f.build.skills[0].parameters.as_mut().unwrap()[0]
        .slot
        .declaration = SlotOwnerDefId::Skill(def("sibling"));
    assert_eq!(
        BuildSpec::new(f.build, OwnedInputLimits::default())
            .unwrap_err()
            .kind,
        StructuralErrorKind::WrongDeclaration
    );
    let mut f = base.clone();
    f.build.skills[0].parameters.as_mut().unwrap()[0].slot.slot = DefId::parse(
        GameVersionNamespace::new("foreign", "v1").unwrap(),
        "raw-level",
    )
    .unwrap();
    assert_eq!(
        BuildSpec::new(f.build, OwnedInputLimits::default())
            .unwrap_err()
            .kind,
        StructuralErrorKind::ForeignNamespace
    );
    for parameters in [Some(vec![]), Some(vec![assignment(12)])] {
        let mut f = base.clone();
        f.build.skills[0].source = AuthoredSkillSource::Gem(id(5));
        f.build.skills[0].parameters = parameters;
        assert_eq!(
            BuildSpec::new(f.build, OwnedInputLimits::default())
                .unwrap_err()
                .kind,
            StructuralErrorKind::WrongDeclaration
        );
    }
}

#[test]
fn omitted_extensions_preserve_record_bytes_and_explicit_null_rejects() {
    let mut record = fixture(Some(SkillInputAuthority::Authored))
        .build
        .skills
        .remove(0);
    record.parameters = None;
    let wire = serde_json::to_value(&record).unwrap();
    assert!(wire.get("parameters").is_none());
    let bytes = serde_json::to_vec(&record).unwrap();
    assert_eq!(
        serde_json::to_vec(&serde_json::from_slice::<SkillUse>(&bytes).unwrap()).unwrap(),
        bytes
    );
    let mut null = wire;
    null["parameters"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<SkillUse>(null).is_err());
    let mut f = fixture(None);
    let input = schema(&mut f);
    let wire = serde_json::to_value(&input).unwrap();
    assert!(wire.get("skill_input").is_none());
    assert!(input.permits_projected_skill_input());
    assert!(!input.permits_authored_skill_input());
    let mut null = wire;
    null["skill_input"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<ParameterSlotSchema>(null).is_err());
}
