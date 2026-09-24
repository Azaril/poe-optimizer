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
