-- Test-only exact-function lifecycle observation. Load before source initialization.
-- No original function replacement. Hooked calls are interpreted, never warm evidence.
local globals, debug_lib, jit_lib = _G, debug, jit
local getinfo, getlocal, gethook, sethook = debug.getinfo, debug.getlocal, debug.gethook, debug.sethook
local status, off, on, flush = jit.status, jit.off, jit.on, jit.flush
local rawget, rawequal, next, type, error, pcall = rawget, rawequal, next, type, error, pcall
local huge, ipairs = math.huge, ipairs
local MAX_EVENTS, MAX_TEXT, MAX_STRING = 8192, 16 * 1024 * 1024, 262144
local MAX_OBJECTS, MAX_DEPTH, MAX_LOCALS = 4096, 128, 256
local function check(ok, message) if not ok then error('jewel lifecycle: ' .. message, 0) end end
local function primitives()
    check(rawequal(rawget(globals,'debug'),debug_lib) and rawequal(rawget(debug_lib,'getinfo'),getinfo) and
        rawequal(rawget(debug_lib,'getlocal'),getlocal) and rawequal(rawget(debug_lib,'gethook'),gethook) and
        rawequal(rawget(debug_lib,'sethook'),sethook),'debug identity changed')
    check(rawequal(rawget(globals,'jit'),jit_lib) and rawequal(rawget(jit_lib,'status'),status) and
        rawequal(rawget(jit_lib,'off'),off) and rawequal(rawget(jit_lib,'on'),on) and
        rawequal(rawget(jit_lib,'flush'),flush),'JIT identity changed')
