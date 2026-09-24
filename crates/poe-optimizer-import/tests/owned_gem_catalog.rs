//! Generic finite source facts compile to owned Gem schemas without evaluator data.
#[allow(dead_code)]
#[path = "support/owned_compact_fixture.rs"]
mod fixture;
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::skill_identities::SkillIdentityCatalog;
use poe_optimizer_import::{
    owned_gem_catalog::*, owned_mapping::*, owned_normalize::*, owned_recipe::*,
    owned_skill_catalog::*, owned_value::*, owned_value_policy::*,
};
use std::{fs, path::PathBuf};

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
struct Context {
    base: StagedOwnedRecipe,
    mapping: OwnedMappingIndex,
    roles: OwnedSkillRoleIndex,
    catalog: SkillIdentityCatalog,
    policy: PhysicalGemSchemaPolicy,
}
fn selector(gem: &poe_optimizer_data::skill_identities::GemIdentity) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::Gem {
        game_id: SourceComponent::Text(gem.game_id.clone()),
        variant_id: SourceComponent::Text(gem.variant_id.clone()),
    })
}
fn context() -> Context {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bundle = fixture::input(&root);
    let base = assemble_owned_recipe(bundle.prior, Default::default()).unwrap();
    let mapping = OwnedMappingIndex::new(
        bundle.mapping,
        base.registry(),
        base.schema(),
        Default::default(),
    )
    .unwrap();
    let roles = OwnedSkillRoleIndex::new(bundle.roles, &mapping, base.schema(), Default::default())
        .unwrap();
    let catalog = SkillIdentityCatalog::new(
        serde_json::from_slice(
            &fs::read(root.join("data/owned/poe2/3887ae68/import/skill-identities.json")).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let source_gems = catalog
        .data()
        .gems
        .iter()
        .filter(|source| {
            if !source.declared_additional_effects.is_empty() || source.effect_list.len() != 1 {
                return false;
            }
            let Some(MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Gem(id)),
                ..
            }) = mapping.lookup(&selector(source))
            else {
                return false;
            };
            matches!(
                roles.role(id),
                Some(OwnedGemRoleRow {
                    role: OwnedGemRole::Known(AuthoredGemRole::SupportAssignment),
                    materialization: OwnedGemMaterialization::Physical,
                    ..
                })
            ) && matches!(base.schema().definition(id), SchemaLookup::Unmapped(_))
        })
        .take(2)
        .map(|source| source.key.clone())
        .collect::<Vec<_>>();
    assert_eq!(source_gems.len(), 2);
    let make_value = |id: &str, attribute: &str, codec| ValueRecipeInput {
        numeric_aliases: vec![],
        id: key(id),
        codec: ValueCodecInput {
            namespace: base.schema().namespace().clone(),
            whitespace: WhitespacePolicy::Exact,
            codec,
        },
        tiers: vec![ValueTier {
            selectors: vec![ValueSelector {
                lane: ValueLane::Attribute,
                name: attribute.into(),
            }],
            duplicates: DuplicatePolicy::Reject,
        }],
        missing: MissingValuePolicy::Pending,
    };
    let policy = PhysicalGemSchemaPolicy {
        effect_membership: GemEffectMembershipPolicy::SinglePrimary,
        schema_version: 1,
        version: key("fixture-physical-inputs-v1"),
        catalog_digest: roles.input().compilation.catalog_digest,
        source_gems,
        level: IntegerRange {
            minimum: BoundedInteger::new(1).unwrap(),
            maximum: BoundedInteger::new(1).unwrap(),
        },
        quality_presence: QualityPresence::Optional,
        quality_kinds: vec![],
        guards: vec![],
        parameters: vec![
            PhysicalGemParameterPolicy {
                schema: ParameterSlotSchema {
                    value: ValueSchema::Boolean,
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::GemParameter],
                },
                value: make_value(
                    "fixture-flag",
                    "corrupted",
                    ValueCodecKind::Boolean {
                        tokens: vec![
                            BooleanToken {
                                token: "true".into(),
                                value: true,
                            },
                            BooleanToken {
                                token: "false".into(),
                                value: false,
                            },
                            BooleanToken {
                                token: "nil".into(),
                                value: false,
                            },
                        ],
                    },
                ),
            },
            PhysicalGemParameterPolicy {
                schema: ParameterSlotSchema {
                    value: ValueSchema::Integer(IntegerRange {
                        minimum: BoundedInteger::new(-8).unwrap(),
                        maximum: BoundedInteger::new(8).unwrap(),
                    }),
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::GemParameter],
                },
                value: make_value(
                    "fixture-delta",
                    "corruptLevel",
                    ValueCodecKind::Integer {
                        syntax: DecimalSyntax::Integer,
                    },
                ),
            },
        ],
    };
    Context {
        base,
        mapping,
        roles,
        catalog,
        policy,
    }
}
fn compile(
    c: &Context,
    policy: &PhysicalGemSchemaPolicy,
) -> Result<CompiledOwnedGemCatalog, PhysicalGemCatalogError> {
    compile_owned_gem_catalog(
        &c.base,
        &c.mapping,
        &c.roles,
        &c.catalog,
        policy,
        Default::default(),
    )
}
#[test]
fn catalog_join_allocates_canonical_slots_and_preserves_unreviewed_coverage() {
    let c = context();
    let result = compile(&c, &c.policy).unwrap();
    assert_eq!(result.receipt.promoted_gems, 2);
    assert_eq!(result.receipt.allocated_parameters, 4);
    assert_eq!(result.receipt.before, *c.base.schema().identity());
    assert_eq!(result.receipt.after, result.staged.receipt.after);
    assert_eq!(
        result.staged.successor.registry.entries.len(),
        c.base.registry().input().entries.len() + 4
    );
    for gem in &result.migration.gems {
        let SchemaState::Known(schema) = &gem.schema else {
            panic!()
        };
        assert!(!schema.skills.is_complete());
        assert!(!schema.quality.allowed_kinds.is_complete());
        assert!(!schema.declarations.parameters.is_complete());
        assert!(!schema.declarations.choices.is_complete());
        assert!(!schema.declarations.grants.is_complete());
        assert!(!schema.declarations.actors.is_complete());
        assert!(!schema.declarations.skill_grants.is_complete());
        assert!(!schema.declarations.outputs.is_complete());
        assert!(!schema.declarations.sockets.is_complete());
        assert_eq!(schema.declarations.parameters.members.len(), 2);
        let rule = result
            .inputs
            .iter()
            .find(|rule| rule.gem == gem.id)
            .unwrap();
        assert_eq!(
            rule.parameters
                .iter()
                .map(|p| p.slot.clone())
                .collect::<Vec<_>>(),
            schema.declarations.parameters.members
        );
        for (actual, expected) in rule.parameters.iter().zip(&c.policy.parameters) {
            assert_eq!(actual.value, expected.value);
        }
    }
    let mut reverse = c.policy.clone();
    reverse.source_gems.reverse();
    let reordered = compile(&c, &reverse).unwrap();
    assert_eq!(reordered.migration, result.migration);
    assert_eq!(reordered.inputs, result.inputs);
    assert_ne!(reordered.receipt.policy, result.receipt.policy);
    let promoted: Vec<_> = result
        .migration
        .gems
        .iter()
        .map(|gem| gem.id.address())
        .collect();
    assert_eq!(
        c.base
            .schema()
            .input()
            .definitions
            .iter()
            .filter(|row| !promoted.contains(&row.address()))
            .collect::<Vec<_>>(),
        result
            .staged
            .successor
            .schema
            .definitions
            .iter()
            .filter(|row| !promoted.contains(&row.address()))
            .collect::<Vec<_>>()
    );
    let mut rules = result.staged.successor.rules.clone();
    rules.definitions = c.base.rules().input().definitions.clone();
    assert_eq!(rules, *c.base.rules().input());
    let mut routing = result.staged.successor.routing.clone();
    routing.definitions = c.base.routing().input().definitions.clone();
    assert_eq!(routing, *c.base.routing().input());
}
#[test]
fn duplicate_unknown_stale_and_unbounded_policy_inputs_fail() {
    let c = context();
    for case in 0..8 {
        let mut policy = c.policy.clone();
        match case {
            0 => policy.source_gems.clear(),
            1 => policy.source_gems.push(policy.source_gems[0].clone()),
            2 => policy.source_gems[0] = "absent-source-definition".into(),
            3 => policy.catalog_digest = digest_owned("wrong", &0, 100).unwrap(),
            4 => policy.schema_version += 1,
            5 => policy.parameters.clear(),
            6 => {
                policy.parameters[0].value.missing = MissingValuePolicy::Explicit {
                    value: poe_optimizer_core::owned_build::ParameterValue::Boolean(false),
                }
            }
            _ => policy.source_gems[0] = "x".repeat(16 * 1024 + 1),
        }
        assert!(compile(&c, &policy).is_err(), "case {case}");
    }
    for limits in [
        PhysicalGemCatalogLimits {
            max_wire_bytes: 1,
            ..Default::default()
        },
        PhysicalGemCatalogLimits {
            max_source_gems: 1,
            ..Default::default()
        },
        PhysicalGemCatalogLimits {
            max_parameters: 1,
            ..Default::default()
        },
        PhysicalGemCatalogLimits {
            max_parameters: 65,
            ..Default::default()
        },
        PhysicalGemCatalogLimits {
            migration: poe_optimizer_import::owned_gem_schema::GemSchemaMigrationLimits {
                max_entries: 1,
                ..Default::default()
            },
            ..Default::default()
        },
    ] {
        assert!(
            compile_owned_gem_catalog(&c.base, &c.mapping, &c.roles, &c.catalog, &c.policy, limits)
                .is_err()
        );
    }
}
#[test]
fn typed_value_schema_and_source_guard_validation_are_shared() {
    let c = context();
    for case in 0..6 {
        let mut policy = c.policy.clone();
        match case {
            0 => policy.parameters[0].schema.value = policy.parameters[1].schema.value.clone(),
            1 => policy.parameters[0].schema.sites = vec![ParameterSite::ItemParameter],
            2 => policy.parameters[0].value.tiers[0].selectors[0].lane = ValueLane::ParentAttribute,
            3 => {
                policy.parameters[0].value.codec.namespace =
                    GameVersionNamespace::new("foreign", "v1").unwrap()
            }
            4 => policy.guards.push(GemInputGuard {
                attribute: "".into(),
                allowed: vec![],
            }),
            _ => {
                policy.guards = vec![GemInputGuard {
                    attribute: "corrupted".into(),
                    allowed: vec![SourceComponent::Missing, SourceComponent::Missing],
                }]
            }
        }
        assert!(compile(&c, &policy).is_err(), "case {case}");
    }
}
#[test]
fn additional_declared_effects_are_not_erased_by_constructed_singletons() {
    let c = context();
    let source = c
        .catalog
        .data()
        .gems
        .iter()
        .find(|source| {
            !source.declared_additional_effects.is_empty() && source.effect_list.len() == 1
        })
        .unwrap();
    let mut policy = c.policy.clone();
    policy.source_gems = vec![source.key.clone()];
    let error = compile(&c, &policy).err().unwrap().to_string();
    assert!(error.contains("unreviewed potential effects"), "{error}");
}
#[test]
fn provider_roles_and_primary_mapping_contradictions_are_rejected() {
    let c = context();
    let source = c.catalog.gem_by_key(&c.policy.source_gems[0]).unwrap();
    let Some(MappingOutcome::Mapped {
        target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)),
        ..
    }) = c.mapping.lookup(&selector(source))
    else {
        panic!()
    };
    for case in 0..2 {
        let mut input = c.roles.input().clone();
        let row = input.roles.iter_mut().find(|row| &row.gem == gem).unwrap();
        match case {
            0 => row.materialization = OwnedGemMaterialization::ProviderOnly,
            _ => row.role = OwnedGemRole::Known(AuthoredGemRole::SkillUse),
        }
        let roles =
            OwnedSkillRoleIndex::new(input, &c.mapping, c.base.schema(), Default::default())
                .unwrap();
        assert!(
            compile_owned_gem_catalog(
                &c.base,
                &c.mapping,
                &roles,
                &c.catalog,
                &c.policy,
                Default::default()
            )
            .is_err()
        );
    }
}
#[test]
fn declared_input_recipe_keeps_missing_and_literal_nil_numeric_values_unresolved() {
    let c = context();
    let result = compile(&c, &c.policy).unwrap();
    let value = &result.inputs[0].parameters[1].value;
    assert_eq!(value.missing, MissingValuePolicy::Pending);
    let codec = OwnedValueCodec::new(value.codec.clone(), Default::default()).unwrap();
    assert!(codec.decode("nil").is_err());
    assert!(codec.decode("bad").is_err());
    assert_eq!(
        codec.decode("-1").unwrap(),
        poe_optimizer_core::owned_build::ParameterValue::Integer(BoundedInteger::new(-1).unwrap())
    );
}

