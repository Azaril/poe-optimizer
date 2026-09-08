//! Immutable FLAG producers with explicit stores and caller-owned query values.
//! This is a query primitive, not build loading, actor scheduling, or an implicit solver.

use super::{ConditionResolver, ConditionVariables, ModifierTag, WeaponConditions};
use crate::{
    modifiers::{ModifierError, ModifierStoreKind, QueryContext, validate_flags},
    multipliers::{ScalarSource, StatThreshold, StatThresholdValue, stat_threshold_matches},
    stats::{ResolvedStatEnvironment, validate_stat},
};
use std::collections::BTreeMap;

pub const MAX_CONDITION_DEPTH: usize = 64;
const MAX_STORES: usize = crate::modifiers::MAX_MODIFIER_LAYERS;
const MAX_FLAGS: usize = 65_536;
const MAX_TEXT_BYTES: usize = 4_096;

/// Scalar values preserve Lua truthiness: only nil and false are false.
/// Non-scalars must remain explicit unsupported input, never disappear in an adapter.
#[derive(Debug, Clone, PartialEq)]
pub enum ConditionValue {
    Nil,
    Boolean(bool),
    Number(f64),
    Text(String),
    Unsupported(String),
}
impl ConditionValue {
    fn result(&self) -> ConditionResult<'_> {
        match self {
            Self::Nil => ConditionResult::Nil,
            Self::Boolean(value) => ConditionResult::Boolean(*value),
            Self::Number(value) => ConditionResult::Number(*value),
            Self::Text(value) => ConditionResult::Text(value),
            Self::Unsupported(_) => unreachable!("Validated scalar condition value"),
        }
    }
    fn validate(&self) -> Result<(), ModifierError> {
        match self {
            Self::Unsupported(feature) => {
                Err(invalid(format!("Unsupported condition value: {feature}")))
            }
            Self::Text(value) => validate_text(value),
            _ => Ok(()),
        }
    }
}

/// Borrowed raw result. FLAG itself returns true/nil; GetCondition can return any scalar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConditionResult<'a> {
    Nil,
    Boolean(bool),
    Number(f64),
    Text(&'a str),
}
impl ConditionResult<'_> {
    pub const fn truthy(self) -> bool {
        !matches!(self, Self::Nil | Self::Boolean(false))
    }
}
pub type ScalarConditions = BTreeMap<String, ConditionValue>;

