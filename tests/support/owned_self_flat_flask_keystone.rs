//! Finite selected Flask/Charm and Keystone source proof. No native coverage.
#[allow(dead_code)]
#[path = "owned_self_flat_minion_store.rs"]
pub(super) mod minion_store;
use construction::radius;
use inherited::construction;
use minion_store::inherited;
use radius::supplier;

use flate2::read::GzDecoder;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    sync::OnceLock,
};

pub const DIR: &str = "data/owned/poe2/3887ae68/self-flat-flask-keystone";
const BODIES: &str = "737a4b750c6f1d8e41bd771676b0e06af552f02c2cc1c6556bd4b228e3782546";
const MANIFEST: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const XML: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";
const PROFILE: &str = "RaisedSkeletonSniper";
const SUMMON: &str = "SummonSkeletalSnipersPlayer";
const EMPTY_SLOTS: [&str; 5] = ["Flask 1", "Flask 2", "Charm 1", "Charm 2", "Charm 3"];

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
        .ok_or_else(|| "complete captured sequence required".into())
}
fn bytes(path: &str, limit: u64) -> Result<Vec<u8>, String> {
    let path = supplier::root().join(path);
    ensure(
        fs::metadata(&path).map_err(|e| e.to_string())?.len() <= limit,
        "bounded retained file",
    )?;
    fs::read(path).map_err(|e| e.to_string())
}
fn digest(value: &Value) -> String {
    supplier::hash(&serde_json::to_vec(value).unwrap())
}
pub fn read(name: &str) -> Value {
    serde_json::from_slice(&bytes(&format!("{DIR}/{name}"), 256 * 1024).unwrap()).unwrap()
}
fn unpack(file: &Value) -> Result<Vec<u8>, String> {
    let compressed = bytes(
        file["archive"].as_str().ok_or("source archive")?,
        128 * 1024,
    )?;
    ensure(
        file["archive_bytes"] == compressed.len()
            && file["archive_sha256"] == supplier::hash(&compressed),
        "compressed source authority",
    )?;
    let mut raw = Vec::new();
    GzDecoder::new(compressed.as_slice())
        .take(512 * 1024 + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    ensure(raw.len() <= 512 * 1024, "bounded expanded source")?;
    Ok(raw)
}

pub fn review_bodies(bodies: &Value) -> Result<(), String> {
    let manifest = bytes(
        "crates/poe-optimizer-pob/data/pob-source-manifest.json",
        1024 * 1024,
    )?;
    ensure(
        supplier::hash(&manifest) == MANIFEST && bodies["source_manifest_sha256"] == MANIFEST,
        "original source manifest",
    )?;
    let manifest: Value = serde_json::from_slice(&manifest).map_err(|e| e.to_string())?;
    ensure(
        bodies["source_revision"] == manifest["upstream_revision"],
        "original revision",
    )?;
    let mut files = BTreeMap::new();
    for file in rows(&bodies["files"])? {
        let text = String::from_utf8(unpack(file)?)
            .map_err(|e| e.to_string())?
            .replace("\r\n", "\n");
        let pin = rows(&manifest["files"])?
            .iter()
            .find(|p| p["path"] == file["path"])
            .ok_or("manifest source file")?;
        ensure(
            file["sha256"] == pin["sha256"]
                && file["sha256"] == supplier::hash(text.as_bytes())
                && file["bytes"] == text.len(),
            "complete original source file",
        )?;
        ensure(
            files
                .insert(file["path"].as_str().ok_or("source path")?.to_owned(), text)
                .is_none(),
            "duplicate source file",
        )?;
    }
    ensure(files.len() == 8, "complete source-file census")?;
    for span in rows(&bodies["spans"])? {
        let file = files
            .get(span["path"].as_str().ok_or("span source path")?)
            .ok_or("span source file")?;
        let first = span["first"].as_u64().ok_or("span start")? as usize;
        let last = span["last"].as_u64().ok_or("span end")? as usize;
        ensure(
            first > 0 && last >= first && last <= file.lines().count(),
            "source span bounds",
        )?;
        let text = file
            .lines()
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        ensure(
            span["body"] == text && span["sha256"] == supplier::hash(text.as_bytes()),
            "source body changed",
        )?;
    }
    ensure(
        rows(&bodies["spans"])?.len() == 17,
        "complete source span census",
    )?;
    let mut next = 1193;
    for segment in rows(&bodies["primary_transition"])? {
        let last = segment["last"].as_u64().ok_or("transition end")?;
        ensure(
            segment["first"] == next && last >= next && last <= 1859,
            "gap or overlap in complete primary transition",
        )?;
        next = last + 1;
    }
    ensure(
        next == 1860,
        "complete primary transition ends after original Keystone query",
    )?;
    let license = unpack(&bodies["license"])?;
    ensure(
        bodies["license"]["bytes"] == license.len()
            && bodies["license"]["sha256"] == supplier::hash(&license),
        "retained upstream license",
    )?;
    let original = bytes(&format!("{DIR}/bodies.json"), 256 * 1024)?;
    ensure(
        supplier::hash(&original) == BODIES
            && serde_json::from_slice::<Value>(&original).map_err(|e| e.to_string())? == *bodies,
        "fixed reviewed destination certificate",
    )
}

pub fn selected() -> &'static Value {
    &minion_store::fixture()["selected"]
}
pub fn radius_fixture() -> &'static Value {
    radius::fixture()
}
pub fn inherited_fixture() -> &'static Value {
    inherited::fixture()
}

