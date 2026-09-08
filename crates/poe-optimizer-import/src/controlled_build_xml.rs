//! Exact XML spans and strict source syntax for the lazy build projection.
use super::{BuildCatalogError, Result};
use roxmltree::{Document, Node, ParsingOptions};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

pub(super) fn fail(message: impl Into<String>) -> BuildCatalogError {
    BuildCatalogError::Source(message.into())
}
pub(super) fn document(xml: &str) -> Result<Document<'_>> {
    if xml.len() > crate::MAX_XML_BYTES {
        return Err(fail("build XML exceeds import byte bound"));
    }
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: crate::MAX_XML_NODES,
            entity_resolver: None,
        },
    )
    .map_err(|e| fail(e.to_string()))?;
    crate::xml_compat::validate_native_with_actor_inputs(&document)
        .map_err(|e| fail(e.to_string()))?;
    Ok(document)
}
pub(super) fn child<'a, 'i>(node: Node<'a, 'i>, name: &str) -> Result<Node<'a, 'i>> {
    let mut matches = node.children().filter(|n| n.has_tag_name(name));
    let first = matches
        .next()
        .ok_or_else(|| fail(format!("missing {name}")))?;
    if matches.next().is_some() {
        return Err(fail(format!("duplicate {name}")));
    }
    Ok(first)
}
pub(super) fn only(node: Node<'_, '_>, attrs: &[&str], children: &[&str]) -> Result<()> {
    if node.tag_name().namespace().is_some()
        || node
            .attributes()
            .any(|a| a.namespace().is_some() || !attrs.contains(&a.name()))
        || node
            .children()
            .filter(Node::is_element)
            .any(|n| n.tag_name().namespace().is_some() || !children.contains(&n.tag_name().name()))
        || node
            .children()
            .any(|n| n.is_text() && !n.text().unwrap_or("").trim_ascii().is_empty())
    {
        return Err(fail(format!(
            "unsupported {} structure",
            node.tag_name().name()
        )));
    }
    Ok(())
}
pub(super) fn fixed(node: Node<'_, '_>, values: &[(&str, &str)]) -> Result<()> {
    for (name, value) in values {
        if node.attribute(*name) != Some(*value) {
            return Err(fail(format!(
                "{}.{} must equal {value}",
                node.tag_name().name(),
                name
            )));
        }
    }
    Ok(())
}
pub(super) fn integer(text: &str) -> Result<u32> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(fail("expected unsigned integer"));
    }
    text.parse().map_err(|_| fail("integer exceeds u32"))
}
pub(super) fn ids(text: &str) -> Result<BTreeSet<u32>> {
    let mut result = BTreeSet::new();
    if !text.is_empty() {
        for text in text.split(',') {
            if !result.insert(integer(text)?) {
                return Err(fail("duplicate node ID"));
            }
        }
    }
    Ok(result)
}
pub(super) fn attribute_options(
    spec: Node<'_, '_>,
    allocated: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, u32>> {
    let containers: Vec<_> = spec
        .children()
        .filter(|n| n.has_tag_name("Overrides"))
        .collect();
    if containers.len() > 1 {
        return Err(fail("duplicate Overrides container"));
    }
    let mut choices = BTreeMap::new();
    if let Some(container) = containers.first() {
        only(*container, &[], &["AttributeOverride"])?;
        let entries: Vec<_> = container.children().filter(Node::is_element).collect();
        if entries.len() > 1 {
            return Err(fail("duplicate AttributeOverride"));
        }
        if let Some(entry) = entries.first() {
            only(*entry, &["strNodes", "dexNodes", "intNodes"], &[])?;
            for (name, index) in [("strNodes", 1), ("dexNodes", 2), ("intNodes", 3)] {
                let text = entry
                    .attribute(name)
                    .ok_or_else(|| fail("AttributeOverride requires all three lists"))?;
                for id in ids(text)? {
                    if !allocated.contains(&id) {
                        return Err(fail(
                            "dormant attribute overrides are outside this allocation scope",
                        ));
                    }
                    if choices.insert(id, index).is_some() {
                        return Err(fail("conflicting attribute choices for one physical node"));
                    }
                }
            }
        }
    }
    Ok(choices)
}
pub(super) fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
pub(super) fn escape_attribute(text: &str) -> String {
    escape(text).replace('"', "&quot;")
}
#[derive(Debug, Clone)]
pub(super) struct ElementSpan {
    pub range: Range<usize>,
    pub insert: usize,
    pub self_closing: bool,
}
impl ElementSpan {
    pub fn of(node: Node<'_, '_>) -> Result<Self> {
        let range = node.range();
        let source = &node.document().input_text()[range.clone()];
        let self_closing = source.ends_with("/>");
        let insert = if self_closing {
            range.end - 2
        } else {
            range.start
                + source
                    .rfind("</")
                    .ok_or_else(|| fail("missing closing element"))?
        };
        Ok(Self {
            range,
            insert,
            self_closing,
        })
    }
    pub fn insert_children(&self, name: &str, children: String) -> (Range<usize>, String) {
        if self.self_closing {
            (self.insert..self.range.end, format!(">{children}</{name}>"))
        } else {
            (self.insert..self.insert, children)
        }
    }
}
/// Apply only nonoverlapping, observed spans. Empty spans at one position may be
/// combined by the caller; ambiguous overlapping edits reject before mutation.
pub(super) fn apply(source: &str, mut edits: Vec<(Range<usize>, String)>) -> Result<String> {
    edits.sort_by_key(|(range, _)| (range.start, range.end));
    let mut previous_end = 0;
    let mut previous_empty = None;
    for (range, _) in &edits {
        if range.start > range.end
            || range.end > source.len()
            || !source.is_char_boundary(range.start)
            || !source.is_char_boundary(range.end)
            || range.start < previous_end
            || (range.is_empty() && previous_empty == Some(range.start))
        {
            return Err(fail("source edits overlap or leave UTF-8 source bounds"));
        }
        previous_end = range.end;
        previous_empty = range.is_empty().then_some(range.start);
    }
    let mut result = source.to_owned();
    for (range, value) in edits.into_iter().rev() {
        result.replace_range(range, &value);
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_attribute_choices_require_unique_allocated_ids_and_complete_lists() {
        for (xml, valid) in [
            (
                "<Spec><Overrides><AttributeOverride strNodes='7' dexNodes='8' intNodes=''/></Overrides></Spec>",
                true,
            ),
            (
                "<Spec><Overrides><AttributeOverride strNodes='7' dexNodes='7' intNodes=''/></Overrides></Spec>",
                false,
            ),
            (
                "<Spec><Overrides><AttributeOverride strNodes='7,7' dexNodes='' intNodes=''/></Overrides></Spec>",
                false,
            ),
            (
                "<Spec><Overrides><AttributeOverride strNodes='9' dexNodes='' intNodes=''/></Overrides></Spec>",
                false,
            ),
            (
                "<Spec><Overrides><AttributeOverride strNodes='7'/></Overrides></Spec>",
                false,
            ),
            ("<Spec><Overrides/><Overrides/></Spec>", false),
        ] {
            let doc = Document::parse(xml).unwrap();
            assert_eq!(
                attribute_options(doc.root_element(), &BTreeSet::from([7, 8])).is_ok(),
                valid,
                "{xml}"
            );
        }
    }
    #[test]
    fn edits_preserve_comments_crlf_and_reject_overlap_without_rewriting_source() {
        let source = "<Spec nodes='7'><!--marker-->\r\n</Spec>";
        let doc = Document::parse(source).unwrap();
        let node = doc.root_element();
        let span = ElementSpan::of(node).unwrap();
        let attr = node.attributes().next().unwrap().range_value();
        let edited = apply(
            source,
            vec![
                (attr, "7,8".into()),
                span.insert_children("Spec", "<Overrides/>".into()),
            ],
        )
        .unwrap();
        assert_eq!(
            edited,
            "<Spec nodes='7,8'><!--marker-->\r\n<Overrides/></Spec>"
        );
        assert!(apply(source, vec![(0..10, String::new()), (9..12, String::new())]).is_err());
        let source = "<Spec nodes='7'/>";
        let doc = Document::parse(source).unwrap();
        assert_eq!(
            apply(
                source,
                vec![
                    ElementSpan::of(doc.root_element())
                        .unwrap()
                        .insert_children("Spec", "<Overrides/>".into())
                ]
            )
            .unwrap(),
            "<Spec nodes='7'><Overrides/></Spec>"
        );
    }
}
