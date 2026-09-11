//! Paired saved-set selection is separate from complete loader/evaluator parity.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/shared_view_source.rs"]
mod shared_view_source;
use mlua::Table;
use poe_optimizer_core::{build_identity::BuildLineage, build_view::ViewRequest};
use poe_optimizer_data::game_data::{GameDataSnapshot, bundled_snapshot};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    selected_view::{ResolveLimits, SetOrigin, resolve_view},
};
use shared_view_source::Oracle;
use std::{path::PathBuf, sync::OnceLock};
fn snapshot() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}
fn paired(xml: &str, obs: &Table) {
    let build = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([31; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let view = resolve_view(
        &build,
        snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    let report = view.report();
    for (name, actual) in [
        ("Skills", &report.skills),
        ("Items", &report.items),
        ("Tree", &report.passives),
        ("Config", &report.configuration),
    ] {
        let expected = domain(obs, name);
        let registered: Vec<f64> = expected
            .get::<Table>("registered_keys")
            .unwrap()
            .sequence_values()
            .map(Result::unwrap)
            .collect();
        let mut native_registered: Vec<f64> =
            actual.sets.iter().map(|set| set.key.value()).collect();
        native_registered.sort_by(f64::total_cmp);
        native_registered.dedup_by(|a, b| *a == *b);
        assert_eq!(
            native_registered, registered,
            "registered set key prefix {name}"
        );
        if !expected.get::<bool>("ok").unwrap() {
            assert!(
                actual.problem.is_some(),
                "source failed but native selected {name}: {actual:?}"
            );
            continue;
        }
        assert!(actual.problem.is_none(), "{name}: {:?}", actual.problem);
        let selected = actual.selected.as_ref().unwrap();
        assert_eq!(
            selected.key.value(),
            expected.get::<f64>("key").unwrap(),
            "{name}"
        );
        // The order is an independent source result, not sorted/unique catalog order.
        let source_order = keys(&expected);
        let native_order: Vec<_> = actual
            .order
            .iter()
            .take_while(|v| v.is_some())
            .map(|v| v.unwrap().value())
            .collect();
        assert_eq!(native_order, source_order, "{name}");
        if let SetOrigin::Authored { source, .. } = selected.origin
            && let Some(title) = build.attribute(source, "title").unwrap()
        {
            assert_eq!(
                Some(title.decoded()),
                expected
                    .get::<Option<String>>("selected_title")
                    .unwrap()
                    .as_deref(),
                "selected source occurrence {name}"
            );
        }
        if name == "Skills" {
            assert_eq!(
                selected.members.len(),
                expected.get::<usize>("groups").unwrap(),
                "selected group ownership"
            );
        }
    }
    let expected = domain(obs, "Items")
        .get::<Option<bool>>("selected_use_second_weapon_set")
        .unwrap();
    if let Some(expected) = expected {
        assert_eq!(report.weapon_state.use_second_weapon_set, Some(expected));
    }
}

fn domain(observation: &Table, name: &str) -> Table {
    observation.get(name).unwrap()
}
fn assert_ok(t: &Table) {
    assert!(
        t.get::<bool>("ok").unwrap(),
        "{:?}",
        t.get::<Option<String>>("error").unwrap()
    );
}
fn keys(t: &Table) -> Vec<f64> {
    t.get::<Table>("order")
        .unwrap()
        .sequence_values()
        .map(Result::unwrap)
        .collect()
}
#[test]
fn all_five_originals_run_four_independent_source_selectors() {
    let oracle = Oracle::new();
    for index in 1..=5 {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../tests/fixtures/builds/breadth-20260908/build-{index:02}.xml"
        ));
        let xml = std::fs::read_to_string(path).unwrap();
        let obs = oracle.observe(&xml);
        paired(&xml, &obs);
        for name in ["Skills", "Items", "Config", "Tree"] {
            let d = domain(&obs, name);
            assert_ok(&d);
            let key = d.get::<f64>("key").unwrap();
            assert!(keys(&d).contains(&key), "build {index} {name}");
            eprintln!(
                "original build={index} domain={name} key={key} title={:?} order={:?}",
                d.get::<Option<String>>("selected_title").unwrap(),
                keys(&d)
            );
        }
    }
}
#[test]
fn independent_keys_and_tree_positions_preserve_duplicate_winners() {
    let xml = r#"<PathOfBuilding2><Skills activeSkillSet="99"><SkillSet id="7" title="first"/><SkillSet id="7.0" title="last"/><SkillSet id="-2.5" title="fraction"/></Skills><Items activeItemSet="-2.5"><ItemSet id="7" title="old"/><ItemSet id="7.0" title="new"/><ItemSet id="-2.5" title="items fraction" useSecondWeaponSet="true"/></Items><Config activeConfigSet="99"><ConfigSet id="7" title="before"/><ConfigSet id="7.0" title="after"/></Config><Tree activeSpec="99"><Spec title="first tree" treeVersion="0_5"/><Spec title="second tree" treeVersion="0_5"/></Tree></PathOfBuilding2>"#;
    let oracle = Oracle::new();
    let obs = oracle.observe(xml);
    paired(xml, &obs);
    for (name, key, title) in [
        ("Skills", 7., "last"),
        ("Items", -2.5, "items fraction"),
        ("Config", 7., "after"),
        ("Tree", 2., "second tree"),
    ] {
        let d = domain(&obs, name);
        assert_ok(&d);
        assert_eq!(d.get::<f64>("key").unwrap(), key);
        assert_eq!(d.get::<String>("selected_title").unwrap(), title);
    }
    assert_eq!(keys(&domain(&obs, "Skills")), vec![7., 7., -2.5]);
    assert_eq!(keys(&domain(&obs, "Config")), vec![7., 7.]);
    assert!(
        domain(&obs, "Items")
            .get::<bool>("selected_use_second_weapon_set")
            .unwrap()
    );
}
#[test]
fn malformed_numeric_requests_fallback_while_invalid_passive_positions_fail() {
    let oracle = Oracle::new();
    for raw in ["garbage", "", "nan", "inf", "0", "-2.5", "0x7"] {
        let xml = format!(
            r#"<PathOfBuilding2><Skills activeSkillSet="{raw}"><SkillSet id="7" title="skills"/></Skills><Items activeItemSet="{raw}"><ItemSet id="7" title="items"/></Items><Config activeConfigSet="{raw}"><ConfigSet id="7" title="config"/></Config><Tree activeSpec="{raw}"><Spec title="tree" treeVersion="0_5"/></Tree></PathOfBuilding2>"#
        );
        let obs = oracle.observe(&xml);
        paired(&xml, &obs);
        for name in ["Skills", "Items", "Config"] {
            let d = domain(&obs, name);
            assert_ok(&d);
            assert_eq!(d.get::<f64>("key").unwrap(), 7.);
        }
        let tree = domain(&obs, "Tree");
        assert_eq!(
            tree.get::<bool>("ok").unwrap(),
            raw != "0" && raw != "-2.5",
            "{raw}"
        );
    }
}

#[test]
fn dense_generated_keys_legacy_defaults_and_fractional_external_ids_pair() {
    let oracle = Oracle::new();
    for (skills, items, config, tree) in [
        (
            r#"<Skills><Skill label="legacy"/></Skills>"#,
            r#"<Items useSecondWeaponSet="true"/>"#,
            r#"<Config/>"#,
            r#"<Tree/>"#,
        ),
        (
            r#"<Skills><SkillSet id="1" title="one"/><SkillSet title="two"/></Skills>"#,
            r#"<Items><ItemSet id="2" title="two"/><ItemSet title="one"/></Items>"#,
            r#"<Config activeConfigSet="0"><ConfigSet id="0" title="zero"/></Config>"#,
            r#"<Tree activeSpec="100"><Spec title="a"/><Spec title="b"/></Tree>"#,
        ),
        (
            r#"<Skills activeSkillSet="-2.5"><SkillSet id="-2.5" title="fraction"/><SkillSet id="0" title="zero"/></Skills>"#,
            r#"<Items activeItemSet="0"><ItemSet id="-2.5" title="fraction"/><ItemSet id="-0" title="zero"/></Items>"#,
            r#"<Config activeConfigSet="-2.5"><ConfigSet id="-2.5" title="fraction"/><ConfigSet id="0" title="zero"/></Config>"#,
            r#"<Tree><Spec title="only"/></Tree>"#,
        ),
    ] {
        let xml = format!("<PathOfBuilding2>{skills}{items}{config}{tree}</PathOfBuilding2>");
        let obs = oracle.observe(&xml);
        paired(&xml, &obs);
    }
}

#[test]
fn malformed_authored_keys_retain_source_failure_instead_of_fake_selection() {
    let oracle = Oracle::new();
    for raw in ["nan", "", "invalid"] {
        let xml = format!(
            r#"<PathOfBuilding2><Skills><SkillSet id="{raw}" title="bad or auto"/></Skills><Items><ItemSet id="{raw}" title="bad or auto"/></Items><Config><ConfigSet id="{raw}" title="bad"/></Config><Tree><Spec title="tree"/></Tree></PathOfBuilding2>"#
        );
        let obs = oracle.observe(&xml);
        paired(&xml, &obs);
        assert!(!domain(&obs, "Config").get::<bool>("ok").unwrap());
    }
}

#[test]
fn sparse_auto_ids_and_infinite_keys_follow_independent_source_tables() {
    let oracle = Oracle::new();
    for ids in [
        &[1., 3.][..],
        &[2., 8.][..],
        &[-2.5, 0.][..],
        &[1., 2., 4.][..],
        &[1., 2., 1., 4.][..],
    ] {
        let skills: String = ids
            .iter()
            .enumerate()
            .map(|(index, id)| format!(r#"<SkillSet id="{id}" title="s{index}"/>"#))
            .collect();
        let items: String = ids
            .iter()
            .enumerate()
            .map(|(index, id)| format!(r#"<ItemSet id="{id}" title="i{index}"/>"#))
            .collect();
        let xml = format!(
            r#"<PathOfBuilding2><Skills activeSkillSet="99">{skills}<SkillSet title="auto"/></Skills><Items activeItemSet="99">{items}<ItemSet title="auto"/></Items><Config/><Tree/></PathOfBuilding2>"#
        );
        let obs = oracle.observe(&xml);
        paired(&xml, &obs);
    }
    for raw in ["inf", "-inf"] {
        let xml = format!(
            r#"<PathOfBuilding2><Skills activeSkillSet="{raw}"><SkillSet id="1"/><SkillSet id="{raw}" title="infinite"/></Skills><Items activeItemSet="{raw}"><ItemSet id="1"/><ItemSet id="{raw}" title="infinite"/></Items><Config activeConfigSet="{raw}"><ConfigSet id="1"/><ConfigSet id="{raw}" title="infinite"/></Config><Tree/></PathOfBuilding2>"#
        );
        let obs = oracle.observe(&xml);
        paired(&xml, &obs);
    }
}

#[test]
fn legacy_mixing_and_sparse_configuration_order_match_original_prefixes() {
    let oracle = Oracle::new();
    for (skills, config) in [
        (
            r#"<Skills><SkillSet id="7"/><Skill/></Skills>"#,
            r#"<Config/>"#,
        ),
        (
            r#"<Skills><Skill label="a"/><SkillSet id="7"/><Skill label="b"/></Skills>"#,
            r#"<Config/>"#,
        ),
        (
            r#"<Skills/>"#,
            r#"<Config><ConfigSet id="7" title="seven"/><Input name="x" number="2"/><ConfigSet id="2" title="two"/></Config>"#,
        ),
        (
            r#"<Skills/>"#,
            r#"<Config><ConfigSet id="7" title="seven"/>text<ConfigSet id="2" title="two"/></Config>"#,
        ),
        (
            r#"<Skills/>"#,
            r#"<Config><ConfigSet id="2" title="two"/><ConfigSet id="8" title="eight"/><ConfigSet title="generated then nil"/></Config>"#,
        ),
        (
            r#"<Skills/>"#,
            r#"<Config> <!-- only whitespace/comment --> </Config>"#,
        ),
        (r#"<Skills/>"#, r#"<Config></Config>"#),
    ] {
        let xml = format!("<PathOfBuilding2>{skills}<Items/>{config}<Tree/></PathOfBuilding2>");
        let obs = oracle.observe(&xml);
        paired(&xml, &obs);
    }
}

#[test]
fn reached_skill_child_and_minion_map_errors_are_not_silently_selected() {
    let oracle = Oracle::new();
    for child in [
        "text",
        r#"<Gem><MinionSkillIndexLookup grantedEffect="e">text</MinionSkillIndexLookup></Gem>"#,
        r#"<Gem><MinionSkillIndexLookup grantedEffect="e"><Map/></MinionSkillIndexLookup></Gem>"#,
        r#"<Gem><MinionSkillIndexLookup grantedEffect="e"><Map skillIndex="nan"/></MinionSkillIndexLookup></Gem>"#,
        r#"<Gem><MinionSkillIndexLookup><Map skillIndex="nan"/></MinionSkillIndexLookup></Gem>"#,
    ] {
        let xml = format!(
            "<PathOfBuilding2><Skills><SkillSet id=\"1\"><Skill>{child}</Skill></SkillSet></Skills><Items/><Config/><Tree/></PathOfBuilding2>"
        );
        let obs = oracle.observe(&xml);
        paired(&xml, &obs);
    }
}
