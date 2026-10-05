//! Finite physical support contribution evidence; ground growth remains unresolved.
use super::magnified_area_support::{edit_gem, edit_group, focus, group_range, set_attr, template};
use super::*;
use std::collections::BTreeSet;

const TEST_NAME: &str =
    "encroaching_ground::encroaching_ground_delivery_uses_physical_ice_recipients";
const MODE: &str = "POE_ENCROACHING_GROUND_SOURCE_CHILD";
const OUTPUT: &str = "POE_ENCROACHING_GROUND_SOURCE_OUT";
const EXTRACTOR: &str = include_str!("physical_support_delivery.lua");
const PROFILE: &str = include_str!("encroaching_ground_delivery.lua");
const EFFECT: &str = "SupportEncroachingGroundPlayer";
const MAGNIFIED: &str = "SupportMagnifiedAreaPlayerTwo";
const RAPID: &str = "SupportRapidCastingPlayer";
const ICE: &str = "IceNovaPlayer";
const GROWTH: [&str; 2] = [
    "support_ground_effect_area_of_effect_+%_final_per_second",
    "support_ground_effect_area_of_effect_+%_final_per_second_max",
];

fn observer() -> String {
    format!(
        "local collect = (function()\n{EXTRACTOR}\nend)()\nreturn (function(collect)\n{PROFILE}\nend)(collect)"
    )
}

