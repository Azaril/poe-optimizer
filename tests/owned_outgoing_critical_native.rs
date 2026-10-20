//! Full authored critical calculation in an explicitly finite native graph.
//! Test inputs and finite closures do not admit production condition coverage.
#[allow(dead_code)]
#[path = "support/owned_outgoing_critical.rs"]
mod family;
#[path = "support/owned_outgoing_critical_native.rs"]
mod native;
#[allow(dead_code)]
#[path = "support/owned_plan_replay.rs"]
mod replay;
