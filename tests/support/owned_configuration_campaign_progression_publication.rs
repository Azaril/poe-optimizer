//! Publish the authored progression fact without changing existing build meaning.
use crate::selected;
#[allow(dead_code)]
#[path = "owned_configuration_accounting.rs"]
mod edits;
#[path = "owned_identity_correspondence.rs"]
mod identity;
#[allow(dead_code)]
#[path = "owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "owned_release_fixture.rs"]
mod release;
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_content::digest_owned,
    owned_rules::{ContributionQuery, DefinitionRules, SupportSourceDomainDeclaration},
};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_mapping::MappingEntry,
    owned_normalize::{
        ConfigurationDispositionRule, ConfigurationRewardControl,
        ConfigurationRewardInventoryPolicy, configuration_disposition_dependencies,
    },
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_reward_policy::{OwnedRewardPolicy, RewardRuleInput},
    owned_source::SourceProjectEvidence,
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_tree_compact,
    },
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const FAMILY: &str = "data/owned/poe2/3887ae68/configuration-campaign-progression";
const SOURCE: &str = "configBossFaceBroken";
const REWARD: &str = "def.00000000000033b4";
const BEFORE: &str = "5b611f43a0eedbf556799bcbd2c295a85d58d3264a7306c24602fd4d41e6e7b6";
const KIND: &str = "configuration-campaign-progression";

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FAMILY)
}
fn read_path(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
pub fn read(name: &str) -> Value {
    let path = data().join(name);
    assert!(fs::metadata(&path).unwrap().len() <= 2 * 1024 * 1024);
    read_path(path)
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Consumer {
    owners: Vec<DefinitionRules>,
    queries: Vec<ContributionQuery>,
    #[serde(default)]
    support_source_domains: Vec<SupportSourceDomainDeclaration>,
}
pub fn review_authoring(a: &Value) -> Result<(), String> {
    let require = |ok, why: &str| if ok { Ok(()) } else { Err(why.to_string()) };
    require(
        a["schema_version"] == 1 && a["before"] == BEFORE,
        "exact predecessor",
    )?;
    let expected = [
        "definitions.json",
        "slots.json",
        "extension.json",
        "mapping.json",
        "reward.json",
        "inventory.json",
        "accounting.json",
        "consumer.json",
        "bindings.json",
        "dependencies.json",
        "source-evidence.json",
    ];
    let rows = a["artifacts"].as_array().ok_or("artifact inventory")?;
    require(rows.len() == expected.len(), "exact artifact census")?;
    for name in expected {
        let pins: Vec<_> = rows
            .iter()
            .filter(|p| p["path"] == format!("{FAMILY}/{name}"))
            .collect();
        require(pins.len() == 1, "exact unique artifact path")?;
        let bytes = fs::read(data().join(name)).map_err(|e| e.to_string())?;
        require(
            bytes.len() <= 2 * 1024 * 1024 && pins[0]["bytes"] == bytes.len(),
            "bounded artifact bytes",
        )?;
        require(
            pins[0]["sha256"] == format!("{:x}", Sha256::digest(&bytes)),
            "artifact hash",
        )?;
    }
    require(
        a["scope"]["publication_ready"] == true,
        "publication readiness",
    )?;
    for field in [
        "complete_request",
        "candidate_source_closure",
        "query_closure_changed",
        "existing_owner_closure_changed",
        "ready_for_native_admission",
    ] {
        require(a["scope"][field] == false, "partial gameplay scope")?;
    }
    require(
        read("dependencies.json")["before"] == a["before"],
        "dependency predecessor",
    )?;
    Ok(())
}

fn remove_exact<T: PartialEq>(rows: &mut Vec<T>, expected: &T) {
    let matches: Vec<_> = rows
        .iter()
        .enumerate()
        .filter_map(|(i, v)| (v == expected).then_some(i))
        .collect();
    assert_eq!(matches.len(), 1, "one exact appended record");
    rows.remove(matches[0]);
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let authored = read("authoring.json");
    review_authoring(&authored).unwrap();
    assert_eq!(json!(prior.receipt().input), authored["before"]);
    let old = prior.input();
    assert!(old.evaluation.is_none());
    assert_eq!(old.recipe.registry.last_issued.get(), 0x33b3);
    assert_eq!(
        old.rewards.rules.len(),
        17,
        "historical finite recipe census"
    );
    let extension: OwnedRecipeExtension = serde_json::from_value(read("extension.json")).unwrap();
    assert_eq!(extension.schema.len(), 4);
    assert!(
        extension.owners.is_empty()
            && extension.tables.is_empty()
            && extension.receivers.is_empty()
    );
    let extended = extend_owned_recipe(prior.assembled(), &extension, Default::default()).unwrap();
    assert_eq!(
        (
            extended.receipt.allocated_entries,
            extended.receipt.refined_subjects,
            extended.receipt.appended_programs
        ),
        (4, 0, 0)
    );
    assert!(extended.refinement.is_none());
    let mapping: MappingEntry = serde_json::from_value(read("mapping.json")).unwrap();
    let carried = transition_owned_catalog_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: old.recipe.clone(),
            successor: extended.successor,
            mapping: old.mapping.clone(),
            roles: old.roles.clone(),
            normalization: old.normalization.clone(),
            rewards: old.rewards.clone(),
            query_sets: old.query_sets.clone(),
            items: old.items.clone(),
            item_source: old.item_source.clone(),
        },
        CatalogAppend {
            mappings: vec![mapping.clone()],
            source: old.mapping.source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(old.tree.clone().unwrap()),
        },
        Default::default(),
    )
    .unwrap();

    // The transition validates all old contracts; independently invert its exact
    // descriptor and registry additions, including canonical inventory order.
    let mut schema = carried.recipe().schema.clone();
    for entry in &extension.schema {
        match entry {
            SchemaExtensionEntry::Definition(d) => remove_exact(&mut schema.definitions, d),
            SchemaExtensionEntry::Slot(s) => remove_exact(&mut schema.slots, s),
        }
    }
    schema.release = old.recipe.schema.release.clone();
    assert!(schema == old.recipe.schema, "exact schema inverse");
    let mut registry = json!(carried.recipe().registry);
    let original_registry = json!(old.recipe.registry);
    for (index, entry) in extension.schema.iter().enumerate().rev() {
        let sequence = 0x33b4 + index;
        let added = registry["entries"].as_array_mut().unwrap().pop().unwrap();
        assert_eq!(added["sequence"], sequence);
        assert_eq!(added["target"], json!(entry.subject()));
        assert_eq!(added["state"], json!({"kind":"active"}));
    }
    assert_eq!(
        registry["revision"].as_u64().unwrap(),
        original_registry["revision"].as_u64().unwrap() + 4
    );
    assert_eq!(registry["last_issued"], 0x33b7);
    registry["revision"] = original_registry["revision"].clone();
    registry["last_issued"] = original_registry["last_issued"].clone();
    assert_eq!(registry, original_registry);
    let mut rules = carried.recipe().rules.clone();
    for domain in &extension.support_source_domains {
        remove_exact(
            &mut rules.support_discovery.as_mut().unwrap().providers,
            domain,
        );
    }
    rules.definitions = old.recipe.rules.definitions.clone();
    assert!(
        rules == old.recipe.rules,
        "every old rule, query and closure stays exact"
    );
    let mut routes = carried.recipe().routing.clone();
    routes.definitions = old.recipe.routing.definitions.clone();
    assert!(routes == old.recipe.routing);
    let mut entries = carried.mapping().input().entries.clone();
    remove_exact(&mut entries, &mapping);
    assert_eq!(entries, old.mapping.entries);
    assert_eq!(carried.rewards().input().rules, old.rewards.rules);

    let mut full = old.clone();
    full.recipe = carried.recipe().clone();
    full.mapping = carried.mapping().input().clone();
    full.roles = carried.roles().input().clone();
    full.normalization = carried.normalization().clone();
    full.rewards = carried.rewards().input().clone();
    full.items = carried.items().input().clone();
    full.item_source = carried.item_source().input().clone();
    full.tree = Some(carried.tree().unwrap().input().clone());
    full.query_sets = carried.query_sets().to_vec();
    let carried_input = full.clone();
    let consumer: Consumer = serde_json::from_value(read("consumer.json")).unwrap();
    assert_eq!(consumer.owners.len(), 1);
    assert_eq!(consumer.owners[0].programs.members.len(), 2);
    assert!(consumer.owners[0].programs.is_complete());
    assert_eq!(consumer.queries.len(), 2);
    assert_eq!(
        extension.support_source_domains.len() + consumer.support_source_domains.len(),
        1
    );
    full.recipe.rules.owners.extend(consumer.owners.clone());
    full.recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(consumer.queries.clone());
    full.recipe
        .rules
        .support_discovery
        .as_mut()
        .unwrap()
        .providers
        .extend(consumer.support_source_domains.clone());
    let reward: RewardRuleInput = serde_json::from_value(read("reward.json")).unwrap();
    full.rewards.rules.push(reward.clone());
    let rewards = OwnedRewardPolicy::new(
        full.rewards.clone(),
        carried.mapping(),
        carried.assembled().schema(),
        Default::default(),
    )
    .unwrap();
    let controls: Vec<ConfigurationRewardControl> =
        serde_json::from_value(read("inventory.json")).unwrap();
    let accounting: Vec<ConfigurationDispositionRule> =
        serde_json::from_value(read("accounting.json")).unwrap();
    assert_eq!((controls.len(), accounting.len()), (2, 2));
    let Some(ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 {
        controls: existing,
        reward_policy,
        ..
    }) = &mut full.normalization.configuration_reward_inventory
    else {
        panic!("existing checked reward inventory")
    };
    assert_eq!(existing.len(), 17);
    existing.extend(controls.clone());
    *reward_policy = *rewards.identity();
    let dependencies =
        configuration_disposition_dependencies(&full.normalization, &rewards, Default::default())
            .unwrap();
    let disposition = full
        .normalization
        .configuration_dispositions
        .as_mut()
        .unwrap();
    disposition.dependencies = dependencies;
    disposition.rules.extend(accounting.clone());
    full.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            old.tree.as_ref().unwrap().content.clone(),
            carried.assembled().registry(),
            carried.assembled().schema(),
            carried.mapping(),
            &full.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(KIND, &authored, 2 * 1024 * 1024).unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    assert!(next.input().evaluation.is_none());
    assert_eq!(next.query_sets(), prior.query_sets());
    let mut inverse = next.input().clone();
    for row in &consumer.owners {
        remove_exact(&mut inverse.recipe.rules.owners, row);
    }
    for row in &consumer.queries {
        remove_exact(
            &mut inverse
                .recipe
                .rules
                .contribution_queries
                .as_mut()
                .unwrap()
                .members,
            row,
        );
    }
    for row in &consumer.support_source_domains {
        remove_exact(
            &mut inverse
                .recipe
                .rules
                .support_discovery
                .as_mut()
                .unwrap()
                .providers,
            row,
        );
    }
    remove_exact(&mut inverse.rewards.rules, &reward);
    let Some(ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 {
        controls: rows,
        reward_policy,
        ..
    }) = &mut inverse.normalization.configuration_reward_inventory
    else {
        panic!()
    };
    for row in &controls {
        remove_exact(rows, row);
    }
    assert_eq!(*reward_policy, *next.rewards().identity());
    *reward_policy = *carried.rewards().identity();
    let disposition = inverse
        .normalization
        .configuration_dispositions
        .as_mut()
        .unwrap();
    assert_eq!(
        disposition.dependencies,
        configuration_disposition_dependencies(
            next.normalization(),
            next.rewards(),
            Default::default()
        )
        .unwrap()
    );
    for row in &accounting {
        remove_exact(&mut disposition.rules, row);
    }
    disposition.dependencies = carried_input
        .normalization
        .configuration_dispositions
        .as_ref()
        .unwrap()
        .dependencies
        .clone();
    assert_eq!(
        inverse.tree.as_ref().unwrap().content,
        carried_input.tree.as_ref().unwrap().content
    );
    inverse.tree = carried_input.tree.clone();
    assert_eq!(
        inverse.provenance.pop().unwrap().prior_input,
        prior.receipt().input
    );
    assert!(
        inverse == carried_input,
        "only the exact new fact, authority and source receipts may change"
    );
    next
}

