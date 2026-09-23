//! Offline base-prefix evidence refines source layout, never whole build coverage.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{
    owned_item_layouts::*, owned_item_lines::*, owned_item_source::*, owned_mapping::*,
    owned_source::*, owned_value::WhitespacePolicy,
};
use sha2::{Digest, Sha256};
#[allow(dead_code)]
#[path = "support/owned_item_normalization.rs"]
mod support;

fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("layout-test", "v1").unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value.to_ascii_lowercase()).unwrap()
}
fn template(value: &str) -> ItemTemplateDefId {
    ItemTemplateDefId::parse(ns(), value.to_ascii_lowercase()).unwrap()
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn declarations(owner: &ItemTemplateDefId) -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::partial(
            vec![],
            vec![SchemaGap {
                subject: SchemaSubject::Definition(owner.address()),
                facet: SchemaFacet::InputSchema,
                code: key("other-item-inputs-unconverted"),
            }],
        ),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn header(id: &str, text: &str, emission: ItemEmission) -> ItemLineRule {
    ItemLineRule {
        id: key(id),
        pattern: vec![ItemPatternPart::Literal(text.into())],
        captures: vec![],
        emissions: vec![emission],
    }
}
struct Fixture {
    schema: OwnedDefinitionSchemaPackage,
    lines: ItemLinePolicyInput,
    source: ItemSourceLayoutPolicyInput,
    catalog: ItemBaseLayoutCatalog,
    policy: ItemLayoutPolicy,
}
impl Fixture {
    fn new() -> Self {
        let names = ["Alpha Tool", "Beta Vessel", "Gamma Relic", "Unrelated Base"];
        let definitions = names
            .iter()
            .map(|name| {
                let id = template(name.split(' ').next().unwrap());
                DefinitionDescriptor::ItemTemplate(DefinitionEntry {
                    id: id.clone(),
                    schema: SchemaState::Known(ItemTemplateSchema {
                        item_level: IntegerRange {
                            minimum: BoundedInteger::new(0).unwrap(),
                            maximum: BoundedInteger::new(100).unwrap(),
                        },
                        equipment_slots: DeclaredSet::complete(vec![]),
                        socket_destinations: DeclaredSet::complete(vec![]),
                        modifiers: DeclaredSet::complete(vec![]),
                        quality: QualityUseSchema {
                            presence: QualityPresence::Forbidden,
                            allowed_kinds: DeclaredSet::complete(vec![]),
                        },
                        declarations: declarations(&id),
                    }),
                })
            })
            .collect();
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
                namespace: ns(),
                release: key("release"),
                semantics_version: key("semantics"),
                definitions,
                slots: vec![],
            },
            OwnedSchemaLimits::default(),
        )
        .unwrap();
        let mut rules: Vec<_> = names
            .iter()
            .map(|name| {
                header(
                    name.split(' ').next().unwrap(),
                    name,
                    ItemEmission::Template {
                        definition: template(name.split(' ').next().unwrap()),
                    },
                )
            })
            .collect();
        for (id, text) in [
            ("rarity", "Rarity: RARE"),
            ("title", "Fixture"),
            ("implicits", "Implicits: 0"),
        ] {
            rules.push(header(
                id,
                text,
                ItemEmission::Metadata {
                    role: key("preamble"),
                },
            ));
        }
        let lines = ItemLinePolicyInput {
            schema_version: OWNED_ITEM_LINE_POLICY_VERSION,
            namespace: ns(),
            version: key("item-inputs"),
            definitions: schema.identity().clone(),
            whitespace: WhitespacePolicy::Exact,
            rules,
        };
        let checked_lines =
            OwnedItemLinePolicy::new(lines.clone(), &schema, Default::default()).unwrap();
        let pin = SourcePin {
            system: ExternalSourceSystem::PathOfBuilding2,
            revision: "test-only-layout-source".into(),
            files: vec![SourceFilePin {
                path: "test-only/bases.json".into(),
                sha256: "a".repeat(64),
            }],
        };
        let source = ItemSourceLayoutPolicyInput {
            schema_version: OWNED_ITEM_SOURCE_POLICY_VERSION,
            namespace: ns(),
            version: key("source-layout"),
            source: pin.clone(),
            item_lines: *checked_lines.identity(),
            dialect: ItemSourceDialect::PobExportedSingleTextV1,
            property_bindings: vec![],
            template_defaults: vec![],
            rule_layouts: lines
                .rules
                .iter()
                .map(|r| ItemRuleSourceLayout {
                    rule: r.id.clone(),
                    role: ItemRuleSourceRole::Header,
                })
                .collect(),
            template_layouts: names
                .iter()
                .enumerate()
                .map(|(i, name)| ItemTemplateSourceLayout {
                    template: template(name.split(' ').next().unwrap()),
                    load_index_prefix: if i == 3 {
                        ItemLoadIndexPrefix::NoGeneratedBuffMembers
                    } else {
                        ItemLoadIndexPrefix::Unresolved
                    },
                })
                .collect(),
        };
        let checked_source = ItemSourceLayoutPolicy::new(
            source.clone(),
            &checked_lines,
            &schema,
            Default::default(),
        )
        .unwrap();
        let catalog = ItemBaseLayoutCatalog {
            schema_version: 1,
            source: pin,
            bases: names[..3]
                .iter()
                .zip([
                    ItemBaseGeneratedPrefix::Absent,
                    ItemBaseGeneratedPrefix::Present,
                    ItemBaseGeneratedPrefix::Unsupported,
                ])
                .map(|(name, prefix)| ItemBaseLayoutRow {
                    source_base: (*name).into(),
                    prefix,
                })
                .collect(),
        };
        let policy = ItemLayoutPolicy {
            schema_version: 1,
            version: key("compiled-layout"),
            catalog_sha256: String::new(),
            definitions: schema.identity().clone(),
            items: *checked_lines.identity(),
            item_source: *checked_source.identity(),
            templates: names[..3]
                .iter()
                .map(|name| ItemLayoutTemplateBinding {
                    source_base: (*name).into(),
                    template: template(name.split(' ').next().unwrap()),
                    header_rule: key(name.split(' ').next().unwrap()),
                })
                .collect(),
        };
        let mut fixture = Self {
            schema,
            lines,
            source,
            catalog,
            policy,
        };
        fixture.repin();
        fixture
    }
    fn bytes(&self) -> Vec<u8> {
        let mut bytes = serde_json::to_vec(&self.catalog).unwrap();
        bytes.push(b'\n');
        bytes
    }
    fn repin(&mut self) {
        self.policy.catalog_sha256 = sha(&self.bytes());
    }
    fn checked(&self) -> (OwnedItemLinePolicy, ItemSourceLayoutPolicy) {
        let lines =
            OwnedItemLinePolicy::new(self.lines.clone(), &self.schema, Default::default()).unwrap();
        let source = ItemSourceLayoutPolicy::new(
            self.source.clone(),
            &lines,
            &self.schema,
            Default::default(),
        )
        .unwrap();
        (lines, source)
    }
    fn rebind_inputs(&mut self) {
        let lines =
            OwnedItemLinePolicy::new(self.lines.clone(), &self.schema, Default::default()).unwrap();
        self.source.item_lines = *lines.identity();
        let source = ItemSourceLayoutPolicy::new(
            self.source.clone(),
            &lines,
            &self.schema,
            Default::default(),
        )
        .unwrap();
        self.policy.items = *lines.identity();
        self.policy.item_source = *source.identity();
        self.policy.definitions = self.schema.identity().clone();
    }
    fn reject(&self) {
        let (lines, source) = self.checked();
        assert!(
            compile_owned_item_layouts(
                &lines,
                &source,
                &self.schema,
                &self.bytes(),
                &self.policy,
                ItemLayoutLimits::default()
            )
            .is_err()
        );
    }
}

