//! Finite, offline source evidence for every attribute-choice intrinsic domain.
//! This authenticates reviewed source; it does not parse or execute Lua.
use flate2::read::GzDecoder;
use poe_optimizer_import::{
    owned_mapping::MappingPackageInput,
    owned_tree_catalog::{TreeCatalogInput, TreeNodeKind, TreePoolKind},
    owned_tree_policy::TreeNormalizationPackageInput,
};
use serde::Deserialize;
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
const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const MANIFEST: &str = "crates/poe-optimizer-pob/data/pob-source-manifest.json";
const MANIFEST_HASH: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const TREE_LUA: &str = "src/TreeData/0_5/tree.lua";
const TREE_JSON: &str = "src/TreeData/0_5/tree.json";
const RAW_JSON_HASH: &str = "6449fe534c0265b21f59f8213254bd3f37a445887582c40aab04ad11cede3e95";
const FILES: &[(&str, &str)] = &[
    (TREE_LUA, "empty-passive-socket-intrinsics/tree.lua.gz"),
    (
        TREE_JSON,
        "passive-structural-inventories/sources/src_TreeData_0_5_tree.json.gz",
    ),
    (
        "src/Classes/PassiveTree.lua",
        "passive-structural-inventories/sources/src_Classes_PassiveTree.lua.gz",
    ),
    (
        "src/Classes/PassiveSpec.lua",
        "passive-structural-inventories/sources/src_Classes_PassiveSpec.lua.gz",
    ),
    (
        "src/Modules/ModParser.lua",
        "support-structural-inventories/sources/src_Modules_ModParser.lua.gz",
    ),
];
const SPANS: &[(&str, &str, usize, usize)] = &[
    ("tree-constructor", "src/Classes/PassiveTree.lua", 59, 446),
    ("process-stats", "src/Classes/PassiveTree.lua", 448, 525),
    ("process-node", "src/Classes/PassiveTree.lua", 528, 595),
    ("saved-choices", "src/Classes/PassiveSpec.lua", 224, 245),
    ("import-overrides", "src/Classes/PassiveSpec.lua", 381, 387),
    (
        "applied-override-and-external-boundary",
        "src/Classes/PassiveSpec.lua",
        1570,
        1578,
    ),
    ("replace-node", "src/Classes/PassiveSpec.lua", 2047, 2063),
    (
        "switch-attribute",
        "src/Classes/PassiveSpec.lua",
        2718,
        2729,
    ),
    ("modifier-forms", "src/Modules/ModParser.lua", 62, 158),
    (
        "attribute-modifier-names",
        "src/Modules/ModParser.lua",
        160,
        171,
    ),
    (
        "attribute-default-empty",
        "src/Modules/ModParser.lua",
        6289,
        6289,
    ),
    ("modifier-scan", "src/Modules/ModParser.lua", 6592, 6614),
    ("modifier-parser", "src/Modules/ModParser.lua", 6619, 7036),
    ("modifier-cache", "src/Modules/ModParser.lua", 7399, 7423),
    (
        "extra-support-contrast",
        "src/Modules/ModParser.lua",
        2219,
        2253,
    ),
    (
        "linked-support-contrast",
        "src/Modules/ModParser.lua",
        3650,
        3659,
    ),
];
// Fixed historical inputs, not hashes supplied by the production domain table.
const ARTIFACTS: &[(&str, usize, &str)] = &[
    (
        "tree/tree-catalog.json",
        1_676_797,
        "37db1255f28d05f67a6d81dfe286bcc698a9811e907c67d29cf640d9c28e5557",
    ),
    (
        "current/mapping.json",
        3_165_465,
        "3b627b9e7195ba18a44f83a73aff8fbffc238162ccfc273992fc129179cdb1a4",
    ),
    (
        "current/tree-normalization.json",
        1_618_228,
        "6384b05d546ea38bb48227594755d3bcaa66eeff42fad9ea8727f022786a5be1",
    ),
    (
        "attribute-stages/count-bindings.json",
        3100,
        "4b6501509beebe49e0da197f657f57df4ac86341e1f7f04e84a87990c9e7bc71",
    ),
];
const OPTIONS: &[(&str, u32, &str, &str)] = &[
    ("strNodes", 26297, "Strength", "Str"),
    ("dexNodes", 14927, "Dexterity", "Dex"),
    ("intNodes", 57022, "Intelligence", "Int"),
];

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pin {
    path: String,
    bytes: usize,
    sha256: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourcePin {
    path: String,
    archive: String,
    archive_bytes: usize,
    archive_sha256: String,
    bytes: usize,
    sha256: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Span {
    role: String,
    path: String,
    first: usize,
    last: usize,
    sha256: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeSpan {
    key: String,
    first: usize,
    last: usize,
    sha256: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    schema_version: u32,
    source_revision: String,
    source_manifest: Pin,
    scope: String,
    files: Vec<SourcePin>,
    spans: Vec<Span>,
    artifacts: Vec<Pin>,
    nodes: Vec<NodeSpan>,
}
// These DTOs describe the exact finite reviewed JSON records only. Denying extra
// fields prevents an overlooked raw override/callback from passing the witness.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawAttribute {
    connections: Vec<Connection>,
    group: u32,
    icon: String,
    is_attribute: bool,
    name: String,
    options: Vec<RawOption>,
    orbit: u32,
    orbit_index: u32,
    skill: u32,
    stats: Vec<String>,
    string_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Connection {
    id: u32,
    orbit: i32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOption {
    icon: String,
    id: u32,
    name: String,
    stats: Vec<String>,
}
struct Proof {
    catalog: TreeCatalogInput,
    mapping: MappingPackageInput,
    tree: TreeNormalizationPackageInput,
    count_bindings: Value,
    spans: BTreeMap<String, String>,
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn require(ok: bool, message: &str) -> Result<(), String> {
    ok.then_some(()).ok_or_else(|| message.to_owned())
}
fn bounded(path: &str, limit: usize) -> Result<Vec<u8>, String> {
    let file = File::open(root().join(path)).map_err(|e| e.to_string())?;
    require(
        file.metadata().map_err(|e| e.to_string())?.len() <= limit as u64,
        "file bound",
    )?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    require(bytes.len() <= limit, "file grew beyond bound")?;
    Ok(bytes)
}
fn pinned(pin: &Pin, limit: usize) -> Result<Vec<u8>, String> {
    require(pin.bytes <= limit, "pin bound")?;
    let bytes = bounded(&pin.path, limit)?;
    require(
        bytes.len() == pin.bytes && hash(&bytes) == pin.sha256,
        "artifact pin",
    )?;
    Ok(bytes)
}
fn evidence() -> Evidence {
    serde_json::from_slice(
        &bounded(
            &format!("{BASE}attribute-support-domains/evidence.json"),
            131072,
        )
        .unwrap(),
    )
    .unwrap()
}
fn span(lines: &[&str], first: usize, last: usize, digest: &str) -> Result<String, String> {
    require(
        first > 0 && last >= first && last <= lines.len(),
        "span bounds",
    )?;
    let text = lines[first - 1..last].join("\n") + "\n";
    require(hash(text.as_bytes()) == digest, "source span digest")?;
    Ok(text)
}
fn reviewed_node(key: &str, value: &Value) -> Result<RawAttribute, String> {
    let node: RawAttribute = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    require(
        key == node.skill.to_string()
            && node.is_attribute
            && node.name == "Attribute"
            && node.icon == "Art/2DArt/SkillIcons/passives/plusattribute.dds"
            && node.stats == ["+5 to any Attribute"]
            && node.options.len() == OPTIONS.len()
            && node
                .string_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')),
        "attribute raw row",
    )?;
    for (option, (_, id, name, _)) in node.options.iter().zip(OPTIONS) {
        require(
            option.id == *id
                && option.name == *name
                && option.icon
                    == format!(
                        "Art/2DArt/SkillIcons/passives/plus{}.dds",
                        name.to_lowercase()
                    )
                && option.stats == [format!("+5 to {name}")],
            "all three raw options",
        )?;
    }
    Ok(node)
}
/// Render only the reviewed fixed row shape, then compare the WHOLE Lua literal.
/// No Lua is parsed, evaluated or accepted through a permissive fallback.
fn lua_literal(node: &RawAttribute) -> String {
    let mut text = format!("\t\t[{}]={{\n\t\t\tconnections={{\n", node.skill);
    for (i, connection) in node.connections.iter().enumerate() {
        let comma = if i + 1 == node.connections.len() {
            ""
        } else {
            ","
        };
        writeln!(
            text,
            "\t\t\t\t[{}]={{\n\t\t\t\t\tid={},\n\t\t\t\t\torbit={}\n\t\t\t\t}}{comma}",
            i + 1,
            connection.id,
            connection.orbit
        )
        .unwrap();
    }
    writeln!(text, "\t\t\t}},\n\t\t\tgroup={},\n\t\t\ticon=\"{}\",\n\t\t\tisAttribute=true,\n\t\t\tname=\"Attribute\",\n\t\t\toptions={{", node.group, node.icon).unwrap();
    for (i, option) in node.options.iter().enumerate() {
        let comma = if i + 1 == node.options.len() { "" } else { "," };
        writeln!(text, "\t\t\t\t[{}]={{\n\t\t\t\t\ticon=\"{}\",\n\t\t\t\t\tid={},\n\t\t\t\t\tname=\"{}\",\n\t\t\t\t\tstats={{\n\t\t\t\t\t\t[1]=\"{}\"\n\t\t\t\t\t}}\n\t\t\t\t}}{comma}", i + 1, option.icon, option.id, option.name, option.stats[0]).unwrap();
    }
    writeln!(text, "\t\t\t}},\n\t\t\torbit={},\n\t\t\torbitIndex={},\n\t\t\tskill={},\n\t\t\tstats={{\n\t\t\t\t[1]=\"+5 to any Attribute\"\n\t\t\t}},\n\t\t\tstringId=\"{}\"\n\t\t}},", node.orbit, node.orbit_index, node.skill, node.string_id).unwrap();
    text
}
fn verify_catalog(
    evidence: &Evidence,
    sources: &BTreeMap<String, String>,
    catalog: &TreeCatalogInput,
) -> Result<(), String> {
    let raw: Value = serde_json::from_str(&sources[TREE_JSON]).map_err(|e| e.to_string())?;
    let nodes = raw["nodes"].as_object().ok_or("raw nodes")?;
    let attributes: BTreeMap<_, _> = nodes
        .iter()
        .filter(|(_, n)| n["isAttribute"] == true)
        .collect();
    let lines: Vec<_> = sources[TREE_LUA].lines().collect();
    let mut witnessed = BTreeSet::new();
    for pin in &evidence.nodes {
        require(witnessed.insert(pin.key.clone()), "duplicate node witness")?;
        let raw = attributes.get(&pin.key).ok_or("non-attribute witness")?;
        let node = reviewed_node(&pin.key, raw)?;
        require(
            span(&lines, pin.first, pin.last, &pin.sha256)? == lua_literal(&node),
            "complete Lua/JSON row equality",
        )?;
    }
    require(
        witnessed == attributes.keys().map(|k| (*k).clone()).collect(),
        "complete raw node census",
    )?;
    require(
        lines
            .iter()
            .filter(|line| **line == "\t\t\tisAttribute=true,")
            .count()
            == witnessed.len(),
        "complete executed attribute literal census",
    )?;
    let mut projected = BTreeSet::new();
    for node in &catalog.nodes {
        if let TreeNodeKind::Attribute { pool } = &node.kind {
            require(
                *pool == TreePoolKind::Ordinary
                    && node.stats == ["+5 to any Attribute"]
                    && node.views.is_empty()
                    && node.unlock.is_empty(),
                "catalog attribute shape",
            )?;
            require(
                projected.insert(node.key.clone()),
                "duplicate catalog attribute",
            )?;
        }
    }
    require(projected == witnessed, "catalog/raw identity equality")?;
    require(
        catalog.attribute_options.len() == OPTIONS.len(),
        "catalog option count",
    )?;
    for (option, (key, _, name, _)) in catalog.attribute_options.iter().zip(OPTIONS) {
        require(
            option.key == *key && option.stats == [format!("+5 to {name}")],
            "catalog all-option equality",
        )?;
    }
    // Census is supplementary: set equality and every complete body above carry
    // the proof, rather than a historical layout count alone.
    require(projected.len() == 293, "reviewed family census")
}
fn authenticate(evidence: &Evidence) -> Result<Proof, String> {
    require(
        evidence.schema_version == 1
            && evidence.source_revision == REVISION
            && !evidence.scope.is_empty(),
        "revision",
    )?;
    require(
        evidence.source_manifest.path == MANIFEST
            && evidence.source_manifest.sha256 == MANIFEST_HASH,
        "manifest authority",
    )?;
    let manifest: Value = serde_json::from_slice(&pinned(&evidence.source_manifest, 262144)?)
        .map_err(|e| e.to_string())?;
    require(
        manifest["upstream_revision"] == REVISION,
        "manifest revision",
    )?;
    require(
        evidence.files.len() == FILES.len()
            && evidence.spans.len() == SPANS.len()
            && evidence.artifacts.len() == ARTIFACTS.len(),
        "fixed evidence inventory",
    )?;
    let mut sources = BTreeMap::new();
    for (pin, (path, archive)) in evidence.files.iter().zip(FILES) {
        require(
            pin.path == *path && pin.archive == format!("{BASE}{archive}"),
            "source inventory",
        )?;
        require(
            pin.bytes <= 4 * 1024 * 1024 && pin.archive_bytes <= 524288,
            "source bounds",
        )?;
        let compressed = bounded(&pin.archive, 524288)?;
        require(
            compressed.len() == pin.archive_bytes && hash(&compressed) == pin.archive_sha256,
            "archive pin",
        )?;
        let mut bytes = Vec::new();
        GzDecoder::new(compressed.as_slice())
            .take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        require(
            bytes.len() == pin.bytes && hash(&bytes) == pin.sha256,
            "decoded source pin",
        )?;
        if pin.path == TREE_JSON {
            // The source manifest inventories Lua. This sibling raw JSON has its
            // own fixed pin; complete row equality connects it to executed Lua.
            require(
                pin.bytes == 2_049_036 && pin.sha256 == RAW_JSON_HASH,
                "raw JSON authority",
            )?;
        } else {
            let expected = manifest["files"]
                .as_array()
                .ok_or("manifest files")?
                .iter()
                .find(|row| row["path"] == pin.path)
                .ok_or("source manifest entry")?;
            require(
                expected["bytes"] == pin.bytes && expected["sha256"] == pin.sha256,
                "manifest source pin",
            )?;
        }
        let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        require(!text.contains('\r'), "source line endings")?;
        sources.insert(pin.path.clone(), text);
    }
    let mut spans = BTreeMap::new();
    for (pin, (role, path, first, last)) in evidence.spans.iter().zip(SPANS) {
        require(
            pin.role == *role && pin.path == *path && pin.first == *first && pin.last == *last,
            "fixed span inventory",
        )?;
        spans.insert(
            pin.role.clone(),
            span(
                &sources[&pin.path].lines().collect::<Vec<_>>(),
                pin.first,
                pin.last,
                &pin.sha256,
            )?,
        );
    }
    let mut artifacts = Vec::new();
    for (pin, (path, bytes, digest)) in evidence.artifacts.iter().zip(ARTIFACTS) {
        require(
            pin.path == format!("{BASE}{path}") && pin.bytes == *bytes && pin.sha256 == *digest,
            "fixed artifact authority",
        )?;
        artifacts.push(pinned(pin, 4 * 1024 * 1024)?);
    }
    let catalog = serde_json::from_slice(&artifacts[0]).map_err(|e| e.to_string())?;
    verify_catalog(evidence, &sources, &catalog)?;
    Ok(Proof {
        catalog,
        mapping: serde_json::from_slice(&artifacts[1]).map_err(|e| e.to_string())?,
        tree: serde_json::from_slice(&artifacts[2]).map_err(|e| e.to_string())?,
        count_bindings: serde_json::from_slice(&artifacts[3]).map_err(|e| e.to_string())?,
        spans,
    })
}
fn proof() -> &'static Proof {
    static PROOF: OnceLock<Proof> = OnceLock::new();
    PROOF.get_or_init(|| authenticate(&evidence()).unwrap())
}
pub(super) fn verified_catalog() -> &'static TreeCatalogInput {
    &proof().catalog
}
pub(super) fn verified_mapping() -> &'static MappingPackageInput {
    &proof().mapping
}
pub(super) fn verified_tree() -> &'static TreeNormalizationPackageInput {
    &proof().tree
}
pub(super) fn verified_count_bindings() -> &'static Value {
    &proof().count_bindings
}

