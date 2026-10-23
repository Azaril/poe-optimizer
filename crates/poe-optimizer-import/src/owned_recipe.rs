//! Offline assembly of persisted owned recipes. No ID allocation, source parsing or evaluation.
//! Each constituent keeps its identity and coverage contract. Compilation is not game coverage.
use crate::owned_mapping::{
    OwnedIdRegistrySnapshot, OwnedMappingError, OwnedMappingLimits, RegistryInput, encode_registry,
};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_content::{ContentDigestError, OwnedContentDigest},
    owned_definitions::{GameVersionNamespace, OwnedDefinitionKey},
    owned_routing::ActionRoutingInput,
    owned_rules::RulePackageInput,
    owned_schema::SchemaSubject,
};
use poe_optimizer_data::{
    owned_routing::{OwnedActionRouting, RoutingError, RoutingLimits, encode_action_routing},
    owned_rules::{OwnedRulePackage, RuleStorageError, RuleStorageLimits, encode_rule_package},
    owned_schema::{
        OwnedDefinitionSchemaPackage, OwnedSchemaLimits, SchemaPackageError, SchemaPackageInput,
        encode_schema_package,
    },
};
use poe_optimizer_engine::owned_rules::{RuleError, RuleLimits};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

mod checked;
pub use checked::CheckedOwnedRecipe;
pub(crate) use checked::{check_recipe_input, validate_definition_space};

pub const OWNED_RECIPE_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedRecipeInput {
    pub schema_version: u32,
    pub registry: RegistryInput,
    pub schema: SchemaPackageInput,
    pub rules: RulePackageInput,
    pub routing: ActionRoutingInput,
}

/// Borrowed wire view in exactly the field order of `OwnedRecipeInput`.
/// A checked recipe supplies canonical constituents; an input view preserves its
/// supplied ordering until those constituents pass their existing constructors.
#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct OwnedRecipeView<'a> {
    pub(crate) schema_version: u32,
    pub(crate) registry: &'a RegistryInput,
    pub(crate) schema: &'a SchemaPackageInput,
    pub(crate) rules: &'a RulePackageInput,
    pub(crate) routing: &'a ActionRoutingInput,
}
impl<'a> From<&'a OwnedRecipeInput> for OwnedRecipeView<'a> {
    fn from(input: &'a OwnedRecipeInput) -> Self {
        Self {
            schema_version: input.schema_version,
            registry: &input.registry,
            schema: &input.schema,
            rules: &input.rules,
            routing: &input.routing,
        }
    }
}
impl OwnedRecipeView<'_> {
    /// Explicitly own the borrowed constituents for an existing typed constructor.
    pub(crate) fn to_input(self) -> OwnedRecipeInput {
        OwnedRecipeInput {
            schema_version: self.schema_version,
            registry: self.registry.clone(),
            schema: self.schema.clone(),
            rules: self.rules.clone(),
            routing: self.routing.clone(),
        }
    }
}

