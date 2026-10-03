//! Explicit contract migration preserves historical releases and ordered requests.
#[path = "support/owned_compact_fixture.rs"]
mod fixture;
#[path = "support/owned_release_migration_v3_fixture.rs"]
mod v3_fixture;
use poe_optimizer_core::{
    owned_build::{ParameterValue, QueryId},
    owned_content::digest_owned,
    owned_definitions::*,
    owned_metrics::MetricMappingInput,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    owned_normalize::{GemQualityPolicy, ImportQueryTarget, ImportQueryTemplate},
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::*,
    owned_release_evaluation::OwnedReleaseEvaluationInput,
    owned_release_migration::*,
    owned_successor::*,
};
use std::{fs, path::PathBuf};

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn prior() -> StagedOwnedRelease {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let first = transition_owned_bundle(fixture::input(&root), Default::default()).unwrap();
    let next = fixture::next(&first);
    let bundle = transition_owned_catalog_with_tree_compact(
        next.clone(),
        fixture::append(&next),
        fixture::tree(&next),
        Default::default(),
    )
    .unwrap();
    assemble_owned_release(
        OwnedReleaseInput {
            schema_version: 1,
            recipe: bundle.recipe().clone(),
            mapping: bundle.mapping().input().clone(),
            roles: bundle.roles().input().clone(),
            normalization: bundle.normalization().clone(),
            rewards: bundle.rewards().input().clone(),
            items: bundle.items().input().clone(),
            item_source: bundle.item_source().input().clone(),
            tree: bundle.tree().map(|tree| tree.input().clone()),
            evaluation: None,
            query_sets: bundle.query_sets().to_vec(),
            provenance: vec![OwnedReleaseProvenance {
                kind: key("prior-authoring-evidence"),
                prior_input: digest_owned("test-prior", &1, 100).unwrap(),
                authoring_input: digest_owned("test-authoring", &2, 100).unwrap(),
            }],
        },
        Default::default(),
    )
    .unwrap()
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: empty(),
        choices: empty(),
        grants: empty(),
        actors: empty(),
        skill_grants: empty(),
        outputs: empty(),
        sockets: empty(),
    }
}
fn record<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn header(prior: &StagedOwnedRelease) -> OwnedReleaseMigrationInput {
    OwnedReleaseMigrationInput {
        schema_version: 1,
        before: prior.receipt().input,
        release: key("actor-supply-migrated"),
        reason: key("explicit-contract-migration"),
        contract: OwnedReleaseContractMigration {
            schema_version: 3,
            schema_semantics_version: key("owned-actor-inputs-v1"),
            operations_version: key("owned-domain-operations-v11"),
            rule_semantics_version: key("owned-actor-rules-v1"),
        },
        schema: vec![],
        tables: vec![],
        owners: vec![],
        receivers: vec![],
        query_targets: vec![],
        evaluation: None,
    }
}

#[test]
fn omitted_evaluation_preserves_the_frozen_v1_wire_and_authoring_digest() {
    // Keep the historical field list independent from the current DTO so a
    // default-valued field accidentally entering the wire changes this witness.
    #[derive(serde::Serialize)]
    struct Legacy<'a> {
        schema_version: u32,
        before: poe_optimizer_core::owned_content::OwnedContentDigest,
        release: &'a OwnedDefinitionKey,
        reason: &'a OwnedDefinitionKey,
        contract: &'a OwnedReleaseContractMigration,
        schema: &'a [SchemaExtensionEntry],
        tables: &'a [IntegerRuleTable],
        owners: &'a [DefinitionRules],
        receivers: &'a [StatReceiver],
        query_targets: &'a [OwnedReleaseQueryTargetMigration],
    }
    let prior = prior();
    let input = header(&prior);
    let historical = Legacy {
        schema_version: input.schema_version,
        before: input.before,
        release: &input.release,
        reason: &input.reason,
        contract: &input.contract,
        schema: &input.schema,
        tables: &input.tables,
        owners: &input.owners,
        receivers: &input.receivers,
        query_targets: &input.query_targets,
    };
    let bytes = serde_json::to_vec(&historical).unwrap();
    assert_eq!(serde_json::to_vec(&input).unwrap(), bytes);
    let restored: OwnedReleaseMigrationInput = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored, input);
    assert!(restored.evaluation.is_none());
    let expected = digest_owned(
        "owned-release-contract-migration-v1",
        &historical,
        1_000_000,
    )
    .unwrap();
    let migrated = compile_owned_release_migration(&prior, input, Default::default()).unwrap();
    assert_eq!(migrated.input().schema_version, 1);
    assert_eq!(
        migrated.input().provenance.last().unwrap().authoring_input,
        expected
    );
}

