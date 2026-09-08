use poe_optimizer_import::{build_source, root_admission};
use roxmltree::Document;

const AUXILIARY: &str = r#"<Import lastRealm="PoE2" lastLeague="A &amp; B" lastAccountHash="account" lastCharacterHash="character" importLink="https://invalid.example/persist-only" useGeneratedItemText="true" exportParty="false"/>
<TreeView searchStr="cooldown" zoomX="-971.25" zoomY="13.5" zoomLevel="8" showStatDifferences="true"/>
<Party destination="All" append="false" ShowAdvanceTools="false"/>
<Calcs><Section id="Offence" subsection="Damage" collapsed="true"/><Section id="unrecognized-layout" collapsed="false"/><Input name="skill_number" number="14"/><Input name="misc_buffMode" string="EFFECTIVE"/><Input name="showMinion" boolean="true"/></Calcs>"#;
fn xml(auxiliary: &str) -> String {
    format!("<PathOfBuilding2>{auxiliary}</PathOfBuilding2>")
}
fn accepted(source: &str) -> bool {
    root_admission::validate_main(&Document::parse(source).unwrap()).is_ok()
}
#[test]
fn accepted_auxiliary_projection_keeps_exact_source_and_distinct_calcs_selector() {
    for source in [xml(AUXILIARY), xml(AUXILIARY).replace('\n', "\r\n")] {
        let document = Document::parse(&source).unwrap();
        let admission = root_admission::validate_main(&document).unwrap();
        assert_eq!(admission.projection().source_xml(), source);
        assert_eq!(admission.projection().sections().len(), 4);
        assert_eq!(
            admission.projection().sections()[3].records()[2]
                .scalar_input()
                .unwrap()
                .value(),
            &poe_optimizer_core::options::Scalar::Number(14.0)
        );
        assert_eq!(
            admission.projection().source_sha256(),
            build_source::project(&document).unwrap().source_sha256()
        );
    }
}
#[test]
fn main_only_proof_does_not_depend_on_config_defaults_or_catalog() {
    for config in [
        "",
        "<Config/>",
        "<Config><Input name=\"customMods\" string=\"\"/></Config>",
        "<Config activeConfigSet=\"1\"><ConfigSet id=\"1\"/></Config>",
    ] {
        assert!(accepted(&xml(&format!("{config}{AUXILIARY}"))));
    }
}
#[test]
fn all_four_source_modes_and_boolean_values_are_preserved() {
    for mode in ["UNBUFFED", "BUFFED", "COMBAT", "EFFECTIVE"] {
        for value in ["true", "false"] {
            assert!(accepted(&xml(&format!(
                r#"<Calcs><Input name="misc_buffMode" string="{mode}"/><Input name="showMinion" boolean="{value}"/></Calcs>"#
            ))));
        }
    }
    for section in [
        "<Import/>",
        "<Party/>",
        "<Calcs/>",
        "<TreeView/>",
        "<TreeView zoomX=\"2\"/>",
        "<TreeView zoomLevel=\"1e300\"/>",
    ] {
        assert!(accepted(&xml(section)), "{section}");
    }
}
#[test]
fn unknown_or_effectful_auxiliary_fields_never_gain_admission() {
    for section in [
        r#"<Import exportParty="true"/>"#,
        r#"<Import callback="download"/>"#,
        "<Import><Anything/></Import>",
        "<Party><ImportedBuffs/></Party>",
        "<Party><ExportedBuffs/></Party>",
        r#"<Party actor="friend"/>"#,
        "<TreeView><Selection/></TreeView>",
        r#"<Calcs unknown="true"/>"#,
        "<Calcs><Unknown/></Calcs>",
        r#"<Calcs><Input name="enemyLevel" number="90"/></Calcs>"#,
        r#"<Calcs><Input name="misc_enemyLevel" number="90"/></Calcs>"#,
        r#"<Calcs><Input name="customMods" string="+1 to Strength"/></Calcs>"#,
        r#"<Calcs><Section collapsed="false"/></Calcs>"#,
        r#"<Calcs><Section id="Offence"><Input name="showMinion" boolean="true"/></Section></Calcs>"#,
        "<Unknown/>",
        "<Spec/>",
        "<Party>text</Party>",
    ] {
        let source = xml(section);
        assert!(
            build_source::project_xml(&source).is_ok(),
            "projection: {section}"
        );
        assert!(!accepted(&source), "admission: {section}");
    }
}
#[test]
fn bad_types_duplicates_namespaces_and_non_ascii_root_text_reject() {
    for section in [
        r#"<Import exportParty="FALSE"/>"#,
        r#"<Party append="1"/>"#,
        r#"<TreeView zoomX="NaN"/>"#,
        r#"<TreeView zoomY="inf"/>"#,
        r#"<Calcs><Input name="skill_number" number="0"/></Calcs>"#,
        r#"<Calcs><Input name="skill_number" number="1.5"/></Calcs>"#,
        r#"<Calcs><Input name="skill_number" number="4294967296"/></Calcs>"#,
        r#"<Calcs><Input name="skill_number" string="1"/></Calcs>"#,
        r#"<Calcs><Input name="showMinion" string="true"/></Calcs>"#,
        r#"<Calcs><Input name="misc_buffMode" string="MAIN"/></Calcs>"#,
        r#"<Calcs><Input name="skill_number" number="1" string="2"/></Calcs>"#,
        r#"<Calcs><Input name="showMinion" boolean="false"/><Input name="showMinion" boolean="true"/></Calcs>"#,
        "<Party/><Party/>",
        "<Build/><Build/>",
        r#"<Party xmlns="foreign"/>"#,
        r#"<Party xmlns:unused="foreign"/>"#,
        r#"<Calcs><Section xmlns:x="foreign" x:id="Offence"/></Calcs>"#,
        "\u{a0}",
    ] {
        assert!(!accepted(&xml(section)), "{section}");
    }
    assert!(!accepted("<PathOfBuilding2 rootSetting=\"ignored\"/>"));
}
#[test]
fn auxiliary_attribute_whitespace_does_not_expand_native_lexical_coverage() {
    let source = xml("<TreeView searchStr=\"a\tb\"/>");
    assert!(build_source::project_xml(&source).is_ok());
    assert!(!accepted(&source));
}
#[test]
fn core_payloads_still_require_their_own_complete_validators() {
    let source = xml("<Skills unsupportedMechanic=\"true\"><Unknown/></Skills>");
    assert!(
        accepted(&source),
        "root admission must not pretend to validate core payloads"
    );
    let data = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    assert!(
        poe_optimizer_import::controlled_build::SourceBuildTemplate::parse(source, data.package())
            .is_err()
    );
}