fn publish(input: &Path, output: &Path) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn source(xml: &[u8]) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([151; 16]),
        Default::default(),
    )
    .unwrap()
}
fn source_rows(e: &SourceProjectEvidence, element: &str) -> Vec<usize> {
    e.rows()
        .iter()
        .filter(|r| {
            r.occurrence().name() == element
                && r.attribute("name")
                    .is_some_and(|a| a.decoded().ok() == Some(SOURCE))
        })
        .map(|r| r.occurrence().id().ordinal() as usize)
        .collect()
}
fn rewards(draft: &Value) -> Vec<&Value> {
    draft["draft"]["rewards"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["definition"]["value"]["key"] == REWARD)
        .collect()
}
fn assert_reward(reward: &Value, count: i64) {
    assert_eq!(
        *reward,
        json!({"id":reward["id"],
        "definition":{"kind":"known","value":read("definitions.json")[0]["value"]["id"]},
        "parameters":{"members":[{
            "slot":{"kind":"known","value":read("reward.json")["outcomes"]["parameter"]},
            "value":{"kind":"known","value":{"kind":"integer","value":count}}
        }],"completion":{"kind":"complete"}}})
    );
}
fn lineage(value: &mut Value, before: &Value, after: &Value) {
    assert_eq!(&value["lineage"], after);
    value["lineage"] = before.clone();
}
fn issued(value: &Value) -> u64 {
    u64::from_str_radix(value.as_str().unwrap(), 16).unwrap()
}

