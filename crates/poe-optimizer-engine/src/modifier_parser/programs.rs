//! Bounded conversion at the original public parser's first-two-value boundary.
//! Raw execution keeps cycles/arbitrary keys; the current parser graph is a DAG.
use super::value::{MAX_OUTPUT_DEPTH, ModifierTable, ModifierValue, OutputBudget};
use super::{ParseOutcome, ParserError, ParserResult};
use crate::lua_pattern::MatchBudget;
use crate::parser_program::{ProgramOutput, ProgramTableId, ProgramValue, ProgramValueGraph};
use std::collections::BTreeMap;
use std::sync::Arc;

pub(super) fn adapt(
    output: &ProgramOutput,
    budget: &mut OutputBudget,
    work: &mut MatchBudget,
) -> ParserResult<ParseOutcome> {
    adapt_graph(output.graph(), budget, work)
}

fn unsupported(stage: &'static str) -> ParserError {
    ParserError::Deferred {
        stage,
        callback: None,
    }
}
fn missing_table() -> ParserError {
    ParserError::InvalidData("program output has a missing table reference".into())
}
#[derive(Clone, Copy)]
enum Key<'a> {
    Text(&'a str),
    Integer(i64),
}
fn key(value: &ProgramValue) -> ParserResult<Key<'_>> {
    match value {
        ProgramValue::Bytes(bytes) => std::str::from_utf8(bytes)
            .map(Key::Text)
            .map_err(|_| unsupported("typed program public output key encoding")),
        ProgramValue::Number(number)
            if number.is_finite()
                && *number >= i64::MIN as f64
                && *number < -(i64::MIN as f64)
                && number.fract() == 0.0 =>
        {
            Ok(Key::Integer(*number as i64))
        }
        _ => Err(unsupported("typed program public output key kind")),
    }
}
#[derive(Clone, Copy)]
enum Visit {
    Active,
    Complete,
}
struct Check<'a> {
    graph: &'a ProgramValueGraph,
    states: BTreeMap<ProgramTableId, Visit>,
    order: Vec<ProgramTableId>,
}
impl Check<'_> {
    fn value(
        &mut self,
        value: &ProgramValue,
        depth: usize,
        budget: &mut OutputBudget,
        work: &mut MatchBudget,
    ) -> ParserResult<()> {
        work.charge(1)?;
        if depth > MAX_OUTPUT_DEPTH {
            return Err(ParserError::ResourceBound("typed program output depth"));
        }
        let bytes = if let ProgramValue::Bytes(bytes) = value {
            bytes.len()
        } else {
            0
        };
        work.charge(bytes as u64)?;
        budget.charge(bytes)?;
        let ProgramValue::Table(id) = value else {
            return Ok(());
        };
        match self.states.get(id) {
            Some(Visit::Active) => {
                return Err(ParserError::ResourceBound(
                    "typed program cyclic public copy",
                ));
            }
            Some(Visit::Complete) => return Ok(()),
            None => {}
        }
        let table = self
            .graph
            .tables
            .get(id.0.checked_sub(1).ok_or_else(missing_table)? as usize)
            .ok_or_else(missing_table)?;
        // Charge graph bookkeeping, traversal order and the eventual table/Arc
        // before allocating them. Payload/key charges below are additional.
        budget.charge(256)?;
        self.states.insert(*id, Visit::Active);
        for (raw_key, value) in &table.entries {
            work.charge(1)?;
            let key_bytes = if let ProgramValue::Bytes(bytes) = raw_key {
                bytes.len()
            } else {
                0
            };
            work.charge(key_bytes as u64)?;
            key(raw_key)?;
            // Accounts for both key and map entry before any result maps exist.
            budget.charge(key_bytes)?;
            if matches!(value, ProgramValue::Nil) {
                return Err(ParserError::InvalidData(
                    "program output contains a nil table entry".into(),
                ));
            }
            self.value(value, depth + 1, budget, work)?;
        }
        self.states.insert(*id, Visit::Complete);
        self.order.push(*id);
        Ok(())
    }
}
fn scalar(
    value: &ProgramValue,
    tables: &BTreeMap<ProgramTableId, Arc<ModifierTable>>,
) -> ParserResult<ModifierValue> {
    Ok(match value {
        ProgramValue::Nil => ModifierValue::Nil,
        ProgramValue::Boolean(value) => ModifierValue::Boolean(*value),
        ProgramValue::Number(value) => ModifierValue::Number(*value),
        ProgramValue::Bytes(bytes) => ModifierValue::Bytes(bytes.clone()),
        ProgramValue::Callback(callback) => ModifierValue::Callback(*callback),
        ProgramValue::Closure(_) | ProgramValue::DefinitionTable(_) => {
            return Err(unsupported("live session value at parser copy boundary"));
        }
        ProgramValue::Table(id) => {
            ModifierValue::Table(tables.get(id).ok_or_else(missing_table)?.clone())
        }
    })
}
fn adapt_graph(
    graph: &ProgramValueGraph,
    budget: &mut OutputBudget,
    work: &mut MatchBudget,
) -> ParserResult<ParseOutcome> {
    let first = graph.values.first().unwrap_or(&ProgramValue::Nil);
    let second = graph.values.get(1).unwrap_or(&ProgramValue::Nil);
    let root = match first {
        ProgramValue::Nil => None,
        ProgramValue::Table(id) => Some(*id),
        _ => return Err(unsupported("typed program public first return kind")),
    };
    let extra = match second {
        ProgramValue::Nil => None,
        ProgramValue::Bytes(bytes) => Some(bytes),
        _ => return Err(unsupported("typed program public second return kind")),
    };
    // Extra raw results are discarded by the source wrapper, including graphs
    // reachable only from them. No speculative conversion of those values occurs.
    let mut check = Check {
        graph,
        states: BTreeMap::new(),
        order: Vec::new(),
    };
    check.value(first, 0, budget, work)?;
    check.value(second, 0, budget, work)?;
    let mut tables = BTreeMap::<ProgramTableId, Arc<ModifierTable>>::new();
    for id in check.order {
        work.charge(1)?;
        let source = &graph.tables[id.0 as usize - 1];
        let mut table = ModifierTable::default();
        for (raw_key, value) in &source.entries {
            work.charge(1)?;
            // Preflight charged all allocations; retain a shared Arc per table.
            let duplicate = match key(raw_key)? {
                Key::Text(name) => table
                    .fields
                    .insert(name.to_owned(), scalar(value, &tables)?),
                Key::Integer(index) => table.indexed.insert(index, scalar(value, &tables)?),
            };
            if duplicate.is_some() {
                return Err(ParserError::InvalidData(
                    "duplicate program output key".into(),
                ));
            }
        }
        tables.insert(id, Arc::new(table));
    }
    let modifiers = root
        .map(|id| {
            let root = tables.remove(&id).ok_or_else(missing_table)?;
            // An acyclic root cannot be referenced by its own descendants. Dropping
            // memoized aliases therefore transfers it without a second map copy.
            drop(tables);
            Arc::try_unwrap(root).map_err(|_| {
                ParserError::InvalidData("unexpected aliased program output root".into())
            })
        })
        .transpose()?;
    Ok(ParseOutcome {
        modifiers,
        extra: extra.cloned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lua_pattern::MatchLimits;
    use crate::parser_program::ProgramTable;
    fn convert(graph: &ProgramValueGraph) -> ParserResult<ParseOutcome> {
        adapt_graph(
            graph,
            &mut OutputBudget::default(),
            &mut MatchBudget::new(MatchLimits::default()),
        )
    }
    fn table(entries: Vec<(ProgramValue, ProgramValue)>) -> ProgramTable {
        ProgramTable { entries }
    }
    fn reference(id: u32) -> ProgramValue {
        ProgramValue::Table(ProgramTableId(id))
    }
    #[test]
    fn public_adjustment_preserves_nil_empty_bytes_and_discards_extra_roots() {
        assert_eq!(
            convert(&ProgramValueGraph::default()).unwrap(),
            ParseOutcome {
                modifiers: None,
                extra: None
            }
        );
        let graph = ProgramValueGraph {
            values: vec![ProgramValue::Nil, ProgramValue::Bytes(vec![]), reference(1)],
            tables: vec![table(vec![(ProgramValue::Number(1.0), reference(1))])],
        };
        assert_eq!(
            convert(&graph).unwrap(),
            ParseOutcome {
                modifiers: None,
                extra: Some(vec![])
            }
        );
        let graph = ProgramValueGraph {
            values: vec![reference(1)],
            tables: vec![table(vec![])],
        };
        assert!(
            convert(&graph)
                .unwrap()
                .modifiers
                .unwrap()
                .indexed
                .is_empty()
        );
        for values in [
            vec![ProgramValue::Boolean(false)],
            vec![ProgramValue::Nil, ProgramValue::Boolean(false)],
        ] {
            assert!(matches!(
                convert(&ProgramValueGraph {
                    values,
                    tables: vec![]
                }),
                Err(ParserError::Deferred { .. })
            ));
        }
    }
    #[test]
    fn shared_output_tables_preserve_aliases_until_the_public_copy() {
        let graph = ProgramValueGraph {
            values: vec![reference(1)],
            tables: vec![
                table(vec![
                    (ProgramValue::Number(1.0), reference(2)),
                    (ProgramValue::Number(2.0), reference(2)),
                ]),
                table(vec![(
                    ProgramValue::Bytes(b"value".to_vec()),
                    ProgramValue::Number(-0.0),
                )]),
            ],
        };
        let result = convert(&graph).unwrap().modifiers.unwrap();
        let (ModifierValue::Table(a), ModifierValue::Table(b)) =
            (result.indexed_value(1), result.indexed_value(2))
        else {
            panic!("tables")
        };
        assert!(Arc::ptr_eq(a, b));
        assert!(
            matches!(a.field("value"), ModifierValue::Number(n) if n.to_bits() == (-0.0f64).to_bits())
        );
        let copy = super::super::value::deep_copy(
            &ModifierValue::Table(Arc::new(result)),
            &mut OutputBudget::default(),
        )
        .unwrap();
        let copy = copy.as_table().unwrap();
        let (ModifierValue::Table(a), ModifierValue::Table(b)) =
            (copy.indexed_value(1), copy.indexed_value(2))
        else {
            panic!("tables")
        };
        assert!(!Arc::ptr_eq(a, b));
    }
    #[test]
    fn cycles_and_unrepresentable_keys_are_explicit_before_result_allocation() {
        let cycle = ProgramValueGraph {
            values: vec![reference(1)],
            tables: vec![table(vec![(ProgramValue::Number(1.0), reference(1))])],
        };
        assert!(matches!(
            convert(&cycle),
            Err(ParserError::ResourceBound(_))
        ));
        for key in [
            ProgramValue::Bytes(vec![255]),
            ProgramValue::Boolean(true),
            reference(1),
            ProgramValue::Number(0.5),
            ProgramValue::Number(2.0f64.powi(63)),
        ] {
            let graph = ProgramValueGraph {
                values: vec![reference(1)],
                tables: vec![table(vec![(key, ProgramValue::Boolean(true))])],
            };
            assert!(matches!(convert(&graph), Err(ParserError::Deferred { .. })));
        }
        let graph = ProgramValueGraph {
            values: vec![reference(1)],
            tables: vec![table(vec![(
                ProgramValue::Number(i64::MIN as f64),
                ProgramValue::Bytes(vec![0, 255]),
            )])],
        };
        assert_eq!(
            convert(&graph)
                .unwrap()
                .modifiers
                .unwrap()
                .indexed_value(i64::MIN),
            &ModifierValue::Bytes(vec![0, 255])
        );
    }
    #[test]
    fn adapter_reuses_failure_inclusive_work_and_output_budgets() {
        let graph = ProgramValueGraph {
            values: vec![ProgramValue::Nil, ProgramValue::Bytes(vec![1; 5])],
            tables: vec![],
        };
        let mut output = OutputBudget::default();
        let mut work = MatchBudget::new(MatchLimits {
            max_steps: 8,
            ..MatchLimits::default()
        });
        assert_eq!(
            adapt_graph(&graph, &mut output, &mut work)
                .unwrap()
                .extra
                .unwrap()
                .len(),
            5
        );
        assert!(adapt_graph(&graph, &mut output, &mut work).is_err());
        let huge = ProgramValueGraph {
            values: vec![
                ProgramValue::Nil,
                ProgramValue::Bytes(vec![0; super::super::value::MAX_OUTPUT_BYTES]),
            ],
            tables: vec![],
        };
        let mut work = MatchBudget::new(MatchLimits {
            max_steps: super::super::value::MAX_OUTPUT_BYTES as u64 + 100,
            ..MatchLimits::default()
        });
        assert!(matches!(
            adapt_graph(&huge, &mut output, &mut work),
            Err(ParserError::ResourceBound(_))
        ));
    }
}
