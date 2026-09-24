//! Optional pinned-source evidence for ordinary item attribute inputs.
//! Fresh, complete ParseRaw executes original assembly; observed source members
//! and integer formatting do not certify native eligibility or actor totals.
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
const FAMILIES: [(&str, &[&str]); 4] = [
    ("Strength", &["Str"]),
    ("Dexterity", &["Dex"]),
    ("Intelligence", &["Int"]),
    ("all Attributes", &["Str", "Dex", "Int", "All"]),
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
                        assert(#trace.calls<512 and #text<=16384,'attribute parser trace bound')
                        trace.calls[#trace.calls+1]={text=text,combined=combined==true,
                            mods=mods and copyTable(mods),extra=extra}
                    end
                    return mods,extra
                end
                function itemLib.applyRange(text,range,scalar,corrupted,...)
                    local out=format(text,range,scalar,corrupted,...)
                    if not trace.assembly then
                        assert(#trace.formats<512 and #text<=16384,'attribute formatter trace bound')
                        trace.formats[#trace.formats+1]={text=text,range=range,scalar=scalar,
                            corrupted=corrupted,output=out}
                    end
                    return out
                end
                return trace
                "#,
            )
            .set_name("@owned-attribute-initial-member-observer")
            .eval()
            .unwrap();
        Self { oracle, trace }
    }

    fn raw(&self, raw: &str) -> Table {
        assert!(raw.len() <= 32768);
        for field in ["calls", "formats"] {
            self.trace
                .set(field, self.oracle.lua.create_table().unwrap())
                .unwrap();
        }
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
            .unwrap_or_else(|error| panic!("ParseRaw {raw:?}: {error}"));
        item
    }

    fn fresh(&self, base: &str, headers: &str, body: &str) -> Table {
        let item = self.raw(&format!(
            "Rarity: RARE\nAttribute Probe\n{base}\nItem Level: 80\nQuality: 0\n{headers}\nImplicits: 0\n{body}"
        ));
        assert_eq!(item.get::<String>("baseName").unwrap(), base);
        item
    }

    fn formats(&self, text: &str) -> Vec<Table> {
        rows(&self.trace, "formats")
            .into_iter()
            .filter(|row| row.get::<String>("text").unwrap() == text)
            .collect()
    }

    fn assert_initial(&self, text: &str, names: &[&str], value: f64, scalar: f64) {
        let formats = self.formats(text);
        assert_eq!(formats.len(), 1, "one initial formatting of {text}");
        assert_eq!(formats[0].get::<f64>("scalar").unwrap(), scalar);
        let formatted: String = formats[0].get("output").unwrap();
        let calls: Vec<_> = rows(&self.trace, "calls")
            .into_iter()
            .filter(|row| {
                row.get::<String>("text").unwrap() == formatted
                    && !row.get::<bool>("combined").unwrap()
            })
            .collect();
        assert_eq!(calls.len(), 1, "one initial parse of {text}: {formatted}");
        assert!(
            calls[0].get::<Option<String>>("extra").unwrap().is_none(),
            "complete initial parse: {text} -> {formatted}"
        );
        assert_modifiers(&calls[0].get::<Table>("mods").unwrap(), names, value);
    }

    fn assert_independent(&self, item: &Table, text: &str, names: &[&str], value: f64) {
        let members = matching(item, text);
        assert_eq!(members.len(), 1, "one physical member: {text}");
        assert_eq!(members[0].0, "explicitModLines");
        assert!(
            members[0]
                .1
                .get::<Option<String>>("extra")
                .unwrap()
                .is_none()
        );
        assert_eq!(members[0].1.get::<Table>("modTags").unwrap().raw_len(), 0);
        assert_modifiers(&members[0].1.get::<Table>("modList").unwrap(), names, value);
        assert_eq!(rows(item, "explicitModLines").len(), 2);
        self.assert_no_combination();
        self.assert_initial(text, names, value, 1.0);
    }

    fn assert_no_combination(&self) {
        assert!(
            !rows(&self.trace, "calls")
                .iter()
                .any(|row| row.get::<bool>("combined").unwrap())
        );
    }

    fn parser(&self) -> Function {
        self.oracle
            .lua
            .globals()
            .get::<Table>("modLib")
            .unwrap()
            .get("parseMod")
            .unwrap()
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

fn assert_modifiers(modifiers: &Table, names: &[&str], value: f64) {
    assert_eq!(modifiers.raw_len(), names.len());
    for (index, name) in names.iter().enumerate() {
        let modifier: Table = modifiers.raw_get(index + 1).unwrap();
        assert_eq!(modifier.get::<String>("name").unwrap(), *name);
        assert_eq!(modifier.get::<String>("type").unwrap(), "BASE");
        assert_eq!(modifier.get::<f64>("value").unwrap(), value);
        assert_eq!(modifier.get::<u32>("flags").unwrap(), 0);
        assert_eq!(modifier.get::<u32>("keywordFlags").unwrap(), 0);
        assert!(matches!(modifier.raw_get::<Value>(1).unwrap(), Value::Nil));
    }
}

fn attribute_records(item: &Table) -> Vec<(String, f64)> {
    rows(item, "baseModList")
        .into_iter()
        .filter_map(|modifier| {
            let name: String = modifier.get("name").unwrap();
            matches!(name.as_str(), "Str" | "Dex" | "Int" | "All")
                .then(|| (name, modifier.get::<f64>("value").unwrap()))
        })
        .collect()
}

fn buffs(base: &Table) -> Vec<String> {
    let mut result = vec![];
    for field in ["flask", "charm"] {
        if let Some(parent) = base.get::<Option<Table>>(field).unwrap()
            && let Some(buff) = parent.get::<Option<Table>>("buff").unwrap()
        {
            assert!(buff.raw_len() <= 32);
            result.extend(buff.sequence_values::<String>().map(Result::unwrap));
        }
    }
    result
}

#[test]
fn all_constructed_bases_keep_one_supplied_attribute_member_and_no_implicit_duplicate() {
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
    let mut parses = 0;
    for (index, (base, generated)) in catalog.iter().enumerate() {
        let headers = match index % 4 {
            0 => "Catalyst: Adaptive\nCatalystQuality: -200",
            1 => "Catalyst: Adaptive\nCatalystQuality: 1000000000000000000000000000000",
            2 => "Catalyst: Adaptive\nCatalystQuality: 20",
            _ => "",
        };
        for (family, names) in FAMILIES {
            for amount in [0, 1_000_000] {
                let text = format!("+{amount} to {family}");
                assert!(!generated.contains(&text), "generated exception: {base}");
                let item = source.fresh(base, headers, &format!("{text}\nwhile stationary"));
                source.assert_independent(&item, &text, names, f64::from(amount));
                assert_eq!(
                    attribute_records(&item),
                    names
                        .iter()
                        .map(|name| ((*name).to_owned(), f64::from(amount)))
                        .collect::<Vec<_>>(),
                    "no source-assembly implicit duplication on {base}: {text}"
                );
                parses += 1;
            }
        }
        if index % 128 == 0 {
            source.oracle.lua.gc_collect().unwrap();
        }
    }
    assert_eq!(parses, 14_048);
    println!(
        "attribute full-catalog source observations: {}",
        json!({"bases":1756,"no_generated_prefix":1743,"fresh_parses":parses,
            "scope":"supplied member and source assembly; no native guard or actor-total claim"})
    );
}

#[test]
fn plus_integer_membership_does_not_authorize_sign_decimal_or_scientific_aliases() {
    let source = Source::new();
    for (family, names) in FAMILIES {
        for number in ["0", "000", "0000010", "1", "10", "999999", "0001000000"] {
            let text = format!("+{number} to {family}");
            let item = source.fresh("Iron Ring", "", &format!("{text}\nwhile stationary"));
            source.assert_independent(&item, &text, names, number.parse().unwrap());
        }
        // Outside the conservative declaration bound can still parse. The
        // proposed source guard is deliberately not the full parser language.
        let text = format!("+1000001 to {family}");
        let item = source.fresh("Iron Ring", "", &format!("{text}\nwhile stationary"));
        source.assert_independent(&item, &text, names, 1_000_001.0);
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
            let text = format!("{number} to {family}");
            source.fresh("Iron Ring", "", &format!("{text}\nwhile stationary"));
            assert!(
                rows(&source.trace, "calls")
                    .iter()
                    .any(|row| row.get::<bool>("combined").unwrap()),
                "source retry outside plus-integer contract: {text}"
            );
        }
        for (number, direct, formatted) in [
            ("-10", -10.0, -10.0),
            ("+10.25", 10.25, 10.0),
            ("+10.5", 10.5, 11.0),
            ("-10.25", -10.25, -10.0),
            ("-10.5", -10.5, -11.0),
        ] {
            let text = format!("{number} to {family}");
            let (mods, extra): (Table, Option<String>) = source.parser().call(&*text).unwrap();
            assert!(extra.is_none());
            assert_modifiers(&mods, names, direct);
            source.fresh("Iron Ring", "", &format!("{text}\nwhile stationary"));
            source.assert_initial(&text, names, formatted, 1.0);
            source.assert_no_combination();
        }
    }
}

#[test]
fn scalability_quantizes_before_magnitude_and_uses_attribute_counts() {
    let source = Source::new();
    let data: Table = source.oracle.lua.globals().get("data").unwrap();
    let scalability: Table = data.get("modScalability").unwrap();
    let formatter: Function = source
        .oracle
        .lua
        .globals()
        .get::<Table>("itemLib")
        .unwrap()
        .get("applyRange")
        .unwrap();
    for (family, names) in FAMILIES {
        let entry: Table = scalability.get(format!("# to {family}")).unwrap();
        assert_eq!(entry.raw_len(), 1);
        let field: Table = entry.raw_get(1).unwrap();
        assert!(field.get::<bool>("isScalable").unwrap());
        assert!(matches!(field.get::<Value>("formats").unwrap(), Value::Nil));
        // Pin source algorithm results rather than applying an independently
        // invented percentage-unit or Rust rounding convention.
        for (number, magnitude, corrupted, expected) in [
            ("+10.5", 1.2, 1.0, 13.0),
            ("+11", 1.2, 1.0, 13.0),
            ("+11", 0.5, 1.0, 5.0),
            ("-11", 0.5, 1.0, -5.0),
            ("+11", 1.0, 0.5, 6.0),
            ("-11", 1.0, 0.5, -5.0),
            ("+10.5", 1.2, 0.5, 7.0),
        ] {
            let formatted: String = formatter
                .call((format!("{number} to {family}"), 1, magnitude, corrupted))
                .unwrap();
            let (mods, extra): (Table, Option<String>) = source.parser().call(&*formatted).unwrap();
            assert!(extra.is_none(), "complete formatted source: {formatted}");
            assert_modifiers(&mods, names, expected);
        }
    }
}

#[test]
fn actual_attribute_tags_and_persistent_advanced_controls_drive_adaptive_catalyst() {
    let source = Source::new();
    for (family, names) in FAMILIES {
        let text = format!("+11 to {family}");
        for (headers, tag, expected, scalar) in [
            ("", "attribute", 11.0, 1.0),
            ("Catalyst: Adaptive\nCatalystQuality: 20", "", 11.0, 1.0),
            ("Catalyst: Adaptive\nCatalystQuality: 20", "life", 11.0, 1.0),
            (
                "Catalyst: Adaptive\nCatalystQuality: 20",
                "attribute",
                13.0,
                1.2,
            ),
            ("Quality (Attribute Modifiers): 20%", "attribute", 13.0, 1.2),
            (
                "Catalyst: Flesh\nCatalystQuality: 20",
                "attribute",
                11.0,
                1.0,
            ),
            (
                "Catalyst: Adaptive\nCatalystQuality: -50",
                "attribute",
                5.0,
                0.5,
            ),
        ] {
            let tags = if tag.is_empty() {
                String::new()
            } else {
                format!("{{tags:{tag}}}")
            };
            let item = source.fresh(
                "Iron Ring",
                headers,
                &format!("{tags}{text}\nwhile stationary"),
            );
            assert_eq!(matching(&item, &text).len(), 1);
            source.assert_initial(&text, names, expected, scalar);
            source.assert_no_combination();
        }
        // Source helper defaults absent catalyst quality to 20; item headers
        // may independently establish an explicit quality before reaching it.
        let policy: Table = source.oracle.lua.globals().get("itemPolicy").unwrap();
        assert_eq!(
            policy
                .get::<Table>("catalysts")
                .unwrap()
                .raw_get::<String>(12)
                .unwrap(),
            "Adaptive"
        );
        let mod_line = source.oracle.lua.create_table().unwrap();
        mod_line
            .set(
                "modTags",
                source
                    .oracle
                    .lua
                    .create_sequence_from(["attribute"])
                    .unwrap(),
            )
            .unwrap();
        let scalar: f64 = policy
            .get::<Function>("scalar")
            .unwrap()
            .call((12, mod_line, Value::Nil))
            .unwrap();
        assert_eq!(scalar, 1.2);
        let next = format!("+8 to {family}");
        let item = source.fresh(
            "Iron Ring",
            "Catalyst: Adaptive\nCatalystQuality: 20",
            &format!("[{{ Modifier - Attribute }}]\n{text}\n{next}"),
        );
        assert_eq!(rows(&item, "explicitModLines").len(), 2);
        for member in rows(&item, "explicitModLines") {
            let tags: Table = member.get("modTags").unwrap();
            assert_eq!(tags.raw_len(), 1);
            assert_eq!(tags.raw_get::<String>(1).unwrap(), "attribute");
        }
        source.assert_initial(&text, names, 13.0, 1.2);
        source.assert_initial(&next, names, 9.0, 1.2);
        source.assert_no_combination();
        source.fresh(
            "Iron Ring",
            "Catalyst: Adaptive\nCatalystQuality: -200",
            &format!("[{{ Modifier - Attribute }}]\n{text}\n{next}"),
        );
        assert_eq!(source.formats(&text)[0].get::<f64>("scalar").unwrap(), -1.0);
        assert!(
            rows(&source.trace, "calls")
                .iter()
                .any(|row| row.get::<bool>("combined").unwrap())
        );
    }
}

#[test]
fn source_occurrences_and_base_implicits_are_not_deduplicated_by_attribute_name() {
    let source = Source::new();
    for ((family, names), (base, amount)) in FAMILIES.into_iter().zip([
        ("Amber Amulet", 10),
        ("Jade Amulet", 10),
        ("Lapis Amulet", 10),
        ("Stellar Amulet", 6),
    ]) {
        let text = format!("+{amount} to {family}");
        let raw = format!(
            "Rarity: RARE\nAttribute Probe\n{base}\nItem Level: 80\nImplicits: 1\n{text}\n{text}"
        );
        let item = source.raw(&raw);
        let members = matching(&item, &text);
        assert_eq!(members.len(), 2, "two actual supplied occurrences: {base}");
        assert_eq!(members[0].0, "implicitModLines");
        assert_eq!(members[1].0, "explicitModLines");
        for (_, member) in members {
            assert_modifiers(
                &member.get::<Table>("modList").unwrap(),
                names,
                f64::from(amount),
            );
        }
        assert_eq!(source.formats(&text).len(), 2);
        source.assert_no_combination();
        let expected: Vec<_> = names
            .iter()
            .chain(names.iter())
            .map(|name| ((*name).to_owned(), f64::from(amount)))
            .collect();
        assert_eq!(attribute_records(&item), expected);
        // The base's implicit definition does not supply a third copy, or any
        // copy when the raw item has no attribute member at all.
        let empty = source.fresh(base, "", "+69 to maximum Life");
        assert!(attribute_records(&empty).is_empty());
    }
    let item = source.fresh("Iron Ring", "", "+11 to all Attributes\n+7 to Intelligence");
    assert_eq!(
        attribute_records(&item),
        [
            ("Str".into(), 11.0),
            ("Dex".into(), 11.0),
            ("Int".into(), 11.0),
            ("All".into(), 11.0),
            ("Int".into(), 7.0),
        ]
    );
}

#[test]
fn complete_predecessors_and_parser_shape_contrasts_preserve_member_boundaries() {
    let source = Source::new();
    for (family, names) in FAMILIES {
        let text = format!("+8 to {family}");
        for previous in [
            "+69 to maximum Life",
            "+40% to Cold Resistance",
            "+11 to all Attributes",
        ] {
            let item = source.fresh(
                "Iron Ring",
                "",
                &format!("{previous}\n{text}\nwhile stationary"),
            );
            assert_eq!(matching(&item, &text).len(), 1);
            source.assert_initial(&text, names, 8.0, 1.0);
            source.assert_no_combination();
        }
        source.fresh("Iron Ring", "", &format!("unreviewed predecessor\n{text}"));
        assert!(rows(&source.trace, "calls").iter().any(|row| {
            row.get::<bool>("combined").unwrap()
                && row.get::<String>("text").unwrap().contains(&text)
        }));
        let text = format!("-0 to {family}");
        let item = source.fresh("Iron Ring", "", &format!("{text}\nwhile stationary"));
        let combined = matching(&item, &format!("{text}\nwhile stationary"));
        assert_eq!(combined.len(), 1);
        let modifiers: Table = combined[0].1.get("modList").unwrap();
        assert_eq!(modifiers.raw_len(), names.len());
        for (index, name) in names.iter().enumerate() {
            let modifier: Table = modifiers.raw_get(index + 1).unwrap();
            assert_eq!(modifier.get::<String>("name").unwrap(), *name);
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
    for (suffix, names) in [
        ("Attributes", &["Str", "Dex", "Int", "All"][..]),
        ("Strength and Dexterity", &["Str", "Dex", "StrDex"][..]),
        ("Strength and Intelligence", &["Str", "Int", "StrInt"][..]),
        ("Dexterity and Intelligence", &["Dex", "Int", "DexInt"][..]),
    ] {
        let (mods, extra): (Table, Option<String>) =
            source.parser().call(format!("+10 to {suffix}")).unwrap();
        assert!(extra.is_none());
        assert_modifiers(&mods, names, 10.0);
    }
    let (mods, extra): (Table, Option<String>) = source
        .parser()
        .call("20% reduced Attribute Requirements")
        .unwrap();
    assert!(extra.is_none());
    assert_eq!(mods.raw_len(), 3);
    for (index, name) in ["StrRequirement", "DexRequirement", "IntRequirement"]
        .iter()
        .enumerate()
    {
        let modifier: Table = mods.raw_get(index + 1).unwrap();
        assert_eq!(modifier.get::<String>("name").unwrap(), *name);
        assert_eq!(modifier.get::<String>("type").unwrap(), "INC");
        assert_eq!(modifier.get::<f64>("value").unwrap(), -20.0);
    }
}

#[test]
fn all_twenty_original_attribute_rows_have_exact_initial_source_shapes() {
    let source = Source::new();
    let mut counts = [0usize; 4];
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
                    let text = line.trim();
                    FAMILIES
                        .iter()
                        .enumerate()
                        .find_map(|(family, (suffix, names))| {
                            let amount = text
                                .strip_prefix('+')?
                                .strip_suffix(&format!(" to {suffix}"))?;
                            if amount.is_empty()
                                || !amount.bytes().all(|byte| byte.is_ascii_digit())
                            {
                                return None;
                            }
                            Some((family, *names, text, amount.parse::<f64>().unwrap()))
                        })
                })
                .collect();
            if candidates.is_empty() {
                continue;
            }
            let item = source.raw(raw);
            for (family, names, text, amount) in candidates {
                counts[family] += 1;
                original_counts[original - 1] += 1;
                let members = matching(&item, text);
                assert_eq!(
                    members.len(),
                    1,
                    "original {original}/item {:?}: {text}",
                    node.attribute("id")
                );
                assert_eq!(members[0].1.get::<Table>("modTags").unwrap().raw_len(), 0);
                source.assert_initial(text, names, amount, 1.0);
            }
        }
    }
    assert_eq!(counts, [5, 2, 9, 4]);
    assert_eq!(original_counts, [4, 5, 3, 2, 6]);
    println!(
        "original attribute source rows: {}",
        json!({
            "strength":counts[0],"dexterity":counts[1],"intelligence":counts[2],"all_attributes":counts[3],
            "by_original":original_counts,"scope":"fresh raw text only; overlays, predecessor guards and native numerical coverage remain separate"
        })
    );
}
