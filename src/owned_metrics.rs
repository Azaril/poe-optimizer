//! Thin host for native requested measurements over injected owned artifacts.
use crate::owned_effects::{
    LoadedPlanArtifacts, PlanArgs, load_artifacts, load_plan, read_bounded,
};
use poe_optimizer_core::owned_binding::DefinitionBindingReport;
use poe_optimizer_data::{
    owned_metrics::{MetricMappingLimits, decode_metric_mapping},
    owned_schema::OwnedDefinitionSchemaPackage,
    owned_stages::{StageStorageLimits, decode_evaluation_stages},
    owned_support_inputs::{SupportInputStorageLimits, decode_support_input_bindings},
    owned_support_receiving::{SupportReceivingStorageLimits, decode_support_receiving},
    owned_supports::{SupportStorageLimits, decode_support_preparation},
};
use poe_optimizer_engine::{
    owned_plan::{
        MetricPlanIdentity, OwnedMetricPlan, OwnedMetricReport, OwnedSupportEffectPlan,
        OwnedSupportMetricPlan, PlanLimits, SupportEffectPlanInputs, SupportMetricStatus,
    },
    owned_supports::SupportPreparationLimits,
};
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
    #[command(flatten)]
    support: SupportArgs,
    /// Also save the report to a new file; existing files are preserved.
    #[arg(long)]
    output: Option<PathBuf>,
}
#[derive(clap::Args)]
struct SupportArgs {
    /// Explicit stage schedule; requires all three support packages.
    #[arg(long, requires_all = ["support_preparation", "support_inputs", "support_receiving"])]
    stages: Option<PathBuf>,
    /// Owned support selection and type-admission definitions.
    #[arg(long, requires_all = ["stages", "support_inputs", "support_receiving"])]
    support_preparation: Option<PathBuf>,
    /// Frozen computed inputs used for support preparation.
    #[arg(long, requires_all = ["stages", "support_preparation", "support_receiving"])]
    support_inputs: Option<PathBuf>,
    /// Explicit support receiver roles and delivery programs.
    #[arg(long, requires_all = ["stages", "support_preparation", "support_inputs"])]
    support_receiving: Option<PathBuf>,
}
struct SupportPaths {
    stages: PathBuf,
    preparation: PathBuf,
    inputs: PathBuf,
    receiving: PathBuf,
}
impl SupportArgs {
    fn paths(self) -> Result<Option<SupportPaths>, Box<dyn Error>> {
        match (self.stages, self.support_preparation, self.support_inputs, self.support_receiving) {
            (None, None, None, None) => Ok(None),
            (Some(stages), Some(preparation), Some(inputs), Some(receiving)) =>
                Ok(Some(SupportPaths { stages, preparation, inputs, receiving })),
            _ => Err("support evaluation requires --stages, --support-preparation, --support-inputs and --support-receiving together".into()),
        }
    }
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
    binding_report: &'a DefinitionBindingReport,
    evaluation: &'a OwnedMetricReport,
    verification: Verification,
}
#[derive(Serialize)]
struct SupportReport<'a> {
    #[serde(flatten)]
    metrics: Report<'a>,
    support_preparation: &'a SupportMetricStatus,
}
fn support_effects(
    base: LoadedPlanArtifacts,
    paths: SupportPaths,
) -> Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>, Box<dyn Error>> {
    let stage_limits = StageStorageLimits::default();
    let stages = Arc::new(decode_evaluation_stages(
        &read_bounded(&paths.stages, stage_limits.max_wire_bytes)?,
        base.definitions.as_ref(),
        &base.stored_rules,
        &base.routing,
        stage_limits,
    )?);
    let preparation_limits = SupportStorageLimits::default();
    let preparation = Arc::new(decode_support_preparation(
        &read_bounded(&paths.preparation, preparation_limits.max_wire_bytes)?,
        base.definitions.as_ref(),
        &base.stored_rules,
        preparation_limits,
    )?);
    let input_limits = SupportInputStorageLimits::default();
    let inputs = Arc::new(decode_support_input_bindings(
        &read_bounded(&paths.inputs, input_limits.max_wire_bytes)?,
        base.definitions.as_ref(),
        &base.stored_rules,
        &preparation,
        &stages,
        input_limits,
    )?);
    let receiving_limits = SupportReceivingStorageLimits::default();
    let receiving = Arc::new(decode_support_receiving(
        &read_bounded(&paths.receiving, receiving_limits.max_wire_bytes)?,
        base.definitions.as_ref(),
        &base.stored_rules,
        &preparation,
        &inputs,
        &stages,
        receiving_limits,
    )?);
    Ok(OwnedSupportEffectPlan::compile(
        SupportEffectPlanInputs {
            request: base.request,
            definitions: base.definitions,
            rules: base.rules,
            routing: base.routing,
            stages,
            preparation,
            inputs,
            receiving,
        },
        PlanLimits::default(),
        SupportPreparationLimits::default(),
    )?)
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    if let Some(paths) = args.support.paths()? {
        let effects = Arc::new(support_effects(load_artifacts(args.plan)?, paths)?);
        let limits = MetricMappingLimits::default();
        let mapping = Arc::new(decode_metric_mapping(
            &read_bounded(&args.metrics, limits.max_wire_bytes)?,
            effects.definitions(),
            limits,
        )?);
        let plan = OwnedSupportMetricPlan::compile(effects, mapping)?;
        let result = plan.evaluate(&mut plan.new_scratch())?;
        let report = SupportReport {
            metrics: Report {
                schema_version: 3,
                document_kind: "owned_metric_report",
                bindings: plan.bindings(),
                binding_report: plan.binding_report(),
                evaluation: &result.evaluation,
                verification: Verification {
                    scope: "requested_owned_metrics_with_support_preparation",
                    contributor_closure: "whole_plan",
                    game_legality: "not_checked",
                    whole_build_parity: "not_established",
                },
            },
            support_preparation: &result.support,
        };
        return write_report(&report, args.output.as_ref());
    }
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
        schema_version: 2,
        document_kind: "owned_metric_report",
        bindings: plan.bindings(),
        binding_report: plan.effect_plan().binding_report(),
        evaluation: &evaluation,
        verification: Verification {
            scope: "requested_owned_metrics",
            contributor_closure: "whole_plan",
            game_legality: "not_checked",
            whole_build_parity: "not_established",
        },
    };
    write_report(&report, args.output.as_ref())
}
fn write_report(report: &impl Serialize, output: Option<&PathBuf>) -> Result<(), Box<dyn Error>> {
    let mut bytes = serde_json::to_vec_pretty(report)?;
    bytes.push(b'\n');
    if let Some(path) = output {
        crate::write_new(path, &bytes)?;
    }
    io::stdout().lock().write_all(&bytes)?;
    Ok(())
}
