//! Import-only producer intent. These tests do not resolve recipients, final
//! participation, or parent-sensitive applicability.
use super::*;

fn bind(a: &Artifacts, p: &mut NormalizationPolicy) {
    let scalar = gem_inventory_scalar_inputs_identity(p, Default::default()).unwrap();
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        definitions,
        roles,
        scalar_inputs,
        ..
    } = p.usage_inputs.as_mut().unwrap();
    *definitions = a.schema.identity().clone();
    *roles = *a.roles.identity();
    *scalar_inputs = scalar;
}

fn boolean_fixture(default: Option<bool>) -> (Artifacts, NormalizationPolicy) {
    let (mut a, mut p) = fixture(false, RewardNumericSelection::RecipePrecedence);
    let slot = row(&mut p).parameters[0].slot.clone();
    let mut schema = a.schema.input().clone();
    let parameter = schema
        .slots
        .iter_mut()
        .find_map(|entry| match entry {
            SlotDescriptor::Parameter(entry) if entry.id == slot => Some(entry),
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(shape) = &mut parameter.schema else {
        panic!()
    };
    shape.value = ValueSchema::Boolean;
    rebind_quality_schema(&mut a, &mut p, schema);
    bind(&a, &mut p);
    let parameter = &mut row(&mut p).parameters[0];
    parameter.recipe.codec.codec = ValueCodecKind::Boolean {
        tokens: vec![
            BooleanToken {
                token: "true".into(),
                value: true,
            },
            BooleanToken {
                token: "false".into(),
                value: false,
            },
        ],
    };
    parameter.recipe.tiers = vec![ValueTier {
        selectors: vec![ValueSelector {
            lane: ValueLane::InputBoolean,
            name: NAME.into(),
        }],
        duplicates: DuplicatePolicy::Reject,
    }];
    parameter.recipe.missing = default.map_or(MissingValuePolicy::Pending, |value| {
        MissingValuePolicy::Explicit {
            value: ParameterValue::Boolean(value),
        }
    });
    (a, p)
}

fn boolean_field(value: &str) -> String {
    format!(r#"<Input name="{NAME}" boolean="{value}"/>"#)
}

fn value(result: &NormalizedImport, index: usize) -> Option<bool> {
    let intents = &result.draft().input().scenario_presets.members[index]
        .scenario
        .usage_intents
        .members;
    if intents.is_empty() {
        return None;
    }
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].parameters.members.len(), 1);
    let ParameterValue::Boolean(value) = intents[0].parameters.members[0]
        .value
        .to_resolved()
        .unwrap()
    else {
        panic!()
    };
    Some(value)
}

fn incomplete(result: &NormalizedImport) {
    for scenario in &result.draft().input().scenario_presets.members {
        assert!(scenario.scenario.usage.members.is_empty());
        for completion in [
            &scenario.scenario.assumptions.completion,
            &scenario.scenario.usage.completion,
            &scenario.scenario.usage_intents.completion,
        ] {
            assert!(matches!(completion, DraftListCompletion::Pending { .. }));
        }
    }
    for choice in &result.draft().input().choice_presets.members {
        assert!(matches!(
            choice.choices.completion,
            DraftListCompletion::Pending { .. }
        ));
    }
}

#[test]
fn authored_false_and_constructor_intent_remain_distinct_in_independent_sets() {
    let (a, mut p) = boolean_fixture(Some(true));
    let text = format!(
        r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1">{}</ConfigSet><ConfigSet id="2">{}</ConfigSet><ConfigSet id="3"/></Config></PathOfBuilding2>"#,
        boolean_field("false"),
        boolean_field("true")
    );
    let projected = run(&text, &a, &p);
    accounting(&a, &mut p);
    let accounted = run(&text, &a, &p);
    let repeated = run(&text, &a, &p);
    assert_eq!(
        [
            value(&accounted, 0),
            value(&accounted, 1),
            value(&accounted, 2)
        ],
        [Some(false), Some(true), Some(true)]
    );
    assert_eq!(projected.draft(), accounted.draft());
    assert_eq!(projected.allocator_after(), accounted.allocator_after());
    assert_eq!(accounted.draft(), repeated.draft());
    assert_eq!(accounted.sidecar().origins, repeated.sidecar().origins);
    incomplete(&accounted);

    let imported = source(&text, 93);
    let policy = row(&mut p).policy.clone();
    let mut expected = projected.sidecar().origins.clone();
    let leaves: Vec<_> = imported
        .occurrences()
        .iter()
        .filter(|o| o.name() == "Input")
        .collect();
    assert_eq!(leaves.len(), 2); // Constructor intent invents no saved leaf.
    for (index, leaf) in leaves.iter().enumerate() {
        let DraftListCompletion::Pending { id: issue, .. } =
            accounted.draft().input().choice_presets.members[index]
                .choices
                .completion
        else {
            panic!()
        };
        let origin = &mut expected[leaf.id().ordinal() as usize];
        assert_eq!(origin.disposition, SourceDisposition::Contributes);
        assert_eq!(
            origin
                .links
                .iter()
                .filter(|link| **link == OwnedOriginTarget::Issue(issue))
                .count(),
            1
        );
        assert!(
            origin
                .links
                .contains(&OwnedOriginTarget::ScenarioUsageIntent {
                    scenario: accounted.draft().input().scenario_presets.members[index].id,
                    policy: policy.clone(),
                })
        );
        origin
            .links
            .retain(|link| *link != OwnedOriginTarget::Issue(issue));
    }
    assert_eq!(expected, accounted.sidecar().origins);
    let UsageInputPolicy::PobOccurrenceUsageV3 { scenario, .. } = p.usage_inputs.as_mut().unwrap();
    scenario.clear();
    p.configuration_dispositions = None;
    let no_projection = run(&text, &a, &p);
    assert_eq!(
        accounted.draft().input().skill_presets,
        no_projection.draft().input().skill_presets
    );
    assert_eq!(accounted.allocator_after(), no_projection.allocator_after());
}

#[test]
fn malformed_present_values_and_wrong_lanes_never_become_constructor_intent() {
    for default in [false, true] {
        let (a, p) = boolean_fixture(Some(default));
        assert_eq!(value(&run(&xml(""), &a, &p), 0), Some(default));
        for body in [
            boolean_field(""),
            boolean_field("False"),
            boolean_field(" true"),
            boolean_field("1"),
            boolean_field("nil"),
            field("Input", "0"),
            format!(r#"<Input name="{NAME}" string="false"/>"#),
            format!(r#"<Placeholder name="{NAME}" boolean="true"/>"#),
            field("Placeholder", "1"),
            boolean_field("true") + &boolean_field("false"),
            boolean_field("false") + &field("Placeholder", "1"),
            boolean_field("true").replace("/>", " future=\"x\"/>"),
            boolean_field("true").replace("/>", "><Future/></Input>"),
        ] {
            let result = run(&xml(&body), &a, &p);
            assert_eq!(value(&result, 0), None, "{body}");
            incomplete(&result);
        }
        let malformed_sibling = String::from(
            r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1"/><ConfigSet id="2"><Input name="other" boolean="nil"/></ConfigSet></Config></PathOfBuilding2>"#,
        );
        let result = run(&malformed_sibling, &a, &p);
        assert_eq!(value(&result, 0), None);
        incomplete(&result);
    }
    let (a, p) = boolean_fixture(None);
    assert_eq!(value(&run(&xml(""), &a, &p), 0), None);
    assert_eq!(
        value(&run(&xml(&boolean_field("false")), &a, &p), 0),
        Some(false)
    );
}

#[test]
fn boolean_constructor_recipe_requires_exact_typed_single_control_authority() {
    let (a, p) = boolean_fixture(Some(true));
    for mutation in 0..9 {
        let mut changed = p.clone();
        let parameter = &mut row(&mut changed).parameters[0];
        match mutation {
            0 => parameter.selection = RewardNumericSelection::NonzeroInputThenNonzeroPlaceholder,
            1 => parameter.recipe.tiers[0].selectors[0].lane = ValueLane::InputNumber,
            2 => parameter
                .recipe
                .tiers
                .push(parameter.recipe.tiers[0].clone()),
            3 => parameter.recipe.tiers[0].selectors.push(ValueSelector {
                lane: ValueLane::InputBoolean,
                name: "other".into(),
            }),
            4 => parameter.recipe.missing = MissingValuePolicy::Absent,
            5 => {
                parameter.recipe.missing = MissingValuePolicy::Explicit {
                    value: ParameterValue::Integer(BoundedInteger::new(1).unwrap()),
                }
            }
            6 => {
                if let ValueCodecKind::Boolean { tokens } = &mut parameter.recipe.codec.codec {
                    tokens[0].value = false;
                }
            }
            7 => {
                if let ValueCodecKind::Boolean { tokens } = &mut parameter.recipe.codec.codec {
                    tokens[0].token = "True".into();
                }
            }
            8 => parameter.recipe.codec.whitespace = WhitespacePolicy::TrimAscii,
            _ => unreachable!(),
        }
        assert!(
            usage_input_tests::run_limits(&xml(""), &a, &changed, Default::default()).is_err(),
            "{mutation}"
        );
    }
    let before = run(&xml(&boolean_field("false")), &a, &p);
    let limits = NormalizationLimits {
        max_work: 1,
        ..Default::default()
    };
    assert!(usage_input_tests::run_limits(&xml(""), &a, &p, limits).is_err());
    let after = run(&xml(&boolean_field("false")), &a, &p);
    assert_eq!(before.draft(), after.draft());
    assert_eq!(before.sidecar().origins, after.sidecar().origins);
}

#[test]
fn boolean_intent_retains_nonempty_partial_applicability_and_unmapped_supply() {
    let (mut a, mut p) = boolean_fixture(Some(true));
    let policy = row(&mut p).policy.clone();
    let mut schema = a.schema.input().clone();
    let supplied = schema
        .definitions
        .iter_mut()
        .find_map(|definition| {
            let DefinitionDescriptor::UsagePolicy(entry) = definition else {
                return None;
            };
            if entry.id != policy {
                return None;
            }
            let SchemaState::Known(shape) = &mut entry.schema else {
                panic!()
            };
            let applicability = shape.scenario_skill_applicability.as_mut().unwrap();
            applicability.closure = SchemaClosure::Partial {
                gaps: vec![SchemaGap {
                    subject: subject(&policy),
                    facet: SchemaFacet::StaticLinks,
                    code: key("parent-path-not-reviewed"),
                }],
            };
            let [UsageSkillApplicability::Supplied(supply)] = applicability.members.as_slice()
            else {
                panic!()
            };
            Some(supply.as_ref().clone())
        })
        .unwrap();
    for slot in &mut schema.slots {
        if let SlotDescriptor::SkillGrant(entry) = slot
            && entry.id == supplied.supply
        {
            entry.schema = SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: SchemaSubject::Slot(SkillGrantSlotDefId::address(&entry.id)),
                    facet: SchemaFacet::StaticLinks,
                    code: key("supply-still-unmapped"),
                }],
            };
        }
    }
    rebind_quality_schema(&mut a, &mut p, schema);
    bind(&a, &mut p);
    accounting(&a, &mut p);
    let schema_before = a.schema.input().clone();
    for body in [String::new(), boolean_field("false")] {
        let result = run(&xml(&body), &a, &p);
        assert_eq!(value(&result, 0), Some(body.is_empty()));
        incomplete(&result);
        assert_eq!(a.schema.input(), &schema_before);
        assert!(matches!(
            a.schema.slot(&supplied.supply),
            SchemaLookup::Unmapped(_)
        ));
    }
}