#[test]
fn shared_policy_fanout_is_bounded_before_slot_or_schema_expansion() {
    let c = context();
    let mut policy = c.policy.clone();
    policy.guards = vec![GemInputGuard {
        attribute: "reviewed".into(),
        allowed: (0..64)
            .map(|i| SourceComponent::Text(format!("{i}{}", "x".repeat(16 * 1024 - 4))))
            .collect(),
    }];
    let limits = PhysicalGemCatalogLimits {
        max_wire_bytes: 2 * 1024 * 1024,
        ..Default::default()
    };
    let error =
        compile_owned_gem_catalog(&c.base, &c.mapping, &c.roles, &c.catalog, &policy, limits)
            .err()
            .unwrap()
            .to_string();
    assert!(
        error.contains("template expansion exceeds byte budget"),
        "{error}"
    );
}
#[test]
fn an_unselected_colliding_source_alias_cannot_supply_a_selected_identity() {
    let c = context();
    let first = c.catalog.gem_by_key(&c.policy.source_gems[0]).unwrap();
    let mut data = c.catalog.data().clone();
    let second = data
        .gems
        .iter_mut()
        .find(|gem| gem.key == c.policy.source_gems[1])
        .unwrap();
    second.game_id = first.game_id.clone();
    second.variant_id = first.variant_id.clone();
    let declaration = data
        .gem_declarations
        .iter_mut()
        .find(|row| row.index == second.winning_declaration)
        .unwrap();
    declaration.identity.game_id = first.game_id.clone();
    declaration.identity.variant_id = first.variant_id.clone();
    let catalog = SkillIdentityCatalog::new(data).unwrap();
    let catalog_digest = digest_owned(
        "owned-skill-source-catalog-v1",
        catalog.data(),
        16 * 1024 * 1024,
    )
    .unwrap();
    let mut policy = c.policy.clone();
    policy.catalog_digest = catalog_digest;
    policy.source_gems.truncate(1);
    let mut input = c.roles.input().clone();
    input.compilation.catalog_digest = catalog_digest;
    let roles =
        OwnedSkillRoleIndex::new(input, &c.mapping, c.base.schema(), Default::default()).unwrap();
    let error = compile_owned_gem_catalog(
        &c.base,
        &c.mapping,
        &roles,
        &catalog,
        &policy,
        Default::default(),
    )
    .err()
    .unwrap()
    .to_string();
    assert!(error.contains("ambiguous identity"), "{error}");
}

