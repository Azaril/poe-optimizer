//! Retire one source-metadata gap; preserve every program and other obligation.
//! Ordinary checks use maintained bytes only. Acquisition reports are an explicit
//! optional offline check and are never runtime or ordinary-CI dependencies.
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_rules::DefinitionRules,
};
use poe_optimizer_import::{
    owned_item_lines::ItemLineRule,
    owned_modifier_value_recipe::{ModifierValuePolicy, compile_owned_modifier_values},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

pub const KIND: &str = "source-bound-charm-component-scalability";
pub const RETIRED: &str = "numeric-component-scalability-unproved";
const CERTIFICATE: &str = "scalability-certificate.json";
const CERTIFICATE_SHA: &str = "cd89d5351e412a38e286225f0de15a4b536dadeb2d2ea7629d8af95ffeda89da";
const MODIFIER: &str = "def.0000000000003172";
const PRIOR: &str = "e85a165738460fa2bc37f3eda92a0732c26771192e4faf999827d8f29864a9f6";
const REMAINING: [&str; 3] = [
    "source-encoding-and-corrupted-range-unproved",
    "remaining-ordered-magnitude-transforms-unconverted",
    "canonical-input-admission-unproved",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/fine-belt-modifiers")
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn pinned(pin: &Value) -> Vec<u8> {
    let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
    assert_eq!(json!(bytes.len()), pin["bytes"]);
    assert_eq!(json!(sha(&bytes)), pin["sha256"]);
    bytes
}
pub fn certificate() -> Value {
    // Retain the prepared certificate byte-for-byte. Its historical kind/status
    // and pending_runtime_checks are acquisition-time metadata, never execution
    // success. Only this proof's digest enters release provenance.
    let bytes = fs::read(data().join(CERTIFICATE)).unwrap();
    assert_eq!(bytes.len(), 262_560);
    assert_eq!(sha(&bytes), CERTIFICATE_SHA);
    serde_json::from_slice(&bytes).unwrap()
}
fn digest(c: &Value) -> OwnedContentDigest {
    digest_owned("owned-charm-component-scalability-v1", c, 1024 * 1024).unwrap()
}
pub fn expected_owner(after: bool) -> DefinitionRules {
    let c = certificate();
    serde_json::from_value(
        c["dependencies"][if after { "owner_after" } else { "owner_before" }].clone(),
    )
    .unwrap()
}

/// Exact certificate equality precedes the inverse: no numerical, source-owner,
/// inventory-order or unrelated-gap mutation can hide behind the gap count.
pub fn check_owner_delta(before: &Value, after: &Value) {
    let c = certificate();
    assert_eq!(*before, c["dependencies"]["owner_before"]);
    assert_eq!(*after, c["dependencies"]["owner_after"]);
    assert_eq!(before["owner"], after["owner"]);
    assert_eq!(before["owner"]["value"]["value"]["key"], MODIFIER);
    assert_eq!(before["programs"]["closure"]["kind"], "partial");
    assert_eq!(after["programs"]["closure"]["kind"], "partial");
    let a = before["programs"]["closure"]["value"]["gaps"]
        .as_array()
        .unwrap();
    let b = after["programs"]["closure"]["value"]["gaps"]
        .as_array()
        .unwrap();
    assert_eq!((a.len(), b.len()), (4, 3));
    assert_eq!(
        a.iter()
            .map(|g| g["code"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [REMAINING[0], RETIRED, REMAINING[1], REMAINING[2]]
    );
    assert_eq!(
        b.iter()
            .map(|g| g["code"].as_str().unwrap())
            .collect::<Vec<_>>(),
        REMAINING
    );
    for gap in a {
        assert_eq!(gap["subject"], before["owner"]);
        assert_eq!(gap["facet"], "game_rules");
    }
    assert_eq!(
        b,
        &a.iter()
            .filter(|g| g["code"] != RETIRED)
            .cloned()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        before["programs"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "catalyst-scalar",
            "ordered-magnitude-scalar",
            "corrupted-base-factor",
            "effective-amount",
            "local-charm-capacity-contribution"
        ]
    );
    let mut inverse = after.clone();
    inverse["programs"]["closure"] = before["programs"]["closure"].clone();
    assert_eq!(
        &inverse, before,
        "whole owner inverse, not a program subset"
    );
    assert_eq!(c["coverage"]["owner"], before["owner"]);
    assert_eq!(c["coverage"]["before"], before["programs"]["closure"]);
    assert_eq!(c["coverage"]["after"], after["programs"]["closure"]);
    assert_eq!(
        Some(&c["coverage"]["retired"]),
        a.iter().find(|g| g["code"] == RETIRED)
    );
}

/// Scan all current emitters, including unselected rules. A second emitter,
/// changed input projection or changed guard requires a new reviewed proof.
pub fn check_admission(rules: &Value, conditions: &Value) {
    let c = certificate();
    let d = &c["dependencies"];
    let emitters: Vec<_> = rules
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| {
            r["emissions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["kind"] == "modifier" && e["value"]["definition"]["key"] == MODIFIER)
        })
        .collect();
    assert_eq!(emitters.len(), 1);
    // The prepared JSON uses 1 while maintained quantity literals use 1.0.
    // Compare the complete strict existing type, preserving every field and
    // ordered element; do not erase fields or normalize arbitrary JSON numbers.
    let actual: ItemLineRule = serde_json::from_value(emitters[0].clone()).unwrap();
    let expected: ItemLineRule = serde_json::from_value(d["item_rule"].clone()).unwrap();
    assert_eq!(actual, expected);
    let guards: Vec<_> = conditions
        .as_array()
        .unwrap()
        .iter()
        .filter(|g| g["rule"] == "ranged-charm-slots")
        .collect();
    assert_eq!(guards, [&d["source_condition"]]);
    assert_eq!(
        d["source_condition"],
        json!({"rule":"ranged-charm-slots","all":[
            {"kind":"initial_scaling_is_one"},{"kind":"no_generated_buff_members"},
            {"kind":"unsigned_integer_capture","value":{"capture":"lower","min":1,"max":1000000}},
            {"kind":"unsigned_integer_capture","value":{"capture":"upper","min":1,"max":1000000}}
        ]})
    );
    let rule = &d["item_rule"];
    assert_eq!(rule["id"], "ranged-charm-slots");
    assert_eq!(rule["emissions"].as_array().unwrap().len(), 1);
    assert_eq!(rule["captures"].as_array().unwrap().len(), 2);
    assert_eq!(
        rule["pattern"],
        json!([
            {"kind":"literal","value":"Has ("},
            {"kind":"numeric_capture","value":{"capture":"lower","syntax":"integer","sign":"optional_minus"}},
            {"kind":"literal","value":"-"},
            {"kind":"numeric_capture","value":{"capture":"upper","syntax":"integer","sign":"optional_minus"}},
            {"kind":"literal","value":") Charm Slot"}
        ])
    );
    let rolls = rule["emissions"][0]["value"]["rolls"].as_array().unwrap();
    assert_eq!(rolls.len(), 25);
    let unique: BTreeSet<_> = rolls
        .iter()
        .map(|r| r["slot"]["slot"]["key"].as_str().unwrap())
        .collect();
    assert_eq!(unique.len(), 25);
    let roll = |key: &str| {
        rolls
            .iter()
            .find(|r| r["slot"]["slot"]["key"] == key)
            .unwrap()
    };
    assert_eq!(
        roll("def.0000000000003188")["value"],
        json!({"kind":"literal","value":{"kind":"boolean","value":false}})
    );
    assert_eq!(
        roll("def.0000000000003189")["value"],
        json!({"kind":"literal","value":{
        "kind":"quantity","value":{"value":1,"unit":{"kind":"unit",
        "namespace":{"game":"poe2","version":"owned-mechanics-v1"},"key":"def.0000000000000001"}}}})
    );
    assert_eq!(
        roll("def.0000000000003173")["value"]["kind"],
        "numeric_projection"
    );
}

/// Maintained proof only: this function never reads an ignored runs path.
pub fn check_authored() {
    let c = certificate();
    assert_eq!(c["schema_version"], 1);
    assert_eq!(c["predecessor"]["receipt"]["input"], PRIOR);
    assert_eq!(c["predecessor"]["receipt"]["query_rows"], 110);
    assert_eq!(
        c["predecessor"]["receipt"]["artifacts"]
            .as_array()
            .unwrap()
            .len(),
        17
    );
    assert_eq!(
        c["scope"],
        json!({"retired_obligations":1,"retained_obligations":3,
        "closed_rule_owners":0,"new_definitions":0,"new_programs":0,"numerical_program_changes":0,
        "import_guard_changes":0,"canonical_input_admission_proven":false,
        "source_lifecycle_parity_proven":false,"broad_non_neutral_domain_restricted":false,
        "whole_build_parity":false})
    );
    let d = &c["dependencies"];
    check_owner_delta(&d["owner_before"], &d["owner_after"]);
    let pins = d["source_files"].as_array().unwrap();
    assert_eq!(pins.len(), 3);
    let actual: Vec<Value> = pins
        .iter()
        .map(|p| serde_json::from_slice(&pinned(p)).unwrap())
        .collect();
    assert_eq!(
        pins.iter()
            .map(|p| p["path"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "data/owned/poe2/3887ae68/fine-belt-modifiers/item-rules.json",
            "data/owned/poe2/3887ae68/fine-belt-modifiers/source-conditions.json",
            "data/owned/poe2/3887ae68/fine-belt-modifiers/numeric-bindings.json",
        ]
    );
    check_admission(&actual[0], &actual[1]);
    let numeric: Vec<_> = actual[2]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["modifier"]["key"] == MODIFIER)
        .collect();
    assert_eq!(numeric, [&d["numeric_binding"]]);
    let binding = &d["numeric_binding"];
    assert_eq!(binding["scaling"]["kind"], "scaled");
    assert_eq!(binding["precision"], 1);
    assert_eq!(binding["display_precision"], 0);
    assert_eq!(binding["sign"], json!({"kind":"direct"}));

    let s = &c["source"];
    let manifest: Value = serde_json::from_slice(&pinned(&s["manifest"])).unwrap();
    assert_eq!(
        s["manifest"]["sha256"],
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    assert_eq!(manifest["upstream_revision"], s["revision"]);
    assert_eq!(s["revision"], "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4");
    let files = s["files"].as_array().unwrap();
    assert_eq!(files.len(), 8);
    let mut paths = BTreeSet::new();
    for pin in files {
        assert!(paths.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| *p == pin)
                .count(),
            1
        );
    }
    let excerpts = s["excerpts"].as_array().unwrap();
    assert_eq!(excerpts.len(), 2);
    assert_eq!(
        excerpts[0],
        json!({"path":"src/Data/ModScalability.lua","first":10051,"last":10052,
        "lines":["\t[\"Has # Charm Slot\"] = { { isScalable = true } },",
                 "\t[\"Has # Charm Slots\"] = { { isScalable = true } },"]})
    );
    assert_eq!(excerpts[1]["path"], "src/Modules/ItemTools.lua");
    assert_eq!(excerpts[1]["first"], 275);
    assert_eq!(excerpts[1]["last"], 278);
    assert_eq!(excerpts[1]["lines"].as_array().unwrap().len(), 4);
    assert_eq!(s["singular_emitter_only"], true);
    assert_eq!(
        s["plural_metadata_corroborating_not_an_import_admission_claim"],
        true
    );
    let reports = s["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    for (pin, mode) in reports.iter().zip(["off", "on"]) {
        assert_eq!(
            pin["path"],
            format!("runs/owned-fine-belt-source-01/source-jit-{mode}.json")
        );
        assert_eq!(pin["bytes"], 1_413_520);
        assert_eq!(
            pin["sha256"],
            "0598dfdcb6b58a7f479ba6af6e04a7e207edfdd333c6a2a1bcf4d5addf2b3d70"
        );
    }
    let cases = s["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 6);
    for (case, (name, capacity)) in cases.iter().zip([
        ("original", 2),
        ("header-high", 2),
        ("header-absent", 2),
        ("legacy-range-zero", 1),
        ("legacy-range-one", 3),
        ("quality-twenty", 2),
    ]) {
        assert_eq!(case["name"], name);
        assert_eq!(case["scalability"], json!([{"isScalable":true}]));
        assert_eq!(case["loaded_charm_capacity"], capacity);
    }
    let xml = pinned(&s["original_xml"]);
    assert_eq!(
        s["original_xml"]["path"],
        "tests/fixtures/builds/breadth-20260908/build-05.xml"
    );
    assert_eq!(s["cases"][0]["xml_sha256"], sha(&xml));
    assert_eq!(s["controls"].as_array().unwrap().len(), 3);
    for (control, name) in s["controls"].as_array().unwrap().iter().zip([
        "implicit-magnitude",
        "charm-tag-magnitude",
        "charm-corrupted-range",
    ]) {
        assert_eq!(control["name"], name);
        assert_eq!(control["capacity"], 3);
    }
}

fn dependencies(endpoint: &StagedOwnedRelease, after: bool) {
    let c = certificate();
    let owner = expected_owner(after);
    let owners: Vec<_> = endpoint
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .filter(|o| o.owner == owner.owner)
        .collect();
    assert_eq!(owners, [&owner]);
    let items = json!(endpoint.input().items);
    let source = json!(endpoint.input().item_source);
    let dialect = source["dialect"].as_object().unwrap();
    assert_eq!(dialect.len(), 1);
    check_admission(
        &items["rules"],
        &dialect.values().next().unwrap()["single_modifier_conditions"],
    );
    let factor_type = owner
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == "effective-amount")
        .unwrap()
        .reads[1]
        .value_type
        .clone();
    let policy = ModifierValuePolicy {
        schema_version: 1,
        version: OwnedDefinitionKey::new("charm-scalability-authentication").unwrap(),
        definitions: endpoint.receipt().definitions.clone(),
        factor_unit: serde_json::from_value(json!(factor_type)["value"]["unit"].clone()).unwrap(),
        bindings: vec![
            serde_json::from_value(c["dependencies"]["numeric_binding"].clone()).unwrap(),
        ],
    };
    // Use the real compiler's idempotent writer check, with unchanged limits and
    // current schema. Never copy or simplify its expression law here.
    let compiled =
        compile_owned_modifier_values(endpoint.assembled(), &policy, Default::default()).unwrap();
    assert_eq!(compiled.extension.owners.len(), 1);
    let generated = &compiled.extension.owners[0];
    assert_eq!(generated.owner, owner.owner);
    assert_eq!(generated.programs.closure, owner.programs.closure);
    assert_eq!(generated.programs.members.len(), 1);
    assert_eq!(
        owner
            .programs
            .members
            .iter()
            .filter(|p| **p == generated.programs.members[0])
            .count(),
        1
    );
}

/// The ordinary shared publication seam. The complete actual predecessor is
/// bound before a single owner is overlaid; acquisition reports remain optional.
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let c = certificate();
    assert_eq!(json!(prior.receipt()), c["predecessor"]["receipt"]);
    assert!(prior.evaluation().is_none());
    dependencies(prior, false);
    let before = expected_owner(false);
    let mut input = prior.input().clone();
    *input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == before.owner)
        .unwrap() = expected_owner(true);
    let proof = OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest(&c),
    };
    input.provenance.push(proof.clone());
    let next = assemble_owned_release(input, Default::default()).unwrap();
    dependencies(&next, true);
    assert_eq!(next.receipt().provenance.last(), Some(&proof));
    let mut inverse = next.input().clone();
    let restored = inverse
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == before.owner)
        .unwrap();
    *restored = before;
    assert_eq!(inverse.provenance.pop(), Some(proof));
    assert_eq!(
        inverse,
        *prior.input(),
        "only one exact gap and proof provenance may change"
    );
    let a = json!(prior.receipt());
    let b = json!(next.receipt());
    for field in [
        "definitions",
        "registry",
        "routing",
        "mapping",
        "roles",
        "normalization",
        "rewards",
        "items",
        "item_source",
        "tree",
        "source",
        "query_sets",
        "query_rows",
        "query_policy_bytes",
    ] {
        assert_eq!(a[field], b[field], "unchanged {field}");
    }
    assert_ne!(prior.receipt().input, next.receipt().input);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}

