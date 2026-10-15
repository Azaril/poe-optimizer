//! Exercise the authored game-data row through the existing normalization fixture.
//! Only definition identities are rebound; grammar, defaults, ranges and guards
//! come from the same files used by full release publication.
use super::*;
use serde::de::DeserializeOwned;
use std::{fs, path::PathBuf};

fn read<T: DeserializeOwned>(file: &str) -> T {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/owned/poe2/3887ae68/configuration-enemy-speed-inputs")
        .join(file);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn adopted() -> (Fixture, NormalizationPolicy) {
    let mut f = fixture();
    let mut input: ConfigurationDefaultInput = read("input.json");
    let extension: Value = read("extension.json");
    let mut input_schema: ExternalInputSchema =
        serde_json::from_value(extension["schema"][2]["value"]["value"]["schema"]["value"].clone())
            .unwrap();
    let ValueSchema::Quantity(range) = &mut input_schema.value else {
        panic!()
    };
    range.minimum = FiniteQuantity::new(range.minimum.value(), f.unit.clone()).unwrap();
    range.maximum = FiniteQuantity::new(range.maximum.value(), f.unit.clone()).unwrap();
    let mut schema = f.a.schema.input().clone();
    for descriptor in &mut schema.definitions {
        match descriptor {
            DefinitionDescriptor::ExternalInput(row) if row.id == f.inputs[0].1 => {
                row.schema = SchemaState::Known(input_schema.clone());
            }
            DefinitionDescriptor::Unit(row) if row.id == f.unit => {
                row.schema = serde_json::from_value(
                    extension["schema"][1]["value"]["value"]["schema"].clone(),
                )
                .unwrap();
            }
            _ => {}
        }
    }
    rebind(&mut f.a, schema);
    input.value_input = f.inputs[0].1.clone();
    input.recipe.codec.namespace = ns();
    let ValueCodecKind::Quantity { unit, .. } = &mut input.recipe.codec.codec else {
        panic!()
    };
    *unit = f.unit.clone();
    let ParameterValue::Quantity(default) = input.constructor_default else {
        panic!()
    };
    input.constructor_default =
        ParameterValue::Quantity(FiniteQuantity::new(default.value(), f.unit.clone()).unwrap());
    let mut p = reviewed(&f);
    let dependencies: Value = read("dependencies.json");
    let Some(EncounterPolicy::PobFreshDefaultConfigEncounterV1 {
        absent_input_names, ..
    }) = &mut p.encounter
    else {
        panic!()
    };
    *absent_input_names =
        serde_json::from_value(dependencies["encounter_policy"]["absent_input_names"].clone())
            .unwrap();
    p.configuration_inputs = Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 {
        mapping_source: *f.a.mapping.source_identity(),
        encounter: f.encounter.clone(),
        inputs: vec![],
        placeholder_fallback_inputs: vec![],
        option_inputs: vec![],
        default_inputs: vec![input],
    });
    let mut rules: Vec<ConfigurationDispositionRule> = read("accounting.json");
    for rule in &mut rules {
        for destination in &mut rule.destinations {
            let ConfigurationDestination::Adapter {
                control: ConfigurationAdapterControl::ScalarInput { input },
            } = destination
            else {
                panic!()
            };
            *input = f.inputs[0].1.clone();
        }
    }
    let p = accounting_tests::bind(&f, p, rules);
    (f, p)
}

fn expected(f: &Fixture, n: f64) -> Vec<(ExternalInputDefId, ParameterValue)> {
    vec![(
        f.inputs[0].1.clone(),
        ParameterValue::Quantity(FiniteQuantity::new(n, f.unit.clone()).unwrap()),
    )]
}

