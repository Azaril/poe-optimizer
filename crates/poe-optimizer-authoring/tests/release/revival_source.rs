//! Finite source evidence for intrinsic revival producers, not a closed writer census.
//! Full pinned constructors remain offline evidence; no Lua is parsed or executed here.
use super::{minion_support_source, skeletal_support_source};
use flate2::read::GzDecoder;
use poe_optimizer_core::owned_schema::{DefinitionAddress, SchemaSubject};
use poe_optimizer_import::{
    owned_mapping::{
        ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, MappingPackageInput,
        SourceComponent,
    },
    owned_tree_catalog::{TreeCatalogInput, TreeNodeKind, TreePoolKind},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs::File, io::Read, path::PathBuf, sync::OnceLock};

const BASE: &str = "data/owned/poe2/3887ae68/";
const MANIFEST: &str = "crates/poe-optimizer-pob/data/pob-source-manifest.json";
const MANIFEST_SHA: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const TREE: &str = "src/TreeData/0_5/tree.lua";
const RAW_TREE: &str = "src/TreeData/0_5/tree.json";
const SKILL: &str = "src/Data/Skills/act_int.lua";
const STAT_MAP: &str = "src/Data/SkillStatMap.lua";
const DATA: &str = "src/Modules/Data.lua";
const TOOLS: &str = "src/Modules/CalcTools.lua";
const ACTIVE: &str = "src/Modules/CalcActiveSkill.lua";
const SETUP: &str = "src/Modules/CalcSetup.lua";
const PARSER: &str = "src/Modules/ModParser.lua";
const MOD_TOOLS: &str = "src/Modules/ModTools.lua";
const PASSIVE_TREE: &str = "src/Classes/PassiveTree.lua";
const PASSIVE_SPEC: &str = "src/Classes/PassiveSpec.lua";
const FILES: &[(&str, &str)] = &[
    (TREE, "empty-passive-socket-intrinsics/tree.lua.gz"),
    (
        RAW_TREE,
        "passive-structural-inventories/sources/src_TreeData_0_5_tree.json.gz",
    ),
    (SKILL, "self-flat-minion-store/act_int.lua.gz"),
    (
        STAT_MAP,
        "support-structural-inventories/sources/src_Data_SkillStatMap.lua.gz",
    ),
    (
        DATA,
        "configuration-enemy-critical-damage-inputs/Data.lua.gz",
    ),
    (TOOLS, "self-flat-skill-construction/child-CalcTools.lua.gz"),
    (
        ACTIVE,
        "configuration-distance-inputs/CalcActiveSkill.lua.gz",
    ),
    (SETUP, "mana-supplier-sources/CalcSetup.lua.gz"),
    (MOD_TOOLS, "player-support-domains/ModTools.lua.gz"),
    (
        PARSER,
        "support-structural-inventories/sources/src_Modules_ModParser.lua.gz",
    ),
    (
        PASSIVE_TREE,
        "passive-structural-inventories/sources/src_Classes_PassiveTree.lua.gz",
    ),
    (
        PASSIVE_SPEC,
        "passive-structural-inventories/sources/src_Classes_PassiveSpec.lua.gz",
    ),
];
// Existing identities independently bound through the retained exact source map.
pub(super) const PASSIVES: &[(&str, &str, f64)] = &[
    ("28573", "def.0000000000000f9f", 5.0),
    ("38972", "def.00000000000012f4", 25.0),
    ("44255", "def.0000000000001476", 5.0),
];
const SKELETAL_SPANS: &[(&str, &str, usize, usize, &str, i32)] = &[
    (
        "SummonSkeletalArsonistsPlayer",
        "def.0000000000000323",
        18047,
        18170,
        "fb2bece57ff904a5d4a5766f3b3df28b112a90279ebd7cc25bc5153d6f5c3515",
        15,
    ),
    (
        "SummonSkeletalFrostMagesPlayer",
        "def.0000000000000326",
        18419,
        18542,
        "c9812f0956393628f52ef649f3d00eaa196a618a9905d9d34aa282f2518949d2",
        35,
    ),
    (
        "SummonSkeletalReaversPlayer",
        "def.0000000000000327",
        18543,
        18676,
        "54e56cb56c42207527985611680171aeb3eeeefeef94cab28a16dd62116a01a1",
        6,
    ),
];
const NODE_SPANS: &[(usize, usize, &str)] = &[
    (
        72280,
        72293,
        "e45cc318ba56c0e0b08c9660c822aad3ac2d5dab8a8e2912c52bd833362401aa",
    ),
    (
        91114,
        91141,
        "1e49479a21ef139cc16d2d3bd9d03eca35939813050b8484b4d16d849553d77d",
    ),
    (
        99718,
        99735,
        "485157ef8545fc351f93cf8c6e0c9c5fae7c08955e9cdfaeab5a348ecbe37692",
    ),
];

