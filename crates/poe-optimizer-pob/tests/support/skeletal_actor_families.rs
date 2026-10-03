//! Finite test inputs for the shared complete-source actor/action observer.
//! This module adds no source loader, interpreter or native calculation path.
use super::*;

const TEST: &str =
    "skeletal_families::complete_skeletal_families_preserve_actor_action_correspondence";
const CHILD: &str = "POE_SKELETAL_ACTOR_ACTION_SOURCE_CHILD";

struct Family {
    name: &'static str,
    gem: &'static str,
    effect: &'static str,
    actor: &'static str,
    command: &'static str,
    children: [&'static str; 2],
    labels: [&'static [&'static str]; 2],
    original_one: u64,
    original_five: &'static [(u64, u64)],
}
const FAMILIES: [Family; 3] = [
    Family {
        name: "arsonist",
        gem: "Metadata/Items/Gems/SkillGemSkeletalArsonist",
        effect: "SummonSkeletalArsonistsPlayer",
        actor: "RaisedSkeletonArsonist",
        command: "CommandSkeletalArsonistPlayer",
        children: [
            "FireBombSkeletonMinion",
            "DestructiveLinkSkeletonBombadierMinion",
        ],
        labels: [&["Fire Bomb", "Hidden"], &["Explosive Demise"]],
        original_one: 145,
        original_five: &[(188, 3), (206, 4), (249, 5), (337, 6), (403, 1)],
    },
    Family {
        name: "frost-mage",
        gem: "Metadata/Items/Gems/SkillGemSkeletalFrostMage",
        effect: "SummonSkeletalFrostMagesPlayer",
        actor: "RaisedSkeletonFrostMage",
        command: "CommandSkeletalFrostMagePlayer",
        children: ["FrostBoltSkeletonMageMinion", "IceArmourSkeletonMageMinion"],
        labels: [&["Projectile", "Explosion"], &["Ice Armour"]],
        original_one: 209,
        original_five: &[(195, 3), (213, 4), (256, 5)],
    },
    Family {
        name: "reaver",
        gem: "Metadata/Items/Gems/SkillGemSkeletalReaver",
        effect: "SummonSkeletalReaversPlayer",
        actor: "RaisedSkeletonReaver",
        command: "CommandSkeletalReaversPlayer",
        children: ["MinionMeleeStep", "EnrageSkeletonReaverMinion"],
        labels: [&["Basic Attack"], &["Enrage"]],
        original_one: 182,
        original_five: &[(224, 4), (267, 5), (328, 6), (393, 1)],
    },
];

#[test]
#[ignore = "requires the optional complete pinned PoB runtime; run explicitly for three-family actor/action breadth"]
fn complete_skeletal_families_preserve_actor_action_correspondence() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = root.join("runs/owned-skeletal-actor-action-source-01");
    fs::create_dir_all(&out).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        run_child(&root, &out, mode == "on");
        return;
    }
    run_modes(&root, &out, TEST, CHILD);
}

