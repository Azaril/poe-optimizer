//! Complete defensive passive lists against authenticated original processing.
//! Intrinsic providers are not final defences or proof of allocation activation.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Function, LuaSerdeExt, Table, Value};
use poe_optimizer_core::owned_content::digest_owned;
use poe_optimizer_import::{
    owned_passive_views::ViewRecipePolicy,
    owned_tree_catalog::{TreeCatalogInput, TreeNodeKind},
};
use poe_optimizer_pob::source;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;

const DATA: &str = "data/owned/poe2/3887ae68/passive-defence-inputs";
fn read(relative: &str) -> Json {
    serde_json::from_slice(&std::fs::read(runtime::repository().join(relative)).unwrap()).unwrap()
}
fn authored(name: &str) -> Json {
    read(&format!("{DATA}/{name}.json"))
}
fn namespace() -> Json {
    json!({"game":"poe2", "version":"owned-mechanics-v1"})
}
fn identity(kind: &str, suffix: &str) -> Json {
    json!({"kind":kind,"namespace":namespace(),"key":format!("def.000000000000{suffix}")})
}
/// Source name and BASE unit; INC always uses percentage points. None denotes
/// the existing integral attribute BASE channel, not an untyped quantity.
fn channels() -> BTreeMap<&'static str, (&'static str, Option<&'static str>)> {
    BTreeMap::from([
        ("def.0000000000001d2e", ("Str", None)),
        ("def.0000000000001d2f", ("Dex", None)),
        ("def.0000000000001d30", ("Int", None)),
        ("def.00000000000029f0", ("EnergyShield", Some("29ed"))),
        ("def.00000000000029f1", ("Evasion", Some("29ee"))),
        ("def.00000000000029f2", ("Armour", Some("29ee"))),
        ("def.00000000000029f3", ("ManaRegen", Some("29ef"))),
        (
            "def.00000000000029f4",
            ("EnergyShieldRechargeFaster", Some("0005")),
        ),
        (
            "def.00000000000029f5",
            ("EvasionGainAsDeflection", Some("0002")),
        ),
        (
            "def.00000000000029f6",
            ("ArmourAppliesToFireDamageTaken", Some("0002")),
        ),
        (
            "def.00000000000029f7",
            ("ArmourAppliesToColdDamageTaken", Some("0002")),
        ),
        (
            "def.00000000000029f8",
            ("ArmourAppliesToLightningDamageTaken", Some("0002")),
        ),
        ("def.00000000000029f9", ("Mana", Some("0003"))),
    ])
}

#[test]
fn defence_channel_definitions_preserve_exact_dimensions_units_and_actor_targets() {
    let extension = authored("extension");
    assert_eq!(extension["schema_version"], 1);
    for key in ["tables", "owners", "receivers"] {
        assert!(
            extension[key].as_array().unwrap().is_empty(),
            "unexpected {key}"
        );
    }
    let declarations = extension["schema"].as_array().unwrap();
    assert_eq!(declarations.len(), 13);
    let mut actual = BTreeMap::new();
    for row in declarations {
        assert_eq!(row["kind"], "definition");
        let descriptor = &row["value"];
        let entry = &descriptor["value"];
        let id = &entry["id"];
        assert_eq!(descriptor["kind"], id["kind"]);
        assert_eq!(id["namespace"], namespace());
        assert_eq!(entry["schema"]["kind"], "known");
        assert!(
            actual
                .insert(id["key"].as_str().unwrap(), &entry["schema"]["value"])
                .is_none()
        );
    }
    for (suffix, dimension) in [
        ("29ed", "resource_points"),
        ("29ee", "rating"),
        ("29ef", "rate"),
    ] {
        assert_eq!(
            actual[format!("def.000000000000{suffix}").as_str()],
            &json!({"dimension":dimension})
        );
    }
    for (key, (_, unit)) in channels()
        .into_iter()
        .filter(|(key, _)| key.contains("29f"))
    {
        assert_eq!(
            actual[key],
            &json!({
                "value":{"kind":"quantity","value":{"unit":identity("unit", unit.unwrap())}},
                "targets":["actor"]
            })
        );
    }
}

