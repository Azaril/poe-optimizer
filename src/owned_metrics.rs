//! Thin host for native requested measurements over injected owned artifacts.
use crate::owned_effects::{
    LoadedPlanArtifacts, PlanArgs, load_artifacts, load_request, read_bounded,
};
use poe_optimizer_core::owned_binding::DefinitionBindingReport;
use poe_optimizer_data::{
    owned_metrics::{MetricMappingLimits, OwnedMetricMapping, decode_metric_mapping},
    owned_schema::OwnedDefinitionSchemaPackage,
    owned_stages::{OwnedEvaluationStages, StageStorageLimits, decode_evaluation_stages},
    owned_support_inputs::{
        OwnedSupportInputBindings, SupportInputStorageLimits, decode_support_input_bindings,
    },
    owned_support_outputs::{
        OwnedSupportOutputBindings, SupportOutputDependencies, SupportOutputStorageLimits,
        decode_support_output_bindings,
    },
    owned_support_receiving::{
        OwnedSupportReceiving, SupportReceivingStorageLimits, decode_support_receiving,
    },
    owned_supports::{OwnedSupportPreparation, SupportStorageLimits, decode_support_preparation},
};
use poe_optimizer_engine::{
    owned_plan::{
        MetricPlanIdentity, OwnedMetricPlan, OwnedMetricReport, OwnedSupportEffectPlan,
        OwnedSupportMetricPlan, PlanLimits, SupportEffectPlanInputs, SupportMetricStatus,
    },
    owned_rules::{CompiledRulePackage, RuleLimits},
    owned_supports::SupportPreparationLimits,
};
use poe_optimizer_import::owned_release::OwnedReleaseLimits;
use serde::Serialize;
use std::{
    error::Error,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Complete owned Request envelope with build, scenario and ordered queries.
    #[arg(long)]
    input: PathBuf,
    /// Checked immutable release directory containing evaluation artifacts.
    #[arg(long, conflicts_with_all = ["schema", "rules", "routing", "metrics", "stages", "support_preparation", "support_inputs", "support_receiving", "support_outputs"])]
    release: Option<PathBuf>,
    /// Exact injected owned definition schema package.
    #[arg(long, required_unless_present = "release")]
    schema: Option<PathBuf>,
    /// Injected owned rule package bound to this schema.
    #[arg(long, required_unless_present = "release")]
    rules: Option<PathBuf>,
    /// Injected owned action-routing package, including explicit empty routing.
    #[arg(long, required_unless_present = "release")]
    routing: Option<PathBuf>,
    /// Injected metric-to-final-stat mapping bound to the same definition schema.
    #[arg(long, required_unless_present = "release")]
    metrics: Option<PathBuf>,
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
    /// Final prepared Skill type channels, bound to the complete support package set.
    #[arg(long, requires_all = ["stages", "support_preparation", "support_inputs", "support_receiving"])]
    support_outputs: Option<PathBuf>,
}
struct SupportPaths {
    stages: PathBuf,
    preparation: PathBuf,
    inputs: PathBuf,
    receiving: PathBuf,
    outputs: Option<PathBuf>,
}
impl SupportArgs {
    fn paths(self) -> Result<Option<SupportPaths>, Box<dyn Error>> {
        match (self.stages, self.support_preparation, self.support_inputs, self.support_receiving) {
            (None, None, None, None) if self.support_outputs.is_none() => Ok(None),
            (Some(stages), Some(preparation), Some(inputs), Some(receiving)) =>
                Ok(Some(SupportPaths { stages, preparation, inputs, receiving, outputs: self.support_outputs })),
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
struct LoadedSupportArtifacts {
    stages: Arc<OwnedEvaluationStages>,
    preparation: Arc<OwnedSupportPreparation>,
    inputs: Arc<OwnedSupportInputBindings>,
    receiving: Arc<OwnedSupportReceiving>,
    outputs: Option<Arc<OwnedSupportOutputBindings>>,
}
fn load_support(
    base: &LoadedPlanArtifacts,
    paths: SupportPaths,
) -> Result<LoadedSupportArtifacts, Box<dyn Error>> {
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
    let outputs = paths
        .outputs
        .as_ref()
        .map(|path| {
            let limits = SupportOutputStorageLimits::default();
            Ok::<_, Box<dyn Error>>(Arc::new(decode_support_output_bindings(
                &read_bounded(path, limits.max_wire_bytes)?,
                &SupportOutputDependencies {
                    definitions: base.definitions.as_ref(),
                    rules: &base.stored_rules,
                    preparation: &preparation,
                    inputs: &inputs,
                    receiving: &receiving,
                    stages: &stages,
                },
                limits,
            )?))
        })
        .transpose()?;
    Ok(LoadedSupportArtifacts {
        stages,
        preparation,
        inputs,
        receiving,
        outputs,
    })
}
impl LoadedSupportArtifacts {
    fn compile(
        self,
        base: LoadedPlanArtifacts,
    ) -> Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>, Box<dyn Error>> {
        let args = SupportEffectPlanInputs {
            request: base.request,
            definitions: base.definitions,
            rules: base.rules,
            routing: base.routing,
            stages: self.stages,
            preparation: self.preparation,
            inputs: self.inputs,
            receiving: self.receiving,
        };
        Ok(if let Some(outputs) = self.outputs {
            OwnedSupportEffectPlan::compile_with_outputs(
                args,
                outputs,
                PlanLimits::default(),
                SupportPreparationLimits::default(),
            )?
        } else {
            OwnedSupportEffectPlan::compile(
                args,
                PlanLimits::default(),
                SupportPreparationLimits::default(),
            )?
        })
    }
}
struct MetricArtifacts {
    base: LoadedPlanArtifacts,
    metrics: Arc<OwnedMetricMapping>,
    support: Option<LoadedSupportArtifacts>,
}
fn load_release_artifacts(path: &Path, input: &Path) -> Result<MetricArtifacts, Box<dyn Error>> {
    let limits = OwnedReleaseLimits::default();
    let mut remaining = limits.max_input_bytes;
    let release = crate::owned_release_cli::load_release(path, &mut remaining, limits)?;
    let evaluation = release.evaluation().ok_or(
        "release does not contain evaluation artifacts; supply an explicit evaluation release",
    )?;
    // Consume the validated immutable snapshot, never re-open paths after checking
    // the release inventory and reproducing its canonical bytes.
    let recipe = release.assembled();
    let definitions = Arc::new(recipe.schema().clone());
    let stored_rules = recipe.rules().clone();
    let rules = Arc::new(CompiledRulePackage::compile_stored(
        &stored_rules,
        definitions.as_ref(),
        RuleLimits::default(),
    )?);
    Ok(MetricArtifacts {
        base: LoadedPlanArtifacts {
            request: load_request(input)?,
            definitions,
            stored_rules,
            rules,
            routing: Arc::new(recipe.routing().clone()),
        },
        metrics: Arc::new(evaluation.metrics().clone()),
        support: evaluation.support().map(|support| LoadedSupportArtifacts {
            stages: Arc::new(support.stages().clone()),
            preparation: Arc::new(support.preparation().clone()),
            inputs: Arc::new(support.inputs().clone()),
            receiving: Arc::new(support.receiving().clone()),
            outputs: support.outputs().map(|outputs| Arc::new(outputs.clone())),
        }),
    })
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let Args {
        input,
        release,
        schema,
        rules,
        routing,
        metrics,
        support,
        output,
    } = args;
    let MetricArtifacts {
        base,
        metrics,
        support,
    } = if let Some(release) = release {
        load_release_artifacts(&release, &input)?
    } else {
        let base = load_artifacts(PlanArgs {
            input,
            schema: schema.ok_or("missing --schema")?,
            rules: rules.ok_or("missing --rules")?,
            routing: routing.ok_or("missing --routing")?,
        })?;
        let limits = MetricMappingLimits::default();
        let metrics = Arc::new(decode_metric_mapping(
            &read_bounded(&metrics.ok_or("missing --metrics")?, limits.max_wire_bytes)?,
            base.definitions.as_ref(),
            limits,
        )?);
        let support = support
            .paths()?
            .map(|paths| load_support(&base, paths))
            .transpose()?;
        MetricArtifacts {
            base,
            metrics,
            support,
        }
    };
    if let Some(support) = support {
        let effects = Arc::new(support.compile(base)?);
        let plan = OwnedSupportMetricPlan::compile(effects, metrics)?;
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
        return write_report(&report, output.as_ref());
    }
    let effects = Arc::new(base.compile()?);
    let plan = OwnedMetricPlan::compile(effects, metrics)?;
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
    write_report(&report, output.as_ref())
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
