//! Canonical inputs publish without reinterpreting nominal inputs or closing coverage.
//! Numeric conformance is covered separately; these tests exercise the real data chain.
use super::support::{bundle, data, json, normalize, success};
use poe_optimizer_core::{
    build_identity::{BuildLineage, InstanceAllocatorState, InstanceId},
    owned_definitions::{ModifierDefId, StatDefId},
    owned_draft::{DraftAllocationAccess, DraftLimits, DraftListCompletion, decode_draft},
    owned_rules::{RuleEffectKind, RuleEntity, RuleReadSource},
    owned_schema::{
        DefinitionDescriptor, SchemaClosure, SchemaDefinitionId, SchemaState, SchemaSubject,
    },
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition};
use poe_optimizer_import::{
    owned_modifier_value_recipe::{ModifierValuePolicy, compile_owned_modifier_values},
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn run(cwd: &Path, command: &str, prior: &Path, flag: &str, input: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg(command)
        .arg(prior)
        .arg(flag)
        .arg(input)
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

// Fresh CLI imports deliberately allocate different host lineages. Alpha-rename
// only decoded InstanceId/InstanceAllocatorState objects, preserving local IDs,
// watermarks, order and every other semantic value. Unknown lineage-bearing
// shapes or foreign lineages fail instead of silently dropping an identity field.
fn canonical_instances(
    mut value: serde_json::Value,
    expected: BuildLineage,
) -> (serde_json::Value, usize) {
    fn visit(value: &mut serde_json::Value, expected: BuildLineage) -> usize {
        match value {
            serde_json::Value::Object(object) if object.contains_key("lineage") => {
                let normalized = BuildLineage::from_bytes([0; 16]);
                if object.contains_key("local") {
                    let id: InstanceId = serde_json::from_value(value.clone())
                        .expect("expected exact instance identity encoding");
                    assert_eq!(id.lineage(), expected, "foreign instance lineage");
                    *value = serde_json::to_value(
                        InstanceId::from_parts(normalized, id.local()).unwrap(),
                    )
                    .unwrap();
                } else {
                    let allocator: InstanceAllocatorState = serde_json::from_value(value.clone())
                        .expect("expected exact allocator encoding");
                    assert_eq!(allocator.lineage(), expected, "foreign allocator lineage");
                    *value = serde_json::to_value(InstanceAllocatorState::from_parts(
                        normalized,
                        allocator.last_issued(),
                    ))
                    .unwrap();
                }
                1
            }
            serde_json::Value::Object(object) => {
                object.values_mut().map(|v| visit(v, expected)).sum()
            }
            serde_json::Value::Array(values) => values.iter_mut().map(|v| visit(v, expected)).sum(),
            _ => 0,
        }
    }
    let count = visit(&mut value, expected);
    (value, count)
}

pub fn check_modifier_values(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("modifier-value-inputs");
    let authored_bytes = bundle(&authored);
    let extension_path = authored.join("extension.json");
    let extension: OwnedRecipeExtension = serde_json::from_value(json(&extension_path)).unwrap();
    let bindings = json(authored.join("bindings.json"));
    let canonical: BTreeSet<ModifierDefId> = bindings["families"]
        .as_array()
        .unwrap()
        .iter()
        .map(|family| serde_json::from_value(family["canonical"].clone()).unwrap())
        .collect();
    assert_eq!(canonical.len(), 11);
    let before_bytes = bundle(prior);
    let before = recipe(prior);
    assert_eq!(before.registry.entries.len(), 9533);
    let stage = cwd.join("modifier-value-inputs-successor");
    let report = success(run(
        cwd,
        "extend-owned-recipe",
        prior,
        "--extension",
        &extension_path,
        &stage,
    ));
    assert_eq!(report["extension"]["allocated_entries"], 310);
    assert_eq!(report["extension"]["refined_subjects"], 338);
    assert_eq!(report["extension"]["appended_programs"], 4);
    assert_eq!(report["extension"]["appended_tables"], 0);
    assert_eq!(report["extension"]["appended_receivers"], 0);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    let staged = recipe(&stage);
    assert_eq!(staged.registry.entries.len(), 9843);
    for entry in &before.registry.entries {
        assert!(staged.registry.entries.contains(entry));
    }
    assert_eq!(
        staged.schema.definitions.len(),
        before.schema.definitions.len() + 15
    );
    assert_eq!(staged.schema.slots.len(), before.schema.slots.len() + 295);
    for slot in &before.schema.slots {
        assert!(
            staged.schema.slots.contains(slot),
            "old nominal and property contracts are immutable"
        );
    }
    let mut refined = 0;
    for old in &before.schema.definitions {
        let new = staged
            .schema
            .definitions
            .iter()
            .find(|entry| entry.address() == old.address())
            .unwrap();
        if new == old {
            continue;
        }
        let (DefinitionDescriptor::ItemTemplate(old), DefinitionDescriptor::ItemTemplate(new)) =
            (old, new)
        else {
            panic!("only existing Partial item memberships may change")
        };
        let (SchemaState::Known(old), SchemaState::Known(new)) = (&old.schema, &new.schema) else {
            panic!("known item template expected")
        };
        assert!(matches!(
            old.modifiers.closure,
            SchemaClosure::Partial { .. }
        ));
        assert_eq!(new.modifiers.closure, old.modifiers.closure);
        assert!(new.modifiers.members.len() > old.modifiers.members.len());
        let mut restored = new.clone();
        restored
            .modifiers
            .members
            .retain(|id| !canonical.contains(id));
        assert_eq!(
            &restored, old,
            "all other template contracts must remain exact"
        );
        refined += 1;
    }
    assert_eq!(refined, 338);
    for owner in &before.rules.owners {
        assert!(
            staged.rules.owners.contains(owner),
            "predecessor programs and coverage stay exact"
        );
    }
    assert_eq!(staged.rules.owners.len(), before.rules.owners.len() + 11);
    assert_eq!(staged.rules.tables, before.rules.tables);
    assert_eq!(staged.rules.receivers, before.rules.receivers);
    assert_eq!(staged.routing.outputs, before.routing.outputs);
    let checked_stage = assemble_owned_recipe(staged.clone(), Default::default()).unwrap();
    let repeated = extend_owned_recipe(&checked_stage, &extension, Default::default()).unwrap();
    assert_eq!(repeated.receipt.allocated_entries, 0);
    assert_eq!(repeated.receipt.refined_subjects, 0);
    assert_eq!(repeated.receipt.appended_programs, 0);
    assert_eq!(repeated.successor, staged);

    // Bind only after the full extension has passed schema and rule validation.
    let policy: ModifierValuePolicy = serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "version": "canonical-modifier-values-v1",
        "definitions": checked_stage.schema().identity(),
        "factor_unit": bindings["factor_unit"],
        "bindings": bindings["compiler_bindings"],
    }))
    .unwrap();
    let tracked_policy: ModifierValuePolicy =
        serde_json::from_value(json(authored.join("policy.json"))).unwrap();
    assert_eq!(
        tracked_policy, policy,
        "tracked policy must bind this exact published schema"
    );
    assert_eq!(policy.bindings.len(), 16);
    assert_eq!(
        policy
            .bindings
            .iter()
            .map(|b| b.modifier.clone())
            .collect::<BTreeSet<_>>(),
        canonical
    );
    let policy_path = cwd.join("modifier-value-policy.json");
    fs::write(&policy_path, serde_json::to_vec_pretty(&policy).unwrap()).unwrap();
    let stage_bytes = bundle(&stage);
    assert!(
        !run(
            cwd,
            "extend-owned-recipe",
            prior,
            "--extension",
            &extension_path,
            &stage
        )
        .status
        .success()
    );
    assert_eq!(bundle(&stage), stage_bytes);
    let output = cwd.join("modifier-value-successor");
    let compiled_report = success(run(
        cwd,
        "compile-owned-modifier-values",
        &stage,
        "--policy",
        &policy_path,
        &output,
    ));
    assert_eq!(compiled_report["modifier_values"]["bindings"], 16);
    assert_eq!(compiled_report["modifier_values"]["owners"], 11);
    assert_eq!(
        compiled_report["modifier_values"]["extension"]["allocated_entries"],
        0
    );
    assert_eq!(
        compiled_report["modifier_values"]["extension"]["refined_subjects"],
        0
    );
    assert_eq!(
        compiled_report["modifier_values"]["extension"]["appended_programs"],
        16
    );
    assert_eq!(compiled_report["publication"]["query_rows"], 110);
    assert_eq!(
        compiled_report["publication"]["whole_build_parity"],
        "not_established"
    );
    let after = recipe(&output);
    assert_eq!(after.registry, staged.registry);
    assert_eq!(after.schema, staged.schema);
    assert_eq!(after.rules.tables, staged.rules.tables);
    assert_eq!(after.rules.receivers, staged.rules.receivers);
    assert_eq!(after.routing, staged.routing);
    for old in &staged.rules.owners {
        let new = after
            .rules
            .owners
            .iter()
            .find(|owner| owner.owner == old.owner)
            .unwrap();
        assert_eq!(new.programs.closure, old.programs.closure);
        assert!(
            old.programs
                .members
                .iter()
                .all(|program| new.programs.members.contains(program))
        );
        let expected = policy
            .bindings
            .iter()
            .filter(|binding| old.owner == SchemaSubject::Definition(binding.modifier.address()))
            .count();
        assert_eq!(
            new.programs.members.len(),
            old.programs.members.len() + expected
        );
    }
    assert_eq!(after.rules.owners.len(), staged.rules.owners.len());
    let checked = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(
        checked.rules().input(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let mut scratch = compiled.new_scratch();
    let corrupted_base: StatDefId =
        serde_json::from_value(bindings["stats"]["corrupted_base_factor"].clone()).unwrap();
    let magnitude: StatDefId =
        serde_json::from_value(bindings["ordered_magnitude"].clone()).unwrap();
    for binding in &policy.bindings {
        let subject = SchemaSubject::Definition(binding.modifier.address());
        let owner = after
            .rules
            .owners
            .iter()
            .find(|owner| owner.owner == subject)
            .unwrap();
        let SchemaClosure::Partial { gaps } = &owner.programs.closure else {
            panic!("numeric component compilation cannot certify the modifier")
        };
        assert!(
            gaps.iter()
                .any(|gap| gap.code.as_str() == "canonical-input-admission-unproved")
        );
        let program = owner
            .programs
            .members
            .iter()
            .find(|p| p.id == binding.program)
            .unwrap();
        assert!(program.reads.iter().any(|read| matches!(&read.source,
            RuleReadSource::Parameter { slot } if slot == &binding.input)));
        assert!(program.reads.iter().any(|read| matches!(&read.source,
            RuleReadSource::Stat { entity: RuleEntity::Modifier, stat } if stat == &corrupted_base)));
        assert!(program.reads.iter().any(|read| matches!(&read.source,
            RuleReadSource::Stat { entity: RuleEntity::Modifier, stat } if stat == &magnitude)));
        assert_eq!(program.effects.len(), 1);
        assert!(matches!(&program.effects[0].effect,
            RuleEffectKind::Derive { entity: RuleEntity::Modifier, stat, .. } if stat == &binding.output));
        // Real authored bindings must not derive a value when raw inputs/factors are absent.
        let result = compiled
            .evaluate(
                &subject,
                &binding.program,
                &[],
                checked.schema(),
                &mut scratch,
            )
            .unwrap();
        assert!(matches!(
            result.effects[0].disposition,
            EffectDisposition::Unresolved { .. }
        ));
        assert_eq!(result.owner_programs_closure, owner.programs.closure);
        assert!(
            !owner
                .programs
                .members
                .iter()
                .flat_map(|p| &p.effects)
                .any(|effect| matches!(&effect.effect,
            RuleEffectKind::Derive { stat, .. } if stat == &corrupted_base)),
            "no implicit corrupted-base default"
        );
        let family = bindings["families"]
            .as_array()
            .unwrap()
            .iter()
            .find(|family| family["canonical"] == serde_json::to_value(&binding.modifier).unwrap())
            .unwrap();
        let has_magnitude = owner
            .programs
            .members
            .iter()
            .flat_map(|p| &p.effects)
            .any(|effect| {
                matches!(&effect.effect,
            RuleEffectKind::Derive { stat, .. } if stat == &magnitude)
            });
        assert_eq!(
            has_magnitude,
            family["magnitude_writer"] == "cloned-reviewed-cold-scalar-programs"
        );
    }
    let idempotent = compile_owned_modifier_values(&checked, &policy, Default::default()).unwrap();
    assert_eq!(idempotent.receipt.extension.appended_programs, 0);
    assert_eq!(idempotent.successor, after);
    let published_bytes = bundle(&output);
    assert!(
        !run(
            cwd,
            "compile-owned-modifier-values",
            &stage,
            "--policy",
            &policy_path,
            &output
        )
        .status
        .success()
    );
    assert_eq!(bundle(&output), published_bytes);
    let replay = cwd.join("modifier-value-replay");
    let replay_report = success(run(
        cwd,
        "compile-owned-modifier-values",
        &stage,
        "--policy",
        &policy_path,
        &replay,
    ));
    assert_eq!(replay_report, compiled_report);
    assert_eq!(bundle(&replay), published_bytes);
    let mut wrong = policy.clone();
    wrong.definitions = assemble_owned_recipe(before.clone(), Default::default())
        .unwrap()
        .schema()
        .identity()
        .clone();
    let wrong_path = cwd.join("modifier-value-wrong-policy.json");
    fs::write(&wrong_path, serde_json::to_vec(&wrong).unwrap()).unwrap();
    let rejected = cwd.join("modifier-value-rejected");
    assert!(
        !run(
            cwd,
            "compile-owned-modifier-values",
            &stage,
            "--policy",
            &wrong_path,
            &rejected
        )
        .status
        .success()
    );
    assert!(!rejected.exists());

    let mut query_rows = 0;
    for case in 1..=5 {
        let query_file = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(output.join(&query_file)).unwrap(),
            before_bytes[&query_file]
        );
        query_rows += json(output.join(&query_file)).as_array().unwrap().len();
        let normalized = cwd.join(format!("modifier-value-original-{case}"));
        let normalization = success(normalize(cwd, &output, case, &normalized, true));
        assert_eq!(normalization["normalization_status"], "pending");
        let draft = decode_draft(
            &fs::read(normalized.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let previous = decode_draft(
            &fs::read(cwd.join(format!("modifier-transform-original-{case}/draft.json"))).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let current_lineage = draft.input().allocator.lineage();
        let prior_lineage = previous.input().allocator.lineage();
        let (current_records, current_ids) =
            canonical_instances(serde_json::to_value(&draft).unwrap(), current_lineage);
        let (prior_records, prior_ids) =
            canonical_instances(serde_json::to_value(&previous).unwrap(), prior_lineage);
        assert!(current_ids > 0);
        assert_eq!(current_ids, prior_ids);
        assert!(
            current_records == prior_records,
            "original-{case}: owned records changed beyond fresh lineage"
        );
        let current_sidecar = json(normalized.join("sidecar.json"));
        let prior_sidecar =
            json(cwd.join(format!("modifier-transform-original-{case}/sidecar.json")));
        for field in ["source_sha256", "source_bytes", "source_schema", "revision"] {
            assert_eq!(
                current_sidecar[field], prior_sidecar[field],
                "original-{case}: {field}"
            );
        }
        for field in [
            "allocator_before",
            "allocator_after",
            "source_allocator",
            "origins",
        ] {
            let (current, current_ids) =
                canonical_instances(current_sidecar[field].clone(), current_lineage);
            let (prior, prior_ids) =
                canonical_instances(prior_sidecar[field].clone(), prior_lineage);
            assert!(current_ids > 0);
            assert_eq!(current_ids, prior_ids);
            assert!(
                current == prior,
                "original-{case}: {field} changed beyond fresh lineage"
            );
        }
        // Package identities belong to the sidecar and must bind each actual
        // schema; a declaration-only successor legitimately changes these.
        assert_eq!(
            current_sidecar["definitions"],
            serde_json::to_value(checked.schema().identity()).unwrap()
        );
        assert_eq!(
            prior_sidecar["definitions"],
            serde_json::to_value(&before.rules.definitions).unwrap()
        );
        assert_ne!(current_sidecar["definitions"], prior_sidecar["definitions"]);
        let input = draft.input();
        assert_eq!(
            input
                .query_presets
                .members
                .iter()
                .map(|p| p.queries.requests.members.len())
                .sum::<usize>(),
            22
        );
        assert!(!input.allocations.members.is_empty());
        assert!(
            input
                .allocations
                .members
                .iter()
                .all(|a| matches!(a.access, DraftAllocationAccess::Pending(_)))
        );
        if matches!(case, 2 | 3) {
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
                    panic!("rune membership cannot become proven empty")
                };
                assert_eq!(code.as_str(), expected);
            }
        }
    }
    assert_eq!(query_rows, 110);
    assert_eq!(bundle(prior), before_bytes);
    assert_eq!(bundle(&stage), stage_bytes);
    assert_eq!(bundle(&output), published_bytes);
    assert_eq!(bundle(&authored), authored_bytes);
    output
}
