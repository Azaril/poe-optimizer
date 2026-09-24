//! Offline compilation of reviewed display observations into finite template allowlists.
//!
//! Exact catalog bytes and predecessor policy identities are bound in the receipt.
//! Catalog validation and a supplied digest do not authenticate source extraction;
//! callers must authenticate the catalog separately before reviewing a request.
//! No base fields, display totals, defaults or numerical effects enter Core.
use crate::{
    owned_item_lines::*,
    owned_item_observations::*,
    owned_item_source::*,
    owned_mapping::{SourceFilePin, SourcePin},
};
use poe_optimizer_core::{
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::{ItemTemplateDefId, OwnedDefinitionKey},
    owned_schema::DefinitionSchemaIndex,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const OWNED_ITEM_OBSERVATION_REQUEST_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemObservationPolicyRequest {
    pub schema_version: u32,
    pub version: OwnedDefinitionKey,
    /// SHA-256 of the exact authenticated catalog bytes, including trailing whitespace.
    pub catalog_sha256: String,
    pub observations: Vec<ItemObservationDeclaration>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemObservationDeclaration {
    pub rule: ItemLineRule,
    /// Semantic identity shared by all reviewed aliases of one display field.
    pub field: OwnedDefinitionKey,
    pub family: ItemObservationFamily,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemObservationFamily {
    Defences,
    Spirit,
    CharmSlots,
}
impl ItemObservationFamily {
    fn permits(self, row: &ItemObservationBaseRow) -> bool {
        match self {
            Self::Defences => row.can_observe_defences(),
            Self::Spirit => row.can_observe_spirit(),
            Self::CharmSlots => row.can_observe_charm_slots(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ItemObservationPolicyLimits {
    pub max_request_bytes: usize,
    pub max_observations: usize,
    /// Additional compiler work; catalog and checked policies retain their own limits.
    pub max_work: usize,
    pub catalog: ItemObservationLimits,
    pub items: ItemLineLimits,
    pub item_source: ItemSourceLimits,
}
impl Default for ItemObservationPolicyLimits {
    fn default() -> Self {
        Self {
            max_request_bytes: 1024 * 1024,
            max_observations: 64,
            max_work: 64 * 1024 * 1024,
            catalog: ItemObservationLimits::default(),
            items: ItemLineLimits::default(),
            item_source: ItemSourceLimits::default(),
        }
    }
}
impl ItemObservationPolicyLimits {
    fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (label, value, maximum) in [
            (
                "request bytes",
                self.max_request_bytes,
                hard.max_request_bytes,
            ),
            ("observations", self.max_observations, hard.max_observations),
            ("work", self.max_work, hard.max_work),
        ] {
            if value == 0 || value > maximum {
                return Err(ItemObservationPolicyError::Limit(label));
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ItemObservationPolicyError {
    #[error("item observation compilation exceeds or has invalid limit: {0}")]
    Limit(&'static str),
    #[error("invalid item observation request or template binding: {0}")]
    Invalid(&'static str),
    #[error("item observation catalog, provenance or predecessor binding differs")]
    Binding,
    #[error(transparent)]
    Catalog(#[from] ItemObservationError),
    #[error(transparent)]
    Items(#[from] ItemLineError),
    #[error(transparent)]
    Source(#[from] ItemSourceError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, ItemObservationPolicyError>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemObservationPolicyReceipt {
    pub catalog_sha256: String,
    pub request: OwnedContentDigest,
    pub before_items: OwnedContentDigest,
    pub after_items: OwnedContentDigest,
    pub before_item_source: OwnedContentDigest,
    pub after_item_source: OwnedContentDigest,
    pub catalog_bases: usize,
    pub exact_base_bindings: usize,
    pub added_observations: usize,
    pub added_template_references: usize,
    pub work_used: usize,
}

#[derive(Clone, Debug)]
pub struct StagedItemObservationPolicy {
    pub items: ItemLinePolicyInput,
    pub item_source: ItemSourceLayoutPolicyInput,
    pub receipt: ItemObservationPolicyReceipt,
}

fn charge(work: &mut usize, amount: usize) -> Result<()> {
    *work = work
        .checked_sub(amount)
        .ok_or(ItemObservationPolicyError::Limit("work"))?;
    Ok(())
}
fn comparisons(entries: usize) -> usize {
    entries.checked_ilog2().map_or(1, |bits| bits as usize + 2)
}
fn merge_source(target: &mut SourcePin, incoming: &SourcePin, work: &mut usize) -> Result<()> {
    if target.system != incoming.system || target.revision != incoming.revision {
        return Err(ItemObservationPolicyError::Binding);
    }
    let mut files = BTreeMap::new();
    let count = target.files.len().saturating_add(incoming.files.len());
    for file in target.files.iter().chain(&incoming.files) {
        charge(
            work,
            file.path
                .len()
                .saturating_add(file.sha256.len())
                .saturating_mul(comparisons(count)),
        )?;
        if let Some(old) = files.insert(file.path.clone(), file.sha256.clone())
            && old != file.sha256
        {
            return Err(ItemObservationPolicyError::Binding);
        }
    }
    target.files = files
        .into_iter()
        .map(|(path, sha256)| SourceFilePin { path, sha256 })
        .collect();
    Ok(())
}

/// Stage source-only recipes and finite template applicability. Existing line
/// recipes, defaults, flags, metadata roles and member prerequisites are retained.
/// V6 upgrades to V7; V7 appends observations without replacing prior declarations.
/// Publication must still use the host's checked successor transition.
pub fn compile_owned_item_observations<I: DefinitionSchemaIndex>(
    catalog_bytes: &[u8],
    request: &ItemObservationPolicyRequest,
    lines: &OwnedItemLinePolicy,
    source: &ItemSourceLayoutPolicy,
    schema: &I,
    limits: ItemObservationPolicyLimits,
) -> Result<StagedItemObservationPolicy> {
    limits.validate()?;
    if request.schema_version != OWNED_ITEM_OBSERVATION_REQUEST_VERSION {
        return Err(ItemObservationPolicyError::Invalid("request version"));
    }
    if request.observations.is_empty() || request.observations.len() > limits.max_observations {
        return Err(ItemObservationPolicyError::Limit("observations"));
    }
    if catalog_bytes.len() > limits.catalog.max_catalog_bytes {
        return Err(ItemObservationPolicyError::Limit("catalog bytes"));
    }
    let mut work = limits.max_work;
    charge(&mut work, catalog_bytes.len())?;
    let request_identity = digest_owned(
        "owned-item-observation-request-v1",
        request,
        limits.max_request_bytes,
    )?;
    let catalog_sha256 = format!("{:x}", Sha256::digest(catalog_bytes));
    if catalog_sha256 != request.catalog_sha256 {
        return Err(ItemObservationPolicyError::Binding);
    }
    let catalog = decode_item_observation_catalog(catalog_bytes, limits.catalog)?;
    let prior_bytes = encode_item_line_policy(lines, limits.items)?;
    charge(
        &mut work,
        prior_bytes
            .len()
            .saturating_mul(comparisons(lines.input().rules.len())),
    )?;
    let prior_source_bytes = encode_item_source_policy(source, limits.item_source)?;
    charge(&mut work, prior_source_bytes.len())?;
    lines.verify_bindings(schema)?;
    let checked_source =
        ItemSourceLayoutPolicy::new(source.input().clone(), lines, schema, limits.item_source)?;
    if checked_source.identity() != source.identity() {
        return Err(ItemObservationPolicyError::Binding);
    }
    let mut item_source = source.input().clone();
    let (flags, metadata, conditions, prior_observations) = match &source.input().dialect {
        ItemSourceDialect::PobExportedSingleTextConditionsV1 {
            flag_bindings,
            metadata_rules,
            single_modifier_conditions,
        } => (
            flag_bindings.clone(),
            metadata_rules.clone(),
            single_modifier_conditions.clone(),
            vec![],
        ),
        ItemSourceDialect::PobExportedSingleTextObservationsV1 {
            flag_bindings,
            metadata_rules,
            single_modifier_conditions,
            preamble_observations,
        } => (
            flag_bindings.clone(),
            metadata_rules.clone(),
            single_modifier_conditions.clone(),
            preamble_observations.clone(),
        ),
        _ => {
            return Err(ItemObservationPolicyError::Invalid(
                "predecessor must use source dialect v6 or v7",
            ));
        }
    };
    merge_source(&mut item_source.source, &catalog.source, &mut work)?;

    let roles: BTreeMap<_, _> = source
        .input()
        .rule_layouts
        .iter()
        .map(|row| (&row.rule, row.role))
        .collect();
    let mut exact_rules: BTreeMap<&str, Vec<&ItemLineRule>> = BTreeMap::new();
    let mut template_uses: BTreeMap<&ItemTemplateDefId, usize> = BTreeMap::new();
    let mut ids: BTreeSet<_> = lines.input().rules.iter().map(|rule| &rule.id).collect();
    for rule in &lines.input().rules {
        if let [ItemPatternPart::Literal(text)] = rule.pattern.as_slice() {
            exact_rules.entry(text.as_str()).or_default().push(rule);
        }
        for emission in &rule.emissions {
            if let ItemEmission::Template { definition } = emission {
                *template_uses.entry(definition).or_default() += 1;
            }
        }
    }
    let mut compatible: BTreeMap<ItemObservationFamily, BTreeSet<ItemTemplateDefId>> =
        BTreeMap::new();
    let mut exact_base_bindings = 0usize;
    for row in &catalog.bases {
        charge(
            &mut work,
            row.source_base
                .len()
                .saturating_add(1)
                .saturating_mul(comparisons(exact_rules.len())),
        )?;
        let Some(matches) = exact_rules.get(row.source_base.as_str()) else {
            continue;
        };
        if matches.len() != 1 {
            return Err(ItemObservationPolicyError::Invalid(
                "ambiguous exact base rules",
            ));
        }
        let rule = matches[0];
        let [ItemEmission::Template { definition }] = rule.emissions.as_slice() else {
            return Err(ItemObservationPolicyError::Invalid(
                "base rule must emit exactly one template",
            ));
        };
        if roles.get(&rule.id) != Some(&ItemRuleSourceRole::Header)
            || !rule.captures.is_empty()
            || template_uses.get(definition) != Some(&1)
        {
            return Err(ItemObservationPolicyError::Invalid(
                "base template needs one exact header binding",
            ));
        }
        exact_base_bindings += 1;
        for family in [
            ItemObservationFamily::Defences,
            ItemObservationFamily::Spirit,
            ItemObservationFamily::CharmSlots,
        ] {
            charge(
                &mut work,
                definition
                    .key()
                    .as_str()
                    .len()
                    .saturating_add(1)
                    .saturating_mul(comparisons(catalog.bases.len())),
            )?;
            if family.permits(row) {
                compatible
                    .entry(family)
                    .or_default()
                    .insert(definition.clone());
            }
        }
    }
    let mut items = lines.input().clone();
    let mut observations = prior_observations;
    let mut added_template_references = 0usize;
    let mut fields = BTreeMap::new();
    for declaration in &request.observations {
        charge(
            &mut work,
            declaration
                .rule
                .id
                .as_str()
                .len()
                .saturating_add(declaration.field.as_str().len())
                .saturating_add(declaration.rule.emissions.len())
                .saturating_mul(comparisons(ids.len())),
        )?;
        if !ids.insert(&declaration.rule.id)
            || declaration.rule.emissions.is_empty()
            || !declaration
                .rule
                .emissions
                .iter()
                .all(|emission| matches!(emission, ItemEmission::Metadata { .. }))
        {
            return Err(ItemObservationPolicyError::Invalid(
                "observation needs a new nonempty metadata-only rule",
            ));
        }
        if fields
            .insert(&declaration.field, declaration.family)
            .is_some_and(|family| family != declaration.family)
        {
            return Err(ItemObservationPolicyError::Invalid(
                "observation field aliases use different families",
            ));
        }
        let Some(templates) = compatible
            .get(&declaration.family)
            .filter(|templates| !templates.is_empty())
        else {
            return Err(ItemObservationPolicyError::Invalid(
                "observation has no compatible exact template bindings",
            ));
        };
        charge(&mut work, templates.len())?;
        added_template_references = added_template_references
            .checked_add(templates.len())
            .filter(|count| *count <= limits.item_source.max_templates)
            .ok_or(ItemObservationPolicyError::Limit("observation templates"))?;
        items.rules.push(declaration.rule.clone());
        item_source.rule_layouts.push(ItemRuleSourceLayout {
            rule: declaration.rule.id.clone(),
            role: ItemRuleSourceRole::Unresolved,
        });
        observations.push(ItemSourcePreambleObservation {
            rule: declaration.rule.id.clone(),
            field: declaration.field.clone(),
            templates: templates.iter().cloned().collect(),
        });
    }
    items.version = request.version.clone();
    let checked_items = OwnedItemLinePolicy::new(items.clone(), schema, limits.items)?;
    item_source.version = request.version.clone();
    item_source.item_lines = *checked_items.identity();
    item_source.schema_version = OWNED_ITEM_SOURCE_OBSERVATION_POLICY_VERSION;
    item_source.dialect = ItemSourceDialect::PobExportedSingleTextObservationsV1 {
        flag_bindings: flags,
        metadata_rules: metadata,
        single_modifier_conditions: conditions,
        preamble_observations: observations,
    };
    let checked_after = ItemSourceLayoutPolicy::new(
        item_source.clone(),
        &checked_items,
        schema,
        limits.item_source,
    )?;
    Ok(StagedItemObservationPolicy {
        items,
        item_source,
        receipt: ItemObservationPolicyReceipt {
            catalog_sha256,
            request: request_identity,
            before_items: *lines.identity(),
            after_items: *checked_items.identity(),
            before_item_source: *source.identity(),
            after_item_source: *checked_after.identity(),
            catalog_bases: catalog.bases.len(),
            exact_base_bindings,
            added_observations: request.observations.len(),
            added_template_references,
            work_used: limits.max_work - work,
        },
    })
}
