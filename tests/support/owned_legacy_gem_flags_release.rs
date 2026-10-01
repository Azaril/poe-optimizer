use super::{Authoring, command, data, read, root, success, write};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_draft::{DraftLimits, DraftListCompletion, decode_draft},
    owned_schema::*,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{GemQualityPolicy, NormalizationPolicy},
    owned_recipe::OwnedRecipeInput,
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
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
fn check_values(
    xml: &[u8],
    out: &Path,
    expected_first: Option<(&str, Option<bool>, Option<f64>)>,
) -> [usize; 2] {
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
        let (index, flag, delta) = match def.key().as_str() {
            "def.000000000000000a" => (0, "def.00000000000030b0", "def.00000000000030a9"),
            "def.0000000000000011" => (1, "def.00000000000030b1", "def.00000000000030aa"),
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
        let is_first =
            expected_first.is_some_and(|(k, _, _)| k == def.key().as_str()) && !first_seen;
        let (expected_flag, expected_delta) = if is_first {
            first_seen = true;
            let (_, b, q) = expected_first.unwrap();
            (b, q)
        } else {
            let token = row.attribute("corrupted").unwrap().decoded().unwrap();
            assert!(matches!(token, "false" | "nil"));
            assert_eq!(
                row.attribute("corruptLevel").unwrap().decoded().unwrap(),
                "0"
            );
            (Some(false), Some(0.))
        };
        for (slot, expected) in [
            (flag, expected_flag.map(ParameterValue::Boolean)),
            (
                delta,
                expected_delta.map(|q| {
                    ParameterValue::Quantity(
                        poe_optimizer_core::owned_definitions::FiniteQuantity::new(
                            q,
                            poe_optimizer_core::owned_definitions::UnitDefId::new(
                                def.namespace().clone(),
                                key("def.000000000000295a"),
                            ),
                        )
                        .unwrap(),
                    )
                }),
            ),
        ] {
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
                assert_eq!(values.len(), 1, "selected {slot}");
                assert_eq!(values[0].value.to_resolved(), Some(expected), "{slot}");
            } else {
                assert!(values.is_empty(), "pending {slot}");
            }
        }
    }
    if expected_first.is_some() {
        assert!(first_seen)
    }
    counts
}
fn replace_attribute(gem: &str, name: &str, value: Option<&str>) -> String {
    let start = gem.find(&format!("{name}=\"")).unwrap();
    let end = start + name.len() + 2 + gem[start + name.len() + 2..].find('"').unwrap() + 1;
    let replacement = value.map(|v| format!("{name}=\"{v}\"")).unwrap_or_default();
    format!("{}{}{}", &gem[..start], replacement, &gem[end..])
}
fn probe(xml: &str, variant: &str, delta: Option<&str>, corrupted: Option<&str>) -> String {
    let offset = xml.find(&format!("variantId=\"{variant}\"")).unwrap();
    let start = xml[..offset].rfind("<Gem ").unwrap();
    let end = offset + xml[offset..].find("/>").unwrap() + 2;
    let revised = replace_attribute(
        &replace_attribute(&xml[start..end], "corruptLevel", delta),
        "corrupted",
        corrupted,
    );
    format!("{}{}{}", &xml[..start], revised, &xml[end..])
}
fn preservation(prior: &StagedOwnedRelease, after: &StagedOwnedRelease, a: &Authoring) {
    let b = prior.input();
    let mut r = after.input().clone();
    assert_eq!(after.receipt().source, prior.receipt().source);
    assert_eq!(
        (after.receipt().query_sets, after.receipt().query_rows),
        (5, 110)
    );
    assert_eq!(
        r.recipe.registry.entries.len(),
        b.recipe.registry.entries.len() + 2
    );
    assert_eq!(
        &r.recipe.registry.entries[..b.recipe.registry.entries.len()],
        b.recipe.registry.entries.as_slice()
    );
    assert_eq!(
        r.recipe.registry.last_issued.get(),
        b.recipe.registry.last_issued.get() + 2
    );
    r.recipe.registry = b.recipe.registry.clone();
    for row in &a.owners {
        let entry = r
            .recipe
            .schema
            .definitions
            .iter_mut()
            .find(|v| v.address() == row.prior.address())
            .unwrap();
        let DefinitionDescriptor::Gem(g) = entry else {
            panic!("Gem")
        };
        let SchemaState::Known(g) = &mut g.schema else {
            panic!("Known")
        };
        assert!(g.quality.allowed_kinds.is_complete());
        assert!(matches!(
            g.declarations.parameters.closure,
            SchemaClosure::Partial { .. }
        ));
        assert_eq!(
            g.declarations
                .parameters
                .members
                .iter()
                .filter(|v| **v == row.flag)
                .count(),
            1
        );
        g.declarations.parameters.members.retain(|v| *v != row.flag);
        assert_eq!(*entry, row.prior);
        let slot = r
            .recipe
            .schema
            .slots
            .iter()
            .position(|v| v.address() == SlotAddress::Parameter(row.flag.clone()))
            .unwrap();
        r.recipe.schema.slots.remove(slot);
        let policy = r
            .normalization
            .gem_inputs
            .as_mut()
            .unwrap()
            .gems
            .iter_mut()
            .find(|v| v.gem == row.gem)
            .unwrap();
        assert_eq!(*policy, row.next_rule);
        *policy = row.prior_rule.clone();
    }
    assert_eq!(r.recipe.schema, b.recipe.schema);
    r.recipe.rules.definitions = b.recipe.rules.definitions.clone();
    r.recipe.routing.definitions = b.recipe.routing.definitions.clone();
    r.mapping.definitions = b.mapping.definitions.clone();
    r.mapping.registry = b.mapping.registry;
    r.roles.definitions = b.roles.definitions.clone();
    r.roles.mapping = b.roles.mapping;
    if let (GemQualityPolicy::Attributes(x), GemQualityPolicy::Attributes(y)) = (
        &mut r.normalization.gem_quality,
        &b.normalization.gem_quality,
    ) {
        x.definitions = y.definitions.clone();
    }
    r.normalization.gem_inputs.as_mut().unwrap().definitions = b
        .normalization
        .gem_inputs
        .as_ref()
        .unwrap()
        .definitions
        .clone();
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
    assert_eq!(&r.provenance[..b.provenance.len()], b.provenance.as_slice());
    r.provenance.pop();
    assert_eq!(
        r, *b,
        "only two raw Boolean declarations/recipes and exact dependency rebinding"
    );
}
pub(super) fn check(prior_path: &Path) {
    let a: Authoring = read(data().join("authoring.json"));
    let extension: OwnedRecipeExtension = read(data().join("extension.json"));
    let prior_hashes = inventory(prior_path);
    let prior = load(prior_path);
    assert_eq!(prior.receipt().input, a.before);
    assert_eq!(prior.assembled().schema().identity(), &a.definitions);
    assert_eq!(prior.input().recipe.schema.schema_version, 4);
    assert_eq!(
        prior.input().recipe.rules.operations_version.as_str(),
        "owned-domain-operations-v13"
    );
    for row in &a.owners {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .find(|v| v.address() == row.prior.address()),
            Some(&row.prior)
        );
        assert_eq!(
            prior
                .normalization()
                .gem_inputs
                .as_ref()
                .unwrap()
                .gems
                .iter()
                .find(|v| v.gem == row.gem),
            Some(&row.prior_rule)
        );
    }
    let known_quality=prior.input().recipe.schema.definitions.iter().filter(|v|matches!(v,DefinitionDescriptor::Gem(g) if matches!(&g.schema,SchemaState::Known(s) if s.quality.allowed_kinds.is_complete()))).count();
    assert_eq!(known_quality, 604);
    let extended = extend_owned_recipe(prior.assembled(), &extension, Default::default()).unwrap();
    assert_eq!(
        (
            extended.receipt.allocated_entries,
            extended.receipt.refined_subjects,
            extended.receipt.appended_programs,
            extended.receipt.appended_tables,
            extended.receipt.appended_receivers
        ),
        (2, 2, 0, 0, 0)
    );
    let mut unordered = extension.clone();
    unordered.schema.swap(0, 1);
    assert!(extend_owned_recipe(prior.assembled(), &unordered, Default::default()).is_err());
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_LEGACY_GEM_FLAGS_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("publication"));
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    let schema = out.join("schema");
    let normalized = out.join("normalized");
    let report = success(
        command("extend-owned-recipe")
            .arg(prior_path)
            .arg("--extension")
            .arg(data().join("extension.json"))
            .arg("--output")
            .arg(&schema)
            .output()
            .unwrap(),
    );
    write(out.join("extension-receipt.json"), &report);
    let mut policy: NormalizationPolicy = read(schema.join("normalization.json"));
    for row in &a.owners {
        let target = policy
            .gem_inputs
            .as_mut()
            .unwrap()
            .gems
            .iter_mut()
            .find(|v| v.gem == row.gem)
            .unwrap();
        assert_eq!(*target, row.prior_rule);
        *target = row.next_rule.clone();
    }
    write(out.join("normalization.json"), &policy);
    let stale = command("publish-owned-normalization")
        .arg(&schema)
        .arg("--normalization")
        .arg(prior_path.join("normalization.json"))
        .arg("--output")
        .arg(out.join("stale-rejected"))
        .output()
        .unwrap();
    assert!(!stale.status.success());
    assert!(!out.join("stale-rejected").exists());
    let report = success(
        command("publish-owned-normalization")
            .arg(&schema)
            .arg("--normalization")
            .arg(out.join("normalization.json"))
            .arg("--output")
            .arg(&normalized)
            .output()
            .unwrap(),
    );
    write(out.join("normalization-receipt.json"), &report);
    let mut input = prior.input().clone();
    input.recipe = OwnedRecipeInput {
        schema_version: 1,
        registry: read(normalized.join("registry.json")),
        schema: read(normalized.join("schema.json")),
        rules: read(normalized.join("rules.json")),
        routing: read(normalized.join("routing.json")),
    };
    assert_eq!(input.recipe, extended.successor);
    input.mapping = read(normalized.join("mapping.json"));
    input.roles = read(normalized.join("roles.json"));
    input.normalization = read(normalized.join("normalization.json"));
    input.rewards = read(normalized.join("rewards.json"));
    input.items = read(normalized.join("items.json"));
    input.item_source = read(normalized.join("item-source.json"));
    input.tree = Some(read(normalized.join("tree-normalization.json")));
    assert_eq!(input.normalization, policy);
    input.provenance.push(OwnedReleaseProvenance {
        kind: key("explicit-legacy-gem-corruption-flags"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-legacy-gem-corruption-flags-v1",
            &(&a, &extension),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let staged = assemble_owned_release(input.clone(), Default::default()).unwrap();
    preservation(&prior, &staged, &a);
    let replay = extend_owned_recipe(staged.assembled(), &extension, Default::default()).unwrap();
    assert_eq!(replay.receipt.allocated_entries, 0);
    assert_eq!(replay.receipt.refined_subjects, 0);
    assert_eq!(replay.successor, staged.input().recipe);
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
        assert_eq!(b, fs::read(package.join(n)).unwrap());
    }
    for query in &prior.input().query_sets {
        let name = format!("queries-{}.json", query.name.as_str());
        assert_eq!(
            fs::read(prior_path.join(&name)).unwrap(),
            fs::read(package.join(&name)).unwrap()
        );
    }
    let mut counts = Vec::new();
    for case in 1..=5 {
        let source = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let destination = out.join(format!("original-{case:02}"));
        let mut c = command("normalize-owned");
        c.arg(&source);
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
        assert_eq!(report["verification"]["calculation"], "not_run");
        counts.push(check_values(&fs::read(source).unwrap(), &destination, None));
        write(out.join(format!("original-{case:02}-report.json")), &report);
    }
    assert_eq!(counts, vec![[0, 1], [6, 0], [0, 0], [0, 0], [0, 5]]);
    let vectors = [
        ("true-zero", Some("0"), Some("true"), Some(true), Some(0.)),
        ("true-delta", Some("2"), Some("true"), Some(true), Some(2.)),
        (
            "false-fraction",
            Some("0.25"),
            Some("false"),
            Some(false),
            Some(0.25),
        ),
        ("nil", Some("nil"), Some("nil"), Some(false), Some(0.)),
        ("missing-flag", Some("2"), None, None, Some(2.)),
        ("bad-flag", Some("2"), Some("TRUE"), None, Some(2.)),
        ("whitespace-flag", Some("2"), Some(" true"), None, Some(2.)),
        ("missing-delta", None, Some("true"), Some(true), None),
        ("bad-delta", Some("bogus"), Some("true"), Some(true), None),
        ("negative", Some("-3"), Some("true"), Some(true), Some(-3.)),
    ];
    for (label, case, variant, gem) in [
        ("twister", 2, "Twister", "def.000000000000000a"),
        ("sniper", 1, "SkeletalSniper", "def.0000000000000011"),
    ] {
        let xml = fs::read_to_string(root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        )))
        .unwrap();
        for (name, delta, flag, b, q) in vectors {
            let mutated = probe(&xml, variant, delta, flag);
            let source = out.join(format!("{label}-{name}.xml"));
            fs::write(&source, mutated.as_bytes()).unwrap();
            let destination = out.join(format!("{label}-{name}"));
            normalize(&package, &source, &destination);
            check_values(mutated.as_bytes(), &destination, Some((gem, b, q)));
        }
    }
    assert_eq!(prior_hashes, inventory(prior_path));
    write(
        out.join("validation.json"),
        &serde_json::json!({"before":prior.receipt().input,"after":staged.receipt().input,"definitions":staged.assembled().schema().identity(),"registry":staged.assembled().registry().identity().unwrap(),"allocated_entries":2,"quality_kind_complete_owners":known_quality,"query_sets":5,"query_rows":110,"original_physical_counts":counts,"originals_pending":5,"probe_cases":20,"prior_unchanged":true,"rebuild_byte_identical":true,"prior_provenance_entries":prior.input().provenance.len(),"final_provenance_entries":staged.input().provenance.len(),"complete_original_builds":0}),
    );
}
