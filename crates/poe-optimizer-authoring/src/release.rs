//! Declarative final-release composition using the existing typed checker.
//!
//! KDL packets and DSL programs compose into the same checked rule package.
//! JSON constituents are temporary migration inputs. Gzip is explicit, lossless
//! transport for retained fixtures. This module
//! performs no filesystem access, source acquisition, patch-history replay or
//! evaluation. Publication happens only after complete assembly succeeds.
use crate::{ManifestError, ManifestLimits, manifest_parse::parse_document};
use poe_optimizer_core::{
    owned_content::OwnedContentDigest, owned_definitions::OwnedDefinitionKey,
};
use poe_optimizer_import::{
    owned_normalize::NamedQuerySet,
    owned_recipe::{OWNED_RECIPE_VERSION, OwnedRecipeInput},
    owned_release::{
        OWNED_EVALUATION_RELEASE_VERSION, OWNED_RELEASE_VERSION, OwnedReleaseError,
        OwnedReleaseInput, OwnedReleaseLimits, StagedOwnedRelease, assemble_owned_release,
        check_release_source_dependencies,
    },
    owned_release_evaluation::{OwnedReleaseEvaluationInput, OwnedReleaseSupportInput},
};
use serde::de::DeserializeOwned;
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    io::{self, Read},
};

