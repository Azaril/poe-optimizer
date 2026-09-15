//! Component projections retain exact declarations; occurrence binding is separate.
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
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("actor-projection-test", "v1").unwrap()
}
fn id<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::parse(ns(), s).unwrap()
}
fn integer(v: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(v).unwrap())
}
fn range() -> IntegerRange {
    IntegerRange {
        minimum: BoundedInteger::new(1).unwrap(),
        maximum: BoundedInteger::new(100).unwrap(),
    }
}
fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn slot<S: DefinitionDomain>(owner: &str, name: &str) -> DeclaredSlot<DefId<S>> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Gem(id(owner)),
        slot: id(name),
    }
}
fn actor(owner: &str) -> DeclaredSlot<ActorSlotDefId> {
    slot(owner, "child")
}
fn parameter(owner: &str) -> DeclaredSlot<ParameterSlotDefId> {
    slot(owner, "input-level")
}
fn subject(owner: &str) -> SchemaSubject {
    SchemaSubject::Definition(id::<GemDefinition>(owner).address())
}
fn gap(subject: SchemaSubject, facet: SchemaFacet) -> SchemaGap {
    SchemaGap {
        subject,
        facet,
        code: key("pending"),
    }
}
fn declarations(owner: &str) -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![parameter(owner)]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![slot(owner, "activation")]),
        actors: DeclaredSet::complete(vec![actor(owner)]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
struct Fixture {
    schema: OwnedDefinitionSchemaPackage,
    rules: RulePackageInput,
}
impl Fixture {
    fn program(&mut self) -> &mut RuleProgram {
        &mut self.rules.owners[0].programs.members[0]
    }
    fn schema(&mut self, edit: impl FnOnce(&mut SchemaPackageInput)) {
        let mut input = self.schema.input().clone();
        edit(&mut input);
        self.schema =
            OwnedDefinitionSchemaPackage::new(input, OwnedSchemaLimits::default()).unwrap();
        self.rules.definitions = self.schema.identity().clone();
    }
    fn stat_type(&mut self, value: ComputedValueType) {
        self.schema(|input| {
            for d in &mut input.definitions {
                if let DefinitionDescriptor::Stat(e) = d {
                    let SchemaState::Known(s) = &mut e.schema else {
                        panic!()
                    };
                    s.value = value.clone();
                }
            }
        });
    }
    fn constant(&mut self, value: ParameterValue) {
        self.program().reads.clear();
        self.program().nodes[0].expression = RuleExpression::Literal { value };
    }
    fn compile(&self) -> CompiledRulePackage {
        CompiledRulePackage::compile(&self.rules, &self.schema, RuleLimits::default()).unwrap()
    }
    fn rejects(&self, message: &str) {
        let error = CompiledRulePackage::compile(&self.rules, &self.schema, RuleLimits::default())
            .unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
    }
}
fn fixture() -> Fixture {
    let mut definitions = vec![
        DefinitionDescriptor::Stat(entry(
            id("child-level"),
            StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![RuleEntityKind::Actor],
            },
        )),
        DefinitionDescriptor::Unit(entry(
            id("count-a"),
            UnitSchema {
                dimension: UnitDimension::Count,
            },
        )),
        DefinitionDescriptor::Unit(entry(
            id("count-b"),
            UnitSchema {
                dimension: UnitDimension::Count,
            },
        )),
        DefinitionDescriptor::Option(entry(id("option-a"), OptionSchema {})),
        DefinitionDescriptor::Option(entry(id("option-b"), OptionSchema {})),
    ];
    let mut slots = Vec::new();
    let mut owners = Vec::new();
    for name in ["summoner-a", "summoner-b"] {
        definitions.push(DefinitionDescriptor::Gem(entry(
            id(name),
            GemSchema {
                level: range(),
                roles: vec![AuthoredGemRole::SkillUse],
                skills: DeclaredSet::complete(vec![]),
                quality: QualityUseSchema {
                    presence: QualityPresence::Forbidden,
                    allowed_kinds: DeclaredSet::complete(vec![]),
                },
                declarations: declarations(name),
            },
        )));
        slots.extend([
            SlotDescriptor::Actor(entry(
                actor(name),
                ActorSlotSchema {
                    skills: DeclaredSet::complete(vec![]),
                    outputs: DeclaredSet::complete(vec![]),
                },
            )),
            SlotDescriptor::Parameter(entry(
                parameter(name),
                ParameterSlotSchema {
                    value: ValueSchema::Integer(range()),
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::GemParameter],
                },
            )),
            SlotDescriptor::Grant(entry(
                slot(name, "activation"),
                GrantSlotSchema {
                    provider_roles: vec![ProviderRole::SkillUse],
                    target: GrantTarget::Actor(actor(name)),
                },
            )),
        ]);
        owners.push(DefinitionRules {
            owner: subject(name),
            programs: DeclaredSet::complete(vec![RuleProgram {
                id: key("supply-level"),
                context: RuleEntityKind::Actor,
                reads: vec![RuleRead {
                    id: key("level"),
                    value_type: ComputedValueType::Integer,
                    source: RuleReadSource::GemLevel,
                }],
                nodes: vec![RuleNode {
                    id: key("value"),
                    expression: RuleExpression::Read {
                        input: key("level"),
                    },
                }],
                effects: vec![RuleEffect {
                    id: key("project"),
                    when: None,
                    effect: RuleEffectKind::ProjectActorStat {
                        actor: actor(name),
                        stat: id("child-level"),
                        value: key("value"),
                    },
                }],
            }]),
        });
    }
    let schema = OwnedDefinitionSchemaPackage::new(
        SchemaPackageInput {
            schema_version: poe_optimizer_data::owned_schema::OWNED_SCHEMA_PACKAGE_VERSION,
            namespace: ns(),
            release: key("fixture"),
            semantics_version: key("actor-projection-schema-v1"),
            definitions,
            slots,
        },
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let rules = RulePackageInput {
        receivers: DeclaredSet::complete(vec![]),
        tables: vec![],
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("fixture"),
        semantics_version: key("actor-projection-rules-v1"),
        operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
        definitions: schema.identity().clone(),
        owners,
    };
    Fixture { schema, rules }
}
fn evaluate(
    f: &Fixture,
    c: &CompiledRulePackage,
    owner: &str,
    facts: &[RuleFact],
    scratch: &mut RuleScratch,
) -> ProgramEvaluation {
    c.evaluate(
        &subject(owner),
        &key("supply-level"),
        facts,
        &f.schema,
        scratch,
    )
    .unwrap()
}
fn fact(level: i64) -> RuleFact {
    RuleFact {
        read: key("level"),
        value: integer(level),
    }
}