fn attribute(
    base: &str,
    lines: &OwnedItemLinePolicy,
    source: &ItemSourceLayoutPolicy,
) -> ItemRangeAttribution {
    let imported = support::source(&format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nFixture\n{base}\nImplicits: 0\n</Item></Items></PathOfBuilding2>"
    ));
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    source
        .attribute(&evidence, support::item_source(&imported, "7"), lines)
        .unwrap()
}

#[test]
fn exact_absence_refines_only_bound_prefix_and_changes_real_attribution() {
    let f = Fixture::new();
    let before_schema = serde_json::to_vec(f.schema.input()).unwrap();
    let before_lines = serde_json::to_vec(&f.lines).unwrap();
    let before_source = serde_json::to_vec(&f.source).unwrap();
    let (lines, source) = f.checked();
    assert!(matches!(
        attribute("Alpha Tool", &lines, &source).report().layout,
        ItemLayoutStatus::Pending(_)
    ));
    let compiled = compile_owned_item_layouts(
        &lines,
        &source,
        &f.schema,
        &f.bytes(),
        &f.policy,
        ItemLayoutLimits::default(),
    )
    .unwrap();
    assert_eq!(compiled.receipt.catalog_sha256, sha(&f.bytes()));
    assert_eq!(compiled.receipt.before, *source.identity());
    assert_eq!(compiled.receipt.bases, 3);
    assert_eq!(compiled.receipt.absent, 1);
    assert_eq!(compiled.receipt.present, 1);
    assert_eq!(compiled.receipt.unsupported, 1);
    assert_eq!(compiled.receipt.refined_prefixes, 1);
    assert!(compiled.receipt.work_used > 0);
    for (i, layout) in compiled.item_source.template_layouts.iter().enumerate() {
        assert_eq!(
            layout.load_index_prefix,
            if i == 0 || i == 3 {
                ItemLoadIndexPrefix::NoGeneratedBuffMembers
            } else {
                ItemLoadIndexPrefix::Unresolved
            }
        );
    }
    let mut unchanged = compiled.item_source.clone();
    unchanged.version = f.source.version.clone();
    unchanged.template_layouts = f.source.template_layouts.clone();
    assert_eq!(unchanged, f.source);
    let checked = ItemSourceLayoutPolicy::new(
        compiled.item_source.clone(),
        &lines,
        &f.schema,
        Default::default(),
    )
    .unwrap();
    assert!(matches!(
        attribute("Alpha Tool", &lines, &checked).report().layout,
        ItemLayoutStatus::Proven
    ));
    for name in ["Beta Vessel", "Gamma Relic"] {
        assert!(matches!(
            attribute(name, &lines, &checked).report().layout,
            ItemLayoutStatus::Pending(_)
        ));
    }
    assert!(matches!(
        attribute("Unrelated Base", &lines, &checked)
            .report()
            .layout,
        ItemLayoutStatus::Proven
    ));
    assert_eq!(serde_json::to_vec(f.schema.input()).unwrap(), before_schema);
    assert_eq!(serde_json::to_vec(&f.lines).unwrap(), before_lines);
    assert_eq!(serde_json::to_vec(&f.source).unwrap(), before_source);
    let replay = compile_owned_item_layouts(
        &lines,
        &source,
        &f.schema,
        &f.bytes(),
        &f.policy,
        ItemLayoutLimits::default(),
    )
    .unwrap();
    assert_eq!(compiled.item_source, replay.item_source);
    assert_eq!(
        serde_json::to_value(compiled.receipt).unwrap(),
        serde_json::to_value(replay.receipt).unwrap()
    );
}

