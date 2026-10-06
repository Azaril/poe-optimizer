//! Published profile/quality/composition programs in an unpublished finite item
//! component. Selected modifier applicability, crafted-quality preparation,
//! per-level/override stages and final Actor defences are outside this boundary.
#[allow(dead_code)]
#[path = "../../crates/poe-optimizer-engine/tests/support/owned_plan_fixture.rs"]
mod base;
use super::{family, release};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_routing::*, owned_rules::*, owned_schema::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting, owned_schema::OwnedDefinitionSchemaPackage,
};
use poe_optimizer_engine::{owned_plan::*, owned_rules::CompiledRulePackage};
use poe_optimizer_import::owned_recipe::OwnedRecipeInput;
use rayon::prelude::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

const PROFILE: &str = "raw-base-defence-profile";
const QUALITY: &str = "declared-standard-item-quality";
const ARMOUR: &str = "pre-override-local-armour";
const SHIELD: &str = "pre-override-local-energy-shield";

fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap()
}
fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(ns(), format!("def.{n:016x}")).unwrap()
}
fn named<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::parse(ns(), format!("fixture.local-defence.{s}")).unwrap()
}
fn decode<T: DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}
fn known(v: &Value) -> &Value {
    assert_eq!(v["kind"], "known");
    &v["value"]
}
fn quantity(n: f64, unit: UnitDefId) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, unit).unwrap())
}
fn entry<I, S>(id: I, schema: S) -> DefinitionEntry<I, S> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn subject<I: SchemaDefinitionId>(id: I) -> SchemaSubject {
    SchemaSubject::Definition(id.address())
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
// Rebind only the existing synthetic topology namespace. Actual published IDs,
// programs and descriptors are copied afterwards without transcoding their bodies.
fn topology<T: Serialize + DeserializeOwned>(v: &T) -> T {
    fn walk(v: &mut Value) {
        match v {
            Value::Object(o)
                if o.get("game").and_then(Value::as_str) == Some("owned-plan-test") =>
            {
                *v = json!({"game":"poe2","version":"owned-mechanics-v1"});
            }
            Value::Object(o) => o.values_mut().for_each(walk),
            Value::Array(a) => a.iter_mut().for_each(walk),
            _ => {}
        }
    }
    let mut v = json!(v);
    walk(&mut v);
    decode(&v)
}
struct Source {
    recipe: OwnedRecipeInput,
    draft: Value,
    bindings: Value,
}
fn source() -> &'static Source {
    static SOURCE: OnceLock<Source> = OnceLock::new();
    SOURCE.get_or_init(|| {
        let path = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_LOCAL_DEFENCE_COMPOSITION_RELEASE")
                .expect("published local defence component release"),
        );
        let inventory = release::inventory(&path);
        let endpoint = release::load(&path);
        family::assert_endpoint(&endpoint);
        let draft: Value = serde_json::from_slice(
            &fs::read(path.parent().unwrap().join("original-05/draft.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(inventory, release::inventory(&path));
        Source {
            recipe: endpoint.input().recipe.clone(),
            draft: draft["draft"].clone(),
            bindings: family::read("bindings.json"),
        }
    })
}

#[derive(Clone)]
struct Channel {
    stat: StatDefId,
    contribution: ContributionKind,
    unit: UnitDefId,
    slot: DeclaredSlot<ParameterSlotDefId>,
}
struct World {
    f: base::Fixture,
    channels: Vec<Channel>,
    templates: Vec<ItemTemplateDefId>,
    actual_owners: Vec<DefinitionRules>,
}
impl World {
    fn new() -> Self {
        let source = source();
        let recipe = &source.recipe;
        let mut f = base::Fixture::new();
        f.schema = topology(&f.schema);
        f.build = topology(&f.build);
        f.scenario = topology(&f.scenario);
        f.queries = topology(&f.queries);
        f.owners = topology(&f.owners);
        for owner in &mut f.owners {
            owner.programs.members.clear();
        }
        f.build.items.clear();
        f.build.equipment.clear();
        let templates: Vec<ItemTemplateDefId> = source.bindings["templates"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| decode(&r["template"]))
            .collect();
        assert_eq!(templates, [def(0x1f1c), def(0x1e0e)]);
        // Leaf descriptors carry actual units and domains. Their empty component
        // owners are not assertions about any selected production provider.
        for d in &recipe.schema.definitions {
            if matches!(
                d,
                DefinitionDescriptor::Unit(_)
                    | DefinitionDescriptor::Stat(_)
                    | DefinitionDescriptor::Option(_)
                    | DefinitionDescriptor::Capability(_)
                    | DefinitionDescriptor::Quality(_)
                    | DefinitionDescriptor::EquipmentSlot(_)
            ) {
                assert!(
                    !f.schema
                        .definitions
                        .iter()
                        .any(|x| x.address() == d.address())
                );
                f.schema.definitions.push(d.clone());
                f.owners.push(DefinitionRules {
                    owner: SchemaSubject::Definition(d.address()),
                    programs: DeclaredSet::complete(vec![]),
                });
            }
        }
        let mut actual_owners = Vec::new();
        for (index, template) in templates.iter().enumerate() {
            let original = recipe
                .schema
                .definitions
                .iter()
                .find(|d| d.address() == template.address())
                .unwrap();
            let mut finite = original.clone();
            let DefinitionDescriptor::ItemTemplate(row) = &mut finite else {
                unreachable!()
            };
            let SchemaState::Known(schema) = &mut row.schema else {
                unreachable!()
            };
            // Only the intrinsic profile and supplied item-quality calculation
            // enter this component. No selected modifier is silently certified.
            schema.modifiers = DeclaredSet::complete(vec![]);
            schema.socket_destinations = DeclaredSet::complete(vec![]);
            let destination = topology(&base::def::<EquipmentSlotDefinition>(if index == 0 {
                "weapon"
            } else {
                "other"
            }));
            schema.equipment_slots = DeclaredSet::complete(vec![destination.clone()]);
            schema.quality.allowed_kinds = DeclaredSet::complete(vec![def(6)]);
            assert_eq!(schema.quality.presence, QualityPresence::Optional);
            let parameters = schema.declarations.parameters.members.clone();
            for slot in &parameters {
                let actual = recipe
                    .schema
                    .slots
                    .iter()
                    .find(|s| s.address() == SlotAddress::Parameter(slot.clone()))
                    .unwrap();
                f.schema.slots.push(actual.clone());
                f.owners.push(DefinitionRules {
                    owner: SchemaSubject::Slot(actual.address()),
                    programs: DeclaredSet::complete(vec![]),
                });
            }
            let mut finite_declarations = empty_slots();
            finite_declarations.parameters.members = parameters;
            schema.declarations = finite_declarations;
            f.schema.definitions.push(finite);
            let actual = recipe
                .rules
                .owners
                .iter()
                .find(|o| o.owner == subject(template.clone()))
                .unwrap();
            assert!(!actual.programs.is_complete());
            actual_owners.push(actual.clone());
            let programs = [PROFILE, QUALITY]
                .into_iter()
                .map(|name| {
                    actual
                        .programs
                        .members
                        .iter()
                        .find(|p| p.id.as_str() == name)
                        .unwrap()
                        .clone()
                })
                .collect();
            f.owners.push(DefinitionRules {
                owner: actual.owner.clone(),
                programs: DeclaredSet::complete(programs),
            });
            let items: Vec<_> = source.draft["items"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|i| i["template"].get("value") == Some(&json!(template)))
                .collect();
            assert_eq!(items.len(), 1);
            let item = items[0];
            let parameters = item["parameters"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| ParameterAssignment {
                    slot: decode(known(&p["slot"])),
                    value: decode(known(&p["value"])),
                })
                .collect();
            let raw_quality = known(&item["quality"]);
            let quality = QualitySelection {
                kind: decode(known(&raw_quality["kind"])),
                amount: decode(known(&raw_quality["amount"])),
            };
            assert_eq!(quality.kind, def(6));
            assert_eq!(quality.amount.value(), 20.0);
            f.build.items.push(ItemRecord {
                id: base::occurrence(3 + index as u64),
                template: template.clone(),
                parameters,
                item_level: decode(known(&item["item_level"])),
                quality: Some(quality),
                modifiers: vec![],
                modifier_order: vec![],
            });
            f.build.equipment.push(EquipmentUse {
                id: base::occurrence(6 + index as u64),
                item: base::occurrence(3 + index as u64),
                destination: EquipmentDestination::CharacterSlot(destination),
                scope: LoadoutScope::Shared,
            });
        }
        let mut channels = BTreeMap::new();
        for name in [ARMOUR, SHIELD] {
            let receiver = recipe
                .rules
                .receivers
                .members
                .iter()
                .find(|r| r.program.as_str() == name)
                .unwrap();
            let owner = recipe
                .rules
                .owners
                .iter()
                .find(|o| o.owner == subject(receiver.stat.clone()))
                .unwrap();
            assert!(owner.programs.is_complete());
            let program = owner
                .programs
                .members
                .iter()
                .find(|p| p.id.as_str() == name)
                .unwrap();
            for read in &program.reads {
                if let RuleReadSource::Contributions {
                    entity,
                    stat,
                    contribution,
                    reduction,
                    empty,
                } = &read.source
                {
                    assert_eq!(*entity, RuleEntity::Current);
                    assert_eq!(*reduction, ContributionReduction::Sum);
                    let ParameterValue::Quantity(empty) = empty else {
                        panic!("quantity contribution channel")
                    };
                    assert_eq!(empty.value(), 0.0);
                    let channel = Channel {
                        stat: stat.clone(),
                        contribution: *contribution,
                        unit: empty.unit().clone(),
                        slot: DeclaredSlot {
                            declaration: SlotOwnerDefId::Modifier(named("groups")),
                            slot: named(
                                &format!("roll-{}-{contribution:?}", stat.key()).to_lowercase(),
                            ),
                        },
                    };
                    if let Some(prior) =
                        channels.insert((stat.clone(), *contribution), channel.clone())
                    {
                        assert_eq!(prior.unit, channel.unit);
                    }
                }
            }
            *f.owner_mut(&owner.owner) = owner.clone();
            f.receivers.members.push(receiver.clone());
        }
        Self {
            f,
            channels: channels.into_values().collect(),
            templates,
            actual_owners,
        }
    }
    fn compile(&self) -> OwnedEffectPlan<OwnedDefinitionSchemaPackage> {
        let schema = Arc::new(
            OwnedDefinitionSchemaPackage::new(self.f.schema.clone(), Default::default()).unwrap(),
        );
        let rules = RulePackageInput {
            existing_actor_rules: None,
            ordered_contributions: None,
            schema_version: OWNED_RULE_PACKAGE_VERSION,
            namespace: ns(),
            release: key("finite-local-defence"),
            semantics_version: key("finite-component"),
            operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
            definitions: schema.identity().clone(),
            owners: self.f.owners.clone(),
            tables: vec![],
            receivers: self.f.receivers.clone(),
            effect_applications: None,
        };
        let rules = Arc::new(
            CompiledRulePackage::compile(&rules, schema.as_ref(), Default::default()).unwrap(),
        );
        let routing = Arc::new(
            OwnedActionRouting::new(
                ActionRoutingInput {
                    schema_version: OWNED_ACTION_ROUTING_VERSION,
                    namespace: ns(),
                    release: key("finite-local-defence"),
                    definitions: schema.identity().clone(),
                    outputs: vec![],
                },
                schema.as_ref(),
                Default::default(),
            )
            .unwrap(),
        );
        OwnedEffectPlan::compile(
            Arc::new(self.f.request()),
            schema,
            rules,
            routing,
            Default::default(),
        )
        .unwrap()
    }
    fn evaluate(&self) -> OwnedEffectsReport {
        let p = self.compile();
        p.evaluate(&mut p.new_scratch()).unwrap()
    }
    fn quality(&mut self, index: usize, value: f64) {
        self.f.build.items[index].quality.as_mut().unwrap().amount =
            FiniteQuantity::new(value, def(2)).unwrap();
    }
    fn install_groups(&mut self) {
        let modifier: ModifierDefId = named("groups");
        let mut declarations = empty_slots();
        let mut program = RuleProgram {
            id: key("test-only-local-groups"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![],
            nodes: vec![],
            effects: vec![],
        };
        for (index, channel) in self.channels.iter().enumerate() {
            declarations.parameters.members.push(channel.slot.clone());
            self.f.schema.slots.push(SlotDescriptor::Parameter(entry(
                channel.slot.clone(),
                ParameterSlotSchema {
                    value: ValueSchema::Quantity(QuantityRange {
                        minimum: FiniteQuantity::new(-1e18, channel.unit.clone()).unwrap(),
                        maximum: FiniteQuantity::new(1e18, channel.unit.clone()).unwrap(),
                    }),
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::ModifierRoll],
                    skill_input: None,
                },
            )));
            let id = key(&format!("group-{index}"));
            program.reads.push(RuleRead {
                id: id.clone(),
                value_type: ComputedValueType::Quantity {
                    unit: channel.unit.clone(),
                },
                source: RuleReadSource::Parameter {
                    slot: channel.slot.clone(),
                },
            });
            program.nodes.push(RuleNode {
                id: id.clone(),
                expression: RuleExpression::Read { input: id.clone() },
            });
            program.effects.push(RuleEffect {
                id: id.clone(),
                when: None,
                effect: RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: channel.stat.clone(),
                    contribution: channel.contribution,
                    value: id,
                },
            });
        }
        self.f
            .schema
            .definitions
            .push(DefinitionDescriptor::Modifier(entry(
                modifier.clone(),
                ModifierSchema { declarations },
            )));
        self.f.owners.push(DefinitionRules {
            owner: subject(modifier.clone()),
            programs: DeclaredSet::complete(vec![program]),
        });
        for (index, item) in self.f.build.items.iter_mut().enumerate() {
            let occurrence = base::occurrence(20 + index as u64);
            item.modifier_order.push(occurrence);
            item.modifiers.push(RolledModifier {
                id: occurrence,
                definition: modifier.clone(),
                rolls: self
                    .channels
                    .iter()
                    .map(|c| ParameterAssignment {
                        slot: c.slot.clone(),
                        value: quantity(0., c.unit.clone()),
                    })
                    .collect(),
            });
        }
        for descriptor in &mut self.f.schema.definitions {
            if let DefinitionDescriptor::ItemTemplate(row) = descriptor
                && self.templates.contains(&row.id)
                && let SchemaState::Known(schema) = &mut row.schema
            {
                schema.modifiers.members.push(modifier.clone());
            }
        }
    }
    fn group(&mut self, item: usize, stat: u64, contribution: ContributionKind, value: f64) {
        let channel = self
            .channels
            .iter()
            .find(|c| c.stat == def(stat) && c.contribution == contribution)
            .unwrap();
        let roll = self.f.build.items[item].modifiers[0]
            .rolls
            .iter_mut()
            .find(|r| r.slot == channel.slot)
            .unwrap();
        roll.value = quantity(value, channel.unit.clone());
    }
}
fn value(report: &OwnedEffectsReport, item: usize, stat: u64) -> Option<&EffectValue> {
    let key = PlanValueKey::Stat {
        entity: ConcreteEntity::EquipmentUse(base::occurrence(6 + item as u64)),
        stat: def(stat),
    };
    report
        .values
        .iter()
        .find(|r| r.key == key)
        .map(|r| &r.value)
}
fn number(report: &OwnedEffectsReport, item: usize, stat: u64) -> f64 {
    let Some(EffectValue::Known {
        value: ParameterValue::Quantity(q),
    }) = value(report, item, stat)
    else {
        panic!(
            "missing numerical EquipmentUse value: {:?}; gaps {:?}",
            value(report, item, stat),
            report.gaps
        )
    };
    q.value()
}
fn finals(report: &OwnedEffectsReport) -> [[f64; 2]; 2] {
    std::array::from_fn(|i| [number(report, i, 0x330c), number(report, i, 0x330d)])
}
fn partial(owner: SchemaSubject) -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: owner,
            facet: SchemaFacet::GameRules,
            code: key("finite-negative-unconverted-contributor"),
        }],
    }
}

