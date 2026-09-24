//! Injected item declarations for normalization tests, not a shipped game catalog.
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_schema::*,
};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits, SourceOccurrenceId},
    decode_build,
    owned_item_lines::*,
    owned_item_source::*,
    owned_mapping::*,
    owned_normalize::*,
    owned_reward_policy::*,
    owned_skill_catalog::*,
    owned_source::*,
    owned_value::*,
    owned_value_policy::*,
};
use std::collections::BTreeMap;

pub fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("item-normalization-tests", "v1").unwrap()
}
fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn integer_range(a: i64, b: i64) -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(a).unwrap(),
        maximum: BoundedInteger::new(b).unwrap(),
    }
}
fn quantity_range(unit: &UnitDefId, maximum: f64) -> ValueSchema {
    ValueSchema::Quantity(QuantityRange {
        minimum: FiniteQuantity::new(0.0, unit.clone()).unwrap(),
        maximum: FiniteQuantity::new(maximum, unit.clone()).unwrap(),
    })
}
fn codec(unit: Option<&UnitDefId>) -> ValueCodecInput {
    ValueCodecInput {
        namespace: ns(),
        whitespace: WhitespacePolicy::Exact,
        codec: match unit {
            Some(unit) => ValueCodecKind::Quantity {
                syntax: DecimalSyntax::Decimal,
                unit: unit.clone(),
                scale: RationalScale {
                    numerator: BoundedInteger::new(1).unwrap(),
                    denominator: BoundedInteger::new(1).unwrap(),
                },
            },
            None => ValueCodecKind::Integer {
                syntax: DecimalSyntax::Integer,
            },
        },
    }
}
fn literal(s: &str) -> ItemPatternPart {
    ItemPatternPart::Literal(s.into())
}
fn capture(s: &str) -> ItemPatternPart {
    ItemPatternPart::Capture(key(s))
}
fn rule(
    name: &str,
    pattern: Vec<ItemPatternPart>,
    captures: Vec<ItemCapture>,
    emissions: Vec<ItemEmission>,
) -> ItemLineRule {
    ItemLineRule {
        id: key(name),
        pattern,
        captures,
        emissions,
    }
}
fn value_capture(name: &str, unit: Option<&UnitDefId>) -> ItemCapture {
    ItemCapture {
        id: key(name),
        codec: ItemCaptureCodec::Value(codec(unit)),
    }
}
fn captured(name: &str) -> ItemLineValue {
    ItemLineValue::Capture(key(name))
}
fn metadata_exact(name: &str, text: &str) -> ItemLineRule {
    rule(
        name,
        vec![literal(text)],
        vec![],
        vec![ItemEmission::Metadata {
            role: key("presentation-or-metadata"),
        }],
    )
}
#[derive(Clone)]
pub struct ModifierSpec {
    pub definition: ModifierDefId,
    pub slots: Vec<DeclaredSlot<ParameterSlotDefId>>,
}
pub struct Artifacts {
    pub registry: OwnedIdRegistry,
    pub schema: OwnedDefinitionSchemaPackage,
    pub mapping: OwnedMappingIndex,
    pub roles: OwnedSkillRoleIndex,
    pub rewards: OwnedRewardPolicy,
    pub items: OwnedItemLinePolicy,
    pub item_source: ItemSourceLayoutPolicy,
    pub spear: ItemTemplateDefId,
    pub staff: ItemTemplateDefId,
    pub quality: QualityDefId,
    pub percent: UnitDefId,
    pub modifiers: BTreeMap<&'static str, ModifierSpec>,
}
pub fn artifacts() -> Artifacts {
    let limits = OwnedMappingLimits::default();
    let mut registry = OwnedIdRegistry::empty(ns(), limits).unwrap();
    let percent = registry.allocate_definition::<UnitDefinition>().unwrap();
    let damage = registry.allocate_definition::<UnitDefinition>().unwrap();
    let quality = registry.allocate_definition::<QualityDefinition>().unwrap();
    let spear = registry
        .allocate_definition::<ItemTemplateDefinition>()
        .unwrap();
    let staff = registry
        .allocate_definition::<ItemTemplateDefinition>()
        .unwrap();
    let granted_level = registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::ItemTemplate(staff.clone()))
        .unwrap();
    let mut definitions = vec![
        DefinitionDescriptor::Unit(entry(
            percent.clone(),
            UnitSchema {
                dimension: UnitDimension::PercentagePoints,
            },
        )),
        DefinitionDescriptor::Unit(entry(
            damage.clone(),
            UnitSchema {
                dimension: UnitDimension::Damage,
            },
        )),
        DefinitionDescriptor::Quality(entry(
            quality.clone(),
            QualitySchema {
                amount: QuantityRange {
                    minimum: FiniteQuantity::new(0.0, percent.clone()).unwrap(),
                    maximum: FiniteQuantity::new(100.0, percent.clone()).unwrap(),
                },
            },
        )),
    ];
    let mut slots = vec![SlotDescriptor::Parameter(entry(
        granted_level.clone(),
        ParameterSlotSchema {
            value: ValueSchema::Integer(integer_range(1, 100)),
            presence: SlotPresence::OptionalOnce,
            sites: vec![ParameterSite::ItemParameter],
        },
    ))];
    let mut modifiers = BTreeMap::new();
    for (name, arity, unit) in [
        ("physical", 1, &percent),
        ("broken-armour", 1, &percent),
        ("strike-range", 1, &percent),
        ("cold", 2, &damage),
        ("lightning", 2, &damage),
        ("critical-bonus", 1, &percent),
        ("attack-speed", 1, &percent),
        ("leech", 1, &percent),
        ("spell", 1, &percent),
    ] {
        let definition = registry
            .allocate_definition::<ModifierDefinition>()
            .unwrap();
        let mut ports = declarations();
        for _ in 0..arity {
            let slot = registry
                .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(
                    definition.clone(),
                ))
                .unwrap();
            // A supported computation domain, deliberately not affix tier/crafting legality.
            slots.push(SlotDescriptor::Parameter(entry(
                slot.clone(),
                ParameterSlotSchema {
                    value: quantity_range(unit, 10000.0),
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::ModifierRoll],
                },
            )));
            ports.parameters.members.push(slot);
        }
        modifiers.insert(
            name,
            ModifierSpec {
                definition: definition.clone(),
                slots: ports.parameters.members.clone(),
            },
        );
        definitions.push(DefinitionDescriptor::Modifier(entry(
            definition,
            ModifierSchema {
                declarations: ports,
            },
        )));
    }
    for template in [&spear, &staff] {
        let mut ports = declarations();
        if template == &staff {
            ports.parameters.members.push(granted_level.clone());
        }
        definitions.push(DefinitionDescriptor::ItemTemplate(entry(
            template.clone(),
            ItemTemplateSchema {
                item_level: integer_range(1, 100),
                equipment_slots: DeclaredSet::complete(vec![]),
                socket_destinations: DeclaredSet::complete(vec![]),
                modifiers: DeclaredSet::complete(
                    modifiers.values().map(|s| s.definition.clone()).collect(),
                ),
                quality: QualityUseSchema {
                    presence: QualityPresence::Optional,
                    allowed_kinds: DeclaredSet::complete(vec![quality.clone()]),
                },
                declarations: ports,
            },
        )));
    }
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: ns(),
            release: key("test-fixture"),
            semantics_version: key("item-inputs-v1"),
            definitions,
            slots,
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let mut rules = vec![
        rule(
            "spear-template",
            vec![literal("Grand Spear")],
            vec![],
            vec![ItemEmission::Template {
                definition: spear.clone(),
            }],
        ),
        rule(
            "staff-template",
            vec![literal("Ashen Staff")],
            vec![],
            vec![ItemEmission::Template {
                definition: staff.clone(),
            }],
        ),
        rule(
            "item-level",
            vec![literal("Item Level: "), capture("value")],
            vec![value_capture("value", None)],
            vec![ItemEmission::ItemLevel {
                value: captured("value"),
            }],
        ),
        rule(
            "quality",
            vec![literal("Quality: "), capture("value")],
            vec![value_capture("value", Some(&percent))],
            vec![ItemEmission::Quality {
                kind: quality.clone(),
                amount: captured("value"),
            }],
        ),
        metadata_exact("blank", ""),
        metadata_exact("spear-name", "Dusk Edge (Attack Speed Adjusted)"),
        metadata_exact("staff-name", "New Item"),
    ];
    for (name, prefix) in [
        ("rarity", "Rarity: "),
        ("crafted", "Crafted: "),
        ("prefix", "Prefix: "),
        ("suffix", "Suffix: "),
        ("sockets", "Sockets: "),
        ("rune", "Rune: "),
        ("requirement", "LevelReq: "),
        ("implicit-count", "Implicits: "),
    ] {
        rules.push(rule(
            name,
            vec![literal(prefix), capture("metadata")],
            vec![ItemCapture {
                id: key("metadata"),
                codec: ItemCaptureCodec::OpaqueText,
            }],
            vec![ItemEmission::Metadata {
                role: key("source-metadata"),
            }],
        ));
    }
    for (name, modifier, prefix, suffix) in [
        ("physical", "physical", "", "% increased Physical Damage"),
        (
            "bonded",
            "broken-armour",
            "{enchant}{rune}Bonded: ",
            "% increased effect of Fully Broken Armour",
        ),
        (
            "range",
            "strike-range",
            "{tags:attack}",
            "% increased Melee Strike Range with this weapon",
        ),
        (
            "critical",
            "critical-bonus",
            "+",
            "% to Critical Damage Bonus",
        ),
        ("speed", "attack-speed", "", "% increased Attack Speed"),
        ("leech", "leech", "Leeches ", "% of Physical Damage as Life"),
        ("spell", "spell", "", "% increased Spell Damage"),
    ] {
        let mut pattern = vec![];
        if !prefix.is_empty() {
            pattern.push(literal(prefix));
        }
        pattern.push(capture("value"));
        pattern.push(literal(suffix));
        let spec = &modifiers[modifier];
        rules.push(rule(
            name,
            pattern,
            vec![value_capture("value", Some(&percent))],
            vec![ItemEmission::Modifier {
                definition: spec.definition.clone(),
                rolls: vec![ItemRollTemplate {
                    slot: spec.slots[0].clone(),
                    value: captured("value"),
                }],
            }],
        ));
    }
    for (name, suffix) in [("cold", " Cold Damage"), ("lightning", " Lightning Damage")] {
        let spec = &modifiers[name];
        rules.push(rule(
            name,
            vec![
                literal("Adds "),
                capture("lower"),
                literal(" to "),
                capture("upper"),
                literal(suffix),
            ],
            vec![
                value_capture("lower", Some(&damage)),
                value_capture("upper", Some(&damage)),
            ],
            vec![ItemEmission::Modifier {
                definition: spec.definition.clone(),
                rolls: vec![
                    ItemRollTemplate {
                        slot: spec.slots[0].clone(),
                        value: captured("lower"),
                    },
                    ItemRollTemplate {
                        slot: spec.slots[1].clone(),
                        value: captured("upper"),
                    },
                ],
            }],
        ));
    }
    // Semantic recipe receives a fraction only from the separate source adapter.
    rules.push(rule(
        "ranged-staff-grant",
        vec![
            literal("Grants Skill: Level ("),
            capture("lower"),
            literal("-"),
            capture("upper"),
            literal(") Firebolt"),
        ],
        vec![value_capture("lower", None), value_capture("upper", None)],
        vec![ItemEmission::ItemParameter {
            slot: granted_level,
            value: ItemLineValue::Interpolate {
                lower: key("lower"),
                upper: key("upper"),
                quantum: ParameterValue::Integer(BoundedInteger::new(1).unwrap()),
                rounding: ItemRangeRounding::NearestTiesPositive,
            },
        }],
    ));
    let items = OwnedItemLinePolicy::new(
        ItemLinePolicyInput {
            schema_version: OWNED_ITEM_LINE_POLICY_VERSION,
            namespace: ns(),
            version: key("explicit-item-lines"),
            definitions: schema.identity().clone(),
            whitespace: WhitespacePolicy::TrimAscii,
            rules,
        },
        &schema,
        ItemLineLimits::default(),
    )
    .unwrap();
    let source_pin = SourcePin {
        system: ExternalSourceSystem::PathOfBuilding2,
        revision: "test-only-injected-declarations".into(),
        files: vec![SourceFilePin {
            path: "test-only/item-policy.json".into(),
            sha256: "a".repeat(64),
        }],
    };
    let item_source = source_policy(&items, &schema, [&spear, &staff], source_pin.clone());
    let mapping = OwnedMappingIndex::new(
        MappingPackageInput {
            schema_version: OWNED_MAPPING_PACKAGE_VERSION,
            namespace: ns(),
            registry: registry.identity().unwrap(),
            definitions: schema.identity().clone(),
            source: source_pin.clone(),
            policy_version: key("test-mapping"),
            entries: vec![],
        },
        &registry,
        &schema,
        limits,
    )
    .unwrap();
    let roles = OwnedSkillRoleIndex::new(
        OwnedSkillRolePackageInput {
            schema_version: OWNED_SKILL_ROLE_VERSION,
            namespace: ns(),
            definitions: schema.identity().clone(),
            mapping: *mapping.identity(),
            roles: vec![],
            compilation: SkillCatalogReceipt {
                source: source_pin,
                catalog_digest: "b".repeat(64).parse().unwrap(),
                policy: SkillCatalogPolicy {
                    version: key("test-mapping"),
                    absent_support: AbsentSupportPolicy::Pending,
                    absent_from_tree: AbsentFromTreePolicy::Pending,
                },
                base_registry: registry.identity().unwrap(),
                staged_registry: registry.identity().unwrap(),
                gem_count: 0,
                skill_count: 0,
            },
        },
        &mapping,
        &schema,
        SkillCatalogLimits::default(),
    )
    .unwrap();
    let rewards = OwnedRewardPolicy::new(
        RewardPolicyInput {
            schema_version: OWNED_REWARD_POLICY_VERSION,
            namespace: ns(),
            version: key("test-rewards"),
            definitions: schema.identity().clone(),
            mapping: *mapping.identity(),
            rules: vec![],
        },
        &mapping,
        &schema,
        RewardPolicyLimits::default(),
    )
    .unwrap();
    Artifacts {
        registry,
        schema,
        mapping,
        roles,
        rewards,
        items,
        item_source,
        spear,
        staff,
        quality,
        percent,
        modifiers,
    }
}
fn recipe(name: &str, boolean: bool) -> ValueRecipeInput {
    ValueRecipeInput {
        id: key(if boolean { "boolean" } else { "level" }),
        codec: ValueCodecInput {
            namespace: ns(),
            whitespace: WhitespacePolicy::Exact,
            codec: if boolean {
                ValueCodecKind::Boolean {
                    tokens: vec![
                        BooleanToken {
                            token: "true".into(),
                            value: true,
                        },
                        BooleanToken {
                            token: "false".into(),
                            value: false,
                        },
                    ],
                }
            } else {
                ValueCodecKind::Integer {
                    syntax: DecimalSyntax::Integer,
                }
            },
        },
        tiers: vec![ValueTier {
            selectors: vec![ValueSelector {
                lane: ValueLane::Attribute,
                name: name.into(),
            }],
            duplicates: DuplicatePolicy::Reject,
        }],
        missing: MissingValuePolicy::Pending,
    }
}
pub fn source(xml: &str) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([29; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
pub fn normalize(source: &ImportedBuildInstance, artifacts: &Artifacts) -> NormalizedImport {
    let evidence = SourceProjectEvidence::collect(source, SourceEvidenceLimits::default()).unwrap();
    normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            tree: None,
            mappings: &artifacts.mapping,
            registry: &artifacts.registry,
            definitions: &artifacts.schema,
            roles: &artifacts.roles,
            rewards: &artifacts.rewards,
            items: &artifacts.items,
            item_source: &artifacts.item_source,
        },
        &NormalizationPolicy {
            version: key("item-test-normalization"),
            namespace: ns(),
            character_level: recipe("level", false),
            gem_level: recipe("level", false),
            gem_enabled: recipe("enabled", true),
            group_enabled: recipe("enabled", true),
            manual_skill_sources: vec![
                SourceComponent::Missing,
                SourceComponent::Text(String::new()),
            ],
            empty_item_keys: vec![SourceComponent::Text("0".into())],
            generated_support_prefixes: vec![],
            allocation_attribute: "nodes".into(),
            single_active_support_target: false,
            equipment_loadouts: vec![],
            skill_scopes: None,
            gem_quality: GemQualityPolicy::Unconverted,
        },
        &[],
        NormalizationLimits::default(),
    )
    .unwrap()
}
pub fn item_source(source: &ImportedBuildInstance, id: &str) -> SourceOccurrenceId {
    let evidence = SourceProjectEvidence::collect(source, SourceEvidenceLimits::default()).unwrap();
    evidence
        .rows()
        .iter()
        .find(|row| {
            row.occurrence().name() == "Item"
                && row.attribute("id").and_then(|a| a.decoded().ok()) == Some(id)
        })
        .unwrap()
        .occurrence()
        .id()
}

