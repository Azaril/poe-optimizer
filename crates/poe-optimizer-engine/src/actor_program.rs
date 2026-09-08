//! Compiled global actor records with borrowed source composition and bounded scratch.
//! Each program is a source fragment, not a modifier-database parent. Grouping
//! fragments into layers is explicit because source MORE rounding is per layer.
use super::*;
use crate::modifiers::round_more_product;

const MAX_RECORDS: usize = 512;
const MAX_LAYERS: usize = 16;
const MAX_PROGRAM_REFERENCES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Base,
    Increased,
    More,
    Override,
    Flag,
}
impl From<ActorNumericOperation> for Kind {
    fn from(value: ActorNumericOperation) -> Self {
        match value {
            ActorNumericOperation::Base => Self::Base,
            ActorNumericOperation::Increased => Self::Increased,
            ActorNumericOperation::More => Self::More,
            ActorNumericOperation::Override => Self::Override,
        }
    }
}
/// The actor IR admits only conditions produced by this exact attribute stage.
/// No external actor, weapon-condition override or skill-local condition enters
/// this representation; those forms must first gain an explicit data/engine seam.
#[derive(Debug, Clone, Copy)]
struct Predicate {
    any: [u16; 8],
    negated: u8,
    len: u8,
}
impl Predicate {
    fn compile(tags: &[ActorModifierTag]) -> Self {
        let mut result = Self {
            any: [0; 8],
            negated: 0,
            len: tags.len() as u8,
        };
        for (index, ActorModifierTag::Condition { variables, negated }) in tags.iter().enumerate() {
            for variable in variables {
                let bit = CONDITIONS
                    .iter()
                    .position(|condition| condition == variable)
                    .expect("complete actor condition vocabulary");
                result.any[index] |= 1 << bit;
            }
            if *negated {
                result.negated |= 1 << index;
            }
        }
        result
    }
    fn matches(self, conditions: u16) -> bool {
        (0..usize::from(self.len))
            .all(|index| (self.any[index] & conditions != 0) != (self.negated & (1 << index) != 0))
    }
}
#[derive(Debug, Clone, Copy)]
struct Row {
    value: f64,
    predicate: Predicate,
}
#[derive(Debug, Clone, Copy)]
struct Bucket {
    name: &'static str,
    kind: Kind,
    start: usize,
    len: usize,
    more_precision: Option<u8>,
}
/// Immutable compiled numeric/flag records. Data-source strings and diagnostics
/// stay with their validated import records; this program retains no source text.
#[derive(Debug, Clone)]
pub struct CompiledActorModifiers {
    binding: Arc<()>,
    rows: Vec<Row>,
    buckets: Vec<Bucket>,
    requires_downstream_defences: bool,
}
impl CompiledActorModifiers {
    pub fn record_count(&self) -> usize {
        self.rows.len()
    }
    pub fn owned_heap_bytes(&self) -> usize {
        self.rows.capacity() * std::mem::size_of::<Row>()
            + self.buckets.capacity() * std::mem::size_of::<Bucket>()
    }
    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }
    fn bucket(&self, name: &str, kind: Kind) -> Option<&Bucket> {
        self.buckets
            .binary_search_by_key(&(name, kind), |bucket| (bucket.name, bucket.kind))
            .ok()
            .map(|index| &self.buckets[index])
    }
    fn rows(&self, bucket: &Bucket) -> &[Row] {
        &self.rows[bucket.start..bucket.start + bucket.len]
    }
}
/// Ordered fragments concatenated into one DB layer. Fresh builds generally
/// place config, equipment slots and passives in the same local layer; making
/// them separate parents would change numeric rounding and override precedence.
#[derive(Debug, Clone, Copy)]
pub struct ActorModifierLayer<'a> {
    pub programs: &'a [&'a CompiledActorModifiers],
}
/// Fixed storage reset on every call, including before input validation. It
/// retains no program references, actor outputs, records copied from programs,
/// heap storage or cross-candidate results. Use independent scratch per worker.
pub struct ActorScratch {
    base: StackRecords,
    base_len: usize,
    conditions: u16,
}
impl Default for ActorScratch {
    fn default() -> Self {
        Self {
            base: StackRecords::new(),
            base_len: 0,
            conditions: 0,
        }
    }
}
impl ActorScratch {
    pub const fn storage_bytes() -> usize {
        std::mem::size_of::<Self>()
    }
    fn reset(&mut self) {
        *self = Self::default();
    }
}
struct ProgramQueries<'a, 's> {
    layers: &'a [ActorModifierLayer<'a>],
    scratch: &'s mut ActorScratch,
}
impl ProgramQueries<'_, '_> {
    fn layer_count(&self) -> usize {
        self.layers.len().max(1)
    }
    fn programs(&self, index: usize) -> &[&CompiledActorModifiers] {
        self.layers
            .get(index)
            .map(|layer| layer.programs)
            .unwrap_or(&[])
    }
    fn base_sum(
        &self,
        name: &str,
        kind: Kind,
        range: std::ops::Range<usize>,
        mut result: f64,
    ) -> f64 {
        for row in &self.scratch.base.as_slice()[range] {
            if row.stat.upstream_name() == name && Kind::from(row.operation) == kind {
                result += row.value;
            }
        }
        result
    }
}
impl ActorQueries for ProgramQueries<'_, '_> {
    fn sum(&self, kind: SumKind, names: &[&str]) -> Result<f64, ActorError> {
        let kind = match kind {
            SumKind::Base => Kind::Base,
            SumKind::Increased => Kind::Increased,
        };
        let mut parent = None;
        for layer in (0..self.layer_count()).rev() {
            let mut result = 0.0;
            for name in names {
                if layer == 0 {
                    result = self.base_sum(name, kind, 0..self.scratch.base_len, result);
                }
                for program in self.programs(layer) {
                    if let Some(bucket) = program.bucket(name, kind) {
                        for row in program.rows(bucket) {
                            result += if row.predicate.matches(self.scratch.conditions) {
                                row.value
                            } else {
                                0.0
                            };
                        }
                    }
                }
                if layer == 0 {
                    result = self.base_sum(
                        name,
                        kind,
                        self.scratch.base_len..self.scratch.base.len,
                        result,
                    );
                }
            }
            if let Some(parent) = parent {
                result += parent;
            }
            parent = Some(result);
        }
        Ok(parent.unwrap_or(0.0))
    }
    fn more(&self, name: &str) -> Result<f64, ActorError> {
        let mut parent = None;
        for layer in (0..self.layer_count()).rev() {
            let mut product = 1.0;
            let mut precision = None;
            for program in self.programs(layer) {
                if let Some(bucket) = program.bucket(name, Kind::More) {
                    for row in program.rows(bucket) {
                        let value = if row.predicate.matches(self.scratch.conditions) {
                            row.value
                        } else {
                            0.0
                        };
                        product *= 1.0 + value / 100.0;
                    }
                    // A disabled MORE still selects this target's precision.
                    precision = bucket.more_precision;
                }
            }
            let mut result = round_more_product(1.0, product, precision);
            if let Some(parent) = parent {
                result *= parent;
            }
            parent = Some(result);
        }
        Ok(parent.unwrap_or(1.0))
    }
    fn override_value(&self, name: &str) -> Result<Option<f64>, ActorError> {
        for layer in 0..self.layer_count() {
            for program in self.programs(layer) {
                if let Some(bucket) = program.bucket(name, Kind::Override) {
                    for row in program.rows(bucket) {
                        if row.predicate.matches(self.scratch.conditions) {
                            return Ok(Some(row.value));
                        }
                    }
                }
            }
        }
        Ok(None)
    }
    fn flag(&self, name: &str) -> bool {
        (0..self.layer_count()).any(|layer| {
            self.programs(layer).iter().any(|program| {
                program.bucket(name, Kind::Flag).is_some_and(|bucket| {
                    program.rows(bucket).iter().any(|row| {
                        row.value != 0.0 && row.predicate.matches(self.scratch.conditions)
                    })
                })
            })
        })
    }
    fn update_conditions(&mut self, conditions: [bool; 12]) -> Result<(), ActorError> {
        self.scratch.conditions = conditions
            .into_iter()
            .enumerate()
            .fold(0, |bits, (index, value)| bits | (u16::from(value) << index));
        Ok(())
    }
    fn add_bonus(&mut self, record: BuiltinRecord) {
        self.scratch.base.add_bonus(record);
    }
    fn finish_bonuses(&mut self) -> Result<(), ActorError> {
        Ok(())
    }
}
impl CompiledGameData {
    /// Validate and compile one source fragment, once per configuration/gear/tree
    /// contribution. Every record is checked even if its conditions never match.
    pub fn compile_actor_modifiers(
        &self,
        records: &[ActorModifierRecord],
    ) -> Result<CompiledActorModifiers, ActorError> {
        if records.len() > MAX_RECORDS {
            return Err(ActorError("Actor program exceeds 512 normalized records"));
        }
        let mut ordered = Vec::with_capacity(records.len());
        for record in records {
            record.validate().map_err(|_|ActorError("Invalid normalized actor target, operation, numeric bound, flags, source or condition tags"))?;
            let (kind, value) = match record.effect {
                ActorModifierEffect::Numeric { operation, value } => (Kind::from(operation), value),
                ActorModifierEffect::Flag { value } => (Kind::Flag, f64::from(value)),
            };
            ordered.push((
                record.stat.upstream_name(),
                kind,
                Row {
                    value,
                    predicate: Predicate::compile(&record.tags),
                },
            ));
        }
        // Stable ordering retains source insertion order within each queried
        // target/operation while avoiding a full source scan for every query.
        ordered.sort_by_key(|(name, kind, _)| (*name, *kind));
        let mut rows = Vec::with_capacity(ordered.len());
        let mut buckets: Vec<Bucket> = Vec::new();
        for (name, kind, row) in ordered {
            if let Some(last) = buckets
                .last_mut()
                .filter(|bucket| bucket.name == name && bucket.kind == kind)
            {
                last.len += 1;
            } else {
                buckets.push(Bucket {
                    name,
                    kind,
                    start: rows.len(),
                    len: 1,
                    more_precision: self.actor_precision.decimal_places(name),
                });
            }
            rows.push(row);
        }
        Ok(CompiledActorModifiers {
            binding: self.actor_binding.clone(),
            rows,
            buckets,
            requires_downstream_defences: records
                .iter()
                .any(|record| requires_downstream_defences(record.stat)),
        })
    }
    /// Calculate a fresh actor from already compiled ordered source fragments.
    /// Successful dispatch, input validation, queries and output construction do
    /// not allocate. All programs must belong to this exact compiled instance.
    pub fn evaluate_actor_resources(
        &self,
        level: u32,
        quests: ActorQuestSelection,
        character: &CharacterInput,
        layers: &[ActorModifierLayer<'_>],
        scratch: &mut ActorScratch,
    ) -> Result<PreparedActorResources, ActorError> {
        scratch.reset();
        character.validate().map_err(|error| ActorError(error.0))?;
        if !(1..=100).contains(&level) {
            return Err(ActorError("Actor character level must be 1..100"));
        }
        if layers.len() > MAX_LAYERS {
            return Err(ActorError(
                "Actor input exceeds sixteen layers or 512 normalized records",
            ));
        }
        let mut count = 0usize;
        let mut programs = 0usize;
        let mut requires_downstream_defences = false;
        for layer in layers {
            programs = programs
                .checked_add(layer.programs.len())
                .ok_or(ActorError("Actor input exceeds 256 program references"))?;
            if programs > MAX_PROGRAM_REFERENCES {
                return Err(ActorError("Actor input exceeds 256 program references"));
            }
            for program in layer.programs {
                if !Arc::ptr_eq(&program.binding, &self.actor_binding) {
                    return Err(ActorError(
                        "Actor program belongs to a different compiled dataset",
                    ));
                }
                count = count.checked_add(program.record_count()).ok_or(ActorError(
                    "Actor input exceeds sixteen layers or 512 normalized records",
                ))?;
                if count > MAX_RECORDS {
                    return Err(ActorError(
                        "Actor input exceeds sixteen layers or 512 normalized records",
                    ));
                }
                requires_downstream_defences |= program.requires_downstream_defences;
            }
        }
        scratch.base = self.actor_base_records(level, quests, character);
        scratch.base_len = scratch.base.len;
        let output = calculate(&mut ProgramQueries { layers, scratch }, self)?;
        Ok(PreparedActorResources {
            binding: self.actor_binding.clone(),
            level,
            quests,
            character: *character,
            output,
            requires_downstream_defences,
        })
    }
}
