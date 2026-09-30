//! Finite receiving roles, shared by skill definitions and referenced by support Gems.
//! Paths are relative to the assigned skill's exact entering provider, never its parent.
use crate::{
    data::DataIdentity, owned_build::DeclaredSlot, owned_content::OwnedContentDigest,
    owned_definitions::*, owned_schema::DeclaredSet,
};
use serde::{Deserialize, Deserializer, Serialize};

pub const OWNED_SUPPORT_RECEIVING_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportReceivingInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub release: OwnedDefinitionKey,
    pub definitions: DataIdentity,
    pub rules: OwnedContentDigest,
    pub preparation: OwnedContentDigest,
    pub inputs: OwnedContentDigest,
    pub stages: OwnedContentDigest,
    pub roles: Vec<SupportReceivingRole>,
    pub targets: Vec<SupportTargetReceivingRoles>,
    pub supports: Vec<SupportReceivingEntry>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportReceiverKind {
    Actor,
    Action,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportReceivingRole {
    pub id: OwnedDefinitionKey,
    pub kind: SupportReceiverKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportTargetReceivingRoles {
    pub owner: SupportTargetDefinition,
    /// Only a Complete inventory proves an unlisted role absent for this skill.
    pub roles: DeclaredSet<SupportReceivingRoleBinding>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SupportTargetDefinition {
    Gem(GemDefId),
    Skill(SkillDefId),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportReceivingRoleBinding {
    pub role: OwnedDefinitionKey,
    pub endpoints: DeclaredSet<SupportReceiverEndpoint>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SupportReceiverEndpoint {
    /// Once per retained position and exact actor, independent of action count.
    Actor {
        path: Vec<DeclaredSlot<GrantSlotDefId>>,
        admission: SupportAdmissionContext,
    },
    Action {
        path: Vec<DeclaredSlot<GrantSlotDefId>>,
        output: DeclaredSlot<ActionOutputDefId>,
        selection: SupportActionSelection,
        admission: SupportAdmissionContext,
    },
}
impl SupportReceiverEndpoint {
    pub fn path(&self) -> &[DeclaredSlot<GrantSlotDefId>] {
        match self {
            Self::Actor { path, .. } | Self::Action { path, .. } => path,
        }
    }
    pub fn kind(&self) -> SupportReceiverKind {
        match self {
            Self::Actor { .. } => SupportReceiverKind::Actor,
            Self::Action { .. } => SupportReceiverKind::Action,
        }
    }
    pub fn admission(&self) -> &SupportAdmissionContext {
        match self {
            Self::Actor { admission, .. } | Self::Action { admission, .. } => admission,
        }
    }
}

fn required_option<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(d)
}

/// Which finite prepared skill context admits this receiving application. Native
/// eligibility remains separate from the authored applicability expression.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SupportAdmissionContext {
    AssignedSkill,
    ReceivingSkill {
        /// Null means no summoner; empty path explicitly means assigned target.
        /// A child uses its inherited selected list and the summoner's post-preparation types.
        #[serde(deserialize_with = "required_option")]
        summoner_path: Option<Vec<DeclaredSlot<GrantSlotDefId>>>,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SupportActionSelection {
    /// Expand only the declared finite product, with complete membership and a
    /// checked bound. Partial parts/modes/stat-sets never become an empty set.
    AllDeclared,
    Exact(Box<SupportActionVariant>),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportActionVariant {
    pub part: ActionPartDefId,
    pub mode: ActionModeDefId,
    pub stat_set: ActionStatSetDefId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportReceivingEntry {
    pub gem: GemDefId,
    pub receivers: DeclaredSet<SupportRolePrograms>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportRolePrograms {
    pub role: OwnedDefinitionKey,
    /// Exactly one final application-local applicability producer.
    pub applicability: OwnedDefinitionKey,
    /// Canonical set of program IDs; expression/effect ledger order is unchanged.
    pub delivery: Vec<OwnedDefinitionKey>,
}
