//! Publish injected critical inputs and their consumer without completing upstream coverage.
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_rules::{ContributionQuery, DefinitionRules, RuleReadSource},
};
use poe_optimizer_import::{
    owned_normalize::{ConfigurationInputsPolicy, ConfigurationNumericInput},
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_membership_refinement_compact,
    },
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/incoming-critical-effect")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn key(v: &str) -> OwnedDefinitionKey {
    v.parse().unwrap()
}

pub fn check_authored_source() -> Value {
    let proof: Value = read("source-vectors.json");
    assert_eq!(proof["status"], "passed");
    let manifest =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        proof["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&manifest))
    );
    let manifest: Value = serde_json::from_slice(&manifest).unwrap();
    assert_eq!(proof["source_revision"], manifest["upstream_revision"]);
    let pins = proof["source_files"].as_array().unwrap();
    for path in [
        "src/Modules/ConfigOptions.lua",
        "src/Classes/ConfigTab.lua",
        "src/Classes/ModDB.lua",
        "src/Modules/CalcDefence.lua",
        "src/Modules/ModParser.lua",
    ] {
        assert_eq!(pins.iter().filter(|p| p["path"] == path).count(), 1);
    }
    for pin in pins {
        assert!(manifest["files"].as_array().unwrap().contains(pin));
    }
    for path in [
        "crates/poe-optimizer-pob/tests/owned_incoming_critical_source.rs",
        "crates/poe-optimizer-pob/tests/support/owned_incoming_critical_source.lua",
        "crates/poe-optimizer-pob/tests/support/configuration_preparation_source.rs",
        "crates/poe-optimizer-pob/tests/support/json_evidence.rs",
        "tests/fixtures/builds/breadth-20260908/build-05.xml",
    ] {
        assert_eq!(
            proof["local_pins"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["path"] == path)
                .count(),
            1
        );
    }
    for pin in proof["local_pins"].as_array().unwrap() {
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], format!("{:x}", Sha256::digest(&bytes)));
    }
    assert_eq!(proof["vectors"].as_array().unwrap().len(), 52);
    proof
}

pub fn authenticate_source() {
    let proof = check_authored_source();
    let reports = proof["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    let mut all = vec![];
    for pin in reports {
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], format!("{:x}", Sha256::digest(&bytes)));
        all.push(bytes);
    }
    assert_eq!(all[0], all[1], "fixed JIT source observations agree");
    let measured: Value = serde_json::from_slice(&all[0]).unwrap();
    assert_eq!(measured["source_revision"], proof["source_revision"]);
    assert_eq!(measured["source_hash"], proof["source_manifest_sha256"]);
    for field in [
        "business_method_wrappers",
        "native_coverage",
        "whole_build_parity",
    ] {
        assert_eq!(measured["evidence"][field], false);
    }
    let observer = proof["local_pins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| {
            p["path"] == "crates/poe-optimizer-pob/tests/support/owned_incoming_critical_source.lua"
        })
        .unwrap();
    assert_eq!(measured["evidence"]["observer_sha256"], observer["sha256"]);
    let names: std::collections::BTreeSet<_> = measured["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 26);
    for name in [
        "original-05",
        "repeat-original-05",
        "warm-changed-to-original-05",
        "never",
        "always",
        "never-and-always",
        "unlucky",
        "always-and-unlucky",
        "actor-and-enemy-increase",
        "enemy-bonus-base-and-increase",
        "configured-evade",
        "damage-over-time",
        "reduction",
        "reduction-upper-clamp",
        "negative-reduction",
        "changed-saved-placeholders",
        "zero-saved-placeholders",
        "missing-saved-placeholders",
        "authored-inputs-with-conflicting-placeholders",
    ] {
        assert!(names.contains(name));
    }
    assert_eq!(measured["evidence"]["case_count"], 26);
    assert_eq!(measured["evidence"]["complete_load_attempts_per_jit"], 27);
    assert_eq!(proof["vectors"], json!(project_vectors(&measured)));
}

