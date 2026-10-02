//! Semantic compilation of inline application programs into the existing DAG.
use super::*;

pub(super) fn source_owner(source: &EffectApplicationSource) -> SchemaSubject {
    match source {
        EffectApplicationSource::Skill { skill } => SchemaSubject::Definition(skill.address()),
        EffectApplicationSource::OwnedSlot { slot } => {
            SchemaSubject::Slot(ActorSlotDefId::address(slot))
        }
    }
}

pub(super) fn preflight<I: DefinitionSchemaIndex>(
    input: &RulePackageInput,
    index: &I,
    l: RuleLimits,
    b: &mut Budget,
) -> Result<(), RuleError> {
    let enabled = RuleOperationsVersion::parse(input.operations_version.as_str())
        .is_some_and(|v| v.supports_effect_applications());
    check(
        enabled == input.effect_applications.is_some(),
        "effect_applications",
        "v15 requires an explicit application registry; older operations forbid it",
    )?;
    let Some(rows) = &input.effect_applications else {
        return Ok(());
    };
    check(
        rows.members.len() <= l.max_programs,
        "effect_applications",
        "application count exceeds limit",
    )?;
    closure(&rows.closure, index, "effect_applications.closure", l, b)?;
    let mut targets = 0;
    let mut mappings = 0;
    for row in &rows.members {
        add(&mut b.programs, 1, l.max_programs, "effect_applications")?;
        add(
            &mut targets,
            row.targets.len(),
            l.max_receiver_targets,
            "application.targets",
        )?;
        add(
            &mut mappings,
            row.stacking.len(),
            l.max_effects,
            "application.stacking",
        )?;
        add(
            &mut b.reads,
            row.program.reads.len(),
            l.max_reads,
            "application.reads",
        )?;
        add(
            &mut b.nodes,
            row.program.nodes.len(),
            l.max_nodes,
            "application.nodes",
        )?;
        add(
            &mut b.effects,
            row.program.effects.len(),
            l.max_effects,
            "application.effects",
        )?;
        for node in &row.program.nodes {
            if let RuleExpression::All { values } | RuleExpression::Any { values } =
                &node.expression
            {
                check(
                    values.len() <= l.max_edges,
                    "application.nodes",
                    "list edge limit exceeded",
                )?;
            }
        }
        b.work(
            1 + row.targets.len()
                + row.stacking.len()
                + row.program.reads.len()
                + row.program.nodes.len()
                + row.program.effects.len(),
            l,
            "application",
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn read<I: DefinitionSchemaIndex>(
    r: &RuleRead,
    p: &RuleProgram,
    declarations: (&SchemaSubject, &Ports<'_>),
    index: &I,
    path: &str,
    l: RuleLimits,
    b: &mut Budget,
    application: Option<&EffectApplicationSource>,
) -> Result<CompiledRead, RuleError> {
    let Some(source) = application else {
        return super::read(r, p, declarations, index, path, l, b);
    };
    let mapped = match &r.source {
        RuleReadSource::EffectSourceParameter { slot } => {
            Some(RuleReadSource::Parameter { slot: slot.clone() })
        }
        RuleReadSource::EffectSourceChoice { slot } => {
            Some(RuleReadSource::Choice { slot: slot.clone() })
        }
        RuleReadSource::Stat {
            entity: RuleEntity::EffectSource,
            stat,
        } => Some(RuleReadSource::Stat {
            entity: RuleEntity::Current,
            stat: stat.clone(),
        }),
        RuleReadSource::Capability {
            entity: RuleEntity::EffectSource,
            capability,
        } => Some(RuleReadSource::Capability {
            entity: RuleEntity::Current,
            capability: capability.clone(),
        }),
        RuleReadSource::External {
            entity: RuleEntity::EffectSource,
            input,
        } => Some(RuleReadSource::External {
            entity: RuleEntity::Current,
            input: input.clone(),
        }),
        RuleReadSource::Contributions {
            entity: RuleEntity::EffectSource,
            stat,
            contribution,
            reduction,
            empty,
        } => Some(RuleReadSource::Contributions {
            entity: RuleEntity::Current,
            stat: stat.clone(),
            contribution: *contribution,
            reduction: *reduction,
            empty: empty.clone(),
        }),
        RuleReadSource::Parameter { .. }
        | RuleReadSource::Choice { .. }
        | RuleReadSource::GemLevel
        | RuleReadSource::ItemLevel
        | RuleReadSource::ItemQualityAmount { .. }
        | RuleReadSource::GemQualityAmount { .. }
        | RuleReadSource::HasItemQuality { .. }
        | RuleReadSource::HasGemQuality { .. }
        | RuleReadSource::ModifierTransforms { .. } => {
            return Err(fail(
                path,
                "application has no unqualified raw input authority",
            ));
        }
        _ => None,
    };
    let Some(mapped) = mapped else {
        return super::read(r, p, declarations, index, path, l, b);
    };
    let source_context = match source {
        EffectApplicationSource::Skill { .. } => RuleEntityKind::Skill,
        EffectApplicationSource::OwnedSlot { .. } => {
            check(
                !matches!(
                    mapped,
                    RuleReadSource::Parameter { .. } | RuleReadSource::Choice { .. }
                ),
                path,
                "actor application sources have no raw parent input authority",
            )?;
            RuleEntityKind::Actor
        }
    };
    let owner = source_owner(source);
    let ports = owner_ports(&owner, index, path)?;
    let source_program = RuleProgram {
        id: p.id.clone(),
        context: source_context,
        reads: vec![],
        nodes: vec![],
        effects: vec![],
    };
    let source_read = RuleRead {
        id: r.id.clone(),
        value_type: r.value_type.clone(),
        source: mapped,
    };
    super::read(
        &source_read,
        &source_program,
        (&owner, &ports),
        index,
        path,
        l,
        b,
    )
}

pub(super) fn compile<I: DefinitionSchemaIndex>(
    input: &mut RulePackageInput,
    tables: &BTreeMap<OwnedDefinitionKey, Arc<IntegerRuleTable>>,
    index: &I,
    l: RuleLimits,
    b: &mut Budget,
) -> Result<BTreeMap<OwnedDefinitionKey, Arc<CompiledProgram>>, RuleError> {
    let mut compiled = BTreeMap::new();
    let Some(rows) = &mut input.effect_applications else {
        return Ok(compiled);
    };
    rows.members.sort_by(|a, b| a.id.cmp(&b.id));
    let mut groups = BTreeMap::new();
    for row in &mut rows.members {
        let path = format!("effect_applications.{}", row.id);
        check(
            !compiled.contains_key(&row.id),
            &path,
            "duplicate application ID",
        )?;
        check(
            !row.targets.is_empty() && !row.program.effects.is_empty(),
            &path,
            "application needs targets and contributions",
        )?;
        let owner = source_owner(&row.source);
        let ports = owner_ports(&owner, index, &path)?;
        let mut target_keys = BTreeSet::new();
        for target in &row.targets {
            check(
                target_keys.insert(target),
                &path,
                "duplicate application target",
            )?;
            let context = match target {
                EffectApplicationTarget::Player => RuleEntityKind::Actor,
                EffectApplicationTarget::Enemy => RuleEntityKind::Enemy,
                EffectApplicationTarget::OwnedSlot { slot } => {
                    known(index.slot(slot), &path)?;
                    RuleEntityKind::Actor
                }
            };
            check(
                row.program.context == context,
                &path,
                "application context differs from recipient kind",
            )?;
        }
        row.targets.sort();
        row.program.reads.sort_by(|a, b| a.id.cmp(&b.id));
        row.program.nodes.sort_by(|a, b| a.id.cmp(&b.id));
        let mut effect_keys = BTreeSet::new();
        let mut group_keys = BTreeSet::new();
        for mapping in &row.stacking {
            check(
                effect_keys.insert(&mapping.effect),
                &path,
                "duplicate stacking effect",
            )?;
            check(
                group_keys.insert((&mapping.family, &mapping.modifier)),
                &path,
                "duplicate application modifier group",
            )?;
            check(
                row.program
                    .effects
                    .iter()
                    .any(|effect| effect.id == mapping.effect),
                &path,
                "stacking references absent effect",
            )?;
            check(
                mapping.reduction == EffectStackingReduction::Maximum,
                &path,
                "unsupported stacking reduction",
            )?;
        }
        check(
            row.stacking.len() == row.program.effects.len(),
            &path,
            "each contribution needs exact stacking membership",
        )?;
        let mut program = row.program.clone();
        for (position, effect) in program.effects.iter_mut().enumerate() {
            check(
                matches!(
                    effect.effect,
                    RuleEffectKind::Contribute {
                        entity: RuleEntity::Current,
                        ..
                    }
                ),
                &path,
                "application can only contribute to its exact recipient",
            )?;
            effect.when = Some(if let Some(existing) = &effect.when {
                let mut suffix = position;
                let guard = loop {
                    let candidate =
                        OwnedDefinitionKey::new(format!("application-activation-{suffix}"))
                            .map_err(|e| fail(&path, &e.to_string()))?;
                    b.work(1 + program.nodes.len(), l, &path)?;
                    if !program.nodes.iter().any(|node| node.id == candidate) {
                        break candidate;
                    }
                    suffix = suffix
                        .checked_add(1)
                        .ok_or_else(|| fail(&path, "guard ID overflow"))?;
                };
                add(&mut b.nodes, 1, l.max_nodes, &path)?;
                program.nodes.push(RuleNode {
                    id: guard.clone(),
                    expression: RuleExpression::All {
                        values: vec![row.activation.clone(), existing.clone()],
                    },
                });
                guard
            } else {
                row.activation.clone()
            });
        }
        let owner_rules = DefinitionRules {
            owner,
            programs: DeclaredSet::complete(vec![]),
        };
        let prepared = super::program(
            &program,
            &owner_rules,
            (&ports, tables),
            index,
            l,
            b,
            &path,
            Some(&row.source),
        )?;
        for mapping in &row.stacking {
            let effect = prepared
                .effects
                .iter()
                .find(|effect| effect.id == mapping.effect)
                .expect("checked stacking effect");
            let RuleEffectKind::Contribute {
                stat, contribution, ..
            } = &effect.kind
            else {
                unreachable!()
            };
            let ty = prepared.nodes[effect.value].ty.clone();
            check(
                numeric(&ty),
                &path,
                "maximum requires a numeric contribution",
            )?;
            let signature = (stat.clone(), *contribution, ty);
            let key = (mapping.family.clone(), mapping.modifier.clone());
            if let Some(previous) = groups.insert(key, signature.clone()) {
                check(
                    previous == signature,
                    &path,
                    "modifier group changes channel, kind, type or unit",
                )?;
            }
        }
        compiled.insert(row.id.clone(), Arc::new(prepared));
    }
    Ok(compiled)
}
