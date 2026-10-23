//! Resource source laws and their explicit retained publication cases.
//! Shared release and preservation helpers are registered once for this suite.
#[path = "support/owned_empty_support_domain.rs"]
mod empty_support;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[allow(dead_code)]
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;

#[path = "resource_sources/flat_life_routing.rs"]
mod flat_life_routing;
#[path = "resource_sources/flat_life_scalability.rs"]
mod flat_life_scalability;
#[path = "resource_sources/life_base_domain.rs"]
mod life_base_domain;
#[path = "resource_sources/life_contribution_queries.rs"]
mod life_contribution_queries;
#[path = "resource_sources/life_equipment_domain.rs"]
mod life_equipment_domain;
#[path = "resource_sources/player_intrinsic_life.rs"]
mod player_intrinsic_life;
#[path = "resource_sources/player_intrinsic_mana.rs"]
mod player_intrinsic_mana;
#[path = "resource_sources/player_life_pool.rs"]
mod player_life_pool;
#[path = "resource_sources/player_spirit_pool.rs"]
mod player_spirit_pool;
#[path = "resource_sources/strength_life.rs"]
mod strength_life;
