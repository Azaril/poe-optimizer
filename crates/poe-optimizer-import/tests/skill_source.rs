use poe_optimizer_import::{
    MAX_XML_BYTES, MAX_XML_NODES, build_source,
    skill_source::{
        self, SkillAttributeRole as R, SkillAttributeSyntax as S, SkillDiagnosticCode as D,
        SkillSourceKind as K, SkillSourceNode, SkillSourceUse as U,
    },
};
use roxmltree::Document;
use sha2::{Digest, Sha256};

fn wrap(contents: &str) -> String {
    format!("<PathOfBuilding2>{contents}</PathOfBuilding2>")
}
fn skills(contents: &str) -> String {
    wrap(&format!("<Skills>{contents}</Skills>"))
}
fn has(node: &SkillSourceNode<'_>, code: D) -> bool {
    node.diagnostics().iter().any(|d| d.code() == code)
}
fn walk<'a, 'input>(node: &'a SkillSourceNode<'input>, out: &mut Vec<&'a SkillSourceNode<'input>>) {
    out.push(node);
    for child in node.children() {
        walk(child, out);
    }
}
fn view(node: &SkillSourceNode<'_>, name: &str) -> (R, S) {
    let a = node
        .attributes()
        .iter()
        .find(|a| node.element().attributes()[a.source_index()].name() == name)
        .unwrap();
    (a.role(), a.syntax())
}
fn check_source(node: &SkillSourceNode<'_>, xml: &str) {
    let e = node.element();
    assert_eq!(e.source_xml(), &xml[e.source_range()]);
    assert_eq!(e.source_xml().as_ptr(), xml[e.source_range()].as_ptr());
    for a in e.attributes() {
        assert_eq!(a.value().raw(), &xml[a.value().range()]);
        assert_eq!(a.value().raw().as_ptr(), xml[a.value().range()].as_ptr());
    }
    for a in node.attributes() {
        assert!(a.source_index() < e.attributes().len());
    }
    for d in node.diagnostics() {
        assert!(e.source_range().contains(&d.byte_offset()));
        assert!(!d.message().is_empty());
    }
    for pair in node.children().windows(2) {
        assert!(pair[0].element().source_range().end <= pair[1].element().source_range().start);
    }
    for child in node.children() {
        check_source(child, xml);
    }
}

#[test]
fn all_five_immutable_builds_preserve_every_saved_set_group_and_instance() {
    // Independently inventoried in runs/root-container-skill-inventory.json;
    // hashes also pinned by the immutable intake index, not this projector.
    let cases = [
        (
            1,
            "e3c0d0b40fa682260a1713acb03d52d720f4b769ac91b0501cbe2a84dc468194",
            vec!["1"],
            vec![19],
            vec![62],
            "1",
        ),
        (
            2,
            "91366bd82a9afdd12ae7d8f695508a1b8d99116567010e082a9d31c4c4d4f631",
            vec!["1", "3", "2", "4", "5", "6"],
            vec![11, 11, 14, 14, 16, 18],
            vec![18, 22, 27, 28, 33, 46],
            "6",
        ),
        (
            3,
            "d3f7c72092f77481d3d1c5e38ec71d8730d607f19c659fc05b8a5db3bbbf9490",
            vec!["1"],
            vec![14],
            vec![62],
            "1",
        ),
        (
            4,
            "62d760d326e21291cd1024f20660bd5046043c4e242b761df5d9a764be61e711",
            vec!["1"],
            vec![15],
            vec![62],
            "1",
        ),
        (
            5,
            "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089",
            vec!["2", "3", "4", "5", "6", "1"],
            vec![5, 7, 12, 14, 15, 15],
            vec![11, 12, 28, 32, 47, 51],
            "4",
        ),
    ];
    let mut totals = [0usize; 3];
    for (line, hash, ids, groups, instances, active) in cases {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../tests/fixtures/builds/breadth-20260908/build-{line:02}.xml"
        ));
        let original = std::fs::read(&path).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(&original)), hash);
        let xml = std::str::from_utf8(&original).unwrap();
        let p = skill_source::project_xml(xml).unwrap();
        assert_eq!(p.source_sha256(), hash);
        assert_eq!(p.source_xml().as_bytes(), original);
        assert_eq!(p.source_xml().as_ptr(), xml.as_ptr());
        assert_eq!(p.containers().len(), 1);
        let container = &p.containers()[0];
        assert_eq!(container.source_use(), U::Container);
        assert_eq!(
            container
                .element()
                .attribute("activeSkillSet")
                .unwrap()
                .decoded(),
            active
        );
        assert_eq!(
            container
                .children()
                .iter()
                .map(|s| s.element().attribute("id").unwrap().decoded())
                .collect::<Vec<_>>(),
            ids
        );
        for (index, set) in container.children().iter().enumerate() {
            assert_eq!(set.kind(), K::SkillSet);
            assert_eq!(set.source_use(), U::SavedSet);
            assert_eq!(set.children().len(), groups[index]);
            assert_eq!(
                set.children()
                    .iter()
                    .map(|g| g.children().len())
                    .sum::<usize>(),
                instances[index]
            );
            totals[0] += 1;
            totals[1] += groups[index];
            totals[2] += instances[index];
            for group in set.children() {
                assert_eq!(group.kind(), K::Skill);
                for gem in group.children() {
                    assert_eq!(gem.kind(), K::Gem);
                    assert!(gem.children().is_empty());
                }
            }
        }
        check_source(container, xml);
        assert_eq!(std::fs::read(path).unwrap(), original);
    }
    assert_eq!(totals, [15, 200, 541]);
}

