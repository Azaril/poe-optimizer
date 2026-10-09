//! Original item parsing at the boundaries of the native fixed-Life admission.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/player_resource_source.rs"]
mod resource;
use poe_optimizer_pob::source as pinned;
use resource::{hash, observe};
use serde_json::json;
use std::{fs, path::Path};
const OBSERVER: &str = r#"
local class = common.classes.Item
local function original(f, suffix, line)
    local d=debug.getinfo(f,"S")
    assert(d.what=="Lua" and d.source:gsub("\\","/"):sub(-#suffix)==suffix and d.linedefined==line)
    return f
end
local parse=original(class.ParseRaw,"Classes/Item.lua",468)
local mods=original(class.BuildModList,"Classes/Item.lua",2694)
local active=original(class.GetActiveModListForSlotNum,"Classes/Item.lua",2198)
local formatter=original(itemLib.formatValue,"Modules/ItemTools.lua",45)
assert(debug.gethook()==nil)
local saved={};for id,item in pairs(build.itemsTab.items) do saved[id]={item=item,raw=item.raw} end
local before=build.calcsTab.mainOutput.Life
local cases={
 {"fixed","+17 to maximum Life",0},{"zero","+0 to maximum Life",0},
 {"leading-zero","+00017 to maximum Life",0},{"upper-bound","+1000000 to maximum Life",0},
 {"beyond-bound","+1000001 to maximum Life",0},{"decimal-integer","+17.0 to maximum Life",0},
 {"fraction","+17.5 to maximum Life",0},{"negative","-17.5 to maximum Life",0},
 {"duplicate","+17 to maximum Life\n+17 to maximum Life",0},
 {"implicit-count","+17 to maximum Life",1},{"implicit-tag","{implicit}+17 to maximum Life",0},
 {"enchant-tag","{enchant}+17 to maximum Life",0},
 {"fixed-range-zero","{range:0}+17 to maximum Life",0},
 {"fixed-range-half","{range:0.5}+17 to maximum Life",0},
 {"fixed-range-one","{range:1}+17 to maximum Life",0},
 {"empty-tags","{tags:}+17 to maximum Life",0},
 {"fractured","{fractured}+17 to maximum Life",0},{"desecrated","{desecrated}+17 to maximum Life",0},
 {"unscalable","{unscalable}+17 to maximum Life",0},
 {"corrupted-identity","{corruptedRange:1}+17 to maximum Life",0},
 {"ranged","{range:0.5}+(10-20) to maximum Life",0}
}
local function records(list)
    local out={}
    for _,m in ipairs(list or {}) do
        if m.name=="Life" and m.type=="BASE" then
            local tags={};for i,t in ipairs(m) do tags[i]=copyTable(t) end
            out[#out+1]={value=m.value,flags=m.flags,keyword_flags=m.keywordFlags,tags=tags}
        end
    end
    return out
end
local results={}
for _,c in ipairs(cases) do
    local raw="Rarity: RARE\nAdmission Control\nSapphire Ring\nImplicits: "..c[3].."\n"..c[2]
    local item=new("Item"):Item("");item.id=9001;parse(item,raw)
    local lines={}
    for _,category in ipairs({"enchant","rune","implicit","explicit"}) do
        for _,line in ipairs(item[category.."ModLines"]) do
            if line.line:find("to maximum Life",1,true) then
                lines[#lines+1]={category=category,line=line.line,mod_tags=copyTable(line.modTags),
                    value_scalar=line.valueScalar,unscalable=line.unscalable,range=line.range,
                    corrupted_range=line.corruptedRange,extra=line.extra,parsed=records(line.modList)}
            end
        end
    end
    mods(item)
    results[#results+1]={name=c[1],body=c[2],implicit_count=c[3],raw=raw,lines=lines,active=records(active(item,1,false))}
end
for id,s in pairs(saved) do assert(build.itemsTab.items[id]==s.item and s.item.raw==s.raw) end
assert(build.calcsTab.mainOutput.Life==before)
assert(class.ParseRaw==parse and class.BuildModList==mods and class.GetActiveModListForSlotNum==active and itemLib.formatValue==formatter and debug.gethook()==nil)
return {controls=results,saved_items_preserved=true,output_preserved=true,methods_replaced=false}
"#;
fn child(root: &Path, out: &Path, jit: bool) {
    let originals = resource::originals(root);
    let mut rows = vec![];
    for (n, xml) in originals
        .iter()
        .chain(std::iter::once(&originals[4]))
        .enumerate()
    {
        let observed = observe(root, xml, None, jit, OBSERVER);
        for field in ["saved_items_preserved", "output_preserved"] {
            assert_eq!(observed["state"][field], true);
        }
        assert_eq!(observed["state"]["methods_replaced"], false);
        let expected = [
            vec![17.],
            vec![0.],
            vec![17.],
            vec![1_000_000.],
            vec![1_000_001.],
            vec![17.],
            vec![18.],
            vec![-18.],
            vec![17., 17.],
            vec![17.],
            vec![17.],
            vec![17.],
            vec![17.],
            vec![17.],
            vec![17.],
            vec![17.],
            vec![17.],
            vec![17.],
            vec![17.],
            vec![17.],
            vec![15.],
        ];
        let controls = observed["state"]["controls"].as_array().unwrap();
        assert_eq!(controls.len(), expected.len());
        for (c, e) in controls.iter().zip(expected) {
            let actual: Vec<_> = c["active"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v["value"].as_f64().unwrap())
                .collect();
            assert_eq!(actual, e, "{}", c["name"]);
        }
        if let Some(first) = rows.first() {
            let first: &serde_json::Value = first;
            assert_eq!(observed["state"], first["observed"]["state"]);
        }
        rows.push(json!({"name":if n==5{"fresh-repeat".into()}else{format!("original-{:02}",n+1)},"xml_sha256":hash(xml.as_bytes()),"observed":observed}));
    }
    let warm = observe(root, &originals[4], Some(&originals[1]), jit, OBSERVER);
    assert_eq!(warm, rows[4]["observed"]);
    let result = json!({"source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":hash(OBSERVER.as_bytes()),"witness_sha256":hash(include_bytes!("owned_flat_life_admission.rs")),"driver_sha256":hash(include_bytes!("support/player_resource_source.rs")),
        "files":(["src/Classes/Item.lua","src/Classes/ItemsTab.lua","src/Modules/ItemTools.lua","src/Modules/ModParser.lua","src/Data/ModScalability.lua"].map(|p|json!({"path":p,"sha256":pinned::expected_file_sha256(p).unwrap()}))),
        "cases":rows,"warm_restoration":warm,"complete_loads":8,"controls_per_observation":21,"whole_build_parity":false});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if jit { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "complete original source, both JIT modes and fresh output"]
fn original_fixed_life_admission_boundaries_are_stable() {
    resource::supervise(
        "original_fixed_life_admission_boundaries_are_stable",
        "POE_FIXED_LIFE_ADMISSION_CHILD",
        "POE_OPTIMIZER_TEST_FIXED_LIFE_ADMISSION_SOURCE_OUT",
        child,
    );
}
