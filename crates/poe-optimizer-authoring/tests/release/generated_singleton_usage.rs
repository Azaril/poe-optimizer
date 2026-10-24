//! Exact source accounting, not Skill admission or complete-build evaluation.
//! The derived fixture removes other selected groups, preserving this saved
//! group's bytes and every archived preset. No native coverage is promoted.
use super::{originals, selected_passive_accounting::with_original, tracked_release};
use poe_optimizer_core::{
    build_identity::{BuildLineage, DraftIssueId, SkillPresetId},
    owned_build::DeclaredSlot,
    owned_definitions::*,
    owned_draft::{DraftFinalization, DraftListCompletion},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits, SourceOccurrenceId},
    decode_build,
    owned_normalize::{
        GemInventoryPolicy, GeneratedSkillInputProvider, NormalizationArtifacts,
        NormalizationPolicy, NormalizedImport, OccurrenceUsageTarget, OwnedOriginTarget,
        UsageInputPolicy, UsageInventoryDestination as Destination, UsageInventoryField,
        UsageInventoryFieldSelector as Field, UsageInventoryPolicy, UsageInventoryRow,
        UsageInventorySource, configuration_disposition_dependencies, normalize_fresh,
        usage_inputs_identity,
    },
    owned_source::{SourceEvidenceLimits, SourceEvidenceRow, SourceProjectEvidence},
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};

