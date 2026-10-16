//! Offline source admission for one selected ordinary attack timing domain.
//! Potential suppliers retain their exact source paths, including inactive and
//! zero records. This is not a runtime evaluator or a native coverage override.
use super::family;
use flate2::read::GzDecoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, sync::OnceLock};

const MANIFEST: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const INPUT_FIELDS: [(&str, usize); 4] = [
    ("environments", 21),
    ("baseline", 18),
    ("configuration", 2),
    ("skill_construction", 1536),
];
const BODY_PATHS: [&str; 11] = [
    "src/Modules/CalcActiveSkill.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/CalcOffence.lua",
    "src/Modules/Data.lua",
    "src/Classes/ModStore.lua",
    "src/Modules/CalcDefence.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcTools.lua",
    "src/Classes/ModDB.lua",
    "src/Classes/ModList.lua",
    "src/Data/Minions.lua",
];
const SPANS: [(&str, &str, usize, usize); 17] = [
    (
        "empty-increased-more-neutral",
        "src/Modules/CalcTools.lua",
        16,
        32,
    ),
    ("root-sum-neutral", "src/Classes/ModDB.lua", 137, 163),
    ("root-more-neutral", "src/Classes/ModDB.lua", 214, 252),
    ("skill-sum-parent", "src/Classes/ModList.lua", 125, 141),
    ("skill-more-parent", "src/Classes/ModList.lua", 164, 193),
    (
        "self-cast-construction",
        "src/Modules/CalcActiveSkill.lua",
        635,
        643,
    ),
    (
        "profile-fixup-source",
        "src/Modules/CalcActiveSkill.lua",
        702,
        710,
    ),
    ("complete-selected-profile", "src/Data/Minions.lua", 83, 120),
    (
        "actor-action-speed",
        "src/Modules/CalcPerform.lua",
        1021,
        1043,
    ),
    (
        "independent-minion-store-root",
        "src/Modules/CalcPerform.lua",
        1203,
        1209,
    ),
    (
        "repeat-initialization",
        "src/Modules/CalcOffence.lua",
        1129,
        1139,
    ),
    (
        "cooldown-creation-gate",
        "src/Modules/CalcOffence.lua",
        1807,
        1814,
    ),
    (
        "ordinary-base-time-branches",
        "src/Modules/CalcOffence.lua",
        2847,
        2889,
    ),
    (
        "ordinary-rate-and-caps",
        "src/Modules/CalcOffence.lua",
        2975,
        3053,
    ),
    (
        "legacy-silver-table-factory",
        "src/Modules/CalcPerform.lua",
        120,
        125,
    ),
    (
        "ailment-modifier-enemy-destination",
        "src/Modules/CalcPerform.lua",
        3338,
        3341,
    ),
    ("server-tick-data", "src/Modules/Data.lua", 235, 245),
];
const PROOFS: [&str; 6] = [
    "self-flat-minion-store/review.json",
    "self-flat-flask-keystone/review.json",
    "self-flat-later-aura-delivery/review.json",
    "self-flat-action-factories/review.json",
    "self-flat-later-helpers/review.json",
    "self-flat-skill-construction/child-review.json",
];
const BRANCH_FIELDS: [&str; 17] = [
    "timeOverride",
    "fixedCastTime",
    "triggerTime",
    "triggerRate",
    "triggered",
    "attackSpeedMultiplier",
    "castTimeOverridesAttackTime",
    "castTimeOverride",
    "channelTimeMultiplier",
    "channelTimeOverride",
    "minChannelTime",
    "cooldown",
    "trapCooldown",
    "reloadTime",
    "boltCount",
    "hitTimeOverride",
    "maxHitRatePerEnemy",
];

