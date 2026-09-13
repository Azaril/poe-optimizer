use super::*;
use crate::item_loading::assembly::AssemblyErrorKind;
use std::collections::BTreeMap;
use std::sync::OnceLock;

fn policy() -> ItemInventoryPolicy {
    static POLICY: OnceLock<ItemInventoryPolicy> = OnceLock::new();
    let mut p = POLICY
        .get_or_init(|| {
            poe_optimizer_data::game_data::bundled_snapshot()
                .unwrap()
                .item_assembly()
                .policy()
                .inventory
                .clone()
        })
        .clone();
    p.layout.base_slots = vec!["Main".into(), "Off".into(), "Body".into()];
    p.layout.swap.slot_pattern = "^never$".into();
    p.layout.embedded.parent_slots = vec!["Body".into()];
    p.layout.embedded.count = 1;
    p.layout.embedded.name_infix = " Jewel ".into();
    p.layout.rune_slots.clear();
    p.layout.passive.nodes.ids.clear();
    p.layout.passive.nodes.validity_nodes = Default::default();
    p
}
#[derive(Default)]
struct Context {
    ids: Vec<f64>,
    valid: BTreeSet<(i64, String)>,
    dependencies: BTreeMap<String, Vec<String>>,
    counts: BTreeMap<i64, f64>,
    invalid_label: bool,
    invalid_query: bool,
    query_cost: u64,
    steps: u64,
    bytes: usize,
    queries: usize,
    node_available: bool,
    node_writes: Vec<(f64, f64)>,
}
impl ItemActivationContext for Context {
    fn inventory_ids(&self) -> &[f64] {
        &self.ids
    }
    fn take_validation_steps(&mut self) -> u64 {
        std::mem::take(&mut self.steps)
    }
    fn take_preparation_bytes(&mut self) -> usize {
        std::mem::take(&mut self.bytes)
    }
    fn node_writes_available(&mut self, _: &[(f64, f64)]) -> Result<bool> {
        Ok(self.node_available)
    }
    fn valid_for_slot(&mut self, id: f64, slot: &str, _: Value<'_>) -> Result<bool> {
        self.queries += 1;
        self.steps += self.query_cost;
        if self.invalid_query {
            return Err(AssemblyError::source("caller query error"));
        }
        Ok(self.valid.contains(&(id as i64, slot.into())))
    }
    fn item_label(&mut self, id: f64) -> Result<String> {
        if self.invalid_label {
            Err(AssemblyError::source("caller malformed rarity/name"))
        } else {
            Ok(format!("Item {id}"))
        }
    }
    fn jewel_socket_count(&mut self, id: f64) -> Result<f64> {
        Ok(*self.counts.get(&(id as i64)).unwrap_or(&0.0))
    }
    fn selection_dependencies(&mut self, slot: &str) -> Result<Vec<String>> {
        self.steps += self.query_cost;
        Ok(self.dependencies.get(slot).cloned().unwrap_or_default())
    }
    fn initial_rune(&mut self, _: &str) -> DependencyResult<ItemActivationRune> {
        panic!("fixture has no rune slots")
    }
    fn select_rune(
        &mut self,
        _: &str,
        _: Value<'_>,
        _: &ItemActivationRune,
    ) -> DependencyResult<ItemActivationRune> {
        panic!("fixture has no rune slots")
    }
    fn set_node_selection(&mut self, node: f64, id: f64, _: ItemNumber) -> DependencyResult<()> {
        assert!(self.node_available);
        self.node_writes.push((node, id));
        DependencyResult::Available(())
    }
}
fn loaded(policy: &ItemInventoryPolicy, rows: &[(&str, &str)]) -> ItemSetState {
    let mut state = ItemSetState::new(policy, ItemSetLimits::default()).unwrap();
    state.begin_load().unwrap();
    state
        .begin_item_set(ItemSetInput {
            id: Some("2"),
            title: Some("Incoming"),
            ..Default::default()
        })
        .unwrap();
    for (name, id) in rows {
        state
            .set_slot(SetSlotInput {
                name: Some(name),
                item_id: Some(id),
                ..Default::default()
            })
            .unwrap();
    }
    state.finish_item_set().unwrap();
    state
        .finish_load(FinishLoadInput {
            active_item_set: Some("2"),
            show_stat_differences: Some("false"),
            ..Default::default()
        })
        .unwrap();
    state
}
fn field(s: &mut ItemSetState, row: Id, key: &str) -> V {
    s.graph.field(row, key).unwrap()
}
fn slot(s: &mut ItemSetState, name: &str) -> Id {
    table(s.graph.field(s.slots, name).unwrap()).unwrap()
}
fn active(s: &mut ItemSetState) -> Id {
    table(s.graph.field(s.root, "activeItemSet").unwrap()).unwrap()
}

