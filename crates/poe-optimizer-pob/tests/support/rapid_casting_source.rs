//! Optional finite original-source contribution evidence, not final cast-rate parity.
use super::magnified_area_support::{
    check_saved_origin, edit_gem, edit_group, focus, group_range, set_attr, template,
};
use super::*;
use std::collections::BTreeSet;

const TEST_NAME: &str =
    "rapid_casting::complete_rapid_casting_delivery_uses_physical_ice_recipients";
const MODE: &str = "POE_RAPID_CASTING_SOURCE_CHILD";
const OUTPUT: &str = "POE_RAPID_CASTING_SOURCE_OUT";
const DELIVERY: &str = include_str!("rapid_casting_delivery.lua");
const I: &str = "SupportRapidCastingPlayer";
const II: &str = "SupportRapidCastingPlayerTwo";
const ICE: &str = "IceNovaPlayer";
const REPORT_LIMIT: usize = 96 * 1024 * 1024;

#[test]
#[ignore = "requires pinned original PoB; finite Rapid Casting source delivery in both JIT modes"]
fn complete_rapid_casting_delivery_uses_physical_ice_recipients() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join(
        std::env::var_os(OUTPUT)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("runs/owned-rapid-casting-source-01")),
    );
    if let Some(mode) = std::env::var_os(MODE) {
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
        "immutable source evidence exists; choose fresh {OUTPUT}"
    );
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST_NAME, "--ignored", "--nocapture"])
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
            if let Some(status) = process.try_wait().unwrap() {
                assert!(status.success(), "{}\n{}", path.display(), tail(&path));
                break;
            }
            if started.elapsed() > Duration::from_secs(600) {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!(
                    "Rapid Casting source deadline: {}\n{}",
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
        "Rapid Casting source JIT parity",
    );
}

fn child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let dir = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json = serde_json::from_slice(&fs::read(dir.join("index.json")).unwrap()).unwrap();
    let originals: Vec<_> = (1..=5)
        .map(|i| fs::read(dir.join(format!("build-{i:02}.xml"))).unwrap())
        .collect();
    let mut cases = Vec::new();
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(digest(bytes), index["builds"][i]["xml_sha256"]);
        cases.push(observe_source(
            root,
            &format!("original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
            None,
        ));
    }
    for (name, xml, control) in controls(std::str::from_utf8(&originals[4]).unwrap()) {
        cases.push(observe_source(root, &name, &xml, enabled, Some(control)));
    }
    for (i, bytes) in originals.iter().enumerate() {
        cases.push(observe_source(
            root,
            &format!("repeat-original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
            None,
        ));
    }
    assert_eq!(cases.len(), 18);
    let mut files = FILES.to_vec();
    files.extend([
        "src/Data/SkillStatMap.lua",
        "src/Data/Skills/act_int.lua",
        "src/Modules/CalcOffence.lua",
        "src/Modules/CalcDefence.lua",
        "src/Classes/ModStore.lua",
        "src/Classes/ModDB.lua",
        "src/Classes/ModList.lua",
    ]);
    let report = json!({
        "schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,
        "manifest_sha256":pinned::manifest_sha256(),"observer_sha256":digest(DELIVERY.as_bytes()),
        "files":files.into_iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),
        "original_sources":originals.iter().enumerate().map(|(i,b)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(b)})).collect::<Vec<_>>(),
        "business_wrappers":false,"source_tables_mutated":false,"source_cfg_modified":false,
        "native_build_parity":false,"native_inventory_authority":false,"final_cast_rate_formula_authority":false,
        "cost_or_reservation_formula_authority":false,"canonical_parity_lifecycle_selected":false,
        "purpose":"finite physical Ice Nova Rapid Casting contribution and exact support-source correspondence",
        "query_observation_kind":"diagnostic_original_method_read","original_calculation_calls_captured":false,
        "observation_order":"exact original per-channel record order; no arithmetic reordered",
        "numeric_tolerance":0,"lifecycle_stages":STAGES,"cases":cases
    });
    let bytes = serde_json::to_vec(&report).unwrap();
    let suffix = if enabled { "on" } else { "off" };
    let raw = out.join(format!("source-jit-{suffix}.raw.json"));
    fs::write(&raw, &bytes).unwrap();
    eprintln!(
        "Rapid Casting evidence: {} bytes at {}",
        bytes.len(),
        raw.display()
    );
    assert!(
        bytes.len() <= REPORT_LIMIT,
        "source report exceeds bound; raw diagnostics: {}",
        raw.display()
    );
    check(&report);
    fs::write(out.join(format!("source-jit-{suffix}.json")), bytes).unwrap();
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(
            fs::read(dir.join(format!("build-{:02}.xml", i + 1))).unwrap(),
            *bytes
        );
    }
}

