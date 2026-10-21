//! Finite offline constructor audit. No Lua behavior is added to native runtime.
#[allow(dead_code)]
#[path = "owned_configuration_accounting.rs"]
mod accounting;
#[allow(dead_code)]
#[path = "owned_mana_supplier_sources.rs"]
mod suppliers;

use serde_json::{Map, Value, json};
use std::{collections::BTreeSet, fs, io::Read, sync::OnceLock};

pub const DIR: &str = suppliers::DIR;
pub fn read(name: &str) -> Value {
    suppliers::read(format!("{DIR}/{name}"))
}
fn require(ok: bool, why: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(why.to_owned()) }
}
fn hash(v: &Value) -> String {
    suppliers::hash(&serde_json::to_vec(v).unwrap())
}
fn arr(v: &Value) -> Result<&[Value], String> {
    if v.as_object().is_some_and(Map::is_empty) {
        Ok(&[])
    } else {
        v.as_array()
            .map(Vec::as_slice)
            .ok_or_else(|| "expected complete source inventory".to_owned())
    }
}

pub fn fixture() -> Result<Value, String> {
    let a = suppliers::acquisition_fixture()?;
    let project = |mut report: Value| json!({"configuration":report["state"]["configuration"].take(),"catalogue":report["state"]["configuration_catalogue"].take(),"calls":report["state"]["callback_invocations"].take()});
    Ok(json!({"off":project(a.jit_off),"on":project(a.jit_on)}))
}

pub fn bodies() -> Value {
    read("configuration-bodies.json")
}

fn authenticate_auxiliary() -> Result<Value, String> {
    let auxiliary = read("configuration-auxiliary-bodies.json");
    let manifest = suppliers::read("crates/poe-optimizer-pob/data/pob-source-manifest.json");
    require(
        auxiliary["source_revision"] == manifest["upstream_revision"],
        "auxiliary source revision changed",
    )?;
    let sources = arr(&auxiliary["sources"])?;
    require(sources.len() == 2, "auxiliary method source census changed")?;
    for source in sources {
        let body = source["source"]
            .as_str()
            .ok_or("missing complete auxiliary source")?;
        require(
            source["sha256"] == suppliers::hash(body.as_bytes()) && source["bytes"] == body.len(),
            "auxiliary body pin changed",
        )?;
        require(
            arr(&manifest["files"])?.iter().any(|p| {
                p["path"] == source["path"]
                    && p["sha256"] == source["sha256"]
                    && p["bytes"] == source["bytes"]
            }),
            "auxiliary body is not original pinned source",
        )?;
        let lines: Vec<_> = body.lines().collect();
        for span in arr(&source["spans"])? {
            let first = span["first"].as_u64().ok_or("missing auxiliary span")? as usize;
            let last = span["last"].as_u64().ok_or("missing auxiliary span")? as usize;
            require(
                first > 0 && first <= last && last <= lines.len(),
                "invalid auxiliary span",
            )?;
            let text = lines[first - 1..last].join("\n");
            require(
                span["body"] == text && span["sha256"] == suppliers::hash(text.as_bytes()),
                "auxiliary method excerpt changed",
            )?;
        }
    }
    Ok(auxiliary)
}

