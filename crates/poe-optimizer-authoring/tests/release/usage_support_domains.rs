//! A finite, test-only review of pure UsagePolicy parameter projections.
//! Compiled schema/rule validation remains mandatory; this is not a compiler or
//! an inferred default for unreviewed source owners.
use super::tracked_release;
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{EffectDisposition, RuleFact};
use std::collections::BTreeSet;

const POLICIES: &[(&str, &str)] = &[
    ("def.0000000000003259", "skill-effect-activation"),
    ("def.0000000000003267", "frost-bomb-requested-global-effect"),
    ("def.000000000000326a", "sniper-requested-count"),
    ("def.000000000000332b", "requested-skill-participation"),
];
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
fn policy(key: &str) -> UsagePolicyDefId {
    UsagePolicyDefId::parse(namespace(), key).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    value.parse().unwrap()
}
pub(super) fn declarations() -> Vec<SupportSourceDomainDeclaration> {
    POLICIES
        .iter()
        .map(|(id, _)| SupportSourceDomainDeclaration {
            owner: SchemaSubject::Definition(policy(id).address()),
            domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
        })
        .collect()
}
fn complete_empty<T>(set: &DeclaredSet<T>) -> bool {
    set.is_complete() && set.members.is_empty()
}

/// Conservative finite family, independent of policy/program/stat names. It
/// admits no grants, extra occurrences, external reads, capability writes,
/// contributions, applicability effects or scenario expansion authority.
fn parameter_projection_only(
    definition: &DefinitionEntry<UsagePolicyDefId, UsagePolicySchema>,
    rules: &DefinitionRules,
    index: &impl DefinitionSchemaIndex,
) -> bool {
    let SchemaState::Known(schema) = &definition.schema else {
        return false;
    };
    let declarations = &schema.declarations;
    if rules.owner != SchemaSubject::Definition(definition.id.address())
        || !rules.programs.is_complete()
        || rules.programs.members.is_empty()
        || schema.targets != [UsageTargetKind::Skill]
        || schema.scenario_skill_applicability.is_some()
        || !declarations.parameters.is_complete()
        || declarations.parameters.members.is_empty()
        || !complete_empty(&declarations.choices)
        || !complete_empty(&declarations.grants)
        || !complete_empty(&declarations.actors)
        || !complete_empty(&declarations.skill_grants)
        || !complete_empty(&declarations.outputs)
        || !complete_empty(&declarations.sockets)
    {
        return false;
    }
    let owner = SlotOwnerDefId::UsagePolicy(definition.id.clone());
    let parameters: BTreeSet<_> = declarations.parameters.members.iter().collect();
    if parameters.len() != declarations.parameters.members.len() {
        return false;
    }
    for parameter in &parameters {
        let SchemaLookup::Known(schema) = index.slot(parameter) else {
            return false;
        };
        if parameter.declaration != owner
            || schema.presence != SlotPresence::RequiredOnce
            || schema.sites != [ParameterSite::UsagePolicyParameter]
            || schema.skill_input.is_some()
            || !matches!(schema.value, ValueSchema::Boolean | ValueSchema::Integer(_))
        {
            return false;
        }
    }
    let mut used = BTreeSet::new();
    for program in &rules.programs.members {
        if program.context != RuleEntityKind::Skill
            || program.reads.is_empty()
            || program.effects.is_empty()
        {
            return false;
        }
        let mut reads = BTreeSet::new();
        for read in &program.reads {
            let RuleReadSource::Parameter { slot } = &read.source else {
                return false;
            };
            if !parameters.contains(slot) || !reads.insert(&read.id) {
                return false;
            }
            let SchemaLookup::Known(schema) = index.slot(slot) else {
                return false;
            };
            if !matches!(
                (&schema.value, &read.value_type),
                (ValueSchema::Boolean, ComputedValueType::Boolean)
                    | (ValueSchema::Integer(_), ComputedValueType::Integer)
            ) {
                return false;
            }
            used.insert(slot);
        }
        let mut nodes = BTreeSet::new();
        for node in &program.nodes {
            let pure = match &node.expression {
                RuleExpression::Read { input } => reads.contains(input),
                RuleExpression::Literal { value } => matches!(
                    value,
                    ParameterValue::Boolean(_)
                        | ParameterValue::Integer(_)
                        | ParameterValue::Quantity(_)
                ),
                RuleExpression::ScaleInteger { value, count } => {
                    nodes.contains(value) && nodes.contains(count)
                }
                RuleExpression::All { values } => {
                    !values.is_empty() && values.iter().all(|value| nodes.contains(value))
                }
                _ => false,
            };
            if !pure || !nodes.insert(&node.id) {
                return false;
            }
        }
        for effect in &program.effects {
            let RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat,
                value,
            } = &effect.effect
            else {
                return false;
            };
            let SchemaLookup::Known(schema) = index.definition(stat) else {
                return false;
            };
            if effect.when.is_some()
                || !nodes.contains(value)
                || !schema.targets.contains(&RuleEntityKind::Skill)
            {
                return false;
            }
        }
    }
    used == parameters
}

