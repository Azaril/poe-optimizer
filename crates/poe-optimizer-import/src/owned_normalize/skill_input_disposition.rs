//! Shared source-field accounting and exact Pending usage attachment for physical
//! and nonphysical skill occurrences. This is Import syntax/provenance, not usage
//! evaluation or an alternative build model.
use super::*;
use crate::owned_source_actions::{
    ImportReferenceContext, SourceActionInspection, SourceActionSelection,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeferredSourceUsageInput {
    pub field: DeferredSourceUsageField,
    pub value: ValueRecipeInput,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeferredSourceUsageField {
    GemCount,
    GemGlobal1,
    GemGlobal2,
    GroupCount,
    GroupFullDps,
}
impl DeferredSourceUsageField {
    fn source(self) -> (&'static str, bool) {
        match self {
            Self::GemCount => ("count", false),
            Self::GemGlobal1 => ("enableGlobal1", false),
            Self::GemGlobal2 => ("enableGlobal2", false),
            Self::GroupCount => ("groupCount", true),
            Self::GroupFullDps => ("includeInFullDPS", true),
        }
    }
    fn count(self) -> bool {
        matches!(self, Self::GemCount | Self::GroupCount)
    }
}

pub(super) struct CompiledDeferredUsage {
    values: Vec<(DeferredSourceUsageField, ValueRecipe)>,
}
fn invalid<T>(reason: &'static str) -> Result<T> {
    Err(NormalizationError::Policy(reason))
}
fn charge(work: &mut usize, amount: usize, limits: NormalizationLimits) -> Result<()> {
    *work = work
        .checked_add(amount)
        .filter(|v| *v <= limits.max_work)
        .ok_or(NormalizationError::Limit("source disposition work"))?;
    Ok(())
}
fn direct(recipe: &ValueRecipeInput, attribute: &str) -> bool {
    recipe.tiers.len() == 1
        && recipe.tiers[0].selectors.len() == 1
        && recipe.tiers[0].duplicates == DuplicatePolicy::Reject
        && recipe.tiers[0].selectors[0].lane == ValueLane::Attribute
        && recipe.tiers[0].selectors[0].name == attribute
}
impl CompiledDeferredUsage {
    pub(super) fn new<I: DefinitionSchemaIndex>(
        inputs: &[DeferredSourceUsageInput],
        definitions: &I,
        limits: NormalizationLimits,
        work: &mut usize,
    ) -> Result<Self> {
        if inputs.len() != 5 {
            return invalid("source disposition deferred field inventory");
        }
        let mut fields = BTreeSet::new();
        let mut deferred = Vec::new();
        for input in inputs {
            charge(work, 1, limits)?;
            let (attribute, group) = input.field.source();
            let value = &input.value;
            if !fields.insert(input.field)
                || !direct(value, attribute)
                || value.codec.namespace != *definitions.namespace()
                || value.codec.whitespace != crate::owned_value::WhitespacePolicy::Exact
                || !value.numeric_aliases.is_empty()
                || value.missing
                    != if group {
                        MissingValuePolicy::Absent
                    } else {
                        MissingValuePolicy::Pending
                    }
            {
                return invalid("source disposition deferred source recipe");
            }
            if input.field.count() {
                let ValueCodecKind::Quantity { unit, scale, .. } = &value.codec.codec else {
                    return invalid("source disposition count codec");
                };
                let SchemaLookup::Known(unit) = definitions.definition(unit) else {
                    return invalid("source disposition count unit");
                };
                if unit.dimension != UnitDimension::Count
                    || scale.numerator.get() != 1
                    || scale.denominator.get() != 1
                {
                    return invalid("source disposition count unit or scale");
                }
            } else if !matches!(value.codec.codec, ValueCodecKind::Boolean { .. }) {
                return invalid("source disposition Boolean codec");
            }
            deferred.push((input.field, ValueRecipe::new(value.clone(), limits.value)?));
        }
        Ok(Self { values: deferred })
    }
    pub(super) fn prove(
        &self,
        b: &mut Builder<'_, '_>,
        row: &SourceEvidenceRow<'_>,
        group: &SourceEvidenceRow<'_>,
    ) -> Result<bool> {
        self.prove_fields(b, row, group, false)
    }
    /// The source saves absent generated usage inputs as missing or `nil`.
    /// This only accounts for their syntax under a retained Pending usage
    /// obligation; it does not assign a native count, Boolean or default.
    pub(super) fn prove_generated(
        &self,
        b: &mut Builder<'_, '_>,
        row: &SourceEvidenceRow<'_>,
        group: &SourceEvidenceRow<'_>,
    ) -> Result<bool> {
        self.prove_fields(b, row, group, true)
    }
    fn prove_fields(
        &self,
        b: &mut Builder<'_, '_>,
        row: &SourceEvidenceRow<'_>,
        group: &SourceEvidenceRow<'_>,
        generated: bool,
    ) -> Result<bool> {
        // The base inventory proves the intrinsic fields and finite row/group
        // grammar. Group main-action preferences have not gained a converter.
        for name in ["mainActiveSkill", "mainActiveSkillCalcs"] {
            b.charge(group.attributes().len())?;
            if !matches!(source_shape::value(group, name), None | Some("nil" | "1")) {
                return Ok(false);
            }
        }
        for (field, recipe) in &self.values {
            let (attribute, from_group) = field.source();
            let selected = if from_group { group } else { row };
            b.charge(selected.attributes().len().saturating_add(1))?;
            if let Some(value) = selected.attribute(attribute) {
                b.charge(value.raw().len())?;
            }
            if generated
                && !from_group
                && selected
                    .attribute(attribute)
                    .is_none_or(|value| value.decoded().ok() == Some("nil"))
            {
                continue;
            }
            match b.scalar_value(selected, recipe)? {
                ScalarValue::Selected(ParameterValue::Quantity(_)) if field.count() => {}
                ScalarValue::Selected(ParameterValue::Boolean(_)) if !field.count() => {}
                ScalarValue::Absent if from_group => {}
                _ => return Ok(false),
            }
        }
        Ok(true)
    }
}
/// Both contexts must cover every saved descendant and every present selector.
/// The caller separately checks that the target root kind is the expected one.
pub(super) fn account_reference(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    mut resolve: impl FnMut(ImportReferenceContext) -> Result<SourceActionInspection>,
    construction_work: usize,
) -> Result<Option<Vec<SourceOccurrenceId>>> {
    let mut accounted_children = BTreeSet::new();
    let mut accounted_legacy = BTreeSet::new();
    let mut accounted_minion = BTreeSet::new();
    // Preserve the physical V3 short circuit: an unresolved MAIN never traverses
    // CALCS. Charge each reached report before deciding whether to inspect more.
    for context in [ImportReferenceContext::Main, ImportReferenceContext::Calcs] {
        let report = resolve(context)?;
        b.charge(
            report
                .work
                .checked_sub(construction_work)
                .ok_or(NormalizationError::Policy(
                    "source disposition reference work",
                ))?,
        )?;
        if !report.resolved {
            return Ok(None);
        }
        match report.selection {
            SourceActionSelection::Explicit { attribute, .. } => {
                accounted_children.insert(attribute.occurrence);
            }
            SourceActionSelection::Absent => {}
            SourceActionSelection::Pending { .. } => return Ok(None),
        }
        accounted_legacy.extend(report.ignored_legacy_attributes);
        if let Some(minion) = report.minion {
            accounted_minion.extend(minion.actor_attributes);
            match minion.action_selection {
                SourceActionSelection::Explicit { attribute, .. } => {
                    accounted_minion.insert(attribute);
                }
                SourceActionSelection::Absent => {}
                SourceActionSelection::Pending { .. } => return Ok(None),
            }
            accounted_children.extend(minion.accounted_occurrences);
        }
    }
    b.charge(row.children().len().saturating_add(row.attributes().len()))?;
    // Minion maps contain a container and keyed entries. Require the proof
    // to cover every descendant exactly, not merely the top-level map.
    let mut pending = row.children().to_vec();
    let mut descendants = BTreeSet::new();
    while let Some(id) = pending.pop() {
        b.charge(1)?;
        if !descendants.insert(id) {
            return Ok(None);
        }
        let children = b.evidence.rows()[id.ordinal() as usize].children();
        b.charge(children.len())?;
        pending.extend_from_slice(children);
    }
    if descendants != accounted_children {
        return Ok(None);
    }
    for (index, attribute) in row.attributes().iter().enumerate() {
        let origin = crate::owned_source::SourceAttributeRef {
            occurrence: row.occurrence().id(),
            index: index as u32,
        };
        if matches!(
            attribute.origin().name.as_str(),
            "statSetIndex" | "statSetIndexCalcs"
        ) && !accounted_legacy.contains(&origin)
        {
            return Ok(None);
        }
        if matches!(
            attribute.origin().name.as_str(),
            "skillMinion" | "skillMinionCalcs" | "skillMinionSkill" | "skillMinionSkillCalcs"
        ) && !accounted_minion.contains(&origin)
        {
            return Ok(None);
        }
    }
    Ok(Some(accounted_children.into_iter().collect()))
}
pub(super) fn unique_link<T: Copy>(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    select: impl Fn(&OwnedOriginTarget) -> Option<T>,
) -> Result<Option<T>> {
    b.charge(b.origins[source.ordinal() as usize].links.len())?;
    let mut links = b.origins[source.ordinal() as usize]
        .links
        .iter()
        .filter_map(select);
    let first = links.next();
    Ok(if links.next().is_none() { first } else { None })
}
pub(super) fn link_once(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    target: OwnedOriginTarget,
) -> Result<()> {
    b.charge(b.origins[source.ordinal() as usize].links.len())?;
    if !b.origins[source.ordinal() as usize].links.contains(&target) {
        b.link(source, target)?;
    }
    Ok(())
}

/// Caller has already proved the exact physical or Direct source of this Skill.
/// A detached, complete or foreign-preset obligation cannot witness attachment.
pub(super) fn attach_pending_usage(
    b: &mut Builder<'_, '_>,
    source: SourceOccurrenceId,
    group: SourceOccurrenceId,
    reference_children: &[SourceOccurrenceId],
    skill: &SkillDraft,
    preset: &mut SkillPresetDraft,
    gem: Option<GemInstanceId>,
) -> Result<bool> {
    let Some(set) = b.ancestor(source, "SkillSet")? else {
        return Ok(false);
    };
    if b.evidence.rows()[source.ordinal() as usize]
        .occurrence()
        .parent()
        != Some(group)
        || b.evidence.rows()[group.ordinal() as usize]
            .occurrence()
            .parent()
            != Some(set)
        || unique_link(b, set, |link| {
            if let OwnedOriginTarget::SkillPreset(id) = link {
                Some(*id)
            } else {
                None
            }
        })? != Some(preset.id)
    {
        return Ok(false);
    }
    b.charge(preset.skills.members.len())?;
    if preset
        .skills
        .members
        .iter()
        .filter(|id| **id == skill.id)
        .count()
        != 1
    {
        return Ok(false);
    }
    if preset.usage_preferences.is_none() {
        preset.usage_preferences =
            Some(b.closure(source, "usage-preferences-not-converted", vec![])?);
    }
    let DraftListCompletion::Pending { id: issue, code } =
        &preset.usage_preferences.as_ref().unwrap().completion
    else {
        return Ok(false);
    };
    if code.as_str() != "usage-preferences-not-converted" {
        return Ok(false);
    }
    let issue = *issue;
    // A detached or other-preset Pending ID is not a retained obligation.
    // All matching links must remain in this exact source set. This also
    // permits repeated physical occurrences to share the preset inventory.
    let mut issue_sources = Vec::new();
    b.charge(b.origins.len())?;
    for index in 0..b.origins.len() {
        b.charge(b.origins[index].links.len())?;
        if b.origins[index]
            .links
            .contains(&OwnedOriginTarget::Issue(issue))
        {
            issue_sources.push(b.origins[index].source);
        }
    }
    if issue_sources.is_empty() {
        return Ok(false);
    }
    for source in issue_sources {
        if source != set && b.ancestor(source, "SkillSet")? != Some(set) {
            return Ok(false);
        }
    }
    for source in [source, group] {
        link_once(b, source, OwnedOriginTarget::SkillPreset(preset.id))?;
        link_once(b, source, OwnedOriginTarget::Issue(issue))?;
    }
    for source in reference_children {
        if let Some(gem) = gem {
            link_once(b, *source, OwnedOriginTarget::Gem(gem))?;
        }
        link_once(b, *source, OwnedOriginTarget::Skill(skill.id))?;
    }
    Ok(true)
}

/// Find an existing same-preset intent obligation without allocating or attaching one.
pub(super) fn pending_intent_usage(
    b: &mut Builder<'_, '_>,
    set: SourceOccurrenceId,
    preset: &SkillPresetDraft,
) -> Result<Option<DraftIssueId>> {
    let Some(intent) = &preset.intent else {
        return Ok(None);
    };
    pending_preset_issue(
        b,
        set,
        preset.id,
        &intent.usage.completion,
        "usage-preferences-not-converted",
    )
}

/// Reuse an existing attached responsibility; never create a new issue or turn
/// Complete into Pending merely to justify a source disposition.
fn pending_preset_issue(
    b: &mut Builder<'_, '_>,
    set: SourceOccurrenceId,
    preset: SkillPresetId,
    completion: &DraftListCompletion,
    expected_code: &str,
) -> Result<Option<DraftIssueId>> {
    if unique_link(b, set, |link| match link {
        OwnedOriginTarget::SkillPreset(id) => Some(*id),
        _ => None,
    })? != Some(preset)
    {
        return Ok(None);
    }
    let DraftListCompletion::Pending { id, code } = completion else {
        return Ok(None);
    };
    if code.as_str() != expected_code {
        return Ok(None);
    }
    let issue = *id;
    let mut origins = Vec::new();
    b.charge(b.origins.len())?;
    for index in 0..b.origins.len() {
        b.charge(b.origins[index].links.len())?;
        if b.origins[index]
            .links
            .contains(&OwnedOriginTarget::Issue(issue))
        {
            origins.push(b.origins[index].source);
        }
    }
    if origins.is_empty() {
        return Ok(None);
    }
    for source in origins {
        if source != set && b.ancestor(source, "SkillSet")? != Some(set) {
            return Ok(None);
        }
    }
    Ok(Some(issue))
}

/// Archived generated syntax belongs to three independent, still-open
/// inventories. A singleton does not resolve its saved source/slot relationship;
/// the authored-order check must retain this responsibility until it does.
pub(super) fn pending_generated_responsibilities(
    b: &mut Builder<'_, '_>,
    set: SourceOccurrenceId,
    preset: &SkillPresetDraft,
) -> Result<Option<[DraftIssueId; 3]>> {
    let (Some(intent), Some(supports)) = (&preset.intent, &preset.authored_support_order) else {
        return Ok(None);
    };
    let mut issues = Vec::new();
    for (completion, code) in [
        (
            &intent.generated_inputs.completion,
            "generated-skill-inputs-not-converted",
        ),
        (&intent.usage.completion, "usage-preferences-not-converted"),
        (
            &supports.completion,
            "support-origin-discovery-not-converted",
        ),
    ] {
        let Some(issue) = pending_preset_issue(b, set, preset.id, completion, code)? else {
            return Ok(None);
        };
        if issues.contains(&issue) {
            return Ok(None);
        }
        issues.push(issue);
    }
    Ok(Some([issues[0], issues[1], issues[2]]))
}
