//! Explicit, offline current-format migration checks; never a runtime compatibility reader.
#[allow(dead_code)]
#[path = "support/owned_identity_correspondence.rs"]
mod correspondence;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_plan_replay.rs"]
mod replay;
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::owned_draft::{DraftLimits, decode_draft};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
};

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn env(name: &str) -> PathBuf {
    std::env::var_os(name).expect(name).into()
}
fn issued(value: &Value) -> u64 {
    u64::from_str_radix(value["last_issued"].as_str().unwrap(), 16).unwrap()
}

fn exact_item_text<'a>(sidecar: &'a Value, item: &Value) -> &'a Value {
    let texts: Vec<_> = sidecar["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|text| {
            let origins: Vec<_> = sidecar["origins"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["source"] == text["source"])
                .collect();
            assert_eq!(origins.len(), 1, "one exact source occurrence");
            origins[0]["links"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|link| link["kind"] == "item" && link["value"] == *item)
                .count()
                == 1
        })
        .collect();
    assert_eq!(
        texts.len(),
        1,
        "one physical Item text, excluding child range origins"
    );
    texts[0]
}

fn assert_source_orders(item: &Value, text: &Value) {
    assert_eq!(item["modifiers"]["completion"]["kind"], "complete");
    assert_eq!(text["attribution"]["layout"]["status"], "proven");
    let mut members = Vec::new();
    for source in text["attribution"]["lines"].as_array().unwrap() {
        if source["member"].is_null() {
            continue;
        }
        let index = source["index"].as_u64().unwrap();
        assert_eq!(source["member"]["line"], index);
        assert_eq!(source["presentation"], false);
        assert!(source["blockers"].as_array().unwrap().is_empty());
        let emitted: Vec<_> = text["lines"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|line| line["index"] == index)
            .collect();
        assert_eq!(emitted.len(), 1);
        let line = emitted[0];
        assert_eq!(line["outcome"]["kind"], "known");
        assert_eq!(line["outcome"]["value"]["rule"], source["rule"]);
        let emissions = line["outcome"]["value"]["emissions"].as_array().unwrap();
        assert_eq!(emissions.len(), 1);
        assert_eq!(emissions[0]["kind"], "modifier");
        let ids = line["modifiers"].as_array().unwrap();
        assert_eq!(ids.len(), 1);
        let category = match source["member"]["category"].as_str().unwrap() {
            "implicit" => 0u8,
            "explicit" => 1u8,
            other => panic!("unproved construction category: {other}"),
        };
        members.push((
            index,
            category,
            source["member"]["ordinal"].as_u64().unwrap(),
            ids[0].clone(),
        ));
    }
    let physical: BTreeSet<_> = item["modifiers"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|member| member["id"].to_string())
        .collect();
    let witnessed: BTreeSet<_> = members.iter().map(|member| member.3.to_string()).collect();
    assert_eq!(members.len(), physical.len());
    assert_eq!(
        witnessed, physical,
        "exact complete source-to-occurrence correspondence"
    );
    members.sort_by_key(|member| member.0);
    assert!(members.windows(2).all(|pair| pair[0].0 != pair[1].0));
    let discovery: Vec<_> = members.iter().map(|member| member.3.clone()).collect();
    assert_eq!(
        item["modifier_orders"]["numeric_transforms"],
        json!({"kind":"known","value":discovery})
    );
    members.sort_by_key(|member| (member.1, member.2));
    for category in [0u8, 1u8] {
        for (position, member) in members
            .iter()
            .filter(|member| member.1 == category)
            .enumerate()
        {
            assert_eq!(member.2, position as u64 + 1);
        }
    }
    let contributions: Vec<_> = members.iter().map(|member| member.3.clone()).collect();
    assert_eq!(
        item["modifier_orders"]["contributions"],
        json!({"kind":"known","value":contributions})
    );
}

