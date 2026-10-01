use super::{command, data, read, root, rule, success, write};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_draft::{DraftLimits, DraftListCompletion, decode_draft},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::NormalizationPolicy,
    owned_recipe::OwnedRecipeInput,
    owned_release::{
        OwnedReleaseInput, OwnedReleaseProvenance, OwnedReleaseReceipt, StagedOwnedRelease,
        assemble_owned_release,
    },
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_successor::NamedQuerySet,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}
fn inventory(p: &Path) -> BTreeMap<String, String> {
    fs::read_dir(p)
        .unwrap()
        .map(|v| {
            let v = v.unwrap();
            assert!(v.file_type().unwrap().is_file());
            (
                v.file_name().into_string().unwrap(),
                format!("{:x}", Sha256::digest(fs::read(v.path()).unwrap())),
            )
        })
        .collect()
}
fn load(p: &Path) -> StagedOwnedRelease {
    let r: OwnedReleaseReceipt = read(p.join("release.json"));
    assert_eq!(r.schema_version, 1);
    assert!(r.evaluation.is_none());
    let input = OwnedReleaseInput {
        schema_version: 1,
        recipe: OwnedRecipeInput {
            schema_version: 1,
            registry: read(p.join("registry.json")),
            schema: read(p.join("schema.json")),
            rules: read(p.join("rules.json")),
            routing: read(p.join("routing.json")),
        },
        mapping: read(p.join("mapping.json")),
        roles: read(p.join("roles.json")),
        normalization: read(p.join("normalization.json")),
        rewards: read(p.join("rewards.json")),
        items: read(p.join("items.json")),
        item_source: read(p.join("item-source.json")),
        tree: r.tree.map(|_| read(p.join("tree-normalization.json"))),
        evaluation: None,
        query_sets: r
            .artifacts
            .iter()
            .filter_map(|v| {
                v.file
                    .strip_prefix("queries-")
                    .and_then(|v| v.strip_suffix(".json"))
                    .map(|n| NamedQuerySet {
                        name: key(n),
                        queries: read(p.join(&v.file)),
                    })
            })
            .collect(),
        provenance: r.provenance.clone(),
    };
    let s = assemble_owned_release(input, Default::default()).unwrap();
    assert_eq!(*s.receipt(), r);
    assert_eq!(s.artifacts().count(), inventory(p).len());
    for (n, b) in s.artifacts() {
        assert_eq!(b, fs::read(p.join(n)).unwrap());
    }
    s
}
fn normalize(package: &Path, input: &Path, output: &Path) -> Value {
    let mut c = command("normalize-owned");
    c.arg(input);
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
    // Probe callers pass the original query path by the input's parent sidecar.
    let case = if input
        .file_name()
        .unwrap()
        .to_string_lossy()
        .contains("twister")
    {
        2
    } else {
        1
    };
    let report = success(
        c.arg("--queries")
            .arg(package.join(format!("queries-original-{case:02}.json")))
            .arg("--output")
            .arg(output)
            .output()
            .unwrap(),
    );
    assert_eq!(report["normalization_status"], "pending");
    assert_eq!(report["verification"]["calculation"], "not_run");
    report
}
fn check_values(xml: &[u8], out: &Path, expected_first: Option<(&str, Option<f64>)>) -> [usize; 2] {
    let d = decode_draft(
        &fs::read(out.join("draft.json")).unwrap(),
        DraftLimits::default(),
    )
    .unwrap();
    let s: Value = read(out.join("sidecar.json"));
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        d.input().allocator.lineage(),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let mut counts = [0, 0];
    let mut first_seen = false;
    assert_eq!(s["source_sha256"], format!("{:x}", Sha256::digest(xml)));
    for g in &d.input().gems.members {
        let Some(def) = g.definition.to_resolved() else {
            continue;
        };
        let (index, slot) = match def.key().as_str() {
            "def.000000000000000a" => (0, "def.00000000000030a9"),
            "def.0000000000000011" => (1, "def.00000000000030aa"),
            _ => continue,
        };
        counts[index] += 1;
        assert!(matches!(
            g.parameters.completion,
            DraftListCompletion::Pending { .. }
        ));
        let origins: Vec<_> = s["origins"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| {
                v["links"].as_array().unwrap().iter().any(|v| {
                    v["kind"] == "gem" && v["value"] == serde_json::to_value(g.id).unwrap()
                })
            })
            .collect();
        assert_eq!(origins.len(), 1);
        let row = evidence
            .rows()
            .iter()
            .find(|v| serde_json::to_value(v.occurrence().id()).unwrap() == origins[0]["source"])
            .unwrap();
        let is_first = expected_first.is_some_and(|(k, _)| k == def.key().as_str()) && !first_seen;
        let expected = if is_first {
            first_seen = true;
            expected_first.unwrap().1
        } else {
            Some(0.)
        };
        let values: Vec<_> = g
            .parameters
            .members
            .iter()
            .filter(|v| {
                v.slot
                    .to_resolved()
                    .is_some_and(|v| v.slot.key().as_str() == slot)
            })
            .collect();
        if let Some(expected) = expected {
            assert_eq!(values.len(), 1);
            let Some(ParameterValue::Quantity(q)) = values[0].value.to_resolved() else {
                panic!("known raw delta");
            };
            assert_eq!(q.value(), expected);
            assert_eq!(q.unit().key().as_str(), "def.000000000000295a");
            assert!(row.attribute("corruptLevel").is_some());
        } else {
            assert!(values.is_empty());
        }
    }
    if expected_first.is_some() {
        assert!(first_seen);
    }
    counts
}
fn replace_attribute(gem: &str, name: &str, value: Option<&str>) -> String {
    let start = gem.find(&format!("{name}=\"")).unwrap();
    let end = start + name.len() + 2 + gem[start + name.len() + 2..].find('"').unwrap() + 1;
    let replacement = value.map(|v| format!("{name}=\"{v}\"")).unwrap_or_default();
    format!("{}{}{}", &gem[..start], replacement, &gem[end..])
}
fn probe(xml: &str, variant: &str, delta: Option<&str>, corrupted: &str) -> String {
    let offset = xml.find(&format!("variantId=\"{variant}\"")).unwrap();
    let start = xml[..offset].rfind("<Gem ").unwrap();
    let end = offset + xml[offset..].find("/>").unwrap() + 2;
    let gem = &xml[start..end];
    let revised = replace_attribute(
        &replace_attribute(gem, "corruptLevel", delta),
        "corrupted",
        Some(corrupted),
    );
    format!("{}{}{}", &xml[..start], revised, &xml[end..])
}
pub(super) fn check(prior_path: &Path) {
    let manifest: Value = read(data().join("authoring.json"));
    let p: NormalizationPolicy = read(data().join("normalization.json"));
    let prior_hashes = inventory(prior_path);
    let prior = load(prior_path);
    assert_eq!(
        serde_json::to_value(prior.receipt().input).unwrap(),
        manifest["before"]
    );
    assert_eq!(prior.input().recipe.schema.schema_version, 4);
    assert_eq!(
        prior.input().recipe.rules.operations_version.as_str(),
        "owned-domain-operations-v13"
    );
    let mut restored = p.clone();
    restored.version = prior.normalization().version.clone();
    for k in ["def.000000000000000a", "def.0000000000000011"] {
        let old = rule(prior.normalization(), k);
        let new = rule(&p, k);
        assert_eq!(old.guards.len(), 2);
        assert!(old.parameters.is_empty());
        assert_eq!(new.guards.as_slice(), &old.guards[..1]);
        let row = restored
            .gem_inputs
            .as_mut()
            .unwrap()
            .gems
            .iter_mut()
            .find(|v| v.gem.key().as_str() == k)
            .unwrap();
        *row = old.clone();
    }
    assert_eq!(restored, *prior.normalization());
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_EFFECTIVE_GEM_RAW_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("publication"));
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    let compact = out.join("compact");
    let transition = success(
        command("publish-owned-normalization")
            .arg(prior_path)
            .arg("--normalization")
            .arg(data().join("normalization.json"))
            .arg("--output")
            .arg(&compact)
            .output()
            .unwrap(),
    );
    write(out.join("compact-publication.json"), &transition);
    let mut input = prior.input().clone();
    input.normalization = read(compact.join("normalization.json"));
    input.tree = Some(read(compact.join("tree-normalization.json")));
    assert_eq!(input.normalization, p);
    assert_eq!(
        input.tree.as_ref().unwrap().content,
        prior.input().tree.as_ref().unwrap().content
    );
    input.provenance.push(OwnedReleaseProvenance {
        kind: key("explicit-effective-gem-raw-normalization"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-effective-gem-raw-normalization-v1",
            &(&manifest, &p),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let staged = assemble_owned_release(input.clone(), Default::default()).unwrap();
    let mut restored = staged.input().clone();
    restored.normalization = prior.input().normalization.clone();
    restored.tree = prior.input().tree.clone();
    restored.provenance.pop();
    assert_eq!(restored, *prior.input());
    assert_eq!(staged.receipt().source, prior.receipt().source);
    let endpoint = out.join("endpoint.json");
    write(&endpoint, &input);
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    for (src, dst) in [(&endpoint, &package), (&package, &rebuilt)] {
        assert_eq!(
            success(
                command("assemble-owned-release")
                    .arg(src)
                    .arg("--output")
                    .arg(dst)
                    .output()
                    .unwrap()
            ),
            serde_json::to_value(staged.receipt()).unwrap()
        );
    }
    assert_eq!(inventory(&package), inventory(&rebuilt));
    for (n, b) in staged.artifacts() {
        if ![
            "normalization.json",
            "tree-normalization.json",
            "release.json",
        ]
        .contains(&n)
        {
            assert_eq!(b, fs::read(prior_path.join(n)).unwrap(), "unchanged {n}");
        }
    }
    let mut counts = Vec::new();
    for case in 1..=5 {
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let destination = out.join(format!("original-{case:02}"));
        let mut c = command("normalize-owned");
        c.arg(&xml);
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
        let report = success(
            c.arg("--queries")
                .arg(package.join(format!("queries-original-{case:02}.json")))
                .arg("--output")
                .arg(&destination)
                .output()
                .unwrap(),
        );
        assert_eq!(report["normalization_status"], "pending");
        counts.push(check_values(&fs::read(xml).unwrap(), &destination, None));
        write(out.join(format!("original-{case:02}-report.json")), &report);
    }
    assert_eq!(counts, vec![[0, 1], [6, 0], [0, 0], [0, 0], [0, 5]]);
    let vectors = [
        ("positive", Some("2"), "false", Some(2.)),
        ("negative", Some("-3"), "false", Some(-3.)),
        ("fractional", Some("0.25"), "false", Some(0.25)),
        ("nil", Some("nil"), "nil", Some(0.)),
        ("missing", None, "false", None),
        ("invalid", Some("1x"), "false", None),
        ("overflow", Some("1e309"), "false", None),
        ("corrupted", Some("2"), "true", None),
    ];
    for (label, case, variant, gem) in [
        ("twister", 2, "Twister", "def.000000000000000a"),
        ("sniper", 1, "SkeletalSniper", "def.0000000000000011"),
    ] {
        let xml = fs::read_to_string(root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        )))
        .unwrap();
        for (name, delta, flag, expected) in vectors {
            let mutated = probe(&xml, variant, delta, flag);
            let source = out.join(format!("{label}-{name}.xml"));
            fs::write(&source, mutated.as_bytes()).unwrap();
            let destination = out.join(format!("{label}-{name}"));
            normalize(&package, &source, &destination);
            check_values(mutated.as_bytes(), &destination, Some((gem, expected)));
        }
    }
    assert_eq!(prior_hashes, inventory(prior_path));
    write(
        out.join("validation.json"),
        &serde_json::json!({"before":prior.receipt().input,"after":staged.receipt().input,"query_sets":staged.receipt().query_sets,"query_rows":staged.receipt().query_rows,"original_raw_deltas":counts,"originals_pending":5,"probe_cases":16,"prior_unchanged":true,"rebuild_byte_identical":true,"prior_provenance_entries":prior.input().provenance.len(),"final_provenance_entries":staged.input().provenance.len(),"complete_original_builds":0}),
    );
}
