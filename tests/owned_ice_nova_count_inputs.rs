//! Source-backed count input transport, independent of complete build evaluation.
#[path = "support/owned_ice_nova_count_inputs.rs"]
mod packet;

use packet::{Fixture, gem, group, values, xml};
use poe_optimizer_core::{
    owned_build::{GeneratedSkillKey, ProviderKey, ProviderRoot, SkillTarget, UsageTarget},
    owned_draft::DraftListCompletion,
};
use poe_optimizer_import::{
    decode_build,
    owned_normalize::{
        OwnedOriginTarget, UsageFallbackAdmission, UsageInputPolicy, UsageValueSource,
    },
};
use serde_json::{Value, json};

fn selected(state: &Value) -> Vec<&Value> {
    state["saved"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["selected"] == true)
        .collect()
}
fn case(name: &str) -> &'static Value {
    packet::source_report()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap()
}

#[test]
fn authenticated_packet_binds_count_policy_exact_source_and_reviewed_companions() {
    packet::check_authored();
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(
            packet::root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let pins = packet::source_report()["files"].as_array().unwrap();
    for mutation in 0..5 {
        let mut bad = pins.clone();
        match mutation {
            0 => bad[0]["path"] = json!("src/Modules/Unreviewed.lua"),
            1 => bad[0]["sha256"] = json!("0".repeat(64)),
            2 => bad[1] = bad[0].clone(),
            3 => bad[0]["bytes"] = json!(1),
            _ => {
                bad.pop();
            }
        }
        assert!(
            std::panic::catch_unwind(|| packet::check_observation_file_pins(&bad, &manifest))
                .is_err()
        );
    }
    let proof: Value = packet::read("source-evidence.json");
    assert_eq!(
        proof["semantics"],
        json!({
            "domain":{"kind":"integer","minimum":0,"maximum":4},
            "gameplay_maximum_proven":false,
            "group_override":"attribute_presence_including_zero",
            "malformed_winning_lane":"pending_no_fallback",
            "malformed_shadowed_occurrence":"not_selected",
            "missing":"pending",
            "duplicate_primary_without_group":"pending",
            "reviewed_companions":5,
            "complete_usage_inventory":false,
            "native_count_consumer_added":false,
            "full_dps_aggregation_added":false
        })
    );
    let mut bad = proof.clone();
    bad["report"]["raw_sha256"] = json!("0".repeat(64));
    assert!(std::panic::catch_unwind(|| packet::authenticate_source(&bad)).is_err());
    let mut bad = proof;
    bad["semantics"]["complete_usage_inventory"] = json!(true);
    assert!(std::panic::catch_unwind(|| packet::authenticate_source(&bad)).is_err());
}

#[test]
fn retained_controls_separate_exact_count_source_from_per_copy_damage_and_aggregate_labels() {
    for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
        for (name, raw, effective) in [
            ("count-zero", 0, 0),
            ("count-three", 3, 3),
            ("group-zero-over-count-three", 3, 0),
            ("group-four-over-count-three", 3, 4),
            ("full-dps-count-one", 1, 1),
            ("full-dps-count-three", 3, 3),
        ] {
            let saved = selected(&case(name)["states"][stage]);
            assert_eq!(saved.len(), 1);
            assert_eq!(saved[0]["source_ordinal"], 239);
            assert_eq!(saved[0]["group_source_ordinal"], 238);
            assert_eq!(saved[0]["loaded"]["count"], raw);
            for mode in ["MAIN", "CALCS"] {
                let actions = saved[0][mode].as_array().unwrap();
                assert!(!actions.is_empty());
                for action in actions {
                    assert_eq!(action["count"], effective);
                    assert_eq!(action["exact_physical_object"], true);
                    assert_eq!(action["exact_group"], true);
                    assert_eq!(action["source_ordinal"], saved[0]["source_ordinal"]);
                    assert_eq!(action["count_candidates"].as_array().unwrap().len(), 1);
                    assert_eq!(action["count_candidates"][0]["exact_action_source"], true);
                }
            }
        }
        for mode in ["MAIN", "CALCS"] {
            let full = |name| {
                case(name)["states"][stage]["full_dps"][mode]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|r| r["name"] == "Ice Nova")
                    .collect::<Vec<_>>()
            };
            let one = full("full-dps-count-one");
            let three = full("full-dps-count-three");
            assert_eq!((one.len(), three.len()), (1, 1));
            assert_eq!(one[0]["count"], 1);
            assert_eq!(three[0]["count"], 3);
            assert_eq!(one[0]["dps"], three[0]["dps"]);
            assert!(one[0]["dps"].as_f64().unwrap() > 0.0);
        }
        let duplicate = selected(&case("duplicate-independent-inputs")["states"][stage]);
        assert_eq!(duplicate.len(), 2);
        assert_ne!(
            duplicate[0]["source_ordinal"],
            duplicate[1]["source_ordinal"]
        );
        assert_eq!(duplicate[1]["loaded"]["count"], 3);
        for mode in ["MAIN", "CALCS"] {
            assert_eq!(duplicate[1][mode][0]["count"], 1);
            assert_eq!(
                duplicate[1][mode][0]["count_candidates"]
                    .as_array()
                    .unwrap()
                    .len(),
                2
            );
        }
        for name in ["count-missing", "count-malformed"] {
            assert_eq!(
                selected(&case(name)["states"][stage])[0]["loaded"]["count"],
                1
            );
        }
    }
    // The raw loader's missing/malformed defaults and label-only aggregate rows
    // are observations, never authorizations for defaults or aggregate identity.
    assert_eq!(
        packet::source_report()["full_dps_rows_have_physical_identity"],
        false
    );
}

