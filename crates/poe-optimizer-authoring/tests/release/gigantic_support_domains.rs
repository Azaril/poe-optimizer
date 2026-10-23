//! Current Boolean and scalar laws, independent of historical publisher machinery.
use super::{gigantic_support_source as source, originals, tracked_release};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_draft::DraftFinalization,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{EffectDisposition, RuleFact};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_mapping::{
        ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, SourceComponent,
    },
    owned_normalize::{NormalizationArtifacts, normalize_fresh},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{fs, path::Path};

fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn binding() -> &'static Value {
    source::artifact("gigantic-following/bindings.json")
}
fn passive() -> PassiveNodeDefId {
    typed(&binding()["node"]["definition"])
}
fn stat(name: &str) -> StatDefId {
    typed(&binding()["channels"][name])
}
fn subject() -> SchemaSubject {
    SchemaSubject::Definition(passive().address())
}
fn selector() -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode {
        tree_version: SourceComponent::Text("0_5".into()),
        node_id: SourceComponent::Text("46365".into()),
        view: SourceComponent::Missing,
    })
}
pub(super) fn declarations() -> Vec<SupportSourceDomainDeclaration> {
    assert_eq!(passive().key().as_str(), "def.0000000000001532");
    let mapping = source::artifact("current/mapping.json");
    let rows: Vec<poe_optimizer_import::owned_mapping::MappingEntry> = typed(&mapping["entries"]);
    let exact: Vec<_> = rows.iter().filter(|row| row.source == selector()).collect();
    assert_eq!(exact.len(), 1);
    assert_eq!(
        exact[0].outcome,
        MappingOutcome::Mapped {
            target: subject(),
            basis: MappingBasis::Exact
        }
    );
    vec![SupportSourceDomainDeclaration {
        owner: subject(),
        domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
    }]
}
fn owner(subject: &SchemaSubject) -> &'static DefinitionRules {
    tracked_release()
        .release()
        .assembled()
        .checked()
        .rules()
        .input()
        .owners
        .iter()
        .find(|row| &row.owner == subject)
        .unwrap()
}
fn empty<T>(value: &DeclaredSet<T>) -> bool {
    value.is_complete() && value.members.is_empty()
}
fn literal(
    program: &RuleProgram,
    stat: &StatDefId,
    value: &ParameterValue,
    contribution: ContributionKind,
) -> bool {
    if program.context != RuleEntityKind::Actor
        || !program.reads.is_empty()
        || program.nodes.len() != 1
        || program.effects.len() != 1
    {
        return false;
    }
    if program.nodes[0].expression
        != (RuleExpression::Literal {
            value: value.clone(),
        })
        || program.effects[0].when.is_some()
    {
        return false;
    }
    matches!(&program.effects[0].effect, RuleEffectKind::Contribute { entity: RuleEntity::Player, stat: target, contribution: kind, value } if target == stat && *kind == contribution && value == &program.nodes[0].id)
}
fn producer_family(
    schema: &DefinitionEntry<PassiveNodeDefId, PassiveNodeSchema>,
    body: &DefinitionRules,
) -> bool {
    let SchemaState::Known(schema_value) = &schema.schema else {
        return false;
    };
    let d = &schema_value.declarations;
    if schema.id != passive()
        || body.owner != subject()
        || !body.programs.is_complete()
        || body.programs.members.len() != 2
        || !empty(&d.parameters)
        || !empty(&d.choices)
        || !empty(&d.grants)
        || !empty(&d.actors)
        || !empty(&d.skill_grants)
        || !empty(&d.outputs)
        || !empty(&d.sockets)
    {
        return false;
    }
    let unit: UnitDefId = typed(&binding()["percent_unit"]);
    let reservation = ParameterValue::Quantity(FiniteQuantity::new(-25.0, unit).unwrap());
    body.programs
        .members
        .iter()
        .filter(|p| {
            literal(
                p,
                &stat("grants"),
                &ParameterValue::Boolean(true),
                ContributionKind::Flag,
            )
        })
        .count()
        == 1
        && body
            .programs
            .members
            .iter()
            .filter(|p| {
                literal(
                    p,
                    &stat("reservation_efficiency"),
                    &reservation,
                    ContributionKind::Increase,
                )
            })
            .count()
            == 1
}
fn passive_schema() -> &'static DefinitionEntry<PassiveNodeDefId, PassiveNodeSchema> {
    let Some(DefinitionDescriptor::PassiveNode(value)) = tracked_release()
        .release()
        .assembled()
        .checked()
        .schema()
        .lookup_definition(&passive().address())
    else {
        panic!("passive schema")
    };
    value
}

