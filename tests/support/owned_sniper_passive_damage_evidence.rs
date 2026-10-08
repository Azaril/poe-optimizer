//! Retained source membership and fresh imported allocations for one bounded
//! passive subtotal. Saved records and source-active records stay distinct:
//! removing saved node 95 leaves 8737 pending in native import, while PoB drops
//! both from its live tree. This helper never repairs that difference implicitly.
#[allow(dead_code)]
#[path = "owned_plain_minion_damage.rs"]
pub mod family;

use super::evidence::selected;
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{Allocation, LoadoutScope},
    owned_definitions::PassiveNodeDefId,
    owned_draft::{AllocationDraft, DraftAllocationAccess, DraftListCompletion},
    owned_rules::{DefinitionRules, StatReceiver},
    owned_schema::{DefinitionDescriptor, SchemaDefinitionId, SchemaSubject},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_recipe_extension::OwnedRecipeExtension,
    owned_release::StagedOwnedRelease,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_tree_policy::TreeTokenRole,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

const DATA: &str = "data/owned/poe2/3887ae68/";
const ORIGINAL: &str = "tests/fixtures/builds/breadth-20260908/build-05.xml";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}
fn decode<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn one<'a>(values: &'a [Value], field: &str, wanted: &Value) -> &'a Value {
    let found: Vec<_> = values.iter().filter(|v| v[field] == *wanted).collect();
    assert_eq!(found.len(), 1, "unique {field}={wanted}");
    found[0]
}
fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|object| object.is_empty()));
        &[]
    }
}

/// Preserve whole current owners, including their paired Life/cooldown effects.
/// The historical damage-only extension authenticates its numerical member; the
/// three later checked closures authenticate the complete current owner/schema.
/// No current owner or incoming contribution inventory is closed by this check.
pub fn authenticate_component(endpoint: &StagedOwnedRelease) {
    family::check_authored();
    let recipe = &endpoint.input().recipe;
    let bindings: family::Bindings = family::read("bindings.json");
    let selected = family::selected_values();
    let extension: OwnedRecipeExtension = family::read("extension.json");
    for definition in family::read::<Vec<DefinitionDescriptor>>("dependencies.json") {
        if !matches!(definition, DefinitionDescriptor::PassiveNode(_)) {
            assert!(recipe.schema.definitions.contains(&definition));
        }
    }
    for owner in family::read::<Vec<DefinitionRules>>("dependency-rules.json") {
        assert!(
            recipe.rules.owners.contains(&owner),
            "exact receiving owner"
        );
    }
    for receiver in family::read::<Vec<StatReceiver>>("receivers.json") {
        assert!(recipe.rules.receivers.members.contains(&receiver));
    }
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    let mut definitions: Vec<DefinitionDescriptor> = Vec::new();
    let mut owners: Vec<DefinitionRules> = Vec::new();
    for packet in [
        "plain-minion-owner-closure",
        "plain-minion-life-passives",
        "command-cooldown",
    ] {
        let authoring: Value = read(format!("{DATA}{packet}/authoring.json"));
        assert_eq!(authoring["source_manifest_sha256"], hash(&manifest_bytes));
        assert_eq!(authoring["source_revision"], manifest["upstream_revision"]);
        for pin in authoring["source_files"].as_array().unwrap() {
            assert_eq!(
                one(manifest["files"].as_array().unwrap(), "path", &pin["path"]),
                pin
            );
        }
        let bytes = fs::read(root().join(format!("{DATA}{packet}/closure.json"))).unwrap();
        if let Some(pins) = authoring["artifacts"].as_array() {
            let pin = one(pins, "file", &json!("closure.json"));
            assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
            assert_eq!(hash(&bytes), pin["sha256"]);
        } else {
            assert_eq!(hash(&bytes), authoring["artifact_sha256"]["closure"]);
        }
        #[derive(Deserialize)]
        struct Closure {
            definitions: Vec<DefinitionDescriptor>,
            owners: Vec<DefinitionRules>,
        }
        let closure: Closure = serde_json::from_slice(&bytes).unwrap();
        definitions.extend(closure.definitions);
        owners.extend(closure.owners);
    }
    let mut checked = BTreeSet::new();
    for binding in &bindings.nodes {
        if !selected.contains_key(&binding.source_id) {
            continue;
        }
        assert!(checked.insert(binding.node.clone()));
        let subject = SchemaSubject::Definition(binding.node.address());
        let expected: Vec<_> = owners
            .iter()
            .filter(|owner| owner.owner == subject)
            .collect();
        assert_eq!(expected.len(), 1);
        let current: Vec<_> = recipe
            .rules
            .owners
            .iter()
            .filter(|owner| owner.owner == subject)
            .collect();
        assert_eq!(
            current, expected,
            "full current owner, including paired effects"
        );
        let damage: Vec<_> = extension
            .owners
            .iter()
            .filter(|owner| owner.owner == subject)
            .collect();
        assert_eq!(damage.len(), 1);
        assert!(
            current[0]
                .programs
                .members
                .contains(&damage[0].programs.members[0])
        );
        let expected: Vec<_> = definitions
            .iter()
            .filter(|d| d.address() == binding.node.address())
            .collect();
        assert_eq!(expected.len(), 1);
        let current: Vec<_> = recipe
            .schema
            .definitions
            .iter()
            .filter(|d| d.address() == binding.node.address())
            .collect();
        assert_eq!(
            current, expected,
            "exact declaration closure, pools and adjacency"
        );
        let tokens: Vec<_> = endpoint
            .tree()
            .unwrap()
            .input()
            .content
            .tokens
            .iter()
            .filter(|t| t.token == binding.source_id)
            .collect();
        assert_eq!(tokens.len(), 1);
        assert!(
            matches!(&tokens[0].role, TreeTokenRole::Allocation { node, pool } if node == &binding.node && pool == &binding.pool)
        );
    }
    assert_eq!(checked.len(), 10);
}

