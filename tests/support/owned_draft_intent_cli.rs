use super::*;
use poe_optimizer_core::{owned_preset_intent::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;

fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: empty(),
        choices: empty(),
        grants: empty(),
        actors: empty(),
        skill_grants: empty(),
        outputs: empty(),
        sockets: empty(),
    }
}
fn known<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn save_intent(directory: &Path, selected: bool, quality: f64) {
    let mut input = draft(false).into_input();
    let supply = DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(definition("granting-item")),
        slot: definition("grant"),
    };
    let parameter = DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(definition("generated-skill")),
        slot: definition("quality"),
    };
    input.items.members.push(
        ItemRecord {
            id: id(10),
            template: definition("granting-item"),
            parameters: vec![],
            item_level: Some(20),
            quality: None,
            modifiers: vec![],
            modifier_order: vec![],
        }
        .into(),
    );
    input.equipment.members.push(
        EquipmentUse {
            id: id(11),
            item: id(10),
            destination: EquipmentDestination::CharacterSlot(definition("weapon")),
            scope: LoadoutScope::Shared,
        }
        .into(),
    );
    input.equipment_presets.members.push(EquipmentPresetDraft {
        id: id(12),
        equipment: list(vec![id(11)]),
    });
    input.skill_presets.members[0].intent = Some(
        SkillPresetIntentV1 {
            schema_version: 1,
            usage: vec![],
            generated_inputs: vec![GeneratedSkillInputBinding {
                target: GeneratedSkillKey {
                    provider: ProviderKey {
                        root: ProviderRoot::EquipmentUse(id(11)),
                        grant_path: vec![],
                    },
                    slot: supply.clone(),
                },
                parameters: vec![ParameterAssignment {
                    slot: parameter.clone(),
                    value: ParameterValue::Quantity(
                        FiniteQuantity::new(quality, definition("quality-unit")).unwrap(),
                    ),
                }],
                applicability: PresetApplicability::WhenExactSourceSelected,
            }],
        }
        .into(),
    );
    let draft = DraftSession::new(input, limits()).unwrap();
    fs::write(
        directory.join("draft.json"),
        encode_draft(&draft, limits()).unwrap(),
    )
    .unwrap();
    let mut selection = selection();
    if selected {
        selection.build.equipment = id(12);
    }
    fs::write(
        directory.join("selection.json"),
        serde_json::to_vec(&selection).unwrap(),
    )
    .unwrap();
    let mut item_declarations = declarations();
    item_declarations.skill_grants.members.push(supply.clone());
    let mut skill_declarations = declarations();
    skill_declarations
        .parameters
        .members
        .push(parameter.clone());
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_V6,
            namespace: namespace(),
            release: OwnedDefinitionKey::new("fixture").unwrap(),
            semantics_version: OwnedDefinitionKey::new("fixture").unwrap(),
            definitions: vec![
                DefinitionDescriptor::ItemTemplate(known(
                    definition("granting-item"),
                    ItemTemplateSchema {
                        item_level: IntegerRange {
                            minimum: BoundedInteger::new(1).unwrap(),
                            maximum: BoundedInteger::new(100).unwrap(),
                        },
                        equipment_slots: DeclaredSet::complete(vec![definition("weapon")]),
                        socket_destinations: empty(),
                        modifiers: empty(),
                        quality: QualityUseSchema {
                            presence: QualityPresence::Forbidden,
                            allowed_kinds: empty(),
                        },
                        declarations: item_declarations,
                    },
                )),
                DefinitionDescriptor::Skill(known(
                    definition("generated-skill"),
                    SkillSchema {
                        directly_selectable: false,
                        declarations: skill_declarations,
                    },
                )),
                DefinitionDescriptor::Unit(known(
                    definition("quality-unit"),
                    UnitSchema {
                        dimension: UnitDimension::Count,
                    },
                )),
                DefinitionDescriptor::EquipmentSlot(known(
                    definition("weapon"),
                    EquipmentSlotSchema {
                        scope: ScopePolicy::Shared,
                    },
                )),
            ],
            slots: vec![
                SlotDescriptor::SkillGrant(known(
                    supply,
                    SkillGrantSlotSchema {
                        skill: definition("generated-skill"),
                        outputs: empty(),
                        preset_inputs: Some(PresetSkillInputPermission {
                            schema_version: 1,
                            parameters: DeclaredSet::complete(vec![parameter.clone()]),
                        }),
                    },
                )),
                SlotDescriptor::Parameter(known(
                    parameter,
                    ParameterSlotSchema {
                        value: ValueSchema::Quantity(QuantityRange {
                            minimum: FiniteQuantity::new(0.0, definition("quality-unit")).unwrap(),
                            maximum: FiniteQuantity::new(40.0, definition("quality-unit")).unwrap(),
                        }),
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![],
                        skill_input: Some(SkillInputAuthority::Projected),
                    },
                )),
            ],
        },
        Default::default(),
    )
    .unwrap();
    fs::write(
        directory.join("definitions.json"),
        encode_schema_package(&schema, Default::default()).unwrap(),
    )
    .unwrap();
}

#[test]
fn generated_intent_requires_definitions_before_any_output() {
    let temp = tempfile::tempdir().unwrap();
    save_intent(temp.path(), true, 12.5);
    successful(run(temp.path(), &[]));
    let output = run(
        temp.path(),
        &[
            "--selection",
            "selection.json",
            "--owned-output",
            "request.json",
            "--draft-output",
            "checked.json",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--definitions"));
    assert!(!temp.path().join("request.json").exists());
    assert!(!temp.path().join("checked.json").exists());
}

#[test]
fn checked_cli_emits_exact_inputs_or_explicit_nonselection_diagnostics() {
    for selected in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        save_intent(temp.path(), selected, 12.5);
        let report = successful(run(
            temp.path(),
            &[
                "--definitions",
                "definitions.json",
                "--selection",
                "selection.json",
                "--owned-output",
                "request.json",
            ],
        ));
        assert_eq!(report["finalization"]["status"], "ready");
        let disposition =
            &report["intent_validation"]["finalization"]["diagnostics"][0]["disposition"]["kind"];
        assert_eq!(
            disposition,
            if selected {
                "applied"
            } else {
                "not_applicable"
            }
        );
        assert_eq!(
            report["intent_validation"]["draft_digest"],
            report["draft_digest"]
        );
        let OwnedDocument::Request(request) = decode_owned(
            &fs::read(temp.path().join("request.json")).unwrap(),
            limits().input,
        )
        .unwrap() else {
            panic!()
        };
        let rows = &request
            .build()
            .input()
            .generated_inputs
            .as_ref()
            .unwrap()
            .bindings;
        assert_eq!(rows.len(), usize::from(selected));
        if selected {
            assert_eq!(
                rows[0].parameters[0].value,
                ParameterValue::Quantity(
                    FiniteQuantity::new(12.5, definition("quality-unit")).unwrap()
                )
            );
        }
        assert_eq!(request.queries().input().requests.len(), 2);
    }
}

#[test]
fn dormant_invalid_quality_fails_before_output() {
    let temp = tempfile::tempdir().unwrap();
    save_intent(temp.path(), false, 41.0);
    let output = run(
        temp.path(),
        &[
            "--definitions",
            "definitions.json",
            "--selection",
            "selection.json",
            "--owned-output",
            "request.json",
        ],
    );
    assert!(!output.status.success());
    assert!(!temp.path().join("request.json").exists());
}
