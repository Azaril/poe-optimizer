//! Requested metric identities are independent from numerical result authority.
#[path = "support/owned_requested_metrics.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use identity::{correspond, relocate};
use poe_optimizer_core::owned_schema::{DefinitionDescriptor, SchemaState};
use poe_optimizer_import::{
    owned_mapping::OwnedMappingIndex,
    owned_recipe_extension::{SchemaExtensionEntry, extend_owned_recipe},
    owned_release::StagedOwnedRelease,
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
fn metric_rows() -> Vec<Value> {
    family::bindings()["metrics"].as_array().unwrap().clone()
}

#[test]
fn authored_identities_have_exact_units_roles_and_selectors_without_computed_bindings() {
    let e = family::extension();
    let mappings = family::mappings();
    let rows = metric_rows();
    assert_eq!(e.schema.len(), 21);
    assert_eq!(mappings.len(), 20);
    assert_eq!(rows.len(), 20);
    assert!(e.owners.is_empty() && e.tables.is_empty() && e.receivers.is_empty());
    assert!(e.operations_version.is_none());
    let mut names = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut units = 0;
    let ledger = family::bindings();
    for entry in &e.schema {
        match entry {
            SchemaExtensionEntry::Definition(DefinitionDescriptor::Unit(d)) => {
                units += 1;
                assert_eq!(
                    serde_json::to_value(&d.id).unwrap(),
                    ledger["new_unit"]["id"]
                );
                assert_eq!(
                    serde_json::to_value(&d.schema).unwrap(),
                    json!({"kind":"known","value":{"dimension":"rate"}})
                );
            }
            SchemaExtensionEntry::Definition(DefinitionDescriptor::Metric(d)) => {
                let id = serde_json::to_value(&d.id).unwrap();
                let row = rows.iter().find(|r| r["metric"] == id).unwrap();
                assert!(ids.insert(id["key"].as_str().unwrap().to_owned()));
                let name = row["name"].as_str().unwrap();
                assert!(names.insert(name.to_owned()));
                let SchemaState::Known(s) = &d.schema else {
                    panic!("declared metric must be Known")
                };
                assert_eq!(
                    serde_json::to_value(s).unwrap(),
                    json!({"targets":row["targets"],"unit":row["unit"],"actor_roles":row["actor_roles"],"provider_roles":row["provider_roles"]})
                );
                assert_eq!(row["calculation_binding"], "absent");
                let selected = matches!(name, "selected_hit_dps" | "selected_average_hit");
                assert_eq!(
                    row["targets"],
                    if selected {
                        json!(["actor", "action"])
                    } else {
                        json!(["actor"])
                    }
                );
                assert_eq!(
                    row["actor_roles"],
                    if selected {
                        json!(["player", "owned"])
                    } else {
                        json!(["player"])
                    }
                );
                assert_eq!(
                    row["provider_roles"],
                    if selected {
                        json!(["character", "skill_use"])
                    } else {
                        json!(["character"])
                    }
                );
                let expected_unit = match name {
                    "life" => "3119",
                    "mana" => "0003",
                    "spirit" => "0004",
                    "energy_shield" => "29ed",
                    "armour" | "evasion" => "29ee",
                    "selected_hit_dps" => "312c",
                    "pob_total_ehp"
                    | "physical_max_hit"
                    | "fire_max_hit"
                    | "cold_max_hit"
                    | "lightning_max_hit"
                    | "chaos_max_hit"
                    | "selected_average_hit" => "1d3a",
                    "fire_resistance_capped_pct"
                    | "cold_resistance_capped_pct"
                    | "lightning_resistance_capped_pct"
                    | "chaos_resistance_capped_pct"
                    | "movement_speed_pct"
                    | "action_speed_pct" => "0002",
                    _ => panic!("unreviewed metric identity {name}"),
                };
                assert_eq!(
                    row["unit"]["key"],
                    format!("def.000000000000{expected_unit}")
                );
                let matches: Vec<_> = mappings
                    .iter()
                    .map(|v| serde_json::to_value(v).unwrap())
                    .filter(|v| v["source"]["value"]["key"]["value"] == name)
                    .collect();
                assert_eq!(
                    matches,
                    vec![
                        json!({"source":{"kind":"catalog","value":{"kind":"metric","key":{"kind":"text","value":name},"version":{"kind":"missing"},"variant":{"kind":"missing"}}},"outcome":{"kind":"mapped","value":{"target":{"kind":"definition","value":{"kind":"metric","value":id}},"basis":{"kind":"exact"}}}})
                    ]
                );
            }
            _ => panic!("no other definitions or slots are part of this identity publication"),
        }
    }
    assert_eq!(units, 1);
    assert_eq!(names.len(), 20);
    let a = family::authoring();
    let c = &a["semantic_catalog"];
    assert_eq!(c["hash_encoding"], "utf8_lf");
    let text = fs::read_to_string(root().join(c["path"].as_str().unwrap()))
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(
        format!("{:x}", Sha256::digest(text.as_bytes())),
        c["sha256"]
    );
    for row in rows {
        // This is a frozen authoring reference, not a runtime parser or oracle.
        let entry = text
            .split("    Binding {")
            .find(|s| s.contains(&format!("id: \"{}\"", row["name"].as_str().unwrap())))
            .unwrap();
        let entry = entry.split("    },").next().unwrap();
        assert!(entry.contains(&format!(
            "raw: \"{}\"",
            row["reference"]["raw_field"].as_str().unwrap()
        )));
        assert!(entry.contains(&format!(
            "description: \"{}\"",
            row["meaning"].as_str().unwrap()
        )));
        let scale = row["reference"]["scale"].as_f64().unwrap();
        assert!(entry.contains(&format!("value_scale: {scale:.1}")));
        assert!(entry.contains(&format!(
            "unit: MetricUnit::{}",
            row["reference"]["unit"].as_str().unwrap()
        )));
    }
}

fn query_members(v: &Value) -> &Vec<Value> {
    v["draft"]["query_presets"]["members"][0]["queries"]["requests"]["members"]
        .as_array()
        .unwrap()
}
fn query_members_mut(v: &mut Value) -> &mut Vec<Value> {
    v["draft"]["query_presets"]["members"][0]["queries"]["requests"]["members"]
        .as_array_mut()
        .unwrap()
}
fn compare(
    case: usize,
    xml: &[u8],
    templates: &[Value],
    old: &Path,
    new: &Path,
    out: &Path,
) -> Value {
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    for v in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(v);
    }
    assert_eq!(sa["schema_version"], 13);
    assert_eq!(sb["schema_version"], 13);
    let pa = a["draft"]["allocator"].clone();
    let na = b["draft"]["allocator"].clone();
    let issued = |v: &Value| u64::from_str_radix(v["last_issued"].as_str().unwrap(), 16).unwrap();
    assert_eq!(issued(&pa) - issued(&na), 22);
    b["draft"]["allocator"] = pa.clone();
    let bindings = metric_rows();
    let mut retired = BTreeSet::new();
    let mut target_counts = BTreeMap::<String, usize>::new();
    assert_eq!(templates.len(), 22);
    assert_eq!(query_members(&a).len(), 22);
    assert_eq!(query_members(&b).len(), 22);
    for (x, y) in query_members_mut(&mut a)
        .iter_mut()
        .zip(query_members_mut(&mut b))
    {
        assert_eq!(x["id"], y["id"]);
        let source = templates.iter().find(|v| v["id"] == x["id"]).unwrap();
        let name = &source["metric"]["value"]["key"]["value"];
        let row = bindings.iter().find(|v| v["name"] == *name).unwrap();
        assert_eq!(x["metric"]["kind"], "pending");
        assert_eq!(x["metric"]["code"], "definition-unmapped");
        assert_eq!(x["metric"]["candidates"], json!([]));
        assert!(retired.insert(x["metric"]["id"]["local"].as_str().unwrap().to_owned()));
        assert_eq!(y["metric"], json!({"kind":"known","value":row["metric"]}));
        *target_counts
            .entry(source["target"]["kind"].as_str().unwrap().to_owned())
            .or_default() += 1;
        // Whole-draft correspondence below compares every target, full provider
        // path and repeated reference after removing only the changed metric.
        x.as_object_mut().unwrap().remove("metric");
        y.as_object_mut().unwrap().remove("metric");
    }
    assert_eq!(retired.len(), 22);
    let mut ids = BTreeMap::new();
    correspond(
        &a,
        &mut b,
        &mut ids,
        "only requested metric identities change",
    );
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
    assert_eq!(removed, 22);
    let old_rows = sa["item_texts"].as_array().unwrap();
    let new_rows = sb["item_texts"].as_array_mut().unwrap();
    assert_eq!(old_rows.len(), new_rows.len());
    for (x, y) in old_rows.iter().zip(new_rows) {
        assert_eq!(x["source"], y["source"]);
        y["attribution"]["policy"] = x["attribution"]["policy"].clone();
        y["attribution"]["item_lines"] = x["attribution"]["item_lines"].clone();
    }
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
        "all source facts and retained issue links preserved",
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
    let old_metric_issues: Vec<_> = x
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| retired.contains(r["id"]["local"].as_str().unwrap()))
        .collect();
    assert_eq!(old_metric_issues.len(), 22);
    for issue in old_metric_issues {
        assert_eq!(issue["code"], "definition-unmapped");
    }
    x.as_array_mut()
        .unwrap()
        .retain(|r| !retired.contains(r["id"]["local"].as_str().unwrap()));
    assert_eq!(x, y);
    let expected = [317, 322, 314, 380, 142][case - 1];
    assert_eq!(count, expected);
    assert_eq!(y.as_array().unwrap().len(), expected - 22);
    let mut first = selected::selection(xml, old);
    let mut second = selected::selection(xml, new);
    selected::canonical(&mut first);
    selected::canonical(&mut second);
    relocate(&mut second, &ids);
    assert_eq!(first, second);
    json!({"original":case,"selected_before":count,"selected_after":expected-22,"retired":retired,"target_counts":target_counts,"allocator_before":pa,"allocator_after":na,"selected_issue_summary":after["selected_issue_summary"]})
}