#[test]
fn enemy_speed_data_preserves_zero_and_overwrites_saved_placeholders() {
    let (f, p) = adopted();
    for (body, amount) in [
        ("", 700.),
        (r#"<Placeholder name="enemySpeed" number="999"/>"#, 700.),
        (r#"<Placeholder name="enemySpeed" number="0"/>"#, 700.),
        (
            r#"<Input name="enemySpeed" number="0"/><Placeholder name="enemySpeed" number="999"/>"#,
            0.,
        ),
        (
            r#"<Placeholder name="enemySpeed" number="999"/><Input name="enemySpeed" number="-17.25"/>"#,
            -17.25,
        ),
        (r#"<Input name="enemySpeed" number="1e6"/>"#, 1_000_000.),
        (
            r#"<Input name="enemySpeed" number="-1000000"/>"#,
            -1_000_000.,
        ),
    ] {
        let source = source(&xml(body), 0xd1);
        let after = normalize(&source, &f, &p, Default::default()).unwrap();
        assert_eq!(values(&after, 0), expected(&f, amount), "{body}");
        let mut unaccounted = p.clone();
        unaccounted.configuration_dispositions = None;
        let before = normalize(&source, &f, &unaccounted, Default::default()).unwrap();
        assert_eq!(before.draft(), after.draft());
        assert_eq!(before.allocator_after(), after.allocator_after());
        if body.contains("<Placeholder") {
            let leaf = accounting_tests::leaf(&source, &after, "enemySpeed", "Placeholder");
            assert_eq!(
                leaf.disposition,
                SourceDisposition::SourceOnly(key("unused-config-placeholder"))
            );
            assert_eq!(
                leaf.links,
                vec![OwnedOriginTarget::ScenarioPreset(
                    after.draft().input().scenario_presets.members[0].id
                )]
            );
        }
        if body.contains("<Input") {
            let leaf = accounting_tests::leaf(&source, &after, "enemySpeed", "Input");
            assert_eq!(leaf.disposition, SourceDisposition::Contributes);
            assert!(
                leaf.links
                    .iter()
                    .all(|l| !matches!(l, OwnedOriginTarget::Issue(_)))
            );
        }
        let scenario = &after.draft().input().scenario_presets.members[0].scenario;
        assert!(matches!(
            scenario.assumptions.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(matches!(
            scenario.usage.completion,
            DraftListCompletion::Pending { .. }
        ));
        let repeated = normalize(&source, &f, &p, Default::default()).unwrap();
        assert_eq!(after.draft(), repeated.draft());
        assert_eq!(
            serde_json::to_value(after.sidecar()).unwrap(),
            serde_json::to_value(repeated.sidecar()).unwrap()
        );
    }
}

#[test]
fn enemy_speed_data_refuses_ambiguous_malformed_and_unproved_encounters() {
    let (f, p) = adopted();
    let mut bodies = vec![
        r#"<Input name="enemySpeed" number="1000001"/>"#.to_owned(),
        r#"<Input name="enemySpeed" number="NaN"/>"#.to_owned(),
        r#"<Input name="enemySpeed" string="0"/>"#.to_owned(),
        r#"<Input name="enemySpeed" number="0"/><Input name="enemySpeed" number="1"/>"#.to_owned(),
        r#"<Placeholder name="enemySpeed" number="700"/><Placeholder name="enemySpeed" number="700"/>"#.to_owned(),
        r#"<Placeholder name="enemySpeed" string="700"/>"#.to_owned(),
        r#"<Input name="enemySpeed" number="0"/><Placeholder name="enemySpeed" number="bad"/>"#.to_owned(),
    ];
    for name in ["enemyIsBoss", "presetBossSkills", "enemySizePreset"] {
        for (tag, attribute, value) in [
            ("Input", "string", "None"),
            ("Input", "number", "0"),
            ("Input", "boolean", "false"),
            ("Placeholder", "string", "None"),
            ("Placeholder", "number", "0"),
        ] {
            bodies.push(format!(r#"<{tag} name="{name}" {attribute}="{value}"/><Placeholder name="enemySpeed" number="999"/>"#));
        }
    }
    for body in bodies {
        let source = source(&xml(&body), 0xd2);
        let after = normalize(&source, &f, &p, Default::default()).unwrap();
        assert!(values(&after, 0).is_empty(), "{body}");
        for tag in ["Input", "Placeholder"] {
            if source.occurrences().iter().any(|r| {
                r.name() == tag
                    && source
                        .attribute(r.id(), "name")
                        .unwrap()
                        .is_some_and(|a| a.decoded() == "enemySpeed")
            }) {
                accounting_tests::pending(accounting_tests::leaf(
                    &source,
                    &after,
                    "enemySpeed",
                    tag,
                ));
            }
        }
    }
}

#[test]
fn enemy_speed_data_keeps_configuration_sets_independent_and_dependencies_checked() {
    let (f, p) = adopted();
    let source = source(
        r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1"><Input name="enemySpeed" number="0"/></ConfigSet><ConfigSet id="2"><Placeholder name="enemySpeed" number="999"/></ConfigSet></Config></PathOfBuilding2>"#,
        0xd3,
    );
    let after = normalize(&source, &f, &p, Default::default()).unwrap();
    assert_eq!(values(&after, 0), expected(&f, 0.));
    assert_eq!(values(&after, 1), expected(&f, 700.));
    let mut stale = p.clone();
    let Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 { default_inputs, .. }) =
        &mut stale.configuration_inputs
    else {
        panic!()
    };
    default_inputs[0].constructor_default =
        ParameterValue::Quantity(FiniteQuantity::new(701., f.unit.clone()).unwrap());
    assert!(normalize(&source, &f, &stale, Default::default()).is_err());
}
