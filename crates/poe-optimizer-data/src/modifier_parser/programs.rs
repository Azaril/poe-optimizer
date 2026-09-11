//! Standalone typed parser programs. Binding never changes the packaged parser schema
//! or grants a parser call site execution capability.
use super::*;
use std::collections::BTreeSet;

mod validate;

pub const PARSER_PROGRAM_SCHEMA_VERSION: u32 = 1;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ParserProgramId(pub u32);

/// Offsets relative to the complete function slice in program provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserProgramLocation {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserProgramProvenance {
    pub source: ItemSourceSpan,
    pub function_start: u32,
    pub function_end: u32,
    pub function_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserProgramData {
    pub schema_version: u32,
    pub programs: Vec<ParserProgram>,
    #[serde(deserialize_with = "unique_map")]
    pub callbacks: BTreeMap<ParserCallbackId, ParserProgramId>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserProgram {
    pub callback: ParserCallbackId,
    /// The first parameter_count local slots are writable, initially visible parameters.
    pub parameter_count: u16,
    pub variadic: bool,
    pub local_count: u16,
    pub bindings: Vec<ParserProgramBinding>,
    pub body: Vec<ParserProgramStatement>,
    pub provenance: ParserProgramProvenance,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserProgramStatement {
    pub location: ParserProgramLocation,
    pub operation: ParserProgramStatementKind,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParserProgramStatementKind {
    /// Evaluate all RHS values before introducing the new lexical slots. Missing
    /// adjusted values initialize to Nil, including a completely absent RHS.
    Declare {
        locals: Vec<u16>,
        values: ParserProgramValueList,
    },
    Assign {
        locals: Vec<u16>,
        values: ParserProgramValueList,
    },
    If {
        branches: Vec<ParserProgramBranch>,
        otherwise: Vec<ParserProgramStatement>,
    },
    ForNumeric {
        local: u16,
        start: ParserProgramExpr,
        limit: ParserProgramExpr,
        step: ParserProgramExpr,
        body: Vec<ParserProgramStatement>,
    },
    ForEach {
        locals: Vec<u16>,
        iterator: ParserProgramIterator,
        body: Vec<ParserProgramStatement>,
    },
    /// Reached writes require an invocation-owned table handle. A borrowed table
    /// is an unavailable effect, never an instruction to clone shared data.
    TableSet {
        table: ParserProgramExpr,
        key: ParserProgramExpr,
        value: ParserProgramExpr,
    },
    /// Append using the applicable current length, not a literal-list counter.
    TableAppend {
        binding: u16,
        table: ParserProgramExpr,
        value: ParserProgramExpr,
    },
    Call {
        call: ParserProgramCall,
    },
    Return {
        values: ParserProgramValueList,
    },
    Break,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserProgramBranch {
    pub condition: ParserProgramExpr,
    pub body: Vec<ParserProgramStatement>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParserProgramIterator {
    Dense {
        table: ParserProgramExpr,
        binding: u16,
    },
    Pattern {
        call: ParserProgramCall,
    },
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserProgramValueList {
    /// Each expression consumes exactly its first result, adjusted to Nil.
    pub values: Vec<ParserProgramExpr>,
    /// Only this final explicit expansion contributes all of its results.
    #[serde(deserialize_with = "required_option")]
    pub tail: Option<Box<ParserProgramPack>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParserProgramPack {
    Call { call: ParserProgramCall },
    Varargs,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserProgramExpr {
    pub location: ParserProgramLocation,
    pub operation: ParserProgramExprKind,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParserProgramExprKind {
    Literal {
        value: ParserFactoryLiteral,
    },
    Bytes {
        value: Vec<u8>,
    },
    Local {
        local: u16,
    },
    /// Captured value kinds stay dynamic; a selected operation performs its own
    /// source type/ownership check. This is not a scalar-only projection.
    Capture {
        upvalue: u16,
    },
    Definition {
        root: ParserProgramDefinitionRoot,
    },
    Get {
        table: Box<ParserProgramExpr>,
        key: Box<ParserProgramExpr>,
    },
    Unary {
        operation: ParserProgramUnary,
        value: Box<ParserProgramExpr>,
    },
    Binary {
        operation: ParserProgramBinary,
        left: Box<ParserProgramExpr>,
        right: Box<ParserProgramExpr>,
    },
    Table {
        fields: Vec<ParserProgramField>,
    },
    /// Expression context adjusts the raw result pack to its first value or Nil.
    Call {
        call: Box<ParserProgramCall>,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParserProgramDefinitionRoot {
    ModFlags,
    KeywordFlags,
    SkillTypes,
    GemIdLookup,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParserProgramUnary {
    Not,
    Negate,
    Length,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParserProgramBinary {
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    Concat,
    /// Lazy, value-returning operations, not eager Boolean reductions.
    And,
    Or,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParserProgramField {
    Named {
        key: String,
        value: ParserProgramExpr,
    },
    Keyed {
        key: ParserProgramExpr,
        value: ParserProgramExpr,
    },
    /// Every literal list field advances its implicit index, including Nil.
    List { value: ParserProgramExpr },
    /// Final literal list entry expands every result while advancing each index.
    Tail { values: ParserProgramPack },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserProgramCall {
    pub binding: u16,
    /// A method receiver is evaluated and indexed before arguments. Only the
    /// declared string-method primitive is currently admitted by this form.
    #[serde(deserialize_with = "required_option")]
    pub receiver: Option<Box<ParserProgramExpr>>,
    pub arguments: ParserProgramValueList,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParserProgramBinding {
    CapturedCallback {
        upvalue: u16,
        callback: ParserCallbackId,
    },
    Intrinsic {
        operation: ParserProgramIntrinsic,
        source: ParserProgramIntrinsicSource,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParserProgramIntrinsic {
    ToNumber,
    StringGsub,
    StringGmatch,
    TableInsert,
    Ipairs,
    CreateMod,
}
impl ParserProgramIntrinsic {
    /// Language/runtime identities, never game-specific lookup names or values.
    pub fn global_path(self) -> Option<&'static [&'static str]> {
        match self {
            Self::ToNumber => Some(&["tonumber"]),
            Self::StringGsub => Some(&["string", "gsub"]),
            Self::StringGmatch => Some(&["string", "gmatch"]),
            Self::TableInsert => Some(&["table", "insert"]),
            Self::Ipairs => Some(&["ipairs"]),
            Self::CreateMod => None,
        }
    }
    pub fn is_string_method(self) -> bool {
        matches!(self, Self::StringGsub | Self::StringGmatch)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ParserProgramIntrinsicSource {
    /// The source lowerer must prove the actual original global primitive and
    /// its table/method environment. This label is not a trust certificate.
    OriginalGlobal,
    Captured {
        upvalue: u16,
        callback: ParserCallbackId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParserProgramCapability {
    Core,
    NumericFor,
    DenseFor,
    PatternFor,
    Varargs,
    LegacyPureCalls,
    RecursiveCalls,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserProgramErrorKind {
    InvalidData,
    Binding,
    ResourceLimit,
    UnsupportedCapability,
}
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ParserProgramError {
    pub kind: ParserProgramErrorKind,
    pub program: Option<ParserProgramId>,
    pub location: Option<ParserProgramLocation>,
    pub message: String,
}
pub type ParserProgramResult<T> = std::result::Result<T, ParserProgramError>;

#[derive(Debug, Clone)]
pub struct ParserProgramCatalog {
    data: Arc<ParserProgramData>,
    owner: ModifierParserCatalog,
    required: BTreeSet<ParserProgramCapability>,
}
impl ParserProgramCatalog {
    pub fn new(data: ParserProgramData, owner: ModifierParserCatalog) -> ParserProgramResult<Self> {
        let required = validate::validate(&data, &owner)?;
        Ok(Self {
            data: Arc::new(data),
            owner,
            required,
        })
    }
    /// Bounded untrusted authoring input. serde's recursion bound applies before
    /// typed allocation; the structural verifier then applies language limits.
    pub fn from_bytes(bytes: &[u8], owner: ModifierParserCatalog) -> ParserProgramResult<Self> {
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(validate::error(
                ParserProgramErrorKind::ResourceLimit,
                None,
                None,
                "program JSON byte bound",
            ));
        }
        let data = serde_json::from_slice(bytes).map_err(|e| {
            validate::error(
                ParserProgramErrorKind::InvalidData,
                None,
                None,
                format!("program JSON: {e}"),
            )
        })?;
        Self::new(data, owner)
    }
    pub fn data(&self) -> &ParserProgramData {
        &self.data
    }
    pub fn owner(&self) -> &ModifierParserCatalog {
        &self.owner
    }
    pub fn is_bound_to(&self, owner: &ModifierParserCatalog) -> bool {
        Arc::ptr_eq(&self.owner.0, &owner.0)
    }
    pub fn program(&self, id: ParserProgramId) -> Option<&ParserProgram> {
        id.0.checked_sub(1)
            .and_then(|i| self.data.programs.get(i as usize))
    }
    pub fn program_id(&self, callback: ParserCallbackId) -> Option<ParserProgramId> {
        self.data.callbacks.get(&callback).copied()
    }
    pub fn for_callback(&self, callback: ParserCallbackId) -> Option<&ParserProgram> {
        self.program(self.program_id(callback)?)
    }
    pub fn required_capabilities(&self) -> &BTreeSet<ParserProgramCapability> {
        &self.required
    }
    pub fn check_capabilities(
        &self,
        available: &BTreeSet<ParserProgramCapability>,
    ) -> ParserProgramResult<()> {
        if let Some(missing) = self.required.difference(available).next() {
            return Err(validate::error(
                ParserProgramErrorKind::UnsupportedCapability,
                None,
                None,
                format!("program capability unavailable: {missing:?}"),
            ));
        }
        Ok(())
    }
    pub fn definition(&self, root: ParserProgramDefinitionRoot) -> &ParserTable {
        match root {
            ParserProgramDefinitionRoot::ModFlags => {
                self.owner.table(self.owner.data().policy.mod_flags)
            }
            ParserProgramDefinitionRoot::KeywordFlags => {
                self.owner.table(self.owner.data().policy.keyword_flags)
            }
            ParserProgramDefinitionRoot::SkillTypes => {
                self.owner.table(self.owner.data().policy.skill_types)
            }
            ParserProgramDefinitionRoot::GemIdLookup => {
                Some(self.owner.dictionary(ParserDictionary::GemIdLookup))
            }
        }
        .expect("validated owner definition root")
    }
}
fn required_option<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> std::result::Result<Option<T>, D::Error> {
    Option::<T>::deserialize(d)
}
