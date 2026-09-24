//! Authored quality admission reuses the existing EquipmentUse input projection.
use super::{
    passive_attributes::check_rebound_inputs,
    scalar_families::recipe,
    skill_scopes::canonical_instances,
    support::{bundle, data, json, normalize, success},
};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::*,
    owned_draft::{DraftField, DraftLimits, DraftQuality, decode_draft},
    owned_rules::{RuleEffectKind, RuleEntity, RuleProgram, RuleReadSource},
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact};
use poe_optimizer_import::{
    owned_defence_profiles::{DefenceProfileCatalog, DefenceProfilePresence},
    owned_item_bases::ItemBasePolicy,
    owned_item_lines::{ItemField, OwnedItemLinePolicy},
    owned_item_source::ItemSourceLayoutPolicy,
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

const PROGRAM: &str = "declared-standard-item-quality";

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

fn quality_program<'a>(
    recipe: &'a OwnedRecipeInput,
    template: &ItemTemplateDefId,
) -> Option<&'a RuleProgram> {
    recipe
        .rules
        .owners
        .iter()
        .find(|owner| owner.owner == SchemaSubject::Definition(template.address()))
        .and_then(|owner| {
            owner
                .programs
                .members
                .iter()
                .find(|p| p.id.as_str() == PROGRAM)
        })
}