/// Reviewed source-layout declarations belong to this test fixture, not native data.
pub fn source_policy<'a>(
    items: &OwnedItemLinePolicy,
    schema: &OwnedDefinitionSchemaPackage,
    templates: impl IntoIterator<Item = &'a ItemTemplateDefId>,
    source: SourcePin,
) -> ItemSourceLayoutPolicy {
    ItemSourceLayoutPolicy::new(
        ItemSourceLayoutPolicyInput {
            schema_version: OWNED_ITEM_SOURCE_POLICY_VERSION,
            namespace: schema.namespace().clone(),
            version: key("reviewed-test-layout"),
            source,
            item_lines: *items.identity(),
            dialect: ItemSourceDialect::PobExportedSingleTextV1,
            property_bindings: vec![],
            template_defaults: vec![],
            rule_layouts: items
                .input()
                .rules
                .iter()
                .map(|r| ItemRuleSourceLayout {
                    rule: r.id.clone(),
                    role: if r.emissions.iter().all(|e| {
                        matches!(
                            e,
                            ItemEmission::Metadata { .. }
                                | ItemEmission::Template { .. }
                                | ItemEmission::ItemLevel { .. }
                                | ItemEmission::Quality { .. }
                        )
                    }) {
                        ItemRuleSourceRole::Header
                    } else {
                        ItemRuleSourceRole::SingleModifier
                    },
                })
                .collect(),
            template_layouts: templates
                .into_iter()
                .map(|t| ItemTemplateSourceLayout {
                    template: t.clone(),
                    load_index_prefix: ItemLoadIndexPrefix::NoGeneratedBuffMembers,
                })
                .collect(),
        },
        items,
        schema,
        ItemSourceLimits::default(),
    )
    .unwrap()
}
