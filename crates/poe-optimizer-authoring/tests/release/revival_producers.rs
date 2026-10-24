//! Revival component facts retain exact recipients and incomplete source coverage.
//! No duration consumer, parent inheritance or complete-build result is asserted.
use super::{revival_source, tracked_release, tracked_sources};
use flate2::read::GzDecoder;
use poe_optimizer_authoring::load_names;
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition};
use std::{collections::BTreeSet, io::Read as _, sync::OnceLock};

const ENABLED: &str = "stat.revival.enabled";
const BASE: &str = "stat.revival.base_override";
const INCREASED: &str = "stat.revival.speed_increased";
const INTRINSIC: &str = "intrinsic-revival";
const SPEED: &str = "ordinary-revival-speed";
const QUERY_IDS: [&str; 3] = ["revival-enabled", "revival-base", "revival-increased"];

pub(super) fn stat_names() -> BTreeSet<String> {
    [ENABLED, BASE, INCREASED].map(str::to_owned).into()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
struct Intrinsic {
    owner: SchemaSubject,
    supply: DeclaredSlot<SkillGrantSlotDefId>,
}
struct Bindings {
    enabled: StatDefId,
    base: StatDefId,
    increased: StatDefId,
    seconds: UnitDefId,
    percent: UnitDefId,
    intrinsics: [Intrinsic; 4],
    // Exact query order; equal game names and values do not merge these owners.
    passives: [SchemaSubject; 3],
}
fn bindings() -> &'static Bindings {
    static BINDINGS: OnceLock<Bindings> = OnceLock::new();
    BINDINGS.get_or_init(|| {
        let built = tracked_release();
        let schema = built.release().assembled().schema();
        let names = built
            .name_allocation()
            .names()
            .symbols(schema, Default::default())
            .unwrap();
        let aliases = load_names(
            tracked_sources().text("names/revival-producers.kdl"),
            schema,
            Default::default(),
        )
        .unwrap();
        let stat = |name| match names.definition(name).unwrap() {
            DefinitionAddress::Stat(id) => id.clone(),
            _ => panic!("allocated generic Stat {name}"),
        };
        let unit = |name| match aliases.definition(name).unwrap() {
            DefinitionAddress::Unit(id) => id.clone(),
            _ => panic!("known Unit {name}"),
        };
        let passive = |name, expected| match aliases.definition(name).unwrap() {
            DefinitionAddress::PassiveNode(id) => {
                assert_eq!(id.key().as_str(), expected);
                SchemaSubject::Definition(id.address())
            }
            _ => panic!("exact passive source {name}"),
        };
        // Fixed source-to-owned joins, independent of the query membership under test.
        let intrinsics = [
            (
                "skill.sniper_skill",
                "0012",
                "0011",
                "skill_grant_slot.sniper_primary_skill",
                "0016",
            ),
            (
                "skill.skeletal_arsonists",
                "0323",
                "0914",
                "skill_grant_slot.skeletal_arsonists_primary",
                "3285",
            ),
            (
                "skill.skeletal_frost_mages",
                "0326",
                "0917",
                "skill_grant_slot.skeletal_frost_mages_primary",
                "328f",
            ),
            (
                "skill.skeletal_reavers",
                "0327",
                "0918",
                "skill_grant_slot.skeletal_reavers_primary",
                "3299",
            ),
        ]
        .map(|(name, skill_key, gem_key, supply_name, supply_key)| {
            let DefinitionAddress::Skill(skill) = aliases.definition(name).unwrap() else {
                panic!("exact skeletal Skill")
            };
            assert_eq!(skill.key().as_str(), format!("def.000000000000{skill_key}"));
            let SlotAddress::SkillGrant(supply) = aliases.slot(supply_name).unwrap() else {
                panic!("exact physical Gem supply")
            };
            assert_eq!(
                supply.slot.key().as_str(),
                format!("def.000000000000{supply_key}")
            );
            let SlotOwnerDefId::Gem(gem) = &supply.declaration else {
                panic!("physical Gem declaration")
            };
            assert_eq!(gem.key().as_str(), format!("def.000000000000{gem_key}"));
            let SchemaLookup::Known(declaration) = schema.slot(supply) else {
                panic!("known supplying slot")
            };
            assert_eq!(&declaration.skill, skill);
            Intrinsic {
                owner: SchemaSubject::Definition(skill.address()),
                supply: supply.clone(),
            }
        });
        Bindings {
            enabled: stat(ENABLED),
            base: stat(BASE),
            increased: stat(INCREASED),
            seconds: unit("unit.seconds"),
            percent: unit("unit.percentage_points"),
            intrinsics,
            passives: [
                passive("passive_node.minion_revive_speed.a", "def.0000000000000f9f"),
                passive("passive_node.restless_dead", "def.00000000000012f4"),
                passive("passive_node.minion_revive_speed.b", "def.0000000000001476"),
            ],
        }
    })
}
fn rules() -> &'static RulePackageInput {
    tracked_release()
        .release()
        .assembled()
        .checked()
        .rules()
        .input()
}
fn owner(subject: &SchemaSubject) -> &'static DefinitionRules {
    rules()
        .owners
        .iter()
        .find(|row| &row.owner == subject)
        .unwrap()
}
fn program(subject: &SchemaSubject, id: &str) -> &'static RuleProgram {
    owner(subject)
        .programs
        .members
        .iter()
        .find(|row| row.id.as_str() == id)
        .unwrap()
}
fn quantity(value: f64, unit: &UnitDefId) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap())
}
fn gap(stat: &StatDefId, code: &str) -> SchemaGap {
    SchemaGap {
        subject: SchemaSubject::Definition(stat.address()),
        facet: SchemaFacet::GameRules,
        code: key(code),
    }
}
fn member(
    owner: &SchemaSubject,
    program: &str,
    effect: &str,
    origin: ContributionOrigin,
    order: Option<ContributionOrder>,
) -> ContributionMember {
    ContributionMember {
        producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
            owner: owner.clone(),
            program: key(program),
            effect: key(effect),
            origin,
        }),
        order,
    }
}
fn expected_queries() -> Vec<ContributionQuery> {
    let b = bindings();
    let intrinsic_members = |effect| {
        b.intrinsics
            .iter()
            .map(|intrinsic| {
                member(
                    &intrinsic.owner,
                    INTRINSIC,
                    effect,
                    ContributionOrigin::Skill {
                        authored: false,
                        supplies: vec![intrinsic.supply.clone()],
                    },
                    None,
                )
            })
            .collect()
    };
    vec![
        ContributionQuery {
            id: key(QUERY_IDS[0]),
            stat: b.enabled.clone(),
            contribution: ContributionKind::Flag,
            groups: vec![ContributionGroup {
                id: key("sources"),
                reduction: ContributionReduction::Any,
                ordering: ContributionOrdering::Unordered,
                empty: Some(ParameterValue::Boolean(false)),
                members: DeclaredSet::partial(
                    intrinsic_members("reviving"),
                    vec![gap(&b.enabled, "revival-flag-incoming-sources-unproved")],
                ),
            }],
        },
        ContributionQuery {
            id: key(QUERY_IDS[1]),
            stat: b.base.clone(),
            contribution: ContributionKind::Override,
            groups: vec![ContributionGroup {
                id: key("sources"),
                reduction: ContributionReduction::RequireAgreement,
                ordering: ContributionOrdering::Unordered,
                empty: None,
                members: DeclaredSet::partial(
                    intrinsic_members("base-duration"),
                    vec![gap(&b.base, "revival-base-incoming-sources-unproved")],
                ),
            }],
        },
        ContributionQuery {
            id: key(QUERY_IDS[2]),
            stat: b.increased.clone(),
            contribution: ContributionKind::Increase,
            groups: vec![ContributionGroup {
                id: key("sources"),
                reduction: ContributionReduction::Sum,
                ordering: ContributionOrdering::Ordered,
                empty: Some(quantity(0.0, &b.percent)),
                members: DeclaredSet::partial(
                    b.passives
                        .iter()
                        .enumerate()
                        .map(|(index, owner)| {
                            member(
                                owner,
                                SPEED,
                                "speed",
                                ContributionOrigin::Allocation,
                                Some(ContributionOrder {
                                    source_rank: index as u32,
                                    program_rank: 0,
                                    effect_rank: 0,
                                    slot_ranks: vec![],
                                }),
                            )
                        })
                        .collect(),
                    vec![
                        gap(&b.increased, "revival-speed-incoming-sources-unproved"),
                        gap(&b.increased, "revival-numeric-order-domain-unproved"),
                    ],
                ),
            }],
        },
    ]
}
fn assert_program(
    subject: &SchemaSubject,
    id: &str,
    context: RuleEntityKind,
    expected: &[(
        RuleEntity,
        &StatDefId,
        ContributionKind,
        &str,
        ParameterValue,
    )],
) {
    let body = program(subject, id);
    assert_eq!(body.context, context);
    assert!(
        body.reads.is_empty(),
        "intrinsic constants introduce no saved-input dependence"
    );
    assert_eq!(body.nodes.len(), expected.len());
    assert_eq!(body.effects.len(), expected.len());
    for ((node, effect), (entity, stat, contribution, name, value)) in
        body.nodes.iter().zip(&body.effects).zip(expected)
    {
        assert_eq!(
            node.expression,
            RuleExpression::Literal {
                value: value.clone()
            }
        );
        assert_eq!(effect.id, key(name));
        assert!(effect.when.is_none());
        assert_eq!(
            effect.effect,
            RuleEffectKind::Contribute {
                entity: *entity,
                stat: (*stat).clone(),
                contribution: *contribution,
                value: node.id.clone(),
            }
        );
    }
}

