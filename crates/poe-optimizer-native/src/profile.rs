//! Strict source-document projection for the closed native Spark and Mace profiles.
use poe_optimizer_core::{evaluation::*, options::*};
use poe_optimizer_engine::{
    mace::{MaceInput, MaceWeapon},
    spark::{SparkInput, SparkQuestRewards},
};
use roxmltree::{Document, Node, ParsingOptions};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy)]
pub(crate) enum NativeInput {
    Spark(SparkInput),
    Mace(MaceInput),
}
pub(crate) struct Profile {
    pub input: NativeInput,
    pub support_keys: Vec<String>,
    pub support_order: Vec<String>,
    pub prepared_supports: Option<poe_optimizer_engine::mace_supports::PreparedMaceSupports>,
    pub weapon_record: Option<poe_optimizer_import::mace_item::ValidatedMaceWeapon>,
    pub prepared_weapon: Option<poe_optimizer_engine::weapon::PreparedWeaponStats>,
    pub equipment: BTreeMap<String, poe_optimizer_import::equipment::ValidatedEquipmentItem>,
    pub actor_modifiers: poe_optimizer_import::actor_modifiers::ValidatedActorModifiers,
    pub actor_quests: poe_optimizer_engine::actor::ActorQuestSelection,
    pub prepared_actor: poe_optimizer_engine::actor::PreparedActorResources,
    pub prepared_armour: BTreeMap<String, poe_optimizer_engine::armour::PreparedArmour>,
    pub tree: crate::tree::NativeTree,
    pub enemy_level: u32,
    pub config: BTreeMap<String, Scalar>,
    pub export_xml: String,
    pub group_label: Option<String>,
}
fn unsupported(message: impl Into<String>) -> EvaluationError {
    EvaluationError::new(EvaluationErrorKind::UnsupportedCapability, message)
}
fn child<'a, 'input>(
    node: Node<'a, 'input>,
    name: &str,
) -> Result<Node<'a, 'input>, EvaluationError> {
    let values: Vec<_> = node.children().filter(|n| n.has_tag_name(name)).collect();
    if values.len() != 1 {
        return Err(unsupported(format!(
            "Native build profile requires exactly one {name}"
        )));
    }
    Ok(values[0])
}
fn only(node: Node<'_, '_>, attrs: &[&str], children: &[&str]) -> Result<(), EvaluationError> {
    if node.tag_name().namespace().is_some()
        || node
            .attributes()
            .any(|a| a.namespace().is_some() || !attrs.contains(&a.name()))
        || node
            .children()
            .filter(Node::is_element)
            .any(|n| n.tag_name().namespace().is_some() || !children.contains(&n.tag_name().name()))
        || node
            .children()
            .any(|n| n.is_text() && !n.text().unwrap_or("").trim().is_empty())
    {
        return Err(unsupported(format!(
            "Unsupported native build fields or content in {}",
            node.tag_name().name()
        )));
    }
    Ok(())
}
fn fixed(node: Node<'_, '_>, pairs: &[(&str, &str)]) -> Result<(), EvaluationError> {
    for (key, value) in pairs {
        if node.attribute(*key) != Some(*value) {
            return Err(unsupported(format!(
                "Native build profile requires {}.{key}={value}",
                node.tag_name().name()
            )));
        }
    }
    Ok(())
}
fn scalar(node: Node<'_, '_>) -> Result<Scalar, EvaluationError> {
    only(node, &["name", "number", "string", "boolean"], &[])?;
    let kinds: Vec<_> = ["number", "string", "boolean"]
        .into_iter()
        .filter_map(|k| node.attribute(k).map(|v| (k, v)))
        .collect();
    if kinds.len() != 1 {
        return Err(unsupported(
            "Configuration scalar must have exactly one type",
        ));
    }
    match kinds[0] {
        ("number", s) => s
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .map(Scalar::Number)
            .ok_or_else(|| unsupported("Configuration number must be finite")),
        ("boolean", "true") => Ok(Scalar::Boolean(true)),
        ("boolean", "false") => Ok(Scalar::Boolean(false)),
        ("string", s) => Ok(Scalar::Text(s.into())),
        _ => Err(unsupported("Invalid configuration scalar")),
    }
}
fn validate_config(
    name: &str,
    value: &Scalar,
    is_mace: bool,
    data: &crate::CompiledGameData,
) -> Result<(), EvaluationError> {
    let valid = match (name, value) {
        ("enemyIsBoss", Scalar::Text(v)) => {
            if is_mace {
                v == "None"
            } else {
                ["None", "Boss", "Pinnacle"].contains(&v.as_str())
            }
        }
        ("enemyArmour" | "enemyEvasion", Scalar::Number(v)) if is_mace => {
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
        ) => !v,
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
        (name, Scalar::Boolean(_))
            if data
                .snapshot()
                .package()
                .quests
                .config_keys
                .iter()
                .any(|key| key == name)
                || data
                    .snapshot()
                    .package()
                    .actor
                    .spirit_quests
                    .iter()
                    .any(|quest| quest.config_key == name) =>
        {
            true
        }
        _ => false,
    };
    if !valid {
        return Err(unsupported(format!(
            "Unsupported native build configuration {name}={value:?}"
        )));
    }
    Ok(())
}
fn number(map: &BTreeMap<String, Scalar>, name: &str) -> f64 {
    match &map[name] {
        Scalar::Number(n) => *n,
        _ => unreachable!("validated configuration type"),
    }
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
}

