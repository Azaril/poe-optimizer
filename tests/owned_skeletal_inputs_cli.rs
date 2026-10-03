//! Reviewed skeletal topology and physical inputs preserve every original build.
#[path = "support/owned_skeletal_inputs.rs"]
mod family;
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_import::{
    owned_normalize::{GemInventoryPolicy, PrimaryGemInputDisposition},
    owned_release::StagedOwnedRelease,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn command(name: &str, args: &[&Path]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg(name)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn publish(input: &Path, output: &Path) -> Value {
    command(
        "assemble-owned-release",
        &[input, Path::new("--output"), output],
    )
}
fn dispositions(next: &StagedOwnedRelease) -> Vec<PrimaryGemInputDisposition> {
    let authored = family::dispositions();
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 {
        primary_dispositions,
        ..
    }) = &next.input().normalization.gem_inventory
    else {
        panic!("V3 inventory");
    };
    authored
        .iter()
        .map(|row| {
            let found: Vec<_> = primary_dispositions
                .iter()
                .filter(|candidate| candidate.physical.gem == row.physical.gem)
                .collect();
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].physical, row.physical);
            found[0].clone()
        })
        .collect()
}

#[test]
fn skeletal_packet_declares_partial_topology_and_exact_physical_dispositions() {
    family::check_authored();
}

fn expected_target(row: &PrimaryGemInputDisposition, locator: &Value, set: usize) -> Value {
    let r = json!(row.reference_action);
    let a = &r["actions"][0];
    let provider = |grants: Vec<Value>| json!({"skill_use":locator,"grant_path":grants});
    json!({"kind":"action","value":{
        "actor":{"kind":"owned","value":{"provider":provider(vec![r["entering_grant"].clone()]),
            "slot":r["minion"]["population"]}},
        "provider":provider(vec![r["entering_grant"].clone(),r["minion"]["entering_grant"].clone(),a["entering_grant"].clone()]),
        "output":a["output"],"part":a["part"],"mode":a["mode"],"stat_set":a["stat_sets"][set]["stat_set"]
    }})
}

fn resolve(package: &Path, source: &Path, adapter: &Path, request: &Path) -> Value {
    let report = command(
        "resolve-owned-action",
        &[
            source,
            Path::new("--release"),
            package,
            Path::new("--correspondence"),
            adapter,
            Path::new("--request"),
            request,
        ],
    );
    for field in ["source_execution", "calculation", "whole_build_parity"] {
        assert_eq!(report[field], false);
    }
    assert_eq!(report["document_kind"], "owned_source_action_resolution");
    report
}

fn original_correspondence(
    package: &Path,
    out: &Path,
    rows: &[PrimaryGemInputDisposition],
) -> Vec<Value> {
    let mut reports = vec![];
    for case in [1, 5] {
        let path = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let xml = fs::read(&path).unwrap();
        for (family_index, row) in rows.iter().enumerate() {
            let frame = preservation::frame(&xml, &[&row.physical]);
            let adapter = out.join(format!("adapter-{family_index}.json"));
            write(&adapter, &row.reference_action);
            for loc in frame.locations {
                let locator = json!({"source_sha256":format!("{:x}",Sha256::digest(&xml)),
                    "occurrence_ordinal":loc.ordinal,"expected_gem":row.physical.gem});
                for context in ["main", "calcs"] {
                    let request = out.join(format!(
                        "reference-{case:02}-{}-{context}.json",
                        loc.ordinal
                    ));
                    write(&request, &json!({"skill_use":locator,"context":context}));
                    let report = resolve(package, &path, &adapter, &request);
                    assert_eq!(
                        report["report"]["target"],
                        expected_target(row, &locator, 0)
                    );
                    reports.push(json!({"original":case,"source_ordinal":loc.ordinal,
                        "context":context,"target":report["report"]["target"]}));
                }
            }
        }
    }
    assert_eq!(reports.len(), 30);
    reports
}

