//! Bounded source review; parser rejection never supplies an absence proof.
use super::{EXCLUDED, FAMILIES, bounded_read, data, exact, hash, read, rejected, root};
use flate2::read::GzDecoder;
use poe_optimizer_data::skill_identities::{SkillIdentityCatalog, SkillIdentityData};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    sync::OnceLock,
};

const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const TREE: &str = "src/TreeData/0_5/tree.json";
const TREE_HASH: &str = "6449fe534c0265b21f59f8213254bd3f37a445887582c40aab04ad11cede3e95";
const DESCRIPTIONS: &str = "src/Data/StatDescriptions/stat_descriptions.lua";
const PINS: [(&str, &str, &str); 6] = [
    (
        "source_manifest",
        "crates/poe-optimizer-pob/data/pob-source-manifest.json",
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675",
    ),
    (
        "recipes",
        "data/owned/poe2/3887ae68/self-flat-attack-damage/selected-domain.json",
        "ebe5ed14555ef4200f843dbf2972e3d3bd5fe82b4d3c4b558e07f187d0c1681f",
    ),
    (
        "selected_bindings",
        "data/owned/poe2/3887ae68/self-flat-selected-bindings/bindings.json",
        "2b54e203a49e35da80e92a0a3dd09e990ff79574cb5629d9852634ca185727b4",
    ),
    (
        "catalog",
        "data/owned/poe2/3887ae68/import/skill-identities.json",
        "b9064ec447ccf3c3fc88244655a36bbb4453e50e16ffa64eda9880f756aed10e",
    ),
    (
        "grant_bindings",
        "data/owned/poe2/3887ae68/djinn-tree-grants/bindings.json",
        "780c278e35c29ac1aa5eeba9cf1028396334af0dba1a25bc7d828aa8c4113aaa",
    ),
    (
        "grant_authority",
        "data/owned/poe2/3887ae68/djinn-tree-grants/authoring.json",
        "70955a2e23c0ed6340be04e1bda4561a8cb67fc5431f406f61d3d53dbaca36d7",
    ),
];
struct Facts {
    proof: Value,
    inputs: BTreeMap<&'static str, Value>,
    sources: BTreeMap<String, String>,
    tree: Value,
}
fn string(v: &Value) -> &str {
    v.as_str().unwrap()
}
fn facts() -> &'static Facts {
    static FACTS: OnceLock<Facts> = OnceLock::new();
    FACTS.get_or_init(|| {
        let proof = read("evidence.json");
        assert_eq!(proof["source_revision"], REVISION);
        let mut inputs = BTreeMap::new();
        for (name, path, digest) in PINS {
            let pin = &proof[name];
            assert_eq!(pin["path"], path);
            assert_eq!(pin["sha256"], digest);
            let bytes = bounded_read(root().join(path), 4 * 1024 * 1024);
            assert_eq!(pin["bytes"], bytes.len());
            assert_eq!(hash(&bytes), digest);
            inputs.insert(name, serde_json::from_slice::<Value>(&bytes).unwrap());
        }
        let catalog: SkillIdentityData = serde_json::from_value(inputs["catalog"].clone()).unwrap();
        SkillIdentityCatalog::new(catalog).unwrap();
        assert_eq!(inputs["source_manifest"]["upstream_revision"], REVISION);
        assert_eq!(inputs["recipes"]["source_revision"], REVISION);
        assert_eq!(inputs["grant_authority"]["source_revision"], REVISION);
        assert_eq!(
            inputs["grant_authority"]["source_tree_json_sha256"],
            TREE_HASH
        );
        assert_eq!(
            inputs["grant_authority"]["artifact_sha256"]["bindings"],
            proof["grant_bindings"]["sha256"]
        );
        let mut sources = BTreeMap::new();
        let mut total = 0;
        for pin in proof["sources"].as_array().unwrap() {
            let path = string(&pin["path"]);
            let archive = format!("sources/{}.gz", path.replace('/', "_"));
            assert_eq!(pin["archive"], archive);
            let bytes = bounded_read(data().join(archive), 2 * 1024 * 1024);
            assert_eq!(pin["archive_bytes"], bytes.len());
            assert_eq!(pin["archive_sha256"], hash(&bytes));
            let mut decoded = Vec::new();
            GzDecoder::new(bytes.as_slice())
                .take(8 * 1024 * 1024 + 1)
                .read_to_end(&mut decoded)
                .unwrap();
            total += decoded.len();
            assert!(decoded.len() <= 8 * 1024 * 1024 && total <= 16 * 1024 * 1024);
            assert_eq!(pin["bytes"], decoded.len());
            assert_eq!(pin["sha256"], hash(&decoded));
            assert!(!decoded.contains(&b'\r'));
            if path == TREE {
                assert_eq!(pin["authority"], "pinned-tree-json");
                assert_eq!(pin["sha256"], TREE_HASH);
            } else {
                assert_eq!(pin["authority"], "lua-source-manifest");
                let source = exact(&inputs["source_manifest"]["files"], "/path", &pin["path"]);
                assert_eq!(source["sha256"], pin["sha256"]);
                assert_eq!(source["bytes"], pin["bytes"]);
            }
            assert!(
                sources
                    .insert(path.to_owned(), String::from_utf8(decoded).unwrap())
                    .is_none()
            );
        }
        assert_eq!(sources.len(), 9);
        let tree = serde_json::from_str(&sources[TREE]).unwrap();
        let facts = Facts {
            proof,
            inputs,
            sources,
            tree,
        };
        check_boundaries(&facts);
        facts
    })
}
fn span(sources: &BTreeMap<String, String>, row: &Value) -> String {
    let first = row
        .get("first_line")
        .unwrap_or(&row["line"])
        .as_u64()
        .unwrap() as usize;
    let last = row
        .get("last_line")
        .unwrap_or(&row["end_line"])
        .as_u64()
        .unwrap() as usize;
    assert!(first > 0 && last >= first && last - first < 1000);
    let lines: Vec<_> = sources[string(&row["path"])]
        .split_inclusive('\n')
        .collect();
    assert!(last <= lines.len());
    let body = lines[first - 1..last].concat();
    assert_eq!(hash(body.as_bytes()), string(&row["sha256"]));
    body
}
fn check_boundaries(f: &Facts) {
    let rows = f.proof["boundaries"].as_array().unwrap();
    let expected = [
        ("src/Classes/PassiveTree.lua", 211, 310),
        ("src/Classes/PassiveTree.lua", 407, 417),
        ("src/Classes/PassiveTree.lua", 448, 525),
        ("src/Classes/PassiveSpec.lua", 360, 387),
        ("src/Classes/PassiveSpec.lua", 1319, 1334),
        ("src/Classes/PassiveSpec.lua", 1450, 1530),
        ("src/Classes/PassiveSpec.lua", 1555, 1790),
        ("src/Classes/PassiveSpec.lua", 2047, 2077),
        ("src/Classes/PassiveSpec.lua", 2619, 2704),
        ("src/Modules/CalcSetup.lua", 200, 412),
        ("src/Modules/CalcSetup.lua", 1669, 1730),
        ("src/Modules/ModParser.lua", 2186, 2200),
        ("src/Modules/ModParser.lua", 3589, 3590),
        ("src/Export/Scripts/statdesc.lua", 14, 47),
        ("src/Export/Scripts/statdesc.lua", 109, 122),
        ("src/Modules/StatDescriber.lua", 13, 39),
    ];
    assert_eq!(rows.len(), expected.len());
    for (row, (path, first, last)) in rows.iter().zip(expected) {
        assert_eq!(row["path"], path);
        assert_eq!(row["first_line"], first);
        assert_eq!(row["last_line"], last);
        let _ = span(&f.sources, row);
    }
    let parser = span(&f.sources, &rows[2]);
    assert!(parser.starts_with("function PassiveTreeClass:ProcessStats("));
    assert!(parser.contains("if mod.list and not mod.extra then"));
    let constructor = span(&f.sources, &rows[9]);
    assert!(constructor.starts_with("function calcs.buildModListForNode("));
    assert!(constructor.ends_with("\treturn modList\nend\n"));
    for marker in [
        "env.radiusJewelList",
        "\"PassiveSkillHasOtherEffect\"",
        "\"NodeModifier\"",
        "node.grantedSkills = wipeTable(node.grantedSkills)",
        "modList:List(nil, \"ExtraSkill\")",
        "source = \"Tree:\" .. node.id",
    ] {
        assert!(constructor.contains(marker));
    }
    let spec = &f.sources["src/Classes/PassiveSpec.lua"];
    assert!(spec.contains("self:ReplaceNode(node, override)"));
    assert!(spec.contains("function PassiveSpecClass:NodeAdditionOrReplacementFromString("));
}
fn identity(row: &Value, f: &Facts) {
    let node_id = string(&row["node_id"]);
    assert_eq!(&row["node"], &f.tree["nodes"][node_id]);
    let mapping = &row["mapping"];
    assert_eq!(
        mapping["source"],
        json!({
            "kind":"definition","value":{"kind":"passive_node","value":{
                "node_id":{"kind":"text","value":node_id},
                "tree_version":{"kind":"text","value":"0_5"},
                "view":{"kind":"missing"}
            }}
        })
    );
    assert_eq!(mapping["outcome"]["kind"], "mapped");
    assert_eq!(
        mapping["outcome"]["value"]["basis"],
        json!({"kind":"exact"})
    );
    let target = &mapping["outcome"]["value"]["target"];
    assert_eq!(target["kind"], "definition");
    assert_eq!(target["value"]["kind"], "passive_node");
    assert_eq!(target["value"]["value"]["key"], row["passive"]);
    assert_eq!(
        target["value"]["value"]["namespace"],
        json!({"game":"poe2","version":"owned-mechanics-v1"})
    );
    let binding = exact(
        &f.inputs["selected_bindings"]["passives"],
        "/source_token",
        &row["node_id"],
    );
    assert_eq!(binding, &row["selected_binding"]);
    assert_eq!(binding["mapping"], *mapping);
    assert_eq!(
        binding["binding"]["value"]["node"]["value"],
        target["value"]["value"]
    );
    assert_eq!(binding["binding"]["value"]["node"]["kind"], "known");
    let recipe = exact(&f.inputs["recipes"]["tree"], "/id", &row["node_id"]);
    assert_eq!(
        recipe, &row["recipe"],
        "retain full parser output including rejected remainders"
    );
    assert_eq!(recipe["name"], row["node"]["name"]);
    let lines: Vec<_> = recipe["stats"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["line"].clone())
        .collect();
    assert_eq!(json!(lines), row["node"]["stats"]);
}
fn intrinsic_shape(node: &Value) {
    let allowed = [
        "activeEffectImage",
        "ascendancyName",
        "connections",
        "flavourText",
        "group",
        "icon",
        "isNotable",
        "name",
        "nodeOverlay",
        "orbit",
        "orbitIndex",
        "recipe",
        "skill",
        "stats",
        "stringId",
    ];
    assert!(
        node.as_object()
            .unwrap()
            .keys()
            .all(|k| allowed.contains(&k.as_str())),
        "unreviewed intrinsic/alternate node field"
    );
    assert!(node["skill"].as_u64().is_some());
    assert!(
        node["stats"]
            .as_array()
            .is_some_and(|r| !r.is_empty() && r.iter().all(Value::is_string))
    );
    assert!(node["connections"].is_array());
}
fn no_class_start_injection(row: &Value, tree: &Value) {
    let id = row["node"]["skill"].as_u64().unwrap();
    let nodes = tree["nodes"].as_object().unwrap();
    let selected = &row["node"];
    let mut neighbors = BTreeSet::new();
    for link in selected["connections"].as_array().unwrap() {
        neighbors.insert(link["id"].as_u64().unwrap());
    }
    for node in nodes.values() {
        if node["connections"]
            .as_array()
            .is_some_and(|links| links.iter().any(|link| link["id"] == id))
        {
            neighbors.insert(node["skill"].as_u64().unwrap());
        }
    }
    for adjacent in neighbors {
        let node = nodes
            .get(&adjacent.to_string())
            .expect("every reference exists");
        assert!(node.get("classesStart").is_none());
    }
}
fn semantics(row: &Value, f: &Facts) {
    let lines = row["node"]["stats"].as_array().unwrap();
    let rows = row["semantics"].as_array().unwrap();
    if !row["grant"].is_null() {
        assert!(rows.is_empty());
        assert_eq!(lines.len(), 1);
        return;
    }
    assert_eq!(
        rows.len(),
        lines.len(),
        "every line needs meaning, even if the parser drops it"
    );
    for (semantic, line) in rows.iter().zip(lines) {
        assert_eq!(semantic["source"]["path"], DESCRIPTIONS);
        let body = span(&f.sources, &semantic["source"]);
        let template = string(&semantic["template"]);
        assert!(body.contains(&format!("text=\"{template}\"")));
        let ids = semantic["stat_ids"].as_array().unwrap();
        assert_eq!(ids.len(), 1);
        let stat = string(&ids[0]);
        let stat_body = body.split_once("\n\t\tstats={\n").unwrap().1;
        assert!(stat_body.contains(&format!("\"{stat}\"")));
        let (classification, required_template) = match stat {
            "puppet_master_effect_+%" => (
                "existing-buff-magnitude",
                "{0}% increased Effect of Puppet Master",
            ),
            "archon_effect_on_you_+%" => (
                "existing-buff-magnitude",
                "{0}% increased effect of Archon Buffs on you",
            ),
            "minion_commanded_skill_damage_+%_per_different_persistent_minion_in_presence" => (
                "existing-minion-damage",
                "Minions deal {0}% increased Damage with Command Skills for each different type of Persistent Minion in your Presence",
            ),
            "base_mana_cost_efficiency_+%_of_command_skills" => (
                "existing-skill-cost",
                "{0}% increased Mana Cost Efficiency of Command Skills",
            ),
            "max_puppet_master_stacks_+" => (
                "existing-buff-capacity",
                "+{0} maximum stacks of Puppet Master",
            ),
            _ => panic!("unreviewed stat meaning cannot establish direct structural absence"),
        };
        assert_eq!(semantic["classification"], classification);
        assert_eq!(template, required_template);
        let value = semantic["value"].as_i64().unwrap();
        assert_eq!(template.replace("{0}", &value.to_string()), string(line));
    }
}
fn grant(row: &Value, f: &Facts) {
    let grant = &row["grant"];
    if grant.is_null() {
        return;
    }
    let prior = exact(
        &f.inputs["grant_bindings"]["families"],
        "/node_id",
        &row["node"]["skill"],
    );
    for field in ["source_effect", "skill", "grant", "supply"] {
        assert_eq!(grant[field], prior[field]);
    }
    let effect = &grant["source_effect"];
    let catalog = &f.inputs["catalog"];
    let skill = exact(&catalog["skills"], "/id", effect);
    assert_eq!(skill, &grant["skill_identity"]);
    assert_eq!(skill["from_tree"], true);
    assert_ne!(skill["support"], true);
    let declared = exact(
        &catalog["skill_declarations"],
        "/index",
        &skill["winning_declaration"],
    );
    assert_eq!(declared, &grant["skill_declaration"]);
    assert_eq!(declared["id"], *effect);
    let body = span(&f.sources, &declared["source"]);
    assert!(body.starts_with(&format!("skills[\"{}\"] = {{", string(effect))));
    assert!(body.contains("\tfromTree = true,"));
    assert_eq!(
        row["node"]["stats"],
        json!([format!("Grants Skill: {}", string(&skill["name"]))])
    );
    let lookup_matches = catalog["skills"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| string(&r["name"]).to_lowercase() == string(&skill["name"]).to_lowercase())
        .count();
    assert_eq!(lookup_matches, 1);
    let mapping = &grant["skill_mapping"];
    assert_eq!(
        mapping["source"],
        json!({"kind":"definition","value":{"kind":"skill","value":{
            "effect_id":{"kind":"text","value":effect}
        }}})
    );
    assert_eq!(mapping["outcome"]["kind"], "mapped");
    assert_eq!(
        mapping["outcome"]["value"]["basis"],
        json!({"kind":"exact"})
    );
    assert_eq!(
        mapping["outcome"]["value"]["target"],
        json!({
            "kind":"definition","value":{"kind":"skill","value":grant["skill"]}
        })
    );
    assert_eq!(
        row["recipe"]["stats"],
        json!([{
            "line":row["node"]["stats"][0], "remainder":null,
            "mods":{"[1]":{"flags":0,"keywordFlags":0,"name":"ExtraSkill","type":"LIST",
                "value":{"level":1,"skillId":effect}}}
        }])
    );
    let dep = read("dependencies.json");
    for field in ["grant", "supply"] {
        let slot = exact(&dep["slots"], "/value/id", &grant[field]);
        assert_eq!(
            slot["value"]["id"]["declaration"]["definition"]["key"],
            row["passive"]
        );
    }
    let supply = exact(&dep["slots"], "/value/id", &grant["supply"]);
    assert_eq!(supply["value"]["schema"]["value"]["skill"], grant["skill"]);
    // The summon Skill owns its descendants. This packet never closes them.
    assert_eq!(
        supply["value"]["schema"]["value"]["outputs"]["closure"]["kind"],
        "partial"
    );
}
fn census(proof: &Value) {
    assert_eq!(proof["candidate_census_complete"], true);
    assert_eq!(proof["candidate_closure_complete"], false);
    let mut keys = BTreeSet::new();
    for (field, expected) in [("families", FAMILIES), ("unresolved", EXCLUDED)] {
        let rows = proof[field].as_array().unwrap();
        assert_eq!(rows.len(), expected.len());
        for (row, (node, owned)) in rows.iter().zip(expected) {
            assert_eq!(row["node_id"], node.to_string());
            assert_eq!(row["passive"], owned);
            assert!(keys.insert(owned));
            if field == "unresolved" {
                assert_eq!(
                    row["reason"],
                    if [10561, 23265].contains(&node) {
                        "unresolved-djinn-command-grant"
                    } else {
                        "missing-pinned-buff-payload"
                    }
                );
            }
        }
    }
    assert_eq!(keys.len(), 12);
}
pub(super) fn check() {
    let f = facts();
    census(&f.proof);
    let mut source_lines = 0;
    let mut remainder_lines = 0;
    for row in f.proof["families"].as_array().unwrap() {
        identity(row, f);
        intrinsic_shape(&row["node"]);
        no_class_start_injection(row, &f.tree);
        semantics(row, f);
        grant(row, f);
        source_lines += row["recipe"]["stats"].as_array().unwrap().len();
        remainder_lines += row["recipe"]["stats"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| !r["remainder"].is_null())
            .count();
    }
    assert_eq!((source_lines, remainder_lines), (7, 4));
    for row in f.proof["unresolved"].as_array().unwrap() {
        identity(row, f);
    }
}
pub(super) fn identity_controls() {
    let f = facts();
    let row = &f.proof["families"][0];
    identity(row, f);
    for pointer in [
        "/mapping/source/value/value/view",
        "/mapping/source/value/value/tree_version",
        "/mapping/source/value/value/node_id",
    ] {
        let mut bad = row.clone();
        *bad.pointer_mut(pointer).unwrap() = json!({"kind":"text","value":"unreviewed"});
        rejected(|| identity(&bad, f));
    }
    let mut bad = row.clone();
    bad["grant"]["source_effect"] = json!("unknown-child");
    rejected(|| grant(&bad, f));
    let mut bad = row.clone();
    bad["grant"]["skill"] = f.proof["families"][2]["grant"]["skill"].clone();
    rejected(|| grant(&bad, f));
}
pub(super) fn semantic_controls() {
    let f = facts();
    for index in [1, 3, 4, 5] {
        let row = &f.proof["families"][index];
        semantics(row, f);
        let mut omitted = row.clone();
        omitted["semantics"].as_array_mut().unwrap().pop();
        rejected(|| semantics(&omitted, f));
        let mut unknown = row.clone();
        unknown["semantics"][0]["stat_ids"] = json!(["unknown_factory"]);
        rejected(|| semantics(&unknown, f));
        let mut dropped = row.clone();
        dropped["recipe"]["stats"][0]["remainder"] = Value::Null;
        // Even a cleared parser remainder cannot change the reviewed node meaning.
        dropped["node"]["stats"][0] = json!("Grants Skill: an unreviewed ability");
        rejected(|| semantics(&dropped, f));
    }
}
pub(super) fn structural_controls() {
    let f = facts();
    let row = &f.proof["families"][0];
    for field in [
        "masteryEffects",
        "grantedSkills",
        "addMinionList",
        "options",
        "isJewelSocket",
    ] {
        let mut node = row["node"].clone();
        node[field] = json!({"unreviewed":"factory"});
        rejected(|| intrinsic_shape(&node));
    }
    let mut extra = row.clone();
    extra["node"]["stats"]
        .as_array_mut()
        .unwrap()
        .push(json!("Grants Skill: unknown"));
    rejected(|| grant(&extra, f));
    let mut removed = row.clone();
    removed["recipe"]["stats"][0]["mods"] = json!({});
    rejected(|| grant(&removed, f));
    let mut node = f.proof["families"][3].clone();
    node["node"]["stats"]
        .as_array_mut()
        .unwrap()
        .push(json!("unknown intrinsic"));
    rejected(|| semantics(&node, f));
}
pub(super) fn census_controls() {
    let f = facts();
    census(&f.proof);
    for field in ["families", "unresolved"] {
        let mut omitted = f.proof.clone();
        omitted[field].as_array_mut().unwrap().pop();
        rejected(|| census(&omitted));
    }
    let mut promoted = f.proof.clone();
    promoted["families"][1] = promoted["unresolved"][1].clone();
    rejected(|| census(&promoted));
    let mut complete = f.proof.clone();
    complete["candidate_closure_complete"] = json!(true);
    rejected(|| census(&complete));
}
