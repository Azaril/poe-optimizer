//! Manual nonphysical occurrences through the shared complete-source lifecycle.
use super::*;

const TEST: &str = "djinn_families::complete_manual_djinn_occurrences_preserve_source_actions";
const CHILD: &str = "POE_DJINN_ACTOR_ACTION_SOURCE_CHILD";
// Name, source edits, group edits, child XML, duplicate source, expected failure.
type SourceInputControl<'a> = (
    &'a str,
    Vec<AttributeEdit<'a>>,
    Vec<AttributeEdit<'a>>,
    Option<&'a str>,
    bool,
    bool,
);
struct Family {
    name: &'static str,
    game: &'static str,
    effect: &'static str,
    command: &'static str,
    actor: &'static str,
    children: &'static [&'static str],
    labels: &'static [&'static [&'static str]],
    scopes: &'static [&'static [&'static str]],
    original_one: u64,
    original_five: &'static [(u64, u64)],
}
const FAMILIES: [Family; 2] = [
    Family {
        name: "sand",
        game: "Metadata/Items/Gem/SkillGemAscendancySummonSandDjinn",
        effect: "SummonSandDjinnPlayer",
        command: "CommandSandDjinnKnifeThrowPlayer",
        actor: "SandDjinn",
        children: &[
            "KnifeThrowSandDjinn",
            "ExplosiveTeleportSandDjinn",
            "HandSlamSandDjinn",
        ],
        labels: &[
            &["Projectile", "Eruption"],
            &["Kelari's Deception", "Hidden"],
            &["Kelari's Judgment"],
        ],
        scopes: &[
            &["djinn_knife_throw_statset_0", "djinn_knife_throw_statset_1"],
            &["djinn_explosive_teleport", "djinn_explosive_teleport"],
            &["djinn_hand_slam"],
        ],
        original_one: 138,
        original_five: &[(198, 3), (216, 4), (259, 5), (300, 6), (364, 1)],
    },
    Family {
        name: "water",
        game: "Metadata/Items/Gem/SkillGemAscendancySummonWaterDjinn",
        effect: "SummonWaterDjinnPlayer",
        command: "CommandWaterDjinnBubblePlayer",
        actor: "WaterDjinn",
        children: &[
            "WaterBubbleWaterDjinn",
            "ChilledGroundBurstWaterDjinn",
            "ESRechargeForceRestartWaterDjinn",
            "ChilledGroundOasisConvertWaterDjinn",
            "PassiveTriggeredManaWaveWaterDjinn",
        ],
        labels: &[
            &["Navira's Embrace"],
            &["Navira's Fracturing"],
            &["Navira's Well"],
            &["Navira's Oasis"],
            &["Navira's Calming"],
        ],
        scopes: &[
            &["djinn_bubble"],
            &["djinn_ground_burst"],
            &["djinn_force_recharge"],
            &["djinn_oasis"],
            &["djinn_triggered_wave"],
        ],
        original_one: 188,
        original_five: &[(229, 4), (272, 5), (294, 6), (357, 1)],
    },
];

#[test]
#[ignore = "requires optional complete pinned PoB; run explicitly for manual Djinn source/input correspondence"]
fn complete_manual_djinn_occurrences_preserve_source_actions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-djinn-actor-action-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        run_child(&root, &out, mode == "on");
        return;
    }
    run_modes(&root, &out, TEST, CHILD);
}

fn observe(root: &Path, name: &str, xml: &str, enabled: bool, failure: bool) -> Json {
    let families: Vec<_> = FAMILIES.iter().map(|f| (f.game, f.effect)).collect();
    match observe_family_inputs(root, name, xml, enabled, &families, true) {
        Ok(mut result) => {
            assert!(
                !failure,
                "{name}: expected a complete-source failure, got a build"
            );
            result["status"] = json!("complete");
            result["attribute_dispositions"] = attribute_dispositions(&result["states"]["fresh"]);
            result
        }
        Err(error) => {
            let message = error.to_string();
            eprintln!("{name}: original source failure: {message}");
            assert!(
                failure,
                "{name}: unexpected complete-source failure: {message}"
            );
            let site = if message.contains("Classes/SkillsTab.lua:388: table index is nil") {
                "SkillsTab.LoadSkill:malformed-map-key"
            } else if message.contains(
                "Modules/CalcTools.lua:178: attempt to index local 'statSet' (a nil value)",
            ) {
                "CalcTools.buildSkillInstanceStats:unavailable-stat-set"
            } else {
                panic!(
                    "{name}: unreviewed failure, not an admitted expected source error: {message}"
                )
            };
            json!({"name":name,"xml_sha256":digest(xml.as_bytes()),"status":"source_failure",
                "source_error_site":site,"native_inventory_authority":false})
        }
    }
}

