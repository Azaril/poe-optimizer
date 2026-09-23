//! Canonical numeric recipes execute natively; source admission and item cache
//! history are intentionally outside these component tests.
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_routing::{ActionRoutingInput, OWNED_ACTION_ROUTING_VERSION},
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::{
    OWNED_SCHEMA_PACKAGE_VERSION, OwnedDefinitionSchemaPackage, SchemaPackageInput,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact};
use poe_optimizer_import::{owned_mapping::*, owned_modifier_value_recipe::*, owned_recipe::*};

fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn quantity(value: f64, unit: &UnitDefId) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap())
}
fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn slots(parameters: Vec<DeclaredSlot<ParameterSlotDefId>>) -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(parameters),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
struct Fixture {
    recipe: OwnedRecipeInput,
    policy: ModifierValuePolicy,
    negative: DeclaredSlot<ParameterSlotDefId>,
    other_input: DeclaredSlot<ParameterSlotDefId>,
    second_input: DeclaredSlot<ParameterSlotDefId>,
    second_output: StatDefId,
    prior_output: StatDefId,
    other_factor: UnitDefId,
}
impl Fixture {
    fn new() -> Self {
        let namespace = GameVersionNamespace::new("test", "modifier-values").unwrap();
        let mut registry = OwnedIdRegistry::empty(namespace.clone(), Default::default()).unwrap();
        let unit: UnitDefId = registry.allocate_definition().unwrap();
        let factor: UnitDefId = registry.allocate_definition().unwrap();
        let other_factor: UnitDefId = registry.allocate_definition().unwrap();
        let modifier: ModifierDefId = registry.allocate_definition().unwrap();
        let other: ModifierDefId = registry.allocate_definition().unwrap();
        let input = registry
            .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(modifier.clone()))
            .unwrap();
        let negative = registry
            .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(modifier.clone()))
            .unwrap();
        let second_input = registry
            .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(modifier.clone()))
            .unwrap();
        let other_input = registry
            .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(other.clone()))
            .unwrap();
        let output: StatDefId = registry.allocate_definition().unwrap();
        let second_output: StatDefId = registry.allocate_definition().unwrap();
        let prior_output: StatDefId = registry.allocate_definition().unwrap();
        let base: StatDefId = registry.allocate_definition().unwrap();
        let magnitude: StatDefId = registry.allocate_definition().unwrap();
        let range = ValueSchema::Quantity(QuantityRange {
            minimum: FiniteQuantity::new(-1e16, unit.clone()).unwrap(),
            maximum: FiniteQuantity::new(1e16, unit.clone()).unwrap(),
        });
        let mut definitions = vec![
            DefinitionDescriptor::Unit(entry(
                unit.clone(),
                UnitSchema {
                    dimension: UnitDimension::Damage,
                },
            )),
            DefinitionDescriptor::Unit(entry(
                factor.clone(),
                UnitSchema {
                    dimension: UnitDimension::DimensionlessFactor,
                },
            )),
            DefinitionDescriptor::Unit(entry(
                other_factor.clone(),
                UnitSchema {
                    dimension: UnitDimension::DimensionlessFactor,
                },
            )),
            DefinitionDescriptor::Modifier(entry(
                modifier.clone(),
                ModifierSchema {
                    declarations: slots(vec![
                        input.clone(),
                        negative.clone(),
                        second_input.clone(),
                    ]),
                },
            )),
            DefinitionDescriptor::Modifier(entry(
                other,
                ModifierSchema {
                    declarations: slots(vec![other_input.clone()]),
                },
            )),
        ];
        for (stat, unit) in [
            (&output, &unit),
            (&second_output, &unit),
            (&prior_output, &unit),
            (&base, &factor),
            (&magnitude, &factor),
        ] {
            definitions.push(DefinitionDescriptor::Stat(entry(
                stat.clone(),
                StatSchema {
                    value: ComputedValueType::Quantity { unit: unit.clone() },
                    targets: vec![RuleEntityKind::Modifier],
                },
            )));
        }
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
                namespace: namespace.clone(),
                release: key("test-release"),
                semantics_version: key("test-semantics"),
                definitions,
                slots: vec![
                    SlotDescriptor::Parameter(entry(
                        input.clone(),
                        ParameterSlotSchema {
                            value: range.clone(),
                            presence: SlotPresence::RequiredOnce,
                            sites: vec![ParameterSite::ModifierRoll],
                        },
                    )),
                    SlotDescriptor::Parameter(entry(
                        negative.clone(),
                        ParameterSlotSchema {
                            value: ValueSchema::Boolean,
                            presence: SlotPresence::RequiredOnce,
                            sites: vec![ParameterSite::ModifierRoll],
                        },
                    )),
                    SlotDescriptor::Parameter(entry(
                        second_input.clone(),
                        ParameterSlotSchema {
                            value: range.clone(),
                            presence: SlotPresence::RequiredOnce,
                            sites: vec![ParameterSite::ModifierRoll],
                        },
                    )),
                    SlotDescriptor::Parameter(entry(
                        other_input.clone(),
                        ParameterSlotSchema {
                            value: range,
                            presence: SlotPresence::RequiredOnce,
                            sites: vec![ParameterSite::ModifierRoll],
                        },
                    )),
                ],
            },
            Default::default(),
        )
        .unwrap();
        let owner = SchemaSubject::Definition(modifier.address());
        let prior = RuleProgram {
            id: key("prior"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![],
            nodes: vec![RuleNode {
                id: key("value"),
                expression: RuleExpression::Literal {
                    value: quantity(7.0, &unit),
                },
            }],
            effects: vec![RuleEffect {
                id: key("derive"),
                when: None,
                effect: RuleEffectKind::Derive {
                    entity: RuleEntity::Modifier,
                    stat: prior_output.clone(),
                    value: key("value"),
                },
            }],
        };
        let recipe = OwnedRecipeInput {
            schema_version: OWNED_RECIPE_VERSION,
            registry: registry.input().clone(),
            schema: schema.input().clone(),
            rules: RulePackageInput {
                schema_version: OWNED_RULE_PACKAGE_VERSION,
                namespace: namespace.clone(),
                release: key("test-release"),
                semantics_version: key("test-semantics"),
                operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
                definitions: schema.identity().clone(),
                tables: vec![],
                owners: vec![DefinitionRules {
                    owner: owner.clone(),
                    programs: DeclaredSet::partial(
                        vec![prior],
                        vec![SchemaGap {
                            subject: owner,
                            facet: SchemaFacet::GameRules,
                            code: key("encoding-and-item-coverage-unproved"),
                        }],
                    ),
                }],
                receivers: DeclaredSet::complete(vec![]),
            },
            routing: ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace,
                release: key("test-release"),
                definitions: schema.identity().clone(),
                outputs: vec![],
            },
        };
        let policy = ModifierValuePolicy {
            schema_version: 1,
            version: key("canonical-components"),
            definitions: schema.identity().clone(),
            factor_unit: factor,
            bindings: vec![ModifierValueBinding {
                modifier,
                program: key("effective"),
                input,
                unit,
                output,
                precision: 1,
                display_precision: 0,
                sign: ModifierValueSign::Direct,
                scaling: ModifierValueScaling::Scaled {
                    corrupted_base: base,
                    magnitude,
                },
            }],
        };
        Self {
            recipe,
            policy,
            negative,
            other_input,
            second_input,
            second_output,
            prior_output,
            other_factor,
        }
    }
    fn rebind(&mut self) {
        let schema =
            OwnedDefinitionSchemaPackage::new(self.recipe.schema.clone(), Default::default())
                .unwrap();
        self.recipe.rules.definitions = schema.identity().clone();
        self.recipe.routing.definitions = schema.identity().clone();
        self.policy.definitions = schema.identity().clone();
    }
    fn base(&self) -> StagedOwnedRecipe {
        assemble_owned_recipe(self.recipe.clone(), Default::default()).unwrap()
    }
    fn compile(&self) -> Result<StagedModifierValueRecipe, ModifierValueRecipeError> {
        compile_owned_modifier_values(&self.base(), &self.policy, Default::default())
    }
    fn qualifier(&mut self) {
        self.policy.bindings[0].sign = ModifierValueSign::Qualifier {
            negative: self.negative.clone(),
        };
        let unit = self.policy.bindings[0].unit.clone();
        let parameter = self.input_schema();
        let ValueSchema::Quantity(range) = &mut parameter.value else {
            panic!()
        };
        range.minimum = FiniteQuantity::new(0.0, unit).unwrap();
        self.rebind();
    }
    fn input_schema(&mut self) -> &mut ParameterSlotSchema {
        let input = self.policy.bindings[0].input.clone();
        self.recipe
            .schema
            .slots
            .iter_mut()
            .find_map(|slot| {
                if let SlotDescriptor::Parameter(row) = slot
                    && row.id == input
                    && let SchemaState::Known(schema) = &mut row.schema
                {
                    Some(schema)
                } else {
                    None
                }
            })
            .unwrap()
    }
    fn facts(&self, value: f64, base: f64, magnitude: f64, negative: bool) -> Vec<RuleFact> {
        let mut facts = vec![RuleFact {
            read: key("component"),
            value: quantity(value, &self.policy.bindings[0].unit),
        }];
        if matches!(
            self.policy.bindings[0].scaling,
            ModifierValueScaling::Scaled { .. }
        ) {
            facts.push(RuleFact {
                read: key("corruption-factor"),
                value: quantity(base, &self.policy.factor_unit),
            });
            facts.push(RuleFact {
                read: key("magnitude-factor"),
                value: quantity(magnitude, &self.policy.factor_unit),
            });
        }
        if matches!(
            self.policy.bindings[0].sign,
            ModifierValueSign::Qualifier { .. }
        ) {
            facts.push(RuleFact {
                read: key("negative"),
                value: ParameterValue::Boolean(negative),
            });
        }
        facts
    }
}
fn evaluated(
    out: &OwnedRecipeInput,
    binding: &ModifierValueBinding,
    facts: &[RuleFact],
) -> EffectDisposition {
    let base = assemble_owned_recipe(out.clone(), Default::default()).unwrap();
    let compiled =
        CompiledRulePackage::compile(base.rules().input(), base.schema(), Default::default())
            .unwrap();
    let owner = SchemaSubject::Definition(binding.modifier.address());
    let mut scratch = compiled.new_scratch();
    let result = compiled
        .evaluate(&owner, &binding.program, facts, base.schema(), &mut scratch)
        .unwrap();
    let missing = compiled
        .evaluate(&owner, &binding.program, &[], base.schema(), &mut scratch)
        .unwrap();
    assert!(matches!(
        missing.effects[0].disposition,
        EffectDisposition::Unresolved { .. }
    ));
    assert_eq!(
        result,
        compiled
            .evaluate(&owner, &binding.program, facts, base.schema(), &mut scratch)
            .unwrap()
    );
    assert_ne!(result.owner_programs_closure, SchemaClosure::Complete);
    assert_eq!(result.effects.len(), 1);
    assert!(
        matches!(&result.effects[0].effect, RuleEffectKind::Derive { entity: RuleEntity::Modifier, stat, .. } if stat == &binding.output)
    );
    result.effects[0].disposition.clone()
}
fn number(result: EffectDisposition) -> f64 {
    let EffectDisposition::Applied {
        value: ParameterValue::Quantity(v),
    } = result
    else {
        panic!("expected numeric output: {result:?}")
    };
    v.value()
}
#[test]
fn precision_multiplication_half_offsets_and_final_display_rounding_are_literal() {
    let mut f = Fixture::new();
    f.policy.bindings[0].scaling = ModifierValueScaling::Unscaled;
    for (precision, display_precision, raw, expected) in [
        (100, 2, 2.005, 2.01),
        (100, 2, 2.465, 2.47),
        (100, 2, -2.005, -2.01),
        (1, 0, f64::from_bits(0.5_f64.to_bits() - 1), 1.0),
        (1, 0, 4_503_599_627_370_497.0, 4_503_599_627_370_498.0),
        (6, 0, 2.5, 3.0),
        (3, 2, 4.3, 4.33),
        (100, 1, 2.45, 2.5),
    ] {
        f.policy.bindings[0].precision = precision;
        f.policy.bindings[0].display_precision = display_precision;
        let out = f.compile().unwrap();
        let result = number(evaluated(
            &out.successor,
            &f.policy.bindings[0],
            &f.facts(raw, 1.0, 1.0, false),
        ));
        assert_eq!(
            result, expected,
            "p={precision} display={display_precision} raw={raw}"
        );
    }
    assert_eq!(
        (2.005_f64 / 0.01 + 0.5).floor() * 0.01,
        2.0,
        "reciprocal quantum would change the first result"
    );
}
#[test]
fn corruption_precedes_magnitude_and_direct_signed_values_differ_from_reduced_qualifiers() {
    let mut f = Fixture::new();
    for (qualifier, value, base, magnitude, negative, expected) in [
        (false, -3.0, 1.0, 1.0, false, -3.0),
        (false, -3.0, 1.5, 1.0, false, -4.0),
        (false, -3.0, 1.5, 1.2, false, -4.0),
        (false, 25.5, 1.5, 1.2, false, 46.0),
        (false, 1000.0, 1.0, (100.0 + 0.7) / 100.0, false, 1007.0),
        (false, 25.0, 1.0, 2.4, false, 60.0),
        (false, 25.0, 1.0, 2.2, false, 55.0),
        (true, 3.0, 1.5, 1.0, true, -5.0),
        (true, 3.0, 1.5, 1.2, true, -6.0),
        (true, 0.0, 1.0, 1.0, false, 0.0),
    ] {
        if qualifier {
            f.qualifier();
        }
        let out = f.compile().unwrap();
        assert_eq!(
            number(evaluated(
                &out.successor,
                &f.policy.bindings[0],
                &f.facts(value, base, magnitude, negative)
            )),
            expected
        );
    }
    f.policy.bindings[0].precision = 100;
    f.policy.bindings[0].display_precision = 2;
    let out = f.compile().unwrap();
    assert_eq!(
        number(evaluated(
            &out.successor,
            &f.policy.bindings[0],
            &f.facts(2.43, 1.5, 1.2, false)
        )),
        4.38
    );
}
#[test]
fn scaled_inputs_never_default_and_component_non_scalability_does_not_demand_factors() {
    let mut f = Fixture::new();
    f.qualifier();
    let out = f.compile().unwrap();
    let facts = f.facts(3.0, 1.0, 1.0, true);
    for missing in [
        "component",
        "negative",
        "corruption-factor",
        "magnitude-factor",
    ] {
        let partial: Vec<_> = facts
            .iter()
            .filter(|fact| fact.read.as_str() != missing)
            .cloned()
            .collect();
        assert!(
            matches!(
                evaluated(&out.successor, &f.policy.bindings[0], &partial),
                EffectDisposition::Unresolved { .. }
            ),
            "{missing}"
        );
    }
    f.policy.bindings[0].scaling = ModifierValueScaling::Unscaled;
    let out = f.compile().unwrap();
    assert_eq!(out.extension.owners[0].programs.members[0].reads.len(), 2);
    assert_eq!(
        number(evaluated(
            &out.successor,
            &f.policy.bindings[0],
            &f.facts(3.0, 50.0, 50.0, true)
        )),
        -3.0
    );
}
#[test]
fn appends_are_idempotent_preserve_old_bytes_and_keep_independent_component_outputs() {
    let mut f = Fixture::new();
    let mut second = f.policy.bindings[0].clone();
    second.program = key("effective-maximum");
    second.output = f.second_output.clone();
    second.input = f.second_input.clone();
    second.precision = 100;
    second.display_precision = 2;
    f.policy.bindings.push(second);
    let before = f.base();
    let out = f.compile().unwrap();
    assert_eq!(out.receipt.bindings, 2);
    assert_eq!(out.receipt.owners, 1);
    assert_eq!(out.receipt.extension.appended_programs, 2);
    assert_eq!(out.successor.registry, *before.registry().input());
    assert_eq!(out.successor.schema, *before.schema().input());
    assert_eq!(out.successor.routing, *before.routing().input());
    assert_eq!(
        out.successor.rules.operations_version,
        before.rules().input().operations_version
    );
    let prior = &before.rules().input().owners[0];
    let successor = &out.successor.rules.owners[0];
    assert_eq!(successor.programs.closure, prior.programs.closure);
    assert_eq!(
        serde_json::to_vec(
            successor
                .programs
                .members
                .iter()
                .find(|p| p.id == key("prior"))
                .unwrap()
        )
        .unwrap(),
        serde_json::to_vec(&prior.programs.members[0]).unwrap()
    );
    for (binding, value, expected) in [
        (&f.policy.bindings[0], 11.0, 13.0),
        (&f.policy.bindings[1], 19.125, 22.95),
    ] {
        let facts = f.facts(value, 1.0, 1.2, false);
        assert_eq!(number(evaluated(&out.successor, binding, &facts)), expected);
    }
    let successor = assemble_owned_recipe(out.successor.clone(), Default::default()).unwrap();
    let replay = compile_owned_modifier_values(&successor, &f.policy, Default::default()).unwrap();
    assert_eq!(replay.successor, out.successor);
    assert_eq!(replay.receipt.extension.appended_programs, 0);
    f.policy.bindings.reverse();
    assert_eq!(f.compile().unwrap().successor, out.successor);
    // Existing operation contracts require no upgrade for these arithmetic rules.
    f.recipe.rules.operations_version = key(OWNED_RULE_OPERATIONS_V6);
    assert_eq!(
        f.compile()
            .unwrap()
            .successor
            .rules
            .operations_version
            .as_str(),
        OWNED_RULE_OPERATIONS_V6
    );
}
#[test]
fn schema_ownership_units_presence_and_qualifier_domain_are_validated() {
    for case in 0..10 {
        let mut f = Fixture::new();
        match case {
            0 => f.policy.factor_unit = f.policy.bindings[0].unit.clone(),
            1 => f.policy.bindings[0].unit = f.other_factor.clone(),
            2 => f.policy.bindings[0].input = f.other_input.clone(),
            3 => f.input_schema().presence = SlotPresence::OptionalOnce,
            4 => {
                f.policy.bindings[0].sign = ModifierValueSign::Qualifier {
                    negative: f.negative.clone(),
                }
            }
            5 => {
                f.qualifier();
                f.policy.bindings[0].sign = ModifierValueSign::Qualifier {
                    negative: f.policy.bindings[0].input.clone(),
                }
            }
            6 => f.policy.factor_unit = f.other_factor.clone(),
            7 => {
                f.policy.bindings[0].output =
                    StatDefId::parse(f.recipe.schema.namespace.clone(), "absent").unwrap()
            }
            8 => {
                let target = f.policy.bindings[0].output.clone();
                for d in &mut f.recipe.schema.definitions {
                    if let DefinitionDescriptor::Stat(d) = d
                        && d.id == target
                        && let SchemaState::Known(s) = &mut d.schema
                    {
                        s.targets = vec![RuleEntityKind::Actor];
                    }
                }
            }
            9 => f.recipe.rules.owners.clear(),
            _ => unreachable!(),
        }
        f.rebind();
        assert!(f.compile().is_err(), "case {case}");
    }
    let mut f = Fixture::new();
    let stale = f.policy.definitions.clone();
    f.recipe.schema.release = key("changed-schema");
    f.rebind();
    f.policy.definitions = stale;
    assert!(matches!(
        f.compile(),
        Err(ModifierValueRecipeError::Binding)
    ));
}
#[test]
fn collisions_and_upstream_output_aliases_reject_without_replacing_prior_programs() {
    let mut f = Fixture::new();
    f.policy.bindings[0].output = f.prior_output.clone();
    assert!(matches!(
        f.compile(),
        Err(ModifierValueRecipeError::Invalid(
            "existing modifier output writer conflicts"
        ))
    ));
    let mut f = Fixture::new();
    f.policy.bindings[0].program = key("prior");
    assert!(matches!(
        f.compile(),
        Err(ModifierValueRecipeError::Invalid(
            "existing program differs"
        ))
    ));
    let mut f = Fixture::new();
    f.policy.bindings.push(f.policy.bindings[0].clone());
    assert!(f.compile().is_err());
    f.policy.bindings[1].output = f.second_output.clone();
    assert!(matches!(
        f.compile(),
        Err(ModifierValueRecipeError::Invalid(
            "duplicate modifier program"
        ))
    ));
    let mut f = Fixture::new();
    f.recipe.rules.owners[0].programs.closure = SchemaClosure::Complete;
    assert!(f.compile().is_err());
    // A component quantity may be dimensionless, but its output may not feed
    // its own upstream factor stage (nor another requested stage on this owner).
    let mut f = Fixture::new();
    let factor = f.policy.factor_unit.clone();
    let input = f.input_schema();
    input.value = ValueSchema::Quantity(QuantityRange {
        minimum: FiniteQuantity::new(0.0, factor.clone()).unwrap(),
        maximum: FiniteQuantity::new(100.0, factor.clone()).unwrap(),
    });
    f.policy.bindings[0].unit = factor;
    let ModifierValueScaling::Scaled { corrupted_base, .. } = &f.policy.bindings[0].scaling else {
        panic!()
    };
    f.policy.bindings[0].output = corrupted_base.clone();
    f.rebind();
    assert!(matches!(
        f.compile(),
        Err(ModifierValueRecipeError::Invalid(
            "effective outputs cannot supply their upstream factors"
        ))
    ));
}
#[test]
fn finite_policy_work_node_and_precision_limits_and_strict_wire_contract_reject() {
    let f = Fixture::new();
    let base = f.base();
    let used = f.compile().unwrap().receipt;
    for limits in [
        ModifierValueRecipeLimits {
            max_policy_bytes: 1,
            ..Default::default()
        },
        ModifierValueRecipeLimits {
            max_nodes: used.nodes - 1,
            ..Default::default()
        },
        ModifierValueRecipeLimits {
            max_work: used.work_used - 1,
            ..Default::default()
        },
        ModifierValueRecipeLimits {
            max_bindings: 0,
            ..Default::default()
        },
        ModifierValueRecipeLimits {
            max_nodes: ModifierValueRecipeLimits::default().max_nodes + 1,
            ..Default::default()
        },
    ] {
        assert!(compile_owned_modifier_values(&base, &f.policy, limits).is_err());
    }
    let limits = ModifierValueRecipeLimits {
        max_nodes: used.nodes,
        max_work: used.work_used,
        ..Default::default()
    };
    assert!(compile_owned_modifier_values(&base, &f.policy, limits).is_ok());
    for (precision, display) in [(0, 0), (1_000_001, 0), (1, 13)] {
        let mut policy = f.policy.clone();
        policy.bindings[0].precision = precision;
        policy.bindings[0].display_precision = display;
        assert!(matches!(
            compile_owned_modifier_values(&base, &policy, Default::default()),
            Err(ModifierValueRecipeError::Invalid(
                "precision outside supported bounds"
            ))
        ));
    }
    let mut wire = serde_json::to_value(&f.policy).unwrap();
    wire["bindings"][0]["implicit_source_default"] = true.into();
    assert!(serde_json::from_value::<ModifierValuePolicy>(wire).is_err());
    let bytes = serde_json::to_vec(&f.policy).unwrap();
    assert_eq!(
        serde_json::from_slice::<ModifierValuePolicy>(&bytes).unwrap(),
        f.policy
    );
}

