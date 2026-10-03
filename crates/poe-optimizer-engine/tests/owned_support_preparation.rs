//! Authored native component inputs; these tests do not claim whole-build support delivery.
use poe_optimizer_core::{
    build_identity::{
        BuildLineage, BuildRevision, GemInstanceId, InstanceAllocatorState, InstanceId, SkillUseId,
        SupportAssignmentId, WeaponLoadoutId,
    },
    owned_build::*,
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
    owned_supports::*,
};
use poe_optimizer_data::{
    owned_rules::{OwnedRulePackage, RuleStorageLimits},
    owned_schema::{
        OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, OwnedSchemaLimits,
        SchemaPackageInput,
    },
    owned_supports::{OwnedSupportPreparation, SupportStorageLimits},
};
use poe_optimizer_engine::owned_supports::*;
use std::collections::BTreeSet;

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("support-test", "v1").unwrap()
}
fn def<K: DefinitionDomain>(value: &str) -> DefId<K> {
    DefId::parse(namespace(), value).unwrap()
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn instance(value: u64) -> InstanceId {
    InstanceId::from_parts(BuildLineage::from_bytes([17; 16]), value).unwrap()
}
fn predicate_type(value: &str) -> SupportTypePredicate {
    SupportTypePredicate::Type(key(value))
}
fn definition(effect: &str) -> SupportPreparationDefinition {
    SupportPreparationDefinition {
        effect: key(effect),
        families: None,
        plus_version_of: None,
        requires: None,
        excludes: None,
        added_types: vec![],
        gems_only: false,
        from_item: false,
        is_support: true,
        is_trigger: false,
        ignore_minion_types: false,
    }
}
fn collect_predicate(value: &SupportTypePredicate, types: &mut BTreeSet<OwnedDefinitionKey>) {
    match value {
        SupportTypePredicate::Type(value) => {
            types.insert(value.clone());
        }
        SupportTypePredicate::All(values) | SupportTypePredicate::Any(values) => {
            for value in values {
                collect_predicate(value, types);
            }
        }
        SupportTypePredicate::Not(value) => collect_predicate(value, types),
    }
}
fn package(definitions: Vec<(&str, SupportPreparationDefinition)>) -> OwnedSupportPreparation {
    let declarations = || DeclaredSlots {
        parameters: empty(),
        choices: empty(),
        grants: empty(),
        actors: empty(),
        skill_grants: empty(),
        outputs: empty(),
        sockets: empty(),
    };
    let mut entries = vec![DefinitionDescriptor::Unit(known(
        def("quality"),
        UnitSchema {
            dimension: UnitDimension::PercentagePoints,
        },
    ))];
    let mut supports = vec![];
    let mut types: BTreeSet<_> = [
        "attack", "spell", "duration", "minion", "immune", "a", "b", "c", "d", "seed", "unused",
    ]
    .into_iter()
    .map(key)
    .collect();
    let mut effects = BTreeSet::new();
    let mut families = BTreeSet::new();
    for (name, definition) in definitions {
        entries.push(DefinitionDescriptor::Gem(known(
            def(name),
            GemSchema {
                level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                },
                roles: vec![AuthoredGemRole::SupportAssignment],
                skills: empty(),
                quality: QualityUseSchema {
                    presence: QualityPresence::Forbidden,
                    allowed_kinds: empty(),
                },
                declarations: declarations(),
            },
        )));
        effects.insert(definition.effect.clone());
        effects.extend(definition.plus_version_of.iter().cloned());
        types.extend(definition.added_types.iter().cloned());
        for predicate in [&definition.requires, &definition.excludes]
            .into_iter()
            .flatten()
        {
            collect_predicate(predicate, &mut types);
        }
        families.extend(definition.families.iter().flatten().cloned());
        supports.push(SupportPreparationEntry {
            gem: def(name),
            preparation: SchemaState::Known(definition),
        });
    }
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: namespace(),
            release: key("schema"),
            semantics_version: key("v1"),
            definitions: entries,
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let rules = OwnedRulePackage::new(
        RulePackageInput {
            effect_applications: None,
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: namespace(),
            release: key("rules"),
            semantics_version: key("v1"),
            operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
            definitions: schema.identity().clone(),
            tables: vec![],
            owners: vec![],
            receivers: empty(),
        },
        &schema,
        RuleStorageLimits::default(),
    )
    .unwrap();
    OwnedSupportPreparation::new(
        SupportPreparationInput {
            schema_version: OWNED_SUPPORT_PREPARATION_VERSION,
            namespace: namespace(),
            release: key("supports"),
            definitions: schema.identity().clone(),
            rules: *rules.identity(),
            policy: SupportPreparationPolicy::OrderedReplacementRetryFrontierV1,
            quality_unit: def("quality"),
            types: types.into_iter().collect(),
            effects: effects.into_iter().collect(),
            families: families.into_iter().collect(),
            supports,
        },
        &schema,
        &rules,
        SupportStorageLimits::default(),
    )
    .unwrap()
}
fn origin(id: u64, gem: &str, level: i64, quality: f64) -> ResolvedSupportOrigin {
    ResolvedSupportOrigin {
        assignment: SupportAssignmentId::from_instance_id(instance(id)),
        gem: def(gem),
        enabled: Some(true),
        effective_level: Some(BoundedInteger::new(level).unwrap()),
        effective_quality: Some(FiniteQuantity::new(quality, def("quality")).unwrap()),
    }
}
fn types(values: &[&str]) -> DeclaredSet<OwnedDefinitionKey> {
    DeclaredSet::complete(values.iter().copied().map(key).collect())
}
fn target() -> SupportPreparationTarget {
    SupportPreparationTarget {
        target: SkillTarget::Authored(SkillUseId::from_instance_id(instance(100))),
        enabled: Some(true),
        types: SupportTypeContext {
            skill_types: types(&["spell"]),
            minion_types: None,
        },
        summoner: None,
        cannot_be_supported: Some(false),
        has_gem: Some(true),
        from_item: Some(false),
        is_player_actor: Some(true),
    }
}
fn prepare(
    package: &OwnedSupportPreparation,
    origins: &[ResolvedSupportOrigin],
    target: &SupportPreparationTarget,
) -> SupportPreparationOutcome {
    prepare_supports(
        package,
        origins,
        target,
        SupportPreparationLimits::default(),
    )
    .unwrap()
}
fn result(
    package: &OwnedSupportPreparation,
    origins: &[ResolvedSupportOrigin],
    target: &SupportPreparationTarget,
) -> PreparedSupports {
    match prepare(package, origins, target) {
        SupportPreparationOutcome::Known(result) => result,
        other => panic!("expected known component preparation: {other:?}"),
    }
}
fn positions(result: &PreparedSupports) -> Vec<usize> {
    result
        .selected
        .iter()
        .map(|position| position.origin_index)
        .collect()
}

