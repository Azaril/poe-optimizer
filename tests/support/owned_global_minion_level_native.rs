//! Finite, unpublished component domains for actual authored modifier families.
//!
//! Each family's descriptors, programs and numeric binding come from the same
//! tracked artifacts as real publication. Only this independent fixture supplies
//! complete topology, catalyst boundary facts and absence of other contributors.
//! No real release closure is changed, and no source runtime supplies a result.
// The same shared artifact helper also exposes real publication entry points,
// which are deliberately unused by this hermetic component.
#[allow(dead_code)]
#[path = "owned_modifier_category.rs"]
pub mod categories;
#[allow(dead_code)]
#[path = "owned_global_minion_level.rs"]
mod family;

use poe_optimizer_core::{
    build_identity::*, data::DataIdentity, owned_build::*, owned_definitions::*, owned_routing::*,
    owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting,
    owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageInput},
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use poe_optimizer_import::{
    owned_mapping::{RegistryEntry, RegistryInput, RegistryState},
    owned_modifier_value_recipe::{ModifierValuePolicy, compile_owned_modifier_values},
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry},
};
use std::{collections::BTreeMap, sync::Arc};

fn key(value: &str) -> OwnedDefinitionKey {
    value.parse().unwrap()
}
fn integer(value: i64) -> BoundedInteger {
    BoundedInteger::new(value).unwrap()
}
fn known<I, S>(id: I, schema: S) -> DefinitionEntry<I, S> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn empty_slots() -> DeclaredSlots {
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
fn id<K: DefinitionDomain>(namespace: &GameVersionNamespace, number: u64) -> DefId<K> {
    DefId::parse(namespace.clone(), format!("def.{number:016x}")).unwrap()
}
fn sequence(subject: &SchemaSubject) -> u64 {
    let text = match subject {
        SchemaSubject::Definition(value) => value.key(),
        SchemaSubject::Slot(value) => value.key(),
    }
    .as_str();
    u64::from_str_radix(text.strip_prefix("def.").unwrap(), 16).unwrap()
}
fn registry(schema: &SchemaPackageInput) -> RegistryInput {
    let mut actual = BTreeMap::new();
    for subject in schema
        .definitions
        .iter()
        .map(|d| SchemaSubject::Definition(d.address()))
        .chain(
            schema
                .slots
                .iter()
                .map(|s| SchemaSubject::Slot(s.address())),
        )
    {
        assert!(
            actual.insert(sequence(&subject), subject).is_none(),
            "conflicting allocated key/domain"
        );
    }
    let last = *actual.last_key_value().unwrap().0;
    // The numeric compiler requires a dense allocation history. These unused
    // positions are deliberately synthetic registry-only Option addresses; no
    // schema/program or source meaning is attached to them. This history never
    // leaves this fixture or enters a publication path.
    RegistryInput {
        schema_version: 1,
        namespace: schema.namespace.clone(),
        revision: integer(last as i64),
        last_issued: integer(last as i64),
        entries: (1..=last)
            .map(|number| RegistryEntry {
                sequence: integer(number as i64),
                target: actual.remove(&number).unwrap_or_else(|| {
                    SchemaSubject::Definition(
                        id::<OptionDefinition>(&schema.namespace, number).address(),
                    )
                }),
                state: RegistryState::Active,
            })
            .collect(),
    }
}
fn scalar_program(
    name: &str,
    slot: DeclaredSlot<ParameterSlotDefId>,
    stat: StatDefId,
    value_type: ComputedValueType,
) -> RuleProgram {
    RuleProgram {
        id: key(name),
        context: RuleEntityKind::EquipmentUse,
        reads: vec![RuleRead {
            id: key("input"),
            value_type,
            source: RuleReadSource::Parameter { slot },
        }],
        nodes: vec![RuleNode {
            id: key("input"),
            expression: RuleExpression::Read {
                input: key("input"),
            },
        }],
        effects: vec![RuleEffect {
            id: key("output"),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat,
                value: key("input"),
            },
        }],
    }
}
fn quantity(value: f64, unit: &UnitDefId) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap())
}
pub fn occurrence<T: BuildInstanceId>(number: u64) -> T {
    T::from_instance_id(
        InstanceId::from_parts(BuildLineage::from_bytes([0x77; 16]), number).unwrap(),
    )
}