#[test]
fn schema3_operations11_migrate_to_explicit_schema4_operations13_and_metric_artifact() {
    let old = prior();
    let prior = compile_owned_release_migration(&old, header(&old), Default::default()).unwrap();
    assert_eq!(prior.input().recipe.schema.schema_version, 3);
    assert_eq!(
        prior.input().recipe.rules.operations_version.as_str(),
        "owned-domain-operations-v11"
    );
    let mut migration = header(&prior);
    migration.schema_version = 2;
    migration.release = key("explicit-preparation-contract");
    migration.contract.schema_version = 4;
    migration.contract.operations_version = key("owned-domain-operations-v13");
    let mut schema = prior.input().recipe.schema.clone();
    schema.schema_version = migration.contract.schema_version;
    schema.semantics_version = migration.contract.schema_semantics_version.clone();
    schema.release = migration.release.clone();
    let endpoint = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    migration.evaluation = Some(OwnedReleaseEvaluationInput {
        metrics: MetricMappingInput {
            schema_version: 1,
            namespace: prior.input().recipe.schema.namespace.clone(),
            release: key("reviewed-metric-bindings"),
            definitions: endpoint.identity().clone(),
            bindings: vec![],
        },
        support: None,
    });
    let authoring =
        digest_owned("owned-release-contract-migration-v2", &migration, 1_000_000).unwrap();
    let result =
        compile_owned_release_migration(&prior, migration.clone(), Default::default()).unwrap();
    assert_eq!(result.input().schema_version, 2);
    assert_eq!(result.input().recipe.schema.schema_version, 4);
    assert_eq!(
        result.input().recipe.rules.operations_version.as_str(),
        "owned-domain-operations-v13"
    );
    assert!(result.evaluation().unwrap().support().is_none());
    assert_eq!(result.query_sets(), prior.query_sets());
    assert_eq!(
        result.input().recipe.registry,
        prior.input().recipe.registry
    );
    assert_eq!(
        result.input().recipe.rules.owners,
        prior.input().recipe.rules.owners
    );
    assert_eq!(
        result.input().recipe.rules.tables,
        prior.input().recipe.rules.tables
    );
    assert_eq!(
        result.input().recipe.rules.receivers,
        prior.input().recipe.rules.receivers
    );
    assert_eq!(result.receipt().source, prior.receipt().source);
    assert_eq!(
        result.input().provenance.last().unwrap().authoring_input,
        authoring
    );
    assert!(result.artifacts().any(|(name, _)| name == "metrics.json"));
    // The supplied schema binding cannot be a commitment to the predecessor.
    migration.evaluation.as_mut().unwrap().metrics.definitions =
        prior.receipt().definitions.clone();
    assert!(matches!(
        compile_owned_release_migration(&prior, migration, Default::default()),
        Err(OwnedReleaseError::Evaluation(_))
    ));
    assert!(prior.evaluation().is_none());
    let rebuilt = assemble_owned_release(result.input().clone(), Default::default()).unwrap();
    assert!(rebuilt.artifacts().eq(result.artifacts()));
}
fn subject_key(subject: &SchemaSubject) -> &OwnedDefinitionKey {
    match subject {
        SchemaSubject::Definition(address) => address.key(),
        SchemaSubject::Slot(address) => address.key(),
    }
}
fn actor_migration(prior: &StagedOwnedRelease) -> OwnedReleaseMigrationInput {
    let mut migration = header(prior);
    let mut actor_slot = prior.input().recipe.schema.slots.iter().find(|row| matches!(row,
        SlotDescriptor::Actor(entry) if matches!(&entry.schema, SchemaState::Known(schema) if !schema.skills.members.is_empty())
    )).unwrap().clone();
    let SlotDescriptor::Actor(entry) = &mut actor_slot else {
        unreachable!()
    };
    let SchemaState::Known(actor_schema) = &mut entry.schema else {
        unreachable!()
    };
    let skill_id = actor_schema.skills.members[0].clone();
    let output = actor_schema
        .outputs
        .members
        .iter()
        .find(|output| output.declaration == SlotOwnerDefId::Skill(skill_id.clone()))
        .unwrap()
        .clone();
    let mut skill = prior
        .input()
        .recipe
        .schema
        .definitions
        .iter()
        .find(|row| row.address() == skill_id.address())
        .unwrap()
        .clone();
    let DefinitionDescriptor::Skill(entry) = &mut skill else {
        unreachable!()
    };
    let SchemaState::Known(skill_schema) = &mut entry.schema else {
        unreachable!()
    };
    assert!(skill_schema.declarations.parameters.is_complete());
    assert!(skill_schema.declarations.parameters.members.is_empty());
    let mut registry = prior.assembled().registry().clone();
    let actor: ActorDefId = registry.allocate_definition().unwrap();
    let owner = SlotOwnerDefId::Actor(actor.clone());
    let parameter = registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Skill(skill_id.clone()))
        .unwrap();
    let supply = registry
        .allocate_slot::<SkillGrantSlotDefinition>(owner.clone())
        .unwrap();
    let grant = registry
        .allocate_slot::<GrantSlotDefinition>(owner)
        .unwrap();
    actor_schema.provider_definition = Some(actor.clone());
    skill_schema.declarations.parameters = DeclaredSet::complete(vec![parameter.clone()]);
    let mut ports = declarations();
    ports.skill_grants.members.push(supply.clone());
    ports.grants.members.push(grant.clone());
    migration.schema = vec![
        SchemaExtensionEntry::Definition(skill),
        SchemaExtensionEntry::Slot(actor_slot),
        SchemaExtensionEntry::Definition(DefinitionDescriptor::Actor(record(
            actor.clone(),
            ActorSchema {
                declarations: ports,
            },
        ))),
        SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(record(
            parameter.clone(),
            ParameterSlotSchema {
                skill_input: None,
                value: ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            },
        ))),
        SchemaExtensionEntry::Slot(SlotDescriptor::SkillGrant(record(
            supply.clone(),
            SkillGrantSlotSchema {
                skill: skill_id,
                outputs: DeclaredSet::complete(vec![output]),
            },
        ))),
        SchemaExtensionEntry::Slot(SlotDescriptor::Grant(record(
            grant.clone(),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Skill(supply.clone()),
            },
        ))),
    ];
    migration
        .schema
        .sort_by_key(|row| subject_key(&row.subject()).clone());
    migration.owners.push(DefinitionRules {
        owner: SchemaSubject::Definition(actor.address()),
        programs: DeclaredSet::complete(vec![RuleProgram {
            id: key("supply-reviewed-ability"),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![
                RuleNode {
                    id: key("active"),
                    expression: RuleExpression::Literal {
                        value: ParameterValue::Boolean(true),
                    },
                },
                RuleNode {
                    id: key("level"),
                    expression: RuleExpression::Literal {
                        value: ParameterValue::Integer(BoundedInteger::new(1).unwrap()),
                    },
                },
            ],
            effects: vec![
                RuleEffect {
                    id: key("activate"),
                    when: None,
                    effect: RuleEffectKind::ActivateGrant {
                        slot: grant.clone(),
                        enabled: key("active"),
                    },
                },
                RuleEffect {
                    id: key("level-input"),
                    when: None,
                    effect: RuleEffectKind::ProjectSkillParameter {
                        skill: supply,
                        parameter,
                        value: key("level"),
                    },
                },
            ],
        }]),
    });
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/owned/poe2/3887ae68/selected-actions/original-05.json");
    let queries: Vec<ImportQueryTemplate> =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    for query in queries
        .into_iter()
        .filter(|row| matches!(row.id.as_str(), "reference-14" | "reference-16"))
    {
        let ImportQueryTarget::Action(mut action) = query.target else {
            panic!("reviewed action")
        };
        action.provider.grant_path.push(grant.clone());
        migration
            .query_targets
            .push(OwnedReleaseQueryTargetMigration {
                query_set: key("original-05"),
                query_id: query.id,
                target: ImportQueryTarget::Action(action),
            });
    }
    assert_eq!(migration.query_targets.len(), 2);
    migration
}

