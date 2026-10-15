//! Actual reporting declarations, rebound only to this finite world's namespace.
use super::*;
use serde_json::{Value, json};
const POLICY: &str = "def.000000000000338e";
const PARAMETER: &str = "def.000000000000338f";
fn namespace(value: &mut Value) {
    match value {
        Value::Object(o) => {
            if let Some(n) = o.get_mut("namespace") {
                assert_eq!(*n, json!({"game":"poe2","version":"owned-mechanics-v1"}));
                *n = json!(ns());
            }
            for v in o.values_mut() {
                namespace(v);
            }
        }
        Value::Array(a) => {
            for v in a {
                namespace(v);
            }
        }
        _ => {}
    }
}
fn install(f: &mut Fixture) {
    let mut e: Value = serde_json::from_slice(include_bytes!(
        "../../../../data/owned/poe2/3887ae68/reporting-inclusion/extension.json"
    ))
    .unwrap();
    namespace(&mut e);
    assert_eq!(e["schema"].as_array().unwrap().len(), 2);
    f.schema
        .definitions
        .push(serde_json::from_value(e["schema"][0]["value"].clone()).unwrap());
    f.schema
        .slots
        .push(serde_json::from_value(e["schema"][1]["value"].clone()).unwrap());
    let owner: DefinitionRules = serde_json::from_value(e["owners"][0].clone()).unwrap();
    assert!(owner.programs.is_complete() && owner.programs.members.is_empty());
    let mut n: Value = serde_json::from_slice(include_bytes!(
        "../../../../data/owned/poe2/3887ae68/reporting-inclusion/native.json"
    ))
    .unwrap();
    namespace(&mut n);
    let support: SupportSourceDomainDeclaration =
        serde_json::from_value(n["support"].clone()).unwrap();
    assert_eq!(support.owner, owner.owner);
    assert_eq!(
        support.domain,
        SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly)
    );
    f.owners.push(owner);
    assert_eq!(
        assignment_only_domains(&f.owners).providers.last(),
        Some(&support),
        "the finite fixture generates the exact authored support declaration"
    );
}
fn preference(target: UsageTarget, included: bool) -> UsagePolicySelection {
    UsagePolicySelection {
        policy: def(POLICY),
        target,
        parameters: vec![ParameterAssignment {
            slot: parameter(SlotOwnerDefId::UsagePolicy(def(POLICY)), PARAMETER),
            value: ParameterValue::Boolean(included),
        }],
    }
}
fn ordinary_equal(a: &OwnedEffectsReport, b: &OwnedEffectsReport) {
    assert_eq!(a.gaps, b.gaps);
    assert_eq!(a.effects, b.effects);
    assert_eq!(a.values, b.values);
    assert_eq!(a.application_groups, b.application_groups);
}
#[test]
fn reporting_intent_retains_identity_without_any_gameplay_invocation() {
    let mut f = base();
    install(&mut f);
    f.scenario.usage = vec![usage("skill-usage", authored(20), true)];
    let baseline = evaluate(&f);
    f.scenario.usage.push(preference(authored(20), false));
    f.scenario.usage.push(preference(authored(21), true));
    let a = f.compile().unwrap();
    assert!(a.gaps().is_empty());
    assert_eq!(a.request().scenario(), f.request().scenario());
    assert_eq!(a.request().queries().input(), &f.queries);
    let expected = a.evaluate(&mut a.new_scratch()).unwrap();
    // Canonical usage order inserts the metadata record before this existing
    // activation. Its provenance position changes; its actual invocation,
    // recipient and computed value must otherwise be exactly preserved.
    let mut positional_inverse = expected.clone();
    assert_eq!(positional_inverse.effects.len(), 1);
    assert_eq!(
        baseline.effects[0].key.invocation.origin,
        RuleOrigin::Usage { index: 0 }
    );
    assert_eq!(
        positional_inverse.effects[0].key.invocation.origin,
        RuleOrigin::Usage { index: 1 }
    );
    positional_inverse.effects[0].key.invocation.origin = RuleOrigin::Usage { index: 0 };
    ordinary_equal(&baseline, &positional_inverse);
    f.scenario.usage[1].parameters[0].value = ParameterValue::Boolean(true);
    let b = f.compile().unwrap();
    assert_ne!(a.bindings().request, b.bindings().request);
    assert_ne!(a.identity(), b.identity());
    let mut scratch = a.new_scratch();
    assert_eq!(a.evaluate(&mut scratch).unwrap(), expected);
    ordinary_equal(&expected, &b.evaluate(&mut scratch).unwrap());
    assert_eq!(a.evaluate(&mut scratch).unwrap(), expected);
    let reports: Vec<_> = (0..16)
        .into_par_iter()
        .map(|_| a.evaluate(&mut a.new_scratch()).unwrap())
        .collect();
    assert!(reports.iter().all(|r| r == &expected));
    assert_eq!(
        activation_effects(&expected),
        1,
        "only the independently configured activation policy"
    );
}
#[test]
fn reporting_intent_preserves_generated_disabled_and_required_target_validation() {
    let mut f = generated_fixture();
    install(&mut f);
    let baseline = evaluate(&f);
    f.scenario.usage = vec![
        preference(UsageTarget::Skill(generated(30)), true),
        preference(UsageTarget::Skill(generated(31)), false),
    ];
    ordinary_equal(&baseline, &evaluate(&f));
    f.build.gems[0].parameters[0].value = ParameterValue::Boolean(false);
    let disabled = evaluate(&f);
    assert_eq!(activation_effects(&disabled), 0);
    f.scenario.usage.clear();
    ordinary_equal(&disabled, &evaluate(&f));
    for target in [authored(99), UsageTarget::Skill(generated(99))] {
        f.scenario.usage = vec![preference(target, true)];
        assert!(matches!(f.compile(), Err(PlanError::Invalid(_))));
    }
    f.scenario.usage = vec![preference(authored(20), true)];
    f.scenario.usage[0].parameters[0].value = integer(1);
    assert!(matches!(f.compile(), Err(PlanError::Invalid(_))));
    f.scenario.usage = vec![preference(authored(20), true)];
    f.scenario.usage[0].parameters.clear();
    assert!(matches!(f.compile(), Err(PlanError::Invalid(_))));
    f.scenario.usage = vec![preference(authored(20), true)];
    f.owners.retain(|o| o.owner != policy_owner(POLICY));
    assert!(
        evaluate(&f)
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::MissingPrograms)
    );
}

