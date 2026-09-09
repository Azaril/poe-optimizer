use poe_optimizer_data::game_data::{
    GameDataLoader, GameDataPackage, GameDataSnapshot, LoadLimits, TrustPolicy, bundled_snapshot,
};
use poe_optimizer_data::unique_requirements::UniqueRequirementData;
use poe_optimizer_import::item_loading::*;
use std::sync::OnceLock;

fn snapshot() -> &'static GameDataSnapshot {
    static SNAPSHOT: OnceLock<GameDataSnapshot> = OnceLock::new();
    SNAPSHOT.get_or_init(|| bundled_snapshot().unwrap())
}
fn custom(mut package: GameDataPackage) -> GameDataSnapshot {
    package.refresh_section_digests().unwrap();
    GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap()
}
#[derive(Default)]
struct ExplicitDependency {
    calls: usize,
}
impl ItemLoadProvider for ExplicitDependency {
    fn lookup_unique(&mut self, _: &UniqueRequest) -> DependencyResult<Option<UniqueOutcome>> {
        self.calls += 1;
        DependencyResult::Available(Some(UniqueOutcome {
            natural_level: Some(ItemNumber::new(999.0)),
            level: None,
        }))
    }
}
fn level(result: DependencyResult<Option<UniqueOutcome>>) -> Option<f64> {
    let DependencyResult::Available(outcome) = result else {
        panic!("expected an available lookup, got {result:?}")
    };
    outcome.and_then(|outcome| outcome.natural_level.and_then(ItemNumber::value))
}

#[test]
fn selected_native_lookup_does_not_fall_through_for_hits_or_definite_misses() {
    let original = snapshot();
    let mut package = original.package().clone();
    let entry = &mut package.unique_requirements.complete_mut().unwrap().entries[0];
    let name = entry.canonical_key.clone();
    // Values are caller-authored data. No item identity is selected by production code.
    entry.natural_level = Some(73.25);
    entry.level = Some(80.0);
    let selected = custom(package);
    assert_ne!(selected.identity(), original.identity());
    let mut native =
        NativeItemLoadProvider::with_native_unique_lookup(&selected, ExplicitDependency::default());
    let hit = UniqueRequest {
        name,
        title: None,
        base_name: None,
    };
    let DependencyResult::Available(Some(result)) = native.lookup_unique(&hit) else {
        panic!("exact configured key must resolve")
    };
    assert_eq!(
        result.natural_level.and_then(ItemNumber::value),
        Some(73.25)
    );
    assert_eq!(result.level.and_then(ItemNumber::value), Some(80.0));
    let miss = UniqueRequest {
        name: "caller missing unique identity".into(),
        title: None,
        base_name: None,
    };
    assert_eq!(level(native.lookup_unique(&miss)), None);
    assert_eq!(native.dependencies().calls, 0);

    // Existing explicit-dependency composition remains independently selectable.
    let mut explicit =
        NativeItemLoadProvider::with_dependencies(&selected, ExplicitDependency::default());
    assert_eq!(level(explicit.lookup_unique(&hit)), Some(999.0));
    assert_eq!(level(explicit.lookup_unique(&miss)), Some(999.0));
    assert_eq!(explicit.dependencies().calls, 2);
}

#[test]
fn unavailable_selected_data_stops_instead_of_using_an_explicit_dependency_as_fallback() {
    let mut package = snapshot().package().clone();
    let name = package.unique_requirements.complete().unwrap().entries[0]
        .canonical_key
        .clone();
    package.unique_requirements = UniqueRequirementData::unavailable("caller omitted unique facts");
    let selected = custom(package);
    let request = UniqueRequest {
        name,
        title: None,
        base_name: None,
    };
    let mut native =
        NativeItemLoadProvider::with_native_unique_lookup(&selected, ExplicitDependency::default());
    assert!(
        matches!(native.lookup_unique(&request), DependencyResult::Unavailable(reason)
        if reason == "caller omitted unique facts")
    );
    assert_eq!(native.dependencies().calls, 0);
    let mut explicit =
        NativeItemLoadProvider::with_dependencies(&selected, ExplicitDependency::default());
    assert_eq!(level(explicit.lookup_unique(&request)), Some(999.0));
    assert_eq!(explicit.dependencies().calls, 1);
}
