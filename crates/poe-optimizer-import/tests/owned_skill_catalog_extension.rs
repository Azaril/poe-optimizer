//! Same-pin identity extension from injected data; no source VM or game defaults.
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

struct Seed {
    registry: OwnedIdRegistry,
    schema: OwnedDefinitionSchemaPackage,
    mappings: OwnedMappingIndex,
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn ports() -> DeclaredSlots {
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
fn range() -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(0).unwrap(),
        maximum: BoundedInteger::new(100).unwrap(),
    }
}
fn source_gem(game: &str, variant: &str) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::Gem {
        game_id: SourceComponent::Text(game.into()),
        variant_id: SourceComponent::Text(variant.into()),
    })
}
fn source_skill(name: &str) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::Skill {
        effect_id: SourceComponent::Text(name.into()),
    })
}
fn mapped(mappings: &[MappingEntry], source: &ExternalSelector) -> SchemaSubject {
    match &mappings
        .iter()
        .find(|m| &m.source == source)
        .unwrap()
        .outcome
    {
        MappingOutcome::Mapped { target, .. } => target.clone(),
        _ => panic!("expected mapped test seed"),
    }
}
fn seed() -> Seed {
    let fresh = compile_fresh_owned_skill_catalog(
        &SkillIdentityCatalog::new(data()).unwrap(),
        &OwnedIdRegistry::empty(ns(), limits().mapping).unwrap(),
        &pin(),
        &policy(AbsentSupportPolicy::NonSupport),
        limits(),
    )
    .unwrap();
    let mut registry = fresh.registry;
    let mut definitions = fresh.definitions;
    let mut entries = fresh.mappings;
    let SchemaSubject::Definition(DefinitionAddress::Gem(gem)) =
        mapped(&entries, &source_gem("external-active", "v-b"))
    else {
        unreachable!()
    };
    let SchemaSubject::Definition(DefinitionAddress::Skill(skill)) =
        mapped(&entries, &source_skill("active-effect"))
    else {
        unreachable!()
    };
    let parameter = registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Gem(gem.clone()))
        .unwrap();
    for d in &mut definitions {
        match d {
            DefinitionDescriptor::Gem(e) if e.id == gem => {
                let mut declarations = ports();
                declarations.parameters.members.push(parameter.clone());
                e.schema = SchemaState::Known(GemSchema {
                    level: range(),
                    roles: vec![AuthoredGemRole::SkillUse],
                    skills: DeclaredSet::complete(vec![skill.clone()]),
                    quality: QualityUseSchema {
                        presence: QualityPresence::Forbidden,
                        allowed_kinds: empty(),
                    },
                    declarations,
                });
            }
            DefinitionDescriptor::Skill(e) if e.id == skill => {
                let mut declarations = ports();
                declarations.outputs.closure = SchemaClosure::Partial {
                    gaps: vec![SchemaGap {
                        subject: SchemaSubject::Definition(skill.address()),
                        facet: SchemaFacet::StaticLinks,
                        code: key("other-outputs"),
                    }],
                };
                e.schema = SchemaState::Known(SkillSchema {
                    directly_selectable: false,
                    declarations,
                });
            }
            _ => {}
        }
    }
    let reward: RewardDefId = registry.allocate_definition().unwrap();
    definitions.push(DefinitionDescriptor::Reward(DefinitionEntry {
        id: reward.clone(),
        schema: SchemaState::Known(RewardSchema {
            declarations: ports(),
        }),
    }));
    entries.push(MappingEntry {
        source: ExternalSelector::Definition(ExternalOwnerSelector::Reward {
            key: SourceComponent::Text("unrelated".into()),
        }),
        outcome: MappingOutcome::Mapped {
            target: SchemaSubject::Definition(reward.address()),
            basis: MappingBasis::Exact,
        },
    });
    entries.push(MappingEntry {
        source: source_gem("reviewed-alias-not-a-catalog-row", "v-b"),
        outcome: MappingOutcome::Mapped {
            target: SchemaSubject::Definition(gem.address()),
            basis: MappingBasis::ReviewedAlias {
                reason: key("same-identity"),
            },
        },
    });
    entries.push(MappingEntry {
        source: ExternalSelector::Definition(ExternalOwnerSelector::Reward {
            key: SourceComponent::Text("unresolved-unrelated".into()),
        }),
        outcome: MappingOutcome::Unmapped {
            issue: key("not-converted"),
        },
    });
    let retired: OptionDefId = registry.allocate_definition().unwrap();
    registry
        .retire(
            &SchemaSubject::Definition(retired.address()),
            key("removed"),
        )
        .unwrap();
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: ns(),
            release: key("seed"),
            semantics_version: key("input-v1"),
            definitions,
            slots: vec![SlotDescriptor::Parameter(DefinitionEntry {
                id: parameter,
                schema: SchemaState::Known(ParameterSlotSchema {
                    value: ValueSchema::Integer(range()),
                    presence: SlotPresence::OptionalOnce,
                    sites: vec![ParameterSite::GemParameter],
                }),
            })],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let mappings = mapping(&registry, &schema, entries);
    Seed {
        registry,
        schema,
        mappings,
    }
}
fn mapping(
    registry: &OwnedIdRegistry,
    schema: &OwnedDefinitionSchemaPackage,
    entries: Vec<MappingEntry>,
) -> OwnedMappingIndex {
    OwnedMappingIndex::new(
        MappingPackageInput {
            schema_version: OWNED_MAPPING_PACKAGE_VERSION,
            namespace: ns(),
            registry: registry.identity().unwrap(),
            definitions: schema.identity().clone(),
            source: pin(),
            policy_version: policy(AbsentSupportPolicy::NonSupport).version,
            entries,
        },
        registry,
        schema,
        limits().mapping,
    )
    .unwrap()
}
fn extended_data() -> SkillIdentityData {
    let mut d = data();
    add_skill(&mut d, "new-effect", Some(false));
    add_gem(&mut d, "new-gem", "new-external", "v-c", "new-effect");
    d
}
fn extend(
    seed: &Seed,
    d: SkillIdentityData,
) -> Result<OwnedSkillCatalogExtension, SkillCatalogError> {
    compile_owned_skill_catalog_extension(
        &SkillIdentityCatalog::new(d).unwrap(),
        &seed.registry,
        &seed.schema,
        &seed.mappings,
        &pin(),
        &policy(AbsentSupportPolicy::NonSupport),
        limits(),
    )
}
fn assembled(
    seed: &Seed,
    result: &OwnedSkillCatalogExtension,
) -> (
    OwnedDefinitionSchemaPackage,
    OwnedMappingIndex,
    OwnedSkillRoleIndex,
) {
    let mut input = seed.schema.input().clone();
    input.definitions = result.definitions.clone();
    input.slots = result.slots.clone();
    let schema = OwnedDefinitionSchemaPackage::new(input, OwnedSchemaLimits::default()).unwrap();
    let mapping = mapping(&result.registry, &schema, result.mappings.clone());
    let roles = OwnedSkillRoleIndex::new(
        OwnedSkillRolePackageInput {
            schema_version: OWNED_SKILL_ROLE_VERSION,
            namespace: ns(),
            definitions: schema.identity().clone(),
            mapping: *mapping.identity(),
            compilation: result.receipt.clone(),
            roles: result.roles.clone(),
        },
        &mapping,
        &schema,
        limits(),
    )
    .unwrap();
    (schema, mapping, roles)
}

