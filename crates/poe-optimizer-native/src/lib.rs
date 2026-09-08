//! Native build evaluation: owned Rust inputs and calculations, with an injectable clock.
//! No PoB checkout, Lua state, subprocess, filesystem or network access is required.
#![forbid(unsafe_code)]
mod build_candidates;
mod candidates;
pub use build_candidates::{PreparedBuildCandidates, PreparedBuildFootprint};
mod profile;
pub use candidates::{
    NativeMetricSnapshot, NativeMetricValue, PreparedMaceCandidates, PreparedMaceFootprint,
};
mod tree;

use poe_optimizer_core::{BuildSummary, coverage::*, evaluation::*, metrics::*, options::*};
pub use poe_optimizer_engine::CompiledGameData;
pub use poe_optimizer_engine::actor::ActorScratch;
use poe_optimizer_engine::{mace, spark};
use profile::NativeInput;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc, time::Duration};

/// Hosts supply a monotonic clock; browser bindings can use performance.now().
pub trait EvaluationClock: Send + Sync {
    fn now(&self) -> Duration;
}
/// Native-process monotonic clock; unavailable in browser builds without a host clock.
pub struct HostClock;
#[cfg(not(target_arch = "wasm32"))]
impl EvaluationClock for HostClock {
    fn now(&self) -> Duration {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        START.get_or_init(std::time::Instant::now).elapsed()
    }
}

