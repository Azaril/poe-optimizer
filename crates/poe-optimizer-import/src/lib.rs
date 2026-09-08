//! Bounded, lossless decoding of Path of Building 2 XML and share codes.
//!
//! Import validates the container, not the build's mechanics. In particular, cached
//! metrics in an export are preserved as source data and are never evaluated here.

pub mod actor_assembly;
pub mod actor_modifiers;
pub mod configuration;
pub mod controlled_build;
pub mod controlled_mace;
pub mod equipment;
mod item_formatting;
pub mod mace_item;
mod modifier_syntax;
pub mod preflight;
pub mod source_xml;
pub mod xml_compat;

use base64::{
    Engine as _,
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
};
use flate2::{Decompress, FlushDecompress, Status};
use roxmltree::{Document, ParsingOptions};
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Maximum share-code source size, including surrounding whitespace.
pub const MAX_SHARE_CODE_BYTES: usize = 1024 * 1024;
/// Maximum raw or decompressed XML size, including any BOM and whitespace.
pub const MAX_XML_BYTES: usize = 8 * 1024 * 1024;
/// Bounds the validation parser's tree allocation independently of byte size.
pub const MAX_XML_NODES: u32 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportFormat {
    RawXml,
    ShareCode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportedBuild {
    /// Exact UTF-8 XML, including unknown fields and original whitespace.
    pub xml: String,
    pub format: ImportFormat,
    /// SHA-256 of the exact XML bytes, not a semantic candidate identity.
    pub sha256: String,
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("the build input is empty")]
    Empty,
    #[error("build input is {actual} bytes; the limit for this format is {limit} bytes")]
    SourceTooLarge { actual: usize, limit: usize },
    #[error("decompressed XML exceeds the {MAX_XML_BYTES}-byte limit")]
    XmlTooLarge,
    #[error("build XML must be UTF-8: {0}")]
    InvalidUtf8(#[from] std::str::Utf8Error),
    #[error("invalid URL-safe base64 PoB share code: {0}")]
    InvalidBase64(#[from] base64::DecodeError),
    #[error("invalid zlib-compressed PoB share code: {0}")]
    InvalidZlib(#[from] flate2::DecompressError),
    #[error("the PoB share code contains an incomplete zlib stream")]
    IncompleteZlibStream,
    #[error("the PoB share code has data after the complete zlib stream")]
    TrailingCompressedData,
    #[error("invalid build XML: {0}")]
    InvalidXml(#[from] roxmltree::Error),
    #[error("expected an unnamespaced PathOfBuilding2 root; found {actual}")]
    WrongRoot { actual: String },
}

/// Decode raw XML or a URL-safe base64(zlib(XML)) PoB share code.
///
/// Share codes may be padded or unpadded and have surrounding ASCII whitespace.
/// A raw XML input is preserved byte for byte. DTDs, external entities, undeclared
/// entity references, multiple roots and malformed XML are rejected. Predefined
/// XML escapes and valid numeric character references remain supported.
///
/// Callers reading a file or network body should also bound that read; this API
/// limits decoding and parsing allocations after receiving its input slice.
pub fn decode_build(bytes: &[u8]) -> Result<ImportedBuild, ImportError> {
    if bytes.len() > MAX_XML_BYTES {
        return Err(ImportError::SourceTooLarge {
            actual: bytes.len(),
            limit: MAX_XML_BYTES,
        });
    }
    let trimmed = bytes.trim_ascii();
    if trimmed.is_empty() {
        return Err(ImportError::Empty);
    }
    let xml_prefix = bytes
        .strip_prefix(b"\xef\xbb\xbf")
        .unwrap_or(bytes)
        .trim_ascii_start();
    let (xml, format) = if xml_prefix.starts_with(b"<") {
        (std::str::from_utf8(bytes)?.to_owned(), ImportFormat::RawXml)
    } else {
        if bytes.len() > MAX_SHARE_CODE_BYTES {
            return Err(ImportError::SourceTooLarge {
                actual: bytes.len(),
                limit: MAX_SHARE_CODE_BYTES,
            });
        }
        let compressed = URL_SAFE
            .decode(trimmed)
            .or_else(|_| URL_SAFE_NO_PAD.decode(trimmed))?;
        let decoded = inflate_bounded(&compressed)?;
        (
            std::str::from_utf8(&decoded)?.to_owned(),
            ImportFormat::ShareCode,
        )
    };

    validate_xml(&xml)?;
    let sha256 = format!("{:x}", Sha256::digest(xml.as_bytes()));
    Ok(ImportedBuild {
        xml,
        format,
        sha256,
    })
}

fn inflate_bounded(compressed: &[u8]) -> Result<Vec<u8>, ImportError> {
    let mut stream = Decompress::new(true);
    let mut xml = Vec::new();
    let mut buffer = [0u8; 16 * 1024];
    loop {
        let input_before = stream.total_in() as usize;
        let output_before = stream.total_out() as usize;
        let status = stream.decompress(
            &compressed[input_before..],
            &mut buffer,
            FlushDecompress::None,
        )?;
        let produced = stream.total_out() as usize - output_before;
        if produced > MAX_XML_BYTES - xml.len() {
            return Err(ImportError::XmlTooLarge);
        }
        xml.extend_from_slice(&buffer[..produced]);
        if status == Status::StreamEnd {
            if stream.total_in() as usize != compressed.len() {
                return Err(ImportError::TrailingCompressedData);
            }
            return Ok(xml);
        }
        if stream.total_in() as usize == input_before && produced == 0 {
            return Err(ImportError::IncompleteZlibStream);
        }
    }
}

fn validate_xml(xml: &str) -> Result<(), ImportError> {
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: MAX_XML_NODES,
            entity_resolver: None,
        },
    )?;
    let root = document.root_element().tag_name();
    if root.name() != "PathOfBuilding2" || root.namespace().is_some() {
        let actual = match root.namespace() {
            Some(namespace) => format!("{{{namespace}}}{}", root.name()),
            None => root.name().to_owned(),
        };
        return Err(ImportError::WrongRoot { actual });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use flate2::{Compression, write::ZlibEncoder};

    use super::*;

    const FIXTURE_XML: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/builds/pobarchives-Dfz36mCq.xml"
    ));
    const FIXTURE_SHARE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/builds/pobarchives-Dfz36mCq.import.txt"
    ));
    const FIXTURE_HASH: &str = "e3c0d0b40fa682260a1713acb03d52d720f4b769ac91b0501cbe2a84dc468194";

    fn compress(xml: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(xml).unwrap();
        encoder.finish().unwrap()
    }

    fn share_code(xml: &[u8]) -> String {
        URL_SAFE_NO_PAD.encode(compress(xml))
    }

    #[test]
    fn supplied_share_and_raw_fixture_have_identical_xml_and_hash() {
        let raw = decode_build(FIXTURE_XML).unwrap();
        let shared = decode_build(FIXTURE_SHARE).unwrap();
        assert_eq!(raw.format, ImportFormat::RawXml);
        assert_eq!(shared.format, ImportFormat::ShareCode);
        assert_eq!(raw.xml.as_bytes(), FIXTURE_XML);
        assert_eq!(shared.xml.as_bytes(), FIXTURE_XML);
        assert_eq!(raw.sha256, FIXTURE_HASH);
        assert_eq!(shared.sha256, FIXTURE_HASH);
    }

    #[test]
    fn preserves_unknown_fields_whitespace_and_escaped_text() {
        let xml = "\u{feff}<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n\
            <PathOfBuilding2 future=\"&amp;&#x1F600;\">\r\n\
            <Unknown><![CDATA[keep <this> & unchanged]]></Unknown>\r\n\
            &lt;&gt;&apos;&quot;&#32;</PathOfBuilding2>\r\n";
        let decoded = decode_build(xml.as_bytes()).unwrap();
        assert_eq!(decoded.xml, xml);
        assert_eq!(
            decode_build(share_code(xml.as_bytes()).as_bytes())
                .unwrap()
                .xml,
            xml
        );
    }

    #[test]
    fn accepts_padded_and_unpadded_codes_with_outer_whitespace() {
        let xml = b"<PathOfBuilding2><Build/></PathOfBuilding2>";
        let compressed = compress(xml);
        for code in [
            URL_SAFE.encode(&compressed),
            URL_SAFE_NO_PAD.encode(&compressed),
        ] {
            let wrapped = format!(" \r\n\t{code}\t\r\n ");
            assert_eq!(
                decode_build(wrapped.as_bytes()).unwrap().xml.as_bytes(),
                xml
            );
        }
    }

    #[test]
    fn rejects_malformed_base64_and_internal_whitespace() {
        for code in ["!", "a", "====", "eA==garbage", "e A==", "eA==\neA=="] {
            assert!(
                matches!(
                    decode_build(code.as_bytes()),
                    Err(ImportError::InvalidBase64(_))
                ),
                "{code:?}"
            );
        }
    }

    #[test]
    fn rejects_empty_and_non_utf8_xml() {
        assert!(matches!(decode_build(b" \r\n\t"), Err(ImportError::Empty)));
        let xml = b"<PathOfBuilding2>\xff</PathOfBuilding2>";
        assert!(matches!(
            decode_build(xml),
            Err(ImportError::InvalidUtf8(_))
        ));
        assert!(matches!(
            decode_build(share_code(xml).as_bytes()),
            Err(ImportError::InvalidUtf8(_))
        ));
    }

    #[test]
    fn rejects_wrong_or_namespaced_roots() {
        for xml in [
            "<PathOfBuilding/>",
            "<html/>",
            "<PathOfBuilding2 xmlns='urn:other'/>",
        ] {
            assert!(
                matches!(
                    decode_build(xml.as_bytes()),
                    Err(ImportError::WrongRoot { .. })
                ),
                "{xml}"
            );
        }
    }

    #[test]
    fn rejects_dtds_and_undeclared_entities() {
        for xml in [
            "<!DOCTYPE PathOfBuilding2><PathOfBuilding2/>",
            "<!DOCTYPE PathOfBuilding2 SYSTEM 'file:///private'><PathOfBuilding2/>",
            "<!DOCTYPE PathOfBuilding2 [<!ENTITY x 'expand'>]><PathOfBuilding2>&x;</PathOfBuilding2>",
            "<PathOfBuilding2>&external;</PathOfBuilding2>",
            "<PathOfBuilding2 data='&external;'/>",
        ] {
            assert!(
                matches!(
                    decode_build(xml.as_bytes()),
                    Err(ImportError::InvalidXml(_))
                ),
                "{xml}"
            );
        }
    }

    #[test]
    fn rejects_incomplete_or_malformed_xml_documents() {
        for xml in [
            "<PathOfBuilding2>",
            "<PathOfBuilding2><Build></PathOfBuilding2>",
            "<PathOfBuilding2/><Other/>",
            "<PathOfBuilding2/>trailing text",
            "<PathOfBuilding2><![CDATA[unfinished</PathOfBuilding2>",
            "<PathOfBuilding2 x='1' x='2'/>",
            "<PathOfBuilding2 x='<bad'/>",
            "<PathOfBuilding2>&#0;</PathOfBuilding2>",
            "<PathOfBuilding2>\u{1}</PathOfBuilding2>",
            "<PathOfBuilding2><!-- bad -- comment --></PathOfBuilding2>",
        ] {
            assert!(
                matches!(
                    decode_build(xml.as_bytes()),
                    Err(ImportError::InvalidXml(_))
                ),
                "{xml}"
            );
        }
    }

    #[test]
    fn rejects_truncated_zlib_at_every_boundary() {
        let compressed = compress(b"<PathOfBuilding2><Build/></PathOfBuilding2>");
        for end in 1..compressed.len() {
            let code = URL_SAFE_NO_PAD.encode(&compressed[..end]);
            assert!(
                matches!(
                    decode_build(code.as_bytes()),
                    Err(ImportError::IncompleteZlibStream | ImportError::InvalidZlib(_))
                ),
                "truncated at byte {end}"
            );
        }
    }

    #[test]
    fn rejects_bad_zlib_checksum_and_trailing_or_concatenated_streams() {
        let compressed = compress(b"<PathOfBuilding2/>");
        let mut corrupt = compressed.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        assert!(matches!(
            decode_build(URL_SAFE_NO_PAD.encode(corrupt).as_bytes()),
            Err(ImportError::InvalidZlib(_))
        ));
        for suffix in [b"trailing".to_vec(), compressed.clone()] {
            let mut trailing = compressed.clone();
            trailing.extend_from_slice(&suffix);
            assert!(matches!(
                decode_build(URL_SAFE_NO_PAD.encode(trailing).as_bytes()),
                Err(ImportError::TrailingCompressedData)
            ));
        }
    }

    #[test]
    fn bounds_both_source_formats_including_surrounding_whitespace() {
        let code = vec![b'A'; MAX_SHARE_CODE_BYTES + 1];
        assert!(matches!(
            decode_build(&code),
            Err(ImportError::SourceTooLarge {
                limit: MAX_SHARE_CODE_BYTES,
                ..
            })
        ));
        let mut xml = b"<PathOfBuilding2/>".to_vec();
        xml.resize(MAX_XML_BYTES + 1, b' ');
        assert!(matches!(
            decode_build(&xml),
            Err(ImportError::SourceTooLarge {
                limit: MAX_XML_BYTES,
                ..
            })
        ));
    }

    #[test]
    fn bounds_expansion_and_accepts_exact_xml_byte_limit() {
        let mut xml = b"<PathOfBuilding2/>".to_vec();
        xml.resize(MAX_XML_BYTES, b' ');
        assert_eq!(decode_build(&xml).unwrap().xml.len(), MAX_XML_BYTES);
        assert_eq!(
            decode_build(share_code(&xml).as_bytes()).unwrap().xml.len(),
            MAX_XML_BYTES
        );
        xml.push(b' ');
        assert!(matches!(
            decode_build(share_code(&xml).as_bytes()),
            Err(ImportError::XmlTooLarge)
        ));
    }

    #[test]
    fn bounds_xml_node_count() {
        let xml = format!(
            "<PathOfBuilding2>{}</PathOfBuilding2>",
            "<Node/>".repeat(MAX_XML_NODES as usize)
        );
        assert!(matches!(
            decode_build(xml.as_bytes()),
            Err(ImportError::InvalidXml(roxmltree::Error::NodesLimitReached))
        ));
    }
}