#[test]
fn effective_level_then_quality_then_first_encounter_controls_same_effect() {
    let package = package(vec![("one", definition("effect"))]);
    let origins = [
        origin(8, "one", 1, 10.0),
        origin(2, "one", 1, 10.0),
        origin(9, "one", 1, 20.0),
        origin(1, "one", 2, 0.0),
    ];
    assert_eq!(
        positions(&result(&package, &origins[..2], &target())),
        vec![0]
    );
    assert_eq!(
        positions(&result(&package, &origins[..3], &target())),
        vec![2]
    );
    let prepared = result(&package, &origins, &target());
    assert_eq!(positions(&prepared), vec![3]);
    assert_eq!(
        prepared.ordered_origins,
        origins
            .iter()
            .map(|origin| origin.assignment)
            .collect::<Vec<_>>()
    );
    assert_eq!(prepared.preparation, *package.identity());
    assert_eq!(prepared.target, target().target);
    assert_eq!(
        positions(&result(
            &package,
            &[origins[1].clone(), origins[0].clone()],
            &target()
        )),
        vec![0]
    );
}

#[test]
fn family_presence_suppresses_plus_handling_and_later_family_member_wins() {
    let mut base = definition("base");
    let mut plus = definition("plus");
    plus.plus_version_of = Some(key("base"));
    let origins = [origin(1, "base", 10, 99.0), origin(2, "plus", 1, 0.0)];
    let pkg = package(vec![("base", base.clone()), ("plus", plus.clone())]);
    assert_eq!(positions(&result(&pkg, &origins, &target())), vec![1]);
    assert_eq!(
        positions(&result(
            &pkg,
            &[origins[1].clone(), origins[0].clone()],
            &target()
        )),
        vec![0]
    );
    base.families = Some(vec![]);
    plus.families = Some(vec![]);
    let pkg = package(vec![("base", base.clone()), ("plus", plus.clone())]);
    assert_eq!(positions(&result(&pkg, &origins, &target())), vec![0, 1]);
    base.families = Some(vec![key("shared")]);
    plus.families = Some(vec![key("shared")]);
    let pkg = package(vec![("base", base), ("plus", plus)]);
    assert_eq!(
        positions(&result(
            &pkg,
            &[origins[1].clone(), origins[0].clone()],
            &target()
        )),
        vec![1]
    );
}

#[test]
fn replacement_multiplicity_and_same_effect_first_match_are_preserved() {
    let mut first = definition("first");
    first.families = Some(vec![key("a")]);
    let mut second = definition("second");
    second.families = Some(vec![key("b")]);
    let mut combined = definition("combined");
    combined.families = Some(vec![key("a"), key("b")]);
    let pkg = package(vec![
        ("first", first),
        ("second", second),
        ("combined", combined),
    ]);
    let origins = [
        origin(1, "first", 1, 0.0),
        origin(2, "second", 1, 0.0),
        origin(3, "combined", 1, 0.0),
        origin(4, "combined", 2, 0.0),
    ];
    let duplicated = result(&pkg, &origins[..3], &target());
    assert_eq!(positions(&duplicated), vec![2, 2]);
    assert_eq!(
        duplicated.selected[0].assignment,
        duplicated.selected[1].assignment
    );
    assert_ne!(
        duplicated.selected[0].position,
        duplicated.selected[1].position
    );
    assert_eq!(positions(&result(&pkg, &origins, &target())), vec![3, 2]);
}