#[test]
fn separate_summoner_values_and_targets_survive_shared_component_scratch() {
    let f = fixture();
    let c = f.compile();
    let mut scratch = c.new_scratch();
    // Repeated same-definition providers reuse this component with different
    // facts. Their public occurrence keys are deliberately the resolver's job.
    for (owner, level) in [
        ("summoner-a", 11),
        ("summoner-a", 20),
        ("summoner-b", 7),
        ("summoner-a", 11),
    ] {
        let result = evaluate(&f, &c, owner, &[fact(level)], &mut scratch);
        assert_eq!(result.effects.len(), 1); // No implicit activation or stat write to Player.
        assert_eq!(
            result.effects[0].effect,
            RuleEffectKind::ProjectActorStat {
                actor: actor(owner),
                stat: id("child-level"),
                value: key("value"),
            }
        );
        assert_eq!(
            result.effects[0].disposition,
            EffectDisposition::Applied {
                value: integer(level)
            }
        );
    }
    let missing = evaluate(&f, &c, "summoner-a", &[], &mut scratch);
    assert_eq!(
        missing.effects[0].disposition,
        EffectDisposition::Unresolved {
            input: key("level")
        }
    );
}

#[test]
fn projection_guard_is_lazy_and_explicit_false_activation_is_independent() {
    let mut f = fixture();
    f.program().nodes.push(RuleNode {
        id: key("disabled"),
        expression: RuleExpression::Literal {
            value: ParameterValue::Boolean(false),
        },
    });
    f.program().effects[0].when = Some(key("disabled"));
    f.program().effects.push(RuleEffect {
        id: key("activate"),
        when: None,
        effect: RuleEffectKind::ActivateGrant {
            slot: slot("summoner-a", "activation"),
            enabled: key("disabled"),
        },
    });
    let c = f.compile();
    let result = evaluate(&f, &c, "summoner-a", &[], &mut c.new_scratch());
    assert_eq!(result.effects[0].disposition, EffectDisposition::Inactive);
    assert_eq!(
        result.effects[1].disposition,
        EffectDisposition::Applied {
            value: ParameterValue::Boolean(false)
        }
    );
    f.program().effects[0].when = None;
    let c = f.compile();
    let result = evaluate(&f, &c, "summoner-a", &[fact(20)], &mut c.new_scratch());
    assert_eq!(
        result.effects[0].disposition,
        EffectDisposition::Applied { value: integer(20) }
    );
    assert_eq!(
        result.effects[1].disposition,
        EffectDisposition::Applied {
            value: ParameterValue::Boolean(false)
        }
    );
}