#[test]
fn absent_defaults_and_empty_id_fields_are_not_invented_or_resolved() {
    let missing = skill_source::project_xml("<PathOfBuilding2/>").unwrap();
    assert!(missing.containers().is_empty());
    let xml = skills(
        "<SkillSet><Skill><Gem/><Gem gemId='' skillId='CallerEffect' nameSpec='Caller Label'/></Skill></SkillSet>",
    );
    let p = skill_source::project_xml(&xml).unwrap();
    let c = &p.containers()[0];
    assert!(c.element().attribute("activeSkillSet").is_none());
    assert!(c.element().attribute("defaultGemLevel").is_none());
    let set = &c.children()[0];
    assert!(set.element().attribute("id").is_none());
    assert!(has(set, D::MissingSetId));
    let group = &set.children()[0];
    assert!(group.element().attribute("enabled").is_none());
    assert!(group.element().attribute("mainActiveSkill").is_none());
    let empty = &group.children()[0];
    assert!(empty.element().attributes().is_empty());
    assert!(has(empty, D::MissingGemIdentity));
    let explicit = &group.children()[1];
    assert_eq!(explicit.element().attribute("gemId").unwrap().decoded(), "");
    assert_eq!(
        explicit.element().attribute("skillId").unwrap().decoded(),
        "CallerEffect"
    );
    assert!(explicit.element().attribute("level").is_none());
    assert!(!has(explicit, D::MissingGemIdentity));
    let json = serde_json::to_value(&p).unwrap();
    assert!(json.get("source_xml").is_none());
    assert!(json.get("active_skill_set").is_none());
    assert!(json.get("resolved_skill").is_none());
}

#[test]
fn exact_attributes_preserve_unicode_entities_and_literal_whitespace() {
    let raw = "\tA\r\nB\rC\n\u{00e9}\u{65e5}\u{672c}\u{8a9e}&lt;&gt;&amp;&apos;&quot;&amp;lt;";
    let decoded = "\tA\r\nB\rC\n\u{00e9}\u{65e5}\u{672c}\u{8a9e}<>&'\"&lt;";
    let xml = skills(&format!(
        "<SkillSet id=' 01 ' title=\"{raw}\"><Skill label='{raw}'><Gem nameSpec=\"{raw}\" note='{raw}'/></Skill></SkillSet>"
    ));
    let p = skill_source::project_xml(&xml).unwrap();
    let set = &p.containers()[0].children()[0];
    let group = &set.children()[0];
    let gem = &group.children()[0];
    for (node, name) in [
        (set, "title"),
        (group, "label"),
        (gem, "nameSpec"),
        (gem, "note"),
    ] {
        let value = node.element().attribute(name).unwrap();
        assert_eq!(value.raw(), raw);
        assert_eq!(value.decoded(), decoded);
    }
    assert_eq!(set.element().attribute("id").unwrap().raw(), " 01 ");
    check_source(&p.containers()[0], &xml);
    let normalized = xml.replace("\r\n", "\n");
    assert_ne!(
        p.source_sha256(),
        skill_source::project_xml(&normalized)
            .unwrap()
            .source_sha256()
    );
}

