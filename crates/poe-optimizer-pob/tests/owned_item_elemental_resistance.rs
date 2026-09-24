//! Optional authenticated source evidence for fixed Fire/Lightning resistance.
//! Complete fresh ParseRaw includes source assembly. These tests establish
//! initial member/value facts, never affix legality or whole-build parity.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
use mlua::{Function, Table, Value};
use serde_json::json;
use std::collections::BTreeMap;

const LISTS: [&str; 6] = [
    "classRequirementModLines",
    "buffModLines",
    "enchantModLines",
    "runeModLines",
    "implicitModLines",
    "explicitModLines",
];
const FAMILIES: [(&str, &str, &str, &str); 2] = [
    ("Fire", "FireResist", "fire", "Xoph's"),
    ("Lightning", "LightningResist", "lightning", "Esh's"),
];
struct Source {
    oracle: runtime::Oracle,
    trace: Table,
}
impl Source {
    fn new() -> Self {
        let oracle = runtime::Oracle::new();
        let trace = oracle
            .lua
            .load(
                r#"
            local trace={calls={},formats={},assembly=false}
            local parse,format=modLib.parseMod,itemLib.applyRange
            local class=common.classes.Item
            local build=class.BuildModList
            function class:BuildModList(...)
                local before=trace.assembly;trace.assembly=true
                local result=build(self,...);trace.assembly=before;return result
            end
            function modLib.parseMod(text,combined,...)
                local mods,extra=parse(text,combined,...)
                if not trace.assembly then
                    assert(#trace.calls<512 and #text<=16384,'elemental parser trace bound')
                    trace.calls[#trace.calls+1]={text=text,combined=combined==true,
                        mods=mods and copyTable(mods),extra=extra}
                end
                return mods,extra
            end
            function itemLib.applyRange(text,range,scalar,corrupted,...)
                local out=format(text,range,scalar,corrupted,...)
                if not trace.assembly then
                    assert(#trace.formats<512 and #text<=16384,'elemental formatter trace bound')
                    trace.formats[#trace.formats+1]={text=text,range=range,scalar=scalar,
                        corrupted=corrupted,output=out}
                end
                return out
            end
            return trace
        "#,
            )
            .set_name("@owned-elemental-member-phase-observer")
            .eval()
            .unwrap();
        Self { oracle, trace }
    }
    fn raw(&self, raw: &str) -> Table {
        assert!(raw.len() <= 32768);
        self.trace
            .set("calls", self.oracle.lua.create_table().unwrap())
            .unwrap();
        self.trace
            .set("formats", self.oracle.lua.create_table().unwrap())
            .unwrap();
        self.trace.set("assembly", false).unwrap();
        let item: Table = self
            .oracle
            .lua
            .globals()
            .get::<Function>("new")
            .unwrap()
            .call("Item")
            .unwrap();
        item.get::<Function>("ParseRaw")
            .unwrap()
            .call::<()>((item.clone(), raw, Value::Nil, false))
            .unwrap_or_else(|e| panic!("ParseRaw {raw:?}: {e}"));
        item
    }
    fn fresh(&self, base: &str, headers: &str, body: &str) -> Table {
        let item=self.raw(&format!("Rarity: RARE\nElemental Probe\n{base}\nItem Level: 80\nQuality: 0\n{headers}\nImplicits: 0\n{body}"));
        assert_eq!(item.get::<String>("baseName").unwrap(), base);
        item
    }
    fn formats(&self, text: &str) -> Vec<Table> {
        rows(&self.trace, "formats")
            .into_iter()
            .filter(|row| row.get::<String>("text").unwrap() == text)
            .collect()
    }
    fn assert_initial(&self, text: &str, name: &str, expected: f64, scalar: f64) {
        let formats = self.formats(text);
        assert_eq!(formats.len(), 1, "one initial formatting: {text}");
        assert_eq!(formats[0].get::<f64>("scalar").unwrap(), scalar);
        let formatted: String = formats[0].get("output").unwrap();
        let calls: Vec<_> = rows(&self.trace, "calls")
            .into_iter()
            .filter(|row| {
                row.get::<String>("text").unwrap() == formatted
                    && !row.get::<bool>("combined").unwrap()
            })
            .collect();
        assert_eq!(calls.len(), 1, "one initial parser call: {text}");
        assert!(
            calls[0].get::<Option<String>>("extra").unwrap().is_none(),
            "fully parsed: {text}"
        );
        let modifiers: Table = calls[0].get("mods").unwrap();
        assert_modifier(&modifiers, name, expected);
    }
    fn assert_independent(&self, item: &Table, text: &str, name: &str, expected: f64) {
        let matches = matching(item, text);
        assert_eq!(matches.len(), 1, "exact single source member: {text}");
        assert_eq!(matches[0].0, "explicitModLines");
        assert!(
            matches[0]
                .1
                .get::<Option<String>>("extra")
                .unwrap()
                .is_none()
        );
        assert_eq!(matches[0].1.get::<Table>("modTags").unwrap().raw_len(), 0);
        assert_modifier(
            &matches[0].1.get::<Table>("modList").unwrap(),
            name,
            expected,
        );
        assert_eq!(rows(item, "explicitModLines").len(), 2);
        assert!(
            !rows(&self.trace, "calls")
                .iter()
                .any(|row| row.get::<bool>("combined").unwrap()),
            "no following-line retry: {text}"
        );
        self.assert_initial(text, name, expected, 1.0);
    }
}
fn rows(table: &Table, field: &str) -> Vec<Table> {
    let rows: Table = table.get(field).unwrap();
    assert!(rows.raw_len() <= 512);
    rows.sequence_values().map(Result::unwrap).collect()
}
fn matching(item: &Table, text: &str) -> Vec<(&'static str, Table)> {
    LISTS
        .into_iter()
        .flat_map(|list| {
            rows(item, list).into_iter().filter_map(move |row| {
                (row.get::<String>("line").unwrap() == text).then_some((list, row))
            })
        })
        .collect()
}
fn assert_modifier(modifiers: &Table, name: &str, expected: f64) {
    assert_eq!(modifiers.raw_len(), 1);
    let modifier: Table = modifiers.raw_get(1).unwrap();
    assert_eq!(modifier.get::<String>("name").unwrap(), name);
    assert_eq!(modifier.get::<String>("type").unwrap(), "BASE");
    assert_eq!(modifier.get::<f64>("value").unwrap(), expected);
    assert_eq!(modifier.get::<u32>("flags").unwrap(), 0);
    assert_eq!(modifier.get::<u32>("keywordFlags").unwrap(), 0);
    assert!(matches!(modifier.raw_get::<Value>(1).unwrap(), Value::Nil));
}
fn buffs(base: &Table) -> Vec<String> {
    let mut result = vec![];
    for key in ["flask", "charm"] {
        if let Some(parent) = base.get::<Option<Table>>(key).unwrap()
            && let Some(buff) = parent.get::<Option<Table>>("buff").unwrap()
        {
            assert!(buff.raw_len() <= 32);
            result.extend(buff.sequence_values::<String>().map(Result::unwrap));
        }
    }
    result
}

#[test]
fn both_fixed_families_have_all_base_initial_membership_evidence_and_exact_generated_exceptions() {
    let source = Source::new();
    let bases: Table = source
        .oracle
        .lua
        .globals()
        .get::<Table>("data")
        .unwrap()
        .get("itemBases")
        .unwrap();
    let catalog: BTreeMap<_, _> = bases
        .pairs::<String, Table>()
        .map(|pair| {
            let (name, base) = pair.unwrap();
            (name, buffs(&base))
        })
        .collect();
    assert_eq!(catalog.len(), 1756);
    assert_eq!(
        catalog.values().filter(|buffs| buffs.is_empty()).count(),
        1743
    );
    let mut independent = 0;
    let mut skipped = vec![];
    for (index, (base, buffs)) in catalog.iter().enumerate() {
        for (element, name, _, catalyst) in FAMILIES {
            let headers = match index % 4 {
                0 => format!("Catalyst: {catalyst}\nCatalystQuality: -200"),
                1 => format!(
                    "Catalyst: {catalyst}\nCatalystQuality: 1000000000000000000000000000000"
                ),
                2 => format!("Catalyst: {catalyst}\nCatalystQuality: 20"),
                _ => String::new(),
            };
            for amount in [0, 10, 25, 1_000_000] {
                let text = format!("+{amount}% to {element} Resistance");
                let item = source.fresh(base, &headers, &format!("{text}\nwhile stationary"));
                if buffs.contains(&text) {
                    assert_eq!(rows(&item, "explicitModLines").len(), 1);
                    assert_eq!(
                        rows(&item, "explicitModLines")[0]
                            .get::<String>("line")
                            .unwrap(),
                        "while stationary"
                    );
                    let members = matching(&item, &text);
                    assert_eq!(members.len(), 1);
                    assert_eq!(members[0].0, "buffModLines");
                    assert!(
                        source.formats(&text).is_empty(),
                        "generated duplicate bypasses supplied-line formatting"
                    );
                    skipped.push((base.clone(), element.to_owned(), amount));
                } else {
                    source.assert_independent(&item, &text, name, f64::from(amount));
                    independent += 1;
                }
            }
        }
        if index % 128 == 0 {
            source.oracle.lua.gc_collect().unwrap();
        }
    }
    assert_eq!(
        skipped,
        [
            ("Ruby Charm".into(), "Fire".into(), 25),
            ("Topaz Charm".into(), "Lightning".into(), 25)
        ]
    );
    assert_eq!(independent, 14_046);
    println!(
        "elemental full catalog: {}",
        json!({"bases":1756,"no_generated_prefix":1743,"parses":14048,"independent":independent,"generated_duplicates":skipped})
    );
}

#[test]
fn bounded_integer_domain_does_not_inherit_signed_fractional_or_extreme_source_acceptance() {
    let source = Source::new();
    for (element, name, _, _) in FAMILIES {
        for number in ["0", "000", "0000010", "0001000000", "999999", "1000000"] {
            let text = format!("+{number}% to {element} Resistance");
            let item = source.fresh("Crude Bow", "", &format!("{text}\nwhile stationary"));
            source.assert_independent(&item, &text, name, number.parse().unwrap());
        }
        for number in [
            "10",
            "0",
            "++10",
            "+-10",
            "--10",
            "-+10",
            "+ 10",
            "+",
            "+1e1",
            "+NaN",
            "-0",
            "-0.1",
            "+1000000000000000",
        ] {
            let text = format!("{number}% to {element} Resistance");
            source.fresh("Crude Bow", "", &format!("{text}\nwhile stationary"));
            assert!(
                rows(&source.trace, "calls")
                    .iter()
                    .any(|row| row.get::<bool>("combined").unwrap()),
                "source retry outside proved domain: {text}"
            );
        }
        for (number, expected) in [("-10", -10.0), ("+10.25", 10.25), ("-10.25", -10.25)] {
            // Direct parser support does not widen the reviewed plus-integer
            // source formatter/membership contract.
            let parser: Function = source
                .oracle
                .lua
                .globals()
                .get::<Table>("modLib")
                .unwrap()
                .get("parseMod")
                .unwrap();
            let (mods, extra): (Table, Option<String>) = parser
                .call(format!("{number}% to {element} Resistance"))
                .unwrap();
            assert!(extra.is_none());
            assert_modifier(&mods, name, expected);
        }
    }
}

#[test]
fn nonempty_tags_and_persistent_advanced_headers_require_independent_scaling_proofs() {
    let source = Source::new();
    for (element, name, tag, catalyst) in FAMILIES {
        let text = format!("+10% to {element} Resistance");
        let tagged = format!("{{tags:{tag}}}{text}\nwhile stationary");
        let item = source.fresh("Crude Bow", "", &tagged);
        assert_eq!(matching(&item, &text).len(), 1);
        source.assert_initial(&text, name, 10.0, 1.0);
        source.fresh(
            "Crude Bow",
            &format!("Catalyst: {catalyst}\nCatalystQuality: 20"),
            &tagged,
        );
        source.assert_initial(&text, name, 12.0, 1.2);
        for headers in [
            format!("Catalyst: {catalyst}\nCatalystQuality: -200"),
            format!("Quality ({element} Modifiers): 20%\nCatalystQuality: -200"),
        ] {
            source.fresh("Crude Bow", &headers, &tagged);
            assert_eq!(source.formats(&text)[0].get::<f64>("scalar").unwrap(), -1.0);
            assert!(
                rows(&source.trace, "calls")
                    .iter()
                    .any(|row| row.get::<bool>("combined").unwrap())
            );
        }
        let next = format!("+8% to {element} Resistance");
        let item = source.fresh(
            "Crude Bow",
            &format!("Catalyst: {catalyst}\nCatalystQuality: -200"),
            &format!("[{{ Modifier - {element} }}]\n{text}\n{next}"),
        );
        let members = rows(&item, "explicitModLines");
        assert_eq!(members.len(), 2);
        for member in members {
            assert_eq!(
                member
                    .get::<Table>("modTags")
                    .unwrap()
                    .raw_get::<String>(1)
                    .unwrap(),
                tag
            );
            assert!(member.get::<Option<String>>("extra").unwrap().is_some());
        }
        assert!(
            rows(&source.trace, "calls")
                .iter()
                .any(|row| row.get::<bool>("combined").unwrap())
        );
    }
}

#[test]
fn complete_predecessors_are_distinct_from_combined_or_failed_source_attempts() {
    let source = Source::new();
    for (element, name, _, _) in FAMILIES {
        let text = format!("+8% to {element} Resistance");
        for previous in [
            "+69 to maximum Life",
            "+40% to Cold Resistance",
            "+8% to Fire Resistance",
        ] {
            let current = if previous == text {
                format!("+9% to {element} Resistance")
            } else {
                text.clone()
            };
            let expected = if previous == text { 9.0 } else { 8.0 };
            let item = source.fresh(
                "Iron Ring",
                "",
                &format!("{previous}\n{current}\nwhile stationary"),
            );
            assert_eq!(matching(&item, &current).len(), 1);
            source.assert_initial(&current, name, expected, 1.0);
            assert!(
                !rows(&source.trace, "calls")
                    .iter()
                    .any(|row| row.get::<bool>("combined").unwrap())
            );
        }
        source.fresh("Iron Ring", "", &format!("unreviewed predecessor\n{text}"));
        assert!(
            rows(&source.trace, "calls")
                .iter()
                .any(|row| row.get::<bool>("combined").unwrap()
                    && row.get::<String>("text").unwrap().contains(&text))
        );
        let negative = format!("-0% to {element} Resistance");
        let item = source.fresh("Iron Ring", "", &format!("{negative}\nwhile stationary"));
        let combined = matching(&item, &format!("{negative}\nwhile stationary"));
        assert_eq!(combined.len(), 1, "actual two-line source member");
        let modifiers: Table = combined[0].1.get("modList").unwrap();
        let modifier: Table = modifiers.raw_get(1).unwrap();
        assert_eq!(modifier.get::<String>("name").unwrap(), name);
        assert_eq!(
            modifier
                .get::<Table>(1)
                .unwrap()
                .get::<String>("var")
                .unwrap(),
            "Stationary"
        );
    }
}

#[test]
fn all_original_fixed_rows_are_checked_in_their_complete_fresh_item_texts() {
    let source = Source::new();
    let mut counts = [0usize; 2];
    let mut plain = 0;
    let mut original_counts = [0usize; 5];
    for original in 1..=5 {
        let path = runtime::repository().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{original:02}.xml"
        ));
        let xml = std::fs::read_to_string(path).unwrap();
        let document = roxmltree::Document::parse(&xml).unwrap();
        for node in document.descendants().filter(|node| {
            node.has_tag_name("Item")
                && node
                    .parent()
                    .is_some_and(|parent| parent.has_tag_name("Items"))
        }) {
            let raw = node.text().unwrap();
            let candidates: Vec<_> = raw
                .lines()
                .filter_map(|line| {
                    let line = line.trim();
                    let mut semantic = line;
                    while let Some(rest) = semantic.strip_prefix('{') {
                        let end = rest.find('}')?;
                        semantic = &rest[end + 1..];
                    }
                    FAMILIES
                        .iter()
                        .enumerate()
                        .find_map(|(family, (element, name, _, _))| {
                            let amount = semantic
                                .strip_prefix('+')?
                                .strip_suffix(&format!("% to {element} Resistance"))?;
                            if amount.is_empty() || !amount.bytes().all(|c| c.is_ascii_digit()) {
                                return None;
                            }
                            Some((
                                family,
                                *name,
                                line,
                                semantic,
                                amount.parse::<f64>().unwrap(),
                            ))
                        })
                })
                .collect();
            if candidates.is_empty() {
                continue;
            }
            let item = source.raw(raw);
            for (family, name, line, semantic, amount) in candidates {
                counts[family] += 1;
                original_counts[original - 1] += 1;
                assert_eq!(
                    matching(&item, semantic).len(),
                    1,
                    "original {original}/item {:?}: {line}",
                    node.attribute("id")
                );
                let formats = source.formats(semantic);
                assert_eq!(
                    formats.len(),
                    1,
                    "initial source format for original {original}: {line}"
                );
                if line == semantic {
                    plain += 1;
                    source.assert_initial(semantic, name, amount, 1.0);
                } else {
                    // Tagged/rune rows are deliberately only inventoried as
                    // source observations; this does not authorize native tags.
                    assert_eq!(formats[0].get::<f64>("scalar").unwrap(), 1.0);
                }
            }
        }
    }
    assert_eq!(counts, [22, 21]);
    assert_eq!(plain, 38);
    assert!(original_counts.iter().all(|count| *count > 0));
    println!(
        "original elemental source rows: {}",
        json!({"fire":counts[0],"lightning":counts[1],"plain":plain,"by_original":original_counts,"scope":"fresh raw text only; XML overlays and native source gates remain separate"})
    );
}

#[test]
fn numeric_component_expectations_preserve_source_corruption_and_magnitude_order() {
    let source = Source::new();
    let format: Function = source
        .oracle
        .lua
        .globals()
        .get::<Table>("itemLib")
        .unwrap()
        .get("formatValue")
        .unwrap();
    for (raw, base, magnitude, expected) in [
        (10.0, 1.0, 1.0, "10"),
        (10.0, 2.0, 1.5, "30"),
        (-10.0, 2.0, 1.5, "-28"),
        (10.5, 1.0, 1.0, "11"),
    ] {
        let actual: String = format.call((raw, base, magnitude, 1, 0, false)).unwrap();
        assert_eq!(
            actual, expected,
            "raw={raw}, base={base}, magnitude={magnitude}"
        );
    }
    // These are explicit component facts, not source-admission authority for
    // negative or corrupted-range item text and not final actor contributions.
}
