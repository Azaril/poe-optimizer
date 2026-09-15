//! Finite, injected tree-token interpretation. This is import evidence, not legality.
//! Source pins describe reviewed provenance; no source checkout is loaded here.
use crate::{
    owned_mapping::*,
    owned_normalize::{
        NormalizationError, NormalizationLimits, NormalizationPolicy, validate_normalization_inputs,
    },
};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::DeclaredSlot,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_schema::*,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const OWNED_TREE_POLICY_VERSION: u32 = 1;
const DOMAIN: &str = "owned-tree-normalization-policy-v1";
fn required_option<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> std::result::Result<Option<T>, D::Error> {
    Option::deserialize(d)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeClassRow {
    pub key: String,
    pub class: ClassDefId,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeAscendancyRow {
    pub class_key: String,
    pub key: String,
    pub ordinal: u16,
    pub ascendancy: AscendancyDefId,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TreeTokenRole {
    Allocation {
        node: PassiveNodeDefId,
        pool: PointPoolDefId,
    },
    ImplicitRoot {
        node: PassiveNodeDefId,
    },
    AttachedChoice {
        parent: PassiveNodeDefId,
        slot: DeclaredSlot<ChoiceSlotDefId>,
        option: OptionDefId,
    },
    Unresolved {
        code: OwnedDefinitionKey,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeTokenRow {
    pub token: String,
    pub role: TreeTokenRole,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeAttributeLane {
    pub attribute: String,
    pub option: OptionDefId,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeAttributeRule {
    pub node: PassiveNodeDefId,
    pub slot: DeclaredSlot<ChoiceSlotDefId>,
    pub lanes: Vec<TreeAttributeLane>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeWeaponOverlay {
    pub element: String,
    pub nodes_attribute: String,
    pub loadout: OwnedDefinitionKey,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeNormalizationSyntax {
    pub tree_version_attribute: String,
    pub class_attribute: String,
    pub ascendancy_attribute: String,
    #[serde(deserialize_with = "required_option")]
    pub class_consistency_attribute: Option<String>,
    #[serde(deserialize_with = "required_option")]
    pub ascendancy_consistency_attribute: Option<String>,
    pub overrides_element: String,
    pub attribute_override_element: String,
    pub weapon_overlays: Vec<TreeWeaponOverlay>,
    /// Excluded only from allocation-token interpretation. Other branch obligations remain.
    pub ignored_spec_children: Vec<String>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeNormalizationContent {
    pub version: OwnedDefinitionKey,
    pub source: SourcePin,
    /// Historical compiler provenance, not attestation of exporter/runtime behavior.
    pub catalog: OwnedContentDigest,
    pub policy: OwnedContentDigest,
    pub tree_version: String,
    pub classes: Vec<TreeClassRow>,
    pub ascendancies: Vec<TreeAscendancyRow>,
    pub tokens: Vec<TreeTokenRow>,
    pub attributes: Vec<TreeAttributeRule>,
    pub syntax: TreeNormalizationSyntax,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeNormalizationPackageInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub registry: OwnedContentDigest,
    pub definitions: DataIdentity,
    pub mapping: OwnedContentDigest,
    pub normalization: OwnedContentDigest,
    pub content: TreeNormalizationContent,
}
#[derive(Clone, Copy, Debug)]
pub struct TreePolicyLimits {
    pub mapping: OwnedMappingLimits,
    pub max_entries: usize,
    pub max_collection_entries: usize,
    pub max_string_bytes: usize,
    pub max_schema_work: usize,
    pub max_wire_bytes: usize,
    pub max_base_policy_bytes: usize,
}
impl Default for TreePolicyLimits {
    fn default() -> Self {
        Self {
            mapping: OwnedMappingLimits::default(),
            max_entries: 1_000_000,
            max_collection_entries: 100_000,
            max_string_bytes: 4096,
            max_schema_work: 2_000_000,
            max_wire_bytes: 16 * 1024 * 1024,
            max_base_policy_bytes: 1024 * 1024,
        }
    }
}
impl TreePolicyLimits {
    pub(crate) fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (name, value, max) in [
            ("entries", self.max_entries, hard.max_entries),
            (
                "collection entries",
                self.max_collection_entries,
                hard.max_collection_entries,
            ),
            ("string bytes", self.max_string_bytes, hard.max_string_bytes),
            ("schema work", self.max_schema_work, hard.max_schema_work),
            ("wire bytes", self.max_wire_bytes, hard.max_wire_bytes),
            (
                "base policy bytes",
                self.max_base_policy_bytes,
                hard.max_base_policy_bytes,
            ),
        ] {
            if value == 0 || value > max {
                return Err(TreePolicyError::InvalidLimit(name));
            }
        }
        self.mapping.validate()?;
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
pub enum TreePolicyError {
    #[error("unsupported tree normalization policy version {0}")]
    Version(u32),
    #[error("invalid tree policy limit: {0}")]
    InvalidLimit(&'static str),
    #[error("tree policy exceeds {0}")]
    Limit(&'static str),
    #[error("tree policy artifact bindings disagree")]
    Binding,
    #[error("invalid tree policy: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Mapping(#[from] OwnedMappingError),
    #[error(transparent)]
    Normalization(Box<NormalizationError>),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
impl From<NormalizationError> for TreePolicyError {
    fn from(error: NormalizationError) -> Self {
        Self::Normalization(Box::new(error))
    }
}
type Result<T> = std::result::Result<T, TreePolicyError>;
struct Budget {
    limits: TreePolicyLimits,
    left: usize,
    work: usize,
}
impl Budget {
    fn collection(&mut self, n: usize) -> Result<()> {
        if n > self.limits.max_collection_entries {
            return Err(TreePolicyError::Limit("collection entries"));
        }
        self.left = self
            .left
            .checked_sub(n)
            .ok_or(TreePolicyError::Limit("entries"))?;
        Ok(())
    }
    fn work(&mut self, n: usize) -> Result<()> {
        self.work = self
            .work
            .checked_sub(n)
            .ok_or(TreePolicyError::Limit("schema work"))?;
        Ok(())
    }
    fn text(&self, s: &str) -> Result<()> {
        if s.is_empty() || s.len() > self.limits.max_string_bytes || s.chars().any(char::is_control)
        {
            return Err(TreePolicyError::Invalid("source text"));
        }
        Ok(())
    }
    fn name(&self, s: &str) -> Result<()> {
        self.text(s)?;
        if !s.bytes().enumerate().all(|(i, c)| {
            c.is_ascii_alphabetic()
                || c == b'_'
                || (i > 0 && (c.is_ascii_digit() || c == b'-' || c == b'.'))
        }) {
            return Err(TreePolicyError::Invalid("source name"));
        }
        Ok(())
    }
    fn contains<T: PartialEq>(&mut self, values: &[T], value: &T) -> Result<bool> {
        self.work(values.len())?;
        Ok(values.contains(value))
    }
}
fn active(registry: &OwnedIdRegistry, subject: SchemaSubject) -> Result<()> {
    if !matches!(
        registry.entry(&subject).map(|r| &r.state),
        Some(RegistryState::Active)
    ) {
        return Err(TreePolicyError::Invalid("inactive or missing target"));
    }
    Ok(())
}
fn known<'a, I: DefinitionSchemaIndex, D: SchemaDefinitionId>(
    registry: &OwnedIdRegistry,
    index: &'a I,
    id: &D,
) -> Result<&'a D::Descriptor> {
    active(registry, SchemaSubject::Definition(id.address()))?;
    match index.definition(id) {
        SchemaLookup::Known(s) => Ok(s),
        _ => Err(TreePolicyError::Invalid("unknown target schema")),
    }
}
fn mapped(
    mapping: &OwnedMappingIndex,
    selector: ExternalSelector,
    target: SchemaSubject,
) -> Result<()> {
    if !matches!(mapping.lookup(&selector),Some(MappingOutcome::Mapped{target:actual,..}) if *actual==target)
    {
        return Err(TreePolicyError::Invalid("source mapping disagreement"));
    }
    Ok(())
}
fn choice<I: DefinitionSchemaIndex>(
    registry: &OwnedIdRegistry,
    index: &I,
    b: &mut Budget,
    parent: &PassiveNodeDefId,
    slot: &DeclaredSlot<ChoiceSlotDefId>,
    option: &OptionDefId,
) -> Result<()> {
    if slot.declaration != SlotOwnerDefId::PassiveNode(parent.clone()) {
        return Err(TreePolicyError::Invalid("choice declaration owner"));
    }
    let p = known(registry, index, parent)?;
    if !b.contains(&p.declarations.choices.members, slot)? {
        return Err(TreePolicyError::Invalid("choice not declared"));
    }
    active(
        registry,
        SchemaSubject::Slot(ChoiceSlotDefId::address(slot)),
    )?;
    let SchemaLookup::Known(s) = index.slot(slot) else {
        return Err(TreePolicyError::Invalid("unknown choice schema"));
    };
    if !b.contains(&s.owners, &ChoiceOwnerScope::Allocation)?
        && !b.contains(
            &s.owners,
            &ChoiceOwnerScope::Provider(ProviderRole::Allocation),
        )?
    {
        return Err(TreePolicyError::Invalid("choice cannot target allocation"));
    }
    known(registry, index, option)?;
    let ValueSchema::Option { allowed } = &s.value else {
        return Err(TreePolicyError::Invalid("choice is not option-valued"));
    };
    if !b.contains(&allowed.members, option)? {
        return Err(TreePolicyError::Invalid("option not allowed"));
    }
    Ok(())
}
fn base_digest(policy: &NormalizationPolicy, limit: usize) -> Result<OwnedContentDigest> {
    Ok(digest_owned(
        "owned-normalization-policy-v3",
        policy,
        limit,
    )?)
}

#[derive(Clone, Debug)]
pub struct OwnedTreeNormalizationPolicy {
    input: TreeNormalizationPackageInput,
    identity: OwnedContentDigest,
    tokens: BTreeMap<String, usize>,
    attributes: BTreeMap<PassiveNodeDefId, usize>,
    limits: TreePolicyLimits,
    entries: usize,
    schema_work: usize,
    wire_bytes: usize,
}
impl OwnedTreeNormalizationPolicy {
    pub fn bind_new<I: DefinitionSchemaIndex>(
        content: TreeNormalizationContent,
        registry: &OwnedIdRegistry,
        schema: &I,
        mapping: &OwnedMappingIndex,
        base: &NormalizationPolicy,
        limits: TreePolicyLimits,
    ) -> Result<Self> {
        limits.validate()?;
        Self::new(
            TreeNormalizationPackageInput {
                schema_version: OWNED_TREE_POLICY_VERSION,
                namespace: schema.namespace().clone(),
                registry: registry.identity()?,
                definitions: schema.identity().clone(),
                mapping: *mapping.identity(),
                normalization: base_digest(base, limits.max_base_policy_bytes)?,
                content,
            },
            registry,
            schema,
            mapping,
            base,
            limits,
        )
    }
    pub fn new<I: DefinitionSchemaIndex>(
        mut input: TreeNormalizationPackageInput,
        registry: &OwnedIdRegistry,
        schema: &I,
        mapping: &OwnedMappingIndex,
        base: &NormalizationPolicy,
        limits: TreePolicyLimits,
    ) -> Result<Self> {
        limits.validate()?;
        if input.schema_version != OWNED_TREE_POLICY_VERSION {
            return Err(TreePolicyError::Version(input.schema_version));
        }
        digest_owned(DOMAIN, &input, limits.max_wire_bytes)?;
        registry.validate_limits(limits.mapping)?;
        mapping.verify_bindings(
            registry,
            schema,
            &mapping.input().source,
            &mapping.input().policy_version,
            limits.mapping,
        )?;
        validate_normalization_inputs(
            base,
            mapping,
            schema,
            &[],
            NormalizationLimits {
                mapping: limits.mapping,
                max_policy_bytes: limits.max_base_policy_bytes,
                ..Default::default()
            },
        )?;
        if input.namespace != *schema.namespace()
            || input.registry != registry.identity()?
            || input.definitions != *schema.identity()
            || input.mapping != *mapping.identity()
            || input.normalization != base_digest(base, limits.max_base_policy_bytes)?
        {
            return Err(TreePolicyError::Binding);
        }
        let c = &mut input.content;
        let mut b = Budget {
            limits,
            left: limits.max_entries,
            work: limits.max_schema_work,
        };
        for n in [
            c.source.files.len(),
            c.classes.len(),
            c.ascendancies.len(),
            c.tokens.len(),
            c.attributes.len(),
            c.syntax.weapon_overlays.len(),
            c.syntax.ignored_spec_children.len(),
        ] {
            b.collection(n)?;
        }
        for row in &c.attributes {
            b.collection(row.lanes.len())?;
        }
        b.text(&c.tree_version)?;
        b.text(&c.source.revision)?;
        if c.source.files.is_empty()
            || c.source.system != mapping.input().source.system
            || c.source.revision != mapping.input().source.revision
        {
            return Err(TreePolicyError::Binding);
        }
        c.source.files.sort_by(|a, b| a.path.cmp(&b.path));
        if c.source.files.windows(2).any(|r| r[0].path == r[1].path) {
            return Err(TreePolicyError::Invalid("duplicate source pin"));
        }
        for pin in &c.source.files {
            b.text(&pin.path)?;
            let files = &mapping.input().source.files;
            if !files
                .binary_search_by(|v| v.path.cmp(&pin.path))
                .ok()
                .is_some_and(|i| files[i].sha256 == pin.sha256)
            {
                return Err(TreePolicyError::Binding);
            }
        }
        let sy = &mut c.syntax;
        let mut names = BTreeSet::new();
        for name in [
            &sy.tree_version_attribute,
            &sy.class_attribute,
            &sy.ascendancy_attribute,
            &base.allocation_attribute,
        ]
        .into_iter()
        .chain(sy.class_consistency_attribute.iter())
        .chain(sy.ascendancy_consistency_attribute.iter())
        {
            b.name(name)?;
            if !names.insert(name) {
                return Err(TreePolicyError::Invalid("overlapping Spec attributes"));
            }
        }
        b.name(&sy.overrides_element)?;
        b.name(&sy.attribute_override_element)?;
        let mut elements = BTreeSet::from([sy.overrides_element.as_str()]);
        let mut loadouts = BTreeSet::new();
        let mut allowed_loadouts = BTreeSet::new();
        b.work(base.equipment_loadouts.len())?;
        for row in &base.equipment_loadouts {
            if let crate::owned_normalize::ImportEquipmentScope::Selected { loadouts } = &row.scope
            {
                b.work(loadouts.len())?;
                allowed_loadouts.extend(loadouts);
            }
        }
        for overlay in &sy.weapon_overlays {
            b.name(&overlay.element)?;
            b.name(&overlay.nodes_attribute)?;
            if !elements.insert(&overlay.element)
                || !loadouts.insert(&overlay.loadout)
                || !allowed_loadouts.contains(&overlay.loadout)
            {
                return Err(TreePolicyError::Invalid("weapon overlay relation"));
            }
        }
        for name in &sy.ignored_spec_children {
            b.name(name)?;
            if !elements.insert(name) {
                return Err(TreePolicyError::Invalid("overlapping Spec child roles"));
            }
        }
        sy.weapon_overlays.sort_by(|a, b| a.element.cmp(&b.element));
        sy.ignored_spec_children.sort();
        c.classes.sort_by(|a, b| a.key.cmp(&b.key));
        c.ascendancies
            .sort_by(|a, b| (&a.class_key, &a.key).cmp(&(&b.class_key, &b.key)));
        let mut classes = BTreeMap::new();
        let mut roots = BTreeSet::new();
        for row in &c.classes {
            b.text(&row.key)?;
            if classes.insert(row.key.as_str(), &row.class).is_some() {
                return Err(TreePolicyError::Invalid("duplicate class key"));
            }
            let class = known(registry, schema, &row.class)?;
            b.work(class.implicit_passives.members.len())?;
            roots.extend(class.implicit_passives.members.iter());
            mapped(
                mapping,
                ExternalSelector::Definition(ExternalOwnerSelector::Class {
                    key: SourceComponent::Text(row.key.clone()),
                }),
                SchemaSubject::Definition(row.class.address()),
            )?;
        }
        let mut asckeys = BTreeSet::new();
        let mut ordinals = BTreeSet::new();
        for row in &c.ascendancies {
            b.text(&row.class_key)?;
            b.text(&row.key)?;
            if row.ordinal == 0
                || !asckeys.insert((&row.class_key, &row.key))
                || !ordinals.insert((&row.class_key, row.ordinal))
            {
                return Err(TreePolicyError::Invalid(
                    "duplicate or zero ascendancy key/ordinal",
                ));
            }
            let class = classes
                .get(row.class_key.as_str())
                .ok_or(TreePolicyError::Invalid("ascendancy class key"))?;
            let cs = known(registry, schema, *class)?;
            let asc = known(registry, schema, &row.ascendancy)?;
            if !b.contains(&cs.ascendancies.members, &row.ascendancy)?
                || !b.contains(&asc.classes.members, class)?
            {
                return Err(TreePolicyError::Invalid("class ascendancy membership"));
            }
            b.work(asc.implicit_passives.members.len())?;
            roots.extend(asc.implicit_passives.members.iter());
            mapped(
                mapping,
                ExternalSelector::Definition(ExternalOwnerSelector::Ascendancy {
                    class: SourceComponent::Text(row.class_key.clone()),
                    key: SourceComponent::Text(row.key.clone()),
                }),
                SchemaSubject::Definition(row.ascendancy.address()),
            )?;
        }
        c.tokens.sort_by(|a, b| a.token.cmp(&b.token));
        let mut tokens = BTreeMap::new();
        let mut physical = BTreeSet::new();
        let mut allocations = BTreeSet::new();
        for (i, row) in c.tokens.iter().enumerate() {
            b.text(&row.token)?;
            if tokens.insert(row.token.clone(), i).is_some() {
                return Err(TreePolicyError::Invalid("duplicate tree token"));
            }
            let node = match &row.role {
                TreeTokenRole::Allocation { node, pool } => {
                    let p = known(registry, schema, node)?;
                    known(registry, schema, pool)?;
                    if !b.contains(&p.pools.members, pool)? {
                        return Err(TreePolicyError::Invalid("allocation pool membership"));
                    }
                    allocations.insert(node);
                    Some(node)
                }
                TreeTokenRole::ImplicitRoot { node } => {
                    let p = known(registry, schema, node)?;
                    if !p.pools.is_complete()
                        || !p.pools.members.is_empty()
                        || !roots.contains(node)
                    {
                        return Err(TreePolicyError::Invalid("implicit root membership/pools"));
                    }
                    Some(node)
                }
                TreeTokenRole::AttachedChoice { .. } | TreeTokenRole::Unresolved { .. } => None,
            };
            if let Some(node) = node {
                if !physical.insert(node) {
                    return Err(TreePolicyError::Invalid("duplicate physical node role"));
                }
                mapped(
                    mapping,
                    ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode {
                        tree_version: SourceComponent::Text(c.tree_version.clone()),
                        node_id: SourceComponent::Text(row.token.clone()),
                        view: SourceComponent::Missing,
                    }),
                    SchemaSubject::Definition(node.address()),
                )?;
            }
        }
        for row in &c.tokens {
            if let TreeTokenRole::AttachedChoice {
                parent,
                slot,
                option,
            } = &row.role
            {
                if !allocations.contains(parent) {
                    return Err(TreePolicyError::Invalid(
                        "attached choice parent has no allocation role",
                    ));
                }
                choice(registry, schema, &mut b, parent, slot, option)?;
            }
        }
        c.attributes.sort_by(|a, b| a.node.cmp(&b.node));
        let mut attributes = BTreeMap::new();
        for (i, row) in c.attributes.iter_mut().enumerate() {
            if !allocations.contains(&row.node)
                || attributes.insert(row.node.clone(), i).is_some()
                || row.lanes.is_empty()
            {
                return Err(TreePolicyError::Invalid("attribute parent/lanes"));
            }
            let mut lanes = BTreeSet::new();
            let mut options = BTreeSet::new();
            for lane in &row.lanes {
                b.name(&lane.attribute)?;
                if !lanes.insert(&lane.attribute) || !options.insert(&lane.option) {
                    return Err(TreePolicyError::Invalid("duplicate attribute lane/option"));
                }
                choice(registry, schema, &mut b, &row.node, &row.slot, &lane.option)?;
            }
            row.lanes.sort_by(|a, b| a.attribute.cmp(&b.attribute));
        }
        let identity = digest_owned(DOMAIN, &input, limits.max_wire_bytes)?;
        let wire_bytes = serde_json::to_vec(&input)?.len();
        Ok(Self {
            input,
            identity,
            tokens,
            attributes,
            limits,
            entries: limits.max_entries - b.left,
            schema_work: limits.max_schema_work - b.work,
            wire_bytes,
        })
    }
    pub fn input(&self) -> &TreeNormalizationPackageInput {
        &self.input
    }
    pub fn identity(&self) -> &OwnedContentDigest {
        &self.identity
    }
    pub fn lookup(&self, tree_version: &str, token: &str) -> Option<&TreeTokenRole> {
        if tree_version != self.input.content.tree_version {
            return None;
        }
        self.tokens
            .get(token)
            .map(|i| &self.input.content.tokens[*i].role)
    }
    pub fn attribute(&self, node: &PassiveNodeDefId) -> Option<&TreeAttributeRule> {
        self.attributes
            .get(node)
            .map(|i| &self.input.content.attributes[*i])
    }
    pub fn verify_bindings<I: DefinitionSchemaIndex>(
        &self,
        registry: &OwnedIdRegistry,
        schema: &I,
        mapping: &OwnedMappingIndex,
        base: &NormalizationPolicy,
    ) -> Result<()> {
        mapping.verify_bindings(
            registry,
            schema,
            &mapping.input().source,
            &mapping.input().policy_version,
            self.limits.mapping,
        )?;
        if self.input.namespace != *schema.namespace()
            || self.input.registry != registry.identity()?
            || self.input.definitions != *schema.identity()
            || self.input.mapping != *mapping.identity()
            || self.input.normalization != base_digest(base, self.limits.max_base_policy_bytes)?
        {
            return Err(TreePolicyError::Binding);
        }
        Ok(())
    }
    pub fn validate_limits(&self, limits: TreePolicyLimits) -> Result<()> {
        limits.validate()?;
        if self.entries > limits.max_entries
            || self.schema_work > limits.max_schema_work
            || self.wire_bytes > limits.max_wire_bytes
        {
            return Err(TreePolicyError::Limit("stored policy resources"));
        }
        // Revalidate actual text/collection ceilings without rebuilding semantic indexes.
        let c = &self.input.content;
        for n in [
            c.classes.len(),
            c.ascendancies.len(),
            c.tokens.len(),
            c.attributes.len(),
            c.source.files.len(),
            c.syntax.weapon_overlays.len(),
            c.syntax.ignored_spec_children.len(),
        ] {
            if n > limits.max_collection_entries {
                return Err(TreePolicyError::Limit("collection entries"));
            }
        }
        for row in &c.attributes {
            if row.lanes.len() > limits.max_collection_entries {
                return Err(TreePolicyError::Limit("collection entries"));
            }
        }
        // Stricter strings/base/mapping budgets are applied at reconstruction or binding.
        if limits.max_string_bytes < self.limits.max_string_bytes
            || limits.max_base_policy_bytes < self.limits.max_base_policy_bytes
            || limits.mapping != self.limits.mapping
        {
            return Err(TreePolicyError::Limit(
                "reconstruction required for tighter dependency/text limits",
            ));
        }
        Ok(())
    }
}

pub fn decode_tree_policy<I: DefinitionSchemaIndex>(
    bytes: &[u8],
    registry: &OwnedIdRegistry,
    schema: &I,
    mapping: &OwnedMappingIndex,
    base: &NormalizationPolicy,
    limits: TreePolicyLimits,
) -> Result<OwnedTreeNormalizationPolicy> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(TreePolicyError::Limit("wire bytes"));
    }
    OwnedTreeNormalizationPolicy::new(
        serde_json::from_slice(bytes)?,
        registry,
        schema,
        mapping,
        base,
        limits,
    )
}
