//! Shared all-five exact preservation and real-input mutation checks.
use crate::selected;
#[allow(dead_code)]
#[path = "owned_configuration_publication_cases.rs"]
pub mod cases;
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
fn expected(input: &Value, target: &Value, v: f64) -> Value {
    let mut value = input["constructor_default"].clone();
    value["value"]["value"] = json!(v);
    json!({"input":{"kind":"known","value":input["value_input"]},"target":target,"value":{"kind":"known","value":value}})
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

#[derive(Clone, Copy)]
pub enum LeafRole {
    Contributes,
    IgnoredPlaceholder,
    UnselectedProjection,
}
impl LeafRole {
    fn disposition(self) -> Value {
        match self {
            Self::Contributes => json!({"kind":"contributes"}),
            Self::IgnoredPlaceholder => {
                json!({"kind":"source_only","value":"unused-config-placeholder"})
            }
            Self::UnselectedProjection => {
                json!({"kind":"source_only","value":"unselected-config-projection"})
            }
        }
    }
}
pub struct Output {
    pub policy_target: &'static str,
    pub draft_target: Value,
    pub selection: Option<&'static str>,
    pub ignore_placeholder: bool,
}
pub struct Control {
    pub name: &'static str,
    pub input: Option<&'static str>,
    pub placeholder: Option<&'static str>,
    pub values: Option<Vec<f64>>,
    /// Exact aggregate source role, independently supplied for each saved lane.
    pub roles: [Option<LeafRole>; 2],
}
pub struct Expectations {
    pub outputs: Vec<Output>,
    pub original_placeholder: LeafRole,
    pub default: f64,
    pub controls: Vec<Control>,
}
pub struct Publication<E = Expectations> {
    pub directory: PathBuf,
    pub stage: fn(&StagedOwnedRelease) -> StagedOwnedRelease,
    pub prior: PathBuf,
    pub output: PathBuf,
    pub original05_before: (usize, usize, usize),
    pub new_definitions: usize,
    pub expectations: E,
}
pub fn run(spec: Publication) {
    let inputs = crate::configuration_stage::inputs(read(spec.directory.join("input.json")));
    let source_name = inputs[0].source_name.clone();
    assert!(inputs.iter().all(|input| input.source_name == source_name));
    run_sources(Publication {
        directory: spec.directory,
        stage: spec.stage,
        prior: spec.prior,
        output: spec.output,
        original05_before: spec.original05_before,
        new_definitions: spec.new_definitions,
        expectations: vec![SourceExpectations {
            source_name,
            expected: spec.expectations,
        }],
    });
}
pub struct SourceExpectations {
    pub source_name: String,
    pub expected: Expectations,
}
pub fn checked_group_ranges(
    inputs: &[Value],
    groups: &[SourceExpectations],
) -> Vec<std::ops::Range<usize>> {
    let mut group_ranges = vec![];
    let mut cursor = 0;
    let mut seen = std::collections::BTreeSet::new();
    let mut seen_ids = std::collections::BTreeSet::new();
    for group in groups {
        assert!(
            seen.insert(group.source_name.as_str()),
            "duplicate expected source"
        );
        let end = cursor + group.expected.outputs.len();
        assert!(
            end > cursor && end <= inputs.len(),
            "exact nonempty source group"
        );
        let first = &inputs[cursor];
        assert_eq!(
            first["constructor_default"]["value"]["value"].as_f64(),
            Some(group.expected.default)
        );
        for (input, output) in inputs[cursor..end].iter().zip(&group.expected.outputs) {
            assert_eq!(input["source_name"], group.source_name);
            assert_eq!(input["constructor_default"], first["constructor_default"]);
            assert!(
                seen_ids.insert(input["value_input"]["key"].as_str().unwrap()),
                "distinct outputs across all source groups"
            );
            assert_eq!(input["target"], output.policy_target);
            assert_eq!(
                input.get("selection").and_then(Value::as_str),
                output.selection
            );
            assert_eq!(
                input
                    .get("ignore_numeric_placeholder")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                output.ignore_placeholder
            );
        }
        group_ranges.push(cursor..end);
        cursor = end;
    }
    assert!(!groups.is_empty());
    assert_eq!(
        cursor,
        inputs.len(),
        "every authored output has exactly one expectation"
    );
    group_ranges
}
pub fn run_sources(spec: Publication<Vec<SourceExpectations>>) {
    let inputs: Vec<Value> =
        crate::configuration_stage::inputs(read(spec.directory.join("input.json")))
            .into_iter()
            .map(|v| serde_json::to_value(v).unwrap())
            .collect();
    // Test expectations are explicit. Neither source name nor vector length
    // selects semantics, target ownership or source-accounting outcomes.
    let groups = spec.expectations;
    let group_ranges = checked_group_ranges(&inputs, &groups);
    let defaults: Vec<_> = groups
        .iter()
        .zip(&group_ranges)
        .flat_map(|(g, r)| {
            inputs[r.clone()]
                .iter()
                .zip(&g.expected.outputs)
                .map(|(i, o)| expected(i, &o.draft_target, g.expected.default))
        })
        .collect();
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
            defaults
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
        let issue = json!({"kind":"issue","value":a["draft"]["choice_presets"]["members"][0]["choices"]["completion"]["id"]});
        let scenario = json!({"kind":"scenario_preset","value":read(new.join("draft.json"))["draft"]["scenario_presets"]["members"][0]["id"]});
        let mut changed_leaves = vec![];
        for group in &groups {
            let at = leaf(&evidence, "Placeholder", &group.source_name);
            assert!(
                !changed_leaves.contains(&at),
                "source leaf expectations may not overlap"
            );
            changed_leaves.push(at);
            assert_eq!(
                sa["origins"][at]["disposition"],
                json!({"kind":"contributes"})
            );
            assert_eq!(sa["origins"][at]["links"], json!([issue]));
            assert_eq!(
                sb["origins"][at]["disposition"],
                group.expected.original_placeholder.disposition()
            );
            assert_eq!(sb["origins"][at]["links"], json!([scenario]));
        }
        let counts_before = census(&sa, &evidence, &issue);
        let new_issue = json!({"kind":"issue","value":read(new.join("draft.json"))["draft"]["choice_presets"]["members"][0]["choices"]["completion"]["id"]});
        let counts_after = census(&sb, &evidence, &new_issue);
        assert_eq!(
            counts_after,
            (
                counts_before.0 - groups.len(),
                counts_before.1 - groups.len(),
                counts_before.2
            )
        );
        if case == 5 {
            assert_eq!(counts_before, spec.original05_before);
            assert_eq!(
                counts_after,
                (
                    spec.original05_before.0 - groups.len(),
                    spec.original05_before.1 - groups.len(),
                    spec.original05_before.2
                )
            );
        }
        for &at in &changed_leaves {
            sb["origins"][at]["disposition"] = sa["origins"][at]["disposition"].clone();
            sb["origins"][at]["links"] = json!([new_issue]);
        }
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
        results.push(json!({"original":case,"added_assumptions":inputs.len(),"changed_leaves":changed_leaves,"configuration_before":counts_before,"configuration_after":counts_after,"whole_draft_inverse":true,"whole_sidecar_inverse":true,"local_ids_unchanged":true,"selected_issues":issues.as_array().unwrap().len()}));
    }
    let original =
        fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let mut control_count = 0;
    for (group, range) in groups.into_iter().zip(group_ranges) {
        let source_name = group.source_name.as_str();
        let inputs = &inputs[range.clone()];
        let input_ids: Vec<_> = inputs
            .iter()
            .map(|i| i["value_input"]["key"].clone())
            .collect();
        let expectation = group.expected;
        let mut controls = vec![];
        for case in expectation.controls {
            let Control {
                name,
                input,
                placeholder,
                values,
                roles,
            } = case;
            assert_eq!(roles[0].is_some(), values.is_some() && input.is_some());
            assert_eq!(
                roles[1].is_some(),
                values.is_some() && placeholder.is_some()
            );
            if let Some(values) = &values {
                assert_eq!(values.len(), inputs.len());
            }
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
            controls.push((name, xml, values, roles));
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
                [None, None],
            ));
        }
        for (name, xml, value, roles) in &controls {
            let qualified = format!("{}-{name}", group.source_name);
            let path = out.join(format!("probe-{qualified}.xml"));
            fs::write(&path, xml).unwrap();
            let target = out.join(format!("probe-{qualified}"));
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
                        .zip(&expectation.outputs)
                        .zip(values)
                        .map(|((i, o), v)| expected(i, &o.draft_target, *v))
                        .collect::<Vec<_>>())
                    .unwrap_or_default(),
                "{name}"
            );
            // An ordinary source mutation may remove only this source's outputs.
            // The three Encounter guard probes intentionally invalidate all of them.
            let unrelated: Vec<_> = assumptions(&mut draft)
                .iter()
                .filter(|a| {
                    defaults.iter().any(|d| d["input"] == a["input"])
                        && !input_ids.contains(&a["input"]["value"]["key"])
                })
                .cloned()
                .collect();
            let guards = ["explicit-none", "boss-write", "size-write"];
            let expected_unrelated = if guards.contains(name) {
                vec![]
            } else {
                defaults
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| !range.contains(index))
                    .map(|(_, d)| d.clone())
                    .collect()
            };
            assert_eq!(
                unrelated, expected_unrelated,
                "{qualified}: unrelated input projections remain exact"
            );
            let imported = source(xml.as_bytes());
            let e = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
            let sidecar = read(target.join("sidecar.json"));
            let scenario = json!({"kind":"scenario_preset","value":draft["draft"]["scenario_presets"]["members"][0]["id"]});
            for (lane, element) in ["Input", "Placeholder"].into_iter().enumerate() {
                let rows: Vec<_> = e
                    .rows()
                    .iter()
                    .filter(|r| {
                        r.occurrence().name() == element
                            && r.attribute("name")
                                .is_some_and(|a| a.decoded().ok() == Some(source_name))
                    })
                    .collect();
                assert!(rows.len() <= 1);
                if value.is_some() {
                    assert_eq!(
                        rows.len(),
                        usize::from(roles[lane].is_some()),
                        "{name}: saved lane inventory"
                    );
                }
                for row in rows {
                    let origin = &sidecar["origins"][row.occurrence().id().ordinal() as usize];
                    if value.is_none() {
                        assert!(
                            origin["links"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .any(|l| l["kind"] == "issue"),
                            "{name}: every saved lane stays pending"
                        );
                    } else {
                        assert_eq!(
                            origin["links"],
                            json!([scenario]),
                            "{name}: one exact link per leaf"
                        );
                        assert_eq!(
                            origin["disposition"],
                            roles[lane].unwrap().disposition(),
                            "{name}: exact checked source role"
                        );
                    }
                }
            }
        }
        control_count += controls.len();
    }
    assert_eq!(release::inventory(&prior_path), prior_files);
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":results,"mutation_controls":control_count,"queries":110,"new_definitions":spec.new_definitions,"new_programs":0,"rebuild_byte_identical":true,"inventory_completed":false,"complete_original_builds":0}),
    );
}
