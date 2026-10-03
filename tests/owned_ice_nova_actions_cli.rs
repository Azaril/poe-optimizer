//! Source-selected action alternatives feed the existing owned query/import path.
#[path = "support/owned_ice_nova_actions.rs"]
mod family;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: impl AsRef<Path>, v: &impl Serialize) {
    fs::write(p, serde_json::to_vec(v).unwrap()).unwrap();
}
fn command(name: &str) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    c.arg(name);
    c
}
fn success(o: Output) -> Value {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}
fn publish(input: &Path, out: &Path) -> Value {
    success(
        command("assemble-owned-release")
            .arg(input)
            .arg("--output")
            .arg(out)
            .output()
            .unwrap(),
    )
}

#[test]
fn ice_nova_packet_preserves_partial_mechanics_and_exact_source_correspondence() {
    family::check_authored();
}

fn compare_original(
    case: usize,
    xml: &[u8],
    out: &Path,
    prior_package: &Path,
    package: &Path,
) -> Value {
    let old = out.join(format!("prior-original-{case:02}"));
    let new = out.join(format!("original-{case:02}"));
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    selected::canonical(&mut a);
    selected::canonical(&mut b);
    assert_eq!(
        a, b,
        "new declarations/mappings change no original occurrence, value, inventory or query"
    );
    let mut a: Value = read(old.join("sidecar.json"));
    let mut b: Value = read(new.join("sidecar.json"));
    let before: Value = read(out.join("prior-receipt.json"));
    let after: Value = read(out.join("receipt.json"));
    // Sidecar identity commits to policy AND this request's query templates;
    // the release identity intentionally commits to the reusable policy alone.
    for (sidecar, package) in [(&a, prior_package), (&b, package)] {
        let policy: poe_optimizer_import::owned_normalize::NormalizationPolicy =
            read(package.join("normalization.json"));
        let queries: Vec<poe_optimizer_import::owned_normalize::ImportQueryTemplate> =
            read(package.join(format!("queries-original-{case:02}.json")));
        let expected = poe_optimizer_core::owned_content::digest_owned(
            "owned-normalization-policy-v3",
            &(policy, queries),
            poe_optimizer_import::owned_normalize::NormalizationLimits::default().max_policy_bytes,
        )
        .unwrap();
        assert_eq!(sidecar["policy"], json!(expected));
    }
    b["policy"] = a["policy"].clone();
    for (field, receipt) in [
        ("mapping", "mapping"),
        ("registry", "registry"),
        ("definitions", "definitions"),
        ("skill_roles", "roles"),
        ("reward_policy", "rewards"),
        ("item_policy", "items"),
        ("item_source_policy", "item_source"),
        ("tree_policy", "tree"),
    ] {
        assert_eq!(a[field], before[receipt]);
        assert_eq!(b[field], after[receipt]);
        b[field] = a[field].clone();
    }
    for row in b["item_texts"].as_array_mut().unwrap() {
        assert_eq!(row["attribution"]["item_lines"], after["items"]);
        assert_eq!(row["attribution"]["policy"], after["item_source"]);
        row["attribution"]["item_lines"] = before["items"].clone();
        row["attribution"]["policy"] = before["item_source"].clone();
    }
    // The draft digest includes its fresh random lineage, compared structurally above.
    b["draft"] = a["draft"].clone();
    selected::canonical(&mut a);
    selected::canonical(&mut b);
    assert_eq!(
        a, b,
        "all source evidence and unchanged occurrence correspondence survive"
    );
    let mut a = selected::selection(xml, &old);
    let mut b = selected::selection(xml, &new);
    selected::canonical(&mut a);
    selected::canonical(&mut b);
    assert_eq!(a, b);
    let mut a = selected::finalize(
        xml,
        &old,
        &out.join(format!("prior-selected-{case:02}.json")),
    );
    let mut b = selected::finalize(xml, &new, &out.join(format!("selected-{case:02}.json")));
    for (report, directory) in [(&a, &old), (&b, &new)] {
        let sidecar: Value = read(directory.join("sidecar.json"));
        assert_eq!(report["draft_digest"], sidecar["draft"]);
        assert_eq!(report["finalization"]["draft_digest"], sidecar["draft"]);
    }
    b["draft_digest"] = a["draft_digest"].clone();
    b["finalization"]["draft_digest"] = a["finalization"]["draft_digest"].clone();
    selected::canonical(&mut a);
    selected::canonical(&mut b);
    assert!(
        a == b,
        "selected finalization changed beyond its checked lineage-bound digest"
    );
    let count = b["finalization"]["issues"].as_array().unwrap().len();
    assert_eq!(count, [119, 116, 108, 121, 19][case - 1]);
    json!({"original":case,"source_sha256":format!("{:x}",Sha256::digest(xml)),"selected_issues":count,"whole_draft_and_provenance_preserved":true,"calculation":"not_run"})
}

