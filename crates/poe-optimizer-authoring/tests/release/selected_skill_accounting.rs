//! Saved source accounting, not runtime effect expansion or incoming coverage.
//! A Gem container, its ordered catalog effects, an authored Skill and a generated
//! Skill are different identities. Repeated physical supports remain distinct.
use super::{originals, selected_passive_accounting::with_original, tracked_release};
use poe_optimizer_core::{
    build_identity::*,
    owned_build::{
        AuthoredSkillSource, DeclaredSlot, GeneratedSkillKey, ParameterValue, ProviderRoot,
        SkillTarget,
    },
    owned_definitions::*,
    owned_draft::{DraftFinalization, DraftListCompletion, DraftSessionInput, EvaluationSelection},
    owned_preset_intent::PresetApplicability,
    owned_schema::*,
};
use poe_optimizer_import::{
    build_instance::SourceOccurrenceId,
    owned_mapping::{
        ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, SourceComponent,
    },
    owned_normalize::{
        GeneratedSkillInputPolicy, GeneratedSkillInputProvider, OwnedOriginTarget,
        SourceOwnedOrigin,
    },
    owned_skill_roles::{
        OwnedGemMaterialization, OwnedGemRole, OwnedGemSourceConstruction, OwnedPrimarySkill,
        OwnedSourceEffectResolution,
    },
    owned_source::{SourceEvidenceRow, SourceProjectEvidence},
    owned_value::ValueCodecKind,
};
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, &'static str>;
fn ensure(value: bool, message: &'static str) -> Result<()> {
    value.then_some(()).ok_or(message)
}
fn only<T>(mut values: impl Iterator<Item = T>) -> Result<T> {
    let value = values.next().ok_or("missing exact row")?;
    ensure(values.next().is_none(), "duplicate exact row")?;
    Ok(value)
}
fn attr<'a>(row: &'a SourceEvidenceRow<'_>, key: &str) -> Result<&'a str> {
    row.attribute(key)
        .ok_or("missing source attribute")?
        .decoded()
        .map_err(|_| "invalid source attribute")
}
fn origin(origins: &[SourceOwnedOrigin], source: SourceOccurrenceId) -> Result<&SourceOwnedOrigin> {
    only(origins.iter().filter(|r| r.source == source))
}
fn one_link<T>(
    origin: &SourceOwnedOrigin,
    select: impl Fn(&OwnedOriginTarget) -> Option<T>,
) -> Result<T> {
    only(origin.links.iter().filter_map(select))
}
fn optional_link<T>(
    origin: &SourceOwnedOrigin,
    select: impl Fn(&OwnedOriginTarget) -> Option<T>,
) -> Result<Option<T>> {
    let mut values = origin.links.iter().filter_map(select);
    let result = values.next();
    ensure(values.next().is_none(), "duplicated occurrence link")?;
    Ok(result)
}
#[derive(Debug, PartialEq, Eq)]
enum Kind {
    Physical(SkillUseId),
    Support(SupportAssignmentId),
    Direct(SkillUseId),
    Generated(Box<GeneratedSkillKey>),
}
#[derive(Debug, PartialEq, Eq)]
struct Accounted {
    source: SourceOccurrenceId,
    group: SourceOccurrenceId,
    effect: String,
    definition: SkillDefId,
    gem: Option<GemInstanceId>,
    kind: Kind,
}