#[test]
fn constructor_intent_does_not_replace_a_missing_parameter_responsibility() {
    let (mut a, mut p) = boolean_fixture(Some(true));
    let first = row(&mut p).clone();
    let slot: DeclaredSlot<ParameterSlotDefId> = a
        .registry
        .allocate_slot(SlotOwnerDefId::UsagePolicy(first.policy.clone()))
        .unwrap();
    let mut schema = a.schema.input().clone();
    let mut descriptor = schema
        .slots
        .iter()
        .find_map(|entry| match entry {
            SlotDescriptor::Parameter(entry) if entry.id == first.parameters[0].slot => {
                Some(entry.clone())
            }
            _ => None,
        })
        .unwrap();
    descriptor.id = slot.clone();
    schema.slots.push(SlotDescriptor::Parameter(descriptor));
    let shape = schema
        .definitions
        .iter_mut()
        .find_map(|entry| match entry {
            DefinitionDescriptor::UsagePolicy(entry) if entry.id == first.policy => {
                Some(&mut entry.schema)
            }
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(shape) = shape else {
        panic!()
    };
    shape.declarations.parameters.members.push(slot.clone());
    rebind_quality_schema(&mut a, &mut p, schema);
    bind(&a, &mut p);
    let mut second = first.parameters[0].clone();
    second.slot = slot;
    second.recipe.id = key("independent-boolean-responsibility");
    second.recipe.tiers[0].selectors[0].name = "independent-control".into();
    second.recipe.missing = MissingValuePolicy::Pending;
    row(&mut p).parameters.push(second);
    let missing = run(&xml(""), &a, &p);
    assert!(
        missing.draft().input().scenario_presets.members[0]
            .scenario
            .usage_intents
            .members
            .is_empty()
    );
    incomplete(&missing);
    let complete = run(
        &xml(r#"<Input name="independent-control" boolean="false"/>"#),
        &a,
        &p,
    );
    let parameters = &complete.draft().input().scenario_presets.members[0]
        .scenario
        .usage_intents
        .members[0]
        .parameters
        .members;
    assert_eq!(parameters.len(), 2);
    assert_eq!(
        parameters[0].value.to_resolved(),
        Some(ParameterValue::Boolean(true))
    );
    assert_eq!(
        parameters[1].value.to_resolved(),
        Some(ParameterValue::Boolean(false))
    );
    incomplete(&complete);
}
