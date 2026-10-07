//! Reviewed field disposition for a manual, nonphysical source occurrence.
//! Raw input completeness does not establish usage, support admission or mechanics.
use super::super::skill_input_disposition::{
    CompiledDeferredUsage, account_reference, attach_pending_usage, unique_link,
};
use super::*;
use crate::owned_source_actions::{
    SourceActionCorrespondence, SourceActionCorrespondenceInput, SourceActionLimits,
    SourceDirectActionRequest,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectSkillInputDisposition {
    pub skill: SkillDefId,
    pub reference_action: SourceActionCorrespondenceInput,
    pub deferred_usage: Vec<DeferredSourceUsageInput>,
    /// Finite reviewed values requiring no authored gameplay input in this scope.
    /// These guards cannot replace raw, activation, usage or reference fields.
    pub inert_fields: Vec<GemInputGuard>,
    pub group_guards: Vec<GemInputGuard>,
}

pub(super) struct CompiledDirectDisposition<'p> {
    row: &'p DirectSkillInputDisposition,
    input: &'p DirectSkillInputRule,
    reference: SourceActionCorrespondence,
    deferred: CompiledDeferredUsage,
    fields: BTreeSet<&'p str>,
    group_fields: BTreeSet<&'p str>,
}

pub(in crate::owned_normalize) struct PendingDirectDisposition {
    source: SourceOccurrenceId,
    group: SourceOccurrenceId,
    skill: SkillDefId,
    reference_children: Vec<SourceOccurrenceId>,
    parameters: Vec<ParameterDraft>,
}

const SOURCE_FIELDS: &[&str] = &[
    "gemId",
    "variantId",
    "skillId",
    "nameSpec",
    "enabled",
    "count",
    "enableGlobal1",
    "enableGlobal2",
    "statSetIndex",
    "statSetIndexCalcs",
    "skillMinion",
    "skillMinionCalcs",
    "skillMinionSkill",
    "skillMinionSkillCalcs",
];
const GROUP_FIELDS: &[&str] = &[
    "source",
    "enabled",
    "mainActiveSkill",
    "mainActiveSkillCalcs",
    "groupCount",
    "includeInFullDPS",
];

