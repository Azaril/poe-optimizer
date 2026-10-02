//! Original-source authored-skill observations, not native or whole-build parity.
//! These controls keep evidence useful to the owned importer after retirement of
//! the old native preparation model. The shared source observer is unchanged.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/skill_preparation_source.rs"]
mod skill_preparation_source;
use mlua::{FromLua, Table};
use sha2::{Digest, Sha256};
use skill_preparation_source::{Oracle, repository};

fn corpus(index: usize) -> String {
    let directory = repository().join("tests/fixtures/builds/breadth-20260908");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("index.json")).unwrap()).unwrap();
    let xml = std::fs::read_to_string(directory.join(format!("build-{index:02}.xml"))).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(xml.as_bytes())),
        manifest["builds"][index - 1]["xml_sha256"]
            .as_str()
            .unwrap(),
        "the real original must remain unchanged"
    );
    xml
}

fn rows(observation: &Table, field: &str) -> Vec<Table> {
    observation
        .get::<Table>(field)
        .unwrap()
        .sequence_values()
        .map(Result::unwrap)
        .collect()
}

fn field<T: FromLua>(observation: &Table, key: &str) -> T {
    observation
        .get::<Table>("fields")
        .unwrap()
        .get(key)
        .unwrap()
}

fn successful(o: &Oracle, xml: &str, selected: Option<f64>) -> Table {
    let observation = o.observe(xml, selected);
    assert!(
        observation.get::<bool>("success").unwrap(),
        "source failed: {:?}",
        observation.get::<Option<String>>("error").unwrap()
    );
    assert!(
        observation
            .get::<Option<String>>("error")
            .unwrap()
            .is_none()
    );
    for group in rows(&observation, "groups") {
        assert!(group.get::<bool>("attached").unwrap());
        assert!(group.get::<u32>("processing_passes").unwrap() > 0);
    }
    observation
}

fn passes(observation: &Table) -> Vec<u32> {
    rows(observation, "groups")
        .iter()
        .map(|g| g.get("processing_passes").unwrap())
        .collect()
}

fn document(skills: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Build level="90" className="Ranger" ascendClassName="Deadeye"/><Skills>{skills}</Skills></PathOfBuilding2>"#
    )
}

#[test]
fn independent_original_source_observes_all_five_selected_authored_working_sets() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        if warm {
            for _ in 0..60 {
                successful(
                    &o,
                    &document(r#"<Skill><Gem skillId="TwisterPlayer" level="19"/></Skill>"#),
                    None,
                );
            }
        }
        for (index, key, entries) in [
            (1, 1., 62),
            (2, 6., 46),
            (3, 1., 62),
            (4, 1., 62),
            (5, 4., 28),
        ] {
            let observation = successful(&o, &corpus(index), None);
            assert_eq!(
                observation.get::<f64>("selected_key").unwrap(),
                key,
                "original {index}, JIT={warm}"
            );
            let selected: Vec<u32> = observation
                .get::<Table>("selected_groups")
                .unwrap()
                .sequence_values()
                .map(Result::unwrap)
                .collect();
            let groups = rows(&observation, "groups");
            let count: usize = groups
                .iter()
                .filter(|group| selected.contains(&group.get::<u32>("source").unwrap()))
                .map(|group| group.get::<Table>("gems").unwrap().raw_len())
                .sum();
            assert_eq!(count, entries, "original {index}, JIT={warm}");
            let gems: Vec<Table> = groups.iter().flat_map(|g| rows(g, "gems")).collect();
            if index == 2 {
                assert!(gems.iter().any(|gem| {
                    field::<Option<String>>(gem, "skillId").as_deref() == Some("TwisterPlayer")
                }));
            }
            if index == 5 {
                assert!(
                    gems.iter()
                        .any(|gem| field::<Option<String>>(gem, "skillMinion").is_some())
                );
            }
            eprintln!(
                "original={index}, JIT={warm}, groups={}, authored_entries={}, selected_entries={count}",
                groups.len(),
                gems.len(),
            );
        }
    }
}