pub fn run(prior_path: PathBuf, out: PathBuf) {
    assert!(!out.exists());
    let prior_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = stage(&prior);
    assert_eq!(next.receipt().query_rows, 110);
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    write(out.join("receipt.json"), next.receipt());
    let package = out.join("package");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(
        publish(&package, &out.join("rebuilt")),
        json!(next.receipt())
    );
    assert_eq!(
        release::inventory(&package),
        release::inventory(&out.join("rebuilt"))
    );
    assert_eq!(release::load(&package).receipt(), next.receipt());
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut results = vec![];
    for case in 1..=5 {
        let xml = root.join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let bytes = fs::read(&xml).unwrap();
        let imported = source(&bytes);
        let e = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
        assert!(source_rows(&e, "Input").is_empty());
        let leaves = source_rows(&e, "Placeholder");
        assert_eq!(leaves.len(), 1);
        let leaf = leaves[0];
        let scope = e.rows()[leaf].occurrence().parent().unwrap().ordinal() as usize;
        assert_eq!(e.rows()[scope].occurrence().name(), "ConfigSet");
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&prior_path, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        let a = read_path(old.join("draft.json"));
        let actual = read_path(new.join("draft.json"));
        let mut b = actual.clone();
        assert!(rewards(&a).is_empty());
        let added = rewards(&b);
        assert_eq!(added.len(), 1);
        let reward = added[0].clone();
        let count = [55, 57, 57, 56, 56][case - 1];
        assert_reward(&reward, count);
        remove_exact(
            b["draft"]["rewards"]["members"].as_array_mut().unwrap(),
            &reward,
        );
        assert_eq!(
            b["draft"]["choice_presets"]["members"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        remove_exact(
            b["draft"]["choice_presets"]["members"][0]["rewards"]["members"]
                .as_array_mut()
                .unwrap(),
            &reward["id"],
        );
        let before_lineage = a["draft"]["allocator"]["lineage"].clone();
        let after_lineage = b["draft"]["allocator"]["lineage"].clone();
        assert_ne!(before_lineage, after_lineage);
        assert_eq!(
            issued(&b["draft"]["allocator"]["last_issued"]),
            issued(&a["draft"]["allocator"]["last_issued"]) + 1
        );
        b["draft"]["allocator"] = a["draft"]["allocator"].clone();
        let mut ids = BTreeMap::new();
        identity::correspond(
            &a,
            &mut b,
            &mut ids,
            "whole draft after exact progression inverse",
        );
        let sa = read_path(old.join("sidecar.json"));
        let mut sb = read_path(new.join("sidecar.json"));
        preservation::authenticate(&sa, &old, &prior_path, case, &prior);
        preservation::authenticate(&sb, &new, &package, case, &next);
        assert_eq!(sa["allocator_after"], a["draft"]["allocator"]);
        assert_eq!(sb["allocator_after"], actual["draft"]["allocator"]);
        let issue = json!({"kind":"issue","value":a["draft"]["choice_presets"]["members"][0]["choices"]["completion"]["id"]});
        let new_issue = json!({"kind":"issue","value":actual["draft"]["choice_presets"]["members"][0]["choices"]["completion"]["id"]});
        let reward_link = json!({"kind":"reward","value":reward["id"]});
        let choice_link = json!({"kind":"choice_preset","value":actual["draft"]["choice_presets"]["members"][0]["id"]});
        let linked: Vec<_> = sb["origins"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| o["links"].as_array().unwrap().contains(&reward_link))
            .map(|o| o["source"]["ordinal"].as_u64().unwrap() as usize)
            .collect();
        assert_eq!(
            linked,
            vec![scope, leaf],
            "the Reward has exactly its scope and selected source"
        );
        remove_exact(
            sb["origins"][scope]["links"].as_array_mut().unwrap(),
            &reward_link,
        );
        assert_eq!(sa["origins"][leaf]["links"], json!([issue]));
        assert_eq!(
            sb["origins"][leaf]["disposition"],
            json!({"kind":"contributes"})
        );
        let mut expected_links = sb["origins"][leaf]["links"].as_array().unwrap().clone();
        remove_exact(&mut expected_links, &reward_link);
        remove_exact(&mut expected_links, &choice_link);
        assert!(expected_links.is_empty());
        sb["origins"][leaf]["links"] = json!([new_issue]);
        sb["origins"][leaf]["disposition"] = sa["origins"][leaf]["disposition"].clone();
        let old_items = sa["item_texts"].as_array().unwrap();
        let new_items = sb["item_texts"].as_array_mut().unwrap();
        assert_eq!(old_items.len(), new_items.len());
        for (before, after) in old_items.iter().zip(new_items) {
            lineage(
                &mut after["attribution"]["source"],
                &before_lineage,
                &after_lineage,
            );
            lineage(
                &mut after["attribution"]["source"]["allocator"],
                &before_lineage,
                &after_lineage,
            );
            for field in ["policy", "item_lines"] {
                after["attribution"][field] = before["attribution"][field].clone();
            }
        }
        for field in [
            "policy",
            "mapping",
            "registry",
            "definitions",
            "skill_roles",
            "reward_policy",
            "item_policy",
            "item_source_policy",
            "tree_policy",
            "draft",
            "allocator_after",
        ] {
            sb[field] = sa[field].clone();
        }
        for field in ["allocator_before", "source_allocator"] {
            lineage(&mut sb[field], &before_lineage, &after_lineage);
            assert_eq!(sb[field], sa[field]);
        }
        identity::correspond(
            &sa,
            &mut sb,
            &mut ids,
            "whole sidecar after exact progression inverse",
        );
        let mut selection = selected::selection(&bytes, &new);
        identity::relocate(&mut selection, &ids);
        assert_eq!(selection, selected::selection(&bytes, &old));
        let before = selected::finalize_with_definitions(
            &bytes,
            &old,
            &out.join(format!("prior-selected-{case:02}.json")),
            &prior_path.join("schema.json"),
        );
        let after = selected::finalize_with_definitions(
            &bytes,
            &new,
            &out.join(format!("selected-{case:02}.json")),
            &package.join("schema.json"),
        );
        assert_eq!(after["intent_validation"]["schema_issues"], json!([]));
        let mut issues = after["finalization"]["issues"].clone();
        identity::relocate(&mut issues, &ids);
        assert_eq!(issues, before["finalization"]["issues"]);
        assert_eq!(
            issues.as_array().unwrap().len(),
            [106, 117, 109, 123, 4][case - 1]
        );
        let query_file = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(prior_path.join(&query_file)).unwrap(),
            fs::read(package.join(&query_file)).unwrap()
        );
        results.push(json!({"original":case,"saved_count":count,"whole_draft_inverse":true,"whole_sidecar_inverse":true,"injective_identity_correspondence":true,"selected_issues":issues.as_array().unwrap().len()}));
    }
    let original =
        fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let controls = [
        ("zero-input", Some("0"), Some("56"), Some(56)),
        ("positive-input", Some("7"), Some("56"), Some(7)),
        ("zero-both", Some("0"), Some("0"), None),
        ("absent", None, None, None),
        (
            "malformed-losing-placeholder",
            Some("7"),
            Some("wrong"),
            None,
        ),
        ("fractional-input", Some("7.5"), Some("56"), None),
    ];
    for (name, input, placeholder, count) in controls {
        let xml = edits::field(
            &edits::field(
                &original,
                "Placeholder",
                SOURCE,
                placeholder.map(|s| ("number", s)),
            ),
            "Input",
            SOURCE,
            input.map(|s| ("number", s)),
        );
        let path = out.join(format!("control-{name}.xml"));
        fs::write(&path, &xml).unwrap();
        let target = out.join(format!("control-{name}"));
        release::normalize(&package, &path, 5, &target);
        let draft = read_path(target.join("draft.json"));
        let sidecar = read_path(target.join("sidecar.json"));
        let added = rewards(&draft);
        assert_eq!(added.len(), usize::from(count.is_some()), "{name}");
        let imported = source(xml.as_bytes());
        let e = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
        if let Some(count) = count {
            assert_reward(added[0], count);
            let reward = json!({"kind":"reward","value":added[0]["id"]});
            let choice = json!({"kind":"choice_preset","value":draft["draft"]["choice_presets"]["members"][0]["id"]});
            let selected_element = if input.is_some_and(|v| v != "0") {
                "Input"
            } else {
                "Placeholder"
            };
            for element in ["Input", "Placeholder"] {
                for at in source_rows(&e, element) {
                    let origin = &sidecar["origins"][at];
                    let mut links = origin["links"].as_array().unwrap().clone();
                    remove_exact(&mut links, &choice);
                    if element == selected_element {
                        remove_exact(&mut links, &reward);
                        assert_eq!(origin["disposition"], json!({"kind":"contributes"}));
                    } else {
                        assert_eq!(
                            origin["disposition"],
                            json!({"kind":"source_only","value":"unselected-config-projection"})
                        );
                    }
                    assert!(
                        links.is_empty(),
                        "{name}: exact selected and losing source receipts"
                    );
                }
            }
        } else {
            assert_eq!(
                draft["draft"]["choice_presets"]["members"][0]["rewards"]["completion"]["kind"],
                "pending",
                "{name}: unresolved progression cannot complete reward inventory"
            );
            for at in source_rows(&e, "Input")
                .into_iter()
                .chain(source_rows(&e, "Placeholder"))
            {
                assert!(
                    sidecar["origins"][at]["links"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|l| l["kind"] == "issue"),
                    "{name}: unresolved saved lane"
                );
            }
        }
    }
    assert_eq!(release::inventory(&prior_path), prior_files);
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":results,"queries":110,"historical_reward_recipes":17,"added_reward_recipes":1,"new_definitions":3,"new_slots":1,"new_programs":2,"mutation_controls":controls.len(),"rebuild_byte_identical":true,"complete_original_builds":0,"native_admission":false}),
    );
}
