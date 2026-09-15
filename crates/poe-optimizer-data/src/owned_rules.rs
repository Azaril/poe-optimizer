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
        DefinitionAddress, DefinitionSchemaIndex, SchemaClosure, SchemaFacet, SchemaSubject,
        SlotAddress,
    },
};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug)]
pub struct RuleStorageLimits {
    pub max_owners: usize,
    pub max_programs: usize,
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
            max_owners: 100_000,
            max_programs: 200_000,
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
            ("owners", self.max_owners, hard.max_owners),
            ("programs", self.max_programs, hard.max_programs),
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
    pub owners: usize,
    pub programs: usize,
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
            ("owners", self.owners, l.max_owners),
            ("programs", self.programs, l.max_programs),
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
        input: RulePackageInput,
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
        let identity = digest_owned("owned-rule-package-v1", &input, limits.max_wire_bytes)?;
        let resources = validate_structure(&input, index, limits)?;
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
        owners: input.owners.len(),
        ..Default::default()
    };
    use_.check(l)?;
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
                    .filter(|n| matches!(n.expression, RuleExpression::Read { .. }))
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
                    | ProjectSkillParameter { value, .. } => node_ref(value)?,
                    Capability { enabled, .. } | ActivateGrant { enabled, .. } => {
                        node_ref(enabled)?
                    }
                    SupportApplicability { applicable } => node_ref(applicable)?,
                    Requirement { satisfied, .. } => node_ref(satisfied)?,
                }
            }
        }
    }
    Ok(use_)
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
