//! Directly authored projection contracts, independent of source formats and gems.
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::{
    owned_rules::{OwnedRulePackage, RuleStorageLimits, decode_rule_package, encode_rule_package},
    owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits, SchemaPackageInput},
};
use poe_optimizer_engine::owned_rules::*;

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn namespace() -> GameVersionNamespace {
    GameVersionNamespace::new("projection-tests", "v1").unwrap()
}
fn id<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::parse(namespace(), s).unwrap()
}
fn integer(v: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(v).unwrap())
}
fn range(a: i64, b: i64) -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(a).unwrap(),
        maximum: BoundedInteger::new(b).unwrap(),
    }
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
fn gap(subject: SchemaSubject) -> SchemaGap {
    SchemaGap {
        subject,
        facet: SchemaFacet::InputSchema,
        code: key("unconverted"),
    }
}
fn parameter(owner: SlotOwnerDefId, name: &str) -> DeclaredSlot<ParameterSlotDefId> {
    DeclaredSlot {
        declaration: owner,
        slot: id(name),
    }
}
fn source_parameter() -> DeclaredSlot<ParameterSlotDefId> {
    parameter(SlotOwnerDefId::ItemTemplate(id("item")), "intrinsic-level")
}
fn target_parameter() -> DeclaredSlot<ParameterSlotDefId> {
    parameter(SlotOwnerDefId::Skill(id("skill")), "skill-level")
}
fn grant() -> DeclaredSlot<SkillGrantSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::ItemTemplate(id("item")),
        slot: id("supplied-skill"),
    }
}
fn owner() -> SchemaSubject {
    SchemaSubject::Definition(id::<ItemTemplateDefinition>("item").address())
}
fn slot_schema<'a>(
    input: &'a mut SchemaPackageInput,
    target: &DeclaredSlot<ParameterSlotDefId>,
) -> &'a mut ParameterSlotSchema {
    input
        .slots
        .iter_mut()
        .find_map(|slot| match slot {
            SlotDescriptor::Parameter(e) if &e.id == target => match &mut e.schema {
                SchemaState::Known(v) => Some(v),
                _ => panic!("known fixture parameter"),
            },
            _ => None,
        })
        .unwrap()
}
fn item_schema(input: &mut SchemaPackageInput) -> &mut ItemTemplateSchema {
    input
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::ItemTemplate(e) if e.id == id("item") => match &mut e.schema {
                SchemaState::Known(v) => Some(v),
                _ => panic!("known fixture item"),
            },
            _ => None,
        })
        .unwrap()
}
fn skill_schema(input: &mut SchemaPackageInput) -> &mut SkillSchema {
    input
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::Skill(e) if e.id == id("skill") => match &mut e.schema {
                SchemaState::Known(v) => Some(v),
                _ => panic!("known fixture skill"),
            },
            _ => None,
        })
        .unwrap()
}
struct Fixture {
    schema: OwnedDefinitionSchemaPackage,
    rules: RulePackageInput,
}
impl Fixture {
    fn schema(&mut self, change: impl FnOnce(&mut SchemaPackageInput)) {
        let mut input = self.schema.input().clone();
        change(&mut input);
        self.schema =
            OwnedDefinitionSchemaPackage::new(input, OwnedSchemaLimits::default()).unwrap();
        self.rules.definitions = self.schema.identity().clone();
    }
    fn program(&mut self) -> &mut RuleProgram {
        &mut self.rules.owners[0].programs.members[0]
    }
    fn compile(&self) -> CompiledRulePackage {
        CompiledRulePackage::compile(&self.rules, &self.schema, RuleLimits::default()).unwrap()
    }
    fn rejects(&self, message: &str) {
        let error = CompiledRulePackage::compile(&self.rules, &self.schema, RuleLimits::default())
            .unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
    }
    fn evaluate(&self, value: Option<ParameterValue>) -> EffectDisposition {
        let compiled = self.compile();
        let facts: Vec<_> = value
            .into_iter()
            .map(|value| RuleFact {
                read: key("intrinsic"),
                value,
            })
            .collect();
        let result = compiled
            .evaluate(
                &owner(),
                &key("project"),
                &facts,
                &self.schema,
                &mut compiled.new_scratch(),
            )
            .unwrap();
        assert_eq!(result.effects.len(), 1);
        assert_eq!(
            result.effects[0].effect,
            self.rules.owners[0].programs.members[0].effects[0].effect
        );
        result.effects.into_iter().next().unwrap().disposition
    }
    fn identity_expression(&mut self) {
        let p = self.program();
        p.nodes.truncate(1);
        let RuleEffectKind::ProjectSkillParameter { value, .. } = &mut p.effects[0].effect else {
            panic!()
        };
        *value = key("input");
    }
}
fn fixture() -> Fixture {
    let mut item_ports = declarations();
    item_ports.parameters.members.push(source_parameter());
    item_ports.skill_grants.members.push(grant());
    let mut skill_ports = declarations();
    skill_ports.parameters.members.push(target_parameter());
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: 1,
            namespace: namespace(),
            release: key("fixture"),
            semantics_version: key("projection-schema-v1"),
            definitions: vec![
                DefinitionDescriptor::ItemTemplate(entry(
                    id("item"),
                    ItemTemplateSchema {
                        item_level: range(1, 100),
                        equipment_slots: DeclaredSet::complete(vec![]),
                        socket_destinations: DeclaredSet::complete(vec![]),
                        modifiers: DeclaredSet::complete(vec![]),
                        quality: QualityUseSchema {
                            presence: QualityPresence::Forbidden,
                            allowed_kinds: DeclaredSet::complete(vec![]),
                        },
                        declarations: item_ports,
                    },
                )),
                DefinitionDescriptor::Skill(entry(
                    id("skill"),
                    SkillSchema {
                        directly_selectable: false,
                        declarations: skill_ports,
                    },
                )),
                DefinitionDescriptor::Skill(entry(
                    id("other-skill"),
                    SkillSchema {
                        directly_selectable: false,
                        declarations: declarations(),
                    },
                )),
            ],
            slots: vec![
                SlotDescriptor::Parameter(entry(
                    source_parameter(),
                    ParameterSlotSchema {
                        value: ValueSchema::Integer(range(0, 100)),
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![ParameterSite::ItemParameter],
                    },
                )),
                SlotDescriptor::SkillGrant(entry(
                    grant(),
                    SkillGrantSlotSchema {
                        skill: id("skill"),
                        outputs: DeclaredSet::complete(vec![]),
                    },
                )),
                SlotDescriptor::Parameter(entry(
                    target_parameter(),
                    ParameterSlotSchema {
                        value: ValueSchema::Integer(range(1, 20)),
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![],
                    },
                )),
            ],
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let rules = RulePackageInput {
        receivers: DeclaredSet::complete(vec![]),
        tables: vec![],
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: namespace(),
        release: key("fixture"),
        semantics_version: key("projection-rules-v1"),
        operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
        definitions: schema.identity().clone(),
        owners: vec![DefinitionRules {
            owner: owner(),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key("project"),
                context: RuleEntityKind::EquipmentUse,
                reads: vec![RuleRead {
                    id: key("intrinsic"),
                    value_type: ComputedValueType::Integer,
                    source: RuleReadSource::Parameter {
                        slot: source_parameter(),
                    },
                }],
                nodes: vec![
                    RuleNode {
                        id: key("input"),
                        expression: RuleExpression::Read {
                            input: key("intrinsic"),
                        },
                    },
                    RuleNode {
                        id: key("offset"),
                        expression: RuleExpression::Literal { value: integer(0) },
                    },
                    RuleNode {
                        id: key("computed"),
                        expression: RuleExpression::Add {
                            left: key("input"),
                            right: key("offset"),
                        },
                    },
                ],
                effects: vec![RuleEffect {
                    id: key("project-level"),
                    when: None,
                    effect: RuleEffectKind::ProjectSkillParameter {
                        skill: grant(),
                        parameter: target_parameter(),
                        value: key("computed"),
                    },
                }],
            }]),
        }],
    };
    Fixture { schema, rules }
}

