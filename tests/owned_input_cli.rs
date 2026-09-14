//! Host command checks against caller-authored owned inputs from an empty working directory.
use poe_optimizer_core::{build_identity::*, owned_build::*, owned_definitions::*};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn limits() -> OwnedInputLimits {
    OwnedInputLimits::default()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("caller-sample", "v7").unwrap()
}
fn definition<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(namespace(), key).unwrap()
}
fn id<T: BuildInstanceId>(local: u64) -> T {
    T::from_instance_id(
        InstanceId::from_parts(BuildLineage::from_bytes([0x61; 16]), local).unwrap(),
    )
}
fn build_input() -> BuildInput {
    BuildInput {
        allocator: InstanceAllocatorState::from_parts(BuildLineage::from_bytes([0x61; 16]), 31),
        revision: BuildRevision::from_u64(9),
        game_version: namespace(),
        character: CharacterSpec {
            class: definition("caller-class"),
            ascendancy: None,
            level: 23,
            rewards: vec![],
        },
        weapon_loadouts: vec![id(2), id(1)],
        active_weapon_loadout: id(2),
        items: vec![ItemRecord {
            id: id(3),
            template: definition("caller-item"),
            parameters: vec![ParameterAssignment {
                slot: DeclaredSlot {
                    declaration: SlotOwnerDefId::ItemTemplate(definition("caller-item")),
                    slot: definition("intrinsic-state"),
                },
                value: ParameterValue::Boolean(true),
            }],
            item_level: 17,
            quality: None,
            modifiers: vec![],
        }],
        gems: vec![],
        equipment: vec![EquipmentUse {
            id: id(6),
            item: id(3),
            destination: EquipmentDestination::CharacterSlot(definition("caller-slot")),
            scope: LoadoutScope::Shared,
        }],
        allocations: vec![],
        skills: vec![
            SkillUse {
                id: id(5),
                source: AuthoredSkillSource::Direct(definition("caller-action")),
                enabled: true,
                scope: LoadoutScope::Shared,
            },
            SkillUse {
                id: id(4),
                source: AuthoredSkillSource::Direct(definition("caller-action")),
                enabled: true,
                scope: LoadoutScope::Shared,
            },
        ],
        supports: vec![],
        payload_links: vec![],
        choices: vec![],
    }
}
fn request_input() -> OwnedRequestInput {
    let selected = ActionSelection {
        action: ActionKey {
            actor: ActorKey::Player,
            provider: ProviderKey {
                root: ProviderRoot::SkillUse(id(4)),
                grant_path: vec![],
            },
            output: DeclaredSlot {
                declaration: SlotOwnerDefId::Skill(definition("caller-action")),
                slot: definition("secondary-output"),
            },
        },
        part: definition("additional-part"),
        mode: definition("average-hit"),
        stat_set: definition("caller-stat-set"),
    };
    OwnedRequestInput {
        build: build_input(),
        scenario: ScenarioInput {
            game_version: namespace(),
            enemy: EnemySpec {
                encounter: definition("caller-encounter"),
                level: 41,
            },
            assumptions: vec![],
            usage: vec![],
        },
        queries: QueryInput {
            game_version: namespace(),
            requests: vec![
                MetricRequest {
                    id: QueryId::new("z-first").unwrap(),
                    metric: definition("caller-average"),
                    target: MetricTarget::Action(Box::new(selected)),
                },
                MetricRequest {
                    id: QueryId::new("a-second").unwrap(),
                    metric: definition("caller-resource"),
                    target: MetricTarget::Actor(ActorKey::Player),
                },
            ],
        },
    }
}
fn run(directory: &Path, input: &str, canonical: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command
        .current_dir(directory)
        .arg("check-owned-input")
        .arg(input);
    if let Some(path) = canonical {
        command.arg("--canonical-output").arg(path);
    }
    command.output().unwrap()
}
fn successful(output: Output, kind: &str) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["document_kind"], kind);
    assert_eq!(report["verification"]["structure"], "valid");
    assert_eq!(report["verification"]["definitions"], "not_bound");
    assert_eq!(report["verification"]["legality"], "not_checked");
    assert_eq!(report["verification"]["calculation"], "not_run");
    report
}
fn build_wire() -> Vec<u8> {
    serde_json::to_vec_pretty(
        &json!({"schema_version":1,"document":{"kind":"build","value":build_input()}}),
    )
    .unwrap()
}

#[test]
fn custom_build_checks_from_an_empty_working_directory_without_writing_output() {
    let temp = tempfile::tempdir().unwrap();
    let original = build_wire();
    fs::write(temp.path().join("caller input.json"), &original).unwrap();
    successful(run(temp.path(), "caller input.json", None), "build");
    assert_eq!(
        fs::read(temp.path().join("caller input.json")).unwrap(),
        original
    );
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
}

