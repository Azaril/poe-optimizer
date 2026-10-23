//! Explicit empty passive-socket membership under the fresh TreeTab load grammar.
//! This source proof does not establish passive legality or item mechanics.
use super::source_shape::{charge_row, container_text, plain_row, retire_membership, value};
use super::*;
use crate::source_xml::PobContentEntry;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
// One cold import policy per package, never a per-candidate runtime value.
// Keep the versioned authoring fields together without separately boxing pins.
#[allow(clippy::large_enum_variant)]
pub enum PassiveSocketMembershipPolicy {
    /// One canonical Tree constructs fresh Specs; a single explicitly empty
    /// Sockets container proves only that Spec's authored equipment inventory.
    PobExplicitEmptySpecSocketsV1 {},
    /// Reviewed ordinary passive destinations joined to the allocation in their
    /// own fresh Spec. Requires independently proved Ordinary access, Known
    /// Shared scope, and resolved character class/ascendancy for the fresh load.
    PobOrdinarySharedSpecSocketsV2 {
        definitions: DataIdentity,
        mapping: OwnedContentDigest,
        mapping_source: OwnedContentDigest,
        item_lines: OwnedContentDigest,
        item_source: OwnedContentDigest,
        equipment: OwnedContentDigest,
        tree_content: OwnedContentDigest,
        source_bases: Vec<OrdinaryPassiveJewelBase>,
        bindings: Vec<OrdinaryPassiveSocketBinding>,
    },
}

/// Authored passive-socket grammar and source evidence, without current package
/// commitments. Tree content is bound separately to avoid a normalization cycle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PassiveSocketMembershipContent {
    PobExplicitEmptySpecSocketsV1 {},
    PobOrdinarySharedSpecSocketsV2 {
        mapping_source: OwnedContentDigest,
        source_bases: Vec<OrdinaryPassiveJewelBase>,
        bindings: Vec<OrdinaryPassiveSocketBinding>,
    },
}

impl PassiveSocketMembershipContent {
    /// Only the checked aggregate owner may discard dependency commitments.
    pub(super) fn from_checked(input: PassiveSocketMembershipPolicy) -> Self {
        match input {
            PassiveSocketMembershipPolicy::PobExplicitEmptySpecSocketsV1 {} => {
                Self::PobExplicitEmptySpecSocketsV1 {}
            }
            PassiveSocketMembershipPolicy::PobOrdinarySharedSpecSocketsV2 {
                definitions: _,
                mapping: _,
                mapping_source,
                item_lines: _,
                item_source: _,
                equipment: _,
                tree_content: _,
                source_bases,
                bindings,
            } => Self::PobOrdinarySharedSpecSocketsV2 {
                mapping_source,
                source_bases,
                bindings,
            },
        }
    }

    /// The aggregate supplies identities of its constructed equipment policy
    /// and supplied Tree content, never the Tree package's back-reference.
    /// This raw result still requires the existing final semantic validators.
    pub(super) fn into_input<I: DefinitionSchemaIndex>(
        self,
        definitions: &I,
        mappings: &OwnedMappingIndex,
        items: &OwnedItemLinePolicy,
        item_source: &ItemSourceLayoutPolicy,
        equipment: Option<OwnedContentDigest>,
        tree_content: Option<OwnedContentDigest>,
    ) -> Result<PassiveSocketMembershipPolicy> {
        Ok(match self {
            Self::PobExplicitEmptySpecSocketsV1 {} => {
                PassiveSocketMembershipPolicy::PobExplicitEmptySpecSocketsV1 {}
            }
            Self::PobOrdinarySharedSpecSocketsV2 {
                mapping_source,
                source_bases,
                bindings,
            } => PassiveSocketMembershipPolicy::PobOrdinarySharedSpecSocketsV2 {
                definitions: definitions.identity().clone(),
                mapping: *mappings.identity(),
                mapping_source,
                item_lines: *items.identity(),
                item_source: *item_source.identity(),
                equipment: equipment.ok_or(NormalizationError::Policy(
                    "passive sockets require equipment membership",
                ))?,
                tree_content: tree_content.ok_or(NormalizationError::Policy(
                    "passive sockets require tree content",
                ))?,
                source_bases,
                bindings,
            },
        })
    }
}

pub(super) const SPEC_ATTRIBUTES: &[&str] = &[
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

pub(super) fn canonical_tree(b: &mut Builder<'_, '_>) -> Result<Option<SourceOccurrenceId>> {
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
    _policy: &PassiveSocketMembershipPolicy,
    specs: &BTreeMap<SourceOccurrenceId, usize>,
) -> Result<()> {
    // V2 retains the original explicit-empty proof at the original checkpoint;
    // its additional occupied-placement pass runs only after tree conversion.
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