fn generated(
    evidence: &SourceProjectEvidence<'_>,
    draft: &DraftSessionInput,
    origins: &[SourceOwnedOrigin],
    selection: &EvaluationSelection,
    group: &SourceEvidenceRow<'_>,
    target: &GeneratedSkillKey,
    skill: &SkillDefId,
) -> Result<(DeclaredSlot<ParameterSlotDefId>, UnitDefId)> {
    let schema = tracked_release().release().assembled().checked().schema();
    ensure(
        target.provider.grant_path.is_empty(),
        "unreviewed nested provider",
    )?;
    let Some(GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 { rows: rules, .. }) =
        &tracked_release()
            .release()
            .normalization()
            .generated_skill_inputs
    else {
        return Err("missing checked generated policy");
    };
    let rule = only(rules.iter().filter(|r| {
        &r.skill == skill
            && match &r.provider {
                GeneratedSkillInputProvider::TreeAllocation {
                    skill_supply,
                    source_node_id,
                    ..
                } => {
                    skill_supply == &target.slot
                        && attr(group, "source")
                            .is_ok_and(|s| s == format!("Tree:{source_node_id}"))
                }
                GeneratedSkillInputProvider::ItemModifier {
                    skill_supply,
                    source_name,
                    ..
                } => {
                    skill_supply == &target.slot
                        && attr(group, "source")
                            .is_ok_and(|s| s.ends_with(&format!(":{source_name}")))
                }
            }
    }))?;
    let corroboration = OwnedOriginTarget::GeneratedSkillInput {
        skill_preset: selection.build.skills,
        target: target.clone(),
    };
    let owner = match &target.provider.root {
        ProviderRoot::Allocation(id) => {
            let preset = only(
                draft
                    .allocation_presets
                    .members
                    .iter()
                    .filter(|r| r.id == selection.build.allocations),
            )?;
            ensure(
                preset
                    .allocations
                    .members
                    .iter()
                    .filter(|r| *r == id)
                    .count()
                    == 1,
                "unselected generated allocation",
            )?;
            let allocation = only(draft.allocations.members.iter().filter(|r| r.id == *id))?;
            let spec = originals::selected_spec(evidence);
            let origin = origin(origins, spec.occurrence().id())?;
            ensure(
                origin.links.contains(&OwnedOriginTarget::Allocation(*id))
                    && origin.links.contains(&corroboration),
                "crossed generated allocation source",
            )?;
            let node = allocation
                .node
                .to_resolved()
                .ok_or("unresolved generated allocation")?;
            let GeneratedSkillInputProvider::TreeAllocation {
                source_node_id,
                passive,
                ..
            } = &rule.provider
            else {
                return Err("crossed generated provider family");
            };
            ensure(
                &node == passive
                    && attr(group, "source")? == format!("Tree:{source_node_id}")
                    && group.attribute("slot").is_none(),
                "crossed saved tree selector",
            )?;
            // One source node cannot silently choose a second same-definition allocation.
            ensure(
                draft
                    .allocations
                    .members
                    .iter()
                    .filter(|r| {
                        preset.allocations.members.contains(&r.id)
                            && r.node.to_resolved().as_ref() == Some(&node)
                    })
                    .count()
                    == 1,
                "ambiguous generated allocation occurrence",
            )?;
            SlotOwnerDefId::PassiveNode(node)
        }
        ProviderRoot::ItemModifier {
            equipment_use,
            modifier,
        } => {
            let preset = only(
                draft
                    .equipment_presets
                    .members
                    .iter()
                    .filter(|r| r.id == selection.build.equipment),
            )?;
            ensure(
                preset
                    .equipment
                    .members
                    .iter()
                    .filter(|r| *r == equipment_use)
                    .count()
                    == 1,
                "unselected generated equipment",
            )?;
            let equipped = only(
                draft
                    .equipment
                    .members
                    .iter()
                    .filter(|r| r.id == *equipment_use),
            )?
            .to_resolved()
            .ok_or("unresolved equipment")?;
            let item = only(draft.items.members.iter().filter(|r| r.id == equipped.item))?;
            let rolled = only(item.modifiers.members.iter().filter(|r| r.id == *modifier))?;
            let item_origin = only(origins.iter().filter(|r| {
                r.links.contains(&OwnedOriginTarget::Item(item.id))
                    && evidence
                        .row(r.source)
                        .is_ok_and(|s| s.occurrence().name() == "Item")
            }))?;
            ensure(
                item_origin
                    .links
                    .contains(&OwnedOriginTarget::Modifier(*modifier))
                    && item_origin.links.contains(&corroboration),
                "crossed generated modifier source",
            )?;
            let item_row = evidence
                .row(item_origin.source)
                .map_err(|_| "invalid Item source")?;
            let definition = rolled
                .definition
                .to_resolved()
                .ok_or("unresolved generated modifier")?;
            let GeneratedSkillInputProvider::ItemModifier {
                modifier: expected,
                template,
                source_name,
                ..
            } = &rule.provider
            else {
                return Err("crossed generated provider family");
            };
            ensure(
                &definition == expected
                    && item.template.to_resolved().as_ref() == Some(template)
                    && attr(group, "source")?
                        == format!("Item:{}:{source_name}", attr(item_row, "id")?),
                "crossed saved item selector",
            )?;
            let slot_origin = only(origins.iter().filter(|r| {
                r.links
                    .contains(&OwnedOriginTarget::Equipment(*equipment_use))
                    && evidence
                        .row(r.source)
                        .is_ok_and(|s| s.occurrence().name() == "Slot")
            }))?;
            let slot = evidence
                .row(slot_origin.source)
                .map_err(|_| "invalid equipment source")?;
            let set = evidence
                .row(slot.occurrence().parent().ok_or("missing ItemSet")?)
                .map_err(|_| "invalid ItemSet")?;
            let set_origin = origin(origins, set.occurrence().id())?;
            ensure(
                set.occurrence().name() == "ItemSet"
                    && set_origin
                        .links
                        .contains(&OwnedOriginTarget::EquipmentPreset(preset.id))
                    && set_origin.links.contains(&corroboration)
                    && slot_origin.links.contains(&corroboration)
                    && attr(slot, "name")? == attr(group, "slot")?
                    && attr(slot, "itemId")? == attr(item_row, "id")?,
                "crossed saved equipment selector",
            )?;
            SlotOwnerDefId::Modifier(definition)
        }
        _ => return Err("unreviewed generated root"),
    };
    ensure(
        target.slot.declaration == owner,
        "crossed generated supply owner",
    )?;
    let Some(SlotDescriptor::SkillGrant(supply)) =
        schema.lookup_slot(&SkillGrantSlotDefId::address(&target.slot))
    else {
        return Err("missing generated supply");
    };
    let SchemaState::Known(supply) = &supply.schema else {
        return Err("unresolved generated supply");
    };
    ensure(&supply.skill == skill, "crossed generated skill")?;
    // Output coverage remains a separate obligation; this is not effect expansion.
    ensure(
        !supply.outputs.is_complete(),
        "unexpected generated output closure",
    )?;
    let parameter = only(rule.parameters.iter())?;
    let ValueCodecKind::Quantity { unit, scale, .. } = &parameter.value.codec.codec else {
        return Err("unreviewed generated quality codec");
    };
    ensure(
        scale.numerator.get() == 1 && scale.denominator.get() == 1,
        "unreviewed generated quality scaling",
    )?;
    Ok((parameter.slot.clone(), unit.clone()))
}

