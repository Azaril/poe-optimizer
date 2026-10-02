//! Shared strict saved-SkillSet framing for independent import inventories.
use super::*;
use source_shape::{charge_frame, container_text, plain_row, value};

fn numeric_key(text: &str) -> Option<u64> {
    if text.is_empty() || text.starts_with('0') || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse::<u64>().ok().filter(|v| *v < (1_u64 << 53))
}

const SKILLS_ATTRIBUTES: &[&str] = &[
    "activeSkillSet",
    "defaultGemLevel",
    "defaultGemQuality",
    "sortGemsByDPS",
    "showLegacyGems",
    "showSupportGemTypes",
    "sortGemsByDPSField",
    "matchGemLevelToCharacterLevel",
];
const GROUP_ATTRIBUTES: &[&str] = &[
    "active",
    "enabled",
    "includeInFullDPS",
    "groupCount",
    "label",
    "slot",
    "source",
    "mainActiveSkill",
    "mainActiveSkillCalcs",
    "skillPart",
];
const GEM_ATTRIBUTES: &[&str] = &[
    "nameSpec",
    "gemId",
    "variantId",
    "skillId",
    "level",
    "quality",
    "note",
    "enabled",
    "enableGlobal1",
    "enableGlobal2",
    "count",
    "statSetIndex",
    "statSetIndexCalcs",
    "skillPart",
    "skillPartCalcs",
    "skillStageCount",
    "skillStageCountCalcs",
    "skillMineCount",
    "skillMineCountCalcs",
    "skillMinion",
    "skillMinionCalcs",
    "skillMinionItemSet",
    "skillMinionItemSetCalcs",
    "skillMinionSkill",
    "skillMinionSkillCalcs",
    "corrupted",
    "corruptLevel",
];

/// Check every container and row frame first. The source loader iterates all sets
/// and treats every group child as a Gem, so an unknown sibling is not ignorable.
pub(super) fn sets(b: &mut Builder<'_, '_>) -> Result<Option<Vec<SourceOccurrenceId>>> {
    let evidence = b.evidence;
    let root = &evidence.rows()[0];
    charge_frame(b, root, &[])?;
    if root.occurrence().name() != "PathOfBuilding2"
        || !plain_row(root, &[], false)
        || !container_text(root)
    {
        return Ok(None);
    }
    let mut skills = None;
    for id in root.children() {
        let row = &evidence.rows()[id.ordinal() as usize];
        if row.occurrence().name() == "Skills" {
            if skills.replace(row).is_some() {
                return Ok(None);
            }
        } else if matches!(row.occurrence().name(), "SkillSet" | "Skill" | "Gem") {
            return Ok(None);
        }
    }
    let Some(skills) = skills else {
        return Ok(None);
    };
    charge_frame(b, skills, &["activeSkillSet"])?;
    if !plain_row(skills, SKILLS_ATTRIBUTES, false) || !container_text(skills) {
        return Ok(None);
    }
    let Some(selected) = value(skills, "activeSkillSet").and_then(numeric_key) else {
        return Ok(None);
    };
    if skills.children().len() > b.limits.draft.input.max_collection_entries {
        return Err(NormalizationError::Limit("support inventory sets"));
    }
    let mut keys = BTreeSet::new();
    let mut sets = Vec::new();
    for id in skills.children() {
        let set = &evidence.rows()[id.ordinal() as usize];
        charge_frame(b, set, &["id"])?;
        if set.occurrence().name() != "SkillSet"
            || set.occurrence().parent() != Some(skills.occurrence().id())
            || !plain_row(set, &["id", "title"], false)
            || !container_text(set)
            || !matches!(
                set.authored_instance(),
                Some(AuthoredInstanceId::SkillSet(_))
            )
        {
            return Ok(None);
        }
        let Some(key) = value(set, "id").and_then(numeric_key) else {
            return Ok(None);
        };
        if !keys.insert(key) {
            return Ok(None);
        }
        for id in set.children() {
            let group = &evidence.rows()[id.ordinal() as usize];
            charge_frame(b, group, &[])?;
            if group.occurrence().name() != "Skill"
                || group.occurrence().parent() != Some(set.occurrence().id())
                || !plain_row(group, GROUP_ATTRIBUTES, false)
                || !container_text(group)
                || !matches!(
                    group.authored_instance(),
                    Some(AuthoredInstanceId::SkillGroup(_))
                )
            {
                return Ok(None);
            }
            for id in group.children() {
                let gem = &evidence.rows()[id.ordinal() as usize];
                charge_frame(b, gem, &[])?;
                if gem.occurrence().name() != "Gem"
                    || gem.occurrence().parent() != Some(group.occurrence().id())
                    || !plain_row(gem, GEM_ATTRIBUTES, true)
                    || !matches!(
                        gem.authored_instance(),
                        Some(AuthoredInstanceId::SkillEntry(_))
                    )
                {
                    return Ok(None);
                }
            }
        }
        sets.push(set.occurrence().id());
    }
    Ok(keys.contains(&selected).then_some(sets))
}
