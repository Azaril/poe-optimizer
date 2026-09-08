//! Independent original ModStore/action/timing execution, cold and warmed.
use super::*;
use poe_optimizer_data::game_data::*;
fn load_action(oracle: &Oracle) {
    oracle
        .lua
        .load(format!(
            "local calcs=sourceCalcs;local m_min=math.min;local m_max=math.max;{}",
            section(
                &oracle.sources["src/Modules/CalcPerform.lua"],
                "function calcs.actionSpeedMod(actor)",
                "-- Initialises a minion's modifier database"
            )
        ))
        .exec()
        .unwrap();
}
#[test]
fn fresh_action_data_and_exact_tag_shapes_match_original_functions() {
    let data = poe_optimizer_pob::game_data::extract_pinned_game_data_for_review(
        &repository().join("vendor/path-of-building-poe2"),
    )
    .unwrap()
    .package;
    assert_eq!(data.actor.modifier_rules.len(), 359);
    let mut comparisons = 0;
    for warm in [false, true] {
        let o = Oracle::new(warm);
        load_action(&o);
        let lua = &o.lua;
        let misc = lua
            .globals()
            .get::<Table>("data")
            .unwrap()
            .get::<Table>("misc")
            .unwrap();
        assert_eq!(
            data.action_speed.temporal_chains_effect_cap,
            misc.get::<f64>("TemporalChainsEffectCap").unwrap()
        );
        assert_eq!(
            data.direct_action_timing.server_tick_rate,
            misc.get::<f64>("ServerTickRate").unwrap()
        );
        lua.globals()
            .set("reviewedAction", lua.to_value(&data.action_speed).unwrap())
            .unwrap();
        let check:Function=lua.load(r#"return function()
local p=reviewedAction;local cases=0
for _,action in ipairs({-217,-100,-31,0,17,123}) do
 for _,temporal in ipairs({-125,-75,-17,0,17}) do
  for _,minimum in ipairs({-1,0,57,100,150}) do
   for _,maximum in ipairs({-1,0,17,100,117}) do
    for _,unaffected in ipairs({false,true}) do
     for _,condition in ipairs({false,true}) do
      local db=new('ModDB'):ModDB();db.conditions.StrHigherThanInt=condition
      db:NewMod('ActionSpeed','INC',action,'Custom:Source')
      db:NewMod('ActionSpeed','INC',-31,'Custom:Source',{type='Condition',var='StrHigherThanInt'})
      db:NewMod('ActionSpeed','INC',17,'Item:1:Source')
      db:NewMod('TemporalChainsActionSpeed','INC',temporal,'Raw:Temporal')
      db:NewMod('MinimumActionSpeed','MAX',minimum,'Custom:Minimum')
      db:NewMod('MaximumActionSpeedReduction','MAX',maximum,'Raw:Enemy')
      if unaffected then db:NewMod('UnaffectedBySlows','FLAG',true,'Custom:Flag')end
      local a=(unaffected and math.max(0,action)or action)+17+(condition and not unaffected and -31 or 0)
      local t=unaffected and math.max(0,temporal)or temporal
      local floor=minimum>0 and minimum or p.default_minimum_percent
      local expected=math.max(floor/p.percent_divisor,p.base_multiplier+(math.max(-p.temporal_chains_effect_cap,t)+a)/p.percent_divisor)
      if maximum>0 then expected=math.min((p.percent_divisor-maximum)/p.percent_divisor,expected)end
      local value=sourceCalcs.actionSpeedMod({modDB=db});assert(value==expected,'action source values/query semantics differ')
      assert(db:Max(nil,'MinimumActionSpeed')==(minimum>0 and minimum or nil))
      cases=cases+1
     end
    end
   end
  end
 end
end
return cases end"#).eval().unwrap();
        comparisons += check.call::<usize>(()).unwrap();
        let parser: Function = lua
            .globals()
            .get::<Table>("modLib")
            .unwrap()
            .get("parseMod")
            .unwrap();
        for id in [
            "minimum_action_speed_base",
            "minimum_action_speed_you",
            "minimum_action_speed_cannot",
        ] {
            let rule = data.actor.modifier_rule(id).unwrap();
            assert_eq!(
                rule.modifiers[0].tags,
                [ActorModifierTag::GlobalEffect {
                    effect_type: ActorGlobalEffectType::Global,
                    unscalable: true
                }]
            );
            let (mods, extra): (Table, Option<String>) =
                parser.call(rule.template.as_str()).unwrap();
            assert!(extra.is_none());
            let source = mods.get::<Table>(1).unwrap();
            let raw = source.get::<Table>(1).unwrap();
            assert_eq!(raw.get::<String>("type").unwrap(), "GlobalEffect");
            assert_eq!(raw.get::<String>("effectType").unwrap(), "Global");
            assert!(raw.get::<bool>("unscalable").unwrap());
            assert_eq!(raw.pairs::<mlua::Value, mlua::Value>().count(), 3);
            let (mods, extra): (Option<Table>, Option<String>) = parser
                .call(format!(
                    "{} if Strength is higher than Intelligence",
                    rule.template
                ))
                .unwrap();
            assert!(mods.is_none() || extra.is_some());
        }
        for text in [
            "1.5% increased Action Speed",
            "Your speed is unaffected by slows if Strength is higher than Intelligence",
            "Your action speed is at least 100% of base value if Strength is higher than Intelligence",
        ] {
            let (mods, extra): (Option<Table>, Option<String>) = parser.call(text).unwrap();
            assert!(mods.is_none() || extra.is_some(), "{text}");
        }
        let enemy: (Table, Option<String>) = parser
            .call("Nearby enemy monsters' action speed is at most 90% of base value")
            .unwrap();
        assert!(enemy.1.is_none());
        assert_eq!(
            enemy
                .0
                .get::<Table>(1)
                .unwrap()
                .get::<String>("name")
                .unwrap(),
            "EnemyModifier"
        );
        if warm {
            lua.load("for i=1,500 do modLib.parseMod(i..'% increased Action Speed');modLib.parseMod('Your action speed is at least '..i..'% of base value')end").exec().unwrap();
        }
    }
    assert_eq!(comparisons, 6000);
}
#[test]
fn original_direct_timing_sets_cast_rate_after_action_speed_before_tick_cap() {
    let package = poe_optimizer_pob::game_data::extract_pinned_game_data_for_review(
        &repository().join("vendor/path-of-building-poe2"),
    )
    .unwrap()
    .package;
    let mut checks = 0;
    for warm in [false, true] {
        let o = Oracle::new(warm);
        let lua = &o.lua;
        let source = &o.sources["src/Modules/CalcOffence.lua"];
        let calculate:Function=lua.load(format!("local m_min=math.min;local m_max=math.max;return function(baseTime,skillModList,globalOutput,skillFlags,channel) local output={{}};local cfg=nil;local skillCfg=nil;local source={{}};local skillData={{}};local activeSkill={{skillTypes={{[SkillType.Channel]=channel}}}};local more=skillModList:More(cfg,'Speed');{}\n{}\n{}\nreturn output end",line(source,"output.Repeats = globalOutput.Repeats or"),section(source,"\n\t\t\tlocal inc = skillModList:Sum(\"INC\", cfg, \"Speed\")","\t\t\t-- Crossbows: Adjust attack speed values"),section(source,"\n\t\t\tif output.Speed == 0 then","\n\t\t\tif breakdown then"))).eval().unwrap();
        lua.globals().set("timingOriginal", calculate).unwrap();
        lua.globals()
            .set(
                "timingData",
                lua.to_value(&package.direct_action_timing).unwrap(),
            )
            .unwrap();
        let test:Function=lua.load(r#"return function()
local n=0;local p=timingData
for _,base in ipairs({.001,.033,.317,.7,1,2}) do
 for _,inc in ipairs({-101,-100,-99,0,17,5555}) do
  for _,more in ipairs({0.5,1,1.17}) do
   for _,action in ipairs({-.17,0,.57,1,2,100}) do
    for _,direct in ipairs({false,true})do
     for _,channel in ipairs({false,true})do
      local db=new('ModList'):ModList();db:NewMod('Speed','INC',inc);db:NewMod('Speed','MORE',(more-1)*100)
      local actual=timingOriginal(base,db,{ActionSpeedMod=action},{selfCast=direct},channel)
      local factor=round((1+inc/100)*db:More(nil,'Speed'),p.speed_multiplier_rounding_precision)
      local expected=1/(base/factor);if direct then expected=expected*action end
      local capped=channel and expected or math.min(expected,p.server_tick_rate*p.default_repeats)
      assert(actual.CastRate==expected and actual.Speed==capped,'CastRate must include action speed before capping Speed')
      assert(actual.Time==(capped==0 and 0 or 1/capped),'Time must use final capped rate, with exact zero branch')
      assert(actual.Repeats==p.default_repeats);n=n+1
     end
    end
   end
  end
 end
end
return n end"#).eval().unwrap();
        checks += test.call::<usize>(()).unwrap();
    }
    assert_eq!(checks, 5184);
}
#[test]
fn original_max_reevaluates_in_requesting_store_and_positive_sum_filters_tabulated_rows() {
    for warm in [false, true] {
        let o = Oracle::new(warm);
        o.lua.load(r#"local parent=new('ModDB'):ModDB();local child=new('ModDB'):ModDB(parent)
parent.multipliers.SourceScale=2;child.multipliers.SourceScale=3
parent:NewMod('MinimumActionSpeed','MAX',100,'Parent',{type='Multiplier',var='SourceScale'})
local rows=child:Tabulate('MAX',nil,'MinimumActionSpeed');assert(#rows==1 and rows[1].value==500)
local evaluations=0;local eval=child.EvalMod;child.EvalMod=function(self,...)evaluations=evaluations+1;return eval(self,...)end
assert(child:Max(nil,'MinimumActionSpeed')==500 and evaluations==2,'Max must EvalMod again after the requesting-context Tabulate evaluation')
parent:NewMod('ActionSpeed','INC',17,'Parent',{type='Multiplier',var='SourceScale'})
parent:NewMod('ActionSpeed','INC',-31,'Parent')
child:NewMod('ActionSpeed','INC',7,'Child')
assert(child:SumPositiveValues('INC',nil,'ActionSpeed')==92,'positive sum uses positive requesting-context evaluated rows')
for _,v in ipairs({-100,0,17})do local db=new('ModDB'):ModDB();db:NewMod('MinimumActionSpeed','MAX',v);assert(db:Max(nil,'MinimumActionSpeed')==(v>0 and v or nil))end
"#).exec().unwrap();
    }
}