#[test]
#[ignore = "requires published LOCAL_DEFENCE_COMPOSITION_RELEASE and preserved Original05 draft"]
fn published_profiles_and_quality_reach_two_exact_equipment_receivers() {
    let mut w = World::new();
    let report = w.evaluate();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert_eq!(finals(&report), [[31., 16.], [161., 44.]]);
    let vectors: Value = family::read("source-vectors.json");
    for (index, template) in w.templates.iter().enumerate() {
        let original: Vec<_> = vectors["native_cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["name"] == "original" && r["template"] == json!(template))
            .collect();
        assert_eq!(original.len(), 1);
        assert_eq!(original[0]["raw_quality"], 20);
        assert_eq!(original[0]["armour"], number(&report, index, 0x330c));
        assert_eq!(original[0]["energy_shield"], number(&report, index, 0x330d));
    }
    let receivers: Vec<_> = report
        .effects
        .iter()
        .filter(|e| {
            matches!(
                e.key.invocation.origin,
                RuleOrigin::EquipmentReceiver { .. }
            )
        })
        .collect();
    assert_eq!(receivers.len(), 4);
    for row in &report.values {
        if let PlanValueKey::Stat { entity, stat } = &row.key
            && [def(0x330c), def(0x330d)].contains(stat)
        {
            assert!(
                matches!(entity, ConcreteEntity::EquipmentUse(_)),
                "never final Actor defence"
            );
        }
    }
    // This is an explicitly authored quality-zero component control. It does
    // not assert that PoB UI loading preserves a raw zero header.
    w.quality(0, 0.);
    w.quality(1, 0.);
    assert_eq!(finals(&w.evaluate()), [[26., 13.], [134., 37.]]);
    w.quality(0, 20.);
    w.quality(1, 20.);
    assert_eq!(w.evaluate(), report);
}

#[test]
#[ignore = "requires published LOCAL_DEFENCE_COMPOSITION_RELEASE and authenticated source vectors"]
fn original_local_call_groups_match_the_published_composition_for_both_item_families() {
    let vectors: Value = family::read("source-vectors.json");
    let rows = vectors["native_cases"].as_array().unwrap();
    assert_eq!(rows.len(), 12);
    let mut identities = BTreeSet::new();
    for row in rows {
        let mut w = World::new();
        w.install_groups();
        let template: ItemTemplateDefId = decode(&row["template"]);
        let item = w.templates.iter().position(|t| *t == template).unwrap();
        assert!(identities.insert((row["case_index"].as_u64().unwrap(), template)));
        w.quality(item, row["raw_quality"].as_f64().unwrap());
        let groups = row["groups"].as_array().unwrap();
        assert_eq!(groups.len(), 13);
        let mut seen = BTreeSet::new();
        for group in groups {
            let stat: StatDefId = decode(&group["stat"]);
            let kind: ContributionKind = decode(&group["contribution"]);
            assert!(
                seen.insert((stat.clone(), kind)),
                "each exact local source group occurs once"
            );
            let channel = w
                .channels
                .iter()
                .find(|c| c.stat == stat && c.contribution == kind)
                .unwrap();
            let slot = channel.slot.clone();
            let unit = channel.unit.clone();
            let roll = w.f.build.items[item].modifiers[0]
                .rolls
                .iter_mut()
                .find(|r| r.slot == slot)
                .unwrap();
            roll.value = quantity(group["value"].as_f64().unwrap(), unit);
        }
        let report = w.evaluate();
        assert!(report.gaps.is_empty(), "{:?}", report.gaps);
        assert_eq!(row["raw_profile"]["Armour"], number(&report, item, 0x29fb));
        assert_eq!(
            row["raw_profile"]["EnergyShield"],
            number(&report, item, 0x29fd)
        );
        assert_eq!(
            row["armour"],
            number(&report, item, 0x330c),
            "{}",
            row["name"]
        );
        assert_eq!(
            row["energy_shield"],
            number(&report, item, 0x330d),
            "{}",
            row["name"]
        );
        // The source helper authenticates the original local calls, group
        // identities and final return independently. Supplying those groups
        // here tests composition only; native modifier delivery stays open.
    }
}

#[test]
#[ignore = "requires published LOCAL_DEFENCE_COMPOSITION_RELEASE"]
fn distinct_group_channels_preserve_units_signed_values_and_separate_quality_scaling() {
    use ContributionKind::{Add, Increase};
    let mut w = World::new();
    w.install_groups();
    assert_eq!(w.channels.len(), 13);
    for (stat, kind, n) in [
        (0x330c, Add, 10.25),
        (0x330d, Add, -1.5),
        (0x330e, Add, 2.5),
        (0x330f, Add, 3.25),
        (0x3310, Add, 4.75),
        (0x330c, Increase, 12.5),
        (0x330d, Increase, -10.),
        (0x330e, Increase, 7.25),
        (0x330f, Increase, -3.5),
        (0x3310, Increase, 2.25),
        (0x3311, Increase, 4.25),
    ] {
        w.group(0, stat, kind, n);
    }
    let report = w.evaluate();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert_eq!(finals(&report), [[61., 22.], [161., 44.]]);
    for item in 0..2 {
        for channel in &w.channels {
            let expected = BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::EquipmentUse(base::occurrence(6 + item as u64)),
                    stat: channel.stat.clone(),
                    kind: channel.contribution,
                },
            };
            let actual: Vec<_> = report
                .effects
                .iter()
                .filter(|e| e.target == expected)
                .collect();
            assert_eq!(
                actual.len(),
                1,
                "one source occurrence per exact group and equipment use"
            );
            let EffectValue::Known {
                value: ParameterValue::Quantity(q),
            } = &actual[0].value
            else {
                panic!("known finite group")
            };
            assert_eq!(q.unit(), &channel.unit);
            assert_eq!(
                actual[0].key.invocation.owner,
                subject(named::<ModifierDefinition>("groups"))
            );
        }
    }
    // Deliberately negative half ties distinguish source floor(x + .5) from
    // symmetric rounding and prevent accidental unsigned clamping.
    let mut signed = World::new();
    signed.install_groups();
    signed.quality(0, 0.);
    signed.group(0, 0x330c, Add, -26.5);
    signed.group(0, 0x330d, Add, -14.5);
    assert_eq!(finals(&signed.evaluate()), [[0., -1.], [161., 44.]]);
}

