//! Final constituent composition uses the existing release and Engine checks.
#[allow(dead_code, unused_imports)]
#[path = "../../poe-optimizer-engine/tests/support/owned_fixtures.rs"]
mod engine_fixtures;
#[path = "../../../tests/support/owned_evaluation_release_fixture.rs"]
mod fixture;

use flate2::{Compression, GzBuilder, read::GzDecoder};
use poe_optimizer_authoring::{
    BuiltRelease, ReleaseBuildError, ReleaseBuildLimits, ReleaseSource, SourceEncoding,
    build_release, parse_release_manifest,
};
use poe_optimizer_core::owned_definitions::{DefId, OwnedDefinitionKey};
use poe_optimizer_engine::{
    owned_plan::{OwnedSupportEffectPlan, OwnedSupportMetricPlan, SupportEffectPlanInputs},
    owned_rules::CompiledRulePackage,
};
use poe_optimizer_import::owned_release::{
    OWNED_RELEASE_VERSION, OwnedReleaseError, OwnedReleaseInput, OwnedReleaseReceipt,
    StagedOwnedRelease, assemble_owned_release,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::Path,
    process::{Command, Output},
    sync::{Arc, OnceLock},
};

fn fixture() -> &'static fixture::Fixture {
    static FIXTURE: OnceLock<fixture::Fixture> = OnceLock::new();
    FIXTURE.get_or_init(fixture::fixture)
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::fast());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

struct Sources {
    manifest: String,
    files: Vec<(String, Vec<u8>)>,
    decoded_bytes: usize,
}
impl Sources {
    fn new(input: &OwnedReleaseInput, compressed: bool) -> Self {
        let mut components: Vec<(&str, Vec<u8>)> = Vec::new();
        macro_rules! component {
            ($name:literal, $value:expr) => {
                components.push(($name, serde_json::to_vec($value).unwrap()));
            };
        }
        component!("registry", &input.recipe.registry);
        component!("schema", &input.recipe.schema);
        component!("rules", &input.recipe.rules);
        component!("routing", &input.recipe.routing);
        component!("mapping", &input.mapping);
        component!("roles", &input.roles);
        component!("normalization", &input.normalization);
        component!("rewards", &input.rewards);
        component!("items", &input.items);
        component!("item-source", &input.item_source);
        if let Some(tree) = &input.tree {
            component!("tree-normalization", tree);
        }
        if let Some(evaluation) = &input.evaluation {
            component!("metrics", &evaluation.metrics);
            if let Some(support) = &evaluation.support {
                component!("support-stages", &support.stages);
                component!("support-preparation", &support.preparation);
                component!("support-inputs", &support.inputs);
                component!("support-receiving", &support.receiving);
                if let Some(outputs) = &support.outputs {
                    component!("support-outputs", outputs);
                }
            }
        }
        component!("provenance", &input.provenance);
        let mut output = Self {
            manifest: "release {\n".into(),
            files: Vec::new(),
            decoded_bytes: 0,
        };
        for (component, bytes) in components {
            output.push(
                component,
                &format!("{component}.json"),
                bytes,
                compressed,
                None,
            );
        }
        for query in &input.query_sets {
            output.push(
                "queries",
                &format!("queries-{}.json", query.name),
                serde_json::to_vec(&query.queries).unwrap(),
                compressed,
                Some(query.name.as_str()),
            );
        }
        output.manifest.push_str("}\n");
        output
    }

    fn push(
        &mut self,
        component: &str,
        path: &str,
        bytes: Vec<u8>,
        compressed: bool,
        query: Option<&str>,
    ) {
        let path = if compressed {
            format!("{path}.gz")
        } else {
            path.into()
        };
        let encoding = if compressed { "gzip" } else { "json" };
        let declaration = match query {
            Some(name) => {
                format!("    queries \"{name}\" source=\"{path}\" encoding=\"{encoding}\"\n")
            }
            None => format!("    {component} \"{path}\" encoding=\"{encoding}\"\n"),
        };
        self.manifest.push_str(&declaration);
        self.decoded_bytes += bytes.len();
        self.files
            .push((path, if compressed { gzip(&bytes) } else { bytes }));
    }

