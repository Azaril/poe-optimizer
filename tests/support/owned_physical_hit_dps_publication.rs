//! Append an internal physical component without completing any existing owner.
use super::{dps, migration_preservation, source};
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_rules::{DefinitionRules, RulePackageInput, RuleProgram},
    owned_schema::DefinitionDescriptor,
};
use poe_optimizer_import::owned_release::{
    OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
};
use poe_optimizer_import::owned_release_migration::{
    OwnedReleaseMigrationInput, compile_owned_release_migration,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs};

const BEFORE: &str = "f72d73bb194b457aef22b09c94438bb2d308ba771fb6bc049788e5d09cc965d8";
const LAST: u64 = 0x33b7;
const END: u64 = 0x33bb;
const PROGRAM: &str = "ordinary-physical-hit-dps";
const KIND: &str = "ordinary-physical-hit-dps-component";
const FILES: [&str; 8] = [
    "definitions.json",
    "consumer.json",
    "bindings.json",
    "dependencies.json",
    "migration.json",
    "source-evidence.json",
    "source-domain.json",
    "source-vectors.json",
];

fn read(name: &str) -> Value {
    dps::read(name)
}

pub fn review_authoring(authoring: &Value) -> Result<(), String> {
    if authoring["schema_version"] != 1
        || authoring["before"] != BEFORE
        || authoring["release"] != read("migration.json")["release"]
        || authoring["scope"]["publication_ready"] != true
    {
        return Err("reviewed predecessor and publication required".into());
    }
    for field in [
        "complete_request",
        "candidate_source_closure",
        "query_closure_changed",
        "existing_owner_closure_changed",
        "new_source_execution",
        "ready_for_native_admission",
    ] {
        if authoring["scope"][field] != false {
            return Err(format!("unsupported coverage claim: {field}"));
        }
    }
    let pins = authoring["artifacts"]
        .as_array()
        .ok_or("missing authoring inventory")?;
    if pins.len() != FILES.len() {
        return Err("exact artifact inventory required".into());
    }
    for (pin, name) in pins.iter().zip(FILES) {
        if pin["path"] != format!("data/owned/poe2/3887ae68/physical-hit-dps/{name}") {
            return Err("exact authoring path/order required".into());
        }
        let path = dps::data().join(name);
        let size = fs::metadata(&path).map_err(|e| e.to_string())?.len();
        if size > 2 * 1024 * 1024 || pin["bytes"] != size {
            return Err("bounded exact artifact bytes required".into());
        }
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        if pin["sha256"] != format!("{:x}", Sha256::digest(&bytes)) {
            return Err("exact artifact hash required".into());
        }
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Consumer {
    owners: Vec<DefinitionRules>,
    queries: Vec<poe_optimizer_core::owned_rules::ContributionQuery>,
}

fn consumer() -> Consumer {
    let value: Consumer = serde_json::from_value(read("consumer.json")).unwrap();
    assert_eq!(value.owners.len(), 1);
    assert!(
        value.queries.is_empty(),
        "no new contributor or metric query"
    );
    let owner = &value.owners[0];
    assert!(!owner.programs.is_complete());
    assert_eq!(owner.programs.members.len(), 1);
    assert_eq!(owner.programs.members[0].id.as_str(), PROGRAM);
    value
}

fn writers(rules: &Value, keys: &BTreeSet<&str>) -> Result<Value, String> {
    let writes = |program: &Value| -> Result<bool, String> {
        Ok(program["effects"]
            .as_array()
            .ok_or("missing effect inventory")?
            .iter()
            .any(|effect| {
                effect["effect"]["stat"]["key"]
                    .as_str()
                    .is_some_and(|key| keys.contains(key))
            }))
    };
    let mut result = Vec::new();
    for owner in rules["owners"].as_array().ok_or("missing owners")? {
        for program in owner["programs"]["members"]
            .as_array()
            .ok_or("missing programs")?
        {
            if writes(program)? {
                let program: RuleProgram =
                    serde_json::from_value(program.clone()).map_err(|e| e.to_string())?;
                result.push(json!({"kind":"owner","owner":owner["owner"],"program":program}));
            }
        }
    }
    for application in rules["effect_applications"]["members"]
        .as_array()
        .ok_or("missing application inventory")?
    {
        if writes(&application["program"])? {
            result.push(json!({"kind":"application","application":application}));
        }
    }
    Ok(json!(result))
}

pub fn review_predecessor(schema: &Value, rules: &Value, routing: &Value) -> Result<(), String> {
    let dependencies = read("dependencies.json");
    if dependencies["before"] != BEFORE
        || dependencies["inspection_endpoint"] != BEFORE
        || dependencies["registry_last"] != LAST
    {
        return Err("exact direct-Gem predecessor required".into());
    }
    for (field, source, selector) in [
        ("definitions", &schema["definitions"], "value"),
        ("owners_before", &rules["owners"], "owner"),
        ("input_routes", &routing["outputs"], "output"),
    ] {
        let expected = dependencies[field]
            .as_array()
            .ok_or_else(|| format!("missing dependency inventory: {field}"))?;
        let actual = source
            .as_array()
            .ok_or_else(|| format!("missing predecessor inventory: {field}"))?;
        for row in expected {
            let matching: Vec<_> = actual
                .iter()
                .filter(|candidate| {
                    if field == "definitions" {
                        candidate[selector]["id"] == row[selector]["id"]
                    } else {
                        candidate[selector] == row[selector]
                    }
                })
                .collect();
            let identical = if field == "owners_before" && matching.len() == 1 {
                // Staged releases serialize finite numeric operands as f64.
                // Compare the whole checked record, retaining closure and order.
                let expected: DefinitionRules =
                    serde_json::from_value(row.clone()).map_err(|e| e.to_string())?;
                let actual: DefinitionRules =
                    serde_json::from_value(matching[0].clone()).map_err(|e| e.to_string())?;
                actual == expected
            } else {
                matching == vec![row]
            };
            if !identical {
                return Err(format!("changed or ambiguous dependency: {field}"));
            }
        }
    }
    let keys: Vec<String> = serde_json::from_value(dependencies["potential_writer_keys"].clone())
        .map_err(|e| e.to_string())?;
    let keys: BTreeSet<_> = keys.iter().map(String::as_str).collect();
    let mut expected = dependencies["potential_writers"].clone();
    for row in expected.as_array_mut().ok_or("missing writer census")? {
        if row["kind"] != "owner" {
            return Err("unreviewed predecessor writer kind".into());
        }
        let program: RuleProgram =
            serde_json::from_value(row["program"].clone()).map_err(|e| e.to_string())?;
        row["program"] = json!(program);
    }
    if writers(rules, &keys)? != expected {
        return Err("all-owner/application potential writer census changed".into());
    }
    Ok(())
}

fn remove_component(rules: &mut RulePackageInput) {
    let component = consumer();
    let target: Vec<_> = rules
        .owners
        .iter_mut()
        .filter(|owner| owner.owner == component.owners[0].owner)
        .collect();
    assert_eq!(target.len(), 1);
    let target = target.into_iter().next().unwrap();
    assert_eq!(
        target.programs.members.pop(),
        component.owners[0].programs.members.first().cloned()
    );
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    dps::check();
    source::check();
    review_authoring(&read("authoring.json")).unwrap();
    assert_eq!(prior.receipt().input.to_string(), BEFORE);
    assert_eq!(
        prior.input().recipe.registry.last_issued.get(),
        i64::try_from(LAST).unwrap()
    );
    let old = &prior.input().recipe;
    review_predecessor(&json!(old.schema), &json!(old.rules), &json!(old.routing)).unwrap();
    let authoring = read("authoring.json");
    let migration: OwnedReleaseMigrationInput =
        serde_json::from_value(read("migration.json")).unwrap();
    assert_eq!(migration.before.to_string(), BEFORE);
    assert!(
        migration.owners.is_empty()
            && migration.tables.is_empty()
            && migration.receivers.is_empty()
            && migration.query_targets.is_empty()
            && migration.evaluation.is_none()
    );
    let migrated = compile_owned_release_migration(prior, migration, Default::default()).unwrap();
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(read("definitions.json")).unwrap();
    assert_eq!(definitions.len(), 4);
    let mut schema_inverse = migrated.input().recipe.schema.clone();
    for definition in &definitions {
        let positions: Vec<_> = schema_inverse
            .definitions
            .iter()
            .enumerate()
            .filter_map(|(index, value)| (value.address() == definition.address()).then_some(index))
            .collect();
        assert_eq!(positions.len(), 1);
        assert_eq!(schema_inverse.definitions.remove(positions[0]), *definition);
    }
    schema_inverse.release = old.schema.release.clone();
    assert!(
        schema_inverse == old.schema,
        "exact definition append inverse"
    );
    let mut rules_inverse = migrated.input().recipe.rules.clone();
    rules_inverse.definitions = old.rules.definitions.clone();
    assert!(rules_inverse == old.rules, "schema-only migration");
    let mut registry_inverse = json!(migrated.input().recipe.registry);
    let old_registry = json!(old.registry);
    for sequence in ((LAST + 1)..=END).rev() {
        let entry = registry_inverse["entries"]
            .as_array_mut()
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(entry["sequence"], sequence);
        assert_eq!(entry["state"], json!({"kind":"active"}));
        assert_eq!(entry["target"]["kind"], "definition");
        assert_eq!(
            entry["target"]["value"]["kind"],
            if sequence == END { "unit" } else { "stat" }
        );
        assert_eq!(
            entry["target"]["value"]["value"]["key"],
            format!("def.{sequence:016x}")
        );
    }
    assert_eq!(
        registry_inverse["revision"].as_u64().unwrap(),
        old_registry["revision"].as_u64().unwrap() + END - LAST
    );
    registry_inverse["revision"] = old_registry["revision"].clone();
    registry_inverse["last_issued"] = old_registry["last_issued"].clone();
    assert!(registry_inverse == old_registry, "exact registry inverse");
    migration_preservation::assert_import_rebindings_only(prior, &migrated);

    let mut input = migrated.input().clone();
    let component = consumer();
    let target: Vec<_> = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .filter(|owner| owner.owner == component.owners[0].owner)
        .collect();
    assert_eq!(target.len(), 1);
    let target = target.into_iter().next().unwrap();
    assert!(!target.programs.is_complete());
    let program = &component.owners[0].programs.members[0];
    assert!(!target.programs.members.iter().any(|p| p.id == program.id));
    target.programs.members.push(program.clone());
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(KIND, &authoring, 1024 * 1024).unwrap(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    remove_component(&mut inverse.recipe.rules);
    inverse.provenance = migrated.input().provenance.clone();
    assert!(
        inverse == *migrated.input(),
        "exact component inverse must preserve all prior coverage and inputs"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert!(next.evaluation().is_none());
    assert_eq!(
        next.input().recipe.registry.last_issued.get(),
        i64::try_from(END).unwrap()
    );
    next
}
