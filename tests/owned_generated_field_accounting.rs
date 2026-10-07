//! Current source-accounting cutover against pinned earlier imports.
//! No historical importer mode is kept to reconstruct expected evidence.
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance,
    decode_build,
    owned_source::{SourceEvidenceRow, SourceProjectEvidence},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn attr<'a>(row: &'a SourceEvidenceRow<'_>, name: &str) -> Option<&'a str> {
    row.attribute(name).and_then(|a| a.decoded().ok())
}
fn prove_origin_change(
    old: &Value,
    new: &Value,
    draft: &Value,
    old_sidecar: &Value,
    source: &SourceProjectEvidence<'_>,
) -> Value {
    let mut a = old.clone();
    let mut b = new.clone();
    a.as_object_mut().unwrap().remove("links");
    b.as_object_mut().unwrap().remove("links");
    assert_eq!(a, b, "only provenance links may change");
    assert_eq!(new["disposition"]["kind"], "contributes");
    let ordinal = new["source"]["ordinal"].as_u64().unwrap() as usize;
    let mut row = &source.rows()[ordinal];
    assert!(matches!(row.occurrence().name(), "Skill" | "Gem"));
    while row.occurrence().name() != "SkillSet" {
        row = &source.rows()[row.occurrence().parent().unwrap().ordinal() as usize];
    }
    let set = row;
    let parent = &source.rows()[set.occurrence().parent().unwrap().ordinal() as usize];
    assert_eq!(parent.occurrence().name(), "Skills");
    assert_ne!(
        attr(parent, "activeSkillSet").unwrap(),
        attr(set, "id").unwrap(),
        "deferred proof is for an actual nonselected preset"
    );
    let set_origin = &old_sidecar["origins"][set.occurrence().id().ordinal() as usize];
    let presets: Vec<_> = set_origin["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["kind"] == "skill_preset")
        .collect();
    assert_eq!(presets.len(), 1);
    let preset_link = presets[0];
    let preset = draft["draft"]["skill_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == preset_link["value"])
        .unwrap();
    let mut expected = vec![preset_link.clone()];
    for (completion, code) in [
        (
            &preset["intent"]["generated_inputs"]["completion"],
            "generated-skill-inputs-not-converted",
        ),
        (
            &preset["intent"]["usage"]["completion"],
            "usage-preferences-not-converted",
        ),
        (
            &preset["support_origins"]["completion"],
            "support-origin-discovery-not-converted",
        ),
    ] {
        assert_eq!(completion["kind"], "pending");
        assert_eq!(completion["code"], code);
        let link = json!({"kind":"issue","value":completion["id"]});
        let attached: Vec<_> = old_sidecar["origins"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|origin| origin["links"].as_array().unwrap().contains(&link))
            .collect();
        assert!(!attached.is_empty(), "existing source responsibility");
        for origin in attached {
            let mut owner = &source.rows()[origin["source"]["ordinal"].as_u64().unwrap() as usize];
            while owner.occurrence().name() != "SkillSet" {
                owner = &source.rows()[owner.occurrence().parent().unwrap().ordinal() as usize];
            }
            assert_eq!(
                owner.occurrence().id(),
                set.occurrence().id(),
                "every prior attachment stays in the actual owning preset"
            );
        }
        expected.push(link);
    }
    let configuration: BTreeSet<_> = draft["draft"]["choice_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| {
            let c = &p["choices"]["completion"];
            (c["kind"] == "pending" && c["code"] == "configuration-roles-not-converted")
                .then(|| c["id"].to_string())
        })
        .collect();
    let old_links = old["links"].as_array().unwrap();
    let new_links = new["links"].as_array().unwrap();
    let removed: Vec<_> = old_links
        .iter()
        .filter(|l| !new_links.contains(l))
        .collect();
    assert!(!removed.is_empty());
    assert!(
        removed
            .iter()
            .all(|l| l["kind"] == "issue" && configuration.contains(&l["value"].to_string()))
    );
    for link in old_links.iter().filter(|l| !removed.contains(l)) {
        if !expected.contains(link) {
            expected.push(link.clone());
        }
    }
    assert_eq!(new_links.len(), expected.len());
    assert!(new_links.iter().all(|l| expected.contains(l)));
    assert!(
        !new_links.iter().any(|l| matches!(
            l["kind"].as_str(),
            Some("generated_skill_input" | "gem" | "skill" | "support")
        )),
        "deferred accounting creates no occurrence or resolved input"
    );
    json!({"source":new["source"],"preset":preset["id"],"retained":expected,"removed":removed})
}

fn prove_placeholder_change(
    old: &Value,
    new: &Value,
    draft: &Value,
    old_sidecar: &Value,
    source: &SourceProjectEvidence<'_>,
    policy: &Value,
) -> Value {
    assert_eq!(old["source"], new["source"]);
    assert_eq!(old["disposition"]["kind"], "contributes");
    assert_eq!(
        new["disposition"],
        json!({"kind":"source_only","value":"overwritten-config-placeholder"})
    );
    let ordinal = new["source"]["ordinal"].as_u64().unwrap() as usize;
    let row = &source.rows()[ordinal];
    assert_eq!(row.occurrence().name(), "Placeholder");
    let name = attr(row, "name").unwrap();
    assert!(
        attr(row, "number")
            .unwrap()
            .parse::<f64>()
            .unwrap()
            .is_finite()
    );
    assert_eq!(row.attributes().len(), 2);
    assert!(row.children().is_empty());
    let input_rows: Vec<_> = policy["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|input| input["source_name"] == name)
        .collect();
    assert_eq!(input_rows.len(), 1, "exact raw override policy row");
    let input = input_rows[0];
    let set = &source.rows()[row.occurrence().parent().unwrap().ordinal() as usize];
    assert_eq!(set.occurrence().name(), "ConfigSet");
    assert!(
        !set.children().iter().any(|id| {
            let child = &source.rows()[id.ordinal() as usize];
            child.occurrence().name() == "Input" && attr(child, "name") == Some(name)
        }),
        "these unchanged originals have no authored raw override"
    );
    let links = old_sidecar["origins"][set.occurrence().id().ordinal() as usize]["links"]
        .as_array()
        .unwrap();
    let scenario_links: Vec<_> = links
        .iter()
        .filter(|l| l["kind"] == "scenario_preset")
        .collect();
    let choice_links: Vec<_> = links
        .iter()
        .filter(|l| l["kind"] == "choice_preset")
        .collect();
    assert_eq!(scenario_links.len(), 1);
    assert_eq!(choice_links.len(), 1);
    let scenario = draft["draft"]["scenario_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == scenario_links[0]["value"])
        .unwrap();
    let choice = draft["draft"]["choice_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == choice_links[0]["value"])
        .unwrap();
    assert_eq!(
        scenario["scenario"]["enemy"]["encounter"],
        json!({"kind":"known","value":policy["encounter"]})
    );
    let assumptions = &scenario["scenario"]["assumptions"];
    assert_eq!(assumptions["completion"]["kind"], "pending");
    assert_eq!(
        scenario["scenario"]["usage"]["completion"]["kind"],
        "pending"
    );
    let members = assumptions["members"].as_array().unwrap();
    let presence: Vec<_> = members
        .iter()
        .filter(|a| {
            a["input"]["value"] == input["presence_input"] && a["target"]["kind"] == "enemy"
        })
        .collect();
    assert_eq!(presence.len(), 1);
    assert_eq!(
        presence[0]["value"],
        json!({"kind":"known","value":{"kind":"boolean","value":false}})
    );
    assert!(
        !members
            .iter()
            .any(|a| a["input"]["value"] == input["value_input"])
    );
    let completion = &choice["choices"]["completion"];
    assert_eq!(completion["kind"], "pending");
    assert_eq!(completion["code"], "configuration-roles-not-converted");
    let issue = json!({"kind":"issue","value":completion["id"]});
    assert!(links.contains(&issue));
    assert_eq!(old["links"], json!([issue]));
    assert_eq!(new["links"], json!([scenario_links[0]]));
    json!({"source":new["source"],"name":name,"scenario":scenario["id"],"disposition":new["disposition"]})
}

const PINNED_IMPORTS: [(&str, &str); 5] = [
    (
        "809d64aca292d9b5cad936dc92894028b25122bde3f10b08cd52615441642c75",
        "4d1f8f9daa55b584093201650575da5b54217c46030fb8844f85f63fe0ba09a9",
    ),
    (
        "825f9edad9e3e638df38720c75a9ab432a0014663b68de5c09f157cd44d96205",
        "8d0afbc6312c9c8b1f19f393bbfc9ed3398c5b4a854fe26e1817a73ce4b93f0a",
    ),
    (
        "5c26df0eeff0039f379416ab9330c05f2feb9ba063d375a2b61db8e781468dbf",
        "9367fc3593a17b4a3921e404fe5eea919a9044dd01856596d126e80d898c1bca",
    ),
    (
        "aec54cdef8997cf52adf17cd145a04128bf28739b71aa0a645247665b664ef74",
        "a1f76549f44cc3ab75060e005744f8a633125efae629ed36733e93da931e5a38",
    ),
    (
        "dfbdb387698d08d48502869e4f0c25da1eb943cc45bc7102bbd02514c5ca2a2f",
        "e403daf0e663678bc44eca229d4705f35de8890a9ef5312b12babb5ded178608",
    ),
];
#[test]
#[ignore = "requires pinned penalty02 imports and a fresh output directory"]
fn current_accounting_reimports_all_five_without_changing_build_inputs() {
    let baseline = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_GENERATED_ACCOUNTING_BASELINE")
            .expect("stored penalty02 checkpoint"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_GENERATED_ACCOUNTING_OUTPUT")
            .expect("fresh output directory"),
    );
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    let package = baseline.join("package");
    let inventory = release::inventory(&package);
    let staged = release::load(&package);
    let configuration = read(package.join("normalization.json"))["configuration_inputs"].clone();
    assert_eq!(configuration["inputs"].as_array().unwrap().len(), 14);
    assert_eq!(
        json!(staged.receipt().input),
        "b19134496c85e208a735568b5e5191f3b60f68154be336ab4afb6fd953b6fa01"
    );
    assert_eq!(staged.receipt().query_rows, 110);
    let mut results = vec![];
    for case in 1..=5 {
        let xml = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let bytes = fs::read(&xml).unwrap();
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(&bytes).unwrap(),
            BuildLineage::from_bytes([93; 16]),
            Default::default(),
        )
        .unwrap();
        let source = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
        let before = baseline.join(format!("original-{case:02}"));
        let after = out.join(format!("original-{case:02}"));
        let old_files = release::inventory(&before);
        assert_eq!(
            hash(&fs::read(before.join("draft.json")).unwrap()),
            PINNED_IMPORTS[case - 1].0
        );
        assert_eq!(
            hash(&fs::read(before.join("sidecar.json")).unwrap()),
            PINNED_IMPORTS[case - 1].1
        );
        let report = release::normalize(&package, &xml, case, &after);
        let sidecar_bytes = fs::read(after.join("sidecar.json")).unwrap();
        assert_eq!(report["sidecar_schema_version"], 23);
        assert_eq!(report["sidecar_bytes"], sidecar_bytes.len());
        assert_eq!(report["sidecar_sha256"], hash(&sidecar_bytes));
        let mut old_draft = read(before.join("draft.json"));
        let mut new_draft = read(after.join("draft.json"));
        let mut old = read(before.join("sidecar.json"));
        let mut new: Value = serde_json::from_slice(&sidecar_bytes).unwrap();
        preservation::authenticate(&old, &before, &package, case, &staged);
        preservation::authenticate(&new, &after, &package, case, &staged);
        assert_eq!(old["schema_version"], 21);
        assert_eq!(new["schema_version"], 23);
        assert_eq!(new["source_sha256"], hash(&bytes));
        assert_eq!(new["source_bytes"], bytes.len());
        for v in [&mut old_draft, &mut new_draft, &mut old, &mut new] {
            selected::canonical(v);
        }
        assert_eq!(
            old_draft, new_draft,
            "every value, ID and allocator remains unchanged"
        );
        let old_origins = old["origins"].as_array().unwrap();
        let new_origins = new["origins"].as_array().unwrap();
        assert_eq!(old_origins.len(), new_origins.len());
        let changes: Vec<_> = old_origins
            .iter()
            .zip(new_origins)
            .filter(|(a, b)| a != b)
            .map(|(a, b)| {
                if b["disposition"]["kind"] == "source_only" {
                    prove_placeholder_change(a, b, &new_draft, &old, &source, &configuration)
                } else {
                    prove_origin_change(a, b, &new_draft, &old, &source)
                }
            })
            .collect();
        let placeholder_changes: Vec<_> = changes
            .iter()
            .filter(|c| c.get("disposition").is_some())
            .collect();
        assert_eq!(
            placeholder_changes.len(),
            14,
            "all and only reviewed raw override fields"
        );
        let actual_names: BTreeSet<_> = placeholder_changes
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect();
        let expected_names: BTreeSet<_> = configuration["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["source_name"].as_str().unwrap())
            .collect();
        assert_eq!(actual_names, expected_names);
        let generated_changes: Vec<_> = changes
            .iter()
            .filter(|c| c.get("disposition").is_none())
            .collect();
        if case == 5 {
            assert_eq!(
                generated_changes
                    .iter()
                    .map(|c| c["source"]["ordinal"].as_u64().unwrap())
                    .collect::<Vec<_>>(),
                vec![
                    190, 191, 251, 252, 269, 270, 349, 350, 353, 354, 416, 417, 420, 421
                ]
            );
        }
        if case != 5 {
            assert!(
                generated_changes.is_empty(),
                "no unreviewed generated change in original{case}"
            );
        }
        for v in [&mut old, &mut new] {
            let o = v.as_object_mut().unwrap();
            o.remove("origins");
            o.remove("schema_version");
            o.remove("draft");
        }
        assert_eq!(
            old, new,
            "every other authenticated sidecar dependency is identical"
        );
        let a = selected::selection(&bytes, &before);
        let b = selected::selection(&bytes, &after);
        let mut a = a;
        let mut b = b;
        selected::canonical(&mut a);
        selected::canonical(&mut b);
        assert_eq!(a, b);
        let mut a = selected::finalize_with_definitions(
            &bytes,
            &before,
            &out.join(format!("prior-selected-{case:02}.json")),
            &package.join("schema.json"),
        );
        let mut b = selected::finalize_with_definitions(
            &bytes,
            &after,
            &out.join(format!("selected-{case:02}.json")),
            &package.join("schema.json"),
        );
        selected::canonical(&mut a);
        selected::canonical(&mut b);
        assert_eq!(
            a["finalization"]["draft_digest"],
            read(before.join("sidecar.json"))["draft"]
        );
        assert_eq!(
            b["finalization"]["draft_digest"],
            read(after.join("sidecar.json"))["draft"]
        );
        for report in [&mut a, &mut b] {
            assert_eq!(report["intent_validation"]["schema_issues"], json!([]));
            report["finalization"]
                .as_object_mut()
                .unwrap()
                .remove("draft_digest");
        }
        assert_eq!(a["finalization"], b["finalization"]);
        assert_eq!(
            b["finalization"]["issues"].as_array().unwrap().len(),
            [107, 117, 109, 123, 5][case - 1]
        );
        // Fresh repeat does not reuse a sidecar or provider resolution cache.
        let replay = out.join(format!("replay-original-{case:02}"));
        release::normalize(&package, &xml, case, &replay);
        let mut replay_sidecar = read(replay.join("sidecar.json"));
        preservation::authenticate(&replay_sidecar, &replay, &package, case, &staged);
        selected::canonical(&mut replay_sidecar);
        let mut after_sidecar = read(after.join("sidecar.json"));
        selected::canonical(&mut after_sidecar);
        replay_sidecar.as_object_mut().unwrap().remove("draft");
        after_sidecar.as_object_mut().unwrap().remove("draft");
        assert_eq!(replay_sidecar, after_sidecar);
        let mut replay_draft = read(replay.join("draft.json"));
        selected::canonical(&mut replay_draft);
        assert_eq!(replay_draft, new_draft);
        assert_eq!(old_files, release::inventory(&before));
        assert_eq!(fs::read(&xml).unwrap(), bytes);
        results.push(json!({"original":case,"changes":changes,"sidecar_schema_version":23,"sidecar_sha256":report["sidecar_sha256"],"draft_unchanged":true,"selection_unchanged":true,"selected_issues":b["finalization"]["issues"].as_array().unwrap().len()}));
    }
    assert_eq!(inventory, release::inventory(&package));
    fs::write(out.join("validation.json"),serde_json::to_vec_pretty(&json!({"package":staged.receipt().input,"cases":results,"query_rows":110,"runtime_data_changed":false,"complete_original_builds":0})).unwrap()).unwrap();
}
