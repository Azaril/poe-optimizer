//! Real owned receiver programs, exercised with explicit complete caller facts.
//! These checks do not establish item-source admission or occurrence/global closure.
use super::support::{bundle, data, json, success};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::{FiniteQuantity, ModifierDefId, OwnedDefinitionKey, StatDefId},
    owned_rules::{
        ContributionKind, ContributionReduction, RuleEffectKind, RuleEntity, RuleProgram,
        RuleReadSource,
    },
    owned_schema::{ComputedValueType, SchemaClosure, SchemaDefinitionId, SchemaSubject},
};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, RuleFact, RuleLimits, RuleScratch,
};
use poe_optimizer_import::{
    owned_recipe::{OwnedRecipeInput, StagedOwnedRecipe, assemble_owned_recipe},
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
};
use serde_json::Value;
use std::{
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
struct Components {
    staged: StagedOwnedRecipe,
    compiled: CompiledRulePackage,
}
impl Components {
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
        let declaration = self
            .staged
            .rules()
            .input()
            .owners
            .iter()
            .find(|o| &o.owner == owner)
            .unwrap();
        assert_eq!(result.owner_programs_closure, declaration.programs.closure);
        result.effects[0].disposition.clone()
    }
}
fn number(program: &RuleProgram, name: &str, value: f64) -> RuleFact {
    let read = program
        .reads
        .iter()
        .find(|r| r.id.as_str() == name)
        .unwrap();
    let ComputedValueType::Quantity { unit } = &read.value_type else {
        panic!("quantity read")
    };
    RuleFact {
        read: key(name),
        value: ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap()),
    }
}
fn applied(value: EffectDisposition) -> ParameterValue {
    let EffectDisposition::Applied { value } = value else {
        panic!("applied component expected: {value:?}")
    };
    value
}
fn numeric(value: &ParameterValue) -> f64 {
    let ParameterValue::Quantity(value) = value else {
        panic!("quantity expected")
    };
    value.value()
}
fn unresolved(value: EffectDisposition, expected: &str) {
    assert!(
        matches!(&value, EffectDisposition::Unresolved { input } if input.as_str() == expected),
        "expected missing {expected}: {value:?}"
    );
}
fn owner(binding: &Value, modifier: bool) -> SchemaSubject {
    if modifier {
        let id: ModifierDefId = serde_json::from_value(binding["modifier"].clone()).unwrap();
        SchemaSubject::Definition(id.address())
    } else {
        let id: StatDefId = serde_json::from_value(binding["stat"].clone()).unwrap();
        SchemaSubject::Definition(id.address())
    }
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
fn check_channels(c: &Components, bindings: &Value) {
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
        .staged
        .rules()
        .input()
        .receivers
        .members
        .iter()
        .find(|r| r.id.as_str() == "rounded-physical-minimum")
        .unwrap()
        .targets;
    assert_eq!(existing_targets.len(), 337);
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
            .staged
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
            matches!(&program.reads[0].source, RuleReadSource::Stat { entity: RuleEntity::Current, stat } if serde_json::to_value(stat).unwrap() == binding["raw"])
        );
        for read in &program.reads[1..] {
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
                numeric(&applied(c.evaluate(
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
        assert_eq!(numeric(&effective_value), 12.0);
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
            let expected = numeric(&value);
            assert_eq!(
                numeric(&applied(c.evaluate(
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
                                numeric(&applied(c.evaluate(&o, name, &facts, &mut scratch))),
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

pub fn check_elemental_weapons(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("elemental-weapon-inputs");
    let authored_bytes = bundle(&authored);
    let extension_path = authored.join("extension.json");
    let extension: OwnedRecipeExtension = serde_json::from_value(json(&extension_path)).unwrap();
    let bindings = json(authored.join("bindings.json"));
    assert_eq!(extension.schema.len(), 9);
    assert_eq!(extension.receivers.len(), 8);
    assert_eq!(extension.owners.len(), 12);
    assert_eq!(bindings["channels"].as_array().unwrap().len(), 8);
    assert_eq!(
        bindings["shared_local_elemental"]["stat"]["key"],
        "def.0000000000002929"
    );
    assert_eq!(bindings["shared_local_elemental"]["contribution"], "add");
    assert!(extension.tables.is_empty() && extension.operations_version.is_none());
    let before = recipe(prior);
    let before_bytes = bundle(prior);
    let output = cwd.join("elemental-weapon-successor");
    let report = success(publish(cwd, prior, &extension_path, &output));
    assert_eq!(report["extension"]["allocated_entries"], 9);
    assert_eq!(report["extension"]["appended_receivers"], 8);
    assert_eq!(report["extension"]["appended_programs"], 16);
    assert_eq!(report["extension"]["refined_subjects"], 0);
    assert_eq!(report["extension"]["appended_tables"], 0);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    let after = recipe(&output);
    assert_eq!(
        after.registry.last_issued.get(),
        before.registry.last_issued.get() + 9
    );
    assert!(
        before
            .registry
            .entries
            .iter()
            .all(|entry| after.registry.entries.contains(entry))
    );
    assert_eq!(after.schema.slots, before.schema.slots);
    assert!(
        before
            .schema
            .definitions
            .iter()
            .all(|entry| after.schema.definitions.contains(entry))
    );
    let mut rebound_routing = before.routing.clone();
    rebound_routing.definitions = after.routing.definitions.clone();
    assert_eq!(after.routing, rebound_routing);
    assert_eq!(after.rules.tables, before.rules.tables);
    assert_eq!(
        after.rules.receivers.closure,
        before.rules.receivers.closure
    );
    assert!(
        before.rules.receivers.members.iter().all(|receiver| after
            .rules
            .receivers
            .members
            .contains(receiver))
    );
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
    // The genuine successor crosses the old serialized-size ceiling while
    // remaining inside the default bound and all unchanged structural guards.
    let c = Components::new(after.clone());
    let old_wire_limit = 8 * 1024 * 1024;
    let wire_bytes = serde_json::to_vec(c.staged.rules().input()).unwrap().len();
    assert!(wire_bytes > old_wire_limit);
    assert_eq!(RuleLimits::default().max_wire_bytes, 16 * 1024 * 1024);
    assert!(wire_bytes <= RuleLimits::default().max_wire_bytes);
    let error = CompiledRulePackage::compile(
        c.staged.rules().input(),
        c.staged.schema(),
        RuleLimits {
            max_wire_bytes: old_wire_limit,
            ..Default::default()
        },
    )
    .expect_err("the genuine successor must exceed the explicit old wire cap");
    assert_eq!(error.path, "wire");
    assert_eq!(
        error.message,
        format!("canonical content exceeds {old_wire_limit} bytes")
    );
    check_channels(&c, &bindings);
    let replay = extend_owned_recipe(&c.staged, &extension, Default::default()).unwrap();
    assert_eq!(replay.receipt.allocated_entries, 0);
    assert_eq!(replay.receipt.appended_programs, 0);
    assert_eq!(replay.receipt.appended_receivers, 0);
    assert_eq!(replay.successor, after);
    let published = bundle(&output);
    assert!(
        !publish(cwd, prior, &extension_path, &output)
            .status
            .success()
    );
    assert_eq!(bundle(&output), published);
    for case in 1..=5 {
        let name = format!("queries-original-{case:02}.json");
        assert_eq!(published[&name], before_bytes[&name]);
    }
    assert_eq!(bundle(prior), before_bytes);
    assert_eq!(bundle(&authored), authored_bytes);
    output
}
