//! Target identity does not assert activation, numerical readiness or origin closure.
use super::{Fixture, inventory};
use crate::actions::key;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_draft::*, owned_schema::*};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    owned_mapping::*, owned_normalize::*, owned_reward_policy::OwnedRewardPolicy,
    owned_skill_catalog::*, owned_source_actions::SourceActionCorrespondenceInput,
};
use serde_json::{Value, json};

const SUPPORT: &str = r#"<Gem gemId="support" variantId="variant" skillId="support-effect" level="1" quality="0" enabled="true"/>"#;
const ACTIVE: &str = r#"<Gem gemId="other-active" variantId="variant" skillId="effect" level="17" quality="0" enabled="true"/>"#;

fn fixture() -> Fixture {
    let mut f = Fixture::new();
    let base = &mut f.base.base;
    let direct = base.roles.input().roles[0].primary.clone();
    let OwnedPrimarySkill::Known(primary) = &direct else {
        unreachable!()
    };
    let support_effect = base
        .registry
        .allocate_definition::<SkillDefinition>()
        .unwrap();
    let mut schema = base.schema.input().clone();
    schema
        .definitions
        .push(DefinitionDescriptor::Skill(DefinitionEntry {
            id: support_effect.clone(),
            schema: SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: SchemaSubject::Definition(DefinitionAddress::Skill(
                        support_effect.clone(),
                    )),
                    facet: SchemaFacet::GameRules,
                    code: key("support-mechanics-unconverted"),
                }],
            },
        }));
    let mut mapping = base.mapping.input().clone();
    let mut roles = base.roles.input().clone();
    for (source, role, effect) in [
        (
            "support",
            AuthoredGemRole::SupportAssignment,
            support_effect,
        ),
        ("other-active", AuthoredGemRole::SkillUse, primary.clone()),
    ] {
        let gem = base
            .registry
            .allocate_definition::<GemDefinition>()
            .unwrap();
        schema
            .definitions
            .push(DefinitionDescriptor::Gem(DefinitionEntry {
                id: gem.clone(),
                schema: SchemaState::Known(GemSchema {
                    level: IntegerRange {
                        minimum: BoundedInteger::new(1).unwrap(),
                        maximum: BoundedInteger::new(30).unwrap(),
                    },
                    roles: vec![role],
                    skills: DeclaredSet::complete(if role == AuthoredGemRole::SkillUse {
                        vec![effect.clone()]
                    } else {
                        vec![]
                    }),
                    quality: QualityUseSchema {
                        presence: QualityPresence::Forbidden,
                        allowed_kinds: DeclaredSet::complete(vec![]),
                    },
                    declarations: DeclaredSlots {
                        parameters: DeclaredSet::complete(vec![]),
                        choices: DeclaredSet::complete(vec![]),
                        grants: DeclaredSet::complete(vec![]),
                        actors: DeclaredSet::complete(vec![]),
                        skill_grants: DeclaredSet::complete(vec![]),
                        outputs: DeclaredSet::complete(vec![]),
                        sockets: DeclaredSet::complete(vec![]),
                    },
                }),
            }));
        mapping.entries.push(MappingEntry {
            source: ExternalSelector::Definition(ExternalOwnerSelector::Gem {
                game_id: SourceComponent::Text(source.into()),
                variant_id: SourceComponent::Text("variant".into()),
            }),
            outcome: MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Gem(gem.clone())),
                basis: MappingBasis::Exact,
            },
        });
        roles.roles.push(OwnedGemRoleRow {
            gem,
            primary: OwnedPrimarySkill::Known(effect),
            role: OwnedGemRole::Known(role),
            materialization: OwnedGemMaterialization::Physical,
        });
    }
    base.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    mapping.definitions = base.schema.identity().clone();
    mapping.registry = base.registry.identity().unwrap();
    base.mapping =
        OwnedMappingIndex::new(mapping, &base.registry, &base.schema, Default::default()).unwrap();
    roles.definitions = base.schema.identity().clone();
    roles.mapping = *base.mapping.identity();
    roles.compilation.staged_registry = base.registry.identity().unwrap();
    roles.compilation.gem_count += 2;
    roles.compilation.skill_count += 1;
    base.roles =
        OwnedSkillRoleIndex::new(roles, &base.mapping, &base.schema, Default::default()).unwrap();
    let GemQualityPolicy::Attributes(quality) = &mut f.base.policy.gem_quality else {
        unreachable!()
    };
    quality.definitions = base.schema.identity().clone();
    let DirectSkillInputPolicy::PobManualDirectSkillV2 {
        definitions,
        roles,
        dispositions,
        ..
    } = f.base.policy.direct_skill_inputs.as_mut().unwrap()
    else {
        unreachable!()
    };
    *definitions = base.schema.identity().clone();
    *roles = *base.roles.identity();
    for disposition in dispositions {
        let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 {
            definitions,
            roles,
            ..
        } = &mut disposition.reference_action
        else {
            unreachable!()
        };
        *definitions = base.schema.identity().clone();
        *roles = *base.roles.identity();
    }
    let mut rewards = f.base.rewards.input().clone();
    rewards.definitions = base.schema.identity().clone();
    rewards.mapping = *base.mapping.identity();
    f.base.rewards =
        OwnedRewardPolicy::new(rewards, &base.mapping, &base.schema, Default::default()).unwrap();
    f.base.policy.support_origin_order = Some(SupportOriginOrderPolicy::SavedManualGroupOrder {});
    f
}
fn enable(f: &mut Fixture) {
    f.base.policy.direct_support_targets =
        Some(DirectSupportTargetPolicy::PobManualSingleDirectRootV1 {
            direct_inputs: direct_skill_inputs_identity(
                f.base.policy.direct_skill_inputs.as_ref().unwrap(),
                Default::default(),
            )
            .unwrap(),
        });
}
fn run(f: &Fixture, xml: &str) -> NormalizedImport {
    f.base.run(xml, Default::default()).unwrap()
}

