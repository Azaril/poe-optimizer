//! Injected identity facts only: no bundled snapshot, source VM or game runtime.
use poe_optimizer_core::{owned_definitions::*, owned_schema::*};
use poe_optimizer_data::{owned_schema::*, skill_identities::*};
use poe_optimizer_import::{owned_mapping::*, owned_skill_catalog::*};
use std::collections::BTreeMap;

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("owned-test", "v1").unwrap()
}
fn limits() -> SkillCatalogLimits {
    SkillCatalogLimits::default()
}
fn policy(absent_support: AbsentSupportPolicy) -> SkillCatalogPolicy {
    SkillCatalogPolicy {
        version: key("test-role-policy-v1"),
        absent_support,
        absent_from_tree: AbsentFromTreePolicy::Physical,
    }
}
fn span() -> IdentitySourceSpan {
    IdentitySourceSpan {
        path: "fixture/identities.json".into(),
        line: 1,
        end_line: 1,
        sha256: "a".repeat(64),
    }
}
fn source() -> SkillIdentitySource {
    SkillIdentitySource {
        upstream_revision: "c".repeat(40),
        files: BTreeMap::from([("fixture/identities.json".into(), "a".repeat(64))]),
        skill_module_order: vec!["fixture/identities.json".into()],
        skill_assembly: span(),
        gem_assembly: span(),
        load_skill: span(),
    }
}
fn pin() -> SourcePin {
    SourcePin {
        system: ExternalSourceSystem::PathOfBuilding2,
        revision: "c".repeat(40),
        files: vec![SourceFilePin {
            path: "fixture/identities.json".into(),
            sha256: "a".repeat(64),
        }],
    }
}
fn add_skill(data: &mut SkillIdentityData, id: &str, support: Option<bool>) {
    let index = data.skill_declarations.len() as u32 + 1;
    data.skill_declarations.push(SkillIdentityDeclaration {
        index,
        id: id.into(),
        source: span(),
        identity: DeclaredSkillIdentity {
            name: id.into(),
            base_type_name: None,
            support,
            from_tree: None,
        },
    });
    data.skills.push(SkillIdentity {
        id: id.into(),
        name: id.into(),
        base_type_name: None,
        support,
        from_tree: None,
        winning_declaration: index,
    });
}
fn add_gem(data: &mut SkillIdentityData, name: &str, game: &str, variant: &str, effect: &str) {
    let index = data.gem_declarations.len() as u32 + 1;
    data.gem_declarations.push(GemIdentityDeclaration {
        index,
        key: name.into(),
        source: span(),
        identity: DeclaredGemIdentity {
            game_id: game.into(),
            variant_id: variant.into(),
            name: name.into(),
            name_spec: None,
            base_type_name: None,
            primary_effect_id: effect.into(),
            additional_effects: vec![],
            additional_stat_sets: vec![],
            display_order: None,
        },
    });
    data.gems.push(GemIdentity {
        key: name.into(),
        game_id: game.into(),
        variant_id: variant.into(),
        name: name.into(),
        name_spec: None,
        base_type_name: None,
        primary_effect_id: effect.into(),
        declared_additional_effects: vec![],
        declared_additional_stat_sets: vec![],
        constructed_additional_effects: vec![],
        additional_effects: vec![],
        effect_list: if data.skills.iter().any(|s| s.id == effect) {
            vec![effect.into()]
        } else {
            vec![]
        },
        display_order: None,
        winning_declaration: index,
    });
    data.missing_references = data.expected_missing_references();
}
fn data() -> SkillIdentityData {
    let mut data = SkillIdentityData {
        schema_version: SKILL_IDENTITY_SCHEMA_VERSION,
        capability: SkillIdentityCapability::IdentityOnly,
        source: source(),
        gem_declarations: vec![],
        skill_declarations: vec![],
        gems: vec![],
        skills: vec![],
        missing_references: vec![],
    };
    add_skill(&mut data, "support-effect", Some(true));
    add_skill(&mut data, "active-effect", None);
    add_gem(
        &mut data,
        "support-gem-key",
        "external-support",
        "v-a",
        "support-effect",
    );
    add_gem(
        &mut data,
        "active-gem-key",
        "external-active",
        "v-b",
        "active-effect",
    );
    data
}
fn base() -> OwnedIdRegistry {
    OwnedIdRegistry::empty(ns(), limits().mapping).unwrap()
}
fn compile(data: SkillIdentityData, absent: AbsentSupportPolicy) -> FreshOwnedSkillCatalog {
    compile_fresh_owned_skill_catalog(
        &SkillIdentityCatalog::new(data).unwrap(),
        &base(),
        &pin(),
        &policy(absent),
        limits(),
    )
    .unwrap()
}
fn selector(game: &str, variant: &str) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::Gem {
        game_id: SourceComponent::Text(game.into()),
        variant_id: SourceComponent::Text(variant.into()),
    })
}
fn gem_id(compiled: &FreshOwnedSkillCatalog, game: &str, variant: &str) -> GemDefId {
    let source = selector(game, variant);
    let entry = compiled
        .mappings
        .iter()
        .find(|entry| entry.source == source)
        .unwrap();
    let MappingOutcome::Mapped {
        target: SchemaSubject::Definition(DefinitionAddress::Gem(id)),
        ..
    } = &entry.outcome
    else {
        panic!("expected one exact gem mapping")
    };
    id.clone()
}
fn assemble(
    compiled: &FreshOwnedSkillCatalog,
) -> (
    OwnedDefinitionSchemaPackage,
    OwnedMappingIndex,
    OwnedSkillRolePackageInput,
) {
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: ns(),
            release: key("release"),
            semantics_version: key("identity-schema-v1"),
            definitions: compiled.definitions.clone(),
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let mappings = OwnedMappingIndex::new(
        MappingPackageInput {
            schema_version: OWNED_MAPPING_PACKAGE_VERSION,
            namespace: ns(),
            registry: compiled.registry.identity().unwrap(),
            definitions: schema.identity().clone(),
            source: pin(),
            policy_version: compiled.receipt.policy.version.clone(),
            entries: compiled.mappings.clone(),
        },
        &compiled.registry,
        &schema,
        limits().mapping,
    )
    .unwrap();
    let input = role_input(compiled, &schema, &mappings);
    (schema, mappings, input)
}
fn role_input(
    compiled: &FreshOwnedSkillCatalog,
    schema: &impl DefinitionSchemaIndex,
    mappings: &OwnedMappingIndex,
) -> OwnedSkillRolePackageInput {
    OwnedSkillRolePackageInput {
        schema_version: OWNED_SKILL_ROLE_VERSION,
        namespace: ns(),
        definitions: schema.identity().clone(),
        mapping: *mappings.identity(),
        compilation: compiled.receipt.clone(),
        roles: compiled.roles.clone(),
    }
}