fn multi_policy(c: &Context) -> PhysicalGemSchemaPolicy {
    let mut policy = c.policy.clone();
    policy.version = key("fixture-resolved-potential-skills-v1");
    policy.effect_membership = GemEffectMembershipPolicy::ResolvedPotentialSkillsV1;
    policy.source_gems = c
        .catalog
        .data()
        .gems
        .iter()
        .filter(|source| {
            if source.effect_list.len() <= 1
                || !source.declared_additional_stat_sets.is_empty()
                || source
                    .constructed_additional_effects
                    .iter()
                    .any(|row| c.catalog.skill_by_id(&row.id).is_none())
            {
                return false;
            }
            let Some(MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)),
                basis: MappingBasis::Exact,
            }) = c.mapping.lookup(&selector(source))
            else {
                return false;
            };
            matches!(
                c.roles.role(gem),
                Some(OwnedGemRoleRow {
                    role: OwnedGemRole::Known(AuthoredGemRole::SupportAssignment),
                    materialization: OwnedGemMaterialization::Physical,
                    ..
                })
            ) && matches!(c.base.schema().definition(gem), SchemaLookup::Unmapped(_))
        })
        .map(|source| source.key.clone())
        .collect();
    policy
}
fn rebind_catalog(c: &mut Context, data: poe_optimizer_data::skill_identities::SkillIdentityData) {
    c.catalog = SkillIdentityCatalog::new(data).unwrap();
    let digest = digest_owned(
        "owned-skill-source-catalog-v1",
        c.catalog.data(),
        16 * 1024 * 1024,
    )
    .unwrap();
    c.policy.catalog_digest = digest;
    let mut roles = c.roles.input().clone();
    roles.compilation.catalog_digest = digest;
    c.roles =
        OwnedSkillRoleIndex::new(roles, &c.mapping, c.base.schema(), Default::default()).unwrap();
}
fn skill_selector(effect: &str) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::Skill {
        effect_id: SourceComponent::Text(effect.into()),
    })
}

