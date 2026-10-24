//! Current schema migrations preserve explicit selection and ordered contracts.
use super::*;

fn header(prior: &StagedOwnedRelease) -> OwnedReleaseMigrationInput {
    let mut migration = capability_header(prior);
    migration.schema_version = 5;
    migration.release = key("action-selection-contract");
    migration.reason = key("typed-action-selection-reads");
    migration.contract.schema_version = 7;
    migration.contract.operations_version = key(OWNED_RULE_OPERATIONS_V20);
    migration
}

#[test]
fn current_boolean_operation_contract_migrates_once_and_cannot_downgrade() {
    let mut input = prior().input().clone();
    input.recipe.rules.contribution_queries = Some(DeclaredSet::complete(vec![]));
    let prior =
        migration_fixture::contract(input, OWNED_RULE_OPERATIONS_V21, "before-boolean-contract");
    let mut migration = header(&prior);
    migration.release = key("boolean-contract");
    migration.contract.operations_version = key(OWNED_RULE_OPERATIONS_V22);
    let next = compile_owned_release_migration(&prior, migration, Default::default()).unwrap();
    assert_eq!(
        next.input().recipe.rules.schema_version,
        OWNED_RULE_PACKAGE_VERSION
    );
    assert_eq!(
        next.input().recipe.rules.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V22
    );
    assert_eq!(
        next.input().recipe.rules.owners,
        prior.input().recipe.rules.owners
    );
    assert_source_inputs_preserved(&prior, &next);
    assert_ne!(
        next.receipt().compiled_rules,
        prior.receipt().compiled_rules
    );
    for operations in [OWNED_RULE_OPERATIONS_V20, OWNED_RULE_OPERATIONS_V21] {
        let mut backward = header(&next);
        backward.release = key("unsupported-downgrade");
        backward.contract.operations_version = key(operations);
        assert!(matches!(
            compile_owned_release_migration(&next, backward, Default::default()),
            Err(OwnedReleaseError::Invalid(
                "migration cannot downgrade current operations"
            ))
        ));
    }
}

#[test]
fn exact_v19_and_v20_predecessors_preserve_source_queries_and_rebuilds() {
    for operations in [OWNED_RULE_OPERATIONS_V19, OWNED_RULE_OPERATIONS_V20] {
        let prior =
            migration_fixture::contract(prior().input().clone(), operations, "selection-prior");
        let original = prior.receipt().input;
        let mut migration = header(&prior);
        if operations == OWNED_RULE_OPERATIONS_V20 {
            assert!(matches!(
                compile_owned_release_migration(&prior, migration.clone(), Default::default()),
                Err(OwnedReleaseError::Invalid(
                    "migration contains no semantic changes"
                ))
            ));
            migration.contract.rule_semantics_version = key("explicit-selection-refinement");
        }
        let authoring =
            digest_owned("owned-release-contract-migration-v5", &migration, 1_000_000).unwrap();
        let result =
            compile_owned_release_migration(&prior, migration.clone(), Default::default()).unwrap();
        assert_eq!(result.input().recipe.schema.schema_version, 7);
        assert_eq!(
            result.input().recipe.rules.operations_version.as_str(),
            OWNED_RULE_OPERATIONS_V20
        );
        assert!(
            result.input().recipe.schema.definitions == prior.input().recipe.schema.definitions
        );
        assert!(result.input().recipe.schema.slots == prior.input().recipe.schema.slots);
        assert!(result.input().recipe.registry == prior.input().recipe.registry);
        assert!(result.input().recipe.rules.owners == prior.input().recipe.rules.owners);
        assert!(result.input().recipe.rules.tables == prior.input().recipe.rules.tables);
        assert!(result.input().recipe.rules.receivers == prior.input().recipe.rules.receivers);
        assert_eq!(
            result.input().provenance.last().unwrap().authoring_input,
            authoring
        );
        assert_source_inputs_preserved(&prior, &result);
        let repeated =
            compile_owned_release_migration(&prior, migration, Default::default()).unwrap();
        let rebuilt = assemble_owned_release(result.input().clone(), Default::default()).unwrap();
        assert!(result.artifacts().eq(repeated.artifacts()));
        assert!(result.artifacts().eq(rebuilt.artifacts()));
        assert_eq!(prior.receipt().input, original);
    }
}

