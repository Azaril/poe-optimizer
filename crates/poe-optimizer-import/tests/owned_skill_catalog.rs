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
    exact.mapping.max_entries = 22;
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
    // The immutable base was constructed with looser bounds. Its 19 entries
    // fit the requested limit, but the four newly staged IDs do not.
    let mut populated = base.clone();
    for _ in 0..19 {
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
    exact.mapping.max_entries = 21;
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
