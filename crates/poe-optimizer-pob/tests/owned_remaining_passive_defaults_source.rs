//! Diagnose actual default parsing for the selected, still-Partial passive owners.
//! This is offline source evidence, never authority to make a game mechanic inert.
#![cfg(not(target_arch = "wasm32"))]

#[allow(dead_code)]
#[path = "support/mod_parser_public_source.rs"]
mod parser;
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;

use mlua::{Function, MultiValue, Table, Value};
use poe_optimizer_pob::source;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::Path};

const OBSERVE: &str = r#"
remainingPassiveTree = LoadModule("TreeData/0_5/tree")
LoadModule("Classes/PassiveTree")
remainingPassiveDiskCache = LoadModule("Data/ModCache")
function remainingPassiveDefault(id)
 local raw = assert(remainingPassiveTree.nodes[id])
 assert(raw.skill == id and not raw.isKeystone)
 -- Exactly the constructor descriptor fields consumed by whole ProcessStats.
 -- Constructor layout, allocation, jewel transformations and source grants are
 -- separate evidence. In particular an empty Jewel Socket list is not inert.
 local node = {id=raw.skill, dn=raw.name, sd=copyTable(raw.stats),
  type=raw.isNotable and "Notable" or "Normal"}
 common.classes.PassiveTree.ProcessStats({}, node)
 local records = {}
 for i, mod in ipairs(node.modList) do records[i] = mod end
 return raw, node.mods, records, node.unknown or false, node.extra or false,
  node.sd, node.modKey
end
function remainingPassiveCacheReturn(line)
 local entry = remainingPassiveDiskCache[line]
 if entry then return unpack(copyTable(entry)) end
end
"#;

fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn fingerprint(path: &Path) -> Json {
    let bytes = fs::read(path).unwrap();
    json!({"bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(bytes))})
}

fn selected_rows(baseline: &Path) -> Vec<Json> {
    let draft = read(&baseline.join("original-05/draft.json"));
    let selected = read(&baseline.join("selected-05.json"));
    let rules = read(&baseline.join("package/rules.json"));
    let mapping = read(&baseline.join("package/mapping.json"));
    let schema = read(&baseline.join("package/schema.json"));
    let draft = &draft["draft"];
    let preset = draft["allocation_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == selected["build"]["allocations"])
        .unwrap();
    assert_eq!(preset["allocations"]["completion"]["kind"], "complete");
    let selected_ids = preset["allocations"]["members"].as_array().unwrap();
    assert_eq!(selected_ids.len(), 55);
    let mut out = vec![];
    for allocation in draft["allocations"]["members"].as_array().unwrap() {
        if !selected_ids.contains(&allocation["id"]) {
            continue;
        }
        assert_eq!(allocation["node"]["kind"], "known");
        let definition = &allocation["node"]["value"];
        let owner_id =
            json!({"kind":"definition","value":{"kind":"passive_node","value":definition}});
        let owners: Vec<_> = rules["owners"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|owner| owner["owner"] == owner_id)
            .collect();
        assert_eq!(owners.len(), 1);
        if owners[0]["programs"]["closure"]["kind"] == "complete" {
            continue;
        }
        assert_eq!(owners[0]["programs"]["closure"]["kind"], "partial");
        let mappings: Vec<_> = mapping["entries"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| {
                entry["outcome"]["value"]["target"] == owner_id
                    && entry["source"]["value"]["value"]["view"]["kind"] == "missing"
            })
            .collect();
        assert_eq!(mappings.len(), 1);
        let source = &mappings[0]["source"]["value"]["value"];
        assert_eq!(source["tree_version"]["value"], "0_5");
        let id: u32 = source["node_id"]["value"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let definitions: Vec<_> = schema["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["kind"] == "passive_node" && row["value"]["id"] == *definition)
            .collect();
        assert_eq!(definitions.len(), 1);
        out.push(
            json!({"source_id":id,"allocation":allocation,"mapping":mappings[0],
            "owner":owners[0],"definition":definitions[0]}),
        );
    }
    out.sort_by_key(|row| row["source_id"].as_u64().unwrap());
    assert_eq!(out.len(), 14);
    assert_eq!(
        out.iter()
            .map(|row| row["source_id"].as_u64().unwrap())
            .collect::<BTreeSet<_>>()
            .len(),
        14
    );
    out
}

fn observe(rows: &[Json], jit: bool) -> Json {
    let oracle = parser::PublicSource::new();
    let lua = &oracle.source.lua;
    lua.load(if jit {
        "jit.on()"
    } else {
        "jit.off();jit.flush()"
    })
    .exec()
    .unwrap();
    lua.load(OBSERVE)
        .set_name("@remaining-passive-default-observer")
        .exec()
        .unwrap();
    let default: Function = lua.globals().get("remainingPassiveDefault").unwrap();
    let disk: Table = lua.globals().get("remainingPassiveDiskCache").unwrap();
    let disk_return: Function = lua.globals().get("remainingPassiveCacheReturn").unwrap();
    let tree: Table = lua.globals().get("remainingPassiveTree").unwrap();
    let nodes: Table = tree.get("nodes").unwrap();
    let mut inputs = BTreeSet::new();
    for row in rows {
        let raw: Table = nodes.get(row["source_id"].as_u64().unwrap()).unwrap();
        let stats: Table = raw.get("stats").unwrap();
        let lines: Vec<String> = stats.sequence_values().collect::<Result<_, _>>().unwrap();
        for start in 0..lines.len() {
            for end in start + 1..=lines.len() {
                inputs.insert(lines[start..end].join(" "));
            }
        }
    }
    let mut parsed = vec![];
    for line in inputs {
        assert!(oracle.cache.get::<Value>(line.as_str()).unwrap().is_nil());
        let fresh = oracle
            .capture(oracle.raw(line.as_bytes()).unwrap())
            .unwrap();
        let warm = oracle
            .capture(oracle.raw(line.as_bytes()).unwrap())
            .unwrap();
        assert_eq!(fresh, warm, "public parser cache repeat: {line}");
        let entry: Value = disk.get(line.as_str()).unwrap();
        let present = !entry.is_nil();
        let cached_return = oracle
            .capture(disk_return.call::<MultiValue>(line.as_str()).unwrap())
            .unwrap();
        parsed.push(json!({"line":line,"fresh_and_warm":fresh,
            "disk_entry_present":present,"disk_entry":oracle.capture(MultiValue::from_vec(vec![entry])).unwrap(),
            "disk_return":cached_return,"disk_return_matches_live":present && fresh==cached_return}));
    }
    let mut defaults = vec![];
    for row in rows {
        let id = row["source_id"].as_u64().unwrap();
        let values: MultiValue = default.call(id).unwrap();
        assert_eq!(values.len(), 7);
        let raw = values[0].as_table().unwrap().clone();
        let mods = values[1].as_table().unwrap().clone();
        let accepted = values[2].as_table().unwrap().clone();
        let unknown = values[3].as_boolean().unwrap();
        let extra = values[4].as_boolean().unwrap();
        let mut lines = vec![];
        for (index, line) in raw
            .get::<Table>("stats")
            .unwrap()
            .sequence_values::<String>()
            .enumerate()
        {
            let line = line.unwrap();
            let parsed: Table = mods.get(index + 1).unwrap();
            lines.push(
                json!({"line":line,"list_present":!parsed.get::<Value>("list").unwrap().is_nil(),
                "extra":parsed.get::<Option<String>>("extra").unwrap(),
                "combined":parsed.get::<Option<bool>>("combined").unwrap().unwrap_or(false)}),
            );
        }
        let graph = oracle.capture(values).unwrap();
        assert_eq!(
            graph,
            oracle.capture(default.call(id).unwrap()).unwrap(),
            "ProcessStats repeat: {id}"
        );
        defaults.push(
            json!({"source_id":id,"name":raw.get::<String>("name").unwrap(),
            "unknown":unknown,"extra":extra,"lines":lines,"accepted_count":accepted.raw_len(),
            "full_default_graph":graph}),
        );
    }
    json!({"parser":parsed,"defaults":defaults})
}

#[test]
#[ignore = "offline source audit; needs authenticated selected baseline and a fresh output directory"]
fn remaining_selected_defaults_preserve_actual_parser_remainders() {
    let baseline = std::env::var_os("POE_OPTIMIZER_TEST_REMAINING_PASSIVES_BASELINE")
        .map(std::path::PathBuf::from)
        .expect("set POE_OPTIMIZER_TEST_REMAINING_PASSIVES_BASELINE");
    let output = std::env::var_os("POE_OPTIMIZER_TEST_REMAINING_PASSIVES_OUTPUT")
        .map(std::path::PathBuf::from)
        .expect("set POE_OPTIMIZER_TEST_REMAINING_PASSIVES_OUTPUT");
    assert!(!output.exists(), "retain prior source evidence");
    let rows = selected_rows(&baseline);
    let mut sources = vec![];
    for path in [
        "src/TreeData/0_5/tree.lua",
        "src/Classes/PassiveTree.lua",
        "src/Modules/ModParser.lua",
        "src/Modules/ModTools.lua",
        "src/Data/ModCache.lua",
    ] {
        let text = runtime::verified(path).unwrap();
        let hash = format!("{:x}", Sha256::digest(text.as_bytes()));
        assert_eq!(hash, source::expected_file_sha256(path).unwrap());
        sources.push(json!({"path":path,"sha256":hash,"bytes":text.len()}));
    }
    let off = observe(&rows, false);
    let repeat = observe(&rows, false);
    let on = observe(&rows, true);
    assert_eq!(off, repeat, "independent fresh JIT-off replay");
    assert_eq!(off, on, "independent fresh JIT-on replay");
    let files = [
        "original-05/draft.json",
        "selected-05.json",
        "package/rules.json",
        "package/mapping.json",
        "package/schema.json",
        "package/release.json",
    ]
    .map(|path| json!({"path":path,"identity":fingerprint(&baseline.join(path))}));
    let report = json!({"source_revision":source::UPSTREAM_REVISION,"sources":sources,
        "baseline_files":files,"selected_partial_owners":rows,"observation":off,
        "determinism":{"fresh_jit_off":2,"fresh_jit_on":1,"warm_parser_and_process_stats":true},
        "scope":{"default_process_stats":true,"full_constructor":false,"effective_transformations":false,
            "source_missing_handler_is_gameplay_inert":false,"owned_closure_changes":false,"whole_build_parity":false}});
    fs::create_dir_all(&output).unwrap();
    fs::write(
        output.join("source.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    for row in report["observation"]["defaults"].as_array().unwrap() {
        println!(
            "node {} {}: accepted {}, unknown {}, extra {}",
            row["source_id"], row["name"], row["accepted_count"], row["unknown"], row["extra"]
        );
    }
}
