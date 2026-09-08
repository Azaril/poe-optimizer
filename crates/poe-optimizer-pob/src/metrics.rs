//! PoB raw fields are mapped here, outside the application/calculation contracts.
use poe_optimizer_core::{ActorOutput, EvaluationSnapshot, metrics::*};

struct Binding {
    id: &'static str,
    raw: &'static str,
    unit: MetricUnit,
    minion: bool,
    description: &'static str,
}
const BINDINGS: &[Binding] = &[
    Binding {
        id: "life",
        raw: "Life",
        unit: MetricUnit::PoolPoints,
        minion: false,
        description: "Maximum life pool, not unreserved or currently remaining life.",
    },
    Binding {
        id: "mana",
        raw: "Mana",
        unit: MetricUnit::PoolPoints,
        minion: false,
        description: "Maximum mana pool, before reservation.",
    },
    Binding {
        id: "energy_shield",
        raw: "EnergyShield",
        unit: MetricUnit::PoolPoints,
        minion: false,
        description: "Maximum available energy shield pool.",
    },
    Binding {
        id: "fire_resistance_capped_pct",
        raw: "FireResist",
        unit: MetricUnit::Percent,
        minion: false,
        description: "Capped fire resistance; 75 means 75 percent.",
    },
    Binding {
        id: "cold_resistance_capped_pct",
        raw: "ColdResist",
        unit: MetricUnit::Percent,
        minion: false,
        description: "Capped cold resistance; 75 means 75 percent.",
    },
    Binding {
        id: "lightning_resistance_capped_pct",
        raw: "LightningResist",
        unit: MetricUnit::Percent,
        minion: false,
        description: "Capped lightning resistance; 75 means 75 percent.",
    },
    Binding {
        id: "chaos_resistance_capped_pct",
        raw: "ChaosResist",
        unit: MetricUnit::Percent,
        minion: false,
        description: "Capped chaos resistance; 75 means 75 percent.",
    },
    Binding {
        id: "pob_total_ehp",
        raw: "TotalEHP",
        unit: MetricUnit::Damage,
        minion: false,
        description: "PoB aggregate effective hit pool under the recorded encounter, avoidance and recovery assumptions.",
    },
    Binding {
        id: "physical_max_hit",
        raw: "PhysicalMaximumHitTaken",
        unit: MetricUnit::Damage,
        minion: false,
        description: "Maximum incoming physical hit in the recorded defensive scenario.",
    },
    Binding {
        id: "fire_max_hit",
        raw: "FireMaximumHitTaken",
        unit: MetricUnit::Damage,
        minion: false,
        description: "Maximum incoming fire hit in the recorded defensive scenario.",
    },
    Binding {
        id: "cold_max_hit",
        raw: "ColdMaximumHitTaken",
        unit: MetricUnit::Damage,
        minion: false,
        description: "Maximum incoming cold hit in the recorded defensive scenario.",
    },
    Binding {
        id: "lightning_max_hit",
        raw: "LightningMaximumHitTaken",
        unit: MetricUnit::Damage,
        minion: false,
        description: "Maximum incoming lightning hit in the recorded defensive scenario.",
    },
    Binding {
        id: "chaos_max_hit",
        raw: "ChaosMaximumHitTaken",
        unit: MetricUnit::Damage,
        minion: false,
        description: "Maximum incoming chaos hit; immunity may produce positive infinity, never an implicit finite score.",
    },
    Binding {
        id: "selected_hit_dps",
        raw: "TotalDPS",
        unit: MetricUnit::DamagePerSecond,
        minion: true,
        description: "Selected actor/action hit DPS with PoB speed and quantity multipliers; excludes separate damage-over-time and Full DPS rollups.",
    },
    Binding {
        id: "selected_average_hit",
        raw: "AverageHit",
        unit: MetricUnit::Damage,
        minion: true,
        description: "Selected actor/action average hit including critical strike weighting; not damage per second.",
    },
    Binding {
        id: "spirit",
        raw: "Spirit",
        unit: MetricUnit::PoolPoints,
        minion: false,
        description: "Maximum Spirit pool, before reservation.",
    },
];

pub fn catalog() -> Vec<MetricDefinition> {
    BINDINGS
        .iter()
        .map(|binding| MetricDefinition {
            id: binding.id.into(),
            unit: binding.unit,
            actors: if binding.minion {
                vec![ActorScope::Player, ActorScope::SelectedMinion]
            } else {
                vec![ActorScope::Player]
            },
            description: binding.description.into(),
            schema_version: 1,
        })
        .collect()
}

fn value(actor: Option<&ActorOutput>, raw: &str, needs_action: bool) -> MeasurementValue {
    let Some(actor) = actor else {
        return MeasurementValue::Unavailable {
            reason: "No selected minion actor exists".into(),
        };
    };
    if needs_action && (actor.skill_id.is_none() || !actor.has_hit_damage) {
        return MeasurementValue::Unavailable {
            reason: "The selected actor/action does not produce hit damage".into(),
        };
    }
    if let Some(value) = actor.metrics.get(raw) {
        MeasurementValue::from_number(*value)
    } else if let Some(kind) = actor.non_finite_values.get(raw) {
        MeasurementValue::NonFinite { kind: *kind }
    } else {
        MeasurementValue::Unavailable {
            reason: "The selected actor/action does not produce this measurement".into(),
        }
    }
}

pub fn measurements(snapshot: &EvaluationSnapshot) -> Vec<MetricMeasurement> {
    let mut measurements = Vec::new();
    for binding in BINDINGS {
        for actor in if binding.minion {
            &[ActorScope::Player, ActorScope::SelectedMinion][..]
        } else {
            &[ActorScope::Player][..]
        } {
            let selected = match actor {
                ActorScope::Player => snapshot.coverage.selected_player.as_ref(),
                ActorScope::SelectedMinion => snapshot.coverage.selected_minion.as_ref(),
            };
            let measurement = if binding.raw == "TotalDPS"
                && selected.is_some_and(|skill| skill.show_average)
            {
                MeasurementValue::Unavailable { reason: "Selected action uses average-damage mode; an explicit usage model is required for DPS".into() }
            } else {
                value(
                    match actor {
                        ActorScope::Player => Some(&snapshot.player),
                        ActorScope::SelectedMinion => snapshot.minion.as_ref(),
                    },
                    binding.raw,
                    binding.minion,
                )
            };
            measurements.push(MetricMeasurement {
                query: MetricQuery {
                    actor: *actor,
                    id: binding.id.into(),
                },
                unit: binding.unit,
                value: measurement,

                schema_version: 1,
            });
        }
    }
    measurements
}
