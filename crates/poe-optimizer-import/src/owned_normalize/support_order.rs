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
}

pub(super) fn initialize(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    policy: Option<&SupportOriginOrderPolicy>,
) -> Result<Option<DraftList<SupportOriginSequenceDraft>>> {
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
        let Some(order) = &mut preset.support_origins else {
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
            order.members.push(SupportOriginSequenceDraft {
                target: DraftSkillTarget::Authored((*target).into()),
                origins: Vec::<SupportOrigin>::new().into(),
            });
            self.sequences.insert(key, index);
            index
        };
        let DraftField::Known { value: origins } = &mut order.members[index].origins else {
            unreachable!("source-order collector only creates known local sequences")
        };
        b.charge(1)?;
        if origins.len() >= limit {
            return Err(NormalizationError::Limit("support origin sequence members"));
        }
        origins.push(SupportOrigin::Assignment(assignment));
        Ok(())
    }
}
