//! Validated, untagged numeric subset of the pinned PoB `ModDB` query semantics.
//!
//! This is not a `ModStore::EvalMod` implementation or an importer. Callers must
//! supply every modifier and preserve its tags/value kind so unsupported input
//! fails validation. Layer zero is the queried database; later layers are its
//! successive parents. Ordering affects floating-point sums and overrides.

use std::{collections::BTreeMap, error::Error, fmt};

/// Bit 31 is excluded because upstream AND64 recombines a signed low word.
/// All other nonnegative, exactly representable 53-bit masks are supported.
pub const SUPPORTED_MOD_FLAG_BITS: u64 = ((1_u64 << 53) - 1) & !(1_u64 << 31);
pub const SUPPORTED_KEYWORD_FLAG_BITS: u64 = 0x7fff_ffff;
pub const KEYWORD_MATCH_ALL: u64 = 0x4000_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericKind {
    Base,
    Increased,
    More,
    Override,
}

impl NumericKind {
    pub const fn upstream_name(self) -> &'static str {
        match self {
            Self::Base => "BASE",
            Self::Increased => "INC",
            Self::More => "MORE",
            Self::Override => "OVERRIDE",
        }
    }
}

/// Keep unsupported kinds at the import boundary rather than discarding them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModifierKind {
    Numeric(NumericKind),
    Unsupported(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModifierValue {
    Number(f64),
    /// Examples: `boolean`, `table`, `function`, or `nil`.
    Unsupported {
        kind: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModifierInput {
    pub name: String,
    pub kind: ModifierKind,
    pub value: ModifierValue,
    pub flags: u64,
    pub keyword_flags: u64,
    pub source: Option<String>,
    /// Every tag is currently unsupported, including conditions and scopes.
    pub tag_kinds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumericModifier {
    name: String,
    kind: NumericKind,
    value: f64,
    flags: u64,
    keyword_flags: u64,
    source: Option<String>,
}

impl NumericModifier {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    pub const fn kind(&self) -> NumericKind {
        self.kind
    }

    pub const fn value(&self) -> f64 {
        self.value
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueryContext {
    pub flags: u64,
    pub keyword_flags: u64,
    /// Lua's cfg.source: BASE/INC accept the first colon-delimited component
    /// or the exact string. MORE/OVERRIDE only accept the first component.
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModifierError {
    UnsupportedKind {
        layer: usize,
        modifier: usize,
        kind: String,
    },
    UnsupportedValue {
        layer: usize,
        modifier: usize,
        kind: String,
    },
    UnsupportedTag {
        layer: usize,
        modifier: usize,
        tag: String,
    },
    UnsupportedFlags {
        flags: u64,
        keyword_flags: u64,
    },
    UnsupportedPrecision {
        name: String,
        decimal_places: u8,
    },
    TooManyNames {
        count: usize,
    },
    MissingSource {
        layer: usize,
        modifier: usize,
    },
}

impl fmt::Display for ModifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unsupported native modifier input: {self:?}")
    }
}

impl Error for ModifierError {}

/// Explicit game-data input controlling MORE truncation. Other names use
/// upstream nearest-two-decimal rounding. This does not configure BASE scaling.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MorePrecision {
    decimal_places: BTreeMap<String, u8>,
}

impl MorePrecision {
    pub fn try_new(decimal_places: BTreeMap<String, u8>) -> Result<Self, ModifierError> {
        for (name, places) in &decimal_places {
            if *places > 15 {
                return Err(ModifierError::UnsupportedPrecision {
                    name: name.clone(),
                    decimal_places: *places,
                });
            }
        }
        Ok(Self { decimal_places })
    }

    /// The complete MORE precision subset in pinned Modules/Data.lua.
    pub fn pinned() -> Self {
        Self {
            decimal_places: BTreeMap::from([
                ("SupportManaMultiplier".to_owned(), 4),
                ("ReservationMultiplier".to_owned(), 4),
            ]),
        }
    }
}

/// Immutable validated layers. No actor conditions, mutation, caches or Lua
/// objects live here. Unsupported entries anywhere in the input reject it all.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModifierDatabase {
    layers: Vec<Vec<NumericModifier>>,
}

impl ModifierDatabase {
    pub fn try_new(layers: Vec<Vec<ModifierInput>>) -> Result<Self, ModifierError> {
        let mut resolved = Vec::with_capacity(layers.len());
        for (layer, inputs) in layers.into_iter().enumerate() {
            let mut modifiers = Vec::with_capacity(inputs.len());
            for (modifier, input) in inputs.into_iter().enumerate() {
                if let Some(tag) = input.tag_kinds.into_iter().next() {
                    return Err(ModifierError::UnsupportedTag {
                        layer,
                        modifier,
                        tag,
                    });
                }
                let kind = match input.kind {
                    ModifierKind::Numeric(kind) => kind,
                    ModifierKind::Unsupported(kind) => {
                        return Err(ModifierError::UnsupportedKind {
                            layer,
                            modifier,
                            kind,
                        });
                    }
                };
                let value = match input.value {
                    ModifierValue::Number(value) => value,
                    ModifierValue::Unsupported { kind } => {
                        return Err(ModifierError::UnsupportedValue {
                            layer,
                            modifier,
                            kind,
                        });
                    }
                };
                validate_flags(input.flags, input.keyword_flags)?;
                modifiers.push(NumericModifier {
                    name: input.name,
                    kind,
                    value,
                    flags: input.flags,
                    keyword_flags: input.keyword_flags,
                    source: input.source,
                });
            }
            resolved.push(modifiers);
        }
        Ok(Self { layers: resolved })
    }

    pub fn layer(&self, index: usize) -> Option<&[NumericModifier]> {
        self.layers.get(index).map(Vec::as_slice)
    }

    /// Add BASE or INC modifiers in query-name, insertion, then parent order.
    pub fn sum(
        &self,
        kind: SumKind,
        context: &QueryContext,
        names: &[&str],
    ) -> Result<f64, ModifierError> {
        validate_query(context, names)?;
        let kind = match kind {
            SumKind::Base => NumericKind::Base,
            SumKind::Increased => NumericKind::Increased,
        };
        let mut parent_result = None;
        // This is a right fold: local_sum + parent_sum, not a flattened sum.
        for layer in self.layers.iter().rev() {
            let mut result = 0.0;
            for name in names {
                for modifier in layer {
                    if matches(modifier, kind, context, name)
                        && (context.source.is_none()
                            || modifier.source.as_deref().is_some_and(|source| {
                                Some(source) == context.source.as_deref()
                                    || source_prefix(source) == context.source.as_deref()
                            }))
                    {
                        result += modifier.value;
                    }
                }
            }
            if let Some(parent) = parent_result {
                result += parent;
            }
            parent_result = Some(result);
        }
        Ok(parent_result.unwrap_or(0.0))
    }

    /// Multiply MORE factors using upstream per-name rounding/truncation and
    /// independent parent-layer aggregation. The result is a multiplier.
    pub fn more(
        &self,
        context: &QueryContext,
        names: &[&str],
        precision: &MorePrecision,
    ) -> Result<f64, ModifierError> {
        validate_query(context, names)?;
        let mut local_results = Vec::with_capacity(self.layers.len());
        for (layer_index, layer) in self.layers.iter().enumerate() {
            let mut result = 1.0;
            // Upstream retains this across names but resets it across parents.
            let mut decimal_places = None;
            for name in names {
                let mut mod_result = 1.0;
                for (index, modifier) in layer.iter().enumerate() {
                    if matches(modifier, NumericKind::More, context, name)
                        && matches_prefix(modifier, context, layer_index, index)?
                    {
                        mod_result *= 1.0 + modifier.value / 100.0;
                        if let Some(places) = precision.decimal_places.get(*name) {
                            decimal_places = Some(decimal_places.unwrap_or(*places).max(*places));
                        }
                    }
                }
                if let Some(places) = decimal_places {
                    let power = 10_u64.pow(u32::from(places)) as f64;
                    result = (result * mod_result * power).floor() / power;
                } else {
                    result *= (mod_result * 100.0 + 0.5).floor() / 100.0;
                }
            }
            local_results.push(result);
        }
        let mut parent_result = None;
        for mut result in local_results.into_iter().rev() {
            if let Some(parent) = parent_result {
                result *= parent;
            }
            parent_result = Some(result);
        }
        Ok(parent_result.unwrap_or(1.0))
    }

    /// First matching local modifier in name/insertion order, then parents.
    /// A numeric zero is present, unlike an absent override.
    pub fn override_value(
        &self,
        context: &QueryContext,
        names: &[&str],
    ) -> Result<Option<f64>, ModifierError> {
        validate_query(context, names)?;
        for (layer_index, layer) in self.layers.iter().enumerate() {
            for name in names {
                for (index, modifier) in layer.iter().enumerate() {
                    if matches(modifier, NumericKind::Override, context, name)
                        && matches_prefix(modifier, context, layer_index, index)?
                    {
                        return Ok(Some(modifier.value));
                    }
                }
            }
        }
        Ok(None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SumKind {
    Base,
    Increased,
}

fn validate_flags(flags: u64, keyword_flags: u64) -> Result<(), ModifierError> {
    if flags & !SUPPORTED_MOD_FLAG_BITS != 0 || keyword_flags & !SUPPORTED_KEYWORD_FLAG_BITS != 0 {
        return Err(ModifierError::UnsupportedFlags {
            flags,
            keyword_flags,
        });
    }
    Ok(())
}

fn validate_query(context: &QueryContext, names: &[&str]) -> Result<(), ModifierError> {
    validate_flags(context.flags, context.keyword_flags)?;
    if names.len() > 8 {
        return Err(ModifierError::TooManyNames { count: names.len() });
    }
    Ok(())
}

fn matches(
    modifier: &NumericModifier,
    kind: NumericKind,
    context: &QueryContext,
    name: &str,
) -> bool {
    let keywords = modifier.keyword_flags & !KEYWORD_MATCH_ALL;
    let available = context.keyword_flags & !KEYWORD_MATCH_ALL;
    let keyword_match = if modifier.keyword_flags & KEYWORD_MATCH_ALL != 0 {
        available & keywords == keywords
    } else {
        keywords == 0 || available & keywords != 0
    };
    modifier.name == name
        && modifier.kind == kind
        && context.flags & modifier.flags == modifier.flags
        && keyword_match
}

fn source_prefix(source: &str) -> Option<&str> {
    source.split(':').find(|segment| !segment.is_empty())
}

fn matches_prefix(
    modifier: &NumericModifier,
    context: &QueryContext,
    layer: usize,
    index: usize,
) -> Result<bool, ModifierError> {
    let Some(required) = context.source.as_deref() else {
        return Ok(true);
    };
    let source = modifier
        .source
        .as_deref()
        .ok_or(ModifierError::MissingSource {
            layer,
            modifier: index,
        })?;
    Ok(source_prefix(source) == Some(required))
}
