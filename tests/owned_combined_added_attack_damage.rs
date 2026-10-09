#[allow(dead_code)]
#[path = "support/owned_minion_attack_selection.rs"]
mod attack_selection_family;
#[path = "support/owned_combined_added_attack_damage.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::{Value, json};
use std::path::PathBuf;
#[test]
fn one_reviewed_source_factor_becomes_one_exact_action_combined_factor() {
    family::check_authored();
}
#[test]
fn factor_query_requires_exact_origin_membership_and_original_rounding_order() {
    for edit in 0..7 {
        let mut c: Value = family::read("consumer.json");
        let mut q: Value = family::read("queries.json");
        match edit {
            0 => {
                q[0]["groups"][0]["members"]["members"][0]["producer"]["origin"]["authored"] =
                    json!(true)
            }
            1 => {
                q[0]["groups"][0]["members"]["members"][0]["producer"]["origin"]["supplies"][0]["slot"]
                    ["key"] = json!("def.0000000000003094")
            }
            2 => q[0]["groups"][0]["members"]["members"] = json!([]),
            3 => {
                let copy = q[0]["groups"][0]["members"]["members"][0].clone();
                q[0]["groups"][0]["members"]["members"]
                    .as_array_mut()
                    .unwrap()
                    .push(copy);
            }
            4 => {
                c["owners"][0]["programs"]["members"][0]["nodes"][5]["expression"]["mode"] =
                    json!("nearest_ties_positive")
            }
            5 => {
                c["owners"][0]["programs"]["members"][0]["nodes"][6]["expression"]["value"] =
                    json!("product")
            }
            6 => c["owners"][0]["programs"]["closure"] = json!({"kind":"complete"}),
            _ => unreachable!(),
        }
        assert!(std::panic::catch_unwind(|| family::check_programs(&c, &q)).is_err());
    }
}
#[test]
fn bounded_numeric_proof_requires_all_actual_profile_and_factor_writer_bodies() {
    let d: Value = family::read("dependencies.json");
    let schema = json!({"definitions":d["supply_inventory"]["definitions"],"slots":d["supply_inventory"]["slots"]});
    let owners = d["supply_inventory"]["owners"].clone();
    let routes = json!([d["route"]]);
    family::check_dependencies(&schema, &owners, &routes);
    for edit in 0..5 {
        let mut changed = owners.clone();
        let actor = changed
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|o| {
                o["programs"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["id"] == "finite-actor-baseline")
            })
            .unwrap();
        let programs = actor["programs"]["members"].as_array_mut().unwrap();
        let i = programs
            .iter()
            .position(|p| p["id"] == "finite-actor-baseline")
            .unwrap();
        match edit {
            0 => programs[i]["nodes"][1]["expression"]["value"]["value"]["value"] = json!(1.2),
            1 => programs[i]["effects"][1]["when"] = json!("fact-5"),
            2 => {
                programs.remove(i);
            }
            3 => {
                let mut extra = programs[i].clone();
                extra["id"] = json!("inactive-extra-profile-writer");
                extra["nodes"][5]["expression"]["value"]["value"] = json!(false);
                extra["effects"][1]["when"] = json!("fact-5");
                programs.push(extra);
            }
            4 => programs[i]["effects"][1]["effect"]["value"] = json!("fact-2"),
            _ => unreachable!(),
        }
        assert!(
            std::panic::catch_unwind(|| family::check_dependencies(&schema, &changed, &routes))
                .is_err()
        );
    }
}
#[test]
fn source_domain_rejects_zero_conditional_inherited_and_precision_suppliers() {
    let original: Value = family::read("source-evidence.json");
    family::check_source_value(&original);
    for edit in 0..6 {
        let mut changed = original.clone();
        match edit{
   0=>changed["catalog"]["added_modifiers"][0]["record"]["value"]=json!(0),
   1=>changed["catalog"]["stat_consumers"].as_array_mut().unwrap().push(json!({"skill_id":"AnySupport","stat":"added_damage_+%_final","row":["added_damage_+%_final",0]})),
   2=>changed["catalog"]["full_profile"]["damageFixup"]=json!(0),
   3=>changed["high_precision_mods"]["AddedDamage"]=json!({"MORE":3}),
   4=>changed["extra_stats"].as_array_mut().unwrap().push(json!({"path":"modCache/conditional","record":{"name":"ExtraSkillStat","type":"LIST","value":{"key":"active_skill_added_damage_+%_final","value":0}}})),
   5=>changed["catalog"]["added_modifiers"][0]["record"]["name"]=json!("AddedPhysicalDamage"),
   _=>unreachable!()
  }
        assert!(std::panic::catch_unwind(|| family::check_source_value(&changed)).is_err());
    }
}
#[test]
#[ignore = "authenticates pinned original source; does not host Lua"]
fn retained_domain_and_full_pass_corroboration_authenticate() {
    family::check_source(true);
}
#[test]
fn finite_correspondence_requires_exact_accounted_party_absence() {
    for body in [
        "",
        "<Party/>",
        "<Party destination=\"All\" append=\"false\" ShowAdvanceTools=\"true\"/>",
    ] {
        family::assert_empty_party_source(&format!("<PathOfBuilding2>{body}</PathOfBuilding2>"));
    }
    for body in [
        "<Party/><Party/>",
        "<Party><ImportedBuffs name=\"Aura\"/></Party>",
        "<Party><ImportedBuffs name=\"Aura\">0</ImportedBuffs></Party>",
        "<Party><ImportedBuffs name=\"Aura\">-10|x|AddedDamage|MORE|Attack|-</ImportedBuffs></Party>",
        "<Party><ExportedBuffs/></Party>",
        "<Party>0</Party>",
        "<Party extra=\"0\"/>",
        "<Party append=\"0\"/>",
        "<Party xmlns=\"unknown\"/>",
    ] {
        assert!(
            std::panic::catch_unwind(|| family::assert_empty_party_source(&format!(
                "<PathOfBuilding2>{body}</PathOfBuilding2>"
            )))
            .is_err()
        );
    }
}
fn check_party_input_obligations(package: &std::path::Path, out: &std::path::Path) {
    use poe_optimizer_core::owned_draft::{DraftLimits, decode_draft};
    use poe_optimizer_import::{
        build_instance::{ImportedBuildInstance, InstanceImportLimits},
        build_source, decode_build,
        owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    };
    use std::fs;
    let original = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap();
    family::assert_empty_party_source(&original);
    let projection = build_source::project_xml(&original).unwrap();
    let parties: Vec<_> = projection
        .sections()
        .iter()
        .filter(|s| s.element().name() == "Party")
        .collect();
    assert_eq!(parties.len(), 1);
    let range = parties[0].element().source_range();
    let open = original[range.clone()]
        .strip_suffix("/>")
        .unwrap()
        .to_owned()
        + ">";
    let bodies = [
        ("unchanged", None),
        ("empty", Some(String::new())),
        (
            "empty-import",
            Some(r#"<ImportedBuffs name="Aura"/>"#.to_owned()),
        ),
        (
            "aura-zero",
            Some(
                r#"<ImportedBuffs name="Aura">Probe
100
0|Probe|AddedDamage|MORE|Attack|-|type=GlobalEffect/effectType=Aura
---</ImportedBuffs>"#
                    .to_owned(),
            ),
        ),
        (
            "aura-positive",
            Some(
                r#"<ImportedBuffs name="Aura">Probe
100
20|Probe|AddedDamage|MORE|Attack|-|type=GlobalEffect/effectType=Aura
---</ImportedBuffs>"#
                    .to_owned(),
            ),
        ),
        (
            "aura-negative",
            Some(
                r#"<ImportedBuffs name="Aura">Probe
100
-10|Probe|AddedPhysicalDamage|INC|Attack|-|type=GlobalEffect/effectType=Aura
---</ImportedBuffs>"#
                    .to_owned(),
            ),
        ),
        (
            "other-effects",
            Some(
                r#"<ImportedBuffs name="Aura">otherEffects
Probe|100|20|Probe|AddedDamage|MORE|Attack|-
---</ImportedBuffs>"#
                    .to_owned(),
            ),
        ),
        (
            "enemy-mods",
            Some(
                r#"<ImportedBuffs name="EnemyMods">Probe
20|Probe|SelfPhysicalMin|BASE|Attack|-</ImportedBuffs>"#
                    .to_owned(),
            ),
        ),
    ];
    fs::create_dir_all(out).unwrap();
    let mut reports = Vec::new();
    let mut expected_issues: Option<Value> = None;
    let mut original_queries: Option<Value> = None;
    for (name, body) in bodies {
        let mut xml = original.clone();
        if let Some(body) = &body {
            xml.replace_range(range.clone(), &format!("{open}{body}</Party>"));
        }
        if body.as_ref().is_none_or(String::is_empty) {
            family::assert_empty_party_source(&xml)
        } else {
            assert!(std::panic::catch_unwind(|| family::assert_empty_party_source(&xml)).is_err());
        }
        let path = out.join(format!("{name}.xml"));
        fs::write(&path, &xml).unwrap();
        let dir = out.join(name);
        release::normalize(package, &path, 5, &dir);
        let report = selected::finalize_with_definitions(
            xml.as_bytes(),
            &dir,
            &out.join(format!("{name}-selection.json")),
            &package.join("schema.json"),
        );
        let selected = selected::selection(xml.as_bytes(), &dir);
        let draft_bytes = fs::read(dir.join("draft.json")).unwrap();
        let typed = decode_draft(&draft_bytes, DraftLimits::default()).unwrap();
        let draft: Value = serde_json::from_slice(&draft_bytes).unwrap();
        let draft = &draft["draft"];
        let choice = draft["choice_presets"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == selected["build"]["choices"])
            .unwrap();
        let pending = &choice["choices"]["completion"];
        assert_eq!(pending["kind"], "pending");
        assert_eq!(pending["code"], "configuration-roles-not-converted");
        let issue = &pending["id"];
        assert!(
            report["finalization"]["issues"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["id"] == *issue && r["code"] == pending["code"])
        );
        let mut issues = report["finalization"]["issues"].clone();
        selected::canonical(&mut issues);
        let mut query_presets = draft["query_presets"].clone();
        selected::canonical(&mut query_presets);
        if let Some(expected) = &expected_issues {
            // Exact saved Action correspondences bind the entire source XML.
            // Even equivalent Party spelling must retain their snapshot guards.
            let mut with_snapshot_guards = expected.as_array().unwrap().clone();
            let before = original_queries.as_ref().unwrap();
            assert_eq!(query_presets["members"].as_array().unwrap().len(), 1);
            let owner = query_presets["members"][0]["id"].clone();
            let requests = query_presets["members"][0]["queries"]["requests"]["members"]
                .as_array_mut()
                .unwrap();
            for index in [14usize, 16] {
                let original = &before["members"][0]["queries"]["requests"]["members"][index];
                assert_eq!(original["target"]["kind"], "action");
                assert_eq!(requests[index]["id"], original["id"]);
                assert_eq!(requests[index]["metric"], original["metric"]);
                let target = &requests[index]["target"];
                assert_eq!(target["kind"], "pending");
                assert_eq!(target["value"]["code"], "query-source-snapshot-mismatch");
                assert_eq!(target["value"]["candidates"], json!([]));
                with_snapshot_guards.push(json!({
                    "id":target["value"]["id"], "code":target["value"]["code"],
                    "owner":owner,
                    "path":format!("query_presets.members[0].queries.requests.members[{index}].target.value")
                }));
                requests[index]["target"] = original["target"].clone();
            }
            assert_eq!(
                query_presets, *before,
                "only the two exact source snapshot guards may change saved queries"
            );
            assert_eq!(
                issues,
                json!(with_snapshot_guards),
                "all four original issues survive alongside the two exact snapshot guards"
            );
        } else {
            assert_eq!(name, "unchanged");
            assert_eq!(issues.as_array().unwrap().len(), 4);
            expected_issues = Some(issues);
            original_queries = Some(query_presets);
        }
        let source = ImportedBuildInstance::from_decoded(
            decode_build(xml.as_bytes()).unwrap(),
            typed.input().allocator.lineage(),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
        let party = evidence
            .rows()
            .iter()
            .find(|r| r.occurrence().name() == "Party")
            .unwrap();
        let mut queue = vec![party.occurrence().id()];
        let mut sources = Vec::new();
        while let Some(id) = queue.pop() {
            let row = &evidence.rows()[id.ordinal() as usize];
            queue.extend(row.children());
            sources.push(id);
        }
        let sidecar: Value =
            serde_json::from_slice(&fs::read(dir.join("sidecar.json")).unwrap()).unwrap();
        for id in &sources {
            let origin = sidecar["origins"]
                .as_array()
                .unwrap()
                .iter()
                .find(|o| o["source"] == json!(id))
                .unwrap();
            assert_eq!(origin["disposition"], json!({"kind":"contributes"}));
            assert!(
                origin["links"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|l| l["kind"] == "issue" && l["value"] == *issue)
            );
        }
        reports.push(json!({"case":name,"party_source_rows":sources,"live_selected_issue":issue,"finalization":report["finalization"],"verification":report["verification"],"source_only_dispositions_added":false}));
    }
    fs::write(
        out.join("validation.json"),
        serde_json::to_vec_pretty(&reports).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "requires COMBINED_ADDED_ATTACK_PRIOR, fresh OUTPUT and retained source"]
fn publish_combined_added_attack_preserving_all_five_originals() {
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_COMBINED_ADDED_ATTACK_OUTPUT").expect("output"),
    );
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_COMBINED_ADDED_ATTACK_PRIOR").expect("prior"),
        ),
        out.clone(),
        &family::data(),
        &[],
        &[
            "authoring.json",
            "source-evidence.json",
            "dependencies.json",
        ],
        family::stage,
        json!({"new_definitions":1,"new_programs":1,"new_queries":1,"new_routes":0,"whole_build_parity":false,"bounded_source_domain":true}),
        [106, 117, 109, 123, 4],
    );
    check_party_input_obligations(&out.join("package"), &out.join("party-input-controls"));
}
