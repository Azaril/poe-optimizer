//! The independent original dispatcher authenticates all extracted formatting.
use super::*;
use poe_optimizer_data::game_data::ItemNumberFormat;
#[test]
fn fresh_item_formatting_matches_original_metadata_and_literal_dispatch_cold_and_warm() {
    let extracted = poe_optimizer_pob::game_data::extract_pinned_game_data_for_review(
        &repository().join("vendor/path-of-building-poe2"),
    )
    .unwrap();
    let rules = &extracted.package.item_formatting.rules;
    assert_eq!(rules.len(), 83);
    let mut cases = 0;
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        let lua = &oracle.lua;
        let original: Table = lua
            .globals()
            .get::<Table>("data")
            .unwrap()
            .get("modScalability")
            .unwrap();
        let observe:Function=lua.load("return function(key,value) local originalData=data.modScalability;local originalFormat=itemLib.formatValue;local formats={};data.modScalability={[key]=assert(originalData[key])};itemLib.formatValue=function(v,b,s,p,d,t) formats[#formats+1]={precision=p,display_precision=d,trim_trailing_zeroes=t==true};return originalFormat(v,b,s,p,d,t)end;local line=key:gsub('#',tostring(value));local output=itemLib.applyRange(line,1,1);data.modScalability=originalData;itemLib.formatValue=originalFormat;return formats,output end").eval().unwrap();
        for rule in rules {
            assert!(original.contains_key(rule.template.as_str()).unwrap());
            for value in [0.0, 17.49, 17.5, 17.51] {
                let (formats, _): (Table, String) =
                    observe.call((rule.template.as_str(), value)).unwrap();
                let actual: Vec<ItemNumberFormat> =
                    lua.from_value(mlua::Value::Table(formats)).unwrap();
                assert_eq!(actual, rule.captures, "{} warm{warm}", rule.template);
                cases += 1;
            }
        }
        let apply: Function = lua
            .globals()
            .get::<Table>("itemLib")
            .unwrap()
            .get("applyRange")
            .unwrap();
        for (input, expected) in [
            ("+17.5 to Evasion Rating", "+18 to Evasion Rating"),
            ("+17.5 to evasion rating", "+17.5 to evasion rating"),
            ("+17.5 to Armour", "+18 to Armour"),
            ("-17.5 to Armour", "-18 to Armour"),
            ("+17.5 to Global Armour", "+17.5 to Global Armour"),
            ("+17.5 to Strength", "+18 to Strength"),
            ("+17.5 to maximum Life", "+18 to maximum Life"),
            ("+17.5% to Fire Resistance", "+18% to Fire Resistance"),
            ("+17.5% to All Resistances", "+18% to All Resistances"),
            ("+17.5% to all Resistances", "+17.5% to all Resistances"),
            ("+17.5 to Energy Shield", "+17.5 to Energy Shield"),
            (
                "+17.5 to maximum Energy Shield",
                "+18 to maximum Energy Shield",
            ),
            (
                "+17.5 to Evasion Rating and Energy Shield",
                "+18 to Evasion Rating and Energy Shield",
            ),
            (
                "+17.5 to Armour and Energy Shield",
                "+17.5 to Armour and Energy Shield",
            ),
        ] {
            assert_eq!(
                apply.call::<String>((input, 1, 1)).unwrap(),
                expected,
                "warm{warm}"
            );
        }
    }
    assert_eq!(cases, 664);
}
