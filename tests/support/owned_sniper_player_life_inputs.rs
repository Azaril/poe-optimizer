//! Real selected Life rolls and quest selections enter the existing joined graph.
//! This is finite component coverage: template programs unrelated to catalyst
//! inputs or ordinary applicability and non-Life modifiers remain outside the fixture. The actual Partial
//! owners are retained for refusal controls. No Life pool/reduction is supplied.
use super::*;
use poe_optimizer_core::{
    build_identity::{BuildLineage, ItemRecordId},
    owned_draft::{EquipmentDraft, ItemDraft, RewardDraft},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_release::StagedOwnedRelease,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::Path};

const DELIVERY: &str = "contribute-player-flat-life";
/// Authenticate the reviewed Life changes since the original routing/query
/// packet before admitting them to this finite graph. No historical runtime body.
pub(super) fn check_current_packet(endpoint: &StagedOwnedRelease) {
    life_routing_family::check_authored();
    amulet_life_family::check_authored();
    let deps: amulet_life_family::Dependencies = amulet_life_family::read("dependencies.json");
    let copy = amulet_life_family::consumer();
    let replacements = life_routing_family::replacements();
    assert_eq!(deps.life, replacements[0].after);
    let mut expected = deps.life;
    expected.programs.members.push(copy.program);
    expected.programs.closure = copy.life_closure;
    let SchemaClosure::Partial { gaps } = &mut expected.programs.closure else {
        panic!()
    };
    assert_eq!(gaps.len(), 5);
    // This retirement was published with the unchanged canonical item admission
    // evidence. It removes no numeric/routing/contributor obligation.
    gaps.retain(|g| g.code != key("canonical-input-admission-unproved"));
    assert_eq!(gaps.len(), 4);
    let rules = &endpoint.input().recipe.rules;
    assert_eq!(
        rules
            .owners
            .iter()
            .find(|o| o.owner == expected.owner)
            .unwrap(),
        &expected
    );
    for row in replacements.iter().skip(1) {
        let mut expected = row.after.clone();
        assert!(deps.templates.contains(&expected));
        let extra = copy
            .eligibility
            .iter()
            .find(|o| o.owner == expected.owner)
            .unwrap();
        expected
            .programs
            .members
            .extend(extra.programs.members.clone());
        assert_eq!(
            rules
                .owners
                .iter()
                .find(|o| o.owner == expected.owner)
                .unwrap(),
            &expected
        );
    }
    life_query_family::assert_component_with_reviewed_donor(
        endpoint,
        &replacements[0].before,
        &expected,
        &deps.query,
        &copy.query,
    );
}
#[derive(Clone)]
struct ItemSource {
    original: ItemRecord,
    item: ItemRecord,
    source: String,
    expected: f64,
}
#[derive(Clone)]
pub(super) struct Census {
    items: Vec<ItemSource>,
    equipment: Vec<EquipmentUse>,
    rewards: Vec<RewardSelection>,
    actual_owners: Vec<DefinitionRules>,
    source_records: Vec<Value>,
    catalog_obligations: Vec<Value>,
}
fn inner(w: &mut World) -> &mut shared::World {
    &mut w.sniper.base.source.base.inner
}
fn one<'a>(rows: &'a [Value], field: &str, id: &Value) -> &'a Value {
    let found: Vec<_> = rows.iter().filter(|r| r[field] == *id).collect();
    assert_eq!(found.len(), 1, "unique {field}={id}");
    found[0]
}
fn members(v: &Value) -> &[Value] {
    assert_eq!(v["completion"]["kind"], "complete");
    v["members"].as_array().unwrap()
}
fn known_catalog_records<'a>(v: &'a Value, code: &str) -> &'a [Value] {
    assert_eq!(v["completion"]["kind"], "pending");
    assert_eq!(v["completion"]["code"], code);
    v["members"].as_array().unwrap()
}
fn original_source() -> (Vec<u8>, Vec<Value>) {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap();
    let packet = attribute_flag_family::checked_source();
    let original = one(
        packet["projections"].as_array().unwrap(),
        "name",
        &Value::from("original-05"),
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        original["xml_sha256"]
    );
    let main = original["state"]["modes"]["MAIN"]["read_set"][0]["life"]
        .as_array()
        .unwrap();
    let calcs = original["state"]["modes"]["CALCS"]["read_set"][0]["life"]
        .as_array()
        .unwrap();
    assert_eq!(main, calcs, "same exact source Life records in both modes");
    let rows: Vec<_> = main
        .iter()
        .filter(|r| {
            r["source"].as_str().unwrap().starts_with("Item:")
                || r["source"].as_str().unwrap().starts_with("Quest:")
        })
        .cloned()
        .collect();
    assert_eq!(rows.len(), 7);
    for r in &rows {
        assert_eq!(r["name"], "Life");
        assert_eq!(r["flags"], 0);
        assert_eq!(r["keyword_flags"], 0);
        assert!(r["tags"].as_object().is_some_and(|o| o.is_empty()));
    }
    (bytes, rows)
}
fn import(package: &Path) -> Census {
    let (bytes, source_records) = original_source();
    let directory = tempfile::tempdir().unwrap();
    let xml = directory.path().join("original.xml");
    std::fs::write(&xml, &bytes).unwrap();
    let output = directory.path().join("normalized");
    let report = release::normalize(package, &xml, 5, &output);
    assert_eq!(
        report["normalization_status"], "pending",
        "other build domains are not repaired here"
    );
    let selection = evidence::selected::selection(&bytes, &output);
    let draft: Value = shared::read(output.join("draft.json"));
    let sidecar: Value = shared::read(output.join("sidecar.json"));
    assert_eq!(
        sidecar["source_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    let draft = &draft["draft"];
    // An exact saved selection can resolve known records while the global
    // catalog still has obligations. Keep those obligations: this component
    // cannot prove absence of additional item/socket/reward sources.
    let item_records =
        known_catalog_records(&draft["items"], "socketed-item-membership-not-converted");
    let equipment_records = known_catalog_records(
        &draft["equipment"],
        "socketed-equipment-membership-not-converted",
    );
    let reward_records = known_catalog_records(&draft["rewards"], "rewards-not-converted");
    let catalog_obligations = ["items", "equipment", "rewards"]
        .into_iter()
        .map(|name| draft[name]["completion"].clone())
        .collect();
    let equipment_preset = one(
        members(&draft["equipment_presets"]),
        "id",
        &selection["build"]["equipment"],
    );
    let selected: Vec<EquipmentUse> = members(&equipment_preset["equipment"])
        .iter()
        .map(|id| {
            decode::<EquipmentDraft>(one(equipment_records, "id", id))
                .to_resolved()
                .expect("selected actual equipment use")
        })
        .collect();
    assert_eq!(selected.len(), 9);
    let source = ImportedBuildInstance::from_decoded(
        decode_build(&bytes).unwrap(),
        BuildLineage::from_bytes([93; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let source = SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let labels: BTreeSet<_> = source_records
        .iter()
        .filter_map(|r| r["source"].as_str())
        .filter(|s| s.starts_with("Item:"))
        .collect();
    assert_eq!(labels.len(), 4);
    let mut items = Vec::new();
    let mut equipment = Vec::new();
    for (index, label) in labels.into_iter().enumerate() {
        let source_id = label.split(':').nth(1).unwrap();
        let occurrences: Vec<_> = source
            .rows()
            .iter()
            .filter(|r| {
                r.occurrence().name() == "Item"
                    && r.attribute("id")
                        .is_some_and(|a| a.decoded().unwrap() == source_id)
            })
            .collect();
        assert_eq!(occurrences.len(), 1);
        let origin = serde_json::to_value(occurrences[0].occurrence().id()).unwrap();
        let links = one(sidecar["origins"].as_array().unwrap(), "source", &origin);
        let item_links: Vec<_> = links["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["kind"] == "item")
            .collect();
        assert_eq!(item_links.len(), 1);
        let original = decode::<ItemDraft>(one(item_records, "id", &item_links[0]["value"]))
            .to_resolved()
            .expect("all actual item rolls resolve");
        let uses: Vec<_> = selected.iter().filter(|u| u.item == original.id).collect();
        let expected: Vec<_> = source_records
            .iter()
            .filter(|r| r["source"] == label)
            .collect();
        assert_eq!(
            uses.len(),
            expected.len(),
            "one actual delivery per selected use"
        );
        assert!(!uses.is_empty());
        assert!(
            expected
                .iter()
                .all(|r| r["type"] == "BASE" && r["value"] == expected[0]["value"])
        );
        let mut item = original.clone();
        item.modifiers.retain(|m| m.definition == d(0x3100));
        assert_eq!(item.modifiers.len(), 1);
        assert_eq!(item.modifiers[0].rolls.len(), 24);
        assert_eq!(
            item.modifiers[0]
                .rolls
                .iter()
                .find(|p| p.slot.slot == d(0x3101))
                .unwrap()
                .value,
            quantity(expected[0]["value"].as_f64().unwrap(), &d(0x295a)),
            "source individual amount agrees with the exact imported raw roll in this unscaled domain"
        );
        assert!(original.modifier_order.contains(&item.modifiers[0].id));
        item.id = id(7800 + 10 * index as u64);
        item.modifiers[0].id = id(7801 + 10 * index as u64);
        item.modifier_order = vec![item.modifiers[0].id];
        for actual in uses {
            assert_eq!(actual.scope, LoadoutScope::Shared);
            let mut e = actual.clone();
            e.item = item.id;
            e.id = id(7850 + equipment.len() as u64);
            equipment.push(e);
        }
        items.push(ItemSource {
            original,
            item,
            source: label.into(),
            expected: expected[0]["value"].as_f64().unwrap(),
        });
    }
    assert_eq!(equipment.len(), 5);
    // Build projection unions character and choice-preset reward selections.
    // Saved rewards belonging to another preset are never selected implicitly.
    let character = one(
        members(&draft["character_presets"]),
        "id",
        &selection["build"]["character"],
    );
    let choices = one(
        members(&draft["choice_presets"]),
        "id",
        &selection["build"]["choices"],
    );
    assert_eq!(choices["choices"]["completion"]["kind"], "pending");
    let mut reward_ids = members(&character["rewards"]).to_vec();
    for id in members(&choices["rewards"]) {
        if !reward_ids.contains(id) {
            reward_ids.push(id.clone());
        }
    }
    let mut rewards: Vec<_> = reward_ids
        .iter()
        .map(|id| decode::<RewardDraft>(one(reward_records, "id", id)))
        .filter(|r| {
            [d::<RewardDefinition>(0x29), d(0x53)].contains(&r.definition.to_resolved().unwrap())
        })
        .map(|r| r.to_resolved().expect("actual selected Life reward inputs"))
        .collect();
    assert_eq!(rewards.len(), 2);
    for (i, r) in rewards.iter_mut().enumerate() {
        r.id = id(7860 + i as u64);
    }
    Census {
        items,
        equipment,
        rewards,
        actual_owners: vec![],
        source_records,
        catalog_obligations,
    }
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
pub(super) fn install(
    w: &mut sniper::World,
    endpoint: &StagedOwnedRelease,
    package: &Path,
) -> Census {
    let mut census = import(package);
    let recipe = &endpoint.input().recipe;
    let mut definitions = Vec::new();
    let mut owners = Vec::new();
    let mut early = Vec::new();
    let modifier = recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == subject(d::<ModifierDefinition>(0x3100)))
        .unwrap();
    assert!(!modifier.programs.is_complete());
    assert_eq!(modifier.programs.members.len(), 6);
    census.actual_owners.push(modifier.clone());
    let mut finite = modifier.clone();
    finite.programs.closure = SchemaClosure::Complete;
    for p in &finite.programs.members {
        if p.id != key(DELIVERY) && p.id != key(amulet_life_family::PROGRAM) {
            early.push((finite.owner.clone(), p.id.clone()));
        }
    }
    owners.push(finite);
    for source in &mut census.items {
        let actual = recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == subject(source.item.template.clone()))
            .unwrap();
        assert!(!actual.programs.is_complete());
        census.actual_owners.push(actual.clone());
        let programs: Vec<_> = actual
            .programs
            .members
            .iter()
            .filter(|p| {
                [
                    "catalyst-inputs",
                    life_routing_family::APPLICABILITY,
                    "amulet-copy-eligibility",
                ]
                .contains(&p.id.as_str())
            })
            .cloned()
            .collect();
        assert_eq!(programs.len(), 3);
        let catalyst = programs
            .iter()
            .find(|p| p.id == key("catalyst-inputs"))
            .unwrap();
        let slots: BTreeSet<_> = catalyst
            .reads
            .iter()
            .map(|r| match &r.source {
                RuleReadSource::Parameter { slot } => slot.clone(),
                _ => panic!("actual catalyst input contract changed"),
            })
            .collect();
        assert_eq!(slots.len(), 2);
        source.item.parameters.retain(|p| slots.contains(&p.slot));
        assert_eq!(source.item.parameters.len(), 2);
        early.extend(
            programs
                .iter()
                .map(|p| (actual.owner.clone(), p.id.clone())),
        );
        owners.push(DefinitionRules {
            owner: actual.owner.clone(),
            programs: DeclaredSet::complete(programs),
        });
        let mut descriptor = recipe
            .schema
            .definitions
            .iter()
            .find(|r| r.address() == source.item.template.address())
            .unwrap()
            .clone();
        let DefinitionDescriptor::ItemTemplate(DefinitionEntry {
            schema: SchemaState::Known(s),
            ..
        }) = &mut descriptor
        else {
            panic!()
        };
        assert!(s.modifiers.members.contains(&d(0x3100)));
        // All selected Life-item placement inventories now come unchanged from
        // production data, including both uses of the same physical ring.
        assert!(s.equipment_slots.is_complete());
        for equipped in census.equipment.iter().filter(|e| e.item == source.item.id) {
            let EquipmentDestination::CharacterSlot(slot) = &equipped.destination else {
                panic!("this finite source slice contains only selected character slots")
            };
            assert!(s.equipment_slots.members.contains(slot));
        }
        s.modifiers = DeclaredSet::complete(vec![d(0x3100)]);
        s.declarations.parameters = DeclaredSet::complete(slots.into_iter().collect());
        assert!(s.socket_destinations.members.is_empty());
        assert!(s.declarations.grants.members.is_empty());
        definitions.push(sniper::offering::prolonged::finite(&descriptor));
    }
    for reward in &census.rewards {
        let actual = recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == subject(reward.definition.clone()))
            .unwrap();
        assert!(actual.programs.is_complete());
        owners.push(actual.clone());
    }
    let f = &mut w.base.source.base.inner;
    for d in definitions {
        assert!(
            !f.schema
                .definitions
                .iter()
                .any(|old| old.address() == d.address())
        );
        f.schema.definitions.push(d);
    }
    let mut keys = BTreeSet::new();
    for o in &owners {
        referenced(&serde_json::to_value(o).unwrap(), &mut keys);
    }
    for item in &census.items {
        referenced(&serde_json::to_value(&item.item).unwrap(), &mut keys);
        let template = f
            .schema
            .definitions
            .iter()
            .find(|r| r.address() == item.item.template.address())
            .unwrap();
        referenced(&serde_json::to_value(template).unwrap(), &mut keys);
    }
    for e in &census.equipment {
        referenced(&serde_json::to_value(e).unwrap(), &mut keys);
    }
    let mut done = BTreeSet::new();
    while let Some(k) = keys.iter().find(|k| !done.contains(*k)).cloned() {
        done.insert(k.clone());
        let def = recipe
            .schema
            .definitions
            .iter()
            .find(|r| r.address().key().as_str() == k);
        if let Some(actual) = def {
            if !f
                .schema
                .definitions
                .iter()
                .any(|old| old.address() == actual.address())
            {
                let actual = sniper::offering::prolonged::finite(actual);
                referenced(&serde_json::to_value(&actual).unwrap(), &mut keys);
                f.schema.definitions.push(actual);
            }
            f.owner_mut(SchemaSubject::Definition(actual.address()));
        }
        for s in recipe.schema.slots.iter().filter(
            |s| matches!(s.address(), SlotAddress::Parameter(ref p) if p.slot.key().as_str()==k),
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
        let destination = f.owner_mut(o.owner.clone());
        assert!(destination.programs.members.is_empty());
        *destination = o;
    }
    for source in &census.items {
        f.build.items.push(source.item.clone());
    }
    f.build.equipment.extend(census.equipment.clone());
    assert!(f.build.character.rewards.is_empty());
    f.build.character.rewards.extend(census.rewards.clone());
    w.base.item_programs.extend(early);
    census
}
fn delivered(report: &SupportEffectsReport) -> Vec<&BoundEffectResult> {
    sniper::offering::effects(report)
        .effects
        .iter()
        .filter(|r| {
            r.key.invocation.owner == subject(d::<ModifierDefinition>(0x3100))
                && r.key.invocation.program == key(DELIVERY)
                || [
                    subject(d::<RewardDefinition>(0x29)),
                    subject(d::<RewardDefinition>(0x53)),
                ]
                .contains(&r.key.invocation.owner)
        })
        .collect()
}
pub(super) fn check(w: &World, report: &SupportEffectsReport) {
    let f = &w.sniper.base.source.base.inner;
    let rows = delivered(report);
    let uses: Vec<_> = f
        .build
        .equipment
        .iter()
        .filter(|e| w.life_inputs.items.iter().any(|i| i.item.id == e.item))
        .collect();
    let rewards: Vec<_> = f
        .build
        .character
        .rewards
        .iter()
        .filter(|r| w.life_inputs.rewards.iter().any(|old| old.id == r.id))
        .collect();
    assert_eq!(rows.len(), uses.len() + rewards.len());
    for equipment in uses {
        let item = f
            .build
            .items
            .iter()
            .find(|i| i.id == equipment.item)
            .unwrap();
        let modifier = &item.modifiers[0];
        let origin = RuleOrigin::Provider {
            provider: ProviderKey {
                root: ProviderRoot::ItemModifier {
                    equipment_use: equipment.id,
                    modifier: modifier.id,
                },
                grant_path: vec![],
            },
        };
        let found: Vec<_> = rows
            .iter()
            .filter(|r| r.key.invocation.origin == origin)
            .collect();
        assert_eq!(found.len(), 1, "exact equipped modifier occurrence");
        assert_eq!(
            found[0].target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: d(0x311a),
                    kind: ContributionKind::Add
                }
            }
        );
        let raw = modifier
            .rolls
            .iter()
            .find(|p| p.slot.slot == d(0x3101))
            .unwrap();
        let ParameterValue::Quantity(raw) = &raw.value else {
            panic!()
        };
        // These imported examples have an inapplicable catalyst and no magnitude
        // supplier. This expectation is only for the explicitly tested domain.
        assert_eq!(
            found[0].value,
            EffectValue::Known {
                value: quantity(raw.value(), &d(0x3119))
            }
        );
    }
    for reward in rewards {
        let origin = RuleOrigin::Provider {
            provider: ProviderKey {
                root: ProviderRoot::Reward(reward.id),
                grant_path: vec![],
            },
        };
        let found: Vec<_> = rows
            .iter()
            .filter(|r| r.key.invocation.origin == origin)
            .collect();
        assert_eq!(found.len(), 1);
        let (kind, value, unit) = if reward.definition == d(0x29) {
            (ContributionKind::Add, 20., 0x3119)
        } else {
            (ContributionKind::Increase, 5., 2)
        };
        assert_eq!(
            found[0].target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: d(0x311a),
                    kind
                }
            }
        );
        assert_eq!(
            found[0].value,
            EffectValue::Known {
                value: quantity(value, &d(unit))
            }
        );
    }
    assert!(
        !sniper::offering::effects(report)
            .values
            .iter()
            .any(|v| matches!(&v.key,PlanValueKey::Stat{stat,..} if *stat==d(0x311a))),
        "individual contributions do not imply final Life availability"
    );
}

#[test]
#[ignore = "requires current joined Sniper release"]
fn actual_selected_life_items_and_rewards_deliver_exact_source_occurrences() {
    let w = World::load();
    let r = w.evaluate();
    check(&w, &r);
    assert_eq!(w.life_inputs.catalog_obligations.len(), 3);
    assert!(
        w.life_inputs
            .catalog_obligations
            .iter()
            .all(|o| o["kind"] == "pending")
    );
    let mut measured = Vec::new();
    for source in &w.life_inputs.items {
        let uses: Vec<_> = w
            .life_inputs
            .equipment
            .iter()
            .filter(|e| e.item == source.item.id)
            .collect();
        for _ in &uses {
            measured.push((source.source.clone(), "BASE", source.expected));
        }
        assert_eq!(source.original.template, source.item.template);
        assert!(
            source
                .original
                .modifiers
                .iter()
                .any(|m| m.definition == d(0x3100) && m.rolls == source.item.modifiers[0].rolls)
        );
    }
    measured.extend([
        ("Quest:Act 1: Ogham Manor".into(), "BASE", 20.),
        ("Quest:Interlude 2: Khari Crossing".into(), "INC", 5.),
    ]);
    let mut expected: Vec<_> = w
        .life_inputs
        .source_records
        .iter()
        .map(|r| {
            (
                r["source"].as_str().unwrap().to_owned(),
                r["type"].as_str().unwrap(),
                r["value"].as_f64().unwrap(),
            )
        })
        .collect();
    measured.sort_by(|a, b| a.0.cmp(&b.0));
    expected.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        measured, expected,
        "source identity/value multiset, not an aggregation-order proof"
    );
    let ring = w
        .life_inputs
        .items
        .iter()
        .find(|r| r.item.template == d(0x9dc))
        .unwrap();
    let ring_uses: Vec<_> = w
        .life_inputs
        .equipment
        .iter()
        .filter(|u| u.item == ring.item.id)
        .collect();
    assert_eq!(ring_uses.len(), 2);
    assert_ne!(ring_uses[0].id, ring_uses[1].id);
    assert_ne!(ring_uses[0].destination, ring_uses[1].destination);
}
pub(super) fn change_ring(w: &mut World, value: f64) -> ItemRecordId {
    let item = w
        .life_inputs
        .items
        .iter()
        .find(|r| r.item.template == d(0x9dc))
        .unwrap()
        .item
        .id;
    let item = inner(w)
        .build
        .items
        .iter_mut()
        .find(|i| i.id == item)
        .unwrap();
    item.modifiers[0]
        .rolls
        .iter_mut()
        .find(|p| p.slot.slot == d(0x3101))
        .unwrap()
        .value = quantity(value, &d(0x295a));
    item.id
}
#[test]
#[ignore = "requires current joined Sniper release"]
fn physical_ring_roll_changes_both_uses_and_removing_one_use_preserves_the_other() {
    let mut w = World::load();
    let item = change_ring(&mut w, 14.);
    check(&w, &w.evaluate());
    let removed = inner(&mut w)
        .build
        .equipment
        .iter()
        .find(|e| e.item == item)
        .unwrap()
        .id;
    inner(&mut w).build.equipment.retain(|e| e.id != removed);
    let r = w.evaluate();
    check(&w, &r);
    assert!(!delivered(&r).iter().any(|r|matches!(&r.key.invocation.origin,RuleOrigin::Provider{provider:ProviderKey{root:ProviderRoot::ItemModifier{equipment_use,..},..}} if *equipment_use==removed)));
    inner(&mut w)
        .build
        .character
        .rewards
        .retain(|r| r.definition != d(0x29));
    check(&w, &w.evaluate());
}
#[test]
#[ignore = "requires current joined Sniper release"]
fn life_input_storage_reorder_and_reused_parallel_scratch_preserve_occurrences() {
    let original = World::load();
    let a = original.plan();
    let mut changed = original.clone();
    change_ring(&mut changed, 19.);
    let b = changed.plan();
    let mut reordered = original.clone();
    inner(&mut reordered).build.items.reverse();
    inner(&mut reordered).build.equipment.reverse();
    inner(&mut reordered).build.character.rewards.reverse();
    let reordered = reordered.evaluate();
    let expected = a.evaluate(&mut a.new_scratch()).unwrap();
    assert_eq!(delivered(&expected), delivered(&reordered));
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    pool.install(|| {
        (0..8).into_par_iter().for_each(|_| {
            let mut scratch = a.new_scratch();
            let first = a.evaluate(&mut scratch).unwrap();
            check(&original, &first);
            let middle = b.evaluate(&mut scratch).unwrap();
            check(&changed, &middle);
            let last = a.evaluate(&mut scratch).unwrap();
            check(&original, &last);
            assert_eq!(first, last);
        })
    });
}
#[test]
#[ignore = "requires current joined Sniper release"]
fn life_item_partial_owners_remain_unavailable_in_the_same_graph() {
    for index in [0, 1] {
        let mut w = World::load();
        let actual = w.life_inputs.actual_owners[index].clone();
        let owner = actual.owner.clone();
        // Restore the actual closure without adding excluded unrelated programs.
        inner(&mut w).owner_mut(owner.clone()).programs.closure = actual.programs.closure;
        // These item programs participate in early preparation. Its admission
        // rejects the actual Partial inventory before a plan can be constructed.
        let error = w
            .checked_plan()
            .err()
            .expect("actual Partial owner rejects");
        assert_eq!(
            error,
            "invalid evaluation stages: early readiness needs an early phase and complete owner programs"
        );
    }
}