#[test]
fn provenance_and_each_input_identity_are_exact_bindings() {
    for change in 0..7 {
        let mut f = Fixture::new();
        match change {
            0 => f.policy.catalog_sha256 = "0".repeat(64),
            1 => f.policy.items = digest_owned("stale-items", &false, 1024).unwrap(),
            2 => f.policy.item_source = digest_owned("stale-source", &false, 1024).unwrap(),
            3 => f.policy.definitions.content_sha256 = "0".repeat(64),
            4 => {
                f.catalog.source.revision = "other-revision".into();
                f.repin();
            }
            5 => {
                f.catalog.source.system = ExternalSourceSystem::PathOfBuilding1;
                f.repin();
            }
            6 => {
                f.catalog.source.files[0].sha256 = "b".repeat(64);
                f.repin();
            }
            _ => unreachable!(),
        }
        f.reject();
    }
    let f = Fixture::new();
    let (lines, source) = f.checked();
    let mut extra_byte = f.bytes();
    extra_byte.push(b' ');
    assert!(
        compile_owned_item_layouts(
            &lines,
            &source,
            &f.schema,
            &extra_byte,
            &f.policy,
            ItemLayoutLimits::default()
        )
        .is_err()
    );
}

#[test]
fn catalog_bindings_are_a_bijection_of_exact_existing_headers() {
    for change in 0..9 {
        let mut f = Fixture::new();
        match change {
            0 => {
                f.policy.templates.pop();
            }
            1 => f.policy.templates.push(f.policy.templates[0].clone()),
            2 => f.catalog.bases.push(f.catalog.bases[0].clone()),
            3 => f.policy.templates[0].template = f.policy.templates[1].template.clone(),
            4 => f.policy.templates[0].header_rule = f.policy.templates[1].header_rule.clone(),
            5 => f.policy.templates[0].source_base = "Unregistered Base".into(),
            6 => {
                f.policy.templates[0].template = ItemTemplateDefId::parse(
                    GameVersionNamespace::new("foreign", "v1").unwrap(),
                    "alpha",
                )
                .unwrap()
            }
            7 => f.policy.templates[0].header_rule = key("missing-rule"),
            8 => f.policy.templates[0].template = template("missing-template"),
            _ => unreachable!(),
        }
        f.repin();
        f.reject();
    }
    let mut f = Fixture::new();
    f.lines.rules[0].pattern = vec![ItemPatternPart::Literal("Wrong Literal".into())];
    f.rebind_inputs();
    f.reject();
    let mut f = Fixture::new();
    f.source.rule_layouts[0].role = ItemRuleSourceRole::Unresolved;
    f.rebind_inputs();
    f.reject();
    let mut f = Fixture::new();
    f.source.template_layouts.remove(0);
    f.rebind_inputs();
    f.reject();
}

