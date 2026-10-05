//! Optional full-catalog writer census paired with independently passed cfg observations.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

const TEST: &str = "area_eligibility::complete_area_eligibility_writer_inventory";
const MODE: &str = "POE_AREA_ELIGIBILITY_CHILD";
const OUTPUT: &str = "POE_AREA_ELIGIBILITY_OUT";
const COLLECTOR: &str = include_str!("area_eligibility_source.lua");
const LIMIT: usize = 16 * 1024 * 1024;
const MAGNIFIED: &str = "runs/owned-magnified-area-source-04";
const MAGNIFIED_SHA: &str = "f77f0506c32020c028d1ce007e2600dceef06ff083c2b7ad782177373cba0d6d";

#[test]
#[ignore = "requires complete pinned PoB and passed Magnified source04; cfg Area writer inventory only"]
fn complete_area_eligibility_writer_inventory() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join(
        std::env::var_os(OUTPUT)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("runs/owned-area-eligibility-source-01")),
    );
    if let Some(mode) = std::env::var_os(MODE) {
        assert!(mode == "off" || mode == "on");
        run_child(&root, &out, mode == "on");
        return;
    }
    assert!(
        std::env::var_os(CHILD).is_none(),
        "unset historical source child selector"
    );
    assert!(!out.exists(), "choose fresh {OUTPUT}: {}", out.display());
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env(MODE, mode)
            .env(OUTPUT, &out)
            .env_remove(CHILD)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "{}\n{}", path.display(), tail(&path));
                break;
            }
            if started.elapsed() > Duration::from_secs(300) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("Area source deadline: {}\n{}", path.display(), tail(&path));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    json_evidence::assert_files_equal(
        &out.join("source-jit-off.json"),
        &out.join("source-jit-on.json"),
        "Area census JIT parity",
    );
}

fn run_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let source04 = reference_matrix(root);
    let original_path = "tests/fixtures/builds/breadth-20260908/build-05.xml";
    let xml = fs::read(root.join(original_path)).unwrap();
    let index: Json = serde_json::from_slice(
        &fs::read(root.join("tests/fixtures/builds/breadth-20260908/index.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(digest(&xml), index["builds"][4]["xml_sha256"]);
    let text = std::str::from_utf8(&xml).unwrap();
    let mut cases = vec![];
    for name in ["original-05", "repeat-original-05"] {
        let observed = observe_with_extra(root, name, text, enabled, None, Some(COLLECTOR));
        let states = Json::Object(
            STAGES
                .into_iter()
                .map(|stage| {
                    (
                        stage.to_owned(),
                        observed["states"][stage]["delivery"].clone(),
                    )
                })
                .collect(),
        );
        cases.push(json!({"name":name,"xml_sha256":observed["xml_sha256"],"source_identity":observed["source_identity"],"states":states}));
    }
    let manifest: Json = serde_json::from_slice(
        &fs::read(root.join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap(),
    )
    .unwrap();
    let mut paths: BTreeSet<_> = FILES.into_iter().collect();
    paths.extend(["src/Data/SkillStatMap.lua", "src/Modules/CalcOffence.lua"]);
    for row in manifest["files"].as_array().unwrap() {
        let path = row["path"].as_str().unwrap();
        if path.starts_with("src/Data/Skills/") {
            paths.insert(path);
        }
    }
    let files: Vec<_> = paths
        .into_iter()
        .map(|path| {
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["path"] == path)
                .unwrap()
                .clone()
        })
        .collect();
    let catalog_path = "data/owned/poe2/3887ae68/import/skill-identities.json";
    let catalog_bytes = fs::read(root.join(catalog_path)).unwrap();
    let catalog: Json = serde_json::from_slice(&catalog_bytes).unwrap();
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,
        "manifest_sha256":pinned::manifest_sha256(),"observer_sha256":digest(COLLECTOR.as_bytes()),"files":files,
        "catalog_reference":{"path":catalog_path,"sha256":digest(&catalog_bytes)},
        "original_source":{"path":original_path,"sha256":digest(&xml)},"lifecycle_stages":STAGES,"cases":cases,
        "source04":source04,"business_wrappers":false,"source_tables_mutated":false,"source_cfg_modified":false,
        "numeric_tolerance":0,"native_build_parity":false,"final_area_radius_authority":false,
        "meaning":"cfg Area-modifier eligibility for exact enumerated actions; no arbitrary skill classifier or default"});
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(
        bytes.len() <= LIMIT,
        "Area census report {} exceeds {}",
        bytes.len(),
        LIMIT
    );
    let mode = if enabled { "on" } else { "off" };
    fs::write(out.join(format!("source-jit-{mode}.raw.json")), &bytes).unwrap();
    check(&report, &catalog);
    fs::write(out.join(format!("source-jit-{mode}.json")), &bytes).unwrap();
    assert_eq!(fs::read(root.join(original_path)).unwrap(), xml);
}