fn authenticated_normalization(directory: &Path, source: &Path) -> (Value, Value) {
    let draft_bytes = fs::read(directory.join("draft.json")).unwrap();
    let draft = decode_draft(&draft_bytes, DraftLimits::default()).unwrap();
    let digest_value = draft.digest(64 * 1024 * 1024).unwrap().to_string();
    let side_bytes = fs::read(directory.join("sidecar.json")).unwrap();
    let sidecar: Value = serde_json::from_slice(&side_bytes).unwrap();
    let report = read(directory.join("report.json"));
    assert_eq!(sidecar["draft"], digest_value);
    assert_eq!(report["draft_digest"], digest_value);
    assert_eq!(report["sidecar_sha256"], digest(&side_bytes));
    assert_eq!(report["sidecar_bytes"], side_bytes.len());
    assert_eq!(sidecar["source_sha256"], digest(&fs::read(source).unwrap()));
    let value: Value = serde_json::from_slice(&draft_bytes).unwrap();
    assert_eq!(sidecar["allocator_after"], value["draft"]["allocator"]);
    (value, sidecar)
}

#[test]
#[ignore = "offline authoring: authenticated predecessor gzip and new output directory"]
fn migrate_joined_replay_with_exact_format_inverse() {
    let before = fs::read(env("POE_OPTIMIZER_ORDER_REPLAY_PRIOR")).unwrap();
    assert_eq!(
        digest(&before),
        "d52b4af0d05a1d38ca108acf3f043977c2bfa93c3416ddfe51a7f34cce73013e"
    );
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(before.as_slice())
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 8 * 1024 * 1024);
    let original: Value = serde_json::from_slice(&bytes).unwrap();
    let mut next = original.clone();
    let items = next["build"]["items"].as_array_mut().unwrap();
    let count = items.len();
    for item in items {
        let fields = item.as_object_mut().unwrap();
        let order = fields.remove("modifier_order").unwrap();
        // This retained finite fixture explicitly used one sequence for both
        // consumers. This preserves that fixture, not arbitrary source imports.
        assert!(
            fields
                .insert(
                    "modifier_orders".into(),
                    json!({
                        "contributions":order, "numeric_transforms":order
                    })
                )
                .is_none()
        );
    }
    let input: replay::ReplayInput = serde_json::from_value(next).unwrap();
    let plan = input.compile().unwrap();
    let mut scratch = plan.new_scratch();
    let report = plan.evaluate(&mut scratch).unwrap();
    assert_eq!(report, plan.evaluate(&mut scratch).unwrap());
    let fresh = input.compile().unwrap();
    assert_eq!(report, fresh.evaluate(&mut fresh.new_scratch()).unwrap());
    let encoded = input.encode();
    assert_eq!(replay::ReplayInput::decode(&encoded).encode(), encoded);
    let mut restored = serde_json::to_value(&input).unwrap();
    for item in restored["build"]["items"].as_array_mut().unwrap() {
        let fields = item.as_object_mut().unwrap();
        let orders = fields.remove("modifier_orders").unwrap();
        assert_eq!(orders["contributions"], orders["numeric_transforms"]);
        fields.insert("modifier_order".into(), orders["contributions"].clone());
    }
    assert_eq!(
        restored, original,
        "every non-format value and all commitments survive"
    );
    let out = env("POE_OPTIMIZER_ORDER_REPLAY_OUTPUT");
    fs::create_dir(&out).unwrap();
    fs::write(out.join("owned-sniper-replay.json.gz"), &encoded).unwrap();
    let raw = serde_json::to_vec(&input).unwrap();
    write(
        out.join("proof.json"),
        &json!({"format_inverse":true,"items":count,
        "fresh_and_reused_reports_equal":true,"deterministic_encoding":true,
        "prior_sha256":digest(&before),"gzip_sha256":digest(&encoded),
        "gzip_bytes":encoded.len(),"json_sha256":digest(&raw),"json_bytes":raw.len()}),
    );
}

