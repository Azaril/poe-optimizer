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
    pub source_selectors: usize,
    pub named_sources: usize,
    pub source_bindings: usize,
    pub gaps: usize,
    pub schema_work: usize,
}
impl RoutingUse {
    fn check(self, limits: RoutingLimits) -> Result<(), RoutingError> {
        for (name, value, maximum) in [
            ("outputs", self.outputs, limits.max_outputs),
            ("routes", self.routes, limits.max_routes),
            ("source selectors", self.source_selectors, limits.max_routes),
            ("named sources", self.named_sources, limits.max_routes * 3),
            (
                "source bindings",
                self.source_bindings,
                limits.max_routes * 3,
            ),
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
fn selection<I: DefinitionSchemaIndex>(
    selection: &ActionRouteSelection,
    schema: &ActionOutputSchema,
    index: &I,
    path: &str,
    used: &mut RoutingUse,
    limits: RoutingLimits,
) -> Result<(), RoutingError> {
    if let ActionRouteSelection::Exact(selector) = selection {
        used.charge(3, limits)?;
        known(index.definition(&selector.part), path)?;
        known(index.definition(&selector.mode), path)?;
        known(index.definition(&selector.stat_set), path)?;
        member(&selector.part, &schema.parts, path, used, limits)?;
        member(&selector.mode, &schema.modes, path, used, limits)?;
        member(&selector.stat_set, &schema.stat_sets, path, used, limits)?;
    }
    Ok(())
}
fn source_kind(origin: &ActionSourceOrigin) -> RuleEntityKind {
    match origin {
        ActionSourceOrigin::ActionActor => RuleEntityKind::Actor,
        ActionSourceOrigin::CurrentAction => RuleEntityKind::Action,
        ActionSourceOrigin::PlayerEquipment { .. } => RuleEntityKind::EquipmentUse,
    }
}
fn selectors<I: DefinitionSchemaIndex>(
    output: &ActionOutputRoutes,
    version: u32,
    schema: &ActionOutputSchema,
    index: &I,
    path: &str,
    used: &mut RoutingUse,
    limits: RoutingLimits,
) -> Result<(), RoutingError> {
    if version == OWNED_ACTION_ROUTING_V1 {
        if output.source_selectors.is_some() {
            return Err(invalid(path, "v1 cannot declare source selectors"));
        }
        return Ok(());
    }
    let selectors = output
        .source_selectors
        .as_ref()
        .ok_or_else(|| invalid(path, "v2 requires explicit source selectors"))?;
    add(&mut used.source_selectors, selectors.members.len())?;
    used.charge(selectors.members.len(), limits)?;
    if let SchemaClosure::Partial { gaps } = &selectors.closure {
        add(&mut used.gaps, gaps.len())?;
        used.check(limits)?;
        if gaps.is_empty() {
            return Err(invalid(path, "partial selectors require gaps"));
        }
        let expected = SchemaSubject::Slot(SlotAddress::ActionOutput(output.output.clone()));
        let mut codes = BTreeSet::new();
        for gap in gaps {
            used.charge(1, limits)?;
            if gap.subject != expected
                || gap.facet != SchemaFacet::GameRules
                || !codes.insert(&gap.code)
            {
                return Err(invalid(path, "invalid or duplicate selector closure gap"));
            }
        }
    }
    let mut ids = BTreeSet::new();
    for selector in &selectors.members {
        add(&mut used.named_sources, selector.sources.len())?;
        used.charge(selector.sources.len() + 1, limits)?;
        if !ids.insert(&selector.id) {
            return Err(invalid(path, "duplicate source selector"));
        }
        // The finite policy admits at most an equipped source and two alternatives.
        if selector.sources.is_empty() || selector.sources.len() > 3 {
            return Err(invalid(
                path,
                "selector requires one to three reachable sources",
            ));
        }
        selection(&selector.selection, schema, index, path, used, limits)?;
        let mut sources = std::collections::BTreeMap::new();
        for source in &selector.sources {
            if sources.insert(&source.id, &source.origin).is_some() {
                return Err(invalid(path, "duplicate named source"));
            }
            if let ActionSourceOrigin::PlayerEquipment { slot } = &source.origin {
                used.charge(1, limits)?;
                known(index.definition(slot), path)?;
            }
        }
        let mut reachable = BTreeSet::new();
        match &selector.policy {
            ActionSourcePolicy::Fixed { source } => {
                if !matches!(
                    sources.get(source),
                    Some(ActionSourceOrigin::ActionActor | ActionSourceOrigin::CurrentAction)
                ) {
                    return Err(invalid(
                        path,
                        "fixed source requires an actor or current action origin",
                    ));
                }
                reachable.insert(source);
            }
            ActionSourcePolicy::EquipmentEligibility {
                source,
                capability,
                when_empty,
                when_ineligible,
            } => {
                if !matches!(
                    sources.get(source),
                    Some(ActionSourceOrigin::PlayerEquipment { .. })
                ) {
                    return Err(invalid(
                        path,
                        "eligibility source must name player equipment",
                    ));
                }
                used.charge(1, limits)?;
                let capability = known(index.definition(capability), path)?;
                used.charge(capability.targets.len(), limits)?;
                if !capability.targets.contains(&RuleEntityKind::EquipmentUse) {
                    return Err(invalid(
                        path,
                        "eligibility capability must target EquipmentUse",
                    ));
                }
                reachable.insert(source);
                for outcome in [when_empty, when_ineligible] {
                    if let ActionSourceOutcome::Use { source } = outcome {
                        if !matches!(
                            sources.get(source),
                            Some(
                                ActionSourceOrigin::ActionActor | ActionSourceOrigin::CurrentAction
                            )
                        ) {
                            return Err(invalid(
                                path,
                                "alternative source requires an actor or current action origin",
                            ));
                        }
                        reachable.insert(source);
                    }
                }
            }
        }
        if reachable.len() != sources.len() {
            return Err(invalid(
                path,
                "declared sources must exactly match reachable policy outcomes",
            ));
        }
    }
    Ok(())
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
        selectors(
            output,
            input.schema_version,
            schema,
            index,
            &path,
            &mut used,
            limits,
        )?;
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
            selection(&route.selection, schema, index, &path, &mut used, limits)?;
            if let ActionStatRouteSource::Selected { selector, stats } = &route.source {
                add(&mut used.source_bindings, stats.len())?;
                used.charge(stats.len(), limits)?;
                if stats.len() > 3 {
                    return Err(invalid(
                        &path,
                        "selected route admits at most three branch statistics",
                    ));
                }
                let selectors = output
                    .source_selectors
                    .as_ref()
                    .ok_or_else(|| invalid(&path, "selected route requires v2 source selectors"))?;
                used.charge(selectors.members.len(), limits)?;
                let selector = selectors
                    .members
                    .iter()
                    .find(|s| &s.id == selector)
                    .ok_or_else(|| invalid(&path, "unknown source selector"))?;
                if selector.selection != route.selection {
                    return Err(invalid(
                        &path,
                        "route and source selector selections differ",
                    ));
                }
                let target = stat(
                    index,
                    &route.target,
                    RuleEntityKind::Action,
                    &path,
                    &mut used,
                    limits,
                )?;
                if stats.len() != selector.sources.len() {
                    return Err(invalid(&path, "source stat bindings must be exhaustive"));
                }
                let mut seen = BTreeSet::new();
                for binding in stats {
                    if !seen.insert(&binding.source) {
                        return Err(invalid(&path, "duplicate source stat binding"));
                    }
                    let origin = selector
                        .sources
                        .iter()
                        .find(|s| s.id == binding.source)
                        .ok_or_else(|| invalid(&path, "unknown source stat binding"))?;
                    let source = stat(
                        index,
                        &binding.stat,
                        source_kind(&origin.origin),
                        &path,
                        &mut used,
                        limits,
                    )?;
                    if source != target {
                        return Err(invalid(&path, "source and target type/unit mismatch"));
                    }
                }
                continue;
            }
            let (source_stat, source_kind) = match &route.source {
                ActionStatRouteSource::PlayerEquipment { slot, stat } => {
                    used.charge(1, limits)?;
                    known(index.definition(slot), &path)?;
                    (stat, RuleEntityKind::EquipmentUse)
                }
                ActionStatRouteSource::ActionActor { stat } => (stat, RuleEntityKind::Actor),
                ActionStatRouteSource::Selected { .. } => unreachable!("validated above"),
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
        if !matches!(
            input.schema_version,
            OWNED_ACTION_ROUTING_V1 | OWNED_ACTION_ROUTING_VERSION
        ) {
            return Err(RoutingError::Version(input.schema_version));
        }
        if input.definitions != *index.identity()
            || input.namespace != *index.namespace()
            || input.definitions.validate().is_err()
        {
            return Err(RoutingError::Binding);
        }
        // Bound direct-authored values before auxiliary indexes or output buffers.
        digest_owned(
            if input.schema_version == OWNED_ACTION_ROUTING_V1 {
                DOMAIN
            } else {
                "owned-action-routing-v2"
            },
            &input,
            limits.max_wire_bytes,
        )?;
        let resources = validate(&input, index, limits)?;
        input.outputs.sort_by(|a, b| a.output.cmp(&b.output));
        for output in &mut input.outputs {
            output.routes.members.sort_by(|a, b| a.id.cmp(&b.id));
            for route in &mut output.routes.members {
                if let ActionStatRouteSource::Selected { stats, .. } = &mut route.source {
                    stats.sort_by(|a, b| a.source.cmp(&b.source));
                }
            }
            if let Some(selectors) = &mut output.source_selectors {
                selectors.members.sort_by(|a, b| a.id.cmp(&b.id));
                for selector in &mut selectors.members {
                    selector.sources.sort_by(|a, b| a.id.cmp(&b.id));
                }
                if let SchemaClosure::Partial { gaps } = &mut selectors.closure {
                    gaps.sort_by(|a, b| a.code.cmp(&b.code));
                }
            }
            if let SchemaClosure::Partial { gaps } = &mut output.routes.closure {
                gaps.sort_by(|a, b| a.code.cmp(&b.code));
            }
        }
        let identity = digest_owned(
            if input.schema_version == OWNED_ACTION_ROUTING_V1 {
                DOMAIN
            } else {
                "owned-action-routing-v2"
            },
            &input,
            limits.max_wire_bytes,
        )?;
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
    pub fn source_selectors_for(
        &self,
        output: &poe_optimizer_core::owned_build::DeclaredSlot<ActionOutputDefId>,
    ) -> Option<&DeclaredSet<ActionSourceSelector>> {
        self.input
            .outputs
            .binary_search_by(|entry| entry.output.cmp(output))
            .ok()
            .and_then(|i| self.input.outputs[i].source_selectors.as_ref())
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