#[test]
fn exact_identity_and_roles_bind_without_claiming_complete_input_schema() {
    let compiled = compile(data(), AbsentSupportPolicy::NonSupport);
    assert_eq!(compiled.definitions.len(), 4);
    assert_eq!(compiled.mappings.len(), 4);
    assert_eq!(compiled.roles.len(), 2);
    for definition in &compiled.definitions {
        match definition {
            DefinitionDescriptor::Gem(row) => {
                assert!(matches!(row.schema, SchemaState::Unmapped { .. }))
            }
            DefinitionDescriptor::Skill(row) => {
                assert!(matches!(row.schema, SchemaState::Unmapped { .. }))
            }
            other => panic!("unexpected definition: {other:?}"),
        }
    }
    let (schema, mappings, input) = assemble(&compiled);
    let index = OwnedSkillRoleIndex::new(input.clone(), &mappings, &schema, limits()).unwrap();
    let support = gem_id(&compiled, "external-support", "v-a");
    let active = gem_id(&compiled, "external-active", "v-b");
    assert_eq!(
        index.role(&support).unwrap().role,
        OwnedGemRole::Known(AuthoredGemRole::SupportAssignment)
    );
    assert_eq!(
        index.role(&active).unwrap().role,
        OwnedGemRole::Known(AuthoredGemRole::SkillUse)
    );
    assert_eq!(
        index.lookup(&selector("external-support", "v-a")),
        mappings.lookup(&selector("external-support", "v-a"))
    );
    assert!(
        index
            .lookup(&selector("external-support", "unknown-variant"))
            .is_none()
    );
    assert!(
        index
            .lookup(&ExternalSelector::Definition(ExternalOwnerSelector::Gem {
                game_id: SourceComponent::Text("external-support".into()),
                variant_id: SourceComponent::Missing,
            }))
            .is_none()
    );
    let decoded: OwnedSkillRolePackageInput =
        serde_json::from_slice(&serde_json::to_vec(&input).unwrap()).unwrap();
    let rebuilt = OwnedSkillRoleIndex::new(decoded, &mappings, &schema, limits()).unwrap();
    assert_eq!(index.identity(), rebuilt.identity());
}

#[test]
fn absent_support_policy_is_explicit_and_injected_role_changes_are_observed() {
    let pending = compile(data(), AbsentSupportPolicy::Pending);
    let active = gem_id(&pending, "external-active", "v-b");
    assert!(matches!(
        pending
            .roles
            .iter()
            .find(|row| row.gem == active)
            .unwrap()
            .role,
        OwnedGemRole::Unmapped { .. }
    ));
    let non_support = compile(data(), AbsentSupportPolicy::NonSupport);
    assert_eq!(pending.definitions, non_support.definitions);
    assert_eq!(pending.mappings, non_support.mappings);
    assert_ne!(pending.roles, non_support.roles);
    let mut changed = data();
    changed.skills[0].support = Some(false);
    changed.skill_declarations[0].identity.support = Some(false);
    let changed = compile(changed, AbsentSupportPolicy::Pending);
    let support = gem_id(&changed, "external-support", "v-a");
    assert_eq!(
        changed
            .roles
            .iter()
            .find(|row| row.gem == support)
            .unwrap()
            .role,
        OwnedGemRole::Known(AuthoredGemRole::SkillUse)
    );
    assert_ne!(
        pending.receipt.catalog_digest,
        changed.receipt.catalog_digest
    );
}

#[test]
fn compilation_is_deterministic_from_the_same_base_and_explicitly_appends_fresh_ids() {
    let catalog = SkillIdentityCatalog::new(data()).unwrap();
    let base = base();
    let before = base.input().clone();
    let a = compile_fresh_owned_skill_catalog(
        &catalog,
        &base,
        &pin(),
        &policy(AbsentSupportPolicy::NonSupport),
        limits(),
    )
    .unwrap();
    let b = compile_fresh_owned_skill_catalog(
        &catalog,
        &base,
        &pin(),
        &policy(AbsentSupportPolicy::NonSupport),
        limits(),
    )
    .unwrap();
    assert_eq!(base.input(), &before);
    assert_eq!(a.registry.input(), b.registry.input());
    assert_eq!(a.definitions, b.definitions);
    assert_eq!(a.mappings, b.mappings);
    assert_eq!(a.roles, b.roles);
    assert_eq!(a.receipt, b.receipt);
    let appended = compile_fresh_owned_skill_catalog(
        &catalog,
        &a.registry,
        &pin(),
        &policy(AbsentSupportPolicy::NonSupport),
        limits(),
    )
    .unwrap();
    assert_eq!(appended.registry.input().entries.len(), 8);
    assert_eq!(
        &appended.registry.input().entries[..4],
        &a.registry.input().entries
    );
    assert_ne!(
        gem_id(&a, "external-support", "v-a"),
        gem_id(&appended, "external-support", "v-a")
    );
    assert_eq!(
        appended.receipt.base_registry,
        a.registry.identity().unwrap()
    );
}

#[test]
fn colliding_exact_selectors_remain_ambiguous_without_selecting_a_role() {
    let mut source = data();
    add_gem(
        &mut source,
        "other-support-key",
        "external-support",
        "v-a",
        "active-effect",
    );
    let compiled = compile(source, AbsentSupportPolicy::NonSupport);
    let (schema, mappings, input) = assemble(&compiled);
    let index = OwnedSkillRoleIndex::new(input, &mappings, &schema, limits()).unwrap();
    let Some(MappingOutcome::Ambiguous { candidates, .. }) =
        index.lookup(&selector("external-support", "v-a"))
    else {
        panic!("selector collision must remain ambiguous")
    };
    assert_eq!(candidates.len(), 2);
    let roles: Vec<_> = candidates
        .iter()
        .map(|candidate| {
            let SchemaSubject::Definition(DefinitionAddress::Gem(id)) = candidate else {
                panic!("wrong target kind")
            };
            &index.role(id).unwrap().role
        })
        .collect();
    assert!(roles.contains(&&OwnedGemRole::Known(AuthoredGemRole::SkillUse)));
    assert!(roles.contains(&&OwnedGemRole::Known(AuthoredGemRole::SupportAssignment)));
}

#[test]
fn ambiguous_catalog_rows_remain_diagnostic_but_cannot_certify_exchanged_owned_observations() {
    let mut source = data();
    add_gem(
        &mut source,
        "same-primary-other-key",
        "external-support",
        "v-a",
        "support-effect",
    );
    // The same source selector/primary/role can still have different construction.
    let stat_set = IndexedIdentityReference {
        index: 3,
        id: "other-stat-set".into(),
    };
    source.gem_declarations[2].identity.additional_stat_sets = vec![stat_set.clone()];
    source.gems[2].declared_additional_stat_sets = vec![stat_set];
    let catalog = SkillIdentityCatalog::new(source.clone()).unwrap();
    let compiled = compile(source, AbsentSupportPolicy::NonSupport);
    let (schema, mapping, input) = assemble(&compiled);
    let original = OwnedSkillRoleIndex::new(input.clone(), &mapping, &schema, limits()).unwrap();
    assert!(matches!(
        mapping.lookup(&selector("external-support", "v-a")),
        Some(MappingOutcome::Ambiguous { .. })
    ));
    assert!(matches!(
        verify_owned_skill_catalog_reproduction(&catalog, &original, &mapping, limits()),
        Err(SkillCatalogError::ReproductionMismatch)
    ));
    let first = input
        .roles
        .iter()
        .position(|row| observed(row).catalog_gem_key == "support-gem-key")
        .unwrap();
    let second = input
        .roles
        .iter()
        .position(|row| observed(row).catalog_gem_key == "same-primary-other-key")
        .unwrap();
    assert_eq!(input.roles[first].primary, input.roles[second].primary);
    assert_eq!(input.roles[first].role, input.roles[second].role);
    assert_eq!(
        input.roles[first].materialization,
        input.roles[second].materialization
    );
    let mut changed = input.clone();
    changed.roles[first].construction = input.roles[second].construction.clone();
    changed.roles[second].construction = input.roles[first].construction.clone();
    let exchanged = OwnedSkillRoleIndex::new(changed, &mapping, &schema, limits()).unwrap();
    assert_ne!(original.identity(), exchanged.identity());
    assert!(matches!(
        verify_owned_skill_catalog_reproduction(&catalog, &exchanged, &mapping, limits()),
        Err(SkillCatalogError::ReproductionMismatch)
    ));
}

