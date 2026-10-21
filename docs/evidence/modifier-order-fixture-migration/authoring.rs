//! One-off authoring only. Never linked by a production or maintained test target.
//! Reauthor three historical typed evidence snapshots without re-importing source.
use flate2::{Compression, GzBuilder, read::GzDecoder};
use poe_optimizer_core::owned_draft::{
    DraftLimits, OWNED_DRAFT_SCHEMA_VERSION, decode_draft, encode_draft,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits, SourceRole},
    decode_build,
    owned_source::{SourceContentEvidence, SourceEvidenceLimits, SourceProjectEvidence},
    source_xml::PobContentEntry,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const BASE: &str = "data/owned/poe2/3887ae68";
const MANA: &str = "mana-supplier-sources";
const SELECTED: &str = "self-flat-selected-bindings";
const OLD_CODE: &str = "item-modifier-order-not-converted";
const CONTRIBUTIONS: &str = "item-modifier-contribution-order-not-converted";
const TRANSFORMS: &str = "item-modifier-numeric-transform-order-not-converted";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn json_bytes(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    bytes
}
fn read(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn inflate(bytes: &[u8]) -> Vec<u8> {
    let mut raw = vec![];
    GzDecoder::new(bytes)
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut raw)
        .unwrap();
    assert!(raw.len() <= 2 * 1024 * 1024);
    raw
}
fn compress(bytes: &[u8]) -> Vec<u8> {
    let mut writer = GzBuilder::new().mtime(0).write(vec![], Compression::best());
    writer.write_all(bytes).unwrap();
    writer.finish().unwrap()
}

