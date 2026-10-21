//! Finite joined Spirit proof: actual imported Amulet rolls and selected rewards.
//! Completing this test graph does not complete any production supplier inventory.
use super::*;
use poe_optimizer_core::{
    owned_draft::{ItemDraft, RewardDraft},
    owned_stages::{EvaluationStage, FrozenStageChannel, StageChannel},
};
use std::collections::BTreeSet;
const FINAL: &str = "fixture.player-spirit-final";
const READY: &str = "fixture.player-spirit-ready";
const CONTROL: &str = "test-player-spirit-inputs";
const DIRECT: &str = "contribute-player-ranged-spirit";
const COPY: &str = "amulet-copy-ranged-spirit";
const POOL: &str = "resolve-player-spirit";
fn inner(w: &mut World) -> &mut shared::World {
    &mut w.sniper.base.source.base.inner
}
fn referenced(v: &Value, keys: &mut BTreeSet<String>) {
    match v {
        Value::Object(m) => {
            if let Some(k) = m.get("key").and_then(Value::as_str) {
                keys.insert(k.to_owned());
            }
            for v in m.values() {
                referenced(v, keys);
            }
        }
        Value::Array(a) => {
            for v in a {
                referenced(v, keys);
            }
        }
        _ => {}
    }
}
fn one<'a>(rows: &'a [Value], id: &Value) -> &'a Value {
    let a: Vec<_> = rows.iter().filter(|x| x["id"] == *id).collect();
    assert_eq!(a.len(), 1);
    a[0]
}
fn selected_sources() -> (RolledModifier, Vec<RewardSelection>) {
    let (draft, selection) = imported_selection(&path());
    let draft = &draft["draft"];
    assert_eq!(draft["items"]["completion"]["kind"], "pending");
    assert_eq!(draft["rewards"]["completion"]["kind"], "pending");
    let preset = one(
        draft["equipment_presets"]["members"].as_array().unwrap(),
        &selection["build"]["equipment"],
    );
    let item_ids: Vec<_> = preset["equipment"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| {
            one(draft["equipment"]["members"].as_array().unwrap(), id)["item"]["value"].clone()
        })
        .collect();
    let items: Vec<_> = draft["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| item_ids.contains(&i["id"]))
        .map(decode::<ItemDraft>)
        .map(|i| i.to_resolved().expect("selected item input resolves"))
        .filter(|i| i.template == d::<ItemTemplateDefinition>(0x2343))
        .collect();
    assert_eq!(items.len(), 1);
    let item = &items[0];
    let mods: Vec<_> = item
        .modifiers
        .iter()
        .filter(|m| m.definition == d(0x314d))
        .collect();
    assert_eq!(mods.len(), 1);
    let mut modifier = mods[0].clone();
    assert!(item.modifier_orders.contributions.contains(&modifier.id));
    assert!(
        item.modifier_orders
            .numeric_transforms
            .contains(&modifier.id)
    );
    assert_eq!(modifier.rolls.len(), 24);
    assert_eq!(
        modifier
            .rolls
            .iter()
            .find(|r| r.slot.slot == d(0x314e))
            .unwrap()
            .value,
        quantity(12.5, &d(0x295a))
    );
    // The common finite fixture has its own lineage. Rebind this exact occurrence
    // once, retaining all authored rolls and its existing physical Amulet/use.
    modifier.id = id(7981);
    let character = one(
        draft["character_presets"]["members"].as_array().unwrap(),
        &selection["build"]["character"],
    );
    let choices = one(
        draft["choice_presets"]["members"].as_array().unwrap(),
        &selection["build"]["choices"],
    );
    let mut ids = character["rewards"]["members"].as_array().unwrap().clone();
    for v in choices["rewards"]["members"].as_array().unwrap() {
        if !ids.contains(v) {
            ids.push(v.clone());
        }
    }
    let mut rewards: Vec<_> = ids
        .iter()
        .map(|id| decode::<RewardDraft>(one(draft["rewards"]["members"].as_array().unwrap(), id)))
        .map(|r| r.to_resolved().expect("selected reward input resolves"))
        .filter(|r| [d::<RewardDefinition>(0x28), d(0x30), d(0x63)].contains(&r.definition))
        .collect();
    assert_eq!(rewards.len(), 3);
    for (i, r) in rewards.iter_mut().enumerate() {
        r.id = id(7982 + i as u64);
    }
    (modifier, rewards)
}
fn world() -> World {
    static WORLD: OnceLock<World> = OnceLock::new();

    WORLD.get_or_init(build_world).clone()
}
fn build_world() -> World {
    let endpoint = release::load(&path());
    player_spirit_pool_family::assert_component(&endpoint);
    let recipe = &endpoint.input().recipe;
    let c = player_spirit_pool_family::consumer();
    let deps = player_spirit_pool_family::dependencies();

    let (modifier, rewards) = selected_sources();
    let mut w = World::load();

    let mut owners = c.owners.clone();
    owners.extend(deps.donor_owners.clone());
    for o in &mut owners {
        o.programs.closure = SchemaClosure::Complete;
    }
    let f = inner(&mut w);
    let amulet = f
        .build
        .items
        .iter_mut()
        .find(|i| i.template == d(0x2343))
        .unwrap();
    let item_id = amulet.id;
    let template = amulet.template.clone();
    assert!(
        !amulet
            .modifiers
            .iter()
            .any(|m| m.definition == modifier.definition)
    );
    amulet.modifier_orders.contributions.push(modifier.id);
    amulet.modifier_orders.numeric_transforms.push(modifier.id);
    amulet.modifiers.push(modifier.clone());

    let uses: Vec<_> = f
        .build
        .equipment
        .iter()
        .filter(|u| u.item == item_id)
        .collect();
    assert_eq!(uses.len(), 1);
    assert_eq!(
        uses[0].destination,
        EquipmentDestination::CharacterSlot(d(0x6a))
    );

    let DefinitionDescriptor::ItemTemplate(DefinitionEntry {
        schema: SchemaState::Known(t),
        ..
    }) = f
        .schema
        .definitions
        .iter_mut()
        .find(|r| r.address() == template.address())
        .unwrap()
    else {
        panic!()
    };
    assert!(t.modifiers.is_complete());
    assert!(!t.modifiers.members.contains(&d(0x314d)));
    t.modifiers.members.push(d(0x314d));

    let mut keys = BTreeSet::new();
    referenced(&serde_json::to_value(&c.queries).unwrap(), &mut keys);
    referenced(&serde_json::to_value(&c.receivers).unwrap(), &mut keys);
    for o in &owners {
        referenced(&serde_json::to_value(o).unwrap(), &mut keys);
    }
    referenced(&serde_json::to_value(&modifier).unwrap(), &mut keys);
    for r in &rewards {
        referenced(&serde_json::to_value(r).unwrap(), &mut keys);
    }
    let mut done = BTreeSet::new();
    while let Some(k) = keys.iter().find(|k| !done.contains(*k)).cloned() {
        done.insert(k.clone());
        if let Some(actual) = recipe
            .schema
            .definitions
            .iter()
            .find(|r| r.address().key().as_str() == k)
        {
            if !f
                .schema
                .definitions
                .iter()
                .any(|o| o.address() == actual.address())
            {
                let actual = sniper::offering::prolonged::finite(actual);
                referenced(&serde_json::to_value(&actual).unwrap(), &mut keys);
                f.schema.definitions.push(actual);
            }
            f.owner_mut(SchemaSubject::Definition(actual.address()));
        }
        for s in recipe.schema.slots.iter().filter(
            |s| matches!(s.address(),SlotAddress::Parameter(ref p) if p.slot.key().as_str()==k),
        ) {
            if !f
                .schema
                .slots
                .iter()
                .any(|old| old.address() == s.address())
            {
                let s = sniper::offering::prolonged::finite(s);
                referenced(&serde_json::to_value(&s).unwrap(), &mut keys);
                f.schema.slots.push(s);
            }
        }
    }
    for o in owners {
        let existing = f.owner_mut(o.owner.clone());
        assert!(existing.programs.members.is_empty());
        *existing = o;
    }
    f.build.character.rewards.extend(rewards);

    for p in &c.owners[1].programs.members {
        if ![DIRECT, COPY].contains(&p.id.as_str()) {
            w.sniper
                .base
                .item_programs
                .push((c.owners[1].owner.clone(), p.id.clone()));
        }
    }
    w.sniper.receivers.members.extend(c.receivers);

    for mut q in c.queries {
        assert!(!q.groups[0].members.is_complete());
        q.groups[0].members.closure = SchemaClosure::Complete;
        assert!(
            !w.sniper
                .base
                .contribution_queries
                .members
                .iter()
                .any(|v| v.id == q.id)
        );
        w.sniper.base.contribution_queries.members.push(q);
    }
    w
}
fn configure(stages: &mut EvaluationStagesInput) {
    stages.stages.extend([
        EvaluationStage {
            id: key(READY),
            predecessors: vec![key("deliver")],
        },
        EvaluationStage {
            id: key(FINAL),
            predecessors: vec![key(READY)],
        },
    ]);
    for row in &mut stages.programs.members {
        if row.program == key(POOL) {
            row.stage = key(FINAL);
        } else if [DIRECT, COPY, CONTROL].contains(&row.program.as_str())
            || row.program == key("flat-resource-contribution")
        {
            row.stage = key("deliver");
        }
    }
    for row in &mut stages.readiness.as_mut().unwrap().programs.members {
        if [POOL, DIRECT, COPY, CONTROL, "flat-resource-contribution"]
            .contains(&row.program.as_str())
        {
            row.phase = ReadinessPhase::Execution;
            row.role = ReadinessProgramRole::Execution;
            row.outputs.clear();
        }
    }
    for q in player_spirit_pool_family::consumer().queries {
        stages.frozen_channels.push(FrozenStageChannel {
            channel: StageChannel::Contributions {
                scope: RuleEntityKind::Actor,
                stat: q.stat,
                contribution: q.contribution,
            },
            stage: key(READY),
        });
    }
}
fn plan(w: &World) -> shared::Plan {
    w.checked_plan_configured(configure).unwrap()
}
fn report(w: &World) -> SupportEffectsReport {
    let p = plan(w);
    p.evaluate(&mut p.new_scratch()).unwrap()
}
fn value(r: &SupportEffectsReport) -> &EffectValue {
    &sniper::offering::effects(r)
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: d(0x3166),
                }
        })
        .unwrap()
        .value
}
fn spirit(r: &SupportEffectsReport, n: f64) {
    assert_eq!(
        value(r),
        &EffectValue::Known {
            value: quantity(n, &d(4))
        }
    );
    assert!(!sniper::offering::effects(r).values.iter().any(|v|matches!(&v.key,PlanValueKey::Stat{entity,stat} if *stat==d(0x3166)&&*entity!=ConcreteEntity::Actor(ActorKey::Player))),"Spirit final receiver belongs only to Player");
}
fn raw(w: &mut World, n: f64) {
    inner(w)
        .build
        .items
        .iter_mut()
        .find(|i| i.template == d(0x2343))
        .unwrap()
        .modifiers
        .iter_mut()
        .find(|m| m.definition == d(0x314d))
        .unwrap()
        .rolls
        .iter_mut()
        .find(|r| r.slot.slot == d(0x314e))
        .unwrap()
        .value = quantity(n, &d(0x295a));
}
fn control(w: &mut World, stat: u64, kind: ContributionKind, n: f64) {
    let owner = subject(d::<ActorDefinition>(0x332a));
    let u = if (0x3389..=0x338b).contains(&stat) || kind == ContributionKind::Increase {
        2
    } else if kind == ContributionKind::Multiply {
        1
    } else {
        4
    };
    let index = inner(w)
        .owner_mut(owner.clone())
        .programs
        .members
        .iter()
        .find(|p| p.id == key(CONTROL))
        .map_or(0, |p| p.effects.len());
    let node = key(&format!("value-{index}"));
    let effect = key(&format!("source-{index}"));
    let p = RuleProgram {
        id: key(CONTROL),
        context: RuleEntityKind::Actor,
        reads: vec![],
        nodes: vec![RuleNode {
            id: node.clone(),
            expression: RuleExpression::Literal {
                value: quantity(n, &d(u)),
            },
        }],
        effects: vec![RuleEffect {
            id: effect.clone(),
            when: None,
            effect: RuleEffectKind::Contribute {
                entity: RuleEntity::Current,
                stat: d(stat),
                contribution: kind,
                value: node,
            },
        }],
    };
    let programs = &mut inner(w).owner_mut(owner.clone()).programs.members;
    if let Some(old) = programs.iter_mut().find(|p| p.id == key(CONTROL)) {
        old.nodes.extend(p.nodes);
        old.effects.extend(p.effects);
    } else {
        programs.push(p);
    }
    let q = w
        .sniper
        .base
        .contribution_queries
        .members
        .iter_mut()
        .find(|q| q.stat == d(stat) && q.contribution == kind)
        .unwrap();
    q.groups[0].members.members.push(ContributionMember {
        producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
            owner,
            program: key(CONTROL),
            effect,
            origin: ContributionOrigin::ExistingActor {
                application: key("shared-player-initialization"),
            },
        }),
        order: if kind == ContributionKind::Override {
            None
        } else {
            Some(ContributionOrder {
                source_rank: 9,
                program_rank: 0,
                effect_rank: index as u32,
                slot_ranks: vec![],
            })
        },
    });
}
#[test]
#[ignore = "requires current joined Sniper release"]
fn spirit_source_vectors_use_imported_amulet_and_three_actual_rewards() {
    for row in player_spirit_pool_family::source_vectors()["vectors"]
        .as_array()
        .unwrap()
    {
        let mut w = world();
        let amount = match row["name"].as_str().unwrap() {
            "original" => 12.5,
            "legacy-zero" => 10.,
            "legacy-one" => 15.,
            _ => panic!(),
        };
        raw(&mut w, amount);
        let r = report(&w);
        assert!(r.gaps.is_empty());
        spirit(&r, row["final_spirit"].as_f64().unwrap());
        let copy: Vec<_> = sniper::offering::effects(&r)
            .effects
            .iter()
            .filter(|e| e.key.invocation.program == key(COPY))
            .collect();
        assert_eq!(copy.len(), 1);
        assert_eq!(
            copy[0].value,
            EffectValue::Known {
                value: quantity(0., &d(4))
            }
        );
    }
    let w = world();
    for reward in [0x28, 0x30, 0x63] {
        let mut x = w.clone();
        inner(&mut x)
            .build
            .character
            .rewards
            .retain(|r| r.definition != d(reward));
        spirit(&report(&x), 113. - if reward == 0x63 { 40. } else { 30. });
    }
    let mut unequipped = w.clone();
    unequipped.sniper.base.remove_item("amulet");
    spirit(&report(&unequipped), 100.);
    let mut inactive = w.clone();
    let template = inner(&mut inactive).owner_mut(subject(d::<ItemTemplateDefinition>(0x2343)));
    let eligible = template
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("amulet-copy-eligibility"))
        .unwrap();
    assert_eq!(eligible.nodes.len(), 1);
    eligible.nodes[0].expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(false),
    };
    let r = report(&inactive);
    spirit(&r, 113.);
    let copies: Vec<_> = sniper::offering::effects(&r)
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(COPY))
        .collect();
    assert_eq!(copies.len(), 1);
    assert_eq!(copies[0].value, EffectValue::Inactive);
    let mut missing = w.clone();
    inner(&mut missing)
        .build
        .items
        .iter_mut()
        .find(|i| i.template == d(0x2343))
        .unwrap()
        .modifiers
        .iter_mut()
        .find(|m| m.definition == d(0x314d))
        .unwrap()
        .rolls
        .retain(|r| r.slot.slot != d(0x314e));
    assert!(missing.checked_plan_configured(configure).is_err());
}
#[test]
#[ignore = "requires current joined Sniper release"]
fn spirit_fresh_reused_and_rayon_execution_agree() {
    let a = plan(&world());
    let mut changed = world();
    raw(&mut changed, 15.);
    let b = plan(&changed);
    let ar = a.evaluate(&mut a.new_scratch()).unwrap();
    let br = b.evaluate(&mut b.new_scratch()).unwrap();
    spirit(&ar, 113.);
    spirit(&br, 115.);
    assert_ne!(a.identity(), b.identity());
    let mut scratch = a.new_scratch();
    for (p, e) in [(&a, &ar), (&b, &br), (&a, &ar)] {
        assert_eq!(p.evaluate(&mut scratch).unwrap(), *e);
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let results: Vec<_> = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map_init(
                || a.new_scratch(),
                |scratch, i| {
                    let (p, e) = if i % 2 == 0 { (&a, &ar) } else { (&b, &br) };
                    (p.evaluate(scratch).unwrap(), e)
                },
            )
            .collect()
    });
    for (r, e) in results {
        assert_eq!(&r, e);
    }
}
#[test]
#[ignore = "requires current joined Sniper release"]
fn spirit_synthetic_transform_rounding_and_override_controls_are_explicit() {
    for (stat, kind, n, expected) in [
        (0x3166, ContributionKind::Increase, 10., 124.),
        (0x3166, ContributionKind::Multiply, 1.1, 124.),
        (0x3389, ContributionKind::Add, 25., 85.),
        (0x338c, ContributionKind::Add, 10., 123.),
        (0x338d, ContributionKind::Add, 1.5, 115.),
        (0x3166, ContributionKind::Override, 0., 0.),
        (0x3166, ContributionKind::Override, 17., 17.),
    ] {
        let mut w = world();
        control(&mut w, stat, kind, n);
        spirit(&report(&w), expected);
    }
    let mut bad = world();
    control(&mut bad, 0x338d, ContributionKind::Add, 2_f64.powi(52));
    assert!(format!("{:?}", value(&report(&bad))).contains("NumericRange"));
    control(&mut bad, 0x3166, ContributionKind::Override, 0.);
    spirit(&report(&bad), 0.);
    let mut conflicting = world();
    control(&mut conflicting, 0x3166, ContributionKind::Override, 0.);
    control(&mut conflicting, 0x3166, ContributionKind::Override, 2.);
    assert!(!matches!(
        value(&report(&conflicting)),
        EffectValue::Known { .. }
    ));
    let mut partial = world();
    let actual = player_spirit_pool_family::consumer().queries.remove(0);
    let query_id = actual.id.clone();
    *partial
        .sniper
        .base
        .contribution_queries
        .members
        .iter_mut()
        .find(|q| q.id == query_id)
        .unwrap() = actual;
    assert!(matches!(
        report(&partial).outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    let mut owner_partial = world();
    let actual = player_spirit_pool_family::consumer().owners.remove(1);
    let owner = actual.owner.clone();
    *inner(&mut owner_partial).owner_mut(owner) = actual;
    assert!(owner_partial.checked_plan_configured(configure).is_err());
}

#[test]
#[ignore = "requires current joined Sniper release"]
fn spirit_copy_uses_retained_scaling_probes_and_requires_frozen_channels() {
    for row in player_spirit_pool_family::source_vectors()["vectors"]
        .as_array()
        .unwrap()
    {
        for probe in row["copy_probes"].as_array().unwrap() {
            let mut w = world();
            raw(
                &mut w,
                match row["name"].as_str().unwrap() {
                    "original" => 12.5,
                    "legacy-zero" => 10.,
                    "legacy-one" => 15.,
                    _ => panic!(),
                },
            );
            // Synthetic input to the already declared frozen percentage. The
            // copied-Spirit program and source probe body remain unchanged.
            let owner = inner(&mut w).owner_mut(subject(d::<StatDefinition>(0x32e4)));
            let p = owner
                .programs
                .members
                .iter_mut()
                .find(|p| p.id == key("pre-amulet-bonus-snapshot"))
                .unwrap();
            assert_eq!(p.nodes.len(), 1);
            p.nodes[0].expression = RuleExpression::Literal {
                value: quantity(probe["factor"].as_f64().unwrap() * 100., &d(2)),
            };
            let r = report(&w);
            let rows: Vec<_> = sniper::offering::effects(&r)
                .effects
                .iter()
                .filter(|e| e.key.invocation.program == key(COPY))
                .collect();
            assert_eq!(rows.len(), 1);
            let expected = probe["records"][0]["value"].as_f64().unwrap();
            assert_eq!(
                rows[0].value,
                EffectValue::Known {
                    value: quantity(expected, &d(4))
                }
            );
            spirit(&r, row["final_spirit"].as_f64().unwrap() + expected);
        }
    }
    for stat in [0x32e4, 0x3166, 0x338c] {
        let result = world().checked_plan_configured(|stages| {
            configure(stages);
            stages.stages.push(EvaluationStage {
                id: key("fixture.spirit-too-late"),
                predecessors: vec![key(FINAL)],
            });
            let frozen = stages
                .frozen_channels
                .iter_mut()
                .find(|r| match &r.channel {
                    StageChannel::Stat { stat: s, .. } => stat == 0x32e4 && *s == d(stat),
                    StageChannel::Contributions {
                        stat: s,
                        contribution,
                        ..
                    } => stat != 0x32e4 && *s == d(stat) && *contribution == ContributionKind::Add,
                    _ => false,
                })
                .unwrap();
            frozen.stage = key("fixture.spirit-too-late");
        });
        assert!(result.is_err_and(|e| {
            e.contains("frozen channel read occurs before or outside frozen stage")
        }));
    }
}
#[test]
#[ignore = "requires current joined Sniper release"]
fn saved_spirit_reference_17_binds_player_313c_to_final_3166() {
    use poe_optimizer_core::{owned_draft::MetricRequestDraft, owned_metrics::*};
    use poe_optimizer_data::owned_metrics::OwnedMetricMapping;
    use std::sync::Arc;
    let endpoint = release::load(&path());
    let (draft, _) = imported_selection(&path());
    let saved = draft["draft"]["query_presets"]["members"][0]["queries"]["requests"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(decode::<MetricRequestDraft>)
        .find(|r| r.id.as_str() == "reference-17")
        .unwrap()
        .to_resolved()
        .unwrap();
    assert_eq!(saved.metric, d(0x313c));
    assert_eq!(saved.target, MetricTarget::Actor(ActorKey::Player));
    let descriptor = endpoint
        .input()
        .recipe
        .schema
        .definitions
        .iter()
        .find(|d| d.address() == saved.metric.address())
        .unwrap()
        .clone();
    for blocked in [false, true] {
        let mut w = world();
        if blocked {
            let actual = player_spirit_pool_family::consumer().queries.remove(0);
            let query_id = actual.id.clone();
            *w.sniper
                .base
                .contribution_queries
                .members
                .iter_mut()
                .find(|q| q.id == query_id)
                .unwrap() = actual;
        }
        let mut inputs = plan(&w).snapshot();
        assert!(
            !inputs
                .schema
                .definitions
                .iter()
                .any(|d| d.address() == descriptor.address())
        );
        inputs.schema.definitions.push(descriptor.clone());
        inputs.queries.requests.push(saved.clone());
        inputs.rebind_test_edit().unwrap();
        let effects = inputs.compile().unwrap();
        let mapping = OwnedMetricMapping::new(
            MetricMappingInput {
                schema_version: OWNED_METRIC_MAPPING_VERSION,
                namespace: inputs.schema.namespace.clone(),
                release: key("fixture-final-player-spirit-metric"),
                definitions: effects.definitions().identity().clone(),
                bindings: vec![MetricStatBinding {
                    metric: saved.metric.clone(),
                    role: MetricBindingRole::PlayerActor,
                    stat: d(0x3166),
                }],
            },
            effects.definitions(),
            Default::default(),
        )
        .unwrap();
        let metrics =
            OwnedSupportMetricPlan::compile(Arc::new(effects), Arc::new(mapping)).unwrap();
        let report = metrics.evaluate(&mut metrics.new_scratch()).unwrap();
        let result = report
            .evaluation
            .results
            .iter()
            .find(|r| r.request.id == saved.id)
            .unwrap();
        assert_eq!(result.request, saved);
        assert_eq!(result.stat, Some(d(0x3166)));
        if blocked {
            assert!(!matches!(result.value, EffectValue::Known { .. }));
        } else {
            assert_eq!(
                result.value,
                EffectValue::Known {
                    value: quantity(113., &d(4))
                }
            );
        }
    }
}
