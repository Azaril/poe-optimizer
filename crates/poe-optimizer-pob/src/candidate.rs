//! Exact, calibrated whole-build alternatives for the experimental finite search path.
//!
//! This adapter intentionally accepts only the four independently calibrated Mace
//! fixtures. It is not an arbitrary XML mutation engine or a game-legality validator.
use std::collections::{BTreeMap, BTreeSet};

use poe_optimizer_core::{
    candidate::*,
    coverage::{SkillActor, SkillOrigin, SkillResolution},
    evaluation::{BuildDocument, BuildFormat, EvaluationResult},
    options::{EvaluationOptions, Scalar},
};
use roxmltree::{Document, Node, ParsingOptions};
use sha2::{Digest, Sha256};
use thiserror::Error;

const CLASS_ID: &str = "6";
const CLASS_START: u32 = 47175;
const SKILL_SLOT: &str = "pob-group-1";
const MAIN_SKILL: &str = "Melee1HMacePlayer";
const SUPPORT_SKILL: &str = "SupportBrutalityPlayer";
const SOURCE_FINGERPRINT: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
// Frozen normalizations observed in the pinned MAIN host, guarded across all four
// fixtures below. These are drift guards, not independent mechanics expectations.
const EXPORTED_INPUT_NAMES: &[&str] = &[
    "multiplierNearbyEnemies",
    "enemyIsBoss",
    "enemyDamageType",
    "enemySpeed",
    "enemyCritChance",
    "enemyPhysicalDamage",
    "enemyLightningDamage",
    "enemyColdDamage",
    "enemyFireDamage",
    "enemyChaosDamage",
    "enemyArmour",
];
const EXPECTED_INPUTS: &str = r#"{
  "ChanceToIgnoreEnemyPhysicalDamageReductionMode": "AVERAGE",
  "ConcPathBypassCD": true,
  "FlickerStrikeBypassCD": true,
  "GamblesprintMovementSpeed": 20.0,
  "SecondsSinceInevitableCrit": 10.0,
  "VigilantStrikeBypassCD": true,
  "companionInPresence": true,
  "conditionBeenHitRecently": false,
  "conditionChampionIntimidate": true,
  "conditionCritRecently": false,
  "conditionEnemyChilled": false,
  "conditionEnemyIgnited": false,
  "conditionEnemyShocked": false,
  "doomBlastSource": "vixen",
  "elementalConfluxElement": 1.0,
  "enemyArmour": 0.0,
  "enemyChaosDamage": 0.0,
  "enemyChaosResist": 0.0,
  "enemyColdDamage": 0.0,
  "enemyColdPen": 0.0,
  "enemyColdResist": 0.0,
  "enemyCritChance": 0.0,
  "enemyDamageType": "Melee",
  "enemyFireDamage": 0.0,
  "enemyFirePen": 0.0,
  "enemyFireResist": 0.0,
  "enemyIsBoss": "None",
  "enemyLevel": 60.0,
  "enemyLightningDamage": 0.0,
  "enemyLightningPen": 0.0,
  "enemyLightningResist": 0.0,
  "enemyPhysicalDamage": 1000.0,
  "enemyPhysicalOverwhelm": 0.0,
  "enemySizePreset": "Medium",
  "enemySpeed": 1000.0,
  "inDemonForm": true,
  "multiplierCurrentManaPercentage": 100.0,
  "multiplierNearbyEnemies": 1.0,
  "multiplierNearbyRareOrUniqueEnemies": 0.0,
  "presetBossSkills": "None",
  "purpleFlameStacks": 10.0,
  "questAct 1ClearfellBeira": true,
  "questAct 1FreythornKing In The Mists": true,
  "questAct 1Ogham ManorCandlemass": true,
  "questAct 2Spires of DesharSisters of Garukhan Shrine": true,
  "questAct 2Valley of the TitansMedallion": "None",
  "questAct 3Azak BogIgnagduk": true,
  "questAct 3Jiquani's MachinariumBlackjaw": true,
  "questAct 3Venom CryptsVenom Draught": "None",
  "questAct 4Abandoned PrisonGoddess of Justice": "None",
  "questAct 4Eye of HinekoraSilent Hall": true,
  "questAct 4Eye of HinekoraTribal Medicine": "None",
  "questAct 4Halls Of The DeadNgamahu's Test": "None",
  "questAct 4Halls Of The DeadTasalio's Test": "None",
  "questAct 4Halls Of The DeadTawhoa's Test": "None",
  "questInterlude 2Khari CrossingMolten Shrine": true,
  "questInterlude 2QimahSeven Pillars": "None",
  "questInterlude 3Kriar VillageLythara": true,
  "raiseSpectreEnableBuffs": true,
  "raiseSpectreEnableCurses": true,
  "repeatMode": "AVERAGE",
  "resistancePenalty": -60.0,
  "resourceGainMode": "AVERAGE",
  "summonCompanionEnableBuffs": true,
  "summonCompanionEnableCurses": true,
  "summonElementalRelicEnableAngerAura": true,
  "summonElementalRelicEnableHatredAura": true,
  "summonElementalRelicEnableWrathAura": true,
  "targetBrandedEnemy": true,
  "touchedDebuffsCount": 10.0
}"#;
const EXPECTED_PLACEHOLDERS: &str = r#"{
  "ChillStacks": 1.0,
  "ScorchStacks": 1.0,
  "ShockStacks": 1.0,
  "conditionCorruptingCryStages": 10.0,
  "demonFormStacks": 10.0,
  "enemyArmour": 1500.0,
  "enemyChaosDamage": 25.0,
  "enemyColdDamage": 62.0,
  "enemyCritChance": 5.0,
  "enemyCritDamage": 30.0,
  "enemyCriticalWeaknessStacks": 20.0,
  "enemyDamageRollRange": 70.0,
  "enemyDistance": 20.0,
  "enemyEvasion": 591.0,
  "enemyFireDamage": 62.0,
  "enemyLevel": 60.0,
  "enemyLightningDamage": 62.0,
  "enemyPhysicalDamage": 62.0,
  "enemySpeed": 700.0,
  "multiplierCurrentEnergyShield": 100.0,
  "multiplierDifferentAmmoFired": 1.0,
  "multiplierDifferentGrenadeFired": 1.0,
  "multiplierStunnedRecently": 1.0,
  "multiplierWarcryUsedRecently": 1.0,
  "multiplierWitheredStackCountSelf": 15.0,
  "sigilOfPowerStages": 1.0
}"#;
const APPROVED_SOURCES: &[&str] = &[
    "67899c77f30c672fbe171784cdbe1c0c2de8095bc59717fdc68bd11d1ab744f6",
    "aaf6f8db883cd9bddc7da186eee85267fa022635a55f8abdd4b440d3a0a430a1",
    "1e7132ba67af27a06bc10f799f44165d973a7872701b424000457fec02d43e71",
    "203aaab49e000798a32380c5a25d414bd61ea04a32b8d646b3e9fe6948c7b2d5",
];