/// Fixed encounter/quest inputs validated through the complete source parser.
/// No selected character, resource, or local weapon calculation is performed.
pub(crate) struct ScenarioProfile {
    pub input: NativeInput,
    pub config: BTreeMap<String, Scalar>,
    pub actor_quests: poe_optimizer_engine::actor::ActorQuestSelection,
}
// This short-lived projection stays on the stack rather than adding a heap
// allocation to every full document preparation.
#[allow(clippy::large_enum_variant)]
enum ProfileProjection {
    Complete(Profile),
    Scenario(ScenarioProfile),
}
pub(crate) fn parse(
    request: &EvaluationRequest,
    data: &crate::CompiledGameData,
) -> Result<Profile, EvaluationError> {
    match parse_projection(request, data, true)? {
        ProfileProjection::Complete(profile) => Ok(profile),
        ProfileProjection::Scenario(_) => unreachable!("full projection requested"),
    }
}
pub(crate) fn prepare_scenario(
    request: &EvaluationRequest,
    data: &crate::CompiledGameData,
) -> Result<ScenarioProfile, EvaluationError> {
    let known = crate::metric_catalog();
    let mut queries = BTreeSet::new();
    for query in &request.metrics {
        if !queries.insert(query) {
            return Err(EvaluationError::new(
                EvaluationErrorKind::InvalidRequest,
                "Duplicate metric query",
            ));
        }
        if !known
            .iter()
            .any(|metric| metric.id == query.id && metric.actors.contains(&query.actor))
        {
            return Err(EvaluationError::new(
                EvaluationErrorKind::UnsupportedCapability,
                format!(
                    "Native backend does not implement {:?}.{}",
                    query.actor, query.id
                ),
            ));
        }
    }
    match parse_projection(request, data, false)? {
        ProfileProjection::Scenario(profile) => Ok(profile),
        ProfileProjection::Complete(_) => unreachable!("scenario projection requested"),
    }
}
fn parse_projection(
    request: &EvaluationRequest,
    data: &crate::CompiledGameData,
    prepare_numeric: bool,
) -> Result<ProfileProjection, EvaluationError> {
    request
        .options
        .validate()
        .map_err(|e| EvaluationError::new(EvaluationErrorKind::InvalidRequest, e))?;
    if request.build.content.len() > poe_optimizer_import::MAX_XML_BYTES {
        return Err(unsupported("Native XML exceeds byte limit"));
    }
    let doc = Document::parse_with_options(
        &request.build.content,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: poe_optimizer_import::MAX_XML_NODES,
            entity_resolver: None,
        },
    )
    .map_err(|e| EvaluationError::new(EvaluationErrorKind::InvalidRequest, e.to_string()))?;
    poe_optimizer_import::xml_compat::validate_native_with_actor_inputs(&doc)
        .map_err(|error| unsupported(format!("Native {error}")))?;
    let root = doc.root_element();
    if root.tag_name().name() != "PathOfBuilding2" {
        return Err(unsupported("Expected PathOfBuilding2 XML"));
    }
    only(
        root,
        &[],
        &["Build", "Tree", "Skills", "Items", "Config", "Notes"],
    )?;
    let build = child(root, "Build")?;
    let main_group = child(child(child(root, "Skills")?, "SkillSet")?, "Skill")?;
    let main_gem = main_group
        .children()
        .find(|node| node.has_tag_name("Gem"))
        .ok_or_else(|| unsupported("Native profile requires a main active gem"))?;
    let is_mace = match main_gem.attribute("skillId") {
        Some(id) if id == data.snapshot().package().spark.skill_id => false,
        Some(id) if id == data.snapshot().package().mace.skill_id => true,
        _ => {
            return Err(unsupported(
                "Native profiles currently support Spark or Mace Strike",
            ));
        }
    };
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
    for cached in build.children().filter(Node::is_element) {
        only(cached, &["stat", "value"], &[])?;
    }
    let level = build
        .attribute("level")
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|n| (1..=100).contains(n))
        .ok_or_else(|| unsupported("Character level must be 1..100"))?;
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
            ("treeVersion", &data.snapshot().tree().source.tree_version),
            ("masteryEffects", ""),
        ],
    )?;
    let resolved_tree = if prepare_numeric {
        Some(crate::tree::NativeTree::resolve(build, spec, data)?)
    } else {
        crate::tree::validate_calculation_source(&data.snapshot().tree().source)?;
        poe_optimizer_import::controlled_build::parse_passive_allocation(
            build,
            spec,
            data.snapshot().package(),
        )
        .map_err(|error| unsupported(error.to_string()))?
        .resolve(data.snapshot())
        .map_err(|error| unsupported(error.to_string()))?;
        None
    };
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
    let skill_set = child(skills, "SkillSet")?;
    only(skill_set, &["id", "title"], &["Skill"])?;
    fixed(skill_set, &[("id", "1")])?;
    let skill = child(skill_set, "Skill")?;
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
    if gems.is_empty() || gems.len() > if is_mace { 3 } else { 1 } {
        return Err(unsupported(
            "Native profile requires one active skill and zero to two reviewed Mace supports",
        ));
    }
    let package = data.snapshot().package();
    if is_mace {
        let gem = &package.mace;
        validate_gem(
            gems[0],
            &gem.name,
            &gem.skill_id,
            &gem.game_id,
            &gem.variant_id,
        )?;
    } else {
        let gem = &package.spark;
        validate_gem(
            gems[0],
            &gem.name,
            &gem.skill_id,
            &gem.game_id,
            &gem.variant_id,
        )?;
    }
    let mut support_order = Vec::new();
    for node in &gems[1..] {
        let gem = package
            .supports
            .iter()
            .find(|gem| Some(gem.skill_id.as_str()) == node.attribute("skillId"))
            .ok_or_else(|| unsupported("Unknown native Mace support"))?;
        validate_gem(
            *node,
            &gem.name,
            &gem.skill_id,
            &gem.game_id,
            &gem.variant_id,
        )?;
        support_order.push(gem.id.clone());
    }
    let mut support_keys = support_order.clone();
    support_keys.sort();
    let prepared_supports = if is_mace {
        Some(
            data.mace_support_loadout(&support_keys)
                .map_err(|error| unsupported(error.to_string()))?
                .clone(),
        )
    } else {
        None
    };
    let items = child(root, "Items")?;
    only(items, &["activeItemSet"], &["Item", "ItemSet"])?;
    fixed(items, &[("activeItemSet", "1")])?;
    let item_set = child(items, "ItemSet")?;
    only(item_set, &["id", "title", "useSecondWeaponSet"], &["Slot"])?;
    fixed(item_set, &[("id", "1")])?;
    if item_set
        .attribute("useSecondWeaponSet")
        .is_some_and(|value| value != "false")
        || (is_mace && item_set.attribute("useSecondWeaponSet") != Some("false"))
    {
        return Err(unsupported(
            "Native equipment requires the first weapon set",
        ));
    }
    let mut supplied = BTreeMap::new();
    for item in items.children().filter(|node| node.has_tag_name("Item")) {
        let parsed = poe_optimizer_import::equipment::parse_equipment_item_xml(item, package)
            .map_err(|error| unsupported(error.to_string()))?;
        if supplied.insert(parsed.pob_item_id(), parsed).is_some() {
            return Err(unsupported("Duplicate physical item ID"));
        }
    }
    let mut equipment = BTreeMap::new();
    let mut selected_slots = BTreeSet::new();
    for slot in item_set.children().filter(Node::is_element) {
        only(slot, &["name", "itemId"], &[])?;
        let name = slot
            .attribute("name")
            .ok_or_else(|| unsupported("Missing equipment slot name"))?;
        if !poe_optimizer_import::equipment::EQUIPMENT_SOURCE_ORDER.contains(&name)
            || !selected_slots.insert(name)
        {
            return Err(unsupported("Unsupported or duplicate equipment slot"));
        }
        let raw_id = slot
            .attribute("itemId")
            .ok_or_else(|| unsupported("Missing equipment slot itemId"))?;
        let id = raw_id
            .parse::<u32>()
            .ok()
            .filter(|id| raw_id == id.to_string())
            .ok_or_else(|| {
                unsupported("Equipment slot itemId must be canonical unsigned integer")
            })?;
        if id == 0 {
            continue;
        }
        let item = supplied
            .remove(&id)
            .ok_or_else(|| unsupported("Unknown or multiply equipped physical item"))?;
        if !item.allowed_slots().iter().any(|slot| slot == name)
            || (!is_mace && item.weapon().is_some())
            || equipment.insert(name.to_owned(), item).is_some()
        {
            return Err(unsupported(
                "Unsupported, duplicate or incompatible equipment slot",
            ));
        }
    }
    // Known unequipped inventory is preserved in source and contributes no modifiers.
    let weapon = equipment
        .get("Weapon 1")
        .and_then(|item| item.weapon())
        .cloned();
    if is_mace && weapon.is_none() {
        return Err(unsupported("Native Mace requires one main-hand weapon"));
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
        poe_optimizer_import::actor_modifiers::parse_actor_configuration(config_set, package)
            .map_err(|error| unsupported(error.to_string()))?;
    let mut config = BTreeMap::new();
    let mut ranges = BTreeMap::new();
    for input in config_set
        .children()
        .filter(|node| node.has_tag_name("Input"))
    {
        // The shared actor parser validates legacy text and current modifier blocks.
        // Like PoB's migration, it keeps actor sources outside the scalar encounter map.
        if input.attribute("name") == Some("customMods") {
            continue;
        }
        let name = input
            .attribute("name")
            .ok_or_else(|| unsupported("Missing configuration name"))?;
        let value = scalar(input)?;
        validate_config(name, &value, is_mace, data)?;
        if config.insert(name.to_owned(), value).is_some() {
            return Err(unsupported("Duplicate configuration input"));
        }
        ranges.insert(name.to_owned(), input.range());
    }
    let expected: BTreeSet<_> = [
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
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    if package
        .quests
        .config_keys
        .iter()
        .map(String::as_str)
        .chain(
            package
                .actor
                .spirit_quests
                .iter()
                .map(|quest| quest.config_key.as_str()),
        )
        .any(|key| {
            expected.contains(key)
                || [
                    "resistancePenalty",
                    "enemyArmour",
                    "enemyEvasion",
                    "customMods",
                ]
                .contains(&key)
        })
    {
        return Err(unsupported(
            "Quest configuration keys conflict with native encounter or actor inputs",
        ));
    }
    if !expected.is_subset(&config.keys().cloned().collect::<BTreeSet<_>>()) {
        return Err(unsupported(
            "Native build profile requires all explicit encounter inputs",
        ));
    }
    if is_mace && !config.contains_key("enemyArmour") {
        return Err(unsupported("Native Mace requires explicit enemyArmour"));
    }
    let notes: Vec<_> = root
        .children()
        .filter(|n| n.has_tag_name("Notes"))
        .collect();
    if notes.len() > 1
        || notes
            .iter()
            .any(|n| n.attributes().len() != 0 || n.children().any(|c| c.is_element()))
    {
        return Err(unsupported("Unsupported Notes structure"));
    }
    if let Some(selection) = &request.options.selection
        && (selection.socket_group != 1
            || selection.active_skill.is_some_and(|n| n != 1)
            || selection.minion_skill.is_some())
    {
        return Err(unsupported(
            "Native build profiles support only group 1 action 1 and no minion",
        ));
    }
    let original = config.clone();
    if let Some(encounter) = &request.options.encounter {
        if let Some(level) = encounter.enemy_level {
            config.insert("enemyLevel".into(), Scalar::Number(level as f64));
        }
        if let Some(boss) = encounter.boss {
            config.insert(
                "enemyIsBoss".into(),
                Scalar::Text(
                    match boss {
                        BossKind::Normal => "None",
                        BossKind::Standard => "Boss",
                        BossKind::Pinnacle => "Pinnacle",
                        BossKind::Uber => {
                            return Err(unsupported(
                                "Uber encounter is unsupported by native build profiles",
                            ));
                        }
                    }
                    .into(),
                ),
            );
        }
        if let Some(hit) = &encounter.incoming_hit {
            for (key, value) in [
                ("enemyPhysicalDamage", hit.physical),
                ("enemyFireDamage", hit.fire),
                ("enemyColdDamage", hit.cold),
                ("enemyLightningDamage", hit.lightning),
                ("enemyChaosDamage", hit.chaos),
            ] {
                config.insert(key.into(), Scalar::Number(value));
            }
        }
    }
    for (key, value) in &config {
        validate_config(key, value, is_mace, data)?;
    }
    if [
        "enemyPhysicalDamage",
        "enemyFireDamage",
        "enemyColdDamage",
        "enemyLightningDamage",
        "enemyChaosDamage",
    ]
    .into_iter()
    .all(|name| number(&config, name) == 0.0)
    {
        return Err(unsupported(
            "Incoming hit must contain a positive component",
        ));
    }
    // Input cached outputs are not calculations and must not survive a fresh export.
    let mut replacements: Vec<_> = build
        .children()
        .filter(Node::is_element)
        .map(|n| (n.range(), String::new()))
        .collect();
    for (name, value) in &config {
        if original.get(name) != Some(value) {
            let attr = match value {
                Scalar::Number(n) => format!("number=\"{n}\""),
                Scalar::Boolean(b) => format!("boolean=\"{b}\""),
                Scalar::Text(s) => format!("string=\"{}\"", escape(s)),
            };
            replacements.push((
                ranges[name].clone(),
                format!("<Input name=\"{}\" {attr}/>", escape(name)),
            ));
        }
    }
    let export_xml = apply_source_edits(&request.build.content, replacements)?;
    let quest =
        |index: usize| match config.get(&data.snapshot().package().quests.config_keys[index]) {
            Some(Scalar::Boolean(v)) => *v,
            None => data.snapshot().package().quests.default_enabled[index],
            _ => unreachable!(),
        };
    let penalty = match config.get("resistancePenalty") {
        Some(Scalar::Number(n)) => *n,
        None => {
            data.snapshot()
                .package()
                .encounters
                .default_resistance_penalty
        }
        _ => unreachable!(),
    };
    let quests = SparkQuestRewards {
        candlemass: quest(0),
        molten_shrine: quest(1),
        silent_hall: quest(2),
        beira: quest(3),
        garukhan: quest(4),
        blackjaw: quest(5),
    };
    let mut actor_quests = data.actor_quest_selection(quests);
    for (index, quest) in package.actor.spirit_quests.iter().enumerate() {
        if let Some(Scalar::Boolean(enabled)) = config.get(&quest.config_key) {
            actor_quests.spirit[index] = *enabled;
        }
    }
    let enemy_level = number(&config, "enemyLevel") as u32;
    let input = if let Some(record) = &weapon {
        let weapon = weapon_slot(record.weapon_key())?;
        let item_level = record.item_level();
        let quality = record.quality();
        let enemy_evasion = match config.get("enemyEvasion") {
            Some(Scalar::Number(value)) => *value,
            None => data
                .monster_evasion(enemy_level)
                .map_err(|error| unsupported(error.to_string()))?,
            _ => unreachable!("validated evasion type"),
        };
        NativeInput::Mace(MaceInput {
            character_level: level,
            weapon,
            quality,
            item_level,
            brutality: false,
            resistance_penalty: penalty,
            quests,
            enemy_armour: number(&config, "enemyArmour"),
            enemy_evasion,
            enemy_fire_resistance: number(&config, "enemyFireResist"),
        })
    } else {
        NativeInput::Spark(SparkInput {
            character_level: level,
            resistance_penalty: penalty,
            enemy_lightning_resistance: number(&config, "enemyLightningResist"),
            quests,
        })
    };
    if !prepare_numeric {
        return Ok(ProfileProjection::Scenario(ScenarioProfile {
            input,
            config,
            actor_quests,
        }));
    }
    let resolved_tree = resolved_tree.expect("full numeric tree requested");
    let mut prepared_armour = BTreeMap::new();
    for (slot, item) in &equipment {
        if let Some(records) = item.armour_modifiers() {
            let armour = data
                .prepare_armour_with_source(
                    item.base_id(),
                    item.quality(),
                    item.item_level(),
                    &item.modifier_source(),
                    records,
                )
                .map_err(|error| unsupported(error.to_string()))?;
            if armour.source_global_records() != item.actor_modifiers() {
                return Err(unsupported(
                    "armour global record projection differs from selected preparation",
                ));
            }
            prepared_armour.insert(slot.clone(), armour);
        }
    }
    // PoB constructs one local actor layer: configuration, source slot order, passives.
    let mut actor_records = actor_modifiers.records().to_vec();
    for slot in poe_optimizer_import::equipment::EQUIPMENT_SOURCE_ORDER {
        if let Some(item) = equipment.get(slot) {
            actor_records.extend_from_slice(
                prepared_armour
                    .get(slot)
                    .map_or(item.actor_modifiers(), |armour| armour.global_records()),
            );
        }
    }
    actor_records.extend(resolved_tree.actor_modifiers().cloned());
    let actor_layers = vec![actor_records];
    let prepared_actor = data
        .prepare_actor_with_armour(
            level,
            actor_quests,
            data.receiving_scenario(quests, penalty),
            &resolved_tree.character,
            &actor_layers,
            poe_optimizer_engine::armour::ArmourSlots {
                helmet: prepared_armour.get("Helmet"),
                gloves: prepared_armour.get("Gloves"),
                boots: prepared_armour.get("Boots"),
                body_armour: prepared_armour.get("Body Armour"),
            },
        )
        .map_err(|error| unsupported(error.to_string()))?;
    let prepared_weapon = weapon
        .as_ref()
        .map(|record| {
            data.prepare_mace_weapon(
                weapon_slot(record.weapon_key())?,
                record.quality(),
                record.item_level(),
                record.local_modifiers(),
            )
            .map_err(|error| unsupported(error.to_string()))
        })
        .transpose()?;
    Ok(ProfileProjection::Complete(Profile {
        input,
        support_keys,
        support_order,
        prepared_supports,
        weapon_record: weapon,
        prepared_weapon,
        actor_modifiers,
        equipment,
        actor_quests,
        prepared_actor,
        prepared_armour,
        tree: resolved_tree,
        enemy_level,
        config,
        export_xml,
        group_label: skill.attribute("label").map(str::to_owned),
    }))
}

fn validate_gem(
    node: Node<'_, '_>,
    name: &str,
    skill_id: &str,
    game_id: &str,
    variant_id: &str,
) -> Result<(), EvaluationError> {
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
            ("skillId", skill_id),
            ("gemId", game_id),
            ("variantId", variant_id),
            ("level", "1"),
            ("quality", "0"),
            ("enabled", "true"),
            ("enableGlobal1", "true"),
            ("enableGlobal2", "true"),
            ("count", "1"),
        ],
    )
}

