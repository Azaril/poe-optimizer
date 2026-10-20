//! Two authored Direct source identities, using the current generic Import receipts.
//! The derived groups are accounting test frames, never native-valid build claims.
use crate::{family, positive, preservation, release, selected, source_groups};
use family::{Output, data, read, root, sha, write};
use poe_optimizer_import::{
    owned_normalize::UsageInventoryPolicy, owned_release::StagedOwnedRelease,
};
use serde::Serialize;
use serde_json::{Value, json};
use source_groups::Context;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const IDS: [&str; 2] = ["def.0000000000000656", "def.0000000000000657"];
fn rows(value: &Value) -> &[Value] {
    value.as_array().unwrap()
}
fn digest(value: &Value) -> String {
    sha(&serde_json::to_vec(value).unwrap())
}
fn proof() -> Value {
    read(data().join("evidence.json"))["manual_direct_field_accounting"].clone()
}
fn families() -> Vec<Value> {
    rows(&proof()["families"]).to_vec()
}
fn typed(destinations: &Value) -> Vec<Value> {
    rows(destinations)
        .iter()
        .filter(|d| d["kind"] == "usage_parameter")
        .cloned()
        .collect()
}
fn index(inventory: &Value, f: &Value) -> usize {
    let found: Vec<_> = rows(&inventory["rows"])
        .iter()
        .enumerate()
        .filter(|(_, r)| r["source"] == f["source"])
        .map(|(i, _)| i)
        .collect();
    assert_eq!(found.len(), 1);
    found[0]
}
/// Only the two reviewed destination lists are restored. This finite inverse
/// also composes with the independent skeletal inverse for its older seal;
/// the actual skeletal control keeps the current Direct rows unchanged.
pub fn restore_prior_destinations(inventory: &Value) -> Value {
    let mut inverse = inventory.clone();
    let mut changed = 0;
    let mut preserved = 0;
    assert_eq!(rows(&inventory["rows"]).len(), 25);
    assert_eq!(
        rows(&inventory["rows"])
            .iter()
            .map(|r| rows(&r["fields"]).len())
            .sum::<usize>(),
        456
    );
    for f in families() {
        let i = index(inventory, &f);
        let current = &inventory["rows"][i];
        assert_eq!(current["source"]["kind"], "occurrence");
        assert_eq!(current["source"]["target"]["kind"], "authored_direct");
        assert_eq!(current["source"]["target"]["gem"], f["gem"]);
        assert_eq!(current["source"]["target"]["skill"], f["primary"]);
        assert_eq!(json!(rows(&current["fields"]).len()), f["field_count"]);
        let mut seen = BTreeSet::new();
        for field in inverse["rows"][i]["fields"].as_array_mut().unwrap() {
            assert!(seen.insert(serde_json::to_string(&field["field"]).unwrap()));
            let kind = field["field"]["kind"].as_str().unwrap();
            let name = field["field"]["name"].as_str().unwrap_or("");
            let kept = typed(&field["destinations"]);
            preserved += kept.len();
            let count = (kind == "group_attribute" && name == "groupCount")
                || (kind == "occurrence_attribute" && name == "count");
            let reporting = kind == "group_attribute" && name == "includeInFullDPS";
            let enabled =
                name == "enabled" && matches!(kind, "group_attribute" | "occurrence_attribute");
            if count || reporting || enabled {
                assert_eq!(kept.len(), 1);
                let (policy, parameter) = if count {
                    ("def.000000000000326a", "def.000000000000326b")
                } else if reporting {
                    ("def.000000000000338e", "def.000000000000338f")
                } else if kind == "group_attribute" {
                    ("def.000000000000332b", "def.000000000000332c")
                } else {
                    ("def.000000000000332b", "def.000000000000332d")
                };
                assert_eq!(kept[0]["policy"]["key"], policy);
                assert_eq!(kept[0]["parameter"]["slot"]["key"], parameter);
                assert_eq!(
                    kept[0]["parameter"]["declaration"]["definition"],
                    kept[0]["policy"]
                );
            } else {
                assert!(kept.is_empty());
            }
            let expected = if count || reporting {
                kept.clone()
            } else {
                let mut v = vec![json!({"kind":"input_disposition"})];
                v.extend(kept.clone());
                v
            };
            assert_eq!(field["destinations"], json!(expected));
            if kind == "group_attribute" && name == "groupCount" {
                continue;
            }
            let reason = match kind {
                "group_attribute" => "group-field-consumers-not-yet-reviewed",
                "occurrence_attribute" => "source-field-consumers-not-yet-reviewed",
                "reference_descendants" => "exact-source-reference-accounting-not-yet-reviewed",
                _ => panic!("unreviewed field kind"),
            };
            let mut old = vec![json!({"kind":"deferred","reason":reason})];
            old.extend(kept);
            assert_ne!(field["destinations"], json!(old));
            field["destinations"] = json!(old);
            changed += 1;
        }
    }
    assert_eq!(changed, 50);
    assert_eq!(preserved, 10);
    inverse
}
fn deferred(inventory: &Value) -> Value {
    let p = proof();
    let mut inverse = restore_prior_destinations(inventory);
    inverse["evidence"] = p["prior_inventory"]["evidence"].clone();
    assert_eq!(
        json!(digest(&inverse)),
        p["prior_inventory"]["canonical_sha256"],
        "exact whole e8 inventory inverse; every other row and field retained"
    );
    inverse
}
pub fn check_data() {
    let p = proof();
    assert_eq!(
        p["kind"],
        "checked-authored-direct-reference-field-destinations"
    );
    assert_eq!(p["changed_destination_lists"], 50);
    assert_eq!(p["preserved_usage_parameter_destinations"], 10);
    assert_eq!(
        rows(&p["families"])
            .iter()
            .map(|f| f["gem"]["key"].as_str().unwrap())
            .collect::<Vec<_>>(),
        IDS
    );
    let inventory = read(data().join("inventory.json"));
    deferred(&inventory);
    for pin in rows(&p["source_proofs"]) {
        let b = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(json!(b.len()), pin["bytes"]);
        assert_eq!(json!(sha(&b)), pin["sha256"]);
    }
    let census = read(data().join("source-census.json"));
    let mut all = BTreeSet::new();
    let mut manual = 0;
    let mut selected = BTreeSet::new();
    for f in families() {
        let mut observed = Vec::new();
        for (case, c) in rows(&census).iter().enumerate() {
            for saved in rows(&c["gem_rows"]) {
                if !rows(&saved["attributes"])
                    .iter()
                    .any(|a| a["name"] == "skillId" && a["decoded"] == f["skill_id"])
                {
                    continue;
                }
                let source: Vec<_> = rows(&saved["group_attributes"])
                    .iter()
                    .filter(|a| a["name"] == "source")
                    .collect();
                assert!(source.len() <= 1);
                let marker = source.first().map_or(Value::Null, |a| a["decoded"].clone());
                observed.push(json!({"case":case+1,"ordinal":saved["ordinal"],"group":saved["group"],"source_marker":marker}));
                assert!(all.insert((case + 1, saved["ordinal"].as_u64().unwrap())));
                if marker.is_null() {
                    manual += 1;
                }
                if case == 4 && saved["ordinal"] == f["selected_primary"] {
                    assert!(selected.insert(f["gem"]["key"].as_str().unwrap().to_owned()));
                    assert_eq!(saved["selected"], true);
                    assert_eq!(saved["set_id"], "4");
                    assert_eq!(saved["group"], f["selected_group"]);
                    assert_eq!(marker, Value::Null);
                    assert_eq!(saved["child_elements"], json!([]));
                    for (name, value) in [
                        ("count", "1"),
                        ("enabled", "true"),
                        ("enableGlobal1", "true"),
                        ("enableGlobal2", "true"),
                    ] {
                        let a: Vec<_> = rows(&saved["attributes"])
                            .iter()
                            .filter(|a| a["name"] == name)
                            .collect();
                        assert_eq!(a.len(), 1);
                        assert_eq!(a[0]["decoded"], value);
                    }
                    let siblings: Vec<_> = rows(&c["gem_rows"])
                        .iter()
                        .filter(|s| {
                            s["group"] == saved["group"] && s["ordinal"] != saved["ordinal"]
                        })
                        .collect();
                    assert_eq!(siblings.len(), rows(&f["selected_companions"]).len());
                    for (s, expected) in siblings.into_iter().zip(rows(&f["selected_companions"])) {
                        assert_eq!(s["ordinal"], expected["ordinal"]);
                        assert!(rows(&s["attributes"]).iter().any(
                            |a| a["name"] == "skillId" && a["decoded"] == expected["skill_id"]
                        ));
                    }
                }
            }
        }
        assert_eq!(
            json!(observed),
            f["source_occurrences"],
            "full lexical census, including excluded generated source frames"
        );
    }
    assert_eq!(json!(all.len()), p["source_occurrences"]);
    assert_eq!(json!(manual), p["manual_source_occurrences"]);
    assert_eq!(selected.len(), 2);
}
/// Exact current policy and role joins. Definitions/source commitments are
/// independently authenticated by the single shared release loader and stage.
pub fn authenticate(prior: &StagedOwnedRelease) {
    let roles = serde_json::to_value(&prior.input().roles).unwrap();
    let n = serde_json::to_value(&prior.input().normalization).unwrap();
    for f in families() {
        let r: Vec<_> = rows(&roles["roles"])
            .iter()
            .filter(|r| r["gem"] == f["gem"])
            .collect();
        assert_eq!(r.len(), 1);
        assert_eq!(json!(digest(r[0])), f["role_canonical_sha256"]);
        assert_eq!(r[0]["materialization"]["kind"], "provider_only");
        assert_eq!(r[0]["role"], json!({"kind":"known","value":"skill_use"}));
        assert_eq!(r[0]["construction"]["kind"], "observed");
        let c = &r[0]["construction"]["value"];
        assert_eq!(c["resolved_effects"], f["constructed_source_effects"]);
        assert_eq!(rows(&c["resolved_effects"]).len(), 2);
        assert_eq!(c["resolved_effects"][0], c["primary"]);
        assert_eq!(c["missing_references"], json!([]));
        assert!(c["display_order"].is_null());
        let d: Vec<_> = rows(&n["direct_skill_inputs"]["dispositions"])
            .iter()
            .filter(|d| d["skill"] == f["primary"])
            .collect();
        assert_eq!(d.len(), 1);
        let mut reference = d[0]["reference_action"].clone();
        for key in ["definitions", "source", "roles", "catalog"] {
            assert!(reference.as_object_mut().unwrap().remove(key).is_some());
        }
        assert_eq!(reference["kind"], f["reference_kind"]);
        assert_eq!(reference["minion"]["source_id"], f["minion_source"]);
        let semantics = json!({"reference_action":reference,"deferred_usage":d[0]["deferred_usage"],"inert_fields":d[0]["inert_fields"],"group_guards":d[0]["group_guards"]});
        assert_eq!(
            json!(digest(&semantics)),
            f["disposition_semantics_canonical_sha256"]
        );
        let scalar: Vec<_> = rows(&n["direct_skill_inputs"]["skills"])
            .iter()
            .filter(|s| s["gem"] == f["gem"])
            .collect();
        assert_eq!(scalar.len(), 1);
        assert_eq!(
            json!(digest(scalar[0])),
            f["direct_scalar_canonical_sha256"]
        );
        let parameters: Vec<_> = rows(&scalar[0]["parameters"])
            .iter()
            .map(|p| json!({"slot":p["slot"],"field":p["value"]["tiers"][0]["selectors"][0]["name"],"unit":p["value"]["codec"]["codec"]["value"]["unit"]}))
            .collect();
        assert_eq!(json!(parameters), f["raw_parameters"]);
        let u: Vec<_> = rows(&n["usage_inputs"]["occurrences"])
            .iter()
            .filter(|u| u["target"] == f["source"]["target"])
            .collect();
        assert_eq!(u.len(), 1);
        assert_eq!(
            json!(digest(&u[0]["policies"])),
            f["usage_policies_canonical_sha256"]
        );
        for support in rows(&f["selected_companions"]) {
            let matches: Vec<_> = rows(&roles["roles"])
                .iter()
                .filter(|r| r["gem"] == support["gem"])
                .collect();
            assert_eq!(matches.len(), 1);
            assert_eq!(json!(digest(matches[0])), support["role_canonical_sha256"]);
            assert_eq!(
                matches[0]["role"],
                json!({"kind":"known","value":"support_assignment"})
            );
        }
    }
}
pub fn mutation_controls() {
    let original = read(data().join("inventory.json"));
    let f = &families()[0];
    for (kind, name) in [
        ("group_attribute", "source"),
        ("group_attribute", "mainActiveSkillCalcs"),
        ("group_attribute", "groupCount"),
        ("occurrence_attribute", "statSetIndex"),
        ("occurrence_attribute", "count"),
    ] {
        let mut changed = original.clone();
        let i = index(&changed, f);
        let fields = changed["rows"][i]["fields"].as_array_mut().unwrap();
        let n = fields.len();
        fields.retain(|r| !(r["field"]["kind"] == kind && r["field"]["name"] == name));
        assert_eq!(fields.len() + 1, n);
        assert!(
            std::panic::catch_unwind(|| deferred(&changed)).is_err(),
            "{kind}/{name}"
        );
    }
    let mut changed = original.clone();
    changed["rows"][0]["fields"][0]["destinations"] = json!([{"kind":"input_disposition"}]);
    assert_ne!(changed, original);
    assert!(std::panic::catch_unwind(|| deferred(&changed)).is_err());
    let mut changed = original.clone();
    let i = index(&changed, f);
    changed["rows"][i]["source"]["target"]["skill_id"] = json!("different-source-effect");
    assert!(std::panic::catch_unwind(|| deferred(&changed)).is_err());
}
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryControl {
    Deferred,
    BlockedReference,
    BlockedCountConsumer,
    BlockedManualSource,
}
pub fn controlled_inventory(control: InventoryControl) -> UsageInventoryPolicy {
    let original = read(data().join("inventory.json"));
    let mut value = original.clone();
    match control {
        InventoryControl::Deferred => value = deferred(&original),
        _ => {
            for f in families() {
                let i = index(&value, &f);
                let fields = value["rows"][i]["fields"].as_array_mut().unwrap();
                let mut changed = 0;
                for field in fields.iter_mut() {
                    let matched = match control {
                        InventoryControl::BlockedReference => {
                            field["field"]["kind"] == "reference_descendants"
                        }
                        InventoryControl::BlockedCountConsumer => {
                            field["field"] == json!({"kind":"occurrence_attribute","name":"count"})
                        }
                        InventoryControl::BlockedManualSource => {
                            field["field"] == json!({"kind":"group_attribute","name":"source"})
                        }
                        _ => unreachable!(),
                    };
                    if matched {
                        field["destinations"].as_array_mut().unwrap().push(json!({"kind":"deferred","reason":"test-only-independent-unproved-consumer"}));
                        changed += 1;
                    }
                }
                assert_eq!(changed, 1);
            }
        }
    }
    value["evidence"] = original["evidence"].clone();
    serde_json::from_value(value).unwrap()
}
fn actual_positive(output: &Output, xml: &[u8], f: &Value) {
    let frame = source_groups::frame(xml, f["skill_id"].as_str().unwrap());
    let draft = &output.draft["draft"];
    assert_eq!(rows(&draft["skills"]["members"]).len(), 1);
    assert_eq!(rows(&draft["gems"]["members"]).len() + 1, frame.gems.len());
    assert_eq!(
        rows(&draft["supports"]["members"]).len() + 1,
        frame.gems.len()
    );
    let skill = preservation::link(&output.sidecar, frame.gems[0], "skill");
    let record = preservation::member(&output.draft, "skills", &skill);
    assert_eq!(
        record["source"],
        json!({"kind":"direct","value":{"kind":"known","value":f["primary"]}})
    );
    assert_eq!(record["parameters"]["completion"]["kind"], "complete");
    assert_eq!(rows(&record["parameters"]["members"]).len(), 2);
    for expected in rows(&f["raw_parameters"]) {
        let matches: Vec<_> = rows(&record["parameters"]["members"])
            .iter()
            .filter(|p| p["slot"] == json!({"kind":"known","value":expected["slot"]}))
            .collect();
        assert_eq!(matches.len(), 1);
        let value = match expected["field"].as_str().unwrap() {
            "level" => 20.0,
            "quality" => 0.0,
            _ => panic!("unreviewed raw input"),
        };
        assert_eq!(
            matches[0]["value"],
            json!({"kind":"known","value":{"kind":"quantity","value":{"value":value,"unit":expected["unit"]}}})
        );
    }
    assert_eq!(record["enabled"], json!({"kind":"known","value":true}));
    let primary_origin: Vec<_> = rows(&output.sidecar["origins"])
        .iter()
        .filter(|o| o["source"]["ordinal"] == frame.gems[0])
        .collect();
    assert_eq!(primary_origin.len(), 1);
    assert!(
        !rows(&primary_origin[0]["links"])
            .iter()
            .any(|l| l["kind"] == "gem"),
        "Direct has no invented physical primary Gem"
    );
    let preset = preservation::link(&output.sidecar, frame.set, "skill_preset");
    assert_eq!(draft["skill_presets"]["members"][0]["id"], preset);
    for (ordinal, expected) in frame
        .gems
        .iter()
        .skip(1)
        .zip(rows(&f["selected_companions"]))
    {
        let gem = preservation::link(&output.sidecar, *ordinal, "gem");
        let g = preservation::member(&output.draft, "gems", &gem);
        assert_eq!(
            g["definition"],
            json!({"kind":"known","value":expected["gem"]})
        );
        assert_eq!(g["parameters"]["completion"]["kind"], "complete");
        let support = preservation::link(&output.sidecar, *ordinal, "support");
        let s = preservation::member(&output.draft, "supports", &support);
        assert_eq!(s["support"], json!({"kind":"known","value":gem}));
        assert_eq!(
            s["target"],
            json!({"kind":"authored","value":{"kind":"known","value":skill}})
        );
    }
    let target =
        json!({"kind":"skill","value":{"kind":"authored","value":{"kind":"known","value":skill}}});
    let usage = positive::usage(&output.draft);
    assert_eq!(rows(&usage["members"]).len(), 3);
    let mut slots = Vec::new();
    for binding in rows(&usage["members"]) {
        assert_eq!(binding["applicability"], "required");
        let s = &binding["selection"];
        assert_eq!(s["target"], target);
        assert_eq!(s["parameters"]["completion"], json!({"kind":"complete"}));
        for p in rows(&s["parameters"]["members"]) {
            let slot = p["slot"]["value"]["slot"]["key"].as_str().unwrap();
            let value = match slot {
                "def.000000000000326b" => json!({"kind":"integer","value":1}),
                "def.000000000000332c" | "def.000000000000332d" => {
                    json!({"kind":"boolean","value":true})
                }
                "def.000000000000338f" => json!({"kind":"boolean","value":false}),
                _ => panic!("unexpected parameter"),
            };
            assert_eq!(p["value"], json!({"kind":"known","value":value}));
            slots.push(slot);
        }
    }
    slots.sort_unstable();
    assert_eq!(
        slots,
        [
            "def.000000000000326b",
            "def.000000000000332c",
            "def.000000000000332d",
            "def.000000000000338f"
        ]
    );
}
fn publish_control(
    prior: &StagedOwnedRelease,
    out: &Path,
    control: InventoryControl,
) -> (PathBuf, StagedOwnedRelease) {
    let staged = family::stage_manual_direct_inventory_control(prior, control);
    fs::create_dir_all(out).unwrap();
    let input = out.join("endpoint.json");
    write(&input, staged.input());
    let package = out.join("package");
    assert_eq!(family::publish(&input, &package), json!(staged.receipt()));
    (package, staged)
}
pub fn run_controls() {
    let prior_path =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_MANUAL_DIRECT_USAGE_PRIOR").unwrap());
    let next_path =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_MANUAL_DIRECT_USAGE_NEXT").unwrap());
    let out =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_MANUAL_DIRECT_USAGE_OUTPUT").unwrap());
    assert!(!out.exists());
    family::checked_predecessor(&prior_path);
    family::check();
    let prior_files = release::inventory(&prior_path);
    let next_files = release::inventory(&next_path);
    let prior = release::load(&prior_path);
    let next = release::load(&next_path);
    family::checked_successor(&prior, &next);
    fs::create_dir_all(&out).unwrap();
    let (counter_path, counter) = publish_control(
        &prior,
        &out.join("deferred-inventory"),
        InventoryControl::Deferred,
    );
    let census = read(data().join("source-census.json"));
    let source = root().join(census[4]["path"].as_str().unwrap());
    let original = fs::read(&source).unwrap();
    assert_eq!(json!(sha(&original)), census[4]["sha256"]);
    assert_eq!(json!(original.len()), census[4]["bytes"]);
    let context = Context {
        before_path: &counter_path,
        before: &counter,
        next_path: &next_path,
        next: &next,
        out: &out,
    };
    let mut results = Vec::new();
    for f in families() {
        let ordinal = f["selected_group"].as_u64().unwrap() as usize;
        let skill_id = f["skill_id"].as_str().unwrap();
        let (xml, proof) = source_groups::derive(&original, ordinal, skill_id);
        let frame = source_groups::frame(&xml, skill_id);
        assert_eq!(frame.gems.len(), 4);
        let (before, after, directory) =
            context.execute(&format!("group-{ordinal}-unchanged"), &xml);
        actual_positive(&before, &xml, &f);
        actual_positive(&after, &xml, &f);
        let retirement =
            positive::compare_usage_retirement(&before, &after, &[frame.group, frame.gems[0]]);
        let finalized = selected::finalize_with_definitions(
            &xml,
            &directory.join("reviewed"),
            &directory.join("selection.json"),
            &next_path.join("schema.json"),
        );
        assert!(!rows(&finalized["finalization"]["issues"]).is_empty());
        write(directory.join("derivation.json"), &proof);
        results.push(json!({"name":format!("group-{ordinal}-unchanged"),"retirement":retirement,"native_execution":"not_run"}));
        for (label, group, name, value) in [
            ("main-two", true, "mainActiveSkill", Some("2")),
            ("calcs-zero", true, "mainActiveSkillCalcs", Some("0")),
            ("secondary-false", false, "enableGlobal2", Some("false")),
            ("empty-source", true, "source", Some("")),
            (
                "wrong-minion-calcs",
                false,
                "skillMinionCalcs",
                Some("unreviewed-minion"),
            ),
            (
                "unknown-source",
                true,
                "source",
                Some("unreviewed-provider"),
            ),
        ] {
            let (changed, p) = source_groups::attribute(&xml, skill_id, group, name, value);
            let name = format!("group-{ordinal}-{label}");
            results.push(if label == "unknown-source" {
                context.source_refusal(&name, &changed, &p)
            } else {
                context.negative(&name, &changed, &p)
            });
        }
    }
    let f = &families()[0];
    let (xml, proof) = source_groups::derive(
        &original,
        f["selected_group"].as_u64().unwrap() as usize,
        f["skill_id"].as_str().unwrap(),
    );
    for (label, control) in [
        ("blocked-reference", InventoryControl::BlockedReference),
        (
            "unaccounted-count-consumer",
            InventoryControl::BlockedCountConsumer,
        ),
        (
            "unaccounted-manual-source",
            InventoryControl::BlockedManualSource,
        ),
    ] {
        let (path, staged) =
            publish_control(&prior, &out.join(format!("package-{label}")), control);
        let c = Context {
            before_path: &counter_path,
            before: &counter,
            next_path: &path,
            next: &staged,
            out: &out,
        };
        results.push(c.negative(label, &xml, &proof));
    }
    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(prior_files, release::inventory(&prior_path));
    assert_eq!(next_files, release::inventory(&next_path));
    for entry in rows(&census) {
        let b = fs::read(root().join(entry["path"].as_str().unwrap())).unwrap();
        assert_eq!(json!(sha(&b)), entry["sha256"]);
        assert_eq!(json!(b.len()), entry["bytes"]);
    }
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"positive_original_groups":[215,228],"results":results,"source_builds_unchanged":true,"complete_original_builds":0,"native_execution":"not_run","fresh_repeats":true}),
    );
}