#[test]
fn intrinsic_item_input_projects_into_exact_skill_parameter_without_a_gem_or_activation() {
    let f = fixture();
    assert!(
        f.schema
            .input()
            .definitions
            .iter()
            .all(|d| !matches!(d, DefinitionDescriptor::Gem(_)))
    );
    assert_eq!(
        f.evaluate(Some(integer(11))),
        EffectDisposition::Applied { value: integer(11) }
    );
    assert_eq!(
        f.evaluate(Some(integer(1))),
        EffectDisposition::Applied { value: integer(1) }
    );
    assert_eq!(
        f.evaluate(Some(integer(20))),
        EffectDisposition::Applied { value: integer(20) }
    );
    let target_id = target_parameter();
    let target = f.schema.slot(&target_id);
    assert!(matches!(target, SchemaLookup::Known(s) if s.sites.is_empty()));
    assert_eq!(f.rules.owners[0].programs.members[0].effects.len(), 1);
}

#[test]
fn computed_out_of_supported_domain_retains_value_and_does_not_clamp() {
    let mut f = fixture();
    f.program().nodes[1].expression = RuleExpression::Literal { value: integer(2) };
    assert_eq!(
        f.evaluate(Some(integer(19))),
        EffectDisposition::UnsupportedValue { value: integer(21) }
    );
    assert_eq!(
        f.evaluate(Some(integer(10))),
        EffectDisposition::Applied { value: integer(12) }
    );
    f.program().nodes[1].expression = RuleExpression::Literal { value: integer(-2) };
    assert_eq!(
        f.evaluate(Some(integer(0))),
        EffectDisposition::UnsupportedValue { value: integer(-2) }
    );
}

