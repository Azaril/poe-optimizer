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
