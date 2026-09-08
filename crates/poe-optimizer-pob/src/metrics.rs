//! PoB raw fields are mapped here, outside the application/calculation contracts.
use poe_optimizer_core::{ActorOutput, EvaluationSnapshot, metrics::*};

struct Binding {
    id: &'static str,
    raw: &'static str,
    value_scale: f64,
    unit: MetricUnit,
    minion: bool,
    description: &'static str,
}
const BINDINGS: &[Binding] = &[
    Binding {
        id: "life",
        raw: "Life",
        value_scale: 1.0,
        unit: MetricUnit::PoolPoints,
        minion: false,
        description: "Maximum life pool, not unreserved or currently remaining life.",
    },
    Binding {
        id: "mana",
        raw: "Mana",
        value_scale: 1.0,
        unit: MetricUnit::PoolPoints,
        minion: false,
        description: "Maximum mana pool, before reservation.",
    },
    Binding {
        id: "energy_shield",
        raw: "EnergyShield",
        value_scale: 1.0,
        unit: MetricUnit::PoolPoints,
        minion: false,
        description: "Maximum available energy shield pool.",
    },
    Binding {
        id: "fire_resistance_capped_pct",
        raw: "FireResist",
        value_scale: 1.0,
        unit: MetricUnit::Percent,
        minion: false,
        description: "Capped fire resistance; 75 means 75 percent.",
    },
    Binding {
        id: "cold_resistance_capped_pct",
        raw: "ColdResist",
        value_scale: 1.0,
        unit: MetricUnit::Percent,
        minion: false,
        description: "Capped cold resistance; 75 means 75 percent.",
    },
    Binding {
        id: "lightning_resistance_capped_pct",
        raw: "LightningResist",
        value_scale: 1.0,
        unit: MetricUnit::Percent,
        minion: false,
        description: "Capped lightning resistance; 75 means 75 percent.",
    },
    Binding {
        id: "chaos_resistance_capped_pct",
        raw: "ChaosResist",
        value_scale: 1.0,
        unit: MetricUnit::Percent,
        minion: false,
        description: "Capped chaos resistance; 75 means 75 percent.",
    },
    Binding {
        id: "pob_total_ehp",
        raw: "TotalEHP",
        value_scale: 1.0,
        unit: MetricUnit::Damage,
        minion: false,
        description: "PoB aggregate effective hit pool under the recorded encounter, avoidance and recovery assumptions.",
    },
    Binding {
        id: "physical_max_hit",
        raw: "PhysicalMaximumHitTaken",
        value_scale: 1.0,
        unit: MetricUnit::Damage,
        minion: false,
        description: "Maximum incoming physical hit in the recorded defensive scenario.",
    },
    Binding {
        id: "fire_max_hit",
        raw: "FireMaximumHitTaken",
        value_scale: 1.0,
        unit: MetricUnit::Damage,
        minion: false,
        description: "Maximum incoming fire hit in the recorded defensive scenario.",
    },
    Binding {
        id: "cold_max_hit",
        raw: "ColdMaximumHitTaken",
        value_scale: 1.0,
        unit: MetricUnit::Damage,
        minion: false,
        description: "Maximum incoming cold hit in the recorded defensive scenario.",
    },
    Binding {
        id: "lightning_max_hit",
        raw: "LightningMaximumHitTaken",
        value_scale: 1.0,
        unit: MetricUnit::Damage,
        minion: false,
        description: "Maximum incoming lightning hit in the recorded defensive scenario.",
    },
    Binding {
        id: "chaos_max_hit",
        raw: "ChaosMaximumHitTaken",
        value_scale: 1.0,
        unit: MetricUnit::Damage,
        minion: false,
        description: "Maximum incoming chaos hit; immunity may produce positive infinity, never an implicit finite score.",
    },
    Binding {
        id: "selected_hit_dps",
        raw: "TotalDPS",
        value_scale: 1.0,
        unit: MetricUnit::DamagePerSecond,
        minion: true,
        description: "Selected actor/action hit DPS with PoB speed and quantity multipliers; excludes separate damage-over-time and Full DPS rollups.",
    },
    Binding {
        id: "selected_average_hit",
        raw: "AverageHit",
        value_scale: 1.0,
        unit: MetricUnit::Damage,
        minion: true,
        description: "Selected actor/action average hit including critical strike weighting; not damage per second.",
    },
    Binding {
        id: "spirit",
        raw: "Spirit",
        value_scale: 1.0,
        unit: MetricUnit::PoolPoints,
        minion: false,
        description: "Maximum Spirit pool, before reservation.",
    },
    Binding {
        id: "armour",
        raw: "Armour",
        value_scale: 1.0,
        unit: MetricUnit::RatingPoints,
        minion: false,
        description: "Final armour rating; not physical damage reduction or maximum hit taken.",
    },
    Binding {
        id: "evasion",
        raw: "Evasion",
        value_scale: 1.0,
        unit: MetricUnit::RatingPoints,
        minion: false,
        description: "Final evasion rating; not chance to evade under an encounter.",
    },
    Binding {
        id: "movement_speed_pct",
        raw: "EffectiveMovementSpeedMod",
        value_scale: 100.0,
        unit: MetricUnit::Percent,
        minion: false,
        description: "Effective movement speed as a percentage of baseline; 100 means baseline, 120 means 20 percent faster. Includes movement and action-speed modifiers, not absolute travel speed.",
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

fn value(
    actor: Option<&ActorOutput>,
    raw: &str,
    needs_action: bool,
    scale: f64,
) -> MeasurementValue {
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
        MeasurementValue::from_number(*value * scale)
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
                    binding.value_scale,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn defence_ratings_are_player_only_versioned_rating_points() {
        let definitions = catalog();
        for id in ["armour", "evasion"] {
            let definition = definitions.iter().find(|entry| entry.id == id).unwrap();
            assert_eq!(definition.unit, MetricUnit::RatingPoints);
            assert_eq!(definition.actors, [ActorScope::Player]);
            assert_eq!(definition.schema_version, 1);
            assert_eq!(
                serde_json::to_value(definition.unit).unwrap(),
                "rating_points"
            );
        }
        // New definitions append without changing existing catalog positions.
        assert_eq!(definitions[15].id, "spirit");
        assert_eq!(definitions[16].id, "armour");
        assert_eq!(definitions[17].id, "evasion");
    }

    #[test]
    fn rating_mapping_preserves_availability_without_requiring_a_hit_action() {
        let mut actor = ActorOutput {
            skill_name: None,
            skill_id: None,
            has_hit_damage: false,
            metrics: BTreeMap::new(),
            non_finite_metrics: Vec::new(),
            non_finite_values: BTreeMap::new(),
        };
        for id in ["armour", "evasion"] {
            let binding = BINDINGS.iter().find(|entry| entry.id == id).unwrap();
            assert!(matches!(
                value(
                    Some(&actor),
                    binding.raw,
                    binding.minion,
                    binding.value_scale
                ),
                MeasurementValue::Unavailable { .. }
            ));
            actor.metrics.insert(binding.raw.into(), 125.0);
            assert_eq!(
                value(
                    Some(&actor),
                    binding.raw,
                    binding.minion,
                    binding.value_scale
                ),
                MeasurementValue::Finite { value: 125.0 }
            );
            actor.metrics.remove(binding.raw);
            for kind in [
                NonFiniteKind::PositiveInfinity,
                NonFiniteKind::NegativeInfinity,
                NonFiniteKind::NotANumber,
            ] {
                actor.non_finite_values.insert(binding.raw.into(), kind);
                assert_eq!(
                    value(
                        Some(&actor),
                        binding.raw,
                        binding.minion,
                        binding.value_scale
                    ),
                    MeasurementValue::NonFinite { kind }
                );
            }
            actor.non_finite_values.remove(binding.raw);
        }
    }

    #[test]
    fn movement_percentage_scales_baseline_and_preserves_unavailable_and_nonfinite() {
        let definition = catalog().pop().unwrap();
        assert_eq!(definition.id, "movement_speed_pct");
        assert_eq!(definition.unit, MetricUnit::Percent);
        assert_eq!(definition.actors, [ActorScope::Player]);
        assert_eq!(definition.schema_version, 1);
        let binding = BINDINGS.last().unwrap();
        let mut actor = ActorOutput {
            skill_name: None,
            skill_id: None,
            has_hit_damage: false,
            metrics: BTreeMap::new(),
            non_finite_metrics: Vec::new(),
            non_finite_values: BTreeMap::new(),
        };
        let read = |actor: &ActorOutput| {
            value(
                Some(actor),
                binding.raw,
                binding.minion,
                binding.value_scale,
            )
        };
        assert!(matches!(read(&actor), MeasurementValue::Unavailable { .. }));
        for (ratio, percentage) in [
            (1.0, 100.0),
            (0.96, 96.0),
            (1.2, 120.0),
            (-0.25, -25.0),
            (0.0, 0.0),
        ] {
            actor.metrics.insert(binding.raw.into(), ratio);
            assert_eq!(read(&actor), MeasurementValue::Finite { value: percentage });
        }
        actor.metrics.insert(binding.raw.into(), f64::MAX);
        assert_eq!(
            read(&actor),
            MeasurementValue::NonFinite {
                kind: NonFiniteKind::PositiveInfinity
            }
        );
        actor.metrics.clear();
        for kind in [
            NonFiniteKind::PositiveInfinity,
            NonFiniteKind::NegativeInfinity,
            NonFiniteKind::NotANumber,
        ] {
            actor.non_finite_values.insert(binding.raw.into(), kind);
            assert_eq!(read(&actor), MeasurementValue::NonFinite { kind });
        }
        assert!(matches!(
            value(None, binding.raw, binding.minion, binding.value_scale),
            MeasurementValue::Unavailable { .. }
        ));
    }
}
