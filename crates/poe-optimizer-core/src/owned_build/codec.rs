use super::*;
use crate::owned_inventory::{InventoryError, InventoryInput, InventorySnapshot};
use crate::owned_project::{BuildProject, ProjectError, ProjectInput};
use serde::{Deserialize, Serialize};
use std::{fmt, io};

/// Independently authored documents; request envelopes contain the same records.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum OwnedDocument {
    Build(Box<BuildSpec>),
    Inventory(Box<InventorySnapshot>),
    Project(Box<BuildProject>),
    Scenario(ScenarioSpec),
    Query(QuerySpec),
    Request(Box<OwnedEvaluationRequest>),
}

#[derive(Debug)]
pub enum CodecError {
    Structure(StructuralError),
    Inventory(InventoryError),
    Project(ProjectError),
    Json(serde_json::Error),
    UnsupportedVersion(u32),
    TooLarge { maximum: usize },
}
impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Structure(error) => error.fmt(f),
            Self::Inventory(error) => error.fmt(f),
            Self::Project(error) => error.fmt(f),
            Self::Json(error) => error.fmt(f),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported owned input schema version {version}")
            }
            Self::TooLarge { maximum } => write!(f, "owned document exceeds {maximum} bytes"),
        }
    }
}
impl std::error::Error for CodecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Structure(e) => Some(e),
            Self::Inventory(e) => Some(e),
            Self::Project(e) => Some(e),
            Self::Json(e) => Some(e),
            _ => None,
        }
    }
}
impl From<StructuralError> for CodecError {
    fn from(value: StructuralError) -> Self {
        Self::Structure(value)
    }
}
impl From<ProjectError> for CodecError {
    fn from(value: ProjectError) -> Self {
        Self::Project(value)
    }
}
impl From<InventoryError> for CodecError {
    fn from(value: InventoryError) -> Self {
        Self::Inventory(value)
    }
}
impl From<serde_json::Error> for CodecError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireInput {
    schema_version: u32,
    document: DocumentInput,
}
#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum DocumentInput {
    Build(Box<BuildInput>),
    Inventory(Box<InventoryInput>),
    Project(Box<ProjectInput>),
    Scenario(ScenarioInput),
    Query(QueryInput),
    Request(Box<OwnedRequestInput>),
}
#[derive(Serialize)]
struct WireOutput<'a> {
    schema_version: u32,
    document: &'a OwnedDocument,
}

/// Bounded JSON decoding. All semantic objects have fixed fields, so serde rejects
/// duplicate/unknown keys instead of silently collapsing them into a generic map.
/// No import source, definition package, filesystem or evaluator is consulted.
pub fn decode_owned(bytes: &[u8], limits: OwnedInputLimits) -> Result<OwnedDocument, CodecError> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(CodecError::TooLarge {
            maximum: limits.max_wire_bytes,
        });
    }
    let wire: WireInput = serde_json::from_slice(bytes)?;
    if wire.schema_version != OWNED_INPUT_SCHEMA_VERSION {
        return Err(CodecError::UnsupportedVersion(wire.schema_version));
    }
    Ok(match wire.document {
        DocumentInput::Build(input) => {
            OwnedDocument::Build(Box::new(BuildSpec::new(*input, limits)?))
        }
        DocumentInput::Project(input) => {
            OwnedDocument::Project(Box::new(BuildProject::new(*input, limits)?))
        }
        DocumentInput::Inventory(input) => {
            OwnedDocument::Inventory(Box::new(InventorySnapshot::new(*input, limits)?))
        }
        DocumentInput::Scenario(input) => {
            OwnedDocument::Scenario(ScenarioSpec::new(input, limits)?)
        }
        DocumentInput::Query(input) => OwnedDocument::Query(QuerySpec::new(input, limits)?),
        DocumentInput::Request(input) => {
            OwnedDocument::Request(Box::new(OwnedEvaluationRequest::new(
                BuildSpec::new(input.build, limits)?,
                ScenarioSpec::new(input.scenario, limits)?,
                QuerySpec::new(input.queries, limits)?,
                limits,
            )?))
        }
    })
}

/// Deterministic owned encoding: unordered records are canonicalized at validated
/// construction; query order and ordered grant-path steps remain meaningful.
/// A tighter caller limit is checked again before encoding immutable inputs.
pub fn encode_owned(
    document: &OwnedDocument,
    limits: OwnedInputLimits,
) -> Result<Vec<u8>, CodecError> {
    match document {
        OwnedDocument::Build(input) => structure::validate_build(input.input(), limits)?,
        OwnedDocument::Inventory(input) => input.validate_limits(limits)?,
        OwnedDocument::Project(input) => input.validate_limits(limits)?,
        OwnedDocument::Scenario(input) => {
            structure::validate_scenario(input.input(), limits, None)?
        }
        OwnedDocument::Query(input) => structure::validate_queries(input.input(), limits, None)?,
        OwnedDocument::Request(input) => structure::validate_request(
            input.build().input(),
            input.scenario().input(),
            input.queries().input(),
            limits,
        )?,
    }
    let mut writer = BoundedWriter {
        bytes: Vec::new(),
        maximum: limits.max_wire_bytes,
        exceeded: false,
    };
    let result = serde_json::to_writer(
        &mut writer,
        &WireOutput {
            schema_version: OWNED_INPUT_SCHEMA_VERSION,
            document,
        },
    );
    if writer.exceeded {
        return Err(CodecError::TooLarge {
            maximum: writer.maximum,
        });
    }
    result?;
    Ok(writer.bytes)
}
struct BoundedWriter {
    bytes: Vec<u8>,
    maximum: usize,
    exceeded: bool,
}
impl io::Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.maximum - self.bytes.len() {
            self.exceeded = true;
            return Err(io::Error::other("owned document byte limit exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
