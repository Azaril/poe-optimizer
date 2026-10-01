//! Opt-in physical singleton inventory. This does not close item parameters,
//! static template declarations, rule owners, or aggregate contributors.
use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemModifierMembershipPolicy {
    PobFreshOrdinarySingletonV1 {
        definitions: DataIdentity,
        item_lines: OwnedContentDigest,
        item_source: OwnedContentDigest,
        templates: Vec<OrdinarySingletonBase>,
        modifier_rules: Vec<OwnedDefinitionKey>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrdinarySingletonBase {
    pub template: ItemTemplateDefId,
    pub generated_members: OrdinaryBaseMembers,
}

/// Reviewed immutable source-base member generation, not a per-build observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrdinaryBaseMembers {
    NoBuffImplicitRuneOrClassMembers,
}

pub(super) struct CompiledItemModifierMembership<'p> {
    templates: BTreeSet<&'p ItemTemplateDefId>,
    rules: BTreeSet<&'p OwnedDefinitionKey>,
    pub work: usize,
}

fn charge(work: &mut usize, n: usize, limits: NormalizationLimits) -> Result<()> {
    *work = work
        .checked_add(n)
        .filter(|v| *v <= limits.max_work)
        .ok_or(NormalizationError::Limit("item modifier membership work"))?;
    Ok(())
}

pub(super) fn validate_base<'p, I: DefinitionSchemaIndex>(
    policy: &'p NormalizationPolicy,
    definitions: &I,
    limits: NormalizationLimits,
) -> Result<Option<CompiledItemModifierMembership<'p>>> {
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
        definitions: identity,
        templates,
        modifier_rules,
        ..
    }) = &policy.item_modifier_membership
    else {
        return Ok(None);
    };
    if identity != definitions.identity() {
        return Err(NormalizationError::Binding);
    }
    if templates.len() > 4096 || modifier_rules.len() > 4096 {
        return Err(NormalizationError::Limit(
            "item modifier membership policy rows",
        ));
    }
    let Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 {
        templates: augment_bases,
        ..
    }) = &policy.equipment_membership
    else {
        return Err(NormalizationError::Policy(
            "item modifier membership needs ordinary augment proof",
        ));
    };
    let mut compiled = CompiledItemModifierMembership {
        templates: BTreeSet::new(),
        rules: BTreeSet::new(),
        work: 0,
    };
    for row in templates {
        charge(
            &mut compiled.work,
            row.template
                .key()
                .as_str()
                .len()
                .saturating_add(augment_bases.len())
                .saturating_add(1),
            limits,
        )?;
        if row.template.namespace() != definitions.namespace()
            || !matches!(
                definitions.definition(&row.template),
                SchemaLookup::Known(_)
            )
            || !augment_bases.iter().any(|v| v.template == row.template)
            || !compiled.templates.insert(&row.template)
        {
            return Err(NormalizationError::Policy(
                "item modifier membership base facts",
            ));
        }
    }
    for rule in modifier_rules {
        charge(
            &mut compiled.work,
            rule.as_str().len().saturating_add(1),
            limits,
        )?;
        if !compiled.rules.insert(rule) {
            return Err(NormalizationError::Policy(
                "duplicate item modifier member rule",
            ));
        }
    }
    Ok(Some(compiled))
}

pub(super) fn compile<'p, I: DefinitionSchemaIndex>(
    policy: &'p NormalizationPolicy,
    definitions: &I,
    items: &OwnedItemLinePolicy,
    source: &ItemSourceLayoutPolicy,
    limits: NormalizationLimits,
) -> Result<Option<CompiledItemModifierMembership<'p>>> {
    let Some(mut result) = validate_base(policy, definitions, limits)? else {
        return Ok(None);
    };
    let Some(ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
        item_lines,
        item_source,
        ..
    }) = &policy.item_modifier_membership
    else {
        unreachable!()
    };
    if item_lines != items.identity() || item_source != source.identity() {
        return Err(NormalizationError::Binding);
    }
    let conditional = match &source.input().dialect {
        ItemSourceDialect::PobExportedSingleTextConditionsV1 {
            single_modifier_conditions,
            ..
        }
        | ItemSourceDialect::PobExportedSingleTextObservationsV1 {
            single_modifier_conditions,
            ..
        }
        | ItemSourceDialect::PobExportedSingleTextCategoriesV1 {
            single_modifier_conditions,
            ..
        } => single_modifier_conditions.as_slice(),
        _ => &[],
    };
    for id in &result.rules {
        charge(
            &mut result.work,
            items
                .input()
                .rules
                .len()
                .saturating_add(source.input().rule_layouts.len())
                .saturating_add(conditional.len())
                .saturating_add(1),
            limits,
        )?;
        let Some(rule) = items.input().rules.iter().find(|v| &v.id == *id) else {
            return Err(NormalizationError::Policy(
                "unknown singleton modifier rule",
            ));
        };
        charge(&mut result.work, rule.emissions.len(), limits)?;
        // Other emissions could carry extra physical state not represented by
        // the singleton member proof. Keep the first domain exactly one.
        if !matches!(rule.emissions.as_slice(), [ItemEmission::Modifier { .. }]) {
            return Err(NormalizationError::Policy(
                "singleton rule must emit one modifier",
            ));
        }
        let role = source
            .input()
            .rule_layouts
            .iter()
            .find(|v| &v.rule == *id)
            .map(|v| v.role);
        if role != Some(ItemRuleSourceRole::SingleModifier)
            && !(role == Some(ItemRuleSourceRole::Unresolved)
                && conditional.iter().any(|v| &v.rule == *id))
        {
            return Err(NormalizationError::Policy("singleton modifier source role"));
        }
    }
    Ok(Some(result))
}

