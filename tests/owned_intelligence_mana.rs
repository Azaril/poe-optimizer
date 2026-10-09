//! Joined native resource dependencies and explicit source-control fixtures.
#[path = "support/owned_mana_pool_adjustments.rs"]
mod adjustments;
#[path = "support/owned_intelligence_mana.rs"]
mod family;
#[path = "support/owned_mana_override.rs"]
mod mana_override;
#[path = "support/owned_mana_contribution_queries.rs"]
mod mana_queries;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_plan_replay.rs"]
mod replay;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::{
    build_identity::InstanceAllocator, owned_build::*, owned_definitions::*, owned_rules::*,
    owned_schema::*, owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release_migration::OwnedReleaseMigrationInput,
};
use rayon::prelude::*;
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(
        GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap(),
        format!("def.{n:016x}"),
    )
    .unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn subject(n: u64) -> SchemaSubject {
    SchemaSubject::Definition(def::<StatDefinition>(n).address())
}
fn actor() -> SchemaSubject {
    SchemaSubject::Definition(def::<ActorDefinition>(0x332a).address())
}
fn stage(i: &mut replay::ReplayInput, owner: SchemaSubject, p: &RuleProgram, template: &str) {
    let mut row = i
        .stages
        .programs
        .members
        .iter()
        .find(|x| x.program.as_str() == template)
        .unwrap()
        .clone();
    row.owner = owner.clone();
    row.program = p.id.clone();
    i.stages.programs.members.push(row);
    let readiness = i.stages.readiness.as_mut().unwrap();
    let mut row = readiness
        .programs
        .members
        .iter()
        .find(|x| x.program.as_str() == template)
        .unwrap()
        .clone();
    row.owner = owner;
    row.program = p.id.clone();
    readiness.programs.members.push(row);
}
fn world() -> replay::ReplayInput {
    let mut i = replay::ReplayInput::decode(
        &fs::read(family::root().join("tests/fixtures/owned-sniper-replay.json.gz")).unwrap(),
    );
    let c = family::consumer();
    let m = family::migration();
    let d: Value = family::read("dependencies.json");
    let deps: Vec<DefinitionDescriptor> = serde_json::from_value(d["definitions"].clone()).unwrap();
    for descriptor in deps {
        if let Some(actual) = i
            .schema
            .definitions
            .iter()
            .find(|x| x.address() == descriptor.address())
        {
            // The existing shared Actor has the same empty supply declarations;
            // the finite replay owns coverage, not production owner closure.
            assert_eq!(actual, &descriptor);
        } else {
            i.schema.definitions.push(descriptor);
        }
    }
    for row in m.schema {
        let SchemaExtensionEntry::Definition(d) = row else {
            panic!()
        };
        // The current replay already carries these published definitions. This
        // fixture installs only the additional component programs and stages.
        assert!(i.schema.definitions.contains(&d));
    }
    for o in &c.owners {
        for p in &o.programs.members {
            stage(&mut i, o.owner.clone(), p, "resolve-all-inherent-disabled");
        }
        i.rules.support_discovery.as_mut().unwrap().providers.push(
            SupportSourceDomainDeclaration {
                owner: o.owner.clone(),
                domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
            },
        );
    }
    let frozen:Vec<_>=i.stages.frozen_channels.iter().filter(|x|matches!(&x.channel,StageChannel::Stat{stat,..}|StageChannel::Contributions{stat,..} if *stat==def(0x3315))).cloned().collect();
    assert_eq!(frozen.len(), 2);
    for n in [0x3355, 0x3356] {
        for mut f in frozen.clone() {
            match &mut f.channel {
                StageChannel::Stat { stat, .. } | StageChannel::Contributions { stat, .. } => {
                    *stat = def(n)
                }
                _ => panic!(),
            };
            i.stages.frozen_channels.push(f);
        }
    }
    i.rules.owners.extend(c.owners);
    i.rules.receivers.members.extend(c.receivers);
    i.rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(c.queries);
    let intrinsic: OwnedReleaseMigrationInput = serde_json::from_slice(
        &fs::read(
            family::root().join("data/owned/poe2/3887ae68/player-intrinsic-mana/migration.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for p in c
        .actor
        .programs
        .members
        .into_iter()
        .chain(intrinsic.owners[0].programs.members.clone())
    {
        stage(&mut i, actor(), &p, "contribute-inherent-strength-life");
        i.rules
            .owners
            .iter_mut()
            .find(|o| o.owner == actor())
            .unwrap()
            .programs
            .members
            .push(p);
    }
    i.build.character.level = 92;
    i.rebind_test_edit().unwrap();
    i
}
fn set_intelligence(i: &mut replay::ReplayInput, value: Option<i64>) {
    let Some(v) = value else {
        i.rules.receivers.members.retain(|r| r.stat != def(0x1d30));
        return;
    };
    let p = &mut i
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == subject(0x1d30))
        .unwrap()
        .programs
        .members[0];
    // Source-control fixture only: retain the real final-input address and stage.
    // Baseline test below leaves the complete calculated Intelligence path intact.
    p.reads.clear();
    p.nodes.clear();
    p.nodes.push(RuleNode {
        id: key("test-input"),
        expression: RuleExpression::Literal {
            value: ParameterValue::Integer(BoundedInteger::new(v).unwrap()),
        },
    });
    let RuleEffectKind::Derive { value, .. } = &mut p.effects[0].effect else {
        panic!()
    };
    *value = key("test-input");
}
const FLAGS: [(&str, u64); 4] = [
    ("NoAttributeBonuses", 0x3315),
    ("NoIntelligenceAttributeBonuses", 0x3355),
    ("NoIntBonusToMana", 0x3356),
    ("DoubledInherentAttributeBonuses", 0x3318),
];
fn flags(i: &mut replay::ReplayInput, sources: &[(u64, bool)]) {
    if sources.is_empty() {
        return;
    }
    let p:RuleProgram=serde_json::from_value(json!({"id":"test-inherent-flags","context":"actor","reads":[],
        "nodes":sources.iter().enumerate().map(|(n,(_,v))|json!({"id":format!("value-{n}"),"expression":{"kind":"literal","value":{"kind":"boolean","value":v}}})).collect::<Vec<_>>(),
        "effects":sources.iter().enumerate().map(|(n,(stat,_))|json!({"id":format!("flag-{n}"),"when":null,"effect":{"kind":"contribute","entity":"current","stat":def::<StatDefinition>(*stat),"contribution":"flag","value":format!("value-{n}")}})).collect::<Vec<_>>() })).unwrap();
    stage(i, actor(), &p, "strength-life-halving");
    i.rules
        .owners
        .iter_mut()
        .find(|o| o.owner == actor())
        .unwrap()
        .programs
        .members
        .push(p);
    for (n, (stat, _)) in sources.iter().enumerate() {
        let q = i
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members
            .iter_mut()
            .find(|q| q.stat == def(*stat) && q.contribution == ContributionKind::Flag)
            .unwrap();
        q.groups[0].members.members.push(serde_json::from_value(json!({"producer":{"kind":"program_effect","owner":actor(),"program":"test-inherent-flags","effect":format!("flag-{n}"),"origin":{"kind":"existing_actor","application":"shared-player-initialization"}},"order":null})).unwrap());
    }
}
fn report(mut i: replay::ReplayInput) -> SupportEffectsReport {
    i.rebind_test_edit().unwrap();
    let p = i.compile().unwrap();
    p.evaluate(&mut p.new_scratch()).unwrap()
}
fn effects(r: &SupportEffectsReport) -> &OwnedEffectsReport {
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    let SupportEffectsOutcome::Evaluated { effects } = &r.outcome else {
        panic!("{r:?}")
    };
    assert!(effects.gaps.is_empty());
    effects
}
fn mana(r: &SupportEffectsReport) -> &EffectValue {
    let rows: Vec<_> = effects(r)
        .effects
        .iter()
        .filter(|e| e.key.invocation.program.as_str() == family::PROGRAM)
        .collect();
    assert_eq!(rows.len(), 1);
    let e = rows[0];
    assert_eq!(
        e.key.invocation.origin,
        RuleOrigin::ExistingActor {
            application: key("shared-player-initialization"),
            actor: ActorKey::Player
        }
    );
    assert_eq!(
        e.target,
        BoundEffectTarget::Contribution {
            key: ContributionKey {
                entity: ConcreteEntity::Actor(ActorKey::Player),
                stat: def(0x29f9),
                kind: ContributionKind::Add
            }
        }
    );
    &e.value
}
fn amount(n: f64) -> EffectValue {
    EffectValue::Known {
        value: ParameterValue::Quantity(FiniteQuantity::new(n, def(3)).unwrap()),
    }
}
#[test]
fn original_sniper_calculated_intelligence_supplies_mana() {
    family::check_authored();
    let r = report(world());
    assert_eq!(mana(&r), &amount(210.));
    let int = effects(&r)
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def(0x1d30),
                }
        })
        .unwrap();
    assert_eq!(
        int.value,
        EffectValue::Known {
            value: ParameterValue::Integer(BoundedInteger::new(105).unwrap())
        }
    );
    let intrinsic = effects(&r)
        .effects
        .iter()
        .find(|e| e.key.invocation.program.as_str() == "intrinsic-player-mana")
        .unwrap();
    assert_eq!(intrinsic.value, amount(398.));
    assert!(
        !effects(&r)
            .values
            .iter()
            .any(|v| matches!(&v.key,PlanValueKey::Stat{stat,..} if *stat==def(0x29f9))),
        "these inputs do not constitute final Mana"
    );
}
#[test]
fn source_controls_preserve_disabled_absence_enabled_zero_and_boolean_doubling() {
    let v: Value = family::read("source-vectors.json");
    for row in v["vectors"].as_array().unwrap() {
        let mut i = world();
        set_intelligence(&mut i, Some(row["intelligence"].as_i64().unwrap()));
        let mut controls: Vec<_> = FLAGS
            .iter()
            .filter(|(name, _)| row["flags"][name] == true)
            .map(|(_, n)| (*n, true))
            .collect();
        if row["name"] == "original-05-duplicate-double" {
            controls.push((0x3318, true));
        }
        flags(&mut i, &controls);
        let r = report(i);
        let expected = if row["emitted_records"].as_array().unwrap().is_empty() {
            EffectValue::Inactive
        } else {
            amount(row["amount"].as_f64().unwrap())
        };
        assert_eq!(mana(&r), &expected, "{}", row["name"]);
    }
}
#[test]
fn missing_input_and_incomplete_or_unlisted_flags_cannot_become_zero() {
    let mut missing = world();
    set_intelligence(&mut missing, None);
    assert!(matches!(
        mana(&report(missing.clone())),
        EffectValue::Unresolved { .. }
    ));
    flags(&mut missing, &[(0x3355, true)]);
    assert_eq!(mana(&report(missing)), &EffectValue::Inactive);
    let mut zero = world();
    set_intelligence(&mut zero, Some(0));
    zero.rules
        .receivers
        .members
        .retain(|r| r.stat != def(0x3318));
    assert!(
        matches!(mana(&report(zero)), EffectValue::Unresolved { .. }),
        "enabled zero still requires the doubling input"
    );
    let mut partial = world();
    partial
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.stat == def(0x3355))
        .unwrap()
        .groups[0]
        .members
        .closure = serde_json::from_value(json!({"kind":"partial","value":{"gaps":[{
        "subject":subject(0x3355),"facet":"game_rules","code":"unreviewed-mana-flag-sources"
    }]}}))
    .unwrap();
    assert!(
        matches!(
            report(partial).outcome,
            SupportEffectsOutcome::Unavailable { .. }
        ),
        "an empty partial query is not false"
    );
    let mut extra = world();
    flags(&mut extra, &[(0x3355, false)]);
    extra
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.stat == def(0x3355))
        .unwrap()
        .groups[0]
        .members
        .members
        .clear();
    extra.rebind_test_edit().unwrap();
    assert!(
        extra.compile().is_err(),
        "even an unread false writer needs membership"
    );
}
#[test]
fn mana_replays_match_across_fresh_reused_unknown_and_parallel_workers() {
    let mut a = mana_query_world();
    adjustments::install(&mut a);
    let mut b = a.clone();
    flags(&mut b, &[(0x3318, true)]);
    b.rebind_test_edit().unwrap();
    let mut c = a.clone();
    adjustment_sources(&mut c);
    flags(&mut c, &[(0x3355, true)]);
    c.rebind_test_edit().unwrap();
    let mut unknown = a.clone();
    unknown
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == actor())
        .unwrap()
        .programs
        .closure = family::consumer().actor.programs.closure;
    unknown.rebind_test_edit().unwrap();
    let plans = [
        a.compile().unwrap(),
        unknown.compile().unwrap(),
        b.compile().unwrap(),
        c.compile().unwrap(),
    ];
    let expected: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    assert_eq!(mana(&expected[0]), &amount(210.));
    assert_eq!(mana(&expected[2]), &amount(420.));
    assert_eq!(mana(&expected[3]), &EffectValue::Inactive);
    assert_eq!(
        adjustment_value(&expected[0], 0x335a),
        query_quantity(0., 3)
    );
    assert_eq!(
        adjustment_value(&expected[3], 0x335a),
        query_quantity(25.25, 3)
    );
    assert!(matches!(
        expected[1].outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    let seq = [0, 1, 2, 3, 0];
    let mut scratch = plans[0].new_scratch();
    for n in seq {
        assert_eq!(plans[n].evaluate(&mut scratch).unwrap(), expected[n]);
    }
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..12).into_par_iter().for_each(|_| {
                let mut s = plans[0].new_scratch();
                for n in seq {
                    assert_eq!(plans[n].evaluate(&mut s).unwrap(), expected[n]);
                }
            });
        });
}
#[test]
#[ignore = "requires INTELLIGENCE_MANA_PRIOR/OUTPUT and authenticated source reports"]
fn publish_intelligence_mana_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_INTELLIGENCE_MANA_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_INTELLIGENCE_MANA_OUTPUT").unwrap()),
        &family::data(),
        &[],
        &["authoring.json", "dependencies.json", "source-vectors.json"],
        family::stage,
        json!({"new_definitions":2,"new_programs":3,"new_queries":2,"final_mana":false,"whole_build_parity":false}),
        [107, 117, 109, 123, 4],
    );
}

