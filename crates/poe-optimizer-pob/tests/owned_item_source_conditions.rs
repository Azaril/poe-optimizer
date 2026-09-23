//! Source-reference evidence for a conditional independent-member proof.
//! These are fresh complete ParseRaw calls (including its nested BuildModList),
//! with phase-only observers. No ItemsTab/UI replay or actor evaluation runs.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;

use mlua::{Function, Table, Value};
use serde_json::json;
use std::collections::BTreeMap;

const SUFFIX: &str = "% to Cold Resistance";

fn install_observer(oracle: &runtime::Oracle) {
    oracle
        .lua
        .load(
            r#"
        sourceConditionTrace = {phase='parse_raw', calls={}, formats={}}
        local trace = sourceConditionTrace
        local parse, format = modLib.parseMod, itemLib.applyRange
        local class = common.classes.Item
        local build = class.BuildModList
        function class:BuildModList(...)
            local phase = trace.phase
            trace.phase = 'build_mod_list'
            local result = build(self, ...)
            trace.phase = phase
            return result
        end
        function modLib.parseMod(text, combined, ...)
            assert(#trace.calls < 256 and #text <= 8192, 'source condition parser trace bound')
            local mods, extra = parse(text, combined, ...)
            trace.calls[#trace.calls+1] = {text=text,combined=combined==true,
                phase=trace.phase,has_modifiers=mods~=nil,extra=extra}
            return mods, extra
        end
        function itemLib.applyRange(text, range, scalar, corrupted, ...)
            assert(#trace.formats < 256 and #text <= 8192, 'source condition format trace bound')
            local result = format(text, range, scalar, corrupted, ...)
            trace.formats[#trace.formats+1] = {text=text,range=range,scalar=scalar,
                corrupted=corrupted,output=result,phase=trace.phase}
            return result
        end
    "#,
        )
        .set_name("@source-condition-read-only-phase-observer")
        .exec()
        .unwrap();
}

fn fresh(oracle: &runtime::Oracle, base: &str, headers: &str, lines: &str) -> (Table, Table) {
    assert!(base.len() <= 512 && headers.len() <= 4096 && lines.len() <= 4096);
    let trace: Table = oracle.lua.globals().get("sourceConditionTrace").unwrap();
    trace
        .set("calls", oracle.lua.create_table().unwrap())
        .unwrap();
    trace
        .set("formats", oracle.lua.create_table().unwrap())
        .unwrap();
    trace.set("phase", "parse_raw").unwrap();
    let raw = format!(
        "Rarity: RARE\nSource Condition Probe\n{base}\nItem Level: 80\nQuality: 0\n{headers}\nImplicits: 0\n{lines}"
    );
    let item: Table = oracle
        .lua
        .globals()
        .get::<Function>("new")
        .unwrap()
        .call("Item")
        .unwrap();
    item.get::<Function>("ParseRaw")
        .unwrap()
        .call::<()>((item.clone(), raw, Value::Nil, false))
        .unwrap_or_else(|error| panic!("ParseRaw {base:?}/{lines:?}: {error}"));
    assert_eq!(item.get::<String>("baseName").unwrap(), base);
    (item, trace)
}

fn tables(table: &Table, field: &str) -> Vec<Table> {
    let values: Table = table.get(field).unwrap();
    assert!(values.raw_len() <= 256);
    values.sequence_values().map(Result::unwrap).collect()
}

fn assert_independent(item: &Table, trace: &Table, raw: &str, expected: f64) {
    let lines = tables(item, "explicitModLines");
    assert_eq!(
        lines.len(),
        2,
        "supplied member plus independent suffix: {raw}"
    );
    assert_eq!(lines[0].get::<String>("line").unwrap(), raw);
    assert_eq!(lines[1].get::<String>("line").unwrap(), "while stationary");
    assert!(lines[0].get::<Option<String>>("extra").unwrap().is_none());
    assert_eq!(lines[0].get::<Table>("modTags").unwrap().raw_len(), 0);
    let modifiers = tables(&lines[0], "modList");
    assert_eq!(modifiers.len(), 1);
    assert_eq!(modifiers[0].get::<String>("name").unwrap(), "ColdResist");
    assert_eq!(modifiers[0].get::<f64>("value").unwrap(), expected);
    let calls = tables(trace, "calls");
    assert!(
        calls
            .iter()
            .all(|call| !call.get::<bool>("combined").unwrap()),
        "combined retry for {raw}"
    );
    let formats: Vec<_> = tables(trace, "formats")
        .into_iter()
        .filter(|row| {
            row.get::<String>("phase").unwrap() == "parse_raw"
                && row.get::<String>("text").unwrap() == raw
        })
        .collect();
    assert_eq!(formats.len(), 1, "initial supplied-line format: {raw}");
    assert_eq!(formats[0].get::<f64>("scalar").unwrap(), 1.0);
    let formatted: String = formats[0].get("output").unwrap();
    assert_eq!(formatted, format!("+{expected:.0}{SUFFIX}"));
    let initial: Vec<_> = calls
        .iter()
        .filter(|row| {
            row.get::<String>("phase").unwrap() == "parse_raw"
                && row.get::<String>("text").unwrap() == formatted
        })
        .collect();
    assert_eq!(initial.len(), 1, "initial fully parsed member: {raw}");
    assert!(initial[0].get::<bool>("has_modifiers").unwrap());
    assert!(initial[0].get::<Option<String>>("extra").unwrap().is_none());
}

fn generated_buffs(base: &Table) -> Vec<String> {
    let mut lines = vec![];
    for key in ["flask", "charm"] {
        match base.get::<Value>(key).unwrap() {
            Value::Nil => {}
            Value::Table(parent) => match parent.get::<Value>("buff").unwrap() {
                Value::Nil => {}
                Value::Table(buff) => {
                    assert!(buff.raw_len() <= 32);
                    for line in buff.sequence_values::<String>() {
                        let line = line.unwrap();
                        assert!(line.len() <= 1024);
                        lines.push(line);
                    }
                }
                other => panic!("unreviewed constructed buff value: {other:?}"),
            },
            other => panic!("unreviewed constructed base parent: {other:?}"),
        }
    }
    lines
}

#[test]
fn bounded_tagless_plus_integer_is_independent_only_with_reviewed_source_context() {
    let oracle = runtime::Oracle::new();
    install_observer(&oracle);
    let data: Table = oracle.lua.globals().get("data").unwrap();
    let bases: Table = data.get("itemBases").unwrap();
    let mut catalog = BTreeMap::new();
    for pair in bases.pairs::<String, Table>() {
        let (name, base) = pair.unwrap();
        assert!(catalog.len() < 4096 && name.len() <= 512);
        catalog.insert(name, generated_buffs(&base));
    }
    assert_eq!(catalog.len(), 1756);
    assert_eq!(
        catalog.values().filter(|lines| lines.is_empty()).count(),
        1743
    );
    assert_eq!(
        catalog.values().filter(|lines| !lines.is_empty()).count(),
        13
    );
    let contexts = [
        "Catalyst: Tul's\nCatalystQuality: -200",
        "Catalyst: Tul's\nCatalystQuality: 1000000000000000000000000000000",
        "Catalyst: Tul's\nCatalystQuality: 20",
        "",
    ];
    let mut skipped = vec![];
    let mut independent = 0;
    for (index, (base, buffs)) in catalog.iter().enumerate() {
        for amount in [0, 10, 25, 1_000_000] {
            let raw = format!("+{amount}{SUFFIX}");
            let (item, trace) = fresh(
                &oracle,
                base,
                contexts[index % contexts.len()],
                &format!("{raw}\nwhile stationary"),
            );
            if buffs.contains(&raw) {
                // The source has already materialized this base buff. It consumes
                // the matching physical input line without creating another member.
                let supplied = tables(&item, "explicitModLines");
                assert_eq!(supplied.len(), 1);
                assert_eq!(
                    supplied[0].get::<String>("line").unwrap(),
                    "while stationary"
                );
                assert_eq!(
                    tables(&item, "buffModLines")
                        .iter()
                        .filter(|line| line.get::<String>("line").unwrap() == raw)
                        .count(),
                    1
                );
                assert!(
                    !tables(&trace, "formats").iter().any(|row| row
                        .get::<String>("phase")
                        .unwrap()
                        == "parse_raw"
                        && row.get::<String>("text").unwrap() == raw)
                );
                skipped.push((base.clone(), amount));
            } else {
                assert_independent(&item, &trace, &raw, f64::from(amount));
                independent += 1;
            }
        }
        if index % 128 == 0 {
            oracle.lua.gc_collect().unwrap();
        }
    }
    assert_eq!(skipped, [("Sapphire Charm".to_owned(), 25)]);
    assert_eq!(independent, 7023);
    println!(
        "source-conditioned full catalog: {}",
        json!({"bases":1756,"no_generated_prefix":1743,"generated_prefix":13,"parses":7024,"independent_observations":independent,"skipped":skipped,"native_scope":"only reviewed no-generated-prefix, tagless bounded integer lines; all existing header and predecessor guards remain"})
    );

    // The enormous catalyst quality is deliberately independent of the raw
    // component bound. Empty source modTags return unity before quality is read.
    let mut contextual = 0;
    for headers in [
        "",
        "Corrupted",
        "Quality: 20",
        "Crafted: true\nPrefix: None\nSuffix: None",
        "Catalyst: Unknown\nCatalystQuality: -200",
        contexts[0],
        contexts[1],
        contexts[2],
    ] {
        for number in ["0", "000", "0000010", "0001000000", "999999", "1000000"] {
            let raw = format!("+{number}{SUFFIX}");
            let (item, trace) = fresh(
                &oracle,
                "Crude Bow",
                headers,
                &format!("{raw}\nwhile stationary"),
            );
            assert_independent(&item, &trace, &raw, number.parse().unwrap());
            contextual += 1;
        }
    }
    println!("source-conditioned context/leading-zero cases: {contextual}");

    // Excluding raw tags alone is not sufficient when an advanced-copy section
    // prepends tags to the next physical line. This remains a separate global
    // unsupported-source control, not a permission granted by the new condition.
    let (item, trace) = fresh(
        &oracle,
        "Crude Bow",
        contexts[0],
        "{ Modifier - Cold }\n+10% to Cold Resistance\n+8% to Cold Resistance",
    );
    let lines = tables(&item, "explicitModLines");
    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[0]
            .get::<Table>("modTags")
            .unwrap()
            .raw_get::<String>(1)
            .unwrap(),
        "cold"
    );
    assert!(lines[0].get::<Option<String>>("extra").unwrap().is_some());
    assert!(
        tables(&trace, "calls")
            .iter()
            .any(|call| call.get::<bool>("combined").unwrap())
    );
    println!("advanced-copy tagless physical text retains global source-control blocker");
}

#[test]
fn prior_unconditional_source_roles_need_separate_context_and_formatting_proofs() {
    let oracle = runtime::Oracle::new();
    install_observer(&oracle);
    let apply: Function = oracle
        .lua
        .globals()
        .get::<Table>("itemLib")
        .unwrap()
        .get("applyRange")
        .unwrap();
    let parser: Function = oracle
        .lua
        .globals()
        .get::<Table>("modLib")
        .unwrap()
        .get("parseMod")
        .unwrap();
    let mut count = 0;
    for raw in [
        "(20-30)% to Cold Resistance",
        "+(0-0)% to Cold Resistance",
        "10 to maximum Life",
        "5% to all Elemental Resistances",
        "1% to Critical Hit Chance",
        "+1% to Critical Hit Chance",
        "+(1000000000000000-1000000000000000)% to Cold Resistance",
        "Adds (1000000000000000-1000000000000000) to (1000000000000000-1000000000000000) Physical Damage",
    ] {
        let formatted: String = apply.call((raw, 0.5, 1.0, 1.0)).unwrap();
        let (mods, extra): (Value, Option<String>) = parser.call(formatted.as_str()).unwrap();
        println!(
            "prior source-role formatting audit: {}",
            json!({"raw":raw,"formatted":formatted,"has_modifiers":matches!(mods,Value::Table(_)),"extra":extra})
        );
        if matches!(
            raw,
            "(20-30)% to Cold Resistance" | "+(0-0)% to Cold Resistance"
        ) {
            assert!(!formatted.starts_with('+'));
            assert!(extra.is_some());
        }
        count += 1;
    }
    for (catalyst, tags, raw) in [
        ("Flesh", "life", "+10 to maximum Life"),
        ("Xoph's", "fire", "+10% to all Elemental Resistances"),
        ("Skittering", "speed", "10% increased Attack Speed"),
    ] {
        let (item, trace) = fresh(
            &oracle,
            "Crude Bow",
            &format!("Catalyst: {catalyst}\nCatalystQuality: -200"),
            &format!("{{tags:{tags}}}{raw}\n+8% to Cold Resistance"),
        );
        let formats: Vec<_> = tables(&trace, "formats").into_iter().filter(|row| row.get::<String>("phase").unwrap() == "parse_raw").map(|row| json!({"raw":row.get::<String>("text").unwrap(),"scalar":row.get::<Option<f64>>("scalar").unwrap(),"formatted":row.get::<String>("output").unwrap()})).collect();
        let calls: Vec<_> = tables(&trace, "calls").into_iter().filter(|row| row.get::<String>("phase").unwrap() == "parse_raw").map(|row| json!({"text":row.get::<String>("text").unwrap(),"combined":row.get::<bool>("combined").unwrap(),"extra":row.get::<Option<String>>("extra").unwrap()})).collect();
        println!(
            "prior source-role catalyst audit: {}",
            json!({"catalyst":catalyst,"tags":tags,"raw":raw,"formats":formats,"calls":calls,"stored_members":tables(&item,"explicitModLines").len()})
        );
        count += 1;
    }
    assert_eq!(count, 11);
    // These observations are migration evidence, not approval of source grouping,
    // final effective values, or affix-generation legality for any old rule.
}
