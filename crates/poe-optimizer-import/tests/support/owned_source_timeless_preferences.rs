//! Default search-tool preferences retain source facts without closing owners.
use super::*;

const CODE: &str = "source-timeless-search-defaults";
const DEFAULTS: &str = r#"<TimelessData devotionVariant1="1" devotionVariant2="1" searchList="" searchListFallback=""/>"#;
const HEADER: &str = r#"<Build targetVersion="0_1" viewMode="IMPORT" level="73" className="Caller class" ascendClassName="Caller ascendancy" mainSocketGroup="1" characterLevelAutoMode="false">"#;

fn document(timeless: &str) -> String {
    format!(
        r#"<PathOfBuilding2>{HEADER}<PlayerStat stat="retained-cache" value="99"/>{timeless}<Buffs buffList="retained-display"/></Build><Tree activeSpec="1"><Spec title="same" treeVersion="0_5" classId="1" classInternalId="1" ascendClassId="0" ascendancyInternalId="" nodes="1"/><Spec title="same" treeVersion="0_5" classId="2" classInternalId="2" ascendClassId="0" ascendancyInternalId="" nodes="2"/></Tree><Skills activeSkillSet="9"><SkillSet id="9" title="same"><Skill enabled="true"><Gem gemId="active" variantId="v" level="17" enabled="true"/></Skill></SkillSet><SkillSet id="2" title="same"><Skill enabled="true"><Gem gemId="active" variantId="v" level="1" enabled="true"/></Skill></SkillSet></Skills><Config activeConfigSet="1"><ConfigSet id="1"><Input name="unrelated-role" number="7"/></ConfigSet></Config><Calcs><Input name="skill_number" number="1"/><Input name="misc_buffMode" string="EFFECTIVE"/></Calcs></PathOfBuilding2>"#
    )
}

fn timeless_policy(a: &Artifacts, enabled: bool) -> NormalizationPolicy {
    let mut p = url_policy(a);
    let SourcePresentationPolicy::PobFreshPresentationV1 {
        empty_socket_urls,
        timeless_search_defaults,
        ..
    } = p.source_presentation.as_mut().unwrap();
    *empty_socket_urls = false;
    *timeless_search_defaults = enabled;
    p.character_reward_inventory = Some(
        CharacterRewardInventoryPolicy::PobFreshCharacterOnlyEmptyV1 {
            mapping_source: *a.mapping.source_identity(),
            target_version: "0_1".into(),
            tree_version: "0_5".into(),
        },
    );
    p
}

fn assert_delta(
    xml: &str,
    before: &NormalizedImport,
    after: &NormalizedImport,
    expected_changes: usize,
) {
    assert_eq!(before.draft().input(), after.draft().input());
    assert_eq!(before.allocator_after(), after.allocator_after());
    assert_eq!(
        before.draft().input().query_presets,
        after.draft().input().query_presets
    );
    let imported = source(xml, 117);
    let mut expected = before.sidecar().origins.clone();
    let mut changed = 0;
    for (old, new) in expected.iter_mut().zip(&after.sidecar().origins) {
        if old == new {
            continue;
        }
        changed += 1;
        assert_eq!(old.source, new.source);
        let occurrence = imported.occurrence(new.source).unwrap();
        assert_eq!(occurrence.name(), "TimelessData");
        assert_eq!(
            imported
                .occurrence(occurrence.parent().unwrap())
                .unwrap()
                .name(),
            "Build"
        );
        assert_eq!(old.disposition, SourceDisposition::Contributes);
        assert!(!old.links.is_empty());
        assert!(
            old.links
                .iter()
                .all(|l| matches!(l, OwnedOriginTarget::Issue(_)))
        );
        assert_eq!(new.disposition, SourceDisposition::SourceOnly(key(CODE)));
        assert!(new.links.is_empty());
        old.links.clear();
        old.disposition = new.disposition.clone();
    }
    assert_eq!(changed, expected_changes);
    assert_eq!(expected, after.sidecar().origins);
    // The source and every native/result commitment remain identical. Only the
    // opted-in policy commitment and the exact proved origin may differ.
    let before_wire = serde_json::to_value(before.sidecar()).unwrap();
    let mut after_wire = serde_json::to_value(after.sidecar()).unwrap();
    after_wire["policy"] = before_wire["policy"].clone();
    after_wire["origins"] = before_wire["origins"].clone();
    assert_eq!(before_wire, after_wire);
}

