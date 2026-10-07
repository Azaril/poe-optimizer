//! Published recipient rules, canonical items and source preparation in one plan.
//! Only the selected component inventories and isolated passive adjacency are
//! finite test boundaries. Neither minion receipt nor whole-build closure follows.
#[allow(dead_code)]
#[path = "owned_offering_final_inputs_fixture.rs"]
mod offering;
use offering::{decode, def, id, key, quantity, shared, subject};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_rules::*, owned_schema::*,
    owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use rayon::prelude::*;
use serde_json::Value;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::OnceLock,
};

const RETENTION: &str = "ordinary-amulet-retention-factor";
const APPLICABILITY: &str = "ordinary-item-direct-applicability";
const TALISMAN: &str = "talisman-ordinary-amulet-retention";
const SNAPSHOT: &str = "pre-amulet-bonus-snapshot";
const MYSTIC: &str = "mystic-attunement-amulet-percent";
const DIRECT: &str = "contribute-player-minion-gem-level";
const COPY: &str = "amulet-copy-minion-gem-level";

fn retention() -> StatDefId {
    def("def.0000000000003305")
}
fn applicability() -> StatDefId {
    def("def.0000000000003306")
}
fn snapshot() -> StatDefId {
    def("def.00000000000032e4")
}

#[derive(Clone)]
struct World {
    base: offering::World,
    receivers: DeclaredSet<StatReceiver>,
    early: Vec<(SchemaSubject, OwnedDefinitionKey)>,
    passives: Vec<(Allocation, DefinitionRules)>,
}
impl World {
    fn load() -> Self {
        static WORLD: OnceLock<World> = OnceLock::new();
        WORLD.get_or_init(Self::load_once).clone()
    }
    fn load_once() -> Self {
        let path = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_ORDINARY_ITEM_ROUTING_RELEASE")
                .expect("published ordinary item routing package"),
        );
        Self::load_release(&path)
    }
    fn load_release(path: &Path) -> Self {
        let before = crate::release::inventory(path);
        let endpoint = crate::release::load(path);
        crate::family::assert_component(&endpoint);
        let address = crate::family::expected_modifier_owner().owner;
        let expected = endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == address)
            .unwrap()
            .clone();
        let mut base = offering::World::load_release_with_item_owner(path, Some(&expected));
        let mut early = vec![];
        for stat in [retention(), applicability()] {
            let definition = endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .find(|d| d.address() == stat.address())
                .unwrap()
                .clone();
            offering::add_definition(&mut base.source, definition);
            base.source.base.inner.owner_mut(subject(stat));
        }
        // Remove only the historical fixture boundary; use the actual published
        // snapshot reducer together with the new recipient reducer.
        for owner in &mut base.source.base.inner.owners {
            owner
                .programs
                .members
                .retain(|p| p.id != key(offering::SNAPSHOT));
        }
        base.item_programs
            .retain(|(_, p)| p != &key(offering::SNAPSHOT));
        let mut receivers = vec![];
        for stat in [retention(), snapshot()] {
            let owner = endpoint
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .find(|o| o.owner == subject(stat.clone()))
                .unwrap()
                .clone();
            assert!(owner.programs.is_complete());
            early.extend(
                owner
                    .programs
                    .members
                    .iter()
                    .map(|p| (owner.owner.clone(), p.id.clone())),
            );
            let address = owner.owner.clone();
            *base.source.base.inner.owner_mut(address) = owner;
            let found: Vec<_> = endpoint
                .input()
                .recipe
                .rules
                .receivers
                .members
                .iter()
                .filter(|r| r.stat == stat)
                .cloned()
                .collect();
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].targets, vec![StatReceiverTarget::Player]);
            receivers.extend(found);
        }
        for template in base
            .source
            .base
            .inner
            .build
            .items
            .iter()
            .map(|i| i.template.clone())
            .collect::<Vec<_>>()
        {
            let owner = endpoint
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .find(|o| o.owner == subject(template.clone()))
                .unwrap();
            let programs: Vec<_> = owner
                .programs
                .members
                .iter()
                .filter(|p| p.id == key(APPLICABILITY))
                .cloned()
                .collect();
            assert_eq!(programs.len(), 1);
            let target = base.source.base.inner.owner_mut(owner.owner.clone());
            assert!(
                !target
                    .programs
                    .members
                    .iter()
                    .any(|p| p.id == key(APPLICABILITY))
            );
            target.programs.members.extend(programs);
            base.item_programs
                .push((owner.owner.clone(), key(APPLICABILITY)));
        }
        let passives = [
            ("def.0000000000001338", TALISMAN, 6200),
            ("def.0000000000001b09", MYSTIC, 6201),
        ]
        .into_iter()
        .map(|(node, program, occurrence)| {
            install_passive(&mut base, &endpoint, node, program, occurrence, &mut early)
        })
        .collect();
        assert_eq!(before, crate::release::inventory(path));
        Self {
            base,
            receivers: DeclaredSet::complete(receivers),
            early,
            passives,
        }
    }
    fn select(&mut self, talisman: bool, mystic: bool) {
        self.base.source.base.inner.build.allocations = self
            .passives
            .iter()
            .zip([talisman, mystic])
            .filter(|(_, selected)| *selected)
            .map(|((allocation, _), _)| allocation.clone())
            .collect();
    }
    fn checked_plan(
        &self,
        late_retention: bool,
        early_direct: bool,
    ) -> std::result::Result<shared::Plan, String> {
        self.base.checked_plan_configured(
            OWNED_EVALUATION_STAGES_V4,
            false,
            self.receivers.clone(),
            |stages| {
                configure_item_stages(
                    stages,
                    &self.base.source.base.inner.owners,
                    &self.early,
                    &[
                        "source-prepare",
                        "source-census",
                        "source-assembly",
                        "facts",
                        "apply",
                        "deliver",
                    ],
                    late_retention,
                    early_direct,
                )
            },
        )
    }
    fn plan(&self) -> shared::Plan {
        self.checked_plan(false, false).unwrap()
    }
    fn evaluate(&self) -> SupportEffectsReport {
        let plan = self.plan();
        plan.evaluate(&mut plan.new_scratch()).unwrap()
    }
    fn check(
        &self,
        report: &SupportEffectsReport,
        talisman: bool,
        mystic: bool,
        levels: [i64; 2],
        qualities: [f64; 2],
    ) {
        offering::check(&self.base, report, levels, qualities);
        let effects = offering::effects(report);
        for (stat, value) in [
            (
                retention(),
                quantity(
                    if talisman { 0.0 } else { 1.0 },
                    &def("def.0000000000000001"),
                ),
            ),
            (
                snapshot(),
                quantity(
                    if mystic { 25.0 } else { 0.0 },
                    &def("def.0000000000000002"),
                ),
            ),
        ] {
            let target = PlanValueKey::Stat {
                entity: ConcreteEntity::Actor(ActorKey::Player),
                stat,
            };
            let found: Vec<_> = effects.values.iter().filter(|v| v.key == target).collect();
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].value, EffectValue::Known { value });
        }
        for equipment in &self.base.source.base.inner.build.equipment {
            let item = self
                .base
                .source
                .base
                .inner
                .build
                .items
                .iter()
                .find(|i| i.id == equipment.item)
                .unwrap();
            let amulet = item.template == decode(&self.base.bindings["ordinary_items"]["amulet"]);
            let target = PlanValueKey::Stat {
                entity: ConcreteEntity::EquipmentUse(equipment.id),
                stat: applicability(),
            };
            let found: Vec<_> = effects.values.iter().filter(|v| v.key == target).collect();
            assert_eq!(found.len(), 1);
            assert_eq!(
                found[0].value,
                EffectValue::Known {
                    value: ParameterValue::Boolean(!amulet || !talisman)
                }
            );
            let direct: Vec<_> = effects.effects.iter().filter(|e| e.key.invocation.program == key(DIRECT)
                && matches!(&e.key.invocation.origin, RuleOrigin::Provider { provider }
                    if provider.root == ProviderRoot::ItemModifier { equipment_use: equipment.id, modifier: item.modifiers[0].id })).collect();
            assert_eq!(direct.len(), 1);
            if amulet && talisman {
                assert_eq!(direct[0].value, EffectValue::Inactive);
            } else {
                assert!(matches!(direct[0].value, EffectValue::Known { .. }));
            }
        }
        for (index, enabled) in [talisman, mystic].into_iter().enumerate() {
            let (allocation, owner) = &self.passives[index];
            let rows: Vec<_> = effects
                .effects
                .iter()
                .filter(|e| e.key.invocation.program == owner.programs.members[0].id)
                .collect();
            assert_eq!(rows.len(), usize::from(enabled));
            if enabled {
                assert!(
                    matches!(&rows[0].key.invocation.origin, RuleOrigin::Provider { provider }
                if provider.root == ProviderRoot::Allocation(allocation.id) && provider.grant_path.is_empty())
                );
            }
        }
        assert!(
            effects
                .effects
                .iter()
                .all(|e| e.key.invocation.program != key(offering::SNAPSHOT))
        );
    }
}

