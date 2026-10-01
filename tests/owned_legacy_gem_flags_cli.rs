//! Reviewed raw Boolean inputs, independent from effective input calculations.
#[path = "support/owned_legacy_gem_flags_release.rs"]
mod release;

use poe_optimizer_core::{
    build_identity::BuildLineage,
    data::DataIdentity,
    owned_build::{DeclaredSlot, ParameterValue},
    owned_content::OwnedContentDigest,
    owned_definitions::*,
    owned_schema::*,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::GemInputRule,
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry},
    owned_source::SourceAttributeRef,
    owned_value_policy::*,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/legacy-gem-corruption-flags")
}
fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: impl AsRef<Path>, v: &impl Serialize) {
    fs::write(p, serde_json::to_vec(v).unwrap()).unwrap()
}
fn command(name: &str) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    c.arg(name);
    c
}
fn success(o: Output) -> Value {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Authoring {
    schema_version: u32,
    before: OwnedContentDigest,
    definitions: DataIdentity,
    source_revision: String,
    source_files: BTreeMap<String, String>,
    source_tests: Vec<String>,
    owners: Vec<Owner>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Owner {
    gem: GemDefId,
    flag: DeclaredSlot<ParameterSlotDefId>,
    delta: DeclaredSlot<ParameterSlotDefId>,
    prior: DefinitionDescriptor,
    prior_rule: GemInputRule,
    next_rule: GemInputRule,
}

#[test]
fn authored_boolean_additions_preserve_quality_and_partial_input_membership() {
    let a: Authoring = read(data().join("authoring.json"));
    let e: OwnedRecipeExtension = read(data().join("extension.json"));
    assert_eq!(a.schema_version, 1);
    assert_eq!(a.definitions.schema_version, 4);
    assert_eq!(
        a.source_revision,
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    assert_eq!(e.schema_version, 1);
    assert!(
        e.operations_version.is_none()
            && e.tables.is_empty()
            && e.owners.is_empty()
            && e.receivers.is_empty()
    );
    assert_eq!(a.owners.len(), 2);
    assert_eq!(e.schema.len(), 4);
    assert!(a.source_files.contains_key("src/Classes/SkillsTab.lua"));
    for source in &a.source_tests {
        assert!(root().join(source).is_file())
    }
    for (i, (row, (gem, flag, delta))) in a
        .owners
        .iter()
        .zip([
            (
                "def.000000000000000a",
                "def.00000000000030b0",
                "def.00000000000030a9",
            ),
            (
                "def.0000000000000011",
                "def.00000000000030b1",
                "def.00000000000030aa",
            ),
        ])
        .enumerate()
    {
        assert_eq!(row.gem.key().as_str(), gem);
        assert_eq!(row.flag.slot.key().as_str(), flag);
        assert_eq!(row.delta.slot.key().as_str(), delta);
        assert_eq!(row.flag.declaration, SlotOwnerDefId::Gem(row.gem.clone()));
        let mut expected = row.prior.clone();
        let DefinitionDescriptor::Gem(owner) = &mut expected else {
            panic!("Gem")
        };
        assert_eq!(owner.id, row.gem);
        let SchemaState::Known(schema) = &mut owner.schema else {
            panic!("Known")
        };
        assert!(schema.quality.allowed_kinds.is_complete());
        assert!(matches!(
            schema.declarations.parameters.closure,
            SchemaClosure::Partial { .. }
        ));
        assert_eq!(
            schema.declarations.parameters.members,
            vec![row.delta.clone()]
        );
        schema
            .declarations
            .parameters
            .members
            .push(row.flag.clone());
        assert_eq!(e.schema[i], SchemaExtensionEntry::Definition(expected));
        let SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(slot)) = &e.schema[i + 2] else {
            panic!("parameter")
        };
        assert_eq!(slot.id, row.flag);
        assert_eq!(
            slot.schema,
            SchemaState::Known(ParameterSlotSchema {
                value: ValueSchema::Boolean,
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::GemParameter],
            })
        );
        assert_eq!(row.prior_rule.gem, row.gem);
        assert_eq!(row.prior_rule.guards.len(), 1);
        assert_eq!(row.prior_rule.guards[0].attribute, "corrupted");
        assert_eq!(
            serde_json::to_value(&row.prior_rule.guards[0].allowed).unwrap(),
            serde_json::json!([{"kind":"text","value":"false"},{"kind":"text","value":"nil"}])
        );
        assert_eq!(row.prior_rule.parameters.len(), 1);
        assert_eq!(row.prior_rule.parameters[0].slot, row.delta);
        assert_eq!(row.next_rule.gem, row.gem);
        assert!(row.next_rule.guards.is_empty());
        assert_eq!(row.next_rule.parameters.len(), 2);
        assert_eq!(row.next_rule.parameters[0], row.prior_rule.parameters[0]);
        assert_eq!(row.next_rule.parameters[1].slot, row.flag);
        assert_eq!(
            row.next_rule.parameters[1].value,
            a.owners[0].next_rule.parameters[1].value
        );
    }
}

#[test]
fn reviewed_boolean_tokens_remain_exact_and_missing_is_not_an_implicit_false() {
    let a: Authoring = read(data().join("authoring.json"));
    let input = &a.owners[0].next_rule.parameters[1].value;
    let recipe = ValueRecipe::new(input.clone(), Default::default()).unwrap();
    let source = ImportedBuildInstance::from_decoded(
        decode_build(b"<PathOfBuilding2><Gem corrupted=\"false\"/></PathOfBuilding2>").unwrap(),
        BuildLineage::from_bytes([120; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let row = source
        .occurrences()
        .iter()
        .find(|v| v.name() == "Gem")
        .unwrap();
    let origin = SourceAttributeRef {
        occurrence: row.id(),
        index: 0,
    };
    let selector = &input.tiers[0].selectors[0];
    assert_eq!(selector.name, "corrupted");
    assert_eq!(selector.lane, ValueLane::Attribute);
    for (text, expected) in [("true", true), ("false", false), ("nil", false)] {
        let decision = recipe
            .decide(&[ValueCandidate {
                selector,
                origin,
                value: CandidateValue::Decoded(text),
            }])
            .unwrap();
        assert!(
            matches!(decision.outcome,ValueOutcome::Selected{origin:actual,value:ParameterValue::Boolean(value)} if actual==origin && value==expected)
        );
    }
    for text in ["TRUE", "False", "1", "0", "", " true", "false ", "bogus"] {
        assert!(
            matches!(
                recipe
                    .decide(&[ValueCandidate {
                        selector,
                        origin,
                        value: CandidateValue::Decoded(text)
                    }])
                    .unwrap()
                    .outcome,
                ValueOutcome::Pending { .. }
            ),
            "{text}"
        );
    }
    assert!(matches!(
        recipe.decide(&[]).unwrap().outcome,
        ValueOutcome::Pending { .. }
    ));
    assert!(
        recipe
            .decide(&[
                ValueCandidate {
                    selector,
                    origin,
                    value: CandidateValue::Decoded("true")
                },
                ValueCandidate {
                    selector,
                    origin,
                    value: CandidateValue::Decoded("false")
                },
            ])
            .is_err()
    );
}

#[test]
#[ignore = "requires the explicit checked physical-quality-kind successor"]
fn real_boolean_successor_preserves_release_and_normalizes_originals_and_probes() {
    let prior = std::env::var_os("POE_OPTIMIZER_TEST_LEGACY_GEM_FLAGS_PRIOR")
        .expect("explicit predecessor required");
    release::check(&PathBuf::from(prior));
}