#[test]
fn missing_primary_effect_does_not_invent_a_skill_definition_or_support_role() {
    let mut source = data();
    add_gem(
        &mut source,
        "missing-primary",
        "external-missing",
        "v-c",
        "absent-effect",
    );
    let compiled = compile(source, AbsentSupportPolicy::NonSupport);
    let missing = gem_id(&compiled, "external-missing", "v-c");
    let row = compiled
        .roles
        .iter()
        .find(|row| row.gem == missing)
        .unwrap();
    assert!(matches!(row.primary, OwnedPrimarySkill::Unmapped { .. }));
    assert!(matches!(row.role, OwnedGemRole::Unmapped { .. }));
    assert!(matches!(
        row.materialization,
        OwnedGemMaterialization::Unmapped { .. }
    ));
    assert_eq!(compiled.receipt.skill_count, 2);
    let (schema, mapping, input) = assemble(&compiled);
    assert!(OwnedSkillRoleIndex::new(input, &mapping, &schema, limits()).is_ok());
}

#[test]
fn source_and_final_binding_conflicts_are_rejected() {
    let catalog = SkillIdentityCatalog::new(data()).unwrap();
    let mut changed_pin = pin();
    changed_pin.files[0].sha256 = "b".repeat(64);
    assert!(matches!(
        compile_fresh_owned_skill_catalog(
            &catalog,
            &base(),
            &changed_pin,
            &policy(AbsentSupportPolicy::Pending),
            limits()
        ),
        Err(SkillCatalogError::SourceMismatch)
    ));
    let mut changed_pin = pin();
    changed_pin.revision = "d".repeat(40);
    assert!(matches!(
        compile_fresh_owned_skill_catalog(
            &catalog,
            &base(),
            &changed_pin,
            &policy(AbsentSupportPolicy::Pending),
            limits()
        ),
        Err(SkillCatalogError::SourceMismatch)
    ));
    let compiled = compile(data(), AbsentSupportPolicy::NonSupport);
    let (schema, mappings, input) = assemble(&compiled);
    let mut foreign = input.clone();
    foreign.namespace = GameVersionNamespace::new("foreign", "v1").unwrap();
    assert!(matches!(
        OwnedSkillRoleIndex::new(foreign, &mappings, &schema, limits()),
        Err(SkillCatalogError::ForeignNamespace)
    ));
    let mut changed = input.clone();
    changed.mapping = "0".repeat(64).parse().unwrap();
    assert!(matches!(
        OwnedSkillRoleIndex::new(changed, &mappings, &schema, limits()),
        Err(SkillCatalogError::BindingMismatch)
    ));
    let mut changed = input.clone();
    changed.definitions.content_sha256 = "b".repeat(64);
    assert!(matches!(
        OwnedSkillRoleIndex::new(changed, &mappings, &schema, limits()),
        Err(SkillCatalogError::BindingMismatch)
    ));
    let mut changed = input.clone();
    changed.compilation.policy.version = key("other-policy");
    assert!(matches!(
        OwnedSkillRoleIndex::new(changed, &mappings, &schema, limits()),
        Err(SkillCatalogError::BindingMismatch)
    ));
    let mut changed = input.clone();
    changed.roles[1].gem = changed.roles[0].gem.clone();
    assert!(matches!(
        OwnedSkillRoleIndex::new(changed, &mappings, &schema, limits()),
        Err(SkillCatalogError::DuplicateRole)
    ));
    let mut changed = input;
    changed.roles[0].primary =
        OwnedPrimarySkill::Known(DefId::parse(ns(), "unregistered-skill").unwrap());
    assert!(matches!(
        OwnedSkillRoleIndex::new(changed, &mappings, &schema, limits()),
        Err(SkillCatalogError::UnknownTarget)
    ));
}

fn construction_data() -> SkillIdentityData {
    let mut source = data();
    add_skill(&mut source, "late-effect", Some(false));
    let declared = vec![IndexedIdentityReference {
        index: 2,
        id: "missing-command".into(),
    }];
    let stat_sets = vec![IndexedIdentityReference {
        index: 4,
        id: "additional-stat-set".into(),
    }];
    let order = Some(vec![3, 1, 2]);
    let declaration = &mut source.gem_declarations[1].identity;
    declaration.additional_effects = declared.clone();
    declaration.additional_stat_sets = stat_sets.clone();
    declaration.display_order = order.clone();
    let gem = &mut source.gems[1];
    gem.declared_additional_effects = declared.clone();
    gem.declared_additional_stat_sets = stat_sets;
    gem.constructed_additional_effects = declared;
    gem.constructed_additional_effects.extend([
        IndexedIdentityReference {
            index: 5,
            id: "late-effect".into(),
        },
        IndexedIdentityReference {
            index: 8,
            id: "late-effect".into(),
        },
    ]);
    gem.additional_effects = vec!["late-effect".into(), "late-effect".into()];
    gem.effect_list = vec![
        "late-effect".into(),
        "active-effect".into(),
        "late-effect".into(),
    ];
    gem.display_order = order;
    source.missing_references = source.expected_missing_references();
    source
}

fn observed(row: &OwnedGemRoleRow) -> &ObservedGemSourceConstruction {
    let OwnedGemSourceConstruction::Observed(facts) = &row.construction else {
        panic!("compiled source observation")
    };
    facts
}

fn active_observed(input: &mut OwnedSkillRolePackageInput) -> &mut ObservedGemSourceConstruction {
    input
        .roles
        .iter_mut()
        .find_map(|row| match &mut row.construction {
            OwnedGemSourceConstruction::Observed(facts)
                if facts.catalog_gem_key == "active-gem-key" =>
            {
                Some(facts.as_mut())
            }
            _ => None,
        })
        .unwrap()
}

#[test]
fn construction_preserves_sparse_missing_declared_and_late_repeated_display_positions() {
    let source = construction_data();
    let catalog = SkillIdentityCatalog::new(source.clone()).unwrap();
    let compiled = compile(source.clone(), AbsentSupportPolicy::NonSupport);
    let (schema, mapping, input) = assemble(&compiled);
    let roles = OwnedSkillRoleIndex::new(input, &mapping, &schema, limits()).unwrap();
    verify_owned_skill_catalog_reproduction(&catalog, &roles, &mapping, limits()).unwrap();
    let active = gem_id(&compiled, "external-active", "v-b");
    let row = roles.role(&active).unwrap();
    let facts = observed(row);
    assert_eq!(facts.winning_declaration, 2);
    assert_eq!(facts.catalog_gem_key, source.gems[1].key);
    assert_eq!(facts.display_order, Some(vec![3, 1, 2]));
    assert_eq!(
        facts
            .resolved_effects
            .iter()
            .map(|effect| effect.effect_id.as_str())
            .collect::<Vec<_>>(),
        ["late-effect", "active-effect", "late-effect"]
    );
    assert_eq!(facts.resolved_effects[0], facts.resolved_effects[2]);
    assert_eq!(
        facts
            .resolved_additional_effects
            .iter()
            .map(|effect| effect.effect_id.as_str())
            .collect::<Vec<_>>(),
        ["late-effect", "late-effect"]
    );
    assert_eq!(
        facts
            .constructed_additional_effects
            .iter()
            .map(|effect| effect.index)
            .collect::<Vec<_>>(),
        [2, 5, 8]
    );
    assert_eq!(facts.declared_additional_effects.len(), 1);
    assert_eq!(
        facts.missing_references,
        [
            OwnedMissingSourceEffect {
                kind: OwnedMissingSourceEffectKind::DeclaredAdditionalEffect,
                index: Some(2),
                effect_id: "missing-command".into(),
            },
            OwnedMissingSourceEffect {
                kind: OwnedMissingSourceEffectKind::ConstructedAdditionalEffect,
                index: Some(2),
                effect_id: "missing-command".into(),
            },
        ]
    );
    assert_eq!(
        facts.additional_stat_sets,
        [OwnedSourceStatSetReference {
            index: 4,
            id: "additional-stat-set".into(),
        }]
    );
    // Catalog identities do not create mechanics or map stat sets as Skills.
    assert_eq!(compiled.receipt.skill_count, 3);
    for effect in &facts.resolved_effects {
        let OwnedSourceEffectResolution::Mapped(id) = &effect.resolution else {
            panic!("resolved source position")
        };
        assert!(matches!(schema.definition(id), SchemaLookup::Unmapped(_)));
    }
    assert_eq!(row.role, OwnedGemRole::Known(AuthoredGemRole::SkillUse));
    assert!(matches!(
        schema.definition(&active),
        SchemaLookup::Unmapped(_)
    ));
}

