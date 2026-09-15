//! Directly authored component evidence for four owned effect families.
//!
//! Coefficients are synthetic test data. Facts stand for independently resolved
//! inputs; this helper does not establish incoming-effect closure, provider
//! existence, game legality, source conversion, or full-build numerical coverage.
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{
    OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, OwnedSchemaLimits,
    SchemaPackageInput,
};

pub struct Fixture {
    pub schema: OwnedDefinitionSchemaPackage,
    pub rules: RulePackageInput,
    pub cases: Vec<Case>,
}
pub struct Case {
    pub name: &'static str,
    pub owner: SchemaSubject,
    pub program: OwnedDefinitionKey,
    pub facts: Vec<(OwnedDefinitionKey, ParameterValue)>,
    pub expected: Vec<ExpectedEffect>,
}
pub struct ExpectedEffect {
    pub id: OwnedDefinitionKey,
    /// None means a false effect guard, not a missing numeric value or zero.
    pub value: Option<ParameterValue>,
}

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("authored-rule-tests", "v1").unwrap()
}
fn id<K: DefinitionDomain>(value: &str) -> DefId<K> {
    DefId::parse(namespace(), value).unwrap()
}
fn integer(value: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(value).unwrap())
}
fn quantity(value: f64, unit: &UnitDefId) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap())
}
fn quantity_type(unit: &UnitDefId) -> ComputedValueType {
    ComputedValueType::Quantity { unit: unit.clone() }
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
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
fn range(minimum: i64, maximum: i64) -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(minimum).unwrap(),
        maximum: BoundedInteger::new(maximum).unwrap(),
    }
}
fn read(name: &str, value_type: ComputedValueType, source: RuleReadSource) -> RuleRead {
    RuleRead {
        id: key(name),
        value_type,
        source,
    }
}
fn node(name: &str, expression: RuleExpression) -> RuleNode {
    RuleNode {
        id: key(name),
        expression,
    }
}
fn read_node(name: &str, input: &str) -> RuleNode {
    node(name, RuleExpression::Read { input: key(input) })
}
fn literal(name: &str, value: ParameterValue) -> RuleNode {
    node(name, RuleExpression::Literal { value })
}
fn effect(name: &str, when: Option<&str>, effect: RuleEffectKind) -> RuleEffect {
    RuleEffect {
        id: key(name),
        when: when.map(key),
        effect,
    }
}
fn expected(name: &str, value: Option<ParameterValue>) -> ExpectedEffect {
    ExpectedEffect {
        id: key(name),
        value,
    }
}
fn fact(name: &str, value: ParameterValue) -> (OwnedDefinitionKey, ParameterValue) {
    (key(name), value)
}
fn subject<I: SchemaDefinitionId>(id: &I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
}

