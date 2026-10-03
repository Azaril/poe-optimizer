//! Manual Direct inputs retain exact occurrence ownership after reviewed closure.
#[path = "support/owned_djinn_actions.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{DirectSkillInputDisposition, DirectSkillInputPolicy},
    owned_release::StagedOwnedRelease,
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

#[test]
fn djinn_packet_preserves_direct_authority_and_partial_mechanics() {
    family::check_authored();
}

struct Location {
    family: usize,
    ordinal: usize,
    group: usize,
    set: usize,
    manual: bool,
    selected: bool,
    range: Range<usize>,
    attributes: Vec<(String, String)>,
    children: Vec<usize>,
}
struct Frame {
    locations: Vec<Location>,
    source_sets: BTreeMap<usize, usize>,
}
fn frame(xml: &[u8], families: &[Value]) -> Frame {
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([96; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let rows = evidence.rows();
    let skills = rows
        .iter()
        .find(|r| r.occurrence().name() == "Skills")
        .unwrap();
    let active = skills
        .attribute("activeSkillSet")
        .unwrap()
        .decoded()
        .unwrap();
    let mut source_sets = BTreeMap::new();
    for row in rows {
        let mut parent = Some(row.occurrence().id());
        while let Some(id) = parent {
            let p = &rows[id.ordinal() as usize];
            if p.occurrence().name() == "SkillSet" {
                source_sets.insert(
                    row.occurrence().id().ordinal() as usize,
                    id.ordinal() as usize,
                );
                break;
            }
            parent = p.occurrence().parent();
        }
    }
    let locations = rows
        .iter()
        .filter(|r| r.occurrence().name() == "Gem")
        .filter_map(|row| {
            let game = row.attribute("gemId")?.decoded().unwrap();
            let family = families.iter().position(|f| f["game_id"] == game)?;
            for (field, expected) in [
                ("variantId", "variant_id"),
                ("skillId", "skill_id"),
                ("nameSpec", "name_spec"),
            ] {
                assert_eq!(
                    row.attribute(field).unwrap().decoded().unwrap(),
                    families[family][expected]
                );
            }
            let group = &rows[row.occurrence().parent().unwrap().ordinal() as usize];
            assert_eq!(group.occurrence().name(), "Skill");
            let set = &rows[group.occurrence().parent().unwrap().ordinal() as usize];
            assert_eq!(set.occurrence().name(), "SkillSet");
            let range = row.occurrence().range();
            Some(Location {
                family,
                ordinal: row.occurrence().id().ordinal() as usize,
                group: group.occurrence().id().ordinal() as usize,
                set: set.occurrence().id().ordinal() as usize,
                manual: group
                    .attribute("source")
                    .is_none_or(|v| v.decoded().unwrap().is_empty()),
                selected: set.attribute("id").unwrap().decoded().unwrap() == active,
                children: rows
                    .iter()
                    .filter(|c| {
                        c.occurrence().id() != row.occurrence().id()
                            && c.occurrence().range().start >= range.start
                            && c.occurrence().range().end <= range.end
                    })
                    .map(|c| c.occurrence().id().ordinal() as usize)
                    .collect(),
                range,
                attributes: row
                    .attributes()
                    .iter()
                    .map(|a| (a.origin().name.clone(), a.decoded().unwrap().to_owned()))
                    .collect(),
            })
        })
        .collect();
    Frame {
        locations,
        source_sets,
    }
}
fn usage(draft: &Value, side: &Value, loc: &Location) -> (Value, Value) {
    let preset = preservation::link(side, loc.set, "skill_preset");
    let skill = preservation::link(side, loc.ordinal, "skill");
    let row = preservation::member(draft, "skill_presets", &preset);
    assert_eq!(
        row["skills"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|id| **id == skill)
            .count(),
        1
    );
    let usage = &row["usage_preferences"];
    assert_eq!(usage["completion"]["kind"], "pending");
    assert_eq!(
        usage["completion"]["code"],
        "usage-preferences-not-converted"
    );
    (preset, usage.clone())
}
fn no_gem(side: &Value, loc: &Location) {
    assert!(
        !preservation::origin(side, loc.ordinal)["links"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["kind"] == "gem")
    );
}
struct Comparison<'a> {
    prior: &'a StagedOwnedRelease,
    next: &'a StagedOwnedRelease,
    prior_path: &'a Path,
    package: &'a Path,
    out: &'a Path,
    families: &'a [Value],
}
fn compare_original(case: usize, xml: &[u8], c: &Comparison<'_>) -> Value {
    let old = c.out.join(format!("prior-original-{case:02}"));
    let new = c.out.join(format!("original-{case:02}"));
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    preservation::authenticate(&sa, &old, c.prior_path, case, c.prior);
    preservation::authenticate(&sb, &new, c.package, case, c.next);
    for side in [&sa, &sb] {
        assert_eq!(side["source_sha256"], format!("{:x}", Sha256::digest(xml)));
        assert_eq!(side["source_bytes"], xml.len());
    }
    for value in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(value);
    }
    assert_eq!(a["draft"]["allocator"], b["draft"]["allocator"]);
    assert_eq!(sa["allocator_after"], sb["allocator_after"]);
    let frame = frame(xml, c.families);
    let expected = [[1, 0, 0, 0, 5], [1, 0, 0, 0, 4]];
    for (family, counts) in expected.iter().enumerate() {
        assert_eq!(
            frame
                .locations
                .iter()
                .filter(|r| r.manual && r.family == family)
                .count(),
            counts[case - 1]
        );
    }
    let mut retired = BTreeSet::new();
    let mut owned_skills = BTreeSet::new();
    let mut generated = 0;
    for loc in &frame.locations {
        no_gem(&sa, loc);
        no_gem(&sb, loc);
        if !loc.manual {
            generated += 1;
            for side in [&sa, &sb] {
                assert!(
                    !preservation::origin(side, loc.ordinal)["links"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|x| x["kind"] == "skill")
                );
            }
            continue;
        }
        let id = preservation::link(&sa, loc.ordinal, "skill");
        assert_eq!(id, preservation::link(&sb, loc.ordinal, "skill"));
        assert!(owned_skills.insert(id["local"].as_str().unwrap().to_owned()));
        let before = preservation::member(&a, "skills", &id);
        let after = preservation::member(&b, "skills", &id);
        assert_eq!(
            before["source"],
            json!({"kind":"direct","value":{"kind":"known","value":c.families[loc.family]["skill"]}})
        );
        for field in ["id", "source", "enabled", "scope"] {
            assert_eq!(before[field], after[field]);
        }
        assert_eq!(
            before["parameters"]["members"],
            after["parameters"]["members"]
        );
        let parameters = before["parameters"]["members"].as_array().unwrap();
        assert_eq!(parameters.len(), 2);
        for (index, field) in ["raw_level", "raw_quality"].iter().enumerate() {
            assert_eq!(
                parameters[index]["slot"],
                json!({"kind":"known","value":c.families[loc.family][field]})
            );
            assert_eq!(parameters[index]["value"]["kind"], "known");
            assert_eq!(parameters[index]["value"]["value"]["kind"], "quantity");
            assert_eq!(
                parameters[index]["value"]["value"]["value"]["value"].as_f64(),
                Some([20.0, 0.0][index])
            );
        }
        let completion = &before["parameters"]["completion"];
        assert_eq!(completion["kind"], "pending");
        assert_eq!(completion["code"], "direct-skill-parameters-not-converted");
        assert_eq!(
            after["parameters"]["completion"],
            json!({"kind":"complete"})
        );
        let issue = completion["id"].clone();
        assert!(retired.insert(issue["local"].as_str().unwrap().to_owned()));
        let (preset, preferences) = usage(&a, &sa, loc);
        assert_eq!(usage(&b, &sb, loc), (preset.clone(), preferences.clone()));
        let pending = &preferences["completion"]["id"];
        let owners: Vec<_> = sa["origins"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                row["links"]
                    .as_array()
                    .unwrap()
                    .contains(&json!({"kind":"issue","value":pending}))
            })
            .collect();
        assert!(!owners.is_empty());
        for owner in owners {
            assert_eq!(
                frame
                    .source_sets
                    .get(&(owner["source"]["ordinal"].as_u64().unwrap() as usize)),
                Some(&loc.set)
            );
        }
        let links = preservation::origin_mut(&mut sa, loc.ordinal)["links"]
            .as_array_mut()
            .unwrap();
        let count = links.len();
        links.retain(|v| *v != json!({"kind":"issue","value":issue}));
        assert_eq!(count - links.len(), 1);
        for source in [loc.ordinal, loc.group] {
            let links = preservation::origin_mut(&mut sa, source)["links"]
                .as_array_mut()
                .unwrap();
            for link in [
                json!({"kind":"skill_preset","value":preset}),
                json!({"kind":"issue","value":pending}),
            ] {
                if !links.contains(&link) {
                    links.push(link);
                }
            }
        }
        for child in &loc.children {
            let links = preservation::origin_mut(&mut sa, *child)["links"]
                .as_array_mut()
                .unwrap();
            let link = json!({"kind":"skill","value":id});
            if !links.contains(&link) {
                links.push(link);
            }
        }
        let owner = a["draft"]["skills"]["members"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|s| s["id"] == id)
            .unwrap();
        owner["parameters"]["completion"] = json!({"kind":"complete"});
    }
    assert!(
        a == b,
        "all IDs, raw values, physical items/Gems, presets, usage, queries and remaining obligations survive"
    );
    for field in [
        "definitions",
        "mapping",
        "registry",
        "skill_roles",
        "reward_policy",
        "item_policy",
        "item_source_policy",
        "draft",
        "policy",
        "tree_policy",
    ] {
        sb[field] = sa[field].clone();
    }
    for row in sb["item_texts"].as_array_mut().unwrap() {
        row["attribution"]["item_lines"] = json!(c.prior.receipt().items);
        row["attribution"]["policy"] = json!(c.prior.receipt().item_source);
    }
    assert!(
        sa == sb,
        "only checked dependency identities and exact same-preset disposition links change"
    );
    let mut selected_before = selected::selection(xml, &old);
    let mut selected_after = selected::selection(xml, &new);
    selected::canonical(&mut selected_before);
    selected::canonical(&mut selected_after);
    assert_eq!(selected_before, selected_after);
    let before = selected::finalize(
        xml,
        &old,
        &c.out.join(format!("prior-selected-{case:02}.json")),
    );
    let after = selected::finalize(xml, &new, &c.out.join(format!("selected-{case:02}.json")));
    for (report, directory) in [(&before, &old), (&after, &new)] {
        let sidecar: Value = read(directory.join("sidecar.json"));
        assert_eq!(report["draft_digest"], sidecar["draft"]);
        assert_eq!(report["finalization"]["draft_digest"], sidecar["draft"]);
    }
    let mut x = before["finalization"]["issues"].clone();
    let mut y = after["finalization"]["issues"].clone();
    selected::canonical(&mut x);
    selected::canonical(&mut y);
    let before_count = x.as_array().unwrap().len();
    assert_eq!(before_count, [115, 116, 108, 121, 14][case - 1]);
    x.as_array_mut()
        .unwrap()
        .retain(|row| !retired.contains(row["id"]["local"].as_str().unwrap()));
    assert_eq!(x, y, "all other selected obligations survive");
    assert_eq!(
        y.as_array().unwrap().len(),
        [113, 116, 108, 121, 12][case - 1]
    );
    if case == 5 {
        assert_eq!(
            y.as_array()
                .unwrap()
                .iter()
                .filter(|v| v["code"] == "support-target-not-converted")
                .count(),
            6
        );
    }
    json!({"original":case,"direct_input_lists_completed":retired.len(),"selected_before":before_count,"selected_after":y.as_array().unwrap().len(),"exact_local_ids":true,"allocated_sources_unmaterialized":generated,"usage_pending":true})
}

