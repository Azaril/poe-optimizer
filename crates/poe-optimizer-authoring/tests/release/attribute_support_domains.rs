//! Complete attribute-choice laws over the maintained release. Offline source
//! proofs and exact retained mappings establish the family; no build is a default.
use super::{attribute_support_source as source, originals, tracked_release};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_draft::DraftFinalization,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{EffectDisposition, RuleFact, RuleScratch};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_mapping::{
        ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingEntry, MappingOutcome,
        SourceComponent,
    },
    owned_normalize::{NormalizationArtifacts, normalize_fresh},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_tree_catalog::TreeNodeKind,
    owned_tree_policy::{TreeAttributeRule, TreeTokenRole},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

fn selector(node: &str) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode {
        tree_version: SourceComponent::Text(source::verified_catalog().tree_version.clone()),
        node_id: SourceComponent::Text(node.into()),
        view: SourceComponent::Missing,
    })
}
fn exact_node<'a>(
    entries: &'a [MappingEntry],
    selector: &ExternalSelector,
) -> Option<&'a PassiveNodeDefId> {
    let mut matches = entries.iter().filter(|entry| &entry.source == selector);
    let MappingOutcome::Mapped {
        target: SchemaSubject::Definition(DefinitionAddress::PassiveNode(node)),
        basis: MappingBasis::Exact,
    } = &matches.next()?.outcome
    else {
        return None;
    };
    matches.next().is_none().then_some(node)
}
fn attributes() -> Vec<&'static TreeAttributeRule> {
    let catalog = source::verified_catalog();
    let tree = &source::verified_tree().content;
    assert_eq!(tree.tree_version, catalog.tree_version);
    assert_eq!(tree.source, catalog.source);
    let mut result = Vec::new();
    let mut unique = BTreeSet::new();
    for node in &catalog.nodes {
        if !matches!(node.kind, TreeNodeKind::Attribute { .. }) {
            continue;
        }
        let owner = exact_node(&source::verified_mapping().entries, &selector(&node.key)).unwrap();
        assert!(unique.insert(owner.clone()));
        let token = tree
            .tokens
            .iter()
            .find(|row| row.token == node.key)
            .unwrap();
        assert!(matches!(&token.role, TreeTokenRole::Allocation { node, .. } if node == owner));
        let mut matches = tree.attributes.iter().filter(|row| &row.node == owner);
        let attribute = matches.next().unwrap();
        assert!(matches.next().is_none());
        result.push(attribute);
    }
    assert_eq!(result.len(), 293);
    assert_eq!(tree.attributes.len(), 293);
    result.sort_by(|left, right| left.node.cmp(&right.node));
    result
}
pub(super) fn declarations() -> Vec<SupportSourceDomainDeclaration> {
    attributes()
        .into_iter()
        .map(|attribute| SupportSourceDomainDeclaration {
            owner: SchemaSubject::Definition(attribute.node.address()),
            domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
        })
        .collect()
}
struct Channels {
    unit: UnitDefId,
    by_lane: BTreeMap<String, [StatDefId; 2]>,
}
fn channels() -> Channels {
    let bindings = source::verified_count_bindings();
    let unit = serde_json::from_value(bindings["unit"].clone()).unwrap();
    let mut by_lane = BTreeMap::new();
    assert_eq!(bindings["passes"].as_array().unwrap().len(), 2);
    for (pass, name) in ["first", "second"].into_iter().enumerate() {
        assert_eq!(bindings["passes"][pass]["id"], name);
        assert_eq!(
            bindings["passes"][pass]["inputs"].as_array().unwrap().len(),
            3
        );
    }
    for (index, lane) in ["strNodes", "dexNodes", "intNodes"].into_iter().enumerate() {
        by_lane.insert(
            lane.into(),
            [0, 1].map(|pass| {
                serde_json::from_value(bindings["passes"][pass]["inputs"][index].clone()).unwrap()
            }),
        );
    }
    Channels { unit, by_lane }
}
fn complete_empty<T>(set: &DeclaredSet<T>) -> bool {
    set.is_complete() && set.members.is_empty()
}
fn node<'a>(program: &'a RuleProgram, id: &OwnedDefinitionKey) -> Option<&'a RuleExpression> {
    program
        .nodes
        .iter()
        .find(|node| &node.id == id)
        .map(|node| &node.expression)
}
fn guarded_option<'a>(
    program: &'a RuleProgram,
    read: &OwnedDefinitionKey,
    guard: &OwnedDefinitionKey,
) -> Option<&'a OptionDefId> {
    let RuleExpression::Compare {
        operation: RuleComparison::Equal,
        left,
        right,
    } = node(program, guard)?
    else {
        return None;
    };
    for (read_node, option_node) in [(left, right), (right, left)] {
        if matches!(node(program, read_node), Some(RuleExpression::Read { input }) if input == read)
            && let Some(RuleExpression::Literal {
                value: ParameterValue::Option(option),
            }) = node(program, option_node)
        {
            return Some(option);
        }
    }
    None
}
/// This finite structural check does not inspect names or infer source absence
/// from an empty member list. Every relevant inventory must be complete, and the
/// authenticated source review remains an independent prerequisite.
fn attribute_family(
    definition: &DefinitionEntry<PassiveNodeDefId, PassiveNodeSchema>,
    choice: &DefinitionEntry<DeclaredSlot<ChoiceSlotDefId>, ChoiceSlotSchema>,
    attribute: &TreeAttributeRule,
    rules: &DefinitionRules,
    channels: &Channels,
) -> bool {
    let (SchemaState::Known(schema), SchemaState::Known(choice_schema)) =
        (&definition.schema, &choice.schema)
    else {
        return false;
    };
    let slots = &schema.declarations;
    if definition.id != attribute.node
        || choice.id != attribute.slot
        || attribute.slot.declaration != SlotOwnerDefId::PassiveNode(attribute.node.clone())
        || rules.owner != SchemaSubject::Definition(attribute.node.address())
        || !rules.programs.is_complete()
        || rules.programs.members.len() != 1
        || !slots.choices.is_complete()
        || slots.choices.members != [attribute.slot.clone()]
        || !complete_empty(&slots.parameters)
        || !complete_empty(&slots.grants)
        || !complete_empty(&slots.actors)
        || !complete_empty(&slots.skill_grants)
        || !complete_empty(&slots.outputs)
        || !complete_empty(&slots.sockets)
        || choice_schema.presence != SlotPresence::RequiredOnce
        || choice_schema.owners != [ChoiceOwnerScope::Allocation]
        || attribute.lanes.len() != 3
    {
        return false;
    }
    let ValueSchema::Option { allowed } = &choice_schema.value else {
        return false;
    };
    let options: BTreeSet<_> = attribute.lanes.iter().map(|lane| &lane.option).collect();
    let lanes: BTreeSet<_> = attribute.lanes.iter().map(|lane| &lane.attribute).collect();
    if !allowed.is_complete()
        || allowed.members.len() != 3
        || options.len() != 3
        || options != allowed.members.iter().collect()
        || lanes != channels.by_lane.keys().collect()
    {
        return false;
    }
    let program = &rules.programs.members[0];
    if program.context != RuleEntityKind::Actor
        || program.reads.len() != 1
        || program.effects.len() != 6
    {
        return false;
    }
    let read = &program.reads[0];
    if read.value_type != ComputedValueType::Option
        || !matches!(&read.source, RuleReadSource::Choice { slot } if slot == &attribute.slot)
    {
        return false;
    }
    for current in &program.nodes {
        let admitted = match &current.expression {
            RuleExpression::Read { input } => input == &read.id,
            RuleExpression::Literal {
                value: ParameterValue::Option(option),
            } => options.contains(option),
            RuleExpression::Literal {
                value: ParameterValue::Quantity(amount),
            } => amount.value() == 5.0 && amount.unit() == &channels.unit,
            RuleExpression::Compare { .. } => guarded_option(program, &read.id, &current.id)
                .is_some_and(|option| options.contains(option)),
            _ => false,
        };
        if !admitted {
            return false;
        }
    }
    let mut expected: BTreeSet<_> = attribute
        .lanes
        .iter()
        .flat_map(|lane| {
            channels.by_lane[&lane.attribute]
                .iter()
                .map(|stat| (&lane.option, stat))
        })
        .collect();
    for effect in &program.effects {
        let RuleEffectKind::Contribute {
            entity: RuleEntity::Player,
            stat,
            contribution: ContributionKind::Add,
            value,
        } = &effect.effect
        else {
            return false;
        };
        let Some(option) = effect
            .when
            .as_ref()
            .and_then(|guard| guarded_option(program, &read.id, guard))
        else {
            return false;
        };
        if !expected.remove(&(option, stat))
            || !matches!(node(program, value), Some(RuleExpression::Literal { value: ParameterValue::Quantity(amount) }) if amount.value() == 5.0 && amount.unit() == &channels.unit)
        {
            return false;
        }
    }
    expected.is_empty()
}
fn entries(
    attribute: &TreeAttributeRule,
) -> (
    &'static DefinitionEntry<PassiveNodeDefId, PassiveNodeSchema>,
    &'static DefinitionEntry<DeclaredSlot<ChoiceSlotDefId>, ChoiceSlotSchema>,
    &'static DefinitionRules,
) {
    let recipe = tracked_release().release().assembled().checked();
    let owner = SchemaSubject::Definition(attribute.node.address());
    let Some(DefinitionDescriptor::PassiveNode(definition)) =
        recipe.schema().lookup_definition(&attribute.node.address())
    else {
        panic!("passive definition")
    };
    let Some(SlotDescriptor::Choice(choice)) = recipe
        .schema()
        .lookup_slot(&SlotAddress::Choice(attribute.slot.clone()))
    else {
        panic!("exact choice")
    };
    let rules = recipe
        .rules()
        .input()
        .owners
        .iter()
        .find(|row| row.owner == owner)
        .unwrap();
    (definition, choice, rules)
}
fn assert_projection(
    attribute: &TreeAttributeRule,
    option: &OptionDefId,
    channels: &Channels,
    scratch: &mut RuleScratch,
) {
    let recipe = tracked_release().release().assembled().checked();
    let (_, _, rules) = entries(attribute);
    let program = &rules.programs.members[0];
    let result = recipe
        .compiled_rules()
        .evaluate(
            &rules.owner,
            &program.id,
            &[RuleFact {
                read: program.reads[0].id.clone(),
                value: ParameterValue::Option(option.clone()),
            }],
            recipe.schema().as_ref(),
            scratch,
        )
        .unwrap();
    assert_eq!(result.effects.len(), 6);
    assert!(matches!(
        result.owner_programs_closure,
        SchemaClosure::Complete
    ));
    let mut active = BTreeSet::new();
    let mut inactive = 0;
    for effect in &result.effects {
        match &effect.disposition {
            EffectDisposition::Applied {
                value: ParameterValue::Quantity(amount),
            } => {
                assert_eq!(amount.value(), 5.0);
                assert_eq!(amount.unit(), &channels.unit);
                let RuleEffectKind::Contribute {
                    entity: RuleEntity::Player,
                    stat,
                    contribution: ContributionKind::Add,
                    ..
                } = &effect.effect
                else {
                    panic!("Player Count add")
                };
                assert!(active.insert(stat.clone()));
            }
            EffectDisposition::Inactive => inactive += 1,
            other => panic!("unexpected projection {other:?}"),
        }
    }
    let lane = attribute
        .lanes
        .iter()
        .find(|lane| &lane.option == option)
        .unwrap();
    assert_eq!(
        active,
        channels.by_lane[&lane.attribute].iter().cloned().collect()
    );
    assert_eq!(inactive, 4);
}