#[derive(Debug, Clone, PartialEq)]
pub enum FlagTag {
    Predicate(ModifierTag),
    /// Current-store ordinary GetStat, with a constant optional percentage.
    /// Multiplier percentages remain a retained, explicitly rejected dependency.
    StatThreshold(StatThreshold),
    Unsupported(String),
}
#[derive(Debug, Clone, PartialEq)]
pub struct FlagModifierInput {
    pub name: String,
    pub value: ConditionValue,
    pub flags: u64,
    pub keyword_flags: u64,
    pub source: Option<String>,
    pub tags: Vec<FlagTag>,
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConditionStoreInput {
    /// Query semantics belong to the producer's actual store, independently of
    /// the queried child whose actor/condition context evaluates the record.
    pub kind: ModifierStoreKind,
    pub parent: Option<usize>,
    pub actor: usize,
    pub flags: Vec<FlagModifierInput>,
    pub unsupported_features: Vec<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConditionProgramActor {
    /// Actor's own base modDB; a queried skill store may be a distinct store.
    pub store: usize,
    pub links: BTreeMap<String, usize>,
    pub weapon_one: WeaponConditions,
    pub weapon_two: WeaponConditions,
    pub unsupported_features: Vec<String>,
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConditionProgramInput {
    pub stores: Vec<ConditionStoreInput>,
    pub actors: Vec<ConditionProgramActor>,
    pub unsupported_features: Vec<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionProgram {
    input: ConditionProgramInput,
    layer_kinds: Vec<Vec<ModifierStoreKind>>,
    indices: Vec<StoreFlagIndex>,
    requires_stats: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct StoreFlagIndex {
    names: BTreeMap<String, Vec<usize>>,
    conditions: BTreeMap<String, Vec<usize>>,
}

/// Runtime values can change without recompiling producers. Every store has one
/// explicit local table; inheritance is defined solely by the compiled parent links.
#[derive(Debug, Clone, Copy)]
pub struct ConditionQuery<'a> {
    pub context: &'a QueryContext,
    pub store_conditions: &'a [ScalarConditions],
    pub overrides: &'a ScalarConditions,
    pub skill_conditions: &'a ScalarConditions,
    /// Either empty when no StatThreshold exists, or exactly one table per store.
    pub stats: &'a [ResolvedStatEnvironment],
    pub query_actor: Option<&'a str>,
    pub ignore_source_in_check_conditions: bool,
}
impl<'a> ConditionQuery<'a> {
    pub fn new(context: &'a QueryContext, store_conditions: &'a [ScalarConditions]) -> Self {
        static EMPTY: ScalarConditions = BTreeMap::new();
        Self {
            context,
            store_conditions,
            overrides: &EMPTY,
            skill_conditions: &EMPTY,
            stats: &[],
            query_actor: None,
            ignore_source_in_check_conditions: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BoundConditionQuery<'a> {
    program: &'a ConditionProgram,
    store: usize,
    query: ConditionQuery<'a>,
}

fn invalid(reason: impl Into<String>) -> ModifierError {
    ModifierError::InvalidConditionContext {
        reason: reason.into(),
    }
}
fn validate_text(value: &str) -> Result<(), ModifierError> {
    if value.len() > MAX_TEXT_BYTES {
        Err(invalid("Condition input string exceeds bounded length"))
    } else {
        Ok(())
    }
}
fn validate_values(values: &ScalarConditions) -> Result<(), ModifierError> {
    if values.len() > MAX_FLAGS {
        return Err(invalid("Too many explicit condition values"));
    }
    for (key, value) in values {
        validate_text(key)?;
        value.validate()?;
    }
    Ok(())
}
fn validate_variables(variables: &ConditionVariables) -> Result<(), ModifierError> {
    if variables.iter().len() > MAX_FLAGS {
        return Err(invalid("Too many condition alternatives"));
    }
    for variable in variables.iter() {
        validate_text(variable)?;
    }
    Ok(())
}
pub(super) fn validate_predicate(tag: &ModifierTag) -> Result<(), ModifierError> {
    match tag {
        ModifierTag::Global => Ok(()),
        ModifierTag::GlobalEffect {
            effect_type,
            unscalable,
        } if effect_type == "Global" && *unscalable => Ok(()),
        ModifierTag::Condition { variables, .. } => validate_variables(variables),
        ModifierTag::ActorCondition {
            actor, variables, ..
        } => {
            if let Some(actor) = actor {
                validate_text(actor)?;
            }
            if let Some(variables) = variables {
                validate_variables(variables)?;
            }
            Ok(())
        }
        _ => Err(invalid("Unsupported condition predicate")),
    }
}

impl ConditionProgram {
    pub fn try_new(input: ConditionProgramInput) -> Result<Self, ModifierError> {
        if let Some(feature) = input.unsupported_features.first() {
            return Err(invalid(format!("Unsupported condition program: {feature}")));
        }
        if input.stores.is_empty()
            || input.stores.len() > MAX_STORES
            || input.actors.is_empty()
            || input.actors.len() > MAX_STORES
        {
            return Err(invalid(
                "Condition program requires bounded, nonempty stores and actors",
            ));
        }
        let mut count = 0_usize;
        let mut requires_stats = false;
        for store in &input.stores {
            if store.actor >= input.actors.len()
                || store
                    .parent
                    .is_some_and(|parent| parent >= input.stores.len())
            {
                return Err(invalid("Invalid condition store actor or parent reference"));
            }
            if let Some(feature) = store.unsupported_features.first() {
                return Err(invalid(format!("Unsupported condition store: {feature}")));
            }
            count = count
                .checked_add(store.flags.len())
                .ok_or_else(|| invalid("Too many FLAG producers"))?;
            if count > MAX_FLAGS {
                return Err(invalid("Too many FLAG producers"));
            }
            for flag in &store.flags {
                validate_text(&flag.name)?;
                flag.value.validate()?;
                validate_flags(flag.flags, flag.keyword_flags)?;
                if let Some(source) = &flag.source {
                    validate_text(source)?;
                }
                if flag.tags.len() > MAX_CONDITION_DEPTH {
                    return Err(invalid("Too many FLAG tags"));
                }
                for tag in &flag.tags {
                    match tag {
                        FlagTag::Predicate(tag) => validate_predicate(tag)?,
                        FlagTag::StatThreshold(tag) => {
                            requires_stats = true;
                            for name in tag.stats.names() {
                                validate_text(name)?;
                                validate_stat(name).map_err(|error| invalid(error.to_string()))?;
                            }
                            if let StatThresholdValue::Stat(name) = &tag.threshold {
                                validate_text(name)?;
                                validate_stat(name).map_err(|error| invalid(error.to_string()))?;
                            }
                            if matches!(tag.percent, Some(ScalarSource::Multiplier(_))) {
                                return Err(invalid(
                                    "FLAG StatThreshold multiplier percentage is unsupported",
                                ));
                            }
                        }
                        FlagTag::Unsupported(feature) => {
                            return Err(invalid(format!("Unsupported FLAG dependency: {feature}")));
                        }
                    }
                }
            }
        }
        for actor in &input.actors {
            if actor.store >= input.stores.len()
                || actor
                    .links
                    .values()
                    .any(|index| *index >= input.actors.len())
            {
                return Err(invalid("Invalid condition actor store or role reference"));
            }
            if let Some(feature) = actor.unsupported_features.first() {
                return Err(invalid(format!("Unsupported condition actor: {feature}")));
            }
            for role in actor.links.keys() {
                validate_text(role)?;
            }
        }
        let mut layer_kinds = Vec::with_capacity(input.stores.len());
        for index in 0..input.stores.len() {
            let mut seen = [false; MAX_STORES];
            let mut current = Some(index);
            let mut kinds = Vec::new();
            while let Some(store) = current {
                if seen[store] {
                    return Err(invalid("Condition store parent cycle"));
                }
                seen[store] = true;
                kinds.push(input.stores[store].kind);
                current = input.stores[store].parent;
            }
            layer_kinds.push(kinds);
        }
        let indices = input
            .stores
            .iter()
            .map(|store| {
                let mut index = StoreFlagIndex::default();
                for (position, flag) in store.flags.iter().enumerate() {
                    index
                        .names
                        .entry(flag.name.clone())
                        .or_default()
                        .push(position);
                    if let Some(condition) = flag.name.strip_prefix("Condition:") {
                        index
                            .conditions
                            .entry(condition.into())
                            .or_default()
                            .push(position);
                    }
                }
                index
            })
            .collect();
        Ok(Self {
            input,
            layer_kinds,
            indices,
            requires_stats,
        })
    }
    pub fn input(&self) -> &ConditionProgramInput {
        &self.input
    }
    pub fn bind<'a>(
        &'a self,
        store: usize,
        query: &ConditionQuery<'a>,
    ) -> Result<BoundConditionQuery<'a>, ModifierError> {
        if store >= self.input.stores.len()
            || query.store_conditions.len() != self.input.stores.len()
        {
            return Err(invalid("Condition query store/table count mismatch"));
        }
        if (self.requires_stats || !query.stats.is_empty())
            && query.stats.len() != self.input.stores.len()
        {
            return Err(invalid("Condition query stat table count mismatch"));
        }
        validate_flags(query.context.flags, query.context.keyword_flags)?;
        for values in query.store_conditions {
            validate_values(values)?;
        }
        validate_values(query.overrides)?;
        validate_values(query.skill_conditions)?;
        Ok(BoundConditionQuery {
            program: self,
            store,
            query: *query,
        })
    }
}

impl<'a> BoundConditionQuery<'a> {
    pub fn get_condition(
        &self,
        variable: &str,
        no_mod: bool,
    ) -> Result<ConditionResult<'a>, ModifierError> {
        State::new(self.program, self.query).get_condition(self.store, variable, no_mod)
    }
    pub fn flag(&self, names: &[&str]) -> Result<Option<bool>, ModifierError> {
        if names.len() > 8 {
            return Err(ModifierError::TooManyNames { count: names.len() });
        }
        State::new(self.program, self.query).flag(self.store, names)
    }
}
impl ConditionResolver for BoundConditionQuery<'_> {
    fn store_layer_count(&self) -> usize {
        self.program.layer_kinds[self.store].len()
    }
    fn store_kind(&self, layer: usize) -> Option<ModifierStoreKind> {
        self.program.layer_kinds[self.store].get(layer).copied()
    }
    fn validate_query(&self, query: &QueryContext) -> Result<(), ModifierError> {
        if self.query.context == query {
            Ok(())
        } else {
            Err(invalid("Numeric query differs from bound condition query"))
        }
    }
    fn matches(&self, tags: &[ModifierTag]) -> Result<bool, ModifierError> {
        for tag in tags {
            validate_predicate(tag)?;
        }
        let mut state = State::new(self.program, self.query);
        matches_predicates(
            tags,
            &mut ProgramLookup {
                state: &mut state,
                store: self.store,
            },
        )
    }
}

struct State<'a> {
    program: &'a ConditionProgram,
    query: ConditionQuery<'a>,
    stack: [Option<(usize, usize, usize)>; MAX_CONDITION_DEPTH],
    depth: usize,
}
impl<'a> State<'a> {
    fn new(program: &'a ConditionProgram, query: ConditionQuery<'a>) -> Self {
        Self {
            program,
            query,
            stack: [None; MAX_CONDITION_DEPTH],
            depth: 0,
        }
    }
    fn target(&self, store: usize, role: &str) -> Option<usize> {
        let actors = &self.program.input.actors;
        let actor = &actors[self.program.input.stores[store].actor];
        let direct = actor.links.get(role).copied();
        let index = if role == "player" {
            direct
                .or_else(|| {
                    actor
                        .links
                        .get("parent")
                        .and_then(|parent| actors[*parent].links.get("player"))
                        .copied()
                })
                .or_else(|| {
                    actor
                        .links
                        .get("enemy")
                        .and_then(|enemy| actors[*enemy].links.get("player"))
                        .copied()
                })
        } else {
            direct
        };
        index.map(|index| actors[index].store)
    }
    fn get_condition(
        &mut self,
        store: usize,
        variable: &str,
        no_mod: bool,
    ) -> Result<ConditionResult<'a>, ModifierError> {
        if let Some(value) = self.query.overrides.get(variable)
            && !matches!(value, ConditionValue::Nil)
        {
            return Ok(value.result());
        }
        let mut current = Some(store);
        while let Some(index) = current {
            if let Some(value) = self.query.store_conditions[index].get(variable)
                && value.result().truthy()
            {
                return Ok(value.result());
            }
            current = self.program.input.stores[index].parent;
        }
        if no_mod {
            return Ok(ConditionResult::Boolean(false));
        }
        // Match without allocating a cached Condition:<variable> string.
        let result = self.condition_flag(store, variable);
        result.map(|flag| {
            if flag.is_some() {
                ConditionResult::Boolean(true)
            } else {
                ConditionResult::Nil
            }
        })
    }
    fn flag(&mut self, store: usize, names: &[&str]) -> Result<Option<bool>, ModifierError> {
        let mut current = Some(store);
        while let Some(index) = current {
            for name in names {
                if let Some(positions) = self.program.indices[index].names.get(*name)
                    && self.flag_layer(store, index, positions)?
                {
                    return Ok(Some(true));
                }
            }
            current = self.program.input.stores[index].parent;
        }
        Ok(None)
    }
    fn condition_flag(
        &mut self,
        store: usize,
        variable: &str,
    ) -> Result<Option<bool>, ModifierError> {
        let mut current = Some(store);
        while let Some(index) = current {
            if let Some(positions) = self.program.indices[index].conditions.get(variable)
                && self.flag_layer(store, index, positions)?
            {
                return Ok(Some(true));
            }
            current = self.program.input.stores[index].parent;
        }
        Ok(None)
    }
    fn flag_layer(
        &mut self,
        context_store: usize,
        layer: usize,
        positions: &[usize],
    ) -> Result<bool, ModifierError> {
        let query = self.query.context;
        for &index in positions {
            let flag = &self.program.input.stores[layer].flags[index];
            if !crate::modifiers::matches_masks(flag.flags, flag.keyword_flags, query) {
                continue;
            }
            let bypass_source = self.program.input.stores[layer].kind == ModifierStoreKind::ModDb
                && self.query.ignore_source_in_check_conditions;
            if !bypass_source && let Some(required) = query.source.as_deref() {
                let source = flag.source.as_deref().ok_or(ModifierError::MissingSource {
                    layer,
                    modifier: index,
                })?;
                if source.split(':').find(|part| !part.is_empty()) != Some(required) {
                    continue;
                }
            }
            let frame = (context_store, layer, index);
            if self.stack[..self.depth].contains(&Some(frame)) {
                return Err(invalid(format!(
                    "Recursive FLAG dependency at store {context_store}, layer {layer}, record {index}"
                )));
            }
            if self.depth == MAX_CONDITION_DEPTH {
                return Err(invalid("Condition dependency depth exceeds bound"));
            }
            self.stack[self.depth] = Some(frame);
            self.depth += 1;
            let mut enabled = true;
            for tag in &flag.tags {
                let matches = match tag {
                    FlagTag::Predicate(tag) => matches_predicates(
                        std::slice::from_ref(tag),
                        &mut ProgramLookup {
                            state: self,
                            store: context_store,
                        },
                    )?,
                    FlagTag::StatThreshold(tag) => {
                        let percent = match tag.percent {
                            Some(ScalarSource::Constant(value)) => Some(value),
                            None => None,
                            _ => unreachable!("Validated threshold dependency"),
                        };
                        stat_threshold_matches(tag, &self.query.stats[context_store], percent)
                            .map_err(|error| invalid(error.to_string()))?
                    }
                    FlagTag::Unsupported(_) => unreachable!("Validated FLAG dependency"),
                };
                if !matches {
                    enabled = false;
                    break;
                }
            }
            self.depth -= 1;
            self.stack[self.depth] = None;
            if enabled && flag.value.result().truthy() {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// Shared source Condition/ActorCondition predicate implementation. A lookup can
/// be table-only or recursively producer-aware without changing tag semantics.
pub(super) trait PredicateLookup {
    fn weapon(&self) -> Option<&WeaponConditions>;
    fn query_actor(&self) -> Option<&str>;
    fn actor_exists(&self, role: &str) -> bool;
    fn condition(&mut self, role: Option<&str>, variable: &str) -> Result<bool, ModifierError>;
    fn skill_condition(&self, variable: &str) -> bool;
}
pub(super) fn matches_predicates(
    tags: &[ModifierTag],
    lookup: &mut impl PredicateLookup,
) -> Result<bool, ModifierError> {
    for tag in tags {
        let matched = match tag {
            ModifierTag::Global | ModifierTag::GlobalEffect { .. } => true,
            ModifierTag::Condition { variables, negated } => {
                let mut matched = false;
                for variable in variables.iter() {
                    let added = lookup
                        .weapon()
                        .filter(|_| *negated)
                        .and_then(|weapon| weapon.added.get(variable))
                        .copied();
                    if let Some(added) = added {
                        if !added {
                            return Ok(false);
                        }
                    } else if lookup.condition(None, variable)? || lookup.skill_condition(variable)
                    {
                        matched = true;
                        break;
                    }
                }
                matched != *negated
            }
            ModifierTag::ActorCondition {
                actor,
                variables,
                negated,
            } => {
                let exists = actor
                    .as_deref()
                    .is_none_or(|role| lookup.actor_exists(role));
                let mut matched = false;
                if exists && let Some(variables) = variables {
                    for variable in variables.iter() {
                        if lookup.condition(actor.as_deref(), variable)? {
                            matched = true;
                            break;
                        }
                    }
                } else {
                    matched = actor.is_some() && actor.as_deref() == lookup.query_actor();
                }
                matched != *negated
            }
            ModifierTag::Unsupported(_) => return Err(invalid("Unsupported condition predicate")),
        };
        if !matched {
            return Ok(false);
        }
    }
    Ok(true)
}
struct ProgramLookup<'s, 'a> {
    state: &'s mut State<'a>,
    store: usize,
}
impl PredicateLookup for ProgramLookup<'_, '_> {
    fn weapon(&self) -> Option<&WeaponConditions> {
        let actor =
            &self.state.program.input.actors[self.state.program.input.stores[self.store].actor];
        if actor.weapon_one.counts_as_all_one_handed {
            Some(&actor.weapon_one)
        } else if actor.weapon_two.counts_as_all_one_handed {
            Some(&actor.weapon_two)
        } else {
            None
        }
    }
    fn query_actor(&self) -> Option<&str> {
        self.state.query.query_actor
    }
    fn actor_exists(&self, role: &str) -> bool {
        self.state.target(self.store, role).is_some()
    }
    fn condition(&mut self, role: Option<&str>, variable: &str) -> Result<bool, ModifierError> {
        let store = match role {
            Some(role) => self.state.target(self.store, role),
            None => Some(self.store),
        };
        let Some(store) = store else {
            return Ok(false);
        };
        // Predicate variables originate in the immutable program or external
        // numeric DB. Recursion owns no references beyond this method's query.
        self.state
            .get_condition(store, variable, false)
            .map(ConditionResult::truthy)
    }
    fn skill_condition(&self, variable: &str) -> bool {
        self.state
            .query
            .skill_conditions
            .get(variable)
            .is_some_and(|value| value.result().truthy())
    }
}