#[derive(Clone)]
struct SourceCase {
    name: String,
    xml_sha256: String,
    active: BTreeMap<String, f64>,
}

fn source_cases() -> &'static [SourceCase] {
    static CASES: OnceLock<Vec<SourceCase>> = OnceLock::new();
    CASES.get_or_init(|| {
        family::check_authored();
        let authoring: Value = family::read("authoring.json");
        // Reuse the original report authenticator; no additional oracle or VM.
        let report = family::source_proof(&authoring);
        let manifest_bytes = fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
        assert_eq!(hash(&manifest_bytes), authoring["source_manifest_sha256"]);
        let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
        assert_eq!(manifest["upstream_revision"], authoring["source_revision"]);
        for pin in authoring["source_files"].as_array().unwrap() {
            assert_eq!(one(manifest["files"].as_array().unwrap(), "path", &pin["path"]), pin);
        }
        assert_eq!(report["evidence"]["complete_load_attempts_per_jit"], 38);
        let records: Vec<Value> = family::read("source-records.json");
        let original = family::selected_values();
        let mut result = Vec::new();
        for name in ["original-05", "without-plain-node"] {
            let case = one(report["cases"].as_array().unwrap(), "name", &json!(name));
            assert_eq!(case["available"], true);
            let state = &case["state"];
            for field in ["cached_outputs_preserved", "loaded_state_preserved", "original_functions_preserved", "query_state_preserved", "saved_specs_preserved", "fresh_actor_construction"] {
                assert_eq!(state[field], true, "{name}: {field}");
            }
            assert_eq!(state["source_actor_level_mutated"], false);
            assert_eq!(state["business_method_wrappers"], false);
            let mut expected = original.clone();
            if name == "without-plain-node" {
                assert_eq!(expected.remove("95"), Some(10.0));
                assert_eq!(expected.remove("8737"), Some(10.0));
            }
            let observed = rows(&state["plain_minion_damage_family"]);
            assert_eq!(observed.len(), records.len());
            let mut actual = BTreeMap::new();
            for record in &records {
                let current = one(observed, "id", &record["id"]);
                let id = record["id"].as_u64().unwrap().to_string();
                let mut expected_record = record.clone();
                expected_record["allocated"] = json!(expected.contains_key(&id));
                assert_eq!(current, &expected_record, "unchanged static record, exact live membership");
                if current["allocated"] == true {
                    assert!(actual.insert(id.clone(), *original.get(&id).unwrap()).is_none());
                }
            }
            assert_eq!(actual, expected);
            let actor = one(rows(&state["main"]["actors"]), "summon_effect_id", &json!("SummonSkeletalSnipersPlayer"));
            assert_eq!(actor["actor_profile"], "RaisedSkeletonSniper");
            assert_eq!(actor["physical_level"], 20);
            assert_eq!(actor["effective_level"], 22);
            assert_eq!(actor["actor_level"], 44);
            assert_eq!(actor["quality"], 0);
            assert_eq!(actor["fresh_actor"], true);
            assert_eq!(actor["hidden_damage_fixup"], 0);
            let action = one(rows(&actor["children"]), "effect_id", &json!("MinionMeleeBow"));
            assert_eq!(action["effect_name"], "Basic Attack");
            assert_eq!(action["selected"], true);
            assert_eq!(action["summoner_owns_actor"], true);
            let calls: Vec<_> = rows(&action["damage_calls"]).iter().filter(|c| c["damage_type"] == "Physical" && c["critical"] == false).collect();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0]["query_state_preserved"], true);
            let mut contributors = BTreeMap::new();
            let mut other = Vec::new();
            for contribution in rows(&calls[0]["increased_records"]) {
                let m = &contribution["mod"];
                if let Some(id) = m["source"].as_str().unwrap().strip_prefix("Tree:") {
                    let value = *expected.get(id).unwrap();
                    assert_eq!(contribution["value"].as_f64(), Some(value));
                    assert_eq!(m["value"].as_f64(), Some(value));
                    assert_eq!(*contribution, json!({"mod":{"flags":0,"keyword_flags":0,"name":"Damage","source":format!("Tree:{id}"),"tags":{},"type":"INC","value":m["value"]},"value":contribution["value"]}));
                    assert!(contributors.insert(id.to_owned(), value).is_none());
                } else {
                    other.push(contribution);
                }
            }
            assert_eq!(contributors, expected, "actual filtered call membership");
            assert_eq!(other.len(), 1);
            assert_eq!(other[0]["mod"]["source"], "Skill:PainOfferingPlayer");
            // Offering is present in the full source call but outside this
            // passive subtotal. Never inject its saved result into native input.
            assert_eq!(other[0]["mod"]["tags"], json!([{"effectType":"Buff","type":"GlobalEffect"}]));
            assert_eq!(expected.len(), if name == "original-05" {10} else {8});
            assert_eq!(expected.values().sum::<f64>(), if name == "original-05" {68.0} else {48.0});
            result.push(SourceCase { name: name.to_owned(), xml_sha256: case["xml_sha256"].as_str().unwrap().to_owned(), active: expected });
        }
        result
    })
}

