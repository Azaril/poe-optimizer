//! Strict immutable Spark/Mace source projection and lazy materialization spans.
use super::xml::*;
use super::{BuildCatalogError, Result, TemplateProfile};
use crate::{
    actor_modifiers::{ValidatedActorModifiers, parse_actor_configuration},
    equipment::{ValidatedEquipmentItem, parse_equipment_item_xml},
};
use poe_optimizer_core::{
    evaluation::{BuildDocument, BuildFormat},
    options::Scalar,
};
use poe_optimizer_data::{
    game_data::GameDataPackage,
    passive_allocation::{AttributeOption, PassiveAllocationSelection, ResolvedPassiveAllocation},
};
use roxmltree::Node;
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    sync::Arc,
};

pub(super) const CONFIG_NAMES: &[&str] = &[
    "enemyIsBoss",
    "enemyLevel",
    "enemyFireResist",
    "enemyColdResist",
    "enemyLightningResist",
    "enemyChaosResist",
    "enemyDamageType",
    "enemyPhysicalDamage",
    "enemyFireDamage",
    "enemyColdDamage",
    "enemyLightningDamage",
    "enemyChaosDamage",
    "enemyPhysicalOverwhelm",
    "enemyFirePen",
    "enemyColdPen",
    "enemyLightningPen",
    "enemyCritChance",
    "enemySpeed",
    "conditionEnemyShocked",
    "conditionEnemyChilled",
    "conditionEnemyIgnited",
    "conditionCritRecently",
    "conditionBeenHitRecently",
    "multiplierNearbyEnemies",
    "multiplierNearbyRareOrUniqueEnemies",
];