#[test]
fn generic_revival_channels_and_seven_producers_keep_exact_owner_and_recipient_authority() {
    revival_source::check_proof();
    let b = bindings();
    let schema = tracked_release().release().assembled().schema();
    for (stat, value) in [
        (&b.enabled, ComputedValueType::Boolean),
        (
            &b.base,
            ComputedValueType::Quantity {
                unit: b.seconds.clone(),
            },
        ),
        (
            &b.increased,
            ComputedValueType::Quantity {
                unit: b.percent.clone(),
            },
        ),
    ] {
        let SchemaLookup::Known(actual) = schema.definition(stat) else {
            panic!("known generic Stat")
        };
        assert_eq!(actual.value, value);
        assert_eq!(
            actual.targets.iter().copied().collect::<BTreeSet<_>>(),
            BTreeSet::from([RuleEntityKind::Skill, RuleEntityKind::Actor])
        );
        assert_eq!(actual.targets.len(), 2);
    }
    for intrinsic in &b.intrinsics {
        assert_program(
            &intrinsic.owner,
            INTRINSIC,
            RuleEntityKind::Skill,
            &[
                (
                    RuleEntity::Current,
                    &b.enabled,
                    ContributionKind::Flag,
                    "reviving",
                    ParameterValue::Boolean(true),
                ),
                (
                    RuleEntity::Current,
                    &b.base,
                    ContributionKind::Override,
                    "base-duration",
                    quantity(12.0, &b.seconds),
                ),
            ],
        );
        assert_eq!(
            program(&intrinsic.owner, INTRINSIC),
            program(&b.intrinsics[0].owner, INTRINSIC)
        );
    }
    for (subject, amount) in b.passives.iter().zip([5.0, 25.0, 5.0]) {
        assert_program(
            subject,
            SPEED,
            RuleEntityKind::Actor,
            &[(
                RuleEntity::Player,
                &b.increased,
                ContributionKind::Increase,
                "speed",
                quantity(amount, &b.percent),
            )],
        );
    }
    assert_ne!(b.passives[0], b.passives[2]);
    assert_eq!(
        program(&b.passives[0], SPEED),
        program(&b.passives[2], SPEED),
        "one shared body does not erase the two exact passive owners"
    );
    // Account for every production writer to these channels, not only selected
    // sources or a matching numerical example. No parent/default writer appears.
    let channels = BTreeSet::from([b.enabled.clone(), b.base.clone(), b.increased.clone()]);
    let mut writers = Vec::new();
    for owner in &rules().owners {
        for program in &owner.programs.members {
            for effect in &program.effects {
                if matches!(&effect.effect,
                    RuleEffectKind::Contribute { stat, .. } | RuleEffectKind::Derive { stat, .. }
                        if channels.contains(stat))
                {
                    let address = (owner.owner.clone(), program.id.clone(), effect.id.clone());
                    assert!(!writers.contains(&address));
                    writers.push(address);
                }
            }
        }
    }
    let expected: Vec<_> = b
        .intrinsics
        .iter()
        .flat_map(|intrinsic| {
            [
                (intrinsic.owner.clone(), key(INTRINSIC), key("reviving")),
                (
                    intrinsic.owner.clone(),
                    key(INTRINSIC),
                    key("base-duration"),
                ),
            ]
        })
        .chain(
            b.passives
                .iter()
                .map(|owner| (owner.clone(), key(SPEED), key("speed"))),
        )
        .collect();
    assert_eq!(writers.len(), expected.len());
    assert!(expected.iter().all(|address| writers.contains(address)));
    assert!(
        rules()
            .receivers
            .members
            .iter()
            .all(|receiver| !channels.contains(&receiver.stat))
    );
}

