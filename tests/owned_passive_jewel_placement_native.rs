//! Published Ruby inputs through a finite, unpublished passive receiving topology.
//! This proves native occurrence relations, not complete original-build parity.
#[allow(dead_code)]
#[path = "support/owned_global_minion_level_native.rs"]
mod component;
#[allow(dead_code)]
#[path = "support/owned_fire_damage_modifier.rs"]
mod fire;
#[allow(dead_code)]
#[path = "support/owned_ruby_item_inputs.rs"]
mod physical;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;

use component::{
    AuthoredComponent, AuthoredTemplateInputs, CategoryBindings, ComponentBindings, Fixture,
    occurrence,
};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_project::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::{
    owned_normalize::OrdinaryPassiveSocketBinding, owned_recipe_extension::*,
    owned_release::StagedOwnedRelease,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};

fn typed<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn one(rows: &Value, predicate: impl Fn(&Value) -> bool) -> &Value {
    let found: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| predicate(v))
        .collect();
    assert_eq!(found.len(), 1);
    found[0]
}
fn assignments(collection: &Value) -> Vec<ParameterAssignment> {
    assert_eq!(collection["completion"], json!({"kind":"complete"}));
    collection["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            assert_eq!(row["slot"]["kind"], "known");
            assert_eq!(row["value"]["kind"], "known");
            ParameterAssignment {
                slot: typed(&row["slot"]["value"]),
                value: typed(&row["value"]["value"]),
            }
        })
        .collect()
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
fn known<I, S>(id: I, schema: S) -> DefinitionEntry<I, S> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}