pub struct NativeBackend<C = HostClock> {
    clock: C,
    data: Arc<CompiledGameData>,
    identity: BackendIdentity,
}
#[cfg(not(target_arch = "wasm32"))]
impl NativeBackend<HostClock> {
    pub fn new() -> Self {
        Self::with_clock(HostClock)
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl Default for NativeBackend<HostClock> {
    fn default() -> Self {
        Self::new()
    }
}

/// Validated immutable source and resolved inputs, reusable across independent calculations.
/// Shared actor/weapon components are prepared once; every evaluation computes fresh skill outputs.
pub struct PreparedEvaluation {
    data: Arc<CompiledGameData>,
    identity: BackendIdentity,
    request: EvaluationRequest,
    profile: profile::Profile,
}
impl PreparedEvaluation {
    pub fn data_identity(&self) -> &poe_optimizer_core::data::DataIdentity {
        self.data.identity()
    }
    pub fn request(&self) -> &EvaluationRequest {
        &self.request
    }
    /// Pure calculation entry point for native/browser hosts. No time or OS calls.
    pub fn calculate(&self) -> Result<NativeCalculation, EvaluationError> {
        match &self.profile.input {
            NativeInput::Spark(input) => spark::evaluate_with_actor(
                input,
                &self.profile.tree.character,
                &self.data,
                &self.profile.prepared_actor,
            )
            .map(NativeCalculation::Spark)
            .map_err(|e| {
                EvaluationError::new(EvaluationErrorKind::CalculationFailed, e.to_string())
            }),
            NativeInput::Mace(input) => mace::evaluate_with_actor(
                input,
                &self.profile.tree.character,
                &self.data,
                self.profile
                    .prepared_weapon
                    .as_ref()
                    .expect("prepared Mace weapon"),
                self.profile
                    .prepared_supports
                    .as_ref()
                    .expect("prepared Mace support loadout"),
                &self.profile.prepared_actor,
            )
            .map(NativeCalculation::Mace)
            .map_err(|e| {
                EvaluationError::new(EvaluationErrorKind::CalculationFailed, e.to_string())
            }),
        }
    }
}
/// Complete computed output for one supported profile; no host or Lua values.
#[derive(Debug, Clone, Copy)]
pub enum NativeCalculation {
    Spark(spark::SparkOutput),
    Mace(mace::MaceOutput),
}
impl NativeCalculation {
    pub fn profile_id(&self) -> &'static str {
        match self {
            Self::Spark(_) => spark::PROFILE_ID,
            Self::Mace(_) => mace::PROFILE_ID,
        }
    }
    fn values(&self) -> BTreeMap<&'static str, MeasurementValue> {
        macro_rules! resources {
            ($o:expr) => {{
                let o = $o;
                [
                    ("life", o.life),
                    ("mana", o.mana),
                    ("energy_shield", o.energy_shield),
                    ("fire_resistance_capped_pct", o.fire_resistance),
                    ("cold_resistance_capped_pct", o.cold_resistance),
                    ("lightning_resistance_capped_pct", o.lightning_resistance),
                    ("chaos_resistance_capped_pct", o.chaos_resistance),
                    ("selected_hit_dps", o.hit_dps),
                    ("spirit", o.spirit),
                    ("armour", o.armour),
                    ("evasion", o.evasion),
                    ("movement_speed_pct", 100.0 * o.effective_movement_speed_mod),
                    ("action_speed_pct", 100.0 * o.action_speed_mod),
                ]
            }};
        }
        let (raw, average) = match self {
            Self::Spark(o) => (resources!(o), MeasurementValue::from_number(o.average_hit)),
            Self::Mace(o) => (
                resources!(o),
                MeasurementValue::Unavailable {
                    reason: candidates::MACE_AVERAGE_REASON.into(),
                },
            ),
        };
        let mut values: BTreeMap<_, _> = raw
            .into_iter()
            .map(|(key, value)| (key, MeasurementValue::from_number(value)))
            .collect();
        values.insert("selected_average_hit", average);
        values
    }
    fn diagnostic(&self, profile: &profile::Profile, data: &CompiledGameData) -> serde_json::Value {
        let mut value = match self {
            Self::Spark(o) => {
                serde_json::json!({"strength":o.strength,"dexterity":o.dexterity,"intelligence":o.intelligence,"armour":o.armour,"evasion":o.evasion,"cast_rate":o.cast_rate,"crit_chance":o.crit_chance,"crit_multiplier":o.crit_multiplier,"effective_enemy_lightning_resistance":o.effective_enemy_lightning_resistance})
            }
            Self::Mace(o) => {
                serde_json::json!({"strength":o.strength,"dexterity":o.dexterity,"intelligence":o.intelligence,"armour":o.armour,"evasion":o.evasion,"attack_rate":o.attack_rate,"accuracy":o.accuracy,"hit_chance":o.hit_chance,"crit_chance":o.crit_chance,"crit_multiplier":o.crit_multiplier,"main_hand_average_hit":o.main_hand_average_hit,"average_damage":o.average_damage,"effective_enemy_fire_resistance":o.effective_enemy_fire_resistance,"effective_enemy_evasion":o.effective_enemy_evasion})
            }
        };
        value["equipment"] = serde_json::json!(
            profile
                .equipment
                .iter()
                .map(|(slot, item)| (slot, item.diagnostic()))
                .collect::<BTreeMap<_, _>>()
        );
        if let NativeInput::Mace(i) = &profile.input {
            let record = profile
                .weapon_record
                .as_ref()
                .expect("admitted Mace weapon");
            let prepared = profile
                .prepared_weapon
                .as_ref()
                .expect("prepared Mace weapon");
            let weapon = prepared.stats();
            value["weapon_item"] = record.diagnostic();
            value["weapon_stats"] = serde_json::json!({
                "physical_minimum":weapon.physical_minimum,"physical_maximum":weapon.physical_maximum,
                "fire_minimum":weapon.fire_minimum,"fire_maximum":weapon.fire_maximum,
                "physical_present":weapon.physical_present,"fire_present":weapon.fire_present,
                "attack_speed_increased":weapon.attack_speed_increased,"attack_rate":weapon.attack_rate,
                "critical_chance":weapon.critical_chance,
                "consumed_modifier_count":prepared.consumed_modifier_count(),
                "modifier_roll_count":prepared.modifier_roll_count(),
            });
            value["weapon_base"] = serde_json::json!(data.weapon(i.weapon).name);
            value["weapon_quality"] = serde_json::json!(i.quality);
            value["weapon_item_level"] = serde_json::json!(i.item_level);
            value["support_loadout"] = serde_json::json!(profile.support_keys);
            value["configured_supports"] = serde_json::json!(
                profile
                    .support_keys
                    .iter()
                    .map(|key| data
                        .snapshot()
                        .package()
                        .support(key)
                        .expect("validated support"))
                    .collect::<Vec<_>>()
            );
            value["resolved_enemy_armour"] = serde_json::json!(i.enemy_armour);
            value["resolved_enemy_evasion"] = serde_json::json!(i.enemy_evasion);
        }
        value["local_armour"] = poe_optimizer_import::actor_assembly::local_armour_evidence(
            profile
                .prepared_armour
                .iter()
                .map(|(slot, armour)| (slot.as_str(), &profile.equipment[slot], armour)),
        );
        value["receiving_defence"] =
            poe_optimizer_import::actor_assembly::receiving_defence_evidence(
                profile
                    .prepared_actor
                    .receiving()
                    .expect("complete native actor preparation"),
            );
        value["movement"] = poe_optimizer_import::actor_assembly::movement_evidence(
            profile.prepared_actor.movement(),
        );
        value["action_speed"] = poe_optimizer_import::actor_assembly::action_speed_evidence(
            profile.prepared_actor.action_speed(),
        );
        value["action_timing"] =
            poe_optimizer_import::actor_assembly::action_timing_evidence(match self {
                Self::Spark(output) => output.timing,
                Self::Mace(output) => output.timing,
            });
        value["actor_modifiers"] = profile.actor_modifiers.diagnostic();
        let actor = profile.prepared_actor.values();
        value["actor_resources"] = serde_json::json!({
            "strength":actor.attributes.strength,"dexterity":actor.attributes.dexterity,
            "intelligence":actor.attributes.intelligence,"life":actor.life,"mana":actor.mana,
            "spirit":actor.spirit,"accuracy":actor.accuracy,
            "lowest_of_maximum_life_and_maximum_mana":actor.lowest_of_maximum_life_and_maximum_mana,
            "lowest_attribute":actor.lowest_attribute,"total_attributes":actor.total_attributes,
            "low_life_percentage":actor.low_life_percentage,"full_life_percentage":actor.full_life_percentage,
            "life_has_override":actor.life_has_override,"mana_has_override":actor.mana_has_override,
            "spirit_has_override":actor.spirit_has_override,"chaos_inoculation":actor.chaos_inoculation,
            "full_life_from_chaos_inoculation":actor.full_life_from_chaos_inoculation,
        });
        value["profile"] = serde_json::json!(self.profile_id());
        value["supports_prepared_inputs"] = serde_json::json!(true);
        value["calculation_result_cached"] = serde_json::json!(false);
        value
    }
}
struct BuildInfo<'a> {
    level: u32,
    skill_id: &'a str,
    skill_name: &'a str,
    game_id: &'a str,
    variant_id: &'a str,
    supports: Vec<&'a poe_optimizer_data::game_data::SupportData>,
}
impl<'a> BuildInfo<'a> {
    fn of(profile: &profile::Profile, data: &'a CompiledGameData) -> Self {
        let package = data.snapshot().package();
        match &profile.input {
            NativeInput::Spark(i) => Self {
                level: i.character_level,
                skill_id: &package.spark.skill_id,
                skill_name: &package.spark.name,
                game_id: &package.spark.game_id,
                variant_id: &package.spark.variant_id,
                supports: Vec::new(),
            },
            NativeInput::Mace(i) => Self {
                level: i.character_level,
                skill_id: &package.mace.skill_id,
                skill_name: &package.mace.name,
                game_id: &package.mace.game_id,
                variant_id: &package.mace.variant_id,
                supports: profile
                    .support_order
                    .iter()
                    .map(|key| package.support(key).expect("validated support"))
                    .collect(),
            },
        }
    }
}

