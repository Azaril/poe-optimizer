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
-- Select textual forms from the actual parser table, then derive every typed
-- mapping from actual parser results. Probe numbers identify parameter positions;
-- they are extraction operands, not game balance or generated build expectations.
function source_extract_item_rule(selection)
    local pattern=selection.form_pattern
    local form=assert(sourceItemForms[pattern],'item form absent from source parser')
    assert(form=='DMG' or form=='INC','unsupported item source form')
    local captures={}
    local at=1
    while true do
        local first,last=pattern:find('(%d+)',at,true)
        if not first then break end
        captures[#captures+1]='unsigned_integer';at=last+1
    end
    assert(#captures>=1 and #captures<=2,'unsupported item numeric source captures')
    local function render(values)
        local seen={}
        local text=selection.template:gsub('{(%d)}',function(index)
            local i=tonumber(index)+1
            assert(values[i]~=nil and not seen[i],'invalid item capture placeholder')
            seen[i]=true;return tostring(values[i])
        end)
        for i in ipairs(values) do assert(seen[i],'unused item capture') end
        return text
    end
    local probes=#captures==2 and {101,211} or {101}
    local mods,extra=modLib.parseMod(render(probes))
    assert(extra==nil,'partially parsed item source form')
    dense_array(mods,'item source modifiers')
    assert(#mods==#captures,'unsupported item source modifier count')
    local names={PhysicalMin='physical_minimum',PhysicalMax='physical_maximum',FireMin='fire_minimum',FireMax='fire_maximum',PhysicalDamage='physical_damage',Speed='speed',CritChance='critical_chance'}
    local mappings={}
    for _,modifier in ipairs(mods) do
        local stat=assert(names[modifier.name],'unsupported local item modifier target')
        local operation=assert(({BASE='base',INC='increased'})[modifier.type],'unsupported local item modifier operation')
        local capture
        for index,value in ipairs(probes) do if value==modifier.value then capture=index-1 end end
        assert(capture~=nil,'item source modifier does not preserve captured roll')
        local expectedFlags=modifier.name=='Speed' and ModFlag.Attack or 0
        assert(modifier.flags==expectedFlags and modifier.keywordFlags==0,'item source modifier is not exact local scope')
        assert(equal(modifier,modLib.createMod(modifier.name,modifier.type,modifier.value,nil,modifier.flags,modifier.keywordFlags)),'unconsumed item source fields or tags')
        mappings[#mappings+1]={stat=stat,operation=operation,capture=capture,flags=modifier.flags,keyword_flags=modifier.keywordFlags}
    end
    if form=='DMG' then
        assert(#mappings==2 and mappings[1].operation=='base' and mappings[2].operation=='base' and mappings[1].capture==0 and mappings[2].capture==1,'unsupported local damage source mapping shape')
        assert((mappings[1].stat=='physical_minimum' and mappings[2].stat=='physical_maximum') or (mappings[1].stat=='fire_minimum' and mappings[2].stat=='fire_maximum'),'unsupported local damage source endpoints')
    else
        assert(#mappings==1 and mappings[1].operation=='increased' and mappings[1].capture==0,'unsupported local increased source mapping shape')
        assert(mappings[1].stat=='physical_damage' or mappings[1].stat=='speed' or mappings[1].stat=='critical_chance','unsupported local increased source target')
    end
    for _,values in ipairs(#captures==2 and {{0,1},{17,37},{999,1000}} or {{0},{17},{999}}) do
        local actual,remainder=modLib.parseMod(render(values))
        local expected={}
        for index,mapping in ipairs(mappings) do
            local original=mods[index]
            expected[index]=modLib.createMod(original.name,original.type,values[mapping.capture+1],nil,original.flags,original.keywordFlags)
        end
        assert(remainder==nil and equal(actual,expected),'item source mapping changes with captured operands')
    end
    return {id=selection.id,template=selection.template,captures=captures,modifiers=mappings}
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
function source_critical_chance_cap(modifier)
    local value=numeric(modifier.value)
    assert(equal(modifier,modLib.createMod('CritChanceCap','BASE',value,'Base')),'unconsumed critical chance cap operation, source, flags or tags')
    return value
end

local receiving_stats={Armour=true,Evasion=true,EnergyShield=true,ArmourAndEvasion=true,Defences=true,FireResist=true,ColdResist=true,LightningResist=true,ChaosResist=true,ElementalResist=true}
local actor_stats={MovementSpeed=true,ArmourAndEnergyShield=true,EvasionAndEnergyShield=true,Armour=true,Evasion=true,EnergyShield=true,ArmourAndEvasion=true,Defences=true,FireResist=true,ColdResist=true,LightningResist=true,ChaosResist=true,ElementalResist=true,Str=true,Dex=true,Int=true,Life=true,Mana=true,Spirit=true,Accuracy=true,ExtraLife=true,ExtraMana=true,ExtraSpirit=true,LifeTotal=true,ManaTotal=true,SpiritTotal=true,LifeConvertToEnergyShield=true,LifeConvertToArmour=true,LifeConvertToEvasion=true,ManaConvertToEnergyShield=true,ManaConvertToArmour=true,ManaConvertToEvasion=true,SpiritConvertToEnergyShield=true,SpiritConvertToArmour=true,SpiritConvertToEvasion=true,DexAccBonusOverride=true,LowLifePercentage=true,FullLifePercentage=true}
local actor_flags={["Condition:IgnoreMovementPenalties"]=true,MovementSpeedCannotBeBelowBase=true,NoAttributeBonuses=true,DoubledInherentAttributeBonuses=true,NoStrengthAttributeBonuses=true,NoStrBonusToLife=true,HalvesLifeFromStrength=true,NoDexterityAttributeBonuses=true,NoDexBonusToAccuracy=true,NoIntelligenceAttributeBonuses=true,NoIntBonusToMana=true,ChaosInoculation=true}
local actor_conditions={IgnoreMovementPenalties=true,TwoHighestAttributesEqual=true,DexHigherThanInt=true,StrHigherThanInt=true,IntHigherThanDex=true,StrHigherThanDex=true,IntHigherThanStr=true,DexHigherThanStr=true,StrHighestAttribute=true,IntHighestAttribute=true,DexHighestAttribute=true,IntSingleHighestAttribute=true,DexSingleHighestAttribute=true}
local actor_operations={BASE='base',INC='increased',MORE='more',OVERRIDE='override'}
local function actor_name(name) if name=='Condition:IgnoreMovementPenalties' then return 'ignore_movement_penalties' end;return (name:gsub('(%l)(%u)','%1_%2'):lower()) end
function source_convert_actor_modifier(modifier)
    local tags,rawTags={},{}
    for key in pairs(modifier) do
        assert(({name=true,type=true,value=true,source=true,flags=true,keywordFlags=true})[key] or (type(key)=='number' and key%1==0 and key>=1 and key<=#modifier),'unconsumed actor modifier field')
    end
    assert(modifier.flags==0 and modifier.keywordFlags==0,'actor modifier has unsupported flags or keyword flags')
    assert(modifier.source==nil or type(modifier.source)=='string','actor source is not text')
    for _,tag in ipairs(modifier) do
        if tag.type=='Global' then
            keys(tag,{type=true},'Global marker');assert(receiving_stats[modifier.name],'Global marker is outside reviewed receiving targets')
            tags[#tags+1]={type='global'}
        else
            keys(tag,{type=true,var=true,varList=true,neg=true},'actor condition')
            assert(tag.type=='Condition' and ((type(tag.var)=='string' and tag.varList==nil) or (tag.var==nil and type(tag.varList)=='table')),'unsupported actor condition shape')
            assert(tag.neg==nil or type(tag.neg)=='boolean','invalid actor negation')
            local vars=tag.varList or {tag.var};dense_array(vars,'actor condition variables')
            local names={}
            for _,var in ipairs(vars) do assert(actor_conditions[var],'unsupported actor condition variable');assert(var~='IgnoreMovementPenalties' or modifier.name=='MovementSpeed','dynamic movement condition target permits a cycle or stage feedback');names[#names+1]=actor_name(var) end
            assert(#names>=1 and #names<=12,'actor condition count')
            tags[#tags+1]={type='condition',variables=names,negated=tag.neg or false}
        end
        rawTags[#rawTags+1]=tag
    end
    local effect
    if modifier.type=='FLAG' then
        assert(actor_flags[modifier.name] and type(modifier.value)=='boolean','unsupported actor flag target or value')
        effect={kind='flag',value=modifier.value}
    else
        assert(actor_stats[modifier.name] and actor_operations[modifier.type],'unsupported actor numeric target or operation')
        local allOperations={MovementSpeed=true,Str=true,Dex=true,Int=true,Life=true,Mana=true,Spirit=true,Accuracy=true}
        local localArmour=modifier.name=='ArmourAndEnergyShield' or modifier.name=='EvasionAndEnergyShield'
        assert((receiving_stats[modifier.name] and (modifier.type=='INC' or (modifier.type=='BASE' and modifier.name~='Defences'))) or (localArmour and (modifier.type=='BASE' or modifier.type=='INC')) or (not receiving_stats[modifier.name] and not localArmour and (allOperations[modifier.name] or (modifier.name=='DexAccBonusOverride' and modifier.type=='OVERRIDE') or (modifier.name~='DexAccBonusOverride' and modifier.type=='BASE'))),'actor numeric operation does not apply to target')
        effect={kind='numeric',operation=actor_operations[modifier.type],value=numeric(modifier.value,true)}
    end
    assert(equal(modifier,modLib.createMod(modifier.name,modifier.type,modifier.value,modifier.source,0,0,unpack(rawTags))),'unconsumed actor modifier structure')
    return {stat=actor_name(modifier.name),effect=effect,source=modifier.source,flags=modifier.flags,keyword_flags=modifier.keywordFlags,tags=tags}
end
local function actor_captures(pattern)
    local captures={}
    for token in pattern:gmatch('%b()') do
        if token=='(%d+)' then captures[#captures+1]='unsigned_integer'
        elseif token=='([%+%-][%d%.]+)' then captures[#captures+1]='signed_decimal'
        elseif token=='([%d%.]+)' then captures[#captures+1]='unsigned_decimal'
        else error('unrepresented actor source numeric capture '..token) end
    end
    return captures
end
function source_extract_actor_rule(selection)
    local actual
    if selection.source_kind=='form' then actual=assert(sourceActorForms[selection.pattern],'actor form missing from actual source table')
    elseif selection.source_kind=='special' then actual=assert(sourceActorSpecials[selection.pattern],'actor special missing from actual source table')
    else error('unknown actor source rule kind') end
    if selection.condition_pattern then assert(sourceActorTags[selection.condition_pattern],'actor condition pattern missing from source') end
    local captures=actor_captures(selection.pattern)
    local function render(values)
        local text=selection.template
        for i,kind in ipairs(captures) do
            local value=values[i]
            local word=(kind=='signed_decimal' and value>=0 and '+' or '')..tostring(value)
            local marker='{'..(i-1)..'}';local first=text:find(marker,1,true)
            assert(first and not text:find(marker,first+#marker,true),'missing/duplicate actor template capture')
            text=text:sub(1,first-1)..word..text:sub(first+#marker)
        end
        assert(not text:find('[{}]'),'unknown actor template placeholder')
        return text
    end
    local function parse(values)
        local mods,extra=modLib.parseMod(render(values))
        assert(mods and extra==nil,'actor template not fully consumed by actual source parser: '..render(values))
        dense_array(mods,'actor parser output');assert(#mods>=1 and #mods<=8,'unsupported actor output count')
        local converted={}
        for _,mod in ipairs(mods) do converted[#converted+1]=source_convert_actor_modifier(mod) end
        return converted
    end
    local values={101,211};local records=parse(values);local modifiers={}
    for _,record in ipairs(records) do
        local mapping={stat=record.stat,flags=record.flags,keyword_flags=record.keyword_flags,tags=record.tags}
        assert(record.source==nil,'actor grammar produced an unexpected intrinsic source')
        if record.effect.kind=='flag' then
            assert(#captures==0,'flag actor rule consumed numeric input without effect')
            mapping.effect=record.effect
        elseif #captures==0 then
            mapping.effect={kind='numeric',operation=record.effect.operation,value={kind='constant',value=record.effect.value}}
        else
            assert(#captures==1,'multiple actor numeric captures need an explicit source mapping expansion')
            local multiplier=record.effect.value/values[1]
            local value
            if selection.pattern=='your movement speed is (%d+)%% of its base value' then
                assert(record.stat=='movement_speed' and record.effect.operation=='override','movement override changed target')
                value={kind='capture_divided',index=0,divisor=values[1]/record.effect.value}
            else
                assert(multiplier==1 or multiplier==-1,'source actor capture has unrepresented numerical transform')
                value={kind='capture',index=0,multiplier=multiplier}
            end
            mapping.effect={kind='numeric',operation=record.effect.operation,value=value}
        end
        modifiers[#modifiers+1]=mapping
    end
    for _,probe in ipairs({0,1,17,999}) do
        local concrete=parse({probe,probe+37});local expected={}
        for _,mapping in ipairs(modifiers) do
            local effect=mapping.effect
            if effect.kind=='numeric' then
                effect={kind='numeric',operation=effect.operation,value=effect.value.kind=='capture' and probe*effect.value.multiplier or effect.value.kind=='capture_divided' and probe/effect.value.divisor or effect.value.value}
            end
            expected[#expected+1]={stat=mapping.stat,effect=effect,flags=mapping.flags,keyword_flags=mapping.keyword_flags,tags=mapping.tags}
        end
        assert(equal(concrete,expected),'actor source rule changes capture semantics')
    end
    if captures[1]=='signed_decimal' then
        local concrete=parse({-17.5})
        for i,mapping in ipairs(modifiers) do assert(concrete[i].effect.value==-17.5*mapping.effect.value.multiplier,'actor signed/decimal source grammar changed') end
    end
    return {id=selection.id,template=selection.template,captures=captures,modifiers=modifiers}
end
function source_extract_actor_data(policy,character,init,resource_actor)
    local function exact_base(name)
        local modifier=one_mod(init,name);local value=numeric(modifier.value)
        assert(equal(modifier,modLib.createMod(name,'BASE',value,'Base')),'actor base record changed shape')
        return value
    end
    for _,entry in ipairs({{'Life',character.life_per_level,character.initial_life},{'Mana',character.mana_per_level,character.initial_mana},{'Accuracy',character.accuracy_per_level,-character.accuracy_per_level}}) do
        assert(equal(one_mod(init,entry[1]),modLib.createMod(entry[1],'BASE',entry[2],'Base',{type='Multiplier',var='Level',base=entry[3]})),'actor level record changed complete shape')
    end
    local ci={modDB=new('ModDB'):ModDB(),output={}};ci.modDB:NewMod('ChaosInoculation','FLAG',true);sourceCalcs.doActorLifeManaSpirit(ci,true)
    local data_actor={
        initial_spirit=exact_base('Spirit'),minimum_spirit=resource_actor.output.Spirit,
        low_life_threshold=resource_actor.output.LowLifePercentage/100,full_life_threshold=resource_actor.output.FullLifePercentage/100,
        attribute_bonus_multiplier=sourceNormalAttributeMultiplier,
        doubled_attribute_bonus_multiplier=sourceAttributeBonuses({'DoubledInherentAttributeBonuses'}):Sum('BASE',nil,'Life')/character.life_per_strength,
        halved_life_per_strength=sourceAttributeBonuses({'HalvesLifeFromStrength'}):Sum('BASE',nil,'Life')/sourceNormalAttributeMultiplier,
        chaos_inoculation_life=ci.output.Life,
        high_precision_mods={},spirit_quests={},modifier_rules={}
    }
    for name,operations in pairs(data.highPrecisionMods) do
        local record={}
        for operation,places in pairs(operations) do
            assert(actor_operations[operation] and type(places)=='number' and places%1==0 and places>=0 and places<=15,'unsupported source precision record')
            record[actor_operations[operation]]=places
        end
        assert(not empty(record),'empty source precision record');data_actor.high_precision_mods[name]=record
    end
    for _,info in ipairs(policy.spirit_quests) do
        local quest=unique(data.questRewards,function(q)return q.Info==info end,'Spirit quest '..info)
        local key='quest'..quest.Description..quest.Area..quest.Info
        local config=unique(sourceQuestConfig,function(c)return c.var==key end,'Spirit quest config')
        assert(config.type=='check' and type(config.defaultState)=='boolean','unsupported Spirit quest config')
        local db=new('ModDB'):ModDB();config.apply(true,db,new('ModDB'):ModDB())
        local modifier=one_mod(db,'Spirit')
        local record=source_convert_actor_modifier(modifier)
        assert(record.stat=='spirit' and record.effect.kind=='numeric' and record.effect.operation=='base' and #record.tags==0,'Spirit quest changed target/operation')
        local count=0;for _,mods in pairs(db.mods) do count=count+#mods end;assert(count==1,'unconsumed Spirit quest modifiers')
        data_actor.spirit_quests[#data_actor.spirit_quests+1]={config_key=key,default_enabled=config.defaultState,modifiers={record}}
    end
    for _,selection in ipairs(policy.actor_rules) do data_actor.modifier_rules[#data_actor.modifier_rules+1]=source_extract_actor_rule(selection) end
    return data_actor
end

function source_extract_receiving_defence()
    local resources,resistances={},{}
    dense_array(sourceReceiverResources,'receiving source resources');assert(#sourceReceiverResources==6,'receiving resource stage changed')
    for i,source in ipairs(sourceReceiverResources) do
        if i<=3 then
            keys(source,{name=true,basePerSlot=true,globalBase=true,conversionRate=true,mods=true,modsTotal=true,defence=true},'receiving resource')
            assert(source.defence==true and empty(source.basePerSlot) and empty(source.conversionRate) and source.globalBase==0,'receiving initial state changed')
            assert(equal(source.modsTotal,{source.name..'Total'}),'receiving total-stat query changed')
            local names={};dense_array(source.mods,'receiving query names')
            for _,name in ipairs(source.mods)do assert(receiving_stats[name],'unknown receiving source query');names[#names+1]=actor_name(name)end
            resources[#resources+1]={stat=actor_name(source.name),query_stats=names}
        end
    end
    dense_array(sourceReceiverResistTypes,'source resistance types')
    for _,name in ipairs(sourceReceiverResistTypes)do
        local names={actor_name(name..'Resist')};assert(receiving_stats[name..'Resist'],'unknown source resistance')
        if sourceReceiverElemental[name] then names[#names+1]='elemental_resist' end
        resistances[#resistances+1]={stat=actor_name(name..'Resist'),query_stats=names}
    end
    return {resources=resources,resistances=resistances}
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
        critical_chance_cap=source_critical_chance_cap(one_mod(init,'CritChanceCap')),
        life_per_level=constants.life_per_level,
        initial_life=one_mod(init,'Life')[1].base,
        mana_per_level=constants.mana_per_level,
        initial_mana=one_mod(init,'Mana')[1].base,
        life_per_strength=bonuses:Sum('BASE',nil,'Life')/sourceNormalAttributeMultiplier,
        mana_per_intelligence=bonuses:Sum('BASE',nil,'Mana')/sourceNormalAttributeMultiplier,
        accuracy_per_level=constants.accuracy_rating_per_level,
        accuracy_per_dexterity=bonuses:Sum('BASE',nil,'Accuracy')/sourceNormalAttributeMultiplier,
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
    local item_modifier_rules={}
    for _,selection in ipairs(policy.item_rules) do item_modifier_rules[#item_modifier_rules+1]=source_extract_item_rule(selection) end
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
    return {receiving_defence=source_extract_receiving_defence(),movement=source_extract_movement(),character=character,actor=source_extract_actor_data(policy,character,init,actor),spark=spark,mace=mace,supports=supports,weapons=weapons,item_modifier_rules=item_modifier_rules,quests=quests,monsters={armour=data.monsterArmourTable,evasion=data.monsterEvasionTable}}
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
-- Complete original PassiveTree.ProcessStats output, including multiline combination.
-- BASE/INC integer source sums and boolean flags are admitted; source table traversal
-- does not establish a portable ordering for passive MORE or competing OVERRIDE.
function source_extract_passive(stats,id,name)
 local node={id=id,sd=copyTable(stats),dn=name,type='Normal'}
 local ok=pcall(PassiveTreeClass.ProcessStats,PassiveTreeClass,node)
 if not ok or node.unknown or node.extra then return nil,'unsupported_source_parser_output' end
 local effects,actor={},{}
 local forbidden={ArmourAndEnergyShield=true,EvasionAndEnergyShield=true,LifeConvertToEnergyShield=true,LifeConvertToArmour=true,LifeConvertToEvasion=true,ManaConvertToEnergyShield=true,ManaConvertToArmour=true,ManaConvertToEvasion=true,SpiritConvertToEnergyShield=true,SpiritConvertToArmour=true,SpiritConvertToEvasion=true,ChaosInoculation=true}
 for _,line in ipairs(node.mods) do
  local list=copyTable(line.list or {})
  if #list>0 then
   local converted={};local allactor=true
   for _,mod in ipairs(list) do
    local success,record=pcall(source_convert_actor_modifier,mod)
    if not success or forbidden[mod.name] or (mod.type~='BASE' and mod.type~='INC' and mod.type~='FLAG') then allactor=false;break end
    assert(type(mod.value)~='number' or mod.value%1==0,'noninteger passive actor source requires explicit order audit')
    converted[#converted+1]=record
   end
   if allactor then for _,record in ipairs(converted) do actor[#actor+1]=record end
   else
    for _,mod in ipairs(list) do
     assert(mod.source=='Tree:'..id,'unexpected passive modifier source');mod.source=nil
     if type(mod.value)=='table' and mod.value.mod then assert(mod.value.mod.source=='Tree:'..id,'unexpected nested passive source');mod.value.mod.source=nil end
    end
    local success,effect=pcall(source_convert_modifiers,list)
    if not success then return nil,'unsupported_whole_modifier_effect' end
    effects[#effects+1]=effect
   end
  end
 end
 return {effects=effects,actor_modifiers=actor}
end
-- Enumerate an entire source base family and admit by complete data/implicit capability.
function source_extract_jewellery(rules)
 local result={};local excluded={}
 local allowed={type=true,tags=true,implicit=true,implicitModTypes=true,req=true}
 for name,base in pairs(sourceJewelleryBases) do
  local ok,value=pcall(function()
   keys(base,allowed,'jewellery base');assert(base.type=='Amulet','unsupported jewellery type')
   assert(equal(base.tags,{amulet=true,default=true}),'unsupported jewellery base tags')
   keys(base.req or {},{level=true,str=true,dex=true,int=true},'jewellery requirements')
   local text=assert(base.implicit);assert(type(text)=='string' and not text:find('\n',1,true),'multiple implicit source lines')
   local minimum,maximum=text:match('^%+%((%d+)%-(%d+)%)')
   assert(minimum and maximum,'unsupported source implicit range grammar')
   minimum=tonumber(minimum);maximum=tonumber(maximum);numeric(minimum);numeric(maximum);assert(minimum<=maximum)
   local template=text:gsub('^%+%(%d+%-%d+%)','{0}')
   local rule=unique(rules,function(r)return r.template==template and #r.captures==1 and r.captures[1]=='signed_decimal' end,'jewellery actor rule')
   local parsed,extra=modLib.parseMod(text:gsub('^%+%(%d+%-%d+%)','+101'))
   assert(extra==nil);dense_array(parsed,'jewellery implicit')
   for _,m in ipairs(parsed) do assert(m.type=='BASE');source_convert_actor_modifier(m) end
   assert(#parsed==#rule.modifiers,'incomplete jewellery implicit rule')
   for i,m in ipairs(parsed) do
    local map=rule.modifiers[i];assert(map.effect.kind=='numeric' and map.effect.operation=='base' and map.effect.value.kind=='capture' and map.effect.value.index==0 and map.effect.value.multiplier==1,'unsupported implicit parameter mapping')
    assert(m.flags==map.flags and m.keywordFlags==map.keyword_flags and #m==0 and #map.tags==0,'implicit scope mismatch')
    assert(actor_name(m.name)==map.stat and m.value==101,'implicit target/value mismatch')
   end
   dense_array(base.implicitModTypes,'implicit type groups');assert(#base.implicitModTypes==1);dense_array(base.implicitModTypes[1],'implicit types')
   return {id=name:lower():gsub(' ','_'),name=name,slot='amulet',requirements={level=(base.req or{}).level or 0,attributes={strength=(base.req or{}).str or 0,dexterity=(base.req or{}).dex or 0,intelligence=(base.req or{}).int or 0}},implicit={actor_rule_id=rule.id,minimum=minimum,maximum=maximum,source_text=text,modifier_types=base.implicitModTypes[1]}}
  end)
  if ok then result[#result+1]=value else excluded[#excluded+1]=name end
 end
 table.sort(result,function(a,b)return a.id<b.id end);table.sort(excluded)
 return result,excluded
end

-- Enumerate the four complete fixed armour families; unsupported whole bases
-- stay excluded instead of stripping a Ward, implicit or hidden source field.
function source_extract_armour()
 local result,excluded={},{}
 local allowed={type=true,subType=true,quality=true,socketLimit=true,tags=true,implicitModTypes=true,armour=true,req=true}
 local slots={Helmet='helmet',Gloves='gloves',Boots='boots',['Body Armour']='body_armour'}
 local subtypes={['Armour']=true,['Evasion']=true,['Energy Shield']=true,['Armour/Evasion']=true,['Armour/Energy Shield']=true,['Evasion/Energy Shield']=true,['Armour/Evasion/Energy Shield']=true}
 local tagNames={armour=true,default=true,helmet=true,gloves=true,boots=true,body_armour=true,str_armour=true,dex_armour=true,int_armour=true,str_dex_armour=true,str_int_armour=true,dex_int_armour=true,str_dex_int_armour=true,ezomyte_basetype=true,maraketh_basetype=true,vaal_basetype=true,karui_basetype=true}
 for name,base in pairs(sourceArmourBases)do
  local ok,value=pcall(function()
   keys(base,allowed,'armour base');assert(slots[base.type] and subtypes[base.subType],'unsupported armour slot/subtype')
   assert(base.quality==20 and base.socketLimit==(base.type=='Body Armour' and 4 or 3),'unsupported base quality/socket defaults')
   keys(base.tags,tagNames,'armour base tags');assert(base.tags.armour==true and base.tags.default==true and base.tags[slots[base.type]]==true,'missing armour base tags')
   for tag,value in pairs(base.tags)do assert(value==true,'nonboolean armour tag');if tag=='helmet' or tag=='gloves' or tag=='boots' or tag=='body_armour' then assert(tag==slots[base.type],'conflicting armour slot tag')end end
   assert(empty(base.implicitModTypes),'unconsumed armour implicit metadata')
   keys(base.req or {},{level=true,str=true,dex=true,int=true},'armour requirements')
   for key,value in pairs(base.req or {})do numeric(value);assert(value%1==0 and (key~='level' or value<=100),'unsupported armour requirement')end
   keys(base.armour,{Armour=true,Evasion=true,EnergyShield=true,MovementPenalty=true},'fixed armour ratings');assert(not empty(base.armour),'empty armour base')
   for _,value in pairs(base.armour)do numeric(value)end
   local id=name:lower():gsub('[^%w]+','_'):gsub('^_',''):gsub('_$','')
   return {id=id,name=name,slot=slots[base.type],quality=base.quality,armour=base.armour.Armour or 0,evasion=base.armour.Evasion or 0,energy_shield=base.armour.EnergyShield or 0,movement_penalty=base.armour.MovementPenalty,requirements={level=(base.req or{}).level or 0,attributes={strength=(base.req or{}).str or 0,dexterity=(base.req or{}).dex or 0,intelligence=(base.req or{}).int or 0}}}
  end)
  if ok then result[#result+1]=value else excluded[#excluded+1]=name end
 end
 table.sort(result,function(a,b)return a.id<b.id end);table.sort(excluded)
 return result,excluded
end

-- Observe the original ItemTools dispatcher with one authenticated metadata row
-- at a time. Its format-label interpretation remains original source code.
function source_extract_item_formatting(records)
 local function shape(text)
  return text:gsub('{%d+}','#'):gsub('[%+%-]?#','#'):gsub('[%+%-]?%d+%.?%d*','#'):lower()
 end
 local shapes={}
 for _,rule in ipairs(records.actor.modifier_rules)do shapes[shape(rule.template)]=true end
 for _,rule in ipairs(records.item_modifier_rules)do shapes[shape(rule.template)]=true end
 local all=data.modScalability;local original=itemLib.formatValue;local rules={}
 for template,metadata in pairs(all)do
  if shapes[shape(template)]then
   dense_array(metadata,'item formatting metadata')
   local _,count=template:gsub('#','');assert(count==#metadata and count<=2,'item formatting arity changed')
   for _,record in ipairs(metadata)do
    keys(record,{isScalable=true,formats=true},'item scalability record');assert(type(record.isScalable)=='boolean','item scalability flag')
    if record.formats then dense_array(record.formats,'item format labels');for _,label in ipairs(record.formats)do assert(type(label)=='string','item format label')end end
   end
   local captures={}
   itemLib.formatValue=function(value,baseScalar,scalar,precision,displayPrecision,ifRequired)
    assert(baseScalar==1 and scalar==1,'non-unit item format probe')
    captures[#captures+1]={precision=precision,display_precision=displayPrecision,trim_trailing_zeroes=ifRequired==true}
    return original(value,baseScalar,scalar,precision,displayPrecision,ifRequired)
   end
   data.modScalability={[template]=metadata}
   local probe=template:gsub('#','37.25')
   itemLib.applyRange(probe,1,1)
   data.modScalability=all;itemLib.formatValue=original
   assert(#captures==count,'source item formatter did not consume exact key')
   rules[#rules+1]={template=template,captures=captures}
  end
 end
 table.sort(rules,function(a,b)return a.template<b.template end)
 return {rules=rules}
end

-- Execute actual source branches to observe defaults, queries, precision and the
-- synthetic item modifier. Neutral action speed remains an admission proof.
function source_extract_movement()
 local baseActor={modDB=new('ModDB'):ModDB(),output={}}
 local action=sourceCalcs.actionSpeedMod(baseActor);assert(action==1,'nonneutral action baseline requires offence support')
 baseActor.output.ActionSpeedMod=action
 local originalRound=round;local precision
 sourceMovementRound=function(value,places)assert(type(places)=='number' and places%1==0 and (precision==nil or precision==places),'movement rounding changed');precision=places;return originalRound(value,places)end
 local base=sourceMovement(baseActor).MovementSpeedMod
 sourceMovementRound=nil
 local minimumActor={modDB=new('ModDB'):ModDB(),output={ActionSpeedMod=action}}
 minimumActor.modDB:NewMod('MovementSpeed','OVERRIDE',0);minimumActor.modDB:NewMod('MovementSpeedCannotBeBelowBase','FLAG',true)
 local minimum=sourceMovement(minimumActor).MovementSpeedMod
 local observed={};local query={}
 for _,method in ipairs({'Override','Sum','More','Flag'})do
  query[method]=function(_,...)local args={...};local name=args[select('#',...)];observed[#observed+1]={method=method,name=name};if method=='Override'then return nil elseif method=='Flag'then return false elseif method=='More'then return 1 else return 0 end end
 end
 sourceMovement({modDB=query,output={ActionSpeedMod=action}})
 local observedText={};for _,q in ipairs(observed)do observedText[#observedText+1]=q.method..':'..tostring(q.name)end;assert(equal(observed,{{method='Override',name='MovementSpeed'},{method='Flag',name='MovementSpeedEqualHighestLinkedPlayers'},{method='Sum',name='MovementSpeed'},{method='Sum',name='MovementSpeed'},{method='More',name='MovementSpeed'},{method='Flag',name='MovementSpeedCannotBeBelowBase'}}),'movement query targets/order changed: '..table.concat(observedText,','))
 local penalty=sourceArmourPenalty(101);assert(#penalty==1,'generated movement penalty count changed');local modifier=penalty[1]
 assert(equal(modifier,modLib.createMod('MovementSpeed','BASE',-101,'Item:1:Source probe',{type='Condition',var='IgnoreMovementPenalties',neg=true})),'generated movement penalty shape changed')
 modifier=copyTable(modifier);modifier.source=nil;local converted=source_convert_actor_modifier(modifier)
 local mapping={stat=converted.stat,effect={kind='numeric',operation=converted.effect.operation,value={kind='capture',index=0,multiplier=converted.effect.value/101}},flags=converted.flags,keyword_flags=converted.keyword_flags,tags=converted.tags}
 return {base_multiplier=base,minimum_multiplier=minimum,default_action_speed_multiplier=action,rounding_precision=precision,query_stats={actor_name(observed[1].name)},penalty_modifier=mapping}
end