fn authenticate_bodies(bodies: &Value) -> Result<(), String> {
    static AUTHENTICATED: OnceLock<()> = OnceLock::new();
    AUTHENTICATED.get_or_init(accounting::check_authored);
    authenticate_auxiliary()?;
    let manifest = suppliers::read("crates/poe-optimizer-pob/data/pob-source-manifest.json");
    for field in ["source", "constructor_source"] {
        let expected = if field == "source" {
            "src/Modules/ConfigOptions.lua"
        } else {
            "src/Classes/ConfigTab.lua"
        };
        require(
            bodies[field]["path"] == expected
                && bodies[field]["archive"]
                    == format!(
                        "data/owned/poe2/3887ae68/configuration-accounting/evidence/pob/{expected}"
                    )
                && arr(&manifest["files"])?
                    .iter()
                    .any(|pin| pin["path"] == expected && pin["sha256"] == bodies[field]["sha256"]),
            "configuration body archive is not the authenticated original source",
        )?;
        let path = bodies[field]["archive"]
            .as_str()
            .ok_or("missing original body archive")?;
        let source = fs::read_to_string(suppliers::root().join(path))
            .map_err(|e| e.to_string())?
            .replace("\r\n", "\n");
        require(
            bodies[field]["sha256"] == suppliers::hash(source.as_bytes()),
            "configuration source pin changed",
        )?;
        let family = if field == "source" {
            "bodies"
        } else {
            "constructors"
        };
        let lines: Vec<_> = source.lines().collect();
        for body in arr(&bodies[family])? {
            let first = body["first"].as_u64().ok_or("missing body span")? as usize;
            let last = body["last"].as_u64().ok_or("missing body span")? as usize;
            require(
                first > 0 && first <= last && last <= lines.len(),
                "invalid reviewed body span",
            )?;
            let text = lines[first - 1..last].join("\n");
            require(
                body["body"] == text && body["sha256"] == suppliers::hash(text.as_bytes()),
                "configuration callback body changed",
            )?;
            if family == "bodies" {
                let mut names = Vec::new();
                // Checks the declarations in manually reviewed exact source;
                // it is not a parser/interpreter for arbitrary Lua programs.
                for part in text.split(":NewMod(\"").skip(1) {
                    names.push(
                        part.split_once('"')
                            .ok_or("writer declaration")?
                            .0
                            .to_owned(),
                    );
                }
                for part in text.split("modLib.createMod(\"").skip(1) {
                    names.push(
                        part.split_once('"')
                            .ok_or("nested writer declaration")?
                            .0
                            .to_owned(),
                    );
                }
                let mut expected: Vec<_> = arr(&body["declared_names"])?
                    .iter()
                    .map(|v| v.as_str().unwrap().to_owned())
                    .collect();
                names.sort();
                expected.sort();
                require(
                    names == expected,
                    "configuration writer declarations changed",
                )?;
                require(
                    names
                        .iter()
                        .all(|n| !suppliers::relevant_names().contains(n)),
                    "configuration body declares Mana adjustment supplier",
                )?;
            }
        }
    }
    require(
        arr(&bodies["bodies"])?.len() == 48,
        "selected callback body census changed",
    )?;
    for (classification, count) in [
        ("literal-writer-domain", 45),
        ("inactive-dynamic-boss-branch", 1),
        ("quest-parser-output-boundary", 2),
    ] {
        require(
            arr(&bodies["bodies"])?
                .iter()
                .filter(|b| b["classification"] == classification)
                .count()
                == count,
            "reviewed callback domain census changed",
        )?;
    }
    require(
        bodies["scope"]["native_coverage_authorized"] == false
            && bodies["scope"]["quest_parser_closure"] == false,
        "configuration proof promoted coverage",
    )
}

// This is the source constructor's finite primitive gate, confined to offline
// evidence. Unknown/compound values are refused; numeric zero is never false.
fn truthy(v: &Value) -> Result<bool, String> {
    match v {
        Value::Null => Ok(false),
        Value::Bool(b) => Ok(*b),
        Value::Number(_) | Value::String(_) => Ok(true),
        _ => Err("unreviewed source gate value".into()),
    }
}
pub fn dispatch_argument(row: &Value, state: &Value) -> Result<Option<Value>, String> {
    if row["apply"].is_null() {
        return Ok(None);
    }
    let name = row["fields"]["var"]
        .as_str()
        .ok_or("callback lacks exact variable identity")?;
    let kind = row["fields"]["type"]
        .as_str()
        .ok_or("callback lacks exact dispatch kind")?;
    let input = &state["input"][name];
    let placeholder = &state["placeholder"][name];
    match kind {
        "check" => Ok(truthy(input)?.then_some(json!(true))),
        "list" | "text" => Ok(truthy(input)?.then(|| input.clone())),
        "count" | "integer" | "countAllowZero" | "float" => {
            for v in [input, placeholder] {
                if truthy(v)? && (kind != "count" || v.as_f64() != Some(0.0)) {
                    return Ok(Some(v.clone()));
                }
            }
            Ok(None)
        }
        _ => Err(format!("unreviewed callback dispatch kind {kind}")),
    }
}

fn defaults(catalogue: &[Value]) -> Result<Value, String> {
    let mut input = Map::new();
    let mut placeholder = Map::new();
    for row in catalogue {
        let Some(name) = row["fields"]["var"].as_str() else {
            continue;
        };
        if !row["default_state"].is_null() {
            input.insert(name.to_owned(), row["default_state"].clone());
        }
        if !row["default_placeholder"].is_null() {
            placeholder.insert(name.to_owned(), row["default_placeholder"].clone());
        }
        if let Some(index) = row["fields"]["defaultIndex"].as_u64() {
            let values: Vec<_> = arr(&row["list"]["_positions"])?
                .iter()
                .filter(|v| v["index"] == index)
                .collect();
            require(
                values.len() == 1,
                "default option index is missing or ambiguous",
            )?;
            input.insert(name.to_owned(), values[0]["value"]["val"].clone());
        }
    }
    Ok(json!({"input":input,"placeholder":placeholder}))
}

fn append_delta(before: &Value, after: &Value) -> Result<Vec<Value>, String> {
    let before = arr(before)?;
    let after = arr(after)?;
    require(
        after.len() >= before.len() && after[..before.len()] == *before,
        "callback rewrites or removes prior modifier records",
    )?;
    Ok(after[before.len()..].to_vec())
}
fn record_names(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(o) => {
            if let (Some(name), Some(_)) = (o.get("name").and_then(Value::as_str), o.get("type")) {
                out.push(name.to_owned());
            }
            for v in o.values() {
                record_names(v, out)
            }
        }
        Value::Array(a) => {
            for v in a {
                record_names(v, out)
            }
        }
        _ => {}
    }
}

