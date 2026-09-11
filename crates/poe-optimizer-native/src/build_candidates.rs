//! Typed complete candidate evaluation over lazy source components.
//! The prepared evaluator retains numerical inputs and a private ownership token;
//! transient source documents validate introduced support axes during preparation.
//! Repeated calculation retains no XML or authored-loader state.
use crate::profile::NativeInput;
use crate::{
    CompiledGameData, EvaluationClock, NativeBackend, NativeCalculation, NativeMetricSnapshot,
};
use poe_optimizer_core::{evaluation::*, metrics::*, options::EvaluationOptions};
use poe_optimizer_engine::{
    mace::{self, MaceWeapon},
    spark,
    weapon::PreparedWeaponStats,
};
use poe_optimizer_import::controlled_build::{
    AdmittedBuildSelection, ControlledBuildBinding, ControlledBuildCatalog,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

fn contract(message: impl Into<String>) -> EvaluationError {
    EvaluationError::new(EvaluationErrorKind::BackendContract, message)
}
fn preparation(message: impl Into<String>) -> EvaluationError {
    EvaluationError::new(EvaluationErrorKind::UnsupportedCapability, message)
}
fn calculation(message: impl Into<String>) -> EvaluationError {
    EvaluationError::new(EvaluationErrorKind::CalculationFailed, message)
}
struct WeaponComponent {
    weapon: MaceWeapon,
    quality: u32,
    item_level: u32,
    stats: Result<PreparedWeaponStats, EvaluationError>,
}
struct SelectedMetric {
    index: usize,
    query: MetricQuery,
    unit: MetricUnit,
    schema_version: u32,
}
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct PreparedBuildFootprint {
    pub weapon_components: usize,
    pub metric_selectors: usize,
    pub support_components: usize,
    /// Available support definitions rejected by cold authored-loader admission.
    pub deferred_support_errors: usize,
    pub deferred_weapon_errors: usize,
    pub owned_component_bytes: usize,
    pub retained_xml_bytes: usize,
    pub cached_candidate_results: usize,
}
/// Immutable and Send+Sync: handles share the actor result used for admission;
/// calculation borrows compiled weapon/support data and creates fresh skill outputs.
pub struct PreparedBuildCandidates {
    data: Arc<CompiledGameData>,
    identity: BackendIdentity,
    binding: ControlledBuildBinding,
    input: NativeInput,
    weapons: BTreeMap<String, WeaponComponent>,
    support_admission: BTreeMap<String, Result<(), EvaluationError>>,
    metrics: Vec<SelectedMetric>,
}
impl PreparedBuildCandidates {
    pub fn data_identity(&self) -> &poe_optimizer_core::data::DataIdentity {
        self.data.identity()
    }
    pub fn backend_identity(&self) -> &BackendIdentity {
        &self.identity
    }
    pub fn footprint(&self) -> PreparedBuildFootprint {
        PreparedBuildFootprint {
            weapon_components: self.weapons.len(),
            metric_selectors: self.metrics.len(),
            support_components: self.support_admission.len(),
            deferred_support_errors: self
                .support_admission
                .values()
                .filter(|result| result.is_err())
                .count(),
            deferred_weapon_errors: self.weapons.values().filter(|v| v.stats.is_err()).count(),
            owned_component_bytes: std::mem::size_of::<Self>()
                + self
                    .weapons
                    .keys()
                    .map(|id| id.capacity() + std::mem::size_of::<WeaponComponent>())
                    .sum::<usize>()
                + self
                    .support_admission
                    .iter()
                    .map(|(key, result)| {
                        key.capacity()
                            + std::mem::size_of::<Result<(), EvaluationError>>()
                            + result
                                .as_ref()
                                .err()
                                .map_or(0, |error| error.message.capacity())
                    })
                    .sum::<usize>()
                + self.metrics.capacity() * std::mem::size_of::<SelectedMetric>()
                + self
                    .metrics
                    .iter()
                    .map(|m| m.query.id.capacity())
                    .sum::<usize>(),
            retained_xml_bytes: 0,
            cached_candidate_results: 0,
        }
    }
    pub fn calculate(
        &self,
        handle: &AdmittedBuildSelection,
    ) -> Result<NativeCalculation, EvaluationError> {
        if !self.binding.accepts(handle) {
            return Err(contract("Typed build handle belongs to another catalog"));
        }
        for key in handle.support_keys() {
            self.support_admission
                .get(key)
                .ok_or_else(|| contract("Admitted support has no prepared skill component"))?
                .as_ref()
                .map_err(|error| EvaluationError::new(error.kind, error.message.clone()))?;
        }
        let character = handle.character().map_err(preparation)?;
        match self.input {
            NativeInput::Spark(input) => {
                spark::evaluate_with_actor(&input, character, &self.data, handle.actor())
                    .map(NativeCalculation::Spark)
                    .map_err(|e| calculation(e.to_string()))
            }
            NativeInput::Mace(mut input) => {
                let id = handle
                    .selection()
                    .candidate
                    .equipment
                    .get("Weapon 1")
                    .ok_or_else(|| contract("Admitted Mace lacks main hand"))?;
                let component = self
                    .weapons
                    .get(id)
                    .ok_or_else(|| contract("Admitted weapon has no prepared component"))?;
                input.weapon = component.weapon;
                input.quality = component.quality;
                input.item_level = component.item_level;
                let weapon = component
                    .stats
                    .as_ref()
                    .map_err(|e| EvaluationError::new(e.kind, e.message.clone()))?;
                let supports = self
                    .data
                    .mace_support_loadout(handle.support_keys())
                    .map_err(|e| contract(e.to_string()))?;
                mace::evaluate_with_actor(
                    &input,
                    character,
                    &self.data,
                    weapon,
                    supports,
                    handle.actor(),
                )
                .map(NativeCalculation::Mace)
                .map_err(|e| calculation(e.to_string()))
            }
        }
    }
    pub fn measure(
        &self,
        handle: &AdmittedBuildSelection,
    ) -> Result<NativeMetricSnapshot, EvaluationError> {
        self.calculate(handle)
            .map(NativeMetricSnapshot::from_calculation)
    }
    pub fn snapshot_measurements(&self, snapshot: &NativeMetricSnapshot) -> Vec<MetricMeasurement> {
        self.metrics
            .iter()
            .map(|metric| MetricMeasurement {
                query: metric.query.clone(),
                unit: metric.unit,
                schema_version: metric.schema_version,
                value: snapshot.values()[metric.index].to_owned(),
            })
            .collect()
    }
}
impl<C: EvaluationClock> NativeBackend<C> {
    /// Reuses complete source validation and fixed-scenario projection once.
    /// Selected numeric failures remain candidate-local; the imported allocation
    /// is never replaced or calculated as a hidden preparation prerequisite.
    pub fn prepare_controlled_build(
        &self,
        catalog: &ControlledBuildCatalog,
        queries: &[MetricQuery],
    ) -> Result<PreparedBuildCandidates, EvaluationError> {
        self.prepare_controlled_build_with_lineage(
            catalog,
            queries,
            crate::preparation::host_lineage()?,
        )
    }
    /// Portable preparation: the host assigns this import's lineage once.
    pub fn prepare_controlled_build_with_lineage(
        &self,
        catalog: &ControlledBuildCatalog,
        queries: &[MetricQuery],
        lineage: poe_optimizer_core::build_identity::BuildLineage,
    ) -> Result<PreparedBuildCandidates, EvaluationError> {
        if !Arc::ptr_eq(&self.data, catalog.data()) {
            return Err(contract(
                "Catalog requires the same compiled dataset instance",
            ));
        }
        let request = EvaluationRequest {
            build: BuildDocument {
                format: BuildFormat::PathOfBuilding2Xml,
                content: catalog.template().source().into(),
            },
            options: EvaluationOptions::default(),
            metrics: queries.to_vec(),
        };
        let source = crate::preparation::import_request(&request, lineage)?;
        let view = crate::preparation::saved_view(&source, &self.data)?;
        let skills = crate::skills::prepare_authored_skills(
            &source,
            &view,
            &self.data,
            crate::skills::SkillPreparationLimits::default(),
        )?;
        let scenario = crate::profile::prepare_scenario(&request, &self.data, &view)?;
        crate::profile::validate_loaded_skill_projection(&source, &self.data, &skills)?;
        if scenario.config != *catalog.template().config()
            || scenario.actor_quests != catalog.quests()
        {
            return Err(contract("Native and catalog scenario preparation disagree"));
        }
        // Validate introduced supports once through the same source loader as a
        // fresh document. This remains linear in available support axes; rejected
        // definitions affect only candidates selecting those supports.
        let mut support_admission = BTreeMap::new();
        for support in &self.data.snapshot().package().supports {
            if catalog.support_instance(&support.id).is_some() {
                let admitted = catalog
                    .support_preparation_build(&support.id)
                    .map_err(|error| contract(error.to_string()))
                    .and_then(|build| {
                        crate::candidate_skill_admission::validate(build, &self.data, lineage)
                    });
                support_admission.insert(support.id.clone(), admitted);
            }
        }
        let mut weapons = BTreeMap::new();
        for (id, item) in catalog.items() {
            if let Some(record) = item.weapon() {
                let weapon = match record.weapon_key() {
                    "wooden_club" => MaceWeapon::WoodenClub,
                    "smithing_hammer" => MaceWeapon::SmithingHammer,
                    _ => return Err(contract("Unsupported compiled weapon capability")),
                };
                let stats = self
                    .data
                    .prepare_mace_weapon(
                        weapon,
                        record.quality(),
                        record.item_level(),
                        record.local_modifiers(),
                    )
                    .map_err(|e| preparation(e.to_string()));
                weapons.insert(
                    id.into(),
                    WeaponComponent {
                        weapon,
                        quality: record.quality(),
                        item_level: record.item_level(),
                        stats,
                    },
                );
            }
        }
        let requested: BTreeSet<_> = queries.iter().collect();
        let metrics = crate::metric_catalog()
            .into_iter()
            .enumerate()
            .filter_map(|(index, definition)| {
                let query = MetricQuery {
                    actor: ActorScope::Player,
                    id: definition.id,
                };
                (requested.is_empty() || requested.contains(&query)).then_some(SelectedMetric {
                    index,
                    query,
                    unit: definition.unit,
                    schema_version: definition.schema_version,
                })
            })
            .collect();
        Ok(PreparedBuildCandidates {
            data: self.data.clone(),
            identity: self.identity.clone(),
            binding: catalog.binding(),
            input: scenario.input,
            weapons,
            support_admission,
            metrics,
        })
    }
    pub fn evaluate_controlled_build(
        &self,
        prepared: &PreparedBuildCandidates,
        handle: &AdmittedBuildSelection,
        budget: EvaluationBudget,
    ) -> Result<NativeMetricSnapshot, EvaluationError> {
        let start = self.clock.now();
        if prepared.identity != self.identity || !Arc::ptr_eq(&self.data, &prepared.data) {
            return Err(contract(
                "Prepared build evaluator belongs to another backend/data instance",
            ));
        }
        if budget.timeout_ms == 0 {
            return Err(EvaluationError::new(
                EvaluationErrorKind::InvalidRequest,
                "Evaluation timeout must be positive",
            ));
        }
        self.elapsed(start, budget)?;
        let mut snapshot = prepared.measure(handle)?;
        snapshot.set_elapsed(self.elapsed(start, budget)?);
        Ok(snapshot)
    }
}
