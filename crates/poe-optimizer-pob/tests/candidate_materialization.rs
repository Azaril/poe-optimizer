//! Finite whole-build mapping and fail-closed realization checks. Lua executes only
//! in a dedicated test child, preserving the application's process ownership model.
use poe_optimizer_core::{
    EvaluationSnapshot,
    evaluation::{BackendIdentity, BuildDocument, BuildFormat, EvaluationResult},
};
use poe_optimizer_pob::candidate::{
    CandidateBridgeError, PobBuildAlternative, PobCandidateCatalog,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const FIXTURES: &[&str] = &[
    "mace-wooden",
    "mace-wooden-brutality",
    "mace-smithing",
    "mace-smithing-brutality",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

fn builds() -> Vec<PobBuildAlternative> {
    FIXTURES
        .iter()
        .map(|name| PobBuildAlternative {
            id: (*name).into(),
            xml: fs::read_to_string(root().join(format!("tests/fixtures/calibration/{name}.xml")))
                .unwrap(),
        })
        .collect()
}

#[test]
fn registry_identity_is_order_independent_and_scopes_each_finite_subset() {
    let original = PobCandidateCatalog::from_builds(builds()).unwrap();
    let mut reversed = builds();
    reversed.reverse();
    let reversed = PobCandidateCatalog::from_builds(reversed).unwrap();
    assert_eq!(original.catalog(), reversed.catalog());
    assert_eq!(
        original.alternatives()[0].candidate,
        reversed.alternatives()[0].candidate
    );
    let subset = PobCandidateCatalog::from_builds(vec![builds().remove(0)]).unwrap();
    assert_ne!(original.catalog().identity, subset.catalog().identity);
    assert!(matches!(
        original.materialize(&subset.alternatives()[0].candidate),
        Err(CandidateBridgeError::UnknownCandidate)
    ));
}

#[test]
fn registry_rejects_changed_xml_and_ambiguous_or_empty_inputs() {
    assert!(PobCandidateCatalog::from_builds(Vec::new()).is_err());
    let input = builds().remove(0);
    let mut changed = input.clone();
    changed.xml = changed.xml.replace("level=\"60\"", "level=\"61\"");
    assert!(matches!(
        PobCandidateCatalog::from_builds(vec![changed]),
        Err(CandidateBridgeError::UnsupportedSource)
    ));
    let mut unknown = input.clone();
    unknown.xml = unknown.xml.replace(
        "</PathOfBuilding2>",
        "<Extension enabled=\"true\"/></PathOfBuilding2>",
    );
    assert!(matches!(
        PobCandidateCatalog::from_builds(vec![unknown]),
        Err(CandidateBridgeError::UnsupportedSource)
    ));
    let mut duplicate = input.clone();
    duplicate.id = "different label".into();
    assert!(PobCandidateCatalog::from_builds(vec![input.clone(), duplicate]).is_err());
    let mut other = builds().remove(1);
    other.id.clone_from(&input.id);
    assert!(PobCandidateCatalog::from_builds(vec![input, other]).is_err());
}

#[test]
fn exact_payloads_distinguish_item_rolls_and_preserve_source_bytes() {
    let inputs = builds();
    let registry = PobCandidateCatalog::from_builds(inputs.clone()).unwrap();
    let catalog = registry.catalog();
    let domain = poe_optimizer_core::candidate::CandidateDomain::new(
        catalog.clone(),
        poe_optimizer_core::candidate::CandidateConstraints {
            budgets: poe_optimizer_core::candidate::CandidateBudgets {
                active_skill_count: 1,
                supports_per_skill: 1,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    for alternative in registry.alternatives() {
        assert!(domain.validate(&alternative.candidate).is_searchable());
    }
    assert_eq!(catalog.items.len(), 2);
    assert_eq!(catalog.active_skills.len(), 1);
    assert_eq!(catalog.supports.len(), 1);
    assert_eq!(catalog.classes["6"].start_node_id, 47175);
    for payload in catalog
        .items
        .values()
        .map(|item| &item.payload)
        .chain(catalog.active_skills.values().map(|skill| &skill.payload))
        .chain(catalog.supports.values().map(|support| &support.payload))
    {
        assert_eq!(
            payload.sha256,
            format!("{:x}", Sha256::digest(payload.content.as_bytes()))
        );
    }
    for alternative in registry.alternatives() {
        assert!(
            alternative.candidate.passives.is_empty(),
            "implicit class root is not a paid allocation"
        );
        let original = inputs
            .iter()
            .find(|input| input.id == alternative.id)
            .unwrap();
        let materialized = registry.materialize(&alternative.candidate).unwrap();
        assert_eq!(materialized.format, BuildFormat::PathOfBuilding2Xml);
        assert_eq!(materialized.content.as_bytes(), original.xml.as_bytes());
        assert_eq!(
            alternative.xml_sha256,
            format!("{:x}", Sha256::digest(original.xml.as_bytes()))
        );
    }
    let ids: std::collections::BTreeSet<_> = registry
        .alternatives()
        .iter()
        .map(|alternative| alternative.candidate.equipment["Weapon 1"].clone())
        .collect();
    assert_eq!(
        ids.len(),
        2,
        "XML item id=1 must not merge different weapons"
    );
}

#[test]
fn mutations_outside_exact_registered_states_are_not_materialized() {
    let registry = PobCandidateCatalog::from_builds(builds()).unwrap();
    let original = &registry.alternatives()[0].candidate;
    let mut changed = original.clone();
    changed.class_id = "7".into();
    assert!(matches!(
        registry.materialize(&changed),
        Err(CandidateBridgeError::UnknownCandidate)
    ));
    let mut changed = original.clone();
    changed.passives.insert(3936);
    assert!(registry.materialize(&changed).is_err());
    let mut changed = original.clone();
    changed.equipment.clear();
    assert!(registry.materialize(&changed).is_err());
    let mut changed = original.clone();
    changed.skills.clear();
    assert!(registry.materialize(&changed).is_err());
}

fn clone_result(result: &EvaluationResult) -> EvaluationResult {
    serde_json::from_slice(&serde_json::to_vec(result).unwrap()).unwrap()
}

fn evaluate_in_child(xml: &str) -> EvaluationResult {
    let scratch = tempfile::tempdir().unwrap();
    let input = scratch.path().join("input.xml");
    let output = scratch.path().join("output.json");
    let diagnostics = scratch.path().join("stderr.log");
    let worker_scratch = scratch.path().join("worker");
    fs::create_dir(&worker_scratch).unwrap();
    fs::write(&input, xml).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "materialization_worker",
            "--ignored",
            "--nocapture",
        ])
        .current_dir(root().join("vendor/path-of-building-poe2/src"))
        .env("POE_OPTIMIZER_CANDIDATE_INPUT", &input)
        .env("POE_OPTIMIZER_CANDIDATE_OUTPUT", &output)
        .env("POE_OPTIMIZER_CANDIDATE_SCRATCH", &worker_scratch)
        .stdout(Stdio::null())
        .stderr(Stdio::from(fs::File::create(&diagnostics).unwrap()))
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(90);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("candidate realization child exceeded 90 seconds");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(
        status.success(),
        "{}",
        fs::read_to_string(&diagnostics).unwrap()
    );
    let snapshot: EvaluationSnapshot = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
    let measurements = poe_optimizer_pob::metrics::measurements(&snapshot);
    EvaluationResult {
        backend: BackendIdentity {
            data: None,
            id: "pob-poe2-mlua".into(),
            implementation_version: env!("CARGO_PKG_VERSION").into(),
            rules_revision: snapshot.runtime.upstream_revision,
            source_fingerprint: snapshot.runtime.source_hash,
            adapter_fingerprint: snapshot.runtime.adapter_hash,
        },
        build: snapshot.build,
        context: snapshot.context,
        coverage: snapshot.coverage,
        measurements,
        exports: vec![BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: snapshot.export_xml,
        }],
        warnings: snapshot.warnings,
        elapsed_ms: snapshot.elapsed_ms,
        diagnostic_only: true,
        attachments: Vec::new(),
    }
}

#[test]
fn fresh_pob_exports_preserve_every_registered_candidate_and_detect_drift() {
    let registry = PobCandidateCatalog::from_builds(builds()).unwrap();
    for alternative in registry.alternatives() {
        let document = registry.materialize(&alternative.candidate).unwrap();
        let result = evaluate_in_child(&document.content);
        registry
            .validate_realization(&alternative.candidate, &result)
            .unwrap_or_else(|error| panic!("{}: {error}", alternative.id));
        let mut changed = clone_result(&result);
        changed.build.allocated_nodes.push(3936);
        assert!(
            registry
                .validate_realization(&alternative.candidate, &changed)
                .is_err()
        );
        let mut changed = clone_result(&result);
        changed.coverage.groups[0].gems[0].quality = Some(20.0);
        assert!(
            registry
                .validate_realization(&alternative.candidate, &changed)
                .is_err()
        );
        let mut changed = clone_result(&result);
        changed.coverage.selected_player.as_mut().unwrap().skill_id =
            Some("MeleeUnarmedPlayer".into());
        assert!(
            registry
                .validate_realization(&alternative.candidate, &changed)
                .is_err()
        );
        let mut changed = clone_result(&result);
        changed.exports[0].content = changed.exports[0]
            .content
            .replace("LevelReq: 0", "LevelReq: 99");
        assert!(
            registry
                .validate_realization(&alternative.candidate, &changed)
                .is_err()
        );
        let mut changed = clone_result(&result);
        changed.exports[0].content = changed.exports[0]
            .content
            .replace("classInternalId=\"6\"", "classInternalId=\"7\"");
        assert!(
            registry
                .validate_realization(&alternative.candidate, &changed)
                .is_err()
        );
        let mut changed = clone_result(&result);
        changed.context.config_inputs.insert(
            "enemyArmour".into(),
            poe_optimizer_core::options::Scalar::Number(99_999.0),
        );
        assert!(
            registry
                .validate_realization(&alternative.candidate, &changed)
                .is_err()
        );
        let mut changed = clone_result(&result);
        changed.context.config_inputs.insert(
            "customMods".into(),
            poe_optimizer_core::options::Scalar::Text("100% more Damage".into()),
        );
        assert!(
            registry
                .validate_realization(&alternative.candidate, &changed)
                .is_err()
        );
        let mut changed = clone_result(&result);
        changed.context.config_placeholders.insert(
            "enemyEvasion".into(),
            poe_optimizer_core::options::Scalar::Number(0.0),
        );
        assert!(
            registry
                .validate_realization(&alternative.candidate, &changed)
                .is_err()
        );
        for (from, to) in [
            ("activeConfigSet=\"1\"", "activeConfigSet=\"2\""),
            (
                "</ConfigSet>",
                "<Input name=\"customMods\" string=\"100% more Damage\"/></ConfigSet>",
            ),
            (
                "</CustomModifierBlock>",
                "100% more Damage</CustomModifierBlock>",
            ),
            (
                "</Item>",
                "<ModRange id=\"1\" range=\"1\"/>Rarity: NORMAL\nSmithing Hammer\n</Item>",
            ),
            (
                "</Item>",
                "<!-- split -->Rarity: NORMAL\nSmithing Hammer\n</Item>",
            ),
            ("runeName=\"None\"", "runeName=\"Iron Rune\""),
            ("statSetIndex=\"nil\"", "statSetIndex=\"2\""),
        ] {
            let mut changed = clone_result(&result);
            changed.exports[0].content = changed.exports[0].content.replace(from, to);
            assert_ne!(
                changed.exports[0].content, result.exports[0].content,
                "mutation {from} matched fixture"
            );
            assert!(
                registry
                    .validate_realization(&alternative.candidate, &changed)
                    .is_err(),
                "accepted export mutation {from}"
            );
        }
        let mut changed = clone_result(&result);
        changed.backend.source_fingerprint = "0".repeat(64);
        assert!(
            registry
                .validate_realization(&alternative.candidate, &changed)
                .is_err()
        );
        let mut changed = clone_result(&result);
        changed.exports.clear();
        assert!(
            registry
                .validate_realization(&alternative.candidate, &changed)
                .is_err()
        );
    }
}

#[test]
#[ignore = "child process fixture invoked by fresh_pob_exports_preserve_every_registered_candidate_and_detect_drift"]
fn materialization_worker() {
    let input =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_CANDIDATE_INPUT").expect("child-only input"));
    let output = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_CANDIDATE_OUTPUT").expect("child-only output"),
    );
    let scratch = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_CANDIDATE_SCRATCH").expect("child-only scratch"),
    );
    let result = poe_optimizer_pob::runtime::evaluate(
        &root().join("vendor/path-of-building-poe2"),
        &scratch,
        &fs::read_to_string(input).unwrap(),
    )
    .unwrap();
    fs::write(output, serde_json::to_vec(&result).unwrap()).unwrap();
}
