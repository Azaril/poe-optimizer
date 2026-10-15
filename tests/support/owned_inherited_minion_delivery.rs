//! Original inherited List and nested AddMod correspondence; no native coverage.
#[allow(dead_code)]
#[path = "owned_self_flat_skill_construction.rs"]
mod construction;

use flate2::read::GzDecoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::PathBuf,
    sync::OnceLock,
};

pub const DIR: &str = "data/owned/poe2/3887ae68/self-flat-inherited-minion-delivery";
const MANIFEST: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const AUTHORITY: &str = "03a2c140246a44644fb9bfa7552c543259d41b5bed8b04201d29031b2f4bcf6c";
const BODIES: &str = "be50b1e712c3a3f87c9b8ec91460487f985a480a4baafb0679704386dbe9dee9";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn bytes(path: &str, limit: u64) -> Result<Vec<u8>, String> {
    let path = root().join(path);
    ensure(
        fs::metadata(&path).map_err(|e| e.to_string())?.len() <= limit,
        "bounded retained file",
    )?;
    fs::read(path).map_err(|e| e.to_string())
}
pub fn read(name: &str) -> Value {
    serde_json::from_slice(&bytes(&format!("{DIR}/{name}"), 2 * 1024 * 1024).unwrap()).unwrap()
}
fn unpack(path: &str, size: u64) -> Result<Vec<u8>, String> {
    ensure(size <= 16 * 1024 * 1024, "bounded raw evidence size")?;
    let compressed = bytes(path, 4 * 1024 * 1024)?;
    let mut raw = Vec::new();
    GzDecoder::new(compressed.as_slice())
        .take(size + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    ensure(raw.len() as u64 == size, "exact expanded evidence size")?;
    Ok(raw)
}

fn ensure(ok: bool, reason: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(reason.into()) }
}
fn rows(value: &Value) -> Result<&[Value], String> {
    if value.as_object().is_some_and(serde_json::Map::is_empty) {
        return Ok(&[]);
    }
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| "captured sequence required".into())
}
fn number(value: &Value) -> Result<u64, String> {
    value
        .as_u64()
        .ok_or_else(|| "captured positive identity required".into())
}
fn encoded(value: &Value) -> String {
    serde_json::to_string(value).unwrap()
}
fn bag<'a>(values: impl IntoIterator<Item = &'a Value>) -> BTreeMap<String, usize> {
    let mut result = BTreeMap::new();
    for value in values {
        *result.entry(encoded(value)).or_default() += 1;
    }
    result
}
fn wrapper(body: &Value) -> Result<(), String> {
    construction::review_record_domain(body)?;
    ensure(
        body["name"] == "MinionModifier"
            && body["type"] == "LIST"
            && body["flags"] == 0
            && body["keywordFlags"] == 0,
        "reviewed outer List filter",
    )?;
    ensure(
        body.get("_positions")
            .is_none_or(|v| rows(v).is_ok_and(<[_]>::is_empty)),
        "tagged outer wrapper requires separate EvalMod authority",
    )?;
    ensure(
        body["value"]
            .as_object()
            .is_some_and(|v| v.len() == 1 && v.contains_key("mod")),
        "reviewed unconditional nested payload",
    )
}
fn candidates(chain: &Value) -> Result<Vec<(&Value, &Value)>, String> {
    let chain = rows(chain)?;
    ensure(
        !chain.is_empty() && chain.len() <= 16,
        "complete bounded ancestry",
    )?;
    let mut ids = BTreeSet::new();
    let mut result = Vec::new();
    for (index, store) in chain.iter().enumerate() {
        ensure(
            ids.insert(number(&store["store_id"])?),
            "unique ancestry identity",
        )?;
        if let Some(next) = chain.get(index + 1) {
            ensure(
                store["parent_store_id"] == next["store_id"],
                "exact parent ancestry",
            )?;
        } else {
            ensure(store["parent_store_id"] == false, "complete root ancestry")?;
        }
        let mut positions = BTreeSet::new();
        let records = rows(&store["records"])?;
        ensure(
            store["names"].as_object().is_some(),
            "complete local name census",
        )?;
        for record in records {
            let position = number(&record["position"])?;
            ensure(
                position > 0 && positions.insert(position),
                "unique local source position",
            )?;
            ensure(
                (store["kind"] == "keyed" && record["bucket"] == "MinionModifier")
                    || (store["kind"] == "list" && record["bucket"] == false),
                "exact source storage kind",
            )?;
            wrapper(&record["body"])?;
            result.push((store, record));
        }
        let count = store["names"]
            .get("MinionModifier")
            .map(number)
            .transpose()?
            .unwrap_or(0);
        ensure(
            count == records.len() as u64,
            "complete local wrapper inventory",
        )?;
        if store["kind"] == "keyed" {
            ensure(
                records
                    .iter()
                    .enumerate()
                    .all(|(i, r)| r["position"] == i + 1),
                "complete keyed bucket positions",
            )?;
        }
    }
    Ok(result)
}

