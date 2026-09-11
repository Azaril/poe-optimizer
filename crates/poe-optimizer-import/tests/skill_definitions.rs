#[path = "../../../tests/support/skill_preparation_edits.rs"]
mod preparation_edits;
use poe_optimizer_data::{
    game_data::{
        GameDataLoader, GameDataPackage, GameDataSnapshot, LoadLimits, TrustPolicy,
        bundled_snapshot,
    },
    skill_identities::{
        GemIdentity, GemIdentityResolutionReason as Reason, GemIdentityResolutionStatus as Status,
    },
};
use poe_optimizer_import::{
    skill_definitions::{
        InstanceIdentityLookup, InstanceIdentityResolution as Resolution, SkillIdentityRecord,
        lookup_definitions,
    },
    skill_source::{self, SkillSourceKind, SkillSourceUse},
};
use std::collections::BTreeSet;

fn snapshot(mut package: GameDataPackage) -> GameDataSnapshot {
    package.skill_identities.missing_references =
        package.skill_identities.expected_missing_references();
    preparation_edits::refresh_lookups(&mut package);
    package.refresh_section_digests().unwrap();
    GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap()
}
fn edit_gem(package: &mut GameDataPackage, index: usize, edit: impl FnOnce(&mut GemIdentity)) {
    let old = package.skill_identities.gems[index].key.clone();
    edit(&mut package.skill_identities.gems[index]);
    let gem = package.skill_identities.gems[index].clone();
    preparation_edits::rename_gem(package, &old, &gem.key);
    // Retain duplicate source declaration occurrences when changing a test key.
    for declaration in &mut package.skill_identities.gem_declarations {
        if declaration.key == old {
            declaration.key = gem.key.clone();
        }
    }
    let declaration =
        &mut package.skill_identities.gem_declarations[gem.winning_declaration as usize - 1];
    declaration.identity.game_id = gem.game_id;
    declaration.identity.variant_id = gem.variant_id;
    declaration.identity.name = gem.name;
    declaration.identity.name_spec = gem.name_spec;
    declaration.identity.base_type_name = gem.base_type_name;
    declaration.identity.primary_effect_id = gem.primary_effect_id;
    declaration.identity.additional_effects = gem.declared_additional_effects;
    declaration.identity.additional_stat_sets = gem.declared_additional_stat_sets;
    declaration.identity.display_order = gem.display_order;
}
fn attr(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn wrap(gems: &str) -> String {
    format!("<PathOfBuilding2><Skills><Skill>{gems}</Skill></Skills></PathOfBuilding2>")
}
fn instance<'a, 'input>(
    record: &'a SkillIdentityRecord<'input>,
) -> &'a InstanceIdentityLookup<'input> {
    match record {
        SkillIdentityRecord::Instance(i) => i,
        _ => panic!("expected gem instance"),
    }
}
fn external<'a>(record: &'a SkillIdentityRecord<'_>) -> (Status, Reason, Vec<&'a str>) {
    match &instance(record).resolution {
        Resolution::ExternalGem {
            status,
            reason,
            candidates,
        } => (
            *status,
            *reason,
            candidates.iter().map(|c| c.key.as_str()).collect(),
        ),
        _ => panic!("expected external-gem identity evidence"),
    }
}

