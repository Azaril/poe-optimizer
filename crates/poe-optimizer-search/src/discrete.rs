//! Bounded coupled proposals over finite axes. Game legality remains the adapter's job.
use crate::EvaluationControl;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscretePoint {
    pub space_id: String,
    pub choices: Vec<u32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DiscreteSpace {
    id: String,
    cardinalities: Vec<u32>,
    locks: BTreeMap<usize, u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Neighborhood {
    pub proposals_per_round: usize,
    /// Every nth round reserves one in four proposal attempts for full random restarts.
    pub restart_every: usize,
    /// Zero allows all mutable axes; otherwise a positive radius cap.
    pub max_changed_axes: usize,
}
impl Default for Neighborhood {
    fn default() -> Self {
        Self {
            proposals_per_round: 64,
            restart_every: 4,
            max_changed_axes: 0,
        }
    }
}
impl Neighborhood {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=4096).contains(&self.proposals_per_round)
            || self.restart_every == 0
            || self.max_changed_axes > 128
        {
            return Err("Neighborhood requires 1..4096 proposals, a positive restart interval and at most128 changed axes".into());
        }
        Ok(())
    }
}
impl DiscreteSpace {
    /// Identity must bind the ordered axis catalogs and fixed choices in the host's run data.
    pub fn new(
        id: String,
        cardinalities: Vec<u32>,
        locks: BTreeMap<usize, u32>,
    ) -> Result<Self, String> {
        if id.trim().is_empty()
            || id.len() > 256
            || cardinalities.is_empty()
            || cardinalities.len() > 128
            || cardinalities.contains(&0)
        {
            return Err("A discrete space needs an identity and1..128 nonempty axes".into());
        }
        if locks
            .iter()
            .any(|(axis, choice)| cardinalities.get(*axis).is_none_or(|n| choice >= n))
        {
            return Err("Fixed choice lies outside its axis".into());
        }
        Ok(Self {
            id,
            cardinalities,
            locks,
        })
    }
    pub fn cardinalities(&self) -> &[u32] {
        &self.cardinalities
    }
    pub fn point(&self, choices: Vec<u32>) -> Result<DiscretePoint, String> {
        let point = DiscretePoint {
            space_id: self.id.clone(),
            choices,
        };
        self.validate(&point)?;
        Ok(point)
    }
    pub fn validate(&self, point: &DiscretePoint) -> Result<(), String> {
        if point.space_id != self.id
            || point.choices.len() != self.cardinalities.len()
            || point
                .choices
                .iter()
                .zip(&self.cardinalities)
                .any(|(choice, n)| choice >= n)
            || self
                .locks
                .iter()
                .any(|(axis, choice)| point.choices[*axis] != *choice)
        {
            return Err(
                "Point has a different space, invalid choice or violated fixed axis".into(),
            );
        }
        Ok(())
    }
    pub fn first(&self) -> DiscretePoint {
        let mut choices = vec![0; self.cardinalities.len()];
        for (axis, choice) in &self.locks {
            choices[*axis] = *choice;
        }
        DiscretePoint {
            space_id: self.id.clone(),
            choices,
        }
    }
    /// None means the product exceeds u128, not that the space is empty.
    pub fn size(&self) -> Option<u128> {
        self.cardinalities
            .iter()
            .enumerate()
            .try_fold(1u128, |size, (axis, n)| {
                size.checked_mul(if self.locks.contains_key(&axis) {
                    1
                } else {
                    u128::from(*n)
                })
            })
    }
    /// Only explicit, bounded enumeration may establish full finite-domain coverage.
    pub fn enumerate(&self, max_states: usize) -> Result<Vec<DiscretePoint>, String> {
        let size = self
            .size()
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| *n <= max_states)
            .ok_or("Discrete domain exceeds enumeration limit")?;
        let mut points = Vec::new();
        points.try_reserve_exact(size).map_err(|e| e.to_string())?;
        let mut current = self.first();
        for _ in 0..size {
            points.push(current.clone());
            for axis in (0..self.cardinalities.len()).rev() {
                if self.locks.contains_key(&axis) {
                    continue;
                }
                current.choices[axis] += 1;
                if current.choices[axis] < self.cardinalities[axis] {
                    break;
                }
                current.choices[axis] = 0;
            }
        }
        Ok(points)
    }
    /// Proposal radius cycles from one through the configured maximum. Sampling is
    /// deterministic for identical ordered parents/settings/seed, without enumerating products.
    /// Empty output means no sampled new neighbor; it never certifies exhaustive coverage.
    pub fn propose(
        &self,
        parents: &[DiscretePoint],
        round: usize,
        seed: u64,
        limit: usize,
        settings: &Neighborhood,
        control: &EvaluationControl<'_>,
    ) -> Result<Vec<DiscretePoint>, String> {
        settings.validate()?;
        if round == 0 {
            return Err("Proposal rounds start at one".into());
        }
        for parent in parents {
            self.validate(parent)?;
        }
        let count = limit.min(settings.proposals_per_round);
        if count == 0 || control.should_stop() {
            return Ok(vec![]);
        }
        let axes: Vec<_> = self
            .cardinalities
            .iter()
            .enumerate()
            .filter(|(axis, n)| **n > 1 && !self.locks.contains_key(axis))
            .map(|(axis, _)| axis)
            .collect();
        if axes.is_empty() {
            return Ok(if parents.is_empty() {
                vec![self.first()]
            } else {
                vec![]
            });
        }
        let max_radius = if settings.max_changed_axes == 0 {
            axes.len()
        } else {
            axes.len().min(settings.max_changed_axes)
        };
        let radius = 1 + (round - 1) % max_radius;
        let mut random = Random(seed);
        let mut output = BTreeSet::new();
        let parent_set: BTreeSet<_> = parents.iter().collect();
        for attempt in 0..count.saturating_mul(4) {
            if output.len() == count || control.should_stop() {
                break;
            }
            let mut point = if parents.is_empty() {
                self.first()
            } else {
                parents[attempt % parents.len()].clone()
            };
            if parents.is_empty()
                || (round.is_multiple_of(settings.restart_every) && attempt.is_multiple_of(4))
            {
                for axis in &axes {
                    point.choices[*axis] = random.below(self.cardinalities[*axis]);
                }
            } else {
                let mut shuffled = axes.clone();
                for index in 0..radius {
                    let other = index + random.below((shuffled.len() - index) as u32) as usize;
                    shuffled.swap(index, other);
                    let axis = shuffled[index];
                    let old = point.choices[axis];
                    let chosen = random.below(self.cardinalities[axis] - 1);
                    point.choices[axis] = chosen + u32::from(chosen >= old);
                }
            }
            if !parent_set.contains(&point) {
                output.insert(point);
            }
        }
        Ok(output.into_iter().collect())
    }
}
struct Random(u64);
impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    fn below(&mut self, upper: u32) -> u32 {
        // Multiply-high maps a u32 draw without float/platform differences. The tiny
        // finite modulo bias is irrelevant to this heuristic; this is not cryptographic.
        ((u64::from(self.next() as u32) * u64::from(upper)) >> 32) as u32
    }
}