fn negative_bindings(prior: &StagedOwnedRelease, next: &StagedOwnedRelease) -> usize {
    let mut duplicate = next.input().mapping.clone();
    duplicate.entries.push(family::mappings()[0].clone());
    assert!(
        OwnedMappingIndex::new(
            duplicate,
            next.assembled().registry(),
            next.assembled().schema(),
            Default::default()
        )
        .is_err()
    );
    let mut wrong_kind = serde_json::to_value(&next.input().mapping).unwrap();
    let source = serde_json::to_value(&family::mappings()[0].source).unwrap();
    let entry = wrong_kind["entries"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|v| v["source"] == source)
        .unwrap();
    entry["outcome"]["value"]["target"] = json!({"kind":"definition","value":{"kind":"unit","value":family::bindings()["new_unit"]["id"]}});
    assert!(
        OwnedMappingIndex::new(
            serde_json::from_value(wrong_kind).unwrap(),
            next.assembled().registry(),
            next.assembled().schema(),
            Default::default()
        )
        .is_err()
    );
    for (field, value) in [
        ("game", "foreign-metric-game"),
        ("key", "def.ffffffffffffffff"),
    ] {
        let mut e = serde_json::to_value(family::extension()).unwrap();
        let unit = &mut e["schema"][1]["value"]["value"]["schema"]["value"]["unit"];
        if field == "game" {
            unit["namespace"][field] = json!(value);
        } else {
            unit[field] = json!(value);
        }
        assert!(
            extend_owned_recipe(
                prior.assembled(),
                &serde_json::from_value(e).unwrap(),
                Default::default()
            )
            .is_err(),
            "foreign or missing exact unit"
        );
    }
    let mut stale = next.input().mapping.clone();
    stale.definitions = prior.receipt().definitions.clone();
    assert!(
        OwnedMappingIndex::new(
            stale,
            next.assembled().registry(),
            next.assembled().schema(),
            Default::default()
        )
        .is_err()
    );
    5
}
fn unknown_query_probe(package: &Path, out: &Path) {
    let mut queries: Value = read(package.join("queries-original-05.json"));
    queries[0]["metric"]["value"]["key"]["value"] = json!("unreviewed-metric-name");
    let path = out.join("unknown-query.json");
    write(&path, &queries);
    let dest = out.join("unknown-query");
    let mut c = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    c.arg("normalize-owned")
        .arg(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"));
    for (flag, file) in [
        ("--policy", "normalization.json"),
        ("--registry", "registry.json"),
        ("--definitions", "schema.json"),
        ("--mapping", "mapping.json"),
        ("--roles", "roles.json"),
        ("--rewards", "rewards.json"),
        ("--items", "items.json"),
        ("--item-source", "item-source.json"),
        ("--tree-policy", "tree-normalization.json"),
    ] {
        c.arg(flag).arg(package.join(file));
    }
    let output = c
        .arg("--queries")
        .arg(path)
        .arg("--output")
        .arg(&dest)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["verification"]["calculation"], "not_run");
    let draft: Value = read(dest.join("draft.json"));
    let queries = query_members(&draft);
    assert_eq!(queries[0]["metric"]["kind"], "pending");
    assert_eq!(queries[0]["metric"]["code"], "definition-unmapped");
    assert!(queries[1..].iter().all(|q| q["metric"]["kind"] == "known"));
}

