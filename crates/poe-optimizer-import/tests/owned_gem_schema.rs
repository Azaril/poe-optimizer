//! Explicit schema knowledge is separate from source admission and rule coverage.
#[path = "support/owned_compact_fixture.rs"]
mod fixture;
use poe_optimizer_core::{
    owned_build::DeclaredSlot, owned_content::digest_owned, owned_definitions::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    owned_gem_schema::*, owned_mapping::OwnedMappingIndex, owned_recipe::*, owned_skill_catalog::*,
    owned_successor::*,
};
use std::path::PathBuf;

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
struct Context {
    bundle: SuccessorBundleInput,
    base: StagedOwnedRecipe,
    mapping: OwnedMappingIndex,
    roles: OwnedSkillRoleIndex,
    input: GemSchemaMigrationInput,
}
fn partial<T>(gem: &GemDefId, members: Vec<T>) -> DeclaredSet<T> {
    DeclaredSet {
        members,
        closure: SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: SchemaSubject::Definition(gem.address()),
                facet: SchemaFacet::InputSchema,
                code: key("fixture-unconverted"),
            }],
        },
    }
}
fn descriptor(
    row: &OwnedGemRoleRow,
    slots: Vec<DeclaredSlot<ParameterSlotDefId>>,
) -> DefinitionEntry<GemDefId, GemSchema> {
    let (OwnedPrimarySkill::Known(primary), OwnedGemRole::Known(role)) = (&row.primary, &row.role)
    else {
        panic!("fixture role must be known")
    };
    DefinitionEntry {
        id: row.gem.clone(),
        schema: SchemaState::Known(GemSchema {
            level: IntegerRange {
                minimum: BoundedInteger::new(1).unwrap(),
                maximum: BoundedInteger::new(100).unwrap(),
            },
            roles: vec![*role],
            skills: partial(&row.gem, vec![primary.clone()]),
            quality: QualityUseSchema {
                presence: QualityPresence::Forbidden,
                allowed_kinds: DeclaredSet::complete(vec![]),
            },
            declarations: DeclaredSlots {
                parameters: DeclaredSet::complete(slots),
                choices: partial(&row.gem, vec![]),
                grants: partial(&row.gem, vec![]),
                actors: partial(&row.gem, vec![]),
                skill_grants: partial(&row.gem, vec![]),
                outputs: partial(&row.gem, vec![]),
                sockets: partial(&row.gem, vec![]),
            },
        }),
    }
}
fn context() -> Context {
    let mut bundle = fixture::input(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    // The early finite catalog is sufficient; there is no need to reconstruct
    // the much larger complete checkpoint for a Gem-only schema contract test.
    bundle.successor = bundle.prior.clone();
    context_from(bundle)
}
fn context_from(bundle: SuccessorBundleInput) -> Context {
    let base = assemble_owned_recipe(bundle.prior.clone(), Default::default()).unwrap();
    let mapping = OwnedMappingIndex::new(
        bundle.mapping.clone(),
        base.registry(),
        base.schema(),
        Default::default(),
    )
    .unwrap();
    let roles = OwnedSkillRoleIndex::new(
        bundle.roles.clone(),
        &mapping,
        base.schema(),
        Default::default(),
    )
    .unwrap();
    let row = roles
        .input()
        .roles
        .iter()
        .find(|r| {
            matches!(r.materialization, OwnedGemMaterialization::Physical)
                && matches!(r.primary, OwnedPrimarySkill::Known(_))
                && matches!(r.role, OwnedGemRole::Known(_))
                && matches!(base.schema().definition(&r.gem), SchemaLookup::Unmapped(_))
        })
        .unwrap();
    let mut registry = base.registry().clone();
    let slot = registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Gem(row.gem.clone()))
        .unwrap();
    let input = GemSchemaMigrationInput {
        schema_version: 1,
        before: base.schema().identity().clone(),
        mapping: *mapping.identity(),
        roles: *roles.identity(),
        source: mapping.input().source.clone(),
        gems: vec![descriptor(row, vec![slot.clone()])],
        parameters: vec![DefinitionEntry {
            id: slot,
            schema: SchemaState::Known(ParameterSlotSchema {
                value: ValueSchema::Boolean,
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::GemParameter],
            }),
        }],
    };
    Context {
        bundle,
        base,
        mapping,
        roles,
        input,
    }
}
fn stage(
    c: &Context,
    input: &GemSchemaMigrationInput,
) -> Result<StagedGemSchemaMigration, SuccessorBundleError> {
    stage_owned_gem_schema(&c.base, &c.mapping, &c.roles, input, Default::default())
}
fn gem(input: &mut GemSchemaMigrationInput) -> &mut GemSchema {
    let SchemaState::Known(s) = &mut input.gems[0].schema else {
        panic!()
    };
    s
}
fn bind(recipe: &mut OwnedRecipeInput) {
    let schema =
        OwnedDefinitionSchemaPackage::new(recipe.schema.clone(), Default::default()).unwrap();
    recipe.rules.definitions = schema.identity().clone();
    recipe.routing.definitions = schema.identity().clone();
}

