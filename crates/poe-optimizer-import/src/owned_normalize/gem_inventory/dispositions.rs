//! Finite source fields accounted for outside the intrinsic Gem assignment list.
//! Decoding deferred values proves syntax only; the actual preset obligation
//! remains Pending and no computed usage record is manufactured.
use super::*;
use crate::owned_source_actions::{
    SourceActionCorrespondence, SourceActionCorrespondenceInput, SourceActionLimits,
    SourceActionRequest,
};

pub(super) const GROUP_ATTRIBUTES: &[&str] = &[
    "enabled",
    "source",
    "label",
    "mainActiveSkill",
    "mainActiveSkillCalcs",
    "includeInFullDPS",
    "groupCount",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimaryGemInputDisposition {
    pub physical: PhysicalGemInputInventory,
    pub reference_action: SourceActionCorrespondenceInput,
    pub deferred_usage: Vec<DeferredGemUsageInput>,
}
use super::super::skill_input_disposition::{
    CompiledDeferredUsage, account_reference, attach_pending_usage, unique_link,
};
pub use super::super::skill_input_disposition::{
    DeferredSourceUsageField as DeferredGemUsageField,
    DeferredSourceUsageInput as DeferredGemUsageInput,
};

pub(super) struct CompiledDisposition<'p> {
    row: &'p PrimaryGemInputDisposition,
    identity: OwnedContentDigest,
    reference: SourceActionCorrespondence,
    deferred: CompiledDeferredUsage,
    pub(super) work: usize,
}
pub(in crate::owned_normalize) struct PendingDisposition {
    source: SourceOccurrenceId,
    group: SourceOccurrenceId,
    gem: GemDefId,
    identity: OwnedContentDigest,
    reference_children: Vec<SourceOccurrenceId>,
}
pub(in crate::owned_normalize) struct ProvenDisposition {
    source: SourceOccurrenceId,
    group: SourceOccurrenceId,
    gem: GemDefId,
    identity: OwnedContentDigest,
}

pub(super) fn compile<'p, I: DefinitionSchemaIndex>(
    row: &'p PrimaryGemInputDisposition,
    definitions: &I,
    roles: &OwnedSkillRoleIndex,
    mappings: &OwnedMappingIndex,
    limits: NormalizationLimits,
) -> Result<CompiledDisposition<'p>> {
    let identity = digest_owned(
        "owned-physical-gem-disposition-v1",
        row,
        limits.max_policy_bytes.min(MAX_NORMALIZATION_POLICY_BYTES),
    )?;
    let mut work = 0;
    charge(
        &mut work,
        serde_json::to_vec(row)
            .map_err(|_| NormalizationError::Policy("gem disposition encoding"))?
            .len(),
        limits,
    )?;
    if !row.reference_action.matches_physical(
        &row.physical.gem,
        &row.physical.game_id,
        &row.physical.variant_id,
        &row.physical.skill_id,
        &row.physical.name_spec,
    ) {
        return invalid("gem disposition reference identity");
    }
    let hard = SourceActionLimits::default();
    let reference = SourceActionCorrespondence::new(
        row.reference_action.clone(),
        definitions,
        roles,
        mappings,
        SourceActionLimits {
            max_work: limits.max_work.min(hard.max_work),
            max_wire_bytes: limits.max_policy_bytes.min(hard.max_wire_bytes),
            max_output_bytes: limits.max_policy_bytes.min(hard.max_output_bytes),
            max_map_rows: limits
                .draft
                .input
                .max_collection_entries
                .min(hard.max_map_rows),
            max_stat_sets: limits
                .draft
                .input
                .max_collection_entries
                .min(hard.max_stat_sets),
            value: limits.value,
        },
    )?;
    charge(&mut work, reference.construction_work(), limits)?;
    let deferred = CompiledDeferredUsage::new(&row.deferred_usage, definitions, limits, &mut work)?;
    Ok(CompiledDisposition {
        row,
        identity,
        reference,
        deferred,
        work,
    })
}