fn empty_custom(state: &Value) -> Result<(), String> {
    require(
        state["input"]["customMods"].is_null() || state["input"]["customMods"] == "",
        "unreviewed custom fallback text",
    )?;
    let rows = arr(&state["custom_blocks"]["_positions"])?;
    require(
        rows.len() == 1
            && rows[0]["index"] == 1
            && rows[0]["value"] == json!({"title":"Default","enabled":true,"text":""}),
        "custom modifier block membership or text changed",
    )
}

// Independent immutable acquisition/publication anchors. Editable summary hashes
// cannot authenticate a coherently changed source matrix or native owner.
const REWARD_SOURCE: &str = "4fa239bb7f72b936b522a84fb6adb0139cdddb2a4b593ce69a64bb722f7a2e9d";
const REWARD_RELEASE: &str = "ea5e38460e1dcb8df912a4dcddfd31d3e0be89dca2f5ede339b1a7173a1f34f2";
const REWARD_REPORT: &str = "6f86a5bcbde72f45b528cf309602536b96ee6045049380721623ad5bce0e1958";

fn reward_archive(metadata: &Value, role: &str, limit: usize) -> Result<Vec<u8>, String> {
    let pin = &metadata["files"][role];
    let file = format!("configuration-reward-{role}.gz");
    require(pin["file"] == file, "reward archive role changed")?;
    let path = suppliers::root().join(DIR).join(file);
    require(
        fs::metadata(&path).map_err(|e| e.to_string())?.len() <= limit as u64,
        "reward compressed archive exceeds bound",
    )?;
    let compressed = fs::read(path).map_err(|e| e.to_string())?;
    require(
        compressed.len() <= limit
            && pin["compressed_bytes"] == compressed.len()
            && pin["compressed_sha256"] == suppliers::hash(&compressed),
        "reward compressed archive changed",
    )?;
    let mut raw = Vec::new();
    flate2::read::GzDecoder::new(compressed.as_slice())
        .take(limit as u64 + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    require(
        raw.len() <= limit && pin["bytes"] == raw.len() && pin["sha256"] == suppliers::hash(&raw),
        "reward expanded archive changed or exceeds bound",
    )?;
    Ok(raw)
}
fn unique<'a>(
    rows: &'a [Value],
    test: impl Fn(&Value) -> bool,
    why: &str,
) -> Result<&'a Value, String> {
    let mut found = rows.iter().filter(|row| test(row));
    let row = found.next().ok_or_else(|| why.to_owned())?;
    require(found.next().is_none(), why)?;
    Ok(row)
}
fn decode(raw: &[u8]) -> Result<Value, String> {
    serde_json::from_slice(raw).map_err(|e| e.to_string())
}

