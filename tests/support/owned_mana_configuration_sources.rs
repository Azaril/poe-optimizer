//! Finite offline constructor audit. No Lua behavior is added to native runtime.
#[allow(dead_code)]
#[path = "owned_configuration_accounting.rs"]
mod accounting;
#[allow(dead_code)]
#[path = "owned_mana_supplier_sources.rs"]
mod suppliers;

use serde_json::{Map, Value, json};
use std::{collections::BTreeSet, fs, sync::OnceLock};

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

pub fn review_state(state: &Value, bodies: &Value) -> Result<Value, String> {
    authenticate_bodies(bodies)?;
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
        let mut unresolved = Vec::new();
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
                match body["classification"].as_str().ok_or("missing reviewed callback class")? {
                    "literal-writer-domain"=>require(names.iter().all(|n|declared.contains(n.as_str())),"constructed callback record falls outside complete literal writer domain")?,
                    "inactive-dynamic-boss-branch"=>require(argument=="None"&&names.is_empty(),"dynamic boss branch requires separate source review")?,
                    "quest-parser-output-boundary"=>unresolved.push(json!({"var":call["var"],"argument":argument,"body":body["sha256"],"emitted_names":names,"closure":"pending-quest-parser-authority"})),
                    _=>return Err("unreviewed callback writer domain".into()),
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
        phases.push(json!({"phase":phase,"active_set":current["active_set"],"invoked":active,"inactive_apply_declarations":inactive,"without_apply":catalogue.len()-active-inactive,"calls":dispositions,"quest_parser_boundaries":unresolved,"custom_text":"complete-empty-constructor-path","player_records":arr(&current["player_records"])?.len(),"enemy_records":arr(&current["enemy_records"])?.len(),"configuration_sha256":hash(&current)}));
    }
    require(
        at == calls.len() && variables.len() == 63,
        "unaccounted actual callback occurrence",
    )?;
    Ok(
        json!({"catalogue_entries":663,"apply_declarations":537,"distinct_invoked_variables":63,"defaults":constructed_defaults,"phases":phases,"scope":{"direct_literal_callback_suppliers_excluded":true,"selected_none_boss_dynamic_branch_excluded":true,"empty_custom_text_suppliers_excluded":true,"quest_parser_closure":false,"later_direct_configuration_consumers_covered":false,"native_coverage_authorized":false,"saved_input_dispositions_changed":false}}),
    )
}

pub fn reconstruct() -> Result<Value, String> {
    let source = fixture()?;
    let bodies = bodies();
    let i = suppliers::load();
    let selected = suppliers::reconcile_selected_inputs(&i)?;
    Ok(
        json!({"schema_version":1,"kind":"offline-configuration-mana-supplier-review","selected_publication":selected["publication"],"selection":selected["selection"],"source_origin_count":selected["source_origin_count"],"bodies_sha256":hash(&bodies),"auxiliary_bodies_sha256":hash(&authenticate_auxiliary()?),"off":review_state(&source["off"],&bodies)?,"on":review_state(&source["on"],&bodies)?,"remaining":bodies["remaining"],"native_coverage_authorized":false,"saved_input_dispositions_changed":false}),
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
            .join("configuration-review.json"),
        serde_json::to_vec_pretty(&artifact).unwrap(),
    )
    .unwrap();
}
pub fn check() -> Result<(), String> {
    require(
        read("configuration-review.json") == reconstruct()?,
        "configuration supplier review differs from reconstruction",
    )
}
