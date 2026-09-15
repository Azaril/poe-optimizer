//! A checked single bundle feeds the existing normalizer with no runtime rebind.
use poe_optimizer_core::{
    owned_definitions::ItemTemplateDefId,
    owned_draft::{DraftField, DraftLimits, decode_draft},
};
use serde_json::Value;
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
fn json(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn publish(cwd: &Path, mapping: &Path, output: &Path) -> Output {
    let data = data();
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command
        .current_dir(cwd)
        .arg("publish-owned-successor")
        .arg(data.join("import/compiled/recipe.json"))
        .arg("--successor")
        .arg(data.join("resistance/recipe.json"))
        .arg("--mapping")
        .arg(mapping)
        .arg("--roles")
        .arg(data.join("import/compiled/roles.json"))
        .arg("--normalization")
        .arg(data.join("import/policies/normalization.json"))
        .arg("--rewards")
        .arg(data.join("import/policies/rewards.json"))
        .arg("--items")
        .arg(data.join("resistance/items.json"))
        .arg("--item-source")
        .arg(data.join("resistance/item-source.json"));
    for i in 1..=5 {
        command.arg("--query-set").arg(format!(
            "original-{i:02}={}",
            data.join(format!("import/queries/original-{i:02}.json"))
                .display()
        ));
    }
    command.arg("--output").arg(output).output().unwrap()
}
fn normalize(cwd: &Path, bundle: &Path, case: usize, output: &Path) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command
        .current_dir(cwd)
        .arg("normalize-owned")
        .arg(root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
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
    ] {
        command.arg(flag).arg(bundle.join(file));
    }
    command
        .arg("--queries")
        .arg(bundle.join(format!("queries-original-{case:02}.json")))
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
fn bundle(path: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(path)
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
#[test]
fn checked_publication_preserves_all_five_inputs_and_adds_real_item_conversion() {
    let temp = tempfile::tempdir().unwrap();
    let published = temp.path().join("current");
    let transition = success(publish(
        temp.path(),
        &data().join("import/compiled/mapping.json"),
        &published,
    ));
    assert_eq!(transition["calculation"], "not_run");
    assert_eq!(transition["whole_build_parity"], "not_established");
    assert_eq!(transition["query_rows"], 110);
    assert_eq!(json(published.join("transition.json")), transition);
    assert_eq!(fs::read_dir(&published).unwrap().count(), 18);
    let mut shipped = bundle(&data().join("current"));
    assert!(shipped.remove("README.md").is_some());
    assert_eq!(
        bundle(&published),
        shipped,
        "committed current artifacts are stale"
    );
    assert_eq!(
        json(published.join("recipe.json")),
        json(data().join("resistance/recipe.json"))
    );
    let sapphire: ItemTemplateDefId = serde_json::from_value(
        json(data().join("resistance/ids.json"))["allocations"]["sapphire-ring-template"].clone(),
    )
    .unwrap();
    let expected_items = [16, 34, 17, 21, 28];
    let expected_gems = [52, 153, 57, 59, 157];
    let expected_rewards = [16, 17, 15, 16, 17];
    let mut query_count = 0;
    for case in 1..=5 {
        let output = temp.path().join(format!("normalized-{case}"));
        let report = success(normalize(temp.path(), &published, case, &output));
        assert_eq!(report["counts"]["items"], expected_items[case - 1]);
        assert_eq!(report["counts"]["gems"], expected_gems[case - 1]);
        assert_eq!(report["counts"]["rewards"], expected_rewards[case - 1]);
        assert_eq!(
            report["source"]["occurrences"],
            report["source"]["origin_rows"]
        );
        assert_eq!(report["normalization_status"], "pending");
        assert_eq!(report["verification"]["artifact_bindings"], "checked");
        assert_eq!(report["verification"]["calculation"], "not_run");
        let sidecar = json(output.join("sidecar.json"));
        for (field, binding) in [
            ("registry", "registry"),
            ("definitions", "definitions"),
            ("mapping", "mapping"),
            ("skill_roles", "roles"),
            ("reward_policy", "rewards"),
        ] {
            assert_eq!(sidecar[field], transition["after"][binding], "{field}");
        }
        let draft = decode_draft(
            &fs::read(output.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let rows = &draft.input().query_presets.members[0]
            .queries
            .requests
            .members;
        let expected = json(published.join(format!("queries-original-{case:02}.json")));
        assert_eq!(rows.len(), expected.as_array().unwrap().len());
        for (row, expected) in rows.iter().zip(expected.as_array().unwrap()) {
            assert_eq!(serde_json::to_value(&row.id).unwrap(), expected["id"]);
        }
        query_count += rows.len();
        if case == 5 {
            let rings: Vec<_> = draft
                .input()
                .items
                .members
                .iter()
                .filter(
                    |item| matches!(&item.template,DraftField::Known{value} if value==&sapphire),
                )
                .collect();
            assert!(!rings.is_empty());
            let ring = rings
                .iter()
                .find(|item| item.modifiers.members.len() == 2)
                .unwrap();
            assert_eq!(ring.modifiers.members[0].rolls.members.len(), 22);
            assert_eq!(
                draft
                    .input()
                    .equipment
                    .members
                    .iter()
                    .filter(|use_| matches!(&use_.item,DraftField::Known{value} if *value==ring.id))
                    .count(),
                8
            );
            assert_eq!(ring.parameters.members.len(), 2);
            assert!(
                matches!(&ring.modifier_order, DraftField::Pending(_)),
                "source execution order has not been converted"
            );
        }
    }
    assert_eq!(query_count, 110);
}
#[test]
fn stale_bindings_and_existing_destinations_never_publish_or_clobber() {
    let temp = tempfile::tempdir().unwrap();
    let mapping = data().join("import/compiled/mapping.json");
    let mut stale = json(&mapping);
    stale["definitions"] =
        json(data().join("resistance/recipe.json"))["rules"]["definitions"].clone();
    let bad = temp.path().join("stale.json");
    fs::write(&bad, serde_json::to_vec(&stale).unwrap()).unwrap();
    let output = temp.path().join("not-created");
    assert!(!publish(temp.path(), &bad, &output).status.success());
    assert!(!output.exists());
    let existing = temp.path().join("existing");
    fs::create_dir(&existing).unwrap();
    fs::write(existing.join("sentinel"), b"caller-owned").unwrap();
    let before = bundle(&existing);
    assert!(!publish(temp.path(), &mapping, &existing).status.success());
    assert_eq!(bundle(&existing), before);
    assert_eq!(
        fs::read_dir(temp.path()).unwrap().count(),
        2,
        "no staging output retained"
    );
}
