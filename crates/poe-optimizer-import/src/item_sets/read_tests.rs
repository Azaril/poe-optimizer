use super::super::{
    FinishLoadInput, ItemActivationContext, ItemActivationProgress, ItemActivationRune, ItemNumber,
    ItemSetInput, ItemSetLimits, ItemSetPhase, SocketUrlInput,
};
use super::*;
use crate::item_loading::{DependencyResult, assembly::AssemblyErrorKind};
use crate::item_slot_validity::Value;
use poe_optimizer_data::item_assembly::ItemInventoryPolicy;
use std::sync::OnceLock;

fn policy() -> ItemInventoryPolicy {
    static POLICY: OnceLock<ItemInventoryPolicy> = OnceLock::new();
    let mut policy = POLICY
        .get_or_init(|| {
            poe_optimizer_data::game_data::bundled_snapshot()
                .unwrap()
                .item_assembly()
                .policy()
                .inventory
                .clone()
        })
        .clone();
    policy.layout.base_slots = vec!["Caller body".into()];
    policy.layout.swap.slot_pattern = "^never$".into();
    policy.layout.embedded.parent_slots.clear();
    policy.layout.rune_slots.clear();
    policy.layout.passive.nodes.ids.clear();
    policy.layout.passive.nodes.validity_nodes = Default::default();
    policy.defaults.default_set_title = "Caller default".into();
    policy
}
fn fresh() -> ItemSetState {
    ItemSetState::new(&policy(), ItemSetLimits::default()).unwrap()
}
fn add(state: &mut ItemSetState, key: Option<&str>, title: Option<&str>) {
    state
        .begin_item_set(ItemSetInput {
            id: key,
            title,
            ..Default::default()
        })
        .unwrap();
    state.finish_item_set().unwrap();
}
fn key(number: f64) -> NumericValue {
    NumericValue::new(number)
}

struct EmptyInventory;
impl ItemActivationContext for EmptyInventory {
    fn inventory_ids(&self) -> &[f64] {
        &[]
    }
    fn take_validation_steps(&mut self) -> u64 {
        0
    }
    fn node_writes_available(&mut self, _: &[(f64, f64)]) -> Result<bool> {
        panic!("fixture has no passive sockets")
    }
    fn valid_for_slot(&mut self, _: f64, _: &str, _: Value<'_>) -> Result<bool> {
        panic!("fixture has no inventory")
    }
    fn item_label(&mut self, _: f64) -> Result<String> {
        panic!("no inventory")
    }
    fn jewel_socket_count(&mut self, _: f64) -> Result<f64> {
        panic!("no inventory")
    }
    fn selection_dependencies(&mut self, _: &str) -> Result<Vec<String>> {
        Ok(vec![])
    }
    fn initial_rune(&mut self, _: &str) -> DependencyResult<ItemActivationRune> {
        panic!("fixture has no runes")
    }
    fn select_rune(
        &mut self,
        _: &str,
        _: Value<'_>,
        _: &ItemActivationRune,
    ) -> DependencyResult<ItemActivationRune> {
        panic!("fixture has no runes")
    }
    fn set_node_selection(&mut self, _: f64, _: f64, _: ItemNumber) -> DependencyResult<()> {
        panic!("fixture has no passive sockets")
    }
}
fn activate(state: &mut ItemSetState, requested: Option<&str>) {
    state
        .finish_load(FinishLoadInput {
            active_item_set: requested,
            ..Default::default()
        })
        .unwrap();
    assert!(matches!(
        state.continue_activation(&mut EmptyInventory).unwrap(),
        ItemActivationProgress::AwaitingSyncLoadouts
    ));
}

#[test]
fn constructor_reset_and_legacy_fallback_are_read_from_actual_live_roots() {
    let mut state = fresh();
    {
        let mut read = state.read_view(ItemSetReadLimits::default());
        assert!(read.is_singleton().unwrap());
        let active = read.active().unwrap().unwrap();
        let previous = read.previous().unwrap().unwrap();
        let first_key = read.ordered_key(1).unwrap().unwrap();
        let winner = read.winner(first_key).unwrap().unwrap();
        assert!(active.same_identity(&previous));
        assert!(active.same_identity(&winner));
        assert_eq!(read.title(&active).unwrap(), Some("Caller default"));
    }
    state.begin_load().unwrap();
    {
        let mut read = state.read_view(ItemSetReadLimits::default());
        assert_eq!(read.dense_order_len().unwrap(), 0);
        assert!(read.ordered_key(1).unwrap().is_none());
        assert!(read.winner(key(1.0)).unwrap().is_none());
        assert_eq!(read.active_key().unwrap().unwrap().value(), 0.0);
        assert!(
            read.active()
                .unwrap()
                .unwrap()
                .same_identity(&read.previous().unwrap().unwrap())
        );
    }
    activate(&mut state, None);
    let mut read = state.read_view(ItemSetReadLimits::default());
    assert!(read.is_singleton().unwrap());
    assert!(
        read.active()
            .unwrap()
            .unwrap()
            .same_identity(&read.previous().unwrap().unwrap())
    );
    assert_eq!(read.active_key().unwrap().unwrap().value(), 1.0);
}