#[test]
fn observed_singleton_retains_missing_command_and_stat_sets_without_guessing_completeness() {
    let mut source = construction_data();
    let gem = &mut source.gems[1];
    gem.constructed_additional_effects = gem.declared_additional_effects.clone();
    gem.additional_effects.clear();
    gem.effect_list = vec![gem.primary_effect_id.clone()];
    gem.display_order = None;
    source.gem_declarations[1].identity.display_order = None;
    source.missing_references = source.expected_missing_references();
    let catalog = SkillIdentityCatalog::new(source.clone()).unwrap();
    let compiled = compile(source, AbsentSupportPolicy::NonSupport);
    let (schema, mapping, input) = assemble(&compiled);
    let roles = OwnedSkillRoleIndex::new(input, &mapping, &schema, limits()).unwrap();
    verify_owned_skill_catalog_reproduction(&catalog, &roles, &mapping, limits()).unwrap();
    let active = gem_id(&compiled, "external-active", "v-b");
    let facts = observed(roles.role(&active).unwrap());
    assert_eq!(facts.resolved_effects.len(), 1);
    assert_eq!(facts.missing_references.len(), 2);
    assert_eq!(facts.additional_stat_sets.len(), 1);
    assert_eq!(
        facts.declared_additional_effects[0].effect.resolution,
        OwnedSourceEffectResolution::Missing
    );
}

#[test]
fn construction_is_required_and_explicit_unavailable_never_passes_catalog_reproduction() {
    let source = construction_data();
    let catalog = SkillIdentityCatalog::new(source.clone()).unwrap();
    let compiled = compile(source, AbsentSupportPolicy::NonSupport);
    let (schema, mapping, mut input) = assemble(&compiled);
    let mut wire = serde_json::to_value(&input.roles[0]).unwrap();
    wire.as_object_mut().unwrap().remove("construction");
    assert!(serde_json::from_value::<OwnedGemRoleRow>(wire).is_err());
    let active = gem_id(&compiled, "external-active", "v-b");
    let facts = observed(input.roles.iter().find(|row| row.gem == active).unwrap());
    let mut wire = serde_json::to_value(facts).unwrap();
    wire.as_object_mut().unwrap().remove("display_order");
    assert!(serde_json::from_value::<ObservedGemSourceConstruction>(wire).is_err());
    let mut wire = serde_json::to_value(&facts.missing_references[0]).unwrap();
    wire.as_object_mut().unwrap().remove("index");
    assert!(serde_json::from_value::<OwnedMissingSourceEffect>(wire).is_err());
    for row in &mut input.roles {
        row.construction = OwnedGemSourceConstruction::Unavailable;
    }
    let roles = OwnedSkillRoleIndex::new(input, &mapping, &schema, limits()).unwrap();
    assert!(matches!(
        verify_owned_skill_catalog_reproduction(&catalog, &roles, &mapping, limits()),
        Err(SkillCatalogError::ReproductionMismatch)
    ));
    assert_eq!(
        roles.role(&active).unwrap().role,
        OwnedGemRole::Known(AuthoredGemRole::SkillUse)
    );
}

#[test]
fn runtime_construction_rejects_wrong_identity_joins_and_inconsistent_missing_census() {
    let compiled = compile(construction_data(), AbsentSupportPolicy::NonSupport);
    let (schema, mapping, input) = assemble(&compiled);
    for mutation in 0..12 {
        let mut changed = input.clone();
        let facts = active_observed(&mut changed);
        match mutation {
            0 => facts.game_id = "other-external-gem".into(),
            1 => facts.variant_id = "other-variant".into(),
            2 => facts.primary.effect_id = "support-effect".into(),
            3 => facts.resolved_effects[0].effect_id = "support-effect".into(),
            4 => {
                facts.resolved_effects[0].resolution = OwnedSourceEffectResolution::Mapped(
                    DefId::parse(
                        GameVersionNamespace::new("foreign", "v1").unwrap(),
                        "foreign-skill",
                    )
                    .unwrap(),
                )
            }
            5 => facts.missing_references[0].index = Some(7),
            6 => facts.missing_references[0].kind = OwnedMissingSourceEffectKind::PrimaryEffect,
            7 => {
                facts.constructed_additional_effects.remove(0);
            }
            8 => facts.constructed_additional_effects[1].index = 2,
            9 => facts.additional_stat_sets[0].index = 0,
            10 => facts.resolved_effects.push(OwnedSourceEffectReference {
                effect_id: "missing-command".into(),
                resolution: OwnedSourceEffectResolution::Missing,
            }),
            11 => facts.winning_declaration = 0,
            _ => unreachable!(),
        };
        assert!(
            OwnedSkillRoleIndex::new(changed, &mapping, &schema, limits()).is_err(),
            "accepted construction mutation {mutation}"
        );
    }
}

#[test]
fn offline_reproduction_rejects_structurally_valid_forgery_even_with_fresh_package_digest() {
    let source = construction_data();
    let catalog = SkillIdentityCatalog::new(source.clone()).unwrap();
    let compiled = compile(source, AbsentSupportPolicy::NonSupport);
    let (schema, mapping, input) = assemble(&compiled);
    let original = OwnedSkillRoleIndex::new(input.clone(), &mapping, &schema, limits()).unwrap();
    for mutation in 0..8 {
        let mut changed = input.clone();
        let facts = active_observed(&mut changed);
        match mutation {
            0 => facts.resolved_effects.swap(0, 1),
            1 => {
                facts.resolved_effects.pop();
            }
            2 => facts.display_order = None,
            3 => facts.winning_declaration = 1,
            4 => facts.additional_stat_sets.clear(),
            5 => facts.resolved_additional_effects.clear(),
            6 => {
                // Internally consistent omission still cannot certify a singleton.
                facts.declared_additional_effects.clear();
                facts.constructed_additional_effects.clear();
                facts.resolved_additional_effects.clear();
                facts.resolved_effects = vec![facts.primary.clone()];
                facts.missing_references.clear();
            }
            7 => facts.catalog_gem_key = "invented-catalog-row".into(),
            _ => unreachable!(),
        }
        let forged = OwnedSkillRoleIndex::new(changed, &mapping, &schema, limits()).unwrap();
        assert_ne!(forged.identity(), original.identity());
        assert!(matches!(
            verify_owned_skill_catalog_reproduction(&catalog, &forged, &mapping, limits()),
            Err(SkillCatalogError::ReproductionMismatch)
        ));
    }
}