// Finite query-binding fixture. Programs and membership are the published data;
// passive topology below is deliberately finite, not a legal full game tree.
fn mana_query_world() -> replay::ReplayInput {
    let mut i = world();
    let deps: mana_queries::Dependencies = mana_queries::read("dependencies.json");
    for d in deps.definitions {
        if i.schema
            .definitions
            .iter()
            .any(|x| x.address() == d.address())
        {
            continue;
        }
        let mut d = json!(d);
        if d["kind"] == "passive_node" {
            d["value"]["schema"]["value"]["adjacent"]["members"] = json!([]);
            d["value"]["schema"]["value"]["pools"]["members"] =
                json!([i.build.allocations[0].pool]);
        }
        i.schema
            .definitions
            .push(serde_json::from_value(d).unwrap());
    }
    for d in deps.producers {
        if i.rules.owners.iter().any(|o| o.owner == d.owner) {
            continue;
        }
        stage(&mut i, d.owner.clone(), &d.program, "strength-life-halving");
        i.rules.support_discovery.as_mut().unwrap().providers.push(
            SupportSourceDomainDeclaration {
                owner: d.owner.clone(),
                domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
            },
        );
        i.rules.owners.push(DefinitionRules {
            owner: d.owner,
            programs: DeclaredSet::complete(vec![d.program]),
        });
    }
    i.rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(mana_queries::queries());
    let mut reward = i.build.character.rewards[0].clone();
    let mut allocator = InstanceAllocator::from_state(i.build.allocator);
    reward.id = allocator.allocate().unwrap();
    i.build.allocator = allocator.state();
    reward.definition = def(0x003e);
    i.build.character.rewards.push(reward);
    // Diagnostic output channels do not replace the final Mana stat or formula.
    for (n, unit) in [(0xff01, 3), (0xff02, 2), (0xff03, 1)] {
        i.schema.definitions.push(serde_json::from_value(json!({"kind":"stat","value":{
            "id":def::<StatDefinition>(n),"schema":{"kind":"known","value":{
                "value":{"kind":"quantity","value":{"unit":def::<UnitDefinition>(unit)}},"targets":["actor"]}}
        }})).unwrap());
    }
    let rows = [
        (
            "intrinsic",
            "mana-base-contributions",
            "intrinsic",
            3,
            0xff01,
        ),
        ("inherent", "mana-base-contributions", "inherent", 3, 0xff01),
        (
            "rewards",
            "mana-increased-contributions",
            "rewards",
            2,
            0xff02,
        ),
        (
            "passives",
            "mana-increased-contributions",
            "passives",
            2,
            0xff02,
        ),
        ("more", "mana-more-contributions", "sources", 1, 0xff03),
    ];
    let p:RuleProgram = serde_json::from_value(json!({"id":"test-observe-mana-queries","context":"actor",
        "reads":rows.iter().map(|(id,q,g,u,_)|json!({"id":id,"value_type":{"kind":"quantity","value":{"unit":def::<UnitDefinition>(*u)}},"source":{"kind":"contribution_query","value":{"entity":"current","query":q,"group":g}}})).collect::<Vec<_>>(),
        "nodes":rows.iter().map(|(id,_,_,_,_)|json!({"id":id,"expression":{"kind":"read","input":id}})).collect::<Vec<_>>(),
        "effects":rows.iter().map(|(id,_,_,_,stat)|json!({"id":id,"when":null,"effect":{"kind":"contribute","entity":"current","stat":def::<StatDefinition>(*stat),"contribution":"add","value":id}})).collect::<Vec<_>>() })).unwrap();
    stage(&mut i, actor(), &p, "contribute-inherent-strength-life");
    i.stages
        .programs
        .members
        .iter_mut()
        .find(|x| x.program == p.id)
        .unwrap()
        .stage = key("test-mana-queries");
    i.stages.stages.push(
        serde_json::from_value(
            json!({"id":"test-mana-queries","predecessors":["player-inherent-life-contribution"]}),
        )
        .unwrap(),
    );
    for kind in [
        ContributionKind::Add,
        ContributionKind::Increase,
        ContributionKind::Multiply,
    ] {
        i.stages.frozen_channels.push(serde_json::from_value(json!({"channel":{"kind":"contributions","scope":"actor","stat":def::<StatDefinition>(0x29f9),"contribution":kind},"stage":"player-inherent-life-contribution"})).unwrap());
    }
    i.rules
        .owners
        .iter_mut()
        .find(|o| o.owner == actor())
        .unwrap()
        .programs
        .members
        .push(p);
    i.rebind_test_edit().unwrap();
    i
}
fn query_value(r: &SupportEffectsReport, id: &str) -> EffectValue {
    effects(r)
        .effects
        .iter()
        .find(|e| {
            e.key.invocation.program.as_str() == "test-observe-mana-queries"
                && e.key.effect.as_str() == id
        })
        .unwrap()
        .value
        .clone()
}
fn query_quantity(n: f64, unit: u64) -> EffectValue {
    EffectValue::Known {
        value: ParameterValue::Quantity(FiniteQuantity::new(n, def(unit)).unwrap()),
    }
}
#[test]
fn checked_mana_queries_bind_real_actor_reward_and_passive_programs() {
    let i = mana_query_world();
    let r = report(i.clone());
    for (id, n, unit) in [
        ("intrinsic", 398., 3),
        ("inherent", 210., 3),
        ("rewards", 5., 2),
        ("passives", 0., 2),
        ("more", 1., 1),
    ] {
        assert_eq!(query_value(&r, id), query_quantity(n, unit));
    }
    for (nodes, expected) in [
        (vec![0x109b], -10.),
        (vec![0x18d8], -30.),
        (vec![0x109b, 0x18d8], -40.),
    ] {
        let mut x = i.clone();
        let mut allocator = InstanceAllocator::from_state(x.build.allocator);
        for node in nodes {
            let mut a = x.build.allocations[0].clone();
            a.node = def(node);
            a.id = allocator.allocate().unwrap();
            a.choices.clear();
            x.build.allocations.push(a);
        }
        x.build.allocator = allocator.state();
        assert_eq!(
            query_value(&report(x), "passives"),
            query_quantity(expected, 2)
        );
    }
    let mut x = i.clone();
    x.build
        .character
        .rewards
        .retain(|r| r.definition != def(0x003e));
    assert_eq!(query_value(&report(x), "rewards"), query_quantity(0., 2));
    let mut x = i;
    set_intelligence(&mut x, None);
    assert!(matches!(
        query_value(&report(x), "inherent"),
        EffectValue::Unresolved { .. }
    ));
}
#[test]
fn checked_mana_membership_rejects_inactive_and_duplicate_bound_sources() {
    let mut i = mana_query_world();
    // Publication's census checks unselected declarations; the native request
    // checks all its potential bound writers, including an inactive one.
    flags(&mut i, &[(0x3355, true)]);
    i.rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id.as_str() == "mana-base-contributions")
        .unwrap()
        .groups[1]
        .members
        .members
        .pop();
    i.rebind_test_edit().unwrap();
    assert!(i.compile().is_err());
    let mut i = mana_query_world();
    let reward = i.build.character.rewards.last().unwrap().clone();
    let mut duplicate = reward;
    let mut allocator = InstanceAllocator::from_state(i.build.allocator);
    duplicate.id = allocator.allocate().unwrap();
    i.build.allocator = allocator.state();
    i.build.character.rewards.push(duplicate);
    i.rebind_test_edit().unwrap();
    assert!(
        i.compile().is_err(),
        "equal semantic numeric positions must not silently reorder duplicate rewards"
    );
}
#[test]
#[ignore = "requires MANA_QUERIES_PRIOR/OUTPUT"]
fn publish_mana_queries_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_MANA_QUERIES_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_MANA_QUERIES_OUTPUT").unwrap()),
        &mana_queries::data(),
        &[],
        &["authoring.json", "dependencies.json"],
        mana_queries::stage,
        json!({"new_queries":3,"groups":5,"potential_writers":5,"final_mana":false,"whole_build_parity":false}),
        [107, 117, 109, 123, 4],
    );
}

