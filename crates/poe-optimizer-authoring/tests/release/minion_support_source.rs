//! Full-source evidence for exactly eighteen reviewed numeric Minion passives.
//! Source is authenticated offline evidence, never an alternate evaluator.
use flate2::read::GzDecoder;
use poe_optimizer_import::{
    owned_mapping::MappingPackageInput,
    owned_tree_catalog::{TreeCatalogInput, TreeNodeKind, TreePoolKind},
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
    (
        "applied-override-and-external-boundary",
        "src/Classes/PassiveSpec.lua",
        1570,
        1578,
    ),
    ("replace-node", "src/Classes/PassiveSpec.lua", 2047, 2063),
    ("modifier-forms", "src/Modules/ModParser.lua", 62, 158),
    ("life-name", "src/Modules/ModParser.lua", 178, 178),
    ("area-name", "src/Modules/ModParser.lua", 655, 655),
    ("cooldown-name", "src/Modules/ModParser.lua", 675, 675),
    ("damage-name", "src/Modules/ModParser.lua", 707, 707),
    (
        "command-conditions",
        "src/Modules/ModParser.lua",
        1160,
        1161,
    ),
    ("minion-prefix", "src/Modules/ModParser.lua", 1230, 1231),
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
    (
        "external-minion-skill-contrast",
        "src/Modules/ModParser.lua",
        2479,
        2480,
    ),
];
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
        "plain-minion-owner-closure/bindings.json",
        1745,
        "80a09e3a9f106f5b9e858251147287a597befb18048063bd8aa58620e94579d1",
    ),
    (
        "plain-minion-owner-closure/source-vectors.json",
        42232,
        "dc20b1228f9fa471cf805b509147d5d3c33aab34833fae654cab63f2cff7e708",
    ),
    (
        "plain-minion-life-passives/bindings.json",
        2802,
        "73dc671a0eabd4aa5b18c3a848ad4142ef4597ed66f8b7589daf0baccaff8c82",
    ),
    (
        "plain-minion-life-passives/source-vectors.json",
        120987,
        "584fc347de8f336c191fe55d66d0fa6ad6fc1d6e3eef6f2700aee7b29bcdd1df",
    ),
    (
        "minion-life-passives/bindings.json",
        2050,
        "301c17c8682c442a80e1fa1e8e4cce26b7039d3f8220d72ddc944af5aeefde2f",
    ),
    (
        "minion-life-passives/source-vectors.json",
        152220,
        "2ec61fe7682a5dc33b75365c6dfbb370a5598b9fdfac95739f05ee1c3d37bcdb",
    ),
    (
        "command-cooldown/bindings.json",
        8007,
        "43696bf7f7066a3db1f580ba44ae7690da092dae583e9b1be8db2f8f2177ed93",
    ),
    (
        "command-cooldown/source-vectors.json",
        154305,
        "4513b3662cc4ff2001ed95b69e93f475d3b996d9d06160e9b3fb2fb8040c98d9",
    ),
    (
        "command-damage/bindings.json",
        7153,
        "243a179ba99aa4bb881998e2fbdddbfd7cbe626d83b23ac371f579f5ec5d9d67",
    ),
    (
        "command-damage/source-vectors.json",
        1842840,
        "da5f56aaa5ebefd3de63eed1242b65b383b55852161d85e07d22c54e6b2e5242",
    ),
    (
        "growing-swarm/bindings.json",
        5366,
        "25cbb06215e087ecc05babd378c34933c80a4753168b58ab3cd25a4b8b392682",
    ),
    (
        "growing-swarm/source-vectors.json",
        163779,
        "1fa659d3092125cb77cfaa9a4d1a87ad298e35350c0984c39fbff4da4dc9654a",
    ),
    (
        "plain-minion-owner-closure/closure.json",
        10014,
        "e2bbce3be6074e055b3a99f41ffe136814ba5895bb718212bf14115d378921b7",
    ),
    (
        "plain-minion-life-passives/closure.json",
        24858,
        "eb10d88e7af55b25872b3efc6b59fce101ef18a312440ce386116c7ff2e363a2",
    ),
    (
        "command-cooldown/closure.json",
        32960,
        "22d16256b952454741d35c26b0585ce6c0a926e354d67e49f2cadbcd112c71c0",
    ),
    (
        "growing-swarm/closure.json",
        5235,
        "d7695fa84c94c42f7bf0773ab65f7f5cb7de8064adbdff76ebfa583b2a0c686c",
    ),
    (
        "command-cooldown/migration.json",
        31793,
        "080225df907256e4406121ed8202205c7341e659dbef1aa4c8a0f750ea4186ae",
    ),
    (
        "growing-swarm/migration.json",
        38485,
        "61195dc3588ac6e0ee8f3b5bcff78d6268caedf8ca4b66637c5797af473e0953",
    ),
];
pub(super) const SOURCE_IDS: &[&str] = &[
    "8737", "95", "19006", "229", "39461", "54453", "1218", "40894", "14598", "35645", "4345",
    "43979", "50837", "6077", "25927", "32847", "41511", "14945",
];
const FAMILIES: &[&str] = &[
    "plain-minion-owner-closure",
    "plain-minion-life-passives",
    "minion-life-passives",
    "command-cooldown",
    "command-damage",
    "growing-swarm",
];
struct Expected {
    key: &'static str,
    family: &'static str,
    name: &'static str,
    stats: &'static [&'static str],
}
const ROWS: &[Expected] = &[
    Expected {
        key: "8737",
        family: "plain-minion-owner-closure",
        name: "Minion Damage",
        stats: &["Minions deal 10% increased Damage"],
    },
    Expected {
        key: "95",
        family: "plain-minion-owner-closure",
        name: "Minion Damage",
        stats: &["Minions deal 10% increased Damage"],
    },
    Expected {
        key: "19006",
        family: "plain-minion-life-passives",
        name: "Minion Damage and Life",
        stats: &[
            "Minions have 6% increased maximum Life",
            "Minions deal 6% increased Damage",
        ],
    },
    Expected {
        key: "229",
        family: "plain-minion-life-passives",
        name: "Minion Damage and Life",
        stats: &[
            "Minions have 6% increased maximum Life",
            "Minions deal 6% increased Damage",
        ],
    },
    Expected {
        key: "39461",
        family: "plain-minion-life-passives",
        name: "Minion Damage and Life",
        stats: &[
            "Minions have 6% increased maximum Life",
            "Minions deal 6% increased Damage",
        ],
    },
    Expected {
        key: "54453",
        family: "plain-minion-life-passives",
        name: "Minion Damage and Life",
        stats: &[
            "Minions have 6% increased maximum Life",
            "Minions deal 6% increased Damage",
        ],
    },
    Expected {
        key: "1218",
        family: "minion-life-passives",
        name: "Minion Life",
        stats: &["Minions have 10% increased maximum Life"],
    },
    Expected {
        key: "40894",
        family: "minion-life-passives",
        name: "Minion Life",
        stats: &["Minions have 10% increased maximum Life"],
    },
    Expected {
        key: "14598",
        family: "command-cooldown",
        name: "Minion Damage and Command Speed",
        stats: &[
            "Minions deal 6% increased Damage",
            "Minions have 8% increased Cooldown Recovery Rate for Command Skills",
        ],
    },
    Expected {
        key: "35645",
        family: "command-cooldown",
        name: "Command Skill Cooldown",
        stats: &["Minions have 20% increased Cooldown Recovery Rate for Command Skills"],
    },
    Expected {
        key: "4345",
        family: "command-cooldown",
        name: "Minion Damage and Command Speed",
        stats: &[
            "Minions deal 6% increased Damage",
            "Minions have 8% increased Cooldown Recovery Rate for Command Skills",
        ],
    },
    Expected {
        key: "43979",
        family: "command-cooldown",
        name: "Minion Damage and Command Skill Cooldown",
        stats: &[
            "Minions deal 6% increased Damage",
            "Minions have 8% increased Cooldown Recovery Rate for Command Skills",
        ],
    },
    Expected {
        key: "50837",
        family: "command-cooldown",
        name: "Minion Damage and Command Speed",
        stats: &[
            "Minions deal 6% increased Damage",
            "Minions have 8% increased Cooldown Recovery Rate for Command Skills",
        ],
    },
    Expected {
        key: "6077",
        family: "command-cooldown",
        name: "Command Skill Cooldown",
        stats: &["Minions have 20% increased Cooldown Recovery Rate for Command Skills"],
    },
    Expected {
        key: "25927",
        family: "command-damage",
        name: "Command Skill Damage",
        stats: &["Minions deal 20% increased Damage with Command Skills"],
    },
    Expected {
        key: "32847",
        family: "command-damage",
        name: "Command Skill Damage",
        stats: &["Minions deal 20% increased Damage with Command Skills"],
    },
    Expected {
        key: "41511",
        family: "command-damage",
        name: "Command Skill Damage",
        stats: &["Minions deal 15% increased Damage with Command Skills"],
    },
    Expected {
        key: "14945",
        family: "growing-swarm",
        name: "Growing Swarm",
        stats: &[
            "Minions have 20% increased Area of Effect",
            "Minions have 20% increased Cooldown Recovery Rate",
        ],
    },
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
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawNode {
    connections: Vec<Connection>,
    group: u32,
    icon: String,
    is_notable: Option<bool>,
    name: String,
    orbit: u32,
    orbit_index: u32,
    recipe: Option<Vec<String>>,
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
struct Proof {
    catalog: TreeCatalogInput,
    mapping: MappingPackageInput,
    artifacts: BTreeMap<String, Value>,
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
            &format!("{BASE}minion-support-domains/evidence.json"),
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
fn safe_literal(text: &str) -> bool {
    text.chars()
        .all(|c| c.is_ascii() && !c.is_ascii_control() && !matches!(c, '"' | '\\'))
}
fn reviewed_node(expected: &Expected, raw: &Value) -> Result<RawNode, String> {
    let mut fields = BTreeSet::from([
        "connections",
        "group",
        "icon",
        "name",
        "orbit",
        "orbitIndex",
        "skill",
        "stats",
        "stringId",
    ]);
    let notable = expected.key == "14945";
    if notable {
        fields.extend(["isNotable", "recipe"]);
    }
    require(
        raw.as_object()
            .ok_or("raw object")?
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            == fields,
        "complete raw field inventory",
    )?;
    let node: RawNode = serde_json::from_value(raw.clone()).map_err(|e| e.to_string())?;
    require(
        node.skill.to_string() == expected.key
            && node.name == expected.name
            && node.stats == expected.stats
            && node.is_notable == notable.then_some(true)
            && node.recipe
                == notable.then(|| vec!["Guilt".into(), "Paranoia".into(), "Fear".into()])
            && [&node.icon, &node.name, &node.string_id]
                .into_iter()
                .all(|s| safe_literal(s)),
        "reviewed raw node",
    )?;
    Ok(node)
}
/// A fixed-shape encoder for these reviewed records, not a Lua parser. Whole
/// literal equality includes connection rows, recipe, flags and all stat rows.
fn lua_literal(node: &RawNode) -> String {
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
        "\t\t\tname=\"{}\",\n\t\t\torbit={},\n\t\t\torbitIndex={},",
        node.name, node.orbit, node.orbit_index
    )
    .unwrap();
    if let Some(recipe) = &node.recipe {
        text.push_str("\t\t\trecipe={\n");
        for (i, item) in recipe.iter().enumerate() {
            let comma = if i + 1 == recipe.len() { "" } else { "," };
            writeln!(text, "\t\t\t\t[{}]=\"{item}\"{comma}", i + 1).unwrap();
        }
        text.push_str("\t\t\t},\n");
    }
    writeln!(text, "\t\t\tskill={},\n\t\t\tstats={{", node.skill).unwrap();
    for (i, stat) in node.stats.iter().enumerate() {
        let comma = if i + 1 == node.stats.len() { "" } else { "," };
        writeln!(text, "\t\t\t\t[{}]=\"{stat}\"{comma}", i + 1).unwrap();
    }
    writeln!(
        text,
        "\t\t\t}},\n\t\t\tstringId=\"{}\"\n\t\t}},",
        node.string_id
    )
    .unwrap();
    text
}
fn verify_source(
    evidence: &Evidence,
    sources: &BTreeMap<String, String>,
    catalog: &TreeCatalogInput,
    artifacts: &BTreeMap<String, Value>,
) -> Result<(), String> {
    require(
        catalog_has_no_additional_owner_inputs(catalog),
        "catalog choice or class-root adjacency",
    )?;
    require(
        evidence.nodes.len() == ROWS.len()
            && ROWS
                .iter()
                .map(|row| row.key)
                .eq(SOURCE_IDS.iter().copied()),
        "exact selected inventory",
    )?;
    let raw: Value = serde_json::from_str(&sources[TREE_JSON]).map_err(|e| e.to_string())?;
    let raw_nodes = raw["nodes"].as_object().ok_or("raw nodes")?;
    let lines: Vec<_> = sources[TREE_LUA].lines().collect();
    let mut seen = BTreeSet::new();
    for (pin, expected) in evidence.nodes.iter().zip(ROWS) {
        require(
            pin.key == expected.key && seen.insert(pin.key.clone()),
            "ordered exact node witness",
        )?;
        let node = reviewed_node(expected, raw_nodes.get(&pin.key).ok_or("missing raw node")?)?;
        let text = span(&lines, pin.first, pin.last, &pin.sha256)?;
        require(
            text == lua_literal(&node),
            "complete Lua/JSON node equality",
        )?;
        let mut matches = catalog.nodes.iter().filter(|row| row.key == expected.key);
        let row = matches.next().ok_or("catalog node")?;
        require(
            matches.next().is_none()
                && matches!(
                    row.kind,
                    TreeNodeKind::Allocation {
                        pool: TreePoolKind::Ordinary
                    }
                )
                && row.stats == node.stats
                && row.views.is_empty()
                && row.unlock.is_empty(),
            "catalog node equality",
        )?;
        let vectors = &artifacts[&format!("{}/source-vectors.json", expected.family)];
        let rows = vectors["static_nodes"]
            .as_array()
            .or_else(|| vectors["source_excerpts"].as_array())
            .ok_or("retained source rows")?;
        let mut matches = rows.iter().filter(|row| row["source_id"] == expected.key);
        let retained = matches.next().ok_or("retained node")?;
        require(
            matches.next().is_none()
                && retained["path"] == TREE_LUA
                && retained["first_line"] == pin.first
                && retained["last_line"] == pin.last
                && retained["text"] == text,
            "retained complete node equality",
        )?;
        if let Some(retained_catalog) = retained.get("catalog_row") {
            require(
                *retained_catalog == serde_json::to_value(row).map_err(|e| e.to_string())?,
                "retained catalog equality",
            )?;
        }
    }
    // These packet memberships are an independent fixed source inventory. They
    // never expand to other nodes having matching display text.
    for family in FAMILIES {
        let bindings = &artifacts[&format!("{family}/bindings.json")];
        let expected: BTreeSet<_> = ROWS
            .iter()
            .filter(|row| row.family == *family)
            .map(|row| row.key)
            .collect();
        let actual: Vec<_> = if *family == "growing-swarm" {
            vec![
                bindings["source_id"]
                    .as_str()
                    .ok_or("single source identity")?,
            ]
        } else {
            bindings["nodes"]
                .as_array()
                .ok_or("binding nodes")?
                .iter()
                .map(|row| row["source_id"].as_str().ok_or("source identity"))
                .collect::<Result<_, _>>()?
        };
        require(
            actual.len() == expected.len()
                && actual.into_iter().collect::<BTreeSet<_>>() == expected,
            "exact packet membership",
        )?;
    }
    Ok(())
}
fn catalog_has_no_additional_owner_inputs(catalog: &TreeCatalogInput) -> bool {
    let roots: BTreeSet<_> = catalog
        .classes
        .iter()
        .map(|class| class.root.as_str())
        .collect();
    !catalog.nodes.iter().any(|node| matches!(&node.kind, TreeNodeKind::AttachedChoice { parent } if SOURCE_IDS.contains(&parent.as_str())))
        && !catalog.edges.iter().chain(&catalog.unresolved_edges).any(|edge| {
            (SOURCE_IDS.contains(&edge.left.as_str()) && roots.contains(edge.right.as_str()))
                || (SOURCE_IDS.contains(&edge.right.as_str()) && roots.contains(edge.left.as_str()))
        })
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
    let mut artifacts: BTreeMap<String, Value> = BTreeMap::new();
    for (pin, (path, bytes, digest)) in evidence.artifacts.iter().zip(ARTIFACTS) {
        require(
            pin.path == format!("{BASE}{path}") && pin.bytes == *bytes && pin.sha256 == *digest,
            "fixed artifact authority",
        )?;
        artifacts.insert(
            (*path).to_owned(),
            serde_json::from_slice(&pinned(pin, 4 * 1024 * 1024)?).map_err(|e| e.to_string())?,
        );
    }
    let catalog = serde_json::from_value(artifacts["tree/tree-catalog.json"].clone())
        .map_err(|e| e.to_string())?;
    verify_source(evidence, &sources, &catalog, &artifacts)?;
    Ok(Proof {
        catalog,
        mapping: serde_json::from_value(artifacts["current/mapping.json"].clone())
            .map_err(|e| e.to_string())?,
        artifacts,
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
pub(super) fn verified_bindings(family: &str) -> &'static Value {
    &proof().artifacts[&format!("{family}/bindings.json")]
}
pub(super) fn verified_closure(family: &str) -> &'static Value {
    &proof().artifacts[&format!("{family}/closure.json")]
}
pub(super) fn verified_migration(family: &str) -> &'static Value {
    &proof().artifacts[&format!("{family}/migration.json")]
}

#[test]
fn exact_eighteen_complete_bodies_and_retained_packets_are_authenticated() {
    let catalog = verified_catalog();
    assert_eq!(catalog.tree_version, "0_5");
    assert_eq!(catalog.source.revision, REVISION);
    assert_eq!(SOURCE_IDS.len(), 18);
    assert_eq!(ROWS.iter().map(|row| row.stats.len()).sum::<usize>(), 27);
    for family in FAMILIES {
        assert_eq!(verified_bindings(family)["schema_version"], 1);
    }
}

#[test]
fn scalar_minion_and_command_paths_do_not_conflate_external_origin_factories() {
    let spans = &proof().spans;
    for (role, first) in [
        ("tree-constructor", "function PassiveTreeClass:PassiveTree("),
        ("process-stats", "function PassiveTreeClass:ProcessStats("),
        ("process-node", "function PassiveTreeClass:ProcessNode("),
        ("replace-node", "function PassiveSpecClass:ReplaceNode("),
        ("modifier-scan", "local function scan("),
        ("modifier-parser", "local function parseMod("),
    ] {
        assert!(spans[role].starts_with(first) && spans[role].ends_with("end\n"));
    }
    assert!(spans["tree-constructor"].contains("node.sd = node.stats or {}"));
    assert!(spans["tree-constructor"].contains("elseif node[\"not\"] or node.isNotable then"));
    assert!(spans["tree-constructor"].contains("node.modList:NewMod(\"Condition:ConnectedTo\"..class.name..\"Start\", \"FLAG\", true, \"Tree:\"..nodeId)"));
    assert!(spans["process-node"].contains("self:ProcessStats(node)"));
    assert!(spans["process-node"].contains("if node.recipe then"));
    assert!(spans["process-node"].contains("if node.isSwitchable or node.isAttribute then"));
    assert!(spans["process-stats"].contains("modLib.parseMod(line)"));
    assert!(spans["process-stats"].contains("if mod.list and not mod.extra then"));
    assert!(spans["process-stats"].contains("node.unknown = true"));
    assert!(spans["process-stats"].contains("node.extra = true"));
    assert!(spans["process-stats"].contains("node.modList:AddMod(mod)"));
    assert!(spans["modifier-forms"].contains("[\"^(%d+)%% increased\"] = \"INC\""));
    for (role, name, modifier) in [
        ("life-name", "maximum life", "Life"),
        ("area-name", "area of effect", "AreaOfEffect"),
        (
            "cooldown-name",
            "cooldown recovery rate",
            "CooldownRecovery",
        ),
        ("damage-name", "damage", "Damage"),
    ] {
        assert_eq!(spans[role], format!("\t[\"{name}\"] = \"{modifier}\",\n"));
    }
    assert_eq!(
        spans["minion-prefix"],
        "\t[\"^minions \"] = { addToMinion = true },\n\t[\"^minions [cthd][ae][ukva][sel]e? \"] = { addToMinion = true },\n"
    );
    for prefix in ["with", "for"] {
        assert!(spans["command-conditions"].contains(&format!("[\"{prefix} command skills\"] = {{ tag = {{ type = \"Condition\", var = \"CommandableSkill\" }} }}")));
    }
    let parser = &spans["modifier-parser"];
    assert!(parser.contains("local specialMod, specialLine, cap = scan(line, specialModList)"));
    assert!(parser.contains("preFlag, line, preFlagCap = scan(line, preFlagList)"));
    assert!(parser.contains("modType = \"INC\""));
    assert!(parser.contains("modTag, line, tagCap = scan(line, modTagList)"));
    assert!(parser.contains("modFlag, line = scan(line, modFlagList, true)"));
    assert!(parser.contains("for _, data in pairs({ modName, preFlag, modFlag, modTag, modTag2, skillTag, modExtraTags }) do"));
    assert!(parser.contains("t_insert(tagList, copyTable(data.tag))"));
    assert!(parser.contains("elseif misc.addToMinion then"));
    assert!(parser.contains(
        "modList[i] = mod(\"MinionModifier\", \"LIST\", { mod = effectMod }, unpack(tagList))"
    ));
    assert!(spans["modifier-cache"].contains("return unpack(copyTable(cache[line]))"));
    assert!(spans["extra-support-contrast"].contains("mod(\"ExtraSupport\", \"LIST\""));
    assert!(spans["linked-support-contrast"].contains("mod(\"LinkedSupport\", \"LIST\""));
    assert!(spans["external-minion-skill-contrast"].contains("mod(\"ExtraMinionSkill\", \"LIST\""));
    assert!(
        spans["applied-override-and-external-boundary"]
            .contains("if node.conqueredBy and node.type ~= \"Socket\" then")
    );
    assert!(spans["replace-node"].contains("old.modList:AddList(newNode.modList)"));
}

#[test]
fn raw_witness_refuses_added_behavior_changed_conditions_and_unsafe_literals() {
    let original = evidence();
    assert!(authenticate(&original).is_ok());
    let compressed = bounded(&original.files[1].archive, 524288).unwrap();
    let mut bytes = Vec::new();
    GzDecoder::new(compressed.as_slice())
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(hash(&bytes), RAW_JSON_HASH);
    let raw: Value = serde_json::from_slice(&bytes).unwrap();
    for expected in ROWS {
        let node = &raw["nodes"][expected.key];
        assert!(reviewed_node(expected, node).is_ok());
        let mut changed = node.clone();
        changed["stats"]
            .as_array_mut()
            .unwrap()
            .push(json!("Socketed Gems are Supported by Level 1 Fork"));
        assert!(reviewed_node(expected, &changed).is_err());
        let mut changed = node.clone();
        changed["options"] = json!([]);
        assert!(reviewed_node(expected, &changed).is_err());
        for suffix in ["\"", "\\", "\n"] {
            let mut changed = node.clone();
            changed["stringId"] = json!(format!("source{suffix}"));
            assert!(reviewed_node(expected, &changed).is_err());
        }
    }
    let command = ROWS.iter().find(|row| row.key == "25927").unwrap();
    let mut changed = raw["nodes"][command.key].clone();
    changed["stats"] = json!(["Minions deal 20% increased Damage"]);
    assert!(reviewed_node(command, &changed).is_err());
    let growing = ROWS.iter().find(|row| row.key == "14945").unwrap();
    let mut changed = raw["nodes"][growing.key].clone();
    changed["recipe"] = Value::Null;
    assert!(reviewed_node(growing, &changed).is_err());
    changed = raw["nodes"][growing.key].clone();
    changed["isNotable"] = Value::Null;
    assert!(reviewed_node(growing, &changed).is_err());
}

#[test]
fn incomplete_or_retargeted_evidence_cannot_expand_the_reviewed_family() {
    let original = evidence();
    assert!(authenticate(&original).is_ok());
    let mut missing = original.clone();
    missing.nodes.pop();
    assert!(authenticate(&missing).is_err());
    let mut replaced = original.clone();
    replaced.nodes[0].key = "14945".into();
    assert!(authenticate(&replaced).is_err());
    let mut changed = original.clone();
    changed.nodes[0].first += 1;
    assert!(authenticate(&changed).is_err());
    let mut changed = original.clone();
    changed.artifacts[1].sha256 = "0".repeat(64);
    assert!(authenticate(&changed).is_err());
    let mut reduced = original;
    reduced.spans.retain(|span| span.role != "modifier-parser");
    assert!(authenticate(&reduced).is_err());
}

#[test]
fn catalog_refuses_attached_choices_and_automatic_class_start_flags() {
    let catalog = verified_catalog();
    assert!(catalog_has_no_additional_owner_inputs(catalog));
    let mut changed = catalog.clone();
    let mut child = changed.nodes[0].clone();
    child.key = "adversarial-attached-choice".into();
    child.kind = TreeNodeKind::AttachedChoice {
        parent: SOURCE_IDS[0].into(),
    };
    changed.nodes.push(child);
    assert!(!catalog_has_no_additional_owner_inputs(&changed));
    for reverse in [false, true] {
        let mut changed = catalog.clone();
        let root = changed.classes[0].root.clone();
        let node = SOURCE_IDS[0].to_owned();
        let (left, right) = if reverse { (root, node) } else { (node, root) };
        changed
            .edges
            .push(poe_optimizer_import::owned_tree_catalog::TreeEdgeInput { left, right });
        assert!(!catalog_has_no_additional_owner_inputs(&changed));
    }
}

fn observed_record(id: &str, name: &str, amount: u32, command: bool) -> Value {
    let mut inner = json!({"flags":0,"keywordFlags":0,"name":name,"source":format!("Tree:{id}"),"type":"INC","value":amount});
    if command {
        inner["_positions"] =
            json!([{"index":1,"value":{"type":"Condition","var":"CommandableSkill"}}]);
    }
    json!({"flags":0,"keyword_flags":0,"name":"MinionModifier","source":format!("Tree:{id}"),"tags":{},"type":"LIST","value":{"mod":inner}})
}

#[test]
fn retained_default_records_preserve_multiplicity_and_do_not_certify_effective_transforms() {
    let artifacts = &proof().artifacts;
    for (family, count) in [
        ("plain-minion-owner-closure", 16),
        ("plain-minion-life-passives", 32),
        ("growing-swarm", 3),
    ] {
        let vectors = &artifacts[&format!("{family}/source-vectors.json")];
        let reports = vectors["reports"].as_array().unwrap();
        assert_eq!(reports.len(), 2);
        assert_ne!(reports[0]["path"], reports[1]["path"]);
        for field in ["bytes", "sha256", "observations"] {
            assert_eq!(reports[0][field], reports[1][field]);
        }
        let expected_ids: BTreeSet<_> = ROWS
            .iter()
            .filter(|row| row.family == family)
            .map(|row| row.key)
            .collect();
        for report in reports {
            let rows: Vec<_> = report["observations"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|o| o["value"].get("modifiers").map(|_| &o["value"]))
                .collect();
            assert_eq!(rows.len(), count);
            let mut seen = BTreeSet::new();
            for row in rows {
                let id = row["id"].as_u64().unwrap().to_string();
                assert!(expected_ids.contains(id.as_str()));
                seen.insert(id.clone());
                let expected = ROWS.iter().find(|row| row.key == id).unwrap();
                assert_eq!(row["stats"], json!(expected.stats));
                assert_eq!(row["effective_same_definition"], false);
                assert_eq!(row["effective_name"], row["name"]);
                let records = match family {
                    "plain-minion-owner-closure" => vec![observed_record(&id, "Damage", 10, false)],
                    "plain-minion-life-passives" => vec![
                        observed_record(&id, "Life", 6, false),
                        observed_record(&id, "Damage", 6, false),
                    ],
                    "growing-swarm" => {
                        assert_eq!(row["allocated"], true);
                        vec![
                            observed_record(&id, "AreaOfEffect", 20, false),
                            observed_record(&id, "CooldownRecovery", 20, false),
                        ]
                    }
                    _ => unreachable!(),
                };
                assert_eq!(row["modifiers"], json!(records));
            }
            assert_eq!(
                seen.iter().map(String::as_str).collect::<BTreeSet<_>>(),
                expected_ids
            );
        }
        if family != "plain-minion-owner-closure" {
            assert_eq!(
                vectors["observation_semantics"]["modifiers"],
                "mainEnv.spec.tree.nodes[id].modList"
            );
            assert_eq!(
                vectors["observation_semantics"]["effective_same_definition"],
                "mainEnv.spec.nodes[id] == mainEnv.spec.tree.nodes[id]"
            );
            assert_eq!(
                vectors["observation_semantics"]["effective_modifiers_recorded"],
                false
            );
        }
    }
    assert_eq!(
        artifacts["growing-swarm/source-vectors.json"]["observation_semantics"]["area_receiving_query_observed"],
        false
    );
    let vectors = &artifacts["command-cooldown/source-vectors.json"];
    let reports = vectors["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    let observations = reports[0]["observations"].as_array().unwrap();
    assert_eq!(observations.len(), 7);
    let mut sources = 0;
    for family in observations.iter().filter_map(|o| o["value"].as_array()) {
        sources += 1;
        assert_eq!(family.len(), 7);
        for binding in verified_bindings("command-cooldown")["nodes"]
            .as_array()
            .unwrap()
        {
            let id = binding["source_id"].as_str().unwrap();
            let id_number: u64 = id.parse().unwrap();
            let matching: Vec<_> = family.iter().filter(|row| row["id"] == id_number).collect();
            assert_eq!(matching.len(), 1);
            let row = matching[0];
            assert_eq!(row["allocated"], true);
            assert_eq!(row["effective_same_definition"], false);
            assert_eq!(row["effective_name"], row["name"]);
            let amount = u32::try_from(binding["amount"].as_u64().unwrap()).unwrap();
            let mut records = Vec::new();
            if amount == 8 {
                records.push(observed_record(id, "Damage", 6, false));
            }
            records.push(observed_record(id, "CooldownRecovery", amount, true));
            assert_eq!(row["modifiers"], json!(records));
        }
    }
    assert_eq!(sources, 3);
}
