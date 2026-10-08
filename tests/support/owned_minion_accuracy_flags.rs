//! One offline Boolean cutover. Historical integers are proof inputs, never runtime alternatives.
#[allow(dead_code)]
#[path = "owned_release_migration_preservation.rs"]
pub mod preservation;
#[path = "minion_accuracy_source_vectors.rs"]
mod source_vectors;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_rules::{ContributionQuery, DefinitionRules, RuleProgram},
    owned_schema::{DefinitionDescriptor, SchemaSubject},
};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_revision::{OwnedReleaseRevisionInput, compile_owned_release_revision},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub const KIND: &str = "minion-accuracy-flags-v1";
pub const QUERIES: [&str; 2] = [
    "minion-accuracy-inheritance-flags",
    "minion-accuracy-cannot-block-flags",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/minion-accuracy-flags")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn read_path<T: DeserializeOwned>(path: &str) -> T {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}
fn key(text: &str) -> OwnedDefinitionKey {
    text.parse().unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgramReplacement {
    pub owner: SchemaSubject,
    pub program: RuleProgram,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Programs {
    schema_version: u32,
    programs: Vec<ProgramReplacement>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pin {
    path: String,
    bytes: usize,
    sha256: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependencies {
    schema_version: u32,
    prior_input: OwnedContentDigest,
    prior_schema_release: OwnedDefinitionKey,
    prior_rules_release: OwnedDefinitionKey,
    pins: Vec<Pin>,
}
pub fn programs() -> Vec<ProgramReplacement> {
    let packet: Programs = read("programs.json");
    assert_eq!(packet.schema_version, 1);
    packet.programs
}
pub fn schemas() -> Vec<DefinitionDescriptor> {
    read("schemas.json")
}
pub fn queries() -> Vec<ContributionQuery> {
    read("queries.json")
}
fn old_extension() -> Value {
    read_path("data/owned/poe2/3887ae68/minion-accuracy/extension.json")
}
fn old_programs() -> Vec<ProgramReplacement> {
    old_extension()["owners"]
        .as_array()
        .unwrap()
        .iter()
        .map(|owner| {
            assert_eq!(owner["programs"]["members"].as_array().unwrap().len(), 1);
            serde_json::from_value(json!({
                "owner":owner["owner"], "program":owner["programs"]["members"][0]
            }))
            .unwrap()
        })
        .collect()
}
fn old_schemas() -> Vec<DefinitionDescriptor> {
    old_extension()["schema"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            matches!(
                row["value"]["value"]["id"]["key"].as_str(),
                Some("def.0000000000003219" | "def.000000000000321e")
            )
        })
        .map(|row| serde_json::from_value(row["value"].clone()).unwrap())
        .collect()
}

/// Verify only the intended integer-presence representation changes; all numeric
/// arithmetic, recipient identities, ordering and output channels remain exact.
pub fn check_replacements(rows: &[ProgramReplacement]) {
    assert_eq!(rows.len(), 2);
    for (index, (old, new)) in old_programs().iter().zip(rows).enumerate() {
        assert_eq!(old.owner, new.owner);
        let mut expected = json!(old.program);
        let read_id = ["inheritance-flags", "cannot-block-flags"][index];
        let read = expected["reads"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["id"] == read_id)
            .unwrap();
        read["value_type"] = json!({"kind":"boolean"});
        read["source"] = json!({"kind":"contribution_query","value":{
            "entity":(["player","enemy"][index]),"query":QUERIES[index],"group":"sources"
        }});
        let remove = if index == 0 {
            ["zero", "valid-flags", "ordinary-branch"]
        } else {
            ["zero-flags", "valid-flags", "cannot-block"]
        };
        expected["nodes"]
            .as_array_mut()
            .unwrap()
            .retain(|n| !remove.contains(&n["id"].as_str().unwrap()));
        let nodes = expected["nodes"].as_array_mut().unwrap();
        if index == 0 {
            nodes
                .iter_mut()
                .find(|n| n["id"] == "no-inheritance")
                .unwrap()["expression"] = json!({"kind":"not","value":"flags"});
            expected["effects"][0]["when"] = Value::Null;
            expected["effects"][1]["when"] = json!("no-inheritance");
        } else {
            nodes
                .iter_mut()
                .find(|n| n["id"] == "effective-block")
                .unwrap()["expression"]["condition"] = json!("flags");
            for effect in expected["effects"].as_array_mut().unwrap() {
                effect["when"] = Value::Null;
            }
        }
        let expected: RuleProgram = serde_json::from_value(expected).unwrap();
        assert_eq!(new.program, expected);
    }
}

pub fn check_authored() {
    let dependencies: Dependencies = read("dependencies.json");
    let authoring: Value = read("authoring.json");
    assert_eq!(dependencies.schema_version, 1);
    assert_eq!(authoring["schema_version"], 1);
    assert_eq!(authoring["kind"], KIND);
    assert_eq!(authoring["before"], json!(dependencies.prior_input));
    assert_eq!(
        authoring["scope"],
        json!({
            "schema_replacements":2,"replaced_programs":2,"new_queries":2,
            "new_definitions":0,"new_programs":0,"new_producers":0,
            "closed_rule_owners":0,"closed_query_memberships":0,
            "evaluation_bundle_added":false,"whole_build_parity":false
        })
    );
    assert_eq!(dependencies.pins.len(), 4);
    for pin in &dependencies.pins {
        let bytes = fs::read(root().join(&pin.path)).unwrap();
        assert_eq!(bytes.len(), pin.bytes);
        assert_eq!(hash(&bytes), pin.sha256);
    }
    let artifacts = authoring["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), 4);
    for (pin, name) in artifacts.iter().zip([
        "schemas.json",
        "programs.json",
        "queries.json",
        "dependencies.json",
    ]) {
        assert_eq!(pin["file"], name);
        let bytes = fs::read(data().join(name)).unwrap();
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], hash(&bytes));
    }
    let schemas = schemas();
    assert_eq!(schemas.len(), 2);
    for (old, new) in old_schemas().into_iter().zip(&schemas) {
        let mut expected = json!(old);
        assert_eq!(
            expected["value"]["schema"]["value"]["value"],
            json!({"kind":"integer"})
        );
        expected["value"]["schema"]["value"]["value"] = json!({"kind":"boolean"});
        assert_eq!(json!(new), expected);
    }
    check_replacements(&programs());
    let queries = queries();
    assert_eq!(queries.len(), 2);
    for (index, query) in queries.iter().enumerate() {
        let stat = json!(schemas[index])["value"]["id"].clone();
        assert_eq!(
            json!(query),
            json!({
                "id":QUERIES[index],"stat":stat,"contribution":"flag",
                "groups":[{"id":"sources","reduction":"any","empty":{"kind":"boolean","value":false},
                    "members":{"members":[],"closure":{"kind":"partial","value":{"gaps":[{
                        "subject":{"kind":"definition","value":{"kind":"stat","value":stat}},
                        "facet":"game_rules","code":"accuracy-flag-producer-domain-unproved"
                    }]}}},"ordering":"unordered"}]
            })
        );
    }
    checked_source_vectors();
}

/// Existing measured values only. This does not recalculate expected hit chance.
pub fn checked_source_vectors() -> Vec<Value> {
    let authoring: Value = read_path("data/owned/poe2/3887ae68/minion-accuracy/authoring.json");
    let measured: Value = read_path("tests/fixtures/calibration/minion-accuracy-3887ae68.json");
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(measured["source_hash"], hash(&bytes));
    assert_eq!(measured["source_revision"], manifest["upstream_revision"]);
    assert_eq!(authoring["source_revision"], measured["source_revision"]);
    assert_eq!(authoring["source_manifest_sha256"], measured["source_hash"]);
    for pin in authoring["source_files"].as_array().unwrap() {
        assert!(manifest["files"].as_array().unwrap().contains(pin));
    }
    for pin in measured["source_files"].as_array().unwrap() {
        let candidates: Vec<_> = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|actual| actual["path"] == pin["path"])
            .collect();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0]["sha256"], pin["sha256"]);
    }
    let observer = fs::read(
        root().join("crates/poe-optimizer-pob/tests/support/owned_minion_accuracy_source.lua"),
    )
    .unwrap();
    assert_eq!(measured["observer_sha256"], hash(&observer));
    for original in measured["originals"].as_array().unwrap() {
        let bytes = fs::read(
            root()
                .join("tests/fixtures/builds/breadth-20260908")
                .join(original["name"].as_str().unwrap()),
        )
        .unwrap();
        assert_eq!(original["sha256"], hash(&bytes));
    }
    assert_eq!(measured["observed_counts"]["sniper_consumers"], 13);
    let rows = measured["sniper"].as_array().unwrap();
    assert_eq!(rows.len(), 13);
    for row in rows {
        let passes = row["consumer"]["passes"].as_array().unwrap();
        assert_eq!(passes.len(), 1);
        assert_eq!(
            passes[0]["flags"]["player_minion_accuracy_equals_accuracy"],
            false
        );
    }
    rows.clone()
}
pub fn authenticate_source() {
    check_authored();
    let authoring: Value = read_path("data/owned/poe2/3887ae68/minion-accuracy/authoring.json");
    let proof = &authoring["source_validation"];
    assert_eq!(proof["status"], "passed");
    let off = fs::read(root().join(proof["evidence_json"].as_str().unwrap())).unwrap();
    let on = fs::read(root().join(proof["evidence_on_json"].as_str().unwrap())).unwrap();
    assert_eq!(off, on);
    assert_eq!(off.len() as u64, proof["evidence_bytes"].as_u64().unwrap());
    assert_eq!(hash(&off), proof["evidence_sha256"]);
    let source: Value = serde_json::from_slice(&off).unwrap();
    assert_eq!(
        source_vectors::project(&source),
        read_path::<Value>("tests/fixtures/calibration/minion-accuracy-3887ae68.json")
    );
}

