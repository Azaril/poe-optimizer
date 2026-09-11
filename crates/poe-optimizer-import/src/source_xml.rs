//! Source slices decoded with the pinned PoB XML reader's five named entities.
//! No XML attribute whitespace normalization or recursive entity decoding occurs.
use roxmltree::Node;
use serde::Serialize;
use std::{error::Error, fmt, ops::Range};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceXmlError {
    pub byte_offset: usize,
    pub reason: String,
}
impl fmt::Display for SourceXmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "source XML at byte {}: {}",
            self.byte_offset, self.reason
        )
    }
}
impl Error for SourceXmlError {}
pub(crate) fn invalid(at: usize, reason: impl Into<String>) -> SourceXmlError {
    SourceXmlError {
        byte_offset: at,
        reason: reason.into(),
    }
}

/// Immutable exact bytes and one-pass named-entity decoding from the same source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceText<'input> {
    range: Range<usize>,
    raw: &'input str,
    decoded: String,
}
impl<'input> SourceText<'input> {
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
    pub fn raw(&self) -> &'input str {
        self.raw
    }
    pub fn decoded(&self) -> &str {
        &self.decoded
    }
    pub(crate) fn from_range(
        xml: &'input str,
        range: Range<usize>,
        literal: bool,
    ) -> Result<Self, SourceXmlError> {
        let raw = xml
            .get(range.clone())
            .ok_or_else(|| invalid(range.start, "invalid source range"))?;
        let decoded = if literal {
            raw.to_owned()
        } else {
            decode_named_entities_at(raw, range.start)?
        };
        Ok(Self {
            range,
            raw,
            decoded,
        })
    }
}
/// Decode exactly once. Unknown/numeric entities reject rather than copying PoB's deletion.
pub fn decode_named_entities(content: &str) -> Result<String, SourceXmlError> {
    decode_named_entities_at(content, 0)
}
fn decode_named_entities_at(content: &str, offset: usize) -> Result<String, SourceXmlError> {
    let mut decoded = String::with_capacity(content.len());
    let mut rest = content;
    while let Some((prefix, entity)) = rest.split_once('&') {
        decoded.push_str(prefix);
        let at = offset + content.len() - entity.len() - 1;
        let (name, next) = entity
            .split_once(';')
            .ok_or_else(|| invalid(at, "unterminated XML entity"))?;
        decoded.push(match name {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "apos" => '\'',
            "quot" => '"',
            _ => {
                return Err(invalid(
                    at,
                    "only PoB-compatible named XML entities are admitted",
                ));
            }
        });
        rest = next;
    }
    decoded.push_str(rest);
    Ok(decoded)
}
/// Read original attribute bytes, not roxmltree's whitespace-normalized value.
pub fn attribute<'input>(
    node: Node<'_, 'input>,
    name: &str,
) -> Result<Option<SourceText<'input>>, SourceXmlError> {
    node.attributes()
        .find(|a| a.name() == name)
        .map(|a| {
            if a.namespace().is_some() {
                return Err(invalid(
                    a.range().start,
                    "namespaced attributes are unsupported",
                ));
            }
            SourceText::from_range(node.document().input_text(), a.range_value(), false)
        })
        .transpose()
}
/// A complete unsplit ordinary-text or CDATA payload, retaining authored whitespace.
/// Callers that model PoB's XML table must separately apply its ordinary-text trimming.
pub fn element_text<'input>(node: Node<'_, 'input>) -> Result<SourceText<'input>, SourceXmlError> {
    if node.children().any(|n| !n.is_text()) || node.children().count() > 1 {
        return Err(invalid(
            node.range().start,
            "element must contain one unsplit text payload",
        ));
    }
    let xml = node.document().input_text();
    let source = &xml[node.range()];
    let mut quote = None;
    let mut start = None;
    for (offset, byte) in source.bytes().enumerate() {
        match quote {
            Some(value) if value == byte => quote = None,
            Some(_) => {}
            None if matches!(byte, b'\'' | b'"') => quote = Some(byte),
            None if byte == b'>' => {
                start = Some(offset + 1);
                break;
            }
            _ => {}
        }
    }
    let start = start.ok_or_else(|| invalid(node.range().start, "missing opening tag"))?;
    if source[..start].ends_with("/>") {
        return SourceText::from_range(
            xml,
            node.range().start + start..node.range().start + start,
            true,
        );
    }
    let end = source
        .rfind("</")
        .ok_or_else(|| invalid(node.range().start, "missing closing tag"))?;
    let content = source
        .get(start..end)
        .ok_or_else(|| invalid(node.range().start, "invalid text range"))?;
    let (start, end, literal) = if let Some(text) = content
        .strip_prefix("<![CDATA[")
        .and_then(|v| v.strip_suffix("]]>"))
    {
        if text.contains("]]>") || text.contains("<!--") {
            return Err(invalid(
                node.range().start + start,
                "split/comment-containing CDATA is unsupported",
            ));
        }
        (start + 9, end - 3, true)
    } else {
        if content.contains('<') {
            return Err(invalid(
                node.range().start + start,
                "unsupported text fragments",
            ));
        }
        (start, end, false)
    };
    SourceText::from_range(
        xml,
        node.range().start + start..node.range().start + end,
        literal,
    )
}