#[derive(Debug, Clone)]
pub struct PobBuildAlternative {
    pub id: String,
    pub xml: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PobCandidateAlternative {
    pub id: String,
    pub xml_sha256: String,
    pub candidate: Candidate,
}

#[derive(Debug, Error)]
pub enum CandidateBridgeError {
    #[error("invalid finite PoB catalog: {0}")]
    InvalidCatalog(String),
    #[error(
        "unsupported source XML: only the four committed Mace calibration fixtures are currently mapped"
    )]
    UnsupportedSource,
    #[error("candidate is not an exact member of this calibrated PoB catalog")]
    UnknownCandidate,
    #[error("requested candidate differs from the realized PoB build: {0}")]
    Realization(String),
}

#[derive(Debug)]
struct SourceEntry {
    xml: String,
    weapon_text: String,
    gems: Vec<BTreeMap<String, String>>,
}

/// Immutable materialization registry. Membership includes the catalog identity and
/// all six candidate dimensions. Source XML is returned byte-for-byte unchanged.
#[derive(Debug)]
pub struct PobCandidateCatalog {
    catalog: CandidateCatalog,
    alternatives: Vec<PobCandidateAlternative>,
    sources: BTreeMap<Candidate, SourceEntry>,
}

impl PobCandidateCatalog {
    /// Construct a calibrated finite subset. IDs are display labels; source hashes
    /// and the versioned projection define the catalog's calculation identity.
    pub fn from_builds(builds: Vec<PobBuildAlternative>) -> Result<Self, CandidateBridgeError> {
        if builds.is_empty() || builds.len() > APPROVED_SOURCES.len() {
            return Err(CandidateBridgeError::InvalidCatalog(
                "provide between one and four distinct calibrated alternatives".into(),
            ));
        }
        let mut ids = BTreeSet::new();
        let mut hashes = BTreeSet::new();
        let mut imports = Vec::new();
        for build in builds {
            if build.id.trim().is_empty() || build.id.len() > 128 || !ids.insert(build.id.clone()) {
                return Err(CandidateBridgeError::InvalidCatalog(
                    "alternative IDs must be unique, nonblank, and at most 128 bytes".into(),
                ));
            }
            let imported = crate::import::decode_build(build.xml.as_bytes())
                .map_err(|error| CandidateBridgeError::InvalidCatalog(error.to_string()))?;
            if imported.format != crate::import::ImportFormat::RawXml
                || !APPROVED_SOURCES.contains(&imported.sha256.as_str())
            {
                return Err(CandidateBridgeError::UnsupportedSource);
            }
            if !hashes.insert(imported.sha256.clone()) {
                return Err(CandidateBridgeError::InvalidCatalog(
                    "duplicate source alternatives are not separate candidates".into(),
                ));
            }
            imports.push((build.id, imported));
        }
        let fingerprint = hash(&format!(
            "pob-calibrated-mace-projection-v1\n{}\n{}\n{}",
            crate::runtime::UPSTREAM_REVISION,
            SOURCE_FINGERPRINT,
            hashes.into_iter().collect::<Vec<_>>().join("\n")
        ));
        let identity = CatalogIdentity {
            schema_version: 1,
            game: "path_of_exile_2".into(),
            rules_revision: crate::runtime::UPSTREAM_REVISION.into(),
            content_fingerprint: fingerprint,
        };
        let mut catalog = CandidateCatalog {
            identity: identity.clone(),
            classes: BTreeMap::new(),
            ascendancies: BTreeMap::new(),
            passive_nodes: BTreeMap::new(),
            equipment_slots: BTreeSet::new(),
            skill_slots: BTreeSet::new(),
            items: BTreeMap::new(),
            active_skills: BTreeMap::new(),
            supports: BTreeMap::new(),
            support_definition_limits: BTreeMap::new(),
            unsupported_mechanics: BTreeSet::new(),
        };
        catalog.classes.insert(
            CLASS_ID.into(),
            ClassDefinition {
                start_node_id: CLASS_START,
            },
        );
        catalog.passive_nodes.insert(
            CLASS_START,
            PassiveNode {
                kind: PassiveKind::ClassStart {
                    class_ids: BTreeSet::from([CLASS_ID.into()]),
                },
                point_cost: 0,
                links: BTreeSet::new(),
                unsupported_mechanics: BTreeSet::new(),
            },
        );
        catalog.equipment_slots.insert("Weapon 1".into());
        catalog.skill_slots.insert(SKILL_SLOT.into());
        let mut alternatives = Vec::new();
        let mut sources = BTreeMap::new();
        for (id, imported) in imports {
            // Approved byte identities make the deliberately narrow paths below
            // unambiguous. Parsing remains bounded and never runs XML/Lua content.
            let document = parse(&imported.xml)?;
            let root = document.root_element();
            let item = child(child(root, "Items")?, "Item")?;
            let weapon_text = item.text().unwrap_or_default().trim().to_owned();
            let item_payload = payload("pob2-item-text-v1", weapon_text.clone());
            let item_id = format!("pob-item-1:{}", item_payload.sha256);
            let weapon_name = weapon_text.lines().nth(1).unwrap_or_default().to_owned();
            catalog.items.insert(
                item_id.clone(),
                ItemInstance {
                    definition_id: weapon_name,
                    payload: item_payload,
                    compatible_slots: BTreeSet::from(["Weapon 1".into()]),
                    ..Default::default()
                },
            );
            let group = child(child(child(root, "Skills")?, "SkillSet")?, "Skill")?;
            let mut gems = Vec::new();
            let mut active_id = String::new();
            let mut support_ids = BTreeSet::new();
            for gem in group.children().filter(|node| node.has_tag_name("Gem")) {
                let attributes: BTreeMap<String, String> = gem
                    .attributes()
                    .map(|attribute| (attribute.name().into(), attribute.value().into()))
                    .collect();
                let skill_id = attributes.get("skillId").cloned().unwrap_or_default();
                let gem_payload = payload(
                    "pob2-gem-attributes-json-v1",
                    serde_json::to_string(&attributes)
                        .map_err(|error| CandidateBridgeError::InvalidCatalog(error.to_string()))?,
                );
                let instance_id = format!("pob-gem:{}", gem_payload.sha256);
                if skill_id == MAIN_SKILL {
                    active_id.clone_from(&instance_id);
                    catalog.active_skills.insert(
                        instance_id,
                        ActiveSkillInstance {
                            definition_id: skill_id,
                            payload: gem_payload,
                            ..Default::default()
                        },
                    );
                } else if skill_id == SUPPORT_SKILL {
                    support_ids.insert(instance_id.clone());
                    catalog.supports.insert(
                        instance_id,
                        SupportInstance {
                            definition_id: skill_id.clone(),
                            payload: gem_payload,
                            compatible_active_skill_ids: BTreeSet::from([MAIN_SKILL.into()]),
                            ..Default::default()
                        },
                    );
                    catalog.support_definition_limits.insert(skill_id, 1);
                } else {
                    return Err(CandidateBridgeError::UnsupportedSource);
                }
                gems.push(attributes);
            }
            let candidate = Candidate {
                catalog: identity.clone(),
                class_id: CLASS_ID.into(),
                ascendancy_id: None,
                passives: BTreeSet::new(),
                equipment: BTreeMap::from([("Weapon 1".into(), item_id)]),
                skills: BTreeMap::from([(
                    SKILL_SLOT.into(),
                    SkillAssignment {
                        active_instance_id: active_id,
                        support_instance_ids: support_ids,
                    },
                )]),
            };
            alternatives.push(PobCandidateAlternative {
                id,
                xml_sha256: imported.sha256,
                candidate: candidate.clone(),
            });
            sources.insert(
                candidate,
                SourceEntry {
                    xml: imported.xml,
                    weapon_text,
                    gems,
                },
            );
        }
        alternatives.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(Self {
            catalog,
            alternatives,
            sources,
        })
    }

