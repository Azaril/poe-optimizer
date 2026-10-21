//! Bounded source-side destination proof for the selected Sniper's later helpers.
//! No native coverage, numeric parity, or arbitrary future-pass exclusion is added.
#[allow(dead_code)]
#[path = "owned_self_flat_minion_store.rs"]
pub(super) mod minion_store;

use flate2::read::GzDecoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::PathBuf, sync::OnceLock};

pub const DIR: &str = "data/owned/poe2/3887ae68/self-flat-later-helpers";
const BODIES: &str = "c8b99d9a6d63ce813641a07969baa4a8bf3a417f02545b917849fd1da5b858ef";
const MANIFEST: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const MINION_REVIEW: &str = "eaa1bbd66e770a85f0a49ed4e834780279d52abf810872294cfa9b08e47c8266";
const BASIC: &str = "MinionMeleeBow";
const EXCLUDED_DIRECT_NAMES: [&str; 11] = [
    "PhysicalMin",
    "PhysicalMax",
    "GainMainHandDmgFromParent",
    "UseHollowPalmDamage",
    "UseFacebreakerItemDamage",
    "Condition:HollowPalm",
    "ExtraSkillMod",
    "ExtraSkillStat",
    "AffectedByAuraMod",
    "MinionModifier",
    "Keystone",
];
const WRITER_METHODS: [&str; 7] = [
    ":NewMod(",
    ":AddMod(",
    ":AddList(",
    ":ScaleAddMod(",
    ":ScaleAddList(",
    ":CopyList(",
    ":MergeMod(",
];
const WRITER_ROLES: [&str; 8] = [
    "attributes-and-calculation-callees",
    "enemy-modifier-transport",
    "misc",
    "charges",
    "action-speed",
    "defence-complete",
    "breakdown-complete",
    "player-offence-minion-writes",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest(value: &Value) -> String {
    hash(&serde_json::to_vec(value).unwrap())
}
fn ensure(ok: bool, why: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(why.into()) }
}
fn bytes(path: &str, limit: u64) -> Result<Vec<u8>, String> {
    let path = root().join(path);
    ensure(
        fs::metadata(&path).map_err(|e| e.to_string())?.len() <= limit,
        "bounded retained file",
    )?;
    fs::read(path).map_err(|e| e.to_string())
}
fn rows(value: &Value) -> Result<&[Value], String> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| "explicit sequence required".into())
}
fn empty(value: &Value) -> bool {
    value.as_object().is_some_and(serde_json::Map::is_empty)
        || value.as_array().is_some_and(Vec::is_empty)
}
pub fn read(name: &str) -> Value {
    serde_json::from_slice(&bytes(&format!("{DIR}/{name}"), 2 * 1024 * 1024).unwrap()).unwrap()
}
pub fn fixture() -> &'static Value {
    minion_store::fixture()
}
fn span<'a>(body: &'a Value, role: &str) -> Result<&'a str, String> {
    let matches = rows(&body["spans"])?
        .iter()
        .filter(|s| s["role"] == role)
        .collect::<Vec<_>>();
    ensure(matches.len() == 1, "exact complete reviewed role")?;
    matches[0]["body"]
        .as_str()
        .ok_or_else(|| "source body".into())
}