/// The existing finite item's checked component, without Offering, supports,
/// passive controls or synthetic snapshot/applicability producers. This is test
/// metadata, never a production coverage declaration.
#[derive(Clone)]
#[allow(dead_code)]
pub struct ItemDonors {
    pub definitions: Vec<DefinitionDescriptor>,
    pub slots: Vec<SlotDescriptor>,
    pub owners: Vec<DefinitionRules>,
    pub receivers: DeclaredSet<StatReceiver>,
    pub items: Vec<ItemRecord>,
    pub equipment: Vec<EquipmentUse>,
    pub actual_modifier: DefinitionRules,
    early: Vec<(SchemaSubject, OwnedDefinitionKey)>,
}

#[allow(dead_code)]
pub fn item_donors(path: &Path) -> ItemDonors {
    let w = World::load_release(path);
    let inner = &w.base.source.base.inner;
    assert!(inner.build.allocations.is_empty());
    let mut selected = vec![];
    for owner in inner.build.items.iter().flat_map(|item| {
        std::iter::once(subject(item.template.clone()))
            .chain(item.modifiers.iter().map(|m| subject(m.definition.clone())))
    }) {
        if !selected.contains(&owner) {
            selected.push(owner);
        }
    }
    selected.extend([subject(retention()), subject(snapshot())]);
    let owners: Vec<_> = inner
        .owners
        .iter()
        .filter(|o| selected.contains(&o.owner))
        .cloned()
        .collect();
    assert_eq!(owners.len(), selected.len());
    assert!(owners.iter().all(|o| o.programs.is_complete()));
    assert!(
        owners
            .iter()
            .flat_map(|o| &o.programs.members)
            .all(|p| p.id != key(offering::SNAPSHOT) && p.id != key(offering::DIRECT_BOUNDARY))
    );

    // Traverse references in already typed fixture records, then copy exact
    // descriptors. No interpreted game logic or inferred default is involved.
    // The namespace/kind/key triple prevents a bare key from aliasing a domain.
    fn references(value: &Value, found: &mut BTreeSet<String>) {
        match value {
            Value::Object(fields) => {
                if fields.contains_key("namespace")
                    && fields.contains_key("kind")
                    && fields.contains_key("key")
                {
                    assert_eq!(fields.len(), 3);
                    found.insert(serde_json::to_string(value).unwrap());
                }
                for value in fields.values() {
                    references(value, found);
                }
            }
            Value::Array(values) => {
                for value in values {
                    references(value, found);
                }
            }
            _ => {}
        }
    }
    let mut needed = BTreeSet::new();
    references(
        &serde_json::json!((
            &owners,
            &w.receivers,
            &inner.build.items,
            &inner.build.equipment
        )),
        &mut needed,
    );
    let mut definitions = vec![];
    let mut slots = vec![];
    loop {
        let before = needed.len();
        for d in &inner.schema.definitions {
            let id = serde_json::to_value(d.address()).unwrap()["value"].clone();
            if needed.contains(&serde_json::to_string(&id).unwrap())
                && !definitions
                    .iter()
                    .any(|old: &DefinitionDescriptor| old.address() == d.address())
            {
                references(&serde_json::to_value(d).unwrap(), &mut needed);
                definitions.push(d.clone());
            }
        }
        for s in &inner.schema.slots {
            let id = serde_json::to_value(s.address()).unwrap()["value"]["slot"].clone();
            if needed.contains(&serde_json::to_string(&id).unwrap())
                && !slots
                    .iter()
                    .any(|old: &SlotDescriptor| old.address() == s.address())
            {
                references(&serde_json::to_value(s).unwrap(), &mut needed);
                slots.push(s.clone());
            }
        }
        if needed.len() == before {
            break;
        }
    }
    assert!(definitions.iter().all(|d| matches!(
        d,
        DefinitionDescriptor::Unit(_)
            | DefinitionDescriptor::Option(_)
            | DefinitionDescriptor::Stat(_)
            | DefinitionDescriptor::Modifier(_)
            | DefinitionDescriptor::ItemTemplate(_)
            | DefinitionDescriptor::EquipmentSlot(_)
    )));
    assert!(
        slots
            .iter()
            .all(|s| matches!(s, SlotDescriptor::Parameter(_)))
    );
    let early = w
        .early
        .into_iter()
        .filter(|(owner, _)| selected.contains(owner))
        .collect();
    ItemDonors {
        definitions,
        slots,
        owners,
        receivers: w.receivers,
        items: inner.build.items.clone(),
        equipment: inner.build.equipment.clone(),
        actual_modifier: w.base.actual_modifier,
        early,
    }
}

