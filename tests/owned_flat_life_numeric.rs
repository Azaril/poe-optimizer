//! Native replay of captured Life numeric boundaries, including a retained
//! source-text discrepancy. This is component evidence, not complete coverage.
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::FiniteQuantity, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, RuleFact, RuleScratch,
};
use rayon::prelude::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn vectors() -> Value {
    serde_json::from_slice(
        &fs::read(root().join("tests/fixtures/owned-flat-life-numeric.json")).unwrap(),
    )
    .unwrap()
}
struct Fixture {
    schema: OwnedDefinitionSchemaPackage,
    compiled: CompiledRulePackage,
    owner: SchemaSubject,
    program: RuleProgram,
}
impl Fixture {
    fn new() -> Self {
        let mut bytes = vec![];
        flate2::read::GzDecoder::new(
            fs::File::open(root().join("tests/fixtures/owned-sniper-replay.json.gz")).unwrap(),
        )
        .read_to_end(&mut bytes)
        .unwrap();
        let replay: Value = serde_json::from_slice(&bytes).unwrap();
        let schema = OwnedDefinitionSchemaPackage::new(
            serde_json::from_value(replay["schema"].clone()).unwrap(),
            Default::default(),
        )
        .unwrap();
        let rules: RulePackageInput = serde_json::from_value(replay["rules"].clone()).unwrap();
        let compiled = CompiledRulePackage::compile(&rules, &schema, Default::default()).unwrap();
        let owner = rules
            .owners
            .iter()
            .find(|o| serde_json::json!(o.owner)["value"]["value"]["key"] == "def.0000000000003100")
            .unwrap();
        let program = owner
            .programs
            .members
            .iter()
            .find(|p| p.id.as_str() == "effective-amount")
            .unwrap()
            .clone();
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(&program).unwrap())
            ),
            vectors()["program_sha256"]
        );
        assert_eq!(program.reads.len(), 3);
        Self {
            schema,
            compiled,
            owner: owner.owner.clone(),
            program,
        }
    }
    fn facts(&self, c: &Value) -> Vec<RuleFact> {
        self.program
            .reads
            .iter()
            .map(|r| {
                let field = match r.id.as_str() {
                    "component" => "raw",
                    "corruption-factor" => "corruption",
                    "magnitude-factor" => "magnitude",
                    _ => panic!(),
                };
                let ComputedValueType::Quantity { unit } = &r.value_type else {
                    panic!()
                };
                RuleFact {
                    read: r.id.clone(),
                    value: ParameterValue::Quantity(
                        FiniteQuantity::new(c[field].as_f64().unwrap(), unit.clone()).unwrap(),
                    ),
                }
            })
            .collect()
    }
    fn run(&self, facts: &[RuleFact], scratch: &mut RuleScratch) -> EffectDisposition {
        let result = self
            .compiled
            .evaluate(&self.owner, &self.program.id, facts, &self.schema, scratch)
            .unwrap();
        assert_eq!(result.effects.len(), 1);
        result.effects[0].disposition.clone()
    }
    fn check(&self, c: &Value, scratch: &mut RuleScratch) {
        let actual = self.run(&self.facts(c), scratch);
        let EffectDisposition::Applied {
            value: ParameterValue::Quantity(value),
        } = actual
        else {
            panic!("{actual:?}")
        };
        assert_eq!(
            value.value(),
            c["expected_native"].as_f64().unwrap(),
            "{}",
            c["name"]
        );
    }
}

#[test]
fn released_life_program_preserves_numeric_results_without_source_text_round_trip() {
    let f = Fixture::new();
    let v = vectors();
    let cases = v["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 17);
    assert_eq!(v["numeric_domain_closed"], false);
    assert_eq!(v["whole_build_parity"], false);
    let differences: Vec<_> = cases
        .iter()
        .filter(|c| c["value"] != c["expected_native"])
        .collect();
    assert_eq!(differences.len(), 1);
    assert_eq!(differences[0]["name"], "large-magnitude");
    assert_eq!(differences[0]["expected_native"], 1_000_997_998_001_001_u64);
    assert_eq!(differences[0]["value"], 1_000_997_998_001_000_u64);
    let mut scratch = f.compiled.new_scratch();
    for c in cases {
        f.check(c, &mut scratch);
        f.check(c, &mut f.compiled.new_scratch());
        let facts = f.facts(c);
        for missing in 0..facts.len() {
            let mut partial = facts.clone();
            partial.remove(missing);
            assert!(matches!(
                f.run(&partial, &mut scratch),
                EffectDisposition::Unresolved { .. }
            ));
            f.check(c, &mut scratch);
        }
    }
    for c in cases.iter().rev() {
        f.check(c, &mut scratch);
    }
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..4).into_par_iter().for_each(|offset| {
                let mut scratch = f.compiled.new_scratch();
                for i in 0..cases.len() {
                    f.check(&cases[(i + offset) % cases.len()], &mut scratch);
                }
            });
        });
}

#[test]
fn retained_numeric_evidence_matches_its_source_witness() {
    let v = vectors();
    for (field, path) in [
        (
            "witness_sha256",
            "crates/poe-optimizer-pob/tests/owned_flat_life_numeric.rs",
        ),
        (
            "driver_sha256",
            "crates/poe-optimizer-pob/tests/support/player_resource_source.rs",
        ),
    ] {
        let bytes = fs::read(root().join(path)).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), v[field]);
    }
}
