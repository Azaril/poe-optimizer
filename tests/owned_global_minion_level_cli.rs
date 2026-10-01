#[path = "support/owned_family_preservation.rs"]
mod preservation;
// Narrow source-admitted item family; this does not close whole-build coverage.
#[path = "support/owned_global_minion_level.rs"]
mod family;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_import::owned_release::StagedOwnedRelease;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[path = "support/owned_identity_correspondence.rs"]
mod identity;
use identity::{correspond, relocate};
fn compare_original(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let a: Value = read(old.join("draft.json"));
    let b: Value = read(new.join("draft.json"));
    let sa: Value = read(old.join("sidecar.json"));
    let sb: Value = read(new.join("sidecar.json"));
    let expected: &[(u64, u64, &str)] = match case {
        1 => &[
            (272, 16, "def.0000000000002a33"),
            (305, 13, "def.00000000000030ca"),
            (305, 14, "def.000000000000298c"),
        ],
        5 => &[
            (469, 13, "def.000000000000298c"),
            (480, 20, "def.000000000000298c"),
            (576, 20, "def.00000000000030ca"),
        ],
        _ => &[],
    };
    let mut additions = BTreeMap::new();
    let mut old_modifier_pairs = vec![];
    let mut census = vec![];
    let texts = sa["item_texts"].as_array().unwrap();
    assert_eq!(texts.len(), sb["item_texts"].as_array().unwrap().len());
    for (ta, tb) in texts.iter().zip(sb["item_texts"].as_array().unwrap()) {
        assert_eq!(ta["source"], tb["source"]);
        assert_eq!(ta["content_entry"], tb["content_entry"]);
        let ordinal = tb["source"]["ordinal"].as_u64().unwrap();
        assert_eq!(
            ta["lines"].as_array().unwrap().len(),
            tb["lines"].as_array().unwrap().len()
        );
        for (la, lb) in ta["lines"]
            .as_array()
            .unwrap()
            .iter()
            .zip(tb["lines"].as_array().unwrap())
        {
            assert_eq!(la["index"], lb["index"]);
            assert_eq!(la["text"], lb["text"]);
            let index = lb["index"].as_u64().unwrap();
            let x = la["modifiers"].as_array().unwrap();
            let y = lb["modifiers"].as_array().unwrap();
            if let Some((_, _, definition)) = expected
                .iter()
                .find(|(o, i, _)| (*o, *i) == (ordinal, index))
            {
                assert!(x.is_empty());
                assert_eq!(y.len(), 1);
                assert_eq!(lb["outcome"]["kind"], "known");
                additions.insert(
                    serde_json::to_string(&y[0]).unwrap(),
                    (*definition).to_string(),
                );
            } else {
                assert_eq!(
                    x.len(),
                    y.len(),
                    "unexpected admission at {case}/{ordinal}/{index}"
                );
                old_modifier_pairs.extend(x.iter().cloned().zip(y.iter().cloned()));
            }
            if lb["text"]
                .as_str()
                .unwrap()
                .contains("to Level of all Minion Skills")
            {
                census.push(json!({"source":tb["source"],"line":index,"raw":lb["text"],"outcome":lb["outcome"],"modifier_ids":lb["modifiers"],"layout":tb["attribution"]["layout"],"source_line":tb["attribution"]["lines"].as_array().unwrap().iter().find(|v|v["index"]==lb["index"])}));
            }
        }
    }
    assert_eq!(additions.len(), expected.len());
    let mut restored = b["draft"].clone();
    let allocator = restored["allocator"].clone();
    restored["allocator"] = a["draft"]["allocator"].clone();
    let mut fresh_issues = BTreeSet::new();
    let mut removed = vec![];
    let mut added = vec![];
    let old_items = a["draft"]["items"]["members"].as_array().unwrap();
    for (index, item) in restored["items"]["members"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        item["modifiers"]["members"].as_array_mut().unwrap().retain(|m| {
            let Some(def)=additions.get(&serde_json::to_string(&m["id"]).unwrap()) else {return true};
            assert_eq!(m["definition"]["value"]["key"],*def);
            assert_eq!(m["rolls"]["completion"]["code"],"modifier-roll-schema-partial");
            fresh_issues.insert(serde_json::to_string(&m["rolls"]["completion"]["id"]).unwrap());
            added.push(json!({"item":item_id(&b,index),"modifier":m["id"],"definition":def,"rolls_completion":m["rolls"]["completion"]}));false
        });
        if case == 5
            && item["template"]["value"]["key"] == "def.0000000000001f1c"
            && old_items[index]["item_level"]["kind"] == "pending"
        {
            assert_eq!(item["item_level"], json!({"kind":"known","value":null}));
            removed.push(old_items[index]["item_level"]["id"].clone());
            item["item_level"] = old_items[index]["item_level"].clone();
        }
    }
    assert_eq!(removed.len(), usize::from(case == 5));
    assert_eq!(
        added.len(),
        expected.len(),
        "every admitted sidecar member occurs in the draft"
    );
    let mut ids = BTreeMap::new();
    correspond(&a["draft"], &mut restored, &mut ids, "draft");
    for (old, mut new) in old_modifier_pairs {
        relocate(&mut new, &ids);
        assert_eq!(
            old, new,
            "existing modifier keeps exact source correspondence"
        );
    }
    assert_eq!(
        sa["origins"].as_array().unwrap().len(),
        sb["origins"].as_array().unwrap().len()
    );
    for (oa, ob) in sa["origins"]
        .as_array()
        .unwrap()
        .iter()
        .zip(sb["origins"].as_array().unwrap())
    {
        assert_eq!(oa["source"], ob["source"]);
        assert_eq!(oa["disposition"], ob["disposition"]);
        let old: Vec<_> = oa["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["kind"] != "issue")
            .cloned()
            .collect();
        let mut new: Vec<_> = ob["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| {
                v["kind"] != "issue"
                    && !additions.contains_key(&serde_json::to_string(&v["value"]).unwrap())
            })
            .cloned()
            .collect();
        for v in &mut new {
            relocate(v, &ids)
        }
        assert_eq!(old, new, "unchanged exact source-to-entity links");
    }
    if matches!(case, 2..=4) {
        let mut x = a.clone();
        let mut y = b.clone();
        selected::canonical(&mut x);
        selected::canonical(&mut y);
        assert_eq!(x, y, "unaffected originals keep exact local IDs");
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
    let mut new_issues = after["finalization"]["issues"].as_array().unwrap().clone();
    let retired: Vec<_> = old_issues
        .iter()
        .filter(|v| removed.contains(&v["id"]))
        .cloned()
        .collect();
    old_issues.retain(|v| !removed.contains(&v["id"]));
    let introduced: Vec<_> = new_issues
        .iter()
        .filter(|v| fresh_issues.contains(&serde_json::to_string(&v["id"]).unwrap()))
        .cloned()
        .collect();
    new_issues.retain(|v| !fresh_issues.contains(&serde_json::to_string(&v["id"]).unwrap()));
    for v in &mut new_issues {
        relocate(v, &ids);
    }
    // New members precede some existing modifiers; derive the only permitted
    // path changes from those exact source-corresponding modifier identities.
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
            if additions.contains_key(&serde_json::to_string(&m["id"]).unwrap()) {
                continue;
            }
            let mut id = m["id"].clone();
            relocate(&mut id, &ids);
            let old = old_items[i]["modifiers"]["members"]
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
    let mut path_changes = vec![];
    for old in &old_issues {
        let new = new_issues
            .iter_mut()
            .find(|v| v["id"] == old["id"])
            .unwrap();
        if new["path"] != old["path"] {
            let path = old["path"].as_str().unwrap();
            let (prefix, to) = paths
                .iter()
                .find(|(prefix, _)| path.starts_with(prefix.as_str()))
                .expect("only added source members relocate paths");
            assert_eq!(new["path"], format!("{}{}", to, &path[prefix.len()..]));
            path_changes.push(json!({"id":old["id"],"before":old["path"],"after":new["path"]}));
            new["path"] = old["path"].clone();
        }
    }
    assert_eq!(
        old_issues, new_issues,
        "all remaining selected issue identities and ownership preserved"
    );
    assert_eq!(
        introduced.len(),
        if case == 1 {
            3
        } else if case == 5 {
            1
        } else {
            0
        }
    );
    assert_eq!(retired.len(), usize::from(case == 5));
    let mut selection_new = selected::selection(xml, new);
    relocate(&mut selection_new, &ids);
    assert_eq!(
        selection_new,
        selected::selection(xml, old),
        "saved selected request follows exact source correspondence"
    );
    if case == 5 {
        let crown = sb["item_texts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["source"]["ordinal"] == 576)
            .unwrap();
        assert_eq!(crown["attribution"]["layout"], json!({"status":"proven"}));
        assert_eq!(crown["defaults"]["item_level_absent"], true);
    }
    json!({"original":case,"selected_before":before["finalization"]["issues"].as_array().unwrap().len(),"selected_after":after["finalization"]["issues"].as_array().unwrap().len(),"introduced_selected_issues":introduced,"retired_selected_issues":retired,"relocated_issue_paths":path_changes,"source_admission":census,"added_occurrences":added,"allocator_before":a["draft"]["allocator"],"allocator_after":allocator,"corresponding_ids":ids.len(),"selected_issue_summary":after["selected_issue_summary"]})
}
fn probes(package: &Path, out: &Path) -> usize {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let start = original.find("<Item id=\"21\">").unwrap();
    let end = start + original[start..].find("</Item>").unwrap() + 7;
    let item = &original[start..end];
    let needle = "+1 to Level of all Minion Skills";
    assert_eq!(item.matches(needle).count(), 1);
    let cases = [
        (
            "desecrated",
            "{desecrated}+1 to Level of all Minion Skills",
            false,
        ),
        (
            "tagged",
            "{tags:minion}+1 to Level of all Minion Skills",
            false,
        ),
        ("fractional", "+1.5 to Level of all Minion Skills", false),
        ("negative", "-1 to Level of all Minion Skills", false),
        (
            "corruption",
            "{corruptedRange:1.5}+1 to Level of all Minion Skills",
            false,
        ),
        ("range", "+(1-1) to Level of all Minion Skills", false),
        ("implicit", "+1 to Level of all Minion Skills", true),
        ("enchant", "{enchant}+1 to Level of all Minion Skills", true),
    ];
    for (label, text, admitted) in cases {
        let mut changed = item.replace(needle, text);
        if label == "implicit" {
            changed = changed.replace("Implicits: 0", "Implicits: 1");
        }
        let xml = format!("{}{}{}", &original[..start], changed, &original[end..]);
        let path = out.join(format!("probe-{label}.xml"));
        fs::write(&path, xml).unwrap();
        let dest = out.join(format!("probe-{label}"));
        release::normalize(package, &path, 5, &dest);
        let sidecar: Value = read(dest.join("sidecar.json"));
        let row = sidecar["item_texts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["source"]["ordinal"] == 576)
            .unwrap();
        let line = row["lines"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| {
                v["text"]
                    .as_str()
                    .is_some_and(|s| s.contains("to Level of all Minion Skills"))
            })
            .unwrap();
        assert_eq!(
            line["modifiers"].as_array().unwrap().len(),
            usize::from(admitted),
            "{label}"
        );
        assert_eq!(row["defaults"]["item_level_absent"], admitted, "{label}");
        if admitted {
            let d: Value = read(dest.join("draft.json"));
            let m = d["draft"]["items"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|v| v["modifiers"]["members"].as_array().unwrap())
                .find(|v| v["id"] == line["modifiers"][0])
                .unwrap();
            assert_eq!(m["definition"]["value"]["key"], "def.00000000000030ca");
            assert_eq!(
                m["rolls"]["completion"]["code"],
                "modifier-roll-schema-partial"
            );
        } else {
            assert_eq!(row["attribution"]["layout"]["status"], "pending");
        }
    }
    cases.len()
}
fn item_id(d: &Value, index: usize) -> Value {
    d["draft"]["items"]["members"][index]["id"].clone()
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: impl AsRef<Path>, v: &impl Serialize) {
    fs::write(p, serde_json::to_vec(v).unwrap()).unwrap();
}
fn run(input: &Path, output: &Path) -> Value {
    let o = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}
#[test]
fn one_data_driven_family_has_exact_typed_dependencies_and_conservative_source_domain() {
    let e = family::extension();
    let b = family::bindings();
    assert_eq!(e.schema.len(), 24);
    assert_eq!(e.owners.len(), 1);
    assert_eq!(e.owners[0].programs.members.len(), 4);
    assert!(!e.owners[0].programs.is_complete());
    assert_eq!(b.templates.len(), 6);
    assert_eq!(b.properties.len(), 21);
    assert_eq!(b.amount.declaration, b.corrupted_base.declaration);
    assert_eq!(b.unit.key().as_str(), "def.000000000000295a");
    assert_eq!(b.factor_unit.key().as_str(), "def.0000000000000001");
    assert_eq!(b.effective.key().as_str(), "def.000000000000295b");
    assert_eq!(b.minion_level.key().as_str(), "def.00000000000030ab");
    assert_eq!(family::dependency_definitions().len(), 24);
    let authoring = family::authoring();
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        authoring["source_manifest_sha256"]
    );
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(manifest["upstream_revision"], authoring["source_revision"]);
    assert_eq!(authoring["source_files"].as_array().unwrap().len(), 6);
    for pin in authoring["source_files"].as_array().unwrap() {
        assert!(manifest["files"].as_array().unwrap().contains(pin));
    }
    assert_eq!(family::item_rule().id.as_str(), "fixed-global-minion-level");
    let condition = serde_json::to_value(family::source_condition()).unwrap();
    assert_eq!(
        condition["all"],
        json!([{"kind":"no_source_scaling_tags"},{"kind":"initial_scaling_is_one"},{"kind":"no_generated_buff_members"},{"kind":"unsigned_integer_capture","value":{"capture":"amount","min":0,"max":1000000}}])
    );
}
fn preservation(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) {
    let b = family::bindings();
    preservation::check(
        prior,
        next,
        preservation::FamilyChange {
            modifier: &b.modifier,
            templates: &b.templates,
            dependencies: &family::dependency_definitions(),
            extension: &family::extension(),
            last_issued: 0x30e1,
            rule: &family::item_rule(),
            condition: &family::source_condition(),
            default: &family::source_default(),
        },
    );
}
#[test]
#[ignore = "requires explicit checked real empty-passive-socket predecessor"]
fn real_family_publication_preserves_release_and_reports_original_source_admission() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_GLOBAL_MINION_PRIOR").expect("explicit predecessor"),
    );
    let before = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    preservation(&prior, &next);
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_GLOBAL_MINION_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("publication"));
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    for (src, dst) in [(&out.join("endpoint.json"), &package), (&package, &rebuilt)] {
        assert_eq!(run(src, dst), serde_json::to_value(next.receipt()).unwrap());
    }
    assert_eq!(release::inventory(&package), release::inventory(&rebuilt));
    let mut summaries = vec![];
    for case in 1..=5 {
        let q = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(prior_path.join(&q)).unwrap(),
            fs::read(package.join(q)).unwrap()
        );
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&prior_path, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        summaries.push(compare_original(
            case,
            &fs::read(&xml).unwrap(),
            &old,
            &new,
            &out,
        ));
    }
    let probes = probes(&package, &out);
    assert_eq!(before, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"definitions":next.receipt().definitions,"registry":next.receipt().registry,"queries":110,"provenance":11,"originals":summaries,"probes":probes,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