#[test]
fn intrinsic_domain_preserves_boolean_grant_separate_reservation_and_topology() {
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    let declarations = declarations();
    assert_eq!(
        release.mapping().lookup(&selector()),
        Some(&MappingOutcome::Mapped {
            target: subject(),
            basis: MappingBasis::Exact
        })
    );
    assert_eq!(
        recipe
            .rules()
            .input()
            .support_discovery
            .as_ref()
            .unwrap()
            .providers
            .iter()
            .filter(|row| row.owner == subject())
            .collect::<Vec<_>>(),
        [&declarations[0]]
    );
    assert!(producer_family(passive_schema(), owner(&subject())));
    let historical: DefinitionDescriptor =
        typed(&source::artifact("gigantic-following/closure.json")["definitions"][0]);
    assert_eq!(
        recipe.schema().lookup_definition(&passive().address()),
        Some(&historical)
    );
    let current_schema: DefinitionDescriptor =
        typed(&source::artifact("gigantic-flags/schemas.json")[0]);
    assert_eq!(
        recipe.schema().lookup_definition(&stat("grants").address()),
        Some(&current_schema)
    );
    assert_eq!(stat("grants").key().as_str(), "def.0000000000003307");
    let mut scratch = recipe.compiled_rules().new_scratch();
    for p in &owner(&subject()).programs.members {
        let result = recipe
            .compiled_rules()
            .evaluate(
                &subject(),
                &p.id,
                &[],
                recipe.schema().as_ref(),
                &mut scratch,
            )
            .unwrap();
        assert_eq!(result.owner_programs_closure, SchemaClosure::Complete);
        assert_eq!(result.effects.len(), 1);
        let RuleExpression::Literal { value } = &p.nodes[0].expression else {
            unreachable!()
        };
        assert_eq!(
            result.effects[0].disposition,
            EffectDisposition::Applied {
                value: value.clone()
            }
        );
    }
    assert!(release.evaluation().is_none());
}

#[test]
fn numeric_presence_wrong_authority_partial_inventory_and_missing_paired_effect_fail_proof() {
    let schema = passive_schema();
    let original = owner(&subject());
    assert!(producer_family(schema, original));
    let mut renamed = original.clone();
    for (i, p) in renamed.programs.members.iter_mut().enumerate() {
        p.id = format!("renamed-{i}").parse().unwrap();
        p.effects[0].id = "renamed-effect".parse().unwrap();
    }
    assert!(producer_family(schema, &renamed));
    for control in 0..5 {
        let mut body = original.clone();
        let flag = body
            .programs
            .members
            .iter_mut()
            .find(|p| {
                matches!(
                    p.nodes[0].expression,
                    RuleExpression::Literal {
                        value: ParameterValue::Boolean(true)
                    }
                )
            })
            .unwrap();
        let flag_id = flag.id.clone();
        match control {
            0 => {
                flag.nodes[0].expression = RuleExpression::Literal {
                    value: ParameterValue::Integer(BoundedInteger::new(1).unwrap()),
                }
            }
            1 => {
                if let RuleEffectKind::Contribute { entity, .. } = &mut flag.effects[0].effect {
                    *entity = RuleEntity::Current;
                }
            }
            2 => {
                if let RuleEffectKind::Contribute { contribution, .. } = &mut flag.effects[0].effect
                {
                    *contribution = ContributionKind::Add;
                }
            }
            3 => body.programs.members.retain(|p| p.id == flag_id),
            4 => {
                body.programs.closure = SchemaClosure::Partial {
                    gaps: vec![SchemaGap {
                        subject: subject(),
                        facet: SchemaFacet::GameRules,
                        code: "unproved".parse().unwrap(),
                    }],
                }
            }
            _ => unreachable!(),
        }
        assert!(!producer_family(schema, &body), "control {control}");
    }
}