/// Checks the original operation law before any immutable acquisition seal.
/// This is temporal input/output correspondence, not recipe evaluation parity.
pub fn review_delivery(delivery: &Value) -> Result<Value, String> {
    ensure(
        delivery["returned"] == true
            && delivery["original_list_return_observed"] == true
            && delivery["original_list_call_count"] == 1,
        "one original List and complete delivery return",
    )?;
    ensure(
        delivery["source_stores"] == delivery["source_stores_after"],
        "source stores unchanged by receiving",
    )?;
    let input = candidates(&delivery["source_stores"])?;
    let returned = rows(&delivery["list_return"])?;
    ensure(
        input.len() == returned.len(),
        "all original unfiltered candidates returned",
    )?;
    ensure(
        delivery["cfg"]["present"] == true
            && delivery["cfg"]["granted_effect_is_source"] == true
            && delivery["cfg"]["skill_gem_is_source"] == true,
        "exact source configuration",
    )?;
    for (index, ((store, candidate), output)) in input.iter().zip(returned).enumerate() {
        ensure(
            output["position"] == index + 1 && output["body"] == candidate["body"]["value"],
            "original ordered List value correspondence",
        )?;
        let exact = json!({"store_id":store["store_id"],"position":candidate["position"],"bucket":candidate["bucket"]});
        let mut expected = Vec::new();
        // Object identity is captured by the observer, never inferred from equal bodies.
        // The current finite source has distinct wrapper values; aliases remain explicit.
        for source in rows(&output["exact_raw_value_candidates"])? {
            ensure(source == &exact, "unique exact raw value source occurrence")?;
            expected.push(source);
        }
        ensure(
            expected.len() == 1,
            "unique exact raw value source occurrence",
        )?;
    }
    let added = rows(&delivery["added"])?;
    ensure(
        added.len() == returned.len(),
        "every unconditional payload appended once",
    )?;
    let mut counts = BTreeMap::<String, u64>::new();
    for (index, (output, appended)) in returned.iter().zip(added).enumerate() {
        ensure(
            appended["position"] == index + 1 && appended["body"] == output["body"]["mod"],
            "original ordered AddMod payload correspondence",
        )?;
        ensure(
            rows(&appended["returned_value_matches"])? == [json!(index + 1)],
            "exact original returned payload pointer",
        )?;
        ensure(
            appended["original_append_observed"] == true,
            "original AddMod execution",
        )?;
        let before = number(&appended["before_count"])?;
        let after = number(&appended["after_count"])?;
        ensure(
            before.checked_add(1) == Some(after),
            "one original bucket append",
        )?;
        let name = appended["body"]["name"]
            .as_str()
            .ok_or("named nested payload")?
            .to_owned();
        if let Some(previous) = counts.insert(name, after) {
            ensure(previous == before, "same-name append continuity")?;
        }
    }
    Ok(
        json!({"candidates":input.len(),"returned":returned.len(),"appended":added.len(),
        "all_outer_wrappers_unconditional":true,"source_mutation":false,"unique_pointer_correspondence":true}),
    )
}