#[allow(dead_code)]
impl ItemDonors {
    pub fn configure_stages(&self, stages: &mut EvaluationStagesInput, tail: &[&str]) {
        stages.schema_version = OWNED_EVALUATION_STAGES_V4;
        configure_item_stages(stages, &self.owners, &self.early, tail, false, false);
    }
}

fn configure_item_stages(
    stages: &mut EvaluationStagesInput,
    owners: &[DefinitionRules],
    early: &[(SchemaSubject, OwnedDefinitionKey)],
    tail: &[&str],
    late_retention: bool,
    early_direct: bool,
) {
    let names: Vec<_> = [
        "prepare",
        "routing-contributors",
        "routing-resolve",
        "routing-applicability",
        "item-delivery",
    ]
    .into_iter()
    .chain(tail.iter().copied())
    .collect();
    stages.stages = names
        .iter()
        .enumerate()
        .map(|(i, name)| EvaluationStage {
            id: key(name),
            predecessors: i
                .checked_sub(1)
                .map(|n| vec![key(names[n])])
                .unwrap_or_default(),
        })
        .collect();
    for row in &mut stages.programs.members {
        row.stage = match row.program.as_str() {
            TALISMAN | MYSTIC => key(if late_retention && row.program == key(TALISMAN) {
                "item-delivery"
            } else {
                "routing-contributors"
            }),
            RETENTION | SNAPSHOT => key("routing-resolve"),
            APPLICABILITY => key("routing-applicability"),
            DIRECT => key(if early_direct {
                "prepare"
            } else {
                "item-delivery"
            }),
            COPY => key("item-delivery"),
            _ => row.stage.clone(),
        };
    }
    for row in &mut stages.readiness.as_mut().unwrap().programs.members {
        if early.contains(&(row.owner.clone(), row.program.clone())) {
            let program = owners
                .iter()
                .find(|o| o.owner == row.owner)
                .unwrap()
                .programs
                .members
                .iter()
                .find(|p| p.id == row.program)
                .unwrap();
            row.phase = ReadinessPhase::Structural;
            row.role = ReadinessProgramRole::PreparationFacts;
            row.outputs = program
                .effects
                .iter()
                .map(|e| offering::channel(program.context, &e.effect))
                .collect();
        }
    }
    for row in &mut stages.frozen_channels {
        if matches!(&row.channel, StageChannel::Contributions { stat, .. } if *stat == def::<StatDefinition>("def.00000000000030ab"))
        {
            row.stage = key("item-delivery");
        }
    }
    for (stat, kind) in [
        (retention(), ContributionKind::Multiply),
        (snapshot(), ContributionKind::Add),
    ] {
        stages.frozen_channels.extend([
            FrozenStageChannel {
                channel: StageChannel::Contributions {
                    scope: RuleEntityKind::Actor,
                    stat: stat.clone(),
                    contribution: kind,
                },
                stage: key("routing-contributors"),
            },
            FrozenStageChannel {
                channel: StageChannel::Stat {
                    scope: RuleEntityKind::Actor,
                    stat,
                },
                stage: key("routing-resolve"),
            },
        ]);
    }
    stages.frozen_channels.push(FrozenStageChannel {
        channel: StageChannel::Stat {
            scope: RuleEntityKind::EquipmentUse,
            stat: applicability(),
        },
        stage: key("routing-applicability"),
    });
}

