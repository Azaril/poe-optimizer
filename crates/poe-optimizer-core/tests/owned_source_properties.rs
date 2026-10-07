//! Release declarations bind existing occurrences; they never embed build IDs.
use poe_optimizer_core::{
    owned_build::DeclaredSlot, owned_definitions::*, owned_source_properties::*,
};
use serde_json::{Value, json};

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn def<T: DefinitionDomain>(value: &str) -> DefId<T> {
    DefId::new(
        GameVersionNamespace::new("source-properties", "test").unwrap(),
        key(value),
    )
}
fn supply() -> DeclaredSlot<SkillGrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::PassiveNode(def("supplying-node")),
        slot: def("supplied-skill"),
    }
}

#[test]
fn occurrence_forms_require_explicit_declaration_applicability() {
    let authored = SourcePropertyOccurrence::AuthoredSkillUse {};
    assert_eq!(
        serde_json::to_value(&authored).unwrap(),
        json!({"kind":"authored_skill_use"})
    );
    let generated = SourcePropertyOccurrence::GeneratedSkill {
        skill_supply: supply(),
    };
    let encoded = serde_json::to_value(&generated).unwrap();
    assert_eq!(
        encoded,
        json!({"kind":"generated_skill", "skill_supply":supply()})
    );
    for value in [authored, generated] {
        assert_eq!(
            serde_json::from_value::<SourcePropertyOccurrence>(
                serde_json::to_value(&value).unwrap()
            )
            .unwrap(),
            value,
        );
    }
    for invalid in [
        json!("authored_skill_use_v1"),
        json!({"kind":"generated_skill"}),
        json!({"kind":"generated_skill", "skill_supply":null}),
        json!({"kind":"authored_skill_use", "skill_supply":supply()}),
        json!({"kind":"generated_skill", "provider":{"root":"allocation"}}),
    ] {
        assert!(serde_json::from_value::<SourcePropertyOccurrence>(invalid).is_err());
    }
    for (field, extra) in [
        ("provider", json!({"root":"allocation"})),
        ("skill_preset", json!("saved-set-4")),
        ("source_name", json!("source display name")),
        ("include_authored", json!(true)),
    ] {
        let mut invalid = encoded.clone();
        invalid[field] = extra;
        assert!(serde_json::from_value::<SourcePropertyOccurrence>(invalid).is_err());
    }
}

#[test]
fn assembly_binding_infers_ownership_without_an_arbitrary_override() {
    for (binding, spelling) in [
        (SourcePropertyAssemblyBinding::InputOwner, "input_owner"),
        (
            SourcePropertyAssemblyBinding::ExactSupplyingProvider,
            "exact_supplying_provider",
        ),
    ] {
        let row = SourcePropertyAssemblyProgram {
            program: key("assemble-final-inputs"),
            binding,
        };
        let encoded = serde_json::to_value(&row).unwrap();
        assert_eq!(
            encoded,
            json!({"program":"assemble-final-inputs", "binding":spelling})
        );
        assert_eq!(
            serde_json::from_value::<SourcePropertyAssemblyProgram>(encoded.clone()).unwrap(),
            row
        );
        for (field, extra) in [
            (
                "owner",
                json!({"kind":"skill", "definition":def::<SkillDefinition>("foreign")}),
            ),
            ("provider", json!({"root":"allocation"})),
            ("grant_path", json!([])),
        ] {
            let mut invalid = encoded.clone();
            invalid[field] = extra;
            assert!(serde_json::from_value::<SourcePropertyAssemblyProgram>(invalid).is_err());
        }
    }
    for invalid in [
        json!("assemble-final-inputs"),
        json!({"program":"assemble-final-inputs"}),
        json!({"program":"assemble-final-inputs", "binding":null}),
        json!({"program":"assemble-final-inputs", "binding":"nearest_parent"}),
        json!({"program":"assemble-final-inputs", "binding":"entered_child"}),
    ] {
        assert!(serde_json::from_value::<SourcePropertyAssemblyProgram>(invalid).is_err());
    }
}

#[test]
fn owner_effect_does_not_carry_a_second_occurrence_identity() {
    let endpoint = SourcePropertyEffectEndpoint::OwnerSkill {};
    let encoded = serde_json::to_value(&endpoint).unwrap();
    assert_eq!(encoded, json!({"kind":"owner_skill"}));
    assert_eq!(
        serde_json::from_value::<SourcePropertyEffectEndpoint>(encoded.clone()).unwrap(),
        endpoint
    );
    for invalid in [
        json!({"kind":"direct_owner"}),
        json!({"kind":"owner_skill", "skill":def::<SkillDefinition>("foreign")}),
        json!({"kind":"owner_skill", "provider":{"root":"allocation"}}),
        json!({"kind":"owner_skill", "physical_gem_as_skill":true}),
        Value::Null,
    ] {
        assert!(serde_json::from_value::<SourcePropertyEffectEndpoint>(invalid).is_err());
    }
}