const SOURCE: &str = "New Item, Ashen Staff";
const OTHER_SOURCE: &str = "Sol Pole, Ashen Staff";
const GROUP_REASON: &str = "group-field-consumers-not-yet-reviewed";
const SOURCE_REASON: &str = "source-field-consumers-not-yet-reviewed";

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn inventory(policy: &NormalizationPolicy) -> &UsageInventoryPolicy {
    let Some(UsageInputPolicy::PobOccurrenceUsageV3 {
        inventory: Some(value),
        ..
    }) = &policy.usage_inputs
    else {
        panic!("current checked usage inventory")
    };
    value
}
fn inventory_mut(policy: &mut NormalizationPolicy) -> &mut UsageInventoryPolicy {
    let Some(UsageInputPolicy::PobOccurrenceUsageV3 {
        inventory: Some(value),
        ..
    }) = &mut policy.usage_inputs
    else {
        panic!("current checked usage inventory")
    };
    value
}
fn source_matches(row: &UsageInventoryRow, expected: &str) -> bool {
    matches!(&row.source,
        UsageInventorySource::Occurrence { target }
        if matches!(target.as_ref(), OccurrenceUsageTarget::Generated { correspondence }
            if matches!(&correspondence.provider,
                GeneratedSkillInputProvider::ItemModifier { source_name, .. }
                if source_name == expected)))
}
fn row_index(inventory: &UsageInventoryPolicy, expected: &str) -> usize {
    let indices: Vec<_> = inventory
        .rows
        .iter()
        .enumerate()
        .filter_map(|(i, row)| source_matches(row, expected).then_some(i))
        .collect();
    assert_eq!(
        indices.len(),
        1,
        "one exact generated source correspondence"
    );
    indices[0]
}
fn namespace(row: &UsageInventoryRow) -> &GameVersionNamespace {
    let UsageInventorySource::Occurrence { target } = &row.source else {
        panic!()
    };
    let OccurrenceUsageTarget::Generated { correspondence } = target.as_ref() else {
        panic!()
    };
    assert_eq!(correspondence.gem.key().as_str(), "def.0000000000000778");
    assert_eq!(correspondence.skill.key().as_str(), "def.0000000000000134");
    assert_eq!(
        correspondence.game_id,
        "Metadata/Items/Gems/SkillGemFirebolt"
    );
    assert_eq!(correspondence.variant_id, "Firebolt");
    assert_eq!(correspondence.skill_id, "FireboltPlayer");
    let GeneratedSkillInputProvider::ItemModifier {
        modifier,
        skill_supply,
        template,
        ..
    } = &correspondence.provider
    else {
        panic!()
    };
    assert_eq!(modifier.key().as_str(), "def.00000000000031c6");
    assert_eq!(
        skill_supply.declaration,
        SlotOwnerDefId::Modifier(modifier.clone())
    );
    assert_eq!(skill_supply.slot.key().as_str(), "def.00000000000031c9");
    assert_eq!(template.key().as_str(), "def.0000000000001d75");
    correspondence.skill.namespace()
}
fn parameter(namespace: &GameVersionNamespace, policy: &str, slot: &str) -> Destination {
    let policy = UsagePolicyDefId::new(namespace.clone(), key(policy));
    Destination::UsageParameter {
        parameter: DeclaredSlot {
            declaration: SlotOwnerDefId::UsagePolicy(policy.clone()),
            slot: ParameterSlotDefId::new(namespace.clone(), key(slot)),
        },
        policy,
    }
}
fn deferred(reason: &str) -> Destination {
    Destination::Deferred {
        reason: key(reason),
    }
}
fn expected_fields(namespace: &GameVersionNamespace, prior: bool) -> Vec<UsageInventoryField> {
    let count = || parameter(namespace, "def.000000000000326a", "def.000000000000326b");
    let group = |name: &str, destinations| UsageInventoryField {
        field: Field::GroupAttribute { name: name.into() },
        destinations,
    };
    let occurrence = |name: &str, destinations| UsageInventoryField {
        field: Field::OccurrenceAttribute { name: name.into() },
        destinations,
    };
    let input = || vec![Destination::InputDisposition];
    let mut fields = vec![
        group(
            "enabled",
            vec![
                Destination::InputDisposition,
                parameter(namespace, "def.000000000000332b", "def.000000000000332c"),
            ],
        ),
        group("groupCount", vec![count()]),
        group(
            "includeInFullDPS",
            vec![parameter(
                namespace,
                "def.000000000000338e",
                "def.000000000000338f",
            )],
        ),
        group("label", input()),
        group(
            "mainActiveSkill",
            if prior {
                vec![deferred(GROUP_REASON)]
            } else {
                input()
            },
        ),
        group(
            "mainActiveSkillCalcs",
            if prior {
                vec![deferred(GROUP_REASON)]
            } else {
                input()
            },
        ),
        group("slot", input()),
        group("source", input()),
        occurrence("corrupted", input()),
        occurrence("corruptLevel", input()),
        occurrence(
            "count",
            if prior {
                vec![deferred(SOURCE_REASON), count()]
            } else {
                vec![count()]
            },
        ),
        occurrence(
            "enabled",
            vec![
                Destination::InputDisposition,
                parameter(namespace, "def.000000000000332b", "def.000000000000332d"),
            ],
        ),
        occurrence("enableGlobal1", input()),
        occurrence(
            "enableGlobal2",
            if prior {
                vec![deferred(SOURCE_REASON)]
            } else {
                input()
            },
        ),
    ];
    for name in [
        "gemId",
        "level",
        "nameSpec",
        "quality",
        "skillId",
        "variantId",
    ] {
        fields.push(occurrence(name, input()));
    }
    fields.push(UsageInventoryField {
        field: Field::ReferenceDescendants,
        destinations: input(),
    });
    assert_eq!(fields.len(), 21);
    fields
}

/// Historical equality gates remove only this current, reviewed field delta.
/// Dependency rebinding belongs to the caller's checked authoring construction.
pub(super) fn restore_prior_destinations(policy: &mut NormalizationPolicy) {
    restore_inventory(inventory_mut(policy));
}

/// Authored source inverses run before the current aggregate binds dependencies.
pub(super) fn restore_inventory(inventory: &mut UsageInventoryPolicy) {
    let index = row_index(inventory, SOURCE);
    assert_eq!(index, 12);
    let row = &mut inventory.rows[index];
    let namespace = namespace(row).clone();
    assert_eq!(row.fields, expected_fields(&namespace, false));
    row.fields = expected_fields(&namespace, true);
}