#[test]
fn saved_set_overwrites_inactive_groups_and_explicit_selection_reprocesses_in_source_order() {
    let o = Oracle::new(false);
    let xml = document(
        r#"<SkillSet id="7"><Skill label="overwritten"><Gem skillId="TwisterPlayer" level="4"/></Skill></SkillSet><SkillSet id="2"><Skill label="inactive" enabled="false"><Gem skillId="TwisterPlayer" level="13" enabled="false"/></Skill></SkillSet><SkillSet id="07"><Skill label="winner"><Gem skillId="TwisterPlayer" level="19"/></Skill></SkillSet>"#,
    );
    let saved = successful(&o, &xml, None);
    assert_eq!(passes(&saved), [1, 1, 2]);
    assert_eq!(saved.get::<f64>("selected_key").unwrap(), 7.);
    let same = successful(&o, &xml, Some(7.));
    assert_eq!(passes(&same), [1, 1, 3]);
    assert_eq!(same.get::<f64>("selected_key").unwrap(), 7.);
    let changed = successful(&o, &xml, Some(2.));
    assert_eq!(passes(&changed), [1, 2, 2]);
    assert_eq!(changed.get::<f64>("selected_key").unwrap(), 2.);
    let groups = rows(&changed, "groups");
    assert_eq!(field::<String>(&groups[0], "label"), "overwritten");
    assert_eq!(field::<String>(&groups[1], "label"), "inactive");
    assert!(!field::<bool>(&groups[1], "enabled"));
    assert_eq!(field::<String>(&groups[2], "label"), "winner");
    let selected: Vec<u32> = changed
        .get::<Table>("selected_groups")
        .unwrap()
        .sequence_values()
        .map(Result::unwrap)
        .collect();
    assert_eq!(selected, [groups[1].get::<u32>("source").unwrap()]);
    successful(
        &o,
        &document(
            r#"<Skill label="legacy"><Gem level="1"/></Skill><SkillSet id="7"><Skill/></SkillSet><Skill label="legacy-again"><Gem level="1"/></Skill>"#,
        ),
        None,
    );
}

