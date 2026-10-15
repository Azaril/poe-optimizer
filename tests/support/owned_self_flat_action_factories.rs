//! Exact source-side Basic Attack factory applicability, not runtime coverage.
#[allow(dead_code)]
#[path = "owned_self_flat_minion_store.rs"]
mod minion_store;

use flate2::read::GzDecoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::PathBuf, sync::OnceLock};

pub const DIR: &str = "data/owned/poe2/3887ae68/self-flat-action-factories";
const BODIES: &str = "0dc1667f104bb5d88409e00142642fd5855ab3f6a624540757b632991e85c6a8";
const MANIFEST: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const REPORT: &str = "7a723a4eece7b297737006b08d1dc61ec0047d13cfd2b9fbfeaee94eb52397cb";
const BASIC: &str = "MinionMeleeBow";
const SUMMON: &str = "SummonSkeletalSnipersPlayer";
const PROFILE: &str = "RaisedSkeletonSniper";
pub const FLAGS: u64 = 42_949_813_253;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn digest(v: &Value) -> String {
    hash(&serde_json::to_vec(v).unwrap())
}
fn ensure(ok: bool, why: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(why.into()) }
}
fn bytes(path: &str, limit: u64) -> Result<Vec<u8>, String> {
    let p = root().join(path);
    ensure(
        fs::metadata(&p).map_err(|e| e.to_string())?.len() <= limit,
        "bounded retained file",
    )?;
    fs::read(p).map_err(|e| e.to_string())
}
fn rows(v: &Value) -> Result<&[Value], String> {
    if v.as_object().is_some_and(serde_json::Map::is_empty) {
        return Ok(&[]);
    }
    v.as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| "complete captured sequence".into())
}
fn empty(v: &Value) -> bool {
    rows(v).is_ok_and(<[_]>::is_empty)
}
fn one<'a>(v: &'a Value, key: &str, wanted: &str) -> Result<&'a Value, String> {
    let matches = rows(v)?
        .iter()
        .filter(|v| v[key] == wanted)
        .collect::<Vec<_>>();
    ensure(matches.len() == 1, "exact unique source identity")?;
    Ok(matches[0])
}
pub fn read(name: &str) -> Value {
    serde_json::from_slice(&bytes(&format!("{DIR}/{name}"), 128 * 1024).unwrap()).unwrap()
}
pub fn fixture() -> &'static Value {
    minion_store::fixture()
}
pub fn report() -> &'static Value {
    static R: OnceLock<Value> = OnceLock::new();
    R.get_or_init(|| {
        let b = bytes(
            "data/owned/poe2/3887ae68/minion-preconversion-source/report.json",
            5 * 1024 * 1024,
        )
        .unwrap();
        assert_eq!(b.len(), 4_041_068);
        assert_eq!(hash(&b), REPORT);
        serde_json::from_slice(&b).unwrap()
    })
}

