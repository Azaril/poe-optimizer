//! Shared finite reward/item contribution fixture. No published resource reducer.
#[allow(dead_code)]
#[path = "owned_ranged_spirit_native.rs"]
mod spirit;

use poe_optimizer_core::{
    build_identity::InstanceAllocator, owned_build::*, owned_definitions::*, owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_engine::owned_plan::OwnedEffectsReport;
use poe_optimizer_import::owned_mapping::OwnedIdRegistry;
use serde::de::DeserializeOwned;
use serde_json::Value;
pub use spirit::component::Fixture;
use std::{fs, path::PathBuf};

pub fn packet<T: DeserializeOwned>(family: &str, name: &str) -> T {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68")
        .join(family)
        .join(name);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn add_definition(f: &mut Fixture, definition: DefinitionDescriptor) {
    let address = definition.address();
    if let Some(existing) = f
        .recipe
        .schema
        .definitions
        .iter()
        .find(|d| d.address() == address)
    {
        assert_eq!(existing, &definition);
        return;
    }
    assert!(
        !f.recipe
            .schema
            .definitions
            .iter()
            .any(|d| d.address().key() == address.key())
    );
    assert!(
        !f.recipe
            .schema
            .slots
            .iter()
            .any(|s| s.address().key() == address.key())
    );
    let number =
        u64::from_str_radix(address.key().as_str().strip_prefix("def.").unwrap(), 16).unwrap();
    // The shared component already uses dense registry-only Option placeholders.
    // Extend that finite history for later authored addresses, then replace only
    // the unused placeholder at this exact key. This never mutates real data.
    if f.recipe.registry.last_issued.get() < number as i64 {
        let mut registry =
            OwnedIdRegistry::new(f.recipe.registry.clone(), Default::default()).unwrap();
        while registry.input().last_issued.get() < number as i64 {
            registry.allocate_definition::<OptionDefinition>().unwrap();
        }
        f.recipe.registry = registry.input().clone();
    }
    let entry = f
        .recipe
        .registry
        .entries
        .iter_mut()
        .find(|entry| entry.sequence.get() == number as i64)
        .unwrap();
    assert!(matches!(
        &entry.target,
        SchemaSubject::Definition(DefinitionAddress::Option(_))
    ));
    entry.target = SchemaSubject::Definition(address);
    f.recipe.schema.definitions.push(definition);
}

pub fn append_packet(f: &mut Fixture, family: &str) {
    let dependencies: Value = packet(family, "dependencies.json");
    let closure: Value = packet(family, "closure.json");
    let support_packet: Value = packet("reward-support-domains", "extension.json");
    let support_domains: Vec<SupportSourceDomainDeclaration> =
        serde_json::from_value(support_packet["support_source_domains"].clone()).unwrap();
    for field in [&dependencies["definitions"], &closure["definitions"]] {
        let definitions: Vec<DefinitionDescriptor> = serde_json::from_value(field.clone()).unwrap();
        for definition in definitions {
            add_definition(f, definition);
        }
    }
    let owners: Vec<DefinitionRules> = serde_json::from_value(closure["owners"].clone()).unwrap();
    let mut allocator = InstanceAllocator::from_state(f.build.allocator);
    for owner in owners {
        assert!(owner.programs.is_complete());
        assert!(
            !f.recipe
                .rules
                .owners
                .iter()
                .any(|old| old.owner == owner.owner)
        );
        let SchemaSubject::Definition(DefinitionAddress::Reward(reward)) = &owner.owner else {
            panic!("only actual Reward providers belong in this fixture extension")
        };
        f.build.character.rewards.push(RewardSelection {
            id: allocator.allocate().unwrap(),
            definition: reward.clone(),
            parameters: vec![],
        });
        // Reuse the published certificate for this exact numerical Reward;
        // absent certificates must never become fixture-inferred completeness.
        let domain = support_domains
            .iter()
            .find(|d| d.owner == owner.owner)
            .expect("an appended Reward requires its published support-source certificate");
        let providers = &mut f.recipe.rules.support_discovery.as_mut().unwrap().providers;
        assert!(!providers.iter().any(|old| old.owner == domain.owner));
        providers.push(domain.clone());
        f.recipe.rules.owners.push(owner);
    }
    f.build.allocator = allocator.state();
    OwnedIdRegistry::new(f.recipe.registry.clone(), Default::default()).unwrap();
}

pub fn fixture(families: &[&str]) -> Fixture {
    let mut f = spirit::fixture(None);
    spirit::clear_properties(&mut f);
    for (item, modifier) in [(0, 0), (0, 1), (1, 0)] {
        f.set_raw(item, modifier, 12.5);
    }
    for family in families {
        append_packet(&mut f, family);
    }
    // Only the original finite item component is closed here. Every appended
    // Reward owner and its schema comes unchanged from the actual packets.
    f.complete_domain();
    f
}

pub fn report(f: &Fixture) -> OwnedEffectsReport {
    let plan = f.plan().unwrap();
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}