fn requests(xml: &str, c: &Value) -> Vec<Value> {
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([63; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    evidence.rows().iter().filter(|row|row.occurrence().name()=="Gem" && row.attribute("skillId").is_some_and(|a|a.decoded().unwrap()==c["source_identity"]["primary_effect_id"].as_str().unwrap()))
        .flat_map(|row|["main","calcs"].map(|context|json!({"skill_use":{"source_sha256":evidence.identity().source_sha256,"occurrence_ordinal":row.occurrence().id().ordinal(),"expected_gem":c["physical_gem"]},"context":context}))).collect()
}
fn resolve(package: &Path, adapter: &Path, source: &Path, request: &Path) -> Value {
    {
        let value = success(
            command("resolve-owned-action")
                .arg(source)
                .arg("--release")
                .arg(package)
                .arg("--correspondence")
                .arg(adapter)
                .arg("--request")
                .arg(request)
                .output()
                .unwrap(),
        );
        assert_eq!(value["document_kind"], "owned_source_action_resolution");
        for field in ["source_execution", "calculation", "whole_build_parity"] {
            assert_eq!(value[field], false);
        }
        value
    }
}
fn normalize_queries(package: &Path, xml: &Path, queries: &Path, out: &Path) -> Value {
    let mut cmd = command("normalize-owned");
    cmd.arg(xml);
    for (flag, file) in [
        ("--policy", "normalization.json"),
        ("--registry", "registry.json"),
        ("--definitions", "schema.json"),
        ("--mapping", "mapping.json"),
        ("--roles", "roles.json"),
        ("--rewards", "rewards.json"),
        ("--items", "items.json"),
        ("--item-source", "item-source.json"),
        ("--tree-policy", "tree-normalization.json"),
    ] {
        cmd.arg(flag).arg(package.join(file));
    }
    success(
        cmd.arg("--queries")
            .arg(queries)
            .arg("--output")
            .arg(out)
            .output()
            .unwrap(),
    )
}

fn diagnostic_controls(package: &Path, adapter: &Path, out: &Path) -> Vec<Value> {
    let path = root().join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let original = fs::read_to_string(&path).unwrap();
    let c: Value = family::read("correspondence.json");
    let source = ImportedBuildInstance::from_decoded(
        decode_build(original.as_bytes()).unwrap(),
        BuildLineage::from_bytes([63; 16]),
        Default::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let targets = requests(&original, &c);
    assert_eq!(targets.len(), 8);
    for (i, request) in targets.iter().enumerate() {
        let p = out.join(format!("request-original-{i}.json"));
        write(&p, request);
        let result = resolve(package, adapter, &path, &p);
        assert_eq!(result["report"]["selection"]["kind"], "absent");
        assert_eq!(result["report"]["target"]["kind"], "action");
        assert_eq!(
            result["report"]["target"]["value"]["stat_set"],
            c["stat_sets"][0]["stat_set"]
        );
    }
    // The fixture's saved selected preset is located from source, never a production default.
    let skills = evidence
        .rows()
        .iter()
        .find(|r| r.occurrence().name() == "Skills")
        .unwrap();
    let active = skills
        .attribute("activeSkillSet")
        .unwrap()
        .decoded()
        .unwrap();
    let selected = targets
        .iter()
        .position(|request| {
            let row = &evidence.rows()
                [request["skill_use"]["occurrence_ordinal"].as_u64().unwrap() as usize];
            let group = &evidence.rows()[row.occurrence().parent().unwrap().ordinal() as usize];
            let set = &evidence.rows()[group.occurrence().parent().unwrap().ordinal() as usize];
            set.attribute("id").unwrap().decoded().unwrap() == active
        })
        .unwrap();
    let ordinal = targets[selected]["skill_use"]["occurrence_ordinal"]
        .as_u64()
        .unwrap() as usize;
    let row = &evidence.rows()[ordinal];
    let range = row.occurrence().range();
    let fragment = &original[range.clone()];
    assert!(fragment.trim_end().ends_with("/>"));
    let effect = c["source_identity"]["primary_effect_id"].as_str().unwrap();
    let child =
        |name: &str, index: &str| format!("<{name} grantedEffect=\"{effect}\" index=\"{index}\"/>");
    let mut reports = vec![];
    for (name, children, main, calcs) in [
        (
            "independent",
            format!(
                "{}{}",
                child("StatSetIndex", "2"),
                child("StatSetCalcsIndex", "1")
            ),
            Some(1),
            Some(0),
        ),
        (
            "reverse",
            format!(
                "{}{}",
                child("StatSetIndex", "1"),
                child("StatSetCalcsIndex", "2")
            ),
            Some(0),
            Some(1),
        ),
        ("malformed", child("StatSetIndex", "bad"), None, Some(0)),
        ("zero", child("StatSetIndex", "0"), None, Some(0)),
        ("fractional", child("StatSetIndex", "1.5"), None, Some(0)),
        ("outside", child("StatSetIndex", "3"), None, Some(0)),
        (
            "duplicate",
            format!(
                "{}{}",
                child("StatSetIndex", "1"),
                child("StatSetIndex", "2")
            ),
            None,
            Some(0),
        ),
        (
            "missing",
            format!("<StatSetIndex grantedEffect=\"{effect}\"/>"),
            None,
            Some(0),
        ),
    ] {
        let close = fragment.rfind("/>").unwrap();
        let changed_fragment = format!(
            "{}>{children}</Gem>{}",
            &fragment[..close],
            &fragment[close + 2..]
        );
        let mut xml = original.clone();
        xml.replace_range(range.clone(), &changed_fragment);
        let source_path = out.join(format!("control-{name}.xml"));
        fs::write(&source_path, &xml).unwrap();
        let reqs = requests(&xml, &c);
        assert_eq!(reqs.len(), 8);
        let mut result_targets = vec![];
        for (i, request) in reqs.iter().enumerate() {
            let p = out.join(format!("control-{name}-request-{i}.json"));
            write(&p, request);
            let resolved = resolve(package, adapter, &source_path, &p);
            let expected = if i == selected {
                main
            } else if i == selected + 1 {
                calcs
            } else {
                Some(0)
            };
            let target = &resolved["report"]["target"];
            match expected {
                Some(index) => {
                    assert_eq!(target["kind"], "action", "{name}/{i}");
                    assert_eq!(
                        target["value"]["stat_set"],
                        c["stat_sets"][index]["stat_set"]
                    );
                }
                None => {
                    assert_eq!(target["kind"], "unresolved", "{name}/{i}");
                    assert_eq!(resolved["report"]["selection"]["kind"], "pending");
                }
            }
            result_targets.push(target.clone());
        }
        if name == "independent" {
            let old_queries: Value = read(package.join("queries-original-05.json"));
            let mut template = old_queries
                .as_array()
                .unwrap()
                .iter()
                .find(|q| q["target"]["kind"] == "action")
                .unwrap()
                .clone();
            template["target"] = result_targets[selected].clone();
            let q = out.join("ice-nova-diagnostic-query.json");
            write(&q, &vec![template]);
            let target_dir = out.join("ice-nova-diagnostic");
            let report = normalize_queries(package, &source_path, &q, &target_dir);
            assert_eq!(report["verification"]["calculation"], "not_run");
            let d: Value = read(target_dir.join("draft.json"));
            let sidecar: Value = read(target_dir.join("sidecar.json"));
            let origin = sidecar["origins"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| {
                    v["source"]["ordinal"] == reqs[selected]["skill_use"]["occurrence_ordinal"]
                })
                .unwrap();
            let link = |kind: &str| {
                let found: Vec<_> = origin["links"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|v| v["kind"] == kind)
                    .collect();
                assert_eq!(found.len(), 1);
                found[0]["value"].clone()
            };
            let known = |v: Value| json!({"kind":"known","value":v});
            let preset = &d["draft"]["query_presets"]["members"][0];
            assert_eq!(link("query_preset"), preset["id"]);
            let queries = preset["queries"]["requests"]["members"].as_array().unwrap();
            assert_eq!(queries.len(), 1);
            assert_eq!(
                queries[0]["target"],
                json!({"kind":"action","value":{
                    "action":{"actor":{"kind":"player"},"provider":{
                        "root":{"kind":"skill_use","value":known(link("skill"))},
                        "grant_path":{"members":[known(c["entering_grant"].clone())],"completion":{"kind":"complete"}}
                    },"output":known(c["output"].clone())},
                    "part":known(c["part"].clone()),"mode":known(c["mode"].clone()),
                    "stat_set":known(c["stat_sets"][1]["stat_set"].clone())
                }})
            );
            let gem = d["draft"]["gems"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .find(|g| g["id"] == link("gem"))
                .unwrap();
            assert_eq!(gem["definition"], known(c["physical_gem"].clone()));
            assert_eq!(gem["parameters"]["completion"]["kind"], "pending");
            let skill = d["draft"]["skills"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["id"] == link("skill"))
                .unwrap();
            assert_eq!(
                skill["source"],
                json!({"kind":"gem","value":known(link("gem"))})
            );
            let unfinished = selected::finalize(
                xml.as_bytes(),
                &target_dir,
                &out.join("diagnostic-selected.json"),
            );
            let issues = serde_json::to_string(&unfinished["finalization"]["issues"]).unwrap();
            for code in [
                "gem-parameters-not-converted",
                "usage-preferences-not-converted",
            ] {
                assert!(
                    issues.contains(code),
                    "query selection must preserve {code}"
                );
            }
        }
        reports.push(json!({"name":name,"resolved_contexts":8,"archived_selection_resolutions_preserved":true,"independent_main_calcs":true}));
    }
    let stale = out.join("request-stale.json");
    write(&stale, &targets[selected]);
    let changed = out.join("control-independent.xml");
    assert_eq!(
        resolve(package, adapter, &changed, &stale)["report"]["target"]["kind"],
        "unresolved"
    );
    assert_eq!(fs::read_to_string(path).unwrap(), original);
    reports
}

#[test]
#[ignore = "requires checked Sniper release and authenticated two-mode spell source evidence"]
fn publish_ice_nova_actions_and_resolve_saved_alternatives_without_changing_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ICE_NOVA_PRIOR").expect("checked prior"),
    );
    let out =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_ICE_NOVA_OUTPUT").expect("new output"));
    assert!(!out.exists());
    let prior_inventory = release::inventory(&prior_path);
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
    assert_eq!(
        release::inventory(&package),
        release::inventory(&out.join("rebuilt"))
    );
    assert_eq!(release::inventory(&package).len(), 18);
    let adapter = out.join("adapter.json");
    write(&adapter, &family::adapter(&next));
    let mut originals = vec![];
    for case in 1..=5 {
        let query = format!("queries-original-{case:02}.json");
        assert_eq!(
            fs::read(package.join(&query)).unwrap(),
            fs::read(prior_path.join(query)).unwrap()
        );
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
        originals.push(compare_original(
            case,
            &fs::read(xml).unwrap(),
            &out,
            &prior_path,
            &package,
        ));
    }
    let controls = diagnostic_controls(&package, &adapter, &out);
    assert_eq!(prior_inventory, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":originals,"controls":controls,"queries":110,"artifacts":18,"complete_original_builds":0,"rebuild_byte_identical":true,"prior_unchanged":true}),
    );
}
