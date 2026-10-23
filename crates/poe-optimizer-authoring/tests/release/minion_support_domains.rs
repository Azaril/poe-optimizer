//! Source-bounded numerical passive laws against the maintained release. The
//! source proof is independent of these bodies and of the support inventory.
use super::{minion_support_source as source, originals, tracked_release};
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
        ExternalOwnerSelector, ExternalSelector, MappingBasis, MappingOutcome, SourceComponent,
    },
    owned_normalize::{NormalizationArtifacts, normalize_fresh},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path};

const FAMILIES: [&str; 6] = [
    "plain-minion-owner-closure",
    "plain-minion-life-passives",
    "minion-life-passives",
    "command-cooldown",
    "command-damage",
    "growing-swarm",
];
fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
struct NumericOwner {
    source: String,
    node: PassiveNodeDefId,
    unit: UnitDefId,
    // Each inner vector is one complete program, preserving paired effects.
    programs: Vec<Vec<(StatDefId, f64)>>,
}
fn selector(id: &str) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode {
        tree_version: SourceComponent::Text(source::verified_catalog().tree_version.clone()),
        node_id: SourceComponent::Text(id.into()),
        view: SourceComponent::Missing,
    })
}
fn owners() -> Vec<NumericOwner> {
    let mut owners = Vec::new();
    for family in FAMILIES {
        let b = source::verified_bindings(family);
        let unit: UnitDefId = typed(if family == FAMILIES[0] {
            &b["unit"]
        } else {
            &b["percent_unit"]
        });
        if family == "growing-swarm" {
            owners.push(NumericOwner {
                source: b["source_id"].as_str().unwrap().into(),
                node: typed(&b["node"]),
                unit,
                programs: vec![vec![
                    (typed(&b["channels"]["area_player"]), 20.0),
                    (typed(&b["channels"]["cooldown_player"]), 20.0),
                ]],
            });
            continue;
        }
        for row in b["nodes"].as_array().unwrap() {
            let amount = row["amount"].as_f64().unwrap();
            let programs = match family {
                "plain-minion-owner-closure" => vec![vec![(typed(&b["player_stat"]), amount)]],
                "plain-minion-life-passives" => vec![
                    vec![(typed(&b["damage_stat"]), amount)],
                    vec![(typed(&b["life_increase"]), amount)],
                ],
                "minion-life-passives" => vec![vec![(typed(&b["life_increase"]), amount)]],
                "command-cooldown" => {
                    let mut programs = Vec::new();
                    if amount == 8.0 {
                        programs.push(vec![(typed(&b["damage_stat"]), 6.0)]);
                    }
                    programs.push(vec![(typed(&b["channels"]["player"]), amount)]);
                    programs
                }
                "command-damage" => vec![vec![(typed(&b["channels"]["player"]), amount)]],
                _ => unreachable!(),
            };
            owners.push(NumericOwner {
                source: row["source_id"].as_str().unwrap().into(),
                node: typed(row.get("node").unwrap_or(&row["definition"])),
                unit: unit.clone(),
                programs,
            });
        }
    }
    assert_eq!(owners.len(), 18);
    let mut keys = BTreeSet::new();
    for row in &owners {
        assert!(keys.insert(row.node.clone()));
        let entries: Vec<_> = source::verified_mapping()
            .entries
            .iter()
            .filter(|entry| entry.source == selector(&row.source))
            .collect();
        assert_eq!(entries.len(), 1);
        assert!(matches!(&entries[0].outcome, MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::PassiveNode(node)), basis: MappingBasis::Exact
        } if node == &row.node));
    }
    // Fixed source inventory, never all nodes matching a name or stat string.
    assert_eq!(
        owners
            .iter()
            .map(|row| row.source.as_str())
            .collect::<BTreeSet<_>>(),
        source::SOURCE_IDS.iter().copied().collect()
    );
    owners.sort_by_key(|row| {
        source::SOURCE_IDS
            .iter()
            .position(|id| *id == row.source)
            .unwrap()
    });
    owners
}
pub(super) fn declarations() -> Vec<SupportSourceDomainDeclaration> {
    owners()
        .into_iter()
        .map(|row| SupportSourceDomainDeclaration {
            owner: SchemaSubject::Definition(row.node.address()),
            domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
        })
        .collect()
}
fn complete_empty<T>(set: &DeclaredSet<T>) -> bool {
    set.is_complete() && set.members.is_empty()
}
fn definition(
    node: &PassiveNodeDefId,
) -> &'static DefinitionEntry<PassiveNodeDefId, PassiveNodeSchema> {
    let Some(DefinitionDescriptor::PassiveNode(row)) = tracked_release()
        .release()
        .assembled()
        .checked()
        .schema()
        .lookup_definition(&node.address())
    else {
        panic!("exact passive schema")
    };
    row
}
fn rules(subject: &SchemaSubject) -> &'static DefinitionRules {
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
fn expression<'a>(program: &'a RuleProgram, id: &OwnedDefinitionKey) -> &'a RuleExpression {
    &program
        .nodes
        .iter()
        .find(|node| &node.id == id)
        .unwrap()
        .expression
}
fn literal_program(program: &RuleProgram, unit: &UnitDefId, expected: &[(StatDefId, f64)]) -> bool {
    if program.context != RuleEntityKind::Actor
        || !program.reads.is_empty()
        || program.nodes.len() != 1
        || program.effects.len() != expected.len()
    {
        return false;
    }
    let RuleExpression::Literal {
        value: ParameterValue::Quantity(amount),
    } = &program.nodes[0].expression
    else {
        return false;
    };
    if amount.unit() != unit {
        return false;
    }
    let mut remaining = expected.to_vec();
    for effect in &program.effects {
        let RuleEffectKind::Contribute {
            entity: RuleEntity::Player,
            stat,
            contribution: ContributionKind::Increase,
            value,
        } = &effect.effect
        else {
            return false;
        };
        if effect.when.is_some() || value != &program.nodes[0].id {
            return false;
        }
        let Some(index) = remaining
            .iter()
            .position(|(wanted, number)| wanted == stat && *number == amount.value())
        else {
            return false;
        };
        remaining.remove(index);
    }
    remaining.is_empty()
}
fn numeric_family(
    row: &NumericOwner,
    definition: &DefinitionEntry<PassiveNodeDefId, PassiveNodeSchema>,
    rules: &DefinitionRules,
) -> bool {
    let SchemaState::Known(schema) = &definition.schema else {
        return false;
    };
    let d = &schema.declarations;
    if definition.id != row.node
        || rules.owner != SchemaSubject::Definition(row.node.address())
        || !rules.programs.is_complete()
        || rules.programs.members.len() != row.programs.len()
        || !complete_empty(&d.parameters)
        || !complete_empty(&d.choices)
        || !complete_empty(&d.grants)
        || !complete_empty(&d.actors)
        || !complete_empty(&d.skill_grants)
        || !complete_empty(&d.outputs)
        || !complete_empty(&d.sockets)
    {
        return false;
    }
    let mut remaining: Vec<_> = row.programs.iter().collect();
    for program in &rules.programs.members {
        let Some(index) = remaining
            .iter()
            .position(|expected| literal_program(program, &row.unit, expected))
        else {
            return false;
        };
        remaining.remove(index);
    }
    remaining.is_empty()
}
fn quantity(value: f64, unit: &UnitDefId) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap())
}
fn applied_value(
    owner: &SchemaSubject,
    program: &RuleProgram,
    facts: &[RuleFact],
    scratch: &mut RuleScratch,
) -> ParameterValue {
    let recipe = tracked_release().release().assembled().checked();
    let result = recipe
        .compiled_rules()
        .evaluate(owner, &program.id, facts, recipe.schema().as_ref(), scratch)
        .unwrap();
    assert_eq!(result.effects.len(), 1);
    let EffectDisposition::Applied { value } = &result.effects[0].disposition else {
        panic!("expected resolved component effect: {:?}", result.effects)
    };
    value.clone()
}
fn assert_missing(owner: &SchemaSubject, program: &RuleProgram, scratch: &mut RuleScratch) {
    let recipe = tracked_release().release().assembled().checked();
    let result = recipe
        .compiled_rules()
        .evaluate(owner, &program.id, &[], recipe.schema().as_ref(), scratch)
        .unwrap();
    assert!(
        result
            .effects
            .iter()
            .all(|effect| matches!(effect.disposition, EffectDisposition::Unresolved { .. }))
    );
}

