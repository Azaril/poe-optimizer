//! Bounded KDL v2 decoding for offline manifests.
//!
//! The upstream lexer and event reader implement the grammar. We retain a flat
//! tree of string entries, with no includes, IO, implicit property replacement,
//! or recursive DOM construction. A lexer preflight also bounds child blocks
//! that the reader would skip because they were slash-dashed.
use crate::SymbolError;
use just_kdl::{
    dom::{Event, Value},
    lexer::{Lexer, Token},
    reader::Reader,
};
use std::{collections::BTreeMap, fmt};

pub const MAX_MANIFEST_DEPTH: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestLimits {
    /// Checked before constructing either upstream parser. This also bounds
    /// temporary storage for an individual decoded token or multiline string.
    pub max_source_bytes: usize,
    /// Retained nodes; slash-dashed content is still covered by source/depth caps.
    pub max_nodes: usize,
    /// Combined retained positional arguments and properties across all nodes.
    pub max_entries: usize,
    /// Retained node nesting, counting roots as one. The same bound applies to
    /// all lexical child blocks, including slash-dashed blocks.
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

fn check_lexical_depth(source: &str, maximum: usize) -> Result<(), ManifestError> {
    let mut lexer = Lexer::new(source.as_bytes());
    let mut depth = 0_usize;
    loop {
        // The upstream lexer still validates skipped strings and numbers. No
        // raw-character scan can confuse braces in comments, escapes or strings.
        let (token, _) = lexer.next_token(true);
        match token.map_err(|_| error(ManifestErrorKind::Syntax, None, "invalid KDL syntax"))? {
            Token::OpenCurly => {
                if depth == maximum {
                    return Err(error(
                        ManifestErrorKind::Depth,
                        None,
                        "manifest lexical child-block depth limit exceeded",
                    ));
                }
                depth += 1;
            }
            Token::CloseCurly => {
                depth = depth.checked_sub(1).ok_or_else(|| {
                    error(
                        ManifestErrorKind::Syntax,
                        None,
                        "unmatched KDL child-block close",
                    )
                })?;
            }
            Token::Eof => {
                return if depth == 0 {
                    Ok(())
                } else {
                    Err(error(
                        ManifestErrorKind::Syntax,
                        None,
                        "unclosed KDL child block",
                    ))
                };
            }
            _ => {}
        }
    }
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

struct Frame {
    node: usize,
    children: bool,
}

/// Decode a complete document. All downstream semantics operate on this flat
/// representation; dropping a rejected deeply nested input cannot recurse.
pub(crate) fn parse_document(
    source: &str,
    limits: ManifestLimits,
) -> Result<Document, ManifestError> {
    check_limits(source, limits)?;
    check_lexical_depth(source, limits.max_depth)?;
    let mut document = Document {
        nodes: Vec::new(),
        roots: Vec::new(),
    };
    let mut stack: Vec<Frame> = Vec::new();
    let mut entries = 0_usize;
    let mut text_bytes = 0_usize;
    for event in Reader::new(source.as_bytes()) {
        let (event, _) = event.map_err(|_| {
            error(
                ManifestErrorKind::Syntax,
                stack.last().map(|frame| frame.node),
                "invalid KDL syntax",
            )
        })?;
        match event {
            Event::Node { r#type, name } => {
                let id = document.nodes.len();
                if r#type.is_some() {
                    return Err(error(
                        ManifestErrorKind::Shape,
                        Some(id),
                        "manifest node types are unsupported",
                    ));
                }
                if id == limits.max_nodes {
                    return Err(error(
                        ManifestErrorKind::Nodes,
                        Some(id),
                        "manifest node limit exceeded",
                    ));
                }
                if stack.len() == limits.max_depth {
                    return Err(error(
                        ManifestErrorKind::Depth,
                        Some(id),
                        "manifest node depth limit exceeded",
                    ));
                }
                if stack.last().is_some_and(|frame| !frame.children) {
                    return Err(error(
                        ManifestErrorKind::Syntax,
                        Some(id),
                        "node outside a child block",
                    ));
                }
                charge_text(&mut text_bytes, name.len(), limits.max_text_bytes, id)?;
                document.nodes.push(Node {
                    name: name.to_string(),
                    arguments: Vec::new(),
                    properties: BTreeMap::new(),
                    children: Vec::new(),
                });
                if let Some(parent) = stack.last() {
                    document.nodes[parent.node].children.push(id);
                } else {
                    document.roots.push(id);
                }
                stack.push(Frame {
                    node: id,
                    children: false,
                });
            }
            Event::Entry(entry) => {
                let frame = stack
                    .last()
                    .filter(|frame| !frame.children)
                    .ok_or_else(|| {
                        error(
                            ManifestErrorKind::Syntax,
                            None,
                            "entry outside a node's entries",
                        )
                    })?;
                let id = frame.node;
                if entries == limits.max_entries {
                    return Err(error(
                        ManifestErrorKind::Entries,
                        Some(id),
                        "manifest entry limit exceeded",
                    ));
                }
                entries += 1;
                if entry.r#type.is_some() {
                    return Err(error(
                        ManifestErrorKind::Shape,
                        Some(id),
                        "manifest entry types are unsupported",
                    ));
                }
                let Value::String(value) = entry.value else {
                    return Err(error(
                        ManifestErrorKind::Shape,
                        Some(id),
                        "manifest entries must be strings",
                    ));
                };
                let node = &mut document.nodes[id];
                if let Some(name) = entry.name {
                    if node.properties.contains_key(name.as_str()) {
                        return Err(error(
                            ManifestErrorKind::DuplicateProperty,
                            Some(id),
                            "duplicate manifest property",
                        ));
                    }
                    charge_text(&mut text_bytes, name.len(), limits.max_text_bytes, id)?;
                    charge_text(&mut text_bytes, value.len(), limits.max_text_bytes, id)?;
                    node.properties.insert(name.to_string(), value.to_string());
                } else {
                    charge_text(&mut text_bytes, value.len(), limits.max_text_bytes, id)?;
                    node.arguments.push(value.to_string());
                }
            }
            Event::Children => {
                let frame = stack
                    .last_mut()
                    .filter(|frame| !frame.children)
                    .ok_or_else(|| {
                        error(
                            ManifestErrorKind::Syntax,
                            None,
                            "child block outside a node",
                        )
                    })?;
                frame.children = true;
            }
            Event::End => {
                stack.pop().ok_or_else(|| {
                    error(ManifestErrorKind::Syntax, None, "unmatched KDL node end")
                })?;
            }
        }
    }
    if !stack.is_empty() {
        return Err(error(
            ManifestErrorKind::Syntax,
            stack.last().map(|frame| frame.node),
            "unfinished KDL node",
        ));
    }
    Ok(document)
}
