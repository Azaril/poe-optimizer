//! Final obligation integrity for the consumed normalization result.
//!
//! This verifies correspondence, not source meaning or completeness authority.
//! A source-only row may retain a non-obligation link (for example an explicitly
//! empty equipment slot's weapon loadout), but it cannot carry unresolved issues.
use super::*;

pub(super) fn validate(b: &mut Builder<'_, '_>, draft: &DraftSession) -> Result<()> {
    // Core owns the bounded traversal of every live draft obligation. Do not
    // reconstruct that traversal from serialized fields or selected presets.
    let validation = draft.validate_limits(b.limits.draft)?;
    audit(
        b.evidence.rows().iter().map(|row| row.occurrence().id()),
        &b.origins,
        &validation.issues,
        &mut b.work,
        b.limits.max_work,
    )
}

fn charge(work: &mut usize, amount: usize, maximum: usize) -> Result<()> {
    // This is the Builder's existing work counter, split from its borrowed
    // evidence/origins; no independent or resettable budget is introduced.
    *work = work
        .checked_add(amount)
        .filter(|value| *value <= maximum)
        .ok_or(NormalizationError::Limit("work"))?;
    Ok(())
}

fn audit(
    expected: impl ExactSizeIterator<Item = SourceOccurrenceId>,
    origins: &[SourceOwnedOrigin],
    issues: &[DraftIssue],
    work: &mut usize,
    maximum: usize,
) -> Result<()> {
    charge(work, 1, maximum)?;
    if expected.len() != origins.len() {
        return Err(NormalizationError::Policy("origin evidence row count"));
    }
    charge(work, issues.len(), maximum)?;
    let live: BTreeSet<_> = issues.iter().map(|issue| issue.id).collect();
    if live.len() != issues.len() {
        return Err(NormalizationError::Policy("origin duplicate live issue"));
    }
    let mut linked = BTreeSet::new();
    for (source, origin) in expected.zip(origins) {
        charge(work, 1, maximum)?;
        if origin.source != source {
            return Err(NormalizationError::Policy("origin evidence row identity"));
        }
        if matches!(origin.disposition, SourceDisposition::Contributes) && origin.links.is_empty() {
            return Err(NormalizationError::Policy(
                "origin contribution has no link",
            ));
        }
        for target in &origin.links {
            charge(work, 1, maximum)?;
            if let OwnedOriginTarget::Issue(issue) = target {
                if matches!(origin.disposition, SourceDisposition::SourceOnly(_)) {
                    return Err(NormalizationError::Policy("source-only origin has issue"));
                }
                charge(work, 2, maximum)?;
                if !live.contains(issue) {
                    return Err(NormalizationError::Policy("origin issue is not live"));
                }
                linked.insert(*issue);
            }
        }
    }
    for issue in &live {
        charge(work, 1, maximum)?;
        if !linked.contains(issue) {
            return Err(NormalizationError::Policy(
                "live issue has no source origin",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_instance::{ImportedBuildInstance, InstanceImportLimits};

    fn sources(xml: &str) -> Vec<SourceOccurrenceId> {
        ImportedBuildInstance::from_decoded(
            crate::decode_build(xml.as_bytes()).unwrap(),
            BuildLineage::from_bytes([1; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap()
        .occurrences()
        .iter()
        .map(|row| row.id())
        .collect()
    }

    fn instance(local: u64) -> InstanceId {
        InstanceId::from_parts(BuildLineage::from_bytes([1; 16]), local).unwrap()
    }

    fn issue(local: u64) -> DraftIssue {
        DraftIssue {
            id: DraftIssueId::from_instance_id(instance(local)),
            owner: None,
            path: "fixture.completion".into(),
            code: key("fixture-pending"),
        }
    }

    fn fixture() -> (
        Vec<SourceOccurrenceId>,
        Vec<SourceOwnedOrigin>,
        Vec<DraftIssue>,
    ) {
        let sources = sources("<PathOfBuilding2><Notes/></PathOfBuilding2>");
        let issues = vec![issue(1)];
        let origins = sources
            .iter()
            .map(|source| SourceOwnedOrigin {
                source: *source,
                disposition: SourceDisposition::Contributes,
                links: vec![OwnedOriginTarget::Issue(issues[0].id)],
            })
            .collect();
        (sources, origins, issues)
    }

    fn check(
        sources: &[SourceOccurrenceId],
        origins: &[SourceOwnedOrigin],
        issues: &[DraftIssue],
    ) -> Result<()> {
        audit(sources.iter().copied(), origins, issues, &mut 0, 1_000)
    }

    #[test]
    fn shared_live_fallback_links_and_nonissue_source_correspondence_are_valid() {
        let (sources, mut origins, issues) = fixture();
        check(&sources, &origins, &issues).unwrap();
        origins[1].disposition = SourceDisposition::SourceOnly(key("explicit-empty-equipment-use"));
        origins[1].links = vec![OwnedOriginTarget::WeaponLoadout {
            key: key("weapon-set-two"),
            id: WeaponLoadoutId::from_instance_id(instance(2)),
        }];
        check(&sources, &origins, &issues).unwrap();
        origins[1].links.clear();
        check(&sources, &origins, &issues).unwrap();
    }

    #[test]
    fn source_rows_require_exact_count_order_and_document_identity() {
        let (sources, origins, issues) = fixture();
        assert!(check(&sources, &origins[..1], &issues).is_err());
        assert!(check(&sources[..1], &origins, &issues).is_err());
        let mut changed = origins.clone();
        changed.swap(0, 1);
        assert!(check(&sources, &changed, &issues).is_err());
        changed = origins.clone();
        changed[1].source = changed[0].source;
        assert!(check(&sources, &changed, &issues).is_err());
        changed = origins.clone();
        changed[1].source = self::sources("<PathOfBuilding2><Notes>x</Notes></PathOfBuilding2>")[1];
        assert!(check(&sources, &changed, &issues).is_err());
    }

    #[test]
    fn unaccounted_and_source_only_pending_rows_are_rejected() {
        let (sources, mut origins, issues) = fixture();
        origins[1].links.clear();
        assert!(matches!(
            check(&sources, &origins, &issues),
            Err(NormalizationError::Policy(
                "origin contribution has no link"
            ))
        ));
        origins[1].disposition = SourceDisposition::SourceOnly(key("presentation"));
        origins[1]
            .links
            .push(OwnedOriginTarget::Issue(issues[0].id));
        assert!(matches!(
            check(&sources, &origins, &issues),
            Err(NormalizationError::Policy("source-only origin has issue"))
        ));
    }

    #[test]
    fn retiring_a_shared_issue_cannot_leave_another_source_link() {
        let (sources, mut origins, _) = fixture();
        origins[0].links = vec![OwnedOriginTarget::ChoicePreset(
            ChoicePresetId::from_instance_id(instance(2)),
        )];
        assert!(matches!(
            check(&sources, &origins, &[]),
            Err(NormalizationError::Policy("origin issue is not live"))
        ));
    }

    #[test]
    fn each_live_obligation_requires_a_link_not_just_another_linked_issue() {
        let (sources, origins, mut issues) = fixture();
        issues.push(issue(2));
        assert!(matches!(
            check(&sources, &origins, &issues),
            Err(NormalizationError::Policy(
                "live issue has no source origin"
            ))
        ));
    }

    #[test]
    fn audit_charges_shared_budget_before_indexes_and_joins() {
        let (sources, origins, issues) = fixture();
        let mut exact = 7;
        audit(
            sources.iter().copied(),
            &origins,
            &issues,
            &mut exact,
            1_000,
        )
        .unwrap();
        let mut work = 7;
        audit(sources.iter().copied(), &origins, &issues, &mut work, exact).unwrap();
        assert_eq!(work, exact);
        let mut work = 7;
        assert!(matches!(
            audit(
                sources.iter().copied(),
                &origins,
                &issues,
                &mut work,
                exact - 1
            ),
            Err(NormalizationError::Limit("work"))
        ));
        let mut work = usize::MAX;
        assert!(matches!(
            audit(
                sources.iter().copied(),
                &origins,
                &issues,
                &mut work,
                usize::MAX
            ),
            Err(NormalizationError::Limit("work"))
        ));
    }
}
