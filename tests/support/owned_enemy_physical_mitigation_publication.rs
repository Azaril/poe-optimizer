//! Publish only the Partial armour response, preserving every existing input.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_rules::{ContributionQuery, DefinitionRules, RulePackageInput, RuleProgram},
    owned_schema::DefinitionDescriptor,
};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf, sync::OnceLock};

const FAMILY: &str = "data/owned/poe2/3887ae68/enemy-physical-mitigation";
const FILES: [&str; 7] = [
    "definitions.json",
    "consumer.json",
    "bindings.json",
    "dependencies.json",
    "migration.json",
    "source-evidence.json",
    "source-vectors.json",
];
const PROGRAM: &str = "ordinary-armour-adjusted-physical-averages";
const QUERIES: [&str; 4] = [
    "enemy-armour-add",
    "enemy-armour-increase",
    "enemy-armour-multiply",
    "enemy-armour-override",
];

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FAMILY)
}
pub fn read(name: &str) -> Value {
    let path = data().join(name);
    assert!(fs::metadata(&path).unwrap().len() <= 2 * 1024 * 1024);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn ensure(ok: bool, message: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(message.into()) }
}
fn owner(value: &Value) -> Result<DefinitionRules, String> {
    serde_json::from_value(value.clone()).map_err(|e| format!("typed owner: {e}"))
}
fn program(value: &Value) -> Result<RuleProgram, String> {
    serde_json::from_value(value.clone()).map_err(|e| format!("typed program: {e}"))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Consumer {
    owners: Vec<DefinitionRules>,
    queries: Vec<ContributionQuery>,
}
fn consumer() -> Consumer {
    serde_json::from_value(read("consumer.json")).unwrap()
}

pub fn review_authoring(a: &Value) -> Result<(), String> {
    let d = read("dependencies.json");
    let m = read("migration.json");
    let before = a["before"].as_str().ok_or("actual predecessor identity")?;
    ensure(
        a["schema_version"] == 1
            && before.len() == 64
            && before.bytes().all(|b| b.is_ascii_hexdigit())
            && a["before"] == d["before"]
            && a["before"] == d["inspection_endpoint"]
            && a["before"] == m["before"]
            && a["release"] == m["release"],
        "exact publication predecessor and release",
    )?;
    ensure(
        a["scope"]["publication_ready"] == true,
        "publication readiness",
    )?;
    for field in [
        "complete_request",
        "complete_mitigation",
        "candidate_source_closure",
        "query_closure_changed",
        "existing_owner_closure_changed",
        "new_source_execution",
        "ready_for_native_admission",
    ] {
        ensure(a["scope"][field] == false, "Partial publication scope")?;
    }
    let rows = a["artifacts"].as_array().ok_or("artifact inventory")?;
    ensure(rows.len() == FILES.len(), "exact artifact census")?;
    for (pin, name) in rows.iter().zip(FILES) {
        ensure(
            pin["path"] == format!("{FAMILY}/{name}"),
            "exact artifact path/order",
        )?;
        let path = data().join(name);
        let size = fs::metadata(&path).map_err(|e| e.to_string())?.len();
        ensure(
            size <= 2 * 1024 * 1024 && pin["bytes"] == size,
            "bounded exact artifact bytes",
        )?;
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        ensure(
            pin["sha256"] == format!("{:x}", Sha256::digest(&bytes)),
            "exact artifact hash",
        )?;
    }
    Ok(())
}

/// Inventory potential writes without treating false/zero as source absence.
fn writers(rules: &Value, keys: &BTreeSet<&str>) -> Result<Value, String> {
    let writes = |p: &Value| -> Result<bool, String> {
        Ok(p["effects"]
            .as_array()
            .ok_or("program effects")?
            .iter()
            .any(|e| {
                e["effect"]["stat"]["key"]
                    .as_str()
                    .is_some_and(|k| keys.contains(k))
            }))
    };
    let mut rows = vec![];
    for o in rules["owners"].as_array().ok_or("owner inventory")? {
        for p in o["programs"]["members"]
            .as_array()
            .ok_or("program inventory")?
        {
            if writes(p)? {
                rows.push(json!({"kind":"owner","owner":o["owner"],"program":program(p)?}));
            }
        }
    }
    for a in rules["effect_applications"]["members"]
        .as_array()
        .ok_or("application inventory")?
    {
        if writes(&a["program"])? {
            rows.push(json!({"kind":"application","application":a}));
        }
    }
    Ok(json!(rows))
}

pub fn review_predecessor(schema: &Value, rules: &Value, routing: &Value) -> Result<(), String> {
    let d = read("dependencies.json");
    ensure(
        d["before"] == read("authoring.json")["before"] && d["registry_last"] == 0x33b1,
        "actual configured-Weakness predecessor",
    )?;
    let expected_owners = d["owners_before"]
        .as_array()
        .ok_or("prior owner inventory")?;
    ensure(expected_owners.len() == 1, "one existing consumer owner")?;
    for expected in expected_owners {
        let actual: Vec<_> = rules["owners"]
            .as_array()
            .ok_or("owner inventory")?
            .iter()
            .filter(|o| o["owner"] == expected["owner"])
            .collect();
        ensure(
            actual.len() == 1 && owner(actual[0])? == owner(expected)?,
            "exact prior owner and closure",
        )?;
    }
    let definitions = schema["definitions"]
        .as_array()
        .ok_or("definition inventory")?;
    for expected in d["definitions"]
        .as_array()
        .ok_or("dependency definitions")?
    {
        let actual: Vec<_> = definitions
            .iter()
            .filter(|v| v["value"]["id"] == expected["value"]["id"])
            .collect();
        ensure(actual.len() == 1, "dependency descriptor multiplicity")?;
        let actual: DefinitionDescriptor =
            serde_json::from_value(actual[0].clone()).map_err(|e| e.to_string())?;
        let expected: DefinitionDescriptor =
            serde_json::from_value(expected.clone()).map_err(|e| e.to_string())?;
        ensure(actual == expected, "exact dependency descriptor")?;
    }
    let expected_keys: Vec<_> = [0x31e0, 0x33a0, 0x33a1, 0x33ab, 0x33b2, 0x33b3]
        .into_iter()
        .map(|n| format!("def.{n:016x}"))
        .collect();
    ensure(
        d["potential_writer_keys"] == json!(expected_keys),
        "exact potential writer channels",
    )?;
    let keys = expected_keys.iter().map(String::as_str).collect();
    let mut expected = d["potential_writers"].clone();
    for row in expected.as_array_mut().ok_or("expected writer inventory")? {
        ensure(row["kind"] == "owner", "reviewed prior writer kind")?;
        row["program"] = json!(program(&row["program"])?);
    }
    ensure(
        writers(rules, &keys)? == expected,
        "all-owner/application potential writer census",
    )?;
    let routes = d["input_routes"]
        .as_array()
        .ok_or("input route commitments")?;
    ensure(routes.len() == 1, "one exact Action output route")?;
    for expected in routes {
        let actual: Vec<_> = routing["outputs"]
            .as_array()
            .ok_or("routing outputs")?
            .iter()
            .filter(|r| r["output"] == expected["output"])
            .collect();
        ensure(actual == vec![expected], "exact Action input routes")?;
    }
    Ok(())
}

pub fn check_authored() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let a = read("authoring.json");
        review_authoring(&a).unwrap();
        let m: OwnedReleaseMigrationInput = serde_json::from_value(read("migration.json")).unwrap();
        assert_eq!(m.schema_version, 5);
        assert_eq!(json!(m.before), a["before"]);
        assert_eq!(json!(m.release), a["release"]);
        assert_eq!(m.contract.schema_version, 7);
        assert_eq!(
            m.contract.operations_version.as_str(),
            "owned-domain-operations-v28"
        );
        let definitions = read("definitions.json");
        let definitions = definitions.as_array().unwrap();
        assert_eq!(definitions.len(), 2);
        let expected: Vec<_> = definitions
            .iter()
            .map(|d| json!({"kind":"definition","value":d}))
            .collect();
        assert_eq!(json!(m.schema), json!(expected));
        assert!(
            m.owners.is_empty()
                && m.tables.is_empty()
                && m.receivers.is_empty()
                && m.query_targets.is_empty()
                && m.evaluation.is_none()
        );
        let c = consumer();
        assert_eq!(c.owners.len(), 1);
        assert_eq!(c.owners[0].programs.members.len(), 1);
        assert_eq!(c.owners[0].programs.members[0].id.as_str(), PROGRAM);
        assert!(!c.owners[0].programs.is_complete());
        let d = read("dependencies.json");
        let before = owner(&d["owners_before"][0]).unwrap();
        assert_eq!(before.owner, c.owners[0].owner);
        assert!(!before.programs.is_complete());
        assert_eq!(c.queries.len(), QUERIES.len());
        for (q, id) in c.queries.iter().zip(QUERIES) {
            assert_eq!(q.id.as_str(), id);
            assert_eq!(q.groups.len(), 1);
            assert!(!q.groups[0].members.is_complete());
        }
        let b = read("bindings.json");
        assert_eq!(b["ready_for_native_admission"], false);
        assert_eq!(b["publication_scope"], "partial_component");
        assert_eq!(b["program"], PROGRAM);
        assert_eq!(b["inputs"], d["input_channels"]);
        assert_eq!(b["outputs"], d["output_channels"]);
        assert_eq!(b["armour_member"]["query"], QUERIES[0]);
        assert_eq!(
            b["armour_member"]["member"],
            json!(c.queries[0].groups[0].members.members[0])
        );
    });
}

