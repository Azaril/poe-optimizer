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
    /// The historical singleton domain is retained independently. Paired bases
    /// admit exactly one implicit and one explicit physical member.
    PobFreshOrdinaryImplicitExplicitV2 {
        definitions: DataIdentity,
        item_lines: OwnedContentDigest,
        item_source: OwnedContentDigest,
        templates: Vec<OrdinarySingletonBase>,
        modifier_rules: Vec<OwnedDefinitionKey>,
        paired_templates: Vec<OrdinaryImplicitExplicitBase>,
    },
}

impl ItemModifierMembershipPolicy {
    pub(crate) fn bindings_mut(
        &mut self,
    ) -> (
        &mut DataIdentity,
        &mut OwnedContentDigest,
        &mut OwnedContentDigest,
    ) {
        match self {
            Self::PobFreshOrdinarySingletonV1 {
                definitions,
                item_lines,
                item_source,
                ..
            }
            | Self::PobFreshOrdinaryImplicitExplicitV2 {
                definitions,
                item_lines,
                item_source,
                ..
            } => (definitions, item_lines, item_source),
        }
    }

    pub(super) fn template_count(&self) -> usize {
        match self {
            Self::PobFreshOrdinarySingletonV1 { templates, .. } => templates.len(),
            Self::PobFreshOrdinaryImplicitExplicitV2 {
                templates,
                paired_templates,
                ..
            } => templates.len().saturating_add(paired_templates.len()),
        }
    }

    pub(super) fn admits_construction(&self, template: &ItemTemplateDefId, paired: bool) -> bool {
        match self {
            Self::PobFreshOrdinarySingletonV1 { templates, .. } => {
                !paired && templates.iter().any(|v| &v.template == template)
            }
            Self::PobFreshOrdinaryImplicitExplicitV2 {
                templates,
                paired_templates,
                ..
            } => {
                if paired {
                    paired_templates.iter().any(|v| &v.template == template)
                } else {
                    templates.iter().any(|v| &v.template == template)
                }
            }
        }
    }
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrdinaryImplicitExplicitBase {
    pub template: ItemTemplateDefId,
    pub generated_members: OrdinaryImplicitExplicitMembers,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrdinaryImplicitExplicitMembers {
    OneImplicitNoBuffEnchantRuneOrClassMembers,
}

pub(super) struct CompiledItemModifierMembership<'p> {
    templates: BTreeSet<&'p ItemTemplateDefId>,
    paired_templates: BTreeSet<&'p ItemTemplateDefId>,
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
    let Some(input) = &policy.item_modifier_membership else {
        return Ok(None);
    };
    let (identity, templates, modifier_rules, paired_templates) = match input {
        ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
            definitions,
            templates,
            modifier_rules,
            ..
        } => (definitions, templates, modifier_rules, &[][..]),
        ItemModifierMembershipPolicy::PobFreshOrdinaryImplicitExplicitV2 {
            definitions,
            templates,
            modifier_rules,
            paired_templates,
            ..
        } => (
            definitions,
            templates,
            modifier_rules,
            paired_templates.as_slice(),
        ),
    };
    if identity != definitions.identity() {
        return Err(NormalizationError::Binding);
    }
    if templates.len().saturating_add(paired_templates.len()) > 4096 || modifier_rules.len() > 4096
    {
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
        paired_templates: BTreeSet::new(),
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
    for row in paired_templates {
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
            || compiled.templates.contains(&row.template)
            || !compiled.paired_templates.insert(&row.template)
        {
            return Err(NormalizationError::Policy(
                "paired item modifier membership base facts",
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
    let Some(
        ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
            item_lines,
            item_source,
            ..
        }
        | ItemModifierMembershipPolicy::PobFreshOrdinaryImplicitExplicitV2 {
            item_lines,
            item_source,
            ..
        },
    ) = &policy.item_modifier_membership
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
pub(super) struct ModifierMembershipProof {
    source: SourceOccurrenceId,
    socket_capacity: usize,
    paired_order: Option<[(usize, usize); 2]>,
}
impl ModifierMembershipProof {
    pub(super) fn capacity_for(&self, source: SourceOccurrenceId, paired: bool) -> Option<usize> {
        (self.source == source && self.paired_order.is_some() == paired)
            .then_some(self.socket_capacity)
    }

    pub(super) fn paired_order_for(
        &self,
        source: SourceOccurrenceId,
    ) -> Option<[(usize, usize); 2]> {
        (self.source == source)
            .then_some(self.paired_order)
            .flatten()
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
    ) -> Result<Option<ModifierMembershipProof>> {
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
            template.key().as_str().len().saturating_mul(
                self.templates
                    .len()
                    .saturating_add(self.paired_templates.len())
                    .saturating_add(1),
            ),
        )?;
        if self.paired_templates.contains(template) {
            return self.prove_pair(b, row, attribution, converted, context, template);
        }
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
        Ok(Some(ModifierMembershipProof {
            source: row.occurrence().id(),
            socket_capacity: fresh.socket_capacity,
            paired_order: None,
        }))
    }

    fn prove_pair(
        &self,
        b: &mut Builder<'_, '_>,
        row: &SourceEvidenceRow<'_>,
        attribution: &ItemRangeAttribution,
        converted: &ItemTextConversion<'_>,
        context: ItemModifierProofContext<'_, '_>,
        template: &ItemTemplateDefId,
    ) -> Result<Option<ModifierMembershipProof>> {
        let report = attribution.report();
        b.charge(
            report
                .lines
                .len()
                .saturating_add(converted.modifiers.len())
                .saturating_add(converted.issues.len()),
        )?;
        if converted.modifiers.len() != 2 {
            return Ok(None);
        }
        let mut ordered = [None, None];
        for line in &report.lines {
            let Some(member) = line.member else { continue };
            let position = match member.category {
                SourceModifierCategory::Implicit => 0,
                SourceModifierCategory::Explicit => 1,
                _ => return Ok(None),
            };
            if line.presentation
                || !line.blockers.is_empty()
                || member.ordinal != 1
                || member.line != line.index
                || ordered[position].is_some()
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
            b.charge(
                converted
                    .modifiers
                    .len()
                    .saturating_add(converted.lines.len()),
            )?;
            let mut matches = converted
                .modifiers
                .iter()
                .filter(|m| m.line == line.index && m.emission == 0);
            let Some(modifier) = matches.next() else {
                return Ok(None);
            };
            if matches.next().is_some() || modifier.rolls_closure != SchemaClosure::Complete {
                return Ok(None);
            }
            b.charge(modifier.rolls.len())?;
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
                || !matches!(emissions.as_slice(), [ConvertedItemEmission::Modifier { definition, rolls, .. }] if definition == &modifier.definition && rolls == &modifier.rolls)
            {
                return Ok(None);
            }
            ordered[position] = Some((modifier.line, modifier.emission));
        }
        let [Some(implicit), Some(explicit)] = ordered else {
            return Ok(None);
        };
        if implicit == explicit {
            return Ok(None);
        }
        for issue in &converted.issues {
            b.charge(issue.lines.len())?;
            if issue.lines.is_empty()
                && issue.problem == ItemTextProblem::RequiredParameterMissing
                && let Some(inputs) = context.inputs
                && inputs.permits_missing_template_parameters(b, converted)?
            {
                continue;
            }
            if issue.problem != ItemTextProblem::SchemaPartial || !issue.lines.is_empty() {
                return Ok(None);
            }
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
        Ok(Some(ModifierMembershipProof {
            source: row.occurrence().id(),
            socket_capacity: fresh.socket_capacity,
            paired_order: Some([implicit, explicit]),
        }))
    }
}
