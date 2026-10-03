//! Physical list completeness preserves unresolved usage and exact source ownership.
#[path = "support/owned_ice_nova_inventory.rs"]
mod family;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::{build_identity::BuildLineage, owned_content::digest_owned};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{ImportQueryTemplate, NormalizationLimits, NormalizationPolicy},
    owned_release::assemble_owned_release,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    ops::Range,
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

struct Location {
    ordinal: usize,
    group: usize,
    set: usize,
    set_id: String,
    selected: bool,
    range: Range<usize>,
    group_header: Range<usize>,
    attributes: Vec<(String, String)>,
    group_attributes: Vec<(String, String)>,
    children: Vec<usize>,
}
struct Frame {
    locations: Vec<Location>,
    source_sets: BTreeMap<usize, usize>,
}
fn frame(xml: &[u8]) -> Frame {
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([93; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let e = SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let rows = e.rows();
    let skills = rows
        .iter()
        .find(|r| r.occurrence().name() == "Skills")
        .unwrap();
    let active = skills
        .attribute("activeSkillSet")
        .unwrap()
        .decoded()
        .unwrap();
    let physical = family::disposition().physical;
    let mut source_sets = BTreeMap::new();
    for row in rows {
        let mut ancestor = Some(row.occurrence().id());
        while let Some(id) = ancestor {
            let a = &rows[id.ordinal() as usize];
            if a.occurrence().name() == "SkillSet" {
                source_sets.insert(
                    row.occurrence().id().ordinal() as usize,
                    id.ordinal() as usize,
                );
                break;
            }
            ancestor = a.occurrence().parent();
        }
    }
    let text = std::str::from_utf8(xml).unwrap();
    let locations = rows
        .iter()
        .filter(|r| {
            r.occurrence().name() == "Gem"
                && r.attribute("gemId")
                    .is_some_and(|a| a.decoded().unwrap() == physical.game_id)
        })
        .map(|row| {
            assert_eq!(
                row.attribute("variantId").unwrap().decoded().unwrap(),
                physical.variant_id
            );
            let group = &rows[row.occurrence().parent().unwrap().ordinal() as usize];
            let set = &rows[group.occurrence().parent().unwrap().ordinal() as usize];
            assert_eq!(set.occurrence().name(), "SkillSet");
            let set_id = set.attribute("id").unwrap().decoded().unwrap().to_owned();
            let start = group.occurrence().range().start;
            Location {
                ordinal: row.occurrence().id().ordinal() as usize,
                group: group.occurrence().id().ordinal() as usize,
                set: set.occurrence().id().ordinal() as usize,
                selected: set_id == active,
                set_id,
                range: row.occurrence().range(),
                group_header: start..start + text[start..].find('>').unwrap() + 1,
                attributes: row
                    .attributes()
                    .iter()
                    .map(|a| (a.origin().name.clone(), a.decoded().unwrap().to_owned()))
                    .collect(),
                group_attributes: group
                    .attributes()
                    .iter()
                    .map(|a| (a.origin().name.clone(), a.decoded().unwrap().to_owned()))
                    .collect(),
                children: row
                    .children()
                    .iter()
                    .map(|id| id.ordinal() as usize)
                    .collect(),
            }
        })
        .collect();
    Frame {
        locations,
        source_sets,
    }
}
fn origin(sidecar: &Value, ordinal: usize) -> &Value {
    let rows: Vec<_> = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["source"]["ordinal"] == ordinal)
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0]
}
fn origin_mut(sidecar: &mut Value, ordinal: usize) -> &mut Value {
    sidecar["origins"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["source"]["ordinal"] == ordinal)
        .unwrap()
}
fn link(sidecar: &Value, ordinal: usize, kind: &str) -> Value {
    let rows: Vec<_> = origin(sidecar, ordinal)["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["kind"] == kind)
        .collect();
    assert_eq!(rows.len(), 1, "exact {kind} source link at {ordinal}");
    rows[0]["value"].clone()
}
fn member<'a>(draft: &'a Value, field: &str, id: &Value) -> &'a Value {
    let rows: Vec<_> = draft["draft"][field]["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| &r["id"] == id)
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0]
}
fn preset_usage(draft: &Value, sidecar: &Value, loc: &Location) -> (Value, Value) {
    let preset_id = link(sidecar, loc.set, "skill_preset");
    let skill = link(sidecar, loc.ordinal, "skill");
    let preset = member(draft, "skill_presets", &preset_id);
    assert_eq!(
        preset["skills"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|id| **id == skill)
            .count(),
        1
    );
    let usage = &preset["usage_preferences"];
    assert_eq!(usage["completion"]["kind"], "pending");
    assert_eq!(
        usage["completion"]["code"],
        "usage-preferences-not-converted"
    );
    (preset_id, usage.clone())
}
fn policy_identity(sidecar: &Value, package: &Path, case: usize) {
    let policy: NormalizationPolicy = read(package.join("normalization.json"));
    let queries: Vec<ImportQueryTemplate> =
        read(package.join(format!("queries-original-{case:02}.json")));
    let expected = digest_owned(
        "owned-normalization-policy-v3",
        &(policy, queries),
        NormalizationLimits::default().max_policy_bytes,
    )
    .unwrap();
    assert_eq!(sidecar["policy"], json!(expected));
}