mod occurrence_plan {
    use super::*;
    use poe_optimizer_core::{build_identity::*, owned_build::*};
    use poe_optimizer_data::owned_routing::OwnedActionRouting;
    use poe_optimizer_engine::owned_plan::*;
    use std::sync::Arc;

    fn occurrence<T: BuildInstanceId>(id: u64) -> T {
        T::from_instance_id(InstanceId::from_parts(BuildLineage::from_bytes([82; 16]), id).unwrap())
    }
    fn level_range() -> IntegerRange {
        IntegerRange {
            minimum: BoundedInteger::new(1).unwrap(),
            maximum: BoundedInteger::new(100).unwrap(),
        }
    }
    fn scalar_program(
        name: &str,
        slot: DeclaredSlot<ParameterSlotDefId>,
        stat: StatDefId,
        unit: &UnitDefId,
    ) -> RuleProgram {
        RuleProgram {
            id: key(name),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![RuleRead {
                id: key("input"),
                value_type: ComputedValueType::Quantity { unit: unit.clone() },
                source: RuleReadSource::Parameter { slot },
            }],
            nodes: vec![RuleNode {
                id: key("value"),
                expression: RuleExpression::Read {
                    input: key("input"),
                },
            }],
            effects: vec![RuleEffect {
                id: key("value"),
                when: None,
                effect: RuleEffectKind::Derive {
                    entity: RuleEntity::Modifier,
                    stat,
                    value: key("value"),
                },
            }],
        }
    }
    struct OccurrenceFixture {
        recipe: OwnedRecipeInput,
        build: BuildInput,
        scenario: ScenarioInput,
        binding: ModifierValueBinding,
        base: StatDefId,
        magnitude: StatDefId,
        class: ClassDefId,
    }
    impl OccurrenceFixture {
        fn new() -> Self {
            let mut f = Fixture::new();
            f.policy.bindings[0].precision = 100;
            f.policy.bindings[0].display_precision = 2;
            let binding = f.policy.bindings[0].clone();
            let ModifierValueScaling::Scaled {
                corrupted_base: base,
                magnitude,
            } = binding.scaling.clone()
            else {
                unreachable!()
            };
            let factor = f.policy.factor_unit.clone();
            let mut registry =
                OwnedIdRegistry::new(f.recipe.registry.clone(), Default::default()).unwrap();
            let class: ClassDefId = registry.allocate_definition().unwrap();
            let encounter: EncounterDefId = registry.allocate_definition().unwrap();
            let template: ItemTemplateDefId = registry.allocate_definition().unwrap();
            let destinations: Vec<EquipmentSlotDefId> = (0..3)
                .map(|_| registry.allocate_definition().unwrap())
                .collect();
            let initial: StatDefId = registry.allocate_definition().unwrap();
            let channel: StatDefId = registry.allocate_definition().unwrap();
            let base_roll = registry
                .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(
                    binding.modifier.clone(),
                ))
                .unwrap();
            let initial_roll = registry
                .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Modifier(
                    binding.modifier.clone(),
                ))
                .unwrap();
            f.recipe.schema.definitions.extend([
                DefinitionDescriptor::Class(entry(
                    class.clone(),
                    ClassSchema {
                        level: level_range(),
                        ascendancies: DeclaredSet::complete(vec![]),
                        implicit_passives: DeclaredSet::complete(vec![]),
                        declarations: slots(vec![]),
                    },
                )),
                DefinitionDescriptor::Encounter(entry(
                    encounter.clone(),
                    EncounterSchema {
                        enemy_level: level_range(),
                        external_inputs: DeclaredSet::complete(vec![]),
                    },
                )),
                DefinitionDescriptor::ItemTemplate(entry(
                    template.clone(),
                    ItemTemplateSchema {
                        item_level: level_range(),
                        equipment_slots: DeclaredSet::complete(destinations.clone()),
                        socket_destinations: DeclaredSet::complete(vec![]),
                        modifiers: DeclaredSet::complete(vec![binding.modifier.clone()]),
                        quality: QualityUseSchema {
                            presence: QualityPresence::Forbidden,
                            allowed_kinds: DeclaredSet::complete(vec![]),
                        },
                        declarations: slots(vec![]),
                    },
                )),
            ]);
            for destination in &destinations {
                f.recipe
                    .schema
                    .definitions
                    .push(DefinitionDescriptor::EquipmentSlot(entry(
                        destination.clone(),
                        EquipmentSlotSchema {
                            scope: ScopePolicy::Either,
                        },
                    )));
            }
            for stat in [&initial, &channel] {
                f.recipe
                    .schema
                    .definitions
                    .push(DefinitionDescriptor::Stat(entry(
                        stat.clone(),
                        StatSchema {
                            value: ComputedValueType::Quantity {
                                unit: factor.clone(),
                            },
                            targets: vec![RuleEntityKind::Modifier],
                        },
                    )));
            }
            for descriptor in &mut f.recipe.schema.definitions {
                if let DefinitionDescriptor::Modifier(row) = descriptor
                    && row.id == binding.modifier
                    && let SchemaState::Known(schema) = &mut row.schema
                {
                    schema
                        .declarations
                        .parameters
                        .members
                        .extend([base_roll.clone(), initial_roll.clone()]);
                }
            }
            for slot in [&base_roll, &initial_roll] {
                f.recipe.schema.slots.push(SlotDescriptor::Parameter(entry(
                    slot.clone(),
                    ParameterSlotSchema {
                        value: ValueSchema::Quantity(QuantityRange {
                            minimum: FiniteQuantity::new(0.0, factor.clone()).unwrap(),
                            maximum: FiniteQuantity::new(10.0, factor.clone()).unwrap(),
                        }),
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![ParameterSite::ModifierRoll],
                    },
                )));
            }
            f.recipe.rules.owners[0].programs.members.extend([
                scalar_program("base-stage", base_roll.clone(), base.clone(), &factor),
                scalar_program(
                    "initial-stage",
                    initial_roll.clone(),
                    initial.clone(),
                    &factor,
                ),
                RuleProgram {
                    id: key("ordered-stage"),
                    context: RuleEntityKind::EquipmentUse,
                    reads: vec![RuleRead {
                        id: key("factor"),
                        value_type: ComputedValueType::Quantity {
                            unit: factor.clone(),
                        },
                        source: RuleReadSource::ModifierTransforms {
                            stat: channel,
                            initial,
                        },
                    }],
                    nodes: vec![RuleNode {
                        id: key("factor"),
                        expression: RuleExpression::Read {
                            input: key("factor"),
                        },
                    }],
                    effects: vec![RuleEffect {
                        id: key("factor"),
                        when: None,
                        effect: RuleEffectKind::Derive {
                            entity: RuleEntity::Modifier,
                            stat: magnitude.clone(),
                            value: key("factor"),
                        },
                    }],
                },
            ]);
            for subject in [class.address(), encounter.address(), template.address()] {
                f.recipe.rules.owners.push(DefinitionRules {
                    owner: SchemaSubject::Definition(subject),
                    programs: DeclaredSet::complete(vec![]),
                });
            }
            f.recipe.registry = registry.input().clone();
            f.rebind();
            let out = f.compile().unwrap();
            let modifier_owner = out
                .successor
                .rules
                .owners
                .iter()
                .find(|o| o.owner == SchemaSubject::Definition(binding.modifier.address()))
                .unwrap();
            assert!(
                matches!(
                    modifier_owner.programs.closure,
                    SchemaClosure::Partial { .. }
                ),
                "compilation must not promote fixture coverage"
            );
            let rolled = |id, value, base, initial| RolledModifier {
                id: occurrence(id),
                definition: binding.modifier.clone(),
                rolls: vec![
                    ParameterAssignment {
                        slot: binding.input.clone(),
                        value: quantity(value, &binding.unit),
                    },
                    ParameterAssignment {
                        slot: f.negative.clone(),
                        value: ParameterValue::Boolean(false),
                    },
                    ParameterAssignment {
                        slot: f.second_input.clone(),
                        value: quantity(0.0, &binding.unit),
                    },
                    ParameterAssignment {
                        slot: base_roll.clone(),
                        value: quantity(base, &factor),
                    },
                    ParameterAssignment {
                        slot: initial_roll.clone(),
                        value: quantity(initial, &factor),
                    },
                ],
            };
            let namespace = f.recipe.schema.namespace.clone();
            let build = BuildInput {
                allocator: InstanceAllocatorState::from_parts(
                    BuildLineage::from_bytes([82; 16]),
                    100,
                ),
                revision: BuildRevision::from_u64(1),
                game_version: namespace.clone(),
                character: CharacterSpec {
                    class: class.clone(),
                    ascendancy: None,
                    level: 20,
                    rewards: vec![],
                },
                weapon_loadouts: vec![occurrence(1), occurrence(2)],
                active_weapon_loadout: occurrence(1),
                items: vec![
                    ItemRecord {
                        id: occurrence(3),
                        template: template.clone(),
                        parameters: vec![],
                        item_level: Some(20),
                        quality: None,
                        modifiers: vec![rolled(4, 2.43, 1.5, 1.2), rolled(5, 3.5, 1.0, 1.6)],
                        modifier_order: vec![occurrence(4), occurrence(5)],
                    },
                    ItemRecord {
                        id: occurrence(30),
                        template,
                        parameters: vec![],
                        item_level: Some(20),
                        quality: None,
                        modifiers: vec![rolled(31, 7.125, 1.0, 1.2)],
                        modifier_order: vec![occurrence(31)],
                    },
                ],
                equipment: vec![
                    EquipmentUse {
                        id: occurrence(6),
                        item: occurrence(3),
                        destination: EquipmentDestination::CharacterSlot(destinations[0].clone()),
                        scope: LoadoutScope::Shared,
                    },
                    EquipmentUse {
                        id: occurrence(7),
                        item: occurrence(3),
                        destination: EquipmentDestination::CharacterSlot(destinations[1].clone()),
                        scope: LoadoutScope::Shared,
                    },
                    EquipmentUse {
                        id: occurrence(32),
                        item: occurrence(30),
                        destination: EquipmentDestination::CharacterSlot(destinations[2].clone()),
                        scope: LoadoutScope::Shared,
                    },
                ],
                gems: vec![],
                allocations: vec![],
                skills: vec![],
                supports: vec![],
                payload_links: vec![],
                choices: vec![],
            };
            let scenario = ScenarioInput {
                game_version: namespace,
                enemy: EnemySpec {
                    encounter,
                    level: 20,
                },
                assumptions: vec![],
                usage: vec![],
            };
            Self {
                recipe: out.successor,
                build,
                scenario,
                binding,
                base,
                magnitude,
                class,
            }
        }
        fn complete(&mut self) {
            // Explicit truth for this finite synthetic fixture only. Real source
            // owners retain their gaps; the compiler never performs this step.
            for owner in &mut self.recipe.rules.owners {
                owner.programs.closure = SchemaClosure::Complete;
            }
        }
        fn plan(&self) -> OwnedEffectPlan<OwnedDefinitionSchemaPackage> {
            let schema = Arc::new(
                OwnedDefinitionSchemaPackage::new(self.recipe.schema.clone(), Default::default())
                    .unwrap(),
            );
            let rules = Arc::new(
                CompiledRulePackage::compile(
                    &self.recipe.rules,
                    schema.as_ref(),
                    Default::default(),
                )
                .unwrap(),
            );
            let routing = Arc::new(
                OwnedActionRouting::new(
                    self.recipe.routing.clone(),
                    schema.as_ref(),
                    Default::default(),
                )
                .unwrap(),
            );
            let limits = OwnedInputLimits::default();
            let request = OwnedEvaluationRequest::new(
                BuildSpec::new(self.build.clone(), limits).unwrap(),
                ScenarioSpec::new(self.scenario.clone(), limits).unwrap(),
                QuerySpec::new(
                    QueryInput {
                        game_version: self.build.game_version.clone(),
                        requests: vec![],
                    },
                    limits,
                )
                .unwrap(),
                limits,
            )
            .unwrap();
            OwnedEffectPlan::compile(
                Arc::new(request),
                schema,
                rules,
                routing,
                Default::default(),
            )
            .unwrap()
        }
        fn value(
            &self,
            report: &OwnedEffectsReport,
            equipment: u64,
            modifier: u64,
            stat: &StatDefId,
        ) -> EffectValue {
            let wanted = PlanValueKey::Stat {
                entity: ConcreteEntity::Modifier(ProviderKey {
                    root: ProviderRoot::ItemModifier {
                        equipment_use: occurrence(equipment),
                        modifier: occurrence(modifier),
                    },
                    grant_path: vec![],
                }),
                stat: stat.clone(),
            };
            report
                .values
                .iter()
                .find(|v| v.key == wanted)
                .unwrap_or_else(|| panic!("missing {wanted:?}: {report:?}"))
                .value
                .clone()
        }
        fn assert_outputs(&self, report: &OwnedEffectsReport, first: f64) {
            assert!(report.gaps.is_empty(), "{:?}", report.gaps);
            for (equipment, modifier, expected) in [
                (6, 4, first),
                (7, 4, first),
                (6, 5, 5.6),
                (7, 5, 5.6),
                (32, 31, 8.55),
            ] {
                assert_eq!(
                    self.value(report, equipment, modifier, &self.binding.output),
                    EffectValue::Known {
                        value: quantity(expected, &self.binding.unit)
                    }
                );
            }
        }
    }
    #[test]
    fn canonical_recipes_bind_real_occurrences_and_isolate_item_edits_and_parallel_scratch() {
        let mut f = OccurrenceFixture::new();
        f.complete();
        let first = f.plan();
        let first_report = first.evaluate(&mut first.new_scratch()).unwrap();
        f.assert_outputs(&first_report, 4.38);
        f.build.items[0].modifiers[0]
            .rolls
            .iter_mut()
            .find(|r| r.slot == f.binding.input)
            .unwrap()
            .value = quantity(3.01, &f.binding.unit);
        f.build.revision = BuildRevision::from_u64(2);
        let second = f.plan();
        assert_ne!(first.identity(), second.identity());
        let second_report = second.evaluate(&mut second.new_scratch()).unwrap();
        f.assert_outputs(&second_report, 5.42);
        let mut scratch = first.new_scratch();
        for _ in 0..3 {
            assert_eq!(second.evaluate(&mut scratch).unwrap(), second_report);
            assert_eq!(first.evaluate(&mut scratch).unwrap(), first_report);
        }
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let (first, second, first_report, second_report) =
                    (&first, &second, &first_report, &second_report);
                scope.spawn(move || {
                    let mut scratch = first.new_scratch();
                    for _ in 0..4 {
                        assert_eq!(second.evaluate(&mut scratch).unwrap(), *second_report);
                        assert_eq!(first.evaluate(&mut scratch).unwrap(), *first_report);
                    }
                });
            }
        });
    }
    #[test]
    fn missing_scalar_stage_cannot_turn_into_unity_or_known_effective_values() {
        for missing in ["base-stage", "initial-stage", "ordered-stage"] {
            let mut f = OccurrenceFixture::new();
            f.complete();
            f.recipe
                .rules
                .owners
                .iter_mut()
                .find(|o| o.owner == SchemaSubject::Definition(f.binding.modifier.address()))
                .unwrap()
                .programs
                .members
                .retain(|p| p.id != key(missing));
            let plan = f.plan();
            let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
            for (equipment, modifier) in [(6, 4), (7, 4), (6, 5), (7, 5), (32, 31)] {
                assert!(
                    matches!(
                        f.value(&report, equipment, modifier, &f.binding.output),
                        EffectValue::Unresolved { .. }
                    ),
                    "missing {missing}: {report:?}"
                );
            }
        }
    }
    #[test]
    fn partial_owner_and_unrelated_global_gaps_block_even_empty_ordered_channels() {
        for unrelated in [false, true] {
            let mut f = OccurrenceFixture::new();
            if unrelated {
                f.complete();
                let subject = SchemaSubject::Definition(f.class.address());
                f.recipe
                    .rules
                    .owners
                    .iter_mut()
                    .find(|o| o.owner == subject)
                    .unwrap()
                    .programs
                    .closure = SchemaClosure::Partial {
                    gaps: vec![SchemaGap {
                        subject: subject.clone(),
                        facet: SchemaFacet::GameRules,
                        code: key("unreviewed-class-effects"),
                    }],
                };
            }
            let plan = f.plan();
            let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
            assert!(!report.gaps.is_empty());
            for (equipment, modifier) in [(6, 4), (7, 4), (6, 5), (7, 5), (32, 31)] {
                // Individual effects retain component evidence; the global
                // completeness gate still withholds every public final value.
                let base_key = PlanValueKey::Stat {
                    entity: ConcreteEntity::Modifier(ProviderKey {
                        root: ProviderRoot::ItemModifier {
                            equipment_use: occurrence(equipment),
                            modifier: occurrence(modifier),
                        },
                        grant_path: vec![],
                    }),
                    stat: f.base.clone(),
                };
                let base_effect = report.effects.iter().find(|effect| {
                    matches!(&effect.target, BoundEffectTarget::Value { key } if key == &base_key)
                }).unwrap();
                let expected_base = if modifier == 4 { 1.5 } else { 1.0 };
                assert!(
                    matches!(&base_effect.value, EffectValue::Known {
                    value: ParameterValue::Quantity(value),
                } if value.value() == expected_base),
                    "component evidence: {base_effect:?}"
                );
                for stat in [&f.base, &f.magnitude, &f.binding.output] {
                    assert_eq!(
                        f.value(&report, equipment, modifier, stat),
                        EffectValue::Unresolved {
                            reason: PlanGapReason::IncompleteContributors,
                            read: None,
                        },
                        "partial membership cannot certify any final or empty transform sequence: {report:?}"
                    );
                }
            }
        }
    }
}