// Reuse the exact production passive in an isolated numerical component. This
// fixture does not certify its full adjacency or grant late item-granted nodes
// the ordinary allocated node's early authority.
fn routing_control(w: &mut World) -> Allocation {
    let endpoint = release::load(&path());
    let recipe = &endpoint.input().recipe;
    let node: PassiveNodeDefId = d(0x1338);
    let actual = recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == subject(node.clone()))
        .unwrap();
    assert!(!actual.programs.is_complete());
    assert_eq!(actual.programs.members.len(), 1);
    assert_eq!(
        actual.programs.members[0].id,
        key("talisman-ordinary-amulet-retention")
    );
    let mut descriptor = sniper::offering::prolonged::finite(
        recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == node.address())
            .unwrap(),
    );
    let DefinitionDescriptor::PassiveNode(DefinitionEntry {
        schema: SchemaState::Known(schema),
        ..
    }) = &mut descriptor
    else {
        panic!()
    };
    let pool = schema.pools.members[0].clone();
    schema.adjacent = DeclaredSet::complete(vec![]);
    let f = inner(w);
    assert!(
        !f.schema
            .definitions
            .iter()
            .any(|d| d.address() == node.address())
    );
    f.schema.definitions.push(descriptor);
    if !f
        .schema
        .definitions
        .iter()
        .any(|d| d.address() == pool.address())
    {
        f.schema.definitions.push(
            recipe
                .schema
                .definitions
                .iter()
                .find(|d| d.address() == pool.address())
                .unwrap()
                .clone(),
        );
    }
    f.owner_mut(subject(pool.clone()));
    *f.owner_mut(actual.owner.clone()) = sniper::offering::prolonged::finite(actual);
    w.sniper
        .early
        .push((actual.owner.clone(), actual.programs.members[0].id.clone()));
    Allocation {
        id: id(7930),
        node,
        pool,
        scope: LoadoutScope::Shared,
        access: AllocationAccess::Ordinary,
        choices: vec![],
    }
}
fn routing_plan(w: &World) -> shared::Plan {
    w.checked_plan_configured(|stages| {
        for row in &mut stages.programs.members {
            if row.program == key("talisman-ordinary-amulet-retention") {
                row.stage = key("routing-contributors");
            }
        }
    })
    .unwrap()
}
fn add_amulet_life(w: &mut World, amount: f64) {
    let mut modifier = w.life_inputs.items[0].item.modifiers[0].clone();
    modifier.id = id(7931);
    modifier
        .rolls
        .iter_mut()
        .find(|p| p.slot.slot == d(0x3101))
        .unwrap()
        .value = quantity(amount, &d(0x295a));
    let f = inner(w);
    let amulet = f
        .build
        .items
        .iter_mut()
        .find(|i| i.template == d(0x2343))
        .unwrap();
    amulet.modifier_order.push(modifier.id);
    amulet.modifiers.push(modifier);
    let DefinitionDescriptor::ItemTemplate(DefinitionEntry {
        schema: SchemaState::Known(schema),
        ..
    }) = f
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == amulet.template.address())
        .unwrap()
    else {
        panic!()
    };
    schema.modifiers.members.push(d(0x3100));
}
fn life_rows(report: &SupportEffectsReport) -> Vec<&BoundEffectResult> {
    sniper::offering::effects(report)
        .effects
        .iter()
        .filter(|e| {
            e.key.invocation.owner == subject(d::<ModifierDefinition>(0x3100))
                && e.key.invocation.program == key(DELIVERY)
        })
        .collect()
}