#[test]
fn whole_attribute_family_and_all_options_are_authenticated_independently() {
    let catalog = verified_catalog();
    assert_eq!(catalog.tree_version, "0_5");
    assert_eq!(catalog.source.revision, REVISION);
    assert_eq!(catalog.attribute_options.len(), 3);
}

#[test]
fn complete_constructor_and_choice_dispatch_preserve_intrinsic_origin_boundary() {
    let spans = &proof().spans;
    for (role, first) in [
        ("tree-constructor", "function PassiveTreeClass:PassiveTree("),
        ("process-stats", "function PassiveTreeClass:ProcessStats("),
        ("process-node", "function PassiveTreeClass:ProcessNode("),
        ("replace-node", "function PassiveSpecClass:ReplaceNode("),
        (
            "switch-attribute",
            "function PassiveSpecClass:SwitchAttributeNode(",
        ),
        ("modifier-scan", "local function scan("),
        ("modifier-parser", "local function parseMod("),
    ] {
        assert!(spans[role].starts_with(first) && spans[role].ends_with("end\n"));
    }
    assert!(
        spans["tree-constructor"]
            .contains("io.open(\"TreeData/\"..treeVersion..\"/tree.lua\", \"r\")")
    );
    assert!(spans["tree-constructor"].contains("node.modList:NewMod(\"Condition:ConnectedTo\"..class.name..\"Start\", \"FLAG\", true, \"Tree:\"..nodeId)"));
    assert!(spans["process-node"].contains("setmetatable(switchNode, { __index = node })"));
    assert!(spans["process-node"].contains("switchNode.id = node.id"));
    assert!(spans["process-node"].contains("switchNode.sd = switchNode.stats"));
    assert!(spans["process-node"].contains("self:ProcessStats(switchNode)"));
    for (index, (key, _, name, _)) in OPTIONS.iter().enumerate() {
        assert!(spans["saved-choices"].contains(&format!("child.attrib.{key}")));
        assert!(spans["saved-choices"].contains(&format!(
            "self:SwitchAttributeNode(tonumber({}Id), {})",
            name.to_lowercase(),
            index + 1
        )));
    }
    assert!(spans["switch-attribute"].contains("local option = newNode.options[attributeIndex]"));
    assert!(spans["switch-attribute"].contains("self:ReplaceNode(newNode, option)"));
    assert!(spans["switch-attribute"].contains("self.tree:ProcessStats(newNode)"));
    assert!(spans["replace-node"].contains("old.sd = newNode.sd"));
    assert!(spans["replace-node"].contains("old.modList:AddList(newNode.modList)"));
    assert!(spans["import-overrides"].contains("self:ReplaceNode(node, override)"));
    assert!(
        spans["applied-override-and-external-boundary"]
            .contains("self:ReplaceNode(node, self.hashOverrides[node.id])")
    );
    assert!(
        spans["applied-override-and-external-boundary"]
            .contains("if node.conqueredBy and node.type ~= \"Socket\" then")
    );
    // Bare +5 lines take the BASE form and the three literal scalar names. The
    // entire parser, not just these landmarks, is authenticated above.
    assert!(spans["modifier-forms"].contains("[\"^([%+%-][%d%.]+)%%? to\"] = \"BASE\""));
    for (_, _, name, modifier) in OPTIONS {
        assert!(
            spans["attribute-modifier-names"]
                .contains(&format!("[\"{}\"] = \"{modifier}\"", name.to_lowercase()))
        );
    }
    assert!(spans["modifier-parser"].contains("local modType = \"BASE\""));
    assert_eq!(
        spans["attribute-default-empty"],
        "\t[\"[%+%-](%d+) to any attribute\"] = { },\n"
    );
    assert!(
        spans["modifier-parser"]
            .contains("local specialMod, specialLine, cap = scan(line, specialModList)")
    );
    assert!(spans["modifier-parser"].contains("return copyTable(specialMod)"));
    assert!(
        spans["modifier-parser"].contains("name = name .. (modSuffix or misc.modSuffix or \"\")")
    );
    assert!(spans["modifier-cache"].contains("return unpack(copyTable(cache[line]))"));
    // Parser failure remains unknown; it is never turned into absence evidence.
    assert!(spans["process-stats"].contains("node.unknown = true"));
    assert!(spans["process-stats"].contains("node.extra = true"));
    assert!(spans["process-stats"].contains("if mod.list and not mod.extra then"));
    assert!(spans["extra-support-contrast"].contains("mod(\"ExtraSupport\", \"LIST\""));
    assert!(spans["linked-support-contrast"].contains("mod(\"LinkedSupport\", \"LIST\""));
}