fn load_reward_fixture() -> Result<Value, String> {
    use poe_optimizer_core::owned_draft::{DraftLimits, decode_draft};
    let metadata = read("configuration-reward-authority.json");
    require(
        metadata["scope"]
            == json!({"finite_callback_values":39,"controls":17,
        "actual_calls_per_observation":34,"native_coverage_authorized":false,
        "global_quest_parser_closure":false,"default_records":9,"saved_records":22,
        "selected_native_rewards":17}),
        "reward authority scope changed",
    )?;
    let source_raw = reward_archive(&metadata, "source", 8 * 1024 * 1024)?;
    let release_raw = reward_archive(&metadata, "release", 128 * 1024)?;
    let report_raw = reward_archive(&metadata, "report", 8 * 1024)?;
    require(
        suppliers::hash(&source_raw) == REWARD_SOURCE
            && suppliers::hash(&release_raw) == REWARD_RELEASE
            && suppliers::hash(&report_raw) == REWARD_REPORT,
        "independent reward acquisition/publication anchor changed",
    )?;
    let source = decode(&source_raw)?;
    let release = decode(&release_raw)?;
    let report = decode(&report_raw)?;
    require(
        release["input"] == metadata["current_input"],
        "reward endpoint changed",
    )?;
    let mut artifacts = Map::new();
    for (role, file, limit) in [
        ("rules", "rules.json", 24 * 1024 * 1024),
        ("mapping", "mapping.json", 8 * 1024 * 1024),
        ("policy", "rewards.json", 128 * 1024),
    ] {
        let raw = reward_archive(&metadata, role, limit)?;
        let pin = unique(
            arr(&release["artifacts"])?,
            |p| p["file"] == file,
            "release artifact missing or duplicated",
        )?;
        require(
            pin["sha256"] == suppliers::hash(&raw) && pin["bytes"] == raw.len(),
            "reward artifact is not the independently pinned publication",
        )?;
        artifacts.insert(role.to_owned(), decode(&raw)?);
    }
    let draft_raw = reward_archive(&metadata, "draft", 2 * 1024 * 1024)?;
    let typed = decode_draft(&draft_raw, DraftLimits::default()).map_err(|e| e.to_string())?;
    let digest = serde_json::to_value(
        typed
            .digest(DraftLimits::default().input.max_wire_bytes)
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let draft = decode(&draft_raw)?;
    require(
        digest == report["draft_digest"]
            && draft["draft"]["allocator"] == report["allocator_after"],
        "reward selected draft is not the authenticated normalization result",
    )?;
    let manifest = suppliers::read("crates/poe-optimizer-pob/data/pob-source-manifest.json");
    let quest = reward_archive(&metadata, "quest-data", 16 * 1024)?;
    let quest_pin = unique(
        arr(&manifest["files"])?,
        |p| p["path"] == "src/Data/QuestRewards.lua",
        "quest source pin missing",
    )?;
    require(
        quest_pin["sha256"] == suppliers::hash(&quest) && quest_pin["bytes"] == quest.len(),
        "quest metadata source changed",
    )?;
    for pin in arr(&source["evidence"]["files"])? {
        let p = unique(
            arr(&manifest["files"])?,
            |p| p["path"] == pin["path"],
            "reward source manifest entry missing",
        )?;
        require(
            p["sha256"] == pin["sha256"],
            "reward witness source revision changed",
        )?;
    }
    let case = &source["cases"][4];
    require(
        case["available"] == true
            && case["xml_sha256"] == report["source"]["sha256"]
            && case["state"]["original_methods_preserved"] == true
            && case["state"]["business_method_wrappers"] == false,
        "reward source case identity or original method authority changed",
    )?;
    let state = &case["state"];
    require(
        arr(&state["dynamic_census"])?.len() == 17 && arr(&state["matrix"])?.len() == 39,
        "reward finite source census changed",
    )?;
    let prior = suppliers::load();
    suppliers::reconcile_selected_inputs(&prior)?;
    require(
        report["source"]["sha256"] == prior.report["source"]["sha256"],
        "reward selection changed source build",
    )?;
    let mut prior_choice = prior.selection["build"]["choices"].clone();
    require(
        prior_choice["lineage"] == prior.draft["draft"]["allocator"]["lineage"],
        "historical choice selection lineage changed",
    )?;
    prior_choice["lineage"] = draft["draft"]["allocator"]["lineage"].clone();
    let sidecar_raw = reward_archive(&metadata, "sidecar", 2 * 1024 * 1024)?;
    require(
        report["sidecar_sha256"] == suppliers::hash(&sidecar_raw)
            && report["sidecar_bytes"] == sidecar_raw.len(),
        "reward sidecar differs from fixed normalization report",
    )?;
    let sidecar = decode(&sidecar_raw)?;
    let ordinal = &source["evidence"]["originals"][4]["config_source"];
    let origin = unique(
        arr(&sidecar["origins"])?,
        |o| {
            o["source"]["ordinal"] == *ordinal
                && o["source"]["source_sha256"] == report["source"]["sha256"]
        },
        "selected ConfigSet origin missing",
    )?;
    require(
        unique(
            arr(&origin["links"])?,
            |l| l["kind"] == "choice_preset",
            "selected ConfigSet choice link missing or duplicated",
        )?["value"]
            == prior_choice,
        "selected ConfigSet maps to different choice preset",
    )?;
    let choice = unique(
        arr(&draft["draft"]["choice_presets"]["members"])?,
        |r| r["id"] == prior_choice,
        "selected reward preset missing",
    )?;
    require(
        choice["rewards"]["completion"]["kind"] == "complete"
            && arr(&choice["rewards"]["members"])?.len() == 17,
        "selected reward membership is not complete",
    )?;
    let rewards = arr(&draft["draft"]["rewards"]["members"])?;
    let mut native = Vec::new();
    for id in arr(&choice["rewards"]["members"])? {
        let reward = unique(
            rewards,
            |r| r["id"] == *id,
            "selected reward occurrence missing or duplicated",
        )?;
        require(
            reward["definition"]["kind"] == "known"
                && reward["parameters"] == json!({"members":[],"completion":{"kind":"complete"}}),
            "reward parameters or definition unresolved",
        )?;
        let definition = &reward["definition"]["value"];
        let owner = unique(
            arr(&artifacts["rules"]["owners"])?,
            |o| o["owner"]["value"]["value"] == *definition,
            "selected reward owner missing or duplicated",
        )?;
        native.push(json!({"occurrence":reward,"owner":owner}));
    }
    // Preserve pending global rewards, choices, and all other normalization gaps.
    Ok(
        json!({"authority":metadata,"census":state["dynamic_census"],"matrix":state["matrix"],
        "saved_input":state["saved"]["input"],"source_selection":state["saved"]["selected"],"policy":artifacts["policy"],
        "mapping":arr(&artifacts["mapping"]["entries"])?.iter().filter(|r|r["source"]["kind"]=="configuration" && r["source"]["value"]["role"]=="reward").cloned().collect::<Vec<_>>(),
        "native":native,"selection":prior_choice,"selected_config_origin":origin,"selected_members":choice["rewards"],
        "global_reward_completion":draft["draft"]["rewards"]["completion"],
        "choice_completion":choice["choices"]["completion"],"draft_digest":digest}),
    )
}

pub fn reward_fixture() -> Result<&'static Value, String> {
    static REWARDS: OnceLock<Result<Value, String>> = OnceLock::new();
    REWARDS
        .get_or_init(load_reward_fixture)
        .as_ref()
        .map_err(Clone::clone)
}

