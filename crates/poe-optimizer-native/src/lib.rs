//! Native build evaluation: owned Rust inputs and calculations, with an injectable clock.
//! No PoB checkout, Lua state, subprocess, filesystem or network access is required.
#![forbid(unsafe_code)]
mod profile;

use poe_optimizer_core::{BuildSummary, coverage::*, evaluation::*, metrics::*, options::*};
use poe_optimizer_engine::{mace, spark};
use profile::NativeInput;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, time::Duration};

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
/// It caches parsing only: every evaluation recomputes the complete supported pipeline.
pub struct PreparedEvaluation {
    request: EvaluationRequest,
    profile: profile::Profile,
}
impl PreparedEvaluation {
    pub fn request(&self) -> &EvaluationRequest {
        &self.request
    }
    /// Pure calculation entry point for native/browser hosts. No time or OS calls.
    pub fn calculate(&self) -> Result<NativeCalculation, EvaluationError> {
        match &self.profile.input {
            NativeInput::Spark(input) => spark::evaluate(input)
                .map(NativeCalculation::Spark)
                .map_err(|e| {
                    EvaluationError::new(EvaluationErrorKind::CalculationFailed, e.to_string())
                }),
            NativeInput::Mace(input) => {
                mace::evaluate(input)
                    .map(NativeCalculation::Mace)
                    .map_err(|e| {
                        EvaluationError::new(EvaluationErrorKind::CalculationFailed, e.to_string())
                    })
            }
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
                ]
            }};
        }
        let (raw,average)=match self {
            Self::Spark(o)=>(resources!(o),MeasurementValue::from_number(o.average_hit)),
            Self::Mace(o)=>(resources!(o),MeasurementValue::Unavailable{reason:"Attack AverageHit is stored per hand; the current metric contract does not aggregate hands.".into()}),
        };
        let mut values: BTreeMap<_, _> = raw
            .into_iter()
            .map(|(key, value)| (key, MeasurementValue::from_number(value)))
            .collect();
        values.insert("selected_average_hit", average);
        values
    }
    fn diagnostic(&self, input: &NativeInput) -> serde_json::Value {
        let mut value = match self {
            Self::Spark(o) => {
                serde_json::json!({"strength":o.strength,"dexterity":o.dexterity,"intelligence":o.intelligence,"cast_rate":o.cast_rate,"crit_chance":o.crit_chance,"crit_multiplier":o.crit_multiplier,"effective_enemy_lightning_resistance":o.effective_enemy_lightning_resistance})
            }
            Self::Mace(o) => {
                serde_json::json!({"strength":o.strength,"dexterity":o.dexterity,"intelligence":o.intelligence,"attack_rate":o.attack_rate,"accuracy":o.accuracy,"hit_chance":o.hit_chance,"crit_chance":o.crit_chance,"crit_multiplier":o.crit_multiplier,"main_hand_average_hit":o.main_hand_average_hit,"average_damage":o.average_damage,"effective_enemy_fire_resistance":o.effective_enemy_fire_resistance,"effective_enemy_evasion":o.effective_enemy_evasion})
            }
        };
        if let NativeInput::Mace(i) = input {
            value["weapon_base"] = serde_json::json!(i.weapon.data().name);
            value["weapon_quality"] = serde_json::json!(i.quality);
            value["weapon_item_level"] = serde_json::json!(i.item_level);
            value["brutality_i"] = serde_json::json!(i.brutality);
            value["resolved_enemy_armour"] = serde_json::json!(i.enemy_armour);
            value["resolved_enemy_evasion"] = serde_json::json!(i.enemy_evasion);
        }
        value["profile"] = serde_json::json!(self.profile_id());
        value["supports_prepared_inputs"] = serde_json::json!(true);
        value["calculation_result_cached"] = serde_json::json!(false);
        value
    }
}
struct BuildInfo {
    level: u32,
    class_name: &'static str,
    root: u32,
    skill_id: &'static str,
    skill_name: &'static str,
    game_id: &'static str,
    variant_id: &'static str,
    brutality: bool,
}
impl BuildInfo {
    fn of(input: &NativeInput) -> Self {
        match input {
            NativeInput::Spark(i) => Self {
                level: i.character_level,
                class_name: "Sorceress",
                root: 54447,
                skill_id: "SparkPlayer",
                skill_name: "Spark",
                game_id: "Metadata/Items/Gems/SkillGemSpark",
                variant_id: "Spark",
                brutality: false,
            },
            NativeInput::Mace(i) => Self {
                level: i.character_level,
                class_name: "Warrior",
                root: 47175,
                skill_id: "Melee1HMacePlayer",
                skill_name: "Mace Strike",
                game_id: "Metadata/Items/Gem/SkillGemPlayerDefault1HMace",
                variant_id: "PlayerDefault1HMace",
                brutality: i.brutality,
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
    ]
    .into_iter()
    .map(|(id, unit)| MetricDefinition {
        id: id.into(),
        unit,
        actors: vec![ActorScope::Player],
        description: format!("{id}, calculated within the declared native profile."),
        schema_version: 1,
    })
    .collect()
}
pub fn backend_identity() -> BackendIdentity {
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
                include_str!("../Cargo.toml"),
                include_str!("../../poe-optimizer-engine/src/spark.rs"),
                include_str!("../../poe-optimizer-engine/src/mace.rs"),
                include_str!("../../poe-optimizer-engine/src/defence.rs"),
                include_str!("../../poe-optimizer-engine/Cargo.toml"),
                include_str!("../../poe-optimizer-import/src/lib.rs"),
                include_str!("../../poe-optimizer-import/src/xml_compat.rs"),
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
            BackendIdentity {
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
        Self { clock }
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
            profile: profile::parse(request)?,
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
        let info = BuildInfo::of(&prepared.profile.input);
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
        for name in profile::QUEST_KEYS {
            if !prepared.profile.config.contains_key(name) {
                defaults.insert(name.into(), Scalar::Boolean(true));
            }
        }
        if !prepared.profile.config.contains_key("resistancePenalty") {
            defaults.insert("resistancePenalty".into(), Scalar::Number(-60.0));
        }
        let mut result=EvaluationResult {
            backend:backend_identity(),
            build:BuildSummary{level:info.level,class_name:info.class_name.into(),ascendancy_name:"None".into(),tree_version:"0_5".into(),main_socket_group:1,allocated_nodes:vec![info.root],skill_groups:1},
            context:EvaluationContext{requested:prepared.request.options.clone(),calculation_mode:"MAIN".into(),enemy_level:prepared.profile.enemy_level,config_inputs:prepared.profile.config.clone(),config_placeholders:defaults,player_conditions:BTreeMap::new(),enemy_conditions:BTreeMap::new()},
            coverage:coverage(prepared.profile.group_label.clone(), &info),measurements,
            exports:vec![BuildDocument{format:BuildFormat::PathOfBuilding2Xml,content:prepared.profile.export_xml.clone()}],
            warnings:vec![format!("Native supported profile: {}. Other build mechanics are rejected.",output.profile_id()),"Full DPS rollups, EHP and maximum-hit calculations are not implemented by this backend.".into()],
            elapsed_ms:0.0,diagnostic_only:true,
            attachments:vec![DiagnosticAttachment{media_type:"application/vnd.poe-optimizer.native-profile+json;version=1".into(),content:output.diagnostic(&prepared.profile.input).to_string()}],
        };
        result.elapsed_ms = self.elapsed(start, budget)?;
        result.validate_recorded()?;
        result.elapsed_ms = self.elapsed(start, budget)?;
        Ok(result)
    }
}
impl<C: EvaluationClock> CalculationBackend for NativeBackend<C> {
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
    if info.brutality {
        gems.push(gem_coverage(
            2,
            "Brutality I",
            "SupportBrutalityPlayer",
            "Metadata/Items/Gems/SupportGemBrutality",
            "BrutalitySupport",
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