#[test]
fn injected_external_identifiers_drive_exact_and_single_fallback_evidence() {
    let base = bundled_snapshot().unwrap();
    let mut package = base.package().clone();
    let original = package.skill_identities.gems[0].clone();
    let game = "Caller/External & Identifier";
    let variant = "CallerVariant";
    let key = "Caller/InternalKey";
    assert!(
        package
            .skill_identities
            .gems
            .iter()
            .all(|g| g.game_id != game && g.key != key)
    );
    edit_gem(&mut package, 0, |g| {
        g.game_id = game.into();
        g.variant_id = variant.into();
        g.key = key.into();
        g.name = "Caller display name".into();
    });
    let selected = snapshot(package);
    let package_before = selected.package().canonical_bytes().unwrap();
    let xml = wrap(&format!(
        "<Gem gemId='{}' variantId='{variant}'/><Gem gemId='{}'/><Gem gemId='{}' variantId='AbsentVariant'/><Gem gemId='{}' variantId='{}'/>",
        attr(game),
        attr(game),
        attr(game),
        attr(&original.game_id),
        attr(&original.variant_id)
    ));
    let source = skill_source::project_xml(&xml).unwrap();
    let result = lookup_definitions(&source, &selected).unwrap();
    assert_eq!(
        external(&result.records()[0].record),
        (Status::Exact, Reason::ExactExternalVariant, vec![key])
    );
    assert_eq!(
        external(&result.records()[1].record),
        (
            Status::SingleFallback,
            Reason::MissingVariantSingleCandidate,
            vec![key]
        )
    );
    assert_eq!(
        external(&result.records()[2].record),
        (
            Status::SingleFallback,
            Reason::UnknownVariantSingleCandidate,
            vec![key]
        )
    );
    assert!(
        external(&result.records()[3].record)
            .2
            .iter()
            .all(|k| *k != key)
    );
    assert_eq!(result.source_xml_sha256(), source.source_sha256());
    assert_eq!(result.data(), selected.identity());
    assert_ne!(result.data(), base.identity());
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["data_trust"]["status"], "custom_unreviewed");
    assert_eq!(json["game_mechanics"], "not_evaluated");
    assert_eq!(json["socket_group_processing"], "not_run");
    assert_eq!(json["name_matching"], "not_run");
    assert_eq!(json["actor_resolution"], "not_resolved");
    assert_eq!(json["active_set_selection"], "not_resolved");
    assert_eq!(
        instance(&result.records()[0].record)
            .game_id
            .as_ref()
            .unwrap()
            .raw(),
        attr(game)
    );
    assert_eq!(
        instance(&result.records()[0].record)
            .game_id
            .as_ref()
            .unwrap()
            .decoded(),
        game
    );
    assert_eq!(source.source_xml(), xml);
    assert_eq!(
        selected.package().canonical_bytes().unwrap(),
        package_before
    );
    let original_lookup = lookup_definitions(&source, &base).unwrap();
    assert_eq!(
        external(&original_lookup.records()[0].record).0,
        Status::Missing
    );
}

#[test]
fn ambiguous_fallback_and_exact_variant_collisions_never_pick_a_winner() {
    let mut package = bundled_snapshot().unwrap().package().clone();
    let game = "Caller/AmbiguousExternal";
    for index in 0..3 {
        edit_gem(&mut package, index, |g| {
            g.game_id = game.into();
            g.variant_id = if index == 2 { "Other" } else { "Collision" }.into();
        });
    }
    let keys = package.skill_identities.gems[..3]
        .iter()
        .map(|g| g.key.clone())
        .collect::<BTreeSet<_>>();
    let collision = package.skill_identities.gems[..2]
        .iter()
        .map(|g| g.key.clone())
        .collect::<BTreeSet<_>>();
    let data = snapshot(package);
    let xml = wrap(&format!(
        "<Gem gemId='{game}'/><Gem gemId='{game}' variantId='Unknown'/><Gem gemId='{game}' variantId='Collision'/><Gem gemId='{game}' variantId='Other'/><Gem gemId='Caller/Absent'/>"
    ));
    let source = skill_source::project_xml(&xml).unwrap();
    let result = lookup_definitions(&source, &data).unwrap();
    for (index, reason, expected) in [
        (0, Reason::MissingVariantMultipleCandidates, &keys),
        (1, Reason::UnknownVariantMultipleCandidates, &keys),
        (2, Reason::CollidingExternalVariant, &collision),
    ] {
        let (status, actual, values) = external(&result.records()[index].record);
        assert_eq!(status, Status::Ambiguous);
        assert_eq!(actual, reason);
        assert_eq!(
            values
                .into_iter()
                .map(str::to_owned)
                .collect::<BTreeSet<_>>(),
            *expected
        );
    }
    assert_eq!(external(&result.records()[3].record).0, Status::Exact);
    assert_eq!(
        external(&result.records()[4].record),
        (Status::Missing, Reason::UnknownExternalId, vec![])
    );
}

