//! Bounded deterministic storage for owned rule authoring packages.
//!
//! This validates the envelope, exact schema binding, membership evidence and
//! graph references. Engine compilation separately checks operation semantics,
//! types, scopes and cycles; storage alone is not executable authority.
use poe_optimizer_core::{
    data::DataIdentity,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_rules::*,
    owned_schema::{
        ComputedValueType, DefinitionAddress, DefinitionSchemaIndex, SchemaClosure, SchemaFacet,
        SchemaLookup, SchemaSubject, SlotAddress,
    },
};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug)]
pub struct RuleStorageLimits {
    pub max_receivers: usize,
    pub max_receiver_targets: usize,
    pub max_receiver_work: usize,
    pub max_owners: usize,
    pub max_programs: usize,
    pub max_tables: usize,
    pub max_table_cells: usize,
    pub max_reads: usize,
    pub max_nodes: usize,
    pub max_effects: usize,
    pub max_edges: usize,
    pub max_gaps: usize,
    pub max_wire_bytes: usize,
}
impl Default for RuleStorageLimits {
    fn default() -> Self {
        Self {
            max_receivers: 100_000,
            max_receiver_targets: 1_000_000,
            max_receiver_work: 4_000_000,
            max_owners: 100_000,
            max_programs: 200_000,
            max_tables: 65_536,
            max_table_cells: 2_000_000,
            max_reads: 1_000_000,
            max_nodes: 2_000_000,
            max_effects: 1_000_000,
            max_edges: 8_000_000,
            max_gaps: 200_000,
            max_wire_bytes: 64 * 1024 * 1024,
        }
    }
}
impl RuleStorageLimits {
    pub fn validate(self) -> Result<(), RuleStorageError> {
        let hard = Self::default();
        for (name, actual, maximum) in [
            (
                "receiver work",
                self.max_receiver_work,
                hard.max_receiver_work,
            ),
            ("receivers", self.max_receivers, hard.max_receivers),
            (
                "receiver targets",
                self.max_receiver_targets,
                hard.max_receiver_targets,
            ),
            ("owners", self.max_owners, hard.max_owners),
            ("programs", self.max_programs, hard.max_programs),
            ("tables", self.max_tables, hard.max_tables),
            ("table cells", self.max_table_cells, hard.max_table_cells),
            ("reads", self.max_reads, hard.max_reads),
            ("nodes", self.max_nodes, hard.max_nodes),
            ("effects", self.max_effects, hard.max_effects),
            ("edges", self.max_edges, hard.max_edges),
            ("gaps", self.max_gaps, hard.max_gaps),
            ("bytes", self.max_wire_bytes, hard.max_wire_bytes),
        ] {
            if actual == 0 || actual > maximum {
                return Err(RuleStorageError::InvalidLimit(name));
            }
        }
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
pub enum RuleStorageError {
    #[error("invalid rule storage limit: {0}")]
    InvalidLimit(&'static str),
    #[error("rule storage exceeds {0}")]
    Limit(&'static str),
    #[error("unsupported owned rule package version {0}")]
    Version(u32),
    #[error("rule package and definition schema bindings disagree")]
    Binding,
    #[error("invalid owned rule package structure: {0}")]
    Structure(&'static str),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct RuleStorageUse {
    pub receivers: usize,
    pub receiver_targets: usize,
    pub receiver_work: usize,
    pub owners: usize,
    pub programs: usize,
    pub tables: usize,
    pub table_cells: usize,
    pub reads: usize,
    pub nodes: usize,
    pub effects: usize,
    /// Expression-to-read/node and effect-to-node references.
    pub edges: usize,
    pub gaps: usize,
}
impl RuleStorageUse {
    fn check(self, l: RuleStorageLimits) -> Result<(), RuleStorageError> {
        for (name, n, max) in [
            ("receiver work", self.receiver_work, l.max_receiver_work),
            ("receivers", self.receivers, l.max_receivers),
            (
                "receiver targets",
                self.receiver_targets,
                l.max_receiver_targets,
            ),
            ("owners", self.owners, l.max_owners),
            ("programs", self.programs, l.max_programs),
            ("tables", self.tables, l.max_tables),
            ("table cells", self.table_cells, l.max_table_cells),
            ("reads", self.reads, l.max_reads),
            ("nodes", self.nodes, l.max_nodes),
            ("effects", self.effects, l.max_effects),
            ("edges", self.edges, l.max_edges),
            ("gaps", self.gaps, l.max_gaps),
        ] {
            if n > max {
                return Err(RuleStorageError::Limit(name));
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct OwnedRulePackage {
    input: RulePackageInput,
    identity: OwnedContentDigest,
    canonical: Vec<u8>,
    resources: RuleStorageUse,
}
impl OwnedRulePackage {
    pub fn new<I: DefinitionSchemaIndex>(
        mut input: RulePackageInput,
        index: &I,
        limits: RuleStorageLimits,
    ) -> Result<Self, RuleStorageError> {
        limits.validate()?;
        if input.schema_version != OWNED_RULE_PACKAGE_VERSION {
            return Err(RuleStorageError::Version(input.schema_version));
        }
        if input.definitions != *index.identity()
            || input.namespace != *index.namespace()
            || input.definitions.validate().is_err()
        {
            return Err(RuleStorageError::Binding);
        }
        // Size bound before secondary indexes or serialization buffers.
        digest_owned("owned-rule-package-v2", &input, limits.max_wire_bytes)?;
        let resources = validate_structure(&input, index, limits)?;
        input.receivers.members.sort_by(|a, b| a.id.cmp(&b.id));
        for receiver in &mut input.receivers.members {
            receiver.targets.sort();
        }
        let identity = digest_owned("owned-rule-package-v2", &input, limits.max_wire_bytes)?;
        let canonical = serde_json::to_vec(&input)?;
        Ok(Self {
            input,
            identity,
            canonical,
            resources,
        })
    }
    pub fn input(&self) -> &RulePackageInput {
        &self.input
    }
    pub fn identity(&self) -> &OwnedContentDigest {
        &self.identity
    }
    pub fn definitions(&self) -> &DataIdentity {
        &self.input.definitions
    }
    pub fn resources(&self) -> RuleStorageUse {
        self.resources
    }
}
#[derive(Eq, Ord, PartialEq, PartialOrd)]
enum OwnerKey<'a> {
    Definition(&'a DefinitionAddress),
    Slot(&'a SlotAddress),
}
fn owner_key(owner: &SchemaSubject) -> OwnerKey<'_> {
    match owner {
        SchemaSubject::Definition(v) => OwnerKey::Definition(v),
        SchemaSubject::Slot(v) => OwnerKey::Slot(v),
    }
}
fn add(total: &mut usize, n: usize) -> Result<(), RuleStorageError> {
    *total = total
        .checked_add(n)
        .ok_or(RuleStorageError::Limit("aggregate size"))?;
    Ok(())
}
fn validate_structure<I: DefinitionSchemaIndex>(
    input: &RulePackageInput,
    index: &I,
    l: RuleStorageLimits,
) -> Result<RuleStorageUse, RuleStorageError> {
    let mut use_ = RuleStorageUse {
        receivers: input.receivers.members.len(),
        owners: input.owners.len(),
        tables: input.tables.len(),
        ..Default::default()
    };
    use_.check(l)?;
    // Each row, target and gap has a constant number of indexed schema lookups.
    // Check all counts before allocating registry indexes, including unused rows.
    for receiver in &input.receivers.members {
        add(&mut use_.receiver_targets, receiver.targets.len())?;
    }
    if let SchemaClosure::Partial { gaps } = &input.receivers.closure {
        add(&mut use_.gaps, gaps.len())?;
    }
    use_.check(l)?;
    let mut tables = BTreeSet::new();
    for table in &input.tables {
        add(&mut use_.table_cells, table.rows.len())?;
        use_.check(l)?;
        if !tables.insert(&table.id) {
            return Err(RuleStorageError::Structure("duplicate integer table"));
        }
        if table.domain_size() != Some(table.rows.len()) || table.rows.is_empty() {
            return Err(RuleStorageError::Structure(
                "integer table requires every domain row",
            ));
        }
        for value in &table.rows {
            use poe_optimizer_core::owned_build::ParameterValue;
            let valid = match (value, &table.value_type) {
                (ParameterValue::Boolean(_), ComputedValueType::Boolean)
                | (ParameterValue::Integer(_), ComputedValueType::Integer) => true,
                (ParameterValue::Quantity(v), ComputedValueType::Quantity { unit }) => {
                    v.unit() == unit && matches!(index.definition(unit), SchemaLookup::Known(_))
                }
                (ParameterValue::Option(v), ComputedValueType::Option) => {
                    matches!(index.definition(v), SchemaLookup::Known(_))
                }
                _ => false,
            };
            if !valid {
                return Err(RuleStorageError::Structure(
                    "invalid integer table cell type/unit/definition",
                ));
            }
        }
    }
    let mut owners = BTreeSet::new();
    for owner in &input.owners {
        if !owners.insert(owner_key(&owner.owner)) {
            return Err(RuleStorageError::Structure("duplicate owner"));
        }
        let valid = match &owner.owner {
            SchemaSubject::Definition(id) => {
                id.namespace() == &input.namespace
                    && index
                        .lookup_definition(id)
                        .is_some_and(|d| d.address() == *id)
            }
            SchemaSubject::Slot(id) => {
                id.namespace() == &input.namespace
                    && id.declaration().namespace() == &input.namespace
                    && index.lookup_slot(id).is_some_and(|d| d.address() == *id)
            }
        };
        if !valid {
            return Err(RuleStorageError::Structure("missing or foreign rule owner"));
        }
        if let SchemaClosure::Partial { gaps } = &owner.programs.closure {
            add(&mut use_.gaps, gaps.len())?;
            use_.check(l)?;
            if gaps.is_empty() {
                return Err(RuleStorageError::Structure(
                    "partial programs need gap evidence",
                ));
            }
            let mut codes = BTreeSet::new();
            for gap in gaps {
                if gap.subject != owner.owner
                    || gap.facet != SchemaFacet::GameRules
                    || !codes.insert(&gap.code)
                {
                    return Err(RuleStorageError::Structure("invalid owner program gap"));
                }
            }
        }
        add(&mut use_.programs, owner.programs.members.len())?;
        use_.check(l)?;
        let mut programs = BTreeSet::new();
        for p in &owner.programs.members {
            if !programs.insert(&p.id) {
                return Err(RuleStorageError::Structure("duplicate program"));
            }
            add(&mut use_.reads, p.reads.len())?;
            add(&mut use_.nodes, p.nodes.len())?;
            add(&mut use_.effects, p.effects.len())?;
            use_.check(l)?;
            let reads: BTreeSet<_> = p.reads.iter().map(|r| &r.id).collect();
            let nodes: BTreeSet<_> = p.nodes.iter().map(|r| &r.id).collect();
            let effects: BTreeSet<_> = p.effects.iter().map(|r| &r.id).collect();
            if reads.len() != p.reads.len()
                || nodes.len() != p.nodes.len()
                || effects.len() != p.effects.len()
            {
                return Err(RuleStorageError::Structure("duplicate local identity"));
            }
            // Read edges address a separate local table; count them before the
            // node-reference visitor borrows the aggregate budget.
            add(
                &mut use_.edges,
                p.nodes
                    .iter()
                    .filter(|n| {
                        matches!(
                            n.expression,
                            RuleExpression::Read { .. } | RuleExpression::LookupIntegerTable { .. }
                        )
                    })
                    .count(),
            )?;
            use_.check(l)?;
            let mut node_ref = |key: &poe_optimizer_core::owned_definitions::OwnedDefinitionKey| {
                add(&mut use_.edges, 1)?;
                use_.check(l)?;
                if nodes.contains(key) {
                    Ok(())
                } else {
                    Err(RuleStorageError::Structure("missing expression node"))
                }
            };
            for node in &p.nodes {
                use RuleExpression::*;
                match &node.expression {
                    OrdinaryTiming { recipe } => {
                        for input in recipe.inputs() {
                            node_ref(input)?;
                        }
                    }
                    LookupIntegerTable { table, key } => {
                        if !tables.contains(table) {
                            return Err(RuleStorageError::Structure("missing integer table"));
                        }
                        node_ref(key)?;
                    }
                    Literal { .. } => {}
                    Read { input } => {
                        if !reads.contains(input) {
                            return Err(RuleStorageError::Structure("missing read declaration"));
                        }
                    }
                    Add { left, right }
                    | Subtract { left, right }
                    | Minimum { left, right }
                    | Maximum { left, right }
                    | Compare { left, right, .. } => {
                        node_ref(left)?;
                        node_ref(right)?;
                    }
                    Scale { value, factor } => {
                        node_ref(value)?;
                        node_ref(factor)?;
                    }
                    ScaleInteger { value, count } => {
                        node_ref(value)?;
                        node_ref(count)?;
                    }
                    DivideFactor { value, divisor } => {
                        node_ref(value)?;
                        node_ref(divisor)?;
                    }
                    Ratio {
                        numerator,
                        denominator,
                        ..
                    } => {
                        node_ref(numerator)?;
                        node_ref(denominator)?;
                    }
                    PercentAsFactor { percent, .. } => node_ref(percent)?,
                    Round { value, .. } | Not { value } => node_ref(value)?,
                    All { values } | Any { values } => {
                        for value in values {
                            node_ref(value)?;
                        }
                    }
                    Select {
                        condition,
                        when_true,
                        when_false,
                    } => {
                        node_ref(condition)?;
                        node_ref(when_true)?;
                        node_ref(when_false)?;
                    }
                }
            }
            for effect in &p.effects {
                if let Some(when) = &effect.when {
                    node_ref(when)?;
                }
                use RuleEffectKind::*;
                match &effect.effect {
                    Contribute { value, .. }
                    | Derive { value, .. }
                    | ProjectSkillParameter { value, .. }
                    | ProjectActorStat { value, .. } => node_ref(value)?,
                    Capability { enabled, .. } | ActivateGrant { enabled, .. } => {
                        node_ref(enabled)?
                    }
                    SupportApplicability { applicable } => node_ref(applicable)?,
                    Requirement { satisfied, .. } => node_ref(satisfied)?,
                }
            }
        }
    }
    validate_receivers(input, index, l, &mut use_)?;
    Ok(use_)
}
fn validate_receivers<I: DefinitionSchemaIndex>(
    input: &RulePackageInput,
    index: &I,
    limits: RuleStorageLimits,
    usage: &mut RuleStorageUse,
) -> Result<(), RuleStorageError> {
    use poe_optimizer_core::owned_schema::RuleEntityKind;
    let invalid = RuleStorageError::Structure;
    let program_count = usage.programs;
    let mut work = |n| {
        add(&mut usage.receiver_work, n)?;
        usage.check(limits)
    };
    let programs: std::collections::BTreeMap<_, _> = if input.receivers.members.is_empty() {
        Default::default()
    } else {
        work(input.owners.len() + program_count)?;
        input
            .owners
            .iter()
            .filter_map(|owner| {
                if let SchemaSubject::Definition(DefinitionAddress::Stat(stat)) = &owner.owner {
                    Some((stat, &owner.programs.members))
                } else {
                    None
                }
            })
            .flat_map(|(stat, programs)| programs.iter().map(move |p| ((stat, &p.id), p)))
            .collect()
    };
    let mut ids = BTreeSet::new();
    let mut selected = BTreeSet::new();
    for receiver in &input.receivers.members {
        work(3)?;
        if !ids.insert(&receiver.id) || !selected.insert((&receiver.stat, &receiver.program)) {
            return Err(invalid("duplicate receiver ID or stat/program"));
        }
        let SchemaLookup::Known(stat) = index.definition(&receiver.stat) else {
            return Err(invalid(
                "missing, unmapped, foreign or inconsistent receiver stat",
            ));
        };
        work(stat.targets.len() + 1)?;
        if !stat.targets.contains(&RuleEntityKind::Actor) {
            return Err(invalid("receiver stat must admit Actor targets"));
        }
        if let ComputedValueType::Quantity { unit } = &stat.value
            && !matches!(index.definition(unit), SchemaLookup::Known(_))
        {
            return Err(invalid("receiver stat unit must be known"));
        }
        if receiver.targets.is_empty() {
            return Err(invalid("receiver requires explicit targets"));
        }
        let mut targets = BTreeSet::new();
        for target in &receiver.targets {
            work(1)?;
            if !targets.insert(target) {
                return Err(invalid("duplicate receiver target"));
            }
            if let ActorReceiverTarget::OwnedSlot { slot } = target
                && !matches!(index.slot(slot), SchemaLookup::Known(_))
            {
                return Err(invalid(
                    "missing, unmapped, foreign or inconsistent receiver actor slot",
                ));
            }
        }
        let Some(program) = programs.get(&(&receiver.stat, &receiver.program)) else {
            return Err(invalid("receiver requires an existing stat-owned program"));
        };
        if program.context != RuleEntityKind::Actor
            || program.effects.len() != 1
            || !matches!(&program.effects[0].effect,
                RuleEffectKind::Derive { entity: RuleEntity::Current | RuleEntity::Actor, stat, .. }
                if stat == &receiver.stat)
        {
            return Err(invalid(
                "receiver requires exactly one Actor-context final derive to its stat",
            ));
        }
    }
    if let SchemaClosure::Partial { gaps } = &input.receivers.closure {
        if gaps.is_empty() {
            return Err(invalid("partial receivers need gap evidence"));
        }
        let mut seen = BTreeSet::new();
        for gap in gaps {
            work(1)?;
            let valid = match &gap.subject {
                SchemaSubject::Definition(id) => {
                    id.namespace() == &input.namespace
                        && index
                            .lookup_definition(id)
                            .is_some_and(|d| d.address() == *id)
                }
                SchemaSubject::Slot(id) => {
                    id.namespace() == &input.namespace
                        && id.declaration().namespace() == &input.namespace
                        && index.lookup_slot(id).is_some_and(|d| d.address() == *id)
                }
            };
            if !valid
                || gap.facet != SchemaFacet::GameRules
                || !seen.insert((owner_key(&gap.subject), &gap.code))
            {
                return Err(invalid("invalid receiver registry gap"));
            }
        }
    }
    Ok(())
}
pub fn decode_rule_package<I: DefinitionSchemaIndex>(
    bytes: &[u8],
    index: &I,
    limits: RuleStorageLimits,
) -> Result<OwnedRulePackage, RuleStorageError> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(RuleStorageError::Limit("bytes"));
    }
    OwnedRulePackage::new(serde_json::from_slice(bytes)?, index, limits)
}
pub fn encode_rule_package(
    package: &OwnedRulePackage,
    limits: RuleStorageLimits,
) -> Result<Vec<u8>, RuleStorageError> {
    limits.validate()?;
    package.resources.check(limits)?;
    if package.canonical.len() > limits.max_wire_bytes {
        return Err(RuleStorageError::Limit("bytes"));
    }
    Ok(package.canonical.clone())
}
