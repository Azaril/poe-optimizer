use crate::{actions, fixture};
use poe_optimizer_core::{owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{
    owned_mapping::*, owned_normalize::*, owned_reward_policy::*, owned_skill_catalog::*,
    owned_source_actions::*, owned_value::ValueCodecKind, owned_value_policy::*,
};

pub fn fixture() -> fixture::Fixture {
    from_fixture(fixture::Fixture::new())
}

pub fn from_fixture(mut f: fixture::Fixture) -> fixture::Fixture {
    let row = f.row().clone();
    let policy = f
        .base
        .registry
        .allocate_definition::<UsagePolicyDefinition>()
        .unwrap();
    let parameters: Vec<_> = (0..3)
        .map(|_| {
            f.base
                .registry
                .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::UsagePolicy(
                    policy.clone(),
                ))
                .unwrap()
        })
        .collect();
    let ValueCodecKind::Quantity { unit, .. } = &row.deferred_usage[0].value.codec.codec else {
        unreachable!()
    };
    let mut schema = f.base.schema.input().clone();
    schema
        .definitions
        .push(DefinitionDescriptor::UsagePolicy(DefinitionEntry {
            id: policy.clone(),
            schema: SchemaState::Known(UsagePolicySchema {
                scenario_skill_applicability: None,
                targets: vec![UsageTargetKind::Skill],
                declarations: DeclaredSlots {
                    parameters: DeclaredSet::complete(parameters.clone()),
                    choices: DeclaredSet::complete(vec![]),
                    grants: DeclaredSet::complete(vec![]),
                    actors: DeclaredSet::complete(vec![]),
                    skill_grants: DeclaredSet::complete(vec![]),
                    outputs: DeclaredSet::complete(vec![]),
                    sockets: DeclaredSet::complete(vec![]),
                },
            }),
        }));
    for (i, parameter) in parameters.iter().enumerate() {
        schema
            .slots
            .push(SlotDescriptor::Parameter(DefinitionEntry {
                id: parameter.clone(),
                schema: SchemaState::Known(ParameterSlotSchema {
                    value: if i == 0 {
                        ValueSchema::Quantity(QuantityRange {
                            minimum: FiniteQuantity::new(0.0, unit.clone()).unwrap(),
                            maximum: FiniteQuantity::new(1000.0, unit.clone()).unwrap(),
                        })
                    } else {
                        ValueSchema::Boolean
                    },
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::UsagePolicyParameter],
                    skill_input: None,
                }),
            }));
    }
    f.base.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    let mut mapping = f.base.mapping.input().clone();
    mapping.definitions = f.base.schema.identity().clone();
    mapping.registry = f.base.registry.identity().unwrap();
    f.base.mapping = OwnedMappingIndex::new(
        mapping,
        &f.base.registry,
        &f.base.schema,
        Default::default(),
    )
    .unwrap();
    let mut roles = f.base.roles.input().clone();
    roles.definitions = f.base.schema.identity().clone();
    roles.mapping = *f.base.mapping.identity();
    f.base.roles =
        OwnedSkillRoleIndex::new(roles, &f.base.mapping, &f.base.schema, Default::default())
            .unwrap();
    let definition = f.base.schema.identity().clone();
    let GemQualityPolicy::Attributes(quality) = &mut f.policy.gem_quality else {
        unreachable!()
    };
    quality.definitions = definition.clone();
    f.policy.gem_inputs.as_mut().unwrap().definitions = definition.clone();
    let SchemaLookup::Known(gem) = f.base.schema.definition(&f.base.gem) else {
        unreachable!()
    };
    let supply = gem.declarations.skill_grants.members[0].clone();
    let SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 { primary, .. } =
        &f.base.input
    else {
        unreachable!()
    };
    let mut group_count = row.deferred_usage[3].value.clone();
    group_count.missing = MissingValuePolicy::Pending;
    let projections = vec![OccurrenceUsagePolicy {
        policy: policy.clone(),
        parameters: vec![
            UsageParameterInput {
                slot: parameters[0].clone(),
                source: UsageValueSource::ContainingGroupOverride {
                    group: Box::new(group_count),
                    occurrence: row.deferred_usage[0].value.clone(),
                    fallback_admission: UsageFallbackAdmission::RequestedOccurrence,
                },
            },
            UsageParameterInput {
                slot: parameters[1].clone(),
                source: UsageValueSource::Occurrence {
                    value: row.deferred_usage[1].value.clone(),
                },
            },
            UsageParameterInput {
                slot: parameters[2].clone(),
                source: UsageValueSource::Occurrence {
                    value: row.deferred_usage[2].value.clone(),
                },
            },
        ],
    }];
    let attributes: Vec<_> = [
        "gemId",
        "variantId",
        "skillId",
        "nameSpec",
        "level",
        "quality",
        "corrupted",
        "corruptLevel",
        "enabled",
        "count",
        "enableGlobal1",
        "enableGlobal2",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let mut fields: Vec<_> = attributes
        .iter()
        .map(|name| UsageInventoryField {
            field: UsageInventoryFieldSelector::OccurrenceAttribute { name: name.clone() },
            destinations: vec![match name.as_str() {
                "count" => UsageInventoryDestination::UsageParameter {
                    policy: policy.clone(),
                    parameter: parameters[0].clone(),
                },
                "enableGlobal1" => UsageInventoryDestination::UsageParameter {
                    policy: policy.clone(),
                    parameter: parameters[1].clone(),
                },
                "enableGlobal2" => UsageInventoryDestination::UsageParameter {
                    policy: policy.clone(),
                    parameter: parameters[2].clone(),
                },
                _ => UsageInventoryDestination::InputDisposition,
            }],
        })
        .collect();
    fields.extend([
        UsageInventoryField {
            field: UsageInventoryFieldSelector::GroupAttribute {
                name: "enabled".into(),
            },
            destinations: vec![UsageInventoryDestination::InputDisposition],
        },
        UsageInventoryField {
            field: UsageInventoryFieldSelector::GroupAttribute {
                name: "groupCount".into(),
            },
            destinations: vec![UsageInventoryDestination::UsageParameter {
                policy: policy.clone(),
                parameter: parameters[0].clone(),
            }],
        },
        UsageInventoryField {
            field: UsageInventoryFieldSelector::ReferenceDescendants,
            destinations: vec![UsageInventoryDestination::InputDisposition],
        },
    ]);
    f.policy.usage_inputs = Some(UsageInputPolicy::PobOccurrenceUsageV3 {
        definitions: definition.clone(),
        source: f.base.roles.input().compilation.source.clone(),
        roles: *f.base.roles.identity(),
        catalog: f.base.roles.input().compilation.catalog_digest,
        scalar_inputs: gem_inventory_scalar_inputs_identity(&f.policy, Default::default()).unwrap(),
        physical: vec![PrimarySkillUsageInput {
            gem: f.base.gem.clone(),
            game_id: "physical".into(),
            variant_id: "variant".into(),
            skill_id: "effect".into(),
            name_spec: "Fixture skill".into(),
            primary: primary.clone(),
            supply,
            grant: f.base.grant.clone(),
            attributes,
            guards: vec![],
            group_attributes: [
                "enabled",
                "groupCount",
                "mainActiveSkill",
                "mainActiveSkillCalcs",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            group_guards: vec![],
            policies: projections,
        }],
        occurrences: vec![],
        scenario: vec![],
        inventory: Some(UsageInventoryPolicy {
            evidence: "a".repeat(64).parse().unwrap(),
            rows: vec![UsageInventoryRow {
                source: UsageInventorySource::Physical {
                    gem: f.base.gem.clone(),
                },
                fields,
            }],
        }),
    });
    let mut rewards = f.rewards.input().clone();
    rewards.definitions = definition;
    rewards.mapping = *f.base.mapping.identity();
    f.rewards =
        OwnedRewardPolicy::new(rewards, &f.base.mapping, &f.base.schema, Default::default())
            .unwrap();
    rebind(&mut f);
    f
}

pub fn rebind(f: &mut fixture::Fixture) {
    let scalar = gem_inventory_scalar_inputs_identity(&f.policy, Default::default()).unwrap();
    let usage = usage_inputs_identity(&f.policy, Default::default()).unwrap();
    let GemInventoryPolicy::PobFreshPhysicalV3 {
        definitions,
        roles,
        scalar_inputs,
        usage_inputs,
        primary_dispositions,
        ..
    } = f.policy.gem_inventory.as_mut().unwrap();
    *definitions = f.base.schema.identity().clone();
    *roles = *f.base.roles.identity();
    *scalar_inputs = scalar;
    *usage_inputs = usage;
    let (SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
        definitions, roles, ..
    }
    | SourceActionCorrespondenceInput::PobPhysicalSingletonMinionActionsV1 {
        definitions,
        roles,
        ..
    }) = &mut primary_dispositions[0].reference_action
    else {
        unreachable!()
    };
    *definitions = f.base.schema.identity().clone();
    *roles = *f.base.roles.identity();
}

pub fn inventory(f: &mut fixture::Fixture) -> &mut UsageInventoryPolicy {
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        f.policy.usage_inputs.as_mut().unwrap();
    inventory.as_mut().unwrap()
}

pub fn defer(f: &mut fixture::Fixture, field: &str) {
    let row = inventory(f).rows[0].fields.iter_mut().find(|row| matches!(
        &row.field, UsageInventoryFieldSelector::OccurrenceAttribute { name } if name == field
    )).unwrap();
    row.destinations.push(UsageInventoryDestination::Deferred {
        reason: actions::key("unreviewed-consumer"),
    });
    rebind(f);
}