#[test]
fn unknown_selection_inputs_are_demanded_without_raw_gem_fallback() {
    let pkg = package(vec![("one", definition("one"))]);
    let first = origin(1, "one", 1, 0.0);
    let mut second = origin(2, "one", 2, 0.0);
    second.effective_level = None;
    assert!(matches!(
        prepare(&pkg, &[first.clone(), second.clone()], &target()),
        SupportPreparationOutcome::Unresolved {
            reason: SupportPreparationGap::EffectiveLevel,
            origin_index: Some(1)
        }
    ));
    second.effective_level = Some(BoundedInteger::new(1).unwrap());
    second.effective_quality = None;
    assert!(matches!(
        prepare(&pkg, &[first.clone(), second.clone()], &target()),
        SupportPreparationOutcome::Unresolved {
            reason: SupportPreparationGap::EffectiveQuality,
            origin_index: Some(1)
        }
    ));
    second.effective_level = Some(BoundedInteger::new(2).unwrap());
    // Quality is not needed when the effective levels differ.
    assert_eq!(
        positions(&result(&pkg, &[first, second], &target())),
        vec![1]
    );
}

#[test]
fn disabled_origins_and_targets_do_not_become_selection_participants() {
    let pkg = package(vec![("one", definition("one"))]);
    let mut disabled = origin(1, "missing", 1, 0.0);
    disabled.enabled = Some(false);
    disabled.effective_level = None;
    disabled.effective_quality = None;
    let origins = [disabled, origin(2, "one", 1, 0.0)];
    let prepared = result(&pkg, &origins, &target());
    assert_eq!(prepared.disabled_origins, vec![0]);
    assert_eq!(positions(&prepared), vec![1]);
    assert_eq!(prepared.ordered_origins.len(), 2);
    let mut target = target();
    target.enabled = Some(false);
    assert!(matches!(
        prepare(&pkg, &origins, &target),
        SupportPreparationOutcome::Inactive { .. }
    ));
    target.enabled = None;
    assert!(matches!(
        prepare(&pkg, &origins, &target),
        SupportPreparationOutcome::Unresolved {
            reason: SupportPreparationGap::TargetEnabled,
            ..
        }
    ));
}

#[test]
fn missing_origin_facts_remain_explicit() {
    let pkg = package(vec![("one", definition("one"))]);
    assert!(matches!(
        prepare(&pkg, &[origin(1, "missing", 1, 0.0)], &target()),
        SupportPreparationOutcome::Unresolved {
            reason: SupportPreparationGap::MissingDefinition,
            ..
        }
    ));
    let mut unknown = origin(1, "one", 1, 0.0);
    unknown.enabled = None;
    assert!(matches!(
        prepare(&pkg, &[unknown], &target()),
        SupportPreparationOutcome::Unresolved {
            reason: SupportPreparationGap::OriginEnabled,
            ..
        }
    ));
}

#[test]
fn retry_frontier_retains_known_pinned_order_dependence() {
    let pair = |name: &str, required: &str, added: &str| {
        let mut value = definition(name);
        value.requires = Some(predicate_type(required));
        value.added_types = vec![key(added)];
        value
    };
    let pkg = package(vec![
        ("first", pair("first", "duration", "a")),
        ("delayed", pair("delayed", "b", "c")),
        ("producer", pair("producer", "duration", "b")),
        ("consumer", pair("consumer", "c", "d")),
        ("seed", pair("seed", "spell", "duration")),
    ]);
    let make = |order: &[&str]| {
        order
            .iter()
            .enumerate()
            .map(|(index, gem)| origin(index as u64 + 1, gem, 1, 0.0))
            .collect::<Vec<_>>()
    };
    let sparse = result(
        &pkg,
        &make(&["first", "delayed", "producer", "consumer", "seed"]),
        &target(),
    );
    assert!(sparse.final_types.contains(&key("b")));
    assert!(!sparse.final_types.contains(&key("c")));
    assert!(sparse.selected[1].applicable);
    assert!(!sparse.selected[1].type_additions_applied);
    assert!(!sparse.selected[3].applicable);
    let contiguous = result(
        &pkg,
        &make(&["producer", "delayed", "first", "consumer", "seed"]),
        &target(),
    );
    assert!(contiguous.final_types.contains(&key("c")));
    assert!(
        contiguous
            .selected
            .iter()
            .all(|position| position.applicable)
    );
}

#[test]
fn final_recheck_keeps_types_added_by_a_later_inapplicable_support() {
    let mut early = definition("early");
    early.excludes = Some(predicate_type("immune"));
    early.added_types = vec![key("duration")];
    let mut later = definition("later");
    later.added_types = vec![key("immune")];
    let mut consumer = definition("consumer");
    consumer.requires = Some(predicate_type("duration"));
    let pkg = package(vec![
        ("early", early),
        ("later", later),
        ("consumer", consumer),
    ]);
    let prepared = result(
        &pkg,
        &[
            origin(1, "early", 1, 0.0),
            origin(2, "later", 1, 0.0),
            origin(3, "consumer", 1, 0.0),
        ],
        &target(),
    );
    assert!(!prepared.selected[0].applicable);
    assert!(prepared.selected[0].type_additions_applied);
    assert!(prepared.final_types.contains(&key("duration")));
    assert!(prepared.selected[2].applicable);
}

#[test]
fn requirements_union_minion_types_but_exclusions_use_only_effective_skill_types() {
    let mut support = definition("one");
    support.requires = Some(predicate_type("attack"));
    support.excludes = Some(predicate_type("attack"));
    let pkg = package(vec![("one", support.clone())]);
    let mut target = target();
    target.types.minion_types = Some(types(&["attack"]));
    assert!(result(&pkg, &[origin(1, "one", 1, 0.0)], &target).selected[0].applicable);
    support.ignore_minion_types = true;
    let pkg = package(vec![("one", support)]);
    assert!(!result(&pkg, &[origin(1, "one", 1, 0.0)], &target).selected[0].applicable);
}