#[test]
#[ignore = "requires published LOCAL_DEFENCE_COMPOSITION_RELEASE"]
fn authored_group_association_survives_order_sensitive_component_controls() {
    use ContributionKind::{Add, Increase};
    let mut base_order = World::new();
    base_order.install_groups();
    base_order.quality(0, 0.);
    for (stat, n) in [
        (0x330c, 1e17),
        (0x330d, 1e17),
        (0x330e, -1e17),
        (0x3310, -1e17),
        (0x330f, 3.),
    ] {
        base_order.group(0, stat, Add, n);
    }
    // Group flattening loses the raw-profile addition; swapping hybrid order
    // loses the final +3. These are finite arithmetic controls, not legal rolls.
    assert_eq!(finals(&base_order.evaluate()), [[35., 19.], [161., 44.]]);
    let mut increase_order = World::new();
    increase_order.install_groups();
    for (stat, n) in [(0x330d, 1e17), (0x330f, 7.), (0x3310, -1e17)] {
        increase_order.group(0, stat, Increase, n);
    }
    assert_eq!(
        number(&increase_order.evaluate(), 0, 0x330d),
        16.,
        "ES adds Armour+ES before Evasion+ES; sorting the groups would yield17"
    );
}

#[test]
#[ignore = "requires published LOCAL_DEFENCE_COMPOSITION_RELEASE"]
fn alternate_quality_and_unconverted_crafted_quality_respect_the_published_boundary() {
    use ContributionKind::Add;
    let mut w = World::new();
    w.install_groups();
    w.group(0, 0x3312, Add, 1.);
    assert_eq!(finals(&w.evaluate()), [[26., 13.], [161., 44.]]);
    w.group(0, 0x3312, Add, -1.);
    assert_eq!(finals(&w.evaluate()), [[31., 16.], [161., 44.]]);
    for n in [1., -1.] {
        w.group(0, 0x3313, Add, n);
        let report = w.evaluate();
        for stat in [0x330c, 0x330d] {
            assert_eq!(
                value(&report, 0, stat),
                Some(&EffectValue::Inactive),
                "the packet does not invent crafted-quality preparation"
            );
        }
        assert_eq!(
            [number(&report, 1, 0x330c), number(&report, 1, 0x330d)],
            [161., 44.]
        );
    }
    w.group(0, 0x3313, Add, 0.);
    assert_eq!(finals(&w.evaluate()), [[31., 16.], [161., 44.]]);
    let crafted = w
        .channels
        .iter()
        .find(|c| c.stat == def(0x3313))
        .unwrap()
        .slot
        .clone();
    let producer = &mut w
        .f
        .owner_mut(&subject(named::<ModifierDefinition>("groups")))
        .programs
        .members[0];
    let read = producer
        .reads
        .iter_mut()
        .find(|r| {
            matches!(&r.source,
        RuleReadSource::Parameter { slot } if *slot == crafted)
        })
        .unwrap();
    // No final producer exists for the crafted-quality input channel. An
    // unresolved contributing source must not be mistaken for an empty stream.
    read.source = RuleReadSource::Stat {
        entity: RuleEntity::Current,
        stat: def(0x3313),
    };
    let report = w.evaluate();
    for stat in [0x330c, 0x330d] {
        assert!(matches!(
            value(&report, 0, stat),
            Some(EffectValue::Unresolved { .. })
        ));
    }
}

