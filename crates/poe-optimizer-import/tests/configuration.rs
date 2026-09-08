use poe_optimizer_core::options::Scalar;
use poe_optimizer_import::{configuration::*, source_xml, xml_compat};
use roxmltree::Document;
fn wrap(config: &str) -> String {
    format!("<PathOfBuilding2>{config}</PathOfBuilding2>")
}
fn gate(xml: &str) -> Result<(), xml_compat::XmlCompatibilityError> {
    xml_compat::validate_native_with_configuration(&Document::parse(xml).unwrap())
}
#[test]
fn exact_raw_ranges_named_entities_and_attribute_whitespace_survive() {
    let xml = wrap(
        "<Config activeConfigSet=\"7\"><ConfigSet id=\"7\" title=\"A &amp; B\"><Input name=\"unknownKey\" string=\"\tA\r\nB\n&lt;&gt;&amp;&apos;&quot;&amp;lt;\"/></ConfigSet></Config>",
    );
    let p = project_xml(&xml).unwrap();
    let input = &p.active_set().inputs()[0];
    assert_eq!(input.value(), &Scalar::Text("\tA\r\nB\n<>&'\"&lt;".into()));
    assert_eq!(
        input.value_source().raw(),
        &xml[input.value_source().range()]
    );
    assert_eq!(input.source_xml(), &xml[input.source_range()]);
    assert_eq!(p.source_xml(), xml);
    assert_eq!(p.active_set().title(), "A & B");
    assert!(xml_compat::validate_native(&xml).is_err());
    gate(&xml).unwrap();
    let doc = Document::parse(&xml).unwrap();
    let n = doc.descendants().find(|n| n.has_tag_name("Input")).unwrap();
    assert_eq!(read_input_scalar(n).unwrap(), input.value().clone());
    assert_ne!(
        n.attribute("string").unwrap(),
        input.value_source().decoded()
    );
    assert_eq!(
        source_xml::decode_named_entities("&amp;#10;").unwrap(),
        "&#10;"
    );
}
#[test]
fn cross_kind_order_and_unhandled_source_are_explicit() {
    let xml = wrap(
        "<Config><ConfigSet id=\"2\"><Input name=\"same\" string=\"input\"/><Future raw=\"untouched\"><Nested/></Future><Placeholder name=\"same\" string=\"placeholder\"/><CustomModifierBlock title=\"T\" enabled=\"false\">  +3 to Strength\r\n </CustomModifierBlock><Input name=\"flag\" boolean=\"false\"/></ConfigSet></Config>",
    );
    let p = project_xml(&xml).unwrap();
    let s = p.active_set();
    assert_eq!(
        s.records_in_source_order(),
        &[
            ConfigurationRecordIndex::Input(0),
            ConfigurationRecordIndex::Unknown(0),
            ConfigurationRecordIndex::Placeholder(0),
            ConfigurationRecordIndex::Block(0),
            ConfigurationRecordIndex::Input(1)
        ]
    );
    assert_eq!(s.inputs()[0].value(), &Scalar::Text("input".into()));
    assert_eq!(
        s.placeholders()[0].value(),
        &Scalar::Text("placeholder".into())
    );
    assert_eq!(
        s.unknown_records()[0].source_xml(),
        "<Future raw=\"untouched\"><Nested/></Future>"
    );
    assert_eq!(s.blocks()[0].text().decoded(), "  +3 to Strength\r\n ");
    assert_eq!(s.blocks()[0].pob_text(), "+3 to Strength");
    assert!(!s.blocks()[0].enabled());
    assert!(p.diagnostic()["projection"].get("source_xml").is_none());
}
#[test]
fn authored_values_do_not_apply_configtab_migrations() {
    let xml = wrap(
        "<Config><Input name=\"enemyIsBoss\" string=\"shaper\"/><Input name=\"presetBossSkills\" string=\"Uber Example\"/><Input name=\"customMods\" string=\"+5 to Strength\"/><Placeholder name=\"sourceOnly\" string=\"authored\"/></Config>",
    );
    let p = project_xml(&xml).unwrap();
    assert_eq!(p.layout(), ConfigurationLayout::Legacy);
    assert!(p.active_set().is_implicit());
    assert_eq!(
        p.active_set().inputs()[0].value(),
        &Scalar::Text("shaper".into())
    );
    assert_eq!(
        p.active_set().inputs()[1].value(),
        &Scalar::Text("Uber Example".into())
    );
    assert_eq!(p.active_set().inputs()[2].name(), "customMods");
    assert!(p.active_set().blocks().is_empty());
    assert_eq!(p.active_set().placeholders()[0].name(), "sourceOnly");
}
#[test]
fn missing_empty_legacy_and_active_fallback_provenance() {
    for (config, layout) in [
        ("", ConfigurationLayout::Missing),
        ("<Config/>", ConfigurationLayout::Empty),
        (
            "<Config><Input name=\"x\" number=\"2\"/></Config>",
            ConfigurationLayout::Legacy,
        ),
    ] {
        let xml = wrap(config);
        let p = project_xml(&xml).unwrap();
        assert_eq!(p.layout(), layout);
        assert_eq!(p.active_set_id(), 1);
        assert_eq!(p.requested_active_set_id(), None);
        assert_eq!(p.active_set_resolution(), ActiveSetResolution::DefaultOne);
        assert!(p.active_set().id_source().is_none());
    }
    for (active, requested, resolution) in [
        ("", None, ActiveSetResolution::MissingDefaultUsesFirst),
        (
            " activeConfigSet=\"9\"",
            Some(9),
            ActiveSetResolution::Requested,
        ),
        (
            " activeConfigSet=\"8\"",
            Some(8),
            ActiveSetResolution::MissingRequestedUsesFirst,
        ),
    ] {
        let xml = wrap(&format!(
            "<Config{active}><ConfigSet id=\"2\"/><ConfigSet id=\"9\"/></Config>"
        ));
        let p = project_xml(&xml).unwrap();
        assert_eq!(
            p.sets()
                .iter()
                .map(ConfigSetProjection::id)
                .collect::<Vec<_>>(),
            vec![2, 9]
        );
        assert_eq!(p.requested_active_set_id(), requested);
        assert_eq!(p.active_set_resolution(), resolution);
        assert_eq!(p.active_set_id(), if requested == Some(9) { 9 } else { 2 });
    }
}
#[test]
fn only_successfully_projected_string_input_values_receive_whitespace_exemption() {
    for good in [
        "<Config><Input name=\"arbitrary\" string=\"a\nb\"/></Config>",
        "<Config><ConfigSet id=\"3\"><Input name=\"arbitrary\" string=\"a\nb\"/></ConfigSet></Config>",
    ] {
        gate(&wrap(good)).unwrap();
    }
    for bad in [
        "<Other><Input name=\"x\" string=\"a\nb\"/></Other>",
        "<Config><Other><Input name=\"x\" string=\"a\nb\"/></Other></Config>",
        "<Config><Placeholder name=\"x\" string=\"a\nb\"/></Config>",
        "<Config><ConfigSet id=\"1\" title=\"a\nb\"/></Config>",
        "<Config><Input name=\"a\nb\" string=\"v\"/></Config>",
        "<Config><Input name=\"x\" number=\"1\n\"/></Config>",
        "<Config><Input name=\"x\" string=\"a\nb\" number=\"1\"/></Config>",
        "<Config><Input name=\"x\" string=\"a\nb\"/><Input name=\"x\" string=\"again\"/></Config>",
        "<Config><ConfigSet><Input name=\"x\" string=\"a\nb\"/></ConfigSet></Config>",
    ] {
        assert!(gate(&wrap(bad)).is_err(), "{bad}");
    }
    let duplicate = wrap("<Config/><Config/>");
    assert!(
        gate(&duplicate)
            .unwrap_err()
            .reason
            .contains("duplicate Config")
    );
}
#[test]
fn ambiguous_structures_and_lexical_scalar_forms_fail_closed() {
    for bad in [
        "<Config/><Config/>",
        "<Config><ConfigSet id=\"1\"/><ConfigSet id=\"01\"/></Config>",
        "<Config><ConfigSet id=\"1\"/><Input name=\"x\" number=\"1\"/></Config>",
        "<Config><ConfigSet id=\"1\"><ConfigSet id=\"2\"/></ConfigSet></Config>",
        "<Config><Input name=\"x\" number=\"1\"/><Input name=\"x\" number=\"2\"/></Config>",
        "<Config><Placeholder name=\"x\" number=\"1\"/><Placeholder name=\"x\" number=\"2\"/></Config>",
        "<Config><Input name=\"x\"/></Config>",
        "<Config><Input number=\"2\"/></Config>",
        "<Config><Input name=\"x\" number=\"1\" boolean=\"true\"/></Config>",
        "<Config><Placeholder name=\"x\" boolean=\"true\"/></Config>",
        "<Config><Input name=\"x\" boolean=\"TRUE\"/></Config>",
        "<Config><Input name=\"x\" number=\"NaN\"/></Config>",
        "<Config><Input name=\"x\" number=\"1e999\"/></Config>",
        "<Config><Input name =\"x\" string=\"value\"/></Config>",
        "<Config><Input name=\"x\" string=\"&#10;\"/></Config>",
        "<Config xmlns=\"urn:foreign\"><Input name=\"x\" string=\"v\"/></Config>",
        "<Config>text</Config>",
        "<Config><CustomModifierBlock>A<!--cut-->B</CustomModifierBlock></Config>",
    ] {
        assert!(project_xml(&wrap(bad)).is_err(), "{bad}");
    }
    assert!(project_xml("<!DOCTYPE PathOfBuilding2><PathOfBuilding2/>").is_err());
}
#[test]
fn finite_decimal_numbers_preserve_source_compatible_ascii_trimming() {
    for (text, value) in [
        ("  +1.25e2\t", 125.0_f64),
        (".5", 0.5),
        ("1.", 1.0),
        ("-0", -0.0),
    ] {
        let xml = wrap(&format!(
            "<Config><Input name=\"n\" number=\"{text}\"/></Config>"
        ));
        let p = project_xml(&xml).unwrap();
        let Scalar::Number(n) = p.active_set().inputs()[0].value() else {
            panic!()
        };
        assert_eq!(n.to_bits(), value.to_bits());
    }
}
#[test]
fn configuration_bounds_apply_before_retaining_large_values() {
    let sets = (1..=MAX_CONFIG_SETS + 1)
        .map(|i| format!("<ConfigSet id=\"{i}\"/>"))
        .collect::<String>();
    assert!(project_xml(&wrap(&format!("<Config>{sets}</Config>"))).is_err());
    let inputs = (0..=MAX_CONFIG_RECORDS)
        .map(|i| format!("<Input name=\"{i}\" number=\"1\"/>"))
        .collect::<String>();
    assert!(project_xml(&wrap(&format!("<Config>{inputs}</Config>"))).is_err());
    let value = "a".repeat(MAX_CONFIG_VALUE_BYTES + 1);
    assert!(
        project_xml(&wrap(&format!(
            "<Config><Input name=\"x\" string=\"{value}\"/></Config>"
        )))
        .is_err()
    );
    let value = "a".repeat(MAX_CONFIG_BYTES);
    assert!(project_xml(&wrap(&format!("<Config><Future>{value}</Future></Config>"))).is_err());
}
#[test]
fn all_supplied_documents_project_every_config_set_without_changing_source() {
    for line in 1..=5 {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../tests/fixtures/builds/breadth-20260908/build-{line:02}.xml"
        ));
        let bytes = std::fs::read(path).unwrap();
        let xml = std::str::from_utf8(&bytes).unwrap();
        let p = project_xml(xml).unwrap();
        assert_eq!(p.source_xml().as_bytes(), bytes);
        let doc = Document::parse(xml).unwrap();
        assert_eq!(
            p.sets().len(),
            doc.descendants()
                .filter(|n| n.has_tag_name("ConfigSet"))
                .count()
        );
        for set in p.sets() {
            for r in set.inputs().iter().chain(set.placeholders()) {
                assert_eq!(r.source_xml(), &xml[r.source_range()]);
                assert_eq!(r.value_source().raw(), &xml[r.value_source().range()]);
            }
        }
    }
}