pub fn metric_catalog() -> Vec<MetricDefinition> {
    use MetricUnit::*;
    [
        ("life", PoolPoints),
        ("mana", PoolPoints),
        ("energy_shield", PoolPoints),
        ("fire_resistance_capped_pct", Percent),
        ("cold_resistance_capped_pct", Percent),
        ("lightning_resistance_capped_pct", Percent),
        ("chaos_resistance_capped_pct", Percent),
        ("selected_average_hit", Damage),
        ("selected_hit_dps", DamagePerSecond),
        ("spirit", PoolPoints),
        ("armour", RatingPoints),
        ("evasion", RatingPoints),
        ("movement_speed_pct", Percent),
        ("action_speed_pct", Percent),
    ]
    .into_iter()
    .map(|(id, unit)| MetricDefinition {
        id: id.into(),
        unit,
        actors: vec![ActorScope::Player],
        description: match id {
            "armour" => "Player Armour rating; not physical damage mitigation.".into(),
            "evasion" => "Player Evasion rating; not chance to evade.".into(),
            "action_speed_pct" => "Effective player action speed relative to baseline; 100 is baseline, not increased attack or cast speed.".into(),
            "movement_speed_pct" => "Effective player movement speed relative to baseline; 100 is baseline, not increased or absolute speed.".into(),
            _ => format!("{id}, calculated within the declared native profile."),
        },
        schema_version: 1,
    })
    .collect()
}
/// Identity of the reviewed default package. Instance users must call `NativeBackend::identity`.
pub fn backend_identity() -> BackendIdentity {
    identity_for(&CompiledGameData::bundled().expect("invalid packaged native data"))
}
fn identity_for(data: &CompiledGameData) -> BackendIdentity {
    let mut identity = implementation_identity();
    identity.data = Some(data.identity().clone());
    identity
}
fn implementation_identity() -> BackendIdentity {
    static IDENTITY: std::sync::OnceLock<BackendIdentity> = std::sync::OnceLock::new();
    IDENTITY
        .get_or_init(|| {
            let sources: BTreeMap<_, _> = spark::SOURCE_FILES
                .iter()
                .chain(mace::SOURCE_FILES.iter())
                .map(|s| (s.path, s.sha256))
                .collect();
            let mut adapter = Sha256::new();
            for text in [
                include_str!("lib.rs"),
                include_str!("profile.rs"),
                include_str!("tree.rs"),
                include_str!("candidates.rs"),
                include_str!("build_candidates.rs"),
                include_str!("../../poe-optimizer-import/src/equipment.rs"),
                include_str!("../../poe-optimizer-import/src/controlled_build.rs"),
                include_str!("../../poe-optimizer-import/src/controlled_build_template.rs"),
                include_str!("../../poe-optimizer-import/src/controlled_build_xml.rs"),
                include_str!("../../poe-optimizer-import/src/controlled_build_evidence.rs"),
                include_str!("../../poe-optimizer-engine/src/actor_program.rs"),
                include_str!("../../poe-optimizer-import/src/controlled_mace.rs"),
                include_str!("../../poe-optimizer-import/src/mace_item.rs"),
                include_str!("../../poe-optimizer-import/src/item_formatting.rs"),
                include_str!("../../poe-optimizer-import/src/modifier_syntax.rs"),
                include_str!("../../poe-optimizer-import/src/actor_modifiers.rs"),
                include_str!("../../poe-optimizer-import/src/actor_assembly.rs"),
                include_str!("../../poe-optimizer-engine/src/actor.rs"),
                include_str!("../../poe-optimizer-engine/src/actor_receiving.rs"),
                include_str!("../../poe-optimizer-engine/src/armour.rs"),
                include_str!("../../poe-optimizer-engine/src/item_format.rs"),
                include_str!("../../poe-optimizer-engine/src/movement.rs"),
                include_str!("../../poe-optimizer-engine/src/action_speed.rs"),
                include_str!("../../poe-optimizer-engine/src/timing.rs"),
                include_str!("../../poe-optimizer-engine/src/multipliers.rs"),
                include_str!("../../poe-optimizer-engine/src/weapon.rs"),
                include_str!("../../poe-optimizer-engine/src/offence.rs"),
                include_str!("../../poe-optimizer-engine/src/character.rs"),
                include_str!("../../poe-optimizer-engine/src/data.rs"),
                include_str!("../../poe-optimizer-core/src/data.rs"),
                include_str!("../Cargo.toml"),
                include_str!("../../poe-optimizer-engine/src/spark.rs"),
                include_str!("../../poe-optimizer-engine/src/mace.rs"),
                include_str!("../../poe-optimizer-engine/src/mace_supports.rs"),
                include_str!("../../poe-optimizer-engine/src/modifiers.rs"),
                include_str!("../../poe-optimizer-engine/src/conditions.rs"),
                include_str!("../../poe-optimizer-engine/src/defence.rs"),
                include_str!("../../poe-optimizer-engine/src/resistance.rs"),
                include_str!("../../poe-optimizer-engine/Cargo.toml"),
                include_str!("../../poe-optimizer-import/src/lib.rs"),
                include_str!("../../poe-optimizer-import/src/xml_compat.rs"),
                include_str!("../../poe-optimizer-import/src/source_xml.rs"),
                include_str!("../../poe-optimizer-import/src/configuration.rs"),
                include_str!("../../poe-optimizer-import/Cargo.toml"),
                include_str!("../../poe-optimizer-core/src/lib.rs"),
                include_str!("../../poe-optimizer-core/src/evaluation.rs"),
                include_str!("../../poe-optimizer-core/src/options.rs"),
                include_str!("../../poe-optimizer-core/src/coverage.rs"),
                include_str!("../../poe-optimizer-core/src/metrics.rs"),
                include_str!("../../poe-optimizer-core/Cargo.toml"),
                include_str!("../../../Cargo.lock"),
            ] {
                adapter.update(text.replace("\r\n", "\n"));
            }
            adapter.update(poe_optimizer_data::implementation_fingerprint());
            BackendIdentity {
                data: None,
                id: "native-poe2".into(),
                implementation_version: env!("CARGO_PKG_VERSION").into(),
                rules_revision: poe_optimizer_engine::UPSTREAM_REVISION.into(),
                source_fingerprint: format!(
                    "{:x}",
                    Sha256::digest(serde_json::to_vec(&sources).unwrap())
                ),
                adapter_fingerprint: format!("{:x}", adapter.finalize()),
            }
        })
        .clone()
}
impl<C: EvaluationClock> NativeBackend<C> {
    pub fn with_clock(clock: C) -> Self {
        Self::with_data(
            CompiledGameData::bundled().expect("invalid packaged native data"),
            clock,
        )
        .expect("invalid native backend data")
    }
    /// Explicit data injection; no loading or acquisition occurs during calculation.
    pub fn with_data(data: Arc<CompiledGameData>, clock: C) -> Result<Self, EvaluationError> {
        let identity = identity_for(&data);
        Ok(Self {
            clock,
            data,
            identity,
        })
    }
    pub fn identity(&self) -> BackendIdentity {
        self.identity.clone()
    }
    pub fn data(&self) -> &Arc<CompiledGameData> {
        &self.data
    }
    /// Parsing/validation can be moved outside a hot loop; no calculation result is cached.
    pub fn prepare(
        &self,
        request: &EvaluationRequest,
    ) -> Result<PreparedEvaluation, EvaluationError> {
        let known = metric_catalog();
        let mut queries = std::collections::BTreeSet::new();
        for query in &request.metrics {
            if !queries.insert(query) {
                return Err(EvaluationError::new(
                    EvaluationErrorKind::InvalidRequest,
                    "Duplicate metric query",
                ));
            }
            if !known
                .iter()
                .any(|m| m.id == query.id && m.actors.contains(&query.actor))
            {
                return Err(EvaluationError::new(
                    EvaluationErrorKind::UnsupportedCapability,
                    format!(
                        "Native backend does not implement {:?}.{}",
                        query.actor, query.id
                    ),
                ));
            }
        }
        Ok(PreparedEvaluation {
            profile: profile::parse(request, &self.data)?,
            data: Arc::clone(&self.data),
            identity: self.identity.clone(),
            request: request.clone(),
        })
    }
    fn elapsed(&self, start: Duration, budget: EvaluationBudget) -> Result<f64, EvaluationError> {
        let elapsed = self.clock.now().checked_sub(start).ok_or_else(|| {
            EvaluationError::new(
                EvaluationErrorKind::BackendContract,
                "Evaluation clock moved backwards",
            )
        })?;
        if elapsed >= Duration::from_millis(budget.timeout_ms) {
            return Err(EvaluationError::new(
                EvaluationErrorKind::Timeout,
                "Native evaluation deadline exceeded",
            ));
        }
        Ok(elapsed.as_secs_f64() * 1000.0)
    }
    /// Recalculates from immutable resolved inputs; safe to call concurrently through Rayon.
    pub fn evaluate_prepared(
        &self,
        prepared: &PreparedEvaluation,
        budget: EvaluationBudget,
    ) -> Result<EvaluationResult, EvaluationError> {
        let start = self.clock.now();
        self.finish(prepared, start, budget)
    }
    fn finish(
        &self,
        prepared: &PreparedEvaluation,
        start: Duration,
        budget: EvaluationBudget,
    ) -> Result<EvaluationResult, EvaluationError> {
        if prepared.identity != self.identity {
            return Err(EvaluationError::new(
                EvaluationErrorKind::BackendContract,
                "Prepared evaluation belongs to a different data or calculation identity",
            ));
        }
        if budget.timeout_ms == 0 {
            return Err(EvaluationError::new(
                EvaluationErrorKind::InvalidRequest,
                "Evaluation timeout must be positive",
            ));
        }
        self.elapsed(start, budget)?;
        let output = prepared.calculate()?;
        self.elapsed(start, budget)?;
        let values = output.values();
        let info = BuildInfo::of(&prepared.profile, &self.data);
        let metrics = metric_catalog().into_iter().filter(|m| {
            prepared.request.metrics.is_empty()
                || prepared.request.metrics.iter().any(|q| q.id == m.id)
        });
        let measurements = metrics
            .map(|m| MetricMeasurement {
                query: MetricQuery {
                    actor: ActorScope::Player,
                    id: m.id.clone(),
                },
                unit: m.unit,
                value: values[m.id.as_str()].clone(),
                schema_version: m.schema_version,
            })
            .collect();
        let mut defaults = BTreeMap::new();
        for (name, enabled) in self
            .data
            .snapshot()
            .package()
            .quests
            .config_keys
            .iter()
            .zip(self.data.snapshot().package().quests.default_enabled)
        {
            if !prepared.profile.config.contains_key(name) {
                defaults.insert(name.clone(), Scalar::Boolean(enabled));
            }
        }
        for quest in &self.data.snapshot().package().actor.spirit_quests {
            if !prepared.profile.config.contains_key(&quest.config_key) {
                defaults.insert(
                    quest.config_key.clone(),
                    Scalar::Boolean(quest.default_enabled),
                );
            }
        }
        if !prepared.profile.config.contains_key("resistancePenalty") {
            defaults.insert(
                "resistancePenalty".into(),
                Scalar::Number(
                    self.data
                        .snapshot()
                        .package()
                        .encounters
                        .default_resistance_penalty,
                ),
            );
        }
        let mut result=EvaluationResult {
            backend:self.identity.clone(),
            build:BuildSummary{level:info.level,class_name:prepared.profile.tree.class.name.clone(),ascendancy_name:prepared.profile.tree.ascendancy_name().into(),tree_version:self.data.snapshot().tree().source.tree_version.clone(),main_socket_group:1,allocated_nodes:prepared.profile.tree.allocated_nodes.clone(),skill_groups:1},
            context:EvaluationContext{requested:prepared.request.options.clone(),calculation_mode:"MAIN".into(),enemy_level:prepared.profile.enemy_level,config_inputs:prepared.profile.config.clone(),config_placeholders:defaults,player_conditions:prepared.profile.prepared_actor.values().conditions().map(|(name,value)|(name.to_owned(),value)).collect(),enemy_conditions:BTreeMap::new()},
            coverage:coverage(prepared.profile.group_label.clone(), &info),measurements,
            exports:vec![BuildDocument{format:BuildFormat::PathOfBuilding2Xml,content:prepared.profile.export_xml.clone()}],
            warnings:vec![format!("Native supported profile: {}. Other build mechanics are rejected.",output.profile_id()),"Full DPS rollups, EHP and maximum-hit calculations are not implemented by this backend.".into()],
            elapsed_ms:0.0,diagnostic_only:true,
            attachments:vec![DiagnosticAttachment{media_type:match &prepared.profile.input { NativeInput::Spark(_) => "application/vnd.poe-optimizer.native-profile+json;version=7", NativeInput::Mace(_) => "application/vnd.poe-optimizer.native-profile+json;version=9" }.into(),content:output.diagnostic(&prepared.profile, &self.data).to_string()}, DiagnosticAttachment{media_type:"application/vnd.poe-optimizer.native-tree+json;version=3".into(),content:prepared.profile.tree.diagnostic(&self.data).to_string()}],
        };
        result.attachments.push(DiagnosticAttachment {
            media_type: "application/vnd.poe-optimizer.game-data+json;version=1".into(),
            content: serde_json::json!({"identity":self.data.identity(),"trust":self.data.snapshot().trust()}).to_string(),
        });
        result.warnings.push(format!("Game data trust: {:?}; identity {}. Data selection does not certify build legality or parity.", self.data.snapshot().trust(), self.data.identity().content_sha256));
        result.elapsed_ms = self.elapsed(start, budget)?;
        result.validate_recorded()?;
        result.elapsed_ms = self.elapsed(start, budget)?;
        Ok(result)
    }
}
impl<C: EvaluationClock> CalculationBackend for NativeBackend<C> {
    fn identity(&self) -> Option<BackendIdentity> {
        Some(self.identity.clone())
    }
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            id: "native-poe2".into(),
            build_formats: vec![BuildFormat::PathOfBuilding2Xml],
            metrics: metric_catalog(),
            full_build_evaluation: true,
            skill_selection: true,
            encounter_overrides: true,
        }
    }
    fn calculate(
        &self,
        request: &EvaluationRequest,
        budget: EvaluationBudget,
    ) -> Result<EvaluationResult, EvaluationError> {
        let start = self.clock.now();
        if budget.timeout_ms == 0 {
            return Err(EvaluationError::new(
                EvaluationErrorKind::InvalidRequest,
                "Evaluation timeout must be positive",
            ));
        }
        let prepared = self.prepare(request)?;
        self.finish(&prepared, start, budget)
    }
}
fn coverage(label: Option<String>, info: &BuildInfo) -> BuildCoverage {
    let mut gems = vec![gem_coverage(
        1,
        info.skill_name,
        info.skill_id,
        info.game_id,
        info.variant_id,
        false,
    )];
    for (index, support) in info.supports.iter().enumerate() {
        gems.push(gem_coverage(
            index + 2,
            &support.name,
            &support.skill_id,
            &support.game_id,
            &support.variant_id,
            true,
        ));
    }
    BuildCoverage {
        schema_version: 1,
        active_skill_set_id: Some(1),
        passives: None,
        groups: vec![SkillGroupCoverage {
            index: 1,
            label,
            enabled: true,
            slot: None,
            provenance: SkillProvenance {
                kind: SkillOrigin::Manual,
                source: None,
                item_id: None,
                item_name: None,
                node_id: None,
            },
            include_in_full_dps: true,
            group_count: Some(1.0),
            main_active_skill: Some(1),
            gems,
        }],
        selected_player: Some(SelectedSkillContext {
            actor: SkillActor::Player,
            skill_id: Some(info.skill_id.into()),
            skill_name: Some(info.skill_name.into()),
            group_index: Some(1),
            gem_index: Some(1),
            actor_skill_index: Some(1),
            minion_id: None,
            part_index: None,
            part_name: None,
            stat_set_index: Some(1),
            stat_set_label: None,
            show_average: false,
            synthesized_default_attack: false,
        }),
        selected_minion: None,
        full_dps: FullDpsCoverage {
            included_group_count: 1,
            selected_group_included: true,
            active_skills: vec![FullDpsSkillCoverage {
                group_index: Some(1),
                gem_index: Some(1),
                skill_id: Some(info.skill_id.into()),
                skill_name: Some(info.skill_name.into()),
                included: true,
                count: Some(1.0),
                count_enabled: true,
            }],
            reported_contributions: vec![],
        },
        unresolved_entry_count: 0,
        tree_connections: vec![],
    }
}

fn gem_coverage(
    index: usize,
    name: &str,
    id: &str,
    game: &str,
    variant: &str,
    support: bool,
) -> GemCoverage {
    GemCoverage {
        index,
        name: Some(name.into()),
        gem_id: Some(game.into()),
        gem_game_id: Some(game.into()),
        variant_id: Some(variant.into()),
        skill_id: Some(id.into()),
        enabled: true,
        count: Some(1.0),
        level: Some(1.0),
        quality: Some(0.0),
        is_support: Some(support),
        resolution: SkillResolution::ResolvedGem,
        diagnostic: None,
        hint: ResolutionHint::None,
        related_candidates: vec![],
    }
}