struct Inputs {
    endpoint: StagedOwnedRelease,
    item: Value,
    pool: PointPoolDefId,
    ruby: OrdinaryPassiveSocketBinding,
    other: OrdinaryPassiveSocketBinding,
    expected: f64,
}
impl Inputs {
    fn load() -> Self {
        let authoring: Value = serde_json::from_str(include_str!(
            "../data/owned/poe2/3887ae68/ordinary-passive-jewel-placement/authoring.json"
        ))
        .unwrap();
        assert_eq!(authoring["source_validation"]["status"], "passed");
        let bytes = fs::read(
            std::env::var_os("POE_OPTIMIZER_TEST_PASSIVE_JEWEL_PLACEMENT_SOURCE")
                .expect("set to verified complete passive-jewel source evidence"),
        )
        .unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            authoring["source_validation"]["evidence_sha256"]
                .as_str()
                .unwrap()
        );
        let reference: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(reference["source_revision"], authoring["source_revision"]);
        assert_eq!(
            reference["source_hash"],
            authoring["source_manifest_sha256"]
        );
        let original = one(&reference["cases"], |c| c["name"] == "original-04");
        let xml = fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/builds/breadth-20260908/build-04.xml"),
        )
        .unwrap();
        assert_eq!(original["xml_sha256"], format!("{:x}", Sha256::digest(xml)));
        for field in [
            "saved_items_preserved",
            "saved_specs_preserved",
            "saved_selections_preserved",
            "main_output_preserved",
            "calcs_output_preserved",
            "original_functions_preserved",
        ] {
            assert_eq!(original["state"][field], true, "{field}");
        }
        let received = one(&original["state"]["assignments"], |r| {
            r["node"] == 46882 && r["item_id"] == 1
        });
        assert_eq!(received["valid"], true);
        assert_eq!(received["node_facts"]["alloc_mode"], 0);
        for mode in ["main", "calcs"] {
            assert_eq!(received["receiving"][mode], true);
        }
        let effect = |mode: &str| {
            one(&received["merged_modifiers"][mode], |m| {
                m["name"] == "FireDamage" && m["type"] == "INC"
            })["value"]
                .as_f64()
                .unwrap()
        };
        let expected = effect("main");
        assert_eq!(effect("calcs"), expected);

        let output = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_PASSIVE_JEWEL_PLACEMENT_OUTPUT")
                .expect("set to verified passive-jewel publication output"),
        );
        let endpoint = release::load(&output.join("package"));
        let read = |name| -> Value {
            serde_json::from_slice(&fs::read(output.join(name)).unwrap()).unwrap()
        };
        let draft = read("original-04/draft.json");
        let sidecar = read("original-04/sidecar.json");
        let origin = one(&sidecar["origins"], |o| o["source"]["ordinal"] == 171);
        let item_id = &one(&origin["links"], |l| l["kind"] == "item")["value"];
        let item = one(&draft["draft"]["items"]["members"], |i| &i["id"] == item_id).clone();
        let receiving = one(&draft["draft"]["equipment"]["members"], |e| {
            &e["item"]["value"] == item_id
        });
        assert_eq!(
            receiving["scope"],
            json!({"kind":"known","value":{"kind":"shared"}})
        );
        assert_eq!(receiving["destination"]["kind"], "passive_socket");
        let destination = &receiving["destination"]["value"];
        let allocation = one(&draft["draft"]["allocations"]["members"], |a| {
            a["id"] == destination["allocation"]["value"]
        });
        let bindings: Vec<OrdinaryPassiveSocketBinding> = serde_json::from_str(include_str!(
            "../data/owned/poe2/3887ae68/ordinary-passive-jewel-placement/bindings.json"
        ))
        .unwrap();
        let ruby = bindings
            .iter()
            .find(|r| r.node_token == "46882")
            .unwrap()
            .clone();
        let other = bindings
            .iter()
            .find(|r| r.node_token == "7960")
            .unwrap()
            .clone();
        assert_eq!(
            typed::<PassiveNodeDefId>(&allocation["node"]["value"]),
            ruby.node
        );
        assert_eq!(
            typed::<SocketSlotDefId>(&destination["slot"]["value"]),
            ruby.slot
        );
        let pool = typed(&allocation["pool"]["value"]);
        assert_eq!(assignments(&item["parameters"]).len(), 6);
        assert_eq!(item["modifiers"]["completion"], json!({"kind":"complete"}));
        assert_eq!(item["modifiers"]["members"].as_array().unwrap().len(), 1);
        assert_eq!(
            assignments(&item["modifiers"]["members"][0]["rolls"]).len(),
            24
        );
        let extension: OwnedRecipeExtension = serde_json::from_str(include_str!(
            "../data/owned/poe2/3887ae68/ordinary-passive-jewel-placement/extension.json"
        ))
        .unwrap();
        for entry in extension.schema {
            let SchemaExtensionEntry::Definition(d) = entry else {
                panic!("socket definition slice")
            };
            assert!(endpoint.input().recipe.schema.definitions.contains(&d));
        }
        Self {
            endpoint,
            item,
            pool,
            ruby,
            other,
            expected,
        }
    }

    fn fixture(&self) -> Fixture {
        let b = fire::bindings();
        let recipe = &self.endpoint.input().recipe;
        let definition = |address: DefinitionAddress| {
            recipe
                .schema
                .definitions
                .iter()
                .find(|d| d.address() == address)
                .unwrap()
                .clone()
        };
        let owner = |address: DefinitionAddress| {
            recipe
                .rules
                .owners
                .iter()
                .find(|o| o.owner == SchemaSubject::Definition(address.clone()))
                .unwrap()
                .clone()
        };
        let fire_owner = owner(b.modifier.address());
        let template = typed::<ItemTemplateDefId>(&self.item["template"]["value"]);
        let template_owner = owner(template.address());
        assert_eq!(fire_owner.programs.members.len(), 4);
        assert_eq!(template_owner.programs.members.len(), 2);
        assert!(matches!(
            fire_owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        assert!(matches!(
            template_owner.programs.closure,
            SchemaClosure::Partial { .. }
        ));
        let slots = |declaration| {
            recipe
                .schema
                .slots
                .iter()
                .filter(|s| s.address().declaration() == &declaration)
                .cloned()
                .collect::<Vec<_>>()
        };
        let fire_slots = slots(SlotOwnerDefId::Modifier(b.modifier.clone()));
        let template_slots = slots(SlotOwnerDefId::ItemTemplate(template.clone()));
        assert_eq!(fire_slots.len(), 24);
        assert_eq!(template_slots.len(), 6);
        let mut dependencies = BTreeMap::new();
        for d in fire::dependency_definitions()
            .into_iter()
            .chain(physical::dependency_definitions())
        {
            assert!(recipe.schema.definitions.contains(&d));
            dependencies.insert(d.address(), d);
        }
        // Explicitly finite topology: these nodes have only the supplied socket
        // and no other effects. This never replaces the published Partial nodes.
        for binding in [&self.ruby, &self.other] {
            let mut declarations = empty_slots();
            declarations.sockets = DeclaredSet::complete(vec![binding.slot.clone()]);
            let node = DefinitionDescriptor::PassiveNode(known(
                binding.node.clone(),
                PassiveNodeSchema {
                    pools: DeclaredSet::complete(vec![self.pool.clone()]),
                    adjacent: DeclaredSet::complete(vec![]),
                    declarations,
                },
            ));
            dependencies.insert(node.address(), node);
            let socket = definition(binding.slot.address());
            let DefinitionDescriptor::SocketSlot(DefinitionEntry {
                schema: SchemaState::Known(schema),
                ..
            }) = &socket
            else {
                panic!("published socket")
            };
            assert_eq!(
                schema.owner,
                SlotOwnerDefId::PassiveNode(binding.node.clone())
            );
            assert_eq!(schema.kind, SocketKind::Passive);
            assert_eq!(schema.scope, ScopePolicy::Shared);
            dependencies.insert(socket.address(), socket);
        }
        // Either permits the isolated Selected-allocation contrast below. Source
        // admission remains Shared-only; no broader source fact is asserted.
        let pool = DefinitionDescriptor::PointPool(known(
            self.pool.clone(),
            PointPoolSchema {
                scope: PointPoolScope::Either,
            },
        ));
        dependencies.insert(pool.address(), pool);
        let mut schema = vec![SchemaExtensionEntry::Definition(definition(
            b.modifier.address(),
        ))];
        schema.extend(fire_slots.into_iter().map(SchemaExtensionEntry::Slot));
        let c = component::categories::bindings();
        let mut f = Fixture::from_compiled_effects_with_template_inputs(
            AuthoredComponent {
                bindings: ComponentBindings {
                    modifier: b.modifier,
                    amount: b.amount,
                    properties: b.properties,
                    corrupted_base: b.corrupted_base,
                    unit: b.unit.clone(),
                    contribution_unit: b.unit,
                    factor_unit: b.factor_unit,
                    effective: b.effective.clone(),
                    contribution: b.effective,
                },
                extension: OwnedRecipeExtension {
                    support_source_domains: vec![],
                    schema_version: 1,
                    version: "finite-passive-receiving".parse().unwrap(),
                    schema,
                    operations_version: Some(recipe.rules.operations_version.clone()),
                    tables: vec![],
                    owners: vec![fire_owner.clone()],
                    receivers: vec![],
                },
                dependencies: dependencies.into_values().collect(),
                numeric_policy: |_| panic!("published compiled programs"),
                category: Some(CategoryBindings {
                    slot: b.category,
                    explicit: c.explicit,
                    implicit: c.implicit,
                    enchant: c.enchant,
                }),
                category_target: None,
                catalyst_property: "fire",
                catalyst_amount: 20.0,
                parameter_count: 24,
                parameters_complete: true,
                last_authored: 0x320e,
                release: "finite-passive-receiving",
            },
            AuthoredTemplateInputs {
                template: template.clone(),
                slots: template_slots,
                owner: template_owner.clone(),
                assignments: assignments(&self.item["parameters"]),
            },
        );
        f.build.items.truncate(1);
        f.build.equipment.truncate(2);
        let item = &mut f.build.items[0];
        item.item_level = Some(55);
        item.modifiers.truncate(1);
        item.modifier_order.truncate(1);
        item.modifiers[0].rolls = assignments(&self.item["modifiers"]["members"][0]["rolls"]);
        for (i, binding) in [&self.ruby, &self.other].into_iter().enumerate() {
            let allocation = occurrence(10 + i as u64);
            f.build.allocations.push(Allocation {
                id: allocation,
                node: binding.node.clone(),
                pool: self.pool.clone(),
                scope: LoadoutScope::Shared,
                access: AllocationAccess::Ordinary,
                choices: vec![],
            });
            f.build.equipment[i].destination = EquipmentDestination::PassiveSocket {
                allocation,
                slot: binding.slot.clone(),
            };
            f.recipe.rules.owners.push(DefinitionRules {
                owner: SchemaSubject::Definition(binding.node.address()),
                programs: DeclaredSet::complete(vec![]),
            });
        }
        let DefinitionDescriptor::ItemTemplate(DefinitionEntry {
            schema: SchemaState::Known(s),
            ..
        }) = f
            .recipe
            .schema
            .definitions
            .iter_mut()
            .find(|d| d.address() == template.address())
            .unwrap()
        else {
            panic!()
        };
        s.socket_destinations =
            DeclaredSet::complete(vec![self.ruby.slot.clone(), self.other.slot.clone()]);
        f.complete_domain();
        assert_eq!(
            f.family_owner_mut().programs.members,
            fire_owner.programs.members
        );
        assert_eq!(
            f.recipe
                .rules
                .owners
                .iter()
                .find(|o| o.owner == template_owner.owner)
                .unwrap()
                .programs
                .members,
            template_owner.programs.members
        );
        f
    }
}

fn report(f: &Fixture, uses: &[u64], expected: f64) -> OwnedEffectsReport {
    let plan = f.plan().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    for &id in uses {
        assert_eq!(f.effective(&report, id, 4), &f.expected(expected));
    }
    assert!(
        !report
            .effects
            .iter()
            .any(|e| matches!(e.target, BoundEffectTarget::Contribution { .. }))
    );
    report
}
fn absent(report: &OwnedEffectsReport, id: u64) {
    assert!(!report.values.iter().any(|v| matches!(&v.key,
        PlanValueKey::Stat { entity: ConcreteEntity::Modifier(ProviderKey {
            root: ProviderRoot::ItemModifier { equipment_use, .. }, .. }), .. }
        if *equipment_use == occurrence(id))));
}

#[test]
#[ignore = "requires verified passive-jewel publication and complete source evidence"]
fn passive_receiving_uses_preserve_occurrences_activation_and_parallel_scratch() {
    let input = Inputs::load();
    let mut f = input.fixture();
    assert_eq!(f.build.equipment[0].item, f.build.equipment[1].item);
    assert_ne!(f.build.equipment[0].id, f.build.equipment[1].id);
    let both = report(&f, &[6, 7], input.expected);
    let first = f.plan().unwrap();
    // The equipment is still Shared. Only its allocation ancestor is inactive.
    f.build.allocations[1].scope = LoadoutScope::Selected {
        loadouts: vec![occurrence(2)],
    };
    let only_first = report(&f, &[6], input.expected);
    absent(&only_first, 7);
    let second = f.plan().unwrap();
    assert_ne!(first.identity(), second.identity());
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let (a, b, before, after) = (&first, &second, &both, &only_first);
            scope.spawn(move || {
                let mut scratch = a.new_scratch();
                for _ in 0..3 {
                    assert_eq!(a.evaluate(&mut scratch).unwrap(), *before);
                    assert_eq!(b.evaluate(&mut scratch).unwrap(), *after);
                    assert_eq!(a.evaluate(&mut scratch).unwrap(), *before);
                }
            });
        }
    });
    f.build.active_weapon_loadout = occurrence(2);
    report(&f, &[6, 7], input.expected);
}

