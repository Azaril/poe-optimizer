//! Injected, bounded item-line declarations. This is not Item.ParseRaw or a game evaluator.
//! Header evidence and explicit modifier lines are separate from metadata. No occurrence
//! IDs are allocated here, and converted lines never prove whole-item coverage or legality.
use crate::owned_value::*;
use poe_optimizer_core::{
    data::DataIdentity,
    owned_build::{DeclaredSlot, ParameterAssignment, ParameterValue, QualitySelection},
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_schema::*,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod convert;
mod schema;

pub const OWNED_ITEM_LINE_POLICY_VERSION: u32 = 1;
const DOMAIN: &str = "owned-item-line-policy-v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemLinePolicyInput {
    pub schema_version: u32,
    pub namespace: GameVersionNamespace,
    pub version: OwnedDefinitionKey,
    pub definitions: DataIdentity,
    pub whitespace: WhitespacePolicy,
    pub rules: Vec<ItemLineRule>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemLineRule {
    pub id: OwnedDefinitionKey,
    pub pattern: Vec<ItemPatternPart>,
    pub captures: Vec<ItemCapture>,
    /// Empty means explicitly classified syntax, not unknown semantics.
    pub emissions: Vec<ItemEmission>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ItemPatternPart {
    Literal(String),
    Capture(OwnedDefinitionKey),
    /// Maximal ASCII numeric token; lexical matching precedes semantic decoding.
    /// It never retries a shorter token to satisfy a subsequent literal or codec.
    NumericCapture {
        capture: OwnedDefinitionKey,
        syntax: DecimalSyntax,
        sign: ItemNumericSign,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemNumericSign {
    Optional,
    OptionalMinus,
    Forbidden,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemCapture {
    pub id: OwnedDefinitionKey,
    pub codec: ItemCaptureCodec,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ItemCaptureCodec {
    Value(ValueCodecInput),
    OpaqueText,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemRangeRounding {
    Floor,
    Ceiling,
    Truncate,
    NearestTiesPositive,
    /// Literal signed half offset: x >= 0 uses floor(x + 0.5), otherwise
    /// ceil(x - 0.5). IEEE offset rounding is preserved, including next-down
    /// 0.5 rounding to 1 and 2^52 + 1 rounding to 2^52 + 2.
    SymmetricHalfOffset,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ItemLineValue {
    Literal(ParameterValue),
    Capture(OwnedDefinitionKey),
    /// Explicit f64 interpolation followed by rounding to a positive quantum.
    /// Endpoints and quantum must share Integer kind or an exact Quantity unit.
    /// A supplied range fraction in [0,1] is mandatory; no source/default lookup.
    Interpolate {
        lower: OwnedDefinitionKey,
        upper: OwnedDefinitionKey,
        quantum: ParameterValue,
        rounding: ItemRangeRounding,
    },
    /// Literal a + fraction * (b - a), then explicit quantum rounding. Every
    /// intermediate must remain finite, including the difference at endpoints.
    /// This is distinct from Interpolate's stable convex/endpoints arithmetic.
    InterpolateOffset {
        lower: OwnedDefinitionKey,
        upper: OwnedDefinitionKey,
        quantum: ParameterValue,
        rounding: ItemRangeRounding,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemRollTemplate {
    pub slot: DeclaredSlot<ParameterSlotDefId>,
    pub value: ItemLineValue,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ItemEmission {
    Metadata {
        role: OwnedDefinitionKey,
    },
    Template {
        definition: ItemTemplateDefId,
    },
    ItemLevel {
        value: ItemLineValue,
    },
    Quality {
        kind: QualityDefId,
        amount: ItemLineValue,
    },
    ItemParameter {
        slot: DeclaredSlot<ParameterSlotDefId>,
        value: ItemLineValue,
    },
    Modifier {
        definition: ModifierDefId,
        rolls: Vec<ItemRollTemplate>,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct ItemLineLimits {
    pub value: OwnedValueLimits,
    pub max_wire_bytes: usize,
    pub max_rules: usize,
    pub max_parts: usize,
    pub max_captures: usize,
    pub max_emissions: usize,
    pub max_rolls: usize,
    pub max_policy_text_bytes: usize,
    pub max_schema_work: usize,
    pub max_source_bytes: usize,
    pub max_line_bytes: usize,
    pub max_lines: usize,
    pub max_work: usize,
    pub max_output_declarations: usize,
}
impl Default for ItemLineLimits {
    fn default() -> Self {
        Self {
            value: OwnedValueLimits::default(),
            max_wire_bytes: 8 * 1024 * 1024,
            max_rules: 2048,
            max_parts: 16384,
            max_captures: 8192,
            max_emissions: 16384,
            max_rolls: 32768,
            max_policy_text_bytes: 2 * 1024 * 1024,
            max_schema_work: 1_000_000,
            max_source_bytes: 1024 * 1024,
            max_line_bytes: 65536,
            max_lines: 8192,
            max_work: 64 * 1024 * 1024,
            max_output_declarations: 65536,
        }
    }
}
impl ItemLineLimits {
    pub fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (name, v, max) in self
            .fields()
            .into_iter()
            .zip(hard.fields())
            .map(|((n, v), (_, m))| (n, v, m))
        {
            if v == 0 || v > max {
                return Err(ItemLineError::InvalidLimit(name));
            }
        }
        self.value.validate()?;
        Ok(())
    }
    fn fields(self) -> [(&'static str, usize); 13] {
        [
            ("wire bytes", self.max_wire_bytes),
            ("rules", self.max_rules),
            ("parts", self.max_parts),
            ("captures", self.max_captures),
            ("emissions", self.max_emissions),
            ("rolls", self.max_rolls),
            ("policy text bytes", self.max_policy_text_bytes),
            ("schema work", self.max_schema_work),
            ("source bytes", self.max_source_bytes),
            ("line bytes", self.max_line_bytes),
            ("lines", self.max_lines),
            ("work", self.max_work),
            ("output declarations", self.max_output_declarations),
        ]
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ItemLineError {
    #[error("unsupported item-line policy version {0}")]
    UnsupportedVersion(u32),
    #[error("invalid item-line limit: {0}")]
    InvalidLimit(&'static str),
    #[error("item-line resource limit: {0}")]
    Limit(&'static str),
    #[error("item-line policy schema binding differs")]
    Binding,
    #[error("{path}: {reason}")]
    InvalidPolicy { path: String, reason: &'static str },
    #[error("inconsistent schema index: {0:?}")]
    IndexFault(Box<SchemaSubject>),
    #[error("source line indices must be positive and strictly increasing")]
    LineOrder,
    #[error(transparent)]
    Codec(#[from] ValueCodecError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
pub type Result<T> = std::result::Result<T, ItemLineError>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemSchemaUnknown {
    Missing,
    Unmapped,
    Partial,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ItemLinePending {
    UnknownLine,
    AmbiguousRules,
    AmbiguousCapture,
    MalformedCapture {
        capture: OwnedDefinitionKey,
        reason: String,
    },
    MissingRangeFraction,
    InvalidRangeFraction,
    InvalidRange,
    Schema {
        subject: Box<SchemaSubject>,
        status: ItemSchemaUnknown,
    },
    ValueOutsideSchema {
        slot: Option<Box<DeclaredSlot<ParameterSlotDefId>>>,
    },
    UnsupportedLineLayout,
    SourceMeaningUnresolved,
    /// Source syntax proves this line is presentation, not an owned declaration.
    SourcePresentation,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ConvertedItemEmission {
    Metadata {
        role: OwnedDefinitionKey,
    },
    Template {
        definition: ItemTemplateDefId,
    },
    ItemLevel {
        value: BoundedInteger,
    },
    Quality {
        value: QualitySelection,
    },
    ItemParameter {
        assignment: ParameterAssignment,
    },
    Modifier {
        definition: ModifierDefId,
        rolls: Vec<ParameterAssignment>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ItemLineOutcome {
    Known {
        rule: OwnedDefinitionKey,
        emissions: Vec<ConvertedItemEmission>,
    },
    Pending {
        reason: ItemLinePending,
        candidates: Vec<OwnedDefinitionKey>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ItemLineEvidence<'a> {
    pub index: usize,
    pub text: &'a str,
    pub outcome: ItemLineOutcome,
}
#[derive(Clone, Copy, Debug)]
pub struct ItemLineInput<'a> {
    pub index: usize,
    pub text: &'a str,
    pub range_fraction: Option<f64>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ItemField<T> {
    Absent,
    Known { value: T, line: usize },
    Pending { lines: Vec<usize> },
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LocatedItemModifier {
    pub line: usize,
    pub emission: usize,
    pub definition: ModifierDefId,
    pub rolls: Vec<ParameterAssignment>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LocatedItemParameter {
    pub line: usize,
    pub emission: usize,
    pub assignment: ParameterAssignment,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemTextProblem {
    DuplicateHeader,
    DuplicateParameter,
    TemplateUnavailable,
    WrongParameterOwner,
    ModifierNotAllowed,
    LevelOutsideSchema,
    QualityNotAllowed,
    SchemaPartial,
    RequiredParameterMissing,
    RequiredQualityMissing,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ItemTextIssue {
    pub problem: ItemTextProblem,
    pub lines: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ItemTextConversion<'a> {
    pub lines: Vec<ItemLineEvidence<'a>>,
    pub template: ItemField<ItemTemplateDefId>,
    pub item_level: ItemField<BoundedInteger>,
    pub quality: ItemField<QualitySelection>,
    pub modifiers: Vec<LocatedItemModifier>,
    pub parameters: Vec<LocatedItemParameter>,
    pub issues: Vec<ItemTextIssue>,
}

#[derive(Clone, Debug)]
pub struct OwnedItemLinePolicy {
    input: ItemLinePolicyInput,
    identity: OwnedContentDigest,
    limits: ItemLineLimits,
    rules: Vec<BoundRule>,
    rule_indices: BTreeMap<OwnedDefinitionKey, usize>,
    templates: BTreeMap<ItemTemplateDefId, TemplateContext>,
    schema_work: usize,
}
#[derive(Clone, Debug)]
struct BoundRule {
    codecs: BTreeMap<OwnedDefinitionKey, OwnedValueCodec>,
    constraints: Vec<Option<ValueSchema>>,
    pending: Option<ItemLinePending>,
}
#[derive(Clone, Debug)]
struct TemplateContext {
    level: IntegerRange,
    quality: QualityUseSchema,
    modifiers: DeclaredSet<ModifierDefId>,
    required_parameters: Vec<DeclaredSlot<ParameterSlotDefId>>,
    parameters_complete: bool,
}
fn charge(left: &mut usize, n: usize, name: &'static str) -> Result<()> {
    *left = left.checked_sub(n).ok_or(ItemLineError::Limit(name))?;
    Ok(())
}
fn invalid<T>(path: &str, reason: &'static str) -> Result<T> {
    Err(ItemLineError::InvalidPolicy {
        path: path.into(),
        reason,
    })
}
impl OwnedItemLinePolicy {
    pub(crate) fn source_limits(&self) -> ItemLineLimits {
        self.limits
    }
    pub fn input(&self) -> &ItemLinePolicyInput {
        &self.input
    }
    pub fn identity(&self) -> &OwnedContentDigest {
        &self.identity
    }
    pub fn verify_bindings<I: DefinitionSchemaIndex>(&self, schema: &I) -> Result<()> {
        if &self.input.definitions != schema.identity()
            || &self.input.namespace != schema.namespace()
        {
            return Err(ItemLineError::Binding);
        }
        Ok(())
    }
}
pub fn decode_item_line_policy<I: DefinitionSchemaIndex>(
    bytes: &[u8],
    schema: &I,
    limits: ItemLineLimits,
) -> Result<OwnedItemLinePolicy> {
    limits.validate()?;
    if bytes.len() > limits.max_wire_bytes {
        return Err(ItemLineError::Limit("wire bytes"));
    }
    OwnedItemLinePolicy::new(serde_json::from_slice(bytes)?, schema, limits)
}
pub fn encode_item_line_policy(
    policy: &OwnedItemLinePolicy,
    limits: ItemLineLimits,
) -> Result<Vec<u8>> {
    limits.validate()?;
    schema::validate_shape(&policy.input, limits)?;
    if policy.schema_work > limits.max_schema_work {
        return Err(ItemLineError::Limit("schema work"));
    }
    digest_owned(DOMAIN, &policy.input, limits.max_wire_bytes)?;
    Ok(serde_json::to_vec(&policy.input)?)
}

/// Adapter-only override; candidate IDs still flow through the same aggregate guards.
pub(crate) struct SourceLinePending<'a> {
    pub reason: ItemLinePending,
    pub candidates: &'a [OwnedDefinitionKey],
}
