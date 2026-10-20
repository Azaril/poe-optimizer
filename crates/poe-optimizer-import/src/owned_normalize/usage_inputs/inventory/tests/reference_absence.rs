//! Consumer/transport controls use explicit private inspection reports; public
//! integration tests independently exercise the real reference producer.
use super::*;
use crate::owned_source_actions::{
    ImportReferenceContext, SourceActionInspection, SourceActionSelection, SourceMinionActionReport,
};

fn reference(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    minion: bool,
    fail_calcs: bool,
) -> Result<Option<skill_input_disposition::CheckedSourceReference>> {
    let row = &b.evidence.rows()[source.ordinal() as usize];
    skill_input_disposition::account_reference(
        b,
        row,
        |context| {
            Ok(SourceActionInspection {
                resolved: !(fail_calcs && matches!(context, ImportReferenceContext::Calcs)),
                selection: SourceActionSelection::Absent,
                ignored_legacy_attributes: vec![],
                minion: minion.then(|| SourceMinionActionReport {
                    actor_attributes: vec![],
                    action_selection: SourceActionSelection::Absent,
                    accounted_occurrences: vec![],
                }),
                work: 0,
            })
        },
        0,
    )
}

fn record(
    b: &mut Builder<'_, '_>,
    case: &mut Case,
    reference: &skill_input_disposition::CheckedSourceReference,
) -> Result<()> {
    let source = case.receipts.inputs[0].source;
    case.receipts.inputs.clear();
    record_input_disposition(
        b,
        source,
        &case.draft.skill_presets.members[0],
        &case.draft.skills.members[0],
        Some(&case.draft.gems.members[0]),
        AuthoredInputProof::ReferenceDisposition(reference),
        &mut case.receipts,
    )
}

#[test]
fn only_full_reference_reports_supply_their_exact_absent_fields() {
    for minion in [false, true] {
        with_case(SOURCE, |b, mut case| {
            let source = case.receipts.inputs[0].source;
            let proof = reference(b, source, minion, false).unwrap().unwrap();
            let expected: &[&str] = if minion {
                &[
                    "statSetIndex",
                    "statSetIndexCalcs",
                    "skillMinion",
                    "skillMinionCalcs",
                    "skillMinionSkill",
                    "skillMinionSkillCalcs",
                ]
            } else {
                &["statSetIndex", "statSetIndexCalcs"]
            };
            assert_eq!(proof.absent_fields().collect::<Vec<_>>(), expected);
            record(b, &mut case, &proof).unwrap();
            for name in expected {
                case.rows[0].fields.push(UsageInventoryField {
                    field: occurrence_field(name),
                    destinations: vec![UsageInventoryDestination::InputDisposition],
                });
            }
            case.rows[0].fields.push(UsageInventoryField {
                field: group_field("label"),
                destinations: vec![UsageInventoryDestination::InputDisposition],
            });
            finish(b, &mut case).unwrap();
            assert!(matches!(
                case.draft.skill_presets.members[0]
                    .intent
                    .as_ref()
                    .unwrap()
                    .usage
                    .completion,
                DraftListCompletion::Complete
            ));
        });
    }
}

#[test]
fn late_reference_failure_or_foreign_source_cannot_mint_absence_authority() {
    with_case(SOURCE, |b, mut case| {
        let source = case.receipts.inputs[0].source;
        let origins = b.origins.clone();
        assert!(reference(b, source, true, true).unwrap().is_none());
        assert_eq!(b.origins, origins);
        let other = b
            .evidence
            .rows()
            .iter()
            .rev()
            .find(|row| row.occurrence().name() == "Gem")
            .unwrap()
            .occurrence()
            .id();
        assert_ne!(other, source);
        let foreign = reference(b, other, true, false).unwrap().unwrap();
        record(b, &mut case, &foreign).unwrap();
        assert!(case.receipts.inputs.is_empty());
        let before = case.draft.clone();
        finish(b, &mut case).unwrap();
        assert_eq!(case.draft, before);
        assert_eq!(b.origins, origins);
    });
}

#[test]
fn absent_receipt_work_failure_preserves_outputs_links_and_allocator() {
    let mut cost = 0;
    with_case(SOURCE, |b, mut case| {
        let source = case.receipts.inputs[0].source;
        let proof = reference(b, source, true, false).unwrap().unwrap();
        let start = b.work;
        record(b, &mut case, &proof).unwrap();
        cost = b.work - start;
        assert_eq!(case.receipts.inputs.len(), 1);
    });
    with_case(SOURCE, |b, mut case| {
        let source = case.receipts.inputs[0].source;
        let proof = reference(b, source, true, false).unwrap().unwrap();
        let before = case.draft.clone();
        let origins = b.origins.clone();
        let allocator = b.allocator.state();
        b.limits.max_work = b.work + cost - 1;
        assert!(matches!(
            record(b, &mut case, &proof),
            Err(NormalizationError::Limit(_))
        ));
        assert!(case.receipts.inputs.is_empty());
        assert_eq!(case.draft, before);
        assert_eq!(b.origins, origins);
        assert_eq!(b.allocator.state(), allocator);
    });
}
