use poe_optimizer_core::{owned_build::DeclaredSlot, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance, owned_mapping::*,
    owned_normalize::ImportSkillUseLocator, owned_skill_catalog::*, owned_source_actions::*,
    owned_value::*, owned_value_policy::*,
};

pub fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
pub fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("source-action-test", "v1").unwrap()
}
fn pin() -> SourcePin {
    SourcePin {
        system: ExternalSourceSystem::PathOfBuilding2,
        revision: "c".repeat(40),
        files: vec![SourceFilePin {
            path: "fixture/catalog.json".into(),
            sha256: "a".repeat(64),
        }],
    }
}
fn declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
pub struct Fixture {
    pub input: SourceActionCorrespondenceInput,
    pub schema: OwnedDefinitionSchemaPackage,
    pub mapping: OwnedMappingIndex,
    pub roles: OwnedSkillRoleIndex,
    pub gem: GemDefId,
    pub grant: DeclaredSlot<GrantSlotDefId>,
    pub sets: [ActionStatSetDefId; 2],
}
impl Fixture {
    pub fn new() -> Self {
        let mut registry = OwnedIdRegistry::empty(ns(), Default::default()).unwrap();
        let base = registry.identity().unwrap();
        let gem = registry.allocate_definition::<GemDefinition>().unwrap();
        let primary = registry.allocate_definition::<SkillDefinition>().unwrap();
        let supply = registry
            .allocate_slot::<SkillGrantSlotDefinition>(SlotOwnerDefId::Gem(gem.clone()))
            .unwrap();
        let grant = registry
            .allocate_slot::<GrantSlotDefinition>(SlotOwnerDefId::Gem(gem.clone()))
            .unwrap();
        let output = registry
            .allocate_slot::<ActionOutputDefinition>(SlotOwnerDefId::Skill(primary.clone()))
            .unwrap();
        let part = registry
            .allocate_definition::<ActionPartDefinition>()
            .unwrap();
        let mode = registry
            .allocate_definition::<ActionModeDefinition>()
            .unwrap();
        let sets = [
            registry
                .allocate_definition::<ActionStatSetDefinition>()
                .unwrap(),
            registry
                .allocate_definition::<ActionStatSetDefinition>()
                .unwrap(),
        ];
        let mut gem_declarations = declarations();
        gem_declarations.grants.members.push(grant.clone());
        gem_declarations.skill_grants.members.push(supply.clone());
        // Identity/query correspondence does not imply whole input coverage.
        gem_declarations.parameters = DeclaredSet::partial(
            vec![],
            vec![SchemaGap {
                subject: SchemaSubject::Definition(DefinitionAddress::Gem(gem.clone())),
                facet: SchemaFacet::InputSchema,
                code: key("fixture-incomplete-inputs"),
            }],
        );
        let mut skill_declarations = declarations();
        skill_declarations.outputs.members.push(output.clone());
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
                namespace: ns(),
                release: key("fixture"),
                semantics_version: key("fixture-v1"),
                definitions: vec![
                    DefinitionDescriptor::Gem(entry(
                        gem.clone(),
                        GemSchema {
                            level: IntegerRange {
                                minimum: BoundedInteger::new(1).unwrap(),
                                maximum: BoundedInteger::new(30).unwrap(),
                            },
                            roles: vec![AuthoredGemRole::SkillUse],
                            skills: DeclaredSet::complete(vec![primary.clone()]),
                            quality: QualityUseSchema {
                                presence: QualityPresence::Forbidden,
                                allowed_kinds: DeclaredSet::complete(vec![]),
                            },
                            declarations: gem_declarations,
                        },
                    )),
                    DefinitionDescriptor::Skill(entry(
                        primary.clone(),
                        SkillSchema {
                            directly_selectable: false,
                            declarations: skill_declarations,
                        },
                    )),
                    DefinitionDescriptor::ActionPart(entry(part.clone(), ActionPartSchema {})),
                    DefinitionDescriptor::ActionMode(entry(mode.clone(), ActionModeSchema {})),
                    DefinitionDescriptor::ActionStatSet(entry(
                        sets[0].clone(),
                        ActionStatSetSchema {},
                    )),
                    DefinitionDescriptor::ActionStatSet(entry(
                        sets[1].clone(),
                        ActionStatSetSchema {},
                    )),
                ],
                slots: vec![
                    SlotDescriptor::SkillGrant(entry(
                        supply.clone(),
                        SkillGrantSlotSchema {
                            skill: primary.clone(),
                            outputs: DeclaredSet::complete(vec![output.clone()]),
                        },
                    )),
                    SlotDescriptor::Grant(entry(
                        grant.clone(),
                        GrantSlotSchema {
                            provider_roles: vec![ProviderRole::SkillUse],
                            target: GrantTarget::Skill(supply.clone()),
                        },
                    )),
                    SlotDescriptor::ActionOutput(entry(
                        output.clone(),
                        ActionOutputSchema {
                            actor_role: DeclaredActorRole::Player,
                            parts: DeclaredSet::complete(vec![part.clone()]),
                            modes: DeclaredSet::complete(vec![mode.clone()]),
                            stat_sets: DeclaredSet::complete(sets.to_vec()),
                            choices: DeclaredSet::complete(vec![]),
                        },
                    )),
                ],
            },
            Default::default(),
        )
        .unwrap();
        let mapping = OwnedMappingIndex::new(
            MappingPackageInput {
                schema_version: OWNED_MAPPING_PACKAGE_VERSION,
                namespace: ns(),
                registry: registry.identity().unwrap(),
                definitions: schema.identity().clone(),
                source: pin(),
                policy_version: key("fixture-v1"),
                entries: vec![
                    MappingEntry {
                        source: ExternalSelector::Definition(ExternalOwnerSelector::Gem {
                            game_id: SourceComponent::Text("physical".into()),
                            variant_id: SourceComponent::Text("variant".into()),
                        }),
                        outcome: MappingOutcome::Mapped {
                            target: SchemaSubject::Definition(DefinitionAddress::Gem(gem.clone())),
                            basis: MappingBasis::Exact,
                        },
                    },
                    MappingEntry {
                        source: ExternalSelector::Definition(ExternalOwnerSelector::Skill {
                            effect_id: SourceComponent::Text("effect".into()),
                        }),
                        outcome: MappingOutcome::Mapped {
                            target: SchemaSubject::Definition(DefinitionAddress::Skill(
                                primary.clone(),
                            )),
                            basis: MappingBasis::Exact,
                        },
                    },
                ],
            },
            &registry,
            &schema,
            Default::default(),
        )
        .unwrap();
        let roles = OwnedSkillRoleIndex::new(
            OwnedSkillRolePackageInput {
                schema_version: OWNED_SKILL_ROLE_VERSION,
                namespace: ns(),
                definitions: schema.identity().clone(),
                mapping: *mapping.identity(),
                compilation: SkillCatalogReceipt {
                    source: pin(),
                    catalog_digest: "b".repeat(64).parse().unwrap(),
                    policy: SkillCatalogPolicy {
                        version: key("fixture-v1"),
                        absent_support: AbsentSupportPolicy::Pending,
                        absent_from_tree: AbsentFromTreePolicy::Physical,
                    },
                    base_registry: base,
                    staged_registry: registry.identity().unwrap(),
                    gem_count: 1,
                    skill_count: 1,
                },
                roles: vec![OwnedGemRoleRow {
                    gem: gem.clone(),
                    primary: OwnedPrimarySkill::Known(primary.clone()),
                    role: OwnedGemRole::Known(AuthoredGemRole::SkillUse),
                    materialization: OwnedGemMaterialization::Physical,
                }],
            },
            &mapping,
            &schema,
            Default::default(),
        )
        .unwrap();
        let input = SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
            definitions: schema.identity().clone(),
            source: pin(),
            roles: *roles.identity(),
            catalog: roles.input().compilation.catalog_digest,
            gem: gem.clone(),
            game_id: "physical".into(),
            variant_id: "variant".into(),
            skill_id: "effect".into(),
            name_spec: "Fixture skill".into(),
            primary,
            primary_supply: supply,
            entering_grant: grant.clone(),
            output,
            part,
            mode,
            stat_sets: sets
                .iter()
                .enumerate()
                .map(|(index, id)| SourceStatSetMapping {
                    source_index: index as u32 + 1,
                    stat_set: id.clone(),
                })
                .collect(),
            absent_stat_set: Some(sets[0].clone()),
            index: ValueRecipeInput {
                id: key("fixture-index"),
                codec: ValueCodecInput {
                    namespace: ns(),
                    whitespace: WhitespacePolicy::Exact,
                    codec: ValueCodecKind::Integer {
                        syntax: DecimalSyntax::Scientific,
                    },
                },
                tiers: vec![ValueTier {
                    selectors: vec![ValueSelector {
                        lane: ValueLane::Attribute,
                        name: "index".into(),
                    }],
                    duplicates: DuplicatePolicy::Reject,
                }],
                missing: MissingValuePolicy::Pending,
                numeric_aliases: vec![],
            },
        };
        Self {
            input,
            schema,
            mapping,
            roles,
            gem,
            grant,
            sets,
        }
    }
    pub fn compile(
        &self,
        limits: SourceActionLimits,
    ) -> Result<SourceActionCorrespondence, SourceActionError> {
        SourceActionCorrespondence::new(
            self.input.clone(),
            &self.schema,
            &self.roles,
            &self.mapping,
            limits,
        )
    }
    pub fn request(
        &self,
        source: &ImportedBuildInstance,
        nth: usize,
        context: ImportReferenceContext,
    ) -> SourceActionRequest {
        SourceActionRequest {
            skill_use: ImportSkillUseLocator {
                source_sha256: source.source_sha256().into(),
                occurrence_ordinal: source
                    .occurrences()
                    .iter()
                    .filter(|row| row.name() == "Gem")
                    .nth(nth)
                    .unwrap()
                    .id()
                    .ordinal(),
                expected_gem: self.gem.clone(),
            },
            context,
        }
    }
}
pub fn xml(gems: &str) -> String {
    format!(
        "<PathOfBuilding2><Skills activeSkillSet=\"1\"><SkillSet id=\"1\"><Skill>{gems}</Skill></SkillSet></Skills></PathOfBuilding2>"
    )
}
pub fn gem(attributes: &str, children: &str) -> String {
    format!(
        "<Gem gemId=\"physical\" variantId=\"variant\" skillId=\"effect\" nameSpec=\"Fixture skill\"{attributes}>{children}</Gem>"
    )
}
pub fn maps(effect: &str, main: &str, calcs: &str) -> String {
    format!(
        "<StatSetIndex grantedEffect=\"{effect}\" index=\"{main}\"/><StatSetCalcsIndex grantedEffect=\"{effect}\" index=\"{calcs}\"/>"
    )
}