#[test]
fn migration_atomically_binds_new_actor_and_parameters_without_rewriting_prior_release() {
    let prior = prior();
    let migration = actor_migration(&prior);
    let authoring = digest_owned(
        "owned-release-contract-migration-v1",
        &migration,
        16 * 1024 * 1024,
    )
    .unwrap();
    let result =
        compile_owned_release_migration(&prior, migration.clone(), Default::default()).unwrap();
    assert_eq!(result.receipt().query_sets, 5);
    assert_eq!(result.receipt().query_rows, 110);
    assert_eq!(result.input().recipe.schema.schema_version, 3);
    assert_eq!(
        result.input().recipe.rules.operations_version.as_str(),
        "owned-domain-operations-v11"
    );
    assert_eq!(
        result.input().recipe.registry.last_issued.get(),
        prior.input().recipe.registry.last_issued.get() + 4
    );
    assert_eq!(
        &result.input().recipe.registry.entries[..prior.input().recipe.registry.entries.len()],
        &prior.input().recipe.registry.entries
    );
    assert_eq!(result.receipt().source, prior.receipt().source);
    assert_eq!(
        &result.input().provenance[..prior.input().provenance.len()],
        &prior.input().provenance
    );
    assert_eq!(
        result.input().provenance.last().unwrap(),
        &OwnedReleaseProvenance {
            kind: migration.reason.clone(),
            prior_input: prior.receipt().input,
            authoring_input: authoring,
        }
    );
    let mut changed = 0;
    for (before_set, after_set) in prior.query_sets().iter().zip(result.query_sets()) {
        assert_eq!(before_set.name, after_set.name);
        assert_eq!(before_set.queries.len(), after_set.queries.len());
        for (before, after) in before_set.queries.iter().zip(&after_set.queries) {
            assert_eq!((&before.id, &before.metric), (&after.id, &after.metric));
            if before.target != after.target {
                changed += 1;
                let patch = migration
                    .query_targets
                    .iter()
                    .find(|row| row.query_set == before_set.name && row.query_id == before.id)
                    .unwrap();
                assert_eq!(after.target, patch.target);
            } else {
                assert_eq!(before, after);
            }
        }
    }
    assert_eq!(changed, 2);
    // Restore only declared edits and exact dependent bindings. Any other change fails.
    let mut restored = result.input().clone();
    restored.recipe = prior.input().recipe.clone();
    restored.mapping.definitions = prior.receipt().definitions.clone();
    restored.mapping.registry = prior.receipt().registry;
    restored.roles.definitions = prior.receipt().definitions.clone();
    restored.roles.mapping = prior.receipt().mapping;
    if let GemQualityPolicy::Attributes(quality) = &mut restored.normalization.gem_quality {
        quality.definitions = prior.receipt().definitions.clone();
    }
    if let Some(gems) = &mut restored.normalization.gem_inputs {
        gems.definitions = prior.receipt().definitions.clone();
    }
    restored.rewards.definitions = prior.receipt().definitions.clone();
    restored.rewards.mapping = prior.receipt().mapping;
    restored.items.definitions = prior.receipt().definitions.clone();
    restored.item_source.item_lines = prior.receipt().items;
    assert_eq!(
        restored.tree.as_ref().unwrap().content,
        prior.input().tree.as_ref().unwrap().content
    );
    restored.tree = prior.input().tree.clone();
    restored.query_sets = prior.input().query_sets.clone();
    restored.provenance = prior.input().provenance.clone();
    assert_eq!(restored, *prior.input());
    // Existing schema/rule rows survive unless named in this explicit migration.
    for old in &prior.input().recipe.schema.definitions {
        let after = result
            .assembled()
            .schema()
            .lookup_definition(&old.address())
            .unwrap();
        assert_eq!(
            after,
            migration
                .schema
                .iter()
                .find_map(|row| match row {
                    SchemaExtensionEntry::Definition(row) if row.address() == old.address() =>
                        Some(row),
                    _ => None,
                })
                .unwrap_or(old)
        );
    }
    for old in &prior.input().recipe.schema.slots {
        let after = result
            .assembled()
            .schema()
            .lookup_slot(&old.address())
            .unwrap();
        assert_eq!(
            after,
            migration
                .schema
                .iter()
                .find_map(|row| match row {
                    SchemaExtensionEntry::Slot(row) if row.address() == old.address() => Some(row),
                    _ => None,
                })
                .unwrap_or(old)
        );
    }
    let mut routing = result.input().recipe.routing.clone();
    routing.definitions = prior.receipt().definitions.clone();
    assert_eq!(routing, prior.input().recipe.routing);
    for old in &prior.input().recipe.rules.owners {
        assert_eq!(
            result
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .find(|row| row.owner == old.owner),
            Some(old)
        );
    }
    assert_eq!(
        result.input().recipe.rules.tables,
        prior.input().recipe.rules.tables
    );
    assert_eq!(
        result.input().recipe.rules.receivers,
        prior.input().recipe.rules.receivers
    );
    let repeated = compile_owned_release_migration(&prior, migration, Default::default()).unwrap();
    assert!(result.artifacts().eq(repeated.artifacts()));
    let rebuilt = assemble_owned_release(result.input().clone(), Default::default()).unwrap();
    assert!(result.artifacts().eq(rebuilt.artifacts()));
    let old_rebuilt = assemble_owned_release(prior.input().clone(), Default::default()).unwrap();
    assert!(prior.artifacts().eq(old_rebuilt.artifacts()));
}