fn observe_source(
    root: &Path,
    name: &str,
    xml: &str,
    enabled: bool,
    control: Option<Json>,
) -> Json {
    let mut observed = observe_with_extra(root, name, xml, enabled, control, Some(DELIVERY));
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

fn controls(original: &str) -> Vec<(String, String, Json)> {
    let first = template(original, ICE, I);
    // Preserve a genuine archived physical source row, rather than making up a
    // catalog/game/variant join. This changes only the controlled active group.
    let document = roxmltree::Document::parse(original).unwrap();
    let second = document
        .descendants()
        .find(|n| {
            n.has_tag_name("Gem")
                && n.attribute("skillId") == Some(II)
                && n.parent().is_some_and(|g| {
                    g.has_tag_name("Skill")
                        && g.attribute("source").is_none()
                        && g.children()
                            .find(|v| v.has_tag_name("Gem"))
                            .is_some_and(|v| v.attribute("skillId") == Some(ICE))
                })
        })
        .unwrap();
    let second = &original[second.range()];
    let focused = focus(original, ICE, None, 1, 2);
    let tier_two = edit_group(&focused, ICE, |group| {
        edit_gem(group, I, |_| second.to_owned())
    });
    let mut out = vec![];
    for (name, xml, kind, winner) in [
        ("ice-tier-i", focused.clone(), "focus", Some(I)),
        ("ice-tier-ii", tier_two.clone(), "focus", Some(II)),
        (
            "ice-remove",
            edit_group(&focused, ICE, |g| edit_gem(g, I, |_| String::new())),
            "absent",
            None,
        ),
        (
            "ice-disable",
            edit_group(&focused, ICE, |g| {
                edit_gem(g, I, |gem| set_attr(gem, "enabled", "false"))
            }),
            "absent",
            None,
        ),
        (
            "family-ii-last",
            edit_group(&focused, ICE, |g| {
                edit_gem(g, I, |_| format!("{first}{second}"))
            }),
            "family",
            Some(II),
        ),
        (
            "family-i-last",
            edit_group(&focused, ICE, |g| {
                edit_gem(g, I, |_| format!("{second}{first}"))
            }),
            "family",
            Some(I),
        ),
        ("repeat-ice-tier-i", focused, "focus", Some(I)),
        ("repeat-ice-tier-ii", tier_two, "focus", Some(II)),
    ] {
        assert_ne!(xml, original);
        let (range, _) = group_range(&xml, ICE);
        let group = roxmltree::Document::parse(&xml[range]).unwrap();
        let saved: Vec<_> = group
            .root_element()
            .children()
            .filter(|n| n.has_tag_name("Gem"))
            .collect();
        let winner_position = winner.map(|effect| {
            saved
                .iter()
                .position(|g| g.attribute("skillId") == Some(effect))
                .unwrap()
                + 1
        });
        out.push((
            name.to_owned(),
            xml,
            json!({"kind":kind,"effect":ICE,"winner":winner,
            "winner_position":winner_position,"main_set":1,"calcs_set":2}),
        ));
    }
    assert_eq!(out.len(), 8);
    out
}

fn case<'a>(report: &'a Json, name: &str) -> &'a Json {
    let cases: Vec<_> = rows(&report["cases"])
        .iter()
        .filter(|c| c["name"] == name)
        .collect();
    assert_eq!(cases.len(), 1);
    cases[0]
}

