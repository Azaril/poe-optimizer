//! Source-selected alternatives and definition-bound preparation evidence.
//!
//! This is not full LoadDB execution or a calculation admission. Items, passive
//! allocation, defaults and effective skill processing retain explicit frontiers.
mod items_passives;
mod skills_config;
use crate::{
    build_instance::{
        AuthoredInstanceId, ImportedBuildInstance, InstanceImportError, SourceOccurrenceId,
    },
    source_xml::{self, PobContentEntry, SourceXmlError},
};
use poe_optimizer_core::{
    build_identity::{BuildLineage, BuildRevision},
    build_view::{SelectionRequest, ViewRequest, WeaponStateRequest},
    data::DataIdentity,
};
use poe_optimizer_data::game_data::GameDataSnapshot;
use poe_optimizer_engine::lua_number::parse_number;
use roxmltree::Node;
use serde::{Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const SELECTED_VIEW_SCHEMA: u32 = 1;
pub const RESOLVER_SEMANTICS: &str = "poe2-authored-set-selection-v1";
#[derive(Debug, Clone, Copy)]
pub struct ResolveLimits {
    pub max_records: usize,
    pub max_fragments: usize,
    pub max_text_bytes: usize,
}
impl Default for ResolveLimits {
    fn default() -> Self {
        Self {
            max_records: 32768,
            max_fragments: 131072,
            max_text_bytes: 8 * 1024 * 1024,
        }
    }
}
#[derive(Debug, Error)]
pub enum ResolveError {
    #[error(transparent)]
    Import(#[from] InstanceImportError),
    #[error(transparent)]
    Source(#[from] SourceXmlError),
    #[error("view request: {0}")]
    Request(&'static str),
    #[error("view resolution exceeds {0} limit")]
    Resource(&'static str),
    #[error("selected view belongs to another build owner or definition snapshot")]
    ForeignBinding,
    #[error("source selection invariant: {0}")]
    Invariant(&'static str),
}
type Result<T> = std::result::Result<T, ResolveError>;

/// Bit-preserving wire value, including negative zero and non-finite selectors.
#[derive(Debug, Clone, Copy)]
pub struct NumericValue(f64);
impl NumericValue {
    pub fn new(value: f64) -> Self {
        Self(value)
    }
    pub fn value(self) -> f64 {
        self.0
    }
}
impl Serialize for NumericValue {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{:016x}", self.0.to_bits()))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionDomain {
    Skills,
    Items,
    Passives,
    Configuration,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SetOrigin {
    Authored {
        instance: AuthoredInstanceId,
        source: SourceOccurrenceId,
    },
    /// A source-loader default, not a fabricated authored/candidate instance ID.
    Default {
        domain: SelectionDomain,
        container: Option<SourceOccurrenceId>,
    },
}
impl SetOrigin {
    pub fn instance(self) -> Option<AuthoredInstanceId> {
        if let Self::Authored { instance, .. } = self {
            Some(instance)
        } else {
            None
        }
    }
    pub fn source(self) -> Option<SourceOccurrenceId> {
        match self {
            Self::Authored { source, .. } => Some(source),
            Self::Default { container, .. } => container,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct SetRecord {
    pub key: NumericValue,
    pub origin: SetOrigin,
    pub members: Vec<AuthoredInstanceId>,
}
#[derive(Debug, Clone, Serialize)]
pub struct SelectorInput {
    pub raw: Option<String>,
    pub parsed: Option<NumericValue>,
    pub requested: NumericValue,
}
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionRule {
    ExactKey,
    FirstOrderedKey,
    UpperClampedPosition,
    ExactPosition,
    LegacyPosition,
    ConstructorDefault,
    ExplicitInstance,
}
#[derive(Debug, Clone, Serialize)]
pub struct SelectionProblem {
    pub code: &'static str,
    pub source: Option<SourceOccurrenceId>,
    pub message: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct DomainSelection {
    pub domain: SelectionDomain,
    pub container: Option<SourceOccurrenceId>,
    pub authored: SelectorInput,
    pub override_instance: Option<AuthoredInstanceId>,
    pub saved_selection_problem: Option<SelectionProblem>,
    /// Every creation, including overwritten instances. No deduplication of source.
    pub sets: Vec<SetRecord>,
    /// Original loader's ordering keys; holes are retained for Config.
    pub order: Vec<Option<NumericValue>>,
    pub selected: Option<SetRecord>,
    pub rule: Option<SelectionRule>,
    pub problem: Option<SelectionProblem>,
    #[serde(skip)]
    winners: BTreeMap<u64, usize>,
}
impl DomainSelection {
    pub(super) fn new(
        domain: SelectionDomain,
        container: Option<SourceOccurrenceId>,
        authored: SelectorInput,
    ) -> Self {
        Self {
            domain,
            container,
            authored,
            override_instance: None,
            saved_selection_problem: None,
            sets: vec![],
            order: vec![],
            selected: None,
            rule: None,
            problem: None,
            winners: BTreeMap::new(),
        }
    }
    pub(super) fn fail(
        &mut self,
        code: &'static str,
        source: Option<SourceOccurrenceId>,
        message: impl Into<String>,
    ) {
        self.selected = None;
        self.rule = None;
        self.problem = Some(SelectionProblem {
            code,
            source,
            message: message.into(),
        });
    }
    fn numeric_key(key: f64) -> u64 {
        if key == 0.0 { 0 } else { key.to_bits() }
    }
    pub(super) fn push_record(&mut self, record: SetRecord) {
        debug_assert!(!record.key.value().is_nan());
        self.winners
            .insert(Self::numeric_key(record.key.value()), self.sets.len());
        self.sets.push(record);
    }
    pub(super) fn lookup(&self, key: f64) -> Option<&SetRecord> {
        self.winners
            .get(&Self::numeric_key(key))
            .map(|&index| &self.sets[index])
    }
    pub(super) fn lookup_mut(&mut self, key: f64) -> Option<&mut SetRecord> {
        let index = *self.winners.get(&Self::numeric_key(key))?;
        self.sets.get_mut(index)
    }
    pub(super) fn choose_key(&mut self) {
        let requested = self.authored.requested.value();
        let (key, rule) = if self.lookup(requested).is_some() {
            (requested, SelectionRule::ExactKey)
        } else {
            match self.order.first().and_then(|v| *v) {
                Some(key) => (key.value(), SelectionRule::FirstOrderedKey),
                None => {
                    self.fail(
                        "missing_first_key",
                        self.container,
                        "source selector has no first ordered key",
                    );
                    return;
                }
            }
        };
        self.selected = self.lookup(key).cloned();
        if self.selected.is_some() {
            self.rule = Some(rule)
        } else {
            self.fail(
                "missing_selected_key",
                self.container,
                "source selector dereferences a missing set",
            );
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct PreparationFrontier {
    pub stage: &'static str,
    pub source: Option<SourceOccurrenceId>,
    pub reason: &'static str,
}
#[derive(Debug, Clone, Serialize)]
pub struct WeaponStateEvidence {
    pub authored: Option<String>,
    pub request: WeaponStateRequest,
    pub use_second_weapon_set: Option<bool>,
}
#[derive(Debug, Clone, Serialize)]
pub struct SelectedSkillIdentity {
    pub instance: AuthoredInstanceId,
    pub source: SourceOccurrenceId,
    pub resolution: crate::skill_definitions::InstanceIdentityResolution,
}
#[derive(Debug, Clone, Serialize)]
pub struct SelectedIdentityEvidence {
    pub entries: Vec<SelectedSkillIdentity>,
    pub problem: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct SelectedViewReport {
    pub schema_version: u32,
    pub resolver_semantics: &'static str,
    pub label: String,
    pub lineage: BuildLineage,
    pub revision: BuildRevision,
    pub source_sha256: String,
    pub data: DataIdentity,
    pub skills: DomainSelection,
    pub items: DomainSelection,
    pub passives: DomainSelection,
    pub configuration: DomainSelection,
    pub weapon_state: WeaponStateEvidence,
    pub skill_identities: SelectedIdentityEvidence,
    /// Independent loader-local selection failures, including earlier repeated containers.
    pub load_problems: Vec<SelectionProblem>,
    pub frontiers: Vec<PreparationFrontier>,
}
/// Non-forgeable view ownership. Reports cannot be deserialized into this object.
#[derive(Debug)]
pub struct SelectedView<'data> {
    owner: ImportedBuildInstance,
    data: &'data GameDataSnapshot,
    report: SelectedViewReport,
}
impl SelectedView<'_> {
    pub fn report(&self) -> &SelectedViewReport {
        &self.report
    }
    pub fn validate_binding(
        &self,
        build: &ImportedBuildInstance,
        data: &GameDataSnapshot,
    ) -> Result<()> {
        if !self.owner.shares_storage_with(build) || !std::ptr::eq(self.data, data) {
            return Err(ResolveError::ForeignBinding);
        }
        Ok(())
    }
    pub fn build(&self) -> &ImportedBuildInstance {
        &self.owner
    }
}

pub(super) struct Context<'a> {
    pub build: &'a ImportedBuildInstance,
    pub data: &'a GameDataSnapshot,
    pub records_left: usize,
    pub fragments_left: usize,
    pub text_left: usize,
    sources: BTreeMap<usize, SourceOccurrenceId>,
    instances: BTreeMap<SourceOccurrenceId, AuthoredInstanceId>,
}
impl Context<'_> {
    pub fn source(&self, node: Node<'_, '_>) -> Result<SourceOccurrenceId> {
        self.sources
            .get(&node.range().start)
            .copied()
            .ok_or(ResolveError::Invariant("unmapped source node"))
    }
    pub fn instance(&self, node: Node<'_, '_>) -> Result<AuthoredInstanceId> {
        self.instances
            .get(&self.source(node)?)
            .copied()
            .ok_or(ResolveError::Invariant("unmapped authored set/member"))
    }
    pub fn origin(&self, node: Node<'_, '_>) -> Result<SetOrigin> {
        Ok(SetOrigin::Authored {
            instance: self.instance(node)?,
            source: self.source(node)?,
        })
    }
    pub fn attribute(&mut self, node: Node<'_, '_>, name: &str) -> Result<Option<String>> {
        self.attribute_at(self.source(node)?, name)
    }
    pub fn attribute_at(
        &mut self,
        source: SourceOccurrenceId,
        name: &str,
    ) -> Result<Option<String>> {
        let value = self.build.attribute(source, name)?;
        if let Some(value) = value {
            self.text_left = self
                .text_left
                .checked_sub(value.decoded().len())
                .ok_or(ResolveError::Resource("decoded text bytes"))?;
            Ok(Some(value.decoded().into()))
        } else {
            Ok(None)
        }
    }
    pub fn selector(&mut self, node: Option<Node<'_, '_>>, name: &str) -> Result<SelectorInput> {
        let raw = match node {
            Some(n) => self.attribute(n, name)?,
            None => None,
        };
        let parsed = raw.as_ref().and_then(|v| parse_number(v.as_bytes()));
        Ok(SelectorInput {
            raw,
            parsed: parsed.map(NumericValue::new),
            requested: NumericValue::new(parsed.unwrap_or(1.0)),
        })
    }
    pub fn key(&mut self, node: Node<'_, '_>) -> Result<Option<f64>> {
        Ok(self
            .attribute(node, "id")?
            .and_then(|v| parse_number(v.as_bytes())))
    }
    pub fn record(&mut self) -> Result<()> {
        self.records_left = self
            .records_left
            .checked_sub(1)
            .ok_or(ResolveError::Resource("selection records"))?;
        Ok(())
    }
    /// Original XML.lua array slots, including text entries which affect Config order.
    pub fn children<'a, 'input>(
        &mut self,
        node: Node<'a, 'input>,
    ) -> Result<Vec<Option<Node<'a, 'input>>>> {
        let content =
            source_xml::ordered_content(node, &mut self.fragments_left, &mut self.text_left)?;
        let elements: Vec<_> = node.children().filter(Node::is_element).collect();
        Ok(content
            .consumed()
            .iter()
            .map(|e| match e {
                PobContentEntry::Element { child_index } => Some(elements[*child_index]),
                PobContentEntry::Text { .. } => None,
            })
            .collect())
    }
    pub fn known(&self, node: Node<'_, '_>) -> bool {
        self.source(node)
            .ok()
            .and_then(|id| self.build.occurrence(id).ok())
            .is_some_and(|o| !o.has_namespace_context())
    }
}

fn override_selection(
    ctx: &Context<'_>,
    selection: &mut DomainSelection,
    requested: Option<AuthoredInstanceId>,
) -> Result<()> {
    let Some(id) = requested else { return Ok(()) };
    ctx.build.binding(id)?;
    selection.override_instance = Some(id);
    selection.saved_selection_problem = selection.problem.clone();
    if selection
        .problem
        .as_ref()
        .is_some_and(|p| p.code == "invalid_spec_position")
    {
        // Explicit view requests replace the saved positional choice, while
        // source-loader/preparation failures cannot be bypassed this way.
        selection.problem = None;
    } else if selection.problem.is_some() {
        return Ok(());
    }
    let found = selection
        .sets
        .iter()
        .find(|r| r.origin.instance() == Some(id));
    match found {
        None => selection.fail(
            "unavailable_override",
            selection.container,
            "requested instance is outside the final loaded set collection",
        ),
        Some(record) => {
            if selection
                .lookup(record.key.value())
                .is_some_and(|winner| winner.origin == record.origin)
            {
                selection.selected = Some(record.clone());
                selection.rule = Some(SelectionRule::ExplicitInstance);
            } else {
                selection.fail("shadowed_override",record.origin.source(),"requested occurrence was replaced by a later source key; it cannot be silently retargeted");
            }
        }
    }
    Ok(())
}
fn requested<T: Copy>(
    request: SelectionRequest<T>,
    f: impl FnOnce(T) -> AuthoredInstanceId,
) -> Option<AuthoredInstanceId> {
    match request {
        SelectionRequest::Saved => None,
        SelectionRequest::Instance(id) => Some(f(id)),
    }
}

pub fn resolve_view<'data>(
    build: &ImportedBuildInstance,
    data: &'data GameDataSnapshot,
    request: &ViewRequest,
    limits: ResolveLimits,
) -> Result<SelectedView<'data>> {
    request.validate().map_err(ResolveError::Request)?;
    source_xml::validate_ordered_xml_with_namespaces(build.source_xml())?;
    let document = crate::parse_document(build.source_xml()).map_err(InstanceImportError::from)?;
    let mut ctx = Context {
        build,
        data,
        records_left: limits.max_records,
        fragments_left: limits.max_fragments,
        text_left: limits.max_text_bytes,
        sources: build
            .occurrences()
            .iter()
            .map(|s| (s.range().start, s.id()))
            .collect(),
        instances: build
            .instances()
            .iter()
            .map(|b| (b.source(), b.instance()))
            .collect(),
    };
    let root = document.root_element();
    let roots = ctx.children(root)?;
    let mut skills = skills_config::skills(&mut ctx, None)?;
    let mut items = items_passives::items(&mut ctx, None)?;
    let mut configuration = skills_config::configuration(&mut ctx, None)?;
    let mut passives = items_passives::passives(&mut ctx, None)?;
    let mut frontiers = Vec::new();
    let mut load_problems = Vec::new();
    // Root LoadDB dispatches ordinary sections first and defers every Tree/Spec.
    for node in roots.iter().flatten().copied() {
        if !ctx.known(node) {
            frontiers.push(PreparationFrontier{stage:"namespace_context",source:Some(ctx.source(node)?),reason:"namespaced source is retained but not interpreted as a native loader section"});
            continue;
        }
        match node.tag_name().name() {
            "Skills" => {
                skills = skills_config::skills(&mut ctx, Some(node))?;
                if let Some(problem) = &skills.problem {
                    load_problems.push(problem.clone());
                }
            }
            "Items" => {
                items = items_passives::items(&mut ctx, Some(node))?;
                if let Some(problem) = &items.problem {
                    load_problems.push(problem.clone());
                }
            }
            "Config" => {
                configuration = skills_config::configuration(&mut ctx, Some(node))?;
                if let Some(problem) = &configuration.problem {
                    load_problems.push(problem.clone());
                }
            }
            _ => {}
        }
    }
    for node in roots
        .iter()
        .flatten()
        .copied()
        .filter(|n| matches!(n.tag_name().name(), "Tree" | "Spec"))
    {
        if ctx.known(node) {
            passives = items_passives::passives(&mut ctx, Some(node))?;
            if let Some(problem) = &passives.problem {
                load_problems.push(problem.clone());
            }
        }
    }
    override_selection(
        &ctx,
        &mut skills,
        requested(request.skills, AuthoredInstanceId::SkillSet),
    )?;
    override_selection(
        &ctx,
        &mut items,
        requested(request.items, AuthoredInstanceId::ItemSet),
    )?;
    override_selection(
        &ctx,
        &mut passives,
        requested(request.passives, AuthoredInstanceId::PassiveSpec),
    )?;
    override_selection(
        &ctx,
        &mut configuration,
        requested(request.configuration, AuthoredInstanceId::ConfigSet),
    )?;
    let skill_identities = selected_identities(&mut ctx, &skills)?;
    let authored = items
        .selected
        .as_ref()
        .and_then(|r| r.origin.source())
        .map(|s| ctx.attribute_at(s, "useSecondWeaponSet"))
        .transpose()?
        .flatten();
    let use_second_weapon_set = items.selected.as_ref().map(|_| match request.weapon_state {
        WeaponStateRequest::Saved => authored.as_deref() == Some("true"),
        WeaponStateRequest::Primary => false,
        WeaponStateRequest::Secondary => true,
    });
    for (stage, source, reason) in [
        (
            "skill_processing",
            skills.container,
            "selected authored groups require definition resolution, grants, supports and actor/action producers",
        ),
        (
            "item_preparation",
            items.container,
            "item registration depends on ParseRaw, base validation and BuildModList; slot references are not resolved from raw IDs",
        ),
        (
            "passive_loading",
            passives.container,
            "version validation, allocation, jewel writeback and provider effects remain separate preparation stages",
        ),
        (
            "configuration_effects",
            configuration.container,
            "configuration defaults, migrations and modifier generation require definition-driven preparation",
        ),
        (
            "root_lifecycle",
            None,
            "this projection computes independent selector evidence; it does not assert completion past an earlier loader failure",
        ),
    ] {
        frontiers.push(PreparationFrontier {
            stage,
            source,
            reason,
        });
    }
    Ok(SelectedView {
        owner: build.clone(),
        data,
        report: SelectedViewReport {
            schema_version: SELECTED_VIEW_SCHEMA,
            resolver_semantics: RESOLVER_SEMANTICS,
            label: request.label.clone(),
            lineage: build.lineage(),
            revision: build.revision(),
            source_sha256: build.source_sha256().into(),
            data: data.identity().clone(),
            skills,
            items,
            passives,
            configuration,
            weapon_state: WeaponStateEvidence {
                authored,
                request: request.weapon_state,
                use_second_weapon_set,
            },
            skill_identities,
            load_problems,
            frontiers,
        },
    })
}

fn selected_identities(
    ctx: &mut Context<'_>,
    selection: &DomainSelection,
) -> Result<SelectedIdentityEvidence> {
    use crate::skill_definitions::{SkillIdentityRecord, lookup_definitions};
    let mut output = SelectedIdentityEvidence {
        entries: vec![],
        problem: None,
    };
    let Some(selected) = &selection.selected else {
        return Ok(output);
    };
    let members: BTreeSet<_> = selected.members.iter().copied().collect();
    let projection = match ctx.build.project_skills() {
        Ok(p) => p,
        Err(e) => {
            output.problem = Some(e.to_string());
            return Ok(output);
        }
    };
    let lookup = match lookup_definitions(&projection, ctx.data) {
        Ok(p) => p,
        Err(e) => {
            output.problem = Some(e.to_string());
            return Ok(output);
        }
    };
    for located in lookup.records() {
        let Some(range) = &located.group_source_range else {
            continue;
        };
        let Some(group_source) = ctx.sources.get(&range.start) else {
            continue;
        };
        let Some(group) = ctx.instances.get(group_source) else {
            continue;
        };
        if !members.contains(group) {
            continue;
        }
        if let SkillIdentityRecord::Instance(record) = &located.record {
            let source = *ctx
                .sources
                .get(&record.source_range.start)
                .ok_or(ResolveError::Invariant("unmapped identity source"))?;
            let instance = *ctx
                .instances
                .get(&source)
                .ok_or(ResolveError::Invariant("unmapped identity entry"))?;
            ctx.record()?;
            charge_identity_text(ctx, &record.resolution)?;
            output.entries.push(SelectedSkillIdentity {
                instance,
                source,
                resolution: record.resolution.clone(),
            });
        }
    }
    Ok(output)
}

fn charge_identity_text(
    ctx: &mut Context<'_>,
    resolution: &crate::skill_definitions::InstanceIdentityResolution,
) -> Result<()> {
    use crate::skill_definitions::InstanceIdentityResolution as R;
    let mut charge = |text: &str| -> Result<()> {
        ctx.text_left = ctx
            .text_left
            .checked_sub(text.len())
            .ok_or(ResolveError::Resource("definition identity text bytes"))?;
        Ok(())
    };
    match resolution {
        R::ExternalGem { candidates, .. } => {
            for gem in candidates {
                for text in [
                    &gem.key,
                    &gem.game_id,
                    &gem.variant_id,
                    &gem.name,
                    &gem.primary_effect_id,
                ] {
                    charge(text)?;
                }
                for text in &gem.constructed_effect_list {
                    charge(text)?;
                }
            }
        }
        R::ExplicitEffect {
            matched,
            possible_primary_gem_keys,
        } => {
            if let Some(skill) = matched {
                charge(&skill.id)?;
                charge(&skill.name)?;
            }
            for text in possible_primary_gem_keys {
                charge(text)?;
            }
        }
        R::NameOnlyNotResolved | R::MissingIdentity => {}
    }
    Ok(())
}
