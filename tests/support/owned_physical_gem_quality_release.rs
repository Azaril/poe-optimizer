use super::{catalog_join, command, data, key, read, root, success, write};
use poe_optimizer_core::{
    owned_draft::{DraftLimits, DraftQuality, decode_draft},
    owned_schema::*,
};
use poe_optimizer_import::{
    owned_normalize::GemQualityPolicy,
    owned_recipe::OwnedRecipeInput,
    owned_release::{
        OwnedReleaseInput, OwnedReleaseReceipt, StagedOwnedRelease, assemble_owned_release,
    },
    owned_release_revision::{OwnedReleaseRevisionInput, compile_owned_release_revision},
    owned_successor::NamedQuerySet,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
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
    let s = assemble_owned_release(
        OwnedReleaseInput {
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
        },
        Default::default(),
    )
    .unwrap();
    assert_eq!(*s.receipt(), r);
    assert_eq!(s.artifacts().count(), inventory(p).len());
    for (n, b) in s.artifacts() {
        assert_eq!(b, fs::read(p.join(n)).unwrap());
    }
    s
}
fn normalize(package: &Path, xml: &Path, case: usize, out: &Path) -> Value {
    let mut c = command("normalize-owned");
    c.arg(xml);
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
    let r = success(
        c.arg("--queries")
            .arg(package.join(format!("queries-original-{case:02}.json")))
            .arg("--output")
            .arg(out)
            .output()
            .unwrap(),
    );
    assert_eq!(r["normalization_status"], "pending");
    assert_eq!(r["verification"]["calculation"], "not_run");
    r
}
fn preservation(
    before: &StagedOwnedRelease,
    after: &StagedOwnedRelease,
    revision: &OwnedReleaseRevisionInput,
) {
    let b = before.input();
    let mut r = after.input().clone();
    assert_eq!(after.receipt().source, before.receipt().source);
    assert_eq!(r.recipe.registry, b.recipe.registry);
    assert_eq!(r.query_sets, b.query_sets);
    assert_eq!(
        (after.receipt().query_sets, after.receipt().query_rows),
        (5, 110)
    );
    let correction: BTreeSet<_> = revision.definitions.iter().map(|v| v.address()).collect();
    let mut changed = 0;
    for entry in &mut r.recipe.schema.definitions {
        if correction.contains(&entry.address()) {
            let old = b
                .recipe
                .schema
                .definitions
                .iter()
                .find(|v| v.address() == entry.address())
                .unwrap();
            let DefinitionDescriptor::Gem(new) = entry else {
                panic!("Gem only")
            };
            let DefinitionDescriptor::Gem(old_gem) = old else {
                panic!("Gem only")
            };
            let (SchemaState::Known(new), SchemaState::Known(old)) =
                (&mut new.schema, &old_gem.schema)
            else {
                panic!("Known only")
            };
            assert_eq!(new.quality.allowed_kinds.closure, SchemaClosure::Complete);
            assert!(matches!(
                old.quality.allowed_kinds.closure,
                SchemaClosure::Partial { .. }
            ));
            new.quality.allowed_kinds.closure = old.quality.allowed_kinds.closure.clone();
            changed += 1;
        }
    }
    assert_eq!(changed, 603);
    r.recipe.schema.release = b.recipe.schema.release.clone();
    assert_eq!(r.recipe.schema, b.recipe.schema);
    r.recipe.rules.definitions = b.recipe.rules.definitions.clone();
    r.recipe.routing.definitions = b.recipe.routing.definitions.clone();
    r.mapping.definitions = b.mapping.definitions.clone();
    r.mapping.registry = b.mapping.registry;
    r.roles.definitions = b.roles.definitions.clone();
    r.roles.mapping = b.roles.mapping;
    if let (GemQualityPolicy::Attributes(a), GemQualityPolicy::Attributes(b)) = (
        &mut r.normalization.gem_quality,
        &b.normalization.gem_quality,
    ) {
        a.definitions = b.definitions.clone();
    }
    if let (Some(a), Some(b)) = (&mut r.normalization.gem_inputs, &b.normalization.gem_inputs) {
        a.definitions = b.definitions.clone();
    }
    r.rewards.definitions = b.rewards.definitions.clone();
    r.rewards.mapping = b.rewards.mapping;
    r.items.definitions = b.items.definitions.clone();
    r.item_source.item_lines = b.item_source.item_lines;
    assert_eq!(
        r.tree.as_ref().map(|v| &v.content),
        b.tree.as_ref().map(|v| &v.content)
    );
    r.tree = b.tree.clone();
    assert_eq!(r.provenance.len(), b.provenance.len() + 1);
    r.provenance.pop();
    assert_eq!(r, *b, "facet-only revision and exact dependent rebinding");
}
fn probe(xml: &str, quality: Option<&str>, kind: Option<&str>) -> String {
    let offset = xml.find("variantId=\"Twister\"").unwrap();
    let start = xml[..offset].rfind("<Gem ").unwrap();
    let end = offset + xml[offset..].find("/>").unwrap() + 2;
    let gem = &xml[start..end];
    let q = gem.find("quality=\"").unwrap();
    let qe = q + 9 + gem[q + 9..].find('"').unwrap() + 1;
    let attr = quality
        .map(|v| format!("quality=\"{v}\""))
        .unwrap_or_default();
    let mut revised = format!("{}{}{}", &gem[..q], attr, &gem[qe..]);
    if let Some(kind) = kind {
        assert!(!revised.contains("qualityId="));
        revised = revised.replacen("<Gem ", &format!("<Gem qualityId=\"{kind}\" "), 1);
    }
    format!("{}{}{}", &xml[..start], revised, &xml[end..])
}
pub(super) fn check(prior_path: &Path) {
    let prior_hashes = inventory(prior_path);
    let prior = load(prior_path);
    let revision: OwnedReleaseRevisionInput = read(data().join("revision.json"));
    assert_eq!(revision.before, prior.receipt().input);
    let joined = catalog_join(&prior.input().mapping);
    let mut expected = Vec::new();
    let mut counts = [0, 0, 0];
    for row in &prior.input().recipe.schema.definitions {
        let DefinitionDescriptor::Gem(entry) = row else {
            continue;
        };
        assert!(joined.contains_key(&entry.id));
        match &entry.schema {
            SchemaState::Known(gem) => {
                assert_eq!(gem.quality.allowed_kinds.members.len(), 1);
                assert_eq!(
                    gem.quality.allowed_kinds.members[0].key().as_str(),
                    "def.0000000000000006"
                );
                if gem.quality.allowed_kinds.is_complete() {
                    counts[1] += 1;
                    assert_eq!(entry.id.key().as_str(), "def.0000000000000011");
                } else {
                    counts[0] += 1;
                    let mut next = row.clone();
                    let DefinitionDescriptor::Gem(entry) = &mut next else {
                        unreachable!()
                    };
                    let SchemaState::Known(gem) = &mut entry.schema else {
                        unreachable!()
                    };
                    gem.quality.allowed_kinds.closure = SchemaClosure::Complete;
                    expected.push(next);
                }
            }
            SchemaState::Unmapped { .. } => counts[2] += 1,
        }
    }
    assert_eq!(counts, [603, 1, 362]);
    assert_eq!(expected, revision.definitions);
    assert!(revision.slots.is_empty());
    let staged =
        compile_owned_release_revision(&prior, revision.clone(), Default::default()).unwrap();
    preservation(&prior, &staged, &revision);
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_PHYSICAL_GEM_QUALITY_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("quality"));
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    let report = success(
        command("assemble-owned-release")
            .arg(prior_path)
            .arg("--revision")
            .arg(data().join("revision.json"))
            .arg("--output")
            .arg(&package)
            .output()
            .unwrap(),
    );
    assert_eq!(report, serde_json::to_value(staged.receipt()).unwrap());
    assert_eq!(
        success(
            command("assemble-owned-release")
                .arg(&package)
                .arg("--output")
                .arg(&rebuilt)
                .output()
                .unwrap()
        ),
        report
    );
    assert_eq!(inventory(&package), inventory(&rebuilt));
    assert_eq!(
        fs::read(package.join("registry.json")).unwrap(),
        fs::read(prior_path.join("registry.json")).unwrap()
    );
    for case in 1..=5 {
        let name = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(package.join(&name)).unwrap(),
            fs::read(prior_path.join(&name)).unwrap()
        );
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let r = normalize(
            &package,
            &xml,
            case,
            &out.join(format!("original-{case:02}")),
        );
        write(out.join(format!("original-{case:02}-report.json")), &r);
    }
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-02.xml"))
            .unwrap();
    for (label, amount, kind, expected) in [
        ("missing", None, None, None),
        ("nil", Some("nil"), None, None),
        ("malformed", Some("invalid"), None, None),
        ("bogus-kind", Some("20"), Some("bogus"), None),
        ("fraction", Some("0.5"), None, Some(0.5)),
        ("zero", Some("0"), None, Some(0.)),
    ] {
        let text = probe(&original, amount, kind);
        let xml = out.join(format!("probe-{label}.xml"));
        fs::write(&xml, text.as_bytes()).unwrap();
        let destination = out.join(format!("probe-{label}"));
        normalize(&package, &xml, 2, &destination);
        let draft = decode_draft(
            &fs::read(destination.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let gem = draft
            .input()
            .gems
            .members
            .iter()
            .find(|v| {
                v.definition
                    .to_resolved()
                    .is_some_and(|v| v.key().as_str() == "def.000000000000000a")
            })
            .unwrap();
        if let Some(expected) = expected {
            let DraftQuality::Known { value: Some(q) } = &gem.quality else {
                panic!("selected quality {label}");
            };
            assert_eq!(
                q.kind.to_resolved().unwrap().key().as_str(),
                "def.0000000000000006"
            );
            assert_eq!(q.amount.to_resolved().unwrap().value(), expected);
        } else {
            assert!(
                matches!(gem.quality, DraftQuality::Pending(_)),
                "{label}: {:?}",
                gem.quality
            );
        }
    }
    assert_eq!(prior_hashes, inventory(prior_path));
    write(
        out.join("validation.json"),
        &serde_json::json!({"before":prior.receipt().input,"after":staged.receipt().input,"definitions":staged.receipt().definitions,"registry":staged.receipt().registry,"corrected_gems":603,"already_complete_unchanged":1,"unmapped_unchanged":362,"query_sets":5,"query_rows":110,"originals_pending":5,"quality_probes":6,"prior_provenance_entries":prior.input().provenance.len(),"final_provenance_entries":staged.input().provenance.len(),"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