#[test]
fn successful_copy_population_preserves_previous_alias_and_stops_before_sync() {
    let mut state = loaded(&policy(), &[("Body", "1")]);
    let root = state.root;
    let previous = table(field(&mut state, root, "previousActiveItemSet")).unwrap();
    let body = slot(&mut state, "Body");
    state
        .graph
        .set_field(body, "note", V::Bytes(b"retained note".to_vec()))
        .unwrap();
    let item_list = state.graph.table().unwrap();
    let labels = state.graph.table().unwrap();
    state.graph.append(item_list, V::Number(999.0)).unwrap();
    state
        .graph
        .append(labels, V::Bytes(b"old label".to_vec()))
        .unwrap();
    state
        .graph
        .set_field(body, "items", V::Table(item_list))
        .unwrap();
    state
        .graph
        .set_field(body, "list", V::Table(labels))
        .unwrap();
    let mut ctx = Context {
        ids: vec![1.0],
        valid: BTreeSet::from([(1, "Body".into())]),
        counts: BTreeMap::from([(1, 1.0)]),
        query_cost: 3,
        ..Default::default()
    };
    assert!(matches!(
        state.continue_activation(&mut ctx).unwrap(),
        ItemActivationProgress::AwaitingSyncLoadouts
    ));
    assert_eq!(state.phase(), ItemSetPhase::AwaitingSyncLoadouts);
    let current = active(&mut state);
    assert_ne!(current, previous);
    let old_body = table(field(&mut state, previous, "Body")).unwrap();
    assert_eq!(
        field(&mut state, old_body, "note"),
        V::Bytes(b"retained note".to_vec())
    );
    assert_eq!(field(&mut state, body, "selItemId"), V::Number(1.0));
    assert_eq!(field(&mut state, body, "items"), V::Table(item_list));
    assert_eq!(field(&mut state, body, "list"), V::Table(labels));
    assert_eq!(state.graph.number(item_list, 1.0).unwrap(), V::Number(0.0));
    assert_eq!(state.graph.number(item_list, 2.0).unwrap(), V::Number(1.0));
    assert_eq!(
        state.graph.number(labels, 2.0).unwrap(),
        V::Bytes(b"Item 1".to_vec())
    );
    let child = slot(&mut state, "Body Jewel 1");
    assert_eq!(field(&mut state, child, "inactive"), V::Boolean(false));
    let root = state.root;
    assert_eq!(
        field(&mut state, root, "showStatDifferences"),
        V::Boolean(true)
    );
    assert_eq!(field(&mut state, root, "buildFlag"), V::Boolean(true));
    assert!(state.begin_load().is_err());
    let usage = state.usage().steps;
    state.continue_activation(&mut ctx).unwrap();
    assert_eq!(
        state.usage().steps,
        usage,
        "does not replay completed population"
    );
}
#[test]
fn legacy_fallback_previous_current_alias_reads_live_values_after_writes() {
    let mut state = ItemSetState::new(&policy(), ItemSetLimits::default()).unwrap();
    state.begin_load().unwrap();
    state
        .legacy_slot(LegacySlotInput {
            name: Some("Main"),
            item_id: Some("1"),
            active: None,
        })
        .unwrap();
    state.finish_load(FinishLoadInput::default()).unwrap();
    let mut ctx = Context {
        ids: vec![1.0],
        valid: BTreeSet::from([(1, "Main".into())]),
        ..Default::default()
    };
    state.continue_activation(&mut ctx).unwrap();
    let root = state.root;
    assert_eq!(
        field(&mut state, root, "activeItemSet"),
        field(&mut state, root, "previousActiveItemSet")
    );
    let main = slot(&mut state, "Main");
    assert_eq!(field(&mut state, main, "selItemId"), V::Number(1.0));
}
#[test]
fn clearing_a_read_parent_is_an_order_frontier_without_guessed_population() {
    let mut state = loaded(&policy(), &[("Main", "2"), ("Off", "3")]);
    let mut ctx = Context {
        ids: vec![2.0, 3.0],
        valid: BTreeSet::from([(3, "Off".into())]),
        dependencies: BTreeMap::from([("Off".into(), vec!["Main".into()])]),
        ..Default::default()
    };
    assert!(matches!(
        state.continue_activation(&mut ctx).unwrap(),
        ItemActivationProgress::AwaitingDependency {
            stage: "population_order",
            ..
        }
    ));
    let main = slot(&mut state, "Main");
    assert_eq!(field(&mut state, main, "selItemId"), V::Number(2.0));
    assert_eq!(field(&mut state, main, "items"), V::Nil);
    assert!(state.failure().is_none());
}
#[test]
fn unrelated_invalid_selection_clears_both_live_and_saved_rows() {
    let mut state = loaded(&policy(), &[("Main", "9")]);
    let mut ctx = Context::default();
    state.continue_activation(&mut ctx).unwrap();
    let main = slot(&mut state, "Main");
    let current = active(&mut state);
    let row = table(field(&mut state, current, "Main")).unwrap();
    assert_eq!(field(&mut state, main, "selItemId"), V::Number(0.0));
    assert_eq!(field(&mut state, row, "selItemId"), V::Number(0.0));
}
#[test]
fn unchanged_node_writes_require_available_owned_storage_and_remain_reached() {
    let mut p = policy();
    p.layout.passive.nodes.ids = vec![7];
    p.layout.passive.nodes.validity_nodes.indexed.insert(
        7,
        poe_optimizer_data::item_loading::ItemMetadataValue::Table(Default::default()),
    );
    let node_name = format!("{}7", p.layout.passive.slot_prefix);
    let mut state = loaded(&p, &[]);
    let mut ctx = Context::default();
    assert!(matches!(
        state.continue_activation(&mut ctx).unwrap(),
        ItemActivationProgress::AwaitingDependency {
            stage: "population_order",
            ..
        }
    ));
    assert!(ctx.node_writes.is_empty());
    ctx.node_available = true;
    assert!(matches!(
        state.continue_activation(&mut ctx).unwrap(),
        ItemActivationProgress::AwaitingSyncLoadouts
    ));
    assert_eq!(ctx.node_writes, vec![(7.0, 0.0)]);
    let node = slot(&mut state, &node_name);
    assert_eq!(field(&mut state, node, "selItemId"), V::Number(0.0));
}
#[test]
fn changed_node_selection_refuses_cluster_order_before_any_setter() {
    let mut p = policy();
    p.layout.passive.nodes.ids = vec![7];
    p.layout.passive.nodes.validity_nodes.indexed.insert(
        7,
        poe_optimizer_data::item_loading::ItemMetadataValue::Table(Default::default()),
    );
    let node_name = format!("{}7", p.layout.passive.slot_prefix);
    let mut state = loaded(&p, &[]);
    let node = slot(&mut state, &node_name);
    state
        .graph
        .set_field(node, "selItemId", V::Number(9.0))
        .unwrap();
    let mut ctx = Context {
        node_available: true,
        ..Default::default()
    };
    assert!(matches!(
        state.continue_activation(&mut ctx).unwrap(),
        ItemActivationProgress::AwaitingDependency {
            stage: "population_order",
            ..
        }
    ));
    assert_eq!(field(&mut state, node, "selItemId"), V::Number(9.0));
    assert!(ctx.node_writes.is_empty());
}
#[test]
fn fallible_candidate_labels_are_order_frontiers_not_fabricated_source_failures() {
    let mut state = loaded(&policy(), &[]);
    let mut ctx = Context {
        ids: vec![1.0],
        valid: BTreeSet::from([(1, "Main".into())]),
        invalid_label: true,
        ..Default::default()
    };
    assert!(matches!(
        state.continue_activation(&mut ctx).unwrap(),
        ItemActivationProgress::AwaitingDependency {
            stage: "population_order",
            ..
        }
    ));
    assert!(state.failure().is_none());
    let main = slot(&mut state, "Main");
    assert_eq!(field(&mut state, main, "list"), V::Nil);
}
#[test]
fn failed_probes_drain_actual_work_and_native_context_bytes_are_bounded() {
    let mut state = loaded(&policy(), &[]);
    let mut ctx = Context {
        ids: vec![1.0],
        invalid_query: true,
        query_cost: 123,
        ..Default::default()
    };
    let before = state.usage().steps;
    assert!(matches!(
        state.continue_activation(&mut ctx).unwrap(),
        ItemActivationProgress::AwaitingDependency {
            stage: "population_order",
            ..
        }
    ));
    assert!(state.usage().steps >= before + 123);
    assert_eq!(ctx.steps, 0);
    assert_eq!(ctx.queries, 1);
    let mut state = loaded(&policy(), &[]);
    let mut ctx = Context {
        bytes: state.limits().max_bytes,
        ..Default::default()
    };
    assert_eq!(
        state.continue_activation(&mut ctx).unwrap_err().kind,
        AssemblyErrorKind::Resource
    );
    assert_eq!(state.phase(), ItemSetPhase::Failed);
}
#[test]
fn aliasing_rows_across_slot_keys_is_refused_before_commuting_copy_claim() {
    let mut state = loaded(&policy(), &[("Main", "1")]);
    let current = table(state.graph.number(state.sets, 2.0).unwrap()).unwrap();
    let row = field(&mut state, current, "Main");
    state.graph.set_field(current, "Off", row).unwrap();
    let mut ctx = Context::default();
    assert!(matches!(
        state.continue_activation(&mut ctx).unwrap(),
        ItemActivationProgress::AwaitingDependency {
            stage: "slot_copy_order",
            ..
        }
    ));
    let main = slot(&mut state, "Main");
    assert_eq!(field(&mut state, main, "selItemId"), V::Number(0.0));
}

