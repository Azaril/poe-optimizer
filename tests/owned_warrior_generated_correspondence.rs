//! Archived generated correspondence preserves the actual unresolved owners.
#[path = "support/owned_warrior_generated_correspondence.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_draft::{DraftLimits, encode_draft},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{
        GemInventoryPolicy, NormalizationArtifacts, NormalizationLimits, NormalizationPolicy,
        NormalizedImport, normalize_fresh,
    },
    owned_release::{StagedOwnedRelease, assemble_owned_release},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

fn endpoints() -> &'static (StagedOwnedRelease, StagedOwnedRelease) {
    static PACKAGES: OnceLock<(StagedOwnedRelease, StagedOwnedRelease)> = OnceLock::new();
    PACKAGES.get_or_init(|| {
        let p = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_WARRIOR_CORRESPONDENCE_PRIOR")
                .expect("explicit current predecessor"),
        );
        let old = release::load(&p);
        let new = family::stage(&old);
        (old, new)
    })
}
fn original() -> String {
    fs::read_to_string(family::root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
        .unwrap()
}
fn normalize(
    p: &StagedOwnedRelease,
    xml: &str,
    policy: Option<&NormalizationPolicy>,
    limits: NormalizationLimits,
) -> Result<NormalizedImport, String> {
    normalize_case(p, xml, policy, limits, 5)
}
fn normalize_case(
    p: &StagedOwnedRelease,
    xml: &str,
    policy: Option<&NormalizationPolicy>,
    limits: NormalizationLimits,
    case: usize,
) -> Result<NormalizedImport, String> {
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([109; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let policy = policy.unwrap_or(p.normalization());
    let tree = OwnedTreeNormalizationPolicy::bind_new(
        p.input().tree.as_ref().unwrap().content.clone(),
        p.assembled().registry(),
        p.assembled().schema(),
        p.mapping(),
        policy,
        Default::default(),
    )
    .map_err(|e| e.to_string())?;
    normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            mappings: p.mapping(),
            registry: p.assembled().registry(),
            definitions: p.assembled().schema(),
            roles: p.roles(),
            rewards: p.rewards(),
            items: p.items(),
            item_source: p.item_source(),
            tree: Some(&tree),
        },
        policy,
        &p.query_sets()
            .iter()
            .find(|q| q.name.as_str() == format!("original-{case:02}"))
            .unwrap()
            .queries,
        limits,
    )
    .map_err(|e| e.to_string())
}
fn origins(result: &NormalizedImport) -> Value {
    json!(result.sidecar().origins)
}
fn pending(v: &Value, code: &str, ids: &mut Vec<Value>) {
    match v {
        Value::Object(o) => {
            if o.get("kind") == Some(&json!("pending")) && o.get("code") == Some(&json!(code)) {
                ids.push(o["id"].clone());
            }
            for v in o.values() {
                pending(v, code, ids);
            }
        }
        Value::Array(a) => {
            for v in a {
                pending(v, code, ids);
            }
        }
        _ => {}
    }
}
fn codes(result: &NormalizedImport, code: &str) -> Vec<Value> {
    let mut ids = vec![];
    pending(&json!(result.draft().input()), code, &mut ids);
    ids.sort_by_cached_key(Value::to_string);
    ids.dedup();
    ids
}
fn config_rows(result: &NormalizedImport) -> Vec<u64> {
    let issues = codes(result, "configuration-roles-not-converted");
    assert_eq!(issues.len(), 1);
    origins(result)
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| {
            r["links"]
                .as_array()
                .unwrap()
                .iter()
                .any(|l| l["kind"] == "issue" && l["value"] == issues[0])
        })
        .map(|r| r["source"]["ordinal"].as_u64().unwrap())
        .collect()
}
fn row(result: &NormalizedImport, ordinal: u64) -> Value {
    row_from(&origins(result), ordinal)
}
fn row_from(origins: &Value, ordinal: u64) -> Value {
    origins
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["source"]["ordinal"] == ordinal)
        .unwrap()
        .clone()
}
fn exact_physical_origin_delta(before: &NormalizedImport, after_origins: &Value, retired: &Value) {
    let draft = json!(before.draft().input());
    let before_gem = row(before, 152);
    let skill = &before_gem["links"]
        .as_array()
        .unwrap()
        .iter()
        .find(|link| link["kind"] == "skill")
        .unwrap()["value"];
    let preset = draft["skill_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|preset| {
            preset["skills"]["members"]
                .as_array()
                .unwrap()
                .contains(skill)
        })
        .unwrap();
    let usage = preset.pointer("/intent/usage/completion").unwrap();
    assert_eq!(usage["kind"], "pending");
    assert_eq!(usage["code"], "usage-preferences-not-converted");
    let config = codes(before, "configuration-roles-not-converted");
    for ordinal in [151, 152] {
        let old = row(before, ordinal);
        let new = row_from(after_origins, ordinal);
        let mut expected: Vec<_> = old["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|link| {
                link["kind"] != "issue"
                    || (link["value"] != retired["id"] && !config.contains(&link["value"]))
            })
            .cloned()
            .collect();
        for link in [
            json!({"kind":"skill_preset","value":preset["id"]}),
            json!({"kind":"issue","value":usage["id"]}),
        ] {
            if !expected.contains(&link) {
                expected.push(link);
            }
        }
        let mut actual = new["links"].as_array().unwrap().clone();
        expected.sort_by_cached_key(Value::to_string);
        actual.sort_by_cached_key(Value::to_string);
        assert_eq!(actual, expected, "exact physical origin delta at {ordinal}");
        // The new proof does not manufacture or replace a physical identity.
        for kind in ["gem", "skill"] {
            assert_eq!(
                old["links"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|l| l["kind"] == kind)
                    .collect::<Vec<_>>(),
                new["links"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|l| l["kind"] == kind)
                    .collect::<Vec<_>>()
            );
        }
    }
}
fn change(xml: &str, attribute: &str, value: &str) -> String {
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([109; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let set = evidence
        .rows()
        .iter()
        .find(|n| {
            n.occurrence().name() == "SkillSet"
                && n.attribute("id")
                    .is_some_and(|a| a.decoded().unwrap() == "3")
        })
        .unwrap();
    let gem = evidence
        .rows()
        .iter()
        .find(|n| {
            n.occurrence().name() == "Gem"
                && n.attribute("skillId")
                    .is_some_and(|a| a.decoded().unwrap() == "SummonSkeletalWarriorsPlayer")
                && n.occurrence().parent().is_some_and(|group| {
                    evidence.rows()[group.ordinal() as usize]
                        .occurrence()
                        .parent()
                        == Some(set.occurrence().id())
                })
        })
        .unwrap();
    let input = gem.attribute(attribute).unwrap();
    assert_ne!(input.decoded().unwrap(), value);
    let mut changed = xml.to_owned();
    changed.replace_range(input.range(), value);
    changed
}
#[test]
fn warrior_packet_retains_catalog_roles_and_partial_mechanics() {
    family::check_authored();
}

fn write_json(path: impl AsRef<Path>, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn selected_report(xml: &str, normalized: &NormalizedImport, dir: &Path, schema: &Path) -> Value {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("draft.json"),
        encode_draft(normalized.draft(), DraftLimits::default()).unwrap(),
    )
    .unwrap();
    write_json(dir.join("sidecar.json"), &json!(normalized.sidecar()));
    let report = selected::finalize_with_definitions(
        xml.as_bytes(),
        dir,
        &dir.join("selection.json"),
        schema,
    );
    write_json(dir.join("selected-report.json"), &report);
    report
}
fn original_one_item_grant_stays_unadmitted(
    before: &NormalizedImport,
    after: &NormalizedImport,
) -> Value {
    let old = json!(before.sidecar());
    let new = json!(after.sidecar());
    let item = |sidecar: &Value| {
        sidecar["item_texts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["source"]["ordinal"] == 328)
            .unwrap()
            .clone()
    };
    let old = item(&old);
    let new = item(&new);
    let line = |item: &Value| {
        item["lines"]
            .as_array()
            .unwrap()
            .iter()
            .find(|line| line["index"] == 14)
            .unwrap()
            .clone()
    };
    let attributed = |item: &Value| {
        item["attribution"]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .find(|line| line["index"] == 14)
            .unwrap()
            .clone()
    };
    let mut expected_line = line(&old);
    assert_eq!(
        expected_line["text"],
        "Grants Skill: Level 18 Skeletal Warrior Minion"
    );
    assert_eq!(
        expected_line["outcome"],
        json!({"kind":"pending","value":{"reason":{"kind":"source_meaning_unresolved"},"candidates":[]}})
    );
    assert_eq!(expected_line["modifiers"], json!([]));
    expected_line["outcome"]["value"]["candidates"] = json!(["fixed-item-warrior-grant"]);
    assert_eq!(
        line(&new),
        expected_line,
        "recognition alone must not supply the raw item grant"
    );
    let mut expected_attribution = attributed(&old);
    assert_eq!(
        expected_attribution["blockers"],
        json!(["possible_combined_line", "unknown_member", "unknown_header"])
    );
    assert_eq!(expected_attribution["rule"], Value::Null);
    expected_attribution["rule"] = json!("fixed-item-warrior-grant");
    expected_attribution["pending_candidates"] = json!(["fixed-item-warrior-grant"]);
    expected_attribution["blockers"] = json!(["possible_combined_line", "unknown_header"]);
    assert_eq!(attributed(&new), expected_attribution);
    assert_eq!(old["attribution"]["layout"], new["attribution"]["layout"]);
    assert_eq!(
        new["attribution"]["layout"],
        json!({"status":"pending","problems":["rune_lifecycle","unknown_member","unknown_header","possible_combined_line"]})
    );
    json!({"source_ordinal":328,"line":line(&new),"attribution":attributed(&new),"layout":new["attribution"]["layout"],"native_modifier_emitted":false})
}
// Recognizing the implicit also resolves the following explicit member. The
// second emission uses the unchanged, previously authored minion-level recipe;
// it is neither another Warrior grant nor an extra completion-issue identity.
fn original_five_exact_item_emissions(
    old: &StagedOwnedRelease,
    new: &StagedOwnedRelease,
    before: &NormalizedImport,
    after: &NormalizedImport,
) -> Vec<Value> {
    let recipe = |p: &StagedOwnedRelease| {
        json!(
            p.input()
                .items
                .rules
                .iter()
                .find(|r| r.id.as_str() == "fixed-global-minion-level")
                .unwrap()
        )
    };
    assert_eq!(recipe(old), recipe(new));
    let owner = |p: &StagedOwnedRelease| {
        json!(p.input().recipe.rules.owners)
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r.pointer("/owner/value/value/key") == Some(&json!("def.00000000000030ca")))
            .unwrap()
            .clone()
    };
    assert_eq!(owner(old), owner(new));
    let item_text = |n: &NormalizedImport| {
        json!(n.sidecar().item_texts)
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["source"]["ordinal"] == 583)
            .unwrap()
            .clone()
    };
    let prior = item_text(before);
    let next = item_text(after);
    assert_eq!(prior["source"], next["source"]);
    assert_eq!(prior["lines"].as_array().unwrap().len(), 20);
    assert_eq!(next["lines"].as_array().unwrap().len(), 20);
    assert_eq!(
        prior["lines"].as_array().unwrap()[..18],
        next["lines"].as_array().unwrap()[..18]
    );
    assert_eq!(
        prior["attribution"]["layout"],
        json!({"status":"pending","problems":["unknown_member","unknown_header","possible_combined_line"]})
    );
    assert_eq!(next["attribution"]["layout"], json!({"status":"proven"}));
    let item_link = row(after, 583)["links"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["kind"] == "item")
        .unwrap()["value"]
        .clone();
    let draft = json!(after.draft().input());
    let item = draft["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == item_link)
        .unwrap();
    assert_eq!(item["template"]["value"]["key"], "def.0000000000001fe3");
    assert_eq!(
        item["modifiers"]["completion"]["code"],
        "item-modifiers-not-converted"
    );
    assert_eq!(item["modifiers"]["members"].as_array().unwrap().len(), 2);
    let mut added = Vec::new();
    for (offset, definition, rule, text, category, kind) in [
        (
            18,
            "def.000000000000335e",
            "ranged-item-warrior-grant",
            "{range:0.5}Grants Skill: Level (1-20) Skeletal Warrior Minion",
            "implicit",
            "warrior_raw_grant",
        ),
        (
            19,
            "def.00000000000030ca",
            "fixed-global-minion-level",
            "+1 to Level of all Minion Skills",
            "explicit",
            "existing_minion_level_after_layout_resolution",
        ),
    ] {
        let old_line = &prior["lines"][offset];
        let line = &next["lines"][offset];
        assert_eq!(old_line["text"], text);
        assert_eq!(line["text"], text);
        assert_eq!(old_line["outcome"]["kind"], "pending");
        assert_eq!(old_line["modifiers"], json!([]));
        assert_eq!(
            old_line["outcome"]["value"]["candidates"],
            if offset == 18 {
                json!([])
            } else {
                json!([rule])
            }
        );
        assert_eq!(line["outcome"]["kind"], "known");
        assert_eq!(line["outcome"]["value"]["rule"], rule);
        assert_eq!(line["modifiers"].as_array().unwrap().len(), 1);
        let attribution = &next["attribution"]["lines"][offset];
        assert_eq!(attribution["rule"], rule);
        assert_eq!(
            attribution["member"],
            json!({"category":category,"ordinal":1,"line":offset+1})
        );
        assert_eq!(attribution["blockers"], json!([]));
        assert_eq!(
            attribution["range"],
            json!({"status":"resolved","fraction":0.5,"winning_write":offset-17})
        );
        let emitted = line["outcome"]["value"]["emissions"].as_array().unwrap();
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0]["kind"], "modifier");
        assert_eq!(emitted[0]["value"]["definition"]["key"], definition);
        let modifier = &item["modifiers"]["members"][offset - 18];
        assert_eq!(modifier["id"], line["modifiers"][0]);
        assert_eq!(
            modifier["definition"],
            json!({"kind":"known","value":emitted[0]["value"]["definition"]})
        );
        let rolls: Vec<_> = emitted[0]["value"]["rolls"].as_array().unwrap().iter()
            .map(|r| json!({"slot":{"kind":"known","value":r["slot"]},"value":{"kind":"known","value":r["value"]}})).collect();
        assert_eq!(
            modifier["rolls"],
            json!({"members":rolls,"completion":{"kind":"complete"}})
        );
        let amount = &rolls[0]["value"]["value"];
        if offset == 18 {
            assert_eq!(amount, &json!({"kind":"integer","value":11}));
        } else {
            assert_eq!(amount["kind"], "quantity");
            assert_eq!(amount["value"]["value"], 1.0);
            assert_eq!(amount["value"]["unit"]["key"], "def.000000000000295a");
        }
        added.push(json!({"kind":kind,"source_ordinal":583,"range_source_ordinal":next["attribution"]["writes"][offset-17]["origin"]["occurrence"]["ordinal"],"line":offset+1,"item":item_link,"modifier":modifier,"attribution":attribution}));
    }
    // The two existing XML writes become resolvable against the proved member
    // layout. No source identity, fraction, order, or inline write changes.
    let mut expected_writes = prior["attribution"]["writes"].clone();
    assert_eq!(expected_writes.as_array().unwrap().len(), 3);
    for (index, ordinal, line) in [(1, 584, 19), (2, 585, 20)] {
        assert_eq!(
            expected_writes[index]["origin"]["occurrence"]["ordinal"],
            ordinal
        );
        assert_eq!(
            expected_writes[index]["target"],
            json!({"status":"pending"})
        );
        expected_writes[index]["target"] = json!({"status":"line","value":line});
    }
    assert_eq!(expected_writes, next["attribution"]["writes"]);
    added
}