#[test]
#[ignore = "requires the exact checked ordinary-item-input predecessor"]
fn real_metric_identities_preserve_original_requests_and_leave_calculations_absent() {
    let p = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_REQUESTED_METRICS_PRIOR").expect("explicit prior"),
    );
    let before = release::inventory(&p);
    let prior = release::load(&p);
    let next = family::stage(&prior);
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_REQUESTED_METRICS_OUTPUT")
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
    let a = family::authoring();
    for row in a["query_files"].as_array().unwrap() {
        let case = row["original"].as_u64().unwrap();
        let name = format!("queries-original-{case:02}.json");
        let bytes = fs::read(p.join(&name)).unwrap();
        assert_eq!(bytes.len() as u64, row["bytes"].as_u64().unwrap());
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), row["sha256"]);
        assert_eq!(inv.get(&name), before.get(&name));
        assert_eq!(bytes, fs::read(package.join(name)).unwrap());
    }
    let negatives = negative_bindings(&prior, &next);
    let mut reports = vec![];
    let mut targets = BTreeMap::<String, usize>::new();
    for case in 1..=5 {
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&p, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        let queries: Vec<Value> = read(p.join(format!("queries-original-{case:02}.json")));
        let report = compare(case, &fs::read(xml).unwrap(), &queries, &old, &new, &out);
        for (kind, n) in report["target_counts"].as_object().unwrap() {
            *targets.entry(kind.clone()).or_default() += n.as_u64().unwrap() as usize;
        }
        reports.push(report);
    }
    assert_eq!(
        serde_json::to_value(&targets).unwrap(),
        json!({"player":98,"action":4,"unresolved":8})
    );
    unknown_query_probe(&package, &out);
    assert_eq!(before, release::inventory(&p));
    assert!(next.input().evaluation.is_none());
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"definitions":next.receipt().definitions,"registry":next.receipt().registry,"prior_provenance":prior.input().provenance.len(),"final_provenance":next.input().provenance.len(),"queries":next.receipt().query_rows,"metric_definitions":20,"new_units":1,"new_mapping_rows":20,"target_counts":targets,"originals":reports,"invalid_binding_rejections":negatives,"unknown_metric_probe":"pending","prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0,"numerical_metric_bindings":0}),
    );
}
