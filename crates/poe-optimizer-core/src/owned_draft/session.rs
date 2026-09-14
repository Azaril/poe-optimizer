//! Session DTOs retain independent semantic alternatives without selecting defaults.
use super::records::*;
use crate::{build_identity::*, owned_definitions::*, owned_project::*};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterPresetDraft {
    pub id: CharacterPresetId,
    pub class: DraftField<ClassDefId>,
    pub ascendancy: DraftField<Option<AscendancyDefId>>,
    pub level: DraftField<u16>,
    pub rewards: DraftList<RewardSelectionId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EquipmentPresetDraft {
    pub id: EquipmentPresetId,
    pub equipment: DraftList<ItemSlotUseId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllocationPresetDraft {
    pub id: AllocationPresetId,
    pub allocations: DraftList<AllocationId>,
    pub equipment: DraftList<ItemSlotUseId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillPresetDraft {
    pub id: SkillPresetId,
    pub skills: DraftList<SkillUseId>,
    pub supports: DraftList<SupportAssignmentId>,
    pub payload_links: DraftList<PayloadLinkId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChoicePresetDraft {
    pub id: ChoicePresetId,
    pub choices: DraftList<ChoiceDraft>,
    pub rewards: DraftList<RewardSelectionId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioPresetDraft {
    pub id: ScenarioPresetId,
    pub scenario: ScenarioDraft,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryPresetDraft {
    pub id: QueryPresetId,
    pub queries: QueryDraft,
}
/// Explicit selection; no source indexes, scenario/query pairing or active defaults.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationSelection {
    pub build: VariantSelection,
    pub scenario: ScenarioPresetId,
    pub queries: QueryPresetId,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionDraft {
    pub character: DraftField<CharacterPresetId>,
    pub equipment: DraftField<EquipmentPresetId>,
    pub allocations: DraftField<AllocationPresetId>,
    pub skills: DraftField<SkillPresetId>,
    pub choices: DraftField<ChoicePresetId>,
    pub active_weapon_loadout: DraftField<WeaponLoadoutId>,
    pub scenario: DraftField<ScenarioPresetId>,
    pub queries: DraftField<QueryPresetId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedVariantDraft {
    pub id: SavedVariantId,
    pub selection: SelectionDraft,
}
/// Raw partial semantic records, not validated input or numerical authority.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftSessionInput {
    pub allocator: InstanceAllocatorState,
    pub revision: BuildRevision,
    pub game_version: GameVersionNamespace,
    pub weapon_loadouts: DraftList<WeaponLoadoutId>,
    pub items: DraftList<ItemDraft>,
    pub gems: DraftList<GemDraft>,
    pub rewards: DraftList<RewardDraft>,
    pub equipment: DraftList<EquipmentDraft>,
    pub allocations: DraftList<AllocationDraft>,
    pub skills: DraftList<SkillDraft>,
    pub supports: DraftList<SupportDraft>,
    pub payload_links: DraftList<PayloadDraft>,
    pub character_presets: DraftList<CharacterPresetDraft>,
    pub equipment_presets: DraftList<EquipmentPresetDraft>,
    pub allocation_presets: DraftList<AllocationPresetDraft>,
    pub skill_presets: DraftList<SkillPresetDraft>,
    pub choice_presets: DraftList<ChoicePresetDraft>,
    pub scenario_presets: DraftList<ScenarioPresetDraft>,
    pub query_presets: DraftList<QueryPresetDraft>,
    pub saved_variants: DraftList<SavedVariantDraft>,
}
impl From<EvaluationSelection> for SelectionDraft {
    fn from(v: EvaluationSelection) -> Self {
        Self {
            character: v.build.character.into(),
            equipment: v.build.equipment.into(),
            allocations: v.build.allocations.into(),
            skills: v.build.skills.into(),
            choices: v.build.choices.into(),
            active_weapon_loadout: v.build.active_weapon_loadout.into(),
            scenario: v.scenario.into(),
            queries: v.queries.into(),
        }
    }
}
impl SelectionDraft {
    pub fn to_resolved(&self) -> Option<EvaluationSelection> {
        Some(EvaluationSelection {
            build: VariantSelection {
                character: self.character.to_resolved()?,
                equipment: self.equipment.to_resolved()?,
                allocations: self.allocations.to_resolved()?,
                skills: self.skills.to_resolved()?,
                choices: self.choices.to_resolved()?,
                active_weapon_loadout: self.active_weapon_loadout.to_resolved()?,
            },
            scenario: self.scenario.to_resolved()?,
            queries: self.queries.to_resolved()?,
        })
    }
}

// Identity list members are already complete; this does not certify membership.
macro_rules! identity_member {
    ($($id:ty),+ $(,)?) => { $(
        impl ResolveDraft for $id {
            type Resolved=Self;
            fn to_resolved(&self)->Option<Self>{Some(*self)}
        }
    )+ };
}
identity_member!(
    WeaponLoadoutId,
    RewardSelectionId,
    ItemSlotUseId,
    AllocationId,
    SkillUseId,
    SupportAssignmentId,
    PayloadLinkId
);
macro_rules! preset_conversion {
    ($draft:ident => $complete:ident { $($field:ident),+ $(,)? }) => {
        impl From<$complete> for $draft {
            fn from(value:$complete)->Self {
                Self{id:value.id,$($field:value.$field.into(),)+}
            }
        }
        impl ResolveDraft for $draft {
            type Resolved=$complete;
            fn to_resolved(&self)->Option<$complete>{
                Some($complete{id:self.id,$($field:self.$field.to_resolved()?,)+})
            }
        }
        impl $draft {
            pub fn to_resolved(&self)->Option<$complete>{ResolveDraft::to_resolved(self)}
        }
    };
}
preset_conversion!(CharacterPresetDraft=>CharacterPreset{class,ascendancy,level,rewards});
preset_conversion!(EquipmentPresetDraft=>EquipmentPreset{equipment});
preset_conversion!(AllocationPresetDraft=>AllocationPreset{allocations,equipment});
preset_conversion!(SkillPresetDraft=>SkillPreset{skills,supports,payload_links});
preset_conversion!(ChoicePresetDraft=>ChoicePreset{choices,rewards});
