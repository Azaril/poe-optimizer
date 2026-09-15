//! Bounded offline passive-tree identity and structural-schema compilation.
//!
//! Source keys, text and pins remain Import authoring evidence. This compiler
//! never evaluates stat text, repairs a prior binding, or certifies game legality.
use crate::{owned_mapping::*, owned_recipe::*, owned_tree_policy::*};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::DeclaredSlot,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::DefinitionRules,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const OWNED_TREE_CATALOG_VERSION: u32 = 1;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeCatalogInput {
    pub schema_version: u32,
    pub tree_version: String,
    pub source: SourcePin,
    pub classes: Vec<TreeClassInput>,
    pub nodes: Vec<TreeNodeInput>,
    pub edges: Vec<TreeEdgeInput>,
    pub unresolved_edges: Vec<TreeEdgeInput>,
    pub attribute_options: Vec<TreeAttributeOptionInput>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeClassInput {
    pub key: String,
    pub root: String,
    pub ascendancies: Vec<TreeAscendancyInput>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeAscendancyInput {
    pub key: String,
    pub ordinal: u16,
    pub root: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeNodeInput {
    pub key: String,
    pub kind: TreeNodeKind,
    pub stats: Vec<String>,
    pub views: Vec<TreeStatViewInput>,
    pub unlock: Vec<String>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TreeNodeKind {
    ImplicitRoot,
    Allocation { pool: TreePoolKind },
    Attribute { pool: TreePoolKind },
    AttachedChoice { parent: String },
    Unsupported { code: OwnedDefinitionKey },
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreePoolKind {
    Ordinary,
    Ascendancy,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeStatViewInput {
    pub selector: String,
    pub stats: Vec<String>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeEdgeInput {
    pub left: String,
    pub right: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeAttributeOptionInput {
    pub key: String,
    pub stats: Vec<String>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeCatalogPolicy {
    pub version: OwnedDefinitionKey,
    pub level: IntegerRange,
    pub syntax: TreeNormalizationSyntax,
}
#[derive(Clone, Copy, Debug)]
pub struct TreeCatalogLimits {
    pub max_wire_bytes: usize,
    pub max_output_bytes: usize,
    pub max_rows: usize,
    pub max_entries: usize,
    pub max_text_bytes: usize,
    pub max_string_bytes: usize,
    pub max_work: usize,
    pub recipe: OwnedRecipeLimits,
    pub mapping: OwnedMappingLimits,
}
impl Default for TreeCatalogLimits {
    fn default() -> Self {
        Self {
            max_wire_bytes: 8 * 1024 * 1024,
            max_output_bytes: 64 * 1024 * 1024,
            max_rows: 20_000,
            max_entries: 1_000_000,
            max_text_bytes: 8 * 1024 * 1024,
            max_string_bytes: 16 * 1024,
            max_work: 64 * 1024 * 1024,
            recipe: OwnedRecipeLimits::default(),
            mapping: OwnedMappingLimits::default(),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum TreeCatalogError {
    #[error("tree catalog exceeds {0}")]
    Limit(&'static str),
    #[error("invalid tree catalog: {0}")]
    Invalid(String),
    #[error("tree catalog prior binding or source footprint mismatch")]
    Binding,
    #[error("tree catalog cannot reuse unresolved or contradictory selector")]
    Reuse,
    #[error("tree catalog would change an existing descriptor or owner program")]
    Preservation,
    #[error(transparent)]
    Mapping(#[from] OwnedMappingError),
    #[error(transparent)]
    Recipe(#[from] OwnedRecipeError),
    #[error(transparent)]
    Schema(#[from] SchemaPackageError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, TreeCatalogError>;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeCatalogCounts {
    pub reused_definitions: usize,
    pub allocated_definitions: usize,
    pub reused_slots: usize,
    pub allocated_slots: usize,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeCatalogReceipt {
    pub catalog: OwnedContentDigest,
    pub policy: OwnedContentDigest,
    pub before_registry: OwnedContentDigest,
    pub after_registry: OwnedContentDigest,
    pub before_definitions: DataIdentity,
    pub after_definitions: DataIdentity,
    pub counts: TreeCatalogCounts,
    pub work_used: usize,
}
#[derive(Clone, Debug)]
pub struct StagedTreeCatalog {
    pub successor: OwnedRecipeInput,
    /// Only selectors absent from the prior mapping; reruns emit no old rows.
    pub new_mappings: Vec<MappingEntry>,
    pub content: TreeNormalizationContent,
    pub receipt: TreeCatalogReceipt,
}

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).expect("compiler-owned code")
}
fn invalid(s: impl Into<String>) -> TreeCatalogError {
    TreeCatalogError::Invalid(s.into())
}
fn text(s: &str) -> SourceComponent {
    SourceComponent::Text(s.to_owned())
}
fn subject<I: SchemaDefinitionId>(id: &I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}
fn gap(subject: &SchemaSubject, facet: SchemaFacet, code: &str) -> SchemaGap {
    SchemaGap {
        subject: subject.clone(),
        facet,
        code: key(code),
    }
}
fn partial<T>(subject: &SchemaSubject, values: Vec<T>) -> DeclaredSet<T> {
    DeclaredSet::partial(
        values,
        vec![gap(
            subject,
            SchemaFacet::InputSchema,
            "tree-declarations-not-converted",
        )],
    )
}
fn declarations(
    owner: &SchemaSubject,
    choices: Vec<DeclaredSlot<ChoiceSlotDefId>>,
) -> DeclaredSlots {
    DeclaredSlots {
        parameters: partial(owner, vec![]),
        choices: partial(owner, choices),
        grants: partial(owner, vec![]),
        actors: partial(owner, vec![]),
        skill_grants: partial(owner, vec![]),
        outputs: partial(owner, vec![]),
        sockets: partial(owner, vec![]),
    }
}
fn source_node(version: &str, node: &str) -> ExternalOwnerSelector {
    ExternalOwnerSelector::PassiveNode {
        tree_version: text(version),
        node_id: text(node),
        view: SourceComponent::Missing,
    }
}
fn catalog_selector(
    version: &str,
    kind: ExternalCatalogKind,
    name: &str,
    variant: &str,
) -> ExternalSelector {
    ExternalSelector::Catalog {
        kind,
        key: text(name),
        version: text(version),
        variant: text(variant),
    }
}
impl TreeCatalogLimits {
    fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (name, value, maximum) in [
            ("wire bytes", self.max_wire_bytes, hard.max_wire_bytes),
            ("output bytes", self.max_output_bytes, hard.max_output_bytes),
            ("rows", self.max_rows, hard.max_rows),
            ("entries", self.max_entries, hard.max_entries),
            ("text bytes", self.max_text_bytes, hard.max_text_bytes),
            ("string bytes", self.max_string_bytes, hard.max_string_bytes),
            ("work", self.max_work, hard.max_work),
        ] {
            if value == 0 || value > maximum {
                return Err(TreeCatalogError::Limit(name));
            }
        }
        self.mapping.validate()?;
        Ok(())
    }
}
struct Budget {
    limits: TreeCatalogLimits,
    work: usize,
    entries: usize,
    text: usize,
}
impl Budget {
    fn work(&mut self, n: usize) -> Result<()> {
        self.work = self
            .work
            .checked_add(n)
            .ok_or(TreeCatalogError::Limit("work"))?;
        if self.work > self.limits.max_work {
            return Err(TreeCatalogError::Limit("work"));
        }
        Ok(())
    }
    fn rows(&mut self, n: usize) -> Result<()> {
        if n > self.limits.max_rows {
            return Err(TreeCatalogError::Limit("rows"));
        }
        self.expansion(n)
    }
    fn expansion(&mut self, n: usize) -> Result<()> {
        self.entries = self
            .entries
            .checked_add(n)
            .ok_or(TreeCatalogError::Limit("entries"))?;
        if self.entries > self.limits.max_entries {
            return Err(TreeCatalogError::Limit("entries"));
        }
        self.work(n)
    }
    fn string(&mut self, s: &str, empty: bool) -> Result<()> {
        if (!empty && s.is_empty()) || s.len() > self.limits.max_string_bytes {
            return Err(invalid("empty or oversized source string"));
        }
        self.text = self
            .text
            .checked_add(s.len())
            .ok_or(TreeCatalogError::Limit("text bytes"))?;
        if self.text > self.limits.max_text_bytes {
            return Err(TreeCatalogError::Limit("text bytes"));
        }
        self.work(s.len())
    }
    fn lookup(&mut self, s: &str) -> Result<()> {
        // Every comparison in a bounded balanced source index may inspect this text.
        self.work(s.len().saturating_add(1).saturating_mul(32))
    }
}
fn check_input(input: &TreeCatalogInput, policy: &TreeCatalogPolicy, b: &mut Budget) -> Result<()> {
    if input.schema_version != OWNED_TREE_CATALOG_VERSION {
        return Err(invalid("unsupported catalog version"));
    }
    if policy.level.minimum > policy.level.maximum {
        return Err(invalid("reversed level domain"));
    }
    b.string(&input.tree_version, false)?;
    b.string(&input.source.revision, false)?;
    b.rows(input.source.files.len())?;
    if input.source.files.is_empty() {
        return Err(invalid("empty source footprint"));
    }
    for pin in &input.source.files {
        b.string(&pin.path, false)?;
        b.string(&pin.sha256, false)?;
    }
    b.rows(input.classes.len())?;
    b.rows(input.nodes.len())?;
    b.rows(input.edges.len())?;
    b.rows(input.unresolved_edges.len())?;
    b.rows(input.attribute_options.len())?;
    if input.classes.is_empty() || input.nodes.is_empty() {
        return Err(invalid("empty class/node catalog"));
    }
    for class in &input.classes {
        b.string(&class.key, false)?;
        b.string(&class.root, false)?;
        b.rows(class.ascendancies.len())?;
        for asc in &class.ascendancies {
            b.string(&asc.key, false)?;
            b.string(&asc.root, false)?;
        }
    }
    for node in &input.nodes {
        b.string(&node.key, false)?;
        b.rows(node.stats.len())?;
        b.rows(node.views.len())?;
        b.rows(node.unlock.len())?;
        for s in &node.stats {
            b.string(s, true)?;
        }
        for view in &node.views {
            b.string(&view.selector, false)?;
            b.rows(view.stats.len())?;
            for s in &view.stats {
                b.string(s, true)?;
            }
        }
        for s in &node.unlock {
            b.string(s, false)?;
        }
        if let TreeNodeKind::AttachedChoice { parent } = &node.kind {
            b.string(parent, false)?;
        }
    }
    for edge in input.edges.iter().chain(&input.unresolved_edges) {
        b.string(&edge.left, false)?;
        b.string(&edge.right, false)?;
    }
    for option in &input.attribute_options {
        b.string(&option.key, false)?;
        b.rows(option.stats.len())?;
        for s in &option.stats {
            b.string(s, true)?;
        }
    }
    Ok(())
}
/// Decode only the strict neutral catalog; compilation validates relationships and
/// exact prior artifact bindings. No source file is read by this module.
pub fn decode_tree_catalog(bytes: &[u8], limits: TreeCatalogLimits) -> Result<TreeCatalogInput> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(TreeCatalogError::Limit("wire bytes"));
    }
    Ok(serde_json::from_slice(bytes)?)
}
fn union_source(old: &SourcePin, new: &SourcePin, b: &mut Budget) -> Result<SourcePin> {
    if old.system != new.system || old.revision != new.revision {
        return Err(TreeCatalogError::Binding);
    }
    b.rows(old.files.len())?;
    let mut files: BTreeMap<&str, &SourceFilePin> =
        old.files.iter().map(|p| (p.path.as_str(), p)).collect();
    let mut seen = BTreeSet::new();
    for pin in &new.files {
        b.lookup(&pin.path)?;
        if !seen.insert(&pin.path) {
            return Err(invalid("duplicate source pin"));
        }
        if let Some(previous) = files.insert(&pin.path, pin)
            && previous.sha256 != pin.sha256
        {
            return Err(TreeCatalogError::Binding);
        }
    }
    Ok(SourcePin {
        system: old.system,
        revision: old.revision.clone(),
        files: files.into_values().cloned().collect(),
    })
}
struct Allocator<'a> {
    registry: OwnedIdRegistry,
    mapping: &'a OwnedMappingIndex,
    new_mappings: Vec<MappingEntry>,
    counts: TreeCatalogCounts,
}
impl Allocator<'_> {
    fn resolve(
        &mut self,
        selector: ExternalSelector,
        fresh: impl FnOnce(
            &mut OwnedIdRegistry,
        ) -> std::result::Result<SchemaSubject, OwnedMappingError>,
    ) -> Result<SchemaSubject> {
        if let Some(outcome) = self.mapping.lookup(&selector) {
            let MappingOutcome::Mapped { target, .. } = outcome else {
                return Err(TreeCatalogError::Reuse);
            };
            if !self
                .registry
                .entry(target)
                .is_some_and(|e| matches!(e.state, RegistryState::Active))
            {
                return Err(TreeCatalogError::Reuse);
            }
            match target {
                SchemaSubject::Definition(_) => self.counts.reused_definitions += 1,
                SchemaSubject::Slot(_) => self.counts.reused_slots += 1,
            }
            return Ok(target.clone());
        }
        let target = fresh(&mut self.registry)?;
        match &target {
            SchemaSubject::Definition(_) => self.counts.allocated_definitions += 1,
            SchemaSubject::Slot(_) => self.counts.allocated_slots += 1,
        }
        self.new_mappings.push(MappingEntry {
            source: selector,
            outcome: MappingOutcome::Mapped {
                target: target.clone(),
                basis: MappingBasis::Exact,
            },
        });
        Ok(target)
    }
    fn choice(
        &mut self,
        selector: ExternalSelector,
        parent: PassiveNodeDefId,
    ) -> Result<DeclaredSlot<ChoiceSlotDefId>> {
        let owner = SlotOwnerDefId::PassiveNode(parent);
        let target = self.resolve(selector, |r| {
            Ok(SchemaSubject::Slot(SlotAddress::Choice(
                r.allocate_slot::<ChoiceSlotDefinition>(owner.clone())?,
            )))
        })?;
        match target {
            SchemaSubject::Slot(SlotAddress::Choice(v)) if v.declaration == owner => Ok(v),
            _ => Err(TreeCatalogError::Reuse),
        }
    }
}
macro_rules! allocate_method {
    ($method:ident,$domain:ident,$variant:ident,$id:ident) => {
        impl Allocator<'_> {
            fn $method(&mut self, selector: ExternalSelector) -> Result<$id> {
                match self.resolve(selector, |r| {
                    Ok(subject(&r.allocate_definition::<$domain>()?))
                })? {
                    SchemaSubject::Definition(DefinitionAddress::$variant(id)) => Ok(id),
                    _ => Err(TreeCatalogError::Reuse),
                }
            }
        }
    };
}
allocate_method!(class, ClassDefinition, Class, ClassDefId);
allocate_method!(
    ascendancy,
    AscendancyDefinition,
    Ascendancy,
    AscendancyDefId
);
allocate_method!(node, PassiveNodeDefinition, PassiveNode, PassiveNodeDefId);
allocate_method!(pool, PointPoolDefinition, PointPool, PointPoolDefId);
allocate_method!(option, OptionDefinition, Option, OptionDefId);
fn preserve_definition(
    definitions: &mut BTreeMap<DefinitionAddress, DefinitionDescriptor>,
    proposed: DefinitionDescriptor,
) -> Result<()> {
    match definitions.get(&proposed.address()) {
        Some(existing) if *existing != proposed => Err(TreeCatalogError::Preservation),
        Some(_) => Ok(()),
        None => {
            definitions.insert(proposed.address(), proposed);
            Ok(())
        }
    }
}
fn preserve_slot(
    slots: &mut BTreeMap<SlotAddress, SlotDescriptor>,
    proposed: SlotDescriptor,
) -> Result<()> {
    match slots.get(&proposed.address()) {
        Some(existing) if *existing != proposed => Err(TreeCatalogError::Preservation),
        Some(_) => Ok(()),
        None => {
            slots.insert(proposed.address(), proposed);
            Ok(())
        }
    }
}
fn definition<I, D>(id: I, schema: D) -> DefinitionEntry<I, D> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn physical(node: &TreeNodeInput) -> bool {
    matches!(
        node.kind,
        TreeNodeKind::ImplicitRoot
            | TreeNodeKind::Allocation { .. }
            | TreeNodeKind::Attribute { .. }
    )
}

/// Extend one explicitly supplied, validated endpoint. Existing IDs, registry
/// history, descriptors and programs are immutable. Only positive exact-domain
/// mappings are reusable; unresolved mappings never allocate replacement IDs.
pub fn compile_owned_tree_catalog_extension(
    base: &StagedOwnedRecipe,
    mapping: &OwnedMappingIndex,
    input: &TreeCatalogInput,
    policy: &TreeCatalogPolicy,
    limits: TreeCatalogLimits,
) -> Result<StagedTreeCatalog> {
    limits.validate()?;
    let catalog_digest = digest_owned("owned-tree-catalog-v1", input, limits.max_wire_bytes)?;
    let policy_digest = digest_owned(
        "owned-tree-catalog-policy-v1",
        policy,
        limits.max_wire_bytes,
    )?;
    digest_owned(
        "owned-tree-prior-v1",
        &(
            base.registry().input(),
            base.schema().input(),
            base.rules().input(),
            base.routing().input(),
        ),
        limits.recipe.max_wire_bytes,
    )?;
    let mut budget = Budget {
        limits,
        work: 0,
        entries: 0,
        text: 0,
    };
    check_input(input, policy, &mut budget)?;
    mapping.verify_bindings(
        base.registry(),
        base.schema(),
        &mapping.input().source,
        &mapping.input().policy_version,
        limits.mapping,
    )?;
    let source = union_source(&mapping.input().source, &input.source, &mut budget)?;
    for n in [
        base.registry().input().entries.len(),
        base.schema().input().definitions.len(),
        base.schema().input().slots.len(),
        mapping.input().entries.len(),
    ] {
        budget.rows(n)?;
    }
    // Revalidate the immutable endpoint under this caller's tighter nested limits.
    let checked_base = assemble_owned_recipe(
        OwnedRecipeInput {
            schema_version: OWNED_RECIPE_VERSION,
            registry: base.registry().input().clone(),
            schema: base.schema().input().clone(),
            rules: base.rules().input().clone(),
            routing: base.routing().input().clone(),
        },
        limits.recipe,
    )?;
    let mut allocator = Allocator {
        registry: OwnedIdRegistry::new(checked_base.registry().input().clone(), limits.mapping)?,
        mapping,
        new_mappings: vec![],
        counts: TreeCatalogCounts::default(),
    };
    let mut nodes = BTreeMap::new();
    for node in &input.nodes {
        budget.lookup(&node.key)?;
        if nodes.insert(node.key.as_str(), node).is_some() {
            return Err(invalid("duplicate node key"));
        }
        let mut views = BTreeSet::new();
        for view in &node.views {
            budget.lookup(&view.selector)?;
            if !views.insert(&view.selector) {
                return Err(invalid("duplicate stat view"));
            }
        }
        let mut unlock = BTreeSet::new();
        for token in &node.unlock {
            budget.lookup(token)?;
            if !unlock.insert(token) {
                return Err(invalid("duplicate prerequisite"));
            }
        }
    }
    for node in nodes.values() {
        for prerequisite in &node.unlock {
            budget.lookup(prerequisite)?;
            if !nodes.contains_key(prerequisite.as_str()) {
                return Err(invalid("unknown prerequisite"));
            }
        }
    }
    let mut classes = BTreeMap::new();
    let mut expected_roots = BTreeSet::new();
    for class in &input.classes {
        budget.lookup(&class.key)?;
        if classes.insert(class.key.as_str(), class).is_some() {
            return Err(invalid("duplicate class key"));
        }
        let mut names = BTreeSet::new();
        let mut ordinals = BTreeSet::new();
        for asc in &class.ascendancies {
            budget.lookup(&asc.key)?;
            if asc.ordinal == 0 || !names.insert(&asc.key) || !ordinals.insert(asc.ordinal) {
                return Err(invalid("duplicate/zero ascendancy key or ordinal"));
            }
        }
        if ordinals
            .iter()
            .copied()
            .ne(1..=u16::try_from(class.ascendancies.len())
                .map_err(|_| invalid("ascendancy ordinal overflow"))?)
        {
            return Err(invalid("noncontiguous ascendancy ordinals"));
        }
        for root in std::iter::once(&class.root).chain(class.ascendancies.iter().map(|a| &a.root)) {
            budget.lookup(root)?;
            if !nodes
                .get(root.as_str())
                .is_some_and(|n| matches!(n.kind, TreeNodeKind::ImplicitRoot))
            {
                return Err(invalid("class/ascendancy root not declared implicit"));
            }
            expected_roots.insert(root.as_str());
        }
    }
    for node in nodes.values() {
        if matches!(node.kind, TreeNodeKind::ImplicitRoot)
            && !expected_roots.contains(node.key.as_str())
        {
            return Err(invalid("orphan implicit root"));
        }
    }
    let mut edges = BTreeSet::new();
    let mut adjacent: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut unresolved_adjacent = BTreeSet::new();
    for edge in &input.edges {
        budget.lookup(&edge.left)?;
        budget.lookup(&edge.right)?;
        let left = nodes
            .get(edge.left.as_str())
            .ok_or_else(|| invalid("unknown edge endpoint"))?;
        let right = nodes
            .get(edge.right.as_str())
            .ok_or_else(|| invalid("unknown edge endpoint"))?;
        let pair = if edge.left < edge.right {
            (edge.left.as_str(), edge.right.as_str())
        } else {
            (edge.right.as_str(), edge.left.as_str())
        };
        if edge.left == edge.right || !edges.insert(pair) {
            return Err(invalid("duplicate/self edge"));
        }
        if physical(left) && physical(right) {
            adjacent.entry(&left.key).or_default().insert(&right.key);
            adjacent.entry(&right.key).or_default().insert(&left.key);
        } else {
            if physical(left) && matches!(right.kind, TreeNodeKind::Unsupported { .. }) {
                unresolved_adjacent.insert(left.key.as_str());
            }
            if physical(right) && matches!(left.kind, TreeNodeKind::Unsupported { .. }) {
                unresolved_adjacent.insert(right.key.as_str());
            }
        }
    }
    let mut dangling = BTreeSet::new();
    for edge in &input.unresolved_edges {
        budget.lookup(&edge.left)?;
        budget.lookup(&edge.right)?;
        if !nodes.contains_key(edge.left.as_str())
            || nodes.contains_key(edge.right.as_str())
            || !dangling.insert((&edge.left, &edge.right))
        {
            return Err(invalid("invalid/duplicate unresolved edge"));
        }
        unresolved_adjacent.insert(edge.left.as_str());
    }
    let mut attached: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for node in nodes.values() {
        if let TreeNodeKind::AttachedChoice { parent } = &node.kind {
            budget.lookup(parent)?;
            if !nodes
                .get(parent.as_str())
                .is_some_and(|p| matches!(p.kind, TreeNodeKind::Allocation { .. }))
            {
                return Err(invalid("attached choice has no ordinary physical parent"));
            }
            let pair = if node.key < *parent {
                (node.key.as_str(), parent.as_str())
            } else {
                (parent.as_str(), node.key.as_str())
            };
            if !edges.contains(&pair) {
                return Err(invalid("attached choice lacks semantic parent edge"));
            }
            attached.entry(parent).or_default().push(&node.key);
        }
        if !physical(node)
            && mapping
                .lookup(&ExternalSelector::Definition(source_node(
                    &input.tree_version,
                    &node.key,
                )))
                .is_some()
        {
            return Err(TreeCatalogError::Reuse);
        }
    }
    let mut attribute_options = BTreeMap::new();
    for option in &input.attribute_options {
        budget.lookup(&option.key)?;
        if attribute_options
            .insert(option.key.as_str(), option)
            .is_some()
        {
            return Err(invalid("duplicate attribute lane"));
        }
    }
    if nodes
        .values()
        .any(|n| matches!(n.kind, TreeNodeKind::Attribute { .. }))
        && attribute_options.is_empty()
    {
        return Err(invalid("attribute node without explicit alternatives"));
    }
    // Bound multiplicative choice/attribute output before allocating any IDs or
    // cloning repeated lane names. Count every occurrence, not just unique keys.
    let attribute_count = nodes
        .values()
        .filter(|n| matches!(n.kind, TreeNodeKind::Attribute { .. }))
        .count();
    let lane_count = attribute_count
        .checked_mul(attribute_options.len())
        .ok_or(TreeCatalogError::Limit("expanded choices"))?;
    let physical_count = nodes.values().filter(|n| physical(n)).count();
    budget.expansion(
        physical_count
            .checked_mul(32)
            .ok_or(TreeCatalogError::Limit("expanded declarations"))?,
    )?;
    budget.expansion(
        lane_count
            .checked_mul(3)
            .ok_or(TreeCatalogError::Limit("expanded choices"))?,
    )?;
    let lane_text = attribute_options
        .keys()
        .try_fold(0usize, |n, k| n.checked_add(k.len()))
        .ok_or(TreeCatalogError::Limit("expanded lane text"))?;
    let repeated_text = lane_text
        .checked_mul(attribute_count)
        .ok_or(TreeCatalogError::Limit("expanded lane text"))?;
    if repeated_text > limits.max_output_bytes {
        return Err(TreeCatalogError::Limit("expanded lane text"));
    }
    budget.work(repeated_text)?;
    let mut definitions: BTreeMap<_, _> = checked_base
        .schema()
        .input()
        .definitions
        .iter()
        .map(|d| (d.address(), d.clone()))
        .collect();
    let mut slots: BTreeMap<_, _> = checked_base
        .schema()
        .input()
        .slots
        .iter()
        .map(|s| (s.address(), s.clone()))
        .collect();
    let mut class_ids = BTreeMap::new();
    let mut ascendancy_ids = BTreeMap::new();
    let mut node_ids = BTreeMap::new();
    let mut pool_ids = BTreeMap::new();
    let mut option_ids = BTreeMap::new();
    let mut attached_ids = BTreeMap::new();
    let mut choice_slots = BTreeMap::new();
    // Stable lexical source order and fixed definition-family order; no symbol is
    // derived from a source name. Registry allocation remains the sole ID source.
    for class in classes.values() {
        budget.lookup(&class.key)?;
        class_ids.insert(
            class.key.as_str(),
            allocator.class(ExternalSelector::Definition(ExternalOwnerSelector::Class {
                key: text(&class.key),
            }))?,
        );
    }
    for class in classes.values() {
        let sorted: BTreeMap<_, _> = class
            .ascendancies
            .iter()
            .map(|a| (a.key.as_str(), a))
            .collect();
        for asc in sorted.values() {
            budget.lookup(&asc.key)?;
            ascendancy_ids.insert(
                (class.key.as_str(), asc.key.as_str()),
                allocator.ascendancy(ExternalSelector::Definition(
                    ExternalOwnerSelector::Ascendancy {
                        class: text(&class.key),
                        key: text(&asc.key),
                    },
                ))?,
            );
        }
    }
    for node in nodes.values().filter(|n| physical(n)) {
        budget.lookup(&node.key)?;
        node_ids.insert(
            node.key.as_str(),
            allocator.node(ExternalSelector::Definition(source_node(
                &input.tree_version,
                &node.key,
            )))?,
        );
    }
    for (pool, name, scope) in [
        (TreePoolKind::Ordinary, "ordinary", PointPoolScope::Either),
        (
            TreePoolKind::Ascendancy,
            "ascendancy",
            PointPoolScope::Shared,
        ),
    ] {
        let id = allocator.pool(catalog_selector(
            &input.tree_version,
            ExternalCatalogKind::PointPool,
            name,
            "tree-pool",
        ))?;
        preserve_definition(
            &mut definitions,
            DefinitionDescriptor::PointPool(definition(id.clone(), PointPoolSchema { scope })),
        )?;
        pool_ids.insert(pool, id);
    }
    for option in attribute_options.values() {
        budget.lookup(&option.key)?;
        let id = allocator.option(catalog_selector(
            &input.tree_version,
            ExternalCatalogKind::Option,
            &option.key,
            "attribute",
        ))?;
        preserve_definition(
            &mut definitions,
            DefinitionDescriptor::Option(definition(id.clone(), OptionSchema {})),
        )?;
        option_ids.insert(option.key.as_str(), id);
    }
    for node in nodes
        .values()
        .filter(|n| matches!(n.kind, TreeNodeKind::AttachedChoice { .. }))
    {
        budget.lookup(&node.key)?;
        let id = allocator.option(catalog_selector(
            &input.tree_version,
            ExternalCatalogKind::Option,
            &node.key,
            "attached",
        ))?;
        preserve_definition(
            &mut definitions,
            DefinitionDescriptor::Option(definition(id.clone(), OptionSchema {})),
        )?;
        attached_ids.insert(node.key.as_str(), id);
    }
    for node in nodes.values().filter(|n| {
        matches!(n.kind, TreeNodeKind::Attribute { .. }) || attached.contains_key(n.key.as_str())
    }) {
        budget.lookup(&node.key)?;
        let id = allocator.choice(
            ExternalSelector::Slot {
                owner: source_node(&input.tree_version, &node.key),
                kind: ExternalSlotKind::Choice,
                key: text("options"),
            },
            node_ids[&node.key.as_str()].clone(),
        )?;
        let mut allowed: Vec<_> = if matches!(node.kind, TreeNodeKind::Attribute { .. }) {
            option_ids.values().cloned().collect()
        } else {
            attached[node.key.as_str()]
                .iter()
                .map(|n| attached_ids[n].clone())
                .collect()
        };
        allowed.sort();
        preserve_slot(
            &mut slots,
            SlotDescriptor::Choice(definition(
                id.clone(),
                ChoiceSlotSchema {
                    value: ValueSchema::Option {
                        allowed: DeclaredSet::complete(allowed),
                    },
                    presence: SlotPresence::RequiredOnce,
                    owners: vec![ChoiceOwnerScope::Allocation],
                },
            )),
        )?;
        choice_slots.insert(node.key.as_str(), id);
    }
    let mut new_owners = vec![];
    for class in classes.values() {
        let id = class_ids[class.key.as_str()].clone();
        let owner = subject(&id);
        let mut asc: Vec<_> = class
            .ascendancies
            .iter()
            .map(|a| ascendancy_ids[&(class.key.as_str(), a.key.as_str())].clone())
            .collect();
        asc.sort();
        preserve_definition(
            &mut definitions,
            DefinitionDescriptor::Class(definition(
                id,
                ClassSchema {
                    level: policy.level.clone(),
                    ascendancies: DeclaredSet::complete(asc),
                    implicit_passives: DeclaredSet::complete(vec![
                        node_ids[class.root.as_str()].clone(),
                    ]),
                    declarations: declarations(&owner, vec![]),
                },
            )),
        )?;
        new_owners.push(owner);
        for asc in &class.ascendancies {
            let id = ascendancy_ids[&(class.key.as_str(), asc.key.as_str())].clone();
            let owner = subject(&id);
            preserve_definition(
                &mut definitions,
                DefinitionDescriptor::Ascendancy(definition(
                    id,
                    AscendancySchema {
                        classes: DeclaredSet::complete(vec![class_ids[class.key.as_str()].clone()]),
                        implicit_passives: DeclaredSet::complete(vec![
                            node_ids[asc.root.as_str()].clone(),
                        ]),
                        declarations: declarations(&owner, vec![]),
                    },
                )),
            )?;
            new_owners.push(owner);
        }
    }
    let mut tokens = vec![];
    let mut attributes = vec![];
    for node in nodes.values() {
        budget.lookup(&node.key)?;
        if let Some(id) = node_ids.get(node.key.as_str()) {
            let owner = subject(id);
            let pools = match node.kind {
                TreeNodeKind::ImplicitRoot => vec![],
                TreeNodeKind::Allocation { pool } | TreeNodeKind::Attribute { pool } => {
                    vec![pool_ids[&pool].clone()]
                }
                _ => unreachable!(),
            };
            let mut neighbors: Vec<_> = adjacent
                .get(node.key.as_str())
                .into_iter()
                .flatten()
                .map(|n| node_ids[n].clone())
                .collect();
            neighbors.sort();
            let links = if unresolved_adjacent.contains(node.key.as_str()) {
                DeclaredSet::partial(
                    neighbors,
                    vec![gap(
                        &owner,
                        SchemaFacet::StaticLinks,
                        "tree-adjacency-not-converted",
                    )],
                )
            } else {
                DeclaredSet::complete(neighbors)
            };
            preserve_definition(
                &mut definitions,
                DefinitionDescriptor::PassiveNode(definition(
                    id.clone(),
                    PassiveNodeSchema {
                        pools: DeclaredSet::complete(pools),
                        adjacent: links,
                        declarations: declarations(
                            &owner,
                            choice_slots
                                .get(node.key.as_str())
                                .cloned()
                                .into_iter()
                                .collect(),
                        ),
                    },
                )),
            )?;
            new_owners.push(owner);
        }
        let role = match &node.kind {
            TreeNodeKind::ImplicitRoot => TreeTokenRole::ImplicitRoot {
                node: node_ids[node.key.as_str()].clone(),
            },
            TreeNodeKind::Allocation { pool } | TreeNodeKind::Attribute { pool } => {
                TreeTokenRole::Allocation {
                    node: node_ids[node.key.as_str()].clone(),
                    pool: pool_ids[pool].clone(),
                }
            }
            TreeNodeKind::AttachedChoice { parent } => TreeTokenRole::AttachedChoice {
                parent: node_ids[parent.as_str()].clone(),
                slot: choice_slots[parent.as_str()].clone(),
                option: attached_ids[node.key.as_str()].clone(),
            },
            TreeNodeKind::Unsupported { code } => TreeTokenRole::Unresolved { code: code.clone() },
        };
        tokens.push(TreeTokenRow {
            token: node.key.clone(),
            role,
        });
        if matches!(node.kind, TreeNodeKind::Attribute { .. }) {
            attributes.push(TreeAttributeRule {
                node: node_ids[node.key.as_str()].clone(),
                slot: choice_slots[node.key.as_str()].clone(),
                lanes: option_ids
                    .iter()
                    .map(|(name, id)| TreeAttributeLane {
                        attribute: (*name).to_owned(),
                        option: id.clone(),
                    })
                    .collect(),
            });
        }
    }
    budget.rows(definitions.len())?;
    budget.rows(slots.len())?;
    budget.rows(allocator.new_mappings.len())?;
    let mut schema_input = checked_base.schema().input().clone();
    schema_input.definitions = definitions.into_values().collect();
    schema_input.slots = slots.into_values().collect();
    let schema = OwnedDefinitionSchemaPackage::new(schema_input, limits.recipe.schema)?;
    let mut rules = checked_base.rules().input().clone();
    let existing: BTreeSet<_> = rules
        .owners
        .iter()
        .filter_map(|o| match &o.owner {
            SchemaSubject::Definition(a) => Some(a.clone()),
            SchemaSubject::Slot(_) => None,
        })
        .collect();
    for owner in new_owners {
        let SchemaSubject::Definition(address) = &owner else {
            unreachable!()
        };
        if !existing.contains(address) {
            rules.owners.push(DefinitionRules {
                programs: DeclaredSet::partial(
                    vec![],
                    vec![gap(
                        &owner,
                        SchemaFacet::GameRules,
                        "tree-game-rules-not-converted",
                    )],
                ),
                owner,
            });
        }
    }
    rules.definitions = schema.identity().clone();
    let mut routing = checked_base.routing().input().clone();
    routing.definitions = schema.identity().clone();
    allocator.registry.validate_limits(limits.mapping)?;
    base.registry().validate_successor(&allocator.registry)?;
    // Validate the full mapping result, while returning only the append rows.
    let mut final_mapping = mapping.input().clone();
    final_mapping.source = source;
    final_mapping.registry = allocator.registry.identity()?;
    final_mapping.definitions = schema.identity().clone();
    final_mapping
        .entries
        .extend(allocator.new_mappings.iter().cloned());
    OwnedMappingIndex::new(final_mapping, &allocator.registry, &schema, limits.mapping)?;
    let successor = OwnedRecipeInput {
        schema_version: OWNED_RECIPE_VERSION,
        registry: allocator.registry.input().clone(),
        schema: schema.input().clone(),
        rules,
        routing,
    };
    assemble_owned_recipe(successor.clone(), limits.recipe)?;
    let content = TreeNormalizationContent {
        version: policy.version.clone(),
        source: input.source.clone(),
        catalog: catalog_digest,
        policy: policy_digest,
        tree_version: input.tree_version.clone(),
        classes: classes
            .values()
            .map(|c| TreeClassRow {
                key: c.key.clone(),
                class: class_ids[c.key.as_str()].clone(),
            })
            .collect(),
        ascendancies: classes
            .values()
            .flat_map(|c| {
                c.ascendancies.iter().map(|a| TreeAscendancyRow {
                    class_key: c.key.clone(),
                    key: a.key.clone(),
                    ordinal: a.ordinal,
                    ascendancy: ascendancy_ids[&(c.key.as_str(), a.key.as_str())].clone(),
                })
            })
            .collect(),
        tokens,
        attributes,
        syntax: policy.syntax.clone(),
    };
    let receipt = TreeCatalogReceipt {
        catalog: catalog_digest,
        policy: policy_digest,
        before_registry: base.registry().identity()?,
        after_registry: allocator.registry.identity()?,
        before_definitions: base.schema().identity().clone(),
        after_definitions: schema.identity().clone(),
        counts: allocator.counts,
        work_used: budget.work,
    };
    digest_owned(
        "owned-tree-catalog-result-v1",
        &(&successor, &allocator.new_mappings, &content, &receipt),
        limits.max_output_bytes,
    )?;
    Ok(StagedTreeCatalog {
        successor,
        new_mappings: allocator.new_mappings,
        content,
        receipt,
    })
}