#[test]
fn migration_rejects_unknown_contracts_stale_inputs_schema_reuse_and_query_scope_changes() {
    let prior = prior();
    let valid = actor_migration(&prior);
    let mut cases = vec![];
    let mut bad = valid.clone();
    bad.schema_version = 2;
    cases.push(bad);
    let mut bad = valid.clone();
    bad.before = digest_owned("stale", &1, 100).unwrap();
    cases.push(bad);
    let mut bad = valid.clone();
    bad.release = prior.input().recipe.schema.release.clone();
    cases.push(bad);
    for version in [2, 4] {
        let mut bad = valid.clone();
        bad.contract.schema_version = version;
        cases.push(bad);
    }
    for version in ["owned-domain-operations-v10", "owned-domain-operations-v12"] {
        let mut bad = valid.clone();
        bad.contract.operations_version = key(version);
        cases.push(bad);
    }
    let mut bad = valid.clone();
    bad.schema.reverse();
    cases.push(bad);
    let mut bad = valid.clone();
    bad.schema.insert(1, bad.schema[0].clone());
    cases.push(bad);
    let mut bad = valid.clone();
    let position = bad
        .schema
        .iter()
        .position(|row| {
            matches!(
                row,
                SchemaExtensionEntry::Definition(DefinitionDescriptor::Skill(_))
            )
        })
        .unwrap();
    let SchemaExtensionEntry::Definition(DefinitionDescriptor::Skill(entry)) =
        &bad.schema[position]
    else {
        unreachable!()
    };
    bad.schema[position] = SchemaExtensionEntry::Definition(
        prior
            .assembled()
            .schema()
            .lookup_definition(&entry.id.address())
            .unwrap()
            .clone(),
    );
    cases.push(bad);
    let mut bad = valid.clone();
    let new = bad
        .schema
        .iter_mut()
        .find_map(|row| match row {
            SchemaExtensionEntry::Definition(DefinitionDescriptor::Actor(entry)) => Some(entry),
            _ => None,
        })
        .unwrap();
    new.id = ActorDefId::parse(new.id.namespace().clone(), "def.ffffffffffffffff").unwrap();
    cases.push(bad);
    let mut bad = valid.clone();
    bad.query_targets[0].query_set = key("missing-set");
    cases.push(bad);
    let mut bad = valid.clone();
    bad.query_targets[0].query_id = QueryId::new("missing-row").unwrap();
    cases.push(bad);
    let mut bad = valid.clone();
    bad.query_targets.push(bad.query_targets[0].clone());
    cases.push(bad);
    let mut bad = valid.clone();
    bad.query_targets.reverse();
    cases.push(bad);
    let mut bad = valid.clone();
    bad.query_targets[0].target = prior
        .query_sets()
        .iter()
        .find(|set| set.name == bad.query_targets[0].query_set)
        .unwrap()
        .queries
        .iter()
        .find(|row| row.id == bad.query_targets[0].query_id)
        .unwrap()
        .target
        .clone();
    cases.push(bad);
    // Every schema row is installed before ordinary rule type-checking runs.
    let mut bad = valid.clone();
    let parameter = bad
        .schema
        .iter_mut()
        .find_map(|row| match row {
            SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(entry)) => Some(entry),
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(parameter) = &mut parameter.schema else {
        unreachable!()
    };
    parameter.value = ValueSchema::Boolean;
    cases.push(bad);
    for (i, case) in cases.into_iter().enumerate() {
        assert!(
            compile_owned_release_migration(&prior, case, Default::default()).is_err(),
            "case {i}"
        );
    }
    let mut wire = serde_json::to_value(&valid).unwrap();
    wire["query_targets"][0]["metric"] = serde_json::json!({"injected":true});
    assert!(serde_json::from_value::<OwnedReleaseMigrationInput>(wire).is_err());
    let mut wire = serde_json::to_value(&valid).unwrap();
    wire["skip_preservation"] = true.into();
    assert!(serde_json::from_value::<OwnedReleaseMigrationInput>(wire).is_err());
}

#[test]
fn migration_preserves_existing_rule_contents_and_coverage_closures() {
    let prior = prior();
    let valid = actor_migration(&prior);
    let existing = prior
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|owner| !owner.programs.members.is_empty())
        .unwrap();
    let mut bad = valid.clone();
    let mut rewritten = existing.clone();
    rewritten.programs.members[0].context = RuleEntityKind::Environment;
    bad.owners.push(rewritten);
    let error = compile_owned_release_migration(&prior, bad, Default::default())
        .err()
        .unwrap();
    assert!(error.to_string().contains("rewrite existing program"));
    let partial = prior
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|owner| !owner.programs.is_complete())
        .unwrap();
    let mut bad = valid.clone();
    let mut owner = partial.clone();
    owner.programs.closure = SchemaClosure::Complete;
    bad.owners.push(owner);
    let error = compile_owned_release_migration(&prior, bad, Default::default())
        .err()
        .unwrap();
    assert!(error.to_string().contains("change prior closure"));
    let mut bad = valid.clone();
    bad.owners.push(bad.owners[0].clone());
    assert!(compile_owned_release_migration(&prior, bad, Default::default()).is_err());
    let mut bad = valid;
    let duplicate = bad.owners[0].programs.members[0].clone();
    bad.owners[0].programs.members.push(duplicate);
    assert!(compile_owned_release_migration(&prior, bad, Default::default()).is_err());
}

