use poe_optimizer_data::game_data::*;

fn fixture() -> GameDataPackage {
    bundled_snapshot().unwrap().package().clone()
}
fn load(mut package: GameDataPackage) -> Result<GameDataSnapshot, GameDataError> {
    package.refresh_section_digests()?;
    GameDataLoader::from_bytes(
        &package.canonical_bytes()?,
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
}
fn rename(data: &mut CompleteUniqueRequirements, index: usize, key: &str) {
    let entry = &mut data.entries[index];
    let old = std::mem::replace(&mut entry.canonical_key, key.into());
    let prototype = entry.prototype.clone();
    let outcome = data
        .prototypes
        .iter_mut()
        .find(|row| row.prototype == prototype)
        .unwrap();
    outcome.disposition = UniquePrototypeDisposition::Inserted {
        canonical_key: key.into(),
    };
    for lookup in &mut outcome.lookup_keys {
        if *lookup == old {
            *lookup = key.into();
        }
    }
    outcome.lookup_keys.sort();
}
#[test]
fn complete_catalog_covers_original_inventory_without_assembly_capability() {
    let snapshot = bundled_snapshot().unwrap();
    let data = snapshot.unique_requirements().data().complete().unwrap();
    assert_eq!(data.entries.len(), 443);
    assert_eq!(
        data.prototypes.len(),
        snapshot
            .item_loading()
            .unique_groups()
            .values()
            .map(Vec::len)
            .sum::<usize>()
    );
    assert!(data.construction.loading_cleared && data.construction.source_loop_completed);
    assert_eq!(
        data.inputs.mod_cache_mode,
        UniqueModCacheMode::OriginalStoredCache
    );
    assert_eq!(
        snapshot.item_loading().data().capability,
        ItemLoadingCapability::DefinitionsOnly
    );
    for entry in &data.entries {
        assert_eq!(
            snapshot
                .unique_requirements()
                .lookup(&entry.canonical_key, None, None),
            UniqueRequirementLookup::Ready(Some(entry))
        );
    }
}
#[test]
fn injected_lookup_policy_retains_exact_priority_and_removes_only_one_nonempty_prefix() {
    let mut package = fixture();
    let data = package.unique_requirements.complete_mut().unwrap();
    rename(data, 0, "Fixture / Base");
    rename(data, 1, "Fixture / Prefix Base");
    data.entries
        .sort_by(|a, b| a.canonical_key.cmp(&b.canonical_key));
    data.policy = UniqueLookupPolicy {
        base_prefixes: vec!["Prefix ".into(), "Second ".into()],
        title_base_separator: " / ".into(),
    };
    let snapshot = load(package).unwrap();
    let catalog = snapshot.unique_requirements();
    fn key(value: UniqueRequirementLookup<'_>) -> Option<&str> {
        match value {
            UniqueRequirementLookup::Ready(Some(entry)) => Some(entry.canonical_key.as_str()),
            _ => None,
        }
    }
    assert_eq!(
        key(catalog.lookup(
            "Fixture / Prefix Base",
            Some("Fixture"),
            Some("Prefix Base")
        )),
        Some("Fixture / Prefix Base")
    );
    assert_eq!(
        key(catalog.lookup("miss", Some("Fixture"), Some("Prefix Base"))),
        Some("Fixture / Base")
    );
    assert_eq!(
        key(catalog.lookup("miss", Some("Fixture"), Some("Second Base"))),
        Some("Fixture / Base")
    );
    assert_eq!(
        key(catalog.lookup("miss", Some("Fixture"), Some("Prefix Prefix Base"))),
        Some("Fixture / Prefix Base")
    );
    for (title, base) in [
        (Some("Fixture"), Some("Prefix ")),
        (None, Some("Prefix Base")),
        (Some("Fixture"), None),
        (Some("fixture"), Some("Prefix Base")),
    ] {
        assert_eq!(
            catalog.lookup("miss", title, base),
            UniqueRequirementLookup::Ready(None)
        );
    }
}
#[test]
fn absence_is_not_an_empty_complete_database_and_reason_is_bounded() {
    let mut package = fixture();
    package.unique_requirements =
        UniqueRequirementData::unavailable("caller has no constructed unique data");
    let snapshot = load(package).unwrap();
    assert_eq!(
        snapshot
            .unique_requirements()
            .lookup("anything", None, None),
        UniqueRequirementLookup::Unavailable("caller has no constructed unique data")
    );
    assert!(
        UniqueRequirementCatalog::new(UniqueRequirementData::unavailable("x".repeat(4096))).is_ok()
    );
    assert!(
        UniqueRequirementCatalog::new(UniqueRequirementData::unavailable("x".repeat(4097)))
            .is_err()
    );
    assert!(UniqueRequirementCatalog::new(UniqueRequirementData::unavailable("")).is_err());
}
#[test]
fn changed_constructor_inputs_cannot_reuse_ready_catalog() {
    let mut package = fixture();
    package.item_loading.policy.default_item_quality += 1.0;
    assert!(
        load(package.clone())
            .unwrap_err()
            .0
            .contains("stale construction input identity")
    );
    package.unique_requirements = UniqueRequirementData::unavailable("construction inputs changed");
    assert!(load(package).is_ok());
    let mut package = fixture();
    package
        .unique_requirements
        .complete_mut()
        .unwrap()
        .inputs
        .tree_version = "caller-other-tree".into();
    assert!(load(package).is_err());
}
#[test]
fn omitted_middle_prototype_rejects_even_with_self_consistent_counts() {
    let mut package = fixture();
    let data = package.unique_requirements.complete_mut().unwrap();
    let removed = data.prototypes.remove(data.prototypes.len() / 2);
    data.entries
        .retain(|entry| entry.prototype != removed.prototype);
    data.construction.constructors_finished = data.prototypes.len() as u32;
    data.construction.insertions = data.entries.len() as u32;
    assert!(
        load(package)
            .unwrap_err()
            .0
            .contains("incomplete prototype accounting")
    );
}
#[test]
fn constructor_collision_and_noncolliding_cross_entry_lookup_are_rejected() {
    let mut package = fixture();
    let data = package.unique_requirements.complete_mut().unwrap();
    let key = data.entries[1].canonical_key.clone();
    rename(data, 0, &key);
    assert!(
        load(package)
            .unwrap_err()
            .0
            .contains("ambiguous constructed canonical key")
    );
    let mut package = fixture();
    let data = package.unique_requirements.complete_mut().unwrap();
    let target = data
        .entries
        .iter()
        .find(|entry| entry.prototype != data.prototypes[0].prototype)
        .unwrap()
        .canonical_key
        .clone();
    data.prototypes[0].lookup_keys.push(target);
    data.prototypes[0].lookup_keys.sort();
    assert!(load(package).unwrap_err().0.contains("cross-prototype"));
}
#[test]
fn source_nil_absence_zero_and_signed_zero_have_distinct_validated_representations() {
    let mut package = fixture();
    let data = package.unique_requirements.complete_mut().unwrap();
    data.entries[0].natural_level = None;
    data.entries[0].level = Some(-0.0);
    let key = data.entries[0].canonical_key.clone();
    let snapshot = load(package.clone()).unwrap();
    let UniqueRequirementLookup::Ready(Some(entry)) =
        snapshot.unique_requirements().lookup(&key, None, None)
    else {
        panic!()
    };
    assert_eq!(entry.level.unwrap().to_bits(), (-0.0f64).to_bits());
    assert!(entry.natural_level.is_none());
    package.unique_requirements.complete_mut().unwrap().entries[0].level = None;
    assert!(load(package).is_err());
    let mut data = fixture().unique_requirements;
    data.complete_mut().unwrap().entries[0].natural_level = Some(f64::NAN);
    assert!(data.validate().is_err());
}
#[test]
fn incomplete_readiness_and_unknown_or_stale_source_records_are_rejected() {
    let mut package = fixture();
    package
        .unique_requirements
        .complete_mut()
        .unwrap()
        .construction
        .loading_cleared = false;
    assert!(load(package).is_err());
    let mut package = fixture();
    package
        .unique_requirements
        .complete_mut()
        .unwrap()
        .source
        .construction_spans
        .remove("stored_cache");
    assert!(load(package).is_err());
    let mut package = fixture();
    package
        .unique_requirements
        .complete_mut()
        .unwrap()
        .prototypes[0]
        .prototype
        .raw_sha256 = "0".repeat(64);
    assert!(load(package).is_err());
    let mut package = fixture();
    package.unique_requirements.complete_mut().unwrap().entries[0].base_name =
        "unknown caller base".into();
    assert!(load(package).is_err());
}
#[test]
fn duplicate_json_keys_and_retained_lookup_resource_excess_reject() {
    let package = fixture();
    let bytes = String::from_utf8(package.canonical_bytes().unwrap()).unwrap();
    let altered = bytes.replacen(
        "\"title_base_separator\":",
        "\"title_base_separator\":\"discarded\",\"title_base_separator\":",
        1,
    );
    assert_ne!(bytes, altered);
    assert!(
        GameDataPackage::decode_for_authoring(altered.as_bytes(), &LoadLimits::default()).is_err()
    );
    let mut data = package.unique_requirements;
    data.complete_mut().unwrap().prototypes[0].lookup_keys = vec!["x".repeat(4097)];
    assert!(data.validate().is_err());
}
