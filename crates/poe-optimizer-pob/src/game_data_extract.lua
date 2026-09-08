-- Pinned extraction conversion policy. No balance constants or calculated builds.
local function unique(values, predicate, label)
    local found, count
    count = 0
    for _, value in pairs(values) do
        if predicate(value) then found=value; count=count+1 end
    end
    assert(count==1, 'missing or ambiguous '..label)
    return found
end
source_unique = unique
local function equal(a,b,depth)
    depth=(depth or 0)+1
    assert(depth<=32,'modifier structure depth exceeded')
    if type(a)~=type(b) then return false end
    if type(a)~='table' then return a==b end
    for k,v in pairs(a) do if not equal(v,b[k],depth) then return false end end
    for k in pairs(b) do if a[k]==nil then return false end end
    return true
end
local function numeric(value)
    assert(type(value)=='number' and value==value and value>=0 and value<=1000000, 'unsupported numeric source value')
    return value
end
local function empty(t) return next(t)==nil end
local function keys(t,allowed,label)
    for k in pairs(t) do assert(allowed[k], 'unsupported '..label..' field '..tostring(k)) end
end
function source_convert_modifiers(mods)
    assert(type(mods)=='table' and #mods>=1 and #mods<=3,'unsupported modifier list')
    local m=mods[1]
    local value=m.value
    local stat, expected
    local make=modLib.createMod
    if m.name=='MinionModifier' and m.type=='LIST' then
        assert(type(value)=='table' and type(value.mod)=='table','invalid minion modifier')
        value=numeric(value.mod.value)
        stat='minion_damage_increased'
        expected={make('MinionModifier','LIST',{mod=make('Damage','INC',value)})}
    elseif m.name=='Speed' and m.type=='INC' then
        value=numeric(value); stat='skill_speed_increased'
        expected={make('Speed','INC',value),make('WarcrySpeed','INC',value),make('TotemPlacementSpeed','INC',value)}
    elseif m.type=='BASE' then
        value=numeric(value)
        stat=({Armour='armour_flat',Evasion='evasion_flat',EnergyShield='energy_shield_flat'})[m.name]
        assert(stat,'unsupported base modifier target')
        expected={make(m.name,'BASE',value)}
    elseif m.name=='Damage' and m.type=='INC' then
        value=numeric(value)
        local flags={[ModFlag.Spell]='spell_damage_increased',[ModFlag.Attack]='attack_damage_increased',[ModFlag.Melee]='melee_damage_increased',[ModFlag.Projectile]='projectile_damage_increased'}
        stat=flags[m.flags]; assert(stat,'unsupported damage modifier flags')
        expected={make('Damage','INC',value,nil,m.flags)}
    else error('unsupported modifier operation') end
    assert(equal(mods,expected),'unconsumed modifier flags, tags, actor scope or fields')
    return {stat=stat,value=value}
end
function source_extract_effect(text)
    local mods, extra=modLib.parseMod(text)
    assert(extra==nil,'partially parsed source stat')
    return source_convert_modifiers(mods)
end
local function gem(id)
    local g=unique(sourceGems,function(g)return g.grantedEffectId==id end,'gem '..id)
    return {skill_id=g.grantedEffectId,game_id=g.gameId,variant_id=g.variantId,name=g.name}
end
local function class_id(name)
    return unique(sourceTree.classes,function(c)return c.name==name end,'class '..name).integerId
end
local function one_mod(db,name)
    local mods=assert(db.mods[name],'missing resource initialization')
    assert(#mods==1,'ambiguous resource initialization')
    return mods[1]
end
function source_extract_records(policy)
    local constants=data.characterConstants
    local init=sourceResourceInitialization()
    local bonuses=sourceAttributeBonuses()
    local actor={modDB=new('ModDB'):ModDB(),output={}}
    sourceCalcs.doActorLifeManaSpirit(actor,true)
    local character={
        base_evasion=constants.base_evasion_rating,
        critical_damage_bonus=constants.base_critical_hit_damage_bonus,
        life_per_level=constants.life_per_level,
        initial_life=one_mod(init,'Life')[1].base,
        mana_per_level=constants.mana_per_level,
        initial_mana=one_mod(init,'Mana')[1].base,
        life_per_strength=bonuses:Sum('BASE',nil,'Life'),
        mana_per_intelligence=bonuses:Sum('BASE',nil,'Mana'),
        accuracy_per_level=constants.accuracy_rating_per_level,
        accuracy_per_dexterity=bonuses:Sum('BASE',nil,'Accuracy'),
        minimum_life=actor.output.Life,minimum_mana=actor.output.Mana,
    }
    assert(character.accuracy_per_dexterity==data.misc.AccuracyPerDexBase,'inconsistent accuracy operands')
    local spark=gem(policy.spark_skill)
    spark.default_class_id=class_id(policy.spark_default_class)
    local source=assert(skills[policy.spark_skill])
    local stats=source.statSets[1]
    assert(stats.stats[1]=='spell_minimum_base_lightning_damage' and stats.stats[2]=='spell_maximum_base_lightning_damage','Spark operand mapping changed')
    spark.lightning_minimum=stats.levels[1][1]; spark.lightning_maximum=stats.levels[1][2]
    spark.cast_time=source.castTime; spark.critical_chance=source.levels[1].critChance
    local mace=gem(policy.mace_skill)
    mace.default_class_id=class_id(policy.mace_default_class)
    assert(sourceBaseMultiplier(skills[policy.mace_skill].levels[1],{})==1,'Mace level-one multiplier needs schema expansion')
    mace.brutality=gem(policy.support_skill)
    local support=skills[policy.support_skill].statSets[1]
    assert(#support.constantStats==1 and support.constantStats[1][1]=='support_brutality_physical_damage_+%_final','unsupported Brutality constants')
    assert(equal(support.stats,{'deal_no_elemental_damage','base_deal_no_chaos_damage'}),'unsupported Brutality operations')
    mace.brutality.physical_more=support.constantStats[1][2]
    local weapons={}
    for _,selection in ipairs(policy.weapons) do
        local base=assert(sourceBases[selection[2]],'missing weapon base')
        keys(base,{type=true,quality=true,socketLimit=true,tags=true,implicitModTypes=true,weapon=true,req=true},'weapon base')
        assert(base.type=='One Hand Mace' and empty(base.implicitModTypes),'unsupported weapon type or implicit')
        keys(base.weapon,{PhysicalMin=true,PhysicalMax=true,FireMin=true,FireMax=true,AttackRateBase=true,CritChanceBase=true,Range=true},'weapon stats')
        keys(base.req,{str=true},'weapon requirements')
        local w=base.weapon
        weapons[#weapons+1]={id=selection[1],name=selection[2],physical_minimum=w.PhysicalMin,physical_maximum=w.PhysicalMax,fire_minimum=w.FireMin or 0,fire_maximum=w.FireMax or 0,attack_rate=w.AttackRateBase,critical_chance=w.CritChanceBase,required_strength=base.req.str or 0}
    end
    local quests={config_keys={},default_enabled={}}
    local targets={{'Life','BASE','flat_life'},{'Life','INC','life_increased'},{'Mana','INC','mana_increased'},{'ColdResist','BASE','elemental_resistance'},{'LightningResist','BASE','elemental_resistance'},{'FireResist','BASE','elemental_resistance'}}
    for i,info in ipairs(policy.quests) do
        local q=unique(data.questRewards,function(q)return q.Info==info end,'quest '..info)
        local key='quest'..q.Description..q.Area..q.Info
        quests.config_keys[i]=key
        local config=unique(sourceQuestConfig,function(c)return c.var==key end,'quest config '..key)
        assert(type(config.defaultState)=='boolean','quest default must be boolean')
        quests.default_enabled[i]=config.defaultState
        local mods,extra=modLib.parseMod(q.Stat)
        local target=targets[i]
        assert(extra==nil and #mods==1,'unsupported quest modifier count')
        local value=numeric(mods[1].value)
        assert(equal(mods,{modLib.createMod(target[1],target[2],value)}),'unsupported quest modifier structure')
        assert(quests[target[3]]==nil or quests[target[3]]==value,'elemental quest values diverged; schema expansion required')
        quests[target[3]]=value
    end
    return {character=character,spark=spark,mace=mace,weapons=weapons,quests=quests,monsters={armour=data.monsterArmourTable,evasion=data.monsterEvasionTable}}
end
function source_encounter_build(level)
    local build={characterLevel=level}
    local config={build=build,configSets={{input={},placeholder={}}},activeConfigSetId=1,UpdateLevel=sourceUpdateLevel}
    build.configTab=config
    config.varControls=setmetatable({},{__index=function(t,key)
        local control={SetPlaceholder=function(_,value) config.configSets[1].placeholder[key]=value end}
        rawset(t,key,control);return control
    end})
    return build
end