#[test]
fn summoner_scope_is_explicit_and_child_type_additions_do_not_modify_it() {
    let mut producer = definition("producer");
    producer.requires = Some(predicate_type("spell"));
    producer.added_types = vec![key("duration")];
    let mut consumer = definition("consumer");
    consumer.requires = Some(predicate_type("duration"));
    let pkg = package(vec![("producer", producer), ("consumer", consumer)]);
    let mut target = target();
    target.types.skill_types = types(&["attack"]);
    target.summoner = Some(SupportTypeContext {
        skill_types: types(&["spell"]),
        minion_types: None,
    });
    target.is_player_actor = Some(false);
    let prepared = result(
        &pkg,
        &[origin(1, "producer", 1, 0.0), origin(2, "consumer", 1, 0.0)],
        &target,
    );
    assert!(prepared.final_types.contains(&key("duration")));
    assert!(prepared.selected[0].applicable);
    assert!(!prepared.selected[1].applicable);
    assert_eq!(
        target.summoner.unwrap().skill_types.members,
        vec![key("spell")]
    );
}

#[test]
fn absent_summoner_minion_types_fall_back_but_present_empty_types_do_not() {
    let mut support = definition("one");
    support.requires = Some(predicate_type("attack"));
    let pkg = package(vec![("one", support)]);
    let mut target = target();
    target.types.minion_types = Some(types(&["attack"]));
    target.summoner = Some(SupportTypeContext {
        skill_types: types(&["spell"]),
        minion_types: None,
    });
    assert!(result(&pkg, &[origin(1, "one", 1, 0.0)], &target).selected[0].applicable);
    target.summoner.as_mut().unwrap().minion_types = Some(empty());
    assert!(!result(&pkg, &[origin(1, "one", 1, 0.0)], &target).selected[0].applicable);
}

#[test]
fn empty_other_family_list_does_not_make_outer_work_free() {
    let mut first = definition("first");
    first.families = Some(vec![]);
    let mut incoming = definition("incoming");
    incoming.families = Some(
        (0..2000)
            .map(|index| key(&format!("family-{index}")))
            .collect(),
    );
    let pkg = package(vec![("first", first), ("incoming", incoming)]);
    let origins = [origin(1, "first", 1, 0.0), origin(2, "incoming", 1, 0.0)];
    assert_eq!(
        prepare_supports(
            &pkg,
            &origins,
            &target(),
            SupportPreparationLimits {
                max_work: 100,
                ..Default::default()
            }
        ),
        Err(SupportPreparationError::Limit("work"))
    );
}

#[test]
fn origin_flags_and_missing_target_facts_are_not_inferred_from_names() {
    let mut support = definition("one");
    support.gems_only = true;
    let mut target = target();
    target.has_gem = None;
    let pkg = package(vec![("one", support.clone())]);
    assert!(matches!(
        prepare(&pkg, &[origin(1, "one", 1, 0.0)], &target),
        SupportPreparationOutcome::Unresolved {
            reason: SupportPreparationGap::Applicability,
            ..
        }
    ));
    target.cannot_be_supported = Some(true);
    assert!(!result(&pkg, &[origin(1, "one", 1, 0.0)], &target).selected[0].applicable);
    target.cannot_be_supported = Some(false);
    target.has_gem = Some(false);
    assert!(!result(&pkg, &[origin(1, "one", 1, 0.0)], &target).selected[0].applicable);
    support.gems_only = false;
    support.from_item = true;
    target.from_item = Some(true);
    let pkg = package(vec![("one", support.clone())]);
    assert!(!result(&pkg, &[origin(1, "one", 1, 0.0)], &target).selected[0].applicable);
    support.is_support = false;
    let pkg = package(vec![("one", support.clone())]);
    assert!(result(&pkg, &[origin(1, "one", 1, 0.0)], &target).selected[0].applicable);
    support.is_trigger = true;
    target.is_player_actor = Some(false);
    let pkg = package(vec![("one", support)]);
    assert!(!result(&pkg, &[origin(1, "one", 1, 0.0)], &target).selected[0].applicable);
}

#[test]
fn partial_type_absence_stays_unknown_but_positive_membership_and_false_guards_resolve() {
    let mut support = definition("one");
    support.requires = Some(predicate_type("attack"));
    let mut target = target();
    target.types.skill_types = DeclaredSet::partial(vec![key("spell")], vec![]);
    let pkg = package(vec![("one", support.clone())]);
    assert!(matches!(
        prepare(&pkg, &[origin(1, "one", 1, 0.0)], &target),
        SupportPreparationOutcome::Unresolved {
            reason: SupportPreparationGap::Applicability,
            ..
        }
    ));
    target.types.skill_types.members.push(key("attack"));
    let prepared = result(&pkg, &[origin(1, "one", 1, 0.0)], &target);
    assert!(prepared.selected[0].applicable);
    assert!(!prepared.final_types_complete);
    support.requires = Some(SupportTypePredicate::All(vec![
        predicate_type("unused"),
        SupportTypePredicate::Any(vec![]),
    ]));
    let pkg = package(vec![("one", support)]);
    assert!(!result(&pkg, &[origin(1, "one", 1, 0.0)], &target).selected[0].applicable);
}

