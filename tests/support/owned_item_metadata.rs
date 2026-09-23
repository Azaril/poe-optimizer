//! Inert source metadata improves admission without adding owned gameplay facts.
use super::support::{bundle, data, json, normalize, success};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::ParameterValue,
    owned_content::digest_owned,
    owned_draft::{
        DraftAllocationAccess, DraftField, DraftLimits, DraftListCompletion, decode_draft,
    },
    owned_schema::SchemaClosure,
};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_item_lines::{ConvertedItemEmission, ItemLineOutcome, OwnedItemLinePolicy},
    owned_item_source::{
        ItemLayoutStatus, ItemRangeAttribution, ItemRangeDecision, ItemSourceDialect,
        ItemSourceLayoutPolicy, ItemSourceProblem,
    },
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_source::SourceProjectEvidence,
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
fn attribute(
    text: &str,
    source: &ItemSourceLayoutPolicy,
    lines: &OwnedItemLinePolicy,
) -> ItemRangeAttribution {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let xml = format!(
        "<PathOfBuilding2><Items><Item id=\"7\">{escaped}</Item></Items></PathOfBuilding2>"
    );
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([95; 16]),
        Default::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    let row = evidence
        .rows()
        .iter()
        .find(|r| r.occurrence().name() == "Item")
        .unwrap();
    source
        .attribute(&evidence, row.occurrence().id(), lines)
        .unwrap()
}
fn item(headers: &str, body: &str) -> String {
    format!("Rarity: RARE\nMetadata Fixture\nGrand Spear\n{headers}Implicits: 0\n{body}")
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
fn check_preamble_cases(output: &Path) {
    let checked = assemble_owned_recipe(recipe(output), Default::default()).unwrap();
    let lines = OwnedItemLinePolicy::new(
        serde_json::from_value(json(output.join("items.json"))).unwrap(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let source = ItemSourceLayoutPolicy::new(
        serde_json::from_value(json(output.join("item-source.json"))).unwrap(),
        &lines,
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let body = "{tags:physical,damage}{range:0.5}(10-14)% increased Physical Damage";
    let baseline = attribute(&item("", body), &source, &lines);
    let baseline_values = baseline.convert(&lines).unwrap();
    for headers in [
        "Unique ID: abc-123\n",
        "Unique ID: alpha\nUnique ID: beta\n",
        "Unique ID: {range:0.1}{tags:physical}{rune}\n",
    ] {
        let attributed = attribute(&item(headers, body), &source, &lines);
        assert!(
            matches!(attributed.report().layout, ItemLayoutStatus::Proven),
            "metadata preamble: {:?}",
            attributed.report().layout
        );
        assert_eq!(
            attributed.report().writes.len(),
            baseline.report().writes.len()
        );
        let converted = attributed.convert(&lines).unwrap();
        assert!(
            converted.parameters.is_empty(),
            "metadata cannot emit item parameters"
        );
        assert_eq!(
            converted.defaults.parameters,
            baseline_values.defaults.parameters
        );
        assert_eq!(converted.modifiers.len(), 1);
        assert_eq!(
            converted.modifiers[0].rolls,
            baseline_values.modifiers[0].rolls
        );
        assert!(matches!(
            converted.modifiers[0].rolls_closure,
            SchemaClosure::Partial { .. }
        ));
        assert!(
            converted.modifiers[0]
                .rolls
                .iter()
                .any(|r| matches!(&r.value, ParameterValue::Quantity(q) if q.value() == 12.0))
        );
        for row in attributed
            .report()
            .lines
            .iter()
            .filter(|l| l.raw.starts_with("Unique ID: "))
        {
            assert!(row.member.is_none() && row.blockers.is_empty());
            assert_eq!(row.semantic_text, row.raw);
            assert!(
                row.properties.is_empty()
                    && row.property_tokens.is_empty()
                    && row.flag_tokens.is_empty()
            );
            assert!(matches!(row.range, ItemRangeDecision::Absent));
            metadata_only(
                &converted
                    .lines
                    .iter()
                    .find(|l| l.index == row.index)
                    .unwrap()
                    .outcome,
            );
            assert!(!converted.modifiers.iter().any(|m| m.line == row.index));
            assert!(!converted.parameters.iter().any(|p| p.line == row.index));
        }
    }
    for raw in [
        format!(
            "Rarity: RARE\nMetadata Fixture\nUnique ID: too-early\nGrand Spear\nImplicits: 0\n{body}"
        ),
        item("", "17% increased Physical Damage\nUnique ID: too-late"),
        item("Unique ID: \n", body),
        item("Unknown Header: inert-looking\n", body),
    ] {
        let attributed = attribute(&raw, &source, &lines);
        assert!(!matches!(
            attributed.report().layout,
            ItemLayoutStatus::Proven
        ));
        assert!(
            attributed
                .convert(&lines)
                .unwrap()
                .defaults
                .parameters
                .is_empty()
        );
    }
    for value in [
        "prefix {variant:1} suffix",
        "{version:1}",
        "{group:1}",
        "prefix Foil Unique suffix",
        "[text]",
        "<text>",
    ] {
        let attributed = attribute(
            &item(&format!("Unique ID: {value}\n"), body),
            &source,
            &lines,
        );
        assert!(
            matches!(attributed.report().layout, ItemLayoutStatus::Unsupported(ref p) if p.contains(&ItemSourceProblem::UnsupportedSourceControl)),
            "control must be unsupported: {value}"
        );
        assert!(attributed.report().lines.iter().any(|l| {
            l.raw.starts_with("Unique ID: ")
                && l.blockers
                    .contains(&ItemSourceProblem::UnsupportedSourceControl)
        }));
        let converted = attributed.convert(&lines).unwrap();
        assert!(
            converted.modifiers.is_empty()
                && converted.parameters.is_empty()
                && converted.defaults.parameters.is_empty()
        );
    }
    // Even the same metadata-only recipe needs an explicitly versioned source
    // admission declaration; the old v4 dialect does not gain this behavior.
    let mut old_input = source.input().clone();
    let ItemSourceDialect::PobExportedSingleTextPreambleV1 { flag_bindings, .. } =
        old_input.dialect
    else {
        panic!("new preamble dialect")
    };
    old_input.schema_version = 4;
    old_input.dialect = ItemSourceDialect::PobExportedSingleTextFlagsV1 { flag_bindings };
    let old = ItemSourceLayoutPolicy::new(old_input, &lines, checked.schema(), Default::default())
        .unwrap();
    let attributed = attribute(&item("Unique ID: abc-123\n", body), &old, &lines);
    assert!(!matches!(
        attributed.report().layout,
        ItemLayoutStatus::Proven
    ));
    assert!(
        attributed
            .report()
            .lines
            .iter()
            .any(|l| l.raw.starts_with("Unique ID: ")
                && l.blockers.contains(&ItemSourceProblem::UnknownHeader))
    );
    assert!(attributed.convert(&lines).unwrap().modifiers.is_empty());
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
            assert!(matches!(item.modifier_order, DraftField::Pending(_)));
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
    check_preamble_cases(&output);
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
