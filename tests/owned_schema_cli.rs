//! Injected schemas are checked from an empty directory, with no PoB or bundled data access.
use poe_optimizer_core::{owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("caller-game", "v3").unwrap()
}
fn id<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(namespace(), key).unwrap()
}
fn input() -> SchemaPackageInput {
    SchemaPackageInput {
        schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
        namespace: namespace(),
        release: OwnedDefinitionKey::new("caller-release").unwrap(),
        semantics_version: OwnedDefinitionKey::new("input-schema-v1").unwrap(),
        definitions: vec![
            DefinitionDescriptor::ExternalInput(DefinitionEntry {
                id: id("encounter-distance"),
                schema: SchemaState::Known(ExternalInputSchema {
                    value: ValueSchema::Quantity(QuantityRange {
                        minimum: FiniteQuantity::new(0.0, id("distance")).unwrap(),
                        maximum: FiniteQuantity::new(50.0, id("distance")).unwrap(),
                    }),
                    targets: vec![AssumptionTargetKind::Enemy],
                }),
            }),
            DefinitionDescriptor::Unit(DefinitionEntry {
                id: id("distance"),
                schema: SchemaState::Known(UnitSchema {
                    dimension: UnitDimension::Distance,
                }),
            }),
        ],
        slots: vec![],
    }
}
fn run(directory: &Path, input: &str, canonical: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command
        .current_dir(directory)
        .arg("check-owned-schema")
        .arg(input);
    if let Some(path) = canonical {
        command.arg("--canonical-output").arg(path);
    }
    command.output().unwrap()
}
fn successful(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["document_kind"], "definition_schema_package");
    assert_eq!(
        report["verification"],
        json!({"schema":"valid","declared_topology":"potential_only","build_binding":"not_run","legality":"not_checked","calculation":"not_run"})
    );
    report
}
#[test]
fn caller_schema_has_exact_canonical_artifact_identity_and_data_only_changes() {
    let temp = tempfile::tempdir().unwrap();
    let original = serde_json::to_vec_pretty(&input()).unwrap();
    fs::write(temp.path().join("input.json"), &original).unwrap();
    let first = successful(run(temp.path(), "input.json", Some("canonical.json")));
    let canonical = fs::read(temp.path().join("canonical.json")).unwrap();
    let package = decode_schema_package(&canonical, OwnedSchemaLimits::default()).unwrap();
    assert_eq!(
        first["identity"],
        serde_json::to_value(package.identity()).unwrap()
    );
    assert_eq!(
        first["identity"]["content_sha256"],
        format!("{:x}", Sha256::digest(&canonical))
    );
    assert_eq!(first["definition_entries"], 2);
    assert_eq!(first["slot_entries"], 0);
    assert_eq!(
        encode_schema_package(&package, OwnedSchemaLimits::default()).unwrap(),
        canonical
    );
    assert_eq!(fs::read(temp.path().join("input.json")).unwrap(), original);
    let mut changed = input();
    let DefinitionDescriptor::ExternalInput(entry) = &mut changed.definitions[0] else {
        panic!("wrong fixture")
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        panic!("wrong fixture")
    };
    let ValueSchema::Quantity(range) = &mut schema.value else {
        panic!("wrong fixture")
    };
    range.maximum = FiniteQuantity::new(75.0, id("distance")).unwrap();
    fs::write(
        temp.path().join("changed.json"),
        serde_json::to_vec(&changed).unwrap(),
    )
    .unwrap();
    let second = successful(run(temp.path(), "changed.json", None));
    assert_ne!(
        first["identity"]["content_sha256"],
        second["identity"]["content_sha256"]
    );
    assert_eq!(
        first["identity"]["semantics_version"],
        second["identity"]["semantics_version"]
    );
}
#[test]
fn explicit_unmapped_definition_survives_without_becoming_known_or_missing() {
    let temp = tempfile::tempdir().unwrap();
    let mut input = input();
    input
        .definitions
        .push(DefinitionDescriptor::Skill(DefinitionEntry {
            id: id("unmapped-skill"),
            schema: SchemaState::Unmapped {
                gaps: vec![SchemaGap {
                    subject: SchemaSubject::Definition(DefinitionAddress::Skill(id(
                        "unmapped-skill",
                    ))),
                    facet: SchemaFacet::InputSchema,
                    code: OwnedDefinitionKey::new("missing-conversion").unwrap(),
                }],
            },
        }));
    fs::write(
        temp.path().join("input.json"),
        serde_json::to_vec(&input).unwrap(),
    )
    .unwrap();
    successful(run(temp.path(), "input.json", Some("canonical.json")));
    let package = decode_schema_package(
        &fs::read(temp.path().join("canonical.json")).unwrap(),
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        package.definition(&id::<SkillDefinition>("unmapped-skill")),
        SchemaLookup::Unmapped(_)
    ));
    assert!(matches!(
        package.definition(&id::<SkillDefinition>("absent-skill")),
        SchemaLookup::Missing
    ));
}
#[test]
fn malformed_or_dangling_schema_fails_before_output_creation() {
    let temp = tempfile::tempdir().unwrap();
    let mut missing_unit = input();
    missing_unit.definitions.pop();
    let mut unknown = serde_json::to_value(input()).unwrap();
    unknown["source_program"] = json!({});
    let mut missing_field = serde_json::to_value(input()).unwrap();
    missing_field.as_object_mut().unwrap().remove("slots");
    for malformed in [
        serde_json::to_value(missing_unit).unwrap(),
        unknown,
        missing_field,
    ] {
        fs::write(
            temp.path().join("input.json"),
            serde_json::to_vec(&malformed).unwrap(),
        )
        .unwrap();
        let result = run(temp.path(), "input.json", Some("canonical.json"));
        assert!(!result.status.success());
        assert!(!result.stderr.is_empty());
        assert!(!temp.path().join("canonical.json").exists());
    }
}
#[test]
fn schema_check_never_overwrites_inputs_or_existing_outputs() {
    let temp = tempfile::tempdir().unwrap();
    let original = serde_json::to_vec(&input()).unwrap();
    fs::write(temp.path().join("input.json"), &original).unwrap();
    fs::write(temp.path().join("existing.json"), b"preserve").unwrap();
    for target in ["input.json", "existing.json"] {
        let result = run(temp.path(), "input.json", Some(target));
        assert!(!result.status.success());
        assert_eq!(fs::read(temp.path().join("input.json")).unwrap(), original);
        assert_eq!(
            fs::read(temp.path().join("existing.json")).unwrap(),
            b"preserve"
        );
    }
}
