//! Finite selected parsed-record transport, not passive or native completeness.
#[allow(dead_code)]
#[path = "owned_mana_radius_sources.rs"]
mod radius;
use radius::supplier;

use flate2::read::GzDecoder;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    sync::OnceLock,
};

const MANIFEST: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const GATES: [&str; 11] = [
    "PassiveSkillHasNoEffect",
    "AllocatedPassiveSkillHasNoEffect",
    "PassiveSkillEffect",
    "PassiveSkillHasOtherEffect",
    "NodeModifier",
    "CanExplode",
    "JewelSmallPassiveSkillEffect",
    "JewelNotablePassiveSkillEffect",
    "SmallPassiveSkillEffect",
    "ExtraJewelFunc",
    "Keystone",
];

pub fn read(name: &str) -> Value {
    supplier::read(format!("{}/{name}", supplier::DIR))
}
fn ensure(ok: bool, why: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(why.into()) }
}
fn array<'a>(v: &'a Value, why: &str) -> Result<&'a [Value], String> {
    if v.as_object().is_some_and(serde_json::Map::is_empty) {
        return Ok(&[]);
    }
    v.as_array().map(Vec::as_slice).ok_or_else(|| why.into())
}
fn digest(v: &Value) -> String {
    supplier::hash(&serde_json::to_vec(v).unwrap())
}
fn token(v: &Value) -> String {
    serde_json::to_string(v).unwrap()
}
fn bag<'a>(values: impl IntoIterator<Item = &'a Value>) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for v in values {
        *out.entry(token(v)).or_default() += 1;
    }
    out
}

pub fn fixture() -> &'static Value {
    static FIXTURE: OnceLock<Value> = OnceLock::new();
    FIXTURE.get_or_init(||{
        radius::check().unwrap();
        let source=supplier::acquisition_fixture().unwrap();
        let project=|v:&Value|json!({"environments":v["state"]["environments"].as_array().unwrap().iter()
            .map(|e|json!({"mode":e["mode"],"nodes":e["nodes"]})).collect::<Vec<_>>(),
            "node_construction":v["state"]["node_construction"],"node_lists":v["state"]["node_lists"]});
        json!({"off":project(&source.jit_off),"on":project(&source.jit_on),
            "memberships":radius::selected()["passives"],
            "historical_publication":radius::selected()["historical_publication"],
            "radius_review_sha256":supplier::hash(&fs::read(supplier::root().join(supplier::DIR).join("radius-review.json")).unwrap()),
            "radius_report_sha256":"8784718a733d10878da117724b3dff69e97d380c0140bfbab1cc02ffbf11a2da",
            "source02_comparison_sha256":"be695bc423aeb637b6d5eb8343246cf6c9879eb4549f82cc7c8ba4b046ab8155"})
    })
}

pub fn review_bodies(bodies: &Value) -> Result<(), String> {
    let bytes =
        fs::read(supplier::root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"))
            .map_err(|e| e.to_string())?;
    ensure(
        supplier::hash(&bytes) == MANIFEST,
        "independent source manifest",
    )?;
    let manifest: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    ensure(
        bodies["schema_version"] == 1
            && bodies["kind"] == "selected-passive-constructor-source-bodies"
            && bodies["source_manifest"] == MANIFEST
            && bodies["source_revision"] == REVISION,
        "body source identity",
    )?;
    let paths = [
        ("src/Modules/CalcSetup.lua", "CalcSetup.lua.gz"),
        ("src/Classes/ModList.lua", "passive-ModList.lua.gz"),
        (
            "src/Classes/ModStore.lua",
            "../configuration-distance-inputs/ModStore.lua.gz",
        ),
    ];
    let files = array(&bodies["files"], "source file census")?;
    ensure(files.len() == 3, "three exact source files")?;
    let mut texts = BTreeMap::new();
    for (row, (path, archive)) in files.iter().zip(paths) {
        ensure(
            row["path"] == path && row["archive"] == archive,
            "source file path",
        )?;
        let pin = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["path"] == path)
            .ok_or("manifest file")?;
        let file = supplier::root().join(supplier::DIR).join(archive);
        ensure(
            fs::metadata(&file).map_err(|e| e.to_string())?.len() <= 64 * 1024,
            "bounded compressed source",
        )?;
        let compressed = fs::read(file).map_err(|e| e.to_string())?;
        ensure(
            row["compressed_bytes"] == compressed.len()
                && row["compressed_sha256"] == supplier::hash(&compressed),
            "archive source commitment",
        )?;
        let mut bytes = Vec::new();
        GzDecoder::new(compressed.as_slice())
            .take(256 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        ensure(
            bytes.len() <= 256 * 1024
                && row["bytes"] == bytes.len()
                && row["sha256"] == supplier::hash(&bytes)
                && row["sha256"] == pin["sha256"],
            "manifest-authenticated complete source",
        )?;
        texts.insert(path, String::from_utf8(bytes).map_err(|e| e.to_string())?);
    }
    let spans = [
        ("scratch-list-reset", 0, 188, 197),
        ("complete-selected-node-constructor", 0, 200, 412),
        ("complete-node-list-transport", 0, 415, 464),
        ("flat-list-construction-and-append", 1, 20, 31),
        ("ordered-list-append", 1, 90, 96),
        ("list-value-lookup-boundary", 1, 309, 326),
        ("empty-store-construction", 2, 65, 71),
        ("scale-copy-name-preservation-boundary", 2, 82, 119),
        ("copy-list-transport", 2, 121, 125),
        ("scale-list-dispatch-boundary", 2, 127, 135),
    ];
    let actual = array(&bodies["spans"], "source body census")?;
    ensure(
        actual.len() == spans.len(),
        "complete constructor/transport spans",
    )?;
    for (row, (role, path, first, last)) in actual.iter().zip(spans) {
        let path = paths[path].0;
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
            "original complete source body changed",
        )?;
    }
    ensure(
        bodies["scope"]
            == json!({"finite_selected_parsed_record_transport_only":true,"arbitrary_parsing_covered":false,
        "numerical_scaling_or_order_covered":false,"generated_skill_delivery_covered":false,"native_coverage_authorized":false}),
        "source proof scope",
    )
}

