//! Finite offline facts from the final constructed skill-definition table.
//!
//! The existing bounded static-data constructor resolves module order and later
//! writes. This projection does not interpret source declarations, load a build,
//! or turn an absent field into a gameplay default.
use std::{collections::BTreeMap, io::Write, path::Path};

use mlua::{Table, Value};
use poe_optimizer_import::owned_mapping::{ExternalSourceSystem, SourceFilePin, SourcePin};
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

use crate::{game_data, owned_item_bases::BoundedBytes, skill_identity_extract, source};

// The constructor can load only this source map. Data.lua owns the actual module
// list and order; an unadmitted dependency fails rather than searching the disk.
const PATHS: &[&str] = &[
    "src/Modules/Common.lua",
    "src/Modules/Data.lua",
    "src/Data/Global.lua",
    "src/Data/Gems.lua",
    "src/Data/SkillStatMap.lua",
    "src/Data/Assets.lua",
    "src/Data/Skills/SkillAssets.lua",
    "src/Data/Skills/act_str.lua",
    "src/Data/Skills/act_dex.lua",
    "src/Data/Skills/act_int.lua",
    "src/Data/Skills/other.lua",
    "src/Data/Skills/minion.lua",
    "src/Data/Skills/spectre.lua",
    "src/Data/Skills/sup_str.lua",
    "src/Data/Skills/sup_dex.lua",
    "src/Data/Skills/sup_int.lua",
];

