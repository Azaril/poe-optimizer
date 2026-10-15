//! One typed scenario input; later enemy speed/action-speed consumers stay open.
#[allow(dead_code)]
#[path = "owned_mana_supplier_sources.rs"]
mod suppliers;

use flate2::read::GzDecoder;
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::{
        ConfigurationDefaultInput, ConfigurationDispositionRule, ConfigurationInputsPolicy,
        configuration_disposition_dependencies,
    },
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry, extend_owned_recipe},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_membership_refinement_compact,
    },
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf, sync::OnceLock};

pub const INPUT: &str = "def.0000000000003370";
pub const UNIT: &str = "def.000000000000336f";
const KIND: &str = "configured-enemy-attack-cast-interval";
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn directory() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/configuration-enemy-speed-inputs")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(directory().join(name)).unwrap()).unwrap()
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn value(v: impl Serialize) -> Value {
    serde_json::to_value(v).unwrap()
}
pub fn input() -> ConfigurationDefaultInput {
    read("input.json")
}
pub fn accounting() -> Vec<ConfigurationDispositionRule> {
    read("accounting.json")
}

pub fn check_authored() {
    let i = input();
    assert_eq!(i.source_name, "enemySpeed");
    assert_eq!(i.value_input.key().as_str(), INPUT);
    assert_eq!(value(i.target), "enemy");
    assert_eq!(value(&i.constructor_default)["value"]["value"], 700.0);
    assert_eq!(value(&i.constructor_default)["value"]["unit"]["key"], UNIT);
    assert!(i.ignore_numeric_placeholder);
    assert_eq!(
        value(&i.recipe)["tiers"],
        json!([{"selectors":[{"lane":"input_number","name":"enemySpeed"}],"duplicates":"reject"}])
    );
    assert_eq!(
        value(&i.recipe)["codec"]["codec"]["value"]["scale"],
        json!({"numerator":1,"denominator":1})
    );
    let e: OwnedRecipeExtension = read("extension.json");
    assert_eq!(
        (
            e.schema.len(),
            e.owners.len(),
            e.tables.len(),
            e.receivers.len()
        ),
        (3, 0, 0, 0)
    );
    let d: Value = read("dependencies.json");
    let mut expected = d["encounter"].clone();
    expected["value"]["schema"]["value"]["external_inputs"]["members"]
        .as_array_mut()
        .unwrap()
        .push(value(&i.value_input));
    assert_eq!(value(&e.schema[0])["value"], expected);
    assert_eq!(
        value(&e.schema[1])["value"]["value"]["schema"]["value"]["dimension"],
        "time"
    );
    assert_eq!(
        value(&e.schema[2])["value"]["value"]["schema"]["value"]["targets"],
        json!(["enemy"])
    );
    let rows = accounting();
    assert_eq!(rows.len(), 2);
    for (row, lane) in rows.iter().zip(["input_number", "placeholder_number"]) {
        assert_eq!(
            value(&row.selector),
            json!({"lane":lane,"name":"enemySpeed"})
        );
        assert_eq!(
            value(&row.destinations),
            json!([{"kind":"adapter","control":{"kind":"scalar_input","input":i.value_input}}])
        );
    }
    assert_eq!(
        d["encounter_policy"]["absent_input_names"],
        json!(["enemyIsBoss", "presetBossSkills", "enemySizePreset"])
    );
    check_source();
}