#[test]
fn exact_direct_targets_preserve_every_other_field_and_spent_issue_identity() {
    let mut f = fixture();
    let text = inventory::xml(&format!("{}{SUPPORT}{SUPPORT}", inventory::GEM));
    let old = run(&f, &text);
    enable(&mut f);
    let new = run(&f, &text);
    let before = old.draft().input();
    let after = new.draft().input();
    assert_eq!(after.supports.members.len(), 2);
    let target = DraftSkillTarget::Authored(after.skills.members[0].id.into());
    let mut restored = after.clone();
    let mut retired = vec![];
    for (old, new) in before
        .supports
        .members
        .iter()
        .zip(&mut restored.supports.members)
    {
        assert_eq!(new.target, target);
        let DraftSkillTarget::Pending(issue) = &old.target else {
            panic!("prior target")
        };
        retired.push(issue.id);
        new.target = old.target.clone();
    }
    for (old, new) in before
        .skill_presets
        .members
        .iter()
        .zip(&mut restored.skill_presets.members)
    {
        let order = new.support_origins.as_ref().unwrap();
        assert_eq!(
            order.completion,
            old.support_origins.as_ref().unwrap().completion
        );
        assert_eq!(order.members.len(), 1);
        assert_eq!(order.members[0].target, target);
        assert_eq!(
            order.members[0].origins.to_resolved(),
            Some(
                after
                    .supports
                    .members
                    .iter()
                    .map(|s| SupportOrigin::Assignment(s.id))
                    .collect()
            )
        );
        new.support_origins = old.support_origins.clone();
    }
    assert_eq!(&restored, before);
    for (a, b) in old.sidecar().origins.iter().zip(&new.sidecar().origins) {
        let mut expected = a.clone();
        expected
            .links
            .retain(|link| !matches!(link, OwnedOriginTarget::Issue(id) if retired.contains(id)));
        assert_eq!(&expected, b);
    }
    assert_eq!(new.sidecar().schema_version, 20);
    assert_eq!(old.sidecar().allocator_after, new.sidecar().allocator_after);
    assert!(
        after.skill_presets.members[0]
            .usage_preferences
            .as_ref()
            .unwrap()
            .to_resolved()
            .is_none()
    );
}

