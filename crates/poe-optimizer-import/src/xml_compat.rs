//! Evaluator-only compatibility with the pinned runtime/lua/xml.lua reader.
//!
//! Generic XML decoding remains lossless. Call this after bounded, DTD-disabled
//! XML parsing: this linear lexical gate is not a second XML parser. PoB's small
//! reader silently loses numeric entities and attributes written using otherwise
//! valid XML syntax that its patterns do not recognize. Preserve the source by
//! rejecting those forms before either calculation backend interprets it.
use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlCompatibilityError {
    pub byte_offset: usize,
    pub reason: String,
}
impl fmt::Display for XmlCompatibilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "XML compatibility check failed at byte {}: {}",
            self.byte_offset, self.reason
        )
    }
}
impl Error for XmlCompatibilityError {}

fn error(byte_offset: usize, reason: impl Into<String>) -> XmlCompatibilityError {
    XmlCompatibilityError {
        byte_offset,
        reason: reason.into(),
    }
}
fn whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}
fn after(xml: &str, start: usize, delimiter: &str) -> Result<usize, XmlCompatibilityError> {
    xml.get(start..)
        .and_then(|tail| tail.find(delimiter))
        .map(|offset| start + offset + delimiter.len())
        .ok_or_else(|| error(start, "unterminated XML construct"))
}

/// Reject lexical forms that the upstream reader would silently reinterpret.
/// Named XML entities are supported. Comments and processing instructions are
/// ignored, and entity-like text inside a whole-element CDATA section stays literal.
/// Adjacent ordinary/CDATA fragments reject because PoB keeps separate strings.
/// This borrows the input and uses constant auxiliary memory; callers enforce
/// document byte/node limits and well-formedness with their existing XML parser.
pub fn validate(xml: &str) -> Result<(), XmlCompatibilityError> {
    validate_impl(xml, false, &[])
}

/// Also reject literal attribute tabs/line breaks before native interpretation.
/// Standard XML processors normalize these to spaces, whereas PoB retains them.
/// Shared reference preflight intentionally permits them: actual quest choice
/// strings contain raw newlines, and that backend passes those strings unchanged
/// to PoB. Native profiles must not silently evaluate normalized alternatives.
pub fn validate_native(xml: &str) -> Result<(), XmlCompatibilityError> {
    validate_impl(xml, true, &[])
}

/// Permit source attribute whitespace only in string Input values covered by a
/// successfully validated authored configuration projection. Unknown records,
/// placeholders, titles, names, and arbitrary string attributes stay strict.
/// Native capability checks remain the caller's responsibility.
pub fn validate_native_with_configuration(
    document: &roxmltree::Document<'_>,
) -> Result<(), XmlCompatibilityError> {
    let projection = crate::configuration::project(document).map_err(|e| {
        error(
            e.byte_offset,
            format!("configuration source projection failed: {}", e.reason),
        )
    })?;
    let mut preserved = projection.preserved_string_input_ranges();
    preserved.sort_unstable_by_key(|range| range.start);
    validate_impl(document.input_text(), true, &preserved)
}