pub fn domain() -> &'static Value {
    static DOMAIN: OnceLock<Value> = OnceLock::new();
    DOMAIN.get_or_init(|| family::read("timing-source-domain.json"))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn authenticated(pin: &Value) -> Vec<u8> {
    let path = family::root().join(pin["path"].as_str().unwrap());
    assert!(fs::metadata(&path).unwrap().len() <= 96 * 1024 * 1024);
    let bytes = fs::read(path).unwrap();
    assert_eq!(bytes.len(), pin["bytes"]);
    assert_eq!(hash(&bytes), pin["sha256"]);
    bytes
}
fn expand(bytes: &[u8], limit: u64) -> Vec<u8> {
    let mut raw = vec![];
    GzDecoder::new(bytes)
        .take(limit + 1)
        .read_to_end(&mut raw)
        .unwrap();
    assert!(raw.len() as u64 <= limit);
    raw
}
fn is_empty(value: &Value) -> bool {
    value.as_object().is_some_and(serde_json::Map::is_empty)
        || value.as_array().is_some_and(Vec::is_empty)
}
fn names(field: &str) -> Vec<&str> {
    domain()[field]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect()
}

fn collect(value: &Value, path: &str, entries: &mut Vec<(String, Value)>) {
    match value {
        Value::Object(object) => {
            if object.contains_key("type")
                && object
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|n| names("target_names").contains(&n))
            {
                entries.push((path.to_owned(), value.clone()));
            }
            for (key, child) in object {
                if [
                    "conditions",
                    "overrideCond",
                    "skillCond",
                    "skill_conditions",
                ]
                .contains(&key.as_str())
                    && let Some(conditions) = child.as_object()
                {
                    for name in names("condition_names") {
                        if let Some(value) = conditions.get(name) {
                            entries.push((
                                format!("{path}/{key}/{name}"),
                                json!({"direct_condition":value}),
                            ));
                        }
                    }
                }
                collect(child, &format!("{path}/{key}"), entries);
            }
        }
        Value::Array(rows) => {
            for (index, child) in rows.iter().enumerate() {
                collect(child, &format!("{path}/{index}"), entries);
            }
        }
        _ => {}
    }
}

/// Complete targeted-record census. Grouping compresses repeated captures only;
/// the hash commits every exact path and multiplicity, not just unique values.
pub fn census(value: &Value) -> (usize, Value) {
    let mut entries = vec![];
    collect(value, "", &mut entries);
    group_entries(entries)
}
fn group_entries(mut entries: Vec<(String, Value)>) -> (usize, Value) {
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let count = entries.len();
    let mut groups: Vec<(Value, Vec<String>)> = vec![];
    for (path, record) in entries {
        if let Some((_, paths)) = groups.iter_mut().find(|(r, _)| r == &record) {
            paths.push(path);
        } else {
            groups.push((record, vec![path]));
        }
    }
    let rows: Vec<_> = groups
        .into_iter()
        .map(|(record, paths)| {
            json!({"record":record,"first_path":paths[0],"occurrences":paths.len(),
            "paths_sha256":hash(&serde_json::to_vec(&paths).unwrap())})
        })
        .collect();
    (count, json!(rows))
}

fn effect_identity(effect: &Value) -> Value {
    let mut result = json!({"id":effect["id"],"fields":effect["fields"]});
    for key in ["castTime", "support"] {
        if let Some(value) = effect["granted_effect_fields"].get(key) {
            if result.get("granted_fields").is_none() {
                result["granted_fields"] = json!({});
            }
            result["granted_fields"][key] = value.clone();
        }
    }
    result
}
fn constructor_context(row: &Value) -> Value {
    let mut result = json!({});
    for key in [
        "actor_is_player",
        "actor_parent_is_player",
        "actor_profile",
        "source",
        "summoner",
        "socket_group",
        "mode",
        "phase",
        "skill_flags",
        "skill_data",
    ] {
        if let Some(value) = row.get(key) {
            result[key] = value.clone();
        }
    }
    result["active_effect"] = effect_identity(&row["active_effect"]);
    for key in ["effects", "supports"] {
        result[key] = if let Some(rows) = row[key].as_array() {
            json!(rows.iter().map(effect_identity).collect::<Vec<_>>())
        } else {
            row[key].clone()
        };
    }
    result
}
fn expected_projection<'a>(expected: &'a Value, name: &str) -> &'a Value {
    if let Some(value) = expected.get(name) {
        return value;
    }
    assert_eq!(expected["id"], "jit-on");
    assert_eq!(expected[format!("{name}_same_as")], "jit-off");
    &domain()["inputs"][0][name]
}

