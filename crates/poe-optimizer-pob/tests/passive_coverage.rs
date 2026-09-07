//! Compare independent source projection with observed fresh live PassiveSpec data.
//! The single cached matrix uses at most two evaluator child processes at a time.
use poe_optimizer_core::{
    EvaluationSnapshot,
    coverage::{PassiveCoverage, PassiveRootRole, PassiveSwitchKind},
};
use poe_optimizer_pob::tree_data::{OverrideProvenance, TreeDataSnapshot};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    sync::OnceLock,
    time::{Duration, Instant},
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}
fn source() -> PathBuf {
    root().join("vendor/path-of-building-poe2")
}
fn supervised(mode: &str, input: Option<&str>) -> Vec<u8> {
    let scratch = tempfile::tempdir().unwrap();
    let output = scratch.path().join("output.json");
    let diagnostics = scratch.path().join("stderr.txt");
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "passive_observation_worker",
            "--ignored",
            "--nocapture",
        ])
        .current_dir(source().join("src"))
        .env("POE_PASSIVE_TEST_MODE", mode)
        .env("POE_PASSIVE_TEST_OUTPUT", &output)
        .env("POE_PASSIVE_TEST_SCRATCH", scratch.path())
        .stdout(Stdio::null())
        .stderr(Stdio::from(fs::File::create(&diagnostics).unwrap()));
    if let Some(input) = input {
        let path = scratch.path().join("input.xml");
        fs::write(&path, input).unwrap();
        command.env("POE_PASSIVE_TEST_INPUT", path);
    }
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
            panic!("passive observation child exceeded 90 seconds");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(
        status.success(),
        "{}",
        fs::read_to_string(diagnostics).unwrap()
    );
    assert!(fs::metadata(&output).unwrap().len() <= poe_optimizer_core::MAX_WIRE_BYTES as u64);
    fs::read(output).unwrap()
}
fn evaluate(xml: &str) -> EvaluationSnapshot {
    serde_json::from_slice(&supervised("evaluate", Some(xml))).unwrap()
}
fn xml(
    class_id: u32,
    class_name: &str,
    asc_index: u32,
    asc_id: &str,
    asc_name: &str,
    nodes: &[u32],
    additions: &str,
) -> String {
    let allocated = nodes
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<PathOfBuilding2>
<Build level="60" className="{class_name}" ascendClassName="{asc_name}" targetVersion="0_1" characterLevelAutoMode="false" mainSocketGroup="1" viewMode="CALCS"/>
<Tree activeSpec="1"><Spec title="Observed passive fixture" classId="{class_id}" classInternalId="{class_id}" ascendClassId="{asc_index}" ascendancyInternalId="{asc_id}" treeVersion="0_5" nodes="{allocated}" masteryEffects="">{additions}</Spec></Tree>
<Skills activeSkillSet="1"><SkillSet id="1"/></Skills>
<Items activeItemSet="1"><ItemSet id="1" useSecondWeaponSet="false"/></Items>
<Config activeConfigSet="1"><ConfigSet id="1"><Input name="enemyLevel" number="60"/></ConfigSet></Config>
</PathOfBuilding2>"#
    )
}
struct Case {
    id: String,
    class_id: u32,
    ascendancy_id: Option<String>,
    paid: Vec<u32>,
    xml: String,
}
struct Observed {
    case: Case,
    result: EvaluationSnapshot,
}
struct Matrix {
    tree: TreeDataSnapshot,
    observations: Vec<Observed>,
}
fn matrix() -> &'static Matrix {
    static MATRIX: OnceLock<Matrix> = OnceLock::new();
    MATRIX.get_or_init(|| {
        let tree: TreeDataSnapshot = serde_json::from_slice(&supervised("tree", None)).unwrap();
        let mut cases = Vec::new();
        for (&class_id, class) in &tree.classes {
            cases.push(Case {
                id: format!("{class_id}/none"),
                class_id,
                ascendancy_id: None,
                paid: vec![],
                xml: xml(class_id, &class.name, 0, "", "None", &[], ""),
            });
            for asc_id in &class.ascendancy_ids {
                let asc = &tree.ascendancies[asc_id];
                cases.push(Case {
                    id: format!("{class_id}/{asc_id}"),
                    class_id,
                    ascendancy_id: Some(asc_id.clone()),
                    paid: vec![],
                    xml: xml(
                        class_id,
                        &class.name,
                        asc.class_index,
                        asc_id,
                        &asc.name,
                        &[],
                        "",
                    ),
                });
            }
        }
        assert_eq!(cases.len(), 31, "pinned class/ascendancy matrix changed");
        for (&class_id, class) in &tree.classes {
            for node in tree.ordinary_entrances(class_id).unwrap() {
                cases.push(Case {
                    id: format!("{class_id}/entrance/{node}"),
                    class_id,
                    ascendancy_id: None,
                    paid: vec![node],
                    xml: xml(class_id, &class.name, 0, "", "None", &[node], ""),
                });
            }
        }
        assert_eq!(cases.len(), 47, "pinned ordinary entrance matrix changed");
        let results = std::thread::scope(|scope| {
            let workers: Vec<_> = (0..2)
                .map(|worker| {
                    let cases = &cases;
                    scope.spawn(move || {
                        cases
                            .iter()
                            .enumerate()
                            .filter(|(index, _)| index % 2 == worker)
                            .map(|(index, case)| (index, evaluate(&case.xml)))
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            workers
                .into_iter()
                .flat_map(|worker| worker.join().unwrap())
                .collect::<BTreeMap<_, _>>()
        });
        Matrix {
            tree,
            observations: cases
                .into_iter()
                .zip(results.into_values())
                .map(|(case, result)| Observed { case, result })
                .collect(),
        }
    })
}
fn passives(result: &EvaluationSnapshot) -> &PassiveCoverage {
    let evidence = result
        .coverage
        .passives
        .as_ref()
        .expect("fresh evaluator must supply passive evidence");
    evidence.validate().unwrap();
    assert_eq!(
        evidence
            .allocated_nodes
            .iter()
            .map(|node| node.physical_node_id)
            .collect::<Vec<_>>(),
        result.build.allocated_nodes
    );
    evidence
}

#[test]
fn all_31_class_ascendancy_identities_and_16_entrances_match_fresh_live_state() {
    let matrix = matrix();
    let mut switched_classes = BTreeSet::new();
    let mut switched_ascendancies = BTreeSet::new();
    for observation in &matrix.observations {
        let case = &observation.case;
        let result = &observation.result;
        let evidence = passives(result);
        let class = &matrix.tree.classes[&case.class_id];
        assert_eq!(evidence.class.internal_id, class.integer_id, "{}", case.id);
        assert_eq!(evidence.class.index, case.class_id, "{}", case.id);
        assert_eq!(evidence.class.name, class.name, "{}", case.id);
        assert_eq!(evidence.class.start_node_id, class.start_node_id);
        assert_eq!(evidence.tree_version, "0_5");
        assert!(evidence.secondary_ascendancy.is_none());
        let mut requested = BTreeSet::from([class.start_node_id]);
        requested.extend(&case.paid);
        if let Some(id) = &case.ascendancy_id {
            let expected = &matrix.tree.ascendancies[id];
            let actual = evidence.ascendancy.as_ref().unwrap();
            assert_eq!(
                actual.internal_id.as_deref(),
                Some(id.as_str()),
                "{}",
                case.id
            );
            assert_eq!(
                actual.catalog_id.as_deref(),
                Some(expected.catalog_id.as_str())
            );
            assert_eq!(actual.index, expected.class_index);
            assert_eq!(actual.name, expected.name);
            assert_eq!(actual.start_node_id, Some(expected.start_node_id));
            requested.insert(expected.start_node_id);
        } else {
            assert!(evidence.ascendancy.is_none());
        }
        assert_eq!(
            result
                .build
                .allocated_nodes
                .iter()
                .copied()
                .collect::<BTreeSet<_>>(),
            requested,
            "{} silently pruned or added nodes",
            case.id
        );
        let counts = &evidence.allocation_counts;
        assert_eq!(counts.ordinary, case.paid.len() as u32, "{}", case.id);
        assert_eq!(
            (
                counts.ascendancy,
                counts.secondary_ascendancy,
                counts.sockets,
                counts.weapon_set_1,
                counts.weapon_set_2
            ),
            (0, 0, 0, 0, 0)
        );
        for node in &evidence.allocated_nodes {
            let expected = matrix
                .tree
                .effective_node(
                    case.class_id,
                    case.ascendancy_id.as_deref(),
                    node.physical_node_id,
                )
                .unwrap();
            assert_eq!(
                node.display_name, expected.name,
                "{} physical {} effective name",
                case.id, node.physical_node_id
            );
            assert_eq!(
                node.stats, expected.stats,
                "{} physical {} effective stats",
                case.id, node.physical_node_id
            );
            assert_eq!(
                node.name, matrix.tree.nodes[&node.physical_node_id].name,
                "live inherited name must remain distinct from dn"
            );
            assert_eq!(node.allocation_mode, 0);
            assert!(node.free_allocation.is_none());
            assert!(!node.has_hash_override);
            assert!(!node.is_conquered);
            if node.physical_node_id == class.start_node_id {
                assert!(node.implicit_roots.contains(&PassiveRootRole::Class));
            }
            if node.is_switchable {
                let switch = node
                    .switch
                    .as_ref()
                    .expect("live switchable allocation missing reverse-map evidence");
                assert_eq!(
                    switch.selected_source_id, expected.effective_source_id,
                    "{} physical {}",
                    case.id, node.physical_node_id
                );
                assert!(
                    switch.stats_reference_matches_selected_source,
                    "{} source stats reference changed",
                    case.id
                );
                assert!(switch.display_name_matches_selected_source);
                match expected.provenance {
                    OverrideProvenance::Base => assert_eq!(switch.kind, PassiveSwitchKind::Base),
                    OverrideProvenance::Class { selector, .. } => {
                        assert_eq!(switch.kind, PassiveSwitchKind::Class);
                        assert_eq!(switch.selector.as_deref(), Some(selector.as_str()));
                        switched_classes.insert(case.class_id);
                    }
                    OverrideProvenance::Ascendancy {
                        internal_id,
                        selector,
                    } => {
                        assert_eq!(switch.kind, PassiveSwitchKind::Ascendancy);
                        assert_eq!(switch.selector.as_deref(), Some(selector.as_str()));
                        switched_ascendancies.insert(internal_id);
                    }
                }
            }
        }
        if case.paid.is_empty() {
            for (stat, expected) in [
                ("Str", class.base_strength),
                ("Dex", class.base_dexterity),
                ("Int", class.base_intelligence),
            ] {
                assert_eq!(
                    result.player.metrics[stat],
                    f64::from(expected),
                    "{} base {stat}",
                    case.id
                );
            }
            for stat in ["Life", "Mana"] {
                assert!(
                    result.player.metrics[stat] > 0.0,
                    "{} resource {stat}",
                    case.id
                );
            }
        }
    }
    assert!(switched_classes.contains(&1));
    assert!(switched_classes.contains(&8));
    assert!(switched_ascendancies.contains("Witch3b"));
    let abyssal = matrix
        .observations
        .iter()
        .find(|value| value.case.ascendancy_id.as_deref() == Some("Witch3b"))
        .unwrap();
    let root = passives(&abyssal.result)
        .allocated_nodes
        .iter()
        .find(|node| node.physical_node_id == 23710)
        .unwrap();
    assert_eq!(
        root.display_name, "Lich",
        "the Abyssal Lich root inherits Lich's live display name"
    );
}

#[test]
fn supplied_build_and_weapon_allocation_modes_are_observed_without_repair() {
    let _ = matrix(); // Complete the cached two-worker matrix before extra processes.
    let supplied = evaluate(
        &fs::read_to_string(root().join("tests/fixtures/builds/pobarchives-Dfz36mCq.xml")).unwrap(),
    );
    let evidence = passives(&supplied);
    assert_eq!(evidence.class.name, "Sorceress");
    assert_eq!(
        evidence.ascendancy.as_ref().unwrap().name,
        "Disciple of Varashta"
    );
    assert_eq!(evidence.allocated_nodes.len(), 130);
    assert!(
        evidence
            .allocated_nodes
            .iter()
            .any(|node| node.has_hash_override)
    );
    let swapped = evaluate(&xml(
        6,
        "Warrior",
        0,
        "",
        "None",
        &[3936],
        r#"<WeaponSet1 nodes="3936"/>"#,
    ));
    let evidence = passives(&swapped);
    let node = evidence
        .allocated_nodes
        .iter()
        .find(|node| node.physical_node_id == 3936)
        .unwrap();
    assert_eq!(node.allocation_mode, 1);
    assert_eq!(evidence.allocation_counts.ordinary, 1);
    assert_eq!(evidence.allocation_counts.weapon_set_1, 1);
    assert_eq!(evidence.allocation_counts.weapon_set_2, 0);
    // Ordinary and weapon-set accounting overlap; adding the buckets would overcount.
    assert_eq!(evidence.allocated_nodes.len(), 2);
}

#[test]
#[ignore = "fresh passive observation worker invoked by parent tests"]
fn passive_observation_worker() {
    let output = PathBuf::from(std::env::var_os("POE_PASSIVE_TEST_OUTPUT").expect("child output"));
    match std::env::var("POE_PASSIVE_TEST_MODE").unwrap().as_str() {
        "tree" => fs::write(
            output,
            serde_json::to_vec(
                &poe_optimizer_pob::tree_data::extract_pinned_tree(&source(), "0_5").unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        "evaluate" => {
            let input = PathBuf::from(std::env::var_os("POE_PASSIVE_TEST_INPUT").unwrap());
            let scratch = PathBuf::from(std::env::var_os("POE_PASSIVE_TEST_SCRATCH").unwrap());
            let snapshot = poe_optimizer_pob::runtime::evaluate(
                &source(),
                &scratch,
                &fs::read_to_string(input).unwrap(),
            )
            .unwrap();
            fs::write(output, serde_json::to_vec(&snapshot).unwrap()).unwrap();
        }
        _ => panic!("unknown child mode"),
    }
}