fn adjustment_world() -> replay::ReplayInput {
    let mut i = mana_query_world();
    adjustments::install(&mut i);
    i
}
fn adjustment_value(r: &SupportEffectsReport, stat: u64) -> EffectValue {
    effects(r)
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def(stat),
                }
        })
        .unwrap()
        .value
        .clone()
}
// Explicit finite controls only: selected values are not published game data.
fn adjustment_sources(i: &mut replay::ReplayInput) {
    let rows = [
        (0x3357, 2, 25.),
        (0x3358, 2, 10.),
        (0x3359, 2, 0.),
        (0x335a, 3, 25.25),
        (0x335b, 3, 7.),
    ];
    let p:RuleProgram=serde_json::from_value(json!({"id":"test-mana-adjustment-sources","context":"actor",
        "reads":[{"id":"active","value_type":{"kind":"boolean"},"source":{"kind":"stat","value":{"entity":"current","stat":def::<StatDefinition>(0x3355)}}}],
        "nodes":std::iter::once(json!({"id":"active","expression":{"kind":"read","input":"active"}})).chain(rows.iter().enumerate().map(|(n,(_,u,v))|json!({"id":format!("value-{n}"),"expression":{"kind":"literal","value":{"kind":"quantity","value":{"unit":def::<UnitDefinition>(*u),"value":v}}}}))).collect::<Vec<_>>(),
        "effects":rows.iter().enumerate().map(|(n,(stat,_,_))|json!({"id":format!("source-{n}"),"when":"active","effect":{"kind":"contribute","entity":"current","stat":def::<StatDefinition>(*stat),"contribution":"add","value":format!("value-{n}")}})).collect::<Vec<_>>() })).unwrap();
    // This controlled producer needs the resolved flag. Move its channel freeze
    // and receivers after it; retain explicit stages rather than skipping checks.
    stage(i, actor(), &p, "contribute-inherent-strength-life");
    i.rules
        .owners
        .iter_mut()
        .find(|o| o.owner == actor())
        .unwrap()
        .programs
        .members
        .push(p);
    for q in adjustments::consumer().queries {
        let index = rows
            .iter()
            .position(|(stat, _, _)| def::<StatDefinition>(*stat) == q.stat)
            .unwrap();
        let g = &mut i
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members
            .iter_mut()
            .find(|r| r.id == q.id)
            .unwrap()
            .groups[0];
        g.members.members.push(serde_json::from_value(json!({"producer":{"kind":"program_effect","owner":actor(),"program":"test-mana-adjustment-sources","effect":format!("source-{index}"),"origin":{"kind":"existing_actor","application":"shared-player-initialization"}},"order":{"source_rank":0,"program_rank":0,"effect_rank":index,"slot_ranks":[]}})).unwrap());
    }
    for o in adjustments::consumer().owners {
        for row in &mut i.stages.programs.members {
            if row.owner == o.owner {
                row.stage = key("test-mana-queries");
            }
        }
    }
    for row in &mut i.stages.frozen_channels {
        if matches!(&row.channel,StageChannel::Contributions{stat,..} if rows.iter().any(|(n,_,_)|*stat==def(*n)))
        {
            row.stage = key("player-inherent-life-contribution");
        }
    }
}
#[test]
fn mana_adjustments_preserve_units_recipients_and_inactive_zero() {
    let i = adjustment_world();
    let r = report(i.clone());
    for (stat, unit) in [
        (0x3357, 2),
        (0x3358, 2),
        (0x3359, 2),
        (0x335a, 3),
        (0x335b, 3),
    ] {
        assert_eq!(adjustment_value(&r, stat), query_quantity(0., unit));
        assert!(!effects(&r).values.iter().any(|v|matches!(&v.key,PlanValueKey::Stat{entity,stat:s} if *s==def(stat)&&*entity!=ConcreteEntity::Actor(ActorKey::Player))));
    }
    let mut inactive = i.clone();
    adjustment_sources(&mut inactive);
    assert_eq!(
        adjustment_value(&report(inactive.clone()), 0x335a),
        query_quantity(0., 3)
    );
    let mut active = inactive;
    flags(&mut active, &[(0x3355, true)]);
    let r = report(active);
    for (stat, unit, value) in [
        (0x3357, 2, 25.),
        (0x3358, 2, 10.),
        (0x3359, 2, 0.),
        (0x335a, 3, 25.25),
        (0x335b, 3, 7.),
    ] {
        assert_eq!(adjustment_value(&r, stat), query_quantity(value, unit));
    }
    assert!(
        !effects(&r)
            .values
            .iter()
            .any(|v| matches!(&v.key,PlanValueKey::Stat{stat,..} if *stat==def(0x29f9))),
        "input collectors do not claim final Mana"
    );
}
#[test]
fn mana_adjustments_refuse_unknown_partial_and_unlisted_sources() {
    let mut i = adjustment_world();
    adjustment_sources(&mut i);
    i.rules.receivers.members.retain(|r| r.stat != def(0x3355));
    let r = report(i);
    assert!(matches!(
        adjustment_value(&r, 0x335a),
        EffectValue::Unresolved { .. }
    ));
    let mut i = adjustment_world();
    adjustment_sources(&mut i);
    i.rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.stat == def(0x335a))
        .unwrap()
        .groups[0]
        .members
        .members
        .clear();
    i.rebind_test_edit().unwrap();
    assert!(
        i.compile().is_err(),
        "an inactive source still requires membership"
    );
    let mut i = adjustment_world();
    i.rules.contribution_queries.as_mut().unwrap().members.iter_mut().find(|q|q.stat==def(0x335a)).unwrap().groups[0].members.closure=serde_json::from_value(json!({"kind":"partial","value":{"gaps":[{"subject":subject(0x335a),"facet":"game_rules","code":"unreviewed-conversion-sources"}]}})).unwrap();
    assert!(matches!(
        report(i).outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
}
#[test]
#[ignore = "requires MANA_ADJUSTMENTS_PRIOR/OUTPUT"]
fn publish_mana_adjustments_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_MANA_ADJUSTMENTS_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_MANA_ADJUSTMENTS_OUTPUT").unwrap()),
        &adjustments::data(),
        &[],
        &["authoring.json", "dependencies.json", "bindings.json"],
        adjustments::stage,
        json!({"new_definitions":5,"new_queries":5,"new_programs":5,"new_receivers":5,"final_mana":false,"complete_conversion_mechanics":false,"whole_build_parity":false}),
        [107, 117, 109, 123, 4],
    );
}

