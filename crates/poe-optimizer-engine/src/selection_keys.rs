//! Numeric key occupancy and length for fresh, insert-only LuaJIT tables.
//!
//! PoB skill/configuration set constructors choose an absent ID with `#sets + 1`.
//! Sparse table length depends on insertion history and the array/hash split, so
//! neither the largest key nor the first absent positive integer is equivalent.
//! This module models only empty tables receiving numeric keys and non-nil
//! values. It does not model deletion, preallocated/table-literal entries, other
//! key types, metamethods, traversal order, or JIT length hints.
//!
//! Resize bins and length searches are adapted from `lj_tab.c` / `lj_tab.h` in
//! bundled `luajit-src 210.7.3+1ee778a` (LuaJIT commit `1ee778a`). The reduction to
//! occupancy counts is valid because an insert-only hash table has no dead keys:
//! a new out-of-array key triggers rehash exactly when every hash slot is used.
//! Hash placement cannot change that trigger or the integer-key resize bins.
//!
//! Copyright (C) 2005-2026 Mike Pall. All rights reserved.
//! Copyright (C) 1994-2012 Lua.org, PUC-Rio.
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in
//! all copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
//! THE SOFTWARE.

use crate::lua_number::parse_number;
use std::{collections::BTreeSet, error::Error, fmt};

const MAX_ARRAY_BITS: usize = 28;
const MAX_ARRAY_SIZE: usize = (1 << (MAX_ARRAY_BITS - 1)) + 1;
const MAX_HASH_SIZE: usize = 1 << 26;
const LENGTH_LINEAR_THRESHOLD: usize = 0x7fff_fffd;

/// An invalid source key and a native resource limit are deliberately distinct.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericSetKeyError {
    /// The source text converts to Lua nil rather than a numeric key.
    NotNumber,
    /// Lua rejects NaN as a table assignment key.
    NanKey,
    /// The caller's maximum number of distinct keys would be exceeded.
    KeyLimit { maximum: usize },
    /// The pinned source runtime would reject the required table allocation size.
    TableOverflow,
}

impl fmt::Display for NumericSetKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotNumber => f.write_str("set ID is not a Lua number"),
            Self::NanKey => f.write_str("table index is NaN"),
            Self::KeyLimit { maximum } => write!(f, "numeric set key limit {maximum} exceeded"),
            Self::TableOverflow => f.write_str("LuaJIT table size limit exceeded"),
        }
    }
}
impl Error for NumericSetKeyError {}

/// An insertion-history-sensitive numeric key set, with a caller supplied cap.
///
/// Numeric equality is exact IEEE equality, with positive and negative zero
/// sharing one key. Noninteger, negative and infinite keys consume hash slots.
/// Inserting an existing key returns `false` without changing table layout.
/// Space is O(distinct keys); each insertion uses O(log(keys) + 28) work.
/// No source-sized array or hash-slot allocation is performed.
#[derive(Clone, Debug)]
pub struct NumericSetKeys {
    keys: BTreeSet<u64>,
    bins: [usize; MAX_ARRAY_BITS],
    array_size: usize,
    hash_size: usize,
    hash_used: usize,
    max_keys: usize,
}

impl NumericSetKeys {
    /// Construct the equivalent of an unhinted, empty Lua `{}` table.
    pub fn new(max_keys: usize) -> Self {
        Self {
            keys: BTreeSet::new(),
            bins: [0; MAX_ARRAY_BITS],
            array_size: 0,
            hash_size: 0,
            hash_used: 0,
            max_keys,
        }
    }

    /// Number of distinct assigned keys, independently of Lua's `#` operator.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Query numeric equality. NaN is never present.
    pub fn contains(&self, value: f64) -> bool {
        !value.is_nan() && self.keys.contains(&key_bits(value))
    }

    /// Apply the pinned one-argument `tonumber` conversion, then assign a key.
    /// The caller bounds source bytes before invoking this linear-time parser.
    pub fn insert_text(&mut self, value: &[u8]) -> Result<bool, NumericSetKeyError> {
        self.insert(parse_number(value).ok_or(NumericSetKeyError::NotNumber)?)
    }

