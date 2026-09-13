-- Test-only observation of complete original ItemsTab:Load activations.
-- Loaded before source initialization. No source method or iterator replacement.
local globals, debug_lib, jit_lib = _G, debug, jit
local getinfo,getlocal,gethook,sethook,getmetatable=debug.getinfo,debug.getlocal,debug.gethook,debug.sethook,debug.getmetatable
local status,off,on,flush=jit.status,jit.off,jit.on,jit.flush
local rawget,rawset,next,type,rawequal,error,pcall,ipairs=rawget,rawset,next,type,rawequal,error,pcall,ipairs
local huge=math.huge
local MAX_EVENTS,MAX_OBJECTS,MAX_DEPTH,MAX_STATES=131072,16384,128,256
local MAX_ROWS,MAX_TEXT,MAX_STRING=2000000,64*1024*1024,262144
local function check(ok,message) if not ok then error('item-set lifecycle: '..message,0) end end
local function primitives()
    check(rawequal(rawget(globals,'debug'),debug_lib) and rawequal(rawget(debug_lib,'getinfo'),getinfo) and
        rawequal(rawget(debug_lib,'getlocal'),getlocal) and rawequal(rawget(debug_lib,'gethook'),gethook) and
        rawequal(rawget(debug_lib,'sethook'),sethook) and rawequal(rawget(debug_lib,'getmetatable'),getmetatable),'debug identity changed')
    check(rawequal(rawget(globals,'jit'),jit_lib) and rawequal(rawget(jit_lib,'status'),status) and
        rawequal(rawget(jit_lib,'off'),off) and rawequal(rawget(jit_lib,'on'),on) and rawequal(rawget(jit_lib,'flush'),flush),'JIT identity changed')