// This finite component fixture admits the actual override program only. It
// deliberately does not claim Blood Magic's other mechanic or legal topology.
fn override_world() -> replay::ReplayInput {
    let mut i = adjustment_world();
    let d: mana_override::Dependencies = mana_override::read("dependencies.json");
    for definition in d.definitions {
        if i.schema
            .definitions
            .iter()
            .any(|x| x.address() == definition.address())
        {
            continue;
        }
        let mut v = json!(definition);
        assert_eq!(v["kind"], "passive_node");
        let schema = &mut v["value"]["schema"]["value"];
        schema["adjacent"]["members"] = json!([]);
        schema["pools"]["members"] = json!([i.build.allocations[0].pool]);
        for declaration in schema["declarations"].as_object_mut().unwrap().values_mut() {
            assert!(declaration["members"].as_array().unwrap().is_empty());
            declaration["closure"] = json!({"kind":"complete"});
        }
        i.schema
            .definitions
            .push(serde_json::from_value(v).unwrap());
    }
    let mut owner = mana_override::migration().owners.remove(0);
    owner.programs.closure = SchemaClosure::Complete;
    stage(
        &mut i,
        owner.owner.clone(),
        &owner.programs.members[0],
        "strength-life-halving",
    );
    i.rules
        .support_discovery
        .as_mut()
        .unwrap()
        .providers
        .push(SupportSourceDomainDeclaration {
            owner: owner.owner.clone(),
            domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
        });
    i.rules.owners.push(owner);
    i.rules.operations_version = key(OWNED_RULE_OPERATIONS_V27);
    i.rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(mana_override::queries());
    i.stages.frozen_channels.push(serde_json::from_value(json!({"channel":{"kind":"contributions","scope":"actor","stat":def::<StatDefinition>(0x29f9),"contribution":"override"},"stage":"player-inherent-life-contribution"})).unwrap());
    i.schema.definitions.push(serde_json::from_value(json!({"kind":"stat","value":{"id":def::<StatDefinition>(0xff04),"schema":{"kind":"known","value":{"value":{"kind":"boolean"},"targets":["actor"]}}}})).unwrap());
    let p:RuleProgram=serde_json::from_value(json!({"id":"test-observe-mana-override","context":"actor",
        "reads":[
            {"id":"present","value_type":{"kind":"boolean"},"source":{"kind":"contribution_selection","value":{"entity":"current","query":"mana-override-contributions","group":"sources","projection":"present"}}},
            {"id":"value","value_type":{"kind":"quantity","value":{"unit":def::<UnitDefinition>(3)}},"source":{"kind":"contribution_selection","value":{"entity":"current","query":"mana-override-contributions","group":"sources","projection":"value"}}}],
        "nodes":[{"id":"present","expression":{"kind":"read","input":"present"}},{"id":"value","expression":{"kind":"read","input":"value"}}],
        "effects":[{"id":"present","when":null,"effect":{"kind":"contribute","entity":"current","stat":def::<StatDefinition>(0xff04),"contribution":"flag","value":"present"}},
            {"id":"value","when":"present","effect":{"kind":"contribute","entity":"current","stat":def::<StatDefinition>(0xff01),"contribution":"add","value":"value"}}]})).unwrap();
    stage(&mut i, actor(), &p, "test-observe-mana-queries");
    i.rules
        .owners
        .iter_mut()
        .find(|o| o.owner == actor())
        .unwrap()
        .programs
        .members
        .push(p);
    i.rebind_test_edit().unwrap();
    i
}
fn allocate_override(i: &mut replay::ReplayInput) {
    let mut allocator = InstanceAllocator::from_state(i.build.allocator);
    let mut a = i.build.allocations[0].clone();
    a.node = def(0x16be);
    a.id = allocator.allocate().unwrap();
    a.choices.clear();
    i.build.allocations.push(a);
    i.build.allocator = allocator.state();
}
fn override_value(r: &SupportEffectsReport, id: &str) -> EffectValue {
    effects(r)
        .effects
        .iter()
        .find(|e| {
            e.key.invocation.program.as_str() == "test-observe-mana-override"
                && e.key.effect.as_str() == id
        })
        .unwrap()
        .value
        .clone()
}
#[test]
fn actual_mana_override_packet_preserves_gaps_and_checked_source_membership() {
    mana_override::check_authored();
    let owners = mana_override::migration().owners;
    let mut bad = owners.clone();
    bad[0].programs.members[0].effects[0].when = Some(key("zero"));
    assert!(std::panic::catch_unwind(|| mana_override::check_membership(&bad, &[])).is_err());
    let mut bad = owners.clone();
    let mut duplicate = bad[0].programs.members[0].clone();
    duplicate.id = key("unreviewed-override");
    bad[0].programs.members.push(duplicate);
    assert!(std::panic::catch_unwind(|| mana_override::check_membership(&bad, &[])).is_err());
}
#[test]
fn actual_mana_override_distinguishes_unselected_from_selected_zero() {
    let mut i = override_world();
    let r = report(i.clone());
    assert_eq!(
        override_value(&r, "present"),
        EffectValue::Known {
            value: ParameterValue::Boolean(false)
        }
    );
    assert_eq!(override_value(&r, "value"), EffectValue::Inactive);
    allocate_override(&mut i);
    let r = report(i);
    assert_eq!(
        override_value(&r, "present"),
        EffectValue::Known {
            value: ParameterValue::Boolean(true)
        }
    );
    assert_eq!(override_value(&r, "value"), query_quantity(0., 3));
    let contributions: Vec<_> = effects(&r)
        .effects
        .iter()
        .filter(|e| e.key.invocation.program.as_str() == mana_override::PROGRAM)
        .collect();
    assert_eq!(contributions.len(), 1);
    assert_eq!(contributions[0].value, query_quantity(0., 3));
    assert!(
        !effects(&r)
            .values
            .iter()
            .any(|v| matches!(&v.key,PlanValueKey::Stat{stat,..} if *stat==def(0x29f9))),
        "selection inputs are not a final Mana consumer"
    );
}
#[test]
fn actual_mana_override_refuses_unlisted_and_partial_sources() {
    let mut i = override_world();
    allocate_override(&mut i);
    let q = i
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id.as_str() == "mana-override-contributions")
        .unwrap();
    q.groups[0].members.members.clear();
    i.rebind_test_edit().unwrap();
    assert!(i.compile().is_err());
    let mut i = override_world();
    allocate_override(&mut i);
    let real = mana_override::migration().owners.remove(0);
    i.rules
        .owners
        .iter_mut()
        .find(|o| o.owner == real.owner)
        .unwrap()
        .programs
        .closure = real.programs.closure;
    assert!(
        matches!(report(i).outcome, SupportEffectsOutcome::Unavailable { .. }),
        "the production owner still has other unsupported mechanics"
    );
}
#[test]
fn actual_mana_override_is_stable_across_fresh_reused_and_parallel_workers() {
    let base = override_world();
    let mut allocated = base.clone();
    allocate_override(&mut allocated);
    allocated.rebind_test_edit().unwrap();
    let mut unknown = allocated.clone();
    let actual = mana_override::migration().owners.remove(0);
    unknown
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == actual.owner)
        .unwrap()
        .programs
        .closure = actual.programs.closure;
    unknown.rebind_test_edit().unwrap();
    let plans = [
        base.compile().unwrap(),
        allocated.compile().unwrap(),
        unknown.compile().unwrap(),
    ];
    let expected: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    assert_eq!(
        override_value(&expected[0], "present"),
        EffectValue::Known {
            value: ParameterValue::Boolean(false)
        }
    );
    assert_eq!(override_value(&expected[1], "value"), query_quantity(0., 3));
    assert!(matches!(
        expected[2].outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    let mut scratch = plans[0].new_scratch();
    for n in [0, 1, 2, 1, 0] {
        assert_eq!(plans[n].evaluate(&mut scratch).unwrap(), expected[n]);
    }
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..12).into_par_iter().for_each(|_| {
                let mut scratch = plans[0].new_scratch();
                for n in [0, 1, 2, 1, 0] {
                    assert_eq!(plans[n].evaluate(&mut scratch).unwrap(), expected[n]);
                }
            })
        });
}
#[test]
#[ignore = "requires MANA_OVERRIDE_PRIOR/OUTPUT and authenticated source reports"]
fn publish_mana_override_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_MANA_OVERRIDE_PRIOR").unwrap()),
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_MANA_OVERRIDE_OUTPUT").unwrap()),
        &mana_override::data(),
        &[],
        &["authoring.json", "dependencies.json", "source-vectors.json"],
        mana_override::stage,
        json!({"new_queries":1,"new_programs":1,"closed_owners":0,"final_mana":false,"whole_build_parity":false}),
        [107, 117, 109, 123, 4],
    );
}

