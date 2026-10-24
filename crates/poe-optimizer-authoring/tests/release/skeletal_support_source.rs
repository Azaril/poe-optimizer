//! Finite, authenticated intrinsic source proof. No Lua execution or parser.
//! Missing mappings remain missing mechanics, never numerical zeroes.
use flate2::read::GzDecoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs::File, io::Read, path::PathBuf, sync::OnceLock};

const BASE: &str = "data/owned/poe2/3887ae68/";
const EVIDENCE_HASH: &str = "0bb9c59418518887b7e1e4b894e0a5e4c39e03808cc90967f4eb5301e384fb72";
const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const FILES: &[(&str, &str)] = &[
    (
        "src/Data/Skills/act_int.lua",
        "self-flat-minion-store/act_int.lua.gz",
    ),
    (
        "src/Data/Minions.lua",
        "self-flat-minion-store/Minions.lua.gz",
    ),
    (
        "src/Data/SkillStatMap.lua",
        "support-structural-inventories/sources/src_Data_SkillStatMap.lua.gz",
    ),
    (
        "src/Modules/Data.lua",
        "configuration-enemy-critical-damage-inputs/Data.lua.gz",
    ),
];
const ARTIFACTS: &[&str] = &[
    "import/skill-identities.json",
    "self-flat-attack-damage/selected-domain.json",
    "skeletal-inputs/bindings.json",
    "skeletal-inputs/migration.json",
];
/// Source key, owned Skill, supplied Actor, exact population slot, child source IDs.
pub(super) const CHAINS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "SummonSkeletalArsonistsPlayer",
        "0323",
        "3287",
        "3288",
        "FireBombSkeletonMinion",
        "DestructiveLinkSkeletonBombadierMinion",
    ),
    (
        "SummonSkeletalFrostMagesPlayer",
        "0326",
        "3291",
        "3292",
        "FrostBoltSkeletonMageMinion",
        "IceArmourSkeletonMageMinion",
    ),
    (
        "SummonSkeletalReaversPlayer",
        "0327",
        "329b",
        "329c",
        "MinionMeleeStep",
        "EnrageSkeletonReaverMinion",
    ),
];
const PROFILES: &[&str] = &[
    "RaisedSkeletonArsonist",
    "RaisedSkeletonFrostMage",
    "RaisedSkeletonReaver",
];
const ROLES: &[&str] = &["arsonist", "frost", "reaver"];

