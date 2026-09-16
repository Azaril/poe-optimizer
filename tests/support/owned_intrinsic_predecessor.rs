//! Shared setup used only by equipment publication integration tests.
use super::support::{data, root, success};
use std::{
    path::{Path, PathBuf},
    process::Command,
};
pub(crate) fn intrinsic_predecessor(cwd: &Path) -> PathBuf {
    let mut prior = data().join("current");
    for (name, folder) in [
        ("compile-owned-attributes", "attributes"),
        ("compile-owned-passive-views", "passive-views"),
    ] {
        let out = cwd.join(folder);
        success(
            cargo_command(cwd, name)
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
        cargo_command(cwd, "compile-owned-class-bases")
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
        cargo_command(cwd, "compile-owned-intrinsic-attack")
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

fn cargo_command(cwd: &Path, name: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command.current_dir(cwd).arg(name);
    command
}