#[test]
fn reporting_empty_gameplay_owner_still_requires_its_support_domain() {
    let mut f = base();
    install(&mut f);
    f.scenario.usage = vec![preference(authored(20), true)];
    let schema = Arc::new(
        OwnedDefinitionSchemaPackage::new(f.schema.clone(), OwnedSchemaLimits::default()).unwrap(),
    );
    let routing = Arc::new(
        OwnedActionRouting::new(
            ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: ns(),
                release: key("routing"),
                definitions: schema.identity().clone(),
                outputs: f.routes.clone(),
            },
            schema.as_ref(),
            RoutingLimits::default(),
        )
        .unwrap(),
    );
    for (unknown, reason) in [
        (false, PlanGapReason::MissingSupportSources),
        (true, PlanGapReason::UnmappedSupportSources),
    ] {
        let mut domains = assignment_only_domains(&f.owners);
        let row = domains
            .providers
            .iter()
            .position(|r| r.owner == policy_owner(POLICY))
            .unwrap();
        if unknown {
            domains.providers[row].domain = SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: policy_owner(POLICY),
                    facet: SchemaFacet::GameRules,
                    code: key("unproved-reporting-support-domain"),
                }],
            };
        } else {
            domains.providers.remove(row);
        }
        let rules = Arc::new(
            CompiledRulePackage::compile(
                &RulePackageInput {
                    support_discovery: Some(domains),
                    existing_actor_rules: None,
                    contribution_queries: None,
                    effect_applications: None,
                    receivers: f.receivers.clone(),
                    tables: f.tables.clone(),
                    schema_version: OWNED_RULE_PACKAGE_VERSION,
                    namespace: ns(),
                    release: key("rules"),
                    semantics_version: key("test-v1"),
                    operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
                    definitions: schema.identity().clone(),
                    owners: f.owners.clone(),
                },
                schema.as_ref(),
                RuleLimits::default(),
            )
            .unwrap(),
        );
        let plan = OwnedEffectPlan::compile(
            Arc::new(f.request()),
            schema.clone(),
            rules,
            routing.clone(),
            PlanLimits::default(),
        )
        .unwrap();
        assert!(
            plan.gaps()
                .iter()
                .any(|g| g.subject == Some(policy_owner(POLICY)) && g.reason == reason)
        );
    }
}
fn project(f: &Fixture, preferred: UsagePolicySelection) -> (BuildProject, VariantSelection) {
    let b = &f.build;
    let selection = VariantSelection {
        character: occurrence(50),
        equipment: occurrence(51),
        allocations: occurrence(52),
        skills: occurrence(53),
        choices: occurrence(54),
        active_weapon_loadout: b.active_weapon_loadout,
    };
    let input = ProjectInput {
        allocator: b.allocator,
        revision: b.revision,
        game_version: b.game_version.clone(),
        weapon_loadouts: b.weapon_loadouts.clone(),
        items: b.items.clone(),
        gems: b.gems.clone(),
        rewards: b.character.rewards.clone(),
        equipment: b.equipment.clone(),
        allocations: b.allocations.clone(),
        skills: b.skills.clone(),
        supports: b.supports.clone(),
        payload_links: b.payload_links.clone(),
        character_presets: vec![CharacterPreset {
            id: selection.character,
            class: b.character.class.clone(),
            ascendancy: b.character.ascendancy.clone(),
            level: b.character.level,
            rewards: vec![],
        }],
        equipment_presets: vec![EquipmentPreset {
            id: selection.equipment,
            equipment: vec![],
        }],
        allocation_presets: vec![AllocationPreset {
            id: selection.allocations,
            allocations: vec![],
            equipment: vec![],
        }],
        skill_presets: vec![SkillPreset {
            intent: None,
            id: selection.skills,
            skills: b.skills.iter().map(|s| s.id).collect(),
            supports: vec![],
            authored_support_order: None,
            payload_links: vec![],
            usage_preferences: Some(vec![preferred]),
        }],
        choice_presets: vec![ChoicePreset {
            id: selection.choices,
            choices: vec![],
            rewards: vec![],
        }],
        saved_variants: vec![],
    };
    (
        BuildProject::new(input, OwnedInputLimits::default()).unwrap(),
        selection,
    )
}
#[test]
fn reporting_preset_intent_uses_exact_whole_record_scenario_overrides() {
    let mut f = base();
    install(&mut f);
    let (p, s) = project(&f, preference(authored(20), false));
    let compose = |scenario| {
        compose_request(
            &p,
            &s,
            None,
            ScenarioSpec::new(scenario, OwnedInputLimits::default()).unwrap(),
            QuerySpec::new(f.queries.clone(), OwnedInputLimits::default()).unwrap(),
            OwnedInputLimits::default(),
        )
        .unwrap()
    };
    let a = compose(f.scenario.clone());
    assert_eq!(
        a.scenario().input().usage,
        vec![preference(authored(20), false)]
    );
    let mut scenario = f.scenario.clone();
    scenario.usage = vec![
        preference(authored(20), true),
        preference(authored(21), false),
    ];
    let b = compose(scenario);
    assert_eq!(b.scenario().input().usage.len(), 2);
    assert!(
        b.scenario()
            .input()
            .usage
            .contains(&preference(authored(20), true))
    );
    let ap = compile_request(&f, a);
    let bp = compile_request(&f, b);
    assert_ne!(ap.bindings().request, bp.bindings().request);
    ordinary_equal(
        &ap.evaluate(&mut ap.new_scratch()).unwrap(),
        &bp.evaluate(&mut bp.new_scratch()).unwrap(),
    );
    assert_eq!(ap.request().queries(), bp.request().queries());
    assert_eq!(ap.request().build(), bp.request().build());
}
