//! Selected constructor outputs, not a certificate for final incoming values.
//! Foreign modifiers, conditions, transformations and numerical ownership stay open.
use super::{
    class_support_domains as classes, class_support_source as class_source, originals,
    player_support_domains as player, player_support_source as player_source,
    selected_passive_accounting::with_original, tracked_release,
};
use poe_optimizer_core::{
    owned_definitions::*,
    owned_draft::{
        DraftField, DraftFinalization, DraftListCompletion, DraftSessionInput, EvaluationSelection,
    },
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_import::{
    build_instance::AuthoredInstanceId,
    owned_mapping::{
        ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, SourceComponent,
    },
    owned_normalize::{OwnedOriginTarget, SourceDisposition, SourceOwnedOrigin},
    owned_source::SourceProjectEvidence,
};
use serde_json::json;
use std::collections::BTreeSet;

type Result<T> = std::result::Result<T, &'static str>;
fn ensure(value: bool, why: &'static str) -> Result<()> {
    value.then_some(()).ok_or(why)
}
fn only<T>(mut values: impl Iterator<Item = T>) -> Result<T> {
    let first = values.next().ok_or("missing exact row")?;
    ensure(values.next().is_none(), "duplicate exact row")?;
    Ok(first)
}

// Exact consumed/emitted channels, not spelling fragments. Pinned constructors and
// name-preserving factories must authenticate each candidate before this join.
const WATCHED_CHANNELS: &[(&str, &str)] = &[
    ("RevivingMinion", "FLAG"),
    ("BaseReviveTime", "OVERRIDE"),
    ("MinionRevivalSpeed", "INC"),
    ("MinionRevivalSpeed", "MORE"),
    ("UmbralWell", "FLAG"),
    ("UmbralWellBuffValue", "BASE"),
    ("ProjectileSpeed", "INC"),
    ("DealNoDamage", "FLAG"),
];
#[derive(Debug, PartialEq, Eq)]
enum Output {
    DifferentChannel,
    Relevant,
    UnresolvedKind,
}
fn classify(name: &str, kind: &str) -> Output {
    if WATCHED_CHANNELS.contains(&(name, kind)) {
        Output::Relevant
    } else if WATCHED_CHANNELS.iter().any(|(read, _)| *read == name) {
        // An unexpected operation on a watched channel is never an absence proof.
        Output::UnresolvedKind
    } else {
        Output::DifferentChannel
    }
}
fn application() -> DeclaredSet<ExistingActorRuleApplication> {
    serde_json::from_value(player_source::verified_ownership()["applicability"].clone()).unwrap()
}
fn account(
    evidence: &SourceProjectEvidence<'_>,
    draft: &DraftSessionInput,
    origins: &[SourceOwnedOrigin],
    selection: &EvaluationSelection,
    applications: Option<&DeclaredSet<ExistingActorRuleApplication>>,
) -> Result<(ClassDefId, ActorDefId)> {
    let character = only(
        draft
            .character_presets
            .members
            .iter()
            .filter(|row| row.id == selection.build.character),
    )?;
    only(
        draft
            .allocation_presets
            .members
            .iter()
            .filter(|row| row.id == selection.build.allocations),
    )?;
    let character_link = OwnedOriginTarget::CharacterPreset(character.id);
    let allocation_link = OwnedOriginTarget::AllocationPreset(selection.build.allocations);
    let spec = originals::selected_spec(evidence);
    ensure(
        matches!(
            spec.authored_instance(),
            Some(AuthoredInstanceId::PassiveSpec(_))
        ) && !spec.occurrence().has_namespace_context(),
        "selected source is not the authored Spec",
    )?;
    // Build and Tree now retain legitimate header provenance for this same
    // Character. The owning row is the exact saved Spec, never the first link.
    let origin = only(
        origins
            .iter()
            .filter(|row| row.source == spec.occurrence().id()),
    )?;
    ensure(
        origin.disposition == SourceDisposition::Contributes
            && only(origin.links.iter().filter_map(|link| match link {
                OwnedOriginTarget::CharacterPreset(id) => Some(*id),
                _ => None,
            }))? == character.id
            && only(origin.links.iter().filter_map(|link| match link {
                OwnedOriginTarget::AllocationPreset(id) => Some(*id),
                _ => None,
            }))? == selection.build.allocations,
        "crossed or duplicate selected source links",
    )?;
    let root = evidence.rows().first().ok_or("missing source root")?;
    let tree_source = evidence
        .row(spec.occurrence().parent().ok_or("missing Spec parent")?)
        .map_err(|_| "unknown Spec parent")?;
    let build_source = only(evidence.rows().iter().filter(|row| {
        row.occurrence().parent() == Some(root.occurrence().id())
            && row.occurrence().name() == "Build"
    }))?;
    ensure(
        tree_source.occurrence().name() == "Tree"
            && tree_source.occurrence().parent() == Some(root.occurrence().id())
            && !tree_source.occurrence().has_namespace_context()
            && !build_source.occurrence().has_namespace_context(),
        "foreign source header",
    )?;
    let DraftListCompletion::Pending { id: issue, code } = &draft.saved_variants.completion else {
        return Err("saved selection must remain unresolved");
    };
    ensure(
        draft.saved_variants.members.is_empty() && code.as_str() == "saved-selection-not-converted",
        "changed saved selection authority",
    )?;
    let tree_origin = only(
        origins
            .iter()
            .filter(|row| row.source == tree_source.occurrence().id()),
    )?;
    let build_origin = only(
        origins
            .iter()
            .filter(|row| row.source == build_source.occurrence().id()),
    )?;
    ensure(
        tree_origin.disposition == SourceDisposition::Contributes
            && tree_origin.links
                == [
                    allocation_link.clone(),
                    character_link.clone(),
                    OwnedOriginTarget::Issue(*issue),
                ],
        "changed selected Tree projection",
    )?;
    let mut build_links: Vec<_> = draft
        .character_presets
        .members
        .iter()
        .map(|row| OwnedOriginTarget::CharacterPreset(row.id))
        .collect();
    build_links.extend([
        OwnedOriginTarget::SkillPreset(selection.build.skills),
        OwnedOriginTarget::Issue(*issue),
    ]);
    ensure(
        build_origin.disposition == SourceDisposition::Contributes
            && build_origin.links == build_links,
        "changed shared Build projection",
    )?;
    let mut character_sources = BTreeSet::new();
    for projection in origins
        .iter()
        .filter(|row| row.links.contains(&character_link))
    {
        ensure(
            [origin.source, tree_origin.source, build_origin.source].contains(&projection.source)
                && character_sources.insert(projection.source)
                && projection
                    .links
                    .iter()
                    .filter(|link| **link == character_link)
                    .count()
                    == 1,
            "foreign or duplicate Character projection",
        )?;
    }
    ensure(
        character_sources.len() == 3,
        "missing Character source projection",
    )?;
    // Import also links each Spec child and each attribute-override entry to
    // its containing preset. The proved parent Tree is a header projection;
    // every other auxiliary origin must still descend from the selected Spec.
    let mut spec_owners = 0;
    let mut projected_sources = BTreeSet::new();
    for projection in origins
        .iter()
        .filter(|row| row.links.contains(&allocation_link))
    {
        ensure(
            projection
                .links
                .iter()
                .filter(|row| **row == allocation_link)
                .count()
                == 1
                && projected_sources.insert(projection.source),
            "duplicate allocation source projection",
        )?;
        let source = evidence
            .row(projection.source)
            .map_err(|_| "unknown allocation source")?;
        if projection.source == tree_origin.source {
            continue;
        }
        spec_owners += usize::from(source.occurrence().name() == "Spec");
        let mut cursor = source;
        let mut descendant = false;
        // Evidence is bounded already; retain an explicit walk bound even for
        // a detached malformed graph, without an unbounded ancestor traversal.
        for _ in 0..evidence.rows().len() {
            if cursor.occurrence().id() == origin.source {
                descendant = true;
                break;
            }
            let Some(parent) = cursor.occurrence().parent() else {
                break;
            };
            cursor = evidence
                .row(parent)
                .map_err(|_| "unknown source ancestor")?;
        }
        ensure(
            descendant,
            "allocation projection belongs to a foreign Spec",
        )?;
    }
    ensure(spec_owners == 1, "duplicate allocation Spec owner")?;
    let release = tracked_release().release();
    let tree = &release.tree().ok_or("missing tree policy")?.input().content;
    let raw = spec
        .attribute(&tree.syntax.class_attribute)
        .ok_or("missing source Class")?
        .decoded()
        .map_err(|_| "malformed source Class")?;
    let selector = ExternalSelector::Definition(ExternalOwnerSelector::Class {
        key: SourceComponent::Text(raw.into()),
    });
    let retained = only(
        class_source::verified_mapping()
            .entries
            .iter()
            .filter(|row| row.source == selector),
    )?;
    ensure(
        release.mapping().lookup(&selector) == Some(&retained.outcome),
        "changed source mapping",
    )?;
    let MappingOutcome::Mapped {
        target: SchemaSubject::Definition(DefinitionAddress::Class(class)),
        basis: MappingBasis::Exact,
    } = &retained.outcome
    else {
        return Err("Class source is not exactly mapped");
    };
    ensure(
        character.class.to_resolved().as_ref() == Some(class),
        "crossed selected Class",
    )?;
    let declared = only(tree.classes.iter().filter(|row| row.key == raw))?;
    ensure(
        &declared.class == class,
        "tree Class differs from source mapping",
    )?;
    let expected = application();
    ensure(
        applications == Some(&expected),
        "unreviewed shared Actor application",
    )?;
    let shared = only(expected.members.iter())?;
    ensure(
        shared.targets == [ExistingActorRuleTarget::Player],
        "not the actual Player",
    )?;
    let rules = release.assembled().checked().rules().input();
    for subject in [class.address(), shared.owner.address()] {
        let owner = only(
            rules
                .owners
                .iter()
                .filter(|row| row.owner == SchemaSubject::Definition(subject.clone())),
        )?;
        ensure(
            !owner.programs.is_complete(),
            "intrinsic accounting cannot complete an owner",
        )?;
    }
    Ok((class.clone(), shared.owner.clone()))
}

#[test]
fn full_constructor_outputs_are_disjoint_while_context_and_foreign_sources_remain_open() {
    let census = player_source::verified_intrinsic_modifiers();
    player_source::check_intrinsic_modifiers(census);
    let rows = census.as_array().unwrap();
    assert_eq!(rows.len(), 148);
    let mut lines = BTreeSet::new();
    for row in rows {
        assert!(lines.insert(row["line"].as_u64().unwrap()));
        assert_eq!(
            classify(row["name"].as_str().unwrap(), row["kind"].as_str().unwrap()),
            Output::DifferentChannel
        );
    }
    // Dynamic names come from the complete fixed three-name Class loop, whose
    // body and dispatch are independently checked by both source helpers.
    class_source::check_intrinsic_outputs(
        class_source::verified_classes(),
        class_source::verified_unarmed(),
    );
    for (name, field) in [
        ("Str", "base_str"),
        ("Dex", "base_dex"),
        ("Int", "base_int"),
    ] {
        assert_eq!(classify(name, "BASE"), Output::DifferentChannel);
        for class in class_source::verified_classes().as_array().unwrap() {
            assert!(class[field].as_u64().is_some());
        }
    }
    // Unarmed fields belong to weapon construction, not arbitrary ModDB names.
    let fields = BTreeSet::from([
        "AttackRate",
        "CritChance",
        "PhysicalMax",
        "PhysicalMin",
        "type",
    ]);
    for row in class_source::verified_unarmed()["classes"]
        .as_array()
        .unwrap()
    {
        assert_eq!(
            row["fields"]
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            fields
        );
        for field in &fields {
            assert_eq!(classify(field, "weapon-field"), Output::DifferentChannel);
        }
    }
    // The pinned constructors also set Buffed/Combat/Effective conditions and
    // Level context. Those may affect foreign conditional modifiers. Neither
    // they nor item/tree/config/party/cache/support/application inputs receive
    // a known-empty disposition here. Current native programs are NOT used to
    // infer that any unconverted source output is absent.
    for (name, kind) in WATCHED_CHANNELS {
        assert_eq!(classify(name, kind), Output::Relevant);
        assert_eq!(
            classify(name, "unreviewed-operation"),
            Output::UnresolvedKind
        );
    }
}

#[test]
fn saved_class_and_single_shared_player_join_without_closing_original_five() {
    classes::check_accounted_classes();
    player::check_accounted_player();
    with_original(|evidence, normalized, selection| {
        let draft = normalized.draft();
        let snapshot = draft.clone();
        let origins = normalized.sidecar().origins.clone();
        let release = tracked_release().release();
        let recipe = release.assembled().checked();
        let rules = recipe.rules().input();
        assert!(!rules.contribution_queries.as_ref().unwrap().is_complete());
        let (class, actor) = account(
            evidence,
            draft.input(),
            &origins,
            &selection,
            rules.existing_actor_rules.as_ref(),
        )
        .unwrap();
        let selected_source = originals::selected_spec(evidence).occurrence().id();
        let tree_source = originals::selected_spec(evidence)
            .occurrence()
            .parent()
            .unwrap();
        let character_link = OwnedOriginTarget::CharacterPreset(selection.build.character);
        let character_sources: BTreeSet<_> = origins
            .iter()
            .filter(|row| row.links.contains(&character_link))
            .map(|row| evidence.row(row.source).unwrap().occurrence().name())
            .collect();
        assert_eq!(character_sources, BTreeSet::from(["Build", "Spec", "Tree"]));
        let link = OwnedOriginTarget::AllocationPreset(selection.build.allocations);
        let auxiliary: Vec<_> = origins
            .iter()
            .filter(|row| {
                row.source != selected_source
                    && row.source != tree_source
                    && row.links.contains(&link)
            })
            .collect();
        assert!(!auxiliary.is_empty());
        let syntax = &release.tree().unwrap().input().content.syntax;
        for required in [
            &syntax.overrides_element,
            &syntax.attribute_override_element,
        ] {
            assert!(
                auxiliary
                    .iter()
                    .any(|row| evidence.row(row.source).unwrap().occurrence().name()
                        == required.as_str())
            );
        }
        assert_eq!(class.key().as_str(), "def.0000000000000a23");
        assert_eq!(actor.key().as_str(), "def.000000000000332a");
        let proof = draft
            .prove_intent(
                recipe.schema().as_ref(),
                Default::default(),
                Default::default(),
            )
            .unwrap();
        let checked = draft
            .finalize_selection_checked(
                recipe.schema().as_ref(),
                selection,
                &proof,
                Default::default(),
                Default::default(),
            )
            .unwrap();
        assert!(
            matches!(checked.finalization(), DraftFinalization::Pending { issues, .. } if issues.len() == 5)
        );
        assert_eq!(checked.draft_digest(), normalized.sidecar().draft);
        assert_eq!(draft, &snapshot);
        assert_eq!(normalized.sidecar().origins, origins);
        assert!(release.evaluation().is_none());
    });
}

#[test]
fn crossed_selection_and_copied_or_retargeted_actor_authority_are_rejected() {
    with_original(|evidence, normalized, selection| {
        let draft = normalized.draft().input();
        let origins = &normalized.sidecar().origins;
        let expected = application();
        let baseline = account(evidence, draft, origins, &selection, Some(&expected)).unwrap();
        let character_index = draft
            .character_presets
            .members
            .iter()
            .position(|p| p.id == selection.build.character)
            .unwrap();
        let link = OwnedOriginTarget::CharacterPreset(selection.build.character);
        let selected_spec = originals::selected_spec(evidence);
        let origin_index = origins
            .iter()
            .position(|row| row.source == selected_spec.occurrence().id())
            .unwrap();
        let tree_index = origins
            .iter()
            .position(|row| Some(row.source) == selected_spec.occurrence().parent())
            .unwrap();
        let build_source = only(evidence.rows().iter().filter(|row| {
            row.occurrence().name() == "Build"
                && row.occurrence().parent() == Some(evidence.rows()[0].occurrence().id())
        }))
        .unwrap()
        .occurrence()
        .id();
        let build_index = origins
            .iter()
            .position(|row| row.source == build_source)
            .unwrap();
        let allocation_link = OwnedOriginTarget::AllocationPreset(selection.build.allocations);
        let auxiliary_index = origins
            .iter()
            .position(|row| {
                row.source != origins[origin_index].source
                    && row.source != origins[tree_index].source
                    && row.links.contains(&allocation_link)
            })
            .unwrap();
        let foreign_spec = evidence
            .rows()
            .iter()
            .find(|row| {
                row.occurrence().name() == "Spec"
                    && row.occurrence().id() != origins[origin_index].source
            })
            .unwrap();
        let foreign_index = origins
            .iter()
            .position(|row| row.source == foreign_spec.occurrence().id())
            .unwrap();
        let foreign_character = draft
            .character_presets
            .members
            .iter()
            .find(|row| row.id != selection.build.character)
            .unwrap()
            .id;
        let foreign_allocation = draft
            .allocation_presets
            .members
            .iter()
            .find(|row| row.id != selection.build.allocations)
            .unwrap()
            .id;
        let foreign_skill = draft
            .skill_presets
            .members
            .iter()
            .find(|row| row.id != selection.build.skills)
            .unwrap()
            .id;
        for control in 0..27 {
            let mut changed = draft.clone();
            let mut links = origins.clone();
            let mut applications = Some(expected.clone());
            match control {
                0 => links[origin_index].links.retain(|row| row != &link),
                1 => links[origin_index].links.push(link.clone()),
                2 => links.push(links[origin_index].clone()),
                3 => links[origin_index].links.retain(|row| {
                    row != &OwnedOriginTarget::AllocationPreset(selection.build.allocations)
                }),
                4 => changed
                    .character_presets
                    .members
                    .push(changed.character_presets.members[character_index].clone()),
                5 => {
                    changed.character_presets.members[character_index].class = DraftField::Known {
                        value: classes::declarations()
                            .into_iter()
                            .find_map(|row| match row.owner {
                                SchemaSubject::Definition(DefinitionAddress::Class(class))
                                    if class != baseline.0 =>
                                {
                                    Some(class)
                                }
                                _ => None,
                            })
                            .unwrap(),
                    }
                }
                6 => applications = None,
                7 => {
                    let members = &mut applications.as_mut().unwrap().members;
                    let mut copied = members[0].clone();
                    copied.id = "copied-initializer".parse().unwrap();
                    members.push(copied);
                }
                8 => {
                    applications.as_mut().unwrap().members[0].targets =
                        vec![ExistingActorRuleTarget::ScenarioEnemy]
                }
                9 => {
                    let recipe = tracked_release().release().assembled().checked();
                    let foreign = recipe
                        .schema()
                        .input()
                        .definitions
                        .iter()
                        .find_map(|row| match row {
                            DefinitionDescriptor::Actor(row) if row.id != baseline.1 => {
                                Some(row.id.clone())
                            }
                            _ => None,
                        })
                        .unwrap();
                    applications.as_mut().unwrap().members[0].owner = foreign;
                }
                10 => {
                    let selected = originals::selected_spec(evidence);
                    let other = evidence
                        .rows()
                        .iter()
                        .find(|row| {
                            row.occurrence().name() == "Spec"
                                && row.occurrence().id() != selected.occurrence().id()
                                && row.attribute("classInternalId").unwrap().decoded().unwrap()
                                    == selected
                                        .attribute("classInternalId")
                                        .unwrap()
                                        .decoded()
                                        .unwrap()
                        })
                        .unwrap()
                        .occurrence()
                        .id();
                    // Swap rather than duplicate source IDs: the original
                    // source census still contains each Spec exactly once.
                    let other_index = links.iter().position(|row| row.source == other).unwrap();
                    links[other_index].source = links[origin_index].source;
                    links[origin_index].source = other;
                }
                11 => links[foreign_index].links.push(allocation_link.clone()),
                12 => links.push(links[auxiliary_index].clone()),
                13 => links[auxiliary_index].links.push(allocation_link.clone()),
                14 => {
                    links[auxiliary_index].source = *foreign_spec.children().first().unwrap();
                }
                15 => links[foreign_index].links.push(link.clone()),
                16 => links[auxiliary_index].links.push(link.clone()),
                17 => links.push(links[tree_index].clone()),
                18 => links[tree_index].links.push(allocation_link.clone()),
                19 => {
                    links[tree_index].links[0] =
                        OwnedOriginTarget::AllocationPreset(foreign_allocation)
                }
                20 => {
                    links[tree_index].links[1] =
                        OwnedOriginTarget::CharacterPreset(foreign_character)
                }
                21 => links[tree_index]
                    .links
                    .retain(|row| !matches!(row, OwnedOriginTarget::Issue(_))),
                22 => links.push(links[build_index].clone()),
                23 => links[build_index].links.push(link.clone()),
                24 => {
                    let selected = links[build_index]
                        .links
                        .iter_mut()
                        .find(|row| matches!(row, OwnedOriginTarget::SkillPreset(_)))
                        .unwrap();
                    *selected = OwnedOriginTarget::SkillPreset(foreign_skill);
                }
                25 => {
                    // Preserve both source IDs exactly once while exchanging
                    // their incompatible header responsibilities.
                    let tree_source = links[tree_index].source;
                    links[tree_index].source = links[build_index].source;
                    links[build_index].source = tree_source;
                }
                26 => {
                    links[tree_index].disposition = SourceDisposition::SourceOnly(
                        OwnedDefinitionKey::new("unreviewed-header").unwrap(),
                    )
                }
                _ => unreachable!(),
            }
            assert!(
                account(
                    evidence,
                    &changed,
                    &links,
                    &selection,
                    applications.as_ref()
                )
                .is_err(),
                "control {control}"
            );
        }
        assert_eq!(
            account(evidence, draft, origins, &selection, Some(&expected)).unwrap(),
            baseline
        );
    });
}

#[test]
fn changed_missing_extra_and_retargeted_constructor_outputs_require_new_proof() {
    let census = player_source::verified_intrinsic_modifiers();
    player_source::check_intrinsic_modifiers(census);
    class_source::check_intrinsic_outputs(
        class_source::verified_classes(),
        class_source::verified_unarmed(),
    );
    for control in 0..5 {
        let mut changed = census.clone();
        match control {
            0 => changed[0]["name"] = json!("RevivingMinion"),
            1 => changed[0]["kind"] = json!("OVERRIDE"),
            2 => {
                changed.as_array_mut().unwrap().remove(0);
            }
            3 => {
                changed
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"line":900,"name":"MinionRevivalSpeed","kind":"INC"}));
            }
            4 => changed[0]["recipient"] = json!("enemy"),
            _ => unreachable!(),
        }
        assert!(
            std::panic::catch_unwind(|| player_source::check_intrinsic_modifiers(&changed))
                .is_err()
        );
    }
    for control in 0..3 {
        let mut classes = class_source::verified_classes().clone();
        let mut unarmed = class_source::verified_unarmed().clone();
        match control {
            0 => classes[0]["base_str"] = json!(99),
            1 => classes[0]["UmbralWell"] = json!(true),
            2 => unarmed["classes"][0]["fields"]["ProjectileSpeed"] = json!(15),
            _ => unreachable!(),
        }
        assert!(
            std::panic::catch_unwind(|| class_source::check_intrinsic_outputs(&classes, &unarmed))
                .is_err()
        );
    }
}