#[test]
fn extends_only_missing_ids_preserving_known_ports_aliases_partial_gaps_and_tombstones() {
    let seed = seed();
    let before = seed.registry.input().clone();
    let result = extend(&seed, extended_data()).unwrap();
    assert_eq!(
        result.counts,
        SkillCatalogExtensionCounts {
            reused_gems: 2,
            reused_skills: 2,
            allocated_gems: 1,
            allocated_skills: 1,
        }
    );
    assert_eq!(seed.registry.input(), &before);
    assert_eq!(
        &result.registry.input().entries[..before.entries.len()],
        &before.entries
    );
    assert_eq!(
        result.registry.input().last_issued.get(),
        before.last_issued.get() + 2
    );
    seed.registry.validate_successor(&result.registry).unwrap();
    for old in &seed.schema.input().definitions {
        assert_eq!(
            result
                .definitions
                .iter()
                .find(|d| d.address() == old.address())
                .unwrap(),
            old
        );
    }
    assert_eq!(result.slots, seed.schema.input().slots);
    for old in &seed.mappings.input().entries {
        assert_eq!(
            result
                .mappings
                .iter()
                .find(|m| m.source == old.source)
                .unwrap(),
            old
        );
    }
    let (schema, mappings, roles) = assembled(&seed, &result);
    let SchemaSubject::Definition(DefinitionAddress::Gem(gem)) =
        mapped(&result.mappings, &source_gem("new-external", "v-c"))
    else {
        unreachable!()
    };
    assert!(matches!(schema.definition(&gem), SchemaLookup::Unmapped(_)));
    assert!(matches!(
        roles.role(&gem).unwrap().role,
        OwnedGemRole::Known(AuthoredGemRole::SkillUse)
    ));
    assert_eq!(roles.input().mapping, *mappings.identity());
    assert_ne!(schema.identity(), seed.schema.identity());
    assert_eq!(
        result.receipt.policy,
        policy(AbsentSupportPolicy::NonSupport)
    );
}
#[test]
fn repeating_the_extension_allocates_nothing_and_preserves_snapshot_identities() {
    let seed = seed();
    let first = extend(&seed, extended_data()).unwrap();
    let (schema, mappings, _) = assembled(&seed, &first);
    let next = Seed {
        registry: first.registry.clone(),
        schema,
        mappings,
    };
    let second = extend(&next, extended_data()).unwrap();
    assert_eq!(
        second.counts,
        SkillCatalogExtensionCounts {
            reused_gems: 3,
            reused_skills: 3,
            allocated_gems: 0,
            allocated_skills: 0,
        }
    );
    assert_eq!(second.registry.input(), first.registry.input());
    assert_eq!(second.definitions, first.definitions);
    assert_eq!(second.slots, first.slots);
    assert_eq!(second.mappings, first.mappings);
    assert_eq!(second.roles, first.roles);
    let (schema, mapping, _) = assembled(&next, &second);
    assert_eq!(schema.identity(), next.schema.identity());
    assert_eq!(mapping.identity(), next.mappings.identity());
    // Receipt records this operation's actual predecessor; no historical policy proof.
    assert_eq!(second.receipt.base_registry, second.receipt.staged_registry);
}
#[test]
fn affected_ambiguous_unmapped_and_duplicate_source_selectors_never_allocate_replacements() {
    let seed = seed();
    let target = source_gem("external-active", "v-b");
    let active = mapped(&seed.mappings.input().entries, &target);
    let support = mapped(
        &seed.mappings.input().entries,
        &source_gem("external-support", "v-a"),
    );
    for outcome in [
        MappingOutcome::Unmapped {
            issue: key("unresolved"),
        },
        MappingOutcome::Ambiguous {
            candidates: vec![active, support],
            issue: key("ambiguous"),
        },
    ] {
        let mut entries = seed.mappings.input().entries.clone();
        entries
            .iter_mut()
            .find(|m| m.source == target)
            .unwrap()
            .outcome = outcome;
        let changed = Seed {
            registry: seed.registry.clone(),
            schema: seed.schema.clone(),
            mappings: mapping(&seed.registry, &seed.schema, entries),
        };
        assert!(matches!(
            extend(&changed, extended_data()),
            Err(SkillCatalogError::ReuseUnresolved(_))
        ));
        assert_eq!(changed.registry.input(), seed.registry.input());
    }
    let mut d = extended_data();
    add_gem(
        &mut d,
        "duplicate-catalog-row",
        "new-external",
        "v-c",
        "new-effect",
    );
    assert!(matches!(
        extend(&seed, d),
        Err(SkillCatalogError::DuplicateSourceSelector(_))
    ));
    assert_eq!(seed.registry.input().last_issued.get(), 7);
}
#[test]
fn stale_registry_schema_pin_policy_and_retired_schema_subjects_fail_atomically() {
    let seed = seed();
    let catalog = SkillIdentityCatalog::new(extended_data()).unwrap();
    let before = seed.registry.input().clone();
    let mut registry = seed.registry.clone();
    registry.allocate_definition::<OptionDefinition>().unwrap();
    assert!(
        compile_owned_skill_catalog_extension(
            &catalog,
            &registry,
            &seed.schema,
            &seed.mappings,
            &pin(),
            &policy(AbsentSupportPolicy::NonSupport),
            limits()
        )
        .is_err()
    );
    let mut input = seed.schema.input().clone();
    input.release = key("stale-release");
    let changed = OwnedDefinitionSchemaPackage::new(input, OwnedSchemaLimits::default()).unwrap();
    assert!(
        compile_owned_skill_catalog_extension(
            &catalog,
            &seed.registry,
            &changed,
            &seed.mappings,
            &pin(),
            &policy(AbsentSupportPolicy::NonSupport),
            limits()
        )
        .is_err()
    );
    let mut source = pin();
    source.files.push(SourceFilePin {
        path: "unreviewed-extra".into(),
        sha256: "b".repeat(64),
    });
    assert!(
        compile_owned_skill_catalog_extension(
            &catalog,
            &seed.registry,
            &seed.schema,
            &seed.mappings,
            &source,
            &policy(AbsentSupportPolicy::NonSupport),
            limits()
        )
        .is_err()
    );
    let mut changed_policy = policy(AbsentSupportPolicy::NonSupport);
    changed_policy.version = key("changed-version");
    assert!(
        compile_owned_skill_catalog_extension(
            &catalog,
            &seed.registry,
            &seed.schema,
            &seed.mappings,
            &pin(),
            &changed_policy,
            limits()
        )
        .is_err()
    );
    let mut registry = seed.registry.clone();
    registry
        .retire(
            &SchemaSubject::Slot(seed.schema.input().slots[0].address()),
            key("retired-port"),
        )
        .unwrap();
    let mappings = mapping(
        &registry,
        &seed.schema,
        seed.mappings.input().entries.clone(),
    );
    assert!(
        compile_owned_skill_catalog_extension(
            &catalog,
            &registry,
            &seed.schema,
            &mappings,
            &pin(),
            &policy(AbsentSupportPolicy::NonSupport),
            limits()
        )
        .is_err()
    );
    assert_eq!(seed.registry.input(), &before);
}
#[test]
fn known_schema_role_and_materialization_conflicts_are_not_overwritten() {
    let seed = seed();
    let mut d = extended_data();
    for s in &mut d.skills {
        if s.id == "active-effect" {
            s.from_tree = Some(true);
        }
    }
    for s in &mut d.skill_declarations {
        if s.id == "active-effect" {
            s.identity.from_tree = Some(true);
        }
    }
    assert!(matches!(
        extend(&seed, d),
        Err(SkillCatalogError::SchemaConflict)
    ));
    let mut input = seed.schema.input().clone();
    for d in &mut input.definitions {
        if let DefinitionDescriptor::Gem(e) = d
            && let SchemaState::Known(schema) = &mut e.schema
        {
            schema.roles = vec![AuthoredGemRole::SupportAssignment];
        }
    }
    let schema = OwnedDefinitionSchemaPackage::new(input, OwnedSchemaLimits::default()).unwrap();
    let mappings = mapping(
        &seed.registry,
        &schema,
        seed.mappings.input().entries.clone(),
    );
    let changed = Seed {
        registry: seed.registry.clone(),
        schema,
        mappings,
    };
    assert!(matches!(
        extend(&changed, extended_data()),
        Err(SkillCatalogError::SchemaConflict)
    ));
}
#[test]
fn missing_primary_and_provider_only_source_evidence_remain_explicit() {
    let seed = seed();
    let mut d = extended_data();
    add_gem(
        &mut d,
        "missing-primary",
        "missing-external",
        "v",
        "absent-effect",
    );
    add_skill(&mut d, "provider-effect", Some(false));
    for s in &mut d.skills {
        if s.id == "provider-effect" {
            s.from_tree = Some(true);
        }
    }
    for s in &mut d.skill_declarations {
        if s.id == "provider-effect" {
            s.identity.from_tree = Some(true);
        }
    }
    add_gem(
        &mut d,
        "provider-only",
        "provider-external",
        "v",
        "provider-effect",
    );
    let result = extend(&seed, d).unwrap();
    let (schema, _, roles) = assembled(&seed, &result);
    for (source, missing) in [
        (source_gem("missing-external", "v"), true),
        (source_gem("provider-external", "v"), false),
    ] {
        let SchemaSubject::Definition(DefinitionAddress::Gem(gem)) =
            mapped(&result.mappings, &source)
        else {
            unreachable!()
        };
        let role = roles.role(&gem).unwrap();
        assert!(matches!(schema.definition(&gem), SchemaLookup::Unmapped(_)));
        if missing {
            assert!(matches!(role.primary, OwnedPrimarySkill::Unmapped { .. }));
            assert!(matches!(
                role.materialization,
                OwnedGemMaterialization::Unmapped { .. }
            ));
        } else {
            assert_eq!(role.materialization, OwnedGemMaterialization::ProviderOnly);
        }
    }
    assert!(
        result
            .mappings
            .iter()
            .all(|m| m.source != source_skill("absent-effect"))
    );
}
#[test]
fn aggregate_resource_limits_include_preserved_state_and_fail_without_publishing() {
    let seed = seed();
    let d = SkillIdentityCatalog::new(extended_data()).unwrap();
    let before = seed.registry.input().clone();
    for (i, mut l) in [limits(); 5].into_iter().enumerate() {
        match i {
            0 => l.mapping.max_wire_bytes = 1,
            1 => l.mapping.max_entries = 10,
            2 => l.mapping.max_collection_entries = 1,
            3 => l.mapping.max_string_bytes = 1,
            _ => l.mapping.max_total_string_bytes = 1,
        }
        assert!(
            compile_owned_skill_catalog_extension(
                &d,
                &seed.registry,
                &seed.schema,
                &seed.mappings,
                &pin(),
                &policy(AbsentSupportPolicy::NonSupport),
                l
            )
            .is_err()
        );
        l.mapping.max_entries = 0;
        assert!(
            compile_owned_skill_catalog_extension(
                &d,
                &seed.registry,
                &seed.schema,
                &seed.mappings,
                &pin(),
                &policy(AbsentSupportPolicy::NonSupport),
                l
            )
            .is_err()
        );
    }
    assert_eq!(seed.registry.input(), &before);
}
