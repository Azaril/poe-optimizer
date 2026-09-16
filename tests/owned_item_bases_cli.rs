//! Full finite catalog publication and all-five import evidence, without source execution.
#[allow(dead_code)]
#[path = "support/owned_bundle_cli.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_draft::{DraftAllocationAccess, DraftField, DraftLimits, decode_draft},
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition};
use poe_optimizer_import::{owned_item_bases::ItemBasePolicy, owned_recipe::assemble_owned_recipe};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use support::{bundle, data, json, normalize, root, success};
fn command(cwd: &Path, name: &str) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    c.current_dir(cwd).arg(name);
    c
}
fn predecessor(cwd: &Path) -> PathBuf {
    let mut prior = data().join("current");
    for (name, folder) in [
        ("compile-owned-attributes", "attributes"),
        ("compile-owned-passive-views", "passive-views"),
    ] {
        let out = cwd.join(folder);
        success(
            command(cwd, name)
                .arg(&prior)
                .arg("--catalog")
                .arg(data().join("tree/tree-catalog.json"))
                .arg("--policy")
                .arg(data().join(folder).join("policy.json"))
                .arg("--statistics")
                .arg(data().join(folder).join("statistics.json"))
                .arg("--output")
                .arg(&out)
                .output()
                .unwrap(),
        );
        prior = out;
    }
    let classes = cwd.join("classes");
    success(
        command(cwd, "compile-owned-class-bases")
            .arg(&prior)
            .arg("--source-tree")
            .arg(root().join("vendor/path-of-building-poe2/src/TreeData/0_5/tree.json"))
            .arg("--policy")
            .arg(data().join("class-bases/policy.json"))
            .arg("--output")
            .arg(&classes)
            .output()
            .unwrap(),
    );
    let out = cwd.join("intrinsic");
    success(
        command(cwd, "compile-owned-intrinsic-attack")
            .arg(classes)
            .arg("--catalog")
            .arg(data().join("intrinsic-attack/catalog.json"))
            .arg("--policy")
            .arg(data().join("intrinsic-attack/policy.json"))
            .arg("--definitions")
            .arg(data().join("intrinsic-attack/definitions.json"))
            .arg("--output")
            .arg(&out)
            .output()
            .unwrap(),
    );
    out
}
fn run(
    cwd: &Path,
    prior: &Path,
    catalog: &Path,
    policy: &Path,
    definitions: &Path,
    out: &Path,
) -> Output {
    command(cwd, "compile-owned-item-bases")
        .arg(prior)
        .arg("--catalog")
        .arg(catalog)
        .arg("--policy")
        .arg(policy)
        .arg("--definitions")
        .arg(definitions)
        .arg("--output")
        .arg(out)
        .output()
        .unwrap()
}
#[test]
fn broad_templates_preserve_history_queries_and_unresolved_originals() {
    let temp = tempfile::tempdir().unwrap();
    let cwd = temp.path();
    let prior = predecessor(cwd);
    let before = bundle(&prior);
    let inputs = data().join("item-bases");
    let catalog = inputs.join("catalog.json");
    let policy = inputs.join("policy.json");
    let definitions = inputs.join("definitions.json");
    let out = cwd.join("bases");
    let report = success(run(cwd, &prior, &catalog, &policy, &definitions, &out));
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    let old: Value = serde_json::from_slice(&before["registry.json"]).unwrap();
    let new = json(out.join("registry.json"));
    assert_eq!(new["last_issued"], 9238);
    assert_eq!(
        &new["entries"].as_array().unwrap()[..7482],
        old["entries"].as_array().unwrap()
    );
    let recipe = assemble_owned_recipe(
        serde_json::from_value(json(out.join("recipe.json"))).unwrap(),
        Default::default(),
    )
    .unwrap();
    let compiled =
        CompiledRulePackage::compile(recipe.rules().input(), recipe.schema(), Default::default())
            .unwrap();
    let policy_data: ItemBasePolicy = serde_json::from_value(json(&policy)).unwrap();
    let mut scratch = compiled.new_scratch();
    let mut counts = [0; 2];
    for binding in &policy_data.templates {
        let owner = SchemaSubject::Definition(binding.template.address());
        let row = recipe
            .rules()
            .input()
            .owners
            .iter()
            .find(|r| r.owner == owner)
            .unwrap();
        assert!(!row.programs.is_complete());
        let programs: Vec<_> = row.programs.members.iter().filter(|p| p.effects.iter().any(|e|
            matches!(&e.effect, RuleEffectKind::Capability{capability,..} if capability == &policy_data.capability))).collect();
        assert_eq!(programs.len(), 1);
        let result = compiled
            .evaluate(&owner, &programs[0].id, &[], recipe.schema(), &mut scratch)
            .unwrap();
        let [effect] = result.effects.as_slice() else {
            panic!("one template fact expected")
        };
        let EffectDisposition::Applied {
            value: ParameterValue::Boolean(enabled),
        } = &effect.disposition
        else {
            panic!("{:?}", effect.disposition)
        };
        counts[usize::from(*enabled)] += 1;
        match binding.source_base.as_str() {
            "Grand Spear" | "Sinister Quarterstaff" => assert!(*enabled),
            "Rattling Sceptre" | "Shrine Sceptre" | "Ashen Staff" | "Sapphire Ring" => {
                assert!(!*enabled)
            }
            _ => {}
        }
    }
    assert_eq!(counts, [1419, 337]);
    for case in 1..=5 {
        let draft_path = cwd.join(format!("original-{case}"));
        let status = success(normalize(cwd, &out, case, &draft_path, true));
        assert_eq!(status["normalization_status"], "pending");
        let draft = decode_draft(
            &fs::read(draft_path.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        assert!(
            draft
                .input()
                .allocations
                .members
                .iter()
                .all(|a| matches!(a.access, DraftAllocationAccess::Pending(_)))
        );
        // Broad identity recognition does not claim complete item mechanics.
        let known = draft
            .input()
            .items
            .members
            .iter()
            .filter(|i| matches!(i.template, DraftField::Known { .. }))
            .count();
        assert_eq!(known, [13, 28, 16, 19, 25][case - 1], "original {case}");
        let expected_base = [
            "Rattling Sceptre",
            "Grand Spear",
            "Sinister Quarterstaff",
            "Ashen Staff",
            "Ashen Staff",
        ][case - 1];
        let expected_template = &policy_data
            .templates
            .iter()
            .find(|b| b.source_base == expected_base)
            .unwrap()
            .template;
        assert!(draft.input().items.members.iter().any(
            |i| matches!(&i.template, DraftField::Known{value} if value == expected_template)
        ));
        assert!(
            draft
                .input()
                .items
                .members
                .iter()
                .all(|i| i.to_resolved().is_none())
        );
        let name = format!("queries-original-{case:02}.json");
        assert_eq!(fs::read(out.join(&name)).unwrap(), before[&name]);
    }
    let rerun = cwd.join("rerun");
    success(run(cwd, &out, &catalog, &policy, &definitions, &rerun));
    for (name, bytes) in bundle(&out) {
        if !["transition.json", "catalog-append.json"].contains(&name.as_str()) {
            assert_eq!(bytes, fs::read(rerun.join(&name)).unwrap(), "{name}");
        }
    }
    assert_eq!(bundle(&prior), before);
    let occupied = cwd.join("occupied");
    fs::create_dir(&occupied).unwrap();
    fs::write(occupied.join("keep"), b"caller").unwrap();
    assert!(
        !run(cwd, &out, &catalog, &policy, &definitions, &occupied)
            .status
            .success()
    );
    assert_eq!(fs::read(occupied.join("keep")).unwrap(), b"caller");
    // Neither changed artifact bytes nor misplaced/changed typed IDs can publish.
    let rejected = cwd.join("rejected");
    let changed_catalog = cwd.join("changed-catalog.json");
    let mut changed = fs::read(&catalog).unwrap();
    changed.push(b' ');
    fs::write(&changed_catalog, changed).unwrap();
    assert!(
        !run(
            cwd,
            &prior,
            &changed_catalog,
            &policy,
            &definitions,
            &rejected
        )
        .status
        .success()
    );
    assert!(!rejected.exists());
    let changed_definitions = cwd.join("changed-definitions.json");
    let mut changed = json(&definitions);
    changed[0]["value"]["schema"]["value"]["targets"] = serde_json::json!(["actor"]);
    fs::write(&changed_definitions, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(
        !run(
            cwd,
            &prior,
            &catalog,
            &policy,
            &changed_definitions,
            &rejected
        )
        .status
        .success()
    );
    assert!(!rejected.exists());
}
#[cfg(feature = "pob")]
#[test]
fn optional_export_reproduces_catalog_and_does_not_replace_output() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("exported");
    let call = || {
        command(temp.path(), "export-owned-item-bases")
            .arg("--source-root")
            .arg(root().join("vendor/path-of-building-poe2"))
            .arg("--output")
            .arg(&output)
            .output()
            .unwrap()
    };
    let report = success(call());
    assert_eq!(report["bases"], 1756);
    assert_eq!(report["table_weapon_fields"], 337);
    assert_eq!(
        fs::read(output.join("catalog.json")).unwrap(),
        fs::read(data().join("item-bases/catalog.json")).unwrap()
    );
    let before = bundle(&output);
    assert!(!call().status.success());
    assert_eq!(bundle(&output), before);
}
