//! Finite source-side minion initializer and receiving-supplier reconciliation.
//! This is an offline proof; it adds no runtime exclusions or native membership.
#[allow(dead_code)]
#[path = "owned_inherited_minion_delivery.rs"]
pub(crate) mod inherited;

use flate2::read::GzDecoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::PathBuf, sync::OnceLock};

pub const DIR: &str = "data/owned/poe2/3887ae68/self-flat-minion-store";
const BODIES: &str = "4135bdc54032242addb41942ec79d93b8557cb9b71e5d223db7c90f769badab5";
const MANIFEST: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const SUMMON: &str = "SummonSkeletalSnipersPlayer";
const PROFILE: &str = "RaisedSkeletonSniper";
const BASIC: &str = "MinionMeleeBow";
const FORBIDDEN: [&str; 9] = [
    "PhysicalMin",
    "PhysicalMax",
    "GainMainHandDmgFromParent",
    "UseHollowPalmDamage",
    "UseFacebreakerItemDamage",
    "Condition:HollowPalm",
    "ExtraSkillMod",
    "AffectedByAuraMod",
    "Keystone",
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
fn ensure(ok: bool, message: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(message.into()) }
}
fn bytes(path: &str, limit: u64) -> Result<Vec<u8>, String> {
    let p = root().join(path);
    ensure(
        fs::metadata(&p).map_err(|e| e.to_string())?.len() <= limit,
        "bounded retained source",
    )?;
    fs::read(p).map_err(|e| e.to_string())
}
fn rows(value: &Value) -> Result<&[Value], String> {
    if value.as_object().is_some_and(serde_json::Map::is_empty) {
        return Ok(&[]);
    }
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| "complete captured sequence required".into())
}
fn empty(value: &Value) -> bool {
    rows(value).is_ok_and(<[_]>::is_empty)
}
pub fn read(name: &str) -> Value {
    serde_json::from_slice(&bytes(&format!("{DIR}/{name}"), 1024 * 1024).unwrap()).unwrap()
}
pub fn fixture() -> &'static Value {
    inherited::sources()
}

pub fn review_bodies(body: &Value) -> Result<(), String> {
    let retained = bytes(&format!("{DIR}/bodies.json"), 128 * 1024)?;
    ensure(
        hash(&retained) == BODIES
            && serde_json::from_slice::<Value>(&retained).map_err(|e| e.to_string())? == *body,
        "exact reviewed complete body authority",
    )?;
    let raw = bytes(
        "crates/poe-optimizer-pob/data/pob-source-manifest.json",
        1024 * 1024,
    )?;
    ensure(
        hash(&raw) == MANIFEST && body["source_manifest_sha256"] == MANIFEST,
        "original manifest identity",
    )?;
    let manifest: Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    ensure(
        body["source_revision"] == manifest["upstream_revision"],
        "original revision",
    )?;
    let mut files = BTreeMap::new();
    for file in rows(&body["files"])? {
        let compressed = bytes(file["archive"].as_str().ok_or("archive path")?, 512 * 1024)?;
        ensure(
            file["archive_bytes"] == compressed.len()
                && file["archive_sha256"] == hash(&compressed),
            "compressed source identity",
        )?;
        let mut raw = Vec::new();
        GzDecoder::new(compressed.as_slice())
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut raw)
            .map_err(|e| e.to_string())?;
        ensure(raw.len() <= 2 * 1024 * 1024, "bounded expanded source")?;
        // Original source manifest explicitly authenticates CRLF -> LF normalization.
        let text = String::from_utf8(raw)
            .map_err(|e| e.to_string())?
            .replace("\r\n", "\n");
        let pin = rows(&manifest["files"])?
            .iter()
            .find(|p| p["path"] == file["path"])
            .ok_or("manifest source path")?;
        ensure(
            file["bytes"] == text.len()
                && file["sha256"] == hash(text.as_bytes())
                && file["sha256"] == pin["sha256"],
            "complete original file",
        )?;
        ensure(
            files
                .insert(file["path"].as_str().ok_or("source path")?.to_owned(), text)
                .is_none(),
            "duplicate source file",
        )?;
    }
    ensure(
        files.len() == 5 && rows(&body["spans"])?.len() == 14,
        "complete source census",
    )?;
    for span in rows(&body["spans"])? {
        let text = files
            .get(span["path"].as_str().ok_or("span source path")?)
            .ok_or("span file")?;
        let first = span["first"].as_u64().ok_or("first source line")? as usize;
        let last = span["last"].as_u64().ok_or("last source line")? as usize;
        ensure(
            first > 0 && last >= first && last <= text.lines().count(),
            "bounded source span",
        )?;
        let actual = text
            .lines()
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        ensure(
            span["body"] == actual && span["sha256"] == hash(actual.as_bytes()),
            "whole source span",
        )?;
    }
    for (at, prefix, count) in [(0, "modDB:NewMod(", 84), (1, "minion.modDB:NewMod(", 24)] {
        let text = body["spans"][at]["body"]
            .as_str()
            .ok_or("initializer source")?;
        let mut names = Vec::<String>::new();
        for line in text.lines() {
            if let Some(rest) = line.trim().strip_prefix(prefix) {
                let arg = rest.split_once(',').ok_or("writer argument")?.0;
                names.push(
                    serde_json::from_str(arg)
                        .map_err(|_| "unreviewed computed initializer writer")?,
                );
            }
        }
        ensure(
            names.len() == count
                && json!(names) == body["initializer_writers"][at]["literal_writes"]
                && body["initializer_writers"][at]["dynamic_writes"] == json!([]),
            "complete initializer writer inventory",
        )?;
        ensure(
            names.iter().all(|n| !FORBIDDEN.contains(&n.as_str())),
            "initializer creates selected flat or gate supplier",
        )?;
    }
    let summon = body["spans"][5]["body"].as_str().unwrap();
    ensure(
        summon.starts_with("skills[\"SummonSkeletalSnipersPlayer\"] = {")
            && !summon.contains("minionUses")
            && !summon.contains("minionHasItemSet"),
        "complete summon equipment declaration",
    )?;
    Ok(())
}