    fn sources(&self) -> Vec<ReleaseSource<'_>> {
        self.files
            .iter()
            .map(|(path, bytes)| ReleaseSource { path, bytes })
            .collect()
    }

    fn build(&self, limits: ReleaseBuildLimits) -> Result<BuiltRelease, ReleaseBuildError> {
        let manifest = parse_release_manifest(&self.manifest, limits)?;
        build_release(&manifest, &self.sources(), limits)
    }

    fn encoded_bytes(&self) -> usize {
        self.files.iter().map(|(_, bytes)| bytes.len()).sum()
    }
}

fn files(release: &StagedOwnedRelease) -> BTreeMap<String, Vec<u8>> {
    release
        .artifacts()
        .map(|(name, bytes)| (name.into(), bytes.to_vec()))
        .collect()
}

fn invalid<T>(result: Result<T, ReleaseBuildError>, message: &str) {
    match result {
        Err(ReleaseBuildError::Invalid(actual)) => assert_eq!(actual, message),
        Err(other) => panic!("expected invalid {message:?}, got {other}"),
        Ok(_) => panic!("unexpected success: {message}"),
    }
}

fn limited<T>(result: Result<T, ReleaseBuildError>, message: &str) {
    match result {
        Err(ReleaseBuildError::Limit(actual)) => assert_eq!(actual, message),
        Err(other) => panic!("expected limit {message:?}, got {other}"),
        Ok(_) => panic!("unexpected success: {message}"),
    }
}

fn assert_metric(release: &StagedOwnedRelease) {
    let recipe = release.assembled();
    let definitions = Arc::new(recipe.schema().clone());
    let rules = Arc::new(
        CompiledRulePackage::compile_stored(
            recipe.rules(),
            definitions.as_ref(),
            Default::default(),
        )
        .unwrap(),
    );
    let evaluation = release.evaluation().unwrap();
    let support = evaluation.support().unwrap();
    let effects = OwnedSupportEffectPlan::compile_with_outputs(
        SupportEffectPlanInputs {
            request: Arc::new(fixture().request.clone()),
            definitions,
            rules,
            routing: Arc::new(recipe.routing().clone()),
            stages: Arc::new(support.stages().clone()),
            preparation: Arc::new(support.preparation().clone()),
            inputs: Arc::new(support.inputs().clone()),
            receiving: Arc::new(support.receiving().clone()),
        },
        Arc::new(support.outputs().unwrap().clone()),
        Default::default(),
        Default::default(),
    )
    .unwrap();
    let plan =
        OwnedSupportMetricPlan::compile(Arc::new(effects), Arc::new(evaluation.metrics().clone()))
            .unwrap();
    assert_eq!(
        plan.evaluate(&mut plan.new_scratch()).unwrap(),
        fixture().expected
    );
}

#[test]
fn json_and_gzip_constituents_reproduce_the_existing_release_and_metric() {
    let expected = assemble_owned_release(fixture().input.clone(), Default::default()).unwrap();
    for compressed in [false, true] {
        let source = Sources::new(&fixture().input, compressed);
        let manifest = parse_release_manifest(&source.manifest, Default::default()).unwrap();
        assert!(manifest.sources().iter().all(|spec| spec.encoding()
            == if compressed {
                SourceEncoding::Gzip
            } else {
                SourceEncoding::Json
            }));
        let mut supplied = source.sources();
        supplied.reverse();
        let actual = build_release(&manifest, &supplied, Default::default()).unwrap();
        assert_eq!(actual.encoded_bytes(), source.encoded_bytes());
        assert_eq!(actual.decoded_bytes(), source.decoded_bytes);
        assert_eq!(actual.release().input(), expected.input());
        assert_eq!(actual.release().receipt(), expected.receipt());
        assert_eq!(files(actual.release()), files(&expected));
        assert_metric(actual.release());
    }
}

