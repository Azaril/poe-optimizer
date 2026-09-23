//! Source-layout publication preserves mechanics and never closes original builds.
use super::support::{bundle, data, json, normalize, success};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::ParameterValue,
    owned_content::digest_owned,
    owned_draft::{
        DraftAllocationAccess, DraftField, DraftLimits, DraftListCompletion, decode_draft,
    },
};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_item_layouts::ItemLayoutPolicy,
    owned_item_lines::OwnedItemLinePolicy,
    owned_item_source::{
        ItemLayoutStatus, ItemRangeAttribution, ItemRangeDecision, ItemSourceDefaultScope,
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

fn publish(cwd: &Path, prior: &Path, catalog: &Path, policy: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("compile-owned-item-layouts")
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
    base: &str,
    headers: &str,
    body: &str,
    overlay: &str,
    source: &ItemSourceLayoutPolicy,
    lines: &OwnedItemLinePolicy,
) -> ItemRangeAttribution {
    let xml = format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nLayout Fixture\n{base}\n{headers}Implicits: 0\n{body}{overlay}</Item></Items></PathOfBuilding2>"
    );
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([94; 16]),
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
fn check_source_transition(prior: &Path, output: &Path, policy: &Value, catalog: &Value) {
    let before = json(prior.join("item-source.json"));
    let after = json(output.join("item-source.json"));
    let mut unchanged = after.clone();
    unchanged["version"] = before["version"].clone();
    unchanged["template_layouts"] = before["template_layouts"].clone();
    assert!(
        unchanged == before,
        "only source prefix evidence and version may change"
    );
    assert_eq!(after["version"], policy["version"]);
    let old_layouts = before["template_layouts"].as_array().unwrap();
    let new_layouts = after["template_layouts"].as_array().unwrap();
    assert_eq!(old_layouts.len(), new_layouts.len());
    let prefixes: BTreeMap<_, _> = catalog["bases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| {
            (
                b["source_base"].as_str().unwrap(),
                b["prefix"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(prefixes.len(), 1756);
    let bindings: BTreeMap<_, _> = policy["templates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| {
            (
                serde_json::to_string(&b["template"]).unwrap(),
                b["source_base"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(bindings.len(), 1756);
    let mut refined = 0;
    for (old, new) in old_layouts.iter().zip(new_layouts) {
        assert_eq!(old["template"], new["template"]);
        let id = serde_json::to_string(&old["template"]).unwrap();
        if let Some(name) = bindings.get(&id) {
            match prefixes[name] {
                "absent" => {
                    assert_eq!(
                        new["load_index_prefix"], "no_generated_buff_members",
                        "{name}"
                    );
                    refined += usize::from(old["load_index_prefix"] == "unresolved");
                }
                "present" | "unsupported" => {
                    assert!(new == old, "non-absence evidence changed {name}")
                }
                other => panic!("unknown catalog evidence {other}"),
            }
        } else {
            assert!(new == old, "unbound source template changed: {id}");
        }
    }
    assert_eq!(refined, 1742);
    assert_eq!(
        catalog["bases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["source_base"] == "Cleansing Charm")
            .unwrap()["prefix"],
        "present",
        "empty source buff text is still a generated prefix member"
    );
}
fn check_real_source_cases(prior: &Path, output: &Path) {
    let checked = assemble_owned_recipe(recipe(output), Default::default()).unwrap();
    let lines = OwnedItemLinePolicy::new(
        serde_json::from_value(json(output.join("items.json"))).unwrap(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let old = ItemSourceLayoutPolicy::new(
        serde_json::from_value(json(prior.join("item-source.json"))).unwrap(),
        &lines,
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let new = ItemSourceLayoutPolicy::new(
        serde_json::from_value(json(output.join("item-source.json"))).unwrap(),
        &lines,
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let before = attribute("Grand Spear", "", "", "", &old, &lines);
    assert!(
        matches!(before.report().layout, ItemLayoutStatus::Pending(ref p) if p.contains(&ItemSourceProblem::UnknownTemplatePrefix))
    );
    assert!(
        before
            .convert(&lines)
            .unwrap()
            .defaults
            .parameters
            .is_empty()
    );
    let after = attribute("Grand Spear", "", "", "", &new, &lines);
    assert!(matches!(after.report().layout, ItemLayoutStatus::Proven));
    assert!(matches!(
        after.report().default_scope,
        ItemSourceDefaultScope::Proven { .. }
    ));
    let converted = after.convert(&lines).unwrap();
    assert_eq!(converted.defaults.parameters.len(), 2);
    assert!(!converted.defaults.item_level_absent && !converted.defaults.quality_absent);
    assert!(
        converted
            .defaults
            .parameters
            .iter()
            .any(|p| matches!(&p.value, ParameterValue::Quantity(q) if q.value() == 20.0))
    );
    assert!(converted.defaults.parameters.iter().any(|p| matches!(&p.value, ParameterValue::Option(o) if o.key().as_str() == "def.00000000000009eb")));
    let body = "{tags:physical,damage}{range:0.5}(10-14)% increased Physical Damage";
    let headers =
        "Crafted: true\nItem Level: 80\nQuality: 0\nCatalyst: Tul's\nCatalystQuality: 0\n";
    let overlay = "<ModRange id=\"1\" range=\"0.25\"/>";
    let before = attribute("Grand Spear", headers, body, overlay, &old, &lines);
    assert!(before.convert(&lines).unwrap().modifiers.is_empty());
    let after = attribute("Grand Spear", headers, body, overlay, &new, &lines);
    assert!(
        matches!(after.report().layout, ItemLayoutStatus::Proven),
        "minimal range fixture: {:?}",
        after.report().layout
    );
    let line = after
        .report()
        .lines
        .iter()
        .find(|l| l.raw.contains("increased Physical Damage"))
        .unwrap();
    assert!(matches!(line.range, ItemRangeDecision::Resolved { fraction, .. } if fraction == 0.25));
    let converted = after.convert(&lines).unwrap();
    assert_eq!(converted.modifiers.len(), 1);
    assert_eq!(converted.parameters.len(), 2);
    assert!(converted.defaults.parameters.is_empty());
    assert!(
        converted.modifiers[0]
            .rolls
            .iter()
            .any(|r| matches!(&r.value, ParameterValue::Quantity(q) if q.value() == 11.0))
    );
    assert!(matches!(
        converted.modifiers[0].rolls_closure,
        poe_optimizer_core::owned_schema::SchemaClosure::Partial { .. }
    ));
    let charm = attribute("Cleansing Charm", "", "", "", &new, &lines);
    assert!(
        matches!(charm.report().layout, ItemLayoutStatus::Pending(ref p) if p.contains(&ItemSourceProblem::UnknownTemplatePrefix))
    );
    assert!(
        charm
            .convert(&lines)
            .unwrap()
            .defaults
            .parameters
            .is_empty()
    );
    // A base-prefix proof does not make unrelated source controls disappear.
    for (headers, body) in [
        ("UnknownControl: enabled\n", ""),
        ("", "{variant:1}(10-14)% increased Physical Damage"),
        ("", "{rune}17% increased Physical Damage"),
    ] {
        let rejected = attribute("Grand Spear", headers, body, "", &new, &lines);
        assert!(!matches!(
            rejected.report().layout,
            ItemLayoutStatus::Proven
        ));
        assert!(
            rejected
                .convert(&lines)
                .unwrap()
                .defaults
                .parameters
                .is_empty()
        );
    }
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
fn layouts(sidecar: &Value) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for item in sidecar["item_texts"].as_array().unwrap() {
        *counts
            .entry(
                item["attribution"]["layout"]["status"]
                    .as_str()
                    .unwrap_or("unavailable")
                    .to_owned(),
            )
            .or_default() += 1;
    }
    counts
}
fn check_originals(cwd: &Path, prior: &Path, output: &Path) {
    let mut queries = 0;
    let mut summary = vec![];
    for case in 1..=5 {
        let query_file = format!("queries-original-{case:02}.json");
        assert!(
            fs::read(prior.join(&query_file)).unwrap()
                == fs::read(output.join(&query_file)).unwrap(),
            "query file changed: {query_file}"
        );
        queries += json(output.join(query_file)).as_array().unwrap().len();
        let path = cwd.join(format!("item-layout-original-{case}"));
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
                    panic!("rune membership falsely closed")
                };
                assert_eq!(code.as_str(), expected);
            }
        }
        let sidecar = json(path.join("sidecar.json"));
        // The intervening local/elemental stages alter mechanics only. This
        // earlier chain checkpoint is the same source-admission predecessor.
        let before = json(cwd.join(format!("weapon-catalyst-original-{case}/sidecar.json")));
        for field in ["source_sha256", "source_bytes", "source_schema", "revision"] {
            assert_eq!(
                sidecar[field], before[field],
                "original-{case} source provenance {field}"
            );
        }
        let old = admitted(&before);
        let new = admitted(&sidecar);
        assert!(
            old.is_subset(&new),
            "original-{case}: source-prefix refinement lost an admitted occurrence"
        );
        for item in sidecar["item_texts"].as_array().unwrap() {
            let prior_item = before["item_texts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["source"] == item["source"])
                .unwrap();
            for line in item["attribution"]["lines"].as_array().unwrap() {
                let prior_line = prior_item["attribution"]["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|l| l["index"] == line["index"])
                    .unwrap();
                assert!(
                    line["raw"] == prior_line["raw"],
                    "original-{case}: raw line changed at {}",
                    line["index"]
                );
            }
        }
        summary.push(serde_json::json!({
            "original":case, "normalization":"pending", "query_rows":22,
            "admitted_before":old.len(), "admitted_after":new.len(),
            "new_occurrences":new.difference(&old).collect::<Vec<_>>(),
            "layouts_before":layouts(&before), "layouts_after":layouts(&sidecar),
            "whole_build_parity":"not_established"
        }));
    }
    assert_eq!(queries, 110);
    fs::write(
        cwd.join("item-layouts-admission-summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
}

pub fn check_item_layouts(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("item-layouts");
    let authored_bytes = bundle(&authored);
    let prior_bytes = bundle(prior);
    let catalog_path = authored.join("catalog.json");
    let policy_path = authored.join("policy.json");
    let output = cwd.join("item-layout-successor");
    let report = success(publish(cwd, prior, &catalog_path, &policy_path, &output));
    for (field, expected) in [
        ("bases", 1756),
        ("absent", 1743),
        ("present", 13),
        ("unsupported", 0),
        ("refined_prefixes", 1742),
    ] {
        assert_eq!(report["item_layouts"][field], expected, "{field}");
    }
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    assert_eq!(
        report["publication"]["item_policy_mode"],
        "explicit_successor_bound_inputs"
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
            "item-source.json" | "manifest.json" | "transition.json" | "catalog-append.json"
        ) {
            assert!(
                published.get(name) == Some(bytes),
                "source-only conversion changed {name}"
            );
        }
    }
    check_source_transition(prior, &output, &json(&policy_path), &json(&catalog_path));
    check_real_source_cases(prior, &output);
    assert!(
        !publish(cwd, prior, &catalog_path, &policy_path, &output)
            .status
            .success()
    );
    equal_bundle(&bundle(&output), &published);
    let replay = cwd.join("item-layout-replay");
    assert_eq!(
        success(publish(cwd, prior, &catalog_path, &policy_path, &replay)),
        report
    );
    equal_bundle(&bundle(&replay), &published);
    let policy: ItemLayoutPolicy = serde_json::from_value(json(&policy_path)).unwrap();
    for (label, change) in [("catalog", 0), ("source-binding", 1)] {
        let mut stale = policy.clone();
        if change == 0 {
            stale.catalog_sha256 = "0".repeat(64);
        } else {
            stale.item_source = digest_owned("stale-layout-input", &false, 1024).unwrap();
        }
        let path = cwd.join(format!("item-layout-stale-{label}.json"));
        fs::write(&path, serde_json::to_vec(&stale).unwrap()).unwrap();
        let rejected = cwd.join(format!("item-layout-rejected-{label}"));
        assert!(
            !publish(cwd, prior, &catalog_path, &path, &rejected)
                .status
                .success(),
            "stale {label} accepted"
        );
        assert!(!rejected.exists(), "failed publication left a destination");
    }
    check_originals(cwd, prior, &output);
    equal_bundle(&bundle(&output), &published);
    equal_bundle(&bundle(prior), &prior_bytes);
    equal_bundle(&bundle(&authored), &authored_bytes);
    output
}
