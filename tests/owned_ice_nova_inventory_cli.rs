//! Physical list completeness preserves unresolved usage and exact source ownership.
#[path = "support/owned_ice_nova_inventory.rs"]
mod family;
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use preservation::{header, link, member, origin, preset_usage};

use poe_optimizer_import::owned_release::assemble_owned_release;
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
fn publish(input: &Path, output: &Path) -> Value {
    let result = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}

#[test]
fn ice_nova_disposition_packet_binds_physical_values_and_defers_usage() {
    family::check_authored();
}

fn frame(xml: &[u8]) -> preservation::Frame {
    preservation::frame(xml, &[&family::disposition().physical])
}

fn controls(prior_package: &Path, package: &Path, out: &Path) -> Vec<Value> {
    let path = root().join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let original = fs::read_to_string(&path).unwrap();
    let f = frame(original.as_bytes());
    let target = f.locations.iter().find(|l| l.selected).unwrap();
    assert_eq!(f.locations.iter().filter(|l| l.selected).count(), 1);
    let effect = family::disposition().physical.skill_id;
    let unrelated = "<StatSetIndex grantedEffect=\"unreviewed-effect\" index=\"1\"/>";
    let maps = format!(
        "<StatSetIndex grantedEffect=\"{effect}\" index=\"2\"/><StatSetCalcsIndex grantedEffect=\"{effect}\" index=\"1\"/>"
    );
    struct Control<'a> {
        name: &'a str,
        gem: Vec<(&'a str, Option<&'a str>)>,
        group: Vec<(&'a str, Option<&'a str>)>,
        children: &'a str,
        complete: bool,
    }
    let cases = [
        Control {
            name: "unknown-gem-field",
            gem: vec![("unreviewed", Some("true"))],
            group: vec![],
            children: "",
            complete: false,
        },
        Control {
            name: "unknown-group-field",
            gem: vec![],
            group: vec![("unreviewed", Some("true"))],
            children: "",
            complete: false,
        },
        Control {
            name: "unrelated-child-map",
            gem: vec![],
            group: vec![],
            children: unrelated,
            complete: false,
        },
        Control {
            name: "malformed-count",
            gem: vec![("count", Some("bad"))],
            group: vec![],
            children: "",
            complete: false,
        },
        Control {
            name: "missing-count",
            gem: vec![("count", None)],
            group: vec![],
            children: "",
            complete: false,
        },
        Control {
            name: "count-three-full-dps",
            gem: vec![
                ("count", Some("3")),
                ("enableGlobal1", Some("false")),
                ("enableGlobal2", Some("false")),
            ],
            group: vec![("includeInFullDPS", Some("true"))],
            children: "",
            complete: true,
        },
        Control {
            name: "group-zero",
            gem: vec![("count", Some("3"))],
            group: vec![("groupCount", Some("0"))],
            children: "",
            complete: true,
        },
        Control {
            name: "main-two-calcs-one",
            gem: vec![],
            group: vec![],
            children: &maps,
            complete: true,
        },
    ];
    let mut reports = vec![];
    for c in cases {
        let mut xml = original.clone();
        xml.replace_range(
            target.range.clone(),
            &format!(
                "{}{}</Gem>",
                header("Gem", &target.attributes, &c.gem),
                c.children
            ),
        );
        if !c.group.is_empty() {
            xml.replace_range(
                target.group_header.clone(),
                &header("Skill", &target.group_attributes, &c.group),
            );
        }
        assert_ne!(xml, original);
        let file = out.join(format!("control-{}.xml", c.name));
        fs::write(&file, &xml).unwrap();
        let dir = out.join(format!("control-{}", c.name));
        let prior_dir = out.join(format!("prior-control-{}", c.name));
        // Existing strict source census can itself decline unrelated usage
        // recipes after a mutation. Compare both releases on this exact XML.
        release::normalize(prior_package, &file, 5, &prior_dir);
        release::normalize(package, &file, 5, &dir);
        let mut draft: Value = read(dir.join("draft.json"));
        let mut sidecar: Value = read(dir.join("sidecar.json"));
        selected::canonical(&mut draft);
        selected::canonical(&mut sidecar);
        let mut old: Value = read(prior_dir.join("draft.json"));
        let mut old_sidecar: Value = read(prior_dir.join("sidecar.json"));
        selected::canonical(&mut old);
        selected::canonical(&mut old_sidecar);
        assert_eq!(draft["draft"]["allocator"], old["draft"]["allocator"]);
        assert_eq!(sidecar["source_sha256"], old_sidecar["source_sha256"]);
        let changed = frame(xml.as_bytes());
        assert_eq!(changed.locations.len(), 4);
        for loc in &changed.locations {
            let id = link(&sidecar, loc.ordinal, "gem");
            let old_id = link(&old_sidecar, loc.ordinal, "gem");
            assert_eq!(id, old_id);
            let gem = member(&draft, "gems", &id);
            let old_gem = member(&old, "gems", &old_id);
            for field in ["definition", "level", "quality"] {
                assert_eq!(gem[field], old_gem[field]);
            }
            assert_eq!(
                gem["parameters"]["members"],
                old_gem["parameters"]["members"]
            );
            let complete = !loc.selected || c.complete;
            assert_eq!(
                gem["parameters"]["completion"]["kind"],
                if complete { "complete" } else { "pending" },
                "{} preset {}",
                c.name,
                loc.set_id
            );
            if !complete {
                assert_eq!(
                    gem["parameters"]["completion"]["code"],
                    "gem-parameters-not-converted"
                );
            }
            let (_, usage) = preset_usage(&draft, &sidecar, loc);
            assert!(
                usage == preset_usage(&old, &old_sidecar, loc).1,
                "{}: same edited source preserves existing usage values and obligations",
                c.name
            );
            if loc.selected && c.name == "main-two-calcs-one" {
                assert_eq!(loc.children.len(), 2);
                let skill = link(&sidecar, loc.ordinal, "skill");
                for child in &loc.children {
                    assert_eq!(
                        origin(&sidecar, *child)["links"],
                        json!([{"kind":"gem","value":id},{"kind":"skill","value":skill}])
                    );
                }
            }
        }
        reports.push(json!({"name":c.name,"selected_physical_complete":c.complete,"usage_pending":true,"same_mutated_source":true,"archived_physical_values_preserved":true,"source_sha256":format!("{:x}",Sha256::digest(xml.as_bytes()))}));
    }
    assert_eq!(fs::read_to_string(path).unwrap(), original);
    reports
}

