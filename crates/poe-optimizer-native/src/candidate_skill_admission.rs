//! Cold source validation for skill components introduced after template loading.
//! Temporary authored documents and stage state are discarded before calculation.
use crate::{CompiledGameData, profile, skills};
use poe_optimizer_core::{
    build_identity::BuildLineage,
    evaluation::{BuildDocument, EvaluationError, EvaluationRequest},
    options::EvaluationOptions,
};
use std::sync::Arc;

pub(crate) fn validate(
    build: BuildDocument,
    data: &Arc<CompiledGameData>,
    lineage: BuildLineage,
) -> Result<(), EvaluationError> {
    let request = EvaluationRequest {
        build,
        options: EvaluationOptions::default(),
        metrics: vec![],
    };
    let source = crate::preparation::import_request(&request, lineage)?;
    let view = crate::preparation::saved_view(&source, data)?;
    let skills = skills::prepare_authored_skills(
        &source,
        &view,
        data,
        skills::SkillPreparationLimits::default(),
    )?;
    profile::validate_loaded_skill_projection(&source, data, &skills)
}
