//! Configured Weakness is an encounter snapshot, not inferred hit history.
#[allow(dead_code)]
#[path = "support/owned_configuration_critical_weakness_activation.rs"]
mod activation;
#[allow(dead_code)]
#[path = "support/owned_configuration_default_sources.rs"]
mod configuration_sources;
#[allow(dead_code)]
#[path = "support/owned_configuration_default_stage.rs"]
mod configuration_stage;
#[allow(dead_code)]
#[path = "support/owned_configuration_default_publication.rs"]
mod publication;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::owned_build::ParameterValue;
use poe_optimizer_import::{
    owned_normalize::ConfigurationDefaultInput, owned_value::OwnedValueCodec,
};
use serde_json::{Value, json};

#[test]
fn configured_checkbox_has_exact_enemy_input_and_separate_flag_producer() {
    activation::check_authored();
}

#[test]
fn publication_requires_exactly_seven_artifact_pins_and_partial_scope() {
    let original: Value = activation::read("authoring.json");
    activation::check_authoring(&original);
    for mutation in 0..10 {
        let mut candidate = original.clone();
        match mutation {
            0 => {
                candidate["artifacts"].as_array_mut().unwrap().pop();
            }
            1 => {
                let row = candidate["artifacts"][0].clone();
                candidate["artifacts"].as_array_mut().unwrap().push(row);
            }
            2 => candidate["artifacts"][1] = candidate["artifacts"][0].clone(),
            3 => candidate["artifacts"].as_array_mut().unwrap().swap(0, 1),
            4 => candidate["artifacts"][0]["file"] = json!("unreviewed.json"),
            5 => candidate["artifacts"][0]["bytes"] = json!(0),
            6 => candidate["artifacts"][0]["sha256"] = json!("changed"),
            7 => candidate["preserve_owner_and_query_closure"] = json!(false),
            8 => candidate["source_vm_runs"] = json!(1),
            _ => candidate["before"] = json!("another-predecessor"),
        }
        assert!(std::panic::catch_unwind(|| activation::check_authoring(&candidate)).is_err());
    }
}

#[test]
fn only_exact_boolean_tokens_are_valid_source_values() {
    let input: ConfigurationDefaultInput = activation::read("input.json");
    let codec = OwnedValueCodec::new(input.recipe.codec, Default::default()).unwrap();
    for (text, expected) in [("true", true), ("false", false)] {
        assert_eq!(
            codec.decode(text).unwrap(),
            ParameterValue::Boolean(expected)
        );
    }
    for invalid in [
        "", "True", "FALSE", "1", "0", "nil", " true", "false ", "unknown",
    ] {
        assert!(codec.decode(invalid).is_err(), "{invalid:?}");
    }
    assert_eq!(activation::source_outputs(Some(false)), Some(json!([])));
    assert_eq!(activation::source_outputs(None), None);
    let emitted = activation::source_outputs(Some(true)).unwrap();
    assert_eq!(emitted.as_array().unwrap().len(), 1);
    assert_eq!(emitted[0]["name"], "Condition:ApplyCriticalWeakness");
    assert_eq!(emitted[0]["value"], true);
}

#[test]
fn changed_source_default_dispatch_or_capability_claim_is_refused() {
    let original: Value = activation::read("source-evidence.json");
    activation::authenticate_source(&original);
    for mutation in 0..9 {
        let mut proof = original.clone();
        match mutation {
            0 => proof["spans"][0]["body"] = json!("Player capability only"),
            1 => proof["constructor_dependencies"][1]["sha256"] = json!("changed"),
            2 => proof["scope"]["constructor"] = json!(true),
            3 => proof["scope"]["capability_implies_activation"] = json!(true),
            4 => proof["scope"]["participation_implies_hit"] = json!(true),
            5 => proof["scope"]["global_condition_absence"] = json!(true),
            6 => proof["derivation"]["true"][0]["name"] = json!("ApplyCriticalWeakness"),
            7 => proof["derivation"]["true"][0]["_positions"] = json!([]),
            _ => proof["derivation"]["false"] = json!([{"value":false}]),
        }
        assert!(std::panic::catch_unwind(|| activation::authenticate_source(&proof)).is_err());
    }
}