fn installed(next: &StagedOwnedRelease) -> Vec<DirectSkillInputDisposition> {
    let Some(DirectSkillInputPolicy::PobManualDirectSkillV2 { dispositions, .. }) =
        &next.normalization().direct_skill_inputs
    else {
        panic!("checked DirectV2");
    };
    let authored = family::dispositions();
    assert_eq!(dispositions.len(), authored.len());
    for (actual, expected) in dispositions.iter().zip(authored) {
        assert_eq!(actual.skill, expected.skill);
    }
    dispositions.clone()
}
fn locator(xml: &[u8], loc: &Location, f: &Value) -> Value {
    json!({"source_sha256":format!("{:x}",Sha256::digest(xml)),"occurrence_ordinal":loc.ordinal,"catalog_gem":f["catalog_gem"],"expected_skill":f["skill"]})
}
fn target(row: &DirectSkillInputDisposition, locator: &Value, child: usize, set: usize) -> Value {
    let r = json!(row.reference_action);
    let a = &r["actions"][child];
    json!({"kind":"direct_action","value":{"actor":{"kind":"owned","value":{"provider":{"skill_use":locator,"grant_path":[]},"slot":r["minion"]["population"]}},"provider":{"skill_use":locator,"grant_path":[r["minion"]["entering_grant"],a["entering_grant"]]},"output":a["output"],"part":a["part"],"mode":a["mode"],"stat_set":a["stat_sets"][set]["stat_set"]}})
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
    assert_eq!(report["document_kind"], "owned_source_action_resolution");
    for flag in ["source_execution", "calculation", "whole_build_parity"] {
        assert_eq!(report[flag], false);
    }
    report
}
fn correspondence(
    package: &Path,
    out: &Path,
    rows: &[DirectSkillInputDisposition],
    families: &[Value],
) -> Vec<Value> {
    let mut result = vec![];
    for case in [1, 5] {
        let path = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let xml = fs::read(&path).unwrap();
        let frame = frame(&xml, families);
        for loc in frame.locations.iter().filter(|l| l.manual) {
            let adapter = out.join(format!("adapter-{}.json", loc.family));
            write(&adapter, &rows[loc.family].reference_action);
            let locator = locator(&xml, loc, &families[loc.family]);
            for (context, field) in [
                ("main", "skillMinionSkill"),
                ("calcs", "skillMinionSkillCalcs"),
            ] {
                let child = loc
                    .attributes
                    .iter()
                    .find(|(name, _)| name == field)
                    .unwrap()
                    .1
                    .parse::<usize>()
                    .unwrap()
                    - 1;
                let request = out.join(format!(
                    "reference-{case:02}-{}-{context}.json",
                    loc.ordinal
                ));
                write(&request, &json!({"skill_use":locator,"context":context}));
                let report = resolve(package, &path, &adapter, &request);
                assert_eq!(
                    report["report"]["target"],
                    target(&rows[loc.family], &locator, child, 0)
                );
                result.push(json!({"original":case,"ordinal":loc.ordinal,"context":context,"target":report["report"]["target"]}));
            }
        }
    }
    assert_eq!(result.len(), 22);
    result
}