/// Additional projection limits; the shared constructor independently bounds
/// Lua memory and instructions. These values may be tightened, never enlarged.
#[derive(Clone, Copy, Debug)]
pub struct SkillDefinitionFactsExportLimits {
    pub max_source_bytes: usize,
    pub max_definitions: usize,
    pub max_text_bytes: usize,
    pub max_catalog_bytes: usize,
}
impl Default for SkillDefinitionFactsExportLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 32 * 1024 * 1024,
            max_definitions: 4096,
            max_text_bytes: 1024,
            max_catalog_bytes: 2 * 1024 * 1024,
        }
    }
}
impl SkillDefinitionFactsExportLimits {
    fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (name, actual, maximum) in [
            ("source bytes", self.max_source_bytes, hard.max_source_bytes),
            ("definitions", self.max_definitions, hard.max_definitions),
            ("text bytes", self.max_text_bytes, hard.max_text_bytes),
            (
                "catalog bytes",
                self.max_catalog_bytes,
                hard.max_catalog_bytes,
            ),
        ] {
            if actual == 0 || actual > maximum {
                return Err(invalid(format!("invalid {name} limit")));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum SkillDefinitionFactsExportError {
    #[error("owned skill-definition facts export: {0}")]
    Invalid(String),
    #[error(transparent)]
    Source(#[from] source::SourceError),
    #[error(transparent)]
    Extraction(#[from] game_data::GameDataExtractionError),
    #[error(transparent)]
    Lua(#[from] mlua::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, SkillDefinitionFactsExportError>;
fn invalid(message: impl Into<String>) -> SkillDefinitionFactsExportError {
    SkillDefinitionFactsExportError::Invalid(message.into())
}

/// Exact final source identity, with absence distinct from an explicit false.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillDefinitionFact {
    pub id: String,
    #[serde(deserialize_with = "required_optional_bool")]
    pub cannot_be_supported: Option<bool>,
}
fn required_optional_bool<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<bool>, D::Error> {
    Option::<bool>::deserialize(deserializer)
}

/// Portable acquired facts, not a checked native rule package or source proof.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillDefinitionFactsCatalog {
    pub schema_version: u32,
    pub source: SourcePin,
    pub skills: Vec<SkillDefinitionFact>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SkillDefinitionFactsExportEvidence {
    pub schema_version: u32,
    pub upstream_revision: String,
    pub source_manifest_sha256: String,
    pub extractor_sha256: String,
    /// All admitted constructor sources, including Common.lua and Data.lua.
    pub extraction_source_files: Vec<SourceFilePin>,
    /// Actual LoadModule order, without attempting to reconstruct winning writes.
    pub module_order: Vec<String>,
    /// SHA-256 of catalog_bytes(): pretty JSON followed by exactly one LF.
    pub catalog_sha256: String,
    pub definitions: usize,
    pub absent: usize,
    pub explicit_false: usize,
    pub explicit_true: usize,
}

/// Only fresh pinned acquisition creates this handle. Serialized provenance
/// cannot authorize caller-supplied facts.
#[derive(Debug)]
pub struct AuthenticatedSkillDefinitionFactsExport {
    catalog: SkillDefinitionFactsCatalog,
    catalog_bytes: Vec<u8>,
    evidence: SkillDefinitionFactsExportEvidence,
}
impl AuthenticatedSkillDefinitionFactsExport {
    pub fn catalog(&self) -> &SkillDefinitionFactsCatalog {
        &self.catalog
    }
    pub fn catalog_bytes(&self) -> &[u8] {
        &self.catalog_bytes
    }
    pub fn evidence(&self) -> &SkillDefinitionFactsExportEvidence {
        &self.evidence
    }
    /// A missing definition is an error, independently of an absent field.
    pub fn definition(&self, id: &str) -> Result<&SkillDefinitionFact> {
        self.catalog
            .skills
            .binary_search_by(|row| row.id.as_str().cmp(id))
            .map(|index| &self.catalog.skills[index])
            .map_err(|_| invalid(format!("missing skill definition {id}")))
    }
    pub fn validate_catalog(&self, candidate: &SkillDefinitionFactsCatalog) -> Result<()> {
        if candidate != &self.catalog {
            return Err(invalid(
                "catalog differs from independently authenticated extraction",
            ));
        }
        Ok(())
    }
}

/// Capture every final skill definition through the existing static constructor.
/// This exports source facts only: no occurrence-level noSupports policy, UI
/// state, native coverage, support applicability, or numerical result is implied.
pub fn export_owned_skill_definition_facts(
    root: &Path,
    limits: SkillDefinitionFactsExportLimits,
) -> Result<AuthenticatedSkillDefinitionFactsExport> {
    limits.validate()?;
    let source_manifest_sha256 = source::verify(root)?;
    let sources = read_sources(root, limits)?;
    let (skills, module_order) = construct_facts(&sources, limits)?;
    let extraction_source_files: Vec<_> = sources
        .iter()
        .map(|(path, text)| SourceFilePin {
            path: path.clone(),
            sha256: game_data::hash(text.as_bytes()),
        })
        .collect();
    let catalog = SkillDefinitionFactsCatalog {
        schema_version: 1,
        source: SourcePin {
            system: ExternalSourceSystem::PathOfBuilding2,
            revision: source::UPSTREAM_REVISION.into(),
            files: extraction_source_files.clone(),
        },
        skills,
    };
    let catalog_bytes = catalog_bytes(&catalog, limits.max_catalog_bytes)?;
    let evidence = SkillDefinitionFactsExportEvidence {
        schema_version: 1,
        upstream_revision: source::UPSTREAM_REVISION.into(),
        source_manifest_sha256,
        extractor_sha256: game_data::hash(
            format!(
                "owned-skill-definition-facts-v1\n{}\n{}",
                game_data::extractor_sha256(),
                include_str!("owned_skill_definition_facts.rs").replace("\r\n", "\n")
            )
            .as_bytes(),
        ),
        extraction_source_files,
        module_order,
        catalog_sha256: game_data::hash(&catalog_bytes),
        definitions: catalog.skills.len(),
        absent: catalog
            .skills
            .iter()
            .filter(|row| row.cannot_be_supported.is_none())
            .count(),
        explicit_false: catalog
            .skills
            .iter()
            .filter(|row| row.cannot_be_supported == Some(false))
            .count(),
        explicit_true: catalog
            .skills
            .iter()
            .filter(|row| row.cannot_be_supported == Some(true))
            .count(),
    };
    Ok(AuthenticatedSkillDefinitionFactsExport {
        catalog,
        catalog_bytes,
        evidence,
    })
}

fn read_sources(
    root: &Path,
    limits: SkillDefinitionFactsExportLimits,
) -> Result<BTreeMap<String, String>> {
    let mut sources = BTreeMap::new();
    let mut bytes = 0usize;
    for path in PATHS {
        let text = source::read_verified_text(root, path)?;
        bytes = bytes
            .checked_add(text.len())
            .ok_or_else(|| invalid("source byte count overflow"))?;
        if bytes > limits.max_source_bytes {
            return Err(invalid("source byte limit"));
        }
        sources.insert((*path).to_owned(), text);
    }
    Ok(sources)
}

// Unauthenticated source-map entry point is private. Tests can change constructor
// inputs without ever minting a public authenticated export.
fn construct_facts(
    sources: &BTreeMap<String, String>,
    limits: SkillDefinitionFactsExportLimits,
) -> Result<(Vec<SkillDefinitionFact>, Vec<String>)> {
    let (_lua, result, modules) = skill_identity_extract::construct_catalog(sources)?;
    let data = plain_table(result.raw_get("data")?, "data")?;
    let skills = plain_table(data.raw_get("skills")?, "data.skills")?;
    Ok((project(&skills, limits)?, modules))
}
fn plain_table(value: Value, role: &str) -> Result<Table> {
    match value {
        Value::Table(table) if table.metatable().is_none() => Ok(table),
        Value::Table(_) => Err(invalid(format!("{role} must not have a metatable"))),
        _ => Err(invalid(format!("{role} must be a table"))),
    }
}
fn text(value: Value, limits: SkillDefinitionFactsExportLimits, role: &str) -> Result<String> {
    let Value::String(value) = value else {
        return Err(invalid(format!("{role} must be text")));
    };
    let text = value.to_str()?;
    if text.is_empty() || text.len() > limits.max_text_bytes {
        return Err(invalid(format!("{role} text byte limit")));
    }
    Ok(text.to_owned())
}
fn project(
    skills: &Table,
    limits: SkillDefinitionFactsExportLimits,
) -> Result<Vec<SkillDefinitionFact>> {
    let mut rows = Vec::new();
    for pair in skills.pairs::<Value, Value>() {
        if rows.len() >= limits.max_definitions {
            return Err(invalid("definition count limit"));
        }
        let (key, value) = pair?;
        let id = text(key, limits, "skill key")?;
        let definition = plain_table(value, &id)?;
        if text(definition.raw_get("id")?, limits, "skill id")? != id {
            return Err(invalid(format!("constructed skill ID mismatch {id}")));
        }
        let cannot_be_supported = match definition.raw_get::<Value>("cannotBeSupported")? {
            Value::Nil => None,
            Value::Boolean(value) => Some(value),
            _ => {
                return Err(invalid(format!(
                    "{id}.cannotBeSupported must be Boolean or absent"
                )));
            }
        };
        rows.push(SkillDefinitionFact {
            id,
            cannot_be_supported,
        });
    }
    if rows.is_empty() {
        return Err(invalid("no skill definitions"));
    }
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(rows)
}
fn catalog_bytes(catalog: &SkillDefinitionFactsCatalog, limit: usize) -> Result<Vec<u8>> {
    let mut output = BoundedBytes {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer_pretty(&mut output, catalog)?;
    output
        .write_all(b"\n")
        .map_err(|_| invalid("catalog byte limit"))?;
    Ok(output.bytes)
}

#[cfg(test)]
#[path = "owned_skill_definition_facts/tests.rs"]
mod tests;
