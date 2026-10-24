//! Source-bound presentation dispositions. No owned inventory or issue is closed.
use super::source_shape::{charge_row, container_text, plain_row, value};
use super::*;

mod presentation_frames;
mod timeless_search;
mod trade_search;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourcePresentationPolicy {
    /// Each opt-in family admits only its reviewed source frame. Unrecognized
    /// source remains under the historical fallback obligations.
    PobFreshPresentationV1 {
        mapping_source: OwnedContentDigest,
        empty_socket_urls: bool,
        calcs_sections: bool,
        tree_view: bool,
        empty_notes: bool,
        /// Omission authorizes no cached-output disposition. Serializing false
        /// preserves inherited policies and their existing content identities.
        #[serde(default, skip_serializing_if = "is_false")]
        cached_build_buffs: bool,
        /// A childless Party frame stores future import and display preferences.
        #[serde(default, skip_serializing_if = "is_false")]
        empty_party: bool,
        /// Saved trade-tool preferences retain source provenance without
        /// becoming gameplay inputs or native optimization objectives.
        #[serde(default, skip_serializing_if = "is_false")]
        trade_search_weights: bool,
        /// Remembered import UI preferences with party export disabled.
        /// Absence authorizes no disposition for this independent family.
        #[serde(default, skip_serializing_if = "is_false")]
        import_preferences: bool,
        /// Constructor-equivalent preferences for a future jewel search, not
        /// an equipped jewel, calculation input or optimization objective.
        #[serde(default, skip_serializing_if = "is_false")]
        timeless_search_defaults: bool,
    },
}

fn is_false(value: &bool) -> bool {
    !value
}

pub(super) struct CompiledSourcePresentation {
    empty_socket_urls: bool,
    calcs_sections: bool,
    tree_view: bool,
    empty_notes: bool,
    cached_build_buffs: bool,
    empty_party: bool,
    trade_search_weights: bool,
    import_preferences: bool,
    timeless_search_defaults: bool,
}

pub(super) fn compile(
    policy: &NormalizationPolicy,
    mappings: &OwnedMappingIndex,
    limits: NormalizationLimits,
) -> Result<Option<CompiledSourcePresentation>> {
    let Some(SourcePresentationPolicy::PobFreshPresentationV1 {
        mapping_source,
        empty_socket_urls,
        calcs_sections,
        tree_view,
        empty_notes,
        cached_build_buffs,
        empty_party,
        trade_search_weights,
        import_preferences,
        timeless_search_defaults,
    }) = &policy.source_presentation
    else {
        return Ok(None);
    };
    // An all-disabled policy must still authenticate its source commitment.
    if mapping_source != mappings.source_identity()
        || mappings.input().source.system != ExternalSourceSystem::PathOfBuilding2
    {
        return Err(NormalizationError::Binding);
    }
    if limits.max_work == 0 {
        return Err(NormalizationError::Limit("source presentation policy work"));
    }
    Ok(Some(CompiledSourcePresentation {
        empty_socket_urls: *empty_socket_urls,
        calcs_sections: *calcs_sections,
        tree_view: *tree_view,
        empty_notes: *empty_notes,
        cached_build_buffs: *cached_build_buffs,
        empty_party: *empty_party,
        trade_search_weights: *trade_search_weights,
        import_preferences: *import_preferences,
        timeless_search_defaults: *timeless_search_defaults,
    }))
}

fn numeric_key(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with('0')
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && text.parse::<u32>().is_ok_and(|value| value > 0)
}

fn empty_socket_urls(
    b: &mut Builder<'_, '_>,
) -> Result<Vec<(SourceOccurrenceId, OwnedDefinitionKey)>> {
    let Some(items) = equipment_membership::ordinary_items(b)? else {
        return Ok(vec![]);
    };
    let evidence = b.evidence;
    let parent = &evidence.rows()[items.ordinal() as usize];
    let mut output = Vec::new();
    for source in parent.children() {
        b.charge(1)?;
        let set = &evidence.rows()[source.ordinal() as usize];
        if set.occurrence().name() != "ItemSet" {
            continue;
        }
        charge_row(b, set)?;
        if set.occurrence().parent() != Some(items)
            || !plain_row(set, &["id", "title", "useSecondWeaponSet"], false)
            || !container_text(set)
            || !value(set, "id").is_some_and(numeric_key)
            || value(set, "useSecondWeaponSet")
                .is_some_and(|value| !equipment_membership::boolean_token(value))
        {
            continue;
        }
        // The shared Items census already proves canonical unique ItemSet IDs.
        // Inspect every URL sibling before admitting any: aliases or duplicates
        // cannot leave an earlier empty write looking independently meaningful.
        let mut keys = BTreeSet::new();
        let mut candidates = Vec::new();
        let mut proven = true;
        for child in set.children() {
            b.charge(1)?;
            let row = &evidence.rows()[child.ordinal() as usize];
            if row.occurrence().name() != "SocketIdURL" {
                continue;
            }
            charge_row(b, row)?;
            if row.occurrence().parent() != Some(*source)
                || !plain_row(row, &["nodeId", "itemPbURL", "name"], true)
                || !value(row, "nodeId").is_some_and(|id| numeric_key(id) && keys.insert(id))
                || value(row, "itemPbURL") != Some("")
            {
                proven = false;
                break;
            }
            if candidates.len() >= b.limits.draft.input.max_collection_entries {
                return Err(NormalizationError::Limit("source presentation URL rows"));
            }
            candidates.push(*child);
        }
        if proven {
            b.charge(candidates.len())?;
            for source in candidates {
                if output.len() >= b.limits.draft.input.max_collection_entries {
                    return Err(NormalizationError::Limit("source presentation URL rows"));
                }
                output.push((source, key("empty-socket-trade-url")));
            }
        }
    }
    Ok(output)
}

/// Run after semantic adapters and before the historical fallback pass. A
/// presentation proof never steals an existing semantic link or removes an
/// obligation. Stage every candidate and charge before changing dispositions.
pub(super) fn apply(
    b: &mut Builder<'_, '_>,
    policy: Option<&CompiledSourcePresentation>,
    build_frame: Option<&character_reward_inventory::CompiledCharacterRewardInventory<'_>>,
) -> Result<()> {
    let Some(policy) = policy else { return Ok(()) };
    b.charge(1)?;
    let mut candidates = if policy.empty_socket_urls {
        empty_socket_urls(b)?
    } else {
        vec![]
    };
    if policy.trade_search_weights {
        candidates.extend(trade_search::collect(b)?);
    }
    if policy.timeless_search_defaults {
        candidates.extend(timeless_search::collect(b, build_frame)?);
    }
    candidates.extend(presentation_frames::collect(
        b,
        policy.calcs_sections,
        policy.tree_view,
        policy.empty_notes,
        policy.cached_build_buffs,
        policy.empty_party,
        policy.import_preferences,
    )?);
    if candidates.len() > b.limits.draft.input.max_collection_entries {
        return Err(NormalizationError::Limit("source presentation rows"));
    }
    b.charge(candidates.len())?;
    for (source, disposition) in candidates {
        let origin = &mut b.origins[source.ordinal() as usize];
        if origin.links.is_empty() && matches!(origin.disposition, SourceDisposition::Contributes) {
            origin.disposition = SourceDisposition::SourceOnly(disposition);
        }
    }
    Ok(())
}