#[test]
fn component_execution_preserves_true_twelve_and_three_independent_player_contributions() {
    revival_source::check_proof();
    let b = bindings();
    let recipe = tracked_release().release().assembled().checked();
    let compiled = recipe.compiled_rules();
    let mut scratch = compiled.new_scratch();
    let cases: Vec<_> = b
        .intrinsics
        .iter()
        .map(|intrinsic| (&intrinsic.owner, INTRINSIC))
        .chain(b.passives.iter().map(|owner| (owner, SPEED)))
        .collect();
    let mut original = Vec::new();
    for order in [
        vec![0, 1, 2, 3, 4, 5, 6],
        vec![6, 5, 4, 3, 2, 1, 0],
        vec![0, 4, 1, 5, 2, 6, 3],
    ] {
        for index in order {
            let (owner, name) = cases[index];
            let result = compiled
                .evaluate(
                    owner,
                    &key(name),
                    &[],
                    recipe.schema().as_ref(),
                    &mut scratch,
                )
                .unwrap();
            let fresh = compiled
                .evaluate(
                    owner,
                    &key(name),
                    &[],
                    recipe.schema().as_ref(),
                    &mut compiled.new_scratch(),
                )
                .unwrap();
            assert_eq!(result, fresh);
            assert_eq!(&result.owner, owner);
            assert_eq!(result.program, key(name));
            assert_eq!(
                result.owner_programs_closure,
                self::owner(owner).programs.closure
            );
            assert!(matches!(
                result.owner_programs_closure,
                SchemaClosure::Partial { .. }
            ));
            let expected = if index < b.intrinsics.len() {
                vec![ParameterValue::Boolean(true), quantity(12.0, &b.seconds)]
            } else {
                vec![quantity(
                    [5.0, 25.0, 5.0][index - b.intrinsics.len()],
                    &b.percent,
                )]
            };
            assert_eq!(result.effects.len(), expected.len());
            for (effect, value) in result.effects.iter().zip(expected) {
                assert_eq!(effect.disposition, EffectDisposition::Applied { value });
                assert!(
                    matches!(&effect.effect, RuleEffectKind::Contribute { entity, .. }
                    if *entity == if index < b.intrinsics.len() { RuleEntity::Current } else { RuleEntity::Player })
                );
            }
            if original.len() < cases.len() {
                original.push(result);
            } else {
                assert_eq!(result, original[index]);
            }
        }
    }
    assert_eq!(original.len(), 7);
    for (index, result) in original.iter().enumerate() {
        assert!(
            original[..index]
                .iter()
                .all(|other| other.owner != result.owner)
        );
    }
    for result in &original[1..b.intrinsics.len()] {
        assert_eq!(original[0].effects, result.effects);
        assert_eq!(original[0].program, result.program);
    }
    assert_ne!(original[4].owner, original[6].owner);
    assert_eq!(original[4].effects, original[6].effects);
    assert_eq!(original[4].program, original[6].program);
    // A constant component result is not a ready complete owner or a sum over
    // selected occurrences. In particular the two five-percent sources survive.
    assert!(tracked_release().release().evaluation().is_none());
}

