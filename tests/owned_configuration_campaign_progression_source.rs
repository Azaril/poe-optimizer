//! Portable source review and owned-format contracts; no source VM execution.
use flate2::read::GzDecoder;
use poe_optimizer_core::{owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_import::{
    owned_mapping::MappingEntry,
    owned_normalize::{ConfigurationDispositionRule, ConfigurationRewardControl},
    owned_recipe_extension::OwnedRecipeExtension,
    owned_reward_policy::{RewardNumericSelection, RewardRuleInput, RewardRuleOutput},
    owned_value_policy::{DuplicatePolicy, MissingValuePolicy, ValueLane, ValueRecipe},
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::PathBuf};

const FAMILY: &str = "data/owned/poe2/3887ae68/configuration-campaign-progression";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&fs::read(root().join(FAMILY).join(name)).unwrap()).unwrap()
}
fn packet<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_value(read(name)).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn authenticated(pin: &Value, maximum: u64) -> Vec<u8> {
    let path = root().join(pin["path"].as_str().unwrap());
    assert!(fs::metadata(&path).unwrap().len() <= maximum);
    let raw = fs::read(path).unwrap();
    assert_eq!(raw.len(), pin["bytes"]);
    assert_eq!(hash(&raw), pin["sha256"]);
    raw
}
#[test]
fn source_review_pins_full_bodies_spans_and_all_five_saved_counts() {
    let evidence = read("source-evidence.json");
    assert_eq!(evidence["scope"]["new_source_execution"], false);
    assert_eq!(evidence["scope"]["gameplay_maximum_proven"], false);
    assert_eq!(
        evidence["scope"]["captured_sample_counts_are_defaults"],
        false
    );
    let manifest: Value =
        serde_json::from_slice(&authenticated(&evidence["manifest"], 256 * 1024)).unwrap();
    assert_eq!(
        evidence["manifest"]["sha256"],
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    assert_eq!(evidence["source_revision"], manifest["upstream_revision"]);
    authenticated(&evidence["license"], 128 * 1024);
    let mut bodies = BTreeMap::new();
    let expected_paths = [
        "src/Modules/ConfigOptions.lua",
        "src/Classes/ConfigTab.lua",
        "src/Classes/ImportTab.lua",
    ];
    let sources = evidence["bodies"].as_array().unwrap();
    assert_eq!(sources.len(), expected_paths.len());
    for (body, expected) in sources.iter().zip(expected_paths) {
        assert_eq!(body["source_path"], expected);
        let archived = authenticated(&body["archive"], 512 * 1024);
        let mut raw = vec![];
        GzDecoder::new(archived.as_slice())
            .take(1024 * 1024 + 1)
            .read_to_end(&mut raw)
            .unwrap();
        assert!(raw.len() <= 1024 * 1024);
        assert_eq!(raw.len(), body["bytes"]);
        assert_eq!(hash(&raw), body["sha256"]);
        let pin = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["path"] == expected)
            .unwrap();
        assert_eq!(body["sha256"], pin["sha256"]);
        bodies.insert(expected, String::from_utf8(raw).unwrap());
    }
    let spans = evidence["spans"].as_array().unwrap();
    assert_eq!(spans.len(), 5);
    for span in spans {
        let source = &bodies[span["source_path"].as_str().unwrap()];
        let first = span["first"].as_u64().unwrap() as usize;
        let last = span["last"].as_u64().unwrap() as usize;
        let body = source
            .lines()
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(body, span["body"]);
        assert_eq!(hash(body.as_bytes()), span["sha256"]);
    }
    let control = spans[0]["body"].as_str().unwrap();
    assert!(control.contains("unique bosses defeated in the campaign"));
    assert!(control.contains("Rare monster encounters with the skull marker do not count"));
    assert!(control.contains("Multiplier:BossFaceBroken"));
    assert!(!control.contains("defaultState") && !control.contains("defaultPlaceholderState"));
    let precedence = spans[2]["body"].as_str().unwrap();
    assert!(precedence.contains("input[varData.var] ~= 0"));
    assert!(precedence.contains("placeholder[varData.var] ~= 0"));
    let originals = evidence["originals"].as_array().unwrap();
    assert_eq!(originals.len(), 5);
    for (original, expected) in originals.iter().zip([55, 57, 57, 56, 56]) {
        let xml = String::from_utf8(authenticated(&original["file"], 4 * 1024 * 1024)).unwrap();
        let lines: Vec<_> = xml
            .lines()
            .filter(|line| line.contains("name=\"configBossFaceBroken\""))
            .collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], original["line"]);
        assert!(lines[0].contains("<Placeholder "));
        assert!(lines[0].contains(&format!("number=\"{expected}\"")));
        assert_eq!(original["value"], expected);
    }
}
#[test]
fn typed_packet_has_one_required_integer_parameter_and_no_missing_default() {
    let extension: OwnedRecipeExtension = packet("extension.json");
    assert_eq!(extension.schema.len(), 4);
    assert!(extension.owners.is_empty() && extension.receivers.is_empty());
    let _: MappingEntry = packet("mapping.json");
    let defs: Vec<DefinitionDescriptor> = packet("definitions.json");
    let slots: Vec<SlotDescriptor> = packet("slots.json");
    assert_eq!(defs.len(), 3);
    assert_eq!(slots.len(), 1);
    let SlotDescriptor::Parameter(slot) = &slots[0] else {
        panic!()
    };
    let SchemaState::Known(schema) = &slot.schema else {
        panic!()
    };
    assert_eq!(schema.presence, SlotPresence::RequiredOnce);
    assert_eq!(schema.sites, vec![ParameterSite::RewardParameter]);
    assert_eq!(
        schema.value,
        ValueSchema::Integer(IntegerRange {
            minimum: BoundedInteger::new(0).unwrap(),
            maximum: BoundedInteger::new(BoundedInteger::MAX).unwrap()
        })
    );
    let rule: RewardRuleInput = packet("reward.json");
    assert_eq!(rule.recipe.missing, MissingValuePolicy::Pending);
    ValueRecipe::new(rule.recipe.clone(), Default::default()).unwrap();
    let RewardRuleOutput::NumericParameter(numeric) = &rule.output else {
        panic!()
    };
    assert_eq!(&numeric.parameter, &slot.id);
    assert!(numeric.fixed_parameters.is_empty());
    assert_eq!(
        numeric.selection,
        RewardNumericSelection::NonzeroInputThenNonzeroPlaceholder
    );
    assert_eq!(rule.recipe.tiers.len(), 2);
    for (tier, lane) in rule
        .recipe
        .tiers
        .iter()
        .zip([ValueLane::InputNumber, ValueLane::PlaceholderNumber])
    {
        assert_eq!(tier.duplicates, DuplicatePolicy::Reject);
        assert_eq!(tier.selectors.len(), 1);
        assert_eq!(tier.selectors[0].lane, lane);
        assert_eq!(tier.selectors[0].name, "configBossFaceBroken");
    }
    let inventory: Vec<ConfigurationRewardControl> = packet("inventory.json");
    let accounting: Vec<ConfigurationDispositionRule> = packet("accounting.json");
    assert_eq!(inventory.len(), 2);
    assert_eq!(accounting.len(), 2);
    for (control, tier) in inventory.iter().zip(&rule.recipe.tiers) {
        assert_eq!(control.recipe, rule.recipe.id);
        assert_eq!(control.selector, tier.selectors[0]);
    }
    let consumer = read("consumer.json");
    let owners: Vec<DefinitionRules> = serde_json::from_value(consumer["owners"].clone()).unwrap();
    let queries: Vec<ContributionQuery> =
        serde_json::from_value(consumer["queries"].clone()).unwrap();
    assert_eq!(owners.len(), 1);
    assert!(owners[0].programs.is_complete());
    assert_eq!(owners[0].programs.members.len(), 2);
    assert_eq!(queries.len(), 2);
    assert_eq!(
        queries[0].groups[0].reduction,
        ContributionReduction::RequireAgreement
    );
    assert!(queries[0].groups[0].empty.is_none());
    assert_eq!(
        queries[1].groups[0].reduction,
        ContributionReduction::ExactIntegerSum
    );
    for query in queries {
        assert!(query.groups[0].members.is_complete());
        assert_eq!(query.groups[0].members.members.len(), 1);
        let ContributionProducer::ProgramEffect(producer) =
            &query.groups[0].members.members[0].producer
        else {
            panic!()
        };
        assert_eq!(producer.owner, owners[0].owner);
        assert_eq!(producer.origin, ContributionOrigin::Reward);
    }
}
#[test]
fn authoring_manifest_authenticates_only_the_bounded_progression_packet() {
    let authoring = read("authoring.json");
    assert_eq!(
        authoring["before"],
        "5b611f43a0eedbf556799bcbd2c295a85d58d3264a7306c24602fd4d41e6e7b6"
    );
    assert_eq!(authoring["scope"]["complete_build"], false);
    assert_eq!(
        authoring["scope"]["complete_source_supplier_coverage"],
        false
    );
    assert_eq!(authoring["scope"]["new_source_execution"], false);
    for pin in authoring["artifacts"].as_array().unwrap() {
        authenticated(pin, 512 * 1024);
    }
}
