-- Read-only complete Player attribute execution. No source methods are replaced.
local M = {}
local originals, methods, frames, active, hooked, finished
local attrs = {"Str", "Dex", "Int"}
local comparisons = {"TwoHighestAttributesEqual", "DexHigherThanInt", "StrHigherThanInt",
    "IntHigherThanDex", "StrHigherThanDex", "IntHigherThanStr", "DexHigherThanStr",
    "StrHighestAttribute", "IntHighestAttribute", "DexHighestAttribute",
    "IntSingleHighestAttribute", "DexSingleHighestAttribute"}
local function original(f, path, first)
    assert(type(f)=="function", "missing original "..path)
    local i=debug.getinfo(f,"S")
    assert(i.what=="Lua" and i.source:gsub("\\","/"):sub(-#path)==path)
    assert(i.linedefined==first,"unexpected original line "..path..":"..i.linedefined)
    return {path=path,first=i.linedefined,last=i.lastlinedefined}
end
local function upvalue(f,wanted)
    for i=1,128 do local n,v=debug.getupvalue(f,i);if not n then break end
        if n==wanted then return v end end
    error("missing original upvalue "..wanted)
end
local function equal(a,b)
    if type(a)~=type(b) then return false end
    if type(a)~="table" then return a==b end
    for k,v in pairs(a) do if not equal(v,b[k]) then return false end end
    for k in pairs(b) do if a[k]==nil then return false end end
    return true
end
-- Strict graph for relevant source records/conditions only. Unknown value/key
-- types fail here; this does not traverse arbitrary Player diagnostic objects.
local function graph(value)
    local seen,tables,entries={},{},0
    local encode
    encode=function(v)
        local t=type(v)
        if t=="nil" then return {kind="nil"} end
        if t=="number" then return {kind="number",value=(v~=v or v==math.huge or v==-math.huge) and tostring(v) or v} end
        if t=="string" or t=="boolean" then return {kind=t,value=v} end
        assert(t=="table","unsupported relevant source graph value "..t)
        if seen[v] then return {kind="table",id=seen[v]} end
        local id=#tables+1;assert(id<=32768,"source graph table bound");seen[v]=id
        local row={id=id,entries={}};tables[id]=row
        local keys={};for k in next,v do
            assert(type(k)=="string" or type(k)=="number" or type(k)=="boolean","unsupported relevant source graph key")
            keys[#keys+1]=k
        end
        table.sort(keys,function(a,b)
            if type(a)~=type(b) then return type(a)<type(b) end
            if type(a)=="boolean" then return a==false and b==true end
            return a<b
        end)
        for _,k in ipairs(keys) do entries=entries+1;assert(entries<=262144,"source graph entry bound")
            row.entries[#row.entries+1]={key=encode(k),value=encode(rawget(v,k))}
        end
        return {kind="table",id=id}
    end
    return {root=encode(value),tables=tables}
end
-- Compare every scalar diagnostic field, and inventory (do not descend into)
-- every non-scalar field. Raw top-level identities stay private to this runtime.
local function output_snapshot(output, expected_identity)
    assert(type(output)=="table")
    local scalars,excluded,identity,count={},{},{object=output,fields={}},0
    for k,v in next,output do
        assert(type(k)=="string","unknown top-level Player output key")
        count=count+1;assert(count<=16384,"Player output field bound")
        local t=type(v);identity.fields[k]=v
        if t=="number" or t=="string" or t=="boolean" then
            local value=v
            if t=="number" then
                if v~=v then value="nan" elseif v==math.huge then value="positive_infinity" elseif v==-math.huge then value="negative_infinity" end
            end
            scalars[k]={kind=t,value=value}
        else excluded[#excluded+1]={key=k,value_type=t} end
    end
    table.sort(excluded,function(a,b)return a.key<b.key end)
    if expected_identity then
        assert(expected_identity.object==output,"Player output object changed")
        for k,v in pairs(expected_identity.fields) do
            assert(rawget(output,k)~=nil,"Player output field disappeared")
            if type(v)~="number" and type(v)~="string" and type(v)~="boolean" then
                assert(rawequal(rawget(output,k),v),"non-scalar Player output identity changed")
            end
        end
        for k in next,output do assert(expected_identity.fields[k]~=nil,"Player output field appeared") end
    end
    return {scalar_fields=scalars,excluded_fields=excluded,field_count=count},identity
end
local function record(m)
    return {name=m.name,type=m.type,value=graph(m.value),source=m.source,
        flags=m.flags,keyword_flags=m.keywordFlags,tag_count=#m,full=graph(m)}
end
local function attribute_values(output)
    local out={};for _,name in ipairs(attrs) do
        out[name]={present=rawget(output,name)~=nil,value=rawget(output,name)}
    end
    out.LowestAttribute={present=rawget(output,"LowestAttribute")~=nil,value=rawget(output,"LowestAttribute")}
    out.TotalAttr={present=rawget(output,"TotalAttr")~=nil,value=rawget(output,"TotalAttr")}
    return out
end
local function chain(db)
    local rows,seen={},{}
    while db do
        assert(type(db)=="table" and db.mods and not seen[db] and #rows<32)
        seen[db]=true
        local row={depth=#rows,attributes={},condition_records={},conditions=graph(db.conditions),multipliers=graph(db.multipliers)}
        for _,name in ipairs(attrs) do
            local list={};for i,m in ipairs(db.mods[name] or {}) do
                assert(i<=16384);list[#list+1]={index=i,record=record(m)}
            end
            row.attributes[name]=list
        end
        for name,list in pairs(db.mods) do if name:sub(1,10)=="Condition:" then
            local records={};for i,m in ipairs(list) do assert(i<=16384);records[i]={index=i,record=record(m)} end
            row.condition_records[name]=records
        end end
        local parent=rawget(db,"parent")
        row.parent_kind=parent==nil and "absent" or parent==false and "false" or "store"
        rows[#rows+1]=row;db=parent
    end
    return rows
end
local function local_conditions(db)
    local out={};for _,name in ipairs(comparisons) do
        local v=rawget(db.conditions,name);out[name]={present=v~=nil,value=v}
    end
    return out
end
local function declarations()
    return {{"perform",require("Modules.CalcBase"),"perform","Modules/CalcPerform.lua",1193},
        {"val",calcLib,"val","Modules/CalcTools.lua",50},
        {"sum",common.classes.ModStore,"Sum","Classes/ModStore.lua",202},
        {"more",common.classes.ModStore,"More","Classes/ModStore.lua",261},
        {"sum_internal",common.classes.ModDB,"SumInternal","Classes/ModDB.lua",137},
        {"more_internal",common.classes.ModDB,"MoreInternal","Classes/ModDB.lua",214},
        {"get_stat",common.classes.ModStore,"GetStat","Classes/ModStore.lua",428},
        {"get_condition",common.classes.ModStore,"GetCondition","Classes/ModStore.lua",409}}
end
local function unchanged_methods()
    for _,m in ipairs(declarations()) do assert(m[2][m[3]]==originals[m[1]]) end
    assert(upvalue(originals.perform,"doActorAttribsConditions")==originals.actor)
    assert(upvalue(originals.actor,"calculateAttributes")==originals.attributes)
end
local function depth(db,target)
    local n=0;while db do if db==target then return n end;db=db.parent;n=n+1;assert(n<32) end
    return nil
end
local function pack(...) return {n=select("#",...),...} end
function M.begin(enable,expectedJit)
    assert(debug.gethook()==nil and jit.status()==expectedJit)
    originals,methods,frames,active,hooked,finished={},{},{},nil,enable,false
    for _,m in ipairs(declarations()) do originals[m[1]]=m[2][m[3]];methods[m[1]]=original(m[2][m[3]],m[4],m[5]) end
    originals.actor=upvalue(originals.perform,"doActorAttribsConditions")
    originals.attributes=upvalue(originals.actor,"calculateAttributes")
    methods.actor=original(originals.actor,"Modules/CalcPerform.lua",264)
    methods.attributes=original(originals.attributes,"Modules/CalcPerform.lua",233)
    local failure,sequence=nil,0
    local stacks={sum_internal={},more_internal={},get_stat={},get_condition={}}
    local wanted={[originals.attributes]="attributes",[originals.val]="val",
        [originals.sum_internal]="sum_internal",[originals.more_internal]="more_internal",
        [originals.get_stat]="get_stat",[originals.get_condition]="get_condition"}
    local hook
    hook=function(event,line)
        if failure then return end
        local info=debug.getinfo(2,"f");local kind=wanted[info.func]
        if not kind then return end
        if not active and kind~="attributes" then return end
        if event=="line" and (kind~="attributes" or (line~=237 and line~=242)) then return end
        if event~="call" and event~="return" and event~="line" then return end
        local v={};for i=1,160 do local name,value=debug.getlocal(2,i);if not name then break end;v[name]=value end
        local caller=debug.getinfo(3,"f");local caller_env
        if kind=="attributes" and event=="call" then
            for i=1,64 do local name,value=debug.getlocal(3,i);if not name then break end
                if name=="env" then caller_env=value end
            end
        end
        local ok,problem=xpcall(function()
            sequence=sequence+1;assert(sequence<1000000)
            if kind=="attributes" then
                if event=="call" then
                    local db=assert(v.modDB);local actor=assert(db.actor)
                    -- Caller identity/locals are captured before entering xpcall.
                    assert(caller.func==originals.actor and caller_env)
                    if actor~=caller_env.player then return end
                    assert(caller_env.modDB==db)
                    assert(not active,"nested original attribute evaluation")
                    active={db=db,actor=actor,output=v.output,stages={},entry=attribute_values(v.output),entry_chain=chain(db),entry_conditions=local_conditions(db)}
                    debug.sethook(hook,"crl")
                elseif not active or v.modDB~=active.db then return
                elseif event=="line" and line==237 and active.stage and active.stage.returned then
                    local row=active.stage
                    if row.output then assert(equal(row.output,attribute_values(v.output)))
                    else
                        row.output=attribute_values(v.output);row.conditions_after=local_conditions(active.db)
                        assert(type(v.output[row.stat])=="number")
                        row.value=v.output[row.stat];row.after_chain=chain(active.db)
                    end
                elseif event=="line" and line==242 then
                    assert(active.stage and active.stage.output)
                elseif event=="return" then
                    assert(#active.stages==6 and active.stage and active.stage.output)
                    for _,stack in pairs(stacks) do assert(#stack==0,"unclosed source query") end
                    active.exit=attribute_values(v.output);active.exit_conditions=local_conditions(active.db);active.exit_chain=chain(active.db)
                    frames[#frames+1]=active;assert(#frames<256);active=nil;debug.sethook(hook,"crl")
                end
            elseif kind=="val" then
                if caller.func~=originals.attributes then return end
                assert(v.modStore==active.db and v.cfg==nil)
                if event=="call" then
                    assert(not active.stage or active.stage.output)
                    local index=#active.stages+1;assert(index<=6 and v.name==attrs[(index-1)%3+1])
                    local row={index=index,pass=math.floor((index-1)/3)+1,stat=v.name,
                        input=attribute_values(active.output),conditions_before=local_conditions(active.db),before_chain=chain(active.db),queries={},reads={}}
                    active.stages[index]=row;active.stage=row
                else assert(event=="return" and active.stage.stat==v.name);active.stage.returned=true end
            elseif active.stage and not active.stage.returned then
                local stack=stacks[kind]
                if event=="call" then
                    local row={kind=kind,sequence=sequence,store_depth=depth(active.db,v.self),cfg=graph(v.cfg)}
                    local frame={row=row,self=v.self,cfg=v.cfg,stage=active.stage}
                    if kind=="sum_internal" or kind=="more_internal" then
                        row.name=v.modName;row.contribution=kind=="sum_internal" and v.modType or "MORE"
                        row.flags=v.flags;row.keyword_flags=v.keywordFlags;row.source=v.source
                        row.original_context_is_player=v.context==active.db
                        active.stage.queries[#active.stage.queries+1]=row
                    else
                        assert(row.store_depth~=nil,"attribute getter outside captured Player ancestry")
                        frame.name=kind=="get_stat" and v.stat or v.var;frame.no_mod=v.noMod
                        row.name=frame.name;row.no_mod={present=v.noMod~=nil,value=v.noMod}
                        row.before_output,frame.output_identity=output_snapshot(active.output);row.before_chain=chain(active.db)
                        active.stage.reads[#active.stage.reads+1]=row
                    end
                    stack[#stack+1]=frame
                elseif event=="return" then
                    local frame=assert(stack[#stack],"query return without observed call");stack[#stack]=nil
                    assert(frame.self==v.self and frame.stage==active.stage)
                    if kind=="sum_internal" or kind=="more_internal" then
                        assert(type(v.result)=="number","source internal return must expose actual result")
                        frame.row.result=v.result;frame.row.actual_return_local=true
                    else
                        local before_chain=chain(active.db);local before_output,output_identity=output_snapshot(active.output,frame.output_identity)
                        local replay
                        if kind=="get_stat" then replay=pack(originals.get_stat(frame.self,frame.name,frame.cfg))
                        else replay=pack(originals.get_condition(frame.self,frame.name,frame.cfg,frame.no_mod)) end
                        assert(equal(before_chain,chain(active.db)) and equal(before_output,output_snapshot(active.output,output_identity)),"original getter replay mutated observed state")
                        frame.row.same_state_original_replay=graph(replay)
                        frame.row.actual_call_observed=true;frame.row.direct_return_value_claim=false
                        frame.row.replay_inputs_unchanged=equal(frame.row.before_chain,before_chain) and equal(frame.row.before_output,before_output)
                        assert(frame.row.replay_inputs_unchanged,"getter mutated between observed call and replay")
                    end
                end
            end
        end,debug.traceback)
        if not ok then failure=problem end
    end
    if enable then jit.flush();debug.sethook(hook,"crl") end
    return function()
        if enable then assert(debug.gethook()==hook);debug.sethook() end
        assert(not failure,failure);assert(not active and debug.gethook()==nil and jit.status()==expectedJit)
        unchanged_methods();finished=true
    end
end
local function selected()
    return {items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,
        config=build.configTab.activeConfigSetId,group=build.mainSocketGroup,character_level=build.characterLevel}
end
function M.observe(expectedJit)
    assert(finished and debug.gethook()==nil and jit.status()==expectedJit)
    unchanged_methods()
    local saved=selected();local items={};for id,item in pairs(build.itemsTab.items) do items[id]={item=item,raw=item.raw} end
    local modes={}
    for _,mode in ipairs({"MAIN","CALCS"}) do
        local env=assert(mode=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
        local actor=assert(env.player);local db=assert(actor.modDB);assert(env.modDB==db)
        local before=chain(db);local output,output_identity=output_snapshot(actor.output);local chosen
        if hooked then
            for _,frame in ipairs(frames) do if frame.actor==actor and frame.db==db and frame.output==actor.output then chosen=frame end end
            assert(chosen,"missing original Player attribute frame for selected output")
            for _,name in ipairs(attrs) do assert(chosen.exit[name].value==actor.output[name]) end
        end
        local proof
        if chosen then proof={entry=chosen.entry,entry_chain=chosen.entry_chain,entry_conditions=chosen.entry_conditions,
            stages=chosen.stages,exit=chosen.exit,exit_chain=chosen.exit_chain,exit_conditions=chosen.exit_conditions,
            original_actor_caller=true,exact_player_output_table=true} end
        modes[mode]={class_id=env.classId,class_name=env.spec.curClassName,character_level=actor.level,
            attributes=attribute_values(actor.output),conditions=local_conditions(db),read_set=before,player_output=output,provenance=proof}
        assert(equal(before,chain(db)) and equal(output,output_snapshot(actor.output,output_identity)))
    end
    assert(equal(saved,selected()))
    for id,row in pairs(items) do assert(build.itemsTab.items[id]==row.item and row.item.raw==row.raw) end
    unchanged_methods()
    return {methods=methods,selected=saved,hooked=hooked,modes=modes,
        evidence={original_methods_preserved=true,original_complete_attribute_stage=true,
            full_attribute_ancestry=true,scalar_output_comparison=true,cached_scalar_outputs_preserved=true,
            excluded_nonscalar_field_inventory=true,top_level_output_identities_preserved=true,
            full_output_graph=false,nonscalar_output_graph_excluded=true,deep_output_identity_claim=false,
            getter_targets_limited_to_player_ancestry=true,
            saved_items_preserved=true,saved_selection_preserved=true,business_method_wrappers=false,
            native_contributor_closure=false,default_condition_authority=false,more_grouping_policy=false}}
end
return M
