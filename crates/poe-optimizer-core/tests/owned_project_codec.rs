//! Project envelope laws beyond the registry/composer's separate structural tests.
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*,
    owned_inventory::build_content_binding, owned_project::*,
};
use serde_json::{Value, json};

fn limits() -> OwnedInputLimits {
    OwnedInputLimits::default()
}
fn id<T: BuildInstanceId>(local: u64) -> T {
    T::from_instance_id(
        InstanceId::from_parts(BuildLineage::from_bytes([0x29; 16]), local).unwrap(),
    )
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("codec-project", "v1").unwrap()
}
fn def<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(namespace(), key).unwrap()
}
fn selection() -> VariantSelection {
    VariantSelection {
        character: id(3),
        equipment: id(5),
        allocations: id(6),
        skills: id(7),
        choices: id(8),
        active_weapon_loadout: id(2),
    }
}
fn input() -> ProjectInput {
    ProjectInput {
        allocator: InstanceAllocatorState::from_parts(id::<InstanceId>(1).lineage(), 50),
        revision: BuildRevision::from_u64(4),
        game_version: namespace(),
        weapon_loadouts: vec![id(2), id(1)],
        items: vec![],
        gems: vec![],
        rewards: vec![],
        equipment: vec![],
        allocations: vec![],
        skills: vec![],
        supports: vec![],
        payload_links: vec![],
        character_presets: vec![
            CharacterPreset {
                id: id(4),
                class: def("second-class"),
                ascendancy: Some(def("ascendancy")),
                level: 60,
                rewards: vec![],
            },
            CharacterPreset {
                id: id(3),
                class: def("first-class"),
                ascendancy: None,
                level: 40,
                rewards: vec![],
            },
        ],
        equipment_presets: vec![EquipmentPreset {
            id: id(5),
            equipment: vec![],
        }],
        allocation_presets: vec![AllocationPreset {
            id: id(6),
            allocations: vec![],
            equipment: vec![],
        }],
        skill_presets: vec![SkillPreset {
            id: id(7),
            skills: vec![],
            supports: vec![],
            payload_links: vec![],
        }],
        choice_presets: vec![ChoicePreset {
            id: id(8),
            rewards: vec![],
            choices: vec![],
        }],
        saved_variants: vec![
            SavedVariant {
                id: id(10),
                selection: VariantSelection {
                    character: id(4),
                    active_weapon_loadout: id(1),
                    ..selection()
                },
            },
            SavedVariant {
                id: id(9),
                selection: selection(),
            },
        ],
    }
}
fn document(raw: ProjectInput) -> OwnedDocument {
    OwnedDocument::Project(Box::new(BuildProject::new(raw, limits()).unwrap()))
}
fn unpack(document: OwnedDocument) -> BuildProject {
    match document {
        OwnedDocument::Project(project) => *project,
        other => panic!("expected project, got {other:?}"),
    }
}
fn wire(raw: ProjectInput) -> Value {
    json!({"schema_version":4,"document":{"kind":"project","value":raw}})
}
fn entries(value: &Value) -> usize {
    match value {
        Value::Array(values) => values.len() + values.iter().map(entries).sum::<usize>(),
        Value::Object(values) => values.values().map(entries).sum(),
        _ => 0,
    }
}

#[test]
fn project_envelope_roundtrip_retains_explicit_saved_selections_and_composes_standalone() {
    let original = document(input());
    let bytes = encode_owned(&original, limits()).unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["schema_version"], 4);
    assert_eq!(value["document"]["kind"], "project");
    let decoded = decode_owned(&bytes, limits()).unwrap();
    assert_eq!(decoded, original);
    let project = unpack(decoded);
    assert_eq!(project.saved_variant(id(9)).unwrap().selection, selection());
    assert_eq!(
        project.saved_variant(id(10)).unwrap().selection.character,
        id(4)
    );
    assert_eq!(
        project
            .saved_variant(id(10))
            .unwrap()
            .selection
            .active_weapon_loadout,
        id(1)
    );
    for variant in &project.input().saved_variants {
        let build = compose(&project, &variant.selection, None, limits()).unwrap();
        let standalone = OwnedDocument::Build(Box::new(build));
        assert_eq!(
            decode_owned(&encode_owned(&standalone, limits()).unwrap(), limits()).unwrap(),
            standalone
        );
    }
}

#[test]
fn project_decode_requires_explicit_optional_values_and_selected_loadout() {
    let original = wire(input());
    assert!(decode_owned(&serde_json::to_vec(&original).unwrap(), limits()).is_ok());
    for pointer in [
        "/document/value/character_presets/1",
        "/document/value/saved_variants/1/selection",
    ] {
        let mut missing = original.clone();
        let field = if pointer.ends_with("selection") {
            "active_weapon_loadout"
        } else {
            "ascendancy"
        };
        missing
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(decode_owned(&serde_json::to_vec(&missing).unwrap(), limits()).is_err());
    }
    let mut unknown = original;
    unknown["document"]["value"]["default_preset"] = json!(3);
    assert!(decode_owned(&serde_json::to_vec(&unknown).unwrap(), limits()).is_err());
    let mut duplicate = encode_owned(&document(input()), limits()).unwrap();
    let text = String::from_utf8(duplicate).unwrap();
    duplicate = text
        .replacen(
            "\"kind\":\"project\"",
            "\"kind\":\"project\",\"kind\":\"project\"",
            1,
        )
        .into_bytes();
    assert_ne!(duplicate, text.as_bytes());
    assert!(decode_owned(&duplicate, limits()).is_err());
}

