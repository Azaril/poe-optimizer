//! Independent original Item/ModDB/movement pipeline, cold and warm.
use super::*;
use poe_optimizer_data::game_data::*;
#[test]
fn fresh_movement_data_matches_original_body_penalties_conditions_and_movement_branches() {
    let package = poe_optimizer_pob::game_data::extract_pinned_game_data_for_review(
        &repository().join("vendor/path-of-building-poe2"),
    )
    .unwrap()
    .package;
    assert_eq!(
        package
            .armour_bases
            .iter()
            .filter(|b| b.slot == EquipmentSlot::BodyArmour)
            .count(),
        114
    );
    let mut checks = 0;
    for warm in [false, true] {
        let o = Oracle::new(warm);
        let lua = &o.lua;
        lua.load(format!(
            "local calcs=sourceCalcs;local m_min=math.min;local m_max=math.max;{}",
            section(
                &o.sources["src/Modules/CalcPerform.lua"],
                "function calcs.actionSpeedMod(actor)",
                "-- Initialises a minion's modifier database"
            )
        ))
        .exec()
        .unwrap();
        let movement:Function=lua.load(format!("return function(actor) local modDB=actor.modDB;local output=actor.output;local m_max=math.max;{};return output end",section(&o.sources["src/Modules/CalcDefence.lua"],"\t-- Miscellaneous: move speed, avoidance, weapon swap speed","\n\tif breakdown then\n\t\tbreakdown.EffectiveMovementSpeedMod"))).eval().unwrap();
        let item:Function=lua.load(format!("local t_remove=table.remove;local m_floor=math.floor;{};return function(base) local self={{base=base,quality=0,armourData={{}},modSource='Item:42:Body source'}};local modList=new('ModList'):ModList();{};return modList end",section(&o.sources["src/Classes/Item.lua"],"local function calcLocal(","-- Build list of modifiers"),section(&o.sources["src/Classes/Item.lua"],"\t\tlocal armourData = self.armourData","\telseif self.base.flask then"))).eval().unwrap();
        let bases = lua.create_table().unwrap();
        lua.load(&o.sources["src/Data/Bases/body.lua"])
            .eval::<Function>()
            .unwrap()
            .call::<()>(bases.clone())
            .unwrap();
        assert_eq!(bases.clone().pairs::<String, Table>().count(), 347);
        lua.globals()
            .set("reviewedMovement", lua.to_value(&package.movement).unwrap())
            .unwrap();
        let check:Function=lua.load("return function(movement,mods,mode) local actor={modDB=new('ModDB'):ModDB(),output={}};local db=actor.modDB;for _,m in ipairs(mods)do db:AddMod(m)end;db.conditions.StrHigherThanInt=mode%2==1;if mode>=2 then db:NewMod('Condition:IgnoreMovementPenalties','FLAG',true,'Custom:Source',{type='Condition',var='StrHigherThanInt'})end;if mode>=4 then db:NewMod('MovementSpeed','INC',37,'Custom:Source')end;if mode>=6 then db:NewMod('MovementSpeed','MORE',17,'Custom:Source')end;if mode>=8 then db:NewMod('MovementSpeed','OVERRIDE',.317,'Custom:Source')end;if mode>=10 then db:NewMod('MovementSpeedCannotBeBelowBase','FLAG',true,'Custom:Source')end;local m=reviewedMovement;actor.output.ActionSpeedMod=sourceCalcs.actionSpeedMod(actor);assert(actor.output.ActionSpeedMod==1);local source=movement(actor);local expected=db:Override(nil,unpack(m.query_stats)) or round((m.base_multiplier+db:Sum('BASE',nil,'MovementSpeed'))*calcLib.mod(db,nil,'MovementSpeed'),m.rounding_precision);if db:Flag(nil,'MovementSpeedCannotBeBelowBase')then expected=math.max(expected,m.minimum_multiplier)end;assert(source.MovementSpeedMod==expected);assert(source.EffectiveMovementSpeedMod==round(expected*actor.output.ActionSpeedMod,m.rounding_precision));return source.MovementSpeedMod end").eval().unwrap();
        // Native stat enum serialization is intentionally distinct from upstream query names.
        lua.globals()
            .get::<Table>("reviewedMovement")
            .unwrap()
            .set(
                "query_stats",
                lua.create_sequence_from(["MovementSpeed"]).unwrap(),
            )
            .unwrap();
        for base in package
            .armour_bases
            .iter()
            .filter(|b| b.slot == EquipmentSlot::BodyArmour)
        {
            let raw: Table = bases.get(base.name.as_str()).unwrap();
            let mods: Table = item.call(raw).unwrap();
            assert_eq!(mods.raw_len(), 1);
            let penalty = base.movement_penalty.unwrap();
            let mapping = &package.movement.penalty_modifier;
            let ActorRuleEffect::Numeric {
                value: ActorRuleValue::Capture { multiplier, .. },
                ..
            } = mapping.effect
            else {
                panic!("penalty mapping")
            };
            assert_actor_source_modifier(
                mods.get(1).unwrap(),
                &ActorModifierRecord {
                    stat: ActorStat::MovementSpeed,
                    effect: ActorModifierEffect::Numeric {
                        operation: ActorNumericOperation::Base,
                        value: penalty * multiplier,
                    },
                    source: Some("Item:42:Body source".into()),
                    flags: mapping.flags,
                    keyword_flags: mapping.keyword_flags,
                    tags: mapping.tags.clone(),
                },
            );
            for mode in 0..12 {
                check
                    .call::<f64>((movement.clone(), mods.clone(), mode))
                    .unwrap();
                checks += 1;
            }
        }
        let none = lua.create_table().unwrap();
        none.set("armour", lua.create_table().unwrap()).unwrap();
        let empty: Table = item.call(none.clone()).unwrap();
        assert_eq!(empty.raw_len(), 0);
        none.get::<Table>("armour")
            .unwrap()
            .set("MovementPenalty", 0)
            .unwrap();
        let zero: Table = item.call(none).unwrap();
        assert_eq!(zero.raw_len(), 1);
        assert_eq!(
            zero.get::<Table>(1).unwrap().get::<f64>("value").unwrap(),
            0.0
        );
        let parser: Function = lua
            .globals()
            .get::<Table>("modLib")
            .unwrap()
            .get("parseMod")
            .unwrap();
        let rule = package
            .actor
            .modifier_rule("movement_speed_override")
            .unwrap();
        let ActorRuleEffect::Numeric {
            value: ActorRuleValue::CaptureDivided { divisor, .. },
            ..
        } = rule.modifiers[0].effect
        else {
            panic!("literaldivision")
        };
        let mut differs = false;
        for value in [0_u32, 29, 31, 57, 58, 117, 999, 1_000_000] {
            let (mods, extra): (Table, Option<String>) = parser
                .call(format!("Your movement speed is {value}% of its base value"))
                .unwrap();
            assert!(extra.is_none());
            let actual = mods.get::<Table>(1).unwrap().get::<f64>("value").unwrap();
            assert_eq!(actual, f64::from(value) / divisor);
            differs |= actual != f64::from(value) * (1.0 / divisor);
        }
        assert!(
            differs,
            "must prove literal division differs from reciprocal multiplication"
        );
        for text in [
            "Ignore all movement penalties from armour if Dexterity is higher than Intelligence",
            "Your movement speed is 31% of its base value if Strength is higher than Intelligence",
        ] {
            let (mods, extra): (Option<Table>, Option<String>) = parser.call(text).unwrap();
            assert!(mods.is_none() || extra.is_some());
        }
    }
    assert_eq!(checks, 2736);
}

