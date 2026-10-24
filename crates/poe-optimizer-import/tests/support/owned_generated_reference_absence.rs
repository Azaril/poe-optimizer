//! Absent archived reference/count syntax retains unresolved owners, not values.
//! The synthetic Tree identity is independently configured by the shared fixture.
use super::*;
use poe_optimizer_import::owned_source::SourceEvidenceRow;

fn without_topology() -> Fixture {
    let mut f = configured();
    f.base.policy.direct_skill_inputs = None;
    f.base.policy.gem_inventory = None;
    assert!(f.base.policy.usage_inputs.is_none());
    f
}

fn archived_document(gem: &str, group_attributes: &str) -> String {
    // Keep an explicitly selected empty preset. Neither topology policy is
    // needed to account for an unrelated manual sibling in this fixture.
    without_manual_sibling(&document(gem))
        .replace(
            &format!("<Skill enabled=\"true\">{}</Skill>", inventory::GEM),
            "",
        )
        .replace(
            "source=\"Tree:7\"",
            &format!("source=\"Tree:7\"{group_attributes}"),
        )
}

fn absent_gem(count: Option<&str>) -> String {
    generated().replace(
        "count=\"1\"",
        &count.map_or_else(String::new, |value| format!("count=\"{value}\"")),
    )
}

fn no_outputs(result: &NormalizedImport) {
    let input = result.draft().input();
    assert!(input.skills.members.is_empty());
    assert!(input.gems.members.is_empty());
    assert!(input.supports.members.is_empty());
    assert!(input.items.members.is_empty());
    assert!(input.equipment.members.is_empty());
    assert!(input.allocations.members.is_empty());
    assert!(result.sidecar().origins.iter().all(|row| {
        !row.links.iter().any(|link| {
            matches!(
                link,
                OwnedOriginTarget::GeneratedSkillInput { .. }
                    | OwnedOriginTarget::Skill(_)
                    | OwnedOriginTarget::Gem(_)
                    | OwnedOriginTarget::Support(_)
            )
        })
    }));
}

