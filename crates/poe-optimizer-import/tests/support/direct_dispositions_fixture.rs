//! One normalization fixture shared by Direct input and query tests.
use crate::{direct_minions, inventory};
use poe_optimizer_import::{
    owned_mapping::SourceComponent, owned_normalize::*, owned_reward_policy::OwnedRewardPolicy,
    owned_source_actions::*,
};

pub struct Fixture {
    pub base: inventory::Fixture,
    pub reference: SourceActionCorrespondenceInput,
}
impl Fixture {
    pub fn new() -> Self {
        let physical = crate::minions::Fixture::new();
        let mut base = inventory::Fixture::from_base(physical.base);
        let direct = direct_minions::Fixture::from_base(base.base, physical.input);
        base.base = direct.base;
        let mut reference = direct.input;
        let deferred_usage = base.row().deferred_usage.clone();
        base.policy.gem_inventory = None;
        base.policy.gem_inputs = None;
        let GemQualityPolicy::Attributes(quality) = &mut base.policy.gem_quality else {
            unreachable!()
        };
        quality.definitions = base.base.schema.identity().clone();
        let mut rewards = base.rewards.input().clone();
        rewards.definitions = base.base.schema.identity().clone();
        rewards.mapping = *base.base.mapping.identity();
        base.rewards = OwnedRewardPolicy::new(
            rewards,
            &base.base.mapping,
            &base.base.schema,
            Default::default(),
        )
        .unwrap();
        base.policy.skill_scopes = Some(SkillScopePolicy {
            slot_attribute: "slot".into(),
            shared_slots: vec![SourceComponent::Missing],
        });
        let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 {
            definitions,
            roles,
            catalog_gem,
            game_id,
            variant_id,
            skill_id,
            name_spec,
            skill,
            manual_sources,
            ..
        } = &mut reference
        else {
            unreachable!()
        };
        *definitions = base.base.schema.identity().clone();
        *roles = *base.base.roles.identity();
        base.policy.direct_skill_inputs = Some(DirectSkillInputPolicy::PobManualDirectSkillV2 {
            definitions: definitions.clone(),
            source: base.base.roles.input().compilation.source.clone(),
            roles: *roles,
            catalog: base.base.roles.input().compilation.catalog_digest,
            manual_sources: manual_sources.clone(),
            group_attributes: [
                "source",
                "enabled",
                "slot",
                "mainActiveSkill",
                "mainActiveSkillCalcs",
                "groupCount",
                "includeInFullDPS",
                "label",
            ]
            .map(str::to_owned)
            .to_vec(),
            skills: vec![DirectSkillInputRule {
                gem: catalog_gem.clone(),
                game_id: game_id.clone(),
                variant_id: variant_id.clone(),
                skill_id: skill_id.clone(),
                name_spec: name_spec.clone(),
                skill: skill.clone(),
                attributes: [
                    "gemId",
                    "variantId",
                    "skillId",
                    "nameSpec",
                    "level",
                    "quality",
                    "enabled",
                    "corrupted",
                    "corruptLevel",
                    "count",
                    "enableGlobal1",
                    "enableGlobal2",
                    "statSetIndex",
                    "statSetIndexCalcs",
                    "skillMinion",
                    "skillMinionCalcs",
                    "skillMinionSkill",
                    "skillMinionSkillCalcs",
                ]
                .map(str::to_owned)
                .to_vec(),
                guards: vec![],
                parameters: direct.parameters,
            }],
            dispositions: vec![DirectSkillInputDisposition {
                skill: skill.clone(),
                reference_action: reference.clone(),
                deferred_usage,
                inert_fields: vec![
                    GemInputGuard {
                        attribute: "corrupted".into(),
                        allowed: ["false", "nil"]
                            .map(|s| SourceComponent::Text(s.into()))
                            .to_vec(),
                    },
                    GemInputGuard {
                        attribute: "corruptLevel".into(),
                        allowed: vec![SourceComponent::Text("0".into())],
                    },
                ],
                group_guards: vec![GemInputGuard {
                    attribute: "label".into(),
                    allowed: vec![
                        SourceComponent::Missing,
                        SourceComponent::Text(String::new()),
                    ],
                }],
            }],
        });
        Self { base, reference }
    }
    pub fn row(&mut self) -> &mut DirectSkillInputDisposition {
        let DirectSkillInputPolicy::PobManualDirectSkillV2 { dispositions, .. } =
            self.base.policy.direct_skill_inputs.as_mut().unwrap()
        else {
            unreachable!()
        };
        &mut dispositions[0]
    }
}