#[test]
fn positive_or_unsupported_evidence_cannot_retract_prior_absence() {
    for prefix in [
        ItemBaseGeneratedPrefix::Present,
        ItemBaseGeneratedPrefix::Unsupported,
    ] {
        let mut f = Fixture::new();
        f.source.template_layouts[0].load_index_prefix =
            ItemLoadIndexPrefix::NoGeneratedBuffMembers;
        f.catalog.bases[0].prefix = prefix;
        f.repin();
        f.rebind_inputs();
        f.reject();
    }
    let mut f = Fixture::new();
    f.source.template_layouts[0].load_index_prefix = ItemLoadIndexPrefix::NoGeneratedBuffMembers;
    f.rebind_inputs();
    let (lines, source) = f.checked();
    let compiled = compile_owned_item_layouts(
        &lines,
        &source,
        &f.schema,
        &f.bytes(),
        &f.policy,
        ItemLayoutLimits::default(),
    )
    .unwrap();
    assert_eq!(
        compiled.item_source.template_layouts,
        f.source.template_layouts
    );
}

#[test]
fn malformed_catalog_fields_and_unknown_wire_values_are_rejected() {
    let f = Fixture::new();
    let (lines, source) = f.checked();
    for change in 0..7 {
        let mut wire = serde_json::to_value(&f.catalog).unwrap();
        match change {
            0 => wire["schema_version"] = 2.into(),
            1 => wire["unexpected"] = true.into(),
            2 => wire["bases"][0]["prefix"] = "maybe".into(),
            3 => wire["bases"][0]["source_base"] = "".into(),
            4 => wire["bases"][0]["source_base"] = " Alpha Tool".into(),
            5 => wire["source"]["files"][0]["sha256"] = "invalid".into(),
            6 => wire["bases"][0]["extra"] = true.into(),
            _ => unreachable!(),
        }
        let bytes = serde_json::to_vec(&wire).unwrap();
        let mut policy = f.policy.clone();
        policy.catalog_sha256 = sha(&bytes);
        assert!(
            compile_owned_item_layouts(
                &lines,
                &source,
                &f.schema,
                &bytes,
                &policy,
                ItemLayoutLimits::default()
            )
            .is_err(),
            "catalog case {change}"
        );
    }
    let mut unknown = serde_json::to_value(&f.policy).unwrap();
    unknown["unchecked"] = true.into();
    assert!(serde_json::from_value::<ItemLayoutPolicy>(unknown).is_err());
    let mut invalid = f.policy.clone();
    invalid.schema_version = 2;
    assert!(
        compile_owned_item_layouts(
            &lines,
            &source,
            &f.schema,
            &f.bytes(),
            &invalid,
            ItemLayoutLimits::default()
        )
        .is_err()
    );
}

