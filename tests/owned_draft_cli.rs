//! Caller-authored drafts run from an empty directory, with either CLI feature set.
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_draft::*,
    owned_project::VariantSelection,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn limits() -> DraftLimits {
    DraftLimits::default()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("draft-caller", "v2").unwrap()
}
fn definition<K: DefinitionDomain>(key: &str) -> DefId<K> {
    DefId::parse(namespace(), key).unwrap()
}
fn id<T: BuildInstanceId>(local: u64) -> T {
    T::from_instance_id(
        InstanceId::from_parts(BuildLineage::from_bytes([0x75; 16]), local).unwrap(),
    )
}
fn list<T>(members: Vec<T>) -> DraftList<T> {
    DraftList {
        members,
        completion: DraftListCompletion::Complete,
    }
}
fn selection() -> EvaluationSelection {
    EvaluationSelection {
        build: VariantSelection {
            character: id(2),
            equipment: id(3),
            allocations: id(4),
            skills: id(5),
            choices: id(6),
            active_weapon_loadout: id(1),
        },
        scenario: id(7),
        queries: id(8),
    }
}
fn draft(pending: bool) -> DraftSession {
    let mut queries: QueryDraft = QueryInput {
        game_version: namespace(),
        requests: vec![
            MetricRequest {
                id: QueryId::new("z-first").unwrap(),
                metric: definition("caller-metric"),
                target: MetricTarget::Actor(ActorKey::Player),
            },
            MetricRequest {
                id: QueryId::new("a-second").unwrap(),
                metric: definition("caller-other-metric"),
                target: MetricTarget::Actor(ActorKey::Player),
            },
        ],
    }
    .into();
    if pending {
        queries.requests.members[1].target = DraftMetricTarget::Pending(PendingValue {
            id: id(90),
            code: OwnedDefinitionKey::new("target-unresolved").unwrap(),
            candidates: vec![],
        });
    }
    DraftSession::new(
        DraftSessionInput {
            allocator: InstanceAllocatorState::from_parts(
                BuildLineage::from_bytes([0x75; 16]),
                100,
            ),
            revision: BuildRevision::from_u64(3),
            game_version: namespace(),
            weapon_loadouts: list(vec![id(1)]),
            items: list(vec![]),
            gems: list(vec![]),
            rewards: list(vec![]),
            equipment: list(vec![]),
            allocations: list(vec![]),
            skills: list(vec![]),
            supports: list(vec![]),
            payload_links: list(vec![]),
            character_presets: list(vec![CharacterPresetDraft {
                id: id(2),
                class: DraftField::Known {
                    value: definition("caller-class"),
                },
                ascendancy: DraftField::Known { value: None },
                level: DraftField::Known { value: 24 },
                rewards: list(vec![]),
            }]),
            equipment_presets: list(vec![EquipmentPresetDraft {
                id: id(3),
                equipment: list(vec![]),
            }]),
            allocation_presets: list(vec![AllocationPresetDraft {
                id: id(4),
                allocations: list(vec![]),
            }]),
            skill_presets: list(vec![SkillPresetDraft {
                id: id(5),
                skills: list(vec![]),
                supports: list(vec![]),
                payload_links: list(vec![]),
            }]),
            choice_presets: list(vec![ChoicePresetDraft {
                id: id(6),
                choices: list(vec![]),
            }]),
            scenario_presets: list(vec![ScenarioPresetDraft {
                id: id(7),
                scenario: ScenarioInput {
                    game_version: namespace(),
                    enemy: EnemySpec {
                        encounter: definition("caller-encounter"),
                        level: 34,
                    },
                    assumptions: vec![],
                    usage: vec![],
                }
                .into(),
            }]),
            query_presets: list(vec![QueryPresetDraft { id: id(8), queries }]),
            saved_variants: list(vec![]),
        },
        limits(),
    )
    .unwrap()
}
fn save(directory: &Path, pending: bool) -> Vec<u8> {
    let bytes = encode_draft(&draft(pending), limits()).unwrap();
    fs::write(directory.join("draft.json"), &bytes).unwrap();
    fs::write(
        directory.join("selection.json"),
        serde_json::to_vec(&selection()).unwrap(),
    )
    .unwrap();
    bytes
}
fn run(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(directory)
        .args(["check-owned-draft", "draft.json"])
        .args(arguments)
        .output()
        .unwrap()
}
fn successful(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["document_kind"], "draft");
    assert_eq!(report["owned_draft_schema_version"], 1);
    assert_eq!(report["verification"]["structure"], "valid");
    assert_eq!(report["verification"]["definitions"], "not_bound");
    assert_eq!(report["verification"]["legality"], "not_checked");
    assert_eq!(report["verification"]["calculation"], "not_run");
    report
}