#[test]
fn reproduction_retains_historical_registry_ancestry_but_rejects_wrong_catalog_commitment() {
    let source = construction_data();
    let catalog = SkillIdentityCatalog::new(source.clone()).unwrap();
    let mut compiled = compile(source, AbsentSupportPolicy::NonSupport);
    let receipt = compiled.receipt.clone();
    compiled
        .registry
        .allocate_definition::<ItemTemplateDefinition>()
        .unwrap();
    let (schema, mapping, input) = assemble(&compiled);
    let roles = OwnedSkillRoleIndex::new(input.clone(), &mapping, &schema, limits()).unwrap();
    assert_eq!(roles.input().compilation, receipt);
    assert_ne!(mapping.input().registry, receipt.staged_registry);
    verify_owned_skill_catalog_reproduction(&catalog, &roles, &mapping, limits()).unwrap();
    for mutation in 0..2 {
        let mut changed = input.clone();
        match mutation {
            0 => changed.compilation.catalog_digest = "0".repeat(64).parse().unwrap(),
            1 => changed.compilation.skill_count += 1,
            _ => unreachable!(),
        }
        let forged = OwnedSkillRoleIndex::new(changed, &mapping, &schema, limits()).unwrap();
        assert!(matches!(
            verify_owned_skill_catalog_reproduction(&catalog, &forged, &mapping, limits()),
            Err(SkillCatalogError::ReproductionMismatch)
        ));
    }
}

#[test]
fn construction_nested_collections_text_and_total_reproduction_work_are_bounded() {
    let mut source = construction_data();
    source.gems[1].effect_list = vec!["active-effect".into(); 32];
    let catalog = SkillIdentityCatalog::new(source.clone()).unwrap();
    let compiled = compile(source, AbsentSupportPolicy::NonSupport);
    let (schema, mapping, input) = assemble(&compiled);
    let roles = OwnedSkillRoleIndex::new(input.clone(), &mapping, &schema, limits()).unwrap();
    let mut tight = limits();
    tight.mapping.max_collection_entries = 31;
    assert!(matches!(
        OwnedSkillRoleIndex::new(input.clone(), &mapping, &schema, tight),
        Err(SkillCatalogError::LimitExceeded(
            "construction resolved references"
        ))
    ));
    assert!(verify_owned_skill_catalog_reproduction(&catalog, &roles, &mapping, tight).is_err());
    let original = serde_json::to_vec(roles.input()).unwrap();
    let before = base();
    let base_input = before.input().clone();
    assert!(
        compile_fresh_owned_skill_catalog(
            &catalog,
            &before,
            &pin(),
            &policy(AbsentSupportPolicy::NonSupport),
            tight,
        )
        .is_err()
    );
    assert_eq!(before.input(), &base_input);
    let mut long = input.clone();
    active_observed(&mut long).additional_stat_sets[0].id = "s".repeat(128);
    tight = limits();
    tight.mapping.max_string_bytes = 127;
    assert!(matches!(
        OwnedSkillRoleIndex::new(long, &mapping, &schema, tight),
        Err(SkillCatalogError::LimitExceeded("source text"))
    ));
    // Locate the finite aggregate boundary, then prove exact N vs N-1 behavior.
    // The larger nested observation must not be hidden by the small row count.
    let minimum = (1..1000)
        .find(|max_entries| {
            let mut bounded = limits();
            bounded.mapping.max_entries = *max_entries;
            verify_owned_skill_catalog_reproduction(&catalog, &roles, &mapping, bounded).is_ok()
        })
        .expect("bounded fixture reproduction");
    assert!(minimum > 32);
    tight = limits();
    tight.mapping.max_entries = minimum - 1;
    assert!(verify_owned_skill_catalog_reproduction(&catalog, &roles, &mapping, tight).is_err());
    assert_eq!(serde_json::to_vec(roles.input()).unwrap(), original);
}

#[test]
fn reproduction_bounds_shadowed_declaration_history_independently_of_final_rows() {
    for gem_history in [true, false] {
        let mut source = data();
        for _ in 0..38 {
            if gem_history {
                let mut declaration = source.gem_declarations[1].clone();
                declaration.index = source.gem_declarations.len() as u32 + 1;
                source.gems[1].winning_declaration = declaration.index;
                source.gem_declarations.push(declaration);
            } else {
                let mut declaration = source.skill_declarations[1].clone();
                declaration.index = source.skill_declarations.len() as u32 + 1;
                source.skills[1].winning_declaration = declaration.index;
                source.skill_declarations.push(declaration);
            }
        }
        assert_eq!(source.gems.len(), 2);
        assert_eq!(source.skills.len(), 2);
        let catalog = SkillIdentityCatalog::new(source.clone()).unwrap();
        let compiled = compile(source, AbsentSupportPolicy::NonSupport);
        let (schema, mapping, input) = assemble(&compiled);
        let roles = OwnedSkillRoleIndex::new(input, &mapping, &schema, limits()).unwrap();
        verify_owned_skill_catalog_reproduction(&catalog, &roles, &mapping, limits()).unwrap();
        let before = serde_json::to_vec(roles.input()).unwrap();
        let mut exact = limits();
        exact.mapping.max_collection_entries = 40;
        verify_owned_skill_catalog_reproduction(&catalog, &roles, &mapping, exact).unwrap();
        let mut tight = exact;
        tight.mapping.max_collection_entries = 39;
        let expected = if gem_history {
            "gem declarations"
        } else {
            "skill declarations"
        };
        assert!(matches!(
            verify_owned_skill_catalog_reproduction(&catalog, &roles, &mapping, tight),
            Err(SkillCatalogError::LimitExceeded(name)) if name == expected
        ));
        assert!(matches!(
            compile_fresh_owned_skill_catalog(
                &catalog,
                &base(),
                &pin(),
                &policy(AbsentSupportPolicy::NonSupport),
                tight,
            ),
            Err(SkillCatalogError::LimitExceeded(name)) if name == expected
        ));
        assert_eq!(serde_json::to_vec(roles.input()).unwrap(), before);
    }
}

#[test]
fn resource_failures_leave_the_base_unchanged_including_after_staged_allocations() {
    let base = base();
    let before = base.input().clone();
    let mut source = data();
    add_gem(
        &mut source,
        "collision",
        "external-support",
        "v-a",
        "active-effect",
    );
    let source = SkillIdentityCatalog::new(source).unwrap();
    let mut tight = limits();
    tight.mapping.max_candidates = 1;
    assert!(matches!(
        compile_fresh_owned_skill_catalog(
            &source,
            &base,
            &pin(),
            &policy(AbsentSupportPolicy::NonSupport),
            tight
        ),
        Err(SkillCatalogError::LimitExceeded("selector candidates"))
    ));
    assert_eq!(base.input(), &before);
    let source = SkillIdentityCatalog::new(data()).unwrap();
    let mut exact = limits();
    // Two additional ordered construction positions are now retained explicitly.
    exact.mapping.max_entries = 24;
    assert!(
        compile_fresh_owned_skill_catalog(
            &source,
            &base,
            &pin(),
            &policy(AbsentSupportPolicy::NonSupport),
            exact
        )
        .is_ok()
    );
    // The immutable base was constructed with looser bounds. Its 21 entries
    // fit the requested limit, but the four newly staged IDs do not.
    let mut populated = base.clone();
    for _ in 0..21 {
        populated.allocate_definition::<SkillDefinition>().unwrap();
    }
    let populated_before = populated.input().clone();
    assert!(
        compile_fresh_owned_skill_catalog(
            &source,
            &populated,
            &pin(),
            &policy(AbsentSupportPolicy::NonSupport),
            exact
        )
        .is_err()
    );
    assert_eq!(populated.input(), &populated_before);
    exact.mapping.max_entries = 23;
    assert!(matches!(
        compile_fresh_owned_skill_catalog(
            &source,
            &base,
            &pin(),
            &policy(AbsentSupportPolicy::NonSupport),
            exact
        ),
        Err(SkillCatalogError::LimitExceeded(_))
    ));
    let mut tight = limits();
    tight.mapping.max_wire_bytes = 512;
    assert!(
        compile_fresh_owned_skill_catalog(
            &source,
            &base,
            &pin(),
            &policy(AbsentSupportPolicy::NonSupport),
            tight
        )
        .is_err()
    );
    assert_eq!(base.input(), &before);
}

