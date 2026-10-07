//! Real published programs with explicit component facts. This does not resolve
//! providers, execute modifier transforms, or close the Player contribution set.
use super::{family, release};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::*,
    owned_rules::{ContributionKind, RuleEffectKind, RuleEntity},
    owned_schema::{SchemaDefinitionId, SchemaSubject},
};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, ProgramEvaluation, RuleFact, RuleScratch,
};
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use rayon::prelude::*;
use serde_json::{Value, json};
use std::path::Path;

fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(
        GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap(),
        format!("def.{n:016x}"),
    )
    .unwrap()
}
fn quantity(value: f64, unit: u64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, def(unit)).unwrap())
}
fn fact(read: &str, value: ParameterValue) -> RuleFact {
    RuleFact {
        read: read.parse().unwrap(),
        value,
    }
}
fn applied(report: &ProgramEvaluation, expected: f64, unit: u64) -> ParameterValue {
    assert_eq!(report.effects.len(), 1);
    let value = quantity(expected, unit);
    assert_eq!(
        report.effects[0].disposition,
        EffectDisposition::Applied {
            value: value.clone()
        }
    );
    value
}
struct Fixture<'a> {
    release: &'a StagedOwnedRelease,
    compiled: CompiledRulePackage,
    owner: SchemaSubject,
    contribution: StatDefId,
}
impl Fixture<'_> {
    fn evaluate(
        &self,
        program: &str,
        facts: &[RuleFact],
        scratch: &mut RuleScratch,
    ) -> ProgramEvaluation {
        let result = self
            .compiled
            .evaluate(
                &self.owner,
                &program.parse().unwrap(),
                facts,
                self.release.assembled().schema(),
                scratch,
            )
            .unwrap();
        let authored = self
            .release
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|r| r.owner == self.owner)
            .unwrap();
        assert_eq!(result.owner_programs_closure, authored.programs.closure);
        assert!(!authored.programs.is_complete());
        result
    }
    fn formatted(
        &self,
        raw: f64,
        magnitude: f64,
        corruption: f64,
        expected: f64,
        scratch: &mut RuleScratch,
    ) -> Vec<ProgramEvaluation> {
        let corrupted = self.evaluate(
            "corrupted-base-factor",
            &[fact("corrupted-base", quantity(corruption, 1))],
            scratch,
        );
        let scalar = self.evaluate(
            "ordered-magnitude-scalar",
            &[fact("scalar", quantity(magnitude, 1))],
            scratch,
        );
        let formatted = self.evaluate(
            "effective-amount",
            &[
                fact("component", quantity(raw, 2)),
                fact("corruption-factor", applied(&corrupted, corruption, 1)),
                fact("magnitude-factor", applied(&scalar, magnitude, 1)),
            ],
            scratch,
        );
        let delivered = self.evaluate(
            "contribute-player-global-energy-shield",
            &[fact("effective", applied(&formatted, expected, 2))],
            scratch,
        );
        applied(&delivered, expected, 2);
        assert!(
            matches!(&delivered.effects[0].effect, RuleEffectKind::Contribute { entity: RuleEntity::Player, stat, contribution: ContributionKind::Increase, .. } if *stat == self.contribution)
        );
        vec![corrupted, scalar, formatted, delivered]
    }
}