pub fn review_trace(trace: &Value) -> Result<Value, String> {
    ensure(
        trace["kind"] == "original-inherited-minion-modifier-delivery"
            && trace["original_functions_preserved"] == true
            && trace["observer_removed"] == true
            && trace["business_methods_replaced"] == false,
        "original temporal observer protocol",
    )?;
    let performs = rows(&trace["perform_calls"])?;
    let gates = rows(&trace["gates"])?;
    let deliveries = rows(&trace["deliveries"])?;
    ensure(
        !performs.is_empty() && !deliveries.is_empty(),
        "nonempty actual operation census",
    )?;
    let mut total = 0usize;
    for (index, p) in performs.iter().enumerate() {
        ensure(
            p["perform_id"] == index + 1 && p["returned"] == true,
            "exact completed perform census",
        )?;
        let own_gates: Vec<_> = gates
            .iter()
            .filter(|g| g["perform_id"] == p["perform_id"])
            .collect();
        ensure(
            own_gates
                .iter()
                .all(|g| g["env_id"] == p["env_id"] && g["mode"] == p["mode"]),
            "exact gate environment",
        )?;
        for line in [1844, 2037, 3117, 3397, 3406] {
            ensure(
                own_gates.iter().any(|g| g["line"] == line),
                "complete per-perform gate census",
            )?;
        }
        for gate in own_gates.iter().filter(|g| g["line"] == 2037) {
            ensure(
                gate["has_active_spectre"] == false && gate["branch_entered"] == false,
                "original direct Spectre branch observed skipped",
            )?;
            let skills = rows(&gate["skills"])?;
            ensure(
                !skills.is_empty(),
                "complete Player skill inventory at Spectre gate",
            )?;
            for (i, skill) in skills.iter().enumerate() {
                ensure(
                    skill["ordinal"] == i + 1 && skill["source"]["actor_is_player"] == true,
                    "exact Player skill at Spectre gate",
                )?;
                let id = skill["source"]["effect_id"]
                    .as_str()
                    .ok_or("actual Player effect id")?;
                ensure(
                    !id.starts_with("SummonSpectre"),
                    "Spectre prefix candidate requires separate active-state proof",
                )?;
            }
        }
        let own: Vec<_> = deliveries
            .iter()
            .filter(|d| d["perform_id"] == p["perform_id"])
            .collect();
        ensure(
            own.iter()
                .all(|d| d["env_id"] == p["env_id"] && d["mode"] == p["mode"]),
            "exact delivery environment",
        )?;
        let selected: Vec<_> = own_gates.iter().filter(|g| g["line"] == 1844).collect();
        ensure(selected.len() == 1, "one selected minion branch gate")?;
        let selected = selected[0];
        ensure(
            selected["hostile"] == json!({"kind":"nil"}) || selected["hostile"] == false,
            "hostile recipient remains unsupported",
        )?;
        let skill: Vec<_> = own.iter().filter(|d| d["line"] == 1854).collect();
        ensure(
            skill.len() == usize::from(selected["recipient"]["present"] == true),
            "exact selected Skill delivery census",
        )?;
        for d in skill {
            ensure(
                d["recipient"] == selected["recipient"] && d["from_skill"] == true,
                "exact selected receiving Actor",
            )?;
        }
        let buff_gates: Vec<_> = own_gates.iter().filter(|g| g["line"] == 3117).collect();
        let first = buff_gates[0];
        ensure(
            buff_gates
                .iter()
                .all(|g| g["buffs"] == first["buffs"] && g["recipient"] == first["recipient"]),
            "stable complete buff-loop inventory",
        )?;
        let buffs = rows(&first["buffs"])?;
        let buff_calls: Vec<_> = own.iter().filter(|d| d["line"] == 3123).collect();
        ensure(
            buff_calls.len()
                == if first["recipient"]["present"] == true {
                    buffs.len()
                } else {
                    0
                },
            "complete buff business-call census",
        )?;
        let mut used = BTreeSet::new();
        for d in buff_calls {
            let id = number(&rows(&d["source_stores"])?[0]["store_id"])?;
            ensure(used.insert(id), "buff store delivered exactly once")?;
            let group = buffs
                .iter()
                .find(|b| b["store_id"] == id)
                .ok_or("exact original buff store")?;
            ensure(
                d["recipient"] == first["recipient"] && d["from_skill"] == false,
                "exact buff receiving Actor",
            )?;
            let expected: Vec<_> = rows(&group["records"])?
                .iter()
                .filter(|r| r["body"]["name"] == "MinionModifier")
                .map(|r| &r["body"])
                .collect();
            let actual = candidates(&d["source_stores"])?;
            ensure(
                actual.iter().map(|(_, r)| &r["body"]).collect::<Vec<_>>() == expected,
                "complete actual buff wrapper inventory",
            )?;
        }
        for line in [3397, 3406] {
            let pool: Vec<_> = own_gates.iter().filter(|g| g["line"] == line).collect();
            ensure(
                !pool.is_empty()
                    && pool.iter().all(|g| {
                        g["branch_entered"] == false && g["candidates"] == pool[0]["candidates"]
                    }),
                "Life-pool branch unsupported unless explicitly absent",
            )?;
            let ancestry = rows(&pool[0]["candidates"])?;
            ensure(
                !ancestry.is_empty() && ancestry.len() <= 16,
                "complete Life-pool source ancestry",
            )?;
            let mut identities = BTreeSet::new();
            let names: &[&str] = if line == 3397 {
                &["TakenFromSpectresBeforeYou", "TotalSpectreLife"]
            } else {
                &[
                    "TakenFromCompanionBeforeYou",
                    "TakenFromCompanionBeforeYouFromDeflected",
                    "TotalCompanionLife",
                ]
            };
            for (index, store) in ancestry.iter().enumerate() {
                ensure(
                    identities.insert(number(&store["store_id"])?),
                    "unique Life-pool source ancestor",
                )?;
                let parent = ancestry
                    .get(index + 1)
                    .map_or(json!(false), |s| s["store_id"].clone());
                ensure(
                    store["parent_store_id"] == parent,
                    "complete Life-pool parent chain",
                )?;
                ensure(
                    store["names"].as_object().is_some(),
                    "complete Life-pool name census",
                )?;
                for name in names {
                    ensure(
                        store["names"].get(*name).is_none_or(|v| v == 0),
                        "Life-pool named candidate cannot be omitted",
                    )?;
                }
                ensure(
                    rows(&store["records"])?.is_empty(),
                    "Life-pool gate producer remains unaccounted",
                )?;
            }
        }
        ensure(
            own.iter().all(|d| d["line"] == 1854 || d["line"] == 3123),
            "unreviewed temporal delivery site",
        )?;
        for d in own {
            total += review_delivery(d)?["appended"].as_u64().unwrap() as usize;
        }
    }
    ensure(
        rows(&trace["life_pool_calls"])?.is_empty(),
        "Life-pool calls require separate recipient proof",
    )?;
    for (i, d) in deliveries.iter().enumerate() {
        ensure(
            d["delivery_id"] == i + 1
                && number(&d["perform_id"])? > 0
                && number(&d["perform_id"])? <= performs.len() as u64,
            "exact delivery identity census",
        )?;
    }
    ensure(
        gates
            .iter()
            .all(|g| number(&g["perform_id"]).is_ok_and(|i| i > 0 && i <= performs.len() as u64)),
        "no orphan gate",
    )?;
    Ok(
        json!({"performs":performs.len(),"deliveries":deliveries.len(),"appended":total,
        "life_pool_branches_observed_absent":true,"native_coverage_authorized":false,
        "all_minion_store_writers_accounted":false}),
    )
}

