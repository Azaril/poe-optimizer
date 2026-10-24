//! Intrinsic template domains do not certify modifiers, equipment completeness,
//! generated descendants, numerical ownership or incoming contribution queries.
use super::{
    item_support_source as source, selected_passive_accounting::with_original, tracked_release,
};
use poe_optimizer_core::{
    build_identity::ItemRecordId,
    owned_definitions::*,
    owned_draft::{DraftFinalization, DraftListCompletion},
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, SourceOccurrenceId},
    owned_mapping::{MappingBasis, MappingEntry, MappingOutcome},
    owned_normalize::{OwnedOriginTarget, SourceOwnedOrigin},
    owned_source::SourceProjectEvidence,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn mapping(index: usize) -> MappingEntry {
    typed(&source::proof()["families"][index]["mapping_entries"][0])
}
pub(super) fn declarations() -> Vec<SupportSourceDomainDeclaration> {
    source::TEMPLATES
        .iter()
        .enumerate()
        .map(|(index, key)| {
            let row = mapping(index);
            let MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::ItemTemplate(template)),
                basis: MappingBasis::Exact,
            } = row.outcome
            else {
                panic!("exact ItemTemplate mapping")
            };
            assert_eq!(template.key().as_str(), *key);
            SupportSourceDomainDeclaration {
                owner: SchemaSubject::Definition(template.address()),
                domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
            }
        })
        .collect()
}
fn empty<T>(value: &DeclaredSet<T>) {
    assert!(value.is_complete() && value.members.is_empty());
}

// Item range writes also refer to the physical record. They are provenance
// projections, not additional Item owners or a modifier-completeness claim.
fn item_source(
    evidence: &SourceProjectEvidence<'_>,
    origins: &[SourceOwnedOrigin],
    item: ItemRecordId,
) -> Result<SourceOccurrenceId, &'static str> {
    let link = OwnedOriginTarget::Item(item);
    let mut sources = BTreeSet::new();
    let mut owner = None;
    let mut projections = Vec::new();
    for origin in origins.iter().filter(|row| row.links.contains(&link)) {
        if !sources.insert(origin.source)
            || origin
                .links
                .iter()
                .filter(|target| matches!(target, OwnedOriginTarget::Item(_)))
                .count()
                != 1
        {
            return Err("duplicate or crossed Item correspondence");
        }
        let source = evidence
            .row(origin.source)
            .map_err(|_| "unknown Item correspondence source")?;
        match source.occurrence().name() {
            "Item" => {
                if !matches!(
                    source.authored_instance(),
                    Some(AuthoredInstanceId::ItemRecord(_))
                ) {
                    return Err("source is not an authored Item record");
                }
                if owner.replace(origin.source).is_some() {
                    return Err("duplicate Item owner");
                }
            }
            "ModRange" => projections.push(source),
            _ => return Err("unreviewed Item correspondence source"),
        }
    }
    let owner = owner.ok_or("missing Item owner")?;
    for projection in projections {
        if projection.occurrence().parent() != Some(owner) {
            return Err("range projection belongs to a foreign Item");
        }
    }
    Ok(owner)
}

#[test]
fn four_exact_template_domains_preserve_intrinsic_and_numerical_boundaries() {
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    let rules = recipe.rules().input();
    let expected = declarations();
    let providers = &rules.support_discovery.as_ref().unwrap().providers;
    let actual: Vec<_> = providers
        .iter()
        .filter(|row| {
            matches!(
                row.owner,
                SchemaSubject::Definition(DefinitionAddress::ItemTemplate(_))
            )
        })
        .collect();
    assert_eq!(actual.len(), expected.len());
    for (index, declaration) in expected.iter().enumerate() {
        assert_eq!(
            actual.iter().filter(|row| ***row == *declaration).count(),
            1
        );
        let row = mapping(index);
        assert_eq!(release.mapping().lookup(&row.source), Some(&row.outcome));
        let SchemaSubject::Definition(address) = &declaration.owner else {
            unreachable!()
        };
        let DefinitionDescriptor::ItemTemplate(descriptor) =
            recipe.schema().lookup_definition(address).unwrap()
        else {
            unreachable!()
        };
        let SchemaState::Known(schema) = &descriptor.schema else {
            panic!("known template")
        };
        let DefinitionDescriptor::ItemTemplate(retained) =
            typed(&source::migration()["schema"][index]["value"])
        else {
            panic!("retained template")
        };
        let SchemaState::Known(retained) = retained.schema else {
            panic!("retained known template")
        };
        // Exact owned membership, not observed emptiness of a loaded mod list.
        empty(&schema.declarations.grants);
        empty(&schema.declarations.actors);
        empty(&schema.declarations.skill_grants);
        assert_eq!(schema.declarations.grants, retained.declarations.grants);
        assert_eq!(schema.declarations.actors, retained.declarations.actors);
        assert_eq!(
            schema.declarations.skill_grants,
            retained.declarations.skill_grants
        );
        assert!(!schema.modifiers.is_complete());
        assert!(!schema.declarations.parameters.is_complete());
        assert!(!schema.declarations.choices.is_complete());
        assert!(!schema.declarations.outputs.is_complete());
        assert!(!schema.declarations.sockets.is_complete());
        let owners: Vec<_> = rules
            .owners
            .iter()
            .filter(|row| row.owner == declaration.owner)
            .collect();
        assert_eq!(owners.len(), 1);
        assert!(!owners[0].programs.is_complete());
    }
    // Item-template admission never lends authority to its rolled modifiers.
    // Only the separately proved fixed-Life family has its own domain now.
    let modifiers: Vec<_> = providers
        .iter()
        .filter(|row| {
            matches!(
                row.owner,
                SchemaSubject::Definition(DefinitionAddress::Modifier(_))
            )
        })
        .cloned()
        .collect();
    assert_eq!(modifiers, super::flat_life_support_domain::declarations());
    assert!(!rules.contribution_queries.as_ref().unwrap().is_complete());
    assert!(release.evaluation().is_none());
}

