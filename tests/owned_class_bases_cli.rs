//! Class source conversion stays offline; unresolved game rules remain explicit.
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_draft::{DraftAllocationAccess, DraftLimits, decode_draft},
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition};
use poe_optimizer_import::{
    owned_recipe::assemble_owned_recipe, owned_tree_policy::TreeNormalizationPackageInput,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68")
}
fn read(p: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn success(o: Output) -> Value {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}
fn files(p: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(p)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().into_string().unwrap(),
                fs::read(e.path()).unwrap(),
            )
        })
        .collect()
}
fn command(cwd: &Path, name: &str) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    c.current_dir(cwd).arg(name);
    c
}
struct Fixture {
    temp: tempfile::TempDir,
    prior: PathBuf,
    source: PathBuf,
    policy: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let attr = temp.path().join("attributes");
        let prior = temp.path().join("views");
        for (name, input, folder, output) in [
            (
                "compile-owned-attributes",
                data().join("current"),
                "attributes",
                &attr,
            ),
            (
                "compile-owned-passive-views",
                attr.clone(),
                "passive-views",
                &prior,
            ),
        ] {
            success(
                command(temp.path(), name)
                    .arg(input)
                    .arg("--catalog")
                    .arg(data().join("tree/tree-catalog.json"))
                    .arg("--policy")
                    .arg(data().join(folder).join("policy.json"))
                    .arg("--statistics")
                    .arg(data().join(folder).join("statistics.json"))
                    .arg("--output")
                    .arg(output)
                    .output()
                    .unwrap(),
            );
        }
        let source = temp.path().join("source.json");
        fs::copy(
            root().join("vendor/path-of-building-poe2/src/TreeData/0_5/tree.json"),
            &source,
        )
        .unwrap();
        let policy = temp.path().join("policy.json");
        fs::copy(data().join("class-bases/policy.json"), &policy).unwrap();
        Self {
            temp,
            prior,
            source,
            policy,
        }
    }
    fn run(&self, input: &Path, out: &Path) -> Output {
        command(self.temp.path(), "compile-owned-class-bases")
            .arg(input)
            .arg("--source-tree")
            .arg(&self.source)
            .arg("--policy")
            .arg(&self.policy)
            .arg("--output")
            .arg(out)
            .output()
            .unwrap()
    }
}
#[test]
fn all_classes_publish_base_components_without_claiming_complete_game_rules() {
    let f = Fixture::new();
    let before = files(&f.prior);
    let out = f.temp.path().join("classes");
    let report = success(f.run(&f.prior, &out));
    assert_eq!(report["class_bases"]["converted_classes"], 8);
    assert_eq!(report["class_bases"]["refined_classes"], 8);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    assert_eq!(
        read(out.join("registry.json")),
        read(f.prior.join("registry.json"))
    );
    assert_eq!(
        read(out.join("rules.json"))["operations_version"],
        "owned-domain-operations-v7"
    );
    assert_eq!(
        report["publication"]["schema_refinement"]["schema_version"],
        2
    );
    assert_eq!(
        report["publication"]["schema_refinement"]["owners"]
            .as_array()
            .unwrap()
            .len(),
        8
    );
    let recipe = assemble_owned_recipe(
        serde_json::from_value(read(out.join("recipe.json"))).unwrap(),
        Default::default(),
    )
    .unwrap();
    let compiled =
        CompiledRulePackage::compile(recipe.rules().input(), recipe.schema(), Default::default())
            .unwrap();
    let tree: TreeNormalizationPackageInput =
        serde_json::from_value(read(out.join("tree-normalization.json"))).unwrap();
    let raw = read(&f.source);
    let by_key: BTreeMap<_, _> = raw["classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["integerId"].as_u64().unwrap().to_string(), c))
        .collect();
    let mut count = 0;
    for class in &tree.content.classes {
        let owner = SchemaSubject::Definition(class.class.address());
        let row = recipe
            .rules()
            .input()
            .owners
            .iter()
            .find(|r| r.owner == owner)
            .unwrap();
        assert!(
            !row.programs.is_complete(),
            "unarmed and other intrinsic effects remain unresolved"
        );
        assert_eq!(row.programs.members.len(), 1);
        let result = compiled
            .evaluate(
                &owner,
                &row.programs.members[0].id,
                &[],
                recipe.schema(),
                &mut compiled.new_scratch(),
            )
            .unwrap();
        let values: BTreeMap<_, _> = result
            .effects
            .iter()
            .map(|e| {
                let RuleEffectKind::Contribute {
                    entity,
                    stat,
                    contribution,
                    ..
                } = &e.effect
                else {
                    panic!("class base contribution")
                };
                assert_eq!(*entity, RuleEntity::Player);
                assert_eq!(*contribution, ContributionKind::Add);
                let value = match &e.disposition {
                    EffectDisposition::Applied {
                        value: ParameterValue::Integer(n),
                    } => n.get(),
                    other => panic!("{other:?}"),
                };
                (stat.clone(), value)
            })
            .collect();
        let source = by_key[&class.key];
        let policy: poe_optimizer_import::owned_class_bases::ClassBaseRecipePolicy =
            serde_json::from_value(read(&f.policy)).unwrap();
        for field in policy.fields {
            assert_eq!(
                values[&field.stat],
                source[&field.source_field].as_i64().unwrap()
            );
        }
        assert_eq!(values.values().sum::<i64>(), 29);
        count += 1;
    }
    assert_eq!(count, 8);
    for i in 1..=5 {
        let normalized = f.temp.path().join(format!("original-{i}"));
        let mut cmd = command(f.temp.path(), "normalize-owned");
        cmd.arg(root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{i:02}.xml"
        )));
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
            cmd.arg(flag).arg(out.join(file));
        }
        cmd.arg("--queries")
            .arg(out.join(format!("queries-original-{i:02}.json")))
            .arg("--output")
            .arg(&normalized);
        let status = success(cmd.output().unwrap());
        assert_eq!(status["normalization_status"], "pending");
        assert_eq!(status["verification"]["calculation"], "not_run");
        let draft = decode_draft(
            &fs::read(normalized.join("draft.json")).unwrap(),
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
        for preset in &draft.input().character_presets.members {
            let class = preset.class.to_resolved().unwrap();
            assert!(tree.content.classes.iter().any(|c| c.class == class));
        }
        let q = format!("queries-original-{i:02}.json");
        assert_eq!(fs::read(out.join(&q)).unwrap(), before[&q]);
    }
    let rerun = f.temp.path().join("rerun");
    let again = success(f.run(&out, &rerun));
    assert_eq!(again["class_bases"]["refined_classes"], 0);
    assert_eq!(again["class_bases"]["changed_program_owners"], 0);
    for (name, bytes) in files(&out) {
        if name != "transition.json" {
            assert_eq!(bytes, fs::read(rerun.join(&name)).unwrap(), "{name}");
        }
    }
    assert_eq!(before, files(&f.prior));
    let occupied = f.temp.path().join("occupied");
    fs::create_dir(&occupied).unwrap();
    fs::write(occupied.join("keep"), b"caller").unwrap();
    assert!(!f.run(&out, &occupied).status.success());
    assert_eq!(fs::read(occupied.join("keep")).unwrap(), b"caller");
}
#[test]
fn source_pin_policy_and_bounds_fail_without_publication() {
    let f = Fixture::new();
    let original = fs::read(&f.source).unwrap();
    let out = f.temp.path().join("rejected");
    let mut changed = original.clone();
    changed.push(b' ');
    fs::write(&f.source, changed).unwrap();
    assert!(!f.run(&f.prior, &out).status.success());
    assert!(!out.exists());
    fs::write(&f.source, original).unwrap();
    let original = read(&f.policy);
    let mut wrong = original.clone();
    wrong["expected_class_fields"] = json!(["integerId"]);
    fs::write(&f.policy, serde_json::to_vec(&wrong).unwrap()).unwrap();
    assert!(!f.run(&f.prior, &out).status.success());
    assert!(!out.exists());
    let mut wrong = original.clone();
    wrong["fields"][0]["stat"]["key"] = json!("def.0000000000000002");
    fs::write(&f.policy, serde_json::to_vec(&wrong).unwrap()).unwrap();
    assert!(!f.run(&f.prior, &out).status.success());
    assert!(!out.exists());
    fs::write(&f.policy, serde_json::to_vec(&original).unwrap()).unwrap();
    fs::File::create(&f.source)
        .unwrap()
        .set_len(64 * 1024 * 1024 + 1)
        .unwrap();
    assert!(!f.run(&f.prior, &out).status.success());
    assert!(!out.exists());
}
