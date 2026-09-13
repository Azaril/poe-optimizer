-- Test-only observation of complete original ItemsTab:Load activations.
-- Loaded before source initialization. No source method or iterator replacement.
local globals, debug_lib, jit_lib = _G, debug, jit
local getinfo,getlocal,gethook,sethook,getmetatable=debug.getinfo,debug.getlocal,debug.gethook,debug.sethook,debug.getmetatable
local status,off,on,flush=jit.status,jit.off,jit.on,jit.flush
local getupvalue=debug.getupvalue
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
local function start(observed,options)
    check(options==nil or (type(options)=='table' and getmetatable(options)==nil),'options')
    local after_populate=options and rawget(options,'after_populate') or false
    local activation_context=options and rawget(options,'activation_context')
    local loadouts=options and rawget(options,'loadouts')
    if loadouts==nil then loadouts=false end
    check(type(loadouts)=='boolean','loadouts option')
    check(type(after_populate)=='boolean','after_populate option')
    check(activation_context==nil or type(activation_context)=='function','activation context observer')
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
    local tree_class,skills_class,config_class,build_class,spec_class
    if loadouts then
        build_class=rawget(classes,'ControlHost');spec_class=rawget(classes,'PassiveSpec')
        tree_class,skills_class,config_class=rawget(classes,'TreeTab'),rawget(classes,'SkillsTab'),rawget(classes,'ConfigTab')
        for _,binding in ipairs({
            {owner=build,key='GetLoadoutByName',name='lookup_loadout',parameters={'self','loadoutName'}},
            {owner=tree_class,key='GetSpecList',name='get_spec_list',parameters={'self'}},
            {owner=tree_class,key='SetActiveSpec',name='activate_spec',parameters={'self','specId','deferSync'}},
            {owner=skills_class,key='SetActiveSkillSet',name='activate_skill_set',parameters={'self','skillSetId','deferSync'}},
            {owner=config_class,key='SetActiveConfigSet',name='activate_config_set',parameters={'self','configSetId','init','deferSync'}},
        }) do bindings[#bindings+1]=binding end
    end
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
        if loadouts then
            check(rawequal(rawget(debug_lib,'getupvalue'),getupvalue),'callback upvalue inspector changed')
            check(rawequal(rawget(classes,'TreeTab'),tree_class) and rawequal(rawget(classes,'SkillsTab'),skills_class) and
                rawequal(rawget(classes,'ConfigTab'),config_class) and rawequal(rawget(classes,'ControlHost'),build_class) and rawequal(rawget(classes,'PassiveSpec'),spec_class),'original loadout classes changed')
        end
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
        if after_populate then
            local spec=rawget(build,'spec')
            check(spec==nil or type(spec)=='table','population spec owner')
            out.nodeJewels=spec and clone(rawget(spec,'jewels'),0) or nil
        end
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
    -- Opt-in loadout state. The identity envelope is observational metadata;
    -- compare the remaining joint graph without rewriting nested aliases.
    local callback_bindings,retained_callbacks={},{}
    local function current_controls()
        local controls=rawget(build,'controls')
        check(controls==nil or type(controls)=='table','build controls owner')
        local importer=rawget(build,'importTab')
        check(importer==nil or (type(importer)=='table' and rawequal(rawget(importer,'build'),build)),'import owner')
        local exports=importer and rawget(importer,'controls')
        check(exports==nil or type(exports)=='table','export controls owner')
        return {
            {name='loadout_selection',control=controls and rawget(controls,'buildLoadouts'),owner=build},
            {name='export_spec_selection',control=exports and rawget(exports,'buildPlannerSpec'),owner=importer},
            {name='export_skill_selection',control=exports and rawget(exports,'buildPlannerSkillSet'),owner=importer},
            {name='export_item_selection',control=exports and rawget(exports,'buildPlannerItemSet'),owner=importer},
        }
    end
    local function verify_callbacks()
        local current=current_controls()
        for i,entry in ipairs(current) do
            local old=callback_bindings[i]
            check(old~=nil and rawequal(old.control,entry.control) and rawequal(old.receiver_owner,entry.owner),
                'loadout callback owner changed within root')
            if entry.control~=nil then
                check(type(entry.control)=='table' and rawequal(rawget(entry.control,'selFunc'),old.fn),
                    'loadout callback changed within root')
                if old.fn~=nil then
                    for at,value in ipairs(old.upvalues) do
                        local name,current_value=getupvalue(old.fn,at)
                        check(name==value.name and rawequal(current_value,value.value),'loadout callback capture changed within root')
                    end
                    check(getupvalue(old.fn,#old.upvalues+1)==nil,'loadout callback capture count changed')
                end
            end
        end
    end
    local function bind_callbacks()
        -- Build:Init legitimately creates new controls between outer roots.
        -- Within one root they must retain exact Function/owner identity.
        for _,old in ipairs(callback_bindings) do
            if old.fn~=nil then targets[old.fn]=nil end
        end
        local next_bindings={}
        for i,entry in ipairs(current_controls()) do
            local control,fn=entry.control,nil
            if control~=nil then
                check(type(control)=='table','loadout control')
                fn=rawget(control,'selFunc');check(fn==nil or type(fn)=='function','loadout callback kind')
            end
            local old=callback_bindings[i]
            local b
            if old and rawequal(old.control,control) and rawequal(old.receiver_owner,entry.owner) and rawequal(old.fn,fn) then b=old
            else
                b={name=entry.name,control=control,receiver_owner=entry.owner,fn=fn,dynamic=true,parameters={'index','value'}}
                if fn~=nil then
                    local info=getinfo(fn,'S');check(info.what=='Lua','original Lua loadout callback')
                    check(#functions<MAX_OBJECTS,'function binding bound')
                    b.token=#functions+1;b.upvalues={}
                    local captures,owner_capture={},false
                    for at=1,33 do
                        local name,value=getupvalue(fn,at)
                        if name==nil then break end
                        check(at<=32,'callback capture bound');row()
                        b.upvalues[at]={name=name,value=value}
                        local is_owner=name=='self' and rawequal(value,entry.owner)
                        if name=='self' then check(is_owner,'callback self capture owner') end
                        owner_capture=owner_capture or is_owner
                        captures[at]={name=finite(name),kind=type(value),owner_identity=is_owner}
                    end
                    functions[b.token]={token=b.token,name=b.name,source=finite(info.source),first_line=info.linedefined,last_line=info.lastlinedefined,
                        control_token=token(control),owner_token=token(entry.owner),dynamic_callback=true,
                        captures=captures,owner_capture_observed=owner_capture}
                    retained_callbacks[#retained_callbacks+1]=b
                end
            end
            if fn~=nil then check(targets[fn]==nil,'distinct loadout callback');targets[fn]=b end
            next_bindings[i]=b
        end
        callback_bindings=next_bindings
        -- An unchanged Function still retains its original captured owners;
        -- validate before any next source-root body can run.
        verify_callbacks()
    end
    local function loadout_state(result_pack)
        verify()
        local memo,count={},0
        local function allocate(value)
            count=count+1;check(count<=32768,'loadout snapshot table bound')
            local out={};if value then memo[value]=out end;return out
        end
        local clone
        clone=function(value,depth,field)
            if type(value)~='table' then return finite(value) end
            check(depth<=64,'loadout snapshot depth bound')
            if memo[value] then return memo[value] end
            local meta=getmetatable(value)
            if meta~=nil then
                local class_name=type(meta)=='table' and rawget(meta,'_className')
                local label=type(class_name)=='string' and finite(class_name) or 'unlabelled'
                check(false,'loadout retained value metatable in '..(field or 'unknown')..': '..label)
            end
            local out=allocate(value)
            for key,v in next,value do
                row();check(type(key)=='string' or type(key)=='number','loadout finite key')
                rawset(out,finite(key),clone(v,depth+1,field))
            end
            return out
        end
        local function fields(value,names,class)
            if type(value)~='table' then return finite(value) end
            if memo[value] then return memo[value] end
            local meta=getmetatable(value)
            check(meta==nil or (type(class)=='table' and rawequal(meta,class) and getmetatable(class)==nil and
                rawequal(rawget(class,'__index'),class)),'loadout projected class lookup for '..names[1])
            local out=allocate(value)
            for _,key in ipairs(names) do
                row();local v=rawget(value,key)
                check(v~=nil or meta==nil or rawget(class,key)==nil,'inherited loadout data field')
                out[key]=clone(v,0,key)
            end
            return out
        end
        local function spec_row(spec)
            check(type(spec)=='table','loadout spec row')
            if getmetatable(spec)~=nil then
                check(type(spec_class)=='table' and rawequal(getmetatable(spec),spec_class) and
                    rawequal(rawget(spec,'build'),build),'actual PassiveSpec class and owner')
            end
            return fields(spec,{'title','treeVersion','jewels'},spec_class)
        end
        local function domain(key,class)
            local value=rawget(build,key)
            check(value==nil or (type(value)=='table' and rawequal(rawget(value,'build'),build)),'loadout domain '..key)
            return value,class
        end
        local tree=domain('treeTab',tree_class)
        local itemtab=domain('itemsTab',items)
        local skills=domain('skillsTab',skills_class)
        local config=domain('configTab',config_class)
        local importer=domain('importTab',import)
        local controls=rawget(build,'controls');check(controls==nil or type(controls)=='table','loadout controls')
        local drop=controls and rawget(controls,'buildLoadouts')
        local identity={build_token=token(build),domain_presence={tree=tree~=nil,items=itemtab~=nil,skills=skills~=nil,config=config~=nil,export=importer~=nil},callbacks={}}
        local out={identity=identity}
        local function identity_field(name,value)
            if value~=nil then check(type(value)=='table','loadout identity value');identity[name]=token(value) end
        end
        -- Project actual spec rows first so loadoutsList and supplied result
        -- references share these exact projected rows instead of cloning classes.
        if tree~=nil then
            local specs=rawget(tree,'specList');check(specs==nil or (type(specs)=='table' and getmetatable(specs)==nil),'source spec list')
            if specs~=nil then
                local projected=allocate(specs)
                for key,spec in next,specs do
                    row();check(type(key)=='number' or type(key)=='string','spec key')
                    projected[finite(key)]=spec_row(spec)
                end
            end
            out.tree=fields(tree,{'activeSpec','specList','showConvert'},tree_class)
            identity_field('tree_token',tree);identity_field('spec_list_token',specs)
        end
        -- Set row projection is intentionally the declared loadout consumer
        -- contract, not complete Item/Skills/Config state serialization.
        local function sets(tab,map_key,order_key,active_key,class)
            if tab==nil then return nil end
            local map=rawget(tab,map_key)
            check(map==nil or (type(map)=='table' and getmetatable(map)==nil),'loadout set map')
            if map~=nil then
                local projected=allocate(map)
                for key,value in next,map do
                    row();check(type(key)=='number' or type(key)=='string','set map key')
                    projected[finite(key)]=fields(value,{'id','title'})
                end
            end
            return fields(tab,{order_key,map_key,active_key,'modFlag'},class)
        end
        out.items=sets(itemtab,'itemSets','itemSetOrderList','activeItemSetId',items)
        out.skills=sets(skills,'skillSets','skillSetOrderList','activeSkillSetId',skills_class)
        out.config=sets(config,'configSets','configSetOrderList','activeConfigSetId',config_class)
        identity_field('items_token',itemtab);identity_field('skills_token',skills);identity_field('config_token',config)
        if itemtab~=nil then
            local active_set=rawget(itemtab,'activeItemSet')
            out.items.activeItemSet=fields(active_set,{'id','title'});identity_field('active_item_set_token',active_set)
        end
        if rawget(build,'spec')~=nil then
            local spec=rawget(build,'spec')
            spec_row(spec)
            identity_field('active_spec_token',spec)
        end
        -- TreeTab:Load can replace specList/build.spec before Sync clears
        -- loadoutsList. Preserve those reached old objects, not current-row joins.
        local loadouts_list=rawget(build,'loadoutsList')
        check(loadouts_list==nil or (type(loadouts_list)=='table' and getmetatable(loadouts_list)==nil),'source loadout list')
        if loadouts_list~=nil then
            local projected=memo[loadouts_list] or allocate(loadouts_list)
            for key,spec in next,loadouts_list do
                row();check(type(key)=='number' or type(key)=='string','loadout list key')
                projected[finite(key)]=spec_row(spec)
            end
        end
        out.build=fields(build,{'loadoutsList','treeListSpecialLinks','itemListSpecialLinks','skillListSpecialLinks','configListSpecialLinks',
            'activeLoadout','buildFlag','modFlag','spec'},build_class)
        for _,key in ipairs({'loadoutsList','treeListSpecialLinks','itemListSpecialLinks','skillListSpecialLinks','configListSpecialLinks'}) do
            identity_field(key..'_token',rawget(build,key))
        end
        out.loadoutDropdown=fields(drop,{'list','selIndex','searchTerm','searchInfos','ignoreOrder'},dropdown)
        identity_field('loadout_dropdown_token',drop)
        if importer~=nil then
            local exports=rawget(importer,'controls');check(exports==nil or type(exports)=='table','loadout export controls')
            out.export=fields(importer,{'exportSpecIndex','exportSkillSetId','exportItemSetId'},import)
            if exports~=nil then
                for _,key in ipairs({'buildPlannerSpec','buildPlannerSkillSet','buildPlannerItemSet'}) do
                    out.export[key]=fields(rawget(exports,key),{'list','selIndex','searchTerm','searchInfos','ignoreOrder'},dropdown)
                end
                out.export.path=fields(rawget(exports,'poe2ExportPath'),{'buf','caret','selStart'},rawget(classes,'EditControl'))
                out.export.buildName=fields(rawget(exports,'buildPlannerBuildName'),{'buf','placeholder'},rawget(classes,'EditControl'))
            end
        end
        local versions=rawget(globals,'treeVersions')
        check(versions==nil or (type(versions)=='table' and getmetatable(versions)==nil),'loadout version map')
        out.versions={latestTreeVersion=finite(rawget(globals,'latestTreeVersion')),treeVersions={}}
        if versions~=nil then
            for key,value in next,versions do
                row();check(type(key)=='string' or type(key)=='number','version key')
                out.versions.treeVersions[finite(key)]=fields(value,{'display'})
            end
        end
        for _,binding in ipairs(callback_bindings) do
            if binding.control~=nil then
                identity.callbacks[binding.name]={control_token=token(binding.control),function_token=binding.token,
                    owner_token=token(binding.receiver_owner),callback_present=binding.fn~=nil}
            end
        end
        if result_pack~=nil then
            check(type(result_pack)=='table' and getmetatable(result_pack)==nil,'direct result pack')
            local n=rawget(result_pack,'n')
            check(type(n)=='number' and n>=0 and n<=64 and n%1==0,'direct result pack cardinality')
            for key in next,result_pack do
                row();check(key=='n' or (type(key)=='number' and key>=1 and key<=n and key%1==0),'direct result pack key')
            end
            out.result_pack=clone(result_pack,0,'result_pack')
        end
        return out
    end
    local events,states,active={}, {}, {}
    local loadout_states,direct_snapshots={},0
    local input_contexts={}
    local failed,finished,hook=nil,false,nil
    local capture={}
    local function parameter(frame,index,wanted)
        local name,value=getlocal(frame+1,index)
        check(name==wanted,'parameter '..wanted..' got '..(name or 'nil'));return value
    end
    local function snapshot(entry,phase,ordinal)
        if loadouts then check(#states+#loadout_states+direct_snapshots<MAX_STATES,'loadout state count bound')
        else check(#states<MAX_STATES,'state count bound') end
        states[#states+1]={event_ordinal=ordinal,call_ordinal=entry.ordinal,phase=phase,name=entry.binding.name,
            receiver_token=token(entry.receiver),prior_set_token=entry.prior and token(entry.prior) or nil,
            value=state(entry.receiver,entry.prior)}
    end
    local function loadout_snapshot(entry,phase,ordinal)
        check(#states+#loadout_states+direct_snapshots<MAX_STATES,'loadout state count bound')
        loadout_states[#loadout_states+1]={event_ordinal=ordinal,call_ordinal=entry.ordinal,phase=phase,name=entry.binding.name,
            receiver_token=token(entry.receiver),root_call_ordinal=entry.root.ordinal,load_call_ordinal=entry.load and entry.load.ordinal or nil,
            value=loadout_state()}
    end
    if loadouts then function capture.loadout_snapshot(result_pack)
        check(not finished,'loadout capture already finished')
        check(#states+#loadout_states+direct_snapshots<MAX_STATES,'direct snapshot count bound')
        direct_snapshots=direct_snapshots+1
        if #active==0 then bind_callbacks() else verify_callbacks() end
        return loadout_state(result_pack)
    end end
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
            if loadouts then
                if #active==0 then verify();bind_callbacks() else verify_callbacks() end
            end
            local values={};for i,wanted in ipairs(b.parameters) do values[i]=parameter(frame,i,wanted) end
            local receiver=b.dynamic and b.control or values[1];check(type(receiver)=='table','actual receiver')
            local entry={binding=b,ordinal=record.ordinal,receiver=receiver}
            record.receiver_token=token(receiver)
            if #active>0 then
                local parent=active[#active]
                record.parent_call_ordinal=parent.ordinal;entry.load=parent.load;entry.root=parent.root
            end
            if b.name=='items_load' then
                check(rawequal(receiver,rawget(build,'itemsTab')),'actual loaded itemsTab')
                check(type(values[2])=='table' and rawget(values[2],'elem')=='Items','actual Items XML')
                entry.load=entry;entry.xml=values[2];entry.prior=rawget(receiver,'activeItemSet')
                record.xml_token=token(values[2]);record.context=context(receiver)
                local attr=rawget(values[2],'attrib');check(type(attr)=='table','Items attributes');record.requested_set=scalar(rawget(attr,'activeItemSet'))
                snapshot(entry,'before',record.ordinal)
            elseif not loadouts then check(entry.load~=nil,'nested call outside complete Load')
            elseif #active==0 then
                check(b.name=='sync_loadouts' or b.name=='lookup_loadout','unadmitted loadout root')
                check(rawequal(receiver,build),'actual loadout root receiver')
            end
            entry.root=entry.root or entry
            if entry.load then record.load_call_ordinal=entry.load.ordinal end
            if loadouts then
                record.root_call_ordinal=entry.root.ordinal
                local caller=getinfo(frame+1,'fS')
                if caller then
                    local known=targets[caller.func]
                    record.direct_caller_function_token=known and known.token or nil
                    record.direct_caller_source=finite(caller.source)
                    record.direct_caller_first_line=caller.linedefined
                end
            end
            local item_owner=entry.load and entry.load.receiver or (loadouts and rawget(build,'itemsTab'))
            if b.name=='activate_set' then
                check(rawequal(receiver,item_owner),'activation receiver');entry.prior=rawget(receiver,'activeItemSet')
                record.requested_set=scalar(values[2]);record.defer_sync=scalar(values[3]);snapshot(entry,'before',record.ordinal)
                if activation_context then
                    check(#input_contexts<8,'activation input capture count')
                    local value=activation_context(receiver,record)
                    check(type(value)=='table' and getmetatable(value)==nil,'activation input projection')
                    input_contexts[#input_contexts+1]={call_ordinal=record.ordinal,receiver_token=record.receiver_token,value=value}
                end
            elseif b.name=='populate_slots' and after_populate then
                local caller=getinfo(frame+1,'f')
                record.direct_activation_caller=caller and rawequal(caller.func,bindings[3].fn) or false
                if record.direct_activation_caller then
                    local parent=active[#active]
                    check(parent and parent.binding.name=='activate_set' and rawequal(parent.receiver,receiver),'population activation parent')
                    entry.activation_parent=parent
                end
            elseif b.name=='create_set' then record.requested_set=scalar(values[2]);record.title=scalar(values[3])
            elseif b.name=='validity' then
                check(rawequal(receiver,item_owner),'validity receiver');check(type(values[2])=='table','validity item')
                record.item_token=token(values[2]);record.item_id=scalar(rawget(values[2],'id'));record.slot=finite(values[3]);record.context=context(receiver)
                record.item_set_kind=type(values[4]);record.flag_state_kind=type(values[5])
            elseif b.name=='populate_slot' or b.name=='set_slot' then
                check(type(item_owner)=='table' and rawequal(rawget(receiver,'itemsTab'),item_owner),'slot receiver owner')
                local name=rawget(receiver,'slotName');check(type(name)=='string','actual slot name')
                local map=rawget(item_owner,'slots');check(type(map)=='table' and rawequal(rawget(map,name),receiver),'actual slot map identity')
                record.slot=finite(name);record.slot_state=slot_summary(receiver)
                if b.name=='populate_slot' then
                    local caller=getinfo(frame+1,'f');record.direct_populate_slots_caller=caller and rawequal(caller.func,bindings[4].fn) or false
                    if record.direct_populate_slots_caller then check(active[#active] and active[#active].binding.name=='populate_slots','actual traversal parent') end
                else record.requested_item=scalar(values[2]) end
            elseif b.name=='select_value' then
                if loadouts and type(values[2])=='table' then record.value={kind='table',token=token(values[2])}
                else record.value=scalar(values[2]) end
                record.key=scalar(values[3])
            elseif b.name=='select_index' then record.index=scalar(values[2]);record.no_callback=scalar(values[3])
            elseif b.name=='sync_loadouts' then
                if loadouts then check(rawequal(receiver,build),'Sync owner') end
                record.skip_build_planner_sync=scalar(values[2])
            elseif b.name=='activate_loadout' then
                record.loadout_kind=type(values[2])
                if loadouts then
                    check(rawequal(receiver,build),'SetActiveLoadout owner')
                    if type(values[2])=='table' then
                        record.loadout_token=token(values[2]);record.requested_loadout={}
                        for _,key in ipairs({'specId','itemSetId','skillSetId','configSetId'}) do
                            row();record.requested_loadout[key]=scalar(rawget(values[2],key))
                        end
                    else record.loadout=scalar(values[2]) end
                end
            elseif b.name=='lookup_loadout' then
                check(rawequal(receiver,build),'GetLoadoutByName owner');record.requested_name=scalar(values[2])
            elseif b.name=='get_spec_list' or b.name=='activate_spec' then
                check(rawequal(receiver,rawget(build,'treeTab')),'actual TreeTab owner')
                if b.name=='activate_spec' then record.requested_spec=scalar(values[2]);record.defer_sync=scalar(values[3]) end
            elseif b.name=='activate_skill_set' then
                check(rawequal(receiver,rawget(build,'skillsTab')),'actual SkillsTab owner')
                record.requested_set=scalar(values[2]);record.defer_sync=scalar(values[3])
            elseif b.name=='activate_config_set' then
                check(rawequal(receiver,rawget(build,'configTab')),'actual ConfigTab owner')
                record.requested_set=scalar(values[2]);record.init=scalar(values[3]);record.defer_sync=scalar(values[4])
            elseif b.name=='refresh_export_sets' and loadouts then
                check(rawequal(receiver,rawget(build,'importTab')),'actual ImportTab owner')
            elseif b.dynamic then
                local caller=getinfo(frame+1,'f')
                check(caller and rawequal(caller.func,bindings[9].fn),'original SetSel callback caller')
                local parent=active[#active]
                check(parent and parent.binding.name=='select_index' and rawequal(parent.receiver,receiver),'actual callback control parent')
                check(rawequal(rawget(receiver,'selFunc'),b.fn),'actual control callback')
                record.control_token=token(receiver);record.callback_owner_token=token(b.receiver_owner)
                record.index=scalar(values[1])
                if type(values[2])=='table' then record.value={kind='table',token=token(values[2])}
                else record.value=scalar(values[2]) end
            end
            if loadouts and (b.name=='sync_loadouts' or b.name=='lookup_loadout' or b.name=='activate_loadout') then
                loadout_snapshot(entry,'before',record.ordinal)
            end
            active[#active+1]=entry
        elseif event=='return' then
            local entry=active[#active];check(entry and rawequal(entry.binding.fn,b.fn),'complete call return pairing')
            record.call_ordinal=entry.ordinal;record.receiver_token=token(entry.receiver)
            if entry.load then record.load_call_ordinal=entry.load.ordinal end
            if loadouts then record.root_call_ordinal=entry.root.ordinal;verify_callbacks() end
            if b.name=='items_load' or b.name=='activate_set' then snapshot(entry,'after',record.ordinal)
            elseif b.name=='populate_slots' and entry.activation_parent then
                check(rawequal(active[#active-1],entry.activation_parent),'population return activation identity')
                record.activation_call_ordinal=entry.activation_parent.ordinal
                snapshot(entry.activation_parent,'after_populate',record.ordinal)
            elseif b.name=='populate_slot' or b.name=='set_slot' then record.slot=finite(rawget(entry.receiver,'slotName'));record.slot_state=slot_summary(entry.receiver) end
            if loadouts and (b.name=='sync_loadouts' or b.name=='lookup_loadout' or b.name=='activate_loadout') then
                loadout_snapshot(entry,'after',record.ordinal)
            end
            active[#active]=nil
        else error('item-set lifecycle: unknown hook event',0) end
        events[#events+1]=record
    end
    hook=function(event)
        if event~='call' and event~='return' then return end
        local info=getinfo(2,'f');local b=info and targets[info.func]
        if b and (b.name=='items_load' or #active>0 or (loadouts and (b.name=='sync_loadouts' or b.name=='lookup_loadout'))) then
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
            local final_items=rawget(build,'itemsTab')
            local finite_items
            if not loadouts or final_items~=nil then finite_items=state(final_items,nil) end
            capture.report={events=events,states=states,functions=functions,incomplete_calls=incomplete,
                activation_input_contexts=activation_context and input_contexts or nil,
                finite_post_import=finite_items,
                scope={observed=observed,complete_original_load_boundary=true,whole_original_object_graph=false,
                    source_methods_unchanged=true,original_iterators_unchanged=true,jit_disabled=true,warm_claim=false,
                    parameter_projection_not_actual_arity=true,validity_return_values_not_observed=true,
                    item_projection_fields={'id','name','baseName','type','rarity','jewelSocketCount'},
                    rune_definition_projection_fields={'name'},trade_transform_presence_only=true},
                bounds={events=MAX_EVENTS,objects=MAX_OBJECTS,depth=MAX_DEPTH,states=MAX_STATES,rows=MAX_ROWS,text_bytes=MAX_TEXT},
                retained_objects=#objects,rows=rows,text_bytes=bytes}
            if loadouts then
                if #active==0 then bind_callbacks() else verify_callbacks() end
                capture.report.loadout_states=loadout_states
                capture.report.finite_post_loadouts=loadout_state()
                capture.report.scope.loadouts=true
                capture.report.scope.finite_post_import_available=finite_items~=nil
                capture.report.scope.loadout_roots_outside_load=true
                capture.report.scope.loadout_return_packs_not_observed=true
                capture.report.scope.loadout_result_pack_supplied_by_direct_caller=true
                capture.report.scope.loadout_projection_fields={
                    spec={'title','treeVersion','jewels'},set={'id','title'},
                    dropdown={'list','selIndex','searchTerm','searchInfos','ignoreOrder'}}
                capture.report.scope.complete_cross_domain_state=false
                capture.report.loadout_direct_snapshots=direct_snapshots
                capture.report.retained_objects=#objects;capture.report.rows=rows;capture.report.text_bytes=bytes
            end
        end)
        if prior_jit then on() end
        if not ok then failed=failed or message end
        if failed then error(failed,0) end
        return capture.report
    end
    return capture
end
return {start=start}