fn no_flask_item(item: &Value) -> Result<(), String> {
    if item == &json!({"present":false}) {
        return Ok(());
    }
    ensure(item["present"] == true, "item presence is unavailable")?;
    let kind = item["fields"]["type"]
        .as_str()
        .ok_or("complete resolved item type required")?;
    ensure(
        item["base_fields"]["type"] == kind,
        "resolved item/base type disagreement",
    )?;
    ensure(
        !matches!(kind, "Flask" | "Charm"),
        "selected Flask/Charm provider requires new proof",
    )
}

pub fn review_flasks(trace: &Value, membership: &Value) -> Result<Value, String> {
    let equipment = rows(&membership["memberships"]["equipment"])?;
    ensure(equipment.len() == 9, "nine selected owned equipment uses")?;
    let mut uses = BTreeMap::new();
    let mut owned_uses = BTreeSet::new();
    for e in equipment {
        let slot = e["slot"].as_str().ok_or("exact owned equipment slot")?;
        let item = e["source_item_id"]
            .as_str()
            .ok_or("source item ID")?
            .parse::<u64>()
            .map_err(|e| e.to_string())?;
        ensure(
            e["preset"] == membership["selection"]["build"]["equipment"]
                && e["source_use"]["parent"]["ordinal"] == 597
                && e["source_use"]["source"]["source_sha256"] == XML
                && uses.insert(slot, item).is_none()
                && owned_uses.insert(serde_json::to_string(&e["owned_use"]).unwrap()),
            "exact selected ItemSet/use membership",
        )?;
    }
    ensure(
        uses.values().copied().collect::<BTreeSet<_>>().len() == 8,
        "eight actual selected item records; repeated ring use preserved",
    )?;
    let calls = rows(&trace["init_calls"])?;
    ensure(calls.len() == 21, "complete fresh environment census")?;
    let mut modes = BTreeMap::<String, usize>::new();
    let mut all_candidates = 0;
    for call in calls {
        *modes
            .entry(call["mode"].as_str().ok_or("source mode")?.into())
            .or_default() += 1;
        for phase in ["entry_slots", "exit_slots"] {
            let slots = &call[phase];
            ensure(
                slots["item_zero"] == json!({"present":false}),
                "items[0] must remain an observed absent provider",
            )?;
            let ordered = rows(&slots["ordered"])?;
            ensure(ordered.len() == 113, "all ordered slots are required")?;
            let mut empty = BTreeSet::new();
            let mut actual = BTreeMap::new();
            for slot in ordered {
                let name = slot["fields"]["slotName"].as_str().ok_or("slot name")?;
                no_flask_item(&slot["selected_item"])?;
                if EMPTY_SLOTS.contains(&name) {
                    ensure(
                        slot["fields"]["selItemId"] == 0
                            && slot["fields"]["active"] == false
                            && slot["selected_item"] == json!({"present":false})
                            && slot["spec_item"] == json!({"present":false})
                            && empty.insert(name),
                        "exact empty selected Flask/Charm slot",
                    )?;
                }
                if slot["selected_item"]["present"] == true {
                    let id = slot["selected_item"]["fields"]["id"]
                        .as_u64()
                        .ok_or("resolved original item identity")?;
                    ensure(
                        slot["fields"]["selItemId"] == id && actual.insert(name, id).is_none(),
                        "unique selected item use",
                    )?;
                }
            }
            ensure(
                empty.len() == 5 && actual == uses,
                "complete selected ItemSet correspondence",
            )?;
        }
        let own: Vec<_> = rows(&trace["slot_candidates"])?
            .iter()
            .filter(|r| r["invocation_id"] == call["invocation_id"])
            .collect();
        ensure(own.len() == 112, "complete actual slot candidate census")?;
        let mut present = BTreeMap::new();
        for candidate in own {
            no_flask_item(&candidate["resolved_item"])?;
            if candidate["resolved_item"]["present"] == true {
                let slot = candidate["slot_name"]
                    .as_str()
                    .ok_or("actual candidate slot")?;
                let id = candidate["resolved_item"]["fields"]["id"]
                    .as_u64()
                    .ok_or("actual item ID")?;
                ensure(
                    present.insert(slot, id).is_none(),
                    "duplicate actual item use",
                )?;
            }
            all_candidates += 1;
        }
        ensure(
            present == uses,
            "actual selection agrees with exact owned uses",
        )?;
    }
    ensure(
        all_candidates == 2352 && rows(&trace["slot_candidates"])?.len() == all_candidates,
        "no hidden or orphan candidate",
    )?;
    // This separately checks every ordered/map identity, Ring3's sole skip,
    // selected55+2, all fresh calls and absence of override/reuse/acceleration.
    radius::review_trace(trace)?;
    ensure(
        membership == selected(),
        "independent selected source authority",
    )?;
    Ok(
        json!({"fresh_environments":21,"modes":modes,"ordered_slots_per_environment":113,
        "actual_candidates_per_environment":112,"actual_candidates":all_candidates,
        "selected_item_records":8,"selected_item_uses":9,"empty_flask_slots":2,"empty_charm_slots":3,
        "selected_item_set_source_ordinal":597,"flask_and_charm_maps_empty_by_complete_construction":true,
        "positive_payload_evaluation_proved":false,"flask_flags_assumed_false":false}),
    )
}

