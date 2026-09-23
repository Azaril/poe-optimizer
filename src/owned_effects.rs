//! Host I/O for owned occurrence effect resolution; no metric conversion or source backend.
use poe_optimizer_core::{
    owned_binding::DefinitionBindingReport,
    owned_build::{OwnedDocument, OwnedInputLimits, decode_owned},
};
use poe_optimizer_data::{
    owned_routing::{RoutingLimits, decode_action_routing},
    owned_rules::{RuleStorageLimits, decode_rule_package},
    owned_schema::{OwnedSchemaLimits, decode_schema_package},
};
use poe_optimizer_engine::{
    owned_plan::{OwnedEffectPlan, OwnedEffectsReport, PlanIdentity, PlanLimits},
    owned_rules::{CompiledRulePackage, RuleLimits},
};
use serde::Serialize;
use std::{
    error::Error,
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(clap::Args)]
pub(crate) struct PlanArgs {
    /// Complete owned Request envelope with build, scenario and ordered queries.
    #[arg(long)]
    input: PathBuf,
    /// Exact injected owned definition schema package.
    #[arg(long)]
    schema: PathBuf,
    /// Injected owned rule package bound to this schema.
    #[arg(long)]
    rules: PathBuf,
    /// Injected owned action-routing package, including explicit empty routing.
    #[arg(long)]
    routing: PathBuf,
}
#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(flatten)]
    plan: PlanArgs,
    /// Also save the component report to a new file; existing files are preserved.
    #[arg(long)]
    output: Option<PathBuf>,
}
pub(crate) fn read_bounded(path: &Path, maximum: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > maximum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "owned effects input exceeds byte bound",
        ));
    }
    Ok(bytes)
}
#[derive(Serialize)]
struct Verification {
    scope: &'static str,
    metric_conversion: &'static str,
    game_legality: &'static str,
    whole_build_parity: &'static str,
}
#[derive(Serialize)]
struct Report<'a> {
    schema_version: u32,
    document_kind: &'static str,
    bindings: &'a PlanIdentity,
    binding_report: &'a DefinitionBindingReport,
    resolution: &'a OwnedEffectsReport,
    verification: Verification,
}
pub(crate) fn load_plan(
    args: PlanArgs,
) -> Result<
    OwnedEffectPlan<poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage>,
    Box<dyn Error>,
> {
    let input_limits = OwnedInputLimits::default();
    let OwnedDocument::Request(request) = decode_owned(
        &read_bounded(&args.input, input_limits.max_wire_bytes)?,
        input_limits,
    )?
    else {
        return Err("effect resolution requires a complete owned Request document".into());
    };
    let schema_limits = OwnedSchemaLimits::default();
    let definitions = Arc::new(decode_schema_package(
        &read_bounded(&args.schema, schema_limits.max_wire_bytes)?,
        schema_limits,
    )?);
    let storage_limits = RuleStorageLimits::default();
    let stored_rules = decode_rule_package(
        &read_bounded(&args.rules, storage_limits.max_wire_bytes)?,
        definitions.as_ref(),
        storage_limits,
    )?;
    let rules = Arc::new(CompiledRulePackage::compile(
        stored_rules.input(),
        definitions.as_ref(),
        RuleLimits::default(),
    )?);
    let routing_limits = RoutingLimits::default();
    let routing = Arc::new(decode_action_routing(
        &read_bounded(&args.routing, routing_limits.max_wire_bytes)?,
        definitions.as_ref(),
        routing_limits,
    )?);
    let plan = OwnedEffectPlan::compile(
        Arc::from(request),
        definitions,
        rules,
        routing,
        PlanLimits::default(),
    )?;
    Ok(plan)
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let plan = load_plan(args.plan)?;
    let mut scratch = plan.new_scratch();
    let resolution = plan.evaluate(&mut scratch)?;
    let report = Report {
        schema_version: 2,
        document_kind: "owned_effects_report",
        bindings: plan.bindings(),
        binding_report: plan.binding_report(),
        resolution: &resolution,
        verification: Verification {
            scope: "owned_effect_component",
            metric_conversion: "not_run",
            game_legality: "not_checked",
            whole_build_parity: "not_established",
        },
    };
    let mut bytes = serde_json::to_vec_pretty(&report)?;
    bytes.push(b'\n');
    if let Some(path) = &args.output {
        super::write_new(path, &bytes)?;
    }
    io::stdout().lock().write_all(&bytes)?;
    Ok(())
}
