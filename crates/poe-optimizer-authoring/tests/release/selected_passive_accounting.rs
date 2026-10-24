//! Exact saved-source accounting. Intrinsic disjointness does not close incoming
//! numerical queries, effective-node transformations, descendants or readiness.
use super::{
    attribute_support_domains as attributes, attribute_support_source as attribute_source,
    gigantic_support_domains as gigantic, minion_support_domains as minions, originals,
    root_support_domains as roots, socket_support_source as socket,
    structural_support_domains as structural, structural_support_source as structural_source,
    tracked_release, view_support_domains as views,
};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{LoadoutScope, ParameterValue},
    owned_definitions::*,
    owned_draft::{
        DraftAllocationAccess, DraftFinalization, DraftListCompletion, DraftSessionInput,
        EvaluationSelection,
    },
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_mapping::{
        ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, SourceComponent,
    },
    owned_normalize::{
        NormalizationArtifacts, NormalizedImport, OwnedOriginTarget, SourceOwnedOrigin,
        normalize_fresh,
    },
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_tree_policy::TreeTokenRole,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    sync::OnceLock,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    Attribute,
    Scalar,
    Root,
    View,
    Socket,
    Gigantic,
    Structural,
    Unresolved,
}
impl Family {
    fn batch(self) -> usize {
        match self {
            Self::Structural => 1,
            Self::Unresolved => 2,
            _ => 0,
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
struct Accounted {
    source: String,
    node: PassiveNodeDefId,
    family: Family,
}
type Result<T> = std::result::Result<T, &'static str>;
fn ensure(value: bool, why: &'static str) -> Result<()> {
    value.then_some(()).ok_or(why)
}
fn only<T>(mut values: impl Iterator<Item = T>) -> Result<T> {
    let first = values.next().ok_or("missing exact row")?;
    ensure(values.next().is_none(), "duplicate exact row")?;
    Ok(first)
}
fn selector(source: &str) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode {
        tree_version: SourceComponent::Text("0_5".into()),
        node_id: SourceComponent::Text(source.into()),
        view: SourceComponent::Missing,
    })
}
fn node(source: &str) -> Result<PassiveNodeDefId> {
    let entry = only(
        attribute_source::verified_mapping()
            .entries
            .iter()
            .filter(|r| r.source == selector(source)),
    )?;
    ensure(
        tracked_release().release().mapping().lookup(&entry.source) == Some(&entry.outcome),
        "current exact mapping changed",
    )?;
    match &entry.outcome {
        MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::PassiveNode(node)),
            basis: MappingBasis::Exact,
        } => Ok(node.clone()),
        _ => Err("not an exact physical passive mapping"),
    }
}
fn definition(
    node: &PassiveNodeDefId,
) -> &'static DefinitionEntry<PassiveNodeDefId, PassiveNodeSchema> {
    let Some(DefinitionDescriptor::PassiveNode(value)) = tracked_release()
        .release()
        .assembled()
        .checked()
        .schema()
        .lookup_definition(&node.address())
    else {
        panic!("known passive definition")
    };
    value
}
fn rules(node: &PassiveNodeDefId) -> &'static DefinitionRules {
    only(
        tracked_release()
            .release()
            .assembled()
            .checked()
            .rules()
            .input()
            .owners
            .iter()
            .filter(|r| r.owner == SchemaSubject::Definition(node.address())),
    )
    .unwrap()
}
fn families() -> &'static BTreeMap<PassiveNodeDefId, Family> {
    static FAMILIES: OnceLock<BTreeMap<PassiveNodeDefId, Family>> = OnceLock::new();
    FAMILIES.get_or_init(|| {
        let mut result = BTreeMap::new();
        for (family, domains) in [
            (Family::Attribute, attributes::declarations()),
            (Family::Scalar, minions::declarations()),
            (Family::View, views::declarations()),
            (Family::Gigantic, gigantic::declarations()),
            (Family::Structural, structural::declarations()),
        ] {
            for domain in domains {
                let SchemaSubject::Definition(DefinitionAddress::PassiveNode(id)) = domain.owner
                else {
                    panic!("physical passive family")
                };
                assert!(result.insert(id, family).is_none());
            }
        }
        for (descriptor, _) in roots::reviewed_roots() {
            let DefinitionAddress::PassiveNode(id) = descriptor.address() else {
                panic!("implicit passive root")
            };
            assert!(result.insert(id, Family::Root).is_none());
        }
        socket::check_authored();
        let closure: socket::Closure = socket::read("closure.json");
        let DefinitionAddress::PassiveNode(id) = closure.definition.address() else {
            panic!("socket passive")
        };
        assert!(result.insert(id, Family::Socket).is_none());
        let unresolved = structural_source::evidence()["unresolved"]
            .as_array()
            .unwrap();
        assert_eq!(unresolved.len(), 6);
        for row in unresolved {
            let id = node(row["node_id"].as_str().unwrap()).unwrap();
            assert_eq!(id.key().as_str(), row["passive"].as_str().unwrap());
            assert!(result.insert(id, Family::Unresolved).is_none());
        }
        result
    })
}
fn accounts_for(
    family: Family,
    definition: &DefinitionEntry<PassiveNodeDefId, PassiveNodeSchema>,
    body: &DefinitionRules,
) -> bool {
    if families().get(&definition.id) != Some(&family) {
        return false;
    }
    match family {
        Family::Attribute => attributes::accounts_for(definition, body),
        Family::Scalar => minions::accounts_for(definition, body),
        Family::Gigantic => gigantic::producer_family(definition, body),
        Family::Root => roots::reviewed_roots()
            .iter()
            .any(|(d, r)| d == &DefinitionDescriptor::PassiveNode(definition.clone()) && r == body),
        Family::Socket => {
            let expected: socket::Closure = socket::read("closure.json");
            expected.definition == DefinitionDescriptor::PassiveNode(definition.clone())
                && expected.owner == *body
        }
        Family::View => {
            // The existing finite shape plus both whole branches authenticate
            // this snapshot; equality here also rejects detached mutations.
            views::check_accounted_views();
            definition == self::definition(&definition.id) && body == rules(&definition.id)
        }
        Family::Structural => {
            structural::check_accounted_structure();
            !body.programs.is_complete()
                && definition == self::definition(&definition.id)
                && body == rules(&definition.id)
        }
        Family::Unresolved => !body.programs.is_complete(),
    }
}
pub(super) fn with_original(
    test: impl FnOnce(&SourceProjectEvidence<'_>, &NormalizedImport, EvaluationSelection),
) {
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let xml = fs::read(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml")).unwrap();
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(&xml).unwrap(),
        BuildLineage::from_bytes([5; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let query_set = release
        .query_sets()
        .iter()
        .find(|r| r.name.as_str() == "original-05")
        .unwrap();
    let normalized = normalize_fresh(
        &evidence,
        *imported.allocator_state(),
        NormalizationArtifacts {
            mappings: release.mapping(),
            registry: recipe.registry(),
            definitions: recipe.schema().as_ref(),
            roles: release.roles(),
            rewards: release.rewards(),
            items: release.items(),
            item_source: release.item_source(),
            tree: release.tree(),
        },
        release.normalization(),
        &query_set.queries,
        Default::default(),
    )
    .unwrap();
    let selection = originals::saved(&evidence, &normalized);
    test(&evidence, &normalized, selection);
}
fn account(
    evidence: &SourceProjectEvidence<'_>,
    draft: &DraftSessionInput,
    origins: &[SourceOwnedOrigin],
    selection: &EvaluationSelection,
) -> Result<Vec<Accounted>> {
    let preset = only(
        draft
            .allocation_presets
            .members
            .iter()
            .filter(|r| r.id == selection.build.allocations),
    )?;
    ensure(
        preset.allocations.completion == DraftListCompletion::Complete
            && preset.equipment.completion == DraftListCompletion::Complete,
        "selected source membership is unresolved",
    )?;
    ensure(
        preset.equipment.members.is_empty(),
        "occupied passive sockets require their own source accounting",
    )?;
    let selected: BTreeSet<_> = preset.allocations.members.iter().copied().collect();
    ensure(
        selected.len() == preset.allocations.members.len(),
        "duplicated selected allocation",
    )?;
    let origin = only(origins.iter().filter(|origin| {
        origin
            .links
            .contains(&OwnedOriginTarget::AllocationPreset(preset.id))
            && evidence
                .row(origin.source)
                .is_ok_and(|row| row.occurrence().name() == "Spec")
    }))?;
    let spec = evidence
        .row(origin.source)
        .map_err(|_| "invalid source origin")?;
    ensure(
        origin
            .links
            .iter()
            .filter(|link| **link == OwnedOriginTarget::AllocationPreset(preset.id))
            .count()
            == 1,
        "ambiguous preset link",
    )?;
    ensure(
        origin.links.contains(&OwnedOriginTarget::CharacterPreset(
            selection.build.character,
        )),
        "foreign character source",
    )?;
    let raw = spec
        .attribute("nodes")
        .ok_or("saved nodes missing")?
        .decoded()
        .map_err(|_| "invalid node attribute")?;
    let tokens: Vec<_> = raw.split(',').collect();
    ensure(
        tokens.iter().copied().collect::<BTreeSet<_>>().len() == tokens.len(),
        "duplicate saved node",
    )?;
    let mut allocated = BTreeSet::new();
    let mut implicit = BTreeSet::new();
    let mut result = Vec::new();
    let tree = &attribute_source::verified_tree().content;
    for token in tokens {
        let id = node(token)?;
        let family = *families().get(&id).ok_or("unreviewed source family")?;
        let role = &only(tree.tokens.iter().filter(|r| r.token == token))?.role;
        match role {
            TreeTokenRole::Allocation { node, pool } if node == &id => {
                let allocation = only(draft.allocations.members.iter().filter(|a| {
                    selected.contains(&a.id) && a.node.to_resolved().as_ref() == Some(&id)
                }))?;
                ensure(
                    allocated.insert(allocation.id),
                    "source aliases an allocation",
                )?;
                ensure(
                    allocation.pool.to_resolved().as_ref() == Some(pool),
                    "foreign allocation pool",
                )?;
                ensure(
                    allocation.scope.to_resolved() == Some(LoadoutScope::Shared)
                        && allocation.access == DraftAllocationAccess::Ordinary,
                    "unreviewed allocation scope or access",
                )?;
                let link = OwnedOriginTarget::Allocation(allocation.id);
                ensure(
                    origin.links.iter().filter(|r| **r == link).count() == 1,
                    "missing or duplicate allocation origin",
                )?;
                ensure(
                    origins.iter().filter(|r| r.links.contains(&link)).count() == 1,
                    "allocation has foreign source origin",
                )?;
                if family == Family::Attribute {
                    let attribute = only(tree.attributes.iter().filter(|r| r.node == id))?;
                    ensure(
                        allocation.choices.completion == DraftListCompletion::Complete,
                        "attribute choice coverage unresolved",
                    )?;
                    let choice = only(allocation.choices.members.iter())?;
                    ensure(
                        choice.slot.to_resolved().as_ref() == Some(&attribute.slot),
                        "foreign attribute choice slot",
                    )?;
                    let Some(ParameterValue::Option(option)) = choice.value.to_resolved() else {
                        return Err("missing attribute option");
                    };
                    let overrides = only(evidence.rows().iter().filter(|r| {
                        r.occurrence().parent() == Some(origin.source)
                            && r.occurrence().name() == tree.syntax.overrides_element
                    }))?;
                    let saved = only(evidence.rows().iter().filter(|r| {
                        r.occurrence().parent() == Some(overrides.occurrence().id())
                            && r.occurrence().name() == tree.syntax.attribute_override_element
                    }))?;
                    let expected = only(attribute.lanes.iter().filter(|lane| {
                        saved.attribute(&lane.attribute).is_some_and(|v| {
                            v.decoded()
                                .is_ok_and(|v| v.split(',').any(|part| part == token))
                        })
                    }))?;
                    ensure(expected.option == option, "saved attribute option differs")?;
                }
            }
            TreeTokenRole::ImplicitRoot { node } if node == &id => {
                let link = OwnedOriginTarget::ImplicitPassive {
                    character: selection.build.character,
                    node: id.clone(),
                };
                ensure(
                    origin.links.iter().filter(|r| **r == link).count() == 1,
                    "missing or duplicate implicit source",
                )?;
                ensure(
                    origins.iter().filter(|r| r.links.contains(&link)).count() == 1,
                    "implicit root has foreign origin",
                )?;
                ensure(implicit.insert(id.clone()), "duplicate implicit passive")?;
            }
            _ => return Err("unaccounted tree token role"),
        }
        ensure(
            accounts_for(family, definition(&id), rules(&id)),
            "whole source/output family not accounted",
        )?;
        result.push(Accounted {
            source: token.into(),
            node: id,
            family,
        });
    }
    ensure(
        allocated == selected,
        "selected allocation missing from source",
    )?;
    let allocation_links: Vec<_> = origin
        .links
        .iter()
        .filter_map(|link| match link {
            OwnedOriginTarget::Allocation(id) => Some(*id),
            _ => None,
        })
        .collect();
    ensure(
        allocation_links.len() == allocated.len()
            && allocation_links.iter().copied().collect::<BTreeSet<_>>() == allocated,
        "unaccounted allocation source link",
    )?;
    let implicit_links: Vec<_> = origin
        .links
        .iter()
        .filter_map(|link| match link {
            OwnedOriginTarget::ImplicitPassive { character, node }
                if *character == selection.build.character =>
            {
                Some(node.clone())
            }
            _ => None,
        })
        .collect();
    ensure(
        implicit_links.len() == implicit.len()
            && implicit_links.iter().cloned().collect::<BTreeSet<_>>() == implicit,
        "unaccounted implicit source link",
    )?;
    Ok(result)
}

#[test]
fn selected_sources_join_exactly_fifty_five_allocations_and_two_implicit_roots() {
    with_original(|evidence, normalized, selection| {
        let draft = normalized.draft();
        let before = draft.clone();
        let origins = normalized.sidecar().origins.clone();
        let rows = account(evidence, draft.input(), &origins, &selection).unwrap();
        assert_eq!(rows.len(), 57);
        let mut batches = [0; 3];
        for row in &rows {
            batches[row.family.batch()] += 1;
        }
        assert_eq!(batches, [45, 6, 6]);
        for (family, count) in [
            (Family::Attribute, 22),
            (Family::Scalar, 18),
            (Family::Root, 2),
            (Family::View, 1),
            (Family::Socket, 1),
            (Family::Gigantic, 1),
        ] {
            assert_eq!(rows.iter().filter(|r| r.family == family).count(), count);
        }
        assert_eq!(
            rows.iter()
                .filter(|r| r.family == Family::Structural)
                .map(|r| r.source.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["13289", "19318", "32705", "35560", "45333", "62210"])
        );
        assert_eq!(
            rows.iter()
                .filter(|r| r.family == Family::Unresolved)
                .map(|r| r.source.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["10561", "14686", "23265", "26926", "53324", "53795"])
        );
        let schema = tracked_release().release().assembled().checked().schema();
        let proof = draft
            .prove_intent(schema.as_ref(), Default::default(), Default::default())
            .unwrap();
        let checked = draft
            .finalize_selection_checked(
                schema.as_ref(),
                selection,
                &proof,
                Default::default(),
                Default::default(),
            )
            .unwrap();
        assert!(
            matches!(checked.finalization(), DraftFinalization::Pending { issues, .. } if issues.len() == 5)
        );
        assert_eq!(draft, &before);
        assert_eq!(normalized.sidecar().origins, origins);
        assert_eq!(checked.draft_digest(), normalized.sidecar().draft);
        assert!(tracked_release().release().evaluation().is_none());
    });
}

#[test]
fn missing_duplicate_foreign_origins_choices_and_occupied_sockets_do_not_become_absence() {
    with_original(|evidence, normalized, selection| {
        let draft = normalized.draft().input();
        let origins = &normalized.sidecar().origins;
        let baseline = account(evidence, draft, origins, &selection).unwrap();
        let preset_index = draft
            .allocation_presets
            .members
            .iter()
            .position(|p| p.id == selection.build.allocations)
            .unwrap();
        let first = draft.allocation_presets.members[preset_index]
            .allocations
            .members[0];
        let link = OwnedOriginTarget::Allocation(first);
        let origin_index = origins
            .iter()
            .position(|r| r.links.contains(&link))
            .unwrap();
        for control in 0..9 {
            let mut changed = draft.clone();
            let mut links = origins.clone();
            match control {
                0 => links[origin_index].links.retain(|r| r != &link),
                1 => links[origin_index].links.push(link.clone()),
                2 => links.push(links[origin_index].clone()),
                3 => changed.allocation_presets.members[preset_index]
                    .allocations
                    .members
                    .push(first),
                4 => {
                    changed.allocations.members.retain(|a| a.id != first);
                }
                5 => {
                    let id = &baseline
                        .iter()
                        .find(|r| r.family == Family::Attribute)
                        .unwrap()
                        .node;
                    let members = &changed.allocation_presets.members[preset_index]
                        .allocations
                        .members;
                    let a = changed
                        .allocations
                        .members
                        .iter_mut()
                        .find(|a| {
                            members.contains(&a.id) && a.node.to_resolved().as_ref() == Some(id)
                        })
                        .unwrap();
                    a.choices.members.clear();
                }
                6 => changed.allocation_presets.members[preset_index]
                    .equipment
                    .members
                    .push(changed.equipment.members[0].id),
                7 => {
                    links[origin_index].links.retain(|r| r != &link);
                    let other = links
                        .iter_mut()
                        .find(|r| {
                            r.source != origins[origin_index].source
                                && evidence
                                    .row(r.source)
                                    .is_ok_and(|row| row.occurrence().name() == "Spec")
                        })
                        .unwrap();
                    other.links.push(link.clone());
                }
                8 => {
                    let id = &baseline
                        .iter()
                        .find(|r| r.family == Family::Attribute)
                        .unwrap()
                        .node;
                    let members = &changed.allocation_presets.members[preset_index]
                        .allocations
                        .members;
                    let a = changed
                        .allocations
                        .members
                        .iter_mut()
                        .find(|a| {
                            members.contains(&a.id) && a.node.to_resolved().as_ref() == Some(id)
                        })
                        .unwrap();
                    let previous = a.choices.members[0].value.to_resolved().unwrap();
                    let attribute = attribute_source::verified_tree()
                        .content
                        .attributes
                        .iter()
                        .find(|r| &r.node == id)
                        .unwrap();
                    let other = attribute
                        .lanes
                        .iter()
                        .map(|r| ParameterValue::Option(r.option.clone()))
                        .find(|v| *v != previous)
                        .unwrap();
                    a.choices.members[0].value = other.into();
                }
                _ => unreachable!(),
            }
            assert!(
                account(evidence, &changed, &links, &selection).is_err(),
                "control {control}"
            );
        }
        assert_eq!(
            account(evidence, draft, origins, &selection).unwrap(),
            baseline
        );
    });
}

#[test]
fn whole_family_proofs_refuse_changed_conditions_paired_effects_and_unproved_completion() {
    for (source, family, mutation) in [
        ("15782", Family::Attribute, 0),
        ("19006", Family::Scalar, 1),
        ("14945", Family::Scalar, 2),
        ("46365", Family::Gigantic, 1),
    ] {
        let id = node(source).unwrap();
        let definition = definition(&id);
        let original = rules(&id);
        assert!(accounts_for(family, definition, original));
        let mut body = original.clone();
        match mutation {
            0 => body.programs.members[0].effects[0].when = None,
            1 => {
                body.programs.members.pop();
            }
            2 => {
                body.programs.members[0].effects.pop();
            }
            _ => unreachable!(),
        }
        assert!(!accounts_for(family, definition, &body), "source {source}");
    }
    let id = node("10561").unwrap();
    assert_eq!(families()[&id], Family::Unresolved);
    assert!(!accounts_for(Family::Root, definition(&id), rules(&id)));
    let mut fake_completion = rules(&id).clone();
    fake_completion.programs.closure = SchemaClosure::Complete;
    assert!(!accounts_for(
        Family::Unresolved,
        definition(&id),
        &fake_completion
    ));
}
