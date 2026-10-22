/// A half-open UTF-8 byte range in the exact input passed to the parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn text(self, source: &str) -> Option<&str> {
        source.get(self.start..self.end)
    }

    pub(crate) fn cover(self, other: Self) -> Self {
        Self {
            start: self.start,
            end: other.end,
        }
    }
}

/// An index local to one parsed expression's arena, not a resolved symbol ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExprId(pub(crate) usize);

impl ExprId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitSuffix {
    Percent(Span),
    Named(Span),
}

/// Decimal spelling is retained without rounding, range checks or unit inference.
/// A sign is always a separate unary node, including the sign in `-0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumericLiteral {
    pub spelling: Span,
    pub unit: Option<UnitSuffix>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOperator {
    Negate,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

impl BinaryOperator {
    pub(crate) fn is_comparison(self) -> bool {
        matches!(
            self,
            Self::Equal
                | Self::NotEqual
                | Self::Less
                | Self::LessEqual
                | Self::Greater
                | Self::GreaterEqual
        )
    }
}

/// Syntax only. In particular, a path or call grants no read/function authority.
/// Boolean operands stay ordered; parsing does not change their IR failure rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExprKind {
    Number(NumericLiteral),
    Boolean(bool),
    /// A quoted, validated Core key. This is metadata, never a scalar string.
    /// The inner span excludes quotes; the containing node's span includes them.
    Key(Span),
    Path {
        segments: Vec<Span>,
    },
    Call {
        /// A Path node; arbitrary expression invocation is not supported.
        callee: ExprId,
        arguments: Vec<ExprId>,
    },
    Unary {
        operator: UnaryOperator,
        operator_span: Span,
        operand: ExprId,
    },
    Binary {
        operator: BinaryOperator,
        operator_span: Span,
        left: ExprId,
        right: ExprId,
    },
    Group {
        inner: ExprId,
    },
    /// Explicit lazy branch structure. An `else if` is another If in else_branch.
    /// Only later lowering may produce the existing lazy Select operation.
    If {
        condition: ExprId,
        then_branch: ExprId,
        else_branch: ExprId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExprNode {
    pub span: Span,
    pub kind: ExprKind,
}

/// Flat postorder arena: every edge points to an earlier node. Even a long
/// left-associated expression has no recursive ownership or recursive Drop.
#[derive(Debug, PartialEq, Eq)]
pub struct ParsedExpression<'source> {
    pub(crate) source: &'source str,
    pub(crate) nodes: Vec<ExprNode>,
    pub(crate) root: ExprId,
}

impl<'source> ParsedExpression<'source> {
    pub fn source(&self) -> &'source str {
        self.source
    }

    pub fn root(&self) -> ExprId {
        self.root
    }

    pub fn nodes(&self) -> &[ExprNode] {
        &self.nodes
    }

    pub fn node(&self, id: ExprId) -> Option<&ExprNode> {
        self.nodes.get(id.0)
    }

    pub fn text(&self, span: Span) -> Option<&'source str> {
        span.text(self.source)
    }
}

/// A sequential named expression. Its RHS may refer to earlier declarations,
/// but parsing alone does not establish symbol visibility or type correctness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LetStatement {
    pub name: Span,
    pub value: ExprId,
    pub span: Span,
}

/// A formula block shares one syntax arena and one set of source byte offsets.
/// Each statement's expression occupies the next contiguous postorder range.
/// Read scopes, owner membership and effects are not inferred from these names.
#[derive(Debug, PartialEq, Eq)]
pub struct ParsedBlock<'source> {
    pub(crate) source: &'source str,
    pub(crate) nodes: Vec<ExprNode>,
    pub(crate) statements: Vec<LetStatement>,
}

impl<'source> ParsedBlock<'source> {
    pub fn source(&self) -> &'source str {
        self.source
    }

    pub fn nodes(&self) -> &[ExprNode] {
        &self.nodes
    }

    pub fn statements(&self) -> &[LetStatement] {
        &self.statements
    }

    pub fn node(&self, id: ExprId) -> Option<&ExprNode> {
        self.nodes.get(id.0)
    }

    pub fn text(&self, span: Span) -> Option<&'source str> {
        span.text(self.source)
    }
}

/// One declared read's identity, authoring alias, type and source constructor.
/// Parsing does not authenticate the source or infer the alias's value type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadDeclaration {
    pub id: Span,
    pub name: Span,
    pub value_type: ExprId,
    pub source: ExprId,
    pub span: Span,
}

/// One declared effect, retaining its exact identity and optional guard.
/// Lowering validates its constructor and requires existing named value handles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectDeclaration {
    pub id: Span,
    pub when: Option<ExprId>,
    pub effect: ExprId,
    pub span: Span,
}

/// An unchecked program declaration with ordered reads, formulas and effects.
/// Every expression uses the same flat arena and original source byte offsets.
/// Metadata constructors are syntax only and do not become executable values.
#[derive(Debug, PartialEq, Eq)]
pub struct ParsedProgram<'source> {
    pub(crate) source: &'source str,
    pub(crate) nodes: Vec<ExprNode>,
    pub(crate) id: Span,
    pub(crate) context: Span,
    pub(crate) reads: Vec<ReadDeclaration>,
    pub(crate) statements: Vec<LetStatement>,
    pub(crate) effects: Vec<EffectDeclaration>,
}

impl<'source> ParsedProgram<'source> {
    pub fn source(&self) -> &'source str {
        self.source
    }

    pub fn nodes(&self) -> &[ExprNode] {
        &self.nodes
    }

    pub fn id(&self) -> Span {
        self.id
    }

    pub fn context(&self) -> Span {
        self.context
    }

    pub fn reads(&self) -> &[ReadDeclaration] {
        &self.reads
    }

    pub fn statements(&self) -> &[LetStatement] {
        &self.statements
    }

    pub fn effects(&self) -> &[EffectDeclaration] {
        &self.effects
    }

    pub fn node(&self, id: ExprId) -> Option<&ExprNode> {
        self.nodes.get(id.0)
    }

    pub fn text(&self, span: Span) -> Option<&'source str> {
        span.text(self.source)
    }
}
