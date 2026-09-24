//! Exact full-list passive attribute data against authenticated original tree
//! descriptors and complete ProcessStats/parser bodies. No allocation admission.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Function, LuaSerdeExt, Table, Value};
use poe_optimizer_core::owned_content::digest_owned;
use poe_optimizer_import::{
    owned_passive_views::ViewRecipePolicy,
    owned_tree_catalog::{TreeCatalogInput, TreeNodeKind},
};
use poe_optimizer_pob::source;
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;

fn read(relative: &str) -> Json {
    serde_json::from_slice(&std::fs::read(runtime::repository().join(relative)).unwrap()).unwrap()
}

fn reviewed_families() -> BTreeSet<Vec<String>> {
    [
        vec!["+10 to Intelligence"],
        vec!["+12 to Strength"],
        vec!["+25 to Dexterity"],
        vec!["+25 to Intelligence"],
        vec!["+25 to Strength"],
        vec!["+3 to all Attributes"],
        vec!["+4 to Strength", "+4 to Dexterity"],
        vec!["+5 to all Attributes"],
        vec!["+8 to Dexterity"],
        vec!["+8 to Intelligence"],
        vec!["+8 to Strength"],
        vec!["3% increased Attributes"],
        vec!["4% increased Strength"],
        vec!["5% increased Strength"],
        vec!["7% increased Attributes"],
    ]
    .into_iter()
    .map(|lines| lines.into_iter().map(str::to_owned).collect())
    .collect()
}

