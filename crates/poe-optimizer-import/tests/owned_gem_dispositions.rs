//! Physical assignment closure requires exhaustive source dispositions, not
//! fabricated usage preferences or complete definition/calculation coverage.
#[allow(dead_code)]
#[path = "support/source_actions_fixture.rs"]
mod actions;
#[path = "support/gem_dispositions_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{owned_draft::*, owned_schema::*};
use poe_optimizer_import::{
    owned_normalize::*, owned_source_actions::*, owned_value::*, owned_value_policy::*,
};

fn completion(result: &NormalizedImport) -> Vec<bool> {
    result
        .draft()
        .input()
        .gems
        .members
        .iter()
        .map(|gem| matches!(gem.parameters.completion, DraftListCompletion::Complete))
        .collect()
}
fn run(f: &Fixture, text: &str) -> NormalizedImport {
    f.run(text, Default::default()).unwrap()
}
fn pending_usage(result: &NormalizedImport) {
    for preset in &result.draft().input().skill_presets.members {
        let usage = preset
            .usage_preferences
            .as_ref()
            .expect("actual usage obligation");
        assert!(usage.members.is_empty(), "no invented UsagePolicy records");
        let DraftListCompletion::Pending { id, code } = &usage.completion else {
            panic!("physical closure cannot close usage");
        };
        assert_eq!(code.as_str(), "usage-preferences-not-converted");
        assert!(result.sidecar().origins.iter().any(|row| {
            row.links.contains(&OwnedOriginTarget::Issue(*id))
                && row
                    .links
                    .contains(&OwnedOriginTarget::SkillPreset(preset.id))
        }));
    }
}

#[test]
fn intrinsic_inventory_closes_without_a_usage_policy_and_preserves_real_obligations() {
    let f = Fixture::new();
    assert!(f.policy.usage_inputs.is_none());
    let before = serde_json::to_vec(&f.policy).unwrap();
    for raw in [
        GEM.to_owned(),
        with_children(&maps("2", "1")),
        GEM.replace("enabled=\"true\"", "enabled=\"false\""),
        GEM.replace("corrupted=\"false\"", "corrupted=\"nil\"")
            .replace("corruptLevel=\"0\"", "corruptLevel=\"nil\""),
    ] {
        let result = run(&f, &xml(&raw));
        assert_eq!(completion(&result), [true], "{raw}");
        assert_eq!(
            result.draft().input().gems.members[0]
                .parameters
                .members
                .len(),
            2
        );
        assert!(
            result.draft().input().skills.members[0]
                .parameters
                .is_none()
        );
        pending_usage(&result);
    }
    let SchemaLookup::Known(gem) = f.base.schema.definition(&f.base.gem) else {
        unreachable!()
    };
    assert!(!gem.declarations.parameters.is_complete());
    assert_eq!(serde_json::to_vec(&f.policy).unwrap(), before);
}

#[test]
fn every_reference_child_is_accounted_and_legacy_headers_are_only_overwritten_evidence() {
    let f = Fixture::new();
    let explicit = with_children(&maps("2", "1"));
    let result = run(&f, &xml(&explicit));
    assert_eq!(completion(&result), [true]);
    let gem = result.draft().input().gems.members[0].id;
    let skill = result.draft().input().skills.members[0].id;
    let rows: Vec<_> = result
        .sidecar()
        .origins
        .iter()
        .filter(|row| row.links == [OwnedOriginTarget::Gem(gem), OwnedOriginTarget::Skill(skill)])
        .collect();
    assert_eq!(
        rows.len(),
        2,
        "both exact reference child origins are linked"
    );
    for raw in [
        GEM.replace("/>", " statSetIndex=\"99\" statSetIndexCalcs=\"garbage\"/>"),
        explicit.replace("<Gem ", "<Gem statSetIndex=\"garbage\" "),
    ] {
        assert_eq!(completion(&run(&f, &xml(&raw))), [true]);
    }
    for children in [
        "<StatSetIndex grantedEffect=\"other-effect\" index=\"1\"/>".into(),
        format!(
            "{}<StatSetIndex grantedEffect=\"other-effect\" index=\"1\"/>",
            maps("1", "2")
        ),
        format!("{}{}", maps("1", "2"), maps("2", "1")),
        maps("unknown", "1"),
        maps("1", "3"),
        "<StatSetIndex grantedEffect=\"effect\"/>".into(),
        "<StatSetIndex xmlns=\"urn:foreign\" grantedEffect=\"effect\" index=\"1\"/>".into(),
        "<StatSetIndex grantedEffect=\"effect\" index=\"1\" future=\"true\"/>".into(),
        "<Future/>".into(),
    ] {
        assert_eq!(
            completion(&run(&f, &xml(&with_children(&children)))),
            [false],
            "{children}"
        );
    }
    let mut f = Fixture::new();
    let SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
        absent_stat_set, ..
    } = &mut f.row().reference_action
    else {
        panic!("historical player adapter")
    };
    *absent_stat_set = None;
    assert_eq!(completion(&run(&f, &xml(GEM))), [false]);
    assert_eq!(completion(&run(&f, &xml(&explicit))), [true]);
}

