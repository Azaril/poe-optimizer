//! Offline authored input preparation. This never promotes pre-support values
//! into final supplied Skill parameters or claims whole-build coverage.
#[path = "support/owned_effective_gem_release.rs"]
mod release;

use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::owned_effective_gem_recipe::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn compile_cli(policy: &Path, schema: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("compile-owned-effective-gem-inputs")
        .arg(policy)
        .arg("--definitions")
        .arg(schema)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn rejected(output: Output, destination: &Path) {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    assert!(!destination.exists());
}

fn record<I, S>(id: I, schema: S) -> DefinitionEntry<I, S> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn slots() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}

fn synthetic() -> (OwnedDefinitionSchemaPackage, EffectiveGemRecipeInput) {
    let ns = GameVersionNamespace::new("effective-input-cli-test", "v1").unwrap();
    let gem = GemDefId::new(ns.clone(), key("synthetic-active"));
    let count = UnitDefId::new(ns.clone(), key("count"));
    let percent = UnitDefId::new(ns.clone(), key("percentage-points"));
    let quality = QualityDefId::new(ns.clone(), key("ordinary"));
    let level = StatDefId::new(ns.clone(), key("pre-support-level"));
    let amount = StatDefId::new(ns.clone(), key("pre-support-quality"));
    let corruption = DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(gem.clone()),
        slot: ParameterSlotDefId::new(ns.clone(), key("raw-corruption-delta")),
    };
    let mut declarations = slots();
    declarations.parameters.members.push(corruption.clone());
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: 4,
            namespace: ns,
            release: key("synthetic"),
            semantics_version: key("synthetic"),
            definitions: vec![
                DefinitionDescriptor::Unit(record(
                    count.clone(),
                    UnitSchema {
                        dimension: UnitDimension::Count,
                    },
                )),
                DefinitionDescriptor::Unit(record(
                    percent.clone(),
                    UnitSchema {
                        dimension: UnitDimension::PercentagePoints,
                    },
                )),
                DefinitionDescriptor::Quality(record(
                    quality.clone(),
                    QualitySchema {
                        amount: QuantityRange {
                            minimum: FiniteQuantity::new(0., percent.clone()).unwrap(),
                            maximum: FiniteQuantity::new(100., percent.clone()).unwrap(),
                        },
                    },
                )),
                DefinitionDescriptor::Gem(record(
                    gem.clone(),
                    GemSchema {
                        level: IntegerRange {
                            minimum: BoundedInteger::new(1).unwrap(),
                            maximum: BoundedInteger::new(40).unwrap(),
                        },
                        roles: vec![AuthoredGemRole::SkillUse],
                        skills: DeclaredSet::complete(vec![]),
                        quality: QualityUseSchema {
                            presence: QualityPresence::Optional,
                            allowed_kinds: DeclaredSet::complete(vec![quality.clone()]),
                        },
                        declarations,
                    },
                )),
                DefinitionDescriptor::Stat(record(
                    level.clone(),
                    StatSchema {
                        value: ComputedValueType::Quantity {
                            unit: count.clone(),
                        },
                        targets: vec![RuleEntityKind::Skill],
                    },
                )),
                DefinitionDescriptor::Stat(record(
                    amount.clone(),
                    StatSchema {
                        value: ComputedValueType::Quantity { unit: percent },
                        targets: vec![RuleEntityKind::Skill],
                    },
                )),
            ],
            slots: vec![SlotDescriptor::Parameter(record(
                corruption.clone(),
                ParameterSlotSchema {
                    skill_input: None,
                    value: ValueSchema::Quantity(QuantityRange {
                        minimum: FiniteQuantity::new(-100., count.clone()).unwrap(),
                        maximum: FiniteQuantity::new(100., count.clone()).unwrap(),
                    }),
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::GemParameter],
                },
            ))],
        },
        Default::default(),
    )
    .unwrap();
    let binding = serde_json::from_value(serde_json::json!({
        "gem":gem,"program":"pre-support-inputs","role":{"kind":"active_pre_support","corruption":corruption},
        "quality":quality,"quality_absence":"zero_when_proven_singleton","level_unit":count,
        "external_level":[],"external_quality":[],
        "level_output":level,"quality_output":amount
    })).unwrap();
    let policy = EffectiveGemRecipeInput {
        schema_version: 1,
        version: key("synthetic-policy"),
        definitions: schema.identity().clone(),
        bindings: vec![binding],
    };
    (schema, policy)
}

#[test]
fn recipe_cli_matches_public_compiler_and_preserves_existing_destination() {
    let temp = tempfile::tempdir().unwrap();
    let (schema, policy) = synthetic();
    let schema_path = temp.path().join("schema.json");
    let policy_path = temp.path().join("policy.json");
    write(&schema_path, schema.input());
    write(&policy_path, &policy);
    let output = temp.path().join("compiled");
    let report = success(compile_cli(&policy_path, &schema_path, &output));
    let compiled = compile_effective_gem_recipe(&policy, &schema, Default::default()).unwrap();
    assert_eq!(
        read::<Value>(output.join("programs.json")),
        serde_json::to_value(compiled).unwrap()
    );
    assert_eq!(report, read::<Value>(output.join("report.json")));
    assert_eq!(
        report["verification"]["final_active_inputs"],
        "not_produced"
    );
    assert_eq!(report["verification"]["coverage"], "unchanged");
    let before = fs::read(output.join("programs.json")).unwrap();
    let second = compile_cli(&policy_path, &schema_path, &output);
    assert!(!second.status.success());
    assert_eq!(before, fs::read(output.join("programs.json")).unwrap());
}

#[test]
fn recipe_cli_rejects_stale_schema_and_invalid_output_scope_before_publication() {
    let temp = tempfile::tempdir().unwrap();
    let (schema, policy) = synthetic();
    let policy_path = temp.path().join("policy.json");
    let schema_path = temp.path().join("schema.json");
    write(&policy_path, &policy);
    let mut changed = schema.input().clone();
    changed.release = key("different-release");
    write(&schema_path, &changed);
    rejected(
        compile_cli(&policy_path, &schema_path, &temp.path().join("stale")),
        &temp.path().join("stale"),
    );
    let mut invalid = policy.clone();
    invalid.bindings[0].quality_output = invalid.bindings[0].level_output.clone();
    write(&policy_path, &invalid);
    write(&schema_path, schema.input());
    rejected(
        compile_cli(&policy_path, &schema_path, &temp.path().join("bad-scope")),
        &temp.path().join("bad-scope"),
    );
}

#[test]
#[ignore = "requires an explicitly supplied immutable real Cleric predecessor"]
fn explicitly_supplied_real_cleric_release_has_reproducible_preparation_successor() {
    let path = std::env::var_os("POE_OPTIMIZER_TEST_EFFECTIVE_GEM_PRIOR")
        .expect("POE_OPTIMIZER_TEST_EFFECTIVE_GEM_PRIOR must select the checked real predecessor");
    release::check(&PathBuf::from(path));
}