fn reward_bindings() -> Result<Vec<Value>, String> {
    let mut result = Vec::new();
    for (packet, binding_pin, closure_pin) in [
        (
            "flat-resource-rewards",
            "09ea1ade25f5081996c8e158f1fb0ec28e9cafad2c5ff75461fa621a5c5b4c78",
            "4f3c4f0808867cbcca1ccb68fd599b4de337a1515d24ad95ae858f1b8b3bdcee",
        ),
        (
            "permanent-reward-effects",
            "f1cb476058326905ac9ca0585db3a08d23ce0f15e4b1c0a35d866b763d2455ed",
            "567ef0f5348994856bcac224e03eb9d5926f7438f940f1343a074ac695de7a4e",
        ),
        (
            "reward-recovery-inputs-v1",
            "2aecfd216f70e535c1e8bf9c0b53f3268c0295b9a1c2413049352c333107055e",
            "72178f47d52a8499a6e260ca6c6e9a565dc264627b2ad80b441730f0a49a81bb",
        ),
    ] {
        let bindings_raw = fs::read(
            suppliers::root().join(format!("data/owned/poe2/3887ae68/{packet}/bindings.json")),
        )
        .map_err(|e| e.to_string())?;
        let closure_raw = fs::read(
            suppliers::root().join(format!("data/owned/poe2/3887ae68/{packet}/closure.json")),
        )
        .map_err(|e| e.to_string())?;
        require(
            suppliers::hash(&bindings_raw) == binding_pin
                && suppliers::hash(&closure_raw) == closure_pin,
            "accepted reward effect packet changed",
        )?;
        let bindings = decode(&bindings_raw)?;
        let closure = decode(&closure_raw)?;
        for row in arr(&bindings["rewards"])? {
            let owner = unique(
                arr(&closure["owners"])?,
                |o| o["owner"]["value"]["value"] == row["reward"],
                "reward effect owner missing",
            )?;
            let effects = if row["effects"].is_null() {
                vec![
                    json!({"source_stat":row["source_stat"],"source_type":"BASE","source_tags":{},"amount":row["amount"]}),
                ]
            } else {
                arr(&row["effects"])?.to_vec()
            };
            let records: Vec<_> = effects.iter().map(|e| json!({"flags":0,"keyword_flags":0,
                "name":e["source_stat"],"source":row["source_origin"],"tags":e["source_tags"],"type":e["source_type"],"value":e["amount"]})).collect();
            result.push(json!({"key":row["source_option"],"value":row["source_value"],"source_index":row["source_index"],"records":records,"owner":owner}));
        }
    }
    let definition = |kind: &str, key: &str| json!({"kind":kind,"namespace":{"game":"poe2","version":"owned-mechanics-v1"},"key":format!("def.{key}")});
    for (id, key, index, amount, source) in [
        (
            "0000000000000027",
            "questAct 1ClearfellBeira",
            1,
            10,
            "Quest:Act 1: Clearfell",
        ),
        (
            "000000000000004d",
            "questAct 4Halls Of The DeadTasalio's Test",
            19,
            5,
            "Quest:Act 4: Halls Of The Dead",
        ),
    ] {
        let value = if amount == 10 {
            json!(true)
        } else {
            json!("+5% to Cold Resistance")
        };
        let owner = json!({"owner":{"kind":"definition","value":{"kind":"reward","value":definition("reward",id)}},
            "programs":{"closure":{"kind":"complete"},"members":[{"id":"resistance-contribution","context":"actor","reads":[],
                "nodes":[{"id":"amount-0","expression":{"kind":"literal","value":{"kind":"quantity","value":{"value":f64::from(amount),"unit":definition("unit","0000000000000002")}}}}],
                "effects":[{"id":"amount-0","when":null,"effect":{"kind":"contribute","entity":"player","stat":definition("stat","00000000000009d4"),"contribution":"add","value":"amount-0"}}]}]}});
        result.push(json!({"key":key,"value":value,"source_index":index,"owner":owner,"records":[{"flags":0,"keyword_flags":0,"name":"ColdResist","source":source,"tags":{},"type":"BASE","value":amount}]}));
    }
    Ok(result)
}