fn entry(
    id: &str,
) -> (
    &'static DefinitionEntry<UsagePolicyDefId, UsagePolicySchema>,
    &'static DefinitionRules,
) {
    let recipe = tracked_release().release().assembled().checked();
    let owner = policy(id).address();
    let Some(DefinitionDescriptor::UsagePolicy(entry)) = recipe.schema().lookup_definition(&owner)
    else {
        panic!("known UsagePolicy")
    };
    let rules = recipe
        .rules()
        .input()
        .owners
        .iter()
        .find(|row| row.owner == SchemaSubject::Definition(owner.clone()))
        .unwrap();
    (entry, rules)
}

#[test]
fn reviewed_usage_domains_are_exact_complete_parameter_projection_owners() {
    let recipe = tracked_release().release().assembled().checked();
    let current = &recipe
        .rules()
        .input()
        .support_discovery
        .as_ref()
        .unwrap()
        .providers;
    for ((id, program), declaration) in POLICIES.iter().zip(declarations()) {
        let (entry, rules) = entry(id);
        assert!(parameter_projection_only(
            entry,
            rules,
            recipe.schema().as_ref()
        ));
        assert_eq!(rules.programs.members.len(), 1);
        assert_eq!(rules.programs.members[0].id.as_str(), *program);
        assert_eq!(
            current
                .iter()
                .filter(|row| row.owner == declaration.owner)
                .count(),
            1
        );
        assert!(current.contains(&declaration));
        let SchemaState::Known(schema) = &entry.schema else {
            unreachable!()
        };
        assert!(
            schema.scenario_skill_applicability.is_none(),
            "source absence grants no scenario expansion"
        );
        assert!(
            schema
                .declarations
                .parameters
                .members
                .iter()
                .all(|slot| slot.declaration == SlotOwnerDefId::UsagePolicy(entry.id.clone()))
        );
    }
    let (_, count) = entry(POLICIES[2].0);
    let RuleReadSource::Parameter { slot } = &count.programs.members[0].reads[0].source else {
        unreachable!()
    };
    let SchemaLookup::Known(parameter) = recipe.schema().slot(slot) else {
        unreachable!()
    };
    assert!(
        matches!(&parameter.value, ValueSchema::Integer(range) if range.minimum.get() == 0 && range.maximum.get() == 4)
    );
    assert!(tracked_release().release().input().evaluation.is_none());
}

