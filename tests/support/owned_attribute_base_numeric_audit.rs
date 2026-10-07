//! Counterexamples, not permission to publish BASE membership. These component
//! checks retain every production closure. Direct facts do not establish item
//! admission, source ordering, or complete-build coverage. In particular, a
//! positive passive *suffix* argument does not apply to source paths with later
//! bonus contributions (the source audit found +8 Strength/+7 Intelligence).
use super::release;
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::*,
    owned_schema::{SchemaDefinitionId, SchemaSubject},
};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, NumericalFailure, RuleFact,
};
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use serde_json::Value;
use std::path::PathBuf;

fn fold(prefix: f64, values: &[f64]) -> f64 {
    // Explicit IEEE-754 left fold, not a substitute native contribution engine.
    values.iter().fold(prefix, |sum, value| sum + value)
}

#[test]
fn integral_donors_do_not_authorize_reordering_signed_base_contributions() {
    let x = 9_007_199_254_740_000.0;
    assert!(x <= BoundedInteger::MAX as f64);
    assert_eq!(fold(7.0, &[x, x, -x, -x]), 8.0);
    assert_eq!(fold(7.0, &[x, -x, x, -x]), 7.0);
}

#[test]
fn positive_suffix_permutations_do_not_prove_base_subtotal_equality() {
    let prefix = 9_007_199_254_740_994.0;
    let a = fold(prefix, &[3.0, 5.0, 25.0]);
    let b = fold(prefix, &[5.0, 3.0, 25.0]);
    assert_eq!(a, 9_007_199_254_741_024.0);
    assert_eq!(b, 9_007_199_254_741_028.0);
    assert!(a > BoundedInteger::MAX as f64);
    assert!(b > BoundedInteger::MAX as f64);
    // A later signed contribution exposes this previously out-of-range
    // difference as small valid totals. Thus suffix really must mean suffix.
    let later = -9_007_199_254_740_000.0;
    assert_eq!(a + later, 1024.0);
    assert_eq!(b + later, 1028.0);
}

