//! Independent caller-fact laws over the maintained DSL release.
//! Explicit component inputs do not establish item admission or build closure.
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::{
        FiniteQuantity, ModifierDefId, OptionDefId, OwnedDefinitionKey, StatDefId,
    },
    owned_rules::{
        ContributionKind, ContributionReduction, RuleEffectKind, RuleEntity, RuleProgram,
        RuleReadSource,
    },
    owned_schema::{ComputedValueType, SchemaClosure, SchemaDefinitionId, SchemaSubject},
};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, RuleFact, RuleScratch,
};
use poe_optimizer_import::owned_recipe::CheckedOwnedRecipe;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

fn key(value: &str) -> OwnedDefinitionKey {
    value.parse().unwrap()
}
fn local_bindings() -> Value {
    serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/owned/poe2/3887ae68/local-scaling-inputs/bindings.json"
    )))
    .unwrap()
}
fn elemental_bindings() -> &'static Value {
    static BINDINGS: OnceLock<Value> = OnceLock::new();
    BINDINGS.get_or_init(|| {
        serde_json::from_slice(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/owned/poe2/3887ae68/elemental-weapon-inputs/bindings.json"
        )))
        .unwrap()
    })
}
fn option(name: &str) -> OptionDefId {
    // Semantic identities survive DSL SSA-node renaming. The expected match and
    // lazy-branch values below remain independent of the authored formula.
    static IDS: OnceLock<Value> = OnceLock::new();
    let ids = IDS.get_or_init(|| {
        serde_json::from_slice(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/owned/poe2/3887ae68/resistance/ids.json"
        )))
        .unwrap()
    });
    serde_json::from_value(ids["allocations"][format!("catalyst-{name}")].clone()).unwrap()
}
fn subject(binding: &Value) -> SchemaSubject {
    let modifier: ModifierDefId = serde_json::from_value(binding["modifier"].clone()).unwrap();
    SchemaSubject::Definition(modifier.address())
}
fn owner(binding: &Value, modifier: bool) -> SchemaSubject {
    if modifier {
        subject(binding)
    } else {
        let stat: StatDefId = serde_json::from_value(binding["stat"].clone()).unwrap();
        SchemaSubject::Definition(stat.address())
    }
}
struct Component {
    recipe: &'static CheckedOwnedRecipe,
    compiled: &'static CompiledRulePackage,
}
impl Component {
    fn new() -> Self {
        let recipe = super::tracked_release().release().assembled().checked();
        Self {
            recipe,
            compiled: recipe.compiled_rules().as_ref(),
        }
    }
    fn program(&self, owner: &SchemaSubject, name: &str) -> &RuleProgram {
        self.recipe
            .rules()
            .input()
            .owners
            .iter()
            .find(|row| &row.owner == owner)
            .unwrap()
            .programs
            .members
            .iter()
            .find(|program| program.id.as_str() == name)
            .unwrap()
    }
    fn evaluate(
        &self,
        owner: &SchemaSubject,
        name: &str,
        facts: &[RuleFact],
        scratch: &mut RuleScratch,
    ) -> EffectDisposition {
        let result = self
            .compiled
            .evaluate(
                owner,
                &key(name),
                facts,
                self.recipe.schema().as_ref(),
                scratch,
            )
            .unwrap();
        assert_eq!(result.effects.len(), 1, "{name}");
        let declaration = self
            .recipe
            .rules()
            .input()
            .owners
            .iter()
            .find(|row| &row.owner == owner)
            .unwrap();
        assert_eq!(result.owner_programs_closure, declaration.programs.closure);
        result.effects[0].disposition.clone()
    }
}
fn fact(name: &str, value: ParameterValue) -> RuleFact {
    RuleFact {
        read: key(name),
        value,
    }
}
fn number(program: &RuleProgram, name: &str, value: f64) -> RuleFact {
    let read = program
        .reads
        .iter()
        .find(|r| r.id.as_str() == name)
        .unwrap();
    let ComputedValueType::Quantity { unit } = &read.value_type else {
        panic!("quantity read {name}")
    };
    fact(
        name,
        ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap()),
    )
}
fn applied(disposition: EffectDisposition) -> ParameterValue {
    let EffectDisposition::Applied { value } = disposition else {
        panic!("expected applied component: {disposition:?}")
    };
    value
}
fn numeric(value: &ParameterValue) -> f64 {
    let ParameterValue::Quantity(value) = value else {
        panic!("quantity")
    };
    value.value()
}
fn numeric_in(value: &ParameterValue, unit: &Value) -> f64 {
    let ParameterValue::Quantity(value) = value else {
        panic!("quantity")
    };
    assert_eq!(serde_json::to_value(value.unit()).unwrap(), *unit);
    value.value()
}
fn factor(value: &ParameterValue) -> f64 {
    numeric_in(value, &elemental_bindings()["units"]["factor"])
}
fn damage(value: &ParameterValue) -> f64 {
    numeric_in(value, &elemental_bindings()["units"]["damage"])
}
fn unresolved(disposition: EffectDisposition, expected: &str) {
    assert!(
        matches!(&disposition, EffectDisposition::Unresolved { input } if input.as_str() == expected),
        "expected missing {expected}, got {disposition:?}"
    );
}
fn catalyst_facts(
    program: &RuleProgram,
    option: &OptionDefId,
    properties: &[&str],
) -> Vec<RuleFact> {
    program
        .reads
        .iter()
        .map(|read| {
            let name = read.id.as_str();
            match name {
                "unscalable" => fact(name, ParameterValue::Boolean(false)),
                "catalyst-kind" => fact(name, ParameterValue::Option(option.clone())),
                "catalyst-amount" => number(program, name, 20.0),
                _ => fact(
                    name,
                    ParameterValue::Boolean(
                        properties.contains(&name.strip_prefix("property-").unwrap()),
                    ),
                ),
            }
        })
        .collect()
}
fn check_catalysts(c: &Component, bindings: &Value) {
    let mut scratch = c.compiled.new_scratch();
    let none = option("none");
    assert_eq!(bindings["families"].as_array().unwrap().len(), 9);
    assert_eq!(
        bindings["families"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["family"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        [
            "physical-increase",
            "attack-speed-increase",
            "critical-increase",
            "critical-flat",
            "physical-flat",
            "cold-flat",
            "fire-flat",
            "lightning-flat",
            "chaos-flat"
        ]
        .into_iter()
        .collect::<BTreeSet<_>>()
    );
    let selections = [
        ("life", "life"),
        ("mana", "mana"),
        ("defence", "defences"),
        ("physical", "physical"),
        ("fire", "fire"),
        ("cold", "cold"),
        ("lightning", "lightning"),
        ("chaos", "chaos"),
        ("attack", "attack"),
        ("caster", "caster"),
        ("speed", "speed"),
        ("attribute", "attribute"),
        ("minion", "minion"),
    ];
    for family in bindings["families"].as_array().unwrap() {
        let owner = subject(family);
        assert!(matches!(
            c.recipe
                .rules()
                .input()
                .owners
                .iter()
                .find(|row| row.owner == owner)
                .unwrap()
                .programs
                .closure,
            SchemaClosure::Partial { .. }
        ));
        let program = c.program(&owner, "catalyst-scalar");
        for (selected, property) in selections {
            let option = option(selected);
            let mut facts = catalyst_facts(program, &option, &[property]);
            assert_eq!(
                factor(&applied(c.evaluate(
                    &owner,
                    "catalyst-scalar",
                    &facts,
                    &mut scratch
                ))),
                1.2
            );
            // Matching selection requires its actual enabled amount.
            facts.retain(|f| f.read.as_str() != "catalyst-amount");
            unresolved(
                c.evaluate(&owner, "catalyst-scalar", &facts, &mut scratch),
                "catalyst-amount",
            );
            let mut nonmatch = catalyst_facts(program, &option, &[]);
            nonmatch.retain(|f| f.read.as_str() != "catalyst-amount");
            assert_eq!(
                factor(&applied(c.evaluate(
                    &owner,
                    "catalyst-scalar",
                    &nonmatch,
                    &mut scratch
                ))),
                1.0
            );
            let mut missing_property = catalyst_facts(program, &option, &[]);
            missing_property.retain(|f| f.read.as_str() != format!("property-{property}"));
            unresolved(
                c.evaluate(&owner, "catalyst-scalar", &missing_property, &mut scratch),
                &format!("property-{property}"),
            );
        }
        for property in ["defences", "armour", "evasion", "energyshield"] {
            let facts = catalyst_facts(program, &option("defence"), &[property]);
            assert_eq!(
                factor(&applied(c.evaluate(
                    &owner,
                    "catalyst-scalar",
                    &facts,
                    &mut scratch
                ))),
                1.2
            );
        }
        // Explicit None is different from a missing selection; neither a missing
        // amount nor unrelated property is demanded on the disabled branch.
        let disabled = vec![
            fact("unscalable", ParameterValue::Boolean(false)),
            fact("catalyst-kind", ParameterValue::Option(none.clone())),
        ];
        assert_eq!(
            factor(&applied(c.evaluate(
                &owner,
                "catalyst-scalar",
                &disabled,
                &mut scratch
            ))),
            1.0
        );
        unresolved(
            c.evaluate(&owner, "catalyst-scalar", &disabled[..1], &mut scratch),
            "catalyst-kind",
        );
        unresolved(
            c.evaluate(&owner, "catalyst-scalar", &disabled[1..], &mut scratch),
            "unscalable",
        );
        let unscalable = [fact("unscalable", ParameterValue::Boolean(true))];
        assert_eq!(
            factor(&applied(c.evaluate(
                &owner,
                "catalyst-scalar",
                &unscalable,
                &mut scratch
            ))),
            1.0
        );
        let ordered = c.program(&owner, "ordered-magnitude-scalar");
        assert!(
            matches!(&ordered.reads[0].source, RuleReadSource::ModifierTransforms { stat, initial }
            if serde_json::to_value(stat).unwrap() == bindings["stats"]["ordered_scalar"] && serde_json::to_value(initial).unwrap() == bindings["stats"]["initial_scalar"])
        );
        unresolved(
            c.evaluate(&owner, "ordered-magnitude-scalar", &[], &mut scratch),
            "scalar",
        );
        for scalar in [0.0, 1.0, 1.2, 1.75] {
            let facts = [number(ordered, "scalar", scalar)];
            assert_eq!(
                factor(&applied(c.evaluate(
                    &owner,
                    "ordered-magnitude-scalar",
                    &facts,
                    &mut scratch
                ))),
                scalar
            );
        }
    }
}

fn component_value(
    c: &Component,
    binding: &Value,
    raw: f64,
    negative: bool,
    scratch: &mut RuleScratch,
) -> ParameterValue {
    let owner = subject(binding);
    let scalar = c.program(&owner, "catalyst-scalar");
    let facts = catalyst_facts(scalar, &option("attack"), &["attack"]);
    let initial = applied(c.evaluate(&owner, "catalyst-scalar", &facts, scratch));
    assert_eq!(factor(&initial), 1.2);
    // This explicit caller fact stands for an independently proven complete
    // empty transform sequence. Program evaluation does not prove that closure.
    let magnitude = applied(c.evaluate(
        &owner,
        "ordered-magnitude-scalar",
        &[fact("scalar", initial)],
        scratch,
    ));
    assert_eq!(factor(&magnitude), 1.2);
    let base_program = c.program(&owner, "corrupted-base-factor");
    let base_read = base_program.reads[0].id.as_str();
    let base = applied(c.evaluate(
        &owner,
        "corrupted-base-factor",
        &[number(base_program, base_read, 1.0)],
        scratch,
    ));
    assert_eq!(factor(&base), 1.0);
    let effective_name = format!("effective-{}", binding["component"].as_str().unwrap());
    let effective = c.program(&owner, &effective_name);
    let mut facts = vec![
        number(effective, "component", raw),
        fact("corruption-factor", base),
        fact("magnitude-factor", magnitude),
    ];
    if effective.reads.iter().any(|r| r.id.as_str() == "negative") {
        facts.push(fact("negative", ParameterValue::Boolean(negative)));
    }
    for input in ["component", "corruption-factor", "magnitude-factor"] {
        let absent: Vec<_> = facts
            .iter()
            .filter(|f| f.read.as_str() != input)
            .cloned()
            .collect();
        unresolved(c.evaluate(&owner, &effective_name, &absent, scratch), input);
    }
    let effective_value = applied(c.evaluate(&owner, &effective_name, &facts, scratch));
    let contribution = binding["program"].as_str().unwrap();
    unresolved(c.evaluate(&owner, contribution, &[], scratch), "effective");
    let result = applied(c.evaluate(
        &owner,
        contribution,
        &[fact("effective", effective_value)],
        scratch,
    ));
    numeric_in(&result, &binding["unit"]);
    result
}
fn check_component_chain(c: &Component, bindings: &Value) {
    let mut scratch = c.compiled.new_scratch();
    let mut contributions = BTreeMap::new();
    let cases = [
        (
            "physical-increase",
            "amount",
            "local-minimum-increase",
            25.0,
            30.0,
        ),
        (
            "physical-increase",
            "amount",
            "local-maximum-increase",
            25.0,
            30.0,
        ),
        (
            "attack-speed-increase",
            "amount",
            "local-attack-rate-increase",
            25.0,
            30.0,
        ),
        (
            "critical-increase",
            "amount",
            "local-critical-chance-increase",
            25.0,
            30.0,
        ),
        (
            "critical-flat",
            "amount",
            "local-critical-chance-add",
            2.5,
            3.0,
        ),
        ("physical-flat", "minimum", "local-minimum-add", 10.0, 12.0),
        ("physical-flat", "maximum", "local-maximum-add", 20.0, 24.0),
    ];
    let bindings_rows = bindings["contributions"].as_array().unwrap();
    assert_eq!(bindings_rows.len(), cases.len());
    for (family, component, name, raw, expected) in cases {
        let binding = bindings_rows
            .iter()
            .find(|row| {
                row["family"] == family && row["component"] == component && row["program"] == name
            })
            .unwrap();
        let result = component_value(c, binding, raw, false, &mut scratch);
        assert_eq!(numeric(&result), expected);
        let owner = subject(binding);
        let program = c.program(&owner, binding["program"].as_str().unwrap());
        assert_eq!(program.reads.len(), 1);
        assert!(
            matches!(&program.reads[0].source, RuleReadSource::Stat { entity: RuleEntity::Modifier, stat }
            if serde_json::to_value(stat).unwrap() == binding["input"])
        );
        assert!(
            matches!(&program.effects[0].effect, RuleEffectKind::Contribute { entity: RuleEntity::Current, stat, contribution, .. }
            if serde_json::to_value(stat).unwrap() == binding["target"] && serde_json::to_value(contribution).unwrap() == binding["contribution"])
        );
        let address = (
            binding["target"]["key"].as_str().unwrap().to_string(),
            binding["contribution"].as_str().unwrap().to_string(),
        );
        assert!(contributions.insert(address, result).is_none());
        if binding["family"].as_str().unwrap().ends_with("increase") {
            assert_eq!(
                numeric(&component_value(c, binding, raw, true, &mut scratch)),
                -expected
            );
        }
    }
    // Each reduction supplied below is explicit and complete only for this
    // caller-fact test. It does not close incoming membership in the real build.
    for (name, raw, expected) in [
        ("pre-override-attack-rate", 1.4, 1.82),
        ("pre-override-critical-chance", 5.0, 10.4),
        ("rounded-physical-minimum", 55.0, 105.0),
        ("rounded-physical-maximum", 91.0, 179.0),
    ] {
        let receiver = c
            .recipe
            .rules()
            .input()
            .receivers
            .members
            .iter()
            .find(|r| r.id.as_str() == name)
            .unwrap();
        assert!(!receiver.targets.is_empty());
        let owner = SchemaSubject::Definition(receiver.stat.address());
        let program = c.program(&owner, name);
        let mut facts = vec![];
        for read in &program.reads {
            let value = match read.id.as_str() {
                "raw" => number(program, "raw", raw).value,
                "quality" => number(program, "quality", 20.0).value,
                "quality-effect" | "alternate-quality" => {
                    number(program, read.id.as_str(), 0.0).value
                }
                "increase" | "flat" => {
                    let kind = if read.id.as_str() == "increase" {
                        "increase"
                    } else {
                        "add"
                    };
                    contributions[&(receiver.stat.key().as_str().to_string(), kind.to_string())]
                        .clone()
                }
                other => panic!("unexpected receiver input {other}"),
            };
            facts.push(fact(read.id.as_str(), value));
        }
        assert_eq!(
            numeric(&applied(c.evaluate(&owner, name, &facts, &mut scratch))),
            expected
        );
        let absent: Vec<_> = facts
            .iter()
            .filter(|f| f.read.as_str() != "increase")
            .cloned()
            .collect();
        unresolved(c.evaluate(&owner, name, &absent, &mut scratch), "increase");
    }
    // Independent worker scratch and alternating inputs cannot leak a previous
    // canonical result. These are component calls, not occurrence-binding proof.
    let binding = &bindings["contributions"][0];
    std::thread::scope(|scope| {
        let workers: Vec<_> = [10.0, 25.0, 40.0]
            .into_iter()
            .map(|raw| {
                scope.spawn(move || {
                    let mut scratch = c.compiled.new_scratch();
                    for _ in 0..3 {
                        assert_eq!(
                            numeric(&component_value(c, binding, raw, false, &mut scratch)),
                            raw * 1.2
                        );
                        assert_eq!(
                            numeric(&component_value(c, binding, 25.0, true, &mut scratch)),
                            -30.0
                        );
                    }
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
    });
}

fn receiver_facts(
    program: &RuleProgram,
    raw: f64,
    flat: f64,
    specific: f64,
    elemental: f64,
) -> Vec<RuleFact> {
    program
        .reads
        .iter()
        .map(|read| {
            let value = match read.id.as_str() {
                "raw" => raw,
                "flat" => flat,
                "type-increase" => specific,
                "elemental-increase" => elemental,
                other => panic!("unexpected receiver read {other}"),
            };
            number(program, read.id.as_str(), value)
        })
        .collect()
}
fn check_channels(c: &Component, bindings: &Value) {
    let channels = bindings["channels"].as_array().unwrap();
    let mut scratch = c.compiled.new_scratch();
    // Independent address inventory catches crossed min/max, type and raw channels.
    let expected = [
        ("cold", "minimum", "2921", "241b", "2600", "253f"),
        ("cold", "maximum", "2922", "241a", "2600", "2540"),
        ("fire", "minimum", "2923", "241e", "261d", "253f"),
        ("fire", "maximum", "2924", "241d", "261d", "2540"),
        ("lightning", "minimum", "2925", "2420", "263a", "253f"),
        ("lightning", "maximum", "2926", "241f", "263a", "2540"),
        ("chaos", "minimum", "2927", "2419", "2657", "253f"),
        ("chaos", "maximum", "2928", "2418", "2657", "2540"),
    ];
    let existing_targets = &c
        .recipe
        .rules()
        .input()
        .receivers
        .members
        .iter()
        .find(|r| r.id.as_str() == "rounded-physical-minimum")
        .unwrap()
        .targets;
    assert!(!existing_targets.is_empty());
    assert_eq!(
        channels.len(),
        expected.len(),
        "exact independently reviewed channel inventory"
    );
    for (binding, (family, endpoint, stat, raw, modifier, effective)) in
        channels.iter().zip(expected)
    {
        assert_eq!(binding["family"], family);
        assert_eq!(binding["endpoint"], endpoint);
        for (field, suffix) in [
            ("stat", stat),
            ("raw", raw),
            ("modifier", modifier),
            ("effective", effective),
        ] {
            assert_eq!(binding[field]["key"], format!("def.000000000000{suffix}"));
        }
        let receiver_owner = owner(binding, false);
        let receiver_name = binding["receiver"].as_str().unwrap();
        let receiver = c
            .recipe
            .rules()
            .input()
            .receivers
            .members
            .iter()
            .find(|r| r.id.as_str() == receiver_name)
            .unwrap();
        assert_eq!(&receiver.targets, existing_targets);
        assert_eq!(
            serde_json::to_value(&receiver.stat).unwrap(),
            binding["stat"]
        );
        let program = c.program(&receiver_owner, receiver_name);
        assert_eq!(program.reads.len(), if family == "chaos" { 2 } else { 4 });
        assert!(
            matches!(&program.reads.iter().find(|read| read.id.as_str() == "raw").unwrap().source, RuleReadSource::Stat { entity: RuleEntity::Current, stat } if serde_json::to_value(stat).unwrap() == binding["raw"])
        );
        for read in program
            .reads
            .iter()
            .filter(|read| read.id.as_str() != "raw")
        {
            let (target, kind, unit) = match read.id.as_str() {
                "flat" => (
                    &binding["stat"],
                    ContributionKind::Add,
                    &bindings["units"]["damage"],
                ),
                "type-increase" => (
                    &binding["stat"],
                    ContributionKind::Increase,
                    &bindings["units"]["percentage"],
                ),
                "elemental-increase" => (
                    &bindings["shared_local_elemental"]["stat"],
                    ContributionKind::Add,
                    &bindings["units"]["percentage"],
                ),
                other => panic!("unexpected reduction {other}"),
            };
            assert!(
                matches!(&read.source, RuleReadSource::Contributions { entity: RuleEntity::Current, stat, contribution, reduction: ContributionReduction::Sum, empty: ParameterValue::Quantity(empty) }
                if serde_json::to_value(stat).unwrap() == *target && *contribution == kind && empty.value() == 0.0 && serde_json::to_value(empty.unit()).unwrap() == *unit)
            );
        }
        assert!(
            matches!(&program.effects[0].effect, RuleEffectKind::Derive { entity: RuleEntity::Current, stat, .. } if serde_json::to_value(stat).unwrap() == binding["stat"])
        );
        let cases: &[(f64, f64, f64, f64, f64)] = if family == "chaos" {
            // Chaos neither rounds the pair nor consumes any local/elemental increase.
            &[
                (10.0, 3.25, 200.0, 500.0, 13.25),
                (0.0, 2.5, 0.0, 0.0, 2.5),
                (0.0, -2.5, 0.0, 0.0, -2.5),
                (0.0, 0.0, 0.0, 0.0, 0.0),
            ]
        } else {
            // Local and all-elemental increases add; flat damage is scaled together
            // with raw damage, and final round is floor(value+0.5), including negatives.
            &[
                (10.0, 3.0, 20.0, 30.0, 20.0),
                (10.0, 0.0, 25.0, 50.0, 18.0),
                (0.0, 2.5, 0.0, 0.0, 3.0),
                (0.0, -2.5, 0.0, 0.0, -2.0),
                (1.49999999999999, 0.0, 0.0, 0.0, 1.0),
                (1.5, 0.0, 0.0, 0.0, 2.0),
                (0.0, 0.0, 20.0, 30.0, 0.0),
                (10.0, 0.0, -25.0, 0.0, 8.0),
                (10.0, 0.0, -100.0, 0.0, 0.0),
                (10.0, 0.0, -200.0, 0.0, -10.0),
            ]
        };
        for &(raw, flat, specific, elemental, expected) in cases {
            let facts = receiver_facts(program, raw, flat, specific, elemental);
            assert_eq!(
                damage(&applied(c.evaluate(
                    &receiver_owner,
                    receiver_name,
                    &facts,
                    &mut scratch
                ))),
                expected,
                "{receiver_name}"
            );
        }
        let facts = receiver_facts(program, 10.0, 3.0, 20.0, 30.0);
        for read in &program.reads {
            let missing: Vec<_> = facts
                .iter()
                .filter(|f| f.read != read.id)
                .cloned()
                .collect();
            unresolved(
                c.evaluate(&receiver_owner, receiver_name, &missing, &mut scratch),
                read.id.as_str(),
            );
        }
        // The canonical effective stage and the new contribution are evaluated
        // separately with explicit facts, preserving the Partial owner report.
        let modifier_owner = owner(binding, true);
        let effective_name = format!("effective-{endpoint}");
        let effective_program = c.program(&modifier_owner, &effective_name);
        let effective_facts = [
            number(effective_program, "component", 10.0),
            number(effective_program, "corruption-factor", 1.0),
            number(effective_program, "magnitude-factor", 1.2),
        ];
        let effective_value = applied(c.evaluate(
            &modifier_owner,
            &effective_name,
            &effective_facts,
            &mut scratch,
        ));
        assert_eq!(damage(&effective_value), 12.0);
        let contribution = binding["contribution_program"].as_str().unwrap();
        let producer = c.program(&modifier_owner, contribution);
        assert!(
            matches!(&producer.reads[0].source, RuleReadSource::Stat { entity: RuleEntity::Modifier, stat } if serde_json::to_value(stat).unwrap() == binding["effective"])
        );
        assert!(
            matches!(&producer.effects[0].effect, RuleEffectKind::Contribute { entity: RuleEntity::Current, stat, contribution: ContributionKind::Add, .. } if serde_json::to_value(stat).unwrap() == binding["stat"])
        );
        unresolved(
            c.evaluate(&modifier_owner, contribution, &[], &mut scratch),
            "effective",
        );
        for value in [effective_value, number(producer, "effective", -2.5).value] {
            let expected = damage(&value);
            assert_eq!(
                damage(&applied(c.evaluate(
                    &modifier_owner,
                    contribution,
                    &[RuleFact {
                        read: key("effective"),
                        value
                    }],
                    &mut scratch
                ))),
                expected
            );
        }
    }
    // Receiver scratch is worker-local and changed inputs do not retain prior values.
    std::thread::scope(|scope| {
        let workers: Vec<_> = channels
            .iter()
            .map(|binding| {
                scope.spawn(move || {
                    let mut scratch = c.compiled.new_scratch();
                    let o = owner(binding, false);
                    let name = binding["receiver"].as_str().unwrap();
                    let program = c.program(&o, name);
                    let chaos = binding["family"] == "chaos";
                    for _ in 0..3 {
                        for (raw, flat, expected) in [
                            (10.0, 2.0, if chaos { 12.0 } else { 15.0 }),
                            (2.0, 0.0, if chaos { 2.0 } else { 3.0 }),
                        ] {
                            let facts = receiver_facts(program, raw, flat, 0.0, 25.0);
                            assert_eq!(
                                damage(&applied(c.evaluate(&o, name, &facts, &mut scratch))),
                                expected
                            );
                        }
                    }
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
    });
}

#[test]
fn tracked_catalysts_preserve_matching_properties_and_lazy_missing_input_laws() {
    check_catalysts(&Component::new(), &local_bindings());
}
#[test]
fn tracked_local_scaling_preserves_component_chains_and_worker_scratch() {
    check_component_chain(&Component::new(), &local_bindings());
}
#[test]
fn tracked_elemental_weapons_preserve_exact_channels_rounding_and_worker_scratch() {
    check_channels(&Component::new(), elemental_bindings());
}