#[test]
fn retained_constructor_absence_is_distinct_from_a_false_callback() {
    let original = activation::observation();
    activation::check_observation(&original);
    for mutation in 0..7 {
        let mut observed = original.clone();
        match mutation {
            0 => observed["constructors"][0]["input"] = json!(false),
            1 => observed["constructors"][1]["default_state"] = json!(true),
            2 => observed["constructors"][0]["placeholder"] = json!(false),
            3 => observed["catalogue"][0]["apply"]["path"] = json!("@unreviewed.lua"),
            4 => {
                observed["catalogue"][0]["fields"]["ifFlag"] =
                    json!("Condition:ApplyCriticalWeakness")
            }
            5 => observed["calls"] = json!([{"argument_fields":{"val":false}}]),
            _ => {
                observed["constructors"].as_array_mut().unwrap().pop();
            }
        }
        assert!(std::panic::catch_unwind(|| activation::check_observation(&observed)).is_err());
    }
}

#[test]
fn exact_policy_scope_and_every_potential_flag_member_are_sealed() {
    let input: Value = activation::read("input.json");
    let definitions: Value = activation::read("definitions.json");
    let accounting: Value = activation::read("accounting.json");
    for mutation in 0..7 {
        let (mut i, mut d, mut a) = (input.clone(), definitions.clone(), accounting.clone());
        match mutation {
            0 => i["target"] = json!("player"),
            1 => i["constructor_default"]["value"] = json!(true),
            2 => i["recipe"]["tiers"][0]["selectors"][0]["lane"] = json!("input_number"),
            3 => i["recipe"]["tiers"][0]["duplicates"] = json!("first"),
            4 => d[0]["value"]["schema"]["value"]["targets"] = json!(["player_actor"]),
            5 => a[0]["destinations"] = json!([]),
            _ => i["recipe"]["codec"]["codec"]["value"]["tokens"][1]["value"] = json!(true),
        }
        assert!(std::panic::catch_unwind(|| activation::check_policy(&i, &d, &a)).is_err());
    }
    let original: Value = activation::read("consumer.json");
    for mutation in 0..9 {
        let mut c = original.clone();
        match mutation {
            0 => c["program"]["reads"][0]["source"]["value"]["entity"] = json!("player"),
            1 => c["program"]["effects"][0]
                .as_object_mut()
                .unwrap()
                .remove("when")
                .map(|_| ())
                .unwrap(),
            2 => c["program"]["nodes"][1]["expression"]["value"]["value"] = json!(false),
            3 => c["program"]["effects"][0]["effect"]["entity"] = json!("player"),
            4 => c["member"]["producer"]["origin"] = json!({"kind":"actor"}),
            5 => c["member"]["producer"]["effect"] = json!("critical-weakness"),
            6 => {
                c["member"]["producer"]["owner"]["value"]["value"]["key"] =
                    json!("def.0000000000000001")
            }
            7 => c["query"] = json!("outgoing-critical-chance-add"),
            _ => {
                c["program"]["reads"][0]["source"]["value"]["input"]["key"] =
                    json!("def.0000000000003390")
            }
        }
        assert!(std::panic::catch_unwind(|| activation::check_consumer(&c)).is_err());
    }
}

fn expectations() -> Vec<publication::TypedSourceExpectations> {
    use publication::{LeafRole::Contributes, Output, TypedControl, TypedSourceExpectations};
    vec![TypedSourceExpectations {
        source_name: activation::SOURCE.into(),
        outputs: vec![Output {
            policy_target: "enemy",
            draft_target: json!({"kind":"actor","value":{"kind":"scenario_enemy"}}),
            selection: None,
            ignore_placeholder: false,
        }],
        default: ParameterValue::Boolean(false),
        original_leaf: None,
        controls: [
            ("absent", None, None, Some(false)),
            ("true", Some(("boolean", "true")), None, Some(true)),
            ("false", Some(("boolean", "false")), None, Some(false)),
            ("capitalized", Some(("boolean", "True")), None, None),
            ("unknown", Some(("boolean", "unknown")), None, None),
            ("numeric-lane", Some(("number", "1")), None, None),
            ("string-lane", Some(("string", "true")), None, None),
            ("placeholder-only", None, Some("0"), None),
            (
                "losing-placeholder",
                Some(("boolean", "true")),
                Some("0"),
                None,
            ),
        ]
        .into_iter()
        .map(|(name, input, placeholder, result)| TypedControl {
            name,
            input,
            placeholder,
            values: result.map(|b| vec![ParameterValue::Boolean(b)]),
            roles: [
                (result.is_some() && input.is_some()).then_some(Contributes),
                None,
            ],
        })
        .collect(),
    }]
}