fn conservative_policy() -> NormalizationPolicy {
    let release = tracked_release().release();
    let mut policy = release.normalization().clone();
    restore_prior_destinations(&mut policy);
    let usage = usage_inputs_identity(&policy, Default::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. }) =
        &mut policy.gem_inventory
    else {
        panic!("current physical inventory")
    };
    *usage_inputs = usage;
    let dependencies =
        configuration_disposition_dependencies(&policy, release.rewards(), Default::default())
            .unwrap();
    policy
        .configuration_dispositions
        .as_mut()
        .unwrap()
        .dependencies = dependencies;
    policy
}
fn normalize(
    evidence: &SourceProjectEvidence<'_>,
    policy: &NormalizationPolicy,
    original_queries: bool,
) -> NormalizedImport {
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    // This is an explicit alternative authored test policy. Bind identical tree
    // content using the real constructor, never repair a supplied runtime seal.
    let tree = OwnedTreeNormalizationPolicy::bind_new(
        release.tree().unwrap().input().content.clone(),
        recipe.registry(),
        recipe.schema().as_ref(),
        release.mapping(),
        policy,
        Default::default(),
    )
    .unwrap();
    let queries = if original_queries {
        release
            .query_sets()
            .iter()
            .find(|q| q.name.as_str() == "original-05")
            .unwrap()
            .queries
            .as_slice()
    } else {
        &[]
    };
    normalize_fresh(
        evidence,
        evidence.identity().allocator,
        NormalizationArtifacts {
            mappings: release.mapping(),
            registry: recipe.registry(),
            definitions: recipe.schema().as_ref(),
            roles: release.roles(),
            rewards: release.rewards(),
            items: release.items(),
            item_source: release.item_source(),
            tree: Some(&tree),
        },
        policy,
        queries,
        Default::default(),
    )
    .unwrap()
}
fn only<T>(mut values: impl Iterator<Item = T>) -> T {
    let first = values.next().expect("exact source or output");
    assert!(values.next().is_none(), "unambiguous source or output");
    first
}
fn attr<'a>(row: &'a SourceEvidenceRow<'_>, name: &str) -> &'a str {
    row.attribute(name).unwrap().decoded().unwrap()
}
fn preset_for(result: &NormalizedImport, source: SourceOccurrenceId) -> SkillPresetId {
    let origin = only(
        result
            .sidecar()
            .origins
            .iter()
            .filter(|r| r.source == source),
    );
    only(origin.links.iter().filter_map(|link| match link {
        OwnedOriginTarget::SkillPreset(id) => Some(*id),
        _ => None,
    }))
}
fn pending_usage(result: &NormalizedImport, preset: SkillPresetId) -> DraftIssueId {
    let preset = only(
        result
            .draft()
            .input()
            .skill_presets
            .members
            .iter()
            .filter(|p| p.id == preset),
    );
    let DraftListCompletion::Pending { id, code } =
        &preset.intent.as_ref().unwrap().usage.completion
    else {
        panic!("retained independent usage obligation")
    };
    assert_eq!(code.as_str(), "usage-preferences-not-converted");
    *id
}
fn assert_pending(result: &NormalizedImport, evidence: &SourceProjectEvidence<'_>) {
    let schema = tracked_release().release().assembled().checked().schema();
    let draft = result.draft();
    let proof = draft
        .prove_intent(schema.as_ref(), Default::default(), Default::default())
        .unwrap();
    let checked = draft
        .finalize_selection_checked(
            schema.as_ref(),
            originals::saved(evidence, result),
            &proof,
            Default::default(),
            Default::default(),
        )
        .unwrap();
    assert!(matches!(
        checked.finalization(),
        DraftFinalization::Pending { .. }
    ));
    assert_eq!(checked.draft_digest(), result.sidecar().draft);
    assert!(tracked_release().release().evaluation().is_none());
}
fn assert_original_archived(evidence: &SourceProjectEvidence<'_>, result: &NormalizedImport) {
    let set = &evidence.rows()[167];
    assert_eq!(attr(set, "id"), "2");
    let preset_id = preset_for(result, set.occurrence().id());
    let preset = only(
        result
            .draft()
            .input()
            .skill_presets
            .members
            .iter()
            .filter(|p| p.id == preset_id),
    );
    assert!(
        preset
            .intent
            .as_ref()
            .unwrap()
            .generated_inputs
            .members
            .is_empty()
    );
    let issue = pending_usage(result, preset_id);
    for ordinal in [174, 175] {
        let source = &evidence.rows()[ordinal];
        let origin = only(
            result
                .sidecar()
                .origins
                .iter()
                .filter(|r| r.source == source.occurrence().id()),
        );
        assert!(origin.links.contains(&OwnedOriginTarget::Issue(issue)));
        assert!(
            !origin
                .links
                .iter()
                .any(|link| matches!(link, OwnedOriginTarget::GeneratedSkillInput { .. }))
        );
        assert!(
            origin
                .links
                .iter()
                .filter(|link| matches!(link, OwnedOriginTarget::Issue(_)))
                .count()
                >= 2
        );
    }
    assert_eq!(attr(&evidence.rows()[175], "skillId"), "FireboltPlayer");
}