pub fn run(package: &Path) -> Value {
    let inventory = release::inventory(package);
    let staged = release::load(package);
    family::assert_endpoint(&staged);
    let compiled = CompiledRulePackage::compile(
        &staged.input().recipe.rules,
        staged.assembled().schema(),
        Default::default(),
    )
    .unwrap();
    let bindings = family::bindings();
    let fixture = Fixture {
        release: &staged,
        compiled,
        owner: SchemaSubject::Definition(bindings.modifier.address()),
        contribution: bindings.contribution,
    };
    let mut scratch = fixture.compiled.new_scratch();
    let vectors: Value = family::read("source-vectors.json");
    let report = &vectors["report"];
    let mut inputs = Vec::new();
    // Exact values from the three source occurrences; this intentionally says
    // nothing about whether the current build selected these occurrences.
    for build in report["builds"].as_array().unwrap() {
        for case in build["state"]["cases"].as_array().unwrap() {
            let raw = case["line"]
                .as_str()
                .unwrap()
                .split('%')
                .next()
                .unwrap()
                .parse::<f64>()
                .unwrap();
            inputs.push((raw, 1., 1., raw));
        }
    }
    assert_eq!(inputs.len(), 3);
    for case in report["builds"][0]["state"]["formats"].as_array().unwrap() {
        let raw = case["line"]
            .as_str()
            .unwrap()
            .split('%')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        inputs.push((
            raw,
            case["magnitude"].as_f64().unwrap(),
            case["corrupted_base"].as_f64().unwrap(),
            case["records"][0]["value"].as_f64().unwrap(),
        ));
    }
    assert_eq!(inputs.len(), 9);
    for name in ["integer-0", "integer-1000000"] {
        let control = report["builds"][0]["state"]["constructors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == name)
            .unwrap();
        let value = control["after"]["active"][0]["value"].as_f64().unwrap();
        inputs.push((value, 1., 1., value));
    }
    let serial: Vec<_> = inputs
        .iter()
        .map(|&(raw, magnitude, corruption, expected)| {
            fixture.formatted(raw, magnitude, corruption, expected, &mut scratch)
        })
        .collect();
    for count in [1, 4] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(count)
            .build()
            .unwrap();
        let parallel: Vec<_> = pool.install(|| {
            inputs
                .par_iter()
                .map_init(
                    || fixture.compiled.new_scratch(),
                    |s, &(raw, magnitude, corruption, expected)| {
                        fixture.formatted(raw, magnitude, corruption, expected, s)
                    },
                )
                .collect()
        });
        assert_eq!(parallel, serial);
    }
    // Missing demanded inputs and an unrelated invocation cannot leave a cached
    // known value in a reused worker's scratch.
    let missing = fixture.evaluate(
        "effective-amount",
        &[fact("component", quantity(44., 2))],
        &mut scratch,
    );
    assert!(matches!(
        missing.effects[0].disposition,
        EffectDisposition::Unresolved { .. }
    ));
    let (raw, magnitude, corruption, expected) = inputs[0];
    assert_eq!(
        fixture.formatted(raw, magnitude, corruption, expected, &mut scratch),
        serial[0]
    );
    let missing = fixture.evaluate("contribute-player-global-energy-shield", &[], &mut scratch);
    assert!(matches!(
        missing.effects[0].disposition,
        EffectDisposition::Unresolved { .. }
    ));

    // Parsed Global scope does not set a source property tag. Without an inline
    // property, Carapace quality has no effect, including negative quality.
    let owner = &family::extension().owners[0];
    let catalyst = owner
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == "catalyst-scalar")
        .unwrap();
    for amount in [20., -200.] {
        let facts: Vec<_> = catalyst
            .reads
            .iter()
            .map(|r| {
                let value = match r.id.as_str() {
                    "catalyst-kind" => ParameterValue::Option(def(0x09ee)),
                    "catalyst-amount" => quantity(amount, 2),
                    "unscalable" => ParameterValue::Boolean(false),
                    id if id.starts_with("property-") => ParameterValue::Boolean(false),
                    id => panic!("unexpected catalyst dependency {id}"),
                };
                RuleFact {
                    read: r.id.clone(),
                    value,
                }
            })
            .collect();
        applied(
            &fixture.evaluate("catalyst-scalar", &facts, &mut scratch),
            1.,
            1,
        );
    }
    assert_eq!(release::inventory(package), inventory);
    json!({"source_component_cases":inputs.len(),"programs":5,"rayon_threads":[1,4],"reused_scratch_missing_inputs":true,"provider_resolution":false,"modifier_transform_execution":false,"final_energy_shield":false,"full_build_parity":false})
}