/// These checks are regression assertions on a manually reviewed, pinned whole
/// source. They are deliberately not an alternative Lua parser/name allowlist.
pub fn review_semantics(body: &Value) -> Result<(), String> {
    let mut sites = Vec::new();
    for s in rows(&body["spans"])? {
        if !WRITER_ROLES.contains(&s["role"].as_str().ok_or("role")?) {
            continue;
        }
        let first = s["first"].as_u64().ok_or("first")?;
        for (offset, line) in s["body"].as_str().ok_or("body")?.lines().enumerate() {
            if WRITER_METHODS.iter().any(|method| line.contains(method)) {
                sites.push(json!({"role":s["role"],"line":first+offset as u64,"body":line.trim()}));
            }
        }
    }
    ensure(
        sites.len() == 125 && json!(sites) == body["writer_sites"],
        "complete reviewed writer sites",
    )?;
    let review = &body["review"];
    ensure(
        review["actor_relations"]
            == json!({"Player":"Enemy","Enemy":"Player","selected_friendly_minion":"Enemy"}),
        "exact Actor destinations",
    )?;
    ensure(
        review["dynamic_name_domains"]
            == json!({
                "resist":["Fire","Cold","Lightning","Chaos"],
                "elemental_resist":["Fire","Cold","Lightning"],
                "converted_nondefence_resource":["Life","Mana"],
                "regeneration_resource":["Mana","Life","EnergyShield","Rage","Ward"]
            }),
        "closed computed writer name domains",
    )?;
    let mut literal = 0;
    let mut computed = 0;
    for site in &sites {
        let line = site["body"].as_str().ok_or("writer body")?;
        let Some((_, args)) = line.split_once(":NewMod(") else {
            continue;
        };
        if let Some(quoted) = args.strip_prefix('"')
            && let Some((name, tail)) = quoted.split_once('"')
            && tail.starts_with(',')
        {
            ensure(
                !EXCLUDED_DIRECT_NAMES.contains(&name),
                "direct writer creates reviewed flat or factory name",
            )?;
            literal += 1;
        } else {
            // These are all computed-name writes in the complete reviewed
            // bodies. Domain expansion is explicit; nested EnemyModifier and
            // ExtraAura payloads are handled separately, never by this check.
            ensure(
                site["role"] == "defence-complete"
                    && [876, 896, 899, 902, 921, 1440, 1441, 1713, 1723, 1743]
                        .contains(&site["line"].as_u64().ok_or("computed line")?),
                "unreviewed computed writer",
            )?;
            computed += 1;
        }
    }
    ensure(
        literal == 114 && computed == 10,
        "complete direct writer name census",
    )?;
    let domains = &review["dynamic_name_domains"];
    for (domain, prefixes, suffixes) in [
        ("resist", &[""][..], &["Resist", "ResistMax"][..]),
        ("converted_nondefence_resource", &["Extra"][..], &[""][..]),
        ("converted_nondefence_resource", &[""][..], &["Total"][..]),
        (
            "regeneration_resource",
            &[""][..],
            &["Regen", "Recovery"][..],
        ),
        (
            "regeneration_resource",
            &["Condition:CanGain"][..],
            &[""][..],
        ),
    ] {
        for name in rows(&domains[domain])? {
            let name = name.as_str().ok_or("computed name domain")?;
            for prefix in prefixes {
                for suffix in suffixes {
                    let name = format!("{prefix}{name}{suffix}");
                    ensure(
                        !EXCLUDED_DIRECT_NAMES.contains(&name.as_str()),
                        "computed writer creates reviewed flat or factory name",
                    )?;
                }
            }
        }
    }
    let enemy = span(body, "enemy-modifier-transport")?;
    ensure(
        enemy.contains("local enemyDB = actor.enemy.modDB")
            && enemy.contains("actor.modDB:Tabulate(nil, nil, \"EnemyModifier\")")
            && enemy.contains("enemyDB:AddMod(modLib.setSource(mod, source))"),
        "arbitrary EnemyModifier destination",
    )?;
    let relations = span(body, "player-enemy-relations")?;
    let minion = span(body, "minion-create")?;
    ensure(
        relations.contains("env.player.enemy = env.enemy")
            && relations.contains("env.enemy.enemy = env.player")
            && minion.contains("minion.parent = env.player\n\t\t\t\tminion.enemy = env.enemy"),
        "friendly minion enemy relation",
    )?;
    let defence = span(body, "defence-complete")?;
    ensure(
        defence.contains("local resistTypeList = { \"Fire\", \"Cold\", \"Lightning\", \"Chaos\" }")
            && defence.contains(
                "local resources = {\"Mana\", \"Life\", \"Energy Shield\", \"Rage\", \"Ward\"}",
            ),
        "computed writer source domains",
    )?;
    ensure(
        defence.contains("modDB:List(nil, \"GrantReserved\"..pool..\"AsAura\")")
            && defence.contains("local auraMod = copyTable(value.mod)")
            && defence.contains("modDB:NewMod(\"ExtraAura\", \"LIST\", { mod = auraMod })"),
        "late nested aura retained",
    )?;
    let late = span(body, "late-order-and-destinations")?;
    let aura = late
        .find("modDB:List(nil, \"ExtraAura\")")
        .ok_or("current aura consumer")?;
    let player_defence = late
        .find("calcs.defence(env, env.player)")
        .ok_or("Player defence")?;
    let minion_defence = late
        .find("calcs.defence(env, env.minion)")
        .ok_or("minion defence")?;
    let trigger = late
        .find("calcs.triggers(env, env.minion)")
        .ok_or("minion trigger")?;
    let offence = late
        .find("calcs.offence(env, env.minion, env.minion.mainSkill)")
        .ok_or("minion offence")?;
    ensure(
        aura < player_defence
            && player_defence < minion_defence
            && minion_defence < trigger
            && trigger < offence,
        "same-pass source order",
    )?;
    ensure(
        review["generic_edges"][0]["arbitrary_nested_name"] == true
            && review["generic_edges"][0]["selected_minion_destination"] == false
            && review["generic_edges"][1]["arbitrary_nested_name"] == true
            && review["generic_edges"][1]["current_pass_delivery_before_creation"] == true
            && review["generic_edges"][1]["reused_or_later_pass_absence_proved"] == false,
        "generic edges and later-pass limitation",
    )?;
    ensure(
        review["player_offence_minion_writes"] == json!(["CritChance", "CritMultiplier"]),
        "Player offence cross-Actor writers",
    )?;
    let crit = span(body, "player-offence-minion-writes")?;
    ensure(
        crit.contains("env.minion.modDB:NewMod(\"CritChance\"")
            && crit.contains("env.minion.modDB:NewMod(\"CritMultiplier\""),
        "Player offence minion write bodies",
    )?;
    let triggers = span(body, "trigger-complete")?;
    let keys = triggers
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("[\"")?;
            let (name, tail) = rest.split_once("\"]")?;
            tail.trim().starts_with("= function").then_some(name)
        })
        .collect::<Vec<_>>();
    ensure(
        keys.len() == 61
            && json!(keys) == review["trigger_config_keys"]
            && !keys.contains(&"basic attack"),
        "exact trigger dispatch keys",
    )?;
    ensure(triggers.contains("local triggerName = actor.mainSkill.triggeredBy and actor.mainSkill.triggeredBy.grantedEffect.name")
        && triggers.contains("local uniqueName = isTriggered(actor.mainSkill) and getUniqueItemTriggerName(actor.mainSkill)")
        && triggers.contains("actor.mainSkill.skillData.triggered = nil"), "complete trigger dispatch")?;
    let prefix = span(body, "pre-extra-stat-construction")?;
    ensure(
        prefix
            .lines()
            .filter(|l| l.contains("activeSkill.triggeredBy ="))
            .count()
            == 1
            && prefix.contains("if match and skillEffect.grantedEffect.isTrigger then"),
        "derived triggeredBy construction",
    )?;
    let player_loop = span(body, "player-triggered-on-death")?;
    ensure(
        player_loop.starts_with("\tfor _, activeSkill in ipairs(env.player.activeSkillList) do")
            && player_loop.contains(
                "if activeSkill.skillData.triggeredOnDeath and not skillFlags.minion then",
            ),
        "Player-only trigger mutation",
    )?;
    for name in [
        "new_source_execution",
        "native_coverage_authorized",
        "all_temporal_minion_writers_accounted",
        "universal_factory_absence",
        "numeric_order_authority",
    ] {
        ensure(body["scope"][name] == false, "bounded proof scope")?;
    }
    Ok(())
}