#[test]
fn exact_actor_declaration_membership_and_known_schema_are_required() {
    let mut f = fixture();
    let RuleEffectKind::ProjectActorStat { actor: target, .. } = &mut f.program().effects[0].effect
    else {
        panic!()
    };
    *target = actor("summoner-b"); // Real registered slot, wrong declaring owner.
    f.rejects("projected actor declaration does not equal owner");

    let mut f = fixture();
    f.schema(|input| {
        let DefinitionDescriptor::Gem(e) = input
            .definitions
            .iter_mut()
            .find(|d| d.address() == id::<GemDefinition>("summoner-a").address())
            .unwrap()
        else {
            panic!()
        };
        let SchemaState::Known(gem) = &mut e.schema else {
            panic!()
        };
        gem.declarations.actors = DeclaredSet::partial(
            vec![],
            vec![gap(subject("summoner-a"), SchemaFacet::StaticLinks)],
        );
    });
    f.rejects("slot is not a declared member");

    let mut f = fixture();
    f.schema(|input| {
        let SlotDescriptor::Actor(e) = input
            .slots
            .iter_mut()
            .find(|d| d.address() == ActorSlotDefId::address(&actor("summoner-a")))
            .unwrap()
        else {
            panic!()
        };
        e.schema = SchemaState::Unmapped {
            gaps: vec![gap(
                SchemaSubject::Slot(ActorSlotDefId::address(&e.id)),
                SchemaFacet::StaticLinks,
            )],
        };
    });
    f.rejects("missing, unmapped");

    let mut f = fixture();
    f.constant(integer(10));
    f.rules.owners[0].owner = SchemaSubject::Slot(ActorSlotDefId::address(&actor("summoner-a")));
    f.rejects("projected actor declaration does not equal owner");
}

#[test]
fn actor_target_type_and_exact_unit_are_checked_without_implicit_conversion() {
    let mut f = fixture();
    f.schema(|input| {
        let DefinitionDescriptor::Stat(e) = input
            .definitions
            .iter_mut()
            .find(|d| matches!(d, DefinitionDescriptor::Stat(_)))
            .unwrap()
        else {
            panic!()
        };
        let SchemaState::Known(s) = &mut e.schema else {
            panic!()
        };
        s.targets = vec![RuleEntityKind::Action];
    });
    f.rejects("stat target/context mismatch");
    let mut f = fixture();
    f.stat_type(ComputedValueType::Boolean);
    f.rejects("effect value type/unit mismatch");
    let mut f = fixture();
    f.stat_type(ComputedValueType::Quantity {
        unit: id("count-a"),
    });
    f.constant(ParameterValue::Quantity(
        FiniteQuantity::new(5.0, id("count-b")).unwrap(),
    ));
    f.rejects("effect value type/unit mismatch"); // Same dimension is not same unit.
    for foreign in [false, true] {
        let mut f = fixture();
        let RuleEffectKind::ProjectActorStat { stat, .. } = &mut f.program().effects[0].effect
        else {
            panic!()
        };
        *stat = if foreign {
            DefId::parse(
                GameVersionNamespace::new("foreign", "v1").unwrap(),
                "child-level",
            )
            .unwrap()
        } else {
            id("missing")
        };
        f.rejects("missing, unmapped");
    }
}

#[test]
fn all_computed_kinds_project_and_option_stat_does_not_invent_membership() {
    for (ty, value) in [
        (ComputedValueType::Boolean, ParameterValue::Boolean(false)),
        (ComputedValueType::Integer, integer(49)),
        (
            ComputedValueType::Quantity {
                unit: id("count-a"),
            },
            ParameterValue::Quantity(FiniteQuantity::new(-0.5, id("count-a")).unwrap()),
        ),
        (
            ComputedValueType::Option,
            ParameterValue::Option(id("option-a")),
        ),
        (
            ComputedValueType::Option,
            ParameterValue::Option(id("option-b")),
        ),
    ] {
        let mut f = fixture();
        f.stat_type(ty);
        f.constant(value.clone());
        // Other-owner program is unrelated and removed because its original
        // integer effect intentionally does not fit the changed target stat.
        f.rules.owners.truncate(1);
        let c = f.compile();
        let result = evaluate(&f, &c, "summoner-a", &[], &mut c.new_scratch());
        assert_eq!(
            result.effects[0].disposition,
            EffectDisposition::Applied { value }
        );
    }
    let mut f = fixture();
    f.stat_type(ComputedValueType::Option);
    f.constant(ParameterValue::Option(id("missing-option")));
    f.rejects("missing, unmapped");
}