fn assert_exact_accounting(
    baseline: &NormalizedImport,
    actual: &NormalizedImport,
    baseline_text: &str,
    text: &str,
) {
    assert_eq!(actual.draft(), baseline.draft());
    assert_eq!(actual.allocator_after(), baseline.allocator_after());
    let owners = three_owners(actual);
    assert_eq!(owners, three_owners(baseline));
    let preset = actual.draft().input().skill_presets.members[0].id;
    let configuration = configuration(baseline);
    let mut expected = baseline.sidecar().origins.clone();
    let source = |xml: &str| {
        ImportedBuildInstance::from_decoded(
            decode_build(xml.as_bytes()).unwrap(),
            BuildLineage::from_bytes([71; 16]),
            Default::default(),
        )
        .unwrap()
    };
    let before = source(baseline_text);
    let after = source(text);
    let before_rows = SourceProjectEvidence::collect(&before, Default::default()).unwrap();
    let after_rows = SourceProjectEvidence::collect(&after, Default::default()).unwrap();
    assert_eq!(before_rows.rows().len(), after_rows.rows().len());
    assert_eq!(expected.len(), before_rows.rows().len());
    assert_eq!(actual.sidecar().origins.len(), after_rows.rows().len());
    let selected_pair = pair(text);
    assert_eq!(selected_pair, pair(baseline_text));
    for (ordinal, (old, new)) in before_rows.rows().iter().zip(after_rows.rows()).enumerate() {
        let old_occurrence = old.occurrence();
        let new_occurrence = new.occurrence();
        assert_eq!(expected[ordinal].source, old_occurrence.id());
        assert_eq!(
            actual.sidecar().origins[ordinal].source,
            new_occurrence.id()
        );
        assert_eq!(old_occurrence.id().ordinal(), new_occurrence.id().ordinal());
        assert_eq!(old_occurrence.name(), new_occurrence.name());
        assert_eq!(old_occurrence.namespace(), new_occurrence.namespace());
        assert_eq!(
            old_occurrence.has_namespace_context(),
            new_occurrence.has_namespace_context()
        );
        assert_eq!(old_occurrence.role(), new_occurrence.role());
        assert_eq!(
            old_occurrence.parent().map(|id| id.ordinal()),
            new_occurrence.parent().map(|id| id.ordinal())
        );
        assert_eq!(
            old.children()
                .iter()
                .map(|id| id.ordinal())
                .collect::<Vec<_>>(),
            new.children()
                .iter()
                .map(|id| id.ordinal())
                .collect::<Vec<_>>()
        );
        let allowed = if ordinal as u32 == selected_pair[0] {
            &[
                "groupCount",
                "includeInFullDPS",
                "mainActiveSkill",
                "mainActiveSkillCalcs",
            ][..]
        } else if ordinal as u32 == selected_pair[1] {
            &["count", "enabled", "enableGlobal1", "enableGlobal2"][..]
        } else {
            &[][..]
        };
        let stable_attributes = |row: &SourceEvidenceRow<'_>| {
            row.attributes()
                .iter()
                .filter(|a| !allowed.contains(&a.origin().name.as_str()))
                .map(|a| {
                    (
                        a.origin().name.clone(),
                        a.origin().namespace.clone(),
                        a.raw().to_owned(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(stable_attributes(old), stable_attributes(new));
        // XML edits change the content-addressed snapshot. Rebind only after
        // proving exact ordinal/frame correspondence against both actual XMLs.
        expected[ordinal].source = new_occurrence.id();
    }
    for ordinal in selected_pair {
        assert!(is_fallback(baseline, ordinal));
        assert!(!is_fallback(actual, ordinal));
        let row = &mut expected[ordinal as usize];
        for target in std::iter::once(OwnedOriginTarget::SkillPreset(preset))
            .chain(owners.into_iter().map(OwnedOriginTarget::Issue))
        {
            if !row.links.contains(&target) {
                row.links.push(target);
            }
        }
        row.links.retain(
            |link| !matches!(link, OwnedOriginTarget::Issue(id) if configuration.contains(id)),
        );
    }
    assert_eq!(actual.sidecar().origins, expected);
    no_outputs(actual);
}

#[test]
fn absent_generated_references_and_counts_account_only_the_exact_archived_pair() {
    let f = without_topology();
    let with_topology = configured();
    // A saved numeric count still requires its checked recipe. The exact same
    // source/preset owners exist before the new absence proof can succeed.
    let baseline_text = archived_document(&generated(), "");
    let baseline = run(&f, &baseline_text);
    three_owners(&baseline);
    no_outputs(&baseline);
    for count in [None, Some("nil")] {
        for group_count in [None, Some("nil")] {
            let group = format!(
                "{} includeInFullDPS=\"nil\" mainActiveSkill=\"nil\" mainActiveSkillCalcs=\"1\"",
                group_count.map_or_else(String::new, |value| format!(" groupCount=\"{value}\""))
            );
            let text = archived_document(&absent_gem(count), &group);
            let actual = run(&f, &text);
            assert_exact_accounting(&baseline, &actual, &baseline_text, &text);
            // Adding an unused, valid topology adapter cannot revoke an
            // independent absence proof or turn it into emitted values.
            let via_adapter = run(&with_topology, &text);
            assert_eq!(via_adapter.draft(), actual.draft());
            assert_eq!(via_adapter.allocator_after(), actual.allocator_after());
            assert_eq!(via_adapter.sidecar().origins, actual.sidecar().origins);
            no_outputs(&via_adapter);
            let replay = run(&f, &text);
            assert_eq!(actual.draft(), replay.draft());
            assert_eq!(actual.allocator_after(), replay.allocator_after());
            assert_eq!(
                serde_json::to_value(actual.sidecar()).unwrap(),
                serde_json::to_value(replay.sidecar()).unwrap()
            );
        }
    }
    for (gem, group) in [
        (
            absent_gem(None)
                .replace("enableGlobal1=\"true\"", "")
                .replace("enableGlobal2=\"true\"", ""),
            " includeInFullDPS=\"false\"",
        ),
        (
            absent_gem(Some("nil"))
                .replace("enabled=\"true\"", "enabled=\"false\"")
                .replace("enableGlobal1=\"true\"", "enableGlobal1=\"nil\"")
                .replace("enableGlobal2=\"true\"", "enableGlobal2=\"false\""),
            " includeInFullDPS=\"true\"",
        ),
    ] {
        let text = archived_document(&gem, group);
        let actual = run(&f, &text);
        assert_exact_accounting(&baseline, &actual, &baseline_text, &text);
    }
}

#[test]
fn numeric_or_malformed_counts_require_existing_typed_authority() {
    let f = without_topology();
    let accepted_text = archived_document(&absent_gem(None), "");
    let accepted = run(&f, &accepted_text);
    for ordinal in pair(&accepted_text) {
        assert!(!is_fallback(&accepted, ordinal));
    }
    for token in ["0", "1", "-1", "1.5", "bad", "NaN", "false", " nil"] {
        for group in [false, true] {
            let text = if group {
                archived_document(&absent_gem(None), &format!(" groupCount=\"{token}\""))
            } else {
                archived_document(&absent_gem(Some(token)), "")
            };
            let actual = run(&f, &text);
            assert_eq!(actual.draft(), accepted.draft(), "{token}, group={group}");
            assert_eq!(actual.allocator_after(), accepted.allocator_after());
            assert_eq!(three_owners(&actual), three_owners(&accepted));
            for ordinal in pair(&text) {
                assert!(is_fallback(&actual, ordinal), "{token}, group={group}");
            }
            no_outputs(&actual);
        }
    }
}

#[test]
fn every_present_reference_selector_and_unreviewed_frame_remains_unaccounted() {
    let f = without_topology();
    let accepted_text = archived_document(&absent_gem(None), "");
    let accepted = run(&f, &accepted_text);
    for ordinal in pair(&accepted_text) {
        assert!(!is_fallback(&accepted, ordinal));
    }
    let mut refused = Vec::new();
    // Escaped source values fail the checked source frame before archived
    // responsibility owners are necessarily created; they are never absence.
    refused.push(archived_document(&absent_gem(Some("n&#105;l")), ""));
    refused.push(archived_document(
        &absent_gem(None),
        " groupCount=\"n&#105;l\"",
    ));
    for name in [
        "statSetIndex",
        "statSetIndexCalcs",
        "skillMinion",
        "skillMinionCalcs",
        "skillMinionSkill",
        "skillMinionSkillCalcs",
    ] {
        for value in ["nil", "1"] {
            refused.push(archived_document(
                &absent_gem(None).replace(" quality=", &format!(" {name}=\"{value}\" quality=")),
                "",
            ));
        }
    }
    for (before, after) in [
        ("quality=\"0\"", "quality=\"bad\""),
        ("quality=\"0\"", "quality=\"1001\""),
        ("quality=\"0\"", "quality=\"nil\""),
        ("quality=\"0\"", ""),
        ("enabled=\"true\"", "enabled=\"nil\""),
        ("enableGlobal1=\"true\"", "enableGlobal1=\"maybe\""),
        ("enableGlobal2=\"true\"", "enableGlobal2=\"n&#105;l\""),
        ("corrupted=\"nil\"", "corrupted=\"true\""),
        ("corruptLevel=\"nil\"", "corruptLevel=\"0\""),
        ("quality=\"0\"", "unknown=\"true\" quality=\"0\""),
        ("/>", "><Unexpected/></Gem>"),
    ] {
        refused.push(archived_document(
            &absent_gem(None).replace(before, after),
            "",
        ));
    }
    for group in [
        " includeInFullDPS=\"maybe\"",
        " mainActiveSkill=\"2\"",
        " mainActiveSkillCalcs=\"2\"",
        " label=\"unreviewed\"",
        " unknown=\"true\"",
    ] {
        refused.push(archived_document(&absent_gem(None), group));
    }
    for text in refused {
        let actual = run(&f, &text);
        for ordinal in pair(&text) {
            assert!(is_fallback(&actual, ordinal), "{text}");
        }
        no_outputs(&actual);
    }
}

#[test]
fn archived_absence_never_replaces_selection_identity_or_existing_owners() {
    let f = without_topology();
    let text = archived_document(&absent_gem(Some("nil")), "");
    let accepted = run(&f, &text);
    three_owners(&accepted);
    for ordinal in pair(&text) {
        assert!(!is_fallback(&accepted, ordinal));
    }
    for changed in [
        text.replace("activeSkillSet=\"2\"", "activeSkillSet=\"1\""),
        text.replace("activeSkillSet=\"2\"", "activeSkillSet=\"9\""),
        text.replace("activeSkillSet=\"2\"", ""),
        text.replace("source=\"Tree:7\"", "source=\"Tree:07\""),
        text.replace("level=\"1\"", "level=\"bad\""),
        text.replace(
            "<SkillSet id=\"1\">",
            &format!(
                "<SkillSet id=\"1\"><Skill source=\"Tree:7\" enabled=\"true\">{}</Skill>",
                absent_gem(None)
            ),
        ),
    ] {
        let actual = run(&f, &changed);
        for ordinal in pair(&changed) {
            assert!(is_fallback(&actual, ordinal), "{changed}");
        }
        no_outputs(&actual);
    }
    let mut missing_owner = without_topology();
    missing_owner.base.policy.support_origin_order = None;
    let refused = run(&missing_owner, &text);
    assert!(
        refused.draft().input().skill_presets.members[0]
            .authored_support_order
            .is_none()
    );
    for ordinal in pair(&text) {
        assert!(is_fallback(&refused, ordinal));
    }
    no_outputs(&refused);
    let replay = run(&f, &text);
    assert_eq!(accepted.draft(), replay.draft());
    assert_eq!(accepted.allocator_after(), replay.allocator_after());
    assert_eq!(
        serde_json::to_value(accepted.sidecar()).unwrap(),
        serde_json::to_value(replay.sidecar()).unwrap()
    );
}

#[test]
fn archived_absence_accounting_has_an_exact_work_budget_and_clean_replay() {
    let f = without_topology();
    let text = archived_document(&absent_gem(Some("nil")), " includeInFullDPS=\"nil\"");
    let expected = run(&f, &text);
    for ordinal in pair(&text) {
        assert!(!is_fallback(&expected, ordinal));
    }
    let mut low = 1;
    let mut high = NormalizationLimits::default().max_work;
    while low < high {
        let middle = low + (high - low) / 2;
        match f.base.run(
            &text,
            NormalizationLimits {
                max_work: middle,
                ..Default::default()
            },
        ) {
            Ok(_) => high = middle,
            Err(NormalizationError::Limit(_)) => low = middle + 1,
            Err(other) => panic!("unexpected work-boundary error: {other:?}"),
        }
    }
    let exact = f
        .base
        .run(
            &text,
            NormalizationLimits {
                max_work: low,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(exact.draft(), expected.draft());
    assert_eq!(exact.allocator_after(), expected.allocator_after());
    assert_eq!(
        serde_json::to_value(exact.sidecar()).unwrap(),
        serde_json::to_value(expected.sidecar()).unwrap()
    );
    assert!(matches!(
        f.base.run(
            &text,
            NormalizationLimits {
                max_work: low - 1,
                ..Default::default()
            }
        ),
        Err(NormalizationError::Limit(_))
    ));
    let repeated = run(&f, &text);
    assert_eq!(repeated.draft(), expected.draft());
    assert_eq!(repeated.allocator_after(), expected.allocator_after());
    assert_eq!(
        serde_json::to_value(repeated.sidecar()).unwrap(),
        serde_json::to_value(expected.sidecar()).unwrap()
    );
}
