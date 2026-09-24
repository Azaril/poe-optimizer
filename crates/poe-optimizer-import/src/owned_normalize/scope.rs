//! Reviewed source-slot absence is separate from native skill activation.
use super::*;

/// Exact parent-group syntax that proves an authored skill is shared across
/// weapon loadouts. Unlisted or unavailable values never inherit a default.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillScopePolicy {
    pub slot_attribute: String,
    pub shared_slots: Vec<SourceComponent>,
}

pub(super) fn validate(
    policy: Option<&SkillScopePolicy>,
    limits: NormalizationLimits,
) -> Result<()> {
    let Some(policy) = policy else { return Ok(()) };
    if policy.slot_attribute.is_empty() || policy.slot_attribute.len() > 128 {
        return Err(NormalizationError::Policy("skill scope attribute"));
    }
    if policy.shared_slots.len() > 64
        || policy.shared_slots.iter().collect::<BTreeSet<_>>().len() != policy.shared_slots.len()
    {
        return Err(NormalizationError::Policy("skill scope rules"));
    }
    if policy.shared_slots.iter().any(|value| {
        matches!(value, SourceComponent::Text(text) if text.len() > limits.mapping.max_string_bytes)
    }) {
        return Err(NormalizationError::Policy("skill scope source bytes"));
    }
    Ok(())
}

impl Builder<'_, '_> {
    pub(super) fn skill_scope(
        &mut self,
        source: SourceOccurrenceId,
        group: &SourceEvidenceRow<'_>,
        policy: Option<&SkillScopePolicy>,
    ) -> Result<DraftField<LoadoutScope>> {
        let Some(policy) = policy else {
            return self.pending(source, "skill-scope-not-converted");
        };
        if group.occurrence().has_namespace_context() {
            return self.pending(source, "skill-scope-not-converted");
        }
        self.charge(group.attributes().len())?;
        let mut attributes = group.attributes().iter().filter(|attribute| {
            attribute.origin().namespace.is_none()
                && attribute.origin().name == policy.slot_attribute
        });
        let attribute = attributes.next();
        if attributes.next().is_some() {
            return self.pending(source, "skill-scope-not-converted");
        }
        let value = if let Some(attribute) = attribute {
            if attribute.raw().len() > self.limits.mapping.max_string_bytes {
                return Err(NormalizationError::Limit("skill scope selector bytes"));
            }
            let Ok(value) = attribute.decoded() else {
                return self.pending(source, "skill-scope-not-converted");
            };
            self.charge(value.len())?;
            SourceComponent::Text(value.into())
        } else {
            SourceComponent::Missing
        };
        self.charge(policy.shared_slots.len())?;
        if policy.shared_slots.contains(&value) {
            Ok(LoadoutScope::Shared.into())
        } else {
            self.pending(source, "skill-scope-not-converted")
        }
    }
}
