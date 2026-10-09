//! Offline representation cutover; the predecessor is proof, never a runtime lane.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_rules::{ContributionQuery, DefinitionRules, RuleProgram, StatReceiver},
    owned_schema::{DefinitionDescriptor, SchemaClosure, SchemaSubject},
};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_revision::{OwnedReleaseRevisionInput, compile_owned_release_revision},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub const KIND: &str = "gigantic-flags-v1";
pub const QUERY: &str = "minion-gigantic-grants";
pub const GROUP: &str = "sources";
const FILES: [&str; 4] = [
    "schemas.json",
    "programs.json",
    "queries.json",
    "dependencies.json",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/gigantic-flags")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn historical(name: &str) -> Value {
    serde_json::from_slice(&fs::read(root().join("data/owned/poe2/3887ae68").join(name)).unwrap())
        .unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
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
    definitions: Vec<DefinitionDescriptor>,
    owners: Vec<DefinitionRules>,
    benefit: ProgramReplacement,
    benefit_owner_closure: SchemaClosure,
    receiver: StatReceiver,
}
pub fn programs() -> Vec<ProgramReplacement> {
    let p: Programs = read("programs.json");
    assert_eq!(p.schema_version, 1);
    p.programs
}
pub fn schemas() -> Vec<DefinitionDescriptor> {
    read("schemas.json")
}
pub fn queries() -> Vec<ContributionQuery> {
    read("queries.json")
}
fn old_schema() -> DefinitionDescriptor {
    serde_json::from_value(
        historical("gigantic-following/migration.json")["schema"][0]["value"].clone(),
    )
    .unwrap()
}
fn old_programs() -> Vec<ProgramReplacement> {
    let closure = historical("gigantic-following/closure.json");
    let migration = historical("gigantic-following/migration.json");
    [&closure["owners"][0], &migration["owners"][0]]
        .into_iter()
        .map(|o| {
            serde_json::from_value(
                json!({"owner":o["owner"],"program":o["programs"]["members"][0]}),
            )
            .unwrap()
        })
        .collect()
}
pub fn check_replacements(rows: &[ProgramReplacement]) {
    let old = old_programs();
    assert_eq!(rows.len(), 2);
    let mut producer = json!(old[0]);
    producer["program"]["nodes"][0]["expression"]["value"] = json!({"kind":"boolean","value":true});
    producer["program"]["effects"][0]["effect"]["contribution"] = json!("flag");
    assert_eq!(json!(rows[0]), producer);
    let mut receiver = json!(old[1]);
    receiver["program"]["reads"][0]["value_type"] = json!({"kind":"boolean"});
    receiver["program"]["reads"][0]["source"] = json!({"kind":"contribution_query","value":{"entity":"player","query":QUERY,"group":GROUP}});
    receiver["program"]["nodes"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    receiver["program"]["effects"][0]["effect"]["value"] = json!("grants");
    assert_eq!(json!(rows[1]), receiver);
}
pub fn check_queries(rows: &[ContributionQuery]) {
    let old = old_programs();
    let stat = json!(old_schema())["value"]["id"].clone();
    assert_eq!(
        json!(rows),
        json!([{"id":QUERY,"stat":stat,"contribution":"flag","groups":[{
            "id":GROUP,"reduction":"any","ordering":"unordered","empty":{"kind":"boolean","value":false},
            "members":{"members":[{"producer":{"kind":"program_effect","owner":old[0].owner,"program":old[0].program.id,"effect":"grant","origin":{"kind":"allocation"}},"order":null}],
            "closure":{"kind":"partial","value":{"gaps":[{"subject":{"kind":"definition","value":{"kind":"stat","value":stat}},"facet":"game_rules","code":"gigantic-flag-producer-domain-unproved"}]}}}
        }]}])
    );
}
pub fn check_authored() {
    let d: Dependencies = read("dependencies.json");
    let a: Value = read("authoring.json");
    assert_eq!(d.schema_version, 1);
    assert_eq!(a["schema_version"], 1);
    assert_eq!(a["kind"], KIND);
    assert_eq!(a["before"], json!(d.prior_input));
    assert_eq!(
        a["scope"],
        json!({"schema_replacements":1,"replaced_programs":2,"new_queries":1,
        "new_definitions":0,"new_programs":0,"new_producers":0,"closed_rule_owners":0,
        "closed_query_memberships":0,"evaluation_bundle_added":false,"whole_build_parity":false})
    );
    assert_eq!(d.pins.len(), 9);
    for p in &d.pins {
        let b = fs::read(root().join(&p.path)).unwrap();
        assert_eq!(b.len(), p.bytes);
        assert_eq!(hash(&b), p.sha256);
    }
    assert_eq!(a["artifacts"].as_array().unwrap().len(), FILES.len());
    for (p, n) in a["artifacts"].as_array().unwrap().iter().zip(FILES) {
        let b = fs::read(data().join(n)).unwrap();
        assert_eq!(p["file"], n);
        assert_eq!(p["bytes"], b.len());
        assert_eq!(p["sha256"], hash(&b));
        assert!(!b.contains(&b'\r'));
    }
    let mut expected = json!(old_schema());
    expected["value"]["schema"]["value"]["value"] = json!({"kind":"boolean"});
    assert_eq!(json!(schemas()), json!([expected]));
    assert_eq!(d.owners.len(), 2);
    let closure = historical("gigantic-following/closure.json");
    let migration = historical("gigantic-following/migration.json");
    assert_eq!(
        json!(d.owners),
        json!([closure["owners"][0], migration["owners"][0]])
    );
    assert!(d.owners.iter().all(|o| o.programs.is_complete()));
    assert_eq!(json!(d.receiver), migration["receivers"][0]);
    assert_eq!(d.definitions.len(), 6);
    assert!(d.definitions.contains(&old_schema()));
    let benefit = historical("gigantic-benefits/migration.json");
    let life = historical("minion-life-source/replacement.json");
    assert_eq!(json!(d.benefit.owner), benefit["owners"][0]["owner"]);
    assert_eq!(
        json!(d.benefit_owner_closure),
        benefit["owners"][0]["programs"]["closure"]
    );
    let mut program = benefit["owners"][0]["programs"]["members"][0].clone();
    assert_eq!(program, life["before"]);
    program["effects"][0]["effect"]["stat"]["key"] = json!("def.000000000000311a");
    assert_eq!(program, life["after"]);
    assert_eq!(json!(d.benefit.program), program);
    check_replacements(&programs());
    check_queries(&queries());
    check_source(false);
}

// Authentication of retained source projections, never an alternative evaluator.
fn check_source(full: bool) {
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    for name in ["gigantic-following", "gigantic-benefits"] {
        let a = historical(&format!("{name}/authoring.json"));
        let v = historical(&format!("{name}/source-vectors.json"));
        assert_eq!(a["source_manifest_sha256"], hash(&manifest_bytes));
        assert_eq!(a["source_revision"], manifest["upstream_revision"]);
        assert_eq!(v["status"], "passed");
        for p in a["source_files"].as_array().unwrap() {
            assert!(manifest["files"].as_array().unwrap().contains(p));
        }
        for (n, digest) in a["artifact_sha256"].as_object().unwrap() {
            let b =
                fs::read(root().join(format!("data/owned/poe2/3887ae68/{name}/{n}.json"))).unwrap();
            assert_eq!(*digest, hash(&b));
        }
        let reports = v["reports"].as_array().unwrap();
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0]["observations"], reports[1]["observations"]);
        assert_eq!(reports[0]["sha256"], reports[1]["sha256"]);
        for report in reports {
            if full {
                let bytes = fs::read(root().join(report["path"].as_str().unwrap())).unwrap();
                assert_eq!(report["bytes"], bytes.len());
                assert_eq!(report["sha256"], hash(&bytes));
                let actual: Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(actual["source_revision"], a["source_revision"]);
                for row in report["observations"].as_array().unwrap() {
                    for p in row["projections"].as_array().unwrap() {
                        assert_eq!(
                            actual.pointer(p["pointer"].as_str().unwrap()),
                            Some(&p["value"])
                        );
                    }
                }
            }
        }
        if name == "gigantic-following" {
            let observations = reports[0]["observations"].as_array().unwrap();
            assert_eq!(observations.len(), 7);
            for row in observations {
                let ps = row["projections"].as_array().unwrap();
                assert_eq!(ps.len(), 10);
                let removed = row["case"] == "without-gigantic";
                assert_eq!(ps[6]["value"], !removed);
                if !removed {
                    assert_eq!(ps[7]["value"][0]["mod"]["name"], "Gigantic");
                    assert_eq!(ps[7]["value"][0]["mod"]["type"], "FLAG");
                    assert_eq!(ps[7]["value"][0]["value"], true);
                }
            }
            if full {
                let report = &v["default_report"];
                let bytes = fs::read(root().join(report["path"].as_str().unwrap())).unwrap();
                assert_eq!(report["bytes"], bytes.len());
                assert_eq!(report["sha256"], hash(&bytes));
                let actual: Value = serde_json::from_slice(&bytes).unwrap();
                for p in
                    std::iter::once(&report["default"]).chain(report["parser"].as_array().unwrap())
                {
                    assert_eq!(
                        actual.pointer(p["pointer"].as_str().unwrap()),
                        Some(&p["value"])
                    );
                }
            }
        }
    }
}
pub fn authenticate_source() {
    check_authored();
    check_source(true);
}
/// Exact bytes of retained original and node-removal controls; not native inputs.
pub fn source_xml_hashes() -> [String; 2] {
    authenticate_source();
    let v = historical("gigantic-benefits/source-vectors.json");
    let actual: Value = serde_json::from_slice(
        &fs::read(root().join(v["reports"][0]["path"].as_str().unwrap())).unwrap(),
    )
    .unwrap();
    assert_eq!(actual["cases"][0]["name"], "original-05");
    assert_eq!(actual["cases"][2]["name"], "without-gigantic");
    [0, 2].map(|i| {
        actual["cases"][i]["xml_sha256"]
            .as_str()
            .unwrap()
            .to_owned()
    })
}

