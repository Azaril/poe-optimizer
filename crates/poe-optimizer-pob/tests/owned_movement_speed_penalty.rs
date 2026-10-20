//! Complete original Item construction and actual ModDB conditional filtering.
//! No final movement metric, complete source inventory or legality claim.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const NAME: &str = "actual_item_movement_penalty_retains_fraction_and_conditional_receiver";
const CHILD: &str = "POE_MOVEMENT_PENALTY_CHILD";
#[test]
fn actual_item_movement_penalty_retains_fraction_and_conditional_receiver() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        let enabled = mode == "on";
        let xml =
            fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
                .unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(xml.as_bytes())),
            "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089"
        );
        assert_eq!(
            pinned::expected_file_sha256("src/Classes/Item.lua").unwrap(),
            "97341d95bcc0863280fcf60e68af9459664a5ef06f588aaa0c8db1908f85f534"
        );
        let mut prior = None;
        for _ in 0..2 {
            let before = |lua: &Lua| {
                lua.globals().set("penaltyJit", enabled)?;
                lua.load("if penaltyJit then jit.on() else jit.off();jit.flush() end")
                    .exec()?;
                Ok(())
            };
            let temp = tempfile::tempdir().unwrap();
            let r = source::observe_with_build_hook_unwrapped(
                &root.join("vendor/path-of-building-poe2"),
                temp.path(),
                &xml,
                None,
                false,
                Some(&before),
                None,
                Some(&observe),
            )
            .unwrap();
            assert_eq!(r["configuration_method_wrappers"], false);
            assert_eq!(r["original_build_output_available"], true);
            let s = r["additional_observation"].clone();
            assert_eq!(s["preserved"], true);
            let cases = s["cases"].as_array().unwrap();
            assert_eq!(cases.len(), 4);
            for (i, (raw, filtered)) in [(0.03, -0.03), (0.07, -0.07), (0.0, 0.0)]
                .into_iter()
                .enumerate()
            {
                assert_eq!(cases[i]["raw"].as_f64().unwrap(), raw);
                assert_eq!(cases[i]["unignored"].as_f64().unwrap(), filtered);
                assert_eq!(cases[i]["ignored"], 0);
                assert_eq!(cases[i]["records"].as_array().unwrap().len(), 1);
                assert_eq!(
                    cases[i]["records"][0]["tags"],
                    serde_json::json!([{"type":"Condition","var":"IgnoreMovementPenalties","neg":true}])
                );
            }
            assert!(cases[3]["raw"].is_null());
            assert!(
                cases[3]["records"].as_array().is_some_and(Vec::is_empty)
                    || cases[3]["records"]
                        .as_object()
                        .is_some_and(serde_json::Map::is_empty)
            );
            assert_eq!(cases[3]["unignored"], 0);
            assert_eq!(cases[3]["ignored"], 0);
            if let Some(p) = &prior {
                assert_eq!(p, &s, "fresh matched lifecycle")
            } else {
                prior = Some(s)
            };
        }
        let path = PathBuf::from(std::env::var_os("POE_MOVEMENT_PENALTY_OUTPUT").unwrap())
            .join(format!("{}.json", mode.to_string_lossy()));
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        file.write_all(&serde_json::to_vec_pretty(&prior.unwrap()).unwrap())
            .unwrap();
        return;
    }
    let out = tempfile::Builder::new()
        .prefix("poe-movement-penalty-")
        .tempdir()
        .unwrap()
        .keep();
    for mode in ["off", "on"] {
        let log = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(out.join(format!("{mode}.log")))
            .unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env(CHILD, mode)
            .env("POE_MOVEMENT_PENALTY_OUTPUT", &out)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "source child failed; evidence {}",
                    out.display()
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(180) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source deadline: {}", out.display())
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    assert_eq!(
        fs::read(out.join("off.json")).unwrap(),
        fs::read(out.join("on.json")).unwrap()
    );
    eprintln!("source component evidence retained at {}", out.display());
}
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@movement-penalty-original-item-observer")
        .eval()?;
    Ok(lua.from_value(value)?)
}
const OBSERVE: &str = r##"
local function original(f,path,line)
 local i=debug.getinfo(f,"S");assert(i.what=="Lua" and i.source:gsub("\\","/"):sub(-#path)==path and i.linedefined==line);return f
end
local class=common.classes.Item
local parse=original(class.ParseRaw,"Classes/Item.lua",468)
local mods=original(class.BuildModList,"Classes/Item.lua",2694)
local slot=original(class.BuildModListForSlotNum,"Classes/Item.lua",2414)
local active=original(class.GetActiveModListForSlotNum,"Classes/Item.lua",2198)
local sum=original(common.classes.ModStore.Sum,"Classes/ModStore.lua",202)
local add=original(common.classes.ModDB.AddList,"Classes/ModDB.lua",113)
local selected=assert(build.itemsTab.items[19]);assert(selected.baseName=="Tattered Robe" and selected.base.armour.MovementPenalty==0.03 and not selected.base.weapon)
assert(build.calcsTab.mainEnv.player.itemList["Body Armour"]==selected)
local boots=assert(build.itemsTab.items[22]);assert(boots.baseName=="Cryptic Leggings" and not boots.base.armour.MovementPenalty)
local originalBase=selected.base;local originalArmour=selected.base.armour;local output=build.calcsTab.mainOutput;local scalars={}
for k,v in pairs(output)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then scalars[k]=v end end
local result={cases={}}
local function probe(name,raw,amount,change)
 local item=new("Item"):Item("");item.id=9901;parse(item,raw)
 if change then local b={};for k,v in pairs(item.base)do b[k]=v end;local a={};for k,v in pairs(b.armour)do a[k]=v end;a.MovementPenalty=amount;b.armour=a;item.base=b end
 mods(item);local list=active(item,1,false);local records={}
 for _,m in ipairs(list)do if m.name=="MovementSpeed"and m.type=="BASE"then local tags={};for _,t in ipairs(m)do local c={};for k,v in pairs(t)do c[k]=v end;tags[#tags+1]=c end;records[#records+1]={value=m.value,tags=tags};assert(m.source==item.modSource)end end
 local db=new("ModDB");add(db,list);db.conditions.IgnoreMovementPenalties=false;local no=sum(db,"BASE",nil,"MovementSpeed");db.conditions.IgnoreMovementPenalties=true;local yes=sum(db,"BASE",nil,"MovementSpeed")
 result.cases[#result.cases+1]={name=name,raw=item.base.armour.MovementPenalty,records=records,unignored=no,ignored=yes}
end
probe("actual-selected-source",selected.raw,nil,false)
probe("synthetic-private-base-changed",selected.raw,0.07,true)
probe("synthetic-private-base-zero",selected.raw,0,true)
probe("actual-absent-field-source",boots.raw,nil,false)
assert(selected==build.itemsTab.items[19]and selected.base==originalBase and selected.base.armour==originalArmour and originalArmour.MovementPenalty==0.03)
assert(class.ParseRaw==parse and class.BuildModList==mods and class.BuildModListForSlotNum==slot and class.GetActiveModListForSlotNum==active)
assert(output==build.calcsTab.mainOutput);for k,v in pairs(scalars)do assert(output[k]==v)end
result.preserved=true;return result
"##;