#[test]
fn false_guard_skips_missing_projection_value_but_true_guard_keeps_it_unresolved() {
    let mut f = fixture();
    f.program().nodes.push(RuleNode {
        id: key("guard"),
        expression: RuleExpression::Literal {
            value: ParameterValue::Boolean(false),
        },
    });
    f.program().effects[0].when = Some(key("guard"));
    assert_eq!(f.evaluate(None), EffectDisposition::Inactive);
    f.program().nodes.last_mut().unwrap().expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(true),
    };
    assert_eq!(
        f.evaluate(None),
        EffectDisposition::Unresolved {
            input: key("intrinsic")
        }
    );
}

#[test]
fn source_declaration_and_target_skill_or_slot_are_exact() {
    let mut f = fixture();
    let RuleEffectKind::ProjectSkillParameter { skill, .. } = &mut f.program().effects[0].effect
    else {
        panic!()
    };
    skill.declaration = SlotOwnerDefId::Skill(id("skill"));
    f.rejects("projected skill declaration does not equal owner");
    let mut f = fixture();
    let RuleEffectKind::ProjectSkillParameter { parameter, .. } =
        &mut f.program().effects[0].effect
    else {
        panic!()
    };
    parameter.declaration = SlotOwnerDefId::Skill(id("other-skill"));
    f.rejects("projected parameter declaration does not equal target skill");
    let mut f = fixture();
    let RuleEffectKind::ProjectSkillParameter { parameter, .. } =
        &mut f.program().effects[0].effect
    else {
        panic!()
    };
    parameter.slot = id("unlisted");
    f.rejects("slot is not a declared member");
}

#[test]
fn unknown_required_target_schemas_reject_even_with_false_guard() {
    for which in 0..3 {
        let mut f = fixture();
        f.schema(|input| match which {
            0 => {
                let s = input
                    .slots
                    .iter_mut()
                    .find_map(|s| match s {
                        SlotDescriptor::SkillGrant(e) => Some(e),
                        _ => None,
                    })
                    .unwrap();
                s.schema = SchemaState::Unmapped {
                    gaps: vec![gap(SchemaSubject::Slot(SkillGrantSlotDefId::address(
                        &s.id,
                    )))],
                };
            }
            1 => {
                let s = input
                    .definitions
                    .iter_mut()
                    .find_map(|s| match s {
                        DefinitionDescriptor::Skill(e) if e.id == id("skill") => Some(e),
                        _ => None,
                    })
                    .unwrap();
                s.schema = SchemaState::Unmapped {
                    gaps: vec![gap(SchemaSubject::Definition(s.id.address()))],
                };
            }
            _ => {
                let s = input
                    .slots
                    .iter_mut()
                    .find_map(|s| match s {
                        SlotDescriptor::Parameter(e) if e.id == target_parameter() => Some(e),
                        _ => None,
                    })
                    .unwrap();
                s.schema = SchemaState::Unmapped {
                    gaps: vec![gap(SchemaSubject::Slot(ParameterSlotDefId::address(&s.id)))],
                };
            }
        });
        f.program().nodes.push(RuleNode {
            id: key("guard"),
            expression: RuleExpression::Literal {
                value: ParameterValue::Boolean(false),
            },
        });
        f.program().effects[0].when = Some(key("guard"));
        f.rejects("missing, unmapped");
    }
}