/// Direct lexical fragments. Child element bytes are borrowed but omitted from
/// serialization, avoiding repeated whole subtrees in ancestor evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceContentKind {
    Text,
    Cdata,
    Comment,
    ProcessingInstruction,
    Element,
}
#[derive(Debug, Clone, Serialize)]
pub struct SourceContentFragment<'input> {
    kind: SourceContentKind,
    range: Range<usize>,
    #[serde(skip)]
    raw: &'input str,
    #[serde(skip_serializing_if = "Option::is_none")]
    text_source: Option<&'input str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    child_index: Option<usize>,
}
impl<'input> SourceContentFragment<'input> {
    pub fn kind(&self) -> SourceContentKind {
        self.kind
    }
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
    pub fn raw(&self) -> &'input str {
        self.raw
    }
    pub fn child_index(&self) -> Option<usize> {
        self.child_index
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PobTextKind {
    Ordinary,
    Cdata,
}
/// Original XML.lua array entries, before any application Load method runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PobContentEntry {
    Text {
        text_kind: PobTextKind,
        text: String,
        fragment_indices: Vec<usize>,
    },
    Element {
        child_index: usize,
    },
}
#[derive(Debug, Clone, Serialize)]
pub struct SourceContent<'input> {
    fragments: Vec<SourceContentFragment<'input>>,
    consumed: Vec<PobContentEntry>,
}
impl<'input> SourceContent<'input> {
    pub fn fragments(&self) -> &[SourceContentFragment<'input>] {
        &self.fragments
    }
    pub fn consumed(&self) -> &[PobContentEntry] {
        &self.consumed
    }
}
fn delimited_end(xml: &str, at: usize, delimiter: &str) -> Result<usize, SourceXmlError> {
    xml[at..]
        .find(delimiter)
        .map(|n| at + n + delimiter.len())
        .ok_or_else(|| invalid(at, "unterminated source XML construct"))
}
fn opening_end(xml: &str, at: usize) -> Result<usize, SourceXmlError> {
    let mut quote = None;
    for (offset, byte) in xml.as_bytes()[at..].iter().copied().enumerate() {
        match quote {
            Some(q) if q == byte => quote = None,
            Some(_) => {}
            None if matches!(byte, b'\'' | b'"') => quote = Some(byte),
            None if byte == b'>' => return Ok(at + offset + 1),
            None => {}
        }
    }
    Err(invalid(at, "unterminated source XML tag"))
}
/// The same bounded lexical subset as other source projections, with ordered
/// mixed text/CDATA and explicit unknown namespace contexts. Native gates stay
/// unchanged. Well-formedness and DTD rejection are the caller's responsibility.
pub(crate) fn validate_ordered_xml(xml: &str) -> Result<(), SourceXmlError> {
    validate_ordered_xml_impl(xml, false)
}
/// Item source additionally preserves prefixed namespace syntax as unknown
/// context. This does not widen the older projectors' namespace admission.
pub(crate) fn validate_ordered_xml_with_namespaces(xml: &str) -> Result<(), SourceXmlError> {
    validate_ordered_xml_impl(xml, true)
}
fn validate_ordered_xml_impl(xml: &str, preserve_namespaces: bool) -> Result<(), SourceXmlError> {
    if xml.starts_with('\u{feff}') {
        return Err(invalid(0, "a leading BOM becomes a PoB text entry"));
    }
    let mut at = 0;
    while at < xml.len() {
        if xml[at..].starts_with("<!--") {
            at = delimited_end(xml, at + 4, "-->")?;
        } else if xml[at..].starts_with("<![CDATA[") {
            let end = delimited_end(xml, at + 9, "]]>")?;
            stripped_comments(xml, at + 9..end - 3)?;
            at = end;
        } else if xml[at..].starts_with("<?") {
            let end = delimited_end(xml, at + 2, "?>")?;
            if xml[at + 2..end - 2].contains('>') {
                return Err(invalid(
                    at,
                    "processing instruction terminates early in PoB",
                ));
            }
            at = end;
        } else if xml[at..].starts_with("<!") {
            return Err(invalid(at, "source XML declarations are unsupported"));
        } else if xml.as_bytes()[at] == b'<' {
            let end = opening_end(xml, at)?;
            validate_source_tag(&xml[at..end], at, preserve_namespaces)?;
            at = end;
        } else {
            let end = xml[at..].find('<').map_or(xml.len(), |n| at + n);
            decode_named_entities_at(&xml[at..end], at)?;
            at = end;
        }
    }
    Ok(())
}
fn validate_source_tag(
    tag: &str,
    offset: usize,
    preserve_namespaces: bool,
) -> Result<(), SourceXmlError> {
    let bytes = tag.as_bytes();
    let mut at = if tag.starts_with("</") { 2 } else { 1 };
    let begin = at;
    while at < bytes.len() && (bytes[at].is_ascii_alphanumeric() || bytes[at] == b':') {
        at += 1;
    }
    if at == begin {
        return Err(invalid(offset + at, "unsupported source element name"));
    }
    if tag.starts_with("</") {
        if tag[at..].trim_ascii() != ">" {
            return Err(invalid(offset + at, "unsupported source closing tag"));
        }
        return Ok(());
    }
    loop {
        while at < bytes.len() && bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        if matches!(&tag[at..], ">" | "/>") {
            return Ok(());
        }
        let begin = at;
        // Namespaced attributes stay opaque source evidence, not plain fields.
        while at < bytes.len()
            && (bytes[at].is_ascii_alphanumeric() || (preserve_namespaces && bytes[at] == b':'))
        {
            at += 1;
        }
        if at == begin || bytes.get(at) != Some(&b'=') {
            return Err(invalid(
                offset + at,
                "attributes require ASCII names immediately followed by '='",
            ));
        }
        at += 1;
        let quote = *bytes
            .get(at)
            .filter(|b| matches!(b, b'\'' | b'"'))
            .ok_or_else(|| {
                invalid(
                    offset + at,
                    "attribute '=' must immediately precede its quote",
                )
            })?;
        at += 1;
        let value_start = at;
        while at < bytes.len() && bytes[at] != quote {
            if bytes[at] == b'>' {
                return Err(invalid(
                    offset + at,
                    "literal '>' in an attribute terminates PoB's tag early",
                ));
            }
            at += 1;
        }
        if at == bytes.len() {
            return Err(invalid(offset + at, "unterminated source attribute"));
        }
        decode_named_entities_at(&tag[value_start..at], offset + value_start)?;
        at += 1;
    }
}
fn stripped_comments(
    xml: &str,
    range: Range<usize>,
) -> Result<std::borrow::Cow<'_, str>, SourceXmlError> {
    let raw = &xml[range.clone()];
    if !raw.contains("<!--") {
        return Ok(std::borrow::Cow::Borrowed(raw));
    }
    let mut output = String::new();
    let mut at = range.start;
    while let Some(index) = xml[at..range.end].find("<!--") {
        let start = at + index;
        output.push_str(&xml[at..start]);
        let Some(end) = xml[start + 4..].find("-->").map(|n| start + 4 + n + 3) else {
            output.push_str(&xml[start..range.end]);
            return Ok(std::borrow::Cow::Owned(output));
        };
        if end > range.end {
            return Err(invalid(
                start,
                "global comment removal crosses a CDATA boundary",
            ));
        }
        at = end;
    }
    output.push_str(&xml[at..range.end]);
    if output.contains("]]>") {
        return Err(invalid(
            range.start,
            "comment removal creates a CDATA closing delimiter",
        ));
    }
    Ok(std::borrow::Cow::Owned(output))
}
/// Preserve direct raw fragments and the original XML.lua array-entry semantics.
/// Shared budgets bound allocation across every projected descendant.
pub fn ordered_content<'input>(
    node: Node<'_, 'input>,
    fragments_left: &mut usize,
    text_bytes_left: &mut usize,
) -> Result<SourceContent<'input>, SourceXmlError> {
    let xml = node.document().input_text();
    let start = opening_end(xml, node.range().start)?;
    let mut output = SourceContent {
        fragments: Vec::new(),
        consumed: Vec::new(),
    };
    if xml[node.range().start..start].ends_with("/>") {
        return Ok(output);
    }
    let end = node.range().start
        + xml[node.range()]
            .rfind("</")
            .ok_or_else(|| invalid(start, "missing source closing tag"))?;
    let children: Vec<_> = node.children().filter(Node::is_element).collect();
    let mut child_index = 0;
    let mut at = start;
    let mut pending = String::new();
    let mut pending_indices = Vec::new();
    while at < end {
        *fragments_left = fragments_left
            .checked_sub(1)
            .ok_or_else(|| invalid(at, "item source fragment limit exceeded"))?;
        let index = output.fragments.len();
        let (kind, next, child) = if children
            .get(child_index)
            .is_some_and(|child| child.range().start == at)
        {
            flush_ordinary(
                &mut output,
                &mut pending,
                &mut pending_indices,
                text_bytes_left,
                at,
            )?;
            output
                .consumed
                .push(PobContentEntry::Element { child_index });
            let next = children[child_index].range().end;
            child_index += 1;
            (SourceContentKind::Element, next, Some(child_index - 1))
        } else if xml[at..].starts_with("<!--") {
            pending_indices.push(index);
            (
                SourceContentKind::Comment,
                delimited_end(xml, at + 4, "-->")?,
                None,
            )
        } else if xml[at..].starts_with("<![CDATA[") {
            flush_ordinary(
                &mut output,
                &mut pending,
                &mut pending_indices,
                text_bytes_left,
                at,
            )?;
            let next = delimited_end(xml, at + 9, "]]>")?;
            let text = stripped_comments(xml, at + 9..next - 3)?;
            if !text.trim_ascii().is_empty() {
                *text_bytes_left = text_bytes_left
                    .checked_sub(text.len())
                    .ok_or_else(|| invalid(at, "item source decoded text limit exceeded"))?;
                output.consumed.push(PobContentEntry::Text {
                    text_kind: PobTextKind::Cdata,
                    text: text.into_owned(),
                    fragment_indices: vec![index],
                });
            }
            (SourceContentKind::Cdata, next, None)
        } else if xml[at..].starts_with("<?") {
            flush_ordinary(
                &mut output,
                &mut pending,
                &mut pending_indices,
                text_bytes_left,
                at,
            )?;
            (
                SourceContentKind::ProcessingInstruction,
                delimited_end(xml, at + 2, "?>")?,
                None,
            )
        } else {
            let next = xml[at..end].find('<').map_or(end, |n| at + n);
            if next == at {
                return Err(invalid(at, "unrecognized direct source content"));
            }
            if pending.len().saturating_add(next - at) > *text_bytes_left {
                return Err(invalid(at, "item source decoded text limit exceeded"));
            }
            pending.push_str(&xml[at..next]);
            pending_indices.push(index);
            (SourceContentKind::Text, next, None)
        };
        if next > end {
            return Err(invalid(at, "source fragment crosses its parent"));
        }
        output.fragments.push(SourceContentFragment {
            kind,
            range: at..next,
            raw: &xml[at..next],
            text_source: (kind != SourceContentKind::Element).then_some(&xml[at..next]),
            child_index: child,
        });
        at = next;
    }
    flush_ordinary(
        &mut output,
        &mut pending,
        &mut pending_indices,
        text_bytes_left,
        at,
    )?;
    Ok(output)
}
fn flush_ordinary(
    output: &mut SourceContent<'_>,
    pending: &mut String,
    indices: &mut Vec<usize>,
    budget: &mut usize,
    at: usize,
) -> Result<(), SourceXmlError> {
    let trimmed = pending.trim_ascii();
    if !trimmed.is_empty() {
        let text = decode_named_entities_at(trimmed, at)?;
        *budget = budget
            .checked_sub(text.len())
            .ok_or_else(|| invalid(at, "item source decoded text limit exceeded"))?;
        output.consumed.push(PobContentEntry::Text {
            text_kind: PobTextKind::Ordinary,
            text,
            fragment_indices: std::mem::take(indices),
        });
    } else {
        indices.clear();
    }
    pending.clear();
    Ok(())
}
