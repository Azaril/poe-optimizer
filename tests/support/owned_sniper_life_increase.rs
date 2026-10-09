//! Six actual selected passive sources feed the published Sniper Life receiver.
//! Only topology is projected away for this finite numerical graph. The real
//! Actor owner and global contributor inventories remain Partial; no final Life.
#[allow(dead_code)]
#[path = "owned_minion_life_increase.rs"]
mod family;
use super::*;
use poe_optimizer_core::owned_stages::{EvaluationStage, FrozenStageChannel, StageChannel};
use poe_optimizer_import::{
    owned_release::StagedOwnedRelease, owned_release_migration::OwnedReleaseMigrationInput,
    owned_tree_policy::TreeTokenRole,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::Path};

const PRODUCER: &str = "ordinary-minion-life";
const RECEIVED: &str = "received-minion-life-increase";
pub(super) const RECEIVE_STAGE: &str = "minion-life-increase";

#[derive(Clone)]
struct Source {
    source_id: String,
    original: Allocation,
    selected: Allocation,
    amount: f64,
}
#[derive(Clone)]
pub(super) struct Census {
    sources: Vec<Source>,
    expected: f64,
}
fn actor_owner() -> SchemaSubject {
    SchemaSubject::Slot(SlotAddress::Actor(actor_slot()))
}