#[test]
fn explicit_physical_gem_and_owned_required_input_preserve_all_other_data() {
    let c = context();
    let next = stage(&c, &c.input).unwrap();
    assert_eq!(next.receipt.promoted_gems, 1);
    assert_eq!(next.receipt.allocated_parameters, 1);
    assert_ne!(next.receipt.before, next.receipt.after);
    assert_eq!(
        next.successor.registry.entries.len(),
        c.base.registry().input().entries.len() + 1
    );
    let old_rows: Vec<_> = c
        .base
        .schema()
        .input()
        .definitions
        .iter()
        .filter(|row| row.address() != c.input.gems[0].id.address())
        .collect();
    let new_rows: Vec<_> = next
        .successor
        .schema
        .definitions
        .iter()
        .filter(|row| row.address() != c.input.gems[0].id.address())
        .collect();
    assert_eq!(old_rows, new_rows);
    let mut rules = next.successor.rules.clone();
    rules.definitions = c.base.rules().input().definitions.clone();
    assert_eq!(&rules, c.base.rules().input());
    let mut routing = next.successor.routing.clone();
    routing.definitions = c.base.routing().input().definitions.clone();
    assert_eq!(&routing, c.base.routing().input());
    let after = assemble_owned_recipe(next.successor.clone(), Default::default()).unwrap();
    let SchemaLookup::Known(schema) = after.schema().definition(&c.input.gems[0].id) else {
        panic!()
    };
    assert!(schema.declarations.parameters.is_complete());
    assert!(!schema.skills.is_complete());
    assert!(!schema.declarations.outputs.is_complete());
    let encoded =
        serde_json::to_vec(&SchemaDeclarationRefinement::from(next.refinement.clone())).unwrap();
    let decoded: SchemaDeclarationRefinement = serde_json::from_slice(&encoded).unwrap();
    decoded
        .validate_current_metadata(
            &next.refinement.before,
            &next.refinement.after,
            after.schema(),
        )
        .unwrap();

    let mut partial_input = c.input.clone();
    gem(&mut partial_input).declarations.parameters =
        partial(&c.input.gems[0].id, vec![c.input.parameters[0].id.clone()]);
    let partial_result = stage(&c, &partial_input).unwrap();
    assert_ne!(partial_result.refinement.after, next.refinement.after);
}

#[test]
fn stale_foreign_unlisted_and_unbounded_inputs_fail_before_publication() {
    let c = context();
    for case in 0..7 {
        let mut bad = c.input.clone();
        match case {
            0 => bad.before.content_sha256 = "0".repeat(64),
            1 => bad.mapping = digest_owned("wrong-mapping", &1, 100).unwrap(),
            2 => bad.roles = digest_owned("wrong-roles", &1, 100).unwrap(),
            3 => bad.source.revision.push('x'),
            4 => bad.schema_version = 2,
            5 => bad.gems.clear(),
            _ => bad.gems.push(bad.gems[0].clone()),
        }
        assert!(stage(&c, &bad).is_err(), "case {case}");
    }
    for limits in [
        GemSchemaMigrationLimits {
            max_entries: 1,
            ..Default::default()
        },
        GemSchemaMigrationLimits {
            max_wire_bytes: 1,
            ..Default::default()
        },
        GemSchemaMigrationLimits {
            max_entries: 0,
            ..Default::default()
        },
    ] {
        assert!(stage_owned_gem_schema(&c.base, &c.mapping, &c.roles, &c.input, limits).is_err());
    }
    let mut foreign = c.input.clone();
    let other = c
        .roles
        .input()
        .roles
        .iter()
        .find(|r| r.gem != c.input.gems[0].id)
        .unwrap();
    foreign.parameters[0].id.declaration = SlotOwnerDefId::Gem(other.gem.clone());
    assert!(stage(&c, &foreign).is_err());
    let mut unlisted = c.input.clone();
    unlisted.parameters[0].id.declaration = SlotOwnerDefId::Skill(match &other.primary {
        OwnedPrimarySkill::Known(s) => s.clone(),
        _ => panic!(),
    });
    assert!(stage(&c, &unlisted).is_err());
}

