//! Finite source fields accounted for outside the intrinsic Gem assignment list.
//! Decoding deferred values proves syntax only; the actual preset obligation
//! remains Pending and no computed usage record is manufactured.
use super::*;
use crate::owned_source_actions::{
    ImportReferenceContext, SourceActionCorrespondence, SourceActionCorrespondenceInput,
    SourceActionLimits, SourceActionRequest, SourceActionSelection,
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeferredGemUsageInput {
    pub field: DeferredGemUsageField,
    pub value: ValueRecipeInput,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeferredGemUsageField {
    GemCount,
    GemGlobal1,
    GemGlobal2,
    GroupCount,
    GroupFullDps,
}
impl DeferredGemUsageField {
    fn source(self) -> (&'static str, bool) {
        match self {
            Self::GemCount => ("count", false),
            Self::GemGlobal1 => ("enableGlobal1", false),
            Self::GemGlobal2 => ("enableGlobal2", false),
            Self::GroupCount => ("groupCount", true),
            Self::GroupFullDps => ("includeInFullDPS", true),
        }
    }
    fn count(self) -> bool {
        matches!(self, Self::GemCount | Self::GroupCount)
    }
}

pub(super) struct CompiledDisposition<'p> {
    row: &'p PrimaryGemInputDisposition,
    identity: OwnedContentDigest,
    reference: SourceActionCorrespondence,
    deferred: Vec<(DeferredGemUsageField, ValueRecipe)>,
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
    let SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
        gem,
        game_id,
        variant_id,
        skill_id,
        name_spec,
        ..
    } = &row.reference_action;
    if gem != &row.physical.gem
        || game_id != &row.physical.game_id
        || variant_id != &row.physical.variant_id
        || skill_id != &row.physical.skill_id
        || name_spec != &row.physical.name_spec
    {
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
    if row.deferred_usage.len() != 5 {
        return invalid("gem disposition deferred field inventory");
    }
    let mut fields = BTreeSet::new();
    let mut deferred = Vec::new();
    for input in &row.deferred_usage {
        charge(&mut work, 1, limits)?;
        let (attribute, group) = input.field.source();
        let value = &input.value;
        if !fields.insert(input.field)
            || !direct(value, attribute)
            || value.codec.namespace != *definitions.namespace()
            || value.codec.whitespace != crate::owned_value::WhitespacePolicy::Exact
            || !value.numeric_aliases.is_empty()
            || value.missing
                != if group {
                    MissingValuePolicy::Absent
                } else {
                    MissingValuePolicy::Pending
                }
        {
            return invalid("gem disposition deferred source recipe");
        }
        if input.field.count() {
            let ValueCodecKind::Quantity { unit, scale, .. } = &value.codec.codec else {
                return invalid("gem disposition count codec");
            };
            let SchemaLookup::Known(unit) = definitions.definition(unit) else {
                return invalid("gem disposition count unit");
            };
            if unit.dimension != UnitDimension::Count
                || scale.numerator.get() != 1
                || scale.denominator.get() != 1
            {
                return invalid("gem disposition count unit or scale");
            }
        } else if !matches!(value.codec.codec, ValueCodecKind::Boolean { .. }) {
            return invalid("gem disposition Boolean codec");
        }
        deferred.push((input.field, ValueRecipe::new(value.clone(), limits.value)?));
    }
    Ok(CompiledDisposition {
        row,
        identity,
        reference,
        deferred,
        work,
    })
}

impl CompiledDisposition<'_> {
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
        // The base inventory proves the intrinsic fields and finite row/group
        // grammar. Group main-action preferences have not gained a converter.
        for name in ["mainActiveSkill", "mainActiveSkillCalcs"] {
            b.charge(group.attributes().len())?;
            if !matches!(source_shape::value(group, name), None | Some("nil" | "1")) {
                return Ok(None);
            }
        }
        for (field, recipe) in &self.deferred {
            let (attribute, from_group) = field.source();
            let selected = if from_group { group } else { row };
            b.charge(selected.attributes().len().saturating_add(1))?;
            if let Some(value) = selected.attribute(attribute) {
                b.charge(value.raw().len())?;
            }
            match b.scalar_value(selected, recipe)? {
                ScalarValue::Selected(ParameterValue::Quantity(_)) if field.count() => {}
                ScalarValue::Selected(ParameterValue::Boolean(_)) if !field.count() => {}
                ScalarValue::Absent if from_group => {}
                _ => return Ok(None),
            }
        }
        let mut accounted_children = BTreeSet::new();
        let mut accounted_legacy = BTreeSet::new();
        for context in [ImportReferenceContext::Main, ImportReferenceContext::Calcs] {
            let report = self.reference.resolve(
                b.evidence,
                &SourceActionRequest {
                    skill_use: ImportSkillUseLocator {
                        source_sha256: b.evidence.identity().source_sha256.into(),
                        occurrence_ordinal: row.occurrence().id().ordinal(),
                        expected_gem: self.row.physical.gem.clone(),
                    },
                    context,
                },
            )?;
            // Construction is charged once in CompiledDisposition::work. Every
            // new traversal/codec/report operation is charged to the same import.
            b.charge(
                report
                    .work
                    .checked_sub(self.reference.construction_work())
                    .ok_or(NormalizationError::Policy("gem disposition reference work"))?,
            )?;
            if !matches!(report.target, ImportQueryTarget::Action(_)) {
                return Ok(None);
            }
            match report.selection {
                SourceActionSelection::Explicit { attribute, .. } => {
                    accounted_children.insert(attribute.occurrence);
                }
                SourceActionSelection::Absent => {}
                SourceActionSelection::Pending { .. } => return Ok(None),
            }
            accounted_legacy.extend(report.ignored_legacy_attributes);
        }
        b.charge(row.children().len().saturating_add(row.attributes().len()))?;
        if row.children().len() != accounted_children.len()
            || row
                .children()
                .iter()
                .any(|id| !accounted_children.contains(id))
        {
            return Ok(None);
        }
        for (index, attribute) in row.attributes().iter().enumerate() {
            if matches!(
                attribute.origin().name.as_str(),
                "statSetIndex" | "statSetIndexCalcs"
            ) && !accounted_legacy.contains(&crate::owned_source::SourceAttributeRef {
                occurrence: row.occurrence().id(),
                index: index as u32,
            }) {
                return Ok(None);
            }
        }
        Ok(Some(PendingDisposition {
            source: row.occurrence().id(),
            group: group.occurrence().id(),
            gem: self.row.physical.gem.clone(),
            identity: self.identity,
            reference_children: accounted_children.into_iter().collect(),
        }))
    }
}