#[test]
fn repeated_and_dormant_sources_keep_independent_targets_and_disabled_state() {
    let mut f = fixture();
    enable(&mut f);
    let direct = inventory::GEM.replace("level=\"17\"", "level=\"bad\"");
    let disabled = SUPPORT.replace("enabled=\"true\"", "enabled=\"false\"");
    let text = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="2"><SkillSet id="1"><Skill enabled="false">{direct}{SUPPORT}</Skill></SkillSet><SkillSet id="2"><Skill enabled="true">{}{disabled}{SUPPORT}</Skill></SkillSet></Skills></PathOfBuilding2>"#,
        inventory::GEM
    );
    let result = run(&f, &text);
    let d = result.draft().input();
    assert_eq!(d.supports.members.len(), 3);
    assert_eq!(
        d.supports.members[0].target.to_resolved(),
        Some(SkillTarget::Authored(d.skills.members[0].id))
    );
    for row in &d.supports.members[1..] {
        assert_eq!(
            row.target.to_resolved(),
            Some(SkillTarget::Authored(d.skills.members[1].id))
        );
    }
    assert_eq!(d.supports.members[0].enabled.to_resolved(), Some(false));
    assert_eq!(d.supports.members[1].enabled.to_resolved(), Some(false));
    assert!(
        d.skills.members[0]
            .parameters
            .as_ref()
            .unwrap()
            .members
            .iter()
            .any(|p| p.value.to_resolved().is_none())
    );
    let repeated = run(&f, &text);
    assert_eq!(result.draft().input(), repeated.draft().input());
    assert_eq!(
        serde_json::to_value(result.sidecar()).unwrap(),
        serde_json::to_value(repeated.sidecar()).unwrap()
    );
    let _ = run(&f, &inventory::xml(&format!("{}{SUPPORT}", inventory::GEM)));
    let repeated = run(&f, &text);
    assert_eq!(result.draft().input(), repeated.draft().input());
    assert_eq!(
        serde_json::to_value(result.sidecar()).unwrap(),
        serde_json::to_value(repeated.sidecar()).unwrap()
    );
    for tree in ["", r#"<Tree><Spec id="1" nodes="123"/></Tree>"#] {
        let source = inventory::xml(&format!("{}{SUPPORT}", inventory::GEM))
            .replace("</PathOfBuilding2>", &format!("{tree}</PathOfBuilding2>"));
        let result = run(&f, &source);
        let d = result.draft().input();
        assert_eq!(
            d.supports.members[0].target.to_resolved(),
            Some(SkillTarget::Authored(d.skills.members[0].id))
        );
    }
}

#[test]
fn ambiguous_unreviewed_and_generated_groups_keep_targets_pending() {
    let mut f = fixture();
    enable(&mut f);
    for text in [
        inventory::xml(&format!("{}{}{SUPPORT}", inventory::GEM, inventory::GEM)),
        inventory::xml(&format!("{}{ACTIVE}{SUPPORT}", inventory::GEM)),
        inventory::xml(&format!("{}<Unknown/>{SUPPORT}", inventory::GEM)),
        inventory::xml(&format!(
            "{}<Gem gemId=\"unknown\"/>{SUPPORT}",
            inventory::GEM
        )),
        inventory::xml(&format!("{}{SUPPORT}", inventory::GEM))
            .replace("<Skill enabled", "<Skill source=\"Tree:1\" enabled"),
        inventory::xml(&format!("{}{SUPPORT}", inventory::GEM))
            .replace("<Skill enabled", "<Skill unreviewed=\"true\" enabled"),
    ] {
        let result = run(&f, &text);
        assert!(!result.draft().input().supports.members.is_empty());
        assert!(
            result
                .draft()
                .input()
                .supports
                .members
                .iter()
                .all(|s| matches!(s.target, DraftSkillTarget::Pending(_)))
        );
        assert_ne!(result.sidecar().schema_version, 20);
    }
    f.base.policy.single_active_support_target = false;
    assert!(matches!(
        run(&f, &inventory::xml(&format!("{}{SUPPORT}", inventory::GEM)))
            .draft()
            .input()
            .supports
            .members[0]
            .target,
        DraftSkillTarget::Pending(_)
    ));
}

