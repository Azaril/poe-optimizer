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