    pub fn catalog(&self) -> &CandidateCatalog {
        &self.catalog
    }

    pub fn alternatives(&self) -> &[PobCandidateAlternative] {
        &self.alternatives
    }

    pub fn materialize(
        &self,
        candidate: &Candidate,
    ) -> Result<BuildDocument, CandidateBridgeError> {
        let source = self
            .sources
            .get(candidate)
            .ok_or(CandidateBridgeError::UnknownCandidate)?;
        Ok(BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: source.xml.clone(),
        })
    }

    /// Reject normalization that changes the requested candidate. This checks the
    /// pinned source identity, snapshot coverage, and newly exported active sets.
    /// It does not certify complete mechanics, acquisition, or game topology.
    pub fn validate_realization(
        &self,
        candidate: &Candidate,
        result: &EvaluationResult,
    ) -> Result<(), CandidateBridgeError> {
        let source = self
            .sources
            .get(candidate)
            .ok_or(CandidateBridgeError::UnknownCandidate)?;
        let mismatch = |message: &str| CandidateBridgeError::Realization(message.into());
        result
            .validate_recorded()
            .map_err(|error| mismatch(&error.to_string()))?;
        if result.backend.id != "pob-poe2-mlua"
            || result.backend.rules_revision != crate::runtime::UPSTREAM_REVISION
            || result.backend.source_fingerprint != SOURCE_FINGERPRINT
        {
            return Err(mismatch(
                "backend/rules/source identity is outside the calibrated projection",
            ));
        }
        let expected_inputs: BTreeMap<String, Scalar> = serde_json::from_str(EXPECTED_INPUTS)
            .map_err(|error| mismatch(&format!("invalid internal input guard: {error}")))?;
        let expected_placeholders: BTreeMap<String, Scalar> =
            serde_json::from_str(EXPECTED_PLACEHOLDERS).map_err(|error| {
                mismatch(&format!("invalid internal placeholder guard: {error}"))
            })?;
        if result.context.requested != EvaluationOptions::default()
            || result.context.calculation_mode != "MAIN"
            || result.context.enemy_level != 60
            || result.context.config_inputs != expected_inputs
            || result.context.config_placeholders != expected_placeholders
        {
            return Err(mismatch(
                "realized encounter or configuration differs from the calibrated scenario",
            ));
        }
        let summary = &result.build;
        if summary.class_name != "Warrior"
            || summary.ascendancy_name != "None"
            || summary.level != 60
            || summary.tree_version != "0_5"
            || summary.main_socket_group != 1
            || summary.skill_groups != 1
            || summary.allocated_nodes != [CLASS_START]
        {
            return Err(mismatch(
                "class, ascendancy, level, tree allocation, or selected group changed",
            ));
        }
        let coverage = &result.coverage;
        if coverage.active_skill_set_id != Some(1)
            || coverage.groups.len() != 1
            || coverage.unresolved_entry_count != 0
            || coverage.selected_minion.is_some()
        {
            return Err(mismatch(
                "skill-set coverage changed or contains unresolved entries",
            ));
        }
        let selected = coverage
            .selected_player
            .as_ref()
            .ok_or_else(|| mismatch("missing selected player action"))?;
        if selected.skill_id.as_deref() != Some(MAIN_SKILL)
            || selected.group_index != Some(1)
            || selected.gem_index != Some(1)
            || selected.synthesized_default_attack
            || selected.actor != SkillActor::Player
            || selected.actor_skill_index != Some(1)
            || selected.minion_id.is_some()
            || selected.part_index.is_some()
            || selected.stat_set_index != Some(1)
            || selected.show_average
        {
            return Err(mismatch(
                "selected player action fell back or changed ownership",
            ));
        }
        let group = &coverage.groups[0];
        if group.index != 1
            || !group.enabled
            || group.provenance.kind != SkillOrigin::Manual
            || !group.include_in_full_dps
            || group.group_count != Some(1.0)
            || group.main_active_skill != Some(1)
            || group.gems.len() != source.gems.len()
            || group.slot.is_some()
            || group.provenance.source.is_some()
            || group.provenance.item_id.is_some()
            || group.provenance.node_id.is_some()
        {
            return Err(mismatch("manual skill group changed configuration"));
        }
        for (index, (actual, expected)) in group.gems.iter().zip(&source.gems).enumerate() {
            if actual.index != index + 1
                || !actual.enabled
                || actual.resolution != SkillResolution::ResolvedGem
                || actual.skill_id.as_ref() != expected.get("skillId")
                || actual.gem_game_id.as_ref() != expected.get("gemId")
                || actual.variant_id.as_ref() != expected.get("variantId")
                || actual.level != Some(1.0)
                || actual.quality != Some(0.0)
                || actual.count != Some(1.0)
                || actual.is_support != Some(index > 0)
            {
                return Err(mismatch(
                    "active/support gem identity or configuration changed",
                ));
            }
        }
        let exports: Vec<_> = result
            .exports
            .iter()
            .filter(|export| export.format == BuildFormat::PathOfBuilding2Xml)
            .collect();
        if exports.len() != 1 {
            return Err(mismatch("exactly one fresh PoB XML export is required"));
        }
        let document = parse(&exports[0].content)?;
        let root = document.root_element();
        let config = child(root, "Config")?;
        let config_set = child(config, "ConfigSet")?;
        if config.attribute("activeConfigSet") != Some("1")
            || config_set.attribute("id") != Some("1")
            || config
                .children()
                .filter(Node::is_element)
                .any(|node| !node.has_tag_name("ConfigSet"))
        {
            return Err(mismatch("exported encounter set changed"));
        }
        let mut config_names = BTreeSet::new();
        for entry in config_set.children().filter(Node::is_element) {
            let name = entry.tag_name().name();
            if !config_names.insert((name, entry.attribute("name"))) {
                return Err(mismatch(
                    "exported configuration contains ambiguous duplicate entries",
                ));
            }
            match name {
                "Input" | "Placeholder" => {
                    let expected = if name == "Input" {
                        &expected_inputs
                    } else {
                        &expected_placeholders
                    };
                    let actual = config_scalar(entry)?;
                    if entry.attribute("name").and_then(|key| expected.get(key)) != Some(&actual) {
                        return Err(mismatch("exported encounter input or placeholder changed"));
                    }
                }
                "CustomModifierBlock"
                    if entry.attribute("title") == Some("Default")
                        && entry.attribute("enabled") == Some("true")
                        && !entry.children().any(|node| {
                            node.is_element()
                                || (node.is_text()
                                    && !node.text().unwrap_or_default().trim().is_empty())
                        }) => {}
                _ => {
                    return Err(mismatch(
                        "exported encounter acquired an unsupported configuration",
                    ));
                }
            }
        }
        if EXPORTED_INPUT_NAMES
            .iter()
            .any(|name| !config_names.contains(&("Input", Some(*name))))
            || config_names
                .iter()
                .filter(|(kind, _)| *kind == "Input")
                .count()
                != EXPORTED_INPUT_NAMES.len()
            || expected_placeholders
                .keys()
                .any(|name| !config_names.contains(&("Placeholder", Some(name.as_str()))))
            || !config_names.contains(&("CustomModifierBlock", None))
        {
            return Err(mismatch(
                "exported encounter lost required normalized inputs or placeholders",
            ));
        }
        let exported_build = child(root, "Build")?;
        for (name, expected) in [
            ("level", "60"),
            ("className", "Warrior"),
            ("ascendClassName", "None"),
            ("mainSocketGroup", "1"),
        ] {
            if exported_build.attribute(name) != Some(expected) {
                return Err(mismatch(
                    "exported build identity differs from the requested build",
                ));
            }
        }
        let tree = child(root, "Tree")?;
        let spec = child(tree, "Spec")?;
        if tree.attribute("activeSpec") != Some("1")
            || spec.attribute("classInternalId") != Some(CLASS_ID)
            || spec.attribute("ascendancyInternalId") != Some("")
            || spec.attribute("nodes") != Some("47175")
            || spec.attribute("treeVersion") != Some("0_5")
            || spec.attribute("masteryEffects") != Some("")
            || spec.attribute("ascendClassId") != Some("0")
            || spec
                .attribute("secondaryAscendClassId")
                .is_some_and(|value| value != "nil")
        {
            return Err(mismatch("exported class/ascendancy/tree selection changed"));
        }
        if spec
            .children()
            .filter(|node| node.has_tag_name("Sockets"))
            .any(|node| node.children().any(|child| child.is_element()))
        {
            return Err(mismatch("exported tree acquired a socketed item"));
        }
        for overrides in spec
            .children()
            .filter(|node| node.has_tag_name("Overrides"))
        {
            for entry in overrides.children().filter(Node::is_element) {
                if !entry.has_tag_name("AttributeOverride")
                    || entry.attributes().any(|attribute| {
                        !["strNodes", "dexNodes", "intNodes"].contains(&attribute.name())
                            || !attribute.value().is_empty()
                    })
                {
                    return Err(mismatch("exported passive override was added"));
                }
            }
        }
        let items = child(root, "Items")?;
        let item_set = child(items, "ItemSet")?;
        if items.attribute("activeItemSet") != Some("1")
            || item_set.attribute("id") != Some("1")
            || item_set.attribute("useSecondWeaponSet") != Some("false")
            || items.attribute("useSecondWeaponSet") != Some("false")
        {
            return Err(mismatch("exported item set or weapon swap changed"));
        }
        let equipped: Vec<_> = item_set
            .children()
            .filter(|node| node.has_tag_name("Slot") && node.attribute("itemId") != Some("0"))
            .collect();
        if equipped.len() != 1
            || equipped[0].attribute("name") != Some("Weapon 1")
            || equipped[0].attribute("itemId") != Some("1")
        {
            return Err(mismatch("equipped item instance or slot changed"));
        }
        if item_set
            .children()
            .filter(|node| node.has_tag_name("RuneSlot"))
            .any(|node| node.attribute("runeName") != Some("None"))
        {
            return Err(mismatch("exported equipment acquired a rune"));
        }
        let item = child(items, "Item")?;
        if item.attribute("id") != Some("1")
            || item.attributes().any(|attribute| attribute.name() != "id")
            || item.children().any(|node| !node.is_text())
            || item.children().filter(Node::is_text).count() != 1
        {
            return Err(mismatch("exported item instance ID changed"));
        }
        let lines: Vec<_> = item
            .text()
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        let mut expected: Vec<_> = source.weapon_text.lines().collect();
        // The pinned normal-item exporter inserts this derived field for both
        // independently calibrated weapons. No other item-text rewrite is allowed.
        expected.insert(4, "LevelReq: 0");
        if lines != expected {
            return Err(mismatch("equipped item payload changed"));
        }
        let skills = child(root, "Skills")?;
        let skill_set = child(skills, "SkillSet")?;
        let skill = child(skill_set, "Skill")?;
        if skills.attribute("activeSkillSet") != Some("1") || skill_set.attribute("id") != Some("1")
        {
            return Err(mismatch("exported active skill set changed"));
        }
        for (name, expected) in [
            ("mainActiveSkill", "1"),
            ("mainActiveSkillCalcs", "1"),
            ("enabled", "true"),
            ("includeInFullDPS", "true"),
            ("groupCount", "1"),
        ] {
            if skill.attribute(name) != Some(expected) {
                return Err(mismatch("exported skill group configuration changed"));
            }
        }
        if skill.attributes().any(|attribute| {
            ![
                "label",
                "mainActiveSkill",
                "mainActiveSkillCalcs",
                "enabled",
                "includeInFullDPS",
                "groupCount",
            ]
            .contains(&attribute.name())
        }) {
            return Err(mismatch(
                "exported skill group acquired an unsupported configuration",
            ));
        }
        let gems: Vec<_> = skill
            .children()
            .filter(|node| node.has_tag_name("Gem"))
            .collect();
        if gems.len() != source.gems.len() {
            return Err(mismatch("exported gem count changed"));
        }
        for (actual, expected) in gems.iter().zip(&source.gems) {
            for attribute in actual.attributes() {
                if !expected.contains_key(attribute.name()) {
                    let allowed_default = match attribute.name() {
                        "corrupted" => attribute.value() == "false",
                        "corruptLevel" => attribute.value() == "0",
                        "statSetIndex" | "statSetIndexCalcs" => attribute.value() == "nil",
                        _ => false,
                    };
                    if !allowed_default {
                        return Err(mismatch(
                            "exported gem acquired an unsupported configuration",
                        ));
                    }
                }
            }
            for (name, value) in expected {
                if actual.attribute(name.as_str()) != Some(value.as_str()) {
                    return Err(mismatch("exported active/support attributes changed"));
                }
            }
        }
        Ok(())
    }
}

