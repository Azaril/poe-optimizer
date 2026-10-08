//! Local physical-assignment order is source evidence, not complete effect discovery.
use super::*;

/// Reviewed ordering syntax. This policy neither discovers generated/item origins
/// nor decides how cross-linked groups or additional granted effects are merged.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SupportOriginOrderPolicy {
    SavedManualGroupOrder {},
    /// Also census saved physical assignments using exact source/role bindings.
    /// This does not complete effect discovery or resolve assignment targets.
    SavedManualGroupOrderWithPhysicalInventoryV2 {
        mapping_source: OwnedContentDigest,
        roles: OwnedContentDigest,
    },
    /// Also admits exact source-reviewed skill IDs with no physical gem mapping
    /// or support role. These rows prove only zero physical assignments; they do
    /// not discover effects, providers, occurrence inputs or assignment targets.
    SavedManualGroupOrderWithNonphysicalSkillInventoryV3 {
        mapping_source: OwnedContentDigest,
        roles: OwnedContentDigest,
        nonphysical_skill_ids: Vec<String>,
    },
}

pub(super) fn initialize(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    policy: Option<&SupportOriginOrderPolicy>,
) -> Result<Option<DraftList<AuthoredSupportOrderDraft>>> {
    match policy {
        None => Ok(None),
        Some(_) => Ok(Some(b.closure(
            source,
            "support-origin-discovery-not-converted",
            vec![],
        )?)),
    }
}

#[derive(Default)]
pub(super) struct OrderIndex {
    sequences: BTreeMap<(SkillPresetId, SkillUseId), usize>,
}
impl OrderIndex {
    /// Called once per physical support in source encounter order. Allocation IDs
    /// index occurrences only; they never determine the sequence member order.
    pub(super) fn record(
        &mut self,
        b: &mut Builder<'_, '_>,
        preset: &mut SkillPresetDraft,
        assignment: SupportAssignmentId,
        target: &DraftSkillTarget,
    ) -> Result<()> {
        let Some(order) = &mut preset.authored_support_order else {
            return Ok(());
        };
        b.charge(1)?;
        // The caller's existing reviewed manual-group rule proves this exact
        // authored target. Unresolved/generated targets retain their obligations.
        let DraftSkillTarget::Authored(DraftField::Known { value: target }) = target else {
            return Ok(());
        };
        let limit = b.limits.draft.input.max_collection_entries;
        let key = (preset.id, *target);
        let index = if let Some(index) = self.sequences.get(&key) {
            *index
        } else {
            b.charge(1)?;
            if order.members.len() >= limit {
                return Err(NormalizationError::Limit("support origin sequences"));
            }
            let index = order.members.len();
            order.members.push(AuthoredSupportOrderDraft {
                target: DraftSkillTarget::Authored((*target).into()),
                assignments: Vec::<SupportAssignmentId>::new().into(),
            });
            self.sequences.insert(key, index);
            index
        };
        let DraftField::Known { value: origins } = &mut order.members[index].assignments else {
            unreachable!("source-order collector only creates known local sequences")
        };
        b.charge(1)?;
        if origins.len() >= limit {
            return Err(NormalizationError::Limit("support origin sequence members"));
        }
        origins.push(assignment);
        Ok(())
    }
}

/// Complete only saved assignments and their relationships. Runtime capabilities
/// are checked by the common native composed-request gate, never by this census.
pub(super) fn complete(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
    presets: &BTreeMap<SourceOccurrenceId, usize>,
    sets: Option<&[SourceOccurrenceId]>,
    generated: &generated_skill_inputs::InputAccounting,
) -> Result<()> {
    let Some(sets) = sets else { return Ok(()) };
    b.charge(
        draft
            .supports
            .members
            .len()
            .saturating_add(generated.resolved.len()),
    )?;
    let supports: BTreeMap<_, _> = draft
        .supports
        .members
        .iter()
        .map(|row| (row.id, row))
        .collect();
    let generated: BTreeMap<_, _> = generated
        .resolved
        .iter()
        .map(|row| (row.source, row))
        .collect();
    for source in sets {
        b.charge(1)?;
        let Some(index) = presets.get(source).copied() else {
            continue;
        };
        let preset = &mut draft.skill_presets.members[index];
        let Some(order) = &mut preset.authored_support_order else {
            continue;
        };
        let DraftListCompletion::Pending { id, code } = &order.completion else {
            continue;
        };
        if code.as_str() != "support-origin-discovery-not-converted"
            || !matches!(preset.supports.completion, DraftListCompletion::Complete)
            || !assignments_match(b, &preset.supports.members, &order.members, &supports)?
            || !source_relationships_known(b, *source, generated.get(source).copied())?
            || !exclusive_issue(b, *source, *id)?
        {
            continue;
        }
        source_shape::retire_membership(
            b,
            *source,
            &mut order.completion,
            "support-origin-discovery-not-converted",
        )?;
    }
    Ok(())
}