#[test]
fn bound_role_index_rejects_roles_outside_known_schema_permissions() {
    let compiled = compile(data(), AbsentSupportPolicy::NonSupport);
    let support = gem_id(&compiled, "external-support", "v-a");
    let row = compiled
        .roles
        .iter()
        .find(|row| row.gem == support)
        .unwrap();
    let OwnedPrimarySkill::Known(primary) = &row.primary else {
        panic!("known primary")
    };
    let mut definitions = compiled.definitions.clone();
    for definition in &mut definitions {
        if let DefinitionDescriptor::Gem(entry) = definition
            && entry.id == support
        {
            entry.schema = SchemaState::Known(GemSchema {
                level: IntegerRange {
                    minimum: BoundedInteger::new(0).unwrap(),
                    maximum: BoundedInteger::new(20).unwrap(),
                },
                roles: vec![AuthoredGemRole::SkillUse],
                skills: DeclaredSet::complete(vec![primary.clone()]),
                quality: QualityUseSchema {
                    presence: QualityPresence::Forbidden,
                    allowed_kinds: DeclaredSet::complete(vec![]),
                },
                declarations: DeclaredSlots {
                    parameters: DeclaredSet::complete(vec![]),
                    choices: DeclaredSet::complete(vec![]),
                    grants: DeclaredSet::complete(vec![]),
                    actors: DeclaredSet::complete(vec![]),
                    skill_grants: DeclaredSet::complete(vec![]),
                    outputs: DeclaredSet::complete(vec![]),
                    sockets: DeclaredSet::complete(vec![]),
                },
            });
        }
    }
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: ns(),
            release: key("release"),
            semantics_version: key("schema-v1"),
            definitions,
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let mappings = OwnedMappingIndex::new(
        MappingPackageInput {
            schema_version: OWNED_MAPPING_PACKAGE_VERSION,
            namespace: ns(),
            registry: compiled.registry.identity().unwrap(),
            definitions: schema.identity().clone(),
            source: pin(),
            policy_version: compiled.receipt.policy.version.clone(),
            entries: compiled.mappings.clone(),
        },
        &compiled.registry,
        &schema,
        limits().mapping,
    )
    .unwrap();
    assert!(matches!(
        OwnedSkillRoleIndex::new(
            role_input(&compiled, &schema, &mappings),
            &mappings,
            &schema,
            limits()
        ),
        Err(SkillCatalogError::SchemaConflict)
    ));
}

#[test]
fn catalogue_support_primary_is_not_required_as_an_executable_skill_supply() {
    for (game, variant, role) in [
        (
            "external-support",
            "v-a",
            AuthoredGemRole::SupportAssignment,
        ),
        ("external-active", "v-b", AuthoredGemRole::SkillUse),
    ] {
        for include_primary in [false, true] {
            let mut compiled = compile(data(), AbsentSupportPolicy::NonSupport);
            let gem = gem_id(&compiled, game, variant);
            let original = compiled
                .roles
                .iter()
                .find(|row| row.gem == gem)
                .unwrap()
                .clone();
            let OwnedPrimarySkill::Known(primary) = &original.primary else {
                panic!("fixture primary")
            };
            let DefinitionDescriptor::Gem(entry) = compiled
                .definitions
                .iter_mut()
                .find(|d| d.address() == gem.address())
                .unwrap()
            else {
                panic!("fixture Gem")
            };
            entry.schema = SchemaState::Known(GemSchema {
                level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(20).unwrap(),
                },
                roles: vec![role],
                skills: DeclaredSet::complete(if include_primary {
                    vec![primary.clone()]
                } else {
                    vec![]
                }),
                quality: QualityUseSchema {
                    presence: QualityPresence::Forbidden,
                    allowed_kinds: DeclaredSet::complete(vec![]),
                },
                declarations: DeclaredSlots {
                    parameters: DeclaredSet::complete(vec![]),
                    choices: DeclaredSet::complete(vec![]),
                    grants: DeclaredSet::complete(vec![]),
                    actors: DeclaredSet::complete(vec![]),
                    skill_grants: DeclaredSet::complete(vec![]),
                    outputs: DeclaredSet::complete(vec![]),
                    sockets: DeclaredSet::complete(vec![]),
                },
            });
            let (schema, mapping, input) = assemble(&compiled);
            if role == AuthoredGemRole::SupportAssignment && !include_primary {
                let mut uncertain = input.clone();
                uncertain
                    .roles
                    .iter_mut()
                    .find(|row| row.gem == gem)
                    .unwrap()
                    .role = OwnedGemRole::Unmapped {
                    issue: key("fixture-role-unresolved"),
                };
                assert!(
                    matches!(
                        OwnedSkillRoleIndex::new(uncertain, &mapping, &schema, limits()),
                        Err(SkillCatalogError::SchemaConflict)
                    ),
                    "only an explicitly classified support primary can be excluded"
                );
            }
            let result = OwnedSkillRoleIndex::new(input, &mapping, &schema, limits());
            if role == AuthoredGemRole::SkillUse && !include_primary {
                assert!(matches!(result, Err(SkillCatalogError::SchemaConflict)));
            } else {
                // A historical support descriptor retaining this association is
                // readable too; the producer correction does not erase identity.
                let index = result.unwrap();
                assert_eq!(index.role(&gem), Some(&original));
                assert!(matches!(
                    schema.definition(primary),
                    SchemaLookup::Unmapped(_)
                ));
            }
        }
    }
}

#[test]
fn shared_source_pin_may_include_other_domains_but_binds_every_catalog_file() {
    let catalog = SkillIdentityCatalog::new(data()).unwrap();
    let mut full_pin = pin();
    full_pin.files.push(SourceFilePin {
        path: "fixture/items.json".into(),
        sha256: "b".repeat(64),
    });
    let compiled = compile_fresh_owned_skill_catalog(
        &catalog,
        &base(),
        &full_pin,
        &policy(AbsentSupportPolicy::NonSupport),
        limits(),
    )
    .unwrap();
    assert_eq!(compiled.receipt.source, full_pin);
    full_pin.files.reverse();
    let permuted = compile_fresh_owned_skill_catalog(
        &catalog,
        &base(),
        &full_pin,
        &policy(AbsentSupportPolicy::NonSupport),
        limits(),
    )
    .unwrap();
    assert_eq!(compiled.receipt, permuted.receipt);
    full_pin
        .files
        .retain(|file| file.path != "fixture/identities.json");
    assert!(matches!(
        compile_fresh_owned_skill_catalog(
            &catalog,
            &base(),
            &full_pin,
            &policy(AbsentSupportPolicy::NonSupport),
            limits(),
        ),
        Err(SkillCatalogError::SourceMismatch)
    ));
}

#[test]
fn empty_owned_artifacts_supply_no_guessed_identity_or_role() {
    // There is no fabricated empty source catalog: this is a directly authored
    // role artifact bound to valid empty owned schema/mapping packages.
    let mut staged = compile(data(), AbsentSupportPolicy::Pending);
    staged.registry = base();
    staged.definitions.clear();
    staged.mappings.clear();
    staged.roles.clear();
    staged.receipt.gem_count = 0;
    staged.receipt.skill_count = 0;
    staged.receipt.base_registry = staged.registry.identity().unwrap();
    staged.receipt.staged_registry = staged.registry.identity().unwrap();
    let (schema, mappings, input) = assemble(&staged);
    let index = OwnedSkillRoleIndex::new(input, &mappings, &schema, limits()).unwrap();
    assert!(index.input().roles.is_empty());
    assert!(index.lookup(&selector("external-support", "v-a")).is_none());
}

