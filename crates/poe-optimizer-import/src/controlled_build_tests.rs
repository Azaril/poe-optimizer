use super::*;
use poe_optimizer_data::game_data::{self, GameDataLoader, LoadLimits, TrustPolicy};
use std::sync::OnceLock;
const MACE: &str = include_str!("../../../tests/fixtures/calibration/mace-wooden.xml");
const SPARK: &str = include_str!("../../../tests/fixtures/calibration/spark-mapping.xml");
fn compiled() -> Arc<CompiledGameData> {
    static DATA: OnceLock<Arc<CompiledGameData>> = OnceLock::new();
    DATA.get_or_init(|| {
        Arc::new(
            CompiledGameData::compile(Arc::new(game_data::bundled_snapshot().unwrap())).unwrap(),
        )
    })
    .clone()
}
fn catalog(source: &str, items: Vec<EquipmentAlternative>) -> Arc<ControlledBuildCatalog> {
    Arc::new(ControlledBuildCatalog::new(compiled(), source.into(), items).unwrap())
}
fn constraints() -> CandidateConstraints {
    CandidateConstraints {
        budgets: CandidateBudgets {
            ordinary_passive_points: 20,
            ascendancy_passive_points: 8,
            active_skill_count: 1,
            supports_per_skill: 2,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn domain(catalog: Arc<ControlledBuildCatalog>) -> ControlledBuildDomain {
    ControlledBuildDomain::new(catalog, constraints(), AttributeOptionLocks::default()).unwrap()
}
fn amulet(id: &str, numeric: u32, extra: &str) -> EquipmentAlternative {
    EquipmentAlternative {
        instance_id: id.into(),
        pob_item_id: numeric,
        item_text: format!(
            "Rarity: RARE\nTrial Pendant\nAmber Amulet\nItem Level: 82\nQuality: 0\nImplicits: 1\n+12 to Strength\n{extra}"
        ),
    }
}
// The graph finds a supported multi-edge path; fixture IDs are not an admission policy.
fn attribute_path(catalog: &ControlledBuildCatalog) -> (BTreeSet<u32>, u32) {
    let source = catalog.source_selection();
    let root = catalog.catalog.classes[&source.candidate.class_id].start_node_id;
    let mut pending = std::collections::VecDeque::from([(root, BTreeSet::new())]);
    let mut visited = BTreeSet::from([root]);
    while let Some((at, path)) = pending.pop_front() {
        for next in &catalog.catalog.passive_nodes[&at].links {
            if visited.contains(next)
                || !matches!(
                    catalog.catalog.passive_nodes[next].kind,
                    PassiveKind::Ordinary
                )
            {
                continue;
            }
            let mut trial = source.clone();
            trial.candidate.passives = path.clone();
            trial.candidate.passives.insert(*next);
            for id in &trial.candidate.passives {
                if catalog.compiled.snapshot().tree().allocation_nodes[id]
                    .attribute_options
                    .contains_key(&AttributeOption::Strength)
                {
                    trial
                        .attribute_options
                        .insert(*id, AttributeOption::Strength);
                }
            }
            if trial
                .allocation(catalog.catalog())
                .unwrap()
                .resolve(catalog.compiled.snapshot())
                .is_err()
            {
                continue;
            }
            visited.insert(*next);
            if trial.candidate.passives.len() > 1 && trial.attribute_options.contains_key(next) {
                return (trial.candidate.passives, *next);
            }
            pending.push_back((*next, trial.candidate.passives));
        }
    }
    panic!("selected source class has no admitted multi-edge attribute path")
}
fn with_path(catalog: &ControlledBuildCatalog) -> (BuildSelection, u32) {
    let (path, attribute) = attribute_path(catalog);
    let mut selection = catalog.source_selection();
    selection.candidate.passives = path;
    for id in &selection.candidate.passives {
        if catalog.compiled.snapshot().tree().allocation_nodes[id]
            .attribute_options
            .contains_key(&AttributeOption::Strength)
        {
            selection
                .attribute_options
                .insert(*id, AttributeOption::Strength);
        }
    }
    (selection, attribute)
}
#[test]
fn both_profiles_round_trip_connected_allocations_and_explicit_attribute_identity() {
    for source in [MACE, SPARK] {
        let source = source
            .replace("\r\n", "\n")
            .replace('\n', "\r\n")
            .replace("<Notes>", "<!-- preserve \u{2603} exactly -->\r\n<Notes>");
        let catalog = catalog(&source, vec![]);
        let domain = domain(catalog.clone());
        let mut scratch = ActorScratch::default();
        let (selection, attribute) = with_path(&catalog);
        let strength = domain.admit(selection.clone(), &mut scratch).unwrap();
        let mut dexterity = selection.clone();
        dexterity
            .attribute_options
            .insert(attribute, AttributeOption::Dexterity);
        let dexterity = domain.admit(dexterity, &mut scratch).unwrap();
        assert_ne!(strength, dexterity);
        assert_ne!(
            strength.selection().fingerprint(),
            dexterity.selection().fingerprint()
        );
        assert!(
            strength.actor().values().attributes.strength
                > dexterity.actor().values().attributes.strength
        );
        assert!(
            strength.actor().values().attributes.dexterity
                < dexterity.actor().values().attributes.dexterity
        );
        for admitted in [&strength, &dexterity] {
            let materialized = domain.materialize(admitted).unwrap();
            assert!(
                materialized
                    .content
                    .contains("<!-- preserve \u{2603} exactly -->\r\n<Notes>")
            );
            assert!(materialized.content.contains(catalog.source().active_xml()));
            let parsed =
                SourceBuildTemplate::parse(materialized.content, compiled().snapshot().package())
                    .unwrap();
            assert_eq!(parsed.allocation(), &admitted.tree().selection);
            assert_eq!(
                parsed.actor_modifiers().diagnostic(),
                catalog.source().actor_modifiers().diagnostic()
            );
            for (id, item) in catalog.source().items() {
                assert_eq!(parsed.items()[id].source_text(), item.source_text());
            }
        }
        let mut missing = selection.clone();
        missing.attribute_options.remove(&attribute);
        assert!(domain.validate_structure(&missing).is_err());
        let mut connector_removed = selection;
        connector_removed
            .candidate
            .passives
            .retain(|id| *id == attribute);
        connector_removed
            .attribute_options
            .retain(|id, _| *id == attribute);
        assert!(domain.validate_structure(&connector_removed).is_err());
    }
}
#[test]
fn source_bytes_and_component_storage_do_not_grow_across_lazy_candidates() {
    let catalog = catalog(
        MACE,
        vec![
            amulet("first", 7, "+30 to maximum Life"),
            amulet("second", 8, "+40 to maximum Mana"),
        ],
    );
    let before = serde_json::to_value(catalog.footprint()).unwrap();
    let domain = domain(catalog.clone());
    let mut scratch = ActorScratch::default();
    let (selection, attribute) = with_path(&catalog);
    for choice in AttributeOption::ALL {
        for item in [None, Some("first"), Some("second")] {
            let mut selection = selection.clone();
            selection.attribute_options.insert(attribute, choice);
            if let Some(item) = item {
                selection
                    .candidate
                    .equipment
                    .insert("Amulet".into(), item.into());
            }
            let admitted = domain.admit(selection, &mut scratch).unwrap();
            let result = domain.materialize(&admitted).unwrap();
            let parsed =
                SourceBuildTemplate::parse(result.content, compiled().snapshot().package())
                    .unwrap();
            assert_eq!(parsed.allocation(), &admitted.tree().selection);
            if let Some(item) = item {
                let selected = catalog.item(item).unwrap();
                assert_eq!(
                    parsed.items()[&selected.pob_item_id()].diagnostic(),
                    selected.diagnostic()
                );
            }
        }
    }
    assert_eq!(serde_json::to_value(catalog.footprint()).unwrap(), before);
    assert_eq!(catalog.source().source(), MACE);
    assert_eq!(catalog.footprint().item_components, 3);
    assert_eq!(catalog.footprint().materialized_candidates, 0);
    assert_eq!(catalog.footprint().cached_candidate_results, 0);
}
#[test]
fn selected_equipment_contributes_before_requirements_and_conditions() {
    let weapon=EquipmentAlternative{instance_id:"demanding".into(),pob_item_id:9,item_text:"Rarity: RARE\nDemanding Club\nSmithing Hammer\nItem Level: 1\nQuality: 0\nImplicits: 0\n+5 to Strength\n".into()};
    let mut package = compiled().snapshot().package().clone();
    let baseline = package.tree.class(6).unwrap().base_strength;
    package
        .weapons
        .iter_mut()
        .find(|w| w.name == "Smithing Hammer")
        .unwrap()
        .requirements
        .attributes
        .strength = baseline + 17;
    package.refresh_section_digests().unwrap();
    let snapshot = GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    let data = Arc::new(CompiledGameData::compile(Arc::new(snapshot)).unwrap());
    let source=MACE.replace("</ConfigSet>","<CustomModifierBlock enabled=\"true\">+20 to Intelligence\n+30 to Spirit if Strength is higher than Intelligence</CustomModifierBlock></ConfigSet>");
    let catalog = Arc::new(
        ControlledBuildCatalog::new(data, source, vec![weapon, amulet("strength", 7, "")]).unwrap(),
    );
    let domain = domain(catalog.clone());
    let mut scratch = ActorScratch::default();
    let mut selection = catalog.source_selection();
    selection
        .candidate
        .equipment
        .insert("Weapon 1".into(), "demanding".into());
    let before = domain.prepare(selection.clone(), &mut scratch).unwrap();
    let before_values = before.actor().values();
    assert!(before_values.attributes.strength < before_values.attributes.intelligence);
    let invalid = before.requirements();
    assert!(!invalid.is_legal());
    assert_eq!(invalid.available.strength, baseline + 5);
    assert!(matches!(
        domain.admit(selection.clone(), &mut scratch),
        Err(BuildCatalogError::Requirements(_))
    ));
    selection
        .candidate
        .equipment
        .insert("Amulet".into(), "strength".into());
    let admitted = domain.admit(selection, &mut scratch).unwrap();
    assert_eq!(admitted.requirements().available.strength, baseline + 17);
    assert_eq!(admitted.requirements().required.strength, baseline + 17);
    let after_values = admitted.actor().values();
    assert!(after_values.attributes.strength > after_values.attributes.intelligence);
    assert_eq!(after_values.spirit, before_values.spirit + 30.0);
    assert_eq!(catalog.item("strength").unwrap().pob_item_id(), 7);
    assert!(
        catalog
            .item("strength")
            .unwrap()
            .actor_modifiers()
            .iter()
            .all(|r| r.source.as_deref() == Some("Item:7:Trial Pendant, Amber Amulet"))
    );
}
#[test]
fn physical_instance_ids_slot_compatibility_and_foreign_grammar_fail_closed() {
    let first = amulet("a", 7, "");
    let mut duplicate = first.clone();
    duplicate.instance_id = "b".into();
    assert!(
        ControlledBuildCatalog::new(compiled(), MACE.into(), vec![first.clone(), duplicate])
            .is_err()
    );
    let mut second = first.clone();
    second.instance_id = "b".into();
    second.pob_item_id = 8;
    let catalog = catalog(MACE, vec![first, second]);
    assert_eq!(
        catalog.item("a").unwrap().source_sha256(),
        catalog.item("b").unwrap().source_sha256()
    );
    assert_ne!(
        catalog.item("a").unwrap().actor_modifiers(),
        catalog.item("b").unwrap().actor_modifiers()
    );
    let domain = domain(catalog.clone());
    let mut selection = catalog.source_selection();
    selection
        .candidate
        .equipment
        .insert("Weapon 1".into(), "a".into());
    assert!(domain.validate_structure(&selection).is_err());
    let mut foreign = amulet("foreign", 20, "");
    foreign.item_text = foreign.item_text.replace("to Strength", "custom Strength");
    assert!(ControlledBuildCatalog::new(compiled(), MACE.into(), vec![foreign]).is_err());
    let mut conflict = EquipmentAlternative {
        instance_id: "conflict".into(),
        pob_item_id: 1,
        item_text: catalog.source().items()[&1].source_text().into(),
    };
    conflict.item_text = conflict.item_text.replace("Quality: 0", "Quality: 1");
    assert!(ControlledBuildCatalog::new(compiled(), MACE.into(), vec![conflict]).is_err());
}
#[test]
fn opaque_handles_bind_both_catalog_and_domain_without_owning_source() {
    let first = catalog(MACE, vec![]);
    let equivalent = catalog(MACE, vec![]);
    assert_eq!(first.catalog().identity, equivalent.catalog().identity);
    let one = domain(first.clone());
    let other_rules = domain(first.clone());
    let foreign = domain(equivalent.clone());
    let mut scratch = ActorScratch::default();
    let handle = one.admit(first.source_selection(), &mut scratch).unwrap();
    let same = foreign
        .admit(equivalent.source_selection(), &mut scratch)
        .unwrap();
    assert_eq!(handle, same);
    let token = first.binding();
    assert!(token.accepts(&handle));
    assert!(!token.accepts(&same));
    assert!(one.validate_handle(&handle).is_ok());
    assert!(matches!(
        other_rules.validate_handle(&handle),
        Err(BuildCatalogError::Ownership)
    ));
    assert!(matches!(
        equivalent.materialize(&handle),
        Err(BuildCatalogError::Ownership)
    ));
    assert_eq!(
        serde_json::to_value(&handle).unwrap(),
        serde_json::to_value(handle.selection()).unwrap()
    );
    let weak = Arc::downgrade(&first);
    drop(one);
    drop(other_rules);
    drop(first);
    assert!(weak.upgrade().is_none());
    assert!(token.accepts(&handle));
}
#[test]
fn core_budgets_locks_and_attribute_locks_remain_independent() {
    let catalog = catalog(MACE, vec![]);
    let (selection, attribute) = with_path(&catalog);
    let mut small = constraints();
    small.budgets.ordinary_passive_points = 1;
    assert!(
        ControlledBuildDomain::new(catalog.clone(), small, AttributeOptionLocks::default())
            .unwrap()
            .validate_structure(&selection)
            .is_err()
    );
    let locks = AttributeOptionLocks {
        required: BTreeMap::from([(attribute, AttributeOption::Dexterity)]),
        ..Default::default()
    };
    let locked = ControlledBuildDomain::new(catalog.clone(), constraints(), locks.clone()).unwrap();
    assert!(locked.validate_structure(&selection).is_err());
    let mut dexterity = selection.clone();
    dexterity
        .attribute_options
        .insert(attribute, AttributeOption::Dexterity);
    assert!(locked.validate_structure(&dexterity).is_ok());
    let mut contradiction = constraints();
    contradiction.locks.unallocated_passives.insert(attribute);
    assert!(ControlledBuildDomain::new(catalog.clone(), contradiction, locks).is_err());
    let ordinary = *selection
        .candidate
        .passives
        .iter()
        .find(|id| !selection.attribute_options.contains_key(id))
        .unwrap();
    let invalid = AttributeOptionLocks {
        required: BTreeMap::from([(ordinary, AttributeOption::Strength)]),
        ..Default::default()
    };
    assert!(ControlledBuildDomain::new(catalog.clone(), constraints(), invalid).is_err());
    let mut frozen = constraints();
    frozen.locks.equipment.insert("Weapon 1".into(), None);
    assert!(
        ControlledBuildDomain::new(catalog, frozen, AttributeOptionLocks::default())
            .unwrap()
            .validate_structure(&selection)
            .is_err()
    );
}

#[test]
fn class_and_ascendancy_changes_patch_source_without_inventing_paid_roots() {
    let catalog = catalog(MACE, vec![]);
    let domain = domain(catalog.clone());
    let mut scratch = ActorScratch::default();
    let mut visited = 0;
    for (id, ascendancy) in &catalog.catalog().ascendancies {
        let mut selection = catalog.source_selection();
        selection.candidate.class_id = ascendancy.class_id.clone();
        selection.candidate.ascendancy_id = Some(id.clone());
        let handle = domain.admit(selection, &mut scratch).unwrap();
        let result = domain.materialize(&handle).unwrap();
        assert!(result.content.contains("ascendancyInternalId="));
        let parsed =
            SourceBuildTemplate::parse(result.content, compiled().snapshot().package()).unwrap();
        assert_eq!(parsed.allocation(), &handle.tree().selection);
        assert!(handle.tree().selection.ordinary_nodes.is_empty());
        assert!(handle.tree().selection.ascendancy_nodes.is_empty());
        assert_eq!(handle.tree().implicit_roots.len(), 2);
        visited += 1;
    }
    assert!(visited > 6);
}
#[test]
fn cached_outputs_are_removed_while_unchanged_override_comments_and_item_cdata_survive() {
    let catalog = catalog(MACE, vec![]);
    let domain = domain(catalog.clone());
    let (selection, _) = with_path(&catalog);
    let handle = domain
        .admit(selection, &mut ActorScratch::default())
        .unwrap();
    let text = domain.materialize(&handle).unwrap().content;
    let text=text.replace("viewMode=\"CALCS\"/>","viewMode=\"CALCS\"><PlayerStat stat=\"Str\" value=\"999999\"/><MinionStat stat=\"Life\" value=\"1\"/></Build>")
        .replace("<Overrides>","<Overrides><!-- preserve original override comment -->")
        .replace("<Item id=\"1\">","<Item id=\"1\"><![CDATA[").replace("</Item>","]]></Item>");
    let catalog = Arc::new(ControlledBuildCatalog::new(compiled(), text.clone(), vec![]).unwrap());
    let domain = super::tests::domain(catalog.clone());
    let handle = domain
        .admit(catalog.source_selection(), &mut ActorScratch::default())
        .unwrap();
    let result = domain.materialize(&handle).unwrap().content;
    assert!(!result.contains("<PlayerStat"));
    assert!(!result.contains("<MinionStat"));
    assert!(result.contains("<Overrides><!-- preserve original override comment -->"));
    assert!(result.contains("<Item id=\"1\"><![CDATA["));
    assert_eq!(catalog.source().source(), text);
}

#[test]
fn scalar_composition_failure_is_deferred_without_relabeling_legal_requirements() {
    let original = catalog(MACE, vec![]);
    let (mut selection, _) = with_path(&original);
    let allocation = selection
        .allocation(original.catalog())
        .unwrap()
        .resolve(original.compiled.snapshot())
        .unwrap();
    assert!(allocation.views.len() >= 2);
    let keys: BTreeSet<_> = allocation
        .views
        .iter()
        .map(|view| view.source.key.clone())
        .collect();
    let mut package = compiled().snapshot().package().clone();
    for view in &mut package.passive_effects {
        if keys.contains(&view.key) {
            view.effects = vec![game_data::PassiveEffect {
                stat: game_data::PassiveStat::SkillSpeedIncreased,
                value: 750_000.0,
            }];
        }
    }
    package.refresh_section_digests().unwrap();
    let snapshot = GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    let compiled = Arc::new(CompiledGameData::compile(Arc::new(snapshot)).unwrap());
    let catalog = Arc::new(ControlledBuildCatalog::new(compiled, MACE.into(), vec![]).unwrap());
    selection.candidate.catalog = catalog.catalog().identity.clone();
    let domain = domain(catalog);
    let handle = domain
        .admit(selection, &mut ActorScratch::default())
        .unwrap();
    assert!(handle.requirements().is_legal());
    assert!(handle.character().unwrap_err().contains("1000000"));
    assert!(
        domain.materialize(&handle).is_ok(),
        "document evaluation must have an opportunity to report the calculation failure"
    );
}

#[test]
fn oversized_unknown_selection_rejects_before_per_node_diagnostic_allocation() {
    let catalog = catalog(MACE, vec![]);
    let domain = domain(catalog.clone());
    let mut selection = catalog.source_selection();
    selection.candidate.passives = (100_000..150_000).collect();
    let error = domain
        .validate_structure(&selection)
        .unwrap_err()
        .to_string();
    assert!(error.contains("preparation bound"));
    assert!(error.len() < 128);
    selection.candidate.passives.clear();
    selection.attribute_options = (100_000..150_000)
        .map(|id| (id, AttributeOption::Strength))
        .collect();
    let error = domain
        .validate_structure(&selection)
        .unwrap_err()
        .to_string();
    assert!(error.contains("preparation bound"));
    assert!(error.len() < 128);
}

#[test]
fn receiving_preparation_keeps_same_actor_requirements_and_level_rejections() {
    for (base, implicit, required_level) in [
        ("Lunar Amulet", "+25 to maximum Energy Shield", 14),
        ("Pearlescent Amulet", "+8% to all Elemental Resistances", 30),
    ] {
        for level in [required_level - 1, required_level] {
            let source = MACE.replace("<Build level=\"60\"", &format!("<Build level=\"{level}\""));
            let item = EquipmentAlternative {
                instance_id: "defence".into(),
                pob_item_id: 73,
                item_text: format!(
                    "Rarity: RARE\nReceiving Pendant\n{base}\nItem Level: 82\nQuality: 0\nImplicits: 1\n{implicit}\n+20 to Strength\n+100 to Armour\n150% increased Armour"
                ),
            };
            let catalog = catalog(&source, vec![item]);
            let domain = domain(catalog.clone());
            let mut selection = catalog.source_selection();
            selection
                .candidate
                .equipment
                .insert("Amulet".into(), "defence".into());
            let prepared = domain
                .prepare(selection.clone(), &mut ActorScratch::default())
                .unwrap();
            assert_eq!(
                prepared.requirements().available.strength as f64,
                prepared.actor().values().attributes.strength
            );
            assert_eq!(prepared.actor().receiving().unwrap().armour, 250.0);
            if level == required_level {
                assert!(
                    domain
                        .admit(selection, &mut ActorScratch::default())
                        .is_ok()
                );
            } else {
                let error = domain
                    .admit(selection, &mut ActorScratch::default())
                    .unwrap_err();
                let BuildCatalogError::Requirements(assessment) = error else {
                    panic!("wrong error: {error}")
                };
                assert_eq!(assessment.violations.len(), 1);
                assert_eq!(assessment.violations[0].requirement, "level");
                assert_eq!(assessment.violations[0].required, required_level);
            }
        }
    }
}

#[test]
fn authored_receiver_scope_gate_ignores_migrated_passives_but_includes_unused_new_items() {
    let original = catalog(MACE, vec![]);
    assert!(!original.uses_receiving_defence_scope());
    let source = MACE.replace("nodes=\"\"", "nodes=\"38646\"");
    assert!(!catalog(&source, vec![]).uses_receiving_defence_scope());
    let receiving = EquipmentAlternative { instance_id:"unused".into(),pob_item_id:73,
        item_text:"Rarity: NORMAL\nLunar Amulet\nItem Level: 1\nQuality: 0\nImplicits: 1\n+20 to maximum Energy Shield".into() };
    assert!(catalog(MACE, vec![receiving]).uses_receiving_defence_scope());
    let source = MACE.replace(
        "</ConfigSet>",
        "<CustomModifierBlock enabled=\"true\">+1 to Armour</CustomModifierBlock></ConfigSet>",
    );
    assert!(catalog(&source, vec![]).uses_receiving_defence_scope());
}

#[test]
fn armour_scope_requirements_slots_and_empty_selections_remain_separate() {
    let item = EquipmentAlternative { instance_id:"helm".into(),pob_item_id:77,
        item_text:"Rarity: NORMAL\nBrimmed Helm\nItem Level: 82\nQuality: 20\nImplicits: 0\n+20 to Strength".into() };
    let source = MACE.replace("level=\"60\"", "level=\"4\"");
    let catalog = catalog(&source, vec![item]);
    assert!(catalog.uses_local_armour_scope());
    let domain = domain(catalog.clone());
    let bare = catalog.source_selection();
    assert!(
        domain
            .admit(bare.clone(), &mut ActorScratch::default())
            .is_ok()
    );
    let mut selected = bare.clone();
    selected
        .candidate
        .equipment
        .insert("Helmet".into(), "helm".into());
    let prepared = domain
        .prepare(selected.clone(), &mut ActorScratch::default())
        .unwrap();
    assert_eq!(prepared.requirements().violations.len(), 1);
    assert_eq!(prepared.requirements().violations[0].requirement, "level");
    assert_eq!(prepared.requirements().violations[0].required, 5);
    assert_eq!(prepared.requirements().violations[0].available, 4);
    assert!(
        domain
            .admit(selected.clone(), &mut ActorScratch::default())
            .is_err()
    );
    selected.candidate.equipment.remove("Helmet");
    selected
        .candidate
        .equipment
        .insert("Gloves".into(), "helm".into());
    assert!(
        domain
            .prepare(selected, &mut ActorScratch::default())
            .is_err()
    );
    let mut locked = constraints();
    locked.required_item_instance_ids.insert("helm".into());
    let locked =
        ControlledBuildDomain::new(catalog.clone(), locked, AttributeOptionLocks::default())
            .unwrap();
    assert!(locked.admit(bare, &mut ActorScratch::default()).is_err());
    assert_eq!(catalog.footprint().materialized_candidates, 0);
    assert_eq!(catalog.footprint().cached_candidate_results, 0);
    assert_eq!(catalog.footprint().local_armour_components, 1);
}

#[test]
fn body_and_unused_movement_items_are_gated_and_obey_slot_and_requirement_locks() {
    let body = EquipmentAlternative {
        instance_id: "supplied-body".into(),
        pob_item_id: 44,
        item_text:
            "Rarity: NORMAL\nRusted Cuirass\nItem Level: 82\nQuality: 0\nLevelReq: 61\nImplicits: 0"
                .into(),
    };
    let catalog = catalog(SPARK, vec![body]);
    assert!(catalog.uses_movement_scope());
    let domain = domain(catalog.clone());
    let bare = catalog.source_selection();
    assert!(
        domain
            .admit(bare.clone(), &mut ActorScratch::default())
            .is_ok()
    );
    let mut selected = bare.clone();
    selected
        .candidate
        .equipment
        .insert("Body Armour".into(), "supplied-body".into());
    let checked = domain
        .requirements(&selected, &mut ActorScratch::default())
        .unwrap();
    assert!(
        checked
            .violations
            .iter()
            .any(|v| v.requirement == "level" && v.required == 61 && v.available == 60)
    );
    selected.candidate.equipment.remove("Body Armour");
    selected
        .candidate
        .equipment
        .insert("Helmet".into(), "supplied-body".into());
    assert!(domain.validate_structure(&selected).is_err());
    let mut rules = constraints();
    rules
        .required_item_instance_ids
        .insert("supplied-body".into());
    let locked =
        ControlledBuildDomain::new(catalog, rules, AttributeOptionLocks::default()).unwrap();
    assert!(locked.admit(bare, &mut ActorScratch::default()).is_err());
}