#[test]
fn duplicates_legacy_groups_and_unknown_containers_remain_ordered() {
    let xml = wrap(
        "<Skills activeSkillSet='99'><Skill active='true'/><Future><SkillSet id='1'/></Future><SkillSet id='1'/><SkillSet id='01'/><SkillSet id='-0'/><SkillSet id='0'/></Skills><Skills><SkillSet id='1'/></Skills>",
    );
    let p = skill_source::project_xml(&xml).unwrap();
    assert_eq!(p.containers().len(), 2);
    let first = &p.containers()[0];
    assert_eq!(
        first
            .children()
            .iter()
            .map(SkillSourceNode::kind)
            .collect::<Vec<_>>(),
        [
            K::Skill,
            K::Unknown,
            K::SkillSet,
            K::SkillSet,
            K::SkillSet,
            K::SkillSet
        ]
    );
    assert!(has(first, D::MixedLegacyAndSavedSets));
    assert!(has(&first.children()[0], D::LegacyDirectGroup));
    assert_eq!(first.children()[1].source_use(), U::Ignored);
    assert_eq!(first.children()[1].children()[0].kind(), K::Unknown);
    assert_eq!(first.children()[1].children()[0].source_use(), U::Ignored);
    assert!(has(&first.children()[3], D::DuplicateNumericSetId));
    assert!(has(&first.children()[5], D::DuplicateNumericSetId));
    assert!(!has(&first.children()[2], D::DuplicateNumericSetId));
    assert!(has(&p.containers()[1], D::DuplicateSkillsContainer));
    assert!(!has(
        &p.containers()[1].children()[0],
        D::DuplicateNumericSetId
    ));
    assert_eq!(
        first
            .element()
            .attribute("activeSkillSet")
            .unwrap()
            .decoded(),
        "99"
    );
    check_source(first, &xml);
}

#[test]
fn positional_gem_and_minion_map_quirks_do_not_rename_unknown_nodes() {
    let xml = skills(
        "<Skill><CallerGem gemId='external' variantId='V'><StatSetIndex grantedEffect='E' index='2'/><MinionSkillIndexLookup grantedEffect='E'><CallerMap skillIndex='1' statSetIndex='3'/><MinionSkillIndexMap skillIndex='01' statSetIndex='4'/><OtherMap/></MinionSkillIndexLookup></CallerGem></Skill>",
    );
    let p = skill_source::project_xml(&xml).unwrap();
    let gem = &p.containers()[0].children()[0].children()[0];
    assert_eq!(gem.kind(), K::Unknown);
    assert_eq!(gem.source_use(), U::GemInstance);
    assert!(has(gem, D::UnknownElement));
    assert!(has(gem, D::PositionalGemChild));
    assert_eq!(view(gem, "gemId"), (R::CatalogIdentity, S::Text));
    assert_eq!(gem.children()[0].kind(), K::StatSetIndex);
    let lookup = &gem.children()[1];
    assert_eq!(lookup.kind(), K::MinionSkillIndexLookup);
    assert_eq!(lookup.children()[0].kind(), K::Unknown);
    assert_eq!(lookup.children()[0].source_use(), U::MinionIndexMap);
    assert!(has(&lookup.children()[0], D::PositionalMinionMapChild));
    assert!(has(&lookup.children()[1], D::DuplicateLookupKey));
    assert!(has(&lookup.children()[2], D::MissingLookupKey));
    assert_eq!(lookup.children()[0].element().name(), "CallerMap");
    check_source(&p.containers()[0], &xml);
}