/// Constituent constructors retain independent aggregate resource budgets. This
/// wrapper also bounds total input/output bytes and indexed registry checks.
#[derive(Clone, Copy, Debug)]
pub struct OwnedRecipeLimits {
    pub max_wire_bytes: usize,
    pub max_output_bytes: usize,
    pub max_registry_checks: usize,
    pub registry: OwnedMappingLimits,
    pub schema: OwnedSchemaLimits,
    pub rules: RuleStorageLimits,
    pub compile: RuleLimits,
    pub routing: RoutingLimits,
}
impl Default for OwnedRecipeLimits {
    fn default() -> Self {
        Self {
            max_wire_bytes: 64 * 1024 * 1024,
            max_output_bytes: 64 * 1024 * 1024,
            max_registry_checks: 1_000_000,
            registry: OwnedMappingLimits::default(),
            schema: OwnedSchemaLimits::default(),
            rules: RuleStorageLimits::default(),
            compile: RuleLimits::default(),
            routing: RoutingLimits::default(),
        }
    }
}
impl OwnedRecipeLimits {
    fn validate(self) -> Result<(), OwnedRecipeError> {
        let hard = Self::default();
        for (name, value, maximum) in [
            ("input bytes", self.max_wire_bytes, hard.max_wire_bytes),
            ("output bytes", self.max_output_bytes, hard.max_output_bytes),
            (
                "registry checks",
                self.max_registry_checks,
                hard.max_registry_checks,
            ),
        ] {
            if value == 0 || value > maximum {
                return Err(OwnedRecipeError::InvalidLimit(name));
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OwnedRecipeError {
    #[error("unsupported owned recipe version {0}")]
    Version(u32),
    #[error("invalid recipe limit: {0}")]
    InvalidLimit(&'static str),
    #[error("owned recipe exceeds {0}")]
    Limit(&'static str),
    #[error("recipe registry and schema namespaces disagree")]
    Namespace,
    #[error("schema address is not active in the supplied registry: {0:?}")]
    Unregistered(Box<SchemaSubject>),
    #[error(transparent)]
    Registry(#[from] OwnedMappingError),
    #[error(transparent)]
    Schema(#[from] SchemaPackageError),
    #[error(transparent)]
    Rules(#[from] RuleStorageError),
    #[error(transparent)]
    Compile(#[from] RuleError),
    #[error(transparent)]
    Routing(#[from] RoutingError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RecipeArtifactManifest {
    pub file: &'static str,
    pub bytes: usize,
    pub sha256: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OwnedRecipeManifest {
    pub schema_version: u32,
    pub document_kind: &'static str,
    /// Recipe commitment selected by validation. Standalone assembly retains the
    /// supplied typed input; a complete release commits canonical constituents.
    pub recipe: OwnedContentDigest,
    pub namespace: GameVersionNamespace,
    pub registry: OwnedContentDigest,
    pub definitions: DataIdentity,
    pub rules: OwnedContentDigest,
    pub compiled_rules: OwnedContentDigest,
    pub routing: OwnedContentDigest,
    pub operations_version: OwnedDefinitionKey,
    pub rule_semantics_version: OwnedDefinitionKey,
    pub partial_rule_owners: usize,
    pub partial_route_outputs: usize,
    /// Missing entries stay unknown; these counts do not certify completeness.
    pub coverage: &'static str,
    pub calculation: &'static str,
    pub whole_build_parity: &'static str,
    pub artifacts: Vec<RecipeArtifactManifest>,
}
#[derive(Debug)]
pub struct RecipeArtifact {
    name: &'static str,
    bytes: Vec<u8>,
}
impl RecipeArtifact {
    pub fn name(&self) -> &'static str {
        self.name
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Immutable validated artifacts ready for host publication. Construction never
/// edits the persisted registry. The host owns no-clobber persistence.
pub struct StagedOwnedRecipe {
    checked: CheckedOwnedRecipe,
    manifest: OwnedRecipeManifest,
    artifacts: Vec<RecipeArtifact>,
}
impl StagedOwnedRecipe {
    pub fn checked(&self) -> &CheckedOwnedRecipe {
        &self.checked
    }
    pub fn registry(&self) -> &OwnedIdRegistrySnapshot {
        self.checked.registry()
    }
    pub fn schema(&self) -> &OwnedDefinitionSchemaPackage {
        self.checked.schema().as_ref()
    }
    pub fn rules(&self) -> &OwnedRulePackage {
        self.checked.rules().as_ref()
    }
    pub fn routing(&self) -> &OwnedActionRouting {
        self.checked.routing().as_ref()
    }
    pub fn manifest(&self) -> &OwnedRecipeManifest {
        &self.manifest
    }
    /// Fixed safe basenames, including manifest.json last.
    pub fn artifacts(&self) -> &[RecipeArtifact] {
        &self.artifacts
    }
}

pub fn decode_owned_recipe(
    bytes: &[u8],
    limits: OwnedRecipeLimits,
) -> Result<StagedOwnedRecipe, OwnedRecipeError> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(OwnedRecipeError::Limit("input bytes"));
    }
    // No Value intermediate: all nested duplicate/missing/unknown fields reject.
    assemble_owned_recipe(serde_json::from_slice(bytes)?, limits)
}

pub fn assemble_owned_recipe(
    input: OwnedRecipeInput,
    limits: OwnedRecipeLimits,
) -> Result<StagedOwnedRecipe, OwnedRecipeError> {
    let checked = CheckedOwnedRecipe::new(input, limits)?;
    stage_checked_recipe(checked, limits)
}

/// Encode already checked constituents without compiling the executable again.
/// The checked recipe carries constructor validation; these limits bound staging.
pub(crate) fn stage_checked_recipe(
    checked: CheckedOwnedRecipe,
    limits: OwnedRecipeLimits,
) -> Result<StagedOwnedRecipe, OwnedRecipeError> {
    limits.validate()?;
    let registry = checked.registry();
    let schema = checked.schema().as_ref();
    let rules = checked.rules().as_ref();
    let routing = checked.routing().as_ref();
    let mut artifacts = Vec::with_capacity(5);
    let mut used = 0usize;
    // Each encoder has its own ceiling; shared accounting happens before retaining
    // each additional artifact. Only one constituent-sized temporary is admitted.
    push_artifact(
        &mut artifacts,
        &mut used,
        limits.max_output_bytes,
        "registry.json",
        encode_registry(registry, limits.registry)?,
    )?;
    push_artifact(
        &mut artifacts,
        &mut used,
        limits.max_output_bytes,
        "schema.json",
        encode_schema_package(schema, limits.schema)?,
    )?;
    push_artifact(
        &mut artifacts,
        &mut used,
        limits.max_output_bytes,
        "rules.json",
        encode_rule_package(rules, limits.rules)?,
    )?;
    push_artifact(
        &mut artifacts,
        &mut used,
        limits.max_output_bytes,
        "routing.json",
        encode_action_routing(routing, limits.routing)?,
    )?;
    let manifest = checked.manifest(
        artifacts
            .iter()
            .map(|a| RecipeArtifactManifest {
                file: a.name,
                bytes: a.bytes.len(),
                sha256: format!("{:x}", Sha256::digest(&a.bytes)),
            })
            .collect(),
    )?;
    push_artifact(
        &mut artifacts,
        &mut used,
        limits.max_output_bytes,
        "manifest.json",
        serde_json::to_vec(&manifest)?,
    )?;
    Ok(StagedOwnedRecipe {
        checked,
        manifest,
        artifacts,
    })
}

fn push_artifact(
    output: &mut Vec<RecipeArtifact>,
    used: &mut usize,
    maximum: usize,
    name: &'static str,
    bytes: Vec<u8>,
) -> Result<(), OwnedRecipeError> {
    *used = used
        .checked_add(bytes.len())
        .ok_or(OwnedRecipeError::Limit("output bytes"))?;
    if *used > maximum {
        return Err(OwnedRecipeError::Limit("output bytes"));
    }
    output.push(RecipeArtifact { name, bytes });
    Ok(())
}