#[test]
fn resolved_two_and_three_effects_keep_one_physical_gem_and_partial_potential_membership() {
    let c = context();
    let policy = multi_policy(&c);
    assert_eq!(policy.source_gems.len(), 52);
    let result = compile(&c, &policy).unwrap();
    assert_eq!(result.receipt.promoted_gems, 52);
    assert_eq!(result.receipt.allocated_parameters, 104);
    let mut members = 0;
    let mut generated_only = 0;
    let mut primary_last = 0;
    for key in &policy.source_gems {
        let source = c.catalog.gem_by_key(key).unwrap();
        let Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)),
            ..
        }) = c.mapping.lookup(&selector(source))
        else {
            panic!()
        };
        let migrated = result
            .migration
            .gems
            .iter()
            .find(|row| &row.id == gem)
            .unwrap();
        let SchemaState::Known(schema) = &migrated.schema else {
            panic!()
        };
        let mut expected = source
            .effect_list
            .iter()
            .map(|effect| {
                let Some(MappingOutcome::Mapped {
                    target: SchemaSubject::Definition(DefinitionAddress::Skill(skill)),
                    basis: MappingBasis::Exact,
                }) = c.mapping.lookup(&skill_selector(effect))
                else {
                    panic!()
                };
                skill.clone()
            })
            .collect::<Vec<_>>();
        expected.sort();
        assert_eq!(schema.skills.members, expected);
        assert!(!schema.skills.is_complete());
        assert_eq!(schema.roles, [AuthoredGemRole::SupportAssignment]);
        assert!(!schema.declarations.parameters.is_complete());
        assert!(schema.declarations.skill_grants.members.is_empty());
        assert!(schema.declarations.outputs.members.is_empty());
        assert!(!schema.declarations.skill_grants.is_complete());
        assert!(!schema.declarations.outputs.is_complete());
        assert_eq!(
            result.inputs.iter().filter(|row| &row.gem == gem).count(),
            1
        );
        members += expected.len();
        generated_only += usize::from(source.declared_additional_effects.is_empty());
        primary_last += usize::from(source.effect_list.last() == Some(&source.primary_effect_id));
    }
    assert_eq!(members, 105);
    assert!(generated_only >= 3);
    assert!(primary_last >= 1);
    assert_eq!(
        result.staged.successor.schema.definitions.len(),
        c.base.schema().input().definitions.len()
    );
}