fn assert_refused(xml: &str, a: &Artifacts, context: &str) {
    let before = normalize(xml, a, &timeless_policy(a, false), Default::default())
        .unwrap_or_else(|error| panic!("{context}: baseline {error:?}"));
    let after = normalize(xml, a, &timeless_policy(a, true), Default::default())
        .unwrap_or_else(|error| panic!("{context}: enabled {error:?}"));
    assert_eq!(presentation_count(&after, CODE), 0, "{context}");
    assert_delta(xml, &before, &after, 0);
}

#[test]
fn exact_defaults_change_only_the_source_child_and_preserve_all_owned_meaning() {
    let a = artifacts(true);
    for row in [
        DEFAULTS,
        r#"<TimelessData searchListFallback="" devotionVariant2="1" searchList="" devotionVariant1="1"/>"#,
        "<TimelessData devotionVariant1='1' devotionVariant2='1' searchList='' searchListFallback=''> \n\t </TimelessData>",
    ] {
        let xml = document(row);
        let before = normalize(&xml, &a, &timeless_policy(&a, false), Default::default()).unwrap();
        let after = normalize(&xml, &a, &timeless_policy(&a, true), Default::default()).unwrap();
        assert_eq!(presentation_count(&after, CODE), 1, "{row}");
        assert_delta(&xml, &before, &after, 1);
        assert_eq!(after.draft().input().character_presets.members.len(), 2);
        assert_eq!(after.draft().input().skill_presets.members.len(), 2);
        assert!(!after.draft().input().query_presets.members.is_empty());
        assert!(after.sidecar().origins.iter().any(|origin| {
            origin
                .links
                .iter()
                .any(|link| matches!(link, OwnedOriginTarget::CharacterPreset(_)))
        }));
        assert!(after.sidecar().origins.iter().any(|origin| {
            origin
                .links
                .iter()
                .any(|link| matches!(link, OwnedOriginTarget::SkillPreset(_)))
        }));
        assert!(matches!(
            &after.draft().input().saved_variants.completion,
            DraftListCompletion::Pending { code, .. }
                if code.as_str() == "saved-selection-not-converted"
        ));
        assert!(matches!(
            &after.draft().input().choice_presets.members[0].choices.completion,
            DraftListCompletion::Pending { code, .. }
                if code.as_str() == "configuration-roles-not-converted"
        ));
        for character in &after.draft().input().character_presets.members {
            assert!(matches!(
                &character.rewards.completion,
                DraftListCompletion::Pending { code, .. }
                    if code.as_str() == "character-rewards-not-converted"
            ));
        }
        assert_eq!(presentation_count(&after, "source-cached-build-buffs"), 0);
    }
}

#[test]
fn opt_in_and_checked_target_frame_are_independent_required_authorities() {
    let a = artifacts(true);
    let xml = document(DEFAULTS);
    let disabled = timeless_policy(&a, false);
    let before = normalize(&xml, &a, &disabled, Default::default()).unwrap();
    let wire = serde_json::to_value(disabled.source_presentation.as_ref().unwrap()).unwrap();
    assert!(wire.get("timeless_search_defaults").is_none());
    let restored: SourcePresentationPolicy = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(&restored).unwrap(), wire);
    let mut omitted = disabled.clone();
    omitted.source_presentation = Some(restored);
    let omitted_result = normalize(&xml, &a, &omitted, Default::default()).unwrap();
    assert_eq!(
        serde_json::to_value(before.sidecar()).unwrap(),
        serde_json::to_value(omitted_result.sidecar()).unwrap()
    );
    assert_delta(&xml, &before, &omitted_result, 0);

    for with_frame in [false, true] {
        let mut missing = timeless_policy(&a, true);
        if with_frame {
            let CharacterRewardInventoryPolicy::PobFreshCharacterOnlyEmptyV1 {
                target_version, ..
            } = missing.character_reward_inventory.as_mut().unwrap();
            *target_version = "other".into();
        } else {
            missing.character_reward_inventory = None;
        }
        let mut baseline = missing.clone();
        let SourcePresentationPolicy::PobFreshPresentationV1 {
            timeless_search_defaults,
            ..
        } = baseline.source_presentation.as_mut().unwrap();
        *timeless_search_defaults = false;
        let old = normalize(&xml, &a, &baseline, Default::default()).unwrap();
        let actual = normalize(&xml, &a, &missing, Default::default()).unwrap();
        assert_eq!(presentation_count(&actual, CODE), 0);
        assert_delta(&xml, &old, &actual, 0);
    }
    // Source binding remains mandatory even when the optional family is off.
    for enabled in [false, true] {
        let mut wrong_source = timeless_policy(&a, enabled);
        let SourcePresentationPolicy::PobFreshPresentationV1 { mapping_source, .. } =
            wrong_source.source_presentation.as_mut().unwrap();
        *mapping_source = "f".repeat(64).parse().unwrap();
        assert!(matches!(
            normalize(&xml, &a, &wrong_source, Default::default()),
            Err(NormalizationError::Binding)
        ));
        let mut wrong_frame_source = timeless_policy(&a, enabled);
        let CharacterRewardInventoryPolicy::PobFreshCharacterOnlyEmptyV1 { mapping_source, .. } =
            wrong_frame_source
                .character_reward_inventory
                .as_mut()
                .unwrap();
        *mapping_source = "f".repeat(64).parse().unwrap();
        assert!(matches!(
            normalize(&xml, &a, &wrong_frame_source, Default::default()),
            Err(NormalizationError::Binding)
        ));
    }
}

