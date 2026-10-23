//! Inert source metadata improves admission without adding owned gameplay facts.
use super::support::{bundle, data, json, normalize, success};
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_draft::{
        DraftAllocationAccess, DraftField, DraftLimits, DraftListCompletion, decode_draft,
    },
};
use poe_optimizer_import::{
    owned_item_lines::{ConvertedItemEmission, ItemLineOutcome, OwnedItemLinePolicy},
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn publish(
    cwd: &Path,
    prior: &Path,
    authored: &Path,
    items: &Path,
    source: &Path,
    output: &Path,
) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("extend-owned-recipe")
        .arg(prior)
        .arg("--extension")
        .arg(authored.join("extension.json"))
        .arg("--items")
        .arg(items)
        .arg("--item-source")
        .arg(source)
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
fn equal_bundle(actual: &BTreeMap<String, Vec<u8>>, expected: &BTreeMap<String, Vec<u8>>) {
    assert!(
        actual.keys().eq(expected.keys()),
        "bundle filenames changed"
    );
    for (name, bytes) in expected {
        assert!(
            actual.get(name) == Some(bytes),
            "bundle file changed: {name}"
        );
    }
}
fn metadata_only(outcome: &ItemLineOutcome) {
    let ItemLineOutcome::Known { rule, emissions } = outcome else {
        panic!("metadata grammar must be known")
    };
    assert_eq!(rule.as_str(), "unique-id-metadata");
    assert_eq!(emissions.len(), 1);
    assert!(
        matches!(&emissions[0], ConvertedItemEmission::Metadata { role } if role.as_str() == "source-identity-metadata")
    );
}
fn check_policy_change(prior: &Path, output: &Path) {
    let before = json(prior.join("items.json"));
    let after = json(output.join("items.json"));
    let rules = after["rules"].as_array().unwrap();
    assert_eq!(rules.len(), before["rules"].as_array().unwrap().len() + 1);
    let metadata = rules
        .iter()
        .find(|r| r["id"] == "unique-id-metadata")
        .unwrap();
    assert_eq!(
        metadata["pattern"],
        serde_json::json!([
            {"kind":"literal","value":"Unique ID: "}, {"kind":"capture","value":"value"}
        ])
    );
    assert_eq!(
        metadata["captures"],
        serde_json::json!([{"id":"value","codec":{"kind":"opaque_text"}}])
    );
    assert_eq!(
        metadata["emissions"],
        serde_json::json!([{"kind":"metadata","value":{"role":"source-identity-metadata"}}])
    );
    let mut unchanged = after.clone();
    unchanged["version"] = before["version"].clone();
    unchanged["rules"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["id"] != "unique-id-metadata");
    assert!(
        unchanged == before,
        "existing item policy semantics changed"
    );
    let before = json(prior.join("item-source.json"));
    let after = json(output.join("item-source.json"));
    assert_eq!(after["schema_version"], 5);
    let dialect = &after["dialect"]["pob_exported_single_text_preamble_v1"];
    assert_eq!(
        dialect["metadata_rules"],
        serde_json::json!(["unique-id-metadata"])
    );
    assert!(
        dialect["flag_bindings"]
            == before["dialect"]["pob_exported_single_text_flags_v1"]["flag_bindings"],
        "flag bindings changed"
    );
    let roles = after["rule_layouts"].as_array().unwrap();
    assert_eq!(
        roles.len(),
        before["rule_layouts"].as_array().unwrap().len() + 1
    );
    assert_eq!(
        roles
            .iter()
            .find(|r| r["rule"] == "unique-id-metadata")
            .unwrap()["role"],
        "header"
    );
    let mut unchanged = after.clone();
    for field in ["schema_version", "version", "dialect", "item_lines"] {
        unchanged[field] = before[field].clone();
    }
    unchanged["rule_layouts"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["rule"] != "unique-id-metadata");
    assert!(
        unchanged == before,
        "metadata publication changed unrelated source policy"
    );
}
type Admission = (u64, u64, usize);
fn admitted(sidecar: &Value) -> BTreeSet<Admission> {
    sidecar["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|item| {
            let source = item["source"]["ordinal"].as_u64().unwrap();
            item["lines"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(move |line| {
                    let index = line["index"].as_u64().unwrap();
                    (0..line["modifiers"].as_array().unwrap().len())
                        .map(move |modifier| (source, index, modifier))
                })
        })
        .collect()
}
fn blockers(sidecar: &Value) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for item in sidecar["item_texts"].as_array().unwrap() {
        for line in item["attribution"]["lines"].as_array().unwrap() {
            for blocker in line["blockers"].as_array().unwrap() {
                *counts
                    .entry(blocker.as_str().unwrap().to_owned())
                    .or_default() += 1;
            }
        }
    }
    counts
}
fn check_originals(cwd: &Path, prior: &Path, output: &Path) {
    let checked = assemble_owned_recipe(recipe(output), Default::default()).unwrap();
    let lines = OwnedItemLinePolicy::new(
        serde_json::from_value(json(output.join("items.json"))).unwrap(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let mut summary = vec![];
    let (mut queries, mut unique_ids, mut old_count) = (0, 0, 0);
    for case in 1..=5 {
        let name = format!("queries-original-{case:02}.json");
        assert!(
            fs::read(prior.join(&name)).unwrap() == fs::read(output.join(&name)).unwrap(),
            "query bytes changed: {name}"
        );
        queries += json(output.join(name)).as_array().unwrap().len();
        let path = cwd.join(format!("item-metadata-original-{case}"));
        assert_eq!(
            success(normalize(cwd, output, case, &path, true))["normalization_status"],
            "pending"
        );
        let draft = decode_draft(
            &fs::read(path.join("draft.json")).unwrap(),
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
        assert!(!input.items.members.is_empty() && !input.allocations.members.is_empty());
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
            assert!(matches!(
                item.modifier_orders.contributions,
                DraftField::Pending(_)
            ));
            assert!(matches!(
                item.modifier_orders.numeric_transforms,
                DraftField::Pending(_)
            ));
            assert!(item.to_resolved().is_none());
        }
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
                    panic!("rune collection falsely complete")
                };
                assert_eq!(code.as_str(), expected);
            }
        }
        let sidecar = json(path.join("sidecar.json"));
        let before = json(cwd.join(format!("item-layout-original-{case}/sidecar.json")));
        for field in ["source_sha256", "source_bytes", "source_schema", "revision"] {
            assert_eq!(
                sidecar[field], before[field],
                "original-{case}: source provenance {field}"
            );
        }
        let old = admitted(&before);
        let new = admitted(&sidecar);
        assert!(
            old.is_subset(&new),
            "original-{case}: metadata publication lost an occurrence"
        );
        old_count += old.len();
        let (mut count, mut known) = (0, 0);
        for item in sidecar["item_texts"].as_array().unwrap() {
            let old_item = before["item_texts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|i| i["source"] == item["source"])
                .unwrap();
            for row in item["attribution"]["lines"].as_array().unwrap() {
                let old_row = old_item["attribution"]["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["index"] == row["index"])
                    .unwrap();
                assert!(
                    row["raw"] == old_row["raw"],
                    "original-{case}: raw line changed at {}",
                    row["index"]
                );
                let raw = row["raw"].as_str().unwrap();
                if !raw.starts_with("Unique ID: ") {
                    continue;
                }
                count += 1;
                metadata_only(
                    &lines
                        .convert_line(row["index"].as_u64().unwrap() as usize, raw, None)
                        .unwrap()
                        .outcome,
                );
                assert_eq!(row["rule"], "unique-id-metadata");
                assert!(row["member"].is_null());
                let writes = item["attribution"]["writes"].as_array().unwrap();
                let unresolved_overlay = writes.iter().any(|write| {
                    write["origin"]["kind"] == "xml" && write["target"]["status"] == "pending"
                });
                // An unresolved XML target conservatively invalidates every line's
                // range attribution, including inert metadata. It does not make
                // metadata a modifier member or supply a usable numeric value.
                assert_eq!(
                    row["range"]["status"],
                    if unresolved_overlay {
                        "pending"
                    } else {
                        "absent"
                    },
                    "original-{case}: metadata range lacks its source-overlay reason"
                );
                if unresolved_overlay {
                    assert_ne!(item["attribution"]["layout"]["status"], "proven");
                }
                assert!(
                    !writes.iter().any(|write| {
                        write["target"]["status"] == "line"
                            && write["target"]["value"] == row["index"]
                    }),
                    "metadata became a resolved range-write target"
                );
                assert!(row["property_tokens"].as_array().unwrap().is_empty());
                let normalized = item["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["index"] == row["index"])
                    .unwrap();
                assert!(
                    normalized["modifiers"].as_array().unwrap().is_empty(),
                    "metadata emitted a modifier"
                );
                if normalized["outcome"]["kind"] == "known" {
                    known += 1;
                    assert_eq!(normalized["outcome"]["value"]["rule"], "unique-id-metadata");
                    let emissions = normalized["outcome"]["value"]["emissions"]
                        .as_array()
                        .unwrap();
                    assert_eq!(emissions.len(), 1);
                    assert_eq!(emissions[0]["kind"], "metadata");
                }
            }
        }
        unique_ids += count;
        summary.push(serde_json::json!({
            "original":case,"normalization":"pending","query_rows":22,
            "unique_id_lines":count,"known_metadata_lines":known,
            "admitted_before":old.len(),"admitted_after":new.len(),
            "new_occurrences":new.difference(&old).collect::<Vec<_>>(),
            "blockers_before":blockers(&before),"blockers_after":blockers(&sidecar),
            "whole_build_parity":"not_established"
        }));
    }
    assert_eq!(queries, 110);
    assert_eq!(unique_ids, 97);
    assert_eq!(old_count, 7);
    fs::write(
        cwd.join("item-metadata-admission-summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
}

pub fn check_item_metadata(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("item-metadata-inputs");
    let authored_bytes = bundle(&authored);
    let prior_bytes = bundle(prior);
    let items = authored.join("items.json");
    let source = authored.join("item-source.json");
    let output = cwd.join("item-metadata-successor");
    let report = success(publish(cwd, prior, &authored, &items, &source, &output));
    for field in [
        "allocated_entries",
        "refined_subjects",
        "appended_tables",
        "appended_programs",
        "appended_receivers",
    ] {
        assert_eq!(report["extension"][field], 0, "{field}");
    }
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["before"],
        report["publication"]["after"]
    );
    assert_eq!(
        report["publication"]["item_policy_mode"],
        "explicit_successor_bound_inputs"
    );
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    assert_eq!(
        json(output.join("registry.json"))["entries"]
            .as_array()
            .unwrap()
            .len(),
        10537
    );
    let published = bundle(&output);
    assert!(
        published.keys().eq(prior_bytes.keys()),
        "publication file set changed"
    );
    for (name, bytes) in &prior_bytes {
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
                "metadata publication changed {name}"
            );
        }
    }
    assert!(json(output.join("items.json")) == json(&items));
    assert!(json(output.join("item-source.json")) == json(&source));
    check_policy_change(prior, &output);
    assert!(
        !publish(cwd, prior, &authored, &items, &source, &output)
            .status
            .success()
    );
    equal_bundle(&bundle(&output), &published);
    let replay = cwd.join("item-metadata-replay");
    assert_eq!(
        success(publish(cwd, prior, &authored, &items, &source, &replay)),
        report
    );
    equal_bundle(&bundle(&replay), &published);
    let mut stale = json(&source);
    stale["item_lines"] =
        serde_json::to_value(digest_owned("stale-metadata-items", &false, 1024).unwrap()).unwrap();
    let stale_path = cwd.join("item-metadata-stale-source.json");
    fs::write(&stale_path, serde_json::to_vec(&stale).unwrap()).unwrap();
    let rejected = cwd.join("item-metadata-rejected");
    assert!(
        !publish(cwd, prior, &authored, &items, &stale_path, &rejected)
            .status
            .success()
    );
    assert!(!rejected.exists());
    check_originals(cwd, prior, &output);
    equal_bundle(&bundle(&output), &published);
    equal_bundle(&bundle(prior), &prior_bytes);
    equal_bundle(&bundle(&authored), &authored_bytes);
    output
}