pub fn review_rewards(rewards: &Value) -> Result<Value, String> {
    let original = reward_fixture()?;
    let bindings = reward_bindings()?;
    let mut receipts = Vec::new();
    let mut native_ids = BTreeSet::new();
    for census in arr(&rewards["census"])? {
        let key = census["key"].as_str().ok_or("quest key missing")?;
        let selected = &rewards["saved_input"][key];
        let matrix = unique(
            arr(&rewards["matrix"])?,
            |m| m["key"] == key && m["value"] == *selected,
            "saved quest value outside finite invoked domain",
        )?;
        let binding = unique(
            &bindings,
            |b| b["key"] == key && b["value"] == *selected,
            "saved quest effect binding missing",
        )?;
        require(
            binding["source_index"] == census["source_index"]
                && binding["records"] == matrix["records"]
                && arr(&matrix["enemy_records"])?.is_empty(),
            "quest full source record/body/order differs from accepted effect binding",
        )?;
        let rule = unique(
            arr(&rewards["policy"]["rules"])?,
            |r| r["recipe"]["tiers"][0]["selectors"][0]["name"] == key,
            "quest policy rule missing",
        )?;
        let codec = &rule["recipe"]["codec"]["codec"];
        let token = selected
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| selected.to_string());
        let decoded = unique(
            arr(&codec["value"]["tokens"])?,
            |t| t["token"] == token,
            "quest selected codec token missing",
        )?;
        let outcome = unique(
            arr(&rule["outcomes"])?,
            |o| o["when"] == json!({"kind":codec["kind"],"value":decoded["value"]}),
            "quest selected policy outcome missing",
        )?;
        require(
            outcome["outcome"]["kind"] == "reward" && outcome["outcome"]["parameters"] == json!([]),
            "quest selected outcome is not parameter-free reward",
        )?;
        let mapping = unique(
            arr(&rewards["mapping"])?,
            |m| m["source"] == outcome["outcome"]["selector"],
            "quest exact selected mapping missing",
        )?;
        let definition = &binding["owner"]["owner"]["value"];
        require(
            mapping["outcome"]["kind"] == "mapped"
                && mapping["outcome"]["value"]["target"]
                    == json!({"kind":"definition","value":definition}),
            "quest mapping selects different reward",
        )?;
        let native = unique(
            arr(&rewards["native"])?,
            |n| n["occurrence"]["definition"]["value"] == definition["value"],
            "selected native reward missing or duplicated",
        )?;
        require(
            native["owner"] == binding["owner"],
            "selected native reward whole owner/recipient differs",
        )?;
        require(
            native_ids.insert(hash(&native["occurrence"]["id"])),
            "selected native reward used twice",
        )?;
        receipts.push(json!({"key":key,"value":selected,"source_index":census["source_index"],
            "source_records":matrix["records"],"occurrence":native["occurrence"],"owner_sha256":hash(&native["owner"]),
            "policy_sha256":hash(rule),"mapping_sha256":hash(mapping)}));
    }
    require(
        receipts.len() == 17 && native_ids.len() == 17 && arr(&rewards["native"])?.len() == 17,
        "selected reward occurrence census changed",
    )?;
    require(
        rewards == original,
        "reward reconciliation differs from independent source/publication authority",
    )?;
    Ok(
        json!({"controls":17,"finite_invoked_values":39,"selected_native_rewards":receipts,
        "selected_preset":rewards["selection"],"global_reward_completion":rewards["global_reward_completion"],
        "choice_completion":rewards["choice_completion"],"publication":rewards["authority"]["current_input"],
        "global_quest_parser_closure":false,"native_coverage_authorized":false}),
    )
}