pub fn review_bodies(body: &Value) -> Result<(), String> {
    let raw = bytes(&format!("{DIR}/bodies.json"), 2 * 1024 * 1024)?;
    ensure(
        hash(&raw) == BODIES
            && serde_json::from_slice::<Value>(&raw).map_err(|e| e.to_string())? == *body,
        "exact independently pinned complete source review",
    )?;
    let raw_manifest = bytes(
        "crates/poe-optimizer-pob/data/pob-source-manifest.json",
        1024 * 1024,
    )?;
    ensure(
        hash(&raw_manifest) == MANIFEST && body["source_manifest_sha256"] == MANIFEST,
        "source manifest",
    )?;
    let manifest: Value = serde_json::from_slice(&raw_manifest).map_err(|e| e.to_string())?;
    ensure(
        body["source_revision"] == manifest["upstream_revision"],
        "source revision",
    )?;
    let mut files = BTreeMap::new();
    for file in rows(&body["files"])? {
        let compressed = bytes(file["archive"].as_str().ok_or("archive")?, 512 * 1024)?;
        ensure(
            file["archive_bytes"] == compressed.len()
                && file["archive_sha256"] == hash(&compressed),
            "archive identity",
        )?;
        let mut raw = Vec::new();
        GzDecoder::new(compressed.as_slice())
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut raw)
            .map_err(|e| e.to_string())?;
        ensure(raw.len() <= 2 * 1024 * 1024, "expanded source bound")?;
        let text = String::from_utf8(raw)
            .map_err(|e| e.to_string())?
            .replace("\r\n", "\n");
        let path = file["path"].as_str().ok_or("source path")?;
        let original = rows(&manifest["files"])?
            .iter()
            .find(|p| p["path"] == path)
            .ok_or("original source path")?;
        ensure(
            file["bytes"] == text.len()
                && file["sha256"] == hash(text.as_bytes())
                && file["sha256"] == original["sha256"],
            "complete original source",
        )?;
        ensure(
            files.insert(path.to_owned(), text).is_none(),
            "unique source path",
        )?;
    }
    ensure(
        files.len() == 11 && rows(&body["spans"])?.len() == 21,
        "complete file and body census",
    )?;
    for s in rows(&body["spans"])? {
        let text = files
            .get(s["path"].as_str().ok_or("span path")?)
            .ok_or("span file")?;
        let first = s["first"].as_u64().ok_or("first")? as usize;
        let last = s["last"].as_u64().ok_or("last")? as usize;
        ensure(
            first > 0 && last >= first && last <= text.lines().count(),
            "source span bounds",
        )?;
        let actual = text
            .lines()
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        ensure(
            s["body"] == actual && s["sha256"] == hash(actual.as_bytes()),
            "complete body bytes",
        )?;
    }
    // Whole-file check prevents a second direct cross-Actor writer from hiding
    // outside the reviewed CritChance/CritMultiplier block.
    let offence = files.get("src/Modules/CalcOffence.lua").unwrap();
    let references = offence
        .lines()
        .filter(|line| line.contains("env.minion"))
        .collect::<Vec<_>>();
    ensure(
        references.len() == 2
            && references
                .iter()
                .all(|line| line.contains("env.minion.modDB:NewMod(\"Crit")),
        "complete Player offence minion references",
    )?;
    ensure(
        !offence.contains("\"ExtraAura\"")
            && !span(body, "trigger-complete")?.contains("\"ExtraAura\""),
        "no later ExtraAura consumer in called offence or trigger modules",
    )?;
    review_semantics(body)
}

