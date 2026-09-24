//! Finite offline membership authoring. Runtime packages retain materialized sets.
//! Patches do not infer item legality, close coverage, or repair stale bindings.
use crate::{
    owned_recipe::StagedOwnedRecipe,
    owned_recipe_extension::{
        OwnedRecipeExtension, RecipeExtensionError, RecipeExtensionLimits, SchemaExtensionEntry,
        StagedRecipeExtension, extend_owned_recipe,
    },
};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_schema::*,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
};

pub const OWNED_RECIPE_MEMBERSHIP_PATCH_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeMembershipPatchBindings {
    pub registry: OwnedContentDigest,
    pub definitions: DataIdentity,
    pub rules: OwnedContentDigest,
    pub routing: OwnedContentDigest,
}
impl RecipeMembershipPatchBindings {
    pub fn from_recipe(recipe: &StagedOwnedRecipe) -> Self {
        Self {
            registry: recipe.manifest().registry,
            definitions: recipe.schema().identity().clone(),
            rules: *recipe.rules().identity(),
            routing: *recipe.routing().identity(),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RecipeMembershipPatch {
    /// Both lists are nonempty, strictly sorted typed IDs. A template may occur
    /// in only one group. Reusing an existing member is rejected, not ignored.
    ItemTemplateModifiers {
        templates: Vec<ItemTemplateDefId>,
        add: Vec<ModifierDefId>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeMembershipPatchInput {
    pub schema_version: u32,
    pub version: OwnedDefinitionKey,
    pub before: RecipeMembershipPatchBindings,
    /// Exact V1 allocation/rule extension supplied alongside this request.
    pub extension: OwnedContentDigest,
    pub patches: Vec<RecipeMembershipPatch>,
}
#[derive(Clone, Copy, Debug)]
pub struct RecipeMembershipPatchLimits {
    pub max_request_bytes: usize,
    pub max_groups: usize,
    pub max_target_references: usize,
    pub max_modifier_references: usize,
    pub max_insertions: usize,
    /// One unit per indexed operation or 64 serialized bytes; serialization and
    /// copying are charged separately. Inner extension validation has its own cap.
    pub max_work: usize,
    pub extension: RecipeExtensionLimits,
}
impl Default for RecipeMembershipPatchLimits {
    fn default() -> Self {
        Self {
            max_request_bytes: 4 * 1024 * 1024,
            max_groups: 256,
            max_target_references: 100_000,
            max_modifier_references: 100_000,
            max_insertions: 1_000_000,
            max_work: 4_000_000,
            extension: RecipeExtensionLimits::default(),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum RecipeMembershipPatchError {
    #[error("invalid recipe membership patch: {0}")]
    Invalid(&'static str),
    #[error("recipe membership patch limit: {0}")]
    Limit(&'static str),
    #[error(transparent)]
    Extension(#[from] RecipeExtensionError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, RecipeMembershipPatchError>;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeMembershipPatchReceipt {
    pub request: OwnedContentDigest,
    pub before: RecipeMembershipPatchBindings,
    pub supplied_extension: OwnedContentDigest,
    pub expanded_extension: OwnedContentDigest,
    pub after_definitions: DataIdentity,
    pub patched_templates: usize,
    pub inserted_members: usize,
    pub expanded_bytes: usize,
    pub work_used: usize,
}
pub struct StagedRecipeMembershipPatch {
    pub expanded: OwnedRecipeExtension,
    pub staged: StagedRecipeExtension,
    pub receipt: RecipeMembershipPatchReceipt,
}
fn charge(left: &mut usize, count: usize) -> Result<()> {
    *left = left
        .checked_sub(count)
        .ok_or(RecipeMembershipPatchError::Limit("work"))?;
    Ok(())
}
fn units(bytes: usize) -> usize {
    bytes.div_ceil(64)
}
impl RecipeMembershipPatchLimits {
    fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (name, value, maximum) in [
            (
                "request bytes",
                self.max_request_bytes,
                hard.max_request_bytes,
            ),
            ("groups", self.max_groups, hard.max_groups),
            (
                "target references",
                self.max_target_references,
                hard.max_target_references,
            ),
            (
                "modifier references",
                self.max_modifier_references,
                hard.max_modifier_references,
            ),
            ("insertions", self.max_insertions, hard.max_insertions),
            ("work", self.max_work, hard.max_work),
            (
                "extension bytes",
                self.extension.max_wire_bytes,
                hard.extension.max_wire_bytes,
            ),
            (
                "extension work",
                self.extension.max_work,
                hard.extension.max_work,
            ),
        ] {
            if value == 0 || value > maximum {
                return Err(RecipeMembershipPatchError::Limit(name));
            }
        }
        Ok(())
    }
}
// Count bytes without buffering. Charge incrementally before processing each
// write, so a small work limit stops traversing a large nested descriptor early.
struct Measure<'a> {
    bytes: usize,
    maximum: usize,
    left: &'a mut usize,
    failure: Option<&'static str>,
}
impl io::Write for Measure<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self
            .bytes
            .checked_add(bytes.len())
            .filter(|n| *n <= self.maximum);
        let Some(next) = next else {
            self.failure = Some("expanded bytes");
            return Err(io::Error::other("patch byte limit"));
        };
        if charge(self.left, units(next) - units(self.bytes)).is_err() {
            self.failure = Some("work");
            return Err(io::Error::other("patch work limit"));
        }
        self.bytes = next;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn measure<T: Serialize>(value: &T, maximum: usize, left: &mut usize) -> Result<usize> {
    let mut writer = Measure {
        bytes: 0,
        maximum,
        left,
        failure: None,
    };
    let result = serde_json::to_writer(&mut writer, value);
    if let Some(reason) = writer.failure {
        return Err(RecipeMembershipPatchError::Limit(reason));
    }
    result?;
    Ok(writer.bytes)
}
#[derive(Serialize)]
struct Envelope<T> {
    kind: &'static str,
    value: T,
}
fn entry_key(entry: &SchemaExtensionEntry) -> OwnedDefinitionKey {
    match entry {
        SchemaExtensionEntry::Definition(row) => row.address().key().clone(),
        SchemaExtensionEntry::Slot(row) => row.address().key().clone(),
    }
}

/// Decode only after checking wire size. Typed callers are independently checked
/// by the compiler, including their aggregate references and serialization size.
pub fn decode_recipe_membership_patch(
    bytes: &[u8],
    limits: RecipeMembershipPatchLimits,
) -> Result<RecipeMembershipPatchInput> {
    limits.validate()?;
    if bytes.len() > limits.max_request_bytes {
        return Err(RecipeMembershipPatchError::Limit("request bytes"));
    }
    Ok(serde_json::from_slice(bytes)?)
}

pub fn compile_owned_recipe_membership_patch(
    base: &StagedOwnedRecipe,
    extension: &OwnedRecipeExtension,
    request: &RecipeMembershipPatchInput,
    limits: RecipeMembershipPatchLimits,
) -> Result<StagedRecipeMembershipPatch> {
    limits.validate()?;
    if request.schema_version != OWNED_RECIPE_MEMBERSHIP_PATCH_VERSION
        || extension.schema_version != 1
    {
        return Err(RecipeMembershipPatchError::Invalid("version"));
    }
    if request.before != RecipeMembershipPatchBindings::from_recipe(base) {
        return Err(RecipeMembershipPatchError::Invalid("prior bindings"));
    }
    if request.patches.is_empty() || request.patches.len() > limits.max_groups {
        return Err(RecipeMembershipPatchError::Limit("groups"));
    }
    let mut targets = 0usize;
    let mut modifiers = 0usize;
    let mut insertions = 0usize;
    for RecipeMembershipPatch::ItemTemplateModifiers { templates, add } in &request.patches {
        if templates.is_empty() || add.is_empty() {
            return Err(RecipeMembershipPatchError::Invalid(
                "empty membership group",
            ));
        }
        targets = targets
            .checked_add(templates.len())
            .filter(|n| *n <= limits.max_target_references)
            .ok_or(RecipeMembershipPatchError::Limit("target references"))?;
        modifiers = modifiers
            .checked_add(add.len())
            .filter(|n| *n <= limits.max_modifier_references)
            .ok_or(RecipeMembershipPatchError::Limit("modifier references"))?;
        insertions = templates
            .len()
            .checked_mul(add.len())
            .and_then(|n| insertions.checked_add(n))
            .filter(|n| *n <= limits.max_insertions)
            .ok_or(RecipeMembershipPatchError::Limit("insertions"))?;
    }
    let mut left = limits.max_work;
    charge(
        &mut left,
        request.patches.len() + targets + modifiers + insertions,
    )?;
    let request_bytes = measure(request, limits.max_request_bytes, &mut left)?;
    charge(&mut left, units(request_bytes))?;
    let request_digest = digest_owned(
        "owned-recipe-membership-patch-v1",
        request,
        limits.max_request_bytes,
    )?;
    let extension_bytes = measure(extension, limits.extension.max_wire_bytes, &mut left)?;
    charge(&mut left, units(extension_bytes))?;
    let extension_digest = digest_owned(
        "owned-recipe-extension-v1",
        extension,
        limits.extension.max_wire_bytes,
    )?;
    if request.extension != extension_digest {
        return Err(RecipeMembershipPatchError::Invalid("extension binding"));
    }
    charge(&mut left, extension.schema.len())?;
    let mut extension_definitions = BTreeMap::new();
    let mut previous = None;
    for entry in &extension.schema {
        let key = entry_key(entry);
        if previous.as_ref().is_some_and(|prior| prior >= &key) {
            return Err(RecipeMembershipPatchError::Invalid(
                "extension global key order",
            ));
        }
        previous = Some(key);
        if let SchemaExtensionEntry::Definition(row) = entry {
            extension_definitions.insert(row.address(), row);
        }
    }
    let namespace = base.schema().namespace();
    let mut selected = BTreeSet::new();
    let mut planned = Vec::with_capacity(targets);
    let mut maximum_expanded = extension_bytes;
    let mut schema_entries = extension.schema.len();
    for RecipeMembershipPatch::ItemTemplateModifiers { templates, add } in &request.patches {
        if templates.windows(2).any(|pair| pair[0] >= pair[1])
            || add.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(RecipeMembershipPatchError::Invalid(
                "membership IDs need unique canonical order",
            ));
        }
        let mut added_bytes = 0usize;
        for modifier in add {
            if modifier.namespace() != namespace {
                return Err(RecipeMembershipPatchError::Invalid("foreign modifier"));
            }
            let address = modifier.address();
            let definition = extension_definitions
                .get(&address)
                .copied()
                .or_else(|| base.schema().lookup_definition(&address));
            if !matches!(definition, Some(DefinitionDescriptor::Modifier(row)) if matches!(row.schema, SchemaState::Known(_)))
            {
                return Err(RecipeMembershipPatchError::Invalid(
                    "unknown or unmapped modifier",
                ));
            }
            // Include commas conservatively; this also bounds copying every ID
            // into every selected descriptor before allocating the expansion.
            added_bytes = added_bytes
                .checked_add(measure(modifier, limits.extension.max_wire_bytes, &mut left)? + 1)
                .ok_or(RecipeMembershipPatchError::Limit("expanded bytes"))?;
        }
        for template in templates {
            if template.namespace() != namespace {
                return Err(RecipeMembershipPatchError::Invalid("foreign template"));
            }
            if !selected.insert(template.clone()) {
                return Err(RecipeMembershipPatchError::Invalid(
                    "duplicate template target",
                ));
            }
            let address = template.address();
            if extension_definitions.contains_key(&address) {
                return Err(RecipeMembershipPatchError::Invalid(
                    "full-record membership conflict",
                ));
            }
            let Some(DefinitionDescriptor::ItemTemplate(row)) =
                base.schema().lookup_definition(&address)
            else {
                return Err(RecipeMembershipPatchError::Invalid("unknown template"));
            };
            let SchemaState::Known(schema) = &row.schema else {
                return Err(RecipeMembershipPatchError::Invalid("unmapped template"));
            };
            if schema.modifiers.is_complete() {
                return Err(RecipeMembershipPatchError::Invalid(
                    "Complete modifier membership",
                ));
            }
            charge(&mut left, schema.modifiers.members.len() + add.len())?;
            // Both lists are canonical. Linear comparison checks existing facts
            // without an uncharged targets-by-existing-members cross product.
            let mut existing = schema.modifiers.members.iter().peekable();
            for modifier in add {
                while existing.peek().is_some_and(|member| *member < modifier) {
                    existing.next();
                }
                if existing.peek().is_some_and(|member| *member == modifier) {
                    return Err(RecipeMembershipPatchError::Invalid(
                        "member already present",
                    ));
                }
            }
            let descriptor_bytes = measure(
                &Envelope {
                    kind: "definition",
                    value: Envelope {
                        kind: "item_template",
                        value: row,
                    },
                },
                limits.extension.max_wire_bytes,
                &mut left,
            )?;
            // Empty member arrays need one fewer comma. Borrowed wrappers match
            // the V1 wire exactly, so preflight accepts the exact byte boundary.
            let clone_bytes = descriptor_bytes
                .checked_add(added_bytes - usize::from(schema.modifiers.members.is_empty()))
                .ok_or(RecipeMembershipPatchError::Limit("expanded bytes"))?;
            maximum_expanded = maximum_expanded
                .checked_add(clone_bytes)
                .and_then(|n| n.checked_add(usize::from(schema_entries != 0)))
                .filter(|n| *n <= limits.extension.max_wire_bytes)
                .ok_or(RecipeMembershipPatchError::Limit("expanded bytes"))?;
            schema_entries += 1;
            charge(&mut left, units(clone_bytes))?;
            planned.push((row, add));
        }
    }
    charge(&mut left, units(extension_bytes))?;
    let mut expanded = extension.clone();
    for (prior, add) in planned {
        let mut row = prior.clone();
        let SchemaState::Known(schema) = &mut row.schema else {
            unreachable!("preflight checked descriptor");
        };
        schema.modifiers.members.extend(add.iter().cloned());
        expanded.schema.push(SchemaExtensionEntry::Definition(
            DefinitionDescriptor::ItemTemplate(row),
        ));
    }
    charge(
        &mut left,
        expanded
            .schema
            .len()
            .saturating_mul(expanded.schema.len().ilog2() as usize + 1),
    )?;
    expanded.schema.sort_by_cached_key(entry_key);
    let expanded_bytes = measure(&expanded, limits.extension.max_wire_bytes, &mut left)?;
    // The extension constructor hashes this stream again, then applies all old
    // allocation, schema, rule and membership-proof checks under their own caps.
    charge(&mut left, units(expanded_bytes))?;
    let staged = extend_owned_recipe(base, &expanded, limits.extension)?;
    let receipt = RecipeMembershipPatchReceipt {
        request: request_digest,
        before: request.before.clone(),
        supplied_extension: extension_digest,
        expanded_extension: staged.receipt.extension,
        after_definitions: staged.receipt.after_definitions.clone(),
        patched_templates: targets,
        inserted_members: insertions,
        expanded_bytes,
        work_used: limits.max_work - left,
    };
    Ok(StagedRecipeMembershipPatch {
        expanded,
        staged,
        receipt,
    })
}