/// Compatibility alias for the former customMods-only entry point.
pub fn validate_native_with_actor_inputs(
    document: &roxmltree::Document<'_>,
) -> Result<(), XmlCompatibilityError> {
    validate_native_with_configuration(document)
}
fn validate_impl(
    xml: &str,
    native: bool,
    preserved: &[std::ops::Range<usize>],
) -> Result<(), XmlCompatibilityError> {
    if xml.starts_with('\u{feff}') {
        return Err(error(
            0,
            "a leading UTF-8 BOM becomes a text node before the build root in PoB",
        ));
    }
    let bytes = xml.as_bytes();
    let mut at = 0;
    let mut cdata_may_start = false;
    let mut cdata_awaiting_close = false;
    while at < bytes.len() {
        if bytes[at] == b'&' && bytes.get(at + 1) == Some(&b'#') {
            return Err(error(
                at,
                "numeric character references are deleted by PoB; use literal text or a supported named entity",
            ));
        }
        if bytes[at] != b'<' {
            if !whitespace(bytes[at]) {
                if cdata_awaiting_close {
                    return Err(error(
                        at,
                        "CDATA must be the complete text content of its element; PoB keeps adjacent text as separate fragments",
                    ));
                }
                cdata_may_start = false;
            }
            at += 1;
            continue;
        }
        if xml[at..].starts_with("<!--") {
            at = after(xml, at + 4, "-->")?;
            continue;
        }
        if xml[at..].starts_with("<![CDATA[") {
            if !cdata_may_start || cdata_awaiting_close {
                return Err(error(
                    at,
                    "CDATA must be the complete text content of its element; PoB keeps adjacent text as separate fragments",
                ));
            }
            let end = after(xml, at + 9, "]]>")?;
            // Upstream removes comments globally before recognizing CDATA.
            if xml[at + 9..end - 3].contains("<!--") {
                return Err(error(
                    at,
                    "comment delimiters inside CDATA are removed by PoB",
                ));
            }
            at = end;
            cdata_awaiting_close = true;
            cdata_may_start = false;
            continue;
        }
        if xml[at..].starts_with("<?") {
            let end = after(xml, at + 2, "?>")?;
            if bytes[at + 2..end - 2].contains(&b'>') {
                return Err(error(
                    at,
                    "a greater-than sign inside a processing instruction terminates PoB's tag pattern early",
                ));
            }
            at = end;
            continue;
        }
        if xml[at..].starts_with("</") {
            at = after(xml, at + 2, ">")?;
            cdata_may_start = false;
            cdata_awaiting_close = false;
            continue;
        }
        if xml[at..].starts_with("<!") {
            return Err(error(at, "unsupported declaration"));
        }
        if cdata_awaiting_close {
            return Err(error(
                at,
                "CDATA must be the complete text content of its element",
            ));
        }
        at += 1;
        let name_start = at;
        while at < bytes.len() && (bytes[at].is_ascii_alphanumeric() || bytes[at] == b':') {
            at += 1;
        }
        if at == name_start
            || (at < bytes.len() && !whitespace(bytes[at]) && !matches!(bytes[at], b'/' | b'>'))
        {
            return Err(error(
                name_start,
                "element names must match PoB's ASCII letter/digit/colon pattern",
            ));
        }
        loop {
            while at < bytes.len() && whitespace(bytes[at]) {
                at += 1;
            }
            match bytes.get(at) {
                Some(b'>') => {
                    cdata_may_start = true;
                    at += 1;
                    break;
                }
                Some(b'/') if bytes.get(at + 1) == Some(&b'>') => {
                    cdata_may_start = false;
                    at += 2;
                    break;
                }
                None => return Err(error(at, "unterminated start tag")),
                _ => {}
            }
            let attribute_start = at;
            while at < bytes.len() && bytes[at].is_ascii_alphanumeric() {
                at += 1;
            }
            if at == attribute_start || bytes.get(at) != Some(&b'=') {
                return Err(error(
                    attribute_start,
                    "attributes require ASCII letter/digit names immediately followed by '=' (no surrounding whitespace)",
                ));
            }
            at += 1;
            let quote = match bytes.get(at) {
                Some(b'\'') => b'\'',
                Some(b'"') => b'"',
                _ => {
                    return Err(error(
                        at,
                        "attribute '=' must be immediately followed by its opening quote",
                    ));
                }
            };
            at += 1;
            while at < bytes.len() && bytes[at] != quote {
                if native
                    && matches!(bytes[at], b'\t' | b'\r' | b'\n')
                    && !preserved
                        .get(preserved.partition_point(|range| range.end <= at))
                        .is_some_and(|range| range.contains(&at))
                {
                    return Err(error(
                        at,
                        "literal tabs and line breaks in attribute values normalize differently in XML and PoB",
                    ));
                }
                if bytes[at] == b'>' {
                    return Err(error(
                        at,
                        "literal '>' inside an attribute terminates PoB's tag pattern early; use '&gt;'",
                    ));
                }
                if bytes[at] == b'&' && bytes.get(at + 1) == Some(&b'#') {
                    return Err(error(
                        at,
                        "numeric character references are deleted by PoB; use literal text or a supported named entity",
                    ));
                }
                at += 1;
            }
            if at == bytes.len() {
                return Err(error(at, "unterminated attribute value"));
            }
            at += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actor_legacy_whitespace_exception_is_exactly_scoped_to_raw_parsed_string_attributes() {
        let good = "<PathOfBuilding2><Config><ConfigSet id=\"1\"><Input name=\"customMods\" string=\"\t+3 to Strength\r\n+5 to Intelligence\n\"/></ConfigSet></Config></PathOfBuilding2>";
        let check = |xml: &str| {
            let document = roxmltree::Document::parse(xml).unwrap();
            validate_native_with_actor_inputs(&document)
        };
        assert!(validate_native(good).is_err());
        check(good).unwrap();
        for bad in [
            good.replace("<Config>", "<Future>")
                .replace("</Config>", "</Future>"),
            good.replace("<Input ", "<Input extra=\"true\" "),
            good.replace(
                "<ConfigSet id=\"1\">",
                "<ConfigSet id=\"1\" xmlns=\"urn:lookalike\">",
            ),
            good.replace("<Config>", "<Config xmlns=\"urn:lookalike\">"),
            good.replace("<Input ", "<x:Input xmlns:x=\"urn:lookalike\" "),
            good.replace("<PathOfBuilding2>", "<Other>")
                .replace("</PathOfBuilding2>", "</Other>"),
            good.replace("+3", "&#43;3"),
            good.replace(
                "name=\"customMods\"",
                "name=\"customMods\" title=\"bad\nheader\"",
            ),
        ] {
            assert!(check(&bad).is_err(), "accepted {bad}");
        }
    }
    #[test]
    fn supported_escapes_comments_cdata_processing_instructions_and_prose_are_untouched() {
        for xml in [
            "<Root level=\"60\" name='A &amp; B &gt; C &lt; D &quot; E &apos; F'/>",
            "<!-- level = '6&#48;' --><Root><![CDATA[Use &#48; or level = \"60\" here > there.]]></Root>",
            "<?xml version=\"1.0\"?><Root>Prose: level = '60'; a > b.</Root>",
            "<Root\n\tlevel=\"60\"\r\n/>",
            "<Root>Unicode: café and 日本語</Root>",
        ] {
            validate(xml).unwrap();
            validate_native(xml).unwrap();
        }
    }
    #[test]
    fn native_rejects_attribute_normalization_while_reference_preserves_quest_strings() {
        for raw in [
            "30% increased Charm Effect Duration\n\t+1 Charm Slot",
            "a\rb",
            "a\tb",
        ] {
            let xml = format!("<Input name='quest' string='{raw}'/>");
            validate(&xml).unwrap();
            assert!(validate_native(&xml).is_err());
        }
        let fixture = include_str!("../../../tests/fixtures/builds/pobarchives-Dfz36mCq.xml");
        validate(fixture).unwrap();
    }
    #[test]
    fn evaluator_rejects_leading_bom_but_allows_the_character_in_literal_content() {
        for xml in ["\u{feff}<Root/>", "\u{feff}<?xml version='1.0'?><Root/>"] {
            let error = validate(xml).unwrap_err();
            assert_eq!(error.byte_offset, 0);
            assert!(error.reason.contains("BOM"));
            assert!(validate_native(xml).is_err());
        }
        for xml in [
            "<Root>\u{feff}</Root>",
            "<Root><![CDATA[\u{feff}]]></Root>",
            "<!--\u{feff}--><Root/>",
        ] {
            validate(xml).unwrap();
            validate_native(xml).unwrap();
        }
    }
    #[test]
    fn rejects_upstream_silent_reinterpretations_and_handles_truncated_input() {
        for xml in [
            "<Root level =\"60\"/>",
            "<Root level= \"60\"/>",
            "<Root level='6&#48;'/>",
            "<Root level='6&#x30;'/>",
            "<Root>&#48;</Root>",
            "<Root label='a>b'/>",
            "<Root x-level='60'/>",
            "<Root foo:level='60'/>",
            "<Root-name/>",
            "<Root><![CDATA[a<!--lost-->b]]></Root>",
            "<?test a > b?><Root/>",
            "<Root",
            "<Root a='",
            "<!--",
            "<![CDATA[",
            "<?",
        ] {
            assert!(validate(xml).is_err(), "{xml}");
        }
    }
}
