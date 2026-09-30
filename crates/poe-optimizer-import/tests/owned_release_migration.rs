//! Explicit contract migration preserves historical releases and ordered requests.
#[path = "support/owned_compact_fixture.rs"]
mod fixture;
use poe_optimizer_core::{
    owned_build::{ParameterValue, QueryId},
    owned_content::digest_owned,
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_import::{
    owned_normalize::{GemQualityPolicy, ImportQueryTarget, ImportQueryTemplate},
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::*,
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
    }
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
