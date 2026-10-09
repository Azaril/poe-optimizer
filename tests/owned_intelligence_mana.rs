//! Joined native resource dependencies and explicit source-control fixtures.
#[path = "support/owned_intelligence_mana.rs"]
mod family;
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
        i.schema.definitions.push(d);
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
    let a = mana_query_world();
    let mut b = a.clone();
    flags(&mut b, &[(0x3318, true)]);
    b.rebind_test_edit().unwrap();
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
    ];
    let expected: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    assert_eq!(mana(&expected[0]), &amount(210.));
    assert_eq!(mana(&expected[2]), &amount(420.));
    assert!(matches!(
        expected[1].outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    let seq = [0, 1, 2, 0];
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
