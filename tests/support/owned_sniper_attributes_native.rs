//! Actual class/passive attribute bodies join the existing item-driven graph.
//! This finite numerical graph excludes ascendancy/tree legality and closes its
//! selected Class/global inventories only here. Production closures stay Partial.
use super::*;
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_core::owned_stages::{EvaluationStage, FrozenStageChannel, StageChannel};
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

const FACTORS: &str = "attribute-empty-factors";
const FIRST: &str = "attribute-first-pass";
const SECOND: &str = "attribute-second-pass";
#[derive(Clone)]
pub(super) struct Census {
    original: Vec<Allocation>,
    selected: Vec<Allocation>,
    actual_class: DefinitionRules,
    actual_registry: SchemaClosure,
    source_nodes: BTreeMap<PassiveNodeDefId, String>,
}
fn inner(w: &mut World) -> &mut shared::World {
    &mut w.sniper.base.source.base.inner
}
fn referenced(v: &Value, keys: &mut BTreeSet<String>) {
    match v {
        Value::Object(o) => {
            if o.contains_key("namespace")
                && let Some(k) = o.get("key").and_then(Value::as_str)
            {
                keys.insert(k.into());
            }
            for v in o.values() {
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
fn finite_descriptor(mut descriptor: DefinitionDescriptor) -> DefinitionDescriptor {
    match &mut descriptor {
        DefinitionDescriptor::Class(DefinitionEntry {
            schema: SchemaState::Known(s),
            ..
        }) => {
            s.ascendancies = DeclaredSet::complete(vec![]);
            s.implicit_passives = DeclaredSet::complete(vec![]);
        }
        DefinitionDescriptor::PassiveNode(DefinitionEntry {
            schema: SchemaState::Known(s),
            ..
        }) => {
            s.adjacent = DeclaredSet::complete(vec![]);
        }
        _ => {}
    }
    descriptor
}
pub(super) fn install(
    sniper: &mut sniper::World,
    endpoint: &StagedOwnedRelease,
    package: &Path,
    draft: &Value,
    selection: &Value,
) -> Census {
    attribute_base_family::assert_component(endpoint);
    let recipe = &endpoint.input().recipe;
    let replacements = attribute_base_family::replacements();
    let mut member_owners = Vec::new();
    for member in replacements
        .iter()
        .flat_map(|q| q.after.groups[0].members.members.iter())
    {
        if !member_owners.contains(&member.producer.as_program_effect().unwrap().owner) {
            member_owners.push(member.producer.as_program_effect().unwrap().owner.clone());
        }
    }
    let mut owners: Vec<_> = recipe
        .rules
        .owners
        .iter()
        .filter(|o| member_owners.contains(&o.owner))
        .cloned()
        .collect();
    assert_eq!(owners.len(), 355);
    let xml = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap();
    let normalized = passive_damage_evidence::normalized_selected_allocations(package, &xml);
    assert_eq!(normalized.len(), 55);
    let original: Vec<Allocation> = normalized
        .iter()
        .filter(|a| {
            a.node
                .to_resolved()
                .is_some_and(|node| member_owners.contains(&subject(node)))
        })
        .map(|a| {
            a.to_resolved()
                .expect("actual selected attribute occurrence inputs")
        })
        .collect();
    assert_eq!(original.len(), 22);
    assert!(
        original
            .iter()
            .all(|a| a.access == AllocationAccess::Ordinary && a.scope == LoadoutScope::Shared)
    );
    // Character authority comes from the fresh canonical imported selection,
    // including its actual level before shared Player programs are installed.
    let character = draft["draft"]["character_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == selection["build"]["character"])
        .unwrap();
    assert_eq!(character["class"]["kind"], "known");
    let class: ClassDefId = decode(&character["class"]["value"]);
    assert_eq!(character["level"]["kind"], "known");
    let level = decode(&character["level"]["value"]);
    let actual_class = owners
        .iter()
        .find(|o| o.owner == subject(class.clone()))
        .unwrap()
        .clone();
    assert!(!actual_class.programs.is_complete());
    // Existing complete final-attribute receivers and guarded empty-MORE bodies.
    // No factor literal or observed final attribute is supplied to the graph.
    let input_ids: BTreeSet<_> = (0..6).map(|i| d::<StatDefinition>(0x331b + i)).collect();
    let output_ids: BTreeSet<_> = (0..3)
        .map(|i| d::<StatDefinition>(0x1d2e + i))
        .chain((0..9).map(|i| d::<StatDefinition>(0x3321 + i)))
        .collect();
    for owner in &recipe.rules.owners {
        if matches!(&owner.owner, SchemaSubject::Definition(DefinitionAddress::Stat(s)) if output_ids.contains(s))
        {
            assert!(owner.programs.is_complete());
            owners.push(owner.clone());
        }
    }
    assert_eq!(owners.len(), 367);
    let receivers: Vec<_> = recipe
        .rules
        .receivers
        .members
        .iter()
        .filter(|r| output_ids.contains(&r.stat))
        .cloned()
        .collect();
    assert_eq!(receivers.len(), 12);
    let queries: Vec<_> = recipe
        .rules
        .contribution_queries
        .as_ref()
        .unwrap()
        .members
        .iter()
        .filter(|q| input_ids.contains(&q.stat))
        .cloned()
        .collect();
    assert_eq!(queries.len(), 18);
    assert!(
        queries
            .iter()
            .all(|q| q.groups.iter().all(|g| g.members.is_complete()))
    );
    // INC queries keep their complete current membership too. Their producers
    // need not overlap the BASE set, even if Original05 selects none of them.
    for member in queries
        .iter()
        .flat_map(|q| &q.groups)
        .flat_map(|g| &g.members.members)
    {
        if !owners
            .iter()
            .any(|o| o.owner == member.producer.as_program_effect().unwrap().owner)
        {
            let actual = recipe
                .rules
                .owners
                .iter()
                .find(|o| o.owner == member.producer.as_program_effect().unwrap().owner)
                .unwrap();
            assert!(actual.programs.is_complete());
            owners.push(actual.clone());
        }
    }
    let tree = serde_json::json!(endpoint.input().tree);
    let mut source_nodes = BTreeMap::new();
    for allocation in &original {
        let rows: Vec<_> = tree["content"]["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| {
                r["role"]["kind"] == "allocation"
                    && r["role"]["value"]["node"] == serde_json::json!(allocation.node)
            })
            .collect();
        assert_eq!(
            rows.len(),
            1,
            "exact owned definition to source node identity"
        );
        assert!(
            source_nodes
                .insert(
                    allocation.node.clone(),
                    format!("Tree:{}", rows[0]["token"].as_str().unwrap())
                )
                .is_none()
        );
    }
    let f = &mut sniper.base.source.base.inner;
    let mut keys = BTreeSet::new();
    referenced(&serde_json::json!(owners), &mut keys);
    referenced(&serde_json::json!(queries), &mut keys);
    referenced(&serde_json::json!(receivers), &mut keys);
    referenced(&serde_json::json!(original), &mut keys);
    // Include exact current schema dependencies for every declared query member,
    // including unselected classes/passives. This does not trim membership to
    // whichever example happens to execute.
    let mut definitions = Vec::new();
    let mut slots = Vec::new();
    loop {
        let before = keys.len();
        for actual in &recipe.schema.definitions {
            if keys.contains(actual.address().key().as_str())
                && !definitions
                    .iter()
                    .any(|d: &DefinitionDescriptor| d.address() == actual.address())
            {
                let finite = finite_descriptor(actual.clone());
                referenced(&serde_json::json!(finite), &mut keys);
                definitions.push(finite);
            }
        }
        for actual in &recipe.schema.slots {
            if keys.contains(actual.address().key().as_str())
                && !slots
                    .iter()
                    .any(|s: &SlotDescriptor| s.address() == actual.address())
            {
                referenced(&serde_json::json!(actual), &mut keys);
                slots.push(actual.clone());
            }
        }
        if keys.len() == before {
            break;
        }
        assert!(keys.len() < 2000, "finite attribute dependency closure");
    }
    for definition in definitions {
        let address = definition.address();
        if let Some(existing) = f.schema.definitions.iter().find(|d| d.address() == address) {
            assert_eq!(existing, &definition);
        } else {
            f.schema.definitions.push(definition);
        }
        f.owner_mut(SchemaSubject::Definition(address));
    }
    for slot in slots {
        if let Some(existing) = f
            .schema
            .slots
            .iter()
            .find(|s| s.address() == slot.address())
        {
            assert_eq!(existing, &slot);
        } else {
            f.schema.slots.push(slot);
        }
    }
    for mut owner in owners {
        if matches!(
            owner.owner,
            SchemaSubject::Definition(DefinitionAddress::Class(_))
        ) {
            // Numerical fixture only: keep every actual body; actual Class
            // coverage is separately restored in the mandatory refusal control.
            owner.programs.closure = SchemaClosure::Complete;
        }
        let selected = f.owner_mut(owner.owner.clone());
        assert!(selected.programs.members.is_empty());
        *selected = owner;
    }
    f.build.character.class = class;
    f.build.character.level = level;
    let selected: Vec<_> = original
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let mut a = a.clone();
            a.id = id(7500 + i as u64);
            a
        })
        .collect();
    for a in &selected {
        assert!(
            !f.build
                .allocations
                .iter()
                .any(|old| old.id == a.id || old.node == a.node)
        );
        f.build.allocations.push(a.clone());
    }
    for receiver in receivers {
        assert!(!sniper.receivers.members.iter().any(|r| r.id == receiver.id));
        sniper.receivers.members.push(receiver);
    }
    for query in queries {
        assert!(
            !sniper
                .base
                .contribution_queries
                .members
                .iter()
                .any(|q| q.id == query.id)
        );
        sniper.base.contribution_queries.members.push(query);
    }
    Census {
        original,
        selected,
        actual_class,
        actual_registry: recipe
            .rules
            .contribution_queries
            .as_ref()
            .unwrap()
            .closure
            .clone(),
        source_nodes,
    }
}
pub(super) fn configure(stages: &mut EvaluationStagesInput) {
    stages.stages.extend([
        EvaluationStage {
            id: key(FACTORS),
            predecessors: vec![key("deliver")],
        },
        EvaluationStage {
            id: key(FIRST),
            predecessors: vec![key(FACTORS)],
        },
        EvaluationStage {
            id: key(SECOND),
            predecessors: vec![key(FIRST)],
        },
    ]);
    for row in &mut stages.programs.members {
        if row.program.as_str().ends_with("-empty-more") {
            row.stage = key(FACTORS);
        }
        if row.program.as_str().ends_with("-first-step") {
            row.stage = key(FIRST);
        }
        if row.program.as_str().ends_with("-second-step") {
            row.stage = key(SECOND);
        }
    }
    for i in 0..6 {
        for contribution in [
            ContributionKind::Add,
            ContributionKind::Increase,
            ContributionKind::Multiply,
        ] {
            stages.frozen_channels.push(FrozenStageChannel {
                channel: StageChannel::Contributions {
                    scope: RuleEntityKind::Actor,
                    stat: d(0x331b + i),
                    contribution,
                },
                stage: key("deliver"),
            });
        }
        stages.frozen_channels.push(FrozenStageChannel {
            channel: StageChannel::Stat {
                scope: RuleEntityKind::Actor,
                stat: d(0x3324 + i),
            },
            stage: key(FACTORS),
        });
    }
    for i in 0..3 {
        stages.frozen_channels.push(FrozenStageChannel {
            channel: StageChannel::Stat {
                scope: RuleEntityKind::Actor,
                stat: d(0x3321 + i),
            },
            stage: key(FIRST),
        });
    }
}
fn values(report: &SupportEffectsReport) -> [i64; 6] {
    let r = sniper::offering::effects(report);
    std::array::from_fn(|i| {
        let stat = d(if i < 3 {
            0x3321 + i as u64
        } else {
            0x1d2e + i as u64 - 3
        });
        let rows: Vec<_> = r
            .values
            .iter()
            .filter(|v| {
                v.key
                    == PlanValueKey::Stat {
                        entity: ConcreteEntity::Actor(ActorKey::Player),
                        stat: stat.clone(),
                    }
            })
            .collect();
        assert_eq!(rows.len(), 1);
        let EffectValue::Known {
            value: ParameterValue::Integer(value),
        } = &rows[0].value
        else {
            panic!("known final attribute: {:?}", rows[0])
        };
        value.get()
    })
}
fn check(w: &World, report: &SupportEffectsReport) {
    assert_eq!(values(report), [27, 7, 105, 27, 7, 105]);
    let r = sniper::offering::effects(report);
    for (original, selected) in w.attributes.original.iter().zip(&w.attributes.selected) {
        let mut expected = original.clone();
        expected.id = selected.id;
        assert_eq!(*selected, expected);
    }
    let vectors: Value = attribute_base_family::read("source-vectors.json");
    let stages: Vec<_> = vectors["stages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["case"] == "original-05")
        .cloned()
        .collect();
    check_records(w, report, &stages);
    assert!(r.values.iter().filter(|v|matches!(&v.key,PlanValueKey::Stat {stat,..} if (0..3).any(|i| *stat==d(0x1d2e+i)))).all(|v|matches!(&v.key,PlanValueKey::Stat {entity:ConcreteEntity::Actor(ActorKey::Player),..})));
}
fn check_records(w: &World, report: &SupportEffectsReport, stages: &[Value]) {
    let r = sniper::offering::effects(report);
    assert_eq!(stages.len(), 12);
    for pass in 0..2 {
        let rows:Vec<_>=r.effects.iter().filter(|e| matches!(&e.target,BoundEffectTarget::Contribution {key} if key.kind==ContributionKind::Add && (0..3).any(|i| key.stat==d(0x331b+pass*3+i)))).collect();
        assert_eq!(
            rows.len(),
            69,
            "three class and 22 three-way passive effects per pass"
        );
        assert_eq!(
            rows.iter()
                .filter(|e| e.value == EffectValue::Inactive)
                .count(),
            44
        );
        assert!(rows.iter().all(|e| matches!(&e.key.invocation.origin,RuleOrigin::Provider {provider} if matches!(provider.root,ProviderRoot::Character|ProviderRoot::Allocation(_)) && provider.grant_path.is_empty())));
        let known: Vec<_> = rows
            .iter()
            .filter(|e| matches!(e.value, EffectValue::Known { .. }))
            .collect();
        assert_eq!(
            known.len(),
            25,
            "three class +22selected choice contributions perpass"
        );
        for (i, name) in ["Str", "Dex", "Int"].into_iter().enumerate() {
            let mut actual = Vec::new();
            for row in &known {
                let BoundEffectTarget::Contribution { key: target } = &row.target else {
                    unreachable!()
                };
                if target.stat != d(0x331b + pass * 3 + i as u64) {
                    continue;
                }
                let RuleOrigin::Provider { provider } = &row.key.invocation.origin else {
                    unreachable!()
                };
                let source = match provider.root {
                    ProviderRoot::Character => "Base".to_owned(),
                    ProviderRoot::Allocation(id) => {
                        let node = &w
                            .attributes
                            .selected
                            .iter()
                            .find(|a| a.id == id)
                            .unwrap()
                            .node;
                        w.attributes.source_nodes[node].clone()
                    }
                    _ => panic!("unadmitted producer"),
                };
                let EffectValue::Known {
                    value: ParameterValue::Quantity(amount),
                } = &row.value
                else {
                    panic!("Count")
                };
                assert_eq!(amount.unit(), &d(0x295a));
                actual.push((source, amount.value() as i64));
            }
            actual.sort();
            for mode in ["MAIN", "CALCS"] {
                let stage = stages
                    .iter()
                    .find(|s| s["mode"] == mode && s["pass"] == pass + 1 && s["stat"] == name)
                    .unwrap();
                let mut expected: Vec<_> = stage["records"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| {
                        (
                            r["source"].as_str().unwrap().to_owned(),
                            r["value"].as_i64().unwrap(),
                        )
                    })
                    .collect();
                expected.sort();
                assert_eq!(
                    actual, expected,
                    "exact original source/value multiset in each independently retained pass/mode"
                );
            }
        }
    }
}

fn strength_choice_to_dexterity(xml: &str) -> String {
    // Reproduce the retained source witness's exact single-node XML edit through
    // the importer evidence API. All other input bytes and selected nodes stay.
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([119; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let trees: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|r| r.occurrence().name() == "Tree")
        .collect();
    assert_eq!(trees.len(), 1);
    let tree = trees[0];
    let active: usize = tree
        .attribute("activeSpec")
        .unwrap()
        .decoded()
        .unwrap()
        .parse()
        .unwrap();
    let spec = evidence
        .rows()
        .iter()
        .filter(|r| {
            r.occurrence().parent() == Some(tree.occurrence().id())
                && r.occurrence().name() == "Spec"
        })
        .nth(active - 1)
        .unwrap();
    assert_eq!(
        spec.attribute("nodes")
            .unwrap()
            .decoded()
            .unwrap()
            .split(',')
            .filter(|n| *n == "15782")
            .count(),
        1
    );
    let containers: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|r| {
            r.occurrence().parent() == Some(spec.occurrence().id())
                && r.occurrence().name() == "Overrides"
        })
        .collect();
    assert_eq!(containers.len(), 1);
    let overrides: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|r| {
            r.occurrence().parent() == Some(containers[0].occurrence().id())
                && r.occurrence().name() == "AttributeOverride"
        })
        .collect();
    assert_eq!(overrides.len(), 1);
    let row = overrides[0];
    let strength = row.attribute("strNodes").unwrap().decoded().unwrap();
    let dexterity = row.attribute("dexNodes").unwrap().decoded().unwrap();
    assert!(dexterity.is_empty());
    assert_eq!(strength.split(',').filter(|n| *n == "15782").count(), 1);
    let remaining = strength
        .split(',')
        .filter(|n| *n != "15782")
        .collect::<Vec<_>>()
        .join(",");
    let range = row.occurrence().range();
    let original = &xml[range.clone()];
    let from = format!("strNodes=\"{strength}\"");
    assert_eq!(original.matches(&from).count(), 1);
    assert_eq!(original.matches("dexNodes=\"\"").count(), 1);
    let changed = original
        .replacen(&from, &format!("strNodes=\"{remaining}\""), 1)
        .replacen("dexNodes=\"\"", "dexNodes=\"15782\"", 1);
    let mut result = xml.to_owned();
    result.replace_range(range.clone(), &changed);
    let mut inverse = result.clone();
    inverse.replace_range(range.start..range.start + changed.len(), original);
    assert_eq!(inverse, xml);
    result
}

#[test]
#[ignore = "requires current SNIPER_ITEM_ATTACK_RELEASE with bounded BASE membership"]
fn actual_imported_attribute_choice_changes_exactly_one_source_in_both_passes() {
    let mut w = World::load();
    let vectors: Value = attribute_base_family::read("source-vectors.json");
    let expected = &vectors["choice_control"];
    let original = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap();
    let changed = strength_choice_to_dexterity(&original);
    assert_eq!(
        format!("{:x}", Sha256::digest(changed.as_bytes())),
        expected["xml_sha256"]
    );
    let imported = passive_damage_evidence::normalized_selected_allocations(&path(), &changed);
    assert_eq!(
        imported.len(),
        55,
        "choice edit does not delete saved allocations"
    );
    let changed_node = w
        .attributes
        .source_nodes
        .iter()
        .find(|(_, source)| source.as_str() == "Tree:15782")
        .unwrap()
        .0
        .clone();
    let mut differences = Vec::new();
    let selected: Vec<_> = w
        .attributes
        .selected
        .iter()
        .map(|before| {
            let mut after = imported
                .iter()
                .find(|a| a.node.to_resolved().as_ref() == Some(&before.node))
                .unwrap()
                .to_resolved()
                .unwrap();
            after.id = before.id;
            if &after != before {
                differences.push(after.node.clone());
                let mut inverse = after.clone();
                inverse.choices = before.choices.clone();
                assert_eq!(&inverse, before, "only the authored choice changes");
            }
            after
        })
        .collect();
    assert_eq!(differences, vec![changed_node]);
    for a in &selected {
        *inner(&mut w)
            .build
            .allocations
            .iter_mut()
            .find(|b| b.id == a.id)
            .unwrap() = a.clone();
    }
    w.attributes.selected = selected;
    let report = w.evaluate();
    assert_eq!(values(&report), [22, 12, 105, 22, 12, 105]);
    let mut stages = Vec::new();
    for mode in ["MAIN", "CALCS"] {
        for row in expected["modes"][mode]["stages"].as_array().unwrap() {
            let mut row = row.clone();
            row["mode"] = json!(mode);
            stages.push(row);
        }
    }
    check_records(&w, &report, &stages);
}
#[test]
#[ignore = "requires current SNIPER_ITEM_ATTACK_RELEASE with bounded BASE membership"]
fn actual_class_and_selected_passives_produce_both_attribute_passes_in_joined_graph() {
    let w = World::load();
    let report = w.evaluate();
    check(&w, &report);
    // Existing intrinsic attack is still downstream of its actual item inputs.
    let rows = evidence::checked_cases();
    let original = rows.iter().find(|r| r["physical_level"] == 20).unwrap();
    w.check(&report, [original, original]);
}
#[test]
#[ignore = "requires current SNIPER_ITEM_ATTACK_RELEASE with bounded BASE membership"]
fn duplicate_passive_occurrences_are_rejected_before_reduction() {
    let mut w = World::load();
    let mut a = w.attributes.selected[0].clone();
    a.id = id(7600);
    inner(&mut w).build.allocations.push(a);
    let error = w
        .checked_plan()
        .err()
        .expect("duplicate member cannot raise the finite sum bound");
    assert!(
        error.contains("ordered contribution semantic positions are tied"),
        "{error}"
    );
}
#[test]
#[ignore = "requires current SNIPER_ITEM_ATTACK_RELEASE with bounded BASE membership"]
fn actual_class_and_global_partial_coverage_still_refuse_complete_evaluation() {
    for class in [true, false] {
        let mut w = World::load();
        if class {
            let actual = w.attributes.actual_class.clone();
            let owner = actual.owner.clone();
            *inner(&mut w).owner_mut(owner) = actual;
        } else {
            w.sniper.base.contribution_queries.closure = w.attributes.actual_registry.clone();
        }
        let p = w.plan();
        assert!(!p.gaps().is_empty());
        assert_eq!(
            p.evaluate(&mut p.new_scratch()).unwrap().outcome,
            SupportEffectsOutcome::Unavailable {
                cause: EffectValue::Unresolved {
                    reason: PlanGapReason::IncompleteContributors,
                    read: None
                },
                input: None
            }
        );
    }
}
fn unlisted(w: &mut World, active: bool, value: f64) {
    let class = inner(w).build.character.class.clone();
    inner(w)
        .owner_mut(subject(class))
        .programs
        .members
        .push(RuleProgram {
            id: key("fixture-unsupported-base"),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![
                RuleNode {
                    id: key("value"),
                    expression: RuleExpression::Literal {
                        value: quantity(value, &d(0x295a)),
                    },
                },
                RuleNode {
                    id: key("active"),
                    expression: RuleExpression::Literal {
                        value: ParameterValue::Boolean(active),
                    },
                },
            ],
            effects: vec![RuleEffect {
                id: key("unsupported"),
                when: Some(key("active")),
                effect: RuleEffectKind::Contribute {
                    entity: RuleEntity::Player,
                    stat: d(0x331b),
                    contribution: ContributionKind::Add,
                    value: key("value"),
                },
            }],
        });
}
#[test]
#[ignore = "requires current SNIPER_ITEM_ATTACK_RELEASE with bounded BASE membership"]
fn unlisted_zero_negative_and_inactive_sources_cannot_enter_the_bounded_domain() {
    for (active, value) in [(true, 0.), (false, 0.), (false, -9.), (true, -9.)] {
        let mut w = World::load();
        unlisted(&mut w, active, value);
        let error = w
            .checked_plan()
            .err()
            .expect("unsupported source must fail before activation/value");
        assert!(
            error.contains("actual contribution has no declared membership"),
            "{error}"
        );
    }
    // The actual equipped Crown modifier supplies this counterfactual source.
    // It is not a game conversion: even its inactive zero cannot be mistaken
    // for an empty item BASE domain or disappear after value filtering.
    let mut w = World::load();
    unlisted(&mut w, false, 0.);
    let class = inner(&mut w).build.character.class.clone();
    let mut program = inner(&mut w)
        .owner_mut(subject(class))
        .programs
        .members
        .pop()
        .unwrap();
    assert_eq!(program.id, key("fixture-unsupported-base"));
    program.context = RuleEntityKind::EquipmentUse;
    inner(&mut w)
        .owner_mut(subject(d::<ModifierDefinition>(0x30ca)))
        .programs
        .members
        .push(program);
    let error = w
        .checked_plan()
        .err()
        .expect("inactive actual item occurrence remains a potential contributor");
    assert!(
        error.contains("actual contribution has no declared membership"),
        "{error}"
    );
}
#[test]
#[ignore = "requires current SNIPER_ITEM_ATTACK_RELEASE with bounded BASE membership"]
fn attribute_order_reversal_and_reused_rayon_scratch_preserve_exact_values() {
    let w = World::load();
    let p = w.plan();
    let mut scratch = p.new_scratch();
    let a = p.evaluate(&mut scratch).unwrap();
    check(&w, &a);
    let mut reversed = w.clone();
    inner(&mut reversed).build.allocations.reverse();
    inner(&mut reversed).owners.reverse();
    let q = reversed.plan();
    assert_eq!(values(&q.evaluate(&mut scratch).unwrap()), values(&a));
    let mut removed = w.clone();
    let selected = removed.attributes.selected[0].id;
    inner(&mut removed)
        .build
        .allocations
        .retain(|a| a.id != selected);
    let changed = removed.plan().evaluate(&mut scratch).unwrap();
    assert_ne!(values(&changed), values(&a));
    assert_eq!(p.evaluate(&mut scratch).unwrap(), a);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    pool.install(|| {
        (0..8).into_par_iter().for_each_init(
            || p.new_scratch(),
            |s, _| assert_eq!(p.evaluate(s).unwrap(), a),
        )
    });
}