#[derive(Clone)]
struct Proof {
    spans: BTreeMap<String, String>,
    sources: BTreeMap<String, String>,
    artifacts: BTreeMap<String, Value>,
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn require(condition: bool, message: &str) -> Result<(), String> {
    condition.then_some(()).ok_or_else(|| message.into())
}
fn read(path: &str, maximum: usize) -> Result<Vec<u8>, String> {
    let file = File::open(root().join(path)).map_err(|e| e.to_string())?;
    require(
        file.metadata().map_err(|e| e.to_string())?.len() <= maximum as u64,
        "file bound",
    )?;
    let mut bytes = Vec::new();
    file.take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    require(bytes.len() <= maximum, "file growth")?;
    Ok(bytes)
}
fn pinned(pin: &Value, path: &str, maximum: usize) -> Result<Vec<u8>, String> {
    require(pin["path"] == path, "pin path")?;
    let bytes = read(path, maximum)?;
    require(
        pin["bytes"] == bytes.len() && pin["sha256"] == hash(&bytes),
        "artifact commitment",
    )?;
    Ok(bytes)
}
fn evidence() -> Value {
    let bytes = read(
        &format!("{BASE}skeletal-support-domains/evidence.json"),
        65536,
    )
    .unwrap();
    assert_eq!(hash(&bytes), EVIDENCE_HASH);
    serde_json::from_slice(&bytes).unwrap()
}
fn authenticate(evidence: &Value) -> Result<Proof, String> {
    require(
        evidence["schema_version"] == 1 && evidence["source_revision"] == REVISION,
        "revision",
    )?;
    let manifest: Value = serde_json::from_slice(&pinned(
        &evidence["source_manifest"],
        "crates/poe-optimizer-pob/data/pob-source-manifest.json",
        262144,
    )?)
    .map_err(|e| e.to_string())?;
    require(
        evidence["source_manifest"]["sha256"]
            == "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
            && manifest["upstream_revision"] == REVISION,
        "source authority",
    )?;
    let files = evidence["files"].as_array().ok_or("files")?;
    require(files.len() == FILES.len(), "file inventory")?;
    let mut sources = BTreeMap::new();
    for (pin, (path, archive)) in files.iter().zip(FILES) {
        require(
            pin["path"] == *path && pin["archive"] == format!("{BASE}{archive}"),
            "source path",
        )?;
        let compressed = read(&format!("{BASE}{archive}"), 262144)?;
        require(
            pin["archive_bytes"] == compressed.len() && pin["archive_sha256"] == hash(&compressed),
            "archive commitment",
        )?;
        let mut bytes = Vec::new();
        GzDecoder::new(compressed.as_slice())
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        require(
            bytes.len() <= 2 * 1024 * 1024
                && pin["bytes"] == bytes.len()
                && pin["sha256"] == hash(&bytes),
            "expanded commitment",
        )?;
        let matches: Vec<_> = manifest["files"]
            .as_array()
            .ok_or("manifest files")?
            .iter()
            .filter(|row| row["path"] == *path)
            .collect();
        require(
            matches.len() == 1
                && matches[0]["sha256"] == pin["sha256"]
                && matches[0]["bytes"] == pin["bytes"],
            "manifest join",
        )?;
        let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        require(!text.contains('\r'), "canonical source")?;
        sources.insert((*path).to_owned(), text);
    }
    let rows = evidence["spans"].as_array().ok_or("spans")?;
    require(rows.len() == 15, "full span inventory")?;
    let mut spans = BTreeMap::new();
    for row in rows {
        let text = sources
            .get(row["path"].as_str().ok_or("span path")?)
            .ok_or("span source")?;
        let lines: Vec<_> = text.lines().collect();
        let first = row["first"].as_u64().ok_or("first")? as usize;
        let last = row["last"].as_u64().ok_or("last")? as usize;
        require(
            first > 0 && first <= last && last <= lines.len(),
            "span bounds",
        )?;
        let body = lines[first - 1..last].join("\n") + "\n";
        require(
            row["sha256"] == hash(body.as_bytes()),
            "complete body commitment",
        )?;
        require(
            spans
                .insert(row["role"].as_str().ok_or("role")?.into(), body)
                .is_none(),
            "duplicate span",
        )?;
    }
    let pins = evidence["artifacts"].as_array().ok_or("artifacts")?;
    require(pins.len() == ARTIFACTS.len(), "artifact inventory")?;
    let mut artifacts = BTreeMap::new();
    for (pin, path) in pins.iter().zip(ARTIFACTS) {
        let bytes = pinned(pin, &format!("{BASE}{path}"), 4 * 1024 * 1024)?;
        artifacts.insert(
            (*path).into(),
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?,
        );
    }
    Ok(Proof {
        spans,
        sources,
        artifacts,
    })
}
fn proof() -> &'static Proof {
    static PROOF: OnceLock<Proof> = OnceLock::new();
    PROOF.get_or_init(|| authenticate(&evidence()).unwrap())
}
pub(super) fn bindings() -> &'static Value {
    &proof().artifacts["skeletal-inputs/bindings.json"]
}
pub(super) fn migration() -> &'static Value {
    &proof().artifacts["skeletal-inputs/migration.json"]
}