pub fn review_run(run: &Value, recipes: &Value, selected: &Value) -> Result<Value, String> {
    // Independent exact child joins are retained; tables omitted by h.scalars
    // are never interpreted as observed nil. triggeredBy is source-derived.
    let child = minion_store::inherited::construction::review_child_run(run, recipes, selected)?;
    let mut counts = BTreeMap::<String, usize>::new();
    for row in rows(&run["skill_construction"])?
        .iter()
        .filter(|r| r["active_effect"]["id"] == BASIC)
    {
        ensure(
            row["active_effect"]["granted_effect_fields"]["name"] == "Basic Attack",
            "exact Basic dispatch name",
        )?;
        ensure(
            empty(&row["skill_data"]),
            "Basic trigger and skillData inputs",
        )?;
        ensure(
            empty(&row["active_effect"]["source_instance"])
                && empty(&row["socket_group"])
                && empty(&row["supports"]),
            "Basic unique-trigger input sources",
        )?;
        ensure(
            row["actor_is_player"] == false
                && row["actor_parent_is_player"] == true
                && row["actor_profile"] == "RaisedSkeletonSniper",
            "selected friendly Actor relation",
        )?;
        *counts
            .entry(row["mode"].as_str().ok_or("mode")?.into())
            .or_default() += 1;
    }
    ensure(
        counts
            == BTreeMap::from([
                ("MAIN".into(), 32),
                ("CALCS".into(), 2),
                ("CALCULATOR".into(), 2),
            ]),
        "complete Basic construction observations",
    )?;
    Ok(
        json!({"child_lifecycle":child,"basic_rows_by_mode":counts,"basic_trigger_dispatch":"no matching config; source-derived, not an observed dispatch event","triggered_by_nil_observed":false}),
    )
}
fn dependencies() -> Result<(), String> {
    static CHECKED: OnceLock<Result<(), String>> = OnceLock::new();
    CHECKED
        .get_or_init(|| {
            ensure(
                hash(&bytes(
                    &format!("{}/review.json", minion_store::DIR),
                    32 * 1024,
                )?) == MINION_REVIEW,
                "fixed minion-store authority",
            )?;
            minion_store::check()?;
            minion_store::inherited::construction::check_child()
        })
        .clone()
}
pub fn reconstruct(input: &Value, body: &Value) -> Result<Value, String> {
    dependencies()?;
    review_bodies(body)?;
    let a = review_run(&input["off"], &input["recipes"], &input["selected"])?;
    let b = review_run(&input["on"], &input["recipes"], &input["selected"])?;
    ensure(a == b, "independent Basic input census")?;
    ensure(
        input == fixture(),
        "complete independently authenticated source inputs",
    )?;
    Ok(
        json!({"schema_version":1,"kind":"finite-selected-later-helper-destinations","bodies_sha256":BODIES,
        "minion_store_review_sha256":MINION_REVIEW,"inputs_sha256":digest(input),"construction":a,
        "source_deductions":{
            "attributes_charges_misc_literal_store_names_reviewed":true,
            "defence_computed_names_have_explicit_closed_domains":true,
            "direct_names_exclude_reviewed_self_flat_and_factory_names":true,
            "enemy_modifier_payload_names_remain_arbitrary":true,
            "enemy_modifier_destination_is_actor_enemy_not_selected_friendly_minion":true,
            "Player_offence_minion_writes":["CritChance","CritMultiplier"],
            "selected_Basic_trigger_handler_not_dispatched_by_pinned_source":true,
            "defence_estimations_and_breakdowns_write_outputs_not_modifier_payloads":true,
            "late_ExtraAura_creation_preserved":true,
            "late_ExtraAura_same_pass_delivery_precedes_creation":true,
            "late_ExtraAura_reused_or_future_pass_absence_proved":false},
        "remaining":body["review"]["remaining"],"late_aura_limit":body["review"]["late_aura_limit"],"scope":body["scope"]}),
    )
}
pub fn check() -> Result<(), String> {
    ensure(
        reconstruct(fixture(), &read("bodies.json"))? == read("review.json"),
        "retained later-helper proof",
    )
}
pub fn retain() {
    assert_eq!(
        std::env::var("POE_SELF_FLAT_LATER_HELPERS_RETAIN").as_deref(),
        Ok("1")
    );
    let value = reconstruct(fixture(), &read("bodies.json")).unwrap();
    fs::write(
        root().join(DIR).join("review.json"),
        serde_json::to_string_pretty(&value).unwrap() + "\n",
    )
    .unwrap();
}
