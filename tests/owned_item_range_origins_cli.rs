//! Intrinsic source ownership checked against immutable pre-change CLI outputs.
//! This checkpoint does not publish new game data or execute the Lua reference.
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_source::{SourceEvidenceLimits, SourceEvidenceRow, SourceProjectEvidence},
    source_xml::PobContentEntry,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path, path::PathBuf};

// Captured before the implementation change. Regenerating these with the new
// executable would destroy the independence of the migration check.
const HISTORICAL: [(&str, &str); 5] = [
    (
        "5ed2c20e149e058c0e28e959b5dcf4e1eae9766040504a595d2f3cfa03e53b0d",
        "0057e67c711e23d9b5d784d5511ab2e74b2ec932651cbddf2a3c58d23322dfa0",
    ),
    (
        "b2539a4650ebbba54106c1147175509d94f90d9401798674834f077f272e9475",
        "bc02e403b0893485b2dcc9dd6f89875d8a0586c342e522224146dc0f84839639",
    ),
    (
        "0933b813bd897f242923ea09b1f115f8f486ea638b468538ab28051755ed53c0",
        "a68d591618f028661dafcb7607e5aa4e74527632c420976d89fc45c2437e2b55",
    ),
    (
        "f9e75f04f75d77ab500ba1af02adcbe6085369729809bc2571cb0a5aadc5ae44",
        "0016c81a7a554348ee47e8f16ea30d950b1bee9846d217766c3089ad4ce92589",
    ),
    (
        "0ab326381e13be708c0889a7dfbe07237cec804356ddd1f521add0a390add0ee",
        "8c35fd5cf172c5eaec5489ce5439705e56a6f012e62ef40004d9ff943348e843",
    ),
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn hash(path: impl AsRef<Path>) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}
fn canonical(mut value: Value) -> Value {
    selected::canonical(&mut value);
    value
}
fn rows(value: &Value) -> &[Value] {
    value.as_array().unwrap()
}
fn attr<'a>(row: &'a SourceEvidenceRow<'_>, name: &str) -> &'a str {
    row.attribute(name).unwrap().decoded().unwrap()
}
fn links(mut links: Vec<Value>) -> Value {
    links.sort_by_key(Value::to_string);
    assert!(links.windows(2).all(|pair| pair[0] != pair[1]));
    json!(links)
}

// Diagnostic distinction only: source-row ownership also covers fixed literals.
// This recognizer never calculates values or determines admission.
fn has_numeric_range(text: &str) -> bool {
    text.split('(').skip(1).any(|tail| {
        tail.split_once(')').is_some_and(|(range, _)| {
            range.match_indices('-').any(|(index, _)| {
                range[..index].parse::<f64>().is_ok_and(f64::is_finite)
                    && range[index + 1..].parse::<f64>().is_ok_and(f64::is_finite)
            })
        })
    })
}

struct Expected {
    links: Value,
    owner: Value,
    proven: bool,
    numeric_range: bool,
    refused_roll_obligations: Vec<Value>,
}