fn matching_source(observed: &Value, constructed: &Value, effect: &Value) -> Result<bool, String> {
    let mut projected = observed.clone();
    for row in projected["matches"].as_array_mut().into_iter().flatten() {
        if let Some(value) = row
            .as_object_mut()
            .ok_or("source occurrence row")?
            .remove("effect_id")
        {
            ensure(&value == effect, "actual source effect identity")?;
        }
    }
    Ok(projected == *constructed)
}

/// Reconciles complete captured wrapper bodies to selected node, configuration,
/// and exact Skill-constructor sources. This is an inventory join, not a claim
/// that cross-source modifier arithmetic may be reordered.
pub fn review_sources(trace: &Value, evidence: &Value) -> Result<Value, String> {
    let state = &evidence["off"];
    let environments = rows(&state["environments"])?;
    let mut inherited = None;
    let mut selected_nodes = BTreeSet::new();
    for member in rows(&evidence["selected"]["memberships"]["passives"])? {
        let token = member["source_token"]
            .as_str()
            .ok_or("exact selected node token")?;
        ensure(
            selected_nodes.insert(token.to_owned()),
            "unique selected passive source",
        )?;
    }
    let repeat: Vec<_> = rows(&state["callback_invocations"])?
        .iter()
        .filter(|r| r["var"] == "repeatMode")
        .collect();
    ensure(
        repeat.len() == 2,
        "both original configuration repeat invocations",
    )?;
    let mut config_body = None;
    for callback in repeat {
        ensure(
            callback["kind"] == "list"
                && callback["argument_fields"] == json!({"val":"AVERAGE"})
                && callback["body"]["first_line"] == 871
                && callback["body"]["last_line"] == 879,
            "exact invoked repeat callback branch",
        )?;
        let before: Vec<_> = rows(&callback["before"]["player_records"])?
            .iter()
            .filter(|r| r["body"]["name"] == "MinionModifier")
            .collect();
        let after: Vec<_> = rows(&callback["after"]["player_records"])?
            .iter()
            .filter(|r| r["body"]["name"] == "MinionModifier")
            .collect();
        ensure(
            before.is_empty() && after.len() == 1,
            "one actual configured inherited wrapper",
        )?;
        wrapper(&after[0]["body"])?;
        if let Some(body) = config_body {
            ensure(body == &after[0]["body"], "same constructed repeat payload")?;
        }
        config_body = Some(&after[0]["body"]);
    }
    let config_body = config_body.ok_or("configured repeat source")?;
    for env in environments {
        let mut expected = vec![config_body];
        for node in rows(&env["nodes"])? {
            let id = number(&node["key"])?;
            ensure(
                selected_nodes.contains(&id.to_string()),
                "constructed node belongs to selected allocation",
            )?;
            for row in rows(&node["value"]["records"])? {
                if row["body"]["name"] == "MinionModifier" {
                    ensure(
                        row["body"]["source"] == format!("Tree:{id}"),
                        "exact node wrapper source",
                    )?;
                    wrapper(&row["body"])?;
                    expected.push(&row["body"]);
                }
            }
        }
        let mut actual = Vec::new();
        for store in rows(&env["player_store"])? {
            if let Some(bucket) = store["buckets"].get("MinionModifier") {
                for row in rows(bucket)? {
                    wrapper(&row["body"])?;
                    actual.push(&row["body"]);
                }
            }
        }
        ensure(
            expected.len() == 30 && bag(expected) == bag(actual.iter().copied()),
            "complete selected inherited constructor inventory",
        )?;
        let current = bag(actual);
        if let Some(old) = &inherited {
            ensure(old == &current, "every constructor ancestry agrees")?;
        }
        inherited = Some(current);
    }
    let inherited = inherited.ok_or("constructed Player stores")?;
    let mut joined = 0;
    let mut recipients = BTreeMap::<String, usize>::new();
    for delivery in rows(&trace["deliveries"])? {
        if delivery["from_skill"] != true {
            continue;
        }
        let source = &delivery["source"];
        ensure(
            source["actor_is_player"] == true && delivery["recipient"]["parent_is_player"] == true,
            "exact Player-supplied recipient",
        )?;
        ensure(
            source["minion"] == delivery["recipient"],
            "source owns exact receiving Actor",
        )?;
        let constructed = rows(&state["skill_construction"])?;
        let mut locals = None;
        for row in constructed.iter().filter(|r| {
            r["phase"] == "after-construction"
                && r["mode"] == delivery["mode"]
                && r["actor_is_player"] == true
                && r["active_effect"]["id"] == source["effect_id"]
        }) {
            if !matching_source(&source["source"], &row["source"], &source["effect_id"])? {
                continue;
            }
            let mut bodies = Vec::new();
            for r in rows(&row["records"])? {
                if r["body"]["name"] == "MinionModifier" {
                    wrapper(&r["body"])?;
                    bodies.push(&r["body"]);
                }
            }
            let current = bag(bodies);
            if let Some(old) = &locals {
                ensure(
                    old == &current,
                    "exact constructor source wrapper inventory",
                )?;
            }
            locals = Some(current);
        }
        let mut expected = inherited.clone();
        for (key, count) in locals.ok_or("exact supplying Skill constructor")? {
            *expected.entry(key).or_default() += count;
        }
        let actual = candidates(&delivery["source_stores"])?;
        ensure(
            expected == bag(actual.iter().map(|(_, r)| &r["body"])),
            "all temporal inherited/local wrappers have constructed sources",
        )?;
        *recipients
            .entry(encoded(&delivery["recipient"]["source"]))
            .or_default() += 1;
        joined += 1;
    }
    Ok(
        json!({"constructor_snapshots":environments.len(),"inherited_wrapper_occurrences":30,
        "tree_wrappers":29,"configured_repeat_wrappers":1,"actual_skill_deliveries_joined":joined,
        "exact_recipient_occurrences":recipients,"recipe_value_evaluation_parity":false,
        "all_later_minion_store_writers_accounted":false}),
    )
}