/// Optional offline reproduction of the certificate's original acquisition.
/// No source execution: authenticate full bytes, joins and observed projections.
pub fn check_acquisition() {
    check_authored();
    let c = certificate();
    let predecessor = &c["predecessor"];
    let release: Value = serde_json::from_slice(&pinned(&predecessor["release"])).unwrap();
    assert_eq!(release, predecessor["receipt"]);
    let package = root()
        .join(predecessor["release"]["path"].as_str().unwrap())
        .parent()
        .unwrap()
        .to_path_buf();
    for artifact in release["artifacts"].as_array().unwrap() {
        let file = artifact["file"].as_str().unwrap();
        assert!(!file.contains(['/', '\\']));
        let bytes = fs::read(package.join(file)).unwrap();
        assert_eq!(json!(bytes.len()), artifact["bytes"]);
        assert_eq!(sha(&bytes), artifact["sha256"]);
    }
    let validation: Value = serde_json::from_slice(&pinned(&predecessor["validation"])).unwrap();
    assert_eq!(validation["status"], "validated");
    assert_eq!(validation["after"], release["input"]);
    for flag in [
        "all_import_outputs_preserved",
        "independent_rebuild_byte_identical",
        "original_build_source_bytes_unchanged",
        "prior_package_unchanged",
        "source_files_unchanged",
    ] {
        assert_eq!(validation[flag], true);
    }
    assert_eq!(validation["complete_original_builds_claimed"], false);
    let s = &c["source"];
    for pin in s["files"].as_array().unwrap() {
        let text = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(pin["path"].as_str().unwrap()),
        )
        .unwrap()
        .replace("\r\n", "\n");
        assert_eq!(json!(text.len()), pin["bytes"]);
        assert_eq!(sha(text.as_bytes()), pin["sha256"]);
        for excerpt in s["excerpts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["path"] == pin["path"])
        {
            let first = excerpt["first"].as_u64().unwrap() as usize;
            let last = excerpt["last"].as_u64().unwrap() as usize;
            assert_eq!(
                json!(
                    text.lines()
                        .skip(first - 1)
                        .take(last - first + 1)
                        .collect::<Vec<_>>()
                ),
                excerpt["lines"]
            );
        }
    }
    let mut first = None;
    for pin in s["reports"].as_array().unwrap() {
        let observed: Value = serde_json::from_slice(&pinned(pin)).unwrap();
        assert_eq!(observed["source_hash"], s["manifest"]["sha256"]);
        assert_eq!(
            observed["evidence"]["manifest_sha256"],
            observed["source_hash"]
        );
        assert_eq!(observed["evidence"]["native_input_closure"], false);
        assert_eq!(observed["evidence"]["native_owner_coverage"], false);
        let files = observed["evidence"]["files"].as_array().unwrap();
        assert_eq!(files.len(), 8);
        let mut unique = BTreeSet::new();
        for file in files {
            assert!(unique.insert(file["path"].as_str().unwrap()));
            assert_eq!(
                s["files"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|p| p["path"] == file["path"] && p["sha256"] == file["sha256"])
                    .count(),
                1
            );
        }
        let cases = observed["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 6);
        for (actual, expected) in cases.iter().zip(s["cases"].as_array().unwrap()) {
            assert_eq!(actual["name"], expected["name"]);
            assert_eq!(actual["xml_sha256"], expected["xml_sha256"]);
            let state = &actual["state"];
            assert_eq!(state["scalability"]["charm"], expected["scalability"]);
            for flag in [
                "saved_items_preserved",
                "saved_selections_preserved",
                "main_output_preserved",
                "original_functions_preserved",
            ] {
                assert_eq!(state[flag], true);
            }
            assert_eq!(state["items"].as_array().unwrap().len(), 1);
            assert_eq!(state["items"][0]["id"], 27);
            assert_eq!(
                state["items"][0]["loaded"]["charmLimit"],
                expected["loaded_charm_capacity"]
            );
        }
        let probes = cases[0]["state"]["items"][0]["probes"].as_array().unwrap();
        assert_eq!(probes.len(), 43);
        for control in s["controls"].as_array().unwrap() {
            let matched: Vec<_> = probes
                .iter()
                .filter(|p| p["name"] == control["name"])
                .collect();
            assert_eq!(matched.len(), 1);
            assert_eq!(matched[0]["raw"], control["raw"]);
            assert_eq!(matched[0]["after"]["charmLimit"], control["capacity"]);
        }
        if let Some(previous) = &first {
            assert_eq!(&observed, previous);
        }
        first = Some(observed);
    }
}
