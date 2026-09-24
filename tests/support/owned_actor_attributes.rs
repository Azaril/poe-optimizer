//! Item formatter outputs project into existing integer Actor input channels.
use super::{
    scalar_families::recipe,
    skill_scopes::canonical_instances,
    support::{bundle, data, json, normalize, success},
};
use poe_optimizer_core::{
    owned_build::{LoadoutScope, ParameterValue},
    owned_definitions::*,
    owned_draft::{DraftField, DraftLimits, DraftListCompletion, decode_draft},
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, NumericalFailure, RuleFact,
};
use poe_optimizer_import::{
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

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
    let id: ModifierDefId = serde_json::from_value(family["modifier"].clone()).unwrap();
    SchemaSubject::Definition(id.address())
}
fn quantity(program: &RuleProgram, input: &str, amount: f64) -> RuleFact {
    let read = program
        .reads
        .iter()
        .find(|r| r.id.as_str() == input)
        .unwrap();
    let ComputedValueType::Quantity { unit } = &read.value_type else {
        panic!("quantity input")
    };
    RuleFact {
        read: read.id.clone(),
        value: ParameterValue::Quantity(FiniteQuantity::new(amount, unit.clone()).unwrap()),
    }
}
fn check_components(after: &OwnedRecipeInput, bindings: &Value) {
    let checked = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(
        checked.rules().input(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let mut scratch = compiled.new_scratch();
    let unit: UnitDefId = serde_json::from_value(bindings["effective_unit"].clone()).unwrap();
    let effective_stat: StatDefId =
        serde_json::from_value(bindings["effective_stat"].clone()).unwrap();
    let SchemaLookup::Known(unit_schema) = checked.schema().definition(&unit) else {
        panic!("Count unit")
    };
    assert_eq!(unit_schema.dimension, UnitDimension::Count);
    let SchemaLookup::Known(effective_schema) = checked.schema().definition(&effective_stat) else {
        panic!("Modifier output")
    };
    assert_eq!(effective_schema.targets, [RuleEntityKind::Modifier]);
    assert_eq!(
        effective_schema.value,
        ComputedValueType::Quantity { unit: unit.clone() }
    );
    let expected_channels = [
        ("strength", vec!["def.0000000000001d2e"]),
        ("dexterity", vec!["def.0000000000001d2f"]),
        ("intelligence", vec!["def.0000000000001d30"]),
        (
            "all-attributes",
            vec![
                "def.0000000000001d2e",
                "def.0000000000001d2f",
                "def.0000000000001d30",
            ],
        ),
    ];
    let families = bindings["families"].as_array().unwrap();
    assert_eq!(families.len(), expected_channels.len());
    for (family, (name, expected)) in families.iter().zip(expected_channels) {
        assert_eq!(family["family"], name);
        let subject = subject(family);
        let owner = after
            .rules
            .owners
            .iter()
            .find(|o| o.owner == subject)
            .unwrap();
        assert!(matches!(
            owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        assert_eq!(owner.programs.members.len(), 5);
        let formatter = owner
            .programs
            .members
            .iter()
            .find(|p| p.id.as_str() == family["formatter_program"].as_str().unwrap())
            .unwrap();
        let projection = owner
            .programs
            .members
            .iter()
            .find(|p| p.id.as_str() == family["projection_program"].as_str().unwrap())
            .unwrap();
        assert_eq!(formatter.effects.len(), 1);
        assert_eq!(
            formatter.effects[0].id.as_str(),
            family["formatter_effect"].as_str().unwrap()
        );
        assert!(formatter.effects[0].when.is_none());
        assert!(
            matches!(&formatter.effects[0].effect, RuleEffectKind::Derive {
            entity: RuleEntity::Modifier, stat, value,
        } if stat == &effective_stat && value.as_str() == "formatted")
        );
        // This exact tail is the integral-input proof, not an assumption about
        // arbitrary quantities. Every branch rounds to one Count before /1.
        let formatter_wire = serde_json::to_value(formatter).unwrap();
        let expression = |id: &str| {
            formatter_wire["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|n| n["id"] == id)
                .unwrap()["expression"]
                .clone()
        };
        assert_eq!(
            expression("display-factor"),
            json!({
                "kind":"literal", "value":{"kind":"quantity","value":{
                    "value":1.0,"unit":{"kind":"unit","namespace":{"game":"poe2","version":"owned-mechanics-v1"},"key":"def.0000000000000001"}
                }}
            })
        );
        for (node, source, mode) in [
            (
                "display-rounded-floor",
                "display-rounded-plus-half",
                "floor",
            ),
            (
                "display-rounded-ceiling",
                "display-rounded-minus-half",
                "ceiling",
            ),
        ] {
            assert_eq!(
                expression(node),
                json!({"kind":"round","value":source,"quantum":{"value":1.0,"unit":unit},"mode":mode})
            );
        }
        assert_eq!(
            expression("display-rounded"),
            json!({"kind":"select","condition":"display-rounded-negative","when_true":"display-rounded-ceiling","when_false":"display-rounded-floor"})
        );
        assert_eq!(
            expression("formatted"),
            json!({"kind":"divide_factor","value":"display-rounded","divisor":"display-factor"})
        );
        assert_eq!(
            serde_json::to_value(projection.context).unwrap(),
            "equipment_use"
        );
        assert_eq!(projection.reads.len(), 1);
        assert_eq!(projection.reads[0].id.as_str(), "effective");
        assert_eq!(projection.reads[0].value_type, effective_schema.value);
        assert!(
            matches!(&projection.reads[0].source, RuleReadSource::Stat { entity: RuleEntity::Modifier, stat } if stat == &effective_stat)
        );
        assert_eq!(projection.nodes.len(), 2);
        assert!(
            matches!(&projection.nodes[0].expression, RuleExpression::Read { input } if input.as_str() == "effective")
        );
        assert!(
            matches!(&projection.nodes[1].expression, RuleExpression::QuantizeInteger { value, quantum, mode: RuleRounding::Floor } if value.as_str() == "effective" && quantum.value() == 1.0 && quantum.unit() == &unit)
        );
        let channels: Vec<StatDefId> = family["channels"]
            .as_array()
            .unwrap()
            .iter()
            .map(|channel| serde_json::from_value(channel["stat"].clone()).unwrap())
            .collect();
        assert_eq!(
            channels
                .iter()
                .map(|id| id.key().as_str())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(projection.effects.len(), channels.len());
        assert_eq!(
            channels.iter().collect::<BTreeSet<_>>().len(),
            channels.len()
        );
        for (effect, channel) in projection.effects.iter().zip(&channels) {
            let SchemaLookup::Known(schema) = checked.schema().definition(channel) else {
                panic!("Actor channel")
            };
            assert_eq!(schema.targets, [RuleEntityKind::Actor]);
            assert_eq!(schema.value, ComputedValueType::Integer);
            assert!(effect.when.is_none());
            assert!(
                matches!(&effect.effect, RuleEffectKind::Contribute { entity: RuleEntity::Player, stat, contribution: ContributionKind::Add, value } if stat == channel && value.as_str() == "integer")
            );
        }
        // Different occurrences reuse one compiled program and worker scratch;
        // no value may survive the next invocation, including the compound owner.
        for (raw, corruption, magnitude, expected) in [
            (0.0, 1.0, 1.0, 0),
            (27.0, 1.0, 1.0, 27),
            (-27.0, 1.0, 1.0, -27),
            (10.49, 1.0, 1.0, 10),
            (10.5, 1.0, 1.0, 11),
            (-10.5, 1.0, 1.0, -11),
            (10.0, 2.0, 1.5, 30),
            (-10.0, 2.0, 1.5, -28),
            (3.0, 1.0, 1.2, 3),
            (1000000.0, 1.0, 1.0, 1000000),
            (0.0, 1.0, 1.0, 0),
        ] {
            let facts = [
                quantity(formatter, "component", raw),
                quantity(formatter, "corruption-factor", corruption),
                quantity(formatter, "magnitude-factor", magnitude),
            ];
            assert_eq!(formatter.reads.len(), facts.len());
            let formatted = compiled
                .evaluate(
                    &subject,
                    &formatter.id,
                    &facts,
                    checked.schema(),
                    &mut scratch,
                )
                .unwrap();
            assert_eq!(formatted.owner_programs_closure, owner.programs.closure);
            let EffectDisposition::Applied {
                value: ParameterValue::Quantity(amount),
            } = &formatted.effects[0].disposition
            else {
                panic!("formatted Count")
            };
            assert_eq!(amount.unit(), &unit);
            assert_eq!(amount.value(), f64::from(expected));
            assert_eq!(amount.value().fract(), 0.0);
            let projected = compiled
                .evaluate(
                    &subject,
                    &projection.id,
                    &[RuleFact {
                        read: projection.reads[0].id.clone(),
                        value: ParameterValue::Quantity(amount.clone()),
                    }],
                    checked.schema(),
                    &mut scratch,
                )
                .unwrap();
            assert_eq!(projected.owner_programs_closure, owner.programs.closure);
            assert_eq!(projected.effects.len(), channels.len());
            for effect in projected.effects {
                assert_eq!(
                    effect.disposition,
                    EffectDisposition::Applied {
                        value: ParameterValue::Integer(
                            BoundedInteger::new(i64::from(expected)).unwrap()
                        )
                    }
                );
            }
        }
        for exact in [BoundedInteger::MIN, BoundedInteger::MAX] {
            let result = compiled
                .evaluate(
                    &subject,
                    &projection.id,
                    &[quantity(projection, "effective", exact as f64)],
                    checked.schema(),
                    &mut scratch,
                )
                .unwrap();
            for effect in result.effects {
                assert_eq!(
                    effect.disposition,
                    EffectDisposition::Applied {
                        value: ParameterValue::Integer(BoundedInteger::new(exact).unwrap())
                    }
                );
            }
        }
        let missing = compiled
            .evaluate(
                &subject,
                &projection.id,
                &[],
                checked.schema(),
                &mut scratch,
            )
            .unwrap();
        assert!(missing.effects.iter().all(|e| matches!(&e.disposition, EffectDisposition::Unresolved { input } if input.as_str() == "effective")));
        let wrong_unit: UnitDefId = serde_json::from_value(json!({"kind":"unit","namespace":{"game":"poe2","version":"owned-mechanics-v1"},"key":"def.0000000000000002"})).unwrap();
        for value in [
            ParameterValue::Boolean(true),
            ParameterValue::Integer(BoundedInteger::new(2).unwrap()),
            ParameterValue::Quantity(FiniteQuantity::new(2.0, wrong_unit).unwrap()),
        ] {
            assert!(
                compiled
                    .evaluate(
                        &subject,
                        &projection.id,
                        &[RuleFact {
                            read: projection.reads[0].id.clone(),
                            value
                        }],
                        checked.schema(),
                        &mut scratch
                    )
                    .is_err()
            );
        }
        for overflow in [
            BoundedInteger::MAX as f64 + 1.0,
            BoundedInteger::MIN as f64 - 1.0,
        ] {
            let result = compiled
                .evaluate(
                    &subject,
                    &projection.id,
                    &[quantity(projection, "effective", overflow)],
                    checked.schema(),
                    &mut scratch,
                )
                .unwrap();
            assert!(result.effects.iter().all(|effect| matches!(&effect.disposition, EffectDisposition::NumericalError { node, reason: NumericalFailure::IntegerOverflow } if node.as_str() == "integer")));
        }
    }
}

pub fn check_actor_attributes(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("actor-attribute-inputs");
    let authored_bytes = bundle(&authored);
    let prior_bytes = bundle(prior);
    let extension_path = authored.join("extension.json");
    let extension: OwnedRecipeExtension = serde_json::from_value(json(&extension_path)).unwrap();
    assert!(
        extension.schema.is_empty()
            && extension.tables.is_empty()
            && extension.receivers.is_empty()
            && extension.operations_version.is_none()
    );
    assert_eq!(extension.owners.len(), 4);
    assert!(
        extension
            .owners
            .iter()
            .all(|owner| owner.programs.members.len() == 1)
    );
    let before = recipe(prior);
    let output = cwd.join("actor-attribute-successor");
    let report = success(publish(cwd, prior, &extension_path, &output));
    for field in [
        "allocated_entries",
        "appended_receivers",
        "refined_subjects",
        "appended_tables",
    ] {
        assert_eq!(report["extension"][field], 0);
    }
    assert_eq!(report["extension"]["appended_programs"], 4);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    let after = recipe(&output);
    assert_eq!(after.registry, before.registry);
    assert_eq!(after.schema, before.schema);
    assert_eq!(after.routing, before.routing);
    let mut restored = after.rules.clone();
    for addition in &extension.owners {
        let owner = restored
            .owners
            .iter_mut()
            .find(|owner| owner.owner == addition.owner)
            .unwrap();
        assert_eq!(owner.programs.closure, addition.programs.closure);
        assert!(matches!(
            owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        for program in &addition.programs.members {
            assert_eq!(
                owner
                    .programs
                    .members
                    .iter()
                    .filter(|existing| *existing == program)
                    .count(),
                1
            );
            owner
                .programs
                .members
                .retain(|existing| existing != program);
        }
    }
    assert_eq!(
        restored, before.rules,
        "only the four authored programs may change"
    );
    let bindings = json(authored.join("bindings.json"));
    check_components(&after, &bindings);
    let staged = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    let replay = extend_owned_recipe(&staged, &extension, Default::default()).unwrap();
    assert_eq!(replay.successor, after);
    assert_eq!(replay.receipt.appended_programs, 0);
    assert_eq!(replay.receipt.allocated_entries, 0);
    let published = bundle(&output);
    assert!(
        !publish(cwd, prior, &extension_path, &output)
            .status
            .success()
    );
    // These inputs have the same schema and remain byte-identical through publication.
    for name in [
        "schema.json",
        "registry.json",
        "routing.json",
        "normalization.json",
        "mapping.json",
        "roles.json",
        "rewards.json",
        "items.json",
        "item-source.json",
        "tree-normalization.json",
    ] {
        assert_eq!(published[name], prior_bytes[name], "changed {name}");
    }
    let (mut scopes, mut complete, mut pending, mut modifiers, mut displays, mut queries) =
        (Vec::new(), 0, 0, 0, 0, 0);
    for case in 1..=5 {
        let query_file = format!("queries-original-{case:02}.json");
        assert_eq!(published[&query_file], prior_bytes[&query_file]);
        let destination = cwd.join(format!("actor-attribute-original-{case}"));
        let report = success(normalize(cwd, &output, case, &destination, true));
        assert_eq!(report["normalization_status"], "pending");
        assert_eq!(report["verification"]["calculation"], "not_run");
        let previous_path = cwd.join(format!("skill-scope-original-{case}"));
        let draft = decode_draft(
            &fs::read(destination.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let previous = decode_draft(
            &fs::read(previous_path.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let input = draft.input();
        let mut current = serde_json::to_value(input).unwrap();
        let mut old = serde_json::to_value(previous.input()).unwrap();
        let references = canonical_instances(&mut current, input.allocator.lineage(), &[]);
        assert!(references > 0);
        assert_eq!(
            references,
            canonical_instances(&mut old, previous.input().allocator.lineage(), &[])
        );
        assert_eq!(
            current, old,
            "original-{case}: immutable build facts changed"
        );
        scopes.push(input.skills.members.len());
        assert!(input.skills.members.iter().all(|s| matches!(
            s.scope,
            DraftField::Known {
                value: LoadoutScope::Shared
            }
        )));
        for gem in &input.gems.members {
            match gem.parameters.completion {
                DraftListCompletion::Complete => complete += 1,
                DraftListCompletion::Pending { .. } => pending += 1,
            }
        }
        queries += input
            .query_presets
            .members
            .iter()
            .map(|preset| preset.queries.requests.members.len())
            .sum::<usize>();
        assert!(
            !draft
                .validate_limits(DraftLimits::default())
                .unwrap()
                .issues
                .is_empty()
        );
        let sidecar = json(destination.join("sidecar.json"));
        let old_sidecar = json(previous_path.join("sidecar.json"));
        let mut current_items = sidecar["item_texts"].clone();
        let mut old_items = old_sidecar["item_texts"].clone();
        let references = canonical_instances(&mut current_items, input.allocator.lineage(), &[]);
        assert!(references > 0);
        assert_eq!(
            references,
            canonical_instances(&mut old_items, previous.input().allocator.lineage(), &[])
        );
        assert_eq!(
            current_items, old_items,
            "original-{case}: source attribution changed"
        );
        for item in sidecar["item_texts"].as_array().unwrap() {
            for line in item["lines"].as_array().unwrap() {
                modifiers += line["modifiers"].as_array().unwrap().len();
                if line["outcome"]["kind"] == "known"
                    && line["outcome"]["value"]["rule"]
                        .as_str()
                        .is_some_and(|rule| rule.starts_with("observed-"))
                {
                    displays += 1;
                }
            }
        }
    }
    assert_eq!(scopes, [9, 63, 9, 13, 46]);
    assert_eq!((complete, pending), (0, 478));
    assert_eq!((modifiers, displays, queries), (64, 53, 110));
    assert_eq!(bundle(prior), prior_bytes);
    assert_eq!(bundle(&output), published);
    assert_eq!(bundle(&authored), authored_bytes);
    output
}