pub(super) fn install(
    sniper: &mut sniper::World,
    endpoint: &StagedOwnedRelease,
    package: &Path,
) -> Census {
    family::check_authored();
    let recipe = &endpoint.input().recipe;
    let bindings: Value = family::read("bindings.json");
    let dependencies: Value = family::read("dependencies.json");
    let vectors: Value = family::read("source-vectors.json");
    let migration: OwnedReleaseMigrationInput = family::read("migration.json");
    let actual_actor = recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == actor_owner())
        .unwrap();
    assert!(!actual_actor.programs.is_complete());
    assert_eq!(migration.owners.len(), 1);
    assert_eq!(migration.owners[0].owner, actor_owner());
    assert_eq!(
        actual_actor.programs.closure, migration.owners[0].programs.closure,
        "the replay refusal must restore the actual production coverage"
    );
    assert_eq!(migration.owners[0].programs.members.len(), 1);
    let received = &migration.owners[0].programs.members[0];
    assert_eq!(received.id, key(RECEIVED));
    assert_eq!(
        actual_actor
            .programs
            .members
            .iter()
            .filter(|p| p.id == received.id)
            .collect::<Vec<_>>(),
        [received]
    );
    assert!(
        endpoint
            .receipt()
            .provenance
            .iter()
            .any(|p| p.kind.as_str() == family::KIND)
    );

    let xml = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap();
    // Authenticate the retained source reports, including this exact unchanged
    // source input and the observed transfer. Their historical observer identity
    // remains in the receipt; it is not replaced by today's observer file hash.
    let mut first_report = None;
    for pin in vectors["reports"].as_array().unwrap() {
        let bytes = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(pin["path"].as_str().unwrap()),
        )
        .unwrap();
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), pin["sha256"]);
        if let Some(previous) = &first_report {
            assert_eq!(previous, &bytes);
        } else {
            first_report = Some(bytes);
        }
    }
    let report: Value = serde_json::from_slice(first_report.as_ref().unwrap()).unwrap();
    assert_eq!(report["cases"][0]["name"], "original-05");
    assert_eq!(
        report["cases"][0]["xml_sha256"],
        format!("{:x}", Sha256::digest(xml.as_bytes()))
    );
    for observation in vectors["observations"].as_array().unwrap() {
        let actual = report
            .pointer(observation["pointer"].as_str().unwrap())
            .unwrap();
        for field in [
            "source_occurrence",
            "actor_profile",
            "is_environment_minion",
            "life_delivery",
        ] {
            assert_eq!(actual[field], observation["value"][field]);
        }
    }
    let original = passive_damage_evidence::normalized_selected_allocations(package, &xml);
    assert_eq!(original.len(), 55);
    let owners: Vec<DefinitionRules> = decode(&dependencies["passive_owners"]);
    assert_eq!(owners.len(), 6);
    let f = &mut sniper.base.source.base.inner;
    let mut sources = Vec::new();
    let mut added = 0;
    for (binding, expected_owner) in bindings["nodes"].as_array().unwrap().iter().zip(owners) {
        let node: PassiveNodeDefId = decode(&binding["definition"]);
        let source_id = binding["source_id"].as_str().unwrap();
        let selected: Vec<_> = original
            .iter()
            .filter(|a| a.node.to_resolved() == Some(node.clone()))
            .collect();
        assert_eq!(selected.len(), 1);
        let original = selected[0]
            .to_resolved()
            .expect("actual selected Life source has resolved access and inputs");
        assert_eq!(original.access, AllocationAccess::Ordinary);
        assert_eq!(original.scope, LoadoutScope::Shared);
        assert!(original.choices.is_empty());
        let tokens: Vec<_> = endpoint
            .tree()
            .unwrap()
            .input()
            .content
            .tokens
            .iter()
            .filter(|t| t.token == source_id)
            .collect();
        assert_eq!(tokens.len(), 1);
        assert!(
            matches!(&tokens[0].role, TreeTokenRole::Allocation {node: n,pool} if n == &node && pool == &original.pool)
        );
        assert_eq!(expected_owner.owner, subject(node.clone()));
        assert!(expected_owner.programs.is_complete());
        assert_eq!(
            recipe
                .rules
                .owners
                .iter()
                .filter(|o| o.owner == expected_owner.owner)
                .collect::<Vec<_>>(),
            [&expected_owner]
        );
        let actual_descriptor = recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == node.address())
            .unwrap();
        let mut finite = actual_descriptor.clone();
        let DefinitionDescriptor::PassiveNode(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) = &mut finite
        else {
            panic!("known actual passive")
        };
        assert!(schema.pools.is_complete());
        assert_eq!(
            schema.pools.members.as_slice(),
            std::slice::from_ref(&original.pool)
        );
        // Preserve every real declaration, pool and default body. This fixture
        // does not claim that the projected numerical tree is connected/legal.
        schema.adjacent = DeclaredSet::complete(vec![]);
        let selected = if let Some(existing) = f.build.allocations.iter().find(|a| a.node == node) {
            assert!(
                sources.len() < 4,
                "the four paired sources must already exist"
            );
            let mut rehomed = original.clone();
            rehomed.id = existing.id;
            assert_eq!(&rehomed, existing);
            assert!(f.owners.contains(&expected_owner));
            assert!(f.schema.definitions.contains(&finite));
            existing.clone()
        } else {
            assert!(
                sources.len() >= 4,
                "never replace an existing paired producer"
            );
            assert!(
                !f.schema
                    .definitions
                    .iter()
                    .any(|d| d.address() == finite.address())
            );
            assert!(!f.owners.iter().any(|o| o.owner == expected_owner.owner));
            f.schema.definitions.push(finite);
            f.owners.push(expected_owner);
            let mut rehomed = original.clone();
            rehomed.id = id(8400 + added);
            assert!(!f.build.allocations.iter().any(|a| a.id == rehomed.id));
            f.build.allocations.push(rehomed.clone());
            added += 1;
            rehomed
        };
        assert_ne!(selected.id, original.id, "explicit fixture-lineage remap");
        sources.push(Source {
            source_id: source_id.into(),
            original,
            selected,
            amount: binding["amount"].as_f64().unwrap(),
        });
    }
    assert_eq!(added, 2);
    for descriptor in decode::<Vec<DefinitionDescriptor>>(&dependencies["supporting_definitions"]) {
        assert!(recipe.schema.definitions.contains(&descriptor));
        if matches!(
            descriptor,
            DefinitionDescriptor::Actor(_) | DefinitionDescriptor::PassiveNode(_)
        ) {
            // Actor topology is the parent's selected Basic/Gas component;
            // passive topology is the explicit projection checked above.
            // Neither projection replaces the authentic production descriptor.
            continue;
        }
        assert!(matches!(
            descriptor,
            DefinitionDescriptor::Stat(_) | DefinitionDescriptor::Unit(_)
        ));
        if let Some(existing) = f
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == descriptor.address())
        {
            assert_eq!(existing, &descriptor);
        } else {
            f.owner_mut(SchemaSubject::Definition(descriptor.address()));
            f.schema.definitions.push(descriptor);
        }
    }
    let owner = f.owner_mut(actor_owner());
    assert!(!owner.programs.members.iter().any(|p| p.id == received.id));
    owner.programs.members.push(received.clone());
    let native = &vectors["native_cases"][0];
    assert_eq!(native["name"], "original-05");
    let actual: BTreeSet<_> = sources.iter().map(|s| s.source_id.as_str()).collect();
    assert_eq!(
        actual,
        native["eligible_sources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect()
    );
    let expected = native["received_increase"].as_f64().unwrap();
    assert_eq!(sources.iter().map(|s| s.amount).sum::<f64>(), expected);
    Census { sources, expected }
}

