//! Shared all-five exact preservation and real-input mutation checks.
use crate::selected;
use poe_optimizer_import::owned_release::StagedOwnedRelease;
#[allow(dead_code)]
#[path = "owned_configuration_accounting.rs"]
mod edits;
#[path = "owned_identity_correspondence.rs"]
mod identity;
#[allow(dead_code)]
#[path = "owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "owned_release_fixture.rs"]
mod release;
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance, decode_build, owned_source::SourceProjectEvidence,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, v: &impl Serialize) {
    fs::write(path, serde_json::to_vec(v).unwrap()).unwrap()
}
fn publish(input: &Path, output: &Path) -> Value {
    let r = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    serde_json::from_slice(&r.stdout).unwrap()
}
fn assumptions(draft: &mut Value) -> &mut Vec<Value> {
    assert_eq!(
        draft["draft"]["scenario_presets"]["members"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    draft["draft"]["scenario_presets"]["members"][0]["scenario"]["assumptions"]["members"]
        .as_array_mut()
        .unwrap()
}
fn expected(input: &Value, v: f64) -> Value {
    let mut value = input["constructor_default"].clone();
    value["value"]["value"] = json!(v);
    json!({"input":{"kind":"known","value":input["value_input"]},"target":{"kind":"enemy"},"value":{"kind":"known","value":value}})
}
fn source(xml: &[u8]) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([151; 16]),
        Default::default(),
    )
    .unwrap()
}
fn leaf(e: &SourceProjectEvidence, element: &str, source_name: &str) -> usize {
    let rows: Vec<_> = e
        .rows()
        .iter()
        .filter(|r| {
            r.occurrence().name() == element
                && r.attribute("name")
                    .is_some_and(|a| a.decoded().ok() == Some(source_name))
        })
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0].occurrence().id().ordinal() as usize
}
fn metadata_lineage(v: &mut Value, old: &Value, new: &Value) {
    assert_eq!(
        &v["lineage"], new,
        "exact source metadata must belong to the fresh import"
    );
    v["lineage"] = old.clone();
}
fn census(sidecar: &Value, e: &SourceProjectEvidence, issue: &Value) -> (usize, usize, usize) {
    let mut total = 0;
    let mut inside = 0;
    for row in sidecar["origins"].as_array().unwrap() {
        if !row["links"].as_array().unwrap().contains(issue) {
            continue;
        }
        total += 1;
        let mut current = Some(
            e.rows()[row["source"]["ordinal"].as_u64().unwrap() as usize]
                .occurrence()
                .id(),
        );
        while let Some(id) = current {
            let r = e.rows()[id.ordinal() as usize].occurrence();
            if r.name() == "Config" {
                inside += 1;
                break;
            }
            current = r.parent()
        }
    }
    (total, inside, total - inside)
}