#[test]
fn source_witness_rejects_unselected_option_and_hidden_raw_behavior() {
    let raw_pin = &evidence().files[1];
    let compressed = bounded(&raw_pin.archive, 524288).unwrap();
    let mut raw = Vec::new();
    GzDecoder::new(compressed.as_slice())
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut raw)
        .unwrap();
    assert_eq!(hash(&raw), RAW_JSON_HASH);
    let raw: Value = serde_json::from_slice(&raw).unwrap();
    let hyphenated = &raw["nodes"]["34201"];
    assert_eq!(hyphenated["stringId"], "attributes20-");
    assert!(reviewed_node("34201", hyphenated).is_ok());
    for suffix in ["\"", "\\", "\n"] {
        let mut changed = hyphenated.clone();
        changed["stringId"] = json!(format!("attributes20-{suffix}"));
        assert!(reviewed_node("34201", &changed).is_err());
    }
    let original = &raw["nodes"]["15782"];
    assert!(reviewed_node("15782", original).is_ok());
    for index in 0..3 {
        let mut changed = original.clone();
        changed["options"][index]["stats"] =
            json!(["Socketed Skills are Supported by Level 1 Fork"]);
        assert!(reviewed_node("15782", &changed).is_err());
    }
    let mut changed = original.clone();
    changed["options"][1]["callback"] = json!("ExtraSupport");
    assert!(reviewed_node("15782", &changed).is_err());
    let mut changed = original.clone();
    changed["isSwitchable"] = json!(true);
    assert!(reviewed_node("15782", &changed).is_err());
    let mut changed = original.clone();
    changed["options"].as_array_mut().unwrap().swap(0, 2);
    assert!(reviewed_node("15782", &changed).is_err());
}

#[test]
fn authentication_rejects_missing_relocated_or_rehashed_evidence() {
    let original = evidence();
    assert!(authenticate(&original).is_ok());
    let mut missing = original.clone();
    missing.nodes.pop();
    assert!(authenticate(&missing).is_err());
    let mut relocated = original.clone();
    relocated.nodes[0].first += 1;
    assert!(authenticate(&relocated).is_err());
    let mut rehashed = original.clone();
    rehashed.artifacts[1].sha256 = "0".repeat(64);
    assert!(authenticate(&rehashed).is_err());
    let mut reduced = original;
    reduced.spans.retain(|span| span.role != "modifier-parser");
    assert!(authenticate(&reduced).is_err());
}