fn def<K: DefinitionDomain>(number: u64) -> DefId<K> {
    DefId::parse(
        GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap(),
        format!("def.{number:016x}"),
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
fn applied_quantity(value: EffectDisposition) -> f64 {
    let EffectDisposition::Applied {
        value: ParameterValue::Quantity(value),
    } = value
    else {
        panic!("expected applied Quantity, got {value:?}")
    };
    value.value()
}
fn applied_integer(value: EffectDisposition) -> i64 {
    let EffectDisposition::Applied {
        value: ParameterValue::Integer(value),
    } = value
    else {
        panic!("expected applied Integer, got {value:?}")
    };
    value.get()
}

struct Fixture {
    release: StagedOwnedRelease,
    compiled: CompiledRulePackage,
}
impl Fixture {
    fn evaluate(
        &self,
        owner: SchemaSubject,
        program: &str,
        facts: &[RuleFact],
    ) -> EffectDisposition {
        let owner_rules = self
            .release
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|row| row.owner == owner)
            .unwrap();
        let output = self
            .compiled
            .evaluate(
                &owner,
                &program.parse().unwrap(),
                facts,
                self.release.assembled().schema(),
                &mut self.compiled.new_scratch(),
            )
            .unwrap();
        assert_eq!(output.owner_programs_closure, owner_rules.programs.closure);
        let first = output.effects.first().unwrap().disposition.clone();
        assert!(output.effects.iter().all(|row| row.disposition == first));
        first
    }
    fn modifier(&self, program: &str, facts: &[RuleFact]) -> EffectDisposition {
        self.evaluate(
            SchemaSubject::Definition(def::<ModifierDefinition>(0x295c).address()),
            program,
            facts,
        )
    }
    fn attribute(&self, base: f64, increase: f64) -> EffectDisposition {
        self.evaluate(
            SchemaSubject::Definition(def::<StatDefinition>(0x3321).address()),
            "strength-first-step",
            &[
                fact("base", quantity(base, 0x295a)),
                fact("increase", quantity(increase, 2)),
                fact("effective-more", quantity(1.0, 1)),
            ],
        )
    }
}

fn assert_range(schema: &Value, slot: u64, minimum: f64, maximum: f64) {
    let name = format!("def.{slot:016x}");
    let row = schema["slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["value"]["id"]["slot"]["key"] == name)
        .unwrap();
    let domain = &row["value"]["schema"]["value"]["value"];
    assert_eq!(domain["kind"], "quantity");
    assert_eq!(domain["value"]["minimum"]["value"], minimum);
    assert_eq!(domain["value"]["maximum"]["value"], maximum);
}

#[test]
#[ignore = "requires verified ATTRIBUTE_BASE_RELEASE; direct native component facts only"]
fn current_item_programs_and_attribute_receiver_reproduce_numeric_order_limits() {
    let path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ATTRIBUTE_BASE_RELEASE")
            .expect("current checked increased-attribute release"),
    );
    let inventory = release::inventory(&path);
    let staged = release::load(&path);
    assert_eq!(
        staged.receipt().input.to_string(),
        "29e4e9ef2b321bb6c802d9970666e8bf85afb805830ca9c2bbfc03bceaafb1ef"
    );
    let schema = serde_json::to_value(&staged.input().recipe.schema).unwrap();
    for raw in [0x295d, 0x2975, 0x298d, 0x29a5] {
        assert_range(&schema, raw, -1_000_000.0, 1_000_000.0);
    }
    for corruption in [0x2973, 0x298b, 0x29a3, 0x29bb] {
        assert_range(&schema, corruption, 0.0, 1_000_000.0);
    }
    assert_range(&schema, 0x09fa, -1_000_000.0, 1_000_000.0);
    let rules = &staged.input().recipe.rules;
    let queries = rules.contribution_queries.as_ref().unwrap();
    assert!(!queries.is_complete());
    for query in &queries.members {
        if query.id.as_str().ends_with("-base") {
            assert!(
                query
                    .groups
                    .iter()
                    .all(|group| !group.members.is_complete())
            );
        }
    }
    let json_rules = serde_json::to_value(rules).unwrap();
    let mut sums = [0.0; 3];
    let mut count = 0;
    for owner in json_rules["owners"].as_array().unwrap() {
        if owner["owner"]["value"]["kind"] != "passive_node" {
            continue;
        }
        for program in owner["programs"]["members"].as_array().unwrap() {
            for effect in program["effects"].as_array().unwrap() {
                let effect = &effect["effect"];
                if effect["kind"] != "contribute" || effect["contribution"] != "add" {
                    continue;
                }
                let Some(index) = ["331b", "331c", "331d"].iter().position(|suffix| {
                    effect["stat"]["key"]
                        .as_str()
                        .is_some_and(|key| key.ends_with(suffix))
                }) else {
                    continue;
                };
                let node = program["nodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|node| node["id"] == effect["value"])
                    .unwrap();
                assert_eq!(node["expression"]["kind"], "literal");
                assert_eq!(node["expression"]["value"]["kind"], "quantity");
                let value = node["expression"]["value"]["value"]["value"]
                    .as_f64()
                    .unwrap();
                assert!([3.0, 4.0, 5.0, 8.0, 10.0, 12.0, 25.0].contains(&value));
                sums[index] += value;
                count += 1;
            }
        }
    }
    assert_eq!(count, 962);
    assert_eq!(sums, [1639.0, 1627.0, 1663.0]);
    let compiled =
        CompiledRulePackage::compile(rules, staged.assembled().schema(), Default::default())
            .unwrap();
    let fixture = Fixture {
        release: staged,
        compiled,
    };
    let magnitude = applied_quantity(fixture.modifier(
        "catalyst-scalar",
        &[
            fact("unscalable", ParameterValue::Boolean(false)),
            fact("catalyst-kind", ParameterValue::Option(def(0x09f7))),
            fact("catalyst-amount", quantity(999_900.0, 2)),
            fact("property-attribute", ParameterValue::Boolean(true)),
        ],
    ));
    assert_eq!(magnitude, 10_000.0);
    let item = |raw, corruption| {
        let formatted = applied_quantity(fixture.modifier(
            "effective-amount",
            &[
                fact("component", quantity(raw, 0x295a)),
                fact("corruption-factor", quantity(corruption, 1)),
                fact("magnitude-factor", quantity(magnitude, 1)),
            ],
        ));
        applied_quantity(fixture.modifier(
            "contribute-player-attributes",
            &[fact("effective", quantity(formatted, 0x295a))],
        ))
    };
    let x = item(1_000_000.0, 900_719.925_474);
    let minus_x = item(-1_000_000.0, 900_719.925_474_75);
    assert_eq!(x, 9_007_199_254_740_000.0);
    assert_eq!(minus_x, -x);
    assert_eq!(
        applied_integer(fixture.attribute(fold(7.0, &[x, x, minus_x, minus_x]), 0.0)),
        8
    );
    assert_eq!(
        applied_integer(fixture.attribute(fold(7.0, &[x, minus_x, x, minus_x]), 0.0)),
        7
    );
    // Boundary controls demonstrate the conditional suffix argument's limits;
    // they do not assert that the source's passives are actually a suffix.
    let n = BoundedInteger::MAX as f64;
    assert_eq!(
        applied_integer(fixture.attribute(n - 1.0, 0.0)),
        BoundedInteger::MAX - 1
    );
    let permutations = [
        [3.0, 5.0, 25.0],
        [3.0, 25.0, 5.0],
        [5.0, 3.0, 25.0],
        [5.0, 25.0, 3.0],
        [25.0, 3.0, 5.0],
        [25.0, 5.0, 3.0],
    ];
    for increase in [0.0, 6.0, 8.0, 29.0] {
        for prefix in [-20.0, 7.0, 1000.0] {
            let expected = fixture.attribute(prefix + 33.0, increase);
            assert!(matches!(expected, EffectDisposition::Applied { .. }));
            for permutation in permutations {
                assert_eq!(
                    fixture.attribute(fold(prefix, &permutation), increase),
                    expected
                );
            }
        }
        for permutation in permutations {
            assert_eq!(
                applied_integer(fixture.attribute(fold(-2.0 * (n + 1.0), &permutation), increase)),
                0
            );
            let overflow = fixture.attribute(fold(n + 3.0, &permutation), increase);
            assert!(matches!(
                overflow,
                EffectDisposition::NumericalError {
                    reason: NumericalFailure::IntegerOverflow,
                    ..
                }
            ));
        }
    }
    assert!(matches!(
        fixture.attribute(n, 0.0),
        EffectDisposition::NumericalError {
            reason: NumericalFailure::IntegerOverflow,
            ..
        }
    ));
    assert_eq!(release::inventory(&path), inventory);
}