#[test]
fn main_calcs_and_repeated_lookup_maps_preserve_every_occurrence() {
    let xml = skills(
        "<Skill><Gem statSetIndex='5' statSetIndexCalcs='nil'><StatSetIndex grantedEffect='A&amp;B' index='2'/><StatSetCalcsIndex grantedEffect='A&amp;B' index='3'/><StatSetIndex grantedEffect='A&amp;B' index='4'/><MinionSkillIndexLookup grantedEffect='A&amp;B'><MinionSkillIndexMap skillIndex='1' statSetIndex='2'/></MinionSkillIndexLookup><MinionSkillIndexLookupCalcs grantedEffect='A&amp;B'><MinionSkillIndexMap skillIndex='1' statSetIndex='3'/></MinionSkillIndexLookupCalcs><MinionSkillIndexLookup grantedEffect='A&amp;B'><MinionSkillIndexMap skillIndex='2' statSetIndex='4'/></MinionSkillIndexLookup><StatSetIndex/></Gem></Skill>",
    );
    let p = skill_source::project_xml(&xml).unwrap();
    let gem = &p.containers()[0].children()[0].children()[0];
    assert_eq!(
        view(gem, "statSetIndex"),
        (R::LegacyResetSelection, S::FiniteDecimal)
    );
    assert_eq!(
        view(gem, "statSetIndexCalcs"),
        (R::LegacyResetSelection, S::OtherNumericText)
    );
    assert!(has(gem, D::LegacyStatSetAttributeReset));
    assert_eq!(gem.children().len(), 7);
    assert!(has(&gem.children()[2], D::DuplicateLookupKey));
    assert!(has(&gem.children()[5], D::DuplicateLookupKey));
    assert!(!has(&gem.children()[1], D::DuplicateLookupKey));
    assert!(!has(&gem.children()[4], D::DuplicateLookupKey));
    assert_eq!(
        gem.children()[3].children()[0]
            .element()
            .attribute("skillIndex")
            .unwrap()
            .decoded(),
        "1"
    );
    assert_eq!(
        gem.children()[5].children()[0]
            .element()
            .attribute("skillIndex")
            .unwrap()
            .decoded(),
        "2"
    );
    assert!(has(&gem.children()[6], D::MissingLookupKey));
}

#[test]
fn global_effect_flags_weapon_slot_and_authored_booleans_are_distinct() {
    let xml = skills(
        "<Skill active='true' enabled='false' slot='Weapon 2 Swap' source='Item:7:Caller' includeInFullDPS='nil'><Gem nameSpec='unresolved label' enableGlobal1='false' enableGlobal2='true' enabled='TRUE' count='nil' future='retained'/></Skill>",
    );
    let p = skill_source::project_xml(&xml).unwrap();
    let group = &p.containers()[0].children()[0];
    let gem = &group.children()[0];
    assert_eq!(view(group, "slot"), (R::WeaponSlot, S::Text));
    assert_eq!(view(group, "source"), (R::GrantSource, S::Text));
    assert_eq!(view(group, "active"), (R::Enabled, S::CanonicalBoolean));
    assert_eq!(
        group.element().attribute("enabled").unwrap().decoded(),
        "false"
    );
    assert_eq!(
        view(gem, "enableGlobal1"),
        (R::GlobalEffectToggle, S::CanonicalBoolean)
    );
    assert_eq!(
        view(gem, "enableGlobal2"),
        (R::GlobalEffectToggle, S::CanonicalBoolean)
    );
    assert_eq!(view(gem, "enabled"), (R::Enabled, S::OtherBooleanText));
    assert_eq!(view(gem, "count"), (R::Count, S::OtherNumericText));
    assert!(has(gem, D::NonCanonicalBoolean));
    assert!(has(gem, D::NumericOutsideFiniteDecimal));
    assert!(has(gem, D::UnknownAttribute));
    assert_eq!(
        gem.element().attribute("future").unwrap().decoded(),
        "retained"
    );
}

#[test]
fn numeric_syntax_diagnostics_keep_raw_nonfinite_hex_and_nil_text() {
    for (raw, syntax) in [
        ("  +.5\t", S::FiniteDecimal),
        ("-0", S::FiniteDecimal),
        ("1.7976931348623157e308", S::FiniteDecimal),
        ("4.9406564584124654e-324", S::FiniteDecimal),
        ("1e999", S::NonFiniteDecimal),
        ("NaN", S::OtherNumericText),
        ("nil", S::OtherNumericText),
        ("0x10", S::OtherNumericText),
        ("1e", S::OtherNumericText),
        ("", S::OtherNumericText),
    ] {
        let xml = skills(&format!("<Skill><Gem level='{raw}'/></Skill>"));
        let p = skill_source::project_xml(&xml).unwrap();
        let gem = &p.containers()[0].children()[0].children()[0];
        assert_eq!(view(gem, "level"), (R::Level, syntax), "{raw}");
        assert_eq!(gem.element().attribute("level").unwrap().raw(), raw);
        assert_eq!(
            has(gem, D::NumericOutsideFiniteDecimal),
            syntax != S::FiniteDecimal
        );
    }
}

