//! Cold support binding and warm selection; no receiving or metric authority.
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*,
    owned_supports::*,
};
use poe_optimizer_data::{
    owned_rules::OwnedRulePackage,
    owned_schema::{
        OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, SchemaPackageInput,
    },
    owned_supports::OwnedSupportPreparation,
};
use poe_optimizer_engine::owned_supports::*;

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("support-index", "v1").unwrap()
}
fn def<T: DefinitionDomain>(value: &str) -> DefId<T> {
    DefId::parse(namespace(), value).unwrap()
}
fn id<T: BuildInstanceId>(value: u64) -> T {
    T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([73; 16]), value).unwrap())
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
fn target(value: u64) -> SkillTarget {
    SkillTarget::Authored(id(value))
}
fn package() -> OwnedSupportPreparation {
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: namespace(),
            release: key("test"),
            semantics_version: key("test"),
            definitions: vec![
                DefinitionDescriptor::Unit(known(
                    def("quality"),
                    UnitSchema {
                        dimension: UnitDimension::PercentagePoints,
                    },
                )),
                DefinitionDescriptor::Gem(known(
                    def("support"),
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
                        declarations: DeclaredSlots {
                            parameters: empty(),
                            choices: empty(),
                            grants: empty(),
                            actors: empty(),
                            skill_grants: empty(),
                            outputs: empty(),
                            sockets: empty(),
                        },
                    },
                )),
            ],
            slots: vec![],
        },
        Default::default(),
    )
    .unwrap();
    let rules = OwnedRulePackage::new(
        RulePackageInput {
            support_discovery: None,
            existing_actor_rules: None,
            contribution_queries: None,
            effect_applications: None,
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: namespace(),
            release: key("test"),
            semantics_version: key("test"),
            operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
            definitions: schema.identity().clone(),
            tables: vec![],
            owners: vec![],
            receivers: empty(),
        },
        &schema,
        Default::default(),
    )
    .unwrap();
    OwnedSupportPreparation::new(
        SupportPreparationInput {
            schema_version: OWNED_SUPPORT_PREPARATION_VERSION,
            namespace: namespace(),
            release: key("test"),
            definitions: schema.identity().clone(),
            rules: *rules.identity(),
            policy: SupportPreparationPolicy::OrderedReplacementRetryFrontierV1,
            quality_unit: def("quality"),
            types: vec![],
            effects: vec![key("effect")],
            families: vec![],
            supports: vec![SupportPreparationEntry {
                gem: def("support"),
                preparation: SchemaState::Known(SupportPreparationDefinition {
                    effect: key("effect"),
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
                }),
            }],
        },
        &schema,
        &rules,
        Default::default(),
    )
    .unwrap()
}
fn input() -> BuildInput {
    BuildInput {
        generated_inputs: None,
        allocator: InstanceAllocatorState::from_parts(BuildLineage::from_bytes([73; 16]), 10000),
        revision: BuildRevision::from_u64(1),
        game_version: namespace(),
        character: CharacterSpec {
            class: def("class"),
            ascendancy: None,
            level: 90,
            rewards: vec![],
        },
        weapon_loadouts: vec![id(1)],
        active_weapon_loadout: id(1),
        items: vec![],
        equipment: vec![],
        allocations: vec![],
        gems: (0..3)
            .map(|index| GemInstance {
                id: id(200 + index),
                definition: def("support"),
                parameters: vec![],
                level: 99 - index as u16,
                quality: None,
            })
            .collect(),
        skills: [100, 101]
            .into_iter()
            .map(|value| SkillUse {
                parameters: None,
                id: id(value),
                source: AuthoredSkillSource::Direct(def("skill")),
                enabled: true,
                scope: LoadoutScope::Shared,
            })
            .collect(),
        supports: (0..3)
            .map(|index| SupportAssignment {
                id: id(10 + index),
                support: id(200 + index),
                target: target(if index == 2 { 101 } else { 100 }),
                enabled: true,
            })
            .collect(),
        authored_support_order: Some(vec![
            AuthoredSupportOrder {
                target: target(100),
                assignments: vec![id(11), id(10)],
            },
            AuthoredSupportOrder {
                target: target(101),
                assignments: vec![id(12)],
            },
        ]),
        payload_links: vec![],
        choices: vec![],
    }
}
fn build(input: BuildInput) -> BuildSpec {
    BuildSpec::new(input, Default::default()).unwrap()
}
fn values(ids: &[u64]) -> Vec<EffectiveSupportValues> {
    ids.iter()
        .map(|value| EffectiveSupportValues {
            assignment: id(*value),
            effective_level: Some(BoundedInteger::new(1).unwrap()),
            effective_quality: Some(FiniteQuantity::new(5.0, def("quality")).unwrap()),
        })
        .collect()
}
fn selected(outcome: SupportBuildSelectionOutcome) -> SelectedSupports {
    let SupportBuildSelectionOutcome::Known(selected) = outcome else {
        panic!("known selection")
    };
    selected
}
fn generated(root: u64, steps: usize) -> SkillTarget {
    SkillTarget::Generated(Box::new(GeneratedSkillKey {
        provider: ProviderKey {
            root: ProviderRoot::SkillUse(id(root)),
            grant_path: (0..steps)
                .map(|_| DeclaredSlot {
                    declaration: SlotOwnerDefId::Skill(def("skill")),
                    slot: def("parent-grant"),
                })
                .collect(),
        },
        slot: DeclaredSlot {
            declaration: SlotOwnerDefId::Skill(def("skill")),
            slot: def("generated"),
        },
    }))
}