#[test]
#[ignore = "current canonical package and retained optional source evidence"]
fn five_originals_preserve_every_input_except_reviewed_warrior_grant_and_physical_inventory() {
    let (old, new) = endpoints();
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_WARRIOR_CORRESPONDENCE_OUT")
            .expect("publication output"),
    );
    fs::create_dir_all(&out).unwrap();
    for (name, package) in [("prior-schema.json", old), ("next-schema.json", new)] {
        fs::write(
            out.join(name),
            package
                .artifacts()
                .find(|(n, _)| *n == "schema.json")
                .unwrap()
                .1,
        )
        .unwrap();
    }
    let mut census = Vec::new();
    for case in 1..=5 {
        let xml = fs::read_to_string(family::root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        )))
        .unwrap();
        let before = normalize_case(old, &xml, None, Default::default(), case).unwrap();
        let after = normalize_case(new, &xml, None, Default::default(), case).unwrap();
        let unadmitted_grant = if case == 1 {
            original_one_item_grant_stays_unadmitted(&before, &after)
        } else {
            Value::Null
        };
        for (package, baseline) in [(old, &before), (new, &after)] {
            let repeated = normalize_case(package, &xml, None, Default::default(), case).unwrap();
            assert_eq!(baseline.draft().input(), repeated.draft().input());
            assert_eq!(json!(baseline.sidecar()), json!(repeated.sidecar()));
            assert_eq!(baseline.allocator_after(), repeated.allocator_after());
        }
        let mut original = json!(before.draft().input());
        let mut restored = json!(after.draft().input());
        let added = if case == 5 {
            original_five_exact_item_emissions(old, new, &before, &after)
        } else {
            Vec::new()
        };
        let mut completed = Vec::new();
        for item in restored["items"]["members"].as_array_mut().unwrap() {
            let item_id = item["id"].clone();
            item["modifiers"]["members"]
                .as_array_mut()
                .unwrap()
                .retain(|m| {
                    !added
                        .iter()
                        .any(|a| a["item"] == item_id && a["modifier"] == *m)
                });
        }
        let old_gems = original["gems"]["members"].as_array_mut().unwrap();
        let new_gems = restored["gems"]["members"].as_array_mut().unwrap();
        assert_eq!(old_gems.len(), new_gems.len());
        for (old_gem, gem) in old_gems.iter_mut().zip(new_gems) {
            assert_eq!(old_gem["definition"], gem["definition"]);
            if old_gem["parameters"]["completion"]["kind"]
                != gem["parameters"]["completion"]["kind"]
            {
                assert_eq!(case, 1);
                assert_eq!(gem["definition"]["value"]["key"], "def.000000000000091a");
                assert_eq!(
                    gem["parameters"]["members"],
                    old_gem["parameters"]["members"]
                );
                assert_eq!(gem["parameters"]["completion"], json!({"kind":"complete"}));
                assert_eq!(
                    old_gem["parameters"]["completion"]["code"],
                    "gem-parameters-not-converted"
                );
                completed.push(json!({"before_gem":old_gem["id"],"after_gem":gem["id"],"retired":old_gem["parameters"]["completion"]}));
                old_gem["parameters"]
                    .as_object_mut()
                    .unwrap()
                    .remove("completion");
                gem["parameters"]
                    .as_object_mut()
                    .unwrap()
                    .remove("completion");
            }
        }
        assert_eq!(
            added
                .iter()
                .filter(|a| a["kind"] == "warrior_raw_grant")
                .count(),
            usize::from(case == 5),
            "actual grant count {case}"
        );
        assert_eq!(added.len(), 2 * usize::from(case == 5));
        assert_eq!(completed.len(), usize::from(case == 1));
        let old_allocator = original["allocator"].clone();
        let new_allocator = restored["allocator"].clone();
        let issued =
            |v: &Value| u64::from_str_radix(v["last_issued"].as_str().unwrap(), 16).unwrap();
        // items::apply allocates once per converted modifier; complete rolls
        // allocate no issue. V3 retirement preserves its allocated issue.
        // Original05 admits exactly two modifiers at the resolved source frame.
        assert_eq!(
            issued(&old_allocator) + added.len() as u64,
            issued(&new_allocator)
        );
        assert_eq!(old_allocator["lineage"], new_allocator["lineage"]);
        assert_eq!(json!(before.allocator_after()), old_allocator);
        assert_eq!(json!(after.allocator_after()), new_allocator);
        restored["allocator"] = old_allocator.clone();
        let mut ids = BTreeMap::new();
        identity::correspond(
            &original,
            &mut restored,
            &mut ids,
            "whole source-ordered draft inverse",
        );
        assert_eq!(restored, original);
        let inserted: Vec<_> = added
            .iter()
            .map(|a| {
                u64::from_str_radix(a["modifier"]["id"]["local"].as_str().unwrap(), 16).unwrap()
            })
            .collect();
        for (next_id, prior_id) in &ids {
            let next_id: Value = serde_json::from_str(next_id).unwrap();
            assert_eq!(next_id["lineage"], prior_id["lineage"]);
            let next_local = u64::from_str_radix(next_id["local"].as_str().unwrap(), 16).unwrap();
            let prior_local = u64::from_str_radix(prior_id["local"].as_str().unwrap(), 16).unwrap();
            assert!(!inserted.contains(&next_local));
            assert_eq!(
                prior_local + inserted.iter().filter(|n| **n < next_local).count() as u64,
                next_local,
                "only the actual modifier insertion may relocate a retained identity"
            );
        }
        let old_origins = origins(&before);
        let mut new_origins = origins(&after);
        let item_origin_changes: Vec<_> = if case == 5 {
            [583, 584, 585]
                .into_iter()
                .map(|ordinal| {
                    json!({
                        "source_ordinal":ordinal,
                        "before":row_from(&old_origins, ordinal),
                        "after":row_from(&new_origins, ordinal),
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        let source = ImportedBuildInstance::from_decoded(
            decode_build(xml.as_bytes()).unwrap(),
            BuildLineage::from_bytes([109; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
        let before_draft = json!(before.draft().input());
        let after_draft = json!(after.draft().input());
        for addition in &added {
            let source_ordinal = addition["source_ordinal"].as_u64().unwrap();
            let range_ordinal = addition["range_source_ordinal"].as_u64().unwrap();
            let old_item_id = &ids[&addition["item"].to_string()];
            let find_item = |draft: &Value, id: &Value| {
                draft["items"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|item| &item["id"] == id)
                    .unwrap()
                    .clone()
            };
            let before_item = find_item(&before_draft, old_item_id);
            let after_item = find_item(&after_draft, &addition["item"]);
            let before_issue = &before_item["modifiers"]["completion"];
            let after_issue = &after_item["modifiers"]["completion"];
            for issue in [before_issue, after_issue] {
                assert_eq!(issue["kind"], "pending");
                assert_eq!(issue["code"], "item-modifiers-not-converted");
            }
            assert_eq!(ids[&after_issue["id"].to_string()], before_issue["id"]);
            let old_range = row_from(&old_origins, range_ordinal);
            assert_eq!(
                old_range["links"],
                json!([
                    {"kind":"item","value":old_item_id},
                    {"kind":"issue","value":before_issue["id"]},
                ])
            );
            let mut matches = Vec::new();
            for row in new_origins.as_array_mut().unwrap() {
                let links = row["links"].as_array().unwrap();
                if links.contains(&json!({"kind":"modifier","value":addition["modifier"]["id"]})) {
                    let ordinal = row["source"]["ordinal"].as_u64().unwrap();
                    matches.push(ordinal);
                    assert!(links.contains(&json!({"kind":"item","value":addition["item"]})));
                    let node = &evidence.rows()[ordinal as usize];
                    if ordinal == source_ordinal {
                        assert_eq!(node.occurrence().name(), "Item");
                        assert!(xml[node.occurrence().range()].contains("Skeletal Warrior Minion"));
                        row["links"].as_array_mut().unwrap().retain(|l| {
                            !(l["kind"] == "modifier" && l["value"] == addition["modifier"]["id"])
                        });
                    } else {
                        assert_eq!(ordinal, range_ordinal);
                        assert_eq!(node.occurrence().name(), "ModRange");
                        assert_eq!(
                            u64::from(node.occurrence().parent().unwrap().ordinal()),
                            source_ordinal
                        );
                        assert_eq!(row["source"], old_range["source"]);
                        assert_eq!(row["disposition"], old_range["disposition"]);
                        assert_eq!(
                            row["links"],
                            json!([
                                {"kind":"item","value":addition["item"]},
                                {"kind":"modifier","value":addition["modifier"]["id"]},
                            ])
                        );
                        // item_range_origins::attach formerly pointed this exact
                        // unresolved write at the live item inventory obligation.
                        // It now points at the proved output, without completing
                        // that inventory. Restore only this checked relationship.
                        row["links"][1] = json!({"kind":"issue","value":after_issue["id"]});
                    }
                }
            }
            assert_eq!(matches, [source_ordinal, range_ordinal]);
        }
        identity::relocate(&mut new_origins, &ids);
        if case == 1 {
            exact_physical_origin_delta(&before, &new_origins, &completed[0]["retired"]);
        }
        assert_eq!(
            old_origins.as_array().unwrap().len(),
            new_origins.as_array().unwrap().len()
        );
        let changed: Vec<_> = old_origins.as_array().unwrap().iter().zip(new_origins.as_array().unwrap()).filter_map(|(a,b)| {
            assert_eq!(a["source"],b["source"]);
            assert_eq!(a["disposition"],b["disposition"]);
            if a == b { return None; }
            let ordinal=a["source"]["ordinal"].as_u64().unwrap();
            if case == 1 { assert!([151,152].contains(&ordinal)); }
            else {
                assert_eq!(case,5);
                assert!([185,186,246,247,351,352,418,419].contains(&ordinal));
                let node=&evidence.rows()[ordinal as usize];
                let group=if node.occurrence().name()=="Gem" {node.occurrence().parent().unwrap()} else {node.occurrence().id()};
                let set=evidence.rows()[group.ordinal() as usize].occurrence().parent().unwrap();
                let set_row=row_from(&old_origins,u64::from(set.ordinal()));
                let preset_id=&set_row["links"].as_array().unwrap().iter().find(|l|l["kind"]=="skill_preset").unwrap()["value"];
                let draft=json!(before.draft().input());
                let preset=draft["skill_presets"]["members"].as_array().unwrap().iter().find(|p| &p["id"]==preset_id).unwrap();
                let mut expected=vec![json!({"kind":"skill_preset","value":preset_id})];
                for (pointer,code) in [("/intent/generated_inputs/completion","generated-skill-inputs-not-converted"),("/intent/usage/completion","usage-preferences-not-converted"),("/authored_support_order/completion","support-origin-discovery-not-converted")] {
                    let issue=preset.pointer(pointer).unwrap();
                    assert_eq!(issue["kind"],"pending"); assert_eq!(issue["code"],code);
                    expected.push(json!({"kind":"issue","value":issue["id"]}));
                }
                assert_eq!(b["links"],json!(expected));
            }
            Some(json!({"source_ordinal":ordinal,"before":a["links"],"after_relocated":b["links"]}))
        }).collect();
        assert_eq!(
            changed.len(),
            match case {
                1 => 2,
                5 => 8,
                _ => 0,
            }
        );
        let prior_report = selected_report(
            &xml,
            &before,
            &out.join(format!("original-{case:02}-prior")),
            &out.join("prior-schema.json"),
        );
        let next_report = selected_report(
            &xml,
            &after,
            &out.join(format!("original-{case:02}-next")),
            &out.join("next-schema.json"),
        );
        let mut expected = prior_report["finalization"]["issues"].clone();
        let mut actual = next_report["finalization"]["issues"].clone();
        expected
            .as_array_mut()
            .unwrap()
            .retain(|i| !completed.iter().any(|c| i["id"] == c["retired"]["id"]));
        identity::relocate(&mut actual, &ids);
        assert_eq!(
            expected, actual,
            "only the reviewed selected raw-input obligation can retire"
        );
        // Also retain a portable issue projection after exact IDs were checked.
        selected::canonical(&mut expected);
        selected::canonical(&mut actual);
        assert_eq!(expected, actual);
        let prior_selection = selected::selection(
            xml.as_bytes(),
            &out.join(format!("original-{case:02}-prior")),
        );
        let mut next_selection = selected::selection(
            xml.as_bytes(),
            &out.join(format!("original-{case:02}-next")),
        );
        identity::relocate(&mut next_selection, &ids);
        assert_eq!(prior_selection, next_selection);
        census.push(json!({"original":case,"added_item_modifiers":added,"unadmitted_item_grant":unadmitted_grant,"completed_physical_inputs":completed,"origin_changes":changed,"item_origin_changes":item_origin_changes,"allocator_before":old_allocator,"allocator_after":new_allocator,"identity_bijection":ids,"all_other_draft_values_exact_under_bijection":true,"selected_before":prior_report["selected_issue_summary"],"selected_after":next_report["selected_issue_summary"],"query_count":new.query_sets().iter().find(|q|q.name.as_str()==format!("original-{case:02}")).unwrap().queries.len()}));
    }
    let rebuilt = assemble_owned_release(new.input().clone(), Default::default()).unwrap();
    let before_artifacts: Vec<_> = new.artifacts().collect();
    let after_artifacts: Vec<_> = rebuilt.artifacts().collect();
    assert_eq!(before_artifacts.len(), 18);
    assert_eq!(before_artifacts, after_artifacts);
    assert_eq!(old.query_sets(), new.query_sets());
    assert_eq!(
        new.query_sets()
            .iter()
            .map(|q| q.queries.len())
            .sum::<usize>(),
        110
    );
    write_json(
        out.join("all-five-preservation.json"),
        &json!({"prior":old.receipt(),"next":new.receipt(),"rebuilt_artifacts":18,"rebuilt_bytes_exact":true,"query_sets_exact":true,"queries_preserved":110,"builds":census}),
    );
}
#[test]
#[ignore = "current canonical package and retained optional source evidence"]
fn publish_warrior_correspondence_and_account_exactly_eight_archived_origins() {
    let (old, new) = endpoints();
    let xml = original();
    let before = normalize(old, &xml, None, Default::default()).unwrap();
    let after = normalize(new, &xml, None, Default::default()).unwrap();
    let before_rows = config_rows(&before);
    let after_rows = config_rows(&after);
    let removed: Vec<_> = before_rows
        .iter()
        .filter(|id| !after_rows.contains(id))
        .copied()
        .collect();
    assert_eq!(removed, vec![185, 186, 246, 247, 351, 352, 418, 419]);
    assert_eq!((before_rows.len(), after_rows.len()), (44, 36));
    assert!(after_rows.iter().all(|id| before_rows.contains(id)));
    for ordinal in removed {
        let r = row(&after, ordinal);
        let links = r["links"].as_array().unwrap();
        assert_eq!(
            links.iter().filter(|l| l["kind"] == "skill_preset").count(),
            1
        );
        assert_eq!(links.iter().filter(|l| l["kind"] == "issue").count(), 3);
        assert_eq!(links.len(), 4, "no manufactured physical/generated target");
        for code in [
            "generated-skill-inputs-not-converted",
            "usage-preferences-not-converted",
            "support-origin-discovery-not-converted",
        ] {
            let ids = codes(&after, code);
            assert_eq!(
                links
                    .iter()
                    .filter(|l| l["kind"] == "issue" && ids.contains(&l["value"]))
                    .count(),
                1
            );
        }
    }
    for ordinal in [174, 175] {
        assert!(
            after_rows.contains(&ordinal),
            "archived Firebolt still lacks a Pending usage owner"
        );
    }
    for code in [
        "generated-skill-inputs-not-converted",
        "usage-preferences-not-converted",
        "support-origin-discovery-not-converted",
    ] {
        assert_eq!(codes(&before, code).len(), codes(&after, code).len());
    }
    let repeated = normalize(new, &xml, None, Default::default()).unwrap();
    assert_eq!(
        json!(after.draft().input()),
        json!(repeated.draft().input())
    );
    assert_eq!(origins(&after), origins(&repeated));
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_WARRIOR_CORRESPONDENCE_OUT")
            .expect("fresh publication output"),
    );
    assert!(!out.join("package").exists());
    fs::create_dir_all(out.join("package")).unwrap();
    for (name, bytes) in new.artifacts() {
        fs::write(out.join("package").join(name), bytes).unwrap();
    }
    fs::write(
        out.join("draft.json"),
        serde_json::to_vec_pretty(&after.draft().input()).unwrap(),
    )
    .unwrap();
    fs::write(
        out.join("sidecar.json"),
        serde_json::to_vec_pretty(after.sidecar()).unwrap(),
    )
    .unwrap();
    fs::write(out.join("report.json"),serde_json::to_vec_pretty(&json!({"prior":old.receipt().input,"next":new.receipt().input,"configuration_origins_before":before_rows,"configuration_origins_after":after_rows,"whole_build_complete":false,"numerical_coverage_added":false})).unwrap()).unwrap();
}
#[test]
#[ignore = "current canonical package and retained optional source evidence"]
fn malformed_or_foreign_warrior_fields_keep_configuration_fallback() {
    let (_, p) = endpoints();
    let xml = original();
    for (attribute, value) in [
        ("gemId", "Metadata/Items/Gems/SkillGemSkeletalWarrior"),
        ("variantId", "foreign"),
        ("skillId", "SummonSkeletalSnipersPlayer"),
        ("nameSpec", "foreign"),
        ("quality", "bad"),
        ("count", "bad"),
        ("enableGlobal1", "bad"),
        ("skillMinion", "SandDjinn"),
        ("skillMinionSkill", "2"),
        ("skillMinionSkillCalcs", "2"),
    ] {
        let changed = change(&xml, attribute, value);
        let result = normalize(p, &changed, None, Default::default()).unwrap();
        let linked = config_rows(&result);
        assert!(
            linked.contains(&185) && linked.contains(&186),
            "{attribute}={value}"
        );
        assert!(
            !linked.contains(&246) && !linked.contains(&247),
            "other preset stays independently proved"
        );
    }
}
#[test]
#[ignore = "current canonical package and retained optional source evidence"]
fn absent_physical_disposition_preserves_fallback_without_reclassifying_gem() {
    let (_, p) = endpoints();
    let mut policy = p.normalization().clone();
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 {
        primary_dispositions,
        ..
    }) = &mut policy.gem_inventory
    else {
        panic!()
    };
    primary_dispositions.retain(|r| r.physical.gem.key().as_str() != "def.000000000000091a");
    let result = normalize(p, &original(), Some(&policy), Default::default()).unwrap();
    let linked = config_rows(&result);
    for ordinal in [185, 186, 246, 247, 351, 352, 418, 419] {
        assert!(linked.contains(&ordinal));
    }
    assert_eq!(p.input().roles.roles, endpoints().0.input().roles.roles);
}
#[test]
#[ignore = "current canonical package and retained optional source evidence"]
fn exact_physical_correspondence_identity_and_aggregate_budget_are_required() {
    let (_, p) = endpoints();
    let xml = original();
    for field in [
        "gem",
        "game_id",
        "variant_id",
        "skill_id",
        "name_spec",
        "primary",
    ] {
        let mut policy = json!(p.normalization());
        let dispositions = policy["gem_inventory"]["primary_dispositions"]
            .as_array_mut()
            .unwrap();
        let row = dispositions
            .iter_mut()
            .find(|r| r["physical"]["gem"]["key"] == "def.000000000000091a")
            .unwrap();
        if field == "gem" || field == "primary" {
            row["reference_action"][field]["key"] = json!(if field == "gem" {
                "def.0000000000000011"
            } else {
                "def.0000000000000012"
            });
        } else {
            row["reference_action"][field] = json!("foreign");
        }
        let policy = serde_json::from_value(policy).unwrap();
        assert!(
            normalize(p, &xml, Some(&policy), Default::default()).is_err(),
            "{field}"
        );
    }
    let baseline = normalize(p, &xml, None, Default::default()).unwrap();
    let mut low = 1;
    let mut high = NormalizationLimits::default().max_work;
    while high - low > 1 {
        let mid = low + (high - low) / 2;
        if normalize(
            p,
            &xml,
            None,
            NormalizationLimits {
                max_work: mid,
                ..Default::default()
            },
        )
        .is_ok()
        {
            high = mid;
        } else {
            low = mid;
        }
    }
    assert_eq!(low + 1, high);
    assert!(
        normalize(
            p,
            &xml,
            None,
            NormalizationLimits {
                max_work: low,
                ..Default::default()
            }
        )
        .is_err()
    );
    let exact = normalize(
        p,
        &xml,
        None,
        NormalizationLimits {
            max_work: high,
            ..Default::default()
        },
    )
    .unwrap();
    let recovered = normalize(p, &xml, None, Default::default()).unwrap();
    for result in [exact, recovered] {
        assert_eq!(baseline.draft().input(), result.draft().input());
        assert_eq!(json!(baseline.sidecar()), json!(result.sidecar()));
    }
}