/// Inputs to the shared, unpublished topology fixture. Artifact bodies are
/// supplied intact; these addresses describe their boundaries, not new formulas.
pub struct ComponentBindings {
    pub modifier: ModifierDefId,
    pub amount: DeclaredSlot<ParameterSlotDefId>,
    pub properties: BTreeMap<String, DeclaredSlot<ParameterSlotDefId>>,
    pub corrupted_base: DeclaredSlot<ParameterSlotDefId>,
    pub unit: UnitDefId,
    pub contribution_unit: UnitDefId,
    pub factor_unit: UnitDefId,
    pub effective: StatDefId,
    pub contribution: StatDefId,
}
pub struct CategoryBindings {
    pub slot: DeclaredSlot<ParameterSlotDefId>,
    pub explicit: OptionDefId,
    pub implicit: OptionDefId,
    pub enchant: OptionDefId,
}
pub struct AuthoredComponent {
    pub bindings: ComponentBindings,
    pub extension: OwnedRecipeExtension,
    pub dependencies: Vec<DefinitionDescriptor>,
    pub numeric_policy: fn(DataIdentity) -> ModifierValuePolicy,
    pub category: Option<CategoryBindings>,
    pub category_target: Option<OptionDefId>,
    pub catalyst_property: &'static str,
    pub catalyst_amount: f64,
    pub parameter_count: usize,
    pub parameters_complete: bool,
    pub last_authored: u64,
    pub release: &'static str,
}
/// Actual template input programs/slots with explicit canonical input values.
/// Only the surrounding finite topology is supplied by this unpublished fixture.
pub struct AuthoredTemplateInputs {
    pub template: ItemTemplateDefId,
    pub slots: Vec<SlotDescriptor>,
    pub owner: DefinitionRules,
    pub assignments: Vec<ParameterAssignment>,
}
pub struct Fixture {
    pub family: ComponentBindings,
    pub recipe: OwnedRecipeInput,
    pub build: BuildInput,
    scenario: ScenarioInput,
    original_family: DefinitionRules,
    original_template_owners: Vec<DefinitionRules>,
}
impl Fixture {
    pub fn new() -> Self {
        Self::domain(None)
    }
    #[allow(dead_code)]
    pub fn with_categories(target: OptionDefId) -> Self {
        Self::domain(Some(target))
    }
    fn domain(category_target: Option<OptionDefId>) -> Self {
        let binding = family::bindings();
        assert_eq!(binding.templates.len(), 6);
        let mut extension = family::extension();
        let category = category_target.as_ref().map(|_| categories::bindings());
        if let Some(category) = &category {
            assert_eq!(category.modifier, binding.modifier);
            let refinement = categories::extension();
            assert_eq!(refinement.schema_version, 1);
            assert!(
                refinement
                    .operations_version
                    .as_ref()
                    .is_none_or(|v| v.as_str() == OWNED_RULE_OPERATIONS_V13)
            );
            assert!(refinement.owners.is_empty());
            assert!(refinement.tables.is_empty());
            assert!(refinement.receivers.is_empty());
            for entry in refinement.schema {
                match &entry {
                    SchemaExtensionEntry::Definition(value)
                        if value.address() == binding.modifier.address() =>
                    {
                        let prior = extension.schema.iter_mut().find(|e| matches!(e, SchemaExtensionEntry::Definition(d) if d.address() == value.address())).unwrap();
                        *prior = entry;
                    }
                    _ => extension.schema.push(entry),
                }
            }
        }
        Self::from_authored(AuthoredComponent {
            bindings: ComponentBindings {
                modifier: binding.modifier,
                amount: binding.amount,
                properties: binding.properties,
                corrupted_base: binding.corrupted_base,
                contribution_unit: binding.unit.clone(),
                unit: binding.unit,
                factor_unit: binding.factor_unit,
                effective: binding.effective,
                contribution: binding.minion_level,
            },
            extension,
            dependencies: family::dependency_definitions(),
            numeric_policy: family::numeric_policy,
            parameter_count: 23 + usize::from(category.is_some()),
            parameters_complete: false,
            last_authored: if category.is_some() { 0x30e5 } else { 0x30e1 },
            category: category.map(|c| CategoryBindings {
                slot: c.slot,
                explicit: c.explicit,
                implicit: c.implicit,
                enchant: c.enchant,
            }),
            category_target,
            catalyst_property: "minion",
            catalyst_amount: 25.0,
            release: "synthetic-global-minion-component",
        })
    }
    pub fn from_authored(component: AuthoredComponent) -> Self {
        Self::build_component(component, None, true)
    }
    #[allow(dead_code)]
    pub fn from_authored_with_template_inputs(
        component: AuthoredComponent,
        templates: Vec<AuthoredTemplateInputs>,
    ) -> Self {
        assert_eq!(
            templates.len(),
            2,
            "one authored owner per finite physical item"
        );
        assert_ne!(templates[0].template, templates[1].template);
        Self::build_component(component, Some(templates), true)
    }
    /// Use an already compiled, numerical-only family without inventing a
    /// contribution or an aggregate. The actual single template is reused by
    /// the finite physical test records; callers may retain just one record.
    #[allow(dead_code)]
    pub fn from_compiled_effects_with_template_inputs(
        component: AuthoredComponent,
        template: AuthoredTemplateInputs,
    ) -> Self {
        Self::build_component(component, Some(vec![template]), false)
    }
    /// Exercise compiled numerical programs with explicit finite test producers.
    /// This supplies no physical-template proof, contribution or aggregate.
    #[allow(dead_code)]
    pub fn from_compiled_effects(component: AuthoredComponent) -> Self {
        Self::build_component(component, None, false)
    }
    fn build_component(
        component: AuthoredComponent,
        authored_templates: Option<Vec<AuthoredTemplateInputs>>,
        compile_numeric: bool,
    ) -> Self {
        let AuthoredComponent {
            bindings: family,
            extension,
            dependencies,
            numeric_policy,
            category,
            category_target,
            catalyst_property,
            catalyst_amount: fixture_catalyst_amount,
            parameter_count,
            parameters_complete,
            last_authored,
            release,
        } = component;
        let transforms = category_target.is_some();
        assert!(!transforms || category.is_some());
        assert_eq!(extension.schema_version, 1);
        let operations_version = extension
            .operations_version
            .clone()
            .expect("the actual family selects its operations contract");
        assert!(matches!(
            RuleOperationsVersion::parse(operations_version.as_str()),
            Some(RuleOperationsVersion::V13 | RuleOperationsVersion::V14)
        ));
        assert_eq!(extension.owners.len(), 1);
        let original_family = extension.owners[0].clone();
        assert_eq!(
            original_family.owner,
            SchemaSubject::Definition(family.modifier.address())
        );
        assert_eq!(original_family.programs.members.len(), 4);
        assert!(matches!(
            original_family.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        let namespace = family.modifier.namespace().clone();
        let mut definitions = dependencies;
        let catalyst_options = definitions
            .iter()
            .filter_map(|definition| match definition {
                DefinitionDescriptor::Option(value)
                    if category.as_ref().is_none_or(|category| {
                        ![&category.explicit, &category.implicit, &category.enchant]
                            .contains(&&value.id)
                    }) =>
                {
                    Some(value.id.clone())
                }
                _ => None,
            })
            .collect();
        let mut slots = vec![];
        for entry in &extension.schema {
            match entry {
                SchemaExtensionEntry::Definition(value) => definitions.push(value.clone()),
                SchemaExtensionEntry::Slot(value) => slots.push(value.clone()),
            }
        }
        let DefinitionDescriptor::Modifier(DefinitionEntry {
            schema: SchemaState::Known(modifier_schema),
            ..
        }) = definitions
            .iter()
            .find(|d| d.address() == family.modifier.address())
            .unwrap()
        else {
            panic!("actual authored modifier schema")
        };
        assert_eq!(
            modifier_schema.declarations.parameters.is_complete(),
            parameters_complete
        );
        assert_eq!(
            modifier_schema.declarations.parameters.members.len(),
            parameter_count
        );
        let last = definitions
            .iter()
            .map(|d| SchemaSubject::Definition(d.address()))
            .chain(slots.iter().map(|s| SchemaSubject::Slot(s.address())))
            .map(|s| sequence(&s))
            .max()
            .unwrap();
        assert_eq!(
            last, last_authored,
            "synthetic fixture addresses follow the actual family allocation"
        );
        let original_template_owners: Vec<_> = authored_templates
            .as_ref()
            .into_iter()
            .flatten()
            .map(|input| {
                assert_eq!(
                    input.owner.owner,
                    SchemaSubject::Definition(input.template.address())
                );
                assert_eq!(input.template.namespace(), &namespace);
                for slot in &input.slots {
                    assert_eq!(
                        slot.address().declaration(),
                        &SlotOwnerDefId::ItemTemplate(input.template.clone())
                    );
                    assert!(matches!(slot, SlotDescriptor::Parameter(_)));
                }
                input.owner.clone()
            })
            .collect();
        let last = authored_templates
            .as_ref()
            .into_iter()
            .flatten()
            .fold(last, |last, input| {
                input
                    .slots
                    .iter()
                    .map(|s| sequence(&SchemaSubject::Slot(s.address())))
                    .chain(std::iter::once(sequence(&SchemaSubject::Definition(
                        input.template.address(),
                    ))))
                    .fold(last, u64::max)
            });
        let class: ClassDefId = id(&namespace, last + 1);
        let encounter: EncounterDefId = id(&namespace, last + 2);
        let template: ItemTemplateDefId = id(&namespace, last + 3);
        let destinations: Vec<EquipmentSlotDefId> =
            (4..=6).map(|n| id(&namespace, last + n)).collect();
        let transform_producer: ModifierDefId = id(&namespace, last + 10);
        let catalyst_kind = DeclaredSlot {
            declaration: SlotOwnerDefId::ItemTemplate(template.clone()),
            slot: id(&namespace, last + 7),
        };
        let catalyst_amount = DeclaredSlot {
            declaration: SlotOwnerDefId::ItemTemplate(template.clone()),
            slot: id(&namespace, last + 8),
        };
        let catalyst_program = original_family
            .programs
            .members
            .iter()
            .find(|p| p.id.as_str() == "catalyst-scalar")
            .unwrap();
        let stat_read = |name: &str| {
            let read = catalyst_program
                .reads
                .iter()
                .find(|r| r.id.as_str() == name)
                .unwrap();
            let RuleReadSource::Stat {
                entity: RuleEntity::Current,
                stat,
            } = &read.source
            else {
                panic!("exact equipment scalar boundary")
            };
            (stat.clone(), read.value_type.clone())
        };
        let (kind_stat, kind_type) = stat_read("catalyst-kind");
        let (amount_stat, amount_type) = stat_read("catalyst-amount");
        let ComputedValueType::Quantity { unit: percent } = &amount_type else {
            panic!("percentage amount")
        };
        let percent = percent.clone();
        let RuleExpression::Literal {
            value: ParameterValue::Option(selected_catalyst),
        } = &catalyst_program
            .nodes
            .iter()
            .find(|n| n.id.as_str() == format!("option-{catalyst_property}"))
            .unwrap()
            .expression
        else {
            panic!("authored selected catalyst option")
        };
        let selected_catalyst = selected_catalyst.clone();
        let level = IntegerRange {
            minimum: integer(1),
            maximum: integer(100),
        };
        let mut item_slots = empty_slots();
        item_slots.parameters =
            DeclaredSet::complete(vec![catalyst_kind.clone(), catalyst_amount.clone()]);
        definitions.extend([
            DefinitionDescriptor::Class(known(
                class.clone(),
                ClassSchema {
                    level: level.clone(),
                    ascendancies: DeclaredSet::complete(vec![]),
                    implicit_passives: DeclaredSet::complete(vec![]),
                    declarations: empty_slots(),
                },
            )),
            DefinitionDescriptor::Encounter(known(
                encounter.clone(),
                EncounterSchema {
                    enemy_level: level.clone(),
                    external_inputs: DeclaredSet::complete(vec![]),
                },
            )),
        ]);
        let template_ids: Vec<_> = authored_templates
            .as_ref()
            .map(|inputs| inputs.iter().map(|v| v.template.clone()).collect())
            .unwrap_or_else(|| vec![template.clone()]);
        for (index, template_id) in template_ids.iter().enumerate() {
            let declarations = if let Some(inputs) = &authored_templates {
                let mut declarations = empty_slots();
                declarations.parameters = DeclaredSet::complete(
                    inputs[index]
                        .slots
                        .iter()
                        .map(|slot| {
                            let SlotDescriptor::Parameter(slot) = slot else {
                                unreachable!()
                            };
                            slot.id.clone()
                        })
                        .collect(),
                );
                declarations
            } else {
                item_slots.clone()
            };
            definitions.push(DefinitionDescriptor::ItemTemplate(known(
                template_id.clone(),
                ItemTemplateSchema {
                    item_level: level.clone(),
                    equipment_slots: DeclaredSet::complete(destinations.clone()),
                    socket_destinations: DeclaredSet::complete(vec![]),
                    modifiers: DeclaredSet::complete(if transforms {
                        vec![family.modifier.clone(), transform_producer.clone()]
                    } else {
                        vec![family.modifier.clone()]
                    }),
                    quality: QualityUseSchema {
                        presence: QualityPresence::Forbidden,
                        allowed_kinds: DeclaredSet::complete(vec![]),
                    },
                    declarations,
                },
            )));
        }
        for slot in &destinations {
            definitions.push(DefinitionDescriptor::EquipmentSlot(known(
                slot.clone(),
                EquipmentSlotSchema {
                    scope: ScopePolicy::Either,
                },
            )));
        }
        if let Some(inputs) = &authored_templates {
            for input in inputs {
                slots.extend(input.slots.clone());
            }
        } else {
            for (slot, value) in [
                (
                    catalyst_kind.clone(),
                    ValueSchema::Option {
                        allowed: DeclaredSet::complete(catalyst_options),
                    },
                ),
                (
                    catalyst_amount.clone(),
                    ValueSchema::Quantity(QuantityRange {
                        minimum: FiniteQuantity::new(0.0, percent.clone()).unwrap(),
                        maximum: FiniteQuantity::new(1000.0, percent.clone()).unwrap(),
                    }),
                ),
            ] {
                slots.push(SlotDescriptor::Parameter(known(
                    slot,
                    ParameterSlotSchema {
                        skill_input: None,
                        value,
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![ParameterSite::ItemParameter],
                    },
                )));
            }
        }
        let eligibility: StatDefId = id(&namespace, last + 9);
        if transforms {
            definitions.push(DefinitionDescriptor::Stat(known(
                eligibility.clone(),
                StatSchema {
                    value: ComputedValueType::Boolean,
                    targets: vec![RuleEntityKind::Modifier],
                },
            )));
            definitions.push(DefinitionDescriptor::Modifier(known(
                transform_producer.clone(),
                ModifierSchema {
                    declarations: empty_slots(),
                },
            )));
        }
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: 4,
                namespace: namespace.clone(),
                release: key(release),
                semantics_version: key("unpublished-finite-component"),
                definitions,
                slots,
            },
            Default::default(),
        )
        .unwrap();
        let mut owners = vec![
            original_family.clone(),
            DefinitionRules {
                owner: SchemaSubject::Definition(template.address()),
                programs: DeclaredSet::complete(vec![
                    scalar_program(
                        "fixture-catalyst-kind",
                        catalyst_kind.clone(),
                        kind_stat,
                        kind_type,
                    ),
                    scalar_program(
                        "fixture-catalyst-amount",
                        catalyst_amount.clone(),
                        amount_stat,
                        amount_type,
                    ),
                ]),
            },
            DefinitionRules {
                owner: SchemaSubject::Definition(encounter.address()),
                programs: DeclaredSet::complete(vec![]),
            },
            DefinitionRules {
                owner: SchemaSubject::Definition(class.address()),
                programs: DeclaredSet::complete(vec![RuleProgram {
                    id: key("fixture-player-sum"),
                    context: RuleEntityKind::Actor,
                    reads: vec![RuleRead {
                        id: key("incoming"),
                        value_type: ComputedValueType::Quantity {
                            unit: family.contribution_unit.clone(),
                        },
                        source: RuleReadSource::Contributions {
                            entity: RuleEntity::Player,
                            stat: family.contribution.clone(),
                            contribution: ContributionKind::Add,
                            reduction: ContributionReduction::Sum,
                            empty: quantity(0.0, &family.contribution_unit),
                        },
                    }],
                    nodes: vec![RuleNode {
                        id: key("sum"),
                        expression: RuleExpression::Read {
                            input: key("incoming"),
                        },
                    }],
                    effects: vec![RuleEffect {
                        id: key("final"),
                        when: None,
                        effect: RuleEffectKind::Derive {
                            entity: RuleEntity::Player,
                            stat: family.contribution.clone(),
                            value: key("sum"),
                        },
                    }],
                }]),
            },
        ];
        if authored_templates.is_some() {
            let removed = owners.remove(1);
            assert_eq!(removed.owner, SchemaSubject::Definition(template.address()));
            owners.extend(original_template_owners.clone());
        }
        if !compile_numeric {
            let class_owner = owners
                .iter_mut()
                .find(|owner| owner.owner == SchemaSubject::Definition(class.address()))
                .unwrap();
            class_owner.programs.members.clear();
        }
        if let (Some(category), Some(target)) = (&category, &category_target) {
            assert!([&category.explicit, &category.implicit, &category.enchant].contains(&target));
            owners[0].programs.members.push(RuleProgram {
                id: key("fixture-category-eligibility"),
                context: RuleEntityKind::EquipmentUse,
                reads: vec![RuleRead {
                    id: key("category"),
                    value_type: ComputedValueType::Option,
                    source: RuleReadSource::Parameter {
                        slot: category.slot.clone(),
                    },
                }],
                nodes: vec![
                    RuleNode {
                        id: key("category"),
                        expression: RuleExpression::Read {
                            input: key("category"),
                        },
                    },
                    RuleNode {
                        id: key("target"),
                        expression: RuleExpression::Literal {
                            value: ParameterValue::Option(target.clone()),
                        },
                    },
                    RuleNode {
                        id: key("eligible"),
                        expression: RuleExpression::Compare {
                            operation: RuleComparison::Equal,
                            left: key("category"),
                            right: key("target"),
                        },
                    },
                ],
                effects: vec![RuleEffect {
                    id: key("eligible"),
                    when: None,
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Modifier,
                        stat: eligibility.clone(),
                        value: key("eligible"),
                    },
                }],
            });
            let transform = original_family
                .programs
                .members
                .iter()
                .find(|p| p.id.as_str() == "ordered-magnitude-scalar")
                .unwrap()
                .reads
                .iter()
                .find_map(|r| match &r.source {
                    RuleReadSource::ModifierTransforms { stat, .. } => Some(stat.clone()),
                    _ => None,
                })
                .unwrap();
            owners.push(DefinitionRules {
                owner: SchemaSubject::Definition(transform_producer.address()),
                programs: DeclaredSet::complete(vec![RuleProgram {
                    id: key("fixture-category-transform"),
                    context: RuleEntityKind::EquipmentUse,
                    reads: vec![],
                    nodes: vec![RuleNode {
                        id: key("amount"),
                        expression: RuleExpression::Literal {
                            value: quantity(0.5, &family.factor_unit),
                        },
                    }],
                    effects: vec![RuleEffect {
                        id: key("project"),
                        when: None,
                        effect: RuleEffectKind::ProjectModifierTransform {
                            stat: transform,
                            targets: vec![ModifierTransformTarget {
                                definition: family.modifier.clone(),
                                when: Some(eligibility),
                            }],
                            order: integer(0),
                            operation: ModifierTransformOperation::Add,
                            value: key("amount"),
                        },
                    }],
                }]),
            });
        }
        let recipe = OwnedRecipeInput {
            schema_version: 1,
            registry: registry(schema.input()),
            schema: schema.input().clone(),
            rules: RulePackageInput {
                effect_applications: None,
                schema_version: OWNED_RULE_PACKAGE_VERSION,
                namespace: namespace.clone(),
                release: key(release),
                semantics_version: key("unpublished-finite-component"),
                operations_version,
                definitions: schema.identity().clone(),
                tables: extension.tables.clone(),
                owners,
                receivers: DeclaredSet::complete(extension.receivers.clone()),
            },
            routing: ActionRoutingInput {
                schema_version: OWNED_ACTION_ROUTING_VERSION,
                namespace: namespace.clone(),
                release: key(release),
                definitions: schema.identity().clone(),
                outputs: vec![],
            },
        };
        let checked = assemble_owned_recipe(recipe.clone(), Default::default()).unwrap();
        let successor = if compile_numeric {
            let numeric_policy = numeric_policy(schema.identity().clone());
            compile_owned_modifier_values(&checked, &numeric_policy, Default::default())
                .unwrap()
                .successor
        } else {
            recipe
        };
        let compiled_owner = successor
            .rules
            .owners
            .iter()
            .find(|o| o.owner == original_family.owner)
            .unwrap();
        assert_eq!(
            compiled_owner.programs.closure, original_family.programs.closure,
            "compiler preserves real authored gap semantics"
        );
        assert_eq!(
            compiled_owner.programs.members.len(),
            4 + usize::from(compile_numeric) + usize::from(transforms)
        );
        for program in &original_family.programs.members {
            assert!(
                compiled_owner.programs.members.contains(program),
                "authored program body changed"
            );
        }
        for owner in &original_template_owners {
            assert_eq!(
                successor
                    .rules
                    .owners
                    .iter()
                    .find(|v| v.owner == owner.owner),
                Some(owner),
                "authored template input programs/closure remain unchanged"
            );
        }
        let rolled = |number, amount| {
            let mut rolls: Vec<_> = family
                .properties
                .iter()
                .map(|(name, slot)| ParameterAssignment {
                    slot: slot.clone(),
                    value: ParameterValue::Boolean(name == catalyst_property),
                })
                .collect();
            rolls.extend([
                ParameterAssignment {
                    slot: family.amount.clone(),
                    value: quantity(amount, &family.unit),
                },
                ParameterAssignment {
                    slot: family.corrupted_base.clone(),
                    value: quantity(1.0, &family.factor_unit),
                },
            ]);
            if let Some(category) = &category {
                rolls.push(ParameterAssignment {
                    slot: category.slot.clone(),
                    value: ParameterValue::Option(match number {
                        4 => category.explicit.clone(),
                        5 => category.implicit.clone(),
                        31 => category.enchant.clone(),
                        _ => unreachable!(),
                    }),
                });
            }
            RolledModifier {
                id: occurrence(number),
                definition: family.modifier.clone(),
                rolls,
            }
        };
        let item = |index: usize, number, mut modifiers: Vec<RolledModifier>, amount| {
            if transforms {
                modifiers.push(RolledModifier {
                    id: occurrence(if number == 3 { 40 } else { 41 }),
                    definition: transform_producer.clone(),
                    rolls: vec![],
                });
            }
            ItemRecord {
                id: occurrence(number),
                template: authored_templates
                    .as_ref()
                    .map_or_else(|| template.clone(), |v| v[index % v.len()].template.clone()),
                item_level: Some(20),
                quality: None,
                parameters: authored_templates.as_ref().map_or_else(
                    || {
                        vec![
                            ParameterAssignment {
                                slot: catalyst_kind.clone(),
                                value: ParameterValue::Option(selected_catalyst.clone()),
                            },
                            ParameterAssignment {
                                slot: catalyst_amount.clone(),
                                value: quantity(amount, &percent),
                            },
                        ]
                    },
                    |v| v[index % v.len()].assignments.clone(),
                ),
                modifier_order: modifiers.iter().map(|m| m.id).collect(),
                modifiers,
            }
        };
        let build = BuildInput {
            generated_inputs: None,
            support_origins: None,
            allocator: InstanceAllocatorState::from_parts(
                BuildLineage::from_bytes([0x77; 16]),
                100,
            ),
            revision: BuildRevision::from_u64(1),
            game_version: namespace.clone(),
            character: CharacterSpec {
                class,
                ascendancy: None,
                level: 20,
                rewards: vec![],
            },
            weapon_loadouts: vec![occurrence(1), occurrence(2)],
            active_weapon_loadout: occurrence(1),
            items: vec![
                item(
                    0,
                    3,
                    vec![
                        rolled(4, if transforms { 5.0 } else { 3.0 }),
                        rolled(5, 5.0),
                    ],
                    if transforms {
                        0.0
                    } else {
                        fixture_catalyst_amount
                    },
                ),
                item(
                    1,
                    30,
                    vec![rolled(31, if transforms { 5.0 } else { 7.0 })],
                    0.0,
                ),
            ],
            equipment: [(6, 3, 0), (7, 3, 1), (32, 30, 2)]
                .into_iter()
                .map(|(use_id, item, slot)| EquipmentUse {
                    id: occurrence(use_id),
                    item: occurrence(item),
                    destination: EquipmentDestination::CharacterSlot(destinations[slot].clone()),
                    scope: LoadoutScope::Shared,
                })
                .collect(),
            gems: vec![],
            allocations: vec![],
            skills: vec![],
            supports: vec![],
            payload_links: vec![],
            choices: vec![],
        };
        Self {
            family,
            recipe: successor,
            build,
            scenario: ScenarioInput {
                game_version: namespace,
                enemy: EnemySpec {
                    encounter,
                    level: 20,
                },
                assumptions: vec![],
                usage: vec![],
            },
            original_family,
            original_template_owners,
        }
    }
    pub fn complete_domain(&mut self) {
        // This constructor owns a separate synthetic domain. Its exact finite
        // providers/programs/roll schema have all just been explicitly supplied.
        // The original fixture has no category transforms. The opt-in extension
        // supplies all three exact category Options and one finite test producer.
        // These closures apply only to this domain, never to a real release.
        self.family_owner_mut().programs.closure = SchemaClosure::Complete;
        let definition = self
            .recipe
            .schema
            .definitions
            .iter_mut()
            .find(|d| d.address() == self.family.modifier.address())
            .unwrap();
        let DefinitionDescriptor::Modifier(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) = definition
        else {
            unreachable!()
        };
        schema.declarations.parameters.closure = SchemaClosure::Complete;
        for original in &self.original_template_owners {
            self.recipe
                .rules
                .owners
                .iter_mut()
                .find(|v| v.owner == original.owner)
                .unwrap()
                .programs
                .closure = SchemaClosure::Complete;
        }
        self.rebind();
    }
    fn rebind(&mut self) {
        let schema =
            OwnedDefinitionSchemaPackage::new(self.recipe.schema.clone(), Default::default())
                .unwrap();
        self.recipe.rules.definitions = schema.identity().clone();
        self.recipe.routing.definitions = schema.identity().clone();
    }
    pub fn family_owner_mut(&mut self) -> &mut DefinitionRules {
        self.recipe
            .rules
            .owners
            .iter_mut()
            .find(|owner| owner.owner == self.original_family.owner)
            .unwrap()
    }
    pub fn restore_partial_rules(&mut self) {
        let closure = self.original_family.programs.closure.clone();
        self.family_owner_mut().programs.closure = closure;
    }
    #[allow(dead_code)]
    pub fn restore_template_rules(&mut self) {
        for original in &self.original_template_owners {
            self.recipe
                .rules
                .owners
                .iter_mut()
                .find(|v| v.owner == original.owner)
                .unwrap()
                .programs
                .closure = original.programs.closure.clone();
        }
    }
    pub fn set_raw(&mut self, item: usize, modifier: usize, value: f64) {
        self.build.items[item].modifiers[modifier]
            .rolls
            .iter_mut()
            .find(|p| p.slot == self.family.amount)
            .unwrap()
            .value = quantity(value, &self.family.unit);
        self.build.revision = BuildRevision::from_u64(2);
    }
    pub fn plan(&self) -> Result<OwnedEffectPlan<OwnedDefinitionSchemaPackage>> {
        let schema = Arc::new(
            OwnedDefinitionSchemaPackage::new(self.recipe.schema.clone(), Default::default())
                .unwrap(),
        );
        let rules = Arc::new(
            CompiledRulePackage::compile(&self.recipe.rules, schema.as_ref(), Default::default())
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
    }
    pub fn total<'a>(&self, report: &'a OwnedEffectsReport) -> &'a EffectValue {
        value(
            report,
            PlanValueKey::Stat {
                entity: ConcreteEntity::Actor(ActorKey::Player),
                stat: self.family.contribution.clone(),
            },
        )
    }
    pub fn effective<'a>(
        &self,
        report: &'a OwnedEffectsReport,
        equipment: u64,
        modifier: u64,
    ) -> &'a EffectValue {
        value(
            report,
            PlanValueKey::Stat {
                entity: ConcreteEntity::Modifier(ProviderKey {
                    root: ProviderRoot::ItemModifier {
                        equipment_use: occurrence(equipment),
                        modifier: occurrence(modifier),
                    },
                    grant_path: vec![],
                }),
                stat: self.family.effective.clone(),
            },
        )
    }
    pub fn expected(&self, number: f64) -> EffectValue {
        EffectValue::Known {
            value: quantity(number, &self.family.unit),
        }
    }
    /// The authored contribution may project into a distinct semantic unit.
    #[allow(dead_code)]
    pub fn expected_total(&self, number: f64) -> EffectValue {
        EffectValue::Known {
            value: quantity(number, &self.family.contribution_unit),
        }
    }
}
fn value(report: &OwnedEffectsReport, key: PlanValueKey) -> &EffectValue {
    &report
        .values
        .iter()
        .find(|v| v.key == key)
        .unwrap_or_else(|| panic!("missing {key:?}: {report:?}"))
        .value
}
