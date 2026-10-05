pub(crate) use crate::actions as player;
use player::key;
use poe_optimizer_core::{owned_build::DeclaredSlot, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{
    owned_mapping::*, owned_skill_catalog::*, owned_source_actions::*, owned_value_policy::*,
};

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
    pub base: player::Fixture,
    pub input: SourceActionCorrespondenceInput,
}
impl Fixture {
    pub fn new() -> Self {
        let mut base = player::Fixture::new();
        let SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
            primary,
            primary_supply,
            entering_grant,
            part,
            mode,
            index,
            ..
        } = base.input.clone()
        else {
            unreachable!()
        };
        let mut schema = base.schema.input().clone();
        let mut mapping = base.mapping.input().clone();
        let actor = base
            .registry
            .allocate_definition::<ActorDefinition>()
            .unwrap();
        let population = base
            .registry
            .allocate_slot::<ActorSlotDefinition>(SlotOwnerDefId::Skill(primary.clone()))
            .unwrap();
        let actor_grant = base
            .registry
            .allocate_slot::<GrantSlotDefinition>(SlotOwnerDefId::Skill(primary.clone()))
            .unwrap();
        let mut actor_declarations = declarations();
        let mut actions = Vec::new();
        for n in 1..=2 {
            let skill = base
                .registry
                .allocate_definition::<SkillDefinition>()
                .unwrap();
            let supply = base
                .registry
                .allocate_slot::<SkillGrantSlotDefinition>(SlotOwnerDefId::Actor(actor.clone()))
                .unwrap();
            let grant = base
                .registry
                .allocate_slot::<GrantSlotDefinition>(SlotOwnerDefId::Actor(actor.clone()))
                .unwrap();
            let output = base
                .registry
                .allocate_slot::<ActionOutputDefinition>(SlotOwnerDefId::Skill(skill.clone()))
                .unwrap();
            let mut child = declarations();
            child.outputs.members.push(output.clone());
            schema.definitions.push(DefinitionDescriptor::Skill(entry(
                skill.clone(),
                SkillSchema {
                    directly_selectable: false,
                    declarations: child,
                },
            )));
            actor_declarations.skill_grants.members.push(supply.clone());
            actor_declarations.grants.members.push(grant.clone());
            schema.slots.push(SlotDescriptor::SkillGrant(entry(
                supply.clone(),
                SkillGrantSlotSchema {
                    preset_inputs: None,
                    skill: skill.clone(),
                    outputs: DeclaredSet::complete(vec![output.clone()]),
                },
            )));
            schema.slots.push(SlotDescriptor::Grant(entry(
                grant.clone(),
                GrantSlotSchema {
                    provider_roles: vec![ProviderRole::SkillUse],
                    target: GrantTarget::Skill(supply.clone()),
                },
            )));
            schema.slots.push(SlotDescriptor::ActionOutput(entry(
                output.clone(),
                ActionOutputSchema {
                    actor_role: DeclaredActorRole::ProviderActor,
                    parts: DeclaredSet::complete(vec![part.clone()]),
                    modes: DeclaredSet::complete(vec![mode.clone()]),
                    stat_sets: DeclaredSet::complete(base.sets.to_vec()),
                    choices: DeclaredSet::complete(vec![]),
                },
            )));
            let skill_id = format!("child-{n}");
            mapping.entries.push(MappingEntry {
                source: ExternalSelector::Definition(ExternalOwnerSelector::Skill {
                    effect_id: SourceComponent::Text(skill_id.clone()),
                }),
                outcome: MappingOutcome::Mapped {
                    target: SchemaSubject::Definition(DefinitionAddress::Skill(skill.clone())),
                    basis: MappingBasis::Exact,
                },
            });
            actions.push(SourceMinionActionMapping {
                source_index: n,
                skill_id,
                skill,
                supply,
                entering_grant: grant,
                output,
                part: part.clone(),
                mode: mode.clone(),
                stat_sets: base
                    .sets
                    .iter()
                    .enumerate()
                    .map(|(n, id)| SourceStatSetMapping {
                        source_index: n as u32 + 1,
                        stat_set: id.clone(),
                    })
                    .collect(),
                absent_stat_set: Some(base.sets[0].clone()),
            });
        }
        schema.definitions.push(DefinitionDescriptor::Actor(entry(
            actor.clone(),
            ActorSchema {
                declarations: actor_declarations,
            },
        )));
        for descriptor in &mut schema.definitions {
            if let DefinitionDescriptor::Skill(row) = descriptor
                && row.id == primary
            {
                let SchemaState::Known(skill) = &mut row.schema else {
                    unreachable!()
                };
                skill.declarations.actors.members.push(population.clone());
                skill.declarations.grants.members.push(actor_grant.clone());
            }
        }
        schema.slots.push(SlotDescriptor::Actor(entry(
            population.clone(),
            ActorSlotSchema {
                skills: DeclaredSet::complete(actions.iter().map(|a| a.skill.clone()).collect()),
                outputs: DeclaredSet::complete(actions.iter().map(|a| a.output.clone()).collect()),
                provider_definition: Some(actor.clone()),
            },
        )));
        schema.slots.push(SlotDescriptor::Grant(entry(
            actor_grant.clone(),
            GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Actor(population.clone()),
            },
        )));
        base.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
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
        let recipe = |name: &str, id: &str| {
            let mut recipe = index.clone();
            recipe.id = key(id);
            recipe.tiers[0].selectors = vec![ValueSelector {
                lane: ValueLane::Attribute,
                name: name.into(),
            }];
            recipe
        };
        let input = SourceActionCorrespondenceInput::PobPhysicalSingletonMinionActionsV1 {
            definitions: base.schema.identity().clone(),
            source: base.roles.input().compilation.source.clone(),
            roles: *base.roles.identity(),
            catalog: base.roles.input().compilation.catalog_digest,
            gem: base.gem.clone(),
            game_id: "physical".into(),
            variant_id: "variant".into(),
            skill_id: "effect".into(),
            name_spec: "Fixture skill".into(),
            primary,
            primary_supply,
            entering_grant,
            minion: SourceSingletonMinion {
                source_id: "fixture-minion".into(),
                allow_absent: true,
                actor,
                population,
                entering_grant: actor_grant,
            },
            actions,
            absent_action: Some(1),
            main_action_index: Box::new(recipe("skillMinionSkill", "main-action")),
            calcs_action_index: Box::new(recipe("skillMinionSkillCalcs", "calcs-action")),
            map_skill_index: Box::new(recipe("skillIndex", "map-action")),
            map_stat_set_index: recipe("statSetIndex", "map-stat-set"),
        };
        Self { base, input }
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
    pub fn actions(&self) -> &[SourceMinionActionMapping] {
        let SourceActionCorrespondenceInput::PobPhysicalSingletonMinionActionsV1 {
            actions, ..
        } = &self.input
        else {
            unreachable!()
        };
        actions
    }
    pub fn population(&self) -> &DeclaredSlot<ActorSlotDefId> {
        let SourceActionCorrespondenceInput::PobPhysicalSingletonMinionActionsV1 { minion, .. } =
            &self.input
        else {
            unreachable!()
        };
        &minion.population
    }
}
pub fn maps(main: &str, calcs: &str) -> String {
    format!(
        "<MinionSkillIndexLookup grantedEffect=\"effect\">{main}</MinionSkillIndexLookup><MinionSkillIndexLookupCalcs grantedEffect=\"effect\">{calcs}</MinionSkillIndexLookupCalcs>"
    )
}
pub fn entry_map(action: &str, stat: &str) -> String {
    format!("<MinionSkillIndexMap skillIndex=\"{action}\" statSetIndex=\"{stat}\"/>")
}