#[test]
fn duplicate_numeric_winners_do_not_replace_detached_previous_row_identity() {
    let mut state = fresh();
    state.begin_load().unwrap();
    add(&mut state, Some("1"), Some("First"));
    add(&mut state, Some("1.0"), Some("Winner"));
    {
        let mut read = state.read_view(ItemSetReadLimits::default());
        assert_eq!(read.dense_order_len().unwrap(), 2);
        assert!(!read.is_singleton().unwrap());
        assert_eq!(read.ordered_key(1).unwrap().unwrap().value(), 1.0);
        assert_eq!(read.ordered_key(2).unwrap().unwrap().value(), 1.0);
        let winner = read.winner(key(1.0)).unwrap().unwrap();
        assert_eq!(read.title(&winner).unwrap(), Some("Winner"));
        assert!(!winner.same_identity(&read.active().unwrap().unwrap()));
    }
    activate(&mut state, Some("1"));
    let mut read = state.read_view(ItemSetReadLimits::default());
    let active = read.active().unwrap().unwrap();
    let previous = read.previous().unwrap().unwrap();
    assert!(active.same_identity(&read.winner(key(1.0)).unwrap().unwrap()));
    assert!(!active.same_identity(&previous));
    assert_eq!(read.title(&previous).unwrap(), Some("Caller default"));
    assert_eq!(read.title(&active).unwrap(), Some("Winner"));
}

#[test]
fn numeric_keys_keep_scalar_bits_and_generated_ids_use_injected_defaults() {
    let mut policy = policy();
    policy.defaults.first_set_id = 7.0;
    policy.defaults.set_id_increment = 2.0;
    let mut state = ItemSetState::new(&policy, ItemSetLimits::default()).unwrap();
    state.begin_load().unwrap();
    for (raw, title) in [
        ("0", "Zero"),
        ("-0", "Negative zero"),
        ("0.5", ""),
        ("inf", "Infinity"),
    ] {
        add(&mut state, Some(raw), Some(title));
    }
    add(&mut state, None, None);
    add(&mut state, None, None);
    let mut read = state.read_view(ItemSetReadLimits::default());
    let expected: [f64; 6] = [0.0, -0.0, 0.5, f64::INFINITY, 7.0, 9.0];
    assert_eq!(read.dense_order_len().unwrap(), expected.len());
    for (index, expected) in expected.into_iter().enumerate() {
        assert_eq!(
            read.ordered_key(index + 1)
                .unwrap()
                .unwrap()
                .value()
                .to_bits(),
            expected.to_bits()
        );
    }
    let zero = read.winner(key(0.0)).unwrap().unwrap();
    assert!(zero.same_identity(&read.winner(key(-0.0)).unwrap().unwrap()));
    assert_eq!(read.title(&zero).unwrap(), Some("Negative zero"));
    let fraction = read.winner(key(0.5)).unwrap().unwrap();
    assert_eq!(read.title(&fraction).unwrap(), Some(""));
    assert!(read.winner(key(f64::INFINITY)).unwrap().is_some());
    assert!(read.winner(key(f64::NAN)).unwrap().is_none());
    let generated = read.winner(key(9.0)).unwrap().unwrap();
    assert_eq!(read.title(&generated).unwrap(), Some("Caller default"));
    assert!(read.ordered_key(7).unwrap().is_none());
}

