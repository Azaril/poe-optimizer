//! Exploratory source/native observations, not owner-completeness authority.
use super::*;
use mlua::LuaSerdeExt;
use std::{fs, path::PathBuf};

const OBSERVE: &str = r#"
local raw,corruption,magnitude = ...
local format=itemLib.formatValue
local original={roundSymmetric=roundSymmetric,alwaysPositiveRound=alwaysPositiveRound,floorSymmetric=floorSymmetric}
local trace={}
for name,f in pairs(original) do
 _G[name]=function(...)
  local args={...};local value=f(...)
  trace[#trace+1]={helper=name,input=args[1],precision=args[2],output=value}
  return value
 end
end
local ok,text=pcall(format,raw,corruption,magnitude,1,0,false)
for name,f in pairs(original)do _G[name]=f end
assert(ok,text)
assert(itemLib.formatValue==format)
return {text=text,trace=trace}
"#;

// Factors are installed on a detached, actually parsed ranged source line. This
// is a source-state diagnostic, not a claim that current Import admits these tags.
const LIFECYCLE: &str = r#"
local raw,corruption,magnitude,parse_raw,build_mod_list = ...
local f=item_loading_functions
-- Rust authenticates these exact Function handles without exposing Lua debug.
assert(f.parse_raw==parse_raw and f.build_mod_list==build_mod_list)
local text='Rarity: Normal\nFine Belt\nImplicits: 1\n{tags:charm}{range:0.5}Has (2-2) Charm Slot'
local item=new('Item'):Item(text)
assert(item.baseName=='Fine Belt' and item.base.charmLimit==0)
assert(#item.implicitModLines==1)
local line=item.implicitModLines[1]
assert(line.line=='Has (2-2) Charm Slot' and line.range==0.5)
line.line='Has ('..string.format('%.17g',raw)..'-'..string.format('%.17g',raw)..') Charm Slot'
line.corruptedRange=corruption;line.valueScalar=magnitude
local state={raw_text=text,altered_source_line=line.line,raw_count=raw,corruption=corruption,magnitude=magnitude,source_state_mutation=true}
local function records(list)
 local out={};for _,m in ipairs(list or {})do
  if m.name=='CharmLimit'then out[#out+1]={name=m.name,type=m.type,value=m.value,flags=m.flags,keyword_flags=m.keywordFlags} end
 end;return out
end
state.parsed_before_mutation=records(line.modList)
local ok,err=pcall(f.build_mod_list,item)
state.ok=ok;state.error=not ok and tostring(err)or nil
if ok then
 state.capacity=item.charmLimit
 state.base_records=records(item.baseModList)
 state.remaining_local_records=records(item:GetActiveModListForSlotNum(1,false))
 local first=item.charmLimit;f.build_mod_list(item)
 state.rebuilt_capacity=item.charmLimit;assert(first==item.charmLimit)
end
return state
"#;

#[test]
fn charm_full_numeric_domain_samples_and_final_ranged_lifecycle_remain_separate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    fs::create_dir_all(root.join("runs")).unwrap();
    let output = tempfile::Builder::new()
        .prefix("charm-domain-observation-")
        .tempdir_in(root.join("runs"))
        .unwrap()
        .keep();
    let binding_bytes =
        fs::read(root.join("data/owned/poe2/3887ae68/fine-belt-modifiers/numeric-bindings.json"))
            .unwrap();
    let bindings: Json = serde_json::from_slice(&binding_bytes).unwrap();
    let binding = bindings
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["modifier"]["key"] == "def.0000000000003172")
        .unwrap();
    assert_eq!(binding["precision"], 1);
    assert_eq!(binding["display_precision"], 0);
    assert_eq!(binding["sign"]["kind"], "direct");
    assert_eq!(binding["scaling"]["kind"], "scaled");
    let mut cases = Vec::new();
    for raw in [
        0.0,
        0.5_f64.next_down(),
        0.5,
        0.5_f64.next_up(),
        1.49,
        1.5,
        1.51,
        2.0,
        7.0,
        999_999.5,
        1_000_000.0,
    ] {
        for corruption in [0.0, f64::MIN_POSITIVE, 0.5, 1.0, 1.5, 1_000_000.0] {
            for magnitude in [
                -9999.0,
                -1.0,
                0.0,
                f64::MIN_POSITIVE,
                0.5,
                1.0,
                1.5,
                10_001.0,
                2_f64.powi(52),
                2_f64.powi(53),
            ] {
                let c = Component::direct(raw, corruption, magnitude);
                cases.push((c, native_component(c)));
            }
        }
    }
    let mut reports = Vec::new();
    for jit in [false, true] {
        for replay in 0..2 {
            let oracle = FormatterOracle::new();
            oracle
                .source
                .lua
                .load(if jit {
                    "jit.on()"
                } else {
                    "jit.off();jit.flush()"
                })
                .exec()
                .unwrap();
            catalog_row(&oracle, "Has # Charm Slot", true);
            catalog_row(&oracle, "Has # Charm Slots", true);
            let observe = oracle
                .source
                .lua
                .load(OBSERVE)
                .set_name("@test-only-charm-numeric-observer")
                .into_function()
                .unwrap();
            let methods: Table = oracle
                .source
                .lua
                .globals()
                .get("item_loading_functions")
                .unwrap();
            let parse_raw: Function = methods.get("parse_raw").unwrap();
            let build_mod_list: Function = methods.get("build_mod_list").unwrap();
            for (function, first) in [(&parse_raw, 468), (&build_mod_list, 2694)] {
                let info = function.info();
                assert_eq!(info.what, "Lua");
                assert_eq!(info.source.as_deref(), Some("@src/Classes/Item.lua"));
                assert_eq!(info.line_defined, Some(first));
            }
            assert!(matches!(
                oracle.source.lua.globals().get::<Value>("debug").unwrap(),
                Value::Nil
            ));
            let lifecycle = oracle
                .source
                .lua
                .load(LIFECYCLE)
                .set_name("@test-only-charm-detached-lifecycle")
                .into_function()
                .unwrap();
            let mut numeric = Vec::new();
            for (c, native) in &cases {
                let source: Table = observe.call((c.raw, c.corruption, c.magnitude)).unwrap();
                let source: Json = oracle.source.lua.from_value(Value::Table(source)).unwrap();
                let parsed = source["text"].as_str().unwrap().parse::<f64>().unwrap();
                numeric.push(json!({"raw":c.raw,"corruption":c.corruption,"magnitude":c.magnitude,"native":native,"source":source,"equal":parsed==*native}));
            }
            // Preserve completed arithmetic observations even if item reconstruction fails.
            fs::write(
                output.join(format!(
                    "jit-{}-replay-{replay}-numeric.json",
                    if jit { "on" } else { "off" }
                )),
                serde_json::to_vec_pretty(&json!({
                    "status":"numeric_phase_complete","jit_enabled":jit,"replay":replay,
                    "numeric":numeric,"lifecycle_complete":false
                }))
                .unwrap(),
            )
            .unwrap();
            let mut items = Vec::new();
            for (raw, corruption, magnitude) in [
                (2.0, 1.0, 1.0),
                (2.0, 1.5, 1.0),
                (2.0, 1.5, 1.5),
                (2.0, 1.0, -1.0),
                (2.0, 1.0, 0.0),
                (2.0, 0.0, 1.0),
                (0.0, 1.0, 1.0),
                (7.0, 1_000_000.0, 10_001.0),
                (1_000_000.0, 1_000_000.0, 2_f64.powi(53)),
            ] {
                let source: Table =
                    lifecycle
                        .call((
                            raw,
                            corruption,
                            magnitude,
                            parse_raw.clone(),
                            build_mod_list.clone(),
                        ))
                        .unwrap_or_else(|error| {
                            fs::write(
                            output.join(format!(
                                "jit-{}-replay-{replay}-lifecycle-failure.json",
                                if jit { "on" } else { "off" }
                            )),
                            serde_json::to_vec_pretty(&json!({
                                "status":"lifecycle_call_failed","jit_enabled":jit,"replay":replay,
                                "raw":raw,"corruption":corruption,"magnitude":magnitude,
                                "error":error.to_string(),"completed_items":items
                            })).unwrap(),
                        ).unwrap();
                            panic!(
                                "lifecycle call failed; retained observations at {}: {error}",
                                output.display()
                            );
                        });
                let source: Json = oracle.source.lua.from_value(Value::Table(source)).unwrap();
                let native = native_component(Component::direct(raw, corruption, magnitude));
                items.push(json!({"source":source,"numeric_component":native,"capacity_equal":source["capacity"].as_f64()==Some(native)}));
            }
            let report = json!({"jit_enabled":jit,"replay":replay,"numeric":numeric,"detached_ranged_items":items});
            fs::write(
                output.join(format!(
                    "jit-{}-replay-{replay}.json",
                    if jit { "on" } else { "off" }
                )),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
            reports.push(report);
        }
    }
    let observed = &reports[0];
    for report in &reports[1..] {
        assert_eq!(observed["numeric"], report["numeric"]);
        assert_eq!(
            observed["detached_ranged_items"],
            report["detached_ranged_items"]
        );
    }
    let failures: Vec<_> = observed["numeric"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["equal"] != true)
        .collect();
    let lifecycle_differences: Vec<_> = observed["detached_ranged_items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["capacity_equal"] != true)
        .collect();
    fs::write(output.join("summary.json"),serde_json::to_vec_pretty(&json!({
        "status":"observations_complete","numeric_cases":cases.len(),"fresh_replays":4,
        "numeric_mismatches":failures,"lifecycle_differences":lifecycle_differences,
        "binding_sha256":format!("{:x}",Sha256::digest(&binding_bytes)),
        "source_manifest_sha256":poe_optimizer_pob::source::manifest_sha256(),
        "native_fixture":"existing generic numeric compiler helper; not actual complete3172 owner",
        "closure_authority":false,"source_admission_authority":false,"full_build_evaluation":false
    })).unwrap()).unwrap();
    println!("Charm domain observations: {}", output.display());
    assert_eq!(
        observed["detached_ranged_items"][1]["source"]["capacity"],
        3
    );
    assert!(
        failures.is_empty(),
        "actual numeric mismatch; retained all observations at {}",
        output.display()
    );
    // Lifecycle differences are intentionally evidence for review, not blessed
    // native behavior and not a new domain restriction. This is an observation probe.
}
