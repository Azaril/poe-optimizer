//! Checked publication of a Partial calculation component, never build admission.
use super::{family, migration_preservation, ordinary, rounding};
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_rules::{DefinitionRules, RuleProgram},
    owned_schema::DefinitionDescriptor,
};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, sync::OnceLock};

const BEFORE: &str = "1b2603af5c5a6a698159c717e398a98e478316c9b3069f5310d9ccf35010d511";
const FILES: [&str; 11] = [
    "definitions.json",
    "consumer.json",
    "baseline.json",
    "bindings.json",
    "dependencies.json",
    "migration.json",
    "source-evidence.json",
    "source-domain.json",
    "source-vectors.json",
    "ordinary-source-domain.json",
    "rounding-domain.json",
];
fn ensure(ok: bool, message: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(message.into()) }
}
fn owner(value: &Value) -> Result<DefinitionRules, String> {
    serde_json::from_value(value.clone()).map_err(|e| format!("typed owner: {e}"))
}
fn program(value: &Value) -> Result<RuleProgram, String> {
    serde_json::from_value(value.clone()).map_err(|e| format!("typed program: {e}"))
}

pub fn review_authoring(a: &Value) -> Result<(), String> {
    ensure(
        a["schema_version"] == 1 && a["before"] == BEFORE,
        "exact publication predecessor",
    )?;
    ensure(
        a["scope"]["publication_ready"] == true,
        "publication readiness",
    )?;
    for field in [
        "complete_request",
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
            pin["path"] == format!("data/owned/poe2/3887ae68/outgoing-critical/{name}"),
            "exact artifact path/order",
        )?;
        let path = family::data().join(name);
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

/// Count every potential writer, including zero/inactive effects and application
/// programs. This is an authority check, not evaluation or source applicability.
fn writers(rules: &Value, keys: &BTreeSet<&str>) -> Result<Value, String> {
    let writes = |p: &Value| -> Result<bool, String> {
        Ok(p["effects"]
            .as_array()
            .ok_or("program effects")?
            .iter()
            .any(|e| {
                e["effect"]["stat"]["key"]
                    .as_str()
                    .is_some_and(|key| keys.contains(key))
            }))
    };
    let mut result = vec![];
    for o in rules["owners"].as_array().ok_or("owner inventory")? {
        for p in o["programs"]["members"]
            .as_array()
            .ok_or("program inventory")?
        {
            if writes(p)? {
                result.push(json!({"kind":"owner", "owner":o["owner"], "program":program(p)?}));
            }
        }
    }
    for app in rules["effect_applications"]["members"]
        .as_array()
        .ok_or("application inventory")?
    {
        if writes(&app["program"])? {
            result.push(json!({"kind":"application", "application":app}));
        }
    }
    Ok(json!(result))
}

/// Review a predecessor or the exact component inverse. No coverage is promoted.
pub fn review_predecessor(schema: &Value, rules: &Value, routing: &Value) -> Result<(), String> {
    let d = family::read("dependencies.json");
    ensure(
        d["before"] == BEFORE && d["inspection_endpoint"] == BEFORE && d["registry_last"] == 0x33a9,
        "dependency predecessor identity",
    )?;
    let mut expected_owners = d["owners_before"]
        .as_array()
        .ok_or("before owners")?
        .clone();
    ensure(expected_owners.len() == 2, "two existing consumer owners")?;
    expected_owners.push(d["baseline_owner_before"].clone());
    for expected in expected_owners {
        let actual: Vec<_> = rules["owners"]
            .as_array()
            .ok_or("owner inventory")?
            .iter()
            .filter(|o| o["owner"] == expected["owner"])
            .collect();
        ensure(
            actual.len() == 1 && owner(actual[0])? == owner(&expected)?,
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
        let matches: Vec<_> = definitions
            .iter()
            .filter(|v| v["value"]["id"] == expected["value"]["id"])
            .collect();
        ensure(matches.len() == 1, "dependency descriptor multiplicity")?;
        let a: DefinitionDescriptor =
            serde_json::from_value(matches[0].clone()).map_err(|e| e.to_string())?;
        let b: DefinitionDescriptor =
            serde_json::from_value(expected.clone()).map_err(|e| e.to_string())?;
        ensure(a == b, "exact dependency descriptor")?;
    }
    let mut expected_keys = vec![0x2539, 0x3215, 0x321b, 0x321c];
    expected_keys.extend(0x33aa..=0x33b0);
    let expected_keys: Vec<_> = expected_keys
        .into_iter()
        .map(|n| format!("def.{n:016x}"))
        .collect();
    ensure(
        d["potential_writer_keys"] == json!(expected_keys),
        "exact potential writer channels",
    )?;
    let keys: BTreeSet<_> = expected_keys.iter().map(String::as_str).collect();
    let mut expected_writers = d["potential_writers"].clone();
    for row in expected_writers
        .as_array_mut()
        .ok_or("expected writer inventory")?
    {
        ensure(row["kind"] == "owner", "reviewed prior writer kind")?;
        row["program"] = json!(program(&row["program"])?);
    }
    ensure(
        writers(rules, &keys)? == expected_writers,
        "all-owner/application potential writer census",
    )?;
    let routes = d["input_routes"]
        .as_array()
        .ok_or("input route commitments")?;
    ensure(routes.len() == 1, "one exact Action output route")?;
    for expected in routes {
        let actual: Vec<_> = routing["outputs"]
            .as_array()
            .ok_or("routing output inventory")?
            .iter()
            .filter(|r| r["output"] == expected["output"])
            .collect();
        ensure(
            actual.len() == 1 && *actual[0] == *expected,
            "exact Action intrinsic and accuracy input routes",
        )?;
    }
    Ok(())
}

pub fn check_authored() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        review_authoring(&family::read("authoring.json")).unwrap();
        ordinary::check();
        rounding::review_nodes(&family::read("consumer.json")).unwrap();
        let a = family::read("authoring.json");
        let m: OwnedReleaseMigrationInput =
            serde_json::from_value(family::read("migration.json")).unwrap();
        assert_eq!(m.schema_version, 5);
        assert_eq!(json!(m.before), a["before"]);
        assert_eq!(json!(m.release), a["release"]);
        assert_eq!(m.contract.schema_version, 7);
        assert_eq!(
            m.contract.operations_version.as_str(),
            "owned-domain-operations-v28"
        );
        assert_eq!(m.schema.len(), 7);
        let expected: Vec<_> = family::read("definitions.json")
            .as_array()
            .unwrap()
            .iter()
            .map(|d| json!({"kind":"definition", "value":d}))
            .collect();
        assert_eq!(json!(m.schema), json!(expected));
        assert!(
            m.owners.is_empty()
                && m.tables.is_empty()
                && m.receivers.is_empty()
                && m.query_targets.is_empty()
                && m.evaluation.is_none()
        );
        let d = family::read("dependencies.json");
        let b = family::read("baseline.json");
        assert_eq!(d["baseline_owner_before"]["owner"], b["owner"]);
        let before = owner(&d["baseline_owner_before"]).unwrap();
        let p = program(&b["before"]).unwrap();
        assert_eq!(
            before.programs.members.iter().filter(|v| **v == p).count(),
            1
        );
        for (authored, old) in family::consumer()
            .owners
            .iter()
            .zip(d["owners_before"].as_array().unwrap())
        {
            assert_eq!(authored.owner, owner(old).unwrap().owner);
            assert_eq!(authored.programs.members.len(), 1);
            assert!(
                !authored.programs.is_complete() && !owner(old).unwrap().programs.is_complete()
            );
        }
    });
}