fn inspect_nested(v: &Value) -> Result<(), String> {
    match v {
        Value::Object(o) => {
            ensure(
                !matches!(
                    o.get("kind").and_then(Value::as_str),
                    Some("callback_reference" | "table_reference")
                ),
                "unresolved executable/alias in source body",
            )?;
            if let Some(name) = o.get("name").and_then(Value::as_str) {
                ensure(
                    !GATES.contains(&name),
                    "unreviewed passive transformation gate",
                )?;
            }
            for child in o.values() {
                inspect_nested(child)?;
            }
        }
        Value::Array(a) => {
            for child in a {
                inspect_nested(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn records(v: &Value) -> Result<Vec<&Value>, String> {
    let rows = array(v, "record list")?;
    let mut result = Vec::new();
    for (at, row) in rows.iter().enumerate() {
        ensure(
            row.as_object().is_some_and(|o| o.len() == 2) && row["position"] == at + 1,
            "ordered record positions",
        )?;
        let body = &row["body"];
        ensure(
            body["name"].is_string() && body["type"].is_string(),
            "complete modifier body",
        )?;
        ensure(
            body.get("parsedLine").is_none(),
            "unreviewed jewel parsed-line routing",
        )?;
        for tag in array(
            body.get("_positions").unwrap_or(&Value::Null),
            "modifier tags",
        )
        .or_else(|e| {
            if body.get("_positions").is_none() {
                Ok(&[][..])
            } else {
                Err(e)
            }
        })? {
            ensure(
                !tag["value"]["var"]
                    .as_str()
                    .is_some_and(|s| s.starts_with("WeaponSet")),
                "unreviewed weapon-set tag rewrite",
            )?;
        }
        supplier::check_neutral_body_inventory(body)?;
        inspect_nested(body)?;
        result.push(body);
    }
    Ok(result)
}

fn keyed_nodes(v: &Value) -> Result<BTreeMap<u64, &Value>, String> {
    let mut nodes = BTreeMap::new();
    for row in array(v, "keyed node inventory")? {
        let key = row["key"].as_u64().ok_or("node key")?;
        ensure(
            nodes.insert(key, &row["value"]).is_none(),
            "duplicate keyed node",
        )?;
    }
    ensure(nodes.len() == 57, "complete 57-node inventory")?;
    Ok(nodes)
}

pub fn review_node(node: &Value) -> Result<Vec<&Value>, String> {
    ensure(
        node["fields"]["allocMode"] == 0 && node["fields"]["alloc"] == true,
        "unreviewed node allocation mode",
    )?;
    ensure(
        array(&node["callbacks"], "node callback inventory")?.is_empty()
            && node["keystone"].is_null(),
        "unreviewed callback/keystone producer",
    )?;
    let result = records(&node["records"])?;
    // Empty parsed records do not certify source text or generated descendants.
    Ok(result)
}

fn by_name(records: &[&Value]) -> BTreeMap<String, Vec<Value>> {
    let mut result = BTreeMap::new();
    for body in records {
        result
            .entry(body["name"].as_str().unwrap().to_owned())
            .or_insert_with(Vec::new)
            .push((*body).clone());
    }
    result
}

pub fn review_run(run: &Value, memberships: &Value) -> Result<Value, String> {
    let environments = array(&run["environments"], "selected environments")?;
    let constructors = array(&run["node_construction"], "node constructor returns")?;
    let lists = array(&run["node_lists"], "node-list returns")?;
    ensure(
        environments.len() == 21 && constructors.len() == 2394 && lists.len() == 42,
        "complete construction census",
    )?;
    let catalogue = keyed_nodes(&environments[0]["nodes"])?;
    let source_ids = array(memberships, "exact owned source memberships")?
        .iter()
        .map(|r| {
            r["source_token"]
                .as_str()
                .ok_or("source token")?
                .parse::<u64>()
                .map_err(|e| e.to_string())
        })
        .collect::<Result<BTreeSet<_>, String>>()?;
    ensure(
        catalogue.keys().copied().collect::<BTreeSet<_>>() == source_ids,
        "catalogue differs from 55 allocations plus two roots",
    )?;
    let mut flat = Vec::new();
    let mut generated = Vec::new();
    let mut empty_declarations = Vec::new();
    for (key, node) in &catalogue {
        let mods = review_node(node)?;
        if mods.is_empty() {
            empty_declarations.push(json!({"source_node":key,"stats":node["stats"],
            "description":node["description"],"fields":node["fields"],"mechanics_absence_proven":false}));
        }
        for body in &mods {
            if body["name"] == "ExtraSkill" {
                generated.push(
                    json!({"source_node":key,"body":body,"descendant_delivery_covered":false}),
                );
            }
        }
        flat.extend(mods);
    }
    ensure(
        generated.len() == 2
            && generated
                .iter()
                .map(|r| r["source_node"].as_u64().unwrap())
                .collect::<BTreeSet<_>>()
                == BTreeSet::from([13289, 32705]),
        "exact generated-skill boundaries",
    )?;
    // The payloads are source authority, not spell-name routing or new skills.
    let reference = keyed_nodes(&fixture()["off"]["environments"][0]["nodes"])?;
    for row in &generated {
        let source = row["source_node"].as_u64().unwrap();
        ensure(
            catalogue[&source]["records"] == reference[&source]["records"],
            "generated skill declaration changed",
        )?;
    }
    ensure(
        flat.len() == 57 && empty_declarations.len() == 12,
        "selected parsed record census",
    )?;
    let body_counts = bag(catalogue.values().copied());
    ensure(
        body_counts.len() == 55,
        "distinct catalogue body census with duplicate multiplicity",
    )?;
    let mut mode_counts = BTreeMap::<String, usize>::new();
    for env in environments {
        ensure(
            keyed_nodes(&env["nodes"])? == catalogue,
            "changed keyed environment body",
        )?;
        let mode = env["mode"].as_str().ok_or("environment mode")?;
        *mode_counts.entry(mode.to_owned()).or_default() += 1;
    }
    ensure(
        mode_counts
            == BTreeMap::from([
                ("MAIN".into(), 16),
                ("CALCS".into(), 1),
                ("CALCULATOR".into(), 4),
            ]),
        "exact source environment modes",
    )?;
    let mut actual = BTreeMap::<String, BTreeMap<String, usize>>::new();
    let mut record_count = 0;
    let mut empty_returns = 0;
    for row in constructors {
        ensure(
            row["inc_small_passive_skill"] == 0 && row["radius_jewel_count"] == 0,
            "unreviewed scaling/radius constructor input",
        )?;
        let mode = row["mode"].as_str().ok_or("constructor mode")?;
        ensure(mode_counts.contains_key(mode), "unknown constructor mode")?;
        let include = &row["include_keystone_mods"];
        ensure(
            include.is_null() || include == true,
            "unreviewed keystone argument",
        )?;
        let mods = review_node(&row["node"])?;
        let output = records(&row["output_records"])?;
        ensure(
            row["node"]["records"] == row["output_records"],
            "constructor changed ordered complete records",
        )?;
        ensure(mods == output, "full source/output body correspondence")?;
        record_count += mods.len();
        empty_returns += usize::from(mods.is_empty());
        let lane = format!("{mode}:{}", token(include));
        *actual
            .entry(lane)
            .or_default()
            .entry(token(&row["node"]))
            .or_default() += 1;
    }
    let mut expected = BTreeMap::new();
    for (mode, count) in &mode_counts {
        for flag in ["true", "null"] {
            expected.insert(
                format!("{mode}:{flag}"),
                body_counts
                    .iter()
                    .map(|(body, n)| (body.clone(), n * count))
                    .collect::<BTreeMap<_, _>>(),
            );
        }
    }
    ensure(
        actual == expected,
        "anonymous return body multiplicity differs from keyed catalogue",
    )?;
    let expected_records = bag(flat.iter().copied());
    let mut list_lanes = BTreeMap::<String, usize>::new();
    let mut ordered_receipts = Vec::new();
    for (at, list) in lists.iter().enumerate() {
        let mode = list["mode"].as_str().ok_or("list mode")?;
        let flag = &list["include_keystone_mods"];
        ensure(
            mode_counts.contains_key(mode)
                && (flag.is_null() || flag == true)
                && list["finish_jewels"] == true,
            "list mode/argument",
        )?;
        let selected = keyed_nodes(&list["nodes"])?;
        ensure(
            selected.keys().copied().collect::<BTreeSet<_>>() == source_ids,
            "node-list membership differs",
        )?;
        for (key, node) in selected {
            ensure(
                node["id"] == key && node["fields"] == catalogue[&key]["fields"],
                "node-list source body identity",
            )?;
        }
        ensure(
            array(&list["sockets"]["radius_jewels"], "radius input")?.is_empty()
                && array(&list["sockets"]["extra_radius_nodes"], "extra radius input")?.is_empty(),
            "node-list radius input changed",
        )?;
        let output = records(&list["output_records"])?;
        ensure(
            output.len() == 57 && bag(output.iter().copied()) == expected_records,
            "node-list full-body union/multiplicity",
        )?;
        let lane = format!("{mode}:{}", token(flag));
        *list_lanes.entry(lane.clone()).or_default() += 1;
        ordered_receipts.push(json!({"source_list_ordinal":at,"lane":lane,
            "raw_order_sha256":digest(&list["output_records"]),"within_name_order_sha256":digest(&json!(by_name(&output))),
            "records":output.len(),"numerical_order_authority":false}));
    }
    let expected_lanes: BTreeMap<_, _> = mode_counts
        .iter()
        .flat_map(|(m, n)| ["true", "null"].map(|f| (format!("{m}:{f}"), *n)))
        .collect();
    ensure(list_lanes == expected_lanes, "complete node-list lanes")?;
    let root_names: BTreeMap<_, _> = by_name(&flat)
        .iter()
        .map(|(name, rows)| (name.clone(), rows.len()))
        .collect();
    Ok(
        json!({"environment_count":21,"selected_nodes":57,"distinct_node_bodies":55,"node_returns":2394,
        "ordered_records_returned":record_count,"empty_node_returns":empty_returns,"node_lists":42,
        "root_record_names":root_names,"anonymous_body_lanes":expected_lanes.iter().map(|(k,n)|(k.clone(),n*57)).collect::<BTreeMap<_,_>>(),
        "list_order_receipts":ordered_receipts,"generated_boundaries":generated,"empty_parsed_declarations":empty_declarations,
        "finite_parsed_record_transport_reconciled":true,"native_coverage_authorized":false}),
    )
}

pub fn reconstruct(input: &Value, bodies: &Value) -> Result<Value, String> {
    review_bodies(bodies)?;
    ensure(
        input["memberships"] == radius::selected()["passives"],
        "exact historical source ownership",
    )?;
    let off = review_run(&input["off"], &input["memberships"])?;
    let on = review_run(&input["on"], &input["memberships"])?;
    ensure(
        off == on,
        "independent source runs preserve complete within-name records",
    )?;
    ensure(
        input == fixture(),
        "supplied source evidence differs from authenticated originals",
    )?;
    Ok(
        json!({"schema_version":1,"kind":"selected-passive-constructor-source-review","source_revision":REVISION,
        "source_manifest_sha256":MANIFEST,"source02_comparison_sha256":input["source02_comparison_sha256"],
        "radius_report_sha256":input["radius_report_sha256"],"radius_review_sha256":input["radius_review_sha256"],
        "historical_membership_publication":input["historical_publication"],"memberships_sha256":digest(&input["memberships"]),
        "bodies_sha256":digest(bodies),"mana_adjustment_names":supplier::relevant_names(),"observed":off,
        "scope":{"finite_selected_parsed_record_transport_only":true,"node_ids_assigned_to_anonymous_returns":false,
            "arbitrary_parsing_covered":false,"empty_parse_means_no_mechanics":false,"generated_skill_delivery_covered":false,
            "support_or_later_recipient_delivery_covered":false,"numerical_scaling_or_order_covered":false,
            "native_partial_owners_closed":0,"native_coverage_authorized":false,"current_native_package_changed":false}}),
    )
}

pub fn check() -> Result<(), String> {
    ensure(
        read("passive-review.json") == reconstruct(fixture(), &read("passive-bodies.json"))?,
        "retained passive review changed",
    )
}
pub fn retain() -> Result<(), String> {
    let result = reconstruct(fixture(), &read("passive-bodies.json"))?;
    let mut bytes = serde_json::to_vec_pretty(&result).unwrap();
    bytes.push(b'\n');
    fs::write(
        supplier::root()
            .join(supplier::DIR)
            .join("passive-review.json"),
        bytes,
    )
    .map_err(|e| e.to_string())
}