fn unique_link<T: Copy>(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    select: impl Fn(&OwnedOriginTarget) -> Option<T>,
) -> Result<Option<T>> {
    b.charge(b.origins[source.ordinal() as usize].links.len())?;
    let mut links = b.origins[source.ordinal() as usize]
        .links
        .iter()
        .filter_map(select);
    let first = links.next();
    Ok(if links.next().is_none() { first } else { None })
}
fn link_once(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    target: OwnedOriginTarget,
) -> Result<()> {
    b.charge(b.origins[source.ordinal() as usize].links.len())?;
    if !b.origins[source.ordinal() as usize].links.contains(&target) {
        b.link(source, target)?;
    }
    Ok(())
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
        let Some(set) = b.ancestor(self.source, "SkillSet")? else {
            return Ok(None);
        };
        if b.evidence.rows()[self.source.ordinal() as usize]
            .occurrence()
            .parent()
            != Some(self.group)
            || b.evidence.rows()[self.group.ordinal() as usize]
                .occurrence()
                .parent()
                != Some(set)
            || unique_link(b, set, |link| {
                if let OwnedOriginTarget::SkillPreset(id) = link {
                    Some(*id)
                } else {
                    None
                }
            })? != Some(preset.id)
        {
            return Ok(None);
        }
        b.charge(preset.skills.members.len())?;
        if preset
            .skills
            .members
            .iter()
            .filter(|id| **id == skill.id)
            .count()
            != 1
        {
            return Ok(None);
        }
        if preset.usage_preferences.is_none() {
            preset.usage_preferences =
                Some(b.closure(self.source, "usage-preferences-not-converted", vec![])?);
        }
        let DraftListCompletion::Pending { id: issue, code } =
            &preset.usage_preferences.as_ref().unwrap().completion
        else {
            return Ok(None);
        };
        if code.as_str() != "usage-preferences-not-converted" {
            return Ok(None);
        }
        let issue = *issue;
        // A detached or other-preset Pending ID is not a retained obligation.
        // All matching links must remain in this exact source set. This also
        // permits repeated physical occurrences to share the preset inventory.
        let mut issue_sources = Vec::new();
        b.charge(b.origins.len())?;
        for index in 0..b.origins.len() {
            b.charge(b.origins[index].links.len())?;
            if b.origins[index]
                .links
                .contains(&OwnedOriginTarget::Issue(issue))
            {
                issue_sources.push(b.origins[index].source);
            }
        }
        if issue_sources.is_empty() {
            return Ok(None);
        }
        for source in issue_sources {
            if source != set && b.ancestor(source, "SkillSet")? != Some(set) {
                return Ok(None);
            }
        }
        for source in [self.source, self.group] {
            link_once(b, source, OwnedOriginTarget::SkillPreset(preset.id))?;
            link_once(b, source, OwnedOriginTarget::Issue(issue))?;
        }
        for source in &self.reference_children {
            link_once(b, *source, OwnedOriginTarget::Gem(*gem))?;
            link_once(b, *source, OwnedOriginTarget::Skill(skill.id))?;
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
        let imported=ImportedBuildInstance::from_decoded(decode_build(br#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1"><Skill><Gem/></Skill></SkillSet><SkillSet id="2"><Skill><Gem/></Skill></SkillSet></Skills></PathOfBuilding2>"#).unwrap(),BuildLineage::from_bytes([91;16]),Default::default()).unwrap();
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
}
