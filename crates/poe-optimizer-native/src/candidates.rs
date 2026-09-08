//! Prepared typed native calculations for a strictly admitted controlled domain.
//!
//! Import owns exact membership, requirements and source/catalog binding. This
//! module owns scenario parsing, shared character composition and native metric
//! selection. It retains neither XML nor candidate calculation results. Native
//! calculation preparation grows with axis sizes, not their Cartesian product.
use crate::{CompiledGameData, EvaluationClock, NativeBackend, profile, tree};
use poe_optimizer_core::{evaluation::*, metrics::*};
use poe_optimizer_engine::{
    character::CharacterInput,
    mace::{self, MaceInput, MaceOutput, MaceWeapon},
    mace_supports::PreparedMaceSupports,
};
use poe_optimizer_import::controlled_mace::{
    NativeMaceBinding, NativeMaceCandidate, NativeMaceComponents,
};
use std::{collections::BTreeSet, mem::size_of, sync::Arc};

pub(crate) const MACE_AVERAGE_REASON: &str =
    "Attack AverageHit is stored per hand; the current metric contract does not aggregate hands.";

/// Borrowed/inline metric value. Formatting into the shared owned measurement
/// contract happens only at the scheduler/report boundary, outside calculation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NativeMetricValue {
    Finite(f64),
    NonFinite(NonFiniteKind),
    Unavailable(&'static str),
}
impl NativeMetricValue {
    fn from_number(value: f64) -> Self {
        match MeasurementValue::from_number(value) {
            MeasurementValue::Finite { value } => Self::Finite(value),
            MeasurementValue::NonFinite { kind } => Self::NonFinite(kind),
            MeasurementValue::Unavailable { .. } => unreachable!("numeric conversion"),
        }
    }
    pub fn finite(self) -> Option<f64> {
        match self {
            Self::Finite(value) if value.is_finite() => Some(value),
            _ => None,
        }
    }
    pub fn to_owned(self) -> MeasurementValue {
        match self {
            Self::Finite(value) => MeasurementValue::Finite { value },
            Self::NonFinite(kind) => MeasurementValue::NonFinite { kind },
            Self::Unavailable(reason) => MeasurementValue::Unavailable {
                reason: reason.into(),
            },
        }
    }
}

/// Stack snapshot in native metric-catalog order, including explicit availability.
/// No strings, XML, JSON, diagnostics or cached build result are constructed.
#[derive(Debug, Clone, Copy)]
pub struct NativeMetricSnapshot {
    values: [NativeMetricValue; 9],
    elapsed_ms: f64,
}
impl NativeMetricSnapshot {
    /// Complete native Mace metric catalog order; use the prepared selector's
    /// `snapshot_measurements` for requested metrics and owned query descriptors.
    pub fn values(&self) -> &[NativeMetricValue] {
        &self.values
    }
    pub fn elapsed_ms(&self) -> f64 {
        self.elapsed_ms
    }
    /// This bounded diagnostic profile does not certify complete game coverage.
    pub fn diagnostic_only(&self) -> bool {
        true
    }
    fn from_output(output: MaceOutput) -> Self {
        Self {
            values: [
                NativeMetricValue::from_number(output.life),
                NativeMetricValue::from_number(output.mana),
                NativeMetricValue::from_number(output.energy_shield),
                NativeMetricValue::from_number(output.fire_resistance),
                NativeMetricValue::from_number(output.cold_resistance),
                NativeMetricValue::from_number(output.lightning_resistance),
                NativeMetricValue::from_number(output.chaos_resistance),
                NativeMetricValue::Unavailable(MACE_AVERAGE_REASON),
                NativeMetricValue::from_number(output.hit_dps),
            ],
            elapsed_ms: 0.0,
        }
    }
}
struct SelectedMetric {
    index: usize,
    query: MetricQuery,
    unit: MetricUnit,
    schema_version: u32,
}

/// Component storage, excluding the shared dataset, import catalog/handles and
/// backend identity. Capacity bytes include support key and metric query buffers;
/// allocator metadata, Arc control blocks and stack frames are not estimated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct PreparedMaceFootprint {
    pub weapon_components: usize,
    pub tree_components: usize,
    pub support_components: usize,
    pub metric_selectors: usize,
    pub deferred_character_errors: usize,
    pub owned_component_bytes: usize,
    pub retained_xml_bytes: usize,
    pub cached_candidate_results: usize,
}

