//! Config-scoped transport into typed Scenario intent. This module neither
//! resolves Skill recipients nor completes an imported usage inventory.
use super::*;
use crate::owned_configuration_numeric::{
    ConfigurationNumericDisposition, ConfigurationNumericError, ConfigurationNumericOutcome,
    ConfigurationNumericRecipe,
};
use configuration_dispositions::{
    CheckedConfigurationReceipt, ConfigurationAdapterControl, ConfigurationSourceState,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationUsageIntentInput {
    pub policy: UsagePolicyDefId,
    pub parameters: Vec<ConfigurationUsageParameterInput>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationUsageParameterInput {
    pub slot: DeclaredSlot<ParameterSlotDefId>,
    pub recipe: ValueRecipeInput,
    pub selection: ConfigurationNumericSelection,
}

#[derive(Default)]
pub(super) struct CompiledConfigurationUsage<'p> {
    rows: Vec<BoundConfigurationUsage<'p>>,
}
struct BoundConfigurationUsage<'p> {
    input: &'p ConfigurationUsageIntentInput,
    parameters: Vec<BoundConfigurationParameter<'p>>,
}
struct BoundConfigurationParameter<'p> {
    input: &'p ConfigurationUsageParameterInput,
    recipe: ConfigurationNumericRecipe,
    schema: ValueSchema,
}
pub(in crate::owned_normalize) struct ProvenConfigurationUsage {
    scope: SourceOccurrenceId,
    rows: Vec<ProvenUsage>,
}
struct ProvenUsage {
    intent: ScenarioUsageIntent,
    parameters: Vec<ProvenParameter>,
}
struct ProvenParameter {
    slot: DeclaredSlot<ParameterSlotDefId>,
    sources: Vec<ProvenSource>,
}
struct ProvenSource {
    origin: SourceOccurrenceId,
    selector: ValueSelector,
    selected: bool,
}

fn numeric_error(error: ConfigurationNumericError) -> NormalizationError {
    match error {
        ConfigurationNumericError::Value(error) => NormalizationError::Value(error),
        ConfigurationNumericError::Invalid(_) => {
            NormalizationError::Policy("configuration usage numeric recipe")
        }
    }
}

pub(super) fn compile<'p, I: DefinitionSchemaIndex>(
    rows: &'p [ConfigurationUsageIntentInput],
    definitions: &I,
    work: &mut usize,
    limits: NormalizationLimits,
) -> Result<CompiledConfigurationUsage<'p>> {
    if rows.len() > 256 || rows.len() > limits.draft.input.max_collection_entries {
        return invalid("configuration usage rows");
    }
    charge(work, rows.len(), limits)?;
    let mut policies = BTreeSet::new();
    let mut result = CompiledConfigurationUsage::default();
    for input in rows {
        if !policies.insert(&input.policy) {
            return invalid("duplicate configuration usage policy");
        }
        let schema = policy_schema(
            definitions,
            &input.policy,
            input.parameters.len(),
            work,
            limits,
        )?;
        if schema.scenario_skill_applicability.is_none() {
            return invalid("configuration usage expansion authority");
        }
        let mut slots = BTreeSet::new();
        let mut parameters = Vec::new();
        for parameter in &input.parameters {
            if !slots.insert(&parameter.slot) {
                return invalid("duplicate configuration usage parameter");
            }
            let value = parameter_schema(
                definitions,
                &input.policy,
                schema,
                &parameter.slot,
                work,
                limits,
            )?;
            let compatible = match (&parameter.recipe.codec.codec, &value.value) {
                (ValueCodecKind::Integer { .. }, ValueSchema::Integer(_)) => true,
                (ValueCodecKind::Quantity { unit, .. }, ValueSchema::Quantity(range)) => {
                    unit == range.minimum.unit()
                        && unit == range.maximum.unit()
                        && matches!(definitions.definition(unit), SchemaLookup::Known(_))
                }
                _ => false,
            };
            if !compatible
                || parameter.recipe.codec.namespace != *definitions.namespace()
                || parameter.recipe.codec.whitespace != WhitespacePolicy::Exact
                || !matches!(parameter.recipe.missing, MissingValuePolicy::Pending)
                || !parameter.recipe.numeric_aliases.is_empty()
            {
                return invalid("configuration usage typed numeric recipe");
            }
            charge(work, parameter.recipe.tiers.len(), limits)?;
            for tier in &parameter.recipe.tiers {
                charge(work, tier.selectors.len(), limits)?;
                for selector in &tier.selectors {
                    charge(work, selector.name.len(), limits)?;
                    if selector.name.is_empty()
                        || selector.name.trim_ascii() != selector.name
                        || selector.name.chars().any(char::is_control)
                    {
                        return invalid("configuration usage selector");
                    }
                }
            }
            let recipe = ConfigurationNumericRecipe::new(
                parameter.recipe.clone(),
                parameter.selection,
                limits.value,
            )
            .map_err(numeric_error)?;
            parameters.push(BoundConfigurationParameter {
                input: parameter,
                recipe,
                schema: value.value.clone(),
            });
        }
        result
            .rows
            .push(BoundConfigurationUsage { input, parameters });
    }
    Ok(result)
}

