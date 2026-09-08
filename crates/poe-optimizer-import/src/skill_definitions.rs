//! Injected identity evidence for authored skills, before socket-group processing.
//! No name matching, effective set selection, actor resolution or mechanics run here.
use crate::{
    skill_source::{SkillProjection, SkillSourceKind, SkillSourceNode, SkillSourceUse},
    source_xml::SourceText,
};
use poe_optimizer_core::data::DataIdentity;
use poe_optimizer_data::{
    game_data::{DataTrust, GameDataSnapshot},
    skill_identities::{
        GemIdentity, GemIdentityResolutionReason, GemIdentityResolutionStatus, SkillIdentity,
        SkillIdentityCatalog,
    },
};
use serde::Serialize;
use std::ops::Range;

pub const MAX_IDENTITY_MATCHES: usize = 65_536;
pub const MAX_IDENTITY_STRING_BYTES: usize = 2 * 1024 * 1024;
#[derive(Debug, thiserror::Error)]
#[error("skill identity lookup: {0}")]
pub struct IdentityLookupError(&'static str);
type Result<T> = std::result::Result<T, IdentityLookupError>;
struct Budget {
    matches: usize,
    strings: usize,
}
impl Budget {
    fn record(&mut self) -> Result<()> {
        self.matches = self
            .matches
            .checked_sub(1)
            .ok_or(IdentityLookupError("identity match count exceeds limit"))?;
        Ok(())
    }
    fn text(&mut self, value: &str) -> Result<String> {
        self.strings = self
            .strings
            .checked_sub(value.len())
            .ok_or(IdentityLookupError(
                "identity evidence string bytes exceed limit",
            ))?;
        Ok(value.into())
    }
    fn source<'input>(
        &mut self,
        value: Option<&SourceText<'input>>,
    ) -> Result<Option<SourceText<'input>>> {
        value
            .map(|text| {
                self.strings =
                    self.strings
                        .checked_sub(text.decoded().len())
                        .ok_or(IdentityLookupError(
                            "identity evidence string bytes exceed limit",
                        ))?;
                Ok(text.clone())
            })
            .transpose()
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct GemIdentityMatch {
    pub key: String,
    pub game_id: String,
    pub variant_id: String,
    pub name: String,
    pub primary_effect_id: String,
    pub constructed_effect_list: Vec<String>,
    pub winning_declaration: u32,
}
fn gem_match(value: &GemIdentity, budget: &mut Budget) -> Result<GemIdentityMatch> {
    budget.record()?;
    Ok(GemIdentityMatch {
        key: budget.text(&value.key)?,
        game_id: budget.text(&value.game_id)?,
        variant_id: budget.text(&value.variant_id)?,
        name: budget.text(&value.name)?,
        primary_effect_id: budget.text(&value.primary_effect_id)?,
        constructed_effect_list: value
            .effect_list
            .iter()
            .map(|id| budget.text(id))
            .collect::<Result<_>>()?,
        winning_declaration: value.winning_declaration,
    })
}
#[derive(Debug, Clone, Serialize)]
pub struct SkillIdentityMatch {
    pub id: String,
    pub name: String,
    pub support: Option<bool>,
    pub from_tree: Option<bool>,
    pub winning_declaration: u32,
}
fn skill_match(value: &SkillIdentity, budget: &mut Budget) -> Result<SkillIdentityMatch> {
    budget.record()?;
    Ok(SkillIdentityMatch {
        id: budget.text(&value.id)?,
        name: budget.text(&value.name)?,
        support: value.support,
        from_tree: value.from_tree,
        winning_declaration: value.winning_declaration,
    })
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InstanceIdentityResolution {
    ExternalGem {
        status: GemIdentityResolutionStatus,
        reason: GemIdentityResolutionReason,
        candidates: Vec<GemIdentityMatch>,
    },
    ExplicitEffect {
        matched: Option<SkillIdentityMatch>,
        /// All possible primary owners; not a portable gemForSkill winner.
        possible_primary_gem_keys: Vec<String>,
    },
    NameOnlyNotResolved,
    MissingIdentity,
}
#[derive(Debug, Clone, Serialize)]
pub struct InstanceIdentityLookup<'input> {
    pub source_range: Range<usize>,
    pub source_kind: SkillSourceKind,
    pub game_id: Option<SourceText<'input>>,
    pub variant_id: Option<SourceText<'input>>,
    pub skill_id: Option<SourceText<'input>>,
    pub name_spec: Option<SourceText<'input>>,
    pub resolution: InstanceIdentityResolution,
}
#[derive(Debug, Clone, Serialize)]
pub struct EffectSelectionLookup<'input> {
    pub source_range: Range<usize>,
    pub source_use: SkillSourceUse,
    pub granted_effect: Option<SourceText<'input>>,
    pub matched: Option<SkillIdentityMatch>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "lookup", rename_all = "snake_case")]
