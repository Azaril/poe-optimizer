//! Portable caller selections. Saved source keys and runtime positions are not instance IDs.
use crate::build_identity::{ConfigSetId, ItemSetId, PassiveSpecId, SkillSetId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "instance",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SelectionRequest<T> {
    #[default]
    Saved,
    Instance(T),
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponStateRequest {
    #[default]
    Saved,
    Primary,
    Secondary,
}
/// One view per evaluation. Labels are caller metadata, never admission tokens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewRequest {
    pub label: String,
    pub skills: SelectionRequest<SkillSetId>,
    pub items: SelectionRequest<ItemSetId>,
    pub passives: SelectionRequest<PassiveSpecId>,
    pub configuration: SelectionRequest<ConfigSetId>,
    pub weapon_state: WeaponStateRequest,
}
impl Default for ViewRequest {
    fn default() -> Self {
        Self {
            label: "saved".into(),
            skills: SelectionRequest::Saved,
            items: SelectionRequest::Saved,
            passives: SelectionRequest::Saved,
            configuration: SelectionRequest::Saved,
            weapon_state: WeaponStateRequest::Saved,
        }
    }
}
impl ViewRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.label.trim().is_empty()
            || self.label.len() > 128
            || self.label.chars().any(char::is_control)
        {
            return Err(
                "view label must be nonempty, at most 128 UTF-8 bytes and contain no control characters",
            );
        }
        Ok(())
    }
}