#[test]
fn names_levels_flags_unknown_gem_tags_and_minion_maps_reach_original_processing() {
    let o = Oracle::new(false);
    let xml = document(
        r#"<SkillSet id="1"><Skill active="true" includeInFullDPS="false" groupCount="-2.5" skillPart="7" mainActiveSkill="0" mainActiveSkillCalcs="2"><FutureGem nameSpec="tWiStEr" level="9999" quality="-3" count="0" enabled="false" enableGlobal1="false" enableGlobal2="true" corrupted="true" corruptLevel="-2" skillPart="4" note="&lt;b&gt;kept&lt;/b&gt;" statSetIndex="9" skillMinion="caller-minion" skillMinionItemSet="2" skillMinionSkill="3"><StatSetIndex grantedEffect="e" index="2"/><StatSetIndex grantedEffect="e" index="7"/><StatSetCalcsIndex grantedEffect="e" index="3"/><MinionSkillIndexLookup grantedEffect="e"><Map skillIndex="2" statSetIndex="4"/></MinionSkillIndexLookup><MinionSkillIndexLookup grantedEffect="e"><FutureMap skillIndex="3" statSetIndex="6"/><Map skillIndex="-0" statSetIndex="-1"/></MinionSkillIndexLookup><MinionSkillIndexLookupCalcs grantedEffect="e"><Map skillIndex="2" statSetIndex="8"/></MinionSkillIndexLookupCalcs></FutureGem><Gem nameSpec="&lt;b&gt;A—Bö&lt;/b&gt;" level="1"/><Gem nameSpec=""/><Gem skillId="TwisterPlayer" level="-4"/><Gem skillId="TwisterPlayer" level="2.5"/><Gem skillId="TwisterPlayer" level="inf"/><Gem skillId="TwisterPlayer" level="-inf"/><Gem skillId="TwisterPlayer" level="nan"/><Gem skillId="TwisterPlayer" level="0x4"/><Gem nameSpec="This Gem Has No Possible Match 987654" level="1"/></Skill></SkillSet>"#,
    );
    let result = successful(&o, &xml, None);
    let groups = rows(&result, "groups");
    let gems = rows(&groups[0], "gems");
    assert_eq!(gems.len(), 10);
    assert_eq!(field::<f64>(&gems[0], "skillPart"), 7.);
    assert_eq!(field::<String>(&gems[0], "nameSpec"), "Twister");
    assert_eq!(field::<f64>(&gems[0], "quality"), -3.);
    assert_eq!(field::<f64>(&gems[0], "count"), 0.);
    assert!(!field::<bool>(&gems[0], "enabled"));
    assert!(!field::<bool>(&gems[0], "enableGlobal1"));
    assert!(field::<bool>(&gems[0], "enableGlobal2"));
    let stat_set = gems[0].get::<Table>("stat_set").unwrap();
    assert_eq!(stat_set.get::<f64>("e").unwrap(), 7.);
    assert!(stat_set.get::<Option<f64>>("index").unwrap().is_none());
    assert_eq!(
        gems[0]
            .get::<Table>("stat_set_calcs")
            .unwrap()
            .get::<f64>("e")
            .unwrap(),
        3.
    );
    let minion = gems[0]
        .get::<Table>("minion_skill_lookup")
        .unwrap()
        .get::<Table>("e")
        .unwrap();
    assert!(minion.get::<Option<f64>>(2).unwrap().is_none());
    assert_eq!(minion.get::<f64>(3).unwrap(), 6.);
    assert_eq!(minion.get::<f64>(0).unwrap(), -1.);
    assert_eq!(
        gems[0]
            .get::<Table>("minion_skill_lookup_calcs")
            .unwrap()
            .get::<Table>("e")
            .unwrap()
            .get::<f64>(2)
            .unwrap(),
        8.
    );
    assert_eq!(field::<String>(&gems[1], "nameSpec"), "A-Bo");
    assert!(field::<String>(&gems[1], "errMsg").starts_with("Ambiguous gem name 'A-Bo':"));
    assert!(gems[1].get::<Option<String>>("gem_data").unwrap().is_none());
    assert_eq!(
        field::<String>(gems.last().unwrap(), "errMsg"),
        "Unrecognised gem name 'This Gem Has No Possible Match 987654'"
    );
    assert!(field::<Option<String>>(&gems[2], "errMsg").is_none());
    assert!(gems[2].get::<Option<String>>("gem_data").unwrap().is_none());
}

