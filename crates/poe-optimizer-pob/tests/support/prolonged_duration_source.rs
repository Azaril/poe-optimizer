//! Real non-damaging player-action support delivery, with a damaging-action contrast.
use super::magnified_area_support::{edit_gem, edit_group, focus, group_range, set_attr, template};
use super::*;
use std::collections::BTreeSet;

const TEST_NAME: &str =
    "prolonged_duration::prolonged_duration_uses_real_physical_action_recipients";
const MODE: &str = "POE_PROLONGED_DURATION_SOURCE_CHILD";
const OUTPUT: &str = "POE_PROLONGED_DURATION_SOURCE_OUT";
const COLLECTOR: &str = include_str!("physical_support_delivery.lua");
const PROFILE: &str = include_str!("prolonged_duration_delivery.lua");
const OFFERING: &str = "PainOfferingPlayer";
const ICE: &str = "IceNovaPlayer";
const I: &str = "ProlongedDurationSupportPlayer";
const II: &str = "ProlongedDurationSupportPlayerTwo";
const STAT: &str = "support_more_duration_skill_effect_duration_+%_final";

fn observer() -> String {
    format!(
        "local collect = (function()\n{COLLECTOR}\nend)()\nreturn (function(collect)\n{PROFILE}\nend)(collect)"
    )
}
#[test]
#[ignore = "requires pinned original PoB; finite Prolonged Duration real-action delivery in both JIT modes"]
fn prolonged_duration_uses_real_physical_action_recipients() {
    super::physical_support::run_modes(
        super::physical_support::Witness {
            name: TEST_NAME,
            child_env: MODE,
            output_env: OUTPUT,
            default_output: "runs/owned-prolonged-duration-source-01",
            label: "Prolonged Duration",
        },
        child,
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
    let observer = observer();
    let mut cases = vec![];
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(digest(bytes), index["builds"][i]["xml_sha256"]);
        cases.push(super::physical_support::observe_physical(
            root,
            &format!("original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
            None,
            &observer,
        ));
    }
    for (name, xml, control) in controls(std::str::from_utf8(&originals[4]).unwrap()) {
        cases.push(super::physical_support::observe_physical(
            root,
            &name,
            &xml,
            enabled,
            Some(control),
            &observer,
        ));
    }
    for (i, bytes) in originals.iter().enumerate() {
        cases.push(super::physical_support::observe_physical(
            root,
            &format!("repeat-original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
            None,
            &observer,
        ));
    }
    assert_eq!(cases.len(), 24);
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
    let report = json!({"schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":digest(observer.as_bytes()),"observer_parts":[
            {"path":"crates/poe-optimizer-pob/tests/support/physical_support_delivery.lua","sha256":digest(COLLECTOR.as_bytes())},
            {"path":"crates/poe-optimizer-pob/tests/support/prolonged_duration_delivery.lua","sha256":digest(PROFILE.as_bytes())}],
        "files":files.into_iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),
        "original_sources":originals.iter().enumerate().map(|(i,b)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(b)})).collect::<Vec<_>>(),
        "business_wrappers":false,"source_tables_mutated":false,"source_cfg_modified":false,"native_build_parity":false,
        "native_inventory_authority":false,"final_duration_formula_authority":false,"final_resource_cost_formula_authority":false,
        "reservation_formula_authority":false,"effect_application_uptime_authority":false,"final_input_assembly_authority":false,"canonical_parity_lifecycle_selected":false,
        "purpose":"finite Prolonged Duration delivery to real physical Pain Offering and Ice Nova actions; buff recipients and action duration remain distinct",
        "query_observation_kind":"diagnostic_original_method_read","original_calculation_calls_captured":false,
        "observation_order":"exact original per-channel record order; no arithmetic reordered","numeric_tolerance":0,"lifecycle_stages":STAGES,"cases":cases});
    let bytes = serde_json::to_vec(&report).unwrap();
    let suffix = if enabled { "on" } else { "off" };
    let raw = out.join(format!("source-jit-{suffix}.raw.json"));
    fs::write(&raw, &bytes).unwrap();
    eprintln!(
        "Prolonged Duration evidence: {} bytes at {}",
        bytes.len(),
        raw.display()
    );
    assert!(
        bytes.len() <= 128 * 1024 * 1024,
        "bounded source report; diagnostics {}",
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
fn controls(original: &str) -> Vec<(String, String, Json)> {
    let first = template(original, OFFERING, I);
    let doc = roxmltree::Document::parse(original).unwrap();
    let second = doc
        .descendants()
        .find(|n| n.has_tag_name("Gem") && n.attribute("skillId") == Some(II))
        .unwrap();
    let second = &original[second.range()];
    let pain = focus(original, OFFERING, None, 1, 1);
    let ice = edit_group(&focus(original, ICE, None, 1, 2), ICE, |g| {
        g.replacen("</Skill>", &format!("{first}</Skill>"), 1)
    });
    let raw = template(original, OFFERING, OFFERING);
    let lower = set_attr(&raw, "level", "5");
    let additional = set_attr(&lower, "quality", "13");
    let pair = edit_group(&pain, OFFERING, |g| {
        let g = g.replacen("</Skill>", &format!("{additional}</Skill>"), 1);
        set_attr(&g, "mainActiveSkillCalcs", "2")
    });
    let mut out = vec![];
    for (name, xml, effect, winner, copies) in [
        ("pain-tier-i", pain.clone(), OFFERING, Some(I), 1),
        (
            "pain-tier-ii",
            edit_group(&pain, OFFERING, |g| edit_gem(g, I, |_| second.to_owned())),
            OFFERING,
            Some(II),
            1,
        ),
        (
            "pain-remove",
            edit_group(&pain, OFFERING, |g| edit_gem(g, I, |_| String::new())),
            OFFERING,
            None,
            1,
        ),
        (
            "pain-disable",
            edit_group(&pain, OFFERING, |g| {
                edit_gem(g, I, |s| set_attr(s, "enabled", "false"))
            }),
            OFFERING,
            None,
            1,
        ),
        (
            "family-ii-last",
            edit_group(&pain, OFFERING, |g| {
                edit_gem(g, I, |_| format!("{first}{second}"))
            }),
            OFFERING,
            Some(II),
            1,
        ),
        (
            "family-i-last",
            edit_group(&pain, OFFERING, |g| {
                edit_gem(g, I, |_| format!("{second}{first}"))
            }),
            OFFERING,
            Some(I),
            1,
        ),
        (
            "pain-raw-level",
            edit_group(&pain, OFFERING, |g| {
                edit_gem(g, OFFERING, |s| set_attr(s, "level", "5"))
            }),
            OFFERING,
            Some(I),
            1,
        ),
        (
            "pain-raw-quality",
            edit_group(&pain, OFFERING, |g| {
                edit_gem(g, OFFERING, |s| set_attr(s, "quality", "13"))
            }),
            OFFERING,
            Some(I),
            1,
        ),
        ("pain-distinct-occurrences", pair, OFFERING, Some(I), 2),
        ("ice-tier-i", ice.clone(), ICE, Some(I), 1),
        (
            "ice-tier-ii",
            edit_group(&ice, ICE, |g| edit_gem(g, I, |_| second.to_owned())),
            ICE,
            Some(II),
            1,
        ),
        (
            "ice-remove",
            edit_group(&ice, ICE, |g| edit_gem(g, I, |_| String::new())),
            ICE,
            None,
            1,
        ),
        ("repeat-pain-tier-i", pain, OFFERING, Some(I), 1),
        ("repeat-ice-tier-i", ice, ICE, Some(I), 1),
    ] {
        assert_ne!(xml, original);
        let (range, _) = group_range(&xml, effect);
        let g = roxmltree::Document::parse(&xml[range]).unwrap();
        let gems: Vec<_> = g
            .root_element()
            .children()
            .filter(|n| n.has_tag_name("Gem"))
            .collect();
        let winner_position = winner.map(|id| {
            gems.iter()
                .position(|n| n.attribute("skillId") == Some(id))
                .unwrap()
                + 1
        });
        let sources:Vec<_>=gems.iter().enumerate().filter(|(_,n)|n.attribute("skillId")==Some(effect)).map(|(i,n)|json!({
            "position":i+1,"raw_level":n.attribute("level").unwrap().parse::<u32>().unwrap(),"raw_quality":n.attribute("quality").unwrap().parse::<u32>().unwrap()})).collect();
        assert_eq!(sources.len(), copies);
        out.push((name.to_owned(),xml,json!({"effect":effect,"winner":winner,"winner_position":winner_position,"sources":sources,
            "main_set":1,"calcs_set":if effect==ICE {2}else{1},"main_occurrence":1,"calcs_occurrence":copies})));
    }
    assert_eq!(out.len(), 14);
    out
}
fn rows_same(a: &Json, b: &Json, label: &str) {
    assert!(a == b, "{label} differs");
}
fn declaration(d: &Json, id: &str, value: u32) {
    assert_eq!(d["effect"], id);
    assert_eq!(d["mod_source"], format!("Skill:{id}"));
    rows_same(
        &d["levels"],
        &json!({"positions":[{"index":1,"value":{"levelRequirement":0,"manaMultiplier":20}}]}),
        "level declaration",
    );
    rows_same(
        &d["require_types"],
        &json!({"positions":[{"index":1,"value":9}]}),
        "Duration type",
    );
    assert!(rows(&d["exclude_types"]).is_empty());
    assert!(rows(&d["add_types"]).is_empty());
    assert!(d["add_flags"].is_null() || rows(&d["add_flags"]).is_empty());
    assert_eq!(rows(&d["stat_sets"]).len(), 1);
    let s = &d["stat_sets"][0];
    rows_same(
        &s["constants"],
        &json!({"positions":[{"index":1,"value":{"positions":[{"index":1,"value":STAT},{"index":2,"value":value}]}}]}),
        "duration constant",
    );
    assert!(rows(&s["stats"]).is_empty());
    assert!(s["quality_stats"].is_null() || rows(&s["quality_stats"]).is_empty());
    assert_eq!(rows(&s["declared_maps"]).len(), 1);
    assert_eq!(s["declared_maps"][0]["stat"], STAT);
    assert_eq!(s["declared_maps"][0]["present"], true);
    rows_same(
        &s["declared_maps"][0]["value"],
        &json!({"positions":[{"index":1,"value":{"name":"Duration","type":"MORE","source":format!("Skill:{id}"),"flags":0,"keywordFlags":0}}]}),
        "local Duration mapping",
    );
}
fn channel(q: &Json, name: &str, winner: Option<&str>, value: u32, label: &str) {
    let raw = rows(&q["channels"][name]["raw_source_records"]);
    assert_eq!(raw.len(), usize::from(winner.is_some()), "{label}/{name}");
    for (index, r) in raw.iter().enumerate() {
        let id = winner.unwrap();
        rows_same(
            r,
            &json!({"channel_index":index+1,"ancestor_depth":1,"source_effect":id,
            "record":{"name":name,"type":"MORE","value":value,"source":format!("Skill:{id}"),"flags":0,"keyword_flags":0,"tags":{}}}),
            label,
        );
    }
    let applied: Vec<_> = rows(&q["channels"][name]["applied"])
        .iter()
        .filter(|r| r.get("source_effect").is_some())
        .collect();
    assert_eq!(applied.len(), raw.len(), "{label}/{name}/applied");
    for (index, (a, r)) in applied.iter().zip(raw).enumerate() {
        rows_same(
            a,
            &json!({"value":value,"source_effect":r["source_effect"],"record":r["record"],"source_record_indices":[index+1]}),
            label,
        );
    }
    for a in rows(&q["channels"][name]["applied"])
        .iter()
        .filter(|r| r.get("source_effect").is_none())
    {
        assert!(rows(&a["source_record_indices"]).is_empty());
    }
}
fn context(c: &Json, label: &str) {
    assert!(c["effect"] == OFFERING || c["effect"] == ICE);
    assert_eq!(c["actor_is_player"], true);
    assert_eq!(c["source"]["exact_source_instance"], true);
    let candidates = rows(&c["candidates"]);
    assert!(candidates.len() <= 1);
    for s in candidates {
        assert!(s["effect"] == I || s["effect"] == II);
        for (r, k) in [
            (s, "accepted"),
            (s, "exact_definition"),
            (&s["origin"], "enabled"),
            (&s["origin"], "exact_source_instance"),
        ] {
            assert_eq!(r[k], true);
        }
        assert_ne!(s["origin"]["source_ordinal"], c["source"]["source_ordinal"]);
    }
    let q = &c["queries"];
    for k in [
        "cfg_effect_exact",
        "exact_stat_set",
        "original_query_methods",
    ] {
        assert_eq!(q[k], true);
    }
    for k in [
        "no_attached_minion",
        "no_summoning_parent",
        "exact_physical_instance",
        "exact_actor",
    ] {
        assert_eq!(q["duration"][k], true);
    }
    assert_eq!(q["duration"]["has_reservation"], false);
    assert_eq!(q["duration"]["original_calculation_calls_captured"], false);
    assert_eq!(q["duration"]["primary_definition_exact"], true);
    assert_eq!(rows(&q["duration"]["granted_effects"]).len(), 1);
    assert_eq!(q["duration"]["granted_effects"][0]["effect"], c["effect"]);
    assert_eq!(q["duration"]["granted_effects"][0]["primary"], true);
    assert_eq!(q["duration"]["granted_effects"][0]["support"], false);
    assert_eq!(q["duration_snapshot"]["verified"], true);
    assert_eq!(q["duration_snapshot"]["original_methods_preserved"], true);
    assert!(
        q["duration"]["effective_level"]
            .as_u64()
            .is_some_and(|n| n > 0)
    );
    assert!(q["duration"]["effective_quality"].is_number());
    if c["effect"] == OFFERING {
        assert_eq!(q["duration"]["primary_base"], 6);
        assert_eq!(q["duration"]["secondary_base"], 6);
    }
    let winner = candidates
        .first()
        .filter(|_| q["skill_flags"]["disable"] != true)
        .map(|s| s["effect"].as_str().unwrap());
    channel(
        q,
        "Duration",
        winner,
        if winner == Some(II) { 35 } else { 30 },
        label,
    );
    channel(q, "SupportManaMultiplier", winner, 20, label);
    for name in ["Speed", "ReservationMultiplier", "ExtraSpirit"] {
        let channel = &q["channels"][name];
        assert!(rows(&channel["raw_source_records"]).is_empty());
        assert!(
            rows(&channel["applied"])
                .iter()
                .all(|r| r.get("source_effect").is_none()
                    && rows(&r["source_record_indices"]).is_empty())
        );
    }
}
fn named<'a>(r: &'a Json, name: &str) -> &'a Json {
    let matches: Vec<_> = rows(&r["cases"])
        .iter()
        .filter(|c| c["name"] == name)
        .collect();
    assert_eq!(matches.len(), 1);
    matches[0]
}
fn check(r: &Json) {
    assert_eq!(rows(&r["cases"]).len(), 24);
    let mut selected = BTreeSet::new();
    for case in rows(&r["cases"]) {
        assert_eq!(case["independent_source_bindings_verified"], true);
        for stage in STAGES {
            let d = &case["states"][stage]["delivery"];
            assert_eq!(d["profile_domain"], json!([OFFERING, ICE]));
            assert_eq!(rows(&d["profile_snapshots"]).len(), 2);
            for snapshot in rows(&d["profile_snapshots"]) {
                assert_eq!(snapshot["verified"], true);
            }
            for k in ["original_methods_preserved", "jit_mode_preserved"] {
                assert_eq!(d[k], true);
            }
            for k in [
                "source_cfg_modified",
                "source_tables_mutated",
                "business_wrappers",
                "original_calculation_calls_captured",
            ] {
                assert_eq!(d[k], false);
            }
            assert_eq!(rows(&d["definitions"]).len(), 2);
            declaration(&d["definitions"][0], I, 30);
            declaration(&d["definitions"][1], II, 35);
            assert_eq!(rows(&d["recipient_definitions"]).len(), 2);
            for (definition, effect, count) in [
                (&d["recipient_definitions"][0], OFFERING, 1),
                (&d["recipient_definitions"][1], ICE, 2),
            ] {
                assert_eq!(definition["effect"], effect);
                assert_eq!(definition["support"], false);
                assert_eq!(definition["parts_present"], false);
                assert_eq!(definition["minion_list_present"], false);
                assert_eq!(rows(&definition["stat_sets"]).len(), count);
            }
            for c in rows(&d["contexts"]) {
                context(
                    c,
                    &format!("{}/{stage}/{}/{}", case["name"], c["effect"], c["mode"]),
                );
            }
            if !case["control"].is_null() {
                let control = &case["control"];
                let effect = control["effect"].as_str().unwrap();
                for mode in ["MAIN", "CALCS"] {
                    let found: Vec<_> = rows(&d["contexts"])
                        .iter()
                        .filter(|c| c["effect"] == effect && c["mode"] == mode)
                        .collect();
                    let sources = rows(&control["sources"]);
                    assert_eq!(found.len(), sources.len());
                    let mut origins = BTreeSet::new();
                    for (c, s) in found.iter().zip(sources) {
                        assert_eq!(c["source"]["position"], s["position"]);
                        assert_eq!(c["source"]["raw_level"], s["raw_level"]);
                        assert_eq!(c["source"]["raw_quality"], s["raw_quality"]);
                        assert!(origins.insert(c["source"]["source_ordinal"].as_u64().unwrap()));
                        let candidates = rows(&c["candidates"]);
                        assert_eq!(candidates.len(), usize::from(!control["winner"].is_null()));
                        if let Some(s) = candidates.first() {
                            assert_eq!(s["effect"], control["winner"]);
                            assert_eq!(s["origin"]["position"], control["winner_position"]);
                        }
                    }
                    let picked: Vec<_> = found.iter().filter(|c| c["selected"] == true).collect();
                    assert_eq!(picked.len(), 1);
                    let c = picked[0];
                    let selection = control[if mode == "MAIN" {
                        "main_occurrence"
                    } else {
                        "calcs_occurrence"
                    }]
                    .as_u64()
                    .unwrap() as usize;
                    assert_eq!(c["source"]["position"], sources[selection - 1]["position"]);
                    assert_eq!(c["queries"]["output_available"], true);
                    assert_eq!(
                        c["stat_set_index"],
                        control[if mode == "MAIN" {
                            "main_set"
                        } else {
                            "calcs_set"
                        }]
                    );
                    selected.insert((effect.to_owned(), c["stat_set_index"].as_u64().unwrap()));
                }
            }
        }
    }
    assert_eq!(
        selected,
        BTreeSet::from([
            (OFFERING.to_owned(), 1),
            (ICE.to_owned(), 1),
            (ICE.to_owned(), 2)
        ])
    );
    for (a, b) in (1..=5)
        .map(|i| {
            (
                format!("original-{i:02}"),
                format!("repeat-original-{i:02}"),
            )
        })
        .chain([
            ("pain-tier-i".into(), "repeat-pain-tier-i".into()),
            ("ice-tier-i".into(), "repeat-ice-tier-i".into()),
        ])
    {
        let a = named(r, &a);
        let b = named(r, &b);
        rows_same(&a["xml_sha256"], &b["xml_sha256"], "repeat XML");
        assert_eq!(
            json_evidence::first_difference(&a["states"], &b["states"], "independent-repeat"),
            None,
            "{}/{}",
            a["name"],
            b["name"]
        );
    }
    for stage in STAGES {
        for mode in ["MAIN", "CALCS"] {
            let get = |name: &str| -> &Json {
                rows(&named(r, name)["states"][stage]["delivery"]["contexts"])
                    .iter()
                    .find(|c| c["effect"] == OFFERING && c["mode"] == mode && c["selected"] == true)
                    .unwrap()
            };
            let baseline = get("pain-tier-i");
            let lower = get("pain-raw-level");
            let quality = get("pain-raw-quality");
            assert!(
                lower["queries"]["duration"]["effective_level"]
                    .as_u64()
                    .unwrap()
                    < baseline["queries"]["duration"]["effective_level"]
                        .as_u64()
                        .unwrap()
            );
            assert_eq!(
                quality["queries"]["duration"]["effective_level"],
                baseline["queries"]["duration"]["effective_level"]
            );
            assert!(
                quality["queries"]["duration"]["effective_quality"]
                    .as_f64()
                    .unwrap()
                    > baseline["queries"]["duration"]["effective_quality"]
                        .as_f64()
                        .unwrap()
            );
        }
    }
}