pub enum SkillIdentityRecord<'input> {
    Instance(InstanceIdentityLookup<'input>),
    EffectSelection(EffectSelectionLookup<'input>),
}
#[derive(Debug, Clone, Serialize)]
pub struct LocatedSkillIdentityRecord<'input> {
    /// Zero-based authored container occurrence, never an active-set decision.
    pub container_index: usize,
    /// None for legacy direct groups; no implicit set identity is invented.
    pub set_source_range: Option<Range<usize>>,
    pub group_source_range: Option<Range<usize>>,
    pub record: SkillIdentityRecord<'input>,
}
/// Immutable source/data-bound metadata, not a deserializable native admission token.
#[derive(Debug, Clone, Serialize)]
pub struct SkillDefinitionLookup<'input> {
    schema_version: u32,
    interpretation: &'static str,
    source_xml_sha256: String,
    data: DataIdentity,
    data_trust: DataTrust,
    catalog_schema_version: u32,
    constructed_gem_count: usize,
    constructed_skill_count: usize,
    name_matching: &'static str,
    socket_group_processing: &'static str,
    active_set_selection: &'static str,
    actor_resolution: &'static str,
    game_mechanics: &'static str,
    records: Vec<LocatedSkillIdentityRecord<'input>>,
}
impl<'input> SkillDefinitionLookup<'input> {
    pub fn source_xml_sha256(&self) -> &str {
        &self.source_xml_sha256
    }
    pub fn data(&self) -> &DataIdentity {
        &self.data
    }
    pub fn records(&self) -> &[LocatedSkillIdentityRecord<'input>] {
        &self.records
    }
}
fn instance<'input>(
    node: &SkillSourceNode<'input>,
    catalog: &SkillIdentityCatalog,
    budget: &mut Budget,
) -> Result<InstanceIdentityLookup<'input>> {
    let source = node.element();
    let game_id = budget.source(source.attribute("gemId"))?;
    let variant_id = budget.source(source.attribute("variantId"))?;
    let skill_id = budget.source(source.attribute("skillId"))?;
    let name_spec = budget.source(source.attribute("nameSpec"))?;
    // Lua considers an authored empty string present too. Never fall through
    // from an unknown external game ID to a supplied valid effect ID.
    let resolution = if let Some(game_id) = &game_id {
        let result = catalog.resolve_external(
            game_id.decoded(),
            variant_id.as_ref().map(SourceText::decoded),
        );
        InstanceIdentityResolution::ExternalGem {
            status: result.status,
            reason: result.reason,
            candidates: result
                .candidates
                .into_iter()
                .map(|gem| gem_match(gem, budget))
                .collect::<Result<_>>()?,
        }
    } else if let Some(skill_id) = &skill_id {
        let matched = catalog
            .skill_by_id(skill_id.decoded())
            .map(|skill| skill_match(skill, budget))
            .transpose()?;
        let mut possible_primary_gem_keys = Vec::new();
        if matched.is_some() {
            for gem in catalog.gems_for_effect(skill_id.decoded()) {
                budget.record()?;
                possible_primary_gem_keys.push(budget.text(&gem.key)?);
            }
        }
        InstanceIdentityResolution::ExplicitEffect {
            matched,
            possible_primary_gem_keys,
        }
    } else if name_spec
        .as_ref()
        .is_some_and(|text| !text.decoded().is_empty())
    {
        InstanceIdentityResolution::NameOnlyNotResolved
    } else {
        InstanceIdentityResolution::MissingIdentity
    };
    Ok(InstanceIdentityLookup {
        source_range: source.source_range(),
        source_kind: node.kind(),
        game_id,
        variant_id,
        skill_id,
        name_spec,
        resolution,
    })
}
struct Walker<'a, 'input> {
    catalog: &'a SkillIdentityCatalog,
    budget: Budget,
    records: Vec<LocatedSkillIdentityRecord<'input>>,
}
impl<'input> Walker<'_, 'input> {
    fn node(
        &mut self,
        node: &SkillSourceNode<'input>,
        container: usize,
        mut set: Option<Range<usize>>,
        mut group: Option<Range<usize>>,
    ) -> Result<()> {
        use SkillSourceUse as U;
        match node.source_use() {
            U::SavedSet => {
                set = Some(node.element().source_range());
                group = None;
            }
            U::Group => group = Some(node.element().source_range()),
            _ => {}
        }
        let record = match node.source_use() {
            U::GemInstance => Some(SkillIdentityRecord::Instance(instance(
                node,
                self.catalog,
                &mut self.budget,
            )?)),
            U::MainStatSetSelection
            | U::CalcsStatSetSelection
            | U::MainMinionLookup
            | U::CalcsMinionLookup => {
                let granted_effect = self
                    .budget
                    .source(node.element().attribute("grantedEffect"))?;
                let matched = granted_effect
                    .as_ref()
                    .and_then(|key| self.catalog.skill_by_id(key.decoded()))
                    .map(|skill| skill_match(skill, &mut self.budget))
                    .transpose()?;
                Some(SkillIdentityRecord::EffectSelection(
                    EffectSelectionLookup {
                        source_range: node.element().source_range(),
                        source_use: node.source_use(),
                        granted_effect,
                        matched,
                    },
                ))
            }
            _ => None,
        };
        if let Some(record) = record {
            self.records.push(LocatedSkillIdentityRecord {
                container_index: container,
                set_source_range: set.clone(),
                group_source_range: group.clone(),
                record,
            });
        }
        for child in node.children() {
            self.node(child, container, set.clone(), group.clone())?;
        }
        Ok(())
    }
}
/// Inspect all authored sets; do not choose an active set or process any gem.
/// Unknown identifiers, source fallback candidates and ambiguous owners stay explicit.
pub fn lookup_definitions<'input>(
    projection: &SkillProjection<'input>,
    snapshot: &GameDataSnapshot,
) -> Result<SkillDefinitionLookup<'input>> {
    let catalog = snapshot.skill_identities();
    let mut walker = Walker {
        catalog,
        budget: Budget {
            matches: MAX_IDENTITY_MATCHES,
            strings: MAX_IDENTITY_STRING_BYTES,
        },
        records: Vec::new(),
    };
    for (index, node) in projection.containers().iter().enumerate() {
        walker.node(node, index, None, None)?;
    }
    Ok(SkillDefinitionLookup {
        schema_version: 1,
        interpretation: "authored_identity_before_socket_group_processing",
        source_xml_sha256: projection.source_sha256().into(),
        data: snapshot.identity().clone(),
        data_trust: snapshot.trust().clone(),
        catalog_schema_version: catalog.data().schema_version,
        constructed_gem_count: catalog.data().gems.len(),
        constructed_skill_count: catalog.data().skills.len(),
        name_matching: "not_run",
        socket_group_processing: "not_run",
        active_set_selection: "not_resolved",
        actor_resolution: "not_resolved",
        game_mechanics: "not_evaluated",
        records: walker.records,
    })
}