#[test]
fn projected_values_do_not_expand_parameter_read_access() {
    let mut f = fixture();
    f.program().reads[0].source = RuleReadSource::Parameter {
        slot: parameter("summoner-a"),
    };
    let c = f.compile();
    let result = evaluate(&f, &c, "summoner-a", &[fact(15)], &mut c.new_scratch());
    assert_eq!(
        result.effects[0].disposition,
        EffectDisposition::Applied { value: integer(15) }
    );
    f.program().reads[0].source = RuleReadSource::Parameter {
        slot: parameter("summoner-b"),
    };
    // Even unused reads cannot access another owner's authored parameter.
    f.program().nodes[0].expression = RuleExpression::Literal { value: integer(15) };
    f.rejects("parameter declaration does not equal owner");
}

#[test]
fn known_projection_preserves_partial_owner_coverage() {
    let mut f = fixture();
    f.rules.owners[0].programs.closure = SchemaClosure::Partial {
        gaps: vec![gap(subject("summoner-a"), SchemaFacet::GameRules)],
    };
    f.schema(|input| {
        let DefinitionDescriptor::Gem(e) = input
            .definitions
            .iter_mut()
            .find(|d| d.address() == id::<GemDefinition>("summoner-a").address())
            .unwrap()
        else {
            panic!()
        };
        let SchemaState::Known(gem) = &mut e.schema else {
            panic!()
        };
        gem.declarations.actors.closure = SchemaClosure::Partial {
            gaps: vec![gap(subject("summoner-a"), SchemaFacet::StaticLinks)],
        };
    });
    let c = f.compile();
    let result = evaluate(&f, &c, "summoner-a", &[fact(11)], &mut c.new_scratch());
    assert!(matches!(
        result.owner_programs_closure,
        SchemaClosure::Partial { .. }
    ));
    assert_eq!(
        result.effects[0].disposition,
        EffectDisposition::Applied { value: integer(11) }
    );
}

#[test]
fn storage_and_compile_count_projection_edges_and_gate_operation_version() {
    let mut f = fixture();
    f.rules.owners.truncate(1);
    let limits = RuleStorageLimits::default();
    let package = OwnedRulePackage::new(f.rules.clone(), &f.schema, limits).unwrap();
    assert_eq!(package.resources().edges, 2); // Read edge plus projection value.
    let bytes = encode_rule_package(&package, limits).unwrap();
    let decoded = decode_rule_package(&bytes, &f.schema, limits).unwrap();
    assert_eq!(decoded.input(), &f.rules);
    let tight = RuleStorageLimits {
        max_edges: 1,
        ..limits
    };
    assert!(OwnedRulePackage::new(f.rules.clone(), &f.schema, tight).is_err());
    assert!(decode_rule_package(&bytes, &f.schema, tight).is_err());
    assert!(encode_rule_package(&package, tight).is_err());
    assert!(
        CompiledRulePackage::compile(
            &f.rules,
            &f.schema,
            RuleLimits {
                max_edges: 1,
                ..RuleLimits::default()
            }
        )
        .is_err()
    );
    for version in ["owned-domain-operations-v1", "owned-domain-operations-v2"] {
        f.rules.operations_version = key(version);
        f.rejects("unsupported operation version");
    }
    f.rules.operations_version = key(OWNED_RULE_OPERATIONS_VERSION);
    let RuleEffectKind::ProjectActorStat { value, .. } = &mut f.program().effects[0].effect else {
        panic!()
    };
    *value = key("missing");
    assert!(OwnedRulePackage::new(f.rules.clone(), &f.schema, limits).is_err());
    f.rejects("unknown effect node");
}

#[test]
fn projection_wire_requires_exact_typed_fields_and_rejects_duplicates() {
    let effect = fixture().program().effects[0].effect.clone();
    let raw = serde_json::to_string(&effect).unwrap();
    assert_eq!(
        serde_json::from_str::<RuleEffectKind>(&raw).unwrap(),
        effect
    );
    for field in ["actor", "stat", "value"] {
        let mut value = serde_json::to_value(&effect).unwrap();
        value.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<RuleEffectKind>(value).is_err());
        let field_value = &serde_json::to_value(&effect).unwrap()[field];
        let duplicate = format!("{{\"{field}\":{field_value},{}", &raw[1..]);
        assert!(serde_json::from_str::<RuleEffectKind>(&duplicate).is_err());
    }
    let mut value = serde_json::to_value(&effect).unwrap();
    value["activate"] = true.into();
    assert!(serde_json::from_value::<RuleEffectKind>(value).is_err());
}
