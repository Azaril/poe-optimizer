//! Finite, unpublished component domain for the actual authored minion family.
//!
//! The family descriptors, four programs and numeric binding come from the same
//! tracked artifacts as real publication. Only this independent fixture supplies
//! complete topology, catalyst boundary facts and absence of other contributors.
//! No real release closure is changed, and no source runtime supplies a result.
// The same shared artifact helper also exposes real publication entry points,
// which are deliberately unused by this hermetic component.
#[allow(dead_code)]
#[path = "owned_global_minion_level.rs"]
mod family;

use family::FamilyBindings;
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting,
    owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageInput},
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use poe_optimizer_import::{
    owned_mapping::{RegistryEntry, RegistryInput, RegistryState},
    owned_modifier_value_recipe::compile_owned_modifier_values,
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_recipe_extension::SchemaExtensionEntry,
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

pub struct Fixture {
    pub family: FamilyBindings,
    pub recipe: OwnedRecipeInput,
    pub build: BuildInput,
    scenario: ScenarioInput,
    original_family: DefinitionRules,
}
impl Fixture {
    pub fn new() -> Self {
        let family = family::bindings();
        assert_eq!(
            family.templates.len(),
            6,
            "real authored template inventory is preserved separately"
        );
        let extension = family::extension();
        assert_eq!(extension.schema_version, 1);
        let operations_version = extension
            .operations_version
            .clone()
            .expect("the real family explicitly selects its operations contract");
        assert_eq!(operations_version.as_str(), OWNED_RULE_OPERATIONS_V13);
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
        let mut definitions = family::dependency_definitions();
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
        assert!(!modifier_schema.declarations.parameters.is_complete());
        assert_eq!(modifier_schema.declarations.parameters.members.len(), 23);
        let last = definitions
            .iter()
            .map(|d| SchemaSubject::Definition(d.address()))
            .chain(slots.iter().map(|s| SchemaSubject::Slot(s.address())))
            .map(|s| sequence(&s))
            .max()
            .unwrap();
        assert_eq!(
            last, 0x30e1,
            "synthetic fixture addresses follow the actual family allocation"
        );
        let class: ClassDefId = id(&namespace, last + 1);
        let encounter: EncounterDefId = id(&namespace, last + 2);
        let template: ItemTemplateDefId = id(&namespace, last + 3);
        let destinations: Vec<EquipmentSlotDefId> =
            (4..=6).map(|n| id(&namespace, last + n)).collect();
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
            value: ParameterValue::Option(minion_catalyst),
        } = &catalyst_program
            .nodes
            .iter()
            .find(|n| n.id.as_str() == "option-minion")
            .unwrap()
            .expression
        else {
            panic!("authored minion catalyst option")
        };
        let minion_catalyst = minion_catalyst.clone();
        let options = definitions
            .iter()
            .filter_map(|d| match d {
                DefinitionDescriptor::Option(v) => Some(v.id.clone()),
                _ => None,
            })
            .collect();
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
            DefinitionDescriptor::ItemTemplate(known(
                template.clone(),
                ItemTemplateSchema {
                    item_level: level,
                    equipment_slots: DeclaredSet::complete(destinations.clone()),
                    socket_destinations: DeclaredSet::complete(vec![]),
                    modifiers: DeclaredSet::complete(vec![family.modifier.clone()]),
                    quality: QualityUseSchema {
                        presence: QualityPresence::Forbidden,
                        allowed_kinds: DeclaredSet::complete(vec![]),
                    },
                    declarations: item_slots,
                },
            )),
        ]);
        for slot in &destinations {
            definitions.push(DefinitionDescriptor::EquipmentSlot(known(
                slot.clone(),
                EquipmentSlotSchema {
                    scope: ScopePolicy::Either,
                },
            )));
        }
        for (slot, value) in [
            (
                catalyst_kind.clone(),
                ValueSchema::Option {
                    allowed: DeclaredSet::complete(options),
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
                    value,
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::ItemParameter],
                },
            )));
        }
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: 4,
                namespace: namespace.clone(),
                release: key("synthetic-global-minion-component"),
                semantics_version: key("unpublished-finite-component"),
                definitions,
                slots,
            },
            Default::default(),
        )
        .unwrap();
        let owners = vec![
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
                            unit: family.unit.clone(),
                        },
                        source: RuleReadSource::Contributions {
                            entity: RuleEntity::Player,
                            stat: family.minion_level.clone(),
                            contribution: ContributionKind::Add,
                            reduction: ContributionReduction::Sum,
                            empty: quantity(0.0, &family.unit),
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
                            stat: family.minion_level.clone(),
                            value: key("sum"),
                        },
                    }],
                }]),
            },
        ];
        let recipe = OwnedRecipeInput {
            schema_version: 1,
            registry: registry(schema.input()),
            schema: schema.input().clone(),
            rules: RulePackageInput {
                schema_version: OWNED_RULE_PACKAGE_VERSION,
                namespace: namespace.clone(),
                release: key("synthetic-global-minion-component"),
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
                release: key("synthetic-global-minion-component"),
                definitions: schema.identity().clone(),
                outputs: vec![],
            },
        };
        let checked = assemble_owned_recipe(recipe, Default::default()).unwrap();
        let numeric_policy = family::numeric_policy(schema.identity().clone());
        let compiled =
            compile_owned_modifier_values(&checked, &numeric_policy, Default::default()).unwrap();
        let compiled_owner = compiled
            .successor
            .rules
            .owners
            .iter()
            .find(|o| o.owner == original_family.owner)
            .unwrap();
        assert_eq!(
            compiled_owner.programs.closure, original_family.programs.closure,
            "compiler preserves real authored gap semantics"
        );
        assert_eq!(compiled_owner.programs.members.len(), 5);
        for program in &original_family.programs.members {
            assert!(
                compiled_owner.programs.members.contains(program),
                "authored program body changed"
            );
        }
        let rolled = |number, amount| {
            let mut rolls: Vec<_> = family
                .properties
                .iter()
                .map(|(name, slot)| ParameterAssignment {
                    slot: slot.clone(),
                    value: ParameterValue::Boolean(name == "minion"),
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
            RolledModifier {
                id: occurrence(number),
                definition: family.modifier.clone(),
                rolls,
            }
        };
        let item = |number, modifiers: Vec<RolledModifier>, amount| ItemRecord {
            id: occurrence(number),
            template: template.clone(),
            item_level: Some(20),
            quality: None,
            parameters: vec![
                ParameterAssignment {
                    slot: catalyst_kind.clone(),
                    value: ParameterValue::Option(minion_catalyst.clone()),
                },
                ParameterAssignment {
                    slot: catalyst_amount.clone(),
                    value: quantity(amount, &percent),
                },
            ],
            modifier_order: modifiers.iter().map(|m| m.id).collect(),
            modifiers,
        };
        let build = BuildInput {
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
                item(3, vec![rolled(4, 3.0), rolled(5, 5.0)], 25.0),
                item(30, vec![rolled(31, 7.0)], 0.0),
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
            recipe: compiled.successor,
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
        }
    }
    pub fn complete_domain(&mut self) {
        // This constructor owns a separate synthetic domain. Its exact finite
        // providers/programs/roll schema have all just been explicitly supplied.
        // Every fixture modifier belongs to the explicitly reviewed ordinary
        // component domain, with no implicit/enchant category transforms. Real
        // category authority remains Partial and is never changed in a release.
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
                stat: self.family.minion_level.clone(),
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
}
fn value(report: &OwnedEffectsReport, key: PlanValueKey) -> &EffectValue {
    &report
        .values
        .iter()
        .find(|v| v.key == key)
        .unwrap_or_else(|| panic!("missing {key:?}: {report:?}"))
        .value
}