#[test]
fn exact_revival_queries_and_owner_gaps_remain_partial_without_a_consumer() {
    let b = bindings();
    let queries = rules().contribution_queries.as_ref().unwrap();
    let actual: Vec<_> = queries
        .members
        .iter()
        .filter(|query| QUERY_IDS.contains(&query.id.as_str()))
        .cloned()
        .collect();
    assert_eq!(actual, expected_queries());
    let SchemaClosure::Partial { gaps } = &queries.closure else {
        panic!("unknown total query domain")
    };
    assert_eq!(
        gaps.iter()
            .filter(|actual| **actual == gap(&b.enabled, "revival-query-domain-unproved"))
            .count(),
        1
    );
    for owner in &rules().owners {
        for program in &owner.programs.members {
            assert!(program.reads.iter().all(|read| !matches!(&read.source,
                RuleReadSource::ContributionQuery { query, .. } | RuleReadSource::ContributionSelection { query, .. }
                    if QUERY_IDS.contains(&query.as_str()))), "no revival consumer is authored in this tranche");
        }
    }
    let metadata = tracked_sources()
        .files
        .iter()
        .find(|(path, _)| path == "bootstrap/rules.json.gz")
        .unwrap();
    let mut bytes = Vec::new();
    GzDecoder::new(metadata.1.as_slice())
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 64 * 1024 * 1024);
    let baseline: RulePackageInput = serde_json::from_slice(&bytes).unwrap();
    for subject in b
        .intrinsics
        .iter()
        .map(|intrinsic| &intrinsic.owner)
        .chain(&b.passives)
    {
        let before = baseline
            .owners
            .iter()
            .find(|row| &row.owner == subject)
            .unwrap();
        assert!(!before.programs.is_complete());
        assert_eq!(owner(subject).programs.closure, before.programs.closure);
    }
    assert!(tracked_release().release().evaluation().is_none());
}