/// Immutable native calculation components, bound to one private import catalog
/// and one compiled dataset. Reusable directly from Rayon or portable hosts.
pub struct PreparedMaceCandidates {
    data: Arc<CompiledGameData>,
    identity: BackendIdentity,
    binding: NativeMaceBinding,
    inputs: Vec<MaceInput>,
    characters: Vec<Result<CharacterInput, EvaluationError>>,
    supports: Vec<PreparedMaceSupports>,
    metrics: Vec<SelectedMetric>,
}
impl PreparedMaceCandidates {
    pub fn data_identity(&self) -> &poe_optimizer_core::data::DataIdentity {
        self.data.identity()
    }
    pub fn backend_identity(&self) -> &BackendIdentity {
        &self.identity
    }
    pub fn footprint(&self) -> PreparedMaceFootprint {
        let support_keys = self
            .supports
            .iter()
            .map(|support| {
                // keys() is a slice, so account retained elements rather than guessing
                // the private Vec capacity. Compiled support cloning allocates exactly
                // these elements; String clones allocate their current lengths.
                std::mem::size_of_val(support.keys())
                    + support.keys().iter().map(String::capacity).sum::<usize>()
            })
            .sum::<usize>();
        PreparedMaceFootprint {
            weapon_components: self.inputs.len(),
            tree_components: self.characters.len(),
            support_components: self.supports.len(),
            metric_selectors: self.metrics.len(),
            deferred_character_errors: self
                .characters
                .iter()
                .filter(|entry| entry.is_err())
                .count(),
            owned_component_bytes: self.inputs.capacity() * size_of::<MaceInput>()
                + self.characters.capacity() * size_of::<Result<CharacterInput, EvaluationError>>()
                + self
                    .characters
                    .iter()
                    .filter_map(|entry| entry.as_ref().err())
                    .map(|error| error.message.capacity())
                    .sum::<usize>()
                + self.supports.capacity() * size_of::<PreparedMaceSupports>()
                + self.metrics.capacity() * size_of::<SelectedMetric>()
                + self
                    .metrics
                    .iter()
                    .map(|metric| metric.query.id.capacity())
                    .sum::<usize>()
                + support_keys,
            retained_xml_bytes: 0,
            cached_candidate_results: 0,
        }
    }
    /// Fresh pure numerical calculation. The import handle has already passed
    /// exact membership/requirements admission; O(1) binding and axis checks
    /// prevent cross-catalog reuse before any native kernel is called.
    pub fn calculate(
        &self,
        candidate: &NativeMaceCandidate,
    ) -> Result<MaceOutput, EvaluationError> {
        if !candidate.belongs_to_binding(&self.binding) {
            return Err(contract(
                "Native candidate belongs to a different prepared catalog",
            ));
        }
        let input = self
            .inputs
            .get(candidate.weapon_index())
            .ok_or_else(|| contract("Invalid native weapon axis"))?;
        let character = self
            .characters
            .get(candidate.tree_index())
            .ok_or_else(|| contract("Invalid native tree axis"))?
            .as_ref()
            .map_err(|error| EvaluationError::new(error.kind, error.message.clone()))?;
        let supports = self
            .supports
            .get(candidate.loadout_index())
            .ok_or_else(|| contract("Invalid native support axis"))?;
        mace::evaluate_with_supports(input, character, &self.data, supports).map_err(|error| {
            EvaluationError::new(EvaluationErrorKind::CalculationFailed, error.to_string())
        })
    }
    /// Pure stack measurements; no host clock or OS services are consulted.
    pub fn measure(
        &self,
        candidate: &NativeMaceCandidate,
    ) -> Result<NativeMetricSnapshot, EvaluationError> {
        self.calculate(candidate)
            .map(NativeMetricSnapshot::from_output)
    }
    /// Adapt stack values to the shared scheduler contract. This deliberately
    /// allocates owned query/reason strings and is not part of the numeric kernel.
    pub fn snapshot_measurements(&self, snapshot: &NativeMetricSnapshot) -> Vec<MetricMeasurement> {
        self.metrics
            .iter()
            .map(|metric| MetricMeasurement {
                query: metric.query.clone(),
                unit: metric.unit,
                value: snapshot.values[metric.index].to_owned(),
                schema_version: metric.schema_version,
            })
            .collect()
    }
}
fn contract(message: &'static str) -> EvaluationError {
    EvaluationError::new(EvaluationErrorKind::BackendContract, message)
}
impl<C: EvaluationClock> NativeBackend<C> {
    /// Reuse the strict full native source/config parser exactly once, then retain
    /// only immutable numerical axes. A bound baseline and import-private view
    /// are required; arbitrary numerical fields cannot enter this admission seam.
    pub fn prepare_controlled_mace(
        &self,
        components: &NativeMaceComponents,
        queries: &[MetricQuery],
    ) -> Result<PreparedMaceCandidates, EvaluationError> {
        if components.backend_identity() != &self.identity
            || components.snapshot().identity() != self.data.identity()
        {
            return Err(contract(
                "Native components belong to a different data or calculation identity",
            ));
        }
        let request = EvaluationRequest {
            build: components.template_build(),
            options: components.context().requested.clone(),
            metrics: queries.to_vec(),
        };
        // Full prepare shares query validation and exact parser behavior with
        // baseline/finalist execution. It performs no build calculation here.
        let baseline = self.prepare(&request)?;
        let profile::NativeInput::Mace(scenario) = baseline.profile.input else {
            return Err(contract(
                "Controlled Mace preparation requires the native Mace profile",
            ));
        };
        if scenario.character_level != components.character_level()
            || baseline.profile.config != components.context().config_inputs
            || baseline.profile.enemy_level != components.context().enemy_level
        {
            return Err(contract(
                "Native scenario differs from the strictly bound baseline",
            ));
        }
        let inputs = components
            .weapons()
            .iter()
            .map(|record| {
                let weapon = match record.weapon_key() {
                    "wooden_club" => MaceWeapon::WoodenClub,
                    "smithing_hammer" => MaceWeapon::SmithingHammer,
                    _ => return Err(contract("Unknown native weapon capability slot")),
                };
                Ok(MaceInput {
                    weapon,
                    quality: record.quality(),
                    item_level: record.item_level(),
                    ..scenario
                })
            })
            .collect::<Result<Vec<_>, EvaluationError>>()?;
        // A custom dataset can admit individual effects whose selected sum is
        // outside the numeric scope. Preserve the full-document path's deferred
        // failure: unused/locked-out axes must not abort a valid search domain.
        let characters = components
            .trees()
            .iter()
            .map(|resolved| tree::character_from_resolved(resolved, &self.data))
            .collect();
        let supports = components
            .loadouts()
            .iter()
            .map(|loadout| {
                self.data
                    .mace_support_loadout(loadout.keys())
                    .cloned()
                    .map_err(|error| {
                        EvaluationError::new(
                            EvaluationErrorKind::UnsupportedCapability,
                            error.to_string(),
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
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
        Ok(PreparedMaceCandidates {
            data: self.data.clone(),
            identity: self.identity.clone(),
            binding: components.binding().clone(),
            inputs,
            characters,
            supports,
            metrics,
        })
    }
    /// Timed typed evaluation. The same host clock and deadline semantics as the
    /// full document backend apply; the search caller owns attempt accounting.
    pub fn evaluate_controlled_mace(
        &self,
        prepared: &PreparedMaceCandidates,
        candidate: &NativeMaceCandidate,
        budget: EvaluationBudget,
    ) -> Result<NativeMetricSnapshot, EvaluationError> {
        let start = self.clock.now();
        if prepared.identity != self.identity {
            return Err(contract(
                "Prepared native candidates belong to a different data or calculation identity",
            ));
        }
        if budget.timeout_ms == 0 {
            return Err(EvaluationError::new(
                EvaluationErrorKind::InvalidRequest,
                "Evaluation timeout must be positive",
            ));
        }
        self.elapsed(start, budget)?;
        let mut snapshot = prepared.measure(candidate)?;
        snapshot.elapsed_ms = self.elapsed(start, budget)?;
        Ok(snapshot)
    }
}
