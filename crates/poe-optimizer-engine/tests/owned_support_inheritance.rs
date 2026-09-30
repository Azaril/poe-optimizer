//! Selection inheritance is component evidence, not whole-build receiving authority.
use poe_optimizer_core::{
    build_identity::{BuildLineage, InstanceId, SkillUseId, SupportAssignmentId},
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
fn selected(
    package: &OwnedSupportPreparation,
    origins: &[ResolvedSupportOrigin],
) -> SelectedSupports {
    match select_supports(package, origins, SupportPreparationLimits::default()).unwrap() {
        SupportSelectionOutcome::Known(result) => result,
        other => panic!("expected selected origins: {other:?}"),
    }
}
fn admitted(
    package: &OwnedSupportPreparation,
    selected: &SelectedSupports,
    target: &SupportPreparationTarget,
) -> PreparedSupports {
    match prepare_selected_supports(
        package,
        selected,
        target,
        SupportPreparationLimits::default(),
    )
    .unwrap()
    {
        SupportPreparationOutcome::Known(result) => result,
        other => panic!("expected prepared component: {other:?}"),
    }
}
fn child(
    parent: &SupportPreparationTarget,
    prepared: &PreparedSupports,
) -> SupportPreparationTarget {
    let mut child = target();
    child.target = SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: ProviderKey {
            root: ProviderRoot::SkillUse(SkillUseId::from_instance_id(instance(100))),
            grant_path: vec![DeclaredSlot {
                declaration: SlotOwnerDefId::Gem(def("summon")),
                slot: def("actor"),
            }],
        },
        slot: DeclaredSlot {
            declaration: SlotOwnerDefId::Actor(def("minion")),
            slot: def("child"),
        },
    }));
    child.types.skill_types = types(&["attack"]);
    child.summoner = Some(SupportTypeContext {
        skill_types: if prepared.final_types_complete {
            DeclaredSet::complete(prepared.final_types.clone())
        } else {
            DeclaredSet::partial(prepared.final_types.clone(), vec![])
        },
        minion_types: parent.types.minion_types.clone(),
    });
    child.has_gem = Some(false);
    child.is_player_actor = Some(false);
    child
}

#[test]
fn parent_eligible_support_is_rechecked_for_child_flags() {
    let mut gem_only = definition("gem-only");
    gem_only.gems_only = true;
    let pkg = package(vec![("one", gem_only)]);
    let origins = [origin(1, "one", 12, 7.0)];
    let selection = selected(&pkg, &origins);
    let parent = target();
    let prepared_parent = admitted(&pkg, &selection, &parent);
    let child = child(&parent, &prepared_parent);
    let prepared_child = admitted(&pkg, &selection, &child);
    assert!(prepared_parent.selected[0].applicable);
    assert!(!prepared_child.selected[0].applicable);
    assert_eq!(prepared_child.target, child.target);
    assert_eq!(
        prepared_child.ordered_origins,
        prepared_parent.ordered_origins
    );
}

#[test]
fn parent_rejection_does_not_remove_selected_support_from_child() {
    let mut item_support = definition("item-support");
    item_support.from_item = true;
    let pkg = package(vec![("one", item_support)]);
    let selection = selected(&pkg, &[origin(1, "one", 5, 9.0)]);
    let mut parent = target();
    parent.from_item = Some(true);
    let prepared_parent = admitted(&pkg, &selection, &parent);
    let child = child(&parent, &prepared_parent);
    let prepared_child = admitted(&pkg, &selection, &child);
    assert!(!prepared_parent.selected[0].applicable);
    assert!(prepared_child.selected[0].applicable);
    assert_eq!(
        prepared_child.selected[0].assignment,
        prepared_parent.selected[0].assignment
    );
}