#[test]
fn custom_request_canonical_output_keeps_exact_selectors_and_query_order() {
    let temp = tempfile::tempdir().unwrap();
    let input = request_input();
    let expected = OwnedEvaluationRequest::new(
        BuildSpec::new(input.build.clone(), limits()).unwrap(),
        ScenarioSpec::new(input.scenario.clone(), limits()).unwrap(),
        QuerySpec::new(input.queries.clone(), limits()).unwrap(),
        limits(),
    )
    .unwrap();
    // Write the public DTO with intentionally unsorted build records, not an already canonical wrapper.
    let original = serde_json::to_vec_pretty(
        &json!({"schema_version":1,"document":{"kind":"request","value":input}}),
    )
    .unwrap();
    fs::write(temp.path().join("request.json"), &original).unwrap();
    successful(
        run(temp.path(), "request.json", Some("canonical.json")),
        "request",
    );
    let canonical = fs::read(temp.path().join("canonical.json")).unwrap();
    let decoded = decode_owned(&canonical, limits()).unwrap();
    assert_eq!(decoded, OwnedDocument::Request(Box::new(expected)));
    assert_eq!(encode_owned(&decoded, limits()).unwrap(), canonical);
    let OwnedDocument::Request(request) = decoded else {
        panic!("canonical request changed document kind")
    };
    assert_eq!(request.build().input().game_version, namespace());
    assert_eq!(request.build().input().allocator.last_issued(), 31);
    assert_eq!(request.build().input().revision.get(), 9);
    assert_eq!(
        request.build().input().weapon_loadouts,
        vec![id::<WeaponLoadoutId>(1), id(2)]
    );
    assert_eq!(
        request.build().input().active_weapon_loadout,
        id::<WeaponLoadoutId>(2)
    );
    assert_eq!(
        request
            .build()
            .input()
            .skills
            .iter()
            .map(|skill| skill.id)
            .collect::<Vec<_>>(),
        vec![id::<SkillUseId>(4), id(5)]
    );
    assert_eq!(
        request
            .queries()
            .input()
            .requests
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        ["z-first", "a-second"]
    );
    let MetricTarget::Action(selection) = &request.queries().input().requests[0].target else {
        panic!("lost action selector")
    };
    assert_eq!(
        selection.action.provider.root,
        ProviderRoot::SkillUse(id(4))
    );
    assert_eq!(
        selection.action.output.slot.key().as_str(),
        "secondary-output"
    );
    assert_eq!(selection.part.key().as_str(), "additional-part");
    assert_eq!(selection.mode.key().as_str(), "average-hit");
    assert_eq!(selection.stat_set.key().as_str(), "caller-stat-set");
    assert_eq!(
        fs::read(temp.path().join("request.json")).unwrap(),
        original
    );
}

#[test]
fn invalid_semantic_field_fails_without_creating_canonical_output() {
    let temp = tempfile::tempdir().unwrap();
    let mut invalid: Value = serde_json::from_slice(&build_wire()).unwrap();
    invalid["document"]["value"]["unmodeled_settings"] = json!({"override":123});
    fs::write(
        temp.path().join("invalid.json"),
        serde_json::to_vec(&invalid).unwrap(),
    )
    .unwrap();
    let result = run(temp.path(), "invalid.json", Some("canonical.json"));
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("unmodeled_settings"));
    assert!(!temp.path().join("canonical.json").exists());
}

#[test]
fn existing_canonical_destination_and_input_file_are_never_overwritten() {
    let temp = tempfile::tempdir().unwrap();
    let original = build_wire();
    fs::write(temp.path().join("input.json"), &original).unwrap();
    let preserved = b"existing file content is not an owned document";
    fs::write(temp.path().join("existing.json"), preserved).unwrap();
    for destination in ["existing.json", "input.json"] {
        let result = run(temp.path(), "input.json", Some(destination));
        assert!(!result.status.success(), "overwrote {destination}");
        assert!(!result.stderr.is_empty());
        assert_eq!(
            fs::read(temp.path().join("existing.json")).unwrap(),
            preserved
        );
        assert_eq!(fs::read(temp.path().join("input.json")).unwrap(), original);
    }
}

#[test]
fn standalone_inventory_checks_and_roundtrips_without_a_character_or_data_package() {
    use poe_optimizer_core::owned_inventory::*;
    let temp = tempfile::tempdir().unwrap();
    let build = build_input();
    let input = InventoryInput {
        allocator: build.allocator,
        revision: build.revision,
        game_version: build.game_version,
        items: build.items,
        copies: vec![
            InventoryItem {
                id: id(21),
                item: id(3),
            },
            InventoryItem {
                id: id(20),
                item: id(3),
            },
        ],
        completeness: InventoryCompleteness::Partial,
    };
    let expected = InventorySnapshot::new(input.clone(), limits()).unwrap();
    let original = serde_json::to_vec_pretty(
        &json!({"schema_version":1,"document":{"kind":"inventory","value":input}}),
    )
    .unwrap();
    fs::write(temp.path().join("stock.json"), &original).unwrap();
    successful(
        run(temp.path(), "stock.json", Some("canonical.json")),
        "inventory",
    );
    let canonical = fs::read(temp.path().join("canonical.json")).unwrap();
    assert_eq!(
        decode_owned(&canonical, limits()).unwrap(),
        OwnedDocument::Inventory(Box::new(expected))
    );
    assert_eq!(fs::read(temp.path().join("stock.json")).unwrap(), original);
}

#[test]
fn invalid_inventory_copy_is_rejected_before_any_output_file_is_created() {
    use poe_optimizer_core::owned_inventory::*;
    let temp = tempfile::tempdir().unwrap();
    let build = build_input();
    let input = InventoryInput {
        allocator: build.allocator,
        revision: build.revision,
        game_version: build.game_version,
        items: build.items,
        copies: vec![InventoryItem {
            id: id(20),
            item: id(25),
        }],
        completeness: InventoryCompleteness::Complete,
    };
    fs::write(
        temp.path().join("stock.json"),
        serde_json::to_vec(
            &json!({"schema_version":1,"document":{"kind":"inventory","value":input}}),
        )
        .unwrap(),
    )
    .unwrap();
    let result = run(temp.path(), "stock.json", Some("canonical.json"));
    assert!(!result.status.success());
    assert!(!result.stderr.is_empty());
    assert!(!temp.path().join("canonical.json").exists());
}
