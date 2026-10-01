//! Explicit empty passive-socket membership under the fresh TreeTab load grammar.
//! This source proof does not establish passive legality or item mechanics.
use super::source_shape::{charge_row, container_text, plain_row, retire_membership, value};
use super::*;
use crate::source_xml::PobContentEntry;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PassiveSocketMembershipPolicy {
    /// One canonical Tree constructs fresh Specs; a single explicitly empty
    /// Sockets container proves only that Spec's authored equipment inventory.
    PobExplicitEmptySpecSocketsV1 {},
}

const SPEC_ATTRIBUTES: &[&str] = &[
    "title",
    "treeVersion",
    "classId",
    "classInternalId",
    "ascendClassId",
    "ascendancyInternalId",
    "secondaryAscendClassId",
    "nodes",
    "masteryEffects",
];

fn canonical_tree(b: &mut Builder<'_, '_>) -> Result<Option<SourceOccurrenceId>> {
    let evidence = b.evidence;
    let [source] = evidence.sections(SourceSectionKind::Tree) else {
        return Ok(None);
    };
    let root = &evidence.rows()[0];
    charge_row(b, root)?;
    let row = &evidence.rows()[source.ordinal() as usize];
    charge_row(b, row)?;
    // The section index omits namespaced contexts. Count raw local names too,
    // including the legacy root Spec loader, before accepting a single load.
    let tree_count = root
        .children()
        .iter()
        .filter(|child| {
            matches!(
                evidence.rows()[child.ordinal() as usize]
                    .occurrence()
                    .name(),
                "Tree" | "Spec"
            )
        })
        .count();
    if tree_count != 1
        || root.occurrence().name() != "PathOfBuilding2"
        || !plain_row(root, &[], false)
        || !container_text(root)
        || row.occurrence().name() != "Tree"
        || row.occurrence().parent() != Some(root.occurrence().id())
        || !plain_row(row, &["activeSpec"], false)
        || !container_text(row)
    {
        return Ok(None);
    }
    // Active selection is deliberately not inferred or changed. Each direct
    // Spec has its own source occurrence and independent inventory proof.
    for child in row.children() {
        let child = &evidence.rows()[child.ordinal() as usize];
        b.charge(1)?;
        if child.occurrence().name() != "Spec" || child.occurrence().has_namespace_context() {
            return Ok(None);
        }
    }
    Ok(Some(*source))
}

fn explicit_empty(b: &mut Builder<'_, '_>, row: &SourceEvidenceRow<'_>) -> Result<bool> {
    charge_row(b, row)?;
    if !plain_row(row, SPEC_ATTRIBUTES, false) || !container_text(row) {
        return Ok(false);
    }
    let mut sockets = false;
    let mut url = false;
    let mut overrides = false;
    for child in row.children() {
        let child = &b.evidence.rows()[child.ordinal() as usize];
        charge_row(b, child)?;
        match child.occurrence().name() {
            "Sockets" => {
                if sockets || !plain_row(child, &[], true) {
                    return Ok(false);
                }
                sockets = true;
            }
            "URL" => {
                if url || !plain_row(child, &[], false) || !child.children().is_empty() {
                    return Ok(false);
                }
                let SourceContentEvidence::Available(content) = child.content() else {
                    return Ok(false);
                };
                if !matches!(content.consumed(), [PobContentEntry::Text {text, ..}] if !text.trim_ascii().is_empty())
                {
                    return Ok(false);
                }
                url = true;
            }
            "Overrides" => {
                if overrides || !plain_row(child, &[], false) || !container_text(child) {
                    return Ok(false);
                }
                // This first grammar supports the single saved attribute
                // override record. It changes allocated attributes, not jewels.
                if child.children().len() > 1 {
                    return Ok(false);
                }
                for override_id in child.children() {
                    let entry = &b.evidence.rows()[override_id.ordinal() as usize];
                    charge_row(b, entry)?;
                    if entry.occurrence().name() != "AttributeOverride"
                        || !plain_row(entry, &["strNodes", "dexNodes", "intNodes"], true)
                        || ["strNodes", "dexNodes", "intNodes"]
                            .iter()
                            .any(|name| value(entry, name).is_none())
                    {
                        return Ok(false);
                    }
                }
                overrides = true;
            }
            // Weapon-set, note, legacy and future shapes remain explicit work
            // for later source adapters. No ignored child grants empty authority.
            _ => return Ok(false),
        }
    }
    Ok(sockets)
}

pub(super) fn close(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
    policy: &PassiveSocketMembershipPolicy,
    specs: &BTreeMap<SourceOccurrenceId, usize>,
) -> Result<()> {
    let PassiveSocketMembershipPolicy::PobExplicitEmptySpecSocketsV1 {} = policy;
    let Some(tree) = canonical_tree(b)? else {
        return Ok(());
    };
    for (source, index) in specs {
        b.charge(1)?;
        let row = &b.evidence.rows()[source.ordinal() as usize];
        if row.occurrence().parent() != Some(tree)
            || !draft.allocation_presets.members[*index]
                .equipment
                .members
                .is_empty()
            || !explicit_empty(b, row)?
        {
            continue;
        }
        retire_membership(
            b,
            *source,
            &mut draft.allocation_presets.members[*index]
                .equipment
                .completion,
            "allocation-equipment-membership-not-converted",
        )?;
    }
    Ok(())
}