end
local function start(observed)
    check(type(observed)=='boolean','observation flag');primitives();check(gethook()==nil,'pre-existing hook')
    local common,build=rawget(globals,'common'),rawget(globals,'build')
    check(type(common)=='table' and type(build)=='table','initialized source owners')
    local classes=rawget(common,'classes');check(type(classes)=='table','classes')
    local items,slot,dropdown,undo=rawget(classes,'ItemsTab'),rawget(classes,'ItemSlotControl'),rawget(classes,'DropDownControl'),rawget(classes,'UndoHandler')
    local import=rawget(classes,'ImportTab')
    local bindings={
        {owner=items,key='Load',name='items_load',parameters={'self','xml','dbFileName'}},
        {owner=items,key='CreateItemSet',name='create_set',parameters={'self','itemSetId','name'}},
        {owner=items,key='SetActiveItemSet',name='activate_set',parameters={'self','itemSetId','deferSync'}},
        {owner=items,key='PopulateSlots',name='populate_slots',parameters={'self'}},
        {owner=items,key='IsItemValidForSlot',name='validity',parameters={'self','item','slotName','itemSet','flagState'}},
        {owner=slot,key='Populate',name='populate_slot',parameters={'self'}},
        {owner=slot,key='SetSelItemId',name='set_slot',parameters={'self','selItemId'}},
        {owner=dropdown,key='SelByValue',name='select_value',parameters={'self','value','key'}},
        {owner=dropdown,key='SetSel',name='select_index',parameters={'self','newSel','noCallSelFunc'}},
        {owner=undo,key='ResetUndo',name='reset_undo',parameters={'self'}},
        {owner=items,key='CreateUndoState',name='create_undo',parameters={'self'}},
        {owner=build,key='SyncLoadouts',name='sync_loadouts',parameters={'self','skipBuildPlannerSync'}},
        {owner=build,key='SetActiveLoadout',name='activate_loadout',parameters={'self','loadout'}},
        {owner=import,key='RefreshBuildPlannerSets',name='refresh_export_sets',parameters={'self'}},
    }
    local rows,bytes=0,0
    local function charge(n) check(n>=0 and bytes+n<=MAX_TEXT,'aggregate text bound');bytes=bytes+n end
    local function row() rows=rows+1;check(rows<=MAX_ROWS,'aggregate row bound') end
    local function finite(v)
        local kind=type(v)
        check(kind=='nil' or kind=='boolean' or kind=='string' or (kind=='number' and v==v and v>-huge and v<huge),'finite scalar')
        if kind=='string' then check(#v<=MAX_STRING,'string bound');charge(#v) end
        return v
    end
    local function scalar(v) return {kind=type(v),value=finite(v)} end
    local objects,ids={},{}
    local function token(v)
        check(type(v)=='table','object token type')
        if ids[v] then return ids[v] end
        check(#objects<MAX_OBJECTS,'retained object bound');local id=#objects+1;objects[id]=v;ids[v]=id;return id
    end
    local targets,functions={},{}
    for i,b in ipairs(bindings) do
        check(type(b.owner)=='table','original owner '..b.name)
        b.fn=rawget(b.owner,b.key);check(type(b.fn)=='function' and targets[b.fn]==nil,'distinct original function '..b.name)
        local info=getinfo(b.fn,'S');check(info.what=='Lua','original Lua function')
        b.token=i;targets[b.fn]=b
        functions[i]={token=i,name=b.name,source=finite(info.source),first_line=info.linedefined,last_line=info.lastlinedefined}
    end
    local function verify()
        primitives()
        check(rawequal(rawget(globals,'common'),common) and rawequal(rawget(common,'classes'),classes) and
            rawequal(rawget(globals,'build'),build),'original owners changed')
        check(rawequal(rawget(classes,'ItemsTab'),items) and rawequal(rawget(classes,'ItemSlotControl'),slot) and
            rawequal(rawget(classes,'DropDownControl'),dropdown) and rawequal(rawget(classes,'UndoHandler'),undo) and rawequal(rawget(classes,'ImportTab'),import),'original classes changed')
        for _,b in ipairs(bindings) do check(rawequal(rawget(b.owner,b.key),b.fn),'original function changed '..b.name) end
    end
    local function context(tab)
        check(type(tab)=='table' and rawequal(rawget(tab,'build'),build),'actual ItemsTab build context')
        local calcs=rawget(build,'calcsTab')
        check(calcs==nil or type(calcs)=='table','calcsTab type')
        local env=calcs and rawget(calcs,'mainEnv')
        check(env==nil or type(env)=='table','mainEnv type')
        return {calcs_tab_present=calcs~=nil,main_env_present=env~=nil,
            calcs_tab_token=calcs and token(calcs) or nil,main_env_token=env and token(env) or nil}
    end
    -- Project a fixed finite consumer contract, preserving aliases between its
    -- retained values. It is not a serialization of entire Item/UI instances.
    local function state(tab,prior)
        check(type(tab)=='table' and rawequal(rawget(tab,'build'),build),'snapshot receiver')
        local memo,count={},0
        local function allocate(v)
            count=count+1;check(count<=32768,'snapshot table bound');local out={};if v then memo[v]=out end;return out
        end
        local clone
        clone=function(v,depth)
            if type(v)~='table' then return finite(v) end
            check(depth<=64,'snapshot depth bound');if memo[v] then return memo[v] end
            check(getmetatable(v)==nil,'retained finite value metatable')
            local out=allocate(v)
            for k,x in next,v do row();check(type(k)=='string' or type(k)=='number','finite key');rawset(out,finite(k),clone(x,depth+1)) end
            return out
        end
        local function fields(v,names)
            if v==nil then return nil end
            check(type(v)=='table','projected receiver type');if memo[v] then return memo[v] end
            local out=allocate(v)
            for _,name in ipairs(names) do row();rawset(out,name,clone(rawget(v,name),0)) end
            return out
        end
        local function item_map(map)
            check(type(map)=='table' and getmetatable(map)==nil,'inventory map')
            if memo[map] then return memo[map] end
            local out=allocate(map)
            for id,item in next,map do row();out[finite(id)]=fields(item,{'id','name','baseName','type','rarity','jewelSocketCount'}) end
            return out
        end
        local slot_seen={}
        local slot_state
        slot_state=function(v,depth)
            check(depth<=64,'slot projection depth bound')
            if slot_seen[v] then return memo[v] end
            slot_seen[v]=true
            local out=fields(v,{'slotName','nodeId','selItemId','selIndex','active','inactive','note','items','list'})
            local controls=rawget(v,'controls');check(type(controls)=='table','slot controls')
            local activate=rawget(controls,'activate')
            out.activate=fields(activate,{'state'})
            local jewels=rawget(v,'jewelSocketList');check(type(jewels)=='table','child socket list')
            out.jewelSocketList={}
            for i,child in next,jewels do row();out.jewelSocketList[finite(i)]=slot_state(child,depth+1) end
            return out
        end
        local out=allocate()
        for _,key in ipairs({'activeItemSetId','showStatDifferences','modFlag','itemOrderList','itemSetOrderList','itemSets','activeItemSet'}) do row();out[key]=clone(rawget(tab,key),0) end
        out.previousActiveItemSet=clone(prior,0)
        out.items=item_map(rawget(tab,'items'))
        out.slots={};local slots=rawget(tab,'slots');check(type(slots)=='table' and getmetatable(slots)==nil,'slot map')
        for name,v in next,slots do row();out.slots[finite(name)]=slot_state(v,0) end
        out.runeSlots={};local runes=rawget(tab,'runeSlots');check(type(runes)=='table' and getmetatable(runes)==nil,'rune map')
        for name,v in next,runes do
            row();local projected=fields(v,{'selIndex'});out.runeSlots[finite(name)]=projected
            local list=rawget(v,'list');check(type(list)=='table' and getmetatable(list)==nil,'rune dropdown list');projected.list={}
            for index,value in next,list do row();projected.list[finite(index)]=type(value)=='table' and fields(value,{'name'}) or finite(value) end
        end
        local trade=rawget(tab,'tradeQuery');check(type(trade)=='table','trade owner')
        local selections=rawget(trade,'statSortSelectionList');check(selections==nil or type(selections)=='table','trade rows')
        if selections~=nil then out.trade={} end
        for i,value in next,selections or {} do
            row();local v=fields(value,{'label','stat','weightMult'});out.trade[finite(i)]=v
            local transform=rawget(value,'transform');check(transform==nil or type(transform)=='function','trade transform kind')
            v.transform_present=transform~=nil
        end
        out.undo={};out.redo={}
        for _,key in ipairs({'undo','redo'}) do
            local buffer=rawget(tab,key);check(type(buffer)=='table','undo buffer')
            for i,value in next,buffer do
                row();local v=fields(value,{'activeItemSetId','itemOrderList','slotSelItemId','itemSets','itemSetOrderList'});out[key][finite(i)]=v
                v.items=item_map(rawget(value,'items'))
            end
        end
        local controls=rawget(build,'controls');local loadouts=type(controls)=='table' and rawget(controls,'buildLoadouts')
        out.loadoutDropdown=fields(loadouts,{'selIndex','list'})
        out.activeLoadout=finite(rawget(build,'activeLoadout'))
        out.buildFlag=finite(rawget(build,'buildFlag'))
        local importer=rawget(build,'importTab')
        if importer~=nil then
            check(type(importer)=='table' and rawequal(rawget(importer,'build'),build),'export owner')
            out.export=fields(importer,{'exportSpecIndex','exportSkillSetId','exportItemSetId'})
            local export_controls=rawget(importer,'controls');check(type(export_controls)=='table','export controls')
            for _,key in ipairs({'buildPlannerSpec','buildPlannerSkillSet','buildPlannerItemSet'}) do
                out.export[key]=fields(rawget(export_controls,key),{'selIndex','list'})
            end
            out.export.path=fields(rawget(export_controls,'poe2ExportPath'),{'buf'})
        end
        out.selected={}
        for _,entry in ipairs({{'treeTab','activeSpec'},{'skillsTab','activeSkillSetId'},{'configTab','activeConfigSetId'}}) do
            local owner=rawget(build,entry[1]);if type(owner)=='table' then out.selected[entry[1]]=finite(rawget(owner,entry[2])) end
        end
        return out
    end
    local events,states,active={}, {}, {}
    local failed,finished,hook=nil,false,nil
    local capture={}
    local function parameter(frame,index,wanted)
        local name,value=getlocal(frame+1,index)
        check(name==wanted,'parameter '..wanted..' got '..(name or 'nil'));return value
    end
    local function snapshot(entry,phase,ordinal)
        check(#states<MAX_STATES,'state count bound')
        states[#states+1]={event_ordinal=ordinal,call_ordinal=entry.ordinal,phase=phase,name=entry.binding.name,
            receiver_token=token(entry.receiver),prior_set_token=entry.prior and token(entry.prior) or nil,
            value=state(entry.receiver,entry.prior)}
    end
    local function slot_summary(receiver)
        return {selected=scalar(rawget(receiver,'selItemId')),active=scalar(rawget(receiver,'active')),
            inactive=scalar(rawget(receiver,'inactive')),note=scalar(rawget(receiver,'note')),index=scalar(rawget(receiver,'selIndex'))}
    end
    local function collect(event,b)
        local frame
        for level=2,MAX_DEPTH do local info=getinfo(level,'f');if not info then break end;if rawequal(info.func,b.fn) then frame=level;break end end
        check(frame~=nil,'exact hooked frame');check(#events<MAX_EVENTS,'event count bound')
        local record={ordinal=#events+1,event=event,name=b.name,function_token=b.token}
        if event=='call' then
            check(#active<MAX_DEPTH,'active call bound')
            local values={};for i,wanted in ipairs(b.parameters) do values[i]=parameter(frame,i,wanted) end
            local receiver=values[1];check(type(receiver)=='table','actual receiver')
            local entry={binding=b,ordinal=record.ordinal,receiver=receiver}
            record.receiver_token=token(receiver)
            if #active>0 then record.parent_call_ordinal=active[#active].ordinal;entry.load=active[#active].load end
            if b.name=='items_load' then
                check(rawequal(receiver,rawget(build,'itemsTab')),'actual loaded itemsTab')
                check(type(values[2])=='table' and rawget(values[2],'elem')=='Items','actual Items XML')
                entry.load=entry;entry.xml=values[2];entry.prior=rawget(receiver,'activeItemSet')
                record.xml_token=token(values[2]);record.context=context(receiver)
                local attr=rawget(values[2],'attrib');check(type(attr)=='table','Items attributes');record.requested_set=scalar(rawget(attr,'activeItemSet'))
                snapshot(entry,'before',record.ordinal)
            else check(entry.load~=nil,'nested call outside complete Load') end
            record.load_call_ordinal=entry.load.ordinal
            if b.name=='activate_set' then
                check(rawequal(receiver,entry.load.receiver),'activation receiver');entry.prior=rawget(receiver,'activeItemSet')
                record.requested_set=scalar(values[2]);record.defer_sync=scalar(values[3]);snapshot(entry,'before',record.ordinal)
            elseif b.name=='create_set' then record.requested_set=scalar(values[2]);record.title=scalar(values[3])
            elseif b.name=='validity' then
                check(rawequal(receiver,entry.load.receiver),'validity receiver');check(type(values[2])=='table','validity item')
                record.item_token=token(values[2]);record.item_id=scalar(rawget(values[2],'id'));record.slot=finite(values[3]);record.context=context(receiver)
                record.item_set_kind=type(values[4]);record.flag_state_kind=type(values[5])
            elseif b.name=='populate_slot' or b.name=='set_slot' then
                check(rawequal(rawget(receiver,'itemsTab'),entry.load.receiver),'slot receiver owner')
                local name=rawget(receiver,'slotName');check(type(name)=='string','actual slot name')
                local map=rawget(entry.load.receiver,'slots');check(type(map)=='table' and rawequal(rawget(map,name),receiver),'actual slot map identity')
                record.slot=finite(name);record.slot_state=slot_summary(receiver)
                if b.name=='populate_slot' then
                    local caller=getinfo(frame+1,'f');record.direct_populate_slots_caller=caller and rawequal(caller.func,bindings[4].fn) or false
                    if record.direct_populate_slots_caller then check(active[#active] and active[#active].binding.name=='populate_slots','actual traversal parent') end
                else record.requested_item=scalar(values[2]) end
            elseif b.name=='select_value' then record.value=scalar(values[2]);record.key=scalar(values[3])
            elseif b.name=='select_index' then record.index=scalar(values[2]);record.no_callback=scalar(values[3])
            elseif b.name=='sync_loadouts' then record.skip_build_planner_sync=scalar(values[2])
            elseif b.name=='activate_loadout' then record.loadout_kind=type(values[2])
            end
            active[#active+1]=entry
        elseif event=='return' then
            local entry=active[#active];check(entry and rawequal(entry.binding.fn,b.fn),'complete call return pairing')
            record.call_ordinal=entry.ordinal;record.load_call_ordinal=entry.load.ordinal;record.receiver_token=token(entry.receiver)
            if b.name=='items_load' or b.name=='activate_set' then snapshot(entry,'after',record.ordinal)
            elseif b.name=='populate_slot' or b.name=='set_slot' then record.slot=finite(rawget(entry.receiver,'slotName'));record.slot_state=slot_summary(entry.receiver) end
            active[#active]=nil
        else error('item-set lifecycle: unknown hook event',0) end
        events[#events+1]=record
    end
    hook=function(event)
        if event~='call' and event~='return' then return end
        local info=getinfo(2,'f');local b=info and targets[info.func]
        if b and (b.name=='items_load' or #active>0) then
            if failed then error(failed,0) end
            local ok,message=pcall(collect,event,b);if not ok then failed=message;error(message,0) end
        end
    end
    local prior_jit=status();off();flush()
    if observed then sethook(hook,'cr') end
    function capture.finish()
        if finished then if failed then error(failed,0) end;return capture.report end
        finished=true;local existing=gethook()
        if observed then if rawequal(existing,hook) then sethook() else failed=failed or 'item-set lifecycle: hook tampered' end
        elseif existing~=nil then failed=failed or 'item-set lifecycle: unexpected control hook' end
        local ok,message=pcall(function()
            verify();local incomplete={};for i,entry in ipairs(active) do incomplete[i]={name=entry.binding.name,call_ordinal=entry.ordinal} end
            capture.report={events=events,states=states,functions=functions,incomplete_calls=incomplete,
                finite_post_import=state(rawget(build,'itemsTab'),nil),
                scope={observed=observed,complete_original_load_boundary=true,whole_original_object_graph=false,
                    source_methods_unchanged=true,original_iterators_unchanged=true,jit_disabled=true,warm_claim=false,
                    parameter_projection_not_actual_arity=true,validity_return_values_not_observed=true,
                    item_projection_fields={'id','name','baseName','type','rarity','jewelSocketCount'},
                    rune_definition_projection_fields={'name'},trade_transform_presence_only=true},
                bounds={events=MAX_EVENTS,objects=MAX_OBJECTS,depth=MAX_DEPTH,states=MAX_STATES,rows=MAX_ROWS,text_bytes=MAX_TEXT},
                retained_objects=#objects,rows=rows,text_bytes=bytes}
        end)
        if prior_jit then on() end
        if not ok then failed=failed or message end
        if failed then error(failed,0) end
        return capture.report
    end
    return capture
end
return {start=start}