pub struct Publication {
    pub directory: PathBuf,
    pub stage: fn(&StagedOwnedRelease) -> StagedOwnedRelease,
    pub prior: PathBuf,
    pub output: PathBuf,
    pub original05_before: (usize, usize, usize),
    pub new_definitions: usize,
}
pub fn run(spec: Publication) {
    let inputs: Vec<Value> =
        crate::configuration_stage::inputs(read(spec.directory.join("input.json")))
            .into_iter()
            .map(|v| serde_json::to_value(v).unwrap())
            .collect();
    // This harness has two explicit case families: the historical singleton
    // default adapters, and the reviewed two-output distance selector pair.
    // Vector length alone must not imply either source disposition or semantics.
    let projected = inputs.iter().any(|i| i.get("selection").is_some());
    if projected {
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0]["selection"], "input_then_placeholder");
        assert_eq!(inputs[1]["selection"], "nonzero_input_then_placeholder");
        for input in &inputs {
            assert_eq!(input["source_name"], "enemyDistance");
            assert_eq!(input["target"], "enemy");
            assert!(
                input
                    .get("ignore_numeric_placeholder")
                    .is_none_or(|v| v == false)
            );
        }
    } else {
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0]["ignore_numeric_placeholder"], true);
    }
    let input = &inputs[0];
    let source_name = input["source_name"].as_str().unwrap();
    let input_ids: Vec<_> = inputs
        .iter()
        .map(|i| i["value_input"]["key"].clone())
        .collect();
    let default = input["constructor_default"]["value"]["value"]
        .as_f64()
        .unwrap();
    for i in &inputs {
        assert_eq!(i["source_name"], input["source_name"]);
        assert_eq!(i["constructor_default"], input["constructor_default"]);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let prior_path = spec.prior;
    let out = spec.output;
    assert!(!out.exists());
    let prior_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = (spec.stage)(&prior);
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    write(out.join("receipt.json"), next.receipt());
    let package = out.join("package");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(
        publish(&package, &out.join("rebuilt")),
        json!(next.receipt())
    );
    assert_eq!(
        release::inventory(&package),
        release::inventory(&out.join("rebuilt"))
    );
    assert_eq!(release::load(&package).receipt(), next.receipt());
    assert_eq!(next.receipt().query_rows, 110);
    let mut results = vec![];
    for case in 1..=5 {
        let xml = root.join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let bytes = fs::read(&xml).unwrap();
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&prior_path, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        let a = read(old.join("draft.json"));
        let mut b = read(new.join("draft.json"));
        assert_eq!(
            {
                let assumptions = assumptions(&mut b);
                assumptions.split_off(assumptions.len() - inputs.len())
            },
            inputs
                .iter()
                .map(|i| expected(i, default))
                .collect::<Vec<_>>()
        );
        let oldlineage = a["draft"]["allocator"]["lineage"].clone();
        let newlineage = b["draft"]["allocator"]["lineage"].clone();
        assert_ne!(oldlineage, newlineage);
        assert_eq!(
            a["draft"]["allocator"]["last_issued"],
            b["draft"]["allocator"]["last_issued"]
        );
        b["draft"]["allocator"] = a["draft"]["allocator"].clone();
        let mut ids = BTreeMap::new();
        identity::correspond(&a, &mut b, &mut ids, "exact whole draft inverse");
        for (new, old) in &ids {
            let new: Value = serde_json::from_str(new).unwrap();
            assert_eq!(new["local"], old["local"], "no local ID renumbering");
        }
        let sa = read(old.join("sidecar.json"));
        let mut sb = read(new.join("sidecar.json"));
        preservation::authenticate(&sa, &old, &prior_path, case, &prior);
        preservation::authenticate(&sb, &new, &package, case, &next);
        assert_eq!(sa["allocator_after"], a["draft"]["allocator"]);
        assert_eq!(
            sb["allocator_after"],
            read(new.join("draft.json"))["draft"]["allocator"]
        );
        let imported = source(&bytes);
        let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
        let at = leaf(&evidence, "Placeholder", source_name);
        let issue = json!({"kind":"issue","value":a["draft"]["choice_presets"]["members"][0]["choices"]["completion"]["id"]});
        let scenario = json!({"kind":"scenario_preset","value":read(new.join("draft.json"))["draft"]["scenario_presets"]["members"][0]["id"]});
        assert_eq!(
            sa["origins"][at]["disposition"],
            json!({"kind":"contributes"})
        );
        assert_eq!(sa["origins"][at]["links"], json!([issue]));
        assert_eq!(
            sb["origins"][at]["disposition"],
            if projected {
                json!({"kind":"contributes"})
            } else {
                json!({"kind":"source_only","value":"unused-config-placeholder"})
            }
        );
        assert_eq!(sb["origins"][at]["links"], json!([scenario]));
        let counts_before = census(&sa, &evidence, &issue);
        let new_issue = json!({"kind":"issue","value":read(new.join("draft.json"))["draft"]["choice_presets"]["members"][0]["choices"]["completion"]["id"]});
        let counts_after = census(&sb, &evidence, &new_issue);
        assert_eq!(
            counts_after,
            (counts_before.0 - 1, counts_before.1 - 1, counts_before.2)
        );
        if case == 5 {
            assert_eq!(counts_before, spec.original05_before);
            assert_eq!(
                counts_after,
                (
                    spec.original05_before.0 - 1,
                    spec.original05_before.1 - 1,
                    spec.original05_before.2
                )
            );
        }
        sb["origins"][at]["disposition"] = sa["origins"][at]["disposition"].clone();
        sb["origins"][at]["links"] = json!([new_issue]);
        let old_items = sa["item_texts"].as_array().unwrap();
        let new_items = sb["item_texts"].as_array_mut().unwrap();
        assert_eq!(old_items.len(), new_items.len());
        for (old, new) in old_items.iter().zip(new_items) {
            metadata_lineage(&mut new["attribution"]["source"], &oldlineage, &newlineage);
            metadata_lineage(
                &mut new["attribution"]["source"]["allocator"],
                &oldlineage,
                &newlineage,
            );
            for field in ["policy", "item_lines"] {
                new["attribution"][field] = old["attribution"][field].clone();
            }
        }
        for field in [
            "policy",
            "mapping",
            "registry",
            "definitions",
            "skill_roles",
            "reward_policy",
            "item_policy",
            "item_source_policy",
            "tree_policy",
            "draft",
            "allocator_after",
        ] {
            sb[field] = sa[field].clone();
        }
        for field in ["allocator_before", "source_allocator"] {
            metadata_lineage(&mut sb[field], &oldlineage, &newlineage);
            assert_eq!(sb[field], sa[field]);
        }
        identity::correspond(&sa, &mut sb, &mut ids, "exact whole sidecar inverse");
        let mut selection = selected::selection(&bytes, &new);
        identity::relocate(&mut selection, &ids);
        assert_eq!(selection, selected::selection(&bytes, &old));
        let before = selected::finalize_with_definitions(
            &bytes,
            &old,
            &out.join(format!("prior-selected-{case:02}.json")),
            &prior_path.join("schema.json"),
        );
        let after = selected::finalize_with_definitions(
            &bytes,
            &new,
            &out.join(format!("selected-{case:02}.json")),
            &package.join("schema.json"),
        );
        assert_eq!(after["intent_validation"]["schema_issues"], json!([]));
        let mut issues = after["finalization"]["issues"].clone();
        identity::relocate(&mut issues, &ids);
        assert_eq!(issues, before["finalization"]["issues"]);
        assert_eq!(
            issues.as_array().unwrap().len(),
            [106, 117, 109, 123, 4][case - 1]
        );
        let queries = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(prior_path.join(&queries)).unwrap(),
            fs::read(package.join(&queries)).unwrap()
        );
        results.push(json!({"original":case,"added_assumptions":inputs.len(),"changed_leaf":at,"configuration_before":counts_before,"configuration_after":counts_after,"whole_draft_inverse":true,"whole_sidecar_inverse":true,"local_ids_unchanged":true,"selected_issues":issues.as_array().unwrap().len()}));
    }
    let original =
        fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let mut controls = vec![];
    let cases = if projected {
        vec![
            (
                "changed-placeholder",
                None,
                Some("80"),
                Some(vec![80., 80.]),
            ),
            ("zero", Some("0"), Some("80"), Some(vec![0., 80.])),
            ("negative-zero", Some("-0"), Some("80"), Some(vec![0., 80.])),
            (
                "fraction",
                Some("-12.5"),
                Some("80"),
                Some(vec![-12.5, -12.5]),
            ),
            ("nonzero", Some("50"), Some("80"), Some(vec![50., 50.])),
            ("zero-placeholder", None, Some("0"), Some(vec![0., 0.])),
            ("both-zero", Some("0"), Some("0"), Some(vec![0., 0.])),
            ("zero-constructor", Some("0"), None, Some(vec![0., default])),
            ("absent", None, None, Some(vec![default, default])),
            (
                "boundary",
                Some("1000000"),
                Some("-1000000"),
                Some(vec![1000000., 1000000.]),
            ),
            ("bad-placeholder", Some("0"), Some("NaN"), None),
            ("losing-outside-range", Some("50"), Some("1000001"), None),
            ("input-outside-range", Some("-1000001"), Some("20"), None),
        ]
    } else {
        [
            ("changed-placeholder", None, Some("999"), Some(default)),
            ("zero", Some("0"), Some("999"), Some(0.)),
            ("fraction", Some("-12.5"), Some("999"), Some(-12.5)),
            ("above-probability", Some("150"), Some("999"), Some(150.)),
            ("zero-placeholder", None, Some("0"), Some(default)),
            ("absent", None, None, Some(default)),
            ("bad-placeholder", Some("0"), Some("NaN"), None),
        ]
        .into_iter()
        .map(|(n, i, p, v)| (n, i, p, v.map(|v| vec![v])))
        .collect()
    };
    for (name, input, placeholder, expected_value) in cases {
        let xml = edits::field(
            &edits::field(
                &original,
                "Placeholder",
                source_name,
                placeholder.map(|v| ("number", v)),
            ),
            "Input",
            source_name,
            input.map(|v| ("number", v)),
        );
        controls.push((name, xml, expected_value));
    }
    for (name, key) in [
        ("explicit-none", "presetBossSkills"),
        ("boss-write", "enemyIsBoss"),
        ("size-write", "enemySizePreset"),
    ] {
        controls.push((
            name,
            edits::field(&original, "Input", key, Some(("string", "None"))),
            None,
        ));
    }
    for (name, xml, value) in &controls {
        let path = out.join(format!("probe-{name}.xml"));
        fs::write(&path, xml).unwrap();
        let target = out.join(format!("probe-{name}"));
        release::normalize(&package, &path, 5, &target);
        let mut draft = read(target.join("draft.json"));
        let values: Vec<_> = assumptions(&mut draft)
            .iter()
            .filter(|a| input_ids.contains(&a["input"]["value"]["key"]))
            .cloned()
            .collect();
        assert_eq!(
            values,
            value
                .as_ref()
                .map(|values| inputs
                    .iter()
                    .zip(values)
                    .map(|(i, v)| expected(i, *v))
                    .collect::<Vec<_>>())
                .unwrap_or_default(),
            "{name}"
        );
        if value.is_none() {
            let imported = source(xml.as_bytes());
            let e = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
            let sidecar = read(target.join("sidecar.json"));
            assert!(
                sidecar["origins"][leaf(&e, "Placeholder", source_name)]["links"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|l| l["kind"] == "issue"),
                "{name}"
            );
            if projected {
                for row in e.rows().iter().filter(|r| {
                    r.attribute("name")
                        .is_some_and(|a| a.decoded().ok() == Some(source_name))
                }) {
                    assert!(
                        sidecar["origins"][row.occurrence().id().ordinal() as usize]["links"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|l| l["kind"] == "issue"),
                        "{name}: every unaccounted saved lane stays pending"
                    );
                }
            }
        }
        if projected && value.is_some() {
            let imported = source(xml.as_bytes());
            let e = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
            let sidecar = read(target.join("sidecar.json"));
            let scenario = json!({"kind":"scenario_preset","value":draft["draft"]["scenario_presets"]["members"][0]["id"]});
            let authored = e
                .rows()
                .iter()
                .find(|r| {
                    r.occurrence().name() == "Input"
                        && r.attribute("name")
                            .is_some_and(|a| a.decoded().ok() == Some(source_name))
                })
                .map(|r| {
                    r.attribute("number")
                        .unwrap()
                        .decoded()
                        .unwrap()
                        .parse::<f64>()
                        .unwrap()
                });
            for row in e.rows().iter().filter(|r| {
                r.attribute("name")
                    .is_some_and(|a| a.decoded().ok() == Some(source_name))
            }) {
                let at = row.occurrence().id().ordinal() as usize;
                assert_eq!(
                    sidecar["origins"][at]["links"],
                    json!([scenario]),
                    "{name}: one exact link per leaf"
                );
                let selected =
                    row.occurrence().name() == "Input" || authored.is_none_or(|v| v == 0.);
                assert_eq!(
                    sidecar["origins"][at]["disposition"],
                    if selected {
                        json!({"kind":"contributes"})
                    } else {
                        json!({"kind":"source_only","value":"unselected-config-projection"})
                    },
                    "{name}: checked source role for each saved lane"
                );
            }
        }
    }
    assert_eq!(release::inventory(&prior_path), prior_files);
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":results,"mutation_controls":controls.len(),"queries":110,"new_definitions":spec.new_definitions,"new_programs":0,"rebuild_byte_identical":true,"inventory_completed":false,"complete_original_builds":0}),
    );
}
