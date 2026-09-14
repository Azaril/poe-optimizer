use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::{
        INSTANCE_IMPORT_SCHEMA, ImportedBuildInstance, InstanceImportLimits, ProjectionKind,
        ProjectionState, SourceOccurrenceId,
    },
    decode_build,
    owned_source::*,
    source_xml::{PobContentEntry, SourceContentKind},
};

fn import(xml: &str, lineage: u8) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([lineage; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
fn collect(source: &ImportedBuildInstance) -> SourceProjectEvidence<'_> {
    SourceProjectEvidence::collect(source, SourceEvidenceLimits::default()).unwrap()
}
fn ids(source: &ImportedBuildInstance, name: &str) -> Vec<SourceOccurrenceId> {
    source
        .occurrences()
        .iter()
        .filter(|row| row.name() == name)
        .map(|row| row.id())
        .collect()
}
fn key<'a>(
    parent: SourceOccurrenceId,
    element: &'a str,
    attribute: &'a str,
    value: &'a str,
) -> SourceKeyQuery<'a> {
    SourceKeyQuery {
        parent: Some(parent),
        element: SourceQName {
            namespace: None,
            local: element,
        },
        attribute: SourceQName {
            namespace: None,
            local: attribute,
        },
        value,
    }
}

#[test]
fn all_rows_parents_bindings_and_independent_source_sets_survive() {
    let xml = r#"<PathOfBuilding2><Build level="42"/><Tree><Spec title="first"/><Spec title="second"/></Tree><Items><Item id="26">item text</Item><ItemSet id="2"><Slot name="Ring 1" itemId="26"/><Slot name="Ring 2" itemId="26"/></ItemSet><ItemSet id="1"/></Items><Skills><SkillSet id="4"><Skill><Gem nameSpec="one"/><Surprise nameSpec="two"/></Skill></SkillSet><SkillSet id="3"/></Skills><Config><ConfigSet id="9"/><ConfigSet id="7"/></Config><Unknown>retained</Unknown></PathOfBuilding2>"#;
    let source = import(xml, 1);
    let evidence = collect(&source);
    assert_eq!(evidence.source_xml(), xml);
    assert_eq!(evidence.source_xml().as_ptr(), source.source_xml().as_ptr());
    assert_eq!(evidence.rows().len(), source.occurrences().len());
    assert_eq!(evidence.coverage().len(), source.occurrences().len());
    for (ordinal, occurrence) in source.occurrences().iter().enumerate() {
        let row = evidence.row(occurrence.id()).unwrap();
        assert_eq!(row.occurrence().id(), occurrence.id());
        assert_eq!(evidence.rows()[ordinal].occurrence().id(), occurrence.id());
        assert_eq!(
            evidence
                .row_at_range(occurrence.range())
                .unwrap()
                .occurrence()
                .id(),
            occurrence.id()
        );
        assert_eq!(evidence.coverage()[ordinal].source, occurrence.id());
        assert_eq!(
            evidence.source_fragment(occurrence.id()).unwrap(),
            &xml[occurrence.range()]
        );
        if let Some(parent) = occurrence.parent() {
            assert!(
                evidence
                    .children(parent)
                    .unwrap()
                    .contains(&occurrence.id())
            );
        }
    }
    for binding in source.instances() {
        let row = evidence.row_for_instance(binding.instance()).unwrap();
        assert_eq!(row.occurrence().id(), binding.source());
        assert_eq!(row.authored_instance(), Some(binding.instance()));
    }
    for (section, tag, set, expected) in [
        (SourceSectionKind::Items, "Items", "ItemSet", vec!["2", "1"]),
        (
            SourceSectionKind::Skills,
            "Skills",
            "SkillSet",
            vec!["4", "3"],
        ),
        (
            SourceSectionKind::Config,
            "Config",
            "ConfigSet",
            vec!["9", "7"],
        ),
    ] {
        assert_eq!(evidence.sections(section), ids(&source, tag));
        let values: Vec<_> = ids(&source, set)
            .iter()
            .map(|id| {
                evidence
                    .row(*id)
                    .unwrap()
                    .attribute("id")
                    .unwrap()
                    .decoded()
                    .unwrap()
            })
            .collect();
        assert_eq!(values, expected);
    }
    assert_eq!(
        evidence.sections(SourceSectionKind::Tree),
        ids(&source, "Tree")
    );
    let surprise = ids(&source, "Surprise")[0];
    assert!(
        evidence
            .row(surprise)
            .unwrap()
            .authored_instance()
            .is_some()
    );
    assert_eq!(
        evidence.coverage()[surprise.ordinal() as usize].recognition,
        SourceRecognition::UnknownElementOrContext
    );
    let uses = ids(&source, "Slot");
    assert_ne!(
        evidence.row(uses[0]).unwrap().authored_instance(),
        evidence.row(uses[1]).unwrap().authored_instance()
    );
}