/// Constructed only after exact private source/converted-member joins.
pub(super) struct SingletonProof {
    source: SourceOccurrenceId,
    socket_capacity: usize,
}
impl SingletonProof {
    pub(super) fn capacity_for(&self, source: SourceOccurrenceId) -> Option<usize> {
        (self.source == source).then_some(self.socket_capacity)
    }
}

pub(super) struct ItemModifierProofContext<'a, 'p> {
    pub augments: &'a equipment_membership::CompiledEquipmentMembership<'p>,
    pub ordinary_parent: Option<SourceOccurrenceId>,
    pub inputs: Option<&'a item_parameter_inputs::CompiledItemParameterInputs<'p>>,
}

impl CompiledItemModifierMembership<'_> {
    pub(super) fn prove(
        &self,
        b: &mut Builder<'_, '_>,
        row: &SourceEvidenceRow<'_>,
        attribution: &ItemRangeAttribution,
        converted: &ItemTextConversion<'_>,
        context: ItemModifierProofContext<'_, '_>,
    ) -> Result<Option<SingletonProof>> {
        b.charge(1)?;
        let report = attribution.report();
        if context.ordinary_parent.is_none()
            || row.occurrence().parent() != context.ordinary_parent
            || report.item != row.occurrence().id()
            || !attribution.can_convert_lines()
            || !matches!(report.layout, ItemLayoutStatus::Proven)
        {
            return Ok(None);
        }
        let ItemField::Known {
            value: template, ..
        } = &converted.template
        else {
            return Ok(None);
        };
        b.charge(
            template
                .key()
                .as_str()
                .len()
                .saturating_mul(self.templates.len().saturating_add(1)),
        )?;
        if !self.templates.contains(template) {
            return Ok(None);
        }
        b.charge(
            report
                .lines
                .len()
                .saturating_add(converted.lines.len())
                .saturating_add(converted.modifiers.len())
                .saturating_add(converted.issues.len()),
        )?;
        let mut members = report.lines.iter().filter(|v| v.member.is_some());
        let Some(line) = members.next() else {
            return Ok(None);
        };
        if members.next().is_some()
            || line.presentation
            || !line.blockers.is_empty()
            || line
                .member
                .is_none_or(|v| v.category != SourceModifierCategory::Explicit)
        {
            return Ok(None);
        }
        let Some(rule) = &line.rule else {
            return Ok(None);
        };
        b.charge(
            rule.as_str()
                .len()
                .saturating_mul(self.rules.len().saturating_add(1)),
        )?;
        if !self.rules.contains(rule) {
            return Ok(None);
        }
        let [modifier] = converted.modifiers.as_slice() else {
            return Ok(None);
        };
        b.charge(modifier.rolls.len())?;
        if modifier.line != line.index
            || modifier.emission != 0
            || modifier.rolls_closure != SchemaClosure::Complete
        {
            return Ok(None);
        }
        // Empty-line SchemaPartial can describe remaining template parameter
        // declarations. It is not source member loss and stays visible. Any
        // issue involving this member prevents proof, as do noncoverage errors.
        for issue in &converted.issues {
            b.charge(issue.lines.len())?;
            if issue.lines.is_empty()
                && issue.problem == ItemTextProblem::RequiredParameterMissing
                && let Some(inputs) = context.inputs
                && inputs.permits_missing_template_parameters(b, converted)?
            {
                // This proves only modifier membership. The separate raw-input
                // proof still has to supply every projected value before its
                // physical parameter list can become complete.
                continue;
            }
            if issue.problem != ItemTextProblem::SchemaPartial || !issue.lines.is_empty() {
                return Ok(None);
            }
        }
        let Some(outcome) = converted.lines.iter().find(|v| v.index == line.index) else {
            return Ok(None);
        };
        let ItemLineOutcome::Known {
            rule: converted_rule,
            emissions,
        } = &outcome.outcome
        else {
            return Ok(None);
        };
        if converted_rule != rule
            || !matches!(emissions.as_slice(),[ConvertedItemEmission::Modifier {definition,rolls,..}] if definition==&modifier.definition && rolls==&modifier.rolls)
        {
            return Ok(None);
        }
        let Some(fresh) = equipment_membership::fresh_empty_item_for_template(
            b,
            row.occurrence().id(),
            template,
            context.augments,
        )?
        else {
            return Ok(None);
        };
        Ok(Some(SingletonProof {
            source: row.occurrence().id(),
            socket_capacity: fresh.socket_capacity,
        }))
    }
}