#[test]
fn final_effect_display_order_does_not_become_owned_membership_order() {
    let mut c = context();
    c.policy = multi_policy(&c);
    let chosen = c
        .policy
        .source_gems
        .iter()
        .find(|key| c.catalog.gem_by_key(key).unwrap().effect_list.len() == 3)
        .unwrap()
        .clone();
    c.policy.source_gems = vec![chosen.clone()];
    let original = compile(&c, &c.policy).unwrap();
    let mut data = c.catalog.data().clone();
    data.gems
        .iter_mut()
        .find(|source| source.key == chosen)
        .unwrap()
        .effect_list
        .reverse();
    rebind_catalog(&mut c, data);
    let reordered = compile(&c, &c.policy).unwrap();
    assert_eq!(original.staged.successor, reordered.staged.successor);
    assert_eq!(original.inputs, reordered.inputs);
}

#[test]
fn multi_effect_source_sets_reject_duplicates_missing_members_and_stat_sets() {
    use poe_optimizer_data::skill_identities::IndexedIdentityReference;
    for case in 0..7 {
        let mut c = context();
        c.policy = multi_policy(&c);
        let chosen = c
            .policy
            .source_gems
            .iter()
            .find(|key| {
                let source = c.catalog.gem_by_key(key).unwrap();
                source.effect_list.len() == 3 && source.declared_additional_effects.len() == 2
            })
            .unwrap()
            .clone();
        c.policy.source_gems = vec![chosen.clone()];
        let mut data = c.catalog.data().clone();
        let source = data
            .gems
            .iter_mut()
            .find(|source| source.key == chosen)
            .unwrap();
        match case {
            0 => {
                source.additional_effects.pop();
            }
            1 => {
                source
                    .effect_list
                    .retain(|effect| effect != &source.primary_effect_id);
            }
            2 => source.effect_list.push(source.effect_list[0].clone()),
            3 => source
                .constructed_additional_effects
                .push(IndexedIdentityReference {
                    index: 3,
                    id: source.constructed_additional_effects[0].id.clone(),
                }),
            4 => source
                .additional_effects
                .push(source.additional_effects[0].clone()),
            5 => source
                .declared_additional_stat_sets
                .push(IndexedIdentityReference {
                    index: 1,
                    id: source.primary_effect_id.clone(),
                }),
            _ => {
                let missing = IndexedIdentityReference {
                    index: 3,
                    id: "unresolved-source-effect".into(),
                };
                source.declared_additional_effects.push(missing.clone());
                source.constructed_additional_effects.push(missing);
            }
        }
        let declaration = &mut data.gem_declarations[source.winning_declaration as usize - 1];
        declaration.identity.additional_effects = source.declared_additional_effects.clone();
        declaration.identity.additional_stat_sets = source.declared_additional_stat_sets.clone();
        data.missing_references = data.expected_missing_references();
        rebind_catalog(&mut c, data);
        assert!(compile(&c, &c.policy).is_err(), "case {case}");
    }
    let mut c = context();
    let source = c
        .catalog
        .data()
        .gems
        .iter()
        .find(|source| {
            !source.declared_additional_effects.is_empty() && source.effect_list.len() == 1
        })
        .unwrap();
    c.policy.source_gems = vec![source.key.clone()];
    c.policy.effect_membership = GemEffectMembershipPolicy::ResolvedPotentialSkillsV1;
    assert!(
        compile(&c, &c.policy).is_err(),
        "an unresolved declared effect cannot disappear behind the constructed singleton"
    );
}

