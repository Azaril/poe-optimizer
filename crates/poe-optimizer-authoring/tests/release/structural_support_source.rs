//! Bounded intrinsic-source proof; rejected parser rows never prove absence.
use flate2::read::GzDecoder;
use poe_optimizer_data::skill_identities::{SkillIdentityCatalog, SkillIdentityData};
use poe_optimizer_import::{
    owned_mapping::MappingPackageInput, owned_tree_catalog::TreeCatalogInput,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs::File,
    io::Read,
    path::PathBuf,
    sync::OnceLock,
};

const BASE: &str = "data/owned/poe2/3887ae68/";
const EVIDENCE_HASH: &str = "78b0103cfb95b07b2d39e3ce32d36cb2871eaa1c2c0cac9a0bb3cc77b9f5c0a1";
const MANIFEST_HASH: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const TREE: &str = "src/TreeData/0_5/tree.json";
const TREE_LUA: &str = "src/TreeData/0_5/tree.lua";
const TREE_HASH: &str = "6449fe534c0265b21f59f8213254bd3f37a445887582c40aab04ad11cede3e95";
const DESCRIPTIONS: &str = "src/Data/StatDescriptions/stat_descriptions.lua";
pub(super) const SOURCES: &[(u64, &str)] = &[
    (13289, "def.0000000000000b34"),
    (19318, "def.0000000000000d0f"),
    (32705, "def.00000000000010d4"),
    (35560, "def.00000000000011c2"),
    (45333, "def.00000000000014d8"),
    (62210, "def.00000000000019ec"),
];
const EXCLUDED: &[(u64, &str)] = &[
    (10561, "def.0000000000000a6d"),
    (14686, "def.0000000000000bb3"),
    (23265, "def.0000000000000e15"),
    (26926, "def.0000000000000f2a"),
    (53324, "def.000000000000173f"),
    (53795, "def.0000000000001760"),
];
struct Facts {
    proof: Value,
    inputs: BTreeMap<String, Value>,
    sources: BTreeMap<String, String>,
    tree: Value,
    additions: Value,
    mapping: MappingPackageInput,
    catalog: TreeCatalogInput,
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn string(value: &Value) -> &str {
    value.as_str().unwrap()
}
fn bounded(path: &str, limit: usize) -> Vec<u8> {
    let file = File::open(root().join(path)).unwrap();
    assert!(file.metadata().unwrap().len() <= limit as u64);
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes).unwrap();
    assert!(bytes.len() <= limit);
    bytes
}
fn pin(bytes: &[u8], record: &Value) {
    assert_eq!(record["bytes"], bytes.len());
    assert_eq!(record["sha256"], hash(bytes));
}
fn exact<'a>(rows: &'a Value, pointer: &str, id: &Value) -> &'a Value {
    let matches: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row.pointer(pointer) == Some(id))
        .collect();
    assert_eq!(matches.len(), 1, "one exact retained identity");
    matches[0]
}
fn rejected(f: impl FnOnce() + std::panic::UnwindSafe) {
    assert!(std::panic::catch_unwind(f).is_err());
}
fn facts() -> &'static Facts {
    static FACTS: OnceLock<Facts> = OnceLock::new();
    FACTS.get_or_init(|| {
        let bytes = bounded(
            &format!("{BASE}structural-support-domains/evidence.json"),
            128 * 1024,
        );
        assert_eq!(hash(&bytes), EVIDENCE_HASH);
        let additions: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(additions["schema_version"], 1);
        assert_eq!(additions["source_revision"], REVISION);
        let manifest_pin = &additions["source_manifest"];
        assert_eq!(
            manifest_pin["path"],
            "crates/poe-optimizer-pob/data/pob-source-manifest.json"
        );
        assert_eq!(manifest_pin["sha256"], MANIFEST_HASH);
        let bytes = bounded(string(&manifest_pin["path"]), 1024 * 1024);
        pin(&bytes, manifest_pin);
        let manifest: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(manifest["upstream_revision"], REVISION);
        let mut inputs = BTreeMap::from([("source_manifest".to_owned(), manifest)]);
        assert_eq!(additions["artifacts"].as_array().unwrap().len(), 10);
        for record in additions["artifacts"].as_array().unwrap() {
            let bytes = bounded(string(&record["path"]), 4 * 1024 * 1024);
            pin(&bytes, record);
            assert!(
                inputs
                    .insert(
                        string(&record["name"]).to_owned(),
                        serde_json::from_slice(&bytes).unwrap()
                    )
                    .is_none()
            );
        }
        let proof = inputs["retained_evidence"].clone();
        assert_eq!(proof["source_revision"], REVISION);
        for name in [
            "recipes",
            "selected_bindings",
            "catalog",
            "grant_bindings",
            "grant_authority",
        ] {
            let record = exact(&additions["artifacts"], "/name", &json!(name));
            for field in ["path", "bytes", "sha256"] {
                assert_eq!(proof[name][field], record[field]);
            }
        }
        let skills: SkillIdentityData = serde_json::from_value(inputs["catalog"].clone()).unwrap();
        SkillIdentityCatalog::new(skills).unwrap();
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
        assert_eq!(additions["files"].as_array().unwrap().len(), 10);
        for record in additions["files"].as_array().unwrap() {
            let compressed = bounded(string(&record["archive"]), 2 * 1024 * 1024);
            assert_eq!(record["archive_bytes"], compressed.len());
            assert_eq!(record["archive_sha256"], hash(&compressed));
            let mut bytes = Vec::new();
            GzDecoder::new(compressed.as_slice())
                .take(8 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .unwrap();
            total += bytes.len();
            assert!(bytes.len() <= 8 * 1024 * 1024 && total <= 16 * 1024 * 1024);
            pin(&bytes, record);
            assert!(!bytes.contains(&b'\r'));
            let path = string(&record["path"]);
            if path == TREE {
                assert_eq!(record["sha256"], TREE_HASH);
            } else {
                pin(
                    &bytes,
                    exact(
                        &inputs["source_manifest"]["files"],
                        "/path",
                        &record["path"],
                    ),
                );
            }
            assert!(
                sources
                    .insert(path.to_owned(), String::from_utf8(bytes).unwrap())
                    .is_none()
            );
        }
        let facts = Facts {
            tree: serde_json::from_str(&sources[TREE]).unwrap(),
            mapping: serde_json::from_value(inputs["mapping"].clone()).unwrap(),
            catalog: serde_json::from_value(inputs["tree_catalog"].clone()).unwrap(),
            proof,
            inputs,
            sources,
            additions,
        };
        check_boundaries(&facts);
        check_facts(&facts);
        facts
    })
}
pub(super) fn evidence() -> &'static Value {
    &facts().proof
}
pub(super) fn dependencies() -> &'static Value {
    &facts().inputs["dependencies"]
}
pub(super) fn migration() -> &'static Value {
    &facts().inputs["migration"]
}
pub(super) fn mapping() -> &'static MappingPackageInput {
    &facts().mapping
}
pub(super) fn catalog() -> &'static TreeCatalogInput {
    &facts().catalog
}