#[test]
fn authored_game_id_presence_prevents_effect_fallthrough_even_when_empty() {
    let data = bundled_snapshot().unwrap();
    let effect = &data.package().skill_identities.skills[0];
    let gem = &data.package().skill_identities.gems[0];
    let xml = wrap(&format!(
        "<Gem gemId='' skillId='{}' nameSpec='{}'/><Gem gemId='CallerUnknown' skillId='{}'/><Gem skillId='{}'/><Gem skillId='' nameSpec='{}'/><Gem nameSpec='{}'/><Gem/>",
        attr(&effect.id),
        attr(&gem.name),
        attr(&effect.id),
        attr(&effect.id),
        attr(&gem.name),
        attr(&gem.name)
    ));
    let source = skill_source::project_xml(&xml).unwrap();
    let result = lookup_definitions(&source, &data).unwrap();
    for index in 0..2 {
        assert_eq!(external(&result.records()[index].record).0, Status::Missing);
        assert_eq!(
            instance(&result.records()[index].record)
                .skill_id
                .as_ref()
                .unwrap()
                .decoded(),
            effect.id
        );
    }
    assert_eq!(
        instance(&result.records()[0].record)
            .game_id
            .as_ref()
            .unwrap()
            .decoded(),
        ""
    );
    match &instance(&result.records()[2].record).resolution {
        Resolution::ExplicitEffect {
            matched: Some(matched),
            ..
        } => assert_eq!(matched.id, effect.id),
        _ => panic!(),
    }
    assert!(matches!(
        instance(&result.records()[3].record).resolution,
        Resolution::ExplicitEffect { matched: None, .. }
    ));
    assert!(matches!(
        instance(&result.records()[4].record).resolution,
        Resolution::NameOnlyNotResolved
    ));
    assert!(matches!(
        instance(&result.records()[5].record).resolution,
        Resolution::MissingIdentity
    ));
    assert_eq!(source.source_xml(), xml);
}

#[test]
fn explicit_effect_reports_all_possible_primary_owners_and_preserves_optional_flags() {
    let mut package = bundled_snapshot().unwrap().package().clone();
    let effect = package.skill_identities.skills[0].clone();
    for index in 0..3 {
        edit_gem(&mut package, index, |g| {
            g.primary_effect_id = effect.id.clone();
            g.declared_additional_effects.clear();
            g.declared_additional_stat_sets.clear();
            g.constructed_additional_effects.clear();
            g.additional_effects.clear();
            g.effect_list = vec![effect.id.clone()];
            g.display_order = None;
        });
    }
    let expected = package
        .skill_identities
        .gems
        .iter()
        .filter(|g| g.primary_effect_id == effect.id)
        .map(|g| g.key.clone())
        .collect::<BTreeSet<_>>();
    assert!(expected.len() >= 3);
    let data = snapshot(package);
    let xml = wrap(&format!(
        "<Gem skillId='{}'/><Gem skillId='CallerAbsentEffect'/>",
        attr(&effect.id)
    ));
    let source = skill_source::project_xml(&xml).unwrap();
    let result = lookup_definitions(&source, &data).unwrap();
    match &instance(&result.records()[0].record).resolution {
        Resolution::ExplicitEffect {
            matched: Some(m),
            possible_primary_gem_keys,
        } => {
            assert_eq!(m.id, effect.id);
            assert_eq!(m.support, effect.support);
            assert_eq!(m.from_tree, effect.from_tree);
            assert_eq!(
                possible_primary_gem_keys
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>(),
                expected
            );
        }
        _ => panic!(),
    }
    match &instance(&result.records()[1].record).resolution {
        Resolution::ExplicitEffect {
            matched: None,
            possible_primary_gem_keys,
        } => assert!(possible_primary_gem_keys.is_empty()),
        _ => panic!(),
    }
}