fn lane(row: &SourceEvidenceRow<'_>) -> Option<ValueLane> {
    match (
        row.occurrence().name(),
        source_shape::value(row, "number").is_some(),
    ) {
        ("Input", true) => Some(ValueLane::InputNumber),
        ("Placeholder", true) => Some(ValueLane::PlaceholderNumber),
        _ => None,
    }
}

/// Consult declared responsibilities even when no value or output was admitted.
/// Successful proof rows cannot establish that an unrelated consumer is absent.
pub(super) fn declares_selector(
    b: &mut Builder<'_, '_>,
    policy: &CompiledConfigurationUsage<'_>,
    selector: &ValueSelector,
) -> Result<bool> {
    b.charge(policy.rows.len())?;
    for row in &policy.rows {
        b.charge(row.parameters.len())?;
        for parameter in &row.parameters {
            let recipe = parameter.recipe.input();
            b.charge(recipe.tiers.len())?;
            for tier in &recipe.tiers {
                b.charge(tier.selectors.len())?;
                for candidate in &tier.selectors {
                    b.charge(candidate.name.len())?;
                    if candidate == selector {
                        return Ok(true);
                    }
                }
            }
        }
    }
    Ok(false)
}

pub(super) fn collect(
    b: &mut Builder<'_, '_>,
    policy: &CompiledConfigurationUsage<'_>,
) -> Result<BTreeMap<SourceOccurrenceId, ProvenConfigurationUsage>> {
    let mut result = BTreeMap::new();
    if policy.rows.is_empty() {
        return Ok(result);
    }
    let Some(scopes) = source_shape::fresh_config_sets(b)? else {
        return Ok(result);
    };
    let evidence = b.evidence;
    for scope in scopes {
        let children = evidence.rows()[scope.ordinal() as usize].children();
        let mut rows = Vec::new();
        for rule in &policy.rows {
            b.charge(rule.parameters.len())?;
            let mut assignments = Vec::new();
            let mut parameters = Vec::new();
            let mut complete_row = true;
            for parameter in &rule.parameters {
                let selector_count = parameter
                    .input
                    .recipe
                    .tiers
                    .iter()
                    .fold(0_usize, |total, tier| {
                        total.saturating_add(tier.selectors.len())
                    });
                b.charge(selector_count.saturating_mul(children.len().saturating_add(1)))?;
                let selectors: Vec<_> = parameter
                    .input
                    .recipe
                    .tiers
                    .iter()
                    .flat_map(|tier| &tier.selectors)
                    .collect();
                let mut candidates = Vec::new();
                let mut blocked = false;
                for id in children {
                    let child = &evidence.rows()[id.ordinal() as usize];
                    let Some(name) = source_shape::value(child, "name") else {
                        continue;
                    };
                    if !selectors.iter().any(|selector| selector.name == name) {
                        continue;
                    }
                    source_shape::charge_row(b, child)?;
                    let Some(selector) = selectors
                        .iter()
                        .find(|selector| {
                            selector.name == name && lane(child) == Some(selector.lane)
                        })
                        .copied()
                    else {
                        blocked = true;
                        continue;
                    };
                    if candidates.len() >= b.limits.value.max_candidates {
                        return Err(NormalizationError::Limit("configuration usage candidates"));
                    }
                    let Some((index, attribute)) =
                        b.attributes[id.ordinal() as usize].get("number")
                    else {
                        blocked = true;
                        continue;
                    };
                    candidates.push(ValueCandidate {
                        selector,
                        origin: SourceAttributeRef {
                            occurrence: *id,
                            index: *index,
                        },
                        value: match attribute.decoded() {
                            Ok(value) => CandidateValue::Decoded(value),
                            Err(error) => CandidateValue::Unavailable(error),
                        },
                    });
                }
                if blocked {
                    complete_row = false;
                    continue;
                }
                let decision = match parameter
                    .recipe
                    .decide(&candidates, Some(&parameter.schema))
                {
                    Ok(decision) => decision,
                    Err(ConfigurationNumericError::Value(ValuePolicyError::MultipleValues {
                        ..
                    })) => {
                        complete_row = false;
                        continue;
                    }
                    Err(error) => return Err(numeric_error(error)),
                };
                let ConfigurationNumericOutcome::Selected { value } = decision.outcome else {
                    complete_row = false;
                    continue;
                };
                b.charge(decision.numeric.sources.len())?;
                let mut sources = Vec::new();
                for source in decision.numeric.sources {
                    b.charge(candidates.len())?;
                    let Some(candidate) = candidates
                        .iter()
                        .find(|candidate| candidate.origin == source.origin)
                    else {
                        return invalid("configuration usage source decision");
                    };
                    b.charge(candidate.selector.name.len())?;
                    sources.push(ProvenSource {
                        origin: source.origin.occurrence,
                        selector: (*candidate.selector).clone(),
                        selected: source.disposition == ConfigurationNumericDisposition::Selected,
                    });
                }
                assignments.push(ParameterAssignment {
                    slot: parameter.input.slot.clone(),
                    value,
                });
                parameters.push(ProvenParameter {
                    slot: parameter.input.slot.clone(),
                    sources,
                });
            }
            if complete_row {
                assignments.sort_by(|a, b| a.slot.cmp(&b.slot));
                rows.push(ProvenUsage {
                    intent: ScenarioUsageIntent {
                        policy: rule.input.policy.clone(),
                        parameters: assignments,
                    },
                    parameters,
                });
            }
        }
        if !rows.is_empty() {
            b.charge(rows.len())?;
            result.insert(scope, ProvenConfigurationUsage { scope, rows });
        }
    }
    Ok(result)
}