fn run_child(root: &Path, out: &Path, enabled: bool) {
    assert_eq!(
        pinned::manifest_sha256(),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let directory = root.join("tests/fixtures/builds/breadth-20260908");
    let index: Json =
        serde_json::from_slice(&fs::read(directory.join("index.json")).unwrap()).unwrap();
    let originals: Vec<_> = (1..=5)
        .map(|i| fs::read(directory.join(format!("build-{i:02}.xml"))).unwrap())
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
    let physical: Vec<_> = FAMILIES
        .iter()
        .map(|family| {
            let gem = catalog
                .data()
                .gems
                .iter()
                .find(|g| g.key == family.gem)
                .unwrap();
            assert_eq!(gem.primary_effect_id, family.effect);
            assert_eq!(gem.effect_list, [family.effect]);
            for references in [
                &gem.declared_additional_effects,
                &gem.constructed_additional_effects,
            ] {
                assert_eq!(references.len(), 1);
                assert_eq!(references[0].index, 1);
                assert_eq!(references[0].id, family.command);
            }
            assert!(gem.additional_effects.is_empty());
            gem
        })
        .collect();
    let mut cases = Vec::new();
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(
            digest(bytes),
            index["builds"][i]["xml_sha256"].as_str().unwrap()
        );
        cases.push(observe(
            root,
            &format!("original-{:02}", i + 1),
            std::str::from_utf8(bytes).unwrap(),
            enabled,
        ));
    }
    let xml = std::str::from_utf8(&originals[4]).unwrap();
    // Archive activation is itself observed. Runtime indices can shift because
    // the original evaluator removes unmatched item/tree-generated groups.
    let mut groups = std::collections::BTreeMap::new();
    for preset in [3, 4, 5, 6, 1] {
        let available = FAMILIES
            .iter()
            .find(|f| f.original_five.iter().any(|(_, p)| *p == preset))
            .unwrap();
        let activation = observe(
            root,
            &format!("preset-{preset}-activation"),
            &focus_physical(xml, preset, available.gem, None),
            enabled,
        );
        for family in &FAMILIES {
            if family.original_five.iter().any(|(_, p)| *p == preset) {
                let row = selected_family(&activation["states"]["fresh"], family)[0];
                groups.insert((family.name, preset), row["group"].as_u64().unwrap());
            }
        }
        cases.push(activation);
    }
    for family in &FAMILIES {
        for &(_, preset) in family.original_five {
            let changed = edit_physical(
                xml,
                preset,
                family.gem,
                &[
                    ("skillMinionSkill", Some("1")),
                    ("skillMinionSkillCalcs", Some("2")),
                ],
                &[],
                None,
                None,
            );
            let focused = focus_physical(
                &changed,
                preset,
                family.gem,
                Some(groups[&(family.name, preset)]),
            );
            cases.push(observe(
                root,
                &format!("{}-preset-{preset}-first-main-second-calcs", family.name),
                &focused,
                enabled,
            ));
        }
    }
    for family in &FAMILIES {
        let focused = focus_physical(xml, 4, family.gem, Some(groups[&(family.name, 4)]));
        let last = family.labels[0].len() as u64;
        let first = maps(family.effect, 1, 1, 1);
        let main_last = maps(family.effect, 1, last, 1);
        let calcs_last = maps(family.effect, 1, 1, last);
        let duplicate_maps = format!(
            r#"<MinionSkillIndexLookup grantedEffect="{}"><MinionSkillIndexMap skillIndex="1" statSetIndex="{last}"/><MinionSkillIndexMap skillIndex="1" statSetIndex="1"/></MinionSkillIndexLookup>"#,
            family.effect
        );
        let sibling_maps = maps(family.effect, 2, 1, 1);
        let sibling = [
            ("count", Some("3")),
            ("skillMinionSkill", Some("2")),
            ("skillMinionSkillCalcs", Some("2")),
        ];
        let archived = family
            .original_five
            .iter()
            .find(|(_, p)| *p != 4)
            .unwrap()
            .1;
        let controls = [
            (
                "missing-selectors",
                edit_physical(
                    &focused,
                    4,
                    family.gem,
                    &[
                        ("skillMinion", None),
                        ("skillMinionCalcs", None),
                        ("skillMinionSkill", None),
                        ("skillMinionSkillCalcs", None),
                    ],
                    &[],
                    None,
                    None,
                ),
            ),
            (
                "invalid-selectors",
                edit_physical(
                    &focused,
                    4,
                    family.gem,
                    &[
                        ("skillMinion", Some("unknown-main")),
                        ("skillMinionCalcs", Some("unknown-calcs")),
                        ("skillMinionSkill", Some("bad")),
                        ("skillMinionSkillCalcs", Some("bad")),
                    ],
                    &[],
                    None,
                    None,
                ),
            ),
            (
                "clamped-selectors",
                edit_physical(
                    &focused,
                    4,
                    family.gem,
                    &[
                        ("skillMinionSkill", Some("0")),
                        ("skillMinionSkillCalcs", Some("999")),
                    ],
                    &[],
                    None,
                    None,
                ),
            ),
            (
                "main-second",
                edit_physical(
                    &focused,
                    4,
                    family.gem,
                    &[
                        ("skillMinionSkill", Some("2")),
                        ("skillMinionSkillCalcs", Some("1")),
                    ],
                    &[],
                    None,
                    None,
                ),
            ),
            (
                "calcs-second",
                edit_physical(
                    &focused,
                    4,
                    family.gem,
                    &[
                        ("skillMinionSkill", Some("1")),
                        ("skillMinionSkillCalcs", Some("2")),
                    ],
                    &[],
                    None,
                    None,
                ),
            ),
            (
                "main-last-statset",
                edit_physical(&focused, 4, family.gem, &[], &[], Some(&main_last), None),
            ),
            (
                "calcs-last-statset",
                edit_physical(&focused, 4, family.gem, &[], &[], Some(&calcs_last), None),
            ),
            (
                "explicit-first-maps",
                edit_physical(
                    &focused,
                    4,
                    family.gem,
                    &[
                        ("skillMinion", Some(family.actor)),
                        ("skillMinionCalcs", Some(family.actor)),
                    ],
                    &[],
                    Some(&first),
                    None,
                ),
            ),
            (
                "duplicate-map-key",
                edit_physical(
                    &focused,
                    4,
                    family.gem,
                    &[],
                    &[],
                    Some(&duplicate_maps),
                    None,
                ),
            ),
            (
                "duplicate-physical",
                edit_physical(
                    xml,
                    4,
                    family.gem,
                    &[],
                    &[],
                    None,
                    Some((&sibling, &sibling_maps)),
                ),
            ),
            (
                "disabled-gem",
                edit_physical(
                    &focused,
                    4,
                    family.gem,
                    &[("enabled", Some("false"))],
                    &[],
                    None,
                    None,
                ),
            ),
            (
                "disabled-group",
                edit_physical(
                    xml,
                    4,
                    family.gem,
                    &[],
                    &[("enabled", Some("false"))],
                    None,
                    None,
                ),
            ),
            (
                "archived-only",
                edit_physical(
                    xml,
                    archived,
                    family.gem,
                    &[
                        ("skillMinionSkill", Some("2")),
                        ("skillMinionSkillCalcs", Some("2")),
                    ],
                    &[],
                    Some(&sibling_maps),
                    None,
                ),
            ),
        ];
        for (name, changed) in controls {
            assert_ne!(changed, xml);
            cases.push(observe(
                root,
                &format!("{}-{name}", family.name),
                &changed,
                enabled,
            ));
        }
    }
    cases.push(observe(root, "repeat-original-05", xml, enabled));
    let result = json!({
        "source_revision":"3887ae68a6a6b8bb7b41d1b61998f1aa184201e4","manifest_sha256":pinned::manifest_sha256(),
        "catalog_digest":CATALOG_DIGEST,"physical_identities":physical,"business_wrappers":false,
        "native_inventory_authority":false,"native_build_parity":false,"canonical_parity_lifecycle_selected":false,
        "lifecycle_stages":["fresh_complete_load","requested_original_frame_rebuild_1","requested_original_frame_rebuild_2"],
        "files":(["src/HeadlessWrapper.lua","src/Modules/Build.lua","src/Classes/CalcsTab.lua","src/Classes/SkillsTab.lua","src/Modules/Data.lua","src/Modules/CalcSetup.lua","src/Modules/CalcActiveSkill.lua","src/Modules/Calcs.lua","src/Modules/CalcPerform.lua","src/Data/Minions.lua","src/Data/Skills/minion.lua","src/Data/Skills/act_int.lua","src/Data/Gems.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
        "original_sources":originals.iter().enumerate().map(|(i,bytes)|json!({"path":format!("tests/fixtures/builds/breadth-20260908/build-{:02}.xml",i+1),"sha256":digest(bytes)})).collect::<Vec<_>>(),
        "cases":cases
    });
    let bytes = serde_json::to_vec_pretty(&result).unwrap();
    assert!(bytes.len() <= 64 * 1024 * 1024);
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if enabled { "on" } else { "off" }
        )),
        bytes,
    )
    .unwrap();
    for (i, bytes) in originals.iter().enumerate() {
        assert_eq!(
            &fs::read(directory.join(format!("build-{:02}.xml", i + 1))).unwrap(),
            bytes
        );
    }
    check(&result);
}