pub fn reject_keystone(value: &Value) -> Result<(), String> {
    match value {
        Value::Object(fields) => {
            ensure(
                fields.get("name").is_none_or(|v| v != "Keystone"),
                "Keystone candidate requires separate expansion proof",
            )?;
            for child in fields.values() {
                reject_keystone(child)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                reject_keystone(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}
pub fn review_keystones(trace: &Value, membership: &Value) -> Result<Value, String> {
    let actors = rows(&membership["memberships"]["observed_supplied_actors"])?;
    let exact: Vec<_> = actors.iter().filter(|a| a["profile"] == PROFILE).collect();
    ensure(exact.len() == 1, "one exact selected Sniper source")?;
    let exact = exact[0];
    ensure(
        exact["summon_effect"] == SUMMON && exact["ordinal"] == 4,
        "exact Sniper summoner definition and occurrence",
    )?;
    let source = rows(&exact["source_join"]["matches"])?;
    ensure(
        source.len() == 1
            && source[0]["source"]["source"]["ordinal"] == 211
            && source[0]["source"]["source"]["source_sha256"] == XML,
        "exact saved summoner Gem source",
    )?;
    let observation = &source[0]["observation"];
    let gates: Vec<_> = rows(&trace["gates"])?
        .iter()
        .filter(|g| g["line"] == 1844 && g["recipient"]["profile"] == PROFILE)
        .collect();
    ensure(
        gates.len() == 2,
        "two exact primary Sniper query predecessors",
    )?;
    let mut modes = BTreeSet::new();
    let mut calls = Vec::new();
    for gate in gates {
        let recipient = &gate["recipient"];
        ensure(
            gate["hostile"] == false
                && recipient["present"] == true
                && recipient["is_selected_minion"] == true
                && recipient["parent_is_player"] == true,
            "friendly selected Sniper recipient",
        )?;
        let matches = rows(&recipient["source"]["matches"])?;
        ensure(
            matches.len() == 1
                && matches[0]["ordinal"] == exact["ordinal"]
                && matches[0]["effect_id"] == SUMMON
                && matches[0]["source"]["source_present"] == true
                && rows(&matches[0]["source"]["matches"])? == [observation.clone()],
            "exact receiving summoner source identity",
        )?;
        ensure(
            recipient["children"]
                == json!([
            {"effect_id":"MinionMeleeBow","owns_actor":true,"summoner_owns_actor":true},
            {"effect_id":"GasShotSkeletonSniperMinion","owns_actor":true,"summoner_owns_actor":true}]),
            "exact declared receiving children",
        )?;
        let deliveries: Vec<_> = rows(&trace["deliveries"])?
            .iter()
            .filter(|d| d["perform_id"] == gate["perform_id"] && d["line"] == 1854)
            .collect();
        ensure(
            deliveries.len() == 1,
            "one actual inherited primary predecessor",
        )?;
        let delivery = deliveries[0];
        ensure(
            delivery["recipient"] == *recipient
                && delivery["mode"] == gate["mode"]
                && delivery["env_id"] == gate["env_id"],
            "exact primary receiving store/context",
        )?;
        reject_keystone(&delivery["added"])?;
        ensure(
            rows(&delivery["added"])?.len() == 30,
            "complete actual inherited payload census",
        )?;
        modes.insert(gate["mode"].as_str().ok_or("primary mode")?.to_owned());
        calls.push(
            json!({"perform_id":gate["perform_id"],"env_id":gate["env_id"],"mode":gate["mode"],
            "recipient_store_id":delivery["recipient_store_id"],"recipient":recipient,
            "ordered_original_appends":30,"ordered_payload_sha256":digest(&delivery["added"])}),
        );
    }
    ensure(
        modes == BTreeSet::from(["MAIN".into(), "CALCULATOR".into()]),
        "exact observed primary modes",
    )?;
    // Original business calls, complete candidate ancestry, append order and
    // all per-perform false secondary branches are checked by the prior proof.
    inherited::review_trace(trace)?;
    ensure(
        rows(&trace["perform_calls"])?.len() == 18 && rows(&trace["life_pool_calls"])?.is_empty(),
        "secondary life-pool calls must remain absent",
    )?;
    ensure(
        membership == selected(),
        "independent selected source authority",
    )?;
    Ok(
        json!({"primary":calls,"primary_query_result_deduced_not_observed":true,
        "secondary_perform_census":18,"secondary_query_executed":false,
        "secondary_unexecuted_store_labelled_empty":false,"keystone_map_evaluation_required":false,
        "numeric_reduction_order_inferred":false}),
    )
}

fn authenticate_dependencies() -> Result<(), String> {
    static CHECKED: OnceLock<Result<(), String>> = OnceLock::new();
    CHECKED
        .get_or_init(|| {
            radius::check()?;
            minion_store::check()?;
            construction::check_child()
        })
        .clone()
}
pub fn reconstruct(
    radius_report: &Value,
    inherited_report: &Value,
    membership: &Value,
    bodies: &Value,
) -> Result<Value, String> {
    authenticate_dependencies()?;
    review_bodies(bodies)?;
    let radius_runs = rows(&radius_report["observed"])?;
    ensure(
        radius_runs.len() == 2,
        "two independently retained radius traces",
    )?;
    let flasks = review_flasks(&radius_runs[0], membership)?;
    ensure(
        flasks == review_flasks(&radius_runs[1], membership)?,
        "independent Flask/Charm construction correspondence",
    )?;
    let loads = rows(&inherited_report["loads"])?;
    ensure(loads.len() == 4, "four retained original inherited loads")?;
    let keystones = review_keystones(&loads[0]["additional_observation"], membership)?;
    ensure(
        keystones == review_keystones(&loads[1]["additional_observation"], membership)?,
        "independent exact primary recipient and payload correspondence",
    )?;
    ensure(
        radius_report == radius_fixture() && inherited_report == inherited_fixture(),
        "complete independently authenticated captures",
    )?;
    Ok(
        json!({"schema_version":1,"kind":"finite-selected-flask-charm-keystone-suppliers",
        "bodies_sha256":BODIES,"radius_acquisition_sha256":digest(radius_report),
        "inherited_acquisition_sha256":digest(inherited_report),"selected_bindings_sha256":digest(membership),
        "flasks_and_charms":flasks,"keystones":keystones,
        "primary_transition":bodies["primary_transition"],
        "source_deductions":{"fresh_parentless_sniper_store":true,"initializer_contains_no_keystone":true,
            "all_intervening_destinations_through_primary_query_reviewed":true,"flask_or_charm_payload_before_either_flask_call_absent":true,
            "primary_keystone_candidates_empty":true,"secondary_keystone_call_unreachable_in_retained_runs":true},
        "scope":{"selected_source_boundary_closed":true,"new_source_execution":false,
            "arbitrary_flasks_or_charms_covered":false,"unobserved_override_or_reuse_covered":false,
            "all_temporal_minion_writers_accounted":false,"native_membership_complete":false,
            "native_coverage_authorized":false,"current_native_package_changed":false,
            "cross_acquisition_runtime_ids_joined":false,"global_flags_assumed_false":false},
        "remaining":["Later Party extraAura and original AffectedByAuraMod delivery",
            "Later Actor helpers, defence/trigger call edges and Action-local factories",
            "Exact source-to-native contributor census and final pre-conversion publication",
            "Changed equipment, overrides/reuse, recipients or source bodies require new review"]}),
    )
}
pub fn check() -> Result<(), String> {
    ensure(
        reconstruct(
            radius_fixture(),
            inherited_fixture(),
            selected(),
            &read("bodies.json"),
        )? == read("review.json"),
        "retained Flask/Charm/Keystone review differs",
    )
}
pub fn retain() {
    assert_eq!(
        std::env::var("POE_SELF_FLAT_FLASK_KEYSTONE_RETAIN").as_deref(),
        Ok("1")
    );
    let v = reconstruct(
        radius_fixture(),
        inherited_fixture(),
        selected(),
        &read("bodies.json"),
    )
    .unwrap();
    fs::write(
        supplier::root().join(DIR).join("review.json"),
        serde_json::to_string_pretty(&v).unwrap() + "\n",
    )
    .unwrap();
}
