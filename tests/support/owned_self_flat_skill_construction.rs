//! Offline, finite Skill/support record construction and observed buff transport.
//! No native source inventory, inherited store or unobserved factory is completed.
#[allow(dead_code)]
#[path = "owned_mana_supplier_sources.rs"]
mod supplier;

use flate2::read::GzDecoder;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    sync::OnceLock,
};

pub const DIR: &str = "data/owned/poe2/3887ae68/self-flat-skill-construction";
const BASE: &str = "data/owned/poe2/3887ae68";
const MANIFEST: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const AUTHORITY: &str = "105b36b7a75e02cb33a5882c1d2fcc5ba75661399702f5cac815cf6592045ef9";
const FILES: [(&str, &str); 5] = [
    (
        "src/Modules/CalcActiveSkill.lua",
        "configuration-distance-inputs/CalcActiveSkill.lua.gz",
    ),
    (
        "src/Modules/CalcPerform.lua",
        "configuration-distance-inputs/CalcPerform.lua.gz",
    ),
    (
        "src/Classes/ModList.lua",
        "mana-supplier-sources/passive-ModList.lua.gz",
    ),
    ("src/Classes/ModDB.lua", "life-base-domain/ModDB.lua.gz"),
    (
        "src/Classes/ModStore.lua",
        "configuration-distance-inputs/ModStore.lua.gz",
    ),
];
const SPANS: [(&str, usize, usize, usize); 19] = [
    ("level-value-merge", 0, 22, 51),
    ("scalar-value-only", 0, 54, 66),
    ("global-tag-classification", 0, 69, 78),
    ("mapped-stat-merge", 0, 80, 113),
    ("complete-skill-instance-merge", 0, 116, 141),
    ("support-effect-selection", 0, 144, 233),
    ("support-active-merge-callers", 0, 730, 795),
    ("global-effect-routing", 0, 1033, 1113),
    ("minion-child-construction", 0, 1116, 1185),
    ("buff-group-merge", 1, 42, 67),
    ("minion-payload-delivery", 1, 1161, 1167),
    ("buff-recipient-delivery", 1, 3116, 3134),
    ("flat-list-append", 2, 20, 31),
    ("flat-list-add", 2, 90, 96),
    ("keyed-record-add", 3, 31, 37),
    ("keyed-list-add", 3, 113, 122),
    ("scaled-record-add", 4, 82, 119),
    ("copy-list-add", 4, 121, 125),
    ("scaled-list-add", 4, 127, 135),
];
fn ensure(ok: bool, why: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(why.into()) }
}
fn array(v: &Value) -> Result<&[Value], String> {
    if v.as_object().is_some_and(serde_json::Map::is_empty) {
        return Ok(&[]);
    }
    v.as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| "expected a captured sequence".into())
}
fn token(v: &Value) -> String {
    serde_json::to_string(v).unwrap()
}
fn digest(v: &Value) -> String {
    supplier::hash(token(v).as_bytes())
}
fn bytes(path: &str, limit: u64) -> Result<Vec<u8>, String> {
    let path = supplier::root().join(path);
    ensure(
        fs::metadata(&path).map_err(|e| e.to_string())?.len() <= limit,
        "bounded input file",
    )?;
    fs::read(path).map_err(|e| e.to_string())
}
pub fn read(name: &str) -> Value {
    serde_json::from_slice(&bytes(&format!("{DIR}/{name}"), 2 * 1024 * 1024).unwrap()).unwrap()
}
fn bag<'a>(rows: impl IntoIterator<Item = &'a Value>) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for row in rows {
        *out.entry(token(row)).or_default() += 1;
    }
    out
}
fn positions(body: &Value) -> Result<&[Value], String> {
    let Some(value) = body.get("_positions") else {
        return Ok(&[]);
    };
    let rows = array(value)?;
    for (index, row) in rows.iter().enumerate() {
        ensure(
            row.as_object().is_some_and(|o| o.len() == 2)
                && row["index"] == index + 1
                && row.get("value").is_some(),
            "complete contiguous captured positions",
        )?;
    }
    Ok(rows)
}
fn name_buckets(rows: &[Value]) -> Result<BTreeMap<String, Vec<&Value>>, String> {
    let mut result = BTreeMap::<String, Vec<&Value>>::new();
    for row in rows {
        let name = row["name"].as_str().ok_or("named complete modifier")?;
        result.entry(name.to_owned()).or_default().push(row);
    }
    Ok(result)
}
/// The existing runtime-gate policy permits only cross-stat inventory order at
/// this exact payload path. Every other field and same-stat order stays exact.
pub fn buff_payload_corresponds(left: &Value, right: &Value) -> Result<bool, String> {
    let left_records = name_buckets(array(&left["modifiers"])?)?;
    let right_records = name_buckets(array(&right["modifiers"])?)?;
    let mut left_fields = left.as_object().ok_or("complete buff object")?.clone();
    let mut right_fields = right.as_object().ok_or("complete buff object")?.clone();
    left_fields.remove("modifiers");
    right_fields.remove("modifiers");
    Ok(left_fields == right_fields && left_records == right_records)
}