fn check_structure(
    before: &OwnedRecipeInput,
    after: &OwnedRecipeInput,
    bindings: &Value,
) -> (BTreeSet<ItemTemplateDefId>, RuleProgram) {
    let catalog: DefenceProfileCatalog =
        serde_json::from_value(json(data().join("defence-profiles/catalog.json"))).unwrap();
    let bases: ItemBasePolicy =
        serde_json::from_value(json(data().join("item-bases/policy.json"))).unwrap();
    assert_eq!(catalog.profiles.len(), 1_756);
    assert_eq!(bases.templates.len(), 1_756);
    let present: BTreeSet<_> = catalog
        .profiles
        .iter()
        .filter_map(|row| {
            matches!(row.profile, DefenceProfilePresence::Table { .. }).then_some(row.base.as_str())
        })
        .collect();
    assert_eq!(present.len(), 1_240);
    let selected: BTreeSet<_> = bases
        .templates
        .iter()
        .filter(|row| present.contains(row.source_base.as_str()))
        .map(|row| row.template.clone())
        .collect();
    let declared: BTreeSet<ItemTemplateDefId> = bindings["templates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let source = row["source_base"].as_str().unwrap();
            assert!(present.contains(source));
            let template = serde_json::from_value(row["template"].clone()).unwrap();
            assert!(
                bases
                    .templates
                    .iter()
                    .any(|base| base.source_base == source && base.template == template)
            );
            template
        })
        .collect();
    assert_eq!(declared, selected);
    assert_eq!(
        bindings["templates"].as_array().unwrap().len(),
        selected.len()
    );
    assert_eq!(before.registry.last_issued.get(), 11_228);
    assert_eq!(after.registry, before.registry);
    assert_eq!(after.schema.slots, before.schema.slots);
    assert_eq!(
        after.schema.definitions.len(),
        before.schema.definitions.len()
    );
    let quality: QualityDefId =
        serde_json::from_value(bindings["standard_quality"].clone()).unwrap();
    let unit: UnitDefId = serde_json::from_value(bindings["unit"].clone()).unwrap();
    let stat: StatDefId = serde_json::from_value(bindings["quality_stat"].clone()).unwrap();
    assert_eq!(quality.key().as_str(), "def.0000000000000006");
    assert_eq!(unit.key().as_str(), "def.0000000000000002");
    assert_eq!(stat.key().as_str(), "def.0000000000002427");
    assert_eq!(bindings["program"], PROGRAM);
    let existing = before
        .rules
        .owners
        .iter()
        .flat_map(|owner| &owner.programs.members)
        .find(|program| program.id.as_str() == PROGRAM)
        .unwrap()
        .clone();
    assert_eq!(existing.context, RuleEntityKind::EquipmentUse);
    assert_eq!(existing.reads.len(), 1);
    assert_eq!(
        existing.reads[0].source,
        RuleReadSource::ItemQualityAmount {
            quality: quality.clone()
        }
    );
    assert_eq!(
        existing.reads[0].value_type,
        ComputedValueType::Quantity { unit }
    );
    assert_eq!(existing.effects.len(), 1);
    assert!(existing.effects[0].when.is_none());
    assert!(matches!(&existing.effects[0].effect,
        RuleEffectKind::Derive { entity: RuleEntity::Current, stat: target, .. } if target == &stat));
    let mut restored_schema = after.schema.clone();
    let mut additions = 0;
    for descriptor in &mut restored_schema.definitions {
        let DefinitionDescriptor::ItemTemplate(entry) = descriptor else {
            continue;
        };
        if !selected.contains(&entry.id) {
            continue;
        }
        let old = before
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == entry.id.address())
            .unwrap();
        let DefinitionDescriptor::ItemTemplate(old_entry) = old else {
            unreachable!()
        };
        let (SchemaState::Known(current), SchemaState::Known(previous)) =
            (&mut entry.schema, &old_entry.schema)
        else {
            panic!("known template")
        };
        assert_eq!(current.quality.presence, previous.quality.presence);
        assert_eq!(
            current.quality.allowed_kinds.closure,
            previous.quality.allowed_kinds.closure
        );
        assert!(!current.quality.allowed_kinds.is_complete());
        let mut expected = previous.quality.allowed_kinds.members.clone();
        if !expected.contains(&quality) {
            expected.push(quality.clone());
            expected.sort();
            additions += 1;
        }
        assert_eq!(current.quality.allowed_kinds.members, expected);
        current.quality.allowed_kinds = previous.quality.allowed_kinds.clone();
        assert_eq!(descriptor, old, "only quality membership may change");
    }
    assert_eq!(restored_schema, before.schema, "unrelated schema changed");
    assert_eq!(bindings["added_memberships"], additions);
    let mut restored = after.rules.clone();
    restored.definitions = before.rules.definitions.clone();
    let mut appended = 0;
    for base in &bases.templates {
        let previous = quality_program(before, &base.template);
        let current = quality_program(after, &base.template);
        if selected.contains(&base.template) {
            assert_eq!(current, Some(&existing));
            if let Some(previous) = previous {
                assert_eq!(previous, &existing);
            } else {
                appended += 1;
                let owner = restored
                    .owners
                    .iter_mut()
                    .find(|owner| owner.owner == SchemaSubject::Definition(base.template.address()))
                    .unwrap();
                assert!(!owner.programs.is_complete());
                owner
                    .programs
                    .members
                    .retain(|program| program.id.as_str() != PROGRAM);
            }
        } else {
            assert_eq!(current, previous, "absent profile template changed");
        }
    }
    assert_eq!(
        restored, before.rules,
        "no new assembly, receiver, or arithmetic layer"
    );
    assert_eq!(bindings["appended_programs"], appended);
    let mut routing = after.routing.clone();
    routing.definitions = before.routing.definitions.clone();
    assert_eq!(routing, before.routing);
    (selected, existing)
}

fn check_headers(lines: &OwnedItemLinePolicy, selected: &BTreeSet<ItemTemplateDefId>) {
    let bases: ItemBasePolicy =
        serde_json::from_value(json(data().join("item-bases/policy.json"))).unwrap();
    for base in &bases.templates {
        if !selected.contains(&base.template) {
            continue;
        }
        for amount in [0.0, 20.0, 1_000_000.0] {
            let text = format!("{}\nQuality: {amount}\n", base.source_base);
            let converted = lines.convert_text(&text).unwrap();
            let ItemField::Known { value, .. } = converted.quality else {
                panic!("{} quality {amount}", base.source_base)
            };
            assert_eq!(value.amount.value(), amount);
            assert_eq!(value.kind.key().as_str(), "def.0000000000000006");
            assert_eq!(value.amount.unit().key().as_str(), "def.0000000000000002");
        }
    }
    let base = bases
        .templates
        .iter()
        .find(|base| selected.contains(&base.template))
        .unwrap();
    for header in [
        "",
        "Quality: nope",
        "Quality: -1",
        "Quality: +20",
        "Quality: 20.5",
        "Quality: 20%",
        "Quality: 1000001",
        "Quality: 100000000000000000000000",
        "Quality: 20\nQuality: 20",
    ] {
        let text = format!("{}\n{header}\n", base.source_base);
        assert!(
            !matches!(
                lines.convert_text(&text).unwrap().quality,
                ItemField::Known { .. }
            ),
            "{header}"
        );
    }
    for text in [
        "Sapphire Ring\nQuality: 20\n",
        "Unrecognized Base\nQuality: 20\n",
    ] {
        assert!(
            !matches!(
                lines.convert_text(text).unwrap().quality,
                ItemField::Known { .. }
            ),
            "{text}"
        );
    }
}