#[test]
fn contribution_contracts_preserve_membership_and_reject_downgrades() {
    for (old, new) in [
        (OWNED_RULE_OPERATIONS_V22, OWNED_RULE_OPERATIONS_V23),
        (OWNED_RULE_OPERATIONS_V23, OWNED_RULE_OPERATIONS_V24),
        (OWNED_RULE_OPERATIONS_V25, OWNED_RULE_OPERATIONS_V26),
        (OWNED_RULE_OPERATIONS_V26, OWNED_RULE_OPERATIONS_V27),
        (OWNED_RULE_OPERATIONS_V27, OWNED_RULE_OPERATIONS_V28),
    ] {
        let mut input = prior().input().clone();
        input.recipe.rules.contribution_queries = Some(DeclaredSet::complete(vec![]));
        let prior = migration_fixture::contract(input, old, "contribution-prior");
        let mut migration = header(&prior);
        migration.release = key("contribution-contract");
        migration.contract.operations_version = key(new);
        let next =
            compile_owned_release_migration(&prior, migration.clone(), Default::default()).unwrap();
        assert_source_inputs_preserved(&prior, &next);
        assert_eq!(
            next.input().recipe.rules.owners,
            prior.input().recipe.rules.owners
        );
        assert_eq!(
            next.input().recipe.rules.contribution_queries,
            prior.input().recipe.rules.contribution_queries
        );
        assert_eq!(
            next.input().recipe.rules.existing_actor_rules,
            prior.input().recipe.rules.existing_actor_rules
        );
        assert_ne!(
            next.receipt().compiled_rules,
            prior.receipt().compiled_rules
        );
        let repeated =
            compile_owned_release_migration(&prior, migration, Default::default()).unwrap();
        assert!(next.artifacts().eq(repeated.artifacts()));
        for operations in [
            OWNED_RULE_OPERATIONS_V20,
            OWNED_RULE_OPERATIONS_V21,
            OWNED_RULE_OPERATIONS_V22,
            old,
        ] {
            let mut backward = header(&next);
            backward.contract.operations_version = key(operations);
            assert!(matches!(
                compile_owned_release_migration(&next, backward, Default::default()),
                Err(OwnedReleaseError::Invalid(
                    "migration cannot downgrade current operations"
                ))
            ));
        }
        let mut same = header(&next);
        same.contract.operations_version = key(new);
        assert!(matches!(
            compile_owned_release_migration(&next, same, Default::default()),
            Err(OwnedReleaseError::Invalid(
                "migration contains no semantic changes"
            ))
        ));
    }
}

#[test]
fn current_rejects_wrong_contracts_downgrades_stale_inputs_and_exhausted_budgets() {
    let original = prior();
    let prior = migration_fixture::contract(
        original.input().clone(),
        OWNED_RULE_OPERATIONS_V19,
        "selection-prior",
    );
    let valid = header(&prior);
    for version in [1, 2, 3, 4, 6] {
        let mut bad = valid.clone();
        bad.schema_version = version;
        assert!(compile_owned_release_migration(&prior, bad, Default::default()).is_err());
    }
    for schema in [2, 3, 4, 5, 6, 8] {
        let mut bad = valid.clone();
        bad.contract.schema_version = schema;
        assert!(matches!(
            compile_owned_release_migration(&prior, bad, Default::default()),
            Err(OwnedReleaseError::Invalid(
                "migration requires the current schema contract"
            ))
        ));
    }
    for operations in [
        "owned-domain-operations-v999",
        "owned-domain-operations-v028",
    ] {
        let mut bad = valid.clone();
        bad.contract.operations_version = key(operations);
        assert!(matches!(
            compile_owned_release_migration(&prior, bad, Default::default()),
            Err(OwnedReleaseError::Invalid(
                "migration operations are unsupported"
            ))
        ));
    }
    let mut downgrade = valid.clone();
    downgrade.contract.operations_version = key(OWNED_RULE_OPERATIONS_V18);
    assert!(matches!(
        compile_owned_release_migration(&prior, downgrade, Default::default()),
        Err(OwnedReleaseError::Invalid(
            "migration cannot downgrade current operations"
        ))
    ));
    let mut stale = valid.clone();
    stale.before = digest_owned("stale-selection-prior", &1, 100).unwrap();
    assert!(compile_owned_release_migration(&prior, stale, Default::default()).is_err());
    for limits in [
        OwnedReleaseLimits {
            max_semantic_bytes: 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_artifact_bytes: 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_validation_entries: 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_provenance_entries: 1,
            ..Default::default()
        },
    ] {
        assert!(compile_owned_release_migration(&prior, valid.clone(), limits).is_err());
    }
    let result =
        compile_owned_release_migration(&prior, valid.clone(), Default::default()).unwrap();
    let retry = compile_owned_release_migration(&prior, valid, Default::default()).unwrap();
    assert!(result.artifacts().eq(retry.artifacts()));
    let mut downgrade = header(&result);
    downgrade.release = key("invalid-selection-downgrade");
    downgrade.contract.operations_version = key(OWNED_RULE_OPERATIONS_V19);
    assert!(
        compile_owned_release_migration(&result, downgrade.clone(), Default::default()).is_err()
    );
    downgrade.schema_version = 4;
    assert!(compile_owned_release_migration(&result, downgrade, Default::default()).is_err());
}