fn weapon_slot(key: &str) -> Result<MaceWeapon, EvaluationError> {
    match key {
        "wooden_club" => Ok(MaceWeapon::WoodenClub),
        "smithing_hammer" => Ok(MaceWeapon::SmithingHammer),
        _ => Err(unsupported("Unknown native Mace weapon capability slot")),
    }
}
/// Copy each untouched source span once, including potentially large trailing Notes.
/// All edit spans refer to the original XML and must be disjoint UTF-8 boundaries.
fn apply_source_edits(
    source: &str,
    mut edits: Vec<(std::ops::Range<usize>, String)>,
) -> Result<String, EvaluationError> {
    let invalid = || {
        EvaluationError::new(
            EvaluationErrorKind::BackendContract,
            "Native export edits contain invalid, overlapping or overflowing source spans",
        )
    };
    edits.sort_unstable_by_key(|(range, _)| range.start);
    let mut length = source.len();
    let mut previous_end = 0;
    for (range, value) in &edits {
        if range.start < previous_end || source.get(range.clone()).is_none() {
            return Err(invalid());
        }
        length = length
            .checked_sub(range.len())
            .and_then(|length| length.checked_add(value.len()))
            .ok_or_else(invalid)?;
        previous_end = range.end;
    }
    if length > poe_optimizer_import::MAX_XML_BYTES {
        return Err(unsupported("Native export exceeds XML byte limit"));
    }
    let mut exported = String::with_capacity(length);
    let mut cursor = 0;
    for (range, value) in edits {
        exported.push_str(&source[cursor..range.start]);
        exported.push_str(&value);
        cursor = range.end;
    }
    exported.push_str(&source[cursor..]);
    debug_assert_eq!(exported.len(), length);
    Ok(exported)
}

