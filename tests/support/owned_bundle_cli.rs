//! Shared explicit-path CLI setup; no source checkout or implicit data default.
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
pub(crate) fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub(crate) fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68")
}
pub(crate) fn json(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
pub(crate) fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
pub(crate) fn publish(cwd: &Path, mapping: &Path, output: &Path) -> Output {
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
pub(crate) fn normalize(
    cwd: &Path,
    bundle: &Path,
    case: usize,
    output: &Path,
    tree: bool,
) -> Output {
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
    if tree {
        command
            .arg("--tree-policy")
            .arg(bundle.join("tree-normalization.json"));
    }
    command
        .arg("--queries")
        .arg(bundle.join(format!("queries-original-{case:02}.json")))
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
pub(crate) fn bundle(path: &Path) -> BTreeMap<String, Vec<u8>> {
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