#[test]
fn optional_evaluation_groups_are_explicit_and_complete() {
    for mode in 0..3 {
        let mut input = fixture().input.clone();
        match mode {
            0 => {
                input.evaluation = None;
                input.schema_version = OWNED_RELEASE_VERSION;
            }
            1 => input.evaluation.as_mut().unwrap().support = None,
            _ => {
                input
                    .evaluation
                    .as_mut()
                    .unwrap()
                    .support
                    .as_mut()
                    .unwrap()
                    .outputs = None
            }
        }
        let expected = assemble_owned_release(input.clone(), Default::default()).unwrap();
        assert_eq!(
            files(
                Sources::new(&input, false)
                    .build(Default::default())
                    .unwrap()
                    .release()
            ),
            files(&expected)
        );
    }
    let source = Sources::new(&fixture().input, false);
    for component in [
        "metrics",
        "support-stages",
        "support-preparation",
        "support-inputs",
        "support-receiving",
    ] {
        let text = source
            .manifest
            .lines()
            .filter(|line| !line.trim_start().starts_with(&format!("{component} ")))
            .collect::<Vec<_>>()
            .join("\n");
        invalid(
            parse_release_manifest(&text, Default::default()),
            "incomplete support/evaluation component group",
        );
    }
}

#[test]
fn supplied_source_inventory_is_exact_without_fallbacks() {
    let source = Sources::new(&fixture().input, false);
    let manifest = parse_release_manifest(&source.manifest, Default::default()).unwrap();
    let supplied = source.sources();
    invalid(
        build_release(&manifest, &supplied[1..], Default::default()),
        "source inventory differs",
    );
    let mut changed = supplied.clone();
    changed.push(ReleaseSource {
        path: "extra.json",
        bytes: b"{}",
    });
    invalid(
        build_release(&manifest, &changed, Default::default()),
        "source inventory differs",
    );
    let mut changed = supplied.clone();
    changed[0] = changed[1];
    invalid(
        build_release(&manifest, &changed, Default::default()),
        "duplicate supplied source",
    );
    let mut changed = supplied.clone();
    changed[0].path = "unlisted.json";
    invalid(
        build_release(&manifest, &changed, Default::default()),
        "missing declared source",
    );
    let mut changed = supplied;
    changed[0].path = "../outside.json";
    invalid(
        build_release(&manifest, &changed, Default::default()),
        "invalid supplied source path",
    );
}

#[test]
fn encoded_decoded_and_source_count_budgets_have_exact_boundaries() {
    for compressed in [false, true] {
        let source = Sources::new(&fixture().input, compressed);
        let limits = ReleaseBuildLimits {
            max_encoded_bytes: source.encoded_bytes(),
            max_decoded_bytes: source.decoded_bytes,
            max_sources: source.files.len(),
            ..Default::default()
        };
        let exact = source.build(limits).unwrap();
        assert_eq!(exact.encoded_bytes(), limits.max_encoded_bytes);
        assert_eq!(exact.decoded_bytes(), limits.max_decoded_bytes);
        limited(
            source.build(ReleaseBuildLimits {
                max_encoded_bytes: limits.max_encoded_bytes - 1,
                ..limits
            }),
            "encoded bytes",
        );
        limited(
            source.build(ReleaseBuildLimits {
                max_decoded_bytes: limits.max_decoded_bytes - 1,
                ..limits
            }),
            "decoded bytes",
        );
        limited(
            parse_release_manifest(
                &source.manifest,
                ReleaseBuildLimits {
                    max_sources: limits.max_sources - 1,
                    ..limits
                },
            ),
            "source count",
        );
        let manifest = parse_release_manifest(&source.manifest, limits).unwrap();
        limited(
            build_release(
                &manifest,
                &source.sources(),
                ReleaseBuildLimits {
                    max_sources: limits.max_sources - 1,
                    ..limits
                },
            ),
            "source count",
        );
    }
    let text = Sources::new(&fixture().input, false).manifest;
    for limits in [
        ReleaseBuildLimits {
            max_sources: 0,
            ..Default::default()
        },
        ReleaseBuildLimits {
            max_encoded_bytes: 0,
            ..Default::default()
        },
        ReleaseBuildLimits {
            max_decoded_bytes: 0,
            ..Default::default()
        },
        ReleaseBuildLimits {
            max_encoded_bytes: ReleaseBuildLimits::default().max_encoded_bytes + 1,
            ..Default::default()
        },
    ] {
        invalid(
            parse_release_manifest(&text, limits),
            "invalid release source limits",
        );
    }
}