fn guards<'a>(
    rows: &'a [GemInputGuard],
    allowed: &[&str],
    fields: &mut BTreeSet<&'a str>,
    work: &mut usize,
    limits: NormalizationLimits,
) -> Result<()> {
    if rows.len() > 64 {
        return invalid("direct disposition guard count");
    }
    for row in rows {
        charge(
            work,
            row.attribute.len().saturating_add(row.allowed.len()),
            limits,
        )?;
        if !allowed.contains(&row.attribute.as_str())
            || !fields.insert(&row.attribute)
            || row.allowed.is_empty()
            || row.allowed.len() > 64
            || row.allowed.iter().collect::<BTreeSet<_>>().len() != row.allowed.len()
        {
            return invalid("direct disposition guard inventory");
        }
        for value in &row.allowed {
            if let SourceComponent::Text(text) = value {
                charge(work, text.len(), limits)?;
                if text.len() > limits.mapping.max_string_bytes {
                    return invalid("direct disposition guard token");
                }
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn compile<'p, I: DefinitionSchemaIndex>(
    row: &'p DirectSkillInputDisposition,
    input: &'p DirectSkillInputRule,
    manual_sources: &[SourceComponent],
    group_attributes: &[&str],
    definitions: &I,
    roles: &OwnedSkillRoleIndex,
    mappings: &OwnedMappingIndex,
    limits: NormalizationLimits,
    work: &mut usize,
) -> Result<CompiledDirectDisposition<'p>> {
    if row.skill != input.skill
        || !row.reference_action.matches_direct(
            &input.gem,
            [
                &input.game_id,
                &input.variant_id,
                &input.skill_id,
                &input.name_spec,
            ],
            &input.skill,
            manual_sources,
        )
    {
        return invalid("direct disposition source binding");
    }
    let SchemaLookup::Known(skill) = definitions.definition(&input.skill) else {
        return invalid("direct disposition skill schema");
    };
    charge(
        work,
        input
            .parameters
            .len()
            .saturating_add(skill.declarations.parameters.members.len()),
        limits,
    )?;
    let supplied: BTreeSet<_> = input.parameters.iter().map(|p| &p.slot).collect();
    // Complete raw source accounting must not omit an already declared required
    // authored input. Projected-only inputs belong to generated occurrences;
    // Partial declaration membership still does not become complete here.
    for slot in &skill.declarations.parameters.members {
        if matches!(definitions.slot(slot), SchemaLookup::Known(schema)
            if schema.presence == SlotPresence::RequiredOnce
                && schema.permits_authored_skill_input())
            && !supplied.contains(slot)
        {
            return invalid("direct disposition missing required authored input");
        }
    }
    let hard = SourceActionLimits::default();
    let reference = SourceActionCorrespondence::new(
        row.reference_action.clone(),
        definitions,
        roles,
        mappings,
        SourceActionLimits {
            max_work: limits.max_work.min(hard.max_work),
            max_wire_bytes: limits.max_policy_bytes.min(hard.max_wire_bytes),
            max_output_bytes: limits.max_policy_bytes.min(hard.max_output_bytes),
            max_map_rows: limits
                .draft
                .input
                .max_collection_entries
                .min(hard.max_map_rows),
            max_stat_sets: limits
                .draft
                .input
                .max_collection_entries
                .min(hard.max_stat_sets),
            value: limits.value,
        },
    )?;
    charge(work, reference.construction_work(), limits)?;
    let deferred = CompiledDeferredUsage::new(&row.deferred_usage, definitions, limits, work)?;
    let mut fields: BTreeSet<_> = SOURCE_FIELDS.iter().copied().collect();
    for parameter in &input.parameters {
        // The enclosing raw-input compiler already checked this direct recipe.
        let name = parameter.value.tiers[0].selectors[0].name.as_str();
        charge(work, name.len().saturating_add(1), limits)?;
        if !fields.insert(name) {
            return invalid("direct disposition overlapping raw input");
        }
    }
    let allowed: Vec<_> = input.attributes.iter().map(String::as_str).collect();
    guards(&row.inert_fields, &allowed, &mut fields, work, limits)?;
    let mut group_fields: BTreeSet<_> = GROUP_FIELDS.iter().copied().collect();
    guards(
        &row.group_guards,
        group_attributes,
        &mut group_fields,
        work,
        limits,
    )?;
    Ok(CompiledDirectDisposition {
        row,
        input,
        reference,
        deferred,
        fields,
        group_fields,
    })
}

impl CompiledDirectDisposition<'_> {
    pub(super) fn deferred_usage(&self) -> &CompiledDeferredUsage {
        &self.deferred
    }
    pub(super) fn selector_adapter(&self) -> &SourceActionCorrespondence {
        &self.reference
    }
    pub(super) fn reference(
        &self,
        b: &mut Builder<'_, '_>,
        row: &SourceEvidenceRow<'_>,
        group: &SourceEvidenceRow<'_>,
    ) -> Result<Option<Vec<SourceOccurrenceId>>> {
        b.charge(
            row.attributes()
                .len()
                .saturating_add(group.attributes().len()),
        )?;
        if row
            .attributes()
            .iter()
            .any(|a| !self.fields.contains(a.origin().name.as_str()))
            || group
                .attributes()
                .iter()
                .any(|a| !self.group_fields.contains(a.origin().name.as_str()))
            || !b.gem_guards_match(row, &self.row.inert_fields)?
            || !b.gem_guards_match(group, &self.row.group_guards)?
            || !self.deferred.prove(b, row, group)?
        {
            return Ok(None);
        }
        let evidence = b.evidence;
        let request = |context| SourceDirectActionRequest {
            skill_use: ImportDirectSkillUseLocator {
                source_sha256: evidence.identity().source_sha256.into(),
                occurrence_ordinal: row.occurrence().id().ordinal(),
                catalog_gem: self.input.gem.clone(),
                expected_skill: self.input.skill.clone(),
            },
            context,
        };
        account_reference(
            b,
            row,
            |context| Ok(self.reference.inspect_direct(evidence, &request(context))?),
            self.reference.construction_work(),
        )
    }
    pub(super) fn capture(
        &self,
        b: &mut Builder<'_, '_>,
        row: &SourceEvidenceRow<'_>,
        group: &SourceEvidenceRow<'_>,
        parameters: &[ParameterDraft],
        reference_children: Vec<SourceOccurrenceId>,
    ) -> Result<Option<PendingDirectDisposition>> {
        b.charge(parameters.len())?;
        if parameters.len() != self.input.parameters.len()
            || parameters.iter().any(|p| {
                !matches!(p.slot, DraftField::Known { .. })
                    || !matches!(p.value, DraftField::Known { .. })
            })
        {
            return Ok(None);
        }
        Ok(Some(PendingDirectDisposition {
            source: row.occurrence().id(),
            group: group.occurrence().id(),
            skill: self.input.skill.clone(),
            reference_children,
            parameters: parameters.to_vec(),
        }))
    }
}

impl PendingDirectDisposition {
    pub(in crate::owned_normalize) fn attach(
        &self,
        b: &mut Builder<'_, '_>,
        skill: &SkillDraft,
        preset: &mut SkillPresetDraft,
    ) -> Result<bool> {
        let DraftAuthoredSkillSource::Direct(DraftField::Known { value }) = &skill.source else {
            return Ok(false);
        };
        let Some(parameters) = &skill.parameters else {
            return Ok(false);
        };
        b.charge(
            parameters
                .members
                .len()
                .saturating_add(self.parameters.len()),
        )?;
        if value != &self.skill
            || parameters.members != self.parameters
            || !matches!(skill.enabled, DraftField::Known { .. })
            || !matches!(skill.scope, DraftField::Known { .. })
            || !matches!(&parameters.completion, DraftListCompletion::Pending { code, .. }
                if code.as_str() == "direct-skill-parameters-not-converted")
            || unique_link(b, self.source, |link| match link {
                OwnedOriginTarget::Skill(id) => Some(*id),
                _ => None,
            })? != Some(skill.id)
        {
            return Ok(false);
        }
        b.charge(b.origins[self.source.ordinal() as usize].links.len())?;
        let DraftListCompletion::Pending { id: issue, .. } = parameters.completion else {
            return Ok(false);
        };
        let origins = &b.origins[self.source.ordinal() as usize].links;
        if origins
            .iter()
            .any(|link| matches!(link, OwnedOriginTarget::Gem(_)))
            || !origins.contains(&OwnedOriginTarget::Issue(issue))
        {
            return Ok(false);
        }
        attach_pending_usage(
            b,
            self.source,
            self.group,
            &self.reference_children,
            skill,
            preset,
            None,
        )
    }
}
