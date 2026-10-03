//! Direct counterpart of the shared synthetic actor topology. The retained
//! physical DTO in base.input is fixture tooling, never the Direct correspondence.
use crate::{actions, minions};
use poe_optimizer_core::{owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance, owned_mapping::*, owned_normalize::*,
    owned_skill_catalog::*, owned_source_actions::*, owned_value::*,
};

pub struct Fixture {
    pub base: actions::Fixture,
    pub input: SourceActionCorrespondenceInput,
    pub parameters: Vec<DirectSkillParameterInput>,
}
impl Fixture {
    pub fn new() -> Self {
        let original = minions::Fixture::new();
        Self::from_base(original.base, original.input)
    }
    pub fn from_base(
        mut base: actions::Fixture,
        physical_minion_input: SourceActionCorrespondenceInput,
    ) -> Self {
        let SourceActionCorrespondenceInput::PobPhysicalSingletonMinionActionsV1 {
            game_id,
            variant_id,
            skill_id,
            name_spec,
            primary: skill,
            minion,
            actions,
            absent_action,
            main_action_index,
            calcs_action_index,
            map_skill_index,
            map_stat_set_index,
            ..
        } = physical_minion_input
        else {
            unreachable!()
        };
        let mut schema = base.schema.input().clone();
        schema.schema_version = OWNED_SCHEMA_PACKAGE_V5;
        // Catalog-only identities have no physical Gem schema or physical
        // projection authority. Keep the allocated IDs as historical registry
        // entries, but remove the synthetic physical slots from this package.
        schema
            .slots
            .retain(|slot| slot.address().declaration() != &SlotOwnerDefId::Gem(base.gem.clone()));
        for descriptor in &mut schema.definitions {
            if let DefinitionDescriptor::Gem(row) = descriptor
                && row.id == base.gem
            {
                row.schema = SchemaState::Unmapped {
                    gaps: vec![SchemaGap {
                        subject: SchemaSubject::Definition(DefinitionAddress::Gem(
                            base.gem.clone(),
                        )),
                        facet: SchemaFacet::InputSchema,
                        code: actions::key("catalog-only-provider"),
                    }],
                };
            }
        }
        let mut parameters = Vec::new();
        for (name, dimension) in [
            ("level", UnitDimension::Count),
            ("quality", UnitDimension::PercentagePoints),
        ] {
            let slot = base
                .registry
                .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Skill(skill.clone()))
                .unwrap();
            let unit = base
                .registry
                .allocate_definition::<UnitDefinition>()
                .unwrap();
            schema
                .definitions
                .push(DefinitionDescriptor::Unit(DefinitionEntry {
                    id: unit.clone(),
                    schema: SchemaState::Known(UnitSchema { dimension }),
                }));
            schema
                .slots
                .push(SlotDescriptor::Parameter(DefinitionEntry {
                    id: slot.clone(),
                    schema: SchemaState::Known(ParameterSlotSchema {
                        value: ValueSchema::Quantity(QuantityRange {
                            minimum: FiniteQuantity::new(-1000.0, unit.clone()).unwrap(),
                            maximum: FiniteQuantity::new(1000.0, unit.clone()).unwrap(),
                        }),
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![ParameterSite::SkillParameter],
                        skill_input: Some(SkillInputAuthority::AuthoredOrProjected),
                    }),
                }));
            let mut value = map_stat_set_index.clone();
            value.id = actions::key(name);
            value.tiers[0].selectors[0].name = name.into();
            value.codec.codec = ValueCodecKind::Quantity {
                syntax: DecimalSyntax::Scientific,
                unit,
                scale: RationalScale {
                    numerator: BoundedInteger::new(1).unwrap(),
                    denominator: BoundedInteger::new(1).unwrap(),
                },
            };
            parameters.push(DirectSkillParameterInput { slot, value });
        }
        for descriptor in &mut schema.definitions {
            if let DefinitionDescriptor::Skill(row) = descriptor
                && row.id == skill
            {
                let SchemaState::Known(value) = &mut row.schema else {
                    unreachable!()
                };
                value.directly_selectable = true;
                value.declarations.parameters =
                    DeclaredSet::complete(parameters.iter().map(|p| p.slot.clone()).collect());
            }
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
        roles
            .roles
            .iter_mut()
            .find(|r| r.gem == base.gem)
            .unwrap()
            .materialization = OwnedGemMaterialization::ProviderOnly;
        base.roles =
            OwnedSkillRoleIndex::new(roles, &base.mapping, &base.schema, Default::default())
                .unwrap();
        let SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
            definitions,
            roles,
            ..
        } = &mut base.input
        else {
            unreachable!()
        };
        *definitions = base.schema.identity().clone();
        *roles = *base.roles.identity();
        let input = SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 {
            definitions: base.schema.identity().clone(),
            source: base.roles.input().compilation.source.clone(),
            roles: *base.roles.identity(),
            catalog: base.roles.input().compilation.catalog_digest,
            catalog_gem: base.gem.clone(),
            game_id,
            variant_id,
            skill_id,
            name_spec,
            skill,
            manual_sources: vec![
                SourceComponent::Missing,
                SourceComponent::Text(String::new()),
            ],
            minion,
            actions,
            absent_action,
            main_action_index,
            calcs_action_index,
            map_skill_index,
            map_stat_set_index,
        };
        Self {
            base,
            input,
            parameters,
        }
    }
    pub fn compile(
        &self,
        limits: SourceActionLimits,
    ) -> Result<SourceActionCorrespondence, SourceActionError> {
        SourceActionCorrespondence::new(
            self.input.clone(),
            &self.base.schema,
            &self.base.roles,
            &self.base.mapping,
            limits,
        )
    }
    pub fn skill(&self) -> &SkillDefId {
        let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 {
            skill, ..
        } = &self.input
        else {
            unreachable!()
        };
        skill
    }
    pub fn actions(&self) -> &[SourceMinionActionMapping] {
        let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 {
            actions,
            ..
        } = &self.input
        else {
            unreachable!()
        };
        actions
    }
    pub fn request(
        &self,
        source: &ImportedBuildInstance,
        nth: usize,
        context: ImportReferenceContext,
    ) -> SourceDirectActionRequest {
        SourceActionRequest {
            skill_use: ImportDirectSkillUseLocator {
                source_sha256: source.source_sha256().into(),
                occurrence_ordinal: source
                    .occurrences()
                    .iter()
                    .filter(|r| r.name() == "Gem")
                    .nth(nth)
                    .unwrap()
                    .id()
                    .ordinal(),
                catalog_gem: self.base.gem.clone(),
                expected_skill: self.skill().clone(),
            },
            context,
        }
    }
}
