use poe_optimizer_core::options::Scalar;
use poe_optimizer_import::{
    MAX_XML_BYTES, MAX_XML_NODES,
    build_source::{self, RootSectionKind, SourceElement},
};
use roxmltree::Document;
use sha2::{Digest, Sha256};

fn wrap(contents: &str) -> String {
    format!("<PathOfBuilding2>{contents}</PathOfBuilding2>")
}

fn exact_element(element: &SourceElement<'_>, xml: &str) {
    assert_eq!(element.source_xml(), &xml[element.source_range()]);
    assert_eq!(
        element.source_xml().as_ptr(),
        xml[element.source_range()].as_ptr(),
        "source fragments should borrow their original bytes"
    );
    for attribute in element.attributes() {
        let value = attribute.value();
        assert_eq!(value.raw(), &xml[value.range()]);
        assert_eq!(value.raw().as_ptr(), xml[value.range()].as_ptr());
    }
}

#[test]
fn immutable_corpus_hashes_and_independently_inventoried_sections_are_exact() {
    // Hashes are the immutable intake index; orders/counts were independently
    // inventoried with Python's ElementTree, rather than this Rust projection.
    let cases = [
        (
            1,
            49_642,
            "e3c0d0b40fa682260a1713acb03d52d720f4b769ac91b0501cbe2a84dc468194",
            "Build Import Party Tree Notes Skills Calcs TreeView Items Config",
            51,
        ),
        (
            2,
            116_512,
            "91366bd82a9afdd12ae7d8f695508a1b8d99116567010e082a9d31c4c4d4f631",
            "Build Tree Skills Config TreeView Items Calcs Import Notes Party",
            51,
        ),
        (
            3,
            46_532,
            "d3f7c72092f77481d3d1c5e38ec71d8730d607f19c659fc05b8a5db3bbbf9490",
            "Build Tree Skills Config TreeView Items Calcs Party Import Notes",
            51,
        ),
        (
            4,
            46_397,
            "62d760d326e21291cd1024f20660bd5046043c4e242b761df5d9a764be61e711",
            "Build Tree Skills Items Import Calcs TreeView Notes Party Config",
            52,
        ),
        (
            5,
            106_378,
            "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089",
            "Build Tree Skills Config TreeView Items Calcs Party Import Notes",
            51,
        ),
    ];
    for (line, bytes, hash, section_order, calcs_count) in cases {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../tests/fixtures/builds/breadth-20260908/build-{line:02}.xml"
        ));
        let original = std::fs::read(&path).unwrap();
        assert_eq!(original.len(), bytes);
        assert_eq!(format!("{:x}", Sha256::digest(&original)), hash);
        let xml = std::str::from_utf8(&original).unwrap();
        let projection = build_source::project_xml(xml).unwrap();
        assert_eq!(projection.source_sha256(), hash);
        assert_eq!(projection.source_xml().as_bytes(), original);
        assert_eq!(projection.source_xml().as_ptr(), xml.as_ptr());
        assert_eq!(projection.sections().len(), 10);
        assert_eq!(
            projection
                .sections()
                .iter()
                .map(|s| s.element().name())
                .collect::<Vec<_>>(),
            section_order.split_ascii_whitespace().collect::<Vec<_>>()
        );
        exact_element(projection.root(), xml);
        for section in projection.sections() {
            exact_element(section.element(), xml);
            assert_ne!(section.kind(), RootSectionKind::Unknown);
            if section.kind() == RootSectionKind::Calcs {
                assert_eq!(section.records().len(), calcs_count);
                assert_eq!(
                    section
                        .records()
                        .iter()
                        .filter(|r| r.element().name() == "Input")
                        .count(),
                    2
                );
                assert_eq!(
                    section
                        .records()
                        .iter()
                        .filter(|r| r.element().name() == "Section")
                        .count(),
                    calcs_count - 2
                );
                assert_eq!(
                    section
                        .records()
                        .iter()
                        .filter(|r| r.scalar_input().is_some())
                        .count(),
                    2
                );
            } else {
                assert!(section.records().is_empty());
            }
            for record in section.records() {
                exact_element(record.element(), xml);
                assert!(record.scalar_error().is_none());
                if let Some(input) = record.scalar_input() {
                    assert_eq!(input.source_xml(), &xml[input.source_range()]);
                    assert_eq!(
                        input.value_source().raw(),
                        &xml[input.value_source().range()]
                    );
                }
            }
        }
        assert_eq!(std::fs::read(path).unwrap(), original);
    }
}