#[derive(Debug)]
struct Spans {
    fields: BTreeMap<String, Range<usize>>,
    ascendancy_insert: usize,
    spec: ElementSpan,
    overrides: Option<Range<usize>>,
    items: ElementSpan,
    item_ranges: BTreeMap<u32, Range<usize>>,
    item_set: ElementSpan,
    slot_ranges: BTreeMap<String, Range<usize>>,
    support_ranges: Vec<Range<usize>>,
    support_insert: usize,
    cached_outputs: Vec<Range<usize>>,
}
/// Strict source projection; numeric/passive capability admission is performed
/// separately by the selected catalog. Original bytes remain immutable.
#[derive(Debug)]
pub struct SourceBuildTemplate {
    source: Arc<str>,
    profile: TemplateProfile,
    level: u32,
    config: BTreeMap<String, Scalar>,
    actor_modifiers: Arc<ValidatedActorModifiers>,
    allocation: PassiveAllocationSelection,
    items: BTreeMap<u32, ValidatedEquipmentItem>,
    equipment: BTreeMap<String, u32>,
    active_xml: String,
    support_order: Vec<String>,
    spans: Spans,
}
impl SourceBuildTemplate {
    pub fn parse(source: String, data: &GameDataPackage) -> Result<Self> {
        let doc = document(&source)?;
        let root = doc.root_element();
        if !root.has_tag_name("PathOfBuilding2") {
            return Err(fail("expected PathOfBuilding2"));
        }
        only(
            root,
            &[],
            &["Build", "Tree", "Skills", "Items", "Config", "Notes"],
        )?;
        let build = child(root, "Build")?;
        only(
            build,
            &[
                "level",
                "className",
                "ascendClassName",
                "targetVersion",
                "characterLevelAutoMode",
                "mainSocketGroup",
                "viewMode",
            ],
            &["PlayerStat", "MinionStat"],
        )?;
        fixed(
            build,
            &[
                ("targetVersion", "0_1"),
                ("characterLevelAutoMode", "false"),
                ("mainSocketGroup", "1"),
            ],
        )?;
        let level = integer(
            build
                .attribute("level")
                .ok_or_else(|| fail("missing level"))?,
        )?;
        if !(1..=100).contains(&level) {
            return Err(fail("character level must be1..100"));
        }
        let mut cached_outputs = Vec::new();
        for node in build.children().filter(Node::is_element) {
            only(node, &["stat", "value"], &[])?;
            cached_outputs.push(node.range());
        }
        let tree = child(root, "Tree")?;
        only(tree, &["activeSpec"], &["Spec"])?;
        fixed(tree, &[("activeSpec", "1")])?;
        let spec = child(tree, "Spec")?;
        only(
            spec,
            &[
                "title",
                "classId",
                "classInternalId",
                "ascendClassId",
                "ascendancyInternalId",
                "treeVersion",
                "nodes",
                "masteryEffects",
            ],
            &["Overrides"],
        )?;
        fixed(
            spec,
            &[
                ("treeVersion", &data.tree.source.tree_version),
                ("masteryEffects", ""),
            ],
        )?;
        let allocation = parse_passive_allocation(build, spec, data)?;
        let mut fields = BTreeMap::new();
        for (node, names) in [
            (build, &["className", "ascendClassName"][..]),
            (
                spec,
                &[
                    "classId",
                    "classInternalId",
                    "ascendClassId",
                    "ascendancyInternalId",
                    "nodes",
                ][..],
            ),
        ] {
            for name in names {
                if let Some(attr) = node.attributes().find(|a| a.name() == *name) {
                    fields.insert((*name).to_owned(), attr.range_value());
                }
            }
        }
        let ascendancy_insert = spec
            .attributes()
            .next()
            .ok_or_else(|| fail("Spec attributes missing"))?
            .range()
            .start;
        let skills = child(root, "Skills")?;
        only(
            skills,
            &["activeSkillSet", "defaultGemLevel", "defaultGemQuality"],
            &["SkillSet"],
        )?;
        fixed(
            skills,
            &[
                ("activeSkillSet", "1"),
                ("defaultGemLevel", "normalMaximum"),
                ("defaultGemQuality", "0"),
            ],
        )?;
        let set = child(skills, "SkillSet")?;
        only(set, &["id", "title"], &["Skill"])?;
        fixed(set, &[("id", "1")])?;
        let skill = child(set, "Skill")?;
        only(
            skill,
            &[
                "enabled",
                "includeInFullDPS",
                "groupCount",
                "label",
                "mainActiveSkill",
                "mainActiveSkillCalcs",
            ],
            &["Gem"],
        )?;
        fixed(
            skill,
            &[
                ("enabled", "true"),
                ("includeInFullDPS", "true"),
                ("groupCount", "1"),
                ("mainActiveSkill", "1"),
                ("mainActiveSkillCalcs", "1"),
            ],
        )?;
        let gems: Vec<_> = skill.children().filter(Node::is_element).collect();
        let main = *gems
            .first()
            .ok_or_else(|| fail("one main gem is required"))?;
        let profile = match main.attribute("skillId") {
            Some(id) if id == data.mace.skill_id => TemplateProfile::Mace,
            Some(id) if id == data.spark.skill_id => TemplateProfile::Spark,
            _ => {
                return Err(fail(
                    "main skill must be selected-data Spark or Mace Strike",
                ));
            }
        };
        if gems.len()
            > if profile == TemplateProfile::Mace {
                3
            } else {
                1
            }
        {
            return Err(fail("unsupported active/support gem count"));
        }
        let (name, id, game, variant) = match profile {
            TemplateProfile::Mace => (
                &data.mace.name,
                &data.mace.skill_id,
                &data.mace.game_id,
                &data.mace.variant_id,
            ),
            TemplateProfile::Spark => (
                &data.spark.name,
                &data.spark.skill_id,
                &data.spark.game_id,
                &data.spark.variant_id,
            ),
        };
        validate_gem(main, name, id, game, variant)?;
        let mut support_order = Vec::new();
        for gem in &gems[1..] {
            let support = data
                .supports
                .iter()
                .find(|s| Some(s.skill_id.as_str()) == gem.attribute("skillId"))
                .ok_or_else(|| fail("unknown support identity"))?;
            validate_gem(
                *gem,
                &support.name,
                &support.skill_id,
                &support.game_id,
                &support.variant_id,
            )?;
            support_order.push(support.id.clone());
        }
        if profile == TemplateProfile::Mace {
            data.validate_mace_support_loadout(&support_order)
                .map_err(|e| fail(e.to_string()))?;
        }
        let active_xml = source[main.range()].to_owned();
        let support_ranges = gems[1..].iter().map(Node::range).collect();
        let support_insert = main.range().end;
        let items_node = child(root, "Items")?;
        only(items_node, &["activeItemSet"], &["Item", "ItemSet"])?;
        fixed(items_node, &[("activeItemSet", "1")])?;
        let item_set = child(items_node, "ItemSet")?;
        only(item_set, &["id", "title", "useSecondWeaponSet"], &["Slot"])?;
        fixed(item_set, &[("id", "1")])?;
        if (profile == TemplateProfile::Mace
            && item_set.attribute("useSecondWeaponSet") != Some("false"))
            || item_set
                .attribute("useSecondWeaponSet")
                .is_some_and(|v| v != "false")
        {
            return Err(fail("second weapon set is unsupported"));
        }
        let mut items = BTreeMap::new();
        let mut item_ranges = BTreeMap::new();
        for node in items_node.children().filter(|n| n.has_tag_name("Item")) {
            let item = parse_equipment_item_xml(node, data).map_err(|e| fail(e.to_string()))?;
            let id = item.pob_item_id();
            if items.insert(id, item).is_some() {
                return Err(fail("duplicate numeric item ID"));
            }
            item_ranges.insert(id, node.range());
        }
        let mut equipment = BTreeMap::new();
        let mut slot_ranges = BTreeMap::new();
        for slot in item_set.children().filter(Node::is_element) {
            only(slot, &["name", "itemId"], &[])?;
            let name = slot
                .attribute("name")
                .ok_or_else(|| fail("slot requires name"))?
                .to_owned();
            if !crate::equipment::EQUIPMENT_SOURCE_ORDER.contains(&name.as_str()) {
                return Err(fail("equipment slot is outside the admitted profile"));
            }
            let id = integer(
                slot.attribute("itemId")
                    .ok_or_else(|| fail("slot requires itemId"))?,
            )?;
            if slot_ranges.insert(name.clone(), slot.range()).is_some() {
                return Err(fail("duplicate equipment slot"));
            }
            if id != 0 {
                let item = items
                    .get(&id)
                    .ok_or_else(|| fail("slot references missing item"))?;
                if !item.allowed_slots().contains(&name) {
                    return Err(fail("item is incompatible with source slot"));
                }
                equipment.insert(name, id);
            }
        }
        if profile == TemplateProfile::Mace && !equipment.contains_key("Weapon 1") {
            return Err(fail("Mace profile requires main hand weapon"));
        }
        if profile == TemplateProfile::Spark && equipment.contains_key("Weapon 1") {
            return Err(fail(
                "Spark equipment scope admits armour and jewellery only",
            ));
        }
        if equipment.values().collect::<BTreeSet<_>>().len() != equipment.len() {
            return Err(fail("one item instance cannot occupy multiple slots"));
        }
        let config_node = child(root, "Config")?;
        only(config_node, &["activeConfigSet"], &["ConfigSet"])?;
        fixed(config_node, &[("activeConfigSet", "1")])?;
        let config_set = child(config_node, "ConfigSet")?;
        only(
            config_set,
            &["id", "title"],
            &["Input", "CustomModifierBlock"],
        )?;
        fixed(config_set, &[("id", "1")])?;
        let actor_modifiers =
            Arc::new(parse_actor_configuration(config_set, data).map_err(|e| fail(e.to_string()))?);
        if data
            .quests
            .config_keys
            .iter()
            .map(String::as_str)
            .chain(
                data.actor
                    .spirit_quests
                    .iter()
                    .map(|q| q.config_key.as_str()),
            )
            .any(|key| {
                CONFIG_NAMES.contains(&key)
                    || [
                        "resistancePenalty",
                        "enemyArmour",
                        "enemyEvasion",
                        "customMods",
                    ]
                    .contains(&key)
            })
        {
            return Err(fail("quest keys overlap actor/encounter inputs"));
        }
        let mut config = BTreeMap::new();
        for input in config_set.children().filter(|n| n.has_tag_name("Input")) {
            if input.attribute("name") == Some("customMods") {
                continue;
            }
            let name = input
                .attribute("name")
                .ok_or_else(|| fail("configuration requires name"))?;
            let value = scalar(input)?;
            validate_config(name, &value, profile, data)?;
            if config.insert(name.to_owned(), value).is_some() {
                return Err(fail("duplicate config input"));
            }
        }
        if CONFIG_NAMES.iter().any(|key| !config.contains_key(*key))
            || (profile == TemplateProfile::Mace && !config.contains_key("enemyArmour"))
        {
            return Err(fail("all supported encounter inputs must be explicit"));
        }
        if [
            "enemyPhysicalDamage",
            "enemyFireDamage",
            "enemyColdDamage",
            "enemyLightningDamage",
            "enemyChaosDamage",
        ]
        .iter()
        .all(|key| config[*key] == Scalar::Number(0.0))
        {
            return Err(fail("incoming hit needs a positive component"));
        }
        let notes: Vec<_> = root
            .children()
            .filter(|n| n.has_tag_name("Notes"))
            .collect();
        if notes.len() > 1
            || notes
                .iter()
                .any(|n| n.attributes().len() != 0 || n.children().any(|child| child.is_element()))
        {
            return Err(fail("unsupported Notes structure"));
        }
        let spans = Spans {
            fields,
            ascendancy_insert,
            spec: ElementSpan::of(spec)?,
            overrides: spec
                .children()
                .find(|n| n.has_tag_name("Overrides"))
                .map(|n| n.range()),
            items: ElementSpan::of(items_node)?,
            item_ranges,
            item_set: ElementSpan::of(item_set)?,
            slot_ranges,
            support_ranges,
            support_insert,
            cached_outputs,
        };
        Ok(Self {
            source: Arc::from(source),
            profile,
            level,
            config,
            actor_modifiers,
            allocation,
            items,
            equipment,
            active_xml,
            support_order,
            spans,
        })
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn profile(&self) -> TemplateProfile {
        self.profile
    }
    pub fn level(&self) -> u32 {
        self.level
    }
    pub fn config(&self) -> &BTreeMap<String, Scalar> {
        &self.config
    }
    pub fn actor_modifiers(&self) -> &Arc<ValidatedActorModifiers> {
        &self.actor_modifiers
    }
    pub fn allocation(&self) -> &PassiveAllocationSelection {
        &self.allocation
    }
    pub fn items(&self) -> &BTreeMap<u32, ValidatedEquipmentItem> {
        &self.items
    }
    pub fn equipment(&self) -> &BTreeMap<String, u32> {
        &self.equipment
    }
    pub fn active_xml(&self) -> &str {
        &self.active_xml
    }
    pub fn support_order(&self) -> &[String] {
        &self.support_order
    }
    pub(super) fn materialize(
        &self,
        tree: &ResolvedPassiveAllocation,
        equipment: &BTreeMap<String, &ValidatedEquipmentItem>,
        supports: &[String],
        data: &GameDataPackage,
    ) -> Result<BuildDocument> {
        let mut edits: Vec<_> = self
            .spans
            .cached_outputs
            .iter()
            .cloned()
            .map(|r| (r, String::new()))
            .collect();
        let nodes = tree
            .selection
            .ordinary_nodes
            .union(&tree.selection.ascendancy_nodes)
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        for (name, value) in [
            ("className", tree.class.name.clone()),
            (
                "ascendClassName",
                tree.ascendancy
                    .as_ref()
                    .map_or("None", |a| a.name.as_str())
                    .to_owned(),
            ),
            ("classId", tree.class.integer_id.to_string()),
            ("classInternalId", tree.class.integer_id.to_string()),
            (
                "ascendClassId",
                tree.ascendancy
                    .as_ref()
                    .map_or(0, |a| a.class_index)
                    .to_string(),
            ),
            (
                "ascendancyInternalId",
                tree.ascendancy
                    .as_ref()
                    .map_or("", |a| a.internal_id.as_str())
                    .to_owned(),
            ),
            ("nodes", nodes),
        ] {
            if let Some(range) = self.spans.fields.get(name) {
                edits.push((range.clone(), escape_attribute(&value)));
            } else if name == "ascendancyInternalId" && !value.is_empty() {
                edits.push((
                    self.spans.ascendancy_insert..self.spans.ascendancy_insert,
                    format!("ascendancyInternalId=\"{}\" ", escape_attribute(&value)),
                ));
            }
        }
        if tree.selection.attribute_options != self.allocation.attribute_options {
            let lists = AttributeOption::ALL.map(|option| {
                tree.selection
                    .attribute_options
                    .iter()
                    .filter_map(|(id, value)| (*value == option).then_some(id.to_string()))
                    .collect::<Vec<_>>()
                    .join(",")
            });
            let text = if tree.selection.attribute_options.is_empty() {
                String::new()
            } else {
                format!(
                    "<Overrides><AttributeOverride strNodes=\"{}\" dexNodes=\"{}\" intNodes=\"{}\"/></Overrides>",
                    lists[0], lists[1], lists[2]
                )
            };
            if let Some(range) = &self.spans.overrides {
                edits.push((range.clone(), text));
            } else if !text.is_empty() {
                edits.push(self.spans.spec.insert_children("Spec", text));
            }
        }
        let mut new_items = String::new();
        let mut seen = BTreeSet::new();
        for item in equipment.values() {
            if !seen.insert(item.pob_item_id()) {
                return Err(fail("duplicate selected item instance"));
            }
            let text = format!(
                "<Item id=\"{}\">{}</Item>",
                item.pob_item_id(),
                escape(item.source_text())
            );
            if let Some(range) = self.spans.item_ranges.get(&item.pob_item_id()) {
                if self.items[&item.pob_item_id()].source_text() != item.source_text() {
                    edits.push((range.clone(), text));
                }
            } else {
                new_items.push_str(&text);
            }
        }
        if !new_items.is_empty() {
            edits.push(self.spans.items.insert_children("Items", new_items));
        }
        let mut new_slots = String::new();
        for slot in self
            .spans
            .slot_ranges
            .keys()
            .chain(equipment.keys())
            .collect::<BTreeSet<_>>()
        {
            let id = equipment.get(slot).map_or(0, |item| item.pob_item_id());
            if self.equipment.get(slot).copied().unwrap_or(0) == id {
                continue;
            }
            let text = format!(
                "<Slot name=\"{}\" itemId=\"{id}\"/>",
                escape_attribute(slot)
            );
            if let Some(range) = self.spans.slot_ranges.get(slot) {
                edits.push((range.clone(), text));
            } else {
                new_slots.push_str(&text);
            }
        }
        if !new_slots.is_empty() {
            edits.push(self.spans.item_set.insert_children("ItemSet", new_slots));
        }
        if supports != self.support_order {
            for range in &self.spans.support_ranges {
                edits.push((range.clone(), String::new()));
            }
            let text = supports
                .iter()
                .map(|key| data.support(key).ok_or_else(|| fail("unknown support")))
                .collect::<Result<Vec<_>>>()?
                .iter()
                .map(|s| gem_xml(&s.name, &s.skill_id, &s.game_id, &s.variant_id))
                .collect::<Vec<_>>()
                .join("");
            if !text.is_empty() {
                edits.push((self.spans.support_insert..self.spans.support_insert, text));
            }
        }
        Ok(BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: apply(&self.source, edits)?,
        })
    }
}
pub(super) fn gem_xml(name: &str, id: &str, game: &str, variant: &str) -> String {
    format!(
        "<Gem nameSpec=\"{}\" skillId=\"{}\" gemId=\"{}\" variantId=\"{}\" level=\"1\" quality=\"0\" enabled=\"true\" enableGlobal1=\"true\" enableGlobal2=\"true\" count=\"1\"/>",
        escape_attribute(name),
        escape_attribute(id),
        escape_attribute(game),
        escape_attribute(variant)
    )
}
fn validate_gem(node: Node<'_, '_>, name: &str, id: &str, game: &str, variant: &str) -> Result<()> {
    only(
        node,
        &[
            "nameSpec",
            "skillId",
            "gemId",
            "variantId",
            "level",
            "quality",
            "enabled",
            "enableGlobal1",
            "enableGlobal2",
            "count",
        ],
        &[],
    )?;
    fixed(
        node,
        &[
            ("nameSpec", name),
            ("skillId", id),
            ("gemId", game),
            ("variantId", variant),
            ("level", "1"),
            ("quality", "0"),
            ("enabled", "true"),
            ("enableGlobal1", "true"),
            ("enableGlobal2", "true"),
            ("count", "1"),
        ],
    )
}
fn scalar(node: Node<'_, '_>) -> Result<Scalar> {
    crate::configuration::read_input_scalar(node).map_err(|error| fail(error.to_string()))
}
fn validate_config(
    name: &str,
    value: &Scalar,
    profile: TemplateProfile,
    data: &GameDataPackage,
) -> Result<()> {
    let valid = match (name, value) {
        ("enemyIsBoss", Scalar::Text(v)) => {
            if profile == TemplateProfile::Mace {
                v == "None"
            } else {
                ["None", "Boss", "Pinnacle"].contains(&v.as_str())
            }
        }
        ("enemyArmour" | "enemyEvasion", Scalar::Number(v)) if profile == TemplateProfile::Mace => {
            (0.0..=1_000_000.0).contains(v)
        }
        ("enemyDamageType", Scalar::Text(v)) => v == "Melee",
        (
            "conditionEnemyShocked"
            | "conditionEnemyChilled"
            | "conditionEnemyIgnited"
            | "conditionCritRecently"
            | "conditionBeenHitRecently",
            Scalar::Boolean(v),
        ) => !*v,
        ("enemyLevel", Scalar::Number(v)) => (1.0..=85.0).contains(v) && v.fract() == 0.0,
        (
            "enemyFireResist" | "enemyColdResist" | "enemyLightningResist" | "enemyChaosResist",
            Scalar::Number(v),
        ) => (-200.0..=200.0).contains(v),
        (
            "enemyPhysicalDamage"
            | "enemyFireDamage"
            | "enemyColdDamage"
            | "enemyLightningDamage"
            | "enemyChaosDamage",
            Scalar::Number(v),
        ) => (0.0..=1_000_000.0).contains(v),
        (
            "enemyPhysicalOverwhelm" | "enemyFirePen" | "enemyColdPen" | "enemyLightningPen",
            Scalar::Number(v),
        ) => (0.0..=100.0).contains(v),
        ("enemyCritChance", Scalar::Number(v)) => *v == 0.0,
        ("enemySpeed", Scalar::Number(v)) => (1.0..=60_000.0).contains(v),
        ("multiplierNearbyEnemies", Scalar::Number(v)) => *v == 1.0,
        ("multiplierNearbyRareOrUniqueEnemies", Scalar::Number(v)) => *v == 0.0 || *v == 1.0,
        ("resistancePenalty", Scalar::Number(v)) => (-60.0..=0.0).contains(v),
        (key, Scalar::Boolean(_)) => {
            data.quests.config_keys.iter().any(|s| s == key)
                || data.actor.spirit_quests.iter().any(|q| q.config_key == key)
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(BuildCatalogError::Source(format!(
            "unsupported config {name}={value:?}"
        )))
    }
}

/// Read the exact admitted allocation syntax shared by catalog and native source paths.
/// Callers validate the containing document/Build frame and resolve this selection
/// against the selected GameDataSnapshot before numerical use.
pub fn parse_passive_allocation(
    build: Node<'_, '_>,
    spec: Node<'_, '_>,
    data: &GameDataPackage,
) -> Result<PassiveAllocationSelection> {
    only(
        spec,
        &[
            "title",
            "classId",
            "classInternalId",
            "ascendClassId",
            "ascendancyInternalId",
            "treeVersion",
            "nodes",
            "masteryEffects",
        ],
        &["Overrides"],
    )?;
    fixed(
        spec,
        &[
            ("treeVersion", &data.tree.source.tree_version),
            ("masteryEffects", ""),
        ],
    )?;
    let class_id = integer(
        spec.attribute("classInternalId")
            .ok_or_else(|| fail("missing classInternalId"))?,
    )?;
    let class = data.tree.class(class_id).map_err(|e| fail(e.to_string()))?;
    let source_class = integer(
        spec.attribute("classId")
            .ok_or_else(|| fail("missing classId"))?,
    )?;
    if ![class.integer_id, class.source_index].contains(&source_class)
        || build.attribute("className") != Some(&class.name)
    {
        return Err(fail("class identity fields disagree"));
    }
    let ascendancy_index = integer(
        spec.attribute("ascendClassId")
            .ok_or_else(|| fail("missing ascendClassId"))?,
    )?;
    let ascendancy_id = spec.attribute("ascendancyInternalId").unwrap_or("");
    let ascendancy = if ascendancy_index == 0 {
        if !ascendancy_id.is_empty() || build.attribute("ascendClassName") != Some("None") {
            return Err(fail("no-ascendancy fields disagree"));
        }
        None
    } else {
        let asc = data
            .tree
            .ascendancy(class_id, ascendancy_id)
            .map_err(|e| fail(e.to_string()))?;
        if asc.class_index != ascendancy_index
            || build.attribute("ascendClassName") != Some(&asc.name)
        {
            return Err(fail("ascendancy identity fields disagree"));
        }
        Some(asc)
    };
    let requested = ids(spec
        .attribute("nodes")
        .ok_or_else(|| fail("missing nodes"))?)?;
    let mut roots = BTreeSet::from([class.start_node_id]);
    if let Some(asc) = ascendancy {
        roots.insert(asc.start_node_id);
    }
    let paid: BTreeSet<_> = requested.difference(&roots).copied().collect();
    let (ascendancy_nodes, ordinary_nodes) = paid
        .iter()
        .copied()
        .partition(|id| data.tree.ascendancy_nodes.contains_key(id));
    let attribute_options = attribute_options(spec, &paid)?
        .into_iter()
        .map(|(id, index)| {
            (
                id,
                match index {
                    1 => AttributeOption::Strength,
                    2 => AttributeOption::Dexterity,
                    3 => AttributeOption::Intelligence,
                    _ => unreachable!(),
                },
            )
        })
        .collect();
    let allocation = PassiveAllocationSelection {
        class_id,
        ascendancy_id: ascendancy.map(|a| a.internal_id.clone()),
        ordinary_nodes,
        ascendancy_nodes,
        attribute_options,
    };
    Ok(allocation)
}