fn skill_link(sidecar: &Value, ordinal: usize) -> Option<Value> {
    let links: Vec<_> = preservation::origin(sidecar, ordinal)["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|link| link["kind"] == "skill")
        .collect();
    assert!(links.len() <= 1);
    links.first().map(|link| link["value"].clone())
}

fn controls(c: &Comparison<'_>, rows: &[DirectSkillInputDisposition]) -> Vec<Value> {
    let path = root().join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let original = fs::read_to_string(&path).unwrap();
    let original_frame = frame(original.as_bytes(), c.families);
    let maps = |family: usize, main: usize, main_set: usize, calcs: usize| {
        let effect = c.families[family]["skill_id"].as_str().unwrap();
        format!(
            "<MinionSkillIndexLookup grantedEffect=\"{effect}\"><MinionSkillIndexMap skillIndex=\"{main}\" statSetIndex=\"{main_set}\"/></MinionSkillIndexLookup><MinionSkillIndexLookupCalcs grantedEffect=\"{effect}\"><MinionSkillIndexMap skillIndex=\"{calcs}\" statSetIndex=\"1\"/></MinionSkillIndexLookupCalcs>"
        )
    };
    let sand_maps = maps(0, 2, 2, 3);
    let water_maps = maps(1, 4, 1, 5);
    struct Control<'a> {
        name: &'a str,
        family: usize,
        attributes: Vec<(&'a str, Option<&'a str>)>,
        children: &'a str,
        admitted: bool,
        complete: bool,
        /// MAIN child/set and CALCS child/set, zero based in the checked adapter.
        alternatives: Option<[(usize, usize); 2]>,
    }
    let cases = [
        Control {
            name: "sand-edited-raw-disabled",
            family: 0,
            attributes: vec![
                ("level", Some("19.5")),
                ("quality", Some("7.25")),
                ("enabled", Some("false")),
            ],
            children: "",
            admitted: true,
            complete: true,
            alternatives: None,
        },
        Control {
            name: "water-edited-raw-disabled",
            family: 1,
            attributes: vec![
                ("level", Some("18.5")),
                ("quality", Some("6.25")),
                ("enabled", Some("false")),
            ],
            children: "",
            admitted: true,
            complete: true,
            alternatives: None,
        },
        Control {
            name: "malformed-count",
            family: 0,
            attributes: vec![("count", Some("not-a-number"))],
            children: "",
            admitted: true,
            complete: false,
            alternatives: None,
        },
        Control {
            name: "unknown-source-field",
            family: 1,
            attributes: vec![("unreviewed", Some("true"))],
            children: "",
            admitted: false,
            complete: false,
            alternatives: None,
        },
        Control {
            name: "sand-independent-action-maps",
            family: 0,
            attributes: vec![
                ("skillMinionSkill", Some("2")),
                ("skillMinionSkillCalcs", Some("3")),
            ],
            children: &sand_maps,
            admitted: true,
            complete: true,
            alternatives: Some([(1, 1), (2, 0)]),
        },
        Control {
            name: "water-independent-action-maps",
            family: 1,
            attributes: vec![
                ("skillMinionSkill", Some("4")),
                ("skillMinionSkillCalcs", Some("5")),
            ],
            children: &water_maps,
            admitted: true,
            complete: true,
            alternatives: Some([(3, 0), (4, 0)]),
        },
    ];
    let mut reports = vec![];
    for control in cases {
        let original_loc = original_frame
            .locations
            .iter()
            .find(|loc| loc.manual && loc.selected && loc.family == control.family)
            .unwrap();
        let mut xml = original.clone();
        xml.replace_range(
            original_loc.range.clone(),
            &format!(
                "{}{}</Gem>",
                preservation::header("Gem", &original_loc.attributes, &control.attributes),
                control.children
            ),
        );
        assert_ne!(xml, original);
        let source = c.out.join(format!("control-{}.xml", control.name));
        fs::write(&source, &xml).unwrap();
        let old = c.out.join(format!("prior-control-{}", control.name));
        let new = c.out.join(format!("control-{}", control.name));
        // The predecessor's strict flat census can decline nested maps. Use
        // the same edited source on both sides; never compare to original XML.
        release::normalize(c.prior_path, &source, 5, &old);
        release::normalize(c.package, &source, 5, &new);
        let mut a: Value = read(old.join("draft.json"));
        let mut b: Value = read(new.join("draft.json"));
        let mut sa: Value = read(old.join("sidecar.json"));
        let mut sb: Value = read(new.join("sidecar.json"));
        preservation::authenticate(&sa, &old, c.prior_path, 5, c.prior);
        preservation::authenticate(&sb, &new, c.package, 5, c.next);
        let source_hash = format!("{:x}", Sha256::digest(xml.as_bytes()));
        for side in [&sa, &sb] {
            assert_eq!(side["source_sha256"], source_hash);
            assert_eq!(side["source_bytes"], xml.len());
        }
        for value in [&mut a, &mut b, &mut sa, &mut sb] {
            selected::canonical(value);
        }
        let changed = frame(xml.as_bytes(), c.families);
        assert_eq!(changed.locations.len(), original_frame.locations.len());
        let mut changed_target = None;
        let mut admitted = 0;
        for loc in &changed.locations {
            no_gem(&sa, loc);
            no_gem(&sb, loc);
            let before = skill_link(&sa, loc.ordinal);
            let after = skill_link(&sb, loc.ordinal);
            if !loc.manual {
                assert!(before.is_none() && after.is_none());
                continue;
            }
            let modified = loc.selected && loc.family == control.family;
            if modified {
                assert!(changed_target.replace(loc).is_none());
            }
            if modified && !control.admitted {
                assert!(
                    after.is_none(),
                    "unknown source frame cannot create a Direct root"
                );
                continue;
            }
            admitted += 1;
            let id = after.expect("reviewed manual Direct occurrence");
            let skill = preservation::member(&b, "skills", &id);
            assert_eq!(
                skill["source"],
                json!({"kind":"direct","value":{"kind":"known","value":c.families[loc.family]["skill"]}})
            );
            let parameters = skill["parameters"]["members"].as_array().unwrap();
            assert_eq!(parameters.len(), 2);
            for (index, (attribute, field)) in [("level", "raw_level"), ("quality", "raw_quality")]
                .iter()
                .enumerate()
            {
                let expected = loc
                    .attributes
                    .iter()
                    .find(|(name, _)| name == attribute)
                    .unwrap()
                    .1
                    .parse::<f64>()
                    .unwrap();
                assert_eq!(
                    parameters[index]["slot"],
                    json!({"kind":"known","value":c.families[loc.family][field]})
                );
                assert_eq!(parameters[index]["value"]["kind"], "known");
                assert_eq!(parameters[index]["value"]["value"]["kind"], "quantity");
                assert_eq!(
                    parameters[index]["value"]["value"]["value"]["value"].as_f64(),
                    Some(expected)
                );
            }
            let complete = !modified || control.complete;
            assert_eq!(
                skill["parameters"]["completion"]["kind"],
                if complete { "complete" } else { "pending" },
                "{} ordinal {}",
                control.name,
                loc.ordinal
            );
            if !complete {
                assert_eq!(
                    skill["parameters"]["completion"]["code"],
                    "direct-skill-parameters-not-converted"
                );
            }
            let (_, preferences) = usage(&b, &sb, loc);
            if let Some(prior_id) = before {
                let prior_skill = preservation::member(&a, "skills", &prior_id);
                for field in ["source", "enabled", "scope"] {
                    assert_eq!(prior_skill[field], skill[field]);
                }
                assert_eq!(
                    prior_skill["parameters"]["members"],
                    skill["parameters"]["members"]
                );
                if control.children.is_empty() && control.admitted {
                    assert_eq!(prior_id, id);
                    assert!(
                        usage(&a, &sa, loc).1 == preferences,
                        "same edited source preserves the preset's usage obligations and values"
                    );
                }
            }
            if modified && control.name.ends_with("-edited-raw-disabled") {
                assert_eq!(skill["enabled"], json!({"kind":"known","value":false}));
            }
            if modified && control.alternatives.is_some() {
                assert_eq!(loc.children.len(), 4);
                for child in &loc.children {
                    assert_eq!(
                        preservation::origin(&sb, *child)["links"],
                        json!([{"kind":"skill","value":id}])
                    );
                }
            }
        }
        assert_eq!(admitted, 9 - usize::from(!control.admitted));
        let loc = changed_target.unwrap();
        let mut targets = vec![];
        if let Some(alternatives) = control.alternatives {
            let adapter = c.out.join(format!("control-{}-adapter.json", control.name));
            write(&adapter, &rows[control.family].reference_action);
            let locator = locator(xml.as_bytes(), loc, &c.families[loc.family]);
            for (context, (child, set)) in ["main", "calcs"].into_iter().zip(alternatives) {
                let request = c
                    .out
                    .join(format!("control-{}-{context}-request.json", control.name));
                write(&request, &json!({"skill_use":locator,"context":context}));
                let report = resolve(c.package, &source, &adapter, &request);
                let expected = target(&rows[control.family], &locator, child, set);
                assert_eq!(report["report"]["target"], expected);
                targets.push(json!({"context":context,"target":expected}));
            }
        }
        reports.push(json!({"name":control.name,"source_sha256":source_hash,"selected_direct_admitted":control.admitted,"selected_intrinsic_inputs_complete":control.complete,"same_edited_source":true,"other_raw_inputs_preserved":true,"usage_pending":true,"physical_gems_created":0,"correspondence":targets}));
    }
    assert_eq!(fs::read_to_string(path).unwrap(), original);
    reports
}