    /// Assign a non-nil value. Errors leave occupancy and layout unchanged.
    pub fn insert(&mut self, value: f64) -> Result<bool, NumericSetKeyError> {
        if value.is_nan() {
            return Err(NumericSetKeyError::NanKey);
        }
        let bits = key_bits(value);
        if self.keys.contains(&bits) {
            return Ok(false);
        }
        if self.keys.len() >= self.max_keys {
            return Err(NumericSetKeyError::KeyLimit {
                maximum: self.max_keys,
            });
        }
        let mut bins = self.bins;
        let array_key = array_key(value);
        if let Some(key) = array_key {
            // Source groups 0, 1, 2 together, then (2, 4], (4, 8], ... .
            let bin = if key <= 2 {
                0
            } else {
                (usize::BITS - 1 - (key - 1).leading_zeros()) as usize
            };
            bins[bin] += 1;
        }
        let mut array_size = self.array_size;
        let mut hash_size = self.hash_size;
        let mut hash_used = self.hash_used;
        if !array_key.is_some_and(|key| key < array_size) {
            if hash_used == hash_size {
                let (new_array_size, array_used) = best_array_size(&bins);
                array_size = new_array_size;
                hash_used = self.keys.len() + 1 - array_used;
                hash_size = hash_capacity(hash_used)?;
                if array_size > MAX_ARRAY_SIZE {
                    return Err(NumericSetKeyError::TableOverflow);
                }
            } else {
                hash_used += 1;
            }
        }
        self.keys.insert(bits);
        self.bins = bins;
        self.array_size = array_size;
        self.hash_size = hash_size;
        self.hash_used = hash_used;
        Ok(true)
    }

    /// Evaluate the pinned interpreter's table length for the current layout.
    ///
    /// This is a non-nil to nil boundary, not necessarily the contiguous prefix.
    /// Binary/widening searches perform at most 64 probes before LuaJIT's rare
    /// overflow fallback, whose linear scan is bounded by `len() + 1` probes.
    /// Each probe is O(log(keys)); no memory is allocated.
    pub fn sequence_length(&self) -> usize {
        let hi = self.array_size.saturating_sub(1);
        if hi > 0 && !self.has_integer(hi) {
            return self.binary_boundary(0, hi);
        }
        if self.hash_size == 0 {
            return hi;
        }
        let mut lo = hi;
        let mut hi = hi + 1;
        while self.has_integer(hi) {
            lo = hi;
            hi *= 2;
            if hi > LENGTH_LINEAR_THRESHOLD {
                let mut lo = 1;
                while self.has_integer(lo) {
                    lo += 1;
                }
                return lo - 1;
            }
        }
        self.binary_boundary(lo, hi)
    }

    fn has_integer(&self, key: usize) -> bool {
        self.keys.contains(&(key as f64).to_bits())
    }