fn receiving_projection(trace: &Value) -> Result<Value, String> {
    let mut result = trace.clone();
    let mut projected = Vec::new();
    for delivery in rows(&trace["deliveries"])? {
        // Each original trace has already proved ordered one-to-one pointer
        // correspondence. Only incidental cross-name positions are projected.
        let input = candidates(&delivery["source_stores"])?;
        let mut origins = Vec::new();
        let mut ranks = BTreeMap::<(u64, String), usize>::new();
        for (store, record) in &input {
            let name = record["body"]["value"]["mod"]["name"]
                .as_str()
                .ok_or("named receiving bucket")?;
            let key = (number(&store["store_id"])?, name.to_owned());
            let rank = ranks.entry(key).or_default();
            *rank += 1;
            let mut origin = json!({"store_id":store["store_id"],"bucket":record["bucket"],"name":name,"same_name_ordinal":rank});
            if store["kind"] == "keyed" {
                origin["keyed_position"] = record["position"].clone();
            }
            origins.push(origin);
        }
        let mut projected_delivery = delivery.clone();
        for field in ["source_stores", "source_stores_after"] {
            let mut stores = Vec::new();
            for store in rows(&delivery[field])? {
                let mut buckets = BTreeMap::<String, Vec<Value>>::new();
                for record in rows(&store["records"])? {
                    let name = record["body"]["value"]["mod"]["name"]
                        .as_str()
                        .ok_or("named receiving bucket")?;
                    let mut value = record.clone();
                    if store["kind"] == "list" {
                        value.as_object_mut().unwrap().remove("position");
                    }
                    buckets.entry(name.to_owned()).or_default().push(value);
                }
                let mut value = store.clone();
                value["records"] = serde_json::to_value(buckets).unwrap();
                stores.push(value);
            }
            projected_delivery[field] = json!(stores);
        }
        for field in ["list_return", "added"] {
            let mut buckets = BTreeMap::<String, Vec<Value>>::new();
            for (index, row) in rows(&delivery[field])?.iter().enumerate() {
                let name = if field == "list_return" {
                    &row["body"]["mod"]["name"]
                } else {
                    &row["body"]["name"]
                };
                let name = name.as_str().ok_or("named receiving bucket")?;
                let mut value = row.clone();
                let object = value.as_object_mut().unwrap();
                object.remove("position");
                object.remove(if field == "list_return" {
                    "exact_raw_value_candidates"
                } else {
                    "returned_value_matches"
                });
                value["original_source_occurrence"] = origins[index].clone();
                buckets.entry(name.to_owned()).or_default().push(value);
            }
            projected_delivery[field] = serde_json::to_value(buckets).unwrap();
        }
        projected.push(projected_delivery);
    }
    result["deliveries"] = json!(projected);
    Ok(result)
}