fn account(
    evidence: &SourceProjectEvidence<'_>,
    draft: &DraftSessionInput,
    origins: &[SourceOwnedOrigin],
    selection: &EvaluationSelection,
) -> Result<Vec<Accounted>> {
    let release = tracked_release().release();
    let schema = release.assembled().checked().schema();
    let preset = only(
        draft
            .skill_presets
            .members
            .iter()
            .filter(|r| r.id == selection.build.skills),
    )?;
    ensure(
        preset.skills.completion == DraftListCompletion::Complete
            && preset.supports.completion == DraftListCompletion::Complete,
        "unresolved authored inventory",
    )?;
    let set = originals::selected_skill_set(evidence);
    let set_id = set.occurrence().id();
    ensure(
        one_link(origin(origins, set_id)?, |r| {
            if let OwnedOriginTarget::SkillPreset(id) = r {
                Some(*id)
            } else {
                None
            }
        })? == preset.id,
        "crossed saved skill preset",
    )?;
    let intent = preset.intent.as_ref().ok_or("missing preset intent")?;
    ensure(
        intent.generated_inputs.completion == DraftListCompletion::Complete,
        "unresolved generated inputs",
    )?;
    let mut rows = Vec::new();
    let mut skills = Vec::new();
    let mut supports = Vec::new();
    let mut gems = BTreeSet::new();
    let mut targets = BTreeSet::new();
    let mut expected_order = BTreeMap::new();
    for group_id in set.children() {
        let group = evidence.row(*group_id).map_err(|_| "missing group")?;
        ensure(
            group.occurrence().name() == "Skill" && group.occurrence().parent() == Some(set_id),
            "crossed group",
        )?;
        let mut group_skills = Vec::new();
        let mut group_supports = Vec::new();
        for source_id in group.children() {
            let row = evidence.row(*source_id).map_err(|_| "missing Gem row")?;
            ensure(
                row.occurrence().name() == "Gem"
                    && row.occurrence().parent() == Some(*group_id)
                    && row.children().is_empty(),
                "unreviewed source row structure",
            )?;
            let source = origin(origins, *source_id)?;
            let gem = optional_link(source, |r| {
                if let OwnedOriginTarget::Gem(id) = r {
                    Some(*id)
                } else {
                    None
                }
            })?;
            let skill = optional_link(source, |r| {
                if let OwnedOriginTarget::Skill(id) = r {
                    Some(*id)
                } else {
                    None
                }
            })?;
            let support = optional_link(source, |r| {
                if let OwnedOriginTarget::Support(id) = r {
                    Some(*id)
                } else {
                    None
                }
            })?;
            let projected = optional_link(source, |r| {
                if let OwnedOriginTarget::GeneratedSkillInput {
                    skill_preset,
                    target,
                } = r
                {
                    Some((*skill_preset, target.clone()))
                } else {
                    None
                }
            })?;
            let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
                game_id: SourceComponent::Text(attr(row, "gemId")?.into()),
                variant_id: SourceComponent::Text(attr(row, "variantId")?.into()),
            });
            let Some(MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Gem(definition)),
                basis: MappingBasis::Exact,
            }) = release.mapping().lookup(&selector)
            else {
                return Err("unmapped catalog identity");
            };
            let role = release
                .roles()
                .role(definition)
                .ok_or("missing catalog role")?;
            let OwnedPrimarySkill::Known(primary) = &role.primary else {
                return Err("unresolved primary");
            };
            let OwnedGemSourceConstruction::Observed(facts) = &role.construction else {
                return Err("missing construction evidence");
            };
            ensure(
                facts.game_id == attr(row, "gemId")?
                    && facts.variant_id == attr(row, "variantId")?
                    && facts.primary.effect_id == attr(row, "skillId")?
                    && facts.primary.resolution
                        == OwnedSourceEffectResolution::Mapped(primary.clone()),
                "crossed primary source effect",
            )?;
            if let Some(id) = gem {
                ensure(gems.insert(id), "reused physical Gem occurrence")?;
                let instance = only(draft.gems.members.iter().filter(|r| r.id == id))?;
                ensure(
                    instance.definition.to_resolved().as_ref() == Some(definition),
                    "crossed physical Gem definition",
                )?;
            }
            let kind = if let Some((skill_preset, target)) = projected {
                ensure(
                    skill_preset == preset.id
                        && gem.is_none()
                        && skill.is_none()
                        && support.is_none()
                        && group.attribute("source").is_some(),
                    "generated input materialized as authored source",
                )?;
                ensure(
                    targets.insert(target.clone()),
                    "duplicate generated occurrence",
                )?;
                let binding = only(
                    intent
                        .generated_inputs
                        .members
                        .iter()
                        .filter(|r| r.target.to_resolved().as_ref() == Some(&target)),
                )?;
                ensure(
                    binding.applicability == PresetApplicability::WhenExactSourceSelected
                        && binding.parameters.completion == DraftListCompletion::Complete,
                    "unresolved exact generated input",
                )?;
                let parameter = only(binding.parameters.members.iter())?
                    .to_resolved()
                    .ok_or("unresolved saved generated quality")?;
                let quality = attr(row, "quality")?
                    .parse::<f64>()
                    .map_err(|_| "invalid saved generated quality")?;
                let (expected_slot, expected_unit) =
                    generated(evidence, draft, origins, selection, group, &target, primary)?;
                ensure(
                    parameter.slot == expected_slot
                        && matches!(&parameter.value, ParameterValue::Quantity(value) if value.value() == quality && value.unit() == &expected_unit),
                    "crossed saved generated quality",
                )?;
                let Some(SlotDescriptor::SkillGrant(supply)) =
                    schema.lookup_slot(&SkillGrantSlotDefId::address(&target.slot))
                else {
                    return Err("missing generated supply");
                };
                ensure(
                    matches!(&supply.schema, SchemaState::Known(s) if s.preset_inputs.as_ref().is_some_and(|p| p.parameters.members.contains(&parameter.slot))),
                    "unpermitted generated quality input",
                )?;
                let link = OwnedOriginTarget::GeneratedSkillInput {
                    skill_preset,
                    target: target.clone(),
                };
                ensure(
                    origin(origins, *group_id)?.links.contains(&link)
                        && origin(origins, set_id)?.links.contains(&link),
                    "missing generated input source corroboration",
                )?;
                Kind::Generated(Box::new(target))
            } else {
                ensure(
                    group.attribute("source").is_none(),
                    "generated source lost typed target",
                )?;
                match (&role.role, &role.materialization, gem, skill, support) {
                    (
                        OwnedGemRole::Known(AuthoredGemRole::SupportAssignment),
                        OwnedGemMaterialization::Physical,
                        Some(gem),
                        None,
                        Some(id),
                    ) => {
                        let assignment =
                            only(draft.supports.members.iter().filter(|r| r.id == id))?
                                .to_resolved()
                                .ok_or("unresolved support")?;
                        ensure(assignment.support == gem, "crossed support Gem")?;
                        group_supports.push(assignment);
                        supports.push(id);
                        Kind::Support(id)
                    }
                    (
                        OwnedGemRole::Known(AuthoredGemRole::SkillUse),
                        materialization,
                        gem,
                        Some(id),
                        None,
                    ) => {
                        let authored = only(draft.skills.members.iter().filter(|r| r.id == id))?
                            .to_resolved()
                            .ok_or("unresolved authored Skill")?;
                        let kind = match (materialization, gem, authored.source) {
                            (
                                OwnedGemMaterialization::Physical,
                                Some(gem),
                                AuthoredSkillSource::Gem(actual),
                            ) if gem == actual => Kind::Physical(id),
                            (
                                OwnedGemMaterialization::ProviderOnly,
                                None,
                                AuthoredSkillSource::Direct(actual),
                            ) if &actual == primary => {
                                let Some(DefinitionDescriptor::Skill(entry)) =
                                    schema.lookup_definition(&primary.address())
                                else {
                                    return Err("missing Direct definition");
                                };
                                ensure(
                                    matches!(&entry.schema, SchemaState::Known(s) if s.directly_selectable),
                                    "Direct source lacks permission",
                                )?;
                                Kind::Direct(id)
                            }
                            _ => return Err("crossed authored Skill source"),
                        };
                        group_skills.push(id);
                        skills.push(id);
                        kind
                    }
                    _ => return Err("unsupported source/link combination"),
                }
            };
            rows.push(Accounted {
                source: *source_id,
                group: *group_id,
                effect: attr(row, "skillId")?.into(),
                definition: primary.clone(),
                gem,
                kind,
            });
        }
        if !group_supports.is_empty() {
            let root = only(group_skills.iter().copied())?;
            for assignment in &group_supports {
                ensure(
                    assignment.target == SkillTarget::Authored(root),
                    "support crossed exact source group",
                )?;
            }
            ensure(
                expected_order
                    .insert(
                        SkillTarget::Authored(root),
                        group_supports.iter().map(|r| r.id).collect::<Vec<_>>(),
                    )
                    .is_none(),
                "reused group root",
            )?;
        }
    }
    ensure(
        skills == preset.skills.members && supports == preset.supports.members,
        "selected preset membership/order changed",
    )?;
    ensure(
        skills.iter().collect::<BTreeSet<_>>().len() == skills.len()
            && supports.iter().collect::<BTreeSet<_>>().len() == supports.len(),
        "duplicate selected occurrence",
    )?;
    ensure(
        targets.len() == intent.generated_inputs.members.len(),
        "extra generated input binding",
    )?;
    let order = preset
        .authored_support_order
        .as_ref()
        .ok_or("missing authored order")?;
    ensure(
        order.completion == DraftListCompletion::Complete,
        "unresolved authored order",
    )?;
    let mut actual_order = BTreeMap::new();
    for row in &order.members {
        let row = row.to_resolved().ok_or("unresolved authored order row")?;
        ensure(
            actual_order.insert(row.target, row.assignments).is_none(),
            "duplicate order target",
        )?;
    }
    ensure(actual_order == expected_order, "crossed support order")?;
    Ok(rows)
}