#[derive(Clone)]
pub struct PassiveCase {
    pub case_name: String,
    /// Original imported records, with their original occurrence IDs and Pending
    /// access intact. These are the selected plain-family slice, not all passives.
    pub saved_allocations: Vec<AllocationDraft>,
    pub allocations: Option<Vec<Allocation>>,
    pub pending_nodes: Vec<PassiveNodeDefId>,
    pub observed_active_nodes: Vec<PassiveNodeDefId>,
    pub saved_subtotal: f64,
    pub expected_source_subtotal: f64,
    pub source_xml_sha256: String,
}

pub fn remove_saved_node(xml: &str, remove: &str) -> String {
    // The retained source test's exact byte-preserving mutation protocol.
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([119; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let trees: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|row| row.occurrence().name() == "Tree")
        .collect();
    assert_eq!(trees.len(), 1);
    let tree = trees[0];
    let active: usize = tree
        .attribute("activeSpec")
        .unwrap()
        .decoded()
        .unwrap()
        .parse()
        .unwrap();
    let spec = evidence
        .rows()
        .iter()
        .filter(|row| {
            row.occurrence().parent() == Some(tree.occurrence().id())
                && row.occurrence().name() == "Spec"
        })
        .nth(active - 1)
        .unwrap();
    let nodes = spec.attribute("nodes").unwrap().decoded().unwrap();
    assert_eq!(nodes.split(',').filter(|n| *n == remove).count(), 1);
    let replaced = nodes
        .split(',')
        .filter(|n| *n != remove)
        .collect::<Vec<_>>()
        .join(",");
    let range = spec.occurrence().range();
    let original = &xml[range.clone()];
    let edited = original.replacen(
        &format!("nodes=\"{nodes}\""),
        &format!("nodes=\"{replaced}\""),
        1,
    );
    assert_ne!(original, edited);
    let mut out = xml.to_owned();
    out.replace_range(range, &edited);
    out
}

/// Import an exact Original05/control XML and retain every saved selection,
/// including unresolved allocation records. Callers choose their finite family
/// only after this shared import; this does not establish active connectivity.
pub fn normalized_selected_allocations(package: &Path, xml: &str) -> Vec<AllocationDraft> {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("candidate.xml");
    fs::write(&path, xml).unwrap();
    let output = directory.path().join("imported");
    crate::release::normalize(package, &path, 5, &output);
    let selection = selected::selection(xml.as_bytes(), &output);
    let draft: Value = read(output.join("draft.json"));
    let sidecar: Value = read(output.join("sidecar.json"));
    assert_eq!(sidecar["source_sha256"], hash(xml.as_bytes()));
    let preset = one(
        draft["draft"]["allocation_presets"]["members"]
            .as_array()
            .unwrap(),
        "id",
        &selection["build"]["allocations"],
    );
    assert_eq!(preset["allocations"]["completion"]["kind"], "complete");
    let ids = preset["allocations"]["members"].as_array().unwrap();
    let all = draft["draft"]["allocations"]["members"].as_array().unwrap();
    let mut seen = BTreeSet::new();
    ids.iter()
        .map(|id| {
            let allocation: AllocationDraft = decode(one(all, "id", id));
            assert!(seen.insert(allocation.id), "distinct selected occurrences");
            allocation
        })
        .collect()
}

/// Fresh actual import of the two hash-matched source inputs. No observed output
/// is a native input; source-active sets remain separate expected observations.
pub fn normalized_allocations(package: &Path) -> Vec<PassiveCase> {
    let sources = source_cases();
    let bindings: family::Bindings = family::read("bindings.json");
    let selected_values = family::selected_values();
    let original = fs::read_to_string(root().join(ORIGINAL)).unwrap();
    sources
        .iter()
        .map(|source| {
            let xml = if source.name == "original-05" {
                original.clone()
            } else {
                remove_saved_node(&original, "95")
            };
            assert_eq!(
                hash(xml.as_bytes()),
                source.xml_sha256,
                "exact retained source mutation"
            );
            let imported = normalized_selected_allocations(package, &xml);
            assert_eq!(
                imported.len(),
                if source.name == "original-05" { 55 } else { 54 }
            );
            let mut saved_allocations = Vec::new();
            let mut pending_nodes = Vec::new();
            let mut seen = BTreeSet::new();
            let mut subtotal = 0.0;
            for allocation in imported {
                let Some(binding) = bindings
                    .nodes
                    .iter()
                    .find(|b| allocation.node.to_resolved().as_ref() == Some(&b.node))
                else {
                    continue;
                };
                assert!(selected_values.contains_key(&binding.source_id));
                assert!(seen.insert(binding.source_id.clone()));
                assert_eq!(allocation.node.to_resolved().as_ref(), Some(&binding.node));
                assert_eq!(allocation.pool.to_resolved().as_ref(), Some(&binding.pool));
                assert_eq!(allocation.scope.to_resolved(), Some(LoadoutScope::Shared));
                assert!(allocation.choices.members.is_empty());
                assert_eq!(allocation.choices.completion, DraftListCompletion::Complete);
                match &allocation.access {
                    DraftAllocationAccess::Ordinary => assert!(allocation.to_resolved().is_some()),
                    DraftAllocationAccess::Pending(pending) => {
                        assert_eq!(source.name, "without-plain-node");
                        assert_eq!(binding.source_id, "8737");
                        assert_eq!(pending.code.as_str(), "allocation-access-not-converted");
                        assert!(pending.candidates.is_empty());
                        assert!(allocation.to_resolved().is_none());
                        pending_nodes.push(binding.node.clone());
                    }
                    _ => panic!("unexpected granted access in this exact saved family"),
                }
                subtotal += binding.value;
                saved_allocations.push(allocation);
            }
            let mut expected_saved: BTreeSet<_> = selected_values.keys().cloned().collect();
            if source.name != "original-05" {
                assert!(expected_saved.remove("95"));
            }
            assert_eq!(seen, expected_saved, "all saved family rows retained");
            assert_eq!(
                subtotal,
                if source.name == "original-05" {
                    68.0
                } else {
                    58.0
                }
            );
            assert_eq!(
                pending_nodes.len(),
                usize::from(source.name != "original-05")
            );
            let allocations = saved_allocations
                .iter()
                .map(AllocationDraft::to_resolved)
                .collect::<Option<Vec<_>>>();
            assert_eq!(allocations.is_some(), source.name == "original-05");
            let observed_active_nodes = source
                .active
                .keys()
                .map(|id| {
                    bindings
                        .nodes
                        .iter()
                        .find(|b| &b.source_id == id)
                        .unwrap()
                        .node
                        .clone()
                })
                .collect();
            PassiveCase {
                case_name: source.name.clone(),
                saved_allocations,
                allocations,
                pending_nodes,
                observed_active_nodes,
                saved_subtotal: subtotal,
                expected_source_subtotal: source.active.values().sum(),
                source_xml_sha256: source.xml_sha256.clone(),
            }
        })
        .collect()
}

/// Explicit test-authored removal, not import normalization or an automatic
/// connectivity repair. Every remaining field must resolve through the existing
/// structural conversion. The caller must separately test native legality.
pub fn explicit_candidate_without(
    case: &PassiveCase,
    remove: &PassiveNodeDefId,
) -> Vec<Allocation> {
    assert_eq!(
        case.saved_allocations
            .iter()
            .filter(|a| a.node.to_resolved().as_ref() == Some(remove))
            .count(),
        1
    );
    case.saved_allocations
        .iter()
        .filter(|a| a.node.to_resolved().as_ref() != Some(remove))
        .map(|a| {
            a.to_resolved()
                .expect("explicit removal must leave fully known imported records")
        })
        .collect()
}
