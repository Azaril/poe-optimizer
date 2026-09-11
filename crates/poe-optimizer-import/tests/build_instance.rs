use poe_optimizer_core::build_identity::{BuildLineage, InstanceId, ItemRecordId, SkillSetId};
use poe_optimizer_import::{
    ImportFormat, ImportedBuild,
    build_instance::{
        AuthoredInstanceId as I, ImportedBuildInstance, InstanceImportError, InstanceImportLimits,
        ProjectionKind, ProjectionState, SourceRole,
    },
    decode_build,
    skill_source::{SkillSourceKind, SkillSourceUse},
    source_xml::PobContentEntry,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

fn import(xml: &str, lineage: u8) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([lineage; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
fn records(
    build: &ImportedBuildInstance,
    name: &str,
) -> Vec<poe_optimizer_import::build_instance::SourceOccurrenceId> {
    build
        .occurrences()
        .iter()
        .filter(|o| o.name() == name)
        .map(|o| o.id())
        .collect()
}

#[test]
fn all_five_originals_retain_every_source_element_and_distinct_authored_instances() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/builds/breadth-20260908");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("index.json")).unwrap()).unwrap();
    let mut totals = [0usize; 4];
    for (index, entry) in manifest["builds"].as_array().unwrap().iter().enumerate() {
        let bytes = std::fs::read(directory.join(entry["xml"].as_str().unwrap())).unwrap();
        let hash = format!("{:x}", Sha256::digest(&bytes));
        assert_eq!(hash, entry["xml_sha256"]);
        let decoded = decode_build(&bytes).unwrap();
        let build = ImportedBuildInstance::from_decoded(
            decoded,
            BuildLineage::from_bytes([index as u8; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        assert_eq!(build.source_xml().as_bytes(), bytes);
        assert_eq!(build.source_sha256(), hash);
        let document = roxmltree::Document::parse(std::str::from_utf8(&bytes).unwrap()).unwrap();
        let elements: Vec<_> = document
            .descendants()
            .filter(roxmltree::Node::is_element)
            .collect();
        assert_eq!(build.occurrences().len(), elements.len());
        for (occurrence, raw) in build.occurrences().iter().zip(&elements) {
            assert_eq!(occurrence.range(), raw.range());
            assert_eq!(
                build.source_fragment(occurrence.id()).unwrap(),
                &build.source_xml()[raw.range()]
            );
            match raw.parent_element() {
                Some(parent) => assert_eq!(
                    build
                        .occurrence(occurrence.parent().unwrap())
                        .unwrap()
                        .range(),
                    parent.range()
                ),
                None => assert!(occurrence.parent().is_none()),
            }
        }
        let mut identifiers = BTreeSet::new();
        for binding in build.instances() {
            assert!(identifiers.insert(binding.instance().instance_id()));
            assert_eq!(
                build.binding(binding.instance()).unwrap().source(),
                binding.source()
            );
            let source = build.occurrence(binding.source()).unwrap();
            let expected_name = match binding.instance() {
                I::SkillSet(_) => {
                    totals[0] += 1;
                    "SkillSet"
                }
                I::SkillGroup(_) => {
                    totals[1] += 1;
                    "Skill"
                }
                I::SkillEntry(_) => {
                    totals[2] += 1;
                    "Gem"
                }
                I::ItemRecord(_) => {
                    totals[3] += 1;
                    "Item"
                }
                I::ItemSet(_) => "ItemSet",
                I::ItemSlotUse(_) => source.name(),
                I::PassiveSpec(_) => "Spec",
                I::ConfigSet(_) => "ConfigSet",
            };
            assert_eq!(source.name(), expected_name);
        }
        // Compare source-only views, not effective selection or numerical output.
        assert_eq!(
            build.project_skills().unwrap().source_xml(),
            build.source_xml()
        );
        assert_eq!(
            build.project_items().unwrap().source_xml(),
            build.source_xml()
        );
        assert!(build.project_configuration().is_ok());
        assert_eq!(
            build.allocator_state().last_issued() as usize,
            identifiers.len()
        );
    }
    assert_eq!(totals, [15, 200, 541, 116]);
}

#[test]
fn duplicate_external_ids_and_same_item_references_do_not_merge_instances() {
    let xml = r#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1"><Skill><Gem nameSpec="X"/></Skill></SkillSet><SkillSet id="1.0"><Skill><Gem nameSpec="X"/></Skill></SkillSet></Skills><Items><Item id="26">A</Item><Item id="26">B</Item><ItemSet id="2"><Slot name="Ring 1" itemId="26"/><Slot name="Ring 2" itemId="26"/></ItemSet><ItemSet id="2"/></Items></PathOfBuilding2>"#;
    let build = import(xml, 1);
    let skill_sets: Vec<_> = build
        .instances()
        .iter()
        .filter(|b| matches!(b.instance(), I::SkillSet(_)))
        .collect();
    assert_eq!(skill_sets.len(), 2);
    assert_eq!(
        build
            .attribute(skill_sets[0].source(), "id")
            .unwrap()
            .unwrap()
            .raw(),
        "1"
    );
    assert_eq!(
        build
            .attribute(skill_sets[1].source(), "id")
            .unwrap()
            .unwrap()
            .raw(),
        "1.0"
    );
    let item_records: Vec<_> = build
        .instances()
        .iter()
        .filter(|b| matches!(b.instance(), I::ItemRecord(_)))
        .collect();
    let uses: Vec<_> = build
        .instances()
        .iter()
        .filter(|b| matches!(b.instance(), I::ItemSlotUse(_)))
        .collect();
    assert_eq!(item_records.len(), 2);
    assert_eq!(uses.len(), 2);
    assert_ne!(uses[0].instance(), uses[1].instance());
    for usage in uses {
        assert_eq!(
            build
                .attribute(usage.source(), "itemId")
                .unwrap()
                .unwrap()
                .raw(),
            "26"
        );
    }
    // No winner is inferred: actual item loading can reject a raw duplicate.
    assert!(
        build
            .source_fragment(item_records[0].source())
            .unwrap()
            .contains(">A<")
    );
    assert!(
        build
            .source_fragment(item_records[1].source())
            .unwrap()
            .contains(">B<")
    );
}

#[test]
fn failed_configuration_does_not_erase_authored_sets_or_other_sections() {
    for xml in [
        r#"<PathOfBuilding2><Config><ConfigSet id="1"/><ConfigSet id="1"/></Config><Items><Item id="4"/></Items></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><Config><ConfigSet id="1"/></Config><Config><ConfigSet id="2"/></Config><Items><Item id="4"/></Items></PathOfBuilding2>"#,
        r#"<PathOfBuilding2><Config><ConfigSet id="1"><Input name="x" number="1"/><Input name="x" number="2"/></ConfigSet></Config><Items><Item id="4"/></Items></PathOfBuilding2>"#,
    ] {
        let build = import(xml, 2);
        assert!(build.project_configuration().is_err());
        assert!(build.projections().iter().any(|state| matches!(
            state,
            ProjectionState::Unavailable {
                projection: ProjectionKind::Configuration,
                ..
            }
        )));
        assert_eq!(
            build
                .instances()
                .iter()
                .filter(|b| matches!(b.instance(), I::ConfigSet(_)))
                .count(),
            records(&build, "ConfigSet").len()
        );
        assert!(
            build
                .instances()
                .iter()
                .any(|b| matches!(b.instance(), I::ItemRecord(_)))
        );
        assert_eq!(build.source_xml(), xml);
    }
}

#[test]
fn unknown_positional_gem_gets_an_instance_but_unknown_subtrees_do_not() {
    let build = import(
        r#"<PathOfBuilding2><Skills><Skill><FutureGem nameSpec="Caller ability"><Opaque><Gem/></Opaque></FutureGem></Skill><Opaque><Skill><Gem/></Skill></Opaque></Skills></PathOfBuilding2>"#,
        3,
    );
    let entries: Vec<_> = build
        .instances()
        .iter()
        .filter(|b| matches!(b.instance(), I::SkillEntry(_)))
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        build.occurrence(entries[0].source()).unwrap().role(),
        SourceRole::Skill {
            kind: SkillSourceKind::Unknown,
            usage: SkillSourceUse::GemInstance
        }
    );
    assert_eq!(records(&build, "Gem").len(), 2);
}

#[test]
fn namespaces_and_legacy_layouts_remain_source_without_invented_default_sets() {
    let build = import(
        r#"<PathOfBuilding2><Skills><Skill><Gem/></Skill></Skills><Items><Slot name="Weapon 1" itemId="7"/></Items><Spec><Sockets><Socket nodeId="3" itemId="7"/></Sockets></Spec><Config><Input name="future" string="x"/></Config></PathOfBuilding2>"#,
        4,
    );
    assert_eq!(
        build
            .instances()
            .iter()
            .filter(|b| matches!(
                b.instance(),
                I::SkillSet(_) | I::ItemSet(_) | I::ConfigSet(_)
            ))
            .count(),
        0
    );
    assert_eq!(
        build
            .instances()
            .iter()
            .filter(|b| matches!(b.instance(), I::ItemSlotUse(_)))
            .count(),
        2
    );
    assert_eq!(
        build
            .instances()
            .iter()
            .filter(|b| matches!(b.instance(), I::PassiveSpec(_)))
            .count(),
        1
    );
    let namespaced = import(
        r#"<PathOfBuilding2 xmlns:x="urn:future"><Skills><SkillSet id="1"><Skill><Gem/></Skill></SkillSet></Skills><Config><ConfigSet id="1"/></Config><x:Items><x:Item id="2"/></x:Items></PathOfBuilding2>"#,
        5,
    );
    assert!(namespaced.instances().is_empty());
    assert!(
        namespaced
            .occurrences()
            .iter()
            .all(|o| o.has_namespace_context())
    );
    assert_eq!(records(&namespaced, "Item").len(), 1);
    let local = import(
        r#"<PathOfBuilding2><Skills><Skill xmlns:p="urn:unused"><Gem xmlns=""/></Skill><Skill><Gem/></Skill></Skills></PathOfBuilding2>"#,
        13,
    );
    assert_eq!(local.instances().len(), 2);
    assert!(local.instances().iter().all(|binding| {
        !local
            .occurrence(binding.source())
            .unwrap()
            .has_namespace_context()
    }));
    assert_eq!(records(&local, "Gem").len(), 2);
}

#[test]
fn snapshot_clones_share_storage_but_independent_imports_have_distinct_instances() {
    let xml = "<PathOfBuilding2><Skills><Skill><Gem/></Skill></Skills></PathOfBuilding2>";
    let first = import(xml, 6);
    let clone = first.clone();
    let independent = import(xml, 7);
    assert!(first.shares_storage_with(&clone));
    assert!(!first.shares_storage_with(&independent));
    assert_eq!(first.source_xml().as_ptr(), clone.source_xml().as_ptr());
    assert_eq!(
        first.instances()[0].instance(),
        clone.instances()[0].instance()
    );
    assert_ne!(
        first.instances()[0].instance(),
        independent.instances()[0].instance()
    );
    assert_eq!(
        first.instances()[0].source(),
        independent.instances()[0].source()
    );
    assert!(
        first
            .binding(independent.instances()[0].instance())
            .is_err()
    );
    assert!(
        first
            .occurrence(independent.instances()[0].source())
            .is_ok()
    );
    drop(first);
    assert_eq!(clone.source_xml(), xml);
}

#[test]
fn membership_rejects_wrong_domain_lineage_and_unissued_ids() {
    let build = import(
        "<PathOfBuilding2><Items><Item id='1'/></Items></PathOfBuilding2>",
        8,
    );
    let item = build.instances()[0].instance().instance_id();
    assert!(matches!(
        build.binding(I::SkillSet(SkillSetId::from_instance_id(item))),
        Err(InstanceImportError::ForeignInstance)
    ));
    let never_issued = InstanceId::from_parts(build.lineage(), 99).unwrap();
    assert!(
        build
            .binding(I::ItemRecord(ItemRecordId::from_instance_id(never_issued)))
            .is_err()
    );
    let changed = import(
        "<PathOfBuilding2><Items><Item id='2'/></Items></PathOfBuilding2>",
        8,
    );
    assert!(build.occurrence(changed.occurrences()[0].id()).is_err());
}

#[test]
fn item_text_resets_and_ranges_reuse_ordered_source_projection() {
    let xml = r#"<PathOfBuilding2><Items><Item id="1">First<ModRange id="1" range="0.2"/><![CDATA[Second]]><!--ignored--><ModRange id="2" range="0.8"/>Third</Item></Items></PathOfBuilding2>"#;
    let build = import(xml, 9);
    let projection = build.project_items().unwrap();
    let item = &projection.containers()[0].children()[0];
    let consumed = item.ordered_content().consumed();
    assert_eq!(consumed.len(), 5);
    assert!(matches!(&consumed[0],PobContentEntry::Text{text,..} if text=="First"));
    assert!(matches!(
        consumed[1],
        PobContentEntry::Element { child_index: 0 }
    ));
    assert!(matches!(&consumed[2],PobContentEntry::Text{text,..} if text=="Second"));
    assert!(matches!(
        consumed[3],
        PobContentEntry::Element { child_index: 1 }
    ));
    assert!(matches!(&consumed[4],PobContentEntry::Text{text,..} if text=="Third"));
    assert_eq!(build.source_xml(), xml);
}

#[test]
fn attribute_values_are_exact_and_only_named_entities_decode_once() {
    let xml = "<PathOfBuilding2><Skills><Skill label='A\nB&amp;lt;'/></Skills></PathOfBuilding2>";
    let build = import(xml, 10);
    let attribute = build
        .attribute(records(&build, "Skill")[0], "label")
        .unwrap()
        .unwrap();
    assert_eq!(attribute.raw(), "A\nB&amp;lt;");
    assert_eq!(attribute.decoded(), "A\nB&lt;");
    let numeric = import(
        "<PathOfBuilding2><Skills><Skill label='&#65;'/></Skills></PathOfBuilding2>",
        11,
    );
    assert!(
        numeric
            .attribute(records(&numeric, "Skill")[0], "label")
            .is_err()
    );
    assert!(
        numeric
            .projections()
            .iter()
            .any(|s| matches!(s, ProjectionState::Unavailable { .. }))
    );
    assert_eq!(numeric.instances().len(), 1);
}

#[test]
fn public_decode_dto_is_revalidated_instead_of_trusting_its_hash() {
    let mut decoded = decode_build(b"<PathOfBuilding2/>").unwrap();
    decoded.xml = "<PathOfBuilding2><Items/></PathOfBuilding2>".into();
    assert!(matches!(
        ImportedBuildInstance::from_decoded(
            decoded,
            BuildLineage::from_bytes([1; 16]),
            InstanceImportLimits::default()
        ),
        Err(InstanceImportError::SourceIdentityMismatch)
    ));
    for xml in ["<NotPoB/>", "<PathOfBuilding2>"] {
        let forged = ImportedBuild {
            xml: xml.into(),
            format: ImportFormat::RawXml,
            sha256: format!("{:x}", Sha256::digest(xml.as_bytes())),
        };
        assert!(matches!(
            ImportedBuildInstance::from_decoded(
                forged,
                BuildLineage::from_bytes([1; 16]),
                InstanceImportLimits::default()
            ),
            Err(InstanceImportError::Import(_))
        ));
    }
    let oversized = ImportedBuild {
        xml: "x".repeat(poe_optimizer_import::MAX_XML_BYTES + 1),
        format: ImportFormat::RawXml,
        sha256: String::new(),
    };
    assert!(matches!(
        ImportedBuildInstance::from_decoded(
            oversized,
            BuildLineage::from_bytes([1; 16]),
            InstanceImportLimits::default()
        ),
        Err(InstanceImportError::Import(_))
    ));
}

#[test]
fn resource_limits_reject_the_whole_import_and_do_not_truncate_source() {
    let xml=b"<PathOfBuilding2><Skills><SkillSet id='1'><Skill><Gem/></Skill></SkillSet></Skills></PathOfBuilding2>";
    let base = InstanceImportLimits::default();
    for limits in [
        InstanceImportLimits {
            max_occurrences: 0,
            ..base
        },
        InstanceImportLimits {
            max_occurrences: 4,
            ..base
        },
        InstanceImportLimits {
            max_instances: 2,
            ..base
        },
        InstanceImportLimits {
            max_depth: 3,
            ..base
        },
        InstanceImportLimits {
            max_attributes: 0,
            ..base
        },
        InstanceImportLimits {
            max_metadata_bytes: 1,
            ..base
        },
    ] {
        assert!(matches!(
            ImportedBuildInstance::from_decoded(
                decode_build(xml).unwrap(),
                BuildLineage::from_bytes([1; 16]),
                limits
            ),
            Err(InstanceImportError::ResourceLimit(_))
        ));
    }
    let exact = InstanceImportLimits {
        max_occurrences: 5,
        max_instances: 3,
        max_depth: 4,
        max_attributes: 1,
        ..base
    };
    let valid = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([1; 16]),
        exact,
    )
    .unwrap();
    assert_eq!(valid.source_xml().as_bytes(), xml);
    assert_eq!(valid.instances().len(), 3);
}

#[test]
fn reports_have_exact_wire_identities_and_no_embedded_source_copy() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<ImportedBuildInstance>();
    let build = import(
        "<PathOfBuilding2><Items><Item id='1'>Private source payload</Item></Items></PathOfBuilding2>",
        12,
    );
    let report = serde_json::to_value(build.report()).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["lineage"], "0c".repeat(16));
    assert_eq!(report["revision"], "0000000000000000");
    assert_eq!(
        report["instances"][0]["instance"]["id"]["local"],
        "0000000000000001"
    );
    assert!(
        !serde_json::to_string(&report)
            .unwrap()
            .contains("Private source payload")
    );
}
