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
            ParserFactoryExpr::ToNumber { value } => {
                // The closed, one-argument primitive preserves Number bits and
                // returns nil for other non-string kinds; it never calls a value.
                let value = self.factory_expr(value, callback, arguments, depth + 1)?;
                if let V::Bytes(bytes) = &value {
                    self.budget.charge(bytes.len() as u64)?;
                }
                self.output.charge(0)?;
                Ok(value.number().map(V::Number).unwrap_or(V::Nil))
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
                let values = self.factory_arguments(args, callback, arguments, depth)?;
                Ok(V::Table(Arc::new(create_mod(values, &mut self.output)?)))
            }
            ParserFactoryExpr::Flag { args, .. } => {
                // Validation proves owner.flag -> helper.mod -> original constructor.
                // Evaluate every source argument before invoking that closed helper.
                let values = self.factory_arguments(args, callback, arguments, depth)?;
                let policy = &self.parser.catalog.data().policy;
                Ok(V::Table(Arc::new(create_flag(
                    values,
                    &policy.flag_mod_type,
                    policy.flag_mod_value,
                    self.budget,
                    &mut self.output,
                )?)))
            }
        }
    }

    fn factory_arguments(
        &mut self,
        expressions: &[ParserFactoryExpr],
        callback: ParserCallbackId,
        arguments: FactoryArguments<'_>,
        depth: usize,
    ) -> ParserResult<Vec<V>> {
        let bytes = expressions
            .len()
            .checked_mul(std::mem::size_of::<V>())
            .ok_or(ParserError::ResourceBound("factory argument storage"))?;
        self.output.charge(bytes)?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(expressions.len())
            .map_err(|_| ParserError::ResourceBound("factory argument allocation"))?;
        for expression in expressions {
            values.push(self.factory_expr(expression, callback, arguments, depth + 1)?);
        }
        Ok(values)
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

/// The proven flag wrapper inserts two injected literals after the bound name.
/// Its evaluated varargs are forwarded without a second vector or nil compaction.
fn create_flag(
    arguments: Vec<V>,
    kind: &str,
    value: bool,
    work: &mut MatchBudget,
    budget: &mut OutputBudget,
) -> ParserResult<ModifierTable> {
    work.charge(kind.len() as u64 + 3)?;
    budget.charge(kind.len())?;
    budget.charge(0)?;
    let mut arguments = arguments.into_iter();
    let name = arguments.next().unwrap_or(V::Nil);
    let prefix = [name, V::Bytes(kind.as_bytes().to_vec()), V::Boolean(value)];
    create_mod(prefix.into_iter().chain(arguments), budget)
}

/// Exact positional createMod construction over owned arguments, including nil
/// holes and non-string names/types. Argument evaluation is a separate prior step.
fn create_mod(
    arguments: impl IntoIterator<Item = V>,
    budget: &mut OutputBudget,
) -> ParserResult<ModifierTable> {
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
    fn flag_zero_arguments_bind_a_missing_name_and_injected_prefix() {
        let row = create_flag(
            vec![],
            "Caller\0Type",
            false,
            &mut MatchBudget::default(),
            &mut OutputBudget::default(),
        )
        .unwrap();
        assert_eq!(row.fields.len(), 4);
        assert!(!row.fields.contains_key("name"));
        assert_eq!(row.field("type"), &V::Bytes(b"Caller\0Type".to_vec()));
        assert_eq!(row.field("value"), &V::Boolean(false));
        assert_eq!(row.field("flags"), &V::Number(0.0));
        assert_eq!(row.field("keywordFlags"), &V::Number(0.0));
        assert!(row.indexed.is_empty());
    }

    #[test]
    fn flag_nil_tail_positions_and_shared_tables_are_not_compacted_or_copied() {
        let shared = Arc::new(ModifierTable::default());
        let row = create_flag(
            vec![
                V::Table(shared.clone()),
                V::Nil,
                V::Bytes(b"7".to_vec()),
                V::Number(-0.0),
                V::Nil,
                V::Table(shared.clone()),
                V::Nil,
            ],
            "CallerType",
            false,
            &mut MatchBudget::default(),
            &mut OutputBudget::default(),
        )
        .unwrap();
        assert_eq!(row.field("flags"), &V::Number(0.0));
        let V::Number(keywords) = row.field("keywordFlags") else {
            panic!("keywords")
        };
        assert_eq!(keywords.to_bits(), (-0.0_f64).to_bits());
        assert!(!row.fields.contains_key("source"));
        assert_eq!(row.indexed.len(), 1);
        assert!(!row.indexed.contains_key(&1));
        let V::Table(tag) = row.indexed_value(2) else {
            panic!("tag")
        };
        let V::Table(name) = row.field("name") else {
            panic!("name")
        };
        assert!(Arc::ptr_eq(tag, &shared));
        assert!(Arc::ptr_eq(name, &shared));
    }

    #[test]
    fn flag_keeps_raw_source_and_independent_numeric_tail_types() {
        let row = create_flag(
            vec![
                V::Number(5.0),
                V::Bytes(vec![0xff, 0]),
                V::Number(f64::NAN),
                V::Number(f64::INFINITY),
                V::Nil,
                V::Boolean(false),
            ],
            "Kind",
            true,
            &mut MatchBudget::default(),
            &mut OutputBudget::default(),
        )
        .unwrap();
        assert_eq!(row.field("name"), &V::Number(5.0));
        assert_eq!(row.field("source"), &V::Bytes(vec![0xff, 0]));
        assert_eq!(row.field("value"), &V::Boolean(true));
        let V::Number(flags) = row.field("flags") else {
            panic!("flags")
        };
        assert!(flags.is_nan());
        assert_eq!(row.field("keywordFlags"), &V::Number(f64::INFINITY));
        assert_eq!(row.indexed_value(2), &V::Boolean(false));
        assert!(!row.indexed.contains_key(&1));
    }

    #[test]
    fn flag_prefix_copy_consumes_work_and_output_budgets_before_growth() {
        let mut work = MatchBudget::new(crate::lua_pattern::MatchLimits {
            max_steps: 2,
            ..Default::default()
        });
        assert!(matches!(
            create_flag(vec![], "", false, &mut work, &mut OutputBudget::default()),
            Err(ParserError::Scan(_))
        ));
        let mut output = OutputBudget::default();
        output.charge(value::MAX_OUTPUT_BYTES - 1).unwrap();
        assert!(matches!(
            create_flag(
                vec![],
                "Kind",
                false,
                &mut MatchBudget::default(),
                &mut output
            ),
            Err(ParserError::ResourceBound(_))
        ));
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

#[cfg(test)]
mod number_tests {
    use super::*;
    use std::sync::OnceLock;

    fn parser() -> &'static CompiledModifierParser {
        static PARSER: OnceLock<CompiledModifierParser> = OnceLock::new();
        PARSER.get_or_init(|| {
            CompiledModifierParser::new(
                poe_optimizer_data::game_data::bundled_snapshot()
                    .unwrap()
                    .modifier_parser(),
            )
            .unwrap()
        })
    }
    fn evaluate(
        expression: &ParserFactoryExpr,
        arguments: &[V],
        work: &mut MatchBudget,
        output: OutputBudget,
    ) -> ParserResult<V> {
        Run {
            parser: parser(),
            budget: work,
            output,
            source_tables: BTreeMap::new(),
        }
        .factory_expr(
            expression,
            ParserCallbackId(0), // No captured data is used by these scalar expressions.
            FactoryArguments::Raw(arguments),
            0,
        )
    }
    fn convert() -> ParserFactoryExpr {
        ParserFactoryExpr::ToNumber {
            value: Box::new(ParserFactoryExpr::Argument(0)),
        }
    }

    #[test]
    fn number_values_keep_their_bits_without_text_or_arithmetic_roundtrips() {
        for number in [
            0.0,
            -0.0,
            1e-300,
            1e300,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ] {
            let result = evaluate(
                &convert(),
                &[V::Number(number)],
                &mut MatchBudget::default(),
                OutputBudget::default(),
            )
            .unwrap();
            let V::Number(actual) = result else {
                panic!("number result")
            };
            assert_eq!(actual.to_bits(), number.to_bits());
        }
    }

    #[test]
    fn missing_and_nonconvertible_values_return_nil_without_invoking_them() {
        let values = [
            V::Nil,
            V::Boolean(false),
            V::Boolean(true),
            V::Table(Arc::new(ModifierTable::default())),
            V::Callback(ParserCallbackId(1)),
            V::Bytes(b"12\0".to_vec()),
            V::Bytes(vec![0xff]),
        ];
        for arguments in std::iter::once(&[][..]).chain(values.iter().map(std::slice::from_ref)) {
            assert_eq!(
                evaluate(
                    &convert(),
                    arguments,
                    &mut MatchBudget::default(),
                    OutputBudget::default()
                )
                .unwrap(),
                V::Nil,
            );
        }
    }

    #[test]
    fn conversion_charges_each_input_byte_and_its_result_value() {
        let input = V::Bytes([vec![b' '; 191], vec![b'7']].concat());
        let mut enough = MatchBudget::new(crate::lua_pattern::MatchLimits {
            max_steps: 194,
            ..Default::default()
        });
        assert_eq!(
            evaluate(
                &convert(),
                std::slice::from_ref(&input),
                &mut enough,
                OutputBudget::default()
            )
            .unwrap(),
            V::Number(7.0),
        );
        assert_eq!(enough.steps_used(), 194); // Two expression nodes plus192 bytes.
        let mut short = MatchBudget::new(crate::lua_pattern::MatchLimits {
            max_steps: 193,
            ..Default::default()
        });
        assert!(matches!(
            evaluate(&convert(), &[input], &mut short, OutputBudget::default()),
            Err(ParserError::Scan(_)),
        ));
        let mut output = OutputBudget::default();
        for _ in 0..value::MAX_OUTPUT_VALUES - 1 {
            output.charge(0).unwrap();
        }
        assert!(matches!(
            evaluate(
                &convert(),
                &[V::Number(1.0)],
                &mut MatchBudget::default(),
                output
            ),
            Err(ParserError::ResourceBound(_)),
        ));
    }

    #[test]
    fn conversion_keeps_child_errors_before_later_expression_work() {
        let expression = ParserFactoryExpr::Concat {
            left: Box::new(ParserFactoryExpr::ToNumber {
                value: Box::new(ParserFactoryExpr::Negate(Box::new(
                    ParserFactoryExpr::Literal(ParserFactoryLiteral::Nil),
                ))),
            }),
            // This would be invalid if reached; the left source error must win.
            right: Box::new(ParserFactoryExpr::CapturedScalar { upvalue: 0 }),
        };
        assert!(matches!(
            evaluate(&expression, &[], &mut MatchBudget::default(), OutputBudget::default()),
            Err(ParserError::SourceError(message)) if message == "arithmetic on a non-number",
        ));
    }
}