end
local function start(observed)
    if observed == nil then observed = true end
    check(type(observed)=='boolean','observation flag')
    primitives(); check(gethook()==nil,'pre-existing hook')
    local data, main = rawget(globals,'data'), rawget(globals,'main')
    local common = rawget(globals,'common')
    check(type(data)=='table' and type(main)=='table' and type(common)=='table','source initialization')
    local classes=rawget(common,'classes')
    local item, items, tree = rawget(classes,'Item'),rawget(classes,'ItemsTab'),rawget(classes,'TreeTab')
    local bindings={
        { owner=data, key='setJewelRadiiGlobally', name='radius_set', parameters={'treeVersion'} },
        { owner=main, key='LoadTree', name='load_tree', parameters={'self','treeVersion'} },
        { owner=items, key='Load', name='items_load', parameters={'self','xml'} },
        { owner=item, key='ParseRaw', name='parse_raw', parameters={'self','raw'} },
        { owner=tree, key='Load', name='tree_load', parameters={'self','xml'} },
        { owner=tree, key='SetActiveSpec', name='select_spec', parameters={'self','specId'} },
    }
    local targets, functions={},{}
    local bytes=0
    local function charge(n) check(n>=0 and bytes+n<=MAX_TEXT,'aggregate text bound');bytes=bytes+n end
    local function scalar(value)
        local kind=type(value)
        check(kind=='nil' or kind=='boolean' or kind=='string' or (kind=='number' and value==value and value>-huge and value<huge),'scalar representation')
        if kind=='string' then check(#value<=MAX_STRING,'scalar byte bound');charge(#value) end
        return {kind=kind,value=value}
    end
    for i,b in ipairs(bindings) do
        check(type(b.owner)=='table','method owner missing')
        b.fn=rawget(b.owner,b.key)
        check(type(b.fn)=='function' and not targets[b.fn],'distinct exact original functions')
        local info=getinfo(b.fn,'S'); check(info.what=='Lua','original Lua declaration')
        local source=info.source or '';check(#source<=4096,'declaration path bound');charge(#source)
        b.token=i;targets[b.fn]=b
        functions[i]={token=i,name=b.name,source=source,first_line=info.linedefined,last_line=info.lastlinedefined}
    end
    local objects, ids={},{}
    local function token(value)
        check(type(value)=='table','object identity type')
        if ids[value] then return ids[value] end
        check(#objects<MAX_OBJECTS,'retained object bound')
        local id=#objects+1;objects[id]=value;ids[value]=id;return id
    end
    local versions=rawget(data,'jewelRadii');check(type(versions)=='table','radius definitions')
    local configured_latest=scalar(rawget(globals,'latestTreeVersion'))
    check(configured_latest.kind=='string','configured startup tree version')
    local function context()
        local current=rawget(data,'jewelRadius')
        local out={kind=type(current),max_radius=scalar(rawget(data,'maxJewelRadius')),version_keys={}}
        if current==nil then return out end
        check(type(current)=='table','current radius type');out.table_token=token(current)
        local count=0
        for version,definition in next,versions do
            count=count+1;check(count<=128,'radius version inventory bound')
            check(type(version)=='string' and type(definition)=='table','radius version shape')
            if rawequal(current,definition) then out.version_keys[#out.version_keys+1]=scalar(version) end
        end
        return out
    end
    local function observable()
        local build=rawget(globals,'build');check(type(build)=='table','final build owner')
        local spec=rawget(build,'spec');local tree_tab=rawget(build,'treeTab');local items_tab=rawget(build,'itemsTab')
        check(type(spec)=='table' and type(tree_tab)=='table' and type(items_tab)=='table','final build components')
        local inventory=rawget(items_tab,'items');check(type(inventory)=='table','final inventory')
        local out={tree_version=scalar(rawget(spec,'treeVersion')),active_spec=scalar(rawget(tree_tab,'activeSpec')),items={},radii={},item_order={}}
        local count=0
        for id,item in next,inventory do
            count=count+1;check(count<=MAX_OBJECTS and type(item)=='table','final inventory bound/type')
            out.items[count]={id=scalar(id),name=scalar(rawget(item,'name')),radius_label=scalar(rawget(item,'jewelRadiusLabel')),radius_index=scalar(rawget(item,'jewelRadiusIndex'))}
        end
        local order=rawget(items_tab,'itemOrderList');check(type(order)=='table','final item order')
        local ended=false
        for i=1,MAX_OBJECTS+1 do local id=rawget(order,i);if id==nil then ended=true;break end
            check(i<=MAX_OBJECTS,'item order bound');out.item_order[i]=scalar(id)
        end
        check(ended,'item order termination')
        local radii=rawget(data,'jewelRadius')
        if radii~=nil then
            check(type(radii)=='table','final radius shape');ended=false
            for i=1,129 do local radius=rawget(radii,i);if radius==nil then ended=true;break end
                check(i<=128 and type(radius)=='table','final radius row bound')
                local row={};for _,key in ipairs({'label','inner','outer','innerSquared','outerSquared'}) do row[key]=scalar(rawget(radius,key)) end
                out.radii[i]=row
            end
            check(ended,'radius row termination')
        end
        out.max_radius=scalar(rawget(data,'maxJewelRadius'))
        return out
    end
    local events,active={},{}
    local failed,finished,hook=nil,false,nil
    local capture={}
    local function declaration(level,slot,expected)
        local name,value=getlocal(level+1,slot)
        check(name==expected,'parameter declaration '..expected..' got '..(name or 'nil'))
        return value
    end
    local function node(frame)
        for i=1,MAX_LOCALS do
            local name,value=getlocal(frame+1,i)
            if not name then return nil end
            if name=='node' and type(value)=='table' and rawget(value,'elem')=='Item' then return value end
        end
        error('jewel lifecycle: local inventory bound',0)
    end
    local function collect(event,b)
        local frame
        for level=2,MAX_DEPTH do local info=getinfo(level,'f');if not info then break end
            if rawequal(info.func,b.fn) then frame=level;break end
        end
        check(frame~=nil,'exact hooked frame missing')
        check(#events<MAX_EVENTS,'event count bound')
        local row={ordinal=#events+1,event=event,function_token=b.token,name=b.name,context=context()}
        if event=='call' then
            check(#active<MAX_DEPTH,'active call bound')
            local entry={binding=b,call_ordinal=row.ordinal}
            local values={}
            for i,wanted in ipairs(b.parameters) do values[i]=declaration(frame,i,wanted) end
            if b.parameters[1]=='self' then
                check(type(values[1])=='table','receiver type');entry.receiver=values[1];row.receiver_token=token(values[1])
            end
            if b.name=='radius_set' then row.requested_version=scalar(values[1])
            elseif b.name=='load_tree' then row.requested_version=scalar(values[2])
            elseif b.name=='select_spec' then row.requested_spec=scalar(values[2])
            elseif b.name=='items_load' or b.name=='tree_load' then
                check(type(values[2])=='table','XML parameter type');entry.xml=values[2];row.xml_token=token(values[2]);row.xml_element=scalar(rawget(values[2],'elem'))
            elseif b.name=='parse_raw' then
                row.raw=scalar(values[2]);row.item_id=scalar(rawget(values[1],'id'))
                for level=frame+1,MAX_DEPTH do
                    local info=getinfo(level,'f');if not info then break end
                    if rawequal(info.func,bindings[3].fn) then
                        local receiver=declaration(level,1,'self')
                        local xml=declaration(level,2,'xml')
                        row.items_receiver_token=token(receiver);row.items_xml_token=token(xml)
                        local current=node(level)
                        if current then row.item_xml_token=token(current);local attr=rawget(current,'attrib');check(type(attr)=='table','Item XML attributes');row.xml_item_id=scalar(rawget(attr,'id')) end
                        break
                    end
                end
            end
            if #active>0 then row.parent_call_ordinal=active[#active].call_ordinal end
            active[#active+1]=entry
        elseif event=='return' then
            local entry=active[#active]
            check(entry and rawequal(entry.binding.fn,b.fn),'exact active return pairing')
            row.call_ordinal=entry.call_ordinal
            if entry.receiver then row.receiver_token=token(entry.receiver) end
            if entry.xml then row.xml_token=token(entry.xml) end
            if b.name=='parse_raw' then row.item_id=scalar(rawget(entry.receiver,'id'));row.radius_label=scalar(rawget(entry.receiver,'jewelRadiusLabel'));row.radius_index=scalar(rawget(entry.receiver,'jewelRadiusIndex')) end
            active[#active]=nil
        else error('jewel lifecycle: unmodeled target hook event',0) end
        events[#events+1]=row
    end
    hook=function(event)
        if event~='call' and event~='return' then return end
        local info=getinfo(2,'f');local binding=info and targets[info.func]
        if binding then
            if failed then error(failed,0) end
            local ok,message=pcall(collect,event,binding)
            if not ok then failed=message;error(message,0) end
        end
    end
    local prior_jit=status();off();flush()
    local initial=context()
    if observed then sethook(hook,'cr') end
    function capture.finish()
        if finished then if failed then error(failed,0) end;return capture.report end
        finished=true
        local existing=gethook()
        if observed then
            if rawequal(existing,hook) then sethook() else failed=failed or 'jewel lifecycle: hook tampered' end
        elseif existing~=nil then failed=failed or 'jewel lifecycle: unexpected control hook' end
        local ok,message=pcall(function()
            primitives();check(rawequal(rawget(globals,'data'),data) and rawequal(rawget(globals,'main'),main),'global owner changed')
            check(rawequal(rawget(data,'jewelRadii'),versions),'radius definition owner changed')
            check(rawequal(rawget(globals,'common'),common) and rawequal(rawget(common,'classes'),classes) and rawequal(rawget(classes,'Item'),item) and rawequal(rawget(classes,'ItemsTab'),items) and rawequal(rawget(classes,'TreeTab'),tree),'class identity changed')
            for _,b in ipairs(bindings) do check(rawequal(rawget(b.owner,b.key),b.fn),'original function changed '..b.name) end
            local incomplete={};for i,entry in ipairs(active) do incomplete[i]={name=entry.binding.name,call_ordinal=entry.call_ordinal} end
            capture.report={events=events,functions=functions,configured_latest_tree_version=configured_latest,finite_post_import=observable(),initial_context=initial,final_context=context(),incomplete_calls=incomplete,
                scope={original_functions_unchanged=true,observed=observed,jit_disabled_during_import=true,warm_claim=false,
                    parameter_projection_not_actual_arity=true,table_tokens_are_retained_live_identity=true},bounds={events=MAX_EVENTS,text=MAX_TEXT,objects=MAX_OBJECTS,depth=MAX_DEPTH},text_bytes=bytes}
        end)
        if prior_jit then on() end
        if not ok then failed=failed or message end
        if failed then error(failed,0) end
        return capture.report
    end
    return capture
end
return {start=start}
