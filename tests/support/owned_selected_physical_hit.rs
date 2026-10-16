//! Source-admitted physical hit endpoints; no whole-build coverage promotion.
use super::{migration_preservation, physical_base_family};
use flate2::read::GzDecoder;
use poe_optimizer_core::{owned_content::*, owned_rules::*, owned_schema::*};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf, sync::OnceLock};

pub const PROGRAM: &str = "selected-physical-hit";
pub const CONDITIONS: [u64; 2] = [0x339e, 0x339f];
pub const OUTPUTS: [u64; 2] = [0x33a0, 0x33a1];
const FILES: [&str; 8] = [
    "authoring.json",
    "migration.json",
    "consumer.json",
    "dependencies.json",
    "bindings.json",
    "source-evidence.json",
    "source-vectors.json",
    "source-domain.json",
];
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/selected-physical-hit")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn authenticated(pin: &Value) -> Vec<u8> {
    let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
    assert_eq!(bytes.len(), pin["bytes"]);
    assert_eq!(hash(&bytes), pin["sha256"]);
    bytes
}
fn decompress(bytes: &[u8], limit: u64) -> Vec<u8> {
    let mut raw = vec![];
    GzDecoder::new(bytes)
        .take(limit + 1)
        .read_to_end(&mut raw)
        .unwrap();
    assert!(raw.len() as u64 <= limit);
    raw
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub owners: Vec<DefinitionRules>,
    pub queries: Vec<ContributionQuery>,
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn migration() -> OwnedReleaseMigrationInput {
    read("migration.json")
}
fn commitment() -> OwnedContentDigest {
    digest_owned(PROGRAM, &FILES.map(read::<Value>), 16 * 1024 * 1024).unwrap()
}

/// Inspect complete captured source bodies, including zero, false, inactive and
/// nested suppliers. Source-specific spelling stays within this offline check.
pub fn reject_unreviewed_condition_records(value: &Value) -> Result<(), String> {
    match value {
        Value::Object(o) => {
            if o.contains_key("type")
                && matches!(
                    o.get("name").and_then(Value::as_str),
                    Some(
                        "Condition:Intimidated"
                            | "Condition:Party:Intimidated"
                            | "Condition:Debilitated"
                            | "PhysicalDamageConvertToRandom"
                    )
                )
            {
                return Err("unreviewed exact-recipient condition supplier".into());
            }
            for field in [
                "conditions",
                "overrideCond",
                "skillCond",
                "skill_conditions",
            ] {
                if let Some(conditions) = o.get(field).and_then(Value::as_object)
                    && ["Intimidated", "Party:Intimidated", "Debilitated"]
                        .iter()
                        .any(|n| conditions.contains_key(*n))
                {
                    return Err("unreviewed direct condition-state supplier".into());
                }
            }
            for child in o.values() {
                reject_unreviewed_condition_records(child)?;
            }
        }
        Value::Array(rows) => {
            for child in rows {
                reject_unreviewed_condition_records(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}
pub fn source_report() -> &'static Value {
    static REPORT: OnceLock<Value> = OnceLock::new();
    REPORT.get_or_init(|| {
        let evidence: Value = read("source-evidence.json");
        let report = &evidence["report"];
        let raw = decompress(&authenticated(&report["archive"]), 24 * 1024 * 1024);
        assert_eq!(raw.len(), report["bytes"]);
        assert_eq!(hash(&raw), report["sha256"]);
        assert_eq!(
            hash(&raw),
            "030f14d71a01a2c54862eb858249b2abca1ba1caa9337dbffb3458118777e935"
        );
        serde_json::from_slice(&raw).unwrap()
    })
}
fn source_rows(v: &Value) -> &[Value] {
    if let Some(rows) = v.as_array() {
        rows
    } else {
        assert!(v.as_object().is_some_and(serde_json::Map::is_empty));
        &[]
    }
}
pub fn check_source() {
    let evidence: Value = read("source-evidence.json");
    physical_base_family::check_source();
    let report = source_report();
    assert_eq!(report["source_revision"], evidence["source_revision"]);
    assert_eq!(report["evidence"]["case_count"], 37);
    assert_eq!(report["evidence"]["complete_load_attempts_per_jit"], 38);
    assert_eq!(report["evidence"]["business_method_wrappers"], false);
    let mut inputs = vec![];
    for pin in evidence["selected_inputs"].as_array().unwrap() {
        let bytes = authenticated(pin);
        let raw = if pin["path"].as_str().unwrap().ends_with(".gz") {
            decompress(&bytes, 96 * 1024 * 1024)
        } else {
            bytes
        };
        inputs.push(serde_json::from_slice::<Value>(&raw).unwrap());
    }
    assert_eq!(inputs.len(), 3);
    for source in &inputs[..2] {
        let state = &source["state"];
        for (field, count) in [
            ("environments", 21),
            ("baseline", 18),
            ("configuration", 2),
            ("skill_construction", 1536),
        ] {
            assert_eq!(source_rows(&state[field]).len(), count);
            reject_unreviewed_condition_records(&state[field]).unwrap();
        }
    }
    reject_unreviewed_condition_records(&inputs[2]).unwrap();
    check_source_domain(report, &evidence);
    let vectors: Value = read("source-vectors.json");
    authenticated(&evidence["vectors"]);
    let mut reconstructed = vec![];
    for case in report["cases"].as_array().unwrap() {
        for mode in ["main", "calcs"] {
            if case["state"].get(mode).is_none() {
                continue;
            }
            for actor in source_rows(&case["state"][mode]["actors"]) {
                if actor["actor_profile"] != "RaisedSkeletonSniper" {
                    continue;
                }
                for child in source_rows(&actor["children"]) {
                    if child["effect_id"] != "MinionMeleeBow" {
                        continue;
                    }
                    if child.get("damage_calls").is_none() {
                        continue;
                    }
                    for call in source_rows(&child["damage_calls"]) {
                        if call["damage_type"] != "Physical" {
                            continue;
                        }
                        assert!(
                            call.get("returned_min").is_some()
                                && call.get("returned_max").is_some(),
                            "observed physical call must retain both returned endpoints"
                        );
                        reconstructed.push(json!({"case":case["name"],"mode":mode,
                            "source_occurrence":actor["source_occurrence"],"quality":actor["quality"],
                            "hidden_damage_fixup":actor["hidden_damage_fixup"],"call":call}));
                    }
                }
            }
        }
    }
    assert_eq!(reconstructed.len(), 76);
    assert_eq!(json!(reconstructed), vectors["vectors"]);
    // This is the original observed return, never a calculated expected value.
    for row in reconstructed.iter().filter(|r| r["case"] == "original-05") {
        assert_eq!(row["hidden_damage_fixup"], 0);
        assert_eq!(row["call"]["returned_min"], 574);
        assert_eq!(row["call"]["returned_max"], 1068);
        assert_eq!(row["call"]["minimum_more"], 1);
        assert_eq!(row["call"]["maximum_more"], 1);
        assert_eq!(row["call"]["query_state_preserved"], true);
    }
}

fn scanned_line(text: &str) -> bool {
    [
        "Intimidated",
        "Debilitated",
        "DamageConvertTo",
        "DamageGainAs",
        "DamageAs",
        "overrideCond",
        "conditions[",
        "condList[",
        "skillCond",
        "NewMod(\"Damage\"",
        "NewMod(\"PhysicalDamage\"",
        "NewMod(\"MinPhysicalDamage\"",
        "NewMod(\"MaxPhysicalDamage\"",
    ]
    .iter()
    .any(|s| text.contains(s))
}
pub fn check_selected_physical_inputs(inputs: &Value) -> Result<(), String> {
    reject_unreviewed_condition_records(inputs)?;
    for row in source_rows(&inputs["raw_skill_modifiers"]) {
        let name = row["mod"]["name"]
            .as_str()
            .ok_or("missing source modifier name")?;
        if name.contains("DamageConvertTo")
            || name.ends_with("DamageGainAsPhysical")
            || name.ends_with("DamageAsPhysical")
            || [
                "PhysicalDamage",
                "MinPhysicalDamage",
                "MaxPhysicalDamage",
                "DealNoDamage",
                "DealNoPhysical",
            ]
            .contains(&name)
        {
            return Err("unreviewed physical conversion, gain, type or endpoint supplier".into());
        }
    }
    if inputs["bases"]["Physical"]["can_deal"] != true {
        return Err("physical damage suppression is outside the admitted source domain".into());
    }
    // Exact final source matrices are an independent numerical control. The
    // raw-record census above is what rejects inactive/zero potential suppliers.
    let types = ["Chaos", "Cold", "Fire", "Lightning", "Physical"];
    let conversion = inputs["conversion_table"]
        .as_object()
        .ok_or("conversion matrix")?;
    if conversion.keys().map(String::as_str).collect::<Vec<_>>() != types {
        return Err("incomplete conversion matrix".into());
    }
    for (from, row) in conversion {
        let row = row.as_object().ok_or("conversion row")?;
        if row.keys().map(String::as_str).collect::<Vec<_>>()
            != ["Chaos", "Cold", "Fire", "Lightning", "Physical", "mult"]
        {
            return Err("incomplete conversion row".into());
        }
        for (to, amount) in row {
            let expected = if to == "mult" { 1.0 } else { 0.0 };
            if amount.as_f64() != Some(expected) {
                return Err(format!("nonidentity conversion {from}/{to}"));
            }
        }
    }
    let gain = inputs["gain_table"].as_object().ok_or("gain matrix")?;
    if gain.keys().map(String::as_str).collect::<Vec<_>>() != types {
        return Err("incomplete gain matrix".into());
    }
    for row in gain.values() {
        let row = row.as_object().ok_or("gain row")?;
        if row.keys().map(String::as_str).collect::<Vec<_>>() != types {
            return Err("incomplete gain row".into());
        }
        if row["Physical"].as_f64() != Some(0.0) {
            return Err("physical gain".into());
        }
    }
    Ok(())
}
fn check_source_domain(report: &Value, evidence: &Value) {
    authenticated(&evidence["domain"]);
    let domain: Value = read("source-domain.json");
    assert_eq!(domain["complete_bodies"].as_array().unwrap().len(), 5);
    for source in domain["complete_bodies"].as_array().unwrap() {
        let bytes = decompress(&authenticated(&source["archive"]), 2 * 1024 * 1024);
        assert_eq!(bytes.len(), source["bytes"]);
        assert_eq!(hash(&bytes), source["sha256"]);
        let text = String::from_utf8(bytes).unwrap().replace("\r\n", "\n");
        let scan: Vec<_> = text
            .split('\n')
            .enumerate()
            .filter(|(_, s)| scanned_line(s))
            .map(|(n, s)| json!({"line":n+1,"text":s}))
            .collect();
        assert_eq!(json!(scan), source["scan"]);
    }
    let original = report["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "original-05")
        .unwrap();
    let mut selected = vec![];
    for mode in ["main", "calcs"] {
        if original["state"].get(mode).is_none() {
            continue;
        }
        for actor in source_rows(&original["state"][mode]["actors"]) {
            if actor["actor_profile"] != "RaisedSkeletonSniper" {
                continue;
            }
            assert_eq!(actor["hidden_damage_fixup"], 0);
            assert_eq!(actor["hostile"], false);
            for child in source_rows(&actor["children"]) {
                if child["effect_id"] != "MinionMeleeBow" {
                    continue;
                }
                if child.get("passes").is_none() {
                    continue;
                }
                for pass in source_rows(&child["passes"]) {
                    check_selected_physical_inputs(&pass["inputs"]).unwrap();
                    selected.push(json!({"mode":mode,"source_occurrence":actor["source_occurrence"],"actor_profile":actor["actor_profile"],"profile":actor["profile"],"hidden_damage_fixup":actor["hidden_damage_fixup"],"label":pass["label"],"inputs":pass["inputs"]}));
                }
            }
        }
    }
    assert_eq!(selected.len(), 1);
    assert_eq!(json!(selected), domain["selected"]);
}

pub fn writers(rules: &Value, keys: &[&str]) -> Value {
    let mut rows = vec![];
    let touches = |p: &Value| {
        p["effects"].as_array().unwrap().iter().any(|e| {
            e["effect"]["stat"]["key"]
                .as_str()
                .is_some_and(|k| keys.contains(&k))
        })
    };
    for owner in rules["owners"].as_array().unwrap() {
        for program in owner["programs"]["members"].as_array().unwrap() {
            if touches(program) {
                rows.push(json!({"kind":"owner","owner":owner["owner"],"program":program}));
            }
        }
    }
    for application in rules["effect_applications"]["members"].as_array().unwrap() {
        if touches(&application["program"]) {
            rows.push(json!({"kind":"application","application":application}));
        }
    }
    json!(rows)
}
pub fn check_native(schema: &Value, rules: &Value, routes: &Value, after: bool) {
    let dep: Value = read("dependencies.json");
    let c = consumer();
    let mut prior = rules.clone();
    let owner = prior["owners"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|o| o["owner"] == json!(c.owners[0].owner))
        .unwrap();
    if after {
        let rows = owner["programs"]["members"].as_array_mut().unwrap();
        let hits: Vec<_> = rows
            .iter()
            .enumerate()
            .filter_map(|(n, p)| (p["id"] == PROGRAM).then_some(n))
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(rows.remove(hits[0]), json!(c.owners[0].programs.members[0]));
    }
    assert_eq!(*owner, dep["owner_before"]);
    assert_eq!(
        writers(
            &prior,
            &[
                "def.000000000000339e",
                "def.000000000000339f",
                "def.00000000000033a0",
                "def.00000000000033a1"
            ]
        ),
        json!([])
    );
    assert_eq!(
        writers(
            &prior,
            &[
                "def.000000000000339c",
                "def.000000000000339d",
                "def.000000000000335c",
                "def.000000000000001d"
            ]
        ),
        dep["input_writers"]
    );
    assert_eq!(prior["effect_applications"], dep["effect_applications"]);
    assert_eq!(prior["existing_actor_rules"], dep["existing_actor_rules"]);
    for descriptor in dep["definitions"].as_array().unwrap() {
        assert!(
            schema["definitions"]
                .as_array()
                .unwrap()
                .contains(descriptor)
        );
    }
    let more: Vec<_> = prior["contribution_queries"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|q| q["id"] == dep["more_query"]["id"])
        .collect();
    assert_eq!(more, vec![&dep["more_query"]]);
    assert_eq!(
        prior["contribution_queries"]["closure"],
        dep["remaining_query_registry_closure"]
    );
    assert_eq!(
        prior["receivers"]["closure"],
        dep["remaining_receiver_registry_closure"]
    );
    physical_base_family::check_native(schema, &prior, routes, true);
}
pub fn check_authored() {
    let authoring: Value = read("authoring.json");
    for name in &FILES[1..] {
        let bytes = fs::read(data().join(name)).unwrap();
        assert_eq!(bytes.len(), authoring["artifacts"][name]["bytes"]);
        assert_eq!(hash(&bytes), authoring["artifacts"][name]["sha256"]);
    }
    let consumer = consumer();
    assert_eq!(consumer.owners.len(), 1);
    assert_eq!(consumer.owners[0].programs.members.len(), 1);
    assert!(matches!(
        consumer.owners[0].programs.closure,
        SchemaClosure::Partial { .. }
    ));
    assert_eq!(consumer.queries.len(), 2);
    for query in &consumer.queries {
        assert_eq!(query.contribution, ContributionKind::Flag);
        assert_eq!(query.groups.len(), 1);
        assert!(query.groups[0].members.is_complete());
        assert!(query.groups[0].members.members.is_empty());
    }
    let dependencies: Value = read("dependencies.json");
    for pin in dependencies["upstream_packet_files"].as_array().unwrap() {
        authenticated(pin);
    }
    let p = &consumer.owners[0].programs.members[0];
    assert_eq!(p.id.as_str(), PROGRAM);
    assert_eq!(p.reads.len(), 7);
    assert_eq!(p.effects.len(), 2);
    // Using the old already-rounded factor would round the bucket twice.
    assert!(
        !serde_json::to_string(p)
            .unwrap()
            .contains("def.000000000000330b")
    );
    check_source();
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let recipe = &endpoint.input().recipe;
    for entry in migration().schema {
        let poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry::Definition(d) =
            entry
        else {
            panic!("Stat only")
        };
        assert!(recipe.schema.definitions.contains(&d));
    }
    check_native(
        &json!(recipe.schema),
        &json!(recipe.rules),
        &json!(recipe.routing.outputs),
        true,
    );
    for q in consumer().queries {
        assert!(
            recipe
                .rules
                .contribution_queries
                .as_ref()
                .unwrap()
                .members
                .contains(&q)
        );
    }
    assert!(endpoint.evaluation().is_none());
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    assert_eq!(
        a["scope"]["publication_ready"], true,
        "source/census review is still pending"
    );
    assert_eq!(json!(prior.receipt().input), a["before"]);
    assert_eq!(prior.input().recipe.registry.last_issued.get(), 0x339d);
    let r = &prior.input().recipe;
    check_native(
        &json!(r.schema),
        &json!(r.rules),
        &json!(r.routing.outputs),
        false,
    );
    let migrated = compile_owned_release_migration(prior, migration(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    let c = consumer();
    input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == c.owners[0].owner)
        .unwrap()
        .programs
        .members
        .extend(c.owners[0].programs.members.clone());
    input
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(c.queries);
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: PROGRAM.parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: commitment(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    assert_eq!(
        inverse
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == c.owners[0].owner)
            .unwrap()
            .programs
            .members
            .pop(),
        Some(c.owners[0].programs.members[0].clone())
    );
    for q in consumer().queries.into_iter().rev() {
        assert_eq!(
            inverse
                .recipe
                .rules
                .contribution_queries
                .as_mut()
                .unwrap()
                .members
                .pop(),
            Some(q)
        );
    }
    inverse.provenance = migrated.input().provenance.clone();
    assert!(inverse == *migrated.input());
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    next
}