#[test]
#[ignore = "requires current joined Sniper release"]
fn talisman_suppresses_amulet_life_but_keeps_every_other_physical_use() {
    let mut a = World::load();
    let allocation = routing_control(&mut a);
    add_amulet_life(&mut a, 13.);
    let pa = routing_plan(&a);
    let ra = pa.evaluate(&mut pa.new_scratch()).unwrap();
    let baseline = life_rows(&ra);
    assert_eq!(baseline.len(), 6);
    assert!(
        baseline
            .iter()
            .all(|e| matches!(e.value, EffectValue::Known { .. }))
    );
    let mut b = a.clone();
    inner(&mut b).build.allocations.push(allocation.clone());
    let pb = routing_plan(&b);
    let rb = pb.evaluate(&mut pb.new_scratch()).unwrap();
    let diverted = life_rows(&rb);
    assert_eq!(
        diverted.len(),
        6,
        "inactive sources retain exact identities"
    );
    let inactive: Vec<_> = diverted
        .iter()
        .filter(|e| e.value == EffectValue::Inactive)
        .collect();
    assert_eq!(inactive.len(), 1);
    let prior = baseline.iter().find(|e| e.key == inactive[0].key).unwrap();
    assert_eq!(
        prior.value,
        EffectValue::Known {
            value: quantity(13., &d(0x3119))
        }
    );
    for unchanged in diverted.iter().filter(|e| e.value != EffectValue::Inactive) {
        assert_eq!(
            baseline
                .iter()
                .find(|e| e.key == unchanged.key)
                .unwrap()
                .value,
            unchanged.value
        );
    }
    inner(&mut b)
        .build
        .allocations
        .retain(|r| r.id != allocation.id);
    let again = routing_plan(&b);
    assert_eq!(again.identity(), pa.identity());
    let mut scratch = pa.new_scratch();
    for (p, expected) in [(&pa, &ra), (&pb, &rb), (&again, &ra)] {
        assert_eq!(p.evaluate(&mut scratch).unwrap(), *expected);
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    pool.install(|| {
        (0..12).into_par_iter().for_each_init(
            || pa.new_scratch(),
            |scratch, i| {
                let (p, expected) = if i % 2 == 0 { (&pa, &ra) } else { (&pb, &rb) };
                assert_eq!(p.evaluate(scratch).unwrap(), *expected);
            },
        )
    });
}

#[test]
#[ignore = "requires current joined Sniper release"]
fn missing_life_applicability_never_defaults_to_delivery() {
    let mut w = World::load();
    let owner = subject(d::<ItemTemplateDefinition>(0x238c));
    inner(&mut w)
        .owner_mut(owner.clone())
        .programs
        .members
        .retain(|p| p.id != key(life_routing_family::APPLICABILITY));
    w.sniper
        .base
        .item_programs
        .retain(|(o, p)| *o != owner || *p != key(life_routing_family::APPLICABILITY));
    let p = w.plan();
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    let rows = life_rows(&r);
    assert_eq!(rows.len(), 5);
    assert_eq!(
        rows.iter()
            .filter(|e| matches!(
                e.value,
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    ..
                }
            ))
            .count(),
        1
    );
    assert_eq!(
        rows.iter()
            .filter(|e| matches!(e.value, EffectValue::Known { .. }))
            .count(),
        4
    );
}

#[test]
#[ignore = "requires current joined Sniper release"]
fn life_delivery_cannot_read_applicability_before_its_frozen_stage() {
    let w = World::load();
    let error = w
        .checked_plan_configured(|stages| {
            stages
                .programs
                .members
                .iter_mut()
                .find(|p| p.program == key(DELIVERY))
                .unwrap()
                .stage = key("prepare");
        })
        .err()
        .expect("early delivery must be rejected");
    assert!(
        error.contains("frozen channel read occurs before or outside frozen stage"),
        "{error}"
    );
}
