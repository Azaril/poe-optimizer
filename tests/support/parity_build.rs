//! Test-only assembly of independent numerical cases through the retained lazy domain.
//! No finite catalog enumeration or copied numerical outputs live here.
use poe_optimizer_core::{candidate::*, evaluation::*};
use poe_optimizer_data::{class_tree::ClassTreeSelection, game_data::GameDataSnapshot};
use poe_optimizer_import::{
    controlled_build::*,
    item_source::{self, ItemSourceKind, ItemSourceNode},
    source_xml::PobContentEntry,
};
use poe_optimizer_native::CompiledGameData;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Deserialize)]
pub struct WeaponCase {
    pub id: String,
    pub item_text: String,
}

pub struct ParityBuild {
    pub catalog: Arc<ControlledBuildCatalog>,
    domain: ControlledBuildDomain,
}
#[allow(dead_code)] // Each integration target uses only the fixture operations it exercises.
impl ParityBuild {
    pub fn new(data: Arc<GameDataSnapshot>, source: String, weapons: Vec<WeaponCase>) -> Self {
        let compiled = Arc::new(CompiledGameData::compile(data).unwrap());
        let alternatives = weapons
            .into_iter()
            .enumerate()
            .map(|(i, w)| EquipmentAlternative {
                instance_id: w.id,
                // Keep source inventory ID1 intact; alternatives are distinct physical items.
                pob_item_id: 100 + i as u32,
                item_text: w.item_text,
            })
            .collect();
        let catalog =
            Arc::new(ControlledBuildCatalog::new(compiled, source, alternatives).unwrap());
        let domain = ControlledBuildDomain::new(
            catalog.clone(),
            CandidateConstraints {
                budgets: CandidateBudgets {
                    ordinary_passive_points: 1,
                    ascendancy_passive_points: 1,
                    active_skill_count: 1,
                    supports_per_skill: 2,
                    ..Default::default()
                },
                ..Default::default()
            },
            AttributeOptionLocks::default(),
        )
        .unwrap();
        Self { catalog, domain }
    }
    pub fn template_build(&self) -> BuildDocument {
        BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: self.catalog.source().source().into(),
        }
    }
    pub fn prepare(
        &self,
        tree: &ClassTreeSelection,
        weapon: &str,
        supports: &[String],
    ) -> PreparedBuildSelection {
        let mut selection = self.catalog.source_selection();
        selection.candidate.class_id = tree.class_id.to_string();
        selection.candidate.ascendancy_id = tree.ascendancy_id.clone();
        selection.candidate.passives = tree
            .entrance_node_id
            .into_iter()
            .chain(tree.ascendancy_node_id)
            .collect();
        selection.attribute_options.clear();
        selection
            .candidate
            .equipment
            .insert("Weapon 1".into(), weapon.into());
        selection
            .candidate
            .skills
            .values_mut()
            .next()
            .unwrap()
            .support_instance_ids = supports
            .iter()
            .map(|key| self.catalog.support_instance(key).unwrap().into())
            .collect();
        let prepared = self
            .domain
            .prepare(
                selection.clone(),
                &mut poe_optimizer_native::ActorScratch::default(),
            )
            .unwrap();
        if !prepared.requirements().is_legal() {
            // Diagnostic parity includes infeasible builds. They must remain
            // rejected by the independent search admission boundary.
            assert!(matches!(
                self.domain.admit(
                    selection,
                    &mut poe_optimizer_native::ActorScratch::default()
                ),
                Err(BuildCatalogError::Requirements(_))
            ));
        }
        prepared
    }

    pub fn materialize(&self, selection: &PreparedBuildSelection) -> BuildDocument {
        self.catalog.materialize_prepared(selection).unwrap()
    }
    pub fn assert_realization(
        &self,
        selection: &PreparedBuildSelection,
        native: &EvaluationResult,
        reference: &EvaluationResult,
        identity: &BackendIdentity,
        reference_baseline: &EvaluationResult,
    ) {
        self.catalog
            .validate_prepared_native_realization(selection, native, identity)
            .unwrap();
        reference.validate_recorded().unwrap();
        self.assert_reference_equipment(selection, reference);
        assert_eq!(reference.backend, reference_baseline.backend);
        assert_eq!(
            reference.context.requested,
            reference_baseline.context.requested
        );
        assert_eq!(
            reference.context.calculation_mode,
            reference_baseline.context.calculation_mode
        );
        assert_eq!(
            reference.context.enemy_level,
            reference_baseline.context.enemy_level
        );
        assert_eq!(
            reference.context.config_inputs,
            reference_baseline.context.config_inputs
        );
        assert_eq!(
            reference.context.config_placeholders,
            reference_baseline.context.config_placeholders
        );
        assert_eq!(reference.build.level, native.build.level);
        assert_eq!(reference.build.class_name, native.build.class_name);
        assert_eq!(
            reference.build.ascendancy_name,
            native.build.ascendancy_name
        );
        assert_eq!(
            reference.build.allocated_nodes,
            native.build.allocated_nodes
        );
        assert_eq!(
            reference.build.main_socket_group,
            native.build.main_socket_group
        );
        assert_eq!(
            reference.coverage.active_skill_set_id,
            native.coverage.active_skill_set_id
        );
        assert_eq!(reference.coverage.unresolved_entry_count, 0);
        let (actual, expected) = (
            reference.coverage.selected_player.as_ref().unwrap(),
            native.coverage.selected_player.as_ref().unwrap(),
        );
        assert_eq!(actual.actor, expected.actor);
        assert_eq!(actual.skill_id, expected.skill_id);
        assert_eq!(actual.group_index, expected.group_index);
        assert_eq!(actual.gem_index, expected.gem_index);
        assert_eq!(actual.part_index, expected.part_index);
        assert_eq!(actual.stat_set_index, expected.stat_set_index);
        assert_eq!(
            reference.coverage.groups.len(),
            native.coverage.groups.len()
        );
        for (a, b) in reference
            .coverage
            .groups
            .iter()
            .zip(&native.coverage.groups)
        {
            assert_eq!(a.enabled, b.enabled);
            assert_eq!(a.include_in_full_dps, b.include_in_full_dps);
            assert_eq!(a.gems.len(), b.gems.len());
            for (a, b) in a.gems.iter().zip(&b.gems) {
                // PoB's gem_id is its internal catalog key; the legacy native
                // coverage adapter uses the external game ID. Bind the actual
                // source key to the injected definition before comparing meaning.
                let declared = self
                    .catalog
                    .data()
                    .snapshot()
                    .skill_identities()
                    .gem_by_key(a.gem_id.as_deref().expect("reference gem key"))
                    .expect("reference gem key belongs to the injected catalog");
                assert_eq!(Some(declared.game_id.as_str()), a.gem_game_id.as_deref());
                assert_eq!(Some(declared.variant_id.as_str()), a.variant_id.as_deref());
                assert_eq!(
                    Some(declared.primary_effect_id.as_str()),
                    a.skill_id.as_deref()
                );
                assert_eq!(b.gem_id, b.gem_game_id);
                assert_eq!(a.gem_game_id, b.gem_game_id);
                assert_eq!(a.variant_id, b.variant_id);
                assert_eq!(a.skill_id, b.skill_id);
                assert_eq!(a.enabled, b.enabled);
                assert_eq!(a.count, b.count);
                assert_eq!(a.level, b.level);
                assert_eq!(a.quality, b.quality);
                assert_eq!(a.is_support, b.is_support);
                assert_eq!(a.resolution, b.resolution);
            }
        }
    }
    // The fresh PoB export supplies source-side equipment evidence. Only selected
    // slots are compared: the lazy domain may legitimately retain unequipped inventory.
    fn assert_reference_equipment(
        &self,
        selection: &PreparedBuildSelection,
        reference: &EvaluationResult,
    ) {
        fn attribute<'a>(node: &'a ItemSourceNode<'_>, name: &str) -> &'a str {
            node.element()
                .attribute(name)
                .unwrap_or_else(|| panic!("missing reference {name}"))
                .decoded()
        }
        assert_eq!(reference.exports.len(), 1);
        assert_eq!(reference.exports[0].format, BuildFormat::PathOfBuilding2Xml);
        let projection = item_source::project_xml(&reference.exports[0].content).unwrap();
        let [items] = projection.containers() else {
            panic!("reference must export exactly one Items container");
        };
        assert_eq!(attribute(items, "useSecondWeaponSet"), "false");
        let active_id = attribute(items, "activeItemSet");
        let sets: Vec<_> = items
            .children()
            .iter()
            .filter(|node| {
                node.kind() == ItemSourceKind::ItemSet && attribute(node, "id") == active_id
            })
            .collect();
        let [active] = sets.as_slice() else {
            panic!("reference active item set must resolve uniquely");
        };
        assert_eq!(attribute(active, "useSecondWeaponSet"), "false");
        let mut slot_names = BTreeSet::new();
        let mut occupied = BTreeMap::new();
        for slot in active.children() {
            if slot.kind() == ItemSourceKind::RuneSlot {
                assert_eq!(
                    attribute(slot, "runeName"),
                    "None",
                    "unexpected equipment rune"
                );
            }
            if slot.kind() != ItemSourceKind::Slot {
                continue;
            }
            let name = attribute(slot, "name");
            assert!(
                slot_names.insert(name),
                "duplicate reference equipment slot {name}"
            );
            let raw_id = attribute(slot, "itemId");
            let id: u32 = raw_id.parse().expect("canonical reference equipment ID");
            assert_eq!(raw_id, id.to_string());
            if id != 0 {
                occupied.insert(name.to_owned(), id);
            }
        }
        let expected: BTreeMap<_, _> = selection
            .selection()
            .candidate
            .equipment
            .iter()
            .map(|(slot, instance)| {
                (
                    slot.clone(),
                    self.catalog.item(instance).unwrap().pob_item_id(),
                )
            })
            .collect();
        assert_eq!(
            occupied, expected,
            "reference equipped slots or physical item IDs changed"
        );
        for instance in selection.selection().candidate.equipment.values() {
            let expected = self.catalog.item(instance).unwrap();
            let id = expected.pob_item_id().to_string();
            let records: Vec<_> = items
                .children()
                .iter()
                .filter(|node| node.kind() == ItemSourceKind::Item && attribute(node, "id") == id)
                .collect();
            let [item] = records.as_slice() else {
                panic!("reference equipped item {id} must resolve uniquely");
            };
            assert_eq!(
                item.element().attributes().len(),
                1,
                "unexpected reference item metadata"
            );
            let lines: Vec<_> = item
                .ordered_content()
                .consumed()
                .iter()
                .filter_map(|entry| match entry {
                    PobContentEntry::Text { text, .. } => Some(text.as_str()),
                    PobContentEntry::Element { .. } => None,
                })
                .flat_map(str::lines)
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect();
            assert_eq!(
                lines,
                expected.pob_export_lines(),
                "reference equipped item {id} payload changed"
            );
            assert_eq!(
                item.children().len(),
                expected.weapon().map_or_else(
                    || expected.modifier_lines().len(),
                    |weapon| weapon.modifier_lines().len(),
                ),
                "reference item {id} modifier count changed"
            );
            for (index, range) in item.children().iter().enumerate() {
                assert_eq!(range.kind(), ItemSourceKind::ModRange);
                assert_eq!(range.element().attributes().len(), 2);
                assert_eq!(attribute(range, "id"), (index + 1).to_string());
                assert_eq!(
                    attribute(range, "range"),
                    "0.5",
                    "reference item {id} fixed modifier range changed"
                );
                assert!(range.children().is_empty());
                assert!(!range.element().has_non_whitespace_text());
            }
        }
    }
}
