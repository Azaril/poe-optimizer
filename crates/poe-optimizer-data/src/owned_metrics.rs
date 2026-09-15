//! Bounded immutable metric-to-final-stat declarations. No runtime value lookup.
use poe_optimizer_core::{
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_metrics::*,
    owned_schema::*,
};
use serde::Serialize;
use std::collections::BTreeSet;

const DOMAIN: &str = "owned-metric-mapping-v1";
#[derive(Clone, Copy, Debug)]
pub struct MetricMappingLimits {
    pub max_bindings: usize,
    pub max_schema_work: usize,
    pub max_wire_bytes: usize,
}
impl Default for MetricMappingLimits {
    fn default() -> Self {
        Self {
            max_bindings: 100_000,
            max_schema_work: 2_000_000,
            max_wire_bytes: 64 * 1024 * 1024,
        }
    }
}
impl MetricMappingLimits {
    pub fn validate(self) -> Result<(), MetricMappingError> {
        let hard = Self::default();
        for (name, value, maximum) in [
            ("bindings", self.max_bindings, hard.max_bindings),
            ("schema work", self.max_schema_work, hard.max_schema_work),
            ("bytes", self.max_wire_bytes, hard.max_wire_bytes),
        ] {
            if value == 0 || value > maximum {
                return Err(MetricMappingError::InvalidLimit(name));
            }
        }
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
pub enum MetricMappingError {
    #[error("invalid metric mapping limit: {0}")]
    InvalidLimit(&'static str),
    #[error("metric mapping exceeds {0}")]
    Limit(&'static str),
    #[error("unsupported owned metric mapping version {0}")]
    Version(u32),
    #[error("metric mapping and definition schema bindings disagree")]
    Binding,
    #[error("invalid metric mapping at {path}: {message}")]
    Invalid { path: String, message: &'static str },
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
fn invalid(path: &str, message: &'static str) -> MetricMappingError {
    MetricMappingError::Invalid {
        path: path.to_owned(),
        message,
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct MetricMappingUse {
    pub bindings: usize,
    pub schema_work: usize,
}
impl MetricMappingUse {
    fn check(self, limits: MetricMappingLimits) -> Result<(), MetricMappingError> {
        if self.bindings > limits.max_bindings {
            return Err(MetricMappingError::Limit("bindings"));
        }
        if self.schema_work > limits.max_schema_work {
            return Err(MetricMappingError::Limit("schema work"));
        }
        Ok(())
    }
    fn charge(&mut self, n: usize, limits: MetricMappingLimits) -> Result<(), MetricMappingError> {
        self.schema_work = self
            .schema_work
            .checked_add(n)
            .ok_or(MetricMappingError::Limit("schema work"))?;
        self.check(limits)
    }
}
fn known<'a, T>(lookup: SchemaLookup<'a, T>, path: &str) -> Result<&'a T, MetricMappingError> {
    match lookup {
        SchemaLookup::Known(value) => Ok(value),
        SchemaLookup::Missing => Err(invalid(path, "missing schema")),
        SchemaLookup::Unmapped(_) => Err(invalid(path, "unmapped schema")),
        SchemaLookup::NamespaceMismatch => Err(invalid(path, "foreign namespace")),
        SchemaLookup::InconsistentIndex => Err(invalid(path, "inconsistent schema index")),
    }
}
fn validate<I: DefinitionSchemaIndex>(
    input: &MetricMappingInput,
    index: &I,
    limits: MetricMappingLimits,
) -> Result<MetricMappingUse, MetricMappingError> {
    let mut used = MetricMappingUse {
        bindings: input.bindings.len(),
        schema_work: 0,
    };
    used.check(limits)?;
    let mut keys = BTreeSet::new();
    for (i, binding) in input.bindings.iter().enumerate() {
        let path = format!("bindings[{i}]");
        if !keys.insert((&binding.metric, binding.role)) {
            return Err(invalid(&path, "duplicate metric/role key"));
        }
        used.charge(1, limits)?;
        let metric = known(index.definition(&binding.metric), &path)?;
        let (target_kind, stat_kind, actor_role) = match binding.role {
            MetricBindingRole::PlayerActor => (
                MetricTargetKind::Actor,
                RuleEntityKind::Actor,
                Some(MetricActorRole::Player),
            ),
            MetricBindingRole::OwnedActor => (
                MetricTargetKind::Actor,
                RuleEntityKind::Actor,
                Some(MetricActorRole::Owned),
            ),
            MetricBindingRole::Action => (MetricTargetKind::Action, RuleEntityKind::Action, None),
        };
        used.charge(metric.targets.len(), limits)?;
        if !metric.targets.contains(&target_kind) {
            return Err(invalid(&path, "metric target kind mismatch"));
        }
        if let Some(actor_role) = actor_role {
            used.charge(metric.actor_roles.len(), limits)?;
            if !metric.actor_roles.contains(&actor_role) {
                return Err(invalid(&path, "metric actor role mismatch"));
            }
        }
        if binding.role == MetricBindingRole::PlayerActor {
            used.charge(metric.provider_roles.len(), limits)?;
            if !metric.provider_roles.contains(&ProviderRole::Character) {
                return Err(invalid(
                    &path,
                    "player metric excludes the Character provider",
                ));
            }
        }
        used.charge(1, limits)?;
        known(index.definition(&metric.unit), &path)?;
        used.charge(1, limits)?;
        let stat = known(index.definition(&binding.stat), &path)?;
        used.charge(stat.targets.len(), limits)?;
        if !stat.targets.contains(&stat_kind) {
            return Err(invalid(&path, "stat target kind mismatch"));
        }
        match &stat.value {
            ComputedValueType::Quantity { unit } if unit == &metric.unit => {}
            _ => {
                return Err(invalid(
                    &path,
                    "metric requires an exact Quantity(unit) stat",
                ));
            }
        }
        // Exact unit identity equality above means the Known unit lookup already
        // validates both sides. No integer, boolean or same-dimension conversion.
    }
    Ok(used)
}
/// Canonical declared relationships only. Final-stat availability and the exact
/// actor/action provider are established by the request binder and evaluator.
#[derive(Clone, Debug)]
pub struct OwnedMetricMapping {
    input: MetricMappingInput,
    identity: OwnedContentDigest,
    canonical: Vec<u8>,
    resources: MetricMappingUse,
}
impl OwnedMetricMapping {
    pub fn new<I: DefinitionSchemaIndex>(
        mut input: MetricMappingInput,
        index: &I,
        limits: MetricMappingLimits,
    ) -> Result<Self, MetricMappingError> {
        limits.validate()?;
        if input.schema_version != OWNED_METRIC_MAPPING_VERSION {
            return Err(MetricMappingError::Version(input.schema_version));
        }
        if input.definitions != *index.identity()
            || input.namespace != *index.namespace()
            || input.definitions.validate().is_err()
        {
            return Err(MetricMappingError::Binding);
        }
        // Bound direct-authored input before auxiliary indexes or output buffers.
        digest_owned(DOMAIN, &input, limits.max_wire_bytes)?;
        let resources = validate(&input, index, limits)?;
        input
            .bindings
            .sort_by(|a, b| a.metric.cmp(&b.metric).then(a.role.cmp(&b.role)));
        let identity = digest_owned(DOMAIN, &input, limits.max_wire_bytes)?;
        let canonical = serde_json::to_vec(&input)?;
        Ok(Self {
            input,
            identity,
            canonical,
            resources,
        })
    }
    pub fn input(&self) -> &MetricMappingInput {
        &self.input
    }
    pub fn identity(&self) -> &OwnedContentDigest {
        &self.identity
    }
    pub fn resources(&self) -> MetricMappingUse {
        self.resources
    }
    pub fn verify_bindings<I: DefinitionSchemaIndex>(
        &self,
        index: &I,
    ) -> Result<(), MetricMappingError> {
        if self.input.definitions != *index.identity() || self.input.namespace != *index.namespace()
        {
            return Err(MetricMappingError::Binding);
        }
        Ok(())
    }
    pub fn validate_limits(&self, limits: MetricMappingLimits) -> Result<(), MetricMappingError> {
        limits.validate()?;
        self.resources.check(limits)?;
        if self.canonical.len() > limits.max_wire_bytes {
            return Err(MetricMappingError::Limit("bytes"));
        }
        Ok(())
    }
    /// Exact immutable lookup. None means no declared mapping; it is never zero,
    /// another actor's mapping, a default role, or proof of a complete catalog.
    pub fn stat_for(&self, metric: &MetricDefId, role: MetricBindingRole) -> Option<&StatDefId> {
        self.input
            .bindings
            .binary_search_by(|binding| binding.metric.cmp(metric).then(binding.role.cmp(&role)))
            .ok()
            .map(|i| &self.input.bindings[i].stat)
    }
}
pub fn decode_metric_mapping<I: DefinitionSchemaIndex>(
    bytes: &[u8],
    index: &I,
    limits: MetricMappingLimits,
) -> Result<OwnedMetricMapping, MetricMappingError> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(MetricMappingError::Limit("bytes"));
    }
    OwnedMetricMapping::new(serde_json::from_slice(bytes)?, index, limits)
}
pub fn encode_metric_mapping(
    package: &OwnedMetricMapping,
    limits: MetricMappingLimits,
) -> Result<Vec<u8>, MetricMappingError> {
    package.validate_limits(limits)?;
    Ok(package.canonical.clone())
}