#[test]
fn deferred_values_classify_finite_syntax_without_computing_count_or_usage() {
    let f = Fixture::new();
    for count in ["0", "3", "-1.5", "1e2"] {
        let text = xml(&GEM.replace("count=\"1\"", &format!("count=\"{count}\""))).replace(
            "<Skill enabled=\"true\">",
            "<Skill enabled=\"true\" groupCount=\"0\" includeInFullDPS=\"false\">",
        );
        let result = run(&f, &text);
        assert_eq!(completion(&result), [true], "{count}");
        pending_usage(&result);
    }
    for field in ["count", "enableGlobal1", "enableGlobal2"] {
        let original = if field == "count" { "1" } else { "true" };
        for replacement in ["", "bad", "NaN", " true", "1 "] {
            let raw = GEM.replace(
                &format!("{field}=\"{original}\""),
                &format!("{field}=\"{replacement}\""),
            );
            assert_eq!(
                completion(&run(&f, &xml(&raw))),
                [false],
                "{field}: {replacement:?}"
            );
        }
        let raw = GEM.replace(&format!(" {field}=\"{original}\""), "");
        assert_eq!(completion(&run(&f, &xml(&raw))), [false], "missing {field}");
    }
    for attributes in [
        "groupCount=\"bad\"",
        "groupCount=\"nil\"",
        "includeInFullDPS=\"bad\"",
        "mainActiveSkill=\"2\"",
        "mainActiveSkillCalcs=\"2\"",
    ] {
        let text = xml(GEM).replace(
            "<Skill enabled=\"true\">",
            &format!("<Skill enabled=\"true\" {attributes}>"),
        );
        assert_eq!(completion(&run(&f, &text)), [false], "{attributes}");
    }
    for field in ["enableGlobal1", "enableGlobal2"] {
        let result = run(
            &f,
            &xml(&GEM.replace(&format!("{field}=\"true\""), &format!("{field}=\"false\""))),
        );
        assert_eq!(completion(&result), [true]);
        pending_usage(&result);
    }
}

#[test]
fn exact_occurrences_and_archived_presets_cannot_borrow_dispositions() {
    let f = Fixture::new();
    let bad = GEM.replace("count=\"1\"", "count=\"bad\"");
    let off = GEM.replace("enabled=\"true\"", "enabled=\"false\"");
    let text = format!(
        "<PathOfBuilding2><Skills activeSkillSet=\"2\"><SkillSet id=\"1\"><Skill enabled=\"true\">{GEM}{bad}{GEM}</Skill></SkillSet><SkillSet id=\"2\"><Skill enabled=\"false\">{off}{bad}</Skill></SkillSet></Skills></PathOfBuilding2>"
    );
    let result = run(&f, &text);
    assert_eq!(completion(&result), [true, false, true, true, false]);
    pending_usage(&result);
    let presets = &result.draft().input().skill_presets.members;
    assert_ne!(
        presets[0].usage_preferences.as_ref().unwrap().completion,
        presets[1].usage_preferences.as_ref().unwrap().completion
    );
    for row in &result.sidecar().origins {
        for (index, link) in row.links.iter().enumerate() {
            assert!(
                !row.links[..index].contains(link),
                "duplicate provenance link"
            );
        }
    }
    let no_preset = format!(
        "<PathOfBuilding2><Skills><Skill enabled=\"true\">{GEM}</Skill></Skills></PathOfBuilding2>"
    );
    let result = run(&f, &no_preset);
    assert_eq!(completion(&result), [false]);
    assert!(result.draft().input().skill_presets.members.is_empty());
}