type Result<T> = std::result::Result<T, String>;
struct Proof {
    sources: BTreeMap<&'static str, String>,
    raw: Value,
}
fn require(condition: bool, message: &str) -> Result<()> {
    condition.then_some(()).ok_or_else(|| message.to_owned())
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn bounded(path: &str, maximum: usize) -> Result<Vec<u8>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let file = File::open(root.join(path)).map_err(|e| e.to_string())?;
    require(
        file.metadata().map_err(|e| e.to_string())?.len() <= maximum as u64,
        "file bound",
    )?;
    let mut bytes = Vec::new();
    file.take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    require(bytes.len() <= maximum, "file grew beyond bound")?;
    Ok(bytes)
}
fn source_pin(manifest: &Value, path: &str) -> Result<(usize, String)> {
    // The raw JSON is preserved upstream evidence corresponding to the executed
    // tree.lua; it is deliberately not a fabricated entry in the Lua manifest.
    if path == RAW_TREE {
        return Ok((
            2_049_036,
            "6449fe534c0265b21f59f8213254bd3f37a445887582c40aab04ad11cede3e95".into(),
        ));
    }
    let mut rows = manifest["files"]
        .as_array()
        .ok_or("manifest files")?
        .iter()
        .filter(|row| row["path"] == path);
    let row = rows.next().ok_or("missing source pin")?;
    require(rows.next().is_none(), "duplicate source pin")?;
    let bytes = row["bytes"].as_u64().ok_or("source length")?;
    require(bytes <= 4 * 1024 * 1024, "source limit")?;
    Ok((
        bytes as usize,
        row["sha256"].as_str().ok_or("source digest")?.into(),
    ))
}
fn check_source(bytes: &[u8], pin: &(usize, String)) -> Result<()> {
    require(
        bytes.len() == pin.0 && hash(bytes) == pin.1,
        "complete source changed",
    )
}
fn span(text: &str, first: usize, last: usize) -> Result<String> {
    require(first > 0 && last >= first, "span bounds")?;
    let lines: Vec<_> = text
        .lines()
        .skip(first - 1)
        .take(last - first + 1)
        .collect();
    require(lines.len() == last - first + 1, "incomplete source span")?;
    Ok(lines.join("\n") + "\n")
}
fn checked_span(text: &str, first: usize, last: usize, digest: &str) -> Result<String> {
    let body = span(text, first, last)?;
    require(
        hash(body.as_bytes()) == digest,
        "reviewed complete body changed",
    )?;
    Ok(body)
}
fn expected_node(key: &str) -> Value {
    match key {
        "28573" => {
            json!({"connections":[],"group":977,"icon":"Art/2DArt/SkillIcons/passives/minionlife.dds",
            "name":"Minion Revive Speed","orbit":7,"orbitIndex":6,"skill":28573,
            "stats":["Minions Revive 5% faster"],"stringId":"minion_defence19"})
        }
        "38972" => json!({"connections":[{"id":34552,"orbit":0},{"id":8357,"orbit":0}],"group":505,
            "icon":"Art/2DArt/SkillIcons/passives/miniondamageBlue.dds","isNotable":true,"name":"Restless Dead",
            "orbit":3,"orbitIndex":8,"recipe":["Despair","Fear","Disgust"],"skill":38972,
            "stats":["Minions Revive 25% faster"],"stringId":"minion_offence39"}),
        "44255" => json!({"connections":[{"id":28573,"orbit":-3}],"group":977,
            "icon":"Art/2DArt/SkillIcons/passives/minionlife.dds","name":"Minion Revive Speed",
            "orbit":7,"orbitIndex":12,"skill":44255,"stats":["Minions Revive 5% faster"],"stringId":"minion_defence12"}),
        _ => panic!("not a reviewed source"),
    }
}
fn selector(key: &str) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode {
        tree_version: SourceComponent::Text("0_5".into()),
        node_id: SourceComponent::Text(key.into()),
        view: SourceComponent::Missing,
    })
}
fn check_mapping(
    mapping: &MappingPackageInput,
    source: &ExternalSelector,
    key: &str,
) -> Result<()> {
    let mut matches = mapping
        .entries
        .iter()
        .filter(|entry| &entry.source == source);
    let entry = matches.next().ok_or("missing exact mapping")?;
    require(matches.next().is_none(), "duplicate source mapping")?;
    require(
        matches!(&entry.outcome, MappingOutcome::Mapped {
            target: SchemaSubject::Definition(address), basis: MappingBasis::Exact,
        } if address.key().as_str() == key && match source {
            ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode { .. }) => matches!(address, DefinitionAddress::PassiveNode(_)),
            ExternalSelector::Definition(ExternalOwnerSelector::Skill { .. }) => matches!(address, DefinitionAddress::Skill(_)),
            _ => false,
        }),
        "mapping identity, kind or basis",
    )
}
fn check_passives(
    raw: &Value,
    lua: &str,
    catalog: &TreeCatalogInput,
    mapping: &MappingPackageInput,
) -> Result<()> {
    for ((key, owned, amount), (first, last, digest)) in PASSIVES.iter().zip(NODE_SPANS) {
        let expected = expected_node(key);
        require(
            raw["nodes"][key] == expected,
            "whole raw node, including all fields and stats",
        )?;
        let body = checked_span(lua, *first, *last, digest)?;
        // Both entire source representations are fixed; these semantic anchors
        // identify the reviewed literal rather than selecting nodes by a name.
        require(
            body.starts_with(&format!("\t\t[{key}]={{\n"))
                && body.ends_with("\t\t},\n")
                && body.contains(&format!("[1]=\"Minions Revive {amount}% faster\"")),
            "whole executed node",
        )?;
        let mut nodes = catalog.nodes.iter().filter(|node| node.key == *key);
        let node = nodes.next().ok_or("catalog node")?;
        require(
            nodes.next().is_none()
                && matches!(
                    node.kind,
                    TreeNodeKind::Allocation {
                        pool: TreePoolKind::Ordinary
                    }
                )
                && node.stats == vec![format!("Minions Revive {amount}% faster")]
                && node.views.is_empty()
                && node.unlock.is_empty(),
            "exact ordinary intrinsic catalog node",
        )?;
        check_mapping(mapping, &selector(key), owned)?;
        require(
            !catalog.nodes.iter().any(|row| {
                matches!(&row.kind,
                TreeNodeKind::AttachedChoice { parent } if parent.as_str() == *key)
            }) && !catalog
                .edges
                .iter()
                .chain(&catalog.unresolved_edges)
                .any(|edge| {
                    catalog.classes.iter().any(|class| {
                        (edge.left == *key && edge.right == class.root)
                            || (edge.right == *key && edge.left == class.root)
                    })
                }),
            "additional choice or class-start source authority",
        )?;
    }
    Ok(())
}
fn check_sniper(sources: &BTreeMap<&str, String>) -> Result<()> {
    let body = checked_span(
        &sources[SKILL],
        18677,
        18799,
        "fb834997a7c392e6a8e1a190ecd1f864b7537562611f2237dd650e12d54de192",
    )?;
    require(
        body.starts_with("skills[\"SummonSkeletalSnipersPlayer\"] = {\n")
            && body.ends_with("\t}\n}\n")
            && body.contains("{ \"minion_base_resummon_time_ms\", 12000 },")
            && body.contains("\"is_resummoning_minion\",")
            && body.contains("{ \"skeletal_graft_buff_effect_magnitude\", 15 },")
            && body.contains("{ \"active_skill_minion_damage_+%_final\", 1, {  } },"),
        "full intrinsic skill and remaining mechanics",
    )?;
    require(
        span(&sources[STAT_MAP], 2690, 2696)?
            == concat!(
                "[\"is_resummoning_minion\"] = {\n\tflag(\"RevivingMinion\")\n},\n",
                "[\"minion_base_resummon_time_ms\"] = {\n\tmod(\"BaseReviveTime\", \"OVERRIDE\", nil),\n\tdiv = 1000,\n},\n"
            ),
        "exact typed stat construction and milliseconds divisor",
    )?;
    require(
        span(&sources[DATA], 56, 98)?
            .contains("return makeSkillMod(modName, \"FLAG\", true, 0, 0, ...)")
            && span(&sources[DATA], 910, 966)?
                .contains("setmetatable(statSet.statMap, data.skillStatMapMeta)"),
        "flag factory and shared map construction",
    )?;
    let build = span(&sources[TOOLS], 161, 231)?;
    require(
        build.starts_with("function calcLib.buildSkillInstanceStats(")
            && build.ends_with("\treturn stats\nend\n")
            && build.contains("local statValue = statSetLevel[index] or 1")
            && build.contains("stats[stat[1]] = (stats[stat[1]] or 0) + (stat[2] or 0)"),
        "complete stat-instance constructor",
    )?;
    let merge = span(&sources[ACTIVE], 22, 141)?;
    require(merge.contains("newMod.value = value")
        && merge.contains("map.value or statValue * (map.mult or 1) * scalar / (map.div or 1) + (map.base or 0)")
        && merge.contains("stats[stat.key] = (stats[stat.key] or 0) + stat.value")
        && merge.contains("modList:AddList(set.baseMods)"), "complete merge path retains intrinsic and foreign contributions")?;
    // Shared target domains are source semantics, not newly created Actor writers.
    require(
        sources[ACTIVE]
            .contains("local skillModList = new(\"ModList\"):ModList(activeSkill.actor.modDB)")
            && sources[ACTIVE].contains("activeSkill.skillModList = skillModList"),
        "exact skill parent",
    )?;
    Ok(())
}
fn check_skeletal_revival(sources: &BTreeMap<&str, String>) -> Result<()> {
    for &(source, _, first, last, digest, graft) in SKELETAL_SPANS {
        let body = checked_span(&sources[SKILL], first, last, digest)?;
        require(
            body.starts_with(&format!("skills[\"{source}\"] = {{\n"))
                && body.ends_with("\t}\n}\n")
                && body
                    .matches("\"minion_base_resummon_time_ms\", 12000")
                    .count()
                    == 1
                && body.matches("\"is_resummoning_minion\",").count() == 1
                && body.contains(&format!(
                    "{{ \"skeletal_graft_buff_effect_magnitude\", {graft} }}"
                ))
                && body.matches("actorLevel =").count() == 40
                && body.matches("spiritReservationFlat =").count() == 40,
            "whole skeletal intrinsic source, level domain and remaining paired mechanics",
        )?;
    }
    // check_sniper authenticates the shared flag factory, /1000 map, complete
    // stat constructor and local merge/parent boundaries used by all four.
    check_sniper(sources)
}
fn check_passive_constructors(sources: &BTreeMap<&str, String>) -> Result<()> {
    require(
        span(&sources[PARSER], 4736, 4736)?
            == "\t[\"minions revive (%d+)%% faster\"] = function(num) return { mod(\"MinionRevivalSpeed\", \"INC\", num) } end,\n",
        "revival parser family",
    )?;
    let factory = span(&sources[MOD_TOOLS], 57, 91)?;
    require(
        sources[PARSER].contains("local mod = modLib.createMod")
            && factory.starts_with("function modLib.createMod(")
            && factory.ends_with("\t}\nend\n")
            && factory.contains("local flags = 0")
            && factory.contains("local keywordFlags = 0")
            && factory.contains("\t\tvalue = modVal,"),
        "complete ordinary modifier factory preserves numeric value and scope",
    )?;
    let process = span(&sources[PASSIVE_TREE], 448, 525)?;
    require(
        process.contains("local list, extra = modLib.parseMod(line)")
            && process.contains("if mod.list and not mod.extra then")
            && process.contains("mod = modLib.setSource(mod, \"Tree:\"..node.id)"),
        "whole passive-stat constructor",
    )?;
    require(
        span(&sources[PASSIVE_SPEC], 1570, 1578)?
            .contains("self:ReplaceNode(node, self.hashOverrides[node.id])")
            && sources[SETUP].contains(
                "env.modDB:AddList(calcs.buildModListForNodeList(env, env.allocNodes, true))",
            )
            && span(&sources[SETUP], 744, 762)?.contains("modDB = env.modDB,"),
        "effective-source boundary and Player recipient",
    )?;
    Ok(())
}
fn proof() -> &'static Proof {
    static PROOF: OnceLock<Proof> = OnceLock::new();
    PROOF.get_or_init(|| {
        // Reuse existing exact catalog/mapping authentication, not live release output.
        let catalog = minion_support_source::verified_catalog();
        let mapping = minion_support_source::verified_mapping();
        // This independent whole-family proof also authenticates each winning
        // declaration, physical Gem supply, full stat set and local map override.
        skeletal_support_source::check_proof();
        let bytes = bounded(MANIFEST, 262144).unwrap();
        assert_eq!(hash(&bytes), MANIFEST_SHA);
        let manifest: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(manifest["upstream_revision"], REVISION);
        let mut sources = BTreeMap::new();
        let mut total = 0usize;
        for &(path, archive) in FILES {
            let pin = source_pin(&manifest, path).unwrap();
            total = total.checked_add(pin.0).unwrap();
            assert!(total <= 32 * 1024 * 1024);
            let compressed = bounded(&format!("{BASE}{archive}"), 524288).unwrap();
            let mut decoded = Vec::new();
            GzDecoder::new(compressed.as_slice())
                .take(pin.0 as u64 + 1)
                .read_to_end(&mut decoded)
                .unwrap();
            check_source(&decoded, &pin).unwrap();
            let text = String::from_utf8(decoded).unwrap();
            assert!(!text.contains('\r'));
            assert!(sources.insert(path, text).is_none());
        }
        let raw = serde_json::from_str(&sources[RAW_TREE]).unwrap();
        check_passives(&raw, &sources[TREE], catalog, mapping).unwrap();
        check_sniper(&sources).unwrap();
        check_skeletal_revival(&sources).unwrap();
        check_passive_constructors(&sources).unwrap();
        check_mapping(
            mapping,
            &ExternalSelector::Definition(ExternalOwnerSelector::Skill {
                effect_id: SourceComponent::Text("SummonSkeletalSnipersPlayer".into()),
            }),
            "def.0000000000000012",
        )
        .unwrap();
        for &(source, expected, ..) in SKELETAL_SPANS {
            check_mapping(
                mapping,
                &ExternalSelector::Definition(ExternalOwnerSelector::Skill {
                    effect_id: SourceComponent::Text(source.into()),
                }),
                expected,
            )
            .unwrap();
        }
        Proof { sources, raw }
    })
}
pub(super) fn check_proof() {
    let _ = proof();
}