#[test]
fn child_starts_with_own_types_and_parent_final_summoner_types() {
    let mut duration = definition("duration");
    duration.added_types = vec![key("duration")];
    let mut receiving = definition("receiving");
    receiving.requires = Some(predicate_type("duration"));
    receiving.added_types = vec![key("b")];
    let pkg = package(vec![("duration", duration), ("receiving", receiving)]);
    let origins = [
        origin(1, "duration", 1, 0.0),
        origin(2, "receiving", 1, 0.0),
    ];
    let selection = selected(&pkg, &origins);
    let parent = target();
    let prepared_parent = admitted(&pkg, &selection, &parent);
    let child = child(&parent, &prepared_parent);
    let prepared_child = admitted(&pkg, &selection, &child);
    assert!(prepared_parent.final_types.contains(&key("spell")));
    assert!(!prepared_child.final_types.contains(&key("spell")));
    assert!(prepared_child.final_types.contains(&key("attack")));
    assert!(prepared_child.final_types.contains(&key("duration")));
    assert!(prepared_child.final_types.contains(&key("b")));
    assert!(
        prepared_child
            .selected
            .iter()
            .all(|position| position.applicable)
    );
    // Using the parent's pre-preparation context is observably different:
    // additions mutate child types, while requirements still use the summoner.
    let mut wrong_summoner = child.clone();
    wrong_summoner.summoner = Some(parent.types.clone());
    let wrong = admitted(&pkg, &selection, &wrong_summoner);
    assert!(!wrong.selected[1].applicable);
    assert!(!wrong.final_types.contains(&key("b")));
}

#[test]
fn family_replacement_keeps_duplicate_positions_for_every_target() {
    let mut first = definition("first");
    first.families = Some(vec![key("a")]);
    let mut second = definition("second");
    second.families = Some(vec![key("b")]);
    let mut both = definition("both");
    both.families = Some(vec![key("a"), key("b")]);
    both.added_types = vec![key("duration")];
    let pkg = package(vec![("first", first), ("second", second), ("both", both)]);
    let origins = [
        origin(1, "first", 30, 99.0),
        origin(2, "second", 30, 99.0),
        origin(3, "both", 1, 0.0),
    ];
    let selection = selected(&pkg, &origins);
    assert_eq!(selection.selected_origin_indices(), &[2, 2]);
    let parent = target();
    let a = admitted(&pkg, &selection, &parent);
    let b = admitted(&pkg, &selection, &child(&parent, &a));
    for result in [a, b] {
        assert_eq!(
            result
                .selected
                .iter()
                .map(|p| (p.position, p.origin_index, p.assignment))
                .collect::<Vec<_>>(),
            vec![(0, 2, origins[2].assignment), (1, 2, origins[2].assignment)]
        );
    }
}

#[test]
fn selection_snapshots_exact_scalars_and_never_reselects_for_a_child() {
    let pkg = package(vec![("one", definition("one"))]);
    let mut origins = [origin(1, "one", 5, 20.5), origin(2, "one", 5, 20.75)];
    let snapshot = origins.clone();
    let selection = selected(&pkg, &origins);
    assert_eq!(selection.preparation(), *pkg.identity());
    assert_eq!(selection.selected_origin_indices(), &[1]);
    origins[0].effective_level = Some(BoundedInteger::new(100).unwrap());
    origins[1].effective_quality = None;
    origins[1].enabled = Some(false);
    assert_eq!(selection.origins(), &snapshot);
    assert_eq!(selected(&pkg, &origins).selected_origin_indices(), &[0]);
    let parent = target();
    let a = admitted(&pkg, &selection, &parent);
    let b = admitted(&pkg, &selection, &child(&parent, &a));
    assert_eq!(a.selected[0].assignment, snapshot[1].assignment);
    assert_eq!(b.selected[0].assignment, snapshot[1].assignment);
    assert_eq!(selection.origins(), &snapshot);
}

#[test]
fn composed_parent_api_matches_split_selection_and_admission() {
    let mut definition = definition("one");
    definition.requires = Some(predicate_type("spell"));
    definition.added_types = vec![key("duration")];
    let pkg = package(vec![("one", definition)]);
    let mut disabled = origin(3, "unconverted", 1, 0.0);
    disabled.enabled = Some(false);
    let origins = [origin(1, "one", 1, 0.0), origin(2, "one", 2, 0.0), disabled];
    let selection = selected(&pkg, &origins);
    assert_eq!(selection.disabled_origins(), &[2]);
    assert_eq!(
        prepare_supports(
            &pkg,
            &origins,
            &target(),
            SupportPreparationLimits::default()
        )
        .unwrap(),
        prepare_selected_supports(
            &pkg,
            &selection,
            &target(),
            SupportPreparationLimits::default()
        )
        .unwrap(),
    );
}

