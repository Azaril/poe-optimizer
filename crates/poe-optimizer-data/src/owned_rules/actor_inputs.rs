//! Role eligibility is independent of the common mechanical Actor context.
use super::*;
use poe_optimizer_core::owned_schema::AssumptionTargetKind;

pub(super) fn validate<I: DefinitionSchemaIndex>(
    program: &RuleProgram,
    index: &I,
    local: Option<AssumptionTargetKind>,
    source: Option<AssumptionTargetKind>,
    mut charge: impl FnMut(usize) -> Result<(), RuleStorageError>,
) -> Result<(), RuleStorageError> {
    charge(program.reads.len())?;
    for read in &program.reads {
        let RuleReadSource::External { entity, input } = &read.source else {
            continue;
        };
        let target = match entity {
            RuleEntity::Current => local,
            RuleEntity::Actor if program.context == RuleEntityKind::Actor => local,
            RuleEntity::Player => Some(AssumptionTargetKind::PlayerActor),
            RuleEntity::Enemy => Some(AssumptionTargetKind::ScenarioEnemyActor),
            RuleEntity::Environment => Some(AssumptionTargetKind::Environment),
            RuleEntity::EffectSource => source,
            _ => None,
        }
        .ok_or(RuleStorageError::Structure(
            "external input has no admitted bound role",
        ))?;
        let SchemaLookup::Known(schema) = index.definition(input) else {
            return Err(RuleStorageError::Structure(
                "bound external input requires a known schema",
            ));
        };
        charge(schema.targets.len())?;
        if !schema.targets.contains(&target) {
            return Err(RuleStorageError::Structure(
                "external input excludes its bound actor role",
            ));
        }
    }
    Ok(())
}