pub fn review_bodies(v: &Value) -> Result<(), String> {
    let original = bytes(&format!("{DIR}/bodies.json"), 128 * 1024)?;
    ensure(
        hash(&original) == BODIES
            && serde_json::from_slice::<Value>(&original).map_err(|e| e.to_string())? == *v,
        "exact complete factory/filter body authority",
    )?;
    let raw = bytes(
        "crates/poe-optimizer-pob/data/pob-source-manifest.json",
        1024 * 1024,
    )?;
    ensure(
        hash(&raw) == MANIFEST && v["source_manifest_sha256"] == MANIFEST,
        "source manifest identity",
    )?;
    let manifest: Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    ensure(
        v["source_revision"] == manifest["upstream_revision"],
        "source revision",
    )?;
    let mut files = BTreeMap::new();
    for file in rows(&v["files"])? {
        let compressed = bytes(
            file["archive"].as_str().ok_or("source archive")?,
            512 * 1024,
        )?;
        ensure(
            file["archive_bytes"] == compressed.len()
                && file["archive_sha256"] == hash(&compressed),
            "compressed source identity",
        )?;
        let mut out = Vec::new();
        GzDecoder::new(compressed.as_slice())
            .take(1024 * 1024 + 1)
            .read_to_end(&mut out)
            .map_err(|e| e.to_string())?;
        ensure(out.len() <= 1024 * 1024, "bounded expanded source")?;
        let text = String::from_utf8(out)
            .map_err(|e| e.to_string())?
            .replace("\r\n", "\n");
        let pin = rows(&manifest["files"])?
            .iter()
            .find(|p| p["path"] == file["path"])
            .ok_or("manifest file")?;
        ensure(
            file["bytes"] == text.len()
                && file["sha256"] == hash(text.as_bytes())
                && file["sha256"] == pin["sha256"],
            "complete upstream source",
        )?;
        ensure(
            files
                .insert(file["path"].as_str().ok_or("source path")?.to_owned(), text)
                .is_none(),
            "duplicate source archive",
        )?;
    }
    ensure(
        files.len() == 7 && rows(&v["spans"])?.len() == 19,
        "complete source/filter span census",
    )?;
    for span in rows(&v["spans"])? {
        let text = files
            .get(span["path"].as_str().ok_or("span path")?)
            .ok_or("source for span")?;
        let first = span["first"].as_u64().ok_or("first line")? as usize;
        let last = span["last"].as_u64().ok_or("last line")? as usize;
        ensure(
            first > 0 && last >= first && last <= text.lines().count(),
            "valid complete source range",
        )?;
        let body = text
            .lines()
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        ensure(
            span["body"] == body && span["sha256"] == hash(body.as_bytes()),
            "complete source span",
        )?;
    }
    let mut count = 0;
    for family in rows(&v["spell_factories"])? {
        let span = one(
            &v["spans"],
            "role",
            family["role"].as_str().ok_or("factory role")?,
        )?;
        let lines = span["body"]
            .as_str()
            .ok_or("factory body")?
            .lines()
            .filter(|l| l.contains("skillModList:NewMod("))
            .collect::<Vec<_>>();
        ensure(
            family["required_flag"] == "Spell"
                && family["writes"] == lines.len()
                && lines
                    .iter()
                    .all(|l| l.trim_end().ends_with("ModFlag.Spell)")),
            "all reviewed factory writes require Spell",
        )?;
        count += lines.len();
    }
    ensure(count == 20, "twenty spell-only factory writes")?;
    let global = &files["src/Data/Global.lua"];
    ensure(
        global.contains("ModFlag.Attack =\t 0x0000000000000001")
            && global.contains("ModFlag.Spell =\t\t 0x0000000000000002"),
        "exact source Attack/Spell bits",
    )?;
    for path in ["src/Classes/ModList.lua", "src/Classes/ModDB.lua"] {
        ensure(
            files[path].contains("local band = AND64"),
            "original wide flag operation",
        )?;
    }
    let basic = one(&v["spans"], "role", "complete-basic-definition")?["body"]
        .as_str()
        .ok_or("whole Basic definition")?;
    ensure(
        basic.starts_with("skills[\"MinionMeleeBow\"] = {")
            && !basic.contains("function")
            && !basic.contains("Func")
            && !basic.contains("setmetatable"),
        "plain Basic definition has no callback",
    )?;
    Ok(())
}