fn install_passive(
    base: &mut offering::World,
    endpoint: &StagedOwnedRelease,
    node: &str,
    program: &str,
    occurrence: u64,
    early: &mut Vec<(SchemaSubject, OwnedDefinitionKey)>,
) -> (Allocation, DefinitionRules) {
    let passive: PassiveNodeDefId = def(node);
    let actual = endpoint
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == subject(passive.clone()))
        .unwrap()
        .clone();
    assert!(!actual.programs.is_complete());
    assert_eq!(actual.programs.members.len(), 1);
    assert_eq!(actual.programs.members[0].id, key(program));
    let mut definition: DefinitionDescriptor = offering::prolonged::finite(
        endpoint
            .input()
            .recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == passive.address())
            .unwrap(),
    );
    let DefinitionDescriptor::PassiveNode(DefinitionEntry {
        schema: SchemaState::Known(schema),
        ..
    }) = &mut definition
    else {
        panic!()
    };
    assert_eq!(schema.pools.members.len(), 1);
    let pool = schema.pools.members[0].clone();
    schema.adjacent = DeclaredSet::complete(vec![]); // finite numerical component, not full-tree legality
    offering::add_definition(&mut base.source, definition);
    offering::add_definition(
        &mut base.source,
        endpoint
            .input()
            .recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == pool.address())
            .unwrap()
            .clone(),
    );
    base.source.base.inner.owner_mut(subject(pool.clone()));
    *base.source.base.inner.owner_mut(actual.owner.clone()) = offering::prolonged::finite(&actual);
    early.push((actual.owner.clone(), key(program)));
    (
        Allocation {
            id: id(occurrence),
            node: passive,
            pool,
            scope: LoadoutScope::Shared,
            access: AllocationAccess::Ordinary,
            choices: vec![],
        },
        actual,
    )
}

