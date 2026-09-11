//! Paired authored-skill preparation, not effective actors or whole-build parity.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/skill_preparation_source.rs"]
mod skill_preparation_source;
use mlua::{Table, Value};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    build_view::{SelectionRequest, ViewRequest},
};
use poe_optimizer_data::game_data::{GameDataLoader, LoadLimits, TrustPolicy};
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    selected_view::{ResolveLimits, resolve_view},
};
use poe_optimizer_native::{
    CompiledGameData,
    skills::{
        PreparedSkills, SkillIdentityStatus, SkillNumber, SkillPreparationLimits,
        SkillPreparationStatus, SkillValue, prepare_authored_skills,
    },
};
use sha2::{Digest, Sha256};
use skill_preparation_source::{Oracle, repository};
use std::{collections::BTreeMap, sync::Arc};

fn imported(xml: &str) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([91; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
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
fn numbers(actual: &BTreeMap<String, SkillNumber>, expected: &Table, label: &str) {
    let expected: BTreeMap<String, f64> = expected.clone().pairs().map(Result::unwrap).collect();
    assert_eq!(actual.len(), expected.len(), "map keys {label}");
    for (key, expected) in expected {
        same_number(actual[&key].value(), expected, &format!("{label}/{key}"));
    }
}
fn same_number(actual: f64, expected: f64, label: &str) {
    assert!(
        actual.to_bits() == expected.to_bits() || (actual.is_nan() && expected.is_nan()),
        "{label}: {actual:?} != {expected:?}"
    );
}
fn nested_numbers(
    actual: &BTreeMap<String, BTreeMap<String, SkillNumber>>,
    expected: &Table,
    label: &str,
) {
    let expected: BTreeMap<String, Table> = expected.clone().pairs().map(Result::unwrap).collect();
    assert_eq!(actual.len(), expected.len(), "nested map keys {label}");
    for (effect, table) in expected {
        let mut canonical = BTreeMap::new();
        for result in table.pairs::<f64, f64>() {
            let (key, value) = result.unwrap();
            canonical.insert(
                format!("{:016x}", if key == 0.0 { 0 } else { key.to_bits() }),
                value,
            );
        }
        assert_eq!(actual[&effect].len(), canonical.len(), "{label}/{effect}");
        for (key, value) in canonical {
            same_number(actual[&effect][&key].value(), value, label);
        }
    }
}
fn fields(actual: &BTreeMap<String, SkillValue>, expected: &Table, label: &str) {
    let expected: BTreeMap<String, Value> = expected.clone().pairs().map(Result::unwrap).collect();
    assert_eq!(
        actual.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>(),
        "scalar keys {label}"
    );
    for (key, expected) in expected {
        match (&actual[&key], expected) {
            (SkillValue::Boolean(a), Value::Boolean(b)) => assert_eq!(*a, b, "{label}/{key}"),
            (SkillValue::Number(a), Value::Number(b)) => {
                same_number(a.value(), b, &format!("{label}/{key}"))
            }
            (SkillValue::Number(a), Value::Integer(b)) => {
                same_number(a.value(), b as f64, &format!("{label}/{key}"))
            }
            (SkillValue::Text(a), Value::String(b)) => {
                let b = b.to_str().unwrap();
                // Independent Lua hash-table traversal may name a different pair
                // of matches. Its stable contract is ambiguity, not a sorted winner.
                if key == "errMsg" && b.starts_with("Ambiguous gem name '") {
                    assert_eq!(
                        a.split(": matches ").next(),
                        b.split(": matches ").next(),
                        "{label}/{key}"
                    );
                } else {
                    assert_eq!(a, b.as_ref(), "{label}/{key}");
                }
            }
            (a, b) => panic!("type mismatch {label}/{key}: {a:?} vs {b:?}"),
        }
    }
}
fn pair(
    o: &Oracle,
    xml: &str,
    data: &Arc<CompiledGameData>,
    override_key: Option<f64>,
) -> PreparedSkills {
    let build = imported(xml);
    let mut request = ViewRequest::default();
    if let Some(key) = override_key {
        let saved =
            resolve_view(&build, data.snapshot(), &request, ResolveLimits::default()).unwrap();
        let set = saved
            .report()
            .skills
            .sets
            .iter()
            .rev()
            .find(|set| set.key.value() == key)
            .unwrap();
        let Some(AuthoredInstanceId::SkillSet(id)) = set.origin.instance() else {
            panic!("authored override required")
        };
        request.skills = SelectionRequest::Instance(id);
    }
    let view = resolve_view(&build, data.snapshot(), &request, ResolveLimits::default()).unwrap();
    let actual =
        prepare_authored_skills(&build, &view, data, SkillPreparationLimits::default()).unwrap();
    actual.validate_binding(&build, &view, data).unwrap();
    let report = actual.report();
    let expected = o.observe(xml, override_key);
    let ok = expected.get::<bool>("success").unwrap();
    if ok {
        assert_eq!(
            report.status,
            SkillPreparationStatus::Complete,
            "{:?}",
            report.failure
        );
        assert!(report.failure.is_none());
        same_number(
            view.report().skills.selected.as_ref().unwrap().key.value(),
            expected.get("selected_key").unwrap(),
            "independent saved/override selector",
        );
    } else {
        assert_eq!(
            report.status,
            SkillPreparationStatus::SourceFailure,
            "source error {:?}, native {:?}",
            expected.get::<Option<String>>("error").unwrap(),
            report.failure
        );
        assert!(report.failure.is_some());
    }
    let containers: Vec<Table> = expected
        .get::<Table>("containers")
        .unwrap()
        .sequence_values()
        .map(Result::unwrap)
        .collect();
    assert_eq!(report.containers.len(), containers.len());
    for (container, expected) in report.containers.iter().zip(containers) {
        assert_eq!(
            container.source.ordinal(),
            expected.get::<u32>("source").unwrap()
        );
        fields(
            &container.fields,
            &expected.get("fields").unwrap(),
            "container defaults",
        );
        let order: Vec<f64> = expected
            .get::<Table>("order")
            .unwrap()
            .sequence_values()
            .map(Result::unwrap)
            .collect();
        assert_eq!(container.order.len(), order.len());
        for (a, b) in container.order.iter().zip(order) {
            same_number(a.value(), b, "container source set order");
        }
        assert_eq!(
            container.active_set_id.map(|v| v.value()),
            expected.get::<Option<f64>>("active_set_id").unwrap()
        );
    }
    let expected_groups: Vec<Table> = expected
        .get::<Table>("groups")
        .unwrap()
        .sequence_values()
        .map(Result::unwrap)
        .collect();
    let actual_groups: Vec<_> = report
        .groups
        .iter()
        .filter(|g| g.processing_passes > 0)
        .collect();
    assert_eq!(
        actual_groups.len(),
        expected_groups.len(),
        "reached processing groups"
    );
    if ok {
        assert_eq!(report.groups.len(), expected_groups.len());
    }
    for (group, expected) in actual_groups.into_iter().zip(expected_groups) {
        assert_eq!(
            group.source.ordinal(),
            expected.get::<u32>("source").unwrap(),
            "source group order"
        );
        assert_eq!(group.attached, expected.get::<bool>("attached").unwrap());
        assert_eq!(
            group.processing_passes,
            expected.get::<u32>("processing_passes").unwrap()
        );
        fields(
            &group.fields,
            &expected.get("fields").unwrap(),
            &format!("group {}", group.source.ordinal()),
        );
        let gems: Vec<Table> = expected
            .get::<Table>("gems")
            .unwrap()
            .sequence_values()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            group.gems.len(),
            gems.len(),
            "authored entries, including disabled and unresolved"
        );
        for (gem, expected) in group.gems.iter().zip(gems) {
            assert_eq!(gem.source.ordinal(), expected.get::<u32>("source").unwrap());
            let label = format!(
                "group {} gem {}",
                group.source.ordinal(),
                gem.source.ordinal()
            );
            fields(&gem.fields, &expected.get("fields").unwrap(), &label);
            assert_eq!(
                gem.gem_data,
                expected.get::<Option<String>>("gem_data").unwrap(),
                "{label}"
            );
            assert_eq!(
                gem.granted_effect,
                expected.get::<Option<String>>("granted_effect").unwrap(),
                "{label}"
            );
            numbers(&gem.stat_set, &expected.get("stat_set").unwrap(), &label);
            numbers(
                &gem.stat_set_calcs,
                &expected.get("stat_set_calcs").unwrap(),
                &label,
            );
            nested_numbers(
                &gem.minion_skill_lookup,
                &expected.get("minion_skill_lookup").unwrap(),
                &label,
            );
            nested_numbers(
                &gem.minion_skill_lookup_calcs,
                &expected.get("minion_skill_lookup_calcs").unwrap(),
                &label,
            );
            if ok {
                assert!(gem.processed, "{label}");
            }
            if gem
                .text("errMsg")
                .is_some_and(|s| s.starts_with("Ambiguous gem name '"))
            {
                assert_eq!(gem.identity_status, SkillIdentityStatus::AmbiguousName);
                assert!(gem.gem_data.is_none());
            }
        }
    }
    if ok {
        let expected: Vec<u32> = expected
            .get::<Table>("selected_groups")
            .unwrap()
            .sequence_values()
            .map(Result::unwrap)
            .collect();
        let actual: Vec<_> = report
            .selected_groups
            .iter()
            .map(|id| {
                build
                    .binding(AuthoredInstanceId::SkillGroup(*id))
                    .unwrap()
                    .source()
                    .ordinal()
            })
            .collect();
        assert_eq!(actual, expected, "independently selected group occurrences");
    }
    assert_eq!(report.source_sha256, build.source_sha256());
    assert_eq!(report.data, *data.identity());
    assert!(
        !report.frontiers.is_empty(),
        "authored loading is not effective actor/full-build evaluation"
    );
    actual
}
fn document(skills: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Build level="90" className="Ranger" ascendClassName="Deadeye"/><Skills>{skills}</Skills></PathOfBuilding2>"#
    )
}

