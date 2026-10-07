//! Exact overwritten-source accounting is separate from scalar projection.
use super::*;

fn placeholder<'a>(
    imported: &ImportedBuildInstance,
    result: &'a NormalizedImport,
    name: &str,
) -> &'a SourceOwnedOrigin {
    let source = imported
        .occurrences()
        .iter()
        .find(|row| {
            row.name() == "Placeholder"
                && imported
                    .attribute(row.id(), "name")
                    .unwrap()
                    .unwrap()
                    .decoded()
                    == name
        })
        .unwrap()
        .id();
    &result.sidecar().origins[source.ordinal() as usize]
}

#[test]
fn overwritten_raw_placeholders_keep_only_exact_scenario_correspondence() {
    let f = fixture();
    for body in [
        r#"<Placeholder name="raw-0" number="0"/>"#,
        r#"<Placeholder name="raw-0" number="-1234567.5"/>"#,
        r#"<Placeholder name="raw-0" number="1e6"/><Input name="raw-0" number="0"/>"#,
        r#"<Input name="raw-0" number="-7.5"/><Placeholder name="raw-0" number="99"/>"#,
        r#"<Placeholder name="raw-0" number="99"/><Input name="raw-1" number="bad"/>"#,
    ] {
        let imported = source(&xml(body), 0x81);
        let policy = reviewed(&f);
        let result = normalize(&imported, &f, &policy, Default::default()).unwrap();
        let repeated = normalize(&imported, &f, &policy, Default::default()).unwrap();
        assert_eq!(result.draft(), repeated.draft());
        assert_eq!(result.sidecar().origins, repeated.sidecar().origins);
        let origin = placeholder(&imported, &result, "raw-0");
        assert_eq!(
            origin.disposition,
            SourceDisposition::SourceOnly(key("overwritten-config-placeholder"))
        );
        assert_eq!(
            origin.links,
            vec![OwnedOriginTarget::ScenarioPreset(
                result.draft().input().scenario_presets.members[0].id
            )]
        );
        assert!(matches!(
            result.draft().input().choice_presets.members[0]
                .choices
                .completion,
            DraftListCompletion::Pending { .. }
        ));
        let scenario = &result.draft().input().scenario_presets.members[0].scenario;
        assert!(matches!(
            scenario.assumptions.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(matches!(
            scenario.usage.completion,
            DraftListCompletion::Pending { .. }
        ));
        origin_integrity_with_retired(&imported, &result, 1);
    }
}

#[test]
fn unknown_fallback_malformed_and_wrong_encounter_placeholders_keep_responsibility() {
    let f = fixture();
    for (body, name) in [
        (r#"<Placeholder name="unknown" number="3"/>"#, "unknown"),
        (r#"<Placeholder name="raw-1" number="3"/>"#, "raw-1"),
        (r#"<Placeholder name="raw-0" string="3"/>"#, "raw-0"),
        (r#"<Placeholder name="raw-0" number="NaN"/>"#, "raw-0"),
        (r#"<Placeholder name="raw-0" number="inf"/>"#, "raw-0"),
        (r#"<Placeholder name="raw-0" number=" 3"/>"#, "raw-0"),
        (
            r#"<Placeholder name="raw-0" number="3" future="x"/>"#,
            "raw-0",
        ),
        (
            r#"<Placeholder name="raw-0" number="3"><Future/></Placeholder>"#,
            "raw-0",
        ),
        (
            r#"<Placeholder name="raw-0" number="3">text</Placeholder>"#,
            "raw-0",
        ),
        (
            r#"<Placeholder xmlns:q="future" name="raw-0" number="3"/>"#,
            "raw-0",
        ),
        (
            r#"<Placeholder name="raw-0" number="3"/><Placeholder name="raw-0" number="4"/>"#,
            "raw-0",
        ),
        (
            r#"<Placeholder name="raw-0" number="3"/><Input name="raw-0" number="101"/>"#,
            "raw-0",
        ),
        (
            r#"<Placeholder name="raw-0" number="3"/><Input name="encounter-selector" string="other"/>"#,
            "raw-0",
        ),
    ] {
        let imported = source(&xml(body), 0x82);
        let result = normalize(
            &imported,
            &f,
            &fallback_tests::fallback_policy(&f),
            Default::default(),
        )
        .unwrap();
        let origin = placeholder(&imported, &result, name);
        assert_eq!(origin.disposition, SourceDisposition::Contributes, "{body}");
        assert!(
            origin
                .links
                .iter()
                .any(|link| matches!(link, OwnedOriginTarget::Issue(_))),
            "{body}"
        );
    }
}

#[test]
fn foreign_configuration_obligations_and_malformed_siblings_cannot_be_borrowed() {
    let f = fixture();
    for sibling in [
        r#"<ConfigSet id="2"/>"#,
        r#"<ConfigSet id="2"><Future/></ConfigSet>"#,
    ] {
        let text = format!(
            r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1"><Placeholder name="raw-0" number="3"/></ConfigSet>{sibling}</Config></PathOfBuilding2>"#
        );
        let imported = source(&text, 0x83);
        let result = normalize(&imported, &f, &reviewed(&f), Default::default()).unwrap();
        // Without a completed reward census these bare test frames retain the
        // historical shared fallback. Local output cannot remove foreign links.
        let origin = placeholder(&imported, &result, "raw-0");
        assert_eq!(origin.disposition, SourceDisposition::Contributes);
        assert_eq!(
            origin
                .links
                .iter()
                .filter(|link| matches!(link, OwnedOriginTarget::Issue(_)))
                .count(),
            2
        );
    }
}

#[test]
fn absent_policy_retains_placeholder_and_accounting_limits_are_deterministic() {
    let f = fixture();
    let imported = source(&xml(r#"<Placeholder name="raw-0" number="3"/>"#), 0x84);
    let policy = reviewed(&f);
    let mut omitted = policy.clone();
    omitted.configuration_inputs = None;
    let old = normalize(&imported, &f, &omitted, Default::default()).unwrap();
    assert_eq!(
        placeholder(&imported, &old, "raw-0").disposition,
        SourceDisposition::Contributes
    );
    let expected = normalize(&imported, &f, &policy, Default::default()).unwrap();
    let limits = NormalizationLimits {
        max_work: 1,
        ..Default::default()
    };
    assert!(normalize(&imported, &f, &policy, limits).is_err());
    let recovered = normalize(&imported, &f, &policy, Default::default()).unwrap();
    assert_eq!(expected.draft(), recovered.draft());
    assert_eq!(expected.sidecar().origins, recovered.sidecar().origins);
}
