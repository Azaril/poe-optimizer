//! Real source additions preserve the published name ledger, even when sorted earlier.
use super::*;

#[test]
fn tracked_source_addition_and_replay_preserve_the_published_allocation_history() {
    let predecessor = canonical_checkpoint::checkpoint();
    let previous = predecessor.names();
    let saved = serde_json::to_vec(previous.input()).unwrap();
    let saved_registry = previous.registry().input().clone();
    let baseline = tracked_release();
    previous
        .validate_successor(baseline.name_allocation().names())
        .unwrap();

    const NAME: &str = "stat.a_checkpoint_probe";
    assert!(
        previous
            .input()
            .bindings
            .iter()
            .all(|binding| NAME < binding.name.as_str())
    );
    let mut changed = tracked_sources().clone();
    // The manifest has one declaration source. Extend it as an author would,
    // rather than introducing a second declaration component.
    const DECLARATIONS: &str = "declarations/mechanics.kdl";
    let mut declarations = changed.text(DECLARATIONS).to_owned();
    declarations.insert_str(
        declarations.rfind('}').unwrap(),
        &format!("    stat \"{NAME}\" type=\"boolean\" {{ target \"skill\"; }}\n"),
    );
    changed.replace_plain(DECLARATIONS, declarations);
    let expectation = format!(" expected_input=\"{}\"", baseline.release().receipt().input);
    assert_eq!(changed.manifest.matches(&expectation).count(), 1);
    changed.manifest = changed.manifest.replacen(&expectation, "", 1);
    let first = changed
        .build_with_allocation(tracked_allocation(), Default::default())
        .unwrap();
    let allocation = first.name_allocation();
    assert_eq!(allocation.predecessor(), Some(previous.identity().unwrap()));
    previous.validate_successor(allocation.names()).unwrap();
    let baseline_names = baseline.name_allocation().names();
    baseline_names
        .validate_successor(allocation.names())
        .unwrap();
    assert_eq!(
        allocation.names().input().bindings.len(),
        baseline_names.input().bindings.len() + 1
    );
    let added = allocation
        .names()
        .input()
        .bindings
        .iter()
        .find(|binding| binding.name == NAME)
        .unwrap();
    assert!(
        !previous
            .input()
            .bindings
            .iter()
            .any(|binding| binding.target == added.target)
    );
    let current = allocation.names().registry().input();
    assert_eq!(
        current.entries.len(),
        baseline_names.registry().input().entries.len() + 1,
        "exactly one new registry allocation"
    );
    assert_eq!(
        &current.entries[..saved_registry.entries.len()],
        saved_registry.entries.as_slice()
    );
    assert_eq!(
        current.entries.last().unwrap().target,
        added.target,
        "a lexically earlier declaration allocates after saved history"
    );
    let replay = changed
        .build_with_allocation(
            ReleaseAllocationBase::Predecessor {
                names: allocation.names(),
                expected_names: allocation.names().identity().unwrap(),
            },
            Default::default(),
        )
        .unwrap();
    assert_eq!(
        replay.release().receipt().input,
        first.release().receipt().input
    );
    assert_eq!(
        replay.name_allocation().names().input(),
        allocation.names().input()
    );
    assert_eq!(serde_json::to_vec(previous.input()).unwrap(), saved);
    assert_eq!(previous.registry().input(), &saved_registry);
}