fn hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn payload(format: &str, content: String) -> ExactPayload {
    ExactPayload {
        format: format.into(),
        sha256: hash(&content),
        content,
    }
}

fn parse(xml: &str) -> Result<Document<'_>, CandidateBridgeError> {
    crate::preflight::validate(xml)
        .map_err(|error| CandidateBridgeError::Realization(error.to_string()))?;
    Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: crate::import::MAX_XML_NODES,
            entity_resolver: None,
        },
    )
    .map_err(|error| CandidateBridgeError::Realization(error.to_string()))
}

fn child<'a, 'input>(
    parent: Node<'a, 'input>,
    name: &str,
) -> Result<Node<'a, 'input>, CandidateBridgeError> {
    let mut children = parent.children().filter(|node| node.has_tag_name(name));
    let found = children
        .next()
        .ok_or_else(|| CandidateBridgeError::Realization(format!("missing {name} element")))?;
    if children.next().is_some() {
        return Err(CandidateBridgeError::Realization(format!(
            "ambiguous duplicate {name} elements"
        )));
    }
    Ok(found)
}

fn config_scalar(node: Node<'_, '_>) -> Result<Scalar, CandidateBridgeError> {
    let invalid =
        || CandidateBridgeError::Realization("malformed exported configuration scalar".into());
    if node.children().any(|child| child.is_element())
        || node
            .attributes()
            .any(|attribute| !["name", "number", "string", "boolean"].contains(&attribute.name()))
    {
        return Err(invalid());
    }
    let values: Vec<_> = ["number", "string", "boolean"]
        .into_iter()
        .filter_map(|name| node.attribute(name).map(|value| (name, value)))
        .collect();
    match values.as_slice() {
        [("number", value)] => value
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
            .map(Scalar::Number)
            .ok_or_else(invalid),
        [("string", value)] => Ok(Scalar::Text((*value).into())),
        [("boolean", "true")] => Ok(Scalar::Boolean(true)),
        [("boolean", "false")] => Ok(Scalar::Boolean(false)),
        _ => Err(invalid()),
    }
}
