//! Finite, offline source evidence for every class-view intrinsic domain.
//! This authenticates reviewed source; it does not parse or execute Lua.
use flate2::read::GzDecoder;
use poe_optimizer_core::{
    owned_definitions::StatDefId,
    owned_schema::{DefinitionEntry, StatSchema},
};
use poe_optimizer_import::{
    owned_mapping::MappingPackageInput,
    owned_passive_views::ViewRecipePolicy,
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
pub(super) const SOURCE_IDS: [&str; 7] =
    ["1755", "18845", "22314", "41965", "4739", "51184", "60230"];
const SPANS: &[(&str, &str, usize, usize)] = &[
    ("tree-constructor", "src/Classes/PassiveTree.lua", 59, 446),
    ("process-stats", "src/Classes/PassiveTree.lua", 448, 525),
    ("process-node", "src/Classes/PassiveTree.lua", 528, 595),
    (
        "class-view-dispatch",
        "src/Classes/PassiveSpec.lua",
        1483,
        1494,
    ),
    (
        "applied-override-and-external-boundary",
        "src/Classes/PassiveSpec.lua",
        1570,
        1578,
    ),
    ("replace-node", "src/Classes/PassiveSpec.lua", 2047, 2063),
    ("modifier-forms", "src/Modules/ModParser.lua", 62, 158),
    ("intelligence-name", "src/Modules/ModParser.lua", 160, 171),
    ("damage-names", "src/Modules/ModParser.lua", 707, 714),
    ("spell-flag", "src/Modules/ModParser.lua", 1044, 1054),
    ("minion-prefix", "src/Modules/ModParser.lua", 1229, 1231),
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
        1676797,
        "37db1255f28d05f67a6d81dfe286bcc698a9811e907c67d29cf640d9c28e5557",
    ),
    (
        "current/mapping.json",
        3165465,
        "3b627b9e7195ba18a44f83a73aff8fbffc238162ccfc273992fc129179cdb1a4",
    ),
    (
        "current/tree-normalization.json",
        1618228,
        "6384b05d546ea38bb48227594755d3bcaa66eeff42fad9ea8727f022786a5be1",
    ),
    (
        "attribute-stages/count-bindings.json",
        3100,
        "4b6501509beebe49e0da197f657f57df4ac86341e1f7f04e84a87990c9e7bc71",
    ),
    (
        "attribute-stages/count-programs.json",
        4789632,
        "d228211a0e8328f8628313b148de144698f5e0cf7b8d718e16afd7b129bfdded",
    ),
    (
        "passive-views/policy.json",
        28549,
        "c1d368db29b6bb14834e78520e8e27ff483f15e0a7e39666d6d3bb99749554d6",
    ),
    (
        "passive-views/statistics.json",
        2515,
        "ebc7eb124d533707949417674f5f0811ae385aeb92c87d79f601ae82fd19ae43",
    ),
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
struct RawView {
    connections: Vec<Connection>,
    group: u32,
    icon: String,
    #[serde(default)]
    is_notable: Option<bool>,
    is_switchable: bool,
    name: String,
    options: BTreeMap<String, RawOption>,
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
    count_programs: Value,
    policy: ViewRecipePolicy,
    statistics: Vec<DefinitionEntry<StatDefId, StatSchema>>,
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
        &bounded(&format!("{BASE}view-support-domains/evidence.json"), 131072).unwrap(),
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
fn safe_literal(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|c| c.is_ascii() && !c.is_ascii_control() && c != b'"' && c != b'\\')
}
fn reviewed_node(key: &str, value: &Value, policy: &ViewRecipePolicy) -> Result<RawView, String> {
    require(
        value
            .as_object()
            .is_some_and(|v| v.contains_key("isNotable") == (key == "51184"))
            && value.get("isNotable").is_none_or(|v| v == true),
        "notable presence",
    )?;
    let node: RawView = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    let rows: Vec<_> = policy.nodes.iter().filter(|n| n.node == key).collect();
    require(rows.len() == 1, "unique reviewed policy node")?;
    let row = rows[0];
    require(
        key == node.skill.to_string()
            && node.is_switchable
            && node.options.len() == 1
            && node.options.contains_key("Witch")
            && row.views.len() == 1
            && node.stats == row.default.expected_stats
            && node.is_notable == (key == "51184").then_some(true),
        "complete default shape",
    )?;
    let option = &node.options["Witch"];
    require(
        option.stats == row.views[0].effects.expected_stats,
        "complete Witch list",
    )?;
    for text in [
        &node.icon,
        &node.name,
        &node.string_id,
        &option.icon,
        &option.name,
    ]
    .into_iter()
    .chain(node.stats.iter())
    .chain(option.stats.iter())
    {
        require(safe_literal(text), "unescaped literal text")?;
    }
    Ok(node)
}
fn lua_stats(text: &mut String, stats: &[String], indent: &str) {
    for (i, line) in stats.iter().enumerate() {
        writeln!(
            text,
            "{indent}[{}]=\"{}\"{}",
            i + 1,
            line,
            if i + 1 == stats.len() { "" } else { "," }
        )
        .unwrap();
    }
}
// Fixed reviewed record layout only; equality covers the whole executed literal.
fn lua_literal(node: &RawView) -> String {
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
    writeln!(
        text,
        "\t\t\t}},\n\t\t\tgroup={},\n\t\t\ticon=\"{}\",",
        node.group, node.icon
    )
    .unwrap();
    if node.is_notable == Some(true) {
        text.push_str("\t\t\tisNotable=true,\n");
    }
    writeln!(
        text,
        "\t\t\tisSwitchable=true,\n\t\t\tname=\"{}\",\n\t\t\toptions={{\n\t\t\t\tWitch={{",
        node.name
    )
    .unwrap();
    let option = &node.options["Witch"];
    writeln!(
        text,
        "\t\t\t\t\ticon=\"{}\",\n\t\t\t\t\tid={},\n\t\t\t\t\tname=\"{}\",\n\t\t\t\t\tstats={{",
        option.icon, option.id, option.name
    )
    .unwrap();
    lua_stats(&mut text, &option.stats, "\t\t\t\t\t\t");
    writeln!(text,"\t\t\t\t\t}}\n\t\t\t\t}}\n\t\t\t}},\n\t\t\torbit={},\n\t\t\torbitIndex={},\n\t\t\tskill={},\n\t\t\tstats={{",node.orbit,node.orbit_index,node.skill).unwrap();
    lua_stats(&mut text, &node.stats, "\t\t\t\t");
    writeln!(
        text,
        "\t\t\t}},\n\t\t\tstringId=\"{}\"\n\t\t}},",
        node.string_id
    )
    .unwrap();
    text
}
fn verify_catalog(
    evidence: &Evidence,
    sources: &BTreeMap<String, String>,
    catalog: &TreeCatalogInput,
    policy: &ViewRecipePolicy,
) -> Result<(), String> {
    let raw: Value = serde_json::from_str(&sources[TREE_JSON]).map_err(|e| e.to_string())?;
    require(
        evidence
            .nodes
            .iter()
            .map(|n| n.key.as_str())
            .collect::<Vec<_>>()
            == SOURCE_IDS,
        "fixed seven-node inventory",
    )?;
    require(
        policy
            .nodes
            .iter()
            .map(|n| n.node.as_str())
            .collect::<BTreeSet<_>>()
            == SOURCE_IDS.into_iter().collect()
            && policy.nodes.len() == 7,
        "policy source inventory",
    )?;
    let lines: Vec<_> = sources[TREE_LUA].lines().collect();
    let start = lines
        .iter()
        .position(|l| *l == "\tnodes={")
        .ok_or("nodes table")?;
    for pin in &evidence.nodes {
        let node = reviewed_node(&pin.key, &raw["nodes"][&pin.key], policy)?;
        require(
            pin.first > start
                && span(&lines, pin.first, pin.last, &pin.sha256)? == lua_literal(&node),
            "whole Lua/JSON body equality",
        )?;
        let rows: Vec<_> = catalog.nodes.iter().filter(|n| n.key == pin.key).collect();
        require(rows.len() == 1, "unique catalog row")?;
        let row = rows[0];
        require(
            matches!(
                row.kind,
                TreeNodeKind::Allocation {
                    pool: TreePoolKind::Ordinary
                }
            ) && row.stats == node.stats
                && row.views.len() == 1
                && row.views[0].selector == "Witch"
                && row.views[0].stats == node.options["Witch"].stats
                && row.unlock.is_empty(),
            "whole catalog projection",
        )?;
    }
    require(!catalog.nodes.iter().any(|n| matches!(&n.kind, TreeNodeKind::AttachedChoice {parent} if SOURCE_IDS.contains(&parent.as_str()))), "unreviewed attached choice")?;
    // 4739 is adjacent to the shared Witch/Sorceress root. The constructor
    // explicitly adds ConnectedTo*Start FLAGs, which are not support factories.
    let roots: BTreeSet<_> = catalog.classes.iter().map(|c| c.root.as_str()).collect();
    let edges: Vec<_> = catalog
        .edges
        .iter()
        .chain(&catalog.unresolved_edges)
        .filter(|e| {
            (SOURCE_IDS.contains(&e.left.as_str()) && roots.contains(e.right.as_str()))
                || (SOURCE_IDS.contains(&e.right.as_str()) && roots.contains(e.left.as_str()))
        })
        .collect();
    require(
        edges.len() == 1 && edges[0].left == "4739" && edges[0].right == "54447",
        "reviewed class-root edge",
    )?;
    require(
        catalog
            .classes
            .iter()
            .filter(|c| c.root == "54447")
            .map(|c| c.key.as_str())
            .collect::<BTreeSet<_>>()
            == ["1", "7"].into_iter().collect(),
        "shared class-root identity",
    )?;
    Ok(())
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
        artifacts.push(pinned(pin, 8 * 1024 * 1024)?);
    }
    let catalog = serde_json::from_slice(&artifacts[0]).map_err(|e| e.to_string())?;
    let policy = serde_json::from_slice(&artifacts[5]).map_err(|e| e.to_string())?;
    verify_catalog(evidence, &sources, &catalog, &policy)?;
    Ok(Proof {
        catalog,
        mapping: serde_json::from_slice(&artifacts[1]).map_err(|e| e.to_string())?,
        tree: serde_json::from_slice(&artifacts[2]).map_err(|e| e.to_string())?,
        count_bindings: serde_json::from_slice(&artifacts[3]).map_err(|e| e.to_string())?,
        count_programs: serde_json::from_slice(&artifacts[4]).map_err(|e| e.to_string())?,
        policy,
        statistics: serde_json::from_slice(&artifacts[6]).map_err(|e| e.to_string())?,
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

pub(super) fn verified_policy() -> &'static ViewRecipePolicy {
    &proof().policy
}
pub(super) fn verified_statistics() -> &'static [DefinitionEntry<StatDefId, StatSchema>] {
    &proof().statistics
}
pub(super) fn verified_count_programs() -> &'static Value {
    &proof().count_programs
}