pub(super) fn configure(stages: &mut EvaluationStagesInput) {
    stages.stages.push(EvaluationStage {
        id: key(RECEIVE_STAGE),
        predecessors: vec![key("deliver")],
    });
    let mut count = 0;
    for row in &mut stages.programs.members {
        if row.owner == actor_owner() && row.program == key(RECEIVED) {
            row.stage = key(RECEIVE_STAGE);
            count += 1;
        }
    }
    assert_eq!(count, 1);
    stages.frozen_channels.push(FrozenStageChannel {
        channel: StageChannel::Contributions {
            scope: RuleEntityKind::Actor,
            stat: d(0x32e5),
            contribution: ContributionKind::Increase,
        },
        stage: key("deliver"),
    });
}

pub(super) fn check(w: &World, report: &SupportEffectsReport) {
    let r = sniper::offering::effects(report);
    let f = &w.sniper.base.source.base.inner;
    let sources = &w.life_increase.sources;
    let rows: Vec<_> = r
        .effects
        .iter()
        .filter(
            |e| matches!(&e.target, BoundEffectTarget::Contribution {key} if key.stat == d(0x32e5)),
        )
        .collect();
    assert_eq!(rows.len(), 6);
    let mut witnessed = BTreeSet::new();
    for row in rows {
        let RuleOrigin::Provider { provider } = &row.key.invocation.origin else {
            panic!("actual allocation provider")
        };
        let ProviderRoot::Allocation(id) = provider.root else {
            panic!("passive source")
        };
        assert!(provider.grant_path.is_empty() && witnessed.insert(id));
        let source = sources.iter().find(|s| s.selected.id == id).unwrap();
        assert!(f.build.allocations.contains(&source.selected));
        let mut original = source.original.clone();
        original.id = source.selected.id;
        assert_eq!(original, source.selected);
        assert_eq!(
            row.key.invocation.owner,
            subject(source.selected.node.clone())
        );
        assert_eq!(row.key.invocation.program, key(PRODUCER));
        assert_eq!(
            row.target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: d(0x32e5),
                    kind: ContributionKind::Increase
                }
            }
        );
        assert_eq!(
            row.value,
            EffectValue::Known {
                value: quantity(source.amount, &d(2))
            }
        );
    }
    let received: Vec<_> = r
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(RECEIVED))
        .collect();
    assert_eq!(received.len(), 2);
    for index in 0..2 {
        let actor = w.sniper.actor(index);
        let rows: Vec<_> = received.iter().filter(|e| matches!(&e.target, BoundEffectTarget::Contribution {key} if key.entity == ConcreteEntity::Actor(actor.clone()))).collect();
        assert_eq!(rows.len(), 1);
        let row = rows[0];
        let ActorKey::Owned(owned) = actor.clone() else {
            panic!()
        };
        let mut provider = owned.provider;
        provider
            .grant_path
            .push(slot(SlotOwnerDefId::Skill(d(0x12)), 0x20));
        assert_eq!(row.key.invocation.origin, RuleOrigin::Provider { provider });
        assert_eq!(row.key.invocation.owner, actor_owner());
        assert_eq!(
            row.target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(actor),
                    stat: d(0x311a),
                    kind: ContributionKind::Increase
                }
            }
        );
        assert_eq!(
            row.value,
            EffectValue::Known {
                value: quantity(w.life_increase.expected, &d(2))
            }
        );
    }
    assert!(
        !r.values
            .iter()
            .any(|v| matches!(&v.key, PlanValueKey::Stat {stat,..} if *stat == d(0x311a))),
        "no final Life claim"
    );
}