#[test]
fn unknown_source_shape_or_intrinsic_inputs_remain_pending() {
    let f = Fixture::new();
    for raw in [
        GEM.replace("/>", " future=\"true\"/>"),
        GEM.replace("skillId=\"effect\"", "skillId=\"unreviewed\""),
        GEM.replace("nameSpec=\"Fixture skill\"", "nameSpec=\"Other\""),
        GEM.replace("level=\"17\"", "level=\"0\""),
        GEM.replace("quality=\"0\"", "quality=\"101\""),
        GEM.replace("corruptLevel=\"0\"", "corruptLevel=\"11\""),
        GEM.replace("corrupted=\"false\"", "corrupted=\"unknown\""),
        with_children("text"),
    ] {
        assert_eq!(completion(&run(&f, &xml(&raw))), [false], "{raw}");
    }
    // Foreign namespaces do not enter the owned physical-instance domain at
    // all; the containing preset's unconverted skill inventory remains Pending.
    let foreign = run(
        &f,
        &xml(&GEM.replace("<Gem ", "<Gem xmlns=\"urn:future\" ")),
    );
    let draft = foreign.draft().input();
    assert!(draft.gems.members.is_empty());
    assert!(draft.skills.members.is_empty());
    assert!(matches!(
        draft.skill_presets.members[0].skills.completion,
        DraftListCompletion::Pending { .. }
    ));
    for extra in ["<Future/>", "text", "<Gem xmlns=\"urn:future\"/>"] {
        assert!(!completion(&run(&f, &xml(&format!("{GEM}{extra}"))))[0]);
    }
    for attribute in ["slot=\"Helmet\"", "future=\"true\""] {
        let text = xml(GEM).replace(
            "<Skill enabled=\"true\">",
            &format!("<Skill enabled=\"true\" {attribute}>"),
        );
        assert_eq!(completion(&run(&f, &text)), [false]);
    }
}

#[test]
fn canonical_skill_set_inventory_rejects_aliases_and_unreviewed_ancestors() {
    let f = Fixture::new();
    let baseline = xml(GEM);
    for text in [
        baseline.replace("</Skills>", "<SkillSet id=\"1\"/></Skills>"),
        baseline.replace("</Skills>", "<SkillSet id=\"01\"/></Skills>"),
        baseline.replace("<SkillSet id=\"1\">", "<SkillSet id=\"1\" future=\"true\">"),
        baseline.replace(
            "</PathOfBuilding2>",
            "<Skills activeSkillSet=\"1\"/></PathOfBuilding2>",
        ),
        baseline.replace("activeSkillSet=\"1\"", "activeSkillSet=\"2\""),
        baseline.replace(" activeSkillSet=\"1\"", ""),
    ] {
        assert!(
            completion(&run(&f, &text)).iter().all(|complete| !complete),
            "{text}"
        );
    }
}

#[test]
fn source_and_artifact_bindings_and_finite_recipe_contract_are_checked() {
    for case in 0..17 {
        let mut f = Fixture::new();
        match case {
            0 => f.row().deferred_usage.pop().map(|_| ()).unwrap(),
            1 => f.row().deferred_usage[1].field = DeferredGemUsageField::GemCount,
            2 => f.row().deferred_usage[0].value.tiers[0].selectors[0].name = "groupCount".into(),
            3 => {
                f.row().deferred_usage[0].value.tiers[0].selectors[0].lane = ValueLane::InputString
            }
            4 => f.row().deferred_usage[0].value.missing = MissingValuePolicy::Absent,
            5 => f.row().deferred_usage[3].value.missing = MissingValuePolicy::Pending,
            6 => f.row().deferred_usage[0]
                .value
                .numeric_aliases
                .push(NumericTokenAlias {
                    token: "nil".into(),
                    replacement: "0".into(),
                }),
            7 => {
                f.row().deferred_usage[0].value.codec.codec = ValueCodecKind::Integer {
                    syntax: DecimalSyntax::Integer,
                }
            }
            8 => {
                f.row().deferred_usage[1].value.codec.codec =
                    f.row().deferred_usage[0].value.codec.codec.clone()
            }
            9 => {
                let ValueCodecKind::Quantity { scale, .. } =
                    &mut f.row().deferred_usage[0].value.codec.codec
                else {
                    unreachable!()
                };
                scale.numerator =
                    poe_optimizer_core::owned_definitions::BoundedInteger::new(2).unwrap();
            }
            10 => {
                let SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
                    definitions,
                    ..
                } = &mut f.row().reference_action
                else {
                    panic!("historical player adapter")
                };
                definitions.release = "stale".into();
            }
            11 => {
                let SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
                    source, ..
                } = &mut f.row().reference_action
                else {
                    panic!("historical player adapter")
                };
                source.revision = "d".repeat(40);
            }
            12 => f.row().physical.name_spec = "Other".into(),
            _ => {
                let GemInventoryPolicy::PobFreshPhysicalV3 {
                    roles,
                    catalog,
                    scalar_inputs,
                    usage_inputs,
                    primary_dispositions,
                    ..
                } = f.policy.gem_inventory.as_mut().unwrap()
                else {
                    unreachable!()
                };
                match case {
                    13 => *roles = *catalog,
                    14 => *scalar_inputs = *roles,
                    15 => *usage_inputs = *roles,
                    16 => primary_dispositions.push(primary_dispositions[0].clone()),
                    _ => unreachable!(),
                }
            }
        }
        assert!(
            f.run(&xml(GEM), Default::default()).is_err(),
            "invalid case {case}"
        );
    }
}

