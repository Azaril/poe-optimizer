//! Authored skill/configuration set selection in original loader order.
//!
//! Groups here retain authored instance identities. Selection does not execute
//! ProcessSocketGroup, configuration defaults or configuration modifier effects.
use super::*;
use poe_optimizer_engine::selection_keys::{NumericSetKeyError, NumericSetKeys};

fn create(
    ctx: &mut Context<'_>,
    selection: &mut DomainSelection,
    keys: &mut NumericSetKeys,
    key: Option<f64>,
    origin: SetOrigin,
) -> Result<Option<f64>> {
    let key = key.unwrap_or_else(|| keys.sequence_length() as f64 + 1.0);
    ctx.record()?;
    match keys.insert(key) {
        Ok(_) => {
            selection.push_record(SetRecord {
                key: NumericValue::new(key),
                origin,
                members: vec![],
            });
            Ok(Some(key))
        }
        Err(NumericSetKeyError::NanKey) => {
            selection.fail(
                "nan_set_key",
                origin.source(),
                "original set creation rejects a NaN table key",
            );
            Ok(None)
        }
        Err(NumericSetKeyError::KeyLimit { .. } | NumericSetKeyError::TableOverflow) => {
            Err(ResolveError::Resource("numeric set keys"))
        }
        Err(NumericSetKeyError::NotNumber) => Err(ResolveError::Invariant(
            "numeric key insertion returned text error",
        )),
    }
}
fn default_origin(selection: &DomainSelection) -> SetOrigin {
    SetOrigin::Default {
        domain: selection.domain,
        container: selection.container,
    }
}
fn namespace_problem(
    ctx: &Context<'_>,
    selection: &mut DomainSelection,
    node: Node<'_, '_>,
) -> Result<bool> {
    if ctx.known(node) {
        return Ok(false);
    }
    selection.fail(
        "namespace_context",
        Some(ctx.source(node)?),
        "namespaced skill/configuration content requires an explicit loader preparation rule",
    );
    Ok(true)
}

/// Check source-structural failures that precede ProcessSocketGroup. Resolving
/// definitions and executing that producer remain the caller's next frontier.
fn append_group(
    ctx: &mut Context<'_>,
    selection: &mut DomainSelection,
    group: Node<'_, '_>,
    key: f64,
) -> Result<bool> {
    if group.tag_name().name() != "Skill" {
        return Ok(true);
    }
    if namespace_problem(ctx, selection, group)? {
        return Ok(false);
    }
    for gem in ctx.children(group)? {
        let Some(gem) = gem else {
            selection.fail(
                "skill_child_text",
                Some(ctx.source(group)?),
                "LoadSkill reads gem attributes from a text child",
            );
            return Ok(false);
        };
        if namespace_problem(ctx, selection, gem)? {
            return Ok(false);
        }
        for child in ctx.children(gem)?.into_iter().flatten() {
            if namespace_problem(ctx, selection, child)? {
                return Ok(false);
            }
            if matches!(
                child.tag_name().name(),
                "MinionSkillIndexLookup" | "MinionSkillIndexLookupCalcs"
            ) && ctx.attribute(child, "grantedEffect")?.is_some()
            {
                for map in ctx.children(child)? {
                    let Some(map) = map else {
                        selection.fail(
                            "minion_map_text",
                            Some(ctx.source(child)?),
                            "LoadSkill reads minion-map attributes from a text child",
                        );
                        return Ok(false);
                    };
                    if namespace_problem(ctx, selection, map)? {
                        return Ok(false);
                    }
                    let raw = ctx.attribute(map, "skillIndex")?;
                    let index = raw.as_deref().and_then(|raw| parse_number(raw.as_bytes()));
                    if index.is_none_or(f64::is_nan) {
                        selection.fail(if index.is_some() { "nan_minion_map_key" } else { "nil_minion_map_key" }, Some(ctx.source(map)?), "LoadSkill cannot assign a minion-map entry at a nil or NaN numeric key");
                        return Ok(false);
                    }
                }
            }
        }
    }
    let member = match ctx.instance(group) {
        Ok(AuthoredInstanceId::SkillGroup(id)) => AuthoredInstanceId::SkillGroup(id),
        _ => {
            selection.fail(
                "unmapped_skill_group",
                Some(ctx.source(group)?),
                "consumed source group has no authored group identity",
            );
            return Ok(false);
        }
    };
    let Some(record) = selection.lookup_mut(key) else {
        selection.fail(
            "missing_legacy_skill_set",
            Some(ctx.source(group)?),
            "LoadSkill appends a legacy group to missing set 1",
        );
        return Ok(false);
    };
    ctx.record()?;
    record.members.push(member);
    Ok(true)
}

