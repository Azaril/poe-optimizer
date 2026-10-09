//! Intern one selection result per exact channel/query/group in the effect DAG.
use super::*;

fn collect(
    read: &PendingRead,
    keys: &mut BTreeSet<ContributionSelectionKey>,
    work: &mut usize,
) -> Result<()> {
    charge(work, 1)?;
    match read {
        PendingRead::ContributionQuery(channel, query, group, Some(_)) => {
            keys.insert(ContributionSelectionKey {
                channel: channel.clone(),
                query: query.clone(),
                group: group.clone(),
            });
        }
        PendingRead::Required(read) => collect(read, keys, work)?,
        PendingRead::Select {
            when_true,
            when_false,
            ..
        } => {
            collect(when_true, keys, work)?;
            collect(when_false, keys, work)?;
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn register<'a>(
    base: &[EffectNode],
    appended: &mut Vec<EffectNode>,
    reads: impl Iterator<Item = &'a PendingRead>,
    limits: PlanLimits,
    work: &mut usize,
) -> Result<BTreeMap<ContributionSelectionKey, usize>> {
    charge(work, base.len() + appended.len())?;
    let mut results = BTreeMap::new();
    for (index, node) in base.iter().chain(appended.iter()).enumerate() {
        if let BoundEffectTarget::ContributionSelection { key } = &node.target
            && results.insert(key.clone(), index).is_some()
        {
            return Err(PlanError::Invalid("duplicate selection result".into()));
        }
    }
    let mut keys = BTreeSet::new();
    for read in reads {
        collect(read, &mut keys, work)?;
    }
    for key in keys {
        charge(work, 1)?;
        if results.contains_key(&key) {
            continue;
        }
        let index = base.len() + appended.len();
        if index >= limits.max_effects {
            return Err(PlanError::Limit("effects"));
        }
        appended.push(EffectNode {
            key: EffectOccurrenceKey {
                invocation: ProgramOccurrenceKey {
                    origin: RuleOrigin::ContributionSelection {
                        query: key.query.clone(),
                        group: key.group.clone(),
                        recipient: key.channel.entity.clone(),
                    },
                    owner: SchemaSubject::Definition(key.channel.stat.address()),
                    program: key.query.clone(),
                    entity: key.channel.entity.clone(),
                },
                effect: key.group.clone(),
            },
            target: BoundEffectTarget::ContributionSelection { key: key.clone() },
            operation: EffectOperation::NumericSelection {
                candidates: vec![],
                complete: false,
            },
            gates: vec![],
            dependencies: vec![],
        });
        results.insert(key, index);
    }
    Ok(results)
}

pub(super) fn bind(
    key: &ContributionSelectionKey,
    sources: FinalReadSources<'_>,
    complete: bool,
    work: &mut usize,
) -> Result<EffectOperation> {
    let bound = sources.ordered.bind(
        &key.channel,
        &key.query,
        &key.group,
        sources
            .contributions
            .get(&key.channel)
            .map_or(&[], Vec::as_slice),
        complete,
        work,
    )?;
    if bound.reduction != ContributionReduction::RequireAgreement || bound.empty.is_some() {
        return Err(PlanError::Invalid(
            "selection result requires checked agreement".into(),
        ));
    }
    Ok(EffectOperation::NumericSelection {
        candidates: bound.effects,
        complete: bound.complete,
    })
}