#[test]
fn current_v21_schema_append_preserves_ordered_queries_and_actor_applicability() {
    use poe_optimizer_import::owned_mapping::OwnedIdRegistry;
    let before = migration_fixture::contract(
        prior().input().clone(),
        OWNED_RULE_OPERATIONS_V20,
        "before-existing-actor",
    );
    let mut registry =
        OwnedIdRegistry::new(before.input().recipe.registry.clone(), Default::default()).unwrap();
    let actor = registry.allocate_definition::<ActorDefinition>().unwrap();
    let subject = SchemaSubject::Definition(actor.address());
    let mut seed = header(&before);
    seed.schema.push(SchemaExtensionEntry::Definition(
        DefinitionDescriptor::Actor(record(
            actor.clone(),
            ActorSchema {
                declarations: declarations(),
            },
        )),
    ));
    seed.owners.push(DefinitionRules {
        owner: subject.clone(),
        programs: DeclaredSet::partial(
            vec![],
            vec![SchemaGap {
                subject: subject.clone(),
                facet: SchemaFacet::GameRules,
                code: key("remaining-shared-rules"),
            }],
        ),
    });
    let seed = compile_owned_release_migration(&before, seed, Default::default()).unwrap();
    let mut input = seed.input().clone();
    let (stat, empty) = input
        .recipe
        .schema
        .definitions
        .iter()
        .find_map(|d| {
            let DefinitionDescriptor::Stat(row) = d else {
                return None;
            };
            let SchemaState::Known(schema) = &row.schema else {
                return None;
            };
            let empty = match &schema.value {
                ComputedValueType::Integer => {
                    ParameterValue::Integer(BoundedInteger::new(0).unwrap())
                }
                ComputedValueType::Quantity { unit } => {
                    ParameterValue::Quantity(FiniteQuantity::new(0., unit.clone()).unwrap())
                }
                _ => return None,
            };
            Some((row.id.clone(), empty))
        })
        .unwrap();
    input.recipe.rules.operations_version = key(OWNED_RULE_OPERATIONS_V21);
    input.recipe.rules.contribution_queries =
        Some(DeclaredSet::complete(vec![ContributionQuery {
            id: key("retained-ordered-query"),
            stat,
            contribution: ContributionKind::Add,
            groups: vec![ContributionGroup {
                ordering: ContributionOrdering::Ordered,
                id: key("empty-test-domain"),
                reduction: ContributionReduction::Sum,
                empty: Some(empty),
                members: DeclaredSet::complete(vec![]),
            }],
        }]));
    input.recipe.rules.existing_actor_rules =
        Some(DeclaredSet::complete(vec![ExistingActorRuleApplication {
            id: key("retained-player-applicability"),
            owner: actor,
            targets: vec![ExistingActorRuleTarget::Player],
        }]));
    let prior = assemble_owned_release(input, Default::default()).unwrap();
    // Determine this finite fixture's exact preflight boundary through the
    // public assembler, without duplicating its entry-counting implementation.
    let mut empty_registries = prior.input().clone();
    empty_registries
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .clear();
    empty_registries.recipe.rules.existing_actor_rules = None;
    let mut low = 0;
    let mut high = OwnedReleaseLimits::default().max_validation_entries;
    while high - low > 1 {
        let middle = low + (high - low) / 2;
        match assemble_owned_release(
            empty_registries.clone(),
            OwnedReleaseLimits {
                max_validation_entries: middle,
                ..Default::default()
            },
        ) {
            Ok(_) => high = middle,
            Err(OwnedReleaseError::Limit("validation entries")) => low = middle,
            Err(error) => panic!("unexpected finite fixture budget error: {error}"),
        }
    }
    for actor_only in [false, true] {
        let mut added = empty_registries.clone();
        if actor_only {
            added.recipe.rules.existing_actor_rules =
                prior.input().recipe.rules.existing_actor_rules.clone();
        } else {
            added.recipe.rules.contribution_queries =
                prior.input().recipe.rules.contribution_queries.clone();
        }
        assert!(matches!(
            assemble_owned_release(
                added,
                OwnedReleaseLimits {
                    max_validation_entries: high,
                    ..Default::default()
                }
            ),
            Err(OwnedReleaseError::Limit("validation entries"))
        ));
    }
    let mut registry =
        OwnedIdRegistry::new(prior.input().recipe.registry.clone(), Default::default()).unwrap();
    let second = registry.allocate_definition::<ActorDefinition>().unwrap();
    let mut migration = header(&prior);
    migration.release = key("after-v21-schema-append");
    migration.contract.operations_version = key(OWNED_RULE_OPERATIONS_V21);
    migration.schema.push(SchemaExtensionEntry::Definition(
        DefinitionDescriptor::Actor(record(
            second.clone(),
            ActorSchema {
                declarations: declarations(),
            },
        )),
    ));
    let next =
        compile_owned_release_migration(&prior, migration.clone(), Default::default()).unwrap();
    assert_eq!(
        next.input().recipe.rules.contribution_queries,
        prior.input().recipe.rules.contribution_queries
    );
    assert_eq!(
        next.input().recipe.rules.existing_actor_rules,
        prior.input().recipe.rules.existing_actor_rules
    );
    assert_eq!(
        next.input().recipe.rules.owners,
        prior.input().recipe.rules.owners
    );
    assert_eq!(next.input().recipe.registry, *registry.input());
    let mut restored = next.input().recipe.clone();
    restored
        .schema
        .definitions
        .retain(|d| d.address() != second.address());
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    restored.registry = prior.input().recipe.registry.clone();
    assert!(
        restored == prior.input().recipe,
        "exact schema-only recipe inverse"
    );
    assert_source_inputs_preserved(&prior, &next);
    let rebuilt = assemble_owned_release(next.input().clone(), Default::default()).unwrap();
    assert!(next.artifacts().eq(rebuilt.artifacts()));
    migration.contract.operations_version = key(OWNED_RULE_OPERATIONS_V20);
    assert!(matches!(
        compile_owned_release_migration(&prior, migration, Default::default()),
        Err(OwnedReleaseError::Invalid(
            "migration cannot downgrade current operations"
        ))
    ));
}