impl CompiledDisposition<'_> {
    pub(super) fn is_minion(&self) -> bool {
        self.row.reference_action.is_minion()
    }

    pub(super) fn prove(
        &self,
        b: &mut Builder<'_, '_>,
        row: &SourceEvidenceRow<'_>,
        group: &SourceEvidenceRow<'_>,
    ) -> Result<Option<PendingDisposition>> {
        let Some(sets) = skill_source_census::container_sets(b)? else {
            return Ok(None);
        };
        let Some(set) = b.ancestor(row.occurrence().id(), "SkillSet")? else {
            return Ok(None);
        };
        b.charge(sets.len())?;
        if !sets.contains(&set) {
            return Ok(None);
        }
        if !self.deferred.prove(b, row, group)? {
            return Ok(None);
        }
        let evidence = b.evidence;
        let request = |context| SourceActionRequest {
            skill_use: ImportSkillUseLocator {
                source_sha256: evidence.identity().source_sha256.into(),
                occurrence_ordinal: row.occurrence().id().ordinal(),
                expected_gem: self.row.physical.gem.clone(),
            },
            context,
        };
        let Some(reference_children) = account_reference(
            b,
            row,
            |context| {
                Ok(self
                    .reference
                    .inspect_physical(evidence, &request(context))?)
            },
            self.reference.construction_work(),
        )?
        else {
            return Ok(None);
        };
        Ok(Some(PendingDisposition {
            source: row.occurrence().id(),
            group: group.occurrence().id(),
            gem: self.row.physical.gem.clone(),
            identity: self.identity,
            reference_children,
        }))
    }
}