#[test]
fn gzip_requires_one_complete_member_without_trailing_data() {
    for case in 0..5 {
        let mut source = Sources::new(&fixture().input, true);
        let (path, bytes) = source
            .files
            .iter_mut()
            .find(|(path, _)| path == "metrics.json.gz")
            .unwrap();
        let expected_path = path.clone();
        match case {
            0 => {
                let crc = bytes.len() - 8;
                bytes[crc] ^= 1;
            }
            1 => {
                bytes.truncate(bytes.len() - 1);
            }
            2 => bytes.extend_from_slice(b"trailing"),
            3 => bytes.extend(gzip(b"{}")),
            _ => bytes.clear(),
        }
        match source.build(Default::default()) {
            Err(ReleaseBuildError::Compression { path, .. })
                if case == 0 || case == 1 || case == 4 =>
            {
                assert_eq!(path, expected_path)
            }
            Err(ReleaseBuildError::Invalid(reason)) if case == 2 || case == 3 => assert_eq!(
                reason,
                "gzip source contains trailing bytes or another member"
            ),
            Err(other) => panic!("case {case}: {other}"),
            Ok(_) => panic!("case {case} unexpectedly succeeded"),
        }
    }
}

#[test]
fn constituent_json_is_strict_and_reports_its_source() {
    for compressed in [false, true] {
        for suffix in [b" {}".as_slice(), b" trailing".as_slice()] {
            let mut source = Sources::new(&fixture().input, compressed);
            let target = if compressed {
                "metrics.json.gz"
            } else {
                "metrics.json"
            };
            let (_, bytes) = source
                .files
                .iter_mut()
                .find(|(path, _)| path == target)
                .unwrap();
            let mut json =
                serde_json::to_vec(&fixture().input.evaluation.as_ref().unwrap().metrics).unwrap();
            json.extend_from_slice(suffix);
            *bytes = if compressed { gzip(&json) } else { json };
            match source.build(Default::default()) {
                Err(ReleaseBuildError::Json { path, .. }) => assert_eq!(path, target),
                Err(other) => panic!("unexpected error: {other}"),
                Ok(_) => panic!("trailing JSON was accepted"),
            }
        }
    }
}

#[test]
fn manifest_rejects_paths_unknown_fields_and_duplicate_components_or_queries() {
    let source = Sources::new(&fixture().input, false);
    for path in [
        "",
        "/absolute.json",
        "../escape.json",
        "nested/../escape.json",
        "./same.json",
        "a//b.json",
        "a/",
        "C:/outside.json",
    ] {
        let text = source
            .manifest
            .replace("\"schema.json\"", &format!("\"{path}\""));
        invalid(
            parse_release_manifest(&text, Default::default()),
            "source must be a portable relative path",
        );
    }
    for (old, new, reason) in [
        (
            "release {",
            "release extra=\"no\" {",
            "invalid release root fields",
        ),
        (
            "schema \"schema.json\" encoding=\"json\"",
            "schema \"schema.json\" encoding=\"zstd\"",
            "unknown source encoding",
        ),
        (
            "schema \"schema.json\"",
            "schema \"schema.json\" extra=\"no\"",
            "invalid source declaration fields",
        ),
        (
            "schema \"schema.json\"",
            "unknown \"schema.json\"",
            "unknown source component",
        ),
        (
            "schema \"schema.json\"",
            "schema \"registry.json\"",
            "source path used more than once",
        ),
        (
            "}\n",
            "schema \"second-schema.json\"\n}\n",
            "duplicate source component",
        ),
        (
            "}\n",
            "queries \"same\" source=\"one.json\"\nqueries \"same\" source=\"two.json\"\n}\n",
            "invalid or duplicate query set name",
        ),
        (
            "}\n",
            "queries \"Upper\" source=\"one.json\"\n}\n",
            "invalid or duplicate query set name",
        ),
        (
            "}\n",
            "queries \"-bad\" source=\"one.json\"\n}\n",
            "invalid query set key",
        ),
        (
            "}\n",
            "queries \"_bad\" source=\"one.json\"\n}\n",
            "invalid query set key",
        ),
        ("}\n", "queries \"valid\"\n}\n", "queries require a source"),
    ] {
        invalid(
            parse_release_manifest(&source.manifest.replace(old, new), Default::default()),
            reason,
        );
    }
    let text = source
        .manifest
        .lines()
        .filter(|line| !line.trim_start().starts_with("schema "))
        .collect::<Vec<_>>()
        .join("\n");
    invalid(
        parse_release_manifest(&text, Default::default()),
        "missing required source component",
    );
    let text = source.manifest.replace(
        "}\n",
        "queries \"one\" source=\"one.json\"\nqueries \"two\" source=\"two.json\"\n}\n",
    );
    let mut limits = ReleaseBuildLimits::default();
    limits.release.max_query_sets = 1;
    limited(parse_release_manifest(&text, limits), "query sets");
}