#[test]
fn current_cannot_drop_or_silently_rebind_existing_evaluation_artifacts() {
    let prior = migration_fixture::contract(
        prior().input().clone(),
        OWNED_RULE_OPERATIONS_V19,
        "selection-metrics-prior",
    );
    let mut migration = header(&prior);
    migration.evaluation = Some(endpoint_metrics(&prior, &migration));
    let result = compile_owned_release_migration(&prior, migration, Default::default()).unwrap();
    assert!(result.evaluation().is_some());
    let mut next = header(&result);
    next.release = key("next-selection-metrics");
    next.contract.rule_semantics_version = key("explicit-next-selection-semantics");
    assert!(matches!(
        compile_owned_release_migration(&result, next.clone(), Default::default()),
        Err(OwnedReleaseError::Invalid(
            "migration cannot drop prior evaluation artifacts"
        ))
    ));
    next.evaluation = result.input().evaluation.clone();
    assert!(matches!(
        compile_owned_release_migration(&result, next.clone(), Default::default()),
        Err(OwnedReleaseError::Evaluation(_))
    ));
    next.evaluation = Some(endpoint_metrics(&result, &next));
    let updated = compile_owned_release_migration(&result, next, Default::default()).unwrap();
    assert_source_inputs_preserved(&result, &updated);
}