#[test]
fn complete_and_pending_drafts_check_without_selecting_or_writing() {
    for pending in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let original = save(temp.path(), pending);
        let report = successful(run(temp.path(), &[]));
        let validation = draft(pending).validate_limits(limits()).unwrap();
        assert_eq!(report["issue_count"], usize::from(pending));
        assert_eq!(report["issues"], json!(validation.issues));
        assert!(report["finalization"].is_null());
        assert!(report["draft_output"].is_null());
        assert!(report["owned_output"].is_null());
        assert_eq!(fs::read(temp.path().join("draft.json")).unwrap(), original);
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2);
    }
}

#[test]
fn complete_selection_writes_checked_draft_and_owned_request_with_provenance() {
    let temp = tempfile::tempdir().unwrap();
    let original = save(temp.path(), false);
    let report = successful(run(
        temp.path(),
        &[
            "--selection",
            "selection.json",
            "--owned-output",
            "request.json",
            "--draft-output",
            "checked.json",
        ],
    ));
    let checked = fs::read(temp.path().join("checked.json")).unwrap();
    assert_eq!(decode_draft(&checked, limits()).unwrap(), draft(false));
    assert_eq!(checked, original);
    let document = decode_owned(
        &fs::read(temp.path().join("request.json")).unwrap(),
        limits().input,
    )
    .unwrap();
    let OwnedDocument::Request(request) = document else {
        panic!("expected complete owned request");
    };
    assert_eq!(request.queries().input().game_version, namespace());
    let queries = &request.queries().input().requests;
    assert_eq!(queries.len(), 2);
    assert_eq!(queries[0].id, QueryId::new("z-first").unwrap());
    assert_eq!(queries[1].id, QueryId::new("a-second").unwrap());
    assert_eq!(report["finalization"]["status"], "ready");
    assert_eq!(report["finalization"]["selection"], json!(selection()));
    assert_eq!(
        report["finalization"]["draft_digest"],
        report["draft_digest"]
    );
    assert_eq!(report["finalization"]["request"], json!(request));
    let DraftFinalization::Ready(expected) = draft(false)
        .finalize_selection(selection(), limits())
        .unwrap()
    else {
        panic!("fixture must finalize");
    };
    assert_eq!(
        report["finalization"]["request_digest"],
        json!(expected.request_digest())
    );
    assert_eq!(fs::read(temp.path().join("draft.json")).unwrap(), original);
}

#[test]
fn pending_selection_preserves_every_query_and_can_persist_the_draft() {
    let temp = tempfile::tempdir().unwrap();
    let original = save(temp.path(), true);
    let report = successful(run(
        temp.path(),
        &[
            "--selection",
            "selection.json",
            "--draft-output",
            "checked.json",
        ],
    ));
    assert_eq!(report["finalization"]["status"], "pending");
    assert_eq!(report["finalization"]["selection"], json!(selection()));
    assert_eq!(report["finalization"]["issues"], report["issues"]);
    assert_eq!(
        report["finalization"]["queries"],
        json!(draft(true).input().query_presets.members[0].queries)
    );
    assert!(report["finalization"].get("request").is_none());
    let checked = fs::read(temp.path().join("checked.json")).unwrap();
    assert_eq!(decode_draft(&checked, limits()).unwrap(), draft(true));
    assert_eq!(checked, original);
}