#[test]
#[ignore = "requires pinned original PoB; finite physical Encroaching Ground delivery in both JIT modes"]
fn encroaching_ground_delivery_uses_physical_ice_recipients() {
    super::physical_support::run_modes(
        super::physical_support::Witness {
            name: TEST_NAME,
            child_env: MODE,
            output_env: OUTPUT,
            default_output: "runs/owned-encroaching-ground-source-01",
            label: "Encroaching Ground",
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
    let delivery = observer();
    let mut cases = vec![];
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(digest(bytes), index["builds"][i]["xml_sha256"]);
        cases.push(super::physical_support::observe_physical(
            root,
            &format!("original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
            None,
            &delivery,
        ));
    }
    for (name, xml, control) in controls(std::str::from_utf8(&originals[4]).unwrap()) {
        cases.push(super::physical_support::observe_physical(
            root,
            &name,
            &xml,
            enabled,
            Some(control),
            &delivery,
        ));
    }
    for (i, bytes) in originals.iter().enumerate() {
        cases.push(super::physical_support::observe_physical(
            root,
            &format!("repeat-original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
            None,
            &delivery,
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
        "schema_version":1,"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":digest(delivery.as_bytes()),"observer_parts":[
            {"path":"crates/poe-optimizer-pob/tests/support/physical_support_delivery.lua","sha256":digest(EXTRACTOR.as_bytes())},
            {"path":"crates/poe-optimizer-pob/tests/support/encroaching_ground_delivery.lua","sha256":digest(PROFILE.as_bytes())}],
        "files":files.into_iter().map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})).collect::<Vec<_>>(),
        "original_sources":originals.iter().enumerate().map(|(i,b)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(b)})).collect::<Vec<_>>(),
        "business_wrappers":false,"source_tables_mutated":false,"source_cfg_modified":false,
        "native_build_parity":false,"native_inventory_authority":false,"ground_growth_formula_authority":false,
        "final_resource_cost_formula_authority":false,"reservation_formula_authority":false,"canonical_parity_lifecycle_selected":false,
        "purpose":"finite physical Ice Nova Encroaching Ground cost contribution and exact support-source correspondence; unmapped ground growth remains unresolved",
        "query_observation_kind":"diagnostic_original_method_read","original_calculation_calls_captured":false,
        "observation_order":"exact original per-channel record order; no arithmetic reordered",
        "numeric_tolerance":0,"lifecycle_stages":STAGES,"cases":cases
    });
    let bytes = serde_json::to_vec(&report).unwrap();
    let suffix = if enabled { "on" } else { "off" };
    let raw = out.join(format!("source-jit-{suffix}.raw.json"));
    fs::write(&raw, &bytes).unwrap();
    eprintln!(
        "Encroaching Ground evidence: {} bytes at {}",
        bytes.len(),
        raw.display()
    );
    assert!(
        bytes.len() <= 96 * 1024 * 1024,
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

fn controls(original: &str) -> Vec<(String, String, Json)> {
    let focused = focus(original, ICE, None, 1, 2);
    let source = template(original, ICE, EFFECT);
    let higher = set_attr(&source, "quality", "15");
    let mut out = vec![];
    for (name, xml, present, rapid, magnified, last) in [
        ("ice-focused", focused.clone(), true, true, true, false),
        (
            "ice-remove",
            edit_group(&focused, ICE, |g| edit_gem(g, EFFECT, |_| String::new())),
            false,
            true,
            true,
            false,
        ),
        (
            "ice-disable",
            edit_group(&focused, ICE, |g| {
                edit_gem(g, EFFECT, |gem| set_attr(gem, "enabled", "false"))
            }),
            false,
            true,
            true,
            false,
        ),
        (
            "duplicate-equal",
            edit_group(&focused, ICE, |g| {
                edit_gem(g, EFFECT, |_| format!("{source}{source}"))
            }),
            true,
            true,
            true,
            false,
        ),
        (
            "duplicate-higher",
            edit_group(&focused, ICE, |g| {
                edit_gem(g, EFFECT, |_| format!("{source}{higher}"))
            }),
            true,
            true,
            true,
            true,
        ),
        (
            "repeat-ice-focused",
            focused.clone(),
            true,
            true,
            true,
            false,
        ),
        (
            "ice-remove-rapid",
            edit_group(&focused, ICE, |g| edit_gem(g, RAPID, |_| String::new())),
            true,
            false,
            true,
            false,
        ),
        (
            "ice-encroaching-only",
            edit_group(&focused, ICE, |g| {
                let g = edit_gem(g, MAGNIFIED, |_| String::new());
                edit_gem(&g, RAPID, |_| String::new())
            }),
            true,
            false,
            false,
            false,
        ),
    ] {
        assert_ne!(xml, original);
        let (range, _) = group_range(&xml, ICE);
        let document = roxmltree::Document::parse(&xml[range]).unwrap();
        let saved: Vec<_> = document
            .root_element()
            .children()
            .filter(|n| n.has_tag_name("Gem"))
            .collect();
        let positions: Vec<_> = saved
            .iter()
            .enumerate()
            .filter(|(_, g)| g.attribute("skillId") == Some(EFFECT))
            .map(|(i, _)| i + 1)
            .collect();
        let winner = present.then(|| {
            if last {
                *positions.last().unwrap()
            } else {
                positions[0]
            }
        });
        out.push((name.to_owned(), xml, json!({"effect":ICE,"encroaching":present,"rapid":rapid,"magnified":magnified,
            "winner_position":winner,"winner_quality":if last {15}else{0},"main_set":1,"calcs_set":2})));
    }
    assert_eq!(out.len(), 8);
    out
}

fn check_absent_maps(maps: &Json) {
    assert_eq!(rows(maps).len(), 2);
    for (row, name) in rows(maps).iter().zip(GROWTH) {
        assert_eq!(row["stat"], name);
        assert_eq!(row["present"], false);
        assert!(row.get("value").is_none());
    }
}

fn check_definition(def: &Json) {
    assert_eq!(def["effect"], EFFECT);
    assert_eq!(def["mod_source"], format!("Skill:{EFFECT}"));
    assert_eq!(
        def["levels"],
        json!({"positions":[{"index":1,"value":{"levelRequirement":0,"manaMultiplier":10}}]})
    );
    let sets = rows(&def["stat_sets"]);
    assert_eq!(sets.len(), 1);
    assert_eq!(
        sets[0]["constants"],
        json!({"positions":[
            {"index":1,"value":{"positions":[{"index":1,"value":GROWTH[0]},{"index":2,"value":20}]}},
            {"index":2,"value":{"positions":[{"index":1,"value":GROWTH[1]},{"index":2,"value":100}]}}
        ]})
    );
    assert!(rows(&sets[0]["stats"]).is_empty());
    for field in ["quality_stats", "base_mods"] {
        assert!(sets[0][field].is_null() || rows(&sets[0][field]).is_empty());
    }
    assert!(rows(&def["add_types"]).is_empty());
    assert!(def["add_flags"].is_null() || rows(&def["add_flags"]).is_empty());
    check_absent_maps(&sets[0]["declared_maps"]);
}

fn check_channel(q: &Json, name: &str, expected: &[(&str, i64, u64)], label: &str) {
    let channel = &q["channels"][name];
    let raw = rows(&channel["raw_source_records"]);
    assert_eq!(raw.len(), expected.len(), "{label}/{name}/raw-count");
    for (i, (row, (effect, value, flags))) in raw.iter().zip(expected).enumerate() {
        assert_eq!(row["channel_index"], i + 1);
        assert_eq!(row["source_effect"], *effect);
        assert_eq!(
            row["ancestor_depth"], 1,
            "{label}/{name}/physical-base-store"
        );
        let record = json!({"name":name,"type":if name=="Speed" {"INC"}else{"MORE"},"value":value,
            "source":format!("Skill:{effect}"),"flags":flags,"keyword_flags":0,"tags":{}});
        assert!(
            row["record"] == record,
            "{label}/{name}/exact-record: {}",
            row["record"]
        );
    }
    let applied: Vec<_> = rows(&channel["applied"])
        .iter()
        .filter(|r| r.get("source_effect").is_some())
        .collect();
    assert_eq!(applied.len(), raw.len(), "{label}/{name}/applied-count");
    for (i, (row, source)) in applied.iter().zip(raw).enumerate() {
        assert_eq!(row["source_record_indices"], json!([i + 1]));
        assert_eq!(row["record"], source["record"]);
        assert_eq!(row["source_effect"], source["source_effect"]);
        assert_eq!(row["value"], source["record"]["value"]);
    }
    for row in rows(&channel["applied"])
        .iter()
        .filter(|r| r.get("source_effect").is_none())
    {
        assert!(rows(&row["source_record_indices"]).is_empty());
    }
}

fn check_context(context: &Json, label: &str) {
    assert_eq!(context["effect"], ICE, "{label}");
    assert_eq!(context["actor_is_player"], true);
    assert_eq!(context["source"]["exact_source_instance"], true);
    let candidates = rows(&context["candidates"]);
    let mut known = BTreeSet::new();
    for support in candidates {
        let effect = support["effect"].as_str().unwrap();
        assert!([EFFECT, MAGNIFIED, RAPID].contains(&effect));
        assert!(known.insert(effect), "{label}/one-retained-family-source");
        for (record, field) in [
            (support, "exact_definition"),
            (support, "accepted"),
            (&support["origin"], "exact_source_instance"),
            (&support["origin"], "enabled"),
        ] {
            assert_eq!(record[field], true, "{label}/{effect}/{field}");
        }
        assert_ne!(
            support["origin"]["source_ordinal"],
            context["source"]["source_ordinal"]
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
    assert_eq!(q["cast_flag_value"], 16);
    assert_eq!(
        q["query_observation_kind"],
        "diagnostic_original_method_read"
    );
    assert_eq!(q["original_calculation_call_captured"], false);
    assert_eq!(
        q["store_chain"],
        json!([
            {"depth":0,"kind":"ModList","base_skill_store":false,"actor_store":false},
            {"depth":1,"kind":"ModList","base_skill_store":true,"actor_store":false},
            {"depth":2,"kind":"ModDB","base_skill_store":false,"actor_store":true}
        ]),
        "{label}/exact-recipient-store-chain"
    );
    let disabled = q["skill_flags"]["disable"] == true;
    let mut cost = vec![];
    if !disabled {
        // Keep original support-list order, which also determines merge order.
        for support in candidates {
            match support["effect"].as_str().unwrap() {
                EFFECT => cost.push((EFFECT, 10, 0)),
                MAGNIFIED => cost.push((MAGNIFIED, 30, 0)),
                RAPID => (),
                _ => unreachable!(),
            }
        }
    }
    check_channel(q, "SupportManaMultiplier", &cost, label);
    let speed = if !disabled && known.contains(RAPID) {
        vec![(RAPID, 15, 16)]
    } else {
        vec![]
    };
    check_channel(q, "Speed", &speed, label);
    check_channel(q, "ReservationMultiplier", &[], label);
    check_channel(q, "ExtraSpirit", &[], label);
}

fn named<'a>(report: &'a Json, name: &str) -> &'a Json {
    let found: Vec<_> = rows(&report["cases"])
        .iter()
        .filter(|c| c["name"] == name)
        .collect();
    assert_eq!(found.len(), 1);
    found[0]
}

fn check(report: &Json) {
    assert_eq!(rows(&report["cases"]).len(), 18);
    let mut selected_sets = BTreeSet::new();
    for case in rows(&report["cases"]) {
        assert_eq!(case["independent_source_bindings_verified"], true);
        for stage in STAGES {
            let delivery = &case["states"][stage]["delivery"];
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
            assert_eq!(
                delivery["source_cost_precision"],
                json!({"stat":"SupportManaMultiplier","type":"MORE","digits":4})
            );
            assert_eq!(rows(&delivery["definitions"]).len(), 3);
            check_definition(&rows(&delivery["definitions"])[0]);
            check_absent_maps(&delivery["global_stat_maps"]);
            for context in rows(&delivery["contexts"]) {
                let label = format!(
                    "{}/{stage}/{}/{}",
                    case["name"], context["mode"], context["stat_set_index"]
                );
                check_context(context, &label);
            }
            if !case["control"].is_null() {
                for mode in ["MAIN", "CALCS"] {
                    let found: Vec<_> = rows(&delivery["contexts"])
                        .iter()
                        .filter(|c| c["mode"] == mode)
                        .collect();
                    assert_eq!(found.len(), 1);
                    let context = found[0];
                    let control = &case["control"];
                    assert_eq!(context["selected"], true);
                    assert_eq!(context["queries"]["output_available"], true);
                    assert_eq!(context["queries"]["output_is_selected_actor"], true);
                    assert_eq!(
                        context["stat_set_index"],
                        control[if mode == "MAIN" {
                            "main_set"
                        } else {
                            "calcs_set"
                        }]
                    );
                    selected_sets.insert(context["stat_set_index"].as_u64().unwrap());
                    for (effect, field) in [
                        (EFFECT, "encroaching"),
                        (MAGNIFIED, "magnified"),
                        (RAPID, "rapid"),
                    ] {
                        let rows: Vec<_> = rows(&context["candidates"])
                            .iter()
                            .filter(|c| c["effect"] == effect)
                            .collect();
                        assert_eq!(
                            rows.len(),
                            usize::from(control[field].as_bool().unwrap()),
                            "{}/{stage}/{mode}/{effect}",
                            case["name"]
                        );
                        if effect == EFFECT && !rows.is_empty() {
                            assert_eq!(rows[0]["origin"]["position"], control["winner_position"]);
                            assert_eq!(rows[0]["origin"]["raw_quality"], control["winner_quality"]);
                        }
                    }
                    let expected = match (
                        control["encroaching"].as_bool().unwrap(),
                        control["magnified"].as_bool().unwrap(),
                    ) {
                        (true, true) => 1.43,
                        (true, false) => 1.1,
                        (false, true) => 1.3,
                        (false, false) => 1.0,
                    };
                    assert_eq!(
                        context["queries"]["cost_factor"], expected,
                        "{}/{stage}/{mode}/original-cost-subtotal",
                        case["name"]
                    );
                }
            }
        }
    }
    assert_eq!(selected_sets, BTreeSet::from([1, 2]));
    for stage in STAGES {
        let contexts = rows(&named(report, "original-05")["states"][stage]["delivery"]["contexts"]);
        assert_eq!(contexts.len(), 2);
        for context in contexts {
            assert_eq!(
                rows(&context["candidates"])
                    .iter()
                    .map(|c| c["effect"].as_str().unwrap())
                    .collect::<Vec<_>>(),
                vec![EFFECT, MAGNIFIED, RAPID]
            );
            assert_eq!(context["queries"]["cost_factor"], 1.43);
        }
    }
    for (a, b) in (1..=5)
        .map(|i| {
            (
                format!("original-{i:02}"),
                format!("repeat-original-{i:02}"),
            )
        })
        .chain([("ice-focused".into(), "repeat-ice-focused".into())])
    {
        let a = named(report, &a);
        let b = named(report, &b);
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
