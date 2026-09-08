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
local function numeric(value, signed)
    assert(type(value)=='number' and value==value and value>=(signed and -1000000 or 0) and value<=1000000, 'unsupported numeric source value')
    return value
end
local function empty(t) return next(t)==nil end
local function keys(t,allowed,label)
    for k in pairs(t) do assert(allowed[k], 'unsupported '..label..' field '..tostring(k)) end
end
local function dense_array(t,label)
    assert(type(t)=='table','missing '..label..' array')
    local count=0
    for key in pairs(t) do
        assert(type(key)=='number' and key%1==0 and key>=1 and key<=#t,'non-array '..label..' field')
        count=count+1
    end
    assert(count==#t,'sparse '..label..' array')
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
        local resistances={FireResist='fire_resistance_flat',ColdResist='cold_resistance_flat',LightningResist='lightning_resistance_flat',ChaosResist='chaos_resistance_flat',ElementalResist='elemental_resistance_flat'}
        value=numeric(value,resistances[m.name]~=nil)
        stat=resistances[m.name] or ({Armour='armour_flat',Evasion='evasion_flat',EnergyShield='energy_shield_flat'})[m.name]
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
    return {skill_id=g.grantedEffectId,game_id=g.gameId,variant_id=g.variantId,name=g.name,requirements=sourceGemRequirements(g,assert(skills[id]))}
end
local type_names={
    [SkillType.Attack]='attack',[SkillType.MeleeSingleTarget]='melee_single_target',
    [SkillType.Melee]='melee',[SkillType.Area]='area',[SkillType.AttackInPlace]='attack_in_place',
    [SkillType.Damage]='damage',[SkillType.DamageOverTime]='damage_over_time',
    [SkillType.CrossbowAmmoSkill]='crossbow_ammo_skill',[SkillType.Herald]='herald',
    [SkillType.NoAttackOrCastTime]='no_attack_or_cast_time',
}
function source_skill_types(types,is_set)
    assert(type(types)=='table','skill types must be explicit')
    local result,seen={},{}
    for key,value in pairs(types) do
        if is_set then assert(value==true,'non-boolean active skill type') end
        local name=assert(type_names[is_set and key or value],'unsupported skill type or expression operator')
        assert(not seen[name],'duplicate source skill type');seen[name]=true
        result[#result+1]=name
    end
    if not is_set then
        for key in pairs(types) do assert(type(key)=='number' and key>=1 and key<=#types and key%1==0,'non-array eligibility expression') end
    end
    table.sort(result)
    return result
end
function source_extract_support(selection,level,quality)
    local support=gem(selection[2])
    local source=assert(skills[selection[2]])
    keys(source,{name=true,description=true,color=true,support=true,requireSkillTypes=true,addSkillTypes=true,excludeSkillTypes=true,gemFamily=true,levels=true,statSets=true},'support skill')
    assert(source.support==true and empty(source.addSkillTypes),'unsupported support identity or added skill types')
    assert(#source.gemFamily==1 and #source.statSets==1,'ambiguous support family/statset')
    keys(source.gemFamily,{[1]=true},'support family')
    keys(source.statSets,{[1]=true},'support statsets')
    keys(source.levels,{[level]=true},'support levels')
    local sourceLevel=source.levels[level]
    keys(sourceLevel,{levelRequirement=true,manaMultiplier=true},'support level')
    local stats=source.statSets[1]
    keys(stats,{label=true,baseEffectiveness=true,incrementalEffectiveness=true,statDescriptionScope=true,statMap=true,baseFlags=true,constantStats=true,stats=true,levels=true},'support statset')
    assert(empty(stats.baseFlags),'unsupported support base flags')
    keys(stats.levels,{[level]=true},'support statset levels')
    keys(stats.levels[level],{actorLevel=true},'support statset level')
    support.id=selection[1];support.family=source.gemFamily[1];support.level=level;support.quality=quality
    support.color=assert(({[1]='red',[2]='green',[3]='blue'})[source.color],'unsupported support color')
    support.mana_multiplier=sourceLevel.manaMultiplier
    support.eligibility={require_any=source_skill_types(source.requireSkillTypes,false),exclude=source_skill_types(source.excludeSkillTypes,false)}
    support.modifiers={};support.disable_damage={}
    local consumed={}
    dense_array(stats.constantStats,'support constants')
    dense_array(stats.stats,'support stats')
    for _,constant in ipairs(stats.constantStats) do
        dense_array(constant,'support constant pair')
        assert(#constant==2,'unexpected support constant shape')
        local name,value=constant[1],numeric(constant[2],true)
        assert(not consumed[name],'duplicate support stat');consumed[name]=true
        local mapping=assert(stats.statMap and stats.statMap[name] or sourceSupportStatMap[name],'missing support stat mapping')
        assert(#mapping==1,'unsupported support numeric modifier count')
        local modifier=mapping[1]
        local target=assert(({PhysicalDamage='physical_damage',Speed='speed'})[modifier.name],'unsupported support numeric target')
        local operation=assert(({INC='increased',MORE='more'})[modifier.type],'unsupported support numeric operation')
        local scope=assert(({[0]='any',[ModFlag.Melee]='melee',[ModFlag.Attack]='attack'})[modifier.flags],'unsupported support scope')
        assert(equal(mapping,{mod(modifier.name,modifier.type,nil,modifier.flags)}),'unconsumed support modifier fields or conditions')
        support.modifiers[#support.modifiers+1]={stat=target,operation=operation,scope=scope,value=value}
    end
    for _,name in ipairs(stats.stats) do
        assert(not consumed[name],'duplicate support stat');consumed[name]=true
        local mapping=assert(stats.statMap and stats.statMap[name] or sourceSupportStatMap[name],'missing support flag mapping')
        local expected={}
        for _,modifier in ipairs(mapping) do
            local damage=assert(({DealNoPhysical='physical',DealNoFire='fire',DealNoCold='cold',DealNoLightning='lightning',DealNoChaos='chaos'})[modifier.name],'unsupported support flag target')
            expected[#expected+1]=flag(modifier.name)
            support.disable_damage[#support.disable_damage+1]=damage
        end
        assert(#expected>0 and equal(mapping,expected),'unconsumed support flag fields or conditions')
    end
    for name in pairs(stats.statMap or {}) do assert(consumed[name],'unused support stat map') end
    return support
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
    local maceSource=skills[policy.mace_skill]
    mace.skill_types=source_skill_types(maceSource.skillTypes,true)
    keys(maceSource.levels[1].cost,{Mana=true},'Mace cost')
    mace.mana_cost=numeric(maceSource.levels[1].cost.Mana)
    assert(mace.mana_cost==0,'Mace resource cost requires schema expansion')
    mace.support_attribute_costs=sourceSupportRequirements({1,2,3})
    local supports={}
    for _,selection in ipairs(policy.supports) do
        supports[#supports+1]=source_extract_support(selection,policy.support_level,policy.support_quality)
    end
    local weapons={}
    for _,selection in ipairs(policy.weapons) do
        local base=assert(sourceBases[selection[2]],'missing weapon base')
        keys(base,{type=true,quality=true,socketLimit=true,tags=true,implicitModTypes=true,weapon=true,req=true},'weapon base')
        assert(base.type=='One Hand Mace' and empty(base.implicitModTypes),'unsupported weapon type or implicit')
        keys(base.weapon,{PhysicalMin=true,PhysicalMax=true,FireMin=true,FireMax=true,AttackRateBase=true,CritChanceBase=true,Range=true},'weapon stats')
        keys(base.req,{level=true,str=true,dex=true,int=true},'weapon requirements')
        local w=base.weapon
        weapons[#weapons+1]={id=selection[1],name=selection[2],physical_minimum=w.PhysicalMin,physical_maximum=w.PhysicalMax,fire_minimum=w.FireMin or 0,fire_maximum=w.FireMax or 0,attack_rate=w.AttackRateBase,critical_chance=w.CritChanceBase,requirements=sourceItemRequirements(base)}
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
    return {character=character,spark=spark,mace=mace,supports=supports,weapons=weapons,quests=quests,monsters={armour=data.monsterArmourTable,evasion=data.monsterEvasionTable}}
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