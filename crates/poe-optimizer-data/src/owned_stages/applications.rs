//! Explicit stage membership for inline application programs.
use super::*;

pub(super) fn validate<I: DefinitionSchemaIndex>(
    input: &EvaluationStagesInput,
    index: &I,
    rules: &OwnedRulePackage,
    stages: &BTreeMap<OwnedDefinitionKey, usize>,
    used: &mut StageStorageUse,
    limits: StageStorageLimits,
) -> Result<BTreeMap<OwnedDefinitionKey, usize>> {
    let invalid = StageStorageError::Invalid;
    let supported = RuleOperationsVersion::parse(rules.input().operations_version.as_str())
        .is_some_and(RuleOperationsVersion::supports_effect_applications);
    let Some(partition) = &input.effect_applications else {
        return if supported {
            Err(invalid(
                "v15 requires explicit effect application stage membership",
            ))
        } else {
            Ok(BTreeMap::new())
        };
    };
    if !supported {
        return Err(invalid(
            "effect application stages require owned-domain-operations-v15",
        ));
    }
    let Some(registry) = &rules.input().effect_applications else {
        return Err(invalid(
            "effect application stage membership has no rule inventory",
        ));
    };
    used.entries(partition.members.len(), limits)?;
    used.entries(registry.members.len(), limits)?;
    if let SchemaClosure::Partial { gaps } = &partition.closure {
        used.entries(gaps.len(), limits)?;
        if gaps.is_empty() {
            return Err(invalid(
                "partial effect application stages need gap evidence",
            ));
        }
        let mut seen = BTreeSet::new();
        for gap in gaps {
            used.work(1, limits)?;
            let valid = match &gap.subject {
                SchemaSubject::Definition(id) => {
                    id.namespace() == &input.namespace
                        && index
                            .lookup_definition(id)
                            .is_some_and(|d| d.address() == *id)
                }
                SchemaSubject::Slot(id) => {
                    id.namespace() == &input.namespace
                        && id.declaration().namespace() == &input.namespace
                        && index.lookup_slot(id).is_some_and(|d| d.address() == *id)
                }
            };
            if !valid
                || gap.facet != SchemaFacet::GameRules
                || !seen.insert((owner_key(&gap.subject), &gap.code))
            {
                return Err(invalid("invalid effect application stage gap"));
            }
        }
    }
    let known: BTreeMap<_, _> = registry.members.iter().map(|row| (&row.id, row)).collect();
    let mut assigned = BTreeMap::new();
    for row in &partition.members {
        used.work(2, limits)?;
        if !known.contains_key(&row.application) {
            return Err(invalid("unknown staged effect application"));
        }
        let Some(&stage) = stages.get(&row.stage) else {
            return Err(invalid("unknown effect application stage"));
        };
        if assigned.insert(row.application.clone(), stage).is_some() {
            return Err(invalid("duplicate effect application stage classification"));
        }
    }
    if partition.is_complete() && assigned.len() != known.len() {
        return Err(invalid(
            "complete stage partition omits effect applications",
        ));
    }
    // A maximum is one operation over every candidate in the family/channel.
    // It cannot be finalized at one stage and receive another source later.
    let mut groups = BTreeMap::new();
    for application in &registry.members {
        used.entries(application.stacking.len(), limits)?;
        if let Some(&stage) = assigned.get(&application.id) {
            for stacking in &application.stacking {
                let group = (&stacking.family, &stacking.modifier);
                if let Some(previous) = groups.insert(group, stage)
                    && previous != stage
                {
                    return Err(invalid("effect stacking group spans incompatible stages"));
                }
            }
        }
    }
    Ok(assigned)
}
