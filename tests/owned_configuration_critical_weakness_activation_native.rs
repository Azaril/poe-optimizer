//! The actual configured source feeding the full critical consumer.
#[path = "support/owned_configuration_critical_weakness_activation_native.rs"]
mod configured;
#[path = "support/owned_outgoing_critical_native.rs"]
mod critical_native;
#[allow(dead_code)]
#[path = "support/owned_outgoing_critical.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_plan_replay.rs"]
mod replay;
