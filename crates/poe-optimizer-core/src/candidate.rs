//! Finite, resolved build choices and conservative validation, independent of an evaluator.
//!
//! Catalog construction owns source decoding and payload digest verification. This module
//! checks references and explicitly supplied rules; it does not infer missing game mechanics.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const CANDIDATE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogIdentity {
    pub schema_version: u32,
    pub game: String,
    pub rules_revision: String,
    /// SHA-256 of the canonical source manifest, verified by the catalog producer.
    pub content_fingerprint: String,
}

/// Set/map ordering makes equivalent choices equal and suitable for deduplication.
/// Passive start nodes are implicit; `passives` contains allocated, paid nodes only.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub catalog: CatalogIdentity,
    pub class_id: String,
    pub ascendancy_id: Option<String>,
    pub passives: BTreeSet<u32>,
    pub equipment: BTreeMap<String, String>,
    pub skills: BTreeMap<String, SkillAssignment>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SkillAssignment {
    pub active_instance_id: String,
    pub support_instance_ids: BTreeSet<String>,
}

/// Exact payload, including rolls or gem configuration, remains in the catalog.
/// Two identical physical items still require different instance IDs in the catalog map.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExactPayload {
    pub format: String,
    pub content: String,
    /// Producers must verify this digest against the exact UTF-8 content bytes.
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClassDefinition {
    pub start_node_id: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AscendancyDefinition {
    pub class_id: String,
    pub start_node_id: u32,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PassiveKind {
    ClassStart {
        class_ids: BTreeSet<String>,
    },
    AscendancyStart {
        ascendancy_ids: BTreeSet<String>,
    },
    #[default]
    Ordinary,
    Ascendancy {
        ascendancy_ids: BTreeSet<String>,
    },
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PassiveNode {
    pub kind: PassiveKind,
    pub point_cost: u32,
    /// Source edges may be one-directional; the domain derives symmetric adjacency.
    pub links: BTreeSet<u32>,
    pub unsupported_mechanics: BTreeSet<String>,
}

/// Empty class/ascendancy allow-lists mean unrestricted. Required owners are conjunctive.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Availability {
    pub class_ids: BTreeSet<String>,
    pub ascendancy_ids: BTreeSet<String>,
    pub required_item_instance_ids: BTreeSet<String>,
    pub required_passives: BTreeSet<u32>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ItemInstance {
    pub definition_id: String,
    pub payload: ExactPayload,
    /// Empty means incompatible with every slot, not an unrestricted item.
    pub compatible_slots: BTreeSet<String>,
    pub availability: Availability,
    pub resource_costs: BTreeMap<String, u32>,
    pub unsupported_mechanics: BTreeSet<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveSkillInstance {
    pub definition_id: String,
    pub payload: ExactPayload,
    pub availability: Availability,
    pub resource_costs: BTreeMap<String, u32>,
    pub unsupported_mechanics: BTreeSet<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupportInstance {
    pub definition_id: String,
    pub payload: ExactPayload,
    /// Active skill definition IDs, resolved by the catalog producer.
    /// Empty means incompatible with every active skill.
    pub compatible_active_skill_ids: BTreeSet<String>,
    pub resource_costs: BTreeMap<String, u32>,
    pub unsupported_mechanics: BTreeSet<String>,
}

/// A finite, immutable run input. IDs are meaningful only under this exact identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateCatalog {
    pub identity: CatalogIdentity,
    pub classes: BTreeMap<String, ClassDefinition>,
    pub ascendancies: BTreeMap<String, AscendancyDefinition>,
    pub passive_nodes: BTreeMap<u32, PassiveNode>,
    pub equipment_slots: BTreeSet<String>,
    pub skill_slots: BTreeSet<String>,
    pub items: BTreeMap<String, ItemInstance>,
    pub active_skills: BTreeMap<String, ActiveSkillInstance>,
    pub supports: BTreeMap<String, SupportInstance>,
    /// Explicit global per-definition limits. An absent limit permits available instances.
    pub support_definition_limits: BTreeMap<String, u32>,
    pub unsupported_mechanics: BTreeSet<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum AscendancyLock {
    None,
    Id(String),
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SkillGroupLocks {
    pub active_instance_id: Option<String>,
    pub required_support_instance_ids: BTreeSet<String>,
    pub forbidden_support_instance_ids: BTreeSet<String>,
    pub exact_support_instance_ids: Option<BTreeSet<String>>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateLocks {
    pub class_id: Option<String>,
    /// Outer absence leaves ascendancy free; `None` explicitly fixes no ascendancy.
    pub ascendancy: Option<AscendancyLock>,
    pub allocated_passives: BTreeSet<u32>,
    pub unallocated_passives: BTreeSet<u32>,
    /// A null locks a slot empty; an item ID locks that exact instance in that slot.
    pub equipment: BTreeMap<String, Option<String>>,
    pub skill_groups: BTreeMap<String, SkillGroupLocks>,
    pub empty_skill_groups: BTreeSet<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateBudgets {
    pub ordinary_passive_points: u32,
    pub ascendancy_passive_points: u32,
    pub active_skill_count: u32,
    pub supports_per_skill: u32,
    /// Only resolved additive costs are checked; absent resource limits fail closed.
    pub resource_limits: BTreeMap<String, u32>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateConstraints {
    /// At least one assigned active instance of each skill definition must remain enabled.
    pub required_skill_ids: BTreeSet<String>,
    /// Exact physical instances must be equipped, in any compatible unlocked slot.
    pub required_item_instance_ids: BTreeSet<String>,
    pub locks: CandidateLocks,
    pub budgets: CandidateBudgets,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateIssueCode {
    InvalidCatalog,
    InvalidConstraints,
    CatalogMismatch,
    UnknownReference,
    IncompatibleClass,
    DisconnectedPassive,
    WrongPointCategory,
    BudgetExceeded,
    IncompatibleSlot,
    DuplicateInstance,
    IncompatibleSupport,
    MissingRequirement,
    LockViolation,
    MissingResourceBudget,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateIssue {
    pub code: CandidateIssueCode,
    pub path: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateValidation {
    pub violations: Vec<CandidateIssue>,
    pub unsupported_mechanics: BTreeSet<String>,
}

impl CandidateValidation {
    /// Supplied finite rules pass; this does not certify complete game legality.
    pub fn is_valid_within_catalog(&self) -> bool {
        self.violations.is_empty()
    }

    /// Unknown mechanics must not silently enter a search advertised as validated.
    pub fn is_searchable(&self) -> bool {
        self.violations.is_empty() && self.unsupported_mechanics.is_empty()
    }

    fn issue(
        &mut self,
        code: CandidateIssueCode,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.violations.push(CandidateIssue {
            code,
            path: path.into(),
            message: message.into(),
        });
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateDomainError {
    pub issues: Vec<CandidateIssue>,
}

impl std::fmt::Display for CandidateDomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid candidate domain ({} issues)", self.issues.len())
    }
}

impl std::error::Error for CandidateDomainError {}

/// Validates the immutable catalog once and keeps graph construction out of the hot path.
#[derive(Clone, Debug)]
pub struct CandidateDomain {
    catalog: CandidateCatalog,
    constraints: CandidateConstraints,
    adjacency: BTreeMap<u32, BTreeSet<u32>>,
}

impl CandidateDomain {
    pub fn new(
        catalog: CandidateCatalog,
        constraints: CandidateConstraints,
    ) -> Result<Self, CandidateDomainError> {
        let mut report = CandidateValidation::default();
        validate_catalog(&catalog, &mut report);
        validate_constraints(&catalog, &constraints, &mut report);
        if !report.violations.is_empty() {
            return Err(CandidateDomainError {
                issues: report.violations,
            });
        }
        let mut adjacency: BTreeMap<u32, BTreeSet<u32>> = catalog
            .passive_nodes
            .keys()
            .map(|id| (*id, BTreeSet::new()))
            .collect();
        for (id, node) in &catalog.passive_nodes {
            for target in &node.links {
                adjacency.entry(*id).or_default().insert(*target);
                adjacency.entry(*target).or_default().insert(*id);
            }
        }
        Ok(Self {
            catalog,
            constraints,
            adjacency,
        })
    }

    pub fn catalog(&self) -> &CandidateCatalog {
        &self.catalog
    }
    pub fn constraints(&self) -> &CandidateConstraints {
        &self.constraints
    }

    /// Validation never repairs or changes a candidate, a lock, or an unavailable dimension.
    pub fn validate(&self, candidate: &Candidate) -> CandidateValidation {
        use CandidateIssueCode as Code;
        let mut report = CandidateValidation {
            unsupported_mechanics: self.catalog.unsupported_mechanics.clone(),
            ..CandidateValidation::default()
        };
        if candidate.catalog != self.catalog.identity {
            report.issue(
                Code::CatalogMismatch,
                "catalog",
                "Candidate belongs to a different immutable catalog",
            );
            return report;
        }
        let class = self.catalog.classes.get(&candidate.class_id);
        if class.is_none() {
            report.issue(Code::UnknownReference, "class_id", "Unknown class ID");
        }
        let ascendancy = candidate
            .ascendancy_id
            .as_ref()
            .and_then(|id| self.catalog.ascendancies.get(id));
        if candidate.ascendancy_id.is_some() && ascendancy.is_none() {
            report.issue(
                Code::UnknownReference,
                "ascendancy_id",
                "Unknown ascendancy ID",
            );
        }
        if let Some(ascendancy) = ascendancy
            && ascendancy.class_id != candidate.class_id
        {
            report.issue(
                Code::IncompatibleClass,
                "ascendancy_id",
                "Ascendancy does not belong to the selected class",
            );
        }
        let mut ordinary = BTreeSet::new();
        let mut asc_nodes = BTreeSet::new();
        let mut ordinary_points = 0_u64;
        let mut asc_points = 0_u64;
        for id in &candidate.passives {
            let path = format!("passives.{id}");
            let Some(node) = self.catalog.passive_nodes.get(id) else {
                report.issue(Code::UnknownReference, path, "Unknown passive node");
                continue;
            };
            report
                .unsupported_mechanics
                .extend(node.unsupported_mechanics.iter().cloned());
            match &node.kind {
                PassiveKind::Ordinary => {
                    ordinary.insert(*id);
                    ordinary_points += u64::from(node.point_cost);
                }
                PassiveKind::Ascendancy { ascendancy_ids }
                    if candidate
                        .ascendancy_id
                        .as_ref()
                        .is_some_and(|id| ascendancy_ids.contains(id)) =>
                {
                    asc_nodes.insert(*id);
                    asc_points += u64::from(node.point_cost);
                }
                PassiveKind::Ascendancy { .. } => report.issue(
                    Code::IncompatibleClass,
                    path,
                    "Passive belongs to a different ascendancy",
                ),
                PassiveKind::ClassStart { .. } | PassiveKind::AscendancyStart { .. } => report
                    .issue(
                        Code::WrongPointCategory,
                        path,
                        "Start nodes are implicit and cannot be allocated as paid nodes",
                    ),
            }
        }
        let budgets = &self.constraints.budgets;
        check_budget(
            &mut report,
            "ordinary_passive_points",
            ordinary_points,
            budgets.ordinary_passive_points,
        );
        check_budget(
            &mut report,
            "ascendancy_passive_points",
            asc_points,
            budgets.ascendancy_passive_points,
        );
        if let Some(class) = class {
            report.unsupported_mechanics.extend(
                self.catalog.passive_nodes[&class.start_node_id]
                    .unsupported_mechanics
                    .iter()
                    .cloned(),
            );
            self.check_connectivity(class.start_node_id, &ordinary, &mut report);
        }
        if let Some(ascendancy) = ascendancy {
            report.unsupported_mechanics.extend(
                self.catalog.passive_nodes[&ascendancy.start_node_id]
                    .unsupported_mechanics
                    .iter()
                    .cloned(),
            );
            self.check_connectivity(ascendancy.start_node_id, &asc_nodes, &mut report);
        }
        let mut resources = BTreeMap::new();
        let mut item_instances = BTreeSet::new();
        for (slot, id) in &candidate.equipment {
            let path = format!("equipment.{slot}");
            if !self.catalog.equipment_slots.contains(slot) {
                report.issue(Code::UnknownReference, &path, "Unknown equipment slot");
            }
            if !item_instances.insert(id.clone()) {
                report.issue(
                    Code::DuplicateInstance,
                    &path,
                    "The same physical item instance is equipped more than once",
                );
            }
            let Some(item) = self.catalog.items.get(id) else {
                report.issue(Code::UnknownReference, path, "Unknown item instance");
                continue;
            };
            if !item.compatible_slots.contains(slot) {
                report.issue(
                    Code::IncompatibleSlot,
                    &path,
                    "Item is incompatible with this slot",
                );
            }
            check_availability(&item.availability, candidate, &path, &mut report);
            accumulate_resources(&mut resources, &item.resource_costs);
            report
                .unsupported_mechanics
                .extend(item.unsupported_mechanics.iter().cloned());
        }
        let mut active_instances = BTreeSet::new();
        let mut skill_ids = BTreeSet::new();
        let mut support_instances = BTreeSet::new();
        let mut support_counts = BTreeMap::<String, u64>::new();
        check_budget(
            &mut report,
            "active_skill_count",
            candidate.skills.len() as u64,
            budgets.active_skill_count,
        );
        for (slot, choice) in &candidate.skills {
            let path = format!("skills.{slot}");
            if !self.catalog.skill_slots.contains(slot) {
                report.issue(Code::UnknownReference, &path, "Unknown skill group slot");
            }
            if !active_instances.insert(choice.active_instance_id.clone()) {
                report.issue(
                    Code::DuplicateInstance,
                    &path,
                    "The same active gem instance is assigned more than once",
                );
            }
            let active = self.catalog.active_skills.get(&choice.active_instance_id);
            if let Some(active) = active {
                skill_ids.insert(active.definition_id.clone());
                check_availability(&active.availability, candidate, &path, &mut report);
                accumulate_resources(&mut resources, &active.resource_costs);
                report
                    .unsupported_mechanics
                    .extend(active.unsupported_mechanics.iter().cloned());
            } else {
                report.issue(
                    Code::UnknownReference,
                    &path,
                    "Unknown active skill instance",
                );
            }
            check_budget(
                &mut report,
                &format!("{path}.supports"),
                choice.support_instance_ids.len() as u64,
                budgets.supports_per_skill,
            );
            for id in &choice.support_instance_ids {
                let support_path = format!("{path}.supports.{id}");
                if !support_instances.insert(id.clone()) {
                    report.issue(
                        Code::DuplicateInstance,
                        &support_path,
                        "The same support gem instance is assigned more than once",
                    );
                }
                let Some(support) = self.catalog.supports.get(id) else {
                    report.issue(
                        Code::UnknownReference,
                        support_path,
                        "Unknown support instance",
                    );
                    continue;
                };
                if let Some(active) = active
                    && !support
                        .compatible_active_skill_ids
                        .contains(&active.definition_id)
                {
                    report.issue(
                        Code::IncompatibleSupport,
                        &support_path,
                        "Support is incompatible with the active skill definition",
                    );
                }
                *support_counts
                    .entry(support.definition_id.clone())
                    .or_default() += 1;
                accumulate_resources(&mut resources, &support.resource_costs);
                report
                    .unsupported_mechanics
                    .extend(support.unsupported_mechanics.iter().cloned());
            }
        }
        for (id, count) in support_counts {
            if let Some(limit) = self.catalog.support_definition_limits.get(&id) {
                check_budget(
                    &mut report,
                    &format!("support_definition_limits.{id}"),
                    count,
                    *limit,
                );
            }
        }
        for (id, amount) in resources {
            if let Some(limit) = budgets.resource_limits.get(&id) {
                check_budget(&mut report, &format!("resources.{id}"), amount, *limit);
            } else {
                report.issue(
                    Code::MissingResourceBudget,
                    format!("resources.{id}"),
                    "An explicit limit is required for this resolved resource cost",
                );
            }
        }
        for id in &self.constraints.required_skill_ids {
            if !skill_ids.contains(id) {
                report.issue(
                    Code::MissingRequirement,
                    format!("required_skill_ids.{id}"),
                    "Required active skill is absent",
                );
            }
        }
        for id in &self.constraints.required_item_instance_ids {
            if !item_instances.contains(id) {
                report.issue(
                    Code::MissingRequirement,
                    format!("required_item_instance_ids.{id}"),
                    "Required exact item instance is not equipped",
                );
            }
        }
        self.check_locks(candidate, &mut report);
        report
    }

    fn check_connectivity(
        &self,
        root: u32,
        nodes: &BTreeSet<u32>,
        report: &mut CandidateValidation,
    ) {
        let mut visited = BTreeSet::from([root]);
        let mut pending = VecDeque::from([root]);
        while let Some(id) = pending.pop_front() {
            for next in &self.adjacency[&id] {
                if nodes.contains(next) && visited.insert(*next) {
                    pending.push_back(*next);
                }
            }
        }
        for id in nodes.difference(&visited) {
            report.issue(
                CandidateIssueCode::DisconnectedPassive,
                format!("passives.{id}"),
                "Allocated node is not connected to its selected class or ascendancy start",
            );
        }
    }

    fn check_locks(&self, candidate: &Candidate, report: &mut CandidateValidation) {
        use CandidateIssueCode::LockViolation;
        let locks = &self.constraints.locks;
        if let Some(id) = &locks.class_id
            && candidate.class_id != *id
        {
            report.issue(LockViolation, "locks.class_id", "Locked class changed");
        }
        let asc_matches = match &locks.ascendancy {
            None => true,
            Some(AscendancyLock::None) => candidate.ascendancy_id.is_none(),
            Some(AscendancyLock::Id(id)) => candidate.ascendancy_id.as_ref() == Some(id),
        };
        if !asc_matches {
            report.issue(
                LockViolation,
                "locks.ascendancy",
                "Locked ascendancy changed",
            );
        }
        for id in &locks.allocated_passives {
            if !candidate.passives.contains(id) {
                report.issue(
                    LockViolation,
                    format!("locks.allocated_passives.{id}"),
                    "Locked allocated passive was removed",
                );
            }
        }
        for id in &locks.unallocated_passives {
            if candidate.passives.contains(id) {
                report.issue(
                    LockViolation,
                    format!("locks.unallocated_passives.{id}"),
                    "Locked unallocated passive was added",
                );
            }
        }
        for (slot, id) in &locks.equipment {
            if candidate.equipment.get(slot) != id.as_ref() {
                report.issue(
                    LockViolation,
                    format!("locks.equipment.{slot}"),
                    "Locked equipment placement changed",
                );
            }
        }
        for slot in &locks.empty_skill_groups {
            if candidate.skills.contains_key(slot) {
                report.issue(
                    LockViolation,
                    format!("locks.empty_skill_groups.{slot}"),
                    "Locked empty skill group was populated",
                );
            }
        }
        for (slot, lock) in &locks.skill_groups {
            let path = format!("locks.skill_groups.{slot}");
            let Some(choice) = candidate.skills.get(slot) else {
                report.issue(LockViolation, path, "Locked skill group is absent");
                continue;
            };
            if let Some(id) = &lock.active_instance_id
                && choice.active_instance_id != *id
            {
                report.issue(LockViolation, &path, "Locked active skill instance changed");
            }
            if !lock
                .required_support_instance_ids
                .is_subset(&choice.support_instance_ids)
                || !lock
                    .forbidden_support_instance_ids
                    .is_disjoint(&choice.support_instance_ids)
                || lock
                    .exact_support_instance_ids
                    .as_ref()
                    .is_some_and(|ids| *ids != choice.support_instance_ids)
            {
                report.issue(LockViolation, path, "Locked support selection changed");
            }
        }
    }
}

fn check_budget(report: &mut CandidateValidation, path: &str, amount: u64, limit: u32) {
    if amount > u64::from(limit) {
        report.issue(
            CandidateIssueCode::BudgetExceeded,
            path,
            format!("Uses {amount}; budget is {limit}"),
        );
    }
}

fn accumulate_resources(total: &mut BTreeMap<String, u64>, costs: &BTreeMap<String, u32>) {
    for (id, value) in costs {
        *total.entry(id.clone()).or_default() += u64::from(*value);
    }
}

fn check_availability(
    availability: &Availability,
    candidate: &Candidate,
    path: &str,
    report: &mut CandidateValidation,
) {
    use CandidateIssueCode as Code;
    if !availability.class_ids.is_empty() && !availability.class_ids.contains(&candidate.class_id) {
        report.issue(
            Code::IncompatibleClass,
            path,
            "Choice is unavailable to this class",
        );
    }
    if !availability.ascendancy_ids.is_empty()
        && !candidate
            .ascendancy_id
            .as_ref()
            .is_some_and(|id| availability.ascendancy_ids.contains(id))
    {
        report.issue(
            Code::IncompatibleClass,
            path,
            "Choice is unavailable to this ascendancy",
        );
    }
    for id in &availability.required_item_instance_ids {
        if !candidate.equipment.values().any(|equipped| equipped == id) {
            report.issue(
                Code::MissingRequirement,
                path,
                format!("Choice requires equipped item instance {id}"),
            );
        }
    }
    for id in &availability.required_passives {
        if !candidate.passives.contains(id) {
            report.issue(
                Code::MissingRequirement,
                path,
                format!("Choice requires allocated passive {id}"),
            );
        }
    }
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 512
}
fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_payload(payload: &ExactPayload, path: &str, report: &mut CandidateValidation) {
    if !valid_id(&payload.format) || payload.content.is_empty() || !valid_digest(&payload.sha256) {
        report.issue(
            CandidateIssueCode::InvalidCatalog,
            path,
            "Exact payload requires a format, content, and lowercase SHA-256 digest",
        );
    }
}

fn validate_catalog(catalog: &CandidateCatalog, report: &mut CandidateValidation) {
    use CandidateIssueCode::InvalidCatalog;
    if catalog.identity.schema_version != CANDIDATE_SCHEMA_VERSION
        || !valid_id(&catalog.identity.game)
        || !valid_id(&catalog.identity.rules_revision)
        || !valid_digest(&catalog.identity.content_fingerprint)
    {
        report.issue(
            InvalidCatalog,
            "identity",
            "Unsupported catalog schema or invalid source identity",
        );
    }
    if catalog.classes.is_empty() {
        report.issue(InvalidCatalog, "classes", "At least one class is required");
    }
    for (id, class) in &catalog.classes {
        if !valid_id(id)
            || !matches!(catalog.passive_nodes.get(&class.start_node_id), Some(PassiveNode { kind: PassiveKind::ClassStart { class_ids }, .. }) if class_ids.contains(id))
        {
            report.issue(
                InvalidCatalog,
                format!("classes.{id}"),
                "Class must reference its own class-start node",
            );
        }
    }
    for (id, asc) in &catalog.ascendancies {
        if !valid_id(id)
            || !catalog.classes.contains_key(&asc.class_id)
            || !matches!(catalog.passive_nodes.get(&asc.start_node_id), Some(PassiveNode { kind: PassiveKind::AscendancyStart { ascendancy_ids }, .. }) if ascendancy_ids.contains(id))
        {
            report.issue(
                InvalidCatalog,
                format!("ascendancies.{id}"),
                "Ascendancy requires a known class and its own start node",
            );
        }
    }
    for (id, node) in &catalog.passive_nodes {
        let path = format!("passive_nodes.{id}");
        let known_owner = match &node.kind {
            PassiveKind::ClassStart { class_ids } => {
                !class_ids.is_empty()
                    && class_ids.iter().all(|class_id| {
                        catalog
                            .classes
                            .get(class_id)
                            .is_some_and(|class| class.start_node_id == *id)
                    })
            }
            PassiveKind::AscendancyStart { ascendancy_ids } => {
                !ascendancy_ids.is_empty()
                    && ascendancy_ids.iter().all(|ascendancy_id| {
                        catalog
                            .ascendancies
                            .get(ascendancy_id)
                            .is_some_and(|asc| asc.start_node_id == *id)
                    })
            }
            PassiveKind::Ascendancy { ascendancy_ids } => {
                !ascendancy_ids.is_empty()
                    && ascendancy_ids
                        .iter()
                        .all(|ascendancy_id| catalog.ascendancies.contains_key(ascendancy_id))
            }
            PassiveKind::Ordinary => true,
        };
        if !known_owner {
            report.issue(
                InvalidCatalog,
                &path,
                "Passive references an unknown or mismatched owner",
            );
        }
        let start = matches!(
            node.kind,
            PassiveKind::ClassStart { .. } | PassiveKind::AscendancyStart { .. }
        );
        if (start && node.point_cost != 0) || (!start && node.point_cost == 0) {
            report.issue(
                InvalidCatalog,
                &path,
                "Starts cost zero; explicit allocated nodes require a positive point cost",
            );
        }
        for target in &node.links {
            if !catalog.passive_nodes.contains_key(target) {
                report.issue(InvalidCatalog, &path, format!("Dangling edge to {target}; resolve it or explicitly exclude it with coverage evidence before constructing the catalog"));
            }
        }
    }
    for slot in catalog.equipment_slots.iter().chain(&catalog.skill_slots) {
        if !valid_id(slot) {
            report.issue(
                InvalidCatalog,
                "slots",
                "Slot IDs must be nonempty and bounded",
            );
        }
    }
    let active_definitions: BTreeSet<_> = catalog
        .active_skills
        .values()
        .map(|skill| skill.definition_id.as_str())
        .collect();
    let support_definitions: BTreeSet<_> = catalog
        .supports
        .values()
        .map(|skill| skill.definition_id.as_str())
        .collect();
    for (id, item) in &catalog.items {
        let path = format!("items.{id}");
        validate_entry(
            id,
            &item.definition_id,
            &item.payload,
            &item.resource_costs,
            &path,
            report,
        );
        validate_availability(&item.availability, catalog, &path, report);
        if item.compatible_slots.is_empty()
            || !item.compatible_slots.is_subset(&catalog.equipment_slots)
        {
            report.issue(
                InvalidCatalog,
                &path,
                "Item must name known compatible equipment slots",
            );
        }
    }
    for (id, active) in &catalog.active_skills {
        let path = format!("active_skills.{id}");
        validate_entry(
            id,
            &active.definition_id,
            &active.payload,
            &active.resource_costs,
            &path,
            report,
        );
        validate_availability(&active.availability, catalog, &path, report);
    }
    for (id, support) in &catalog.supports {
        let path = format!("supports.{id}");
        validate_entry(
            id,
            &support.definition_id,
            &support.payload,
            &support.resource_costs,
            &path,
            report,
        );
        for active_id in &support.compatible_active_skill_ids {
            if !active_definitions.contains(active_id.as_str()) {
                report.issue(
                    InvalidCatalog,
                    &path,
                    "Support compatibility refers to an unknown active skill definition",
                );
            }
        }
    }
    for id in catalog.support_definition_limits.keys() {
        if !support_definitions.contains(id.as_str()) {
            report.issue(
                InvalidCatalog,
                format!("support_definition_limits.{id}"),
                "Limit refers to an unknown support definition",
            );
        }
    }
}

fn validate_entry(
    id: &str,
    definition_id: &str,
    payload: &ExactPayload,
    costs: &BTreeMap<String, u32>,
    path: &str,
    report: &mut CandidateValidation,
) {
    if !valid_id(id) || !valid_id(definition_id) || costs.keys().any(|id| !valid_id(id)) {
        report.issue(
            CandidateIssueCode::InvalidCatalog,
            path,
            "Instance, definition, and resource IDs must be nonempty and bounded",
        );
    }
    validate_payload(payload, path, report);
}

fn validate_availability(
    availability: &Availability,
    catalog: &CandidateCatalog,
    path: &str,
    report: &mut CandidateValidation,
) {
    if availability
        .class_ids
        .iter()
        .any(|id| !catalog.classes.contains_key(id))
        || availability
            .ascendancy_ids
            .iter()
            .any(|id| !catalog.ascendancies.contains_key(id))
        || availability
            .required_item_instance_ids
            .iter()
            .any(|id| !catalog.items.contains_key(id))
        || availability.required_passives.iter().any(|id| {
            !matches!(
                catalog.passive_nodes.get(id),
                Some(PassiveNode {
                    kind: PassiveKind::Ordinary | PassiveKind::Ascendancy { .. },
                    ..
                })
            )
        })
    {
        report.issue(
            CandidateIssueCode::InvalidCatalog,
            path,
            "Availability refers to an unknown catalog ID",
        );
    }
}

fn validate_constraints(
    catalog: &CandidateCatalog,
    constraints: &CandidateConstraints,
    report: &mut CandidateValidation,
) {
    use CandidateIssueCode::InvalidConstraints;
    let locks = &constraints.locks;
    for id in &constraints.required_skill_ids {
        if !catalog
            .active_skills
            .values()
            .any(|skill| skill.definition_id == *id)
        {
            report.issue(
                InvalidConstraints,
                "required_skill_ids",
                format!("Unknown required skill {id}"),
            );
        }
    }
    for id in &constraints.required_item_instance_ids {
        if !catalog.items.contains_key(id) {
            report.issue(
                InvalidConstraints,
                "required_item_instance_ids",
                format!("Unknown required item instance {id}"),
            );
        }
    }
    if let Some(id) = &locks.class_id
        && !catalog.classes.contains_key(id)
    {
        report.issue(InvalidConstraints, "locks.class_id", "Unknown locked class");
    }
    if let Some(AscendancyLock::Id(id)) = &locks.ascendancy {
        match catalog.ascendancies.get(id) {
            None => report.issue(
                InvalidConstraints,
                "locks.ascendancy",
                "Unknown locked ascendancy",
            ),
            Some(asc)
                if locks
                    .class_id
                    .as_ref()
                    .is_some_and(|class_id| *class_id != asc.class_id) =>
            {
                report.issue(
                    InvalidConstraints,
                    "locks.ascendancy",
                    "Locked class and ascendancy are incompatible",
                )
            }
            Some(_) => {}
        }
    }
    if !locks
        .allocated_passives
        .is_disjoint(&locks.unallocated_passives)
    {
        report.issue(
            InvalidConstraints,
            "locks.passives",
            "A passive cannot be locked both allocated and unallocated",
        );
    }
    for id in locks.allocated_passives.union(&locks.unallocated_passives) {
        if !matches!(
            catalog.passive_nodes.get(id),
            Some(PassiveNode {
                kind: PassiveKind::Ordinary | PassiveKind::Ascendancy { .. },
                ..
            })
        ) {
            report.issue(
                InvalidConstraints,
                "locks.passives",
                "Only known paid passive nodes may be locked",
            );
        }
    }
    for (slot, id) in &locks.equipment {
        if !catalog.equipment_slots.contains(slot)
            || id.as_ref().is_some_and(|id| {
                !catalog
                    .items
                    .get(id)
                    .is_some_and(|item| item.compatible_slots.contains(slot))
            })
        {
            report.issue(
                InvalidConstraints,
                format!("locks.equipment.{slot}"),
                "Locked item placement is unknown or incompatible",
            );
        }
    }
    let locked_items: Vec<_> = locks.equipment.values().flatten().collect();
    if locked_items.iter().copied().collect::<BTreeSet<_>>().len() != locked_items.len() {
        report.issue(
            InvalidConstraints,
            "locks.equipment",
            "The same exact item instance cannot be locked into multiple slots",
        );
    }
    for slot in &locks.empty_skill_groups {
        if !catalog.skill_slots.contains(slot) || locks.skill_groups.contains_key(slot) {
            report.issue(
                InvalidConstraints,
                "locks.empty_skill_groups",
                "Unknown or conflicting locked skill group",
            );
        }
    }
    for (slot, lock) in &locks.skill_groups {
        let path = format!("locks.skill_groups.{slot}");
        if !catalog.skill_slots.contains(slot)
            || lock
                .active_instance_id
                .as_ref()
                .is_some_and(|id| !catalog.active_skills.contains_key(id))
        {
            report.issue(
                InvalidConstraints,
                &path,
                "Unknown locked skill group or active instance",
            );
        }
        let all_ids = lock
            .required_support_instance_ids
            .iter()
            .chain(&lock.forbidden_support_instance_ids)
            .chain(lock.exact_support_instance_ids.iter().flatten());
        if all_ids
            .into_iter()
            .any(|id| !catalog.supports.contains_key(id))
        {
            report.issue(InvalidConstraints, &path, "Unknown locked support instance");
        }
        if !lock
            .required_support_instance_ids
            .is_disjoint(&lock.forbidden_support_instance_ids)
            || lock.exact_support_instance_ids.as_ref().is_some_and(|ids| {
                !lock.required_support_instance_ids.is_subset(ids)
                    || !lock.forbidden_support_instance_ids.is_disjoint(ids)
            })
        {
            report.issue(InvalidConstraints, path, "Conflicting support locks");
        }
    }
    if constraints
        .budgets
        .resource_limits
        .keys()
        .any(|id| !valid_id(id))
    {
        report.issue(
            InvalidConstraints,
            "budgets.resource_limits",
            "Resource IDs must be nonempty and bounded",
        );
    }
}