fn old_code(value: &mut Value) {
    match value {
        Value::Array(rows) => rows.iter_mut().for_each(old_code),
        Value::Object(fields) => {
            if fields.get("code") == Some(&json!("item-modifier-contribution-order-not-converted"))
            {
                fields.insert("code".into(), json!("item-modifier-order-not-converted"));
            }
            fields.values_mut().for_each(old_code);
        }
        _ => {}
    }
}

#[test]
#[ignore = "offline authoring: current checked release, predecessor imports and fresh output directory"]
fn reimport_all_five_preserves_every_saved_value_and_query() {
    let baseline = env("POE_OPTIMIZER_ORDER_PRIOR_PUBLICATION");
    let package = baseline.join("package");
    let checked = release::load(&package);
    assert_eq!(
        checked.receipt().input.to_string(),
        "b8851ea50c9d5fd97e1695e536c89dba185f136d51d6e77da31bd22f8f4a67a9"
    );
    let out = env("POE_OPTIMIZER_ORDER_IMPORT_OUTPUT");
    fs::create_dir(&out).unwrap();
    let mut rows = Vec::new();
    for case in 1..=5 {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let destination = out.join(format!("original-{case:02}"));
        release::normalize(&package, &source, case, &destination);
        let prior = baseline.join(format!("original-{case:02}"));
        let mut before = read(prior.join("draft.json"));
        let mut before_side = read(prior.join("sidecar.json"));
        let (mut after, mut after_side) = authenticated_normalization(&destination, &source);
        let repeated = out.join(format!("replay-{case:02}"));
        release::normalize(&package, &source, case, &repeated);
        let (mut replay_draft, mut replay_side) = authenticated_normalization(&repeated, &source);
        let mut first_draft = after.clone();
        let mut first_side = after_side.clone();
        // Both digests were independently authenticated against typed drafts.
        // Fresh lineage is the only permitted semantic difference between runs.
        replay_side["draft"] = first_side["draft"].clone();
        selected::canonical(&mut first_draft);
        selected::canonical(&mut first_side);
        selected::canonical(&mut replay_draft);
        selected::canonical(&mut replay_side);
        assert_eq!(replay_draft, first_draft, "fresh import determinism");
        assert_eq!(
            replay_side, first_side,
            "fresh source accounting determinism"
        );
        let mut extra = BTreeSet::new();
        let mut extra_owners = BTreeMap::new();
        let old_items = before["draft"]["items"]["members"].as_array().unwrap();
        let new_items = after["draft"]["items"]["members"].as_array_mut().unwrap();
        assert_eq!(old_items.len(), new_items.len());
        for (old, new) in old_items.iter().zip(new_items) {
            let text = exact_item_text(&after_side, &new["id"]);
            if new["modifier_orders"]["contributions"]["kind"] == "known" {
                assert_source_orders(new, text);
            }
            let item_id = new["id"].clone();
            let item_source = text["source"].clone();
            let fields = new.as_object_mut().unwrap();
            let orders = fields.remove("modifier_orders").unwrap();
            let contributions = orders["contributions"].clone();
            let transforms = &orders["numeric_transforms"];
            assert_eq!(contributions["kind"], old["modifier_order"]["kind"]);
            if contributions["kind"] == "known" {
                assert_eq!(transforms["kind"], "known");
            } else {
                assert_eq!(transforms["kind"], "pending");
                assert_eq!(
                    transforms["code"],
                    "item-modifier-numeric-transform-order-not-converted"
                );
                assert_ne!(transforms["id"], contributions["id"]);
                assert!(transforms["candidates"].as_array().unwrap().is_empty());
                assert!(extra.insert(transforms["id"].to_string()));
                assert!(
                    extra_owners
                        .insert(transforms["id"].to_string(), (item_id, item_source))
                        .is_none()
                );
            }
            fields.insert("modifier_order".into(), contributions);
        }
        assert_eq!(
            issued(&after["draft"]["allocator"]),
            issued(&before["draft"]["allocator"]) + extra.len() as u64
        );
        let mut removed = BTreeSet::new();
        for row in after_side["origins"].as_array_mut().unwrap() {
            let source = row["source"].clone();
            let items: Vec<_> = row["links"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|link| link["kind"] == "item")
                .map(|link| link["value"].clone())
                .collect();
            row["links"].as_array_mut().unwrap().retain(|link| {
                if link["kind"] == "issue" && extra.contains(&link["value"].to_string()) {
                    let (owner, expected_source) = &extra_owners[&link["value"].to_string()];
                    assert_eq!(
                        source, *expected_source,
                        "new obligation must belong to the exact physical Item source"
                    );
                    assert_eq!(
                        items,
                        vec![owner.clone()],
                        "new obligation must retain its exact Item owner"
                    );
                    assert!(removed.insert(link["value"].to_string()));
                    false
                } else {
                    true
                }
            });
        }
        assert_eq!(
            removed, extra,
            "every new obligation retains exact source attribution"
        );
        // Only allocator watermarks/lineages and the new order obligation differ.
        after["draft"]["allocator"] = before["draft"]["allocator"].clone();
        after_side["allocator_after"] = before_side["allocator_after"].clone();
        after_side["draft"] = before_side["draft"].clone();
        selected::canonical(&mut before);
        selected::canonical(&mut after);
        selected::canonical(&mut before_side);
        selected::canonical(&mut after_side);
        old_code(&mut after);
        old_code(&mut after_side);
        let mut ids = BTreeMap::new();
        correspondence::correspond(&before, &mut after, &mut ids, "complete draft inverse");
        correspondence::correspond(
            &before_side,
            &mut after_side,
            &mut ids,
            "complete source sidecar inverse",
        );
        let xml = fs::read(&source).unwrap();
        let report = selected::finalize_with_definitions(
            &xml,
            &destination,
            &out.join(format!("selected-{case:02}.json")),
            &package.join("schema.json"),
        );
        let mut old_selection = read(baseline.join(format!("selected-{case:02}.json")));
        let mut new_selection = selected::selection(&xml, &destination);
        selected::canonical(&mut old_selection);
        selected::canonical(&mut new_selection);
        correspondence::correspond(
            &old_selection,
            &mut new_selection,
            &mut ids,
            "saved selection",
        );
        let queries =
            before["draft"]["query_presets"]["members"][0]["queries"]["requests"]["members"]
                .as_array()
                .unwrap()
                .len();
        assert_eq!(queries, 22);
        let issues = report["finalization"]["issues"].as_array().unwrap();
        let selected_new: Vec<_> = issues
            .iter()
            .filter(|r| r["code"] == "item-modifier-numeric-transform-order-not-converted")
            .collect();
        let mut selected_new_ids = BTreeSet::new();
        for issue in &selected_new {
            let id = issue["id"].to_string();
            assert!(selected_new_ids.insert(id.clone()));
            let (owner, _) = extra_owners
                .get(&id)
                .expect("selected new issue was declared by this import");
            assert_eq!(issue["owner"], *owner);
            assert!(
                issue["path"]
                    .as_str()
                    .unwrap()
                    .ends_with(".modifier_orders.numeric_transforms")
            );
        }
        let new_selected = selected_new.len();
        assert_eq!(
            issues.len(),
            [107, 118, 110, 124, 5][case - 1] + new_selected
        );
        rows.push(
            json!({"original":case,"queries":queries,"draft_and_sidecar_inverse":true,
            "independent_source_orders":true,"exact_pending_item_origins":true,
            "fresh_imports_equal_after_lineage_canonicalization":true,
            "source_sha256":digest(&xml),"extra_order_obligations":extra.len(),
            "selected_issues":issues.len(),"selected_extra_order_obligations":new_selected}),
        );
    }
    write(
        out.join("proof.json"),
        &json!({"originals":rows,"queries":110,"complete_builds":0}),
    );
}
