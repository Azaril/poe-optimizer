//! Definition fact only: complete-owner preparation/readiness remains a separate gate.
use super::{initial_skill_types, support_prohibition_source, tracked_release};
use poe_optimizer_authoring::{SymbolRegistry, lower_program, parse_program};
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_definitions::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition};
use std::collections::BTreeSet;

pub(super) const SYMBOL: &str = "stat.cannot_be_supported";
pub(super) const PROGRAM: &str = "definition-support-prohibition";
pub(super) fn stat_names() -> BTreeSet<String> {
    BTreeSet::from([SYMBOL.to_owned()])
}
fn symbols() -> SymbolRegistry {
    let release = tracked_release();
    release
        .name_allocation()
        .names()
        .symbols(release.release().assembled().schema(), Default::default())
        .unwrap()
}
fn stat() -> StatDefId {
    let Some(DefinitionAddress::Stat(id)) = symbols().definition(SYMBOL).cloned() else {
        panic!("allocated generic Boolean Stat")
    };
    id
}
fn program() -> RuleProgram {
    let recipe = tracked_release().release().assembled().checked();
    recipe
        .rules()
        .input()
        .owners
        .iter()
        .find(|row| row.owner == initial_skill_types::owner())
        .unwrap()
        .programs
        .members
        .iter()
        .find(|program| program.id.as_str() == PROGRAM)
        .unwrap()
        .clone()
}
fn component(programs: Vec<RuleProgram>) -> RulePackageInput {
    let source = tracked_release()
        .release()
        .assembled()
        .checked()
        .rules()
        .input();
    let mut owner = source
        .owners
        .iter()
        .find(|owner| owner.owner == initial_skill_types::owner())
        .unwrap()
        .clone();
    owner.programs.members = programs;
    RulePackageInput {
        schema_version: source.schema_version,
        namespace: source.namespace.clone(),
        release: source.release.clone(),
        semantics_version: source.semantics_version.clone(),
        operations_version: source.operations_version.clone(),
        definitions: source.definitions.clone(),
        tables: vec![],
        owners: vec![owner],
        receivers: DeclaredSet::complete(vec![]),
        effect_applications: Some(DeclaredSet::complete(vec![])),
        contribution_queries: Some(DeclaredSet::complete(vec![])),
        existing_actor_rules: None,
        support_discovery: None,
    }
}

#[test]
fn sniper_definition_prohibition_is_a_separate_generic_skill_boolean_and_remains_partial() {
    support_prohibition_source::check_proof();
    let release = tracked_release().release();
    let recipe = release.assembled().checked();
    let SchemaLookup::Known(declaration) = recipe.schema().definition(&stat()) else {
        panic!("known Stat")
    };
    assert_eq!(declaration.value, ComputedValueType::Boolean);
    assert_eq!(declaration.targets, [RuleEntityKind::Skill]);
    assert!(!initial_skill_types::stat_names().contains(SYMBOL));
    let program = program();
    assert_eq!(program.context, RuleEntityKind::Skill);
    assert!(
        program.reads.is_empty(),
        "definition fact cannot depend on saved occurrence usage"
    );
    let [node] = program.nodes.as_slice() else {
        panic!("one definition literal")
    };
    assert_eq!(
        node.expression,
        RuleExpression::Literal {
            value: ParameterValue::Boolean(false)
        }
    );
    let [effect] = program.effects.as_slice() else {
        panic!("one exact output")
    };
    assert!(effect.when.is_none());
    assert_eq!(effect.id.as_str(), "definition-prohibition");
    assert_eq!(
        effect.effect,
        RuleEffectKind::Derive {
            entity: RuleEntity::Current,
            stat: stat(),
            value: node.id.clone()
        }
    );
    let compiled = recipe.compiled_rules();
    let mut scratch = compiled.new_scratch();
    let first = compiled
        .evaluate(
            &initial_skill_types::owner(),
            &program.id,
            &[],
            recipe.schema().as_ref(),
            &mut scratch,
        )
        .unwrap();
    let second = compiled
        .evaluate(
            &initial_skill_types::owner(),
            &program.id,
            &[],
            recipe.schema().as_ref(),
            &mut scratch,
        )
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(first.effects.len(), 1);
    assert_eq!(
        first.effects[0].disposition,
        EffectDisposition::Applied {
            value: ParameterValue::Boolean(false)
        }
    );
    assert!(matches!(
        first.owner_programs_closure,
        SchemaClosure::Partial { .. }
    ));
    assert!(
        release.evaluation().is_none(),
        "component fact does not complete selected-owner mechanics or readiness"
    );
}

#[test]
fn component_true_and_missing_producer_are_distinct_from_the_retained_false_fact() {
    support_prohibition_source::check_proof();
    let schema = tracked_release().release().assembled().schema();
    for expected in [true, false, true] {
        // A component control on the existing owner tests the shared Boolean
        // channel. It does not admit Abyssal as a production Skill definition.
        let text = format!(
            "program \"{PROGRAM}\" context skill {{ let prohibited = {expected}; effect \"definition-prohibition\" = derive(current, {SYMBOL}, prohibited); }}"
        );
        let program = lower_program(
            &parse_program(&text, Default::default()).unwrap(),
            schema,
            &symbols(),
            &[],
            Default::default(),
        )
        .unwrap()
        .program;
        let rules = component(vec![program.clone()]);
        let compiled = CompiledRulePackage::compile(&rules, schema, Default::default()).unwrap();
        let mut scratch = compiled.new_scratch();
        let result = compiled
            .evaluate(
                &initial_skill_types::owner(),
                &program.id,
                &[],
                schema,
                &mut scratch,
            )
            .unwrap();
        assert_eq!(result.effects.len(), 1);
        assert_eq!(
            result.effects[0].disposition,
            EffectDisposition::Applied {
                value: ParameterValue::Boolean(expected)
            }
        );
        assert_eq!(
            result,
            compiled
                .evaluate(
                    &initial_skill_types::owner(),
                    &program.id,
                    &[],
                    schema,
                    &mut scratch
                )
                .unwrap()
        );
    }
    let missing = component(vec![]);
    let compiled = CompiledRulePackage::compile(&missing, schema, Default::default()).unwrap();
    assert!(
        compiled
            .evaluate(
                &initial_skill_types::owner(),
                &OwnedDefinitionKey::new(PROGRAM).unwrap(),
                &[],
                schema,
                &mut compiled.new_scratch()
            )
            .is_err()
    );
    // No missing producer, unavailable support input, or per-occurrence
    // noSupports flag is converted to a shared definition-level false value.
}