fn check_components(
    after: &OwnedRecipeInput,
    selected: &BTreeSet<ItemTemplateDefId>,
    program: &RuleProgram,
) {
    let checked = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(
        checked.rules().input(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let mut scratch = compiled.new_scratch();
    let ComputedValueType::Quantity { unit } = &program.reads[0].value_type else {
        panic!("quality unit")
    };
    // Every authored template instantiates the same existing numerical input projection.
    // Concrete EquipmentUse read authority remains exercised by Engine occurrence tests.
    for template in selected {
        let subject = SchemaSubject::Definition(template.address());
        for amount in [0.0, 20.0, 1_000_000.0] {
            let value =
                ParameterValue::Quantity(FiniteQuantity::new(amount, unit.clone()).unwrap());
            let result = compiled
                .evaluate(
                    &subject,
                    &program.id,
                    &[RuleFact {
                        read: program.reads[0].id.clone(),
                        value: value.clone(),
                    }],
                    checked.schema(),
                    &mut scratch,
                )
                .unwrap();
            assert_eq!(result.effects.len(), 1);
            assert_eq!(
                result.effects[0].disposition,
                EffectDisposition::Applied { value }
            );
        }
        let missing = compiled
            .evaluate(&subject, &program.id, &[], checked.schema(), &mut scratch)
            .unwrap();
        assert!(
            matches!(&missing.effects[0].disposition, EffectDisposition::Unresolved { input } if input == &program.reads[0].id)
        );
    }
}

// Join by the exact origin Item link, never by item enumeration or display name.
// The expected number comes from the retained authored text, not a converted roll.
fn source_quality(sidecar: &Value, item: &Value) -> (Value, u32) {
    let origins: Vec<_> = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|origin| {
            origin["links"]
                .as_array()
                .unwrap()
                .iter()
                .any(|link| link["kind"] == "item" && &link["value"] == item)
        })
        .collect();
    assert_eq!(origins.len(), 1, "exactly one physical source item origin");
    let source = origins[0]["source"].clone();
    let items: Vec<_> = sidecar["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|text| text["source"] == source)
        .collect();
    assert_eq!(items.len(), 1, "exactly one source item text");
    let headers: Vec<_> = items[0]["lines"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|line| line["text"].as_str().unwrap().starts_with("Quality: "))
        .collect();
    assert_eq!(headers.len(), 1, "exactly one authored quality header");
    assert_eq!(headers[0]["outcome"]["kind"], "known");
    let number = headers[0]["text"]
        .as_str()
        .unwrap()
        .strip_prefix("Quality: ")
        .unwrap();
    assert!(!number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()));
    let amount: u32 = number.parse().unwrap();
    assert!(amount <= 1_000_000);
    (source, amount)
}

fn check_originals(cwd: &Path, output: &Path, selected: &BTreeSet<ItemTemplateDefId>) {
    let expected = [5, 9, 4, 4, 10];
    let mut summary = Vec::new();
    for case in 1..=5 {
        let previous_path = cwd.join(format!("item-defence-inputs-original-{case}"));
        let destination = cwd.join(format!("item-quality-inputs-original-{case}"));
        let report = success(normalize(cwd, output, case, &destination, true));
        assert_eq!(report["normalization_status"], "pending");
        assert_eq!(report["verification"]["calculation"], "not_run");
        let previous = decode_draft(
            &fs::read(previous_path.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let draft = decode_draft(
            &fs::read(destination.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let old_sidecar = json(previous_path.join("sidecar.json"));
        let current_sidecar = json(destination.join("sidecar.json"));
        let (old, current) = (previous.input(), draft.input());
        assert_eq!(old.items.members.len(), current.items.members.len());
        let mut removed = Vec::new();
        let mut quality_witnesses = Vec::new();
        for (old, current) in old.items.members.iter().zip(&current.items.members) {
            if old.quality.to_resolved() == current.quality.to_resolved() {
                continue;
            }
            let DraftQuality::Pending(pending) = &old.quality else {
                panic!("prior known quality changed")
            };
            assert_eq!(pending.code.as_str(), "quality-not-converted");
            let Some(Some(quality)) = current.quality.to_resolved() else {
                panic!("quality not resolved")
            };
            assert_eq!(quality.kind.key().as_str(), "def.0000000000000006");
            assert_eq!(quality.amount.unit().key().as_str(), "def.0000000000000002");
            assert!(
                matches!(&old.template, DraftField::Known { value } if selected.contains(value))
            );
            let (source, expected_amount) =
                source_quality(&old_sidecar, &serde_json::to_value(old.id).unwrap());
            let (current_source, current_amount) =
                source_quality(&current_sidecar, &serde_json::to_value(current.id).unwrap());
            assert_eq!(
                source, current_source,
                "quality must remain attached to the same source item"
            );
            assert_eq!(expected_amount, current_amount);
            assert_eq!(
                quality.amount.value(),
                f64::from(expected_amount),
                "original {case}: wrong quality for source {source}"
            );
            quality_witnesses.push(json!({"source":source,"amount":expected_amount}));
            removed.push(pending.id.instance_id().local());
        }
        assert_eq!(removed.len(), expected[case - 1]);
        if case == 2 {
            assert_eq!(
                quality_witnesses
                    .iter()
                    .map(|row| row["amount"].as_u64().unwrap())
                    .collect::<Vec<_>>(),
                [0, 0, 20, 20, 20, 19, 0, 0, 0],
                "Twister project retains nineteen and explicit zero instead of a twenty default"
            );
        }
        assert_eq!(
            old.allocator.last_issued() - current.allocator.last_issued(),
            removed.len() as u64
        );
        let mut old_wire = serde_json::to_value(old).unwrap();
        let mut current_wire = serde_json::to_value(current).unwrap();
        for (old, current) in old_wire["items"]["members"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .zip(current_wire["items"]["members"].as_array().unwrap())
        {
            if old["quality"]["kind"] == "pending" && current["quality"]["kind"] == "known" {
                // Only the source-checked newly resolved value has no instance IDs.
                // Retain prior Pending issue IDs until exact canonical remapping.
                old["quality"] = current["quality"].clone();
            }
        }
        assert_eq!(
            canonical_instances(&mut old_wire, old.allocator.lineage(), &removed),
            canonical_instances(&mut current_wire, current.allocator.lineage(), &[])
        );
        assert_eq!(
            old_wire, current_wire,
            "original {case}: fields beyond quality changed"
        );
        let mut old_items = old_sidecar["item_texts"].clone();
        let mut current_items = current_sidecar["item_texts"].clone();
        assert_eq!(
            old_items.as_array().unwrap().len(),
            current_items.as_array().unwrap().len()
        );
        let mut resolved_issues = 0;
        for (old_item, current_item) in old_items
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .zip(current_items.as_array().unwrap())
        {
            assert_eq!(old_item["source"], current_item["source"]);
            // Artifact identity rebinding is independently validated above. Source
            // membership, every line/roll, defaults and layouts must remain exact.
            for (field, envelope) in [
                ("policy", "item_source_policy"),
                ("item_lines", "item_policy"),
            ] {
                assert_eq!(old_item["attribution"][field], old_sidecar[envelope]);
                assert_eq!(
                    current_item["attribution"][field],
                    current_sidecar[envelope]
                );
                old_item["attribution"][field] = current_item["attribution"][field].clone();
            }
            let quality_lines: BTreeSet<_> = old_item["lines"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|line| {
                    line["text"].as_str().unwrap().starts_with("Quality: ")
                        && line["outcome"]["kind"] == "known"
                })
                .map(|line| line["index"].as_u64().unwrap())
                .collect();
            if old_item["issues"] != current_item["issues"] {
                let issues = old_item["issues"].as_array_mut().unwrap();
                let size = issues.len();
                issues.retain(|issue| {
                    !(issue["problem"] == "schema_partial"
                        && issue["lines"].as_array().is_some_and(|lines| {
                            lines.len() == 1 && quality_lines.contains(&lines[0].as_u64().unwrap())
                        }))
                });
                assert_eq!(
                    size - issues.len(),
                    1,
                    "only one quality admission issue may disappear"
                );
                resolved_issues += 1;
            }
            assert_eq!(old_item["issues"], current_item["issues"]);
        }
        assert_eq!(resolved_issues, expected[case - 1]);
        assert_eq!(
            canonical_instances(&mut old_items, old.allocator.lineage(), &removed),
            canonical_instances(&mut current_items, current.allocator.lineage(), &[])
        );
        assert_eq!(
            old_items, current_items,
            "original {case}: item attribution changed"
        );
        assert!(
            current
                .items
                .members
                .iter()
                .all(|item| item.to_resolved().is_none())
        );
        assert_eq!(
            current
                .query_presets
                .members
                .iter()
                .map(|q| q.queries.requests.members.len())
                .sum::<usize>(),
            22
        );
        summary.push(json!({"original":case,"new_known_quality":removed.len(),"quality_witnesses":quality_witnesses,"queries":22,"normalization":"pending"}));
    }
    fs::write(
        cwd.join("item-quality-inputs-admission-summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
}

pub fn check_item_quality_inputs(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("item-quality-inputs");
    let prior_bytes = bundle(prior);
    let authored_bytes = bundle(&authored);
    let extension_path = authored.join("extension.json");
    let extension: OwnedRecipeExtension = serde_json::from_value(json(&extension_path)).unwrap();
    assert!(
        extension.operations_version.is_none()
            && extension.tables.is_empty()
            && extension.receivers.is_empty()
    );
    let before = recipe(prior);
    let output = cwd.join("item-quality-inputs-successor");
    let report = success(publish(cwd, prior, &extension_path, &output));
    assert_eq!(report["extension"]["allocated_entries"], 0);
    assert_eq!(report["extension"]["appended_tables"], 0);
    assert_eq!(report["extension"]["appended_receivers"], 0);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    let after = recipe(&output);
    let bindings = json(authored.join("bindings.json"));
    let (selected, program) = check_structure(&before, &after, &bindings);
    assert_eq!(
        report["extension"]["refined_subjects"],
        bindings["added_memberships"]
    );
    assert_eq!(
        report["extension"]["appended_programs"],
        bindings["appended_programs"]
    );
    check_rebound_inputs(prior, &output, false, &after);
    let checked = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    let lines = OwnedItemLinePolicy::new(
        serde_json::from_value(json(output.join("items.json"))).unwrap(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    ItemSourceLayoutPolicy::new(
        serde_json::from_value(json(output.join("item-source.json"))).unwrap(),
        &lines,
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    check_headers(&lines, &selected);
    check_components(&after, &selected, &program);
    let replay = extend_owned_recipe(&checked, &extension, Default::default()).unwrap();
    assert_eq!(replay.successor, after);
    assert_eq!(replay.receipt.allocated_entries, 0);
    assert_eq!(replay.receipt.appended_programs, 0);
    assert_eq!(replay.receipt.refined_subjects, 0);
    check_originals(cwd, &output, &selected);
    let published = bundle(&output);
    assert!(
        !publish(cwd, prior, &extension_path, &output)
            .status
            .success()
    );
    assert_eq!(bundle(&output), published);
    assert_eq!(bundle(prior), prior_bytes);
    assert_eq!(bundle(&authored), authored_bytes);
    output
}