#[test]
fn listed_partial_members_compile_but_unlisted_siblings_are_not_authorized() {
    let mut f = fixture();
    f.schema(|input| {
        item_schema(input).declarations.skill_grants.closure = SchemaClosure::Partial {
            gaps: vec![gap(owner())],
        };
        skill_schema(input).declarations.parameters.closure = SchemaClosure::Partial {
            gaps: vec![gap(SchemaSubject::Definition(
                id::<SkillDefinition>("skill").address(),
            ))],
        };
    });
    assert_eq!(
        f.evaluate(Some(integer(11))),
        EffectDisposition::Applied { value: integer(11) }
    );
    // These memberships stay partial: projecting one known cell is no required-input closure proof.
    assert!(
        matches!(f.schema.definition(&id::<SkillDefinition>("skill")), SchemaLookup::Known(s) if !s.declarations.parameters.is_complete())
    );
    let mut sibling = target_parameter();
    sibling.slot = id("unlisted-sibling");
    f.schema(|input| {
        input.slots.push(SlotDescriptor::Parameter(entry(
            sibling.clone(),
            ParameterSlotSchema {
                value: ValueSchema::Integer(range(1, 20)),
                presence: SlotPresence::RequiredOnce,
                sites: vec![],
            },
        )))
    });
    let RuleEffectKind::ProjectSkillParameter { parameter, .. } =
        &mut f.program().effects[0].effect
    else {
        panic!()
    };
    *parameter = sibling;
    f.rejects("slot is not a declared member");
    let mut sibling = grant();
    sibling.slot = id("unlisted-source-sibling");
    f.schema(|input| {
        input.slots.push(SlotDescriptor::SkillGrant(entry(
            sibling.clone(),
            SkillGrantSlotSchema {
                skill: id("skill"),
                outputs: DeclaredSet::complete(vec![]),
            },
        )))
    });
    let RuleEffectKind::ProjectSkillParameter {
        skill, parameter, ..
    } = &mut f.program().effects[0].effect
    else {
        panic!()
    };
    *skill = sibling;
    *parameter = target_parameter();
    f.rejects("slot is not a declared member");
}

#[test]
fn projection_checks_value_type_and_exact_quantity_unit() {
    let mut f = fixture();
    f.program().nodes = vec![RuleNode {
        id: key("computed"),
        expression: RuleExpression::Literal {
            value: ParameterValue::Boolean(true),
        },
    }];
    f.rejects("effect value type/unit mismatch");
    let mut f = fixture();
    let a: UnitDefId = id("unit.a");
    let b: UnitDefId = id("unit.b");
    f.schema(|input| {
        for unit in [&a, &b] {
            input.definitions.push(DefinitionDescriptor::Unit(entry(
                unit.clone(),
                UnitSchema {
                    dimension: UnitDimension::Count,
                },
            )));
        }
        slot_schema(input, &source_parameter()).value = ValueSchema::Quantity(QuantityRange {
            minimum: FiniteQuantity::new(0.0, a.clone()).unwrap(),
            maximum: FiniteQuantity::new(100.0, a.clone()).unwrap(),
        });
        slot_schema(input, &target_parameter()).value = ValueSchema::Quantity(QuantityRange {
            minimum: FiniteQuantity::new(1.0, b.clone()).unwrap(),
            maximum: FiniteQuantity::new(20.0, b.clone()).unwrap(),
        });
    });
    f.program().reads[0].value_type = ComputedValueType::Quantity { unit: a.clone() };
    f.identity_expression();
    f.rejects("effect value type/unit mismatch");
    f.schema(|input| {
        slot_schema(input, &target_parameter()).value = ValueSchema::Quantity(QuantityRange {
            minimum: FiniteQuantity::new(1.0, a.clone()).unwrap(),
            maximum: FiniteQuantity::new(20.0, a.clone()).unwrap(),
        })
    });
    let value = ParameterValue::Quantity(FiniteQuantity::new(21.0, a).unwrap());
    assert_eq!(
        f.evaluate(Some(value.clone())),
        EffectDisposition::UnsupportedValue { value }
    );
}

