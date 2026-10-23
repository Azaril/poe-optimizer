//! Retained native occurrence laws for the historical Count component fixture.
//! Publication now uses the maintained authoring release; these shared native
//! tests remain until their later consumer fixtures move to that release.
#[path = "support/owned_empty_support_domain.rs"]
mod empty_support;
#[allow(dead_code)]
#[path = "support/owned_attribute_count_cutover.rs"]
mod family;
#[path = "support/owned_attribute_count_native.rs"]
mod native;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
