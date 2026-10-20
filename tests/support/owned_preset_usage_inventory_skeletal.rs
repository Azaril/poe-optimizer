//! Existing physical minion reference/input receipts, with no mechanical closure.
//! Authored data remains generic. These four real source groups are test cases.
use crate::{family, positive, preservation, release, selected, source_groups};
use family::{Output, checked_output, data, read, root, sha, write};
use poe_optimizer_import::{
    owned_normalize::UsageInventoryPolicy,
    owned_release::StagedOwnedRelease,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const IDS: [&str; 4] = [
    "def.0000000000000011",
    "def.0000000000000914",
    "def.0000000000000917",
    "def.0000000000000918",
];
fn rows(value: &Value) -> &[Value] {
    value.as_array().unwrap()
}
fn digest(value: &Value) -> String {
    sha(&serde_json::to_vec(value).unwrap())
}
fn proof() -> Value {
    read(data().join("evidence.json"))["skeletal_reference_field_accounting"].clone()
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
fn row<'a>(inventory: &'a Value, f: &Value) -> &'a Value {
    let found: Vec<_> = rows(&inventory["rows"])
        .iter()
        .filter(|r| r["source"]["kind"] == "physical" && r["source"]["gem"] == f["gem"])
        .collect();
    assert_eq!(found.len(), 1);
    found[0]
}
fn row_mut<'a>(inventory: &'a mut Value, f: &Value) -> &'a mut Value {
    let found: Vec<_> = rows(&inventory["rows"])
        .iter()
        .enumerate()
        .filter(|(_, r)| r["source"]["kind"] == "physical" && r["source"]["gem"] == f["gem"])
        .map(|(i, _)| i)
        .collect();
    assert_eq!(found.len(), 1);
    &mut inventory["rows"][found[0]]
}
/// Reconstruct only the previous destination lists; every field and typed
/// consumer must already match the declared current finite shape.
fn deferred(inventory: &Value, p: &Value) -> Value {
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
    for f in rows(&p["families"]) {
        let current = row(inventory, f);
        assert_eq!(json!(rows(&current["fields"]).len()), f["field_count"]);
        let mut seen = BTreeSet::new();
        let fields = row_mut(&mut inverse, f)["fields"].as_array_mut().unwrap();
        for field in fields {
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
        for name in ["statSetIndex", "statSetIndexCalcs"] {
            assert!(
                rows(&current["fields"])
                    .iter()
                    .any(|f| f["field"] == json!({"kind":"occurrence_attribute","name":name})),
                "absence is retained, never deleted from the census"
            );
        }
    }
    assert_eq!(changed, 93);
    assert_eq!(preserved, 20);
    inverse["evidence"] = p["prior_inventory"]["evidence"].clone();
    assert_eq!(
        json!(digest(&inverse)),
        p["prior_inventory"]["canonical_sha256"],
        "whole prior inventory inverse, including all other rows and destinations"
    );
    inverse
}
pub fn check_data() {
    let p = proof();
    assert_eq!(p["kind"], "checked-physical-reference-field-destinations");
    assert_eq!(p["changed_destination_lists"], 93);
    assert_eq!(p["preserved_usage_parameter_destinations"], 20);
    assert_eq!(
        rows(&p["families"])
            .iter()
            .map(|f| f["gem"]["key"].as_str().unwrap())
            .collect::<Vec<_>>(),
        IDS
    );
    let inventory = read(data().join("inventory.json"));
    deferred(&inventory, &p);
    let census = read(data().join("source-census.json"));
    let mut all = BTreeSet::new();
    let mut selected = BTreeSet::new();
    for f in rows(&p["families"]) {
        for source in rows(&f["source_occurrences"]) {
            let case = source["case"].as_u64().unwrap() as usize;
            let ordinal = source["ordinal"].as_u64().unwrap();
            assert!(all.insert((case, ordinal)));
            let matches: Vec<_> = rows(&census[case - 1]["gem_rows"])
                .iter()
                .filter(|r| r["ordinal"] == ordinal)
                .collect();
            assert_eq!(matches.len(), 1);
            let saved = matches[0];
            assert_eq!(saved["group"], source["group"]);
            let skills: Vec<_> = rows(&saved["attributes"])
                .iter()
                .filter(|a| a["name"] == "skillId")
                .collect();
            assert_eq!(skills.len(), 1);
            assert_eq!(skills[0]["decoded"], f["skill_id"]);
            if case == 5 && saved["ordinal"] == f["selected_primary"] {
                assert!(selected.insert(f["gem"]["key"].as_str().unwrap()));
                assert_eq!(saved["group"], f["selected_group"]);
                assert_eq!(saved["selected"], true);
                assert_eq!(saved["set_id"], "4");
                assert_eq!(saved["child_elements"], json!([]));
                for name in ["statSetIndex", "statSetIndexCalcs"] {
                    assert!(!rows(&saved["attributes"]).iter().any(|a| a["name"] == name));
                }
                for (name, value) in [
                    ("count", "1"),
                    ("enableGlobal1", "true"),
                    ("enableGlobal2", "true"),
                ] {
                    let values: Vec<_> = rows(&saved["attributes"])
                        .iter()
                        .filter(|a| a["name"] == name)
                        .collect();
                    assert_eq!(values.len(), 1);
                    assert_eq!(values[0]["decoded"], value);
                }
                let companions: Vec<_> = rows(&census[4]["gem_rows"])
                    .iter()
                    .filter(|r| r["group"] == saved["group"] && r["ordinal"] != saved["ordinal"])
                    .map(|r| r["ordinal"].clone())
                    .collect();
                assert_eq!(
                    companions,
                    rows(&f["selected_companions"])
                        .iter()
                        .map(|c| c["ordinal"].clone())
                        .collect::<Vec<_>>()
                );
            }
        }
    }
    assert_eq!(all.len(), 21);
    assert_eq!(selected.len(), 4);
}
/// The independently sealed predecessor already authenticates whole artifacts.
/// These joins additionally prevent borrowing a different role/reference/recipe.
pub fn authenticate(prior: &StagedOwnedRelease) {
    let roles = serde_json::to_value(&prior.input().roles).unwrap();
    let normalization = serde_json::to_value(&prior.input().normalization).unwrap();
    for f in families() {
        let r: Vec<_> = rows(&roles["roles"])
            .iter()
            .filter(|r| r["gem"] == f["gem"])
            .collect();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0]["role"], json!({"kind":"known","value":"skill_use"}));
        assert_eq!(
            r[0]["primary"],
            json!({"kind":"known","value":f["primary"]})
        );
        assert_eq!(
            json!(digest(&r[0]["construction"])),
            f["construction_canonical_sha256"]
        );
        assert_eq!(
            r[0]["construction"]["value"]["missing_references"],
            f["declared_missing_references"]
        );
        assert_eq!(
            rows(&f["declared_missing_references"]).len(),
            2,
            "missing Command remains unresolved at both source stages"
        );
        let d: Vec<_> = rows(&normalization["gem_inventory"]["primary_dispositions"])
            .iter()
            .filter(|d| d["physical"]["gem"] == f["gem"])
            .collect();
        assert_eq!(d.len(), 1);
        let reference = &d[0]["reference_action"];
        let subset = json!({"kind":reference["kind"],"minion":reference["minion"],"actions":reference["actions"],"absent_action":reference["absent_action"]});
        assert_eq!(json!(digest(&subset)), f["reference_canonical_sha256"]);
        assert_eq!(reference["kind"], f["reference_kind"]);
        assert_eq!(reference["primary"], f["primary"]);
        assert_eq!(reference["minion"]["source_id"], f["minion_source"]);
        let u: Vec<_> = rows(&normalization["usage_inputs"]["physical"])
            .iter()
            .filter(|u| u["gem"] == f["gem"])
            .collect();
        assert_eq!(u.len(), 1);
        assert_eq!(u[0]["primary"], f["primary"]);
        assert_eq!(u[0]["skill_id"], f["skill_id"]);
        assert_eq!(u[0]["supply"], f["supply"]);
        assert_eq!(
            json!(digest(&u[0]["policies"])),
            f["usage_policies_canonical_sha256"]
        );
    }
}
pub fn mutation_controls() {
    let original = read(data().join("inventory.json"));
    let p = proof();
    for (kind, name) in [
        ("occurrence_attribute", "statSetIndex"),
        ("occurrence_attribute", "statSetIndexCalcs"),
        ("group_attribute", "groupCount"),
        ("group_attribute", "includeInFullDPS"),
    ] {
        let mut changed = original.clone();
        let f = &rows(&p["families"])[0];
        let fields = row_mut(&mut changed, f)["fields"].as_array_mut().unwrap();
        let count = fields.len();
        fields.retain(|r| !(r["field"]["kind"] == kind && r["field"]["name"] == name));
        assert_eq!(fields.len() + 1, count);
        assert!(
            std::panic::catch_unwind(|| deferred(&changed, &p)).is_err(),
            "{kind}/{name}"
        );
    }
    let mut changed = original.clone();
    changed["rows"][0]["fields"][0]["destinations"] = json!([{"kind":"input_disposition"}]);
    assert_ne!(changed, original);
    assert!(
        std::panic::catch_unwind(|| deferred(&changed, &p)).is_err(),
        "unrelated destination drift is not masked"
    );
}
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryControl {
    Deferred,
    BlockedLegacyAbsence,
    MissingGroupCount,
    BlockedReference,
}
pub fn controlled_inventory(control: InventoryControl) -> UsageInventoryPolicy {
    let original = read(data().join("inventory.json"));
    let p = proof();
    let mut value = original.clone();
    match control {
        InventoryControl::Deferred => {
            let old = deferred(&original, &p);
            for f in rows(&p["families"]) {
                *row_mut(&mut value, f) = row(&old, f).clone();
            }
        }
        _ => {
            for f in rows(&p["families"]) {
                let fields = row_mut(&mut value, f)["fields"].as_array_mut().unwrap();
                match control {
                    InventoryControl::MissingGroupCount => {
                        let (kind, name) = ("group_attribute", "groupCount");
                        let n = fields.len();
                        fields.retain(|r| {
                            !(r["field"]["kind"] == kind && r["field"]["name"] == name)
                        });
                        assert_eq!(fields.len() + 1, n);
                    }
                    InventoryControl::BlockedLegacyAbsence | InventoryControl::BlockedReference => {
                        let matched:Vec<_>=fields.iter().enumerate().filter(|(_,r)|match control {InventoryControl::BlockedLegacyAbsence=>r["field"]==json!({"kind":"occurrence_attribute","name":"statSetIndex"}),_=>r["field"]["kind"]=="reference_descendants"}).map(|(i,_)|i).collect();
                        assert_eq!(matched.len(), 1);
                        fields[matched[0]]["destinations"].as_array_mut().unwrap().push(json!({"kind":"deferred","reason":"test-only-unproved-reference-consumer"}));
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
    value["evidence"] = original["evidence"].clone();
    serde_json::from_value(value).unwrap()
}

fn frame(xml: &[u8], f: &Value) -> source_groups::Frame {
    source_groups::frame(xml, f["skill_id"].as_str().unwrap())
}
fn usage(output: &Output) -> &Value {
    positive::usage(&output.draft)
}
fn target(output: &Output, f: &Value, ordinal: usize) -> Value {
    let skill = preservation::link(&output.sidecar, ordinal, "skill");
    let gem = preservation::link(&output.sidecar, ordinal, "gem");
    assert_eq!(
        preservation::member(&output.draft, "gems", &gem)["definition"],
        json!({"kind":"known","value":f["gem"]})
    );
    json!({"kind":"skill","value":{"kind":"generated","value":{"provider":{"root":{"kind":"skill_use","value":{"kind":"known","value":skill}},"grant_path":{"members":[],"completion":{"kind":"complete"}}},"slot":{"kind":"known","value":f["supply"]}}}})
}
fn actual_positive(output: &Output, xml: &[u8], f: &Value) {
    let frame = frame(xml, f);
    let draft = &output.draft["draft"];
    assert_eq!(rows(&draft["gems"]["members"]).len(), frame.gems.len());
    assert_eq!(rows(&draft["skills"]["members"]).len(), 1);
    assert_eq!(
        rows(&draft["supports"]["members"]).len() + 1,
        frame.gems.len()
    );
    let skill = preservation::link(&output.sidecar, frame.gems[0], "skill");
    let preset = preservation::link(&output.sidecar, frame.set, "skill_preset");
    assert_eq!(draft["skill_presets"]["members"][0]["id"], preset);
    for (i, ordinal) in frame.gems.iter().copied().enumerate() {
        let gem = preservation::link(&output.sidecar, ordinal, "gem");
        let record = preservation::member(&output.draft, "gems", &gem);
        assert_eq!(record["parameters"]["completion"]["kind"], "complete");
        if i > 0 {
            assert_eq!(
                record["definition"],
                json!({"kind":"known","value":f["selected_companions"][i-1]["gem"]})
            );
            let support = preservation::link(&output.sidecar, ordinal, "support");
            let assignment = preservation::member(&output.draft, "supports", &support);
            assert_eq!(assignment["support"], json!({"kind":"known","value":gem}));
            assert_eq!(
                assignment["target"],
                json!({"kind":"authored","value":{"kind":"known","value":skill}})
            );
        }
    }
    let expected_target = target(output, f, frame.gems[0]);
    let mut observed = Vec::new();
    assert_eq!(rows(&usage(output)["members"]).len(), 3);
    for binding in rows(&usage(output)["members"]) {
        assert_eq!(binding["applicability"], "required");
        let selection = &binding["selection"];
        assert_eq!(selection["target"], expected_target);
        assert_eq!(selection["policy"]["kind"], "known");
        assert_eq!(
            selection["parameters"]["completion"],
            json!({"kind":"complete"})
        );
        for parameter in rows(&selection["parameters"]["members"]) {
            assert_eq!(parameter["slot"]["kind"], "known");
            let slot = parameter["slot"]["value"]["slot"]["key"].as_str().unwrap();
            let expected = match slot {
                "def.000000000000326b" => json!({"kind":"integer","value":1}),
                "def.000000000000332c" | "def.000000000000332d" => {
                    json!({"kind":"boolean","value":true})
                }
                "def.000000000000338f" => json!({"kind":"boolean","value":false}),
                _ => panic!("unexpected usage parameter"),
            };
            assert_eq!(parameter["value"], json!({"kind":"known","value":expected}));
            observed.push(slot);
        }
    }
    observed.sort_unstable();
    assert_eq!(
        observed,
        [
            "def.000000000000326b",
            "def.000000000000332c",
            "def.000000000000332d",
            "def.000000000000338f"
        ]
    );
}
fn attribute(
    xml: &[u8],
    f: &Value,
    group: bool,
    name: &str,
    value: Option<&str>,
) -> (Vec<u8>, Value) {
    let frame = frame(xml, f);
    let imported = positive::instance(xml);
    let e = SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let ordinal = if group { frame.group } else { frame.gems[0] };
    let row = &e.rows()[ordinal];
    let (range, bytes) = if let Some(a) = row.attribute(name) {
        let range = a.range();
        if let Some(v) = value {
            (range, v.as_bytes().to_vec())
        } else {
            let range = range.start.checked_sub(name.len() + 2).unwrap()..range.end + 1;
            assert_eq!(
                &xml[range.clone()],
                format!("{name}=\"{}\"", a.raw()).as_bytes()
            );
            (range, vec![])
        }
    } else {
        let value = value.unwrap();
        let rr = row.occurrence().range();
        let end = rr.start + xml[rr].iter().position(|b| *b == b'>').unwrap();
        let insert = if xml[end - 1] == b'/' { end - 1 } else { end };
        (insert..insert, format!(" {name}=\"{value}\"").into_bytes())
    };
    let changed = source_groups::splice(xml, range.clone(), &bytes);
    assert_eq!(self::frame(&changed, f).gems, frame.gems);
    let proof = json!({"source_ordinal":ordinal,"attribute":name,"group_attribute":group,"new":value,"original_range":[range.start,range.end],"replacement":String::from_utf8(bytes).unwrap(),"before":sha(xml),"after":sha(&changed),"other_bytes_exact":true});
    (changed, proof)
}
fn publish_control(
    prior: &StagedOwnedRelease,
    out: &Path,
    control: InventoryControl,
) -> (PathBuf, StagedOwnedRelease) {
    let staged = family::stage_skeletal_inventory_control(prior, control);
    fs::create_dir_all(out).unwrap();
    let input = out.join("endpoint.json");
    write(&input, staged.input());
    let package = out.join("package");
    assert_eq!(family::publish(&input, &package), json!(staged.receipt()));
    (package, staged)
}
struct Context<'a> {
    before_path: &'a Path,
    before: &'a StagedOwnedRelease,
    next_path: &'a Path,
    next: &'a StagedOwnedRelease,
    out: &'a Path,
}
impl Context<'_> {
    fn execute(&self, name: &str, xml: &[u8]) -> (Output, Output, PathBuf) {
        let directory = self.out.join(name);
        fs::create_dir_all(&directory).unwrap();
        let source = directory.join("source.xml");
        fs::write(&source, xml).unwrap();
        let old = directory.join("counterfactual");
        let new = directory.join("reviewed");
        let repeated = directory.join("repeated");
        release::normalize(self.before_path, &source, 5, &old);
        release::normalize(self.next_path, &source, 5, &new);
        release::normalize(self.next_path, &source, 5, &repeated);
        let before = checked_output(&old, self.before_path, 5, self.before, xml);
        let after = checked_output(&new, self.next_path, 5, self.next, xml);
        assert_eq!(
            after,
            checked_output(&repeated, self.next_path, 5, self.next, xml),
            "independent fresh normalization"
        );
        assert_eq!(fs::read(&source).unwrap(), xml);
        (before, after, directory)
    }
    fn negative(&self, name: &str, xml: &[u8], proof: &Value) -> Value {
        let (before, after, directory) = self.execute(name, xml);
        assert_eq!(usage(&before)["completion"]["kind"], "pending");
        assert_eq!(usage(&after)["completion"]["kind"], "pending");
        assert_eq!(
            before, after,
            "unproved input retains the entire draft, sidecar, diagnostics and allocator"
        );
        write(directory.join("derivation.json"), proof);
        json!({"name":name,"usage":"pending","whole_checked_output_equal":true,"fresh_repeat":true})
    }
    fn source_refusal(&self, name: &str, xml: &[u8], proof: &Value) -> Value {
        let (before, after, directory) = self.execute(name, xml);
        assert_eq!(
            before, after,
            "unknown sources gain no authority from inventory declarations"
        );
        for collection in ["gems", "skills", "supports"] {
            assert_eq!(after.draft["draft"][collection]["members"], json!([]));
        }
        assert_eq!(
            usage(&after),
            &json!({"members":[],"completion":{"kind":"complete"}})
        );
        let generated =
            &after.draft["draft"]["skill_presets"]["members"][0]["intent"]["generated_inputs"];
        assert_eq!(generated["members"], json!([]));
        assert_eq!(generated["completion"]["kind"], "pending");
        assert_eq!(
            generated["completion"]["code"],
            "generated-skill-inputs-not-converted"
        );
        write(directory.join("derivation.json"), proof);
        json!({"name":name,"physical_materialization":"none","usage":"complete_empty_no_target","generated_source_inventory":"pending","whole_checked_output_equal":true,"fresh_repeat":true})
    }
}
pub fn run_controls() {
    let prior_path =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_SKELETAL_USAGE_PRIOR").unwrap());
    let next_path =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_SKELETAL_USAGE_NEXT").unwrap());
    let out = PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_SKELETAL_USAGE_OUTPUT").unwrap());
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
        let (xml, proof) =
            source_groups::derive(&original, ordinal, f["skill_id"].as_str().unwrap());
        let actual = frame(&xml, &f);
        assert_eq!(actual.gems.len(), rows(&f["selected_companions"]).len() + 1);
        let (before, after, directory) =
            context.execute(&format!("group-{ordinal}-unchanged"), &xml);
        actual_positive(&before, &xml, &f);
        actual_positive(&after, &xml, &f);
        // A full physical reference disposition retains both the exact group
        // and primary source as owners of the unresolved usage inventory.
        let retirement =
            positive::compare_usage_retirement(&before, &after, &[actual.group, actual.gems[0]]);
        let finalized = selected::finalize_with_definitions(
            &xml,
            &directory.join("reviewed"),
            &directory.join("selection.json"),
            &next_path.join("schema.json"),
        );
        assert!(
            !rows(&finalized["finalization"]["issues"]).is_empty(),
            "usage accounting is not complete native evaluation"
        );
        write(directory.join("derivation.json"), &proof);
        results.push(json!({"name":format!("group-{ordinal}-unchanged"),"retirement":retirement,"native_execution":"not_run"}));
        for (label, group, name, value) in [
            ("main-two", true, "mainActiveSkill", Some("2")),
            ("calcs-zero", true, "mainActiveSkillCalcs", Some("0")),
            ("secondary-false", false, "enableGlobal2", Some("false")),
            ("secondary-missing", false, "enableGlobal2", None),
            (
                "wrong-minion",
                false,
                "skillMinion",
                Some("unreviewed-minion"),
            ),
            (
                "unknown-source",
                true,
                "source",
                Some("unreviewed-provider"),
            ),
        ] {
            let (changed, p) = attribute(&xml, &f, group, name, value);
            let name = format!("group-{ordinal}-{label}");
            results.push(if label == "unknown-source" {
                context.source_refusal(&name, &changed, &p)
            } else {
                context.negative(&name, &changed, &p)
            });
        }
    }
    // An accounted absence does not discharge a separately unresolved consumer.
    // The emitted count projection also requires its absent override lane.
    let f = &families()[0];
    let ordinal = f["selected_group"].as_u64().unwrap() as usize;
    let (xml, proof) = source_groups::derive(&original, ordinal, f["skill_id"].as_str().unwrap());
    for (label, control) in [
        (
            "blocked-legacy-absence-consumer",
            InventoryControl::BlockedLegacyAbsence,
        ),
        (
            "missing-group-count-lane",
            InventoryControl::MissingGroupCount,
        ),
        (
            "blocked-reference-consumer",
            InventoryControl::BlockedReference,
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
        let bytes = fs::read(root().join(entry["path"].as_str().unwrap())).unwrap();
        assert_eq!(json!(sha(&bytes)), entry["sha256"]);
        assert_eq!(json!(bytes.len()), entry["bytes"]);
    }
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"positive_original_groups":[210,205,212,223],"results":results,"source_builds_unchanged":true,"complete_original_builds":0,"native_execution":"not_run","fresh_repeats":true}),
    );
}