#[test]
fn unknown_or_provider_only_roles_and_unproved_closures_are_rejected() {
    let c = context();
    for materialization in [
        OwnedGemMaterialization::ProviderOnly,
        OwnedGemMaterialization::Unmapped {
            issue: key("unknown-physicality"),
        },
    ] {
        let mut input = c.roles.input().clone();
        input
            .roles
            .iter_mut()
            .find(|r| r.gem == c.input.gems[0].id)
            .unwrap()
            .materialization = materialization;
        let roles =
            OwnedSkillRoleIndex::new(input, &c.mapping, c.base.schema(), Default::default())
                .unwrap();
        let mut migration = c.input.clone();
        migration.roles = *roles.identity();
        assert!(
            stage_owned_gem_schema(&c.base, &c.mapping, &roles, &migration, Default::default())
                .is_err()
        );
    }
    for facet in 0..7 {
        let mut input = c.input.clone();
        let s = gem(&mut input);
        let closure = match facet {
            0 => &mut s.skills.closure,
            1 => &mut s.declarations.choices.closure,
            2 => &mut s.declarations.grants.closure,
            3 => &mut s.declarations.actors.closure,
            4 => &mut s.declarations.skill_grants.closure,
            5 => &mut s.declarations.outputs.closure,
            _ => &mut s.declarations.sockets.closure,
        };
        *closure = SchemaClosure::Complete;
        assert!(stage(&c, &input).is_err(), "facet {facet}");
    }
    let mut input = c.input.clone();
    gem(&mut input).skills.members.clear();
    assert!(stage(&c, &input).is_err());
    let mut input = c.input.clone();
    let original = gem(&mut input).roles[0];
    gem(&mut input).roles = vec![match original {
        AuthoredGemRole::SkillUse => AuthoredGemRole::SupportAssignment,
        AuthoredGemRole::SupportAssignment => AuthoredGemRole::SkillUse,
    }];
    assert!(stage(&c, &input).is_err());
}

#[test]
fn knowledge_migration_cannot_rewrite_known_gems_or_old_programs_or_registry_history() {
    let c = context();
    let next = stage(&c, &c.input).unwrap();
    let after = assemble_owned_recipe(next.successor.clone(), Default::default()).unwrap();
    let mut mapping = c.mapping.input().clone();
    mapping.registry = after.registry().identity().unwrap();
    mapping.definitions = after.schema().identity().clone();
    let mapping = OwnedMappingIndex::new(
        mapping,
        after.registry(),
        after.schema(),
        Default::default(),
    )
    .unwrap();
    let mut roles = c.roles.input().clone();
    roles.definitions = after.schema().identity().clone();
    roles.mapping = *mapping.identity();
    let roles =
        OwnedSkillRoleIndex::new(roles, &mapping, after.schema(), Default::default()).unwrap();
    let mut replay = c.input.clone();
    replay.before = after.schema().identity().clone();
    replay.mapping = *mapping.identity();
    replay.roles = *roles.identity();
    replay.parameters.clear();
    assert!(stage_owned_gem_schema(&after, &mapping, &roles, &replay, Default::default()).is_err());

    let mut changed = next.successor.clone();
    changed.rules.release = key("unrelated-rule-edit");
    let changed = assemble_owned_recipe(changed, Default::default()).unwrap();
    assert!(
        validate_gem_schema_refinement(&next.refinement, &c.base, &changed, &c.mapping, &c.roles)
            .is_err()
    );

    let mut changed = next.successor.clone();
    changed.schema.release = key("unrelated-schema-edit");
    bind(&mut changed);
    let changed = assemble_owned_recipe(changed, Default::default()).unwrap();
    let mut policy = next.refinement.clone();
    policy.after = changed.schema().identity().clone();
    assert!(
        validate_gem_schema_refinement(&policy, &c.base, &changed, &c.mapping, &c.roles).is_err()
    );

    let mut policy = next.refinement.clone();
    policy.parameters.clear();
    assert!(
        validate_gem_schema_refinement(&policy, &c.base, &after, &c.mapping, &c.roles).is_err()
    );
    let mut policy = next.refinement;
    policy.gems.clear();
    assert!(
        validate_gem_schema_refinement(&policy, &c.base, &after, &c.mapping, &c.roles).is_err()
    );
}