// Draft formula integration. The consumer is not published until numeric-domain
// admission is implemented; these tests certify only the stated input domain.
fn pool_world() -> replay::ReplayInput {
    #[derive(serde::Deserialize)]
    struct Consumer {
        owners: Vec<DefinitionRules>,
        receivers: Vec<StatReceiver>,
    }
    let c: Consumer = serde_json::from_slice(
        &fs::read(family::root().join("data/owned/poe2/3887ae68/mana-pool/consumer.json")).unwrap(),
    )
    .unwrap();
    let mut i = override_world();
    for o in &c.owners {
        assert!(!i.rules.owners.iter().any(|r| r.owner == o.owner));
        for p in &o.programs.members {
            stage(&mut i, o.owner.clone(), p, "test-observe-mana-queries");
        }
    }
    i.rules.owners.extend(c.owners);
    i.rules.receivers.members.extend(c.receivers);
    i.rebind_test_edit().unwrap();
    i
}
#[test]
fn draft_mana_pool_uses_real_level_attributes_reward_and_override_queries() {
    let i = pool_world();
    let r = report(i.clone());
    assert_eq!(adjustment_value(&r, 0x29f9), query_quantity(638., 3));
    let mut changed = i.clone();
    changed.build.character.level = 93;
    assert_eq!(
        adjustment_value(&report(changed), 0x29f9),
        query_quantity(643., 3)
    );
    let mut changed = i.clone();
    flags(&mut changed, &[(0x3355, true)]);
    assert_eq!(
        adjustment_value(&report(changed), 0x29f9),
        query_quantity(418., 3)
    );
    let mut changed = i.clone();
    allocate_override(&mut changed);
    assert_eq!(
        adjustment_value(&report(changed), 0x29f9),
        query_quantity(0., 3)
    );
    let mut unknown = i;
    set_intelligence(&mut unknown, None);
    assert!(matches!(
        adjustment_value(&report(unknown), 0x29f9),
        EffectValue::Unresolved { .. }
    ));
}

