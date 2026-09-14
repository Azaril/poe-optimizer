//! The host consumes only caller-owned request and schema files from an empty directory.
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::*;
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("cli-authored", "v9").unwrap()
}
fn def<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(ns(), key).unwrap()
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn input() -> OwnedDocument {
    let limits = OwnedInputLimits::default();
    let lineage = BuildLineage::from_bytes([47; 16]);
    let loadout = WeaponLoadoutId::from_instance_id(InstanceId::from_parts(lineage, 1).unwrap());
    OwnedDocument::Request(Box::new(
        OwnedEvaluationRequest::new(
            BuildSpec::new(
                BuildInput {
                    allocator: InstanceAllocatorState::from_parts(lineage, 1),
                    revision: BuildRevision::from_u64(1),
                    game_version: ns(),
                    character: CharacterSpec {
                        class: def("class"),
                        ascendancy: None,
                        level: 50,
                        rewards: vec![],
                    },
                    weapon_loadouts: vec![loadout],
                    active_weapon_loadout: loadout,
                    items: vec![],
                    gems: vec![],
                    equipment: vec![],
                    allocations: vec![],
                    skills: vec![],
                    supports: vec![],
                    payload_links: vec![],
                    choices: vec![],
                },
                limits,
            )
            .unwrap(),
            ScenarioSpec::new(
                ScenarioInput {
                    game_version: ns(),
                    enemy: EnemySpec {
                        encounter: def("encounter"),
                        level: 50,
                    },
                    assumptions: vec![],
                    usage: vec![],
                },
                limits,
            )
            .unwrap(),
            QuerySpec::new(
                QueryInput {
                    game_version: ns(),
                    requests: vec![MetricRequest {
                        id: QueryId::new("player").unwrap(),
                        metric: def("metric"),
                        target: MetricTarget::Actor(ActorKey::Player),
                    }],
                },
                limits,
            )
            .unwrap(),
            limits,
        )
        .unwrap(),
    ))
}
fn known<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn schema(maximum: i64) -> OwnedDefinitionSchemaPackage {
    let range = || IntegerRange {
        minimum: BoundedInteger::new(1).unwrap(),
        maximum: BoundedInteger::new(maximum).unwrap(),
    };
    OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: 1,
            namespace: ns(),
            release: OwnedDefinitionKey::new("test").unwrap(),
            semantics_version: OwnedDefinitionKey::new("v1").unwrap(),
            definitions: vec![
                DefinitionDescriptor::Class(known(
                    def("class"),
                    ClassSchema {
                        level: range(),
                        ascendancies: empty(),
                        declarations: DeclaredSlots {
                            parameters: empty(),
                            choices: empty(),
                            grants: empty(),
                            actors: empty(),
                            skill_grants: empty(),
                            outputs: empty(),
                            sockets: empty(),
                        },
                    },
                )),
                DefinitionDescriptor::Encounter(known(
                    def("encounter"),
                    EncounterSchema {
                        enemy_level: range(),
                        external_inputs: empty(),
                    },
                )),
                DefinitionDescriptor::Unit(known(
                    def("unit"),
                    UnitSchema {
                        dimension: UnitDimension::Count,
                    },
                )),
                DefinitionDescriptor::Metric(known(
                    def("metric"),
                    MetricSchema {
                        targets: vec![MetricTargetKind::Actor],
                        unit: def("unit"),
                        actor_roles: vec![MetricActorRole::Player],
                        provider_roles: vec![ProviderRole::Character],
                    },
                )),
            ],
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap()
}
fn write_inputs(dir: &Path, maximum: i64) {
    fs::write(
        dir.join("request.json"),
        encode_owned(&input(), OwnedInputLimits::default()).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("schema.json"),
        encode_schema_package(&schema(maximum), OwnedSchemaLimits::default()).unwrap(),
    )
    .unwrap();
}
fn run(dir: &Path, more: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(dir)
        .args([
            "bind-owned-input",
            "request.json",
            "--schema",
            "schema.json",
        ])
        .args(more)
        .output()
        .unwrap()
}
fn report(out: Output) -> Value {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
#[test]
fn binding_uses_injected_schema_and_keeps_validation_separate_from_evaluation() {
    let dir = tempfile::tempdir().unwrap();
    write_inputs(dir.path(), 100);
    let first = report(run(dir.path(), &["--output", "report.json"]));
    assert_eq!(first["document_kind"], "definition_binding_report");
    assert_eq!(
        first["verification"],
        json!({"legality":"not_checked","calculation":"not_run"})
    );
    assert_eq!(first["binding"]["schema"], "valid");
    assert_eq!(first["binding"]["queries"][0]["selector"], "schema_bound");
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(dir.path().join("report.json")).unwrap())
            .unwrap(),
        first
    );
    write_inputs(dir.path(), 40);
    let changed = report(run(dir.path(), &[]));
    assert_eq!(changed["binding"]["schema"], "invalid");
    assert_ne!(
        first["binding"]["data_identity"],
        changed["binding"]["data_identity"]
    );
    assert_eq!(
        first["binding"]["request_digest"],
        changed["binding"]["request_digest"]
    );
    assert!(
        !run(dir.path(), &["--output", "report.json"])
            .status
            .success()
    );
}
#[test]
fn missing_definitions_are_diagnostic_but_malformed_inputs_and_exhausted_budgets_fail() {
    let dir = tempfile::tempdir().unwrap();
    write_inputs(dir.path(), 100);
    let mut package = schema(100).input().clone();
    package
        .definitions
        .retain(|d| !matches!(d, DefinitionDescriptor::Class(_)));
    let package = OwnedDefinitionSchemaPackage::new(package, OwnedSchemaLimits::default()).unwrap();
    fs::write(
        dir.path().join("schema.json"),
        encode_schema_package(&package, OwnedSchemaLimits::default()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        report(run(dir.path(), &[]))["binding"]["schema"],
        "unresolved"
    );
    assert!(
        !run(dir.path(), &["--max-work", "1", "--output", "budget.json"])
            .status
            .success()
    );
    assert!(!dir.path().join("budget.json").exists());
    fs::write(dir.path().join("request.json"), b"not an owned document").unwrap();
    assert!(!run(dir.path(), &["--output", "bad.json"]).status.success());
    assert!(!dir.path().join("bad.json").exists());
}
#[test]
fn standalone_build_does_not_invent_scenario_or_queries() {
    let dir = tempfile::tempdir().unwrap();
    write_inputs(dir.path(), 100);
    let OwnedDocument::Request(request) = input() else {
        panic!()
    };
    fs::write(
        dir.path().join("request.json"),
        encode_owned(
            &OwnedDocument::Build(Box::new(request.build().clone())),
            OwnedInputLimits::default(),
        )
        .unwrap(),
    )
    .unwrap();
    let out = run(dir.path(), &[]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("complete owned request"));
}