#[test]
fn count_zero_group_override_and_malformed_selected_values_do_not_invent_defaults() {
    let f = Fixture::new();
    for (count, parent, expected) in [
        (Some("0"), "", Some(0)),
        (Some("1"), "", Some(1)),
        (Some("2"), "", Some(2)),
        (Some("3"), "", Some(3)),
        (Some("4"), "", Some(4)),
        (Some("3"), r#"groupCount="0""#, Some(0)),
        (Some("3"), r#"groupCount="4""#, Some(4)),
        (Some("bad"), r#"groupCount="0""#, Some(0)),
        (None, r#"groupCount="4""#, Some(4)),
        (Some("3"), r#"groupCount="bad""#, None),
        (Some("3"), r#"groupCount="5""#, None),
        (Some("3"), r#"groupCount="3.5""#, None),
        (Some("3"), r#"groupCount=" 4""#, None),
        (None, "", None),
        (Some("nil"), "", None),
        (Some("bad"), "", None),
        (Some("-1"), "", None),
        (Some("5"), "", None),
        (Some("3.5"), "", None),
        (Some(" 3"), "", None),
    ] {
        let input = xml(&group(&gem(f.row(), count), parent));
        let result = f.run(&input).unwrap();
        assert_eq!(values(&result), vec![vec![expected]], "{count:?} {parent}");
    }
    assert!(
        decode_build(
            xml(&group(
                &gem(f.row(), Some("3")),
                r#"groupCount="0" groupCount="4""#
            ))
            .as_bytes()
        )
        .is_err()
    );
    assert!(
        decode_build(
            xml(&group(
                &gem(f.row(), Some("3")).replace(r#"count="3""#, r#"count="3" count="4""#),
                ""
            ))
            .as_bytes()
        )
        .is_err()
    );
}

#[test]
fn unique_reviewed_companions_are_required_only_for_occurrence_fallback() {
    let f = Fixture::new();
    let UsageValueSource::ContainingGroupOverride {
        fallback_admission: UsageFallbackAdmission::UniqueReviewedPrimary { companions },
        ..
    } = &f.row().policies[0].parameters[0].source
    else {
        panic!();
    };
    let main = gem(f.row(), Some("3"));
    let companion = |index: usize| {
        let c = &companions[index];
        format!(
            r#"<Gem gemId="{}" variantId="{}" skillId="{}" nameSpec="{}" level="1" enabled="false" count="bad"/>"#,
            c.game_id, c.variant_id, c.skill_id, c.name_spec
        )
    };
    for index in 0..companions.len() {
        let input = xml(&group(&format!("{main}{}", companion(index)), ""));
        assert_eq!(values(&f.run(&input).unwrap()), vec![vec![Some(3)]]);
    }
    let all = (0..companions.len()).map(companion).collect::<String>();
    assert_eq!(
        values(&f.run(&xml(&group(&format!("{main}{all}"), ""))).unwrap()),
        vec![vec![Some(3)]]
    );
    for sibling in [
        r#"<Gem gemId="unreviewed" variantId="unreviewed" skillId="unknown" nameSpec="Unknown" level="1" enabled="false"/>"#.to_owned(),
        companion(0).replace("SupportEncroachingGroundPlayer", "IceNovaPlayer"),
        "<Unsupported/>".to_owned(),
    ] {
        let result = f.run(&xml(&group(&format!("{main}{sibling}"), ""))).unwrap();
        assert_eq!(values(&result), vec![vec![None]]);
        let result = f.run(&xml(&group(&format!("{main}{sibling}"), r#"groupCount="0""#))).unwrap();
        assert_eq!(values(&result), vec![vec![Some(0)]]);
    }
}

#[test]
fn duplicate_physical_sources_preserve_independent_targets_without_first_copy_fallback() {
    let f = Fixture::new();
    let duplicates = format!("{}{}", gem(f.row(), Some("1")), gem(f.row(), Some("3")));
    for (parent, expected) in [
        ("", None),
        (r#"groupCount="0""#, Some(0)),
        (r#"groupCount="4""#, Some(4)),
    ] {
        let result = f.run(&xml(&group(&duplicates, parent))).unwrap();
        assert_eq!(values(&result), vec![vec![expected, expected]]);
        let preset = &result.draft().input().skill_presets.members[0];
        let preferences = &preset.usage_preferences.as_ref().unwrap().members;
        assert_eq!(preset.skills.members.len(), 2);
        for (record, source) in preferences.iter().zip(&preset.skills.members) {
            assert_eq!(
                record.target.to_resolved(),
                Some(UsageTarget::Skill(SkillTarget::Generated(Box::new(
                    GeneratedSkillKey {
                        provider: ProviderKey {
                            root: ProviderRoot::SkillUse(*source),
                            grant_path: vec![]
                        },
                        slot: f.row().supply.clone()
                    }
                ))))
            );
        }
        assert_ne!(
            preferences[0].target.to_resolved(),
            preferences[1].target.to_resolved()
        );
    }
}

#[test]
fn independent_presets_disabled_sources_and_origin_links_survive_transport() {
    let f = Fixture::new();
    let first = group(
        &gem(f.row(), Some("1")).replace(r#"enabled="true""#, r#"enabled="false""#),
        r#"groupCount="0""#,
    )
    .replacen(r#"enabled="true""#, r#"enabled="false""#, 1);
    let second = group(&gem(f.row(), Some("3")), "");
    let input = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="8"><SkillSet id="7">{first}</SkillSet><SkillSet id="8">{second}</SkillSet></Skills></PathOfBuilding2>"#
    );
    let result = f.run(&input).unwrap();
    assert_eq!(values(&result), vec![vec![Some(0)], vec![Some(3)]]);
    assert_eq!(
        result.draft().input().skills.members[0]
            .enabled
            .to_resolved(),
        Some(false)
    );
    let again = f.run(&input).unwrap();
    assert_eq!(result.draft().input(), again.draft().input());
    assert_eq!(
        serde_json::to_vec(result.sidecar()).unwrap(),
        serde_json::to_vec(again.sidecar()).unwrap()
    );
    for preset in &result.draft().input().skill_presets.members {
        assert!(matches!(
            preset.usage_preferences.as_ref().unwrap().completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(
            result
                .sidecar()
                .origins
                .iter()
                .filter(|o| o.links.contains(&OwnedOriginTarget::SkillPreset(preset.id)))
                .count()
                >= 3
        );
    }
}

#[test]
fn count_append_is_exact_and_rejects_missing_ambiguous_or_already_converted_sources() {
    let mut f = Fixture::new();
    let expected = packet::usage();
    let UsageInputPolicy::PobOccurrenceUsageV3 { physical, .. } =
        f.policy.usage_inputs.as_mut().unwrap();
    let mut prior = expected.clone();
    prior.policies.clear();
    physical[0] = prior;
    let before = f.policy.clone();
    packet::checked_append(&mut f.policy).unwrap();
    let mut inverse = f.policy.clone();
    let UsageInputPolicy::PobOccurrenceUsageV3 { physical, .. } =
        inverse.usage_inputs.as_mut().unwrap();
    assert_eq!(physical[0], expected);
    physical[0].policies.clear();
    assert_eq!(inverse, before);
    assert!(packet::checked_append(&mut f.policy).is_err());
    for mutation in 0..3 {
        let mut bad = before.clone();
        let UsageInputPolicy::PobOccurrenceUsageV3 { physical, .. } =
            bad.usage_inputs.as_mut().unwrap();
        match mutation {
            0 => physical.clear(),
            1 => physical.push(physical[0].clone()),
            _ => physical[0].skill_id = "other-effect".into(),
        }
        let unchanged = bad.clone();
        assert!(packet::checked_append(&mut bad).is_err());
        assert_eq!(bad, unchanged);
    }
}

#[test]
fn dependencies_and_companion_identity_cannot_be_silently_rebound() {
    let f = Fixture::new();
    let input = xml(&group(&gem(f.row(), Some("3")), ""));
    for mutate in 0..3 {
        let mut bad = Fixture::new();
        let UsageInputPolicy::PobOccurrenceUsageV3 {
            physical, roles, ..
        } = bad.policy.usage_inputs.as_mut().unwrap();
        match mutate {
            0 => *roles = "0".repeat(64).parse().unwrap(),
            1 => {
                physical[0].primary = {
                    let mut v = json!(physical[0].primary);
                    v["key"] = json!("def.000000000000ffff");
                    serde_json::from_value(v).unwrap()
                }
            }
            _ => {
                let gem = physical[0].gem.clone();
                let UsageValueSource::ContainingGroupOverride {
                    fallback_admission: UsageFallbackAdmission::UniqueReviewedPrimary { companions },
                    ..
                } = &mut physical[0].policies[0].parameters[0].source
                else {
                    panic!();
                };
                companions[0].gem = gem;
            }
        }
        assert!(bad.run(&input).is_err());
    }
}