    fn binary_boundary(&self, mut lo: usize, mut hi: usize) -> usize {
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if self.has_integer(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo
    }
}

fn hash_capacity(used: usize) -> Result<usize, NumericSetKeyError> {
    if used > MAX_HASH_SIZE {
        return Err(NumericSetKeyError::TableOverflow);
    }
    Ok(match used {
        0 => 0,
        1 => 2,
        n => n.next_power_of_two(),
    })
}

fn key_bits(value: f64) -> u64 {
    if value == 0.0 { 0 } else { value.to_bits() }
}

fn array_key(value: f64) -> Option<usize> {
    if (0.0..MAX_ARRAY_SIZE as f64).contains(&value) && value.fract() == 0.0 {
        Some(value as usize)
    } else {
        None
    }
}

fn best_array_size(bins: &[usize; MAX_ARRAY_BITS]) -> (usize, usize) {
    let total: usize = bins.iter().sum();
    let mut sum = 0;
    let mut size = 0;
    let mut used = 0;
    for (bit, count) in bins.iter().enumerate() {
        let threshold = 1usize << bit;
        if 2 * total <= threshold || sum == total {
            break;
        }
        if *count > 0 {
            sum += count;
            if 2 * sum > threshold {
                size = 2 * threshold + 1;
                used = sum;
            }
        }
    }
    (size, used)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_length_is_a_layout_boundary() {
        for (keys, expected) in [
            (vec![1.0, 3.0], 1),
            (vec![2.0, 8.0], 2),
            (vec![-2.5, 0.0], 0),
            (vec![1.0, 2.0, 4.0], 4),
        ] {
            let mut state = NumericSetKeys::new(32);
            for key in keys {
                state.insert(key).unwrap();
            }
            assert_eq!(state.sequence_length(), expected);
        }
    }

    #[test]
    fn numeric_identity_and_rejection_do_not_mutate_layout() {
        let mut keys = NumericSetKeys::new(4);
        assert!(keys.insert_text(b"-0").unwrap());
        assert!(!keys.insert_text(b"0x0").unwrap());
        assert!(keys.insert(f64::INFINITY).unwrap());
        assert!(keys.insert(f64::NEG_INFINITY).unwrap());
        assert!(keys.insert(-2.5).unwrap());
        assert!(keys.contains(-0.0));
        assert!(!keys.contains(f64::NAN));
        assert_eq!(keys.len(), 4);
        for (value, error) in [
            (b"nan".as_slice(), NumericSetKeyError::NanKey),
            (b"invalid", NumericSetKeyError::NotNumber),
            (b"1", NumericSetKeyError::KeyLimit { maximum: 4 }),
        ] {
            assert_eq!(keys.insert_text(value), Err(error));
            assert_eq!(keys.len(), 4);
            assert_eq!(keys.sequence_length(), 0);
        }
        assert!(!keys.insert(f64::INFINITY).unwrap());
        assert_eq!(
            NumericSetKeys::new(0).insert(1.0),
            Err(NumericSetKeyError::KeyLimit { maximum: 0 })
        );
    }

    #[test]
    fn rejected_key_does_not_change_later_resize_history() {
        let mut limited = NumericSetKeys::new(3);
        let mut control = NumericSetKeys::new(16);
        for key in [1.0, 2.0, 4.0] {
            limited.insert(key).unwrap();
            control.insert(key).unwrap();
        }
        let before = limited.clone();
        assert_eq!(
            limited.insert(8.0),
            Err(NumericSetKeyError::KeyLimit { maximum: 3 })
        );
        assert_eq!(limited.keys, before.keys);
        assert_eq!(limited.bins, before.bins);
        assert_eq!(limited.array_size, before.array_size);
        assert_eq!(limited.hash_size, before.hash_size);
        assert_eq!(limited.hash_used, before.hash_used);
        // Raising this private test cap permits comparison with a history in
        // which the rejected operation was never attempted.
        limited.max_keys = 16;
        for key in [8.0, -2.0, 3.0, 0.0, 16.0] {
            assert_eq!(limited.insert(key), control.insert(key));
            assert_eq!(limited.sequence_length(), control.sequence_length());
        }
    }

    #[test]
    fn integer_bin_limits_and_source_capacity_are_explicit() {
        for (used, capacity) in [
            (0, 0),
            (1, 2),
            (2, 2),
            (3, 4),
            (MAX_HASH_SIZE, MAX_HASH_SIZE),
        ] {
            assert_eq!(hash_capacity(used), Ok(capacity));
        }
        for used in [MAX_HASH_SIZE + 1, usize::MAX] {
            assert_eq!(hash_capacity(used), Err(NumericSetKeyError::TableOverflow));
        }
        assert_eq!(array_key(-0.0), Some(0));
        assert_eq!(
            array_key((MAX_ARRAY_SIZE - 1) as f64),
            Some(MAX_ARRAY_SIZE - 1)
        );
        for value in [-1.0, 1.5, MAX_ARRAY_SIZE as f64, f64::INFINITY, f64::NAN] {
            assert_eq!(array_key(value), None);
        }
        let mut bins = [0; MAX_ARRAY_BITS];
        bins[0] = 1;
        assert_eq!(best_array_size(&bins), (3, 1));
        bins[1] = 1;
        assert_eq!(best_array_size(&bins), (5, 2));
    }
}