/// Lossless translation between two authenticated observer encodings only.
/// No arbitrary Lua values, modifier parsing, or game evaluation is performed.
pub fn review_quest_call(call: &Value, phase: usize, rewards: &Value) -> Result<Value, String> {
    require(phase < 2, "unknown quest construction phase")?;
    let original = reward_fixture()?;
    require(
        rewards == original,
        "quest matrix/metadata is not independent acquisition authority",
    )?;
    require(
        call["before"]["active_set"] == rewards["source_selection"]["config"]
            && call["after"]["active_set"] == rewards["source_selection"]["config"],
        "quest callback belongs to a different ConfigSet",
    )?;
    let key = call["var"].as_str().ok_or("quest call lacks key")?;
    let census = unique(
        arr(&rewards["census"])?,
        |r| r["key"] == key,
        "quest key outside finite domain",
    )?;
    let argument = &call["argument_fields"]["val"];
    let matrix = unique(
        arr(&rewards["matrix"])?,
        |m| m["key"] == key && m["value"] == *argument,
        "quest argument outside finite invoked domain",
    )?;
    require(
        call["kind"] == census["widget"]
            && call["body"]["path"]
                == "@C:/code/poe-optimizer/vendor/path-of-building-poe2/src/Modules/ConfigOptions.lua"
            && call["body"]["first_line"] == census["callback"]["first"]
            && call["body"]["last_line"] == census["callback"]["last"],
        "quest callback identity changed",
    )?;
    let expected_argument = if phase == 0 {
        if census["widget"] == "check" {
            &census["default"]
        } else {
            &census["options"][0]
        }
    } else {
        &rewards["saved_input"][key]
    };
    require(
        argument == expected_argument,
        "quest default/saved phase selection changed",
    )?;
    let mut counts = [0usize; 2];
    for (i, (lane, source_lane)) in [
        ("player_records", "records"),
        ("enemy_records", "enemy_records"),
    ]
    .into_iter()
    .enumerate()
    {
        let delta = append_delta(&call["before"][lane], &call["after"][lane])?;
        let records = arr(&matrix[source_lane])?;
        require(
            delta.len() == records.len(),
            "quest callback record multiplicity changed",
        )?;
        let before = arr(&call["before"][lane])?.len();
        for (at, (actual, record)) in delta.iter().zip(records).enumerate() {
            require(
                record.as_object().is_some_and(|o| {
                    o.len() == 7
                        && [
                            "flags",
                            "keyword_flags",
                            "name",
                            "source",
                            "tags",
                            "type",
                            "value",
                        ]
                        .iter()
                        .all(|k| o.contains_key(*k))
                }),
                "unreviewed old quest record field",
            )?;
            let mut body = json!({"flags":record["flags"],"keywordFlags":record["keyword_flags"],"name":record["name"],"source":record["source"],"type":record["type"],"value":record["value"]});
            let tags = arr(&record["tags"])?;
            if !tags.is_empty() {
                body["_positions"] = json!(
                    tags.iter()
                        .enumerate()
                        .map(|(index, tag)| json!({"index":index+1,"value":tag}))
                        .collect::<Vec<_>>()
                );
            }
            require(
                *actual == json!({"position":before+at+1,"body":body}),
                "quest complete record, tag, position, or order changed",
            )?;
        }
        counts[i] = records.len();
    }
    Ok(
        json!({"key":key,"value":argument,"phase":phase,"source_index":census["source_index"],
        "player_records":counts[0],"enemy_records":counts[1],"finite_matrix_record_sha256":hash(matrix),
        "native_selection":"saved-phase-only","global_quest_parser_closure":false}),
    )
}

