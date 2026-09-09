//! Independent LuaJIT source probes. The pinned original split/search/combiner
//! spans execute unchanged; separate injected-pattern probes use the same source
//! callbacks with only their pattern literal supplied as fixture data.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Function, Lua, Table};
use poe_optimizer_engine::item_runes::*;
use poe_optimizer_engine::lua_pattern::CompileLimits;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
fn source() -> String {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendor/path-of-building-poe2/src/Classes/Item.lua"
    ))
    .unwrap()
    .replace("\r\n", "\n");
    assert_eq!(
        format!("{:x}", Sha256::digest(text.as_bytes())),
        "97341d95bcc0863280fcf60e68af9459664a5ef06f588aaa0c8db1908f85f534"
    );
    text
}
fn span(s: &str, start: usize, end: usize) -> String {
    s.lines()
        .skip(start - 1)
        .take(end - start + 1)
        .collect::<Vec<_>>()
        .join("\n")
}
fn functions(lua: &Lua, injected: bool) -> Table {
    let s = source();
    let mut split = span(&s, 1374, 1384);
    let mut combine = span(&s, 2120, 2125);
    if injected {
        split = split.replace("\"(%d%.?%d*)\"", "pattern");
        combine = combine.replace("\"(%d%.?%d*)\"", "pattern");
    }
    lua.load(format!("local t_insert=table.insert; local pattern; {split}\n{}\nreturn {{split=function(line,p) pattern=p; return getRuneLineParts(line) end, compare=compareRuneValueSets, equal=runeValueSetsEqual, add=addRuneValueSets, exceeds=runeValueSetExceeds, solve=findRuneCombination, combine=function(stored,incoming,p) pattern=p; local statOrder={{[1]={{line=stored}}}};local orderKey=1;local displayLine=incoming;local ok,err=pcall(function() {combine} end); return ok,statOrder[1].line end}}",span(&s,1404,1474))).eval().unwrap()
}
fn numbers(t: Table) -> Vec<u64> {
    t.sequence_values::<f64>()
        .map(|n| n.unwrap().to_bits())
        .collect()
}
fn cold_warm(lua: &Lua, warm: bool) {
    lua.load(if warm {
        "jit.on(); jit.opt.start('hotloop=1','hotexit=1')"
    } else {
        "jit.off(); jit.flush()"
    })
    .exec()
    .unwrap();
}
fn policy() -> VectorPolicy {
    VectorPolicy {
        missing_value: 0.0,
        epsilon: 1e-9,
    }
}
#[test]
fn unchanged_original_split_and_combination_match_byte_and_numeric_results() {
    let lua = Lua::new();
    let funcs = functions(&lua, false);
    let split: Function = funcs.get("split").unwrap();
    let combine: Function = funcs.get("combine").unwrap();
    let text = RuneText::compile(b"(%d%.?%d*)", CompileLimits::default()).unwrap();
    let mut inputs = vec![
        b"1.23".to_vec(),
        b"12.34".to_vec(),
        b"-12.34".to_vec(),
        b"-.5".to_vec(),
        b"1e-9".to_vec(),
        b"no numbers".to_vec(),
        b"0x1p2".to_vec(),
        b"1\0 2".to_vec(),
        b"99999999999999 damage".to_vec(),
        b"1 damage".to_vec(),
        b"0.00001 damage".to_vec(),
        b"2 3 4".to_vec(),
    ];
    inputs.push(vec![b'9'; 400]);
    let mut observations = 0;
    for warm in [false, true] {
        cold_warm(&lua, warm);
        for _ in 0..3 {
            for a in &inputs {
                let (key, values): (mlua::LuaString, Table) =
                    split.call(lua.create_string(a).unwrap()).unwrap();
                let native = text
                    .line_parts(a, b"#", 1.0, &mut RuneBudget::default())
                    .unwrap();
                assert_eq!(native.stripped, key.as_bytes().as_ref());
                assert_eq!(
                    native
                        .values
                        .iter()
                        .map(|n| n.to_bits())
                        .collect::<Vec<_>>(),
                    numbers(values)
                );
                for b in &inputs {
                    let (ok, result): (bool, mlua::LuaString) = combine
                        .call((lua.create_string(a).unwrap(), lua.create_string(b).unwrap()))
                        .unwrap();
                    let native = text.combine(a, b, &mut RuneBudget::default());
                    match native {
                        Ok(out) => {
                            assert!(ok, "source failed for {a:?}/{b:?}");
                            assert_eq!(out, result.as_bytes().as_ref(), "{a:?}/{b:?}")
                        }
                        Err(RuneError::Source(_)) => {
                            assert!(!ok);
                            assert_eq!(result.as_bytes().as_ref(), a)
                        }
                        e => panic!("unexpected {e:?}"),
                    };
                    observations += 1;
                }
            }
        }
    }
    eprintln!("Unchanged source combination observations: {observations}");
}
#[test]
fn injected_gsub_capture_arity_empty_matches_anchors_and_binary_text_match_lua() {
    let lua = Lua::new();
    let funcs = functions(&lua, true);
    let split: Function = funcs.get("split").unwrap();
    let combine: Function = funcs.get("combine").unwrap();
    let patterns: &[&[u8]] = &[
        b"",
        b"()",
        b"(%d*)",
        b"%d+",
        b"(%d)(%d*)",
        b"()%d*",
        b"^",
        b"^()",
        b"$",
        b"(%a+)",
        b"(.-)",
        b"(%d*)$",
        b"%z",
        b"(%z)",
        b"[",
        b"(a",
        b"%1",
    ];
    let inputs: &[&[u8]] = &[b"", b"1", b"12x34", b"ab", b"1\0 2", b"-0.5", b"nan", b" "];
    for warm in [false, true] {
        cold_warm(&lua, warm);
        for pattern in patterns {
            let native = RuneText::compile(pattern, CompileLimits::default()).unwrap();
            for a in inputs {
                let old = split.call::<(mlua::LuaString, Table)>((
                    lua.create_string(a).unwrap(),
                    lua.create_string(pattern).unwrap(),
                ));
                let new = native.line_parts(a, b"#", 1.0, &mut RuneBudget::default());
                match (old, new) {
                    (Ok((key, values)), Ok(p)) => {
                        assert_eq!(p.stripped, key.as_bytes().as_ref(), "{pattern:?}/{a:?}");
                        assert_eq!(
                            p.values.iter().map(|n| n.to_bits()).collect::<Vec<_>>(),
                            numbers(values),
                            "{pattern:?}/{a:?}"
                        )
                    }
                    (Err(_), Err(RuneError::Pattern(_))) => {}
                    (a, b) => panic!("split {pattern:?}: {a:?}/{b:?}"),
                }
                for b in inputs {
                    let (ok, old): (bool, mlua::LuaString) = combine
                        .call((
                            lua.create_string(a).unwrap(),
                            lua.create_string(b).unwrap(),
                            lua.create_string(pattern).unwrap(),
                        ))
                        .unwrap();
                    let new = native.combine(a, b, &mut RuneBudget::default());
                    match new {
                        Ok(out) => {
                            assert!(ok, "combine {pattern:?}/{a:?}/{b:?}");
                            assert_eq!(out, old.as_bytes().as_ref(), "{pattern:?}/{a:?}/{b:?}")
                        }
                        Err(RuneError::Source(_) | RuneError::Pattern(_)) => {
                            assert!(!ok, "combine {pattern:?}/{a:?}/{b:?}");
                            assert_eq!(old.as_bytes().as_ref(), *a)
                        }
                        e => panic!("unexpected {e:?}"),
                    }
                }
            }
        }
    }
}
#[test]
fn original_vector_ieee_predicates_and_addition_order_match() {
    let lua = Lua::new();
    let f = functions(&lua, false);
    let compare: Function = f.get("compare").unwrap();
    let equal: Function = f.get("equal").unwrap();
    let add: Function = f.get("add").unwrap();
    let exceeds: Function = f.get("exceeds").unwrap();
    let values = vec![
        vec![],
        vec![0.0],
        vec![-0.0],
        vec![1e-9],
        vec![1.0000001e-9],
        vec![f64::NAN],
        vec![f64::INFINITY],
        vec![f64::NEG_INFINITY],
        vec![1e16, 1.0],
        vec![2.0],
        vec![1.0, 2.0],
    ];
    for warm in [false, true] {
        cold_warm(&lua, warm);
        for a in &values {
            for b in &values {
                let mut budget = RuneBudget::default();
                let args = (a.clone(), b.clone());
                assert_eq!(
                    compare.call::<bool>(args.clone()).unwrap(),
                    compare_vectors(a, b, policy(), &mut budget).unwrap()
                );
                assert_eq!(
                    equal.call::<bool>(args.clone()).unwrap(),
                    equal_vectors(a, b, policy(), &mut budget).unwrap()
                );
                assert_eq!(
                    exceeds.call::<bool>(args.clone()).unwrap(),
                    exceeds_vector(a, b, policy(), &mut budget).unwrap()
                );
                let old = add.call::<Table>(args).unwrap();
                let new = add_vectors(a, b, policy(), &mut budget).unwrap();
                let old: Vec<f64> = old.sequence_values().map(|r| r.unwrap()).collect();
                assert_eq!(old.len(), new.len());
                for (a, b) in old.into_iter().zip(new) {
                    assert!(a.is_nan() && b.is_nan() || a.to_bits() == b.to_bits());
                }
            }
        }
    }
}
#[test]
fn unchanged_original_minimum_search_preserves_first_solution_touched_keys_and_caps() {
    let lua = Lua::new();
    let funcs = functions(&lua, false);
    let solve: Function = funcs.get("solve").unwrap();
    let groups = [
        vec![vec![2.0]],
        vec![vec![1.0], vec![2.0]],
        vec![vec![2.0], vec![2.0]],
        vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0]],
        vec![vec![0.0], vec![1.0]],
        vec![vec![-1.0], vec![2.0]],
    ];
    let targets = [
        vec![],
        vec![0.0],
        vec![1.0],
        vec![2.0],
        vec![4.0],
        vec![2.0, 1.0],
    ];
    let mut observations = 0;
    for warm in [false, true] {
        cold_warm(&lua, warm);
        for g in &groups {
            for target in &targets {
                for cap in [-1.0, 0.0, 0.5, 1.5, 3.0] {
                    for limits in [None, Some(vec![0.0]), Some(vec![1.5, 2.0, 3.0])] {
                        let table = lua.create_table().unwrap();
                        for (i, v) in g.iter().enumerate() {
                            let t = lua.create_table().unwrap();
                            t.set("name", (i + 1).to_string()).unwrap();
                            t.set("values", v.clone()).unwrap();
                            table.set(i + 1, t).unwrap();
                        }
                        let max = limits.as_ref().map(|limits| {
                            let t = lua.create_table().unwrap();
                            for (i, c) in limits.iter().enumerate() {
                                t.set((i + 1).to_string(), *c).unwrap();
                            }
                            t
                        });
                        let (old, count): (Option<Table>, Option<usize>) =
                            solve.call((table, target.clone(), cap, max)).unwrap();
                        let new = find_combination(
                            &g.iter().map(Vec::as_slice).collect::<Vec<_>>(),
                            target,
                            cap,
                            limits.as_deref(),
                            policy(),
                            &mut RuneBudget::default(),
                        )
                        .unwrap();
                        assert_eq!(count, new.as_ref().map(|r| r.count));
                        if let (Some(old), Some(new)) = (old, new) {
                            let map: BTreeMap<usize, usize> =
                                old.pairs().map(|r| r.unwrap()).collect();
                            assert_eq!(map, new.counts, "{g:?}/{target:?}/{cap}/{limits:?}");
                        }
                        observations += 1;
                    }
                }
            }
        }
    }
    eprintln!("Unchanged original search observations: {observations}");
}
#[test]
fn number_group_keys_match_lua_concatenation_and_numeric_callback_text() {
    let lua = Lua::new();
    let render: Function = lua
        .load("return function(prefix,n) return prefix .. n end")
        .eval()
        .unwrap();
    let mut values = vec![
        0.0,
        -0.0,
        1e14,
        99999999999999.0,
        1e-5,
        1.000000000000001,
        1.000000000000002,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ];
    let mut seed = 0x93a715ee88aa2233u64;
    for _ in 0..5000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        values.push(f64::from_bits(seed));
    }
    for warm in [false, true] {
        cold_warm(&lua, warm);
        for value in &values {
            let old: mlua::LuaString = render.call(("Injected:", *value)).unwrap();
            let new = number_order_key(b"Injected:", *value, &mut RuneBudget::default()).unwrap();
            assert_eq!(new, old.as_bytes().as_ref(), "{value:?}");
        }
    }
}
