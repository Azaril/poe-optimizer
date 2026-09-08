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