fn exact_scope(links: &[OwnedOriginTarget], scenario: ScenarioPresetId) -> bool {
    let mut scopes = links.iter().filter_map(|link| {
        if let OwnedOriginTarget::ScenarioPreset(id) = link {
            Some(*id)
        } else {
            None
        }
    });
    scopes.next() == Some(scenario) && scopes.next().is_none()
}
fn target(scenario: ScenarioPresetId, policy: &UsagePolicyDefId) -> OwnedOriginTarget {
    OwnedOriginTarget::ScenarioUsageIntent {
        scenario,
        policy: policy.clone(),
    }
}
fn receipt_links_match(
    links: &[OwnedOriginTarget],
    scenario: ScenarioPresetId,
    exact: &OwnedOriginTarget,
) -> bool {
    exact_scope(links, scenario) && links.iter().filter(|link| *link == exact).count() == 1
}
pub(in crate::owned_normalize) fn materialize(
    b: &mut Builder<'_, '_>,
    scope: SourceOccurrenceId,
    scenario: &mut ScenarioPresetDraft,
    proof: Option<&ProvenConfigurationUsage>,
) -> Result<()> {
    let Some(proof) = proof else {
        return Ok(());
    };
    b.charge(b.origins[scope.ordinal() as usize].links.len())?;
    if proof.scope != scope || !exact_scope(&b.origins[scope.ordinal() as usize].links, scenario.id)
    {
        return Ok(());
    }
    if scenario
        .scenario
        .usage_intents
        .members
        .len()
        .saturating_add(proof.rows.len())
        > b.limits.draft.input.max_collection_entries
    {
        return Err(NormalizationError::Limit("configuration usage outputs"));
    }
    for row in &proof.rows {
        b.charge(scenario.scenario.usage_intents.members.len() + row.intent.parameters.len())?;
        if scenario
            .scenario
            .usage_intents
            .members
            .iter()
            .any(|intent| intent.policy.to_resolved().as_ref() == Some(&row.intent.policy))
        {
            return invalid("duplicate materialized configuration usage intent");
        }
        scenario
            .scenario
            .usage_intents
            .members
            .push(row.intent.clone().into());
        let exact = target(scenario.id, &row.intent.policy);
        for origin in std::iter::once(scope).chain(
            row.parameters
                .iter()
                .flat_map(|parameter| parameter.sources.iter().map(|source| source.origin)),
        ) {
            for destination in [
                OwnedOriginTarget::ScenarioPreset(scenario.id),
                exact.clone(),
            ] {
                b.charge(b.origins[origin.ordinal() as usize].links.len())?;
                if !b.origins[origin.ordinal() as usize]
                    .links
                    .contains(&destination)
                {
                    b.link(origin, destination)?;
                }
            }
        }
    }
    Ok(())
}