#[test]
fn namespaces_unknown_descendants_and_text_never_gain_known_roles() {
    let xml = wrap(
        "<Skills xmlns='urn:foreign'><SkillSet id='1'><Skill xmlns=''><Gem nameSpec='still blocked'/></Skill></SkillSet></Skills><Skills><Skill>authored text<Gem><Future><StatSetIndex grantedEffect='E' index='2'/></Future><MinionSkillIndexLookup grantedEffect='E'>map text</MinionSkillIndexLookup></Gem><Gem xmlns='urn:foreign' nameSpec='foreign'/></Skill></Skills><Other><Skills/></Other>",
    );
    let p = skill_source::project_xml(&xml).unwrap();
    assert_eq!(p.containers().len(), 2);
    let mut nodes = Vec::new();
    walk(&p.containers()[0], &mut nodes);
    for node in nodes {
        assert_eq!(node.kind(), K::Unknown);
        assert_eq!(node.source_use(), U::NamespaceUnknown);
        assert!(has(node, D::NamespaceContext));
    }
    let group = &p.containers()[1].children()[0];
    assert!(has(group, D::UnhandledText));
    assert_eq!(
        group.children()[0].children()[0].children()[0].kind(),
        K::Unknown
    );
    assert_eq!(
        group.children()[0].children()[0].children()[0].source_use(),
        U::Ignored
    );
    assert!(has(&group.children()[0].children()[1], D::UnhandledText));
    assert_eq!(group.children()[1].source_use(), U::NamespaceUnknown);
    check_source(&p.containers()[0], &xml);
    check_source(&p.containers()[1], &xml);
}

#[test]
fn container_set_group_and_instance_limits_apply_across_saved_containers() {
    let accepted = wrap(&"<Skills/>".repeat(skill_source::MAX_SKILLS_CONTAINERS));
    assert!(skill_source::project_xml(&accepted).is_ok());
    let rejected = wrap(&"<Skills/>".repeat(skill_source::MAX_SKILLS_CONTAINERS + 1));
    assert!(
        skill_source::project_xml(&rejected)
            .unwrap_err()
            .reason
            .contains("container count")
    );
    for (fragment, limit, message) in [
        ("<SkillSet/>", skill_source::MAX_SKILL_SETS, "set count"),
        ("<Skill/>", skill_source::MAX_SKILL_GROUPS, "group count"),
    ] {
        let half = fragment.repeat(limit / 2);
        let allowed = wrap(&format!("<Skills>{half}</Skills><Skills>{half}</Skills>"));
        assert!(skill_source::project_xml(&allowed).is_ok());
        let denied = allowed.replacen("</Skills>", &format!("{fragment}</Skills>"), 1);
        assert!(
            skill_source::project_xml(&denied)
                .unwrap_err()
                .reason
                .contains(message)
        );
    }
    let gems = "<Gem/>".repeat(skill_source::MAX_GEM_INSTANCES);
    let allowed = skills(&format!("<Skill>{gems}</Skill>"));
    assert!(skill_source::project_xml(&allowed).is_ok());
    let denied = allowed.replace("</Skill>", "<Gem/></Skill>");
    assert!(
        skill_source::project_xml(&denied)
            .unwrap_err()
            .reason
            .contains("instance count")
    );
}

#[test]
fn unknown_payloads_obey_node_depth_and_string_allocation_bounds() {
    let allowed = skills(&"<Future/>".repeat(skill_source::MAX_SKILL_SOURCE_NODES - 1));
    assert!(skill_source::project_xml(&allowed).is_ok());
    let denied = skills(&"<Future/>".repeat(skill_source::MAX_SKILL_SOURCE_NODES));
    assert!(
        skill_source::project_xml(&denied)
            .unwrap_err()
            .reason
            .contains("node count")
    );
    let nested = |n: usize| {
        skills(&format!(
            "{}{}",
            "<Future>".repeat(n),
            "</Future>".repeat(n)
        ))
    };
    assert!(skill_source::project_xml(&nested(skill_source::MAX_SKILL_SOURCE_DEPTH - 1)).is_ok());
    assert!(
        skill_source::project_xml(&nested(skill_source::MAX_SKILL_SOURCE_DEPTH))
            .unwrap_err()
            .reason
            .contains("depth")
    );
    let value = "x".repeat(build_source::MAX_SOURCE_VALUE_BYTES);
    let allowed = skills(&format!("<Skill><Gem nameSpec='{value}'/></Skill>"));
    assert!(skill_source::project_xml(&allowed).is_ok());
    let denied = skills(&format!("<Skill><Gem nameSpec='{value}x'/></Skill>"));
    assert!(
        skill_source::project_xml(&denied)
            .unwrap_err()
            .reason
            .contains("value byte")
    );
    let large = format!("<Gem nameSpec='{value}'/>");
    let count = build_source::MAX_PROJECTED_ATTRIBUTE_BYTES / build_source::MAX_SOURCE_VALUE_BYTES;
    assert!(
        skill_source::project_xml(&skills(&format!(
            "<Skill>{}</Skill>",
            large.repeat(count - 1)
        )))
        .is_ok()
    );
    assert!(
        skill_source::project_xml(&skills(&format!(
            "<Skill>{}</Skill>",
            large.repeat(count + 1)
        )))
        .unwrap_err()
        .reason
        .contains("attribute byte")
    );
}