#[test]
fn ambiguous_names_and_source_runtime_errors_remain_observable() {
    let o = Oracle::new(false);
    let ambiguous = successful(
        &o,
        &document(r#"<Skill><Gem nameSpec="S" level="1"/></Skill>"#),
        None,
    );
    let gems = rows(&rows(&ambiguous, "groups")[0], "gems");
    assert!(field::<String>(&gems[0], "errMsg").starts_with("Ambiguous gem name 'S':"));
    assert!(gems[0].get::<Option<String>>("gem_data").unwrap().is_none());
    for body in [
        r#"<Skill><Gem skillId="TwisterPlayer"/></Skill>"#,
        r#"<Skill><Gem skillId="TwisterPlayer" level="bad"/></Skill>"#,
        r#"<Skill><Gem skillId="TwisterPlayer" level="19"/><Gem nameSpec="[" level="1"/></Skill>"#,
        r#"<Skill><Gem skillId="TwisterPlayer" level="19"/></Skill><Skill><Gem level="1"><MinionSkillIndexLookup grantedEffect="e"><Map skillIndex="bad" statSetIndex="2"/></MinionSkillIndexLookup></Gem></Skill>"#,
        r#"<Skill><Gem skillId="TwisterPlayer" level="19"/></Skill><Skill><Gem level="1"><MinionSkillIndexLookup grantedEffect="e"><Map skillIndex="nan" statSetIndex="2"/></MinionSkillIndexLookup></Gem></Skill>"#,
        r#"<SkillSet id="7"/><Skill><Gem skillId="TwisterPlayer" level="19"/></Skill>"#,
        r#"<Skill><Gem skillId="TwisterPlayer" level="19"/></Skill><Skill>text<Gem/></Skill>"#,
    ] {
        let observation = o.observe(&document(body), None);
        assert!(!observation.get::<bool>("success").unwrap(), "{body}");
        assert!(
            !observation.get::<String>("error").unwrap().is_empty(),
            "{body}"
        );
        assert!(
            observation
                .get::<Table>("selected_groups")
                .unwrap()
                .is_empty(),
            "failed loading must not claim a selected working set: {body}"
        );
    }
}

#[test]
fn original_level_requirement_mutation_changes_observed_level_and_attribute_requirements() {
    let o = Oracle::new(false);
    let xml = document(r#"<Skill><Gem skillId="TwisterPlayer" level="19" quality="20"/></Skill>"#);
    let baseline = successful(&o, &xml, None);
    let first = rows(&rows(&baseline, "groups")[0], "gems").remove(0);
    let changed = field::<f64>(&first, "reqLevel") + 7.;
    o.mutate(&format!(
        "data.skills.TwisterPlayer.levels[19].levelRequirement={changed}"
    ));
    let altered = successful(&o, &xml, None);
    let second = rows(&rows(&altered, "groups")[0], "gems").remove(0);
    assert_eq!(field::<f64>(&second, "reqLevel"), changed);
    assert_ne!(
        field::<f64>(&first, "reqDex"),
        field::<f64>(&second, "reqDex")
    );
    // The unchanged original build also reaches the mutated source tables.
    successful(&o, &corpus(2), None);
}

#[test]
fn external_identity_precedence_and_direct_minion_effect_use_distinct_source_lookups() {
    let o = Oracle::new(false);
    let xml = document(
        r#"<Skill><Gem gemId="Metadata/Items/Gem/SkillGemTwister" variantId="Twister" skillId="SummonBeastPlayer" level="19"/><Gem gemId="Metadata/Items/Gem/SkillGemTwister" variantId="missing" level="19"/><Gem gemId="missing" skillId="TwisterPlayer" level="19"/><Gem gemId="missing" skillId="TwisterPlayer" nameSpec="Twister" level="19"/><Gem skillId="ExplosiveTeleportSandDjinn" level="1"/></Skill>"#,
    );
    let result = successful(&o, &xml, None);
    let gems = rows(&rows(&result, "groups")[0], "gems");
    assert_eq!(field::<String>(&gems[0], "skillId"), "TwisterPlayer");
    assert_eq!(field::<String>(&gems[1], "skillId"), "TwisterPlayer");
    assert!(gems[2].get::<Option<String>>("gem_data").unwrap().is_none());
    assert!(
        gems[2]
            .get::<Option<String>>("granted_effect")
            .unwrap()
            .is_none()
    );
    assert!(gems[3].get::<Option<String>>("gem_data").unwrap().is_some());
    assert!(gems[4].get::<Option<String>>("gem_data").unwrap().is_none());
    assert_eq!(
        gems[4].get::<String>("granted_effect").unwrap(),
        "ExplosiveTeleportSandDjinn"
    );
    assert!(field::<Option<f64>>(&gems[4], "reqLevel").is_none());
}

#[test]
fn repeated_containers_preserve_control_fallback_and_replace_live_groups() {
    let o = Oracle::new(false);
    let xml = r#"<PathOfBuilding2><Build level="90"/><Skills defaultGemLevel="corruptedMaximum" defaultGemQuality="30" showSupportGemTypes="LINEAGE" sortGemsByDPSField="TotalDPS" sortGemsByDPS="false" showLegacyGems="true"><Skill><Gem skillId="TwisterPlayer" level="19"/></Skill></Skills><Skills defaultGemLevel="unavailable" defaultGemQuality="-3" showSupportGemTypes="unavailable" sortGemsByDPSField="unavailable"><Skill><Gem skillId="ExplosiveTeleportSandDjinn" level="1"/></Skill></Skills></PathOfBuilding2>"#;
    let actual = successful(&o, xml, None);
    let containers = rows(&actual, "containers");
    assert_eq!(containers.len(), 2);
    let groups = rows(&actual, "groups");
    assert_eq!(groups.len(), 2);
    let selected: Vec<u32> = actual
        .get::<Table>("selected_groups")
        .unwrap()
        .sequence_values()
        .map(Result::unwrap)
        .collect();
    assert_eq!(selected, [groups[1].get::<u32>("source").unwrap()]);
}