/// The complete constructor, unlike the sparse later diagnostic snapshot,
/// explicitly establishes selfCast and the single ordinary attack branch.
pub fn review_basic_constructor(row: &Value) -> Result<(), String> {
    if row["active_effect"]["id"] != "MinionMeleeBow"
        || row["actor_profile"] != "RaisedSkeletonSniper"
        || row["actor_is_player"] != false
        || row["actor_parent_is_player"] != true
        || row["active_effect"]["granted_effect_fields"]["castTime"] != 1
        || row["active_effect"]["fields"]["level"] != 1
        || row["active_effect"]["fields"]["quality"] != 0
        || !is_empty(&row["supports"])
        || !is_empty(&row["buffs"])
        || !is_empty(&row["skill_data"])
    {
        return Err("unreviewed Basic constructor, raw timing input, support or recipient".into());
    }
    let flags = row["skill_flags"]
        .as_object()
        .ok_or("complete Basic flags")?;
    if flags.get("attack") != Some(&Value::Bool(true))
        || flags.get("selfCast") != Some(&Value::Bool(true))
        || flags.get("weapon1Attack") != Some(&Value::Bool(true))
        || [
            "totem",
            "trap",
            "mine",
            "warcry",
            "channel",
            "triggered",
            "weapon2Attack",
        ]
        .iter()
        .any(|key| flags.contains_key(*key))
    {
        return Err("unreviewed Basic timing branch".into());
    }
    Ok(())
}

/// An observed pass corroborates the source-derived branch. Unknown fields,
/// including explicitly zero/false overrides, never become neutral defaults.
pub fn review_observed_pass(inputs: &Value) -> Result<(), String> {
    let data = inputs["skill_data"].as_object().ok_or("pass SkillData")?;
    if BRANCH_FIELDS.iter().any(|key| data.contains_key(*key))
        || data
            .keys()
            .any(|key| !["damageEffectiveness", "summonSpeed"].contains(&key.as_str()))
        || inputs["source"]["type"] != "Bow"
        || inputs["cfg"]["skillName"] != "Basic Attack"
    {
        return Err("unreviewed final ordinary timing branch or source".into());
    }
    Ok(())
}

pub fn review_selected_identity(selection: &Value, levels: &Value) -> Result<(), String> {
    let profile = &selection["profile"];
    if selection["profile_id"] != "RaisedSkeletonSniper"
        || selection["basic_id"] != "MinionMeleeBow"
        || selection["summon_id"] != "SummonSkeletalSnipersPlayer"
        || selection["selected_roots_have_no_metatable"] != true
        || profile["attackTime"].as_f64() != Some(1.5)
        || profile["weaponType1"] != "Bow"
        || !profile["weaponType2"].is_null()
        || !profile["hostile"].is_null()
        || profile.get("damageFixup").is_some()
        || !is_empty(&profile["modList"])
        || selection != &domain()["selection"]["selected"]
        || levels != &domain()["basic_levels"]["selected"]
    {
        return Err("selected intrinsic profile, ability, callback or level domain changed".into());
    }
    Ok(())
}