mod rules;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceEncoding {
    Plain,
    Gzip,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourceRole {
    Constituent,
    Names,
    Packet,
    Rule,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseSourceSpec {
    path: String,
    encoding: SourceEncoding,
    role: SourceRole,
}
impl ReleaseSourceSpec {
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn encoding(&self) -> SourceEncoding {
        self.encoding
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Component {
    Registry,
    Schema,
    Rules,
    Routing,
    Mapping,
    Roles,
    Normalization,
    Rewards,
    Items,
    ItemSource,
    Tree,
    Metrics,
    Stages,
    Preparation,
    Inputs,
    Receiving,
    Outputs,
    Provenance,
}
impl Component {
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "registry" => Self::Registry,
            "schema" => Self::Schema,
            "rules" => Self::Rules,
            "routing" => Self::Routing,
            "mapping" => Self::Mapping,
            "roles" => Self::Roles,
            "normalization" => Self::Normalization,
            "rewards" => Self::Rewards,
            "items" => Self::Items,
            "item-source" => Self::ItemSource,
            "tree-normalization" => Self::Tree,
            "metrics" => Self::Metrics,
            "support-stages" => Self::Stages,
            "support-preparation" => Self::Preparation,
            "support-inputs" => Self::Inputs,
            "support-receiving" => Self::Receiving,
            "support-outputs" => Self::Outputs,
            "provenance" => Self::Provenance,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ReleaseManifest {
    sources: Vec<ReleaseSourceSpec>,
    components: BTreeMap<Component, usize>,
    queries: Vec<(OwnedDefinitionKey, usize)>,
    packets: Vec<RulePacket>,
    rules: BTreeSet<usize>,
    expected_input: Option<OwnedContentDigest>,
}

#[derive(Clone, Copy, Debug)]
struct RulePacket {
    packet: usize,
    names: usize,
}
impl ReleaseManifest {
    pub fn sources(&self) -> &[ReleaseSourceSpec] {
        &self.sources
    }
    pub fn expected_input(&self) -> Option<OwnedContentDigest> {
        self.expected_input
    }

    fn validate_counts(&self, limits: ReleaseBuildLimits) -> Result<()> {
        if self.sources.len() > limits.max_sources {
            return Err(ReleaseBuildError::Limit("source count"));
        }
        if self.components.len() + self.queries.len() > limits.max_components {
            return Err(ReleaseBuildError::Limit("component count"));
        }
        if self.packets.len() > limits.max_rule_packets {
            return Err(ReleaseBuildError::Limit("rule packet count"));
        }
        if self.queries.len() > limits.release.max_query_sets {
            return Err(ReleaseBuildError::Limit("query sets"));
        }
        Ok(())
    }

    fn add_source(
        &mut self,
        paths: &mut BTreeMap<String, usize>,
        path: &str,
        role: SourceRole,
        encoding: SourceEncoding,
        limits: ReleaseBuildLimits,
    ) -> Result<usize> {
        if !valid_path(path) {
            return Err(ReleaseBuildError::Invalid(
                "source must be a portable relative path",
            ));
        }
        if let Some(&position) = paths.get(path) {
            let spec = &self.sources[position];
            // A shared names file is a single input, not multiple decode work.
            if role == SourceRole::Names && spec.role == role && spec.encoding == encoding {
                return Ok(position);
            }
            return Err(ReleaseBuildError::Invalid(
                "source path used more than once",
            ));
        }
        if self.sources.len() == limits.max_sources {
            return Err(ReleaseBuildError::Limit("source count"));
        }
        let position = self.sources.len();
        paths.insert(path.to_owned(), position);
        self.sources.push(ReleaseSourceSpec {
            path: path.to_owned(),
            encoding,
            role,
        });
        Ok(position)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ReleaseBuildLimits {
    pub manifest: ManifestLimits,
    pub release: OwnedReleaseLimits,
    pub max_sources: usize,
    pub max_components: usize,
    pub max_rule_packets: usize,
    /// Charge program text for every membership, even when files are shared.
    pub max_expanded_rule_bytes: usize,
    pub max_encoded_bytes: usize,
    pub max_decoded_bytes: usize,
}
impl Default for ReleaseBuildLimits {
    fn default() -> Self {
        Self {
            manifest: ManifestLimits::default(),
            release: OwnedReleaseLimits::default(),
            max_sources: 4096,
            max_components: 128,
            max_rule_packets: 128,
            max_expanded_rule_bytes: 16 * 1024 * 1024,
            max_encoded_bytes: 64 * 1024 * 1024,
            max_decoded_bytes: 64 * 1024 * 1024,
        }
    }
}
impl ReleaseBuildLimits {
    fn validate(self) -> Result<()> {
        let hard = Self::default();
        if self.max_sources == 0
            || self.max_sources > hard.max_sources
            || self.max_components == 0
            || self.max_components > hard.max_components
            || self.max_rule_packets == 0
            || self.max_rule_packets > hard.max_rule_packets
            || self.max_expanded_rule_bytes == 0
            || self.max_expanded_rule_bytes > hard.max_expanded_rule_bytes
            || self.max_encoded_bytes == 0
            || self.max_encoded_bytes > hard.max_encoded_bytes
            || self.max_decoded_bytes == 0
            || self.max_decoded_bytes > hard.max_decoded_bytes
        {
            return Err(ReleaseBuildError::Invalid("invalid release source limits"));
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReleaseBuildError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error("invalid release manifest/source: {0}")]
    Invalid(&'static str),
    #[error("release source exceeds {0}")]
    Limit(&'static str),
    #[error("cannot decode JSON source {path}: {source}")]
    Json {
        path: String,
        source: serde_json::Error,
    },
    #[error("cannot decompress source {path}: {source}")]
    Compression { path: String, source: io::Error },
    #[error("source {path} is not UTF-8: {source}")]
    Utf8 {
        path: String,
        source: std::str::Utf8Error,
    },
    #[error("invalid source manifest {path}: {source}")]
    SourceManifest { path: String, source: ManifestError },
    #[error("invalid rule packet {path}: {source}")]
    Packet {
        path: String,
        source: crate::PacketError,
    },
    #[error(transparent)]
    Assembly(#[from] OwnedReleaseError),
}
type Result<T> = std::result::Result<T, ReleaseBuildError>;

/// Paths are portable source labels here. The filesystem host must additionally
/// resolve them under the manifest directory and enforce bounded regular reads.
fn valid_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1024
        && !value
            .chars()
            .any(|c| c == '\\' || c == ':' || c.is_control())
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

pub fn parse_release_manifest(text: &str, limits: ReleaseBuildLimits) -> Result<ReleaseManifest> {
    limits.validate()?;
    let document = parse_document(text, limits.manifest)?;
    if document.roots.len() != 1 {
        return Err(ReleaseBuildError::Invalid("expected one release root"));
    }
    let root = &document.nodes[document.roots[0]];
    if root.name != "release"
        || !root.arguments.is_empty()
        || root.properties.keys().any(|name| name != "expected_input")
    {
        return Err(ReleaseBuildError::Invalid("invalid release root fields"));
    }
    let expected_input = root
        .properties
        .get("expected_input")
        .map(|value| {
            value
                .parse()
                .map_err(|_| ReleaseBuildError::Invalid("invalid expected input digest"))
        })
        .transpose()?;
    let mut output = ReleaseManifest {
        sources: vec![],
        components: BTreeMap::new(),
        queries: vec![],
        packets: vec![],
        rules: BTreeSet::new(),
        expected_input,
    };
    let mut paths = BTreeMap::new();
    let mut queries = BTreeSet::new();
    for &id in &root.children {
        let node = &document.nodes[id];
        if matches!(node.name.as_str(), "rule-packet" | "rule-source") {
            let packet = node.name == "rule-packet";
            if !node.children.is_empty()
                || node.arguments.len() != 1
                || (packet
                    && (node.properties.len() != 1 || !node.properties.contains_key("names")))
                || (!packet && !node.properties.is_empty())
            {
                return Err(ReleaseBuildError::Invalid(
                    "invalid rule source declaration fields",
                ));
            }
            let position = output.add_source(
                &mut paths,
                &node.arguments[0],
                if packet {
                    SourceRole::Packet
                } else {
                    SourceRole::Rule
                },
                SourceEncoding::Plain,
                limits,
            )?;
            if packet {
                let names = output.add_source(
                    &mut paths,
                    &node.properties["names"],
                    SourceRole::Names,
                    SourceEncoding::Plain,
                    limits,
                )?;
                output.packets.push(RulePacket {
                    packet: position,
                    names,
                });
            } else {
                output.rules.insert(position);
            }
            output.validate_counts(limits)?;
            continue;
        }
        let query = node.name == "queries";
        if !node.children.is_empty()
            || node.arguments.len() != 1
            || node
                .properties
                .keys()
                .any(|name| name != "encoding" && !(query && name == "source"))
        {
            return Err(ReleaseBuildError::Invalid(
                "invalid source declaration fields",
            ));
        }
        let path = if query {
            node.properties
                .get("source")
                .ok_or(ReleaseBuildError::Invalid("queries require a source"))?
        } else {
            &node.arguments[0]
        };
        let encoding = match node
            .properties
            .get("encoding")
            .map(String::as_str)
            .unwrap_or("plain")
        {
            "plain" => SourceEncoding::Plain,
            "gzip" => SourceEncoding::Gzip,
            _ => return Err(ReleaseBuildError::Invalid("unknown source encoding")),
        };
        let position =
            output.add_source(&mut paths, path, SourceRole::Constituent, encoding, limits)?;
        if query {
            let name = &node.arguments[0];
            if name.is_empty()
                || name.len() > 64
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
                || !queries.insert(name.as_str())
            {
                return Err(ReleaseBuildError::Invalid(
                    "invalid or duplicate query set name",
                ));
            }
            let name = OwnedDefinitionKey::new(name)
                .map_err(|_| ReleaseBuildError::Invalid("invalid query set key"))?;
            output.queries.push((name, position));
        } else {
            let component = Component::parse(&node.name)
                .ok_or(ReleaseBuildError::Invalid("unknown source component"))?;
            if output.components.insert(component, position).is_some() {
                return Err(ReleaseBuildError::Invalid("duplicate source component"));
            }
        }
        output.validate_counts(limits)?;
    }
    for component in [
        Component::Registry,
        Component::Schema,
        Component::Rules,
        Component::Routing,
        Component::Mapping,
        Component::Roles,
        Component::Normalization,
        Component::Rewards,
        Component::Items,
        Component::ItemSource,
    ] {
        if !output.components.contains_key(&component) {
            return Err(ReleaseBuildError::Invalid(
                "missing required source component",
            ));
        }
    }
    let support = [
        Component::Stages,
        Component::Preparation,
        Component::Inputs,
        Component::Receiving,
    ];
    let some_support = support
        .iter()
        .any(|key| output.components.contains_key(key))
        || output.components.contains_key(&Component::Outputs);
    if some_support
        && (!output.components.contains_key(&Component::Metrics)
            || support
                .iter()
                .any(|key| !output.components.contains_key(key)))
    {
        return Err(ReleaseBuildError::Invalid(
            "incomplete support/evaluation component group",
        ));
    }
    if output.packets.is_empty() && !output.rules.is_empty() {
        return Err(ReleaseBuildError::Invalid(
            "rule sources require a rule packet",
        ));
    }
    Ok(output)
}

#[derive(Clone, Copy, Debug)]
pub struct ReleaseSource<'a> {
    pub path: &'a str,
    pub bytes: &'a [u8],
}

pub struct BuiltRelease {
    release: StagedOwnedRelease,
    encoded_bytes: usize,
    decoded_bytes: usize,
}
impl BuiltRelease {
    pub fn release(&self) -> &StagedOwnedRelease {
        &self.release
    }
    pub fn encoded_bytes(&self) -> usize {
        self.encoded_bytes
    }
    pub fn decoded_bytes(&self) -> usize {
        self.decoded_bytes
    }
}

struct Decoder<'a> {
    manifest: &'a ReleaseManifest,
    decoded: Vec<Cow<'a, [u8]>>,
    decoded_bytes: usize,
}
impl<'a> Decoder<'a> {
    fn new(
        manifest: &'a ReleaseManifest,
        sources: BTreeMap<&str, &'a [u8]>,
        max_decoded_bytes: usize,
    ) -> Result<Self> {
        let mut decoded = Vec::with_capacity(manifest.sources.len());
        let mut left = max_decoded_bytes;
        for spec in &manifest.sources {
            let bytes = sources[spec.path.as_str()];
            let value = match spec.encoding {
                SourceEncoding::Plain => Cow::Borrowed(bytes),
                SourceEncoding::Gzip => {
                    let mut decoder = flate2::bufread::GzDecoder::new(bytes);
                    let mut output = Vec::new();
                    decoder
                        .by_ref()
                        .take(left as u64 + 1)
                        .read_to_end(&mut output)
                        .map_err(|source| ReleaseBuildError::Compression {
                            path: spec.path.clone(),
                            source,
                        })?;
                    if output.len() > left {
                        return Err(ReleaseBuildError::Limit("decoded bytes"));
                    }
                    if !decoder.get_ref().is_empty() {
                        return Err(ReleaseBuildError::Invalid(
                            "gzip source contains trailing bytes or another member",
                        ));
                    }
                    Cow::Owned(output)
                }
            };
            left = left
                .checked_sub(value.len())
                .ok_or(ReleaseBuildError::Limit("decoded bytes"))?;
            decoded.push(value);
        }
        Ok(Self {
            manifest,
            decoded,
            decoded_bytes: max_decoded_bytes - left,
        })
    }

    fn source<T: DeserializeOwned>(&self, position: usize) -> Result<T> {
        serde_json::from_slice(&self.decoded[position]).map_err(|source| ReleaseBuildError::Json {
            path: self.manifest.sources[position].path.clone(),
            source,
        })
    }

    fn text(&self, position: usize) -> Result<&str> {
        std::str::from_utf8(&self.decoded[position]).map_err(|source| ReleaseBuildError::Utf8 {
            path: self.manifest.sources[position].path.clone(),
            source,
        })
    }
    fn required<T: DeserializeOwned>(&self, component: Component) -> Result<T> {
        self.source(self.manifest.components[&component])
    }
    fn optional<T: DeserializeOwned>(&self, component: Component) -> Result<Option<T>> {
        self.manifest
            .components
            .get(&component)
            .copied()
            .map(|position| self.source(position))
            .transpose()
    }
}

/// Decode exact final constituents and run the single existing release checker.
/// All declared files must be supplied exactly once; extras and fallbacks fail.
pub fn build_release(
    manifest: &ReleaseManifest,
    sources: &[ReleaseSource<'_>],
    limits: ReleaseBuildLimits,
) -> Result<BuiltRelease> {
    limits.validate()?;
    manifest.validate_counts(limits)?;
    if sources.len() > limits.max_sources || manifest.sources.len() > limits.max_sources {
        return Err(ReleaseBuildError::Limit("source count"));
    }
    if sources.len() != manifest.sources.len() {
        return Err(ReleaseBuildError::Invalid("source inventory differs"));
    }
    let mut encoded_bytes = 0usize;
    let mut supplied = BTreeMap::new();
    for source in sources {
        if !valid_path(source.path) {
            return Err(ReleaseBuildError::Invalid("invalid supplied source path"));
        }
        encoded_bytes = encoded_bytes
            .checked_add(source.bytes.len())
            .ok_or(ReleaseBuildError::Limit("encoded bytes"))?;
        if encoded_bytes > limits.max_encoded_bytes {
            return Err(ReleaseBuildError::Limit("encoded bytes"));
        }
        if supplied.insert(source.path, source.bytes).is_some() {
            return Err(ReleaseBuildError::Invalid("duplicate supplied source"));
        }
    }
    if manifest
        .sources
        .iter()
        .any(|source| !supplied.contains_key(source.path.as_str()))
    {
        return Err(ReleaseBuildError::Invalid("missing declared source"));
    }
    let decoder = Decoder::new(manifest, supplied, limits.max_decoded_bytes)?;
    let evaluation = if manifest.components.contains_key(&Component::Metrics) {
        Some(OwnedReleaseEvaluationInput {
            metrics: decoder.required(Component::Metrics)?,
            support: if manifest.components.contains_key(&Component::Stages) {
                Some(OwnedReleaseSupportInput {
                    stages: decoder.required(Component::Stages)?,
                    preparation: decoder.required(Component::Preparation)?,
                    inputs: decoder.required(Component::Inputs)?,
                    receiving: decoder.required(Component::Receiving)?,
                    outputs: decoder.optional(Component::Outputs)?,
                })
            } else {
                None
            },
        })
    } else {
        None
    };
    let mut input = OwnedReleaseInput {
        schema_version: if evaluation.is_some() {
            OWNED_EVALUATION_RELEASE_VERSION
        } else {
            OWNED_RELEASE_VERSION
        },
        recipe: OwnedRecipeInput {
            schema_version: OWNED_RECIPE_VERSION,
            registry: decoder.required(Component::Registry)?,
            schema: decoder.required(Component::Schema)?,
            rules: decoder.required(Component::Rules)?,
            routing: decoder.required(Component::Routing)?,
        },
        mapping: decoder.required(Component::Mapping)?,
        roles: decoder.required(Component::Roles)?,
        normalization: decoder.required(Component::Normalization)?,
        rewards: decoder.required(Component::Rewards)?,
        items: decoder.required(Component::Items)?,
        item_source: decoder.required(Component::ItemSource)?,
        tree: decoder.optional(Component::Tree)?,
        evaluation,
        query_sets: manifest
            .queries
            .iter()
            .map(|(name, position)| {
                Ok(NamedQuerySet {
                    name: name.clone(),
                    queries: decoder.source(*position)?,
                })
            })
            .collect::<Result<_>>()?,
        provenance: decoder.optional(Component::Provenance)?.unwrap_or_default(),
    };
    // Authenticate original dependencies before any source program is lowered.
    // This checks no executable rules or evaluation artifacts; complete assembly
    // below remains mandatory after composition against this exact schema.
    let (registry, schema, authored_imports) =
        check_release_source_dependencies(&input, limits.release)?.into_parts()?;
    rules::compose(manifest, &decoder, &schema, &mut input.recipe.rules, limits)?;
    // Content was captured from the original checked owners before composition.
    // Construct current dependencies; evaluation inputs retain their supplied
    // commitments and must independently pass complete assembly below.
    let imports = authored_imports.into_input(&registry, &schema, limits.release)?;
    input.mapping = imports.mapping;
    input.roles = imports.roles;
    input.normalization = imports.normalization;
    input.rewards = imports.rewards;
    input.items = imports.items;
    input.item_source = imports.item_source;
    input.tree = imports.tree;
    let decoded_bytes = decoder.decoded_bytes;
    let release = assemble_owned_release(input, limits.release)?;
    if manifest
        .expected_input
        .is_some_and(|expected| expected != release.receipt().input)
    {
        return Err(ReleaseBuildError::Invalid(
            "assembled input identity differs from expected_input",
        ));
    }
    Ok(BuiltRelease {
        release,
        encoded_bytes,
        decoded_bytes,
    })
}
