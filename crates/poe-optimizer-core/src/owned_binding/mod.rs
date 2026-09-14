//! Bind owned inputs to injected schemas without evaluating effects or importing source state.
//! A diagnostic report is never a prepared-plan authority token.
mod records;
mod selectors;
mod values;
use crate::{
    build_identity::*, data::DataIdentity, owned_build::*, owned_content::*, owned_definitions::*,
    owned_schema::*,
};
use serde::Serialize;
use std::fmt;
#[derive(Clone, Copy, Debug)]
pub struct BindingLimits {
    pub input: OwnedInputLimits,
    pub max_work: usize,
    pub max_issues: usize,
}
impl Default for BindingLimits {
    fn default() -> Self {
        Self {
            input: OwnedInputLimits::default(),
            max_work: 2_000_000,
            max_issues: 4096,
        }
    }
}
/// Indexes address rows in the exact canonical owned request, never source/UI indexes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum BindingLocation {
    Character,
    Reward(RewardSelectionId),
    Item(ItemRecordId),
    Modifier {
        item: ItemRecordId,
        modifier: ModifierInstanceId,
    },
    Gem(GemInstanceId),
    Equipment(ItemSlotUseId),
    Allocation(AllocationId),
    Skill(SkillUseId),
    Support(SupportAssignmentId),
    Payload(PayloadLinkId),
    Choice {
        index: usize,
    },
    Enemy,
    Assumption {
        index: usize,
    },
    Usage {
        index: usize,
    },
    Query(QueryId),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingFacet {
    Definition,
    Level,
    Quality,
    Parameter,
    Choice,
    Destination,
    Scope,
    Role,
    Target,
    Provider,
    Output,
    Part,
    Mode,
    StatSet,
    Pool,
    Access,
    RequiredValues,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BindingSite {
    pub location: BindingLocation,
    pub facet: BindingFacet,
}
impl BindingSite {
    fn new(location: BindingLocation, facet: BindingFacet) -> Self {
        Self { location, facet }
    }
    fn at(&self, facet: BindingFacet) -> Self {
        Self {
            location: self.location.clone(),
            facet,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueClass {
    Invalid,
    Unresolved,
    Unavailable,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingIssueCode {
    MissingDefinition,
    UnmappedSchema,
    PartialMembership,
    NotDeclared,
    ValueKindMismatch,
    UnitMismatch,
    OutOfRange,
    RequiredValueMissing,
    ValueForbidden,
    IncompatibleRole,
    IncompatibleScope,
    WrongOwner,
    MissingProvider,
    DisabledProvider,
    InactiveLoadout,
    ActorMismatch,
    NotDirectlySelectable,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BindingIssue {
    pub site: BindingSite,
    pub class: IssueClass,
    pub code: BindingIssueCode,
    pub subject: Option<SchemaSubject>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaBindingStatus {
    Valid,
    Invalid,
    Unresolved,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Address availability and remaining resolution work only. SchemaBound and
/// PendingResolution may accompany schema Invalid; inspect both statuses.
/// Neither status conveys evaluation or prepared-plan authority.
pub enum SelectorBindingStatus {
    SchemaBound,
    PendingResolution,
    Unavailable,
    Unresolved,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct QueryBinding {
    pub id: QueryId,
    pub schema: SchemaBindingStatus,
    pub selector: SelectorBindingStatus,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DefinitionBindingReport {
    request_digest: OwnedContentDigest,
    data_identity: DataIdentity,
    schema: SchemaBindingStatus,
    issues: Vec<BindingIssue>,
    queries: Vec<QueryBinding>,
}
impl DefinitionBindingReport {
    pub fn request_digest(&self) -> OwnedContentDigest {
        self.request_digest
    }
    pub fn data_identity(&self) -> &DataIdentity {
        &self.data_identity
    }
    pub fn schema(&self) -> SchemaBindingStatus {
        self.schema
    }
    pub fn issues(&self) -> &[BindingIssue] {
        &self.issues
    }
    pub fn queries(&self) -> &[QueryBinding] {
        &self.queries
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexFault {
    InvalidIdentity,
    InconsistentLookup,
    InvalidSchema,
    ForeignReference,
}
#[derive(Debug)]
pub enum BindingError {
    Structure(StructuralError),
    Digest(ContentDigestError),
    InvalidLimit,
    WorkLimit,
    IssueLimit,
    ForeignNamespace,
    Index {
        subject: Option<Box<SchemaSubject>>,
        fault: IndexFault,
    },
}
impl fmt::Display for BindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Structure(e) => e.fmt(f),
            Self::Digest(e) => e.fmt(f),
            Self::InvalidLimit => {
                f.write_str("binding limits must be positive and within their ceilings")
            }
            Self::WorkLimit => f.write_str("definition binding exceeded its work budget"),
            Self::IssueLimit => f.write_str("definition binding exceeded its issue budget"),
            Self::ForeignNamespace => f.write_str("request and definition index namespaces differ"),
            Self::Index { subject, fault } => {
                write!(f, "invalid definition index ({fault:?}) at {subject:?}")
            }
        }
    }
}
impl std::error::Error for BindingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Structure(e) => Some(e),
            Self::Digest(e) => Some(e),
            _ => None,
        }
    }
}
impl From<StructuralError> for BindingError {
    fn from(e: StructuralError) -> Self {
        Self::Structure(e)
    }
}
impl From<ContentDigestError> for BindingError {
    fn from(e: ContentDigestError) -> Self {
        Self::Digest(e)
    }
}
type Result<T = ()> = std::result::Result<T, BindingError>;
#[derive(Clone, Copy, Eq, PartialEq)]
enum Purpose {
    Authored,
    Query,
}
impl Purpose {
    fn absence(self) -> IssueClass {
        match self {
            Self::Authored => IssueClass::Invalid,
            Self::Query => IssueClass::Unavailable,
        }
    }
}
fn schema_status(issues: &[BindingIssue]) -> SchemaBindingStatus {
    if issues.iter().any(|i| i.class == IssueClass::Invalid) {
        SchemaBindingStatus::Invalid
    } else if issues.iter().any(|i| i.class == IssueClass::Unresolved) {
        SchemaBindingStatus::Unresolved
    } else {
        SchemaBindingStatus::Valid
    }
}
/// All ordered query rows survive. Generated activation, game legality, available
/// measurements and numerical effects remain responsibilities of resolution.
pub fn bind_owned_request<I: DefinitionSchemaIndex>(
    index: &I,
    request: &OwnedEvaluationRequest,
    limits: BindingLimits,
) -> Result<DefinitionBindingReport> {
    request.validate_limits(limits.input)?;
    if limits.max_work == 0
        || limits.max_work > 100_000_000
        || limits.max_issues == 0
        || limits.max_issues > 65_536
    {
        return Err(BindingError::InvalidLimit);
    }
    if index.namespace() != &request.build().input().game_version {
        return Err(BindingError::ForeignNamespace);
    }
    if index.identity().validate().is_err()
        || index.identity().game != index.namespace().game().as_str()
    {
        return Err(BindingError::Index {
            subject: None,
            fault: IndexFault::InvalidIdentity,
        });
    }
    let request_digest = digest_owned("owned-request-v1", request, limits.input.max_wire_bytes)?;
    let mut checker = Checker {
        index,
        request,
        limits,
        work: limits.max_work,
        issues: Vec::new(),
    };
    checker.bind_records()?;
    let queries = checker.bind_queries()?;
    Ok(DefinitionBindingReport {
        request_digest,
        data_identity: index.identity().clone(),
        schema: schema_status(&checker.issues),
        issues: checker.issues,
        queries,
    })
}
struct Checker<'a, I> {
    index: &'a I,
    request: &'a OwnedEvaluationRequest,
    limits: BindingLimits,
    work: usize,
    issues: Vec<BindingIssue>,
}
impl<'a, I: DefinitionSchemaIndex> Checker<'a, I> {
    fn charge(&mut self, n: usize) -> Result {
        self.work = self.work.checked_sub(n).ok_or(BindingError::WorkLimit)?;
        Ok(())
    }
    fn issue(
        &mut self,
        site: &BindingSite,
        class: IssueClass,
        code: BindingIssueCode,
        subject: Option<SchemaSubject>,
    ) -> Result {
        self.charge(1)?;
        if self.issues.len() >= self.limits.max_issues {
            return Err(BindingError::IssueLimit);
        }
        self.issues.push(BindingIssue {
            site: site.clone(),
            class,
            code,
            subject,
        });
        Ok(())
    }
    fn fault<T>(&self, subject: Option<SchemaSubject>) -> Result<T> {
        Err(BindingError::Index {
            subject: subject.map(Box::new),
            fault: IndexFault::InvalidSchema,
        })
    }
    fn found<T>(
        &mut self,
        lookup: SchemaLookup<'a, T>,
        subject: SchemaSubject,
        site: &BindingSite,
    ) -> Result<Option<&'a T>> {
        match lookup {
            SchemaLookup::Known(value) => Ok(Some(value)),
            SchemaLookup::Missing => {
                self.issue(
                    site,
                    IssueClass::Unresolved,
                    BindingIssueCode::MissingDefinition,
                    Some(subject),
                )?;
                Ok(None)
            }
            SchemaLookup::Unmapped(gaps) => {
                if gaps.is_empty() {
                    return self.fault(Some(subject));
                }
                self.issue(
                    site,
                    IssueClass::Unresolved,
                    BindingIssueCode::UnmappedSchema,
                    Some(subject),
                )?;
                Ok(None)
            }
            SchemaLookup::NamespaceMismatch => Err(BindingError::Index {
                subject: Some(Box::new(subject)),
                fault: IndexFault::ForeignReference,
            }),
            SchemaLookup::InconsistentIndex => Err(BindingError::Index {
                subject: Some(Box::new(subject)),
                fault: IndexFault::InconsistentLookup,
            }),
        }
    }
    fn definition<D: SchemaDefinitionId>(
        &mut self,
        id: &D,
        site: &BindingSite,
    ) -> Result<Option<&'a D::Descriptor>> {
        self.charge(1)?;
        let index = self.index;
        self.found(
            index.definition(id),
            SchemaSubject::Definition(id.address()),
            site,
        )
    }
    fn slot<S: SchemaSlotId>(
        &mut self,
        id: &DeclaredSlot<S>,
        site: &BindingSite,
    ) -> Result<Option<&'a S::Descriptor>> {
        self.charge(1)?;
        let index = self.index;
        self.found(index.slot(id), SchemaSubject::Slot(S::address(id)), site)
    }
    /// Custom indexes need not sort raw DTO collections. Binding is a bounded cold
    /// path; executable plans may use compact indexed membership.
    fn membership<T: PartialEq>(
        &mut self,
        set: &DeclaredSet<T>,
        target: &T,
        subject: SchemaSubject,
        site: &BindingSite,
        purpose: Purpose,
    ) -> Result<bool> {
        self.charge(set.members.len() + 1)?;
        if let SchemaClosure::Partial { gaps } = &set.closure
            && gaps.is_empty()
        {
            return self.fault(Some(subject));
        }
        if set.members.contains(target) {
            return Ok(true);
        }
        let (class, code) = if set.is_complete() {
            (purpose.absence(), BindingIssueCode::NotDeclared)
        } else {
            (IssueClass::Unresolved, BindingIssueCode::PartialMembership)
        };
        self.issue(site, class, code, Some(subject))?;
        Ok(false)
    }
    fn closure<T>(
        &mut self,
        set: &DeclaredSet<T>,
        subject: SchemaSubject,
        site: &BindingSite,
    ) -> Result {
        if let SchemaClosure::Partial { gaps } = &set.closure {
            if gaps.is_empty() {
                return self.fault(Some(subject));
            }
            self.issue(
                site,
                IssueClass::Unresolved,
                BindingIssueCode::PartialMembership,
                Some(subject),
            )?;
        }
        Ok(())
    }
    fn role<T: PartialEq>(
        &mut self,
        allowed: &[T],
        value: &T,
        subject: Option<SchemaSubject>,
        site: &BindingSite,
    ) -> Result {
        self.charge(allowed.len() + 1)?;
        if !allowed.contains(value) {
            self.issue(
                site,
                IssueClass::Invalid,
                BindingIssueCode::IncompatibleRole,
                subject,
            )?;
        }
        Ok(())
    }
    fn declarations(
        &mut self,
        owner: &SlotOwnerDefId,
        site: &BindingSite,
    ) -> Result<Option<&'a DeclaredSlots>> {
        Ok(match owner {
            SlotOwnerDefId::Class(id) => self.definition(id, site)?.map(|d| &d.declarations),
            SlotOwnerDefId::Ascendancy(id) => self.definition(id, site)?.map(|d| &d.declarations),
            SlotOwnerDefId::Reward(id) => self.definition(id, site)?.map(|d| &d.declarations),
            SlotOwnerDefId::ItemTemplate(id) => self.definition(id, site)?.map(|d| &d.declarations),
            SlotOwnerDefId::Modifier(id) => self.definition(id, site)?.map(|d| &d.declarations),
            SlotOwnerDefId::Gem(id) => self.definition(id, site)?.map(|d| &d.declarations),
            SlotOwnerDefId::Skill(id) => self.definition(id, site)?.map(|d| &d.declarations),
            SlotOwnerDefId::PassiveNode(id) => self.definition(id, site)?.map(|d| &d.declarations),
            SlotOwnerDefId::UsagePolicy(id) => self.definition(id, site)?.map(|d| &d.declarations),
        })
    }
}
fn owner_subject(owner: &SlotOwnerDefId) -> SchemaSubject {
    SchemaSubject::Definition(match owner {
        SlotOwnerDefId::Class(id) => id.address(),
        SlotOwnerDefId::Ascendancy(id) => id.address(),
        SlotOwnerDefId::Reward(id) => id.address(),
        SlotOwnerDefId::ItemTemplate(id) => id.address(),
        SlotOwnerDefId::Modifier(id) => id.address(),
        SlotOwnerDefId::Gem(id) => id.address(),
        SlotOwnerDefId::Skill(id) => id.address(),
        SlotOwnerDefId::PassiveNode(id) => id.address(),
        SlotOwnerDefId::UsagePolicy(id) => id.address(),
    })
}