#[test]
fn arbitrary_duplicate_unknown_and_legacy_sections_keep_authored_order() {
    let fragments = [
        "<Future revision='2'><Nested/></Future>",
        "<Calcs><Input name='same' number='1'/><Input name='same' number='2'/></Calcs>",
        "<Import lastRealm='NotACorpusRealm'/>",
        "<Build className='CallerSuppliedClass'/>",
        "<Spec nodes='caller data'/>",
        "<Future revision='1'/>",
        "<Calcs><Input name='same' string='third'/></Calcs>",
        "<Build className='SecondBuild'/>",
        "<Config/><Config/>",
    ];
    let xml = wrap(&fragments.join("\r\n<!-- preserve this separator -->\n"));
    let projection = build_source::project_xml(&xml).unwrap();
    assert_eq!(projection.sections().len(), 10);
    assert_eq!(projection.sections()[0].kind(), RootSectionKind::Unknown);
    assert_eq!(projection.sections()[4].kind(), RootSectionKind::LegacySpec);
    assert_eq!(projection.sections()[5].kind(), RootSectionKind::Unknown);
    assert_eq!(projection.sections()[8].kind(), RootSectionKind::Config);
    assert_eq!(projection.sections()[9].kind(), RootSectionKind::Config);
    for (section, expected) in projection.sections()[..8].iter().zip(&fragments) {
        assert_eq!(section.element().source_xml(), *expected);
    }
    let records = projection.sections()[1].records();
    assert_eq!(records[0].scalar_input().unwrap().name(), "same");
    assert_eq!(
        records[0].scalar_input().unwrap().value(),
        &Scalar::Number(1.0)
    );
    assert_eq!(
        records[1].scalar_input().unwrap().value(),
        &Scalar::Number(2.0)
    );
    let mut previous_end = projection.root().source_range().start;
    for section in projection.sections() {
        assert!(previous_end <= section.element().source_range().start);
        previous_end = section.element().source_range().end;
        exact_element(section.element(), &xml);
    }
    assert_eq!(projection.source_xml(), xml);
}

#[test]
fn attributes_preserve_lf_cr_tabs_unicode_quotes_and_one_pass_entities() {
    let raw = "\tA\r\nB\rC\n\u{00e9}\u{65e5}\u{672c}\u{8a9e}&lt;&gt;&amp;&apos;&quot;&amp;lt;";
    let expected = "\tA\r\nB\rC\n\u{00e9}\u{65e5}\u{672c}\u{8a9e}<>&'\"&lt;";
    let xml = format!(
        "<PathOfBuilding2 authored=\"{raw}\"><Import importLink='{raw}'/><Calcs><Input name='callerKey' string=\"{raw}\"/></Calcs></PathOfBuilding2>"
    );
    let projection = build_source::project_xml(&xml).unwrap();
    for element in [
        projection.root(),
        projection.sections()[0].element(),
        projection.sections()[1].records()[0].element(),
    ] {
        exact_element(element, &xml);
    }
    let values = [
        projection.root().attribute("authored").unwrap(),
        projection.sections()[0]
            .element()
            .attribute("importLink")
            .unwrap(),
        projection.sections()[1].records()[0]
            .element()
            .attribute("string")
            .unwrap(),
    ];
    for value in values {
        assert_eq!(value.raw(), raw);
        assert_eq!(value.decoded(), expected);
    }
    assert_eq!(
        projection.sections()[1].records()[0]
            .scalar_input()
            .unwrap()
            .value(),
        &Scalar::Text(expected.into())
    );
    let parsed = Document::parse(&xml).unwrap();
    assert_ne!(
        parsed.root_element().attribute("authored").unwrap(),
        expected
    );
    let normalized_xml = xml.replace("\r\n", "\n");
    assert_ne!(
        projection.source_sha256(),
        build_source::project_xml(&normalized_xml)
            .unwrap()
            .source_sha256()
    );
}

