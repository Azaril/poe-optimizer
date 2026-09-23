//! Authenticated source evidence for migrating legacy unconditional member roles.
//! Fresh ParseRaw includes its original nested BuildModList; observers distinguish
//! initial grouping from later assembly. No UI replay or native-value authority.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;

use mlua::{Function, Table, Value};
use serde_json::json;
use std::collections::BTreeSet;

const LISTS: [&str; 6] = [
    "classRequirementModLines",
    "buffModLines",
    "enchantModLines",
    "runeModLines",
    "implicitModLines",
    "explicitModLines",
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
            local parser,formatter=modLib.parseMod,itemLib.applyRange
            local class=common.classes.Item
            local build=class.BuildModList
            function class:BuildModList(...)
                local before=trace.assembly;trace.assembly=true
                local result=build(self,...)
                trace.assembly=before;return result
            end
            function modLib.parseMod(text,combined,...)
                assert(#trace.calls<256 and #text<=8192,'role parser observer bound')
                local mods,extra=parser(text,combined,...)
                trace.calls[#trace.calls+1]={text=text,combined=combined==true,
                    assembly=trace.assembly,has_modifiers=mods~=nil,modifier_count=mods and #mods or 0,extra=extra}
                return mods,extra
            end
            function itemLib.applyRange(text,range,scalar,corrupted,...)
                assert(#trace.formats<256 and #text<=8192,'role formatter observer bound')
                local out=formatter(text,range,scalar,corrupted,...)
                trace.formats[#trace.formats+1]={text=text,range=range,scalar=scalar,
                    corrupted=corrupted,output=out,assembly=trace.assembly}
                return out
            end
            return trace
        "#,
            )
            .set_name("@source-role-test-phase-observer")
            .eval()
            .unwrap();
        Self { oracle, trace }
    }
    fn fresh(&self, base: &str, headers: &str, raw: &str) -> Table {
        assert!(base.len() <= 512 && headers.len() <= 2048 && raw.len() <= 4096);
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
        let text = format!(
            "Rarity: RARE\nRole Migration Probe\n{base}\nItem Level: 80\nQuality: 0\n{headers}\nImplicits: 0\n{raw}"
        );
        item.get::<Function>("ParseRaw")
            .unwrap()
            .call::<()>((item.clone(), text, Value::Nil, false))
            .unwrap_or_else(|error| panic!("source ParseRaw {base}/{raw}: {error}"));
        assert_eq!(item.get::<String>("baseName").unwrap(), base);
        item
    }
    fn assert_single(&self, item: &Table, semantic: &str) {
        let members: Vec<_> = LISTS
            .into_iter()
            .flat_map(|list| rows(item, list))
            .filter(|row| row.get::<String>("line").unwrap() == semantic)
            .collect();
        assert_eq!(members.len(), 1, "exact source member: {semantic}");
        assert!(
            members[0].get::<Option<String>>("extra").unwrap().is_none(),
            "complete source member: {semantic}"
        );
        // Item.lua2688-2690 and2746-2748 may replace the final cache with an
        // empty range result. Initial source recognition is asserted below.
        assert!(
            !rows(&self.trace, "calls")
                .iter()
                .any(|call| call.get::<bool>("combined").unwrap()),
            "combined retry: {semantic}"
        );
        let formats: Vec<_> = rows(&self.trace, "formats")
            .into_iter()
            .filter(|row| {
                !row.get::<bool>("assembly").unwrap()
                    && row.get::<String>("text").unwrap() == semantic
            })
            .collect();
        assert_eq!(formats.len(), 1, "initial source format: {semantic}");
        assert_eq!(
            formats[0].get::<f64>("scalar").unwrap(),
            1.0,
            "empty tags scalar independent of catalyst headers"
        );
        let text: String = formats[0].get("output").unwrap();
        assert!(
            rows(&self.trace, "calls")
                .iter()
                .any(|call| !call.get::<bool>("assembly").unwrap()
                    && call.get::<String>("text").unwrap() == text
                    && call.get::<bool>("has_modifiers").unwrap()
                    && call.get::<usize>("modifier_count").unwrap() > 0
                    && call.get::<Option<String>>("extra").unwrap().is_none()),
            "fully parsed initial format: {semantic}"
        );
    }
    fn observation(&self) -> serde_json::Value {
        let calls:Vec<_>=rows(&self.trace,"calls").into_iter().filter(|row|!row.get::<bool>("assembly").unwrap()).map(|row|json!({"text":row.get::<String>("text").unwrap(),"combined":row.get::<bool>("combined").unwrap(),"extra":row.get::<Option<String>>("extra").unwrap()})).collect();
        let formats:Vec<_>=rows(&self.trace,"formats").into_iter().filter(|row|!row.get::<bool>("assembly").unwrap()).map(|row|json!({"text":row.get::<String>("text").unwrap(),"scalar":row.get::<Option<f64>>("scalar").unwrap(),"output":row.get::<String>("output").unwrap()})).collect();
        json!({"calls":calls,"formats":formats})
    }
}
fn rows(table: &Table, field: &str) -> Vec<Table> {
    let rows: Table = table.get(field).unwrap();
    assert!(rows.raw_len() <= 256);
    rows.sequence_values().map(Result::unwrap).collect()
}

fn legacy_cases() -> Vec<(String, String)> {
    let mut cases = vec![
        (
            "ranged-plus-cold".into(),
            "+(1-1000000)% to Cold Resistance".into(),
        ),
        (
            "ranged-bare-cold".into(),
            "(-1000000--1)% to Cold Resistance".into(),
        ),
        (
            "fixed-elemental".into(),
            "+1000000% to all Elemental Resistances".into(),
        ),
        (
            "ranged-plus-elemental".into(),
            "+(1-1000000)% to all Elemental Resistances".into(),
        ),
        (
            "ranged-bare-elemental".into(),
            "(-1000000--1)% to all Elemental Resistances".into(),
        ),
        ("fixed-life".into(), "+1000000 to maximum Life".into()),
        (
            "local-critical-flat-increase-fixed".into(),
            "+1.2345% to Critical Hit Chance".into(),
        ),
    ];
    for (family, suffix) in [
        ("physical", "Physical Damage"),
        ("attack-speed", "Attack Speed"),
        ("critical", "Critical Hit Chance"),
    ] {
        for (id, word) in [("increase", "increased"), ("reduced", "reduced")] {
            for (form, number) in [("fixed", "1000000"), ("ranged", "(0-1000000)")] {
                cases.push((
                    format!("local-{family}-increase-{id}-{form}"),
                    format!("{number}% {word} {suffix}"),
                ));
            }
        }
    }
    for damage in ["Physical", "Cold", "Fire", "Lightning", "Chaos"] {
        for (form, numbers) in [
            ("fixed", "0 to 1000000"),
            ("ranged", "(0-1) to (999999-1000000)"),
        ] {
            cases.push((
                format!("local-{}-flat-increase-{form}", damage.to_lowercase()),
                format!("Adds {numbers} {damage} Damage"),
            ));
        }
    }
    assert_eq!(cases.len(), 29);
    assert_eq!(
        cases
            .iter()
            .map(|(id, _)| id)
            .collect::<BTreeSet<_>>()
            .len(),
        29
    );
    cases
}

#[test]
fn every_legacy_role_has_explicit_bounded_source_evidence_not_unconditional_authority() {
    let source = Source::new();
    let cases = legacy_cases();
    let controls = [
        "",
        "{range:0}",
        "{range:0.5}",
        "{range:1}",
        "{implicit}",
        "{enchant}",
        "{fractured}",
        "{desecrated}",
    ];
    let contexts = [
        "",
        "Catalyst: Tul's\nCatalystQuality: -200",
        "Catalyst: Reaver\nCatalystQuality: 1000000000000000000000000000000",
    ];
    let mut count = 0;
    for (id, semantic) in &cases {
        for (index, control) in controls.iter().enumerate() {
            let item = source.fresh(
                "Crude Bow",
                contexts[index % contexts.len()],
                &format!("{control}{semantic}\nwhile stationary"),
            );
            source.assert_single(&item, semantic);
            let member = LISTS
                .into_iter()
                .flat_map(|list| rows(&item, list))
                .find(|row| row.get::<String>("line").unwrap() == *semantic)
                .unwrap();
            assert_eq!(member.get::<Table>("modTags").unwrap().raw_len(), 0);
            if id == "local-physical-increase-increase-ranged" && *control == "{range:0}" {
                assert!(
                    rows(&member, "modList").is_empty(),
                    "zero range final cache is separate from initial full parse"
                );
                println!(
                    "zero selected range: initial upper-endpoint source parse complete, final cache empty"
                );
            }
            count += 1;
        }
        println!("reviewed legacy role source shape: {id}: {semantic}");
    }
    // Explicit '+' survives unit precision rounding, including to zero. Critical
    // flat values exercise their actual source precision100 path. The predicate
    // is digit-led and finite; it does not promise native final-value eligibility.
    for number in [
        "+0",
        "+0.0",
        "+0.000001",
        "+0.0049",
        "+0.005",
        "+1.2345",
        "+000001.50",
        "+1000000.0",
    ] {
        let semantic = format!("{number}% to Critical Hit Chance");
        let item = source.fresh(
            "Crude Bow",
            contexts[1],
            &format!("{{fractured}}{{range:0.5}}{semantic}\nwhile stationary"),
        );
        source.assert_single(&item, &semantic);
        count += 1;
    }
    println!(
        "legacy source migration positives: {count}; 29 roles x8 flag contexts plus8 decimal-critical cases; bare negative ranges remain native-unresolved until separately enabled"
    );
}

#[test]
fn actual_admitted_and_predecessor_spellings_have_explicit_context_boundaries() {
    let source = Source::new();
    let chains = [
        (
            "Frayed Shoes",
            "+21 to maximum Life",
            "+10% to Cold Resistance",
        ),
        ("Iron Ring", "+15 to maximum Life", "+8% to Cold Resistance"),
        (
            "Forked Spear",
            "28% increased Physical Damage",
            "Adds 11 to 19 Physical Damage",
        ),
        (
            "Grand Spear",
            "101% increased Physical Damage",
            "Adds 49 to 74 Cold Damage",
        ),
        (
            "Grand Spear",
            "Adds 49 to 74 Cold Damage",
            "Adds 6 to 179 Lightning Damage",
        ),
        (
            "Sinister Quarterstaff",
            "{fractured}167% increased Physical Damage",
            "Adds 13 to 298 Lightning Damage",
        ),
        (
            "Gold Ring",
            "+11% to all Elemental Resistances",
            "+34% to Cold Resistance",
        ),
        (
            "Cannonade Crossbow",
            "22% increased Physical Damage",
            "Adds 5 to 13 Cold Damage",
        ),
    ];
    for (base, previous, current) in chains {
        let raw = format!("{previous}\n{current}\nwhile stationary");
        let item = source.fresh(base, "", &raw);
        source.assert_single(
            &item,
            previous.strip_prefix("{fractured}").unwrap_or(previous),
        );
        source.assert_single(&item, current);
    }
    let ranged = "+(20-30)% to Cold Resistance";
    let tagged = format!(
        "{{tags:cold_resistance,elemental_resistance,elemental,cold,resistance}}{{range:0.5}}{ranged}"
    );
    let item = source.fresh(
        "Sapphire Ring",
        "Crafted: true\nPrefix: {range:0}IncreasedLife1\nPrefix: None\nSuffix: None",
        &format!("{tagged}\n+10 to maximum Life\nwhile stationary"),
    );
    source.assert_single(&item, ranged);
    source.assert_single(&item, "+10 to maximum Life");
    let tagged_member = LISTS
        .into_iter()
        .flat_map(|list| rows(&item, list))
        .find(|row| row.get::<String>("line").unwrap() == ranged)
        .unwrap();
    assert_eq!(tagged_member.get::<Table>("modTags").unwrap().raw_len(), 5);
    println!(
        "actual tagged range without catalyst declarations: {}",
        source.observation()
    );
    // This real spelling has nonempty tags; empty-modTags predicates cannot
    // establish its scalar. Absent/None catalyst requires a separate proof.
    source.fresh(
        "Sapphire Ring",
        "Catalyst: Tul's\nCatalystQuality: -200",
        &format!("{tagged}\n+10 to maximum Life"),
    );
    assert!(
        rows(&source.trace, "calls")
            .iter()
            .any(|row| row.get::<bool>("combined").unwrap())
    );
    println!(
        "actual tagged range negative catalyst contrast: {}",
        source.observation()
    );
}

#[test]
fn malformed_signs_zero_ranges_extreme_endpoints_and_scaling_are_not_single_member_proofs() {
    let source = Source::new();
    let mut bad = vec![
        "10 to maximum Life".to_owned(),
        "5% to all Elemental Resistances".into(),
        "1% to Critical Hit Chance".into(),
        "-0.000001% to Critical Hit Chance".into(),
        "+(0-0)% to Cold Resistance".into(),
        "(20-30)% to Cold Resistance".into(),
        "+(0-0)% to all Elemental Resistances".into(),
        "(20-30)% to all Elemental Resistances".into(),
        "+(1000000000000000-1000000000000000)% to Cold Resistance".into(),
    ];
    for suffix in ["Physical Damage", "Attack Speed", "Critical Hit Chance"] {
        for word in ["increased", "reduced"] {
            bad.push(format!("+17% {word} {suffix}"));
            bad.push(format!("1000000000000000% {word} {suffix}"));
        }
    }
    for damage in ["Physical", "Cold", "Fire", "Lightning", "Chaos"] {
        bad.push(format!("Adds +1 to 2 {damage} Damage"));
        bad.push(format!("Adds (1000000000000000-1000000000000000) to (1000000000000000-1000000000000000) {damage} Damage"));
    }
    for raw in &bad {
        source.fresh("Crude Bow", "", &format!("{raw}\nwhile stationary"));
        assert!(
            rows(&source.trace, "calls")
                .iter()
                .any(|row| !row.get::<bool>("assembly").unwrap()
                    && row.get::<bool>("combined").unwrap()),
            "expected authentic combined retry: {raw}; {}",
            source.observation()
        );
    }
    for (catalyst, tag, semantic) in [
        ("Tul's", "cold", "+(20-30)% to Cold Resistance"),
        ("Flesh", "life", "+10 to maximum Life"),
        ("Xoph's", "fire", "+10% to all Elemental Resistances"),
        ("Skittering", "speed", "17% increased Attack Speed"),
        ("Reaver", "attack", "Adds 1 to 2 Physical Damage"),
    ] {
        source.fresh(
            "Crude Bow",
            &format!("Catalyst: {catalyst}\nCatalystQuality: -200"),
            &format!("{{tags:{tag}}}{semantic}\n+8% to Cold Resistance"),
        );
        assert!(
            rows(&source.trace, "calls")
                .iter()
                .any(|row| row.get::<bool>("combined").unwrap()),
            "scaling counterexample: {semantic}"
        );
    }
    println!(
        "legacy source migration negative cases: {} malformed/extreme plus5 tagged-scaling",
        bad.len()
    );
}

#[test]
fn initial_scaling_absence_is_fresh_single_text_and_normalized_prefix_scoped() {
    let source = Source::new();
    let semantic = "+(20-30)% to Cold Resistance";
    let raw = format!("{{tags:cold}}{{range:0.5}}{semantic}\n+10 to maximum Life");
    let item = source.fresh("Sapphire Ring", "", &raw);
    source.assert_single(&item, semantic);
    assert!(item.get::<Option<f64>>("catalyst").unwrap().is_none());
    for (headers, expected_scalar, retry) in [
        ("Quality (Cold Modifiers): 20%", 1.2, false),
        (
            "Quality (Cold Modifiers): 20%\nCatalystQuality: -200",
            -1.0,
            true,
        ),
        ("[Catalyst]: Tul's\nCatalystQuality: -200", -1.0, true),
        (
            "[ignored|Catalyst]: Tul's\nCatalystQuality: -200",
            -1.0,
            true,
        ),
        (
            "<keyword>{Catalyst}: Tul's\nCatalystQuality: -200",
            -1.0,
            true,
        ),
    ] {
        let item = source.fresh("Sapphire Ring", headers, &raw);
        assert_eq!(item.get::<f64>("catalyst").unwrap(), 6.0);
        let formats: Vec<_> = rows(&source.trace, "formats")
            .into_iter()
            .filter(|row| {
                !row.get::<bool>("assembly").unwrap()
                    && row.get::<String>("text").unwrap() == semantic
            })
            .collect();
        assert_eq!(formats.len(), 1);
        assert_eq!(formats[0].get::<f64>("scalar").unwrap(), expected_scalar);
        assert_eq!(
            rows(&source.trace, "calls")
                .iter()
                .any(|row| row.get::<bool>("combined").unwrap()),
            retry
        );
        println!(
            "source catalyst header/presentation contrast: {}",
            json!({"header":headers,"scalar":expected_scalar,"retry":retry})
        );
    }
    // ParseRaw itself does not clear an earlier catalyst. The dialect's fresh
    // single-text contract is therefore necessary, not merely an optimization.
    let item = source.fresh(
        "Sapphire Ring",
        "Catalyst: Tul's\nCatalystQuality: -200",
        &raw,
    );
    source
        .trace
        .set("calls", source.oracle.lua.create_table().unwrap())
        .unwrap();
    source
        .trace
        .set("formats", source.oracle.lua.create_table().unwrap())
        .unwrap();
    let text = format!(
        "Rarity: RARE\nReused Source Probe\nSapphire Ring\nQuality: 0\nImplicits: 0\n{raw}"
    );
    item.get::<Function>("ParseRaw")
        .unwrap()
        .call::<()>((item.clone(), text, Value::Nil, false))
        .unwrap();
    assert_eq!(item.get::<f64>("catalyst").unwrap(), 6.0);
    assert_eq!(item.get::<f64>("catalystQuality").unwrap(), -200.0);
    assert!(
        rows(&source.trace, "calls")
            .iter()
            .any(|row| row.get::<bool>("combined").unwrap())
    );
    println!("reused ParseRaw object retains catalyst; fresh single-text premise required");
}

#[test]
fn escaped_advanced_header_changes_following_lines_despite_their_empty_raw_tags() {
    let source = Source::new();
    let item = source.fresh(
        "Crude Bow",
        "Catalyst: Tul's\nCatalystQuality: -200",
        "[{ Modifier - Cold }]\n+10% to Cold Resistance\n+8% to Cold Resistance",
    );
    for raw in ["+10% to Cold Resistance", "+8% to Cold Resistance"] {
        let members: Vec<_> = LISTS
            .into_iter()
            .flat_map(|list| rows(&item, list))
            .filter(|row| row.get::<String>("line").unwrap() == raw)
            .collect();
        assert_eq!(members.len(), 1);
        assert_eq!(
            members[0]
                .get::<Table>("modTags")
                .unwrap()
                .raw_get::<String>(1)
                .unwrap(),
            "cold"
        );
        assert!(members[0].get::<Option<String>>("extra").unwrap().is_some());
        let formats: Vec<_> = rows(&source.trace, "formats")
            .into_iter()
            .filter(|row| {
                !row.get::<bool>("assembly").unwrap() && row.get::<String>("text").unwrap() == raw
            })
            .collect();
        assert_eq!(formats.len(), 1);
        assert_eq!(formats[0].get::<f64>("scalar").unwrap(), -1.0);
        assert!(
            formats[0]
                .get::<String>("output")
                .unwrap()
                .starts_with("+-")
        );
    }
    assert!(
        rows(&source.trace, "calls")
            .iter()
            .any(|row| row.get::<bool>("combined").unwrap())
    );
    println!(
        "escaped advanced-copy header poisons both tagless followers: {}",
        source.observation()
    );
}