#[test]
fn draft_mana_pool_matches_every_retained_original_source_vector() {
    use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
    use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact};
    let i = pool_world();
    let schema = OwnedDefinitionSchemaPackage::new(i.schema.clone(), Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(&i.rules, &schema, Default::default()).unwrap();
    let owner = subject(0x29f9);
    let p = key("resolve-player-mana");
    let v: Value = mana_override::read("source-vectors.json");
    let mut scratch = compiled.new_scratch();
    let rows = v["vectors"].as_array().unwrap();
    for row in rows.iter().chain(rows.iter().rev()) {
        let s = &row["source"]["inputs"];
        // This is a scalar consumer boundary: original aggregate inputs are
        // injected only here. The joined test above uses actual producer data.
        let mut facts: Vec<RuleFact> = [
            ("intrinsic", s["base"].as_f64().unwrap(), 3),
            ("inherent", 0., 3),
            ("reward-increase", s["increased"].as_f64().unwrap(), 2),
            ("passive-increase", 0., 2),
            ("more", s["more"].as_f64().unwrap(), 1),
            ("to-energy-shield", s["conversion_sum"].as_f64().unwrap(), 2),
            ("to-armour", 0., 2),
            ("to-evasion", 0., 2),
            ("extra", s["extra"].as_f64().unwrap(), 3),
            ("total", s["total"].as_f64().unwrap(), 3),
        ]
        .map(|(name, value, u)| RuleFact {
            read: key(name),
            value: ParameterValue::Quantity(FiniteQuantity::new(value, def(u)).unwrap()),
        })
        .into();
        facts.push(RuleFact {
            read: key("override-present"),
            value: ParameterValue::Boolean(s["override"]["present"].as_bool().unwrap()),
        });
        if let Some(n) = s["override"]["value"].as_f64() {
            facts.push(RuleFact {
                read: key("override-value"),
                value: ParameterValue::Quantity(FiniteQuantity::new(n, def(3)).unwrap()),
            });
        }
        let result = compiled
            .evaluate(&owner, &p, &facts, &schema, &mut scratch)
            .unwrap();
        assert_eq!(
            result.effects[0].disposition,
            EffectDisposition::Applied {
                value: ParameterValue::Quantity(
                    FiniteQuantity::new(row["source"]["final_mana"].as_f64().unwrap(), def(3))
                        .unwrap()
                )
            },
            "{}",
            row["name"]
        );
        assert_eq!(
            result,
            compiled
                .evaluate(&owner, &p, &facts, &schema, &mut compiled.new_scratch())
                .unwrap()
        );
    }
}

