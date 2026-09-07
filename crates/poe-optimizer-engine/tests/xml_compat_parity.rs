//! Actual pinned parser oracle for the shared evaluator-only lexical gate.
#![cfg(not(target_arch = "wasm32"))]
#[path = "../../poe-optimizer-import/src/xml_compat.rs"]
mod xml_compat;
use mlua::{Function, Lua, Table};
use sha2::{Digest, Sha256};
const XML: &str = include_str!("../../../vendor/path-of-building-poe2/runtime/lua/xml.lua");
struct Reader {
    _lua: Lua,
    read: Function,
    warm: bool,
}
impl Reader {
    fn new(warm: bool) -> Self {
        assert_eq!(
            format!("{:x}", Sha256::digest(XML.replace("\r\n", "\n"))),
            "832b6bb31f1f0e79e3ed6346113691b36ace29ed964e332530eef8b3b63af0e2"
        );
        let lua = Lua::new();
        lua.load(if warm { "jit.on()" } else { "jit.off()" })
            .exec()
            .unwrap();
        let module: Table = lua.load(XML).set_name("pinned-runtime-xml").eval().unwrap();
        let parse: Function = module.get("ParseXML").unwrap();
        lua.globals().set("pinned_parse", parse).unwrap();
        let read=lua.load("return function(text,warm) local result,err; for i=1,(warm and 200 or 1) do result,err=pinned_parse(text) end; return result,err end").eval().unwrap();
        Self {
            _lua: lua,
            read,
            warm,
        }
    }
    fn read(&self, text: &str) -> (Option<Table>, Option<String>) {
        self.read.call((text, self.warm)).unwrap()
    }
    fn root(&self, text: &str) -> Table {
        let (nodes, error) = self.read(text);
        assert!(error.is_none(), "{error:?}");
        nodes.unwrap().get(1).unwrap()
    }
}
#[test]
fn numeric_references_and_spaced_equals_reproduce_actual_pob_attribute_loss() {
    for warm in [false, true] {
        let reader = Reader::new(warm);
        for (xml, expected) in [
            ("<Build level='6&#48;'/>", Some("6")),
            ("<Build level='6&#x30;'/>", Some("6")),
            ("<Build level = '60'/>", None),
            ("<Build level= '60'/>", None),
            ("<Build level\t='60'/>", None),
        ] {
            let root = reader.root(xml);
            let attrs: Table = root.get("attrib").unwrap();
            assert_eq!(
                attrs.get::<Option<String>>("level").unwrap().as_deref(),
                expected
            );
            assert!(xml_compat::validate(xml).is_err());
        }
        // The suffix matching can overwrite a correctly supplied semantic field.
        let xml = "<Build level='60' hidden-level='6'/>";
        let attrs: Table = reader.root(xml).get("attrib").unwrap();
        assert_eq!(attrs.get::<String>("level").unwrap(), "6");
        assert!(xml_compat::validate(xml).is_err());
    }
}
#[test]
fn quoted_delimiters_and_literal_attribute_whitespace_preserve_proven_mismatches() {
    for warm in [false, true] {
        let reader = Reader::new(warm);
        let invalid = "<Root label='a>b'/>";
        assert!(reader.read(invalid).1.is_some());
        assert!(xml_compat::validate(invalid).is_err());
        for raw in ["a\nb", "a\rb", "a\tb"] {
            let xml = format!("<Root label='{raw}'/>");
            let attrs: Table = reader.root(&xml).get("attrib").unwrap();
            assert_eq!(
                attrs.get::<String>("label").unwrap(),
                raw,
                "PoB does not perform standard XML attribute normalization"
            );
            xml_compat::validate(&xml).unwrap();
            assert!(xml_compat::validate_native(&xml).is_err());
        }
    }
}
#[test]
fn cdata_fragmentation_and_comment_stripping_are_rejected_without_changing_literal_cdata() {
    for warm in [false, true] {
        let reader = Reader::new(warm);
        for xml in [
            "<Item>before<![CDATA[middle]]>after</Item>",
            "<Item><![CDATA[before]]>after</Item>",
            "<Item>before<![CDATA[after]]></Item>",
            "<Item><![CDATA[before]]><![CDATA[after]]></Item>",
        ] {
            assert!(
                reader.root(xml).raw_len() > 1,
                "PoB keeps fragments separate: {xml}"
            );
            assert!(xml_compat::validate(xml).is_err());
        }
        let stripped = "<Item><![CDATA[a<!--lost-->b]]></Item>";
        assert_eq!(reader.root(stripped).get::<String>(1).unwrap(), "ab");
        assert!(xml_compat::validate(stripped).is_err());
        let literal = "<Item><!-- level = '6&#48;' --><![CDATA[Use &#48; or level = \"60\". a > b.]]><!-- kept outside content --></Item>";
        xml_compat::validate(literal).unwrap();
        let root = reader.root(literal);
        assert_eq!(root.raw_len(), 1);
        assert_eq!(
            root.get::<String>(1).unwrap(),
            "Use &#48; or level = \"60\". a > b."
        );
    }
}
#[test]
fn supported_named_entities_prose_and_processing_instructions_match_actual_parser() {
    for warm in [false, true] {
        let reader = Reader::new(warm);
        let xml = "<?xml version='1.0'?><!-- level = '6&#48;' --><Root\n label='A &amp; B &gt; C &lt; D &quot; E &apos; F &amp;#48;'\n>Prose: a > b and level = '60'.</Root>";
        xml_compat::validate(xml).unwrap();
        let root = reader.root(xml);
        let attrs: Table = root.get("attrib").unwrap();
        assert_eq!(
            attrs.get::<String>("label").unwrap(),
            "A & B > C < D \" E ' F &#48;"
        );
        assert_eq!(
            root.get::<String>(1).unwrap(),
            "Prose: a > b and level = '60'."
        );
    }
}

#[test]
fn leading_utf8_bom_becomes_the_first_node_before_the_build_root_upstream() {
    for warm in [false, true] {
        let reader = Reader::new(warm);
        for xml in [
            "\u{feff}<PathOfBuilding2/>",
            "\u{feff}<?xml version='1.0'?><PathOfBuilding2/>",
            "\u{feff}<!-- ignored --><PathOfBuilding2/>",
        ] {
            let (nodes, error) = reader.read(xml);
            assert!(error.is_none());
            let nodes = nodes.unwrap();
            assert_eq!(nodes.raw_len(), 2);
            assert_eq!(nodes.get::<String>(1).unwrap(), "\u{feff}");
            let root: Table = nodes.get(2).unwrap();
            assert_eq!(root.get::<String>("elem").unwrap(), "PathOfBuilding2");
            assert!(xml_compat::validate(xml).is_err());
            assert!(xml_compat::validate_native(xml).is_err());
        }
        // A BOM-shaped character inside literal text is not a document prefix.
        let xml = "<Notes><![CDATA[\u{feff}]]></Notes>";
        xml_compat::validate_native(xml).unwrap();
        assert_eq!(reader.root(xml).get::<String>(1).unwrap(), "\u{feff}");
    }
}