fn matches_materialized(rows: &[ScenarioUsageIntentDraft], expected: &ScenarioUsageIntent) -> bool {
    let mut matching = rows
        .iter()
        .filter(|row| row.policy.to_resolved().as_ref() == Some(&expected.policy));
    let Some(actual) = matching.next() else {
        return false;
    };
    matching.next().is_none() && actual.to_resolved().as_ref() == Some(expected)
}
pub(in crate::owned_normalize) fn checked_receipts(
    b: &mut Builder<'_, '_>,
    scope: SourceOccurrenceId,
    scenario: &ScenarioPresetDraft,
    proof: Option<&ProvenConfigurationUsage>,
) -> Result<Vec<CheckedConfigurationReceipt>> {
    let Some(proof) = proof else {
        return Ok(vec![]);
    };
    b.charge(b.origins[scope.ordinal() as usize].links.len())?;
    if proof.scope != scope || !exact_scope(&b.origins[scope.ordinal() as usize].links, scenario.id)
    {
        return Ok(vec![]);
    }
    let mut result = Vec::new();
    for row in &proof.rows {
        b.charge(scenario.scenario.usage_intents.members.len())?;
        for actual in &scenario.scenario.usage_intents.members {
            b.charge(actual.parameters.members.len())?;
        }
        b.charge(row.intent.parameters.len())?;
        let exact = target(scenario.id, &row.intent.policy);
        if !matches_materialized(&scenario.scenario.usage_intents.members, &row.intent)
            || !receipt_links_match(
                &b.origins[scope.ordinal() as usize].links,
                scenario.id,
                &exact,
            )
        {
            continue;
        }
        let mut row_receipts = Vec::new();
        let mut valid = true;
        for parameter in &row.parameters {
            b.charge(parameter.sources.len())?;
            for source in &parameter.sources {
                b.charge(
                    b.origins[source.origin.ordinal() as usize]
                        .links
                        .len()
                        .saturating_add(source.selector.name.len()),
                )?;
                let links = &b.origins[source.origin.ordinal() as usize].links;
                if b.evidence.rows()[source.origin.ordinal() as usize]
                    .occurrence()
                    .parent()
                    != Some(scope)
                    || !receipt_links_match(links, scenario.id, &exact)
                {
                    valid = false;
                    break;
                }
                if result.len().saturating_add(row_receipts.len()) >= b.limits.max_origin_links {
                    return Err(NormalizationError::Limit("configuration usage receipts"));
                }
                row_receipts.push(CheckedConfigurationReceipt {
                    scope,
                    origin: Some(source.origin),
                    selector: Some(source.selector.clone()),
                    control: ConfigurationAdapterControl::UsageIntent {
                        policy: row.intent.policy.clone(),
                        parameter: parameter.slot.clone(),
                    },
                    source_state: if source.selected {
                        if source.selector.lane == ValueLane::PlaceholderNumber {
                            ConfigurationSourceState::Placeholder
                        } else {
                            ConfigurationSourceState::Authored
                        }
                    } else {
                        ConfigurationSourceState::UnselectedProjection
                    },
                    target: exact.clone(),
                    additional_targets: vec![OwnedOriginTarget::ScenarioPreset(scenario.id)],
                    source_only: (!source.selected).then(|| key("unselected-config-projection")),
                });
            }
        }
        if valid {
            result.extend(row_receipts);
        }
    }
    Ok(result)
}

