//! Successor source proofs narrow parser authority while preserving admitted raw facts.
use super::support::{bundle, data, json, normalize, success};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_content::digest_owned,
    owned_draft::{
        DraftAllocationAccess, DraftField, DraftLimits, DraftListCompletion, decode_draft,
    },
};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_item_lines::OwnedItemLinePolicy,
    owned_item_source::{ItemRangeAttribution, ItemSourceLayoutPolicy, ItemSourceProblem},
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

fn publish(cwd: &Path, prior: &Path, authored: &Path, source: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("extend-owned-recipe")
        .arg(prior)
        .arg("--extension")
        .arg(authored.join("extension.json"))
        .arg("--items")
        .arg(prior.join("items.json"))
        .arg("--item-source")
        .arg(source)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
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
fn recipe(path: &Path) -> OwnedRecipeInput {
    OwnedRecipeInput {
        schema_version: 1,
        registry: serde_json::from_value(json(path.join("registry.json"))).unwrap(),
        schema: serde_json::from_value(json(path.join("schema.json"))).unwrap(),
        rules: serde_json::from_value(json(path.join("rules.json"))).unwrap(),
        routing: serde_json::from_value(json(path.join("routing.json"))).unwrap(),
    }
}
fn check_policy(prior: &Path, output: &Path) {
    let before = json(prior.join("item-source.json"));
    let after = json(output.join("item-source.json"));
    assert_eq!(before["schema_version"], 6);
    assert_eq!(after["schema_version"], 6);
    let previous_roles = before["rule_layouts"].as_array().unwrap();
    let next_roles = after["rule_layouts"].as_array().unwrap();
    assert_eq!(previous_roles.len(), next_roles.len());
    let migrated: BTreeSet<_> = previous_roles
        .iter()
        .filter(|row| row["role"] == "single_modifier")
        .map(|row| row["rule"].as_str().unwrap())
        .collect();
    assert_eq!(migrated.len(), 29);
    for (old, next) in previous_roles.iter().zip(next_roles) {
        assert_eq!(old["rule"], next["rule"]);
        if migrated.contains(old["rule"].as_str().unwrap()) {
            assert_eq!(next["role"], "unresolved", "{}", old["rule"]);
        } else {
            assert!(
                next == old,
                "unrelated source role changed: {}",
                old["rule"]
            );
        }
    }
    assert!(
        next_roles
            .iter()
            .all(|row| row["role"] != "single_modifier")
    );
    let previous = &before["dialect"]["pob_exported_single_text_conditions_v1"];
    let next = &after["dialect"]["pob_exported_single_text_conditions_v1"];
    for field in ["flag_bindings", "metadata_rules"] {
        assert!(
            previous[field] == next[field],
            "unrelated dialect field: {field}"
        );
    }
    let conditions = next["single_modifier_conditions"].as_array().unwrap();
    let guarded: BTreeSet<_> = conditions
        .iter()
        .map(|row| row["rule"].as_str().unwrap())
        .collect();
    assert_eq!(guarded.len(), 28);
    assert_eq!(guarded.len(), conditions.len(), "duplicate guarded rule");
    assert!(guarded.contains("fixed-cold"));
    for id in migrated {
        assert_eq!(
            guarded.contains(id),
            !matches!(id, "ranged-bare-cold" | "ranged-bare-elemental"),
            "unexpected reviewed guard membership: {id}"
        );
    }
    for guard in conditions {
        assert!(
            guard["all"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["kind"] == "no_generated_buff_members"),
            "missing generated-prefix exclusion: {}",
            guard["rule"]
        );
    }
    let fixed = conditions
        .iter()
        .find(|r| r["rule"] == "fixed-cold")
        .unwrap();
    assert!(
        previous["single_modifier_conditions"]
            .as_array()
            .unwrap()
            .contains(fixed),
        "previous fixed-cold proof was reinterpreted"
    );
    let mut restored = after;
    for field in ["schema_version", "version", "dialect", "rule_layouts"] {
        restored[field] = before[field].clone();
    }
    assert!(
        restored == before,
        "role migration changed unrelated source contracts"
    );
}
fn attribute(
    base: &str,
    headers: &str,
    body: &str,
    source: &ItemSourceLayoutPolicy,
    lines: &OwnedItemLinePolicy,
) -> ItemRangeAttribution {
    let text = format!("Rarity: RARE\nSource Role Fixture\n{base}\n{headers}Implicits: 0\n{body}");
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let xml = format!(
        "<PathOfBuilding2><Items><Item id=\"7\">{escaped}</Item></Items></PathOfBuilding2>"
    );
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([98; 16]),
        Default::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    let item = evidence
        .rows()
        .iter()
        .find(|r| r.occurrence().name() == "Item")
        .unwrap();
    source
        .attribute(&evidence, item.occurrence().id(), lines)
        .unwrap()
}
fn check_source_cases(prior: &Path, output: &Path) {
    let checked = assemble_owned_recipe(recipe(output), Default::default()).unwrap();
    let lines = OwnedItemLinePolicy::new(
        serde_json::from_value(json(output.join("items.json"))).unwrap(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let load = |path: &Path| {
        ItemSourceLayoutPolicy::new(
            serde_json::from_value(json(path.join("item-source.json"))).unwrap(),
            &lines,
            checked.schema(),
            Default::default(),
        )
        .unwrap()
    };
    let old = load(prior);
    let source = load(output);
    assert_ne!(old.identity(), source.identity());
    for policy in [&old, &source] {
        assert_eq!(
            *policy.identity(),
            digest_owned(
                "owned-item-source-policy-v6",
                policy.input(),
                4 * 1024 * 1024
            )
            .unwrap()
        );
    }
    // Independent source membership does not close numeric/template eligibility.
    for (base, body) in [
        ("Sapphire Ring", "+10 to maximum Life"),
        ("Sapphire Ring", "+11% to all Elemental Resistances"),
        ("Grand Spear", "+1.5% to Critical Hit Chance"),
        ("Grand Spear", "28% increased Physical Damage"),
        ("Grand Spear", "22% reduced Physical Damage"),
        ("Grand Spear", "49% increased Attack Speed"),
        ("Grand Spear", "20% reduced Attack Speed"),
        ("Grand Spear", "12% increased Critical Hit Chance"),
        ("Grand Spear", "12% reduced Critical Hit Chance"),
        ("Grand Spear", "Adds 11 to 19 Physical Damage"),
        ("Grand Spear", "Adds 49 to 74 Cold Damage"),
        ("Grand Spear", "Adds 6 to 179 Lightning Damage"),
        ("Grand Spear", "Adds 5 to 13 Fire Damage"),
        ("Grand Spear", "Adds 5 to 13 Chaos Damage"),
        (
            "Grand Spear",
            "{range:0.5}(20-30)% increased Physical Damage",
        ),
        (
            "Grand Spear",
            "{range:0.5}Adds (1-3) to (10-20) Cold Damage",
        ),
        ("Grand Spear", "{fractured}167% increased Physical Damage"),
        ("Grand Spear", "{desecrated}167% increased Physical Damage"),
        ("Sapphire Ring", "{range:0.5}+(20-30)% to Cold Resistance"),
        (
            "Sapphire Ring",
            "{range:0.5}+(20-30)% to all Elemental Resistances",
        ),
        (
            "Sapphire Ring",
            "{tags:cold_resistance,elemental_resistance,elemental,cold,resistance}{range:0.5}+(20-30)% to Cold Resistance",
        ),
    ] {
        let plan = attribute(base, "", body, &source, &lines);
        let row = plan.report().lines.last().unwrap();
        assert!(
            row.member.is_some(),
            "source member missing: {base}/{body}; {:?}",
            row.blockers
        );
        assert!(
            row.blockers.is_empty(),
            "reviewed source member blocked: {body}"
        );
    }
    // Empty actual modTags remain unscaled under a negative catalyst quality.
    let headers = "Catalyst: Tul's\nCatalystQuality: -200\n";
    let plan = attribute(
        "Grand Spear",
        headers,
        "Adds 5 to 13 Cold Damage",
        &source,
        &lines,
    );
    assert!(plan.report().lines.last().unwrap().member.is_some());
    for text in [
        "10 to maximum Life",
        "10% to all Elemental Resistances",
        "1.5% to Critical Hit Chance",
        "-1% to Critical Hit Chance",
        "(20-30)% to Cold Resistance",
        "(20-30)% to all Elemental Resistances",
        "+(0-0)% to Cold Resistance",
        "+(0-10)% to all Elemental Resistances",
        "+(20-1000001)% to Cold Resistance",
        "+1000001 to maximum Life",
        "1000001% increased Physical Damage",
        "Adds 1 to 1000001 Cold Damage",
        "-1% increased Physical Damage",
        "{tags:cold}Adds 5 to 13 Cold Damage",
        "{tags:cold}{range:0.5}+(20-30)% to Cold Resistance",
        "{corruptedRange:1}Adds 5 to 13 Cold Damage",
        "{corruptedRange:1.5}Adds 5 to 13 Cold Damage",
        "{variant:1}Adds 5 to 13 Cold Damage",
        "{crafted}Adds 5 to 13 Cold Damage",
        "{rune}Adds 5 to 13 Cold Damage",
        "{tags:unknown}Adds 5 to 13 Cold Damage",
    ] {
        let body = format!("{text}\n+10% to Cold Resistance");
        let plan = attribute("Grand Spear", headers, &body, &source, &lines);
        let first = &plan.report().lines[plan.report().lines.len() - 2];
        assert!(
            first.member.is_none(),
            "unproved source member admitted: {text}"
        );
        assert!(
            plan.report()
                .lines
                .last()
                .unwrap()
                .blockers
                .contains(&ItemSourceProblem::PossibleCombinedLine),
            "unproved source line cleared following uncertainty: {text}"
        );
        assert!(
            plan.convert(&lines).unwrap().modifiers.is_empty(),
            "unproved source conversion: {text}"
        );
    }
    for body in [
        "+10 to maximum Life",
        "+10% to all Elemental Resistances",
        "Adds 5 to 13 Cold Damage",
    ] {
        let plan = attribute("Sapphire Charm", "", body, &source, &lines);
        assert!(plan.report().lines.last().unwrap().member.is_none());
        assert!(plan.convert(&lines).unwrap().modifiers.is_empty());
    }
    // Successor publication does not retroactively apply new guards to the predecessor.
    let body = "10 to maximum Life";
    let prior = attribute("Sapphire Ring", "", body, &old, &lines);
    assert!(prior.report().lines.last().unwrap().member.is_some());
    let current = attribute("Sapphire Ring", "", body, &source, &lines);
    assert!(current.report().lines.last().unwrap().member.is_none());
}
type Admission = (u64, u64, usize);
fn admissions(sidecar: &Value) -> BTreeSet<Admission> {
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
                        .map(move |emission| (source, index, emission))
                })
        })
        .collect()
}
fn check_originals(cwd: &Path, prior: &Path, output: &Path) {
    let mut counts = vec![];
    for case in 1..=5 {
        let query = format!("queries-original-{case:02}.json");
        assert!(
            fs::read(prior.join(&query)).unwrap() == fs::read(output.join(&query)).unwrap(),
            "query bytes changed: {query}"
        );
        assert_eq!(json(output.join(query)).as_array().unwrap().len(), 22);
        let destination = cwd.join(format!("source-role-original-{case}"));
        let report = success(normalize(cwd, output, case, &destination, true));
        assert_eq!(report["normalization_status"], "pending");
        let before = json(cwd.join(format!("source-conditions-original-{case}/sidecar.json")));
        let after = json(destination.join("sidecar.json"));
        let previous = admissions(&before);
        let current = admissions(&after);
        assert_eq!(
            current, previous,
            "original-{case}: admitted occurrence set changed"
        );
        counts.push(current.len());
        for field in ["source_sha256", "source_bytes", "source_schema", "revision"] {
            assert_eq!(before[field], after[field], "original-{case}: {field}");
        }
        for item in after["item_texts"].as_array().unwrap() {
            let old = before["item_texts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|old| old["source"] == item["source"])
                .unwrap();
            let old_lines = old["lines"].as_array().unwrap();
            for row in item["lines"].as_array().unwrap() {
                let previous = old_lines
                    .iter()
                    .find(|old| old["index"] == row["index"])
                    .unwrap();
                assert_eq!(row["text"], previous["text"], "physical input changed");
                if !row["modifiers"].as_array().unwrap().is_empty() {
                    assert!(
                        row["outcome"] == previous["outcome"],
                        "admitted raw input changed: original-{case} source{} line{}",
                        item["source"]["ordinal"],
                        row["index"]
                    );
                }
            }
        }
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
        let mut complete = 0;
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
                    complete += 1;
                    let DraftField::Known { value } = &modifier.definition else {
                        panic!("known modifier")
                    };
                    assert_eq!(value.key().as_str(), "def.00000000000009da");
                }
            }
        }
        assert_eq!(
            complete,
            usize::from(case == 5),
            "legacy Life closure changed"
        );
        if matches!(case, 2 | 3) {
            for completion in [&input.items.completion, &input.equipment.completion] {
                assert!(
                    matches!(completion, DraftListCompletion::Pending { .. }),
                    "rune membership falsely complete"
                );
            }
        }
    }
    assert_eq!(counts, [0, 5, 1, 2, 2]);
}
pub fn check_source_role_migration(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("source-role-inputs");
    let authored_bytes = bundle(&authored);
    let prior_bytes = bundle(prior);
    let source = authored.join("item-source.json");
    let output = cwd.join("source-role-successor");
    let report = success(publish(cwd, prior, &authored, &source, &output));
    for field in [
        "allocated_entries",
        "refined_subjects",
        "appended_tables",
        "appended_programs",
        "appended_receivers",
    ] {
        assert_eq!(report["extension"][field], 0, "{field}");
    }
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
    assert!(
        published.keys().eq(prior_bytes.keys()),
        "bundle file set changed"
    );
    for (name, bytes) in &prior_bytes {
        if !matches!(
            name.as_str(),
            "item-source.json" | "manifest.json" | "transition.json" | "catalog-append.json"
        ) {
            assert!(
                published.get(name) == Some(bytes),
                "source-only publication changed {name}"
            );
        }
    }
    assert!(json(output.join("item-source.json")) == json(&source));
    check_policy(prior, &output);
    check_source_cases(prior, &output);
    assert!(
        !publish(cwd, prior, &authored, &source, &output)
            .status
            .success()
    );
    assert!(
        !publish(cwd, prior, &authored, &source, prior)
            .status
            .success()
    );
    equal_bundle(&bundle(&output), &published);
    equal_bundle(&bundle(prior), &prior_bytes);
    let replay = cwd.join("source-role-replay");
    assert_eq!(
        success(publish(cwd, prior, &authored, &source, &replay)),
        report
    );
    equal_bundle(&bundle(&replay), &published);
    let mut stale = json(&source);
    stale["item_lines"] =
        serde_json::to_value(digest_owned("stale-source-role-lines", &false, 1024).unwrap())
            .unwrap();
    let stale_path = cwd.join("source-role-stale.json");
    fs::write(&stale_path, serde_json::to_vec(&stale).unwrap()).unwrap();
    let rejected = cwd.join("source-role-rejected");
    assert!(
        !publish(cwd, prior, &authored, &stale_path, &rejected)
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