#[test]
fn compact_publisher_binds_roles_source_tree_and_every_prior_artifact() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let baseline = transition_owned_bundle(fixture::input(&root), Default::default()).unwrap();
    let input = fixture::next(&baseline);
    let prior = transition_owned_catalog_with_tree_compact(
        input.clone(),
        fixture::append(&input),
        fixture::tree(&input),
        Default::default(),
    )
    .unwrap();
    let c = context_from(fixture::next(&prior));
    let migration = stage(&c, &c.input).unwrap();
    let mut candidate = c.bundle.clone();
    candidate.successor = migration.successor.clone();
    let tree = TreePolicyTransitionInput::RebindPrior {
        prior: Box::new(prior.tree().unwrap().input().clone()),
    };
    let result = transition_owned_catalog_with_gem_refinement_compact(
        candidate.clone(),
        fixture::append(&candidate),
        tree.clone(),
        migration.refinement.clone(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(result.transition().before, prior.transition().after);
    assert_eq!(
        result.transition().schema_policy,
        "explicit_gem_schema_knowledge"
    );
    assert_eq!(
        result.transition().preserved_slots,
        prior.recipe().schema.slots.len()
    );
    assert_eq!(
        result.transition().preserved_definitions,
        prior.recipe().schema.definitions.len() - 1
    );
    assert_eq!(
        result.tree().unwrap().input().content,
        prior.tree().unwrap().input().content
    );
    assert_eq!(result.query_sets(), prior.query_sets());
    assert_eq!(
        result
            .query_sets()
            .iter()
            .map(|set| set.queries.len())
            .sum::<usize>(),
        110
    );
    let mut old_mapping = result.mapping().input().clone();
    old_mapping.registry = prior.mapping().input().registry;
    old_mapping.definitions = prior.mapping().input().definitions.clone();
    assert_eq!(&old_mapping, prior.mapping().input());
    let mut old_roles = result.roles().input().clone();
    old_roles.mapping = prior.roles().input().mapping;
    old_roles.definitions = prior.roles().input().definitions.clone();
    assert_eq!(&old_roles, prior.roles().input());
    let mut old_items = result.items().input().clone();
    old_items.definitions = prior.items().input().definitions.clone();
    assert_eq!(&old_items, prior.items().input());
    let mut old_source = result.item_source().input().clone();
    old_source.item_lines = prior.item_source().input().item_lines;
    assert_eq!(&old_source, prior.item_source().input());
    migration
        .refinement
        .validate_current_catalog(
            &result.transition().before,
            &result.transition().after,
            result.mapping(),
            result.roles(),
            result.assembled().schema(),
        )
        .unwrap();
    let mut stale = result.transition().before.clone();
    stale.roles = digest_owned("stale-prior-role", &0, 100).unwrap();
    assert!(
        migration
            .refinement
            .validate_current_catalog(
                &stale,
                &result.transition().after,
                result.mapping(),
                result.roles(),
                result.assembled().schema(),
            )
            .is_err()
    );
    assert!(
        transition_owned_catalog_with_gem_refinement_compact(
            candidate.clone(),
            fixture::append(&candidate),
            fixture::tree(&candidate),
            migration.refinement.clone(),
            Default::default(),
        )
        .is_err(),
        "V4 cannot replace checked prior tree content"
    );
    let mut append = fixture::append(&candidate);
    append.item_policies = CatalogItemPolicyMode::SuppliedSuccessor;
    assert!(
        transition_owned_catalog_with_gem_refinement_compact(
            candidate,
            append,
            tree,
            migration.refinement,
            Default::default(),
        )
        .is_err(),
        "V4 must preserve checked prior item policies"
    );
}

#[test]
fn slot_allocation_order_foreign_ids_and_unlisted_promotions_are_rejected() {
    let c = context();
    let mut registry = c.base.registry().clone();
    let first = registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Gem(c.input.gems[0].id.clone()))
        .unwrap();
    let second = registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Gem(c.input.gems[0].id.clone()))
        .unwrap();
    let mut input = c.input.clone();
    input.parameters[0].id = second;
    assert!(
        stage(&c, &input).is_err(),
        "cannot skip a registry allocation"
    );
    input = c.input.clone();
    let mut extra = input.parameters[0].clone();
    extra.id = registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Gem(c.input.gems[0].id.clone()))
        .unwrap();
    input.parameters = vec![extra, input.parameters[0].clone()];
    assert!(
        stage(&c, &input).is_err(),
        "allocation list must be ordered"
    );
    input = c.input.clone();
    input.gems[0].id = GemDefId::new(
        GameVersionNamespace::new("other", "version").unwrap(),
        c.input.gems[0].id.key().clone(),
    );
    assert!(stage(&c, &input).is_err());
    input = c.input.clone();
    let SchemaState::Known(slot) = &mut input.parameters[0].schema else {
        panic!()
    };
    slot.sites = vec![ParameterSite::ModifierRoll];
    assert!(stage(&c, &input).is_err());
    assert_eq!(first, c.input.parameters[0].id);

    let mut input = c.input.clone();
    let extra_row = c
        .roles
        .input()
        .roles
        .iter()
        .find(|r| {
            r.gem != input.gems[0].id
                && matches!(
                    c.base.schema().definition(&r.gem),
                    SchemaLookup::Unmapped(_)
                )
                && matches!(r.materialization, OwnedGemMaterialization::Physical)
                && matches!(r.primary, OwnedPrimarySkill::Known(_))
                && matches!(r.role, OwnedGemRole::Known(_))
        })
        .unwrap();
    input.gems.push(descriptor(extra_row, vec![]));
    input.gems.sort_by(|a, b| a.id.cmp(&b.id));
    let next = stage(&c, &input).unwrap();
    let after = assemble_owned_recipe(next.successor, Default::default()).unwrap();
    let mut policy = next.refinement;
    policy.gems.retain(|id| id == &c.input.gems[0].id);
    assert!(
        validate_gem_schema_refinement(&policy, &c.base, &after, &c.mapping, &c.roles).is_err()
    );
}

