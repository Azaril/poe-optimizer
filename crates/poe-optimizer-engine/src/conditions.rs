//! Typed, immutable condition inputs for the supported `ModStore::EvalMod` tags.
//!
//! The legacy context contains explicit condition tables. `ConditionProgram`
//! additionally evaluates represented FLAG producers and their dependencies.
//! Neither boundary is an actor/modifier importer. Actor references preserve `getActor`
//! lookup, including the special player fallback through parent or enemy.

use std::collections::BTreeMap;

use crate::modifiers::ModifierError;

mod condition_program;
pub use condition_program::*;

/// A bound condition query consumed by the ordinary numeric aggregation methods.
pub trait ConditionResolver {
    fn store_layer_count(&self) -> usize;
    fn validate_query(&self, _query: &crate::modifiers::QueryContext) -> Result<(), ModifierError> {
        Ok(())
    }
    fn matches(&self, tags: &[ModifierTag]) -> Result<bool, ModifierError>;
}

pub type Conditions = BTreeMap<String, bool>;

/// Ordered local/parent condition tables. False does not mask a true parent.
pub type ConditionLayers = Vec<Conditions>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionVariables {
    One(String),
    /// OR semantics; an empty list never matches before negation.
    Any(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModifierTag {
    /// Exact source marker: item classification metadata, neutral in global queries.
    Global,
    /// Exact admitted implicit-global metadata, neutral during numerical queries.
    GlobalEffect {
        effect_type: String,
        unscalable: bool,
    },
    Condition {
        variables: ConditionVariables,
        negated: bool,
    },
    ActorCondition {
        /// None targets the queried store, not the actor's base modifier DB.
        actor: Option<String>,
        variables: Option<ConditionVariables>,
        negated: bool,
    },
    /// Retain unknown types/fields at the extraction boundary; never drop them.
    Unsupported(String),
}

/// Weapon metadata consulted by negated Condition tags (e.g. Varunastra).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WeaponConditions {
    pub counts_as_all_one_handed: bool,
    /// The upstream `Added<condition>` fields, with the prefix removed.
    pub added: Conditions,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConditionActor {
    /// This actor's own modifier DB followed by its parent condition tables.
    pub conditions: ConditionLayers,
    /// Actor role to zero-based actor index; missing roles are absent actors.
    pub links: BTreeMap<String, usize>,
    pub weapon_one: WeaponConditions,
    pub weapon_two: WeaponConditions,
    /// Any context feature that cannot be represented must reject the input.
    /// In particular, retain condition-producing FLAG modifiers here.
    pub unsupported_features: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConditionEnvironmentInput {
    /// Queried DB's local and parent condition tables. Parent modifiers still
    /// evaluate against this root context, as in `ModDB`'s context parameter.
    pub store_conditions: ConditionLayers,
    pub current_actor: usize,
    pub actors: Vec<ConditionActor>,
    /// `cfg.overrideCond`; false is a present override that suppresses parents.
    pub overrides: Conditions,
    /// `cfg.skillCond`; Condition checks this after GetCondition returns false.
    /// ActorCondition deliberately does not consult these skill-local values.
    pub skill_conditions: Conditions,
    /// `cfg.actor`, used by ActorCondition when no condition target is present.
    pub query_actor: Option<String>,
    pub unsupported_features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionEnvironment {
    input: ConditionEnvironmentInput,
}

impl ConditionEnvironment {
    pub fn try_new(input: ConditionEnvironmentInput) -> Result<Self, ModifierError> {
        let invalid = |reason| ModifierError::InvalidConditionContext { reason };
        if input.current_actor >= input.actors.len() {
            return Err(invalid("Current actor index is out of range".to_owned()));
        }
        if let Some(feature) = input.unsupported_features.first() {
            return Err(invalid(format!(
                "Unsupported store context feature: {feature}"
            )));
        }
        for (index, actor) in input.actors.iter().enumerate() {
            if let Some(feature) = actor.unsupported_features.first() {
                return Err(invalid(format!(
                    "Unsupported actor {index} context feature: {feature}"
                )));
            }
            for (role, target) in &actor.links {
                if *target >= input.actors.len() {
                    return Err(invalid(format!(
                        "Actor {index} role {role} references missing actor {target}"
                    )));
                }
            }
        }
        Ok(Self { input })
    }

    pub fn input(&self) -> &ConditionEnvironmentInput {
        &self.input
    }

    pub(crate) fn matches(&self, tags: &[ModifierTag]) -> bool {
        condition_program::matches_predicates(tags, &mut LegacyLookup(self))
            .expect("Legacy context contains only validated condition predicates")
    }

    fn condition(&self, layers: &ConditionLayers, variable: &str) -> bool {
        self.input
            .overrides
            .get(variable)
            .copied()
            .unwrap_or_else(|| {
                layers
                    .iter()
                    .any(|layer| layer.get(variable).copied().unwrap_or(false))
            })
    }

    fn actor(&self, role: &str) -> Option<&ConditionActor> {
        let current = &self.input.actors[self.input.current_actor];
        let direct = current.links.get(role).copied();
        let index = if role == "player" {
            direct
                .or_else(|| {
                    current
                        .links
                        .get("parent")
                        .and_then(|parent| self.input.actors[*parent].links.get("player"))
                        .copied()
                })
                .or_else(|| {
                    current
                        .links
                        .get("enemy")
                        .and_then(|enemy| self.input.actors[*enemy].links.get("player"))
                        .copied()
                })
        } else {
            direct
        };
        index.map(|index| &self.input.actors[index])
    }
}

impl ConditionVariables {
    fn iter(&self) -> std::slice::Iter<'_, String> {
        match self {
            Self::One(variable) => std::slice::from_ref(variable).iter(),
            Self::Any(variables) => variables.iter(),
        }
    }
}

impl ConditionResolver for ConditionEnvironment {
    fn store_layer_count(&self) -> usize {
        self.input.store_conditions.len()
    }
    fn matches(&self, tags: &[ModifierTag]) -> Result<bool, ModifierError> {
        for tag in tags {
            condition_program::validate_predicate(tag)?;
        }
        Ok(self.matches(tags))
    }
}

struct LegacyLookup<'a>(&'a ConditionEnvironment);
impl condition_program::PredicateLookup for LegacyLookup<'_> {
    fn weapon(&self) -> Option<&WeaponConditions> {
        let actor = &self.0.input.actors[self.0.input.current_actor];
        if actor.weapon_one.counts_as_all_one_handed {
            Some(&actor.weapon_one)
        } else if actor.weapon_two.counts_as_all_one_handed {
            Some(&actor.weapon_two)
        } else {
            None
        }
    }
    fn query_actor(&self) -> Option<&str> {
        self.0.input.query_actor.as_deref()
    }
    fn actor_exists(&self, role: &str) -> bool {
        self.0.actor(role).is_some()
    }
    fn condition(&mut self, role: Option<&str>, variable: &str) -> Result<bool, ModifierError> {
        let layers = match role {
            None => Some(&self.0.input.store_conditions),
            Some(role) => self.0.actor(role).map(|actor| &actor.conditions),
        };
        Ok(layers.is_some_and(|layers| self.0.condition(layers, variable)))
    }
    fn skill_condition(&self, variable: &str) -> bool {
        self.0
            .input
            .skill_conditions
            .get(variable)
            .copied()
            .unwrap_or(false)
    }
}
