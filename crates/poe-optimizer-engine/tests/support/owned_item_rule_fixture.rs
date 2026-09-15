//! Injected component rules for the optional item oracle comparison.
//! Supplied local reductions are test facts, not an incoming-effect/legality proof.
//! No source/profile code or reference output participates in native evaluation.
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{
    OwnedDefinitionSchemaPackage, OwnedSchemaLimits, SchemaPackageInput,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, RuleFact, RuleLimits};

pub const CHANNELS: [&str; 5] = ["Physical", "Lightning", "Cold", "Fire", "Chaos"];
pub fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s.to_ascii_lowercase()).unwrap()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("item-oracle-components", "v1").unwrap()
}
fn id<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::parse(namespace(), s.to_ascii_lowercase()).unwrap()
}
fn quantity(v: f64, unit: &str) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(v, id(unit)).unwrap())
}
pub fn number_fact(name: &str, v: f64, unit: &str) -> RuleFact {
    RuleFact {
        read: key(name),
        value: quantity(v, unit),
    }
}
pub fn boolean_fact(name: &str, v: bool) -> RuleFact {
    RuleFact {
        read: key(name),
        value: ParameterValue::Boolean(v),
    }
}
pub fn level_fact(v: i64) -> RuleFact {
    RuleFact {
        read: key("decoded-level"),
        value: ParameterValue::Integer(BoundedInteger::new(v).unwrap()),
    }
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
fn integer_range(a: i64, b: i64) -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(a).unwrap(),
        maximum: BoundedInteger::new(b).unwrap(),
    }
}
fn kind(unit: &str) -> ComputedValueType {
    ComputedValueType::Quantity { unit: id(unit) }
}
fn owner(name: &str) -> SchemaSubject {
    SchemaSubject::Definition(id::<ItemTemplateDefinition>(name).address())
}
fn parameter(owner: SlotOwnerDefId, name: &str) -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: owner,
        slot: id(name),
    }
}

pub struct Fixture {
    pub schema: OwnedDefinitionSchemaPackage,
    pub compiled: CompiledRulePackage,
    pub weapon_owner: SchemaSubject,
    pub staff_owner: SchemaSubject,
}
struct Graph {
    program: RuleProgram,
    definitions: Vec<DefinitionDescriptor>,
}
impl Graph {
    fn new() -> Self {
        Self {
            program: RuleProgram {
                id: key("weapon"),
                context: RuleEntityKind::EquipmentUse,
                reads: vec![],
                nodes: vec![],
                effects: vec![],
            },
            definitions: vec![],
        }
    }
    fn node(&mut self, name: &str, expression: RuleExpression) {
        self.program.nodes.push(RuleNode {
            id: key(name),
            expression,
        });
    }
    fn stat(&mut self, name: &str, unit: &str) {
        self.definitions.push(DefinitionDescriptor::Stat(known(
            id(name),
            StatSchema {
                value: kind(unit),
                targets: vec![RuleEntityKind::EquipmentUse],
            },
        )));
    }
    fn read(&mut self, name: &str, ty: ComputedValueType, source: RuleReadSource) {
        self.program.reads.push(RuleRead {
            id: key(name),
            value_type: ty,
            source,
        });
        self.node(name, RuleExpression::Read { input: key(name) });
    }
    fn input_stat(&mut self, name: &str, unit: &str) {
        self.stat(name, unit);
        self.read(
            name,
            kind(unit),
            RuleReadSource::Stat {
                entity: RuleEntity::Current,
                stat: id(name),
            },
        );
    }
    fn contribution(
        &mut self,
        name: &str,
        target: &str,
        unit: &str,
        contribution: ContributionKind,
    ) {
        self.read(
            name,
            kind(unit),
            RuleReadSource::Contributions {
                entity: RuleEntity::Current,
                stat: id(target),
                contribution,
                reduction: ContributionReduction::Sum,
                empty: quantity(0.0, unit),
            },
        );
    }
    fn literal(&mut self, name: &str, value: ParameterValue) {
        self.node(name, RuleExpression::Literal { value });
    }
    fn add(&mut self, out: &str, left: &str, right: &str) {
        self.node(
            out,
            RuleExpression::Add {
                left: key(left),
                right: key(right),
            },
        );
    }
    fn scale(&mut self, out: &str, value: &str, factor: &str) {
        self.node(
            out,
            RuleExpression::Scale {
                value: key(value),
                factor: key(factor),
            },
        );
    }
    fn factor(&mut self, name: &str, percent: &str) {
        let ratio = format!("{name}.ratio");
        self.node(
            &ratio,
            RuleExpression::PercentAsFactor {
                percent: key(percent),
                unit: id("factor"),
            },
        );
        self.add(name, "one", &ratio);
    }
    fn round(&mut self, out: &str, value: &str, quantum: f64, unit: &str) {
        self.node(
            out,
            RuleExpression::Round {
                value: key(value),
                quantum: FiniteQuantity::new(quantum, id(unit)).unwrap(),
                mode: RuleRounding::NearestTiesPositive,
            },
        );
    }
    fn derive(&mut self, name: &str, value: &str, guard: Option<&str>) {
        self.program.effects.push(RuleEffect {
            id: key(name),
            when: guard.map(key),
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: id(name),
                value: key(value),
            },
        });
    }
}

