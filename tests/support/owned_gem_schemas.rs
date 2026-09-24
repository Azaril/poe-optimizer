//! Explicit physical-Gem schema migration through the checked, source-free CLI.
use super::{
    scalar_families::recipe,
    support::{bundle, json, normalize, success},
};
use poe_optimizer_core::{
    owned_build::DeclaredSlot,
    owned_definitions::*,
    owned_draft::{DraftLimits, decode_draft},
    owned_schema::*,
};
use poe_optimizer_import::{
    owned_gem_schema::{GemSchemaMigrationInput, stage_owned_gem_schema},
    owned_mapping::OwnedMappingIndex,
    owned_normalize::NormalizationPolicy,
    owned_recipe::{StagedOwnedRecipe, assemble_owned_recipe},
    owned_skill_catalog::{
        OwnedGemMaterialization, OwnedGemRole, OwnedGemRoleRow, OwnedPrimarySkill,
        OwnedSkillRoleIndex,
    },
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn migrate(cwd: &Path, prior: &Path, migration: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("migrate-owned-gem-schemas")
        .arg(prior)
        .arg("--migration")
        .arg(migration)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
fn republish(cwd: &Path, prior: &Path, policy: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("publish-owned-normalization")
        .arg(prior)
        .arg("--normalization")
        .arg(policy)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
fn write(path: &Path, value: &impl serde::Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn partial<T>(gem: &GemDefId, members: Vec<T>) -> DeclaredSet<T> {
    DeclaredSet::partial(
        members,
        vec![SchemaGap {
            subject: SchemaSubject::Definition(gem.address()),
            facet: SchemaFacet::InputSchema,
            code: OwnedDefinitionKey::new("caller-gem-input-review-incomplete").unwrap(),
        }],
    )
}
fn authored(
    base: &StagedOwnedRecipe,
    mapping: &OwnedMappingIndex,
    roles: &OwnedSkillRoleIndex,
    selected: &[&OwnedGemRoleRow],
) -> GemSchemaMigrationInput {
    let quality = base
        .schema()
        .input()
        .definitions
        .iter()
        .find_map(|definition| match definition {
            DefinitionDescriptor::Quality(row) if matches!(row.schema, SchemaState::Known(_)) => {
                Some(row.id.clone())
            }
            _ => None,
        })
        .unwrap();
    let mut registry = base.registry().clone();
    let mut gems = vec![];
    let mut parameters = vec![];
    for row in selected {
        let (OwnedGemRole::Known(role), OwnedPrimarySkill::Known(primary)) =
            (&row.role, &row.primary)
        else {
            panic!("caller selected unresolved role metadata")
        };
        let mut slots = vec![];
        for value in [
            ValueSchema::Boolean,
            ValueSchema::Integer(IntegerRange {
                minimum: BoundedInteger::new(-8).unwrap(),
                maximum: BoundedInteger::new(8).unwrap(),
            }),
        ] {
            let slot: DeclaredSlot<ParameterSlotDefId> = registry
                .allocate_slot(SlotOwnerDefId::Gem(row.gem.clone()))
                .unwrap();
            slots.push(slot.clone());
            parameters.push(DefinitionEntry {
                id: slot,
                schema: SchemaState::Known(ParameterSlotSchema {
                    value,
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::GemParameter],
                }),
            });
        }
        gems.push(DefinitionEntry {
            id: row.gem.clone(),
            schema: SchemaState::Known(GemSchema {
                level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(1).unwrap(),
                },
                roles: vec![*role],
                skills: partial(&row.gem, vec![primary.clone()]),
                quality: QualityUseSchema {
                    presence: QualityPresence::Optional,
                    allowed_kinds: partial(&row.gem, vec![quality.clone()]),
                },
                declarations: DeclaredSlots {
                    parameters: partial(&row.gem, slots),
                    choices: partial(&row.gem, vec![]),
                    grants: partial(&row.gem, vec![]),
                    actors: partial(&row.gem, vec![]),
                    skill_grants: partial(&row.gem, vec![]),
                    outputs: partial(&row.gem, vec![]),
                    sockets: partial(&row.gem, vec![]),
                },
            }),
        });
    }
    GemSchemaMigrationInput {
        schema_version: 1,
        before: base.schema().identity().clone(),
        mapping: *mapping.identity(),
        roles: *roles.identity(),
        source: mapping.input().source.clone(),
        gems,
        parameters,
    }
}
fn rejected(output: Output, destination: &Path) {
    assert!(
        !output.status.success(),
        "invalid Gem migration unexpectedly succeeded"
    );
    assert!(!output.stderr.is_empty());
    assert!(
        !destination.exists(),
        "failed migration created a destination"
    );
}

pub(super) fn check_gem_schemas(cwd: &Path, prior: &Path) -> PathBuf {
    let before_bytes = bundle(prior);
    let base = assemble_owned_recipe(recipe(prior), Default::default()).unwrap();
    let mapping = OwnedMappingIndex::new(
        serde_json::from_value(json(prior.join("mapping.json"))).unwrap(),
        base.registry(),
        base.schema(),
        Default::default(),
    )
    .unwrap();
    let roles = OwnedSkillRoleIndex::new(
        serde_json::from_value(json(prior.join("roles.json"))).unwrap(),
        &mapping,
        base.schema(),
        Default::default(),
    )
    .unwrap();
    // Selection follows explicit ownership/role/schema metadata, never source
    // names or the active original build. This is authored test data, not game policy.
    let mut eligible: Vec<_> = roles
        .input()
        .roles
        .iter()
        .filter(|row| {
            matches!(row.materialization, OwnedGemMaterialization::Physical)
                && matches!(row.role, OwnedGemRole::Known(_))
                && matches!(row.primary, OwnedPrimarySkill::Known(_))
                && matches!(
                    base.schema().definition(&row.gem),
                    SchemaLookup::Unmapped(_)
                )
        })
        .collect();
    eligible.sort_by(|a, b| a.gem.cmp(&b.gem));
    assert!(eligible.len() >= 2);
    let migration = authored(&base, &mapping, &roles, &eligible[..2]);
    let staged =
        stage_owned_gem_schema(&base, &mapping, &roles, &migration, Default::default()).unwrap();
    assert_eq!(staged.receipt.promoted_gems, 2);
    assert_eq!(staged.receipt.allocated_parameters, 4);
    let migration_path = cwd.join("gem-schema-migration.json");
    write(&migration_path, &migration);
    let output = cwd.join("gem-schema-migration");
    let receipt = success(migrate(cwd, prior, &migration_path, &output));
    assert_eq!(
        receipt["migration"],
        serde_json::to_value(&staged.receipt).unwrap()
    );
    assert_eq!(recipe(&output), staged.successor);
    let transition = json(output.join("transition.json"));
    assert_eq!(transition["schema_refinement"]["schema_version"], 4);
    assert_eq!(transition["schema_policy"], "explicit_gem_schema_knowledge");
    assert_eq!(transition["query_rows"], 110);
    assert_eq!(transition["calculation"], "not_run");
    assert_eq!(transition["whole_build_parity"], "not_established");
    assert_eq!(
        transition["schema_refinement"],
        serde_json::to_value(&staged.refinement).unwrap()
    );

    // The neutral input policy survives a schema change, with only its explicit
    // schema bindings rewritten. An omitted policy would not establish closure.
    let old_policy: NormalizationPolicy =
        serde_json::from_value(json(prior.join("normalization.json"))).unwrap();
    let current_policy: NormalizationPolicy =
        serde_json::from_value(json(output.join("normalization.json"))).unwrap();
    assert!(old_policy.gem_inputs.is_some());
    assert_eq!(
        current_policy.gem_inputs.as_ref().unwrap().definitions,
        staged.receipt.after
    );
    let mut restored_policy = current_policy.clone();
    restored_policy.gem_inputs.as_mut().unwrap().definitions =
        old_policy.gem_inputs.as_ref().unwrap().definitions.clone();
    if let (
        poe_optimizer_import::owned_normalize::GemQualityPolicy::Attributes(new),
        poe_optimizer_import::owned_normalize::GemQualityPolicy::Attributes(old),
    ) = (&mut restored_policy.gem_quality, &old_policy.gem_quality)
    {
        new.definitions = old.definitions.clone();
    }
    assert_eq!(restored_policy, old_policy);

    // This operation loads and validates the persisted V4 metadata before it
    // republishes unchanged schema-bound inputs into a fresh directory.
    let reload = cwd.join("gem-schema-reloaded");
    success(republish(
        cwd,
        &output,
        &output.join("normalization.json"),
        &reload,
    ));
    let output_bytes = bundle(&output);
    let reload_bytes = bundle(&reload);
    for name in [
        "registry.json",
        "schema.json",
        "rules.json",
        "routing.json",
        "mapping.json",
        "roles.json",
        "normalization.json",
        "rewards.json",
        "items.json",
        "item-source.json",
        "tree-normalization.json",
    ] {
        assert_eq!(
            output_bytes[name], reload_bytes[name],
            "checked reload changed {name}"
        );
    }
    let mut query_rows = 0;
    for case in 1..=5 {
        let name = format!("queries-original-{case:02}.json");
        assert_eq!(before_bytes[&name], output_bytes[&name]);
        assert_eq!(output_bytes[&name], reload_bytes[&name]);
        query_rows += json(reload.join(&name)).as_array().unwrap().len();
        let normalized = cwd.join(format!("gem-schema-original-{case}"));
        let report = success(normalize(cwd, &reload, case, &normalized, true));
        assert_eq!(report["normalization_status"], "pending");
        assert_eq!(report["verification"]["calculation"], "not_run");
        let draft = decode_draft(
            &fs::read(normalized.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        assert_eq!(
            draft
                .input()
                .query_presets
                .members
                .iter()
                .map(|p| p.queries.requests.members.len())
                .sum::<usize>(),
            22
        );
        assert!(
            !draft
                .validate_limits(DraftLimits::default())
                .unwrap()
                .issues
                .is_empty()
        );
    }
    assert_eq!(query_rows, 110);

    let mut invalid = migration.clone();
    invalid.source.revision.push_str("-unbound");
    let bad = cwd.join("gem-schema-source-mismatch.json");
    write(&bad, &invalid);
    let destination = cwd.join("gem-schema-source-mismatch");
    rejected(migrate(cwd, prior, &bad, &destination), &destination);

    // Rebind the request so rejection tests Known rewriting, not a stale endpoint.
    let current = assemble_owned_recipe(recipe(&output), Default::default()).unwrap();
    let current_mapping = OwnedMappingIndex::new(
        serde_json::from_value(json(output.join("mapping.json"))).unwrap(),
        current.registry(),
        current.schema(),
        Default::default(),
    )
    .unwrap();
    let current_roles = OwnedSkillRoleIndex::new(
        serde_json::from_value(json(output.join("roles.json"))).unwrap(),
        &current_mapping,
        current.schema(),
        Default::default(),
    )
    .unwrap();
    let mut repeated = migration.clone();
    repeated.before = current.schema().identity().clone();
    repeated.mapping = *current_mapping.identity();
    repeated.roles = *current_roles.identity();
    repeated.source = current_mapping.input().source.clone();
    let bad = cwd.join("gem-schema-known-rewrite.json");
    write(&bad, &repeated);
    let destination = cwd.join("gem-schema-known-rewrite");
    rejected(migrate(cwd, &output, &bad, &destination), &destination);

    let provider = roles
        .input()
        .roles
        .iter()
        .find(|row| {
            matches!(row.materialization, OwnedGemMaterialization::ProviderOnly)
                && matches!(row.role, OwnedGemRole::Known(_))
                && matches!(row.primary, OwnedPrimarySkill::Known(_))
                && matches!(
                    base.schema().definition(&row.gem),
                    SchemaLookup::Unmapped(_)
                )
        })
        .unwrap();
    let invalid = authored(&base, &mapping, &roles, &[provider]);
    let bad = cwd.join("gem-schema-provider-only.json");
    write(&bad, &invalid);
    let destination = cwd.join("gem-schema-provider-only");
    rejected(migrate(cwd, prior, &bad, &destination), &destination);

    let corrupt = cwd.join("gem-schema-corrupt-metadata");
    fs::create_dir(&corrupt).unwrap();
    for (name, bytes) in &output_bytes {
        fs::write(corrupt.join(name), bytes).unwrap();
    }
    let mut metadata: Value = json(corrupt.join("transition.json"));
    metadata["schema_refinement"]["source"]["revision"] =
        Value::String("caller-unbound-source".into());
    write(&corrupt.join("transition.json"), &metadata);
    let destination = cwd.join("gem-schema-corrupt-reload");
    rejected(
        republish(
            cwd,
            &corrupt,
            &corrupt.join("normalization.json"),
            &destination,
        ),
        &destination,
    );
    assert_eq!(
        bundle(prior),
        before_bytes,
        "caller predecessor was modified"
    );
    assert_eq!(
        bundle(&output),
        output_bytes,
        "published migration was modified"
    );
    reload
}