#[test]
fn final_mapping_can_bind_a_valid_registry_successor_after_other_domain_allocation() {
    let compiled = compile(data(), AbsentSupportPolicy::NonSupport);
    let mut extended = compiled.clone();
    extended
        .registry
        .allocate_definition::<ItemTemplateDefinition>()
        .unwrap();
    compiled
        .registry
        .validate_successor(&extended.registry)
        .unwrap();
    assert_ne!(
        extended.receipt.staged_registry,
        extended.registry.identity().unwrap()
    );
    let (schema, mappings, input) = assemble(&extended);
    let index = OwnedSkillRoleIndex::new(input, &mappings, &schema, limits()).unwrap();
    assert_eq!(index.input().compilation, compiled.receipt);
    assert_eq!(index.input().mapping, *mappings.identity());
    assert_ne!(
        mappings.input().registry,
        compiled.registry.identity().unwrap()
    );
    assert!(
        index
            .role(&gem_id(&compiled, "external-support", "v-a"))
            .is_some()
    );
}

fn materialization_catalog(from_tree: Option<bool>) -> SkillIdentityCatalog {
    let mut source = data();
    for skill in &mut source.skills {
        skill.from_tree = from_tree;
    }
    for declaration in &mut source.skill_declarations {
        declaration.identity.from_tree = from_tree;
    }
    SkillIdentityCatalog::new(source).unwrap()
}

fn compile_materialization(
    from_tree: Option<bool>,
    absent_from_tree: AbsentFromTreePolicy,
) -> FreshOwnedSkillCatalog {
    let mut policy = policy(AbsentSupportPolicy::NonSupport);
    policy.absent_from_tree = absent_from_tree;
    compile_fresh_owned_skill_catalog(
        &materialization_catalog(from_tree),
        &base(),
        &pin(),
        &policy,
        limits(),
    )
    .unwrap()
}

#[test]
fn explicit_from_tree_controls_materialization_independently_of_support_role_or_absence_policy() {
    for absent in [
        AbsentFromTreePolicy::Pending,
        AbsentFromTreePolicy::Physical,
    ] {
        for (from_tree, expected) in [
            (true, OwnedGemMaterialization::ProviderOnly),
            (false, OwnedGemMaterialization::Physical),
        ] {
            let compiled = compile_materialization(Some(from_tree), absent);
            let support = gem_id(&compiled, "external-support", "v-a");
            let active = gem_id(&compiled, "external-active", "v-b");
            let (schema, mapping, input) = assemble(&compiled);
            let index = OwnedSkillRoleIndex::new(input, &mapping, &schema, limits()).unwrap();
            assert_eq!(index.role(&support).unwrap().materialization, expected);
            assert_eq!(index.role(&active).unwrap().materialization, expected);
            assert_eq!(
                index.role(&support).unwrap().role,
                OwnedGemRole::Known(AuthoredGemRole::SupportAssignment)
            );
            assert_eq!(
                index.role(&active).unwrap().role,
                OwnedGemRole::Known(AuthoredGemRole::SkillUse)
            );
            assert!(matches!(
                schema.definition(&support),
                SchemaLookup::Unmapped(_)
            ));
        }
    }
}

#[test]
fn absent_from_tree_requires_explicit_policy_and_changed_evidence_changes_immutable_role_binding() {
    let pending = compile_materialization(None, AbsentFromTreePolicy::Pending);
    let physical = compile_materialization(None, AbsentFromTreePolicy::Physical);
    let provided = compile_materialization(Some(true), AbsentFromTreePolicy::Physical);
    assert_eq!(pending.definitions, physical.definitions);
    assert_eq!(pending.mappings, physical.mappings);
    assert_eq!(pending.registry.input(), physical.registry.input());
    assert_eq!(
        pending.receipt.catalog_digest,
        physical.receipt.catalog_digest
    );
    assert_ne!(
        physical.receipt.catalog_digest,
        provided.receipt.catalog_digest
    );
    assert_eq!(physical.definitions, provided.definitions);
    assert_eq!(physical.mappings, provided.mappings);
    let (schema, mapping, pending_input) = assemble(&pending);
    let pending_index =
        OwnedSkillRoleIndex::new(pending_input.clone(), &mapping, &schema, limits()).unwrap();
    let before = *pending_index.identity();
    let physical_index = OwnedSkillRoleIndex::new(
        role_input(&physical, &schema, &mapping),
        &mapping,
        &schema,
        limits(),
    )
    .unwrap();
    let provider_index = OwnedSkillRoleIndex::new(
        role_input(&provided, &schema, &mapping),
        &mapping,
        &schema,
        limits(),
    )
    .unwrap();
    let active = gem_id(&pending, "external-active", "v-b");
    assert!(matches!(
        pending_index.role(&active).unwrap().materialization,
        OwnedGemMaterialization::Unmapped { .. }
    ));
    assert_eq!(
        physical_index.role(&active).unwrap().materialization,
        OwnedGemMaterialization::Physical
    );
    assert_eq!(
        provider_index.role(&active).unwrap().materialization,
        OwnedGemMaterialization::ProviderOnly
    );
    assert_ne!(before, *physical_index.identity());
    assert_ne!(physical_index.identity(), provider_index.identity());
    assert_eq!(pending_index.identity(), &before);
    assert_eq!(pending_index.input(), &pending_input);
    // Raw wire DTOs cannot silently retain the former physical default.
    let mut wire = serde_json::to_value(&physical.receipt.policy).unwrap();
    wire.as_object_mut().unwrap().remove("absent_from_tree");
    assert!(serde_json::from_value::<SkillCatalogPolicy>(wire).is_err());
    let mut wire = serde_json::to_value(&physical.roles[0]).unwrap();
    wire.as_object_mut().unwrap().remove("materialization");
    assert!(serde_json::from_value::<OwnedGemRoleRow>(wire).is_err());
}

#[test]
fn provider_only_placeholder_cannot_be_bound_as_a_known_physical_gem_schema() {
    let compiled = compile_materialization(Some(true), AbsentFromTreePolicy::Physical);
    let support = gem_id(&compiled, "external-support", "v-a");
    let role = compiled
        .roles
        .iter()
        .find(|row| row.gem == support)
        .unwrap();
    let OwnedPrimarySkill::Known(primary) = &role.primary else {
        panic!("known primary")
    };
    let mut definitions = compiled.definitions.clone();
    for definition in &mut definitions {
        if let DefinitionDescriptor::Gem(entry) = definition
            && entry.id == support
        {
            entry.schema = SchemaState::Known(GemSchema {
                level: IntegerRange {
                    minimum: BoundedInteger::new(0).unwrap(),
                    maximum: BoundedInteger::new(20).unwrap(),
                },
                roles: vec![AuthoredGemRole::SupportAssignment],
                skills: DeclaredSet::complete(vec![primary.clone()]),
                quality: QualityUseSchema {
                    presence: QualityPresence::Forbidden,
                    allowed_kinds: DeclaredSet::complete(vec![]),
                },
                declarations: DeclaredSlots {
                    parameters: DeclaredSet::complete(vec![]),
                    choices: DeclaredSet::complete(vec![]),
                    grants: DeclaredSet::complete(vec![]),
                    actors: DeclaredSet::complete(vec![]),
                    skill_grants: DeclaredSet::complete(vec![]),
                    outputs: DeclaredSet::complete(vec![]),
                    sockets: DeclaredSet::complete(vec![]),
                },
            });
        }
    }
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: ns(),
            release: key("release"),
            semantics_version: key("known-gem-schema"),
            definitions,
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let mappings = OwnedMappingIndex::new(
        MappingPackageInput {
            schema_version: OWNED_MAPPING_PACKAGE_VERSION,
            namespace: ns(),
            registry: compiled.registry.identity().unwrap(),
            definitions: schema.identity().clone(),
            source: pin(),
            policy_version: compiled.receipt.policy.version.clone(),
            entries: compiled.mappings.clone(),
        },
        &compiled.registry,
        &schema,
        limits().mapping,
    )
    .unwrap();
    assert!(matches!(
        OwnedSkillRoleIndex::new(
            role_input(&compiled, &schema, &mappings),
            &mappings,
            &schema,
            limits()
        ),
        Err(SkillCatalogError::SchemaConflict)
    ));
    let physical = compile_materialization(Some(false), AbsentFromTreePolicy::Physical);
    assert!(
        OwnedSkillRoleIndex::new(
            role_input(&physical, &schema, &mappings),
            &mappings,
            &schema,
            limits()
        )
        .is_ok()
    );
}