#[test]
fn explicit_permission_refinement_rebinds_without_replacing_provider_programs() {
    let original = prior();
    let before = migration_fixture::contract(
        original.input().clone(),
        OWNED_RULE_OPERATIONS_V17,
        "provider-prior",
    );
    let mut provider = actor_migration(&before);
    provider.schema_version = 5;
    provider.contract = capability_header(&before).contract;
    for row in &mut provider.schema {
        if let SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(entry)) = row {
            let SchemaState::Known(schema) = &mut entry.schema else {
                unreachable!()
            };
            schema.skill_input = Some(SkillInputAuthority::Projected);
        }
    }
    let prior =
        compile_owned_release_migration(&before, provider.clone(), Default::default()).unwrap();
    let parameter = provider
        .schema
        .iter()
        .find_map(|row| match row {
            SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(entry)) => Some(entry.id.clone()),
            _ => None,
        })
        .unwrap();
    let mut supply = provider
        .schema
        .iter()
        .find_map(|row| match row {
            SchemaExtensionEntry::Slot(SlotDescriptor::SkillGrant(entry)) => Some(entry.clone()),
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(schema) = &mut supply.schema else {
        unreachable!()
    };
    assert!(schema.preset_inputs.is_none());
    schema.preset_inputs = Some(PresetSkillInputPermission {
        schema_version: PRESET_SKILL_INPUT_PERMISSION_V1,
        parameters: DeclaredSet::complete(vec![parameter]),
    });
    let mut migration = header(&prior);
    migration.schema = vec![SchemaExtensionEntry::Slot(SlotDescriptor::SkillGrant(
        supply.clone(),
    ))];
    let result =
        compile_owned_release_migration(&prior, migration.clone(), Default::default()).unwrap();
    let mut restored = result.input().recipe.schema.slots.clone();
    let changed = restored
        .iter_mut()
        .find(|row| row.address() == SkillGrantSlotDefId::address(&supply.id))
        .unwrap();
    assert!(*changed == SlotDescriptor::SkillGrant(supply.clone()));
    *changed = prior
        .input()
        .recipe
        .schema
        .slots
        .iter()
        .find(|row| row.address() == SkillGrantSlotDefId::address(&supply.id))
        .unwrap()
        .clone();
    assert!(restored == prior.input().recipe.schema.slots);
    assert!(result.input().recipe.rules.owners == prior.input().recipe.rules.owners);
    assert!(result.input().recipe.registry == prior.input().recipe.registry);
    assert_source_inputs_preserved(&prior, &result);
    let mut historical = migration;
    historical.schema_version = 3;
    assert!(compile_owned_release_migration(&prior, historical, Default::default()).is_err());
    let mut old_schema = result.input().recipe.schema.clone();
    old_schema.schema_version = 5;
    assert!(OwnedDefinitionSchemaPackage::new(old_schema, Default::default()).is_err());

    // Authorization changes are explicit same-contract schema replacements;
    // removing permission does not remove the provider's existing program.
    let mut next = header(&result);
    next.release = key("reviewed-preset-permission-removal");
    let SchemaState::Known(schema) = &mut supply.schema else {
        unreachable!()
    };
    schema.preset_inputs.as_mut().unwrap().parameters = DeclaredSet::complete(vec![]);
    next.schema = vec![SchemaExtensionEntry::Slot(SlotDescriptor::SkillGrant(
        supply.clone(),
    ))];
    let updated = compile_owned_release_migration(&result, next, Default::default()).unwrap();
    assert!(
        updated
            .input()
            .recipe
            .schema
            .slots
            .contains(&SlotDescriptor::SkillGrant(supply))
    );
    assert!(updated.input().recipe.rules.owners == result.input().recipe.rules.owners);
    assert_source_inputs_preserved(&result, &updated);
}