#[test]
fn unavailable_typed_config_projection_keeps_all_lexical_fields_and_records() {
    let source = import(
        r#"<PathOfBuilding2><Config><ConfigSet id="1"><Input name="flag" boolean="not-canonical"/><Input name="amount" number="9007199254740993"/><Input name="amount" number="NaN"/><Placeholder name="amount" number="0"/><Extra value="preserved"/></ConfigSet></Config></PathOfBuilding2>"#,
        2,
    );
    assert!(source.projections().iter().any(|state| matches!(
        state,
        ProjectionState::Unavailable {
            projection: ProjectionKind::Configuration,
            ..
        }
    )));
    let evidence = collect(&source);
    assert_eq!(evidence.rows().len(), source.occurrences().len());
    let rows = ids(&source, "Input");
    assert_eq!(
        evidence
            .row(rows[0])
            .unwrap()
            .attribute("boolean")
            .unwrap()
            .decoded()
            .unwrap(),
        "not-canonical"
    );
    assert_eq!(
        evidence
            .row(rows[1])
            .unwrap()
            .attribute("number")
            .unwrap()
            .decoded()
            .unwrap(),
        "9007199254740993"
    );
    assert_eq!(
        evidence
            .row(rows[2])
            .unwrap()
            .attribute("number")
            .unwrap()
            .decoded()
            .unwrap(),
        "NaN"
    );
    assert!(
        matches!(evidence.lookup_key(key(ids(&source, "ConfigSet")[0], "Input", "name", "amount")).unwrap(), SourceKeyLookup::Ambiguous(matches) if matches.len() == 2)
    );
    assert!(
        evidence
            .coverage()
            .iter()
            .all(|row| row.lexical_issues.is_empty())
    );
    let extra = ids(&source, "Extra")[0];
    assert_eq!(
        evidence.coverage()[extra.ordinal() as usize].recognition,
        SourceRecognition::UnknownElementOrContext
    );
}

#[test]
fn exact_duplicate_keys_do_not_coerce_numeric_spellings_or_missing_and_empty() {
    let source = import(
        r#"<PathOfBuilding2><Items><ItemSet id="1"/><ItemSet id="1.0"/><ItemSet id="1"/><ItemSet id=""/><ItemSet/></Items><Items><ItemSet id="1"/></Items></PathOfBuilding2>"#,
        3,
    );
    let evidence = collect(&source);
    let parents = ids(&source, "Items");
    let sets = ids(&source, "ItemSet");
    let SourceKeyLookup::Ambiguous(matches) = evidence
        .lookup_key(key(parents[0], "ItemSet", "id", "1"))
        .unwrap()
    else {
        panic!("duplicate keys must stay ambiguous")
    };
    assert_eq!(
        matches.iter().map(|v| v.occurrence).collect::<Vec<_>>(),
        vec![sets[0], sets[2]]
    );
    for (parent, value, expected) in [
        (parents[0], "1.0", sets[1]),
        (parents[0], "", sets[3]),
        (parents[1], "1", sets[5]),
    ] {
        let SourceKeyLookup::Unique(reference) = evidence
            .lookup_key(key(parent, "ItemSet", "id", value))
            .unwrap()
        else {
            panic!("expected exact unique key")
        };
        assert_eq!(reference.occurrence, expected);
        assert_eq!(
            evidence.attribute(reference).unwrap().decoded().unwrap(),
            value
        );
    }
    assert!(evidence.row(sets[4]).unwrap().attribute("id").is_none());
    assert_eq!(
        evidence
            .lookup_key(key(parents[0], "ItemSet", "id", "1e0"))
            .unwrap(),
        SourceKeyLookup::Missing
    );
}