fn provenance() -> OwnedReleaseProvenance {
    let dependencies: Dependencies = read("dependencies.json");
    OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: dependencies.prior_input,
        authoring_input: digest_owned(
            "owned-minion-accuracy-flags-v1",
            &(
                read::<Value>("authoring.json"),
                dependencies,
                schemas(),
                programs(),
                queries(),
            ),
            1024 * 1024,
        )
        .unwrap(),
    }
}
fn find_owner<'a>(owners: &'a [DefinitionRules], subject: &SchemaSubject) -> &'a DefinitionRules {
    let found: Vec<_> = owners.iter().filter(|row| &row.owner == subject).collect();
    assert_eq!(found.len(), 1);
    found[0]
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let input = endpoint.input();
    assert_eq!(
        input
            .provenance
            .iter()
            .filter(|p| p.kind == key(KIND))
            .collect::<Vec<_>>(),
        [&provenance()]
    );
    for schema in schemas() {
        assert_eq!(
            input
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|d| d.address() == schema.address())
                .collect::<Vec<_>>(),
            [&schema]
        );
    }
    for replacement in programs() {
        let owner = find_owner(&input.recipe.rules.owners, &replacement.owner);
        assert!(!owner.programs.is_complete());
        assert_eq!(
            owner
                .programs
                .members
                .iter()
                .filter(|p| p.id == replacement.program.id)
                .collect::<Vec<_>>(),
            [&replacement.program]
        );
    }
    let inventory = input.recipe.rules.contribution_queries.as_ref().unwrap();
    assert!(!inventory.is_complete());
    for query in queries() {
        assert_eq!(
            inventory
                .members
                .iter()
                .filter(|q| q.id == query.id)
                .collect::<Vec<_>>(),
            [&query]
        );
    }
}