fn added_span(f: &Facts, record: &Value) -> String {
    let first = record["first"].as_u64().unwrap() as usize;
    let last = record["last"].as_u64().unwrap() as usize;
    let lines: Vec<_> = f.sources[string(&record["path"])].lines().collect();
    assert!(first > 0 && last >= first && last <= lines.len() && last - first < 1000);
    let text = lines[first - 1..last].join("\n") + "\n";
    assert_eq!(record["sha256"], hash(text.as_bytes()));
    text
}
fn scalar_literal(value: &Value) -> String {
    match value {
        Value::String(text) => {
            // This finite witness needs only ASCII strings and escaped newlines.
            assert!(
                text.chars()
                    .all(|c| c.is_ascii() && (!c.is_ascii_control() || c == '\n'))
            );
            serde_json::to_string(text).unwrap()
        }
        Value::Number(number) => number.as_i64().unwrap().to_string(),
        Value::Bool(value) => value.to_string(),
        _ => panic!("unreviewed literal type"),
    }
}
/// Fixed field/shape encoder for the reviewed six records, never a Lua parser.
/// Comparing the whole literal checks every raw field and the outer boundary.
fn node_literal(node: &Value) -> String {
    intrinsic_shape(node);
    let mut text = format!("\t\t[{}]={{\n", node["skill"].as_u64().unwrap());
    for field in [
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
    ] {
        let Some(value) = node.get(field) else {
            continue;
        };
        match field {
            "flavourText" => {
                // The retained JSON presentation field spells line breaks as
                // backslash-n; the executed Lua literal uses newline escapes.
                // This field-specific encoding projection grants no mechanics.
                let text_value = string(value).replace("\\n", "\n");
                assert!(!text_value.contains('\\'));
                writeln!(
                    text,
                    "\t\t\tflavourText={},",
                    scalar_literal(&json!(text_value))
                )
                .unwrap();
            }
            "connections" => {
                text.push_str("\t\t\tconnections={\n");
                let rows = value.as_array().unwrap();
                for (i, row) in rows.iter().enumerate() {
                    assert_eq!(
                        row.as_object()
                            .unwrap()
                            .keys()
                            .map(String::as_str)
                            .collect::<BTreeSet<_>>(),
                        BTreeSet::from(["id", "orbit"])
                    );
                    let comma = if i + 1 == rows.len() { "" } else { "," };
                    writeln!(
                        text,
                        "\t\t\t\t[{}]={{\n\t\t\t\t\tid={},\n\t\t\t\t\torbit={}\n\t\t\t\t}}{comma}",
                        i + 1,
                        scalar_literal(&row["id"]),
                        scalar_literal(&row["orbit"])
                    )
                    .unwrap();
                }
                text.push_str("\t\t\t},\n");
            }
            "nodeOverlay" => {
                assert_eq!(
                    value
                        .as_object()
                        .unwrap()
                        .keys()
                        .map(String::as_str)
                        .collect::<BTreeSet<_>>(),
                    BTreeSet::from(["alloc", "path", "unalloc"])
                );
                text.push_str("\t\t\tnodeOverlay={\n");
                for key in ["alloc", "path", "unalloc"] {
                    let comma = if key == "unalloc" { "" } else { "," };
                    writeln!(text, "\t\t\t\t{key}={}{comma}", scalar_literal(&value[key])).unwrap();
                }
                text.push_str("\t\t\t},\n");
            }
            "recipe" | "stats" => {
                writeln!(text, "\t\t\t{field}={{").unwrap();
                let rows = value.as_array().unwrap();
                for (i, row) in rows.iter().enumerate() {
                    assert!(row.is_string());
                    let comma = if i + 1 == rows.len() { "" } else { "," };
                    writeln!(text, "\t\t\t\t[{}]={}{comma}", i + 1, scalar_literal(row)).unwrap();
                }
                text.push_str("\t\t\t},\n");
            }
            _ => {
                let comma = if field == "stringId" { "" } else { "," };
                writeln!(text, "\t\t\t{field}={}{comma}", scalar_literal(value)).unwrap();
            }
        }
    }
    text.push_str("\t\t},\n");
    text
}
fn catalog_identity(row: &Value, catalog: &Value) {
    let id = &row["node_id"];
    let node = exact(&catalog["nodes"], "/key", id);
    let pool = if row["grant"].is_null() {
        "ordinary"
    } else {
        "ascendancy"
    };
    assert_eq!(
        *node,
        json!({"key":id,"kind":{"kind":"allocation","value":{"pool":pool}},
        "stats":row["node"]["stats"],"views":[],"unlock":[]})
    );
    assert!(
        catalog["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|node| node["kind"]["value"]["parent"] != *id),
        "no attached choice"
    );
}
fn check_executed_nodes(f: &Facts) {
    let nodes = f.additions["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), SOURCES.len());
    for (pin, row) in nodes.iter().zip(f.proof["families"].as_array().unwrap()) {
        assert_eq!(pin["id"], row["node_id"]);
        let record =
            json!({"path":TREE_LUA,"first":pin["first"],"last":pin["last"],"sha256":pin["sha256"]});
        assert_eq!(added_span(f, &record), node_literal(&row["node"]));
        catalog_identity(row, &f.inputs["tree_catalog"]);
        let mapping = exact(
            &f.inputs["mapping"]["entries"],
            "/source",
            &row["mapping"]["source"],
        );
        assert_eq!(mapping, &row["mapping"]);
    }
}
fn check_support_boundaries(f: &Facts) {
    let spans = f.additions["spans"].as_array().unwrap();
    assert_eq!(spans.len(), 27);
    let mut roles = BTreeSet::new();
    for record in spans {
        assert!(roles.insert(string(&record["role"])));
        let _ = added_span(f, record);
    }
    let body = |role| added_span(f, exact(&f.additions["spans"], "/role", &json!(role)));
    assert!(body("tree-constructor").starts_with("function PassiveTreeClass:PassiveTree("));
    assert!(body("process-node").starts_with("function PassiveTreeClass:ProcessNode("));
    let extra = body("extra-support-contrast");
    assert!(extra.starts_with("local function extraSupport("));
    assert!(extra.ends_with("end\n"));
    assert!(extra.contains("mod(\"ExtraSupport\", \"LIST\""));
    assert!(extra.contains("if additional.support then"));
    assert!(extra.contains("mod(\"ExtraSkill\", \"LIST\""));
    assert!(body("linked-support-contrast").contains("mod(\"LinkedSupport\", \"LIST\""));
    assert!(body("extra-minion-skill-contrast").contains("mod(\"ExtraMinionSkill\", \"LIST\""));
    let setup = &f.sources["src/Modules/CalcSetup.lua"];
    assert!(setup.contains("\"LinkedSupport\""));
    assert!(setup.contains("\"ExtraSupport\""));
    // The authenticated constructor separates these externally supplied origins
    // from the complete intrinsic node bodies; none is certified by this packet.
    let grant = &f.proof["families"][0]["recipe"]["stats"][0]["mods"]["[1]"];
    assert_eq!(grant["name"], "ExtraSkill");
    assert_ne!(grant["name"], "ExtraSupport");
}

#[test]
fn authenticated_six_source_domains_preserve_positive_grants_and_all_partial_candidates() {
    let f = facts();
    check_facts(f);
    assert_eq!(f.proof["families"].as_array().unwrap().len(), 6);
    assert_eq!(f.proof["unresolved"].as_array().unwrap().len(), 6);
}

#[test]
fn executed_whole_node_and_catalog_controls_refuse_added_behavior_and_wrong_pools() {
    let f = facts();
    for row in f.proof["families"].as_array().unwrap() {
        let mut raw = row["node"].clone();
        raw["callbacks"] = json!([]);
        rejected(|| {
            node_literal(&raw);
        });
        let mut raw = row["node"].clone();
        raw["stats"][0] = json!("Socketed Gems are Supported by Level 1 Unknown");
        assert_ne!(node_literal(&raw), node_literal(&row["node"]));
        let mut catalog = f.inputs["tree_catalog"].clone();
        let node = catalog["nodes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|node| node["key"] == row["node_id"])
            .unwrap();
        node["kind"]["value"]["pool"] = json!("unreviewed");
        rejected(|| catalog_identity(row, &catalog));
        let mut catalog = f.inputs["tree_catalog"].clone();
        catalog["nodes"].as_array_mut().unwrap().push(
            json!({"key":"extra","kind":{"kind":"choice","value":{"parent":row["node_id"]}}}),
        );
        rejected(|| catalog_identity(row, &catalog));
    }
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
    let dep = &f.inputs["dependencies"];
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
    for (field, expected) in [("families", SOURCES), ("unresolved", EXCLUDED)] {
        let rows = proof[field].as_array().unwrap();
        assert_eq!(rows.len(), expected.len());
        for (row, &(node, owned)) in rows.iter().zip(expected) {
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
fn check_facts(f: &Facts) {
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
    check_executed_nodes(f);
    check_support_boundaries(f);
    for row in f.proof["unresolved"].as_array().unwrap() {
        identity(row, f);
    }
}
#[test]
fn identity_controls() {
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
#[test]
fn semantic_controls() {
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
#[test]
fn structural_controls() {
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
#[test]
fn census_controls() {
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
