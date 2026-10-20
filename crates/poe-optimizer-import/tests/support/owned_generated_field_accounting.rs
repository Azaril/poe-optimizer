//! One current generated-input path, synthetic exact Tree and Item identities.
use super::*;

fn fixture() -> Fixture {
    let mut f = generated();
    // Count transport knows these saved fields syntactically; only the separate
    // accounting pass can prove their semantics or retain their fallback.
    for row in new_rows(&mut f.policy) {
        row.attributes.extend(
            [
                "enableGlobal1",
                "enableGlobal2",
                "corruptLevel",
                "statSetIndex",
                "statSetIndexCalcs",
                "skillMinion",
                "skillMinionCalcs",
                "skillMinionSkill",
                "skillMinionSkillCalcs",
                "note",
            ]
            .map(str::to_string),
        );
        row.group_attributes.extend(
            [
                "label",
                "includeInFullDPS",
                "mainActiveSkill",
                "mainActiveSkillCalcs",
                "active",
            ]
            .map(str::to_string),
        );
    }
    f
}

fn inventory_fixture() -> Fixture {
    let mut f = fixture();
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        occurrences,
        inventory,
        ..
    } = f.policy.usage_inputs.as_mut().unwrap();
    let rows = occurrences
        .iter()
        .map(|row| {
            let intrinsic = |field| UsageInventoryField {
                field,
                destinations: vec![UsageInventoryDestination::InputDisposition],
            };
            let mut fields: Vec<_> = [
                "gemId",
                "variantId",
                "skillId",
                "nameSpec",
                "level",
                "quality",
            ]
            .map(|name| {
                intrinsic(UsageInventoryFieldSelector::OccurrenceAttribute { name: name.into() })
            })
            .into();
            fields.push(intrinsic(UsageInventoryFieldSelector::ReferenceDescendants));
            fields.push(intrinsic(UsageInventoryFieldSelector::GroupAttribute {
                name: "source".into(),
            }));
            if let OccurrenceUsageTarget::Generated { correspondence } = &row.target
                && matches!(
                    correspondence.provider,
                    GeneratedSkillInputProvider::ItemModifier { .. }
                )
            {
                fields.push(intrinsic(UsageInventoryFieldSelector::GroupAttribute {
                    name: "slot".into(),
                }));
            }
            let destinations: Vec<_> = row
                .policies
                .iter()
                .flat_map(|policy| {
                    policy.parameters.iter().map(|parameter| {
                        UsageInventoryDestination::UsageParameter {
                            policy: policy.policy.clone(),
                            parameter: parameter.slot.clone(),
                        }
                    })
                })
                .collect();
            for field in [
                UsageInventoryFieldSelector::OccurrenceAttribute {
                    name: "count".into(),
                },
                UsageInventoryFieldSelector::GroupAttribute {
                    name: "groupCount".into(),
                },
            ] {
                fields.push(UsageInventoryField {
                    field,
                    destinations: destinations.clone(),
                });
            }
            UsageInventoryRow {
                source: UsageInventorySource::Occurrence {
                    target: Box::new(row.target.clone()),
                },
                fields,
            }
        })
        .collect();
    *inventory = Some(UsageInventoryPolicy {
        evidence: "b".repeat(64).parse().unwrap(),
        rows,
    });
    f
}

#[test]
fn generated_inventory_uses_actual_provider_and_value_proofs_without_closing_archived_presets() {
    let mut f = inventory_fixture();
    let text = source();
    // This fixture proves exactly one additional retirement. Its allocator
    // watermark remains spent; the shared helper must not assume all IDs live.
    let after =
        normalize_queries_with_retired(&text, &f, &f.policy, &[], Default::default(), 1).unwrap();
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        f.policy.usage_inputs.as_mut().unwrap();
    *inventory = None;
    let before = run(&text, &f);
    let mut expected = before.draft().input().clone();
    let usage = &mut expected.skill_presets.members[1]
        .intent
        .as_mut()
        .unwrap()
        .usage;
    let DraftListCompletion::Pending { id, .. } = usage.completion else {
        panic!("existing selected usage obligation")
    };
    usage.completion = DraftListCompletion::Complete;
    assert_eq!(&expected, after.draft().input());
    let mut origins = before.sidecar().origins.clone();
    for origin in &mut origins {
        origin
            .links
            .retain(|link| *link != OwnedOriginTarget::Issue(id));
    }
    assert_eq!(origins, after.sidecar().origins);
    assert_eq!(before.allocator_after(), after.allocator_after());
    assert!(matches!(
        after.draft().input().skill_presets.members[0]
            .intent
            .as_ref()
            .unwrap()
            .usage
            .completion,
        DraftListCompletion::Pending { .. }
    ));
    assert_eq!(numbers(&after, 1), [Some(1); 4]);
    assert_eq!(bindings(&after, 1).members.len(), 2);
    assert!(after.draft().input().skills.members.is_empty());
    assert!(!configuration(&after).is_empty());
}

