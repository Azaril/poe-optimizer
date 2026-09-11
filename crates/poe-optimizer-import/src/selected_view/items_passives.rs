use super::*;
use crate::item_source::{ItemSourceKind, ItemSourceUse};
use poe_optimizer_engine::selection_keys::NumericSetKeys;

fn default_record(
    ctx: &mut Context<'_>,
    domain: SelectionDomain,
    container: Option<SourceOccurrenceId>,
) -> Result<SetRecord> {
    ctx.record()?;
    Ok(SetRecord {
        key: NumericValue::new(1.0),
        origin: SetOrigin::Default { domain, container },
        members: vec![],
    })
}
/// Set selection is independent of the deferred ParseRaw/BuildModList producer.
/// No inventory item is admitted or merged just because its raw ID matches.
pub(super) fn items(ctx: &mut Context<'_>, node: Option<Node<'_, '_>>) -> Result<DomainSelection> {
    let container = node.map(|n| ctx.source(n)).transpose()?;
    let selector = ctx.selector(node, "activeItemSet")?;
    let mut result = DomainSelection::new(SelectionDomain::Items, container, selector);
    let mut legacy_slots = Vec::new();
    let mut keys = NumericSetKeys::new(ctx.records_left);
    let mut next_free = 1.0;
    if let Some(node) = node {
        for child in ctx.children(node)?.into_iter().flatten() {
            if !ctx.known(child) {
                result.fail(
                    "namespace_context",
                    Some(ctx.source(child)?),
                    "namespaced item content requires explicit preparation",
                );
                return Ok(result);
            }
            match child.tag_name().name() {
                "Slot" => {
                    ctx.record()?;
                    legacy_slots.push(ctx.instance(child)?);
                }
                "ItemSet" => {
                    let key = match ctx.key(child)? {
                        Some(k) => k,
                        None => {
                            while keys.contains(next_free) {
                                next_free += 1.0;
                            }
                            next_free
                        }
                    };
                    if key.is_nan() {
                        result.fail(
                            "nan_set_key",
                            Some(ctx.source(child)?),
                            "ItemsTab.CreateItemSet rejects a NaN table key",
                        );
                        return Ok(result);
                    }
                    ctx.record()?;
                    keys.insert(key)
                        .map_err(|_| ResolveError::Resource("numeric item set keys"))?;
                    let mut record = SetRecord {
                        key: NumericValue::new(key),
                        origin: ctx.origin(child)?,
                        members: vec![],
                    };
                    for entry in ctx.children(child)?.into_iter().flatten() {
                        if !ctx.known(entry) {
                            result.push_record(record);
                            result.fail(
                                "namespace_context",
                                Some(ctx.source(entry)?),
                                "namespaced item-set content requires explicit preparation",
                            );
                            return Ok(result);
                        }
                        match entry.tag_name().name() {
                            "Slot" | "RuneSlot" => {
                                ctx.record()?;
                                record.members.push(ctx.instance(entry)?);
                            }
                            "SocketIdURL" => {
                                let id = ctx
                                    .attribute(entry, "nodeId")?
                                    .and_then(|s| parse_number(s.as_bytes()));
                                if id.is_none_or(f64::is_nan) {
                                    result.push_record(record);
                                    result.fail("invalid_socket_url_key",Some(ctx.source(entry)?),"ItemsTab.Load indexes itemSet with a nil or NaN SocketIdURL nodeId");
                                    return Ok(result);
                                }
                            }
                            _ => {}
                        }
                    }
                    result.order.push(Some(record.key));
                    result.push_record(record);
                }
                _ => {}
            }
        }
    }
    if result.order.is_empty() {
        let mut record = default_record(ctx, SelectionDomain::Items, container)?;
        record.members = legacy_slots;
        result.order.push(Some(record.key));
        result.push_record(record);
    }
    result.choose_key();
    if node.is_none() {
        result.rule = Some(SelectionRule::ConstructorDefault);
    }
    Ok(result)
}

/// TreeTab uses source positions, ignores authored Spec IDs and only clamps above.
/// PassiveSpec.Load, version registries, allocations and switching side effects
/// remain explicit preparation prerequisites in the outer view report.
pub(super) fn passives(
    ctx: &mut Context<'_>,
    node: Option<Node<'_, '_>>,
) -> Result<DomainSelection> {
    let container = node.map(|n| ctx.source(n)).transpose()?;
    let selector = ctx.selector(node, "activeSpec")?;
    let mut result = DomainSelection::new(SelectionDomain::Passives, container, selector);
    let legacy = node.is_some_and(|n| n.tag_name().name() == "Spec");
    let mut specs = Vec::new();
    if let Some(node) = node {
        if legacy {
            specs.push(node);
        } else {
            for child in ctx.children(node)?.into_iter().flatten() {
                if !ctx.known(child) {
                    result.fail(
                        "namespace_context",
                        Some(ctx.source(child)?),
                        "namespaced passive content requires explicit preparation",
                    );
                    return Ok(result);
                }
                if child.tag_name().name() == "Spec" {
                    specs.push(child);
                }
            }
        }
    }
    for (index, spec) in specs.into_iter().enumerate() {
        ctx.record()?;
        let mut record = SetRecord {
            key: NumericValue::new((index + 1) as f64),
            origin: ctx.origin(spec)?,
            members: vec![],
        };
        // Jewel uses remain owned by this spec; no item identity or allocation is inferred.
        for descendant in spec.descendants().filter(Node::is_element) {
            let occurrence = ctx.build.occurrence(ctx.source(descendant)?)?;
            if !ctx.known(descendant) {
                result.push_record(record);
                result.fail(
                    "namespace_context",
                    Some(ctx.source(descendant)?),
                    "namespaced passive-spec content requires explicit preparation",
                );
                return Ok(result);
            }
            if matches!(
                occurrence.role(),
                crate::build_instance::SourceRole::Item {
                    kind: ItemSourceKind::Socket,
                    usage: ItemSourceUse::JewelAssignment
                }
            ) {
                ctx.record()?;
                record.members.push(ctx.instance(descendant)?);
            }
        }
        result.order.push(Some(record.key));
        result.push_record(record);
    }
    if result.sets.is_empty() {
        let record = default_record(ctx, SelectionDomain::Passives, container)?;
        result.order.push(Some(record.key));
        result.push_record(record);
    }
    let requested = if legacy {
        1.0
    } else {
        result.authored.requested.value()
    };
    // LuaJIT math.min(request, count) chooses count for NaN in the observed
    // two-argument source operation; Rust f64::min has the same operand behavior.
    let selected = requested.min(result.sets.len() as f64);
    result.selected = result.lookup(selected).cloned();
    if result.selected.is_none() {
        result.fail("invalid_spec_position",container,"TreeTab.SetActiveSpec dereferences a missing positional spec after its upper-only clamp");
    } else {
        result.rule = Some(if legacy {
            SelectionRule::LegacyPosition
        } else if node.is_none() {
            SelectionRule::ConstructorDefault
        } else if selected != requested {
            SelectionRule::UpperClampedPosition
        } else {
            SelectionRule::ExactPosition
        });
    }
    Ok(result)
}