fn source_projection(source: &Value) -> Result<Value, String> {
    let mut projection = serde_json::Map::new();
    for (field, count) in INPUT_FIELDS {
        let rows = source["state"][field]
            .as_array()
            .ok_or("complete input field")?;
        if rows.len() != count {
            return Err(format!("incomplete selected {field} census"));
        }
        projection.insert(field.to_owned(), json!(rows));
    }
    Ok(Value::Object(projection))
}
fn review_neutral_records(rows: &Value) -> Result<(), String> {
    for entry in rows.as_array().ok_or("record groups")? {
        let record = &entry["record"];
        if record.get("direct_condition").is_some() {
            return Err("new direct condition provider, including false state".into());
        }
        let name = record["name"].as_str().ok_or("source modifier name")?;
        if [
            "SkillAttackTime",
            "TotalAttackTime",
            "TotalCastTime",
            "MinimumActionSpeed",
            "MaximumActionSpeedReduction",
            "TemporalChainsActionSpeed",
            "UnaffectedBySlows",
            "HasTrauma",
            "SpeedPerTrauma",
            "LegacyOfSilver",
            "HasMaddeningPresence",
            "SelfChillEffectIsReversed",
            "HerEmbrace",
            "UmbralWell",
            "SkillSpeedIncreasedPerGreenSupport",
            "MinionAttackSpeedAppliesToPlayer",
            "CastSpeedAppliesToAttacks",
            "ClawAttackSpeedAppliesToUnarmed",
            "SequentialProjectiles",
            "BarrageAttackTimePenalty",
            "BarrageRepeats",
            "TriggeredBySnipe",
            "Condition:Chilled",
            "Condition:Frozen",
            "Condition:Onslaught",
            "Condition:Adrenaline",
            "Condition:Fanaticism",
            "Condition:ArcaneSurge",
            "Condition:HerEmbrace",
            "Condition:TemporalChains",
            "ExtraSkillStat",
            "ExtraSkillMod",
            "ExtraSupport",
            "AffectedByAuraMod",
            "ExtraAura",
            "Keystone",
            "LegacyFlask",
            "FlasksApplyToMinion",
            "ParentNonUniqueFlasksAppliedToYou",
        ]
        .contains(&name)
            || (["Speed", "CooldownRecovery"].contains(&name) && record["type"] == "BASE")
        {
            return Err(format!("unreviewed potential timing supplier {name}"));
        }
    }
    Ok(())
}

/// Recomputes both semantic guards and the exact recipient-path census from a
/// complete retained capture. A hash-only test cannot replace these checks.
pub fn review_capture(source: &Value, id: &str) -> Result<(), String> {
    let expected = domain()["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|input| input["id"] == id)
        .ok_or("unknown source capture")?;
    if source["source_revision"] != domain()["source_revision"] {
        return Err("source revision".into());
    }
    let projection = source_projection(source)?;
    let (count, actual) = census(&projection);
    review_neutral_records(&actual)?;
    if json!(count) != expected["census_count"] || actual != expected["census"] {
        return Err(
            "potential timing supplier value, source, multiplicity or exact path changed".into(),
        );
    }
    let basic: Vec<_> = projection["skill_construction"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["active_effect"]["id"] == "MinionMeleeBow")
        .cloned()
        .collect();
    if basic.len() != 36 || &json!(basic) != expected_projection(expected, "basic_constructors") {
        return Err("complete Basic constructor closure changed".into());
    }
    for row in &basic {
        review_basic_constructor(row)?;
    }
    let contexts = projection["skill_construction"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(n, row)| (format!("/skill_construction/{n}"), constructor_context(row)))
        .collect();
    if &group_entries(contexts).1 != expected_projection(expected, "constructor_contexts") {
        return Err("constructor source, recipient, effect or support identity changed".into());
    }
    Ok(())
}

pub fn review_selected_recipes(source: &Value) -> Result<(), String> {
    let projection = json!({"items":source["items"],"tree":source["tree"],
        "skills":source["skills"],"minion_profiles":source["minion_profiles"]});
    let (count, actual) = census(&projection);
    review_neutral_records(&actual)?;
    if json!(count) != domain()["selected_inputs"]["census_count"]
        || actual != domain()["selected_inputs"]["census"]
    {
        return Err("selected recipe timing supplier or exact recipient changed".into());
    }
    let basic: Vec<_> = source["skills"]
        .as_array()
        .ok_or("skill recipes")?
        .iter()
        .filter(|row| row["id"] == "MinionMeleeBow")
        .collect();
    if basic.len() != 1 || basic[0] != &domain()["selected_inputs"]["basic_recipe"] {
        return Err("Basic recipe and skill-type authority".into());
    }
    let contexts: Vec<_> = source["skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let mut context = row.as_object().unwrap().clone();
            for key in ["resolved", "base_mods", "stat_set_base_mods"] {
                context.remove(key);
            }
            Value::Object(context)
        })
        .collect();
    if json!(contexts) != domain()["selected_inputs"]["recipe_contexts"] {
        return Err("selected recipe metadata, ownership or callbacks changed".into());
    }
    Ok(())
}

