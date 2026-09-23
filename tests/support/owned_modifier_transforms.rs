//! Publication proves owned data wiring and preserves unresolved build obligations.
//! Actual ordered-fold behavior is covered by the Engine occurrence-binding tests.
use super::support::{bundle, data, json, normalize, success};
use poe_optimizer_core::{
    owned_definitions::{ModifierDefId, StatDefId},
    owned_draft::{
        DraftAllocationAccess, DraftField, DraftLimits, DraftListCompletion, decode_draft,
    },
    owned_rules::{RuleEffectKind, RuleEntity, RuleReadSource},
    owned_schema::{
        ComputedValueType, DefinitionDescriptor, SchemaClosure, SchemaDefinitionId, SchemaState,
        SchemaSubject,
    },
};
use poe_optimizer_engine::owned_rules::CompiledRulePackage;
use poe_optimizer_import::{
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
};
use std::{
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
fn recipe(path: &Path) -> OwnedRecipeInput {
    OwnedRecipeInput {
        schema_version: 1,
        registry: serde_json::from_value(json(path.join("registry.json"))).unwrap(),
        schema: serde_json::from_value(json(path.join("schema.json"))).unwrap(),
        rules: serde_json::from_value(json(path.join("rules.json"))).unwrap(),
        routing: serde_json::from_value(json(path.join("routing.json"))).unwrap(),
    }
}

pub fn check_modifier_transforms(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("modifier-transform-inputs");
    let authored_bytes = bundle(&authored);
    let extension_path = authored.join("extension.json");
    let extension: OwnedRecipeExtension = serde_json::from_value(json(&extension_path)).unwrap();
    let bindings = json(authored.join("bindings.json"));
    let initial: StatDefId = serde_json::from_value(bindings["initial_scalar"].clone()).unwrap();
    let channel: StatDefId = serde_json::from_value(bindings["ordered_scalar"].clone()).unwrap();
    let modifiers: Vec<ModifierDefId> =
        serde_json::from_value(bindings["modifiers"].clone()).unwrap();
    assert_eq!(modifiers.len(), 2);
    let before_bytes = bundle(prior);
    let before = recipe(prior);
    assert_eq!(before.registry.entries.len(), 9532);
    let output = cwd.join("modifier-transform-successor");
    let report = success(publish(cwd, prior, &extension_path, &output));
    assert_eq!(report["extension"]["allocated_entries"], 1);
    assert_eq!(report["extension"]["refined_subjects"], 0);
    assert_eq!(report["extension"]["appended_programs"], 2);
    assert_eq!(report["extension"]["appended_tables"], 0);
    assert_eq!(report["extension"]["appended_receivers"], 0);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );

    let after = recipe(&output);
    assert_eq!(after.registry.entries.len(), 9533);
    for entry in &before.registry.entries {
        assert!(after.registry.entries.contains(entry));
    }
    assert_eq!(
        after.schema.definitions.len(),
        before.schema.definitions.len() + 1
    );
    for definition in &before.schema.definitions {
        assert!(after.schema.definitions.contains(definition));
    }
    assert_eq!(after.schema.slots, before.schema.slots);
    let descriptor = after
        .schema
        .definitions
        .iter()
        .find_map(|d| match d {
            DefinitionDescriptor::Stat(row) if row.id == channel => Some(row),
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(scalar) = &descriptor.schema else {
        panic!("declared factor channel expected")
    };
    assert_eq!(
        scalar.targets,
        vec![poe_optimizer_core::owned_schema::RuleEntityKind::Modifier]
    );
    let ComputedValueType::Quantity { unit } = &scalar.value else {
        panic!("factor quantity expected")
    };
    assert_eq!(
        after.rules.operations_version.as_str(),
        "owned-domain-operations-v10"
    );
    assert_eq!(after.rules.owners.len(), before.rules.owners.len());
    assert_eq!(after.rules.tables, before.rules.tables);
    assert_eq!(after.rules.receivers, before.rules.receivers);
    assert_eq!(after.routing.outputs, before.routing.outputs);

    let selected: Vec<_> = modifiers
        .iter()
        .map(|id| SchemaSubject::Definition(id.address()))
        .collect();
    for previous in &before.rules.owners {
        let current = after
            .rules
            .owners
            .iter()
            .find(|owner| owner.owner == previous.owner)
            .unwrap();
        assert_eq!(current.programs.closure, previous.programs.closure);
        for program in &previous.programs.members {
            assert!(current.programs.members.contains(program));
        }
        assert_eq!(
            current.programs.members.len(),
            previous.programs.members.len() + usize::from(selected.contains(&current.owner))
        );
        if !selected.contains(&current.owner) {
            continue;
        }
        let SchemaClosure::Partial { gaps } = &current.programs.closure else {
            panic!("intermediate scalar cannot close modifier coverage")
        };
        assert!(!gaps.is_empty());
        let program = current
            .programs
            .members
            .iter()
            .find(|p| p.id.as_str() == "ordered-magnitude-scalar")
            .unwrap();
        assert_eq!(program.reads.len(), 1);
        assert_eq!(
            program.reads[0].value_type,
            ComputedValueType::Quantity { unit: unit.clone() }
        );
        assert!(matches!(&program.reads[0].source,
            RuleReadSource::ModifierTransforms { stat, initial: base }
            if stat == &channel && base == &initial));
        assert_eq!(program.effects.len(), 1);
        assert!(matches!(&program.effects[0].effect,
            RuleEffectKind::Derive { entity: RuleEntity::Modifier, stat, .. } if stat == &channel));
    }
    let checked = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    // Semantic compilation validates the new primitive, scopes, and exact units.
    // Supplying a fake fold result would not test the native plan's ordered fold.
    CompiledRulePackage::compile(
        checked.rules().input(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let idempotent = extend_owned_recipe(&checked, &extension, Default::default()).unwrap();
    assert_eq!(idempotent.receipt.allocated_entries, 0);
    assert_eq!(idempotent.receipt.refined_subjects, 0);
    assert_eq!(idempotent.receipt.appended_programs, 0);
    assert_eq!(idempotent.receipt.appended_tables, 0);
    assert_eq!(idempotent.receipt.appended_receivers, 0);
    assert_eq!(idempotent.successor, after);

    let published_bytes = bundle(&output);
    assert!(
        !publish(cwd, prior, &extension_path, &output)
            .status
            .success()
    );
    assert_eq!(bundle(&output), published_bytes);
    let replay = cwd.join("modifier-transform-replay");
    let replay_report = success(publish(cwd, prior, &extension_path, &replay));
    assert_eq!(replay_report["extension"], report["extension"]);
    assert_eq!(bundle(&replay), published_bytes);

    let mut query_rows = 0;
    let mut rune_items = 0;
    for case in 1..=5 {
        let query_file = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(output.join(&query_file)).unwrap(),
            before_bytes[&query_file]
        );
        query_rows += json(output.join(&query_file)).as_array().unwrap().len();
        let normalized = cwd.join(format!("modifier-transform-original-{case}"));
        let normalization = success(normalize(cwd, &output, case, &normalized, true));
        assert_eq!(normalization["normalization_status"], "pending");
        let draft = decode_draft(
            &fs::read(normalized.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let input = draft.input();
        assert_eq!(
            input
                .query_presets
                .members
                .iter()
                .map(|preset| preset.queries.requests.members.len())
                .sum::<usize>(),
            22
        );
        assert_eq!(input.items.members.len(), [16, 34, 17, 21, 28][case - 1]);
        assert!(!input.allocations.members.is_empty());
        assert!(
            input
                .allocations
                .members
                .iter()
                .all(|a| matches!(a.access, DraftAllocationAccess::Pending(_)))
        );
        let sidecar = json(normalized.join("sidecar.json"));
        let mut runes_in_case = 0;
        for item_text in sidecar["item_texts"].as_array().unwrap() {
            let attribution = &item_text["attribution"];
            let layout_gap = attribution["layout"]["problems"]
                .as_array()
                .is_some_and(|problems| problems.iter().any(|problem| problem == "rune_lifecycle"));
            let line_gap = attribution["lines"].as_array().unwrap().iter().any(|line| {
                line["blockers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|problem| problem == "rune_lifecycle")
                    || (!line["presentation"].as_bool().unwrap()
                        && line["raw"]
                            .as_str()
                            .unwrap()
                            .trim_ascii()
                            .starts_with("Rune:"))
            });
            if !layout_gap && !line_gap {
                continue;
            }
            runes_in_case += 1;
            let origin = sidecar["origins"]
                .as_array()
                .unwrap()
                .iter()
                .find(|origin| origin["source"] == item_text["source"])
                .unwrap();
            let item_id = &origin["links"]
                .as_array()
                .unwrap()
                .iter()
                .find(|link| link["kind"] == "item")
                .unwrap()["value"];
            let item = input
                .items
                .members
                .iter()
                .find(|item| serde_json::to_value(item.id).unwrap() == *item_id)
                .unwrap();
            assert!(item.to_resolved().is_none());
            assert!(matches!(item.modifier_order, DraftField::Pending(_)));
            assert!(matches!(
                item.modifiers.completion,
                DraftListCompletion::Pending { .. }
            ));
        }
        if matches!(case, 2 | 3) {
            assert!(runes_in_case > 0, "fixture lost its rune-bearing item");
        }
        if runes_in_case > 0 {
            for (completion, expected) in [
                (
                    &input.items.completion,
                    "socketed-item-membership-not-converted",
                ),
                (
                    &input.equipment.completion,
                    "socketed-equipment-membership-not-converted",
                ),
            ] {
                let DraftListCompletion::Pending { code, .. } = completion else {
                    panic!("unmaterialized augment children cannot close collection membership")
                };
                assert_eq!(code.as_str(), expected);
            }
        }
        rune_items += runes_in_case;
    }
    assert_eq!(query_rows, 110);
    assert!(rune_items > 0);
    assert_eq!(bundle(prior), before_bytes);
    assert_eq!(bundle(&output), published_bytes);
    assert_eq!(bundle(&authored), authored_bytes);
    output
}