#[test]
fn v3_is_explicit_and_empty_extension_keeps_absent_inventory_allocation_and_sidecar() {
    let mut f = Fixture::new();
    let bytes = serde_json::to_vec(&f.policy).unwrap();
    let decoded: NormalizationPolicy = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);
    let mut unknown = serde_json::to_value(&decoded).unwrap();
    unknown["gem_inventory"]["primary_dispositions"][0]["ignore_unknown"] = true.into();
    assert!(serde_json::from_value::<NormalizationPolicy>(unknown).is_err());
    let GemInventoryPolicy::PobFreshPhysicalV3 {
        primary_dispositions,
        ..
    } = f.policy.gem_inventory.as_mut().unwrap()
    else {
        unreachable!()
    };
    primary_dispositions.clear();
    let v3 = run(&f, &xml(GEM));
    let GemInventoryPolicy::PobFreshPhysicalV3 {
        definitions,
        roles,
        catalog,
        scalar_inputs,
        usage_inputs,
        supports,
        primary_skills,
        ..
    } = f.policy.gem_inventory.take().unwrap()
    else {
        unreachable!()
    };
    // Historical V2 requires an actual usage policy, even with zero rows; use
    // the still older absent inventory to prove the empty V3 creates no inputs.
    let v2 = GemInventoryPolicy::PobFreshPhysicalV2 {
        definitions,
        roles,
        catalog,
        scalar_inputs,
        usage_inputs,
        supports,
        primary_skills,
    };
    let bytes = serde_json::to_vec(&v2).unwrap();
    assert_eq!(
        serde_json::to_vec(&serde_json::from_slice::<GemInventoryPolicy>(&bytes).unwrap()).unwrap(),
        bytes
    );
    assert!(
        serde_json::to_value(v2)
            .unwrap()
            .get("primary_dispositions")
            .is_none()
    );
    let absent = run(&f, &xml(GEM));
    assert_eq!(v3.draft(), absent.draft());
    assert_eq!(v3.allocator_after(), absent.allocator_after());
    let mut sidecar = serde_json::to_value(v3.sidecar()).unwrap();
    let expected = serde_json::to_value(absent.sidecar()).unwrap();
    sidecar["policy"] = expected["policy"].clone();
    assert_eq!(sidecar, expected);
}

#[test]
fn aggregate_work_and_value_limits_are_tightenable_and_repeated_calls_are_isolated() {
    let f = Fixture::new();
    let text = xml(&with_children(&maps("2", "1")));
    let a = run(&f, &text);
    assert_eq!(completion(&a), [true]);
    let b = run(&f, &xml(&GEM.replace("count=\"1\"", "count=\"bad\"")));
    assert_eq!(completion(&b), [false]);
    let again = run(&f, &text);
    assert_eq!(a.draft(), again.draft());
    assert_eq!(
        serde_json::to_vec(a.sidecar()).unwrap(),
        serde_json::to_vec(again.sidecar()).unwrap()
    );
    let mut low = 0;
    let mut high = NormalizationLimits::default().max_work;
    while high - low > 1 {
        let midpoint = low + (high - low) / 2;
        let limits = NormalizationLimits {
            max_work: midpoint,
            ..Default::default()
        };
        if f.run(&text, limits).is_ok() {
            high = midpoint
        } else {
            low = midpoint
        }
    }
    assert!(
        f.run(
            &text,
            NormalizationLimits {
                max_work: high,
                ..Default::default()
            }
        )
        .is_ok()
    );
    assert!(
        f.run(
            &text,
            NormalizationLimits {
                max_work: low,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        f.run(
            &text,
            NormalizationLimits {
                max_policy_bytes: 100,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut limits = NormalizationLimits::default();
    limits.value.max_total_candidate_bytes = 20;
    assert!(f.run(&xml(GEM), limits).is_ok());
    let oversized = xml(GEM).replace(
        "<Skill enabled=\"true\">",
        "<Skill enabled=\"true\" groupCount=\"12345678901\">",
    );
    assert!(f.run(&oversized, limits).is_err());
}