/// This seal authenticates full original files independently of the retained excerpts.
pub fn review_bodies(bodies: &Value) -> Result<(), String> {
    let mb = bytes(
        "crates/poe-optimizer-pob/data/pob-source-manifest.json",
        1024 * 1024,
    )?;
    ensure(supplier::hash(&mb) == MANIFEST, "source manifest")?;
    let manifest: Value = serde_json::from_slice(&mb).map_err(|e| e.to_string())?;
    ensure(
        bodies["schema_version"] == 1
            && bodies["kind"] == "selected-self-flat-skill-constructor-bodies"
            && bodies["source_manifest"] == MANIFEST
            && bodies["source_revision"] == manifest["upstream_revision"],
        "body identity",
    )?;
    let files = array(&bodies["files"])?;
    ensure(files.len() == FILES.len(), "complete source file census")?;
    let mut texts = Vec::new();
    for (row, (path, archive)) in files.iter().zip(FILES) {
        let archive = format!("{BASE}/{archive}");
        ensure(
            row["path"] == path && row["archive"] == archive,
            "exact source archive path",
        )?;
        let pin = array(&manifest["files"])?
            .iter()
            .find(|p| p["path"] == path)
            .ok_or("manifest entry")?;
        let compressed = bytes(&archive, 128 * 1024)?;
        ensure(
            row["compressed_bytes"] == compressed.len()
                && row["compressed_sha256"] == supplier::hash(&compressed),
            "compressed source identity",
        )?;
        let mut raw = Vec::new();
        GzDecoder::new(compressed.as_slice())
            .take(512 * 1024 + 1)
            .read_to_end(&mut raw)
            .map_err(|e| e.to_string())?;
        ensure(
            raw.len() <= 512 * 1024
                && row["bytes"] == raw.len()
                && row["sha256"] == supplier::hash(&raw)
                && row["sha256"] == pin["sha256"],
            "original complete source identity",
        )?;
        texts.push(String::from_utf8(raw).map_err(|e| e.to_string())?);
    }
    let spans = array(&bodies["spans"])?;
    ensure(spans.len() == SPANS.len(), "complete reviewed body census")?;
    for (row, (role, file, first, last)) in spans.iter().zip(SPANS) {
        let body = texts[file]
            .lines()
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        ensure(
            row["role"] == role
                && row["path"] == FILES[file].0
                && row["first"] == first
                && row["last"] == last
                && row["body"] == body
                && row["sha256"] == supplier::hash(body.as_bytes()),
            "exact reviewed source body",
        )?;
    }
    Ok(())
}