fn assignments_match(
    b: &mut Builder<'_, '_>,
    members: &[SupportAssignmentId],
    order: &[AuthoredSupportOrderDraft],
    supports: &BTreeMap<SupportAssignmentId, &SupportDraft>,
) -> Result<bool> {
    b.charge(members.len().saturating_add(order.len()))?;
    let mut expected: BTreeMap<SkillUseId, Vec<SupportAssignmentId>> = BTreeMap::new();
    // Physical census preserves source encounter order, including disabled and
    // duplicate-definition occurrences. It is not canonical record-ID order.
    for id in members {
        let Some(row) = supports.get(id) else {
            return Ok(false);
        };
        let DraftSkillTarget::Authored(DraftField::Known { value: target }) = &row.target else {
            return Ok(false);
        };
        expected.entry(*target).or_default().push(*id);
    }
    for row in order {
        let DraftSkillTarget::Authored(DraftField::Known { value: target }) = &row.target else {
            return Ok(false);
        };
        let DraftField::Known { value: assignments } = &row.assignments else {
            return Ok(false);
        };
        b.charge(assignments.len())?;
        if expected.remove(target).as_ref() != Some(assignments) {
            return Ok(false);
        }
    }
    Ok(expected.is_empty())
}

fn source_relationships_known(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    generated: Option<&generated_skill_sources::ResolvedPreset>,
) -> Result<bool> {
    let evidence = b.evidence;
    let set = &evidence.rows()[source.ordinal() as usize];
    b.charge(
        set.children()
            .len()
            .saturating_add(generated.map_or(0, |p| p.sources.len())),
    )?;
    let resolved: BTreeMap<_, _> = generated
        .into_iter()
        .flat_map(|p| &p.sources)
        .map(|row| (row.group, row.source))
        .collect();
    let mut generated_groups = 0usize;
    for group_id in set.children() {
        let group = &evidence.rows()[group_id.ordinal() as usize];
        b.charge(2)?;
        let attributes = &b.attributes[group_id.ordinal() as usize];
        // The full source census already checked duplicates, namespaces and
        // child frames. An explicit empty/nil source is not an absent source.
        if !attributes.contains_key("source") {
            // Saved slot bindings can share supports or change applicability.
            // No native authored relation represents them yet, even if empty,
            // disabled or sharing with no currently selected recipient.
            if attributes.contains_key("slot") {
                return Ok(false);
            }
        } else {
            // Reuse exact item/tree provider correspondence, independently of
            // quality or activation readiness. Archived syntax is not a match.
            if !generated.is_some_and(|plan| plan.complete && plan.archived.is_empty())
                || !matches!(resolved.get(group_id), Some(row) if group.children() == [*row])
            {
                return Ok(false);
            }
            generated_groups += 1;
        }
    }
    Ok(generated_groups == resolved.len())
}

fn exclusive_issue(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    issue: DraftIssueId,
) -> Result<bool> {
    b.charge(b.origins.len())?;
    let work = b
        .origins
        .iter()
        .fold(0usize, |sum, origin| sum.saturating_add(origin.links.len()));
    b.charge(work)?;
    let mut matches = 0;
    for origin in &b.origins {
        for link in &origin.links {
            if *link == OwnedOriginTarget::Issue(issue) {
                if origin.source != source {
                    return Ok(false);
                }
                matches += 1;
            }
        }
    }
    Ok(matches == 1)
}