fn check_definition(def: &Json, id: &str, value: u64) {
    assert_eq!(def["effect"], id);
    assert_eq!(def["mod_source"], format!("Skill:{id}"));
    let levels = rows(&def["levels"]["positions"]);
    assert_eq!(levels.len(), 1);
    assert_eq!(levels[0]["index"], 1);
    assert_eq!(levels[0]["value"], json!({"levelRequirement":0}));
    // Missing coefficients remain absent. No synthetic identity cost/reservation
    // contribution is authorized by this support family.
    for name in [
        "manaMultiplier",
        "reservationMultiplier",
        "manaReservationPercent",
        "spiritReservationFlat",
    ] {
        assert!(
            levels[0]["value"].get(name).is_none(),
            "unexpected {id}/{name}"
        );
    }
    let sets = rows(&def["stat_sets"]);
    assert_eq!(sets.len(), 1);
    let constants = rows(&sets[0]["constants"]["positions"]);
    assert_eq!(constants.len(), 1);
    assert_eq!(constants[0]["index"], 1);
    let pair = rows(&constants[0]["value"]["positions"]);
    assert_eq!(
        pair,
        &[
            json!({"index":1,"value":"base_cast_speed_+%"}),
            json!({"index":2,"value":value})
        ]
    );
    assert!(rows(&sets[0]["stats"]).is_empty());
    assert!(sets[0]["quality_stats"].is_null() || rows(&sets[0]["quality_stats"]).is_empty());
    assert!(sets[0]["base_mods"].is_null() || rows(&sets[0]["base_mods"]).is_empty());
    assert!(rows(&def["add_types"]).is_empty());
    assert!(def["add_flags"].is_null() || rows(&def["add_flags"]).is_empty());
}

fn check_context(context: &Json, label: &str) {
    assert_eq!(context["effect"], ICE, "{label}");
    assert_eq!(context["actor_is_player"], true, "{label}");
    assert_eq!(context["source"]["exact_source_instance"], true, "{label}");
    let candidates = rows(&context["candidates"]);
    assert!(candidates.len() <= 1, "{label}");
    for support in candidates {
        assert!(support["effect"] == I || support["effect"] == II, "{label}");
        assert_eq!(support["exact_definition"], true, "{label}");
        assert_eq!(support["accepted"], true, "{label}");
        assert_eq!(support["origin"]["exact_source_instance"], true, "{label}");
        assert_eq!(support["origin"]["enabled"], true, "{label}");
        assert!(support["origin"]["source_ordinal"].is_u64(), "{label}");
        assert_ne!(
            support["origin"]["source_ordinal"], context["source"]["source_ordinal"],
            "{label}"
        );
    }
    let q = &context["queries"];
    for key in [
        "cfg_effect_exact",
        "exact_stat_set",
        "original_query_methods",
        "cast_flag",
    ] {
        assert_eq!(q[key], true, "{label}/{key}");
    }
    assert_eq!(
        q["query_observation_kind"],
        "diagnostic_original_method_read"
    );
    assert_eq!(q["original_calculation_call_captured"], false);
    assert!(q["speed_increase"].is_number(), "{label}");
    let disabled = q["skill_flags"]["disable"] == true;
    let selected = candidates.first().filter(|_| !disabled);
    let raw = rows(&q["channels"]["Speed"]["raw_source_records"]);
    assert_eq!(
        raw.len(),
        usize::from(selected.is_some()),
        "{label}/raw-speed"
    );
    for r in raw {
        let effect = selected.unwrap()["effect"].as_str().unwrap();
        assert_eq!(r["channel_index"], 1);
        assert_eq!(r["source_effect"], effect);
        // CalcActiveSkill:496-498/733 builds this occurrence's support record
        // in baseSkillModList. CalcPerform:1205 then installs the working list
        // with that exact base as its parent; a local source is at depth one.
        assert_eq!(r["ancestor_depth"], 1, "{label}/local-recipient");
        assert_eq!(
            q["store_chain"],
            json!([
                {"depth":0,"kind":"ModList","base_skill_store":false,"actor_store":false},
                {"depth":1,"kind":"ModList","base_skill_store":true,"actor_store":false},
                {"depth":2,"kind":"ModDB","base_skill_store":false,"actor_store":true}
            ]),
            "{label}/exact-recipient-parent-chain"
        );
        let expected = json!({"name":"Speed","type":"INC","value":if effect==I {15}else{20},
            "source":format!("Skill:{effect}"),"flags":q["cast_flag_value"],"keyword_flags":0,"tags":{}});
        assert!(
            r["record"] == expected,
            "{label}/exact-Speed-record: {}",
            r["record"]
        );
    }
    let applied: Vec<_> = rows(&q["channels"]["Speed"]["applied"])
        .iter()
        .filter(|r| r.get("source_effect").is_some())
        .collect();
    assert_eq!(applied.len(), raw.len(), "{label}/applied-speed");
    for r in applied {
        assert_eq!(r["source_record_indices"], json!([1]));
        assert_eq!(r["record"], raw[0]["record"]);
        assert_eq!(r["source_effect"], raw[0]["source_effect"]);
        assert_eq!(r["value"], raw[0]["record"]["value"]);
    }
    for channel in [
        "SupportManaMultiplier",
        "ReservationMultiplier",
        "ExtraSpirit",
    ] {
        let c = &q["channels"][channel];
        assert!(
            rows(&c["raw_source_records"]).is_empty(),
            "{label}/{channel}/unexpected-raw"
        );
        assert!(
            rows(&c["applied"])
                .iter()
                .all(|r| r.get("source_effect").is_none()
                    && rows(&r["source_record_indices"]).is_empty()),
            "{label}/{channel}/unexpected-applied"
        );
    }
}