#[test]
fn current_flag_query_keeps_exact_allocation_provenance_and_incomplete_membership() {
    let recipe = tracked_release().release().assembled().checked();
    let expected: Vec<ContributionQuery> = typed(source::artifact("gigantic-flags/queries.json"));
    assert_eq!(expected.len(), 1);
    let current = recipe
        .rules()
        .input()
        .contribution_queries
        .as_ref()
        .unwrap()
        .members
        .iter()
        .find(|query| query.id == expected[0].id)
        .unwrap();
    assert_eq!(current, &expected[0]);
    assert_eq!(current.stat, stat("grants"));
    assert_eq!(current.contribution, ContributionKind::Flag);
    let group = &current.groups[0];
    assert_eq!(current.groups.len(), 1);
    assert_eq!(group.reduction, ContributionReduction::Any);
    assert_eq!(group.ordering, ContributionOrdering::Unordered);
    assert_eq!(group.empty, Some(ParameterValue::Boolean(false)));
    assert!(
        !group.members.is_complete(),
        "intrinsic support absence cannot close the Gigantic producer census"
    );
    assert_eq!(group.members.members.len(), 1);
    let member = &group.members.members[0];
    assert!(member.order.is_none());
    let ContributionProducer::ProgramEffect(producer) = &member.producer else {
        panic!("ordinary allocation producer")
    };
    assert_eq!(producer.owner, subject());
    assert_eq!(producer.origin, ContributionOrigin::Allocation);
    assert_eq!(producer.program.as_str(), "ordinary-minion-gigantic");
    assert_eq!(producer.effect.as_str(), "grant");
    let receiver: StatReceiver = typed(&binding()["receiver"]);
    assert_eq!(
        recipe
            .rules()
            .input()
            .receivers
            .members
            .iter()
            .find(|row| row.id == receiver.id),
        Some(&receiver)
    );
    let body = owner(&SchemaSubject::Definition(stat("active").address()));
    assert!(body.programs.is_complete());
    assert_eq!(body.programs.members.len(), 1);
    let p = &body.programs.members[0];
    assert_eq!(p.reads.len(), 1);
    assert_eq!(p.nodes.len(), 1);
    assert_eq!(p.effects.len(), 1);
    assert_eq!(p.reads[0].value_type, ComputedValueType::Boolean);
    assert_eq!(
        p.reads[0].source,
        RuleReadSource::ContributionQuery {
            entity: RuleEntity::Player,
            query: current.id.clone(),
            group: group.id.clone()
        }
    );
    assert!(
        matches!(&p.nodes[0].expression, RuleExpression::Read { input } if input == &p.reads[0].id)
    );
    assert!(p.effects[0].when.is_none());
    assert!(
        matches!(&p.effects[0].effect, RuleEffectKind::Derive { entity: RuleEntity::Current, stat: target, value } if target == &stat("active") && value == &p.nodes[0].id)
    );
    let mut scratch = recipe.compiled_rules().new_scratch();
    // These are explicit component inputs, not a claim that Partial query membership is executable.
    for active in [Some(true), Some(false), None, Some(true)] {
        let facts: Vec<_> = active
            .into_iter()
            .map(|flag| RuleFact {
                read: p.reads[0].id.clone(),
                value: ParameterValue::Boolean(flag),
            })
            .collect();
        let result = recipe
            .compiled_rules()
            .evaluate(
                &body.owner,
                &p.id,
                &facts,
                recipe.schema().as_ref(),
                &mut scratch,
            )
            .unwrap();
        match active {
            Some(flag) => assert_eq!(
                result.effects[0].disposition,
                EffectDisposition::Applied {
                    value: ParameterValue::Boolean(flag)
                }
            ),
            None => assert!(
                matches!(&result.effects[0].disposition, EffectDisposition::Unresolved { input } if input == &p.reads[0].id)
            ),
        }
    }
}