#[test]
fn typed_publication_expectation_preserves_false_and_exact_absence() {
    let inputs = vec![activation::read::<Value>("input.json")];
    assert_eq!(
        publication::checked_typed_group_ranges(&inputs, &expectations()),
        vec![0..1]
    );
    for mutation in 0..4 {
        let mut expected = expectations();
        match mutation {
            0 => expected[0].default = ParameterValue::Boolean(true),
            1 => expected[0].source_name = "different-source".into(),
            2 => expected[0].outputs.clear(),
            _ => expected.extend(expectations()),
        }
        assert!(
            std::panic::catch_unwind(|| publication::checked_typed_group_ranges(
                &inputs, &expected
            ))
            .is_err()
        );
    }
}

#[test]
#[ignore = "requires the authenticated published critical predecessor and fresh output"]
fn publish_critical_weakness_activation_preserving_all_five_originals() {
    let output: std::path::PathBuf =
        std::env::var_os("POE_OPTIMIZER_TEST_CRITICAL_WEAKNESS_ACTIVATION_OUTPUT")
            .expect("output")
            .into();
    publication::run_typed_sources(
        publication::Publication {
            directory: activation::directory(),
            stage: activation::stage,
            prior: std::env::var_os("POE_OPTIMIZER_TEST_CRITICAL_WEAKNESS_ACTIVATION_PRIOR")
                .expect("prior")
                .into(),
            output: output.clone(),
            original05_before: (16, 5, 11),
            new_definitions: 1,
            expectations: expectations(),
        },
        1,
        [106, 117, 109, 123, 4],
    );
    // Matching duplicates must remain unresolved too. Agreeing false values
    // are still two saved records, rather than evidence of an absent checkbox.
    let original = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap();
    assert_eq!(original.matches("</ConfigSet>").count(), 1);
    for (name, values) in [
        ("duplicate-true", ["true", "true"]),
        ("duplicate-false", ["false", "false"]),
        ("conflicting-booleans", ["false", "true"]),
    ] {
        let added = values
            .map(|value| {
                format!(
                    "<Input name=\"{}\" boolean=\"{value}\"/>",
                    activation::SOURCE
                )
            })
            .join("");
        let xml = original.replacen("</ConfigSet>", &format!("{added}</ConfigSet>"), 1);
        let source = output.join(format!("probe-{name}.xml"));
        std::fs::write(&source, xml).unwrap();
        let target = output.join(format!("probe-{name}"));
        publication::release::normalize(&output.join("package"), &source, 5, &target);
        let draft: Value =
            serde_json::from_slice(&std::fs::read(target.join("draft.json")).unwrap()).unwrap();
        let assumptions =
            draft["draft"]["scenario_presets"]["members"][0]["scenario"]["assumptions"]["members"]
                .as_array()
                .unwrap();
        assert!(
            assumptions
                .iter()
                .all(|row| row["input"]["value"]["key"] != activation::INPUT),
            "{name}: no constructor fallback for duplicate saved records"
        );
        let sidecar: Value =
            serde_json::from_slice(&std::fs::read(target.join("sidecar.json")).unwrap()).unwrap();
        let imported = poe_optimizer_import::build_instance::ImportedBuildInstance::from_decoded(
            poe_optimizer_import::decode_build(&std::fs::read(&source).unwrap()).unwrap(),
            poe_optimizer_core::build_identity::BuildLineage::from_bytes([151; 16]),
            Default::default(),
        )
        .unwrap();
        let evidence = poe_optimizer_import::owned_source::SourceProjectEvidence::collect(
            &imported,
            Default::default(),
        )
        .unwrap();
        let rows: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|r| {
                r.occurrence().name() == "Input"
                    && r.attribute("name")
                        .is_some_and(|a| a.decoded().ok() == Some(activation::SOURCE))
            })
            .collect();
        assert_eq!(rows.len(), 2);
        for row in rows {
            assert!(
                sidecar["origins"][row.occurrence().id().ordinal() as usize]["links"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|link| link["kind"] == "issue"),
                "{name}: both duplicate records remain unresolved"
            );
        }
    }
}