pub fn review_state(state: &Value, bodies: &Value) -> Result<Value, String> {
    authenticate_bodies(bodies)?;
    let rewards = reward_fixture()?;
    review_rewards(rewards)?;
    let catalogue = arr(&state["catalogue"])?;
    require(
        catalogue.len() == 663,
        "complete configuration catalogue missing rows",
    )?;
    for (i, row) in catalogue.iter().enumerate() {
        require(
            row["index"] == i + 1,
            "catalogue index or multiplicity changed",
        )?;
    }
    require(
        catalogue.iter().filter(|r| !r["apply"].is_null()).count() == 537,
        "callback declaration census changed",
    )?;
    let calls = arr(&state["calls"])?;
    require(calls.len() == 125, "actual callback census changed")?;
    let configurations = arr(&state["configuration"])?;
    require(
        configurations.len() == 2,
        "configuration construction census changed",
    )?;
    let constructed_defaults = defaults(catalogue)?;
    for field in ["input", "placeholder"] {
        require(
            constructed_defaults[field] == calls[0]["before"][field],
            "declared defaults differ from first actual construction",
        )?;
    }
    let mut at = 0usize;
    let mut phases = Vec::new();
    let mut variables = BTreeSet::new();
    for (phase, final_state) in configurations.iter().enumerate() {
        let mut current = calls.get(at).ok_or("missing construction callback")?["before"].clone();
        require(
            arr(&current["player_records"])?.is_empty()
                && arr(&current["enemy_records"])?.is_empty(),
            "construction must begin with new empty original lists",
        )?;
        let mut dispositions = Vec::new();
        let mut active = 0usize;
        let mut inactive = 0usize;
        let mut reconciled_quests = Vec::new();
        for row in catalogue {
            let argument = dispatch_argument(row, &current)?;
            if let Some(argument) = argument {
                let call = calls
                    .get(at)
                    .ok_or("eligible callback lacks actual invocation")?;
                require(
                    call["var"] == row["fields"]["var"]
                        && call["kind"] == row["fields"]["type"]
                        && call["body"] == row["apply"],
                    "callback occurrence differs from complete dispatch order",
                )?;
                require(
                    call["body"]["path"].as_str().is_some_and(|path| {
                        path.ends_with(
                            "/vendor/path-of-building-poe2/src/Modules/ConfigOptions.lua",
                        )
                    }),
                    "callback body belongs to an unreviewed source file",
                )?;
                require(
                    call["argument_fields"]["val"] == argument,
                    "callback actual argument differs from gate source",
                )?;
                require(
                    call["before"] == current,
                    "callback before/after chain has an unexplained write",
                )?;
                let matched: Vec<_> = arr(&bodies["bodies"])?
                    .iter()
                    .filter(|b| {
                        b["first"] == call["body"]["first_line"]
                            && b["last"] == call["body"]["last_line"]
                            && arr(&b["vars"]).unwrap().contains(&call["var"])
                    })
                    .collect();
                require(
                    matched.len() == 1,
                    "selected callback body missing or ambiguous",
                )?;
                let body = matched[0];
                let player =
                    append_delta(&current["player_records"], &call["after"]["player_records"])?;
                let enemy =
                    append_delta(&current["enemy_records"], &call["after"]["enemy_records"])?;
                let delta = json!({"player":player,"enemy":enemy});
                suppliers::check_neutral_body_inventory(&delta)?;
                let mut names = Vec::new();
                record_names(&delta, &mut names);
                let declared: BTreeSet<_> = arr(&body["declared_names"])?
                    .iter()
                    .map(|v| v.as_str().unwrap())
                    .collect();
                match body["classification"]
                    .as_str()
                    .ok_or("missing reviewed callback class")?
                {
                    "literal-writer-domain" => require(
                        names.iter().all(|n| declared.contains(n.as_str())),
                        "constructed callback record falls outside complete literal writer domain",
                    )?,
                    "inactive-dynamic-boss-branch" => require(
                        argument == "None" && names.is_empty(),
                        "dynamic boss branch requires separate source review",
                    )?,
                    "quest-parser-output-boundary" => {
                        reconciled_quests.push(review_quest_call(call, phase, rewards)?)
                    }
                    _ => return Err("unreviewed callback writer domain".into()),
                }
                variables.insert(call["var"].as_str().unwrap().to_owned());
                dispositions.push(json!({"catalogue_index":row["index"],"variable":call["var"],"argument":argument,"body_sha256":body["sha256"],"classification":body["classification"],"delta_sha256":hash(&delta),"player_record_count":player.len(),"enemy_record_count":enemy.len()}));
                current = call["after"].clone();
                at += 1;
                active += 1;
            } else if !row["apply"].is_null() {
                inactive += 1;
            }
        }
        empty_custom(&current)?;
        require(
            current == *final_state,
            "post-callback constructor changes are unexplained",
        )?;
        require(
            reconciled_quests.len() == 17
                && reconciled_quests
                    .iter()
                    .map(|q| q["player_records"].as_u64().unwrap())
                    .sum::<u64>()
                    == if phase == 0 { 9 } else { 22 },
            "quest phase construction census changed",
        )?;
        phases.push(json!({"phase":phase,"active_set":current["active_set"],"invoked":active,"inactive_apply_declarations":inactive,"without_apply":catalogue.len()-active-inactive,"calls":dispositions,"quest_parser_boundaries":[],"reconciled_quests":reconciled_quests,"custom_text":"complete-empty-constructor-path","player_records":arr(&current["player_records"])?.len(),"enemy_records":arr(&current["enemy_records"])?.len(),"configuration_sha256":hash(&current)}));
    }
    require(
        at == calls.len() && variables.len() == 63,
        "unaccounted actual callback occurrence",
    )?;
    Ok(
        json!({"catalogue_entries":663,"apply_declarations":537,"distinct_invoked_variables":63,"defaults":constructed_defaults,"phases":phases,"scope":{"direct_literal_callback_suppliers_excluded":true,"selected_none_boss_dynamic_branch_excluded":true,"empty_custom_text_suppliers_excluded":true,"selected_quest_construction_reconciled":true,"quest_parser_closure":false,"later_direct_configuration_consumers_covered":false,"native_coverage_authorized":false,"saved_input_dispositions_changed":false}}),
    )
}

pub fn reconstruct() -> Result<Value, String> {
    let source = fixture()?;
    let bodies = bodies();
    let i = suppliers::load();
    let selected = suppliers::reconcile_selected_inputs(&i)?;
    let reward_review = review_rewards(reward_fixture()?)?;
    Ok(
        json!({"schema_version":1,"kind":"offline-configuration-mana-supplier-review","selected_publication":selected["publication"],"selection":selected["selection"],"source_origin_count":selected["source_origin_count"],"bodies_sha256":hash(&bodies),"auxiliary_bodies_sha256":hash(&authenticate_auxiliary()?),"reward_reconciliation":reward_review,"off":review_state(&source["off"],&bodies)?,"on":review_state(&source["on"],&bodies)?,"remaining":["arbitrary/noncatalogue quest strings remain outside the finite source domain","later consumers of configuration fields and emitted control records","native source dispositions and query membership"],"native_coverage_authorized":false,"saved_input_dispositions_changed":false}),
    )
}
pub fn retain() {
    assert_eq!(
        std::env::var("POE_MANA_CONFIGURATION_RETAIN").as_deref(),
        Ok("1")
    );
    let artifact = reconstruct().unwrap();
    fs::write(
        suppliers::root()
            .join(DIR)
            .join("configuration-reward-review.json"),
        serde_json::to_vec_pretty(&artifact).unwrap(),
    )
    .unwrap();
}
pub fn check() -> Result<(), String> {
    require(
        read("configuration-reward-review.json") == reconstruct()?,
        "configuration supplier review differs from reconstruction",
    )
}
