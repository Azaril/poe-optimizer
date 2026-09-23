//! Caller-fact component chains validate owned data, not source or build closure.
use super::support::{bundle, data, json, success};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::{FiniteQuantity, ModifierDefId, OptionDefId, OwnedDefinitionKey},
    owned_rules::{ContributionKind, RuleEffectKind, RuleEntity, RuleProgram, RuleReadSource},
    owned_schema::{ComputedValueType, SchemaClosure, SchemaDefinitionId, SchemaSubject},
};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, RuleFact, RuleScratch,
};
use poe_optimizer_import::{
    owned_recipe::{OwnedRecipeInput, StagedOwnedRecipe, assemble_owned_recipe},
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn key(value: &str) -> OwnedDefinitionKey {
    value.parse().unwrap()
}
fn recipe(path: &Path) -> OwnedRecipeInput {
    OwnedRecipeInput {
        schema_version: 1,
        registry: serde_json::from_value(json(path.join("registry.json"))).unwrap(),
        schema: serde_json::from_value(json(path.join("schema.json"))).unwrap(),
        rules: serde_json::from_value(json(path.join("rules.json"))).unwrap(),
        routing: serde_json::from_value(json(path.join("routing.json"))).unwrap(),
    }
}
fn publish(cwd: &Path, prior: &Path, extension: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("extend-owned-recipe")
        .arg(prior)
        .arg("--extension")
        .arg(extension)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
fn subject(family: &Value) -> SchemaSubject {
    let modifier: ModifierDefId = serde_json::from_value(family["modifier"].clone()).unwrap();
    SchemaSubject::Definition(modifier.address())
}
struct Component {
    staged: StagedOwnedRecipe,
    compiled: CompiledRulePackage,
}
impl Component {
    fn new(input: OwnedRecipeInput) -> Self {
        let staged = assemble_owned_recipe(input, Default::default()).unwrap();
        let compiled = CompiledRulePackage::compile(
            staged.rules().input(),
            staged.schema(),
            Default::default(),
        )
        .unwrap();
        Self { staged, compiled }
    }
    fn program(&self, owner: &SchemaSubject, name: &str) -> &RuleProgram {
        self.staged
            .rules()
            .input()
            .owners
            .iter()
            .find(|o| &o.owner == owner)
            .unwrap()
            .programs
            .members
            .iter()
            .find(|p| p.id.as_str() == name)
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
            .evaluate(owner, &key(name), facts, self.staged.schema(), scratch)
            .unwrap();
        assert_eq!(result.effects.len(), 1);
        assert_eq!(
            result.owner_programs_closure,
            self.staged
                .rules()
                .input()
                .owners
                .iter()
                .find(|o| &o.owner == owner)
                .unwrap()
                .programs
                .closure
        );
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
fn option(program: &RuleProgram, name: &str) -> OptionDefId {
    use poe_optimizer_core::owned_rules::RuleExpression;
    let node = program
        .nodes
        .iter()
        .find(|n| n.id.as_str() == format!("option-{name}"))
        .unwrap();
    let RuleExpression::Literal {
        value: ParameterValue::Option(value),
    } = &node.expression
    else {
        panic!("option literal")
    };
    value.clone()
}
fn check_catalysts(c: &Component, bindings: &Value) {
    let mut scratch = c.compiled.new_scratch();
    let none: OptionDefId = serde_json::from_value(
        json(data().join("resistance/ids.json"))["allocations"]["catalyst-none"].clone(),
    )
    .unwrap();
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
        let program = c.program(&owner, "catalyst-scalar");
        for (selected, property) in selections {
            let option = option(program, selected);
            let mut facts = catalyst_facts(program, &option, &[property]);
            assert_eq!(
                numeric(&applied(c.evaluate(
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
                numeric(&applied(c.evaluate(
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
            let facts = catalyst_facts(program, &option(program, "defence"), &[property]);
            assert_eq!(
                numeric(&applied(c.evaluate(
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
            numeric(&applied(c.evaluate(
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
            numeric(&applied(c.evaluate(
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
                numeric(&applied(c.evaluate(
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
    let facts = catalyst_facts(scalar, &option(scalar, "attack"), &["attack"]);
    let initial = applied(c.evaluate(&owner, "catalyst-scalar", &facts, scratch));
    // This explicit caller fact stands for an independently proven complete
    // empty transform sequence. Program evaluation does not prove that closure.
    let magnitude = applied(c.evaluate(
        &owner,
        "ordered-magnitude-scalar",
        &[fact("scalar", initial)],
        scratch,
    ));
    let base_program = c.program(&owner, "corrupted-base-factor");
    let base_read = base_program.reads[0].id.as_str();
    let base = applied(c.evaluate(
        &owner,
        "corrupted-base-factor",
        &[number(base_program, base_read, 1.0)],
        scratch,
    ));
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
    applied(c.evaluate(
        &owner,
        contribution,
        &[fact("effective", effective_value)],
        scratch,
    ))
}
fn check_component_chain(c: &Component, bindings: &Value) {
    let mut scratch = c.compiled.new_scratch();
    let mut contributions = BTreeMap::new();
    for binding in bindings["contributions"].as_array().unwrap() {
        let raw = match (
            binding["family"].as_str().unwrap(),
            binding["component"].as_str().unwrap(),
        ) {
            ("critical-flat", _) => 2.5,
            ("physical-flat", "minimum") => 10.0,
            ("physical-flat", "maximum") => 20.0,
            _ => 25.0,
        };
        let result = component_value(c, binding, raw, false, &mut scratch);
        let expected = match (
            binding["family"].as_str().unwrap(),
            binding["component"].as_str().unwrap(),
        ) {
            ("critical-flat", _) => 3.0,
            ("physical-flat", "minimum") => 12.0,
            ("physical-flat", "maximum") => 24.0,
            _ => 30.0,
        };
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
            .staged
            .rules()
            .input()
            .receivers
            .members
            .iter()
            .find(|r| r.id.as_str() == name)
            .unwrap();
        assert_eq!(receiver.targets.len(), 337);
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

pub fn check_local_scaling(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("local-scaling-inputs");
    let authored_bytes = bundle(&authored);
    let extension_path = authored.join("extension.json");
    let extension: OwnedRecipeExtension = serde_json::from_value(json(&extension_path)).unwrap();
    let bindings = json(authored.join("bindings.json"));
    assert_eq!(bindings["families"].as_array().unwrap().len(), 9);
    assert_eq!(bindings["contributions"].as_array().unwrap().len(), 7);
    assert!(extension.schema.is_empty());
    assert!(extension.tables.is_empty());
    assert!(extension.receivers.is_empty());
    assert_eq!(extension.owners.len(), 9);
    assert_eq!(
        extension
            .owners
            .iter()
            .map(|o| o.programs.members.len())
            .sum::<usize>(),
        25
    );
    let before_bytes = bundle(prior);
    let before = recipe(prior);
    let output = cwd.join("local-scaling-successor");
    let report = success(publish(cwd, prior, &extension_path, &output));
    for field in [
        "allocated_entries",
        "refined_subjects",
        "appended_tables",
        "appended_receivers",
    ] {
        assert_eq!(report["extension"][field], 0, "{field}");
    }
    assert_eq!(report["extension"]["appended_programs"], 25);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    let after = recipe(&output);
    assert_eq!(after.registry, before.registry);
    assert_eq!(after.schema, before.schema);
    assert_eq!(after.routing, before.routing);
    assert_eq!(after.rules.tables, before.rules.tables);
    assert_eq!(after.rules.receivers, before.rules.receivers);
    assert_eq!(after.rules.owners.len(), before.rules.owners.len());
    for old in &before.rules.owners {
        let new = after
            .rules
            .owners
            .iter()
            .find(|o| o.owner == old.owner)
            .unwrap();
        assert_eq!(new.programs.closure, old.programs.closure);
        assert!(
            old.programs
                .members
                .iter()
                .all(|p| new.programs.members.contains(p))
        );
        let addition = extension.owners.iter().find(|o| o.owner == old.owner);
        assert_eq!(
            new.programs.members.len(),
            old.programs.members.len() + addition.map_or(0, |o| o.programs.members.len())
        );
        if addition.is_some() {
            assert!(matches!(
                new.programs.closure,
                SchemaClosure::Partial { .. }
            ));
        }
    }
    for binding in bindings["contributions"].as_array().unwrap() {
        let kind: ContributionKind =
            serde_json::from_value(binding["contribution"].clone()).unwrap();
        assert!(matches!(
            kind,
            ContributionKind::Add | ContributionKind::Increase
        ));
    }
    let component = Component::new(after.clone());
    // Compare with the reviewed canonical cold algorithms after changing only
    // semantic parameter addresses. Named caller facts alone cannot prove that
    // the authored programs read the right Modifier occurrence slots.
    let canonical = json(data().join("modifier-value-inputs/bindings.json"));
    let source = canonical["families"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["family"] == "cold-resistance")
        .unwrap();
    let cold: ModifierDefId = serde_json::from_value(source["canonical"].clone()).unwrap();
    let cold_owner = SchemaSubject::Definition(cold.address());
    for family in bindings["families"].as_array().unwrap() {
        let target = canonical["families"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["family"] == family["family"])
            .unwrap();
        let owner = subject(family);
        let mut expected = component.program(&cold_owner, "catalyst-scalar").clone();
        for read in &mut expected.reads {
            if let RuleReadSource::Parameter { slot } = &mut read.source {
                let property = if read.id.as_str() == "unscalable" {
                    "unscalable"
                } else {
                    read.id.as_str().strip_prefix("property-").unwrap()
                };
                assert_eq!(
                    serde_json::to_value(&*slot).unwrap(),
                    source["property_inputs"][property]
                );
                *slot =
                    serde_json::from_value(target["property_inputs"][property].clone()).unwrap();
            }
        }
        assert_eq!(&expected, component.program(&owner, "catalyst-scalar"));
        assert_eq!(
            component.program(&cold_owner, "ordered-magnitude-scalar"),
            component.program(&owner, "ordered-magnitude-scalar")
        );
    }
    let repeated = extend_owned_recipe(&component.staged, &extension, Default::default()).unwrap();
    assert_eq!(repeated.receipt.appended_programs, 0);
    assert_eq!(repeated.successor, after);
    check_catalysts(&component, &bindings);
    check_component_chain(&component, &bindings);
    let published = bundle(&output);
    assert!(
        !publish(cwd, prior, &extension_path, &output)
            .status
            .success()
    );
    assert_eq!(bundle(&output), published);
    let replay = cwd.join("local-scaling-replay");
    assert_eq!(
        success(publish(cwd, prior, &extension_path, &replay)),
        report
    );
    assert_eq!(bundle(&replay), published);
    for case in 1..=5 {
        let name = format!("queries-original-{case:02}.json");
        assert_eq!(published[&name], before_bytes[&name]);
    }
    assert_eq!(bundle(prior), before_bytes);
    assert_eq!(bundle(&authored), authored_bytes);
    output
}