impl PendingDisposition {
    pub(in crate::owned_normalize) fn attach(
        &self,
        b: &mut Builder<'_, '_>,
        skill: &SkillDraft,
        preset: &mut SkillPresetDraft,
    ) -> Result<Option<ProvenDisposition>> {
        let DraftAuthoredSkillSource::Gem(DraftField::Known { value: gem }) = &skill.source else {
            return Ok(None);
        };
        if skill.parameters.is_some()
            || unique_link(b, self.source, |link| {
                if let OwnedOriginTarget::Gem(id) = link {
                    Some(*id)
                } else {
                    None
                }
            })? != Some(*gem)
            || unique_link(b, self.source, |link| {
                if let OwnedOriginTarget::Skill(id) = link {
                    Some(*id)
                } else {
                    None
                }
            })? != Some(skill.id)
        {
            return Ok(None);
        }
        if !attach_pending_usage(
            b,
            self.source,
            self.group,
            &self.reference_children,
            skill,
            preset,
            Some(*gem),
        )? {
            return Ok(None);
        }
        Ok(Some(ProvenDisposition {
            source: self.source,
            group: self.group,
            gem: self.gem.clone(),
            identity: self.identity,
        }))
    }
    pub(in crate::owned_normalize) fn completed_by(
        &self,
        proof: Option<&ProvenDisposition>,
    ) -> bool {
        proof.is_some_and(|proof| {
            self.source == proof.source
                && self.group == proof.group
                && self.gem == proof.gem
                && self.identity == proof.identity
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_instance::ImportedBuildInstance, decode_build};
    use poe_optimizer_data::owned_schema::*;

    const SOURCE: &str = r#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1"><Skill><Gem/></Skill></SkillSet><SkillSet id="2"><Skill><Gem/></Skill></SkillSet></Skills></PathOfBuilding2>"#;

    // These are real private Builder/sidecar relationships. The empty checked
    // artifact set is sufficient because attach consumes only already-proven
    // source and instance bindings, not any game-specific schema inference.
    fn with_destination(
        test: impl FnOnce(
            &mut Builder<'_, '_>,
            PendingDisposition,
            SkillDraft,
            SkillPresetDraft,
            SourceOccurrenceId,
        ),
    ) {
        with_source_destination(SOURCE, test);
    }

    fn with_source_destination(
        source_xml: &str,
        test: impl FnOnce(
            &mut Builder<'_, '_>,
            PendingDisposition,
            SkillDraft,
            SkillPresetDraft,
            SourceOccurrenceId,
        ),
    ) {
        let ns = GameVersionNamespace::new("disposition-test", "v1").unwrap();
        let mut registry = OwnedIdRegistry::empty(ns.clone(), Default::default()).unwrap();
        let gem = registry.allocate_definition::<GemDefinition>().unwrap();
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
                namespace: ns.clone(),
                release: key("fixture"),
                semantics_version: key("fixture-v1"),
                definitions: vec![],
                slots: vec![],
            },
            Default::default(),
        )
        .unwrap();
        let pin = SourcePin {
            system: ExternalSourceSystem::PathOfBuilding2,
            revision: "c".repeat(40),
            files: vec![SourceFilePin {
                path: "fixture.json".into(),
                sha256: "a".repeat(64),
            }],
        };
        let mappings = OwnedMappingIndex::new(
            MappingPackageInput {
                schema_version: OWNED_MAPPING_PACKAGE_VERSION,
                namespace: ns.clone(),
                registry: registry.identity().unwrap(),
                definitions: schema.identity().clone(),
                source: pin.clone(),
                policy_version: key("fixture-v1"),
                entries: vec![],
            },
            &registry,
            &schema,
            Default::default(),
        )
        .unwrap();
        let roles = OwnedSkillRoleIndex::new(
            OwnedSkillRolePackageInput {
                schema_version: OWNED_SKILL_ROLE_VERSION,
                namespace: ns.clone(),
                definitions: schema.identity().clone(),
                mapping: *mappings.identity(),
                compilation: SkillCatalogReceipt {
                    source: pin.clone(),
                    catalog_digest: "b".repeat(64).parse().unwrap(),
                    policy: SkillCatalogPolicy {
                        version: key("fixture-v1"),
                        absent_support: AbsentSupportPolicy::Pending,
                        absent_from_tree: AbsentFromTreePolicy::Physical,
                    },
                    base_registry: registry.identity().unwrap(),
                    staged_registry: registry.identity().unwrap(),
                    gem_count: 0,
                    skill_count: 0,
                },
                roles: vec![],
            },
            &mappings,
            &schema,
            Default::default(),
        )
        .unwrap();
        let rewards = OwnedRewardPolicy::new(
            RewardPolicyInput {
                schema_version: OWNED_REWARD_POLICY_VERSION,
                namespace: ns.clone(),
                version: key("fixture-v1"),
                definitions: schema.identity().clone(),
                mapping: *mappings.identity(),
                rules: vec![],
            },
            &mappings,
            &schema,
            Default::default(),
        )
        .unwrap();
        let items = OwnedItemLinePolicy::new(
            ItemLinePolicyInput {
                schema_version: OWNED_ITEM_LINE_POLICY_VERSION,
                namespace: ns.clone(),
                version: key("fixture-v1"),
                definitions: schema.identity().clone(),
                whitespace: crate::owned_value::WhitespacePolicy::Exact,
                rules: vec![],
            },
            &schema,
            Default::default(),
        )
        .unwrap();
        let item_source = ItemSourceLayoutPolicy::new(
            ItemSourceLayoutPolicyInput {
                schema_version: OWNED_ITEM_SOURCE_POLICY_VERSION,
                namespace: ns,
                version: key("fixture-v1"),
                source: pin,
                item_lines: *items.identity(),
                dialect: ItemSourceDialect::PobExportedSingleTextV1,
                property_bindings: vec![],
                template_defaults: vec![],
                rule_layouts: vec![],
                template_layouts: vec![],
            },
            &items,
            &schema,
            Default::default(),
        )
        .unwrap();
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(source_xml.as_bytes()).unwrap(),
            BuildLineage::from_bytes([91; 16]),
            Default::default(),
        )
        .unwrap();
        let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
        let gems: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|r| r.occurrence().name() == "Gem")
            .map(|r| r.occurrence().id())
            .collect();
        let source = gems[0];
        let other_source = gems[1];
        let group = evidence.rows()[source.ordinal() as usize]
            .occurrence()
            .parent()
            .unwrap();
        let set = evidence.rows()[group.ordinal() as usize]
            .occurrence()
            .parent()
            .unwrap();
        let mut b = Builder {
            evidence: &evidence,
            mappings: &mappings,
            roles: &roles,
            rewards: &rewards,
            items: &items,
            item_source: &item_source,
            item_texts: vec![],
            item_range_origins_attached: false,
            allocator: InstanceAllocator::from_state(*imported.allocator_state()),
            limits: NormalizationLimits::default(),
            work: 0,
            links: 0,
            issues: 0,
            origins: evidence
                .rows()
                .iter()
                .map(|r| SourceOwnedOrigin {
                    source: r.occurrence().id(),
                    disposition: SourceDisposition::Contributes,
                    links: vec![],
                })
                .collect(),
            attributes: vec![],
            ordinary_items_source: None,
            fresh_configuration_sets: None,
            fresh_skill_containers: None,
            fresh_skill_sets: None,
        };
        let physical = b.id().unwrap();
        let skill_id = b.id().unwrap();
        let preset_id = b.id().unwrap();
        b.link(source, OwnedOriginTarget::Gem(physical)).unwrap();
        b.link(source, OwnedOriginTarget::Skill(skill_id)).unwrap();
        b.link(set, OwnedOriginTarget::SkillPreset(preset_id))
            .unwrap();
        let skill = SkillDraft {
            id: skill_id,
            source: DraftAuthoredSkillSource::Gem(DraftField::from(physical)),
            enabled: DraftField::from(true),
            scope: DraftField::from(LoadoutScope::Shared),
            parameters: None,
        };
        let preset = SkillPresetDraft {
            intent: None,
            id: preset_id,
            skills: vec![skill_id].into(),
            supports: vec![].into(),
            support_origins: None,
            payload_links: vec![].into(),
            usage_preferences: None,
        };
        let pending = PendingDisposition {
            source,
            group,
            gem,
            identity: "e".repeat(64).parse().unwrap(),
            reference_children: vec![],
        };
        test(&mut b, pending, skill, preset, other_source);
    }

    #[test]
    fn physical_disposition_attachment_requires_actual_preset_membership_and_live_pending() {
        with_destination(|b, pending, skill, mut preset, _| {
            let proof = pending.attach(b, &skill, &mut preset).unwrap().unwrap();
            assert!(pending.completed_by(Some(&proof)));
            assert!(!pending.completed_by(None));
            let before_links = b.links;
            let before_issues = b.issues;
            assert!(pending.attach(b, &skill, &mut preset).unwrap().is_some());
            assert_eq!(b.links, before_links);
            assert_eq!(b.issues, before_issues);
        });
        for case in 0..8 {
            with_destination(|b, pending, mut skill, mut preset, other_source| {
                match case {
                    0 => preset.id = b.id().unwrap(),
                    1 => preset.skills.members.clear(),
                    2 => preset.skills.members.push(skill.id),
                    3 => preset.usage_preferences = Some(complete(vec![])),
                    4 => {
                        preset.usage_preferences =
                            Some(b.closure(pending.source, "other-pending", vec![]).unwrap())
                    }
                    5 => {
                        preset.usage_preferences = Some(DraftList {
                            members: vec![],
                            completion: DraftListCompletion::Pending {
                                id: b.id().unwrap(),
                                code: key("usage-preferences-not-converted"),
                            },
                        })
                    }
                    6 => {
                        preset.usage_preferences = Some(
                            b.closure(other_source, "usage-preferences-not-converted", vec![])
                                .unwrap(),
                        )
                    }
                    7 => skill.parameters = Some(complete(vec![])),
                    _ => unreachable!(),
                }
                let before = preset.clone();
                let before_issues = b.issues;
                let before_links = b.links;
                assert!(
                    pending.attach(b, &skill, &mut preset).unwrap().is_none(),
                    "case {case}"
                );
                assert_eq!(preset, before);
                assert_eq!(b.issues, before_issues);
                assert_eq!(b.links, before_links);
            });
        }
    }

    #[test]
    fn generated_intent_deferral_requires_existing_attached_same_preset_pending() {
        use skill_input_disposition::pending_intent_usage;
        for case in 0..7 {
            with_destination(|b, pending, _, mut preset, other_source| {
                let set = b.ancestor(pending.source, "SkillSet").unwrap().unwrap();
                let usage = b
                    .closure(pending.source, "usage-preferences-not-converted", vec![])
                    .unwrap();
                preset.intent = Some(SkillPresetIntentDraftV1 {
                    schema_version: 1,
                    usage,
                    generated_inputs: complete(vec![]),
                });
                match case {
                    0 => {}
                    1 => preset.intent = None,
                    2 => preset.intent.as_mut().unwrap().usage = complete(vec![]),
                    3 => {
                        preset.intent.as_mut().unwrap().usage.completion =
                            DraftListCompletion::Pending {
                                id: b.id().unwrap(),
                                code: key("usage-preferences-not-converted"),
                            }
                    }
                    4 => {
                        preset.intent.as_mut().unwrap().usage = b
                            .closure(other_source, "usage-preferences-not-converted", vec![])
                            .unwrap()
                    }
                    5 => preset.id = b.id().unwrap(),
                    6 => {
                        preset.intent.as_mut().unwrap().usage = b
                            .closure(pending.source, "wrong-obligation", vec![])
                            .unwrap()
                    }
                    _ => unreachable!(),
                }
                let before = preset.clone();
                let state = (b.allocator.state(), b.issues, b.links, b.origins.clone());
                assert_eq!(
                    pending_intent_usage(b, set, &preset).unwrap().is_some(),
                    case == 0,
                    "case {case}"
                );
                assert_eq!(preset, before);
                assert_eq!(
                    (b.allocator.state(), b.issues, b.links, b.origins.clone()),
                    state,
                    "the proof never creates the owner or changes origin links"
                );
            });
        }
    }

    #[test]
    fn archived_generated_accounting_requires_three_existing_same_preset_obligations() {
        use skill_input_disposition::pending_generated_responsibilities;
        for owner in 0..3 {
            for case in 0..9 {
                with_destination(|b, pending, _, mut preset, other_source| {
                    let set = b.ancestor(pending.source, "SkillSet").unwrap().unwrap();
                    preset.intent = Some(SkillPresetIntentDraftV1 {
                        schema_version: 1,
                        usage: b
                            .closure(pending.source, "usage-preferences-not-converted", vec![])
                            .unwrap(),
                        generated_inputs: b
                            .closure(set, "generated-skill-inputs-not-converted", vec![])
                            .unwrap(),
                    });
                    preset.support_origins = Some(
                        b.closure(set, "support-origin-discovery-not-converted", vec![])
                            .unwrap(),
                    );
                    let expected = pending_generated_responsibilities(b, set, &preset)
                        .unwrap()
                        .unwrap();
                    let code = [
                        "generated-skill-inputs-not-converted",
                        "usage-preferences-not-converted",
                        "support-origin-discovery-not-converted",
                    ][owner];
                    let completion = match owner {
                        0 => &mut preset.intent.as_mut().unwrap().generated_inputs.completion,
                        1 => &mut preset.intent.as_mut().unwrap().usage.completion,
                        2 => &mut preset.support_origins.as_mut().unwrap().completion,
                        _ => unreachable!(),
                    };
                    match case {
                        0 => {}
                        1 => *completion = DraftListCompletion::Complete,
                        2 => {
                            *completion = DraftListCompletion::Pending {
                                id: b.id().unwrap(),
                                code: key(code),
                            }
                        }
                        3 => b
                            .link(other_source, OwnedOriginTarget::Issue(expected[owner]))
                            .unwrap(),
                        4 => {
                            *completion = DraftListCompletion::Pending {
                                id: expected[owner],
                                code: key("wrong-obligation"),
                            }
                        }
                        5 => {
                            if owner == 2 {
                                preset.support_origins = None
                            } else {
                                preset.intent = None
                            }
                        }
                        6 => preset.id = b.id().unwrap(),
                        7 => {
                            let foreign = b.id().unwrap();
                            b.link(set, OwnedOriginTarget::SkillPreset(foreign))
                                .unwrap();
                        }
                        8 => {
                            *completion = DraftListCompletion::Pending {
                                id: expected[(owner + 1) % 3],
                                code: key(code),
                            }
                        }
                        _ => unreachable!(),
                    }
                    let before = preset.clone();
                    let state = (b.allocator.state(), b.issues, b.links, b.origins.clone());
                    assert_eq!(
                        pending_generated_responsibilities(b, set, &preset).unwrap(),
                        (case == 0).then_some(expected),
                        "owner {owner}, case {case}"
                    );
                    assert_eq!(preset, before);
                    assert_eq!(
                        (b.allocator.state(), b.issues, b.links, b.origins.clone()),
                        state,
                        "checking retained owners never creates or alters them"
                    );
                });
            }
        }
    }

    #[test]
    fn immutable_skill_container_census_reuses_only_the_source_frame_and_charges_copies() {
        use skill_source_census::{container_sets, sets};
        let source = SOURCE.replace("<Gem/>", "<Gem><Reference/></Gem>");
        with_source_destination(&source, |b, _, _, _, _| {
            let before = b.work;
            let containers = container_sets(b).unwrap().unwrap();
            let first = b.work - before;
            assert_eq!(containers.len(), 2);
            assert!(first > 1 + containers.len());
            let after = b.work;
            assert_eq!(container_sets(b).unwrap(), Some(containers.clone()));
            assert_eq!(b.work - after, 1 + containers.len());

            // A container proof does not admit the nested Gem for the older
            // flat-row consumers, and their rejection cannot poison it.
            assert!(sets(b).unwrap().is_none());
            let after = b.work;
            assert_eq!(container_sets(b).unwrap(), Some(containers));
            assert_eq!(b.work - after, 3);
            let after = b.work;
            assert!(sets(b).unwrap().is_none());
            assert_eq!(b.work - after, 1);
        });
    }

    #[test]
    fn immutable_skill_container_census_reuses_rejections_without_accepting_aliases() {
        use skill_source_census::{container_sets, sets};
        for source in [
            SOURCE.replace("id=\"2\"", "id=\"1\""),
            SOURCE.replace("id=\"2\"", "id=\"02\""),
            SOURCE.replace("id=\"2\"", "id=\"2\" unknown=\"true\""),
            SOURCE.replace("activeSkillSet=\"1\"", "activeSkillSet=\"3\""),
        ] {
            with_source_destination(&source, |b, _, _, _, _| {
                let before = b.work;
                assert!(container_sets(b).unwrap().is_none());
                assert!(b.work - before > 1);
                let after = b.work;
                assert!(container_sets(b).unwrap().is_none());
                assert_eq!(b.work - after, 1);
                // The independently cached flat-row census must see the same
                // rejected ancestor frame, without rescanning its contents.
                let after = b.work;
                assert!(sets(b).unwrap().is_none());
                assert_eq!(b.work - after, 2);
            });
        }
    }

    #[test]
    fn immutable_skill_container_census_never_caches_work_errors_or_skips_copy_limits() {
        use skill_source_census::container_sets;
        with_destination(|b, _, _, _, _| {
            b.limits.max_work = b.work + 1;
            assert!(matches!(
                container_sets(b),
                Err(NormalizationError::Limit("work"))
            ));
            assert!(b.fresh_skill_containers.is_none());
        });
        with_destination(|b, _, _, _, _| {
            let expected = container_sets(b).unwrap().unwrap();
            let spent = b.work;
            b.limits.max_work = spent + expected.len();
            assert!(matches!(
                container_sets(b),
                Err(NormalizationError::Limit("work"))
            ));
            assert_eq!(b.work, spent + 1 + expected.len());
            assert_eq!(b.fresh_skill_containers, Some(Some(expected)));
        });
        // A fresh import has independent cache/budget state after either error.
        with_destination(|b, _, _, _, _| {
            assert_eq!(container_sets(b).unwrap().unwrap().len(), 2);
        });
    }
}