#[test]
fn complete_intrinsic_sources_and_exact_mappings_support_only_the_reviewed_producers() {
    check_proof();
    assert_eq!(
        PASSIVES.iter().map(|row| row.2).collect::<Vec<_>>(),
        [5.0, 25.0, 5.0]
    );
    // Paired/conditional records remain positive census obligations, not inferred absent.
    let catalog = minion_support_source::verified_catalog();
    for (key, stats) in [
        (
            "15358",
            vec![
                "Minions have 10% increased maximum Life",
                "Minions Revive 8% faster",
            ],
        ),
        (
            "41905",
            vec![
                "Minions Revive 15% faster",
                "Recover 2% of maximum Life when one of your Minions is Revived",
            ],
        ),
        (
            "31129",
            vec!["Minions Revive 35% faster if all your Minions are Companions"],
        ),
    ] {
        let row = catalog.nodes.iter().find(|row| row.key == key).unwrap();
        assert_eq!(row.stats, stats);
        assert!(!PASSIVES.iter().any(|row| row.0 == key));
    }
}

#[test]
fn altered_intrinsic_body_stat_map_or_construction_refuses_the_source_proof() {
    let p = proof();
    check_sniper(&p.sources).unwrap();
    for (path, before, after) in [
        (
            SKILL,
            "\"minion_base_resummon_time_ms\", 12000",
            "\"minion_base_resummon_time_ms\", 7500",
        ),
        (SKILL, "\"is_resummoning_minion\",", "\"another_stat\","),
        (STAT_MAP, "\tdiv = 1000,", "\tdiv = 100,"),
        (
            DATA,
            "makeSkillMod(modName, \"FLAG\", true",
            "makeSkillMod(modName, \"FLAG\", false",
        ),
        (
            TOOLS,
            "local statValue = statSetLevel[index] or 1",
            "local statValue = statSetLevel[index] or 0",
        ),
        (
            ACTIVE,
            "local skillModList = new(\"ModList\"):ModList(activeSkill.actor.modDB)",
            "local skillModList = new(\"ModList\"):ModList(env.player.modDB)",
        ),
    ] {
        let mut changed = p.sources.clone();
        let original = &p.sources[path];
        let value = original.replace(before, after);
        assert_ne!(&value, original, "effective mutation for {path}");
        changed.insert(path, value);
        assert!(check_sniper(&changed).is_err(), "changed {path}");
    }
    let text = p.sources[SETUP].as_bytes();
    let pin = (text.len(), hash(text));
    check_source(text, &pin).unwrap();
    let mut changed = text.to_vec();
    changed.push(b'\n');
    assert!(check_source(&changed, &pin).is_err());
    check_passive_constructors(&p.sources).unwrap();
    let mut changed = p.sources.clone();
    changed.insert(
        PARSER,
        p.sources[PARSER].replace(
            "mod(\"MinionRevivalSpeed\", \"INC\", num)",
            "mod(\"MinionRevivalSpeed\", \"MORE\", num)",
        ),
    );
    assert!(check_passive_constructors(&changed).is_err());
}