#[test]
fn encoding_and_decoding_recheck_exact_project_byte_and_aggregate_limits() {
    let document = document(input());
    let bytes = encode_owned(&document, limits()).unwrap();
    let count = entries(&serde_json::from_slice::<Value>(&bytes).unwrap());
    let exact = OwnedInputLimits {
        max_entries: count,
        max_wire_bytes: bytes.len(),
        ..limits()
    };
    assert_eq!(encode_owned(&document, exact).unwrap(), bytes);
    assert_eq!(decode_owned(&bytes, exact).unwrap(), document);
    for tight in [
        OwnedInputLimits {
            max_entries: count - 1,
            ..exact
        },
        OwnedInputLimits {
            max_wire_bytes: bytes.len() - 1,
            ..exact
        },
    ] {
        assert!(encode_owned(&document, tight).is_err());
        assert!(decode_owned(&bytes, tight).is_err());
    }
}

#[test]
fn unordered_project_wire_canonicalizes_without_changing_selected_content_identity() {
    let raw = input();
    let first = unpack(document(raw.clone()));
    let mut permuted = raw;
    permuted.weapon_loadouts.reverse();
    permuted.character_presets.reverse();
    permuted.saved_variants.reverse();
    let second =
        unpack(decode_owned(&serde_json::to_vec(&wire(permuted)).unwrap(), limits()).unwrap());
    assert_eq!(
        encode_owned(&OwnedDocument::Project(Box::new(first.clone())), limits()).unwrap(),
        encode_owned(&OwnedDocument::Project(Box::new(second.clone())), limits()).unwrap()
    );
    let first_build = compose(&first, &selection(), None, limits()).unwrap();
    let second_build = compose(&second, &selection(), None, limits()).unwrap();
    assert_eq!(
        build_content_binding(&first_build, limits()).unwrap(),
        build_content_binding(&second_build, limits()).unwrap()
    );
    // A changed saved choice remains explicit and produces different selected content,
    // even though no caller changed the project revision counter.
    let mut changed = second.into_input();
    changed
        .saved_variants
        .iter_mut()
        .find(|v| v.id == id(9))
        .unwrap()
        .selection
        .character = id(4);
    let changed =
        unpack(decode_owned(&serde_json::to_vec(&wire(changed)).unwrap(), limits()).unwrap());
    let selected = changed.saved_variant(id(9)).unwrap().selection;
    let changed_build = compose(&changed, &selected, None, limits()).unwrap();
    assert_eq!(first_build.input().revision, changed_build.input().revision);
    assert_ne!(
        build_content_binding(&first_build, limits()).unwrap(),
        build_content_binding(&changed_build, limits()).unwrap()
    );
}

#[test]
fn current_envelope_requires_explicit_contribution_and_rejects_v1_before_parsing_the_old_payload_shape()
 {
    let mut value = wire(input());
    value["document"]["value"]["allocation_presets"][0]
        .as_object_mut()
        .unwrap()
        .remove("equipment");
    assert!(matches!(
        decode_owned(&serde_json::to_vec(&value).unwrap(), limits()),
        Err(CodecError::Json(_))
    ));
    value["schema_version"] = json!(1);
    assert!(matches!(
        decode_owned(&serde_json::to_vec(&value).unwrap(), limits()),
        Err(CodecError::UnsupportedVersion(1))
    ));
    value["schema_version"] = json!(OWNED_INPUT_SCHEMA_VERSION + 1);
    assert!(matches!(
        decode_owned(&serde_json::to_vec(&value).unwrap(), limits()),
        Err(CodecError::UnsupportedVersion(value)) if value == OWNED_INPUT_SCHEMA_VERSION + 1
    ));
}

#[test]
fn complete_choice_reward_membership_is_required_in_current_envelope() {
    let mut value = wire(input());
    value["document"]["value"]["choice_presets"][0]
        .as_object_mut()
        .unwrap()
        .remove("rewards");
    assert!(matches!(
        decode_owned(&serde_json::to_vec(&value).unwrap(), limits()),
        Err(CodecError::Json(_))
    ));
}

#[test]
fn project_item_level_null_survives_selection_and_requires_explicit_wire_presence() {
    let mut raw = input();
    raw.items.push(ItemRecord {
        id: id(11),
        template: def("item"),
        parameters: vec![],
        item_level: None,
        quality: None,
        modifier_order: vec![],
        modifiers: vec![],
    });
    raw.equipment.push(EquipmentUse {
        id: id(12),
        item: id(11),
        destination: EquipmentDestination::CharacterSlot(def("slot")),
        scope: LoadoutScope::Shared,
    });
    raw.equipment_presets[0].equipment.push(id(12));
    let original = document(raw);
    let encoded = encode_owned(&original, limits()).unwrap();
    let decoded = decode_owned(&encoded, limits()).unwrap();
    assert_eq!(decoded, original);
    let project = unpack(decoded);
    let composed = compose(&project, &selection(), None, limits()).unwrap();
    assert_eq!(composed.input().items[0].item_level, None);
    assert_eq!(composed.input().equipment[0].item, id::<ItemRecordId>(11));
    let mut value: Value = serde_json::from_slice(&encoded).unwrap();
    value["document"]["value"]["items"][0]
        .as_object_mut()
        .unwrap()
        .remove("item_level");
    assert!(matches!(
        decode_owned(&serde_json::to_vec(&value).unwrap(), limits()),
        Err(CodecError::Json(_))
    ));
    value["schema_version"] = json!(2);
    assert!(matches!(
        decode_owned(&serde_json::to_vec(&value).unwrap(), limits()),
        Err(CodecError::UnsupportedVersion(2))
    ));
}