pub fn compare_traces(left: &Value, right: &Value, sources: &Value) -> Result<Value, String> {
    let l = review_trace(left)?;
    let r = review_trace(right)?;
    ensure(l == r, "same completed temporal operation census")?;
    let ls = review_sources(left, sources)?;
    let rs = review_sources(right, sources)?;
    ensure(ls == rs, "same exact selected supplying occurrences")?;
    ensure(
        receiving_projection(left)? == receiving_projection(right)?,
        "complete per-name ordered receiving effects differ",
    )?;
    Ok(
        json!({"raw_execution_traces_equal":left==right,"per_name_receiving_effects_equal":true,
        "raw_order_preserved":true,"within_name_order_and_duplicates_exact":true,
        "numeric_reassociation_authority":false,"temporal":l,"sources":ls}),
    )
}

pub fn authenticate_bodies(body: &Value) -> Result<(), String> {
    let retained = bytes(&format!("{DIR}/bodies.json"), 128 * 1024)?;
    ensure(
        hash(&retained) == BODIES
            && serde_json::from_slice::<Value>(&retained).map_err(|e| e.to_string())? == *body,
        "exact complete original operation body authority",
    )?;
    let manifest_bytes = bytes(
        "crates/poe-optimizer-pob/data/pob-source-manifest.json",
        1024 * 1024,
    )?;
    ensure(
        hash(&manifest_bytes) == MANIFEST,
        "exact original source manifest",
    )?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;
    ensure(
        body["source_manifest_sha256"] == MANIFEST
            && body["source_revision"] == manifest["upstream_revision"],
        "body manifest identity",
    )?;
    let mut texts = BTreeMap::new();
    for file in rows(&body["files"])? {
        let path = file["archive"].as_str().ok_or("source archive path")?;
        let stored = bytes(path, 4 * 1024 * 1024)?;
        ensure(
            file["archive_bytes"] == stored.len() && file["archive_sha256"] == hash(&stored),
            "exact source archive bytes",
        )?;
        let raw = if file["encoding"] == "gzip" {
            unpack(path, number(&file["bytes"])?)?
        } else {
            stored
        };
        ensure(
            file["bytes"] == raw.len() && file["sha256"] == hash(&raw),
            "exact full original body bytes",
        )?;
        let pin = rows(&manifest["files"])?
            .iter()
            .find(|p| p["path"] == file["path"])
            .ok_or("original source path")?;
        ensure(
            pin["sha256"] == file["sha256"],
            "source body belongs to manifest",
        )?;
        texts.insert(
            file["path"].as_str().unwrap().to_owned(),
            String::from_utf8(raw).map_err(|e| e.to_string())?,
        );
    }
    for span in rows(&body["spans"])? {
        let text = texts
            .get(span["path"].as_str().ok_or("body span path")?)
            .ok_or("body span original file")?;
        let first = number(&span["first_line"])? as usize;
        let last = number(&span["last_line"])? as usize;
        ensure(
            first > 0 && last >= first && last <= text.lines().count(),
            "bounded complete body span",
        )?;
        let text = text
            .lines()
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        ensure(
            span["body"] == text && span["body_sha256"] == hash(text.as_bytes()),
            "complete source span correspondence",
        )?;
    }
    Ok(())
}