/// Only the necessary flag predicate for the reviewed zero/Attack/Spell masks.
/// The original full source filters are authenticated separately. This does
/// not simulate tags, keyword flags, numerical values, or general AND64.
pub fn flag_predicate(cfg_flags: Option<u64>, required: u64) -> Result<bool, String> {
    let flags = cfg_flags.unwrap_or(0);
    ensure(
        flags < (1_u64 << 53) && required <= 3,
        "reviewed exact flag domain",
    )?;
    Ok(flags & required == required)
}
pub fn review_recipe(recipe: &Value) -> Result<(), String> {
    ensure(
        recipe["id"] == BASIC && empty(&recipe["callbacks"]),
        "exact Basic callback inventory",
    )?;
    ensure(
        recipe["metadata"]["name"] == "Basic Attack"
            && recipe["metadata"]["skillTypes"]["[1]"] == true
            && recipe["metadata"]["skillTypes"].get("[2]").is_none(),
        "Basic Attack recipe types",
    )?;
    for data in std::iter::once(&recipe["base_mods"]).chain(
        rows(&recipe["stat_set_base_mods"])?
            .iter()
            .map(|s| &s["records"]),
    ) {
        ensure(
            data["value"].is_null() && empty(&data["aliases"]) && empty(&data["callbacks"]),
            "empty Basic base and callback bodies",
        )?;
    }
    ensure(
        rows(&recipe["stat_set_base_mods"])?.len() == 1
            && recipe["stat_set_base_mods"][0]["set"] == 1,
        "one actual Basic stat set",
    )?;
    let resolved = rows(&recipe["resolved"])?;
    ensure(resolved.len() == 7, "complete seven-key Basic recipe")?;
    let mut arrow = 0;
    for mapping in resolved {
        ensure(
            mapping["set"] == 1
                && empty(&mapping["mapping"]["aliases"])
                && empty(&mapping["mapping"]["callbacks"]),
            "Basic mapping callback/alias inventory",
        )?;
        if mapping["stat"] == "skill_can_fire_arrows" {
            ensure(
                mapping["mapping"]["value"] == json!({"skillFlag":"arrow"}),
                "only reviewed arrow mapping",
            )?;
            arrow += 1;
        } else {
            ensure(
                mapping["mapping"]["value"].is_null(),
                "no other Basic mapped factory",
            )?;
        }
    }
    ensure(arrow == 1, "one arrow mapping")
}

pub fn review_selection(sources: &Value, corroboration: &Value) -> Result<Value, String> {
    let recipe = one(&sources["recipes"]["skills"], "id", BASIC)?;
    review_recipe(recipe)?;
    let observed = &sources["recipes"]["observed"];
    ensure(
        observed["report"]["sha256"] == REPORT && observed["report"]["bytes"] == 4_041_068,
        "actual pass report binding",
    )?;
    let case = &corroboration["cases"][0];
    let state = &case["state"];
    ensure(
        case["xml_sha256"] == observed["xml_sha256"] && state["selected"] == observed["selected"],
        "same saved source selection",
    )?;
    for key in [
        "original_functions_preserved",
        "loaded_state_preserved",
        "saved_specs_preserved",
        "query_state_preserved",
        "cached_outputs_preserved",
    ] {
        ensure(
            state[key] == true,
            "original source call and state preservation",
        )?;
    }
    let actor = one(&state["main"]["actors"], "summon_effect_id", SUMMON)?;
    ensure(
        actor["actor_profile"] == PROFILE
            && actor["is_environment_minion"] == true
            && actor["fresh_actor"] == true
            && actor["hostile"] == false
            && actor["source_occurrence"] == observed["minion_source_occurrence"],
        "exact selected source Actor",
    )?;
    let child = one(&actor["children"], "effect_id", BASIC)?;
    ensure(
        child["selected"] == true
            && child["summoner_owns_actor"] == true
            && child["summoner_source"] == actor["source_occurrence"],
        "exact generated Action source",
    )?;
    let passes = rows(&child["passes"])?;
    ensure(
        passes.len() == 1
            && passes[0]["label"] == "Main Hand"
            && passes[0]["inputs"]["query_state_preserved"] == true,
        "one actual Main Hand pass",
    )?;
    let base = one(&child["base_calls"], "damage_type", "Physical")?;
    let pre = &base["preconversion"];
    ensure(
        base["observed_at"] == 4137
            && base["query_state_preserved"] == true
            && pre["pass_label"] == "Main Hand"
            && pre["actor_profile"] == PROFILE,
        "actual base call location",
    )?;
    for key in [
        "exact_actor_store",
        "exact_enemy_store",
        "exact_parent",
        "exact_pass_cfg",
        "exact_pass_source",
        "exact_summoner",
    ] {
        ensure(pre[key] == true, "original physical pass identity")?;
    }
    ensure(
        pre["selection"]["line"] == 2543
            && pre["selection"]["kind"] == "weapon1"
            && pre["selection"]["is_attack"] == true
            && pre["selection"]["weapon1_attack"] == true
            && pre["selection"]["weapon2_attack"] == false,
        "actual weapon1 Attack selection",
    )?;
    ensure(
        pre["source_occurrence"] == actor["source_occurrence"]
            && pre["cfg"] == observed["cfg"]
            && passes[0]["inputs"]["cfg"] == pre["cfg"]["fields"],
        "same actual cfg and supplying occurrence",
    )?;
    let cfg = &pre["cfg"];
    ensure(
        cfg["present"] == true
            && cfg["granted_effect"]["id"] == BASIC
            && cfg["granted_effect_is_source"] == true
            && cfg["fields"]["flags"] == FLAGS
            && cfg["fields"]["skillName"] == "Basic Attack"
            && cfg["skill_conditions"]["MainHandAttack"] == true,
        "exact Main Hand flags",
    )?;
    ensure(
        flag_predicate(Some(FLAGS), 1)? && !flag_predicate(Some(FLAGS), 2)?,
        "Attack present and Spell excluded",
    )?;
    // Join the separately observed construction cfg to the actual pass without
    // treating early empty skillStats/conditions as the final cfg object.
    for gate in rows(&sources["gates"])? {
        minion_store::review_gates(gate, sources)?;
        for row in rows(&gate["hollow_palm"])? {
            ensure(
                row["cfg"]["fields"] == cfg["fields"]
                    && row["cfg"]["granted_effect"] == cfg["granted_effect"]
                    && row["source"] == actor["source_occurrence"],
                "construction flags/source join actual pass",
            )?;
        }
        for call in rows(&gate["flag_calls"])? {
            ensure(
                call["cfg"] == json!({"present":false}),
                "parent factory uses original nil cfg Flag",
            )?;
        }
    }
    Ok(
        json!({"actual_main_hand_flags":FLAGS,"attack_bit":1,"spell_bit":2,"spell_factory_write_sites":20,
        "spell_writes_inapplicable":true,"spell_writes_absent_not_asserted":true,
        "factory_runtime_invocation_counts_not_inferred":true,
        "selected_recipe_callbacks":0,"selected_recipe_mapping_keys":7,
        "parent_weapon_provenance":"separate original Flag calls and exact source gate; not Spell filtering",
        "supplemental_channel_values_not_used_for_absence":true}),
    )
}

