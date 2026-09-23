//! Dense finite default catalogs use bounded indexed validation, not quadratic scans.
use poe_optimizer_core::{
    build_identity::BuildLineage,
    owned_build::{DeclaredSlot, ParameterAssignment, ParameterValue},
    owned_definitions::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance, decode_build, owned_item_lines::*, owned_item_source::*,
    owned_mapping::*, owned_source::*, owned_value::WhitespacePolicy,
};
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn id<D: DefinitionDomain>(n: usize) -> DefId<D> {
    DefId::parse(ns(), format!("def.{n:016x}")).unwrap()
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn slot(owner: usize, field: usize) -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(id(owner + 1)),
        slot: id(2000 + owner * 2 + field),
    }
}
fn quantity(n: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, id(10000)).unwrap())
}
struct Fixture {
    schema: OwnedDefinitionSchemaPackage,
    lines: OwnedItemLinePolicy,
    source: ItemSourceLayoutPolicyInput,
}
fn fixture() -> Fixture {
    let mut raw = SchemaPackageInput {
        schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
        namespace: ns(),
        release: key("test"),
        semantics_version: key("test"),
        definitions: vec![DefinitionDescriptor::Unit(known(
            id(10000),
            UnitSchema {
                dimension: UnitDimension::PercentagePoints,
            },
        ))],
        slots: vec![],
    };
    let options: Vec<OptionDefId> = (10001..10015).map(id).collect();
    for option in &options {
        raw.definitions.push(DefinitionDescriptor::Option(known(
            option.clone(),
            OptionSchema {},
        )));
    }
    let mut rules = vec![];
    let mut defaults = vec![];
    let mut layouts = vec![];
    // Same catalog breadth as the real source package: 338 configured templates
    // with two required inputs, alongside 1,418 templates without defaults.
    for index in 0..1756 {
        let template: ItemTemplateDefId = id(index + 1);
        let parameters = if index < 338 {
            vec![slot(index, 0), slot(index, 1)]
        } else {
            vec![]
        };
        raw.definitions
            .push(DefinitionDescriptor::ItemTemplate(known(
                template.clone(),
                ItemTemplateSchema {
                    item_level: IntegerRange {
                        minimum: BoundedInteger::new(1).unwrap(),
                        maximum: BoundedInteger::new(100).unwrap(),
                    },
                    equipment_slots: DeclaredSet::complete(vec![]),
                    socket_destinations: DeclaredSet::complete(vec![]),
                    modifiers: DeclaredSet::complete(vec![]),
                    quality: QualityUseSchema {
                        presence: QualityPresence::Forbidden,
                        allowed_kinds: DeclaredSet::complete(vec![]),
                    },
                    declarations: DeclaredSlots {
                        parameters: DeclaredSet::complete(parameters),
                        choices: DeclaredSet::complete(vec![]),
                        grants: DeclaredSet::complete(vec![]),
                        actors: DeclaredSet::complete(vec![]),
                        skill_grants: DeclaredSet::complete(vec![]),
                        outputs: DeclaredSet::complete(vec![]),
                        sockets: DeclaredSet::complete(vec![]),
                    },
                },
            )));
        if index < 338 {
            for (field, value) in [
                (
                    0,
                    ValueSchema::Option {
                        allowed: DeclaredSet::complete(options.clone()),
                    },
                ),
                (
                    1,
                    ValueSchema::Quantity(QuantityRange {
                        minimum: FiniteQuantity::new(0.0, id(10000)).unwrap(),
                        maximum: FiniteQuantity::new(100.0, id(10000)).unwrap(),
                    }),
                ),
            ] {
                raw.slots.push(SlotDescriptor::Parameter(known(
                    slot(index, field),
                    ParameterSlotSchema {
                        value,
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![ParameterSite::ItemParameter],
                    },
                )));
            }
            defaults.push(ItemSourceTemplateDefaults {
                template: template.clone(),
                item_level: ItemSourceAbsentPolicy::Absent,
                quality: ItemSourceAbsentPolicy::Absent,
                parameters: vec![
                    ItemSourceParameterDefault {
                        assignment: ParameterAssignment {
                            slot: slot(index, 0),
                            value: ParameterValue::Option(options[0].clone()),
                        },
                        headers: vec!["Catalyst".into()],
                    },
                    ItemSourceParameterDefault {
                        assignment: ParameterAssignment {
                            slot: slot(index, 1),
                            value: quantity(20.0),
                        },
                        headers: vec!["CatalystQuality".into()],
                    },
                ],
            });
        }
        rules.push(ItemLineRule {
            id: key(&format!("base-{index:04}")),
            pattern: vec![ItemPatternPart::Literal(format!("Base {index:04}"))],
            captures: vec![],
            emissions: vec![ItemEmission::Template {
                definition: template.clone(),
            }],
        });
        layouts.push(ItemTemplateSourceLayout {
            template,
            load_index_prefix: ItemLoadIndexPrefix::NoGeneratedBuffMembers,
        });
    }
    for (name, text) in [
        ("rarity", "Rarity: RARE"),
        ("title", "Fixture"),
        ("implicit-count", "Implicits: 0"),
    ] {
        rules.push(ItemLineRule {
            id: key(name),
            pattern: vec![ItemPatternPart::Literal(text.into())],
            captures: vec![],
            emissions: vec![ItemEmission::Metadata {
                role: key("preamble"),
            }],
        });
    }
    // Neither catalog source order nor requested-default order is an ID index.
    defaults.reverse();
    layouts.reverse();
    rules.rotate_left(723);
    let schema = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    let lines = OwnedItemLinePolicy::new(
        ItemLinePolicyInput {
            schema_version: OWNED_ITEM_LINE_POLICY_VERSION,
            namespace: ns(),
            version: key("lines"),
            definitions: schema.identity().clone(),
            whitespace: WhitespacePolicy::Exact,
            rules,
        },
        &schema,
        ItemLineLimits::default(),
    )
    .unwrap();
    let source = ItemSourceLayoutPolicyInput {
        schema_version: OWNED_ITEM_SOURCE_POLICY_VERSION,
        namespace: ns(),
        version: key("defaults"),
        source: SourcePin {
            system: ExternalSourceSystem::PathOfBuilding2,
            revision: "test".into(),
            files: vec![SourceFilePin {
                path: "test.json".into(),
                sha256: "a".repeat(64),
            }],
        },
        item_lines: *lines.identity(),
        dialect: ItemSourceDialect::PobExportedSingleTextV1,
        property_bindings: vec![],
        rule_layouts: lines
            .input()
            .rules
            .iter()
            .map(|r| ItemRuleSourceLayout {
                rule: r.id.clone(),
                role: ItemRuleSourceRole::Header,
            })
            .collect(),
        template_layouts: layouts,
        template_defaults: defaults,
    };
    Fixture {
        schema,
        lines,
        source,
    }
}
fn converted_defaults(
    f: &Fixture,
    policy: &ItemSourceLayoutPolicy,
    template: usize,
    headers: &str,
) -> ItemDefaultedInputs {
    let xml = format!(
        "<PathOfBuilding2><Items><Item id=\"7\">Rarity: RARE\nFixture\nBase {template:04}\n{headers}Implicits: 0</Item></Items></PathOfBuilding2>"
    );
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([32; 16]),
        Default::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let item = evidence
        .rows()
        .iter()
        .find(|row| row.occurrence().name() == "Item")
        .unwrap()
        .occurrence()
        .id();
    let plan = policy.attribute(&evidence, item, &f.lines).unwrap();
    plan.convert(&f.lines).unwrap().defaults
}
#[test]
fn dense_defaults_and_full_catalog_validate_under_standard_limits() {
    let f = fixture();
    let policy = ItemSourceLayoutPolicy::new(
        f.source.clone(),
        &f.lines,
        &f.schema,
        ItemSourceLimits::default(),
    )
    .unwrap();
    for template in [0, 167, 337] {
        let defaults = converted_defaults(&f, &policy, template, "");
        assert_eq!(defaults.parameters.len(), 2);
        assert_eq!(defaults.parameters[0].slot, slot(template, 0));
        assert_eq!(
            defaults.parameters[1],
            ParameterAssignment {
                slot: slot(template, 1),
                value: quantity(20.0)
            }
        );
        assert!(defaults.item_level_absent && defaults.quality_absent);
    }
    assert_eq!(
        converted_defaults(&f, &policy, 1755, ""),
        ItemDefaultedInputs::default()
    );
    assert_eq!(
        converted_defaults(&f, &policy, 0, "CatalystQuality: malformed\n"),
        ItemDefaultedInputs::default()
    );
    let bytes = encode_item_source_policy(&policy, ItemSourceLimits::default()).unwrap();
    let decoded =
        decode_item_source_policy(&bytes, &f.lines, &f.schema, ItemSourceLimits::default())
            .unwrap();
    assert_eq!(decoded.identity(), policy.identity());
    assert_eq!(decoded.input(), policy.input());
    let tight = ItemSourceLimits {
        max_schema_work: 1000,
        ..Default::default()
    };
    assert!(matches!(
        ItemSourceLayoutPolicy::new(f.source.clone(), &f.lines, &f.schema, tight),
        Err(ItemSourceError::Limit("schema work"))
    ));
    assert!(matches!(
        encode_item_source_policy(&policy, tight),
        Err(ItemSourceError::Limit("schema work"))
    ));
    assert!(matches!(
        decode_item_source_policy(&bytes, &f.lines, &f.schema, tight),
        Err(ItemSourceError::Limit("schema work"))
    ));
}
#[test]
fn breadth_never_substitutes_an_unrelated_layout_or_line_binding() {
    let f = fixture();
    let mut source = f.source.clone();
    source
        .template_layouts
        .retain(|layout| layout.template != id::<ItemTemplateDefinition>(168));
    assert!(matches!(
        ItemSourceLayoutPolicy::new(source, &f.lines, &f.schema, ItemSourceLimits::default()),
        Err(ItemSourceError::Policy(
            "default template has no source layout or line binding"
        ))
    ));
    let mut lines = f.lines.input().clone();
    lines.rules.retain(|rule| !rule.emissions.iter().any(|emission| matches!(emission, ItemEmission::Template { definition } if definition == &id::<ItemTemplateDefinition>(168))));
    let lines = OwnedItemLinePolicy::new(lines, &f.schema, ItemLineLimits::default()).unwrap();
    let mut source = f.source.clone();
    source.item_lines = *lines.identity();
    source
        .rule_layouts
        .retain(|role| lines.input().rules.iter().any(|rule| role.rule == rule.id));
    assert!(matches!(
        ItemSourceLayoutPolicy::new(source, &lines, &f.schema, ItemSourceLimits::default()),
        Err(ItemSourceError::Policy(
            "default template has no source layout or line binding"
        ))
    ));
    let mut source = f.source;
    source
        .template_defaults
        .push(source.template_defaults[0].clone());
    assert!(matches!(
        ItemSourceLayoutPolicy::new(source, &f.lines, &f.schema, ItemSourceLimits::default()),
        Err(ItemSourceError::Policy("duplicate default template"))
    ));
}