pub fn fixture() -> &'static Value {
    static FIXTURE: OnceLock<Value> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let evidence_bytes = bytes(&format!("{DIR}/source-evidence.json"), 32 * 1024).unwrap();
        assert_eq!(hash(&evidence_bytes), AUTHORITY);
        let evidence: Value = serde_json::from_slice(&evidence_bytes).unwrap();
        assert_eq!(evidence["source_manifest_sha256"], MANIFEST);
        assert_eq!(evidence["strict_trace_comparison_passed"], false);
        let expected = [
            "observed-a.json",
            "observed-b.json",
            "plain-off.json",
            "plain-on.json",
            "acquisition-driver.rs",
            "acquisition-observer.lua",
            "acquisition-helpers.lua",
            "acquisition-bootstrap.rs",
        ];
        let files = rows(&evidence["files"]).unwrap();
        assert_eq!(files.len(), expected.len());
        let mut loads = Vec::new();
        for (pin, name) in files.iter().zip(expected) {
            assert_eq!(pin["name"], name);
            assert_eq!(pin["archive"], format!("{name}.gz"));
            let path = format!("{DIR}/{name}.gz");
            let compressed = bytes(&path, 4 * 1024 * 1024).unwrap();
            assert_eq!(pin["compressed_bytes"], compressed.len());
            assert_eq!(pin["compressed_sha256"], hash(&compressed));
            let raw = unpack(&path, number(&pin["bytes"]).unwrap()).unwrap();
            assert_eq!(pin["sha256"], hash(&raw));
            if name.ends_with(".json") {
                loads.push(serde_json::from_slice::<Value>(&raw).unwrap());
            }
        }
        for key in ["failure_log", "strict_differences"] {
            let pin = &evidence[key];
            let raw = if key == "failure_log" {
                assert_eq!(pin["archive"], "acquisition-failure.log.gz");
                let path = format!("{DIR}/acquisition-failure.log.gz");
                let compressed = bytes(&path, 128 * 1024).unwrap();
                assert_eq!(pin["compressed_bytes"], compressed.len());
                assert_eq!(pin["compressed_sha256"], hash(&compressed));
                unpack(&path, number(&pin["bytes"]).unwrap()).unwrap()
            } else {
                bytes(
                    &format!("{DIR}/{}", pin["path"].as_str().unwrap()),
                    128 * 1024,
                )
                .unwrap()
            };
            assert_eq!(pin["bytes"], raw.len());
            assert_eq!(pin["sha256"], hash(&raw));
        }
        assert_eq!(
            hash(&bytes(evidence["xml"]["path"].as_str().unwrap(), 1024 * 1024).unwrap()),
            evidence["xml"]["sha256"]
        );
        json!({"evidence":evidence,"loads":loads})
    })
}

