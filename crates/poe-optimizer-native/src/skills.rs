//! Native authored skill loading, before effective support/actor/provider assembly.
use crate::CompiledGameData;
use poe_optimizer_core::{
    build_identity::{SkillEntryId, SkillGroupId},
    data::DataIdentity,
    evaluation::{EvaluationError, EvaluationErrorKind},
};
use poe_optimizer_engine::{
    lua_number::parse_number,
    lua_pattern::{CompileLimits, GsubLimits, LuaPattern, MatchBudget, MatchLimits, PatternError},
    selection_keys::NumericSetKeys,
};
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, ImportedBuildInstance, SourceOccurrenceId},
    selected_view::SelectedView,
    source_xml::{self, PobContentEntry},
};
use roxmltree::Node;
use serde::{Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc};

pub const SKILL_PREPARATION_SCHEMA: u32 = 1;
#[derive(Debug, Clone, Copy)]
pub struct SkillPreparationLimits {
    pub max_groups: usize,
    pub max_entries: usize,
    pub max_fields: usize,
    pub max_text_bytes: usize,
    pub max_fragments: usize,
    pub max_pattern_steps: u64,
}
impl Default for SkillPreparationLimits {
    fn default() -> Self {
        Self {
            max_groups: 4096,
            max_entries: 16384,
            max_fields: 500000,
            max_text_bytes: 8 * 1024 * 1024,
            max_fragments: 131072,
            max_pattern_steps: 100_000_000,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct SkillNumber(f64);
impl SkillNumber {
    pub fn value(self) -> f64 {
        self.0
    }
}
impl Serialize for SkillNumber {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("{:016x}", self.0.to_bits()))
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum SkillValue {
    Boolean(bool),
    Number(SkillNumber),
    Text(String),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillIdentityStatus {
    NotProcessed,
    ResolvedGem,
    ResolvedEffect,
    Empty,
    UnresolvedName,
    AmbiguousName,
    AmbiguousDefinition,
    HiddenGem,
    UnresolvedDefinition,
}
#[derive(Debug, Clone, Serialize)]
pub struct PreparedSkillGem {
    pub instance: SkillEntryId,
    pub source: SourceOccurrenceId,
    pub fields: BTreeMap<String, SkillValue>,
    pub gem_data: Option<String>,
    pub granted_effect: Option<String>,
    pub stat_set: BTreeMap<String, SkillNumber>,
    pub stat_set_calcs: BTreeMap<String, SkillNumber>,
    pub minion_skill_lookup: BTreeMap<String, BTreeMap<String, SkillNumber>>,
    pub minion_skill_lookup_calcs: BTreeMap<String, BTreeMap<String, SkillNumber>>,
    pub processed: bool,
    pub identity_status: SkillIdentityStatus,
}
impl PreparedSkillGem {
    pub fn number(&self, key: &str) -> Option<f64> {
        match self.fields.get(key) {
            Some(SkillValue::Number(value)) => Some(value.value()),
            _ => None,
        }
    }
    pub fn text(&self, key: &str) -> Option<&str> {
        match self.fields.get(key) {
            Some(SkillValue::Text(value)) => Some(value),
            _ => None,
        }
    }
    pub fn boolean(&self, key: &str) -> Option<bool> {
        match self.fields.get(key) {
            Some(SkillValue::Boolean(value)) => Some(*value),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct PreparedSkillGroup {
    pub instance: SkillGroupId,
    pub source: SourceOccurrenceId,
    pub fields: BTreeMap<String, SkillValue>,
    pub gems: Vec<PreparedSkillGem>,
    pub attached: bool,
    pub processing_passes: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillPreparationStatus {
    Complete,
    SourceFailure,
    Unsupported,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillFailureKind {
    SourceRuntime,
    UnsupportedSource,
    AmbiguousDefinition,
}
#[derive(Debug, Clone, Serialize)]
pub struct SkillPreparationFailure {
    pub kind: SkillFailureKind,
    pub stage: &'static str,
    pub source: Option<SourceOccurrenceId>,
    pub instance: Option<AuthoredInstanceId>,
    pub message: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct EffectCostOverride {
    pub row_id: String,
    pub effect_id: String,
    pub level: SkillNumber,
    pub origin: SkillEntryId,
}
#[derive(Debug, Clone, Serialize)]
pub struct SkillPreparationReport {
    pub schema_version: u32,
    pub scope: &'static str,
    pub source_sha256: String,
    pub data: DataIdentity,
    pub view_sha256: String,
    pub status: SkillPreparationStatus,
    pub containers: Vec<PreparedSkillContainer>,
    pub groups: Vec<PreparedSkillGroup>,
    pub selected_groups: Vec<SkillGroupId>,
    pub failure: Option<SkillPreparationFailure>,
    pub effect_cost_overrides: Vec<EffectCostOverride>,
    pub frontiers: Vec<&'static str>,
}
pub struct PreparedSkills {
    owner: ImportedBuildInstance,
    data: Arc<CompiledGameData>,
    report: SkillPreparationReport,
    cost_overrides: BTreeMap<String, EffectCostOverride>,
}
impl PreparedSkills {
    pub fn report(&self) -> &SkillPreparationReport {
        &self.report
    }
    /// Tests/provider preparation can query replacement identity without mutating
    /// shared definitions. Aliased effect-level rows observe the same override.
    pub fn has_cost_override(&self, effect_id: &str, level: f64) -> bool {
        self.data
            .snapshot()
            .skill_preparation()
            .effect(effect_id)
            .and_then(|effect| effect.level(level))
            .is_some_and(|row| self.cost_overrides.contains_key(&row.row_id))
    }
    pub fn validate_binding(
        &self,
        build: &ImportedBuildInstance,
        view: &SelectedView<'_>,
        data: &Arc<CompiledGameData>,
    ) -> Result<(), EvaluationError> {
        view.validate_binding(build, data.snapshot())
            .map_err(|e| contract(e.to_string()))?;
        if !self.owner.shares_storage_with(build)
            || !Arc::ptr_eq(&self.data, data)
            || self.report.view_sha256 != view_digest(view)?
        {
            return Err(contract(
                "authored skill stage belongs to another build, view or compiled definition owner",
            ));
        }
        Ok(())
    }
}
fn contract(message: impl Into<String>) -> EvaluationError {
    EvaluationError::new(EvaluationErrorKind::BackendContract, message)
}
fn resource(name: &'static str) -> EvaluationError {
    EvaluationError::new(
        EvaluationErrorKind::InvalidRequest,
        format!("authored skill preparation exceeds {name} limit"),
    )
}
fn view_digest(view: &SelectedView<'_>) -> Result<String, EvaluationError> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(view.report()).map_err(|e| contract(e.to_string()))?)
    ))
}
fn key(value: f64) -> u64 {
    if value == 0.0 { 0 } else { value.to_bits() }
}
fn put_number(fields: &mut BTreeMap<String, SkillValue>, name: &str, value: Option<f64>) {
    if let Some(value) = value {
        fields.insert(name.into(), SkillValue::Number(SkillNumber(value)));
    } else {
        fields.remove(name);
    }
}
fn put_text(fields: &mut BTreeMap<String, SkillValue>, name: &str, value: Option<String>) {
    if let Some(value) = value {
        fields.insert(name.into(), SkillValue::Text(value));
    } else {
        fields.remove(name);
    }
}
fn lua_min(a: f64, b: f64) -> f64 {
    if a < b { a } else { b }
}
fn lua_max(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}

struct Machine<'a> {
    build: &'a ImportedBuildInstance,
    data: &'a Arc<CompiledGameData>,
    report: SkillPreparationReport,
    sources: BTreeMap<usize, SourceOccurrenceId>,
    instances: BTreeMap<SourceOccurrenceId, AuthoredInstanceId>,
    limits: SkillPreparationLimits,
    pattern: MatchBudget,
    cost_overrides: BTreeMap<String, EffectCostOverride>,
}
impl Machine<'_> {
    fn spend(left: &mut usize, amount: usize, name: &'static str) -> Result<(), EvaluationError> {
        *left = left.checked_sub(amount).ok_or_else(|| resource(name))?;
        Ok(())
    }
    fn source(&self, node: Node<'_, '_>) -> Result<SourceOccurrenceId, EvaluationError> {
        self.sources
            .get(&node.range().start)
            .copied()
            .ok_or_else(|| contract("unmapped authored skill source"))
    }
    fn instance(&self, node: Node<'_, '_>) -> Result<AuthoredInstanceId, EvaluationError> {
        self.instances
            .get(&self.source(node)?)
            .copied()
            .ok_or_else(|| contract("unmapped authored skill instance"))
    }
    fn attr(&mut self, node: Node<'_, '_>, name: &str) -> Result<Option<String>, EvaluationError> {
        let value = self
            .build
            .attribute(self.source(node)?, name)
            .map_err(|e| contract(e.to_string()))?;
        value
            .map(|v| {
                Self::spend(
                    &mut self.limits.max_text_bytes,
                    v.decoded().len(),
                    "text bytes",
                )?;
                Ok(v.decoded().to_owned())
            })
            .transpose()
    }
    fn number(&mut self, node: Node<'_, '_>, name: &str) -> Result<Option<f64>, EvaluationError> {
        Ok(self
            .attr(node, name)?
            .as_deref()
            .and_then(|s| parse_number(s.as_bytes())))
    }
    fn children<'a, 'input>(
        &mut self,
        node: Node<'a, 'input>,
    ) -> Result<Vec<Option<Node<'a, 'input>>>, EvaluationError> {
        let content = source_xml::ordered_content(
            node,
            &mut self.limits.max_fragments,
            &mut self.limits.max_text_bytes,
        )
        .map_err(|e| EvaluationError::new(EvaluationErrorKind::InvalidRequest, e.to_string()))?;
        let elements: Vec<_> = node.children().filter(Node::is_element).collect();
        Ok(content
            .consumed()
            .iter()
            .map(|entry| match entry {
                PobContentEntry::Element { child_index } => Some(elements[*child_index]),
                PobContentEntry::Text { .. } => None,
            })
            .collect())
    }
    fn fail(
        &mut self,
        kind: SkillFailureKind,
        stage: &'static str,
        node: Option<Node<'_, '_>>,
        message: impl Into<String>,
    ) -> Result<(), EvaluationError> {
        let source = node.map(|n| self.source(n)).transpose()?;
        let message = message.into();
        Self::spend(
            &mut self.limits.max_text_bytes,
            message.len(),
            "failure message bytes",
        )?;
        self.report.status = if kind == SkillFailureKind::SourceRuntime {
            SkillPreparationStatus::SourceFailure
        } else {
            SkillPreparationStatus::Unsupported
        };
        self.report.failure = Some(SkillPreparationFailure {
            kind,
            stage,
            source,
            instance: source.and_then(|id| self.instances.get(&id).copied()),
            message,
        });
        Ok(())
    }
    fn known(&mut self, node: Node<'_, '_>) -> Result<bool, EvaluationError> {
        if self
            .build
            .occurrence(self.source(node)?)
            .map_err(|e| contract(e.to_string()))?
            .has_namespace_context()
        {
            self.fail(
                SkillFailureKind::UnsupportedSource,
                "source_namespace",
                Some(node),
                "namespaced authored skill content has no admitted loader interpretation",
            )?;
            Ok(false)
        } else {
            Ok(true)
        }
    }
    fn runtime_at(
        &mut self,
        stage: &'static str,
        source: SourceOccurrenceId,
        instance: AuthoredInstanceId,
        message: impl Into<String>,
    ) -> Result<(), EvaluationError> {
        let message = message.into();
        Self::spend(
            &mut self.limits.max_text_bytes,
            message.len(),
            "failure message bytes",
        )?;
        self.report.status = SkillPreparationStatus::SourceFailure;
        self.report.failure = Some(SkillPreparationFailure {
            kind: SkillFailureKind::SourceRuntime,
            stage,
            source: Some(source),
            instance: Some(instance),
            message,
        });
        Ok(())
    }
    fn gsub(
        &mut self,
        input: &[u8],
        pattern: &[u8],
        replacement: &[u8],
    ) -> Result<Vec<u8>, EvaluationError> {
        let compiled = LuaPattern::compile(pattern).map_err(|e| contract(e.to_string()))?;
        compiled
            .gsub(
                input,
                replacement,
                None,
                &mut self.pattern,
                GsubLimits {
                    max_output_bytes: self.limits.max_text_bytes,
                    ..GsubLimits::default()
                },
            )
            .map(|r| r.bytes)
            .map_err(|_| resource("pattern work/output"))
    }
    fn sanitise(&mut self, value: &str) -> Result<String, EvaluationError> {
        if !value.bytes().any(|b| b >= 128 || b == b'<') {
            return Ok(value.into());
        }
        let mut bytes = self.gsub(value.as_bytes(), b"%b<>", b"")?;
        for pattern in [
            b"\xe2\x80\x90".as_slice(),
            b"\xe2\x80\x91",
            b"\xe2\x80\x92",
            b"\xe2\x80\x93",
            b"\xe2\x80\x94",
            b"\xe2\x80\x95",
            b"\xe2\x88\x92",
        ] {
            bytes = self.gsub(&bytes, pattern, b"-")?;
        }
        for (pattern, replacement) in [
            (b"\xe2\x80\xa2 ?".as_slice(), b"".as_slice()),
            (b"\xc3\xa4", b"a"),
            (b"\xc3\xb6", b"o"),
            (b"\xc3\xad", b"i"),
            (b"\xc3\xb3", b"o"),
            (b"\x96", b"-"),
            (b"\x97", b"-"),
            (b"\xe4", b"a"),
            (b"\xf6", b"o"),
            (b"[\x80-\xff]", b"?"),
        ] {
            bytes = self.gsub(&bytes, pattern, replacement)?;
        }
        String::from_utf8(bytes).map_err(|_| contract("sanitiseText did not produce valid UTF-8"))
    }
    fn load_group(&mut self, node: Node<'_, '_>) -> Result<Option<usize>, EvaluationError> {
        if node.tag_name().name() != "Skill" {
            return Ok(None);
        }
        if !self.known(node)? {
            return Ok(None);
        }
        Self::spend(&mut self.limits.max_groups, 1, "groups")?;
        let AuthoredInstanceId::SkillGroup(instance) = self.instance(node)? else {
            return Err(contract("expected authored group identity"));
        };
        let mut fields = BTreeMap::new();
        fields.insert(
            "enabled".into(),
            SkillValue::Boolean(
                self.attr(node, "active")?.as_deref() == Some("true")
                    || self.attr(node, "enabled")?.as_deref() == Some("true"),
            ),
        );
        if let Some(value) = self.attr(node, "includeInFullDPS")? {
            fields.insert(
                "includeInFullDPS".into(),
                SkillValue::Boolean(value == "true"),
            );
        }
        put_number(&mut fields, "groupCount", self.number(node, "groupCount")?);
        for name in ["label", "slot", "source"] {
            put_text(&mut fields, name, self.attr(node, name)?);
        }
        for name in ["mainActiveSkill", "mainActiveSkillCalcs"] {
            put_number(
                &mut fields,
                name,
                Some(self.number(node, name)?.unwrap_or(1.0)),
            );
        }
        self.charge_fields(&fields)?;
        let index = self.report.groups.len();
        self.report.groups.push(PreparedSkillGroup {
            instance,
            source: self.source(node)?,
            fields,
            gems: vec![],
            attached: false,
            processing_passes: 0,
        });
        for child in self.children(node)? {
            let Some(child) = child else {
                self.fail(
                    SkillFailureKind::SourceRuntime,
                    "load_skill",
                    Some(node),
                    "attempt to index gem text child attributes",
                )?;
                return Ok(Some(index));
            };
            if !self.known(child)? {
                return Ok(Some(index));
            }
            Self::spend(&mut self.limits.max_entries, 1, "entries")?;
            let AuthoredInstanceId::SkillEntry(instance) = self.instance(child)? else {
                return Err(contract("expected authored gem instance"));
            };
            let mut gem = PreparedSkillGem {
                instance,
                source: self.source(child)?,
                fields: BTreeMap::new(),
                gem_data: None,
                granted_effect: None,
                stat_set: BTreeMap::new(),
                stat_set_calcs: BTreeMap::new(),
                minion_skill_lookup: BTreeMap::new(),
                minion_skill_lookup_calcs: BTreeMap::new(),
                processed: false,
                identity_status: SkillIdentityStatus::NotProcessed,
            };
            let name = self.attr(child, "nameSpec")?.unwrap_or_default();
            put_text(&mut gem.fields, "nameSpec", Some(self.sanitise(&name)?));
            self.load_identity(child, &mut gem)?;
            if self.report.failure.is_some() {
                self.report.groups[index].gems.push(gem);
                return Ok(Some(index));
            }
            for name in [
                "level",
                "quality",
                "skillPart",
                "skillPartCalcs",
                "skillStageCount",
                "skillStageCountCalcs",
                "skillMineCount",
                "skillMineCountCalcs",
                "skillMinionItemSet",
                "skillMinionItemSetCalcs",
                "skillMinionSkill",
                "skillMinionSkillCalcs",
            ] {
                put_number(&mut gem.fields, name, self.number(child, name)?);
            }
            for name in ["note", "skillMinion", "skillMinionCalcs"] {
                put_text(&mut gem.fields, name, self.attr(child, name)?);
            }
            for name in ["enabled", "enableGlobal1"] {
                gem.fields.insert(
                    name.into(),
                    SkillValue::Boolean(self.attr(child, name)?.is_none_or(|v| v == "true")),
                );
            }
            for name in ["enableGlobal2", "corrupted"] {
                gem.fields.insert(
                    name.into(),
                    SkillValue::Boolean(self.attr(child, name)?.as_deref() == Some("true")),
                );
            }
            put_number(
                &mut gem.fields,
                "count",
                Some(self.number(child, "count")?.unwrap_or(1.0)),
            );
            put_number(
                &mut gem.fields,
                "corruptLevel",
                Some(self.number(child, "corruptLevel")?.unwrap_or(0.0)),
            );
            self.load_maps(child, &mut gem)?;
            Self::spend(&mut self.limits.max_fields, gem.fields.len(), "fields")?;
            self.charge_gem_text(&gem)?;
            self.report.groups[index].gems.push(gem);
            if self.report.failure.is_some() {
                return Ok(Some(index));
            }
        }
        if let Some(raw) = self.attr(node, "skillPart")?
            && let Some(gem) = self.report.groups[index].gems.first_mut()
        {
            put_number(&mut gem.fields, "skillPart", parse_number(raw.as_bytes()));
        }
        self.process_group(index)?;
        Ok(Some(index))
    }
    fn load_maps(
        &mut self,
        node: Node<'_, '_>,
        gem: &mut PreparedSkillGem,
    ) -> Result<(), EvaluationError> {
        for child in self.children(node)?.into_iter().flatten() {
            if !self.known(child)? {
                return Ok(());
            }
            let name = child.tag_name().name();
            if !matches!(
                name,
                "StatSetIndex"
                    | "StatSetCalcsIndex"
                    | "MinionSkillIndexLookup"
                    | "MinionSkillIndexLookupCalcs"
            ) {
                continue;
            }
            let Some(effect) = self.attr(child, "grantedEffect")? else {
                continue;
            };
            Self::spend(&mut self.limits.max_fields, 1, "map fields")?;
            if matches!(name, "StatSetIndex" | "StatSetCalcsIndex") {
                let value = self.number(child, "index")?;
                let map = if name == "StatSetIndex" {
                    &mut gem.stat_set
                } else {
                    &mut gem.stat_set_calcs
                };
                if let Some(value) = value {
                    map.insert(effect, SkillNumber(value));
                } else {
                    map.remove(&effect);
                }
            } else {
                let map = if name == "MinionSkillIndexLookup" {
                    &mut gem.minion_skill_lookup
                } else {
                    &mut gem.minion_skill_lookup_calcs
                };
                map.insert(effect.clone(), BTreeMap::new());
                for row in self.children(child)? {
                    let Some(row) = row else {
                        self.fail(
                            SkillFailureKind::SourceRuntime,
                            "load_minion_map",
                            Some(child),
                            "attempt to index minion-map text child attributes",
                        )?;
                        return Ok(());
                    };
                    if !self.known(row)? {
                        return Ok(());
                    }
                    let index = self.number(row, "skillIndex")?;
                    let value = self.number(row, "statSetIndex")?;
                    let Some(index) = index else {
                        self.fail(
                            SkillFailureKind::SourceRuntime,
                            "load_minion_map",
                            Some(row),
                            "table index is nil",
                        )?;
                        return Ok(());
                    };
                    if index.is_nan() {
                        self.fail(
                            SkillFailureKind::SourceRuntime,
                            "load_minion_map",
                            Some(row),
                            "table index is NaN",
                        )?;
                        return Ok(());
                    }
                    let map = map.get_mut(&effect).expect("created minion effect map");
                    Self::spend(&mut self.limits.max_fields, 1, "map fields")?;
                    let k = format!("{:016x}", key(index));
                    if let Some(value) = value {
                        map.insert(k, SkillNumber(value));
                    } else {
                        map.remove(&k);
                    }
                }
            }
        }
        Ok(())
    }
}
impl Machine<'_> {
    fn load_identity(
        &mut self,
        node: Node<'_, '_>,
        gem: &mut PreparedSkillGem,
    ) -> Result<(), EvaluationError> {
        if let Some(game_id) = self.attr(node, "gemId")? {
            let variant = self.attr(node, "variantId")?;
            let catalog = self.data.snapshot().skill_preparation();
            let found = if let Some(variants) = catalog.external_variants(&game_id) {
                let requested = variant.as_ref().filter(|id| {
                    variants.variants.contains_key(*id)
                        || variants.ambiguous_variants.contains_key(*id)
                });
                let selected = requested.or_else(|| {
                    (variants.canonical_variant_order.len() == 1)
                        .then(|| &variants.canonical_variant_order[0])
                });
                if requested.is_none() && variants.canonical_variant_order.len() > 1 {
                    gem.identity_status = SkillIdentityStatus::AmbiguousDefinition;
                    self.fail(SkillFailureKind::AmbiguousDefinition, "load_skill_identity", Some(node),
                        format!("external gem {game_id:?} fallback depends on unordered source variants {:?}", variants.canonical_variant_order))?;
                    return Ok(());
                }
                if let Some(id) = selected
                    && let Some(candidates) = variants.ambiguous_variants.get(id)
                {
                    gem.identity_status = SkillIdentityStatus::AmbiguousDefinition;
                    self.fail(SkillFailureKind::AmbiguousDefinition, "load_skill_identity", Some(node),
                        format!("external gem {game_id:?} variant {id:?} has ambiguous definition owners {candidates:?}"))?;
                    return Ok(());
                }
                selected.and_then(|id| variants.variants.get(id))
            } else {
                None
            };
            if let Some(id) = found {
                let identity = self
                    .data
                    .snapshot()
                    .skill_identities()
                    .gem_by_key(id)
                    .ok_or_else(|| contract("preparation gem lacks identity"))?;
                put_text(&mut gem.fields, "gemId", Some(identity.key.clone()));
                put_text(
                    &mut gem.fields,
                    "skillId",
                    Some(identity.primary_effect_id.clone()),
                );
                if let Some(name) = &identity.name_spec {
                    put_text(&mut gem.fields, "nameSpec", Some(name.clone()));
                }
            }
        } else if let Some(skill_id) = self.attr(node, "skillId")? {
            let catalog = self.data.snapshot().skill_preparation();
            if let Some(candidates) = catalog.ambiguous_table_gem_for_skill(&skill_id) {
                gem.identity_status = SkillIdentityStatus::AmbiguousDefinition;
                self.fail(
                    SkillFailureKind::AmbiguousDefinition,
                    "load_skill_identity",
                    Some(node),
                    format!("effect {skill_id:?} has ambiguous source gem owners {candidates:?}"),
                )?;
                return Ok(());
            }
            if let Some(effect) = catalog.effect(&skill_id) {
                put_text(
                    &mut gem.fields,
                    "gemId",
                    catalog.table_gem_for_skill(&effect.id).map(str::to_owned),
                );
                put_text(&mut gem.fields, "skillId", Some(effect.id.clone()));
                put_text(&mut gem.fields, "nameSpec", Some(effect.name.clone()));
            }
        }
        Ok(())
    }
    fn find_gem(
        &mut self,
        name: &str,
        gem: &mut PreparedSkillGem,
    ) -> Result<Option<String>, EvaluationError> {
        let bytes = name.as_bytes();
        let letters = bytes.iter().filter(|b| b.is_ascii_alphabetic()).count();
        let lowercase = bytes.iter().filter(|b| b.is_ascii_lowercase()).count();
        let compact = bytes.iter().filter(|&&b| b != b' ').count();
        let lengths = [
            3 + bytes.len() + 3 * letters,
            2 + bytes.len() + 4 * letters,
            6 + compact + 3 * lowercase,
            1 + compact + 2 * letters,
            1 + compact + 5 * letters,
        ];
        if lengths
            .iter()
            .any(|&n| n > CompileLimits::default().max_pattern_bytes)
        {
            return Err(resource("name pattern bytes"));
        }
        Self::spend(
            &mut self.limits.max_text_bytes,
            lengths.iter().sum(),
            "name match patterns",
        )?;
        let mut patterns: Vec<Vec<u8>> = vec![
            b"^ ".to_vec(),
            b"^".to_vec(),
            b"^ ".to_vec(),
            b"^".to_vec(),
            b"^".to_vec(),
        ];
        for byte in name.bytes() {
            if byte.is_ascii_alphabetic() {
                patterns[0].extend_from_slice(&[
                    b'[',
                    byte.to_ascii_uppercase(),
                    byte.to_ascii_lowercase(),
                    b']',
                ]);
                patterns[1].extend_from_slice(&[b' ', byte, b'%', b'l', b'+']);
            } else {
                patterns[0].push(byte);
                patterns[1].push(byte);
            }
            if byte != b' ' {
                if byte.is_ascii_lowercase() {
                    patterns[2].extend_from_slice(b"%l*");
                }
                patterns[2].push(byte);
                if byte.is_ascii_alphabetic() {
                    patterns[3].extend_from_slice(b".*");
                    patterns[4].extend_from_slice(&[
                        b'.',
                        b'*',
                        b'[',
                        byte.to_ascii_uppercase(),
                        byte.to_ascii_lowercase(),
                        b']',
                    ]);
                } else {
                    patterns[4].push(byte);
                }
                patterns[3].push(byte);
            }
        }
        patterns[0].push(b'$');
        patterns[1].push(b'$');
        patterns[2].extend_from_slice(b"%l+$");
        let mut ambiguous = false;
        for pattern in patterns {
            let mut found: Option<(String, String)> = None;
            let mut compiled = None;
            let catalog = self.data.snapshot().skill_preparation();
            for definition in catalog.gems_ordered() {
                let identity = self
                    .data
                    .snapshot()
                    .skill_identities()
                    .gem_by_key(&definition.key)
                    .ok_or_else(|| contract("name search gem lacks identity"))?;
                if compiled.is_none() {
                    match LuaPattern::compile_with_limits(&pattern, CompileLimits::default()) {
                        Ok(value) => compiled = Some(value),
                        Err(PatternError::Source(error)) => {
                            self.runtime_at(
                                "find_skill_gem",
                                gem.source,
                                AuthoredInstanceId::SkillEntry(gem.instance),
                                error.message(),
                            )?;
                            return Ok(None);
                        }
                        Err(_) => return Err(resource("name pattern compilation")),
                    }
                }
                let mut subject = Vec::with_capacity(identity.name.len() + 1);
                subject.push(b' ');
                subject.extend_from_slice(identity.name.as_bytes());
                let matched = compiled
                    .as_ref()
                    .expect("compiled search pattern")
                    .match_captures(&subject, 1, &mut self.pattern);
                match matched {
                    Ok(Some(_)) => {
                        if let Some((_, previous)) = &found {
                            put_text(
                                &mut gem.fields,
                                "errMsg",
                                Some(format!(
                                    "Ambiguous gem name '{name}': matches '{previous}', '{}'",
                                    identity.name
                                )),
                            );
                            ambiguous = true;
                            break;
                        }
                        found = Some((identity.key.clone(), identity.name.clone()));
                    }
                    Ok(None) => {}
                    Err(PatternError::Source(error)) => {
                        self.runtime_at(
                            "find_skill_gem",
                            gem.source,
                            AuthoredInstanceId::SkillEntry(gem.instance),
                            error.message(),
                        )?;
                        return Ok(None);
                    }
                    Err(_) => return Err(resource("name pattern matching")),
                }
            }
            if ambiguous {
                gem.identity_status = SkillIdentityStatus::AmbiguousName;
                return Ok(None);
            }
            if let Some((id, _)) = found {
                gem.fields.remove("errMsg");
                return Ok(Some(id));
            }
        }
        gem.identity_status = SkillIdentityStatus::UnresolvedName;
        put_text(
            &mut gem.fields,
            "errMsg",
            Some(format!("Unrecognised gem name '{name}'")),
        );
        Ok(None)
    }
    fn process_group(&mut self, index: usize) -> Result<(), EvaluationError> {
        self.report.groups[index].processing_passes += 1;
        let mut gems = std::mem::take(&mut self.report.groups[index].gems);
        let mut result = Ok(());
        for gem in &mut gems {
            result = self.process_gem(gem);
            if result.is_err() || self.report.failure.is_some() {
                break;
            }
        }
        self.report.groups[index].gems = gems;
        result
    }
    fn process_gem(&mut self, gem: &mut PreparedSkillGem) -> Result<(), EvaluationError> {
        gem.processed = false;
        let catalog = self.data.snapshot().skill_preparation();
        put_text(
            &mut gem.fields,
            "color",
            Some(catalog.data().colors.unresolved.clone()),
        );
        if gem.text("nameSpec").is_none() {
            put_text(&mut gem.fields, "nameSpec", Some(String::new()));
        }
        let previous = gem
            .gem_data
            .as_deref()
            .and_then(|id| catalog.gem(id))
            .map(|g| g.natural_max_level)
            .or_else(|| {
                gem.boolean("new")
                    .filter(|v| *v)
                    .map(|_| catalog.data().initial_new_gem_level)
            });
        gem.gem_data = None;
        gem.granted_effect = None;
        gem.identity_status = SkillIdentityStatus::UnresolvedDefinition;
        if let Some(id) = gem.text("gemId").map(str::to_owned) {
            gem.fields.remove("errMsg");
            if let Some(identity) = self.data.snapshot().skill_identities().gem_by_key(&id) {
                gem.gem_data = Some(id);
                if !gem
                    .text("nameSpec")
                    .is_some_and(|s| s.starts_with("Companion:") || s.starts_with("Spectre:"))
                {
                    put_text(&mut gem.fields, "nameSpec", Some(identity.name.clone()));
                }
                put_text(
                    &mut gem.fields,
                    "skillId",
                    Some(identity.primary_effect_id.clone()),
                );
            }
        } else if let Some(id) = gem.text("skillId").map(str::to_owned) {
            gem.fields.remove("errMsg");
            if let Some(gem_id) = catalog.string_gem_for_skill(&id) {
                if self
                    .data
                    .snapshot()
                    .skill_identities()
                    .gem_by_key(gem_id)
                    .is_some()
                {
                    gem.gem_data = Some(gem_id.into());
                }
            } else if catalog.effect(&id).is_some() {
                gem.granted_effect = Some(id.clone());
            }
            if gem.boolean("triggered") == Some(true)
                && let Some(effect_id) = &gem.granted_effect
                && let Some(level) = gem.number("level")
                && let Some(row) = catalog
                    .effect(effect_id)
                    .and_then(|effect| effect.level(level))
            {
                Self::spend(&mut self.limits.max_fields, 1, "cost overrides")?;
                Self::spend(
                    &mut self.limits.max_text_bytes,
                    2 * row.row_id.len() + effect_id.len(),
                    "cost override text",
                )?;
                self.cost_overrides.insert(
                    row.row_id.clone(),
                    EffectCostOverride {
                        row_id: row.row_id.clone(),
                        effect_id: effect_id.clone(),
                        level: SkillNumber(level),
                        origin: gem.instance,
                    },
                );
            }
        } else if gem.text("nameSpec").is_some_and(|s| {
            s.bytes()
                .any(|b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 11 | 12))
        }) {
            let name = gem.text("nameSpec").unwrap().to_owned();
            gem.gem_data = self.find_gem(&name, gem)?;
            if self.report.failure.is_some() {
                return Ok(());
            }
            let identity = gem
                .gem_data
                .as_deref()
                .and_then(|id| self.data.snapshot().skill_identities().gem_by_key(id));
            put_text(&mut gem.fields, "gemId", identity.map(|g| g.key.clone()));
            put_text(
                &mut gem.fields,
                "skillId",
                identity.map(|g| g.primary_effect_id.clone()),
            );
            if let Some(identity) = identity {
                put_text(&mut gem.fields, "nameSpec", Some(identity.name.clone()));
            }
        } else {
            gem.fields.remove("errMsg");
            gem.fields.remove("skillId");
            gem.identity_status = SkillIdentityStatus::Empty;
        }
        let catalog = self.data.snapshot().skill_preparation();
        let primary = gem
            .gem_data
            .as_deref()
            .and_then(|id| self.data.snapshot().skill_identities().gem_by_key(id))
            .map(|identity| identity.primary_effect_id.clone());
        if let Some(primary) = &primary {
            let Some(effect) = catalog.effect(primary) else {
                self.runtime_at(
                    "process_socket_group",
                    gem.source,
                    AuthoredInstanceId::SkillEntry(gem.instance),
                    "attempt to index a missing gem grantedEffect",
                )?;
                return Ok(());
            };
            if effect.hide_from_sidebar == Some(true) {
                let message = format!(
                    "{} cannot be used as an active skill",
                    gem.text("nameSpec").unwrap_or("")
                );
                put_text(&mut gem.fields, "errMsg", Some(message));
                gem.gem_data = None;
                gem.identity_status = SkillIdentityStatus::HiddenGem;
            }
        }
        if gem.gem_data.is_some() || gem.granted_effect.is_some() {
            gem.identity_status = if gem.gem_data.is_some() {
                SkillIdentityStatus::ResolvedGem
            } else {
                SkillIdentityStatus::ResolvedEffect
            };
            let effect_id = gem
                .granted_effect
                .clone()
                .or(primary)
                .ok_or_else(|| contract("resolved gem lacks effect identity"))?;
            let effect = catalog
                .effect(&effect_id)
                .ok_or_else(|| contract("resolved effect lacks preparation definition"))?;
            let colors = &catalog.data().colors;
            let color = match effect.color {
                Some(1.0) => &colors.strength,
                Some(2.0) => &colors.dexterity,
                Some(3.0) => &colors.intelligence,
                _ => &colors.normal,
            };
            put_text(&mut gem.fields, "color", Some(color.clone()));
            let gem_definition = gem.gem_data.as_deref().and_then(|id| catalog.gem(id));
            if let Some(previous) = previous
                && let Some(definition) = gem_definition
                && definition.natural_max_level != previous
            {
                put_number(&mut gem.fields, "level", Some(definition.natural_max_level));
                put_number(
                    &mut gem.fields,
                    "naturalMaxLevel",
                    Some(definition.natural_max_level),
                );
            } else if gem.boolean("new") == Some(true) {
                let Some(definition) = gem_definition else {
                    self.runtime_at(
                        "process_socket_group",
                        gem.source,
                        AuthoredInstanceId::SkillEntry(gem.instance),
                        "new gem has no gemData.naturalMaxLevel",
                    )?;
                    return Ok(());
                };
                put_number(&mut gem.fields, "level", Some(definition.natural_max_level));
                put_number(
                    &mut gem.fields,
                    "naturalMaxLevel",
                    Some(definition.natural_max_level),
                );
                gem.fields.remove("new");
            }
            let mut level = gem.number("level");
            if level.and_then(|n| effect.level(n)).is_none() {
                let Some(value) = level else {
                    self.runtime_at(
                        "validate_gem_level",
                        gem.source,
                        AuthoredInstanceId::SkillEntry(gem.instance),
                        "bad argument #2 to math.max (number expected, got nil)",
                    )?;
                    return Ok(());
                };
                level = Some(lua_max(catalog.data().minimum_gem_level, value));
                if effect.levels_length > 0 {
                    level = Some(lua_min(effect.levels_length as f64, level.unwrap()));
                }
            }
            if level.and_then(|n| effect.level(n)).is_none()
                && let Some(definition) = gem_definition
            {
                level = Some(definition.natural_max_level);
            }
            if level.and_then(|n| effect.level(n)).is_none() {
                level = effect.next_level_key;
            }
            put_number(&mut gem.fields, "level", level);
            if let Some(definition) = gem_definition {
                let Some(row) = level.and_then(|n| effect.level(n)) else {
                    self.runtime_at(
                        "gem_requirements",
                        gem.source,
                        AuthoredInstanceId::SkillEntry(gem.instance),
                        "attempt to index missing gem level row",
                    )?;
                    return Ok(());
                };
                put_number(&mut gem.fields, "reqLevel", row.level_requirement);
                let formula = &catalog.data().requirement;
                for (name, multi) in [
                    ("reqStr", definition.req_str),
                    ("reqDex", definition.req_dex),
                    ("reqInt", definition.req_int),
                ] {
                    let requirement = if multi == Some(0.0) || effect.support == Some(true) {
                        0.0
                    } else {
                        let Some(level) = row.level_requirement else {
                            self.runtime_at(
                                "gem_requirements",
                                gem.source,
                                AuthoredInstanceId::SkillEntry(gem.instance),
                                "attempt to perform arithmetic on nil level requirement",
                            )?;
                            return Ok(());
                        };
                        let Some(multi) = multi else {
                            self.runtime_at(
                                "gem_requirements",
                                gem.source,
                                AuthoredInstanceId::SkillEntry(gem.instance),
                                "attempt to perform arithmetic on nil attribute multiplier",
                            )?;
                            return Ok(());
                        };
                        let raw = (formula.base
                            + (level - formula.level_offset) * formula.level_multiplier)
                            * (multi / formula.attribute_divisor).powf(formula.attribute_exponent);
                        let requirement =
                            (raw + formula.round_bias).floor() + formula.result_offset;
                        if requirement < formula.minimum_requirement {
                            0.0
                        } else {
                            requirement
                        }
                    };
                    put_number(&mut gem.fields, name, Some(requirement));
                }
            }
            gem.identity_status = if gem.gem_data.is_some() {
                SkillIdentityStatus::ResolvedGem
            } else {
                SkillIdentityStatus::ResolvedEffect
            };
        }
        Self::spend(
            &mut self.limits.max_fields,
            gem.fields.len(),
            "processed fields",
        )?;
        self.charge_gem_text(gem)?;
        gem.processed = true;
        Ok(())
    }
    fn charge_fields(
        &mut self,
        fields: &BTreeMap<String, SkillValue>,
    ) -> Result<(), EvaluationError> {
        Self::spend(&mut self.limits.max_fields, fields.len(), "expanded fields")?;
        let bytes = fields
            .iter()
            .map(|(key, value)| {
                key.len()
                    + match value {
                        SkillValue::Text(text) => text.len(),
                        _ => 0,
                    }
            })
            .sum();
        Self::spend(
            &mut self.limits.max_text_bytes,
            bytes,
            "expanded field text",
        )
    }
    fn charge_gem_text(&mut self, gem: &PreparedSkillGem) -> Result<(), EvaluationError> {
        let mut bytes = gem
            .fields
            .iter()
            .map(|(key, value)| {
                key.len()
                    + match value {
                        SkillValue::Text(text) => text.len(),
                        _ => 0,
                    }
            })
            .sum::<usize>();
        bytes += gem.gem_data.as_ref().map_or(0, String::len)
            + gem.granted_effect.as_ref().map_or(0, String::len);
        Self::spend(&mut self.limits.max_text_bytes, bytes, "expanded gem text")
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct PreparedSkillContainer {
    pub source: SourceOccurrenceId,
    pub fields: BTreeMap<String, SkillValue>,
    pub order: Vec<SkillNumber>,
    pub active_set_id: Option<SkillNumber>,
}
impl Machine<'_> {
    fn load_container(
        &mut self,
        node: Node<'_, '_>,
        controls: &mut [String; 3],
        flags: &mut [bool; 2],
    ) -> Result<(), EvaluationError> {
        if !self.known(node)? {
            return Ok(());
        }
        let requested_default =
            if self.attr(node, "matchGemLevelToCharacterLevel")?.as_deref() == Some("true") {
                "characterLevel".to_owned()
            } else {
                match self.attr(node, "defaultGemLevel")? {
                    Some(value) if parse_number(value.as_bytes()).is_none() => value,
                    _ => self
                        .data
                        .snapshot()
                        .skill_preparation()
                        .data()
                        .default_gem_level
                        .clone(),
                }
            };
        let requested_quality = self.number(node, "defaultGemQuality")?;
        let policy = self.data.snapshot().skill_preparation().data();
        let quality = lua_max(
            lua_min(
                requested_quality.unwrap_or(policy.default_gem_quality),
                policy.maximum_gem_quality,
            ),
            policy.minimum_gem_quality,
        );
        if let Some(value) = self.attr(node, "sortGemsByDPS")? {
            flags[0] = value == "true";
        }
        if let Some(value) = self.attr(node, "showLegacyGems")? {
            flags[1] = value == "true";
        }
        let requested_support = self.attr(node, "showSupportGemTypes")?.unwrap_or_else(|| {
            self.data
                .snapshot()
                .skill_preparation()
                .data()
                .default_support_type
                .clone()
        });
        let requested_sort = self.attr(node, "sortGemsByDPSField")?.unwrap_or_else(|| {
            self.data
                .snapshot()
                .skill_preparation()
                .data()
                .default_sort_field
                .clone()
        });
        let definitions = self.data.snapshot().skill_preparation().data();
        for (index, (requested, allowed)) in [
            (requested_default, &definitions.default_gem_level_options),
            (requested_support, &definitions.support_type_options),
            (requested_sort, &definitions.sort_field_options),
        ]
        .into_iter()
        .enumerate()
        {
            if allowed.contains(&requested) {
                controls[index] = requested;
            }
        }
        let mut fields = BTreeMap::new();
        for (name, value) in [
            ("defaultGemLevel", &controls[0]),
            ("showSupportGemTypes", &controls[1]),
            ("sortGemsByDPSField", &controls[2]),
        ] {
            put_text(&mut fields, name, Some(value.clone()));
        }
        put_number(&mut fields, "defaultGemQuality", Some(quality));
        fields.insert("sortGemsByDPS".into(), SkillValue::Boolean(flags[0]));
        fields.insert("showLegacyGems".into(), SkillValue::Boolean(flags[1]));
        self.charge_fields(&fields)?;
        let ci = self.report.containers.len();
        self.report.containers.push(PreparedSkillContainer {
            source: self.source(node)?,
            fields,
            order: vec![],
            active_set_id: None,
        });
        let mut keys = NumericSetKeys::new(32768);
        let mut sets: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
        let mut order: Vec<f64> = Vec::new();
        for child in self.children(node)?.into_iter().flatten() {
            if !self.known(child)? {
                return Ok(());
            }
            match child.tag_name().name() {
                "Skill" => {
                    if order.is_empty() {
                        keys.insert(1.0).map_err(|_| resource("skill set keys"))?;
                        sets.insert(key(1.0), vec![]);
                        order.push(1.0);
                        self.report.containers[ci].order.push(SkillNumber(1.0));
                    }
                    if let Some(index) = self.load_group(child)? {
                        if self.report.failure.is_some() {
                            return Ok(());
                        }
                        let Some(set) = sets.get_mut(&key(1.0)) else {
                            self.fail(
                                SkillFailureKind::SourceRuntime,
                                "load_skill",
                                Some(child),
                                "attempt to append legacy group to missing skill set 1",
                            )?;
                            return Ok(());
                        };
                        set.push(index);
                        self.report.groups[index].attached = true;
                    }
                }
                "SkillSet" => {
                    let id = self
                        .number(child, "id")?
                        .unwrap_or_else(|| keys.sequence_length() as f64 + 1.0);
                    if id.is_nan() {
                        self.fail(
                            SkillFailureKind::SourceRuntime,
                            "create_skill_set",
                            Some(child),
                            "table index is NaN",
                        )?;
                        return Ok(());
                    }
                    keys.insert(id).map_err(|_| resource("skill set keys"))?;
                    sets.insert(key(id), vec![]);
                    order.push(id);
                    self.report.containers[ci].order.push(SkillNumber(id));
                    for group in self.children(child)?.into_iter().flatten() {
                        if let Some(index) = self.load_group(group)? {
                            if self.report.failure.is_some() {
                                return Ok(());
                            }
                            sets.get_mut(&key(id))
                                .expect("created skill set")
                                .push(index);
                            self.report.groups[index].attached = true;
                        }
                    }
                }
                _ => {}
            }
        }
        if order.is_empty() {
            sets.insert(key(1.0), vec![]);
            order.push(1.0);
            self.report.containers[ci].order.push(SkillNumber(1.0));
        }
        let desired = self.number(node, "activeSkillSet")?.unwrap_or(1.0);
        let chosen = if !desired.is_nan() && sets.contains_key(&key(desired)) {
            desired
        } else {
            order[0]
        };
        self.report.containers[ci].active_set_id = Some(SkillNumber(chosen));
        if let Some(&index) = sets.get(&key(chosen)).and_then(|v| v.first()) {
            self.process_group(index)?;
        }
        Ok(())
    }
}

pub fn prepare_authored_skills(
    build: &ImportedBuildInstance,
    view: &SelectedView<'_>,
    data: &Arc<CompiledGameData>,
    limits: SkillPreparationLimits,
) -> Result<PreparedSkills, EvaluationError> {
    view.validate_binding(build, data.snapshot())
        .map_err(|e| contract(e.to_string()))?;
    let binding = view_digest(view)?;
    let document = roxmltree::Document::parse_with_options(
        build.source_xml(),
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 100_000,
            ..Default::default()
        },
    )
    .map_err(|e| contract(e.to_string()))?;
    let mut machine = Machine {
        build,
        data,
        report: SkillPreparationReport {
            schema_version: SKILL_PREPARATION_SCHEMA,
            scope: "authored_skill_loading",
            source_sha256: build.source_sha256().into(),
            data: data.identity().clone(),
            view_sha256: binding,
            status: SkillPreparationStatus::Complete,
            containers: vec![],
            groups: vec![],
            selected_groups: vec![],
            failure: None,
            effect_cost_overrides: vec![],
            frontiers: vec![
                "equipment/passive grants and triggered entries require provider execution; LoadSkill does not import a triggered attribute",
                "effective support applicability, actor/action construction and numerical calculation remain separate stages",
            ],
        },
        sources: build
            .occurrences()
            .iter()
            .map(|s| (s.range().start, s.id()))
            .collect(),
        instances: build
            .instances()
            .iter()
            .map(|b| (b.source(), b.instance()))
            .collect(),
        limits,
        pattern: MatchBudget::new(MatchLimits {
            max_steps: limits.max_pattern_steps,
            ..MatchLimits::default()
        }),
        cost_overrides: BTreeMap::new(),
    };
    let catalog = data.snapshot().skill_preparation().data();
    let mut controls = [
        catalog
            .default_gem_level_options
            .first()
            .cloned()
            .ok_or_else(|| contract("empty default-level choices"))?,
        catalog
            .support_type_options
            .first()
            .cloned()
            .ok_or_else(|| contract("empty support-type choices"))?,
        catalog
            .sort_field_options
            .first()
            .cloned()
            .ok_or_else(|| contract("empty sort-field choices"))?,
    ];
    let mut flags = [true, false];
    for node in machine
        .children(document.root_element())?
        .into_iter()
        .flatten()
        .filter(|node| node.tag_name().name() == "Skills")
    {
        machine.load_container(node, &mut controls, &mut flags)?;
        if machine.report.failure.is_some() {
            break;
        }
    }
    if machine.report.failure.is_none()
        && let Some(selected) = &view.report().skills.selected
    {
        for &member in &selected.members {
            let AuthoredInstanceId::SkillGroup(id) = member else {
                return Err(contract("selected skill member is not a group"));
            };
            if !machine
                .report
                .groups
                .iter()
                .any(|group| group.instance == id && group.attached)
            {
                return Err(contract(
                    "selected group was not attached by authored loading",
                ));
            }
            machine.report.selected_groups.push(id);
        }
        if view.report().skills.override_instance.is_some()
            && let Some(id) = machine.report.selected_groups.first()
            && let Some(index) = machine
                .report
                .groups
                .iter()
                .position(|group| &group.instance == id)
        {
            machine.process_group(index)?;
        }
    }
    machine.report.effect_cost_overrides = machine.cost_overrides.values().cloned().collect();
    Ok(PreparedSkills {
        owner: build.clone(),
        data: Arc::clone(data),
        report: machine.report,
        cost_overrides: machine.cost_overrides,
    })
}

#[cfg(test)]
mod overlay_tests {
    use super::*;
    use poe_optimizer_core::{build_identity::BuildLineage, build_view::ViewRequest};
    use poe_optimizer_data::game_data::{GameDataLoader, LoadLimits, TrustPolicy};
    use poe_optimizer_import::{
        build_instance::InstanceImportLimits,
        decode_build,
        selected_view::{ResolveLimits, resolve_view},
    };

    #[test]
    fn triggered_runtime_cost_replacement_preserves_aliases_and_build_isolation() {
        // Provider-seeded runtime state deliberately bypasses LoadSkill, which
        // never imports a triggered attribute from authored XML.
        let original = CompiledGameData::bundled().unwrap();
        let mut package = original.snapshot().package().clone();
        let definition = package
            .skill_preparation
            .effects
            .iter_mut()
            .find(|effect| {
                effect.levels.len() >= 2
                    && effect.levels[0]
                        .cost
                        .as_ref()
                        .is_some_and(|cost| !cost.is_empty())
                    && !package
                        .skill_preparation
                        .string_gem_for_skill
                        .contains_key(&effect.id)
            })
            .unwrap();
        let primary = definition.levels[0].clone();
        let alias_key = definition.levels[1].key;
        definition.levels[1] = primary.clone();
        definition.levels[1].key = alias_key;
        let effect_id = definition.id.clone();
        package.refresh_section_digests().unwrap();
        let bytes = package.canonical_bytes().unwrap();
        let snapshot =
            GameDataLoader::from_bytes(&bytes, &TrustPolicy::AllowCustom, &LoadLimits::default())
                .unwrap();
        let data = Arc::new(CompiledGameData::compile(Arc::new(snapshot)).unwrap());
        let xml = b"<PathOfBuilding2><Skills><Skill><Gem skillId='SparkPlayer' level='1'/></Skill></Skills></PathOfBuilding2>";
        let build = ImportedBuildInstance::from_decoded(
            decode_build(xml).unwrap(),
            BuildLineage::from_bytes([98; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let view = resolve_view(
            &build,
            data.snapshot(),
            &ViewRequest::default(),
            ResolveLimits::default(),
        )
        .unwrap();
        let fresh =
            prepare_authored_skills(&build, &view, &data, SkillPreparationLimits::default())
                .unwrap();
        let limits = SkillPreparationLimits::default();
        let mut machine = Machine {
            build: &build,
            data: &data,
            report: fresh.report().clone(),
            sources: BTreeMap::new(),
            instances: BTreeMap::new(),
            limits,
            pattern: MatchBudget::new(MatchLimits {
                max_steps: limits.max_pattern_steps,
                ..MatchLimits::default()
            }),
            cost_overrides: BTreeMap::new(),
        };
        let mut gem = fresh.report().groups[0].gems[0].clone();
        gem.fields.remove("gemId");
        put_text(&mut gem.fields, "skillId", Some(effect_id.clone()));
        put_number(&mut gem.fields, "level", Some(primary.key));
        gem.fields
            .insert("triggered".into(), SkillValue::Boolean(true));
        machine.process_gem(&mut gem).unwrap();
        assert_eq!(machine.report.status, SkillPreparationStatus::Complete);
        assert!(gem.processed);
        assert_eq!(machine.cost_overrides.len(), 1);
        put_number(&mut gem.fields, "level", Some(alias_key));
        machine.process_gem(&mut gem).unwrap();
        assert_eq!(machine.cost_overrides.len(), 1);
        assert_eq!(
            machine.cost_overrides[&primary.row_id].level.value(),
            alias_key
        );
        let changed = PreparedSkills {
            owner: build.clone(),
            data: Arc::clone(&data),
            report: machine.report,
            cost_overrides: machine.cost_overrides,
        };
        assert!(changed.has_cost_override(&effect_id, primary.key));
        assert!(changed.has_cost_override(&effect_id, alias_key));
        assert!(!fresh.has_cost_override(&effect_id, primary.key));
        assert!(!fresh.has_cost_override(&effect_id, alias_key));
        let unchanged = data
            .snapshot()
            .skill_preparation()
            .effect(&effect_id)
            .unwrap();
        assert_eq!(unchanged.level(primary.key).unwrap().cost, primary.cost);
        assert_eq!(unchanged.level(alias_key).unwrap().cost, primary.cost);
    }
}