#[test]
fn borrowing_and_foreign_owner_checks_never_copy_or_mutate_producer_state() {
    let state = fresh();
    let other = fresh();
    let producer_before = serde_json::to_value(state.usage()).unwrap();
    let mut read = state.read_view(ItemSetReadLimits::default());
    let row = read.active().unwrap().unwrap();
    let title = read.title(&row).unwrap().unwrap();
    assert!(row.belongs_to(&state));
    assert!(!row.belongs_to(&other));
    let mut second = state.read_view(ItemSetReadLimits::default());
    assert!(row.same_identity(&second.active().unwrap().unwrap()));
    assert_eq!(
        title.as_ptr(),
        second.title(&row).unwrap().unwrap().as_ptr()
    );
    let mut foreign = other.read_view(ItemSetReadLimits::default());
    assert!(!row.same_identity(&foreign.active().unwrap().unwrap()));
    assert_eq!(
        foreign.title(&row).unwrap_err().kind,
        AssemblyErrorKind::Unsupported
    );
    assert_eq!(
        serde_json::to_value(state.usage()).unwrap(),
        producer_before
    );
}

#[test]
fn cumulative_read_work_and_text_fail_before_excess_without_refilling_producer() {
    let state = fresh();
    let producer_before = serde_json::to_value(state.usage()).unwrap();
    let mut probe = state.read_view(ItemSetReadLimits::default());
    assert_eq!(probe.dense_order_len().unwrap(), 1);
    let exact_steps = probe.usage().steps;
    assert!(exact_steps > 0);
    let mut read = state.read_view(ItemSetReadLimits {
        max_steps: exact_steps,
        max_text_bytes: 0,
    });
    assert_eq!(read.dense_order_len().unwrap(), 1);
    assert_eq!(read.usage().steps, exact_steps);
    assert_eq!(
        read.is_singleton().unwrap_err().kind,
        AssemblyErrorKind::Resource
    );
    assert_eq!(read.usage().steps, exact_steps);
    let mut short = state.read_view(ItemSetReadLimits {
        max_steps: exact_steps - 1,
        max_text_bytes: 0,
    });
    assert_eq!(
        short.dense_order_len().unwrap_err().kind,
        AssemblyErrorKind::Resource
    );
    assert!(short.usage().steps < exact_steps);

    let mut text = state.read_view(ItemSetReadLimits {
        max_steps: 10000,
        max_text_bytes: "Caller default".len(),
    });
    let row = text.active().unwrap().unwrap();
    assert_eq!(text.title(&row).unwrap(), Some("Caller default"));
    assert_eq!(
        text.title(&row).unwrap_err().kind,
        AssemblyErrorKind::Resource
    );
    assert_eq!(text.usage().text_bytes, "Caller default".len());
    assert_eq!(
        serde_json::to_value(state.usage()).unwrap(),
        producer_before
    );

    let mut zero = state.read_view(ItemSetReadLimits {
        max_steps: 0,
        max_text_bytes: 0,
    });
    assert_eq!(
        zero.dense_order_len().unwrap_err().kind,
        AssemblyErrorKind::Resource
    );
    assert_eq!(zero.usage(), ItemSetReadUsage::default());
}