#[test]
fn old_wire_shapes_are_unchanged_and_v3_cannot_promote_gems() {
    let c = context();
    let next = stage(&c, &c.input).unwrap();
    let before = next.refinement.before.clone();
    let after = next.refinement.after.clone();
    let v1 = PassiveDeclarationRefinement {
        schema_version: 1,
        before: before.clone(),
        after: after.clone(),
        nodes: vec![],
    };
    let v2 = DeclarationClosureRefinement {
        schema_version: 2,
        before: before.clone(),
        after: after.clone(),
        owners: vec![],
    };
    let v3 = SchemaMembershipRefinement {
        schema_version: 3,
        before,
        after,
        subjects: vec![SchemaSubject::Definition(c.input.gems[0].id.address())],
    };
    for (plain, wrapped) in [
        (
            serde_json::to_vec(&v1).unwrap(),
            SchemaDeclarationRefinement::from(v1),
        ),
        (
            serde_json::to_vec(&v2).unwrap(),
            SchemaDeclarationRefinement::from(v2),
        ),
        (
            serde_json::to_vec(&v3).unwrap(),
            SchemaDeclarationRefinement::from(v3.clone()),
        ),
    ] {
        assert_eq!(serde_json::to_vec(&wrapped).unwrap(), plain);
        let restored: SchemaDeclarationRefinement = serde_json::from_slice(&plain).unwrap();
        assert_eq!(restored, wrapped);
    }
    let after = assemble_owned_recipe(next.successor, Default::default()).unwrap();
    assert!(validate_schema_membership_refinement(&v3, &c.base, &after).is_err());
    let mut mixed = serde_json::to_value(next.refinement).unwrap();
    mixed["subjects"] = serde_json::json!([]);
    assert!(serde_json::from_value::<SchemaDeclarationRefinement>(mixed).is_err());
}

#[test]
fn mapping_checked_against_a_different_registry_cannot_author_a_prior_migration() {
    let c = context();
    let mut foreign_registry = c.base.registry().clone();
    let _: UnitDefId = foreign_registry.allocate_definition().unwrap();
    let mut mapping = c.mapping.input().clone();
    mapping.registry = foreign_registry.identity().unwrap();
    let mapping = OwnedMappingIndex::new(
        mapping,
        &foreign_registry,
        c.base.schema(),
        Default::default(),
    )
    .unwrap();
    let mut roles = c.roles.input().clone();
    roles.mapping = *mapping.identity();
    let roles =
        OwnedSkillRoleIndex::new(roles, &mapping, c.base.schema(), Default::default()).unwrap();
    let mut input = c.input.clone();
    input.mapping = *mapping.identity();
    input.roles = *roles.identity();
    assert!(stage_owned_gem_schema(&c.base, &mapping, &roles, &input, Default::default()).is_err());
}
