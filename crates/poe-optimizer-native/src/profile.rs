//! Strict source-document projection for the closed native Spark and Mace profiles.
use poe_optimizer_core::{evaluation::*, options::*};
use poe_optimizer_engine::{
    mace::{self, MaceInput, MaceWeapon},
    spark::{SparkInput, SparkQuestRewards},
};
use roxmltree::{Document, Node, ParsingOptions};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const QUEST_KEYS: [&str; 6] = [
    "questAct 1Ogham ManorCandlemass",
    "questInterlude 2Khari CrossingMolten Shrine",
    "questAct 4Eye of HinekoraSilent Hall",
    "questAct 1ClearfellBeira",
    "questAct 2Spires of DesharSisters of Garukhan Shrine",
    "questAct 3Jiquani's MachinariumBlackjaw",
];
#[derive(Debug, Clone, Copy)]
pub(crate) enum NativeInput {
    Spark(SparkInput),
    Mace(MaceInput),
}
pub(crate) struct Profile {
    pub input: NativeInput,
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
fn validate_config(name: &str, value: &Scalar, is_mace: bool) -> Result<(), EvaluationError> {
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
        (name, Scalar::Boolean(_)) if QUEST_KEYS.contains(&name) => true,
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

pub(crate) fn parse(request: &EvaluationRequest) -> Result<Profile, EvaluationError> {
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
    poe_optimizer_import::xml_compat::validate_native(&request.build.content)
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
        Some("SparkPlayer") => false,
        Some("Melee1HMacePlayer") => true,
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
        &[],
    )?;
    fixed(spec, &[("treeVersion", "0_5"), ("masteryEffects", "")])?;
    let resolved_tree = crate::tree::NativeTree::resolve(build, spec)?;
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
    if gems.is_empty() || gems.len() > if is_mace { 2 } else { 1 } {
        return Err(unsupported(
            "Native profile requires one active skill and only the supported optional Brutality I",
        ));
    }
    if is_mace {
        validate_gem(
            gems[0],
            "Mace Strike",
            "Melee1HMacePlayer",
            "Metadata/Items/Gem/SkillGemPlayerDefault1HMace",
            "PlayerDefault1HMace",
        )?;
    } else {
        validate_gem(
            gems[0],
            "Spark",
            "SparkPlayer",
            "Metadata/Items/Gems/SkillGemSpark",
            "Spark",
        )?;
    }
    let brutality = gems.len() == 2;
    if brutality {
        validate_gem(
            gems[1],
            "Brutality I",
            "SupportBrutalityPlayer",
            "Metadata/Items/Gems/SupportGemBrutality",
            "BrutalitySupport",
        )?;
    }
    let items = child(root, "Items")?;
    only(
        items,
        &["activeItemSet"],
        if is_mace {
            &["Item", "ItemSet"]
        } else {
            &["ItemSet"]
        },
    )?;
    fixed(items, &[("activeItemSet", "1")])?;
    let item_set = child(items, "ItemSet")?;
    let weapon = if is_mace {
        only(item_set, &["id", "title", "useSecondWeaponSet"], &["Slot"])?;
        fixed(item_set, &[("id", "1"), ("useSecondWeaponSet", "false")])?;
        let slot = child(item_set, "Slot")?;
        only(slot, &["name", "itemId"], &[])?;
        fixed(slot, &[("name", "Weapon 1"), ("itemId", "1")])?;
        Some(parse_weapon(child(items, "Item")?)?)
    } else {
        only(item_set, &["id", "title"], &[])?;
        fixed(item_set, &[("id", "1")])?;
        None
    };
    let config_node = child(root, "Config")?;
    only(config_node, &["activeConfigSet"], &["ConfigSet"])?;
    fixed(config_node, &[("activeConfigSet", "1")])?;
    let config_set = child(config_node, "ConfigSet")?;
    only(config_set, &["id", "title"], &["Input"])?;
    fixed(config_set, &[("id", "1")])?;
    let mut config = BTreeMap::new();
    let mut ranges = BTreeMap::new();
    for input in config_set.children().filter(Node::is_element) {
        let name = input
            .attribute("name")
            .ok_or_else(|| unsupported("Missing configuration name"))?;
        let value = scalar(input)?;
        validate_config(name, &value, is_mace)?;
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
        validate_config(key, value, is_mace)?;
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
    let quest = |index: usize| match config.get(QUEST_KEYS[index]) {
        Some(Scalar::Boolean(v)) => *v,
        None => true,
        _ => unreachable!(),
    };
    let penalty = match config.get("resistancePenalty") {
        Some(Scalar::Number(n)) => *n,
        None => -60.0,
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
    let enemy_level = number(&config, "enemyLevel") as u32;
    let input = if let Some((weapon, item_level, quality)) = weapon {
        let enemy_evasion = match config.get("enemyEvasion") {
            Some(Scalar::Number(value)) => *value,
            None => mace::monster_evasion(enemy_level)
                .map_err(|error| unsupported(error.to_string()))?,
            _ => unreachable!("validated evasion type"),
        };
        NativeInput::Mace(MaceInput {
            character_level: level,
            weapon,
            quality,
            item_level,
            brutality,
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
    Ok(Profile {
        input,
        tree: resolved_tree,
        enemy_level,
        config,
        export_xml,
        group_label: skill.attribute("label").map(str::to_owned),
    })
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

fn parse_weapon(item: Node<'_, '_>) -> Result<(MaceWeapon, u32, u32), EvaluationError> {
    if item.tag_name().namespace().is_some()
        || item
            .attributes()
            .any(|attribute| attribute.namespace().is_some() || attribute.name() != "id")
        || item.attribute("id") != Some("1")
        || item.children().count() != 1
        || !item.first_child().is_some_and(|node| node.is_text())
    {
        return Err(unsupported(
            "Native Mace item must be one exact unmodified text payload at XML item ID 1",
        ));
    }
    let raw = item.text().unwrap_or_default();
    if raw.len() > 1024 {
        return Err(unsupported("Native normal mace payload exceeds 1024 bytes"));
    }
    let normalized = raw.replace("\r\n", "\n");
    let lines: Vec<_> = normalized.trim().lines().collect();
    if lines.len() != 5 || lines[0] != "Rarity: NORMAL" || lines[4] != "Implicits: 0" {
        return Err(unsupported(
            "Native Mace requires NORMAL rarity, zero implicits and no modifiers",
        ));
    }
    let weapon = match lines[1] {
        "Wooden Club" => MaceWeapon::WoodenClub,
        "Smithing Hammer" => MaceWeapon::SmithingHammer,
        _ => {
            return Err(unsupported(
                "Native Mace supports only Wooden Club and Smithing Hammer",
            ));
        }
    };
    fn integer(
        text: Option<&str>,
        min: u32,
        max: u32,
        field: &str,
    ) -> Result<u32, EvaluationError> {
        text.and_then(|text| {
            text.parse::<u32>()
                .ok()
                .filter(|value| value.to_string() == text)
        })
        .filter(|value| (min..=max).contains(value))
        .ok_or_else(|| {
            unsupported(format!(
                "Native Mace {field} must be a canonical integer in {min}..={max}"
            ))
        })
    }
    let item_level = integer(lines[2].strip_prefix("Item Level: "), 1, 100, "item level")?;
    let quality = integer(lines[3].strip_prefix("Quality: "), 0, 20, "quality")?;
    Ok((weapon, item_level, quality))
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