#[cfg(test)]
mod scenario_tests {
    use super::*;
    const MACE: &str = include_str!("../../../tests/fixtures/builds/mace-passive-equipment.xml");
    #[test]
    fn fixed_scenario_and_full_projection_reject_the_same_unsupported_source() {
        let data = crate::NativeBackend::new();
        for source in [
            MACE.replace(
                "name=\"enemyCritChance\" number=\"0\"",
                "name=\"enemyCritChance\" number=\"1\"",
            ),
            MACE.replace("<Gem nameSpec=", "<Gem unsupported=\"ignored\" nameSpec="),
            MACE.replace("<PathOfBuilding2>", "<PathOfBuilding2 xmlns=\"foreign\">"),
            MACE.replace("</PathOfBuilding2>", "<Party/></PathOfBuilding2>"),
        ] {
            let request = EvaluationRequest {
                build: BuildDocument {
                    format: BuildFormat::PathOfBuilding2Xml,
                    content: source,
                },
                options: EvaluationOptions::default(),
                metrics: vec![],
            };
            let full = parse(&request, data.data())
                .err()
                .expect("invalid full source");
            let scenario = prepare_scenario(&request, data.data())
                .err()
                .expect("invalid scenario source");
            assert_eq!(scenario.kind, full.kind);
            assert_eq!(scenario.message, full.message);
        }
    }
}
