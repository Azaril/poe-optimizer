//! Four complete default passive producers; recipient and whole-Life coverage stay open.
use super::migration_preservation as preservation;

use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::{OwnedDefinitionKey, StatDefinition},
    owned_rules::{ContributionKind, DefinitionRules, RuleEffectKind, RuleEntity},
    owned_schema::{DefinitionDescriptor, SchemaClosure, SchemaDefinitionId, SchemaState},
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    owned_mapping::OwnedIdRegistry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, PassiveDeclarationRefinement, SuccessorBundleInput,
        TreePolicyTransitionInput, transition_owned_catalog_with_tree_refinement_compact,
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
    root().join("data/owned/poe2/3887ae68/plain-minion-life-passives")
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

fn compare_rows(before: &Value, after: &Value) {
    let prior: Vec<DefinitionDescriptor> =
        serde_json::from_value(before["definitions"].clone()).unwrap();
    let next: Vec<DefinitionDescriptor> =
        serde_json::from_value(after["definitions"].clone()).unwrap();
    assert_eq!(prior.len(), 4);
    assert_eq!(next.len(), 5);
    for (old, new) in prior.iter().zip(&next[..4]) {
        let (DefinitionDescriptor::PassiveNode(old), DefinitionDescriptor::PassiveNode(new)) =
            (old, new)
        else {
            panic!("only the four existing passive definitions")
        };
        assert_eq!(old.id, new.id);
        let (SchemaState::Known(old_schema), SchemaState::Known(new_schema)) =
            (&old.schema, &new.schema)
        else {
            panic!("known default passive schemas")
        };
        let mut restored = new_schema.clone();
        macro_rules! port {
            ($field:ident) => {
                assert!(old_schema.declarations.$field.members.is_empty());
                assert!(!old_schema.declarations.$field.is_complete());
                assert!(new_schema.declarations.$field.members.is_empty());
                assert!(new_schema.declarations.$field.is_complete());
                restored.declarations.$field.closure =
                    old_schema.declarations.$field.closure.clone();
            };
        }
        port!(parameters);
        port!(choices);
        port!(grants);
        port!(actors);
        port!(skill_grants);
        port!(outputs);
        port!(sockets);
        assert_eq!(
            restored, *old_schema,
            "pool, adjacency and every other facet"
        );
    }
    let b: Value = read("bindings.json");
    assert_eq!(
        json!(next[4]),
        json!({
            "kind":"stat", "value":{
                "id":b["life_increase"],
                "schema":{"kind":"known","value":{
                    "value":{"kind":"quantity","value":{"unit":b["percent_unit"]}},
                    "targets":["actor"]
                }}
            }
        })
    );
    let prior: Vec<DefinitionRules> = serde_json::from_value(before["owners"].clone()).unwrap();
    let next: Vec<DefinitionRules> = serde_json::from_value(after["owners"].clone()).unwrap();
    assert_eq!(prior.len(), 4);
    assert_eq!(next.len(), 4);
    for (old, new) in prior.iter().zip(&next) {
        assert_eq!(old.owner, new.owner);
        assert!(!old.programs.is_complete());
        assert!(new.programs.is_complete());
        assert_eq!(old.programs.members.len(), 1);
        assert_eq!(new.programs.members.len(), 2);
        assert_eq!(new.programs.members[0], old.programs.members[0]);
        let damage = &old.programs.members[0];
        let life = &new.programs.members[1];
        assert_eq!(damage.id.as_str(), "ordinary-minion-damage");
        assert_eq!(life.id.as_str(), "ordinary-minion-life");
        assert!(life.reads.is_empty());
        assert_eq!(json!(life.context), "actor");
        assert_eq!(life.nodes.len(), 1);
        assert_eq!(life.effects.len(), 1);
        assert!(life.effects[0].when.is_none());
        assert!(matches!(&life.effects[0].effect,
            RuleEffectKind::Contribute { entity: RuleEntity::Player, stat, contribution: ContributionKind::Increase, value }
            if json!(stat) == b["life_increase"] && value.as_str() == "amount"));
        assert!(matches!(&damage.effects[0].effect,
            RuleEffectKind::Contribute { entity: RuleEntity::Player, stat, contribution: ContributionKind::Increase, value }
            if json!(stat) == b["damage_stat"] && value.as_str() == "amount"));
        assert_eq!(
            json!(life.nodes[0])["expression"],
            json!({
                "kind":"literal", "value":{"kind":"quantity", "value":{
                    "value":6.0, "unit":b["percent_unit"]
                }}
            })
        );
        let mut expected = json!(damage);
        expected["id"] = json!("ordinary-minion-life");
        expected["effects"][0]["effect"]["stat"] = b["life_increase"].clone();
        assert_eq!(
            json!(life),
            expected,
            "one exact additional unguarded contribution"
        );
    }
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("closure.json");
    let v: Value = read("source-vectors.json");
    assert_eq!(c["schema_version"], 1);
    assert_eq!(a["allocated_definitions"], 1);
    assert_eq!(a["registry_last_issued_before"], 0x32e4);
    assert_eq!(a["registry_last_issued_after"], 0x32e5);
    assert_eq!(a["closed_program_owners"], 4);
    assert_eq!(a["closed_empty_declaration_inventories"], 28);
    for field in [
        "before",
        "definitions",
        "registry",
        "roles",
        "normalization",
    ] {
        assert_eq!(b[field], a[field]);
    }
    assert_eq!(d["source"]["input"], a["before"]);
    assert_eq!(d["source"]["definitions"], a["definitions"]);
    assert_eq!(a["scope"]["selected_build_complete"], false);
    assert_eq!(a["scope"]["additional_numerical_programs"], 4);
    assert_eq!(a["scope"]["preserved_damage_programs"], 4);
    assert_eq!(a["scope"]["published_scalar_reducers"], 0);
    assert_eq!(a["scope"]["published_receivers"], 0);
    assert_eq!(a["scope"]["whole_life_result"], false);
    assert_eq!(
        a["scope"]["source_records_are_default_definition_modifiers"],
        true
    );
    assert_eq!(
        a["scope"]["effective_runtime_transformations_proved_absent"],
        false
    );
    assert_eq!(
        v["observation_semantics"]["modifiers"],
        "mainEnv.spec.tree.nodes[id].modList"
    );
    assert_eq!(
        v["observation_semantics"]["effective_same_definition"],
        "mainEnv.spec.nodes[id] == mainEnv.spec.tree.nodes[id]"
    );
    assert_eq!(
        v["observation_semantics"]["effective_modifiers_recorded"],
        false
    );
    let excerpts = v["source_excerpts"].as_array().unwrap();
    assert_eq!(excerpts.len(), 3);
    assert!(
        excerpts[0]["text"]
            .as_str()
            .unwrap()
            .contains("node.__index = node")
    );
    assert!(
        excerpts[1]["text"]
            .as_str()
            .unwrap()
            .contains("self.nodes[treeNode.id] = setmetatable({")
    );
    assert!(
        excerpts[1]["text"]
            .as_str()
            .unwrap()
            .contains("}, treeNode)")
    );
    assert!(
        excerpts[2]["text"]
            .as_str()
            .unwrap()
            .contains("modList:AddList(node.modList)")
    );
    assert_eq!(b["life_increase"]["key"], "def.00000000000032e5");
    assert_eq!(b["percent_unit"]["key"], "def.0000000000000002");
    assert_eq!(b["damage_stat"]["key"], "def.0000000000001d33");
    compare_rows(&d, &c);
    assert_eq!(b["nodes"].as_array().unwrap().len(), 4);
    assert_eq!(
        b["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["source_id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["19006", "229", "39461", "54453"]
    );
    for (binding, definition) in b["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .zip(c["definitions"].as_array().unwrap())
    {
        assert_eq!(binding["node"], definition["value"]["id"]);
        assert_eq!(binding["amount"], 6);
        assert_eq!(binding["damage_program"], "ordinary-minion-damage");
        assert_eq!(binding["life_program"], "ordinary-minion-life");
    }
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
    let mut seen = BTreeSet::new();
    for pin in a["source_files"].as_array().unwrap() {
        assert!(seen.insert(pin["path"].as_str().unwrap()));
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
    assert!(seen.contains("src/TreeData/0_5/tree.lua"));
    assert!(seen.contains("src/Classes/PassiveTree.lua"));
    let cat = &v["catalog"];
    let raw = fs::read(root().join(cat["path"].as_str().unwrap())).unwrap();
    assert_eq!(raw.len() as u64, cat["bytes"].as_u64().unwrap());
    assert_eq!(hash(&raw), cat["sha256"]);
    let catalog: Value = serde_json::from_slice(&raw).unwrap();
    let class_roots: BTreeSet<_> = catalog["classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["root"].as_str().unwrap())
        .collect();
    assert_eq!(json!(class_roots), cat["class_roots"]);
    let nodes = v["static_nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 4);
    for n in nodes {
        let source = n["source_id"].as_str().unwrap();
        let node = catalog["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["key"] == source)
            .unwrap();
        assert_eq!(*node, n["catalog_row"]);
        assert_eq!(
            node["kind"],
            json!({"kind":"allocation","value":{"pool":"ordinary"}})
        );
        assert_eq!(
            node["stats"],
            json!([
                "Minions have 6% increased maximum Life",
                "Minions deal 6% increased Damage"
            ])
        );
        assert_eq!(node["views"], json!([]));
        assert_eq!(node["unlock"], json!([]));
        assert!(
            catalog["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .all(|x| x["kind"]["value"]["parent"] != source)
        );
        assert!(
            catalog["edges"].as_array().unwrap().iter().all(|edge| {
                !((edge["left"] == source && class_roots.contains(edge["right"].as_str().unwrap()))
                    || (edge["right"] == source
                        && class_roots.contains(edge["left"].as_str().unwrap())))
            }),
            "no automatic ConnectedToClassStart flag"
        );
        let top_fields: BTreeSet<_> = n["text"]
            .as_str()
            .unwrap()
            .lines()
            .filter_map(|line| {
                let rest = line.strip_prefix("\t\t\t")?;
                if rest.starts_with('\t') {
                    return None;
                }
                rest.split_once('=').map(|(field, _)| field)
            })
            .collect();
        assert_eq!(
            top_fields,
            BTreeSet::from([
                "connections",
                "group",
                "icon",
                "name",
                "orbit",
                "orbitIndex",
                "skill",
                "stats",
                "stringId"
            ])
        );
        assert!(
            n["text"]
                .as_str()
                .unwrap()
                .contains(&format!("\t\t\tskill={source},\n"))
        );
        assert_eq!(n["mapping_rows"].as_array().unwrap().len(), 1);
        let binding = b["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["source_id"] == source)
            .unwrap();
        assert_eq!(
            n["mapping_rows"][0]["source"]["value"]["value"]["node_id"],
            json!({"kind":"text", "value":source})
        );
        assert_eq!(
            n["mapping_rows"][0]["outcome"]["value"]["target"]["value"]["value"],
            binding["node"]
        );
        assert_eq!(
            n["mapping_rows"][0]["source"]["value"]["value"]["view"],
            json!({"kind":"missing"})
        );
    }
    assert_eq!(cat["candidate_class_edges"], json!([]));
    assert_eq!(cat["attached_choices"], json!([]));
    let reports = v["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    assert_ne!(reports[0]["path"], reports[1]["path"]);
    for field in ["bytes", "sha256", "observations"] {
        assert_eq!(reports[0][field], reports[1][field]);
    }
    assert_eq!(a["source_validation"]["status"], "passed");
    assert_eq!(a["source_validation"]["whole_build_parity"], false);
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
        let rows: Vec<_> = report["observations"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|o| o["value"].get("modifiers").map(|_| &o["value"]))
            .collect();
        assert_eq!(rows.len(), 32);
        for row in rows {
            let id = row["id"].as_u64().unwrap();
            assert!([19006, 229, 39461, 54453].contains(&id));
            assert_eq!(
                row["stats"],
                json!([
                    "Minions have 6% increased maximum Life",
                    "Minions deal 6% increased Damage"
                ])
            );
            assert_eq!(
                row["modifiers"],
                json!([
                    {"flags":0,"keyword_flags":0,"name":"MinionModifier","source":format!("Tree:{id}"),"tags":{},"type":"LIST",
                     "value":{"mod":{"flags":0,"keywordFlags":0,"name":"Life","source":format!("Tree:{id}"),"type":"INC","value":6}}},
                    {"flags":0,"keyword_flags":0,"name":"MinionModifier","source":format!("Tree:{id}"),"tags":{},"type":"LIST",
                     "value":{"mod":{"flags":0,"keywordFlags":0,"name":"Damage","source":format!("Tree:{id}"),"type":"INC","value":6}}}
                ])
            );
        }
    }
}

fn source_proof() {
    let a: Value = read("authoring.json");
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
    let v: Value = read("source-vectors.json");
    for node in v["static_nodes"]
        .as_array()
        .unwrap()
        .iter()
        .chain(v["source_excerpts"].as_array().unwrap())
    {
        assert!(
            a["source_files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|pin| pin["path"] == node["path"])
        );
        let text = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(node["path"].as_str().unwrap()),
        )
        .unwrap();
        let first = node["first_line"].as_u64().unwrap() as usize;
        let last = node["last_line"].as_u64().unwrap() as usize;
        let exact = text
            .lines()
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(exact, node["text"]);
    }
    for evidence in v["reports"].as_array().unwrap() {
        let bytes = fs::read(root().join(evidence["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, evidence["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), evidence["sha256"]);
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(report["source_revision"], a["source_revision"]);
        assert_eq!(report["source_hash"], a["source_manifest_sha256"]);
        assert_eq!(report["cases"].as_array().unwrap().len(), 37);
        assert_eq!(report["evidence"]["complete_load_attempts_per_jit"], 38);
        let selected = report["cases"][4]["state"]["plain_minion_damage_family"]
            .as_array()
            .unwrap();
        for binding in read::<Value>("bindings.json")["nodes"].as_array().unwrap() {
            let source: u64 = binding["source_id"].as_str().unwrap().parse().unwrap();
            let rows: Vec<_> = selected.iter().filter(|row| row["id"] == source).collect();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0]["allocated"], true);
            // The witness compares the mutable per-spec wrapper to its tree
            // definition, not the contents of their modifier lists. Init creates
            // that wrapper with a metatable. The exact raw default records are
            // asserted above; this must not claim effective transformations absent.
            assert_eq!(rows[0]["effective_same_definition"], false);
            assert_eq!(rows[0]["effective_name"], rows[0]["name"]);
        }
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
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in ["definitions", "registry", "roles", "normalization"] {
        assert_eq!(receipt[field], a[field]);
    }
    let vectors: Value = read("source-vectors.json");
    for node in vectors["static_nodes"].as_array().unwrap() {
        for row in node["mapping_rows"].as_array().unwrap() {
            let expected: poe_optimizer_import::owned_mapping::MappingEntry =
                serde_json::from_value(row.clone()).unwrap();
            assert_eq!(
                prior
                    .input()
                    .mapping
                    .entries
                    .iter()
                    .filter(|entry| **entry == expected)
                    .count(),
                1
            );
        }
    }
    let mut recipe = prior.input().recipe.clone();
    let old_definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(d["definitions"].clone()).unwrap();
    let next_definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(c["definitions"].clone()).unwrap();
    let mut registry = OwnedIdRegistry::new(recipe.registry.clone(), Default::default()).unwrap();
    assert_eq!(recipe.registry.last_issued.get(), 0x32e4);
    let new_stat = registry.allocate_definition::<StatDefinition>().unwrap();
    assert_eq!(json!(new_stat), b["life_increase"]);
    recipe.registry = registry.input().clone();
    assert_eq!(recipe.registry.last_issued.get(), 0x32e5);
    let appended_definition = next_definitions[4].clone();
    assert_eq!(appended_definition.address(), new_stat.address());
    assert!(
        !recipe
            .schema
            .definitions
            .iter()
            .any(|x| x.address() == new_stat.address())
    );
    let supporting: Vec<DefinitionDescriptor> =
        serde_json::from_value(d["supporting_definitions"].clone()).unwrap();
    for expected in &supporting {
        assert_eq!(
            recipe
                .schema
                .definitions
                .iter()
                .filter(|x| *x == expected)
                .count(),
            1
        );
    }
    let mut nodes = Vec::new();
    for (old, new) in old_definitions
        .iter()
        .zip(next_definitions[..4].iter().cloned())
    {
        let row = recipe
            .schema
            .definitions
            .iter_mut()
            .find(|x| x.address() == old.address())
            .unwrap();
        assert_eq!(row, old);
        let DefinitionDescriptor::PassiveNode(passive) = &new else {
            unreachable!()
        };
        nodes.push(passive.id.clone());
        *row = new;
    }
    let old_owners: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
    let next_owners: Vec<DefinitionRules> = serde_json::from_value(c["owners"].clone()).unwrap();
    for (old, new) in old_owners.iter().zip(next_owners) {
        let row = recipe
            .rules
            .owners
            .iter_mut()
            .find(|x| x.owner == old.owner)
            .unwrap();
        assert_eq!(row, old);
        *row = new;
    }
    recipe.schema.definitions.push(appended_definition.clone());
    recipe
        .schema
        .definitions
        .sort_by_cached_key(DefinitionDescriptor::address);
    recipe.schema.release = key(b["release"].as_str().unwrap());
    let schema =
        OwnedDefinitionSchemaPackage::new(recipe.schema.clone(), Default::default()).unwrap();
    recipe.rules.definitions = schema.identity().clone();
    recipe.routing.definitions = schema.identity().clone();
    let refined = transition_owned_catalog_with_tree_refinement_compact(
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
        PassiveDeclarationRefinement {
            schema_version: 1,
            before: prior.assembled().schema().identity().clone(),
            after: schema.identity().clone(),
            nodes,
        },
        Default::default(),
    )
    .unwrap();
    let mut input = prior.input().clone();
    input.recipe = refined.recipe().clone();
    input.mapping = refined.mapping().input().clone();
    input.roles = refined.roles().input().clone();
    input.normalization = refined.normalization().clone();
    input.rewards = refined.rewards().input().clone();
    input.items = refined.items().input().clone();
    input.item_source = refined.item_source().input().clone();
    input.tree = refined.tree().map(|tree| tree.input().clone());
    input.query_sets = refined.query_sets().to_vec();
    input.provenance.push(OwnedReleaseProvenance {
        kind: key("source-bound-plain-minion-life-passives"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-plain-minion-life-passives-v1",
            &(a, b, d, c, read::<Value>("source-vectors.json")),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut restored = next.input().recipe.clone();
    assert_eq!(restored.registry, *registry.input());
    restored.registry = prior.input().recipe.registry.clone();
    assert_eq!(
        restored
            .schema
            .definitions
            .iter()
            .filter(|x| **x == appended_definition)
            .count(),
        1
    );
    restored
        .schema
        .definitions
        .retain(|x| x.address() != new_stat.address());
    for old in old_definitions {
        let row = restored
            .schema
            .definitions
            .iter_mut()
            .find(|x| x.address() == old.address())
            .unwrap();
        *row = old;
    }
    restored.schema.release = prior.input().recipe.schema.release.clone();
    for old in old_owners {
        let row = restored
            .rules
            .owners
            .iter_mut()
            .find(|x| x.owner == old.owner)
            .unwrap();
        assert_eq!(row.programs.closure, SchemaClosure::Complete);
        assert_eq!(row.programs.members.len(), old.programs.members.len() + 1);
        assert_eq!(
            row.programs.members[..old.programs.members.len()],
            old.programs.members
        );
        *row = old;
    }
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "only four contributions, four owner and declaration closures, and the one new Stat"
    );
    preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
