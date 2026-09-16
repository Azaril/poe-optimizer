//! Offline package publication and native class-component evidence.
#[allow(dead_code)]
#[path = "support/owned_bundle_cli.rs"]
mod support;
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_draft::{DraftAllocationAccess, DraftLimits, decode_draft},
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition};
use poe_optimizer_import::{
    owned_intrinsic_attack::IntrinsicAttackPolicy, owned_recipe::assemble_owned_recipe,
    owned_tree_policy::TreeNormalizationPackageInput,
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
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
struct Fixture {
    temp: tempfile::TempDir,
    prior: PathBuf,
    catalog: PathBuf,
    policy: PathBuf,
    definitions: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let mut prior = data().join("current");
        for (name, folder) in [
            ("compile-owned-attributes", "attributes"),
            ("compile-owned-passive-views", "passive-views"),
        ] {
            let output = temp.path().join(folder);
            success(
                command(temp.path(), name)
                    .arg(&prior)
                    .arg("--catalog")
                    .arg(data().join("tree/tree-catalog.json"))
                    .arg("--policy")
                    .arg(data().join(folder).join("policy.json"))
                    .arg("--statistics")
                    .arg(data().join(folder).join("statistics.json"))
                    .arg("--output")
                    .arg(&output)
                    .output()
                    .unwrap(),
            );
            prior = output;
        }
        let classes = temp.path().join("classes");
        success(
            command(temp.path(), "compile-owned-class-bases")
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
        let copy = |file: &str| {
            let p = temp.path().join(file);
            fs::copy(data().join("intrinsic-attack").join(file), &p).unwrap();
            p
        };
        let catalog = copy("catalog.json");
        let policy = copy("policy.json");
        let definitions = copy("definitions.json");
        Self {
            temp,
            prior: classes,
            catalog,
            policy,
            definitions,
        }
    }
    fn run(&self, input: &Path, out: &Path) -> Output {
        command(self.temp.path(), "compile-owned-intrinsic-attack")
            .arg(input)
            .arg("--catalog")
            .arg(&self.catalog)
            .arg("--policy")
            .arg(&self.policy)
            .arg("--definitions")
            .arg(&self.definitions)
            .arg("--output")
            .arg(out)
            .output()
            .unwrap()
    }
}
#[test]
fn native_catalog_compilation_preserves_originals_and_adds_injected_intrinsic_components() {
    let f = Fixture::new();
    let before = bundle(&f.prior);
    let out = f.temp.path().join("intrinsic");
    let report = success(f.run(&f.prior, &out));
    assert_eq!(report["intrinsic_attack"]["converted_classes"], 8);
    assert_eq!(report["intrinsic_attack"]["changed_program_owners"], 8);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    let old: Value = serde_json::from_slice(&before["registry.json"]).unwrap();
    let new = json(out.join("registry.json"));
    assert_eq!(new["last_issued"], 7482);
    assert_eq!(
        &new["entries"].as_array().unwrap()[..7476],
        old["entries"].as_array().unwrap()
    );
    assert_eq!(json(out.join("routing.json"))["schema_version"], 1);
    assert_eq!(
        json(out.join("rules.json"))["operations_version"],
        "owned-domain-operations-v7"
    );
    let recipe = assemble_owned_recipe(
        serde_json::from_value(json(out.join("recipe.json"))).unwrap(),
        Default::default(),
    )
    .unwrap();
    let compiled =
        CompiledRulePackage::compile(recipe.rules().input(), recipe.schema(), Default::default())
            .unwrap();
    let policy: IntrinsicAttackPolicy = serde_json::from_value(json(&f.policy)).unwrap();
    let tree: TreeNormalizationPackageInput =
        serde_json::from_value(json(out.join("tree-normalization.json"))).unwrap();
    let expected_max: BTreeMap<_, _> = [
        ("1", 5.0),
        ("2", 5.0),
        ("6", 8.0),
        ("7", 5.0),
        ("8", 5.0),
        ("9", 6.0),
        ("10", 5.0),
        ("11", 6.0),
    ]
    .into_iter()
    .collect();
    for class in &tree.content.classes {
        let owner = SchemaSubject::Definition(class.class.address());
        let row = recipe
            .rules()
            .input()
            .owners
            .iter()
            .find(|r| r.owner == owner)
            .unwrap();
        assert!(!row.programs.is_complete());
        assert_eq!(row.programs.members.len(), 2);
        let program=row.programs.members.iter().find(|p|p.effects.iter().any(|e|matches!(&e.effect,RuleEffectKind::Derive{stat,..} if stat == &policy.fields[0].stat))).unwrap();
        let evaluated = compiled
            .evaluate(
                &owner,
                &program.id,
                &[],
                recipe.schema(),
                &mut compiled.new_scratch(),
            )
            .unwrap();
        assert_eq!(evaluated.effects.len(), 4);
        for field in &policy.fields {
            let effect=evaluated.effects.iter().find(|e|matches!(&e.effect,RuleEffectKind::Derive{entity:RuleEntity::Player,stat,..} if stat==&field.stat)).unwrap();
            let EffectDisposition::Applied {
                value: ParameterValue::Quantity(q),
            } = &effect.disposition
            else {
                panic!("{:?}", effect.disposition)
            };
            let expected = match field.source_field.as_str() {
                "AttackRate" => 1.65,
                "CritChance" => 5.0,
                "PhysicalMin" => 2.0,
                "PhysicalMax" => expected_max[class.key.as_str()],
                _ => panic!("unexpected field"),
            };
            assert_eq!(q.value(), expected);
            assert_eq!(q.unit(), &field.unit);
        }
    }
    assert_eq!(tree.content.classes.len(), 8);
    for case in 1..=5 {
        let draft_path = f.temp.path().join(format!("original-{case}"));
        let status = success(normalize(f.temp.path(), &out, case, &draft_path, true));
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
        let name = format!("queries-original-{case:02}.json");
        assert_eq!(fs::read(out.join(&name)).unwrap(), before[&name]);
    }
    let rerun = f.temp.path().join("rerun");
    let again = success(f.run(&out, &rerun));
    assert_eq!(again["intrinsic_attack"]["changed_program_owners"], 0);
    for (name, bytes) in bundle(&out) {
        if name != "transition.json" {
            assert_eq!(bytes, fs::read(rerun.join(&name)).unwrap(), "{name}");
        }
    }
    assert_eq!(bundle(&f.prior), before);
    let occupied = f.temp.path().join("occupied");
    fs::create_dir(&occupied).unwrap();
    fs::write(occupied.join("keep"), b"caller").unwrap();
    assert!(!f.run(&out, &occupied).status.success());
    assert_eq!(fs::read(occupied.join("keep")).unwrap(), b"caller");
}
#[test]
fn artifact_tamper_and_invalid_appended_descriptors_cannot_publish() {
    let f = Fixture::new();
    let out = f.temp.path().join("rejected");
    let original = fs::read(&f.catalog).unwrap();
    let mut changed = original.clone();
    changed.push(b' ');
    fs::write(&f.catalog, changed).unwrap();
    assert!(!f.run(&f.prior, &out).status.success());
    assert!(!out.exists());
    fs::write(&f.catalog, original).unwrap();
    let defs = json(&f.definitions);
    let mut wrong = defs.clone();
    wrong[0]["value"]["schema"]["value"]["targets"] = serde_json::json!(["equipment_use"]);
    fs::write(&f.definitions, serde_json::to_vec(&wrong).unwrap()).unwrap();
    assert!(!f.run(&f.prior, &out).status.success());
    assert!(!out.exists());
    let mut wrong = defs.clone();
    wrong.as_array_mut().unwrap().reverse();
    fs::write(&f.definitions, serde_json::to_vec(&wrong).unwrap()).unwrap();
    assert!(!f.run(&f.prior, &out).status.success());
    assert!(!out.exists());
    let mut wrong = defs.clone();
    wrong[4]["value"]["id"]["key"] = serde_json::json!("def.000000000000ffff");
    fs::write(&f.definitions, serde_json::to_vec(&wrong).unwrap()).unwrap();
    assert!(!f.run(&f.prior, &out).status.success());
    assert!(!out.exists());
    fs::write(&f.definitions, serde_json::to_vec(&defs).unwrap()).unwrap();
    let mut policy = json(&f.policy);
    policy["constants"]["type"] = serde_json::json!("Sword");
    fs::write(&f.policy, serde_json::to_vec(&policy).unwrap()).unwrap();
    assert!(!f.run(&f.prior, &out).status.success());
    assert!(!out.exists());
}
#[cfg(feature = "pob")]
#[test]
fn optional_export_reproduces_checked_catalog_without_runtime_ui() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("exported");
    let call = || {
        command(temp.path(), "export-owned-intrinsic-attack")
            .arg("--data-lua")
            .arg(root().join("vendor/path-of-building-poe2/src/Modules/Data.lua"))
            .arg("--misc-lua")
            .arg(root().join("vendor/path-of-building-poe2/src/Data/Misc.lua"))
            .arg("--source-pin")
            .arg(data().join("intrinsic-attack/source-pin.json"))
            .arg("--output")
            .arg(&output)
            .output()
            .unwrap()
    };
    let report = success(call());
    assert_eq!(report["classes"], 9);
    assert_eq!(
        fs::read(output.join("catalog.json")).unwrap(),
        fs::read(data().join("intrinsic-attack/catalog.json")).unwrap()
    );
    assert_eq!(
        report["catalog_sha256"],
        json(data().join("intrinsic-attack/policy.json"))["catalog_sha256"]
    );
    let before = bundle(&output);
    assert!(!call().status.success());
    assert_eq!(bundle(&output), before);
}