#[test]
fn firebolt_keeps_exact_modifier_grant_and_supply_ownership() {
    let release = tracked_release().release();
    let schema = release.assembled().checked().schema();
    let relation = &source::proof()["firebolt_owned_relation"];
    let retained: DefinitionDescriptor = typed(&relation["modifier"]);
    let DefinitionDescriptor::Modifier(retained) = retained else {
        panic!("modifier")
    };
    assert_eq!(retained.id.key().as_str(), "def.00000000000031c6");
    let DefinitionDescriptor::Modifier(current) =
        schema.lookup_definition(&retained.id.address()).unwrap()
    else {
        unreachable!()
    };
    let SchemaState::Known(modifier) = &current.schema else {
        panic!("known modifier")
    };
    let owner = SlotOwnerDefId::Modifier(retained.id);
    let SlotDescriptor::Grant(retained_grant) = typed(&relation["grant"]) else {
        panic!("grant")
    };
    let SlotDescriptor::SkillGrant(retained_supply) = typed(&relation["supply"]) else {
        panic!("supply")
    };
    assert_eq!(retained_grant.id.declaration, owner);
    assert_eq!(retained_supply.id.declaration, owner);
    assert_eq!(
        retained_grant.id.slot.key().as_str(),
        "def.00000000000031c8"
    );
    assert_eq!(
        retained_supply.id.slot.key().as_str(),
        "def.00000000000031c9"
    );
    assert!(modifier.declarations.grants.is_complete());
    assert!(modifier.declarations.skill_grants.is_complete());
    assert_eq!(
        modifier.declarations.grants.members,
        vec![retained_grant.id.clone()]
    );
    assert_eq!(
        modifier.declarations.skill_grants.members,
        vec![retained_supply.id.clone()]
    );
    let SlotDescriptor::Grant(grant) = schema
        .lookup_slot(&GrantSlotDefId::address(&retained_grant.id))
        .unwrap()
    else {
        unreachable!()
    };
    assert_eq!(grant.schema, retained_grant.schema);
    let SlotDescriptor::SkillGrant(supply) = schema
        .lookup_slot(&SkillGrantSlotDefId::address(&retained_supply.id))
        .unwrap()
    else {
        unreachable!()
    };
    let SchemaState::Known(supply) = &supply.schema else {
        panic!("known supply")
    };
    assert_eq!(supply.skill.key().as_str(), "def.0000000000000134");
    assert!(!supply.outputs.is_complete());
    assert!(!modifier.declarations.outputs.is_complete());
    let DefinitionDescriptor::Skill(skill) =
        schema.lookup_definition(&supply.skill.address()).unwrap()
    else {
        unreachable!()
    };
    let SchemaState::Known(skill) = &skill.schema else {
        panic!("known Skill")
    };
    assert!(!skill.declarations.outputs.is_complete());
    // The retained dependency packet independently pins these exact source owners.
    for (field, value) in [
        ("preserved_definitions", &relation["modifier"]),
        ("preserved_slots", &relation["grant"]),
        ("preserved_slots", &relation["supply"]),
    ] {
        assert_eq!(
            source::dependencies()[field]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| *row == value)
                .count(),
            1
        );
    }
}