#[test]
fn source_paths_cover_legacy_saved_duplicate_and_unknown_positional_instances() {
    let data = bundled_snapshot().unwrap();
    let effect = &data.package().skill_identities.skills[0].id;
    let escaped = attr(effect);
    let xml = format!(
        "<PathOfBuilding2><Skills activeSkillSet='2'><Skill><UnusualGem skillId='{escaped}'><StatSetIndex grantedEffect='{escaped}' index='2'/><MinionSkillIndexLookup grantedEffect='CallerMissing'><UnusualMap skillIndex='1' statSetIndex='2'/></MinionSkillIndexLookup></UnusualGem></Skill><Future><Skill><Gem skillId='{escaped}'/></Skill></Future><SkillSet id='2'><Skill><Gem skillId='{escaped}'/></Skill></SkillSet><SkillSet id='02'><Skill><Gem nameSpec='Caller Name'/></Skill></SkillSet></Skills><Skills xmlns='urn:foreign'><Skill><Gem skillId='{escaped}'/></Skill></Skills><Skills><Skill><Gem skillId='{escaped}'/></Skill></Skills></PathOfBuilding2>"
    );
    let source = skill_source::project_xml(&xml).unwrap();
    let result = lookup_definitions(&source, &data).unwrap();
    assert_eq!(result.records().len(), 6);
    let records = result.records();
    let container = &source.containers()[0];
    let legacy = &container.children()[0];
    assert_eq!(records[0].container_index, 0);
    assert_eq!(records[0].set_source_range, None);
    assert_eq!(
        records[0].group_source_range,
        Some(legacy.element().source_range())
    );
    assert_eq!(
        instance(&records[0].record).source_kind,
        SkillSourceKind::Unknown
    );
    assert_eq!(
        &xml[instance(&records[0].record).source_range.clone()],
        legacy.children()[0].element().source_xml()
    );
    for (index, usage, known) in [
        (1, SkillSourceUse::MainStatSetSelection, true),
        (2, SkillSourceUse::MainMinionLookup, false),
    ] {
        match &records[index].record {
            SkillIdentityRecord::EffectSelection(s) => {
                assert_eq!(s.source_use, usage);
                assert_eq!(s.matched.is_some(), known);
                assert_eq!(
                    records[index].group_source_range,
                    records[0].group_source_range
                );
            }
            _ => panic!(),
        }
    }
    for (record_index, child_index) in [(3, 2), (4, 3)] {
        let set = &container.children()[child_index];
        assert_eq!(
            records[record_index].set_source_range,
            Some(set.element().source_range())
        );
        assert_eq!(
            records[record_index].group_source_range,
            Some(set.children()[0].element().source_range())
        );
    }
    assert_eq!(records[5].container_index, 2);
    assert!(records.iter().all(|r| r.container_index != 1));
    assert_eq!(records[5].set_source_range, None);
    assert_eq!(source.source_xml(), xml);
}

#[test]
fn empty_source_and_source_data_binding_do_not_request_a_default_build() {
    let base = bundled_snapshot().unwrap();
    let source = skill_source::project_xml("<PathOfBuilding2/>").unwrap();
    let result = lookup_definitions(&source, &base).unwrap();
    assert!(result.records().is_empty());
    assert_eq!(result.source_xml_sha256(), source.source_sha256());
    assert_eq!(result.data(), base.identity());
    let mut package = base.package().clone();
    edit_gem(&mut package, 0, |g| {
        g.name = "Different catalog display text".into()
    });
    let custom = snapshot(package);
    let changed = lookup_definitions(&source, &custom).unwrap();
    assert_ne!(changed.data(), result.data());
    assert_eq!(changed.source_xml_sha256(), result.source_xml_sha256());
    let altered_xml = "<PathOfBuilding2><!-- caller bytes --></PathOfBuilding2>";
    let altered = skill_source::project_xml(altered_xml).unwrap();
    assert_ne!(
        lookup_definitions(&altered, &custom)
            .unwrap()
            .source_xml_sha256(),
        changed.source_xml_sha256()
    );
}