#[test]
#[ignore = "requires the exact skeletal release and authenticated complete Djinn source reports"]
fn publish_djinn_actions_preserving_five_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_DJINN_ACTIONS_PRIOR").expect("exact predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_DJINN_ACTIONS_OUTPUT").expect("new output"),
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
    let inventory = release::inventory(&package);
    assert_eq!(inventory.len(), 18);
    assert_eq!(inventory, release::inventory(&out.join("rebuilt")));
    assert_eq!(prior.query_sets(), next.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    for q in prior.query_sets() {
        let file = format!("queries-{}.json", q.name.as_str());
        assert_eq!(
            fs::read(prior_path.join(&file)).unwrap(),
            fs::read(package.join(&file)).unwrap()
        );
    }
    let b = family::bindings();
    let families = b["families"].as_array().unwrap();
    let rows = installed(&next);
    let comparison = Comparison {
        prior: &prior,
        next: &next,
        prior_path: &prior_path,
        package: &package,
        out: &out,
        families,
    };
    let mut originals = vec![];
    for case in 1..=5 {
        let path = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        release::normalize(
            &prior_path,
            &path,
            case,
            &out.join(format!("prior-original-{case:02}")),
        );
        release::normalize(
            &package,
            &path,
            case,
            &out.join(format!("original-{case:02}")),
        );
        originals.push(compare_original(
            case,
            &fs::read(path).unwrap(),
            &comparison,
        ));
    }
    let correspondence = correspondence(&package, &out, &rows, families);
    let controls = controls(&comparison, &rows);
    assert_eq!(before, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":originals,"correspondence":correspondence,"controls":controls,"queries":110,"artifacts":18,"rebuild_byte_identical":true,"prior_unchanged":true,"manual_direct_occurrences":11,"complete_original_builds":0,"usage_inventory":"pending","support_targets":"pending","native_mechanics":"partial"}),
    );
}