#[test]
fn expected_identity_and_actual_metric_authority_are_checked_after_decode() {
    let mut source = Sources::new(&fixture().input, false);
    let expected = assemble_owned_release(fixture().input.clone(), Default::default()).unwrap();
    source.manifest = source.manifest.replacen(
        "release {",
        &format!("release expected_input=\"{}\" {{", expected.receipt().input),
        1,
    );
    let manifest = parse_release_manifest(&source.manifest, Default::default()).unwrap();
    assert_eq!(manifest.expected_input(), Some(expected.receipt().input));
    assert_eq!(
        source
            .build(Default::default())
            .unwrap()
            .release()
            .receipt(),
        expected.receipt()
    );
    source.manifest = source
        .manifest
        .replace(&expected.receipt().input.to_string(), &"0".repeat(64));
    invalid(
        source.build(Default::default()),
        "assembled input identity differs from expected_input",
    );

    let mut input = fixture().input.clone();
    let metrics = &mut input.evaluation.as_mut().unwrap().metrics;
    metrics.bindings[0].metric = DefId::new(
        metrics.namespace.clone(),
        OwnedDefinitionKey::new("missing-metric").unwrap(),
    );
    match Sources::new(&input, false).build(Default::default()) {
        Err(ReleaseBuildError::Assembly(OwnedReleaseError::Evaluation(_))) => {}
        Err(other) => panic!("unexpected error: {other}"),
        Ok(_) => panic!("an unregistered metric bypassed the existing checker"),
    }
}

#[test]
fn tracked_bootstrap_rebuilds_every_reference_artifact_without_local_predecessors() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/owned/poe2/3887ae68/migration-source");
    let text = fs::read_to_string(root.join("release.kdl")).unwrap();
    let reference_bytes = fs::read(root.join("reference-release.json")).unwrap();
    let reference: OwnedReleaseReceipt = serde_json::from_slice(&reference_bytes).unwrap();
    let manifest = parse_release_manifest(&text, Default::default()).unwrap();
    assert_eq!(manifest.expected_input(), Some(reference.input));
    let owned: Vec<_> = manifest
        .sources()
        .iter()
        .map(|spec| {
            (
                spec.path().to_owned(),
                fs::read(root.join(spec.path())).unwrap(),
            )
        })
        .collect();
    let sources: Vec<_> = owned
        .iter()
        .map(|(path, bytes)| ReleaseSource { path, bytes })
        .collect();
    let actual = build_release(&manifest, &sources, Default::default()).unwrap();
    let release = actual.release();
    assert_eq!(release.receipt(), &reference);
    let artifacts: BTreeMap<_, _> = release.artifacts().collect();
    assert_eq!(artifacts.len(), reference.artifacts.len() + 1);
    assert_eq!(artifacts["release.json"], reference_bytes);
    assert!(
        reference
            .artifacts
            .iter()
            .any(|row| row.file == "manifest.json")
    );
    for row in &reference.artifacts {
        let bytes = artifacts[row.file.as_str()];
        assert_eq!(bytes.len(), row.bytes, "{} length", row.file);
        assert_eq!(
            format!("{:x}", Sha256::digest(bytes)),
            row.sha256,
            "{} digest",
            row.file
        );
    }
    // The supplied constituents are exact retained bytes, while manifest.json
    // is independently derived by the existing recipe assembler.
    let mut decoded_bytes = 0;
    for (spec, (_, encoded)) in manifest.sources().iter().zip(&owned) {
        assert_eq!(spec.encoding(), SourceEncoding::Gzip);
        let mut decoded = Vec::new();
        GzDecoder::new(encoded.as_slice())
            .read_to_end(&mut decoded)
            .unwrap();
        decoded_bytes += decoded.len();
        let name = Path::new(spec.path())
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .strip_suffix(".gz")
            .unwrap();
        if name == "provenance.json" {
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&decoded).unwrap(),
                serde_json::to_value(&reference.provenance).unwrap()
            );
        } else {
            assert_eq!(artifacts[name], decoded, "retained constituent: {name}");
        }
    }
    assert_eq!(
        actual.encoded_bytes(),
        owned.iter().map(|(_, bytes)| bytes.len()).sum::<usize>()
    );
    assert_eq!(actual.decoded_bytes(), decoded_bytes);
    assert_eq!(release.input().provenance, reference.provenance);
    assert!(
        release.evaluation().is_none(),
        "the bootstrap makes no evaluation coverage claim"
    );
}