pub(super) fn skills(ctx: &mut Context<'_>, node: Option<Node<'_, '_>>) -> Result<DomainSelection> {
    let container = node.map(|node| ctx.source(node)).transpose()?;
    let mut selection = DomainSelection::new(
        SelectionDomain::Skills,
        container,
        ctx.selector(node, "activeSkillSet")?,
    );
    let mut keys = NumericSetKeys::new(ctx.records_left);
    if let Some(node) = node {
        for child in ctx.children(node)?.into_iter().flatten() {
            if namespace_problem(ctx, &mut selection, child)? {
                return Ok(selection);
            }
            match child.tag_name().name() {
                "Skill" => {
                    if selection.order.first().and_then(|key| *key).is_none() {
                        selection.order.push(Some(NumericValue::new(1.0)));
                        let origin = default_origin(&selection);
                        create(ctx, &mut selection, &mut keys, Some(1.0), origin)?;
                    }
                    if !append_group(ctx, &mut selection, child, 1.0)? {
                        return Ok(selection);
                    }
                }
                "SkillSet" => {
                    let origin = ctx.origin(child)?;
                    let source_key = ctx.key(child)?;
                    let Some(key) = create(ctx, &mut selection, &mut keys, source_key, origin)?
                    else {
                        return Ok(selection);
                    };
                    selection.order.push(Some(NumericValue::new(key)));
                    for group in ctx.children(child)?.into_iter().flatten() {
                        if !append_group(ctx, &mut selection, group, key)? {
                            return Ok(selection);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if selection.order.is_empty() {
        selection.order.push(Some(NumericValue::new(1.0)));
        let origin = default_origin(&selection);
        create(ctx, &mut selection, &mut keys, Some(1.0), origin)?;
    }
    selection.choose_key();
    if node.is_none() {
        selection.rule = Some(SelectionRule::ConstructorDefault);
    }
    Ok(selection)
}

pub(super) fn configuration(
    ctx: &mut Context<'_>,
    node: Option<Node<'_, '_>>,
) -> Result<DomainSelection> {
    let container = node.map(|node| ctx.source(node)).transpose()?;
    let mut selection = DomainSelection::new(
        SelectionDomain::Configuration,
        container,
        ctx.selector(node, "activeConfigSet")?,
    );
    let mut keys = NumericSetKeys::new(ctx.records_left);
    selection.order.push(Some(NumericValue::new(1.0)));
    let children = node
        .map(|node| ctx.children(node))
        .transpose()?
        .unwrap_or_default();
    // XML.lua marks both self-closing tags and paired tags with no consumed
    // array entries empty, including whitespace/comment-only containers.
    if children.is_empty() {
        let origin = default_origin(&selection);
        create(ctx, &mut selection, &mut keys, Some(1.0), origin)?;
    }
    for (index, child) in children.into_iter().enumerate() {
        if let Some(child) = child {
            if namespace_problem(ctx, &mut selection, child)? {
                return Ok(selection);
            }
            if child.tag_name().name() == "ConfigSet" {
                let source_key = ctx.key(child)?;
                let origin = ctx.origin(child)?;
                if create(ctx, &mut selection, &mut keys, source_key, origin)?.is_none() {
                    return Ok(selection);
                }
                selection
                    .order
                    .resize(selection.order.len().max(index + 1), None);
                selection.order[index] = source_key.map(NumericValue::new);
                if source_key.is_none() {
                    selection.fail("nil_config_set_key", Some(ctx.source(child)?), "CreateConfigSet generates an ID, then Config.Load dereferences the original nil ID");
                    return Ok(selection);
                }
                // Input/Placeholder errors are diagnostic returns ignored by
                // Load. Their value/default/migration effects belong to the
                // configuration preparation stage, not set selection.
                for entry in ctx.children(child)?.into_iter().flatten() {
                    if namespace_problem(ctx, &mut selection, entry)? {
                        return Ok(selection);
                    }
                }
                continue;
            }
        }
        // Original Load treats text/unknown elements as legacy records too.
        // Their XML array position leaves a hole before a later ConfigSet.
        if !keys.contains(1.0) {
            let origin = default_origin(&selection);
            create(ctx, &mut selection, &mut keys, Some(1.0), origin)?;
        }
    }
    selection.choose_key();
    if node.is_none() {
        selection.rule = Some(SelectionRule::ConstructorDefault);
    }
    Ok(selection)
}
