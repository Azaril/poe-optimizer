//! Source-family recognition is separate from native final-value authority and
//! from affix generation legality. These tests execute authenticated source;
//! they do not translate a PoB parser or drive its UI.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;

use mlua::{Function, Table, Value};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const SUFFIX: &str = "% to Cold Resistance";
const CATEGORIES: [&str; 6] = [
    "classRequirementModLines",
    "buffModLines",
    "enchantModLines",
    "runeModLines",
    "implicitModLines",
    "explicitModLines",
];

fn fresh(oracle: &runtime::Oracle, base: &str, headers: &str, line: &str) -> Table {
    assert!(base.len() <= 512);
    let raw = format!(
        "Rarity: RARE\nCold Family Probe\n{base}\n{headers}\nItem Level: 80\nQuality: 0\nImplicits: 0\n{line}"
    );
    assert!(raw.len() <= 4096);
    let item = oracle
        .lua
        .globals()
        .get::<Function>("new")
        .unwrap()
        .call::<Table>("Item")
        .unwrap();
    // This is the saved-text ParseRaw boundary. In particular, no BuildModList,
    // ItemsTab replay, equipment activation, or source affix generator is run.
    item.get::<Function>("ParseRaw")
        .unwrap()
        .call::<()>((item.clone(), raw, Value::Nil, false))
        .unwrap_or_else(|e| panic!("ParseRaw {base:?}, {line:?}: {e}"));
    item
}

fn cold_rows(rows: Table) -> Vec<f64> {
    assert!(rows.raw_len() <= 256);
    rows.sequence_values::<Table>()
        .map(Result::unwrap)
        .filter(|row| row.get::<String>("name").unwrap() == "ColdResist")
        .map(|row| {
            assert_eq!(row.get::<String>("type").unwrap(), "BASE");
            let value: f64 = row.get("value").unwrap();
            assert!(value.is_finite());
            value
        })
        .collect()
}

fn parse_cold(oracle: &runtime::Oracle, text: &str) -> Option<f64> {
    assert!(text.len() <= 1024);
    let (mods, extra): (Value, Value) = oracle
        .lua
        .globals()
        .get::<Table>("modLib")
        .unwrap()
        .get::<Function>("parseMod")
        .unwrap()
        .call(text)
        .unwrap();
    if !matches!(extra, Value::Nil) {
        return None;
    }
    let Value::Table(mods) = mods else {
        return None;
    };
    let rows = cold_rows(mods);
    assert!(rows.len() <= 1, "ambiguous ColdResist rows: {text}");
    rows.first().copied()
}

fn matching_lines(item: &Table, text: &str) -> Vec<(&'static str, Table)> {
    let mut found = vec![];
    for category in CATEGORIES {
        let lines: Table = item.get(category).unwrap();
        assert!(lines.raw_len() <= 256);
        for line in lines.sequence_values::<Table>() {
            let line = line.unwrap();
            if line.get::<String>("line").unwrap() == text {
                found.push((category, line));
            }
        }
    }
    found
}

fn one_line(item: &Table, text: &str, category: &str) -> Table {
    let mut found = matching_lines(item, text);
    assert_eq!(found.len(), 1, "one stored member: {text}");
    let (actual, line) = found.remove(0);
    assert_eq!(actual, category, "member category: {text}");
    line
}

