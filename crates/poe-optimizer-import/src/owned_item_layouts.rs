//! Finite source-layout conversion, isolated from owned gameplay definitions.
//! Supplied catalogs are data; only the optional reference exporter authenticates
//! source construction. No item category or weapon presence implies prefix absence.
use crate::{
    owned_item_lines::{ItemEmission, ItemLineError, ItemPatternPart, OwnedItemLinePolicy},
    owned_item_source::{
        ItemLoadIndexPrefix, ItemRuleSourceRole, ItemSourceDialect, ItemSourceError,
        ItemSourceLayoutPolicy, ItemSourceLayoutPolicyInput, ItemSourceLimits,
    },
    owned_mapping::{ExternalSourceSystem, SourcePin},
};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::ParameterValue,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::{GameVersionNamespace, ItemTemplateDefId, OwnedDefinitionKey},
    owned_schema::{DefinitionSchemaIndex, SchemaLookup},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{cmp::Ordering, io};

pub const OWNED_ITEM_LAYOUT_VERSION: u32 = 1;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemBaseLayoutCatalog {
    pub schema_version: u32,
    pub source: SourcePin,
    pub bases: Vec<ItemBaseLayoutRow>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemBaseLayoutRow {
    pub source_base: String,
    pub prefix: ItemBaseGeneratedPrefix,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemBaseGeneratedPrefix {
    Absent,
    Present,
    Unsupported,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemLayoutPolicy {
    pub schema_version: u32,
    pub version: OwnedDefinitionKey,
    pub catalog_sha256: String,
    pub definitions: DataIdentity,
    pub items: OwnedContentDigest,
    pub item_source: OwnedContentDigest,
    pub templates: Vec<ItemLayoutTemplateBinding>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemLayoutTemplateBinding {
    pub source_base: String,
    pub template: ItemTemplateDefId,
    pub header_rule: OwnedDefinitionKey,
}
#[derive(Clone, Copy, Debug)]
pub struct ItemLayoutLimits {
    pub max_catalog_bytes: usize,
    pub max_policy_bytes: usize,
    pub max_bases: usize,
    pub max_work: usize,
    pub source: ItemSourceLimits,
}
impl Default for ItemLayoutLimits {
    fn default() -> Self {
        Self {
            max_catalog_bytes: 4 * 1024 * 1024,
            max_policy_bytes: 4 * 1024 * 1024,
            max_bases: 8192,
            max_work: 16 * 1024 * 1024,
            source: Default::default(),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ItemLayoutError {
    #[error("item source-layout conversion exceeds or has invalid limit: {0}")]
    Limit(&'static str),
    #[error("invalid item source-layout catalog or policy: {0}")]
    Invalid(&'static str),
    #[error("item source-layout artifact or prior binding differs")]
    Binding,
    #[error("item source-layout evidence contradicts an existing prefix declaration")]
    Preservation,
    #[error(transparent)]
    Items(#[from] ItemLineError),
    #[error(transparent)]
    Source(#[from] ItemSourceError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ItemLayoutReceipt {
    pub catalog_sha256: String,
    pub policy: OwnedContentDigest,
    pub before: OwnedContentDigest,
    pub after: OwnedContentDigest,
    pub bases: usize,
    pub absent: usize,
    pub present: usize,
    pub unsupported: usize,
    pub refined_prefixes: usize,
    pub work_used: usize,
}
#[derive(Clone, Debug)]
pub struct CompiledItemLayouts {
    pub item_source: ItemSourceLayoutPolicyInput,
    pub receipt: ItemLayoutReceipt,
}
type Result<T> = std::result::Result<T, ItemLayoutError>;
fn charge(left: &mut usize, amount: usize) -> Result<()> {
    *left = left
        .checked_sub(amount)
        .ok_or(ItemLayoutError::Limit("work"))?;
    Ok(())
}
fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.trim_ascii() == value && !value.chars().any(char::is_control)
}

// Index keys stay borrowed. Every comparison is charged before inspecting bytes,
// including namespace components when typed IDs share their definition key.
trait IndexKey: Copy + Ord {
    fn comparison_work(self, other: Self) -> usize;
}
impl IndexKey for &str {
    fn comparison_work(self, other: Self) -> usize {
        self.len().min(other.len()).saturating_add(1)
    }
}
impl IndexKey for &ItemTemplateDefId {
    fn comparison_work(self, other: Self) -> usize {
        self.namespace()
            .game()
            .as_str()
            .comparison_work(other.namespace().game().as_str())
            .saturating_add(
                self.namespace()
                    .version()
                    .as_str()
                    .comparison_work(other.namespace().version().as_str()),
            )
            .saturating_add(self.key().as_str().comparison_work(other.key().as_str()))
    }
}
struct SortedIndex<K, V> {
    entries: Vec<(K, V)>,
}
impl<K: IndexKey, V: Copy> SortedIndex<K, V> {
    fn new(
        values: impl IntoIterator<Item = (K, V)>,
        work: &mut usize,
        duplicate: fn() -> ItemLayoutError,
    ) -> Result<Self> {
        let mut entries = Vec::new();
        for value in values {
            charge(work, 1)?;
            entries.push(value);
        }
        let mut width = 1usize;
        while width < entries.len() {
            // Prepay each copied entry before allocating this merge-pass buffer.
            charge(work, entries.len())?;
            let mut next = Vec::with_capacity(entries.len());
            for start in (0..entries.len()).step_by(width.saturating_mul(2)) {
                let middle = start.saturating_add(width).min(entries.len());
                let end = middle.saturating_add(width).min(entries.len());
                let (mut left, mut right) = (start, middle);
                while left < middle && right < end {
                    let a = entries[left].0;
                    let b = entries[right].0;
                    charge(work, a.comparison_work(b))?;
                    match a.cmp(&b) {
                        Ordering::Less => {
                            next.push(entries[left]);
                            left += 1;
                        }
                        Ordering::Greater => {
                            next.push(entries[right]);
                            right += 1;
                        }
                        Ordering::Equal => return Err(duplicate()),
                    }
                }
                next.extend_from_slice(&entries[left..middle]);
                next.extend_from_slice(&entries[right..end]);
            }
            entries = next;
            width = width.saturating_mul(2);
        }
        Ok(Self { entries })
    }
    fn get(&self, key: K, work: &mut usize) -> Result<Option<V>> {
        let (mut start, mut end) = (0, self.entries.len());
        while start < end {
            let middle = start + (end - start) / 2;
            let (candidate, value) = self.entries[middle];
            charge(work, candidate.comparison_work(key))?;
            match candidate.cmp(&key) {
                Ordering::Less => start = middle + 1,
                Ordering::Greater => end = middle,
                Ordering::Equal => return Ok(Some(value)),
            }
        }
        Ok(None)
    }
}

// Count JSON without buffering it, paying for this serialization and all named
// subsequent byte passes before their hash/clone work. Callers also prepay raw
// policy strings so JSON's escape scan cannot precede the work reservation.
struct WorkCounter<'a> {
    work: &'a mut usize,
    maximum: usize,
    written: usize,
    passes: usize,
    exceeded_bytes: bool,
    exceeded_work: bool,
}
impl io::Write for WorkCounter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.maximum.saturating_sub(self.written) {
            self.exceeded_bytes = true;
            return Err(io::Error::other("item layout serialization byte limit"));
        }
        if charge(self.work, bytes.len().saturating_mul(self.passes)).is_err() {
            self.exceeded_work = true;
            return Err(io::Error::other("item layout serialization work limit"));
        }
        self.written += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn reserve_serialized<T: Serialize>(
    value: &T,
    maximum: usize,
    passes: usize,
    work: &mut usize,
) -> Result<()> {
    let mut writer = WorkCounter {
        work,
        maximum,
        written: 0,
        passes,
        exceeded_bytes: false,
        exceeded_work: false,
    };
    let result = serde_json::to_writer(&mut writer, value);
    if writer.exceeded_work {
        return Err(ItemLayoutError::Limit("work"));
    }
    if writer.exceeded_bytes {
        return Err(ContentDigestError::TooLarge { maximum }.into());
    }
    result?;
    Ok(())
}
fn reserve_policy_strings(policy: &ItemLayoutPolicy, work: &mut usize) -> Result<()> {
    for value in [
        policy.version.as_str(),
        &policy.catalog_sha256,
        &policy.definitions.game,
        &policy.definitions.release,
        &policy.definitions.content_sha256,
        &policy.definitions.semantics_version,
    ] {
        charge(work, value.len().saturating_add(1))?;
    }
    for binding in &policy.templates {
        for value in [
            binding.source_base.as_str(),
            binding.header_rule.as_str(),
            binding.template.namespace().game().as_str(),
            binding.template.namespace().version().as_str(),
            binding.template.key().as_str(),
        ] {
            charge(work, value.len().saturating_add(1))?;
        }
    }
    Ok(())
}

fn reserve_definition_strings(
    namespace: &GameVersionNamespace,
    key: &OwnedDefinitionKey,
    work: &mut usize,
) -> Result<()> {
    for value in [
        namespace.game().as_str(),
        namespace.version().as_str(),
        key.as_str(),
    ] {
        charge(work, value.len().saturating_add(1))?;
    }
    Ok(())
}
// The prior policy can contain large unrelated provenance paths, property labels
// or default headers. Its own byte cap does not authorize scanning those strings
// against a smaller remaining compiler budget. Reserve all raw string traversal
// before the counting serializer may inspect them for JSON escapes.
fn reserve_source_strings(source: &ItemSourceLayoutPolicyInput, work: &mut usize) -> Result<()> {
    reserve_definition_strings(&source.namespace, &source.version, work)?;
    charge(work, source.source.revision.len().saturating_add(1))?;
    for pin in &source.source.files {
        charge(
            work,
            pin.path
                .len()
                .saturating_add(pin.sha256.len())
                .saturating_add(2),
        )?;
    }
    for binding in &source.property_bindings {
        charge(
            work,
            binding
                .label
                .len()
                .saturating_add(binding.property.as_str().len())
                .saturating_add(2),
        )?;
    }
    if let ItemSourceDialect::PobExportedSingleTextFlagsV1 { flag_bindings } = &source.dialect {
        for binding in flag_bindings {
            charge(
                work,
                binding
                    .label
                    .label()
                    .len()
                    .saturating_add(binding.property.as_str().len())
                    .saturating_add(2),
            )?;
        }
    }
    for rule in &source.rule_layouts {
        charge(work, rule.rule.as_str().len().saturating_add(1))?;
    }
    for layout in &source.template_layouts {
        reserve_definition_strings(layout.template.namespace(), layout.template.key(), work)?;
    }
    for defaults in &source.template_defaults {
        reserve_definition_strings(defaults.template.namespace(), defaults.template.key(), work)?;
        for parameter in &defaults.parameters {
            let slot = &parameter.assignment.slot;
            reserve_definition_strings(slot.declaration.namespace(), slot.declaration.key(), work)?;
            reserve_definition_strings(slot.slot.namespace(), slot.slot.key(), work)?;
            match &parameter.assignment.value {
                ParameterValue::Quantity(value) => {
                    reserve_definition_strings(value.unit().namespace(), value.unit().key(), work)?
                }
                ParameterValue::Option(value) => {
                    reserve_definition_strings(value.namespace(), value.key(), work)?
                }
                ParameterValue::Boolean(_) | ParameterValue::Integer(_) => {}
            }
            for header in &parameter.headers {
                charge(work, header.len().saturating_add(1))?;
            }
        }
    }
    Ok(())
}

/// Refine only generated-prefix absence from explicitly paired source evidence.
/// Every template binding names its exact existing literal header and source role.
/// Unknown/present prefix evidence stays unresolved; it never becomes an empty list.
pub fn compile_owned_item_layouts<I: DefinitionSchemaIndex>(
    lines: &OwnedItemLinePolicy,
    prior: &ItemSourceLayoutPolicy,
    schema: &I,
    catalog_bytes: &[u8],
    policy: &ItemLayoutPolicy,
    limits: ItemLayoutLimits,
) -> Result<CompiledItemLayouts> {
    let hard = ItemLayoutLimits::default();
    for (name, actual, maximum) in [
        (
            "catalog bytes",
            limits.max_catalog_bytes,
            hard.max_catalog_bytes,
        ),
        (
            "policy bytes",
            limits.max_policy_bytes,
            hard.max_policy_bytes,
        ),
        ("bases", limits.max_bases, hard.max_bases),
        ("work", limits.max_work, hard.max_work),
    ] {
        if actual == 0 || actual > maximum {
            return Err(ItemLayoutError::Limit(name));
        }
    }
    if catalog_bytes.len() > limits.max_catalog_bytes || policy.templates.len() > limits.max_bases {
        return Err(ItemLayoutError::Limit("input"));
    }
    let mut work = limits.max_work;
    // Hashing and JSON decoding are distinct linear passes; neither may run
    // before a caller's tighter work budget has admitted the complete input.
    charge(&mut work, catalog_bytes.len().saturating_mul(2))?;
    reserve_policy_strings(policy, &mut work)?;
    prior.verify_bindings(lines, schema)?;
    if policy.schema_version != OWNED_ITEM_LAYOUT_VERSION {
        return Err(ItemLayoutError::Invalid("policy version"));
    }
    let catalog_sha256 = format!("{:x}", Sha256::digest(catalog_bytes));
    if policy.catalog_sha256 != catalog_sha256
        || policy.definitions != *schema.identity()
        || policy.items != *lines.identity()
        || policy.item_source != *prior.identity()
    {
        return Err(ItemLayoutError::Binding);
    }
    // One bounded counting serialization plus the later serialization/hash.
    reserve_serialized(policy, limits.max_policy_bytes, 3, &mut work)?;
    let policy_identity = digest_owned(
        "owned-item-layout-policy-v1",
        policy,
        limits.max_policy_bytes,
    )?;
    let catalog: ItemBaseLayoutCatalog = serde_json::from_slice(catalog_bytes)?;
    if catalog.schema_version != OWNED_ITEM_LAYOUT_VERSION
        || catalog.bases.is_empty()
        || catalog.bases.len() != policy.templates.len()
    {
        return Err(ItemLayoutError::Invalid("catalog version or membership"));
    }
    if catalog.bases.len() > limits.max_bases {
        return Err(ItemLayoutError::Limit("bases"));
    }
    charge(
        &mut work,
        catalog
            .source
            .revision
            .len()
            .min(prior.input().source.revision.len())
            .saturating_add(1),
    )?;
    if catalog.source.system != ExternalSourceSystem::PathOfBuilding2
        || catalog.source.system != prior.input().source.system
        || catalog.source.revision != prior.input().source.revision
        || catalog.source.files.is_empty()
        || catalog.source.files.len() > prior.input().source.files.len()
    {
        return Err(ItemLayoutError::Binding);
    }
    let existing_pins = SortedIndex::new(
        prior
            .input()
            .source
            .files
            .iter()
            .map(|p| (p.path.as_str(), p.sha256.as_str())),
        &mut work,
        || ItemLayoutError::Binding,
    )?;
    let pins = SortedIndex::new(
        catalog
            .source
            .files
            .iter()
            .map(|p| (p.path.as_str(), p.sha256.as_str())),
        &mut work,
        || ItemLayoutError::Binding,
    )?;
    for &(path, hash) in &pins.entries {
        let old = existing_pins
            .get(path, &mut work)?
            .ok_or(ItemLayoutError::Binding)?;
        charge(&mut work, old.len().min(hash.len()).saturating_add(1))?;
        // The existing validated policy checks syntax; every projection pin must
        // be present unchanged in that source scope, including its exact hash.
        if old != hash {
            return Err(ItemLayoutError::Binding);
        }
    }
    for row in &catalog.bases {
        charge(
            &mut work,
            row.source_base.len().saturating_mul(2).saturating_add(1),
        )?;
        if !valid_text(&row.source_base) {
            return Err(ItemLayoutError::Invalid("duplicate or invalid source base"));
        }
    }
    let rows = SortedIndex::new(
        catalog
            .bases
            .iter()
            .map(|r| (r.source_base.as_str(), r.prefix)),
        &mut work,
        || ItemLayoutError::Invalid("duplicate or invalid source base"),
    )?;
    let rules = SortedIndex::new(
        lines.input().rules.iter().map(|r| (r.id.as_str(), r)),
        &mut work,
        || ItemLayoutError::Binding,
    )?;
    let roles = SortedIndex::new(
        prior
            .input()
            .rule_layouts
            .iter()
            .map(|r| (r.rule.as_str(), r.role)),
        &mut work,
        || ItemLayoutError::Binding,
    )?;
    let layouts = SortedIndex::new(
        prior
            .input()
            .template_layouts
            .iter()
            .enumerate()
            .map(|(i, t)| (&t.template, i)),
        &mut work,
        || ItemLayoutError::Binding,
    )?;
    // The two independently unique projections plus equal catalog cardinality
    // establish a bijection; they never collapse two source names onto one base.
    SortedIndex::new(
        policy
            .templates
            .iter()
            .map(|b| (b.source_base.as_str(), ())),
        &mut work,
        || ItemLayoutError::Invalid("duplicate, foreign or unresolved template binding"),
    )?;
    SortedIndex::new(
        policy.templates.iter().map(|b| (&b.template, ())),
        &mut work,
        || ItemLayoutError::Invalid("duplicate, foreign or unresolved template binding"),
    )?;
    let mut edits = Vec::new();
    let (mut absent, mut present, mut unsupported) = (0, 0, 0);
    for binding in &policy.templates {
        charge(
            &mut work,
            binding
                .source_base
                .len()
                .saturating_add(binding.header_rule.as_str().len())
                .saturating_add(1),
        )?;
        charge(
            &mut work,
            binding
                .template
                .namespace()
                .game()
                .as_str()
                .len()
                .saturating_add(binding.template.namespace().version().as_str().len())
                .saturating_add(2),
        )?;
        if binding.template.namespace() != schema.namespace()
            || !matches!(schema.definition(&binding.template), SchemaLookup::Known(_))
        {
            return Err(ItemLayoutError::Invalid(
                "duplicate, foreign or unresolved template binding",
            ));
        }
        let prefix = rows
            .get(binding.source_base.as_str(), &mut work)?
            .ok_or(ItemLayoutError::Binding)?;
        let rule = rules
            .get(binding.header_rule.as_str(), &mut work)?
            .ok_or(ItemLayoutError::Binding)?;
        charge(
            &mut work,
            binding
                .source_base
                .len()
                .saturating_add(binding.template.comparison_work(&binding.template)),
        )?;
        if !rule.captures.is_empty()
            || !matches!(rule.pattern.as_slice(),[ItemPatternPart::Literal(name)] if name==&binding.source_base)
            || !matches!(rule.emissions.as_slice(),[ItemEmission::Template{definition}] if definition==&binding.template)
            || roles.get(binding.header_rule.as_str(), &mut work)?
                != Some(ItemRuleSourceRole::Header)
        {
            return Err(ItemLayoutError::Invalid(
                "binding needs the exact admitted template header",
            ));
        }
        let index = layouts
            .get(&binding.template, &mut work)?
            .ok_or(ItemLayoutError::Binding)?;
        let old = prior.input().template_layouts[index].load_index_prefix;
        match prefix {
            ItemBaseGeneratedPrefix::Absent => {
                absent += 1;
                if old == ItemLoadIndexPrefix::Unresolved {
                    charge(&mut work, 1)?;
                    edits.push(index);
                }
            }
            ItemBaseGeneratedPrefix::Present => {
                present += 1;
                if old != ItemLoadIndexPrefix::Unresolved {
                    return Err(ItemLayoutError::Preservation);
                }
            }
            ItemBaseGeneratedPrefix::Unsupported => {
                unsupported += 1;
                if old != ItemLoadIndexPrefix::Unresolved {
                    return Err(ItemLayoutError::Preservation);
                }
            }
        }
    }
    // Reserve the counter pass and both owned source-input clones. The source
    // constructor separately enforces its own configured structural/work limits.
    reserve_source_strings(prior.input(), &mut work)?;
    reserve_serialized(prior.input(), limits.source.max_wire_bytes, 3, &mut work)?;
    charge(
        &mut work,
        policy
            .version
            .as_str()
            .len()
            .saturating_mul(2)
            .saturating_add(edits.len()),
    )?;
    let mut output = prior.input().clone();
    output.version = policy.version.clone();
    for &index in &edits {
        output.template_layouts[index].load_index_prefix =
            ItemLoadIndexPrefix::NoGeneratedBuffMembers;
    }
    let checked = ItemSourceLayoutPolicy::new(output, lines, schema, limits.source)?;
    Ok(CompiledItemLayouts {
        item_source: checked.input().clone(),
        receipt: ItemLayoutReceipt {
            catalog_sha256,
            policy: policy_identity,
            before: *prior.identity(),
            after: *checked.identity(),
            bases: catalog.bases.len(),
            absent,
            present,
            unsupported,
            refined_prefixes: edits.len(),
            work_used: limits.max_work - work,
        },
    })
}
