//! Test-only replay of public inputs. No compiled plan, scratch, result or
//! source-language state is accepted as execution authority.
use poe_optimizer_core::{
    owned_build::*, owned_routing::*, owned_rules::*, owned_stages::*, owned_support_inputs::*,
    owned_support_receiving::*, owned_supports::*,
};
use poe_optimizer_data::{
    owned_routing::OwnedActionRouting,
    owned_rules::OwnedRulePackage,
    owned_schema::{OwnedDefinitionSchemaPackage, SchemaPackageInput},
    owned_stages::OwnedEvaluationStages,
    owned_support_inputs::OwnedSupportInputBindings,
    owned_support_receiving::OwnedSupportReceiving,
    owned_supports::OwnedSupportPreparation,
};
use poe_optimizer_engine::{
    owned_plan::*, owned_rules::CompiledRulePackage, owned_supports::SupportPreparationLimits,
};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    ops::Deref,
    sync::Arc,
};

pub type NativePlan = OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>;
type Inputs = SupportEffectPlanInputs<OwnedDefinitionSchemaPackage>;

/// Retain cheap immutable input references in the existing finite fixture.
/// The production plan still owns all validation and execution behavior.
pub struct RetainedPlan {
    inputs: Inputs,
    stored_rules: OwnedRulePackage,
    plan: NativePlan,
}
impl Deref for RetainedPlan {
    type Target = NativePlan;
    fn deref(&self) -> &Self::Target {
        &self.plan
    }
}
impl RetainedPlan {
    pub fn compile(
        inputs: Inputs,
        limits: PlanLimits,
        support_limits: SupportPreparationLimits,
        stored_rules: OwnedRulePackage,
    ) -> std::result::Result<Self, PlanError> {
        assert_eq!(
            inputs.rules.source_identity(),
            Some(*stored_rules.identity()),
            "retained rule artifact must be the compiler's validated source"
        );
        let plan = NativePlan::compile(
            Inputs {
                request: inputs.request.clone(),
                definitions: inputs.definitions.clone(),
                rules: inputs.rules.clone(),
                routing: inputs.routing.clone(),
                stages: inputs.stages.clone(),
                preparation: inputs.preparation.clone(),
                inputs: inputs.inputs.clone(),
                receiving: inputs.receiving.clone(),
            },
            limits,
            support_limits,
        )?;
        Ok(Self {
            inputs,
            stored_rules,
            plan,
        })
    }
    pub fn snapshot(&self) -> ReplayInput {
        let i = &self.inputs;
        ReplayInput {
            schema: i.definitions.input().clone(),
            // Preserve the validated artifact. The compiler's normalized input
            // has a separate identity and is not its source artifact.
            rules: self.stored_rules.input().clone(),
            routing: i.routing.input().clone(),
            stages: i.stages.input().clone(),
            preparation: i.preparation.input().clone(),
            inputs: i.inputs.input().clone(),
            receiving: i.receiving.input().clone(),
            build: i.request.build().input().clone(),
            scenario: i.request.scenario().input().clone(),
            queries: i.request.queries().input().clone(),
        }
    }
}

/// Current development DTOs only; every cross-package identity is revalidated.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayInput {
    pub schema: SchemaPackageInput,
    pub rules: RulePackageInput,
    pub routing: ActionRoutingInput,
    pub stages: EvaluationStagesInput,
    pub preparation: SupportPreparationInput,
    pub inputs: SupportInputBindingsInput,
    pub receiving: SupportReceivingInput,
    pub build: BuildInput,
    pub scenario: ScenarioInput,
    pub queries: QueryInput,
}
impl ReplayInput {
    pub fn encode(&self) -> Vec<u8> {
        let mut output = flate2::GzBuilder::new()
            .mtime(0)
            .operating_system(255)
            .write(Vec::new(), flate2::Compression::best());
        output
            .write_all(&serde_json::to_vec(self).unwrap())
            .unwrap();
        output.finish().unwrap()
    }
    pub fn decode(bytes: &[u8]) -> Self {
        const MAX_BYTES: u64 = 8 * 1024 * 1024;
        let mut json = Vec::new();
        flate2::read::GzDecoder::new(bytes)
            .take(MAX_BYTES + 1)
            .read_to_end(&mut json)
            .unwrap();
        assert!(json.len() as u64 <= MAX_BYTES, "bounded fixture input");
        serde_json::from_slice(&json).unwrap()
    }
    pub fn compile(&self) -> std::result::Result<NativePlan, String> {
        let definitions = Arc::new(
            OwnedDefinitionSchemaPackage::new(self.schema.clone(), Default::default())
                .map_err(|e| e.to_string())?,
        );
        let stored = OwnedRulePackage::new(self.rules.clone(), &*definitions, Default::default())
            .map_err(|e| e.to_string())?;
        let rules = Arc::new(
            CompiledRulePackage::compile_stored(&stored, &*definitions, Default::default())
                .map_err(|e| e.to_string())?,
        );
        let routing = Arc::new(
            OwnedActionRouting::new(self.routing.clone(), &*definitions, Default::default())
                .map_err(|e| e.to_string())?,
        );
        let stages = Arc::new(
            OwnedEvaluationStages::new(
                self.stages.clone(),
                &*definitions,
                &stored,
                &routing,
                Default::default(),
            )
            .map_err(|e| e.to_string())?,
        );
        let preparation = Arc::new(
            OwnedSupportPreparation::new(
                self.preparation.clone(),
                &*definitions,
                &stored,
                Default::default(),
            )
            .map_err(|e| e.to_string())?,
        );
        let inputs = Arc::new(
            OwnedSupportInputBindings::new(
                self.inputs.clone(),
                &*definitions,
                &stored,
                &preparation,
                &stages,
                Default::default(),
            )
            .map_err(|e| e.to_string())?,
        );
        let receiving = Arc::new(
            OwnedSupportReceiving::new(
                self.receiving.clone(),
                &*definitions,
                &stored,
                &preparation,
                &inputs,
                &stages,
                Default::default(),
            )
            .map_err(|e| e.to_string())?,
        );
        let request = OwnedEvaluationRequest::new(
            BuildSpec::new(self.build.clone(), Default::default()).map_err(|e| e.to_string())?,
            ScenarioSpec::new(self.scenario.clone(), Default::default())
                .map_err(|e| e.to_string())?,
            QuerySpec::new(self.queries.clone(), Default::default()).map_err(|e| e.to_string())?,
            Default::default(),
        )
        .map_err(|e| e.to_string())?;
        NativePlan::compile(
            Inputs {
                request: Arc::new(request),
                definitions,
                rules,
                routing,
                stages,
                preparation,
                inputs,
                receiving,
            },
            Default::default(),
            Default::default(),
        )
        .map_err(|e| e.to_string())
    }
}