#[test]
fn large_injected_display_text_bounds_expanded_evidence_before_publication() {
    let mut package = bundled_snapshot().unwrap().package().clone();
    let game = "CallerLongEvidence";
    edit_gem(&mut package, 0, |g| {
        g.game_id = game.into();
        g.variant_id = "V".into();
        g.name = "x".repeat(4096);
    });
    let data = snapshot(package);
    let xml = wrap(&format!("<Gem gemId='{game}' variantId='V'/>").repeat(600));
    let source = skill_source::project_xml(&xml).unwrap();
    assert!(
        lookup_definitions(&source, &data)
            .unwrap_err()
            .to_string()
            .contains("string bytes")
    );
    assert_eq!(source.source_xml(), xml);
}

#[test]
fn authored_unknown_and_additional_effect_selection_references_stay_distinct() {
    let data = bundled_snapshot().unwrap();
    let gem = data
        .package()
        .skill_identities
        .gems
        .iter()
        .find(|g| g.effect_list.len() > 1)
        .unwrap();
    let additional = gem
        .effect_list
        .iter()
        .find(|id| *id != &gem.primary_effect_id)
        .unwrap();
    let xml = wrap(&format!(
        "<Gem gemId='{}' variantId='{}'><StatSetIndex grantedEffect='{}' index='2'/><StatSetCalcsIndex grantedEffect='{}' index='3'/><MinionSkillIndexLookupCalcs grantedEffect='{}'><MinionSkillIndexMap skillIndex='1' statSetIndex='2'/></MinionSkillIndexLookupCalcs><StatSetIndex grantedEffect='CallerMissingEffect' index='99'/></Gem>",
        attr(&gem.game_id),
        attr(&gem.variant_id),
        attr(additional),
        attr(&gem.primary_effect_id),
        attr(additional)
    ));
    let source = skill_source::project_xml(&xml).unwrap();
    let result = lookup_definitions(&source, &data).unwrap();
    assert_eq!(result.records().len(), 5);
    match &instance(&result.records()[0].record).resolution {
        Resolution::ExternalGem { candidates, .. } => {
            assert_eq!(candidates[0].constructed_effect_list, gem.effect_list)
        }
        _ => panic!(),
    }
    for (index, expected) in [
        (1, Some(additional.as_str())),
        (2, Some(gem.primary_effect_id.as_str())),
        (3, Some(additional.as_str())),
        (4, None),
    ] {
        match &result.records()[index].record {
            SkillIdentityRecord::EffectSelection(s) => {
                assert_eq!(s.matched.as_ref().map(|m| m.id.as_str()), expected)
            }
            _ => panic!(),
        }
    }
}

#[test]
fn many_primary_owners_bound_expanded_matches_before_publication() {
    let mut package = bundled_snapshot().unwrap().package().clone();
    let effect = package.skill_identities.skills[0].id.clone();
    for index in 0..100 {
        edit_gem(&mut package, index, |g| {
            g.key = format!("Q{index}");
            g.primary_effect_id = effect.clone();
            g.declared_additional_effects.clear();
            g.declared_additional_stat_sets.clear();
            g.constructed_additional_effects.clear();
            g.additional_effects.clear();
            g.effect_list = vec![effect.clone()];
            g.display_order = None;
        });
    }
    let data = snapshot(package);
    let xml = wrap(&format!("<Gem skillId='{}'/>", attr(&effect)).repeat(700));
    let source = skill_source::project_xml(&xml).unwrap();
    assert!(
        lookup_definitions(&source, &data)
            .unwrap_err()
            .to_string()
            .contains("match count")
    );
    assert_eq!(source.source_xml(), xml);
}
