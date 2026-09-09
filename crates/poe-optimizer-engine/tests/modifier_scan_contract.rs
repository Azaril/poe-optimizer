use poe_optimizer_engine::lua_pattern::{MatchBudget, MatchLimits, PatternError, ResourceKind};
use poe_optimizer_engine::modifier_scan::{
    MAX_SCAN_ROWS, MAX_SCAN_TEXT_BYTES, ScanCapture, ScanError, ScanTable,
};

#[test]
fn ranking_replaces_stale_ties_and_preserves_case_and_first_five_captures() {
    let table =
        ScanTable::compile(["(a)", "[a]", "(a)(b)(c)(d)(e)(f)", "(a)(b)(c)(d)(e)(.)"]).unwrap();
    let found = table
        .scan(b"X ABCDEF Y", false, &mut MatchBudget::default())
        .unwrap()
        .unwrap();
    assert_eq!(found.row_index, 2);
    assert_eq!(found.range, 2..8);
    assert_eq!(found.tied_rows, [3]);
    assert_eq!(found.remainder(), b"X  Y");
    assert_eq!(found.captures.len(), 5);
    assert_eq!(found.captures[0], ScanCapture::Bytes(b"a".to_vec()));
    assert_eq!(found.captures[4], ScanCapture::Bytes(b"e".to_vec()));
}
#[test]
fn position_capture_and_empty_match_preserve_original_bytes() {
    let table = ScanTable::compile(["()$"]).unwrap();
    let found = table
        .scan(b"A\xff", false, &mut MatchBudget::default())
        .unwrap()
        .unwrap();
    assert_eq!(found.range, 2..2);
    assert_eq!(found.captures, [ScanCapture::Position(3)]);
    assert_eq!(found.remainder(), b"A\xff");
}
#[test]
fn later_source_error_is_not_hidden_by_an_earlier_winner() {
    let table = ScanTable::compile(["a", "%"]).unwrap();
    assert!(matches!(
        table.scan(b"a", false, &mut MatchBudget::default()),
        Err(ScanError::Pattern(PatternError::Source(_)))
    ));
}
#[test]
fn plain_scan_does_not_interpret_pattern_characters() {
    let table = ScanTable::compile(["%", "(.)"]).unwrap();
    let found = table
        .scan(b"A (.) B", true, &mut MatchBudget::default())
        .unwrap()
        .unwrap();
    assert_eq!(found.row_index, 1);
    assert_eq!(found.remainder(), b"A  B");
    assert!(found.captures.is_empty());
}
#[test]
fn resource_bounds_cover_empty_rows_and_input_without_a_match() {
    assert!(matches!(
        ScanTable::compile(std::iter::repeat_n("", MAX_SCAN_ROWS + 1)),
        Err(ScanError::ResourceBound("pattern rows"))
    ));
    let empty = ScanTable::compile(std::iter::empty::<&str>()).unwrap();
    assert!(matches!(
        empty.scan(
            &vec![b'x'; MAX_SCAN_TEXT_BYTES + 1],
            false,
            &mut MatchBudget::default()
        ),
        Err(ScanError::ResourceBound("input bytes"))
    ));
    let table = ScanTable::compile(["", "", ""]).unwrap();
    let mut budget = MatchBudget::new(MatchLimits {
        max_steps: 1,
        ..MatchLimits::default()
    });
    assert!(matches!(
        table.scan(b"", false, &mut budget),
        Err(ScanError::Pattern(PatternError::Resource(
            ResourceKind::MatchSteps
        )))
    ));
}
#[test]
fn immutable_patterns_share_across_threads_with_private_budgets() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<ScanTable>();
    let table = ScanTable::compile(["^(%d+)%% increased", "life"]).unwrap();
    std::thread::scope(|scope| {
        let table = &table;
        let workers: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(move || {
                    table
                        .scan(b"12% increased Life", false, &mut MatchBudget::default())
                        .unwrap()
                        .unwrap()
                        .row_index
                })
            })
            .collect();
        for worker in workers {
            assert_eq!(worker.join().unwrap(), 0);
        }
    });
}