fn project_vectors(measured: &Value) -> Vec<Value> {
    let mut vectors = vec![];
    for case in measured["cases"].as_array().unwrap() {
        assert_eq!(case["available"], true);
        let state = &case["state"];
        assert_eq!(state["business_method_wrappers"], false);
        assert_eq!(state["original_functions_preserved"], true);
        assert_eq!(state["hook_restored"], true);
        for mode in ["main", "calcs"] {
            let view = &state[mode];
            let q = &view["queries"];
            let read = |name: &str| {
                if q[name]["result_available"] == true {
                    q[name]["result"].clone()
                } else {
                    Value::Null
                }
            };
            let override_query = &q["override_chance"];
            let bucket = &override_query["bucket_membership"];
            assert!(
                bucket.as_array().is_some_and(Vec::is_empty)
                    || bucket.as_object().is_some_and(serde_json::Map::is_empty)
            );
            assert_eq!(override_query["result_available"], false);
            vectors.push(json!({
                "name":case["name"], "mode":mode, "category":view["category"],
                "raw":[view["input"]["enemyCritChance"],view["input"]["enemyCritDamage"]],
                "flags":[read("never_crit"),read("always_crit"),read("unlucky_crit")],
                "dependencies":[read("actor_chance_increase"),read("enemy_chance_increase"),view["configured_evade"],read("enemy_bonus_base"),read("enemy_bonus_increase"),view["extra_damage_reduction"]],
                "outputs":[view["chance"],view["damage_bonus"],view["critical_effect"]]
            }));
        }
    }
    vectors
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    authenticate_source();
    assert_eq!(
        prior.receipt().input.to_string(),
        "cebc32eb98f05d6716051dde7417d26a2c7667d8361ad82afa1ea87cfc218838"
    );
    let b = prior.input();
    assert_eq!(b.recipe.registry.last_issued.get(), 0x334d);
    assert!(b.evaluation.is_none());
    let authored: OwnedRecipeExtension = read("extension.json");
    let queries: Vec<ContributionQuery> = read("queries.json");
    let inputs: Vec<ConfigurationNumericInput> = read("policy.json");
    assert_eq!(inputs.len(), 2);
    assert_eq!(queries.len(), 3);
    assert!(
        queries
            .iter()
            .all(|q| q.groups.iter().all(|g| !g.members.is_complete()))
    );
    // The extension API has no query-registry append. Stage schema first, then
    // validate query readers and their declarations together in one final recipe.
    let mut extension = authored.clone();
    let mut deferred = vec![];
    for owner in &mut extension.owners {
        let (query, ordinary): (Vec<_>, Vec<_>) = owner.programs.members.drain(..).partition(|p| {
            p.reads
                .iter()
                .any(|r| matches!(r.source, RuleReadSource::ContributionQuery { .. }))
        });
        owner.programs.members = ordinary;
        if !query.is_empty() {
            let mut row = owner.clone();
            row.programs.members = query;
            deferred.push(row);
        }
    }
    extension.owners.retain(|o| !o.programs.members.is_empty());
    let extended = extend_owned_recipe(prior.assembled(), &extension, Default::default()).unwrap();
    let carried = transition_owned_catalog_with_membership_refinement_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: b.recipe.clone(),
            successor: extended.successor,
            mapping: b.mapping.clone(),
            roles: b.roles.clone(),
            normalization: b.normalization.clone(),
            rewards: b.rewards.clone(),
            query_sets: b.query_sets.clone(),
            items: b.items.clone(),
            item_source: b.item_source.clone(),
        },
        CatalogAppend {
            mappings: vec![],
            source: b.mapping.source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(b.tree.clone().unwrap()),
        },
        extended.refinement.unwrap(),
        Default::default(),
    )
    .unwrap();
    let mut full = b.clone();
    full.recipe = carried.recipe().clone();
    full.mapping = carried.mapping().input().clone();
    full.roles = carried.roles().input().clone();
    full.normalization = carried.normalization().clone();
    full.rewards = carried.rewards().input().clone();
    full.items = carried.items().input().clone();
    full.item_source = carried.item_source().input().clone();
    full.recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(queries.clone());
    for owner in deferred {
        append_owner(&mut full.recipe.rules.owners, owner);
    }
    let Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 {
        inputs: current, ..
    }) = &mut full.normalization.configuration_inputs
    else {
        panic!("current config policy")
    };
    current.extend(inputs);
    full.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            b.tree.as_ref().unwrap().content.clone(),
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
        kind: key("incoming-critical-effect"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-incoming-critical-effect-v1",
            &(
                &authored,
                &queries,
                read::<Value>("policy.json"),
                read::<Value>("native-inputs.json"),
                read::<Value>("source-vectors.json"),
            ),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut expected_owners = b.recipe.rules.owners.clone();
    for row in authored.owners {
        append_owner(&mut expected_owners, row);
    }
    // Existing program order and every program body must remain exact.
    assert_eq!(
        expected_owners.len(),
        next.input().recipe.rules.owners.len()
    );
    for expected in expected_owners {
        let actual = next
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == expected.owner)
            .unwrap();
        assert_eq!(actual, &expected);
    }
    for old in &b.recipe.schema.definitions {
        let actual = next
            .input()
            .recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == old.address())
            .unwrap();
        let refinement =
            extension.schema.iter().find_map(|entry| match entry {
                poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry::Definition(
                    d,
                ) if d.address() == old.address() => Some(d),
                _ => None,
            });
        assert_eq!(actual, refinement.unwrap_or(old));
    }
    assert_eq!(next.input().recipe.schema.slots, b.recipe.schema.slots);
    let mut expected_queries = b.recipe.rules.contribution_queries.clone();
    expected_queries.as_mut().unwrap().members.extend(queries);
    assert_eq!(
        next.input().recipe.rules.contribution_queries,
        expected_queries
    );
    let mut expected_receivers = b.recipe.rules.receivers.clone();
    expected_receivers.members.extend(extension.receivers);
    assert_eq!(next.input().recipe.rules.receivers, expected_receivers);
    assert_eq!(next.input().recipe.rules.tables, b.recipe.rules.tables);
    assert_eq!(
        next.input().recipe.rules.effect_applications,
        b.recipe.rules.effect_applications
    );
    assert_eq!(
        next.input().recipe.rules.existing_actor_rules,
        b.recipe.rules.existing_actor_rules
    );
    assert_eq!(next.input().query_sets, b.query_sets);
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.input().evaluation.is_none());
    assert_ne!(json!(next.receipt().rules), json!(prior.receipt().rules));
    next
}
fn append_owner(owners: &mut Vec<DefinitionRules>, addition: DefinitionRules) {
    if let Some(owner) = owners.iter_mut().find(|o| o.owner == addition.owner) {
        assert!(!owner.programs.is_complete());
        assert_eq!(owner.programs.closure, addition.programs.closure);
        for program in addition.programs.members {
            assert!(!owner.programs.members.iter().any(|p| p.id == program.id));
            owner.programs.members.push(program);
        }
    } else {
        owners.push(addition);
    }
}