fn attribute_dispositions(state: &Json) -> Json {
    let result: Vec<_> = rows(&state["saved"]).iter().map(|row| {
        let attributes: Vec<_> = row["attributes"].as_object().unwrap().keys().map(|name| {
            let role = match name.as_str() {
                "gemId" | "variantId" | "skillId" | "nameSpec" => "source_identity",
                "level" | "quality" => "authored_raw_parameter",
                "enabled" => "occurrence_activation",
                "corrupted" | "corruptLevel" => "source_preparation_field_neutral_guard_only",
                "count" | "enableGlobal1" | "enableGlobal2" => "deferred_preset_usage",
                "skillMinion" | "skillMinionCalcs" | "skillMinionSkill" | "skillMinionSkillCalcs" => "reference_action_selection",
                "statSetIndex" | "statSetIndexCalcs" => "overwritten_legacy_header",
                _ => "unreviewed_pending",
            };
            json!({"name":name,"role":role})
        }).collect();
        let group: Vec<_> = row["group_attributes"].as_object().unwrap().keys().map(|name| {
            let role = match name.as_str() {
                "source" => "manual_source_guard",
                "enabled" => "occurrence_activation",
                "mainActiveSkill" | "mainActiveSkillCalcs" => "player_effect_reference_selection",
                "groupCount" | "includeInFullDPS" => "deferred_preset_usage",
                "label" => "source_provenance",
                _ => "unreviewed_pending",
            };
            json!({"name":name,"role":role})
        }).collect();
        json!({"source_ordinal":row["source_ordinal"],"attributes":attributes,"group_attributes":group})
    }).collect();
    json!(result)
}

fn manual(node: roxmltree::Node<'_, '_>) -> bool {
    matches!(
        node.parent().and_then(|p| p.attribute("source")),
        None | Some("" | "nil")
    )
}
fn edit_manual(
    xml: &str,
    preset: u64,
    family: &Family,
    values: &[AttributeEdit<'_>],
    group_values: &[AttributeEdit<'_>],
    children: Option<&str>,
    duplicate: bool,
) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let gem = doc
        .descendants()
        .find(|n| {
            n.has_tag_name("Gem")
                && n.attribute("gemId") == Some(family.game)
                && manual(*n)
                && n.ancestors().any(|s| {
                    s.has_tag_name("SkillSet")
                        && s.attribute("id").and_then(|v| v.parse::<u64>().ok()) == Some(preset)
                })
        })
        .unwrap();
    assert!(gem.children().all(|n| !n.is_element()));
    let mut edits = vec![];
    if !group_values.is_empty() {
        let group = gem.parent().unwrap();
        let start = group.range().start;
        edits.push((
            start..start + xml[start..].find('>').unwrap() + 1,
            rewrite(group, group_values),
        ));
    }
    if !values.is_empty() || children.is_some() || duplicate {
        let mut text = format!("{}{}</Gem>", rewrite(gem, values), children.unwrap_or(""));
        if duplicate {
            text.push_str(&format!(
                "{}</Gem>",
                rewrite(
                    gem,
                    &[
                        ("level", Some("19")),
                        ("quality", Some("7")),
                        ("count", Some("3")),
                        ("skillMinionSkill", Some("2")),
                        ("skillMinionSkillCalcs", Some("2"))
                    ]
                )
            ));
        }
        edits.push((gem.range(), text));
    }
    replace(xml, edits)
}
fn focus_manual(xml: &str, preset: u64, family: &Family, runtime: Option<u64>) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let set = doc
        .descendants()
        .find(|n| {
            n.has_tag_name("SkillSet")
                && n.attribute("id").and_then(|s| s.parse::<u64>().ok()) == Some(preset)
        })
        .unwrap();
    let index = set
        .children()
        .filter(|n| n.has_tag_name("Skill"))
        .position(|g| {
            g.children().any(|n| {
                n.has_tag_name("Gem") && n.attribute("gemId") == Some(family.game) && manual(n)
            })
        })
        .unwrap()
        + 1;
    focus_physical(
        xml,
        preset,
        family.game,
        Some(runtime.unwrap_or(index as u64)),
    )
}
fn maps(effect: &str, child: usize, main: usize, calcs: usize) -> String {
    format!(
        r#"<MinionSkillIndexLookup grantedEffect="{effect}"><MinionSkillIndexMap skillIndex="{child}" statSetIndex="{main}"/></MinionSkillIndexLookup><MinionSkillIndexLookupCalcs grantedEffect="{effect}"><MinionSkillIndexMap skillIndex="{child}" statSetIndex="{calcs}"/></MinionSkillIndexLookupCalcs>"#
    )
}
fn selected_family<'a>(state: &'a Json, family: &Family) -> Vec<&'a Json> {
    rows(&state["saved"])
        .iter()
        .filter(|r| r["selected"] == true && r["attributes"]["gemId"] == family.game)
        .collect()
}