fn reference_matrix(root: &Path) -> Json {
    let paths = [
        format!("{MAGNIFIED}/source-jit-off.json"),
        format!("{MAGNIFIED}/source-jit-on.json"),
    ];
    for path in &paths {
        assert_eq!(fs::metadata(root.join(path)).unwrap().len(), 74_626_853);
        assert_eq!(digest(&fs::read(root.join(path)).unwrap()), MAGNIFIED_SHA);
    }
    let source: Json = serde_json::from_slice(&fs::read(root.join(&paths[0])).unwrap()).unwrap();
    let requested: BTreeSet<_> = expected().into_keys().collect();
    struct ObservationCount {
        area: bool,
        count: u64,
        late: u64,
        pointers: Vec<String>,
    }
    let mut matrix: BTreeMap<(String, u64), ObservationCount> = BTreeMap::new();
    for (ci, case) in rows(&source["cases"]).iter().enumerate() {
        for stage in STAGES {
            for (ri, row) in rows(&case["states"][stage]["delivery"]["contexts"])
                .iter()
                .enumerate()
            {
                let prefix = format!("/cases/{ci}/states/{stage}/delivery/contexts/{ri}");
                for (value, pointer) in std::iter::once((row, prefix.clone())).chain(
                    rows(&row["children"])
                        .iter()
                        .enumerate()
                        .map(|(i, v)| (v, format!("{prefix}/children/{i}"))),
                ) {
                    let effect = value["effect"].as_str().unwrap();
                    if !requested.contains(effect) {
                        continue;
                    }
                    let set = value["stat_set_index"].as_u64().unwrap();
                    let flag = value["queries"]["area_flag"].as_bool().unwrap();
                    let base = value["queries"]["definition_base_flags"]["area"] == true;
                    assert_eq!(flag, base, "source04 cfg differs from base at {pointer}");
                    let entry =
                        matrix
                            .entry((effect.to_owned(), set))
                            .or_insert(ObservationCount {
                                area: flag,
                                count: 0,
                                late: 0,
                                pointers: vec![],
                            });
                    assert_eq!(entry.area, flag);
                    entry.count += 1;
                    if value["queries"]["skill_flags"]["area"] == true && !flag {
                        entry.late += 1;
                    }
                    if entry.pointers.len() < 3 {
                        entry.pointers.push(pointer);
                    }
                }
            }
        }
    }
    assert_eq!(matrix.len(), 14);
    let rows: Vec<_> = matrix
        .into_iter()
        .map(|((effect, index), v)| {
            json!({"effect":effect,"stat_set_index":index,"cfg_area":v.area,"observations":v.count,
            "later_area_true_but_cfg_false":v.late,"sample_pointers":v.pointers})
        })
        .collect();
    json!({"reports":paths.map(|path|json!({"path":path,"bytes":74_626_853,"sha256":MAGNIFIED_SHA})),"matrix":rows})
}

