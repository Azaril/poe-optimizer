//! Source-bound absence of character-owned reward selections, not Config rewards.
use super::source_shape::{container_text, plain_row, value};
use super::*;
use crate::source_xml::PobContentEntry;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CharacterRewardInventoryPolicy {
    PobFreshCharacterOnlyEmptyV1 {
        mapping_source: OwnedContentDigest,
        target_version: String,
        tree_version: String,
    },
}

pub(super) struct CompiledCharacterRewardInventory<'p> {
    tree_version: &'p str,
    target_version: &'p str,
}

pub(super) fn compile<'p>(
    policy: &'p NormalizationPolicy,
    mappings: &OwnedMappingIndex,
    limits: NormalizationLimits,
) -> Result<Option<CompiledCharacterRewardInventory<'p>>> {
    let Some(CharacterRewardInventoryPolicy::PobFreshCharacterOnlyEmptyV1 {
        mapping_source,
        target_version,
        tree_version,
    }) = &policy.character_reward_inventory
    else {
        return Ok(None);
    };
    if mapping_source != mappings.source_identity() {
        return Err(NormalizationError::Binding);
    }
    let bytes = target_version.len().saturating_add(tree_version.len());
    if bytes > limits.max_work
        || bytes > limits.value.max_total_selector_bytes
        || [target_version, tree_version].iter().any(|version| {
            version.len() > 64
                || version.len() > limits.mapping.max_string_bytes
                || version.len() > limits.value.max_selector_bytes
        })
    {
        return Err(NormalizationError::Limit("character reward policy bytes"));
    }
    if [target_version, tree_version].iter().any(|version| {
        version.is_empty()
            || version.trim_ascii() != version.as_str()
            || !version
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    }) {
        return Err(NormalizationError::Policy("character reward tree version"));
    }
    Ok(Some(CompiledCharacterRewardInventory {
        tree_version,
        target_version,
    }))
}

/// Construction is private and occurs only after the whole loader frame passed.
pub(super) struct ProvenCharacterRewards {
    build: SourceOccurrenceId,
    spec: SourceOccurrenceId,
}

fn integer(text: Option<&str>, minimum: u64, maximum: u64) -> bool {
    text.is_some_and(|text| {
        !text.is_empty()
            && (text == "0" || !text.starts_with('0'))
            && text.bytes().all(|b| b.is_ascii_digit())
            && text
                .parse::<u64>()
                .is_ok_and(|n| (minimum..=maximum).contains(&n))
    })
}
const LUA_INTEGER_MAX: u64 = 9_007_199_254_740_991;

fn charge_row(b: &mut Builder<'_, '_>, row: &SourceEvidenceRow<'_>) -> Result<()> {
    source_shape::charge_row(b, row)?;
    if row.children().len() > b.limits.draft.input.max_collection_entries {
        return Err(NormalizationError::Limit(
            "character reward source children",
        ));
    }
    Ok(())
}