fn check(report: &Json) {
    assert_eq!(rows(&report["cases"]).len(), 18);
    let mut selected_sets = BTreeSet::new();
    for c in rows(&report["cases"]) {
        assert_eq!(c["independent_source_bindings_verified"], true);
        for stage in STAGES {
            let delivery = &c["states"][stage]["delivery"];
            for field in ["original_methods_preserved", "jit_mode_preserved"] {
                assert_eq!(delivery[field], true);
            }
            for field in [
                "source_cfg_modified",
                "source_tables_mutated",
                "business_wrappers",
                "original_calculation_calls_captured",
            ] {
                assert_eq!(delivery[field], false);
            }
            assert_eq!(delivery["immutable_snapshot"]["verified"], true);
            let definitions = rows(&delivery["definitions"]);
            assert_eq!(definitions.len(), 2);
            check_definition(&definitions[0], I, 15);
            check_definition(&definitions[1], II, 20);
            for context in rows(&delivery["contexts"]) {
                let label = format!(
                    "{}/{stage}/{}/{}",
                    c["name"], context["mode"], context["stat_set_index"]
                );
                check_context(context, &label);
            }
            if !c["control"].is_null() {
                for mode in ["MAIN", "CALCS"] {
                    let contexts: Vec<_> = rows(&delivery["contexts"])
                        .iter()
                        .filter(|p| p["mode"] == mode)
                        .collect();
                    assert_eq!(contexts.len(), 1, "{}/{stage}/{mode}", c["name"]);
                    let p = contexts[0];
                    assert_eq!(p["selected"], true);
                    assert_eq!(p["queries"]["output_available"], true);
                    assert_eq!(p["queries"]["output_is_selected_actor"], true);
                    assert_eq!(
                        p["stat_set_index"],
                        c["control"][if mode == "MAIN" {
                            "main_set"
                        } else {
                            "calcs_set"
                        }]
                    );
                    selected_sets.insert(p["stat_set_index"].as_u64().unwrap());
                    let candidates = rows(&p["candidates"]);
                    if c["control"]["kind"] == "absent" {
                        assert!(candidates.is_empty());
                    } else {
                        assert_eq!(candidates.len(), 1);
                        assert_eq!(candidates[0]["effect"], c["control"]["winner"]);
                        assert_eq!(
                            candidates[0]["origin"]["position"],
                            c["control"]["winner_position"]
                        );
                    }
                }
            }
        }
    }
    assert_eq!(selected_sets, BTreeSet::from([1, 2]));
    for stage in STAGES {
        let contexts = rows(&case(report, "original-05")["states"][stage]["delivery"]["contexts"]);
        assert_eq!(contexts.len(), 2);
        for c in contexts {
            assert_eq!(
                rows(&c["candidates"])[0]["effect"],
                I,
                "saved Original05 uses tier I"
            );
        }
    }
    for (a, b) in (1..=5)
        .map(|i| {
            (
                format!("original-{i:02}"),
                format!("repeat-original-{i:02}"),
            )
        })
        .chain([
            ("ice-tier-i".into(), "repeat-ice-tier-i".into()),
            ("ice-tier-ii".into(), "repeat-ice-tier-ii".into()),
        ])
    {
        let a = case(report, &a);
        let b = case(report, &b);
        assert_eq!(a["xml_sha256"], b["xml_sha256"]);
        assert_eq!(
            json_evidence::first_difference(&a["states"], &b["states"], "independent-repeat"),
            None,
            "{}/{}",
            a["name"],
            b["name"]
        );
    }
}