#[test]
fn exact_eighteen_sources_preserve_all_twenty_six_numerical_programs() {
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    let mut scratch = recipe.compiled_rules().new_scratch();
    let mut programs = 0;
    let mut effects = 0;
    for (row, domain) in owners().iter().zip(declarations()) {
        let body = rules(&domain.owner);
        assert!(
            numeric_family(row, definition(&row.node), body),
            "source {}",
            row.source
        );
        assert_eq!(
            release.mapping().lookup(&selector(&row.source)),
            source::verified_mapping()
                .entries
                .iter()
                .find(|entry| entry.source == selector(&row.source))
                .map(|entry| &entry.outcome)
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
                .filter(|entry| entry.owner == domain.owner)
                .collect::<Vec<_>>(),
            [&domain]
        );
        for program in &body.programs.members {
            let result = recipe
                .compiled_rules()
                .evaluate(
                    &body.owner,
                    &program.id,
                    &[],
                    recipe.schema().as_ref(),
                    &mut scratch,
                )
                .unwrap();
            assert_eq!(result.owner_programs_closure, SchemaClosure::Complete);
            for effect in &result.effects {
                let RuleEffectKind::Contribute { stat, .. } = &effect.effect else {
                    panic!("numerical contribution")
                };
                let amount = row
                    .programs
                    .iter()
                    .flatten()
                    .find(|(wanted, _)| wanted == stat)
                    .unwrap()
                    .1;
                assert_eq!(
                    effect.disposition,
                    EffectDisposition::Applied {
                        value: quantity(amount, &row.unit)
                    }
                );
                let SchemaLookup::Known(channel) = recipe.schema().definition(stat) else {
                    panic!("known numerical channel")
                };
                assert_eq!(
                    channel.value,
                    ComputedValueType::Quantity {
                        unit: row.unit.clone()
                    }
                );
                assert_eq!(channel.targets, [RuleEntityKind::Actor]);
                effects += 1;
            }
            programs += 1;
        }
    }
    assert_eq!((programs, effects), (26, 27));
    assert!(release.evaluation().is_none());
}