#[test]
fn option_projection_checks_known_members_without_inventing_partial_coverage() {
    let mut f = fixture();
    let a: OptionDefId = id("option.a");
    let b: OptionDefId = id("option.b");
    f.schema(|input| {
        for option in [&a, &b] {
            input.definitions.push(DefinitionDescriptor::Option(entry(
                option.clone(),
                OptionSchema {},
            )));
        }
        slot_schema(input, &source_parameter()).value = ValueSchema::Option {
            allowed: DeclaredSet::complete(vec![a.clone(), b.clone()]),
        };
        slot_schema(input, &target_parameter()).value = ValueSchema::Option {
            allowed: DeclaredSet::partial(
                vec![a.clone()],
                vec![gap(SchemaSubject::Slot(ParameterSlotDefId::address(
                    &target_parameter(),
                )))],
            ),
        };
    });
    f.program().reads[0].value_type = ComputedValueType::Option;
    f.identity_expression();
    assert_eq!(
        f.evaluate(Some(ParameterValue::Option(a.clone()))),
        EffectDisposition::Applied {
            value: ParameterValue::Option(a)
        }
    );
    assert_eq!(
        f.evaluate(Some(ParameterValue::Option(b.clone()))),
        EffectDisposition::UnsupportedValue {
            value: ParameterValue::Option(b.clone())
        }
    );
    f.schema(|input| {
        let option = input
            .definitions
            .iter_mut()
            .find_map(|d| match d {
                DefinitionDescriptor::Option(e) if e.id == b => Some(e),
                _ => None,
            })
            .unwrap();
        option.schema = SchemaState::Unmapped {
            gaps: vec![gap(SchemaSubject::Definition(option.id.address()))],
        };
    });
    f.rejects("missing, unmapped");
}

#[test]
fn projection_storage_counts_value_edges_and_compiler_rejects_v1() {
    let f = fixture();
    let package =
        OwnedRulePackage::new(f.rules.clone(), &f.schema, RuleStorageLimits::default()).unwrap();
    assert_eq!(package.resources().edges, 4); // Read, two Add inputs, projection value.
    let bytes = encode_rule_package(&package, RuleStorageLimits::default()).unwrap();
    let decoded = decode_rule_package(&bytes, &f.schema, RuleStorageLimits::default()).unwrap();
    assert_eq!(decoded.input(), &f.rules);
    assert!(
        OwnedRulePackage::new(
            f.rules.clone(),
            &f.schema,
            RuleStorageLimits {
                max_edges: 3,
                ..RuleStorageLimits::default()
            }
        )
        .is_err()
    );
    assert!(
        CompiledRulePackage::compile(
            &f.rules,
            &f.schema,
            RuleLimits {
                max_edges: 3,
                ..RuleLimits::default()
            }
        )
        .is_err()
    );
    let mut f = fixture();
    f.rules.operations_version = key("owned-domain-operations-v1");
    f.rejects("unsupported operation version");
    let mut f = fixture();
    let RuleEffectKind::ProjectSkillParameter { value, .. } = &mut f.program().effects[0].effect
    else {
        panic!()
    };
    *value = key("missing");
    assert!(
        OwnedRulePackage::new(f.rules.clone(), &f.schema, RuleStorageLimits::default()).is_err()
    );
    f.rejects("unknown effect node");
}