fn component() -> RulePackageInput {
    let b = bindings();
    let source = rules();
    let owners = b
        .intrinsics
        .iter()
        .map(|intrinsic| (&intrinsic.owner, INTRINSIC))
        .chain(b.passives.iter().map(|subject| (subject, SPEED)))
        .map(|(subject, name)| {
            let mut owner = owner(subject).clone();
            owner.programs.members = vec![program(subject, name).clone()];
            owner
        })
        .collect();
    RulePackageInput {
        schema_version: source.schema_version,
        namespace: source.namespace.clone(),
        release: source.release.clone(),
        semantics_version: source.semantics_version.clone(),
        operations_version: source.operations_version.clone(),
        definitions: source.definitions.clone(),
        tables: vec![],
        owners,
        receivers: DeclaredSet::complete(vec![]),
        effect_applications: Some(DeclaredSet::complete(vec![])),
        contribution_queries: Some(DeclaredSet::partial(
            expected_queries(),
            vec![gap(&b.enabled, "revival-query-domain-unproved")],
        )),
        existing_actor_rules: None,
        support_discovery: None,
    }
}

#[test]
fn checked_partial_queries_still_reject_crossed_sources_recipients_units_and_selection_defaults() {
    let b = bindings();
    let schema = tracked_release().release().assembled().schema();
    let valid = component();
    CompiledRulePackage::compile(&valid, schema, Default::default()).unwrap();
    let foreign = schema
        .input()
        .slots
        .iter()
        .find_map(|row| {
            let SlotDescriptor::SkillGrant(row) = row else {
                return None;
            };
            let SchemaState::Known(source) = &row.schema else {
                return None;
            };
            let SchemaSubject::Definition(DefinitionAddress::Skill(sniper)) =
                &b.intrinsics[0].owner
            else {
                unreachable!()
            };
            (source.skill != *sniper).then(|| row.id.clone())
        })
        .unwrap();
    for control in 0..8 {
        let mut bad = valid.clone();
        match control {
            0 => {
                let queries = bad.contribution_queries.as_mut().unwrap();
                let producer = queries.members[0].groups[0].members.members[0]
                    .producer
                    .as_program_effect_mut()
                    .unwrap();
                producer.origin = ContributionOrigin::Skill {
                    authored: false,
                    supplies: vec![foreign.clone()],
                };
            }
            1 => {
                let RuleEffectKind::Contribute { entity, .. } =
                    &mut bad.owners[0].programs.members[0].effects[0].effect
                else {
                    unreachable!()
                };
                *entity = RuleEntity::Player;
            }
            2 => bad.owners[b.intrinsics.len()].programs.members.clear(),
            3 => {
                let group = &mut bad.contribution_queries.as_mut().unwrap().members[2].groups[0];
                let duplicate = group.members.members[0].clone();
                group.members.members.push(duplicate);
            }
            4 => {
                bad.contribution_queries.as_mut().unwrap().members[2].groups[0].empty =
                    Some(quantity(0.0, &b.seconds))
            }
            5 => {
                bad.contribution_queries.as_mut().unwrap().members[1].groups[0].empty =
                    Some(quantity(0.0, &b.seconds))
            }
            6 => {
                // A valid sibling supply cannot authorize the same-valued producer.
                let producer = bad.contribution_queries.as_mut().unwrap().members[0].groups[0]
                    .members
                    .members[1]
                    .producer
                    .as_program_effect_mut()
                    .unwrap();
                producer.origin = ContributionOrigin::Skill {
                    authored: false,
                    supplies: vec![b.intrinsics[2].supply.clone()],
                };
            }
            7 => {
                let group = &mut bad.contribution_queries.as_mut().unwrap().members[1].groups[0];
                group.members.members.push(group.members.members[2].clone());
            }
            _ => unreachable!(),
        }
        assert!(
            CompiledRulePackage::compile(&bad, schema, Default::default()).is_err(),
            "control {control}"
        );
    }
    // No control fixes failure by promoting an owner/query closure or inventing
    // an absent incoming contribution as false, zero, or a neutral multiplier.
    assert!(
        valid
            .owners
            .iter()
            .all(|owner| !owner.programs.is_complete())
    );
    assert!(
        valid
            .contribution_queries
            .as_ref()
            .unwrap()
            .members
            .iter()
            .all(|query| !query.groups[0].members.is_complete())
    );
}
