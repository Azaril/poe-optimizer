//! Source-preserving, bounded weapon/support mutations for a structural Mace profile.
//! Experimental diagnostic domain, not a general build-legality claim.
use poe_optimizer_core::{
    candidate::*,
    coverage::{SkillActor, SkillOrigin, SkillResolution},
    evaluation::{BuildDocument, BuildFormat, EvaluationResult},
    options::{EvaluationContext, EvaluationOptions, Scalar},
};
use roxmltree::{Document, Node, ParsingOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use thiserror::Error;
const MAIN: &str = "Melee1HMacePlayer";
const SUPPORT: &str = "SupportBrutalityPlayer";
const SLOT: &str = "pob-group-1";
const SOURCE: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const SUPPORT_XML: &str = r#"<Gem nameSpec="Brutality I" skillId="SupportBrutalityPlayer" gemId="Metadata/Items/Gems/SupportGemBrutality" variantId="BrutalitySupport" level="1" quality="0" enabled="true" enableGlobal1="true" enableGlobal2="true" count="1"/>"#;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalMaceAlternative {
    pub id: String,
    pub item_text: String,
}
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaceSupportChoice {
    None,
    BrutalityI,
}
#[derive(Debug, Clone, Serialize)]
pub struct ControlledMaceAlternative {
    pub id: String,
    pub weapon_id: String,
    pub support: MaceSupportChoice,
    pub xml_sha256: String,
    pub candidate: Candidate,
}
#[derive(Debug, Error)]
pub enum ControlledMutationError {
    #[error("unsupported controlled Mace input: {0}")]
    Unsupported(String),
    #[error("candidate is not in this immutable controlled Mace domain")]
    UnknownCandidate,
    #[error("controlled Mace realization mismatch: {0}")]
    Realization(String),
}
type Result<T> = std::result::Result<T, ControlledMutationError>;
#[derive(Debug)]
struct Profile {
    level: u32,
    config: BTreeMap<String, Scalar>,
    item_range: Range<usize>,
    support_range: Range<usize>,
    weapon: String,
    active_attributes: BTreeMap<String, String>,
    has_support: bool,
}
#[derive(Debug, Clone)]
struct Choice {
    weapon: String,
    support: MaceSupportChoice,
}
/// Fresh validated template evidence; baseline-derived, not an independent golden.
#[derive(Debug)]
pub struct VerifiedMaceScenario {
    identity: CatalogIdentity,
    context: EvaluationContext,
    adapter_fingerprint: String,
    exported_scenario: String,
}
#[derive(Debug)]
pub struct ControlledMaceCatalog {
    template: String,
    profile: Profile,
    catalog: CandidateCatalog,
    alternatives: Vec<ControlledMaceAlternative>,
    choices: BTreeMap<Candidate, Choice>,
}
impl ControlledMaceCatalog {
    pub fn new(
        template_xml: String,
        weapons: Vec<NormalMaceAlternative>,
        supports: Vec<MaceSupportChoice>,
    ) -> Result<Self> {
        let profile = profile(&template_xml)?;
        if weapons.is_empty() || weapons.len() > 64 || supports.is_empty() || supports.len() > 2 {
            return Err(unsupported(
                "provide 1..64 weapons and 1..2 distinct support choices",
            ));
        }
        let mut ids = BTreeSet::new();
        let mut payloads = BTreeSet::new();
        let mut parsed = Vec::new();
        for weapon in weapons {
            if weapon.id.trim().is_empty()
                || weapon.id.len() > 128
                || !ids.insert(weapon.id.clone())
            {
                return Err(unsupported(
                    "weapon IDs must be distinct nonblank strings of at most 128 bytes",
                ));
            }
            let text = normal_mace(&weapon.item_text)?;
            if !payloads.insert(text.clone()) {
                return Err(unsupported("duplicate exact weapon alternatives"));
            }
            parsed.push((weapon.id, text));
        }
        let support_set: BTreeSet<_> = supports.iter().copied().collect();
        if support_set.len() != supports.len() {
            return Err(unsupported("duplicate support alternatives"));
        }
        parsed.sort();
        let identity = CatalogIdentity {
            schema_version: 1,
            game: "path_of_exile_2".into(),
            rules_revision: crate::runtime::UPSTREAM_REVISION.into(),
            content_fingerprint: hash(
                &serde_json::to_string(&(
                    "pob-controlled-mace-v1",
                    SOURCE,
                    hash(&template_xml),
                    &parsed,
                    &support_set,
                ))
                .map_err(|error| unsupported(&error.to_string()))?,
            ),
        };
        let active_payload = payload(
            "pob2-gem-attributes-json-v1",
            serde_json::to_string(&profile.active_attributes).unwrap(),
        );
        let active_id = format!("pob-gem:{}", active_payload.sha256);
        let support_document = Document::parse(SUPPORT_XML).unwrap();
        let support_payload = payload(
            "pob2-gem-attributes-json-v1",
            serde_json::to_string(&attributes(support_document.root_element())).unwrap(),
        );
        let support_id = format!("pob-gem:{}", support_payload.sha256);
        let mut catalog = CandidateCatalog {
            identity: identity.clone(),
            classes: BTreeMap::from([(
                "6".into(),
                ClassDefinition {
                    start_node_id: 47175,
                },
            )]),
            ascendancies: BTreeMap::new(),
            passive_nodes: BTreeMap::from([(
                47175,
                PassiveNode {
                    kind: PassiveKind::ClassStart {
                        class_ids: BTreeSet::from(["6".into()]),
                    },
                    ..Default::default()
                },
            )]),
            equipment_slots: BTreeSet::from(["Weapon 1".into()]),
            skill_slots: BTreeSet::from([SLOT.into()]),
            items: BTreeMap::new(),
            active_skills: BTreeMap::from([(
                active_id.clone(),
                ActiveSkillInstance {
                    definition_id: MAIN.into(),
                    payload: active_payload,
                    ..Default::default()
                },
            )]),
            supports: BTreeMap::new(),
            support_definition_limits: BTreeMap::new(),
            unsupported_mechanics: BTreeSet::new(),
        };
        if support_set.contains(&MaceSupportChoice::BrutalityI) {
            catalog.supports.insert(
                support_id.clone(),
                SupportInstance {
                    definition_id: SUPPORT.into(),
                    payload: support_payload,
                    compatible_active_skill_ids: BTreeSet::from([MAIN.into()]),
                    ..Default::default()
                },
            );
            catalog.support_definition_limits.insert(SUPPORT.into(), 1);
        }
        let mut choices = BTreeMap::new();
        let mut alternatives = Vec::new();
        for (id, weapon) in parsed {
            let item_payload = payload("pob2-item-text-v1", weapon.clone());
            let item_id = format!("pob-item-1:{}", item_payload.sha256);
            catalog.items.insert(
                item_id.clone(),
                ItemInstance {
                    definition_id: weapon.lines().nth(1).unwrap().into(),
                    payload: item_payload,
                    compatible_slots: BTreeSet::from(["Weapon 1".into()]),
                    ..Default::default()
                },
            );
            for support in &support_set {
                let candidate = Candidate {
                    catalog: identity.clone(),
                    class_id: "6".into(),
                    ascendancy_id: None,
                    passives: BTreeSet::new(),
                    equipment: BTreeMap::from([("Weapon 1".into(), item_id.clone())]),
                    skills: BTreeMap::from([(
                        SLOT.into(),
                        SkillAssignment {
                            active_instance_id: active_id.clone(),
                            support_instance_ids: if *support == MaceSupportChoice::None {
                                BTreeSet::new()
                            } else {
                                BTreeSet::from([support_id.clone()])
                            },
                        },
                    )]),
                };
                let choice = Choice {
                    weapon: weapon.clone(),
                    support: *support,
                };
                let xml = patch(&template_xml, &profile, &choice);
                alternatives.push(ControlledMaceAlternative {
                    id: format!(
                        "{id}/{}",
                        if *support == MaceSupportChoice::None {
                            "none"
                        } else {
                            "brutality_i"
                        }
                    ),
                    weapon_id: id.clone(),
                    support: *support,
                    xml_sha256: hash(&xml),
                    candidate: candidate.clone(),
                });
                choices.insert(candidate, choice);
            }
        }
        Ok(Self {
            template: template_xml,
            profile,
            catalog,
            alternatives,
            choices,
        })
    }
    pub fn catalog(&self) -> &CandidateCatalog {
        &self.catalog
    }
    pub fn alternatives(&self) -> &[ControlledMaceAlternative] {
        &self.alternatives
    }
    pub fn resolve_candidate(
        &self,
        weapon_id: &str,
        support: MaceSupportChoice,
    ) -> Option<&Candidate> {
        self.alternatives
            .iter()
            .find(|value| value.weapon_id == weapon_id && value.support == support)
            .map(|value| &value.candidate)
    }
    pub fn template_build(&self) -> BuildDocument {
        BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: self.template.clone(),
        }
    }
    pub fn materialize(&self, candidate: &Candidate) -> Result<BuildDocument> {
        let choice = self
            .choices
            .get(candidate)
            .ok_or(ControlledMutationError::UnknownCandidate)?;
        Ok(BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: patch(&self.template, &self.profile, choice),
        })
    }
    /// Caller budgets this fresh template attempt; it does not replace final verification.
    pub fn bind_baseline(&self, result: &EvaluationResult) -> Result<VerifiedMaceScenario> {
        let choice = Choice {
            weapon: self.profile.weapon.clone(),
            support: if self.profile.has_support {
                MaceSupportChoice::BrutalityI
            } else {
                MaceSupportChoice::None
            },
        };
        self.check_realization(&choice, result)?;
        Ok(VerifiedMaceScenario {
            identity: self.catalog.identity.clone(),
            context: result.context.clone(),
            adapter_fingerprint: result.backend.adapter_fingerprint.clone(),
            exported_scenario: exported_scenario(&result.exports[0].content)?,
        })
    }
    pub fn validate_realization(
        &self,
        candidate: &Candidate,
        result: &EvaluationResult,
        scenario: &VerifiedMaceScenario,
    ) -> Result<()> {
        let choice = self
            .choices
            .get(candidate)
            .ok_or(ControlledMutationError::UnknownCandidate)?;
        if scenario.identity != self.catalog.identity {
            return Err(mismatch("baseline belongs to another catalog"));
        }
        self.check_realization(choice, result)?;
        let expected = &scenario.context;
        let actual = &result.context;
        if result.backend.adapter_fingerprint != scenario.adapter_fingerprint
            || actual.requested != expected.requested
            || actual.calculation_mode != expected.calculation_mode
            || actual.enemy_level != expected.enemy_level
            || actual.config_inputs != expected.config_inputs
            || actual.config_placeholders != expected.config_placeholders
        {
            return Err(mismatch(
                "effective immutable scenario changed from the verified template",
            ));
        }
        let signature = exported_scenario(&result.exports[0].content)?;
        if signature != scenario.exported_scenario {
            let at = signature
                .chars()
                .zip(scenario.exported_scenario.chars())
                .position(|(a, b)| a != b)
                .unwrap_or(0);
            return Err(mismatch(&format!(
                "exported scenario changed at {at}: actual {:?}; expected {:?}",
                signature.chars().skip(at).take(160).collect::<String>(),
                scenario
                    .exported_scenario
                    .chars()
                    .skip(at)
                    .take(160)
                    .collect::<String>()
            )));
        }
        Ok(())
    }
    fn check_realization(&self, choice: &Choice, result: &EvaluationResult) -> Result<()> {
        result
            .validate_recorded()
            .map_err(|error| mismatch(&error.to_string()))?;
        if result.backend.id != "pob-poe2-mlua"
            || result.backend.rules_revision != crate::runtime::UPSTREAM_REVISION
            || result.backend.source_fingerprint != SOURCE
            || !result.diagnostic_only
        {
            return Err(mismatch(
                "wrong backend identity or missing diagnostic marker",
            ));
        }
        if result.context.requested != EvaluationOptions::default()
            || result.context.calculation_mode != "MAIN"
            || result.context.enemy_level
                != scalar_number(&self.profile.config["enemyLevel"]).unwrap() as u32
        {
            return Err(mismatch("encounter overrides or calculation mode changed"));
        }
        for (name, value) in &self.profile.config {
            if result.context.config_inputs.get(name) != Some(value) {
                return Err(mismatch(&format!("explicit configuration {name} changed")));
            }
        }
        let build = &result.build;
        if build.level != self.profile.level
            || build.class_name != "Warrior"
            || build.ascendancy_name != "None"
            || build.tree_version != "0_5"
            || build.main_socket_group != 1
            || build.skill_groups != 1
            || build.allocated_nodes != [47175]
        {
            return Err(mismatch("class, ascendancy, level, tree or group changed"));
        }
        let coverage = &result.coverage;
        let count = if choice.support == MaceSupportChoice::None {
            1
        } else {
            2
        };
        if coverage.active_skill_set_id != Some(1)
            || coverage.groups.len() != 1
            || coverage.unresolved_entry_count != 0
            || coverage.selected_minion.is_some()
        {
            return Err(mismatch(
                "unexpected skill set, unresolved or minion entries",
            ));
        }
        let selected = coverage
            .selected_player
            .as_ref()
            .ok_or_else(|| mismatch("missing selected player"))?;
        if selected.skill_id.as_deref() != Some(MAIN)
            || selected.actor != SkillActor::Player
            || selected.group_index != Some(1)
            || selected.gem_index != Some(1)
            || selected.actor_skill_index != Some(1)
            || selected.synthesized_default_attack
            || selected.minion_id.is_some()
            || selected.part_index.is_some()
            || selected.stat_set_index != Some(1)
            || selected.show_average
        {
            return Err(mismatch("selected action changed or fell back"));
        }
        let group = &coverage.groups[0];
        if group.index != 1
            || !group.enabled
            || !group.include_in_full_dps
            || group.group_count != Some(1.0)
            || group.main_active_skill != Some(1)
            || group.provenance.kind != SkillOrigin::Manual
            || group.slot.is_some()
            || group.provenance.source.is_some()
            || group.provenance.item_id.is_some()
            || group.provenance.node_id.is_some()
            || group.gems.len() != count
        {
            return Err(mismatch("manual skill group changed"));
        }
        for (index, gem) in group.gems.iter().enumerate() {
            let (skill, game, variant) = if index == 0 {
                (
                    MAIN,
                    "Metadata/Items/Gem/SkillGemPlayerDefault1HMace",
                    "PlayerDefault1HMace",
                )
            } else {
                (
                    SUPPORT,
                    "Metadata/Items/Gems/SupportGemBrutality",
                    "BrutalitySupport",
                )
            };
            if gem.index != index + 1
                || !gem.enabled
                || gem.resolution != SkillResolution::ResolvedGem
                || gem.skill_id.as_deref() != Some(skill)
                || gem.gem_game_id.as_deref() != Some(game)
                || gem.variant_id.as_deref() != Some(variant)
                || gem.level != Some(1.0)
                || gem.quality != Some(0.0)
                || gem.count != Some(1.0)
                || gem.is_support != Some(index > 0)
            {
                return Err(mismatch("exact gem identity or configuration changed"));
            }
        }
        if result.exports.len() != 1 || result.exports[0].format != BuildFormat::PathOfBuilding2Xml
        {
            return Err(mismatch("exactly one fresh PoB XML export is required"));
        }
        check_export(
            &result.exports[0].content,
            choice,
            self.profile.level,
            &result.context,
        )?;
        check_template_frame(&self.template, &result.exports[0].content)?;
        Ok(())
    }
}
fn profile(xml: &str) -> Result<Profile> {
    let document = parse(xml)?;
    let root = document.root_element();
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
        &[],
    )?;
    fixed(
        build,
        &[
            ("className", "Warrior"),
            ("ascendClassName", "None"),
            ("targetVersion", "0_1"),
            ("characterLevelAutoMode", "false"),
            ("mainSocketGroup", "1"),
            ("viewMode", "CALCS"),
        ],
    )?;
    let level = integer(build.attribute("level"), 1, 100, "character level")?;
    let tree = child(root, "Tree")?;
    only(tree, &["activeSpec"], &["Spec"])?;
    fixed(tree, &[("activeSpec", "1")])?;
    let spec = child(tree, "Spec")?;
    if !matches!(spec.attribute("classId"), Some("3" | "6")) {
        return Err(unsupported(
            "legacy classId must be 3 or canonical 6 under classInternalId 6",
        ));
    }
    only(
        spec,
        &[
            "title",
            "classId",
            "classInternalId",
            "ascendClassId",
            "treeVersion",
            "nodes",
            "masteryEffects",
        ],
        &[],
    )?;
    fixed(
        spec,
        &[
            ("classInternalId", "6"),
            ("ascendClassId", "0"),
            ("treeVersion", "0_5"),
            ("nodes", ""),
            ("masteryEffects", ""),
        ],
    )?;
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
    let group = child(set, "Skill")?;
    only(
        group,
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
        group,
        &[
            ("enabled", "true"),
            ("includeInFullDPS", "true"),
            ("groupCount", "1"),
            ("mainActiveSkill", "1"),
            ("mainActiveSkillCalcs", "1"),
        ],
    )?;
    let gems: Vec<_> = group.children().filter(Node::is_element).collect();
    if !(1..=2).contains(&gems.len()) {
        return Err(unsupported(
            "one Mace Strike and zero or one Brutality I required",
        ));
    }
    check_gem(gems[0], false, false)?;
    if gems.len() == 2 {
        check_gem(gems[1], true, false)?;
    }
    let support_range = if gems.len() == 2 {
        gems[1].range()
    } else {
        gems[0].range().end..gems[0].range().end
    };
    let items = child(root, "Items")?;
    only(items, &["activeItemSet"], &["Item", "ItemSet"])?;
    fixed(items, &[("activeItemSet", "1")])?;
    let item = child(items, "Item")?;
    only(item, &["id"], &[])?;
    fixed(item, &[("id", "1")])?;
    if item.children().count() != 1 || !item.first_child().is_some_and(|node| node.is_text()) {
        return Err(unsupported("item must contain one unsplit text payload"));
    }
    let weapon = normal_mace(item.text().unwrap_or_default())?;
    let item_set = child(items, "ItemSet")?;
    only(item_set, &["id", "title", "useSecondWeaponSet"], &["Slot"])?;
    fixed(item_set, &[("id", "1"), ("useSecondWeaponSet", "false")])?;
    let slot = child(item_set, "Slot")?;
    only(slot, &["name", "itemId"], &[])?;
    fixed(slot, &[("name", "Weapon 1"), ("itemId", "1")])?;
    let config = child(root, "Config")?;
    only(config, &["activeConfigSet"], &["ConfigSet"])?;
    fixed(config, &[("activeConfigSet", "1")])?;
    let config_set = child(config, "ConfigSet")?;
    only(config_set, &["id", "title"], &["Input"])?;
    fixed(config_set, &[("id", "1")])?;
    let mut inputs = BTreeMap::new();
    for node in config_set.children().filter(Node::is_element) {
        let name = node
            .attribute("name")
            .ok_or_else(|| unsupported("configuration name missing"))?;
        let value = scalar(node)?;
        validate_input(name, &value)?;
        if inputs.insert(name.into(), value).is_some() {
            return Err(unsupported("duplicate configuration input"));
        }
    }
    for required in INPUT_NAMES {
        if !inputs.contains_key(*required) {
            return Err(unsupported(&format!(
                "explicit configuration {required} required"
            )));
        }
    }
    if [
        "enemyPhysicalDamage",
        "enemyFireDamage",
        "enemyColdDamage",
        "enemyLightningDamage",
        "enemyChaosDamage",
    ]
    .iter()
    .all(|name| scalar_number(&inputs[*name]) == Some(0.0))
    {
        return Err(unsupported(
            "at least one incoming damage component must be positive",
        ));
    }
    for notes in root.children().filter(|node| node.has_tag_name("Notes")) {
        only(notes, &[], &[])?;
    }
    Ok(Profile {
        level,
        config: inputs,
        item_range: item.range(),
        support_range,
        weapon,
        active_attributes: attributes(gems[0]),
        has_support: gems.len() == 2,
    })
}
const INPUT_NAMES: &[&str] = &[
    "enemyIsBoss",
    "enemyLevel",
    "enemyArmour",
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
fn validate_input(name: &str, value: &Scalar) -> Result<()> {
    let valid = match (name, value) {
        ("enemyIsBoss", Scalar::Text(value)) => {
            ["None", "Boss", "Pinnacle"].contains(&value.as_str())
        }
        ("enemyDamageType", Scalar::Text(value)) => value == "Melee",
        (
            "conditionEnemyShocked"
            | "conditionEnemyChilled"
            | "conditionEnemyIgnited"
            | "conditionCritRecently"
            | "conditionBeenHitRecently",
            Scalar::Boolean(value),
        ) => !value,
        ("enemyLevel", Scalar::Number(value)) => {
            (1.0..=100.0).contains(value) && value.fract() == 0.0
        }
        (
            "enemyFireResist" | "enemyColdResist" | "enemyLightningResist" | "enemyChaosResist",
            Scalar::Number(value),
        ) => (-200.0..=100.0).contains(value),
        (
            "enemyArmour"
            | "enemyPhysicalDamage"
            | "enemyFireDamage"
            | "enemyColdDamage"
            | "enemyLightningDamage"
            | "enemyChaosDamage",
            Scalar::Number(value),
        ) => (0.0..=1_000_000.0).contains(value),
        (
            "enemyPhysicalOverwhelm" | "enemyFirePen" | "enemyColdPen" | "enemyLightningPen",
            Scalar::Number(value),
        ) => (0.0..=100.0).contains(value),
        ("enemyCritChance", Scalar::Number(value)) => *value == 0.0,
        ("enemySpeed", Scalar::Number(value)) => (1.0..=60_000.0).contains(value),
        ("multiplierNearbyEnemies", Scalar::Number(value)) => *value == 1.0,
        ("multiplierNearbyRareOrUniqueEnemies", Scalar::Number(value)) => *value == 0.0,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(unsupported(&format!(
            "unsupported configuration {name}={value:?}"
        )))
    }
}
fn check_export(xml: &str, choice: &Choice, level: u32, context: &EvaluationContext) -> Result<()> {
    let document = parse(xml)?;
    let root = document.root_element();
    let build = child(root, "Build")?;
    fixed(
        build,
        &[
            ("className", "Warrior"),
            ("ascendClassName", "None"),
            ("mainSocketGroup", "1"),
        ],
    )?;
    if integer(build.attribute("level"), 1, 100, "exported character level")? != level {
        return Err(mismatch("exported level changed"));
    }
    let tree = child(root, "Tree")?;
    fixed(tree, &[("activeSpec", "1")])?;
    let spec = child(tree, "Spec")?;
    fixed(
        spec,
        &[
            ("classId", "6"),
            ("classInternalId", "6"),
            ("ascendancyInternalId", ""),
            ("ascendClassId", "0"),
            ("treeVersion", "0_5"),
            ("nodes", "47175"),
            ("masteryEffects", ""),
        ],
    )?;
    if spec
        .attribute("secondaryAscendClassId")
        .is_some_and(|value| value != "nil")
    {
        return Err(mismatch("secondary ascendancy changed"));
    }
    for container in spec.children().filter(Node::is_element) {
        match container.tag_name().name() {
            "URL" => {}
            "Sockets" if !container.children().any(|node| node.is_element()) => {}
            "Overrides" => {
                for entry in container.children().filter(Node::is_element) {
                    if !entry.has_tag_name("AttributeOverride")
                        || entry.attributes().any(|attribute| {
                            !["strNodes", "dexNodes", "intNodes"].contains(&attribute.name())
                                || !attribute.value().is_empty()
                        })
                    {
                        return Err(mismatch("passive override added"));
                    }
                }
            }
            _ => return Err(mismatch("unsupported exported passive state")),
        }
    }
    let items = child(root, "Items")?;
    let item_set = child(items, "ItemSet")?;
    fixed(
        items,
        &[("activeItemSet", "1"), ("useSecondWeaponSet", "false")],
    )?;
    fixed(item_set, &[("id", "1"), ("useSecondWeaponSet", "false")])?;
    let equipped: Vec<_> = item_set
        .children()
        .filter(|node| node.has_tag_name("Slot") && node.attribute("itemId") != Some("0"))
        .collect();
    if equipped.len() != 1 {
        return Err(mismatch("equipped item count changed"));
    }
    fixed(equipped[0], &[("name", "Weapon 1"), ("itemId", "1")])?;
    if item_set
        .children()
        .filter(|node| node.has_tag_name("RuneSlot"))
        .any(|node| node.attribute("runeName") != Some("None"))
    {
        return Err(mismatch("equipment rune added"));
    }
    let item = child(items, "Item")?;
    only(item, &["id"], &[])?;
    fixed(item, &[("id", "1")])?;
    if item.children().count() != 1 || !item.first_child().is_some_and(|node| node.is_text()) {
        return Err(mismatch("exported item payload split"));
    }
    let actual: Vec<_> = item
        .text()
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let mut expected: Vec<_> = choice.weapon.lines().collect();
    expected.insert(4, "LevelReq: 0");
    if actual != expected {
        return Err(mismatch("exact equipped item payload changed"));
    }
    let skills = child(root, "Skills")?;
    fixed(skills, &[("activeSkillSet", "1")])?;
    let set = child(skills, "SkillSet")?;
    fixed(set, &[("id", "1")])?;
    let group = child(set, "Skill")?;
    only(
        group,
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
        group,
        &[
            ("enabled", "true"),
            ("includeInFullDPS", "true"),
            ("groupCount", "1"),
            ("mainActiveSkill", "1"),
            ("mainActiveSkillCalcs", "1"),
        ],
    )?;
    let gems: Vec<_> = group.children().filter(Node::is_element).collect();
    if gems.len()
        != if choice.support == MaceSupportChoice::None {
            1
        } else {
            2
        }
    {
        return Err(mismatch("exported support count changed"));
    }
    for (index, gem) in gems.iter().enumerate() {
        check_gem(*gem, index > 0, true)?;
    }
    let config = child(root, "Config")?;
    fixed(config, &[("activeConfigSet", "1")])?;
    let set = child(config, "ConfigSet")?;
    fixed(set, &[("id", "1")])?;
    let mut names = BTreeSet::new();
    for entry in set.children().filter(Node::is_element) {
        let kind = entry.tag_name().name();
        if !names.insert((kind, entry.attribute("name"))) {
            return Err(mismatch("duplicate exported configuration"));
        }
        match kind {
            "Input" | "Placeholder" => {
                let values = if kind == "Input" {
                    &context.config_inputs
                } else {
                    &context.config_placeholders
                };
                let actual = scalar(entry)?;
                if entry.attribute("name").and_then(|name| values.get(name)) != Some(&actual) {
                    return Err(mismatch(
                        "exported configuration differs from effective context",
                    ));
                }
            }
            "CustomModifierBlock"
                if entry.attribute("title") == Some("Default")
                    && entry.attribute("enabled") == Some("true")
                    && !entry.children().any(|node| {
                        node.is_element() || node.text().is_some_and(|text| !text.trim().is_empty())
                    }) => {}
            _ => return Err(mismatch("unsupported exported configuration")),
        }
    }
    Ok(())
}
fn check_gem(node: Node<'_, '_>, support: bool, exported: bool) -> Result<()> {
    let mut allowed = vec![
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
    ];
    if exported {
        allowed.extend([
            "corrupted",
            "corruptLevel",
            "statSetIndex",
            "statSetIndexCalcs",
        ]);
    }
    only(node, &allowed, &[])?;
    let (name, skill, game, variant) = if support {
        (
            "Brutality I",
            SUPPORT,
            "Metadata/Items/Gems/SupportGemBrutality",
            "BrutalitySupport",
        )
    } else {
        (
            "Mace Strike",
            MAIN,
            "Metadata/Items/Gem/SkillGemPlayerDefault1HMace",
            "PlayerDefault1HMace",
        )
    };
    fixed(
        node,
        &[
            ("nameSpec", name),
            ("skillId", skill),
            ("gemId", game),
            ("variantId", variant),
            ("level", "1"),
            ("quality", "0"),
            ("enabled", "true"),
            ("enableGlobal1", "true"),
            ("enableGlobal2", "true"),
            ("count", "1"),
        ],
    )?;
    for (name, expected) in [
        ("corrupted", "false"),
        ("corruptLevel", "0"),
        ("statSetIndex", "nil"),
        ("statSetIndexCalcs", "nil"),
    ] {
        if node.attribute(name).is_some_and(|value| value != expected) {
            return Err(mismatch("unsupported exported gem setting"));
        }
    }
    Ok(())
}
/// Canonical normalized XML except numeric Build outputs and the two mutable fields.
fn exported_scenario(xml: &str) -> Result<String> {
    fn visit(node: Node<'_, '_>) -> String {
        if !node.is_element() {
            return String::new();
        }
        let tag = node.tag_name().name();
        if (tag == "Gem" && node.attribute("skillId") == Some(SUPPORT))
            || (["PlayerStat", "MinionStat", "FullDPSSkill"].contains(&tag)
                && node
                    .parent_element()
                    .is_some_and(|parent| parent.has_tag_name("Build")))
        {
            return String::new();
        }
        let mut output = format!("<{tag}:{:?}>", attributes(node));
        if !(tag == "Item"
            && node
                .parent_element()
                .is_some_and(|parent| parent.has_tag_name("Items")))
        {
            // PoB serializes several source tables with Lua pairs(). Child order
            // is validated explicitly where it has semantics (active/support gems).
            let mut children: Vec<_> = node
                .children()
                .filter(Node::is_element)
                .map(visit)
                .filter(|value| !value.is_empty())
                .collect();
            children.sort();
            for child in children {
                output.push_str(&child);
            }
            for text in node
                .children()
                .filter(Node::is_text)
                .filter_map(|child| child.text())
                .filter(|text| !text.trim().is_empty())
            {
                output.push_str(&format!("{:?}", text.trim()));
            }
        }
        output.push_str(&format!("</{tag}>"));
        output
    }
    let document = parse(xml)?;
    Ok(visit(document.root_element()))
}
fn normal_mace(input: &str) -> Result<String> {
    if input.len() > 1024 {
        return Err(unsupported("normal mace text exceeds 1024 bytes"));
    }
    let text = input.replace("\r\n", "\n");
    let text = text.trim();
    let lines: Vec<_> = text.lines().collect();
    if lines.len() != 5
        || lines[0] != "Rarity: NORMAL"
        || !["Wooden Club", "Smithing Hammer"].contains(&lines[1])
        || lines[4] != "Implicits: 0"
    {
        return Err(unsupported(
            "only exact normal Wooden Club/Smithing Hammer payloads without modifiers are supported",
        ));
    }
    integer(lines[2].strip_prefix("Item Level: "), 1, 100, "item level")?;
    integer(lines[3].strip_prefix("Quality: "), 0, 20, "weapon quality")?;
    Ok(text.into())
}
fn patch(template: &str, profile: &Profile, choice: &Choice) -> String {
    let support = if choice.support == MaceSupportChoice::None {
        String::new()
    } else if profile.has_support {
        SUPPORT_XML.into()
    } else {
        format!("\n        {SUPPORT_XML}")
    };
    let mut patches = vec![
        (
            profile.item_range.clone(),
            format!("<Item id=\"1\">{}\n</Item>", escape(&choice.weapon)),
        ),
        (profile.support_range.clone(), support),
    ];
    patches.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut output = template.to_owned();
    for (range, replacement) in patches {
        output.replace_range(range, &replacement);
    }
    output
}
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn hash(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
fn payload(format: &str, content: String) -> ExactPayload {
    ExactPayload {
        format: format.into(),
        sha256: hash(&content),
        content,
    }
}
fn attributes(node: Node<'_, '_>) -> BTreeMap<String, String> {
    node.attributes()
        .map(|attribute| (attribute.name().into(), attribute.value().into()))
        .collect()
}
fn unsupported(message: &str) -> ControlledMutationError {
    ControlledMutationError::Unsupported(message.into())
}
fn mismatch(message: &str) -> ControlledMutationError {
    ControlledMutationError::Realization(message.into())
}
fn integer(value: Option<&str>, min: u32, max: u32, name: &str) -> Result<u32> {
    value
        .and_then(|text| {
            text.parse::<u32>()
                .ok()
                .filter(|value| value.to_string() == text)
        })
        .filter(|value| (min..=max).contains(value))
        .ok_or_else(|| unsupported(&format!("{name} must be an integer in {min}..={max}")))
}
fn parse(xml: &str) -> Result<Document<'_>> {
    crate::preflight::validate(xml).map_err(|error| unsupported(&error.to_string()))?;
    Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: crate::import::MAX_XML_NODES,
            entity_resolver: None,
        },
    )
    .map_err(|error| unsupported(&error.to_string()))
}
fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Result<Node<'a, 'input>> {
    let mut children = node.children().filter(|node| node.has_tag_name(name));
    let found = children
        .next()
        .ok_or_else(|| unsupported(&format!("missing {name}")))?;
    if children.next().is_some() {
        return Err(unsupported(&format!("duplicate {name}")));
    }
    Ok(found)
}
fn only(node: Node<'_, '_>, allowed_attributes: &[&str], allowed_children: &[&str]) -> Result<()> {
    if node.tag_name().namespace().is_some()
        || node.attributes().any(|attribute| {
            attribute.namespace().is_some() || !allowed_attributes.contains(&attribute.name())
        })
        || node.children().filter(Node::is_element).any(|child| {
            child.tag_name().namespace().is_some()
                || !allowed_children.contains(&child.tag_name().name())
        })
    {
        Err(unsupported(&format!(
            "unmapped attributes or children in {}",
            node.tag_name().name()
        )))
    } else {
        Ok(())
    }
}
fn fixed(node: Node<'_, '_>, expected: &[(&str, &str)]) -> Result<()> {
    for (name, value) in expected {
        if node.attribute(*name) != Some(*value) {
            return Err(unsupported(&format!(
                "{}.{} must be {value:?}",
                node.tag_name().name(),
                name
            )));
        }
    }
    Ok(())
}
fn scalar(node: Node<'_, '_>) -> Result<Scalar> {
    only(node, &["name", "number", "string", "boolean"], &[])?;
    let values: Vec<_> = ["number", "string", "boolean"]
        .into_iter()
        .filter_map(|name| node.attribute(name).map(|value| (name, value)))
        .collect();
    match values.as_slice() {
        [("number", text)] => text
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(Scalar::Number)
            .ok_or_else(|| unsupported("nonfinite configuration number")),
        [("string", text)] => Ok(Scalar::Text((*text).into())),
        [("boolean", "true")] => Ok(Scalar::Boolean(true)),
        [("boolean", "false")] => Ok(Scalar::Boolean(false)),
        _ => Err(unsupported("configuration must contain exactly one scalar")),
    }
}
fn scalar_number(value: &Scalar) -> Option<f64> {
    if let Scalar::Number(value) = value {
        Some(*value)
    } else {
        None
    }
}
/// Compare each explicitly supported source attribute outside mutated state.
/// Generated export defaults are checked by their specific validators and baseline guard.
fn check_template_frame(source_xml: &str, exported_xml: &str) -> Result<()> {
    let source = parse(source_xml)?;
    let exported = parse(exported_xml)?;
    for path in [
        &["Build"][..],
        &["Tree"],
        &["Tree", "Spec"],
        &["Skills"],
        &["Skills", "SkillSet"],
        &["Skills", "SkillSet", "Skill"],
        &["Items"],
        &["Items", "ItemSet"],
        &["Config"],
        &["Config", "ConfigSet"],
    ] {
        let mut left = source.root_element();
        let mut right = exported.root_element();
        for name in path {
            left = child(left, name)?;
            right = child(right, name)?;
        }
        for attribute in left.attributes() {
            // PoB explicitly adds the implicit class root to an empty allocation.
            if left.has_tag_name("Spec") && ["nodes", "classId"].contains(&attribute.name()) {
                continue;
            }
            if right.attribute(attribute.name()) != Some(attribute.value()) {
                return Err(mismatch(&format!(
                    "source {}.{} was not preserved",
                    left.tag_name().name(),
                    attribute.name()
                )));
            }
        }
    }
    let notes = |document: &Document<'_>| {
        document
            .root_element()
            .children()
            .find(|node| node.has_tag_name("Notes"))
            .map(|node| {
                node.children()
                    .filter(Node::is_text)
                    .filter_map(|child| child.text())
                    .collect::<String>()
                    .trim()
                    .to_owned()
            })
            .unwrap_or_default()
    };
    if notes(&source) != notes(&exported) {
        return Err(mismatch("source Notes were not preserved"));
    }
    let build = child(exported.root_element(), "Build")?;
    let buffs = child(build, "Buffs")?;
    only(buffs, &["combatList", "buffList", "curseList"], &[])?;
    fixed(
        buffs,
        &[("combatList", ""), ("buffList", ""), ("curseList", "")],
    )?;
    let timeless = child(build, "TimelessData")?;
    only(
        timeless,
        &[
            "searchList",
            "searchListFallback",
            "devotionVariant1",
            "devotionVariant2",
        ],
        &[],
    )?;
    fixed(
        timeless,
        &[
            ("searchList", ""),
            ("searchListFallback", ""),
            ("devotionVariant1", "1"),
            ("devotionVariant2", "1"),
        ],
    )?;
    if build.children().filter(Node::is_element).any(|node| {
        ![
            "PlayerStat",
            "MinionStat",
            "FullDPSSkill",
            "Buffs",
            "TimelessData",
        ]
        .contains(&node.tag_name().name())
    }) {
        return Err(mismatch(
            "exported build acquired unsupported persistent state",
        ));
    }
    Ok(())
}