#[test]
fn reached_source_failure_retains_readable_unappended_created_row() {
    let mut state = fresh();
    state.begin_load().unwrap();
    add(&mut state, Some("3"), Some("Finished"));
    state
        .begin_item_set(ItemSetInput {
            id: Some("4"),
            title: Some("Open"),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        state
            .socket_url(SocketUrlInput::default())
            .unwrap_err()
            .kind,
        AssemblyErrorKind::Source
    );
    assert_eq!(state.phase(), ItemSetPhase::Failed);
    let mut read = state.read_view(ItemSetReadLimits::default());
    assert_eq!(read.dense_order_len().unwrap(), 1);
    assert_eq!(read.ordered_key(1).unwrap().unwrap().value(), 3.0);
    assert!(read.ordered_key(2).unwrap().is_none());
    let open = read.winner(key(4.0)).unwrap().unwrap();
    assert_eq!(read.title(&open).unwrap(), Some("Open"));
}

#[test]
fn allocation_before_failed_publication_does_not_replace_the_readable_root() {
    let policy = policy();
    let constructor_values = ItemSetState::new(&policy, ItemSetLimits::default())
        .unwrap()
        .usage()
        .values;
    // Two writes publish previous/active ID. A third publishes the new sets;
    // the next publication of the newly allocated order is still rejected.
    for allowed_writes in [2, 3] {
        let mut state = ItemSetState::new(
            &policy,
            ItemSetLimits {
                max_values: constructor_values + allowed_writes,
                ..ItemSetLimits::default()
            },
        )
        .unwrap();
        assert_eq!(
            state.begin_load().unwrap_err().kind,
            AssemblyErrorKind::Resource
        );
        assert_eq!(state.phase(), ItemSetPhase::Failed);
        let producer_before = serde_json::to_value(state.usage()).unwrap();
        let (field, private_id) = if allowed_writes == 2 {
            ("itemSets", state.sets)
        } else {
            ("itemSetOrderList", state.order)
        };
        assert!(matches!(
            state.graph.raw_field(state.root, field),
            Some(V::Table(published)) if *published != private_id
        ));
        let mut read = state.read_view(ItemSetReadLimits::default());
        assert!(read.is_singleton().unwrap());
        assert_eq!(read.ordered_key(1).unwrap().unwrap().value(), 1.0);
        assert!(read.ordered_key(2).unwrap().is_none());
        assert_eq!(read.active_key().unwrap().unwrap().value(), 0.0);
        let active = read.active().unwrap().unwrap();
        assert!(active.same_identity(&read.previous().unwrap().unwrap()));
        assert_eq!(read.title(&active).unwrap(), Some("Caller default"));
        let winner = read.winner(key(1.0)).unwrap();
        if allowed_writes == 2 {
            assert!(active.same_identity(&winner.unwrap()));
        } else {
            assert!(winner.is_none());
        }
        assert_eq!(
            serde_json::to_value(state.usage()).unwrap(),
            producer_before
        );
    }
}

#[test]
fn persistent_identity_survives_moves_and_restarts_without_merging_duplicate_keys() {
    let mut state = fresh();
    let constructor = state.last_created_set().unwrap().clone();
    assert_eq!(state.creation_count(), 1);
    let mut origins = std::collections::HashMap::new();
    origins.insert(constructor.clone(), "constructor");
    state.begin_load().unwrap();
    add(&mut state, Some("1"), Some("First"));
    let first = state.last_created_set().unwrap().clone();
    origins.insert(first.clone(), "first");
    add(&mut state, Some("1.0"), Some("Second"));
    let second = state.last_created_set().unwrap().clone();
    origins.insert(second.clone(), "second");
    assert_eq!(state.creation_count(), 3);
    assert!(!first.same_identity(&second));
    activate(&mut state, Some("1"));
    let moved = Box::new(state);
    assert!(constructor.belongs_to(&moved));
    assert!(first.belongs_to(&moved));
    let mut read = moved.read_view(ItemSetReadLimits::default());
    let current = read.active().unwrap().unwrap().identity();
    let previous = read.previous().unwrap().unwrap().identity();
    assert_eq!(origins.get(&current), Some(&"second"));
    assert_eq!(origins.get(&previous), Some(&"constructor"));
    assert!(current.same_identity(&second));
    let foreign = fresh();
    assert!(!constructor.belongs_to(&foreign));
    assert_ne!(constructor, *foreign.last_created_set().unwrap());
    assert_eq!(origins.len(), 3);
}

#[test]
fn creation_identity_tracks_publication_before_later_failure_and_never_failed_creation() {
    let input = ItemSetInput {
        id: Some("2"),
        title: Some("Published"),
        ..Default::default()
    };
    let mut successful = fresh();
    successful.begin_load().unwrap();
    successful.begin_item_set(input).unwrap();
    // The enclosing operation's last write follows create's map publication.
    let before_last_write = successful.usage().values - 1;

    let mut published = fresh();
    published.begin_load().unwrap();
    let old = published.last_created_set().unwrap().clone();
    published.graph.limits.max_values = before_last_write;
    assert_eq!(
        published.begin_item_set(input).unwrap_err().kind,
        AssemblyErrorKind::Resource
    );
    assert_eq!(published.phase(), ItemSetPhase::Failed);
    assert_eq!(published.creation_count(), 2);
    let created = published.last_created_set().unwrap().clone();
    assert_ne!(old, created);
    let mut read = published.read_view(ItemSetReadLimits::default());
    assert_eq!(read.winner(key(2.0)).unwrap().unwrap().identity(), created);

    let mut unpublished = fresh();
    unpublished.begin_load().unwrap();
    let old = unpublished.last_created_set().unwrap().clone();
    unpublished.graph.limits.max_values = unpublished.usage().values;
    assert_eq!(
        unpublished.begin_item_set(input).unwrap_err().kind,
        AssemblyErrorKind::Resource
    );
    assert_eq!(unpublished.creation_count(), 1);
    assert_eq!(unpublished.last_created_set().unwrap(), &old);
    let mut read = unpublished.read_view(ItemSetReadLimits::default());
    assert!(read.winner(key(2.0)).unwrap().is_none());
}
