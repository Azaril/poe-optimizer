//! Directly authored owned packages; no source adapter, profile, or explicit fact probe.
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_content::digest_owned, owned_definitions::*,
    owned_routing::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::{
    OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, OwnedSchemaLimits,
    SchemaPackageInput, encode_schema_package,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("effects-cli", "v1").unwrap()
}
fn id<K: DefinitionDomain>(v: &str) -> DefId<K> {
    DefId::parse(namespace(), v).unwrap()
}
fn empty<T>() -> DeclaredSet<T> {
    DeclaredSet::complete(vec![])
}
fn ports() -> DeclaredSlots {
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
fn range() -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(1).unwrap(),
        maximum: BoundedInteger::new(100).unwrap(),
    }
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
struct Fixture {
    request: OwnedEvaluationRequest,
    schema: OwnedDefinitionSchemaPackage,
    rules: RulePackageInput,
    routing: ActionRoutingInput,
}
fn fixture(offset: i64) -> Fixture {
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: namespace(),
            release: key("fixture"),
            semantics_version: key("fixture-v1"),
            definitions: vec![
                DefinitionDescriptor::Class(known(
                    id("class"),
                    ClassSchema {
                        level: range(),
                        ascendancies: empty(),
                        declarations: ports(),
                    },
                )),
                DefinitionDescriptor::Encounter(known(
                    id("encounter"),
                    EncounterSchema {
                        enemy_level: range(),
                        external_inputs: empty(),
                    },
                )),
                DefinitionDescriptor::Stat(known(
                    id("level-plus"),
                    StatSchema {
                        value: ComputedValueType::Integer,
                        targets: vec![RuleEntityKind::Actor],
                    },
                )),
            ],
            slots: vec![],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let limits = OwnedInputLimits::default();
    let lineage = BuildLineage::from_bytes([82; 16]);
    let loadout = WeaponLoadoutId::from_instance_id(InstanceId::from_parts(lineage, 1).unwrap());
    let build = BuildSpec::new(
        BuildInput {
            allocator: InstanceAllocatorState::from_parts(lineage, 1),
            revision: BuildRevision::from_u64(1),
            game_version: namespace(),
            character: CharacterSpec {
                class: id("class"),
                ascendancy: None,
                level: 20,
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
    .unwrap();
    let scenario = ScenarioSpec::new(
        ScenarioInput {
            game_version: namespace(),
            enemy: EnemySpec {
                encounter: id("encounter"),
                level: 20,
            },
            assumptions: vec![],
            usage: vec![],
        },
        limits,
    )
    .unwrap();
    let queries = QuerySpec::new(
        QueryInput {
            game_version: namespace(),
            requests: vec![],
        },
        limits,
    )
    .unwrap();
    let request = OwnedEvaluationRequest::new(build, scenario, queries, limits).unwrap();
    let rules = RulePackageInput {
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: namespace(),
        release: key("fixture"),
        semantics_version: key("fixture-v1"),
        operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
        definitions: schema.identity().clone(),
        owners: vec![
            DefinitionRules {
                owner: SchemaSubject::Definition(id::<ClassDefinition>("class").address()),
                programs: DeclaredSet::complete(vec![RuleProgram {
                    id: key("level"),
                    context: RuleEntityKind::Actor,
                    reads: vec![RuleRead {
                        id: key("level"),
                        value_type: ComputedValueType::Integer,
                        source: RuleReadSource::CharacterLevel,
                    }],
                    nodes: vec![
                        RuleNode {
                            id: key("input"),
                            expression: RuleExpression::Read {
                                input: key("level"),
                            },
                        },
                        RuleNode {
                            id: key("offset"),
                            expression: RuleExpression::Literal {
                                value: ParameterValue::Integer(
                                    BoundedInteger::new(offset).unwrap(),
                                ),
                            },
                        },
                        RuleNode {
                            id: key("result"),
                            expression: RuleExpression::Add {
                                left: key("input"),
                                right: key("offset"),
                            },
                        },
                    ],
                    effects: vec![RuleEffect {
                        id: key("derived"),
                        when: None,
                        effect: RuleEffectKind::Derive {
                            entity: RuleEntity::Current,
                            stat: id("level-plus"),
                            value: key("result"),
                        },
                    }],
                }]),
            },
            DefinitionRules {
                owner: SchemaSubject::Definition(id::<EncounterDefinition>("encounter").address()),
                programs: empty(),
            },
        ],
    };
    let routing = ActionRoutingInput {
        schema_version: OWNED_ACTION_ROUTING_VERSION,
        namespace: namespace(),
        release: key("fixture"),
        definitions: schema.identity().clone(),
        outputs: vec![],
    };
    Fixture {
        request,
        schema,
        rules,
        routing,
    }
}
fn save(dir: &Path, f: &Fixture) {
    fs::write(
        dir.join("request.json"),
        encode_owned(
            &OwnedDocument::Request(Box::new(f.request.clone())),
            OwnedInputLimits::default(),
        )
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("schema.json"),
        encode_schema_package(&f.schema, OwnedSchemaLimits::default()).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("rules.json"),
        serde_json::to_vec(&f.rules).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("routing.json"),
        serde_json::to_vec(&f.routing).unwrap(),
    )
    .unwrap();
}
fn args() -> Vec<&'static str> {
    vec![
        "resolve-owned-effects",
        "--input",
        "request.json",
        "--schema",
        "schema.json",
        "--rules",
        "rules.json",
        "--routing",
        "routing.json",
    ]
}
fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(dir)
        .args(args)
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
#[test]
fn explicit_owned_files_resolve_real_request_values_and_preserve_exact_bindings() {
    for (offset, expected) in [(2, 22), (5, 25)] {
        let dir = tempfile::tempdir().unwrap();
        let f = fixture(offset);
        save(dir.path(), &f);
        let mut command = args();
        command.extend(["--output", "report.json"]);
        let output = run(dir.path(), &command);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read(dir.path().join("report.json")).unwrap(),
            output.stdout
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["document_kind"], "owned_effects_report");
        assert_eq!(report["verification"]["scope"], "owned_effect_component");
        assert_eq!(report["verification"]["metric_conversion"], "not_run");
        assert_eq!(
            report["verification"]["whole_build_parity"],
            "not_established"
        );
        assert_eq!(
            report["bindings"]["request"],
            serde_json::to_value(
                digest_owned(
                    "owned-request-v1",
                    &f.request,
                    OwnedInputLimits::default().max_wire_bytes
                )
                .unwrap()
            )
            .unwrap()
        );
        assert_eq!(
            report["bindings"]["definitions"],
            serde_json::to_value(f.schema.identity()).unwrap()
        );
        assert_eq!(report["resolution"]["gaps"], json!([]));
        let values = report["resolution"]["values"].as_array().unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(
            values[0]["value"],
            json!({"status":"known","value":ParameterValue::Integer(BoundedInteger::new(expected).unwrap())})
        );
        assert_eq!(report["resolution"]["effects"].as_array().unwrap().len(), 1);
        assert!(report.get("metrics").is_none());
        let before = fs::read(dir.path().join("report.json")).unwrap();
        assert!(!run(dir.path(), &command).status.success());
        assert_eq!(before, fs::read(dir.path().join("report.json")).unwrap());
    }
}
#[test]
fn partial_program_membership_keeps_effect_evidence_and_withholds_final_values() {
    let dir = tempfile::tempdir().unwrap();
    let mut f = fixture(2);
    let owner = &mut f.rules.owners[0];
    owner.programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner.owner.clone(),
            facet: SchemaFacet::GameRules,
            code: key("unconverted"),
        }],
    };
    save(dir.path(), &f);
    let report = success(run(dir.path(), &args()));
    assert!(!report["resolution"]["gaps"].as_array().unwrap().is_empty());
    assert_eq!(
        report["resolution"]["effects"][0]["value"]["status"],
        "known"
    );
    assert_eq!(
        report["resolution"]["values"][0]["value"]["status"],
        "unresolved"
    );
    assert_eq!(
        report["resolution"]["values"][0]["value"]["reason"],
        "incomplete_contributors"
    );
}
#[test]
fn invalid_or_stale_artifacts_never_publish_a_report() {
    for bad in [
        "rules-binding",
        "routing-binding",
        "old-operations",
        "unknown-field",
        "not-request",
        "oversized",
        "cycle",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let f = fixture(2);
        save(dir.path(), &f);
        match bad {
            "rules-binding" | "routing-binding" | "old-operations" | "unknown-field" => {
                let file = if bad == "routing-binding" {
                    "routing.json"
                } else {
                    "rules.json"
                };
                let mut input: Value =
                    serde_json::from_slice(&fs::read(dir.path().join(file)).unwrap()).unwrap();
                match bad {
                    "rules-binding" | "routing-binding" => {
                        input["definitions"]["content_sha256"] = json!("a".repeat(64))
                    }
                    "old-operations" => {
                        input["operations_version"] = json!("owned-domain-operations-v2")
                    }
                    _ => input["unexpected"] = json!(true),
                }
                fs::write(dir.path().join(file), serde_json::to_vec(&input).unwrap()).unwrap();
            }
            "not-request" => fs::write(
                dir.path().join("request.json"),
                encode_owned(
                    &OwnedDocument::Build(Box::new(f.request.build().clone())),
                    OwnedInputLimits::default(),
                )
                .unwrap(),
            )
            .unwrap(),
            "oversized" => fs::File::create(dir.path().join("request.json"))
                .unwrap()
                .set_len(OwnedInputLimits::default().max_wire_bytes as u64 + 1)
                .unwrap(),
            "cycle" => {
                let mut rules = f.rules.clone();
                rules.owners[0].programs.members[0].nodes.push(RuleNode {
                    id: key("cycle"),
                    expression: RuleExpression::Not {
                        value: key("cycle"),
                    },
                });
                fs::write(
                    dir.path().join("rules.json"),
                    serde_json::to_vec(&rules).unwrap(),
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        let mut command = args();
        command.extend(["--output", "report.json"]);
        assert!(!run(dir.path(), &command).status.success(), "{bad}");
        assert!(!dir.path().join("report.json").exists(), "{bad}");
    }
}
#[test]
fn help_and_required_flags_have_no_implicit_artifact_or_source_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let output = run(dir.path(), &["resolve-owned-effects", "--help"]);
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for flag in ["--input", "--schema", "--rules", "--routing", "--output"] {
        assert!(help.contains(flag), "{help}");
    }
    assert!(!help.contains("--pob"));
    for missing in ["--input", "--schema", "--rules", "--routing"] {
        let mut command = args();
        let index = command.iter().position(|v| *v == missing).unwrap();
        command.drain(index..index + 2);
        let output = run(dir.path(), &command);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(missing));
    }
}