pub fn fixture() -> Fixture {
    let mut g = Graph::new();
    g.literal("one", quantity(1.0, "factor"));
    g.literal("damage.zero", quantity(0.0, "damage"));
    g.read(
        "quality",
        kind("percent"),
        RuleReadSource::ItemQualityAmount {
            quality: id("weapon-quality"),
        },
    );
    g.factor("quality.factor", "quality");
    for channel in CHANNELS {
        let available = format!("{channel}.present");
        g.definitions.push(DefinitionDescriptor::Capability(known(
            id(&available),
            CapabilitySchema {
                targets: vec![RuleEntityKind::EquipmentUse],
            },
        )));
        g.read(
            &available,
            ComputedValueType::Boolean,
            RuleReadSource::Capability {
                entity: RuleEntity::Current,
                capability: id(&available),
            },
        );
        for endpoint in ["Min", "Max"] {
            g.stat(&format!("{channel}.{endpoint}"), "damage");
        }
        let factor = if channel == "Chaos" {
            "one".to_owned()
        } else {
            let increase = format!("{channel}.increase");
            g.contribution(
                &increase,
                &format!("{channel}.Min"),
                "percent",
                ContributionKind::Increase,
            );
            let factor = format!("{channel}.factor");
            g.factor(&factor, &increase);
            factor
        };
        for endpoint in ["Min", "Max"] {
            let prefix = format!("{channel}.{endpoint}");
            let base = format!("{prefix}.base");
            let flat = format!("{prefix}.flat");
            g.input_stat(&base, "damage");
            g.contribution(&flat, &prefix, "damage", ContributionKind::Add);
            let added = format!("{prefix}.added");
            g.add(&added, &base, &flat);
            let increased = format!("{prefix}.increased");
            g.scale(&increased, &added, &factor);
            let qualified = format!("{prefix}.qualified");
            g.scale(
                &qualified,
                &increased,
                if channel == "Physical" {
                    "quality.factor"
                } else {
                    "one"
                },
            );
            let result = format!("{prefix}.result");
            if channel == "Chaos" {
                // Pinned source leaves Chaos endpoints unrounded.
                g.scale(&result, &qualified, "one");
            } else {
                g.round(&result, &qualified, 1.0, "damage");
            }
            g.node(
                &format!("{prefix}.positive"),
                RuleExpression::Compare {
                    operation: RuleComparison::Greater,
                    left: key(&result),
                    right: key("damage.zero"),
                },
            );
        }
        let guard = format!("{channel}.pair");
        g.node(
            &guard,
            RuleExpression::All {
                values: vec![
                    key(&available),
                    key(&format!("{channel}.Min.positive")),
                    key(&format!("{channel}.Max.positive")),
                ],
            },
        );
        for endpoint in ["Min", "Max"] {
            let name = format!("{channel}.{endpoint}");
            g.derive(&name, &format!("{name}.result"), Some(&guard));
        }
    }
    for (name, unit) in [("rate", "rate-unit"), ("crit", "percent")] {
        g.stat(name, unit);
        let base = format!("{name}.base");
        g.input_stat(&base, unit);
        // The pinned rate formula scales the base directly. Only critical
        // chance has an additive local term before its increase multiplier.
        let unscaled = if name == "crit" {
            let flat = format!("{name}.flat");
            g.contribution(&flat, name, unit, ContributionKind::Add);
            let added = format!("{name}.added");
            g.add(&added, &base, &flat);
            added
        } else {
            base
        };
        let inc = format!("{name}.increase");
        g.contribution(&inc, name, "percent", ContributionKind::Increase);
        let factor = format!("{name}.factor");
        g.factor(&factor, &inc);
        let scaled = format!("{name}.scaled");
        g.scale(&scaled, &unscaled, &factor);
        let rounded = format!("{name}.rounded");
        g.round(&rounded, &scaled, 0.01, unit);
        g.derive(name, &rounded, None);
    }
    let source = parameter(SlotOwnerDefId::ItemTemplate(id("staff")), "decoded-level");
    let target = parameter(SlotOwnerDefId::Skill(id("firebolt")), "level");
    let grant: DeclaredSlot<SkillGrantSlotDefId> = DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(id("staff")),
        slot: id("firebolt-grant"),
    };
    let mut staff_declarations = declarations();
    staff_declarations.parameters.members.push(source.clone());
    staff_declarations.skill_grants.members.push(grant.clone());
    let mut skill_declarations = declarations();
    skill_declarations.parameters.members.push(target.clone());
    let mut definitions = g.definitions;
    for (name, dimension) in [
        ("damage", UnitDimension::Damage),
        ("percent", UnitDimension::PercentagePoints),
        ("factor", UnitDimension::DimensionlessFactor),
        ("rate-unit", UnitDimension::Rate),
    ] {
        definitions.push(DefinitionDescriptor::Unit(known(
            id(name),
            UnitSchema { dimension },
        )));
    }
    definitions.push(DefinitionDescriptor::Quality(known(
        id("weapon-quality"),
        QualitySchema {
            amount: QuantityRange {
                minimum: FiniteQuantity::new(0.0, id("percent")).unwrap(),
                maximum: FiniteQuantity::new(100.0, id("percent")).unwrap(),
            },
        },
    )));
    for (name, declarations, quality) in [
        (
            "weapon",
            declarations(),
            QualityUseSchema {
                presence: QualityPresence::Optional,
                allowed_kinds: DeclaredSet::complete(vec![id("weapon-quality")]),
            },
        ),
        (
            "staff",
            staff_declarations,
            QualityUseSchema {
                presence: QualityPresence::Forbidden,
                allowed_kinds: DeclaredSet::complete(vec![]),
            },
        ),
    ] {
        definitions.push(DefinitionDescriptor::ItemTemplate(known(
            id(name),
            ItemTemplateSchema {
                item_level: integer_range(1, 100),
                equipment_slots: DeclaredSet::complete(vec![]),
                socket_destinations: DeclaredSet::complete(vec![]),
                modifiers: DeclaredSet::complete(vec![]),
                quality,
                declarations,
            },
        )));
    }
    definitions.push(DefinitionDescriptor::Skill(known(
        id("firebolt"),
        SkillSchema {
            directly_selectable: false,
            declarations: skill_declarations,
        },
    )));
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: 1,
            namespace: namespace(),
            release: key("pinned-item-component-test"),
            semantics_version: key("owned-item-numeric-prefix-test-v1"),
            definitions,
            slots: vec![
                SlotDescriptor::Parameter(known(
                    source.clone(),
                    ParameterSlotSchema {
                        value: ValueSchema::Integer(integer_range(1, 100)),
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![ParameterSite::ItemParameter],
                    },
                )),
                SlotDescriptor::Parameter(known(
                    target.clone(),
                    ParameterSlotSchema {
                        value: ValueSchema::Integer(integer_range(1, 20)),
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![],
                    },
                )),
                SlotDescriptor::SkillGrant(known(
                    grant.clone(),
                    SkillGrantSlotSchema {
                        skill: id("firebolt"),
                        outputs: DeclaredSet::complete(vec![]),
                    },
                )),
            ],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let staff = RuleProgram {
        id: key("grant"),
        context: RuleEntityKind::EquipmentUse,
        reads: vec![RuleRead {
            id: key("decoded-level"),
            value_type: ComputedValueType::Integer,
            source: RuleReadSource::Parameter { slot: source },
        }],
        nodes: vec![RuleNode {
            id: key("level"),
            expression: RuleExpression::Read {
                input: key("decoded-level"),
            },
        }],
        effects: vec![RuleEffect {
            id: key("firebolt-level"),
            when: None,
            effect: RuleEffectKind::ProjectSkillParameter {
                skill: grant,
                parameter: target,
                value: key("level"),
            },
        }],
    };
    let weapon_owner = owner("weapon");
    let staff_owner = owner("staff");
    let rules = RulePackageInput {
        tables: vec![],
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: namespace(),
        release: key("component-test"),
        semantics_version: key("item-numeric-prefix-and-grant-test-v1"),
        operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
        definitions: schema.identity().clone(),
        owners: vec![
            DefinitionRules {
                owner: weapon_owner.clone(),
                programs: DeclaredSet::complete(vec![g.program]),
            },
            DefinitionRules {
                owner: staff_owner.clone(),
                programs: DeclaredSet::complete(vec![staff]),
            },
        ],
    };
    let compiled = CompiledRulePackage::compile(&rules, &schema, RuleLimits::default()).unwrap();
    Fixture {
        schema,
        compiled,
        weapon_owner,
        staff_owner,
    }
}