#[test]
fn malformed_calcs_scalars_are_retained_with_local_diagnostics() {
    let malformed = [
        "<Input name='missingValue'/>",
        "<Input number='1'/>",
        "<Input name='ambiguous' number='1' string='two'/>",
        "<Input name='notFinite' number='NaN'/>",
        "<Input name='overflow' number='1e999'/>",
        "<Input name='badBoolean' boolean='TRUE'/>",
        "<Input name='nested' string='x'><Unexpected/></Input>",
        "<Input name='extra' number='1' future='preserve'/>",
    ];
    let xml = wrap(&format!(
        "<Calcs>{}<Input name='afterFailures' number='  +1.25e2\t'/><Section id='callerSection' collapsed='notABoolean'/></Calcs>",
        malformed.join("")
    ));
    let projection = build_source::project_xml(&xml).unwrap();
    let records = projection.sections()[0].records();
    assert_eq!(records.len(), malformed.len() + 2);
    for (record, original) in records.iter().zip(malformed) {
        assert_eq!(record.element().source_xml(), original);
        assert!(record.scalar_input().is_none());
        let error = record.scalar_error().unwrap();
        assert!(!error.reason.is_empty());
        assert!(record.element().source_range().contains(&error.byte_offset));
    }
    assert_eq!(
        records[malformed.len()].scalar_input().unwrap().value(),
        &Scalar::Number(125.0)
    );
    let section = records.last().unwrap();
    assert!(section.scalar_input().is_none());
    assert!(section.scalar_error().is_none());
    assert_eq!(
        section.element().attribute("collapsed").unwrap().decoded(),
        "notABoolean"
    );
    let diagnostic = serde_json::to_value(&projection).unwrap();
    assert!(diagnostic["sections"][0]["records"][0]["scalar_error"]["reason"].is_string());
    assert!(diagnostic.get("source_xml").is_none());
}

#[test]
fn finite_calcs_numbers_preserve_bits_without_applying_settings() {
    for (text, expected) in [
        ("-0", -0.0_f64),
        ("4.9406564584124654e-324", f64::from_bits(1)),
        ("1.2e-307", 1.2e-307),
        ("1.7976931348623157e308", f64::MAX),
        ("  +.5\t", 0.5),
    ] {
        let xml = wrap(&format!(
            "<Calcs><Input name='arbitraryNumber' number='{text}'/></Calcs>"
        ));
        let projection = build_source::project_xml(&xml).unwrap();
        let input = projection.sections()[0].records()[0]
            .scalar_input()
            .unwrap();
        let Scalar::Number(actual) = input.value() else {
            panic!("expected number")
        };
        assert_eq!(actual.to_bits(), expected.to_bits());
        assert_eq!(input.value_source().raw(), text);
    }
}

#[test]
fn nested_known_names_and_mechanic_payloads_remain_opaque() {
    let item_payload = "<Items><Calcs><Input name='nestedInvalid' number='NaN'/></Calcs><Item><![CDATA[Rarity: UNIQUE\r\nCaller item <raw> &literal;]]></Item></Items>";
    let party_payload = "<Party><ImportedBuffs name='Aura'><Future><Input name='notACalcsInput' number='NaN'/></Future></ImportedBuffs><ExportedBuffs name='cached'>  retained &amp; text\r\n </ExportedBuffs></Party>";
    let xml = wrap(&format!(
        "{item_payload}{party_payload}<Future><Party/></Future>"
    ));
    let projection = build_source::project_xml(&xml).unwrap();
    let items = &projection.sections()[0];
    assert_eq!(items.element().source_xml(), item_payload);
    assert_eq!(items.element().child_element_count(), 2);
    assert!(items.records().is_empty());
    let party = &projection.sections()[1];
    assert_eq!(party.element().source_xml(), party_payload);
    assert_eq!(party.records().len(), 2);
    assert_eq!(party.records()[0].element().child_element_count(), 1);
    assert!(!party.records()[0].element().has_non_whitespace_text());
    assert!(party.records()[1].element().has_non_whitespace_text());
    for record in party.records() {
        assert!(record.scalar_input().is_none());
        assert!(record.scalar_error().is_none());
        exact_element(record.element(), &xml);
    }
    assert!(projection.sections()[2].records().is_empty());
}

