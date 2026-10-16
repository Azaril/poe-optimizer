//! Authored item-local Evasion data. Source receipts never enter runtime inputs.
use poe_optimizer_core::{owned_definitions::*, owned_rules::*, owned_schema::*};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf, sync::OnceLock};
pub const LOCAL: u64 = 0x33a5;
pub const FINAL: u64 = 0x33a6;
pub const PER_LEVEL: u64 = 0x33a7;
pub const REPLACEMENT: u64 = 0x33a8;
pub const UNIT: u64 = 0x29ee;
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/item-evasion-property")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    assert!(fs::metadata(data().join(name)).unwrap().len() <= 4 * 1024 * 1024);
    let bytes = fs::read(data().join(name)).unwrap();
    assert!(bytes.len() <= 4 * 1024 * 1024);
    serde_json::from_slice(&bytes).unwrap()
}
pub fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
pub fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(
        GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap(),
        format!("def.{n:016x}"),
    )
    .unwrap()
}
pub fn subject<I: SchemaDefinitionId>(id: I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
#[derive(Clone, Deserialize)]
pub struct Delivery {
    pub owner: SchemaSubject,
    pub program: RuleProgram,
}
#[derive(Clone, Deserialize)]
pub struct Consumer {
    pub definitions: Vec<DefinitionDescriptor>,
    pub owners: Vec<DefinitionRules>,
    pub receivers: Vec<StatReceiver>,
    pub delivery: Vec<Delivery>,
    pub queries: Vec<ContributionQuery>,
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn unpack(path: PathBuf, limit: usize) -> Vec<u8> {
    assert!(fs::metadata(&path).unwrap().len() <= limit as u64);
    let bytes = fs::read(path).unwrap();
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(bytes.as_slice())
        .take(limit as u64 + 1)
        .read_to_end(&mut out)
        .unwrap();
    assert!(out.len() <= limit);
    out
}
/// Offline admission review only. Numeric family identity does not establish
/// local applicability; all source tags/flags and exact owner must be accounted.
pub fn review_local_record(record: &Value, exact_source: &str) -> Result<(), String> {
    if record["source"] != exact_source
        || record["flags"] != 0
        || record["keyword_flags"] != 0
        || record["tags"] != serde_json::json!({})
    {
        return Err("local modifier source/flags/tags are not in the proven domain".into());
    }
    let admitted = matches!(
        (record["name"].as_str(), record["type"].as_str()),
        (
            Some("Evasion" | "ArmourAndEvasion" | "EvasionAndEnergyShield"),
            Some("BASE" | "INC")
        ) | (Some("Defences"), Some("INC"))
    );
    if !admitted || !record["value"].as_f64().is_some_and(f64::is_finite) {
        return Err("unadmitted local modifier recipe".into());
    }
    Ok(())
}
pub fn source() -> &'static Value {
    static SOURCE: OnceLock<Value> = OnceLock::new();
    SOURCE.get_or_init(|| {
        let bytes = unpack(data().join("source-report.json.gz"), 10 * 1024 * 1024);
        assert_eq!(bytes.len(), 9_528_071);
        assert_eq!(
            hash(&bytes),
            "a4e1c0ca6baad3905a279592ad6800c3d51b7c9fdf7f38f55933e4b464eac62d"
        );
        serde_json::from_slice(&bytes).unwrap()
    })
}
pub fn check() {
    static CHECK: OnceLock<()> = OnceLock::new();
    CHECK.get_or_init(|| {
        let a: Value = read("authoring.json");
        for pin in a["artifacts"].as_array().unwrap() {
            let path = data().join(pin["name"].as_str().unwrap());
            assert!(fs::metadata(&path).unwrap().len() <= 4 * 1024 * 1024);
            let bytes = fs::read(path).unwrap();
            assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
            assert_eq!(hash(&bytes), pin["sha256"]);
        }
        let c = consumer();
        assert_eq!(c.definitions.len(), 4);
        for (n, descriptor) in (LOCAL..=REPLACEMENT).zip(&c.definitions) {
            assert_eq!(descriptor.address(), def::<StatDefinition>(n).address());
            let DefinitionDescriptor::Stat(row) = descriptor else {
                panic!("stat")
            };
            let SchemaState::Known(schema) = &row.schema else {
                panic!("known")
            };
            assert_eq!(schema.targets, vec![RuleEntityKind::EquipmentUse]);
            assert_eq!(
                schema.value,
                ComputedValueType::Quantity { unit: def(UNIT) }
            );
        }
        assert_eq!(
            (
                c.owners.len(),
                c.receivers.len(),
                c.delivery.len(),
                c.queries.len()
            ),
            (2, 2, 7, 11)
        );
        let source = source();
        assert_eq!(
            source["source_revision"],
            "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
        );
        assert_eq!(source["native_owner_closure"], false);
        let vectors: Value = read("source-vectors.json");
        let rows = vectors["observations"].as_array().unwrap();
        assert_eq!(rows.len(), 3);
        for (row, stage) in rows.iter().zip(["fresh", "rebuild-one", "rebuild-two"]) {
            assert_eq!(row["stage"], stage);
            let state = source.pointer(row["pointer"].as_str().unwrap()).unwrap();
            for (name, expected) in row["value"].as_object().unwrap() {
                let actual = state.get(name).unwrap_or(&state["trace"][name]);
                assert!(
                    projection_matches(expected, actual),
                    "source projection differs at {stage}/{name}"
                );
            }
            for item in row["value"]["items"].as_array().unwrap() {
                assert_eq!(item["armour"]["Evasion"], 75);
                assert_eq!(item["armour"]["EvasionPerLevel"], 0);
            }
        }
        let evidence: Value = read("source-evidence.json");
        assert_eq!(evidence["complete_bodies"].as_array().unwrap().len(), 2);
        for body in evidence["complete_bodies"].as_array().unwrap() {
            let original = source["files"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["path"] == body["source_path"])
                .unwrap();
            assert_eq!(
                original["sha256"], body["sha256"],
                "body bound to immutable observed source manifest"
            );
            let archive = &body["archive"];
            let compressed = fs::read(root().join(archive["path"].as_str().unwrap())).unwrap();
            assert_eq!(hash(&compressed), archive["sha256"]);
            let bytes = unpack(root().join(archive["path"].as_str().unwrap()), 1024 * 1024);
            assert_eq!(hash(&bytes), body["sha256"]);
            assert_eq!(bytes.len() as u64, body["bytes"].as_u64().unwrap());
        }
    });
}

pub fn writer_census(rules: &Value) -> Value {
    let c = consumer();
    let mut targets: std::collections::BTreeSet<_> =
        c.queries.iter().map(|q| q.stat.key().to_string()).collect();
    targets
        .extend([LOCAL, FINAL, 0x29fe, 0x2427].map(|n| def::<StatDefinition>(n).key().to_string()));
    let mut rows = Vec::new();
    for o in rules["owners"].as_array().unwrap() {
        for p in o["programs"]["members"].as_array().unwrap() {
            for e in p["effects"].as_array().unwrap() {
                if e["effect"]["stat"]["key"]
                    .as_str()
                    .is_some_and(|s| targets.contains(s))
                {
                    rows.push(serde_json::json!({"owner":o["owner"],"program":p["id"],"effect":e}));
                }
            }
        }
    }
    for a in rules["effect_applications"]["members"].as_array().unwrap() {
        for e in a["program"]["effects"].as_array().unwrap() {
            if e["effect"]["stat"]["key"]
                .as_str()
                .is_some_and(|s| targets.contains(s))
            {
                rows.push(serde_json::json!({"application":a["id"],"effect":e}));
            }
        }
    }
    serde_json::json!(rows)
}
pub fn check_native(schema: &Value, rules: &Value, published: bool) {
    use serde_json::json;
    let d: Value = read("dependencies.json");
    let c = consumer();
    let mut r = rules.clone();
    if published {
        for delivery in &c.delivery {
            let owner = r["owners"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|o| o["owner"] == json!(delivery.owner))
                .unwrap();
            let actual: RuleProgram = serde_json::from_value(
                owner["programs"]["members"]
                    .as_array_mut()
                    .unwrap()
                    .pop()
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(actual, delivery.program);
        }
        for owner in &c.owners {
            let index = r["owners"]
                .as_array()
                .unwrap()
                .iter()
                .position(|o| o["owner"] == json!(owner.owner))
                .unwrap();
            let actual: DefinitionRules =
                serde_json::from_value(r["owners"].as_array_mut().unwrap().remove(index)).unwrap();
            assert_eq!(&actual, owner);
        }
        for query in &c.queries {
            let rows = r["contribution_queries"]["members"].as_array_mut().unwrap();
            let indices: Vec<_> = rows
                .iter()
                .enumerate()
                .filter(|(_, q)| q["id"] == query.id.as_str())
                .map(|(n, _)| n)
                .collect();
            assert_eq!(indices.len(), 1);
            let actual: ContributionQuery =
                serde_json::from_value(rows.remove(indices[0])).unwrap();
            assert_eq!(&actual, query);
        }
        for receiver in &c.receivers {
            let rows = r["receivers"]["members"].as_array_mut().unwrap();
            let index = rows
                .iter()
                .position(|x| x["id"] == receiver.id.as_str())
                .unwrap();
            let actual: StatReceiver = serde_json::from_value(rows.remove(index)).unwrap();
            assert_eq!(&actual, receiver);
        }
        for def in &c.definitions {
            let actual = schema["definitions"]
                .as_array()
                .unwrap()
                .iter()
                .find(|x| x["value"]["id"] == json!(def.address())["value"])
                .unwrap();
            let actual: DefinitionDescriptor = serde_json::from_value(actual.clone()).unwrap();
            assert_eq!(&actual, def);
        }
    }
    assert!(
        writer_census(&r) == d["potential_writers"],
        "actual complete potential-writer census changed"
    );
    assert_eq!(
        r["contribution_queries"]["closure"],
        d["query_registry_closure"]
    );
    assert_eq!(r["receivers"]["closure"], d["receiver_registry_closure"]);
    for expected in d["owners"].as_array().unwrap() {
        let actual: DefinitionRules = serde_json::from_value(
            r["owners"]
                .as_array()
                .unwrap()
                .iter()
                .find(|o| o["owner"] == expected["owner"])
                .unwrap()
                .clone(),
        )
        .unwrap();
        let expected: DefinitionRules = serde_json::from_value(expected.clone()).unwrap();
        assert_eq!(actual, expected, "exact relevant owner/coverage");
    }
    for expected in d["definitions"].as_array().unwrap() {
        let actual: DefinitionDescriptor = serde_json::from_value(
            schema["definitions"]
                .as_array()
                .unwrap()
                .iter()
                .find(|x| x["value"]["id"] == expected["value"]["id"])
                .unwrap()
                .clone(),
        )
        .unwrap();
        let expected: DefinitionDescriptor = serde_json::from_value(expected.clone()).unwrap();
        assert_eq!(actual, expected);
    }
}
pub fn assert_component(endpoint: &poe_optimizer_import::owned_release::StagedOwnedRelease) {
    check();
    let recipe = &endpoint.input().recipe;
    check_native(
        &serde_json::json!(recipe.schema),
        &serde_json::json!(recipe.rules),
        true,
    );
    assert!(endpoint.evaluation().is_none());
}
pub fn stage(
    prior: &poe_optimizer_import::owned_release::StagedOwnedRelease,
) -> poe_optimizer_import::owned_release::StagedOwnedRelease {
    use poe_optimizer_import::{
        owned_release::{OwnedReleaseProvenance, assemble_owned_release},
        owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
    };
    check();
    let a: Value = read("authoring.json");
    assert_eq!(a["scope"]["publication_ready"], true);
    assert_eq!(a["before"], serde_json::json!(prior.receipt().input));
    assert_eq!(prior.input().recipe.registry.last_issued.get(), 0x33a4);
    check_native(
        &serde_json::json!(prior.input().recipe.schema),
        &serde_json::json!(prior.input().recipe.rules),
        false,
    );
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    let migrated = compile_owned_release_migration(prior, migration, Default::default()).unwrap();
    let mut input = migrated.input().clone();
    let c = consumer();
    for d in &c.delivery {
        input
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == d.owner)
            .unwrap()
            .programs
            .members
            .push(d.program.clone());
    }
    input.recipe.rules.owners.extend(c.owners.clone());
    input
        .recipe
        .rules
        .receivers
        .members
        .extend(c.receivers.clone());
    input
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(c.queries.clone());
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: key("item-evasion-property"),
        prior_input: prior.receipt().input,
        authoring_input: poe_optimizer_core::owned_content::digest_owned(
            "item-evasion-property",
            &a,
            1024 * 1024,
        )
        .unwrap(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    for d in &c.delivery {
        assert_eq!(
            inverse
                .recipe
                .rules
                .owners
                .iter_mut()
                .find(|o| o.owner == d.owner)
                .unwrap()
                .programs
                .members
                .pop(),
            Some(d.program.clone())
        );
    }
    for o in c.owners.iter().rev() {
        assert_eq!(inverse.recipe.rules.owners.pop(), Some(o.clone()));
    }
    for q in c.queries.iter().rev() {
        assert_eq!(
            inverse
                .recipe
                .rules
                .contribution_queries
                .as_mut()
                .unwrap()
                .members
                .pop(),
            Some(q.clone())
        );
    }
    for r in &c.receivers {
        let n = inverse
            .recipe
            .rules
            .receivers
            .members
            .iter()
            .position(|v| v.id == r.id)
            .unwrap();
        assert_eq!(inverse.recipe.rules.receivers.members.remove(n), *r);
    }
    inverse.provenance = migrated.input().provenance.clone();
    assert!(
        inverse == *migrated.input(),
        "exact component inverse preserves every prior rule/data/input"
    );
    super::migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    next
}
fn projection_matches(expected: &Value, actual: &Value) -> bool {
    match expected {
        Value::Object(o) => actual.as_object().is_some_and(|a| {
            o.iter()
                .all(|(k, v)| a.get(k).is_some_and(|x| projection_matches(v, x)))
        }),
        Value::Array(a) => actual.as_array().is_some_and(|b| {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| projection_matches(x, y))
        }),
        _ => expected == actual,
    }
}