#[test]
fn migration_bounds_combined_inputs_allocations_and_prior_provenance() {
    let prior = prior();
    let valid = actor_migration(&prior);
    for limits in [
        OwnedReleaseLimits {
            max_artifact_bytes: 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_input_bytes: 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_validation_entries: 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_queries: 109,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_provenance_entries: 1,
            ..Default::default()
        },
    ] {
        assert!(compile_owned_release_migration(&prior, valid.clone(), limits).is_err());
    }
    let mut limits = OwnedReleaseLimits::default();
    limits.recipe.registry.max_entries = prior.input().recipe.registry.entries.len();
    assert!(compile_owned_release_migration(&prior, valid, limits).is_err());
    let headers = header(&prior);
    let result =
        compile_owned_release_migration(&prior, headers.clone(), Default::default()).unwrap();
    assert_eq!(result.query_sets(), prior.query_sets());
    assert_eq!(
        result.input().recipe.registry,
        prior.input().recipe.registry
    );
    let mut empty = headers;
    empty.before = result.receipt().input;
    empty.release = key("empty-rebrand");
    assert!(compile_owned_release_migration(&result, empty, Default::default()).is_err());
}

#[test]
fn migration_preflights_new_registry_rows_before_copying_the_prior_graph() {
    let prior = prior();
    let headers = header(&prior);
    // Discover the existing fixture's actual budget through the public boundary;
    // do not duplicate the release preflight's constituent accounting in this test.
    let (mut low, mut high) = (1, OwnedReleaseLimits::default().max_validation_entries);
    while low < high {
        let middle = low + (high - low) / 2;
        let limits = OwnedReleaseLimits {
            max_validation_entries: middle,
            ..Default::default()
        };
        match compile_owned_release_migration(&prior, headers.clone(), limits) {
            Ok(_) => high = middle,
            Err(OwnedReleaseError::Limit(_)) => low = middle + 1,
            Err(error) => panic!("unexpected header migration failure: {error}"),
        }
    }
    let mut registry = prior.assembled().registry().clone();
    let id: OptionDefId = registry.allocate_definition().unwrap();
    let mut added = headers;
    added.schema.push(SchemaExtensionEntry::Definition(
        DefinitionDescriptor::Option(record(id, OptionSchema {})),
    ));
    // One semantic addition retains two rows: its registry identity and schema.
    // With room for only one, failure must be in preflight, not final assembly.
    let error = compile_owned_release_migration(
        &prior,
        added.clone(),
        OwnedReleaseLimits {
            max_validation_entries: low + 1,
            ..Default::default()
        },
    )
    .err()
    .unwrap();
    assert!(matches!(
        error,
        OwnedReleaseError::Limit("migration prior and authoring entries")
    ));
    let result = compile_owned_release_migration(
        &prior,
        added,
        OwnedReleaseLimits {
            max_validation_entries: low + 2,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        result.input().recipe.registry.last_issued.get(),
        prior.input().recipe.registry.last_issued.get() + 1
    );
}

fn v3_header(prior: &StagedOwnedRelease) -> OwnedReleaseMigrationInput {
    let mut input = header(prior);
    input.schema_version = 3;
    input.release = key("explicit-occurrence-input-contract");
    input.reason = key("reviewed-occurrence-input-migration");
    input.contract.schema_version = 5;
    input.contract.schema_semantics_version = prior.input().recipe.schema.semantics_version.clone();
    input.contract.operations_version = key(OWNED_RULE_OPERATIONS_V17);
    input.contract.rule_semantics_version = prior.input().recipe.rules.semantics_version.clone();
    input
}

fn endpoint_metrics(
    prior: &StagedOwnedRelease,
    migration: &OwnedReleaseMigrationInput,
) -> OwnedReleaseEvaluationInput {
    assert!(migration.schema.is_empty());
    let mut schema = prior.input().recipe.schema.clone();
    schema.schema_version = migration.contract.schema_version;
    schema.release = migration.release.clone();
    schema.semantics_version = migration.contract.schema_semantics_version.clone();
    let schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    OwnedReleaseEvaluationInput {
        metrics: MetricMappingInput {
            schema_version: 1,
            namespace: schema.namespace().clone(),
            release: key("reviewed-occurrence-metrics"),
            definitions: schema.identity().clone(),
            bindings: vec![],
        },
        support: None,
    }
}

fn assert_source_inputs_preserved(prior: &StagedOwnedRelease, result: &StagedOwnedRelease) {
    assert_eq!(result.receipt().source, prior.receipt().source);
    let mut restored = result.input().clone();
    // Only dependency identities are restored. All source policy content, order,
    // queries and provenance preceding the one migration receipt must survive.
    restored.schema_version = prior.input().schema_version;
    restored.recipe = prior.input().recipe.clone();
    restored.mapping.definitions = prior.receipt().definitions.clone();
    restored.mapping.registry = prior.receipt().registry;
    restored.roles.definitions = prior.receipt().definitions.clone();
    restored.roles.mapping = prior.receipt().mapping;
    if let GemQualityPolicy::Attributes(quality) = &mut restored.normalization.gem_quality {
        quality.definitions = prior.receipt().definitions.clone();
    }
    if let Some(gems) = &mut restored.normalization.gem_inputs {
        gems.definitions = prior.receipt().definitions.clone();
    }
    restored.rewards.definitions = prior.receipt().definitions.clone();
    restored.rewards.mapping = prior.receipt().mapping;
    restored.items.definitions = prior.receipt().definitions.clone();
    restored.item_source.item_lines = prior.receipt().items;
    match (&mut restored.tree, &prior.input().tree) {
        (Some(after), Some(before)) => {
            assert!(
                after.content == before.content,
                "tree source content changed"
            );
            *after = before.clone();
        }
        (None, None) => {}
        _ => panic!("tree presence changed"),
    }
    restored.evaluation = prior.input().evaluation.clone();
    assert_eq!(
        restored.provenance.len(),
        prior.input().provenance.len() + 1
    );
    restored.provenance.pop();
    assert!(
        restored == *prior.input(),
        "inherited source inputs changed"
    );
}

#[test]
fn v3_migrates_only_reviewed_predecessors_and_preserves_partial_source_releases() {
    let original = prior();
    for schema in [4, 5] {
        for operations in [
            OWNED_RULE_OPERATIONS_V15,
            OWNED_RULE_OPERATIONS_V16,
            OWNED_RULE_OPERATIONS_V17,
        ] {
            let prior = v3_fixture::contract(
                original.input().clone(),
                schema,
                operations,
                "reviewed-prior",
            );
            let migration = v3_header(&prior);
            // V5/V17 still requires an actual semantic edit; a release rename is
            // not an excuse to mint another migration identity.
            if schema == 5 && operations == OWNED_RULE_OPERATIONS_V17 {
                assert!(matches!(
                    compile_owned_release_migration(&prior, migration.clone(), Default::default()),
                    Err(OwnedReleaseError::Invalid(
                        "migration contains no semantic changes"
                    ))
                ));
            }
            let mut migration = migration;
            migration.contract.rule_semantics_version = key("reviewed-v17-semantics");
            let authoring =
                digest_owned("owned-release-contract-migration-v3", &migration, 1_000_000).unwrap();
            let result =
                compile_owned_release_migration(&prior, migration.clone(), Default::default())
                    .unwrap();
            assert_eq!(result.input().schema_version, OWNED_RELEASE_VERSION);
            assert_eq!(result.input().recipe.schema.schema_version, 5);
            assert_eq!(
                result.input().recipe.rules.operations_version.as_str(),
                OWNED_RULE_OPERATIONS_V17
            );
            assert!(result.evaluation().is_none());
            assert!(
                result.input().recipe.schema.definitions == prior.input().recipe.schema.definitions,
                "definition content changed"
            );
            assert!(
                result.input().recipe.schema.slots == prior.input().recipe.schema.slots,
                "slot content changed"
            );
            assert!(
                result.input().recipe.registry == prior.input().recipe.registry,
                "registry content changed"
            );
            assert!(
                result.input().recipe.rules.owners == prior.input().recipe.rules.owners,
                "program content changed"
            );
            assert!(
                result.input().recipe.rules.tables == prior.input().recipe.rules.tables,
                "table content changed"
            );
            assert!(
                result.input().recipe.rules.receivers == prior.input().recipe.rules.receivers,
                "receiver content changed"
            );
            let mut routing = result.input().recipe.routing.clone();
            routing.definitions = prior.receipt().definitions.clone();
            assert!(
                routing == prior.input().recipe.routing,
                "routing content changed"
            );
            assert_eq!(
                result.input().provenance.last().unwrap().authoring_input,
                authoring
            );
            assert_source_inputs_preserved(&prior, &result);
            let repeated =
                compile_owned_release_migration(&prior, migration, Default::default()).unwrap();
            assert!(result.artifacts().eq(repeated.artifacts()));
            let rebuilt =
                assemble_owned_release(result.input().clone(), Default::default()).unwrap();
            assert!(result.artifacts().eq(rebuilt.artifacts()));
        }
    }
}

#[test]
fn v3_explicit_authority_replaces_schema_appends_allocations_and_preserves_query_scope() {
    let original = prior();
    let prior = v3_fixture::contract(
        original.input().clone(),
        4,
        OWNED_RULE_OPERATIONS_V15,
        "prior-v15",
    );
    let mut migration = actor_migration(&prior);
    let header = v3_header(&prior);
    migration.schema_version = header.schema_version;
    migration.contract = header.contract;
    for row in &mut migration.schema {
        if let SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(entry)) = row {
            let SchemaState::Known(schema) = &mut entry.schema else {
                unreachable!()
            };
            schema.skill_input = Some(SkillInputAuthority::AuthoredOrProjected);
            schema.sites = vec![ParameterSite::SkillParameter];
        }
    }
    let result =
        compile_owned_release_migration(&prior, migration.clone(), Default::default()).unwrap();
    assert_eq!(
        result.input().recipe.registry.last_issued.get(),
        prior.input().recipe.registry.last_issued.get() + 4
    );
    assert!(
        result
            .input()
            .recipe
            .registry
            .entries
            .starts_with(&prior.input().recipe.registry.entries)
    );
    let parameter = result.input().recipe.schema.slots.iter().find_map(|row| match row {
        SlotDescriptor::Parameter(entry) if matches!(&entry.schema, SchemaState::Known(schema) if schema.skill_input.is_some()) => Some(entry),
        _ => None,
    }).unwrap();
    let SchemaState::Known(parameter) = &parameter.schema else {
        unreachable!()
    };
    assert_eq!(
        parameter.skill_input,
        Some(SkillInputAuthority::AuthoredOrProjected)
    );
    assert_eq!(parameter.sites, [ParameterSite::SkillParameter]);
    // This authored authority cannot be reinterpreted by the old schema or
    // operation contracts, even if someone manually rebuilds outer identities.
    let mut old_schema = result.input().recipe.schema.clone();
    old_schema.schema_version = 4;
    assert!(OwnedDefinitionSchemaPackage::new(old_schema, Default::default()).is_err());
    let mut old_rules = result.input().recipe.rules.clone();
    old_rules.operations_version = key(OWNED_RULE_OPERATIONS_V15);
    let stored = poe_optimizer_data::owned_rules::OwnedRulePackage::new(
        old_rules,
        result.assembled().schema(),
        Default::default(),
    )
    .unwrap();
    assert!(
        poe_optimizer_engine::owned_rules::CompiledRulePackage::compile_stored(
            &stored,
            result.assembled().schema(),
            Default::default()
        )
        .is_err()
    );
    let mut source_restored = result.input().clone();
    for (before_set, after_set) in prior
        .query_sets()
        .iter()
        .zip(&mut source_restored.query_sets)
    {
        assert_eq!(before_set.name, after_set.name);
        assert_eq!(before_set.queries.len(), after_set.queries.len());
        for (before, after) in before_set.queries.iter().zip(&mut after_set.queries) {
            assert_eq!((&before.id, &before.metric), (&after.id, &after.metric));
            if let Some(change) = migration
                .query_targets
                .iter()
                .find(|row| row.query_set == before_set.name && row.query_id == before.id)
            {
                assert_eq!(after.target, change.target);
                after.target = before.target.clone();
            } else {
                assert_eq!(&*after, before);
            }
        }
    }
    let source_restored = assemble_owned_release(source_restored, Default::default()).unwrap();
    assert_source_inputs_preserved(&prior, &source_restored);
    // The same V5/V17 endpoint supports another explicitly reviewed declaration
    // replacement, without requiring an artificial version increase.
    let mut next = v3_header(&result);
    let mut slot = result.input().recipe.schema.slots.iter().find(|row| matches!(row,
        SlotDescriptor::Parameter(entry) if matches!(&entry.schema, SchemaState::Known(schema) if schema.skill_input.is_some())
    )).unwrap().clone();
    let SlotDescriptor::Parameter(entry) = &mut slot else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    schema.presence = SlotPresence::OptionalOnce;
    next.schema.push(SchemaExtensionEntry::Slot(slot));
    compile_owned_release_migration(&result, next, Default::default()).unwrap();
}

