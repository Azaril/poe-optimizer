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
fn charm_numeric_samples_and_ranged_item_reconstruction_are_distinct() {
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

// Read one already rooted source closure upvalue; no Lua host capability is added.
fn charm_original_upvalue(lua: &mlua::Lua, function: &Function, requested: &str) -> Function {
    let mut found = None;
    for index in 1..=i32::from(function.info().num_upvalues) {
        // SAFETY: copy the existing closure's indexed upvalue without changing it.
        let (name, value): (String, Value) = unsafe {
            lua.exec_raw(function.clone(), |state| {
                let name = mlua::ffi::lua_getupvalue(state, 1, index);
                mlua::ffi::lua_pushstring(state, name);
                mlua::ffi::lua_insert(state, -2);
                mlua::ffi::lua_remove(state, 1);
            })
        }
        .unwrap();
        if name == requested {
            assert!(found.is_none(), "duplicate source closure {requested}");
            found = Some(value.as_function().unwrap().clone());
        }
    }
    found.expect("exact original closure upvalue")
}

fn charm_lifecycle_handles(oracle: &FormatterOracle) -> Table {
    let lua = &oracle.source.lua;
    let methods: Table = lua.globals().get("item_loading_functions").unwrap();
    let item_lib: Table = lua.globals().get("itemLib").unwrap();
    let mod_lib: Table = lua.globals().get("modLib").unwrap();
    let parser_observer: Function = mod_lib.get("parseMod").unwrap();
    let range_observer: Function = item_lib.get("applyRange").unwrap();
    let original_range = charm_original_upvalue(lua, &range_observer, "originalFormat");
    let parser_dependency: Function = lua.globals().get("item_loading_parse_dependency").unwrap();
    let parser = charm_original_upvalue(lua, &parser_dependency, "originalParseMod");
    assert_eq!(
        parser.to_pointer(),
        charm_original_upvalue(lua, &parser_observer, "originalParseMod").to_pointer()
    );
    let constructor: Function = methods.get("constructor").unwrap();
    let original_constructor = charm_original_upvalue(lua, &constructor, "originalFunc");
    let build: Function = methods.get("build_mod_list").unwrap();
    // Item.lua forward-declares this local at line 466. Bind the exact
    // dependency of the authenticated BuildModList closure, not a global.
    let ranged = charm_original_upvalue(lua, &build, "getRangedModList");
    let parse_raw: Function = methods.get("parse_raw").unwrap();
    assert_eq!(
        ranged.to_pointer(),
        charm_original_upvalue(lua, &parse_raw, "getRangedModList").to_pointer()
    );
    let handles = lua.create_table().unwrap();
    for (name, function, path, first) in [
        ("constructor", constructor, "@src/Modules/Common.lua", 167),
        (
            "original_constructor",
            original_constructor,
            "@src/Classes/Item.lua",
            93,
        ),
        (
            "parse_raw",
            methods.get("parse_raw").unwrap(),
            "@src/Classes/Item.lua",
            468,
        ),
        ("build", build, "@src/Classes/Item.lua", 2694),
        (
            "original_range",
            original_range,
            "@src/Modules/ItemTools.lua",
            130,
        ),
        ("range", range_observer, "@item-loading-test-observer", 175),
        (
            "zero",
            item_lib.get("isZeroValueLine").unwrap(),
            "@src/Modules/ItemTools.lua",
            72,
        ),
        ("ranged", ranged, "@src/Classes/Item.lua", 2682),
        ("parser", parser, "@src/Modules/ModParser.lua", 7404),
    ] {
        let info = function.info();
        assert_eq!(info.what, "Lua", "{name}");
        assert_eq!(info.source.as_deref(), Some(path), "{name}");
        assert_eq!(info.line_defined, Some(first), "{name}");
        handles.set(name, function).unwrap();
    }
    handles.set("parser_observer", parser_observer).unwrap();
    assert!(matches!(
        lua.globals().get::<Value>("debug").unwrap(),
        Value::Nil
    ));
    handles
}

const CACHE_LIFECYCLE: &str = r#"
local seed,raw,corruption,magnitude,auth=...
assert(auth.parse_raw==item_loading_functions.parse_raw)
assert(auth.build==item_loading_functions.build_mod_list)
assert(auth.constructor==item_loading_functions.constructor)
assert(auth.range==itemLib.applyRange and auth.zero==itemLib.isZeroValueLine)
-- Rust authenticated ranged as the exact local upvalue of auth.build.
assert(auth.parser_observer==modLib.parseMod)
local function copy(v)
 if type(v)~='table'then assert(type(v)~='function');return v end
 local out={};for k,c in pairs(v)do out[copy(k)]=copy(c)end;return out
end
local function records(list)
 local out={};for _,m in ipairs(list or{})do
  if m.name=='CharmLimit'then out[#out+1]=copy(m) end
 end;return out
end
local function row_state(row)
 return {line=row.line,range=row.range,value_scalar=row.valueScalar,corrupted_range=row.corruptedRange,
  extra=row.extra,has_mod_list=row.modList~=nil,mod_list=copy(row.modList),charm_records=records(row.modList)}
end
local function item_state(item)
 local out={has_base=item.base~=nil,base_name=item.baseName,capacity=item.charmLimit,lines={}}
 for _,kind in ipairs({'implicitModLines','explicitModLines'})do
  local rows={};for _,r in ipairs(item[kind]or{})do rows[#rows+1]=row_state(r)end
  out.lines[kind]=rows
 end
 out.base_records=records(item.baseModList)
 out.active_records=records(item:GetActiveModListForSlotNum(1,false))
 return out
end
local function capture(fn)
 local old_range,old_parser=itemLib.applyRange,modLib.parseMod
 local events={}
 itemLib.applyRange=function(...)
  local args={...};local result=old_range(...)
  events[#events+1]={kind='range',line=args[1],position=args[2],magnitude=args[3],corruption=args[4],rendered=result}
  return result
 end
 modLib.parseMod=function(text,combined,...)
  local list,extra=old_parser(text,combined,...)
  events[#events+1]={kind='parse',text=text,combined=combined==true,has_list=list~=nil,list=copy(list),remainder=extra,fully_consumed=list~=nil and not extra}
  return list,extra
 end
 local ok,result=pcall(fn)
 itemLib.applyRange,modLib.parseMod=old_range,old_parser
 assert(itemLib.applyRange==auth.range and modLib.parseMod==auth.parser_observer)
 return {ok=ok,error=not ok and tostring(result)or nil,result_type=type(result),truthy_result=not not result,events=events},result
end
local header='Rarity: Normal\nFine Belt\nImplicits: 1\n'
local seed_line='{tags:charm}{range:0.5}Has ('..seed..'-'..seed..') Charm Slot'
local seed_construction,item=capture(function()return new('Item'):Item(header..seed_line)end)
assert(seed_construction.ok,seed_construction.error)
assert(item.baseName=='Fine Belt' and item.base.charmLimit==0 and item.charmLimit==seed)
assert(item.Item==auth.constructor and #item.implicitModLines==1)
local row=item.implicitModLines[1]
local seed_state=row_state(row)
local cache=row.modList
row.line='Has ('..string.format('%.17g',raw)..'-'..string.format('%.17g',raw)..') Charm Slot'
row.corruptedRange=corruption;row.valueScalar=magnitude
local rendered=auth.range(row.line,row.range,row.valueScalar,row.corruptedRange)
local parsed,remainder=auth.parser(rendered)
local zero=not not auth.zero(rendered)
local ranged_call,ranged=capture(function()return auth.ranged(item,row)end)
assert(ranged_call.ok,ranged_call.error)
local first=capture(function()auth.build(item);return true end)
local after=item_state(item)
local kept_cache=row.modList==cache
local second=capture(function()auth.build(item);return true end)
local rebuilt=item_state(item)
local fresh,fresh_item=capture(function()return new('Item'):Item(header..rendered)end)
local fresh_state=fresh.ok and item_state(fresh_item)or nil
local fresh_rebuild,fresh_rebuilt
if fresh.ok then
 fresh_rebuild=capture(function()auth.build(fresh_item);return true end)
 fresh_rebuilt=item_state(fresh_item)
end
local classification
if zero then classification='zero-line-clears-local-records'
elseif parsed~=nil and not remainder then classification='rendered-line-fully-parsed'
else classification='rendered-line-not-fully-parsed' end
return {seed=seed,raw_count=raw,corruption=corruption,magnitude=magnitude,source_state_mutation=true,
 seed_raw=header..seed_line,seed_construction=seed_construction,seed_state=seed_state,
 altered_source_line=row.line,rendered=rendered,parse={has_list=parsed~=nil,list=copy(parsed),remainder=remainder,fully_consumed=parsed~=nil and not remainder},
 zero_line=zero,classification=classification,ranged_call=ranged_call,ranged_return={replacement_available=not not ranged,returned_kind=type(ranged),value=copy(ranged)},
 first_build=first,after=after,kept_seed_mod_list_identity=kept_cache,second_build=second,rebuilt=rebuilt,
 fresh_rendered_raw=header..rendered,fresh_constructor=fresh,fresh=fresh_state,fresh_rebuild=fresh_rebuild,fresh_rebuilt=fresh_rebuilt}
"#;

#[test]
fn charm_rendered_parser_and_seed_cache_are_separate_from_numeric_arithmetic() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    fs::create_dir_all(root.join("runs")).unwrap();
    let output = tempfile::Builder::new()
        .prefix("charm-domain-observation-cache-")
        .tempdir_in(root.join("runs"))
        .unwrap()
        .keep();
    let cases = [
        ("neutral", 2.0, 1.0, 1.0, true),
        ("corruption-nonneutral", 2.0, 1.5, 1.0, true),
        ("combined-nonneutral", 2.0, 1.5, 1.5, true),
        ("negative-magnitude", 2.0, 1.0, -1.0, false),
        ("zero-magnitude", 2.0, 1.0, 0.0, true),
        ("zero-corruption", 2.0, 0.0, 1.0, true),
        ("zero-raw", 0.0, 1.0, 1.0, true),
        ("large-decimal", 7.0, 1_000_000.0, 10_001.0, true),
        (
            "huge-rendering",
            1_000_000.0,
            1_000_000.0,
            2_f64.powi(53),
            false,
        ),
    ];
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
            let handles = charm_lifecycle_handles(&oracle);
            let observe = oracle
                .source
                .lua
                .load(CACHE_LIFECYCLE)
                .set_name("@test-only-charm-ranged-cache-observation")
                .into_function()
                .unwrap();
            let mut rows = Vec::new();
            for (id, raw, corruption, magnitude, expected_numeric_match) in cases {
                let native = native_component(Component::direct(raw, corruption, magnitude));
                for seed in [2, 7] {
                    let observed: Table = observe
                        .call((seed, raw, corruption, magnitude, handles.clone()))
                        .unwrap_or_else(|error| {
                            fs::write(
                                output.join(format!("jit-{jit}-replay-{replay}-failure.json")),
                                serde_json::to_vec_pretty(
                                    &json!({"status":"source_call_failed","case":id,"seed":seed,
                            "error":error.to_string(),"completed_cases":rows}),
                                )
                                .unwrap(),
                            )
                            .unwrap();
                            panic!(
                                "source lifecycle failed; evidence at {}: {error}",
                                output.display()
                            );
                        });
                    let source: Json = oracle
                        .source
                        .lua
                        .from_value(Value::Table(observed))
                        .unwrap();
                    rows.push(json!({"case":id,"expected_numeric_match":expected_numeric_match,"native_numeric_component":native,"source":source}));
                }
            }
            let report = json!({"jit_enabled":jit,"replay":replay,"cases":rows});
            fs::write(
                output.join(format!("jit-{jit}-replay-{replay}.json")),
                serde_json::to_vec_pretty(&report).unwrap(),
            )
            .unwrap();
            reports.push(report);
        }
    }
    let first = &reports[0]["cases"];
    for report in &reports[1..] {
        assert_eq!(first, &report["cases"]);
    }
    let rows = first.as_array().unwrap();
    assert_eq!(rows.len(), 18);
    for row in rows {
        let source = &row["source"];
        assert_eq!(source["first_build"]["ok"], true);
        assert_eq!(source["second_build"]["ok"], true);
        assert_eq!(source["after"], source["rebuilt"]);
        assert_eq!(
            source["ranged_call"]["events"][0]["rendered"],
            source["rendered"]
        );
        assert!(
            source["first_build"]["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|event| event["kind"] == "parse" && event["text"] == source["rendered"])
        );
        if row["expected_numeric_match"] == true {
            let native = row["native_numeric_component"].as_f64().unwrap();
            assert_eq!(
                source["after"]["capacity"].as_f64(),
                Some(native),
                "{}",
                row["case"]
            );
            assert_eq!(source["fresh_constructor"]["ok"], true);
            assert_eq!(source["fresh_rebuild"]["ok"], true);
            assert_eq!(
                source["fresh"]["capacity"].as_f64(),
                Some(native),
                "{}",
                row["case"]
            );
            assert_eq!(source["fresh"], source["fresh_rebuilt"]);
        } else {
            // This is a diagnostic source-history fact, not native gameplay semantics.
            assert_eq!(source["ranged_return"]["replacement_available"], false);
            assert_eq!(source["ranged_return"]["returned_kind"], "boolean");
            assert_eq!(source["ranged_call"]["result_type"], "boolean");
            assert_eq!(source["ranged_call"]["truthy_result"], false);
            assert_eq!(source["parse"]["fully_consumed"], false);
            assert_eq!(source["kept_seed_mod_list_identity"], true);
            assert_eq!(source["after"]["capacity"], source["seed"]);
            assert_eq!(source["fresh_constructor"]["ok"], true);
            assert_eq!(source["fresh_rebuild"]["ok"], true);
            assert_eq!(source["fresh"]["capacity"].as_f64(), Some(0.0));
            assert_eq!(source["fresh"], source["fresh_rebuilt"]);
        }
    }
    fs::write(output.join("summary.json"),serde_json::to_vec_pretty(&json!({
        "status":"source_observations_complete","fresh_replays":4,"cases_per_replay":18,
        "valid_numeric_cases":7,"seed_counts":[2,7],"source_manifest_sha256":poe_optimizer_pob::source::manifest_sha256(),
        "diagnostic_cases":rows.iter().filter(|r|r["expected_numeric_match"]==false).collect::<Vec<_>>(),
        "native_domain_changed":false,"source_admission_authority":false,"closure_authority":false,"full_build_evaluation":false
    })).unwrap()).unwrap();
    println!("Charm ranged cache observations: {}", output.display());
}