#[test]
fn generated_policy_changes_exactly_four_destinations_and_preserves_typed_consumers() {
    let current = tracked_release().release().normalization();
    let before = inventory(current);
    assert_eq!(
        (
            before.rows.len(),
            before.rows.iter().map(|r| r.fields.len()).sum::<usize>()
        ),
        (25, 456)
    );
    assert_eq!(row_index(before, SOURCE), 12);
    assert_eq!(row_index(before, OTHER_SOURCE), 11);
    assert_eq!(
        before.rows[12].fields,
        expected_fields(namespace(&before.rows[12]), false)
    );
    // This other same-Skill provider has its own still-deferred source frame.
    let mut other_expected = expected_fields(namespace(&before.rows[11]), true);
    for row in &mut other_expected {
        for destination in &mut row.destinations {
            if *destination == Destination::InputDisposition {
                *destination = deferred(match &row.field {
                    Field::GroupAttribute { .. } => GROUP_REASON,
                    Field::OccurrenceAttribute { .. } => SOURCE_REASON,
                    Field::ReferenceDescendants => {
                        "exact-source-reference-accounting-not-yet-reviewed"
                    }
                });
            }
        }
    }
    assert_eq!(before.rows[11].fields, other_expected);
    let mut prior = current.clone();
    restore_prior_destinations(&mut prior);
    let after = inventory(&prior);
    assert_eq!(before.evidence, after.evidence);
    for (index, (new, old)) in before.rows.iter().zip(&after.rows).enumerate() {
        if index != 12 {
            assert_eq!(new, old);
            continue;
        }
        assert_eq!(new.source, old.source);
        let changed: Vec<_> = new
            .fields
            .iter()
            .zip(&old.fields)
            .filter_map(|(new, old)| {
                assert_eq!(new.field, old.field);
                (new.destinations != old.destinations).then_some(new.field.clone())
            })
            .collect();
        assert_eq!(
            changed,
            [
                Field::GroupAttribute {
                    name: "mainActiveSkill".into()
                },
                Field::GroupAttribute {
                    name: "mainActiveSkillCalcs".into()
                },
                Field::OccurrenceAttribute {
                    name: "count".into()
                },
                Field::OccurrenceAttribute {
                    name: "enableGlobal2".into()
                },
            ]
        );
    }
    prior.usage_inputs = current.usage_inputs.clone();
    assert_eq!(&prior, current, "no other normalization field changes");
}

#[test]
fn unchanged_original_preserves_all_outputs_and_archived_generated_obligations() {
    with_original(|evidence, current, selection| {
        assert_eq!(attr(&evidence.rows()[244], "skillId"), "FireboltPlayer");
        assert_eq!(
            attr(&evidence.rows()[243], "source"),
            "Item:28:New Item, Ashen Staff"
        );
        let prior = normalize(evidence, &conservative_policy(), true);
        assert_eq!(current.draft().input(), prior.draft().input());
        assert_eq!(current.sidecar().origins, prior.sidecar().origins);
        assert_eq!(current.allocator_after(), prior.allocator_after());
        pending_usage(current, selection.build.skills);
        assert_original_archived(evidence, current);
        assert_pending(current, evidence);
    });
}

