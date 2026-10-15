//! Finite item construction correspondence. This is not store delivery or coverage.
#[allow(dead_code)]
#[path = "owned_mana_radius_sources.rs"]
mod radius;
use radius::supplier;

use flate2::read::GzDecoder;
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    sync::OnceLock,
};

const MANIFEST: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const RECIPE: &str = "data/owned/poe2/3887ae68/self-flat-attack-damage/selected-domain.json";
const RECIPE_VALUE: &str = "abc764c4728687f465ffea2ea8b16e597e2a91121ee7e2e37a5bbe7d2c90e213";

pub fn read(name: &str) -> Value {
    supplier::read(format!("{}/{name}", supplier::DIR))
}
fn ensure(ok: bool, why: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(why.into()) }
}
fn array<'a>(v: &'a Value, why: &str) -> Result<&'a [Value], String> {
    if v.as_object().is_some_and(Map::is_empty) {
        return Ok(&[]);
    }
    v.as_array().map(Vec::as_slice).ok_or_else(|| why.into())
}
fn digest(v: &Value) -> String {
    supplier::hash(&serde_json::to_vec(v).unwrap())
}
fn one<'a>(mut rows: impl Iterator<Item = &'a Value>, why: &str) -> Result<&'a Value, String> {
    let row = rows.next().ok_or_else(|| why.to_owned())?;
    ensure(rows.next().is_none(), why)?;
    Ok(row)
}

pub fn fixture() -> &'static Value {
    static FIXTURE: OnceLock<Value> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        radius::check().unwrap();
        let recipe=supplier::read(RECIPE);
        assert_eq!(digest(&recipe),RECIPE_VALUE,"independent original recipe authority");
        let binding=supplier::reconcile_selected_inputs(&supplier::load()).unwrap();
        let acquisition=supplier::acquisition_fixture().unwrap();
        let project=|v:&Value|v["state"]["environments"].as_array().unwrap().iter()
            .map(|e|json!({"mode":e["mode"],"items":e["items"]})).collect::<Vec<_>>();
        let radius_runs=radius::fixture()["observed"].as_array().unwrap().iter().map(|t|json!({
            "init_calls":t["init_calls"].as_array().unwrap().iter().map(|c|json!({"invocation_id":c["invocation_id"],"mode":c["mode"]})).collect::<Vec<_>>(),
            "slot_candidates":t["slot_candidates"]})).collect::<Vec<_>>();
        json!({"recipe":recipe,"memberships":binding["memberships"]["equipment"],
            "selection":binding["selection"],"historical_publication":binding["publication"],
            "off":project(&acquisition.jit_off),"on":project(&acquisition.jit_on),"radius":radius_runs})
    })
}