#[test]
fn pinned_fixed_cold_family_is_base_independent_but_not_source_or_numeric_authority() {
    let oracle = runtime::Oracle::new();
    let item_lib: Table = oracle.lua.globals().get("itemLib").unwrap();
    let apply: Function = item_lib.get("applyRange").unwrap();
    let mut syntax = vec![];
    // Explicit source expectations, not the proposed native lexical grammar.
    // Fractional source values are also tested even if a native successor starts
    // with a stricter integer grammar.
    for (number, expected) in [
        ("+10", Some(10.0)),
        ("+8", Some(8.0)),
        ("+34", Some(34.0)),
        ("+1000000000000000", Some(1_000_000_000_000_000.0)),
        ("+0", Some(0.0)),
        ("-0", Some(0.0)),
        ("-0.1", Some(-0.1)),
        ("-10", Some(-10.0)),
        ("+10.25", Some(10.25)),
        ("-10.25", Some(-10.25)),
        ("+0.5", Some(0.5)),
        ("-0.5", Some(-0.5)),
        ("10", None),
        ("0", None),
        ("10.25", None),
        ("++10", None),
        ("+-10", None),
        ("--10", None),
        ("-+10", None),
        ("+ 10", None),
        ("+", None),
        ("", None),
        ("+1e1", None),
        ("+NaN", None),
    ] {
        let text = format!("{number}{SUFFIX}");
        let direct = parse_cold(&oracle, &text);
        assert_eq!(direct, expected, "direct ModParser: {text}");
        let formatted: String = apply.call((text.as_str(), 0.5, 1.0, 1.0)).unwrap();
        let formatted_cold = parse_cold(&oracle, &formatted);
        let item = fresh(&oracle, "Crude Bow", "", &text);
        let lines = matching_lines(&item, &text);
        let cached: Vec<_> = lines
            .iter()
            .flat_map(|(_, line)| cold_rows(line.get("modList").unwrap()))
            .collect();
        // ParseRaw:1323-1325 retains a partial modList together with `extra`.
        // A cached ColdResist row is not itself a fully accepted source line.
        let extras: Vec<Option<String>> = lines
            .iter()
            .map(|(_, line)| line.get("extra").unwrap())
            .collect();
        let accepted: Vec<_> = lines
            .iter()
            .zip(&extras)
            .filter(|(_, extra)| extra.is_none())
            .flat_map(|((_, line), _)| cold_rows(line.get("modList").unwrap()))
            .collect();
        let observation = json!({"raw":text,"direct":direct,"formatted":formatted,"cached":cached,"extras":extras,"accepted":accepted});
        println!("cold syntax observation: {observation}");
        assert_eq!(
            accepted,
            formatted_cold.into_iter().collect::<Vec<_>>(),
            "complete ParseRaw acceptance versus actual applyRange/ModParser: {text} -> {formatted}"
        );
        if matches!(number, "-0" | "-0.1") {
            assert_eq!(direct, expected);
            assert_eq!(formatted, "0% to Cold Resistance");
            assert_eq!(cached, [0.0]);
            assert_eq!(extras.len(), 1);
            assert!(extras[0].is_some());
            assert!(accepted.is_empty());
        }
        syntax.push(observation);
    }
    println!("cold syntax: {}", serde_json::to_string(&syntax).unwrap());

    // Failed/partial parsing can change physical member boundaries. The observer
    // delegates every call to the existing authenticated parser and records its
    // result; it does not implement the formatting, matching, or combine logic.
    let parser: Table = oracle.lua.globals().get("modLib").unwrap();
    let observe: Function = oracle
        .lua
        .load(
            r#"
        return function(original, trace)
            return function(text, combined, ...)
                assert(#trace < 32 and #text <= 4096, 'cold-family trace bound')
                local mods, extra = original(text, combined, ...)
                trace[#trace + 1] = {
                    text=text, combined=combined==true,
                    has_modifiers=mods~=nil, extra=extra
                }
                return mods, extra
            end
        end
    "#,
        )
        .set_name("@cold-family-test-only-parser-observer")
        .eval()
        .unwrap();
    let mut following_lines = vec![];
    for (number, next, attempts, combines) in [
        ("-0", "+10% to Cold Resistance", true, false),
        ("-0.1", "+10% to Cold Resistance", true, false),
        ("-0", "while stationary", true, true),
        ("-0.1", "while stationary", true, true),
        ("+0", "while stationary", false, false),
        ("+1000000000000000", "+10% to Cold Resistance", true, false),
        ("+1000000000000000", "while stationary", true, true),
    ] {
        let original: Function = parser.get("parseMod").unwrap();
        let trace = oracle.lua.create_table().unwrap();
        let wrapped: Function = observe.call((original.clone(), trace.clone())).unwrap();
        parser.set("parseMod", wrapped).unwrap();
        let first = format!("{number}{SUFFIX}");
        let combined = format!("{first}\n{next}");
        let item = fresh(&oracle, "Crude Bow", "", &combined);
        parser.set("parseMod", original).unwrap();
        let calls: Vec<Table> = trace.sequence_values().map(Result::unwrap).collect();
        let joined: Vec<_> = calls
            .iter()
            .filter(|call| call.get::<bool>("combined").unwrap())
            .collect();
        assert_eq!(joined.len(), usize::from(attempts), "retry: {combined}");
        if let Some(joined) = joined.first() {
            assert!(joined.get::<String>("text").unwrap().contains(next));
            assert_eq!(
                joined.get::<Option<String>>("extra").unwrap().is_none(),
                combines,
                "complete combined parser result: {combined}"
            );
        }
        let line = if combines {
            assert_eq!(item.get::<Table>("explicitModLines").unwrap().raw_len(), 1);
            let line = one_line(&item, &combined, "explicitModLines");
            assert!(line.get::<Option<String>>("extra").unwrap().is_none());
            assert_eq!(
                cold_rows(line.get("modList").unwrap()),
                [number.parse::<f64>().unwrap()]
            );
            let modifier: Table = line.get::<Table>("modList").unwrap().raw_get(1).unwrap();
            let condition: Table = modifier.raw_get(1).unwrap();
            assert_eq!(condition.get::<String>("var").unwrap(), "Stationary");
            line
        } else {
            assert_eq!(item.get::<Table>("explicitModLines").unwrap().raw_len(), 2);
            let line = one_line(&item, &first, "explicitModLines");
            assert_eq!(
                line.get::<Option<String>>("extra").unwrap().is_some(),
                attempts
            );
            let next_line = one_line(&item, next, "explicitModLines");
            if next.starts_with('+') {
                assert_eq!(cold_rows(next_line.get("modList").unwrap()), [10.0]);
                assert!(next_line.get::<Option<String>>("extra").unwrap().is_none());
            }
            line
        };
        let calls: Vec<_> = calls
            .iter()
            .map(|call| {
                json!({
                    "text":call.get::<String>("text").unwrap(),
                    "combined":call.get::<bool>("combined").unwrap(),
                    "has_modifiers":call.get::<bool>("has_modifiers").unwrap(),
                    "extra":call.get::<Option<String>>("extra").unwrap()
                })
            })
            .collect();
        let result = json!({"first":first,"next":next,"attempts":attempts,"combines":combines,
            "stored":line.get::<String>("line").unwrap(),
            "cached":cold_rows(line.get("modList").unwrap()),
            "extra":line.get::<Option<String>>("extra").unwrap(),"calls":calls});
        println!("cold following-line observation: {result}");
        following_lines.push(result);
    }
    // A value can be inside the native raw range and still take a different
    // source formatting path. Leading-dot decimal syntax misses the source's
    // generic scalability key; a matched catalyst then exercises its fallback.
    let leading = "+.99999999999999999999% to Cold Resistance";
    assert_eq!(parse_cold(&oracle, leading), Some(1.0));
    let original: Function = parser.get("parseMod").unwrap();
    let trace = oracle.lua.create_table().unwrap();
    let wrapped: Function = observe.call((original.clone(), trace.clone())).unwrap();
    parser.set("parseMod", wrapped).unwrap();
    let leading_item = fresh(
        &oracle,
        "Crude Bow",
        "Catalyst: Tul's\nCatalystQuality: 20",
        &format!("{{tags:cold}}{leading}\n+10% to Cold Resistance"),
    );
    parser.set("parseMod", original).unwrap();
    assert_eq!(
        leading_item
            .get::<Table>("explicitModLines")
            .unwrap()
            .raw_len(),
        2
    );
    let line = one_line(&leading_item, leading, "explicitModLines");
    assert!(line.get::<Option<String>>("extra").unwrap().is_some());
    let next_line = one_line(&leading_item, "+10% to Cold Resistance", "explicitModLines");
    assert_eq!(cold_rows(next_line.get("modList").unwrap()), [10.0]);
    let calls: Vec<Table> = trace.sequence_values().map(Result::unwrap).collect();
    let attempts: Vec<_> = calls
        .iter()
        .filter(|call| call.get::<bool>("combined").unwrap())
        .collect();
    assert_eq!(attempts.len(), 1);
    assert!(
        attempts[0]
            .get::<Option<String>>("extra")
            .unwrap()
            .is_some()
    );
    let calls: Vec<_> = calls
        .iter()
        .map(|call| {
            json!({
                "text":call.get::<String>("text").unwrap(),
                "combined":call.get::<bool>("combined").unwrap(),
                "extra":call.get::<Option<String>>("extra").unwrap()
            })
        })
        .collect();
    let observation = json!({"case":"in-range-leading-dot-with-matched-catalyst","raw":leading,
        "direct":1.0,"extra":line.get::<Option<String>>("extra").unwrap(),"calls":calls});
    println!("cold following-line observation: {observation}");
    following_lines.push(observation);
    // Even an integer within the raw component envelope is not a context-free
    // single-member proof. A matching catalyst with negative quality preserves
    // the authored '+' while substituting a negative numeric component.
    for (quality, flags, expected, retry) in [
        (20, "{tags:cold}", Some(12.0), false),
        (-200, "{tags:cold}", None, true),
        (-200, "", Some(10.0), false),
    ] {
        let original: Function = parser.get("parseMod").unwrap();
        let trace = oracle.lua.create_table().unwrap();
        let wrapped: Function = observe.call((original.clone(), trace.clone())).unwrap();
        parser.set("parseMod", wrapped).unwrap();
        let item = fresh(
            &oracle,
            "Crude Bow",
            &format!("Catalyst: Tul's\nCatalystQuality: {quality}"),
            &format!("{flags}+10% to Cold Resistance\n+8% to Cold Resistance"),
        );
        parser.set("parseMod", original).unwrap();
        assert_eq!(item.get::<Table>("explicitModLines").unwrap().raw_len(), 2);
        let first = one_line(&item, "+10% to Cold Resistance", "explicitModLines");
        let extra = first.get::<Option<String>>("extra").unwrap();
        assert_eq!(extra.is_some(), retry);
        if let Some(expected) = expected {
            assert_eq!(cold_rows(first.get("modList").unwrap()), [expected]);
        }
        let next = one_line(&item, "+8% to Cold Resistance", "explicitModLines");
        assert_eq!(cold_rows(next.get("modList").unwrap()), [8.0]);
        assert!(next.get::<Option<String>>("extra").unwrap().is_none());
        let calls: Vec<Table> = trace.sequence_values().map(Result::unwrap).collect();
        let attempts: Vec<_> = calls
            .iter()
            .filter(|call| call.get::<bool>("combined").unwrap())
            .collect();
        assert_eq!(attempts.len(), usize::from(retry));
        if retry {
            assert!(
                attempts[0]
                    .get::<Option<String>>("extra")
                    .unwrap()
                    .is_some()
            );
            assert!(
                calls.iter().any(|call| {
                    call.get::<String>("text").unwrap() == "+-10% to Cold Resistance"
                        && call.get::<Option<String>>("extra").unwrap().is_some()
                }),
                "negative catalyst formatting must produce an incomplete signed line"
            );
        }
        let calls: Vec<_> = calls
            .iter()
            .map(|call| {
                json!({
                    "text":call.get::<String>("text").unwrap(),
                    "combined":call.get::<bool>("combined").unwrap(),
                    "extra":call.get::<Option<String>>("extra").unwrap()
                })
            })
            .collect();
        let observation = json!({"case":"integer-with-authored-catalyst-context",
            "quality":quality,"flags":flags,"raw":"+10% to Cold Resistance",
            "accepted":expected,"retry":retry,"extra":extra,"calls":calls});
        println!("cold following-line observation: {observation}");
        following_lines.push(observation);
    }
    // Enumerate the actual constructed catalog, not the original build's names
    // or an independently maintained hand-picked list. Bound and sort it before
    // running one fresh complete source parser invocation for each definition.
    let bases: Table = oracle
        .lua
        .globals()
        .get::<Table>("data")
        .unwrap()
        .get("itemBases")
        .unwrap();
    let mut names = BTreeMap::new();
    for pair in bases.pairs::<String, Table>() {
        let (name, base) = pair.unwrap();
        assert!(names.len() < 4096 && name.len() <= 512);
        assert!(
            names
                .insert(name, base.get::<String>("type").unwrap())
                .is_none()
        );
    }
    assert_eq!(names.len(), 1756, "pinned constructed catalog changed");
    let mut category_counts = BTreeMap::<String, usize>::new();
    for (index, (name, kind)) in names.iter().enumerate() {
        let item = fresh(&oracle, name, "", "+10% to Cold Resistance");
        assert_eq!(
            item.get::<String>("baseName").unwrap(),
            *name,
            "exact constructed base selection"
        );
        let line = one_line(&item, "+10% to Cold Resistance", "explicitModLines");
        assert_eq!(cold_rows(line.get("modList").unwrap()), [10.0], "{name}");
        assert!(line.get::<Option<String>>("extra").unwrap().is_none());
        assert_eq!(line.get::<Table>("modTags").unwrap().raw_len(), 0);
        *category_counts.entry(kind.clone()).or_default() += 1;
        // Permit collection of fresh discarded Item tables during the bounded
        // full-catalog pass without keeping an aggregate source object graph.
        if index % 64 == 63 {
            oracle.lua.gc_collect().unwrap();
        }
    }
    for required in ["Boots", "Ring", "Jewel", "Charm", "Flask", "Bow"] {
        assert!(category_counts.contains_key(required), "missing {required}");
    }

    // These are complete source observations, NOT permission to admit these
    // controls into the native ordinary fixed-line path. The raw text stays
    // nominal even when a modifier's initial numeric cache changes.
    let mut controls = vec![];
    for (flags, category, field, amount) in [
        ("{corruptedRange:1.5}", "explicitModLines", None, 15.0),
        ("{rune}", "runeModLines", Some("rune"), 10.0),
        ("{crafted}", "explicitModLines", Some("crafted"), 10.0),
        ("{fractured}", "explicitModLines", Some("fractured"), 10.0),
        ("{desecrated}", "explicitModLines", Some("desecrated"), 10.0),
        ("{unscalable}", "explicitModLines", Some("unscalable"), 10.0),
        ("{unknown:opaque}", "explicitModLines", None, 10.0),
    ] {
        let text = format!("{flags}+10{SUFFIX}");
        let item = fresh(&oracle, "Crude Bow", "", &text);
        let line = one_line(&item, "+10% to Cold Resistance", category);
        assert_eq!(cold_rows(line.get("modList").unwrap()), [amount], "{flags}");
        if let Some(field) = field {
            assert!(line.get::<bool>(field).unwrap(), "{flags}");
        }
        let corruption = line.get::<Option<f64>>("corruptedRange").unwrap();
        assert_eq!(corruption, (amount == 15.0).then_some(1.5));
        controls.push(json!({"control":flags,"category":category,"raw":"+10% to Cold Resistance","parsed":amount,"corrupted_range":corruption}));
    }
    let tagged = fresh(
        &oracle,
        "Crude Bow",
        "",
        "{tags:cold_resistance,elemental,cold}+10% to Cold Resistance",
    );
    let tagged = one_line(&tagged, "+10% to Cold Resistance", "explicitModLines");
    let tags: Vec<String> = tagged
        .get::<Table>("modTags")
        .unwrap()
        .sequence_values()
        .map(Result::unwrap)
        .collect();
    assert_eq!(tags, ["cold_resistance", "elemental", "cold"]);
    assert_eq!(cold_rows(tagged.get("modList").unwrap()), [10.0]);

    let selected = fresh(
        &oracle,
        "Crude Bow",
        "Variant: One",
        "{variant:2}+10% to Cold Resistance",
    );
    let line = one_line(&selected, "+10% to Cold Resistance", "explicitModLines");
    assert!(
        line.get::<Table>("variantList")
            .unwrap()
            .raw_get::<bool>(2)
            .unwrap()
    );
    assert_eq!(cold_rows(line.get("modList").unwrap()), [10.0]);
    assert!(
        !selected
            .get::<Function>("CheckModLineVariant")
            .unwrap()
            .call::<bool>((selected.clone(), line))
            .unwrap(),
        "recognized family does not mean selected/active member"
    );

    // Item corruption and a per-member corruptedRange are independent facts.
    let corrupted = fresh(&oracle, "Crude Bow", "Corrupted", "+10% to Cold Resistance");
    assert!(corrupted.get::<bool>("corrupted").unwrap());
    let line = one_line(&corrupted, "+10% to Cold Resistance", "explicitModLines");
    assert!(line.get::<Option<f64>>("corruptedRange").unwrap().is_none());
    assert_eq!(cold_rows(line.get("modList").unwrap()), [10.0]);

    let pins: Vec<_> = [
        "src/Classes/Item.lua",
        "src/Modules/ModParser.lua",
        "src/Modules/ItemTools.lua",
        "src/Modules/Data.lua",
        "src/Data/ModScalability.lua",
    ]
    .into_iter()
    .map(|path| {
        let text = runtime::verified(path).unwrap();
        json!({"path":path,"sha256_lf":format!("{:x}",Sha256::digest(text.as_bytes()))})
    })
    .collect();
    println!("{}", serde_json::to_string_pretty(&json!({
        "scope":"complete fresh pinned ParseRaw and actual ModParser/applyRange; family recognition only, no UI replay, final build output or affix legality",
        "constructed_bases":names.len(),"categories":category_counts,
        "syntax_cases":syntax.len(),"following_lines":following_lines,"controls":controls,"source_pins":pins,
        "native_admission":"separately validated; recognized source controls do not remove native attribution or eligibility gaps"
    })).unwrap());
}
