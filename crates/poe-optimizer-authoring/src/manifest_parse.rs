//! KDL v2 decoding for maintained offline authoring manifests.
//!
//! The official parser implements the grammar. The source byte cap is checked
//! before parsing; all other limits constrain the retained semantic document
//! after parsing. They do not bound upstream parser recursion or allocations,
//! or the nesting of skipped comments and slash-dashed content. This is not an
//! untrusted upload parser. Imported builds and runtime packs use other formats.
//!
//! We retain a flat tree of string entries, with no includes, IO or implicit
//! property replacement. Conversion and successful DOM cleanup are iterative.
use crate::SymbolError;
use kdl::{KdlDocument, KdlNode, KdlValue};
use std::{collections::BTreeMap, fmt};

pub const MAX_MANIFEST_DEPTH: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestLimits {
    /// Checked before calling the upstream parser. This is an input-size bound,
    /// not a guarantee about that parser's recursion or temporary allocations.
    pub max_source_bytes: usize,
    /// Retained nodes after parsing; excludes slash-dashed content.
    pub max_nodes: usize,
    /// Combined retained positional arguments and properties across all nodes.
    pub max_entries: usize,
    /// Retained node nesting after parsing, counting roots as one. Does not
    /// constrain parser recursion, comments or slash-dashed child blocks.
    pub max_depth: usize,
    /// Combined decoded node names, property names and string values.
    pub max_text_bytes: usize,
}

impl Default for ManifestLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 256 * 1024,
            max_nodes: 8192,
            max_entries: 32768,
            max_depth: 16,
            max_text_bytes: 256 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestErrorKind {
    InvalidLimits,
    SourceBytes,
    Nodes,
    Entries,
    Depth,
    TextBytes,
    Syntax,
    Shape,
    DuplicateProperty,
    NamespaceMismatch,
    Symbols,
    Symbol(SymbolError),
    UnknownSymbol,
    UnsupportedOwner,
    InvalidKey,
}

/// `node` is the retained node's flat index, when known. Diagnostics never
/// copy source-controlled names, values, or upstream error payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestError {
    pub kind: ManifestErrorKind,
    pub node: Option<usize>,
    pub message: &'static str,
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} at node {:?}: {}",
            self.kind, self.node, self.message
        )
    }
}

impl std::error::Error for ManifestError {}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Document {
    pub(crate) nodes: Vec<Node>,
    pub(crate) roots: Vec<usize>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Node {
    pub(crate) name: String,
    pub(crate) arguments: Vec<String>,
    pub(crate) properties: BTreeMap<String, String>,
    pub(crate) children: Vec<usize>,
}

fn error(kind: ManifestErrorKind, node: Option<usize>, message: &'static str) -> ManifestError {
    ManifestError {
        kind,
        node,
        message,
    }
}

fn check_limits(source: &str, limits: ManifestLimits) -> Result<(), ManifestError> {
    if limits.max_source_bytes == 0
        || limits.max_nodes == 0
        || limits.max_entries == 0
        || limits.max_depth == 0
        || limits.max_depth > MAX_MANIFEST_DEPTH
        || limits.max_text_bytes == 0
    {
        return Err(error(
            ManifestErrorKind::InvalidLimits,
            None,
            "manifest limits must be positive and depth at most 64",
        ));
    }
    if source.len() > limits.max_source_bytes {
        return Err(error(
            ManifestErrorKind::SourceBytes,
            None,
            "manifest source byte limit exceeded",
        ));
    }
    Ok(())
}

fn charge_text(
    used: &mut usize,
    count: usize,
    maximum: usize,
    node: usize,
) -> Result<(), ManifestError> {
    let next = used
        .checked_add(count)
        .filter(|next| *next <= maximum)
        .ok_or_else(|| {
            error(
                ManifestErrorKind::TextBytes,
                Some(node),
                "manifest decoded text byte limit exceeded",
            )
        })?;
    *used = next;
    Ok(())
}

/// Decode a complete maintained manifest with the standard parser, then enforce
/// the semantic limits. These limits are not upstream parsing resource guards.
pub(crate) fn parse_document(
    source: &str,
    limits: ManifestLimits,
) -> Result<Document, ManifestError> {
    check_limits(source, limits)?;
    let parsed = KdlDocument::parse_v2(source)
        .map_err(|_| error(ManifestErrorKind::Syntax, None, "invalid KDL v2 syntax"))?;
    let result = flatten_document(&parsed, limits);
    drop_document(parsed);
    result
}

struct Frame<'a> {
    nodes: std::slice::Iter<'a, KdlNode>,
    parent: Option<usize>,
}

