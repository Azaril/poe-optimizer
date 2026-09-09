//! Bounded execution of proven pure modifier callback factories.
//!
//! The catalog supplies expression structure, constants and captured definitions.
//! This is deliberately not a Lua evaluator: unrepresented closures remain pending.
use super::*;
use ModifierValue as V;
use poe_optimizer_data::modifier_parser::{
    ParserFactoryDisposition, ParserFactoryExpr, ParserFactoryField, ParserFactoryLiteral,
    ParserNonFinite,
};

/// Borrow captures in their original positions; no invocation argument vector
/// or extra raw string copy is needed for Prefix or either tag call site.
#[derive(Clone, Copy)]
enum FactoryArguments<'a> {
    Raw(&'a [V]),
    Leading { first: &'a V, captures: &'a [V] },
}
impl<'a> FactoryArguments<'a> {
    fn get(self, index: usize) -> &'a V {
        match self {
            Self::Raw(captures) => captures.get(index).unwrap_or(&V::Nil),
            Self::Leading { first, captures } => {
                if index == 0 {
                    first
                } else {
                    captures.get(index - 1).unwrap_or(&V::Nil)
                }
            }
        }
    }
}

impl Run<'_> {
    pub(super) fn special_factory(
        &mut self,
        callback: ParserCallbackId,
        captures: &[V],
    ) -> ParserResult<ParseOutcome> {
        let parser = self.parser;
        let Some(ParserFactoryDisposition::Pure(factory)) = parser.catalog.factory(callback) else {
            return Err(ParserError::Deferred {
                stage: "special callback",
                callback: Some(callback),
            });
        };
        // The source passes tonumber(cap[1]) followed by every raw capture.
        // Conversion is performed even when the callback ignores its first parameter.
        if let Some(V::Bytes(bytes)) = captures.first() {
            self.budget.charge(bytes.len() as u64)?;
        }
        let first = captures
            .first()
            .and_then(V::number)
            .map(V::Number)
            .unwrap_or(V::Nil);
        let result = self.factory_expr(
            &factory.body,
            callback,
            FactoryArguments::Leading {
                first: &first,
                captures,
            },
            0,
        )?;
        let modifiers = match result {
            V::Nil => None,
            V::Table(table) => {
                // The admitted root is a fresh table/constructor expression.
                // Retain a bound if a future data shape returns a shared table.
                match Arc::try_unwrap(table) {
                    Ok(table) => Some(table),
                    Err(table) => {
                        let copy = deep_copy(&V::Table(table), &mut self.output)?;
                        Some(copy.table()?.clone())
                    }
                }
            }
            _ => {
                return Err(ParserError::Deferred {
                    stage: "special factory return shape",
                    callback: Some(callback),
                });
            }
        };
        // Unlike static special rows, the source does not copy a callback return
        // here. The public parser still performs its final cache-equivalent copy.
        Ok(ParseOutcome {
            modifiers,
            extra: None,
        })
    }

    pub(super) fn prefix_factory(&mut self, selected: Selected) -> ParserResult<V> {
        match selected.value {
            V::Callback(callback) => self.factory_value(
                callback,
                FactoryArguments::Raw(&selected.captures),
                "prefix callback",
            ),
            value => Ok(value),
        }
    }

    pub(super) fn tag_factory(
        &mut self,
        selected: Selected,
        stage: &'static str,
    ) -> ParserResult<V> {
        let V::Callback(callback) = selected.value else {
            return Ok(selected.value);
        };
        let first = selected.captures.first().unwrap_or(&V::Nil);
        // The original method lookup errors before matching or entering the body,
        // even for an otherwise unsupported callback that ignores its arguments.
        let bytes = first.as_bytes().ok_or_else(|| {
            ParserError::SourceError("attempt to index a non-string tag capture".into())
        })?;
        // string:match always runs the pattern interpreter. A returned empty
        // capture/position is still truthy, while no match keeps the raw value.
        let numeric = self
            .parser
            .tag_capture_numeric
            .match_captures(bytes, 1, self.budget)?
            .is_some();
        let converted;
        let leading = if numeric {
            self.budget.charge(bytes.len() as u64)?;
            converted = first.number().map(V::Number).unwrap_or(V::Nil);
            &converted
        } else {
            first
        };
        self.factory_value(
            callback,
            FactoryArguments::Leading {
                first: leading,
                captures: &selected.captures,
            },
            stage,
        )
    }

    fn factory_value(
        &mut self,
        callback: ParserCallbackId,
        arguments: FactoryArguments<'_>,
        stage: &'static str,
    ) -> ParserResult<V> {
        let parser = self.parser;
        let Some(ParserFactoryDisposition::Pure(factory)) = parser.catalog.factory(callback) else {
            return Err(ParserError::Deferred {
                stage,
                callback: Some(callback),
            });
        };
        // Ordinary callers consume the returned metadata directly, then apply
        // their existing tag/wrapper copies and the final public result copy.
        self.factory_expr(&factory.body, callback, arguments, 0)
    }

    fn factory_expr(
        &mut self,
        expression: &ParserFactoryExpr,
        callback: ParserCallbackId,
        arguments: FactoryArguments<'_>,
        depth: usize,
    ) -> ParserResult<V> {
        if depth > value::MAX_OUTPUT_DEPTH {
            return Err(ParserError::ResourceBound("factory expression depth"));
        }
        self.budget.charge(1)?;
        match expression {
            ParserFactoryExpr::Literal(value) => {
                self.output.charge(match value {
                    ParserFactoryLiteral::Text(text) => text.len(),
                    _ => 0,
                })?;
                Ok(match value {
                    ParserFactoryLiteral::Nil => V::Nil,
                    ParserFactoryLiteral::Boolean(v) => V::Boolean(*v),
                    ParserFactoryLiteral::Number(v) => V::Number(*v),
                    ParserFactoryLiteral::Text(v) => V::Bytes(v.as_bytes().to_vec()),
                    ParserFactoryLiteral::NonFinite(v) => V::Number(match v {
                        ParserNonFinite::PositiveInfinity => f64::INFINITY,
                        ParserNonFinite::NegativeInfinity => f64::NEG_INFINITY,
                        ParserNonFinite::Nan => f64::from_bits(0xfff8_0000_0000_0000),
                    }),
                })
            }
            ParserFactoryExpr::Argument(index) => {
                let value = arguments.get(*index as usize);
                self.output.charge(match value {
                    V::Bytes(bytes) => bytes.len(),
                    _ => 0,
                })?;
                Ok(value.clone())
            }
            ParserFactoryExpr::CapturedScalar { upvalue } => {
                let parser = self.parser;
                let value = parser
                    .catalog
                    .callback(callback)
                    .and_then(|row| row.upvalues.get(*upvalue as usize))
                    .map(|row| &row.value)
                    .ok_or_else(|| ParserError::InvalidData("factory upvalue reference".into()))?;
                self.factory_scalar(value, callback)
            }
            ParserFactoryExpr::ConstantField { table, key } => {
                let parser = self.parser;
                let value = parser
                    .catalog
                    .table(*table)
                    .ok_or_else(|| {
                        ParserError::InvalidData("factory constant table reference".into())
                    })?
                    .fields
                    .get(key)
                    .unwrap_or(&ParserValue::Nil);
                self.copy(value)
            }
            ParserFactoryExpr::Negate(value) => {
                let value = self.factory_expr(value, callback, arguments, depth + 1)?;
                if let V::Bytes(bytes) = &value {
                    self.budget.charge(bytes.len() as u64)?;
                }
                self.output.charge(0)?;
                value.negated()
            }
            ParserFactoryExpr::Concat { left, right } => {
                // Evaluate operands in source order before reducing this node.
                // Right association is represented by the injected expression tree.
                let left = self.factory_expr(left, callback, arguments, depth + 1)?;
                let right = self.factory_expr(right, callback, arguments, depth + 1)?;
                strings::concat(&left, &right, self.budget, &mut self.output)
            }
            ParserFactoryExpr::FirstToUpper { value, .. } => {
                // Catalog validation proves this node's captured helper identity.
                let value = self.factory_expr(value, callback, arguments, depth + 1)?;
                strings::first_to_upper(
                    &value,
                    &self.parser.first_to_upper,
                    self.budget,
                    &mut self.output,
                )
            }
            ParserFactoryExpr::Table(fields) => {
                self.output.charge(0)?;
                let mut table = ModifierTable::default();
                let mut index = 1i64;
                for field in fields {
                    self.budget.charge(1)?;
                    match field {
                        ParserFactoryField::Named { key, value } => {
                            self.output.charge(key.len())?;
                            let value = self.factory_expr(value, callback, arguments, depth + 1)?;
                            table.set(key, value);
                        }
                        ParserFactoryField::List(value) => {
                            let value = self.factory_expr(value, callback, arguments, depth + 1)?;
                            if !matches!(value, V::Nil) {
                                table.indexed.insert(index, value);
                            }
                            index = index
                                .checked_add(1)
                                .ok_or(ParserError::ResourceBound("factory table index"))?;
                        }
                    }
                }
                Ok(V::Table(Arc::new(table)))
            }
            ParserFactoryExpr::CreateMod { args } => {
                let bytes = args
                    .len()
                    .checked_mul(std::mem::size_of::<V>())
                    .ok_or(ParserError::ResourceBound("factory argument storage"))?;
                self.output.charge(bytes)?;
                let mut values = Vec::with_capacity(args.len());
                for expression in args {
                    values.push(self.factory_expr(expression, callback, arguments, depth + 1)?);
                }
                Ok(V::Table(Arc::new(create_mod(values, &mut self.output)?)))
            }
        }
    }

    fn factory_scalar(
        &mut self,
        value: &ParserValue,
        callback: ParserCallbackId,
    ) -> ParserResult<V> {
        if matches!(value, ParserValue::Table(_) | ParserValue::Callback(_)) {
            return Err(ParserError::Deferred {
                stage: "special factory captured value shape",
                callback: Some(callback),
            });
        }
        self.copy(value)
    }
}