fn no_final_level(report: &SupportEffectsReport, reason: PlanGapReason) {
    match &report.outcome {
        SupportEffectsOutcome::Unavailable { cause, .. } => assert!(
            matches!(cause, EffectValue::Unresolved { reason: actual, .. } if *actual == reason),
            "{cause:?}"
        ),
        SupportEffectsOutcome::Evaluated { effects } => {
            assert!(effects.effects.iter().any(|e| matches!(&e.value, EffectValue::Unresolved { reason: actual, .. } if *actual == reason)), "{effects:?}");
            assert!(!effects.effects.iter().any(|e| e.key.invocation.program
                == key("pain-offering-final-inputs")
                && e.key.effect == key("project-final-level")
                && matches!(e.value, EffectValue::Known { .. })));
        }
        other => panic!("unexpected refusal {other:?}"),
    }
}

#[test]
#[ignore = "requires published ORDINARY_ITEM_ROUTING_RELEASE; finite native component"]
fn ordinary_routing_keeps_original_items_and_removes_only_diverted_direct_contribution() {
    let mut w = World::load();
    w.check(&w.evaluate(), false, false, [22, 22], [0.0, 0.0]);
    w.select(true, false);
    w.check(&w.evaluate(), true, false, [21, 21], [0.0, 0.0]);
    w.select(false, false);
    w.check(&w.evaluate(), false, false, [22, 22], [0.0, 0.0]);
    w.select(true, false);
    w.base.remove_item("amulet");
    w.check(&w.evaluate(), true, false, [21, 21], [0.0, 0.0]);
    w.base.remove_item("helmet");
    w.check(&w.evaluate(), true, false, [20, 20], [0.0, 0.0]);
}

#[test]
#[ignore = "requires published ORDINARY_ITEM_ROUTING_RELEASE; finite native component"]
fn ordinary_routing_preserves_independent_copy_and_repeated_source_inputs() {
    let mut w = World::load();
    w.base.item_amount("amulet", 5.0);
    w.select(true, true);
    let report = w.evaluate();
    w.check(&report, true, true, [22, 22], [0.0, 0.0]);
    let copies: Vec<_> = offering::effects(&report)
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(COPY) && e.value != EffectValue::Inactive)
        .collect();
    assert_eq!(copies.len(), 1);
    assert_eq!(
        copies[0].value,
        EffectValue::Known {
            value: quantity(1.0, &def("def.000000000000295a"))
        }
    );
    w.select(false, true);
    w.check(&w.evaluate(), false, true, [27, 27], [0.0, 0.0]);
    w.base.raw(3, 7, 13.0);
    w.select(true, true);
    w.check(&w.evaluate(), true, true, [9, 22], [13.0, 0.0]);
    w.select(true, false);
    w.check(&w.evaluate(), true, false, [8, 21], [13.0, 0.0]);
}