fn flatten_document(
    parsed: &KdlDocument,
    limits: ManifestLimits,
) -> Result<Document, ManifestError> {
    let mut document = Document {
        nodes: Vec::new(),
        roots: Vec::new(),
    };
    let mut stack = vec![Frame {
        nodes: parsed.nodes().iter(),
        parent: None,
    }];
    let mut entries = 0_usize;
    let mut text_bytes = 0_usize;
    while let Some(frame) = stack.last_mut() {
        let Some(input) = frame.nodes.next() else {
            stack.pop();
            continue;
        };
        let parent = frame.parent;
        let id = document.nodes.len();
        if id == limits.max_nodes {
            return Err(error(
                ManifestErrorKind::Nodes,
                Some(id),
                "manifest node limit exceeded",
            ));
        }
        if stack.len() > limits.max_depth {
            return Err(error(
                ManifestErrorKind::Depth,
                Some(id),
                "manifest node depth limit exceeded",
            ));
        }
        if input.ty().is_some() {
            return Err(error(
                ManifestErrorKind::Shape,
                Some(id),
                "manifest node types are unsupported",
            ));
        }
        let name = input.name().value();
        charge_text(&mut text_bytes, name.len(), limits.max_text_bytes, id)?;
        let mut node = Node {
            name: name.to_owned(),
            arguments: Vec::new(),
            properties: BTreeMap::new(),
            children: Vec::new(),
        };
        for entry in input.entries() {
            if entries == limits.max_entries {
                return Err(error(
                    ManifestErrorKind::Entries,
                    Some(id),
                    "manifest entry limit exceeded",
                ));
            }
            entries += 1;
            if entry.ty().is_some() {
                return Err(error(
                    ManifestErrorKind::Shape,
                    Some(id),
                    "manifest entry types are unsupported",
                ));
            }
            let KdlValue::String(value) = entry.value() else {
                return Err(error(
                    ManifestErrorKind::Shape,
                    Some(id),
                    "manifest entries must be strings",
                ));
            };
            if let Some(name) = entry.name() {
                let name = name.value();
                if node.properties.contains_key(name) {
                    return Err(error(
                        ManifestErrorKind::DuplicateProperty,
                        Some(id),
                        "duplicate manifest property",
                    ));
                }
                charge_text(&mut text_bytes, name.len(), limits.max_text_bytes, id)?;
                charge_text(&mut text_bytes, value.len(), limits.max_text_bytes, id)?;
                node.properties.insert(name.to_owned(), value.clone());
            } else {
                charge_text(&mut text_bytes, value.len(), limits.max_text_bytes, id)?;
                node.arguments.push(value.clone());
            }
        }
        document.nodes.push(node);
        if let Some(parent) = parent {
            document.nodes[parent].children.push(id);
        } else {
            document.roots.push(id);
        }
        if let Some(children) = input.children() {
            stack.push(Frame {
                nodes: children.nodes().iter(),
                parent: Some(id),
            });
        }
    }
    Ok(document)
}

// Detach child documents before dropping their nodes, including when semantic
// validation failed. This avoids adding recursive cleanup after a successful
// upstream parse; it does not alter or guard the upstream parser itself.
fn drop_document(document: KdlDocument) {
    let mut stack = vec![document.into_iter()];
    while let Some(nodes) = stack.last_mut() {
        if let Some(mut node) = nodes.next() {
            if let Some(children) = node.children_mut().take() {
                stack.push(children.into_iter());
            }
        } else {
            stack.pop();
        }
    }
}
