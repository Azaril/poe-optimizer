//! Bounded lexical value conversion for injected import policy data.
//!
//! This module has no source-format, draft, default or precedence dependency.
//! A present malformed value returns an error; choosing a different source value
//! is the responsibility of a separate, explicitly versioned import policy.

use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::{
        BoundedInteger, FiniteQuantity, GameVersionNamespace, OptionDefId, UnitDefId,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, btree_map::Entry};

pub const HARD_VALUE_SOURCE_BYTES: usize = 1024 * 1024;
pub const HARD_VALUE_TOKEN_BYTES: usize = 64 * 1024;
pub const HARD_VALUE_TOKENS: usize = 65_536;
pub const HARD_VALUE_TOTAL_TOKEN_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OwnedValueLimits {
    pub max_source_bytes: usize,
    pub max_token_bytes: usize,
    pub max_tokens: usize,
    pub max_total_token_bytes: usize,
}
impl Default for OwnedValueLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 64 * 1024,
            max_token_bytes: 16 * 1024,
            max_tokens: 4096,
            max_total_token_bytes: 1024 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueResource {
    SourceBytes,
    TokenBytes,
    Tokens,
    TotalTokenBytes,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhitespacePolicy {
    Exact,
    TrimAscii,
}
impl WhitespacePolicy {
    pub(crate) fn apply(self, source: &str) -> &str {
        match self {
            Self::Exact => source,
            Self::TrimAscii => source.trim_ascii(),
        }
    }
}

/// All grammars admit an optional leading + or - and require at least one digit.
/// Decimal admits 1, 1., .1 and 1.1. Scientific additionally admits e/E followed
/// by an optional sign and one or more decimal digits. No other syntax is implied.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecimalSyntax {
    Integer,
    Decimal,
    Scientific,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanToken {
    pub token: String,
    pub value: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionToken {
    pub token: String,
    pub value: OptionDefId,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RationalScale {
    pub numerator: BoundedInteger,
    pub denominator: BoundedInteger,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ValueCodecKind {
    Boolean {
        tokens: Vec<BooleanToken>,
    },
    Integer {
        syntax: DecimalSyntax,
    },
    Quantity {
        syntax: DecimalSyntax,
        unit: UnitDefId,
        scale: RationalScale,
    },
    Option {
        tokens: Vec<OptionToken>,
    },
}

/// Raw policy DTO, not a validated codec. Outer artifact decoders must bound the
/// input bytes before deserialization; new() then validates this DTO's resources.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValueCodecInput {
    pub namespace: GameVersionNamespace,
    pub whitespace: WhitespacePolicy,
    pub codec: ValueCodecKind,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ValueCodecError {
    #[error("invalid {resource:?} limit; it must be positive and at most {maximum}")]
    InvalidLimit {
        resource: ValueResource,
        maximum: usize,
    },
    #[error("codec {resource:?} is {actual}, exceeding {maximum}")]
    ResourceLimit {
        resource: ValueResource,
        actual: usize,
        maximum: usize,
    },
    #[error("token rows {first} and {second} collide under the declared whitespace policy")]
    DuplicateToken { first: usize, second: usize },
    #[error("option token {index} belongs to another owned namespace")]
    ForeignOptionNamespace { index: usize },
    #[error("quantity unit belongs to another owned namespace")]
    ForeignUnitNamespace,
    #[error("rational scale denominator must be positive")]
    InvalidScaleDenominator,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ValueDecodeError {
    #[error("source value is {actual} bytes, exceeding {maximum}")]
    SourceTooLarge { actual: usize, maximum: usize },
    #[error("present token is absent from the exact token table")]
    UnknownToken,
    #[error("present value does not match the declared decimal grammar")]
    MalformedDecimal,
    #[error("exact decimal value is not an integer")]
    NonIntegralInteger,
    #[error("exact integer lies outside the owned integer bounds")]
    IntegerOutOfRange,
    #[error("quantity input is not representable as a finite number")]
    NonFiniteInput,
    #[error("scaled quantity is not finite")]
    NonFiniteResult,
}

/// Immutable, indexed lexical conversion. Tables preserve their authored rows;
/// lookup keys apply only the explicitly selected whitespace transformation.
#[derive(Clone, Debug)]
pub struct OwnedValueCodec {
    input: ValueCodecInput,
    limits: OwnedValueLimits,
    tokens: BTreeMap<String, (usize, ParameterValue)>,
}
impl OwnedValueCodec {
    pub fn new(input: ValueCodecInput, limits: OwnedValueLimits) -> Result<Self, ValueCodecError> {
        validate_limits(limits)?;
        let mut tokens = BTreeMap::new();
        match &input.codec {
            ValueCodecKind::Boolean { tokens: rows } => {
                validate_tokens(
                    rows.iter().map(|row| row.token.as_str()),
                    rows.len(),
                    limits,
                )?;
                for (index, row) in rows.iter().enumerate() {
                    insert_token(
                        &mut tokens,
                        input.whitespace.apply(&row.token),
                        index,
                        ParameterValue::Boolean(row.value),
                    )?;
                }
            }
            ValueCodecKind::Option { tokens: rows } => {
                validate_tokens(
                    rows.iter().map(|row| row.token.as_str()),
                    rows.len(),
                    limits,
                )?;
                for (index, row) in rows.iter().enumerate() {
                    if row.value.namespace() != &input.namespace {
                        return Err(ValueCodecError::ForeignOptionNamespace { index });
                    }
                    insert_token(
                        &mut tokens,
                        input.whitespace.apply(&row.token),
                        index,
                        ParameterValue::Option(row.value.clone()),
                    )?;
                }
            }
            ValueCodecKind::Quantity { unit, scale, .. } => {
                if unit.namespace() != &input.namespace {
                    return Err(ValueCodecError::ForeignUnitNamespace);
                }
                if scale.denominator.get() <= 0 {
                    return Err(ValueCodecError::InvalidScaleDenominator);
                }
            }
            ValueCodecKind::Integer { .. } => {}
        }
        Ok(Self {
            input,
            limits,
            tokens,
        })
    }

    pub fn input(&self) -> &ValueCodecInput {
        &self.input
    }

    pub fn decode(&self, source: &str) -> Result<ParameterValue, ValueDecodeError> {
        // Check the original input before trimming or allocating a lookup key.
        if source.len() > self.limits.max_source_bytes {
            return Err(ValueDecodeError::SourceTooLarge {
                actual: source.len(),
                maximum: self.limits.max_source_bytes,
            });
        }
        let source = self.input.whitespace.apply(source);
        match &self.input.codec {
            ValueCodecKind::Boolean { .. } | ValueCodecKind::Option { .. } => self
                .tokens
                .get(source)
                .map(|(_, value)| value.clone())
                .ok_or(ValueDecodeError::UnknownToken),
            ValueCodecKind::Integer { syntax } => Ok(ParameterValue::Integer(
                parse_decimal(source, *syntax)?.integer()?,
            )),
            ValueCodecKind::Quantity {
                syntax,
                unit,
                scale,
            } => {
                parse_decimal(source, *syntax)?;
                let value = source
                    .parse::<f64>()
                    .map_err(|_| ValueDecodeError::MalformedDecimal)?;
                if !value.is_finite() {
                    return Err(ValueDecodeError::NonFiniteInput);
                }
                // Core integers are exactly representable as f64. Divide the scale
                // before multiplication to avoid an unnecessary overflowing product.
                let ratio = scale.numerator.get() as f64 / scale.denominator.get() as f64;
                let result = value * ratio;
                FiniteQuantity::new(result, unit.clone())
                    .map(ParameterValue::Quantity)
                    .map_err(|_| ValueDecodeError::NonFiniteResult)
            }
        }
    }
}

impl OwnedValueLimits {
    /// Validate tighten-only lexical bounds, including for empty policy packages.
    pub fn validate(self) -> Result<(), ValueCodecError> {
        validate_limits(self)
    }
}
fn validate_limits(limits: OwnedValueLimits) -> Result<(), ValueCodecError> {
    for (resource, value, maximum) in [
        (
            ValueResource::SourceBytes,
            limits.max_source_bytes,
            HARD_VALUE_SOURCE_BYTES,
        ),
        (
            ValueResource::TokenBytes,
            limits.max_token_bytes,
            HARD_VALUE_TOKEN_BYTES,
        ),
        (ValueResource::Tokens, limits.max_tokens, HARD_VALUE_TOKENS),
        (
            ValueResource::TotalTokenBytes,
            limits.max_total_token_bytes,
            HARD_VALUE_TOTAL_TOKEN_BYTES,
        ),
    ] {
        if value == 0 || value > maximum {
            return Err(ValueCodecError::InvalidLimit { resource, maximum });
        }
    }
    Ok(())
}
fn check_resource(
    resource: ValueResource,
    actual: usize,
    maximum: usize,
) -> Result<(), ValueCodecError> {
    if actual > maximum {
        return Err(ValueCodecError::ResourceLimit {
            resource,
            actual,
            maximum,
        });
    }
    Ok(())
}
fn validate_tokens<'a>(
    tokens: impl Iterator<Item = &'a str>,
    count: usize,
    limits: OwnedValueLimits,
) -> Result<(), ValueCodecError> {
    check_resource(ValueResource::Tokens, count, limits.max_tokens)?;
    let mut total = 0usize;
    for token in tokens {
        check_resource(
            ValueResource::TokenBytes,
            token.len(),
            limits.max_token_bytes,
        )?;
        // A previous iteration has proved total <= its small hard ceiling.
        total += token.len();
        check_resource(
            ValueResource::TotalTokenBytes,
            total,
            limits.max_total_token_bytes,
        )?;
    }
    Ok(())
}
fn insert_token(
    tokens: &mut BTreeMap<String, (usize, ParameterValue)>,
    token: &str,
    index: usize,
    value: ParameterValue,
) -> Result<(), ValueCodecError> {
    match tokens.entry(token.to_owned()) {
        Entry::Vacant(entry) => {
            entry.insert((index, value));
            Ok(())
        }
        Entry::Occupied(entry) => Err(ValueCodecError::DuplicateToken {
            first: entry.get().0,
            second: index,
        }),
    }
}

struct Decimal<'a> {
    negative: bool,
    whole: &'a [u8],
    fraction: &'a [u8],
    exponent: i64,
}
impl Decimal<'_> {
    fn digits(&self) -> impl DoubleEndedIterator<Item = u8> + '_ {
        self.whole.iter().chain(self.fraction.iter()).copied()
    }
    fn integer(&self) -> Result<BoundedInteger, ValueDecodeError> {
        let count = self.whole.len() + self.fraction.len();
        let leading = self.digits().take_while(|digit| *digit == b'0').count();
        if leading == count {
            return Ok(BoundedInteger::new(0).expect("zero is an owned integer"));
        }
        let scale = self.exponent - self.fraction.len() as i64;
        let remove = if scale < 0 {
            let remove = (-scale) as usize;
            let trailing = self
                .digits()
                .rev()
                .take_while(|digit| *digit == b'0')
                .count();
            if remove > trailing {
                return Err(ValueDecodeError::NonIntegralInteger);
            }
            remove
        } else {
            0
        };
        let append = scale.max(0) as usize;
        let significant = count - leading - remove + append;
        if significant > BoundedInteger::MAX.ilog10() as usize + 1 {
            return Err(ValueDecodeError::IntegerOutOfRange);
        }
        let mut magnitude = 0_i64;
        for digit in self
            .digits()
            .take(count - remove)
            .chain(std::iter::repeat_n(b'0', append))
        {
            magnitude = magnitude
                .checked_mul(10)
                .and_then(|value| value.checked_add(i64::from(digit - b'0')))
                .filter(|value| *value <= BoundedInteger::MAX)
                .ok_or(ValueDecodeError::IntegerOutOfRange)?;
        }
        let value = if self.negative { -magnitude } else { magnitude };
        BoundedInteger::new(value).map_err(|_| ValueDecodeError::IntegerOutOfRange)
    }
}

fn parse_decimal(source: &str, syntax: DecimalSyntax) -> Result<Decimal<'_>, ValueDecodeError> {
    let (value, length) = parse_decimal_prefix(source, syntax)?;
    if length != source.len() {
        return Err(ValueDecodeError::MalformedDecimal);
    }
    Ok(value)
}

