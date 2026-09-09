//! Public-runtime regression: database construction must finish before item import.
//! Run Lua in a dedicated process because the source runtime uses process cwd.
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

fn source_root() -> PathBuf {
    project_root().join("vendor/path-of-building-poe2")
}

fn caller_build() -> String {
    let fixture = include_str!("../../../tests/fixtures/calibration/spark-bossing.xml");
    let empty_items = r#"<Items activeItemSet="1"><ItemSet id="1" title="No equipment"/></Items>"#;
    assert_eq!(fixture.matches(empty_items).count(), 1);
    // The unique prototype requires 49, while Gargantuan Mana Flask requires 40.
    // The first caller item deliberately has no authored requirement to mask a
    // database miss. The second verifies that an explicit higher level survives.
    fixture.replacen(
        empty_items,
        r#"<Items activeItemSet="1">
    <Item id="1"><![CDATA[Rarity: UNIQUE
Lavianga's Spirits
Gargantuan Mana Flask]]></Item>
    <Item id="2"><![CDATA[Rarity: UNIQUE
Lavianga's Spirits
Gargantuan Mana Flask
LevelReq: 60]]></Item>
    <ItemSet id="1" title="Unique initialization"/>
  </Items>"#,
        1,
    )
}

#[test]
fn unique_database_is_ready_before_public_runtime_imports_caller_items() {
    let scratch = tempfile::tempdir().unwrap();
    let output = scratch.path().join("export.xml");
    let stdout = scratch.path().join("stdout.txt");
    let stderr = scratch.path().join("stderr.txt");
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "unique_initialization_worker",
            "--ignored",
            "--nocapture",
        ])
        .current_dir(source_root().join("src"))
        .env("POE_UNIQUE_INIT_TEST_OUTPUT", &output)
        .env("POE_UNIQUE_INIT_TEST_SCRATCH", scratch.path())
        .stdout(Stdio::from(fs::File::create(&stdout).unwrap()))
        .stderr(Stdio::from(fs::File::create(&stderr).unwrap()));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(90);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("unique initialization child exceeded 90 seconds");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(
        status.success(),
        "stdout:\n{}\nstderr:\n{}",
        fs::read_to_string(&stdout).unwrap(),
        fs::read_to_string(&stderr).unwrap()
    );
    assert!(fs::metadata(&output).unwrap().len() <= poe_optimizer_core::MAX_WIRE_BYTES as u64);
    let exported = fs::read_to_string(output).unwrap();
    let document = roxmltree::Document::parse(&exported).unwrap();
    let items: Vec<_> = document
        .descendants()
        .filter(|node| node.has_tag_name("Item"))
        .collect();
    assert_eq!(items.len(), 2, "both caller inventory items must survive");
    for (id, expected_level) in [("1", "49"), ("2", "60")] {
        let item = items
            .iter()
            .find(|node| node.attribute("id") == Some(id))
            .unwrap();
        let text = item.text().unwrap();
        assert!(text.contains("Lavianga's Spirits"), "{text}");
        assert!(text.contains("Gargantuan Mana Flask"), "{text}");
        let levels: Vec<_> = text
            .lines()
            .filter_map(|line| line.trim().strip_prefix("LevelReq: "))
            .collect();
        assert_eq!(levels, [expected_level], "caller item {id}: {text}");
    }
}

#[test]
#[ignore = "fresh runtime worker invoked by the parent regression"]
fn unique_initialization_worker() {
    let output = PathBuf::from(std::env::var_os("POE_UNIQUE_INIT_TEST_OUTPUT").unwrap());
    let scratch = PathBuf::from(std::env::var_os("POE_UNIQUE_INIT_TEST_SCRATCH").unwrap());
    let snapshot =
        poe_optimizer_pob::runtime::evaluate(&source_root(), &scratch, &caller_build()).unwrap();
    fs::write(output, snapshot.export_xml).unwrap();
}