fn modifier(source: &str, name: &str, kind: &str) -> Value {
    json!({"flags":0,"keywordFlags":0,"name":name,"source":format!("Skill:{source}"),"type":kind})
}
fn expected_mapping(source: &str, stat: &str) -> Option<Value> {
    let mut value = match stat {
        "display_minion_monster_type"
        | "display_statset_no_hit_damage"
        | "dummy_stat_display_nothing" => return Some(Value::Null),
        "minion_base_resummon_time_ms" => {
            return Some(json!({"[1]":modifier(source,"BaseReviveTime","OVERRIDE"),"div":1000}));
        }
        "base_number_of_projectiles" => {
            return Some(json!({"[1]":modifier(source,"ProjectileCount","BASE"),"base":-1}));
        }
        "is_resummoning_minion" => {
            let mut m = modifier(source, "RevivingMinion", "FLAG");
            m["value"] = json!(true);
            m
        }
        "skeletal_graft_buff_effect_magnitude" => modifier(source, "UmbralWellBuffValue", "BASE"),
        "base_spirit_reservation_efficiency_+%" => {
            modifier(source, "SpiritReservationEfficiency", "INC")
        }
        "armour_break_physical_damage_%_dealt_as_armour_break" => {
            let mut m = modifier(source, "Condition:CanArmourBreak", "FLAG");
            m["value"] = json!(true);
            m["[1]"] =
                json!({"type":"GlobalEffect","effectType":"Buff","effectName":"ArmourBreak"});
            m
        }
        "active_skill_minion_damage_+%_final"
        | "attack_speed_+%_per_rage"
        | "minion_rage_effect_+%" => {
            let (name, kind) = match stat {
                "active_skill_minion_damage_+%_final" => ("Damage", "MORE"),
                "attack_speed_+%_per_rage" => ("Speed", "INC"),
                _ => ("RageEffect", "INC"),
            };
            let mut inner = modifier(source, name, kind);
            if stat == "attack_speed_+%_per_rage" {
                inner["flags"] = json!(1);
                inner["[1]"] = json!({"type":"Multiplier","var":"RageEffect"});
            }
            let mut outer = modifier(source, "MinionModifier", "LIST");
            outer["value"] = json!({"mod":inner});
            outer
        }
        _ => return None,
    };
    Some(json!({"[1]":value.take()}))
}
fn stats(index: usize) -> Vec<&'static str> {
    let mut result = vec![
        "display_minion_monster_type",
        "display_statset_no_hit_damage",
        "minion_base_resummon_time_ms",
        "is_resummoning_minion",
        "skeletal_graft_buff_effect_magnitude",
    ];
    result.extend(match index {
        0 => vec!["dummy_stat_display_nothing", "base_number_of_projectiles"],
        1 => vec![
            "active_skill_minion_damage_+%_final",
            "base_spirit_reservation_efficiency_+%",
        ],
        _ => vec![
            "armour_break_physical_damage_%_dealt_as_armour_break",
            "attack_speed_+%_per_rage",
            "minion_rage_effect_+%",
        ],
    });
    result.sort_unstable();
    result
}
fn check_capture(capture: &Value) -> Result<(), String> {
    require(capture["source_revision"] == REVISION, "capture revision")?;
    for field in [
        "native_coverage_authorized",
        "universal_absence",
        "callbacks_executed",
        "supplied_actor_buffs_applied",
        "unobserved_dynamic_flags_inferred_false",
    ] {
        require(capture["scope"][field] == false, "capture scope")?;
    }
    require(
        capture["scope"]["requires_exact_selected_source_inventory"] == true,
        "inventory boundary",
    )?;
    let mut total = 0;
    let mut missing = 0;
    for (index, (id, _, _, _, _, _)) in CHAINS.iter().enumerate() {
        let rows: Vec<_> = capture["skills"]
            .as_array()
            .ok_or("captured skills")?
            .iter()
            .filter(|row| row["id"] == *id)
            .collect();
        require(rows.len() == 1, "unique captured parent")?;
        let row = rows[0];
        require(
            row["callbacks"] == json!({})
                && row["base_mods"] == json!({"aliases":[],"callbacks":[],"value":null}),
            "parent callbacks/base modifiers",
        )?;
        require(
            row["stat_set_base_mods"]
                == json!([{"records":{"aliases":[],"callbacks":[],"value":null},"set":1}]),
            "all stat-set base modifiers",
        )?;
        require(
            row["metadata"]["support"].is_null()
                && row["metadata"]["minionList"] == json!({"[1]":PROFILES[index]}),
            "profile identity",
        )?;
        let resolved = row["resolved"].as_array().ok_or("resolved mappings")?;
        let mut observed = Vec::new();
        for mapping in resolved {
            let stat = mapping["stat"].as_str().ok_or("stat")?;
            let expected = expected_mapping(id, stat).ok_or("unreviewed stat")?;
            require(
                mapping["set"] == 1
                    && mapping["mapping"] == json!({"aliases":[],"callbacks":[],"value":expected}),
                "exact nested mapping",
            )?;
            if expected.is_null() {
                missing += 1;
            }
            observed.push(stat);
            total += 1;
        }
        observed.sort_unstable();
        require(observed == stats(index), "complete stat census")?;
        let profiles: Vec<_> = capture["minion_profiles"]
            .as_array()
            .ok_or("captured profiles")?
            .iter()
            .filter(|row| row["id"] == PROFILES[index])
            .collect();
        require(profiles.len() == 1, "unique captured profile")?;
        let mods = if index == 2 {
            json!({"[1]":{"flags":0,"keywordFlags":0,"name":"Condition:CanGainRage","type":"FLAG","value":true}})
        } else {
            json!({})
        };
        require(
            *profiles[0]
                == json!({"id":PROFILES[index],"callbacks":{},"mod_list":{"aliases":[],"callbacks":[],"value":mods}}),
            "complete intrinsic profile modifiers",
        )?;
    }
    require(total == 22 && missing == 7, "mapping census totals")
}