#[test]
#[ignore = "requires verified passive-jewel publication and complete source evidence"]
fn passive_destination_ownership_and_published_partial_coverage_remain_enforced() {
    let input = Inputs::load();
    for wrong_allocation in [true, false] {
        let mut f = input.fixture();
        let EquipmentDestination::PassiveSocket { allocation, slot } =
            &mut f.build.equipment[0].destination
        else {
            panic!()
        };
        if wrong_allocation {
            *allocation = occurrence(11);
        } else {
            *slot = input.other.slot.clone();
        }
        assert!(
            matches!(f.plan(), Err(PlanError::Invalid(message)) if message.contains("invalid schema bindings"))
        );
    }
    let mut missing = input.fixture();
    missing.build.allocations.remove(0);
    assert!(BuildSpec::new(missing.build, OwnedInputLimits::default()).is_err());
    for template in [true, false] {
        let mut f = input.fixture();
        if template {
            f.restore_template_rules();
        } else {
            f.restore_partial_rules();
        }
        let plan = f.plan().unwrap();
        assert!(
            !plan
                .evaluate(&mut plan.new_scratch())
                .unwrap()
                .gaps
                .is_empty()
        );
    }
}

#[test]
#[ignore = "requires verified passive-jewel publication and complete source evidence"]
fn independent_passive_presets_keep_same_node_occurrences_and_receivers_separate() {
    let input = Inputs::load();
    let mut f = input.fixture();
    // Two independent saved Specs can allocate the same definition. Their
    // AllocationIds and receiving uses must not collapse into one global node.
    f.build.allocations[1].node = input.ruby.node.clone();
    f.build.equipment[1].destination = EquipmentDestination::PassiveSocket {
        allocation: occurrence(11),
        slot: input.ruby.slot.clone(),
    };
    let build = &f.build;
    let project = BuildProject::new(
        ProjectInput {
            allocator: build.allocator,
            revision: build.revision,
            game_version: build.game_version.clone(),
            weapon_loadouts: build.weapon_loadouts.clone(),
            items: build.items.clone(),
            gems: vec![],
            rewards: vec![],
            equipment: build.equipment.clone(),
            allocations: build.allocations.clone(),
            skills: vec![],
            supports: vec![],
            payload_links: vec![],
            character_presets: vec![CharacterPreset {
                id: occurrence(60),
                class: build.character.class.clone(),
                ascendancy: None,
                level: 20,
                rewards: vec![],
            }],
            equipment_presets: vec![EquipmentPreset {
                id: occurrence(61),
                equipment: vec![],
            }],
            allocation_presets: vec![
                AllocationPreset {
                    id: occurrence(62),
                    allocations: vec![occurrence(10)],
                    equipment: vec![occurrence(6)],
                },
                AllocationPreset {
                    id: occurrence(63),
                    allocations: vec![occurrence(11)],
                    equipment: vec![occurrence(7)],
                },
            ],
            skill_presets: vec![SkillPreset {
                intent: None,
                usage_preferences: None,
                id: occurrence(64),
                skills: vec![],
                supports: vec![],
                support_origins: None,
                payload_links: vec![],
            }],
            choice_presets: vec![ChoicePreset {
                id: occurrence(65),
                choices: vec![],
                rewards: vec![],
            }],
            saved_variants: vec![],
        },
        OwnedInputLimits::default(),
    )
    .unwrap();
    let selection = |preset| VariantSelection {
        character: occurrence(60),
        equipment: occurrence(61),
        allocations: occurrence(preset),
        skills: occurrence(64),
        choices: occurrence(65),
        active_weapon_loadout: occurrence(1),
    };
    let before = serde_json::to_vec(&project).unwrap();
    for (preset, allocation, equipment, other) in [(62, 10, 6, 7), (63, 11, 7, 6), (62, 10, 6, 7)] {
        f.build = compose(
            &project,
            &selection(preset),
            None,
            OwnedInputLimits::default(),
        )
        .unwrap()
        .input()
        .clone();
        assert_eq!(f.build.allocations.len(), 1);
        assert_eq!(f.build.allocations[0].id, occurrence(allocation));
        assert_eq!(f.build.equipment.len(), 1);
        let observed = report(&f, &[equipment], input.expected);
        absent(&observed, other);
    }
    assert_eq!(serde_json::to_vec(&project).unwrap(), before);
    let mut crossed = project.into_input();
    crossed.allocation_presets[0].equipment = vec![occurrence(7)];
    let crossed = BuildProject::new(crossed, OwnedInputLimits::default()).unwrap();
    assert!(compose(&crossed, &selection(62), None, OwnedInputLimits::default()).is_err());
}