#[test]
fn draft_mana_pool_retains_coverage_and_frozen_adjustment_dependencies() {
    let mut i = pool_world();
    i.rules.receivers.members.retain(|r| r.stat != def(0x335a));
    assert!(
        matches!(
            adjustment_value(&report(i), 0x29f9),
            EffectValue::Unresolved { .. }
        ),
        "missing extra Mana is not zero"
    );
    let mut i = pool_world();
    let query = i
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.stat == def(0x335b))
        .unwrap();
    query.groups[0].members.closure = serde_json::from_value(json!({
        "kind": "partial",
        "value": { "gaps": [{
            "subject": subject(0x335b),
            "facet": "game_rules",
            "code": "unreviewed-total-mana-sources"
        }] }
    }))
    .unwrap();
    assert!(matches!(
        report(i).outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    let mut i = pool_world();
    i.stages
        .programs
        .members
        .iter_mut()
        .find(|s| s.program.as_str() == "resolve-player-mana")
        .unwrap()
        .stage = key("inherent-strength-life");
    let error = i.rebind_test_edit().unwrap_err();
    assert!(
        error.contains("frozen channel read occurs before or outside frozen stage"),
        "a consumer cannot run before the stage that freezes its contributions: {error}"
    );
}

#[test]
fn draft_mana_pool_parallel_workers_restore_after_zero_and_unknown() {
    let base = pool_world();
    let mut zero = base.clone();
    allocate_override(&mut zero);
    zero.rebind_test_edit().unwrap();
    let mut missing = base.clone();
    set_intelligence(&mut missing, None);
    missing.rebind_test_edit().unwrap();
    let plans = [
        base.compile().unwrap(),
        zero.compile().unwrap(),
        missing.compile().unwrap(),
    ];
    let expected: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    assert_eq!(
        adjustment_value(&expected[0], 0x29f9),
        query_quantity(638., 3)
    );
    assert_eq!(
        adjustment_value(&expected[1], 0x29f9),
        query_quantity(0., 3)
    );
    assert!(matches!(
        adjustment_value(&expected[2], 0x29f9),
        EffectValue::Unresolved { .. }
    ));
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..12).into_par_iter().for_each(|_| {
                let mut s = plans[0].new_scratch();
                for n in [0, 1, 2, 0, 2, 1, 0] {
                    assert_eq!(plans[n].evaluate(&mut s).unwrap(), expected[n]);
                }
            })
        });
}