#[test]
fn current_benefits_share_boolean_input_and_write_canonical_life_and_damage_once() {
    let recipe = tracked_release().release().assembled().checked();
    let b = source::artifact("gigantic-benefits/bindings.json");
    let slot: DeclaredSlot<ActorSlotDefId> = typed(&b["actor_slot"]);
    let body = owner(&SchemaSubject::Slot(SlotAddress::Actor(slot)));
    assert!(!body.programs.is_complete());
    let p = body
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == "gigantic-life-and-damage")
        .unwrap();
    assert_eq!(p.context, RuleEntityKind::Actor);
    assert_eq!(p.reads.len(), 1);
    assert_eq!(p.nodes.len(), 2);
    assert_eq!(p.effects.len(), 2);
    assert_eq!(
        p.reads[0].source,
        RuleReadSource::Stat {
            entity: RuleEntity::Current,
            stat: stat("active")
        }
    );
    assert_eq!(p.reads[0].value_type, ComputedValueType::Boolean);
    let read = p
        .nodes
        .iter()
        .find(
            |n| matches!(&n.expression, RuleExpression::Read { input } if input == &p.reads[0].id),
        )
        .unwrap();
    let factor = p
        .nodes
        .iter()
        .find(|n| matches!(&n.expression, RuleExpression::Literal { .. }))
        .unwrap();
    let amount =
        ParameterValue::Quantity(FiniteQuantity::new(1.2, typed(&b["factor_unit"])).unwrap());
    assert_eq!(
        factor.expression,
        RuleExpression::Literal {
            value: amount.clone()
        }
    );
    let replacement = source::artifact("minion-life-source/replacement.json");
    let life: StatDefId = typed(&replacement["after"]["effects"][0]["effect"]["stat"]);
    assert_eq!(life.key().as_str(), "def.000000000000311a");
    let expected = [life, typed(&b["channels"]["damage_more"])];
    for (effect, destination) in p.effects.iter().zip(&expected) {
        assert_eq!(effect.when.as_ref(), Some(&read.id));
        assert!(
            matches!(&effect.effect, RuleEffectKind::Contribute { entity: RuleEntity::Current, stat, contribution: ContributionKind::Multiply, value } if stat == destination && value == &factor.id)
        );
    }
    let retired: StatDefId = typed(&b["channels"]["life_more"]);
    assert!(recipe.rules().input().owners.iter().flat_map(|owner| &owner.programs.members)
        .flat_map(|program| &program.effects).all(|effect| !matches!(&effect.effect,
            RuleEffectKind::Contribute { stat, .. } | RuleEffectKind::Derive { stat, .. } if stat == &retired)),
        "retained330a identity has no duplicate live writer");
    let mut scratch = recipe.compiled_rules().new_scratch();
    for active in [Some(true), Some(false), None, Some(true)] {
        let facts: Vec<_> = active
            .into_iter()
            .map(|flag| RuleFact {
                read: p.reads[0].id.clone(),
                value: ParameterValue::Boolean(flag),
            })
            .collect();
        let result = recipe
            .compiled_rules()
            .evaluate(
                &body.owner,
                &p.id,
                &facts,
                recipe.schema().as_ref(),
                &mut scratch,
            )
            .unwrap();
        assert_eq!(result.effects.len(), 2);
        for effect in &result.effects {
            match active {
                Some(true) => assert_eq!(
                    effect.disposition,
                    EffectDisposition::Applied {
                        value: amount.clone()
                    }
                ),
                Some(false) => assert_eq!(effect.disposition, EffectDisposition::Inactive),
                None => assert!(
                    matches!(&effect.disposition, EffectDisposition::Unresolved { input } if input == &p.reads[0].id)
                ),
            }
        }
    }
}

#[test]
fn unchanged_saved_sniper_selects_gigantic_and_keeps_all_pending_obligations() {
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
    let queries = release
        .query_sets()
        .iter()
        .find(|set| set.name.as_str() == "original-05")
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
        &queries.queries,
        Default::default(),
    )
    .unwrap();
    let selection = originals::saved(&evidence, &normalized);
    let draft = normalized.draft();
    let snapshot = draft.clone();
    let selected = &draft
        .input()
        .allocation_presets
        .members
        .iter()
        .find(|preset| preset.id == selection.build.allocations)
        .unwrap()
        .allocations
        .members;
    assert_eq!(
        draft
            .input()
            .allocations
            .members
            .iter()
            .filter(|a| selected.contains(&a.id) && a.node.to_resolved() == Some(passive()))
            .count(),
        1
    );
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
    assert_eq!(draft, &snapshot);
    assert_eq!(checked.draft_digest(), normalized.sidecar().draft);
}