#[test]
fn undecodable_key_sibling_prevents_unique_or_missing_and_retains_evidence() {
    let source = import(
        r#"<PathOfBuilding2><Items><ItemSet id="one"/><ItemSet id="&#111;ne"/><ItemSet id="other"/></Items></PathOfBuilding2>"#,
        4,
    );
    let evidence = collect(&source);
    let parent = ids(&source, "Items")[0];
    let sets = ids(&source, "ItemSet");
    for (value, known) in [("one", 1), ("absent", 0)] {
        let SourceKeyLookup::Unresolved {
            matches,
            unavailable,
        } = evidence
            .lookup_key(key(parent, "ItemSet", "id", value))
            .unwrap()
        else {
            panic!("unknown key cannot prove absence or uniqueness")
        };
        assert_eq!(matches.len(), known);
        assert_eq!(unavailable.len(), 1);
        assert_eq!(unavailable[0].occurrence, sets[1]);
        let attribute = evidence.attribute(unavailable[0]).unwrap();
        assert_eq!(attribute.raw(), "&#111;ne");
        assert!(attribute.decoded().is_err());
        let issue = &evidence.coverage()[sets[1].ordinal() as usize].lexical_issues[0];
        assert_eq!(issue.field, SourceLexicalField::Attribute(0));
        assert_eq!(
            issue.byte_offset,
            attribute.decoded().unwrap_err().byte_offset
        );
    }
}

