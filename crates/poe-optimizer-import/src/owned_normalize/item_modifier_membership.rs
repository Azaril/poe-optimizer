//! Opt-in physical member inventories. This does not close item parameters,
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
    /// Exact category censuses are independent of the retained legacy domains.
    PobFreshOrdinaryMemberCensusV3 {
        definitions: DataIdentity,
        item_lines: OwnedContentDigest,
        item_source: OwnedContentDigest,
        templates: Vec<OrdinarySingletonBase>,
        modifier_rules: Vec<OwnedDefinitionKey>,
        paired_templates: Vec<OrdinaryImplicitExplicitBase>,
        census_templates: Vec<OrdinaryMemberCensusBase>,
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
            }
            | Self::PobFreshOrdinaryMemberCensusV3 {
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
            Self::PobFreshOrdinaryMemberCensusV3 {
                templates,
                paired_templates,
                census_templates,
                ..
            } => templates
                .len()
                .saturating_add(paired_templates.len())
                .saturating_add(census_templates.len()),
        }
    }

    pub(super) fn admits_construction(
        &self,
        template: &ItemTemplateDefId,
        kind: ModifierConstructionKind,
    ) -> bool {
        match self {
            Self::PobFreshOrdinarySingletonV1 { templates, .. } => {
                kind == ModifierConstructionKind::SingletonV1
                    && templates.iter().any(|v| &v.template == template)
            }
            Self::PobFreshOrdinaryImplicitExplicitV2 {
                templates,
                paired_templates,
                ..
            } => {
                if kind == ModifierConstructionKind::PairV2 {
                    paired_templates.iter().any(|v| &v.template == template)
                } else {
                    kind == ModifierConstructionKind::SingletonV1
                        && templates.iter().any(|v| &v.template == template)
                }
            }
            Self::PobFreshOrdinaryMemberCensusV3 {
                templates,
                paired_templates,
                census_templates,
                ..
            } => match kind {
                ModifierConstructionKind::SingletonV1 => {
                    templates.iter().any(|v| &v.template == template)
                }
                ModifierConstructionKind::PairV2 => {
                    paired_templates.iter().any(|v| &v.template == template)
                }
                ModifierConstructionKind::DeclaredV3 => {
                    census_templates.iter().any(|v| &v.template == template)
                }
            },
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrdinaryMemberCensusBase {
    pub template: ItemTemplateDefId,
    pub generated_members: OrdinaryMemberGeneration,
    pub implicit_members: usize,
    pub explicit_members: usize,
}

/// Reviewed absence of other source-generated physical member categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrdinaryMemberGeneration {
    NoBuffEnchantRuneOrClassMembers,
}

const MAX_CENSUS_MEMBERS: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ModifierConstructionKind {
    SingletonV1,
    PairV2,
    DeclaredV3,
}

#[derive(Clone)]
pub(super) struct ModifierConstructionEvidence {
    pub kind: ModifierConstructionKind,
    pub implicit_count: usize,
    pub explicit_count: usize,
    pub socket_capacity: usize,
    pub imported: Option<imported_item_construction::ImportedConstructionEvidence>,
}

pub(super) struct CompiledItemModifierMembership<'p> {
    templates: BTreeSet<&'p ItemTemplateDefId>,
    paired_templates: BTreeSet<&'p ItemTemplateDefId>,
    census_templates: BTreeMap<&'p ItemTemplateDefId, &'p OrdinaryMemberCensusBase>,
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
    let (identity, templates, modifier_rules, paired_templates, census_templates) = match input {
        ItemModifierMembershipPolicy::PobFreshOrdinarySingletonV1 {
            definitions,
            templates,
            modifier_rules,
            ..
        } => (definitions, templates, modifier_rules, &[][..], &[][..]),
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
            &[][..],
        ),
        ItemModifierMembershipPolicy::PobFreshOrdinaryMemberCensusV3 {
            definitions,
            templates,
            modifier_rules,
            paired_templates,
            census_templates,
            ..
        } => (
            definitions,
            templates,
            modifier_rules,
            paired_templates.as_slice(),
            census_templates.as_slice(),
        ),
    };
    if identity != definitions.identity() {
        return Err(NormalizationError::Binding);
    }
    if templates
        .len()
        .saturating_add(paired_templates.len())
        .saturating_add(census_templates.len())
        > 4096
        || modifier_rules.len() > 4096
    {
        return Err(NormalizationError::Limit(
            "item modifier membership policy rows",
        ));
    }
    let Some(augment_policy) = &policy.equipment_membership else {
        return Err(NormalizationError::Policy(
            "item modifier membership needs ordinary augment proof",
        ));
    };
    let augment_bases = augment_policy.templates();
    let mut compiled = CompiledItemModifierMembership {
        templates: BTreeSet::new(),
        paired_templates: BTreeSet::new(),
        census_templates: BTreeMap::new(),
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
    for row in census_templates {
        // Bound the full finite census before storing a row or allocating an order.
        let count = row
            .implicit_members
            .checked_add(row.explicit_members)
            .filter(|v| (1..=MAX_CENSUS_MEMBERS).contains(v))
            .ok_or(NormalizationError::Policy(
                "item modifier member census counts",
            ))?;
        charge(
            &mut compiled.work,
            row.template
                .key()
                .as_str()
                .len()
                .saturating_add(augment_bases.len())
                .saturating_add(count)
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
            || compiled.paired_templates.contains(&row.template)
            || compiled
                .census_templates
                .insert(&row.template, row)
                .is_some()
        {
            return Err(NormalizationError::Policy(
                "item modifier member census base facts",
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
        }
        | ItemModifierMembershipPolicy::PobFreshOrdinaryMemberCensusV3 {
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
        // the member proof. Each admitted source line emits exactly one member.
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
    template: ItemTemplateDefId,
    construction: ModifierConstructionEvidence,
    contributions: Vec<(usize, usize)>,
    numeric_transforms: Option<Vec<(usize, usize)>>,
}

pub(super) struct ModifierSourceOrders<'a> {
    pub contributions: &'a [(usize, usize)],
    pub numeric_transforms: Option<&'a [(usize, usize)]>,
}

impl ModifierMembershipProof {
    pub(super) fn construction_for(
        &self,
        source: SourceOccurrenceId,
        template: &ItemTemplateDefId,
    ) -> Option<&ModifierConstructionEvidence> {
        (self.source == source && &self.template == template).then_some(&self.construction)
    }

    pub(super) fn orders_for(
        &self,
        source: SourceOccurrenceId,
        template: &ItemTemplateDefId,
    ) -> Option<ModifierSourceOrders<'_>> {
        if self.source != source || &self.template != template {
            return None;
        }
        Some(ModifierSourceOrders {
            contributions: &self.contributions,
            numeric_transforms: self.numeric_transforms.as_deref(),
        })
    }
}

/// Source producers are discovered while reading the admitted physical text,
/// before members are regrouped by category. This proof is independent of the
/// contribution sequence: it joins exact source positions, never owned IDs or
/// converter record storage. Call only after the complete physical census and
/// fresh lifecycle have been checked.
fn numeric_transform_order(
    layout: &ItemLayoutStatus,
    lines: &[ItemAttributedLine],
    contributions: &[(usize, usize)],
) -> Option<Vec<(usize, usize)>> {
    let expected: BTreeSet<_> = contributions.iter().copied().collect();
    if expected.len() != contributions.len() || !matches!(layout, ItemLayoutStatus::Proven) {
        return None;
    }
    let mut discovery = BTreeMap::new();
    for line in lines {
        let Some(member) = line.member else { continue };
        let position = (line.index, 0);
        if line.presentation
            || !line.blockers.is_empty()
            || member.line != line.index
            || !expected.contains(&position)
            || discovery.insert(line.index, position).is_some()
        {
            return None;
        }
    }
    (discovery.len() == expected.len()).then(|| discovery.into_values().collect())
}

fn charge_template_copy(b: &mut Builder<'_, '_>, template: &ItemTemplateDefId) -> Result<()> {
    b.charge(
        template
            .key()
            .as_str()
            .len()
            .saturating_add(template.namespace().game().as_str().len())
            .saturating_add(template.namespace().version().as_str().len()),
    )
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
                    .saturating_add(self.census_templates.len())
                    .saturating_add(1),
            ),
        )?;
        if self.paired_templates.contains(template) {
            return self.prove_pair(b, row, attribution, converted, context, template);
        }
        if let Some(census) = self.census_templates.get(template) {
            return self.prove_census(b, row, attribution, converted, context, census);
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
        if fresh.imported.is_some() {
            return Ok(None);
        }
        charge_template_copy(b, template)?;
        b.charge(report.lines.len().saturating_add(1))?;
        Ok(Some(ModifierMembershipProof {
            source: row.occurrence().id(),
            template: template.clone(),
            construction: ModifierConstructionEvidence {
                kind: ModifierConstructionKind::SingletonV1,
                implicit_count: 0,
                explicit_count: 1,
                socket_capacity: fresh.socket_capacity,
                imported: None,
            },
            contributions: vec![(modifier.line, modifier.emission)],
            numeric_transforms: numeric_transform_order(
                &report.layout,
                &report.lines,
                &[(modifier.line, modifier.emission)],
            ),
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
        if fresh.imported.is_some() {
            return Ok(None);
        }
        charge_template_copy(b, template)?;
        b.charge(report.lines.len().saturating_add(2))?;
        Ok(Some(ModifierMembershipProof {
            source: row.occurrence().id(),
            template: template.clone(),
            construction: ModifierConstructionEvidence {
                kind: ModifierConstructionKind::PairV2,
                implicit_count: 1,
                explicit_count: 1,
                socket_capacity: fresh.socket_capacity,
                imported: None,
            },
            contributions: vec![implicit, explicit],
            numeric_transforms: numeric_transform_order(
                &report.layout,
                &report.lines,
                &[implicit, explicit],
            ),
        }))
    }

    fn prove_census(
        &self,
        b: &mut Builder<'_, '_>,
        row: &SourceEvidenceRow<'_>,
        attribution: &ItemRangeAttribution,
        converted: &ItemTextConversion<'_>,
        context: ItemModifierProofContext<'_, '_>,
        census: &OrdinaryMemberCensusBase,
    ) -> Result<Option<ModifierMembershipProof>> {
        let count = census
            .implicit_members
            .checked_add(census.explicit_members)
            .filter(|v| (1..=MAX_CENSUS_MEMBERS).contains(v))
            .ok_or(NormalizationError::Policy(
                "item modifier member census counts",
            ))?;
        let report = attribution.report();
        b.charge(
            report
                .lines
                .len()
                .saturating_add(converted.modifiers.len())
                .saturating_add(converted.issues.len())
                .saturating_add(count),
        )?;
        if converted.modifiers.len() != count {
            return Ok(None);
        }
        let mut ordered = vec![None; count];
        for line in &report.lines {
            let Some(member) = line.member else { continue };
            let (offset, expected) = match member.category {
                SourceModifierCategory::Implicit => (0, census.implicit_members),
                SourceModifierCategory::Explicit => {
                    (census.implicit_members, census.explicit_members)
                }
                _ => return Ok(None),
            };
            if member.ordinal == 0
                || member.ordinal > expected
                || member.line != line.index
                || line.presentation
                || !line.blockers.is_empty()
            {
                return Ok(None);
            }
            let position = offset + member.ordinal - 1;
            if ordered[position].is_some() {
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
                    .saturating_add(converted.lines.len())
                    .saturating_add(count),
            )?;
            let mut matches = converted
                .modifiers
                .iter()
                .filter(|m| m.line == line.index && m.emission == 0);
            let Some(modifier) = matches.next() else {
                return Ok(None);
            };
            if matches.next().is_some()
                || modifier.rolls_closure != SchemaClosure::Complete
                || ordered
                    .iter()
                    .flatten()
                    .any(|v| *v == (modifier.line, modifier.emission))
            {
                return Ok(None);
            }
            b.charge(modifier.rolls.len())?;
            let mut outcomes = converted.lines.iter().filter(|v| v.index == line.index);
            let Some(outcome) = outcomes.next() else {
                return Ok(None);
            };
            if outcomes.next().is_some() {
                return Ok(None);
            }
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
        if ordered.iter().any(Option::is_none) {
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
            &census.template,
            context.augments,
        )?
        else {
            return Ok(None);
        };
        charge_template_copy(b, &census.template)?;
        b.charge(report.lines.len().saturating_add(count))?;
        let contributions: Vec<_> = ordered.into_iter().flatten().collect();
        let numeric_transforms =
            numeric_transform_order(&report.layout, &report.lines, &contributions);
        Ok(Some(ModifierMembershipProof {
            source: row.occurrence().id(),
            template: census.template.clone(),
            construction: ModifierConstructionEvidence {
                kind: ModifierConstructionKind::DeclaredV3,
                implicit_count: census.implicit_members,
                explicit_count: census.explicit_members,
                socket_capacity: fresh.socket_capacity,
                imported: fresh.imported,
            },
            contributions,
            numeric_transforms,
        }))
    }
}

#[cfg(test)]
mod order_tests {
    use super::*;

    fn member(index: usize, category: SourceModifierCategory) -> ItemAttributedLine {
        ItemAttributedLine {
            index,
            decoded_span: index..index + 1,
            raw: "fixture member".into(),
            semantic_text: "fixture member".into(),
            rule: Some(key("fixture-member")),
            presentation: false,
            pending_candidates: vec![],
            member: Some(SourceModifierSlot {
                category,
                ordinal: 1,
                line: index,
            }),
            blockers: vec![],
            range: ItemRangeDecision::Absent,
            property_tokens: vec![],
            flag_tokens: vec![],
            properties: BTreeMap::new(),
        }
    }

    #[test]
    fn cross_category_discovery_does_not_reuse_the_contribution_sequence_or_storage() {
        let mut lines = vec![
            member(4, SourceModifierCategory::Explicit),
            member(9, SourceModifierCategory::Implicit),
        ];
        let contributions = [(9, 0), (4, 0)];
        let expected = Some(vec![(4, 0), (9, 0)]);
        assert_eq!(
            numeric_transform_order(&ItemLayoutStatus::Proven, &lines, &contributions),
            expected
        );
        lines.reverse();
        assert_eq!(
            numeric_transform_order(&ItemLayoutStatus::Proven, &lines, &contributions),
            expected
        );
        assert_eq!(contributions, [(9, 0), (4, 0)]);
    }

    #[test]
    fn discovery_requires_complete_exact_source_correspondence_without_order_fallback() {
        let original = vec![
            member(4, SourceModifierCategory::Explicit),
            member(9, SourceModifierCategory::Implicit),
        ];
        let contributions = [(9, 0), (4, 0)];
        for mutation in [
            "pending-layout",
            "unsupported-layout",
            "missing",
            "extra",
            "duplicate",
            "position",
            "blocked",
            "presentation",
        ] {
            let mut lines = original.clone();
            let mut layout = ItemLayoutStatus::Proven;
            match mutation {
                "pending-layout" => {
                    layout = ItemLayoutStatus::Pending(vec![ItemSourceProblem::UnknownMember])
                }
                "unsupported-layout" => {
                    layout = ItemLayoutStatus::Unsupported(vec![
                        ItemSourceProblem::UnsupportedContentOrder,
                    ])
                }
                "missing" => {
                    lines.pop();
                }
                "extra" => lines.push(member(12, SourceModifierCategory::Explicit)),
                "duplicate" => lines.push(lines[0].clone()),
                "position" => lines[0].member.as_mut().unwrap().line = 3,
                "blocked" => lines[0].blockers.push(ItemSourceProblem::UnsupportedTag),
                "presentation" => lines[0].presentation = true,
                _ => unreachable!(),
            }
            assert_eq!(
                numeric_transform_order(&layout, &lines, &contributions),
                None,
                "{mutation}"
            );
        }
        for positions in [vec![(4, 0), (4, 0)], vec![(4, 1), (9, 0)], vec![(4, 0)]] {
            assert_eq!(
                numeric_transform_order(&ItemLayoutStatus::Proven, &original, &positions),
                None
            );
        }
    }
}
