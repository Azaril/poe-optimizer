//! Thin host for native requested measurements over injected owned artifacts.
use crate::owned_effects::{PlanArgs, load_plan, read_bounded};
use poe_optimizer_data::owned_metrics::{MetricMappingLimits, decode_metric_mapping};
use poe_optimizer_engine::owned_plan::{MetricPlanIdentity, OwnedMetricPlan, OwnedMetricReport};
use serde::Serialize;
use std::{
    error::Error,
    io::{self, Write},
    path::PathBuf,
    sync::Arc,
};

#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(flatten)]
    plan: PlanArgs,
    /// Injected metric-to-final-stat mapping bound to the same definition schema.
    #[arg(long)]
    metrics: PathBuf,
    /// Also save the report to a new file; existing files are preserved.
    #[arg(long)]
    output: Option<PathBuf>,
}
#[derive(Serialize)]
struct Verification {
    scope: &'static str,
    contributor_closure: &'static str,
    game_legality: &'static str,
    whole_build_parity: &'static str,
}
#[derive(Serialize)]
struct Report<'a> {
    schema_version: u32,
    document_kind: &'static str,
    bindings: &'a MetricPlanIdentity,
    evaluation: &'a OwnedMetricReport,
    verification: Verification,
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let effects = Arc::new(load_plan(args.plan)?);
    let limits = MetricMappingLimits::default();
    let mapping = Arc::new(decode_metric_mapping(
        &read_bounded(&args.metrics, limits.max_wire_bytes)?,
        effects.definitions(),
        limits,
    )?);
    let plan = OwnedMetricPlan::compile(effects, mapping)?;
    let evaluation = plan.evaluate(&mut plan.new_scratch())?;
    let report = Report {
        schema_version: 1,
        document_kind: "owned_metric_report",
        bindings: plan.bindings(),
        evaluation: &evaluation,
        verification: Verification {
            scope: "requested_owned_metrics",
            contributor_closure: "whole_plan",
            game_legality: "not_checked",
            whole_build_parity: "not_established",
        },
    };
    let mut bytes = serde_json::to_vec_pretty(&report)?;
    bytes.push(b'\n');
    if let Some(path) = &args.output {
        crate::write_new(path, &bytes)?;
    }
    io::stdout().lock().write_all(&bytes)?;
    Ok(())
}