fn compare(case: usize, xml: &[u8], out: &Path, prior: &Path, package: &Path) -> Value {
    let old = out.join(format!("prior-original-{case:02}"));
    let new = out.join(format!("original-{case:02}"));
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    policy_identity(&sa, prior, case);
    policy_identity(&sb, package, case);
    let before: Value = read(prior.join("release.json"));
    let after: Value = read(package.join("release.json"));
    assert_eq!(sa["tree_policy"], before["tree"]);
    assert_eq!(sb["tree_policy"], after["tree"]);
    for v in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(v);
    }
    assert_eq!(
        a["draft"]["allocator"], b["draft"]["allocator"],
        "local IDs and watermark remain exact"
    );
    assert_eq!(sa["allocator_after"], sb["allocator_after"]);
    let f = frame(xml);
    assert_eq!(f.locations.len(), if case == 5 { 4 } else { 0 });
    let mut retired = BTreeSet::new();
    for loc in &f.locations {
        assert!(loc.children.is_empty());
        let gem_id = link(&sa, loc.ordinal, "gem");
        assert_eq!(link(&sb, loc.ordinal, "gem"), gem_id);
        let gem = member(&a, "gems", &gem_id);
        let next = member(&b, "gems", &gem_id);
        for field in ["id", "definition", "level", "quality"] {
            assert_eq!(gem[field], next[field]);
        }
        assert_eq!(gem["parameters"]["members"], next["parameters"]["members"]);
        assert_eq!(gem["parameters"]["members"].as_array().unwrap().len(), 2);
        assert_eq!(
            gem["parameters"]["completion"]["code"],
            "gem-parameters-not-converted"
        );
        assert_eq!(next["parameters"]["completion"], json!({"kind":"complete"}));
        let issue = gem["parameters"]["completion"]["id"].clone();
        assert!(retired.insert(issue["local"].as_str().unwrap().to_owned()));
        let (preset, usage) = preset_usage(&a, &sa, loc);
        assert_eq!(preset_usage(&b, &sb, loc), (preset.clone(), usage.clone()));
        let usage_issue = &usage["completion"]["id"];
        let existing: Vec<_> = sa["origins"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| {
                r["links"]
                    .as_array()
                    .unwrap()
                    .contains(&json!({"kind":"issue","value":usage_issue}))
            })
            .collect();
        assert!(!existing.is_empty(), "real prior usage obligation");
        for row in existing {
            let ordinal = row["source"]["ordinal"].as_u64().unwrap() as usize;
            assert_eq!(f.source_sets.get(&ordinal), Some(&loc.set));
        }
        let links = origin_mut(&mut sa, loc.ordinal)["links"]
            .as_array_mut()
            .unwrap();
        let before = links.len();
        links.retain(|v| *v != json!({"kind":"issue","value":issue}));
        assert_eq!(before - links.len(), 1);
        for source in [loc.ordinal, loc.group] {
            let links = origin_mut(&mut sa, source)["links"].as_array_mut().unwrap();
            for value in [
                json!({"kind":"skill_preset","value":preset}),
                json!({"kind":"issue","value":usage_issue}),
            ] {
                if !links.contains(&value) {
                    links.push(value);
                }
            }
        }
        let gem = a["draft"]["gems"]["members"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|g| g["id"] == gem_id)
            .unwrap();
        gem["parameters"]["completion"] = json!({"kind":"complete"});
    }
    assert!(
        a == b,
        "all original inputs, IDs, presets, preferences, query rows and remaining obligations survive"
    );
    for field in ["draft", "policy", "tree_policy"] {
        sb[field] = sa[field].clone();
    }
    assert!(
        sa == sb,
        "only exact retired physical issue and same-preset deferred usage provenance changes"
    );
    let mut x = selected::selection(xml, &old);
    let mut y = selected::selection(xml, &new);
    selected::canonical(&mut x);
    selected::canonical(&mut y);
    assert_eq!(x, y, "original saved MAIN/CALCS request selection");
    let before = selected::finalize(
        xml,
        &old,
        &out.join(format!("prior-selected-{case:02}.json")),
    );
    let after = selected::finalize(xml, &new, &out.join(format!("selected-{case:02}.json")));
    for (report, directory) in [(&before, &old), (&after, &new)] {
        let sidecar: Value = read(directory.join("sidecar.json"));
        assert_eq!(report["draft_digest"], sidecar["draft"]);
        assert_eq!(report["finalization"]["draft_digest"], sidecar["draft"]);
    }
    let mut x = before["finalization"]["issues"].clone();
    let mut y = after["finalization"]["issues"].clone();
    selected::canonical(&mut x);
    selected::canonical(&mut y);
    let count = x.as_array().unwrap().len();
    assert_eq!(count, [119, 116, 108, 121, 19][case - 1]);
    x.as_array_mut()
        .unwrap()
        .retain(|v| !retired.contains(v["id"]["local"].as_str().unwrap()));
    assert_eq!(x, y, "all other selected obligations survive");
    assert_eq!(
        y.as_array().unwrap().len(),
        [119, 116, 108, 121, 18][case - 1]
    );
    json!({"original":case,"physical_lists_completed":retired.len(),"selected_before":count,"selected_after":y.as_array().unwrap().len(),"exact_local_ids":true,"usage_pending":true})
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}
fn header(name: &str, attrs: &[(String, String)], changes: &[(&str, Option<&str>)]) -> String {
    let mut text = format!("<{name}");
    for (key, value) in attrs {
        if !changes.iter().any(|(k, _)| *k == key) {
            text.push_str(&format!(" {key}=\"{}\"", escape(value)));
        }
    }
    for (key, value) in changes {
        if let Some(value) = value {
            text.push_str(&format!(" {key}=\"{}\"", escape(value)));
        }
    }
    text.push('>');
    text
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
        originals.push(compare(
            case,
            &fs::read(xml).unwrap(),
            &out,
            &prior_path,
            &package,
        ));
    }
    let controls = controls(&prior_path, &package, &out);
    assert_eq!(before, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":originals,"controls":controls,"queries":110,"artifacts":18,"rebuild_byte_identical":true,"prior_unchanged":true,"complete_original_builds":0,"usage_inventory":"pending"}),
    );
}