fn expected() -> BTreeMap<&'static str, Vec<bool>> {
    [
        ("ChilledGroundBurstWaterDjinn", vec![true]),
        ("ChilledGroundOasisConvertWaterDjinn", vec![false]),
        ("CommandSandDjinnKnifeThrowPlayer", vec![false]),
        ("CommandWaterDjinnBubblePlayer", vec![false]),
        ("ESRechargeForceRestartWaterDjinn", vec![false]),
        ("ExplosiveTeleportSandDjinn", vec![true, true]),
        ("HandSlamSandDjinn", vec![true]),
        ("IceNovaPlayer", vec![true, true]),
        ("KnifeThrowSandDjinn", vec![false, true]),
        ("PassiveTriggeredManaWaveWaterDjinn", vec![false]),
        ("WaterBubbleWaterDjinn", vec![true]),
    ]
    .into_iter()
    .collect()
}
fn check(report: &Json, catalog: &Json) {
    let first = &report["cases"][0]["states"]["fresh"];
    let expected = expected();
    let catalog_all: BTreeSet<_> = rows(&catalog["skills"])
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    let catalog_support: BTreeSet<_> = rows(&catalog["skills"])
        .iter()
        .filter(|s| s["support"] == true)
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    assert!(!catalog_support.is_empty());
    for case in rows(&report["cases"]) {
        for stage in STAGES {
            let state = &case["states"][stage];
            assert_eq!(
                json_evidence::first_difference(first, state, "catalogue-and-observations"),
                None,
                "{}/{stage}",
                case["name"]
            );
            for field in [
                "business_wrappers",
                "source_tables_mutated",
                "source_cfg_modified",
            ] {
                assert_eq!(state[field], false);
            }
            for field in ["original_methods_preserved", "jit_mode_preserved"] {
                assert_eq!(state[field], true);
            }
            assert_eq!(state["immutable_snapshot"]["verified"], true);
            let all: BTreeSet<_> = rows(&state["catalog"])
                .iter()
                .map(|s| s["id"].as_str().unwrap())
                .collect();
            assert_eq!(all, catalog_all);
            let support: BTreeSet<_> = rows(&state["catalog"])
                .iter()
                .filter(|s| s["support"] == true)
                .map(|s| s["id"].as_str().unwrap())
                .collect();
            assert_eq!(support, catalog_support);
            assert_eq!(
                state["support_count"].as_u64().unwrap() as usize,
                support.len()
            );
            assert_eq!(
                state["effect_count"].as_u64().unwrap() as usize,
                rows(&state["catalog"]).len()
            );
            for row in rows(&state["catalog"]) {
                // These declarations are root fields despite their indentation
                // in act_str.lua. Preserve the exact non-Area writer inventory.
                let totem = matches!(
                    row["id"].as_str().unwrap(),
                    "SupportAncestralWarriorTotemPlayer"
                        | "SupportMetaTotemSpellTotemPlayer"
                        | "SupportMortarCannonPlayer"
                );
                assert_eq!(
                    row["add_flags"]["present"], totem,
                    "unexpected root addFlags {}",
                    row["id"]
                );
                if totem {
                    assert_eq!(row["support"], true);
                    assert_eq!(
                        row["add_flags"]["entries"],
                        json!([{"name":"totem","value":true}])
                    );
                } else {
                    assert!(rows(&row["add_flags"]["entries"]).is_empty());
                }
            }
            for map in rows(&state["global_map"]) {
                if map["skill_flag_present"] == true {
                    assert_eq!(map["skill_flag"], "arrow");
                }
            }
            assert_eq!(rows(&state["definitions"]).len(), expected.len());
            for def in rows(&state["definitions"]) {
                let effect = def["effect"].as_str().unwrap();
                let flags = &expected[effect];
                assert_eq!(def["parts_present"], false);
                assert!(rows(&def["parts"]).is_empty());
                assert_eq!(def["root_fallback_exact"], true);
                for map in rows(&def["root_map"]) {
                    assert_ne!(map["skill_flag"], "area");
                }
                let sets = rows(&def["stat_sets"]);
                assert_eq!(sets.len(), flags.len());
                for (i, set) in sets.iter().enumerate() {
                    assert_eq!(set["index"], i + 1);
                    let area = rows(&set["base_flags"]["entries"])
                        .iter()
                        .find(|v| v["name"] == "area")
                        .is_some_and(|v| v["value"] == true);
                    assert_eq!(area, flags[i], "{effect}/{}", i + 1);
                    for map in rows(&set["local_map"]) {
                        assert_ne!(map["skill_flag"], "area");
                    }
                    let prior = rows(&report["source04"]["matrix"])
                        .iter()
                        .filter(|v| v["effect"] == effect && v["stat_set_index"] == i + 1)
                        .collect::<Vec<_>>();
                    assert_eq!(prior.len(), 1);
                    assert_eq!(prior[0]["cfg_area"], area);
                }
            }
            assert!(!rows(&state["observations"]).is_empty());
            for row in rows(&state["observations"]) {
                assert_eq!(row["cfg_available"], true);
                assert_eq!(row["cfg_area"], row["definition_base_area"]);
                let effect = row["effect"].as_str().unwrap();
                let index = row["stat_set_index"].as_u64().unwrap() as usize;
                assert_eq!(row["cfg_area"], expected[effect][index - 1]);
            }
        }
    }
    assert!(
        rows(&report["source04"]["matrix"])
            .iter()
            .any(|v| v["effect"] == "KnifeThrowSandDjinn"
                && v["stat_set_index"] == 1
                && v["later_area_true_but_cfg_false"].as_u64().unwrap() > 0)
    );
}