pub fn reconstruct(
    sources: &Value,
    corroboration: &Value,
    bodies: &Value,
) -> Result<Value, String> {
    static AUTH: OnceLock<Result<(), String>> = OnceLock::new();
    AUTH.get_or_init(minion_store::check).clone()?;
    review_bodies(bodies)?;
    let selection = review_selection(sources, corroboration)?;
    ensure(
        sources == fixture() && corroboration == report(),
        "complete independently authenticated inputs",
    )?;
    Ok(
        json!({"schema_version":1,"kind":"exact-basic-attack-factory-applicability","bodies_sha256":BODIES,
        "inputs_sha256":digest(sources),"actual_pass_report_sha256":REPORT,"selection":selection,
        "scope":{"reviewed_action_factories_closed_for_exact_basic_attack":true,
            "all_spell_suppliers_absent":false,"all_minion_store_writers_closed":false,
            "native_membership_complete":false,"native_coverage_authorized":false,"new_source_execution":false,
            "source_tag_evaluator_reimplemented":false,"general_numeric_order_authority":false},
        "remaining":["Complete minion-store source/delivery inventory (flasks, Keystone and later helper routes) remains separately gated",
            "Native complete selected flat membership and current-ID final preconversion consumer require publication after all source boundaries close"]}),
    )
}
pub fn check() -> Result<(), String> {
    ensure(
        reconstruct(fixture(), report(), &read("bodies.json"))? == read("review.json"),
        "retained Action factory review",
    )
}
pub fn retain() {
    assert_eq!(
        std::env::var("POE_SELF_FLAT_ACTION_FACTORIES_RETAIN").as_deref(),
        Ok("1")
    );
    let r = reconstruct(fixture(), report(), &read("bodies.json")).unwrap();
    fs::write(
        root().join(DIR).join("review.json"),
        serde_json::to_string_pretty(&r).unwrap() + "\n",
    )
    .unwrap();
}
