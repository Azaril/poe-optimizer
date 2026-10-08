//! Exercise CI selection and exit handling without running Cargo recursively.
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn script(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".github/scripts")
        .join(name)
}
fn pwsh() -> Command {
    let mut command = Command::new("pwsh");
    command.args(["-NoLogo", "-NoProfile", "-NonInteractive"]);
    command
}
fn output_text(output: &Output) -> String {
    format!(
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
fn ps_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
fn build_dependency() -> Value {
    json!({
        "kind":["custom-build"], "crate_types":["bin"], "name":"build-script-build",
        "src_path":"fixture/build.rs", "edition":"2024",
        "doc":false, "doctest":false, "test":false
    })
}
fn metadata() -> Value {
    let mut targets: Vec<_> = ["h", "a", "f", "c", "g", "d", "b", "e"]
        .into_iter()
        .map(|name| json!({"kind":["test"], "name":name, "test":true}))
        .collect();
    targets.push(json!({"kind":["bin"], "name":"poe-optimizer", "test":true}));
    targets.push(build_dependency());
    json!({
        "workspace_members":["cli-id"],
        "packages":[{"name":"poe-optimizer-cli", "id":"cli-id", "targets":targets}]
    })
}
fn write_metadata(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn dispatch(metadata: &Path, shard: usize, plan_only: bool, cargo: Option<&Path>) -> Output {
    let mut command = pwsh();
    command
        .arg("-File")
        .arg(script("invoke-cli-shard.ps1"))
        .arg("-MetadataPath")
        .arg(metadata)
        .arg("-Shard")
        .arg(shard.to_string());
    if plan_only {
        command.arg("-PlanOnly");
    }
    if let Some(cargo) = cargo {
        command.arg("-Cargo").arg(cargo);
    }
    command.output().expect("CI provides PowerShell on both OS")
}
fn plan(metadata: &Path, shard: usize) -> Value {
    let output = dispatch(metadata, shard, true, None);
    assert!(output.status.success(), "{}", output_text(&output));
    serde_json::from_slice(&output.stdout).unwrap()
}
fn selected(arguments: &Value) -> BTreeSet<String> {
    let arguments: Vec<_> = arguments
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap())
        .collect();
    assert_eq!(&arguments[..3], &["test", "-p", "poe-optimizer-cli"]);
    assert!(arguments.contains(&"--locked"));
    assert!(arguments.contains(&"--no-fail-fast"));
    assert!(!arguments.contains(&"--all-targets"));
    let mut targets = BTreeSet::new();
    let (pairs, remainder) = arguments[6..].as_chunks::<2>();
    assert!(remainder.is_empty());
    for pair in pairs {
        assert!(matches!(pair[0], "--bin" | "--test"));
        assert!(targets.insert(format!("{}:{}", pair[0], pair[1])));
    }
    targets
}

#[test]
fn cli_shards_cover_every_target_once_in_each_feature_profile_independent_of_source_order() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("metadata.json");
    let mut data = metadata();
    write_metadata(&path, &data);
    let plans: Vec<_> = (0..4).map(|shard| plan(&path, shard)).collect();
    let expected: BTreeSet<_> = ["a", "b", "c", "d", "e", "f", "g", "h"]
        .into_iter()
        .map(|name| format!("--test:{name}"))
        .chain(["--bin:poe-optimizer".to_owned()])
        .collect();
    let mut by_profile: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for (shard, plan) in plans.iter().enumerate() {
        assert_eq!(plan["shard"], shard);
        assert_eq!(plan["shard_count"], 4);
        assert_eq!(plan["targets"].as_array().unwrap().len(), 9);
        assert_eq!(plan["build_dependencies"], json!([build_dependency()]));
        assert_eq!(plan["commands"].as_array().unwrap().len(), 2);
        for command in plan["commands"].as_array().unwrap() {
            let profile = command["profile"].as_str().unwrap();
            assert_eq!(
                command["arguments"][3],
                match profile {
                    "all-features" => "--all-features",
                    "native-only" => "--no-default-features",
                    other => panic!("unexpected profile: {other}"),
                }
            );
            let chosen = selected(&command["arguments"]);
            assert!(
                !chosen
                    .iter()
                    .any(|name| name.contains("build-script-build"))
            );
            assert_eq!(chosen.contains("--bin:poe-optimizer"), shard == 0);
            assert_eq!(chosen.len(), if shard == 0 { 3 } else { 2 });
            let union = by_profile.entry(profile).or_default();
            assert!(union.is_disjoint(&chosen));
            union.extend(chosen);
        }
    }
    assert_eq!(by_profile.len(), 2);
    for union in by_profile.values() {
        assert_eq!(union, &expected);
    }
    data["packages"][0]["targets"]
        .as_array_mut()
        .unwrap()
        .reverse();
    write_metadata(&path, &data);
    for (shard, before) in plans.iter().enumerate() {
        assert_eq!(&plan(&path, shard), before);
    }
    // New targets enter metadata discovery without updating a maintained list.
    data["packages"][0]["targets"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":["test"], "name":"new-target", "test":true}));
    write_metadata(&path, &data);
    let updated = plan(&path, 0);
    assert_eq!(updated["targets"].as_array().unwrap().len(), 10);
    assert!(selected(&updated["commands"][0]["arguments"]).contains("--test:new-target"));
}

#[test]
fn cli_shards_refuse_ambiguous_or_unsupported_target_inventories() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("metadata.json");
    for (name, target, expected) in [
        (
            "duplicate",
            json!({"kind":["test"],"name":"a","test":true}),
            "Duplicate CLI target",
        ),
        (
            "library",
            json!({"kind":["lib"],"name":"future","test":true}),
            "Unsupported CLI target kind",
        ),
        (
            "example",
            json!({"kind":["example"],"name":"future","test":true}),
            "Unsupported CLI target kind",
        ),
        (
            "benchmark",
            json!({"kind":["bench"],"name":"future","test":true}),
            "Unsupported CLI target kind",
        ),
        (
            "required-feature",
            json!({"kind":["test"],"name":"future","test":true,"required-features":["pob"]}),
            "Unsupported CLI target required-features",
        ),
        (
            "disabled-test",
            json!({"kind":["test"],"name":"future","test":false}),
            "Unsupported CLI target test setting",
        ),
    ] {
        let mut data = metadata();
        data["packages"][0]["targets"]
            .as_array_mut()
            .unwrap()
            .push(target);
        write_metadata(&path, &data);
        let output = dispatch(&path, 0, true, None);
        assert!(!output.status.success(), "accepted {name}");
        assert!(
            output_text(&output).contains(expected),
            "{name}: {}",
            output_text(&output)
        );
    }
    for (multiple, expected) in [
        (true, "Multiple CLI custom-build targets"),
        (false, "Malformed CLI custom-build target"),
    ] {
        let mut data = metadata();
        let targets = data["packages"][0]["targets"].as_array_mut().unwrap();
        if multiple {
            targets.push(build_dependency());
        } else {
            targets.last_mut().unwrap()["test"] = json!(true);
        }
        write_metadata(&path, &data);
        let output = dispatch(&path, 0, true, None);
        assert!(!output.status.success());
        assert!(
            output_text(&output).contains(expected),
            "{}",
            output_text(&output)
        );
    }
}

#[test]
fn annotation_wrapper_preserves_default_exit_codes_and_opt_in_continuation() {
    let wrapper = ps_string(
        script("invoke-with-failure-annotation.ps1")
            .to_str()
            .unwrap(),
    );
    for code in [0, 7] {
        let arguments = format!(
            "@('-NoLogo','-NoProfile','-NonInteractive','-Command','Write-Output annotation-sentinel; exit {code}')"
        );
        let default = pwsh()
            .args([
                "-Command",
                // Match the CI PowerShell runner: an outer -Command otherwise
                // normalizes a called script's nonzero exit code to one.
                &format!(
                    "& {wrapper} -FilePath pwsh -ArgumentList {arguments}; exit $LASTEXITCODE"
                ),
            ])
            .output()
            .unwrap();
        assert_eq!(
            default.status.code(),
            Some(code),
            "{}",
            output_text(&default)
        );
        assert_eq!(
            output_text(&default).contains("::error title=Command failed::"),
            code != 0
        );
        let continued = pwsh()
            .args(["-Command", &format!("$code = & {wrapper} -FilePath pwsh -ArgumentList {arguments} -ReturnExitCode; if ($code -isnot [int]) {{ throw 'not one integer' }}; Write-Output ('continued:' + $code); exit 0")])
            .output()
            .unwrap();
        assert!(continued.status.success(), "{}", output_text(&continued));
        let text = output_text(&continued);
        assert_eq!(text.contains("Command exited with code 7"), code != 0);
        assert!(text.contains("annotation-sentinel"));
        assert!(text.contains(&format!("continued:{code}")));
    }
}

#[test]
fn cli_shard_runs_both_profiles_and_preserves_failure_from_either() {
    let temp = tempfile::tempdir().unwrap();
    let metadata_path = temp.path().join("metadata.json");
    write_metadata(&metadata_path, &metadata());
    for (first, second, expected) in [(7, 0, 7), (0, 9, 9), (7, 9, 7), (0, 0, 0)] {
        let log = temp
            .path()
            .join(format!("invocations-{first}-{second}.jsonl"));
        let fake = temp.path().join(format!("cargo-{first}-{second}.ps1"));
        fs::write(
            &fake,
            format!(
                "$profile = if ($args -contains '--all-features') {{ 'all-features' }} else {{ 'native-only' }}\nAdd-Content -LiteralPath {} -Value (ConvertTo-Json -Compress -InputObject ([pscustomobject]@{{ profile=$profile; arguments=@($args) }}))\nWrite-Output ('executed:' + $profile)\nif ($profile -eq 'all-features') {{ exit {first} }}\nexit {second}\n",
                ps_string(log.to_str().unwrap())
            ),
        )
        .unwrap();
        let output = dispatch(&metadata_path, 1, false, Some(&fake));
        assert_eq!(
            output.status.code(),
            Some(expected),
            "{}",
            output_text(&output)
        );
        let calls: Vec<Value> = fs::read_to_string(&log)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0]["profile"], "all-features");
        assert_eq!(calls[1]["profile"], "native-only");
        assert_eq!(
            selected(&calls[0]["arguments"]),
            selected(&calls[1]["arguments"])
        );
        for (profile, code) in [("all-features", first), ("native-only", second)] {
            assert!(output_text(&output).contains(&format!("CLI profile {profile} exited {code}")));
        }
    }
}