#[test]
fn generated_inventory_accounts_literal_primary_true_only_with_live_provider_proof() {
    let mut f = inventory_fixture();
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        f.policy.usage_inputs.as_mut().unwrap();
    for row in &mut inventory.as_mut().unwrap().rows {
        row.fields.push(UsageInventoryField {
            field: UsageInventoryFieldSelector::OccurrenceAttribute {
                name: "enableGlobal1".into(),
            },
            destinations: vec![UsageInventoryDestination::InputDisposition],
        });
    }
    let text = source().replace("<Gem gemId=", r#"<Gem enableGlobal1="true" gemId="#);
    let after =
        normalize_queries_with_retired(&text, &f, &f.policy, &[], Default::default(), 1).unwrap();
    let mut without = f.policy.clone();
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        without.usage_inputs.as_mut().unwrap();
    *inventory = None;
    let before = normalize(&text, &f, &without, Default::default()).unwrap();
    let mut expected = before.draft().input().clone();
    let usage = &mut expected.skill_presets.members[1]
        .intent
        .as_mut()
        .unwrap()
        .usage;
    let DraftListCompletion::Pending { id, .. } = usage.completion else {
        panic!("existing selected usage obligation")
    };
    usage.completion = DraftListCompletion::Complete;
    assert_eq!(&expected, after.draft().input());
    let mut origins = before.sidecar().origins.clone();
    for row in &mut origins {
        row.links
            .retain(|link| *link != OwnedOriginTarget::Issue(id));
    }
    assert_eq!(origins, after.sidecar().origins);
    assert_eq!(before.allocator_after(), after.allocator_after());

    for token in ["false", "nil", "", " true", "true ", "TRUE", "1"] {
        let changed = text.replace(
            r#"enableGlobal1="true""#,
            &format!(r#"enableGlobal1="{token}""#),
        );
        let result = run(&changed, &f);
        assert!(
            matches!(
                result.draft().input().skill_presets.members[1]
                    .intent
                    .as_ref()
                    .unwrap()
                    .usage
                    .completion,
                DraftListCompletion::Pending { .. }
            ),
            "{token}"
        );
    }
    // Numeric entities are rejected by the existing source lexical decoder,
    // before this generated-usage path can attach its own Pending inventory.
    // That earlier refusal must preserve the original bytes and fallback
    // obligations, with no field receipt or retirement from the new gate.
    let escaped = text.replace(r#"enableGlobal1="true""#, r#"enableGlobal1="tr&#117;e""#);
    let imported = crate::source(&escaped, 93);
    assert_eq!(imported.source_xml(), escaped);
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let attributes: Vec<_> = evidence
        .rows()
        .iter()
        .flat_map(|row| row.attributes())
        .filter(|attribute| attribute.origin().name == "enableGlobal1")
        .collect();
    assert!(!attributes.is_empty());
    assert!(
        attributes
            .iter()
            .all(|attribute| attribute.raw() == "tr&#117;e" && attribute.decoded().is_err())
    );
    let unchanged = run(&escaped, &f);
    let baseline = normalize(&escaped, &f, &without, Default::default()).unwrap();
    assert_eq!(unchanged.draft().input(), baseline.draft().input());
    assert_eq!(unchanged.sidecar().origins, baseline.sidecar().origins);
    assert_eq!(unchanged.allocator_after(), baseline.allocator_after());
    assert_retained(&unchanged, &escaped);
    assert!(numbers(&unchanged, 1).is_empty());

    for changed in [
        source(),
        text.replace("Tree:7", "Tree:8"),
        text.replace(r#"quality="12.5""#, r#"quality="bad""#),
    ] {
        let result = run(&changed, &f);
        assert!(matches!(
            result.draft().input().skill_presets.members[1]
                .intent
                .as_ref()
                .unwrap()
                .usage
                .completion,
            DraftListCompletion::Pending { .. }
        ));
    }
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        f.policy.usage_inputs.as_mut().unwrap();
    for row in &mut inventory.as_mut().unwrap().rows {
        row.fields
            .last_mut()
            .unwrap()
            .destinations
            .push(UsageInventoryDestination::Deferred {
                reason: OwnedDefinitionKey::new("unresolved-other-consumer").unwrap(),
            });
    }
    let result = run(&text, &f);
    assert!(matches!(
        result.draft().input().skill_presets.members[1]
            .intent
            .as_ref()
            .unwrap()
            .usage
            .completion,
        DraftListCompletion::Pending { .. }
    ));
}

#[test]
fn generated_inventory_does_not_promote_lexical_switches_or_reference_selection() {
    for field in [
        "enabled",
        "enableGlobal2",
        "mainActiveSkill",
        "mainActiveSkillCalcs",
        "includeInFullDPS",
    ] {
        let mut f = inventory_fixture();
        let group = matches!(
            field,
            "mainActiveSkill" | "mainActiveSkillCalcs" | "includeInFullDPS"
        );
        let value = if field.starts_with("main") {
            "1"
        } else {
            "true"
        };
        let text = source().replace(
            if group {
                "<Skill source="
            } else {
                "<Gem gemId="
            },
            &format!(
                "{} {field}=\"{value}\" {}",
                if group { "<Skill" } else { "<Gem" },
                if group { "source=" } else { "gemId=" }
            ),
        );
        let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
            f.policy.usage_inputs.as_mut().unwrap();
        for row in &mut inventory.as_mut().unwrap().rows {
            row.fields.push(UsageInventoryField {
                field: if group {
                    UsageInventoryFieldSelector::GroupAttribute { name: field.into() }
                } else {
                    UsageInventoryFieldSelector::OccurrenceAttribute { name: field.into() }
                },
                destinations: vec![UsageInventoryDestination::InputDisposition],
            });
        }
        let result = run(&text, &f);
        assert!(
            matches!(
                result.draft().input().skill_presets.members[1]
                    .intent
                    .as_ref()
                    .unwrap()
                    .usage
                    .completion,
                DraftListCompletion::Pending { .. }
            ),
            "{field}"
        );
        assert_eq!(numbers(&result, 1), [Some(1); 4], "{field}");
        assert_eq!(bindings(&result, 1).members.len(), 2, "{field}");
    }
}
fn pairs(text: &str, set_index: usize) -> Vec<u32> {
    let imported = crate::source(text, 93);
    let rows = imported.occurrences();
    let set = rows
        .iter()
        .filter(|r| r.name() == "SkillSet")
        .nth(set_index)
        .unwrap()
        .id();
    let groups: BTreeSet<_> = rows
        .iter()
        .filter(|r| r.parent() == Some(set) && r.name() == "Skill")
        .map(|r| r.id())
        .collect();
    rows.iter()
        .filter(|r| groups.contains(&r.id()) || r.parent().is_some_and(|p| groups.contains(&p)))
        .map(|r| r.id().ordinal())
        .collect()
}
fn configuration(result: &NormalizedImport) -> BTreeSet<DraftIssueId> {
    result
        .draft()
        .input()
        .choice_presets
        .members
        .iter()
        .filter_map(|p| match &p.choices.completion {
            DraftListCompletion::Pending { id, code }
                if code.as_str() == "configuration-roles-not-converted" =>
            {
                Some(*id)
            }
            _ => None,
        })
        .collect()
}
fn fallback(result: &NormalizedImport, ordinal: u32) -> bool {
    let ids = configuration(result);
    result.sidecar().origins[ordinal as usize]
        .links
        .iter()
        .any(|link| matches!(link, OwnedOriginTarget::Issue(id) if ids.contains(id)))
}
fn assert_retained(result: &NormalizedImport, text: &str) {
    for source in pairs(text, 1) {
        assert!(fallback(result, source), "source {source}");
    }
}
#[test]
fn generated_accounting_retires_only_selected_pairs_and_preserves_real_pending_owner() {
    let f = fixture();
    let text = source();
    let result = run(&text, &f);
    assert_eq!(result.sidecar().schema_version, 22);
    let preset = &result.draft().input().skill_presets.members[1];
    let DraftListCompletion::Pending { id, .. } = preset.intent.as_ref().unwrap().usage.completion
    else {
        panic!()
    };
    assert_eq!(pairs(&text, 1).len(), 4);
    for ordinal in pairs(&text, 1) {
        assert!(!fallback(&result, ordinal), "selected source {ordinal}");
        assert!(
            result.sidecar().origins[ordinal as usize]
                .links
                .contains(&OwnedOriginTarget::Issue(id))
        );
    }
    for ordinal in pairs(&text, 0) {
        assert!(fallback(&result, ordinal), "dormant source {ordinal}");
    }
    assert_eq!(numbers(&result, 1), [Some(1); 4]);
    assert_eq!(bindings(&result, 1).members.len(), 2);
    assert!(
        !configuration(&result).is_empty(),
        "shared configuration issue remains live"
    );
    assert!(
        result.draft().input().skills.members.is_empty(),
        "no generated root becomes Direct"
    );
    assert_eq!(
        serde_json::to_value(result.sidecar()).unwrap(),
        serde_json::to_value(run(&text, &f).sidecar()).unwrap(),
        "A/B/A is exact"
    );
}
#[test]
fn generated_accounting_unknown_semantics_preserve_successful_raw_and_count_transport() {
    let f = fixture();
    let original = run(&source(), &f);
    for (old, new) in [
        (" quality=", " enableGlobal1=\"bad\" quality="),
        (" quality=", " corrupted=\"true\" quality="),
        (" quality=", " corruptLevel=\"3\" quality="),
        (" quality=", " skillMinion=\"unknown-actor\" quality="),
        (" quality=", " statSetIndex=\"1\" quality="),
        (" quality=", " note=\"unreviewed\" quality="),
        ("<Skill source=", "<Skill label=\"nonempty\" source="),
        ("<Skill source=", "<Skill mainActiveSkill=\"2\" source="),
        ("<Skill source=", "<Skill active=\"true\" source="),
    ] {
        let text = source().replace(old, new);
        let result = run(&text, &f);
        assert_retained(&result, &text);
        assert_eq!(
            bindings(&result, 1),
            bindings(&original, 1),
            "raw values survive {new}"
        );
        assert_eq!(numbers(&result, 1), [Some(1); 4], "counts survive {new}");
        assert_eq!(
            result.draft().input().allocator,
            original.draft().input().allocator
        );
    }
}
#[test]
fn generated_accounting_requires_actual_known_raw_and_count_receipts() {
    let f = fixture();
    for text in [
        source()
            .replace("quality=\"12.5\"", "quality=\"bad\"")
            .replace("quality=\"20.25\"", "quality=\"bad\""),
        source().replace("count=\"nil\"", "count=\"bad\""),
        source().replace("count=\"nil\"", ""),
        source().replace("<Skill source=", "<Skill groupCount=\"bad\" source="),
        // A valid override does not account for malformed shadowed source data.
        source()
            .replace("count=\"nil\"", "count=\"bad\"")
            .replace("<Skill source=", "<Skill groupCount=\"0\" source="),
    ] {
        let result = run(&text, &f);
        assert_retained(&result, &text);
    }
    let zero = source().replace("<Skill source=", "<Skill groupCount=\"0\" source=");
    let result = run(&zero, &f);
    assert_eq!(numbers(&result, 1), [Some(0); 4]);
    for ordinal in pairs(&zero, 1) {
        assert!(!fallback(&result, ordinal));
    }
    let mut without = fixture();
    without.policy.usage_inputs = None;
    let result = run(&source(), &without);
    assert_retained(&result, &source());
    assert_eq!(bindings(&result, 1).members.len(), 2);
    assert!(matches!(
        result.draft().input().skill_presets.members[1]
            .intent
            .as_ref()
            .unwrap()
            .usage
            .completion,
        DraftListCompletion::Complete
    ));
}
#[test]
fn generated_accounting_preserves_absence_empty_nil_and_unproved_structure() {
    let f = fixture();
    for text in [
        source()
            .replace("source=\"Tree:7\"", "source=\"\"")
            .replace(
                "source=\"Item:1:Test Title, Ordinary Base\"",
                "source=\"nil\"",
            ),
        source()
            .replace("source=\"Tree:7\"", "")
            .replace("source=\"Item:1:Test Title, Ordinary Base\"", ""),
    ] {
        let result = run(&text, &f);
        let mut without = f.policy.clone();
        without.generated_skill_inputs = None;
        let baseline = normalize(&text, &f, &without, Default::default()).unwrap();
        // This fixture independently admits absent/empty manual source as
        // Physical. Such a Gem already has physical provenance, not a generic
        // configuration link. Generated accounting must preserve that exact
        // provenance rather than manufacture or remove a fallback obligation.
        for ordinal in pairs(&text, 1) {
            assert_eq!(
                result.sidecar().origins[ordinal as usize],
                baseline.sidecar().origins[ordinal as usize],
                "source {ordinal}"
            );
        }
        assert!(bindings(&result, 1).members.is_empty());
        assert!(numbers(&result, 1).is_empty());
    }
    for text in [
        source().replace(" quality=", " unknown=\"true\" quality="),
        source()
            .replace("quality=\"12.5\"/>", "quality=\"12.5\"><Unreviewed/></Gem>")
            .replace(
                "quality=\"20.25\"/>",
                "quality=\"20.25\"><Unreviewed/></Gem>",
            ),
    ] {
        let result = run(&text, &f);
        assert_retained(&result, &text);
    }
    let duplicate = source().replace(" quality=", " quality=\"0\" quality=");
    assert!(
        decode_build(duplicate.as_bytes()).is_err(),
        "duplicate XML attributes are rejected before normalization"
    );
    let mut mismatched = fixture();
    let Some(GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 { source, .. }) =
        &mut mismatched.policy.generated_skill_inputs
    else {
        panic!()
    };
    source.revision = "d".repeat(40);
    assert!(
        normalize(
            &self::source(),
            &mismatched,
            &mismatched.policy,
            Default::default()
        )
        .is_err()
    );
}