#[test]
fn pending_owned_output_fails_before_either_requested_file_is_created() {
    let temp = tempfile::tempdir().unwrap();
    let original = save(temp.path(), true);
    let selected = fs::read(temp.path().join("selection.json")).unwrap();
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
    assert!(String::from_utf8_lossy(&output.stderr).contains("remains pending"));
    assert!(!temp.path().join("request.json").exists());
    assert!(!temp.path().join("checked.json").exists());
    assert_eq!(fs::read(temp.path().join("draft.json")).unwrap(), original);
    assert_eq!(
        fs::read(temp.path().join("selection.json")).unwrap(),
        selected
    );
}

#[test]
fn output_files_and_input_are_never_overwritten() {
    let temp = tempfile::tempdir().unwrap();
    let original = save(temp.path(), false);
    let output = run(temp.path(), &["--draft-output", "draft.json"]);
    assert!(!output.status.success());
    assert_eq!(fs::read(temp.path().join("draft.json")).unwrap(), original);
    fs::write(temp.path().join("request.json"), b"existing request").unwrap();
    let output = run(
        temp.path(),
        &[
            "--selection",
            "selection.json",
            "--owned-output",
            "request.json",
        ],
    );
    assert!(!output.status.success());
    assert_eq!(
        fs::read(temp.path().join("request.json")).unwrap(),
        b"existing request"
    );
}

#[test]
fn owned_output_requires_explicit_selection() {
    let temp = tempfile::tempdir().unwrap();
    save(temp.path(), false);
    let output = run(temp.path(), &["--owned-output", "request.json"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--selection"));
    assert!(!temp.path().join("request.json").exists());
}

#[test]
fn strict_selection_errors_precede_all_writes() {
    let selected = serde_json::to_string(&selection()).unwrap();
    let mut unknown = json!(selection());
    unknown["unexpected"] = json!(true);
    let duplicate = format!(
        "{{\"queries\":{},{}",
        serde_json::to_string(&selection().queries).unwrap(),
        &selected[1..]
    );
    let mut nested_unknown = json!(selection());
    nested_unknown["build"]["unexpected"] = json!(true);
    let mut absent = json!(selection());
    absent.as_object_mut().unwrap().remove("scenario");
    for invalid in [
        unknown.to_string(),
        duplicate,
        nested_unknown.to_string(),
        absent.to_string(),
    ] {
        let temp = tempfile::tempdir().unwrap();
        save(temp.path(), false);
        fs::write(temp.path().join("selection.json"), invalid).unwrap();
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
        assert!(!temp.path().join("request.json").exists());
        assert!(!temp.path().join("checked.json").exists());
    }
}

#[test]
fn oversized_selection_is_rejected_before_json_decode_or_output() {
    let temp = tempfile::tempdir().unwrap();
    save(temp.path(), false);
    fs::File::create(temp.path().join("selection.json"))
        .unwrap()
        .set_len(limits().input.max_wire_bytes as u64 + 1)
        .unwrap();
    let output = run(
        temp.path(),
        &[
            "--selection",
            "selection.json",
            "--draft-output",
            "checked.json",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("selection input exceeds"));
    assert!(!temp.path().join("checked.json").exists());
}

#[test]
fn strict_draft_envelope_errors_do_not_write_checked_output() {
    let original = encode_draft(&draft(false), limits()).unwrap();
    let mut unknown: Value = serde_json::from_slice(&original).unwrap();
    unknown["unexpected"] = json!(true);
    let duplicate = format!(
        "{{\"schema_version\":1,{}",
        &String::from_utf8(original).unwrap()[1..]
    );
    for invalid in [unknown.to_string(), duplicate] {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("draft.json"), &invalid).unwrap();
        let output = run(temp.path(), &["--draft-output", "checked.json"]);
        assert!(!output.status.success());
        assert!(!temp.path().join("checked.json").exists());
        assert_eq!(
            fs::read_to_string(temp.path().join("draft.json")).unwrap(),
            invalid
        );
    }
}