#[test]
fn nondefault_missing_or_nonliteral_child_fields_retain_their_exact_responsibilities() {
    let a = artifacts(true);
    let valid = normalize(
        &document(DEFAULTS),
        &a,
        &timeless_policy(&a, true),
        Default::default(),
    )
    .unwrap();
    assert_eq!(presentation_count(&valid, CODE), 1);
    for (field, value, invalid) in [
        ("devotionVariant1", "1", &['2', '0'][..]),
        ("devotionVariant2", "1", &['2', '0'][..]),
    ] {
        let original = format!(r#"{field}="{value}""#);
        assert_refused(
            &document(&DEFAULTS.replace(&original, "")),
            &a,
            &format!("missing {field}"),
        );
        for changed in invalid {
            let row = DEFAULTS.replace(&original, &format!(r#"{field}="{changed}""#));
            assert_refused(
                &document(&row),
                &a,
                &format!("nondefault {field}={changed}"),
            );
        }
        for changed in ["01", "1.0", "+1", "&#49;", " 1", ""] {
            let row = DEFAULTS.replace(&original, &format!(r#"{field}="{changed}""#));
            assert_refused(
                &document(&row),
                &a,
                &format!("nonliteral {field}={changed}"),
            );
        }
    }
    for field in ["searchList", "searchListFallback"] {
        let original = format!(r#"{field}="""#);
        assert_refused(
            &document(&DEFAULTS.replace(&original, "")),
            &a,
            &format!("missing {field}"),
        );
        for changed in [" ", "wanted-node", "&#32;", "&amp;"] {
            let row = DEFAULTS.replace(&original, &format!(r#"{field}="{changed}""#));
            assert_refused(&document(&row), &a, &format!("nonempty {field}={changed}"));
        }
    }
    for attrs in [
        r#"socketFilter="false""#,
        r#"socketFilterDistance="0""#,
        r#"jewelTypeId="1""#,
        r#"conquerorId="1""#,
        r#"socketId="1""#,
        r#"future="""#,
        r#"xmlns="urn:foreign""#,
        r#"xmlns:x="urn:foreign""#,
        r#"xmlns:x="urn:foreign" x:future="""#,
    ] {
        assert_refused(
            &document(&DEFAULTS.replace("/>", &format!(" {attrs}/>"))),
            &a,
            attrs,
        );
    }
    for content in [
        "text",
        "<!-- preserved -->",
        "<![CDATA[]]>",
        "<Nested/>",
        "&#32;",
    ] {
        let row = DEFAULTS.replace("/>", &format!(">{content}</TimelessData>"));
        assert_refused(&document(&row), &a, content);
    }
    let duplicate = DEFAULTS.replace("/>", r#" devotionVariant1="1"/>"#);
    assert!(decode_build(document(&duplicate).as_bytes()).is_err());
}

#[test]
fn duplicate_foreign_or_nonfresh_build_frames_cannot_lend_child_authority() {
    let a = artifacts(true);
    let base = document(DEFAULTS);
    assert_eq!(
        presentation_count(
            &normalize(&base, &a, &timeless_policy(&a, true), Default::default()).unwrap(),
            CODE
        ),
        1
    );
    for (label, xml) in [
        ("absent child", document("")),
        (
            "duplicate child",
            document(&format!("{DEFAULTS}{DEFAULTS}")),
        ),
        (
            "wrapped child",
            document(&format!("<Wrapper>{DEFAULTS}</Wrapper>")),
        ),
        (
            "root child",
            document("").replace(
                "</PathOfBuilding2>",
                &format!("{DEFAULTS}</PathOfBuilding2>"),
            ),
        ),
        (
            "duplicate Build",
            base.replace("</Build>", &format!("</Build>{HEADER}{DEFAULTS}</Build>")),
        ),
        (
            "foreign Build",
            base.replace("<Build ", "<Build xmlns='urn:foreign' "),
        ),
        (
            "unknown root",
            base.replace("<PathOfBuilding2>", "<PathOfBuilding2 future='true'>"),
        ),
        (
            "unknown Build",
            base.replace("<Build ", "<Build future='true' "),
        ),
        (
            "unknown sibling",
            base.replace("</Build>", "<Future/></Build>"),
        ),
        (
            "foreign sibling",
            base.replace("<Buffs ", "<Buffs xmlns='urn:foreign' "),
        ),
        ("Build content", base.replace("</Build>", "text</Build>")),
        (
            "different target",
            base.replace("targetVersion=\"0_1\"", "targetVersion=\"other\""),
        ),
        (
            "escaped target",
            base.replace("targetVersion=\"0_1\"", "targetVersion=\"0&#95;1\""),
        ),
        (
            "automatic level",
            base.replace(
                "characterLevelAutoMode=\"false\"",
                "characterLevelAutoMode=\"true\"",
            ),
        ),
        (
            "missing header input",
            base.replace("mainSocketGroup=\"1\"", ""),
        ),
        (
            "legacy MAIN",
            base.replace("mainSocketGroup=\"1\"", "mainSkillIndex=\"1\""),
        ),
    ] {
        assert_refused(&xml, &a, label);
    }
    // A foreign root is rejected before evidence/normalization exists; test
    // that actual boundary instead of asking the valid-source helper to unwrap it.
    let foreign_root = base.replace("<PathOfBuilding2>", "<PathOfBuilding2 xmlns='urn:foreign'>");
    assert!(matches!(
        decode_build(foreign_root.as_bytes()),
        Err(poe_optimizer_import::ImportError::WrongRoot { actual })
            if actual == "{urn:foreign}PathOfBuilding2"
    ));
}

fn minimum_work(xml: &str, a: &Artifacts, p: &NormalizationPolicy) -> usize {
    let mut low = 1;
    let mut high = NormalizationLimits::default().max_work;
    while low < high {
        let middle = low + (high - low) / 2;
        match normalize(
            xml,
            a,
            p,
            NormalizationLimits {
                max_work: middle,
                ..Default::default()
            },
        ) {
            Ok(_) => high = middle,
            Err(NormalizationError::Limit(_)) => low = middle + 1,
            Err(error) => panic!("unexpected work boundary error: {error:?}"),
        }
    }
    low
}

#[test]
fn exact_work_limits_and_alternating_inputs_preserve_clean_deterministic_replay() {
    let a = artifacts(true);
    let xml = document(DEFAULTS);
    let p = timeless_policy(&a, true);
    let policy_before = serde_json::to_value(&p).unwrap();
    let expected = normalize(&xml, &a, &p, Default::default()).unwrap();
    assert_eq!(presentation_count(&expected, CODE), 1);
    let minimum = minimum_work(&xml, &a, &p);
    let exact = normalize(
        &xml,
        &a,
        &p,
        NormalizationLimits {
            max_work: minimum,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(expected.draft(), exact.draft());
    assert_eq!(expected.allocator_after(), exact.allocator_after());
    assert_eq!(
        serde_json::to_value(expected.sidecar()).unwrap(),
        serde_json::to_value(exact.sidecar()).unwrap()
    );
    assert!(matches!(
        normalize(
            &xml,
            &a,
            &p,
            NormalizationLimits {
                max_work: minimum - 1,
                ..Default::default()
            }
        ),
        Err(NormalizationError::Limit(_))
    ));
    let disabled = timeless_policy(&a, false);
    let mut omitted = disabled.clone();
    let mut wire = serde_json::to_value(&omitted).unwrap();
    wire["source_presentation"]
        .as_object_mut()
        .unwrap()
        .remove("timeless_search_defaults");
    omitted = serde_json::from_value(wire).unwrap();
    assert_eq!(
        minimum_work(&xml, &a, &disabled),
        minimum_work(&xml, &a, &omitted)
    );
    let alternate = document(&DEFAULTS.replace("devotionVariant2=\"1\"", "devotionVariant2=\"2\""));
    assert_eq!(
        presentation_count(
            &normalize(&alternate, &a, &p, Default::default()).unwrap(),
            CODE
        ),
        0
    );
    let replay = normalize(&xml, &a, &p, Default::default()).unwrap();
    assert_eq!(replay.draft(), expected.draft());
    assert_eq!(replay.allocator_after(), expected.allocator_after());
    assert_eq!(
        serde_json::to_value(replay.sidecar()).unwrap(),
        serde_json::to_value(expected.sidecar()).unwrap()
    );
    assert_eq!(serde_json::to_value(&p).unwrap(), policy_before);
}
