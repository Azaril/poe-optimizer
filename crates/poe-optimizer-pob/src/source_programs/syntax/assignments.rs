//! LuaJIT expression-register classification used by standalone indexed reads
//! and assignments. Parentheses retain the underlying descriptor. Logical
//! operators merge pending branches; only branch-free local descriptors stay
//! live registers. This follows pinned lj_parse.c, not expression-value guesses.
use super::*;
#[derive(Clone, Copy)]
enum Constant {
    Nil,
    Boolean(bool),
    Number(f64),
    Text,
}
#[derive(Clone, Copy)]
enum Kind {
    Constant(Constant),
    Local(u16),
    Temporary,
}
#[derive(Clone, Copy)]
struct Shape {
    kind: Kind,
    true_branch: bool,
    false_branch: bool,
}
impl Shape {
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            true_branch: false,
            false_branch: false,
        }
    }
    fn temporary() -> Self {
        Self::new(Kind::Temporary)
    }
    fn branched(self) -> bool {
        self.true_branch || self.false_branch
    }
    fn truth(self) -> Option<bool> {
        match self.kind {
            Kind::Constant(Constant::Nil | Constant::Boolean(false)) => Some(false),
            Kind::Constant(_) => Some(true),
            _ => None,
        }
    }
}
impl Lowerer<'_, '_> {
    pub(super) fn mixed_assignment(
        &self,
        targets: Vec<Expr>,
        values: ParserProgramValueList,
    ) -> LowerResult<ParserProgramStatementKind> {
        let targets = targets
            .into_iter()
            .map(|target| {
                let operation = match target.operation {
                    ParserProgramExprKind::Local { local } => {
                        ParserProgramAssignmentTargetKind::Local { local }
                    }
                    ParserProgramExprKind::Capture { upvalue } => {
                        if !matches!(
                            self.callback.upvalues[upvalue as usize].value,
                            ParserValue::LiveCapture {}
                        ) {
                            return Err(
                                "capture assignment requires a declared live session cell".into()
                            );
                        }
                        ParserProgramAssignmentTargetKind::Capture { upvalue }
                    }
                    ParserProgramExprKind::IndexedRead { table, key } => {
                        ParserProgramAssignmentTargetKind::Indexed {
                            table: *table,
                            key: self.assignment_operand(*key)?,
                        }
                    }
                    _ => {
                        return Err(
                            "assignment requires visible locals, live captures or indexed tables"
                                .into(),
                        );
                    }
                };
                Ok(ParserProgramAssignmentTarget {
                    location: target.location,
                    operation,
                })
            })
            .collect::<LowerResult<Vec<_>>>()?;
        Ok(ParserProgramStatementKind::MixedAssign { targets, values })
    }
    pub(super) fn indexed_read(
        &mut self,
        table: Expr,
        key: Expr,
        start: usize,
        end: usize,
        height: usize,
    ) -> LowerResult<Info> {
        let operation = if self.authorization.standalone_calls {
            ParserProgramExprKind::IndexedRead {
                table: Box::new(self.assignment_operand(table)?),
                key: Box::new(key),
            }
        } else {
            ParserProgramExprKind::Get {
                table: Box::new(table),
                key: Box::new(key),
            }
        };
        self.node(operation, start, end, height)
    }
    fn assignment_operand(&self, value: Expr) -> LowerResult<ParserProgramAssignmentOperand> {
        let shape = self.expression_shape(&value)?;
        if let Kind::Local(local) = shape.kind
            && !shape.branched()
        {
            Ok(ParserProgramAssignmentOperand::LocalRegister { local })
        } else {
            Ok(ParserProgramAssignmentOperand::Evaluated { value })
        }
    }
    fn expression_shape(&self, expression: &Expr) -> LowerResult<Shape> {
        use ParserProgramBinary as B;
        use ParserProgramExprKind as E;
        use ParserProgramUnary as U;
        Ok(match &expression.operation {
            E::Local { local } => Shape::new(Kind::Local(*local)),
            E::Bytes { .. } => Shape::new(Kind::Constant(Constant::Text)),
            E::Literal { value } => Shape::new(Kind::Constant(match value {
                ParserFactoryLiteral::Nil => Constant::Nil,
                ParserFactoryLiteral::Boolean(value) => Constant::Boolean(*value),
                ParserFactoryLiteral::Number(value) => Constant::Number(*value),
                ParserFactoryLiteral::Text(_) => Constant::Text,
                ParserFactoryLiteral::NonFinite(ParserNonFinite::PositiveInfinity) => {
                    Constant::Number(f64::INFINITY)
                }
                ParserFactoryLiteral::NonFinite(ParserNonFinite::NegativeInfinity) => {
                    Constant::Number(f64::NEG_INFINITY)
                }
                ParserFactoryLiteral::NonFinite(ParserNonFinite::Nan) => {
                    return Err("NaN is not a source numeric literal descriptor".into());
                }
            })),
            E::Unary { operation, value } => {
                let mut value = self.expression_shape(value)?;
                match operation {
                    U::Not => {
                        std::mem::swap(&mut value.true_branch, &mut value.false_branch);
                        value.kind = value.truth().map_or(Kind::Temporary, |truth| {
                            Kind::Constant(Constant::Boolean(!truth))
                        });
                        value
                    }
                    U::Negate => {
                        if let Kind::Constant(Constant::Number(number)) = value.kind
                            && !value.branched()
                            && number != 0.0
                        {
                            Shape::new(Kind::Constant(Constant::Number(-number)))
                        } else {
                            Shape::temporary()
                        }
                    }
                    U::Length => Shape::temporary(),
                }
            }
            E::Binary {
                operation,
                left,
                right,
            } => {
                let left = self.expression_shape(left)?;
                let mut right = self.expression_shape(right)?;
                match operation {
                    B::And => {
                        right.false_branch |= left.false_branch || left.truth() != Some(true);
                        right
                    }
                    B::Or => {
                        right.true_branch |= left.true_branch || left.truth() != Some(false);
                        right
                    }
                    B::Add | B::Subtract | B::Multiply | B::Divide | B::Modulo | B::Power => {
                        if let (
                            Kind::Constant(Constant::Number(a)),
                            Kind::Constant(Constant::Number(b)),
                        ) = (left.kind, right.kind)
                            && !left.branched()
                            && !right.branched()
                        {
                            let operation = match operation {
                                B::Add => "+",
                                B::Subtract => "-",
                                B::Multiply => "*",
                                B::Divide => "/",
                                B::Modulo => "%",
                                B::Power => "^",
                                _ => unreachable!(),
                            };
                            // Both operands have proven numeric-constant descriptors.
                            // Compile/evaluate only these rendered numbers and one
                            // arithmetic operator in the isolated lowering Lua host.
                            // This uses the pinned compiler's fold arithmetic, including
                            // its pow/mod dialect, without executing authored source.
                            let text = format!(
                                "return ({}) {operation} ({})",
                                numeric_literal(a),
                                numeric_literal(b)
                            );
                            let number: f64 = self.lua.load(&text).eval().map_err(|error| {
                                format!("numeric constant classification failed: {error}")
                            })?;
                            if number.is_nan() || number.to_bits() == (-0.0f64).to_bits() {
                                Shape::temporary()
                            } else {
                                Shape::new(Kind::Constant(Constant::Number(number)))
                            }
                        } else {
                            Shape::temporary()
                        }
                    }
                    _ => Shape::temporary(),
                }
            }
            _ => Shape::temporary(),
        })
    }
}
fn numeric_literal(value: f64) -> String {
    if value == f64::INFINITY {
        "(1/0)".into()
    } else if value == f64::NEG_INFINITY {
        "(-1/0)".into()
    } else {
        value.to_string()
    }
}