pub fn check_source() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let e: Value = read("source-evidence.json");
        let manifest: Value = serde_json::from_slice(
            &fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(e["source_revision"], manifest["upstream_revision"]);
        assert_eq!(
            e["archives"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a["path"].as_str().unwrap())
                .collect::<Vec<_>>(),
            [
                "src/Modules/ConfigOptions.lua",
                "src/Modules/CalcDefence.lua",
                "src/Classes/ConfigTab.lua",
                "src/Classes/EditControl.lua"
            ]
        );
        let required_spans = [
            (
                "src/Modules/ConfigOptions.lua",
                1982,
                2139,
                "unconditional-placeholder-reset",
            ),
            (
                "src/Modules/ConfigOptions.lua",
                2189,
                2268,
                "boss-speed-replacement",
            ),
            (
                "src/Modules/ConfigOptions.lua",
                2279,
                2279,
                "millisecond-default-declaration",
            ),
            (
                "src/Modules/CalcDefence.lua",
                3409,
                3416,
                "input-placeholder-default-and-millisecond-consumer",
            ),
            (
                "src/Classes/ConfigTab.lua",
                315,
                324,
                "numeric-placeholder-notification",
            ),
            (
                "src/Classes/EditControl.lua",
                110,
                115,
                "placeholder-change-callback",
            ),
        ];
        assert_eq!(e["spans"].as_array().unwrap().len(), required_spans.len());
        for (span, (path, first, last, role)) in
            e["spans"].as_array().unwrap().iter().zip(required_spans)
        {
            assert_eq!(
                (&span["path"], &span["first"], &span["last"], &span["role"]),
                (&json!(path), &json!(first), &json!(last), &json!(role))
            );
        }
        assert_eq!(e["constructor_dependencies"].as_array().unwrap().len(), 4);
        for (dependency, name) in e["constructor_dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .zip([
                "source-comparison.json",
                "configuration-review.json",
                "configuration-bodies.json",
                "configuration-auxiliary-bodies.json",
            ])
        {
            assert_eq!(
                dependency["file"],
                format!("data/owned/poe2/3887ae68/mana-supplier-sources/{name}")
            );
            assert_eq!(
                hash(&fs::read(root().join(dependency["file"].as_str().unwrap())).unwrap()),
                dependency["sha256"]
            );
        }
        assert_eq!(
            e["license"],
            "data/owned/poe2/3887ae68/configuration-accounting/evidence/pob/LICENSE.md"
        );
        assert!(root().join(e["license"].as_str().unwrap()).is_file());
        for archive in e["archives"].as_array().unwrap() {
            let stored = fs::read(root().join(archive["file"].as_str().unwrap())).unwrap();
            assert_eq!(hash(&stored), archive["artifact_sha256"]);
            let bytes = match archive["encoding"].as_str().unwrap() {
                "utf8" => stored,
                "gzip" => {
                    let mut bytes = Vec::new();
                    GzDecoder::new(stored.as_slice())
                        .take(1024 * 1024 + 1)
                        .read_to_end(&mut bytes)
                        .unwrap();
                    bytes
                }
                "source_collection" => {
                    let collection: Value = serde_json::from_slice(&stored).unwrap();
                    let matching: Vec<_> = collection["sources"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|s| s["path"] == archive["path"])
                        .collect();
                    assert_eq!(matching.len(), 1);
                    matching[0]["source"].as_str().unwrap().as_bytes().to_vec()
                }
                _ => panic!("reviewed source encoding"),
            };
            assert!(bytes.len() <= 1024 * 1024);
            assert_eq!(bytes.len(), archive["bytes"]);
            assert_eq!(hash(&bytes), archive["sha256"]);
            assert!(
                manifest["files"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["path"] == archive["path"] && p["sha256"] == archive["sha256"])
            );
            let text = String::from_utf8(bytes).unwrap();
            for span in e["spans"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|s| s["path"] == archive["path"])
            {
                let first = span["first"].as_u64().unwrap() as usize;
                let last = span["last"].as_u64().unwrap() as usize;
                assert!(first > 0 && first <= last && last <= text.split('\n').count());
                let body = text
                    .split('\n')
                    .skip(first - 1)
                    .take(last - first + 1)
                    .collect::<Vec<_>>()
                    .join("\n");
                assert_eq!(body, span["body"]);
                assert_eq!(hash(body.as_bytes()), span["sha256"]);
            }
        }
        let acquisition = suppliers::acquisition_fixture().unwrap();
        for report in [&acquisition.jit_off, &acquisition.jit_on] {
            let catalogue = report["state"]["configuration_catalogue"]
                .as_array()
                .unwrap();
            let matches: Vec<_> = catalogue
                .iter()
                .filter(|r| r["fields"]["var"] == "enemySpeed")
                .collect();
            assert_eq!(matches.len(), 1);
            assert_eq!(matches[0]["fields"]["type"], "countAllowZero");
            assert_eq!(matches[0]["default_placeholder"], 700);
            assert!(matches[0].get("apply").is_none());
            let configs = report["state"]["configuration"].as_array().unwrap();
            assert_eq!(configs.len(), 2);
            for config in configs {
                assert!(config["input"].get("enemySpeed").is_none());
                assert_eq!(config["placeholder"]["enemySpeed"], 700);
                // countAllowZero's raw constructor state is zero; the effective
                // Placeholder is supplied by the separately reviewed callback.
                assert_eq!(config["default_state"]["enemySpeed"], 0);
            }
        }
    });
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let d: Value = read("dependencies.json");
    let b = prior.input();
    assert_eq!(value(prior.receipt().input), d["before"]);
    assert_eq!(b.recipe.registry.last_issued.get(), 0x336e);
    assert_eq!(value(&b.normalization.encounter), d["encounter_policy"]);
    let extension: OwnedRecipeExtension = read("extension.json");
    let extended = extend_owned_recipe(prior.assembled(), &extension, Default::default()).unwrap();
    assert_eq!(
        (
            extended.receipt.allocated_entries,
            extended.receipt.refined_subjects,
            extended.receipt.appended_programs
        ),
        (2, 1, 0)
    );
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
    let Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 { default_inputs, .. }) =
        &mut full.normalization.configuration_inputs
    else {
        panic!("current default-input policy")
    };
    assert!(!default_inputs.iter().any(|i| i.source_name == "enemySpeed"));
    default_inputs.push(input());
    let dependencies = configuration_disposition_dependencies(
        &full.normalization,
        carried.rewards(),
        Default::default(),
    )
    .unwrap();
    let dispositions = full
        .normalization
        .configuration_dispositions
        .as_mut()
        .unwrap();
    dispositions.dependencies = dependencies;
    dispositions.rules.extend(accounting());
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
        kind: key(KIND),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            KIND,
            &(
                read::<Value>("extension.json"),
                read::<Value>("input.json"),
                read::<Value>("accounting.json"),
                read::<Value>("dependencies.json"),
                read::<Value>("source-evidence.json"),
            ),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    // Restore only the declared descriptor additions/refinement, dependency
    // rebindings and new adapter rows; every other complete input stays exact.
    let mut inverse = next.input().clone();
    assert_eq!(&inverse.mapping, carried.mapping().input());
    assert_eq!(&inverse.roles, carried.roles().input());
    assert_eq!(&inverse.rewards, carried.rewards().input());
    assert_eq!(&inverse.items, carried.items().input());
    assert_eq!(&inverse.item_source, carried.item_source().input());
    assert_eq!(
        inverse.tree.as_ref().unwrap().content,
        b.tree.as_ref().unwrap().content
    );
    for entry in &extension.schema {
        let SchemaExtensionEntry::Definition(added) = entry else {
            panic!()
        };
        let at = inverse
            .recipe
            .schema
            .definitions
            .iter()
            .position(|v| v.address() == added.address())
            .unwrap();
        assert_eq!(&inverse.recipe.schema.definitions[at], added);
        if let Some(old) = b
            .recipe
            .schema
            .definitions
            .iter()
            .find(|v| v.address() == added.address())
        {
            inverse.recipe.schema.definitions[at] = old.clone();
        } else {
            inverse.recipe.schema.definitions.remove(at);
        }
    }
    inverse.recipe.schema.release = b.recipe.schema.release.clone();
    inverse.recipe.registry = b.recipe.registry.clone();
    assert_eq!(
        &inverse.recipe.rules.definitions,
        next.assembled().schema().identity()
    );
    assert_eq!(
        &inverse.recipe.routing.definitions,
        next.assembled().schema().identity()
    );
    inverse.recipe.rules.definitions = b.recipe.rules.definitions.clone();
    inverse.recipe.routing.definitions = b.recipe.routing.definitions.clone();
    assert!(
        inverse.recipe.rules == b.recipe.rules,
        "rules differ beyond authenticated schema rebinding"
    );
    assert!(
        inverse.recipe.routing == b.recipe.routing,
        "routing differs beyond authenticated schema rebinding"
    );
    assert!(
        inverse.recipe.schema == b.recipe.schema,
        "schema inverse differs"
    );
    assert!(inverse.recipe == b.recipe, "recipe inverse differs");
    // Successor validation authenticates these rebindings; require all semantic
    // policy contents exact before restoring their dependent identities.
    assert_eq!(
        full_policy_inverse(&inverse.normalization, carried.normalization()),
        *carried.normalization()
    );
    inverse.normalization = b.normalization.clone();
    inverse.mapping = b.mapping.clone();
    inverse.roles = b.roles.clone();
    inverse.rewards = b.rewards.clone();
    inverse.items = b.items.clone();
    inverse.item_source = b.item_source.clone();
    inverse.tree = b.tree.clone();
    assert_eq!(inverse.provenance.pop().unwrap().kind, key(KIND));
    assert!(inverse == *b, "whole release inverse differs");
    next
}

fn full_policy_inverse(
    next: &poe_optimizer_import::owned_normalize::NormalizationPolicy,
    old: &poe_optimizer_import::owned_normalize::NormalizationPolicy,
) -> poe_optimizer_import::owned_normalize::NormalizationPolicy {
    let mut inverse = next.clone();
    let Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 { default_inputs, .. }) =
        &mut inverse.configuration_inputs
    else {
        panic!()
    };
    assert_eq!(default_inputs.pop().unwrap(), input());
    let dispositions = inverse.configuration_dispositions.as_mut().unwrap();
    let rules = accounting();
    assert_eq!(
        dispositions
            .rules
            .split_off(dispositions.rules.len() - rules.len()),
        rules
    );
    dispositions.dependencies = old
        .configuration_dispositions
        .as_ref()
        .unwrap()
        .dependencies
        .clone();
    inverse
}