fn run_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let fixtures = root.join("tests/fixtures/builds/breadth-20260908");
    let manifest: Json =
        serde_json::from_slice(&fs::read(fixtures.join("index.json")).unwrap()).unwrap();
    let originals: Vec<_> = (1..=5)
        .map(|i| fs::read(fixtures.join(format!("build-{i:02}.xml"))).unwrap())
        .collect();
    let catalog = SkillIdentityCatalog::new(
        serde_json::from_slice(
            &fs::read(root.join("data/owned/poe2/3887ae68/import/skill-identities.json")).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        digest_owned(
            "owned-skill-source-catalog-v1",
            catalog.data(),
            64 * 1024 * 1024
        )
        .unwrap()
        .to_string(),
        CATALOG_DIGEST
    );
    let identities: Vec<_> = FAMILIES
        .iter()
        .map(|f| {
            let row = catalog
                .data()
                .gems
                .iter()
                .find(|r| r.game_id == f.game)
                .unwrap();
            assert_eq!(row.primary_effect_id, f.effect);
            assert_eq!(row.effect_list, [f.effect, f.command]);
            assert_eq!(row.additional_effects, [f.command]);
            row
        })
        .collect();
    let mut cases = vec![];
    for (index, bytes) in originals.iter().enumerate() {
        assert_eq!(
            digest(bytes),
            manifest["builds"][index]["xml_sha256"].as_str().unwrap()
        );
        cases.push(observe(
            root,
            &format!("original-{:02}", index + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
            false,
        ));
    }
    let original = std::str::from_utf8(&originals[4]).unwrap();
    let mut runtime = std::collections::BTreeMap::new();
    for preset in [3, 4, 5, 6, 1] {
        let activation = observe(
            root,
            &format!("preset-{preset}-activation"),
            &focus_manual(original, preset, &FAMILIES[0], None),
            enabled,
            false,
        );
        for f in &FAMILIES {
            if f.original_five.iter().any(|(_, p)| *p == preset) {
                let r = selected_family(&activation["states"]["fresh"], f);
                assert_eq!(r.len(), 1);
                runtime.insert((f.name, preset), r[0]["group"].as_u64().unwrap());
            }
        }
        cases.push(activation);
    }
    for (case, xml) in [
        (1, std::str::from_utf8(&originals[0]).unwrap()),
        (5, original),
    ] {
        for f in &FAMILIES {
            let presets: Vec<_> = if case == 1 {
                vec![1]
            } else {
                f.original_five.iter().map(|(_, p)| *p).collect()
            };
            for preset in presets {
                let group = if case == 1 {
                    rows(&cases[0]["states"]["fresh"]["saved"])
                        .iter()
                        .find(|r| r["attributes"]["gemId"] == f.game)
                        .unwrap()["group"]
                        .as_u64()
                        .unwrap()
                } else {
                    runtime[&(f.name, preset)]
                };
                let last = f.children.len().to_string();
                let changed = edit_manual(
                    xml,
                    preset,
                    f,
                    &[
                        ("skillMinionSkill", Some("1")),
                        ("skillMinionSkillCalcs", Some(&last)),
                    ],
                    &[
                        ("mainActiveSkill", Some("1")),
                        ("mainActiveSkillCalcs", Some("1")),
                    ],
                    None,
                    false,
                );
                cases.push(observe(
                    root,
                    &format!(
                        "original-{case:02}-{}-preset-{preset}-first-main-last-calcs",
                        f.name
                    ),
                    &focus_manual(&changed, preset, f, Some(group)),
                    enabled,
                    false,
                ));
            }
        }
    }
    for f in &FAMILIES {
        let focused = focus_manual(original, 4, f, Some(runtime[&(f.name, 4)]));
        let last = f.children.len().to_string();
        let set_last = f.labels[0].len();
        let main_last = maps(f.effect, 1, set_last, 1);
        let calcs_last = maps(f.effect, 1, 1, set_last);
        let exact = maps(f.effect, 1, 1, 1);
        let duplicate = format!(
            r#"<MinionSkillIndexLookup grantedEffect="{}"><MinionSkillIndexMap skillIndex="1" statSetIndex="{set_last}"/><MinionSkillIndexMap skillIndex="1" statSetIndex="1"/></MinionSkillIndexLookup>"#,
            f.effect
        );
        let foreign = maps("unreviewed-source-effect", 1, 1, 1);
        let unknown = maps(f.effect, 999, 1, 1);
        let malformed = format!(
            r#"<MinionSkillIndexLookup grantedEffect="{}"><MinionSkillIndexMap skillIndex="bad" statSetIndex="1"/></MinionSkillIndexLookup>"#,
            f.effect
        );
        let invalid = maps(f.effect, 1, 999, 1);
        let controls: Vec<SourceInputControl<'_>> = vec![
            (
                "missing-selectors",
                vec![
                    ("skillMinion", None),
                    ("skillMinionCalcs", None),
                    ("skillMinionSkill", None),
                    ("skillMinionSkillCalcs", None),
                ],
                vec![],
                None,
                false,
                false,
            ),
            (
                "unknown-actors",
                vec![
                    ("skillMinion", Some("unreviewed-main")),
                    ("skillMinionCalcs", Some("unreviewed-calcs")),
                ],
                vec![],
                None,
                false,
                false,
            ),
            (
                "malformed-child-selectors",
                vec![
                    ("skillMinionSkill", Some("bad")),
                    ("skillMinionSkillCalcs", Some("bad")),
                ],
                vec![],
                None,
                false,
                false,
            ),
            (
                "clamped-child-selectors",
                vec![
                    ("skillMinionSkill", Some("999")),
                    ("skillMinionSkillCalcs", Some("0")),
                ],
                vec![],
                None,
                false,
                false,
            ),
            (
                "last-main",
                vec![
                    ("skillMinionSkill", Some(&last)),
                    ("skillMinionSkillCalcs", Some("1")),
                ],
                vec![],
                None,
                false,
                false,
            ),
            (
                "last-calcs",
                vec![
                    ("skillMinionSkill", Some("1")),
                    ("skillMinionSkillCalcs", Some(&last)),
                ],
                vec![],
                None,
                false,
                false,
            ),
            (
                "main-last-statset",
                vec![],
                vec![],
                Some(&main_last),
                false,
                false,
            ),
            (
                "calcs-last-statset",
                vec![],
                vec![],
                Some(&calcs_last),
                false,
                false,
            ),
            (
                "explicit-first-maps",
                vec![],
                vec![],
                Some(&exact),
                false,
                false,
            ),
            (
                "duplicate-map-key",
                vec![],
                vec![],
                Some(&duplicate),
                false,
                false,
            ),
            (
                "foreign-effect-map",
                vec![],
                vec![],
                Some(&foreign),
                false,
                false,
            ),
            (
                "unknown-child-map",
                vec![],
                vec![],
                Some(&unknown),
                false,
                false,
            ),
            (
                "malformed-map-key",
                vec![],
                vec![],
                Some(&malformed),
                false,
                true,
            ),
            (
                "invalid-statset",
                vec![],
                vec![],
                Some(&invalid),
                false,
                true,
            ),
            ("duplicate-manual-source", vec![], vec![], None, true, false),
            (
                "disabled-source",
                vec![("enabled", Some("false"))],
                vec![],
                None,
                false,
                false,
            ),
            (
                "disabled-group",
                vec![],
                vec![("enabled", Some("false"))],
                None,
                false,
                false,
            ),
            (
                "corrupted-true",
                vec![("corrupted", Some("true"))],
                vec![],
                None,
                false,
                false,
            ),
            (
                "corruption-fraction",
                vec![("corruptLevel", Some("0.25"))],
                vec![],
                None,
                false,
                false,
            ),
            (
                "count-three",
                vec![("count", Some("3"))],
                vec![],
                None,
                false,
                false,
            ),
            (
                "group-zero-override",
                vec![("count", Some("3"))],
                vec![("groupCount", Some("0"))],
                None,
                false,
                false,
            ),
            (
                "global-one-false",
                vec![("enableGlobal1", Some("false"))],
                vec![],
                None,
                false,
                false,
            ),
            (
                "global-two-false",
                vec![("enableGlobal2", Some("false"))],
                vec![],
                None,
                false,
                false,
            ),
            (
                "full-dps",
                vec![("count", Some("3"))],
                vec![("includeInFullDPS", Some("true"))],
                None,
                false,
                false,
            ),
            (
                "command-main",
                vec![],
                vec![
                    ("mainActiveSkill", Some("2")),
                    ("mainActiveSkillCalcs", Some("1")),
                ],
                None,
                false,
                false,
            ),
            (
                "command-calcs",
                vec![],
                vec![
                    ("mainActiveSkill", Some("1")),
                    ("mainActiveSkillCalcs", Some("2")),
                ],
                None,
                false,
                false,
            ),
            (
                "unknown-input-field",
                vec![("unreviewedInput", Some("7"))],
                vec![],
                None,
                false,
                false,
            ),
        ];
        for (name, values, group, children, copy, failure) in controls {
            cases.push(observe(
                root,
                &format!("{}-{name}", f.name),
                &edit_manual(&focused, 4, f, &values, &group, children, copy),
                enabled,
                failure,
            ));
        }
        let archived = f.original_five.iter().find(|(_, p)| *p != 4).unwrap().1;
        cases.push(observe(
            root,
            &format!("{}-archived-only", f.name),
            &edit_manual(
                &focused,
                archived,
                f,
                &[
                    ("skillMinionSkill", Some(&last)),
                    ("level", Some("19")),
                    ("quality", Some("7")),
                ],
                &[],
                None,
                false,
            ),
            enabled,
            false,
        ));
        let second = maps(f.effect, 2, f.labels[1].len(), 1);
        cases.push(observe(
            root,
            &format!("{}-second-child-main-last-set", f.name),
            &edit_manual(
                &focused,
                4,
                f,
                &[
                    ("skillMinionSkill", Some("2")),
                    ("skillMinionSkillCalcs", Some("3")),
                ],
                &[],
                Some(&second),
                false,
            ),
            enabled,
            false,
        ));
        if f.name == "water" {
            cases.push(observe(
                root,
                "water-fourth-main-fifth-calcs",
                &edit_manual(
                    &focused,
                    4,
                    f,
                    &[
                        ("skillMinionSkill", Some("4")),
                        ("skillMinionSkillCalcs", Some("5")),
                    ],
                    &[],
                    None,
                    false,
                ),
                enabled,
                false,
            ));
        }
    }
    cases.push(observe(
        root,
        "repeat-original-05",
        original,
        enabled,
        false,
    ));
    let pins = [
        "src/HeadlessWrapper.lua",
        "src/Modules/Build.lua",
        "src/Classes/CalcsTab.lua",
        "src/Classes/SkillsTab.lua",
        "src/Modules/Data.lua",
        "src/Modules/CalcSetup.lua",
        "src/Modules/CalcActiveSkill.lua",
        "src/Modules/CalcTools.lua",
        "src/Modules/Calcs.lua",
        "src/Modules/CalcPerform.lua",
        "src/Data/Minions.lua",
        "src/Data/Skills/minion.lua",
        "src/Data/Skills/other.lua",
        "src/Data/Gems.lua",
    ];
    let report = json!({"source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","manifest_sha256":pinned::manifest_sha256(),"catalog_digest":CATALOG_DIGEST,
        "source_identities":identities,"business_wrappers":false,"native_inventory_authority":false,"native_build_parity":false,
        "native_supported_property_authority":false,"canonical_parity_lifecycle_selected":false,
        "lifecycle_stages":["fresh","rebuilt_once","rebuilt_twice"],
        "files":pins.map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()})),
        "original_sources":originals.iter().enumerate().map(|(i,b)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(b)})).collect::<Vec<_>>(),"cases":cases});
    // The Direct report includes exact support-source provenance for both
    // supplied effects; omit formatting whitespace, never observed facts.
    let bytes = serde_json::to_vec(&report).unwrap();
    assert!(
        bytes.len() <= 64 * 1024 * 1024,
        "compact source report exceeds its bound: {} bytes",
        bytes.len()
    );
    fs::write(
        out.join(if enabled {
            "source-jit-on.json"
        } else {
            "source-jit-off.json"
        }),
        bytes,
    )
    .unwrap();
    check(&report);
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(
            &fs::read(fixtures.join(format!("build-{:02}.xml", i + 1))).unwrap(),
            bytes
        );
    }
}