pub fn review_bodies(bodies: &Value) -> Result<(), String> {
    let bytes =
        fs::read(supplier::root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"))
            .map_err(|e| e.to_string())?;
    ensure(supplier::hash(&bytes) == MANIFEST, "independent manifest")?;
    let manifest: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    ensure(
        bodies["schema_version"] == 1
            && bodies["kind"] == "selected-equipment-construction-source-bodies"
            && bodies["source_manifest"] == MANIFEST
            && bodies["source_revision"] == REVISION
            && bodies["license"] == "upstream-LICENSE.md.gz",
        "source body authority",
    )?;
    let paths = [
        ("src/Classes/ItemsTab.lua", "equipment-ItemsTab.lua.gz"),
        ("src/Classes/Item.lua", "equipment-Item.lua.gz"),
        ("src/Modules/ItemTools.lua", "equipment-ItemTools.lua.gz"),
    ];
    let rows = array(&bodies["files"], "source files")?;
    ensure(rows.len() == paths.len(), "complete source file census")?;
    let mut texts = BTreeMap::new();
    for (row, (path, archive)) in rows.iter().zip(paths) {
        ensure(
            row["path"] == path && row["archive"] == archive,
            "exact source archive path",
        )?;
        let pin = one(
            array(&manifest["files"], "manifest files")?
                .iter()
                .filter(|r| r["path"] == path),
            "manifest source file",
        )?;
        let file = supplier::root().join(supplier::DIR).join(archive);
        ensure(
            fs::metadata(&file).map_err(|e| e.to_string())?.len() <= 64 * 1024,
            "bounded compressed source",
        )?;
        let compressed = fs::read(file).map_err(|e| e.to_string())?;
        ensure(
            row["compressed_bytes"] == compressed.len()
                && row["compressed_sha256"] == supplier::hash(&compressed),
            "source archive commitment",
        )?;
        let mut raw = Vec::new();
        GzDecoder::new(compressed.as_slice())
            .take(256 * 1024 + 1)
            .read_to_end(&mut raw)
            .map_err(|e| e.to_string())?;
        ensure(
            raw.len() <= 256 * 1024
                && row["bytes"] == raw.len()
                && row["sha256"] == supplier::hash(&raw)
                && row["sha256"] == pin["sha256"],
            "complete manifest-pinned source bytes",
        )?;
        texts.insert(path, String::from_utf8(raw).map_err(|e| e.to_string())?);
    }
    let spans = [
        ("original-item-load-branch", 0, 1199, 1255),
        ("variant-mode-predicates", 1, 358, 368),
        ("active-bonded-slot-list", 1, 2198, 2220),
        ("variant-selection", 1, 2289, 2318),
        ("variant-multiplicity", 1, 2320, 2336),
        ("primary-slot", 1, 2355, 2371),
        ("local-record-consumption", 1, 2384, 2411),
        ("complete-slot-constructor", 1, 2414, 2669),
        ("slot-list-construction", 1, 2671, 2680),
        ("ranged-line-construction", 1, 2682, 2692),
        ("complete-item-constructor", 1, 2694, 2863),
        ("range-and-formatting-boundary", 2, 14, 338),
    ];
    let actual = array(&bodies["spans"], "body spans")?;
    ensure(actual.len() == spans.len(), "complete source span census")?;
    for (row, (role, index, first, last)) in actual.iter().zip(spans) {
        let path = paths[index].0;
        let body = texts[path]
            .lines()
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        ensure(
            row["role"] == role
                && row["path"] == path
                && row["first"] == first
                && row["last"] == last
                && row["body"] == body
                && row["sha256"] == supplier::hash(body.as_bytes()),
            "complete original constructor body",
        )?;
    }
    ensure(
        bodies["scope"]
            == json!({"finite_selected_recipes_only":true,"generic_parser_or_numeric_parity":false,
        "store_delivery_covered":false,"native_coverage_authorized":false}),
        "body proof scope",
    )
}

/// Encoding conversion only: neither filters nor evaluates modifiers. Numeric
/// keys must be the exact contiguous Lua array shape retained by this witness.
pub fn convert_plain(v: &Value) -> Result<Value, String> {
    match v {
        Value::Object(o) => {
            ensure(
                !o.contains_key("_positions")
                    && !matches!(
                        o.get("kind").and_then(Value::as_str),
                        Some("callback_reference" | "table_reference")
                    ),
                "unresolved alias/callback or mixed encoding",
            )?;
            let mut fields = Map::new();
            let mut positions = BTreeMap::new();
            for (key, value) in o {
                if key.starts_with('[') {
                    let index = key
                        .strip_prefix('[')
                        .and_then(|s| s.strip_suffix(']'))
                        .and_then(|s| s.parse::<usize>().ok())
                        .ok_or("invalid numeric source key")?;
                    ensure(
                        key == &format!("[{index}]") && index > 0,
                        "noncanonical numeric source key",
                    )?;
                    positions.insert(index, convert_plain(value)?);
                } else {
                    fields.insert(key.clone(), convert_plain(value)?);
                }
            }
            if !positions.is_empty() {
                ensure(
                    positions.keys().copied().eq(1..=positions.len()),
                    "noncontiguous numeric source keys",
                )?;
                fields.insert(
                    "_positions".into(),
                    json!(
                        positions
                            .into_iter()
                            .map(|(i, v)| json!({"index":i,"value":v}))
                            .collect::<Vec<_>>()
                    ),
                );
            }
            Ok(Value::Object(fields))
        }
        Value::Array(a) => Ok(Value::Array(
            a.iter().map(convert_plain).collect::<Result<_, _>>()?,
        )),
        _ => Ok(v.clone()),
    }
}
fn graph(v: &Value) -> Result<&Value, String> {
    ensure(
        v["aliases"] == json!([]) && v["callbacks"] == json!([]),
        "recipe aliases/callbacks remain unresolved",
    )?;
    supplier::check_neutral_body_inventory(v)?;
    v.get("value")
        .ok_or_else(|| "missing explicit recipe value".into())
}
pub fn recipe_records(v: &Value, base: bool) -> Result<Value, String> {
    let source = graph(v)?;
    ensure(!source.is_null(), "base/active list was not observed")?;
    let mut converted = convert_plain(source)?;
    let fields = converted.as_object_mut().ok_or("recipe list object")?;
    let positions = fields.remove("_positions").unwrap_or_else(|| json!([]));
    if base {
        ensure(
            *fields
                == json!({"actor":{},"conditions":{},"multipliers":{},"parent":false})
                    .as_object()
                    .unwrap()
                    .clone(),
            "base-list inherited state",
        )?;
    } else {
        ensure(fields.is_empty(), "active list has unreviewed metadata")?;
    }
    let rows = array(&positions, "record positions")?
        .iter()
        .map(|r| json!({"position":r["index"],"body":r["value"]}))
        .collect::<Vec<_>>();
    Ok(if rows.is_empty() {
        json!({})
    } else {
        json!(rows)
    })
}
pub fn review_item(item: &Value) -> Result<Value, String> {
    supplier::check_neutral_body_inventory(item)?;
    let id = item["id"].as_str().ok_or("source item ID")?;
    let number = id.parse::<u64>().map_err(|e| e.to_string())?;
    ensure(
        item["loaded_metadata"]["id"] == number && item["loaded_metadata"]["jewelSocketCount"] == 0,
        "loaded item identity/socket count",
    )?;
    for (key, value) in item["loaded_metadata"].as_object().ok_or("item metadata")? {
        if key != "id" && key != "jewelSocketCount" {
            ensure(value.is_null(), "unreviewed loaded item variant")?;
        }
    }
    ensure(
        item["source"]["name"] == "Item"
            && item["source"]["attributes"] == json!({"id":id})
            && item["source"]["text"] == item["raw"]
            && item["original_parsed_xml"]["attrib"] == item["source"]["attributes"],
        "full original XML metadata",
    )?;
    let mut lines = Vec::new();
    for family in [
        "buffModLines",
        "enchantModLines",
        "implicitModLines",
        "explicitModLines",
    ] {
        let plain = graph(&item["parsed_lines"][family])?;
        let encoded = convert_plain(plain)?;
        ensure(
            encoded
                .as_object()
                .is_some_and(|o| o.keys().all(|k| k == "_positions")),
            "parsed line list shape",
        )?;
        if let Some(values) = encoded.get("_positions") {
            for row in array(values, "parsed line positions")? {
                let line = &row["value"];
                for field in [
                    "extra",
                    "disabled",
                    "bonded",
                    "variantList",
                    "versionList",
                    "variantGroupList",
                ] {
                    ensure(line.get(field).is_none(), "unreviewed parsed line branch")?;
                }
                lines.push(line.clone());
            }
        }
    }
    ensure(
        graph(&item["parsed_lines"]["runeModLines"])? == &json!({}),
        "rune source remains outside selected proof",
    )?;
    let ranges = array(&item["source"]["children"], "XML ModRange children")?;
    ensure(
        ranges.len() == lines.len(),
        "every parsed line retains XML range",
    )?;
    for (index, (range, line)) in ranges.iter().zip(&lines).enumerate() {
        ensure(
            range["name"] == "ModRange"
                && range["attributes"] == json!({"id":(index+1).to_string(),"range":"0.5"})
                && range["children"] == json!([])
                && range["content"] == json!([])
                && range["text"] == ""
                && line["range"] == 0.5,
            "original range/line correspondence",
        )?;
    }
    let base = recipe_records(&item["base_records"], true)?;
    let buff = if graph(&item["buff_records"])?.is_null() {
        json!({})
    } else {
        recipe_records(&item["buff_records"], false)?
    };
    ensure(
        array(&buff, "buff records")?.is_empty(),
        "selected buff recipes changed",
    )?;
    let mut uses = Vec::new();
    for use_ in array(&item["uses"], "item uses")? {
        let slot = use_["slot"].as_str().ok_or("source slot")?;
        let number = match slot {
            "Ring 1" | "Weapon 1" => json!(1),
            "Ring 2" => json!(2),
            "Helmet" | "Body Armour" | "Boots" | "Gloves" | "Amulet" | "Belt" => Value::Null,
            _ => return Err("unreviewed selected slot".into()),
        };
        ensure(
            use_["slot_number"] == number
                && use_["source"]["name"] == "Slot"
                && use_["source"]["attributes"]["name"] == slot
                && use_["source"]["attributes"]["itemId"] == id,
            "slot-number/source selection",
        )?;
        let ordinary = recipe_records(&use_["records"], false)?;
        let bonded = recipe_records(&use_["bonded_records"], false)?;
        ensure(
            use_["bonded_lists_match"] == true && ordinary == bonded,
            "both original bonded lists must agree",
        )?;
        uses.push(json!({"slot":slot,"slot_specific":!number.is_null(),"records":ordinary}));
    }
    ensure(!uses.is_empty(), "selected item has no use")?;
    Ok(json!({"id":id,"base":base,"buff":buff,"uses":uses,"parsed_lines":lines.len()}))
}

type RecipeIndex = BTreeMap<String, Value>;
fn recipes(f: &Value) -> Result<(RecipeIndex, RecipeIndex), String> {
    let items = array(&f["recipe"]["items"], "eight item recipes")?;
    ensure(
        items.len() == 8 && f["recipe"]["selected"]["items"] == "2",
        "selected item record census",
    )?;
    let mut by_id = BTreeMap::new();
    let mut by_slot = BTreeMap::new();
    for item in items {
        let reviewed = review_item(item)?;
        let id = item["id"].as_str().unwrap();
        ensure(
            by_id.insert(id.to_owned(), reviewed.clone()).is_none(),
            "duplicate item record",
        )?;
        for use_ in array(&reviewed["uses"], "reviewed uses")? {
            let slot = use_["slot"].as_str().unwrap();
            ensure(
                by_slot
                    .insert(slot.to_owned(), json!({"item":item,"use":use_}))
                    .is_none(),
                "duplicate selected slot",
            )?;
        }
    }
    ensure(by_slot.len() == 9, "nine distinct equipped uses")?;
    let memberships = array(&f["memberships"], "owned equipment")?;
    ensure(memberships.len() == 9, "nine owned equipment occurrences")?;
    let mut slots = BTreeSet::new();
    let mut occurrences = BTreeSet::new();
    let mut items_owned = BTreeMap::new();
    for m in memberships {
        let slot = m["slot"].as_str().ok_or("owned slot")?;
        ensure(slots.insert(slot), "duplicate owned slot")?;
        let entry = by_slot.get(slot).ok_or("unmatched owned slot")?;
        let item = &entry["item"];
        let use_ = one(
            array(&item["uses"], "item uses")?
                .iter()
                .filter(|u| u["slot"] == slot),
            "unique source use",
        )?;
        ensure(
            m["source_item_id"] == item["id"]
                && m["recipe_sha256"] == digest(item)
                && m["use_recipe_sha256"] == digest(use_),
            "exact recipe/use binding",
        )?;
        ensure(
            m["source_item"]["fragment_sha256"]
                == supplier::hash(item["raw_xml"].as_str().ok_or("raw XML")?.as_bytes()),
            "exact saved item XML",
        )?;
        ensure(
            m["preset"] == f["selection"]["build"]["equipment"]
                && m["owned_use"]["item"]["value"] == m["owned_item"]["id"]
                && m["source_item"]["origin"]["links"]
                    .as_array()
                    .is_some_and(|links| {
                        links.contains(&json!({"kind":"item","value":m["owned_item"]["id"]}))
                    })
                && m["source_use"]["origin"]["links"]
                    .as_array()
                    .is_some_and(|links| {
                        links.contains(&json!({"kind":"equipment","value":m["owned_use"]["id"]}))
                    }),
            "exact owned item/use/source authority",
        )?;
        ensure(
            occurrences.insert(digest(&m["owned_use"]["id"])),
            "duplicated owned equipment occurrence",
        )?;
        if let Some(prior) = items_owned.insert(
            m["source_item_id"].as_str().unwrap(),
            m["owned_item"]["id"].clone(),
        ) {
            ensure(
                prior == m["owned_item"]["id"],
                "repeated item record changed ownership",
            )?;
        }
    }
    ensure(items_owned.len() == 8, "eight owned item records")?;
    Ok((by_id, by_slot))
}

pub fn review_environment(e: &Value, f: &Value) -> Result<Value, String> {
    let (by_id, by_slot) = recipes(f)?;
    let rows = array(&e["items"], "constructed equipment inventory")?;
    ensure(rows.len() == 9, "constructed nine-use census")?;
    let mut seen = BTreeSet::new();
    let mut direct = 0;
    let mut unavailable = Vec::new();
    for row in rows {
        let slot = row["key"].as_str().ok_or("constructed slot")?;
        ensure(seen.insert(slot), "duplicate constructed slot")?;
        let entry = by_slot.get(slot).ok_or("unmatched constructed slot")?;
        let item = &entry["item"];
        let reviewed = &by_id[item["id"].as_str().unwrap()];
        let actual = &row["value"];
        supplier::check_neutral_body_inventory(actual)?;
        let id = item["id"].as_str().unwrap().parse::<u64>().unwrap();
        ensure(
            actual["fields"]["id"] == id
                && actual["fields"]["raw"] == item["raw"]
                && actual["fields"]["name"] == item["name"]
                && actual["fields"]["baseName"] == item["base_name"],
            "constructed item identity and full raw input",
        )?;
        for (key, value) in item["loaded_metadata"].as_object().unwrap() {
            ensure(
                actual["fields"].get(key).unwrap_or(&Value::Null) == value,
                "constructed metadata/variant changed",
            )?;
        }
        ensure(
            actual["base_records"] == reviewed["base"]
                && actual["buff_records"] == reviewed["buff"],
            "complete ordered base/buff body join",
        )?;
        if entry["use"]["slot_specific"] == true {
            ensure(
                actual["records"] == json!({}),
                "source02 slot-specific field is not an observed active list",
            )?;
            unavailable.push(json!({"slot":slot,"source_item":id,"reason":"item.modList absent; slot-specific list not captured by source02"}));
        } else {
            ensure(
                actual["records"] == entry["use"]["records"],
                "complete ordered active list join",
            )?;
            direct += 1;
        }
        // The authenticated observer calls raw(item.jewelData): Lua nil removes
        // the field. Do not substitute a JSON null or claim arbitrary absent
        // observations establish empty providers.
        ensure(
            actual.as_object().is_some_and(|o| {
                o.keys().map(String::as_str).eq([
                    "base_records",
                    "buff_records",
                    "fields",
                    "records",
                    "rune_fields",
                    "socketed_items",
                ])
            }) && actual["fields"]["jewelSocketCount"] == 0,
            "selected item observer fields/provider changed",
        )?;
        let sockets = array(
            actual["socketed_items"]
                .get("_positions")
                .unwrap_or(&json!({})),
            "socket positions",
        )?
        .to_vec();
        let runes = array(
            actual["rune_fields"]
                .get("_positions")
                .unwrap_or(&json!({})),
            "rune positions",
        )?
        .to_vec();
        let count = actual["fields"]["itemSocketCount"]
            .as_u64()
            .ok_or("socket count")? as usize;
        ensure(
            sockets.len() == count && runes.len() == count,
            "socket/rune source multiplicity",
        )?;
        let positions_only = |rows: &[Value]| {
            if rows.is_empty() {
                json!({})
            } else {
                json!({"_positions":rows})
            }
        };
        ensure(
            actual["socketed_items"] == positions_only(&sockets)
                && actual["rune_fields"] == positions_only(&runes),
            "unexpected socket/rune source fields",
        )?;
        for (index, (socket, rune)) in sockets.iter().zip(runes).enumerate() {
            ensure(
                *socket == json!({"index":index+1,"value":{"group":index+1}})
                    && rune == json!({"index":index+1,"value":"None"}),
                "occupied or unknown socket provider",
            )?;
        }
    }
    ensure(
        direct == 6 && unavailable.len() == 3,
        "direct versus slot-specific view census",
    )?;
    Ok(
        json!({"direct_lists":direct,"slot_specific_unobserved":unavailable,"item_inventory_sha256":digest(&e["items"])}),
    )
}

pub fn review_radius(trace: &Value, f: &Value) -> Result<Value, String> {
    let (_, by_slot) = recipes(f)?;
    let calls = array(&trace["init_calls"], "radius init calls")?;
    let candidates = array(&trace["slot_candidates"], "radius selected candidates")?;
    ensure(
        calls.len() == 21 && candidates.len() == 2352,
        "actual selection census",
    )?;
    let mut total = 0;
    for (index, call) in calls.iter().enumerate() {
        ensure(
            call["invocation_id"] == index + 1,
            "actual selection invocation",
        )?;
        let rows = candidates
            .iter()
            .filter(|r| r["invocation_id"] == call["invocation_id"])
            .collect::<Vec<_>>();
        ensure(rows.len() == 112, "complete selection candidates")?;
        let mut seen = BTreeSet::new();
        let mut occupied = 0;
        for row in rows {
            let slot = row["slot_name"].as_str().ok_or("candidate slot")?;
            ensure(seen.insert(slot), "duplicate actual slot selection")?;
            let item = &row["resolved_item"];
            if item["present"] == true {
                let entry = by_slot.get(slot).ok_or("unknown occupied candidate")?;
                let expected = &entry["item"];
                let id = expected["id"].as_str().unwrap().parse::<u64>().unwrap();
                ensure(
                    item["fields"]["id"] == id
                        && item["fields"]["raw"] == expected["raw"]
                        && item["fields"]["name"] == expected["name"]
                        && item["fields"]["baseName"] == expected["base_name"]
                        && row["slot_fields"]["selItemId"] == id,
                    "actual candidate identity/raw/selection",
                )?;
                occupied += 1;
            } else {
                ensure(
                    item == &json!({"present":false}) && !by_slot.contains_key(slot),
                    "selected source unexpectedly absent",
                )?;
            }
        }
        ensure(occupied == 9, "actual nine selected uses")?;
        total += occupied;
    }
    Ok(json!({"invocations":21,"selected_uses":total,"candidate_census":2352}))
}

pub fn reconstruct(f: &Value, bodies: &Value) -> Result<Value, String> {
    review_bodies(bodies)?;
    ensure(
        f["recipe"]["original_item_load_source"]
            == json!({"path":"src/Classes/ItemsTab.lua",
        "line":1199,"end_line":1255,"sha256":bodies["spans"][0]["sha256"],"whole_file_sha256":bodies["files"][0]["sha256"]})
            && f["recipe"]["original_xml_parser"]
                == json!({"path":"runtime/lua/xml.lua",
        "sha256":"832b6bb31f1f0e79e3ed6346113691b36ace29ed964e332530eef8b3b63af0e2"}),
        "original Item branch and XML parser authority",
    )?;
    let (by_id, by_slot) = recipes(f)?;
    let mut summaries = Vec::new();
    for mode in ["off", "on"] {
        let envs = array(&f[mode], "source02 environments")?;
        ensure(envs.len() == 21, "source02 full environment census")?;
        let mut modes = BTreeMap::new();
        let mut views = Vec::new();
        for env in envs {
            *modes
                .entry(env["mode"].as_str().ok_or("source mode")?)
                .or_insert(0) += 1;
            views.push(review_environment(env, f)?);
        }
        ensure(
            modes == BTreeMap::from([("MAIN", 16), ("CALCS", 1), ("CALCULATOR", 4)]),
            "source02 modes",
        )?;
        ensure(
            views.windows(2).all(|w| w[0] == w[1]),
            "constructed item inventory differs by source pass",
        )?;
        summaries.push(json!({"environments":21,"direct_list_joins":126,"base_and_buff_joins":189,"inventory":views[0]}));
    }
    ensure(
        summaries[0] == summaries[1],
        "independent source02 item records differ",
    )?;
    let traces = array(&f["radius"], "independent radius observations")?;
    ensure(traces.len() == 2, "two radius observations")?;
    let selections = traces
        .iter()
        .map(|t| review_radius(t, f))
        .collect::<Result<Vec<_>, _>>()?;
    ensure(
        selections[0] == selections[1],
        "independent radius selection mismatch",
    )?;
    // Semantic checks above are independently testable. This final seal prevents
    // coherent reauthoring from certifying itself, including unused source fields.
    ensure(
        f == fixture(),
        "independently authenticated recipe, membership and acquisition inputs",
    )?;
    let bindings=array(&f["memberships"],"memberships")?.iter().map(|m|json!({"slot":m["slot"],
        "source_item_id":m["source_item_id"],"source_item":m["source_item"]["source"],"source_use":m["source_use"]["source"],
        "item":m["owned_item"]["id"],"use":m["owned_use"]["id"],"preset":m["preset"],"recipe_sha256":m["recipe_sha256"],"use_recipe_sha256":m["use_recipe_sha256"]})).collect::<Vec<_>>();
    let generated=by_slot.iter().flat_map(|(slot,e)|{
        array(&e["use"]["records"],"active records").unwrap().iter().filter(|r|r["body"]["name"]=="ExtraSkill")
            .map(move |r|json!({"slot":slot,"source_item":e["item"]["id"],"record":r["body"],"descendant_delivery_covered":false}))
    }).collect::<Vec<_>>();
    Ok(
        json!({"schema_version":1,"kind":"selected-equipment-construction-review","source_revision":REVISION,"source_manifest":MANIFEST,
        "authority":{"recipe_value_sha256":RECIPE_VALUE,"source02_comparison_sha256":"be695bc423aeb637b6d5eb8343246cf6c9879eb4549f82cc7c8ba4b046ab8155",
        "radius_report_sha256":"8784718a733d10878da117724b3dff69e97d380c0140bfbab1cc02ffbf11a2da","historical_publication":f["historical_publication"],
        "source_bodies_sha256":digest(bodies),"fixture_sha256":digest(f)},
        "observed":{"item_records":by_id.len(),"equipment_uses":by_slot.len(),"bonded_active_recipe_lists":18,
        "parsed_lines":by_id.values().map(|r|r["parsed_lines"].as_u64().unwrap()).sum::<u64>(),"per_source02_report":summaries[0],"per_radius_observation":selections[0]},
        "bindings":bindings,"generated_skill_boundaries":generated,"mana_adjustment_names":supplier::relevant_names(),
        "scope":{"finite_original_item_construction_reconciled":true,"source02_slot_specific_active_lists_observed":false,
        "item_to_store_delivery_covered":false,"focus_amulet_copy_replacement_covered":false,"inherited_store_covered":false,
        "generic_parsing_or_numeric_order_covered":false,"generated_support_delivery_covered":false,
        "native_coverage_authorized":false,"native_partial_owners_closed":0,"current_native_package_changed":false}}),
    )
}

pub fn check() -> Result<(), String> {
    let expected = reconstruct(fixture(), &read("equipment-bodies.json"))?;
    ensure(
        read("equipment-review.json") == expected,
        "retained equipment review mismatch",
    )
}
pub fn retain() {
    let result = reconstruct(fixture(), &read("equipment-bodies.json")).unwrap();
    fs::write(
        supplier::root()
            .join(supplier::DIR)
            .join("equipment-review.json"),
        serde_json::to_string_pretty(&result).unwrap() + "\n",
    )
    .unwrap();
}