#[test]
fn namespace_lookalikes_cannot_be_classified_as_known_auxiliary_content() {
    let xml = wrap(
        "<Calcs xmlns='urn:foreign'><Input name='x' number='1'/></Calcs><Calcs><Input xmlns='urn:foreign' name='x' number='2'/><Input name='x' number='3'/></Calcs><Party xmlns='urn:other'/>",
    );
    let projection = build_source::project_xml(&xml).unwrap();
    let foreign = &projection.sections()[0];
    assert_eq!(foreign.kind(), RootSectionKind::Unknown);
    assert_eq!(foreign.element().namespace(), Some("urn:foreign"));
    assert!(foreign.element().has_namespaces());
    assert!(foreign.records().is_empty());
    assert_eq!(
        foreign.element().source_xml(),
        "<Calcs xmlns='urn:foreign'><Input name='x' number='1'/></Calcs>"
    );
    let calcs = &projection.sections()[1];
    assert_eq!(calcs.kind(), RootSectionKind::Calcs);
    assert!(calcs.records()[0].element().has_namespaces());
    assert!(calcs.records()[0].scalar_input().is_none());
    assert!(calcs.records()[0].scalar_error().is_none());
    assert_eq!(
        calcs.records()[1].scalar_input().unwrap().value(),
        &Scalar::Number(3.0)
    );
    assert_eq!(projection.sections()[2].kind(), RootSectionKind::Unknown);
    assert!(build_source::project_xml("<PathOfBuilding2 xmlns='urn:foreign'/>").is_err());
    // Prefix declarations/attributes lie outside the PoB lexical subset.
    assert!(build_source::project_xml(&wrap("<x:Calcs xmlns:x='urn:foreign'/>")).is_err());
}

#[test]
fn root_content_is_evidence_and_does_not_require_or_invent_a_build() {
    let xml = "<?xml version='1.0'?><!--before--><PathOfBuilding2 caller='value'>\r\n authored root text <Notes>note</Notes>\n<!--inside--></PathOfBuilding2><!--after-->";
    let projection = build_source::project_xml(xml).unwrap();
    assert_eq!(projection.source_xml(), xml);
    assert!(projection.root().has_non_whitespace_text());
    assert_eq!(projection.root().child_element_count(), 1);
    assert_eq!(projection.sections()[0].kind(), RootSectionKind::Notes);
    assert_eq!(
        projection.root().attribute("caller").unwrap().decoded(),
        "value"
    );
    let empty = build_source::project_xml("<PathOfBuilding2/>").unwrap();
    assert!(empty.sections().is_empty());
    assert_eq!(empty.root().child_element_count(), 0);
    assert!(!empty.root().has_non_whitespace_text());
}

#[test]
fn document_and_pob_lexical_errors_reject_without_partial_projection() {
    for xml in [
        "",
        "<Other/>",
        "<PathOfBuilding2>",
        "<PathOfBuilding2/><PathOfBuilding2/>",
        "<!DOCTYPE PathOfBuilding2><PathOfBuilding2/>",
        "<!DOCTYPE PathOfBuilding2 [<!ENTITY x 'payload'>]><PathOfBuilding2>&x;</PathOfBuilding2>",
        "<PathOfBuilding2><Calcs x='1' x='2'/></PathOfBuilding2>",
        "<PathOfBuilding2><Import importLink = 'lost'/></PathOfBuilding2>",
        "<PathOfBuilding2><Import importLink='a>b'/></PathOfBuilding2>",
        "<PathOfBuilding2><Import importLink='&#10;'/></PathOfBuilding2>",
        "<PathOfBuilding2><Future>&unknown;</Future></PathOfBuilding2>",
        "\u{feff}<PathOfBuilding2/>",
    ] {
        let error = build_source::project_xml(xml).unwrap_err();
        assert!(!error.reason.is_empty(), "{xml}");
        assert!(error.byte_offset <= xml.len(), "{xml}");
    }
    let source = wrap("<Import importLink='&#10;'/>");
    let parsed = Document::parse(&source).unwrap();
    assert!(
        build_source::project(&parsed).is_err(),
        "external parsers cannot bypass the lexical gate"
    );
}

