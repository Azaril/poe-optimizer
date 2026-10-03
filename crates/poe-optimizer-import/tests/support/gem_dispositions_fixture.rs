use crate::actions;
#[path = "empty_owned_items.rs"]
mod items;
use actions::{key, ns};
use poe_optimizer_core::{build_identity::BuildLineage, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_mapping::*,
    owned_normalize::*,
    owned_reward_policy::*,
    owned_skill_catalog::*,
    owned_source::*,
    owned_source_actions::*,
    owned_value::*,
    owned_value_policy::*,
};

pub const GEM: &str = r#"<Gem gemId="physical" variantId="variant" skillId="effect" nameSpec="Fixture skill" level="17" quality="0" corrupted="false" corruptLevel="0" enabled="true" count="1" enableGlobal1="true" enableGlobal2="true"/>"#;
pub fn xml(gems: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Build level="70"/><Skills activeSkillSet="1"><SkillSet id="1"><Skill enabled="true">{gems}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    )
}
pub fn with_children(children: &str) -> String {
    GEM.replace("/>", &format!(">{children}</Gem>"))
}
pub fn maps(main: &str, calcs: &str) -> String {
    format!(
        r#"<StatSetIndex grantedEffect="effect" index="{main}"/><StatSetCalcsIndex grantedEffect="effect" index="{calcs}"/>"#
    )
}
fn scalar(id: &str, attribute: &str, codec: ValueCodecKind) -> ValueRecipeInput {
    ValueRecipeInput {
        id: key(id),
        codec: ValueCodecInput {
            namespace: ns(),
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
        numeric_aliases: vec![],
    }
}
fn boolean(id: &str, attribute: &str) -> ValueRecipeInput {
    scalar(
        id,
        attribute,
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
    )
}
fn quantity(id: &str, attribute: &str, unit: UnitDefId) -> ValueRecipeInput {
    scalar(
        id,
        attribute,
        ValueCodecKind::Quantity {
            syntax: DecimalSyntax::Scientific,
            unit,
            scale: RationalScale {
                numerator: BoundedInteger::new(1).unwrap(),
                denominator: BoundedInteger::new(1).unwrap(),
            },
        },
    )
}
pub struct Fixture {
    pub base: actions::Fixture,
    pub policy: NormalizationPolicy,
    pub rewards: OwnedRewardPolicy,
}
impl Fixture {
    pub fn new() -> Self {
        Self::from_base(actions::Fixture::new())
    }
    pub fn from_base(mut base: actions::Fixture) -> Self {
        let owner = SlotOwnerDefId::Gem(base.gem.clone());
        let corrupted = base
            .registry
            .allocate_slot::<ParameterSlotDefinition>(owner.clone())
            .unwrap();
        let delta = base
            .registry
            .allocate_slot::<ParameterSlotDefinition>(owner)
            .unwrap();
        let count = base
            .registry
            .allocate_definition::<UnitDefinition>()
            .unwrap();
        let percent = base
            .registry
            .allocate_definition::<UnitDefinition>()
            .unwrap();
        let quality = base
            .registry
            .allocate_definition::<QualityDefinition>()
            .unwrap();
        let mut schema = base.schema.input().clone();
        for (unit, dimension) in [
            (count.clone(), UnitDimension::Count),
            (percent.clone(), UnitDimension::PercentagePoints),
        ] {
            schema
                .definitions
                .push(DefinitionDescriptor::Unit(DefinitionEntry {
                    id: unit,
                    schema: SchemaState::Known(UnitSchema { dimension }),
                }));
        }
        schema
            .definitions
            .push(DefinitionDescriptor::Quality(DefinitionEntry {
                id: quality.clone(),
                schema: SchemaState::Known(QualitySchema {
                    amount: QuantityRange {
                        minimum: FiniteQuantity::new(0.0, percent.clone()).unwrap(),
                        maximum: FiniteQuantity::new(100.0, percent.clone()).unwrap(),
                    },
                }),
            }));
        for definition in &mut schema.definitions {
            if let DefinitionDescriptor::Gem(row) = definition
                && row.id == base.gem
            {
                let SchemaState::Known(gem) = &mut row.schema else {
                    unreachable!()
                };
                gem.declarations.parameters.members = vec![corrupted.clone(), delta.clone()];
                gem.quality = QualityUseSchema {
                    presence: QualityPresence::Required,
                    allowed_kinds: DeclaredSet::complete(vec![quality.clone()]),
                };
            }
        }
        for (slot, value) in [
            (corrupted.clone(), ValueSchema::Boolean),
            (
                delta.clone(),
                ValueSchema::Quantity(QuantityRange {
                    minimum: FiniteQuantity::new(-10.0, count.clone()).unwrap(),
                    maximum: FiniteQuantity::new(10.0, count.clone()).unwrap(),
                }),
            ),
        ] {
            schema
                .slots
                .push(SlotDescriptor::Parameter(DefinitionEntry {
                    id: slot,
                    schema: SchemaState::Known(ParameterSlotSchema {
                        value,
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![ParameterSite::GemParameter],
                        skill_input: None,
                    }),
                }));
        }
        base.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
        let mut mapping = base.mapping.input().clone();
        mapping.registry = base.registry.identity().unwrap();
        mapping.definitions = base.schema.identity().clone();
        base.mapping =
            OwnedMappingIndex::new(mapping, &base.registry, &base.schema, Default::default())
                .unwrap();
        let mut roles = base.roles.input().clone();
        roles.definitions = base.schema.identity().clone();
        roles.mapping = *base.mapping.identity();
        base.roles =
            OwnedSkillRoleIndex::new(roles, &base.mapping, &base.schema, Default::default())
                .unwrap();
        let (definitions, roles) = match &mut base.input {
            SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
                definitions,
                roles,
                ..
            }
            | SourceActionCorrespondenceInput::PobPhysicalSingletonMinionActionsV1 {
                definitions,
                roles,
                ..
            } => (definitions, roles),
        };
        *definitions = base.schema.identity().clone();
        *roles = *base.roles.identity();
        let mut corruption = quantity("corruption-delta", "corruptLevel", count.clone());
        corruption.numeric_aliases.push(NumericTokenAlias {
            token: "nil".into(),
            replacement: "0".into(),
        });
        let mut policy = NormalizationPolicy {
            version: key("fixture-v1"),
            namespace: ns(),
            character_level: scalar(
                "character-level",
                "level",
                ValueCodecKind::Integer {
                    syntax: DecimalSyntax::Integer,
                },
            ),
            gem_level: scalar(
                "gem-level",
                "level",
                ValueCodecKind::Integer {
                    syntax: DecimalSyntax::Integer,
                },
            ),
            gem_enabled: boolean("gem-enabled", "enabled"),
            group_enabled: boolean("group-enabled", "enabled"),
            manual_skill_sources: vec![
                SourceComponent::Missing,
                SourceComponent::Text(String::new()),
            ],
            empty_item_keys: vec![SourceComponent::Text("0".into())],
            generated_support_prefixes: vec!["Item:".into(), "Tree:".into()],
            allocation_attribute: "nodes".into(),
            single_active_support_target: true,
            equipment_loadouts: vec![],
            skill_scopes: None,
            gem_quality: GemQualityPolicy::Attributes(Box::new(GemQualityPolicyInput {
                definitions: base.schema.identity().clone(),
                amount: quantity("gem-quality", "quality", percent),
                kind_attribute: "qualityId".into(),
                kinds: vec![GemQualityKindRule {
                    source: SourceComponent::Missing,
                    kind: quality,
                }],
            })),
            gem_inputs: Some(GemInputPolicy {
                definitions: base.schema.identity().clone(),
                gems: vec![GemInputRule {
                    gem: base.gem.clone(),
                    guards: vec![],
                    parameters: vec![
                        GemParameterInput {
                            slot: corrupted.clone(),
                            value: boolean("corrupted", "corrupted"),
                        },
                        GemParameterInput {
                            slot: delta.clone(),
                            value: corruption,
                        },
                    ],
                }],
            }),
            direct_skill_inputs: None,
            gem_inventory: None,
            usage_inputs: None,
            support_origin_order: None,
            payload_inventory: None,
            equipment_membership: None,
            passive_socket_membership: None,
            item_modifier_membership: None,
            item_parameter_inputs: None,
            enemy_level: None,
            configuration_reward_inventory: None,
            encounter: None,
            character_reward_inventory: None,
            configuration_inputs: None,
        };
        let mut group_count = quantity("group-count", "groupCount", count.clone());
        group_count.missing = MissingValuePolicy::Absent;
        let mut full_dps = boolean("group-full-dps", "includeInFullDPS");
        full_dps.missing = MissingValuePolicy::Absent;
        policy.gem_inventory = Some(GemInventoryPolicy::PobFreshPhysicalV3 {
            definitions: base.schema.identity().clone(),
            roles: *base.roles.identity(),
            catalog: base.roles.input().compilation.catalog_digest,
            scalar_inputs: gem_inventory_scalar_inputs_identity(&policy, Default::default())
                .unwrap(),
            usage_inputs: usage_inputs_identity(&policy, Default::default()).unwrap(),
            supports: vec![],
            primary_skills: vec![],
            primary_dispositions: vec![PrimaryGemInputDisposition {
                physical: PhysicalGemInputInventory {
                    gem: base.gem.clone(),
                    game_id: "physical".into(),
                    variant_id: "variant".into(),
                    skill_id: "effect".into(),
                    name_spec: "Fixture skill".into(),
                    corrupted,
                    corruption_level: delta,
                },
                reference_action: base.input.clone(),
                deferred_usage: vec![
                    DeferredGemUsageInput {
                        field: DeferredGemUsageField::GemCount,
                        value: quantity("gem-count", "count", count),
                    },
                    DeferredGemUsageInput {
                        field: DeferredGemUsageField::GemGlobal1,
                        value: boolean("global-one", "enableGlobal1"),
                    },
                    DeferredGemUsageInput {
                        field: DeferredGemUsageField::GemGlobal2,
                        value: boolean("global-two", "enableGlobal2"),
                    },
                    DeferredGemUsageInput {
                        field: DeferredGemUsageField::GroupCount,
                        value: group_count,
                    },
                    DeferredGemUsageInput {
                        field: DeferredGemUsageField::GroupFullDps,
                        value: full_dps,
                    },
                ],
            }],
        });
        let rewards = OwnedRewardPolicy::new(
            RewardPolicyInput {
                schema_version: OWNED_REWARD_POLICY_VERSION,
                namespace: ns(),
                version: key("empty-fixture-rewards"),
                definitions: base.schema.identity().clone(),
                mapping: *base.mapping.identity(),
                rules: vec![],
            },
            &base.mapping,
            &base.schema,
            Default::default(),
        )
        .unwrap();
        Self {
            base,
            policy,
            rewards,
        }
    }
    pub fn row(&mut self) -> &mut PrimaryGemInputDisposition {
        let GemInventoryPolicy::PobFreshPhysicalV3 {
            primary_dispositions,
            ..
        } = self.policy.gem_inventory.as_mut().unwrap()
        else {
            unreachable!()
        };
        &mut primary_dispositions[0]
    }
    /// A distinct later physical family has an ordinary usage consumer. Adding
    /// an earlier disposition must reuse its existing inventory issue/IDs.
    // This shared fixture is also included by action-only integration targets.
    #[allow(dead_code)]
    pub fn with_ordinary_usage() -> Self {
        let mut f = Self::new();
        let SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
            primary, output, ..
        } = &f.base.input
        else {
            unreachable!()
        };
        let primary = primary.clone();
        let output = output.clone();
        let gem = f
            .base
            .registry
            .allocate_definition::<GemDefinition>()
            .unwrap();
        let supply = f
            .base
            .registry
            .allocate_slot::<SkillGrantSlotDefinition>(SlotOwnerDefId::Gem(gem.clone()))
            .unwrap();
        let grant = f
            .base
            .registry
            .allocate_slot::<GrantSlotDefinition>(SlotOwnerDefId::Gem(gem.clone()))
            .unwrap();
        let policy = f
            .base
            .registry
            .allocate_definition::<UsagePolicyDefinition>()
            .unwrap();
        let parameter = f
            .base
            .registry
            .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::UsagePolicy(policy.clone()))
            .unwrap();
        let SchemaLookup::Known(physical) = f.base.schema.definition(&f.base.gem) else {
            unreachable!()
        };
        let mut physical = physical.clone();
        physical.declarations.parameters = DeclaredSet::complete(vec![]);
        physical.declarations.skill_grants = DeclaredSet::complete(vec![supply.clone()]);
        physical.declarations.grants = DeclaredSet::complete(vec![grant.clone()]);
        let mut schema = f.base.schema.input().clone();
        schema
            .definitions
            .push(DefinitionDescriptor::Gem(DefinitionEntry {
                id: gem.clone(),
                schema: SchemaState::Known(physical),
            }));
        schema
            .definitions
            .push(DefinitionDescriptor::UsagePolicy(DefinitionEntry {
                id: policy.clone(),
                schema: SchemaState::Known(UsagePolicySchema {
                    targets: vec![UsageTargetKind::Skill],
                    declarations: DeclaredSlots {
                        parameters: DeclaredSet::complete(vec![parameter.clone()]),
                        choices: DeclaredSet::complete(vec![]),
                        grants: DeclaredSet::complete(vec![]),
                        actors: DeclaredSet::complete(vec![]),
                        skill_grants: DeclaredSet::complete(vec![]),
                        outputs: DeclaredSet::complete(vec![]),
                        sockets: DeclaredSet::complete(vec![]),
                    },
                }),
            }));
        schema.slots.extend([
            SlotDescriptor::SkillGrant(DefinitionEntry {
                id: supply.clone(),
                schema: SchemaState::Known(SkillGrantSlotSchema {
                    skill: primary.clone(),
                    outputs: DeclaredSet::complete(vec![output]),
                }),
            }),
            SlotDescriptor::Grant(DefinitionEntry {
                id: grant.clone(),
                schema: SchemaState::Known(GrantSlotSchema {
                    provider_roles: vec![ProviderRole::SkillUse],
                    target: GrantTarget::Skill(supply.clone()),
                }),
            }),
            SlotDescriptor::Parameter(DefinitionEntry {
                id: parameter.clone(),
                schema: SchemaState::Known(ParameterSlotSchema {
                    value: ValueSchema::Boolean,
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::UsagePolicyParameter],
                    skill_input: None,
                }),
            }),
        ]);
        f.base.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
        let mut mapping = f.base.mapping.input().clone();
        mapping.definitions = f.base.schema.identity().clone();
        mapping.registry = f.base.registry.identity().unwrap();
        mapping.entries.push(MappingEntry {
            source: ExternalSelector::Definition(ExternalOwnerSelector::Gem {
                game_id: SourceComponent::Text("ordinary".into()),
                variant_id: SourceComponent::Text("variant".into()),
            }),
            outcome: MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Gem(gem.clone())),
                basis: MappingBasis::Exact,
            },
        });
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
        roles.roles.push(OwnedGemRoleRow {
            gem: gem.clone(),
            primary: OwnedPrimarySkill::Known(primary.clone()),
            role: OwnedGemRole::Known(AuthoredGemRole::SkillUse),
            materialization: OwnedGemMaterialization::Physical,
        });
        roles.compilation.gem_count = roles.roles.len();
        f.base.roles =
            OwnedSkillRoleIndex::new(roles, &f.base.mapping, &f.base.schema, Default::default())
                .unwrap();
        let definitions = f.base.schema.identity().clone();
        let roles = *f.base.roles.identity();
        let GemQualityPolicy::Attributes(quality) = &mut f.policy.gem_quality else {
            unreachable!()
        };
        quality.definitions = definitions.clone();
        f.policy.gem_inputs.as_mut().unwrap().definitions = definitions.clone();
        f.policy.usage_inputs = Some(UsageInputPolicy::PobPhysicalPrimarySkillV1 {
            definitions: definitions.clone(),
            roles,
            catalog: f.base.roles.input().compilation.catalog_digest,
            scalar_inputs: gem_inventory_scalar_inputs_identity(&f.policy, Default::default())
                .unwrap(),
            gems: vec![PrimarySkillUsageInput {
                gem,
                game_id: "ordinary".into(),
                variant_id: "variant".into(),
                skill_id: "effect".into(),
                name_spec: "Fixture skill".into(),
                primary,
                supply,
                grant,
                policy,
                attributes: [
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
                .collect(),
                guards: vec![],
                parameters: vec![GemParameterInput {
                    slot: parameter,
                    value: boolean("requested", "enableGlobal1"),
                }],
            }],
        });
        let scalar = gem_inventory_scalar_inputs_identity(&f.policy, Default::default()).unwrap();
        let usage = usage_inputs_identity(&f.policy, Default::default()).unwrap();
        let GemInventoryPolicy::PobFreshPhysicalV3 {
            definitions: d,
            roles: r,
            scalar_inputs,
            usage_inputs,
            primary_dispositions,
            ..
        } = f.policy.gem_inventory.as_mut().unwrap()
        else {
            unreachable!()
        };
        *d = definitions.clone();
        *r = roles;
        *scalar_inputs = scalar;
        *usage_inputs = usage;
        let SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
            definitions: d,
            roles: r,
            ..
        } = &mut primary_dispositions[0].reference_action
        else {
            unreachable!()
        };
        *d = definitions.clone();
        *r = roles;
        let mut rewards = f.rewards.input().clone();
        rewards.definitions = definitions;
        rewards.mapping = *f.base.mapping.identity();
        f.rewards =
            OwnedRewardPolicy::new(rewards, &f.base.mapping, &f.base.schema, Default::default())
                .unwrap();
        f
    }
    pub fn run(
        &self,
        text: &str,
        limits: NormalizationLimits,
    ) -> Result<NormalizedImport, NormalizationError> {
        let source = ImportedBuildInstance::from_decoded(
            decode_build(text.as_bytes()).unwrap(),
            BuildLineage::from_bytes([71; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
        normalize_fresh(
            &evidence,
            *source.allocator_state(),
            NormalizationArtifacts {
                mappings: &self.base.mapping,
                registry: &self.base.registry,
                definitions: &self.base.schema,
                roles: &self.base.roles,
                rewards: &self.rewards,
                items: &items::empty_items(&self.base.schema),
                item_source: &items::empty_item_source(&self.base.schema),
                tree: None,
            },
            &self.policy,
            &[],
            limits,
        )
    }
}