/// Inspect every nested named record, including inactive/zero payloads. This
/// is a finite captured-body inventory; it does not evaluate source tags.
pub fn review_records(value: &Value) -> Result<(), String> {
    match value {
        Value::Object(map) => {
            if map.contains_key("type")
                && let Some(name) = map.get("name").and_then(Value::as_str)
            {
                ensure(
                    !FORBIDDEN.contains(&name),
                    "unreviewed flat/gate/generic source record",
                )?;
            }
            for child in map.values() {
                review_records(child)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                review_records(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn review_run(run: &Value, recipes: &Value) -> Result<Value, String> {
    let profile = rows(&recipes["minion_profiles"])?
        .iter()
        .filter(|p| p["id"] == PROFILE)
        .collect::<Vec<_>>();
    ensure(
        profile.len() == 1
            && empty(&profile[0]["mod_list"]["value"])
            && empty(&profile[0]["mod_list"]["callbacks"])
            && empty(&profile[0]["callbacks"]),
        "exact empty Sniper profile modifiers",
    )?;
    review_records(recipes)?;
    let environments = rows(&run["environments"])?;
    ensure(environments.len() == 21, "complete environment inputs")?;
    for env in environments {
        let nodes = rows(&env["nodes"])?;
        ensure(
            nodes.len() == 57 && nodes.iter().all(|n| n["key"] != 39935),
            "selected Talisman passive guard",
        )?;
        ensure(
            rows(&env["items"])?.len() == 9 && empty(&env["party_records"]),
            "complete selected equipment and Party input",
        )?;
        review_records(env)?;
    }
    ensure(
        rows(&run["baseline"])?.len() == 18 && rows(&run["configuration"])?.len() == 2,
        "complete baseline/configuration inputs",
    )?;
    review_records(&run["baseline"])?;
    review_records(&run["configuration"])?;
    let constructions = rows(&run["skill_construction"])?;
    ensure(constructions.len() == 1536, "complete constructor pairs")?;
    let mut summoners = 0;
    let mut curse_buffs = 0;
    let mut buff_names = BTreeMap::<String, usize>::new();
    for row in constructions {
        review_records(&row["records"])?;
        if row["active_effect"]["id"] == SUMMON {
            ensure(
                row["actor_is_player"] == true
                    && row["source"]["source_present"] == true
                    && rows(&row["source"]["matches"])?.len() == 1,
                "exact selected summoner source",
            )?;
            ensure(
                row["skill_data"]
                    .as_object()
                    .is_some_and(serde_json::Map::is_empty),
                "selected summoner has no equipment-copy skill data",
            )?;
            let fields = row["active_effect"]["granted_effect_fields"]
                .as_object()
                .ok_or("captured granted effect fields")?;
            ensure(
                !fields.contains_key("minionUses") && !fields.contains_key("minionHasItemSet"),
                "unexpected summon equipment metadata",
            )?;
            summoners += 1;
        }
        for buff in rows(&row["buffs"])? {
            review_records(&buff["records"])?;
            let kind = buff["fields"]["type"]
                .as_str()
                .ok_or("complete buff kind")?;
            if matches!(kind, "Curse" | "CurseBuff") {
                curse_buffs += 1;
            }
            ensure(
                matches!(kind, "Buff" | "Debuff"),
                "unreviewed curse/aura/link/warcry supplier",
            )?;
            *buff_names
                .entry(
                    buff["fields"]["name"]
                        .as_str()
                        .ok_or("buff name")?
                        .to_owned(),
                )
                .or_default() += 1;
        }
    }
    ensure(
        summoners == 42 && curse_buffs == 0,
        "complete selected summoner and curse census",
    )?;
    ensure(
        buff_names.keys().map(String::as_str).collect::<Vec<_>>()
            == ["Frost Bomb", "Pain Offering", "Rage"],
        "complete constructed buff names",
    )?;
    Ok(
        json!({"environment_inputs":21,"selected_nodes_per_environment":57,"selected_item_uses":9,
        "summoner_construction_rows":summoners,"profile_modifiers":0,"extra_skill_mod_candidates":0,
        "constructed_curse_buffs":curse_buffs,"constructed_buff_names":buff_names.keys().collect::<Vec<_>>(),
        "affected_by_aura_mod_candidates_in_captured_inputs":0,"captured_keystone_candidates":0,
        "literal_initializer_writes":[84,24],"initializer_formula_values_not_evaluated":true}),
    )
}

fn exact_sniper(row: &Value, sources: &Value) -> Result<(), String> {
    ensure(
        row["actor_profile"] == PROFILE
            && row["effect_id"] == BASIC
            && row["actor_is_player"] == false
            && row["actor_parent_is_player"] == true
            && row["summoner_owns_actor"] == true,
        "exact receiving Sniper Actor",
    )?;
    let matches = rows(&row["source"]["matches"])?;
    ensure(
        row["source"]["source_present"] == true && matches.len() == 1,
        "exact summoner source occurrence",
    )?;
    ensure(
        rows(&sources["off"]["skill_construction"])?
            .iter()
            .any(|c| {
                c["actor_profile"] == PROFILE
                    && c["active_effect"]["id"] == BASIC
                    && c["mode"] == row["mode"]
                    && c["summoner"] == row["source"]
            }),
        "receiving gate joins actual child constructor",
    )
}
pub fn review_gates(gate: &Value, sources: &Value) -> Result<Value, String> {
    ensure(
        gate["original_functions_preserved"] == true
            && gate["business_methods_replaced"] == false
            && gate["query_probes_are_supplemental"] == true,
        "original runtime gate scope",
    )?;
    let parent = rows(&gate["parent_weapon"])?;
    ensure(
        parent.len() == 4 && rows(&gate["flag_calls"])?.len() == 2,
        "original parent-weapon consumer census",
    )?;
    for row in parent {
        exact_sniper(row, sources)?;
        let flag = &row["flag"];
        ensure(
            row["line"] == 820
                && row["exact_actor_store"] == true
                && row["original_branch_entered"] == false
                && row["parent_present"] == true
                && row["parent_main_hand_present"] == true,
            "actual parent-weapon gate",
        )?;
        ensure(
            flag["name"] == "GainMainHandDmgFromParent"
                && empty(&flag["raw"])
                && empty(&flag["eligible"])
                && flag["supplemental_original_flag"] == json!({"present":false})
                && flag["query_state_preserved"] == true,
            "parent flag absent rather than inferred false",
        )?;
    }
    for call in rows(&gate["flag_calls"])? {
        exact_sniper(call, sources)?;
        let index = call["gate_index"]
            .as_u64()
            .ok_or("original flag gate index")? as usize;
        ensure(
            index > 0 && index <= parent.len(),
            "original flag gate identity",
        )?;
        let row = &parent[index - 1];
        ensure(
            call["original_flag_call_observed"] == true
                && call["line"] == 820
                && call["name_from_pinned_callsite"] == "GainMainHandDmgFromParent"
                && call["source"] == row["source"]
                && call["mode"] == row["mode"],
            "original flag invocation source join",
        )?;
    }
    let hollow = rows(&gate["hollow_palm"])?;
    ensure(
        hollow.len() == 18,
        "complete Hollow Palm preparation visits",
    )?;
    for row in hollow {
        exact_sniper(row, sources)?;
        ensure(
            row["line"] == 887
                && row["actor_condition"] == json!({"present":false})
                && row["original_branch_entered"] == false,
            "original Hollow Palm short circuit",
        )?;
    }
    let aura = rows(&gate["aura_queries"])?;
    ensure(aura.len() == 18, "original ExtraAura consumer census")?;
    for row in aura {
        ensure(
            row["line"] == 3169
                && row["name"] == "ExtraAura"
                && row["store_is_player"] == true
                && row["exact_store"] == true
                && row["original_consumer_executed"] == true
                && row["original_list_return_observed"] == true
                && empty(&row["raw"])
                && empty(&row["returned"]),
            "original ExtraAura list absence",
        )?;
    }
    let mut sniper_buffs = 0;
    for row in rows(&gate["group_deliveries"])? {
        review_records(&row["modifiers"])?;
        if row["recipient"] == "selected_minion" {
            let recipient = rows(&row["selected_recipient"]["matches"])?;
            ensure(recipient.len() == 1, "one exact direct-buff recipient")?;
            if recipient[0]["effect_id"] == SUMMON {
                ensure(
                    row["line"] == 3128
                        && row["exact_recipient_store"] == true
                        && row["buff_key"] == "Pain Offering",
                    "actual direct Sniper buff",
                )?;
                let mods = rows(&row["modifiers"])?;
                ensure(
                    mods.len() == 2
                        && mods.iter().all(|m| {
                            m["type"] == "INC"
                                && matches!(m["name"].as_str(), Some("Speed" | "Damage"))
                        }),
                    "complete direct Sniper payload",
                )?;
                ensure(
                    mods.iter().filter(|m| m["name"] == "Speed").count() == 1
                        && mods.iter().filter(|m| m["name"] == "Damage").count() == 1,
                    "direct payload preserves both distinct names",
                )?;
                ensure(
                    rows(&sources["off"]["skill_construction"])?
                        .iter()
                        .any(|c| {
                            c["active_effect"]["id"] == SUMMON
                                && c["source"] == recipient[0]["source"]
                                && c["mode"] == row["mode"]
                        }),
                    "direct buff exact summoner join",
                )?;
                sniper_buffs += 1;
            }
        }
    }
    ensure(
        sniper_buffs == 2,
        "complete direct Sniper buff observations",
    )?;
    Ok(
        json!({"parent_weapon_line_visits":4,"original_parent_flag_calls":2,"hollow_palm_visits":18,
        "original_extra_aura_lists":18,"selected_sniper_direct_buffs":sniper_buffs,
        "gate_probe_values_not_relabelled_as_original_consumers":true}),
    )
}

fn authenticate_dependencies() -> Result<(), String> {
    static CHECKED: OnceLock<Result<(), String>> = OnceLock::new();
    CHECKED.get_or_init(inherited::check).clone()
}
pub fn reconstruct(sources: &Value, bodies: &Value) -> Result<Value, String> {
    authenticate_dependencies()?;
    review_bodies(bodies)?;
    let a = review_run(&sources["off"], &sources["recipes"])?;
    let b = review_run(&sources["on"], &sources["recipes"])?;
    ensure(a == b, "independent input supplier census")?;
    let gates = rows(&sources["gates"])?;
    ensure(gates.len() == 2, "both retained gate traces")?;
    let g = review_gates(&gates[0], sources)?;
    ensure(
        g == review_gates(&gates[1], sources)?,
        "independent receiving gate census",
    )?;
    ensure(
        sources == fixture(),
        "complete authenticated construction and receiving inputs",
    )?;
    Ok(
        json!({"schema_version":1,"kind":"finite-selected-minion-store-suppliers","bodies_sha256":BODIES,
        "inputs_sha256":digest(sources),"inherited_review_sha256":hash(&bytes(&format!("{}/review.json",inherited::DIR),32*1024)?),
        "initialization":a,"receiving_gates":g,
        "source_deductions":{"shared_and_minion_literal_writers_exclude_self_flat_and_gate_names":true,
            "sniper_profile_modifier_list_empty":true,"selected_summoner_has_no_item_set_or_uses_declaration":true,
            "selected_summoner_bow_copy_input_absent":true,"selected_talisman_source_absent":true,
            "iron_mass_other_profile_branch_inapplicable":true,"captured_extra_skill_mod_source_inventory_empty":true,
            "constructed_curse_buff_branch_inapplicable":true},
        "remaining":[
            {"sites":"CalcPerform.lua:1584-1759 (AddList at 1751/1759)",
             "boundary":"Flask-to-minion payloads",
             "missing":"Exact active env.flasks and computed non-Player flask payload inventory at these recipients; nine captured ordinary item uses do not alone prove it empty",
             "next":"Reconcile existing selected equipment/XML flask enablement and complete calcFlaskMods/dispatch bodies; observe only if active payload authority is still missing"},
            {"sites":"CalcPerform.lua:1855-1857,3378-3380",
             "boundary":"Keystone expansion after inherited modifiers",
             "missing":"Complete call-time Keystone candidates, including possible earlier flask delivery; the captured construction/inherited records contain no Keystone",
             "next":"After flask closure, join the exact inherited return/append proof and complete Keystone source map; keep the secondary life-pool recipient distinct"},
            {"sites":"CalcPerform.lua:3204-3223 (writes at 3213/3221)",
             "boundary":"Party extraAura and AffectedByAuraMod delivery",
             "missing":"Exact allyBuffs.extraAura and later Player List/recipient authority; captured Party inputs and AffectedByAuraMod candidates are empty but this latter call was not observed",
             "next":"Reuse accepted empty Party accounting and complete allyBuffs construction; trace later Player-store writers before deciding whether an original List/recipient observation is necessary"},
            {"sites":"CalcPerform.lua:1861,3243-3244,3495-3500",
             "boundary":"Actor attribute/charge/misc helpers and defence/trigger call edges before offence",
             "missing":"A complete destination-name/Actor-relation proof for these helper bodies, including applyEnemyModifiers at 565-579; fixed-name searches alone are not that proof",
             "next":"Reuse pinned complete bodies and exact selected Actor relations to exclude PhysicalMin/PhysicalMax and generic transports; no new scalar observation is indicated"},
            {"sites":"CalcOffence.lua:793-834,1234-1271,2401-2407",
             "boundary":"Action-local added-damage factories and preDamageFunc",
             "missing":"Same-selection joins for spell-only applicability, parent-weapon gate, skillData weapon factories and the exact selected callback identity",
             "next":"Reuse source-backed Basic Attack flags/recipe, empty callback inventory and original parent Flag evidence; authenticate full factory bodies and filter semantics"},
            {"sites":"native checked contribution membership and final physical endpoint",
             "boundary":"Publication after source closure",
             "missing":"Current native writer census/selection admission and non-colliding final endpoint identities",
             "next":"Publish checked selected flat membership and direct-query final physical consumer together only after all source paths are accounted; do not reuse colliding historical draft IDs"}],
        "scope":{"initializer_and_selected_receiving_inputs_reviewed":true,"all_temporal_minion_writers_accounted":false,
            "native_membership_complete":false,"native_coverage_authorized":false,"new_source_execution":false,
            "arbitrary_affected_by_aura_mod_filtering_proved":false,"numeric_order_authority":false}}),
    )
}
pub fn check() -> Result<(), String> {
    ensure(
        reconstruct(fixture(), &read("bodies.json"))? == read("review.json"),
        "retained minion-store proof differs",
    )
}
pub fn retain() {
    assert_eq!(
        std::env::var("POE_SELF_FLAT_MINION_STORE_RETAIN").as_deref(),
        Ok("1")
    );
    let reviewed = reconstruct(fixture(), &read("bodies.json")).unwrap();
    fs::write(
        root().join(DIR).join("review.json"),
        serde_json::to_string_pretty(&reviewed).unwrap() + "\n",
    )
    .unwrap();
}
