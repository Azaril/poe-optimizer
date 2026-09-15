//! Bounded storage for explicit owned action/stat routes. No runtime provider inference.
use poe_optimizer_core::{
    owned_build::DeclaredSlot,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_routing::*,
    owned_schema::*,
};
use serde::Serialize;
use std::collections::BTreeSet;

const DOMAIN: &str = "owned-action-routing-v1";
#[derive(Clone, Copy, Debug)]
pub struct RoutingLimits {
    pub max_outputs: usize,
    pub max_routes: usize,
    pub max_gaps: usize,
    pub max_schema_work: usize,
    pub max_wire_bytes: usize,
}
impl Default for RoutingLimits {
    fn default() -> Self {
        Self {
            max_outputs: 100_000,
            max_routes: 200_000,
            max_gaps: 200_000,
            max_schema_work: 2_000_000,
            max_wire_bytes: 64 * 1024 * 1024,
        }
    }
}
impl RoutingLimits {
    pub fn validate(self) -> Result<(), RoutingError> {
        let hard = Self::default();
        for (name, value, maximum) in [
            ("outputs", self.max_outputs, hard.max_outputs),
            ("routes", self.max_routes, hard.max_routes),
            ("gaps", self.max_gaps, hard.max_gaps),
            ("schema work", self.max_schema_work, hard.max_schema_work),
            ("bytes", self.max_wire_bytes, hard.max_wire_bytes),
        ] {
            if value == 0 || value > maximum {
                return Err(RoutingError::InvalidLimit(name));
            }
        }
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
pub enum RoutingError {
    #[error("invalid routing limit: {0}")]
    InvalidLimit(&'static str),
    #[error("routing package exceeds {0}")]
    Limit(&'static str),
    #[error("unsupported owned action routing version {0}")]
    Version(u32),
    #[error("routing package and definition schema bindings disagree")]
    Binding,
    #[error("invalid action routing at {path}: {message}")]
    Invalid { path: String, message: &'static str },
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
fn invalid(path: &str, message: &'static str) -> RoutingError {
    RoutingError::Invalid {
        path: path.to_owned(),
        message,
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct RoutingUse {
    pub outputs: usize,
    pub routes: usize,
    pub gaps: usize,
    pub schema_work: usize,
}
impl RoutingUse {
    fn check(self, limits: RoutingLimits) -> Result<(), RoutingError> {
        for (name, value, maximum) in [
            ("outputs", self.outputs, limits.max_outputs),
            ("routes", self.routes, limits.max_routes),
            ("gaps", self.gaps, limits.max_gaps),
            ("schema work", self.schema_work, limits.max_schema_work),
        ] {
            if value > maximum {
                return Err(RoutingError::Limit(name));
            }
        }
        Ok(())
    }
    fn charge(&mut self, n: usize, limits: RoutingLimits) -> Result<(), RoutingError> {
        add(&mut self.schema_work, n)?;
        self.check(limits)
    }
}
fn add(value: &mut usize, n: usize) -> Result<(), RoutingError> {
    *value = value
        .checked_add(n)
        .ok_or(RoutingError::Limit("aggregate count"))?;
    Ok(())
}
fn known<'a, T>(lookup: SchemaLookup<'a, T>, path: &str) -> Result<&'a T, RoutingError> {
    match lookup {
        SchemaLookup::Known(value) => Ok(value),
        SchemaLookup::Missing => Err(invalid(path, "missing schema")),
        SchemaLookup::Unmapped(_) => Err(invalid(path, "unmapped schema")),
        SchemaLookup::NamespaceMismatch => Err(invalid(path, "foreign namespace")),
        SchemaLookup::InconsistentIndex => Err(invalid(path, "inconsistent schema index")),
    }
}
fn member<T: PartialEq>(
    value: &T,
    set: &DeclaredSet<T>,
    path: &str,
    used: &mut RoutingUse,
    limits: RoutingLimits,
) -> Result<(), RoutingError> {
    used.charge(set.members.len(), limits)?;
    if !set.members.contains(value) {
        return Err(invalid(path, "selector is not a declared member"));
    }
    Ok(())
}
fn stat<'a, I: DefinitionSchemaIndex>(
    index: &'a I,
    id: &StatDefId,
    kind: RuleEntityKind,
    path: &str,
    used: &mut RoutingUse,
    limits: RoutingLimits,
) -> Result<&'a ComputedValueType, RoutingError> {
    used.charge(1, limits)?;
    let schema = known(index.definition(id), path)?;
    used.charge(schema.targets.len(), limits)?;
    if !schema.targets.contains(&kind) {
        return Err(invalid(path, "stat target kind mismatch"));
    }
    if let ComputedValueType::Quantity { unit } = &schema.value {
        used.charge(1, limits)?;
        known(index.definition(unit), path)?;
    }
    Ok(&schema.value)
}
fn validate<I: DefinitionSchemaIndex>(
    input: &ActionRoutingInput,
    index: &I,
    limits: RoutingLimits,
) -> Result<RoutingUse, RoutingError> {
    let mut used = RoutingUse {
        outputs: input.outputs.len(),
        ..Default::default()
    };
    used.check(limits)?;
    let mut outputs = BTreeSet::new();
    for (i, output) in input.outputs.iter().enumerate() {
        let path = format!("outputs[{i}]");
        if !outputs.insert(&output.output) {
            return Err(invalid(&path, "duplicate output"));
        }
        used.charge(1, limits)?;
        let schema = known(index.slot(&output.output), &path)?;
        add(&mut used.routes, output.routes.members.len())?;
        used.check(limits)?;
        if let SchemaClosure::Partial { gaps } = &output.routes.closure {
            add(&mut used.gaps, gaps.len())?;
            used.check(limits)?;
            if gaps.is_empty() {
                return Err(invalid(&path, "partial routes require gaps"));
            }
            let mut codes = BTreeSet::new();
            let subject = SchemaSubject::Slot(SlotAddress::ActionOutput(output.output.clone()));
            for gap in gaps {
                used.charge(1, limits)?;
                if gap.subject != subject
                    || gap.facet != SchemaFacet::GameRules
                    || !codes.insert(&gap.code)
                {
                    return Err(invalid(&path, "invalid or duplicate route closure gap"));
                }
            }
        }
        let mut ids = BTreeSet::new();
        for (j, route) in output.routes.members.iter().enumerate() {
            let path = format!("outputs[{i}].routes.members[{j}]");
            used.charge(1, limits)?;
            if !ids.insert(&route.id) {
                return Err(invalid(&path, "duplicate route id"));
            }
            if let ActionRouteSelection::Exact(selector) = &route.selection {
                let ActionRouteSelector {
                    part,
                    mode,
                    stat_set,
                } = selector.as_ref();
                used.charge(3, limits)?;
                known(index.definition(part), &path)?;
                known(index.definition(mode), &path)?;
                known(index.definition(stat_set), &path)?;
                member(part, &schema.parts, &path, &mut used, limits)?;
                member(mode, &schema.modes, &path, &mut used, limits)?;
                member(stat_set, &schema.stat_sets, &path, &mut used, limits)?;
            }
            let (source_stat, source_kind) = match &route.source {
                ActionStatRouteSource::PlayerEquipment { slot, stat } => {
                    used.charge(1, limits)?;
                    known(index.definition(slot), &path)?;
                    (stat, RuleEntityKind::EquipmentUse)
                }
                ActionStatRouteSource::ActionActor { stat } => (stat, RuleEntityKind::Actor),
            };
            let source_type = stat(index, source_stat, source_kind, &path, &mut used, limits)?;
            let target_type = stat(
                index,
                &route.target,
                RuleEntityKind::Action,
                &path,
                &mut used,
                limits,
            )?;
            if source_type != target_type {
                return Err(invalid(&path, "source and target type/unit mismatch"));
            }
        }
    }
    Ok(used)
}
/// Canonical immutable declaration package, not proof of any concrete runtime relation.
#[derive(Clone, Debug)]
pub struct OwnedActionRouting {
    input: ActionRoutingInput,
    identity: OwnedContentDigest,
    canonical: Vec<u8>,
    resources: RoutingUse,
}
impl OwnedActionRouting {
    pub fn new<I: DefinitionSchemaIndex>(
        mut input: ActionRoutingInput,
        index: &I,
        limits: RoutingLimits,
    ) -> Result<Self, RoutingError> {
        limits.validate()?;
        if input.schema_version != OWNED_ACTION_ROUTING_VERSION {
            return Err(RoutingError::Version(input.schema_version));
        }
        if input.definitions != *index.identity()
            || input.namespace != *index.namespace()
            || input.definitions.validate().is_err()
        {
            return Err(RoutingError::Binding);
        }
        // Bound direct-authored values before auxiliary indexes or output buffers.
        digest_owned(DOMAIN, &input, limits.max_wire_bytes)?;
        let resources = validate(&input, index, limits)?;
        input.outputs.sort_by(|a, b| a.output.cmp(&b.output));
        for output in &mut input.outputs {
            output.routes.members.sort_by(|a, b| a.id.cmp(&b.id));
            if let SchemaClosure::Partial { gaps } = &mut output.routes.closure {
                gaps.sort_by(|a, b| a.code.cmp(&b.code));
            }
        }
        let identity = digest_owned(DOMAIN, &input, limits.max_wire_bytes)?;
        let canonical = serde_json::to_vec(&input)?;
        Ok(Self {
            input,
            identity,
            canonical,
            resources,
        })
    }
    pub fn input(&self) -> &ActionRoutingInput {
        &self.input
    }
    pub fn identity(&self) -> &OwnedContentDigest {
        &self.identity
    }
    pub fn resources(&self) -> RoutingUse {
        self.resources
    }
    /// Checks immutable package/index identity, not external authenticity or occurrence activity.
    pub fn verify_bindings<I: DefinitionSchemaIndex>(&self, index: &I) -> Result<(), RoutingError> {
        if self.input.definitions != *index.identity() || self.input.namespace != *index.namespace()
        {
            return Err(RoutingError::Binding);
        }
        Ok(())
    }
    pub fn validate_limits(&self, limits: RoutingLimits) -> Result<(), RoutingError> {
        limits.validate()?;
        self.resources.check(limits)?;
        if self.canonical.len() > limits.max_wire_bytes {
            return Err(RoutingError::Limit("bytes"));
        }
        Ok(())
    }
    pub fn routes_for(
        &self,
        output: &DeclaredSlot<ActionOutputDefId>,
    ) -> Option<&DeclaredSet<ActionStatRoute>> {
        self.input
            .outputs
            .binary_search_by(|v| v.output.cmp(output))
            .ok()
            .map(|i| &self.input.outputs[i].routes)
    }
}
pub fn decode_action_routing<I: DefinitionSchemaIndex>(
    bytes: &[u8],
    index: &I,
    limits: RoutingLimits,
) -> Result<OwnedActionRouting, RoutingError> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(RoutingError::Limit("bytes"));
    }
    OwnedActionRouting::new(serde_json::from_slice(bytes)?, index, limits)
}
pub fn encode_action_routing(
    package: &OwnedActionRouting,
    limits: RoutingLimits,
) -> Result<Vec<u8>, RoutingError> {
    package.validate_limits(limits)?;
    Ok(package.canonical.clone())
}