struct Spec {
    family: &'static str,
    names: [&'static str; 3],
    draft_sha: &'static str,
    authority: bool,
}
struct Reauthored {
    family: &'static str,
    old: [Value; 3],
    new: [Value; 3],
    issues: Vec<Value>,
    origins: Vec<usize>,
    receipt: Value,
    report_sha: String,
}

fn prove_orders(
    item: &Value,
    origin: &Value,
    sidecar: &Value,
    evidence: &SourceProjectEvidence<'_>,
) -> Value {
    assert_eq!(item["modifiers"]["completion"], json!({"kind":"complete"}));
    let texts: Vec<_> = sidecar["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["source"] == origin["source"])
        .collect();
    assert_eq!(texts.len(), 1);
    let text = texts[0];
    assert_eq!(text["attribution"]["layout"], json!({"status":"proven"}));
    assert_eq!(text["attribution"]["item"], origin["source"]);
    assert_eq!(text["content_entry"], text["attribution"]["content_entry"]);
    let ordinal = origin["source"]["ordinal"].as_u64().unwrap() as usize;
    let SourceContentEvidence::Available(content) = evidence.rows()[ordinal].content() else {
        panic!("available original Item content")
    };
    let PobContentEntry::Text {
        text: original_text,
        ..
    } = &content.consumed()[text["content_entry"].as_u64().unwrap() as usize]
    else {
        panic!("exact Item text entry")
    };
    let mut rows = vec![];
    let mut seen = BTreeSet::new();
    let mut member_lines = BTreeSet::new();
    for line in text["lines"].as_array().unwrap() {
        let ids = line["modifiers"].as_array().unwrap();
        if ids.is_empty() {
            continue;
        }
        assert_eq!(line["outcome"]["kind"], "known");
        let attributed: Vec<_> = text["attribution"]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["index"] == line["index"])
            .collect();
        assert_eq!(attributed.len(), 1);
        let attr = attributed[0];
        assert_eq!(attr["blockers"], json!([]));
        assert_eq!(attr["raw"], line["text"]);
        let start = attr["decoded_span"]["start"].as_u64().unwrap() as usize;
        let end = attr["decoded_span"]["end"].as_u64().unwrap() as usize;
        assert_eq!(
            original_text.get(start..end).unwrap(),
            attr["raw"].as_str().unwrap()
        );
        assert_eq!(attr["member"]["line"], line["index"]);
        assert!(member_lines.insert(line["index"].as_u64().unwrap()));
        let category = match attr["member"]["category"].as_str().unwrap() {
            "implicit" => 0u64,
            "explicit" => 1u64,
            other => panic!("not an admitted category in these historical fixtures: {other}"),
        };
        let category_ordinal = attr["member"]["ordinal"].as_u64().unwrap();
        let emissions: Vec<_> = line["outcome"]["value"]["emissions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["kind"] == "modifier")
            .collect();
        assert_eq!(emissions.len(), ids.len());
        for (position, (id, emission)) in ids.iter().zip(emissions).enumerate() {
            assert!(seen.insert(serde_json::to_string(id).unwrap()));
            assert!(
                origin["links"]
                    .as_array()
                    .unwrap()
                    .contains(&json!({"kind":"modifier","value":id}))
            );
            let records: Vec<_> = item["modifiers"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["id"] == *id)
                .collect();
            assert_eq!(records.len(), 1);
            assert_eq!(
                records[0]["definition"],
                json!({"kind":"known","value":emission["value"]["definition"]})
            );
            rows.push((category,category_ordinal,position,start,id.clone(),json!({
                "modifier":id,"definition":emission["value"]["definition"],"line":line["index"],
                "decoded_span":attr["decoded_span"],"raw":attr["raw"],"category":attr["member"]["category"],
                "category_ordinal":category_ordinal,"emission_position":position
            })));
        }
    }
    let expected: BTreeSet<_> = item["modifiers"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| serde_json::to_string(&r["id"]).unwrap())
        .collect();
    assert_eq!(seen, expected, "complete exact modifier occurrence census");
    let attributed: BTreeSet<_> = text["attribution"]["lines"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| !r["member"].is_null())
        .map(|r| r["index"].as_u64().unwrap())
        .collect();
    assert_eq!(
        member_lines, attributed,
        "every attributed modifier member is accounted"
    );
    let mut contributions = rows.clone();
    contributions
        .sort_by_key(|(category, ordinal, position, _, _, _)| (*category, *ordinal, *position));
    let mut transforms = rows.clone();
    transforms.sort_by_key(|(_, _, position, start, _, _)| (*start, *position));
    json!({"item":item["id"],"source":origin["source"],"content_entry":text["content_entry"],
        "contribution_evidence":"complete-category-ordinal-emission-order",
        "numeric_transform_evidence":"original-decoded-source-span-and-emission-order",
        "contributions":contributions.iter().map(|r|r.4.clone()).collect::<Vec<_>>(),
        "numeric_transforms":transforms.iter().map(|r|r.4.clone()).collect::<Vec<_>>(),
        "rows":rows.iter().map(|r|r.5.clone()).collect::<Vec<_>>()})
}

fn snapshot(
    spec: Spec,
    root: &Path,
    archive: &Path,
    writes: &mut BTreeMap<PathBuf, Vec<u8>>,
) -> Reauthored {
    let dir = root.join(BASE).join(spec.family);
    let receipt_name = if spec.authority {
        "configuration-reward-authority.json"
    } else {
        "inputs.json"
    };
    let old_receipt = read(&dir.join(receipt_name));
    assert!(old_receipt.get("modifier_order_reauthoring").is_none());
    let mut receipt = old_receipt.clone();
    let mut raw = vec![];
    let mut predecessors = vec![];
    for (index, name) in spec.names.iter().enumerate() {
        let gzip = fs::read(dir.join(name)).unwrap();
        let bytes = inflate(&gzip);
        let pin = if spec.authority {
            &old_receipt["files"][["draft", "sidecar", "report"][index]]
        } else {
            let role = name.strip_suffix(".gz").unwrap();
            let pins: Vec<_> = old_receipt["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["name"] == role)
                .collect();
            assert_eq!(pins.len(), 1);
            pins[0]
        };
        assert_eq!(pin["sha256"], hash(&bytes));
        assert_eq!(pin["compressed_sha256"], hash(&gzip));
        assert_eq!(pin["bytes"], bytes.len());
        if spec.authority {
            assert_eq!(pin["compressed_bytes"], gzip.len());
        }
        let backup = archive.join(spec.family).join(name);
        fs::create_dir_all(backup.parent().unwrap()).unwrap();
        fs::write(&backup, &gzip).unwrap();
        assert_eq!(fs::read(&backup).unwrap(), gzip);
        predecessors.push(json!({
            "file":name,"bytes":bytes.len(),"sha256":hash(&bytes),
            "compressed_bytes":gzip.len(),"compressed_sha256":hash(&gzip)
        }));
        raw.push(bytes);
    }
    assert_eq!(hash(&raw[0]), spec.draft_sha, "exact historical draft");
    let old: [Value; 3] = raw
        .iter()
        .map(|b| serde_json::from_slice(b).unwrap())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    assert_eq!(old[0]["schema_version"], 5);
    assert_eq!(OWNED_DRAFT_SCHEMA_VERSION, 5);
    assert_eq!(old[1]["draft"], old[2]["draft_digest"]);
    assert_eq!(old[1]["allocator_after"], old[0]["draft"]["allocator"]);
    assert_eq!(old[2]["allocator_after"], old[0]["draft"]["allocator"]);
    assert_eq!(old[2]["sidecar_sha256"], hash(&raw[1]));
    assert_eq!(old[2]["sidecar_bytes"], raw[1].len());

    let mut new = old.clone();
    let lineage = old[0]["draft"]["allocator"]["lineage"].clone();
    let xml = fs::read(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml")).unwrap();
    assert_eq!(old[1]["source_sha256"], hash(&xml));
    assert_eq!(old[1]["source_bytes"], xml.len());
    let source = ImportedBuildInstance::from_decoded(
        decode_build(&xml).unwrap(),
        serde_json::from_value(lineage.clone()).unwrap(),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let source_origins = old[1]["origins"].as_array().unwrap();
    assert_eq!(source_origins.len(), evidence.rows().len());
    let mut source_ordinals = BTreeSet::new();
    for origin in source_origins {
        let ordinal = usize::try_from(origin["source"]["ordinal"].as_u64().unwrap()).unwrap();
        let occurrence = evidence.rows().get(ordinal).unwrap().occurrence();
        assert_eq!(
            origin["source"],
            serde_json::to_value(occurrence.id()).unwrap(),
            "the complete source identity must match the retained XML"
        );
        assert!(source_ordinals.insert(ordinal), "unique source origin");
    }
    let last = u64::from_str_radix(
        old[0]["draft"]["allocator"]["last_issued"]
            .as_str()
            .unwrap(),
        16,
    )
    .unwrap();
    let mut issues = vec![];
    let mut origins = vec![];
    let mut known_proofs = vec![];
    let items = old[0]["draft"]["items"]["members"].as_array().unwrap();
    assert_eq!(items.len(), 28);
    for (index, item) in items.iter().enumerate() {
        let old_order = &item["modifier_order"];
        assert!(item.get("modifier_orders").is_none());
        let matches: Vec<_> = old[1]["origins"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .filter(|(_, origin)| {
                let ordinal = origin["source"]["ordinal"].as_u64().unwrap() as usize;
                let occurrence = evidence.rows()[ordinal].occurrence();
                matches!(occurrence.role(), SourceRole::Item { .. })
                    && occurrence.name() == "Item"
                    && !occurrence.has_namespace_context()
                    && origin["links"]
                        .as_array()
                        .unwrap()
                        .contains(&json!({"kind":"item","value":item["id"]}))
            })
            .map(|(i, _)| i)
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "one exact Item origin, never a Slot origin"
        );
        let origin_index = matches[0];
        let origin = &old[1]["origins"][origin_index];
        let numeric = match old_order["kind"].as_str().unwrap() {
            "known" => {
                let proof = prove_orders(item, origin, &old[1], &evidence);
                assert_eq!(proof["contributions"], old_order["value"]);
                let numeric = json!({"kind":"known","value":proof["numeric_transforms"]});
                known_proofs.push(proof);
                numeric
            }
            "pending" => {
                assert_eq!(old_order["code"], OLD_CODE);
                assert_eq!(old_order["candidates"], json!([]));
                assert!(
                    origin["links"]
                        .as_array()
                        .unwrap()
                        .contains(&json!({"kind":"issue","value":old_order["id"]}))
                );
                let issue = json!({"lineage":lineage,"local":format!("{:016x}",last.checked_add(issues.len() as u64 + 1).unwrap())});
                assert_eq!(origin["disposition"], json!({"kind":"contributes"}));
                new[1]["origins"][origin_index]["links"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"kind":"issue","value":issue}));
                issues.push(issue.clone());
                origins.push(origin_index);
                json!({"kind":"pending","id":issue,"code":TRANSFORMS,"candidates":[]})
            }
            other => panic!("unexpected historical order kind {other}"),
        };
        let mut contributions = old_order.clone();
        if contributions["kind"] == "pending" {
            contributions["code"] = json!(CONTRIBUTIONS);
        }
        let current = new[0]["draft"]["items"]["members"][index]
            .as_object_mut()
            .unwrap();
        assert_eq!(current.remove("modifier_order").unwrap(), *old_order);
        assert!(
            current
                .insert(
                    "modifier_orders".into(),
                    json!({"contributions":contributions,"numeric_transforms":numeric})
                )
                .is_none()
        );
    }
    assert_eq!(issues.len(), 20);
    assert_eq!(known_proofs.len(), 8);
    new[0]["draft"]["allocator"]["last_issued"] = json!(format!(
        "{:016x}",
        last.checked_add(issues.len() as u64).unwrap()
    ));
    let typed = decode_draft(
        &serde_json::to_vec(&new[0]).unwrap(),
        DraftLimits::default(),
    )
    .expect("current strict typed draft");
    let encoded = encode_draft(&typed, DraftLimits::default()).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&encoded).unwrap(),
        new[0],
        "the current serializer must not silently alter other authored records"
    );
    let digest = serde_json::to_value(
        typed
            .digest(DraftLimits::default().input.max_wire_bytes)
            .unwrap(),
    )
    .unwrap();
    let validation = typed.validate_limits(DraftLimits::default()).unwrap();
    let mut codes = BTreeMap::<String, usize>::new();
    let mut live = BTreeSet::new();
    for issue in &validation.issues {
        *codes.entry(issue.code.as_str().into()).or_default() += 1;
        assert!(live.insert(serde_json::to_string(&issue.id).unwrap()));
    }
    let mut expected_codes = old[2]["issue_codes"].clone();
    assert_eq!(
        expected_codes.as_object_mut().unwrap().remove(OLD_CODE),
        Some(json!(20))
    );
    assert!(expected_codes.get(CONTRIBUTIONS).is_none());
    assert!(expected_codes.get(TRANSFORMS).is_none());
    expected_codes[CONTRIBUTIONS] = json!(issues.len());
    expected_codes[TRANSFORMS] = json!(issues.len());
    assert_eq!(serde_json::to_value(&codes).unwrap(), expected_codes);
    assert_eq!(
        validation.issues.len() as u64,
        old[2]["issue_count"].as_u64().unwrap() + issues.len() as u64
    );
    // All previous issue links remain; the new explicit obligations have exact
    // origins. Check the same bidirectional live/linked invariant as Import.
    let mut linked = BTreeSet::new();
    for origin in new[1]["origins"].as_array().unwrap() {
        for link in origin["links"].as_array().unwrap() {
            if link["kind"] == "issue" {
                assert_eq!(origin["disposition"], json!({"kind":"contributes"}));
                let id = serde_json::to_string(&link["value"]).unwrap();
                assert!(live.contains(&id));
                linked.insert(id);
            }
        }
    }
    assert_eq!(linked, live);
    new[1]["draft"] = digest.clone();
    new[1]["allocator_after"] = new[0]["draft"]["allocator"].clone();
    // Keep the maintained compact evidence encoding within existing reader
    // bounds; pretty-printing this origin census would exceed two MiB.
    let new_sidecar = serde_json::to_vec(&new[1]).unwrap();
    assert!(new_sidecar.len() <= 2 * 1024 * 1024);
    new[2]["draft_digest"] = digest;
    new[2]["allocator_after"] = new[0]["draft"]["allocator"].clone();
    new[2]["sidecar_sha256"] = json!(hash(&new_sidecar));
    new[2]["sidecar_bytes"] = json!(new_sidecar.len());
    new[2]["issue_count"] = json!(validation.issues.len());
    new[2]["issue_codes"] = serde_json::to_value(codes).unwrap();

    // Exact inverse: removing only the declared current-format edits reconstructs
    // every previous source selection, issue, typed value and receipt field.
    let mut inverse = new.clone();
    inverse[0]["draft"]["allocator"] = old[0]["draft"]["allocator"].clone();
    for (index, item) in items.iter().enumerate() {
        let obj = inverse[0]["draft"]["items"]["members"][index]
            .as_object_mut()
            .unwrap();
        let current = obj.remove("modifier_orders").unwrap();
        let mut prior = current["contributions"].clone();
        if prior["kind"] == "pending" {
            assert_eq!(prior["code"], CONTRIBUTIONS);
            prior["code"] = json!(OLD_CODE);
        }
        assert_eq!(prior, item["modifier_order"]);
        assert!(obj.insert("modifier_order".into(), prior).is_none());
    }
    for (issue, origin) in issues.iter().zip(&origins) {
        assert_eq!(
            inverse[1]["origins"][*origin]["links"]
                .as_array_mut()
                .unwrap()
                .pop()
                .unwrap(),
            json!({"kind":"issue","value":issue})
        );
    }
    for field in ["draft", "allocator_after"] {
        inverse[1][field] = old[1][field].clone();
    }
    for field in [
        "draft_digest",
        "allocator_after",
        "sidecar_sha256",
        "sidecar_bytes",
        "issue_count",
        "issue_codes",
    ] {
        inverse[2][field] = old[2][field].clone();
    }
    assert_eq!(inverse, old, "full exact inverse across all three records");

    let current_bytes = [encoded, new_sidecar, json_bytes(&new[2])];
    let reader_limits = [2 * 1024 * 1024, 2 * 1024 * 1024, 8 * 1024];
    for (bytes, limit) in current_bytes.iter().zip(reader_limits) {
        assert!(
            bytes.len() <= limit,
            "current evidence must fit its unchanged bounded reader"
        );
    }
    let mut successors = vec![];
    for (index, name) in spec.names.iter().enumerate() {
        let bytes = &current_bytes[index];
        let gzip = compress(bytes);
        assert!(
            gzip.len() <= reader_limits[index],
            "compressed evidence must fit its unchanged bounded reader"
        );
        let current = json!({
            "file":name,"bytes":bytes.len(),"sha256":hash(bytes),
            "compressed_bytes":gzip.len(),"compressed_sha256":hash(&gzip)
        });
        successors.push(current.clone());
        let pin = if spec.authority {
            &mut receipt["files"][["draft", "sidecar", "report"][index]]
        } else {
            let role = name.strip_suffix(".gz").unwrap();
            receipt["files"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|pin| pin["name"] == role)
                .unwrap()
        };
        for field in ["bytes", "sha256", "compressed_sha256"] {
            pin[field] = current[field].clone();
        }
        if spec.authority {
            pin["compressed_bytes"] = current["compressed_bytes"].clone();
        }
        writes.insert(dir.join(name), gzip);
    }
    receipt["modifier_order_reauthoring"] = json!({
        "kind":"strict-current-format-evidence",
        "draft_schema_before":5,"draft_schema_after":5,
        "source_release_artifacts_reauthored":false,
        "source_selections_changed":false,"native_coverage_authorized":false,
        "prior_archives":predecessors,"current_archives":successors,
        "independent_known_order_proofs":known_proofs,
        "new_pending_numeric_transform_orders":issues,
        "exact_item_origin_indices":origins,
        "proof":"whole-draft-sidecar-report-exact-inverse",
        "authoring_source":"runs/modifier_order_evidence_authoring.rs",
        "authoring_sha256":hash(&fs::read(root.join("runs/modifier_order_evidence_authoring.rs")).unwrap())
    });
    let mut inverse_receipt = receipt.clone();
    inverse_receipt
        .as_object_mut()
        .unwrap()
        .remove("modifier_order_reauthoring")
        .unwrap();
    inverse_receipt["files"] = old_receipt["files"].clone();
    assert_eq!(
        inverse_receipt, old_receipt,
        "every predecessor receipt and prior migration proof is preserved"
    );
    writes.insert(dir.join(receipt_name), json_bytes(&receipt));
    Reauthored {
        family: spec.family,
        old,
        new,
        issues,
        origins,
        receipt,
        report_sha: hash(&current_bytes[2]),
    }
}

fn relineage(value: &mut Value, before: &Value, after: &Value) {
    match value {
        Value::Object(object) => {
            if let Some(lineage) = object.get_mut("lineage") {
                assert_eq!(lineage, before);
                *lineage = after.clone();
            }
            for child in object.values_mut() {
                relineage(child, before, after);
            }
        }
        Value::Array(array) => {
            for child in array {
                relineage(child, before, after);
            }
        }
        _ => {}
    }
}
fn replace_items(equipment: &mut Value, snapshot: &Reauthored) {
    for row in equipment.as_array_mut().unwrap() {
        let id = row["owned_item"]["id"].clone();
        let before: Vec<_> = snapshot.old[0]["draft"]["items"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["id"] == id)
            .collect();
        let after: Vec<_> = snapshot.new[0]["draft"]["items"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["id"] == id)
            .collect();
        assert_eq!(before.len(), 1);
        assert_eq!(after.len(), 1);
        assert_eq!(row["owned_item"], *before[0]);
        row["owned_item"] = after[0].clone();
    }
}
fn update_derived(root: &Path, data: &[Reauthored], writes: &mut BTreeMap<PathBuf, Vec<u8>>) {
    let selected = data.iter().find(|s| s.family == SELECTED).unwrap();
    let path = root.join(BASE).join(SELECTED).join("bindings.json");
    let old = read(&path);
    let mut new = old.clone();
    assert_eq!(new["publication"]["import_report"], selected.old[2]);
    new["publication"]["import_report"] = selected.new[2].clone();
    let links = |sidecar: &Value| -> Value {
        json!(
            sidecar["origins"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|origin| origin["links"].as_array().unwrap())
                .filter(|link| link["kind"] == "issue")
                .cloned()
                .collect::<Vec<_>>()
        )
    };
    assert_eq!(new["preserved_import_issue_links"], links(&selected.old[1]));
    new["preserved_import_issue_links"] = links(&selected.new[1]);
    replace_items(&mut new["equipment"], selected);
    let mut inverse = new.clone();
    for field in ["publication", "preserved_import_issue_links", "equipment"] {
        inverse[field] = old[field].clone();
    }
    assert_eq!(
        inverse, old,
        "all other selected mechanical/source fields remain exact"
    );
    writes.insert(path, json_bytes(&new));

    let mana = data
        .iter()
        .find(|s| {
            s.family == MANA && s.old[0]["draft"]["allocator"]["last_issued"] == "0000000000000681"
        })
        .unwrap();
    let path = root.join(BASE).join(MANA).join("review.json");
    let old = read(&path);
    let mut new = old.clone();
    assert_eq!(new["selected"]["draft_digest"], mana.old[2]["draft_digest"]);
    new["selected"]["draft_digest"] = mana.new[2]["draft_digest"].clone();
    replace_items(&mut new["selected"]["memberships"]["equipment"], mana);
    let mut inverse = new.clone();
    inverse["selected"]["draft_digest"] = old["selected"]["draft_digest"].clone();
    inverse["selected"]["memberships"]["equipment"] =
        old["selected"]["memberships"]["equipment"].clone();
    assert_eq!(
        inverse, old,
        "all numerical, rule and source review evidence remains exact"
    );
    writes.insert(path, json_bytes(&new));

    let path = root
        .join(BASE)
        .join("player-evasion-final/dependencies.json");
    let old = read(&path);
    let mut new = old.clone();
    let mut prior = selected.old[0]["draft"]["items"].clone();
    let mut current = selected.new[0]["draft"]["items"].clone();
    let before = selected.old[0]["draft"]["allocator"]["lineage"].clone();
    let after = json!("94069f87dcc3175a4e3f7fdff2c0fe1e");
    relineage(&mut prior, &before, &after);
    relineage(&mut current, &before, &after);
    assert_eq!(
        old["selected_snapshot"]["items"], prior,
        "exact authenticated retained snapshot correspondence"
    );
    new["selected_snapshot"]["items"] = current.clone();
    let mut inverse = new.clone();
    inverse["selected_snapshot"]["items"] = prior.clone();
    assert_eq!(inverse, old);
    writes.insert(path, json_bytes(&new));

    let path = root
        .join(BASE)
        .join("item-evasion-property/dependencies.json");
    let old = read(&path);
    let mut new = old.clone();
    assert_eq!(old["items"].as_array().unwrap().len(), 4);
    for item in new["items"].as_array_mut().unwrap() {
        let before: Vec<_> = prior["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["id"] == item["id"])
            .collect();
        let after: Vec<_> = current["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["id"] == item["id"])
            .collect();
        assert_eq!(before.len(), 1);
        assert_eq!(after.len(), 1);
        let original_item = item.clone();
        assert_eq!(item["modifier_order"], before[0]["modifier_order"]);
        assert_eq!(before[0]["modifier_order"]["kind"], "known");
        assert_eq!(
            before[0]["modifier_order"]["value"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        // These four authored snapshots spell quantity values as integers;
        // the retained draft spells the same quantities as floating-point
        // JSON numbers. Preserve their original numbers and all other fields,
        // and compare through the strict current ItemDraft type rather than
        // introducing a generic JSON-number normalization rule.
        let object = item.as_object_mut().unwrap();
        let prior_order = object.remove("modifier_order").unwrap();
        assert!(
            object
                .insert(
                    "modifier_orders".into(),
                    after[0]["modifier_orders"].clone()
                )
                .is_none()
        );
        let authored: poe_optimizer_core::owned_draft::ItemDraft =
            serde_json::from_value(item.clone()).unwrap();
        let retained: poe_optimizer_core::owned_draft::ItemDraft =
            serde_json::from_value(after[0].clone()).unwrap();
        assert_eq!(authored, retained, "whole typed item/source correspondence");
        let mut item_inverse = item.clone();
        let object = item_inverse.as_object_mut().unwrap();
        assert_eq!(
            object.remove("modifier_orders").unwrap(),
            after[0]["modifier_orders"]
        );
        assert!(
            object
                .insert("modifier_order".into(), prior_order)
                .is_none()
        );
        assert_eq!(
            item_inverse, original_item,
            "exact preservation outside the order contract"
        );
    }
    let mut inverse = new.clone();
    inverse["items"] = old["items"].clone();
    assert_eq!(inverse, old);
    writes.insert(path, json_bytes(&new));

    // Existing authored artifacts pin their exact dependency bytes. Update only
    // the authenticated dependency entry, leaving every semantic recipe intact.
    for family in ["item-evasion-property", "player-evasion-final"] {
        let dependency = root.join(BASE).join(family).join("dependencies.json");
        let prior = fs::read(&dependency).unwrap();
        let current = writes.get(&dependency).unwrap();
        let path = root.join(BASE).join(family).join("authoring.json");
        let old = read(&path);
        let mut new = old.clone();
        let indices: Vec<_> = new["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                row["name"] == "dependencies.json"
                    || row["path"] == format!("{BASE}/{family}/dependencies.json")
            })
            .map(|(i, _)| i)
            .collect();
        assert_eq!(indices.len(), 1);
        let index = indices[0];
        let pin = &mut new["artifacts"][index];
        assert_eq!(pin["sha256"], hash(&prior));
        assert_eq!(pin["bytes"], prior.len());
        pin["sha256"] = json!(hash(current));
        pin["bytes"] = json!(current.len());
        let mut inverse = new.clone();
        inverse["artifacts"][index] = old["artifacts"][index].clone();
        assert_eq!(inverse, old);
        writes.insert(path, json_bytes(&new));
    }
}

#[test]
#[ignore = "explicit one-off retained evidence authoring"]
fn reauthor_current_modifier_order_evidence() {
    assert_eq!(
        std::env::var("POE_MODIFIER_ORDER_REAUTHOR").as_deref(),
        Ok("1")
    );
    let root = root();
    let requested_output = std::env::var_os("POE_MODIFIER_ORDER_REAUTHOR_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("runs/modifier-order-evidence-authoring"));
    assert!(requested_output.is_absolute());
    let parent = requested_output.parent().unwrap().canonicalize().unwrap();
    let bounded_runs = root.join("runs").canonicalize().unwrap();
    assert!(
        parent.starts_with(&bounded_runs),
        "resolved output parent must stay within runs"
    );
    let output = parent.join(requested_output.file_name().unwrap());
    assert!(!output.exists(), "do not overwrite archived originals");
    fs::create_dir_all(output.join("originals")).unwrap();
    let mut writes = BTreeMap::new();
    let data = [
        Spec {
            family: SELECTED,
            names: ["draft.json.gz", "sidecar.json.gz", "import-report.json.gz"],
            draft_sha: "251a87b8e19e1cbc26d9e70deeea8f74b64be7aab150fc6db5f8f9e28fb72e4f",
            authority: false,
        },
        Spec {
            family: MANA,
            names: ["draft.json.gz", "sidecar.json.gz", "import-report.json.gz"],
            draft_sha: "b1a355f8c842dd32ecc95d15b485cc437fe8bd13a80416fcf7f56d95573ef651",
            authority: false,
        },
        Spec {
            family: MANA,
            names: [
                "configuration-reward-draft.gz",
                "configuration-reward-sidecar.gz",
                "configuration-reward-report.gz",
            ],
            draft_sha: "2ac07856bfcb0f2994c75d26295ae989642bedd7b03295c2ea595219daa3ec96",
            authority: true,
        },
    ]
    .map(|spec| snapshot(spec, &root, &output.join("originals"), &mut writes));
    update_derived(&root, &data, &mut writes);
    // Archive all original receipts/derived JSON, then install only exact scoped files.
    let mut published = vec![];
    for (path, bytes) in &writes {
        let relative = path.strip_prefix(root.join(BASE)).unwrap();
        let original = fs::read(path).unwrap();
        let archive = output.join("originals").join(relative);
        fs::create_dir_all(archive.parent().unwrap()).unwrap();
        if archive.exists() {
            assert_eq!(fs::read(&archive).unwrap(), original);
        } else {
            fs::write(&archive, &original).unwrap();
        }
        fs::write(path, bytes).unwrap();
        published.push(json!({"path":path.strip_prefix(&root).unwrap().to_string_lossy(),"old_sha256":hash(&original),"new_sha256":hash(bytes)}));
    }
    let summary = json!({
        "files":published,
        "snapshots":data.iter().map(|s|json!({"family":s.family,
            "new_report_sha256":s.report_sha,"new_pending":s.issues,
            "receipt":s.receipt["modifier_order_reauthoring"]})).collect::<Vec<_>>(),
        "native_coverage_authorized":false,"source_selections_changed":false,
        "source_release_artifacts_reauthored":false
    });
    fs::write(output.join("report.json"), json_bytes(&summary)).unwrap();
    println!("{}", serde_json::to_string_pretty(&summary).unwrap());
}