#[test]
fn every_policy_node_has_exact_catalog_source_and_owned_allocation_identity() {
    let policy = read("data/owned/poe2/3887ae68/passive-attribute-inputs/policy.json");
    let bindings = read("data/owned/poe2/3887ae68/passive-attribute-inputs/bindings.json");
    let catalog: TreeCatalogInput =
        serde_json::from_value(read("data/owned/poe2/3887ae68/tree/tree-catalog.json")).unwrap();
    let typed: ViewRecipePolicy = serde_json::from_value(policy.clone()).unwrap();
    assert!(typed.receiver_rules.is_empty() && typed.receivers.is_empty());
    assert_eq!(typed.nodes.len(), 58);
    assert_eq!(bindings["source"]["revision"], source::UPSTREAM_REVISION);
    assert_eq!(
        serde_json::to_value(&catalog.source).unwrap(),
        bindings["source"]
    );
    assert_eq!(
        digest_owned("owned-tree-catalog-v1", &catalog, 8 * 1024 * 1024)
            .unwrap()
            .to_string(),
        bindings["catalog"].as_str().unwrap()
    );
    for file in bindings["source"]["files"].as_array().unwrap() {
        let path = file["path"].as_str().unwrap();
        let text = if path.ends_with(".lua") {
            assert_eq!(
                source::expected_file_sha256(path).unwrap(),
                file["sha256"].as_str().unwrap()
            );
            runtime::verified(path).unwrap()
        } else {
            std::fs::read_to_string(
                runtime::repository()
                    .join("vendor/path-of-building-poe2")
                    .join(path),
            )
            .unwrap()
            .replace("\r\n", "\n")
        };
        assert_eq!(
            format!("{:x}", Sha256::digest(text.as_bytes())),
            file["sha256"].as_str().unwrap()
        );
    }
    let families = reviewed_families();
    assert_eq!(families.len(), 15);
    let expected: BTreeSet<_> = catalog
        .nodes
        .iter()
        .filter(|node| {
            matches!(node.kind, TreeNodeKind::Allocation { .. })
                && node.views.is_empty()
                && node.unlock.is_empty()
                && families.contains(&node.stats)
        })
        .map(|node| node.key.as_str())
        .collect();
    let actual: BTreeSet<_> = typed.nodes.iter().map(|node| node.node.as_str()).collect();
    assert_eq!(actual.len(), 58);
    assert_eq!(
        actual, expected,
        "finite whole-catalog selection, not build-selected nodes"
    );
    let source_nodes: BTreeMap<_, _> = catalog
        .nodes
        .iter()
        .map(|node| (node.key.as_str(), node))
        .collect();
    let binding_nodes: BTreeMap<_, _> = bindings["bindings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| (node["source_node"].as_str().unwrap(), node))
        .collect();
    assert_eq!(binding_nodes.len(), 58);
    let tokens = read("data/owned/poe2/3887ae68/current/tree-normalization.json");
    let tokens: BTreeMap<_, _> = tokens["content"]["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|token| (token["token"].as_str().unwrap(), token))
        .collect();
    let mut seen_families = BTreeSet::new();
    for node in &typed.nodes {
        let original = source_nodes[node.node.as_str()];
        let binding = binding_nodes[node.node.as_str()];
        assert_eq!(original.kind, TreeNodeKind::Allocation { pool: node.pool });
        assert_eq!(original.stats, node.default.expected_stats);
        assert!(node.views.is_empty());
        assert_eq!(
            binding["expected_stats"],
            serde_json::to_value(&original.stats).unwrap()
        );
        assert_eq!(binding["pool"], serde_json::to_value(node.pool).unwrap());
        let token = tokens[node.node.as_str()];
        assert_eq!(token["role"]["kind"], "allocation");
        assert_eq!(token["role"]["value"]["node"], binding["owner"]);
        seen_families.insert(original.stats.clone());
    }
    assert_eq!(seen_families, families);
}

#[test]
fn whole_original_tree_stat_processing_matches_all_policy_contributions() {
    let policy = read("data/owned/poe2/3887ae68/passive-attribute-inputs/policy.json");
    let bindings = read("data/owned/poe2/3887ae68/passive-attribute-inputs/bindings.json");
    let channels = BTreeMap::from([
        ("def.0000000000001d2e", "Str"),
        ("def.0000000000001d2f", "Dex"),
        ("def.0000000000001d30", "Int"),
    ]);
    assert_eq!(bindings["channels"]["strength"], "def.0000000000001d2e");
    assert_eq!(bindings["channels"]["dexterity"], "def.0000000000001d2f");
    assert_eq!(bindings["channels"]["intelligence"], "def.0000000000001d30");
    for warm in [false, true] {
        let oracle = runtime::Oracle::new();
        oracle.lua.globals().set("passiveWarm", warm).unwrap();
        oracle.lua.load(r#"
passiveAttributeTree=LoadModule("TreeData/0_5/tree")
LoadModule("Classes/PassiveTree")
function observePassiveAttribute(id,expected,pool)
 local raw=assert(passiveAttributeTree.nodes[id])
 assert(raw.skill==id)
 for _,field in ipairs({"classesStart","isAscendancyStart","isAttribute","isSwitchable", "isMultipleChoice","isMultipleChoiceOption","isMastery","isOnlyImage","isKeystone","isJewelSocket","isFreeAllocate","containJewelSocket","applyToArmour","noRadius","sinister","aliasPassiveSocket"}) do
  assert(not raw[field],field)
 end
 assert(not raw.options and not raw.unlockConstraint)
 assert((raw.ascendancyName and "ascendancy" or "ordinary")==pool)
 assert(#raw.stats==#expected)
 for index,line in ipairs(expected) do assert(raw.stats[index]==line) end
 -- These are original constructor descriptor fields; ProcessStats itself is
 -- invoked whole. No tree constructor, UI assets, allocation or stat override.
 local node={id=raw.skill,sd=copyTable(raw.stats)}
 common.classes.PassiveTree.ProcessStats({},node)
 assert(not node.unknown and not node.extra and #node.sd==#expected)
 assert(#node.mods==#expected)
 local count=0
 for index,line in ipairs(expected) do
  assert(node.sd[index]==line)
  local row=node.mods[index]
  assert(row.list and not row.extra and not row.combined)
  count=count+#row.list
 end
 assert(count==#node.modList)
 return node.modList
end
if passiveWarm then jit.on() else jit.off();jit.flush() end
"#).set_name("@original-passive-attribute-descriptor-inputs").exec().unwrap();
        let observe: Function = oracle.lua.globals().get("observePassiveAttribute").unwrap();
        let mut totals = (0usize, 0usize, 0usize);
        for node in policy["nodes"].as_array().unwrap() {
            let id: u32 = node["node"].as_str().unwrap().parse().unwrap();
            let lines = node["default"]["expected_stats"].as_array().unwrap();
            let source: Table = observe
                .call((
                    id,
                    oracle
                        .lua
                        .to_value(&node["default"]["expected_stats"])
                        .unwrap(),
                    node["pool"].as_str().unwrap(),
                ))
                .unwrap();
            let mut expected = Vec::new();
            for contribution in node["default"]["contributions"].as_array().unwrap() {
                let name = channels[contribution["stat"]["key"].as_str().unwrap()];
                assert_eq!(
                    contribution["stat"]["namespace"],
                    serde_json::json!({"game":"poe2","version":"owned-mechanics-v1"})
                );
                let (kind, value) = match contribution["contribution"].as_str().unwrap() {
                    "add" => {
                        assert_eq!(contribution["value"]["kind"], "integer");
                        (
                            "BASE",
                            contribution["value"]["value"].as_i64().unwrap() as f64,
                        )
                    }
                    "increase" => {
                        assert_eq!(contribution["value"]["kind"], "quantity");
                        assert_eq!(
                            contribution["value"]["value"]["unit"],
                            bindings["percentage_unit"]
                        );
                        (
                            "INC",
                            contribution["value"]["value"]["value"].as_f64().unwrap(),
                        )
                    }
                    other => panic!("unexpected passive contribution {other}"),
                };
                expected.push((name.to_owned(), kind.to_owned(), value));
            }
            totals.0 += lines.len();
            totals.1 += expected.len();
            let marker = match lines
                .iter()
                .map(|value| value.as_str().unwrap())
                .collect::<Vec<_>>()
                .as_slice()
            {
                ["+3 to all Attributes"] => Some(("BASE", 3.0)),
                ["+5 to all Attributes"] => Some(("BASE", 5.0)),
                ["3% increased Attributes"] => Some(("INC", 3.0)),
                ["7% increased Attributes"] => Some(("INC", 7.0)),
                _ => None,
            };
            let mut actual = Vec::new();
            let mut markers = Vec::new();
            for modifier in source.sequence_values::<Table>().map(Result::unwrap) {
                assert_eq!(modifier.get::<u32>("flags").unwrap(), 0);
                assert_eq!(modifier.get::<u32>("keywordFlags").unwrap(), 0);
                assert!(matches!(modifier.raw_get::<Value>(1).unwrap(), Value::Nil));
                assert_eq!(
                    modifier.get::<String>("source").unwrap(),
                    format!("Tree:{id}")
                );
                let record = (
                    modifier.get::<String>("name").unwrap(),
                    modifier.get::<String>("type").unwrap(),
                    modifier.get::<f64>("value").unwrap(),
                );
                if record.0 == "All" {
                    markers.push(record);
                } else {
                    actual.push(record);
                }
            }
            assert_eq!(actual, expected, "complete source list for node {id}");
            let expected_markers: Vec<_> = marker
                .into_iter()
                .map(|(kind, value)| ("All".to_owned(), kind.to_owned(), value))
                .collect();
            assert_eq!(
                markers, expected_markers,
                "source-only All marker for node {id}"
            );
            totals.2 += markers.len();
        }
        assert_eq!(totals, (60, 94, 17));
    }
}
