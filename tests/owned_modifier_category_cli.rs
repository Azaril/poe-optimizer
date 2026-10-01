//! Explicit category input publication; whole-rule and build coverage stay Partial.
#[path = "support/owned_modifier_category.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use identity::{correspond, relocate};
use poe_optimizer_core::{
    owned_definitions::SlotOwnerDefId,
    owned_schema::{DefinitionDescriptor, SchemaClosure, SchemaDefinitionId, SchemaState},
};
use poe_optimizer_import::{
    owned_item_lines::ItemEmission,
    owned_item_source::ItemSourceDialect,
    owned_normalize::{EquipmentMembershipPolicy, GemQualityPolicy},
    owned_release::StagedOwnedRelease,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: impl AsRef<Path>, v: &impl Serialize) {
    fs::write(p, serde_json::to_vec(v).unwrap()).unwrap();
}
fn run(args: &[&Path]) -> Value {
    let o = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .args(args)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}
fn source_row(sidecar: &Value, ordinal: u64) -> &Value {
    sidecar["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["source"]["ordinal"] == ordinal)
        .unwrap()
}
fn compare_original(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let a: Value = read(old.join("draft.json"));
    let b: Value = read(new.join("draft.json"));
    let sa: Value = read(old.join("sidecar.json"));
    let sb: Value = read(new.join("sidecar.json"));
    let mut left = a["draft"].clone();
    let mut right = b["draft"].clone();
    let allocator = right["allocator"].clone();
    right["allocator"] = left["allocator"].clone();
    let mut removed_modifier = None;
    let mut old_pairs = vec![];
    let mut retired = vec![];
    let mut category_changes = vec![];
    assert_eq!(
        sa["item_texts"].as_array().unwrap().len(),
        sb["item_texts"].as_array().unwrap().len()
    );
    for (x, y) in sa["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .zip(sb["item_texts"].as_array().unwrap())
    {
        assert_eq!(x["source"], y["source"]);
        assert_eq!(x["content_entry"], y["content_entry"]);
        assert_eq!(
            x["lines"].as_array().unwrap().len(),
            y["lines"].as_array().unwrap().len()
        );
        for (p, q) in x["lines"]
            .as_array()
            .unwrap()
            .iter()
            .zip(y["lines"].as_array().unwrap())
        {
            assert_eq!(p["index"], q["index"]);
            assert_eq!(p["text"], q["text"]);
            let before = p["modifiers"].as_array().unwrap();
            let after = q["modifiers"].as_array().unwrap();
            if case == 1 && x["source"]["ordinal"] == 305 && p["index"] == 13 {
                assert_eq!(before.len(), 1);
                assert!(after.is_empty());
                assert_eq!(q["outcome"]["kind"], "pending");
                assert_eq!(
                    q["outcome"]["value"]["reason"]["kind"],
                    "missing_context_option"
                );
                assert_eq!(
                    q["outcome"]["value"]["reason"]["value"]["input"],
                    family::bindings().input.as_str()
                );
                assert_eq!(
                    q["outcome"]["value"]["candidates"],
                    json!(["fixed-global-minion-level"])
                );
                assert_eq!(y["attribution"]["layout"]["status"], "pending");
                removed_modifier = Some(before[0].clone());
                category_changes.push(json!({"source":y["source"],"line":q["index"],"raw":q["text"],"before":p["outcome"],"after":q["outcome"],"layout":y["attribution"]["layout"],"previous_member":x["attribution"]["lines"].as_array().unwrap().iter().find(|v|v["index"]==13).unwrap()["member"]}));
            } else {
                assert_eq!(
                    before.len(),
                    after.len(),
                    "unexpected source admission change"
                );
                old_pairs.extend(before.iter().cloned().zip(after.iter().cloned()));
            }
        }
    }
    let mut removed_count = 0;
    let mut category_count = 0;
    for item in left["items"]["members"].as_array_mut().unwrap() {
        item["modifiers"]["members"]
            .as_array_mut()
            .unwrap()
            .retain(|m| {
                if Some(&m["id"]) != removed_modifier.as_ref() {
                    return true;
                }
                assert_eq!(m["definition"]["value"]["key"], "def.00000000000030ca");
                retired.push(m["rolls"]["completion"]["id"].clone());
                removed_count += 1;
                false
            });
    }
    for (i, item) in right["items"]["members"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        for m in item["modifiers"]["members"].as_array_mut().unwrap() {
            if m["definition"]["value"]["key"] != "def.00000000000030ca" {
                continue;
            }
            assert_eq!(case, 5);
            category_count += 1;
            assert_eq!(m["rolls"]["members"].as_array().unwrap().len(), 24);
            let category = m["rolls"]["members"].as_array_mut().unwrap().pop().unwrap();
            assert_eq!(
                category["slot"]["value"],
                serde_json::to_value(family::bindings().slot).unwrap()
            );
            assert_eq!(
                category["value"],
                json!({"kind":"known","value":{"kind":"option","value":family::bindings().explicit}})
            );
            assert_eq!(m["rolls"]["completion"], json!({"kind":"complete"}));
            let prior = a["draft"]["items"]["members"][i]["modifiers"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["definition"] == m["definition"])
                .unwrap();
            retired.push(prior["rolls"]["completion"]["id"].clone());
            m["rolls"]["completion"] = prior["rolls"]["completion"].clone();
        }
    }
    assert_eq!(removed_count, usize::from(case == 1));
    assert_eq!(category_count, usize::from(case == 5));
    assert_eq!(retired.len(), usize::from(matches!(case, 1 | 5)));
    let mut ids = BTreeMap::new();
    correspond(&left, &mut right, &mut ids, "draft");
    for (old, mut new) in old_pairs {
        relocate(&mut new, &ids);
        assert_eq!(old, new, "retained modifier source identity");
    }
    assert_eq!(
        sa["origins"].as_array().unwrap().len(),
        sb["origins"].as_array().unwrap().len()
    );
    for (x, y) in sa["origins"]
        .as_array()
        .unwrap()
        .iter()
        .zip(sb["origins"].as_array().unwrap())
    {
        assert_eq!(x["source"], y["source"]);
        assert_eq!(x["disposition"], y["disposition"]);
        let old: Vec<_> = x["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["kind"] != "issue" && Some(&v["value"]) != removed_modifier.as_ref())
            .cloned()
            .collect();
        let mut new: Vec<_> = y["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["kind"] != "issue")
            .cloned()
            .collect();
        for v in &mut new {
            relocate(v, &ids);
        }
        assert_eq!(old, new);
    }
    if matches!(case, 2..=4) {
        let mut a = a.clone();
        let mut b = b.clone();
        selected::canonical(&mut a);
        selected::canonical(&mut b);
        assert_eq!(a, b);
    }
    let before = selected::finalize(
        xml,
        old,
        &out.join(format!("original-{case:02}-prior-selection.json")),
    );
    let after = selected::finalize(
        xml,
        new,
        &out.join(format!("original-{case:02}-selection.json")),
    );
    write(
        out.join(format!("original-{case:02}-prior-selected-report.json")),
        &before,
    );
    write(
        out.join(format!("original-{case:02}-selected-report.json")),
        &after,
    );
    let mut old_issues = before["finalization"]["issues"].as_array().unwrap().clone();
    let retired_issues: Vec<_> = old_issues
        .iter()
        .filter(|v| retired.contains(&v["id"]))
        .cloned()
        .collect();
    old_issues.retain(|v| !retired.contains(&v["id"]));
    let mut new_issues = after["finalization"]["issues"].as_array().unwrap().clone();
    for v in &mut new_issues {
        relocate(v, &ids);
    }
    let mut paths = BTreeMap::new();
    for (i, item) in b["draft"]["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        for (j, m) in item["modifiers"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            let mut id = m["id"].clone();
            relocate(&mut id, &ids);
            let old = a["draft"]["items"]["members"][i]["modifiers"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .position(|v| v["id"] == id)
                .unwrap();
            paths.insert(
                format!("items.members[{i}].modifiers.members[{old}]."),
                format!("items.members[{i}].modifiers.members[{j}]."),
            );
        }
    }
    let mut changed_paths = vec![];
    for old in &old_issues {
        let new = new_issues
            .iter_mut()
            .find(|v| v["id"] == old["id"])
            .unwrap();
        if old["path"] != new["path"] {
            let p = old["path"].as_str().unwrap();
            let (from, to) = paths
                .iter()
                .find(|(k, _)| p.starts_with(k.as_str()))
                .unwrap();
            assert_eq!(new["path"], format!("{}{}", to, &p[from.len()..]));
            changed_paths.push(json!({"id":old["id"],"before":old["path"],"after":new["path"]}));
            new["path"] = old["path"].clone();
        }
    }
    assert_eq!(
        old_issues, new_issues,
        "every other selected issue retained, no new issues"
    );
    assert_eq!(retired_issues.len(), usize::from(matches!(case, 1 | 5)));
    let mut selection = selected::selection(xml, new);
    relocate(&mut selection, &ids);
    assert_eq!(selection, selected::selection(xml, old));
    if case == 5 {
        let crown = source_row(&sb, 576);
        assert_eq!(crown["attribution"]["layout"], json!({"status":"proven"}));
        assert_eq!(crown["defaults"]["item_level_absent"], true);
    }
    json!({"original":case,"selected_before":before["finalization"]["issues"].as_array().unwrap().len(),"selected_after":after["finalization"]["issues"].as_array().unwrap().len(),"retired_issues":retired_issues,"relocated_issue_paths":changed_paths,"category_changes":category_changes,"new_category_rolls":category_count,"removed_unproven_modifiers":removed_count,"allocator_before":a["draft"]["allocator"],"allocator_after":allocator,"selected_issue_summary":after["selected_issue_summary"]})
}
fn probes(package: &Path, out: &Path) -> usize {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let start = original.find("<Item id=\"21\">").unwrap();
    let end = start + original[start..].find("</Item>").unwrap() + 7;
    let item = &original[start..end];
    let b = family::bindings();
    let cases = [
        (
            "explicit",
            "+1 to Level of all Minion Skills",
            Some(b.explicit),
        ),
        (
            "implicit",
            "+1 to Level of all Minion Skills",
            Some(b.implicit),
        ),
        (
            "enchant",
            "{enchant}+1 to Level of all Minion Skills",
            Some(b.enchant),
        ),
        (
            "desecrated",
            "{desecrated}+1 to Level of all Minion Skills",
            None,
        ),
        (
            "tagged",
            "{tags:minion}+1 to Level of all Minion Skills",
            None,
        ),
        ("fractional", "+1.5 to Level of all Minion Skills", None),
        (
            "unknown-prefix",
            "Unreviewed source input\n+1 to Level of all Minion Skills",
            None,
        ),
    ];
    for (label, text, expected) in &cases {
        let mut changed = item.replace("+1 to Level of all Minion Skills", text);
        if *label == "implicit" {
            changed = changed.replace("Implicits: 0", "Implicits: 1");
        }
        let xml = format!("{}{}{}", &original[..start], changed, &original[end..]);
        let path = out.join(format!("probe-{label}.xml"));
        fs::write(&path, xml).unwrap();
        let dest = out.join(format!("probe-{label}"));
        release::normalize(package, &path, 5, &dest);
        let s: Value = read(dest.join("sidecar.json"));
        let r = source_row(&s, 576);
        let line = r["lines"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| {
                v["text"]
                    .as_str()
                    .unwrap()
                    .contains("to Level of all Minion Skills")
            })
            .unwrap();
        assert_eq!(
            line["modifiers"].as_array().unwrap().len(),
            usize::from(expected.is_some()),
            "{label}"
        );
        if let Some(expected) = expected {
            assert_eq!(r["attribution"]["layout"]["status"], "proven");
            let d: Value = read(dest.join("draft.json"));
            let m = d["draft"]["items"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|v| v["modifiers"]["members"].as_array().unwrap())
                .find(|v| v["id"] == line["modifiers"][0])
                .unwrap();
            assert_eq!(m["rolls"]["completion"], json!({"kind":"complete"}));
            let roll = m["rolls"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["slot"]["value"]["slot"]["key"] == "def.00000000000030e5")
                .unwrap();
            assert_eq!(
                roll["value"]["value"],
                json!({"kind":"option","value":expected})
            );
        } else {
            assert_eq!(r["attribution"]["layout"]["status"], "pending");
            assert_eq!(r["defaults"]["item_level_absent"], false);
        }
    }
    cases.len()
}
#[test]
fn finite_category_recipe_retains_existing_inputs_and_uses_only_generic_context() {
    let b = family::bindings();
    let e = family::extension();
    assert_eq!(e.schema.len(), 5);
    assert!(e.owners.is_empty() && e.tables.is_empty() && e.receivers.is_empty());
    assert_eq!(
        b.slot.declaration,
        SlotOwnerDefId::Modifier(b.modifier.clone())
    );
    let v = serde_json::to_value(family::roll()).unwrap();
    assert_eq!(
        v["value"],
        json!({"kind":"context_option","value":{"input":b.input}})
    );
    let binding = family::source_binding();
    assert_eq!(binding.input, b.input);
    let values = serde_json::to_value(binding).unwrap();
    assert_eq!(
        values["values"],
        json!([{"category":"enchant","value":b.enchant},{"category":"implicit","value":b.implicit},{"category":"explicit","value":b.explicit}])
    );
    let a = family::authoring();
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        a["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    let m: Value = serde_json::from_slice(&bytes).unwrap();
    for p in a["source_files"].as_array().unwrap() {
        assert!(m["files"].as_array().unwrap().contains(p));
    }
}
fn preservation(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    let b = prior.input();
    let mut n = next.input().clone();
    let ids = family::bindings();
    assert_eq!(
        n.recipe.registry.entries.len(),
        b.recipe.registry.entries.len() + 4
    );
    assert_eq!(
        &n.recipe.registry.entries[..b.recipe.registry.entries.len()],
        b.recipe.registry.entries.as_slice()
    );
    assert_eq!(n.recipe.registry.last_issued.get(), 0x30e5);
    n.recipe.registry = b.recipe.registry.clone();
    assert_eq!(
        n.recipe.schema.definitions.len(),
        b.recipe.schema.definitions.len() + 3
    );
    n.recipe.schema.definitions.retain(|v| {
        ![
            ids.explicit.address(),
            ids.implicit.address(),
            ids.enchant.address(),
        ]
        .contains(&v.address())
    });
    let old = b
        .recipe
        .schema
        .definitions
        .iter()
        .find(|v| v.address() == ids.modifier.address())
        .unwrap();
    let new = n
        .recipe
        .schema
        .definitions
        .iter_mut()
        .find(|v| v.address() == ids.modifier.address())
        .unwrap();
    let (DefinitionDescriptor::Modifier(a), DefinitionDescriptor::Modifier(z)) = (old, new) else {
        panic!()
    };
    let (SchemaState::Known(a), SchemaState::Known(z)) = (&a.schema, &mut z.schema) else {
        panic!()
    };
    assert_eq!(z.declarations.parameters.members.len(), 24);
    assert_eq!(z.declarations.parameters.closure, SchemaClosure::Complete);
    assert_eq!(z.declarations.parameters.members.pop().unwrap(), ids.slot);
    z.declarations.parameters.closure = a.declarations.parameters.closure.clone();
    assert_eq!(n.recipe.schema.slots.len(), b.recipe.schema.slots.len() + 1);
    n.recipe.schema.slots.retain(|v| {
        v.address() != poe_optimizer_core::owned_schema::SlotAddress::Parameter(ids.slot.clone())
    });
    n.recipe.schema.release = b.recipe.schema.release.clone();
    n.recipe.rules.definitions = b.recipe.rules.definitions.clone();
    n.recipe.routing.definitions = b.recipe.routing.definitions.clone();
    n.mapping.definitions = b.mapping.definitions.clone();
    n.mapping.registry = b.mapping.registry;
    n.roles.definitions = b.roles.definitions.clone();
    n.roles.mapping = b.roles.mapping;
    let (GemQualityPolicy::Attributes(x), GemQualityPolicy::Attributes(y)) = (
        &mut n.normalization.gem_quality,
        &b.normalization.gem_quality,
    ) else {
        panic!()
    };
    x.definitions = y.definitions.clone();
    n.normalization.gem_inputs.as_mut().unwrap().definitions = b
        .normalization
        .gem_inputs
        .as_ref()
        .unwrap()
        .definitions
        .clone();
    let (
        Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 { definitions: x, .. }),
        Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 { definitions: y, .. }),
    ) = (
        &mut n.normalization.equipment_membership,
        &b.normalization.equipment_membership,
    )
    else {
        panic!()
    };
    *x = y.clone();
    n.rewards.definitions = b.rewards.definitions.clone();
    n.rewards.mapping = b.rewards.mapping;
    let rule = n
        .items
        .rules
        .iter_mut()
        .find(|v| v.id.as_str() == "fixed-global-minion-level")
        .unwrap();
    let [ItemEmission::Modifier { rolls, .. }] = rule.emissions.as_mut_slice() else {
        panic!()
    };
    assert_eq!(rolls.pop().unwrap(), family::roll());
    n.items.schema_version = b.items.schema_version;
    n.items.version = b.items.version.clone();
    n.items.definitions = b.items.definitions.clone();
    let ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
        flag_bindings,
        metadata_rules,
        single_modifier_conditions,
        preamble_observations,
        category_bindings,
    } = n.item_source.dialect
    else {
        panic!()
    };
    assert_eq!(category_bindings, vec![family::source_binding()]);
    n.item_source.dialect = ItemSourceDialect::PobExportedSingleTextObservationsV1 {
        flag_bindings,
        metadata_rules,
        single_modifier_conditions,
        preamble_observations,
    };
    n.item_source.schema_version = b.item_source.schema_version;
    n.item_source.version = b.item_source.version.clone();
    n.item_source.item_lines = b.item_source.item_lines;
    assert_eq!(
        n.tree.as_ref().unwrap().content,
        b.tree.as_ref().unwrap().content
    );
    n.tree = b.tree.clone();
    assert_eq!(n.provenance.len(), 13);
    n.provenance.truncate(11);
    assert_eq!(
        n, *b,
        "only exact category append, input inventory correction, policy versions and checked bindings change"
    );
}
#[test]
#[ignore = "requires exact real global-minion-level-release-03 predecessor"]
fn real_category_successor_preserves_originals_and_never_promotes_pending_category() {
    let p = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_CATEGORY_PRIOR").expect("explicit prior"),
    );
    let hashes = release::inventory(&p);
    let prior = release::load(&p);
    let prepared = family::stage(&prior);
    preservation(&prior, &prepared.final_release);
    let tmp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_CATEGORY_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| tmp.path().join("publication"));
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    write(out.join("append-endpoint.json"), prepared.appended.input());
    write(out.join("revision.json"), &prepared.revision);
    let append = out.join("appended");
    assert_eq!(
        run(&[
            &out.join("append-endpoint.json"),
            Path::new("--output"),
            &append
        ]),
        serde_json::to_value(prepared.appended.receipt()).unwrap()
    );
    let package = out.join("package");
    assert_eq!(
        run(&[
            &append,
            Path::new("--revision"),
            &out.join("revision.json"),
            Path::new("--output"),
            &package
        ]),
        serde_json::to_value(prepared.final_release.receipt()).unwrap()
    );
    let rebuilt = out.join("rebuilt");
    assert_eq!(
        run(&[&package, Path::new("--output"), &rebuilt]),
        serde_json::to_value(prepared.final_release.receipt()).unwrap()
    );
    assert_eq!(release::inventory(&package), release::inventory(&rebuilt));
    let mut reports = vec![];
    for case in 1..=5 {
        let q = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(p.join(&q)).unwrap(),
            fs::read(package.join(q)).unwrap()
        );
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&p, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        reports.push(compare_original(
            case,
            &fs::read(xml).unwrap(),
            &old,
            &new,
            &out,
        ));
    }
    let probes = probes(&package, &out);
    assert_eq!(hashes, release::inventory(&p));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":prepared.final_release.receipt().input,"definitions":prepared.final_release.receipt().definitions,"registry":prepared.final_release.receipt().registry,"provenance":13,"queries":110,"originals":reports,"probes":probes,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
