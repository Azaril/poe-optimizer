-- Test-only complete-method source binding and finite read-set projection.
-- Loaded before source initialization; never replaces an original function.
local globals, rawget, rawset, next, type, rawequal, error = _G, rawget, rawset, next, type, rawequal, error
local getinfo, getmetatable = debug.getinfo, debug.getmetatable
local huge, ipairs = math.huge, ipairs
local MAX_ROWS, MAX_TABLES, MAX_BYTES, MAX_DEPTH = 262144, 32768, 16*1024*1024, 64
local ITEM_FIELDS = {'type','rarity','baseName','base','clusterJewel','canSocketJewelBase'}
local BASE_FIELDS = {'type','subType','tags'}
local NODE_FIELDS = {'sinister','containJewelSocket','charmSocket','expansionJewel'}
local function check(ok, message) if not ok then error('slot validity observation: '..message,0) end end
local function bind()
    local common=rawget(globals,'common');check(type(common)=='table','common')
    local classes=rawget(common,'classes');check(type(classes)=='table','classes')
    local item_class=rawget(classes,'Item')
    local class=rawget(classes,'ItemsTab');check(type(class)=='table','ItemsTab class')
    local target=rawget(class,'IsItemValidForSlot');check(type(target)=='function','original method')
    local info=getinfo(target,'S');check(info.what=='Lua','original Lua method')
    local build=rawget(globals,'build');check(type(build)=='table','completed original build')
    local tab=rawget(build,'itemsTab');check(type(tab)=='table','actual items tab')
    check(rawequal(rawget(tab,'build'),build),'actual receiver build')
    local inventory,sets,slots=rawget(tab,'items'),rawget(tab,'itemSets'),rawget(tab,'slots')
    check(type(inventory)=='table' and type(sets)=='table' and type(slots)=='table','actual inventory/set/slot maps')
    local spec=rawget(build,'spec');check(type(spec)=='table','current actual spec')
    local tree=rawget(spec,'tree');check(type(tree)=='table','current actual tree')
    local tree_nodes,spec_nodes=rawget(tree,'nodes'),rawget(spec,'nodes')
    check(type(tree_nodes)=='table' and type(spec_nodes)=='table','node maps')
    local function verify()
        check(rawequal(rawget(globals,'common'),common) and rawequal(rawget(common,'classes'),classes) and
            rawequal(rawget(classes,'ItemsTab'),class) and rawequal(rawget(classes,'Item'),item_class) and rawequal(rawget(class,'IsItemValidForSlot'),target),'original method changed')
        check(rawequal(rawget(globals,'build'),build) and rawequal(rawget(build,'itemsTab'),tab) and
            rawequal(rawget(tab,'build'),build) and
            rawequal(rawget(tab,'items'),inventory) and rawequal(rawget(tab,'itemSets'),sets) and
            rawequal(rawget(tab,'slots'),slots),'original receiver changed')
        check(rawequal(rawget(build,'spec'),spec) and rawequal(rawget(spec,'tree'),tree) and
            rawequal(rawget(tree,'nodes'),tree_nodes) and rawequal(rawget(spec,'nodes'),spec_nodes),'original node context changed')
        check(getmetatable(inventory)==nil and getmetatable(sets)==nil and getmetatable(slots)==nil and
            getmetatable(tree_nodes)==nil and getmetatable(spec_nodes)==nil,'lookup map metatable')
    end
    local function projection()
        verify()
        local memo,rows,tables,bytes={},0,0,0
        local function scalar(v)
            local k=type(v)
            check(k=='nil' or k=='boolean' or k=='string' or (k=='number' and v==v and v>-huge and v<huge),'finite scalar')
            if k=='string' then bytes=bytes+#v;check(#v<=65536 and bytes<=MAX_BYTES,'text bound') end
            return v
        end
        local function allocate() tables=tables+1;check(tables<=MAX_TABLES,'table bound');return {} end
        local clone
        clone=function(v,depth)
            if type(v)~='table' then return scalar(v) end
            check(depth<=MAX_DEPTH,'depth bound')
            if memo[v] then return memo[v] end
            check(getmetatable(v)==nil,'retained value metatable')
            local out=allocate();memo[v]=out
            for k,x in next,v do
                rows=rows+1;check(rows<=MAX_ROWS,'row bound')
                check(type(k)=='string' or (type(k)=='number' and k==k and k>-huge and k<huge),'finite key')
                rawset(out,scalar(k),clone(x,depth+1))
            end
            return out
        end
        local function fields(v,names,inherited)
            if type(v)~='table' then return scalar(v) end
            local meta=getmetatable(v)
            -- PassiveSpec:Init uses the exact tree.nodes[id] as the effective
            -- node's metatable. PassiveTree gives that plain table __index=self.
            -- Resolve only this authenticated single table fallback, without
            -- invoking metamethods or accepting structurally similar stand-ins.
            check(meta==nil or (type(inherited)=='table' and rawequal(meta,inherited) and
                getmetatable(inherited)==nil and rawequal(rawget(inherited,'__index'),inherited)),
                'selected-field receiver metatable')
            local out=allocate()
            for _,k in ipairs(names) do
                rows=rows+1;check(rows<=MAX_ROWS,'projected field row bound')
                local value=rawget(v,k)
                if value==nil and meta~=nil then value=rawget(inherited,k) end
                rawset(out,k,clone(value,1))
            end
            return out
        end
        local function item(v)
            check(type(v)=='table','original Item receiver')
            local meta=getmetatable(v)
            check(meta==nil or (rawequal(meta,item_class) and rawequal(rawget(item_class,'__index'),item_class) and getmetatable(item_class)==nil),'unknown Item class lookup')
            local out=allocate()
            for _,k in ipairs(ITEM_FIELDS) do
                rows=rows+1;check(rows<=MAX_ROWS,'projected item row bound')
                local value=rawget(v,k)
                check(value~=nil or meta==nil or rawget(item_class,k)==nil,'inherited Item input field')
                if k=='base' then value=fields(value,BASE_FIELDS)
                elseif k=='clusterJewel' and type(value)=='table' then value=fields(value,{'sizeIndex'})
                else value=clone(value,1) end
                rawset(out,k,value)
            end
            return out
        end
        local items=allocate()
        for id,v in next,inventory do rows=rows+1;check(rows<=MAX_ROWS,'item row bound');items[scalar(id)]=item(v) end
        local projected_tree,projected_spec=allocate(),allocate()
        for id,node in next,tree_nodes do rows=rows+1;check(rows<=MAX_ROWS,'tree row bound');projected_tree[scalar(id)]=fields(node,NODE_FIELDS) end
        for id,node in next,spec_nodes do rows=rows+1;check(rows<=MAX_ROWS,'spec row bound');projected_spec[scalar(id)]=fields(node,NODE_FIELDS,rawget(tree_nodes,id)) end
        return {items=items,itemSets=clone(sets,0),activeItemSet=clone(rawget(tab,'activeItemSet'),0),
            itemOrderList=clone(rawget(tab,'itemOrderList'),0),itemSetOrderList=clone(rawget(tab,'itemSetOrderList'),0),
            treeNodes=projected_tree,specNodes=projected_spec,
            scope={declared_read_set=true,whole_original_items=false,selected_context_after_import=true,
                read_fields=ITEM_FIELDS,base_fields=BASE_FIELDS,node_fields=NODE_FIELDS}}
    end
    local function entries(map,kind)
        local out={}
        for key in next,map do
            check(#out<4096,'entry count bound')
            check(type(key)==kind,'map key kind')
            out[#out+1]=key
        end
        return out
    end
    return {target=target,receiver=tab,items=inventory,sets=sets,slots=entries(slots,'string'),
        item_ids=entries(inventory,'number'),set_ids=entries(sets,'number'),projection=projection,verify=verify,
        declaration={source=info.source,first=info.linedefined,last=info.lastlinedefined},
        scope={source_fed_context=true,native_preparation=false,original_method_replaced=false,call_hook=false}}
end
return {bind=bind}
