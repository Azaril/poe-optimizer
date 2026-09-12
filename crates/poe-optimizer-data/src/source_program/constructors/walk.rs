//! Visits Table expression nodes only after the ordinary bounded IR verifier.
use super::*;
pub(super) fn tables(
    program: &SourceProgram,
    mut visitor: impl FnMut(&SourceProgramExpr, &[SourceProgramField]),
) {
    body(&program.body, &mut visitor)
}
fn body(
    stmts: &[SourceProgramStatement],
    visit: &mut impl FnMut(&SourceProgramExpr, &[SourceProgramField]),
) {
    use SourceProgramStatementKind as S;
    for stmt in stmts {
        match &stmt.operation {
            S::Declare { values, .. }
            | S::Assign { values, .. }
            | S::CaptureSet { values, .. }
            | S::Return { values } => values_list(values, visit),
            S::If {
                branches,
                otherwise,
            } => {
                for branch in branches {
                    expr(&branch.condition, visit);
                    body(&branch.body, visit)
                }
                body(otherwise, visit)
            }
            S::ForNumeric {
                start,
                limit,
                step,
                body: stmts,
                ..
            } => {
                expr(start, visit);
                expr(limit, visit);
                expr(step, visit);
                body(stmts, visit)
            }
            S::ForEach {
                iterator,
                body: stmts,
                ..
            } => {
                match iterator {
                    SourceProgramIterator::Generic { values } => values_list(values, visit),
                    SourceProgramIterator::Dense { table, .. } => expr(table, visit),
                    SourceProgramIterator::Pattern { call: c } => call(c, visit),
                }
                body(stmts, visit)
            }
            S::TableSet { table, key, value } => {
                expr(table, visit);
                expr(key, visit);
                expr(value, visit)
            }
            S::TableAppend { table, value, .. } => {
                expr(table, visit);
                expr(value, visit)
            }
            S::Call { call: c } => call(c, visit),
            S::Break => {}
        }
    }
}
fn values_list(
    values: &SourceProgramValueList,
    visit: &mut impl FnMut(&SourceProgramExpr, &[SourceProgramField]),
) {
    for e in &values.values {
        expr(e, visit)
    }
    if let Some(tail) = &values.tail {
        pack(tail, visit)
    }
}
fn pack(
    pack: &SourceProgramPack,
    visit: &mut impl FnMut(&SourceProgramExpr, &[SourceProgramField]),
) {
    match pack {
        SourceProgramPack::Call { call: c } => call(c, visit),
        SourceProgramPack::Varargs => {}
    }
}
fn call(
    call: &SourceProgramCall,
    visit: &mut impl FnMut(&SourceProgramExpr, &[SourceProgramField]),
) {
    if let Some(receiver) = &call.receiver {
        expr(receiver, visit)
    }
    values_list(&call.arguments, visit)
}
fn expr(e: &SourceProgramExpr, visit: &mut impl FnMut(&SourceProgramExpr, &[SourceProgramField])) {
    use SourceProgramExprKind as E;
    match &e.operation {
        E::Get { table, key } => {
            expr(table, visit);
            expr(key, visit)
        }
        E::Unary { value, .. } => expr(value, visit),
        E::Binary { left, right, .. } => {
            expr(left, visit);
            expr(right, visit)
        }
        E::Table { fields } => {
            visit(e, fields);
            for field in fields {
                match field {
                    SourceProgramField::Named { value, .. }
                    | SourceProgramField::List { value } => expr(value, visit),
                    SourceProgramField::Keyed { key, value } => {
                        expr(key, visit);
                        expr(value, visit)
                    }
                    SourceProgramField::Tail { values } => pack(values, visit),
                }
            }
        }
        E::Call { call: c } => call(c, visit),
        E::Literal { .. }
        | E::Bytes { .. }
        | E::Local { .. }
        | E::Capture { .. }
        | E::Definition { .. }
        | E::NamedDefinition { .. } => {}
    }
}