#[test]
fn selected_four_template_uses_leave_repeated_inventory_and_five_obligations_intact() {
    with_original(|evidence, normalized, selection| {
        let draft = normalized.draft();
        let snapshot = draft.clone();
        let origin_snapshot = normalized.sidecar().origins.clone();
        let input = draft.input();
        let presets: Vec<_> = input
            .equipment_presets
            .members
            .iter()
            .filter(|row| row.id == selection.build.equipment)
            .collect();
        assert_eq!(presets.len(), 1);
        assert!(matches!(
            presets[0].equipment.completion,
            DraftListCompletion::Complete
        ));
        let selected: BTreeSet<_> = presets[0].equipment.members.iter().copied().collect();
        assert_eq!(selected.len(), 9);
        let uses: Vec<_> = input
            .equipment
            .members
            .iter()
            .filter(|row| selected.contains(&row.id))
            .map(|row| row.to_resolved().unwrap())
            .collect();
        assert_eq!(uses.len(), 9);
        let mut multiplicity = BTreeMap::new();
        for use_ in &uses {
            *multiplicity.entry(use_.item).or_insert(0usize) += 1;
        }
        assert_eq!(multiplicity.len(), 8);
        let mut counts: Vec<_> = multiplicity.values().copied().collect();
        counts.sort();
        assert_eq!(counts, [1, 1, 1, 1, 1, 1, 1, 2]);
        for (index, expected) in declarations().iter().enumerate() {
            let SchemaSubject::Definition(DefinitionAddress::ItemTemplate(template)) =
                &expected.owner
            else {
                unreachable!()
            };
            let items: Vec<_> = input
                .items
                .members
                .iter()
                .filter(|row| {
                    multiplicity.contains_key(&row.id)
                        && row.template.to_resolved().as_ref() == Some(template)
                })
                .collect();
            assert_eq!(items.len(), 1);
            assert_eq!(multiplicity[&items[0].id], 1);
            let item = items[0].id;
            let origins = &normalized.sidecar().origins;
            let owner = item_source(evidence, origins, item).unwrap();
            let source_row = evidence.row(owner).unwrap();
            assert_eq!(source_row.occurrence().name(), "Item");
            assert_eq!(
                source_row
                    .attribute("id")
                    .unwrap()
                    .decoded()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap(),
                source::proof()["families"][index]["source_item_id"]
                    .as_u64()
                    .unwrap()
            );
            if index == 0 {
                let link = OwnedOriginTarget::Item(item);
                let owner_index = origins.iter().position(|row| row.source == owner).unwrap();
                // The authentic staff has retained range projections. Preserve
                // their presence without pinning an incidental range-row count.
                let auxiliary_index = origins
                    .iter()
                    .position(|row| row.source != owner && row.links.contains(&link))
                    .unwrap();
                let foreign_range = evidence
                    .rows()
                    .iter()
                    .find(|row| {
                        row.occurrence().name() == "ModRange"
                            && row.occurrence().parent() != Some(owner)
                    })
                    .unwrap()
                    .occurrence()
                    .id();
                let foreign_owner = evidence
                    .rows()
                    .iter()
                    .find(|row| row.occurrence().name() == "Item" && row.occurrence().id() != owner)
                    .unwrap()
                    .occurrence()
                    .id();
                let foreign_index = origins
                    .iter()
                    .position(|row| row.source == foreign_owner)
                    .unwrap();
                let other_item = input
                    .items
                    .members
                    .iter()
                    .find(|row| row.id != item)
                    .unwrap()
                    .id;
                let other_source = evidence
                    .rows()
                    .iter()
                    .find(|row| row.occurrence().name() == "Spec")
                    .unwrap()
                    .occurrence()
                    .id();
                for control in 0..8 {
                    let mut changed = origins.clone();
                    match control {
                        0 => {
                            changed.remove(owner_index);
                        }
                        1 => changed.push(changed[owner_index].clone()),
                        2 => changed.push(changed[auxiliary_index].clone()),
                        3 => changed[auxiliary_index].links.push(link.clone()),
                        4 => changed[auxiliary_index].source = foreign_range,
                        5 => changed[foreign_index].links.push(link.clone()),
                        6 => changed[auxiliary_index]
                            .links
                            .push(OwnedOriginTarget::Item(other_item)),
                        7 => changed[auxiliary_index].source = other_source,
                        _ => unreachable!(),
                    }
                    assert!(
                        item_source(evidence, &changed, item).is_err(),
                        "control {control}"
                    );
                }
                assert_eq!(item_source(evidence, origins, item).unwrap(), owner);
            }
        }
        let recipe = tracked_release().release().assembled().checked();
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
        assert_eq!(normalized.sidecar().origins, origin_snapshot);
    });
}