fn remove_component(rules: &mut poe_optimizer_core::owned_rules::RulePackageInput) {
    let c = family::consumer();
    for appended in c.owners.iter().rev() {
        let target = rules
            .owners
            .iter_mut()
            .find(|o| o.owner == appended.owner)
            .unwrap();
        for p in appended.programs.members.iter().rev() {
            assert_eq!(target.programs.members.pop(), Some(p.clone()));
        }
    }
    for q in c.queries.iter().rev() {
        assert_eq!(
            rules.contribution_queries.as_mut().unwrap().members.pop(),
            Some(q.clone())
        );
    }
    let b = family::read("baseline.json");
    let before = program(&b["before"]).unwrap();
    let after = program(&b["after"]).unwrap();
    let target = rules
        .owners
        .iter_mut()
        .find(|o| json!(o.owner) == b["owner"])
        .unwrap();
    let p = target
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == before.id)
        .unwrap();
    assert_eq!(*p, after);
    *p = before;
}

pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    assert_eq!(endpoint.input().recipe.registry.last_issued.get(), 0x33b0);
    let recipe = &endpoint.input().recipe;
    for definition in family::read("definitions.json").as_array().unwrap() {
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
    let a = family::read("authoring.json");
    assert_eq!(json!(prior.receipt().input), a["before"]);
    assert_eq!(prior.input().recipe.registry.last_issued.get(), 0x33a9);
    review_predecessor(
        &json!(prior.input().recipe.schema),
        &json!(prior.input().recipe.rules),
        &json!(prior.input().recipe.routing),
    )
    .unwrap();
    let migration = serde_json::from_value(family::read("migration.json")).unwrap();
    let migrated = compile_owned_release_migration(prior, migration, Default::default()).unwrap();
    // Authenticate the schema-only transaction independently of the later rule
    // inverse. Dependency rebinding is allowed only at its checked locations.
    let old_recipe = &prior.input().recipe;
    let mut schema_inverse = migrated.input().recipe.schema.clone();
    for expected in family::read("definitions.json").as_array().unwrap() {
        let expected: DefinitionDescriptor = serde_json::from_value(expected.clone()).unwrap();
        // Assembly canonicalizes definitions by typed address, independently
        // of their monotonic allocation sequence in the registry below.
        let matches: Vec<_> = schema_inverse
            .definitions
            .iter()
            .enumerate()
            .filter_map(|(index, actual)| (actual.address() == expected.address()).then_some(index))
            .collect();
        assert_eq!(matches.len(), 1, "exact new definition multiplicity");
        assert_eq!(schema_inverse.definitions.remove(matches[0]), expected);
    }
    schema_inverse.release = old_recipe.schema.release.clone();
    assert!(
        schema_inverse == old_recipe.schema,
        "exact seven-definition append inverse"
    );
    let mut rules_inverse = migrated.input().recipe.rules.clone();
    rules_inverse.definitions = old_recipe.rules.definitions.clone();
    assert!(
        rules_inverse == old_recipe.rules,
        "schema-only migration must preserve every prior rule and closure"
    );
    let mut registry_inverse = json!(migrated.input().recipe.registry);
    let old_registry = json!(old_recipe.registry);
    for sequence in (0x33aa..=0x33b0).rev() {
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
        old_registry["revision"].as_u64().unwrap() + 7
    );
    registry_inverse["revision"] = old_registry["revision"].clone();
    registry_inverse["last_issued"] = old_registry["last_issued"].clone();
    assert!(
        registry_inverse == old_registry,
        "exact registry append inverse"
    );
    migration_preservation::assert_import_rebindings_only(prior, &migrated);
    let mut input = migrated.input().clone();
    let c = family::consumer();
    for appended in &c.owners {
        let target = input
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == appended.owner)
            .unwrap();
        for p in &appended.programs.members {
            assert!(!target.programs.members.iter().any(|v| v.id == p.id));
            target.programs.members.push(p.clone());
        }
    }
    let queries = input.recipe.rules.contribution_queries.as_mut().unwrap();
    for q in &c.queries {
        assert!(!queries.members.iter().any(|v| v.id == q.id));
        queries.members.push(q.clone());
    }
    let b = family::read("baseline.json");
    let target = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| json!(o.owner) == b["owner"])
        .unwrap();
    let before = program(&b["before"]).unwrap();
    let p = target
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == before.id)
        .unwrap();
    assert_eq!(*p, before);
    *p = program(&b["after"]).unwrap();
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: "outgoing-critical-component".parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned("outgoing-critical-component", &a, 1024 * 1024).unwrap(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    remove_component(&mut inverse.recipe.rules);
    inverse.provenance = migrated.input().provenance.clone();
    assert!(
        inverse == *migrated.input(),
        "exact component inverse preserves all prior rules, closures and release inputs"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    next
}