#[test]
fn every_additional_skill_requires_an_exact_mapping_and_owned_identity() {
    for case in 0..3 {
        let mut c = context();
        c.policy = multi_policy(&c);
        c.policy.source_gems.truncate(1);
        let source = c.catalog.gem_by_key(&c.policy.source_gems[0]).unwrap();
        let primary_selector = skill_selector(&source.primary_effect_id);
        let extra_selector = skill_selector(&source.additional_effects[0]);
        let primary_outcome = c.mapping.lookup(&primary_selector).unwrap().clone();
        let extra_outcome = c.mapping.lookup(&extra_selector).unwrap().clone();
        let mut mapping = c.mapping.input().clone();
        match case {
            0 => mapping
                .entries
                .retain(|entry| entry.source != extra_selector),
            1 => {
                for entry in &mut mapping.entries {
                    if entry.source == primary_selector {
                        entry.outcome = extra_outcome.clone();
                    }
                    if entry.source == extra_selector {
                        entry.outcome = primary_outcome.clone();
                    }
                }
            }
            _ => {
                let MappingOutcome::Mapped { target, .. } = primary_outcome else {
                    panic!()
                };
                mapping
                    .entries
                    .iter_mut()
                    .find(|entry| entry.source == extra_selector)
                    .unwrap()
                    .outcome = MappingOutcome::Mapped {
                    target,
                    basis: MappingBasis::ReviewedAlias {
                        reason: key("fixture-alias"),
                    },
                };
            }
        }
        c.mapping = OwnedMappingIndex::new(
            mapping,
            c.base.registry(),
            c.base.schema(),
            Default::default(),
        )
        .unwrap();
        let mut roles = c.roles.input().clone();
        roles.mapping = *c.mapping.identity();
        c.roles = OwnedSkillRoleIndex::new(roles, &c.mapping, c.base.schema(), Default::default())
            .unwrap();
        assert!(compile(&c, &c.policy).is_err(), "case {case}");
    }
    let c = context();
    let policy = multi_policy(&c);
    let source = c.catalog.gem_by_key(&policy.source_gems[0]).unwrap();
    let mut mapping = c.mapping.input().clone();
    let primary = c
        .mapping
        .lookup(&skill_selector(&source.primary_effect_id))
        .unwrap()
        .clone();
    mapping
        .entries
        .iter_mut()
        .find(|entry| entry.source == skill_selector(&source.additional_effects[0]))
        .unwrap()
        .outcome = primary;
    assert!(
        OwnedMappingIndex::new(
            mapping,
            c.base.registry(),
            c.base.schema(),
            Default::default()
        )
        .is_err(),
        "duplicate Exact targets are rejected at the mapping boundary before compilation"
    );
}