#[test]
fn selection_and_repeated_preparation_consume_one_shared_budget() {
    let pkg = package(vec![("one", definition("one"))]);
    let origins = [origin(1, "one", 1, 0.0)];
    let limits = SupportPreparationLimits::default();
    let mut work = limits.max_work;
    let SupportSelectionOutcome::Known(selection) =
        select_supports_with_budget(&pkg, &origins, limits, &mut work).unwrap()
    else {
        panic!("selection")
    };
    let selection_cost = limits.max_work - work;
    let before = work;
    let expected =
        prepare_selected_supports_with_budget(&pkg, &selection, &target(), limits, &mut work)
            .unwrap();
    let preparation_cost = before - work;
    let mut shared = selection_cost + preparation_cost * 2 - 1;
    let SupportSelectionOutcome::Known(selection) =
        select_supports_with_budget(&pkg, &origins, limits, &mut shared).unwrap()
    else {
        panic!("selection")
    };
    assert_eq!(
        prepare_selected_supports_with_budget(&pkg, &selection, &target(), limits, &mut shared)
            .unwrap(),
        expected
    );
    assert_eq!(
        prepare_selected_supports_with_budget(&pkg, &selection, &target(), limits, &mut shared),
        Err(SupportPreparationError::Limit("work"))
    );
    assert_eq!(shared, 0);
    assert_eq!(
        prepare_selected_supports(&pkg, &selection, &target(), limits).unwrap(),
        expected
    );
}

#[test]
fn selected_snapshots_recheck_package_and_tighter_limits_before_use() {
    let pkg = package(vec![("one", definition("one"))]);
    let selection = selected(&pkg, &[origin(1, "one", 1, 0.0), origin(2, "one", 2, 0.0)]);
    let other = package(vec![("one", definition("different-effect"))]);
    let mut inactive = target();
    inactive.enabled = Some(false);
    assert!(matches!(
        prepare_selected_supports(
            &other,
            &selection,
            &inactive,
            SupportPreparationLimits::default()
        ),
        Err(SupportPreparationError::Invalid(_))
    ));
    assert_eq!(
        prepare_selected_supports(
            &pkg,
            &selection,
            &target(),
            SupportPreparationLimits {
                max_origins: 1,
                ..SupportPreparationLimits::default()
            }
        ),
        Err(SupportPreparationError::Limit("origins"))
    );
    assert!(matches!(
        select_supports(
            &pkg,
            &[],
            SupportPreparationLimits {
                max_work: 0,
                ..SupportPreparationLimits::default()
            }
        ),
        Err(SupportPreparationError::Invalid(_))
    ));
    assert!(matches!(
        prepare_selected_supports(
            &pkg,
            &selection,
            &target(),
            SupportPreparationLimits {
                max_types: 0,
                ..SupportPreparationLimits::default()
            }
        ),
        Err(SupportPreparationError::Invalid(_))
    ));
}

#[test]
fn missing_selection_and_target_facts_remain_unresolved() {
    let mut needs_spell = definition("one");
    needs_spell.requires = Some(predicate_type("spell"));
    let pkg = package(vec![("one", needs_spell)]);
    let mut unknown = origin(1, "one", 1, 0.0);
    unknown.enabled = None;
    assert!(matches!(
        select_supports(&pkg, &[unknown], SupportPreparationLimits::default()).unwrap(),
        SupportSelectionOutcome::Unresolved {
            reason: SupportPreparationGap::OriginEnabled,
            origin_index: Some(0)
        }
    ));
    let selection = selected(&pkg, &[origin(1, "one", 1, 0.0)]);
    let mut partial = target();
    partial.types.skill_types = DeclaredSet::partial(vec![], vec![]);
    assert!(matches!(
        prepare_selected_supports(
            &pkg,
            &selection,
            &partial,
            SupportPreparationLimits::default()
        )
        .unwrap(),
        SupportPreparationOutcome::Unresolved {
            reason: SupportPreparationGap::Applicability,
            origin_index: Some(0)
        }
    ));
    partial.enabled = None;
    assert!(matches!(
        prepare_selected_supports(
            &pkg,
            &selection,
            &partial,
            SupportPreparationLimits::default()
        )
        .unwrap(),
        SupportPreparationOutcome::Unresolved {
            reason: SupportPreparationGap::TargetEnabled,
            origin_index: None
        }
    ));
}