/// Shared lexical grammar for item-pattern matching. No integer, unit, scale or
/// finite-value conversion participates in choosing a structural rule match.
pub(crate) fn numeric_prefix_length(source: &str, syntax: DecimalSyntax) -> Option<usize> {
    parse_decimal_prefix(source, syntax)
        .ok()
        .map(|(_, length)| length)
}

fn parse_decimal_prefix(
    source: &str,
    syntax: DecimalSyntax,
) -> Result<(Decimal<'_>, usize), ValueDecodeError> {
    let bytes = source.as_bytes();
    let mut cursor = 0usize;
    let negative = bytes.first() == Some(&b'-');
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        cursor += 1;
    }
    let start = cursor;
    while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
        cursor += 1;
    }
    let whole = &bytes[start..cursor];
    let mut fraction = &bytes[cursor..cursor];
    if bytes.get(cursor) == Some(&b'.') && syntax != DecimalSyntax::Integer {
        cursor += 1;
        let start = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        fraction = &bytes[start..cursor];
    }
    if whole.is_empty() && fraction.is_empty() {
        return Err(ValueDecodeError::MalformedDecimal);
    }
    let mut exponent = 0_i64;
    if matches!(bytes.get(cursor), Some(b'e' | b'E')) && syntax == DecimalSyntax::Scientific {
        cursor += 1;
        let negative_exponent = bytes.get(cursor) == Some(&b'-');
        if matches!(bytes.get(cursor), Some(b'+' | b'-')) {
            cursor += 1;
        }
        let start = cursor;
        // Exponents beyond this cap cannot be canceled by the bounded mantissa.
        // Saturation preserves integral/range decisions without allocating a bigint.
        let cap = bytes.len() as i64 + 64;
        while let Some(digit) = bytes.get(cursor).filter(|digit| digit.is_ascii_digit()) {
            exponent = exponent
                .saturating_mul(10)
                .saturating_add(i64::from(*digit - b'0'))
                .min(cap);
            cursor += 1;
        }
        if cursor == start {
            return Err(ValueDecodeError::MalformedDecimal);
        }
        if negative_exponent {
            exponent = -exponent;
        }
    }
    Ok((
        Decimal {
            negative,
            whole,
            fraction,
            exponent,
        },
        cursor,
    ))
}