pub fn fixture() -> Fixture {
    let item: ItemTemplateDefId = id("item.template");
    let modifier: ModifierDefId = id("modifier.conditioned");
    let support: GemDefId = id("gem.support");
    let quality: QualityDefId = id("quality.local");
    let damage_unit: UnitDefId = id("unit.damage");
    let percent_unit: UnitDefId = id("unit.percent");
    let factor_unit: UnitDefId = id("unit.factor");
    let local_stat: StatDefId = id("stat.local-damage");
    let attribute_stat: StatDefId = id("stat.effective-attribute");
    let damage_stat: StatDefId = id("stat.damage");
    let supported_capability: CapabilityDefId = id("capability.supported-action");
    let grant = DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(item.clone()),
        slot: id::<GrantSlotDefinition>("grant.actor"),
    };
    let actor = DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(item.clone()),
        slot: id::<ActorSlotDefinition>("actor.supplied"),
    };
    let mut item_declarations = declarations();
    item_declarations.grants.members.push(grant.clone());
    item_declarations.actors.members.push(actor.clone());
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: namespace(),
            release: key("test-release"),
            semantics_version: key("input-and-computed-schema-v1"),
            definitions: vec![
                DefinitionDescriptor::Unit(known(
                    damage_unit.clone(),
                    UnitSchema {
                        dimension: UnitDimension::Damage,
                    },
                )),
                DefinitionDescriptor::Unit(known(
                    percent_unit.clone(),
                    UnitSchema {
                        dimension: UnitDimension::PercentagePoints,
                    },
                )),
                DefinitionDescriptor::Unit(known(
                    factor_unit.clone(),
                    UnitSchema {
                        dimension: UnitDimension::DimensionlessFactor,
                    },
                )),
                DefinitionDescriptor::Quality(known(
                    quality.clone(),
                    QualitySchema {
                        amount: QuantityRange {
                            minimum: FiniteQuantity::new(0.0, percent_unit.clone()).unwrap(),
                            maximum: FiniteQuantity::new(100.0, percent_unit.clone()).unwrap(),
                        },
                    },
                )),
                DefinitionDescriptor::ItemTemplate(known(
                    item.clone(),
                    ItemTemplateSchema {
                        item_level: range(1, 100),
                        equipment_slots: DeclaredSet::complete(vec![]),
                        socket_destinations: DeclaredSet::complete(vec![]),
                        modifiers: DeclaredSet::complete(vec![]),
                        quality: QualityUseSchema {
                            presence: QualityPresence::Optional,
                            allowed_kinds: DeclaredSet::complete(vec![quality.clone()]),
                        },
                        declarations: item_declarations,
                    },
                )),
                DefinitionDescriptor::Modifier(known(
                    modifier.clone(),
                    ModifierSchema {
                        declarations: declarations(),
                    },
                )),
                DefinitionDescriptor::Gem(known(
                    support.clone(),
                    GemSchema {
                        level: range(1, 20),
                        roles: vec![AuthoredGemRole::SupportAssignment],
                        skills: DeclaredSet::complete(vec![]),
                        quality: QualityUseSchema {
                            presence: QualityPresence::Forbidden,
                            allowed_kinds: DeclaredSet::complete(vec![]),
                        },
                        declarations: declarations(),
                    },
                )),
                DefinitionDescriptor::Stat(known(
                    local_stat.clone(),
                    StatSchema {
                        value: quantity_type(&damage_unit),
                        targets: vec![RuleEntityKind::EquipmentUse],
                    },
                )),
                DefinitionDescriptor::Stat(known(
                    attribute_stat.clone(),
                    StatSchema {
                        value: ComputedValueType::Integer,
                        targets: vec![RuleEntityKind::Actor],
                    },
                )),
                DefinitionDescriptor::Stat(known(
                    damage_stat.clone(),
                    StatSchema {
                        value: quantity_type(&damage_unit),
                        targets: vec![RuleEntityKind::Actor, RuleEntityKind::Action],
                    },
                )),
                DefinitionDescriptor::Capability(known(
                    supported_capability.clone(),
                    CapabilitySchema {
                        targets: vec![RuleEntityKind::Action],
                    },
                )),
            ],
            slots: vec![
                SlotDescriptor::Grant(known(
                    grant.clone(),
                    GrantSlotSchema {
                        provider_roles: vec![ProviderRole::EquipmentUse],
                        target: GrantTarget::Actor(actor.clone()),
                    },
                )),
                SlotDescriptor::Actor(known(
                    actor,
                    ActorSlotSchema {
                        skills: DeclaredSet::complete(vec![]),
                        outputs: DeclaredSet::complete(vec![]),
                    },
                )),
            ],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();

    // Local arithmetic has explicit stages; quality belongs to candidate facts.
    // The caller-provided reduction facts do not prove incoming membership here.
    let local = RuleProgram {
        id: key("local-item"),
        context: RuleEntityKind::EquipmentUse,
        reads: vec![
            read(
                "flat",
                quantity_type(&damage_unit),
                RuleReadSource::Contributions {
                    entity: RuleEntity::Current,
                    stat: local_stat.clone(),
                    contribution: ContributionKind::Add,
                    reduction: ContributionReduction::Sum,
                    empty: quantity(0.0, &damage_unit),
                },
            ),
            read(
                "increase",
                quantity_type(&percent_unit),
                RuleReadSource::Contributions {
                    entity: RuleEntity::Current,
                    stat: local_stat.clone(),
                    contribution: ContributionKind::Increase,
                    reduction: ContributionReduction::Sum,
                    empty: quantity(0.0, &percent_unit),
                },
            ),
            read(
                "has-quality",
                ComputedValueType::Boolean,
                RuleReadSource::HasItemQuality {
                    quality: quality.clone(),
                },
            ),
            read(
                "quality",
                quantity_type(&percent_unit),
                RuleReadSource::ItemQualityAmount { quality },
            ),
        ],
        nodes: vec![
            literal("base", quantity(100.0, &damage_unit)),
            literal("one", quantity(1.0, &factor_unit)),
            read_node("flat-value", "flat"),
            read_node("increase-value", "increase"),
            read_node("has-quality-value", "has-quality"),
            read_node("quality-value", "quality"),
            node(
                "base-plus-flat",
                RuleExpression::Add {
                    left: key("base"),
                    right: key("flat-value"),
                },
            ),
            node(
                "increase-ratio",
                RuleExpression::PercentAsFactor {
                    percent: key("increase-value"),
                    unit: factor_unit.clone(),
                },
            ),
            node(
                "increased-factor",
                RuleExpression::Add {
                    left: key("one"),
                    right: key("increase-ratio"),
                },
            ),
            node(
                "quality-ratio",
                RuleExpression::PercentAsFactor {
                    percent: key("quality-value"),
                    unit: factor_unit.clone(),
                },
            ),
            node(
                "quality-factor",
                RuleExpression::Add {
                    left: key("one"),
                    right: key("quality-ratio"),
                },
            ),
            node(
                "selected-quality",
                RuleExpression::Select {
                    condition: key("has-quality-value"),
                    when_true: key("quality-factor"),
                    when_false: key("one"),
                },
            ),
            node(
                "increased",
                RuleExpression::Scale {
                    value: key("base-plus-flat"),
                    factor: key("increased-factor"),
                },
            ),
            node(
                "qualified",
                RuleExpression::Scale {
                    value: key("increased"),
                    factor: key("selected-quality"),
                },
            ),
            node(
                "rounded",
                RuleExpression::Round {
                    value: key("qualified"),
                    quantum: FiniteQuantity::new(1.0, damage_unit.clone()).unwrap(),
                    mode: RuleRounding::NearestTiesPositive,
                },
            ),
        ],
        effects: vec![effect(
            "local-result",
            None,
            RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: local_stat,
                value: key("rounded"),
            },
        )],
    };
    let attribute = RuleProgram {
        id: key("attribute-conditioned"),
        context: RuleEntityKind::Actor,
        reads: vec![read(
            "attribute",
            ComputedValueType::Integer,
            RuleReadSource::Stat {
                entity: RuleEntity::Actor,
                stat: attribute_stat.clone(),
            },
        )],
        nodes: vec![
            read_node("attribute-value", "attribute"),
            literal("threshold", integer(100)),
            literal("bonus", quantity(8.0, &damage_unit)),
            node(
                "eligible",
                RuleExpression::Compare {
                    operation: RuleComparison::GreaterOrEqual,
                    left: key("attribute-value"),
                    right: key("threshold"),
                },
            ),
        ],
        effects: vec![effect(
            "conditional-bonus",
            Some("eligible"),
            RuleEffectKind::Contribute {
                entity: RuleEntity::Actor,
                stat: damage_stat.clone(),
                contribution: ContributionKind::Add,
                value: key("bonus"),
            },
        )],
    };
    let support_program = RuleProgram {
        id: key("support-applicability"),
        context: RuleEntityKind::Action,
        reads: vec![read(
            "capability",
            ComputedValueType::Boolean,
            RuleReadSource::Capability {
                entity: RuleEntity::Current,
                capability: supported_capability,
            },
        )],
        nodes: vec![
            read_node("applicable", "capability"),
            literal("factor", quantity(1.5, &factor_unit)),
        ],
        effects: vec![
            effect(
                "applicability",
                None,
                RuleEffectKind::SupportApplicability {
                    applicable: key("applicable"),
                },
            ),
            effect(
                "supported-factor",
                Some("applicable"),
                RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: damage_stat,
                    contribution: ContributionKind::Multiply,
                    value: key("factor"),
                },
            ),
        ],
    };
    let grant_program = RuleProgram {
        id: key("conditional-grant"),
        context: RuleEntityKind::Actor,
        reads: vec![read(
            "attribute",
            ComputedValueType::Integer,
            RuleReadSource::Stat {
                entity: RuleEntity::Actor,
                stat: attribute_stat,
            },
        )],
        nodes: vec![
            read_node("attribute-value", "attribute"),
            literal("threshold", integer(100)),
            node(
                "enabled",
                RuleExpression::Compare {
                    operation: RuleComparison::GreaterOrEqual,
                    left: key("attribute-value"),
                    right: key("threshold"),
                },
            ),
        ],
        effects: vec![effect(
            "actor-grant",
            None,
            RuleEffectKind::ActivateGrant {
                slot: grant,
                enabled: key("enabled"),
            },
        )],
    };
    let rules = RulePackageInput {
        tables: vec![],
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: namespace(),
        release: key("test-rules"),
        semantics_version: key("synthetic-component-coefficients-v1"),
        operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
        definitions: schema.identity().clone(),
        owners: vec![
            DefinitionRules {
                owner: subject(&item),
                programs: DeclaredSet::complete(vec![local, grant_program]),
            },
            DefinitionRules {
                owner: subject(&modifier),
                programs: DeclaredSet::complete(vec![attribute]),
            },
            DefinitionRules {
                owner: subject(&support),
                programs: DeclaredSet::complete(vec![support_program]),
            },
        ],
    };
    let mut cases = vec![];
    for (present, result) in [(true, 165.0), (false, 132.0)] {
        let mut facts = vec![
            fact("flat", quantity(10.0, &damage_unit)),
            fact("increase", quantity(20.0, &percent_unit)),
            fact("has-quality", ParameterValue::Boolean(present)),
        ];
        if present {
            facts.push(fact("quality", quantity(25.0, &percent_unit)));
        }
        cases.push(Case {
            name: if present {
                "local quality present"
            } else {
                "local quality absent"
            },
            owner: subject(&item),
            program: key("local-item"),
            facts,
            expected: vec![expected(
                "local-result",
                Some(quantity(result, &damage_unit)),
            )],
        });
    }
    for (eligible, value) in [(true, 100), (false, 99)] {
        cases.push(Case {
            name: if eligible {
                "attribute threshold met"
            } else {
                "attribute threshold missed"
            },
            owner: subject(&modifier),
            program: key("attribute-conditioned"),
            facts: vec![fact("attribute", integer(value))],
            expected: vec![expected(
                "conditional-bonus",
                eligible.then(|| quantity(8.0, &damage_unit)),
            )],
        });
        cases.push(Case {
            name: if eligible {
                "support applicable"
            } else {
                "support inapplicable"
            },
            owner: subject(&support),
            program: key("support-applicability"),
            facts: vec![fact("capability", ParameterValue::Boolean(eligible))],
            expected: vec![
                expected("applicability", Some(ParameterValue::Boolean(eligible))),
                expected(
                    "supported-factor",
                    eligible.then(|| quantity(1.5, &factor_unit)),
                ),
            ],
        });
        cases.push(Case {
            name: if eligible {
                "grant enabled"
            } else {
                "grant disabled"
            },
            owner: subject(&item),
            program: key("conditional-grant"),
            facts: vec![fact("attribute", integer(value))],
            expected: vec![expected(
                "actor-grant",
                Some(ParameterValue::Boolean(eligible)),
            )],
        });
    }
    Fixture {
        schema,
        rules,
        cases,
    }
}
