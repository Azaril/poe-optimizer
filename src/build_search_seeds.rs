//! Bounded deterministic seed repair. This constructs a useful starting point;
//! failure is a sampled-seed diagnostic, not proof that the domain is empty.
use poe_optimizer_core::candidate::*;
use poe_optimizer_data::class_tree::{AttributeOption, PassiveAllocationSelection};
use poe_optimizer_import::controlled_build::{
    AttributeOptionLocks, BuildSelection, ControlledBuildCatalog, MAX_SELECTED_PASSIVES,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn repair_seed(
    catalog: &ControlledBuildCatalog,
    constraints: &CandidateConstraints,
    attribute_locks: &AttributeOptionLocks,
    mut seed: BuildSelection,
) -> Result<BuildSelection, String> {
    if seed.candidate.passives.len() > MAX_SELECTED_PASSIVES
        || seed.attribute_options.len() > MAX_SELECTED_PASSIVES
    {
        return Err("Seed passive count exceeds preparation bound".into());
    }
    if seed.candidate.catalog != catalog.catalog().identity {
        return Err("Seed belongs to another catalog".into());
    }
    let domain = CandidateDomain::new(catalog.catalog().clone(), constraints.clone())
        .map_err(|e| e.to_string())?;
    let locks = &constraints.locks;
    if let Some(class) = &locks.class_id {
        seed.candidate.class_id = class.clone();
    }
    match &locks.ascendancy {
        Some(AscendancyLock::None) => seed.candidate.ascendancy_id = None,
        Some(AscendancyLock::Id(id)) => {
            seed.candidate.ascendancy_id = Some(id.clone());
            if locks.class_id.is_none() {
                seed.candidate.class_id = catalog.catalog().ascendancies[id].class_id.clone();
            }
        }
        None => {
            if seed.candidate.ascendancy_id.as_ref().is_some_and(|id| {
                catalog
                    .catalog()
                    .ascendancies
                    .get(id)
                    .is_none_or(|asc| asc.class_id != seed.candidate.class_id)
            }) {
                seed.candidate.ascendancy_id = None;
            }
        }
    }
    repair_items(
        catalog.catalog(),
        constraints,
        &mut seed.candidate.equipment,
    )?;
    for (slot, lock) in &locks.skill_groups {
        let target = seed
            .candidate
            .skills
            .get_mut(slot)
            .ok_or_else(|| format!("Locked skill group {slot} has no profile assignment"))?;
        if let Some(id) = &lock.active_instance_id {
            target.active_instance_id = id.clone();
        }
        if let Some(exact) = &lock.exact_support_instance_ids {
            target.support_instance_ids = exact.clone();
        } else {
            target
                .support_instance_ids
                .retain(|id| !lock.forbidden_support_instance_ids.contains(id));
            target
                .support_instance_ids
                .extend(lock.required_support_instance_ids.iter().cloned());
        }
    }
    for slot in &locks.empty_skill_groups {
        seed.candidate.skills.remove(slot);
    }
    seed.candidate
        .passives
        .retain(|id| !locks.unallocated_passives.contains(id));
    seed.candidate.passives.extend(&locks.allocated_passives);
    seed.candidate
        .passives
        .extend(attribute_locks.required.keys());
    if !seed
        .candidate
        .passives
        .is_disjoint(&locks.unallocated_passives)
    {
        return Err("Required attribute node is locked unallocated".into());
    }
    if seed.candidate.passives.len() > MAX_SELECTED_PASSIVES {
        return Err("Required seed passive count exceeds preparation bound".into());
    }
    let allocation = seed
        .allocation(catalog.catalog())
        .map_err(|e| e.to_string())?;
    let ordinary = connect(
        catalog,
        constraints,
        attribute_locks,
        &allocation,
        &seed.attribute_options,
        false,
    )?;
    let ascendancy = connect(
        catalog,
        constraints,
        attribute_locks,
        &allocation,
        &seed.attribute_options,
        true,
    )?;
    seed.candidate.passives = ordinary.0.union(&ascendancy.0).copied().collect();
    seed.attribute_options = ordinary.1;
    let resolved = seed
        .allocation(catalog.catalog())
        .map_err(|e| e.to_string())?;
    resolved
        .resolve(catalog.data().snapshot())
        .map_err(|e| e.to_string())?;
    let validation = domain.validate(&seed.candidate);
    if !validation.is_searchable() {
        return Err(format!(
            "Repaired seed violates unchanged constraints: {validation:?}"
        ));
    }
    for (id, option) in &attribute_locks.required {
        if seed.attribute_options.get(id) != Some(option) {
            return Err("Required attribute option could not be repaired".into());
        }
    }
    for (id, options) in &attribute_locks.forbidden {
        if seed
            .attribute_options
            .get(id)
            .is_some_and(|option| options.contains(option))
        {
            return Err("Forbidden attribute option survived repair".into());
        }
    }
    Ok(seed)
}

// Augmenting-path matching finds an assignment for all mandatory physical item
// instances together. Optional original gear is restored only to remaining slots.
fn repair_items(
    catalog: &CandidateCatalog,
    constraints: &CandidateConstraints,
    equipment: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let mut required = constraints.required_item_instance_ids.clone();
    required.extend(constraints.locks.equipment.values().flatten().cloned());
    let fixed = &constraints.locks.equipment;
    let mut assignments: BTreeMap<String, String> = fixed
        .iter()
        .filter_map(|(slot, id)| id.as_ref().map(|id| (slot.clone(), id.clone())))
        .collect();
    let mut choices = BTreeMap::new();
    for id in &required {
        if assignments.values().any(|value| value == id) {
            continue;
        }
        let item = catalog
            .items
            .get(id)
            .ok_or_else(|| format!("Unknown required item {id}"))?;
        let mut slots: Vec<_> = item
            .compatible_slots
            .iter()
            .filter(|slot| !fixed.contains_key(*slot))
            .cloned()
            .collect();
        slots.sort_by_key(|slot| (equipment.get(slot) != Some(id), slot.clone()));
        choices.insert(id.clone(), slots);
    }
    fn place(
        id: &str,
        choices: &BTreeMap<String, Vec<String>>,
        assignments: &mut BTreeMap<String, String>,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        for slot in &choices[id] {
            if !visited.insert(slot.clone()) {
                continue;
            }
            let displaced = assignments.get(slot).cloned();
            if displaced
                .as_ref()
                .is_none_or(|other| place(other, choices, assignments, visited))
            {
                assignments.insert(slot.clone(), id.into());
                return true;
            }
        }
        false
    }
    for id in choices.keys() {
        if !place(id, &choices, &mut assignments, &mut BTreeSet::new()) {
            return Err(format!(
                "Required item {id} has no compatible unlocked physical slot assignment"
            ));
        }
    }
    for (slot, id) in equipment.iter() {
        if !fixed.contains_key(slot)
            && !assignments.contains_key(slot)
            && !assignments.values().any(|value| value == id)
            && catalog
                .items
                .get(id)
                .is_some_and(|item| item.compatible_slots.contains(slot))
        {
            assignments.insert(slot.clone(), id.clone());
        }
    }
    *equipment = assignments;
    Ok(())
}

type Connected = (BTreeSet<u32>, BTreeMap<u32, AttributeOption>);
fn connect(
    catalog: &ControlledBuildCatalog,
    constraints: &CandidateConstraints,
    locks: &AttributeOptionLocks,
    allocation: &PassiveAllocationSelection,
    preferred: &BTreeMap<u32, AttributeOption>,
    ascendancy: bool,
) -> Result<Connected, String> {
    let targets = if ascendancy {
        &allocation.ascendancy_nodes
    } else {
        &allocation.ordinary_nodes
    };
    if targets.is_empty() {
        return Ok((BTreeSet::new(), BTreeMap::new()));
    }
    let graph = &catalog.catalog().passive_nodes;
    let root = if ascendancy {
        let id = allocation
            .ascendancy_id
            .as_ref()
            .ok_or("Locked ascendancy nodes require a selected ascendancy")?;
        catalog
            .catalog()
            .ascendancies
            .get(id)
            .ok_or("Unknown seed ascendancy")?
            .start_node_id
    } else {
        catalog
            .catalog()
            .classes
            .get(&allocation.class_id.to_string())
            .ok_or("Unknown seed class")?
            .start_node_id
    };
    let budget = u64::from(if ascendancy {
        constraints.budgets.ascendancy_passive_points
    } else {
        constraints.budgets.ordinary_passive_points
    });
    let snapshot = catalog.data().snapshot();
    let mut admitted = BTreeMap::new();
    // Query each possible node once; all source precedence and capability rules
    // are shared with final allocation resolution in the data model.
    for (id, node) in graph {
        if constraints.locks.unallocated_passives.contains(id) {
            continue;
        }
        if ascendancy {
            if matches!(&node.kind,PassiveKind::Ascendancy{ascendancy_ids}if allocation.ascendancy_id.as_ref().is_some_and(|id|ascendancy_ids.contains(id)))
                && allocation.resolve_ascendancy_node(snapshot, *id).is_ok()
            {
                admitted.insert(*id, None);
            }
        } else if matches!(node.kind, PassiveKind::Ordinary) {
            let source = &snapshot.tree().allocation_nodes[id];
            if source.attribute_options.is_empty() {
                if allocation
                    .resolve_ordinary_node(snapshot, *id, None)
                    .is_ok()
                {
                    admitted.insert(*id, None);
                }
            } else {
                let preferred = locks.required.get(id).or(preferred.get(id));
                let candidates = preferred
                    .copied()
                    .into_iter()
                    .chain(AttributeOption::ALL)
                    .collect::<Vec<_>>();
                for option in candidates {
                    if locks
                        .required
                        .get(id)
                        .is_some_and(|required| *required != option)
                        || locks
                            .forbidden
                            .get(id)
                            .is_some_and(|set| set.contains(&option))
                    {
                        continue;
                    }
                    if allocation
                        .resolve_ordinary_node(snapshot, *id, Some(option))
                        .is_ok()
                    {
                        admitted.insert(*id, Some(option));
                        break;
                    }
                }
            }
        }
    }
    if let Some(id) = targets.iter().find(|id| !admitted.contains_key(id)) {
        return Err(format!(
            "Required seed passive {id} has no permitted capability-admitted view"
        ));
    }
    let mut connected = BTreeSet::from([root]);
    let mut missing = targets.clone();
    let mut spent = 0_u64;
    while !missing.is_empty() {
        // Add the nearest remaining target to the connected component. Source
        // point costs are used; this does not solve the general Steiner-tree problem.
        let mut distances: BTreeMap<u32, u64> = connected.iter().map(|id| (*id, 0)).collect();
        let mut queue: BTreeSet<(u64, u32)> = connected.iter().map(|id| (0, *id)).collect();
        let mut previous = BTreeMap::new();
        let mut reached = None;
        while let Some((cost, id)) = queue.pop_first() {
            if distances[&id] != cost {
                continue;
            }
            if missing.contains(&id) {
                reached = Some((cost, id));
                break;
            }
            for next in &graph[&id].links {
                if !admitted.contains_key(next) {
                    continue;
                }
                let extra = if connected.contains(next) {
                    0
                } else {
                    u64::from(graph[next].point_cost)
                };
                let Some(next_cost) = cost
                    .checked_add(extra)
                    .filter(|cost| *cost <= budget.saturating_sub(spent))
                else {
                    continue;
                };
                if distances.get(next).is_none_or(|old| next_cost < *old) {
                    distances.insert(*next, next_cost);
                    previous.insert(*next, id);
                    queue.insert((next_cost, *next));
                }
            }
        }
        let (cost,mut at)=reached.ok_or_else(||format!("Cannot connect required seed passives within the supplied {} point budget or forbidden nodes",if ascendancy{"ascendancy"}else{"ordinary"}))?;
        spent += cost;
        while connected.insert(at) {
            missing.remove(&at);
            at = previous[&at];
        }
        missing.retain(|id| !connected.contains(id));
    }
    connected.remove(&root);
    let options = connected
        .iter()
        .filter_map(|id| admitted[id].map(|option| (*id, option)))
        .collect();
    Ok((connected, options))
}

#[cfg(test)]
mod tests {
    use super::*;
    use poe_optimizer_import::controlled_build::{ControlledBuildDomain, EquipmentAlternative};
    use std::sync::{Arc, OnceLock};
    const TEMPLATE: &str = include_str!("../tests/fixtures/calibration/mace-wooden.xml");
    fn catalog() -> Arc<ControlledBuildCatalog> {
        static DATA: OnceLock<Arc<poe_optimizer_native::CompiledGameData>> = OnceLock::new();
        let data = DATA.get_or_init(|| {
            Arc::new(
                poe_optimizer_native::CompiledGameData::compile(Arc::new(
                    poe_optimizer_data::game_data::bundled_snapshot().unwrap(),
                ))
                .unwrap(),
            )
        });
        Arc::new(ControlledBuildCatalog::new(data.clone(),TEMPLATE.into(),vec![
            EquipmentAlternative{instance_id:"weapon".into(),pob_item_id:2,item_text:"Rarity: NORMAL\nSmithing Hammer\nItem Level: 1\nQuality: 0\nImplicits: 0\n".into()},
            EquipmentAlternative{instance_id:"amulet".into(),pob_item_id:3,item_text:"Rarity: NORMAL\nAmber Amulet\nItem Level: 1\nQuality: 0\nImplicits: 1\n+12 to Strength\n".into()},
        ]).unwrap())
    }
    fn constraints() -> CandidateConstraints {
        CandidateConstraints {
            budgets: CandidateBudgets {
                ordinary_passive_points: 8,
                ascendancy_passive_points: 8,
                active_skill_count: 1,
                supports_per_skill: 2,
                ..Default::default()
            },
            ..Default::default()
        }
    }
    #[test]
    fn oversized_malformed_seed_has_one_bounded_error_before_graph_repair() {
        let catalog = catalog();
        let mut seed = catalog.source_selection();
        seed.candidate.passives = (100_000..150_000).collect();
        let error = repair_seed(
            &catalog,
            &constraints(),
            &AttributeOptionLocks::default(),
            seed,
        )
        .unwrap_err();
        assert_eq!(error, "Seed passive count exceeds preparation bound");
    }
    #[test]
    fn mandatory_physical_items_and_locked_connected_attribute_path_form_legal_seed() {
        let catalog = catalog();
        let mut constraints = constraints();
        constraints.locks.class_id = Some("6".into());
        constraints.required_item_instance_ids = BTreeSet::from(["weapon".into(), "amulet".into()]);
        constraints.locks.allocated_passives.insert(13397);
        constraints.budgets.ordinary_passive_points = 2;
        let attrs = AttributeOptionLocks {
            required: BTreeMap::from([(13397, AttributeOption::Dexterity)]),
            ..Default::default()
        };
        let repaired =
            repair_seed(&catalog, &constraints, &attrs, catalog.source_selection()).unwrap();
        assert_eq!(repaired.candidate.passives, BTreeSet::from([3936, 13397]));
        assert_eq!(
            repaired.attribute_options[&13397],
            AttributeOption::Dexterity
        );
        assert_eq!(repaired.candidate.equipment["Weapon 1"], "weapon");
        assert_eq!(repaired.candidate.equipment["Amulet"], "amulet");
        let domain = ControlledBuildDomain::new(catalog, constraints, attrs).unwrap();
        assert!(
            domain
                .admit(repaired, &mut poe_optimizer_native::ActorScratch::default())
                .is_ok()
        );
    }
    #[test]
    fn repair_respects_forbidden_connectors_real_budgets_and_empty_equipment_locks() {
        let catalog = catalog();
        let mut constraints = constraints();
        constraints.locks.allocated_passives.insert(13397);
        constraints.budgets.ordinary_passive_points = 1;
        assert!(
            repair_seed(
                &catalog,
                &constraints,
                &AttributeOptionLocks::default(),
                catalog.source_selection()
            )
            .is_err()
        );
        constraints.budgets.ordinary_passive_points = 2;
        constraints.locks.unallocated_passives.insert(3936);
        assert!(
            repair_seed(
                &catalog,
                &constraints,
                &AttributeOptionLocks::default(),
                catalog.source_selection()
            )
            .is_err()
        );
        let mut constraints = self::constraints();
        constraints
            .required_item_instance_ids
            .insert("amulet".into());
        constraints.locks.equipment.insert("Amulet".into(), None);
        assert!(
            repair_seed(
                &catalog,
                &constraints,
                &AttributeOptionLocks::default(),
                catalog.source_selection()
            )
            .is_err()
        );
    }
    #[test]
    fn exact_support_locks_and_ascendancy_owner_repair_preserve_constraints() {
        let catalog = catalog();
        let mut constraints = constraints();
        let (id, asc) = catalog
            .catalog()
            .ascendancies
            .iter()
            .find(|(_, asc)| asc.class_id != "6")
            .unwrap();
        constraints.locks.ascendancy = Some(AscendancyLock::Id(id.clone()));
        let slot = catalog
            .source_selection()
            .candidate
            .skills
            .keys()
            .next()
            .unwrap()
            .clone();
        let support = catalog.catalog().supports.keys().next().unwrap().clone();
        constraints.locks.skill_groups.insert(
            slot.clone(),
            SkillGroupLocks {
                exact_support_instance_ids: Some(BTreeSet::from([support.clone()])),
                ..Default::default()
            },
        );
        let repaired = repair_seed(
            &catalog,
            &constraints,
            &AttributeOptionLocks::default(),
            catalog.source_selection(),
        )
        .unwrap();
        assert_eq!(repaired.candidate.class_id, asc.class_id);
        assert_eq!(
            repaired.candidate.ascendancy_id.as_deref(),
            Some(id.as_str())
        );
        assert_eq!(
            repaired.candidate.skills[&slot].support_instance_ids,
            BTreeSet::from([support])
        );
    }
    #[test]
    fn item_matching_reassigns_flexible_instances_and_never_duplicates_physical_ids() {
        let catalog = catalog();
        let mut graph = catalog.catalog().clone();
        graph.equipment_slots = BTreeSet::from(["left".into(), "right".into()]);
        let flexible = graph.items.remove("weapon").unwrap();
        let specific = graph.items.remove("amulet").unwrap();
        graph.items.insert("a_flexible".into(), flexible);
        graph.items.insert("z_specific".into(), specific);
        graph.items.get_mut("a_flexible").unwrap().compatible_slots = graph.equipment_slots.clone();
        graph.items.get_mut("z_specific").unwrap().compatible_slots =
            BTreeSet::from(["left".into()]);
        let mut constraints = constraints();
        constraints.required_item_instance_ids =
            BTreeSet::from(["a_flexible".into(), "z_specific".into()]);
        let mut selected = BTreeMap::from([("left".into(), "a_flexible".into())]);
        repair_items(&graph, &constraints, &mut selected).unwrap();
        assert_eq!(
            selected,
            BTreeMap::from([
                ("left".into(), "z_specific".into()),
                ("right".into(), "a_flexible".into())
            ])
        );
        constraints
            .locks
            .equipment
            .insert("left".into(), Some("a_flexible".into()));
        assert!(repair_items(&graph, &constraints, &mut selected).is_err());
    }
}