#[test]
fn rejects_duplicate_origins_bad_units_unknown_types_and_foreign_namespaces() {
    let pkg = package(vec![("one", definition("one"))]);
    let origin = origin(1, "one", 1, 0.0);
    assert!(matches!(
        prepare_supports(
            &pkg,
            &[origin.clone(), origin.clone()],
            &target(),
            SupportPreparationLimits::default()
        ),
        Err(SupportPreparationError::Invalid(
            "duplicate support origin assignment"
        ))
    ));
    let mut bad = origin.clone();
    bad.effective_quality = Some(FiniteQuantity::new(0.0, def("wrong-unit")).unwrap());
    assert!(matches!(
        prepare_supports(&pkg, &[bad], &target(), SupportPreparationLimits::default()),
        Err(SupportPreparationError::Invalid(
            "effective support quality unit differs from package"
        ))
    ));
    let mut bad = origin.clone();
    bad.gem = DefId::parse(GameVersionNamespace::new("foreign", "v1").unwrap(), "one").unwrap();
    assert!(matches!(
        prepare_supports(&pkg, &[bad], &target(), SupportPreparationLimits::default()),
        Err(SupportPreparationError::Invalid(
            "foreign support origin namespace"
        ))
    ));
    let mut bad = target();
    bad.types.skill_types.members.push(key("not-declared"));
    assert!(matches!(
        prepare_supports(&pkg, &[origin], &bad, SupportPreparationLimits::default()),
        Err(SupportPreparationError::Invalid(
            "target references an undeclared support type"
        ))
    ));
}

