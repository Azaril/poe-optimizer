//! Cold schema authority is independent of PoB and concrete evaluator inputs.
use poe_optimizer_core::{owned_build::DeclaredSlot, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
use serde_json::{Value, json};

#[path = "support/owned_preset_skill_input_permission.rs"]
mod preset_inputs;

fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("skill-input-authority-test", "v1").unwrap()
}
fn id<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(ns(), key).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn declarations(parameter: DeclaredSlot<ParameterSlotDefId>) -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![parameter]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn parameter(
    authority: Option<SkillInputAuthority>,
    sites: Vec<ParameterSite>,
) -> ParameterSlotSchema {
    ParameterSlotSchema {
        value: ValueSchema::Boolean,
        presence: SlotPresence::RequiredOnce,
        sites,
        skill_input: authority,
    }
}
fn input(
    authority: Option<SkillInputAuthority>,
    sites: Vec<ParameterSite>,
    version: u32,
) -> SchemaPackageInput {
    let slot = DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(id("selected-skill")),
        slot: id("raw-skill-input"),
    };
    SchemaPackageInput {
        schema_version: version,
        namespace: ns(),
        release: key("release"),
        semantics_version: key("input-authority-v1"),
        definitions: vec![DefinitionDescriptor::Skill(DefinitionEntry {
            id: id("selected-skill"),
            schema: SchemaState::Known(SkillSchema {
                directly_selectable: true,
                declarations: declarations(slot.clone()),
            }),
        })],
        slots: vec![SlotDescriptor::Parameter(DefinitionEntry {
            id: slot,
            schema: SchemaState::Known(parameter(authority, sites)),
        })],
    }
}
fn package(input: SchemaPackageInput) -> OwnedDefinitionSchemaPackage {
    OwnedDefinitionSchemaPackage::new(input, Default::default()).unwrap()
}
fn reject(input: SchemaPackageInput, expected: SchemaPackageErrorKind) {
    match OwnedDefinitionSchemaPackage::new(input, Default::default()) {
        Err(SchemaPackageError::Invalid { kind, path }) => {
            assert_eq!(kind, expected, "{path}");
            assert!(!path.is_empty());
        }
        other => panic!("expected {expected:?}, got {other:?}"),
    }
}

#[test]
fn all_explicit_authorities_roundtrip_with_distinct_producer_permissions() {
    for (authority, sites, authored, projected) in [
        (
            SkillInputAuthority::Authored,
            vec![ParameterSite::SkillParameter],
            true,
            false,
        ),
        (SkillInputAuthority::Projected, vec![], false, true),
        (
            SkillInputAuthority::AuthoredOrProjected,
            vec![ParameterSite::SkillParameter],
            true,
            true,
        ),
    ] {
        let schema = parameter(Some(authority), sites.clone());
        assert_eq!(schema.permits_authored_skill_input(), authored);
        assert_eq!(schema.permits_projected_skill_input(), projected);
        let p = package(input(Some(authority), sites, OWNED_SCHEMA_PACKAGE_VERSION));
        let bytes = encode_schema_package(&p, Default::default()).unwrap();
        let decoded = decode_schema_package(&bytes, Default::default()).unwrap();
        assert_eq!(p.input(), decoded.input());
        assert_eq!(p.identity(), decoded.identity());
        assert_eq!(
            bytes,
            encode_schema_package(&decoded, Default::default()).unwrap()
        );
        assert_eq!(p.identity().schema_version, OWNED_SCHEMA_PACKAGE_VERSION);
    }
}

#[test]
fn explicit_authority_is_not_a_null_or_open_ended_wire_extension() {
    let schema = parameter(None, vec![]);
    for invalid in [
        Value::Null,
        json!("future_authority"),
        json!({"kind":"authored"}),
        json!(true),
    ] {
        let mut wire = serde_json::to_value(&schema).unwrap();
        wire["skill_input"] = invalid;
        assert!(serde_json::from_value::<ParameterSlotSchema>(wire).is_err());
    }
    let mut unknown = serde_json::to_value(&schema).unwrap();
    unknown["input_authority"] = "authored".into();
    assert!(serde_json::from_value::<ParameterSlotSchema>(unknown).is_err());
    for (authority, token) in [
        (SkillInputAuthority::Authored, "authored"),
        (SkillInputAuthority::Projected, "projected"),
        (
            SkillInputAuthority::AuthoredOrProjected,
            "authored_or_projected",
        ),
    ] {
        assert_eq!(serde_json::to_value(authority).unwrap(), token);
    }
}

#[test]
fn current_schema_rejects_old_formats_without_inventing_authored_permissions() {
    assert_eq!(OWNED_SCHEMA_PACKAGE_VERSION, OWNED_SCHEMA_PACKAGE_V7);
    for version in 2..OWNED_SCHEMA_PACKAGE_VERSION {
        let old = input(
            Some(SkillInputAuthority::Authored),
            vec![ParameterSite::SkillParameter],
            version,
        );
        assert!(
            matches!(OwnedDefinitionSchemaPackage::new(old, Default::default()), Err(SchemaPackageError::UnsupportedVersion(v)) if v == version)
        );
    }
    let implicit = parameter(None, vec![]);
    assert!(!implicit.permits_authored_skill_input());
    assert!(implicit.permits_projected_skill_input());
    reject(
        input(
            None,
            vec![ParameterSite::SkillParameter],
            OWNED_SCHEMA_PACKAGE_VERSION,
        ),
        SchemaPackageErrorKind::WrongParameterSite,
    );
}