#[test]
#[ignore = "requires the exact Ice Nova action release and authenticated occurrence source reports"]
fn publish_ice_nova_physical_inventory_preserving_usage_and_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ICE_INVENTORY_PRIOR").expect("exact predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ICE_INVENTORY_OUTPUT").expect("new output"),
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
    let changed: Vec<_> = before
        .iter()
        .filter(|(name, hash)| after.get(*name) != Some(*hash))
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(
        changed,
        [
            "normalization.json",
            "release.json",
            "tree-normalization.json"
        ]
    );
    for field in ["roles", "catalog", "scalar_inputs", "usage_inputs"] {
        let mut bad = json!(next.input());
        bad["normalization"]["gem_inventory"][field] = json!("0".repeat(64));
        assert!(
            assemble_owned_release(serde_json::from_value(bad).unwrap(), Default::default())
                .is_err(),
            "stale {field}"
        );
    }
    for field in ["roles", "catalog"] {
        let mut bad = json!(next.input());
        bad["normalization"]["gem_inventory"]["primary_dispositions"][0]["reference_action"]
            [field] = json!("0".repeat(64));
        assert!(
            assemble_owned_release(serde_json::from_value(bad).unwrap(), Default::default())
                .is_err(),
            "stale reference {field}"
        );
    }
    let row = family::disposition();
    let expectations = [preservation::InventoryExpectation {
        physical: &row.physical,
        occurrences: [0, 0, 0, 0, 4],
        parameter_count: 2,
    }];
    let comparison = preservation::Comparison {
        prior: &prior,
        next: &next,
        prior_path: &prior_path,
        package: &package,
        out: &out,
        families: &expectations,
        selected_before: [119, 116, 108, 121, 19],
        selected_after: [119, 116, 108, 121, 18],
        rebind_definitions: false,
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
    let controls = controls(&prior_path, &package, &out);
    assert_eq!(before, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":originals,"controls":controls,"queries":110,"artifacts":18,"rebuild_byte_identical":true,"prior_unchanged":true,"complete_original_builds":0,"usage_inventory":"pending"}),
    );
}