#[test]
fn each_skeletal_intrinsic_rejects_changed_revive_or_paired_source_records() {
    let p = proof();
    check_skeletal_revival(&p.sources).unwrap();
    for &(_, _, first, last, _, _) in SKELETAL_SPANS {
        let original = span(&p.sources[SKILL], first, last).unwrap();
        for (before, after) in [
            (
                "\"minion_base_resummon_time_ms\", 12000",
                "\"minion_base_resummon_time_ms\", 7500",
            ),
            (
                "\"is_resummoning_minion\",",
                "\"unreviewed_resummoning_minion\",",
            ),
            (
                "\"skeletal_graft_buff_effect_magnitude\"",
                "\"unreviewed_graft_buff_effect_magnitude\"",
            ),
        ] {
            assert!(original.contains(before));
            let changed = original.replacen(before, after, 1);
            let mut sources = p.sources.clone();
            sources.insert(SKILL, p.sources[SKILL].replacen(&original, &changed, 1));
            assert!(
                check_skeletal_revival(&sources).is_err(),
                "source line {first}: {before}"
            );
        }
    }
}

#[test]
fn full_node_and_mapping_controls_reject_extra_mechanics_or_foreign_authority() {
    let p = proof();
    let catalog = minion_support_source::verified_catalog();
    let mapping = minion_support_source::verified_mapping();
    check_passives(&p.raw, &p.sources[TREE], catalog, mapping).unwrap();
    for replacement in [
        json!([
            "Minions Revive 5% faster",
            "Minions deal 10% increased Damage"
        ]),
        json!(["Minions Revive 50% faster"]),
    ] {
        let mut changed = p.raw.clone();
        changed["nodes"]["28573"]["stats"] = replacement;
        assert!(check_passives(&changed, &p.sources[TREE], catalog, mapping).is_err());
    }
    let mut changed = p.raw.clone();
    changed["nodes"]["38972"]["unexpectedMechanic"] = json!(true);
    assert!(check_passives(&changed, &p.sources[TREE], catalog, mapping).is_err());
    let lua = p.sources[TREE].replace(
        "[1]=\"Minions Revive 25% faster\"",
        "[1]=\"Minions Revive 35% faster\"",
    );
    assert_ne!(lua, p.sources[TREE]);
    assert!(check_passives(&p.raw, &lua, catalog, mapping).is_err());
    assert!(check_mapping(mapping, &selector("28573"), "def.0000000000001476").is_err());
    let mut duplicate = mapping.clone();
    let row = duplicate
        .entries
        .iter()
        .find(|row| row.source == selector("28573"))
        .unwrap()
        .clone();
    duplicate.entries.push(row);
    assert!(check_mapping(&duplicate, &selector("28573"), "def.0000000000000f9f").is_err());
    let mut absent = mapping.clone();
    absent.entries.retain(|row| row.source != selector("28573"));
    assert!(check_mapping(&absent, &selector("28573"), "def.0000000000000f9f").is_err());
}
