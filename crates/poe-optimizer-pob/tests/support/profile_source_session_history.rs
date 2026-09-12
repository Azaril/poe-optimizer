//! Fixed public-parser history, recorded from source before the source host drops.
use super::*;
use mlua::{HookTriggers, VmState};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Clone, Debug, Serialize)]
pub enum Reference {
    Root(&'static str),
    Input(usize),
    Result(usize, usize),
}
pub enum Expected {
    Return(Json),
    SourceError(String),
    Unsupported(Json),
}
pub enum Step {
    Call {
        name: &'static str,
        arguments: Vec<Reference>,
        expected: Expected,
        source_inner_calls: Option<usize>,
    },
    Check {
        label: &'static str,
        values: Vec<Reference>,
        expected: Json,
    },
}
pub struct History {
    pub inputs: ProgramValueGraph,
    pub steps: Vec<Step>,
    pub hit_key: usize,
    pub baseline_row: Json,
    pub successful_public_calls: usize,
}

struct Recorder<'a> {
    lua: &'a Lua,
    parser: &'a Function,
    probes: &'a Table,
    cache: &'a Table,
    inputs: Vec<Value>,
    results: Vec<Vec<Value>>,
    steps: Vec<Step>,
    successful_public_calls: usize,
}
impl Recorder<'_> {
    fn input(&mut self, value: Value) -> Reference {
        let index = self.inputs.len();
        self.inputs.push(value);
        Reference::Input(index)
    }
    fn text(&mut self, text: &str) -> Reference {
        self.input(Value::String(self.lua.create_string(text).unwrap()))
    }
    fn value(&self, value: &Reference) -> Value {
        match value {
            Reference::Root("public.cache") => Value::Table(self.cache.clone()),
            Reference::Root(_) => panic!("unexpected state root"),
            Reference::Input(index) => self.inputs[*index].clone(),
            Reference::Result(call, index) => self.results[*call][*index].clone(),
        }
    }
    fn call(
        &mut self,
        name: &'static str,
        arguments: Vec<Reference>,
        source_error: bool,
        unsupported: bool,
        inner: Option<usize>,
    ) -> Vec<Reference> {
        let function = if name == "original.parser" {
            self.parser.clone()
        } else {
            self.probes
                .raw_get(name.strip_prefix("probe.").unwrap())
                .unwrap()
        };
        let values = arguments
            .iter()
            .map(|arg| self.value(arg))
            .collect::<Vec<_>>();
        let count = Arc::new(AtomicUsize::new(0));
        if inner.is_some() {
            let counter = count.clone();
            self.lua
                .set_hook(HookTriggers::EVERY_LINE, move |_, debug| {
                    if debug
                        .source()
                        .source
                        .as_deref()
                        .is_some_and(|path| path.ends_with("Modules/ModParser.lua"))
                        && debug.current_line() == Some(6621)
                    {
                        counter.fetch_add(1, Ordering::Relaxed);
                    }
                    Ok(VmState::Continue)
                })
                .unwrap();
        }
        let result = function.call::<MultiValue>(MultiValue::from_vec(values));
        self.lua.remove_hook();
        if let Some(expected) = inner {
            assert_eq!(count.load(Ordering::Relaxed), expected);
        }
        let (values, expected) = if source_error {
            (
                Vec::new(),
                Expected::SourceError(result.unwrap_err().to_string()),
            )
        } else {
            let values = result
                .unwrap_or_else(|e| panic!("source {name}: {e}"))
                .into_vec();
            let graph = observation::canonical(&observation::capture(&values));
            if unsupported {
                assert!(
                    values
                        .first()
                        .and_then(Value::as_table)
                        .is_some_and(|t| !t.is_empty())
                );
                (values, Expected::Unsupported(graph))
            } else {
                if name == "original.parser" {
                    self.successful_public_calls += 1;
                }
                (values, Expected::Return(graph))
            }
        };
        let index = self.results.len();
        let refs = (0..values.len())
            .map(|i| Reference::Result(index, i))
            .collect();
        self.results.push(values);
        self.steps.push(Step::Call {
            name,
            arguments,
            expected,
            source_inner_calls: inner,
        });
        refs
    }
    fn probe(&mut self, name: &'static str, args: Vec<Reference>) -> Vec<Reference> {
        self.call(name, args, false, false, None)
    }
    fn lookup(&mut self, key: &Reference) -> Reference {
        self.probe(
            "probe.lookup",
            vec![Reference::Root("public.cache"), key.clone()],
        )
        .remove(0)
    }
    fn check(&mut self, label: &'static str, values: Vec<Reference>) -> Json {
        let raw = values
            .iter()
            .map(|value| self.value(value))
            .collect::<Vec<_>>();
        let expected = observation::canonical(&observation::capture(&raw));
        self.steps.push(Step::Check {
            label,
            values,
            expected: expected.clone(),
        });
        expected
    }
    fn distinct(&mut self, left: Reference, right: Reference, expected: bool) {
        let output = self.probe("probe.distinct", vec![left, right]);
        assert_eq!(self.value(&output[0]), Value::Boolean(expected));
    }
    fn evict(&mut self, key: &Reference, nil: &Reference) {
        self.probe(
            "probe.replace",
            vec![Reference::Root("public.cache"), key.clone(), nil.clone()],
        );
    }
}