#[test]
fn root_and_auxiliary_count_limits_are_global_and_include_boundary_values() {
    let allowed = wrap(&"<Future/>".repeat(build_source::MAX_ROOT_SECTIONS));
    assert_eq!(
        build_source::project_xml(&allowed)
            .unwrap()
            .sections()
            .len(),
        build_source::MAX_ROOT_SECTIONS
    );
    let denied = wrap(&"<Future/>".repeat(build_source::MAX_ROOT_SECTIONS + 1));
    assert!(
        build_source::project_xml(&denied)
            .unwrap_err()
            .reason
            .contains("section count")
    );
    let half = "<Future/>".repeat(build_source::MAX_AUXILIARY_RECORDS / 2);
    let allowed = wrap(&format!("<Calcs>{half}</Calcs><Party>{half}</Party>"));
    assert_eq!(
        build_source::project_xml(&allowed)
            .unwrap()
            .sections()
            .iter()
            .map(|s| s.records().len())
            .sum::<usize>(),
        build_source::MAX_AUXILIARY_RECORDS
    );
    let denied = allowed.replace("</Party>", "<Future/></Party>");
    assert!(
        build_source::project_xml(&denied)
            .unwrap_err()
            .reason
            .contains("record count")
    );
}

#[test]
fn projected_name_attribute_and_aggregate_storage_limits_fail_closed() {
    let attrs = (0..build_source::MAX_ELEMENT_ATTRIBUTES)
        .map(|i| format!(" a{i}='x'"))
        .collect::<String>();
    let allowed = format!("<PathOfBuilding2{attrs}/>");
    assert_eq!(
        build_source::project_xml(&allowed)
            .unwrap()
            .root()
            .attributes()
            .len(),
        build_source::MAX_ELEMENT_ATTRIBUTES
    );
    let denied = format!("<PathOfBuilding2{attrs} extra='x'/>");
    assert!(
        build_source::project_xml(&denied)
            .unwrap_err()
            .reason
            .contains("attribute count")
    );
    let name = "N".repeat(build_source::MAX_SOURCE_NAME_BYTES + 1);
    assert!(build_source::project_xml(&wrap(&format!("<{name}/>"))).is_err());
    assert!(build_source::project_xml(&wrap(&format!("<Future {name}='x'/>"))).is_err());
    let value = "x".repeat(build_source::MAX_SOURCE_VALUE_BYTES);
    let allowed = wrap(&format!("<Future data='{value}'/>"));
    assert_eq!(
        build_source::project_xml(&allowed).unwrap().sections()[0]
            .element()
            .attribute("data")
            .unwrap()
            .raw()
            .len(),
        value.len()
    );
    let denied = wrap(&format!("<Future data='{value}x'/>"));
    assert!(
        build_source::project_xml(&denied)
            .unwrap_err()
            .reason
            .contains("value byte")
    );
    // Every individual value is legal; only the combined retained allocation is too large.
    let per_value = build_source::MAX_SOURCE_VALUE_BYTES;
    let count = build_source::MAX_PROJECTED_ATTRIBUTE_BYTES / per_value;
    let fragment = format!("<Future data='{value}'/>");
    assert!(build_source::project_xml(&wrap(&fragment.repeat(count - 1))).is_ok());
    assert!(
        build_source::project_xml(&wrap(&fragment.repeat(count + 1)))
            .unwrap_err()
            .reason
            .contains("projected attribute byte")
    );
}

#[test]
fn global_xml_limits_also_apply_to_opaque_nested_payloads_and_external_documents() {
    let oversized = wrap(&format!("<Future>{}</Future>", "x".repeat(MAX_XML_BYTES)));
    assert!(
        build_source::project_xml(&oversized)
            .unwrap_err()
            .reason
            .contains("projection limits")
    );
    let dense = wrap(&format!(
        "<Future>{}</Future>",
        "<N/>".repeat(MAX_XML_NODES as usize + 1)
    ));
    assert!(build_source::project_xml(&dense).is_err());
    let external = Document::parse(&dense).unwrap();
    assert!(
        build_source::project(&external)
            .unwrap_err()
            .reason
            .contains("projection limits")
    );
    // An opaque descendant is not mistaken for a shallow projected attribute.
    let opaque = wrap(&format!(
        "<Future><Nested data='{}'/></Future>",
        "x".repeat(build_source::MAX_SOURCE_VALUE_BYTES + 1)
    ));
    let projection = build_source::project_xml(&opaque).unwrap();
    assert_eq!(projection.source_xml(), opaque);
    assert!(projection.sections()[0].records().is_empty());
}