/// Exact positional createMod construction over owned arguments, including nil
/// holes and non-string names/types. Argument evaluation is a separate prior step.
fn create_mod(arguments: Vec<V>, budget: &mut OutputBudget) -> ParserResult<ModifierTable> {
    budget.charge(0)?;
    let mut arguments = arguments.into_iter();
    let name = arguments.next().unwrap_or(V::Nil);
    let kind = arguments.next().unwrap_or(V::Nil);
    let value = arguments.next().unwrap_or(V::Nil);
    // No collection/copy of the trailing varargs is necessary. Each independently
    // typed position can reset where tags start, even if an earlier one was nil.
    let mut first = arguments.next();
    let second = arguments.next();
    let third = arguments.next();
    let mut tag_start = 0;
    let source = if matches!(first, Some(V::Bytes(_))) {
        tag_start = 1;
        first.replace(V::Nil).unwrap()
    } else {
        V::Nil
    };
    let flags = if let Some(V::Number(number)) = second {
        tag_start = 2;
        V::Number(number)
    } else {
        V::Number(0.0)
    };
    let keywords = if let Some(V::Number(number)) = third {
        tag_start = 3;
        V::Number(number)
    } else {
        V::Number(0.0)
    };
    let mut result = ModifierTable::default();
    for (key, value) in [
        ("name", name),
        ("type", kind),
        ("value", value),
        ("flags", flags),
        ("keywordFlags", keywords),
        ("source", source),
    ] {
        budget.charge(key.len())?;
        result.set(key, value);
    }
    // Keep missing vararg slots distinct from absent trailing arguments, and do
    // not compact nil values before assigning their source table positions.
    for (index, value) in [first, second, third]
        .into_iter()
        .flatten()
        .chain(arguments)
        .skip(tag_start)
        .enumerate()
    {
        budget.charge(0)?;
        if !matches!(value, V::Nil) {
            result.indexed.insert(index as i64 + 1, value);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn construct(tail: Vec<V>) -> ModifierTable {
        let mut args = vec![
            V::Bytes(b"Example".to_vec()),
            V::Bytes(b"LIST".to_vec()),
            V::Nil,
        ];
        args.extend(tail);
        create_mod(args, &mut OutputBudget::default()).unwrap()
    }

    #[test]
    fn constructor_preserves_sparse_tags_after_a_source_string() {
        let row = construct(vec![
            V::Bytes(vec![0xff]),
            V::Nil,
            V::Boolean(false),
            V::Number(7.0),
        ]);
        assert_eq!(row.field("source"), &V::Bytes(vec![0xff]));
        assert!(!row.indexed.contains_key(&1));
        assert_eq!(row.indexed_value(2), &V::Boolean(false));
        assert_eq!(row.indexed_value(3), &V::Number(7.0));
        assert!(!row.fields.contains_key("value"));
    }

    #[test]
    fn independent_numeric_positions_discard_earlier_nonsource_arguments() {
        let row = construct(vec![
            V::Boolean(false),
            V::Number(-0.0),
            V::Bytes(b"tag".to_vec()),
        ]);
        let V::Number(flags) = row.field("flags") else {
            panic!("number flags")
        };
        assert_eq!(flags.to_bits(), (-0.0f64).to_bits());
        assert_eq!(row.indexed_value(1), &V::Bytes(b"tag".to_vec()));
        assert!(!row.fields.contains_key("source"));
        let row = construct(vec![
            V::Nil,
            V::Bytes(b"ignored".to_vec()),
            V::Number(f64::INFINITY),
            V::Boolean(false),
        ]);
        assert_eq!(row.field("keywordFlags"), &V::Number(f64::INFINITY));
        assert_eq!(row.indexed_value(1), &V::Boolean(false));
        assert_eq!(row.indexed.len(), 1);
    }

    #[test]
    fn numeric_strings_remain_tags_and_constructor_values_are_uncoerced() {
        let row = construct(vec![
            V::Nil,
            V::Bytes(b"7".to_vec()),
            V::Bytes(b"8".to_vec()),
        ]);
        assert_eq!(row.field("flags"), &V::Number(0.0));
        assert_eq!(row.field("keywordFlags"), &V::Number(0.0));
        assert!(!row.indexed.contains_key(&1));
        assert_eq!(row.indexed_value(2), &V::Bytes(b"7".to_vec()));
        let row = create_mod(
            vec![V::Number(5.0), V::Boolean(false), V::Bytes(vec![0, 255])],
            &mut OutputBudget::default(),
        )
        .unwrap();
        assert_eq!(row.field("name"), &V::Number(5.0));
        assert_eq!(row.field("type"), &V::Boolean(false));
        assert_eq!(row.field("value"), &V::Bytes(vec![0, 255]));
    }

    #[test]
    fn missing_arguments_and_shared_table_values_remain_distinct() {
        let row = create_mod(Vec::new(), &mut OutputBudget::default()).unwrap();
        assert_eq!(row.fields.len(), 2);
        assert!(row.indexed.is_empty());
        let shared = Arc::new(ModifierTable::default());
        let row = create_mod(
            vec![
                V::Nil,
                V::Nil,
                V::Table(shared.clone()),
                V::Table(shared.clone()),
            ],
            &mut OutputBudget::default(),
        )
        .unwrap();
        let V::Table(value) = row.field("value") else {
            panic!("table value")
        };
        let V::Table(tag) = row.indexed_value(1) else {
            panic!("table tag")
        };
        assert!(Arc::ptr_eq(value, &shared));
        assert!(Arc::ptr_eq(tag, &shared));
    }

    #[test]
    fn constructor_bounds_result_storage_before_allocating_all_tag_entries() {
        let mut budget = OutputBudget::default();
        for _ in 0..value::MAX_OUTPUT_VALUES - 8 {
            budget.charge(0).unwrap();
        }
        let args = vec![V::Nil; 32];
        assert!(matches!(
            create_mod(args, &mut budget),
            Err(ParserError::ResourceBound(_))
        ));
    }
}