#[test]
fn source_text_keeps_entities_whitespace_namespaces_and_ordered_content() {
    let xml = "<PathOfBuilding2><Items><Item id=\"x\" custom=\" a\n&amp;amp; b \" missing=\"\"> one&amp;two<!--note--> three<![CDATA[ <raw> ]]><Nested unique=\"child-token\"/>tail</Item></Items><Other xmlns:x=\"urn:x\"><x:Entry id=\"plain\" x:id=\"qualified\"/></Other></PathOfBuilding2>";
    let source = import(xml, 5);
    let evidence = collect(&source);
    let item = evidence.row(ids(&source, "Item")[0]).unwrap();
    assert_eq!(item.attribute("custom").unwrap().raw(), " a\n&amp;amp; b ");
    assert_eq!(
        item.attribute("custom").unwrap().decoded().unwrap(),
        " a\n&amp; b "
    );
    assert_eq!(item.attribute("missing").unwrap().decoded().unwrap(), "");
    let SourceContentEvidence::Available(content) = item.content() else {
        panic!("known lexical content")
    };
    assert!(
        content
            .fragments()
            .iter()
            .any(|fragment| fragment.kind() == SourceContentKind::Comment
                && fragment.raw() == "<!--note-->")
    );
    assert!(
        content
            .fragments()
            .iter()
            .any(|fragment| fragment.kind() == SourceContentKind::Cdata)
    );
    let text: Vec<_> = content
        .consumed()
        .iter()
        .filter_map(|entry| {
            if let PobContentEntry::Text { text, .. } = entry {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(text, vec!["one&two three", " <raw> ", "tail"]);
    // Element references serialize no child XML; the child retains its own row.
    assert!(!serde_json::to_string(item).unwrap().contains("child-token"));
    let other = ids(&source, "Other")[0];
    let entry = ids(&source, "Entry")[0];
    let row = evidence.row(entry).unwrap();
    assert_eq!(row.attributes().len(), 2);
    assert_eq!(row.attribute("id").unwrap().decoded().unwrap(), "plain");
    assert_eq!(
        evidence.coverage()[entry.ordinal() as usize].recognition,
        SourceRecognition::NamespaceContext
    );
    let query = SourceKeyQuery {
        parent: Some(other),
        element: SourceQName {
            namespace: Some("urn:x"),
            local: "Entry",
        },
        attribute: SourceQName {
            namespace: Some("urn:x"),
            local: "id",
        },
        value: "qualified",
    };
    assert!(
        matches!(evidence.lookup_key(query).unwrap(), SourceKeyLookup::Unique(reference) if reference.occurrence == entry)
    );
    assert!(
        evidence
            .source_fragment(other)
            .unwrap()
            .contains("xmlns:x=\"urn:x\"")
    );
}

#[test]
fn unavailable_content_keeps_raw_source_children_and_unrelated_rows() {
    let source = import(
        "<PathOfBuilding2><Unknown>bad &#65;<Child/> tail</Unknown><Build level=\"9\"/></PathOfBuilding2>",
        6,
    );
    let evidence = collect(&source);
    let unknown = ids(&source, "Unknown")[0];
    let SourceContentEvidence::Unavailable(error) = evidence.row(unknown).unwrap().content() else {
        panic!("numeric entity remains unsupported")
    };
    assert!(evidence.source_fragment(unknown).unwrap().contains("&#65;"));
    assert_eq!(evidence.children(unknown).unwrap(), ids(&source, "Child"));
    assert!(
        evidence.coverage()[unknown.ordinal() as usize]
            .lexical_issues
            .iter()
            .any(|issue| issue.field == SourceLexicalField::Content
                && issue.byte_offset == error.byte_offset)
    );
    let build = evidence.row(ids(&source, "Build")[0]).unwrap();
    assert_eq!(build.attribute("level").unwrap().decoded().unwrap(), "9");
}

#[test]
fn evidence_identity_binds_hash_lineage_revision_and_allocator() {
    let xml = "<PathOfBuilding2><Items><Item id=\"1\"/></Items></PathOfBuilding2>";
    let source = import(xml, 7);
    let same_xml = import(xml, 8);
    let foreign = import(
        "<PathOfBuilding2><Items><Item id=\"2\"/></Items></PathOfBuilding2>",
        7,
    );
    let evidence = collect(&source);
    let identity = evidence.identity();
    assert_eq!(identity.instance_import_schema, INSTANCE_IMPORT_SCHEMA);
    assert_eq!(identity.source_sha256, source.source_sha256());
    assert_eq!(identity.lineage, source.lineage());
    assert_eq!(identity.revision, source.revision());
    assert_eq!(identity.allocator, *source.allocator_state());
    assert_eq!(identity.source_bytes, xml.len());
    assert_ne!(identity, collect(&same_xml).identity());
    // Occurrence IDs deliberately address source content, not import lineages.
    assert_eq!(source.occurrences()[0].id(), same_xml.occurrences()[0].id());
    assert!(
        evidence
            .row_for_instance(same_xml.instances()[0].instance())
            .is_err()
    );
    assert!(evidence.row(foreign.occurrences()[0].id()).is_err());
    assert!(
        evidence
            .lookup_key(key(foreign.occurrences()[0].id(), "Items", "id", "1"))
            .is_err()
    );
    assert!(
        evidence
            .attribute(SourceAttributeRef {
                occurrence: source.occurrences()[0].id(),
                index: 99
            })
            .is_err()
    );
    assert!(evidence.row_at_range(0..1).is_none());
}

#[test]
fn conservative_raw_budget_is_charged_once_across_nested_elements() {
    let xml = format!(
        "<PathOfBuilding2>{}t{}</PathOfBuilding2>",
        "<Unknown>".repeat(20),
        "</Unknown>".repeat(20)
    );
    let source = import(&xml, 9);
    let limits = SourceEvidenceLimits {
        max_total_text_bytes: xml.len() + 1,
        ..SourceEvidenceLimits::default()
    };
    let evidence = SourceProjectEvidence::collect(&source, limits).unwrap();
    assert_eq!(evidence.rows().len(), 21);
    let too_small = SourceEvidenceLimits {
        max_total_text_bytes: xml.len(),
        ..limits
    };
    assert!(matches!(
        SourceProjectEvidence::collect(&source, too_small),
        Err(SourceEvidenceError::ResourceLimit("total text bytes"))
    ));
}

#[test]
fn every_resource_ceiling_fails_explicitly_instead_of_reporting_lexical_unavailable() {
    let source = import(
        "<PathOfBuilding2><Build a=\"ab\" b=\"cd\"><Unknown>text</Unknown></Build></PathOfBuilding2>",
        10,
    );
    let base = SourceEvidenceLimits::default();
    for (limits, expected) in [
        (
            SourceEvidenceLimits {
                max_occurrences: 1,
                ..base
            },
            "occurrences",
        ),
        (
            SourceEvidenceLimits {
                max_attributes: 1,
                ..base
            },
            "attributes",
        ),
        (
            SourceEvidenceLimits {
                max_fragments: 1,
                ..base
            },
            "fragments",
        ),
        (
            SourceEvidenceLimits {
                max_depth: 1,
                ..base
            },
            "depth",
        ),
        (
            SourceEvidenceLimits {
                max_value_bytes: 1,
                ..base
            },
            "value bytes",
        ),
        (
            SourceEvidenceLimits {
                max_total_text_bytes: source.source_xml().len() - 1,
                ..base
            },
            "total text bytes",
        ),
        (
            SourceEvidenceLimits {
                max_index_entries: 1,
                ..base
            },
            "index entries",
        ),
    ] {
        assert!(
            matches!(SourceProjectEvidence::collect(&source, limits), Err(SourceEvidenceError::ResourceLimit(name)) if name == expected),
            "expected {expected}"
        );
    }
    let attributes = import("<PathOfBuilding2 a=\"ab\"/>", 10);
    let limits = SourceEvidenceLimits {
        max_total_text_bytes: attributes.source_xml().len() + 1,
        ..base
    };
    assert!(matches!(
        SourceProjectEvidence::collect(&attributes, limits),
        Err(SourceEvidenceError::ResourceLimit("total text bytes"))
    ));
    let content = import("<PathOfBuilding2>ab</PathOfBuilding2>", 10);
    let limits = SourceEvidenceLimits {
        max_value_bytes: 1,
        ..base
    };
    assert!(matches!(
        SourceProjectEvidence::collect(&content, limits),
        Err(SourceEvidenceError::ResourceLimit("value bytes"))
    ));
}

#[test]
fn lexical_failure_does_not_reset_the_shared_fragment_budget() {
    let source = import(
        "<PathOfBuilding2><Bad>&#65;</Bad><Good>ok</Good></PathOfBuilding2>",
        12,
    );
    let limits = SourceEvidenceLimits {
        max_fragments: 4,
        ..SourceEvidenceLimits::default()
    };
    let evidence = SourceProjectEvidence::collect(&source, limits).unwrap();
    assert!(matches!(
        evidence.row(ids(&source, "Bad")[0]).unwrap().content(),
        SourceContentEvidence::Unavailable(_)
    ));
    assert!(matches!(
        evidence.row(ids(&source, "Good")[0]).unwrap().content(),
        SourceContentEvidence::Available(_)
    ));
    let too_small = SourceEvidenceLimits {
        max_fragments: 3,
        ..limits
    };
    assert!(matches!(
        SourceProjectEvidence::collect(&source, too_small),
        Err(SourceEvidenceError::ResourceLimit("fragments"))
    ));
}

#[test]
fn successful_entity_decoding_keeps_raw_attribute_reservation_charged() {
    let value = "&amp;".repeat(16);
    let xml = format!("<PathOfBuilding2 a=\"{value}\"/>");
    let source = import(&xml, 13);
    let limits = SourceEvidenceLimits {
        max_total_text_bytes: xml.len() + value.len(),
        ..SourceEvidenceLimits::default()
    };
    let evidence = SourceProjectEvidence::collect(&source, limits).unwrap();
    assert_eq!(
        evidence.rows()[0]
            .attribute("a")
            .unwrap()
            .decoded()
            .unwrap(),
        "&".repeat(16)
    );
    let too_small = SourceEvidenceLimits {
        max_total_text_bytes: limits.max_total_text_bytes - 1,
        ..limits
    };
    assert!(matches!(
        SourceProjectEvidence::collect(&source, too_small),
        Err(SourceEvidenceError::ResourceLimit("total text bytes"))
    ));
}

#[test]
fn limits_must_be_positive_and_within_hard_ceilings() {
    let source = import("<PathOfBuilding2/>", 11);
    let setters: [fn(&mut SourceEvidenceLimits, usize); 7] = [
        |v, n| v.max_occurrences = n,
        |v, n| v.max_attributes = n,
        |v, n| v.max_fragments = n,
        |v, n| v.max_depth = n,
        |v, n| v.max_value_bytes = n,
        |v, n| v.max_total_text_bytes = n,
        |v, n| v.max_index_entries = n,
    ];
    for set in setters {
        for value in [0, usize::MAX] {
            let mut limits = SourceEvidenceLimits::default();
            set(&mut limits, value);
            assert!(matches!(
                SourceProjectEvidence::collect(&source, limits),
                Err(SourceEvidenceError::InvalidLimit(_))
            ));
        }
    }
}

#[test]
fn all_five_originals_have_one_coverage_entry_per_preserved_occurrence() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/builds/breadth-20260908");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("index.json")).unwrap()).unwrap();
    for (index, entry) in manifest["builds"].as_array().unwrap().iter().enumerate() {
        let bytes = std::fs::read(directory.join(entry["xml"].as_str().unwrap())).unwrap();
        let source = ImportedBuildInstance::from_decoded(
            decode_build(&bytes).unwrap(),
            BuildLineage::from_bytes([index as u8; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence = collect(&source);
        assert_eq!(
            evidence.identity().source_sha256,
            entry["xml_sha256"].as_str().unwrap()
        );
        assert_eq!(evidence.rows().len(), source.occurrences().len());
        assert_eq!(evidence.coverage().len(), source.occurrences().len());
        assert_eq!(evidence.source_xml().as_bytes(), bytes);
        for (row, coverage) in evidence.rows().iter().zip(evidence.coverage()) {
            assert_eq!(row.occurrence().id(), coverage.source);
            assert_eq!(row.attributes().len(), row.occurrence().attributes().len());
        }
        for binding in source.instances() {
            assert_eq!(
                evidence
                    .row_for_instance(binding.instance())
                    .unwrap()
                    .authored_instance(),
                Some(binding.instance())
            );
        }
    }
}