fn observe(root: &Path, name: &str, xml: &str, enabled: bool) -> Json {
    let families = FAMILIES
        .iter()
        .map(|f| (f.gem, f.effect))
        .collect::<Vec<_>>();
    observe_families(root, name, xml, enabled, &families)
}
fn maps(effect: &str, child: u64, main: u64, calcs: u64) -> String {
    format!(
        r#"<MinionSkillIndexLookup grantedEffect="{effect}"><MinionSkillIndexMap skillIndex="{child}" statSetIndex="{main}"/></MinionSkillIndexLookup><MinionSkillIndexLookupCalcs grantedEffect="{effect}"><MinionSkillIndexMap skillIndex="{child}" statSetIndex="{calcs}"/></MinionSkillIndexLookupCalcs>"#
    )
}
fn selected_family<'a>(state: &'a Json, family: &Family) -> Vec<&'a Json> {
    rows(&state["saved"])
        .iter()
        .filter(|r| r["selected"] == true && r["attributes"]["gemId"] == family.gem)
        .collect()
}
fn check(result: &Json) {
    let cases = rows(&result["cases"]);
    assert_eq!(cases.len(), 62);
    let baseline = &cases[4];
    assert_eq!(baseline["states"], cases.last().unwrap()["states"]);
    assert_eq!(
        baseline["source_joins"],
        cases.last().unwrap()["source_joins"]
    );
    for family in &FAMILIES {
        for (case, expected) in [
            (0, vec![family.original_one]),
            (4, family.original_five.iter().map(|(o, _)| *o).collect()),
        ] {
            let found = rows(&cases[case]["states"]["fresh"]["saved"])
                .iter()
                .filter(|r| r["attributes"]["gemId"] == family.gem)
                .map(|r| r["source_ordinal"].as_u64().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(found, expected, "{} originals", family.name);
        }
    }
    assert_eq!(rows(&cases[0]["states"]["fresh"]["saved"]).len(), 3);
    assert_eq!(rows(&baseline["states"]["fresh"]["saved"]).len(), 12);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let revision = case["states"]["fresh"]["output_revision"].as_u64().unwrap();
        for (i, stage) in ["fresh", "rebuilt_once", "rebuilt_twice"]
            .into_iter()
            .enumerate()
        {
            let state = &case["states"][stage];
            for flag in [
                "source_methods_preserved",
                "loader_observer_removed",
                "requested_jit_mode_verified",
                "exact_physical_objects",
                "physical_objects_preserved_across_stages",
                "saved_inputs_preserved",
                "selected_state_preserved",
                "reported_outputs_preserved",
            ] {
                assert_eq!(state[flag], true, "{name} {stage} {flag}");
            }
            assert_eq!(state["output_revision"], revision + i as u64);
            assert_eq!(state["output_lifecycle"]["module_reloads"], 2);
            assert_eq!(state["build_flag"], false);
            for row in rows(&state["saved"]) {
                let family = FAMILIES
                    .iter()
                    .find(|f| row["attributes"]["gemId"] == f.gem)
                    .unwrap();
                assert_eq!(
                    row["resolved_additional_count"], 0,
                    "unresolved Command must remain separate"
                );
                for mode in ["MAIN", "CALCS"] {
                    let actions = rows(&row[mode]);
                    assert!(actions.len() <= 1);
                    if row["selected"] == false {
                        assert!(
                            actions.is_empty(),
                            "{name} {stage}: archived source has no runtime action"
                        );
                    }
                    for action in actions {
                        check_action(action, row, family, mode);
                    }
                }
            }
            if name.starts_with("original-")
                || name == "repeat-original-05"
                || name.ends_with("-activation")
            {
                continue;
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
                if control == "duplicate-physical" {
                    2
                } else {
                    1
                }
            );
            let first = selected[0];
            for mode in ["MAIN", "CALCS"] {
                let actions = rows(&first[mode]);
                let disabled = control == "disabled-gem"
                    || (control == "disabled-group"
                        && rows(&selected_family(&baseline["states"][stage], family)[0][mode])[0]
                            ["is_main_skill"]
                            == false);
                assert_eq!(
                    actions.len(),
                    usize::from(!disabled),
                    "{name} {stage} {mode}"
                );
                if let Some(action) = actions.first() {
                    let expected = if (mode == "MAIN" && control == "main-second")
                        || (mode == "CALCS"
                            && (control == "calcs-second"
                                || control == "clamped-selectors"
                                || control.starts_with("preset-")))
                    {
                        2
                    } else {
                        1
                    };
                    assert_eq!(
                        action["actor"]["selected_child_index"], expected,
                        "{name} {stage} {mode}"
                    );
                    let set = if (mode == "MAIN" && control == "main-last-statset")
                        || (mode == "CALCS" && control == "calcs-last-statset")
                    {
                        family.labels[0].len()
                    } else {
                        1
                    };
                    assert_eq!(
                        rows(&action["actor"]["children"])[0]["stat_set"]["index"],
                        set,
                        "{name} {stage} {mode}"
                    );
                    if !matches!(
                        control,
                        "duplicate-physical" | "disabled-group" | "archived-only"
                    ) {
                        assert_eq!(action["is_main_skill"], true, "{name} {stage} {mode}");
                    }
                }
                if control == "duplicate-physical" {
                    let second = &rows(&selected[1][mode])[0];
                    assert_ne!(second["source_ordinal"], first["source_ordinal"]);
                    assert_eq!(second["actor"]["selected_child_index"], 2);
                    assert_eq!(selected[1]["loaded"]["count"], 3);
                }
            }
            if control == "archived-only" {
                let previous = selected_family(&baseline["states"][stage], family)[0];
                for field in [
                    "attributes",
                    "loaded",
                    "group_state",
                    "group_attributes",
                    "effects",
                ] {
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
                    assert_eq!(current, original);
                }
                assert_eq!(state["outputs"], baseline["states"][stage]["outputs"]);
            }
        }
    }
}
fn check_action(action: &Json, row: &Json, family: &Family, mode: &str) {
    assert_eq!(action["source_ordinal"], row["source_ordinal"]);
    assert_eq!(action["exact_physical_object"], true);
    assert_eq!(action["exact_group"], true);
    assert_eq!(action["effect"], family.effect);
    assert_eq!(action["minion_choices"], json!([family.actor]));
    let actor = &action["actor"];
    assert_eq!(actor["type"], family.actor);
    assert_eq!(actor["unique_actor_identity"], true);
    assert_eq!(actor["source_data_identity"], true);
    assert_eq!(actor["item_set_present"], false);
    assert_eq!(actor["is_selected_actor"], action["is_main_skill"]);
    assert_eq!(
        action["actor_selector_field"],
        if mode == "CALCS" && action["is_main_skill"] == true {
            "skillMinionCalcs"
        } else {
            "skillMinion"
        }
    );
    assert_eq!(action["actor_selector_value"], family.actor);
    assert_eq!(
        action["child_selector_field"],
        if mode == "CALCS" {
            "skillMinionSkillCalcs"
        } else {
            "skillMinionSkill"
        }
    );
    assert_eq!(
        action["child_selector_value"],
        actor["selected_child_index"]
    );
    let children = rows(&actor["children"]);
    assert_eq!(children.len(), 2);
    for (i, child) in children.iter().enumerate() {
        assert_eq!(child["effect"], family.children[i]);
        assert_eq!(child["exact_actor"], true);
        assert_eq!(child["exact_summon"], true);
        assert_eq!(child["source_instance_present"], false);
        assert_eq!(child["stat_set"]["declared_table_identity"], true);
        let labels = rows(&child["stat_sets"])
            .iter()
            .map(|s| s["label"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(labels, family.labels[i]);
    }
}