fn cli(manifest: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-author"))
        .args(["release", "build"])
        .arg(manifest)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}

fn directory_files(path: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(path)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(entry.file_type().unwrap().is_file());
            (
                entry.file_name().into_string().unwrap(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}

fn directory_entries(path: &Path) -> BTreeSet<std::ffi::OsString> {
    fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect()
}

#[test]
fn cli_build_publishes_exact_artifacts_and_failures_leave_no_output_or_staging() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("release inputs λ");
    fs::create_dir(&input).unwrap();
    let source = Sources::new(&fixture().input, true);
    for (path, bytes) in &source.files {
        fs::write(input.join(path), bytes).unwrap();
    }
    let manifest = input.join("release.kdl");
    fs::write(&manifest, &source.manifest).unwrap();
    let initial_sources = directory_files(&input);
    let output = temp.path().join("published release μ");
    let expected = assemble_owned_release(fixture().input.clone(), Default::default()).unwrap();
    let expected_files = files(&expected);
    let result = cli(&manifest, &output);
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stderr.is_empty());
    let mut receipt = expected_files["release.json"].clone();
    receipt.push(b'\n');
    assert_eq!(result.stdout, receipt);
    assert_eq!(directory_files(&output), expected_files);
    assert_eq!(directory_files(&input), initial_sources);
    let published_entries = directory_entries(temp.path());

    let repeated = cli(&manifest, &output);
    assert_eq!(repeated.status.code(), Some(2));
    assert!(repeated.stdout.is_empty());
    assert!(String::from_utf8_lossy(&repeated.stderr).contains("cannot publish release"));
    assert_eq!(directory_files(&output), expected_files);
    assert_eq!(directory_entries(temp.path()), published_entries);

    for (text, message) in [
        (
            "release {\n unknown \"oops.json\"\n}\n".into(),
            "unknown source component",
        ),
        (
            source.manifest.replacen(
                "release {",
                &format!("release expected_input=\"{}\" {{", "0".repeat(64)),
                1,
            ),
            "assembled input identity differs from expected_input",
        ),
    ] {
        fs::write(&manifest, text).unwrap();
        let failed_output = temp.path().join("must not exist ν");
        let before = directory_entries(temp.path());
        let result = cli(&manifest, &failed_output);
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(message),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!failed_output.exists());
        assert_eq!(directory_entries(temp.path()), before);
        assert_eq!(directory_files(&output), expected_files);
    }
    fs::write(&manifest, &source.manifest).unwrap();
    assert_eq!(directory_files(&input), initial_sources);
}

#[cfg(unix)]
#[test]
fn cli_refuses_a_source_symlink_outside_the_manifest_directory() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input");
    fs::create_dir(&input).unwrap();
    let source = Sources::new(&fixture().input, false);
    for (path, bytes) in &source.files {
        if path == "metrics.json" {
            let outside = temp.path().join("outside-metrics.json");
            fs::write(&outside, bytes).unwrap();
            std::os::unix::fs::symlink(&outside, input.join(path)).unwrap();
        } else {
            fs::write(input.join(path), bytes).unwrap();
        }
    }
    let manifest = input.join("release.kdl");
    fs::write(&manifest, source.manifest).unwrap();
    let output = temp.path().join("output");
    let before = directory_entries(temp.path());
    let result = cli(&manifest, &output);
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("source resolves outside the manifest directory")
    );
    assert!(!output.exists());
    assert_eq!(directory_entries(temp.path()), before);
}