fn check(report: &Json) {
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 81);
    assert_eq!(
        cases
            .iter()
            .filter(|c| c["status"] == "source_failure")
            .count(),
        4
    );
    let mut seen = std::collections::BTreeSet::new();
    for case in cases.iter().filter(|c| c["status"] == "complete") {
        let revision = case["states"]["fresh"]["output_revision"].as_u64().unwrap();
        for (stage_index, stage) in ["fresh", "rebuilt_once", "rebuilt_twice"]
            .into_iter()
            .enumerate()
        {
            let s = &case["states"][stage];
            assert_eq!(s["output_revision"], revision + stage_index as u64);
            assert_eq!(s["output_lifecycle"]["module_reloads"], 2);
            assert_eq!(s["build_flag"], false);
            for flag in [
                "source_methods_preserved",
                "loader_observer_removed",
                "requested_jit_mode_verified",
                "exact_source_objects",
                "source_objects_preserved_across_stages",
                "saved_inputs_preserved",
                "selected_state_preserved",
                "reported_outputs_preserved",
            ] {
                assert_eq!(s[flag], true, "{} {stage} {flag}", case["name"]);
            }
            for r in rows(&s["saved"]) {
                let family = FAMILIES
                    .iter()
                    .find(|f| r["attributes"]["gemId"] == f.game)
                    .unwrap();
                assert_eq!(r["source_kind"], "manual_direct");
                assert_eq!(r["resolved_additional_count"], 1);
                assert_eq!(
                    rows(&r["effects"])
                        .iter()
                        .map(|v| v["id"].as_str().unwrap())
                        .collect::<Vec<_>>(),
                    [family.effect, family.command]
                );
                for mode in ["MAIN", "CALCS"] {
                    assert!(rows(&r[mode]).len() <= 1);
                    if r["selected"] == false {
                        assert!(
                            rows(&r[mode]).is_empty(),
                            "archived source is not evaluated"
                        );
                    }
                    for a in rows(&r[mode]) {
                        assert_eq!(a["exact_source_object"], true);
                        assert_eq!(a["source_ordinal"], r["source_ordinal"]);
                        assert_eq!(a["minion_choices"], json!([family.actor]));
                        assert_eq!(
                            a["actor_selector_field"],
                            if mode == "CALCS" && a["is_main_skill"] == true {
                                "skillMinionCalcs"
                            } else {
                                "skillMinion"
                            }
                        );
                        assert_eq!(
                            a["child_selector_field"],
                            if mode == "CALCS" {
                                "skillMinionSkillCalcs"
                            } else {
                                "skillMinionSkill"
                            }
                        );
                        assert_eq!(a["actor"]["type"], family.actor);
                        // This is the pinned source's clamp, applied to the raw
                        // saved preference. The source also writes it back.
                        let field = if mode == "CALCS" {
                            "skillMinionSkillCalcs"
                        } else {
                            "skillMinionSkill"
                        };
                        let expected = r["attributes"][field]
                            .as_str()
                            .and_then(|s| s.parse::<usize>().ok())
                            .unwrap_or(1)
                            .clamp(1, family.children.len());
                        assert_eq!(
                            a["actor"]["selected_child_index"], expected,
                            "{} {stage} {mode}",
                            case["name"]
                        );
                        assert_eq!(a["child_selector_value"], expected);
                        let children = rows(&a["actor"]["children"]);
                        assert_eq!(children.len(), family.children.len());
                        for (i, c) in children.iter().enumerate() {
                            assert_eq!(c["effect"], family.children[i]);
                            assert_eq!(c["exact_actor"], true);
                            assert_eq!(c["exact_summon"], true);
                            let sets = rows(&c["stat_sets"]);
                            assert_eq!(sets.len(), family.labels[i].len());
                            for (j, set) in sets.iter().enumerate() {
                                assert_eq!(set["index"], j + 1);
                                assert_eq!(set["label"], family.labels[i][j]);
                                assert_eq!(set["stat_description_scope"], family.scopes[i][j]);
                            }
                            seen.insert((family.name, i));
                        }
                    }
                }
            }
            check_control(case, s, stage, cases);
            for g in rows(&s["direct"]["runtime_groups"]) {
                for source in rows(&g["sources"]) {
                    let family = FAMILIES
                        .iter()
                        .find(|f| source["game_id"] == f.game)
                        .unwrap();
                    let effects = rows(&source["effects"]);
                    assert_eq!(effects.len(), 2);
                    assert_eq!(effects[1]["id"], family.command);
                    let sets = rows(&effects[1]["stat_sets"]);
                    assert_eq!(sets.len(), 1);
                    assert_eq!(sets[0]["label"], "Command");
                    assert_eq!(sets[0]["stat_description_scope"], "skill_stat_descriptions");
                }
                for mode in ["MAIN", "CALCS"] {
                    for a in rows(&g[mode]) {
                        assert_eq!(a["exact_source_object"], true);
                        assert_eq!(a["minion_present"], a["effect_index"] == 1);
                        if a["source_kind"] == "allocated" {
                            assert!(a["manual_source_ordinal"].is_null());
                            assert_eq!(a["source_node_exact"], true);
                            assert!(rows(&a["supports"]["candidates"]).is_empty());
                        }
                    }
                }
            }
        }
    }
    assert_eq!(seen.len(), 8);
    for (case, counts) in [
        (0, [1, 1]),
        (1, [0, 0]),
        (2, [0, 0]),
        (3, [0, 0]),
        (4, [5, 4]),
    ] {
        for row in rows(&cases[case]["attribute_dispositions"]) {
            for field in ["attributes", "group_attributes"] {
                assert!(
                    rows(&row[field])
                        .iter()
                        .all(|r| r["role"] != "unreviewed_pending"),
                    "original source field needs an explicit disposition"
                );
            }
        }
        for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
            for (f, count) in FAMILIES.iter().zip(counts) {
                let found: Vec<_> = rows(&cases[case]["states"][stage]["saved"])
                    .iter()
                    .filter(|r| r["attributes"]["gemId"] == f.game)
                    .collect();
                assert_eq!(found.len(), count);
                let expected: Vec<_> = if case == 0 {
                    vec![f.original_one]
                } else {
                    f.original_five.iter().map(|(s, _)| *s).collect()
                };
                if count > 0 {
                    assert_eq!(
                        found
                            .iter()
                            .map(|r| r["source_ordinal"].as_u64().unwrap())
                            .collect::<Vec<_>>(),
                        expected
                    );
                    for row in found {
                        assert_eq!(row["loaded"]["level"], 20);
                        assert_eq!(row["loaded"]["quality"], 0);
                        assert_eq!(row["loaded"]["corrupted"], false);
                        assert_eq!(row["loaded"]["corruption_level"], 0);
                        assert_eq!(row["loaded"]["count"], 1);
                        assert_eq!(row["loaded"]["global_1"], true);
                        assert_eq!(row["loaded"]["global_2"], true);
                    }
                }
            }
        }
    }
    assert_eq!(cases.last().unwrap()["states"], cases[4]["states"]);
    assert_eq!(
        cases.last().unwrap()["source_joins"],
        cases[4]["source_joins"]
    );
}

