//! Optional, authenticated complete Item methods establish a finite quality-input
//! scope. This does not establish native local defence assembly, effective quality
//! after source mutation, equipped activation, or complete-build parity.
#![cfg(not(target_arch = "wasm32"))]

use std::collections::{BTreeMap, BTreeSet};

use mlua::{Function, Table};
use poe_optimizer_import::owned_defence_profiles::{DefenceProfileCatalog, DefenceProfilePresence};
use sha2::{Digest, Sha256};

#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;

fn oracle(jit_enabled: bool) -> runtime::Oracle {
    let oracle = runtime::Oracle::new();
    // Reload the entire authenticated module to inspect the constructor before
    // Common wraps it. All Item methods below remain whole original functions.
    oracle
        .lua
        .load(runtime::verified("src/Classes/Item.lua").unwrap())
        .set_name("@src/Classes/Item.lua")
        .exec()
        .unwrap();
    let constructor: Function = oracle
        .lua
        .load("return common.classes.Item.Item")
        .eval()
        .unwrap();
    assert_eq!(
        constructor.info().source.as_deref(),
        Some("@src/Classes/Item.lua")
    );
    assert_eq!(constructor.info().line_defined, Some(93));
    oracle
        .lua
        .globals()
        .set("qualityJitEnabled", jit_enabled)
        .unwrap();
    oracle.lua.load(r#"
function qualityRaw(base,headers,body)
 return "Rarity: RARE\nQuality Input Probe\n"..base.."\nItem Level: 80\n"..headers.."\nImplicits: 0\n"..(body or "")
end
function qualityItem(base,headers,body,highQuality)
 return new("Item"):Item(qualityRaw(base,headers,body),nil,highQuality or false)
end
qualityProbe=new("Item"):Item()
if qualityJitEnabled then jit.on() else jit.off();jit.flush() end
"#).set_name("@owned-quality-complete-method-inputs").exec().unwrap();
    let probe: Table = oracle.lua.globals().get("qualityProbe").unwrap();
    let constructor: Function = probe.get("Item").unwrap();
    assert_eq!(
        constructor.info().source.as_deref(),
        Some("@src/Modules/Common.lua")
    );
    assert_eq!(constructor.info().line_defined, Some(167));
    for (name, line) in [
        ("ParseRaw", 468),
        ("NormaliseQuality", 1805),
        ("BuildModListForSlotNum", 2414),
        ("BuildModList", 2694),
    ] {
        let method: Function = probe.get(name).unwrap();
        assert_eq!(
            method.info().source.as_deref(),
            Some("@src/Classes/Item.lua")
        );
        assert_eq!(method.info().line_defined, Some(line));
    }
    oracle
}

fn profiles() -> DefenceProfileCatalog {
    serde_json::from_str(include_str!(
        "../../../data/owned/poe2/3887ae68/defence-profiles/catalog.json"
    ))
    .unwrap()
}

#[test]
fn all_1240_present_profiles_have_independently_verified_standard_quality_inputs() {
    let catalog = profiles();
    assert_eq!(catalog.profiles.len(), 1756);
    for jit_enabled in [false, true] {
        let oracle = oracle(jit_enabled);
        let scope = oracle.lua.create_table().unwrap();
        for row in &catalog.profiles {
            scope
                .set(
                    row.base.as_str(),
                    matches!(row.profile, DefenceProfilePresence::Table { .. }),
                )
                .unwrap();
        }
        oracle
            .lua
            .globals()
            .set("qualityProfileScope", scope)
            .unwrap();
        oracle.lua.load(r#"
local total,present,empty=0,0,0
local types={}
for name,base in pairs(data.itemBases) do
 total=total+1
 local expected=qualityProfileScope[name]
 assert(type(expected)=="boolean",name.." missing catalog row")
 assert(expected==(type(base.armour)=="table"),name.." profile presence")
 if expected then
  present=present+1
  -- Verify the actual branch prerequisites, not a name/type heuristic or an
  -- assumption that every present armour table wins the source branch.
  assert(base.weapon==nil and base.quality==20,name.." quality/branch scope")
  types[base.type]=(types[base.type] or 0)+1
  if next(base.armour)==nil then empty=empty+1 end
  for _,quality in ipairs({0,20}) do
   local item=qualityItem(name,"Quality: "..quality)
   assert(item.base==base and item.baseName==name,name.." selected base")
   assert(item.quality==quality and item.craftedQuality==0,name.." explicit input")
   assert(type(item.armourData)=="table" and item.weaponData==nil,name.." assembly branch")
   item:BuildModList()
   assert(item.quality==quality and item.craftedQuality==0,name.." unchanged repeat")
  end
 end
end
assert(total==1756 and present==1240 and empty==6)
local expected={ ["Body Armour"]=347,Boots=191,Focus=51,Gloves=201,Helmet=257,Shield=193 }
for name,count in pairs(expected) do assert(types[name]==count,name.." type count");types[name]=nil end
assert(next(types)==nil)
-- Exclusion is task scope, not an assertion that absent profiles forbid quality.
assert(qualityProfileScope["Iron Ring"]==false)
assert(qualityProfileScope["Ashen Staff"]==false)
assert(qualityItem("Ashen Staff","Quality: 20").quality==20)
"#).set_name("@owned-quality-full-catalog-membership").exec().unwrap();
    }
}

#[test]
fn explicit_zero_missing_duplicate_and_tolerated_headers_remain_distinct() {
    for jit_enabled in [false, true] {
        oracle(jit_enabled)
            .lua
            .load(
                r#"
assert(main.defaultItemQuality==20)
for _,quality in ipairs({0,19,20,1000000}) do
 for _,highQuality in ipairs({false,true}) do
  local item=qualityItem("Rusted Cuirass","Quality: "..quality,nil,highQuality)
  assert(item.quality==quality)
 end
end
-- These source tolerance cases do not broaden the owned unsigned-integer grammar,
-- duplicate policy, numeric bound, or absence authority.
for _,case in ipairs({
 {"",0}, {"Quality: malformed",0}, {"Quality: ",0},
 {"Quality: 20\nQuality: 0",0}, {"Quality: 0\nQuality: 20",20},
 {"Quality: 20\nQuality: malformed",0},
 {"Quality: +20",20}, {"Quality: -5",-5}, {"Quality: .5",0.5},
 {"Quality: 20% (augmented)",20}, {"Quality: 1000001",1000001},
}) do
 local item=qualityItem("Rusted Cuirass",case[1])
 assert(item.quality==case[2],case[1].." observed "..tostring(item.quality))
end
assert(qualityItem("Rusted Cuirass","",nil,true).quality==20)
assert(qualityItem("Rusted Cuirass","Quality: 0",nil,true).quality==0)
-- Calling the complete normalization method explicitly is a different lifecycle
-- input from loading an explicit header; ParseRaw does not make this call for 0.
local explicit=qualityItem("Rusted Cuirass","Quality: 19")
assert(explicit.quality==19)
explicit:NormaliseQuality()
assert(explicit.quality==20)
"#,
            )
            .set_name("@owned-quality-header-source-contrasts")
            .exec()
            .unwrap();
    }
}

#[test]
fn fresh_header_quality_is_not_reused_crafted_quality_or_alternate_quality_effect() {
    for jit_enabled in [false, true] {
        oracle(jit_enabled)
            .lua
            .load(
                r#"
local raw=qualityRaw("Rusted Cuirass","Quality: 20","{crafted}+5% to Quality")
local item=new("Item"):Item(raw,nil,false)
assert(item.quality==20 and item.craftedQuality==5 and item.armourData.Armour==54)
item:BuildModList()
assert(item.quality==20 and item.craftedQuality==5)
-- Reusing that object changes the effective value by the crafted-quality delta.
local changed=qualityRaw("Rusted Cuirass","Quality: 20","{crafted}+7% to Quality")
item:ParseRaw(changed,nil,false)
assert(item.quality==22 and item.craftedQuality==7 and item.armourData.Armour==55)
local fresh=new("Item"):Item(changed,nil,false)
assert(fresh.quality==20 and fresh.craftedQuality==7 and fresh.armourData.Armour==54)
-- Alternate quality changes the defence factor, not the imported quality input.
local alternate=qualityItem("Rusted Cuirass","Quality: 20","Quality does not increase Defences")
assert(alternate.quality==20 and alternate.armourData.Armour==45)
"#,
            )
            .set_name("@owned-quality-fresh-reused-and-alternate")
            .exec()
            .unwrap();
    }
}

#[test]
fn selected_original_armour_headers_keep_all_21_physical_values() {
    let selected: BTreeSet<_> = profiles()
        .profiles
        .into_iter()
        .filter_map(|row| {
            matches!(row.profile, DefenceProfilePresence::Table { .. }).then_some(row.base)
        })
        .collect();
    let originals = [
        (
            include_str!("../../../tests/fixtures/builds/breadth-20260908/build-01.xml"),
            "e3c0d0b40fa682260a1713acb03d52d720f4b769ac91b0501cbe2a84dc468194",
        ),
        (
            include_str!("../../../tests/fixtures/builds/breadth-20260908/build-02.xml"),
            "91366bd82a9afdd12ae7d8f695508a1b8d99116567010e082a9d31c4c4d4f631",
        ),
        (
            include_str!("../../../tests/fixtures/builds/breadth-20260908/build-03.xml"),
            "d3f7c72092f77481d3d1c5e38ec71d8730d607f19c659fc05b8a5db3bbbf9490",
        ),
        (
            include_str!("../../../tests/fixtures/builds/breadth-20260908/build-04.xml"),
            "62d760d326e21291cd1024f20660bd5046043c4e242b761df5d9a764be61e711",
        ),
        (
            include_str!("../../../tests/fixtures/builds/breadth-20260908/build-05.xml"),
            "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089",
        ),
    ];
    let expected = [
        BTreeMap::from([("1", 20), ("3", 20), ("4", 20), ("5", 20), ("8", 20)]),
        BTreeMap::from([("6", 19), ("20", 0), ("22", 0), ("25", 0)]),
        BTreeMap::from([("11", 20), ("12", 0), ("13", 20), ("14", 20)]),
        BTreeMap::from([("11", 20), ("12", 20), ("14", 20), ("21", 20)]),
        BTreeMap::from([("19", 20), ("20", 20), ("21", 20), ("22", 20)]),
    ];
    let oracle = oracle(false);
    let constructor: Function = oracle
        .lua
        .load("return function(raw) return new('Item'):Item(raw,nil,false) end")
        .eval()
        .unwrap();
    for ((xml, hash), expected) in originals.into_iter().zip(expected) {
        assert_eq!(format!("{:x}", Sha256::digest(xml.as_bytes())), hash);
        let doc = roxmltree::Document::parse(xml).unwrap();
        let items = doc
            .root_element()
            .children()
            .find(|node| node.has_tag_name("Items"))
            .unwrap();
        let item_set = items
            .children()
            .find(|node| {
                node.has_tag_name("ItemSet")
                    && node.attribute("id") == items.attribute("activeItemSet")
            })
            .unwrap();
        let secondary = item_set.attribute("useSecondWeaponSet") == Some("true");
        let active: BTreeSet<_> = item_set
            .children()
            .filter(|node| node.has_tag_name("Slot"))
            .filter_map(|node| {
                let name = node.attribute("name")?;
                let id = node.attribute("itemId")?;
                (id != "0"
                    && !["Flask", "Charm", "Arm ", "Leg "]
                        .iter()
                        .any(|prefix| name.starts_with(prefix))
                    && (!name.starts_with("Weapon") || name.ends_with(" Swap") == secondary))
                    .then_some(id)
            })
            .collect();
        let mut found = BTreeMap::new();
        for node in items.children().filter(|node| node.has_tag_name("Item")) {
            let id = node.attribute("id").unwrap();
            if !active.contains(id) {
                continue;
            }
            let raw = node
                .children()
                .filter(|node| node.is_text())
                .filter_map(|node| node.text())
                .collect::<String>();
            let bases: Vec<_> = raw
                .lines()
                .map(str::trim)
                .filter(|line| selected.contains(*line))
                .collect();
            if bases.is_empty() {
                continue;
            }
            assert_eq!(bases.len(), 1);
            let headers: Vec<_> = raw
                .lines()
                .filter_map(|line| line.trim().strip_prefix("Quality: "))
                .collect();
            assert_eq!(headers.len(), 1);
            let quality: i64 = headers[0].parse().unwrap();
            let item: Table = constructor.call(raw.clone()).unwrap();
            assert_eq!(item.get::<String>("baseName").unwrap(), bases[0]);
            assert_eq!(
                item.get::<i64>("quality").unwrap(),
                quality,
                "original item {id}"
            );
            assert_eq!(
                item.get::<i64>("craftedQuality").unwrap(),
                0,
                "original item {id}"
            );
            assert!(found.insert(id, quality).is_none());
        }
        assert_eq!(found, expected);
    }
}