/// Recursive names include nested MinionModifier payloads, zero/inactive records
/// and all declared stat-set/base/quality mappings. No eligibility filter runs.
pub fn review_record_domain(value: &Value) -> Result<(), String> {
    fn visit(v: &Value, depth: usize, work: &mut usize) -> Result<(), String> {
        *work += 1;
        ensure(
            depth < 48 && *work <= 500_000,
            "bounded source record traversal",
        )?;
        match v {
            Value::Object(o) => {
                positions(v)?;
                ensure(
                    !matches!(
                        o.get("name").and_then(Value::as_str),
                        Some("PhysicalMin" | "PhysicalMax" | "ExtraSkillStat")
                    ),
                    "physical or external-stat source requires independent admission",
                )?;
                ensure(
                    !matches!(
                        o.get("kind").and_then(Value::as_str),
                        Some("callback_reference" | "table_reference")
                    ),
                    "unresolved callback or alias",
                )?;
                for k in ["callbacks", "aliases"] {
                    if let Some(x) = o.get(k) {
                        ensure(
                            x.is_null() || array(x).is_ok_and(<[Value]>::is_empty),
                            "unreviewed recipe callback or alias",
                        )?;
                    }
                }
                for x in o.values() {
                    visit(x, depth + 1, work)?;
                }
            }
            Value::Array(a) => {
                for x in a {
                    visit(x, depth + 1, work)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    visit(value, 0, &mut 0)
}
fn source_matches(source: &Value, join: &Value) -> Result<bool, String> {
    let observed = array(&source["matches"])?;
    let expected: Vec<_> = array(&join["matches"])?
        .iter()
        .map(|x| x["observation"].clone())
        .collect();
    Ok(source["source_present"] == !expected.is_empty() && observed == expected)
}
fn occurrence(row: &Value, selected: &Value) -> Result<(), String> {
    let membership = &selected["memberships"];
    if row["actor_is_player"] == true {
        ensure(
            row["actor_parent_is_player"] == false,
            "Player cannot be its own child",
        )?;
        let candidates = array(&membership["observed_player_effects"])?
            .iter()
            .filter(|x| x["effect"] == row["active_effect"]["id"])
            .filter(|x| source_matches(&row["source"], &x["source_join"]).unwrap_or(false))
            .count();
        ensure(
            candidates == 1,
            "exact authored/generated Player source occurrence",
        )?;
    } else {
        ensure(
            row["actor_is_player"] == false && row["actor_parent_is_player"] == true,
            "supplied child Actor relationship",
        )?;
        let candidates = array(&membership["observed_supplied_actors"])?
            .iter()
            .filter(|x| x["profile"] == row["actor_profile"])
            .filter(|x| source_matches(&row["summoner"], &x["source_join"]).unwrap_or(false))
            .filter(|x| {
                array(&x["children"])
                    .is_ok_and(|c| c.iter().any(|c| c["effect"] == row["active_effect"]["id"]))
            })
            .count();
        ensure(
            candidates == 1,
            "exact supplied source/profile/child correspondence",
        )?;
    }
    Ok(())
}
fn is_global(body: &Value) -> Result<bool, String> {
    Ok(positions(body)?
        .iter()
        .any(|t| t["value"]["type"] == "GlobalEffect"))
}
fn records(v: &Value) -> Result<Vec<Value>, String> {
    let rows = array(v)?;
    let mut result = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        ensure(
            row["position"] == index + 1 && row["body"].is_object(),
            "ordered full record body",
        )?;
        review_record_domain(&row["body"])?;
        result.push(row["body"].clone());
    }
    Ok(result)
}
fn gate_mod(body: &Value) -> Result<Value, String> {
    let o = body.as_object().ok_or("full modifier object")?;
    ensure(
        o.keys().all(|k| {
            matches!(
                k.as_str(),
                "name" | "type" | "value" | "source" | "flags" | "keywordFlags" | "_positions"
            )
        }),
        "unreviewed modifier field",
    )?;
    let tags: Vec<_> = positions(body)?
        .iter()
        .map(|x| x["value"].clone())
        .collect();
    let tags = if tags.is_empty() {
        json!({})
    } else {
        json!(tags)
    };
    Ok(
        json!({"name":body["name"],"type":body["type"],"value":body["value"],"source":body["source"],"flags":body["flags"],"keyword_flags":body["keywordFlags"],"tags":tags}),
    )
}
fn buff_projection(buff: &Value) -> Result<Value, String> {
    Ok(
        json!({"fields":buff["fields"],"modifiers":records(&buff["records"])?.iter().map(gate_mod).collect::<Result<Vec<_>,_>>()?}),
    )
}

pub fn review_run(run: &Value, recipes: &Value, selected: &Value) -> Result<Value, String> {
    let recipes = array(&recipes["skills"])?;
    ensure(
        recipes.len() == 40,
        "exact selected skill/support recipe inventory",
    )?;
    let mut recipe_ids = BTreeSet::new();
    for recipe in recipes {
        review_record_domain(recipe)?;
        ensure(
            recipe_ids.insert(token(&recipe["id"])),
            "unique recipe identity",
        )?;
    }
    let rows = array(&run["skill_construction"])?;
    ensure(
        rows.len() == 1536,
        "every before/after constructor observation retained",
    )?;
    let mut effects = BTreeSet::new();
    let mut supports = BTreeSet::new();
    let mut census = BTreeMap::<String, usize>::new();
    let mut local_count = 0;
    let mut global_count = 0;
    let mut buff_count = 0;
    let mut basic = 0;
    for pair in rows.as_chunks::<2>().0 {
        let (before, after) = (&pair[0], &pair[1]);
        ensure(
            before["phase"] == "before-effect-routing" && after["phase"] == "after-construction",
            "exact observer call/return pair",
        )?;
        occurrence(before, selected)?;
        let mut b = before.clone();
        let mut a = after.clone();
        for k in ["phase", "records", "buffs"] {
            b.as_object_mut().unwrap().remove(k);
            a.as_object_mut().ok_or("after object")?.remove(k);
        }
        ensure(
            a == b,
            "routing cannot change exact source/recipient context",
        )?;
        ensure(
            array(&before["buffs"])?.is_empty(),
            "pre-routing buff inventory starts empty",
        )?;
        for effect in array(&before["effects"])? {
            ensure(
                recipe_ids.contains(&token(&effect["id"])),
                "constructed effect must have a full reviewed recipe",
            )?;
            effects.insert(token(&effect["id"]));
        }
        for support in array(&before["supports"])? {
            ensure(
                recipe_ids.contains(&token(&support["id"])),
                "support must have a full reviewed recipe",
            )?;
            supports.insert(token(&support["id"]));
        }
        ensure(
            array(&before["effects"])?
                .iter()
                .any(|e| e == &before["active_effect"]),
            "active effect retained in effect list",
        )?;
        let before_records = records(&before["records"])?;
        let after_records = records(&after["records"])?;
        let mut ordinary = Vec::new();
        let mut global = Vec::new();
        for record in before_records {
            if is_global(&record)? {
                global.push(record);
            } else {
                ordinary.push(record);
            }
        }
        ensure(
            ordinary == after_records,
            "ordinary local records preserve exact order/value/multiplicity",
        )?;
        let mut routed = Vec::new();
        for buff in array(&after["buffs"])? {
            routed.extend(records(&buff["records"])?);
            buff_count += 1;
        }
        ensure(
            bag(global.iter()) == bag(routed.iter()),
            "all removed global bodies accounted exactly once in buff output",
        )?;
        // A bag here proves record conservation only. It is not a sum/product order.
        local_count += ordinary.len();
        global_count += global.len();
        if before["active_effect"]["id"] == "MinionMeleeBow" {
            basic += 1;
        }
        *census.entry(token(&json!({"mode":before["mode"],"effect":before["active_effect"]["id"],"profile":before["actor_profile"],"source":before["source"],"summoner":before["summoner"]}))).or_default()+=1;
    }
    ensure(
        effects == recipe_ids && supports.len() == 12 && basic == 18,
        "full 40-recipe consumption and exact child census",
    )?;
    Ok(
        json!({"constructor_pairs":rows.len()/2,"recipe_definitions":recipe_ids.len(),"support_definitions":supports.len(),"basic_attack_pairs":basic,"ordinary_records":local_count,"global_records":global_count,"buffs":buff_count,"occurrences":census}),
    )
}

pub fn review_delivery(run: &Value, gate: &Value, selected: &Value) -> Result<Value, String> {
    let rows = array(&run["skill_construction"])?;
    let mut buffs = Vec::new();
    for row in rows.iter().filter(|r| r["phase"] == "after-construction") {
        for buff in array(&row["buffs"])? {
            buffs.push((row, buff_projection(buff)?));
        }
    }
    for (field, count) in [("player_buff_visits", 36), ("child_buff_visits", 18)] {
        let visits = array(&gate[field])?;
        ensure(visits.len() == count, "complete buff traversal census")?;
        for visit in visits {
            review_record_domain(&visit["buff"])?;
            let mut matched = false;
            for (row, buff) in &buffs {
                if row["mode"] == visit["mode"]
                    && row["active_effect"]["id"] == visit["effect_id"]
                    && (row["source"] == visit["source"] || row["summoner"] == visit["source"])
                    && row["actor_is_player"] == visit["actor_is_player"]
                    && row["actor_parent_is_player"] == visit["actor_parent_is_player"]
                    && if field == "player_buff_visits" {
                        buff_payload_corresponds(buff, &visit["buff"])?
                    } else {
                        *buff == visit["buff"]
                    }
                {
                    matched = true;
                }
            }
            ensure(
                matched,
                "traversed buff must match constructed source, Actor and complete payload",
            )?;
        }
    }
    ensure(
        array(&gate["child_merges"])?.is_empty(),
        "no supplied-child buff merge was observed",
    )?;
    let deliveries = array(&gate["group_deliveries"])?;
    ensure(
        deliveries.len() == 28,
        "complete original group-delivery census",
    )?;
    let mut recipients = BTreeMap::<String, usize>::new();
    let mut supplied_actors = BTreeMap::<String, usize>::new();
    for delivery in deliveries {
        review_record_domain(&delivery["modifiers"])?;
        ensure(
            delivery["exact_recipient_store"] == true,
            "actual receiving store identity",
        )?;
        let delivered = name_buckets(array(&delivery["modifiers"])?)?;
        let mut matching = Vec::new();
        for (row, buff) in &buffs {
            if row["mode"] == delivery["mode"]
                && buff["fields"]["name"] == delivery["buff_key"]
                && name_buckets(array(&buff["modifiers"])?)? == delivered
            {
                matching.push((row, buff));
            }
        }
        ensure(
            !matching.is_empty(),
            "delivered full payload must correspond to a constructed buff",
        )?;
        match delivery["recipient"].as_str() {
            Some("selected_minion") => {
                ensure(
                    delivery["line"] == 3128
                        && matching.iter().all(|(_, b)| {
                            b["fields"]["applyMinions"] == true && b["fields"]["type"] == "Buff"
                        }),
                    "declared minion-buff delivery",
                )?;
                let matches = array(&delivery["selected_recipient"]["matches"])?;
                ensure(matches.len() == 1, "one exact recipient occurrence")?;
                let recipient = &matches[0];
                let corresponding = array(&selected["memberships"]["observed_supplied_actors"])?
                    .iter()
                    .filter(|r| {
                        r["ordinal"] == recipient["ordinal"]
                            && source_matches(&recipient["source"], &r["source_join"])
                                .unwrap_or(false)
                    })
                    .collect::<Vec<_>>();
                ensure(
                    corresponding.len() == 1
                        && corresponding[0]["summon_effect"] == recipient["effect_id"],
                    "exact typed selected recipient source",
                )?;
                ensure(
                    rows.iter().any(|r| {
                        r["mode"] == delivery["mode"]
                            && r["actor_is_player"] == false
                            && r["actor_parent_is_player"] == true
                            && r["actor_profile"] == corresponding[0]["profile"]
                            && r["summoner"] == recipient["source"]
                            && array(&corresponding[0]["children"]).is_ok_and(|children| {
                                children
                                    .iter()
                                    .any(|child| child["effect"] == r["active_effect"]["id"])
                            })
                    }),
                    "selected recipient has its exact observed child Skill constructor",
                )?;
                *supplied_actors
                    .entry(token(&recipient["effect_id"]))
                    .or_default() += 1;
            }
            Some("enemy") => ensure(
                delivery["line"] == 3132
                    && matching
                        .iter()
                        .all(|(_, b)| b["fields"]["type"] == "Debuff"),
                "declared enemy debuff delivery",
            )?,
            _ => return Err("unreviewed delivery recipient".into()),
        }
        *recipients.entry(token(&delivery["recipient"])).or_default() += 1;
    }
    Ok(
        json!({"player_buff_visits":36,"child_buff_visits":18,"child_merges":0,"group_deliveries":deliveries.len(),"recipients":recipients,"supplied_actor_deliveries":supplied_actors,"group_payload_correspondence_only":true,"unique_delivering_source_attribution":false,"numeric_scaling_or_order_proved":false,"unavailable_traversal_flags_used":false}),
    )
}

pub fn fixture() -> &'static Value {
    static FIXTURE: OnceLock<Value> = OnceLock::new();
    FIXTURE.get_or_init(||{
        let authority_bytes=bytes(&format!("{DIR}/authority.json"),32*1024).unwrap();
        assert_eq!(supplier::hash(&authority_bytes),AUTHORITY);
        let authority:Value=serde_json::from_slice(&authority_bytes).unwrap();
        let mut sources=Vec::new();
        for pin in array(&authority["files"]).unwrap(){
            let b=bytes(pin["path"].as_str().unwrap(),2*1024*1024).unwrap();
            assert_eq!(b.len(),pin["bytes"]);assert_eq!(supplier::hash(&b),pin["sha256"]);
            sources.push(serde_json::from_slice::<Value>(&b).unwrap());
        }
        let acquisition=supplier::acquisition_fixture().unwrap();
        let selected=supplier::reconcile_selected_inputs(&supplier::load()).unwrap();
        let mut gates=Vec::new();
        for pin in array(&sources[2]["raw_runs"]).unwrap().iter().filter(|p|p["path"].as_str().unwrap().contains("observed")){
            let b=bytes(&format!("{BASE}/self-flat-runtime-gates/{}",pin["path"].as_str().unwrap()),1024*1024).unwrap();
            assert_eq!(b.len(),pin["bytes"]);assert_eq!(supplier::hash(&b),pin["sha256"]);
            let raw:Value=serde_json::from_slice(&b).unwrap();gates.push(raw["additional_observation"].clone());
        }
        assert_eq!(gates.len(),2);
        json!({"authority":authority,"recipes":sources[0],"selected":selected,"off":acquisition.jit_off["state"],"on":acquisition.jit_on["state"],"gates":gates})
    })
}
pub fn reconstruct(input: &Value, bodies: &Value) -> Result<Value, String> {
    review_bodies(bodies)?;
    ensure(
        supplier::hash(&bytes(&format!("{DIR}/authority.json"), 32 * 1024)?) == AUTHORITY
            && input["authority"] == read("authority.json"),
        "exact proof authority",
    )?;
    let off = review_run(&input["off"], &input["recipes"], &input["selected"])?;
    let on = review_run(&input["on"], &input["recipes"], &input["selected"])?;
    ensure(off == on, "independent constructor-domain correspondence")?;
    let mut delivery = Vec::new();
    for gate in array(&input["gates"])? {
        let proof = review_delivery(&input["off"], gate, &input["selected"])?;
        ensure(
            proof == review_delivery(&input["on"], gate, &input["selected"])?,
            "independent constructed-to-delivered correspondence",
        )?;
        delivery.push(proof);
    }
    ensure(
        delivery.len() == 2 && delivery[0] == delivery[1],
        "independent delivery proof",
    )?;
    ensure(
        input == fixture(),
        "complete supplied records/context must match authenticated acquisition",
    )?;
    Ok(
        json!({"schema_version":1,"kind":"finite-selected-self-flat-skill-construction","authority_sha256":AUTHORITY,"bodies_sha256":digest(bodies),"acquisition_and_correspondence_sha256":digest(input),"selected_source_membership_sha256":digest(&input["selected"]),"recipes_sha256":digest(&input["recipes"]),"construction":off,"delivery":delivery[0],"scope":input["authority"]["scope"],"remaining":input["authority"]["remaining"]}),
    )
}
pub fn check() -> Result<(), String> {
    ensure(
        reconstruct(fixture(), &read("bodies.json"))? == read("review.json"),
        "retained finite proof differs",
    )
}
pub fn retain() {
    assert_eq!(
        std::env::var("POE_SELF_FLAT_SKILL_RETAIN").as_deref(),
        Ok("1")
    );
    let proof = reconstruct(fixture(), &read("bodies.json")).unwrap();
    fs::write(
        supplier::root().join(DIR).join("review.json"),
        serde_json::to_string_pretty(&proof).unwrap() + "\n",
    )
    .unwrap();
}