fn plain(value: &Value, depth: usize, seen: &mut BTreeSet<usize>) -> bool {
    if depth > 20 {
        return false;
    }
    match value {
        Value::Nil | Value::Boolean(_) | Value::Integer(_) | Value::Number(_) => true,
        Value::String(text) => text.to_str().is_ok(),
        Value::Table(table) => {
            table.metatable().is_none()
                && seen.insert(table.to_pointer() as usize)
                && table.clone().pairs::<Value, Value>().all(|entry| {
                    let (key, value) = entry.unwrap();
                    matches!(key, Value::Integer(_) | Value::Number(_) | Value::String(_))
                        && plain(&key, depth + 1, seen)
                        && plain(&value, depth + 1, seen)
                })
        }
        _ => false,
    }
}

pub fn record(lua: &Lua, parser: &Function, probes: &Table, cache: &Table) -> History {
    // Same eligible initialized-row selection and finite histories as public_cache.rs.
    // Keep one candidate, not thousands of mlua table handles.
    let mut eligible: Option<(mlua::LuaString, Table)> = None;
    for entry in cache.clone().pairs::<mlua::LuaString, Table>() {
        let (key, row) = entry.unwrap();
        if row
            .raw_get::<Value>(1)
            .unwrap()
            .as_table()
            .is_some_and(|t| !t.is_empty())
            && plain(&Value::Table(row.clone()), 0, &mut BTreeSet::new())
            && eligible
                .as_ref()
                .is_none_or(|(old, _)| key.as_bytes().as_ref() < old.as_bytes().as_ref())
        {
            eligible = Some((key, row));
        }
    }
    let (hit, original_row) = eligible.expect("initialized plain successful parser cache row");
    let mut r = Recorder {
        lua,
        parser,
        probes,
        cache,
        inputs: Vec::new(),
        results: Vec::new(),
        steps: Vec::new(),
        successful_public_calls: 0,
    };
    let hit = r.input(Value::String(hit));
    let Reference::Input(hit_key) = &hit else {
        unreachable!()
    };
    let hit_key = *hit_key;
    let no = r.input(Value::Boolean(false));
    let yes = r.input(Value::Boolean(true));
    let nil = r.input(Value::Nil);
    let marker = r.input(Value::Integer(123456));
    let row = r.lookup(&hit);
    let baseline_row = r.check("initialized row", vec![row.clone()]);
    let first = r.call(
        "original.parser",
        vec![hit.clone(), no.clone()],
        false,
        false,
        Some(0),
    );
    let second = r.call(
        "original.parser",
        vec![hit.clone(), yes.clone()],
        false,
        false,
        Some(0),
    );
    r.distinct(first[0].clone(), second[0].clone(), true);
    let mut joint = vec![row.clone()];
    joint.extend(first.clone());
    joint.extend(second.clone());
    r.check("cache and returned copy aliases", joint.clone());
    r.probe("probe.mutate", vec![first[0].clone(), marker]);
    r.check("caller mutation isolated", joint);
    let current = r.lookup(&hit);
    r.distinct(row.clone(), current.clone(), false);
    assert_eq!(
        r.check("initialized row unchanged", vec![current]),
        baseline_row
    );

    let mut saved = Vec::new();
    for (text, expected_inner, empty) in [
        ("native parser readiness sentinel never matches", 1, false),
        ("NATIVE PARSER READINESS SENTINEL NEVER MATCHES", 1, false),
        ("20% increased not a stat", 2, true),
    ] {
        let key = r.text(text);
        let absent = r.lookup(&key);
        assert_eq!(r.value(&absent), Value::Nil);
        let miss = r.call(
            "original.parser",
            vec![key.clone(), no.clone()],
            false,
            false,
            Some(expected_inner),
        );
        assert_eq!(miss.len(), 2);
        if empty {
            assert!(r.value(&miss[0]).as_table().is_some_and(Table::is_empty));
        } else {
            assert_eq!(r.value(&miss[0]), Value::Nil);
        }
        assert!(r.value(&miss[1]).as_string().is_some());
        let before = r.lookup(&key);
        let before_graph = r.check("new no-match row", vec![before.clone()]);
        let repeated = r.call(
            "original.parser",
            vec![key.clone(), yes.clone()],
            false,
            false,
            Some(0),
        );
        assert_eq!(
            r.check("miss pack retained", miss.clone()),
            r.check("hit pack", repeated)
        );
        let current = r.lookup(&key);
        r.distinct(before.clone(), current, false);
        r.evict(&key, &nil);
        let again = r.call(
            "original.parser",
            vec![key.clone(), no.clone()],
            false,
            false,
            Some(expected_inner),
        );
        assert_eq!(
            r.check("miss after eviction", again),
            r.check("original miss pack", miss)
        );
        let after = r.lookup(&key);
        r.distinct(before, after.clone(), true);
        assert_eq!(r.check("recreated row", vec![after.clone()]), before_graph);
        saved.push((key, after));
    }
    r.distinct(saved[0].1.clone(), saved[1].1.clone(), true);
    for invalid in [nil.clone(), no.clone()] {
        r.call(
            "original.parser",
            vec![invalid.clone(), no.clone()],
            true,
            false,
            None,
        );
        let absent = r.lookup(&invalid);
        assert_eq!(r.value(&absent), Value::Nil);
        for (key, previous) in &saved {
            let current = r.lookup(key);
            r.distinct(previous.clone(), current.clone(), false);
            r.check(
                "source-error cache prefix unchanged",
                vec![previous.clone(), current],
            );
        }
    }
    let mut positive_keys = Vec::new();
    for text in [
        "+987654 to Strength",
        "123% increased maximum Life",
        "+76% to Fire Resistance",
        "37% increased Damage",
        "Adds 13 to 17 Fire Damage",
        "987653% increased Attack Speed",
    ] {
        let key = r.text(text);
        let absent = r.lookup(&key);
        assert_eq!(r.value(&absent), Value::Nil);
        // Source succeeds. Native must retain the existing explicit copyTable frontier.
        r.call(
            "original.parser",
            vec![key.clone(), no.clone()],
            false,
            true,
            None,
        );
        let prefix = r.lookup(&key);
        assert!(r.value(&prefix).as_table().is_some());
        r.check("positive unsupported cache prefix", vec![prefix]);
        positive_keys.push(key);
    }
    for key in saved.into_iter().map(|(key, _)| key).chain(positive_keys) {
        r.evict(&key, &nil);
        let absent = r.lookup(&key);
        assert_eq!(r.value(&absent), Value::Nil);
    }
    assert_eq!(
        cache.raw_get::<Value>(r.value(&hit)).unwrap(),
        Value::Table(original_row)
    );
    let final_row = r.lookup(&hit);
    r.distinct(row, final_row.clone(), false);
    assert_eq!(
        r.check(
            "original row after raw binding restoration",
            vec![final_row]
        ),
        baseline_row
    );
    assert_eq!(r.successful_public_calls, 11);
    History {
        inputs: observation::capture(&r.inputs),
        steps: r.steps,
        hit_key,
        baseline_row,
        successful_public_calls: r.successful_public_calls,
    }
}