#[test]
fn selected_twenty_eight_sources_preserve_occurrences_and_exact_skill_origins() {
    super::gem_support_domains::check_authored();
    super::skeletal_support_source::check_proof();
    with_original(|evidence, normalized, selection| {
        let draft = normalized.draft();
        let snapshot = draft.clone();
        let origins = &normalized.sidecar().origins;
        let origin_snapshot = origins.clone();
        let rows = account(evidence, draft.input(), origins, &selection).unwrap();
        assert_eq!(
            attr(originals::selected_skill_set(evidence), "id").unwrap(),
            "4"
        );
        assert_eq!(rows.len(), 28);
        let mut counts = [0usize; 4];
        let mut repeated = BTreeMap::new();
        for row in &rows {
            counts[match row.kind {
                Kind::Physical(_) => 0,
                Kind::Support(_) => 1,
                Kind::Direct(_) => 2,
                Kind::Generated(_) => 3,
            }] += 1;
            if matches!(row.kind, Kind::Support(_)) {
                *repeated.entry(row.effect.as_str()).or_insert(0usize) += 1;
            }
        }
        assert_eq!(counts, [7, 16, 2, 3]);
        assert_eq!(
            rows.iter()
                .filter_map(|r| r.gem)
                .collect::<BTreeSet<_>>()
                .len(),
            23
        );
        assert_eq!(repeated["SupportLastGaspPlayer"], 3);
        assert_eq!(repeated["SupportBiddingPlayerTwo"], 2);
        assert_eq!(repeated["SupportMusterPlayer"], 2);
        let selected_definitions: BTreeSet<_> = rows
            .iter()
            .filter(|r| matches!(r.kind, Kind::Physical(_)))
            .map(|r| r.definition.address())
            .collect();
        for domain in super::skeletal_support_domains::declarations() {
            if let SchemaSubject::Definition(owner @ DefinitionAddress::Skill(_)) = domain.owner {
                assert!(selected_definitions.contains(&owner));
            }
        }
        let recipe = tracked_release().release().assembled().checked();
        assert!(tracked_release().release().evaluation().is_none());
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
        assert_eq!(*origins, origin_snapshot);
    });
}

