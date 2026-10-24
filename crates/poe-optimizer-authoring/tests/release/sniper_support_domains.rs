//! Offline source proof for intrinsic origins, independent of numerical coverage.
//! Source text is authenticated evidence, never a Lua evaluator or game parser.
use super::tracked_release;
use flate2::read::GzDecoder;
use poe_optimizer_core::{
    owned_build::DeclaredSlot, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_import::owned_mapping::{
    ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, SourceComponent,
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs::File, io::Read, path::PathBuf};

const BASE: &str = "data/owned/poe2/3887ae68/";
const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const MANIFEST: &str = "crates/poe-optimizer-pob/data/pob-source-manifest.json";
const MANIFEST_HASH: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const FILES: &[(&str, &str)] = &[
    (
        "src/Modules/CalcSetup.lua",
        "mana-supplier-sources/CalcSetup.lua.gz",
    ),
    (
        "src/Modules/CalcActiveSkill.lua",
        "configuration-distance-inputs/CalcActiveSkill.lua.gz",
    ),
    (
        "src/Modules/Data.lua",
        "configuration-enemy-critical-damage-inputs/Data.lua.gz",
    ),
    (
        "src/Modules/ModParser.lua",
        "support-structural-inventories/sources/src_Modules_ModParser.lua.gz",
    ),
    (
        "src/Data/Minions.lua",
        "self-flat-minion-store/Minions.lua.gz",
    ),
    (
        "src/Data/Skills/act_int.lua",
        "self-flat-minion-store/act_int.lua.gz",
    ),
    (
        "src/Data/Skills/minion.lua",
        "self-flat-action-factories/minion.lua.gz",
    ),
    (
        "src/Data/SkillStatMap.lua",
        "support-structural-inventories/sources/src_Data_SkillStatMap.lua.gz",
    ),
];
const SPANS: &[(&str, &str, usize, usize)] = &[
    (
        "summoner-definition",
        "src/Data/Skills/act_int.lua",
        18677,
        18799,
    ),
    ("population-definition", "src/Data/Minions.lua", 83, 120),
    (
        "basic-attack-definition",
        "src/Data/Skills/minion.lua",
        116,
        154,
    ),
    (
        "gas-arrow-definition",
        "src/Data/Skills/minion.lua",
        155,
        264,
    ),
    (
        "active-skill-constructor",
        "src/Modules/CalcActiveSkill.lua",
        144,
        234,
    ),
    (
        "minion-skill-constructor",
        "src/Modules/CalcActiveSkill.lua",
        1116,
        1185,
    ),
    (
        "support-effect-constructor",
        "src/Modules/CalcSetup.lua",
        630,
        666,
    ),
    (
        "support-candidate-construction",
        "src/Modules/CalcSetup.lua",
        1897,
        1976,
    ),
    (
        "active-skill-candidate-transfer",
        "src/Modules/CalcSetup.lua",
        2052,
        2105,
    ),
    ("skill-loader", "src/Modules/Data.lua", 910, 966),
    ("skill-mod-postprocessing", "src/Modules/Data.lua", 72, 98),
    (
        "extra-support-factory",
        "src/Modules/ModParser.lua",
        2219,
        2253,
    ),
    (
        "linked-support-factory",
        "src/Modules/ModParser.lua",
        3650,
        3659,
    ),
    ("stat-duration", "src/Data/SkillStatMap.lua", 11, 14),
    ("stat-radius", "src/Data/SkillStatMap.lua", 632, 634),
    (
        "stat-secondary-radius",
        "src/Data/SkillStatMap.lua",
        635,
        637,
    ),
    ("stat-fire-damage", "src/Data/SkillStatMap.lua", 833, 835),
    (
        "stat-physical-to-fire",
        "src/Data/SkillStatMap.lua",
        1190,
        1192,
    ),
    (
        "stat-poison-chance",
        "src/Data/SkillStatMap.lua",
        1318,
        1321,
    ),
    ("stat-arrow", "src/Data/SkillStatMap.lua", 1796, 1798),
    (
        "stat-minion-damage",
        "src/Data/SkillStatMap.lua",
        2626,
        2628,
    ),
    ("stat-commandable", "src/Data/SkillStatMap.lua", 2681, 2683),
    ("stat-reviving", "src/Data/SkillStatMap.lua", 2690, 2692),
    ("stat-revive-time", "src/Data/SkillStatMap.lua", 2693, 2696),
    ("stat-umbral-well", "src/Data/SkillStatMap.lua", 2986, 2988),
    ("skill-mod-constructors", "src/Modules/Data.lua", 56, 71),
];
// Finite manual census of every quality, alternate-quality, constant and ordinary
// stat key in the three authenticated definitions, across every stat set.
const MAPPED_STATS: &[(&str, &str, &str)] = &[
    (
        "base_skill_effect_duration",
        "stat-duration",
        "skill(\"duration\", nil)",
    ),
    (
        "active_skill_base_area_of_effect_radius",
        "stat-radius",
        "skill(\"radius\", nil)",
    ),
    (
        "active_skill_base_secondary_area_of_effect_radius",
        "stat-secondary-radius",
        "skill(\"radiusSecondary\", nil)",
    ),
    (
        "active_skill_fire_damage_+%_final",
        "stat-fire-damage",
        "mod(\"FireDamage\", \"MORE\", nil)",
    ),
    (
        "active_skill_base_physical_damage_%_to_convert_to_fire",
        "stat-physical-to-fire",
        "mod(\"SkillPhysicalDamageConvertToFire\", \"BASE\", nil)",
    ),
    (
        "display_fake_attack_hit_poison",
        "stat-poison-chance",
        "mod(\"PoisonChance\", \"BASE\", nil, ModFlag.Attack)",
    ),
    (
        "skill_can_fire_arrows",
        "stat-arrow",
        "skillFlag = \"arrow\"",
    ),
    (
        "active_skill_minion_damage_+%_final",
        "stat-minion-damage",
        "mod(\"MinionModifier\", \"LIST\", { mod = mod(\"Damage\", \"MORE\", nil) })",
    ),
    (
        "is_commandable_skill",
        "stat-commandable",
        "flag(\"Condition:CommandableSkill\")",
    ),
    (
        "is_resummoning_minion",
        "stat-reviving",
        "flag(\"RevivingMinion\")",
    ),
    (
        "minion_base_resummon_time_ms",
        "stat-revive-time",
        "mod(\"BaseReviveTime\", \"OVERRIDE\", nil)",
    ),
    (
        "skeletal_graft_buff_effect_magnitude",
        "stat-umbral-well",
        "mod(\"UmbralWellBuffValue\", \"BASE\", nil)",
    ),
];
const ABSENT_STATS: &[&str] = &[
    "display_minion_monster_type",
    "display_statset_no_hit_damage",
    "arrow_projectile_variation",
    "skill_can_fire_wand_projectiles",
    "projectile_uses_contact_position",
    "use_scaled_contact_offset",
    "maintain_projectile_direction_when_using_contact_position",
    "check_for_targets_between_initiator_and_projectile_source",
    "dummy_stat_display_nothing",
    "base_is_projectile",
    "is_area_damage",
    "visual_hit_effect_chaos_is_green",
    "skill_cannot_be_frozen",
    "skill_cannot_be_electrocuted",
    "skill_cannot_be_knocked_back",
    "skill_cannot_be_stunned",
    "quality_display_active_skill_ground_effect_area_of_effect_+%_final_per_second_max_is_gem",
    "active_skill_ground_effect_area_of_effect_+%_final_per_second",
    "active_skill_ground_effect_area_of_effect_+%_final_per_second_max",
    "display_statset_hide_usage_stats",
    "display_skill_poisons_without_hit",
];
const ARTIFACTS: &[&str] = &[
    "actor-ability-supply/bindings.json",
    "import/skill-identities.json",
    "empty-payload-inventory/source-records.json",
    "gem-support-domains/export-classifications.json",
];

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pin {
    path: String,
    bytes: usize,
    sha256: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourcePin {
    path: String,
    archive: String,
    archive_bytes: usize,
    archive_sha256: String,
    bytes: usize,
    sha256: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Span {
    role: String,
    path: String,
    first: usize,
    last: usize,
    sha256: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    schema_version: u32,
    source_revision: String,
    source_manifest: Pin,
    scope: String,
    files: Vec<SourcePin>,
    spans: Vec<Span>,
    artifacts: Vec<Pin>,
}
struct Proof {
    spans: BTreeMap<String, String>,
    artifacts: BTreeMap<String, Value>,
    stat_map: String,
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn require(condition: bool, message: &str) -> Result<(), String> {
    condition.then_some(()).ok_or_else(|| message.to_owned())
}
fn bounded(path: &str, maximum: usize) -> Result<Vec<u8>, String> {
    let file = File::open(root().join(path)).map_err(|e| e.to_string())?;
    require(
        file.metadata().map_err(|e| e.to_string())?.len() <= maximum as u64,
        "file bound",
    )?;
    let mut bytes = Vec::new();
    file.take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    require(bytes.len() <= maximum, "file grew past bound")?;
    Ok(bytes)
}
fn pinned(pin: &Pin, maximum: usize) -> Result<Vec<u8>, String> {
    require(pin.bytes <= maximum, "pin bound")?;
    let bytes = bounded(&pin.path, maximum)?;
    require(
        bytes.len() == pin.bytes && hash(&bytes) == pin.sha256,
        "artifact pin",
    )?;
    Ok(bytes)
}
fn evidence() -> Evidence {
    serde_json::from_slice(
        &bounded(
            &format!("{BASE}sniper-support-domains/evidence.json"),
            65536,
        )
        .unwrap(),
    )
    .unwrap()
}
fn checked_span(pin: &Span, text: &str) -> Result<String, String> {
    require(pin.first > 0 && pin.last >= pin.first, "span bounds")?;
    let lines: Vec<_> = text.lines().collect();
    require(pin.last <= lines.len(), "span beyond source")?;
    let result = lines[pin.first - 1..pin.last].join("\n") + "\n";
    require(
        hash(result.as_bytes()) == pin.sha256,
        "reviewed source span changed",
    )?;
    Ok(result)
}
fn authenticate(evidence: &Evidence) -> Result<Proof, String> {
    require(
        evidence.schema_version == 1
            && evidence.source_revision == REVISION
            && !evidence.scope.is_empty(),
        "source revision",
    )?;
    require(
        evidence.source_manifest.path == MANIFEST
            && evidence.source_manifest.sha256 == MANIFEST_HASH,
        "manifest authority",
    )?;
    let manifest: Value = serde_json::from_slice(&pinned(&evidence.source_manifest, 262144)?)
        .map_err(|e| e.to_string())?;
    require(
        manifest["upstream_revision"] == REVISION,
        "manifest revision",
    )?;
    require(
        evidence.files.len() == FILES.len()
            && evidence.spans.len() == SPANS.len()
            && evidence.artifacts.len() == ARTIFACTS.len(),
        "reviewed inventory",
    )?;
    let mut sources = BTreeMap::new();
    for (pin, (path, archive)) in evidence.files.iter().zip(FILES) {
        require(
            pin.path == *path && pin.archive == format!("{BASE}{archive}"),
            "source inventory",
        )?;
        require(
            pin.bytes <= 2 * 1024 * 1024 && pin.archive_bytes <= 262144,
            "source bound",
        )?;
        let compressed = bounded(&pin.archive, 262144)?;
        require(
            compressed.len() == pin.archive_bytes && hash(&compressed) == pin.archive_sha256,
            "archive pin",
        )?;
        let mut bytes = Vec::new();
        GzDecoder::new(compressed.as_slice())
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        require(
            bytes.len() == pin.bytes && hash(&bytes) == pin.sha256,
            "expanded source pin",
        )?;
        let expected = manifest["files"]
            .as_array()
            .ok_or("manifest files")?
            .iter()
            .find(|row| row["path"] == pin.path)
            .ok_or("source missing from manifest")?;
        require(
            expected["bytes"] == pin.bytes && expected["sha256"] == pin.sha256,
            "manifest source pin",
        )?;
        let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        require(!text.contains('\r'), "canonical source line endings")?;
        sources.insert(pin.path.as_str(), text);
    }
    let mut spans = BTreeMap::new();
    for (pin, (role, path, first, last)) in evidence.spans.iter().zip(SPANS) {
        require(
            pin.role == *role && pin.path == *path && pin.first == *first && pin.last == *last,
            "reviewed span inventory",
        )?;
        spans.insert(
            pin.role.clone(),
            checked_span(pin, &sources[pin.path.as_str()])?,
        );
    }
    let mut artifacts = BTreeMap::new();
    for (pin, path) in evidence.artifacts.iter().zip(ARTIFACTS) {
        require(pin.path == format!("{BASE}{path}"), "binding inventory")?;
        let value =
            serde_json::from_slice(&pinned(pin, 4 * 1024 * 1024)?).map_err(|e| e.to_string())?;
        artifacts.insert((*path).into(), value);
    }
    let stat_map = sources
        .remove("src/Data/SkillStatMap.lua")
        .ok_or("stat map source")?;
    Ok(Proof {
        spans,
        artifacts,
        stat_map,
    })
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
fn skill(key: &str) -> SkillDefId {
    SkillDefId::parse(namespace(), key).unwrap()
}
fn output(owner: &str, key: &str) -> SchemaSubject {
    SchemaSubject::Slot(SlotAddress::ActionOutput(DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(skill(owner)),
        slot: ActionOutputDefId::parse(namespace(), key).unwrap(),
    }))
}
/// Fixed reviewed subjects, independent of the production metadata being tested.
pub(super) fn declarations() -> Vec<SupportSourceDomainDeclaration> {
    vec![
        SchemaSubject::Definition(skill("def.0000000000000012").address()),
        SchemaSubject::Slot(SlotAddress::Actor(DeclaredSlot {
            declaration: SlotOwnerDefId::Skill(skill("def.0000000000000012")),
            slot: ActorSlotDefId::parse(namespace(), "def.000000000000001f").unwrap(),
        })),
        SchemaSubject::Definition(
            ActorDefId::parse(namespace(), "def.0000000000003091")
                .unwrap()
                .address(),
        ),
        SchemaSubject::Definition(skill("def.0000000000000021").address()),
        output("def.0000000000000021", "def.0000000000000022"),
        SchemaSubject::Definition(skill("def.0000000000000024").address()),
        output("def.0000000000000024", "def.0000000000000025"),
        output("def.0000000000000012", "def.0000000000000015"),
    ]
    .into_iter()
    .map(|owner| SupportSourceDomainDeclaration {
        owner,
        domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
    })
    .collect()
}

#[test]
fn authenticated_constructors_preserve_inherited_candidates_and_external_origin_boundaries() {
    check_constructor_boundaries();
}

/// Shared authenticated construction proof; candidate inheritance remains external.
pub(super) fn check_constructor_boundaries() {
    let proof = authenticate(&evidence()).unwrap();
    let span = &proof.spans;
    for (role, header) in [
        (
            "summoner-definition",
            "skills[\"SummonSkeletalSnipersPlayer\"] = {",
        ),
        (
            "population-definition",
            "minions[\"RaisedSkeletonSniper\"] = {",
        ),
        ("basic-attack-definition", "skills[\"MinionMeleeBow\"] = {"),
        (
            "gas-arrow-definition",
            "skills[\"GasShotSkeletonSniperMinion\"] = {",
        ),
    ] {
        let definition = &span[role];
        assert!(definition.starts_with(header) && definition.ends_with("}\n"));
        assert!(
            !definition.contains("function")
                && !definition.contains("ExtraSupport")
                && !definition.contains("LinkedSupport")
        );
    }
    assert!(span["population-definition"].contains(
        "skillList = {\n\t\t\"MinionMeleeBow\",\n\t\t\"GasShotSkeletonSniperMinion\",\n\t}"
    ));
    assert!(span["summoner-definition"].contains("\"RaisedSkeletonSniper\""));
    let active = &span["active-skill-constructor"];
    assert!(active.starts_with("function calcs.createActiveSkill(") && active.ends_with("end\n"));
    assert!(active.contains("supportList = supportList,"));
    assert!(active.contains("activeSkill.effectList = { activeEffect }"));
    assert!(active.contains("for _, supportEffect in ipairs(supportList) do"));
    assert!(active.contains("t_insert(activeSkill.effectList, supportEffect)"));
    // The same candidate table (including nonempty lists and duplicate positions)
    // is forwarded. effectList is separately filtered; it is NOT supportList.
    let minion = &span["minion-skill-constructor"];
    assert!(minion.starts_with("function calcs.createMinionSkills(") && minion.ends_with("end\n"));
    assert!(minion.contains("calcs.createActiveSkill(activeEffect, activeSkill.supportList, env, minion, nil, activeSkill)"));
    assert!(
        minion
            .contains("activeSkill.skillModList:List(activeSkill.skillCfg, \"ExtraMinionSkill\")")
    );
    assert!(minion.contains("t_insert(skillIdList, skill.skillId)"));
    assert!(span["skill-loader"].contains("LoadModule(\"Data/Skills/\" .. type)"));
    assert!(
        span["skill-mod-postprocessing"].starts_with("local function processMod(")
            && span["skill-mod-postprocessing"].ends_with("end\n")
    );
    // Positive contrasts prevent the proof from claiming that every source has
    // no origins. These external factories run before the reviewed constructor.
    assert!(span["extra-support-factory"].contains("mod(\"ExtraSupport\", \"LIST\""));
    assert!(span["linked-support-factory"].contains("mod(\"LinkedSupport\", \"LIST\""));
    assert!(span["support-candidate-construction"].contains("\"ExtraSupport\""));
    assert!(span["support-candidate-construction"].contains("\"LinkedSupport\""));
    assert!(
        span["active-skill-candidate-transfer"]
            .contains("addBestSupport(supportEffect, appliedSupportList, env.mode)")
    );
    assert!(span["active-skill-candidate-transfer"].contains(
        "calcs.createActiveSkill(activeEffect, appliedSupportList, env, env.player, group)"
    ));
    assert!(
        span["support-effect-constructor"]
            .contains("if not grantedEffect or not grantedEffect.support then\n\t\treturn\n\tend")
    );
}

#[test]
fn every_selected_stat_mapping_is_reviewed_without_inferring_numerical_coverage() {
    let proof = authenticate(&evidence()).unwrap();
    let definitions: Vec<_> = [
        "summoner-definition",
        "basic-attack-definition",
        "gas-arrow-definition",
    ]
    .iter()
    .map(|role| proof.spans[*role].as_str())
    .collect();
    for definition in &definitions {
        for field in ["statMap", "baseMods", "qualityMods", "levelMods"] {
            assert!(
                !definition.contains(field),
                "no local {field} overrides the reviewed shared map"
            );
        }
    }
    // The full authenticated map is a literal returned table, with only its
    // outer constructor function. These selected rows contain constant modifier
    // names; Data's make* helpers only store those arguments.
    assert_eq!(proof.stat_map.matches("function").count(), 1);
    assert!(
        proof
            .stat_map
            .contains("return function(mod, flag, skill)\n\treturn {")
    );
    let constructors = &proof.spans["skill-mod-constructors"];
    assert!(constructors.contains("name = modName,"));
    assert!(constructors.contains("return makeSkillMod(modName, \"FLAG\", true, 0, 0, ...)"));
    assert!(constructors.contains("return makeSkillMod(\"SkillData\", \"LIST\", { key = dataKey, value = dataValue }, 0, 0, ...)"));
    assert_eq!((MAPPED_STATS.len(), ABSENT_STATS.len()), (12, 21));
    for (key, role, modifier) in MAPPED_STATS {
        assert!(
            definitions
                .iter()
                .any(|definition| definition.contains(&format!("\"{key}\"")))
        );
        let row = &proof.spans[*role];
        let header = format!("[\"{key}\"] = {{");
        assert_eq!(proof.stat_map.matches(header.as_str()).count(), 1);
        assert!(row.starts_with(&header) && row.ends_with("},\n") && row.contains(*modifier));
        for factory in [
            "ExtraSupport",
            "LinkedSupport",
            "ExtraMinionSkill",
            "function",
        ] {
            assert!(!row.contains(factory));
        }
    }
    for key in ABSENT_STATS {
        assert!(
            definitions
                .iter()
                .any(|definition| definition.contains(&format!("\"{key}\"")))
        );
        assert!(
            !proof.stat_map.contains(&format!("[\"{key}\"]")),
            "{key} has no shared mapping in the authenticated table"
        );
    }
    // Absence from this source table proves no modifier from that route. It is
    // not a claim that the game mechanic is implemented or has a zero value.
}

#[test]
fn exact_owned_domains_match_independent_bindings_without_closing_numerical_coverage() {
    let proof = authenticate(&evidence()).unwrap();
    let rows = declarations();
    let release = tracked_release().release();
    let bindings = &proof.artifacts["actor-ability-supply/bindings.json"];
    let actor: ActorDefId = serde_json::from_value(bindings["actor"].clone()).unwrap();
    assert_eq!(rows[2].owner, SchemaSubject::Definition(actor.address()));
    let population = serde_json::from_value(bindings["population"].clone()).unwrap();
    assert_eq!(
        rows[1].owner,
        SchemaSubject::Slot(SlotAddress::Actor(population))
    );
    assert_eq!(bindings["abilities"].as_array().unwrap().len(), 2);
    for (i, index) in [(0, 3), (1, 5)] {
        let ability = &bindings["abilities"][i];
        let skill: SkillDefId = serde_json::from_value(ability["skill"].clone()).unwrap();
        let output = serde_json::from_value(ability["output"].clone()).unwrap();
        assert_eq!(
            rows[index].owner,
            SchemaSubject::Definition(skill.address())
        );
        assert_eq!(
            rows[index + 1].owner,
            SchemaSubject::Slot(SlotAddress::ActionOutput(output))
        );
    }
    for (effect, index) in [
        ("SummonSkeletalSnipersPlayer", 0),
        ("MinionMeleeBow", 3),
        ("GasShotSkeletonSniperMinion", 5),
    ] {
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Skill {
            effect_id: SourceComponent::Text(effect.into()),
        });
        assert!(
            matches!(release.mapping().lookup(&selector), Some(MappingOutcome::Mapped { target, basis: MappingBasis::Exact }) if *target == rows[index].owner)
        );
        assert!(
            proof.artifacts["import/skill-identities.json"]["skills"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["id"] == effect && row["support"] != true)
        );
    }
    let rules = &release.input().recipe.rules;
    for (index, row) in rows.iter().enumerate() {
        match &row.owner {
            SchemaSubject::Definition(address) => assert_eq!(
                release
                    .assembled()
                    .schema()
                    .lookup_definition(address)
                    .unwrap()
                    .address(),
                *address
            ),
            SchemaSubject::Slot(address) => assert_eq!(
                release
                    .assembled()
                    .schema()
                    .lookup_slot(address)
                    .unwrap()
                    .address(),
                *address
            ),
        }
        assert!(
            rules
                .support_discovery
                .as_ref()
                .unwrap()
                .providers
                .contains(row)
        );
        let programs = &rules
            .owners
            .iter()
            .find(|entry| entry.owner == row.owner)
            .unwrap()
            .programs;
        if index == 7 {
            assert!(
                programs.is_complete() && programs.members.is_empty(),
                "summoner output was already complete-empty"
            );
        } else {
            let SchemaClosure::Partial { gaps } = &programs.closure else {
                panic!("source proof must not close game rules")
            };
            let code = if index == 2 {
                "actor-behaviour-unconverted"
            } else {
                "remaining-game-effects-not-converted"
            };
            assert!(gaps.iter().any(|gap| gap.code.as_str() == code));
        }
    }
    let gem = SchemaSubject::Definition(
        GemDefId::parse(namespace(), "def.0000000000000011")
            .unwrap()
            .address(),
    );
    let programs = &rules
        .owners
        .iter()
        .find(|entry| entry.owner == gem)
        .unwrap()
        .programs;
    assert!(
        matches!(&programs.closure, SchemaClosure::Partial { gaps } if gaps.iter().any(|gap| gap.code.as_str() == "source-command-supply-unresolved"))
    );
    let record = proof.artifacts["empty-payload-inventory/source-records.json"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["primary_effect"] == "SummonSkeletalSnipersPlayer")
        .unwrap();
    assert_eq!(record["declared_references"].as_array().unwrap().len(), 1);
    let reference = &record["declared_references"][0];
    assert_eq!(reference["field"], "additionalGrantedEffectId1");
    assert_eq!(reference["id"], "CommandSkeletalSniperPlayer");
    assert_eq!(reference["present_in_constructed_effect_list"], false);
    assert_eq!(reference["resolves_as_effect"], false);
    assert!(proof.artifacts["gem-support-domains/export-classifications.json"]["records"].as_array().unwrap().iter().any(|row| row["effect_id"] == "CommandSkeletalSniperPlayer" && row["support"] == false));
    assert!(
        release.input().evaluation.is_none(),
        "origin evidence does not create an executable evaluation bundle"
    );
}

#[test]
fn missing_or_changed_constructor_evidence_requires_review() {
    let evidence = evidence();
    let mut incomplete = evidence.clone();
    incomplete.spans.remove(5);
    assert!(authenticate(&incomplete).is_err());
    let mut redirected = evidence.clone();
    redirected.spans[5].first += 1;
    assert!(authenticate(&redirected).is_err());
    let proof = authenticate(&evidence).unwrap();
    let reviewed = &evidence.spans[5];
    let body = &proof.spans["minion-skill-constructor"];
    let local_span = Span {
        first: 1,
        last: body.lines().count(),
        ..reviewed.clone()
    };
    assert_eq!(checked_span(&local_span, body).unwrap(), *body);
    for replacement in ["{}", "otherGroup.supportList"] {
        let changed = body.replace("activeSkill.supportList", replacement);
        assert_ne!(changed, *body);
        assert!(
            checked_span(&local_span, &changed).is_err(),
            "empty or cross-group inheritance is not the reviewed source"
        );
    }
    let mut invented = evidence.clone();
    invented.files[1].sha256 = "0".repeat(64);
    assert!(authenticate(&invented).is_err());
}
