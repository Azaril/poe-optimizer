//! Shared optional reference driver and physical occurrence proof. Family
//! semantics and report assertions stay in their narrow source witnesses.
use super::magnified_area_support::check_saved_origin;
use super::*;

pub(super) struct Witness {
    pub name: &'static str,
    pub child_env: &'static str,
    pub output_env: &'static str,
    pub default_output: &'static str,
    pub label: &'static str,
}

pub(super) fn run_modes(witness: Witness, child: fn(&Path, &Path, bool)) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join(
        std::env::var_os(witness.output_env)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(witness.default_output)),
    );
    if let Some(mode) = std::env::var_os(witness.child_env) {
        assert!(mode == "off" || mode == "on");
        child(&root, &out, mode == "on");
        return;
    }
    assert!(
        std::env::var_os(CHILD).is_none(),
        "unset historical child selector"
    );
    assert!(
        !out.exists(),
        "immutable source evidence exists; choose fresh {}",
        witness.output_env
    );
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", witness.name, "--ignored", "--nocapture"])
            .env(witness.child_env, mode)
            .env(witness.output_env, &out)
            .env_remove(CHILD)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let started = Instant::now();
        loop {
            if let Some(status) = process.try_wait().unwrap() {
                assert!(status.success(), "{}\n{}", path.display(), tail(&path));
                break;
            }
            if started.elapsed() > Duration::from_secs(600) {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!(
                    "{} source deadline: {}\n{}",
                    witness.label,
                    path.display(),
                    tail(&path)
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    json_evidence::assert_files_equal(
        &out.join("source-jit-off.json"),
        &out.join("source-jit-on.json"),
        &format!("{} source JIT parity", witness.label),
    );
}

pub(super) fn observe_physical(
    root: &Path,
    name: &str,
    xml: &str,
    enabled: bool,
    control: Option<Json>,
    delivery: &str,
) -> Json {
    let mut observed = observe_with_extra(root, name, xml, enabled, control, Some(delivery));
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([103; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    assert_eq!(
        observed["source_identity"],
        serde_json::to_value(evidence.identity()).unwrap()
    );
    for stage in STAGES {
        for context in rows(&observed["states"][stage]["delivery"]["contexts"]) {
            let binding = &context["group"];
            let ordinal = binding["source_ordinal"].as_u64().unwrap();
            let group = evidence
                .rows()
                .get(usize::try_from(ordinal).unwrap())
                .unwrap();
            assert_eq!(u64::from(group.occurrence().id().ordinal()), ordinal);
            assert_eq!(group.occurrence().name(), "Skill");
            assert!(group.attribute("source").is_none());
            assert_eq!(binding["source_present"], false);
            let set = evidence.row(group.occurrence().parent().unwrap()).unwrap();
            assert_eq!(set.occurrence().name(), "SkillSet");
            assert_eq!(
                set.attribute("id")
                    .unwrap()
                    .decoded()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap(),
                binding["preset"].as_u64().unwrap()
            );
            check_saved_origin(&context["source"], group, &evidence);
            for candidate in rows(&context["candidates"]) {
                check_saved_origin(&candidate["origin"], group, &evidence);
            }
        }
    }
    observed["independent_source_bindings_verified"] = json!(true);
    observed
}