#[test]
fn all_catalog_attribute_choices_keep_exact_schema_and_six_count_channels() {
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    let channels = channels();
    assert_eq!(channels.unit.key().as_str(), "def.000000000000295a");
    assert_eq!(
        channels
            .by_lane
            .values()
            .flatten()
            .map(|id| id.key().as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "def.000000000000331b",
            "def.000000000000331c",
            "def.000000000000331d",
            "def.000000000000331e",
            "def.000000000000331f",
            "def.0000000000003320",
        ])
    );
    for stat in channels.by_lane.values().flatten() {
        let SchemaLookup::Known(schema) = recipe.schema().definition(stat) else {
            panic!("known Count channel")
        };
        assert_eq!(
            schema.value,
            ComputedValueType::Quantity {
                unit: channels.unit.clone()
            }
        );
        assert_eq!(schema.targets, [RuleEntityKind::Actor]);
    }
    for (attribute, domain) in attributes().into_iter().zip(declarations()) {
        let (definition, choice, rules) = entries(attribute);
        assert!(attribute_family(
            definition, choice, attribute, rules, &channels
        ));
        assert_eq!(
            recipe
                .rules()
                .input()
                .support_discovery
                .as_ref()
                .unwrap()
                .providers
                .iter()
                .filter(|row| row.owner == domain.owner)
                .collect::<Vec<_>>(),
            [&domain]
        );
        assert_eq!(
            release
                .tree()
                .unwrap()
                .input()
                .content
                .attributes
                .iter()
                .find(|row| row.node == attribute.node),
            Some(attribute)
        );
    }
    for node in &source::verified_catalog().nodes {
        if matches!(node.kind, TreeNodeKind::Attribute { .. }) {
            assert_eq!(
                release.mapping().lookup(&selector(&node.key)),
                source::verified_mapping()
                    .entries
                    .iter()
                    .find(|row| row.source == selector(&node.key))
                    .map(|row| &row.outcome)
            );
        }
    }
    assert!(release.evaluation().is_none());
}