#[test]
fn work_origin_type_and_predicate_bounds_reject_before_expansion() {
    let mut support = definition("one");
    support.requires = Some(SupportTypePredicate::Not(Box::new(
        SupportTypePredicate::Not(Box::new(predicate_type("spell"))),
    )));
    let pkg = package(vec![("one", support)]);
    let origins = [origin(1, "one", 1, 0.0), origin(2, "one", 1, 0.0)];
    for (limits, expected) in [
        (
            SupportPreparationLimits {
                max_origins: 1,
                ..Default::default()
            },
            "origins",
        ),
        (
            SupportPreparationLimits {
                max_work: 1,
                ..Default::default()
            },
            "work",
        ),
        (
            SupportPreparationLimits {
                max_predicate_depth: 1,
                ..Default::default()
            },
            "predicate depth",
        ),
    ] {
        assert_eq!(
            prepare_supports(&pkg, &origins, &target(), limits),
            Err(SupportPreparationError::Limit(expected))
        );
    }
    let mut target = target();
    target.types.skill_types.members.push(key("attack"));
    assert_eq!(
        prepare_supports(
            &pkg,
            &origins,
            &target,
            SupportPreparationLimits {
                max_types: 1,
                ..Default::default()
            }
        ),
        Err(SupportPreparationError::Limit("types"))
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn shared_package_preparation_is_repeatable_and_parallel_without_shared_scratch() {
    use rayon::prelude::*;
    let mut support = definition("one");
    support.added_types = vec![key("duration")];
    let pkg = package(vec![("one", support)]);
    let origins = [origin(1, "one", 1, 0.0)];
    let target = target();
    let expected = result(&pkg, &origins, &target);
    let observed: Vec<_> = (0..32)
        .into_par_iter()
        .map(|_| result(&pkg, &origins, &target))
        .collect();
    assert!(observed.iter().all(|value| value == &expected));
    assert_eq!(result(&pkg, &origins, &target), expected);
    assert_eq!(target.types.skill_types.members, vec![key("spell")]);
}

fn build_input(origins: &[ResolvedSupportOrigin]) -> BuildInput {
    let target = target().target;
    BuildInput {
        allocator: InstanceAllocatorState::from_parts(BuildLineage::from_bytes([17; 16]), 1000),
        revision: BuildRevision::from_u64(1),
        game_version: namespace(),
        character: CharacterSpec {
            class: def("class"),
            ascendancy: None,
            level: 90,
            rewards: vec![],
        },
        weapon_loadouts: vec![WeaponLoadoutId::from_instance_id(instance(1))],
        active_weapon_loadout: WeaponLoadoutId::from_instance_id(instance(1)),
        items: vec![],
        equipment: vec![],
        allocations: vec![],
        gems: origins
            .iter()
            .enumerate()
            .map(|(index, origin)| GemInstance {
                id: GemInstanceId::from_instance_id(instance(200 + index as u64)),
                definition: origin.gem.clone(),
                parameters: vec![],
                // These deliberately differ from effective values. They have no
                // authority at this preparation boundary.
                level: 99 - index as u16,
                quality: None,
            })
            .collect(),
        skills: vec![SkillUse {
            parameters: None,
            id: SkillUseId::from_instance_id(instance(100)),
            source: AuthoredSkillSource::Direct(def("skill")),
            enabled: true,
            scope: LoadoutScope::Shared,
        }],
        supports: origins
            .iter()
            .enumerate()
            .map(|(index, origin)| SupportAssignment {
                id: origin.assignment,
                support: GemInstanceId::from_instance_id(instance(200 + index as u64)),
                target: target.clone(),
                enabled: origin.enabled.unwrap(),
            })
            .collect(),
        support_origins: Some(vec![SupportOriginSequence {
            target,
            origins: origins
                .iter()
                .map(|origin| SupportOrigin::Assignment(origin.assignment))
                .collect(),
        }]),
        payload_links: vec![],
        choices: vec![],
    }
}
fn effective(origins: &[ResolvedSupportOrigin]) -> Vec<EffectiveSupportValues> {
    origins
        .iter()
        .map(|origin| EffectiveSupportValues {
            assignment: origin.assignment,
            effective_level: origin.effective_level,
            effective_quality: origin.effective_quality.clone(),
        })
        .collect()
}

#[test]
fn build_adapter_consumes_explicit_order_and_never_physical_level_or_id_sort() {
    let pkg = package(vec![("one", definition("one"))]);
    let origins = [origin(20, "one", 1, 0.0), origin(10, "one", 1, 0.0)];
    let build = BuildSpec::new(build_input(&origins), OwnedInputLimits::default()).unwrap();
    assert_eq!(build.input().supports[0].id, origins[1].assignment);
    let observed = prepare_build_supports(
        &pkg,
        &build,
        &target(),
        &effective(&origins),
        SupportPreparationLimits::default(),
    )
    .unwrap();
    assert_eq!(observed, prepare(&pkg, &origins, &target()));
    // Missing effective comparison inputs must not fall back to the two known
    // physical Gem levels in the validated build.
    assert!(matches!(
        prepare_build_supports(
            &pkg,
            &build,
            &target(),
            &[],
            SupportPreparationLimits::default()
        )
        .unwrap(),
        SupportPreparationOutcome::Unresolved {
            reason: SupportPreparationGap::EffectiveLevel,
            ..
        }
    ));
    let mut unordered = build.into_input();
    unordered.support_origins = None;
    let unordered = BuildSpec::new(unordered, OwnedInputLimits::default()).unwrap();
    assert!(matches!(
        prepare_build_supports(
            &pkg,
            &unordered,
            &target(),
            &effective(&origins),
            SupportPreparationLimits::default()
        )
        .unwrap(),
        SupportPreparationOutcome::Unresolved {
            reason: SupportPreparationGap::OriginOrder,
            ..
        }
    ));
}

#[test]
fn build_adapter_preserves_disabled_state_and_rejects_foreign_scalar_bindings() {
    let pkg = package(vec![("one", definition("one"))]);
    let mut disabled = origin(20, "not-converted", 1, 0.0);
    disabled.enabled = Some(false);
    let origins = [disabled, origin(10, "one", 1, 0.0)];
    let mut input = build_input(&origins);
    let build = BuildSpec::new(input.clone(), OwnedInputLimits::default()).unwrap();
    let observed = prepare_build_supports(
        &pkg,
        &build,
        &target(),
        &effective(&origins),
        SupportPreparationLimits::default(),
    )
    .unwrap();
    assert_eq!(observed, prepare(&pkg, &origins, &target()));
    let foreign = EffectiveSupportValues {
        assignment: SupportAssignmentId::from_instance_id(instance(999)),
        effective_level: None,
        effective_quality: None,
    };
    assert!(matches!(
        prepare_build_supports(
            &pkg,
            &build,
            &target(),
            &[foreign],
            SupportPreparationLimits::default()
        ),
        Err(SupportPreparationError::Invalid(
            "effective support input has no build assignment"
        ))
    ));
    let duplicated = effective(&[origins[0].clone(), origins[0].clone()]);
    assert!(matches!(
        prepare_build_supports(
            &pkg,
            &build,
            &target(),
            &duplicated,
            SupportPreparationLimits::default()
        ),
        Err(SupportPreparationError::Invalid(
            "duplicate effective support input"
        ))
    ));
    input.skills[0].enabled = false;
    let build = BuildSpec::new(input, OwnedInputLimits::default()).unwrap();
    assert!(matches!(
        prepare_build_supports(
            &pkg,
            &build,
            &target(),
            &[],
            SupportPreparationLimits::default()
        )
        .unwrap(),
        SupportPreparationOutcome::Inactive { .. }
    ));
}

#[test]
fn build_adapter_cannot_use_another_targets_effective_values() {
    let pkg = package(vec![("one", definition("one"))]);
    let origins = [origin(10, "one", 1, 0.0), origin(20, "one", 1, 0.0)];
    let mut input = build_input(&origins);
    let sibling = SkillTarget::Authored(SkillUseId::from_instance_id(instance(101)));
    input.skills.push(SkillUse {
        parameters: None,
        id: SkillUseId::from_instance_id(instance(101)),
        source: AuthoredSkillSource::Direct(def("skill")),
        enabled: true,
        scope: LoadoutScope::Shared,
    });
    input.supports[1].target = sibling.clone();
    input.support_origins = Some(vec![
        SupportOriginSequence {
            target: target().target,
            origins: vec![SupportOrigin::Assignment(origins[0].assignment)],
        },
        SupportOriginSequence {
            target: sibling,
            origins: vec![SupportOrigin::Assignment(origins[1].assignment)],
        },
    ]);
    let build = BuildSpec::new(input, OwnedInputLimits::default()).unwrap();
    assert!(matches!(
        prepare_build_supports(
            &pkg,
            &build,
            &target(),
            &effective(&origins),
            SupportPreparationLimits::default()
        ),
        Err(SupportPreparationError::Invalid(
            "effective support input belongs to another target"
        ))
    ));
}

#[test]
fn build_adapter_caller_cannot_activate_a_skill_in_another_loadout() {
    let pkg = package(vec![("one", definition("one"))]);
    let origins = [origin(10, "one", 1, 0.0)];
    let mut input = build_input(&origins);
    let inactive = WeaponLoadoutId::from_instance_id(instance(2));
    input.weapon_loadouts.push(inactive);
    input.skills[0].scope = LoadoutScope::Selected {
        loadouts: vec![inactive],
    };
    let build = BuildSpec::new(input, OwnedInputLimits::default()).unwrap();
    assert!(matches!(
        prepare_build_supports(
            &pkg,
            &build,
            &target(),
            &effective(&origins),
            SupportPreparationLimits::default()
        )
        .unwrap(),
        SupportPreparationOutcome::Inactive { .. }
    ));
}

#[test]
fn complete_global_order_can_prove_empty_target_membership() {
    let pkg = package(vec![("one", definition("one"))]);
    let mut input = build_input(&[]);
    input.support_origins = Some(vec![]);
    let build = BuildSpec::new(input.clone(), OwnedInputLimits::default()).unwrap();
    assert_eq!(
        prepare_build_supports(
            &pkg,
            &build,
            &target(),
            &[],
            SupportPreparationLimits::default()
        )
        .unwrap(),
        prepare(&pkg, &[], &target())
    );
    input.support_origins = None;
    let unknown = BuildSpec::new(input, OwnedInputLimits::default()).unwrap();
    assert!(matches!(
        prepare_build_supports(
            &pkg,
            &unknown,
            &target(),
            &[],
            SupportPreparationLimits::default()
        )
        .unwrap(),
        SupportPreparationOutcome::Unresolved {
            reason: SupportPreparationGap::OriginOrder,
            ..
        }
    ));
}

#[test]
fn build_adapter_preserves_generated_target_without_entering_a_guessed_grant() {
    let pkg = package(vec![("one", definition("one"))]);
    let origins = [origin(10, "one", 1, 0.0)];
    let mut target = target();
    target.target = SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: ProviderKey {
            root: ProviderRoot::SkillUse(SkillUseId::from_instance_id(instance(100))),
            grant_path: vec![DeclaredSlot {
                declaration: SlotOwnerDefId::Skill(def("skill")),
                slot: def("parent-grant"),
            }],
        },
        slot: DeclaredSlot {
            declaration: SlotOwnerDefId::Skill(def("skill")),
            slot: def("generated"),
        },
    }));
    let mut input = build_input(&origins);
    input.supports[0].target = target.target.clone();
    input.support_origins.as_mut().unwrap()[0].target = target.target.clone();
    let build = BuildSpec::new(input, OwnedInputLimits::default()).unwrap();
    let observed = prepare_build_supports(
        &pkg,
        &build,
        &target,
        &effective(&origins),
        SupportPreparationLimits::default(),
    )
    .unwrap();
    assert_eq!(observed, prepare(&pkg, &origins, &target));
    let SupportPreparationOutcome::Known(observed) = observed else {
        panic!("known component facts");
    };
    assert_eq!(observed.target, target.target);
}

