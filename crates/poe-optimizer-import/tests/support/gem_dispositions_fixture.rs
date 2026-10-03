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