fn check_control(case: &Json, state: &Json, stage: &str, cases: &[Json]) {
    let name = case["name"].as_str().unwrap();
    if name.starts_with("original-")
        || name == "repeat-original-05"
        || name.ends_with("-activation")
    {
        if name.contains("-first-main-last-calcs") {
            let family = FAMILIES
                .iter()
                .find(|f| name.contains(&format!("-{}-", f.name)))
                .unwrap();
            let selected = selected_family(state, family);
            assert_eq!(selected.len(), 1);
            for (mode, child) in [("MAIN", 1), ("CALCS", family.children.len())] {
                let actions = rows(&selected[0][mode]);
                assert_eq!(actions.len(), 1, "{name} {stage} {mode}");
                assert_eq!(actions[0]["is_main_skill"], true, "{name} {stage} {mode}");
                assert_eq!(actions[0]["actor"]["selected_child_index"], child);
                assert_eq!(state["selectors"][mode]["effect"], family.effect);
            }
        }
        return;
    }
    let family = FAMILIES
        .iter()
        .find(|f| name.starts_with(&format!("{}-", f.name)))
        .unwrap();
    let control = name
        .strip_prefix(family.name)
        .unwrap()
        .strip_prefix('-')
        .unwrap();
    let selected = selected_family(state, family);
    assert_eq!(
        selected.len(),
        if control == "duplicate-manual-source" {
            2
        } else {
            1
        },
        "{name} {stage}"
    );
    let first = selected[0];
    assert_eq!(state["selection"]["skills"], 4);
    for mode in ["MAIN", "CALCS"] {
        let actions = rows(&first[mode]);
        // A disabled group may still be the explicitly selected main group.
        // Global preferences are observed independently of actual availability.
        if control == "disabled-source" {
            assert!(actions.is_empty(), "{name} {stage} {mode}");
        } else if !matches!(
            control,
            "disabled-group" | "global-one-false" | "global-two-false"
        ) {
            assert_eq!(actions.len(), 1, "{name} {stage} {mode}");
        }
        if let Some(action) = actions.first() {
            let command = (mode == "MAIN" && control == "command-main")
                || (mode == "CALCS" && control == "command-calcs");
            if !matches!(
                control,
                "disabled-group" | "global-one-false" | "global-two-false"
            ) {
                assert_eq!(action["is_main_skill"], !command, "{name} {stage} {mode}");
                assert_eq!(
                    state["selectors"][mode]["effect"],
                    if command {
                        family.command
                    } else {
                        family.effect
                    }
                );
            }
            for (i, child) in rows(&action["actor"]["children"]).iter().enumerate() {
                let expected_set = if i == 0
                    && ((control == "main-last-statset" && mode == "MAIN")
                        || (control == "calcs-last-statset" && mode == "CALCS"))
                {
                    family.labels[0].len()
                } else if i == 1 && control == "second-child-main-last-set" && mode == "MAIN" {
                    family.labels[1].len()
                } else {
                    1
                };
                assert_eq!(
                    child["stat_set"]["index"],
                    expected_set,
                    "{name} {stage} {mode} child{}",
                    i + 1
                );
                assert_eq!(child["stat_set"]["declared_table_identity"], true);
            }
        }
        if control == "duplicate-manual-source" {
            let second = selected[1];
            assert_ne!(second["source_ordinal"], first["source_ordinal"]);
            assert_ne!(second["index"], first["index"]);
            assert_eq!(rows(&second[mode]).len(), 1);
            assert_eq!(rows(&second[mode])[0]["actor"]["selected_child_index"], 2);
            assert_eq!(second["loaded"]["level"], 19);
            assert_eq!(second["loaded"]["quality"], 7);
            assert_eq!(second["loaded"]["count"], 3);
            assert_eq!(first["loaded"]["level"], 20);
            assert_eq!(first["loaded"]["quality"], 0);
            assert_eq!(first["loaded"]["count"], 1);
        }
    }
    match control {
        "disabled-source" => assert_eq!(first["loaded"]["enabled"], false),
        "disabled-group" => assert_eq!(first["group_state"]["enabled"], false),
        "corrupted-true" => assert_eq!(first["loaded"]["corrupted"], true),
        "corruption-fraction" => assert_eq!(first["loaded"]["corruption_level"], 0.25),
        "count-three" | "full-dps" | "group-zero-override" => {
            assert_eq!(first["loaded"]["count"], 3)
        }
        "global-one-false" => assert_eq!(first["loaded"]["global_1"], false),
        "global-two-false" => assert_eq!(first["loaded"]["global_2"], false),
        "unknown-input-field" => {
            assert_eq!(first["attributes"]["unreviewedInput"], "7");
            let row = rows(&case["attribute_dispositions"])
                .iter()
                .find(|r| r["source_ordinal"] == first["source_ordinal"])
                .unwrap();
            assert!(
                rows(&row["attributes"])
                    .iter()
                    .any(|r| r["name"] == "unreviewedInput" && r["role"] == "unreviewed_pending")
            );
        }
        _ => {}
    }
    if control == "group-zero-override" {
        assert_eq!(first["group_state"]["group_count"], 0);
    }
    if control == "full-dps" {
        assert_eq!(first["group_state"]["include_in_full_dps"], true);
    }
    if control == "archived-only" {
        let reference_name = format!("{}-explicit-first-maps", family.name);
        let reference =
            &cases.iter().find(|c| c["name"] == reference_name).unwrap()["states"][stage];
        let previous = selected_family(reference, family)[0];
        for field in ["attributes", "group_attributes", "group_state", "effects"] {
            assert_eq!(first[field], previous[field]);
        }
        for mode in ["MAIN", "CALCS"] {
            let mut current = first[mode].clone();
            let mut original = previous[mode].clone();
            for actions in [&mut current, &mut original] {
                for action in actions.as_array_mut().unwrap() {
                    action.as_object_mut().unwrap().remove("source_ordinal");
                }
            }
            assert_eq!(
                current, original,
                "{name} {stage} {mode} archived edit changed selected source"
            );
        }
        assert_eq!(state["outputs"], reference["outputs"]);
        let changed: Vec<_> = rows(&state["saved"])
            .iter()
            .filter(|r| {
                r["selected"] == false
                    && r["attributes"]["gemId"] == family.game
                    && r["attributes"]["quality"] == "7"
            })
            .collect();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0]["loaded"]["level"], 19);
        assert_eq!(changed[0]["loaded"]["quality"], 7);
    }
}