#[test]
fn retained_topology_receivers_and_action_inventory_survive_publisher_retirement() {
    let recipe = tracked_release().release().assembled().checked();
    for family in [FAMILIES[0], FAMILIES[1], FAMILIES[3], FAMILIES[5]] {
        let closure = source::verified_closure(family);
        for value in closure["definitions"].as_array().unwrap() {
            let old: DefinitionDescriptor = typed(value);
            let DefinitionDescriptor::PassiveNode(old) = old else {
                continue;
            };
            assert_eq!(
                definition(&old.id),
                &old,
                "full passive schema including topology: {family}"
            );
        }
    }
    for family in ["command-cooldown", "growing-swarm"] {
        let migration = source::verified_migration(family);
        for value in migration["receivers"].as_array().unwrap() {
            let expected: StatReceiver = typed(value);
            assert_eq!(
                recipe
                    .rules()
                    .input()
                    .receivers
                    .members
                    .iter()
                    .find(|row| row.id == expected.id),
                Some(&expected)
            );
        }
    }
    let b = source::verified_bindings("command-cooldown");
    let output: DeclaredSlot<ActionOutputDefId> = typed(&b["actions"]["gas"]["output"]);
    let SchemaLookup::Known(schema) = recipe.schema().slot(&output) else {
        panic!("gas output schema")
    };
    assert!(schema.stat_sets.is_complete());
    assert_eq!(
        schema.stat_sets.members,
        b["gas_stat_sets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| typed::<ActionStatSetDefId>(&row["id"]))
            .collect::<Vec<_>>()
    );
    for action in ["basic", "gas"] {
        let slot: DeclaredSlot<ActionOutputDefId> = typed(&b["actions"][action]["output"]);
        let body = rules(&SchemaSubject::Slot(SlotAddress::ActionOutput(slot)));
        assert!(
            !body.programs.is_complete(),
            "ordinary component laws do not close an Action"
        );
        for id in [
            "actor-level-input",
            "commandable-action-fact",
            "conditional-command-cooldown",
            "unconditional-minion-cooldown",
            "conditional-command-damage",
        ] {
            assert_eq!(
                body.programs
                    .members
                    .iter()
                    .filter(|program| program.id.as_str() == id)
                    .count(),
                1
            );
        }
    }
}

#[test]
fn pure_producer_proof_rejects_unknown_inventory_changed_authority_or_extra_effects() {
    let rows = owners();
    let row = &rows[0];
    let original_definition = definition(&row.node);
    let body = rules(&SchemaSubject::Definition(row.node.address()));
    assert!(numeric_family(row, original_definition, body));
    let mut renamed = body.clone();
    renamed.programs.members[0].id = "cosmetic-program-name".parse().unwrap();
    renamed.programs.members[0].effects[0].id = "cosmetic-effect-name".parse().unwrap();
    assert!(numeric_family(row, original_definition, &renamed));
    let gap = SchemaGap {
        subject: body.owner.clone(),
        facet: SchemaFacet::GameRules,
        code: "not-reviewed".parse().unwrap(),
    };
    for control in 0..7 {
        let mut schema = original_definition.clone();
        let mut body = body.clone();
        match control {
            0 => {
                body.programs.closure = SchemaClosure::Partial {
                    gaps: vec![gap.clone()],
                }
            }
            1 => {
                let SchemaState::Known(schema) = &mut schema.schema else {
                    unreachable!()
                };
                schema.declarations.skill_grants.closure = SchemaClosure::Partial {
                    gaps: vec![gap.clone()],
                };
            }
            2 => {
                if let RuleEffectKind::Contribute { entity, .. } =
                    &mut body.programs.members[0].effects[0].effect
                {
                    *entity = RuleEntity::Current;
                }
            }
            3 => {
                body.programs.members[0].effects[0].when =
                    Some(body.programs.members[0].nodes[0].id.clone())
            }
            4 => {
                let duplicate = body.programs.members[0].effects[0].clone();
                body.programs.members[0].effects.push(duplicate);
            }
            5 => {
                body.programs.members[0].nodes[0].expression = RuleExpression::Literal {
                    value: quantity(999.0, &row.unit),
                }
            }
            6 => body.owner = SchemaSubject::Definition(rows[1].node.address()),
            _ => unreachable!(),
        }
        assert!(!numeric_family(row, &schema, &body), "control {control}");
    }
    // Losing either of a paired producer's effects must not retain the proof.
    let row = rows.iter().find(|row| row.programs[0].len() == 2).unwrap();
    let mut body = rules(&SchemaSubject::Definition(row.node.address())).clone();
    body.programs.members[0].effects.pop();
    assert!(!numeric_family(row, definition(&row.node), &body));
}

fn assert_passthrough(
    stat: &StatDefId,
    program_id: &str,
    expected_source: RuleReadSource,
    amount: f64,
    unit: &UnitDefId,
    scratch: &mut RuleScratch,
) {
    let owner = SchemaSubject::Definition(stat.address());
    let body = rules(&owner);
    assert!(body.programs.is_complete());
    assert_eq!(body.programs.members.len(), 1);
    let program = &body.programs.members[0];
    assert_eq!(program.id.as_str(), program_id);
    assert_eq!(program.context, RuleEntityKind::Actor);
    assert_eq!(program.reads.len(), 1);
    assert_eq!(program.reads[0].source, expected_source);
    assert_eq!(
        program.reads[0].value_type,
        ComputedValueType::Quantity { unit: unit.clone() }
    );
    assert_eq!(program.nodes.len(), 1);
    assert!(
        matches!(&program.nodes[0].expression, RuleExpression::Read { input } if input == &program.reads[0].id)
    );
    assert_eq!(program.effects.len(), 1);
    assert!(program.effects[0].when.is_none());
    assert!(
        matches!(&program.effects[0].effect, RuleEffectKind::Derive { entity: RuleEntity::Current, stat: target, value } if target == stat && value == &program.nodes[0].id)
    );
    assert_missing(&owner, program, scratch);
    for amount in [amount, 0.0, amount] {
        let input = quantity(amount, unit);
        assert_eq!(
            applied_value(
                &owner,
                program,
                &[RuleFact {
                    read: program.reads[0].id.clone(),
                    value: input.clone()
                }],
                scratch
            ),
            input
        );
    }
}

#[test]
fn exact_actor_receivers_keep_conditional_and_unconditional_channels_separate() {
    let recipe = tracked_release().release().assembled().checked();
    let mut scratch = recipe.compiled_rules().new_scratch();
    let mut cases = Vec::new();
    for (family, amount) in [("command-cooldown", 72.0), ("command-damage", 55.0)] {
        let b = source::verified_bindings(family);
        cases.push((b, "player", "actor", amount));
    }
    let swarm = source::verified_bindings("growing-swarm");
    cases.extend([
        (swarm, "cooldown_player", "cooldown_actor", 20.0),
        (swarm, "area_player", "area_actor", 20.0),
    ]);
    for (b, player_key, actor_key, amount) in cases {
        let unit: UnitDefId = typed(&b["percent_unit"]);
        let player: StatDefId = typed(&b["channels"][player_key]);
        let actor: StatDefId = typed(&b["channels"][actor_key]);
        let actor_slot: DeclaredSlot<ActorSlotDefId> = typed(&b["actor_slot"]);
        for (stat, targets) in [
            (&player, vec![StatReceiverTarget::Player]),
            (
                &actor,
                vec![StatReceiverTarget::OwnedSlot { slot: actor_slot }],
            ),
        ] {
            let receivers: Vec<_> = recipe
                .rules()
                .input()
                .receivers
                .members
                .iter()
                .filter(|r| &r.stat == stat)
                .collect();
            assert_eq!(receivers.len(), 1);
            assert_eq!(receivers[0].targets, targets);
        }
        assert_passthrough(
            &player,
            b["programs"][player_key].as_str().unwrap(),
            RuleReadSource::Contributions {
                entity: RuleEntity::Current,
                stat: player.clone(),
                contribution: ContributionKind::Increase,
                reduction: ContributionReduction::Sum,
                empty: quantity(0.0, &unit),
            },
            amount,
            &unit,
            &mut scratch,
        );
        assert_passthrough(
            &actor,
            b["programs"][actor_key].as_str().unwrap(),
            RuleReadSource::Stat {
                entity: RuleEntity::Player,
                stat: player,
            },
            amount,
            &unit,
            &mut scratch,
        );
    }
    for action in ["basic", "gas"] {
        let b = source::verified_bindings("command-cooldown");
        let output: DeclaredSlot<ActionOutputDefId> = typed(&b["actions"][action]["output"]);
        let owner = SchemaSubject::Slot(SlotAddress::ActionOutput(output));
        let body = rules(&owner);
        let eligibility = body
            .programs
            .members
            .iter()
            .find(|p| p.id.as_str() == "commandable-action-fact")
            .unwrap();
        assert!(eligibility.reads.is_empty());
        assert_eq!(eligibility.context, RuleEntityKind::Action);
        assert_eq!(eligibility.nodes.len(), 1);
        assert_eq!(eligibility.effects.len(), 1);
        assert!(eligibility.effects[0].when.is_none());
        assert_eq!(
            eligibility.nodes[0].expression,
            RuleExpression::Literal {
                value: ParameterValue::Boolean(action == "gas")
            }
        );
        let commandable: StatDefId = typed(&b["channels"]["commandable"]);
        let SchemaLookup::Known(commandable_schema) = recipe.schema().definition(&commandable)
        else {
            panic!("known eligibility stat")
        };
        assert_eq!(commandable_schema.value, ComputedValueType::Boolean);
        assert_eq!(commandable_schema.targets, [RuleEntityKind::Action]);
        assert!(
            matches!(&eligibility.effects[0].effect, RuleEffectKind::Derive { entity: RuleEntity::Current, stat, value } if stat == &commandable && value == &eligibility.nodes[0].id)
        );
        assert_eq!(
            applied_value(&owner, eligibility, &[], &mut scratch),
            ParameterValue::Boolean(action == "gas")
        );
        for (family, amount) in [("command-cooldown", 72.0), ("command-damage", 55.0)] {
            let b = source::verified_bindings(family);
            let unit: UnitDefId = typed(&b["percent_unit"]);
            let p = body
                .programs
                .members
                .iter()
                .find(|p| p.id.as_str() == b["programs"]["action"].as_str().unwrap())
                .unwrap();
            assert_eq!(p.context, RuleEntityKind::Action);
            assert_eq!(p.reads.len(), 2);
            assert_eq!(p.nodes.len(), 4);
            let eligible = p
                .reads
                .iter()
                .find(|r| r.value_type == ComputedValueType::Boolean)
                .unwrap();
            let incoming = p
                .reads
                .iter()
                .find(|r| r.value_type == ComputedValueType::Quantity { unit: unit.clone() })
                .unwrap();
            assert_eq!(
                eligible.source,
                RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: commandable.clone()
                }
            );
            assert_eq!(
                incoming.source,
                RuleReadSource::Stat {
                    entity: RuleEntity::Actor,
                    stat: typed(&b["channels"]["actor"])
                }
            );
            assert_eq!(p.effects.len(), 1);
            let RuleEffectKind::Contribute {
                entity: RuleEntity::Current,
                stat,
                contribution: ContributionKind::Increase,
                value,
            } = &p.effects[0].effect
            else {
                panic!("exact Action contribution")
            };
            assert_eq!(*stat, typed::<StatDefId>(&b["channels"]["action"]));
            assert!(p.effects[0].when.is_none());
            let RuleExpression::Select {
                condition,
                when_true,
                when_false,
            } = expression(p, value)
            else {
                panic!("explicit conditional selection")
            };
            assert!(
                matches!(expression(p, condition), RuleExpression::Read { input } if input == &eligible.id)
            );
            assert!(
                matches!(expression(p, when_true), RuleExpression::Read { input } if input == &incoming.id)
            );
            assert_eq!(
                expression(p, when_false),
                &RuleExpression::Literal {
                    value: quantity(0.0, &unit)
                }
            );
            assert_missing(&owner, p, &mut scratch);
            for flag in [false, true, false] {
                let facts = [
                    RuleFact {
                        read: eligible.id.clone(),
                        value: ParameterValue::Boolean(flag),
                    },
                    RuleFact {
                        read: incoming.id.clone(),
                        value: quantity(amount, &unit),
                    },
                ];
                assert_eq!(
                    applied_value(&owner, p, &facts, &mut scratch),
                    quantity(if flag { amount } else { 0.0 }, &unit)
                );
            }
            // Known false is sufficient; unknown or true does not invent a received value.
            assert_eq!(
                applied_value(
                    &owner,
                    p,
                    &[RuleFact {
                        read: eligible.id.clone(),
                        value: ParameterValue::Boolean(false)
                    }],
                    &mut scratch
                ),
                quantity(0.0, &unit)
            );
            let unresolved = recipe
                .compiled_rules()
                .evaluate(
                    &owner,
                    &p.id,
                    &[RuleFact {
                        read: eligible.id.clone(),
                        value: ParameterValue::Boolean(true),
                    }],
                    recipe.schema().as_ref(),
                    &mut scratch,
                )
                .unwrap();
            assert!(
                matches!(&unresolved.effects[0].disposition, EffectDisposition::Unresolved { input } if input == &incoming.id)
            );
        }
        let unit: UnitDefId = typed(&swarm["percent_unit"]);
        let p = body
            .programs
            .members
            .iter()
            .find(|p| p.id.as_str() == "unconditional-minion-cooldown")
            .unwrap();
        assert_eq!(p.context, RuleEntityKind::Action);
        assert_eq!(p.reads.len(), 1);
        assert_eq!(p.nodes.len(), 1);
        assert_eq!(p.effects.len(), 1);
        assert_eq!(
            p.reads[0].source,
            RuleReadSource::Stat {
                entity: RuleEntity::Actor,
                stat: typed(&swarm["channels"]["cooldown_actor"])
            }
        );
        assert_eq!(
            p.reads[0].value_type,
            ComputedValueType::Quantity { unit: unit.clone() }
        );
        assert!(
            matches!(&p.nodes[0].expression, RuleExpression::Read { input } if input == &p.reads[0].id)
        );
        assert!(p.effects[0].when.is_none());
        assert!(
            matches!(&p.effects[0].effect, RuleEffectKind::Contribute { entity: RuleEntity::Current, stat, contribution: ContributionKind::Increase, value } if stat == &typed::<StatDefId>(&swarm["channels"]["cooldown_action"]) && value == &p.nodes[0].id)
        );
        assert_missing(&owner, p, &mut scratch);
        assert_eq!(
            applied_value(
                &owner,
                p,
                &[RuleFact {
                    read: p.reads[0].id.clone(),
                    value: quantity(20.0, &unit)
                }],
                &mut scratch
            ),
            quantity(20.0, &unit)
        );
    }
}

#[test]
fn saved_sniper_selects_all_eighteen_sources_without_closing_pending_obligations() {
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
    let expected: BTreeSet<_> = owners().into_iter().map(|row| row.node).collect();
    let actual: BTreeSet<_> = draft
        .input()
        .allocations
        .members
        .iter()
        .filter(|allocation| selected.contains(&allocation.id))
        .filter_map(|allocation| allocation.node.to_resolved())
        .filter(|node| expected.contains(node))
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 18);
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