fn remove_component(rules: &mut RulePackageInput) {
    let c = consumer();
    let target = rules
        .owners
        .iter_mut()
        .find(|o| o.owner == c.owners[0].owner)
        .unwrap();
    assert_eq!(
        target.programs.members.pop(),
        c.owners[0].programs.members.first().cloned()
    );
    for q in c.queries.iter().rev() {
        assert_eq!(
            rules.contribution_queries.as_mut().unwrap().members.pop(),
            Some(q.clone())
        );
    }
}

pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    assert_eq!(endpoint.input().recipe.registry.last_issued.get(), 0x33b3);
    let recipe = &endpoint.input().recipe;
    for definition in read("definitions.json").as_array().unwrap() {
        let definition: DefinitionDescriptor = serde_json::from_value(definition.clone()).unwrap();
        assert!(recipe.schema.definitions.contains(&definition));
    }
    let mut rules = recipe.rules.clone();
    remove_component(&mut rules);
    review_predecessor(&json!(recipe.schema), &json!(rules), &json!(recipe.routing)).unwrap();
    assert!(endpoint.evaluation().is_none());
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a = read("authoring.json");
    assert_eq!(json!(prior.receipt().input), a["before"]);
    assert_eq!(prior.input().recipe.registry.last_issued.get(), 0x33b1);
    let old_recipe = &prior.input().recipe;
    review_predecessor(
        &json!(old_recipe.schema),
        &json!(old_recipe.rules),
        &json!(old_recipe.routing),
    )
    .unwrap();
    let migration = serde_json::from_value(read("migration.json")).unwrap();
    let migrated = compile_owned_release_migration(prior, migration, Default::default()).unwrap();
    let mut schema_inverse = migrated.input().recipe.schema.clone();
    for expected in read("definitions.json").as_array().unwrap() {
        let expected: DefinitionDescriptor = serde_json::from_value(expected.clone()).unwrap();
        let matches: Vec<_> = schema_inverse
            .definitions
            .iter()
            .enumerate()
            .filter_map(|(n, d)| (d.address() == expected.address()).then_some(n))
            .collect();
        assert_eq!(matches.len(), 1, "exact new definition multiplicity");
        assert_eq!(schema_inverse.definitions.remove(matches[0]), expected);
    }
    schema_inverse.release = old_recipe.schema.release.clone();
    assert!(
        schema_inverse == old_recipe.schema,
        "exact two-definition append inverse"
    );
    let mut rules_inverse = migrated.input().recipe.rules.clone();
    rules_inverse.definitions = old_recipe.rules.definitions.clone();
    assert!(
        rules_inverse == old_recipe.rules,
        "schema-only migration preserves every rule and closure"
    );
    let mut registry_inverse = json!(migrated.input().recipe.registry);
    let old_registry = json!(old_recipe.registry);
    for sequence in (0x33b2..=0x33b3).rev() {
        let entry = registry_inverse["entries"]
            .as_array_mut()
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(entry["sequence"], sequence);
        assert_eq!(entry["state"], json!({"kind":"active"}));
        assert_eq!(entry["target"]["kind"], "definition");
        assert_eq!(entry["target"]["value"]["kind"], "stat");
        assert_eq!(
            entry["target"]["value"]["value"]["key"],
            format!("def.{sequence:016x}")
        );
    }
    assert_eq!(
        registry_inverse["revision"].as_u64().unwrap(),
        old_registry["revision"].as_u64().unwrap() + 2
    );
    registry_inverse["revision"] = old_registry["revision"].clone();
    registry_inverse["last_issued"] = old_registry["last_issued"].clone();
    assert!(
        registry_inverse == old_registry,
        "exact registry append inverse"
    );
    migration_preservation::assert_import_rebindings_only(prior, &migrated);
    let mut input = migrated.input().clone();
    let c = consumer();
    let target = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == c.owners[0].owner)
        .unwrap();
    for p in &c.owners[0].programs.members {
        assert!(!target.programs.members.iter().any(|v| v.id == p.id));
        target.programs.members.push(p.clone());
    }
    let queries = input.recipe.rules.contribution_queries.as_mut().unwrap();
    for q in &c.queries {
        assert!(!queries.members.iter().any(|v| v.id == q.id));
        queries.members.push(q.clone());
    }
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: "armour-response-component".parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned("armour-response-component", &a, 1024 * 1024).unwrap(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    remove_component(&mut inverse.recipe.rules);
    inverse.provenance = migrated.input().provenance.clone();
    assert!(
        inverse == *migrated.input(),
        "exact component inverse preserves all prior inputs and closures"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    next
}