#[test]
fn unchanged_saved_group_proves_only_its_selected_usage_inventory() {
    with_original(|original, _, _| {
        let selected = originals::selected_skill_set(original);
        assert_eq!(selected.occurrence().id().ordinal(), 204);
        let retained = &original.rows()[243];
        assert_eq!(
            retained.occurrence().parent(),
            Some(selected.occurrence().id())
        );
        assert_eq!(retained.children().len(), 1);
        assert_eq!(retained.children()[0].ordinal(), 244);
        let saved_group = original
            .source_fragment(retained.occurrence().id())
            .unwrap();
        let mut xml = original.source_xml().to_owned();
        let mut removals: Vec<_> = selected
            .children()
            .iter()
            .filter(|id| **id != retained.occurrence().id())
            .map(|id| original.row(*id).unwrap().occurrence().range())
            .collect();
        assert_eq!(removals.len(), 11);
        removals.sort_by_key(|range| std::cmp::Reverse(range.start));
        for range in removals {
            xml.replace_range(range, "");
        }
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(xml.as_bytes()).unwrap(),
            BuildLineage::from_bytes([5; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
        let derived_set = originals::selected_skill_set(&evidence);
        let [derived_group] = derived_set.children() else {
            panic!("one real saved group")
        };
        assert_eq!(
            evidence.source_fragment(*derived_group).unwrap(),
            saved_group
        );
        for row in original.rows().iter().filter(|r| {
            r.occurrence().name() == "SkillSet" && r.occurrence().id() != selected.occurrence().id()
        }) {
            let same = only(evidence.rows().iter().filter(|r| {
                r.occurrence().name() == "SkillSet" && attr(r, "id") == attr(row, "id")
            }));
            assert_eq!(
                evidence.source_fragment(same.occurrence().id()).unwrap(),
                original.source_fragment(row.occurrence().id()).unwrap()
            );
        }
        let release = tracked_release().release();
        let current = normalize(&evidence, release.normalization(), false);
        let prior = normalize(&evidence, &conservative_policy(), false);
        let preset_id = preset_for(&prior, derived_set.occurrence().id());
        let issue = pending_usage(&prior, preset_id);
        let mut expected = prior.draft().input().clone();
        let preset = expected
            .skill_presets
            .members
            .iter_mut()
            .find(|p| p.id == preset_id)
            .unwrap();
        assert_eq!(
            preset
                .intent
                .as_ref()
                .unwrap()
                .generated_inputs
                .members
                .len(),
            1
        );
        assert_eq!(preset.intent.as_ref().unwrap().usage.members.len(), 3);
        preset.intent.as_mut().unwrap().usage.completion = DraftListCompletion::Complete;
        assert_eq!(
            current.draft().input(),
            &expected,
            "only the selected usage completion changes"
        );
        let mut origins = prior.sidecar().origins.clone();
        let linked: std::collections::BTreeSet<_> = origins
            .iter()
            .filter(|r| r.links.contains(&OwnedOriginTarget::Issue(issue)))
            .map(|r| r.source)
            .collect();
        let [derived_gem] = evidence.row(*derived_group).unwrap().children() else {
            panic!("the retained generated group has one saved Gem")
        };
        // The value projection opens this obligation at the exact Gem; live
        // generated accounting additionally links its group, not the SkillSet.
        assert_eq!(linked, [*derived_group, *derived_gem].into());
        for origin in &mut origins {
            origin
                .links
                .retain(|link| *link != OwnedOriginTarget::Issue(issue));
        }
        assert_eq!(
            current.sidecar().origins,
            origins,
            "only exact usage issue links retire"
        );
        assert_eq!(current.allocator_after(), prior.allocator_after());
        assert_original_archived(&evidence, &current);
        assert_pending(&current, &evidence);
        let replay = normalize(&evidence, release.normalization(), false);
        assert_eq!(current.draft().input(), replay.draft().input());
        assert_eq!(current.sidecar().origins, replay.sidecar().origins);
        assert_eq!(current.allocator_after(), replay.allocator_after());
    });
}