#[test]
fn original_parser_folds_ascii_case_for_actor_flags_conditions_and_local_weapon_forms() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        let compare:Function=oracle.lua.load("return function(a,b)local x,ex=modLib.parseMod(a);local y,ey=modLib.parseMod(b);assert(x and y and not ex and not ey);local function eq(a,b)if type(a)~=type(b)then return false end;if type(a)~='table'then return a==b end;local na,nb=0,0;for k,v in pairs(a)do na=na+1;if not eq(v,b[k])then return false end end;for _ in pairs(b)do nb=nb+1 end;return na==nb end;return eq(x,y)end").eval().unwrap();
        for (a, b) in [
            (
                "Movement speed cannot be modified to below base value",
                "Movement Speed cannot be modified to below base value",
            ),
            (
                "Ignore all movement penalties from armour",
                "IGNORE ALL MOVEMENT PENALTIES FROM ARMOUR",
            ),
            ("+17.5 to Strength", "+17.5 TO sTrEnGtH"),
            (
                "20% increased Movement Speed if Strength is higher than Intelligence",
                "20% INCREASED movement speed IF STRENGTH IS HIGHER THAN INTELLIGENCE",
            ),
            (
                "Your movement speed is 57% of its base value",
                "YOUR Movement Speed IS 57% OF ITS BASE VALUE",
            ),
            ("Adds 3 to 7 Physical Damage", "adds 3 to 7 PHYSICAL DAMAGE"),
            (
                "25% increased Physical Damage",
                "25% INCREASED physical damage",
            ),
            ("12% increased Attack Speed", "12% INCREASED ATTACK SPEED"),
        ] {
            assert!(compare.call::<bool>((a, b)).unwrap(), "{b} warm{warm}");
        }
    }
}