#[test]
fn every_attribute_option_executes_and_missing_choice_remains_unresolved() {
    let recipe = tracked_release().release().assembled().checked();
    let channels = channels();
    let mut scratch = recipe.compiled_rules().new_scratch();
    let mut executed = 0;
    for attribute in attributes() {
        for lane in &attribute.lanes {
            assert_projection(attribute, &lane.option, &channels, &mut scratch);
            executed += 1;
        }
        let (_, _, rules) = entries(attribute);
        let program = &rules.programs.members[0];
        let missing = recipe
            .compiled_rules()
            .evaluate(
                &rules.owner,
                &program.id,
                &[],
                recipe.schema().as_ref(),
                &mut scratch,
            )
            .unwrap();
        assert_eq!(missing.effects.len(), 6);
        assert!(missing.effects.iter().all(|effect| matches!(&effect.disposition, EffectDisposition::Unresolved { input } if input == &program.reads[0].id)));
    }
    assert_eq!(executed, 879);
}

#[test]
fn partial_inventories_foreign_choices_and_wrong_channels_are_not_absence_proofs() {
    let attributes = attributes();
    let attribute = attributes[0];
    let (definition, choice, rules) = entries(attribute);
    let channels = channels();
    let gap = SchemaGap {
        subject: rules.owner.clone(),
        facet: SchemaFacet::GameRules,
        code: "review-required".parse().unwrap(),
    };
    let mut renamed = rules.clone();
    renamed.programs.members[0].id = "independent-program-name".parse().unwrap();
    for (index, effect) in renamed.programs.members[0].effects.iter_mut().enumerate() {
        effect.id = format!("independent-effect-{index}").parse().unwrap();
    }
    assert!(attribute_family(
        definition, choice, attribute, &renamed, &channels
    ));
    for control in 0..6 {
        let mut definition = definition.clone();
        let mut choice = choice.clone();
        let mut rules = rules.clone();
        let SchemaState::Known(schema) = &mut definition.schema else {
            unreachable!()
        };
        match control {
            0 => {
                rules.programs.closure = SchemaClosure::Partial {
                    gaps: vec![gap.clone()],
                }
            }
            1 => {
                schema.declarations.actors.closure = SchemaClosure::Partial {
                    gaps: vec![gap.clone()],
                }
            }
            2 => {
                rules.programs.members[0].reads[0].source = RuleReadSource::Choice {
                    slot: attributes[1].slot.clone(),
                }
            }
            3 => {
                choice.schema = SchemaState::Unmapped {
                    gaps: vec![gap.clone()],
                }
            }
            4 => rules.programs.members[0].effects[0].when = None,
            5 => {
                let value = rules.programs.members[0].effects[0].effect.clone();
                rules.programs.members[0].effects[1].effect = value;
            }
            _ => unreachable!(),
        }
        assert!(
            !attribute_family(&definition, &choice, attribute, &rules, &channels),
            "control {control}"
        );
    }
    let source_node = source::verified_catalog()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, TreeNodeKind::Attribute { .. }))
        .unwrap();
    let selector = selector(&source_node.key);
    let entry = source::verified_mapping()
        .entries
        .iter()
        .find(|row| row.source == selector)
        .unwrap();
    assert!(exact_node(&[], &selector).is_none());
    assert!(exact_node(&[entry.clone(), entry.clone()], &selector).is_none());
    let mut alias = entry.clone();
    let MappingOutcome::Mapped { basis, .. } = &mut alias.outcome else {
        unreachable!()
    };
    *basis = MappingBasis::ReviewedAlias {
        reason: "not-the-exact-binding".parse().unwrap(),
    };
    assert!(exact_node(&[alias], &selector).is_none());
}

#[test]
fn selected_sniper_attributes_use_saved_choices_and_keep_pending_build_obligations() {
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
        &query_set.queries,
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
    let attributes = attributes();
    let channels = channels();
    let mut scratch = recipe.compiled_rules().new_scratch();
    let mut count = 0;
    for allocation in &draft.input().allocations.members {
        if !selected.contains(&allocation.id) {
            continue;
        }
        let node = allocation.node.to_resolved().unwrap();
        let Some(attribute) = attributes.iter().find(|row| row.node == node) else {
            continue;
        };
        assert_eq!(allocation.choices.members.len(), 1);
        let choice = &allocation.choices.members[0];
        assert_eq!(choice.slot.to_resolved(), Some(attribute.slot.clone()));
        let Some(ParameterValue::Option(option)) = choice.value.to_resolved() else {
            panic!("saved exact attribute option")
        };
        assert_projection(attribute, &option, &channels, &mut scratch);
        count += 1;
    }
    assert_eq!(count, 22);
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