#[test]
fn complete_default_and_witch_shapes_are_pinned_for_exactly_seven_physical_nodes() {
    let proof = proof();
    assert_eq!(proof.catalog.tree_version, "0_5");
    assert_eq!(proof.catalog.source.revision, REVISION);
    for (id, default, witch) in [
        (
            "1755",
            vec!["8% increased Spell Damage"],
            vec![
                "8% increased Spell Damage",
                "Minions deal 8% increased Damage",
            ],
        ),
        (
            "18845",
            vec!["10% increased Spell Damage"],
            vec![
                "10% increased Spell Damage",
                "Minions deal 10% increased Damage",
            ],
        ),
        (
            "22314",
            vec!["8% increased Elemental Damage"],
            vec![
                "8% increased Spell Damage",
                "Minions deal 8% increased Damage",
            ],
        ),
        (
            "41965",
            vec!["8% increased Spell Damage"],
            vec![
                "8% increased Spell Damage",
                "Minions deal 8% increased Damage",
            ],
        ),
        (
            "4739",
            vec!["10% increased Spell Damage"],
            vec![
                "10% increased Spell Damage",
                "Minions deal 10% increased Damage",
            ],
        ),
        (
            "51184",
            vec!["20% increased Spell Damage", "+10 to Intelligence"],
            vec![
                "16% increased Spell Damage",
                "Minions deal 16% increased Damage",
                "+10 to Intelligence",
            ],
        ),
        (
            "60230",
            vec!["8% increased Elemental Damage"],
            vec![
                "8% increased Spell Damage",
                "Minions deal 8% increased Damage",
            ],
        ),
    ] {
        let row = proof.policy.nodes.iter().find(|n| n.node == id).unwrap();
        assert_eq!(row.default.expected_stats, default);
        assert_eq!(row.views[0].effects.expected_stats, witch);
    }
}
#[test]
fn full_constructor_parser_and_class_dispatch_keep_whole_list_and_external_boundaries() {
    let s = &proof().spans;
    for (role, first) in [
        ("tree-constructor", "function PassiveTreeClass:PassiveTree("),
        ("process-stats", "function PassiveTreeClass:ProcessStats("),
        ("process-node", "function PassiveTreeClass:ProcessNode("),
        ("replace-node", "function PassiveSpecClass:ReplaceNode("),
        ("modifier-scan", "local function scan("),
        ("modifier-parser", "local function parseMod("),
    ] {
        assert!(s[role].starts_with(first) && s[role].ends_with("end\n"));
    }
    assert!(
        s["tree-constructor"].contains("io.open(\"TreeData/\"..treeVersion..\"/tree.lua\", \"r\")")
    );
    assert!(s["tree-constructor"].contains("node.modList:NewMod(\"Condition:ConnectedTo\"..class.name..\"Start\", \"FLAG\", true, \"Tree:\"..nodeId)"));
    assert!(s["process-node"].contains("setmetatable(switchNode, { __index = node })"));
    assert!(s["process-node"].contains("switchNode.sd = switchNode.stats"));
    assert!(s["process-node"].contains("self:ProcessStats(switchNode)"));
    let dispatch = &s["class-view-dispatch"];
    assert!(dispatch.contains("if self.tree.nodes[id].options[self.curClassName] then"));
    assert!(dispatch.contains("elseif self.tree.nodes[id].options[self.curAscendClassName] then"));
    assert!(dispatch.contains("local nodeToReplace = self.tree.nodes[id]"));
    assert!(dispatch.contains("self:ReplaceNode(node, nodeToReplace)"));
    assert!(s["replace-node"].contains("old.sd = newNode.sd"));
    assert!(s["replace-node"].contains("old.modList = new(\"ModList\"):ModList()"));
    assert!(s["replace-node"].contains("old.modList:AddList(newNode.modList)"));
    assert!(
        s["applied-override-and-external-boundary"]
            .contains("if node.conqueredBy and node.type ~= \"Socket\" then")
    );
    assert!(s["modifier-forms"].contains("[\"^([%+%-][%d%.]+)%%? to\"] = \"BASE\""));
    assert!(s["intelligence-name"].contains("[\"intelligence\"] = \"Int\""));
    for line in [
        "[\"damage\"] = \"Damage\"",
        "[\"elemental damage\"] = \"ElementalDamage\"",
    ] {
        assert!(s["damage-names"].contains(line));
    }
    assert!(s["spell-flag"].contains("[\"spell\"] = { flags = ModFlag.Spell }"));
    assert!(
        s["minion-prefix"]
            .contains("[\"^minions [cthd][ae][ukva][sel]e? \"] = { addToMinion = true }")
    );
    assert!(
        s["modifier-parser"]
            .contains("local specialMod, specialLine, cap = scan(line, specialModList)")
    );
    assert!(s["modifier-parser"].contains("return copyTable(specialMod)"));
    assert!(s["modifier-parser"].contains("name = name .. (modSuffix or misc.modSuffix or \"\")"));
    assert!(
        s["modifier-parser"]
            .contains("mod(\"MinionModifier\", \"LIST\", { mod = effectMod }, unpack(tagList))")
    );
    assert!(s["modifier-cache"].contains("return unpack(copyTable(cache[line]))"));
    assert!(s["process-stats"].contains("node.unknown = true"));
    assert!(s["process-stats"].contains("node.extra = true"));
    assert!(s["process-stats"].contains("if mod.list and not mod.extra then"));
    assert!(s["extra-support-contrast"].contains("mod(\"ExtraSupport\", \"LIST\""));
    assert!(s["linked-support-contrast"].contains("mod(\"LinkedSupport\", \"LIST\""));
}
fn raw_sources() -> BTreeMap<String, String> {
    evidence()
        .files
        .into_iter()
        .map(|p| {
            let bytes = bounded(&p.archive, 524288).unwrap();
            let mut raw = String::new();
            GzDecoder::new(bytes.as_slice())
                .take(4 * 1024 * 1024 + 1)
                .read_to_string(&mut raw)
                .unwrap();
            assert_eq!(hash(raw.as_bytes()), p.sha256);
            (p.path, raw)
        })
        .collect()
}
#[test]
fn unselected_view_extra_behavior_and_changed_catalog_are_rejected_after_valid_baseline() {
    let e = evidence();
    assert!(authenticate(&e).is_ok());
    let sources = raw_sources();
    let raw: Value = serde_json::from_str(&sources[TREE_JSON]).unwrap();
    for id in SOURCE_IDS {
        let original = &raw["nodes"][id];
        assert!(reviewed_node(id, original, verified_policy()).is_ok());
        for (field, value) in [
            ("extraSupport", json!("unreviewed")),
            ("isAttribute", json!(true)),
            ("isNotable", Value::Null),
        ] {
            let mut changed = original.clone();
            changed[field] = value;
            assert!(reviewed_node(id, &changed, verified_policy()).is_err());
        }
        let mut changed = original.clone();
        changed["options"]["Witch"]["stats"][0] = json!("Supports with unreviewed support");
        assert!(reviewed_node(id, &changed, verified_policy()).is_err());
        let mut changed = original.clone();
        changed["options"]["Sorceress"] = original["options"]["Witch"].clone();
        assert!(reviewed_node(id, &changed, verified_policy()).is_err());
        let mut changed = original.clone();
        changed["name"] = json!("escaped\\text");
        assert!(reviewed_node(id, &changed, verified_policy()).is_err());
    }
    let catalog = verified_catalog();
    assert!(verify_catalog(&e, &sources, catalog, verified_policy()).is_ok());
    let mut changed = catalog.clone();
    changed.nodes.retain(|n| n.key != "1755");
    assert!(verify_catalog(&e, &sources, &changed, verified_policy()).is_err());
    let mut changed = catalog.clone();
    changed
        .nodes
        .iter_mut()
        .find(|n| n.key == "4739")
        .unwrap()
        .views[0]
        .stats
        .push("unreviewed".into());
    assert!(verify_catalog(&e, &sources, &changed, verified_policy()).is_err());
    let mut changed = verified_policy().clone();
    changed.nodes.push(changed.nodes[0].clone());
    assert!(verify_catalog(&e, &sources, catalog, &changed).is_err());
    let mut changed = e.clone();
    changed.nodes[0].first -= 1;
    assert!(authenticate(&changed).is_err());
}