pub(in crate::owned_normalize) fn control_matches(
    policy: &NormalizationPolicy,
    selector: &ValueSelector,
    control: &ConfigurationAdapterControl,
    work: &mut usize,
    limits: NormalizationLimits,
) -> Result<bool> {
    let ConfigurationAdapterControl::UsageIntent {
        policy: id,
        parameter,
    } = control
    else {
        return Ok(false);
    };
    let Some(UsageInputPolicy::PobOccurrenceUsageV3 { scenario, .. }) = &policy.usage_inputs else {
        return Ok(false);
    };
    charge(work, scenario.len(), limits)?;
    for row in scenario {
        charge(work, row.parameters.len(), limits)?;
        if &row.policy != id {
            continue;
        }
        for input in &row.parameters {
            if &input.slot != parameter {
                continue;
            }
            for tier in &input.recipe.tiers {
                charge(work, tier.selectors.len(), limits)?;
                if tier.selectors.contains(selector) {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}
pub(in crate::owned_normalize) fn check_projection_destinations(
    policy: &NormalizationPolicy,
    selector: &ValueSelector,
    adapters: &BTreeSet<&ConfigurationAdapterControl>,
    work: &mut usize,
    limits: NormalizationLimits,
) -> Result<()> {
    let Some(UsageInputPolicy::PobOccurrenceUsageV3 { scenario, .. }) = &policy.usage_inputs else {
        return Ok(());
    };
    charge(work, scenario.len(), limits)?;
    for row in scenario {
        charge(work, row.parameters.len(), limits)?;
        for input in &row.parameters {
            for tier in &input.recipe.tiers {
                charge(work, tier.selectors.len(), limits)?;
                if tier.selectors.contains(selector)
                    && !adapters.contains(&ConfigurationAdapterControl::UsageIntent {
                        policy: row.policy.clone(),
                        parameter: input.slot.clone(),
                    })
                {
                    return invalid("configuration usage projection responsibilities");
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance(local: u64) -> InstanceId {
        InstanceId::from_parts(BuildLineage::from_bytes([31; 16]), local).unwrap()
    }
    fn intent() -> ScenarioUsageIntent {
        let namespace = GameVersionNamespace::from_keys(key("test"), key("configuration-usage"));
        let policy = UsagePolicyDefId::new(namespace.clone(), key("stages"));
        ScenarioUsageIntent {
            policy: policy.clone(),
            parameters: vec![ParameterAssignment {
                slot: DeclaredSlot {
                    declaration: SlotOwnerDefId::UsagePolicy(policy),
                    slot: ParameterSlotDefId::new(namespace, key("count")),
                },
                value: ParameterValue::Integer(BoundedInteger::new(0).unwrap()),
            }],
        }
    }

    #[test]
    fn receipt_requires_the_unique_complete_materialized_record() {
        let expected = intent();
        let row: ScenarioUsageIntentDraft = expected.clone().into();
        assert!(matches_materialized(std::slice::from_ref(&row), &expected));
        for mutation in 0..7 {
            let mut rows = vec![row.clone()];
            match mutation {
                0 => rows.clear(),
                1 => rows.push(row.clone()),
                2 => {
                    rows[0].parameters.members[0].value =
                        ParameterValue::Integer(BoundedInteger::new(1).unwrap()).into();
                }
                3 => rows[0].parameters.members.clear(),
                4 => {
                    let extra = rows[0].parameters.members[0].clone();
                    rows[0].parameters.members.push(extra);
                }
                5 => {
                    rows[0].parameters.completion = DraftListCompletion::Pending {
                        id: DraftIssueId::from_instance_id(instance(9)),
                        code: key("unresolved-parameters"),
                    };
                }
                6 => {
                    rows[0].policy = UsagePolicyDefId::new(
                        expected.policy.namespace().clone(),
                        key("different"),
                    )
                    .into();
                }
                _ => unreachable!(),
            }
            assert!(
                !matches_materialized(&rows, &expected),
                "mutation {mutation}"
            );
        }
    }

    #[test]
    fn receipt_requires_exact_scenario_and_intent_source_links() {
        let scenario = ScenarioPresetId::from_instance_id(instance(1));
        let other = ScenarioPresetId::from_instance_id(instance(2));
        let expected = intent();
        let exact = target(scenario, &expected.policy);
        let original = vec![
            OwnedOriginTarget::ScenarioPreset(scenario),
            exact.clone(),
            OwnedOriginTarget::Issue(DraftIssueId::from_instance_id(instance(3))),
        ];
        assert!(receipt_links_match(&original, scenario, &exact));
        for mutation in 0..6 {
            let mut links = original.clone();
            match mutation {
                0 => {
                    links.remove(0);
                }
                1 => {
                    links.remove(1);
                }
                2 => links.push(links[0].clone()),
                3 => links.push(links[1].clone()),
                4 => links[0] = OwnedOriginTarget::ScenarioPreset(other),
                5 => links[1] = target(other, &expected.policy),
                _ => unreachable!(),
            }
            assert!(
                !receipt_links_match(&links, scenario, &exact),
                "mutation {mutation}"
            );
        }
    }
}