pub fn check() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(check_uncached);
}
fn check_uncached() {
    let d = domain();
    assert_eq!(d["kind"], "finite-selected-ordinary-timing-source-domain");
    assert_eq!(
        d["source_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    assert_eq!(d["source_manifest_sha256"], MANIFEST);
    assert_eq!(d["branch_fields"], json!(BRANCH_FIELDS));
    // The source projection must not shrink coherently with its expected census.
    // These seals cover the ordered review vocabulary, not inferred game values.
    for (field, count, digest) in [
        (
            "target_names",
            54,
            "3d163b029a84212bbc6662d96cd43c006155ca75ea023e4f9ae90dc59c25383b",
        ),
        (
            "condition_names",
            8,
            "b43727a1846168c86281536ad957d99d3a17bfebc9820a701818e5d501b4bba2",
        ),
        (
            "scan_tokens",
            86,
            "297a3516f3cedcb463bfd0f53bd3789b4ed9aa0ae2f4d6ad1e0ed3a6802706d8",
        ),
    ] {
        assert_eq!(d[field].as_array().unwrap().len(), count);
        assert_eq!(hash(&serde_json::to_vec(&d[field]).unwrap()), digest);
    }
    let input_proofs = d["input_proofs"].as_array().unwrap();
    assert_eq!(input_proofs.len(), 8);
    for (proof, input) in input_proofs.iter().zip([
        "base_time",
        "increased_percent",
        "more_multiplier",
        "additional_attack_time",
        "additional_cast_time",
        "action_speed_multiplier",
        "repeats",
        "server_tick_rate",
    ]) {
        assert_eq!(proof["input"], input);
    }
    assert_eq!(
        input_proofs[0]["admitted_neutral"],
        json!({"added_speed_time":0,"skill_attack_time_factor":1})
    );
    for (index, neutral) in [(2, 1), (3, 0), (4, 0), (5, 1), (6, 1)] {
        assert_eq!(input_proofs[index]["admitted_neutral"], neutral);
    }
    assert_eq!(input_proofs[7]["value"], "1 / 0.033");
    assert_eq!(
        d["scope"],
        json!({
            "original":5,"profile":"RaisedSkeletonSniper","effect":"MinionMeleeBow",
            "action":"Main Hand","selected_domain_only":true,"all_builds":false,
            "complete_build":false,"native_membership_authorized":false,
            "source_execution":false,"outputs_used_as_neutral_authority":false
        })
    );
    let manifest = expand(
        &authenticated(&d["source_manifest_archive"]),
        2 * 1024 * 1024,
    );
    assert_eq!(hash(&manifest), MANIFEST);
    let manifest: Value = serde_json::from_slice(&manifest).unwrap();
    let bodies = d["complete_bodies"].as_array().unwrap();
    assert_eq!(bodies.len(), BODY_PATHS.len());
    let mut complete = BTreeMap::new();
    for (body, required_path) in bodies.iter().zip(BODY_PATHS) {
        assert_eq!(body["source_path"], required_path);
        let raw = expand(&authenticated(&body["archive"]), 2 * 1024 * 1024);
        assert_eq!(raw.len(), body["bytes"]);
        assert_eq!(hash(&raw), body["sha256"]);
        let source_path = body["source_path"].as_str().unwrap();
        let authorities: Vec<_> = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|pin| pin["path"] == source_path)
            .collect();
        assert_eq!(authorities.len(), 1);
        assert_eq!(authorities[0]["sha256"], body["sha256"]);
        let text = String::from_utf8(raw).unwrap().replace("\r\n", "\n");
        let scan: Vec<_> = text
            .lines()
            .enumerate()
            .filter(|(_, line)| {
                names("scan_tokens")
                    .iter()
                    .any(|token| line.contains(token))
            })
            .map(|(i, line)| json!({"line":i+1,"text":line}))
            .collect();
        assert_eq!(json!(scan), body["scan"]);
        assert!(complete.insert(source_path, text).is_none());
    }
    assert_eq!(complete.len(), 11);
    let spans = d["mechanic_spans"].as_array().unwrap();
    assert_eq!(spans.len(), SPANS.len());
    for (span, (role, path, required_first, required_last)) in spans.iter().zip(SPANS) {
        assert_eq!(span["role"], role);
        assert_eq!(span["path"], path);
        let text = &complete[span["path"].as_str().unwrap()];
        let first = span["first"].as_u64().unwrap() as usize;
        let last = span["last"].as_u64().unwrap() as usize;
        assert_eq!((first, last), (required_first, required_last));
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines[first - 1..last].join("\n"), span["text"]);
    }
    let inputs = d["inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 2);
    for (input, id) in inputs.iter().zip(["jit-off", "jit-on"]) {
        assert_eq!(input["id"], id);
        assert_eq!(
            input["archive"]["path"],
            format!("data/owned/poe2/3887ae68/mana-supplier-sources/source-{id}.json.gz")
        );
        let raw = expand(&authenticated(&input["archive"]), 96 * 1024 * 1024);
        assert_eq!(raw.len(), input["bytes"]);
        assert_eq!(hash(&raw), input["sha256"]);
        review_capture(
            &serde_json::from_slice(&raw).unwrap(),
            input["id"].as_str().unwrap(),
        )
        .unwrap();
    }
    let selected: Value =
        serde_json::from_slice(&authenticated(&d["selected_inputs"]["file"])).unwrap();
    review_selected_recipes(&selected).unwrap();
    let selection: Value = serde_json::from_slice(&authenticated(&d["selection"]["file"])).unwrap();
    assert_eq!(selection["catalog"]["selected"], d["selection"]["selected"]);
    let levels: Value = serde_json::from_slice(&authenticated(&d["basic_levels"]["file"])).unwrap();
    assert_eq!(levels["selected"], d["basic_levels"]["selected"]);
    review_selected_identity(&selection["catalog"]["selected"], &levels["selected"]).unwrap();
    let proofs = d["inherited_proofs"].as_array().unwrap();
    assert_eq!(proofs.len(), PROOFS.len());
    for (pin, required_path) in proofs.iter().zip(PROOFS) {
        assert_eq!(
            pin["path"],
            format!("data/owned/poe2/3887ae68/{required_path}")
        );
        authenticated(pin);
    }
    let original = family::source_report()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "original-05")
        .unwrap();
    let mut count = 0;
    for actor in original["state"]["main"]["actors"].as_array().unwrap() {
        if actor["actor_profile"] != "RaisedSkeletonSniper" {
            continue;
        }
        for child in actor["children"].as_array().unwrap() {
            if child["effect_id"] != "MinionMeleeBow" {
                continue;
            }
            for pass in child["passes"].as_array().unwrap() {
                assert_eq!(pass["label"], "Main Hand");
                review_observed_pass(&pass["inputs"]).unwrap();
                count += 1;
            }
        }
    }
    assert_eq!(count, 1);
}