fn controls(
    package: &Path,
    prior: &Path,
    out: &Path,
    rows: &[PrimaryGemInputDisposition],
) -> Vec<Value> {
    let source = root().join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let original = fs::read_to_string(&source).unwrap();
    let mut reports = vec![];
    for (index, row) in rows.iter().enumerate() {
        let frame = preservation::frame(original.as_bytes(), &[&row.physical]);
        let chosen: Vec<_> = frame.locations.iter().filter(|loc| loc.selected).collect();
        assert_eq!(chosen.len(), 1);
        let loc = chosen[0];
        let r = json!(row.reference_action);
        let last = r["actions"][0]["stat_sets"].as_array().unwrap().len();
        let effect = &row.physical.skill_id;
        let maps = format!(
            "<MinionSkillIndexLookup grantedEffect=\"{effect}\"><MinionSkillIndexMap skillIndex=\"1\" statSetIndex=\"{last}\"/></MinionSkillIndexLookup><MinionSkillIndexLookupCalcs grantedEffect=\"{effect}\"><MinionSkillIndexMap skillIndex=\"1\" statSetIndex=\"{last}\"/></MinionSkillIndexLookupCalcs>"
        );
        let adapter = out.join(format!("adapter-{index}.json"));
        for (name, changes, children, complete) in [
            ("last-stat-set", vec![], maps.as_str(), true),
            (
                "unreviewed-child",
                vec![
                    ("skillMinionSkill", Some("2")),
                    ("skillMinionSkillCalcs", Some("2")),
                ],
                "",
                false,
            ),
            (
                "unknown-actor",
                vec![
                    ("skillMinion", Some("unreviewed-actor")),
                    ("skillMinionCalcs", Some("unreviewed-actor")),
                ],
                "",
                false,
            ),
        ] {
            let mut xml = original.clone();
            xml.replace_range(
                loc.range.clone(),
                &format!(
                    "{}{children}</Gem>",
                    preservation::header("Gem", &loc.attributes, &changes)
                ),
            );
            let path = out.join(format!("control-{index}-{name}.xml"));
            fs::write(&path, &xml).unwrap();
            let old_dir = out.join(format!("prior-control-{index}-{name}"));
            let dir = out.join(format!("control-{index}-{name}"));
            release::normalize(prior, &path, 5, &old_dir);
            release::normalize(package, &path, 5, &dir);
            let mut old: Value = read(old_dir.join("draft.json"));
            let mut draft: Value = read(dir.join("draft.json"));
            let mut old_side: Value = read(old_dir.join("sidecar.json"));
            let mut side: Value = read(dir.join("sidecar.json"));
            for v in [&mut old, &mut draft, &mut old_side, &mut side] {
                selected::canonical(v);
            }
            assert_eq!(old["draft"]["allocator"], draft["draft"]["allocator"]);
            let changed = preservation::frame(xml.as_bytes(), &[&row.physical]);
            for occurrence in &changed.locations {
                let id = preservation::link(&side, occurrence.ordinal, "gem");
                assert_eq!(id, preservation::link(&old_side, occurrence.ordinal, "gem"));
                let before = preservation::member(&old, "gems", &id);
                let after = preservation::member(&draft, "gems", &id);
                for field in ["id", "definition", "level", "quality"] {
                    assert_eq!(before[field], after[field]);
                }
                assert_eq!(
                    before["parameters"]["members"],
                    after["parameters"]["members"]
                );
                assert_eq!(
                    after["parameters"]["completion"]["kind"],
                    if !occurrence.selected || complete {
                        "complete"
                    } else {
                        "pending"
                    }
                );
                assert_eq!(
                    preservation::preset_usage(&old, &old_side, occurrence),
                    preservation::preset_usage(&draft, &side, occurrence)
                );
            }
            let target = changed.locations.iter().find(|loc| loc.selected).unwrap();
            let locator = json!({"source_sha256":format!("{:x}",Sha256::digest(xml.as_bytes())),
                "occurrence_ordinal":target.ordinal,"expected_gem":row.physical.gem});
            for context in ["main", "calcs"] {
                let request = out.join(format!("control-{index}-{name}-{context}.json"));
                write(&request, &json!({"skill_use":locator,"context":context}));
                let report = resolve(package, &path, &adapter, &request);
                if complete {
                    assert_eq!(
                        report["report"]["target"],
                        expected_target(row, &locator, last - 1)
                    );
                } else {
                    assert_eq!(report["report"]["target"]["kind"], "unresolved");
                    assert_eq!(report["report"]["selection"]["kind"], "pending");
                }
            }
            reports.push(json!({"family":row.physical.game_id,"control":name,
                "physical_complete":complete,"usage_pending":true,"same_mutated_source":true}));
        }
    }
    assert_eq!(fs::read_to_string(source).unwrap(), original);
    reports
}

#[test]
#[ignore = "requires exact Sniper inventory release and authenticated two-mode skeletal source reports"]
fn publish_skeletal_inputs_preserving_five_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SKELETAL_INPUTS_PRIOR").expect("exact predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SKELETAL_INPUTS_OUTPUT").expect("new output"),
    );
    assert!(!out.exists());
    let before = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    write(out.join("prior-receipt.json"), prior.receipt());
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
    let after = release::inventory(&package);
    assert_eq!(after.len(), 18);
    assert_eq!(after, release::inventory(&out.join("rebuilt")));
    let rows = dispositions(&next);
    let expectations: Vec<_> = rows
        .iter()
        .zip([[1, 0, 0, 0, 5], [1, 0, 0, 0, 3], [1, 0, 0, 0, 4]])
        .map(|(row, occurrences)| preservation::InventoryExpectation {
            physical: &row.physical,
            occurrences,
            parameter_count: 2,
        })
        .collect();
    let comparison = preservation::Comparison {
        prior: &prior,
        next: &next,
        prior_path: &prior_path,
        package: &package,
        out: &out,
        families: &expectations,
        selected_before: [118, 116, 108, 121, 17],
        selected_after: [115, 116, 108, 121, 14],
        rebind_definitions: true,
    };
    let mut originals = vec![];
    for case in 1..=5 {
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        release::normalize(
            &prior_path,
            &xml,
            case,
            &out.join(format!("prior-original-{case:02}")),
        );
        release::normalize(
            &package,
            &xml,
            case,
            &out.join(format!("original-{case:02}")),
        );
        originals.push(preservation::compare_original(
            case,
            &fs::read(xml).unwrap(),
            &comparison,
        ));
    }
    let correspondence = original_correspondence(&package, &out, &rows);
    let controls = controls(&package, &prior_path, &out, &rows);
    assert_eq!(before, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({
            "before":prior.receipt().input,"after":next.receipt().input,"originals":originals,
            "controls":controls,"correspondence":correspondence,"queries":110,"artifacts":18,
            "rebuild_byte_identical":true,"prior_unchanged":true,"physical_occurrences":15,
            "complete_original_builds":0,"usage_inventory":"pending","native_mechanics":"partial"
        }),
    );
}
