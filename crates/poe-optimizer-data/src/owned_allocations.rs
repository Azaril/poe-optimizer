//! Bounded immutable allocation costs and budget declarations.
//! Storage validates exact schemas and explicit uncertainty; it neither computes
//! capacity nor establishes an allocation's access, activation or legality.
use poe_optimizer_core::{
    owned_allocations::*,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_schema::*,
};
use serde::Serialize;
use std::cmp::Ordering;

const DOMAIN: &str = "owned-allocation-rules-v1";
#[derive(Clone, Copy, Debug)]
pub struct AllocationRuleLimits {
    pub max_costs: usize,
    pub max_budgets: usize,
    /// Aggregate pool references in budget declarations.
    pub max_pool_memberships: usize,
    /// Aggregate gaps in budget registry closure and unmapped capacities.
    pub max_gaps: usize,
    pub max_schema_work: usize,
    pub max_wire_bytes: usize,
}
impl Default for AllocationRuleLimits {
    fn default() -> Self {
        Self {
            max_costs: 100_000,
            max_budgets: 100_000,
            max_pool_memberships: 1_000_000,
            max_gaps: 200_000,
            max_schema_work: 4_000_000,
            max_wire_bytes: 64 * 1024 * 1024,
        }
    }
}
impl AllocationRuleLimits {
    pub fn validate(self) -> Result<(), AllocationRuleError> {
        let hard = Self::default();
        for (name, value, maximum) in [
            ("costs", self.max_costs, hard.max_costs),
            ("budgets", self.max_budgets, hard.max_budgets),
            (
                "pool memberships",
                self.max_pool_memberships,
                hard.max_pool_memberships,
            ),
            ("gaps", self.max_gaps, hard.max_gaps),
            ("schema work", self.max_schema_work, hard.max_schema_work),
            ("bytes", self.max_wire_bytes, hard.max_wire_bytes),
        ] {
            if value == 0 || value > maximum {
                return Err(AllocationRuleError::InvalidLimit(name));
            }
        }
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
pub enum AllocationRuleError {
    #[error("invalid allocation rule limit: {0}")]
    InvalidLimit(&'static str),
    #[error("allocation rules exceed {0}")]
    Limit(&'static str),
    #[error("unsupported owned allocation rules version {0}")]
    Version(u32),
    #[error("allocation rules and definition schema bindings disagree")]
    Binding,
    #[error("invalid allocation rule at {path}: {message}")]
    Invalid { path: String, message: &'static str },
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
fn invalid(path: &str, message: &'static str) -> AllocationRuleError {
    AllocationRuleError::Invalid {
        path: path.into(),
        message,
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct AllocationRuleUse {
    pub costs: usize,
    pub budgets: usize,
    pub pool_memberships: usize,
    pub gaps: usize,
    pub schema_work: usize,
}
impl AllocationRuleUse {
    fn check(self, limits: AllocationRuleLimits) -> Result<(), AllocationRuleError> {
        for (name, value, maximum) in [
            ("costs", self.costs, limits.max_costs),
            ("budgets", self.budgets, limits.max_budgets),
            (
                "pool memberships",
                self.pool_memberships,
                limits.max_pool_memberships,
            ),
            ("gaps", self.gaps, limits.max_gaps),
            ("schema work", self.schema_work, limits.max_schema_work),
        ] {
            if value > maximum {
                return Err(AllocationRuleError::Limit(name));
            }
        }
        Ok(())
    }
    fn work(&mut self, n: usize, limits: AllocationRuleLimits) -> Result<(), AllocationRuleError> {
        add(&mut self.schema_work, n, "schema work")?;
        self.check(limits)
    }
}
fn add(value: &mut usize, n: usize, label: &'static str) -> Result<(), AllocationRuleError> {
    *value = value
        .checked_add(n)
        .ok_or(AllocationRuleError::Limit(label))?;
    Ok(())
}
fn preflight(
    input: &AllocationRulesInput,
    limits: AllocationRuleLimits,
) -> Result<AllocationRuleUse, AllocationRuleError> {
    let mut used = AllocationRuleUse {
        costs: input.costs.len(),
        budgets: input.budgets.members.len(),
        ..Default::default()
    };
    used.check(limits)?;
    if let SchemaClosure::Partial { gaps } = &input.budgets.closure {
        add(&mut used.gaps, gaps.len(), "gaps")?;
        used.check(limits)?;
    }
    for budget in &input.budgets.members {
        add(
            &mut used.pool_memberships,
            budget.pools.len(),
            "pool memberships",
        )?;
        if let SchemaState::Unmapped { gaps } = &budget.capacity {
            add(&mut used.gaps, gaps.len(), "gaps")?;
        }
        used.check(limits)?;
    }
    Ok(used)
}
fn compare_gaps(a: &SchemaGap, b: &SchemaGap) -> Ordering {
    let subject = match (&a.subject, &b.subject) {
        (SchemaSubject::Definition(a), SchemaSubject::Definition(b)) => a.cmp(b),
        (SchemaSubject::Slot(a), SchemaSubject::Slot(b)) => a.cmp(b),
        (SchemaSubject::Definition(_), SchemaSubject::Slot(_)) => Ordering::Less,
        (SchemaSubject::Slot(_), SchemaSubject::Definition(_)) => Ordering::Greater,
    };
    subject
        .then(a.facet.cmp(&b.facet))
        .then(a.code.cmp(&b.code))
}
fn canonicalize(input: &mut AllocationRulesInput) {
    input
        .costs
        .sort_by(|a, b| a.node.cmp(&b.node).then(a.pool.cmp(&b.pool)));
    input.budgets.members.sort_by(|a, b| a.id.cmp(&b.id));
    if let SchemaClosure::Partial { gaps } = &mut input.budgets.closure {
        gaps.sort_by(compare_gaps);
    }
    for budget in &mut input.budgets.members {
        budget.pools.sort();
        if let SchemaState::Unmapped { gaps } = &mut budget.capacity {
            gaps.sort_by(compare_gaps);
        }
    }
}
fn known<'a, T>(lookup: SchemaLookup<'a, T>, path: &str) -> Result<&'a T, AllocationRuleError> {
    match lookup {
        SchemaLookup::Known(value) => Ok(value),
        SchemaLookup::Missing => Err(invalid(path, "missing schema")),
        SchemaLookup::Unmapped(_) => Err(invalid(path, "unmapped schema")),
        SchemaLookup::NamespaceMismatch => Err(invalid(path, "foreign namespace")),
        SchemaLookup::InconsistentIndex => Err(invalid(path, "inconsistent schema index")),
    }
}
fn owner_address(owner: &SlotOwnerDefId) -> DefinitionAddress {
    match owner {
        SlotOwnerDefId::Class(id) => DefinitionAddress::Class(id.clone()),
        SlotOwnerDefId::Ascendancy(id) => DefinitionAddress::Ascendancy(id.clone()),
        SlotOwnerDefId::Reward(id) => DefinitionAddress::Reward(id.clone()),
        SlotOwnerDefId::ItemTemplate(id) => DefinitionAddress::ItemTemplate(id.clone()),
        SlotOwnerDefId::Modifier(id) => DefinitionAddress::Modifier(id.clone()),
        SlotOwnerDefId::Gem(id) => DefinitionAddress::Gem(id.clone()),
        SlotOwnerDefId::Skill(id) => DefinitionAddress::Skill(id.clone()),
        SlotOwnerDefId::PassiveNode(id) => DefinitionAddress::PassiveNode(id.clone()),
        SlotOwnerDefId::UsagePolicy(id) => DefinitionAddress::UsagePolicy(id.clone()),
    }
}
fn check_gaps<I: DefinitionSchemaIndex>(
    gaps: &[SchemaGap],
    path: &str,
    index: &I,
    used: &mut AllocationRuleUse,
    limits: AllocationRuleLimits,
) -> Result<(), AllocationRuleError> {
    if gaps.is_empty() {
        return Err(invalid(
            path,
            "partial or unmapped state needs gap evidence",
        ));
    }
    if gaps.windows(2).any(|w| w[0] == w[1]) {
        return Err(invalid(path, "duplicate gap"));
    }
    for gap in gaps {
        used.work(1, limits)?;
        let valid = match &gap.subject {
            SchemaSubject::Definition(id) => {
                id.namespace() == index.namespace()
                    && index
                        .lookup_definition(id)
                        .is_some_and(|row| row.address() == *id)
            }
            SchemaSubject::Slot(id) => {
                used.work(1, limits)?;
                let owner = owner_address(id.declaration());
                id.namespace() == index.namespace()
                    && owner.namespace() == index.namespace()
                    && index
                        .lookup_definition(&owner)
                        .is_some_and(|row| row.address() == owner)
                    && index
                        .lookup_slot(id)
                        .is_some_and(|row| row.address() == *id)
            }
        };
        // Gap subjects need an owned identity, not a fabricated missing target.
        // Registered Unmapped subjects are valid evidence and remain unresolved.
        if !valid {
            return Err(invalid(
                path,
                "missing, foreign or inconsistent gap subject",
            ));
        }
    }
    Ok(())
}
fn validate<I: DefinitionSchemaIndex>(
    input: &AllocationRulesInput,
    index: &I,
    used: &mut AllocationRuleUse,
    limits: AllocationRuleLimits,
) -> Result<(), AllocationRuleError> {
    if input
        .costs
        .windows(2)
        .any(|w| w[0].node == w[1].node && w[0].pool == w[1].pool)
    {
        return Err(invalid("costs", "duplicate node/pool key"));
    }
    if input.budgets.members.windows(2).any(|w| w[0].id == w[1].id) {
        return Err(invalid("budgets", "duplicate budget id"));
    }
    if let SchemaClosure::Partial { gaps } = &input.budgets.closure {
        check_gaps(gaps, "budgets.closure", index, used, limits)?;
    }
    for (i, cost) in input.costs.iter().enumerate() {
        let path = format!("costs[{i}]");
        if cost.points.get() < 0 {
            return Err(invalid(&path, "negative point cost"));
        }
        used.work(1, limits)?;
        let node = known(index.definition(&cost.node), &path)?;
        used.work(1, limits)?;
        known(index.definition(&cost.pool), &path)?;
        // Custom indexes need not store sorted members. Charge the full bounded
        // scan before use; Partial does not grant permission for unlisted pools.
        used.work(node.pools.members.len(), limits)?;
        if !node.pools.members.contains(&cost.pool) {
            return Err(invalid(&path, "pool not declared by node"));
        }
    }
    for (i, budget) in input.budgets.members.iter().enumerate() {
        let path = format!("budgets.members[{i}]");
        if budget.pools.is_empty() {
            return Err(invalid(&path, "budget has no pools"));
        }
        if budget.pools.windows(2).any(|w| w[0] == w[1]) {
            return Err(invalid(&path, "duplicate budget pool"));
        }
        for pool in &budget.pools {
            used.work(1, limits)?;
            known(index.definition(pool), &path)?;
        }
        match &budget.capacity {
            SchemaState::Unmapped { gaps } => check_gaps(gaps, &path, index, used, limits)?,
            SchemaState::Known(capacity) => {
                if matches!(capacity, AllocationCapacity::PerScope { .. })
                    && !matches!(budget.usage, AllocationBudgetUsage::EachScope { .. })
                {
                    return Err(invalid(
                        &path,
                        "per-scope capacity requires EachScope usage",
                    ));
                }
                used.work(1, limits)?;
                let stat = known(index.definition(capacity.stat()), &path)?;
                used.work(stat.targets.len(), limits)?;
                if !stat.targets.contains(&RuleEntityKind::Actor) {
                    return Err(invalid(&path, "capacity stat excludes Actor target"));
                }
                if stat.value != ComputedValueType::Integer {
                    return Err(invalid(&path, "capacity requires exact Integer stat"));
                }
                // The Engine reads final PlayerActor values per actual loadout;
                // neither finality nor uniformity follows from this descriptor.
            }
        }
    }
    Ok(())
}
#[derive(Clone, Debug)]
pub struct OwnedAllocationRules {
    input: AllocationRulesInput,
    identity: OwnedContentDigest,
    canonical: Vec<u8>,
    resources: AllocationRuleUse,
}
impl OwnedAllocationRules {
    pub fn new<I: DefinitionSchemaIndex>(
        mut input: AllocationRulesInput,
        index: &I,
        limits: AllocationRuleLimits,
    ) -> Result<Self, AllocationRuleError> {
        limits.validate()?;
        if input.schema_version != OWNED_ALLOCATION_RULES_VERSION {
            return Err(AllocationRuleError::Version(input.schema_version));
        }
        if input.definitions != *index.identity()
            || input.namespace != *index.namespace()
            || input.definitions.validate().is_err()
        {
            return Err(AllocationRuleError::Binding);
        }
        let mut resources = preflight(&input, limits)?;
        // Bound authored strings/serialization before secondary indexes or buffers.
        digest_owned(DOMAIN, &input, limits.max_wire_bytes)?;
        canonicalize(&mut input);
        validate(&input, index, &mut resources, limits)?;
        let identity = digest_owned(DOMAIN, &input, limits.max_wire_bytes)?;
        let canonical = serde_json::to_vec(&input)?;
        Ok(Self {
            input,
            identity,
            canonical,
            resources,
        })
    }
    pub fn input(&self) -> &AllocationRulesInput {
        &self.input
    }
    pub fn identity(&self) -> &OwnedContentDigest {
        &self.identity
    }
    pub fn resources(&self) -> AllocationRuleUse {
        self.resources
    }
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical
    }
    pub fn verify_bindings<I: DefinitionSchemaIndex>(
        &self,
        index: &I,
    ) -> Result<(), AllocationRuleError> {
        if self.input.definitions != *index.identity() || self.input.namespace != *index.namespace()
        {
            return Err(AllocationRuleError::Binding);
        }
        Ok(())
    }
    pub fn validate_limits(&self, limits: AllocationRuleLimits) -> Result<(), AllocationRuleError> {
        limits.validate()?;
        self.resources.check(limits)?;
        if self.canonical.len() > limits.max_wire_bytes {
            return Err(AllocationRuleError::Limit("bytes"));
        }
        Ok(())
    }
    /// None is an unknown cost, not free allocation or permission to use another pool.
    pub fn cost_for(
        &self,
        node: &PassiveNodeDefId,
        pool: &PointPoolDefId,
    ) -> Option<&AllocationCost> {
        self.input
            .costs
            .binary_search_by(|cost| cost.node.cmp(node).then(cost.pool.cmp(pool)))
            .ok()
            .map(|i| &self.input.costs[i])
    }
}
pub fn decode_allocation_rules<I: DefinitionSchemaIndex>(
    bytes: &[u8],
    index: &I,
    limits: AllocationRuleLimits,
) -> Result<OwnedAllocationRules, AllocationRuleError> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(AllocationRuleError::Limit("bytes"));
    }
    OwnedAllocationRules::new(serde_json::from_slice(bytes)?, index, limits)
}
pub fn encode_allocation_rules(
    package: &OwnedAllocationRules,
    limits: AllocationRuleLimits,
) -> Result<Vec<u8>, AllocationRuleError> {
    package.validate_limits(limits)?;
    Ok(package.canonical.clone())
}