#[test]
fn attribute_and_diagnostic_limits_bound_many_small_values() {
    let names = [
        "nameSpec",
        "skillId",
        "gemId",
        "variantId",
        "note",
        "level",
        "quality",
        "enabled",
        "enableGlobal1",
        "enableGlobal2",
        "count",
        "corrupted",
        "corruptLevel",
        "skillMinion",
        "skillMinionCalcs",
        "skillPart",
        "skillPartCalcs",
        "skillStageCount",
        "skillStageCountCalcs",
        "skillMineCount",
        "skillMineCountCalcs",
        "skillMinionItemSet",
        "skillMinionItemSetCalcs",
        "skillMinionSkill",
        "skillMinionSkillCalcs",
    ];
    let attrs = names
        .iter()
        .map(|name| {
            format!(
                " {name}='{}'",
                if matches!(
                    *name,
                    "enabled" | "enableGlobal1" | "enableGlobal2" | "corrupted"
                ) {
                    "true"
                } else {
                    "1"
                }
            )
        })
        .collect::<String>();
    let gem = format!("<Gem{attrs}/>");
    let count = skill_source::MAX_SKILL_SOURCE_ATTRIBUTES / names.len();
    assert!(
        skill_source::project_xml(&skills(&format!("<Skill>{}</Skill>", gem.repeat(count))))
            .is_ok()
    );
    assert!(
        skill_source::project_xml(&skills(&format!(
            "<Skill>{}</Skill>",
            gem.repeat(count + 1)
        )))
        .unwrap_err()
        .reason
        .contains("attribute count")
    );
    let attrs = (0..120).map(|n| format!(" a{n}='x'")).collect::<String>();
    let unknown = format!("<Future{attrs}/>");
    let count = skill_source::MAX_SKILL_SOURCE_DIAGNOSTICS / 121;
    assert!(skill_source::project_xml(&skills(&unknown.repeat(count))).is_ok());
    assert!(
        skill_source::project_xml(&skills(&unknown.repeat(count + 1)))
            .unwrap_err()
            .reason
            .contains("diagnostic count")
    );
}

#[test]
fn global_xml_and_lexical_guards_cannot_be_bypassed_by_external_documents() {
    for xml in [
        "",
        "<Other/>",
        "<PathOfBuilding2>",
        "<!DOCTYPE PathOfBuilding2><PathOfBuilding2/>",
        "<PathOfBuilding2 xmlns='urn:foreign'/>",
        "<PathOfBuilding2><Skills activeSkillSet = '1'/></PathOfBuilding2>",
        "<PathOfBuilding2><Skills><Skill><Gem nameSpec='&#10;'/></Skill></Skills></PathOfBuilding2>",
        "<PathOfBuilding2><Skills id='1' id='2'/></PathOfBuilding2>",
    ] {
        assert!(skill_source::project_xml(xml).is_err(), "{xml}");
    }
    let opaque = wrap(&format!("<Other>{}</Other>", "x".repeat(MAX_XML_BYTES)));
    assert!(skill_source::project_xml(&opaque).is_err());
    let nodes = wrap(&format!(
        "<Other>{}</Other>",
        "<N/>".repeat(MAX_XML_NODES as usize + 1)
    ));
    assert!(skill_source::project_xml(&nodes).is_err());
    let doc = Document::parse(&nodes).unwrap();
    assert!(skill_source::project(&doc).is_err());
    let escaped = skills("<Skill><Gem nameSpec='&#65;'/></Skill>");
    let doc = Document::parse(&escaped).unwrap();
    assert!(skill_source::project(&doc).is_err());
}