pub(super) fn check_proof() {
    super::sniper_support_domains::check_constructor_boundaries();
    let proof = proof();
    check_capture(&proof.artifacts["self-flat-attack-damage/selected-domain.json"]).unwrap();
    let catalog = &proof.artifacts["import/skill-identities.json"];
    for (index, (id, _, _, _, first, second)) in CHAINS.iter().enumerate() {
        let body = &proof.spans[&format!("{}-parent", ROLES[index])];
        assert!(body.starts_with(&format!("skills[\"{id}\"] = {{")) && body.ends_with("}\n"));
        let profile = &proof.spans[&format!("{}-profile", ROLES[index])];
        assert!(
            profile.starts_with(&format!("minions[\"{}\"] = {{", PROFILES[index]))
                && profile.ends_with("}\n")
        );
        for source in [body, profile] {
            for forbidden in [
                "function",
                "ExtraSupport",
                "LinkedSupport",
                "ExtraMinionSkill",
                "baseMods",
                "qualityMods",
                "levelMods",
            ] {
                assert!(
                    !source.contains(forbidden),
                    "unexpected intrinsic constructor {forbidden}"
                );
            }
        }
        assert!(body.contains(&format!(
            "minionList = {{\n\t\t\"{}\",\n\t}}",
            PROFILES[index]
        )));
        assert!(profile.contains(&format!(
            "skillList = {{\n\t\t\"{first}\",\n\t\t\"{second}\",\n\t}}"
        )));
        assert_eq!(body.matches("spiritReservationFlat =").count(), 40);
        assert_eq!(body.matches("actorLevel =").count(), 40);
        assert!(body.contains("{ \"minion_base_resummon_time_ms\", 12000 }"));
        assert!(body.contains(&format!(
            "{{ \"skeletal_graft_buff_effect_magnitude\", {} }}",
            [15, 35, 6][index]
        )));
        for stat in stats(index) {
            assert!(
                body.contains(&format!("\"{stat}\"")),
                "full quality/alternate/constant/stat census"
            );
        }
        assert_eq!(body.matches("statMap =").count(), usize::from(index == 2));
        let entries: Vec<_> = catalog["skills"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["id"] == *id)
            .collect();
        assert_eq!(entries.len(), 1);
        assert!(entries[0]["support"].is_null());
        let declarations: Vec<_> = catalog["skill_declarations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["id"] == *id && r["index"] == entries[0]["winning_declaration"])
            .collect();
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0]["source"]["sha256"], hash(body.as_bytes()));
        assert_eq!(
            declarations[0]["source"]["path"],
            "src/Data/Skills/act_int.lua"
        );
        let family = &bindings()["families"][index];
        assert_eq!(
            family["primary"]["key"],
            format!("def.000000000000{}", CHAINS[index].1)
        );
        assert_eq!(family["minion"]["source_id"], PROFILES[index]);
        assert_eq!(family["first_child"]["skill_id"], *first);
        assert_eq!(family["unreviewed_child"]["skill_id"], *second);
        let gems: Vec<_> = catalog["gems"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["primary_effect_id"] == *id)
            .collect();
        assert_eq!(gems, vec![&family["physical_identity"]]);
        assert_eq!(gems[0]["effect_list"], json!([id]));
        assert_eq!(
            gems[0]["constructed_additional_effects"],
            json!([{"index":1,"id":family["missing_command"]}])
        );
        assert!(
            catalog["skills"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["id"] != family["missing_command"])
        );
    }
    let map = &proof.sources["src/Data/SkillStatMap.lua"];
    assert_eq!(map.matches("function").count(), 1);
    assert!(map.contains("return function(mod, flag, skill)\n\treturn {"));
    for (role, key, mechanic) in [
        (
            "projectiles",
            "base_number_of_projectiles",
            "mod(\"ProjectileCount\", \"BASE\", nil)",
        ),
        (
            "minion-damage",
            "active_skill_minion_damage_+%_final",
            "mod(\"MinionModifier\", \"LIST\", { mod = mod(\"Damage\", \"MORE\", nil) })",
        ),
        (
            "reviving",
            "is_resummoning_minion",
            "flag(\"RevivingMinion\")",
        ),
        (
            "revive-time",
            "minion_base_resummon_time_ms",
            "mod(\"BaseReviveTime\", \"OVERRIDE\", nil)",
        ),
        (
            "reservation",
            "base_spirit_reservation_efficiency_+%",
            "mod(\"SpiritReservationEfficiency\", \"INC\", nil)",
        ),
        (
            "armour-break",
            "armour_break_physical_damage_%_dealt_as_armour_break",
            "flag(\"Condition:CanArmourBreak\", { type = \"GlobalEffect\", effectType = \"Buff\", effectName = \"ArmourBreak\" })",
        ),
        (
            "graft",
            "skeletal_graft_buff_effect_magnitude",
            "mod(\"UmbralWellBuffValue\", \"BASE\", nil)",
        ),
    ] {
        let header = format!("[\"{key}\"] = {{");
        assert_eq!(map.matches(&header).count(), 1);
        let row = &proof.spans[role];
        assert!(row.starts_with(&header) && row.ends_with("},\n") && row.contains(mechanic));
        for forbidden in [
            "ExtraSupport",
            "LinkedSupport",
            "ExtraMinionSkill",
            "function",
        ] {
            assert!(!row.contains(forbidden));
        }
    }
    for key in [
        "display_minion_monster_type",
        "display_statset_no_hit_damage",
        "dummy_stat_display_nothing",
    ] {
        assert!(!map.contains(&format!("[\"{key}\"]")));
    }
    let reaver = &proof.spans["reaver-parent"];
    assert!(reaver.contains("mod(\"MinionModifier\", \"LIST\", { mod = mod(\"Speed\", \"INC\", nil, ModFlag.Attack, 0, { type = \"Multiplier\", var = \"RageEffect\" }) })"));
    assert!(reaver.contains(
        "mod(\"MinionModifier\", \"LIST\", { mod = mod(\"RageEffect\", \"INC\", nil) })"
    ));
    assert!(
        proof.spans["reaver-profile"].contains("mod(\"Condition:CanGainRage\", \"FLAG\", true)")
    );
    let constructors = &proof.spans["mod-constructors"];
    assert!(
        constructors.contains("name = modName,")
            && constructors.contains("return makeSkillMod(modName, \"FLAG\", true, 0, 0, ...)")
    );
    let loader = &proof.spans["profile-loader"];
    assert!(
        loader.contains("data.minions = LoadModule(\"Data/Minions\")(makeSkillMod, makeFlagMod)")
    );
    assert!(
        loader.contains("data.minions[name] = spectre")
            && loader.contains("mod.source = \"Minion:\"..minion.name")
    );
    // The separate Spectre merge is retained, not treated as an empty modifier
    // source or evidence about every possible supplied Actor/profile.
}

