//! Display observations refine source evidence without adding numerical authority.
use super::support::{bundle, data, json, normalize, success};
use poe_optimizer_core::{
    owned_definitions::ItemTemplateDefId,
    owned_draft::{
        DraftAllocationAccess, DraftField, DraftLimits, DraftListCompletion, decode_draft,
    },
};
use poe_optimizer_import::{
    owned_item_lines::{ConvertedItemEmission, ItemLineOutcome, OwnedItemLinePolicy},
    owned_item_source::{ItemSourceDialect, ItemSourceLayoutPolicyInput},
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn publish(cwd: &Path, prior: &Path, catalog: &Path, policy: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("compile-owned-item-observations")
        .arg(prior)
        .arg("--catalog")
        .arg(catalog)
        .arg("--policy")
        .arg(policy)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
fn unchanged(actual: &BTreeMap<String, Vec<u8>>, expected: &BTreeMap<String, Vec<u8>>) {
    assert!(
        actual.keys().eq(expected.keys()),
        "bundle filenames changed"
    );
    for (name, bytes) in expected {
        assert!(actual.get(name) == Some(bytes), "bundle changed: {name}");
    }
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
fn template_at_base_position(
    item: &Value,
    lines: &OwnedItemLinePolicy,
) -> Option<ItemTemplateDefId> {
    let rows: Vec<_> = item["lines"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|line| !line["text"].as_str().unwrap().trim_ascii().is_empty())
        .collect();
    let offset = usize::from(
        rows.first()?
            .get("text")?
            .as_str()?
            .starts_with("Item Class: "),
    );
    let rarity = rows.get(offset)?["text"]
        .as_str()?
        .strip_prefix("Rarity: ")?;
    let base_index = offset
        + if matches!(rarity, "RARE" | "UNIQUE" | "RELIC") {
            2
        } else {
            1
        };
    let base = rows.get(base_index)?;
    let converted = lines
        .convert_line(
            base["index"].as_u64()? as usize,
            base["text"].as_str()?,
            None,
        )
        .unwrap();
    let ItemLineOutcome::Known { emissions, .. } = converted.outcome else {
        return None;
    };
    if let [ConvertedItemEmission::Template { definition }] = emissions.as_slice() {
        Some(definition.clone())
    } else {
        None
    }
}
fn check_originals(cwd: &Path, prior: &Path, output: &Path) {
    let checked = assemble_owned_recipe(recipe(output), Default::default()).unwrap();
    let lines = OwnedItemLinePolicy::new(
        serde_json::from_value(json(output.join("items.json"))).unwrap(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let source: ItemSourceLayoutPolicyInput =
        serde_json::from_value(json(output.join("item-source.json"))).unwrap();
    let ItemSourceDialect::PobExportedSingleTextObservationsV1 {
        preamble_observations,
        ..
    } = &source.dialect
    else {
        panic!("observation dialect")
    };
    let observations: BTreeMap<_, _> = preamble_observations.iter().map(|o| (&o.rule, o)).collect();
    assert!(!observations.is_empty());
    let (mut queries, mut observations_admitted) = (0, 0);
    let mut summary = vec![];
    for case in 1..=5 {
        let name = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(prior.join(&name)).unwrap(),
            fs::read(output.join(&name)).unwrap(),
            "query bytes changed: {name}"
        );
        queries += json(output.join(&name)).as_array().unwrap().len();
        let destination = cwd.join(format!("item-observations-original-{case}"));
        assert_eq!(
            success(normalize(cwd, output, case, &destination, true))["normalization_status"],
            "pending"
        );
        let before = json(cwd.join(format!("source-role-original-{case}/sidecar.json")));
        let after = json(destination.join("sidecar.json"));
        for field in ["source_sha256", "source_bytes", "source_schema", "revision"] {
            assert_eq!(
                before[field], after[field],
                "original-{case}: provenance {field}"
            );
        }
        assert_eq!(
            before["item_texts"].as_array().unwrap().len(),
            after["item_texts"].as_array().unwrap().len()
        );
        let (
            mut old_modifiers,
            mut new_modifiers,
            mut display_candidates,
            mut eligible_candidates,
            mut known_observations,
        ) = (0, 0, 0, 0, 0);
        for item in after["item_texts"].as_array().unwrap() {
            let old = before["item_texts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|old| old["source"] == item["source"])
                .unwrap();
            let template = template_at_base_position(item, &lines);
            let implicit_line = item["lines"]
                .as_array()
                .unwrap()
                .iter()
                .find(|line| line["text"].as_str().unwrap().starts_with("Implicits: "))
                .map(|line| line["index"].as_u64().unwrap());
            let mut observed_fields = BTreeSet::new();
            for row in item["lines"].as_array().unwrap() {
                let previous = old["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|old| old["index"] == row["index"])
                    .unwrap();
                assert_eq!(row["text"], previous["text"], "physical source changed");
                let previous_count = previous["modifiers"].as_array().unwrap().len();
                old_modifiers += previous_count;
                new_modifiers += row["modifiers"].as_array().unwrap().len();
                if previous_count > 0 {
                    assert_eq!(
                        row["outcome"], previous["outcome"],
                        "previous raw modifier/rolls changed: original-{case}, source{} line{}",
                        item["source"]["ordinal"], row["index"]
                    );
                    assert_eq!(
                        row["modifiers"].as_array().unwrap().len(),
                        previous_count,
                        "previous modifier occurrence count changed"
                    );
                }
                let index = row["index"].as_u64().unwrap();
                let raw = row["text"].as_str().unwrap();
                let attributed = item["attribution"]["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|a| a["index"] == row["index"])
                    .unwrap();
                let old_attributed = old["attribution"]["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|a| a["index"] == row["index"])
                    .unwrap();
                assert_eq!(
                    attributed["decoded_span"], old_attributed["decoded_span"],
                    "raw span changed"
                );
                let probe = lines.convert_line(index as usize, raw, None).unwrap();
                let ItemLineOutcome::Known { rule, .. } = probe.outcome else {
                    continue;
                };
                let Some(observation) = observations.get(&rule) else {
                    continue;
                };
                display_candidates += 1;
                let eligible = template
                    .as_ref()
                    .is_some_and(|t| observation.templates.contains(t))
                    && implicit_line.is_some_and(|line| index < line);
                eligible_candidates += usize::from(eligible);
                // A grammar match alone grants nothing. Every accepted observation
                // needs its exact template and reviewed source position, even if a
                // later unresolved XML overlay leaves conversion pending.
                if attributed["blockers"].as_array().unwrap().is_empty() {
                    assert!(
                        eligible,
                        "observation admitted outside its template/preamble: {raw}"
                    );
                    assert!(
                        observed_fields.insert(observation.field.clone()),
                        "duplicate observation field"
                    );
                    assert!(attributed["member"].is_null());
                    assert!(attributed["property_tokens"].as_array().unwrap().is_empty());
                }
                assert!(
                    row["modifiers"].as_array().unwrap().is_empty(),
                    "display total became a modifier"
                );
                if row["outcome"]["kind"] == "known" {
                    known_observations += 1;
                    assert!(eligible);
                    assert!(attributed["blockers"].as_array().unwrap().is_empty());
                    assert_eq!(row["outcome"]["value"]["rule"], rule.as_str());
                    let emissions = row["outcome"]["value"]["emissions"].as_array().unwrap();
                    assert!(
                        !emissions.is_empty() && emissions.iter().all(|e| e["kind"] == "metadata"),
                        "observation wrote gameplay input"
                    );
                }
            }
        }
        assert!(new_modifiers >= old_modifiers);
        observations_admitted += known_observations;
        let draft = decode_draft(
            &fs::read(destination.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
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
        for item in &input.items.members {
            assert!(matches!(
                item.parameters.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert!(matches!(
                item.modifiers.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert!(matches!(item.modifier_order, DraftField::Pending(_)));
            assert!(item.to_resolved().is_none());
            for modifier in &item.modifiers.members {
                if matches!(modifier.rolls.completion, DraftListCompletion::Complete) {
                    let DraftField::Known { value } = &modifier.definition else {
                        panic!("known complete modifier")
                    };
                    assert_eq!(
                        value.key().as_str(),
                        "def.00000000000009da",
                        "canonical eligibility closure changed"
                    );
                }
            }
        }
        if matches!(case, 2 | 3) {
            assert!(matches!(
                input.items.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert!(matches!(
                input.equipment.completion,
                DraftListCompletion::Pending { .. }
            ));
        }
        summary.push(serde_json::json!({"original":case,"query_rows":22,"normalization":"pending",
            "previous_modifiers":old_modifiers,"current_modifiers":new_modifiers,
            "display_candidates":display_candidates,"template_preamble_candidates":eligible_candidates,
            "known_observations":known_observations,"whole_build_parity":"not_established"}));
    }
    assert_eq!(queries, 110);
    assert!(
        observations_admitted > 0,
        "observation publication admitted no original display rows"
    );
    fs::write(
        cwd.join("item-observations-admission-summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
}

#[cfg(feature = "pob")]
fn check_export(cwd: &Path, authored: &Path) {
    let output = cwd.join("item-observations-export");
    let run = |output: &Path| {
        Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
            .current_dir(cwd)
            .arg("export-owned-item-observations")
            .arg("--source-root")
            .arg(super::support::root().join("vendor/path-of-building-poe2"))
            .arg("--output")
            .arg(output)
            .output()
            .unwrap()
    };
    let report = success(run(&output));
    assert_eq!(report, json(authored.join("evidence.json")));
    for name in ["catalog.json", "evidence.json"] {
        assert_eq!(
            fs::read(output.join(name)).unwrap(),
            fs::read(authored.join(name)).unwrap()
        );
    }
    let before = bundle(&output);
    assert!(!run(&output).status.success());
    unchanged(&bundle(&output), &before);
}

pub fn check_item_observations(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("item-observations");
    let authored_bytes = bundle(&authored);
    let before = bundle(prior);
    let catalog = authored.join("catalog.json");
    let policy = authored.join("policy.json");
    let output = cwd.join("item-observations-successor");
    let report = success(publish(cwd, prior, &catalog, &policy, &output));
    assert_eq!(
        report["observations"],
        json(authored.join("compiler-receipt.json"))
    );
    assert_eq!(
        report["publication"]["before"],
        report["publication"]["after"]
    );
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    let published = bundle(&output);
    assert!(published.keys().eq(before.keys()));
    for (name, bytes) in &before {
        if !matches!(
            name.as_str(),
            "items.json"
                | "item-source.json"
                | "manifest.json"
                | "transition.json"
                | "catalog-append.json"
        ) {
            assert!(
                published.get(name) == Some(bytes),
                "observation publication changed {name}"
            );
        }
    }
    for name in ["items.json", "item-source.json"] {
        assert_eq!(
            json(output.join(name)),
            json(authored.join(name)),
            "compiled {name}"
        );
    }
    for (input, destination) in [
        (prior, output.clone()),
        (prior, prior.to_path_buf()),
        (output.as_path(), cwd.join("item-observations-reapplied")),
    ] {
        assert!(
            !publish(cwd, input, &catalog, &policy, &destination)
                .status
                .success()
        );
    }
    assert!(!cwd.join("item-observations-reapplied").exists());
    let replay = cwd.join("item-observations-replay");
    assert_eq!(
        success(publish(cwd, prior, &catalog, &policy, &replay)),
        report
    );
    unchanged(&bundle(&replay), &published);
    let stale_catalog = cwd.join("item-observations-stale-catalog.json");
    let mut stale_bytes = fs::read(&catalog).unwrap();
    stale_bytes.push(b' ');
    fs::write(&stale_catalog, stale_bytes).unwrap();
    let stale_policy = cwd.join("item-observations-stale-policy.json");
    let mut stale = json(&policy);
    stale["catalog_sha256"] = Value::String("0".repeat(64));
    fs::write(&stale_policy, serde_json::to_vec(&stale).unwrap()).unwrap();
    for (name, catalog, policy) in [
        ("catalog", &stale_catalog, &policy),
        ("policy", &catalog, &stale_policy),
    ] {
        let destination = cwd.join(format!("item-observations-rejected-{name}"));
        assert!(
            !publish(cwd, prior, catalog, policy, &destination)
                .status
                .success()
        );
        assert!(!destination.exists());
    }
    check_originals(cwd, prior, &output);
    #[cfg(feature = "pob")]
    check_export(cwd, &authored);
    unchanged(&bundle(&output), &published);
    unchanged(&bundle(prior), &before);
    unchanged(&bundle(&authored), &authored_bytes);
    output
}