#[test]
fn membership_limits_bound_each_gem_total_rows_and_expansion_before_cloning() {
    let c = context();
    let policy = multi_policy(&c);
    for limits in [
        PhysicalGemCatalogLimits {
            max_skills_per_gem: 1,
            ..Default::default()
        },
        PhysicalGemCatalogLimits {
            max_total_skill_memberships: 104,
            ..Default::default()
        },
        PhysicalGemCatalogLimits {
            max_skills_per_gem: 0,
            ..Default::default()
        },
        PhysicalGemCatalogLimits {
            max_total_skill_memberships: 4097,
            ..Default::default()
        },
    ] {
        assert!(
            compile_owned_gem_catalog(&c.base, &c.mapping, &c.roles, &c.catalog, &policy, limits)
                .is_err()
        );
    }
    let result = compile_owned_gem_catalog(
        &c.base,
        &c.mapping,
        &c.roles,
        &c.catalog,
        &policy,
        PhysicalGemCatalogLimits {
            max_skills_per_gem: 3,
            max_total_skill_memberships: 105,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.receipt.promoted_gems, 52);
    let mut expanded = policy;
    expanded.guards = vec![GemInputGuard {
        attribute: "reviewed".into(),
        allowed: (0..3)
            .map(|i| SourceComponent::Text(format!("{i}{}", "x".repeat(15_000))))
            .collect(),
    }];
    let template_bytes = serde_json::to_vec(&(
        &expanded.level,
        &expanded.quality_kinds,
        &expanded.guards,
        &expanded.parameters,
    ))
    .unwrap()
    .len();
    let base_bytes =
        expanded.source_gems.len() * (expanded.parameters.len() * 2048 + 8192 + template_bytes);
    let error = compile_owned_gem_catalog(
        &c.base,
        &c.mapping,
        &c.roles,
        &c.catalog,
        &expanded,
        PhysicalGemCatalogLimits {
            max_wire_bytes: base_bytes,
            ..Default::default()
        },
    )
    .err()
    .unwrap()
    .to_string();
    assert!(
        error.contains("potential skill expansion exceeds byte budget"),
        "{error}"
    );
}

#[test]
fn omitted_single_primary_mode_preserves_legacy_wire_and_migration_output() {
    #[derive(serde::Serialize)]
    struct Legacy<'a> {
        schema_version: u32,
        version: &'a OwnedDefinitionKey,
        catalog_digest: poe_optimizer_core::owned_content::OwnedContentDigest,
        source_gems: &'a Vec<String>,
        level: &'a IntegerRange,
        quality_presence: QualityPresence,
        quality_kinds: &'a Vec<QualityDefId>,
        guards: &'a Vec<GemInputGuard>,
        parameters: &'a Vec<PhysicalGemParameterPolicy>,
    }
    let c = context();
    let policy = &c.policy;
    let legacy = Legacy {
        schema_version: policy.schema_version,
        version: &policy.version,
        catalog_digest: policy.catalog_digest,
        source_gems: &policy.source_gems,
        level: &policy.level,
        quality_presence: policy.quality_presence,
        quality_kinds: &policy.quality_kinds,
        guards: &policy.guards,
        parameters: &policy.parameters,
    };
    let bytes = serde_json::to_vec(&legacy).unwrap();
    assert_eq!(serde_json::to_vec(policy).unwrap(), bytes);
    let omitted: PhysicalGemSchemaPolicy = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        omitted.effect_membership,
        GemEffectMembershipPolicy::SinglePrimary
    );
    let mut explicit = serde_json::to_value(&omitted).unwrap();
    explicit["effect_membership"] = serde_json::json!("single_primary");
    let explicit: PhysicalGemSchemaPolicy = serde_json::from_value(explicit).unwrap();
    assert_eq!(serde_json::to_vec(&explicit).unwrap(), bytes);
    let old = compile(&c, &omitted).unwrap();
    assert_eq!(
        old.receipt.policy,
        digest_owned(
            "owned-physical-gem-schema-policy-v1",
            &legacy,
            16 * 1024 * 1024
        )
        .unwrap()
    );
    assert_eq!(compile(&c, &explicit).unwrap().migration, old.migration);
    let mut resolved = omitted;
    resolved.effect_membership = GemEffectMembershipPolicy::ResolvedPotentialSkillsV1;
    let same_singletons = compile(&c, &resolved).unwrap();
    assert_eq!(same_singletons.migration, old.migration);
    assert_eq!(same_singletons.staged.successor, old.staged.successor);
    assert_ne!(same_singletons.receipt.policy, old.receipt.policy);
}
