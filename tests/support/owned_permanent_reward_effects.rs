//! Eight exact Reward producer inventories with explicit ordinary global channels.
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::{OwnedDefinitionKey, StatDefinition},
    owned_rules::DefinitionRules,
    owned_schema::{DefinitionDescriptor, SchemaDefinitionId},
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    owned_mapping::OwnedIdRegistry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_tree_compact,
    },
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/permanent-reward-effects")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|object| object.is_empty()));
        &[]
    }
}
fn observation<'a>(report: &'a Value, pointer: &str) -> &'a Value {
    let matching: Vec<_> = report["observations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["pointer"] == pointer)
        .collect();
    assert_eq!(matching.len(), 1, "exact evidence pointer {pointer}");
    &matching[0]["value"]
}
fn source_records(binding: &Value) -> Vec<Value> {
    binding["effects"].as_array().unwrap().iter().map(|effect| json!({
        "flags":0,"keyword_flags":0,"name":effect["source_stat"],"source":binding["source_origin"],
        "tags":effect["source_tags"],"type":effect["source_type"],"value":effect["amount"]
    })).collect()
}
fn expected_owner(binding: &Value) -> Value {
    let nodes: Vec<_> = binding["effects"].as_array().unwrap().iter().enumerate().map(|(i,e)| json!({
        "id":format!("amount-{i}"),"expression":{"kind":"literal","value":{
            "kind":"quantity","value":{"value":e["amount"].as_f64().unwrap(),"unit":e["unit"]}
        }}
    })).collect();
    let effects: Vec<_> = binding["effects"].as_array().unwrap().iter().enumerate().map(|(i,e)| json!({
        "id":format!("grant-{i}"),"when":null,"effect":{"kind":"contribute","entity":"player",
            "stat":e["stat"],"contribution":e["contribution"],"value":format!("amount-{i}")}
    })).collect();
    json!({"owner":{"kind":"definition","value":{"kind":"reward","value":binding["reward"]}},
    "programs":{"closure":{"kind":"complete"},"members":[{
        "id":binding["program"],"context":"actor","reads":[],"nodes":nodes,"effects":effects
    }]}})
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("closure.json");
    let v: Value = read("source-vectors.json");
    assert_eq!(a["allocated_definitions"], 2);
    assert_eq!(a["registry_last_issued_before"], 0x32e5);
    assert_eq!(a["registry_last_issued_after"], 0x32e7);
    assert_eq!(a["new_complete_program_owners"], 8);
    assert_eq!(a["appended_numerical_programs"], 8);
    assert_eq!(a["appended_numerical_effects"], 12);
    assert_eq!(a["changed_schema_descriptors"], 0);
    assert_eq!(b["release"], "pob-3887ae68-permanent-reward-effects-v1");
    for field in [
        "before",
        "definitions",
        "registry",
        "roles",
        "normalization",
        "mapping",
    ] {
        assert_eq!(b[field], a[field]);
        assert_eq!(d["source"][field], a[field]);
    }
    assert_eq!(b["reward_policy"], a["rewards"]);
    assert_eq!(d["source"]["rewards"], a["rewards"]);
    assert_eq!(d["owners"], json!([]));
    for field in [
        "selection_policy_changed",
        "full_resource_defense_and_resistance_results",
        "selected_build_complete",
    ] {
        assert_eq!(a["scope"][field], false);
    }
    assert_eq!(a["scope"]["published_scalar_reducers"], 0);
    assert_eq!(a["scope"]["published_receivers"], 0);
    let bindings = b["rewards"].as_array().unwrap();
    let owners: Vec<DefinitionRules> = serde_json::from_value(c["owners"].clone()).unwrap();
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(c["definitions"].clone()).unwrap();
    let dependencies: Vec<DefinitionDescriptor> =
        serde_json::from_value(d["definitions"].clone()).unwrap();
    assert_eq!(
        (
            bindings.len(),
            owners.len(),
            definitions.len(),
            dependencies.len()
        ),
        (8, 8, 2, 18)
    );
    assert_eq!(d["absent_owners"].as_array().unwrap().len(), 8);
    for (definition, suffix) in c["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .zip(["32e6", "32e7"])
    {
        assert_eq!(
            definition["value"]["id"]["key"],
            format!("def.000000000000{suffix}")
        );
        assert_eq!(definition["kind"], "stat");
        assert_eq!(
            definition["value"]["schema"],
            json!({"kind":"known","value":{
                "value":{"kind":"quantity","value":{"unit":b["percent_unit"]}},"targets":["actor"]
            }})
        );
    }
    assert_eq!(b["percent_unit"]["key"], "def.0000000000000002");
    let expected = [
        ("002a", 10, 1),
        ("0031", 10, 1),
        ("003e", 5, 1),
        ("0041", 30, 3),
        ("0048", 5, 1),
        ("0052", 5, 1),
        ("0053", 5, 1),
        ("005a", 15, 3),
    ];
    let mut total = 0;
    for (index, (binding, (reward, amount, count))) in bindings.iter().zip(expected).enumerate() {
        assert_eq!(
            binding["reward"]["key"],
            format!("def.000000000000{reward}")
        );
        assert_eq!(binding["program"], "permanent-reward-contributions");
        assert_eq!(binding["effects"].as_array().unwrap().len(), count);
        assert_eq!(json!(owners[index]), expected_owner(binding));
        assert_eq!(d["absent_owners"][index], json!(owners[index].owner));
        let descriptor = &d["definitions"][index];
        assert_eq!(descriptor["kind"], "reward");
        assert_eq!(descriptor["value"]["id"], binding["reward"]);
        assert_eq!(descriptor["value"]["schema"]["kind"], "known");
        let declarations = descriptor["value"]["schema"]["value"]["declarations"]
            .as_object()
            .unwrap();
        assert_eq!(declarations.len(), 7);
        for declaration in declarations.values() {
            assert_eq!(
                *declaration,
                json!({"members":[],"closure":{"kind":"complete"}})
            );
        }
        let expected_names: Vec<&str> = match reward {
            "002a" | "0052" => vec!["LightningResist"],
            "0031" | "0048" => vec!["FireResist"],
            "003e" => vec!["Mana"],
            "0053" => vec!["Life"],
            _ => vec!["Armour", "Evasion", "EnergyShield"],
        };
        for (effect, source_name) in binding["effects"]
            .as_array()
            .unwrap()
            .iter()
            .zip(expected_names)
        {
            total += 1;
            assert_eq!(effect["source_stat"], source_name);
            assert_eq!(effect["amount"], amount);
            assert_eq!(effect["unit"], b["percent_unit"]);
            let (channel, suffix, kind, source_type, global) = match source_name {
                "FireResist" => ("fire_resistance", "32e6", "add", "BASE", false),
                "LightningResist" => ("lightning_resistance", "32e7", "add", "BASE", false),
                "Mana" => ("mana", "29f9", "increase", "INC", false),
                "Life" => ("life", "311a", "increase", "INC", false),
                "Armour" => ("armour", "29f2", "increase", "INC", true),
                "Evasion" => ("evasion", "29f1", "increase", "INC", true),
                "EnergyShield" => ("energy_shield", "29f0", "increase", "INC", true),
                _ => unreachable!(),
            };
            assert_eq!(effect["stat"], b["channels"][channel]);
            assert_eq!(effect["stat"]["key"], format!("def.000000000000{suffix}"));
            assert_eq!(effect["contribution"], kind);
            assert_eq!(effect["source_type"], source_type);
            assert_eq!(
                effect["source_tags"],
                if global {
                    json!([{"type":"Global"}])
                } else {
                    json!({})
                }
            );
        }
    }
    assert_eq!(total, 12);
    let artifacts = a["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        artifacts.keys().map(String::as_str).collect::<Vec<_>>(),
        ["bindings", "closure", "dependencies", "source-vectors"]
    );
    for (name, digest) in artifacts {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(*digest, hash(&bytes));
    }
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(a["source_manifest_sha256"], hash(&bytes));
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    let mut pins = BTreeSet::new();
    for pin in a["source_files"].as_array().unwrap() {
        assert!(pins.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| *p == pin)
                .count(),
            1
        );
    }
    assert!(pins.contains("src/Data/QuestRewards.lua"));
    assert!(pins.contains("src/Modules/ConfigOptions.lua"));
    let source_rows = v["reward_rows"].as_array().unwrap();
    assert_eq!(source_rows.len(), 8);
    let reports = v["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    assert_ne!(reports[0]["path"], reports[1]["path"]);
    for field in ["bytes", "sha256", "observations"] {
        assert_eq!(reports[0][field], reports[1][field]);
    }
    for (binding, source) in bindings.iter().zip(source_rows) {
        assert_eq!(binding["reward"], source["reward"]);
        let census = observation(&reports[0], source["census_pointer"].as_str().unwrap());
        let matrix = observation(&reports[0], source["matrix_pointer"].as_str().unwrap());
        assert_eq!(census["key"], binding["source_option"]);
        assert_eq!(census["source"], binding["source_origin"]);
        assert_eq!(census["source_index"], binding["source_index"]);
        assert_eq!(census["quest"]["useConfig"], true);
        assert!(census["quest"].get("questPoints").is_none());
        assert_eq!(
            matrix,
            &json!({"key":binding["source_option"],"value":binding["source_value"],"records":source_records(binding),"enemy_records":{}})
        );
        let boolean = binding["source_value"].is_boolean();
        let lane = if boolean {
            "input_boolean"
        } else {
            "input_string"
        };
        let policy = &source["policy"];
        assert_eq!(
            policy["recipe"]["tiers"],
            json!([{"selectors":[{"lane":lane,"name":binding["source_option"]}],"duplicates":"last_in_source_order"}])
        );
        assert_eq!(policy["recipe"]["codec"]["whitespace"], "exact");
        let token = if boolean {
            "true"
        } else {
            binding["source_value"].as_str().unwrap()
        };
        let codec = &policy["recipe"]["codec"]["codec"];
        assert_eq!(codec["kind"], if boolean { "boolean" } else { "option" });
        let matching: Vec<_> = codec["value"]["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["token"] == token)
            .collect();
        assert_eq!(matching.len(), 1);
        assert_eq!(
            source["selected_when"],
            json!({"kind":codec["kind"],"value":matching[0]["value"]})
        );
        let outcomes: Vec<_> = policy["outcomes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["when"] == source["selected_when"])
            .collect();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0]["outcome"]["kind"], "reward");
        assert_eq!(outcomes[0]["outcome"]["parameters"], json!([]));
        let mappings = source["mappings"].as_array().unwrap();
        assert_eq!(mappings.len(), if boolean { 2 } else { 1 });
        for (i, mapping) in mappings.iter().enumerate() {
            assert_eq!(
                mapping["source"],
                json!({"kind":"configuration","value":{
                    "key":{"kind":"text","value":binding["source_option"]},"source":if i==0 {"input"}else{"default"},
                    "role":"reward","value":{"kind":"text","value":token}
                }})
            );
            assert_eq!(mapping["outcome"]["kind"], "mapped");
            assert_eq!(
                mapping["outcome"]["value"]["target"],
                expected_owner(binding)["owner"]
            );
        }
        assert_eq!(outcomes[0]["outcome"]["selector"], mappings[0]["source"]);
        if boolean {
            assert_eq!(census["widget"], "check");
            assert_eq!(census["default"], true);
        } else {
            assert_eq!(census["widget"], "list");
            assert!(rows(&census["options"]).contains(&binding["source_value"]));
        }
    }
    for (i, report) in reports.iter().enumerate() {
        let (path, bytes, sha) = if i == 0 {
            ("evidence_json", "evidence_bytes", "evidence_sha256")
        } else {
            (
                "evidence_on_json",
                "evidence_on_bytes",
                "evidence_on_sha256",
            )
        };
        assert_eq!(report["path"], a["source_validation"][path]);
        assert_eq!(report["bytes"], a["source_validation"][bytes]);
        assert_eq!(report["sha256"], a["source_validation"][sha]);
        for original in 0..5 {
            assert_eq!(
                observation(report, &format!("/cases/{original}/name")),
                &json!(format!("original-{:02}", original + 1))
            );
            assert_eq!(
                observation(report, &format!("/cases/{original}/available")),
                &json!(true)
            );
        }
        for binding in bindings {
            let controls = rows(observation(report, "/cases/4/state/saved/controls"));
            let matching: Vec<_> = controls
                .iter()
                .filter(|r| r["key"] == binding["source_option"])
                .collect();
            assert_eq!(matching.len(), 1);
            assert_eq!(matching[0]["input"], binding["source_value"]);
            for suffix in ["quest", "modes/MAIN/quest", "modes/CALCS/quest"] {
                let actual = rows(observation(
                    report,
                    &format!("/cases/4/state/saved/{suffix}"),
                ));
                // Different options can share a Quest source string. Compare full
                // records only after the exact key/value callback join above.
                for record in source_records(binding) {
                    assert_eq!(actual.iter().filter(|row| **row == record).count(), 1);
                }
                assert!(
                    rows(observation(
                        report,
                        &format!("/cases/6/state/saved/{suffix}")
                    ))
                    .is_empty()
                );
            }
        }
    }
    assert_eq!(a["source_validation"]["status"], "passed");
    assert_eq!(a["source_validation"]["whole_build_parity"], false);
}

fn source_proof() {
    let a: Value = read("authoring.json");
    let v: Value = read("source-vectors.json");
    for pin in a["source_files"].as_array().unwrap() {
        let text = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(pin["path"].as_str().unwrap()),
        )
        .unwrap();
        let normalized = text.replace("\r\n", "\n");
        assert_eq!(normalized.len() as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(hash(normalized.as_bytes()), pin["sha256"]);
    }
    for evidence in v["reports"].as_array().unwrap() {
        let bytes = fs::read(root().join(evidence["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, evidence["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), evidence["sha256"]);
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(report["source_hash"], a["source_manifest_sha256"]);
        assert_eq!(report["source_revision"], a["source_revision"]);
        assert_eq!(report["cases"].as_array().unwrap().len(), 30);
        assert_eq!(report["evidence"]["complete_load_attempts"], 30);
        assert_eq!(report["evidence"]["finite_callback_cases"], 39);
        assert_eq!(report["evidence"]["business_method_wrappers"], false);
        let mut pointers = BTreeSet::new();
        for row in evidence["observations"].as_array().unwrap() {
            let pointer = row["pointer"].as_str().unwrap();
            assert!(pointers.insert(pointer));
            assert_eq!(report.pointer(pointer), Some(&row["value"]));
        }
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source_proof();
    assert!(prior.evaluation().is_none());
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("closure.json");
    let v: Value = read("source-vectors.json");
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "roles",
        "normalization",
        "mapping",
        "rewards",
    ] {
        assert_eq!(receipt[field], a[field]);
    }
    assert_eq!(receipt["rules"], d["source"]["rules"]);
    let dependencies: Vec<DefinitionDescriptor> =
        serde_json::from_value(d["definitions"].clone()).unwrap();
    for definition in dependencies {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|row| **row == definition)
                .count(),
            1
        );
    }
    let mapping = json!(prior.input().mapping);
    let policy = json!(prior.input().rewards);
    for source in v["reward_rows"].as_array().unwrap() {
        assert_eq!(
            policy["rules"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| **row == source["policy"])
                .count(),
            1
        );
        for row in source["mappings"].as_array().unwrap() {
            assert_eq!(
                mapping["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|entry| *entry == row)
                    .count(),
                1
            );
        }
    }
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(c["definitions"].clone()).unwrap();
    let owners: Vec<DefinitionRules> = serde_json::from_value(c["owners"].clone()).unwrap();
    let mut recipe = prior.input().recipe.clone();
    let mut registry = OwnedIdRegistry::new(recipe.registry.clone(), Default::default()).unwrap();
    assert_eq!(recipe.registry.last_issued.get(), 0x32e5);
    for definition in &definitions {
        let id = registry.allocate_definition::<StatDefinition>().unwrap();
        assert_eq!(definition.address(), id.address());
        assert!(
            !recipe
                .schema
                .definitions
                .iter()
                .any(|row| row.address() == id.address())
        );
        recipe.schema.definitions.push(definition.clone());
    }
    recipe.registry = registry.input().clone();
    assert_eq!(recipe.registry.last_issued.get(), 0x32e7);
    recipe
        .schema
        .definitions
        .sort_by_cached_key(DefinitionDescriptor::address);
    recipe.schema.release = key(b["release"].as_str().unwrap());
    let schema =
        OwnedDefinitionSchemaPackage::new(recipe.schema.clone(), Default::default()).unwrap();
    recipe.rules.definitions = schema.identity().clone();
    recipe.routing.definitions = schema.identity().clone();
    for owner in &owners {
        assert!(
            !recipe
                .rules
                .owners
                .iter()
                .any(|row| row.owner == owner.owner)
        );
        recipe.rules.owners.push(owner.clone());
    }
    let carried = transition_owned_catalog_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: prior.input().recipe.clone(),
            successor: recipe,
            mapping: prior.input().mapping.clone(),
            roles: prior.input().roles.clone(),
            normalization: prior.input().normalization.clone(),
            rewards: prior.input().rewards.clone(),
            query_sets: prior.input().query_sets.clone(),
            items: prior.input().items.clone(),
            item_source: prior.input().item_source.clone(),
        },
        CatalogAppend {
            mappings: vec![],
            source: prior.input().mapping.source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(prior.input().tree.clone().unwrap()),
        },
        Default::default(),
    )
    .unwrap();
    let mut input = prior.input().clone();
    input.recipe = carried.recipe().clone();
    input.mapping = carried.mapping().input().clone();
    input.roles = carried.roles().input().clone();
    input.normalization = carried.normalization().clone();
    input.rewards = carried.rewards().input().clone();
    input.items = carried.items().input().clone();
    input.item_source = carried.item_source().input().clone();
    input.tree = carried.tree().map(|tree| tree.input().clone());
    input.query_sets = carried.query_sets().to_vec();
    input.provenance.push(OwnedReleaseProvenance {
        kind: key("source-bound-permanent-reward-effects"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-permanent-reward-effects-v1",
            &(a, b, d, c, v),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut restored = next.input().recipe.clone();
    assert_eq!(restored.registry, *registry.input());
    restored.registry = prior.input().recipe.registry.clone();
    assert_eq!(
        restored.schema.definitions.len(),
        prior.input().recipe.schema.definitions.len() + 2
    );
    for definition in definitions {
        assert_eq!(
            restored
                .schema
                .definitions
                .iter()
                .filter(|row| **row == definition)
                .count(),
            1
        );
        restored
            .schema
            .definitions
            .retain(|row| row.address() != definition.address());
    }
    restored.schema.release = prior.input().recipe.schema.release.clone();
    assert_eq!(
        restored.rules.owners.len(),
        prior.input().recipe.rules.owners.len() + 8
    );
    for owner in owners {
        assert_eq!(
            restored
                .rules
                .owners
                .iter()
                .filter(|row| **row == owner)
                .count(),
            1
        );
        restored.rules.owners.retain(|row| row.owner != owner.owner);
    }
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "only two new channels and eight complete numerical owners; every prior field survives"
    );
    super::migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