#[test]
fn pure_usage_projections_keep_false_zero_and_missing_inputs_distinct() {
    let recipe = tracked_release().release().assembled().checked();
    let compiled = recipe.compiled_rules();
    let mut scratch = compiled.new_scratch();
    let count_unit = UnitDefId::parse(namespace(), "def.000000000000295a").unwrap();
    let boolean = |value| ParameterValue::Boolean(value);
    let integer = |value| ParameterValue::Integer(BoundedInteger::new(value).unwrap());
    let quantity =
        |value| ParameterValue::Quantity(FiniteQuantity::new(value, count_unit.clone()).unwrap());
    for (id, program, cases) in [
        (
            POLICIES[0].0,
            POLICIES[0].1,
            vec![
                (vec![("enabled", boolean(false))], boolean(false)),
                (vec![("enabled", boolean(true))], boolean(true)),
            ],
        ),
        (
            POLICIES[1].0,
            POLICIES[1].1,
            vec![
                (vec![("requested", boolean(false))], boolean(false)),
                (vec![("requested", boolean(true))], boolean(true)),
            ],
        ),
        (
            POLICIES[2].0,
            POLICIES[2].1,
            vec![
                (vec![("count", integer(0))], quantity(0.0)),
                (vec![("count", integer(4))], quantity(4.0)),
            ],
        ),
        (
            POLICIES[3].0,
            POLICIES[3].1,
            [false, true]
                .into_iter()
                .flat_map(|group| {
                    [false, true].into_iter().map(move |occurrence| {
                        (
                            vec![
                                ("group-requested", boolean(group)),
                                ("occurrence-requested", boolean(occurrence)),
                            ],
                            boolean(group && occurrence),
                        )
                    })
                })
                .collect(),
        ),
    ] {
        let owner = SchemaSubject::Definition(policy(id).address());
        for (values, expected) in cases {
            let facts: Vec<_> = values
                .into_iter()
                .map(|(read, value)| RuleFact {
                    read: key(read),
                    value,
                })
                .collect();
            let result = compiled
                .evaluate(
                    &owner,
                    &key(program),
                    &facts,
                    recipe.schema().as_ref(),
                    &mut scratch,
                )
                .unwrap();
            assert_eq!(result.effects.len(), 1);
            assert_eq!(
                result.effects[0].disposition,
                EffectDisposition::Applied { value: expected }
            );
            assert!(matches!(
                result.owner_programs_closure,
                SchemaClosure::Complete
            ));
        }
        let missing = compiled
            .evaluate(
                &owner,
                &key(program),
                &[],
                recipe.schema().as_ref(),
                &mut scratch,
            )
            .unwrap();
        assert!(
            matches!(
                missing.effects[0].disposition,
                EffectDisposition::Unresolved { .. }
            ),
            "no missing input default: {program}"
        );
    }
    // Demand both participation inputs with the other known true. A known false
    // predicate may short-circuit evaluation; it cannot supply a missing required
    // authored parameter during request validation.
    for absent in ["group-requested", "occurrence-requested"] {
        let facts: Vec<_> = ["group-requested", "occurrence-requested"]
            .into_iter()
            .filter(|read| *read != absent)
            .map(|read| RuleFact {
                read: key(read),
                value: boolean(true),
            })
            .collect();
        let result = compiled
            .evaluate(
                &SchemaSubject::Definition(policy(POLICIES[3].0).address()),
                &key(POLICIES[3].1),
                &facts,
                recipe.schema().as_ref(),
                &mut scratch,
            )
            .unwrap();
        assert!(
            matches!(&result.effects[0].disposition, EffectDisposition::Unresolved { input } if input.as_str() == absent)
        );
    }
}

#[test]
fn structural_review_refuses_unknown_ownership_or_effects_and_ignores_display_names() {
    let recipe = tracked_release().release().assembled().checked();
    let index = recipe.schema().as_ref();
    let (original, rules) = entry(POLICIES[0].0);
    let gap = SchemaGap {
        subject: rules.owner.clone(),
        facet: SchemaFacet::GameRules,
        code: key("unreviewed-control"),
    };
    let mut renamed = rules.clone();
    renamed.programs.members[0].id = key("another-pure-projection");
    renamed.programs.members[0].effects[0].id = key("another-result");
    assert!(
        parameter_projection_only(original, &renamed, index),
        "no policy/program-name dispatch"
    );
    for control in 0..7 {
        let mut definition = original.clone();
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
            2 => schema.scenario_skill_applicability = Some(DeclaredSet::complete(vec![])),
            3 => schema.targets.push(UsageTargetKind::Actor),
            4 => rules.programs.members[0].reads[0].source = RuleReadSource::CharacterLevel,
            5 => {
                rules.programs.members[0].effects[0].effect = RuleEffectKind::SupportApplicability {
                    applicable: key("n_0"),
                }
            }
            6 => rules.programs.members[0].reads[0].value_type = ComputedValueType::Integer,
            _ => unreachable!(),
        }
        assert!(
            !parameter_projection_only(&definition, &rules, index),
            "control {control}"
        );
    }
    let mut unknown = original.clone();
    unknown.schema = SchemaState::Unmapped { gaps: vec![gap] };
    assert!(!parameter_projection_only(&unknown, rules, index));
    let mut foreign = rules.clone();
    let (_, other) = entry(POLICIES[1].0);
    foreign.programs.members[0].reads[0].source = other.programs.members[0].reads[0].source.clone();
    assert!(
        !parameter_projection_only(original, &foreign, index),
        "a different policy's parameter is not this occurrence's input"
    );
}
