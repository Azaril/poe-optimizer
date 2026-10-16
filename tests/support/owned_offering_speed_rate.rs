//! Offline Offering Speed data. Final rate admission requires the separately
//! enumerated eight-input source domain; no scalar output supplies that authority.
use flate2::read::GzDecoder;
use poe_optimizer_core::{owned_rules::*, owned_schema::*};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf, sync::OnceLock};

pub const APPLICATION: &str = "pain-offering-minion-damage";
pub const SPEED: u64 = 0x33a2;
pub const RATE: u64 = 0x33a3;
pub const PROGRAM: &str = "selected-basic-ordinary-attack-rate";
const REPORT_HASH: &str = "030f14d71a01a2c54862eb858249b2abca1ba1caa9337dbffb3458118777e935";
const SPEED_NODES: [&str; 7] = [
    "speed-base",
    "speed-scaled",
    "speed-scaled-hundred",
    "speed-biased-hundred",
    "speed-rounded-hundred",
    "speed-rounded-two-decimals",
    "speed-delivered",
];
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/offering-speed-rate")
}
fn bounded(path: &std::path::Path, limit: u64) -> Vec<u8> {
    assert!(
        fs::metadata(path).unwrap().len() <= limit,
        "bounded source file"
    );
    fs::read(path).unwrap()
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&bounded(&data().join(name), 2 * 1024 * 1024)).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn authenticated(pin: &Value, limit: u64) -> Vec<u8> {
    let bytes = bounded(&root().join(pin["path"].as_str().unwrap()), limit);
    assert_eq!(bytes.len(), pin["bytes"]);
    assert_eq!(hash(&bytes), pin["sha256"]);
    bytes
}
fn expanded(bytes: &[u8], limit: u64) -> Vec<u8> {
    let mut raw = vec![];
    GzDecoder::new(bytes)
        .take(limit + 1)
        .read_to_end(&mut raw)
        .unwrap();
    assert!(raw.len() as u64 <= limit);
    raw
}
fn rows(v: &Value) -> &[Value] {
    if let Some(a) = v.as_array() {
        a
    } else {
        assert!(v.as_object().is_some_and(serde_json::Map::is_empty));
        &[]
    }
}
pub fn source_report() -> &'static Value {
    static REPORT: OnceLock<Value> = OnceLock::new();
    REPORT.get_or_init(|| {
        let evidence: Value = read("source-evidence.json");
        let pin = &evidence["report"];
        assert_eq!(
            pin["archive"]["path"],
            "data/owned/poe2/3887ae68/selected-physical-hit/physical-source.json.gz"
        );
        let raw = expanded(
            &authenticated(&pin["archive"], 2 * 1024 * 1024),
            24 * 1024 * 1024,
        );
        assert_eq!(raw.len(), pin["bytes"]);
        assert_eq!(hash(&raw), REPORT_HASH);
        assert_eq!(pin["sha256"], REPORT_HASH);
        serde_json::from_slice(&raw).unwrap()
    })
}
pub fn applications() -> DeclaredSet<EffectApplicationRule> {
    read("applications.json")
}
pub fn tables() -> Vec<IntegerRuleTable> {
    read("tables.json")
}
pub fn definitions() -> Vec<DefinitionDescriptor> {
    read("definitions.json")
}
pub fn queries() -> Vec<ContributionQuery> {
    read("queries.json")
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub owners: Vec<DefinitionRules>,
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn migration() -> OwnedReleaseMigrationInput {
    read("migration.json")
}

/// Exact appended-effect inverse. Source identity, activation, recipient paths,
/// prior Damage arithmetic, reads, closure and stacking must all survive.
pub fn check_application(apps: &Value) -> Result<(), String> {
    let dependencies: Value = read("dependencies.json");
    let expected: Value = read("applications.json");
    if apps != &expected {
        return Err("authored application differs".into());
    }
    let mut inverse = apps.clone();
    let app = &mut inverse["members"][0];
    let nodes = app["program"]["nodes"]
        .as_array_mut()
        .ok_or("application nodes")?;
    for name in SPEED_NODES.into_iter().rev() {
        if nodes.pop().ok_or("missing Speed node")?["id"] != name {
            return Err("Speed node order".into());
        }
    }
    let effect = app["program"]["effects"]
        .as_array_mut()
        .ok_or("effects")?
        .pop()
        .ok_or("Speed effect")?;
    if effect["id"] != "speed-increase"
        || effect["effect"]["stat"]["key"] != "def.00000000000033a2"
        || effect["effect"]["entity"] != "current"
    {
        return Err("exact recipient Speed effect".into());
    }
    let stack = app["stacking"]
        .as_array_mut()
        .ok_or("stacking")?
        .pop()
        .ok_or("Speed stacking")?;
    if stack
        != json!({"effect":"speed-increase","family":"pain-offering-buff","modifier":"attack-cast-speed-increase","reduction":"maximum"})
    {
        return Err("independent Speed maximum".into());
    }
    if inverse != dependencies["prior_application_registry"] {
        return Err("prior application inverse".into());
    }
    Ok(())
}
pub fn review_definition(definition: &Value, table: &Value) -> Result<(), String> {
    if definition["effect_id"] != "PainOfferingPlayer"
        || definition["physical_effect_identity"] != true
        || definition["source_tables_preserved"] != true
    {
        return Err("Offering definition identity".into());
    }
    let sets = definition["stat_sets"]
        .as_array()
        .ok_or("source stat sets")?;
    if sets.len() != 1 || sets[0]["stats"][0] != "pain_offering_attack_and_cast_speed_+%" {
        return Err("exact Speed stat position".into());
    }
    if definition["quality_stats"] != json!([["active_skill_base_area_of_effect_radius", 1.2, {}]])
        || definition["alternate_quality_stats"]
            != json!([["pain_offering_attack_and_cast_speed_+%", 1, {}]])
    {
        return Err("ordinary versus alternate quality".into());
    }
    let levels = sets[0]["levels"].as_array().ok_or("level table")?;
    let entries = table["rows"].as_array().ok_or("authored table")?;
    if levels.len() != 40
        || entries.len() != 40
        || table["minimum"] != 1
        || table["maximum"] != 40
        || table["id"] != "pain-offering.speed-increase"
    {
        return Err("exact finite integral level domain".into());
    }
    for (n, (source, authored)) in levels.iter().zip(entries).enumerate() {
        if source["level"] != n + 1
            || source["values"][0].as_i64().is_none()
            || authored["value"]["value"] != source["values"][0]
            || authored["value"]["unit"]["key"] != "def.0000000000000002"
        {
            return Err(format!("Speed table row {}", n + 1));
        }
    }
    Ok(())
}
pub fn reconstruct_vectors(report: &Value) -> Value {
    let original = report["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "original-05")
        .unwrap();
    let mut vectors = vec![];
    for case in report["cases"].as_array().unwrap() {
        for mode in ["main", "calcs"] {
            let Some(state) = case["state"].get(mode) else {
                continue;
            };
            let events = rows(&state["offering_merge_events"]);
            let mut passes = vec![];
            for actor in rows(&state["actors"]) {
                if actor["actor_profile"] != "RaisedSkeletonSniper" {
                    continue;
                }
                for child in rows(&actor["children"]) {
                    if child["effect_id"] != "MinionMeleeBow" {
                        continue;
                    }
                    let Some(observed) = child.get("passes") else {
                        continue;
                    };
                    for pass in rows(observed) {
                        passes.push(json!({"source_occurrence":actor["source_occurrence"],"profile":actor["actor_profile"],"label":pass["label"],"source":pass["inputs"]["source"],"cfg":pass["inputs"]["cfg"],"skill_data":pass["inputs"]["skill_data"],"speed":pass["output"]["Speed"],"time":pass["output"]["Time"],"cast_rate":pass["output"]["CastRate"],"repeats":pass["output"]["Repeats"]}));
                    }
                }
            }
            if !events.is_empty() || !passes.is_empty() {
                vectors.push(json!({"case":case["name"],"mode":mode,"offering_merge_events":events,"basic_passes":passes}));
            }
        }
    }
    json!({"schema_version":1,"kind":"retained-offering-speed-and-ordinary-rate-observations","source_definition":original["state"]["offering_definition"],"vectors":vectors,"scope":{"source_tables":true,"actual_application_records":true,"actual_final_rate_controls":true,"complete_timing_inputs":false,"native_source_closure":false}})
}
pub fn check_authored() {
    let authoring: Value = read("authoring.json");
    let mut names: Vec<_> = authoring["artifacts"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        vec![
            "applications.json",
            "bindings.json",
            "consumer.json",
            "definitions.json",
            "dependencies.json",
            "migration.json",
            "queries.json",
            "source-evidence.json",
            "source-vectors.json",
            "tables.json",
            "timing-obligations.json",
            "timing-source-domain.json"
        ]
    );
    for (name, pin) in authoring["artifacts"].as_object().unwrap() {
        assert_eq!(
            root().join(pin["path"].as_str().unwrap()),
            data().join(name)
        );
        authenticated(pin, 2 * 1024 * 1024);
    }
    let dependencies: Value = read("dependencies.json");
    let previous: Value = serde_json::from_slice(&authenticated(
        &dependencies["prior_application_file"],
        128 * 1024,
    ))
    .unwrap();
    assert_eq!(previous, dependencies["prior_application_registry"]);
    let apps = applications();
    assert_eq!(apps.members.len(), 1);
    assert!(!apps.is_complete());
    check_application(&read::<Value>("applications.json")).unwrap();
    let vectors: Value = read("source-vectors.json");
    assert_eq!(vectors["vectors"].as_array().unwrap().len(), 65);
    assert!(
        reconstruct_vectors(source_report()) == vectors,
        "exact retained source projection"
    );
    let evidence: Value = read("source-evidence.json");
    let manifest_raw = expanded(
        &authenticated(&evidence["source_manifest_archive"], 128 * 1024),
        512 * 1024,
    );
    assert_eq!(hash(&manifest_raw), evidence["source_manifest"]);
    assert_eq!(
        hash(&manifest_raw),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let manifest: Value = serde_json::from_slice(&manifest_raw).unwrap();
    assert_eq!(manifest["upstream_revision"], evidence["source_revision"]);
    let expected_paths = [
        "src/Modules/CalcActiveSkill.lua",
        "src/Modules/CalcPerform.lua",
        "src/Modules/CalcOffence.lua",
        "src/Modules/Data.lua",
        "src/Classes/ModStore.lua",
        "src/Modules/CalcDefence.lua",
        "src/Modules/CalcSetup.lua",
    ];
    let bodies = evidence["bodies"].as_array().unwrap();
    assert_eq!(bodies.len(), expected_paths.len());
    for (body, path) in bodies.iter().zip(expected_paths) {
        assert_eq!(body["source_path"], path);
        let raw = expanded(
            &authenticated(&body["archive"], 512 * 1024),
            2 * 1024 * 1024,
        );
        assert_eq!(raw.len(), body["bytes"]);
        assert_eq!(hash(&raw), body["sha256"]);
        let authority: Vec<_> = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["path"] == path)
            .collect();
        assert_eq!(authority.len(), 1);
        assert_eq!(body["sha256"], authority[0]["sha256"]);
    }
    let t: Value = read("tables.json");
    review_definition(&vectors["source_definition"], &t[0]).unwrap();
    assert_eq!(tables().len(), 1);
    assert_eq!(definitions().len(), 2);
    let q = queries();
    assert_eq!(q.len(), 1);
    assert_eq!(q[0].groups[0].members.members.len(), 1);
    assert!(q[0].groups[0].members.is_complete());
    assert_eq!(q[0].groups[0].ordering, ContributionOrdering::Ordered);
    // No successful source projection substitutes for this admission contract.
    let pending: Value = read("timing-obligations.json");
    assert_eq!(pending["inputs"].as_array().unwrap().len(), 8);
    assert_eq!(pending["no_neutral_literals_from_output"], true);
    super::source_domain::check();
}

pub fn writer_census(rules: &Value, keys: &[&str]) -> Value {
    let touches = |program: &Value| {
        program["effects"].as_array().unwrap().iter().any(|effect| {
            effect["effect"]["stat"]["key"]
                .as_str()
                .is_some_and(|key| keys.contains(&key))
        })
    };
    let mut writers = vec![];
    for owner in rules["owners"].as_array().unwrap() {
        for program in owner["programs"]["members"].as_array().unwrap() {
            if touches(program) {
                writers.push(json!({"kind":"owner","owner":owner["owner"],"program":program}));
            }
        }
    }
    for application in rules["effect_applications"]["members"].as_array().unwrap() {
        if touches(&application["program"]) {
            writers.push(json!({"kind":"application","application":application}));
        }
    }
    json!(writers)
}

/// The new channels cannot acquire extra direct or application writers, even
/// when they are inactive, zero-valued or unselected in the current request.
pub fn check_native(schema: &Value, rules: &Value, routes: &Value, after: bool) {
    let dependencies: Value = read("dependencies.json");
    let authored = consumer();
    let mut prior = rules.clone();
    let owner = prior["owners"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|o| o["owner"] == json!(authored.owners[0].owner))
        .unwrap();
    if after {
        let programs = owner["programs"]["members"].as_array_mut().unwrap();
        let positions: Vec<_> = programs
            .iter()
            .enumerate()
            .filter_map(|(n, p)| (p["id"] == PROGRAM).then_some(n))
            .collect();
        assert_eq!(positions.len(), 1);
        let actual: RuleProgram = serde_json::from_value(programs.remove(positions[0])).unwrap();
        assert_eq!(actual, authored.owners[0].programs.members[0]);
    }
    let actual_owner: DefinitionRules = serde_json::from_value(owner.clone()).unwrap();
    let expected_owner: DefinitionRules =
        serde_json::from_value(dependencies["owner_before"].clone()).unwrap();
    assert_eq!(actual_owner, expected_owner);
    let actual_apps: DeclaredSet<EffectApplicationRule> =
        serde_json::from_value(prior["effect_applications"].clone()).unwrap();
    let before_apps: DeclaredSet<EffectApplicationRule> =
        serde_json::from_value(dependencies["prior_application_registry"].clone()).unwrap();
    assert_eq!(
        actual_apps,
        if after {
            applications()
        } else {
            before_apps.clone()
        }
    );
    prior["effect_applications"] = dependencies["prior_application_registry"].clone();
    assert_eq!(
        writer_census(&prior, &["def.00000000000033a2", "def.00000000000033a3"]),
        json!([])
    );
    assert_eq!(
        writer_census(&prior, &["def.0000000000003211", "def.0000000000003214"]),
        dependencies["intrinsic_rate_writers"]
    );
    assert!(
        writer_census(
            &prior,
            &[
                "def.0000000000003227",
                "def.0000000000003228",
                "def.0000000000003229",
                "def.000000000000322a",
                "def.000000000000322b",
                "def.000000000000322c",
            ]
        ) == dependencies["application_input_writers"],
        "exact activation and scaling potential-writer census"
    );
    let input_queries = dependencies["application_input_queries"]
        .as_array()
        .unwrap();
    for expected in input_queries {
        let actual: Vec<_> = prior["contribution_queries"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|q| q["id"] == expected["id"])
            .collect();
        assert_eq!(actual.len(), 1);
        assert!(actual[0] == expected, "exact application-input query");
    }
    let channel_keys: Vec<_> = input_queries
        .iter()
        .map(|q| q["stat"]["key"].as_str().unwrap())
        .collect();
    assert!(
        writer_census(&prior, &channel_keys) == dependencies["application_input_query_writers"],
        "exact upstream application channel census"
    );
    for expected in dependencies["source_receiver_owners"].as_array().unwrap() {
        let matches: Vec<_> = prior["owners"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| o["owner"] == expected["owner"])
            .collect();
        assert_eq!(matches.len(), 1);
        let a: DefinitionRules = serde_json::from_value(matches[0].clone()).unwrap();
        let b: DefinitionRules = serde_json::from_value(expected.clone()).unwrap();
        assert_eq!(a, b);
    }
    for (actual, expected) in [
        ("definitions", "source_receiver_definitions"),
        ("slots", "source_receiver_slots"),
    ] {
        for descriptor in dependencies[expected].as_array().unwrap() {
            assert!(schema[actual].as_array().unwrap().contains(descriptor));
        }
    }
    for descriptor in dependencies["definitions"].as_array().unwrap() {
        assert!(
            schema["definitions"]
                .as_array()
                .unwrap()
                .contains(descriptor)
        );
    }
    let actual_routes: Vec<_> = routes
        .as_array()
        .unwrap()
        .iter()
        .filter(|output| {
            output["routes"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["target"]["key"] == "def.0000000000003214")
        })
        .collect();
    assert_eq!(json!(actual_routes), dependencies["intrinsic_routes"]);
    assert_eq!(
        prior["existing_actor_rules"],
        dependencies["existing_actor_rules"]
    );
    assert_eq!(
        prior["contribution_queries"]["closure"],
        dependencies["remaining_query_registry_closure"]
    );
    assert_eq!(
        prior["receivers"]["closure"],
        dependencies["remaining_receiver_registry_closure"]
    );
}

pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let recipe = &endpoint.input().recipe;
    for descriptor in definitions() {
        assert!(recipe.schema.definitions.contains(&descriptor));
    }
    check_native(
        &json!(recipe.schema),
        &json!(recipe.rules),
        &json!(recipe.routing.outputs),
        true,
    );
    for table in tables() {
        assert!(recipe.rules.tables.contains(&table));
    }
    for query in queries() {
        assert!(
            recipe
                .rules
                .contribution_queries
                .as_ref()
                .unwrap()
                .members
                .contains(&query)
        );
    }
    assert!(endpoint.evaluation().is_none());
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let authoring: Value = read("authoring.json");
    assert_eq!(
        authoring["scope"]["publication_ready"], true,
        "eight-input source admission is unfinished"
    );
    assert_eq!(authoring["scope"]["complete_timing_authority"], true);
    assert_eq!(authoring["before"], json!(prior.receipt().input));
    assert_eq!(prior.input().recipe.registry.last_issued.get(), 0x33a1);
    let recipe = &prior.input().recipe;
    check_native(
        &json!(recipe.schema),
        &json!(recipe.rules),
        &json!(recipe.routing.outputs),
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
    input.recipe.rules.effect_applications = Some(applications());
    input
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(queries());
    let commitment = poe_optimizer_core::owned_content::digest_owned(
        "offering-speed-rate",
        &authoring,
        2 * 1024 * 1024,
    )
    .unwrap();
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: "offering-speed-rate".parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: commitment,
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
    for query in queries().into_iter().rev() {
        assert_eq!(
            inverse
                .recipe
                .rules
                .contribution_queries
                .as_mut()
                .unwrap()
                .members
                .pop(),
            Some(query)
        );
    }
    inverse.recipe.rules.effect_applications =
        migrated.input().recipe.rules.effect_applications.clone();
    inverse.provenance = migrated.input().provenance.clone();
    assert!(
        inverse == *migrated.input(),
        "exact authored successor inverse"
    );
    super::migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    next
}