#[test]
fn explicit_empty_source_preserves_legacy_raw_input_but_cannot_gain_manual_target() {
    let mut f = fixture();
    let text = inventory::xml(&format!("{}{SUPPORT}", inventory::GEM))
        .replace("<Skill enabled", "<Skill source=\"\" enabled");
    let old = run(&f, &text);
    assert_eq!(old.draft().input().skills.members.len(), 1);
    assert_eq!(old.draft().input().supports.members.len(), 1);
    assert!(matches!(
        old.draft().input().supports.members[0].target,
        DraftSkillTarget::Pending(_)
    ));
    enable(&mut f);
    let new = run(&f, &text);
    assert_eq!(new.draft().input(), old.draft().input());
    assert_eq!(new.sidecar().schema_version, old.sidecar().schema_version);
    assert_ne!(new.sidecar().schema_version, 20);
    let before = serde_json::to_value(old.sidecar()).unwrap();
    let mut after = serde_json::to_value(new.sidecar()).unwrap();
    assert_ne!(before["policy"], after["policy"]);
    after["policy"] = before["policy"].clone();
    assert_eq!(after, before, "only the opt-in policy commitment changes");
}

#[test]
fn omitted_policy_preserves_ordinary_targets_and_stale_authority_is_rejected() {
    let mut f = fixture();
    let text = inventory::xml(&format!("{ACTIVE}{SUPPORT}"));
    let old = run(&f, &text);
    enable(&mut f);
    let new = run(&f, &text);
    assert_eq!(old.draft().input(), new.draft().input());
    assert_eq!(old.sidecar().origins, new.sidecar().origins);
    assert_eq!(old.sidecar().schema_version, new.sidecar().schema_version);
    let mut wire = serde_json::to_value(&f.base.policy).unwrap();
    wire["direct_support_targets"] = Value::Null;
    assert!(serde_json::from_value::<NormalizationPolicy>(wire).is_err());
    let DirectSupportTargetPolicy::PobManualSingleDirectRootV1 { direct_inputs } =
        f.base.policy.direct_support_targets.as_mut().unwrap();
    *direct_inputs = "0".repeat(64).parse().unwrap();
    assert!(matches!(
        f.base.run(&text, Default::default()),
        Err(NormalizationError::Binding)
    ));
    f.base.policy.direct_support_targets = None;
    let wire = serde_json::to_value(&f.base.policy).unwrap();
    assert!(wire.get("direct_support_targets").is_none());
    let mut wire = wire;
    wire["direct_support_targets"] =
        json!({"kind":"pob_manual_single_direct_root_v1","direct_inputs":"0".repeat(64)});
    wire["direct_skill_inputs"]["kind"] = "pob_manual_direct_skill_v1".into();
    wire["direct_skill_inputs"]
        .as_object_mut()
        .unwrap()
        .remove("dispositions");
    f.base.policy = serde_json::from_value(wire).unwrap();
    assert!(f.base.run(&text, Default::default()).is_err());
}

#[test]
fn direct_target_work_and_link_limits_fail_without_partial_results() {
    let mut f = fixture();
    enable(&mut f);
    let text = inventory::xml(&format!("{}{SUPPORT}{SUPPORT}", inventory::GEM));
    let limits = NormalizationLimits {
        max_work: 1,
        ..NormalizationLimits::default()
    };
    assert!(matches!(
        f.base.run(&text, limits),
        Err(NormalizationError::Limit(_))
    ));
    let limits = NormalizationLimits {
        max_origin_links: 1,
        ..NormalizationLimits::default()
    };
    assert!(matches!(
        f.base.run(&text, limits),
        Err(NormalizationError::Limit(_))
    ));
}