fn reviewed_families() -> BTreeSet<Vec<String>> {
    // Exact reviewed source lists; this literal is independent of policy files.
    serde_json::from_str(r#"[["+10 to maximum Energy Shield"],["+3% of Armour also applies to Elemental Damage","5% faster start of Energy Shield Recharge"],["+30% of Armour also applies to Elemental Damage"],["+4% of Armour also applies to Elemental Damage","Gain Deflection Rating equal to 6% of Evasion Rating"],["+5% of Armour also applies to Elemental Damage","4% faster start of Energy Shield Recharge"],["+5% of Armour also applies to Elemental Damage","Gain Deflection Rating equal to 5% of Evasion Rating"],["+6% of Armour also applies to Elemental Damage","3% faster start of Energy Shield Recharge"],["+6% of Armour also applies to Elemental Damage","Gain Deflection Rating equal to 4% of Evasion Rating"],["+8% of Armour also applies to Elemental Damage"],["10% increased Armour","+5% of Armour also applies to Elemental Damage"],["10% increased Armour","10% increased maximum Energy Shield"],["10% increased Armour","Gain Deflection Rating equal to 5% of Evasion Rating"],["10% increased Evasion Rating","+5% of Armour also applies to Elemental Damage"],["10% increased Mana Regeneration Rate"],["10% increased maximum Energy Shield","6% increased Mana Regeneration Rate"],["12% faster start of Energy Shield Recharge","+12 to Intelligence"],["12% increased Armour","12% increased maximum Energy Shield"],["12% increased Armour","4% faster start of Energy Shield Recharge"],["12% increased Evasion Rating","12% increased maximum Energy Shield"],["12% increased Evasion Rating","4% faster start of Energy Shield Recharge"],["12% increased Mana Regeneration Rate"],["12% increased maximum Energy Shield","+5% of Armour also applies to Elemental Damage"],["15% faster start of Energy Shield Recharge"],["15% increased Armour"],["15% increased Evasion Rating"],["15% increased Evasion Rating","15% increased maximum Energy Shield"],["15% increased maximum Energy Shield"],["18% increased Armour"],["18% increased maximum Energy Shield","12% increased Mana Regeneration Rate","6% increased Intelligence"],["20% increased Armour"],["20% increased Evasion Rating"],["20% increased Evasion Rating","Gain Deflection Rating equal to 5% of Evasion Rating","8% increased Dexterity"],["20% increased maximum Energy Shield"],["24% increased Evasion Rating","24% increased maximum Energy Shield"],["25% increased Armour","+15% of Armour also applies to Elemental Damage"],["25% increased Mana Regeneration Rate"],["30% increased maximum Energy Shield","+10 to Intelligence"],["30% reduced maximum Mana"],["40% increased maximum Energy Shield","10% reduced maximum Mana"],["50% increased maximum Energy Shield","20% slower start of Energy Shield Recharge"],["6% faster start of Energy Shield Recharge"],["8% increased Evasion Rating","Gain Deflection Rating equal to 4% of Evasion Rating"],["Gain Deflection Rating equal to 5% of Evasion Rating","4% faster start of Energy Shield Recharge"],["Gain Deflection Rating equal to 8% of Evasion Rating"]]"#).unwrap()
}

#[test]
fn defensive_policy_is_exact_whole_catalog_selection_with_source_and_owned_bindings() {
    let policy: ViewRecipePolicy = serde_json::from_value(authored("policy")).unwrap();
    let bindings = authored("bindings");
    let catalog: TreeCatalogInput =
        serde_json::from_value(read("data/owned/poe2/3887ae68/tree/tree-catalog.json")).unwrap();
    assert!(policy.receiver_rules.is_empty() && policy.receivers.is_empty());
    assert_eq!(policy.nodes.len(), 251);
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
    assert_eq!(families.len(), 44);
    assert_eq!(
        serde_json::to_value(&families).unwrap(),
        bindings["reviewed_families"]
    );
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
    let actual: BTreeSet<_> = policy.nodes.iter().map(|node| node.node.as_str()).collect();
    assert_eq!(actual.len(), policy.nodes.len());
    assert_eq!(
        actual, expected,
        "complete finite catalog selection, not selected-build dispatch"
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
    assert_eq!(binding_nodes.len(), policy.nodes.len());
    let tokens = read("data/owned/poe2/3887ae68/current/tree-normalization.json");
    let tokens: BTreeMap<_, _> = tokens["content"]["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|token| (token["token"].as_str().unwrap(), token))
        .collect();
    let mut seen = BTreeSet::new();
    for node in &policy.nodes {
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
        seen.insert(original.stats.clone());
    }
    assert_eq!(seen, families);
}

#[test]
fn whole_original_passive_processing_and_moddb_queries_match_defensive_contributions() {
    let policy = authored("policy");
    let channels = channels();
    for jit_enabled in [false, true] {
        let oracle = runtime::Oracle::new();
        oracle
            .lua
            .globals()
            .set("defenceJitEnabled", jit_enabled)
            .unwrap();
        oracle.lua.load(r#"
defenceTree=LoadModule("TreeData/0_5/tree")
LoadModule("Classes/PassiveTree")
function observeDefensivePassive(id,expected,pool)
 local raw=assert(defenceTree.nodes[id])
 assert(raw.skill==id)
 for _,field in ipairs({"classesStart","isAscendancyStart","isAttribute","isSwitchable","isMultipleChoice","isMultipleChoiceOption","isMastery","isOnlyImage","isKeystone","isJewelSocket","isFreeAllocate","containJewelSocket","applyToArmour","noRadius","sinister","aliasPassiveSocket"}) do
  assert(not raw[field],field)
 end
 assert(not raw.options and not raw.unlockConstraint)
 assert((raw.ascendancyName and "ascendancy" or "ordinary")==pool)
 assert(#raw.stats==#expected)
 for index,line in ipairs(expected) do assert(raw.stats[index]==line) end
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
 local db=new("ModDB"):ModDB()
 db:AddList(node.modList)
 local sums={}
 for _,mod in ipairs(node.modList) do
  local key=mod.name..":"..mod.type
  sums[key]=db:Sum(mod.type,nil,mod.name)
  assert(sums[key]==db:Sum(mod.type,{},mod.name))
 end
 return node.modList,sums
end
if defenceJitEnabled then jit.on() else jit.off();jit.flush() end
"#).set_name("@original-defensive-passive-descriptor-inputs").exec().unwrap();
        let observe: Function = oracle.lua.globals().get("observeDefensivePassive").unwrap();
        let mut totals = (0usize, 0usize, 0usize);
        for node in policy["nodes"].as_array().unwrap() {
            let id: u32 = node["node"].as_str().unwrap().parse().unwrap();
            let lines = node["default"]["expected_stats"].as_array().unwrap();
            let (source, sums): (Table, Table) = observe
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
            let mut expected_sums = BTreeMap::<String, f64>::new();
            for effect in node["default"]["contributions"].as_array().unwrap() {
                assert_eq!(effect["stat"]["kind"], "stat");
                assert_eq!(effect["stat"]["namespace"], namespace());
                let (name, base_unit) = channels[effect["stat"]["key"].as_str().unwrap()];
                let (kind, value) = match effect["contribution"].as_str().unwrap() {
                    "add" => match base_unit {
                        None => {
                            assert_eq!(effect["value"]["kind"], "integer");
                            ("BASE", effect["value"]["value"].as_i64().unwrap() as f64)
                        }
                        Some(unit) => {
                            assert_eq!(effect["value"]["kind"], "quantity");
                            assert_eq!(effect["value"]["value"]["unit"], identity("unit", unit));
                            ("BASE", effect["value"]["value"]["value"].as_f64().unwrap())
                        }
                    },
                    "increase" => {
                        assert_eq!(effect["value"]["kind"], "quantity");
                        assert_eq!(effect["value"]["value"]["unit"], identity("unit", "0002"));
                        ("INC", effect["value"]["value"]["value"].as_f64().unwrap())
                    }
                    other => panic!("unreviewed defensive contribution {other}"),
                };
                expected.push((name.to_owned(), kind.to_owned(), value));
                *expected_sums.entry(format!("{name}:{kind}")).or_default() += value;
            }
            totals.0 += lines.len();
            totals.1 += expected.len();
            let mut actual = Vec::new();
            for modifier in source.sequence_values::<Table>().map(Result::unwrap) {
                assert_eq!(modifier.get::<u32>("flags").unwrap(), 0);
                assert_eq!(modifier.get::<u32>("keywordFlags").unwrap(), 0);
                assert_eq!(
                    modifier.get::<String>("source").unwrap(),
                    format!("Tree:{id}")
                );
                let name: String = modifier.get("name").unwrap();
                let kind: String = modifier.get("type").unwrap();
                let value: f64 = modifier.get("value").unwrap();
                match modifier.raw_get::<Value>(1).unwrap() {
                    Value::Nil => {}
                    Value::Table(tag) => {
                        // This is an explicit global-scope witness, not a
                        // condition to discard or a general tag allowance.
                        assert_eq!(name, "EnergyShield");
                        assert_eq!(kind, "INC");
                        let pairs: BTreeMap<String, String> =
                            tag.pairs::<String, String>().map(Result::unwrap).collect();
                        assert_eq!(pairs, BTreeMap::from([("type".into(), "Global".into())]));
                        assert!(matches!(modifier.raw_get::<Value>(2).unwrap(), Value::Nil));
                        totals.2 += 1;
                    }
                    other => panic!("unreviewed source tag for node {id}: {other:?}"),
                }
                assert!(matches!(modifier.raw_get::<Value>(2).unwrap(), Value::Nil));
                actual.push((name, kind, value));
            }
            assert_eq!(actual, expected, "entire source list for node {id}");
            let actual_sums: BTreeMap<String, f64> =
                sums.pairs::<String, f64>().map(Result::unwrap).collect();
            assert_eq!(
                actual_sums, expected_sums,
                "whole ModDB queries for node {id}"
            );
        }
        // JIT enabled does not certify execution of a particular trace.
        assert_eq!(totals.0, 353);
        assert_eq!(totals.1, 413);
        assert_eq!(totals.2, 64, "exact global Energy Shield scope witnesses");
    }
}
