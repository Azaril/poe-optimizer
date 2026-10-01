//! Real saved paths supply access classification, never cost or legality proof.
#[path = "support/owned_allocation_access.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use identity::{correspond, relocate};
use poe_optimizer_core::owned_definitions::PassiveNodeDefId;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_release::assemble_owned_release,
    owned_source::{SourceEvidenceLimits, SourceEvidenceRow, SourceProjectEvidence},
    owned_tree_policy::{AllocationAccessPolicy, AllocationRootKind},
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
fn pinned(a: &Value, field: &str) -> Value {
    let p = &a[field];
    assert_eq!(p["hash_encoding"], "utf8_lf");
    let s = fs::read_to_string(root().join(p["path"].as_str().unwrap()))
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(format!("{:x}", Sha256::digest(s.as_bytes())), p["sha256"]);
    serde_json::from_str(&s).unwrap()
}
fn catalogue() -> Value {
    read(root().join("data/owned/poe2/3887ae68/tree/tree-catalog.json"))
}
fn reviewed_tokens(c: &Value, facts: &Value) -> BTreeSet<String> {
    let excluded: BTreeSet<_> = facts["unconverted_node_fields"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| {
            r["fields"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f == "isFreeAllocate" || f == "aliasPassiveSocket")
        })
        .map(|r| r["node"].as_str().unwrap().to_owned())
        .collect();
    c["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| {
            matches!(n["kind"]["kind"].as_str(), Some("allocation" | "attribute"))
                && n["unlock"].as_array().unwrap().is_empty()
                && !excluded.contains(n["key"].as_str().unwrap())
        })
        .map(|n| n["key"].as_str().unwrap().to_owned())
        .collect()
}
#[test]
fn access_policy_is_the_catalogue_family_and_preserves_distinct_pool_roots() {
    let a = family::authoring();
    let c = pinned(&a, "source_catalog");
    let facts = pinned(&a, "source_facts");
    let tokens = pinned(&a, "token_mapping");
    let manifest = pinned(&a, "source_manifest");
    assert_eq!(c["source"]["revision"], a["source_revision"]);
    assert_eq!(manifest["upstream_revision"], a["source_revision"]);
    for pin in a["source_files"].as_array().unwrap() {
        let matches: BTreeSet<_> = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .chain(c["source"]["files"].as_array().unwrap())
            .filter(|p| p["path"] == pin["path"])
            .map(|p| p["sha256"].as_str().unwrap())
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(*matches.first().unwrap(), pin["sha256"].as_str().unwrap());
    }
    assert!(root().join(a["source_test"].as_str().unwrap()).is_file());
    let expected = reviewed_tokens(&c, &facts);
    assert_eq!(expected.len(), 4316);
    let AllocationAccessPolicy::PobIndependentSavedPathsV1 { pools, nodes } = family::policy();
    let actual: BTreeSet<_> = nodes.into_iter().collect();
    assert_eq!(actual.len(), 4316);
    let mut joined = BTreeSet::new();
    let mut pool_roles = BTreeMap::new();
    for row in tokens["content"]["tokens"].as_array().unwrap() {
        let token = row["token"].as_str().unwrap();
        if !expected.contains(token) {
            continue;
        }
        assert_eq!(row["role"]["kind"], "allocation");
        let id: PassiveNodeDefId =
            serde_json::from_value(row["role"]["value"]["node"].clone()).unwrap();
        assert!(actual.contains(&id));
        assert!(joined.insert(id));
        let n = c["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["key"] == token)
            .unwrap();
        let role = n["kind"]["value"]["pool"].as_str().unwrap();
        let pool = row["role"]["value"]["pool"]["key"].as_str().unwrap();
        if let Some(old) = pool_roles.insert(pool.to_owned(), role.to_owned()) {
            assert_eq!(old, role);
        }
    }
    assert_eq!(joined, actual);
    assert_eq!(pools.len(), 2);
    for p in pools {
        assert_eq!(
            pool_roles[p.pool.key().as_str()],
            match p.root {
                AllocationRootKind::Class => "ordinary",
                AllocationRootKind::Ascendancy => "ascendancy",
            }
        );
    }
    let domain = c["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| expected.contains(n["key"].as_str().unwrap()));
    assert_eq!(
        domain
            .clone()
            .filter(|n| n["kind"]["kind"] == "attribute")
            .count(),
        293
    );
    assert_eq!(
        domain
            .filter(|n| !n["views"].as_array().unwrap().is_empty())
            .count(),
        77
    );
    assert_eq!(a["new_definitions"], 0);
    assert_eq!(a["new_programs"], 0);
}
fn attr<'a>(r: &'a SourceEvidenceRow<'_>, name: &str) -> &'a str {
    r.attribute(name).unwrap().decoded().unwrap()
}
fn csv(text: &str) -> BTreeSet<String> {
    if text.is_empty() {
        return BTreeSet::new();
    }
    text.split(',').map(str::to_owned).collect()
}
/// Independently traverse the pinned source graph, not the Import proof output.
/// Called only on unchanged originals with the separately witnessed field grammar.
fn expected_access(xml: &[u8], draft: &Value, sidecar: &Value) -> BTreeSet<String> {
    let lineage = serde_json::from_value(draft["draft"]["allocator"]["lineage"].clone()).unwrap();
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        lineage,
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let c = catalogue();
    let facts: Value = read(root().join("data/owned/poe2/3887ae68/tree/source-facts.json"));
    let admitted = reviewed_tokens(&c, &facts);
    let nodes: BTreeMap<_, _> = c["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| (n["key"].as_str().unwrap(), n))
        .collect();
    let token_map: Value =
        read(root().join("data/owned/poe2/3887ae68/current/tree-normalization.json"));
    let definition_tokens: BTreeMap<_, _> = token_map["content"]["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["role"]["kind"] == "allocation")
        .map(|t| {
            (
                t["role"]["value"]["node"]["key"].as_str().unwrap(),
                t["token"].as_str().unwrap(),
            )
        })
        .collect();
    let mut result = BTreeSet::new();
    for spec in evidence
        .rows()
        .iter()
        .filter(|r| r.occurrence().name() == "Spec")
    {
        let class = c["classes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["key"] == attr(spec, "classInternalId"))
            .unwrap();
        let asc = class["ascendancies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["key"] == attr(spec, "ascendancyInternalId"))
            .unwrap();
        let saved = csv(attr(spec, "nodes"));
        let mut scoped = BTreeSet::new();
        for child in spec.children() {
            let row = &evidence.rows()[child.ordinal() as usize];
            if matches!(row.occurrence().name(), "WeaponSet1" | "WeaponSet2") {
                scoped.extend(csv(attr(row, "nodes")));
            }
        }
        let origin = sidecar["origins"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["source"]["ordinal"] == spec.occurrence().id().ordinal())
            .unwrap();
        let ids: BTreeSet<_> = origin["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["kind"] == "allocation")
            .map(|v| v["value"]["local"].as_str().unwrap())
            .collect();
        let mut reachable = BTreeSet::new();
        for (pool, anchor) in [("ordinary", &class["root"]), ("ascendancy", &asc["root"])] {
            let anchor = anchor.as_str().unwrap();
            let allowed: BTreeSet<_> = saved
                .iter()
                .filter(|n| {
                    admitted.contains(*n)
                        && !scoped.contains(*n)
                        && nodes[n.as_str()]["kind"]["value"]["pool"] == pool
                })
                .cloned()
                .collect();
            let mut reached = BTreeSet::from([anchor.to_owned()]);
            loop {
                let old = reached.len();
                for edge in c["edges"].as_array().unwrap() {
                    let left = edge["left"].as_str().unwrap();
                    let right = edge["right"].as_str().unwrap();
                    if reached.contains(left) && allowed.contains(right) {
                        reached.insert(right.to_owned());
                    }
                    if reached.contains(right) && allowed.contains(left) {
                        reached.insert(left.to_owned());
                    }
                }
                if old == reached.len() {
                    break;
                }
            }
            reachable.extend(reached.into_iter().filter(|n| allowed.contains(n)));
        }
        for a in draft["draft"]["allocations"]["members"].as_array().unwrap() {
            if !ids.contains(a["id"]["local"].as_str().unwrap()) {
                continue;
            }
            assert_eq!(a["node"]["kind"], "known");
            let token = definition_tokens[a["node"]["value"]["key"].as_str().unwrap()];
            if reachable.contains(token)
                && a["choices"]["completion"]["kind"] == "complete"
                && a["choices"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|member| {
                        member["slot"]["kind"] == "known" && member["value"]["kind"] == "known"
                    })
            {
                result.insert(a["id"]["local"].as_str().unwrap().to_owned());
            }
        }
    }
    result
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
fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    let expected = expected_access(xml, &a, &sa);
    for v in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(v);
    }
    let pa = a["draft"]["allocator"].clone();
    let na = b["draft"]["allocator"].clone();
    let mut retired = BTreeSet::new();
    let aa = a["draft"]["allocations"]["members"].as_array_mut().unwrap();
    let bb = b["draft"]["allocations"]["members"].as_array_mut().unwrap();
    assert_eq!(aa.len(), bb.len());
    for (old, new) in aa.iter_mut().zip(bb) {
        if expected.contains(old["id"]["local"].as_str().unwrap()) {
            assert_eq!(old["access"]["kind"], "pending");
            assert_eq!(
                old["access"]["value"]["code"],
                "allocation-access-not-converted"
            );
            assert_eq!(new["access"], json!({"kind":"ordinary"}));
            assert!(
                retired.insert(
                    old["access"]["value"]["id"]["local"]
                        .as_str()
                        .unwrap()
                        .to_owned()
                )
            );
            old.as_object_mut().unwrap().remove("access");
            new.as_object_mut().unwrap().remove("access");
        } else {
            assert_eq!(new["access"]["kind"], "pending");
        }
    }
    assert_eq!(retired.len(), expected.len());
    let issued = |v: &Value| u64::from_str_radix(v["last_issued"].as_str().unwrap(), 16).unwrap();
    assert_eq!(issued(&pa) - issued(&na), expected.len() as u64);
    b["draft"]["allocator"] = pa.clone();
    let mut ids = BTreeMap::new();
    correspond(
        &a,
        &mut b,
        &mut ids,
        "only proven access classifications change",
    );
    let mut removed = 0;
    for row in sa["origins"].as_array_mut().unwrap() {
        row["links"].as_array_mut().unwrap().retain(|l| {
            let remove = l["kind"] == "issue"
                && l["value"]["local"]
                    .as_str()
                    .is_some_and(|id| retired.contains(id));
            removed += usize::from(remove);
            !remove
        });
    }
    assert_eq!(removed, retired.len());
    for field in ["draft", "tree_policy"] {
        sb[field] = sa[field].clone();
    }
    assert_eq!(sa["allocator_after"], pa);
    assert_eq!(sb["allocator_after"], na);
    sb["allocator_after"] = pa.clone();
    correspond(
        &sa,
        &mut sb,
        &mut ids,
        "unchanged source evidence and retained links",
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
        .retain(|v| !retired.contains(v["id"]["local"].as_str().unwrap()));
    assert_eq!(x, y);
    let mut first = selected::selection(xml, old);
    let mut second = selected::selection(xml, new);
    selected::canonical(&mut first);
    selected::canonical(&mut second);
    relocate(&mut second, &ids);
    assert_eq!(first, second);
    json!({"original":case,"full_access_classifications":expected.len(),"selected_before":count,"selected_after":y.as_array().unwrap().len(),"selected_access_classifications":count-y.as_array().unwrap().len(),"allocator_before":pa,"allocator_after":na,"retired":retired,"remaining_by_code":after["selected_issue_summary"]["by_code"]})
}

fn access_probe(
    prior: &Path,
    package: &Path,
    out: &Path,
    label: &str,
    xml: &str,
    spec_ordinal: u32,
    ordinary_must_stay_pending: bool,
) {
    let path = out.join(format!("probe-{label}.xml"));
    fs::write(&path, xml).unwrap();
    let old = out.join(format!("probe-{label}-prior"));
    let new = out.join(format!("probe-{label}"));
    release::normalize(prior, &path, 5, &old);
    release::normalize(package, &path, 5, &new);
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let sidecar: Value = read(new.join("sidecar.json"));
    let scope = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["source"]["ordinal"] == spec_ordinal)
        .unwrap();
    let target_ids: BTreeSet<_> = scope["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["kind"] == "allocation")
        .map(|l| l["value"]["local"].as_str().unwrap())
        .collect();
    let AllocationAccessPolicy::PobIndependentSavedPathsV1 { pools, .. } = family::policy();
    let ordinary = pools
        .iter()
        .find(|p| p.root == AllocationRootKind::Class)
        .unwrap();
    let ordinary = serde_json::to_value(&ordinary.pool).unwrap();
    let aa = a["draft"]["allocations"]["members"].as_array_mut().unwrap();
    let bb = b["draft"]["allocations"]["members"].as_array_mut().unwrap();
    assert_eq!(aa.len(), bb.len());
    for (x, y) in aa.iter_mut().zip(bb) {
        if target_ids.contains(y["id"]["local"].as_str().unwrap())
            && (!ordinary_must_stay_pending || y["pool"]["value"] == ordinary)
        {
            assert_eq!(y["access"]["kind"], "pending", "{label}");
        }
        // Every original scalar, choice, identity and collection remains exact;
        // only access classification and its issue allocation may differ.
        x.as_object_mut().unwrap().remove("access");
        y.as_object_mut().unwrap().remove("access");
    }
    b["draft"]["allocator"] = a["draft"]["allocator"].clone();
    selected::canonical(&mut a);
    selected::canonical(&mut b);
    correspond(
        &a,
        &mut b,
        &mut BTreeMap::new(),
        "negative probe retains all other canonical facts",
    );
}
fn probes(prior: &Path, package: &Path, out: &Path) -> usize {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let draft: Value = read(out.join("prior-original-05/draft.json"));
    let lineage = serde_json::from_value(draft["draft"]["allocator"]["lineage"].clone()).unwrap();
    let source = ImportedBuildInstance::from_decoded(
        decode_build(original.as_bytes()).unwrap(),
        lineage,
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let tree = evidence
        .rows()
        .iter()
        .find(|r| r.occurrence().name() == "Tree")
        .unwrap();
    let index: usize = attr(tree, "activeSpec").parse().unwrap();
    let spec = evidence
        .rows()
        .iter()
        .filter(|r| r.occurrence().name() == "Spec")
        .nth(index - 1)
        .unwrap();
    let ordinal = spec.occurrence().id().ordinal();
    let start = original.match_indices("<Spec ").nth(index - 1).unwrap().0;
    let end = start + original[start..].find('>').unwrap() + 1;
    let head = &original[start..end];
    let replaced = |new: String| format!("{}{}{}", &original[..start], new, &original[end..]);
    for (label, changed) in [
        (
            "unknown-field",
            head.replacen("<Spec ", "<Spec unreviewedAccess=\"1\" ", 1),
        ),
        (
            "namespace",
            head.replacen(
                "<Spec ",
                "<Spec xmlns:q=\"urn:unreviewed\" q:access=\"1\" ",
                1,
            ),
        ),
        (
            "unknown-class",
            head.replace(
                &format!("classInternalId=\"{}\"", attr(spec, "classInternalId")),
                "classInternalId=\"999\"",
            ),
        ),
        ("unknown-child", format!("{head}<UnreviewedAccess/>")),
    ] {
        assert_ne!(changed, head);
        access_probe(
            prior,
            package,
            out,
            label,
            &replaced(changed),
            ordinal,
            false,
        );
    }
    let c = catalogue();
    let class = c["classes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == attr(spec, "classInternalId"))
        .unwrap();
    let anchor = class["root"].as_str().unwrap();
    let saved = csv(attr(spec, "nodes"));
    let facts: Value = read(root().join("data/owned/poe2/3887ae68/tree/source-facts.json"));
    let admitted = reviewed_tokens(&c, &facts);
    let entrances: BTreeSet<_> = c["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| {
            let l = e["left"].as_str().unwrap();
            let r = e["right"].as_str().unwrap();
            if l == anchor && saved.contains(r) && admitted.contains(r) {
                Some(r)
            } else if r == anchor && saved.contains(l) && admitted.contains(l) {
                Some(l)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        entrances.len(),
        1,
        "independently selected sole ordinary entrance"
    );
    let entrance = *entrances.first().unwrap();
    let nodes = attr(spec, "nodes");
    let removed = nodes
        .split(',')
        .filter(|n| *n != entrance)
        .collect::<Vec<_>>()
        .join(",");
    for (label, value) in [
        ("removed-entrance", removed),
        ("duplicate-entrance", format!("{nodes},{entrance}")),
    ] {
        let changed = head.replace(&format!("nodes=\"{nodes}\""), &format!("nodes=\"{value}\""));
        assert_ne!(changed, head);
        access_probe(
            prior,
            package,
            out,
            label,
            &replaced(changed),
            ordinal,
            true,
        );
    }
    6
}

#[test]
#[ignore = "requires the exact checked support-inventory predecessor"]
fn real_positive_paths_preserve_all_other_inputs_and_coverage() {
    let p = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ALLOCATION_ACCESS_PRIOR").expect("explicit prior"),
    );
    let before = release::inventory(&p);
    let prior = release::load(&p);
    let next = family::stage(&prior);
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_ALLOCATION_ACCESS_OUTPUT")
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
        if !["release.json", "tree-normalization.json"].contains(&name.as_str()) {
            assert_eq!(inv.get(name), Some(hash), "unchanged {name}");
        }
    }
    for field in ["registry", "mapping", "normalization"] {
        let mut bad = serde_json::to_value(next.input()).unwrap();
        bad["tree"][field] = json!("0".repeat(64));
        assert!(
            assemble_owned_release(serde_json::from_value(bad).unwrap(), Default::default())
                .is_err()
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
    let probes = probes(&p, &package, &out);
    assert_eq!(before, release::inventory(&p));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"definitions":next.receipt().definitions,"registry":next.receipt().registry,"prior_provenance":prior.input().provenance.len(),"final_provenance":next.input().provenance.len(),"queries":next.receipt().query_rows,"reviewed_nodes":4316,"originals":reports,"source_probes":probes,"stale_binding_rejections":3,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0,"evaluation":"not_run"}),
    );
}
