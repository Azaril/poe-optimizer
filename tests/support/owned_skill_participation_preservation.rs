//! Exact real-import delta: two requested values on the existing supplied Skill.
//! No input inventory, selected issue, source frame or mechanical root is closed.
use crate::{family, preservation, release, selected};
use poe_optimizer_import::{
    owned_normalize::{GemInventoryPolicy, PrimarySkillUsageInput, UsageInputPolicy},
    owned_release::StagedOwnedRelease,
};
use serde_json::{Value, json};
use std::{fs, path::Path};

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn row(endpoint: &StagedOwnedRelease) -> &PrimarySkillUsageInput {
    let b: Value = family::read("bindings.json");
    let UsageInputPolicy::PobOccurrenceUsageV3 { physical, .. } =
        endpoint.normalization().usage_inputs.as_ref().unwrap();
    let rows: Vec<_> = physical
        .iter()
        .filter(|r| json!(r.gem) == b["gem"])
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0]
}
fn frame(xml: &[u8], endpoint: &StagedOwnedRelease) -> preservation::Frame {
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 {
        primary_dispositions,
        ..
    }) = &endpoint.normalization().gem_inventory
    else {
        panic!("current physical inventory")
    };
    let physical = primary_dispositions
        .iter()
        .find(|p| p.physical.gem == row(endpoint).gem)
        .unwrap();
    preservation::frame(xml, &[&physical.physical])
}
fn target(side: &Value, ordinal: usize, endpoint: &StagedOwnedRelease) -> Value {
    json!({"kind":"skill","value":{"kind":"generated","value":{
        "provider":{"root":{"kind":"skill_use","value":{"kind":"known","value":preservation::link(side,ordinal,"skill")}},"grant_path":{"members":[],"completion":{"kind":"complete"}}},
        "slot":{"kind":"known","value":row(endpoint).supply}
    }}})
}
fn expected(target: Value, group: bool, occurrence: bool) -> Value {
    let b: Value = family::read("bindings.json");
    json!({"selection":{"policy":{"kind":"known","value":b["policy"]},"target":target,
        "parameters":{"members":[
            {"slot":{"kind":"known","value":b["group"]},"value":{"kind":"known","value":{"kind":"boolean","value":group}}},
            {"slot":{"kind":"known","value":b["occurrence"]},"value":{"kind":"known","value":{"kind":"boolean","value":occurrence}}}
        ],"completion":{"kind":"complete"}}},"applicability":"required"})
}
fn bool_attribute(attributes: &[(String, String)], name: &str) -> bool {
    match attributes
        .iter()
        .find(|(n, _)| n == name)
        .unwrap()
        .1
        .as_str()
    {
        "true" => true,
        "false" => false,
        _ => panic!("strict saved Boolean"),
    }
}
fn total(draft: &Value) -> usize {
    let b: Value = family::read("bindings.json");
    draft["draft"]["skill_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|p| p["intent"]["usage"]["members"].as_array().unwrap())
        .filter(|u| u["selection"]["policy"] == json!({"kind":"known","value":b["policy"]}))
        .count()
}

pub fn originals(
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    prior_path: &Path,
    package: &Path,
    out: &Path,
) -> Vec<Value> {
    assert_eq!(prior.query_sets(), next.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    for q in prior.query_sets() {
        let name = format!("queries-{}.json", q.name.as_str());
        assert_eq!(
            fs::read(prior_path.join(&name)).unwrap(),
            fs::read(package.join(name)).unwrap()
        );
    }
    let b: Value = family::read("bindings.json");
    let mut results = vec![];
    for case in 1..=5 {
        let xml = family::root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let bytes = fs::read(&xml).unwrap();
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(prior_path, &xml, case, &old);
        let report = release::normalize(package, &xml, case, &new);
        let mut a = read(old.join("draft.json"));
        let mut z = read(new.join("draft.json"));
        let mut sa = read(old.join("sidecar.json"));
        let mut sz = read(new.join("sidecar.json"));
        preservation::authenticate(&sa, &old, prior_path, case, prior);
        preservation::authenticate(&sz, &new, package, case, next);
        assert_eq!(sa["schema_version"], 23);
        assert_eq!(sz["schema_version"], 23);
        assert_eq!(
            report["sidecar_sha256"],
            family::hash(&fs::read(new.join("sidecar.json")).unwrap())
        );
        for v in [&mut a, &mut z, &mut sa, &mut sz] {
            selected::canonical(v);
        }
        let source = frame(&bytes, prior);
        assert_eq!(total(&a), 0);
        assert_eq!(total(&z), source.locations.len());
        for loc in &source.locations {
            let preset = preservation::link(&sa, loc.set, "skill_preset");
            for ordinal in [loc.ordinal, loc.group] {
                assert_eq!(preservation::link(&sa, ordinal, "skill_preset"), preset);
                assert_eq!(preservation::link(&sz, ordinal, "skill_preset"), preset);
            }
            let wanted = target(&sa, loc.ordinal, prior);
            let old_preset = preservation::member(&a, "skill_presets", &preset);
            let count: Vec<_> = old_preset["intent"]["usage"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|u| {
                    u["selection"]["policy"] == json!({"kind":"known","value":b["count_policy"]})
                        && u["selection"]["target"] == wanted
                })
                .collect();
            assert_eq!(
                count.len(),
                1,
                "existing exact supplied-skill count binding"
            );
            assert_eq!(count[0]["applicability"], "required");
            let binding = expected(
                wanted,
                bool_attribute(&loc.group_attributes, "enabled"),
                bool_attribute(&loc.attributes, "enabled"),
            );
            let new_preset = z["draft"]["skill_presets"]["members"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|p| p["id"] == preset)
                .unwrap();
            let usage = new_preset["intent"]["usage"]["members"]
                .as_array_mut()
                .unwrap();
            let matches: Vec<_> = usage
                .iter()
                .enumerate()
                .filter(|(_, u)| **u == binding)
                .map(|(i, _)| i)
                .collect();
            assert_eq!(matches.len(), 1, "one new exact group/occurrence binding");
            usage.remove(matches[0]);
        }
        assert!(
            a == z,
            "original {case}: only exact new usage bindings may change; all IDs, roots, counts and pending inputs survive"
        );
        // All changed commitments have already been authenticated against their
        // actual release/draft. Every origin and all other evidence remain exact.
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
            sz[field] = sa[field].clone();
        }
        for row in sz["item_texts"].as_array_mut().unwrap() {
            row["attribution"]["item_lines"] = json!(prior.receipt().items);
            row["attribution"]["policy"] = json!(prior.receipt().item_source);
        }
        assert!(
            sa == sz,
            "original {case}: whole source evidence and origins must survive"
        );
        let mut x = selected::selection(&bytes, &old);
        let mut y = selected::selection(&bytes, &new);
        selected::canonical(&mut x);
        selected::canonical(&mut y);
        assert_eq!(x, y);
        let before = selected::finalize_with_definitions(
            &bytes,
            &old,
            &out.join(format!("prior-selected-{case:02}.json")),
            &prior_path.join("schema.json"),
        );
        let after = selected::finalize_with_definitions(
            &bytes,
            &new,
            &out.join(format!("selected-{case:02}.json")),
            &package.join("schema.json"),
        );
        for report in [&before, &after] {
            assert!(
                report["intent_validation"]["schema_issues"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        }
        let mut x = before["finalization"]["issues"].clone();
        let mut y = after["finalization"]["issues"].clone();
        selected::canonical(&mut x);
        selected::canonical(&mut y);
        assert_eq!(x, y);
        assert_eq!(bytes, fs::read(&xml).unwrap());
        results.push(json!({"original":case,"participation_preferences":source.locations.len(),"selected_pending_issues":y.as_array().unwrap().len(),"all_other_inputs_and_origins_unchanged":true,"sidecar_sha256":report["sidecar_sha256"]}));
    }
    results
}

fn replace_attribute(
    xml: &str,
    loc: &preservation::Location,
    group: bool,
    name: &str,
    value: Option<&str>,
) -> String {
    let (start, attributes, tag) = if group {
        (loc.group_header.start, &loc.group_attributes, "Skill")
    } else {
        (loc.range.start, &loc.attributes, "Gem")
    };
    let end = start + xml[start..].find('>').unwrap() + 1;
    let closed = xml[start..end].ends_with("/>");
    let mut attrs = attributes.clone();
    attrs.retain(|(n, _)| n != name);
    if let Some(value) = value {
        attrs.push((name.to_owned(), value.to_owned()));
    }
    let escaped = |s: &str| {
        s.replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    let header = format!(
        "<{tag} {}{}",
        attrs
            .iter()
            .map(|(n, v)| format!("{n}=\"{}\"", escaped(v)))
            .collect::<Vec<_>>()
            .join(" "),
        if closed { "/>" } else { ">" }
    );
    let mut edited = xml.to_owned();
    edited.replace_range(start..end, &header);
    edited
}

pub fn controls(next: &StagedOwnedRelease, package: &Path, out: &Path) -> Vec<Value> {
    let xml = fs::read_to_string(
        family::root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap();
    let source = frame(xml.as_bytes(), next);
    let loc = source.locations.iter().find(|l| l.ordinal == 211).unwrap();
    assert_eq!(loc.group, 210);
    assert!(loc.selected);
    let b: Value = family::read("bindings.json");
    let mut results = vec![];
    for (name, group, attribute, value, known) in [
        ("group-false", true, "enabled", Some("false"), true),
        ("occurrence-false", false, "enabled", Some("false"), true),
        ("group-missing", true, "enabled", None, false),
        ("occurrence-missing", false, "enabled", None, false),
        ("group-malformed", true, "enabled", Some("TRUE"), false),
        (
            "occurrence-malformed",
            false,
            "enabled",
            Some("TRUE"),
            false,
        ),
        ("group-unknown-field", true, "unreviewed", Some("1"), false),
        (
            "occurrence-unknown-field",
            false,
            "unreviewed",
            Some("1"),
            false,
        ),
    ] {
        let changed = replace_attribute(&xml, loc, group, attribute, value);
        let path = out.join(format!("control-{name}.xml"));
        fs::write(&path, &changed).unwrap();
        let directory = out.join(format!("control-{name}"));
        release::normalize(package, &path, 5, &directory);
        let draft = read(directory.join("draft.json"));
        let side = read(directory.join("sidecar.json"));
        preservation::authenticate(&side, &directory, package, 5, next);
        let preset = preservation::link(&side, loc.set, "skill_preset");
        let p = preservation::member(&draft, "skill_presets", &preset);
        let wanted = target(&side, loc.ordinal, next);
        let matches: Vec<_> = p["intent"]["usage"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|u| {
                u["selection"]["policy"] == json!({"kind":"known","value":b["policy"]})
                    && u["selection"]["target"] == wanted
            })
            .collect();
        if known {
            assert_eq!(matches.len(), 1);
            assert_eq!(*matches[0], expected(wanted, !group, group));
            let skill = preservation::member(
                &draft,
                "skills",
                &preservation::link(&side, loc.ordinal, "skill"),
            );
            assert_eq!(
                skill["enabled"],
                json!({"kind":"known","value":false}),
                "source disabling remains mechanical root unavailability"
            );
        } else {
            assert!(
                matches
                    .iter()
                    .all(|u| u["selection"]["parameters"]["completion"]["kind"] != "complete"),
                "unknown frame/value must not become a complete requested Boolean pair"
            );
        }
        results.push(json!({"control":name,"complete_requested_pair":known,"source_ordinal":loc.ordinal,"group_ordinal":loc.group}));
    }
    results
}
