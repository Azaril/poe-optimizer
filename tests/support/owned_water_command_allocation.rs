//! Exact declaration correction; this packet grants no activation or numerical coverage.
use flate2::read::GzDecoder;
use poe_optimizer_core::owned_schema::*;
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::StagedOwnedRelease,
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf};

pub const BEFORE: &str = "c8705bf69798b7215053a95462ffb8e87dbc9ad36f4f52ed0df7be946609c300";
const AUTHORING: &str = "33536f7801386378a54f0b51bb36149fc7e5483353edd7326f81bdd8e64ec967";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/water-command-allocation")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn pinned(path: PathBuf, pin: &Value) -> Vec<u8> {
    let bytes = fs::read(path).unwrap();
    assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
    assert_eq!(digest(&bytes), pin["sha256"]);
    bytes
}
pub fn dependencies() -> Value {
    read("dependencies.json")
}
pub fn prior_grant() -> SlotDescriptor {
    let deps = dependencies();
    serde_json::from_value(
        deps["slots"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["value"]["id"]["slot"]["key"] == "def.00000000000032bb")
            .unwrap()
            .clone(),
    )
    .unwrap()
}
pub fn check_delta(m: &OwnedReleaseMigrationInput) {
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.before.to_string(), BEFORE);
    assert_eq!(m.contract.schema_version, 7);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v28"
    );
    assert!(
        m.tables.is_empty()
            && m.owners.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    let [SchemaExtensionEntry::Slot(next)] = m.schema.as_slice() else {
        panic!("one exact slot replacement")
    };
    let prior = prior_grant();
    let SlotDescriptor::Grant(old) = &prior else {
        panic!("grant")
    };
    let SchemaState::Known(old) = &old.schema else {
        panic!("known grant")
    };
    assert_eq!(old.provider_roles, vec![ProviderRole::SkillUse]);
    let mut expected = prior.clone();
    let SlotDescriptor::Grant(row) = &mut expected else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut row.schema else {
        unreachable!()
    };
    schema.provider_roles.push(ProviderRole::Allocation);
    assert_eq!(
        next, &expected,
        "target, owner, ID and every other field remain exact"
    );
    let mut restored = expected;
    let SlotDescriptor::Grant(row) = &mut restored else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut row.schema else {
        unreachable!()
    };
    assert_eq!(schema.provider_roles.pop(), Some(ProviderRole::Allocation));
    assert_eq!(restored, prior);
}
pub fn check_authored() {
    let raw = fs::read(data().join("authoring.json")).unwrap();
    assert_eq!(raw.len(), 4097);
    assert_eq!(digest(&raw), AUTHORING);
    let a: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(a["predecessor"]["input"], BEFORE);
    for pin in a["artifacts"].as_array().unwrap() {
        pinned(data().join(pin["name"].as_str().unwrap()), pin);
    }
    let manifest: Value = serde_json::from_slice(&pinned(
        root().join(a["source_manifest"]["path"].as_str().unwrap()),
        &a["source_manifest"],
    ))
    .unwrap();
    assert_eq!(manifest["upstream_revision"], a["source_revision"]);
    let mut bodies = Vec::new();
    for source in a["sources"].as_array().unwrap() {
        let archived = pinned(
            root().join(source["archive"]["path"].as_str().unwrap()),
            &source["archive"],
        );
        let mut body = Vec::new();
        GzDecoder::new(archived.as_slice())
            .take(16 * 1024 * 1024 + 1)
            .read_to_end(&mut body)
            .unwrap();
        assert!(body.len() <= 16 * 1024 * 1024);
        assert_eq!(body.len() as u64, source["body"]["bytes"].as_u64().unwrap());
        assert_eq!(digest(&body), source["body"]["sha256"]);
        assert_eq!(manifest["files"].as_array().unwrap().iter().filter(|p| p["path"] == source["source"] && p["sha256"] == source["body"]["sha256"]).count(), 1);
        let text = String::from_utf8(body).unwrap();
        for witness in source["witnesses"].as_array().unwrap() {
            assert!(text.contains(witness.as_str().unwrap()));
        }
        bodies.push(text);
    }
    assert_eq!(bodies.len(), 3);
    let node = bodies[0]
        .split("\t\t[32705]={")
        .nth(1)
        .unwrap()
        .split("\n\t\t},")
        .next()
        .unwrap();
    assert!(node.contains("skill=32705") && node.contains("Grants Skill: Navira, the Last Mirage"));
    let gem = bodies[1]
        .split("[\"Metadata/Items/Gems/SkillGemAscendancySummonWaterDjinn\"] = {")
        .nth(1)
        .unwrap()
        .split("\n\t},")
        .next()
        .unwrap();
    for witness in [
        "name = \"Navira, the Last Mirage\"",
        "grantedEffectId = \"SummonWaterDjinnPlayer\"",
        "additionalGrantedEffectId1 = \"CommandWaterDjinnBubblePlayer\"",
    ] {
        assert!(gem.contains(witness));
    }
    let original = pinned(
        root().join(a["original"]["path"].as_str().unwrap()),
        &a["original"],
    );
    let original = String::from_utf8(original).unwrap();
    assert!(
        original.contains("source=\"Tree:32705\"")
            && original.contains("skillId=\"SummonWaterDjinnPlayer\"")
    );
    let prior: Vec<Value> = a["prior_packets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            serde_json::from_slice(&pinned(root().join(p["path"].as_str().unwrap()), p)).unwrap()
        })
        .collect();
    let tree = prior[0]["families"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["node_id"] == 32705)
        .unwrap();
    let action = prior[1]["families"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["skill_id"] == "SummonWaterDjinnPlayer")
        .unwrap();
    assert_eq!(tree["skill"], action["skill"]);
    assert_eq!(tree["passive"]["key"], "def.00000000000010d4");
    assert_eq!(tree["skill"]["key"], "def.000000000000032c");
    assert_eq!(
        action["command"]["skill_id"],
        "CommandWaterDjinnBubblePlayer"
    );
    assert_eq!(action["command"]["skill"]["key"], "def.00000000000000d9");
    assert_eq!(
        action["command"]["entering_grant"]["slot"]["key"],
        "def.00000000000032bb"
    );
    let deps = dependencies();
    let slots = deps["slots"].as_array().unwrap();
    let slot = |k: &str| {
        slots
            .iter()
            .find(|s| s["value"]["id"]["slot"]["key"] == k)
            .unwrap()
    };
    for (grant, supply, skill) in [
        (
            "def.00000000000032d5",
            "def.00000000000032d4",
            "def.000000000000032c",
        ),
        (
            "def.00000000000032bb",
            "def.00000000000032ba",
            "def.00000000000000d9",
        ),
    ] {
        assert_eq!(
            slot(grant)["value"]["schema"]["value"]["target"]["value"],
            slot(supply)["value"]["id"]
        );
        assert_eq!(
            slot(supply)["value"]["schema"]["value"]["skill"]["key"],
            skill
        );
        assert_eq!(
            slot(supply)["value"]["schema"]["value"]["outputs"]["closure"]["kind"],
            "partial"
        );
    }
    assert_eq!(
        slot("def.00000000000032a6")["value"]["schema"]["value"]["provider_roles"],
        json!(["skill_use", "allocation"])
    );
    assert_eq!(
        slot("def.00000000000032bc")["value"]["schema"]["value"]["choices"]["closure"]["kind"],
        "partial"
    );
    check_delta(&read("migration.json"));
}
pub fn checked_migration() -> OwnedReleaseMigrationInput {
    check_authored();
    read("migration.json")
}
/// Existing migration compiler only; exact predecessor/descriptor guards precede it.
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    let request = checked_migration();
    assert_eq!(prior.receipt().input, request.before);
    let deps = dependencies();
    for d in
        serde_json::from_value::<Vec<DefinitionDescriptor>>(deps["definitions"].clone()).unwrap()
    {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| **x == d)
                .count(),
            1
        );
    }
    for s in serde_json::from_value::<Vec<SlotDescriptor>>(deps["slots"].clone()).unwrap() {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .slots
                .iter()
                .filter(|x| **x == s)
                .count(),
            1
        );
    }
    let next = compile_owned_release_migration(prior, request, Default::default()).unwrap();
    assert_eq!(next.input().recipe.registry, prior.input().recipe.registry);
    assert_eq!(
        next.input().recipe.rules.owners,
        prior.input().recipe.rules.owners
    );
    assert_eq!(
        next.input().recipe.rules.tables,
        prior.input().recipe.rules.tables
    );
    assert_eq!(
        next.input().recipe.rules.receivers,
        prior.input().recipe.rules.receivers
    );
    assert_eq!(next.input().query_sets, prior.input().query_sets);
    let mut restored = next.input().recipe.schema.clone();
    restored.release = prior.input().recipe.schema.release.clone();
    *restored
        .slots
        .iter_mut()
        .find(|s| s.address() == prior_grant().address())
        .unwrap() = prior_grant();
    assert_eq!(restored, prior.input().recipe.schema);
    next
}