fn provenance() -> OwnedReleaseProvenance {
    let d: Dependencies = read("dependencies.json");
    OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: d.prior_input,
        authoring_input: digest_owned(
            "owned-gigantic-flags-v1",
            &(
                read::<Value>("authoring.json"),
                d,
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
    let found: Vec<_> = owners.iter().filter(|o| &o.owner == subject).collect();
    assert_eq!(found.len(), 1);
    found[0]
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let d: Dependencies = read("dependencies.json");
    let input = endpoint.input();
    assert_eq!(
        input
            .provenance
            .iter()
            .filter(|p| p.kind == key(KIND))
            .collect::<Vec<_>>(),
        [&provenance()]
    );
    for old in &d.definitions {
        let expected = if old.address() == old_schema().address() {
            schemas()[0].clone()
        } else {
            old.clone()
        };
        assert_eq!(
            input
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| x.address() == expected.address())
                .collect::<Vec<_>>(),
            [&expected]
        );
    }
    for (mut owner, replacement) in d.owners.into_iter().zip(programs()) {
        assert_eq!(owner.owner, replacement.owner);
        let at = owner
            .programs
            .members
            .iter()
            .position(|p| p.id == replacement.program.id)
            .unwrap();
        owner.programs.members[at] = replacement.program;
        assert_eq!(find_owner(&input.recipe.rules.owners, &owner.owner), &owner);
    }
    let owner = find_owner(&input.recipe.rules.owners, &d.benefit.owner);
    assert_eq!(owner.programs.closure, d.benefit_owner_closure);
    assert_eq!(
        owner
            .programs
            .members
            .iter()
            .filter(|p| p.id == d.benefit.program.id)
            .collect::<Vec<_>>(),
        [&d.benefit.program]
    );
    assert_eq!(
        input
            .recipe
            .rules
            .receivers
            .members
            .iter()
            .filter(|r| r.id == d.receiver.id)
            .collect::<Vec<_>>(),
        [&d.receiver]
    );
    let inv = input.recipe.rules.contribution_queries.as_ref().unwrap();
    assert!(!inv.is_complete());
    assert_eq!(
        inv.members
            .iter()
            .filter(|q| q.id == key(QUERY))
            .cloned()
            .collect::<Vec<_>>(),
        queries()
    );
}
pub fn assert_transition(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    assert_component(next);
    let d: Dependencies = read("dependencies.json");
    assert_eq!(prior.receipt().input, d.prior_input);
    let mut recipe = next.input().recipe.clone();
    let at = recipe
        .schema
        .definitions
        .iter()
        .position(|x| x.address() == old_schema().address())
        .unwrap();
    assert_eq!(recipe.schema.definitions[at], schemas()[0]);
    recipe.schema.definitions[at] = old_schema();
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
    assert_eq!(
        recipe
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members
            .pop(),
        Some(queries().remove(0))
    );
    recipe.schema.release = d.prior_schema_release;
    recipe.rules.release = d.prior_rules_release;
    recipe.rules.definitions = prior.input().recipe.rules.definitions.clone();
    recipe.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        recipe,
        prior.input().recipe,
        "exact schema/program/query inverse"
    );
    migration_preservation::assert_import_rebindings_only(prior, next);
    assert_eq!(next.receipt().query_rows, 110);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert!(next.evaluation().is_none());
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    authenticate_source();
    let d: Dependencies = read("dependencies.json");
    assert_eq!(prior.receipt().input, d.prior_input);
    assert_eq!(prior.input().recipe.schema.release, d.prior_schema_release);
    assert_eq!(prior.input().recipe.rules.release, d.prior_rules_release);
    assert!(prior.evaluation().is_none());
    // Unpublished transaction intermediates omit the old bodies solely to rebind
    // the corrected schema. Exact owner closures and positions are restored.
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
        assert_eq!(owner, find_owner(&d.owners, &old.owner));
        let at = owner
            .programs
            .members
            .iter()
            .position(|p| p.id == old.program.id)
            .unwrap();
        assert_eq!(owner.programs.members.remove(at), old.program);
        positions.push(at);
    }
    let receiver_position = stripped
        .recipe
        .rules
        .receivers
        .members
        .iter()
        .position(|r| r.id == d.receiver.id)
        .unwrap();
    assert_eq!(
        stripped
            .recipe
            .rules
            .receivers
            .members
            .remove(receiver_position),
        d.receiver
    );
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
    for (new, at) in programs().into_iter().zip(positions) {
        input
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == new.owner)
            .unwrap()
            .programs
            .members
            .insert(at, new.program);
    }
    input
        .recipe
        .rules
        .receivers
        .members
        .insert(receiver_position, d.receiver);
    let inv = input.recipe.rules.contribution_queries.as_mut().unwrap();
    assert!(!inv.is_complete());
    for q in queries() {
        assert!(!inv.members.iter().any(|old| old.id == q.id));
        inv.members.push(q);
    }
    input.recipe.rules.release = release;
    assert_eq!(input.provenance.last().unwrap().kind, key(KIND));
    *input.provenance.last_mut().unwrap() = provenance();
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_transition(prior, &next);
    next
}