#[test]
fn index_binds_all_targets_preserving_order_and_never_physical_levels() {
    let pkg = package();
    let index = SupportBuildIndex::new(
        &build(input()),
        SupportPreparationLimits {
            max_origins: 2,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        index.targets().cloned().collect::<Vec<_>>(),
        vec![target(100), target(101)]
    );
    assert_eq!(index.authored_activity(&target(100)), Some(true));
    let a = selected(
        index
            .select_for_target(&pkg, &target(100), &values(&[10, 11]), Default::default())
            .unwrap(),
    );
    assert_eq!(
        a.origins()
            .iter()
            .map(|row| row.assignment)
            .collect::<Vec<_>>(),
        vec![id(11), id(10)]
    );
    assert_eq!(a.selected_origin_indices(), &[0]);
    let b = selected(
        index
            .select_for_target(&pkg, &target(101), &values(&[12]), Default::default())
            .unwrap(),
    );
    assert_eq!(b.origins()[0].assignment, id(12));
    assert!(matches!(
        index
            .select_for_target(&pkg, &target(100), &[], Default::default())
            .unwrap(),
        SupportBuildSelectionOutcome::Unresolved {
            reason: SupportPreparationGap::EffectiveLevel,
            ..
        }
    ));
    assert!(matches!(
        SupportBuildIndex::new(
            &build(input()),
            SupportPreparationLimits {
                max_origins: 1,
                ..Default::default()
            }
        ),
        Err(SupportPreparationError::Limit("origins"))
    ));
}

#[test]
fn complete_empty_membership_and_generated_entering_paths_remain_distinct() {
    let pkg = package();
    let mut input = input();
    let generated_target = generated(101, 1);
    input.supports[2].target = generated_target.clone();
    input.authored_support_order.as_mut().unwrap()[1].target = generated_target.clone();
    let index = SupportBuildIndex::new(&build(input), Default::default()).unwrap();
    assert_eq!(index.authored_activity(&generated_target), None);
    assert_eq!(
        selected(
            index
                .select_for_target(&pkg, &generated_target, &values(&[12]), Default::default())
                .unwrap()
        )
        .origins()
        .len(),
        1
    );
    assert!(
        selected(
            index
                .select_for_target(&pkg, &generated(100, 1), &[], Default::default())
                .unwrap()
        )
        .origins()
        .is_empty()
    );
    assert!(
        selected(
            index
                .select_for_target(&pkg, &target(101), &[], Default::default())
                .unwrap()
        )
        .origins()
        .is_empty()
    );
    assert!(matches!(
        index.select_for_target(&pkg, &target(999), &[], Default::default()),
        Err(SupportPreparationError::Invalid(
            "preparation target has no authored build skill"
        ))
    ));
    assert!(matches!(
        index.select_for_target(
            &pkg,
            &generated(100, 2),
            &[],
            SupportPreparationLimits {
                max_target_depth: 1,
                ..Default::default()
            }
        ),
        Err(SupportPreparationError::Limit("target depth"))
    ));
}

#[test]
fn missing_order_and_bad_scalars_keep_their_precedence_over_inactivity() {
    let pkg = package();
    let mut input = input();
    input.skills[0].enabled = false;
    input.weapon_loadouts.push(id(2));
    input.skills[1].scope = LoadoutScope::Selected {
        loadouts: vec![id(2)],
    };
    let index = SupportBuildIndex::new(&build(input.clone()), Default::default()).unwrap();
    for target in [target(100), target(101)] {
        assert_eq!(index.authored_activity(&target), Some(false));
        assert!(matches!(
            index
                .select_for_target(&pkg, &target, &[], Default::default())
                .unwrap(),
            SupportBuildSelectionOutcome::Inactive { .. }
        ));
    }
    for (target, values, expected) in [
        (
            target(101),
            values(&[10]),
            "effective support input belongs to another target",
        ),
        (
            target(100),
            values(&[999]),
            "effective support input has no build assignment",
        ),
        (
            target(100),
            values(&[10, 10]),
            "duplicate effective support input",
        ),
    ] {
        assert_eq!(
            index.select_for_target(&pkg, &target, &values, Default::default()),
            Err(SupportPreparationError::Invalid(expected))
        );
    }
    input.authored_support_order = None;
    let index = SupportBuildIndex::new(&build(input), Default::default()).unwrap();
    assert!(!index.has_origin_order());
    assert_eq!(index.authored_activity(&target(100)), Some(false));
    assert!(matches!(
        index
            .select_for_target(&pkg, &target(100), &values(&[999]), Default::default())
            .unwrap(),
        SupportBuildSelectionOutcome::Unresolved {
            reason: SupportPreparationGap::OriginOrder,
            ..
        }
    ));
}

#[test]
fn disabled_assignments_are_retained_without_demanding_definition_conversion() {
    let pkg = package();
    let mut input = input();
    input.supports[0].enabled = false;
    input.gems[0].definition = def("unconverted");
    let index = SupportBuildIndex::new(&build(input), Default::default()).unwrap();
    let selected = selected(
        index
            .select_for_target(&pkg, &target(100), &values(&[11]), Default::default())
            .unwrap(),
    );
    assert_eq!(selected.origins().len(), 2);
    assert_eq!(selected.disabled_origins(), &[1]);
    assert_eq!(selected.selected_origin_indices(), &[0]);
}

#[test]
fn cold_and_warm_attempts_charge_one_decreasing_budget_including_failures() {
    let pkg = package();
    let build = build(input());
    let limits = SupportPreparationLimits::default();
    let mut work = limits.max_work;
    let index = SupportBuildIndex::new_with_budget(&build, limits, &mut work).unwrap();
    let cold_used = limits.max_work - work;
    assert!(cold_used > 0);
    let mut exact = cold_used;
    SupportBuildIndex::new_with_budget(&build, limits, &mut exact).unwrap();
    assert_eq!(exact, 0);
    let mut short = cold_used - 1;
    assert!(matches!(
        SupportBuildIndex::new_with_budget(&build, limits, &mut short),
        Err(SupportPreparationError::Limit("work"))
    ));
    assert_eq!(short, 0);
    let before = work;
    let expected = index
        .select_for_target_with_budget(&pkg, &target(100), &values(&[10, 11]), limits, &mut work)
        .unwrap();
    let warm_used = before - work;
    assert!(warm_used > 0);
    let mut exact = warm_used;
    assert_eq!(
        index
            .select_for_target_with_budget(
                &pkg,
                &target(100),
                &values(&[10, 11]),
                limits,
                &mut exact
            )
            .unwrap(),
        expected
    );
    assert_eq!(exact, 0);
    let mut short = warm_used - 1;
    assert!(matches!(
        index.select_for_target_with_budget(
            &pkg,
            &target(100),
            &values(&[10, 11]),
            limits,
            &mut short
        ),
        Err(SupportPreparationError::Limit("work"))
    ));
    assert_eq!(short, 0);
    let before = work;
    assert!(
        index
            .select_for_target_with_budget(&pkg, &target(100), &values(&[999]), limits, &mut work)
            .is_err()
    );
    assert!(work < before);
}

#[test]
fn warm_binding_does_not_rescan_unrelated_physical_gems() {
    let pkg = package();
    let mut larger = input();
    larger.gems.extend((1000..1256).map(|value| GemInstance {
        id: id(value),
        definition: def("unrelated"),
        level: 1,
        quality: None,
        parameters: vec![],
    }));
    let limits = SupportPreparationLimits::default();
    let mut observed = Vec::new();
    for input in [input(), larger] {
        let index = SupportBuildIndex::new(&build(input), limits).unwrap();
        let mut work = limits.max_work;
        let selected = index
            .select_for_target_with_budget(
                &pkg,
                &target(100),
                &values(&[10, 11]),
                limits,
                &mut work,
            )
            .unwrap();
        observed.push((selected, work));
    }
    assert_eq!(observed[0], observed[1]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn immutable_index_supports_parallel_independent_selection() {
    use rayon::prelude::*;
    let pkg = package();
    let index = SupportBuildIndex::new(&build(input()), Default::default()).unwrap();
    let values = values(&[10, 11]);
    let expected = index
        .select_for_target(&pkg, &target(100), &values, Default::default())
        .unwrap();
    let actual: Vec<_> = (0..32)
        .into_par_iter()
        .map(|_| {
            index
                .select_for_target(&pkg, &target(100), &values, Default::default())
                .unwrap()
        })
        .collect();
    assert!(actual.iter().all(|value| value == &expected));
}