#[test]
fn missing_duplicate_crossed_and_generated_as_authored_sources_are_refused() {
    with_original(|evidence, normalized, selection| {
        let draft = normalized.draft().input();
        let origins = &normalized.sidecar().origins;
        let baseline = account(evidence, draft, origins, &selection).unwrap();
        let physical = baseline
            .iter()
            .find(|r| matches!(r.kind, Kind::Physical(_)))
            .unwrap();
        let supports: Vec<_> = baseline
            .iter()
            .filter(|r| matches!(r.kind, Kind::Support(_)))
            .collect();
        let generated = baseline
            .iter()
            .find(|r| matches!(r.kind, Kind::Generated(_)))
            .unwrap();
        for control in 0..8 {
            let mut changed = origins.clone();
            let position = changed
                .iter()
                .position(|r| r.source == physical.source)
                .unwrap();
            match control {
                0 => {
                    changed.remove(position);
                }
                1 => changed.push(changed[position].clone()),
                2 => {
                    let link = changed[position]
                        .links
                        .iter()
                        .find(|r| matches!(r, OwnedOriginTarget::Gem(_)))
                        .unwrap()
                        .clone();
                    changed[position].links.push(link);
                }
                3 => {
                    let gem = supports[0].gem.unwrap();
                    for link in &mut changed[position].links {
                        if matches!(link, OwnedOriginTarget::Gem(_)) {
                            *link = OwnedOriginTarget::Gem(gem);
                        }
                    }
                }
                4 => {
                    let source = origin(origins, supports[0].source).unwrap().clone();
                    let target = changed
                        .iter_mut()
                        .find(|r| r.source == supports[1].source)
                        .unwrap();
                    target.links = source.links;
                }
                5 => {
                    let target = changed
                        .iter_mut()
                        .find(|r| r.source == generated.source)
                        .unwrap();
                    target
                        .links
                        .push(OwnedOriginTarget::Gem(physical.gem.unwrap()));
                }
                6 => {
                    let target = changed
                        .iter_mut()
                        .find(|r| r.source == generated.source)
                        .unwrap();
                    target
                        .links
                        .retain(|r| !matches!(r, OwnedOriginTarget::GeneratedSkillInput { .. }));
                }
                7 => {
                    let target = changed
                        .iter_mut()
                        .find(|r| r.source == generated.group)
                        .unwrap();
                    target
                        .links
                        .retain(|r| !matches!(r, OwnedOriginTarget::GeneratedSkillInput { .. }));
                }
                _ => unreachable!(),
            }
            assert!(
                account(evidence, draft, &changed, &selection).is_err(),
                "source control {control}"
            );
        }
        for control in 0..6 {
            let mut changed = draft.clone();
            let Kind::Support(support_id) = supports[0].kind else {
                unreachable!()
            };
            match control {
                0 => {
                    let row = changed
                        .supports
                        .members
                        .iter_mut()
                        .find(|r| r.id == support_id)
                        .unwrap();
                    // First support belongs to the first physical skill. Choose a different exact root.
                    let other = baseline
                        .iter()
                        .find_map(|r| {
                            if let Kind::Physical(id) = r.kind {
                                (r.source != physical.source).then_some(id)
                            } else {
                                None
                            }
                        })
                        .unwrap();
                    row.target = SkillTarget::Authored(other).into();
                }
                1 => {
                    let preset = changed
                        .skill_presets
                        .members
                        .iter_mut()
                        .find(|r| r.id == selection.build.skills)
                        .unwrap();
                    preset.supports.members.push(support_id);
                }
                2 => {
                    let preset = changed
                        .skill_presets
                        .members
                        .iter_mut()
                        .find(|r| r.id == selection.build.skills)
                        .unwrap();
                    preset.skills.members.swap(0, 1);
                }
                3 => {
                    let preset = changed
                        .skill_presets
                        .members
                        .iter_mut()
                        .find(|r| r.id == selection.build.skills)
                        .unwrap();
                    let bindings = &mut preset.intent.as_mut().unwrap().generated_inputs.members;
                    bindings.push(bindings[0].clone());
                }
                4 => {
                    let preset = changed
                        .skill_presets
                        .members
                        .iter_mut()
                        .find(|r| r.id == selection.build.skills)
                        .unwrap();
                    preset.intent.as_mut().unwrap().generated_inputs.completion =
                        preset.intent.as_ref().unwrap().usage.completion.clone();
                }
                5 => {
                    let mut crossed = selection;
                    crossed.build.skills = only(
                        changed
                            .skill_presets
                            .members
                            .iter()
                            .filter(|r| r.id != selection.build.skills)
                            .map(|r| r.id)
                            .take(1),
                    )
                    .unwrap();
                    assert!(account(evidence, &changed, origins, &crossed).is_err());
                    continue;
                }
                _ => unreachable!(),
            }
            assert!(
                account(evidence, &changed, origins, &selection).is_err(),
                "draft control {control}"
            );
        }
        // Change every target reference consistently to another selected physical
        // allocation of the same Skill's passive. Shape and Skill equality alone
        // must not turn an ambiguous saved Tree selector into occurrence authority.
        let Kind::Generated(old) = &generated.kind else {
            unreachable!()
        };
        let old = old.as_ref();
        let ProviderRoot::Allocation(old_id) = old.provider.root else {
            unreachable!()
        };
        let mut changed = draft.clone();
        let preset = changed
            .allocation_presets
            .members
            .iter()
            .find(|r| r.id == selection.build.allocations)
            .unwrap();
        let other = *preset
            .allocations
            .members
            .iter()
            .find(|id| **id != old_id)
            .unwrap();
        let node = changed
            .allocations
            .members
            .iter()
            .find(|r| r.id == old_id)
            .unwrap()
            .node
            .clone();
        changed
            .allocations
            .members
            .iter_mut()
            .find(|r| r.id == other)
            .unwrap()
            .node = node;
        let mut replacement = old.clone();
        replacement.provider.root = ProviderRoot::Allocation(other);
        let preset = changed
            .skill_presets
            .members
            .iter_mut()
            .find(|r| r.id == selection.build.skills)
            .unwrap();
        for binding in &mut preset.intent.as_mut().unwrap().generated_inputs.members {
            if binding.target.to_resolved().as_ref() == Some(old) {
                binding.target = replacement.clone().into();
            }
        }
        let mut changed_origins = origins.clone();
        for origin in &mut changed_origins {
            for link in &mut origin.links {
                if let OwnedOriginTarget::GeneratedSkillInput {
                    skill_preset,
                    target,
                } = link
                    && *skill_preset == selection.build.skills
                    && target == old
                {
                    *target = replacement.clone();
                }
            }
        }
        assert_eq!(
            account(evidence, &changed, &changed_origins, &selection),
            Err("ambiguous generated allocation occurrence")
        );
        assert_eq!(
            account(evidence, draft, origins, &selection).unwrap(),
            baseline
        );
    });
}