#[test]
fn v3_checks_exact_prior_endpoint_versions_and_existing_resource_limits() {
    let original = prior();
    let prior = v3_fixture::contract(
        original.input().clone(),
        4,
        OWNED_RULE_OPERATIONS_V15,
        "prior-v15",
    );
    let valid = v3_header(&prior);
    let mut cases = vec![];
    for version in [1, 2, 4] {
        let mut bad = valid.clone();
        bad.schema_version = version;
        cases.push(bad);
    }
    for version in [3, 4, 6] {
        let mut bad = valid.clone();
        bad.contract.schema_version = version;
        cases.push(bad);
    }
    for version in [
        OWNED_RULE_OPERATIONS_V15,
        OWNED_RULE_OPERATIONS_V16,
        "owned-domain-operations-v18",
    ] {
        let mut bad = valid.clone();
        bad.contract.operations_version = key(version);
        cases.push(bad);
    }
    let mut bad = valid.clone();
    bad.before = digest_owned("stale-v3-prior", &1, 100).unwrap();
    cases.push(bad);
    let mut bad = valid.clone();
    bad.release = prior.input().recipe.schema.release.clone();
    cases.push(bad);
    for (index, bad) in cases.into_iter().enumerate() {
        assert!(
            compile_owned_release_migration(&prior, bad, Default::default()).is_err(),
            "invalid endpoint case {index}"
        );
    }
    for (schema, operations) in [
        (3, OWNED_RULE_OPERATIONS_V15),
        (4, OWNED_RULE_OPERATIONS_V13),
        (5, OWNED_RULE_OPERATIONS_V14),
    ] {
        let unsupported = v3_fixture::contract(
            original.input().clone(),
            schema,
            operations,
            "unsupported-prior",
        );
        assert!(matches!(
            compile_owned_release_migration(
                &unsupported,
                v3_header(&unsupported),
                Default::default()
            ),
            Err(OwnedReleaseError::Invalid(
                "migration prior contract is unsupported"
            ))
        ));
    }
    for limits in [
        OwnedReleaseLimits {
            max_input_bytes: 1,
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
}

#[test]
fn v3_accepts_explicit_checked_metrics_and_never_drops_prior_evaluation() {
    let original = prior();
    let prior = v3_fixture::contract(
        original.input().clone(),
        4,
        OWNED_RULE_OPERATIONS_V15,
        "prior-v15",
    );
    let mut migration = v3_header(&prior);
    migration.evaluation = Some(endpoint_metrics(&prior, &migration));
    let result =
        compile_owned_release_migration(&prior, migration.clone(), Default::default()).unwrap();
    assert_eq!(
        result.input().schema_version,
        OWNED_EVALUATION_RELEASE_VERSION
    );
    assert_eq!(result.input().evaluation, migration.evaluation);
    assert!(result.artifacts().any(|(name, _)| name == "metrics.json"));
    assert_source_inputs_preserved(&prior, &result);
    let mut next = v3_header(&result);
    next.release = key("checked-next-metrics");
    assert!(matches!(
        compile_owned_release_migration(&result, next.clone(), Default::default()),
        Err(OwnedReleaseError::Invalid(
            "migration cannot drop prior evaluation artifacts"
        ))
    ));
    next.evaluation = result.input().evaluation.clone();
    assert!(matches!(
        compile_owned_release_migration(&result, next.clone(), Default::default()),
        Err(OwnedReleaseError::Invalid(
            "migration contains no semantic changes"
        ))
    ));
    // A semantic edit does not authorize silently repairing stale metric bindings.
    next.contract.rule_semantics_version = key("changed-evaluation-rules");
    assert!(matches!(
        compile_owned_release_migration(&result, next.clone(), Default::default()),
        Err(OwnedReleaseError::Evaluation(_))
    ));
    next.evaluation = Some(endpoint_metrics(&result, &next));
    let updated = compile_owned_release_migration(&result, next, Default::default()).unwrap();
    assert_source_inputs_preserved(&result, &updated);
}

#[test]
fn v3_preserves_support_artifact_presence_and_does_not_relax_stage_validation() {
    let prior = v3_fixture::evaluated_prior();
    let valid = v3_header(&prior);
    let mut missing_support = valid.clone();
    missing_support.evaluation = Some(endpoint_metrics(&prior, &valid));
    assert!(matches!(
        compile_owned_release_migration(&prior, missing_support, Default::default()),
        Err(OwnedReleaseError::Invalid(
            "migration cannot drop prior support artifacts"
        ))
    ));
    let mut missing_outputs = valid.clone();
    missing_outputs.evaluation = prior.input().evaluation.clone();
    missing_outputs
        .evaluation
        .as_mut()
        .unwrap()
        .support
        .as_mut()
        .unwrap()
        .outputs = None;
    assert!(matches!(
        compile_owned_release_migration(&prior, missing_outputs, Default::default()),
        Err(OwnedReleaseError::Invalid(
            "migration cannot drop prior support outputs"
        ))
    ));
    let mut incompatible_stages = valid;
    incompatible_stages.evaluation = prior.input().evaluation.clone();
    let endpoint = endpoint_metrics(&prior, &incompatible_stages);
    let evaluation = incompatible_stages.evaluation.as_mut().unwrap();
    evaluation.metrics.definitions = endpoint.metrics.definitions.clone();
    // Rebind only immediate endpoint identities so the original V1 stages reach
    // the real V17 readiness gate. No readiness declarations are fabricated.
    let mut recipe = prior.input().recipe.clone();
    recipe.schema.schema_version = 5;
    recipe.schema.release = incompatible_stages.release.clone();
    recipe.rules.definitions = endpoint.metrics.definitions.clone();
    recipe.rules.operations_version = key(OWNED_RULE_OPERATIONS_V17);
    recipe.routing.definitions = endpoint.metrics.definitions.clone();
    let recipe =
        poe_optimizer_import::owned_recipe::assemble_owned_recipe(recipe, Default::default())
            .unwrap();
    let support = evaluation.support.as_mut().unwrap();
    support.stages.definitions = endpoint.metrics.definitions;
    support.stages.rules = *recipe.rules().identity();
    support.stages.routing = *recipe.routing().identity();
    let error = compile_owned_release_migration(&prior, incompatible_stages, Default::default())
        .err()
        .unwrap();
    assert!(matches!(error, OwnedReleaseError::Evaluation(_)));
    assert!(error.to_string().contains("readiness requires"), "{error}");
}