#[test]
fn sparse_child_lists_refuse_before_population_instead_of_guessing_ipairs_indices() {
    let mut state = loaded(&policy(), &[]);
    let body = slot(&mut state, "Body");
    let children = table(field(&mut state, body, "jewelSocketList")).unwrap();
    let child = state.graph.number(children, 1.0).unwrap();
    state.graph.set(children, V::Number(1.0), V::Nil).unwrap();
    state.graph.set(children, V::Number(2.0), child).unwrap();
    let mut ctx = Context::default();
    assert!(matches!(
        state.continue_activation(&mut ctx).unwrap(),
        ItemActivationProgress::AwaitingDependency {
            stage: "population_order",
            ..
        }
    ));
    assert_eq!(field(&mut state, body, "items"), V::Nil);
}
#[test]
fn competing_parent_child_writers_are_not_assumed_to_commute() {
    let mut state = loaded(&policy(), &[("Body", "1")]);
    let body = slot(&mut state, "Body");
    let main = slot(&mut state, "Main");
    let body_children = field(&mut state, body, "jewelSocketList");
    state
        .graph
        .set_field(main, "jewelSocketList", body_children)
        .unwrap();
    let mut ctx = Context {
        ids: vec![1.0],
        valid: BTreeSet::from([(1, "Body".into())]),
        counts: BTreeMap::from([(1, 1.0)]),
        ..Default::default()
    };
    assert!(matches!(
        state.continue_activation(&mut ctx).unwrap(),
        ItemActivationProgress::AwaitingDependency {
            stage: "population_order",
            ..
        }
    ));
    let child = slot(&mut state, "Body Jewel 1");
    assert_eq!(field(&mut state, child, "inactive"), V::Boolean(true));
    assert_eq!(field(&mut state, body, "items"), V::Nil);
}