#[test]
fn source_projection_pins_are_nonempty_unique_exact_subsets() {
    let mut f = Fixture::new();
    f.source.source.files.push(SourceFilePin {
        path: "test-only/other-source.json".into(),
        sha256: "b".repeat(64),
    });
    f.rebind_inputs();
    let (lines, source) = f.checked();
    assert!(
        compile_owned_item_layouts(
            &lines,
            &source,
            &f.schema,
            &f.bytes(),
            &f.policy,
            ItemLayoutLimits::default()
        )
        .is_ok()
    );
    for change in 0..3 {
        let mut f = Fixture::new();
        match change {
            0 => f.catalog.source.files.clear(),
            1 => f
                .catalog
                .source
                .files
                .push(f.catalog.source.files[0].clone()),
            2 => f.catalog.source.files[0].path = "outside/scope.json".into(),
            _ => unreachable!(),
        }
        f.repin();
        f.reject();
    }
}

#[test]
fn wire_collection_work_and_output_limits_are_enforced_without_mutation() {
    let f = Fixture::new();
    let (lines, source) = f.checked();
    let bytes = f.bytes();
    let baseline = compile_owned_item_layouts(
        &lines,
        &source,
        &f.schema,
        &bytes,
        &f.policy,
        ItemLayoutLimits::default(),
    )
    .unwrap();
    for limits in [
        ItemLayoutLimits {
            max_catalog_bytes: bytes.len() - 1,
            ..Default::default()
        },
        ItemLayoutLimits {
            max_policy_bytes: 1,
            ..Default::default()
        },
        ItemLayoutLimits {
            max_bases: 2,
            ..Default::default()
        },
        ItemLayoutLimits {
            max_work: baseline.receipt.work_used - 1,
            ..Default::default()
        },
        ItemLayoutLimits {
            max_work: 0,
            ..Default::default()
        },
        ItemLayoutLimits {
            max_work: ItemLayoutLimits::default().max_work + 1,
            ..Default::default()
        },
        ItemLayoutLimits {
            source: ItemSourceLimits {
                max_templates: 1,
                ..Default::default()
            },
            ..Default::default()
        },
    ] {
        assert!(
            compile_owned_item_layouts(&lines, &source, &f.schema, &bytes, &f.policy, limits)
                .is_err(),
            "limits={limits:?}"
        );
    }
    let exact = compile_owned_item_layouts(
        &lines,
        &source,
        &f.schema,
        &bytes,
        &f.policy,
        ItemLayoutLimits {
            max_catalog_bytes: bytes.len(),
            max_bases: 3,
            max_work: baseline.receipt.work_used,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(exact.item_source, baseline.item_source);
    assert_eq!(source.input(), &f.source);
    assert_eq!(lines.input(), &f.lines);
    assert_eq!(f.bytes(), bytes);
}

#[test]
fn strict_catalog_and_policy_json_round_trip_and_reject_duplicate_fields() {
    let f = Fixture::new();
    let decoded: ItemBaseLayoutCatalog = serde_json::from_slice(&f.bytes()).unwrap();
    assert_eq!(decoded, f.catalog);
    let policy: ItemLayoutPolicy =
        serde_json::from_slice(&serde_json::to_vec(&f.policy).unwrap()).unwrap();
    assert_eq!(policy, f.policy);
    let (lines, source) = f.checked();
    let duplicated = String::from_utf8(f.bytes()).unwrap().replacen(
        "\"schema_version\":1",
        "\"schema_version\":1,\"schema_version\":1",
        1,
    );
    let mut policy = f.policy.clone();
    policy.catalog_sha256 = sha(duplicated.as_bytes());
    assert!(
        compile_owned_item_layouts(
            &lines,
            &source,
            &f.schema,
            duplicated.as_bytes(),
            &policy,
            ItemLayoutLimits::default()
        )
        .is_err()
    );
    let mut nested = serde_json::to_value(&f.policy).unwrap();
    nested["templates"][0]["ignored"] = true.into();
    assert!(serde_json::from_value::<ItemLayoutPolicy>(nested).is_err());
}

#[test]
fn work_reservations_precede_catalog_decoding_and_policy_hashing() {
    let f = Fixture::new();
    let (lines, source) = f.checked();
    // Matching digest makes this syntactically invalid catalog reach JSON only
    // when sufficient work has been reserved for the complete hash/decode pair.
    let malformed = vec![b'!'; 8192];
    let mut policy = f.policy.clone();
    policy.catalog_sha256 = sha(&malformed);
    assert!(matches!(
        compile_owned_item_layouts(
            &lines,
            &source,
            &f.schema,
            &malformed,
            &policy,
            ItemLayoutLimits {
                max_work: malformed.len(),
                ..Default::default()
            }
        ),
        Err(ItemLayoutError::Limit("work"))
    ));
    assert!(matches!(
        compile_owned_item_layouts(
            &lines,
            &source,
            &f.schema,
            &malformed,
            &policy,
            Default::default()
        ),
        Err(ItemLayoutError::Json(_))
    ));

    // Caller-constructed policy strings are not bounded by their DTO type. They
    // must be accounted before serializing/hashing or discovering a missing row.
    let mut policy = f.policy.clone();
    policy.templates[0].source_base = "z".repeat(100_000);
    assert!(matches!(
        compile_owned_item_layouts(
            &lines,
            &source,
            &f.schema,
            &f.bytes(),
            &policy,
            ItemLayoutLimits {
                max_work: 10_000,
                ..Default::default()
            }
        ),
        Err(ItemLayoutError::Limit("work"))
    ));
    assert!(matches!(
        compile_owned_item_layouts(
            &lines,
            &source,
            &f.schema,
            &f.bytes(),
            &policy,
            Default::default()
        ),
        Err(ItemLayoutError::Binding)
    ));
    assert_eq!(source.input(), &f.source);
}

fn layout_breadth_fixture(count: usize, common_prefix: usize) -> Fixture {
    let mut f = Fixture::new();
    let mut raw = f.schema.input().clone();
    let mut base = raw.definitions[0].clone();
    raw.definitions.clear();
    f.lines
        .rules
        .retain(|rule| matches!(rule.emissions.as_slice(), [ItemEmission::Metadata { .. }]));
    f.source.template_layouts.clear();
    f.catalog.bases.clear();
    f.policy.templates.clear();
    for n in 0..count {
        let id = template(&format!("base-{n:04}"));
        let DefinitionDescriptor::ItemTemplate(entry) = &mut base else {
            unreachable!()
        };
        entry.id = id.clone();
        let SchemaState::Known(schema) = &mut entry.schema else {
            unreachable!()
        };
        schema.declarations = declarations(&id);
        raw.definitions.push(base.clone());
        let name = format!("{} Base {n:04}", "A".repeat(common_prefix));
        let rule = key(&format!("base-{n:04}"));
        f.lines.rules.push(header(
            rule.as_str(),
            &name,
            ItemEmission::Template {
                definition: id.clone(),
            },
        ));
        f.source.template_layouts.push(ItemTemplateSourceLayout {
            template: id.clone(),
            load_index_prefix: ItemLoadIndexPrefix::Unresolved,
        });
        f.catalog.bases.push(ItemBaseLayoutRow {
            source_base: name.clone(),
            prefix: ItemBaseGeneratedPrefix::Absent,
        });
        f.policy.templates.push(ItemLayoutTemplateBinding {
            source_base: name,
            template: id,
            header_rule: rule,
        });
    }
    // Input order is evidence, not a lookup index or a source-name sort promise.
    f.catalog.bases.reverse();
    f.policy.templates.rotate_left(count / 3);
    f.source.template_layouts.reverse();
    f.lines.rules.rotate_left(count / 2);
    f.source.rule_layouts = f
        .lines
        .rules
        .iter()
        .map(|rule| ItemRuleSourceLayout {
            rule: rule.id.clone(),
            role: ItemRuleSourceRole::Header,
        })
        .collect();
    f.schema = OwnedDefinitionSchemaPackage::new(raw, Default::default()).unwrap();
    f.lines.definitions = f.schema.identity().clone();
    f.rebind_inputs();
    f.repin();
    f
}

#[test]
fn full_catalog_breadth_fits_defaults_and_preserves_input_order() {
    let f = layout_breadth_fixture(1756, 8);
    let (lines, source) = f.checked();
    let result = compile_owned_item_layouts(
        &lines,
        &source,
        &f.schema,
        &f.bytes(),
        &f.policy,
        Default::default(),
    )
    .unwrap();
    assert_eq!(result.receipt.bases, 1756);
    assert_eq!(result.receipt.refined_prefixes, 1756);
    assert!(result.receipt.work_used < ItemLayoutLimits::default().max_work);
    for (old, new) in f
        .source
        .template_layouts
        .iter()
        .zip(&result.item_source.template_layouts)
    {
        assert_eq!(new.template, old.template);
        assert_eq!(
            new.load_index_prefix,
            ItemLoadIndexPrefix::NoGeneratedBuffMembers
        );
    }
    assert_eq!(source.input(), &f.source);
    assert_eq!(lines.input(), &f.lines);
}

#[test]
fn long_common_prefix_comparisons_consume_work_and_exact_budget_replays() {
    let f = layout_breadth_fixture(128, 768);
    let (lines, source) = f.checked();
    let bytes = f.bytes();
    let result = compile_owned_item_layouts(
        &lines,
        &source,
        &f.schema,
        &bytes,
        &f.policy,
        Default::default(),
    )
    .unwrap();
    let raw_name_bytes: usize = f
        .catalog
        .bases
        .iter()
        .map(|row| row.source_base.len())
        .sum();
    // Multiple indexed comparisons, not merely one linear pass over each name,
    // must appear in the receipt and in the enforceable caller work budget.
    assert!(result.receipt.work_used > raw_name_bytes * 10);
    for (budget, accepted) in [
        (result.receipt.work_used - 1, false),
        (result.receipt.work_used, true),
    ] {
        let candidate = compile_owned_item_layouts(
            &lines,
            &source,
            &f.schema,
            &bytes,
            &f.policy,
            ItemLayoutLimits {
                max_work: budget,
                ..Default::default()
            },
        );
        if accepted {
            assert_eq!(candidate.unwrap().item_source, result.item_source);
        } else {
            assert!(matches!(candidate, Err(ItemLayoutError::Limit("work"))));
        }
    }
}

#[test]
fn unrelated_prior_source_strings_are_reserved_before_output_escape_scanning() {
    let mut f = Fixture::new();
    let (lines, source) = f.checked();
    let baseline = compile_owned_item_layouts(
        &lines,
        &source,
        &f.schema,
        &f.bytes(),
        &f.policy,
        Default::default(),
    )
    .unwrap();
    // This pin is outside the projected catalog's finite subset. It must remain
    // unchanged, but copying/hashing the complete successor still scans it.
    let path = format!("{}.json", "long-provenance-".repeat(6000));
    f.source.source.files.push(SourceFilePin {
        path: path.clone(),
        sha256: "b".repeat(64),
    });
    f.rebind_inputs();
    let (lines, source) = f.checked();
    let result = compile_owned_item_layouts(
        &lines,
        &source,
        &f.schema,
        &f.bytes(),
        &f.policy,
        Default::default(),
    )
    .unwrap();
    // Charge the raw escape scan in addition to counting and both output clones.
    assert!(result.receipt.work_used - baseline.receipt.work_used >= path.len() * 4);
    let tight = ItemLayoutLimits {
        max_work: result.receipt.work_used - path.len(),
        ..Default::default()
    };
    assert!(matches!(
        compile_owned_item_layouts(&lines, &source, &f.schema, &f.bytes(), &f.policy, tight),
        Err(ItemLayoutError::Limit("work"))
    ));
    assert_eq!(result.item_source.source.files, f.source.source.files);
    assert_eq!(source.input(), &f.source);
}