#[test]
#[ignore = "requires published LOCAL_DEFENCE_COMPOSITION_RELEASE"]
fn missing_inputs_and_incomplete_contributors_never_become_zero_or_identity() {
    let mut quality = World::new();
    quality.f.build.items[0].quality = None;
    let report = quality.evaluate();
    assert!(matches!(
        value(&report, 0, 0x2427),
        Some(EffectValue::Unresolved {
            reason: PlanGapReason::MissingInput,
            ..
        })
    ));
    for stat in [0x330c, 0x330d] {
        assert!(matches!(
            value(&report, 0, stat),
            Some(EffectValue::Unresolved { .. })
        ));
    }
    assert_eq!(
        [number(&report, 1, 0x330c), number(&report, 1, 0x330d)],
        [161., 44.]
    );
    let mut raw = World::new();
    raw.f
        .owner_mut(&subject(raw.templates[0].clone()))
        .programs
        .members
        .retain(|p| p.id.as_str() != PROFILE);
    let report = raw.evaluate();
    for stat in [0x330c, 0x330d] {
        assert!(matches!(
            value(&report, 0, stat),
            Some(EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                ..
            })
        ));
    }
    for mode in 0..3 {
        let mut w = World::new();
        w.install_groups();
        let expected = match mode {
            0 => {
                w.f.receivers.closure = partial(subject(def::<StatDefinition>(0x330c)));
                PlanGapReason::PartialReceivers
            }
            1 => {
                let owner = subject(named::<ModifierDefinition>("groups"));
                w.f.owner_mut(&owner).programs.closure = partial(owner.clone());
                PlanGapReason::PartialPrograms
            }
            _ => {
                let actual = &w.actual_owners[0];
                w.f.owner_mut(&actual.owner).programs.closure = actual.programs.closure.clone();
                PlanGapReason::PartialPrograms
            }
        };
        let report = w.evaluate();
        assert!(
            report.gaps.iter().any(|g| g.reason == expected),
            "{:?}",
            report.gaps
        );
        for stat in [0x330c, 0x330d] {
            assert!(
                matches!(
                    value(&report, 0, stat),
                    Some(EffectValue::Unresolved { .. })
                ),
                "Partial incoming inventory cannot use the complete-empty identity"
            );
        }
    }
    let mut missing_receiver = World::new();
    missing_receiver
        .f
        .receivers
        .members
        .retain(|r| r.program.as_str() != ARMOUR);
    let report = missing_receiver.evaluate();
    assert!(value(&report, 0, 0x330c).is_none());
    assert_eq!(number(&report, 0, 0x330d), 16.);
    assert_eq!(finals(&World::new().evaluate()), [[31., 16.], [161., 44.]]);
}

#[test]
#[ignore = "requires published LOCAL_DEFENCE_COMPOSITION_RELEASE"]
fn independent_equipment_uses_and_serial_rayon_replays_are_deterministic() {
    let a = World::new().compile();
    let mut changed = World::new();
    changed.quality(0, 0.);
    let b = changed.compile();
    let mut scratch = a.new_scratch();
    let first = a.evaluate(&mut scratch).unwrap();
    let second = b.evaluate(&mut scratch).unwrap();
    assert_eq!(finals(&second), [[26., 13.], [161., 44.]]);
    assert_ne!(first, second);
    assert_eq!(a.evaluate(&mut scratch).unwrap(), first);
    let serial: Vec<_> = (0..16)
        .map(|i| {
            let plan = if i % 2 == 0 { &a } else { &b };
            plan.evaluate(&mut scratch).unwrap()
        })
        .collect();
    let parallel: Vec<_> = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..16)
                .into_par_iter()
                .map_init(
                    || a.new_scratch(),
                    |scratch, i| {
                        let plan = if i % 2 == 0 { &a } else { &b };
                        plan.evaluate(scratch).unwrap()
                    },
                )
                .collect()
        });
    assert_eq!(parallel, serial);
}
