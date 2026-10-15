//! Exercise authored default-input rows through the normal normalization fixture.
//! Only definition identities are rebound; grammar, defaults, ranges, units and
//! encounter guards come from the files used by full release publication.
use super::*;
use serde::de::DeserializeOwned;
use std::{fs, path::PathBuf};

struct DataCase {
    directory: &'static str,
    source_name: &'static str,
    default: f64,
}
const INTERVAL: DataCase = DataCase {
    directory: "configuration-enemy-speed-inputs",
    source_name: "enemySpeed",
    default: 700.,
};
const CRITICAL_CHANCE: DataCase = DataCase {
    directory: "configuration-enemy-critical-chance-inputs",
    source_name: "enemyCritChance",
    default: 5.,
};
const CRITICAL_DAMAGE: DataCase = DataCase {
    directory: "configuration-enemy-critical-damage-inputs",
    source_name: "enemyCritDamage",
    default: 30.,
};

fn read<T: DeserializeOwned>(case: &DataCase, file: &str) -> T {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/owned/poe2/3887ae68")
        .join(case.directory)
        .join(file);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn adopted(case: &DataCase) -> (Fixture, NormalizationPolicy) {
    let mut f = fixture();
    let mut input: ConfigurationDefaultInput = read(case, "input.json");
    assert_eq!(input.source_name, case.source_name);
    let extension: Value = read(case, "extension.json");
    let dependencies: Value = read(case, "dependencies.json");
    let input_id = serde_json::to_value(&input.value_input).unwrap();
    let input_descriptor = extension["schema"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["value"]["value"]["id"] == input_id)
        .collect::<Vec<_>>();
    assert_eq!(input_descriptor.len(), 1);
    let mut input_schema: ExternalInputSchema =
        serde_json::from_value(input_descriptor[0]["value"]["value"]["schema"]["value"].clone())
            .unwrap();
    let ValueSchema::Quantity(range) = &mut input_schema.value else {
        panic!()
    };
    let unit_id = serde_json::to_value(range.minimum.unit()).unwrap();
    let unit_schema = extension["schema"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["value"]["value"]["id"] == unit_id)
        .map(|row| &row["value"]["value"]["schema"])
        .unwrap_or_else(|| {
            assert_eq!(dependencies["unit_descriptor"]["value"]["id"], unit_id);
            &dependencies["unit_descriptor"]["value"]["schema"]
        });
    range.minimum = FiniteQuantity::new(range.minimum.value(), f.unit.clone()).unwrap();
    range.maximum = FiniteQuantity::new(range.maximum.value(), f.unit.clone()).unwrap();
    let mut schema = f.a.schema.input().clone();
    for descriptor in &mut schema.definitions {
        match descriptor {
            DefinitionDescriptor::ExternalInput(row) if row.id == f.inputs[0].1 => {
                row.schema = SchemaState::Known(input_schema.clone());
            }
            DefinitionDescriptor::Unit(row) if row.id == f.unit => {
                row.schema = serde_json::from_value(unit_schema.clone()).unwrap();
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
    let mut rules: Vec<ConfigurationDispositionRule> = read(case, "accounting.json");
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

fn preserves_raw_inputs_and_overwrites_placeholders(case: &DataCase) {
    let (f, p) = adopted(case);
    let name = case.source_name;
    for (body, amount) in [
        (String::new(), case.default),
        (
            format!(r#"<Placeholder name="{name}" number="999"/>"#),
            case.default,
        ),
        (
            format!(r#"<Placeholder name="{name}" number="0"/>"#),
            case.default,
        ),
        (
            format!(
                r#"<Input name="{name}" number="0"/><Placeholder name="{name}" number="999"/>"#
            ),
            0.,
        ),
        (
            format!(
                r#"<Placeholder name="{name}" number="999"/><Input name="{name}" number="-17.25"/>"#
            ),
            -17.25,
        ),
        (format!(r#"<Input name="{name}" number="200"/>"#), 200.),
        (
            format!(r#"<Input name="{name}" number="1e6"/>"#),
            1_000_000.,
        ),
        (
            format!(r#"<Input name="{name}" number="-1000000"/>"#),
            -1_000_000.,
        ),
    ] {
        let source = source(&xml(&body), 0xd1);
        let after = normalize(&source, &f, &p, Default::default()).unwrap();
        assert_eq!(values(&after, 0), expected(&f, amount), "{body}");
        let mut unaccounted = p.clone();
        unaccounted.configuration_dispositions = None;
        let before = normalize(&source, &f, &unaccounted, Default::default()).unwrap();
        assert_eq!(before.draft(), after.draft());
        assert_eq!(before.allocator_after(), after.allocator_after());
        if body.contains("<Placeholder") {
            let leaf = accounting_tests::leaf(&source, &after, name, "Placeholder");
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
            let leaf = accounting_tests::leaf(&source, &after, name, "Input");
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

fn refuses_unproved_inputs(case: &DataCase) {
    let (f, p) = adopted(case);
    let name = case.source_name;
    let mut bodies = vec![
        format!(r#"<Input name="{name}" number="1000001"/>"#),
        format!(r#"<Input name="{name}" number="-1000001"/>"#),
        format!(r#"<Input name="{name}" number="NaN"/>"#),
        format!(r#"<Input name="{name}" string="0"/>"#),
        format!(r#"<Input name="{name}" number="0"/><Input name="{name}" number="1"/>"#),
        format!(
            r#"<Placeholder name="{name}" number="700"/><Placeholder name="{name}" number="700"/>"#
        ),
        format!(r#"<Placeholder name="{name}" string="700"/>"#),
        format!(r#"<Input name="{name}" number="0"/><Placeholder name="{name}" number="bad"/>"#),
    ];
    for guard in ["enemyIsBoss", "presetBossSkills", "enemySizePreset"] {
        for (tag, attribute, value) in [
            ("Input", "string", "None"),
            ("Input", "number", "0"),
            ("Input", "boolean", "false"),
            ("Placeholder", "string", "None"),
            ("Placeholder", "number", "0"),
        ] {
            bodies.push(format!(r#"<{tag} name="{guard}" {attribute}="{value}"/><Placeholder name="{name}" number="999"/>"#));
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
                        .is_some_and(|a| a.decoded() == name)
            }) {
                accounting_tests::pending(accounting_tests::leaf(&source, &after, name, tag));
            }
        }
    }
}

fn keeps_sets_and_dependencies_independent(case: &DataCase) {
    let (f, p) = adopted(case);
    let name = case.source_name;
    let source = source(
        &format!(
            r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1"><Input name="{name}" number="0"/></ConfigSet><ConfigSet id="2"><Placeholder name="{name}" number="999"/></ConfigSet></Config></PathOfBuilding2>"#
        ),
        0xd3,
    );
    let after = normalize(&source, &f, &p, Default::default()).unwrap();
    assert_eq!(values(&after, 0), expected(&f, 0.));
    assert_eq!(values(&after, 1), expected(&f, case.default));
    let mut stale = p.clone();
    let Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 { default_inputs, .. }) =
        &mut stale.configuration_inputs
    else {
        panic!()
    };
    default_inputs[0].constructor_default =
        ParameterValue::Quantity(FiniteQuantity::new(case.default + 1., f.unit.clone()).unwrap());
    assert!(normalize(&source, &f, &stale, Default::default()).is_err());
}

#[test]
fn enemy_interval_preserves_raw_inputs_and_overwrites_placeholders() {
    preserves_raw_inputs_and_overwrites_placeholders(&INTERVAL);
}
#[test]
fn enemy_interval_refuses_unproved_inputs() {
    refuses_unproved_inputs(&INTERVAL);
}
#[test]
fn enemy_interval_keeps_sets_and_dependencies_independent() {
    keeps_sets_and_dependencies_independent(&INTERVAL);
}
#[test]
fn enemy_critical_chance_preserves_raw_inputs_and_overwrites_placeholders() {
    preserves_raw_inputs_and_overwrites_placeholders(&CRITICAL_CHANCE);
}
#[test]
fn enemy_critical_chance_refuses_unproved_inputs() {
    refuses_unproved_inputs(&CRITICAL_CHANCE);
}
#[test]
fn enemy_critical_chance_keeps_sets_and_dependencies_independent() {
    keeps_sets_and_dependencies_independent(&CRITICAL_CHANCE);
}
#[test]
fn enemy_critical_damage_preserves_raw_inputs_and_overwrites_placeholders() {
    preserves_raw_inputs_and_overwrites_placeholders(&CRITICAL_DAMAGE);
}
#[test]
fn enemy_critical_damage_refuses_unproved_inputs() {
    refuses_unproved_inputs(&CRITICAL_DAMAGE);
}
#[test]
fn enemy_critical_damage_keeps_sets_and_dependencies_independent() {
    keeps_sets_and_dependencies_independent(&CRITICAL_DAMAGE);
}
