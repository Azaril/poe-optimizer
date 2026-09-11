//! Lazy, source-bound build selections for connected passive and supplied equipment search.
//! This catalog stores component programs and one source document. It never
//! enumerates the Cartesian product or caches candidate XML/numeric results.
#[path = "controlled_build_evidence.rs"]
mod evidence;
#[path = "controlled_build_template.rs"]
mod template;
#[path = "controlled_build_xml.rs"]
mod xml;
use crate::equipment::{ValidatedEquipmentItem, parse_equipment_item};
use poe_optimizer_core::{candidate::*, evaluation::BuildDocument, options::Scalar};
use poe_optimizer_data::{
    class_tree,
    game_data::{RequirementData, SupportColor},
    passive_allocation::{
        AttributeOption, PassiveAllocationSelection, PassiveViewKey, ResolvedPassiveAllocation,
    },
};
use poe_optimizer_engine::{
    CompiledGameData,
    actor::{
        ActorModifierLayer, ActorQuestSelection, ActorScratch, CompiledActorModifiers,
        PreparedActorResources, ReceivingScenario,
    },
    character::CharacterInput,
    spark::SparkQuestRewards,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
pub use template::{SourceBuildTemplate, parse_passive_allocation};
use thiserror::Error;
const SKILL_SLOT: &str = "pob-group-1";
const MAX_ITEMS: usize = 256;
/// Implementation input bound, independent of user-supplied game point budgets.
pub const MAX_SELECTED_PASSIVES: usize = 240;
type Result<T> = std::result::Result<T, BuildCatalogError>;
#[derive(Debug, Error)]
pub enum BuildCatalogError {
    #[error("unsupported controlled build source: {0}")]
    Source(String),
    #[error("invalid controlled build selection: {0}")]
    Structural(String),
    #[error("actor requirement preparation failed: {0}")]
    ActorPreparation(String),
    #[error("controlled build requirements not met: {0:?}")]
    Requirements(RequirementAssessment),
    #[error("controlled build ownership mismatch")]
    Ownership,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateProfile {
    Spark,
    Mace,
}
/// Ordinary core candidates retain their schema. Attribute choices participate
/// explicitly in this wrapper's equality, ordering, serialization and digest.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildSelection {
    pub candidate: Candidate,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attribute_options: BTreeMap<u32, AttributeOption>,
}
impl BuildSelection {
    pub fn fingerprint(&self) -> String {
        hash(&serde_json::to_vec(self).expect("serializable candidate"))
    }
    pub fn allocation(&self, catalog: &CandidateCatalog) -> Result<PassiveAllocationSelection> {
        let (ascendancy_nodes, ordinary_nodes) =
            self.candidate.passives.iter().copied().partition(|id| {
                catalog
                    .passive_nodes
                    .get(id)
                    .is_some_and(|node| matches!(node.kind, PassiveKind::Ascendancy { .. }))
            });
        Ok(PassiveAllocationSelection {
            class_id: self
                .candidate
                .class_id
                .parse()
                .map_err(|_| BuildCatalogError::Structural("class ID is not u32".into()))?,
            ascendancy_id: self.candidate.ascendancy_id.clone(),
            ordinary_nodes,
            ascendancy_nodes,
            attribute_options: self.attribute_options.clone(),
        })
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EquipmentAlternative {
    pub instance_id: String,
    pub pob_item_id: u32,
    pub item_text: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RequirementValues {
    pub level: u32,
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
}
impl RequirementValues {
    fn include(&mut self, other: &RequirementData) {
        self.level = self.level.max(other.level);
        self.strength = self.strength.max(other.attributes.strength);
        self.dexterity = self.dexterity.max(other.attributes.dexterity);
        self.intelligence = self.intelligence.max(other.attributes.intelligence);
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequirementViolation {
    pub requirement: String,
    pub required: u32,
    pub available: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequirementAssessment {
    pub available: RequirementValues,
    pub required: RequirementValues,
    pub violations: Vec<RequirementViolation>,
}
impl RequirementAssessment {
    pub fn is_legal(&self) -> bool {
        self.violations.is_empty()
    }
}
struct ItemComponent {
    item: ValidatedEquipmentItem,
    program: Option<CompiledActorModifiers>,
    armour: Option<poe_optimizer_engine::armour::PreparedArmour>,
}
impl ItemComponent {
    fn program(&self) -> &CompiledActorModifiers {
        self.armour.as_ref().map_or_else(
            || self.program.as_ref().expect("ordinary item program"),
            |armour| armour.global_program(),
        )
    }
}
/// Cloneable ownership token retaining no catalog, source XML or calculation data.
#[derive(Clone)]
pub struct ControlledBuildBinding(Arc<()>);
impl ControlledBuildBinding {
    pub fn accepts(&self, handle: &AdmittedBuildSelection) -> bool {
        Arc::ptr_eq(&self.0, &handle.0.catalog_binding)
    }
}
/// Immutable component storage shared by domains and native evaluation workers.
pub struct ControlledBuildCatalog {
    compiled: Arc<CompiledGameData>,
    source: SourceBuildTemplate,
    catalog: CandidateCatalog,
    items: BTreeMap<String, ItemComponent>,
    item_ids: BTreeMap<u32, String>,
    supports: BTreeMap<String, String>,
    config_program: CompiledActorModifiers,
    passive_programs: BTreeMap<PassiveViewKey, CompiledActorModifiers>,
    quests: ActorQuestSelection,
    receiving_scenario: ReceivingScenario,
    binding: Arc<()>,
}
#[derive(Debug, Clone, Serialize)]
pub struct BuildCatalogFootprint {
    pub source_xml_bytes: usize,
    pub item_components: usize,
    pub passive_components: usize,
    pub compiled_actor_heap_bytes: usize,
    pub local_armour_components: usize,
    pub local_armour_heap_bytes: usize,
    pub materialized_candidates: usize,
    pub cached_candidate_results: usize,
}
impl ControlledBuildCatalog {
    /// Supplied records are reparsed with this exact selected dataset. Template
    /// inventory joins automatically; explicit matching numeric IDs may name its
    /// physical instances but cannot replace their original payloads silently.
    pub fn new(
        compiled: Arc<CompiledGameData>,
        source: String,
        alternatives: Vec<EquipmentAlternative>,
    ) -> Result<Self> {
        let data = compiled.snapshot().package();
        let source = SourceBuildTemplate::parse(source, data)?;
        source
            .allocation()
            .resolve(compiled.snapshot())
            .map_err(|e| BuildCatalogError::Structural(e.to_string()))?;
        if alternatives.len() > MAX_ITEMS {
            return Err(xml::fail("too many supplied equipment instances"));
        }
        let mut inputs = BTreeMap::new();
        let mut item_ids = BTreeMap::new();
        for alternative in alternatives {
            if alternative.instance_id.trim().is_empty()
                || alternative.instance_id.len() > 128
                || inputs.contains_key(&alternative.instance_id)
                || item_ids.contains_key(&alternative.pob_item_id)
            {
                return Err(xml::fail(
                    "equipment instance and numeric IDs must be distinct bounded identities",
                ));
            }
            let parsed =
                parse_equipment_item(&alternative.item_text, data, alternative.pob_item_id)
                    .map_err(|e| xml::fail(e.to_string()))?;
            item_ids.insert(alternative.pob_item_id, alternative.instance_id.clone());
            inputs.insert(alternative.instance_id, parsed);
        }
        for (id, item) in source.items() {
            if let Some(instance) = item_ids.get(id) {
                if inputs[instance].source_text() != item.source_text() {
                    return Err(xml::fail(
                        "supplied numeric item ID conflicts with exact template instance",
                    ));
                }
            } else {
                let instance = format!("pob-item:{id}");
                if inputs.contains_key(&instance) {
                    return Err(xml::fail(
                        "supplied instance ID conflicts with template inventory",
                    ));
                }
                item_ids.insert(*id, instance.clone());
                inputs.insert(instance, item.clone());
            }
        }
        if inputs.len() > MAX_ITEMS {
            return Err(xml::fail(
                "template and supplied inventory exceed component bound",
            ));
        }
        let mut catalog = class_tree::allocation_candidate_catalog(compiled.snapshot())
            .map_err(|e| BuildCatalogError::Structural(e.to_string()))?;
        for (id, item) in &inputs {
            if source.profile() == TemplateProfile::Spark && item.weapon().is_some() {
                return Err(xml::fail(
                    "Spark supplied-equipment scope admits armour and jewellery only",
                ));
            }
            catalog
                .equipment_slots
                .extend(item.allowed_slots().iter().cloned());
            catalog.items.insert(
                id.clone(),
                ItemInstance {
                    definition_id: item.base_id().into(),
                    payload: payload("pob2-item-text-v2", item.source_text().into()),
                    compatible_slots: item.allowed_slots().iter().cloned().collect(),
                    ..Default::default()
                },
            );
        }
        // Empty supported non-weapon slots remain legal without a supplied item.
        catalog.equipment_slots.extend(
            crate::equipment::EQUIPMENT_SOURCE_ORDER[1..]
                .iter()
                .map(|slot| (*slot).into()),
        );
        if source.profile() == TemplateProfile::Mace {
            catalog.equipment_slots.insert("Weapon 1".into());
        }
        catalog.skill_slots.insert(SKILL_SLOT.into());
        let active_id = format!("pob-active:{}", hash(source.active_xml().as_bytes()));
        let skill_id = match source.profile() {
            TemplateProfile::Spark => &data.spark.skill_id,
            TemplateProfile::Mace => &data.mace.skill_id,
        };
        catalog.active_skills.insert(
            active_id.clone(),
            ActiveSkillInstance {
                definition_id: skill_id.clone(),
                payload: payload("pob2-gem-xml-v1", source.active_xml().into()),
                ..Default::default()
            },
        );
        let mut supports = BTreeMap::new();
        if source.profile() == TemplateProfile::Mace {
            for support in &data.supports {
                let text = template::gem_xml(
                    &support.name,
                    &support.skill_id,
                    &support.game_id,
                    &support.variant_id,
                );
                let id = format!("pob-support:{}", hash(text.as_bytes()));
                supports.insert(support.id.clone(), id.clone());
                catalog.supports.insert(
                    id,
                    SupportInstance {
                        definition_id: support.skill_id.clone(),
                        payload: payload("pob2-gem-xml-v1", text),
                        compatible_active_skill_ids: BTreeSet::from([data.mace.skill_id.clone()]),
                        ..Default::default()
                    },
                );
                catalog
                    .support_definition_limits
                    .insert(support.skill_id.clone(), 1);
            }
        }
        let digest = serde_json::to_vec(&(
            "controlled-build-v1",
            compiled.identity(),
            hash(source.source().as_bytes()),
            &catalog,
            &item_ids,
        ))
        .map_err(|e| xml::fail(e.to_string()))?;
        catalog.identity.content_fingerprint = hash(&digest);
        CandidateDomain::new(catalog.clone(), CandidateConstraints::default())
            .map_err(|e| BuildCatalogError::Structural(e.to_string()))?;
        let config_program = compiled
            .compile_actor_modifiers(source.actor_modifiers().records())
            .map_err(|e| BuildCatalogError::ActorPreparation(e.to_string()))?;
        let mut items = BTreeMap::new();
        for (id, item) in inputs {
            let (program, armour) = if let Some(records) = item.armour_modifiers() {
                let armour = compiled
                    .prepare_armour_with_source(
                        item.base_id(),
                        item.quality(),
                        item.item_level(),
                        &item.modifier_source(),
                        records,
                    )
                    .map_err(|e| BuildCatalogError::ActorPreparation(e.to_string()))?;
                if armour.source_global_records() != item.actor_modifiers() {
                    return Err(xml::fail(
                        "armour global record projection differs from selected data preparation",
                    ));
                }
                (None, Some(armour))
            } else {
                (
                    Some(
                        compiled
                            .compile_actor_modifiers(item.actor_modifiers())
                            .map_err(|e| BuildCatalogError::ActorPreparation(e.to_string()))?,
                    ),
                    None,
                )
            };
            items.insert(
                id,
                ItemComponent {
                    item,
                    program,
                    armour,
                },
            );
        }
        let mut passive_programs = BTreeMap::new();
        for view in &data.passive_effects {
            let program = compiled
                .compile_actor_modifiers(&view.actor_modifiers)
                .map_err(|e| BuildCatalogError::ActorPreparation(e.to_string()))?;
            if passive_programs.insert(view.key.clone(), program).is_some() {
                return Err(xml::fail("duplicate admitted passive program"));
            }
        }
        let enabled = std::array::from_fn(|index| {
            match source.config().get(&data.quests.config_keys[index]) {
                Some(Scalar::Boolean(v)) => *v,
                None => data.quests.default_enabled[index],
                _ => unreachable!("validated quest scalar"),
            }
        });
        let mut quests = compiled.actor_quest_selection(SparkQuestRewards::from_enabled(enabled));
        for (index, quest) in data.actor.spirit_quests.iter().enumerate() {
            if let Some(Scalar::Boolean(v)) = source.config().get(&quest.config_key) {
                quests.spirit[index] = *v;
            }
        }
        let penalty = match source.config().get("resistancePenalty") {
            Some(Scalar::Number(value)) => *value,
            None => data.encounters.default_resistance_penalty,
            _ => unreachable!("validated resistance penalty"),
        };
        let receiving_scenario =
            compiled.receiving_scenario(SparkQuestRewards::from_enabled(enabled), penalty);
        Ok(Self {
            compiled,
            source,
            catalog,
            items,
            item_ids,
            supports,
            config_program,
            passive_programs,
            quests,
            receiving_scenario,
            binding: Arc::new(()),
        })
    }
    /// Authored receiving configuration or equipment, including source implicits.
    /// Migrated passive records do not expand the legacy problem source scope.
    pub fn uses_local_armour_scope(&self) -> bool {
        self.items
            .values()
            .any(|component| component.armour.is_some())
    }
    pub fn uses_action_speed_scope(&self) -> bool {
        self.source.actor_modifiers().uses_action_speed()
            || self
                .items
                .values()
                .any(|component| component.item.uses_action_speed())
    }
    pub fn uses_movement_scope(&self) -> bool {
        self.source.actor_modifiers().uses_movement()
            || self.items.values().any(|component| {
                component.item.uses_movement()
                    || component
                        .armour
                        .as_ref()
                        .is_some_and(|armour| !armour.generated_global_records().is_empty())
            })
    }
    pub fn uses_receiving_defence_scope(&self) -> bool {
        self.source.actor_modifiers().uses_receiving_defence()
            || self.items.values().any(|component| {
                component
                    .item
                    .actor_modifiers()
                    .iter()
                    .any(|record| record.stat.is_receiving_defence())
            })
    }
    pub fn compiled(&self) -> &Arc<CompiledGameData> {
        &self.compiled
    }
    pub fn data(&self) -> &Arc<CompiledGameData> {
        &self.compiled
    }
    pub fn binding(&self) -> ControlledBuildBinding {
        ControlledBuildBinding(self.binding.clone())
    }
    pub fn template(&self) -> &SourceBuildTemplate {
        &self.source
    }
    pub fn source(&self) -> &SourceBuildTemplate {
        &self.source
    }
    pub fn catalog(&self) -> &CandidateCatalog {
        &self.catalog
    }
    pub fn quests(&self) -> ActorQuestSelection {
        self.quests
    }
    pub fn items(&self) -> impl Iterator<Item = (&str, &ValidatedEquipmentItem)> {
        self.items
            .iter()
            .map(|(id, item)| (id.as_str(), &item.item))
    }
    pub fn item(&self, id: &str) -> Option<&ValidatedEquipmentItem> {
        self.items.get(id).map(|v| &v.item)
    }
    fn selected_armour<'a>(
        &'a self,
        selection: &BuildSelection,
    ) -> poe_optimizer_engine::armour::ArmourSlots<'a> {
        let selected = |slot| {
            selection
                .candidate
                .equipment
                .get(slot)
                .and_then(|id| self.items[id].armour.as_ref())
        };
        poe_optimizer_engine::armour::ArmourSlots {
            helmet: selected("Helmet"),
            gloves: selected("Gloves"),
            boots: selected("Boots"),
            body_armour: selected("Body Armour"),
        }
    }
    fn local_armour_evidence(&self, selection: &BuildSelection) -> serde_json::Value {
        crate::actor_assembly::local_armour_evidence(
            selection
                .candidate
                .equipment
                .iter()
                .filter_map(|(slot, id)| {
                    let component = &self.items[id];
                    component
                        .armour
                        .as_ref()
                        .map(|armour| (slot.as_str(), &component.item, armour))
                }),
        )
    }
    pub fn support_instance(&self, key: &str) -> Option<&str> {
        self.supports.get(key).map(String::as_str)
    }
    /// Temporary source for native support admission during setup. This is not a legal candidate or evaluation.
    pub fn support_preparation_build(&self, key: &str) -> Result<BuildDocument> {
        if !self.supports.contains_key(key) {
            return Err(BuildCatalogError::Source(
                "unknown candidate support axis".into(),
            ));
        }
        self.source
            .materialize_supports(&[key.to_owned()], self.compiled.snapshot().package())
    }
    pub fn materialize(&self, handle: &AdmittedBuildSelection) -> Result<BuildDocument> {
        if !handle.bound_to(self) {
            return Err(BuildCatalogError::Ownership);
        }
        let items = handle
            .selection()
            .candidate
            .equipment
            .iter()
            .map(|(slot, id)| (slot.clone(), &self.items[id].item))
            .collect();
        self.source.materialize(
            &handle.0.tree,
            &items,
            &handle.0.supports,
            self.compiled.snapshot().package(),
        )
    }
    pub fn source_selection(&self) -> BuildSelection {
        let allocation = self.source.allocation();
        BuildSelection {
            candidate: Candidate {
                catalog: self.catalog.identity.clone(),
                class_id: allocation.class_id.to_string(),
                ascendancy_id: allocation.ascendancy_id.clone(),
                passives: allocation
                    .ordinary_nodes
                    .union(&allocation.ascendancy_nodes)
                    .copied()
                    .collect(),
                equipment: self
                    .source
                    .equipment()
                    .iter()
                    .map(|(slot, id)| (slot.clone(), self.item_ids[id].clone()))
                    .collect(),
                skills: BTreeMap::from([(
                    SKILL_SLOT.into(),
                    SkillAssignment {
                        active_instance_id: self
                            .catalog
                            .active_skills
                            .keys()
                            .next()
                            .expect("one active")
                            .clone(),
                        support_instance_ids: self
                            .source
                            .support_order()
                            .iter()
                            .map(|key| self.supports[key].clone())
                            .collect(),
                    },
                )]),
            },
            attribute_options: allocation.attribute_options.clone(),
        }
    }
    pub fn footprint(&self) -> BuildCatalogFootprint {
        BuildCatalogFootprint {
            source_xml_bytes: self.source.source().len(),
            item_components: self.items.len(),
            passive_components: self.passive_programs.len(),
            local_armour_components: self.items.values().filter(|v| v.armour.is_some()).count(),
            local_armour_heap_bytes: self
                .items
                .values()
                .filter_map(|v| v.armour.as_ref())
                .map(|armour| armour.owned_heap_bytes())
                .sum(),
            compiled_actor_heap_bytes: self.config_program.owned_heap_bytes()
                + self
                    .items
                    .values()
                    .filter_map(|v| v.program.as_ref())
                    .map(|program| program.owned_heap_bytes())
                    .sum::<usize>()
                + self
                    .passive_programs
                    .values()
                    .map(CompiledActorModifiers::owned_heap_bytes)
                    .sum::<usize>(),
            materialized_candidates: 0,
            cached_candidate_results: 0,
        }
    }
    fn selected_supports(&self, selection: &BuildSelection) -> Result<Vec<String>> {
        let skill =
            selection.candidate.skills.get(SKILL_SLOT).ok_or_else(|| {
                BuildCatalogError::Structural("selected skill group missing".into())
            })?;
        let mut keys = Vec::new();
        for id in &skill.support_instance_ids {
            keys.push(
                self.supports
                    .iter()
                    .find_map(|(key, value)| (value == id).then_some(key.clone()))
                    .ok_or_else(|| {
                        BuildCatalogError::Structural("support instance unavailable".into())
                    })?,
            );
        }
        keys.sort();
        if self.source.profile() == TemplateProfile::Mace {
            self.compiled
                .snapshot()
                .package()
                .validate_mace_support_loadout(&keys)
                .map_err(|e| BuildCatalogError::Structural(e.to_string()))?;
        } else if !keys.is_empty() {
            return Err(BuildCatalogError::Structural(
                "Spark support scope is empty".into(),
            ));
        }
        Ok(keys)
    }
}
/// Constraints remain independent from available graph/data and source identity.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AttributeOptionLocks {
    #[serde(default)]
    pub required: BTreeMap<u32, AttributeOption>,
    #[serde(default)]
    pub forbidden: BTreeMap<u32, BTreeSet<AttributeOption>>,
}
pub struct ControlledBuildDomain {
    catalog: Arc<ControlledBuildCatalog>,
    rules: CandidateDomain,
    attribute_locks: AttributeOptionLocks,
    binding: Arc<()>,
}
impl ControlledBuildDomain {
    pub fn new(
        catalog: Arc<ControlledBuildCatalog>,
        constraints: CandidateConstraints,
        attribute_locks: AttributeOptionLocks,
    ) -> Result<Self> {
        let option_is_available = |id: &u32, option: &AttributeOption| {
            catalog
                .compiled
                .snapshot()
                .tree()
                .allocation_nodes
                .get(id)
                .is_some_and(|node| node.attribute_options.contains_key(option))
                && catalog
                    .compiled
                    .snapshot()
                    .package()
                    .passive_view_effects(&PassiveViewKey {
                        physical_node_id: *id,
                        selector:
                            poe_optimizer_data::passive_allocation::PassiveViewSelector::Attribute {
                                option: *option,
                            },
                    })
                    .is_some()
        };
        for (id, option) in &attribute_locks.required {
            if !option_is_available(id, option)
                || constraints.locks.unallocated_passives.contains(id)
                || attribute_locks
                    .forbidden
                    .get(id)
                    .is_some_and(|set| set.contains(option))
            {
                return Err(BuildCatalogError::Structural(
                    "unknown or contradictory attribute choice lock".into(),
                ));
            }
        }
        for (id, options) in &attribute_locks.forbidden {
            if !AttributeOption::ALL
                .iter()
                .any(|option| option_is_available(id, option))
                || options
                    .iter()
                    .any(|option| !option_is_available(id, option))
            {
                return Err(BuildCatalogError::Structural(
                    "unknown forbidden attribute node or option".into(),
                ));
            }
        }
        let rules = CandidateDomain::new(catalog.catalog.clone(), constraints)
            .map_err(|e| BuildCatalogError::Structural(e.to_string()))?;
        Ok(Self {
            catalog,
            rules,
            attribute_locks,
            binding: Arc::new(()),
        })
    }
    pub fn catalog(&self) -> &Arc<ControlledBuildCatalog> {
        &self.catalog
    }
    pub fn rules(&self) -> &CandidateDomain {
        &self.rules
    }
    pub fn validate_structure(
        &self,
        selection: &BuildSelection,
    ) -> Result<ResolvedPassiveAllocation> {
        if selection.candidate.passives.len() > MAX_SELECTED_PASSIVES
            || selection.attribute_options.len() > MAX_SELECTED_PASSIVES
        {
            return Err(BuildCatalogError::Structural(
                "selected passive count exceeds preparation bound".into(),
            ));
        }
        let validation = self.rules.validate(&selection.candidate);
        if !validation.is_searchable() {
            return Err(BuildCatalogError::Structural(format!("{validation:?}")));
        }
        if selection.candidate.skills.len() != 1
            || !selection.candidate.skills.contains_key(SKILL_SLOT)
        {
            return Err(BuildCatalogError::Structural(
                "exactly one selected profile skill is required".into(),
            ));
        }
        if self.catalog.source.profile() == TemplateProfile::Mace
            && !selection.candidate.equipment.contains_key("Weapon 1")
        {
            return Err(BuildCatalogError::Structural(
                "Mace requires a main hand weapon".into(),
            ));
        }
        for (id, option) in &self.attribute_locks.required {
            if selection.attribute_options.get(id) != Some(option) {
                return Err(BuildCatalogError::Structural(
                    "required attribute option changed".into(),
                ));
            }
        }
        for (id, options) in &self.attribute_locks.forbidden {
            if selection
                .attribute_options
                .get(id)
                .is_some_and(|option| options.contains(option))
            {
                return Err(BuildCatalogError::Structural(
                    "forbidden attribute option selected".into(),
                ));
            }
        }
        self.catalog.selected_supports(selection)?;
        selection
            .allocation(&self.catalog.catalog)?
            .resolve(self.catalog.compiled.snapshot())
            .map_err(|e| BuildCatalogError::Structural(e.to_string()))
    }
    /// Resolves one selection without source XML or result-cache growth. Ordinary
    /// scalar calculation failures are retained separately from requirement legality.
    pub fn prepare(
        &self,
        selection: BuildSelection,
        scratch: &mut ActorScratch,
    ) -> Result<PreparedBuildSelection> {
        let tree = self.validate_structure(&selection)?;
        let character = self
            .catalog
            .compiled
            .character_from_allocation(&tree)
            .map_err(|e| e.to_string());
        let actor_character = match &character {
            Ok(character) => *character,
            Err(_) => self
                .catalog
                .compiled
                .class_character_from_allocation(&tree)
                .map_err(|e| BuildCatalogError::ActorPreparation(e.to_string()))?,
        };
        let mut programs =
            Vec::with_capacity(1 + selection.candidate.equipment.len() + tree.views.len());
        programs.push(&self.catalog.config_program);
        // Canonical supported PoB slot order is equipment before passives. Each
        // source fragment is part of one local layer, never a separate parent.
        for slot in crate::equipment::EQUIPMENT_SOURCE_ORDER {
            if let Some(id) = selection.candidate.equipment.get(slot) {
                programs.push(self.catalog.items[id].program());
            }
        }
        for view in &tree.views {
            programs.push(
                self.catalog
                    .passive_programs
                    .get(&view.source.key)
                    .ok_or_else(|| {
                        BuildCatalogError::Structural("passive program is not admitted".into())
                    })?,
            );
        }
        let actor = self
            .catalog
            .compiled
            .evaluate_actor_with_armour(
                self.catalog.source.level(),
                self.catalog.quests,
                self.catalog.receiving_scenario,
                &actor_character,
                &[ActorModifierLayer {
                    programs: &programs,
                }],
                self.catalog.selected_armour(&selection),
                scratch,
            )
            .map_err(|e| BuildCatalogError::ActorPreparation(e.to_string()))?;
        let supports = self.catalog.selected_supports(&selection)?;
        let requirements = self.assess(&selection, &supports, &actor)?;
        Ok(PreparedBuildSelection {
            catalog_binding: self.catalog.binding.clone(),
            domain_binding: self.binding.clone(),
            selection: Arc::new(selection),
            tree: Arc::new(tree),
            character,
            actor,
            requirements,
            supports,
        })
    }
    pub fn requirements(
        &self,
        selection: &BuildSelection,
        scratch: &mut ActorScratch,
    ) -> Result<RequirementAssessment> {
        Ok(self.prepare(selection.clone(), scratch)?.requirements)
    }
    pub fn admit(
        &self,
        selection: BuildSelection,
        scratch: &mut ActorScratch,
    ) -> Result<AdmittedBuildSelection> {
        let prepared = self.prepare(selection, scratch)?;
        if !prepared.requirements.is_legal() {
            return Err(BuildCatalogError::Requirements(prepared.requirements));
        }
        Ok(AdmittedBuildSelection(prepared))
    }
    pub fn validate_handle(&self, handle: &AdmittedBuildSelection) -> Result<()> {
        if Arc::ptr_eq(&self.binding, &handle.0.domain_binding)
            && Arc::ptr_eq(&self.catalog.binding, &handle.0.catalog_binding)
        {
            Ok(())
        } else {
            Err(BuildCatalogError::Ownership)
        }
    }
    pub fn materialize(&self, handle: &AdmittedBuildSelection) -> Result<BuildDocument> {
        self.validate_handle(handle)?;
        let items = handle
            .selection()
            .candidate
            .equipment
            .iter()
            .map(|(slot, id)| (slot.clone(), &self.catalog.items[id].item))
            .collect();
        self.catalog.source.materialize(
            &handle.0.tree,
            &items,
            &handle.0.supports,
            self.catalog.compiled.snapshot().package(),
        )
    }
    fn assess(
        &self,
        selection: &BuildSelection,
        supports: &[String],
        actor: &PreparedActorResources,
    ) -> Result<RequirementAssessment> {
        let attrs = actor.values().attributes;
        let whole = |value: f64| {
            if value.is_finite()
                && value.fract() == 0.0
                && (0.0..=f64::from(u32::MAX)).contains(&value)
            {
                Ok(value as u32)
            } else {
                Err(BuildCatalogError::ActorPreparation(
                    "resolved attributes must be finite whole u32 values".into(),
                ))
            }
        };
        let available = RequirementValues {
            level: self.catalog.source.level(),
            strength: whole(attrs.strength)?,
            dexterity: whole(attrs.dexterity)?,
            intelligence: whole(attrs.intelligence)?,
        };
        let mut required = RequirementValues {
            level: 0,
            strength: 0,
            dexterity: 0,
            intelligence: 0,
        };
        let data = self.catalog.compiled.snapshot().package();
        for id in selection.candidate.equipment.values() {
            required.include(self.catalog.items[id].item.requirements());
        }
        required.include(match self.catalog.source.profile() {
            TemplateProfile::Spark => &data.spark.requirements,
            TemplateProfile::Mace => &data.mace.requirements,
        });
        let mut costs = [0u32; 3];
        for key in supports {
            let support = data.support(key).expect("validated support");
            required.include(&support.requirements);
            let (index, cost) = match support.color {
                SupportColor::Red => (0, data.mace.support_attribute_costs.strength),
                SupportColor::Green => (1, data.mace.support_attribute_costs.dexterity),
                SupportColor::Blue => (2, data.mace.support_attribute_costs.intelligence),
            };
            costs[index] = costs[index].checked_add(cost).ok_or_else(|| {
                BuildCatalogError::ActorPreparation("support requirement overflow".into())
            })?;
        }
        required.strength = required.strength.max(costs[0]);
        required.dexterity = required.dexterity.max(costs[1]);
        required.intelligence = required.intelligence.max(costs[2]);
        let violations = [
            ("level", required.level, available.level),
            ("strength", required.strength, available.strength),
            ("dexterity", required.dexterity, available.dexterity),
            (
                "intelligence",
                required.intelligence,
                available.intelligence,
            ),
        ]
        .into_iter()
        .filter(|(_, required, available)| required > available)
        .map(|(name, required, available)| RequirementViolation {
            requirement: name.into(),
            required,
            available,
        })
        .collect();
        Ok(RequirementAssessment {
            available,
            required,
            violations,
        })
    }
}
/// One bounded preparation, not a cache entry retained by the catalog.
#[derive(Clone)]
pub struct PreparedBuildSelection {
    catalog_binding: Arc<()>,
    domain_binding: Arc<()>,
    selection: Arc<BuildSelection>,
    tree: Arc<ResolvedPassiveAllocation>,
    character: std::result::Result<CharacterInput, String>,
    actor: PreparedActorResources,
    requirements: RequirementAssessment,
    supports: Vec<String>,
}
impl PreparedBuildSelection {
    pub fn selection(&self) -> &BuildSelection {
        &self.selection
    }
    pub fn tree(&self) -> &ResolvedPassiveAllocation {
        &self.tree
    }
    pub fn actor(&self) -> &PreparedActorResources {
        &self.actor
    }
    pub fn character(&self) -> std::result::Result<&CharacterInput, &str> {
        self.character.as_ref().map_err(String::as_str)
    }
    pub fn requirements(&self) -> &RequirementAssessment {
        &self.requirements
    }
    pub fn support_keys(&self) -> &[String] {
        &self.supports
    }
}
#[derive(Clone)]
pub struct AdmittedBuildSelection(PreparedBuildSelection);
impl AdmittedBuildSelection {
    pub fn prepared(&self) -> &PreparedBuildSelection {
        &self.0
    }
    pub fn selection(&self) -> &BuildSelection {
        self.0.selection()
    }
    pub fn character(&self) -> std::result::Result<&CharacterInput, &str> {
        self.0.character()
    }
    pub fn actor(&self) -> &PreparedActorResources {
        self.0.actor()
    }
    pub fn tree(&self) -> &ResolvedPassiveAllocation {
        self.0.tree()
    }
    pub fn requirements(&self) -> &RequirementAssessment {
        self.0.requirements()
    }
    pub fn support_keys(&self) -> &[String] {
        self.0.support_keys()
    }
    pub fn bound_to(&self, catalog: &ControlledBuildCatalog) -> bool {
        Arc::ptr_eq(&self.0.catalog_binding, &catalog.binding)
    }
}
impl PartialEq for AdmittedBuildSelection {
    fn eq(&self, other: &Self) -> bool {
        self.selection() == other.selection()
    }
}
impl Eq for AdmittedBuildSelection {}
impl PartialOrd for AdmittedBuildSelection {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for AdmittedBuildSelection {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.selection().cmp(other.selection())
    }
}
impl std::fmt::Debug for AdmittedBuildSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdmittedBuildSelection")
            .field("selection", self.selection())
            .field("deferred_scalar_error", &self.character().err())
            .finish()
    }
}
impl Serialize for AdmittedBuildSelection {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.selection().serialize(serializer)
    }
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn payload(format: &str, content: String) -> ExactPayload {
    ExactPayload {
        format: format.into(),
        sha256: hash(content.as_bytes()),
        content,
    }
}
#[cfg(test)]
#[path = "controlled_build_tests.rs"]
mod tests;