#[test]
fn skill_authority_and_authored_site_must_agree_exactly() {
    for (authority, sites) in [
        (Some(SkillInputAuthority::Authored), vec![]),
        (Some(SkillInputAuthority::AuthoredOrProjected), vec![]),
        (
            Some(SkillInputAuthority::Projected),
            vec![ParameterSite::SkillParameter],
        ),
        (
            Some(SkillInputAuthority::Projected),
            vec![ParameterSite::GemParameter],
        ),
        (
            Some(SkillInputAuthority::Authored),
            vec![ParameterSite::GemParameter],
        ),
        (
            Some(SkillInputAuthority::AuthoredOrProjected),
            vec![ParameterSite::SkillParameter, ParameterSite::GemParameter],
        ),
    ] {
        reject(
            input(authority, sites, OWNED_SCHEMA_PACKAGE_VERSION),
            SchemaPackageErrorKind::WrongParameterSite,
        );
    }
    reject(
        input(
            Some(SkillInputAuthority::Authored),
            vec![ParameterSite::SkillParameter, ParameterSite::SkillParameter],
            OWNED_SCHEMA_PACKAGE_VERSION,
        ),
        SchemaPackageErrorKind::DuplicateMember,
    );
}

#[test]
fn non_skill_owners_cannot_claim_any_skill_input_authority() {
    for owner in [
        SlotOwnerDefId::Reward(id("owner")),
        SlotOwnerDefId::Modifier(id("owner")),
        SlotOwnerDefId::UsagePolicy(id("owner")),
    ] {
        for authority in [
            SkillInputAuthority::Authored,
            SkillInputAuthority::Projected,
            SkillInputAuthority::AuthoredOrProjected,
        ] {
            let slot = DeclaredSlot {
                declaration: owner.clone(),
                slot: id("raw-skill-input"),
            };
            let declarations = declarations(slot.clone());
            let (definition, sites) = match &owner {
                SlotOwnerDefId::Reward(id) => (
                    DefinitionDescriptor::Reward(DefinitionEntry {
                        id: id.clone(),
                        schema: SchemaState::Known(RewardSchema { declarations }),
                    }),
                    vec![ParameterSite::RewardParameter],
                ),
                SlotOwnerDefId::Modifier(id) => (
                    DefinitionDescriptor::Modifier(DefinitionEntry {
                        id: id.clone(),
                        schema: SchemaState::Known(ModifierSchema { declarations }),
                    }),
                    vec![ParameterSite::ModifierRoll],
                ),
                SlotOwnerDefId::UsagePolicy(id) => (
                    DefinitionDescriptor::UsagePolicy(DefinitionEntry {
                        id: id.clone(),
                        schema: SchemaState::Known(UsagePolicySchema {
                            scenario_skill_applicability: None,
                            targets: vec![UsageTargetKind::Skill],
                            declarations,
                        }),
                    }),
                    vec![ParameterSite::UsagePolicyParameter],
                ),
                _ => unreachable!(),
            };
            let mut candidate = input(None, vec![], OWNED_SCHEMA_PACKAGE_VERSION);
            candidate.definitions = vec![definition];
            candidate.slots = vec![SlotDescriptor::Parameter(DefinitionEntry {
                id: slot,
                schema: SchemaState::Known(parameter(Some(authority), sites)),
            })];
            reject(candidate, SchemaPackageErrorKind::WrongParameterSite);
        }
    }
}

#[test]
fn omitted_parameter_roundtrips_and_explicit_authority_changes_identity() {
    const LEGACY: &str = r#"{"value":{"kind":"boolean"},"presence":"required_once","sites":[]}"#;
    let schema: ParameterSlotSchema = serde_json::from_str(LEGACY).unwrap();
    assert_eq!(schema, parameter(None, vec![]));
    assert_eq!(serde_json::to_string(&schema).unwrap(), LEGACY);
    let legacy_input = input(None, vec![], OWNED_SCHEMA_PACKAGE_VERSION);
    let legacy = package(legacy_input);
    let legacy_bytes = encode_schema_package(&legacy, Default::default()).unwrap();
    assert!(
        !String::from_utf8(legacy_bytes.clone())
            .unwrap()
            .contains("skill_input")
    );
    assert_eq!(
        decode_schema_package(&legacy_bytes, Default::default())
            .unwrap()
            .identity(),
        legacy.identity()
    );
    let implicit = package(input(None, vec![], OWNED_SCHEMA_PACKAGE_VERSION));
    let projected = package(input(
        Some(SkillInputAuthority::Projected),
        vec![],
        OWNED_SCHEMA_PACKAGE_VERSION,
    ));
    let authored = package(input(
        Some(SkillInputAuthority::Authored),
        vec![ParameterSite::SkillParameter],
        OWNED_SCHEMA_PACKAGE_VERSION,
    ));
    let both = package(input(
        Some(SkillInputAuthority::AuthoredOrProjected),
        vec![ParameterSite::SkillParameter],
        OWNED_SCHEMA_PACKAGE_VERSION,
    ));
    assert_ne!(implicit.identity(), projected.identity());
    assert_ne!(projected.identity(), authored.identity());
    assert_ne!(authored.identity(), both.identity());
    assert_eq!(legacy.identity(), implicit.identity());
}