fn expected_origins(xml: &[u8], sidecar: &Value, draft: &Value) -> BTreeMap<usize, Expected> {
    let instance = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([119; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&instance, SourceEvidenceLimits::default()).unwrap();
    let mut expected = BTreeMap::new();
    for row in evidence
        .rows()
        .iter()
        .filter(|row| row.occurrence().name() == "ModRange")
    {
        assert!(!row.occurrence().has_namespace_context());
        assert!(row.children().is_empty());
        assert_eq!(row.attributes().len(), 2);
        assert!(row.attributes().iter().all(|attribute| {
            attribute.origin().namespace.is_none()
                && matches!(attribute.origin().name.as_str(), "id" | "range")
                && attribute.decoded().is_ok()
        }));
        let poe_optimizer_import::owned_source::SourceContentEvidence::Available(range_content) =
            row.content()
        else {
            panic!("caller range content unavailable")
        };
        assert!(range_content.consumed().iter().all(|entry| {
            matches!(entry, PobContentEntry::Text { text, .. } if text.trim_ascii().is_empty())
        }));
        let ordinal = row.occurrence().id().ordinal() as usize;
        let parent_id = row.occurrence().parent().unwrap();
        let parent = &evidence.rows()[parent_id.ordinal() as usize];
        assert_eq!(parent.occurrence().name(), "Item");
        assert!(!parent.occurrence().has_namespace_context());
        assert_eq!(
            evidence.rows()[parent.occurrence().parent().unwrap().ordinal() as usize]
                .occurrence()
                .name(),
            "Items"
        );
        let id: usize = attr(row, "id").parse().unwrap();
        assert!(id > 0);
        assert_eq!(id.to_string(), attr(row, "id"));
        let fraction: f64 = attr(row, "range").parse().unwrap();
        assert!(fraction.is_finite() && (0.0..=1.0).contains(&fraction));
        let owner_origin = &sidecar["origins"][parent_id.ordinal() as usize];
        let owner_links: Vec<_> = rows(&owner_origin["links"])
            .iter()
            .filter(|link| link["kind"] == "item")
            .collect();
        assert_eq!(owner_links.len(), 1);
        let owner_id = &owner_links[0]["value"];
        let owners: Vec<_> = rows(&draft["draft"]["items"]["members"])
            .iter()
            .filter(|item| &item["id"] == owner_id)
            .collect();
        assert_eq!(owners.len(), 1);
        let owner = owners[0];
        let text: Vec<_> = rows(&sidecar["item_texts"])
            .iter()
            .filter(|text| text["source"]["ordinal"] == parent_id.ordinal())
            .collect();
        assert_eq!(text.len(), 1, "one historical consumed text frame");
        let text = text[0];
        let plan = &text["attribution"];
        assert_eq!(
            plan["layout"]["status"] == "proven",
            plan["layout"]["problems"].is_null()
        );
        assert_eq!(plan["item"], owner_origin["source"]);
        assert_eq!(plan["policy"], sidecar["item_source_policy"]);
        assert_eq!(plan["item_lines"], sidecar["item_policy"]);
        let writes: Vec<_> = rows(&plan["writes"])
            .iter()
            .enumerate()
            .filter(|(_, write)| {
                write["origin"]["kind"] == "xml"
                    && write["origin"]["occurrence"]["ordinal"] == ordinal
            })
            .collect();
        assert_eq!(
            writes.len(),
            1,
            "every source overlay has one historical receipt"
        );
        let (write_index, write) = writes[0];
        assert_eq!(
            write["origin"]["occurrence"],
            sidecar["origins"][ordinal]["source"]
        );
        assert_eq!(write["source_id"], id);
        assert_eq!(
            write["fraction"].as_f64().unwrap().to_bits(),
            fraction.to_bits()
        );
        for name in ["id", "range"] {
            let reference = &write["origin"][name];
            assert_eq!(reference["occurrence"], write["origin"]["occurrence"]);
            assert_eq!(
                row.attributes()[reference["index"].as_u64().unwrap() as usize]
                    .origin()
                    .name,
                name
            );
        }
        let poe_optimizer_import::owned_source::SourceContentEvidence::Available(content) =
            parent.content()
        else {
            panic!("caller item content unavailable")
        };
        assert_eq!(plan["content_entry"], 0);
        assert_eq!(text["content_entry"], 0);
        let PobContentEntry::Text { text: raw, .. } = &content.consumed()[0] else {
            panic!("historical range frame starts without item text")
        };
        assert!(
            content.consumed()[1..]
                .iter()
                .all(|entry| { matches!(entry, PobContentEntry::Element { .. }) })
        );
        for line in rows(&plan["lines"]) {
            let start = line["decoded_span"]["start"].as_u64().unwrap() as usize;
            let end = line["decoded_span"]["end"].as_u64().unwrap() as usize;
            assert_eq!(&raw[start..end], line["raw"].as_str().unwrap());
        }
        let consumed_index = write["origin"]["content_entry"].as_u64().unwrap() as usize;
        let PobContentEntry::Element { child_index } = content.consumed()[consumed_index] else {
            panic!("range receipt addresses text")
        };
        assert_eq!(parent.children()[child_index], row.occurrence().id());
        let layout_proven = plan["layout"]["status"] == "proven";
        let mut targets = vec![json!({"kind":"item","value":owner_id})];
        let mut numeric_range = false;
        let mut refused_roll_obligations = vec![];
        if layout_proven {
            assert_eq!(write["target"]["status"], "line");
            let line_index = write["target"]["value"].as_u64().unwrap() as usize;
            let line = &plan["lines"][line_index - 1];
            assert_eq!(line["index"], line_index);
            assert!(!line["member"].is_null());
            assert!(rows(&line["blockers"]).is_empty());
            assert_eq!(line["range"]["status"], "resolved");
            assert_eq!(line["range"]["winning_write"], write_index);
            assert_eq!(line["range"]["fraction"], write["fraction"]);
            numeric_range = has_numeric_range(line["semantic_text"].as_str().unwrap());
            let converted = rows(&text["lines"])
                .iter()
                .find(|line| line["index"] == line_index)
                .unwrap();
            assert_eq!(converted["outcome"]["kind"], "known");
            assert!(!rows(&converted["modifiers"]).is_empty());
            let emissions = rows(&converted["outcome"]["value"]["emissions"]);
            assert_eq!(emissions.len(), rows(&converted["modifiers"]).len());
            for (id, emission) in rows(&converted["modifiers"]).iter().zip(emissions) {
                assert_eq!(emission["kind"], "modifier");
                let emission = &emission["value"];
                let matched: Vec<_> = rows(&owner["modifiers"]["members"])
                    .iter()
                    .filter(|modifier| &modifier["id"] == id)
                    .collect();
                assert_eq!(matched.len(), 1, "modifier belongs to exact physical item");
                let actual = matched[0];
                assert_eq!(actual["definition"]["kind"], "known");
                assert_eq!(actual["definition"]["value"], emission["definition"]);
                let actual_rolls = rows(&actual["rolls"]["members"]);
                let emitted_rolls = rows(&emission["rolls"]);
                assert_eq!(actual_rolls.len(), emitted_rolls.len());
                for (actual, emitted) in actual_rolls.iter().zip(emitted_rolls) {
                    assert_eq!(actual["slot"]["kind"], "known");
                    assert_eq!(actual["value"]["kind"], "known");
                    assert_eq!(actual["slot"]["value"], emitted["slot"]);
                    assert_eq!(actual["value"]["value"], emitted["value"]);
                }
                let completion = &actual["rolls"]["completion"];
                // ConvertedItemEmission omits SchemaClosure::Complete in the
                // historical sidecar. A known subset is not a closed inventory.
                if let Some(closure) = emission.get("rolls_closure") {
                    assert_eq!(closure["kind"], "partial");
                    let gaps = rows(&closure["value"]["gaps"]);
                    assert!(!gaps.is_empty());
                    assert!(gaps.iter().all(|gap| {
                        gap["facet"] == "input_schema"
                            && gap["code"] == "modifier-eligibility-inputs-unconverted"
                    }));
                    assert_eq!(completion["kind"], "pending");
                    assert_eq!(completion["code"], "modifier-roll-schema-partial");
                    refused_roll_obligations.push(json!({
                        "modifier":id,"completion":completion,"closure":closure,
                        "known_assignments_verified":actual_rolls.len()
                    }));
                } else {
                    assert_eq!(completion["kind"], "complete");
                }
                targets.push(json!({"kind":"modifier","value":id}));
            }
            if !refused_roll_obligations.is_empty() {
                targets = rows(&sidecar["origins"][ordinal]["links"]).to_vec();
            }
        } else {
            assert_eq!(write["target"]["status"], "pending");
            let obligation = &owner["modifiers"]["completion"];
            assert_eq!(obligation["kind"], "pending");
            assert_eq!(obligation["code"], "item-modifiers-not-converted");
            targets.push(json!({"kind":"issue","value":obligation["id"]}));
        }
        assert!(
            expected
                .insert(
                    ordinal,
                    Expected {
                        links: links(targets),
                        owner: owner_id.clone(),
                        proven: layout_proven && refused_roll_obligations.is_empty(),
                        numeric_range,
                        refused_roll_obligations,
                    }
                )
                .is_none()
        );
    }
    expected
}

fn compare(case: usize, xml: &[u8], baseline: &Path, output: &Path) -> Value {
    let before_dir = baseline.join(format!("original-{case:02}"));
    let after_dir = output.join(format!("original-{case:02}"));
    let old_side = read(before_dir.join("sidecar.json"));
    let new_side = read(after_dir.join("sidecar.json"));
    assert_eq!(old_side["schema_version"], 18);
    assert_eq!(new_side["schema_version"], 19);
    assert_eq!(
        old_side["source_sha256"],
        format!("{:x}", Sha256::digest(xml))
    );
    let old_draft = read(before_dir.join("draft.json"));
    let new_draft = read(after_dir.join("draft.json"));
    assert_eq!(
        canonical(old_draft.clone()),
        canonical(new_draft),
        "original{case}: complete draft including all issue IDs is unchanged"
    );
    let expected = expected_origins(xml, &old_side, &old_draft);
    let proven = expected.values().filter(|row| row.proven).count();
    let refused: Vec<_> = expected
        .iter()
        .filter(|(_, row)| !row.refused_roll_obligations.is_empty())
        .map(|(ordinal, _)| *ordinal)
        .collect();
    let numeric = expected.values().filter(|row| row.numeric_range).count();
    assert_eq!(expected.len(), [74, 133, 89, 89, 101][case - 1]);
    assert_eq!(proven, [0, 4, 0, 1, 13][case - 1]);
    assert_eq!(
        refused,
        if case == 2 {
            vec![503, 505, 507, 509, 511]
        } else {
            vec![]
        }
    );
    let deferred = expected.len() - proven - refused.len();
    assert_eq!(deferred, [74, 124, 89, 88, 88][case - 1]);
    assert_eq!(numeric, [0, 0, 0, 0, 4][case - 1]);
    assert_eq!(
        rows(&old_side["origins"]).len(),
        rows(&new_side["origins"]).len()
    );
    let mut changed_origins = vec![];
    let mut unchanged_origins = vec![];
    for (ordinal, (old, new)) in rows(&old_side["origins"])
        .iter()
        .zip(rows(&new_side["origins"]))
        .enumerate()
    {
        assert_eq!(old["source"]["ordinal"], ordinal);
        assert_eq!(new["source"], old["source"]);
        if let Some(expected) = expected.get(&ordinal) {
            assert_eq!(old["disposition"]["kind"], "contributes");
            assert_eq!(new["disposition"], old["disposition"]);
            assert!(!rows(&old["links"]).is_empty());
            assert!(
                rows(&old["links"])
                    .iter()
                    .all(|link| link["kind"] == "issue")
            );
            assert_eq!(
                canonical(links(rows(&new["links"]).to_vec())),
                canonical(expected.links.clone()),
                "original{case} source{ordinal}: exact owner and proof targets"
            );
            if expected.refused_roll_obligations.is_empty() {
                assert_ne!(canonical(old.clone()), canonical(new.clone()));
                changed_origins.push(ordinal);
            } else {
                assert_eq!(
                    canonical(old.clone()),
                    canonical(new.clone()),
                    "original{case} source{ordinal}: incomplete roll inventory retains exact fallback"
                );
                unchanged_origins.push(ordinal);
            }
        } else {
            assert_eq!(
                canonical(old.clone()),
                canonical(new.clone()),
                "original{case} source{ordinal}: unrelated origin changed"
            );
            unchanged_origins.push(ordinal);
        }
    }
    assert_eq!(changed_origins.len(), proven + deferred);
    assert_eq!(
        changed_origins.len() + unchanged_origins.len(),
        rows(&old_side["origins"]).len()
    );
    let mut expected_side = old_side.clone();
    for field in ["schema_version", "origins", "draft"] {
        expected_side[field] = new_side[field].clone();
    }
    assert_eq!(
        canonical(expected_side),
        canonical(new_side.clone()),
        "all policy/data bindings, allocator states and item attribution receipts unchanged"
    );
    assert_eq!(
        canonical(selected::selection(xml, &before_dir)),
        canonical(selected::selection(xml, &after_dir)),
        "selected axes and queries unchanged"
    );
    let finalization = selected::finalize_with_definitions(
        xml,
        &after_dir,
        &output.join(format!("selection-{case:02}.json")),
        &baseline.join("package/schema.json"),
    );
    assert!(rows(&finalization["intent_validation"]["schema_issues"]).is_empty());
    assert_eq!(
        finalization["finalization"]["draft_digest"],
        new_side["draft"]
    );
    let mut final_state = finalization["finalization"].clone();
    final_state.as_object_mut().unwrap().remove("draft_digest");
    let historical = read(baseline.join("validation.json"));
    assert_eq!(
        canonical(final_state),
        historical["originals"][case - 1]["finalization"],
        "selected Pending obligations unchanged"
    );
    if case == 5 {
        let role = &old_draft["draft"]["choice_presets"]["members"][0]["choices"]["completion"];
        assert_eq!(role["id"]["local"], "00000000000001f2");
        assert_eq!(role["kind"], "pending");
        assert_eq!(role["code"], "configuration-roles-not-converted");
        let count = |side: &Value| {
            rows(&side["origins"])
                .iter()
                .filter(|origin| {
                    rows(&origin["links"]).iter().any(|link| {
                        link["kind"] == "issue" && link["value"]["local"] == "00000000000001f2"
                    })
                })
                .count()
        };
        assert_eq!(count(&old_side), 180);
        assert_eq!(count(&new_side), 79);
    }
    json!({"original":case,"total_range_origins":expected.len(),"proven_owner_origins":proven,
        "proven_fixed_line_origins":proven-numeric,"proven_numeric_range_line_origins":numeric,
        "deferred_owner_obligations":deferred,"refused_incomplete_roll_origins":refused.len(),
        "changed_origin_ordinals":changed_origins,"unchanged_origin_ordinals":unchanged_origins,
        "origins":expected.iter().map(|(ordinal,row)|json!({"source_ordinal":ordinal,"item":row.owner,
            "proof":if row.proven{"actual_modifier_output"}else if row.refused_roll_obligations.is_empty(){"existing_item_modifier_membership_obligation"}else{"refused_incomplete_roll_inventory"},
            "historical_links":old_side["origins"][*ordinal]["links"],"current_links":new_side["origins"][*ordinal]["links"],
            "refused_roll_obligations":row.refused_roll_obligations,
            "numeric_range_in_source_line":row.numeric_range})).collect::<Vec<_>>(),
        "historical_sidecar_sha256":HISTORICAL[case-1].0,"historical_draft_sha256":HISTORICAL[case-1].1,
        "sidecar_sha256":hash(after_dir.join("sidecar.json")),"sidecar_schema":19,
        "canonical_draft_unchanged":true,"selection_unchanged":true,"all_existing_issues_unchanged":true,
        "unrelated_origins_unchanged":true,"complete_build":false})
}

#[test]
#[ignore = "requires immutable source-presentation-03 historical outputs and its checked package"]
fn intrinsic_item_range_origins_preserve_all_five_historical_requests() {
    let baseline = root().join("runs/owned-source-presentation-03");
    let package = baseline.join("package");
    let output = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_RANGE_ORIGINS_OUTPUT")
            .expect("explicit new output directory"),
    );
    assert!(!output.exists());
    assert_eq!(
        hash(baseline.join("validation.json")),
        "15027da51fecd5b106a8fd11c7862616ed6864e4a4acb6553a3b00266ad11e0e"
    );
    assert_eq!(
        hash(package.join("release.json")),
        "292417a87199ac2d121bf055abfd20f889be372f132839670891f87f24c77fd8"
    );
    for (index, (sidecar, draft)) in HISTORICAL.iter().enumerate() {
        let dir = baseline.join(format!("original-{:02}", index + 1));
        assert_eq!(hash(dir.join("sidecar.json")), *sidecar);
        assert_eq!(hash(dir.join("draft.json")), *draft);
    }
    let package_files = release::inventory(&package);
    let checked = release::load(&package);
    assert_eq!(checked.receipt().query_rows, 110);
    assert_eq!(
        json!(checked.receipt().input),
        read(baseline.join("validation.json"))["after"]
    );
    let binary_hash = hash(env!("CARGO_BIN_EXE_poe-optimizer"));
    fs::create_dir_all(&output).unwrap();
    let mut originals = vec![];
    for case in 1..=5 {
        let xml_path = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let xml = fs::read(&xml_path).unwrap();
        let result = release::normalize(
            &package,
            &xml_path,
            case,
            &output.join(format!("original-{case:02}")),
        );
        assert_eq!(
            result["draft_digest"],
            read(output.join(format!("original-{case:02}/sidecar.json")))["draft"]
        );
        originals.push(compare(case, &xml, &baseline, &output));
        assert_eq!(fs::read(xml_path).unwrap(), xml);
    }
    assert_eq!(release::inventory(&package), package_files);
    assert_eq!(hash(env!("CARGO_BIN_EXE_poe-optimizer")), binary_hash);
    let result = json!({"schema_version":1,"historical_checkpoint":"owned-source-presentation-03",
        "release_input":checked.receipt().input,"package_files":package_files,"package_unchanged":true,
        "normalization_policy_unchanged":true,"sidecar_schema":19,"binary_sha256":binary_hash,
        "checkpoint_test_sha256":hash(root().join("tests/owned_item_range_origins_cli.rs")),
        "normalizer_source_sha256":hash(root().join("crates/poe-optimizer-import/src/owned_normalize.rs")),
        "range_origin_source_sha256":hash(root().join("crates/poe-optimizer-import/src/owned_normalize/item_range_origins.rs")),
        "item_normalizer_source_sha256":hash(root().join("crates/poe-optimizer-import/src/owned_normalize/items.rs")),
        "range_origin_census":{"audited":486,"changed":481,"proven_modifier_owners":18,
            "pending_item_membership_owners":463,"refused_incomplete_roll_inventories":5},
        "coverage_limit":"Original02 sources 503, 505, 507, 509 and 511 have proven source layout and exact known emitted assignments, but modifier eligibility input membership remains partial. Their original fallback links remain unchanged; these rows are not admitted as modifier-owner correspondence.",
        "originals":originals,"queries":110,"complete_original_builds":0,
        "numerical_rules_unchanged":true,"source_execution":"not_run"});
    fs::write(
        output.join("validation.json"),
        serde_json::to_vec(&result).unwrap(),
    )
    .unwrap();
}