pub fn reconstruct(input: &Value, body: &Value) -> Result<Value, String> {
    authenticate_bodies(body)?;
    construction::check()?;
    let loads = rows(&input["loads"])?;
    ensure(loads.len() == 4, "fixed four fresh original loads")?;
    for load in loads {
        ensure(
            load["source_hash"] == MANIFEST
                && load["configuration_method_wrappers"] == false
                && load["original_build_output_available"] == true,
            "complete unwrapped original source load",
        )?;
    }
    let a = &loads[0]["additional_observation"];
    let b = &loads[1]["additional_observation"];
    let compared = compare_traces(a, b, construction::fixture())?;
    ensure(
        a["snapshot"] == b["snapshot"]
            && a["snapshot"] == loads[2]["additional_observation"]
            && a["snapshot"] == loads[3]["additional_observation"],
        "independent original scalar controls differ",
    )?;
    ensure(
        input == fixture(),
        "complete supplied temporal acquisition identity",
    )?;
    Ok(
        json!({"schema_version":1,"kind":"finite-original-inherited-minion-delivery",
        "source_authority_sha256":AUTHORITY,"original_bodies_sha256":BODIES,
        "four_scalar_controls_exact":true,"comparison":compared,
        "scope":{"native_coverage_authorized":false,"all_minion_store_writers_accounted":false,
            "actual_list_parent_and_receiving_correspondence":true,"selected_spectre_bypass_absent":true,
            "ordinary_recipe_value_parity":false,"arbitrary_outer_tags_or_recipient_filters":false},
        "remaining":["ExtraSkillStat source inputs and computed-name expansion",
            "Complete direct/local selected Actor supplier membership and native contributor mapping",
            "Dynamic parent-weapon/Hollow Palm/Rallying Cry gates must be joined to the same admitted selection",
            "Direct minionBuffs AddList delivery is separately covered by existing construction/runtime evidence",
            "No final self-flat query or pre-conversion consumer is published by this packet"]}),
    )
}

pub fn check() -> Result<(), String> {
    ensure(
        reconstruct(fixture(), &read("bodies.json"))? == read("review.json"),
        "retained temporal proof differs",
    )
}
pub fn retain() {
    assert_eq!(
        std::env::var("POE_INHERITED_MINION_RETAIN").as_deref(),
        Ok("1")
    );
    let proof = reconstruct(fixture(), &read("bodies.json")).unwrap();
    fs::write(
        root().join(DIR).join("review.json"),
        serde_json::to_string_pretty(&proof).unwrap() + "\n",
    )
    .unwrap();
}

pub fn sources() -> &'static Value {
    construction::fixture()
}
