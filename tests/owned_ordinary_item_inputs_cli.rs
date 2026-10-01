//! Source-preserving physical input conversion; numerical coverage stays open.
#[path = "support/owned_ordinary_item_inputs.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use identity::{correspond, relocate};
use poe_optimizer_core::owned_schema::{
    DefinitionDescriptor, SchemaClosure, SchemaState, SlotDescriptor, SlotPresence,
};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry, owned_release::assemble_owned_release,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
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
    fs::write(p, serde_json::to_vec(v).unwrap()).unwrap()
}
fn publish(input: &Path, out: &Path) -> Value {
    let r = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(out)
        .output()
        .unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    serde_json::from_slice(&r.stdout).unwrap()
}
#[test]
fn authored_raw_inputs_have_exact_types_without_new_runtime_channels_or_coverage() {
    let e = family::extension();
    let bindings = family::bindings();
    assert_eq!(e.schema.len(), 15);
    assert_eq!(bindings.len(), 2);
    assert!(e.owners.is_empty() && e.tables.is_empty() && e.receivers.is_empty());
    assert!(e.operations_version.is_none());
    assert_eq!(
        family::rarities()
            .iter()
            .map(|r| r.token.as_str())
            .collect::<Vec<_>>(),
        ["NORMAL", "MAGIC", "RARE", "UNIQUE", "RELIC"]
    );
    for b in bindings {
        assert_eq!(b.header_inputs.len(), 2);
        assert_eq!(b.header_inputs[0].rule.as_str(), "rarity");
        assert_eq!(b.header_inputs[1].rule.as_str(), "level-requirement");
        let slots = [
            &b.header_inputs[0].slot,
            &b.corruption_slot,
            &b.header_inputs[1].slot,
            &b.capacity_slot,
        ];
        for (i, slot) in slots.iter().enumerate() {
            let d = e
                .schema
                .iter()
                .find_map(|d| match d {
                    SchemaExtensionEntry::Slot(SlotDescriptor::Parameter(d)) if &d.id == *slot => {
                        Some(d)
                    }
                    _ => None,
                })
                .unwrap();
            let SchemaState::Known(s) = &d.schema else {
                panic!()
            };
            assert_eq!(
                s.presence,
                if i == 2 {
                    SlotPresence::OptionalOnce
                } else {
                    SlotPresence::RequiredOnce
                }
            );
            assert_eq!(
                serde_json::to_value(&s.sites).unwrap(),
                json!(["item_parameter"])
            );
        }
        let d = e
            .schema
            .iter()
            .find_map(|d| match d {
                SchemaExtensionEntry::Definition(DefinitionDescriptor::ItemTemplate(d))
                    if d.id == b.template =>
                {
                    Some(d)
                }
                _ => None,
            })
            .unwrap();
        let SchemaState::Known(s) = &d.schema else {
            panic!()
        };
        assert_eq!(s.declarations.parameters.members.len(), 6);
        assert!(matches!(
            s.declarations.parameters.closure,
            SchemaClosure::Partial { .. }
        ));
    }
    let a = family::authoring();
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        a["source_manifest_sha256"]
    );
    let m: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], m["upstream_revision"]);
    assert_eq!(a["source_files"].as_array().unwrap().len(), 7);
    for pin in a["source_files"].as_array().unwrap() {
        assert!(m["files"].as_array().unwrap().contains(pin));
    }
    assert!(root().join(a["source_test"].as_str().unwrap()).is_file());
}
fn source_item(sidecar: &Value, ordinal: u64) -> &Value {
    let rows: Vec<_> = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["source"]["ordinal"] == ordinal)
        .collect();
    assert_eq!(rows.len(), 1);
    let links: Vec<_> = rows[0]["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "item")
        .collect();
    assert_eq!(links.len(), 1);
    &links[0]["value"]
}
fn item<'a>(draft: &'a Value, id: &Value) -> &'a Value {
    draft["draft"]["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| &r["id"] == id)
        .unwrap()
}
fn raw_assignments(template: &Value, requirement: u64, capacity: u64) -> Vec<Value> {
    let b = family::bindings()
        .into_iter()
        .find(|b| serde_json::to_value(&b.template).unwrap() == *template)
        .unwrap();
    let rare = family::rarities()
        .into_iter()
        .find(|v| v.token == "RARE")
        .unwrap()
        .value;
    [(&b.header_inputs[0].slot,json!({"kind":"option","value":rare})),(&b.corruption_slot,json!({"kind":"boolean","value":false})),(&b.header_inputs[1].slot,json!({"kind":"integer","value":requirement})),(&b.capacity_slot,json!({"kind":"integer","value":capacity}))].into_iter().map(|(slot,value)|json!({"slot":{"kind":"known","value":slot},"value":{"kind":"known","value":value}})).collect()
}
fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    for v in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(v);
    }
    assert_eq!(sa["schema_version"], 12);
    assert_eq!(sb["schema_version"], 13);
    sb["schema_version"] = sa["schema_version"].clone();
    let targets: Vec<_> = if case == 5 {
        [(572, 0, 4), (574, 5, 3)]
            .map(|(o, l, c)| {
                (
                    source_item(&sa, o).clone(),
                    source_item(&sb, o).clone(),
                    l,
                    c,
                )
            })
            .to_vec()
    } else {
        vec![]
    };
    let pa = a["draft"]["allocator"].clone();
    let na = b["draft"]["allocator"].clone();
    let issued = |a: &Value| u64::from_str_radix(a["last_issued"].as_str().unwrap(), 16).unwrap();
    assert_eq!(issued(&pa) - issued(&na), if case == 5 { 2 } else { 0 });
    b["draft"]["allocator"] = pa.clone();
    let mut retired = BTreeSet::new();
    let aa = a["draft"]["items"]["members"].as_array_mut().unwrap();
    let bb = b["draft"]["items"]["members"].as_array_mut().unwrap();
    assert_eq!(aa.len(), bb.len());
    for (x, y) in aa.iter_mut().zip(bb) {
        if let Some((_, new_id, l, c)) = targets.iter().find(|(old_id, _, _, _)| old_id == &x["id"])
        {
            assert_eq!(&y["id"], new_id);
            assert_eq!(
                x["parameters"]["completion"]["code"],
                "item-parameters-not-converted"
            );
            assert!(
                retired.insert(
                    x["parameters"]["completion"]["id"]["local"]
                        .as_str()
                        .unwrap()
                        .to_owned()
                )
            );
            assert_eq!(y["parameters"]["completion"], json!({"kind":"complete"}));
            let expected = raw_assignments(&x["template"]["value"], *l, *c);
            let members = y["parameters"]["members"].as_array_mut().unwrap();
            assert_eq!(members.len(), 6);
            for value in expected {
                let at = members
                    .iter()
                    .position(|v| v == &value)
                    .expect("exact raw input assignment");
                members.remove(at);
            }
            assert_eq!(members.len(), 2);
            assert_eq!(*members, *x["parameters"]["members"].as_array().unwrap());
            for v in [x, y] {
                v["parameters"]
                    .as_object_mut()
                    .unwrap()
                    .remove("completion");
            }
        }
    }
    assert_eq!(retired.len(), if case == 5 { 2 } else { 0 });
    let mut ids = BTreeMap::new();
    correspond(
        &a,
        &mut b,
        &mut ids,
        "only eight raw assignments and two input obligations change",
    );
    if case != 5 {
        assert!(
            ids.iter()
                .all(|(k, v)| serde_json::from_str::<Value>(k).unwrap() == *v),
            "unaffected IDs stay exact"
        );
    }
    let mut removed = 0;
    for row in sa["origins"].as_array_mut().unwrap() {
        row["links"].as_array_mut().unwrap().retain(|r| {
            let remove = r["kind"] == "issue"
                && r["value"]["local"]
                    .as_str()
                    .is_some_and(|id| retired.contains(id));
            removed += usize::from(remove);
            !remove
        });
    }
    assert_eq!(removed, retired.len());
    let old_rows = sa["item_texts"].as_array().unwrap();
    let new_rows = sb["item_texts"].as_array_mut().unwrap();
    assert_eq!(old_rows.len(), new_rows.len());
    let mut evidence = 0;
    for (x, y) in old_rows.iter().zip(new_rows) {
        assert_eq!(x["source"], y["source"]);
        y["attribution"]["policy"] = x["attribution"]["policy"].clone();
        y["attribution"]["item_lines"] = x["attribution"]["item_lines"].clone();
        if let Some(proof) = y.as_object_mut().unwrap().remove("parameter_inputs") {
            assert_eq!(case, 5);
            let ordinal = x["source"]["ordinal"].as_u64().unwrap();
            assert!([572, 574].contains(&ordinal));
            assert_eq!(proof.as_array().unwrap().len(), 4);
            let (requirement, capacity) = if ordinal == 572 { (0, 4) } else { (5, 3) };
            let template = &x["attribution"]["default_scope"]["template"];
            let binding = family::bindings()
                .into_iter()
                .find(|b| serde_json::to_value(&b.template).unwrap() == *template)
                .unwrap();
            for expected in raw_assignments(template, requirement, capacity) {
                let rows: Vec<_> = proof
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|r| {
                        r["slot"] == expected["slot"]["value"]
                            && r["value"] == expected["value"]["value"]
                    })
                    .collect();
                assert_eq!(
                    rows.len(),
                    1,
                    "each proven value has one exact evidence row"
                );
                let origin = &rows[0]["origin"];
                if rows[0]["slot"] == serde_json::to_value(&binding.corruption_slot).unwrap() {
                    assert_eq!(*origin, json!({"kind":"fresh_uncorrupted"}));
                } else {
                    let line = x["lines"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|v| v["index"] == origin["line"])
                        .unwrap();
                    if rows[0]["slot"] == serde_json::to_value(&binding.capacity_slot).unwrap() {
                        assert_eq!(origin["kind"], "empty_socket_capacity");
                        assert_eq!(
                            line["text"],
                            format!("Sockets:{}", " S".repeat(capacity as usize))
                        );
                    } else {
                        let header = binding
                            .header_inputs
                            .iter()
                            .find(|h| serde_json::to_value(&h.slot).unwrap() == rows[0]["slot"])
                            .unwrap();
                        assert_eq!(origin["kind"], "header");
                        assert_eq!(origin["rule"], header.rule.as_str());
                        assert_eq!(origin["capture"], header.capture.as_str());
                        assert_eq!(
                            line["text"],
                            if header.rule.as_str() == "rarity" {
                                "Rarity: RARE".into()
                            } else {
                                format!("LevelReq: {requirement}")
                            }
                        );
                    }
                }
            }
            evidence += 1;
        }
    }
    assert_eq!(evidence, targets.len());
    for field in [
        "draft",
        "definitions",
        "registry",
        "mapping",
        "policy",
        "item_policy",
        "item_source_policy",
        "skill_roles",
        "reward_policy",
        "tree_policy",
    ] {
        sb[field] = sa[field].clone();
    }
    assert_eq!(sa["allocator_after"], pa);
    assert_eq!(sb["allocator_after"], na);
    sb["allocator_after"] = sa["allocator_after"].clone();
    correspond(
        &sa,
        &mut sb,
        &mut ids,
        "source text/defaults/attribution and retained links stay exact",
    );
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
    let mut x = before["finalization"]["issues"].clone();
    let mut y = after["finalization"]["issues"].clone();
    selected::canonical(&mut x);
    selected::canonical(&mut y);
    relocate(&mut y, &ids);
    let count = x.as_array().unwrap().len();
    x.as_array_mut()
        .unwrap()
        .retain(|r| !retired.contains(r["id"]["local"].as_str().unwrap()));
    assert_eq!(x, y);
    assert_eq!(count - y.as_array().unwrap().len(), retired.len());
    if case == 5 {
        assert_eq!(count, 144);
        assert_eq!(y.as_array().unwrap().len(), 142);
    }
    let mut first = selected::selection(xml, old);
    let mut second = selected::selection(xml, new);
    selected::canonical(&mut first);
    selected::canonical(&mut second);
    relocate(&mut second, &ids);
    assert_eq!(first, second);
    json!({"original":case,"selected_before":count,"selected_after":y.as_array().unwrap().len(),"retired":retired,"raw_assignments":4*evidence,"evidence_rows":evidence,"allocator_before":pa,"allocator_after":na,"selected_issue_summary":after["selected_issue_summary"]})
}
fn probes(package: &Path, out: &Path) -> usize {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let start = original.find("<Item id=\"19\">").unwrap();
    let end = start + original[start..].find("</Item>").unwrap() + 7;
    let raw = &original[start..end];
    let cases = [
        ("missing-level", raw.replace("LevelReq: 0\n", "")),
        (
            "duplicate-level",
            raw.replace("LevelReq: 0", "LevelReq: 0\nLevelReq: 77"),
        ),
        (
            "alternate-level",
            raw.replace("LevelReq: 0", "Requires Level: 0"),
        ),
        ("imported-level", raw.replace("LevelReq: 0", "Level: 0")),
        (
            "malformed-level",
            raw.replace("LevelReq: 0", "LevelReq: NaN"),
        ),
        (
            "corrupted",
            raw.replace("Implicits: 0", "Corrupted\nImplicits: 0"),
        ),
        (
            "duplicate-rarity",
            raw.replace("Rarity: RARE", "Rarity: RARE\nRarity: RARE"),
        ),
        (
            "unknown-prefix",
            raw.replace("Quality: 20", "Unknown input state\nQuality: 20"),
        ),
        (
            "occupied-rune",
            raw.replacen("Rune: None", "Rune: Lesser Iron Rune", 1),
        ),
        (
            "extra-member",
            raw.replace(
                "+17 to maximum Life",
                "+17 to maximum Life\n+3 to maximum Life",
            ),
        ),
    ];
    for (label, changed) in &cases {
        assert_ne!(changed, raw);
        let xml = format!("{}{}{}", &original[..start], changed, &original[end..]);
        let p = out.join(format!("probe-{label}.xml"));
        fs::write(&p, xml).unwrap();
        let dest = out.join(format!("probe-{label}"));
        release::normalize(package, &p, 5, &dest);
        let d: Value = read(dest.join("draft.json"));
        let s: Value = read(dest.join("sidecar.json"));
        let i = item(&d, source_item(&s, 572));
        assert_eq!(i["parameters"]["completion"]["kind"], "pending", "{label}");
        assert!(
            s["item_texts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["source"]["ordinal"] == 572)
                .unwrap()
                .get("parameter_inputs")
                .is_none(),
            "{label}"
        );
    }
    let changed = raw.replace("LevelReq: 0", "LevelReq: 77");
    assert_ne!(changed, raw);
    let p = out.join("probe-explicit-77.xml");
    fs::write(
        &p,
        format!("{}{}{}", &original[..start], changed, &original[end..]),
    )
    .unwrap();
    let dest = out.join("probe-explicit-77");
    release::normalize(package, &p, 5, &dest);
    let d: Value = read(dest.join("draft.json"));
    let s: Value = read(dest.join("sidecar.json"));
    let i = item(&d, source_item(&s, 572));
    assert_eq!(i["parameters"]["completion"], json!({"kind":"complete"}));
    for expected in raw_assignments(&i["template"]["value"], 77, 4) {
        assert!(
            i["parameters"]["members"]
                .as_array()
                .unwrap()
                .contains(&expected)
        );
    }
    cases.len() + 1
}
#[test]
#[ignore = "requires the exact checked ordinary-catalyst predecessor"]
fn real_raw_item_inputs_preserve_all_requests_and_independent_coverage() {
    let p = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ORDINARY_INPUTS_PRIOR").expect("explicit prior"),
    );
    let before = release::inventory(&p);
    let prior = release::load(&p);
    let next = family::stage(&prior);
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_ORDINARY_INPUTS_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("publication"));
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    for (src, dst) in [(&out.join("endpoint.json"), &package), (&package, &rebuilt)] {
        assert_eq!(
            publish(src, dst),
            serde_json::to_value(next.receipt()).unwrap()
        );
    }
    let inv = release::inventory(&package);
    assert_eq!(inv, release::inventory(&rebuilt));
    for (name, hash) in &before {
        if name.starts_with("queries-") {
            assert_eq!(inv.get(name), Some(hash));
        }
    }
    for field in ["definitions", "item_lines", "item_source"] {
        let mut bad = serde_json::to_value(next.input()).unwrap();
        let old = match field {
            "definitions" => serde_json::to_value(&prior.receipt().definitions).unwrap(),
            "item_lines" => serde_json::to_value(prior.receipt().items).unwrap(),
            _ => serde_json::to_value(prior.receipt().item_source).unwrap(),
        };
        bad["normalization"]["item_parameter_inputs"][field] = old;
        assert!(
            assemble_owned_release(serde_json::from_value(bad).unwrap(), Default::default())
                .is_err(),
            "stale {field}"
        );
    }
    let mut reports = vec![];
    for case in 1..=5 {
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let a = out.join(format!("prior-original-{case:02}"));
        let b = out.join(format!("original-{case:02}"));
        release::normalize(&p, &xml, case, &a);
        release::normalize(&package, &xml, case, &b);
        reports.push(compare(case, &fs::read(xml).unwrap(), &a, &b, &out));
    }
    let count = probes(&package, &out);
    assert_eq!(before, release::inventory(&p));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"definitions":next.receipt().definitions,"registry":next.receipt().registry,"prior_provenance":prior.input().provenance.len(),"final_provenance":next.input().provenance.len(),"queries":next.receipt().query_rows,"originals":reports,"probes":count,"stale_binding_rejections":3,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