fn fresh_build(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    target_version: &str,
) -> Result<bool> {
    charge_row(b, row)?;
    const ATTRS: &[&str] = &[
        "targetVersion",
        "viewMode",
        "level",
        "className",
        "ascendClassName",
        "mainSocketGroup",
        "characterLevelAutoMode",
    ];
    if !plain_row(row, ATTRS, false)
        || !container_text(row)
        || ATTRS.iter().any(|name| value(row, name).is_none())
        || value(row, "targetVersion") != Some(target_version)
        || value(row, "characterLevelAutoMode") != Some("false")
        || !integer(value(row, "level"), 1, 100)
        || !integer(value(row, "mainSocketGroup"), 1, LUA_INTEGER_MAX)
    {
        return Ok(false);
    }
    let evidence = b.evidence;
    let mut singletons = BTreeSet::new();
    for id in row.children() {
        let child = &evidence.rows()[id.ordinal() as usize];
        charge_row(b, child)?;
        let name = child.occurrence().name();
        let accepted = match name {
            "PlayerStat" | "MinionStat" => {
                plain_row(child, &["stat", "value"], true)
                    && value(child, "stat").is_some_and(|s| !s.is_empty())
                    && value(child, "value").is_some()
            }
            "Buffs" => plain_row(child, &["buffList", "combatList", "curseList"], true),
            "BeastCompanion" => {
                plain_row(child, &["id"], true) && value(child, "id").is_some_and(|s| !s.is_empty())
            }
            "TimelessData" => {
                plain_row(
                    child,
                    &[
                        "devotionVariant1",
                        "devotionVariant2",
                        "searchList",
                        "searchListFallback",
                        "socketFilterDistance",
                    ],
                    true,
                ) && integer(value(child, "devotionVariant1"), 1, LUA_INTEGER_MAX)
                    && integer(value(child, "devotionVariant2"), 1, LUA_INTEGER_MAX)
                    && value(child, "searchList").is_some()
                    && value(child, "searchListFallback").is_some()
                    && (value(child, "socketFilterDistance").is_none()
                        || integer(value(child, "socketFilterDistance"), 0, LUA_INTEGER_MAX))
            }
            _ => false,
        };
        if !accepted || (!matches!(name, "PlayerStat" | "MinionStat") && !singletons.insert(name)) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn fresh_spec(b: &mut Builder<'_, '_>, row: &SourceEvidenceRow<'_>, version: &str) -> Result<bool> {
    charge_row(b, row)?;
    if row.occurrence().name() != "Spec"
        || !matches!(
            row.authored_instance(),
            Some(AuthoredInstanceId::PassiveSpec(_))
        )
        || !plain_row(
            row,
            &[
                "title",
                "treeVersion",
                "classId",
                "classInternalId",
                "ascendClassId",
                "ascendancyInternalId",
                "secondaryAscendClassId",
                "nodes",
                "masteryEffects",
            ],
            false,
        )
        || !container_text(row)
        || value(row, "treeVersion") != Some(version)
        || !integer(value(row, "classId"), 0, LUA_INTEGER_MAX)
        || !integer(value(row, "classInternalId"), 0, LUA_INTEGER_MAX)
        || !integer(value(row, "ascendClassId"), 0, LUA_INTEGER_MAX)
        || value(row, "ascendancyInternalId").is_none()
        || value(row, "nodes").is_none()
        || !matches!(value(row, "masteryEffects"), None | Some(""))
        || !matches!(
            value(row, "secondaryAscendClassId"),
            None | Some("nil" | "0")
        )
    {
        return Ok(false);
    }
    let evidence = b.evidence;
    let mut seen = BTreeSet::new();
    for id in row.children() {
        let child = &evidence.rows()[id.ordinal() as usize];
        charge_row(b, child)?;
        let name = child.occurrence().name();
        if !seen.insert(name) {
            return Ok(false);
        }
        match name {
            "URL" => {
                if !plain_row(child, &[], false)
                    || !child.children().is_empty()
                    || !matches!(child.content(), SourceContentEvidence::Available(content)
                        if content.consumed().iter().any(|entry|
                            matches!(entry, PobContentEntry::Text { text, .. } if !text.trim_ascii().is_empty())))
                {
                    return Ok(false);
                }
            }
            "WeaponSet1" | "WeaponSet2" => {
                if !plain_row(child, &["nodes"], true) || value(child, "nodes").is_none() {
                    return Ok(false);
                }
            }
            "Overrides" => {
                if !plain_row(child, &[], false)
                    || !container_text(child)
                    || child.children().len() != 1
                {
                    return Ok(false);
                }
                let entry = &evidence.rows()[child.children()[0].ordinal() as usize];
                charge_row(b, entry)?;
                if entry.occurrence().name() != "AttributeOverride"
                    || !plain_row(entry, &["strNodes", "dexNodes", "intNodes"], true)
                    || ["strNodes", "dexNodes", "intNodes"]
                        .iter()
                        .any(|key| value(entry, key).is_none())
                {
                    return Ok(false);
                }
            }
            "Sockets" => {
                if !plain_row(child, &[], false) || !container_text(child) {
                    return Ok(false);
                }
                let mut sockets = BTreeSet::new();
                for socket in child.children() {
                    let socket = &evidence.rows()[socket.ordinal() as usize];
                    charge_row(b, socket)?;
                    if socket.occurrence().name() != "Socket"
                        || !plain_row(socket, &["nodeId", "itemId"], true)
                        || !integer(value(socket, "nodeId"), 1, LUA_INTEGER_MAX)
                        || !integer(value(socket, "itemId"), 0, LUA_INTEGER_MAX)
                        || !sockets.insert(value(socket, "nodeId"))
                    {
                        return Ok(false);
                    }
                }
            }
            _ => return Ok(false),
        }
    }
    Ok(["URL", "Sockets", "Overrides"]
        .iter()
        .all(|name| seen.contains(name)))
}

pub(super) fn collect(
    b: &mut Builder<'_, '_>,
    policy: Option<&CompiledCharacterRewardInventory<'_>>,
) -> Result<BTreeMap<SourceOccurrenceId, ProvenCharacterRewards>> {
    let mut proofs = BTreeMap::new();
    let Some(policy) = policy else {
        return Ok(proofs);
    };
    b.charge(
        policy
            .tree_version
            .len()
            .saturating_add(policy.target_version.len()),
    )?;
    let evidence = b.evidence;
    let root = &evidence.rows()[0];
    charge_row(b, root)?;
    if root.occurrence().parent().is_some()
        || root.occurrence().name() != "PathOfBuilding2"
        || !plain_row(root, &[], false)
        || !container_text(root)
    {
        return Ok(proofs);
    }
    let mut sections = BTreeMap::new();
    for id in root.children() {
        let row = &evidence.rows()[id.ordinal() as usize];
        charge_row(b, row)?;
        let name = row.occurrence().name();
        if row.occurrence().has_namespace_context()
            || !matches!(
                name,
                "Build"
                    | "Tree"
                    | "Skills"
                    | "Config"
                    | "TreeView"
                    | "Items"
                    | "Calcs"
                    | "Party"
                    | "Import"
                    | "Notes"
            )
            || sections.insert(name, row).is_some()
        {
            return Ok(proofs);
        }
    }
    let (Some(build), Some(tree)) = (sections.get("Build"), sections.get("Tree")) else {
        return Ok(proofs);
    };
    if !fresh_build(b, build, policy.target_version)?
        || !plain_row(tree, &["activeSpec"], false)
        || !container_text(tree)
        || tree.children().is_empty()
        || !integer(value(tree, "activeSpec"), 1, tree.children().len() as u64)
    {
        return Ok(proofs);
    }
    if tree.children().len() > b.limits.draft.input.max_collection_entries {
        return Err(NormalizationError::Limit("character reward specs"));
    }
    for id in tree.children() {
        let row = &evidence.rows()[id.ordinal() as usize];
        if row.occurrence().parent() != Some(tree.occurrence().id())
            || !fresh_spec(b, row, policy.tree_version)?
        {
            return Ok(proofs);
        }
    }
    b.charge(tree.children().len())?;
    for spec in tree.children() {
        proofs.insert(
            *spec,
            ProvenCharacterRewards {
                build: build.occurrence().id(),
                spec: *spec,
            },
        );
    }
    Ok(proofs)
}

pub(super) fn finish(
    b: &mut Builder<'_, '_>,
    scope: SourceOccurrenceId,
    output: &mut DraftList<RewardSelectionId>,
    proof: Option<&ProvenCharacterRewards>,
) -> Result<()> {
    let Some(proof) = proof else { return Ok(()) };
    if proof.spec != scope
        || b.evidence.rows()[proof.build.ordinal() as usize]
            .occurrence()
            .id()
            != proof.build
        || !output.members.is_empty()
        || !matches!(&output.completion, DraftListCompletion::Pending { code, .. }
            if code.as_str() == "character-rewards-not-converted")
    {
        return Err(NormalizationError::Policy("character reward proof scope"));
    }
    source_shape::retire_membership(
        b,
        scope,
        &mut output.completion,
        "character-rewards-not-converted",
    )
}