#[test]
fn unknown_primary_cannot_claim_known_materialization() {
    let mut source = data();
    add_gem(
        &mut source,
        "missing-primary",
        "external-missing",
        "v-c",
        "absent-effect",
    );
    let compiled = compile(source, AbsentSupportPolicy::NonSupport);
    let missing = gem_id(&compiled, "external-missing", "v-c");
    let (schema, mapping, input) = assemble(&compiled);
    let index = OwnedSkillRoleIndex::new(input.clone(), &mapping, &schema, limits()).unwrap();
    assert!(matches!(
        index.role(&missing).unwrap().materialization,
        OwnedGemMaterialization::Unmapped { .. }
    ));
    for claim in [
        OwnedGemMaterialization::Physical,
        OwnedGemMaterialization::ProviderOnly,
    ] {
        let mut changed = input.clone();
        changed
            .roles
            .iter_mut()
            .find(|row| row.gem == missing)
            .unwrap()
            .materialization = claim;
        assert!(matches!(
            OwnedSkillRoleIndex::new(changed, &mapping, &schema, limits()),
            Err(SkillCatalogError::SchemaConflict)
        ));
    }
    assert_eq!(index.input(), &input);
}

#[test]
fn historical_role_receipt_retains_exact_subset_of_expanded_catalog_pins() {
    let compiled = compile(data(), AbsentSupportPolicy::NonSupport);
    let (schema, mapping, roles) = assemble(&compiled);
    let mut expanded = mapping.input().clone();
    expanded.source.files.push(SourceFilePin {
        path: "fixture/passives.json".into(),
        sha256: "b".repeat(64),
    });
    let next =
        OwnedMappingIndex::new(expanded, &compiled.registry, &schema, limits().mapping).unwrap();
    let mut rebound = roles.clone();
    rebound.mapping = *next.identity();
    let index = OwnedSkillRoleIndex::new(rebound, &next, &schema, limits()).unwrap();
    assert_eq!(index.input().compilation, roles.compilation);
    assert_eq!(index.input().roles, roles.roles);
    assert_ne!(index.input().mapping, roles.mapping);
    // An expanded source footprint does not relax the exact final mapping binding.
    assert!(OwnedSkillRoleIndex::new(roles, &next, &schema, limits()).is_err());
}
#[test]
fn historical_role_pin_subset_rejects_missing_changed_and_foreign_contexts() {
    let compiled = compile(data(), AbsentSupportPolicy::NonSupport);
    let (schema, mapping, roles) = assemble(&compiled);
    let mut changed = mapping.input().clone();
    changed.source.files[0].sha256 = "b".repeat(64);
    let mut missing = mapping.input().clone();
    missing.source.files = vec![SourceFilePin {
        path: "fixture/unrelated.json".into(),
        sha256: "b".repeat(64),
    }];
    let mut revision = mapping.input().clone();
    revision.source.revision = "d".repeat(40);
    let mut system = mapping.input().clone();
    system.source.system = ExternalSourceSystem::PathOfBuilding1;
    for input in [changed, missing, revision, system] {
        let mapping =
            OwnedMappingIndex::new(input, &compiled.registry, &schema, limits().mapping).unwrap();
        let mut roles = roles.clone();
        roles.mapping = *mapping.identity();
        assert!(matches!(
            OwnedSkillRoleIndex::new(roles, &mapping, &schema, limits()),
            Err(SkillCatalogError::BindingMismatch)
        ));
    }
}

/// Structural role validation is not full-catalog reproduction. In particular,
/// a surviving row cannot infer reverse-route uniqueness from its neighbors.
#[test]
fn structurally_valid_role_subset_cannot_prove_reverse_primary_uniqueness() {
    let mut source = data();
    add_skill(&mut source, "other-effect", Some(false));
    add_gem(
        &mut source,
        "colliding-gem-key",
        "external-colliding",
        "v-c",
        "active-effect",
    );
    let additional = IndexedIdentityReference {
        index: 1,
        id: "other-effect".into(),
    };
    source
        .gem_declarations
        .last_mut()
        .unwrap()
        .identity
        .additional_effects = vec![additional.clone()];
    let alias = source.gems.last_mut().unwrap();
    alias.declared_additional_effects = vec![additional.clone()];
    alias.constructed_additional_effects = vec![additional];
    alias.additional_effects = vec!["other-effect".into()];
    alias.effect_list = vec!["active-effect".into(), "other-effect".into()];
    source.missing_references = source.expected_missing_references();

    let catalog = SkillIdentityCatalog::new(source.clone()).unwrap();
    let compiled = compile(source, AbsentSupportPolicy::NonSupport);
    let (schema, mapping, input) = assemble(&compiled);
    let full = OwnedSkillRoleIndex::new(input.clone(), &mapping, &schema, limits()).unwrap();
    verify_owned_skill_catalog_reproduction(&catalog, &full, &mapping, limits()).unwrap();

    let candidates = |index: &OwnedSkillRoleIndex| {
        index
            .input()
            .roles
            .iter()
            .filter(|row| observed(row).primary.effect_id == "active-effect")
            .count()
    };
    assert_eq!(candidates(&full), 2);
    let target = gem_id(&compiled, "external-active", "v-b");
    let alias = gem_id(&compiled, "external-colliding", "v-c");
    let target_row = full.role(&target).unwrap().clone();
    assert_eq!(observed(&target_row).resolved_effects.len(), 1);
    assert_eq!(
        observed(full.role(&alias).unwrap()).resolved_effects.len(),
        2
    );

    let mut subset = input.clone();
    subset.roles.retain(|row| row.gem != alias);
    subset.compilation.gem_count = subset.roles.len();
    // The source pin, catalog commitment, schema, mapping and surviving row
    // stay unchanged; structural loading has no independent catalog census.
    assert_eq!(
        subset.compilation.catalog_digest,
        input.compilation.catalog_digest
    );
    assert_eq!(subset.compilation.source, input.compilation.source);
    assert_eq!(subset.mapping, input.mapping);
    assert_eq!(subset.definitions, input.definitions);
    let subset = OwnedSkillRoleIndex::new(subset, &mapping, &schema, limits()).unwrap();
    assert_eq!(subset.role(&target), Some(&target_row));
    assert_eq!(candidates(&subset), 1);
    assert_ne!(subset.identity(), full.identity());
    assert!(matches!(
        verify_owned_skill_catalog_reproduction(&catalog, &subset, &mapping, limits()),
        Err(SkillCatalogError::ReproductionMismatch)
    ));
}
