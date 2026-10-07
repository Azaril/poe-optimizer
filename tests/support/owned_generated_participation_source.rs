//! Retained generated-source evidence. No native readiness or parity authority.
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_source::{SourceEvidenceLimits, SourceEvidenceRow, SourceProjectEvidence},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};

const CASES: [(usize, &str); 8] = [
    (0, "original-05"),
    (5, "generated-firebolt-focused"),
    (6, "generated-firebolt-selected-group-disabled"),
    (7, "generated-firebolt-selected-gem-disabled"),
    (8, "generated-firebolt-provider-unequipped"),
    (9, "generated-sand-focused"),
    (10, "generated-sand-selected-group-disabled"),
    (11, "generated-sand-provider-unallocated"),
];
const STAGES: [&str; 3] = ["fresh", "rebuilt_once", "rebuilt_twice"];
const EFFECTS: [&str; 3] = [
    "SummonSandDjinnPlayer",
    "CommandSandDjinnKnifeThrowPlayer",
    "FireboltPlayer",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(value: &Value) -> &[Value] {
    if let Some(v) = value.as_array() {
        v
    } else {
        assert!(value.as_object().is_some_and(|v| v.is_empty()));
        &[]
    }
}
fn pin(value: &Value) -> Vec<u8> {
    let bytes = fs::read(root().join(value["path"].as_str().unwrap())).unwrap();
    assert_eq!(bytes.len() as u64, value["bytes"]);
    assert_eq!(hash(&bytes), value["sha256"]);
    bytes
}
fn lookup<'a>(source: &'a Value, pointer: &str) -> &'a Value {
    source
        .pointer(pointer)
        .unwrap_or_else(|| panic!("missing evidence pointer {pointer}"))
}
fn relevant(value: &Value) -> bool {
    value.as_str().is_some_and(|s| EFFECTS.contains(&s))
}
struct Projection<'a> {
    source: &'a Value,
    observations: Vec<Value>,
    arrays: Vec<Value>,
}
impl<'a> Projection<'a> {
    fn observe(&mut self, p: &str) {
        self.observations
            .push(json!({"pointer":p,"value":lookup(self.source,p)}));
    }
    fn fields(&mut self, p: &str, fields: &[&str]) {
        for f in fields {
            if lookup(self.source, p).get(f).is_some() {
                self.observe(&format!("{p}/{f}"));
            }
        }
    }
    fn array(&mut self, p: &str) -> usize {
        let length = rows(lookup(self.source, p)).len();
        self.arrays.push(json!({"pointer":p,"length":length}));
        length
    }
}
fn project(source: &Value) -> (Value, Value) {
    let mut p = Projection {
        source,
        observations: vec![],
        arrays: vec![],
    };
    for (i, _) in CASES {
        let c = format!("/cases/{i}");
        p.fields(
            &c,
            &[
                "name",
                "xml_sha256",
                "changed_fields",
                "independent_fresh_repeat_equal",
                "independent_unhooked_equal",
            ],
        );
        p.observe(&format!("{c}/observation/source_hash"));
        for stage in STAGES {
            let s = format!("{c}/observation/states/{stage}");
            p.fields(
                &s,
                &[
                    "business_wrappers",
                    "calculation_hook",
                    "methods",
                    "native_participation_authority",
                    "outputs_preserved",
                    "source_methods_preserved",
                ],
            );
            p.fields(
                &format!("{s}/provenance"),
                &[
                    "exact_loader_capture",
                    "loader_hook_removed_before_calculation",
                    "granted_skills",
                ],
            );
            for lane in ["saved", "runtime"] {
                let list = format!("{s}/provenance/{lane}");
                for j in 0..p.array(&list) {
                    let row = &lookup(source, &list)[j];
                    if rows(&row["gems"]).iter().any(|g| {
                        relevant(if lane == "saved" {
                            &g["source"]["attributes"]["skillId"]
                        } else {
                            &g["state"]["fields"]["skillId"]
                        })
                    }) {
                        p.observe(&format!("{list}/{j}"));
                    }
                }
            }
            p.observe(&format!("{s}/state/selection"));
            for mode in ["MAIN", "CALCS"] {
                let m = format!("{s}/state/modes/{mode}");
                p.fields(
                    &m,
                    &[
                        "default_unarmed_without_source",
                        "main_index",
                        "node_13289_allocated",
                        "output_is_player",
                        "output_is_selected_minion",
                        "output_present",
                        "selected_group",
                        "weapon_one_item_id",
                    ],
                );
                p.fields(
                    &format!("{m}/main"),
                    &["identity", "group_index", "group_present", "source_present"],
                );
                for j in 0..p.array(&format!("{m}/active")) {
                    let a = format!("{m}/active/{j}");
                    p.fields(
                        &a,
                        &["identity", "group_index", "group_present", "source_present"],
                    );
                    if relevant(&lookup(source, &a)["identity"]["effect"]) {
                        p.fields(
                            &a,
                            &[
                                "actor_is_player",
                                "exact_source_gem_index",
                                "group",
                                "source",
                                "source_item_equipped",
                                "source_node_allocated",
                            ],
                        );
                        let minion = format!("{a}/minion");
                        if source
                            .pointer(&minion)
                            .is_some_and(|v| v.as_object().is_some_and(|o| !o.is_empty()))
                        {
                            p.fields(
                                &minion,
                                &["level", "type", "main_index", "main", "selected"],
                            );
                            for k in 0..p.array(&format!("{minion}/children")) {
                                p.fields(
                                    &format!("{minion}/children/{k}"),
                                    &[
                                        "identity",
                                        "exact_actor",
                                        "exact_summon",
                                        "index",
                                        "is_main",
                                    ],
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    (json!(p.observations), json!(p.arrays))
}
fn index(v: &Value) -> BTreeMap<&str, &Value> {
    let mut result = BTreeMap::new();
    for row in rows(v) {
        assert!(
            result
                .insert(row["pointer"].as_str().unwrap(), &row["value"])
                .is_none(),
            "duplicate evidence pointer"
        );
    }
    result
}
fn observations(v: &Value) {
    assert_eq!(rows(&v["observations"]).len(), 7086);
    assert_eq!(rows(&v["array_inventories"]).len(), 186);
    let o = index(&v["observations"]);
    let mut lengths = BTreeMap::new();
    for row in rows(&v["array_inventories"]) {
        assert!(
            lengths
                .insert(
                    row["pointer"].as_str().unwrap(),
                    row["length"].as_u64().unwrap()
                )
                .is_none()
        );
    }
    let get = |p: &str| *o.get(p).unwrap_or_else(|| panic!("unprojected {p}"));
    for (i, name) in CASES {
        let c = format!("/cases/{i}");
        assert_eq!(get(&format!("{c}/name")), name);
        for field in [
            "independent_fresh_repeat_equal",
            "independent_unhooked_equal",
        ] {
            assert_eq!(get(&format!("{c}/{field}")), true);
        }
        for stage in STAGES {
            let s = format!("{c}/observation/states/{stage}");
            for field in [
                "business_wrappers",
                "calculation_hook",
                "native_participation_authority",
            ] {
                assert_eq!(get(&format!("{s}/{field}")), false);
            }
            for field in [
                "outputs_preserved",
                "source_methods_preserved",
                "provenance/exact_loader_capture",
                "provenance/loader_hook_removed_before_calculation",
            ] {
                assert_eq!(get(&format!("{s}/{field}")), true);
            }
            for mode in ["MAIN", "CALCS"] {
                let m = format!("{s}/state/modes/{mode}");
                let expected = match i {
                    0 if mode == "CALCS" => "SummonSkeletalArsonistsPlayer",
                    0 | 11 => "SummonSkeletalSnipersPlayer",
                    5 | 6 => "FireboltPlayer",
                    7 => "MeleeUnarmedPlayer",
                    8 => "IceNovaPlayer",
                    9 | 10 => "SummonSandDjinnPlayer",
                    _ => unreachable!(),
                };
                assert_eq!(get(&format!("{m}/main/identity"))["effect"], expected);
                assert_eq!(get(&format!("{m}/default_unarmed_without_source")), i == 7);
                assert_eq!(get(&format!("{m}/node_13289_allocated")), i != 11);
                let mut counts = [0, 0, 0];
                for j in 0..lengths[format!("{m}/active").as_str()] {
                    let a = format!("{m}/active/{j}");
                    let identity = get(&format!("{a}/identity"));
                    if !relevant(&identity["effect"]) {
                        continue;
                    }
                    let group = get(&format!("{a}/group"));
                    let source = get(&format!("{a}/source"));
                    assert_eq!(source["enabled"], true);
                    assert_eq!(source["quality"], 0);
                    assert_eq!(identity["quality"], 0);
                    if identity["effect"] == "FireboltPlayer" {
                        counts[0] += 1;
                        assert_eq!(group["source"], "Item:28:New Item, Ashen Staff");
                        assert_eq!(group["enabled"], i != 6);
                        assert_eq!(source["level"], 11);
                        assert_eq!(identity["level"], 11);
                    } else if group["source"] == "Tree:13289" {
                        counts[1] += 1;
                        assert_eq!(group["enabled"], i != 10);
                        assert_eq!(source["level"], 1);
                        assert_eq!(identity["level"], 3);
                    } else {
                        counts[2] += 1;
                        assert!(group.get("source").is_none());
                        assert_eq!(group["enabled"], true);
                        assert_eq!(source["level"], 20);
                        assert_eq!(identity["level"], 22);
                    }
                }
                assert_eq!(
                    counts,
                    [
                        usize::from(![7, 8].contains(&i)),
                        if i == 11 { 0 } else { 2 },
                        2
                    ]
                );
            }
        }
    }
}
fn attributes(row: &SourceEvidenceRow<'_>) -> Value {
    Value::Object(
        row.attributes()
            .iter()
            .map(|a| (a.origin().name.clone(), json!(a.decoded().unwrap())))
            .collect(),
    )
}
fn census(v: &Value, full: bool) {
    assert_eq!(rows(&v["original_inventory"]).len(), 5);
    assert_eq!(rows(&v["baseline_imports"]).len(), 3);
    let mut counts = [0, 0];
    for original in 1..=5 {
        let saved = &v["original_inventory"][original - 1];
        assert_eq!(saved["original"], original);
        let fixture = format!("tests/fixtures/builds/breadth-20260908/build-{original:02}.xml");
        assert_eq!(saved["fixture"], fixture);
        assert_eq!(
            saved["source_ordinal_definition"],
            "zero_based_xml_element_preorder_including_root"
        );
        let bytes = fs::read(root().join(&fixture)).unwrap();
        let source = ImportedBuildInstance::from_decoded(
            decode_build(&bytes).unwrap(),
            BuildLineage::from_bytes([71; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
        let xml = evidence.rows();
        let skills = xml
            .iter()
            .find(|r| r.occurrence().name() == "Skills")
            .unwrap();
        assert_eq!(saved["skills_attributes"], attributes(skills));
        let candidates: Vec<_> = xml
            .iter()
            .filter(|r| {
                r.occurrence().name() == "Gem"
                    && r.attribute("skillId")
                        .is_some_and(|a| [EFFECTS[0], EFFECTS[2]].contains(&a.decoded().unwrap()))
            })
            .collect();
        assert_eq!(candidates.len(), rows(&saved["occurrences"]).len());
        for (gem, row) in candidates.into_iter().zip(rows(&saved["occurrences"])) {
            let group = &xml[gem.occurrence().parent().unwrap().ordinal() as usize];
            let set = &xml[group.occurrence().parent().unwrap().ordinal() as usize];
            assert_eq!(group.occurrence().name(), "Skill");
            assert_eq!(set.occurrence().name(), "SkillSet");
            for (key, node) in [("skill_set", set), ("group", group), ("gem", gem)] {
                assert_eq!(
                    row[format!("{key}_ordinal")],
                    node.occurrence().id().ordinal()
                );
                assert_eq!(row[format!("{key}_attributes")], attributes(node));
            }
            for (key, node) in [("group_index", group), ("gem_index", gem)] {
                let index = xml
                    .iter()
                    .filter(|x| {
                        x.occurrence().parent() == node.occurrence().parent()
                            && x.occurrence().name() == node.occurrence().name()
                    })
                    .position(|x| x.occurrence().id() == node.occurrence().id())
                    .unwrap()
                    + 1;
                assert_eq!(row[key], index);
            }
            assert_eq!(
                row["skill_set"],
                set.attribute("id").unwrap().decoded().unwrap()
            );
            let generated = group
                .attribute("source")
                .is_some_and(|a| !a.decoded().unwrap().is_empty());
            counts[usize::from(!generated)] += 1;
            assert_eq!(row["kind"], if generated { "generated" } else { "manual" });
            let status = if !generated {
                "authored_direct"
            } else if row["skill_set"] != saved["skills_attributes"]["activeSkillSet"] {
                "archived_source_axes_unresolved"
            } else if original == 4 {
                "item_provider_unresolved"
            } else {
                "exact_generated_provider_resolved"
            };
            assert_eq!(row["normalization_status"], status);
        }
    }
    assert_eq!(counts, [9, 6]);
    for (entry, original) in rows(&v["baseline_imports"]).iter().zip([1, 4, 5]) {
        assert_eq!(entry["original"], original);
        for key in ["draft", "sidecar"] {
            assert_eq!(
                entry[key]["path"],
                format!("runs/owned-global-energy-shield-03/original-{original:02}/{key}.json")
            );
            assert_eq!(entry[key]["sha256"].as_str().unwrap().len(), 64);
            assert!(entry[key]["bytes"].as_u64().unwrap() > 0);
        }
        let imported = index(&entry["observations"]);
        for row in rows(&v["original_inventory"][original - 1]["occurrences"]) {
            if row["kind"] != "generated" {
                continue;
            }
            let ids: Vec<_> = imported
                .iter()
                .filter(|(p, value)| {
                    p.ends_with("/id") && value["local"] == row["skill_preset_local"]
                })
                .collect();
            assert_eq!(ids.len(), 1);
            let base = ids[0].0.strip_suffix("/id").unwrap();
            let inputs = imported[format!("{base}/intent/generated_inputs").as_str()];
            let skill = if row["gem_attributes"]["skillId"] == EFFECTS[0] {
                "def.0000000000000322"
            } else {
                "def.0000000000000134"
            };
            let has = rows(&inputs["members"]).iter().any(|member| {
                rows(&member["parameters"]["members"])
                    .iter()
                    .any(|p| p["slot"]["value"]["declaration"]["definition"]["key"] == skill)
            });
            assert_eq!(
                has,
                row["normalization_status"] == "exact_generated_provider_resolved"
            );
        }
        if full {
            let draft: Value = serde_json::from_slice(&pin(&entry["draft"])).unwrap();
            pin(&entry["sidecar"]);
            for (p, value) in imported {
                assert_eq!(lookup(&draft, p), value, "baseline {p}");
            }
        }
    }
}
/// `false` is hermetic; `true` additionally authenticates retained reports/imports.
pub fn verify_source(full: bool) {
    let v: Value = serde_json::from_slice(
        &fs::read(
            root().join("data/owned/poe2/3887ae68/generated-participation/source-vectors.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(v["schema_version"], 1);
    assert_eq!(
        v["scope"],
        json!({"source_evidence_only":true,"native_participation_authority":false,"native_build_parity":false,"native_defaults_authorized":false,"required_input_inventory_complete":false,"source_preview_override_imported":false,"new_source_execution":false})
    );
    assert_eq!(v["case_indexes"], json!(CASES.map(|(i, _)| i)));
    let tracked: BTreeMap<_, _> = rows(&v["tracked_files"])
        .iter()
        .map(|p| (p["path"].as_str().unwrap(), p))
        .collect();
    assert_eq!(tracked.len(), 11);
    assert_eq!(tracked.len(), rows(&v["tracked_files"]).len());
    let mut required = (1..=5)
        .map(|n| format!("tests/fixtures/builds/breadth-20260908/build-{n:02}.xml"))
        .collect::<Vec<_>>();
    required.extend(
        [
            "crates/poe-optimizer-pob/data/pob-source-manifest.json",
            "crates/poe-optimizer-pob/tests/owned_selected_participation_source.rs",
            "crates/poe-optimizer-pob/tests/support/selected_participation_source.lua",
            "crates/poe-optimizer-pob/tests/support/sniper_actor_action_source.lua",
            "crates/poe-optimizer-pob/tests/support/djinn_provider_source.lua",
            "crates/poe-optimizer-pob/tests/support/authored_skill_membership_source.lua",
        ]
        .map(str::to_owned),
    );
    required.sort();
    assert_eq!(
        tracked.keys().copied().collect::<Vec<_>>(),
        required.iter().map(String::as_str).collect::<Vec<_>>()
    );
    for p in tracked.values() {
        pin(p);
    }
    let m = &v["metadata"];
    assert_eq!(
        m["source_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    assert_eq!(m["stages"], json!(STAGES));
    assert_eq!(m["fresh_runtimes_per_case"], 3);
    assert_eq!(m["no_retry_or_settling"], true);
    for field in [
        "native_build_parity",
        "native_participation_authority",
        "source_bug_exception",
    ] {
        assert_eq!(m[field], false);
    }
    let manifest_path = "crates/poe-optimizer-pob/data/pob-source-manifest.json";
    let manifest: Value = serde_json::from_slice(&pin(tracked[manifest_path])).unwrap();
    assert_eq!(m["manifest_sha256"], tracked[manifest_path]["sha256"]);
    assert_eq!(m["source_revision"], manifest["upstream_revision"]);
    for f in rows(&m["files"]) {
        assert_eq!(
            rows(&manifest["files"])
                .iter()
                .filter(|x| x["path"] == f["path"] && x["sha256"] == f["sha256"])
                .count(),
            1
        );
    }
    for (field, file) in [
        ("observer_sha256", "selected_participation_source.lua"),
        ("loader_sha256", "sniper_actor_action_source.lua"),
        ("authentication_sha256", "djinn_provider_source.lua"),
        ("membership_sha256", "authored_skill_membership_source.lua"),
    ] {
        assert_eq!(
            m[field],
            tracked[format!("crates/poe-optimizer-pob/tests/support/{file}").as_str()]["sha256"]
        );
    }
    assert_eq!(
        m["original_xml_sha256"],
        tracked["tests/fixtures/builds/breadth-20260908/build-05.xml"]["sha256"]
    );
    assert_eq!(rows(&v["reports"]).len(), 2);
    for (p, mode) in rows(&v["reports"]).iter().zip(["off", "on"]) {
        assert_eq!(
            p["path"],
            format!("runs/owned-selected-participation-source-01/source-jit-{mode}.json")
        );
        assert_eq!(p["bytes"], 6768645);
        assert_eq!(
            p["sha256"],
            "d2aa60cae2de446dad0c51ffeb65d450cca5b199f3b87d33a6c94afa5f37a6e8"
        );
    }
    observations(&v);
    census(&v, full);
    if full {
        let mut first = None;
        for row in rows(&v["reports"]) {
            let bytes = pin(row);
            if let Some(before) = &first {
                assert_eq!(&bytes, before, "JIT report bytes differ");
            } else {
                first = Some(bytes.clone());
            }
            let report: Value = serde_json::from_slice(&bytes).unwrap();
            let mut metadata = report.clone();
            metadata.as_object_mut().unwrap().remove("cases");
            assert_eq!(metadata, *m);
            for o in rows(&v["observations"]) {
                assert_eq!(lookup(&report, o["pointer"].as_str().unwrap()), &o["value"]);
            }
            for a in rows(&v["array_inventories"]) {
                assert_eq!(
                    rows(lookup(&report, a["pointer"].as_str().unwrap())).len() as u64,
                    a["length"]
                );
            }
            let (observations, arrays) = project(&report);
            assert_eq!(
                observations, v["observations"],
                "exact projected pointer/value inventory"
            );
            assert_eq!(
                arrays, v["array_inventories"],
                "exact projected array inventory"
            );
        }
    }
}