#[test]
fn all_five_exact_originals_pair_every_authored_group_entry_and_selected_view() {
    let data = CompiledGameData::bundled().unwrap();
    for warm in [false, true] {
        let o = Oracle::new(warm);
        if warm {
            for _ in 0..60 {
                o.observe(
                    &document(r#"<Skill><Gem skillId="TwisterPlayer" level="19"/></Skill>"#),
                    None,
                );
            }
        }
        for (index, selected_entries) in [(1, 62), (2, 46), (3, 62), (4, 62), (5, 28)] {
            let xml = corpus(index);
            let prepared = pair(&o, &xml, &data, None);
            let report = prepared.report();
            let count: usize = report
                .groups
                .iter()
                .filter(|g| report.selected_groups.contains(&g.instance))
                .map(|g| g.gems.len())
                .sum();
            assert_eq!(count, selected_entries, "original {index}");
            assert!(
                report
                    .groups
                    .iter()
                    .all(|g| g.gems.iter().all(|gem| gem.processed))
            );
            if index == 2 {
                assert!(
                    report
                        .groups
                        .iter()
                        .flat_map(|g| &g.gems)
                        .any(|gem| gem.text("skillId") == Some("TwisterPlayer"))
                );
            }
            if index == 5 {
                assert!(
                    report
                        .groups
                        .iter()
                        .flat_map(|g| &g.gems)
                        .any(|gem| gem.text("skillMinion").is_some())
                );
            }
        }
    }
}

#[test]
fn saved_set_overwrites_inactive_groups_and_explicit_view_reprocessing_keep_source_order() {
    let data = CompiledGameData::bundled().unwrap();
    let o = Oracle::new(false);
    let xml = document(
        r#"<SkillSet id="7"><Skill label="overwritten"><Gem skillId="TwisterPlayer" level="4"/></Skill></SkillSet><SkillSet id="2"><Skill label="inactive" enabled="false"><Gem skillId="TwisterPlayer" level="13" enabled="false"/></Skill></SkillSet><SkillSet id="07"><Skill label="winner"><Gem skillId="TwisterPlayer" level="19"/></Skill></SkillSet>"#,
    );
    let saved = pair(&o, &xml, &data, None);
    assert_eq!(saved.report().groups.len(), 3);
    assert_eq!(
        saved
            .report()
            .groups
            .iter()
            .map(|g| g.processing_passes)
            .collect::<Vec<_>>(),
        [1, 1, 2]
    );
    let same = pair(&o, &xml, &data, Some(7.));
    assert_eq!(
        same.report()
            .groups
            .iter()
            .map(|g| g.processing_passes)
            .collect::<Vec<_>>(),
        [1, 1, 3]
    );
    let changed = pair(&o, &xml, &data, Some(2.));
    assert_eq!(
        changed
            .report()
            .groups
            .iter()
            .map(|g| g.processing_passes)
            .collect::<Vec<_>>(),
        [1, 2, 2]
    );
    pair(
        &o,
        &document(
            r#"<Skill label="legacy"><Gem level="1"/></Skill><SkillSet id="7"><Skill/></SkillSet><Skill label="legacy-again"><Gem level="1"/></Skill>"#,
        ),
        &data,
        None,
    );
}

#[test]
fn names_levels_flags_unknown_gem_tags_and_minion_maps_pair_complete_processing() {
    let data = CompiledGameData::bundled().unwrap();
    let o = Oracle::new(false);
    let xml = document(
        r#"<SkillSet id="1"><Skill active="true" includeInFullDPS="false" groupCount="-2.5" skillPart="7" mainActiveSkill="0" mainActiveSkillCalcs="2"><FutureGem nameSpec="tWiStEr" level="9999" quality="-3" count="0" enabled="false" enableGlobal1="false" enableGlobal2="true" corrupted="true" corruptLevel="-2" skillPart="4" note="&lt;b&gt;kept&lt;/b&gt;" statSetIndex="9" skillMinion="caller-minion" skillMinionItemSet="2" skillMinionSkill="3"><StatSetIndex grantedEffect="e" index="2"/><StatSetIndex grantedEffect="e" index="7"/><StatSetCalcsIndex grantedEffect="e" index="3"/><MinionSkillIndexLookup grantedEffect="e"><Map skillIndex="2" statSetIndex="4"/></MinionSkillIndexLookup><MinionSkillIndexLookup grantedEffect="e"><FutureMap skillIndex="3" statSetIndex="6"/><Map skillIndex="-0" statSetIndex="-1"/></MinionSkillIndexLookup><MinionSkillIndexLookupCalcs grantedEffect="e"><Map skillIndex="2" statSetIndex="8"/></MinionSkillIndexLookupCalcs></FutureGem><Gem nameSpec="&lt;b&gt;A—Bö&lt;/b&gt;" level="1"/><Gem nameSpec=""/><Gem skillId="TwisterPlayer" level="-4"/><Gem skillId="TwisterPlayer" level="2.5"/><Gem skillId="TwisterPlayer" level="inf"/><Gem skillId="TwisterPlayer" level="-inf"/><Gem skillId="TwisterPlayer" level="nan"/><Gem skillId="TwisterPlayer" level="0x4"/><Gem nameSpec="This Gem Has No Possible Match 987654" level="1"/></Skill></SkillSet>"#,
    );
    let result = pair(&o, &xml, &data, None);
    let gems = &result.report().groups[0].gems;
    assert_eq!(gems[0].number("skillPart"), Some(7.));
    assert_eq!(gems[0].text("nameSpec"), Some("Twister"));
    assert_eq!(gems[0].stat_set["e"].value(), 7.);
    assert!(!gems[0].stat_set.contains_key("index"));
    assert_eq!(gems[1].text("nameSpec"), Some("A-Bo"));
    assert_eq!(gems[1].identity_status, SkillIdentityStatus::AmbiguousName);
    assert_eq!(
        gems.last().unwrap().identity_status,
        SkillIdentityStatus::UnresolvedName
    );
    assert_eq!(gems[2].identity_status, SkillIdentityStatus::Empty);
}

#[test]
fn ambiguous_names_and_source_runtime_errors_preserve_failure_prefixes() {
    let data = CompiledGameData::bundled().unwrap();
    let o = Oracle::new(false);
    let ambiguous = pair(
        &o,
        &document(r#"<Skill><Gem nameSpec="S" level="1"/></Skill>"#),
        &data,
        None,
    );
    assert_eq!(
        ambiguous.report().groups[0].gems[0].identity_status,
        SkillIdentityStatus::AmbiguousName
    );
    for body in [
        r#"<Skill><Gem skillId="TwisterPlayer"/></Skill>"#,
        r#"<Skill><Gem skillId="TwisterPlayer" level="bad"/></Skill>"#,
        r#"<Skill><Gem skillId="TwisterPlayer" level="19"/><Gem nameSpec="[" level="1"/></Skill>"#,
        r#"<Skill><Gem skillId="TwisterPlayer" level="19"/></Skill><Skill><Gem level="1"><MinionSkillIndexLookup grantedEffect="e"><Map skillIndex="bad" statSetIndex="2"/></MinionSkillIndexLookup></Gem></Skill>"#,
        r#"<Skill><Gem skillId="TwisterPlayer" level="19"/></Skill><Skill><Gem level="1"><MinionSkillIndexLookup grantedEffect="e"><Map skillIndex="nan" statSetIndex="2"/></MinionSkillIndexLookup></Gem></Skill>"#,
        r#"<SkillSet id="7"/><Skill><Gem skillId="TwisterPlayer" level="19"/></Skill>"#,
        r#"<Skill><Gem skillId="TwisterPlayer" level="19"/></Skill><Skill>text<Gem/></Skill>"#,
    ] {
        let actual = pair(&o, &document(body), &data, None);
        assert_eq!(
            actual.report().status,
            SkillPreparationStatus::SourceFailure,
            "{body}"
        );
    }
}

#[test]
fn caller_injected_level_requirement_changes_both_native_and_original_source_output() {
    let data = CompiledGameData::bundled().unwrap();
    let mut package = data.snapshot().package().clone();
    let effect = package
        .skill_preparation
        .effects
        .iter()
        .find(|e| e.id == "TwisterPlayer")
        .unwrap();
    let row = effect.levels.iter().find(|row| row.key == 19.).unwrap();
    let row_id = row.row_id.clone();
    let changed = row.level_requirement.unwrap() + 7.;
    for row in package
        .skill_preparation
        .effects
        .iter_mut()
        .flat_map(|e| &mut e.levels)
        .filter(|row| row.row_id == row_id)
    {
        row.level_requirement = Some(changed);
    }
    package.refresh_section_digests().unwrap();
    let snapshot = GameDataLoader::from_bytes(
        &serde_json::to_vec(&package).unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    let changed_data = Arc::new(CompiledGameData::compile(Arc::new(snapshot)).unwrap());
    let o = Oracle::new(false);
    let xml = document(r#"<Skill><Gem skillId="TwisterPlayer" level="19" quality="20"/></Skill>"#);
    let baseline = pair(&o, &xml, &data, None);
    o.mutate(&format!(
        "data.skills.TwisterPlayer.levels[19].levelRequirement={changed}"
    ));
    let altered = pair(&o, &xml, &changed_data, None);
    let first = &baseline.report().groups[0].gems[0];
    let second = &altered.report().groups[0].gems[0];
    assert_ne!(first.number("reqLevel"), second.number("reqLevel"));
    assert_ne!(first.number("reqDex"), second.number("reqDex"));
    assert_ne!(baseline.report().data, altered.report().data);
    // The unchanged real original also travels through the injected-data route.
    pair(&o, &corpus(2), &changed_data, None);
}

#[test]
fn external_identity_precedence_and_direct_minion_effect_use_distinct_source_lookups() {
    let data = CompiledGameData::bundled().unwrap();
    let o = Oracle::new(false);
    let xml = document(
        r#"<Skill><Gem gemId="Metadata/Items/Gem/SkillGemTwister" variantId="Twister" skillId="SummonBeastPlayer" level="19"/><Gem gemId="Metadata/Items/Gem/SkillGemTwister" variantId="missing" level="19"/><Gem gemId="missing" skillId="TwisterPlayer" level="19"/><Gem gemId="missing" skillId="TwisterPlayer" nameSpec="Twister" level="19"/><Gem skillId="ExplosiveTeleportSandDjinn" level="1"/></Skill>"#,
    );
    let result = pair(&o, &xml, &data, None);
    let gems = &result.report().groups[0].gems;
    assert_eq!(gems[0].text("skillId"), Some("TwisterPlayer"));
    assert_eq!(gems[1].text("skillId"), Some("TwisterPlayer"));
    assert_eq!(gems[2].identity_status, SkillIdentityStatus::Empty);
    assert_eq!(gems[3].identity_status, SkillIdentityStatus::ResolvedGem);
    assert_eq!(gems[4].identity_status, SkillIdentityStatus::ResolvedEffect);
    assert!(gems[4].gem_data.is_none());
    assert_eq!(
        gems[4].granted_effect.as_deref(),
        Some("ExplosiveTeleportSandDjinn")
    );
    assert_eq!(gems[4].number("reqLevel"), None);
}

#[test]
fn repeated_containers_preserve_control_fallback_and_replace_live_groups() {
    let data = CompiledGameData::bundled().unwrap();
    let o = Oracle::new(false);
    let xml = r#"<PathOfBuilding2><Build level="90"/><Skills defaultGemLevel="corruptedMaximum" defaultGemQuality="30" showSupportGemTypes="LINEAGE" sortGemsByDPSField="TotalDPS" sortGemsByDPS="false" showLegacyGems="true"><Skill><Gem skillId="TwisterPlayer" level="19"/></Skill></Skills><Skills defaultGemLevel="unavailable" defaultGemQuality="-3" showSupportGemTypes="unavailable" sortGemsByDPSField="unavailable"><Skill><Gem skillId="ExplosiveTeleportSandDjinn" level="1"/></Skill></Skills></PathOfBuilding2>"#;
    let actual = pair(&o, xml, &data, None);
    assert_eq!(actual.report().containers.len(), 2);
    assert_eq!(actual.report().groups.len(), 2);
    assert_eq!(
        actual.report().selected_groups,
        [actual.report().groups[1].instance]
    );
}

#[test]
fn independent_original_source_observes_all_five_selected_authored_working_sets() {
    let o = Oracle::new(false);
    for (index, key, entries) in [
        (1, 1., 62),
        (2, 6., 46),
        (3, 1., 62),
        (4, 1., 62),
        (5, 4., 28),
    ] {
        let observation = o.observe(&corpus(index), None);
        assert!(
            observation.get::<bool>("success").unwrap(),
            "original {index}: {:?}",
            observation.get::<Option<String>>("error").unwrap()
        );
        same_number(
            observation.get("selected_key").unwrap(),
            key,
            "original selected key",
        );
        let selected: Vec<u32> = observation
            .get::<Table>("selected_groups")
            .unwrap()
            .sequence_values()
            .map(Result::unwrap)
            .collect();
        let groups: Vec<Table> = observation
            .get::<Table>("groups")
            .unwrap()
            .sequence_values()
            .map(Result::unwrap)
            .collect();
        let count: usize = groups
            .iter()
            .filter(|group| selected.contains(&group.get::<u32>("source").unwrap()))
            .map(|group| group.get::<Table>("gems").unwrap().raw_len())
            .sum();
        assert_eq!(count, entries, "original {index}");
        eprintln!(
            "original={index}, groups={}, authored_entries={}, selected_entries={count}",
            groups.len(),
            groups
                .iter()
                .map(|g| g.get::<Table>("gems").unwrap().raw_len())
                .sum::<usize>()
        );
    }
}