#[test]
fn build_adapter_and_native_policy_share_one_work_budget() {
    let pkg = package(vec![("one", definition("one"))]);
    let origins = [origin(10, "one", 1, 0.0)];
    let build = BuildSpec::new(build_input(&origins), OwnedInputLimits::default()).unwrap();
    // Find the measured component-only boundary rather than baking a particular
    // private scan count into this contract test. Adapter work must also count.
    let minimum = (1..256)
        .find(|&max_work| {
            prepare_supports(
                &pkg,
                &origins,
                &target(),
                SupportPreparationLimits {
                    max_work,
                    ..Default::default()
                },
            )
            .is_ok()
        })
        .unwrap();
    assert_eq!(
        prepare_build_supports(
            &pkg,
            &build,
            &target(),
            &effective(&origins),
            SupportPreparationLimits {
                max_work: minimum,
                ..Default::default()
            }
        ),
        Err(SupportPreparationError::Limit("work"))
    );
}

#[test]
fn shared_attempt_budget_charges_exact_successes_and_cannot_restart_after_exhaustion() {
    let pkg = package(vec![("one", definition("one"))]);
    let origins = [origin(10, "one", 1, 0.0), origin(20, "one", 2, 0.0)];
    let limits = SupportPreparationLimits::default();
    let mut work = limits.max_work;
    let expected =
        prepare_supports_with_budget(&pkg, &origins, &target(), limits, &mut work).unwrap();
    let used = limits.max_work - work;
    assert!(used > 0);
    assert_eq!(expected, prepare(&pkg, &origins, &target()));

    let mut shared = 2 * used;
    for remaining in [used, 0] {
        assert_eq!(
            prepare_supports_with_budget(&pkg, &origins, &target(), limits, &mut shared).unwrap(),
            expected
        );
        assert_eq!(shared, remaining);
    }
    assert_eq!(
        prepare_supports_with_budget(&pkg, &origins, &target(), limits, &mut shared),
        Err(SupportPreparationError::Limit("work"))
    );
    assert_eq!(shared, 0);

    let mut short = used - 1;
    assert_eq!(
        prepare_supports_with_budget(&pkg, &origins, &target(), limits, &mut short),
        Err(SupportPreparationError::Limit("work"))
    );
    assert_eq!(short, 0);
}