pub fn assert_transition(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    assert_component(next);
    let dependencies: Dependencies = read("dependencies.json");
    assert_eq!(prior.receipt().input, dependencies.prior_input);
    let mut recipe = next.input().recipe.clone();
    for (old, new) in old_schemas().into_iter().zip(schemas()) {
        let at = recipe
            .schema
            .definitions
            .iter()
            .position(|d| d.address() == new.address())
            .unwrap();
        assert_eq!(recipe.schema.definitions[at], new);
        recipe.schema.definitions[at] = old;
    }
    for (old, new) in old_programs().into_iter().zip(programs()) {
        let owner = recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == new.owner)
            .unwrap();
        let at = owner
            .programs
            .members
            .iter()
            .position(|p| p.id == new.program.id)
            .unwrap();
        assert_eq!(owner.programs.members[at], new.program);
        owner.programs.members[at] = old.program;
    }
    let inventory = recipe.rules.contribution_queries.as_mut().unwrap();
    for query in queries().into_iter().rev() {
        assert_eq!(inventory.members.pop(), Some(query));
    }
    recipe.schema.release = dependencies.prior_schema_release;
    recipe.rules.release = dependencies.prior_rules_release;
    recipe.rules.definitions = prior.input().recipe.rules.definitions.clone();
    recipe.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        recipe,
        prior.input().recipe,
        "exact schema/rule/query inverse"
    );
    preservation::assert_import_rebindings_only(prior, next);
    assert_eq!(next.receipt().query_rows, 110);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert!(next.evaluation().is_none());
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    authenticate_source();
    let dependencies: Dependencies = read("dependencies.json");
    assert_eq!(prior.receipt().input, dependencies.prior_input);
    assert_eq!(
        prior.input().recipe.schema.release,
        dependencies.prior_schema_release
    );
    assert_eq!(
        prior.input().recipe.rules.release,
        dependencies.prior_rules_release
    );
    assert!(prior.evaluation().is_none());
    // Temporary checked Partial owners omit only the two old consumers so the
    // existing schema revision can rebind dependencies. This is never published.
    let mut stripped = prior.input().clone();
    let mut positions = vec![];
    for old in old_programs() {
        let owner = stripped
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == old.owner)
            .unwrap();
        assert!(!owner.programs.is_complete());
        let at = owner
            .programs
            .members
            .iter()
            .position(|p| p.id == old.program.id)
            .unwrap();
        assert_eq!(owner.programs.members.remove(at), old.program);
        positions.push(at);
    }
    let stripped = assemble_owned_release(stripped, Default::default()).unwrap();
    let release: OwnedDefinitionKey =
        serde_json::from_value(read::<Value>("authoring.json")["release"].clone()).unwrap();
    let revision = OwnedReleaseRevisionInput {
        schema_version: 1,
        before: stripped.receipt().input,
        release: release.clone(),
        reason: key(KIND),
        definitions: schemas(),
        slots: vec![],
    };
    let revised = compile_owned_release_revision(&stripped, revision, Default::default()).unwrap();
    let mut input = revised.input().clone();
    for (replacement, at) in programs().into_iter().zip(positions) {
        let owner = input
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == replacement.owner)
            .unwrap();
        owner.programs.members.insert(at, replacement.program);
    }
    let inventory = input.recipe.rules.contribution_queries.as_mut().unwrap();
    assert!(!inventory.is_complete());
    for query in queries() {
        assert!(!inventory.members.iter().any(|q| q.id == query.id));
        inventory.members.push(query);
    }
    input.recipe.rules.release = release;
    assert_eq!(input.provenance.last().unwrap().kind, key(KIND));
    *input.provenance.last_mut().unwrap() = provenance();
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_transition(prior, &next);
    next
}