#[test]
#[ignore = "requires published ORDINARY_ITEM_ROUTING_RELEASE; finite native component"]
fn ordinary_routing_requires_actual_receiver_and_template_producer() {
    let mut missing = World::load();
    missing.receivers.members.retain(|r| r.stat != retention());
    no_final_level(&missing.evaluate(), PlanGapReason::MissingProducer);
    // A proved non-Amulet branch needs no Amulet retention value.
    missing.base.remove_item("amulet");
    let report = missing.evaluate();
    offering::check(&missing.base, &report, [21, 21], [0.0, 0.0]);
    let mut missing = World::load();
    for owner in &mut missing.base.source.base.inner.owners {
        owner
            .programs
            .members
            .retain(|p| p.id != key(APPLICABILITY));
    }
    missing
        .base
        .item_programs
        .retain(|(_, p)| p != &key(APPLICABILITY));
    no_final_level(&missing.evaluate(), PlanGapReason::MissingProducer);
}

#[test]
#[ignore = "requires published ORDINARY_ITEM_ROUTING_RELEASE; finite native component"]
fn ordinary_routing_preserves_partial_inventory_and_actual_owner_refusals() {
    let mut partial = World::load();
    partial.receivers.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: subject(retention()),
            facet: SchemaFacet::GameRules,
            code: key("fixture-unreviewed-routing-contributor"),
        }],
    };
    no_final_level(&partial.evaluate(), PlanGapReason::IncompleteContributors);
    let mut partial = World::load();
    partial.select(true, false);
    let owner = partial.passives[0].1.clone();
    let address = owner.owner.clone();
    *partial.base.source.base.inner.owner_mut(address) = owner;
    let error = partial
        .checked_plan(false, false)
        .err()
        .expect("actual Talisman owner stays Partial");
    assert!(error.contains("complete owner programs"), "{error}");
    let mut partial = World::load();
    let owner = partial.base.actual_modifier.clone();
    let address = owner.owner.clone();
    *partial.base.source.base.inner.owner_mut(address) = owner;
    let error = partial
        .checked_plan(false, false)
        .err()
        .expect("actual modifier owner stays Partial");
    assert!(error.contains("complete owner programs"), "{error}");
}

#[test]
#[ignore = "requires published ORDINARY_ITEM_ROUTING_RELEASE; finite native component"]
fn ordinary_routing_rejects_late_routing_and_early_direct_reads() {
    let w = World::load();
    let error = w
        .checked_plan(true, false)
        .err()
        .expect("potential late routing writer");
    assert!(
        error.contains("potential writer occurs after or outside frozen stage"),
        "{error}"
    );
    let error = w
        .checked_plan(false, true)
        .err()
        .expect("direct contribution cannot precede applicability");
    assert!(
        error.contains("frozen channel read occurs before or outside frozen stage"),
        "{error}"
    );
}

#[test]
#[ignore = "requires published ORDINARY_ITEM_ROUTING_RELEASE; finite native component"]
fn ordinary_routing_replays_a_b_a_and_private_rayon_scratch() {
    let a = World::load();
    let mut b = a.clone();
    b.select(true, true);
    b.base.item_amount("amulet", 5.0);
    b.base.raw(3, 7, 13.0);
    let ap = a.plan();
    let bp = b.plan();
    let mut scratch = ap.new_scratch();
    let first = ap.evaluate(&mut scratch).unwrap();
    a.check(&first, false, false, [22, 22], [0.0, 0.0]);
    let second = bp.evaluate(&mut scratch).unwrap();
    b.check(&second, true, true, [9, 22], [13.0, 0.0]);
    assert_eq!(first, ap.evaluate(&mut scratch).unwrap());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports: Vec<_> = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map(|_| bp.evaluate(&mut bp.new_scratch()).unwrap())
            .collect()
    });
    assert!(reports.iter().all(|r| r == &second));
}