#[test]
fn shared_attempt_budget_keeps_component_caps_and_charges_errors_and_early_outcomes() {
    let pkg = package(vec![("one", definition("one"))]);
    let origins = [origin(10, "one", 1, 0.0), origin(20, "one", 2, 0.0)];
    let limits = SupportPreparationLimits::default();
    let mut work = 100;
    assert_eq!(
        prepare_supports_with_budget(
            &pkg,
            &origins,
            &target(),
            SupportPreparationLimits {
                max_work: 3,
                ..limits
            },
            &mut work
        ),
        Err(SupportPreparationError::Limit("work"))
    );
    assert_eq!(work, 97);

    let before = work;
    assert!(matches!(
        prepare_supports_with_budget(
            &pkg,
            &origins,
            &target(),
            SupportPreparationLimits {
                max_work: limits.max_work + 1,
                ..limits
            },
            &mut work
        ),
        Err(SupportPreparationError::Invalid(_))
    ));
    assert_eq!(work, before);

    let duplicate = [origins[0].clone(), origins[0].clone()];
    assert_eq!(
        prepare_supports_with_budget(&pkg, &duplicate, &target(), limits, &mut work),
        Err(SupportPreparationError::Invalid(
            "duplicate support origin assignment"
        ))
    );
    assert!(work < before);
    for enabled in [None, Some(false)] {
        let mut target = target();
        target.enabled = enabled;
        let before = work;
        let observed = prepare_supports_with_budget(&pkg, &[], &target, limits, &mut work).unwrap();
        assert!(matches!(
            observed,
            SupportPreparationOutcome::Unresolved { .. }
                | SupportPreparationOutcome::Inactive { .. }
        ));
        assert!(work < before);
    }
}

#[test]
fn build_budget_accounts_for_binding_and_preparation_without_double_allowance() {
    let pkg = package(vec![("one", definition("one"))]);
    let origins = [origin(10, "one", 1, 0.0)];
    let build = BuildSpec::new(build_input(&origins), OwnedInputLimits::default()).unwrap();
    let limits = SupportPreparationLimits::default();
    let mut direct_work = limits.max_work;
    let direct =
        prepare_supports_with_budget(&pkg, &origins, &target(), limits, &mut direct_work).unwrap();
    let mut work = limits.max_work;
    let prepared = prepare_build_supports_with_budget(
        &pkg,
        &build,
        &target(),
        &effective(&origins),
        limits,
        &mut work,
    )
    .unwrap();
    assert_eq!(prepared, direct);
    assert!(work < direct_work);
    let used = limits.max_work - work;
    let mut exact = used;
    assert_eq!(
        prepare_build_supports_with_budget(
            &pkg,
            &build,
            &target(),
            &effective(&origins),
            limits,
            &mut exact
        )
        .unwrap(),
        prepared
    );
    assert_eq!(exact, 0);
    let mut short = used - 1;
    assert_eq!(
        prepare_build_supports_with_budget(
            &pkg,
            &build,
            &target(),
            &effective(&origins),
            limits,
            &mut short
        ),
        Err(SupportPreparationError::Limit("work"))
    );
    assert_eq!(short, 0);

    let mut missing_order = build_input(&origins);
    missing_order.support_origins = None;
    let missing_order = BuildSpec::new(missing_order, OwnedInputLimits::default()).unwrap();
    let mut work = 5;
    assert_eq!(
        prepare_build_supports_with_budget(&pkg, &missing_order, &target(), &[], limits, &mut work)
            .unwrap(),
        SupportPreparationOutcome::Unresolved {
            reason: SupportPreparationGap::OriginOrder,
            origin_index: None,
        }
    );
    assert!(work < 5);

    let mut duplicate_values = effective(&origins);
    duplicate_values.push(duplicate_values[0].clone());
    let mut work = limits.max_work;
    assert_eq!(
        prepare_build_supports_with_budget(
            &pkg,
            &build,
            &target(),
            &duplicate_values,
            limits,
            &mut work,
        ),
        Err(SupportPreparationError::Invalid(
            "duplicate effective support input"
        ))
    );
    assert!(work < limits.max_work);
}

#[test]
fn worker_budgets_and_failed_calls_do_not_change_subsequent_preparation() {
    use rayon::prelude::*;
    let pkg = package(vec![("one", definition("one"))]);
    let origins = [origin(10, "one", 1, 0.0), origin(20, "one", 2, 0.0)];
    let run = |_| {
        let limits = SupportPreparationLimits::default();
        let mut work = limits.max_work;
        let first =
            prepare_supports_with_budget(&pkg, &origins, &target(), limits, &mut work).unwrap();
        let first_used = limits.max_work - work;
        let invalid = [origins[0].clone(), origins[0].clone()];
        assert!(
            prepare_supports_with_budget(&pkg, &invalid, &target(), limits, &mut work).is_err()
        );
        let before = work;
        let last =
            prepare_supports_with_budget(&pkg, &origins, &target(), limits, &mut work).unwrap();
        assert_eq!(first, last);
        assert_eq!(before - work, first_used);
        (last, work)
    };
    let expected = run(0);
    let parallel: Vec<_> = (0..64).into_par_iter().map(run).collect();
    assert!(parallel.iter().all(|observed| *observed == expected));
}
