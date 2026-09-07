//! Shared evaluator contracts, independent of the CLI and Lua host.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PROTOCOL_VERSION: u32 = 2;
pub const MAX_WIRE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerHello {
    pub protocol_version: u32,
    pub backend: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerRequest {
    pub protocol_version: u32,
    pub request_id: u64,
    pub xml: String,
    pub options: options::EvaluationOptions,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerResponse {
    pub protocol_version: u32,
    pub request_id: u64,
    pub result: Result<EvaluationSnapshot, WorkerFailure>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerFailure {
    pub code: String,
    pub message: String,
}

/// Fresh raw PoB output, not a certified legal build or optimization recommendation.
#[derive(Debug, Serialize, Deserialize)]
pub struct EvaluationSnapshot {
    pub runtime: RuntimeIdentity,
    pub build: BuildSummary,
    pub coverage: coverage::BuildCoverage,
    pub context: options::EvaluationContext,
    pub player: ActorOutput,
    pub minion: Option<ActorOutput>,
    pub warnings: Vec<String>,
    pub export_xml: String,
    pub elapsed_ms: f64,
    pub diagnostics: String,
    pub diagnostics_truncated: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeIdentity {
    pub upstream_revision: String,
    pub source_hash: String,
    pub mlua_version: String,
    pub lua_version: String,
    pub lua_arch: String,
    pub operating_system: String,
    pub luajit_source: String,
    pub utf8_version: String,
    pub adapter_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildSummary {
    pub level: u32,
    pub class_name: String,
    pub ascendancy_name: String,
    pub tree_version: String,
    pub main_socket_group: usize,
    pub allocated_nodes: Vec<u32>,
    pub skill_groups: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActorOutput {
    pub skill_name: Option<String>,
    pub skill_id: Option<String>,
    pub has_hit_damage: bool,
    pub metrics: BTreeMap<String, f64>,
    pub non_finite_metrics: Vec<String>,
    pub non_finite_values: BTreeMap<String, metrics::NonFiniteKind>,
}

pub mod coverage;
pub mod evaluation;
pub mod metrics;
pub mod options;