#[test]
fn complete_parent_profile_and_mapping_sources_preserve_every_intrinsic_output() {
    check_proof();
}

#[test]
fn missing_duplicate_or_changed_nested_mappings_and_profile_outputs_are_rejected() {
    let original = &proof().artifacts["self-flat-attack-damage/selected-domain.json"];
    check_capture(original).unwrap();
    let index = original["skills"]
        .as_array()
        .unwrap()
        .iter()
        .position(|r| r["id"] == CHAINS[2].0)
        .unwrap();
    for change in 0..5 {
        let mut changed = original.clone();
        let mappings = changed["skills"][index]["resolved"].as_array_mut().unwrap();
        match change {
            0 => {
                mappings.pop();
            }
            1 => mappings.push(mappings[0].clone()),
            2 => {
                let r = mappings
                    .iter_mut()
                    .find(|r| r["stat"] == "attack_speed_+%_per_rage")
                    .unwrap();
                r["mapping"]["value"]["[1]"]["value"]["mod"]["name"] = json!("ExtraSupport");
            }
            3 => {
                mappings
                    .iter_mut()
                    .find(|r| r["stat"] == "display_minion_monster_type")
                    .unwrap()["mapping"]["value"] = json!({});
            }
            _ => {
                changed["minion_profiles"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["id"] == PROFILES[2])
                    .unwrap()["mod_list"]["value"] = json!({});
            }
        }
        assert!(check_capture(&changed).is_err());
    }
}

#[test]
fn source_pins_require_complete_bodies_and_cannot_redirect_authority() {
    let original = evidence();
    authenticate(&original).unwrap();
    for change in 0..4 {
        let mut changed = original.clone();
        match change {
            0 => {
                changed["spans"].as_array_mut().unwrap().pop();
            }
            1 => changed["spans"][0]["last"] = json!(18169),
            2 => changed["files"][0]["sha256"] = json!("0".repeat(64)),
            _ => {
                changed["artifacts"][2]["path"] =
                    json!(format!("{BASE}actor-ability-supply/bindings.json"))
            }
        }
        assert!(authenticate(&changed).is_err());
    }
}
