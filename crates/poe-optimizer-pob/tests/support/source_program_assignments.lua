-- Pinned-source semantic probes; nested factories are deliberately source-only.
local probes = {}
local function snapshot(a,b,t,k,x,events)
    return {a=a,b=b,t_is_a=t==a,t_is_b=t==b,k=k,x=x,events=events}
end
function probes.local_registers_changed_by_rhs()
    local a,b={},{}
    local t,k,x=a,'old','initial'
    local events={}
    local function rhs() events[1]='rhs'; t=b; k='new'; return 11,22 end
    t[k],x=rhs()
    return snapshot(a,b,t,k,x,events)
end
function probes.explicit_address_temporaries()
    local a,b={},{}
    local t,k,x=a,'old','initial'
    local events={}
    local function table_address() events[#events+1]='table'; return t end
    local function key_address() events[#events+1]='key'; return k end
    local function rhs() events[#events+1]='rhs'; t=b; k='new'; return 11,22 end
    table_address()[key_address()],x=rhs()
    return snapshot(a,b,t,k,x,events)
end
function probes.local_table_temporary_key()
    local a,b={},{}
    local t,k,x=a,'old','initial'
    local function key_address() return k end
    local function rhs() t=b; k='new'; return 11,22 end
    t[key_address()],x=rhs()
    return snapshot(a,b,t,k,x,{})
end
function probes.temporary_table_local_key()
    local a,b={},{}
    local t,k,x=a,'old','initial'
    local function table_address() return t end
    local function rhs() t=b; k='new'; return 11,22 end
    table_address()[k],x=rhs()
    return snapshot(a,b,t,k,x,{})
end
function probes.upvalues_changed_by_rhs()
    local a,b={},{}
    local t,k,x=a,'old','initial'
    local function rhs() t=b; k='new'; return 11,22 end
    local function assign() t[k],x=rhs() end
    assign()
    return snapshot(a,b,t,k,x,{})
end
function probes.later_table_local_conflict()
    local a,b,c={},{},{}
    local t,k=a,'old'
    local function rhs() t=b; k='new'; return 11,c end
    t[k],t=rhs()
    return {a=a,b=b,c=c,t_is_c=t==c,k=k}
end
function probes.later_key_local_conflict()
    local a,b={},{}
    local t,k=a,'old'
    local function rhs() t=b; k='new'; return 11,'assigned' end
    t[k],k=rhs()
    return {a=a,b=b,t_is_b=t==b,k=k}
end
function probes.both_later_local_conflicts()
    local a,b,c={},{},{}
    local t,k=a,'old'
    local function rhs() t=b; k='new'; return 11,c,'assigned' end
    t[k],t,k=rhs()
    return {a=a,b=b,c=c,t_is_c=t==c,k=k}
end
function probes.local_before_index()
    local i,t=1,{}
    i,t[i]=2,10
    return {i=i,t=t}
end
function probes.index_before_local()
    local i,t=1,{}
    t[i],i=10,2
    return {i=i,t=t}
end
function probes.repeated_local_and_table_aliases()
    local i,t=0,{}
    local alias=t
    i,i,t[1],alias[1]=1,2,3,4
    return {i=i,t=t,alias_is_t=alias==t}
end
function probes.lhs_address_effects_precede_rhs()
    local a,b={},{}
    local t,k,x=a,'old','initial'
    local events={}
    local function change_key() events[#events+1]='key'; t=b; k='new'; return 'address' end
    local function rhs() events[#events+1]='rhs'; return 11,22 end
    t[change_key()],x=rhs()
    return snapshot(a,b,t,k,x,events)
end
function probes.later_lhs_effects_precede_hazard_copy()
    local a,b={},{}
    local t,k,x=a,'old','initial'
    local events={}
    local function address() events[#events+1]='later-key'; t=b; k='new'; return 'second' end
    local function rhs() events[#events+1]='rhs'; return 11,22,33 end
    t[k],a[address()],k=rhs()
    return snapshot(a,b,t,k,x,events)
end
function probes.rhs_error_keeps_address_effects_and_no_stores()
    local t,x={},'initial'
    local events={}
    local function key() events[#events+1]='key'; return 'slot' end
    local function rhs() events[#events+1]='rhs'; error('rhs sentinel') end
    local ok,err=pcall(function() t[key()],x=rhs() end)
    return {ok=ok,error=err,t=t,x=x,events=events}
end
function probes.right_store_error_prevents_left_store()
    local t,bad={},nil
    local events={}
    local function key(name) events[#events+1]=name; return 'slot' end
    local function rhs() events[#events+1]='rhs'; return 11,22 end
    local ok,err=pcall(function() t[key('left')],bad[key('right')]=rhs() end)
    return {ok=ok,error=err,t=t,events=events}
end
function probes.left_store_error_preserves_right_store()
    local t,bad={},nil
    local events={}
    local function key(name) events[#events+1]=name; return 'slot' end
    local function rhs() events[#events+1]='rhs'; return 11,22 end
    local ok,err=pcall(function() bad[key('left')],t[key('right')]=rhs() end)
    return {ok=ok,error=err,t=t,events=events}
end
function probes.address_error_prevents_later_addresses_and_rhs()
    local t,bad={},nil
    local events={}
    local function key(name) events[#events+1]=name; return 'slot' end
    local function rhs() events[#events+1]='rhs'; return 11,22 end
    local ok,err=pcall(function() bad.missing[key('left')],t[key('right')]=rhs() end)
    return {ok=ok,error=err,t=t,events=events}
end
function probes.store_metamethod_rebinds_local_address()
    local a,b={},{}
    local t,k=a,'old'
    local events={}
    local proxy=setmetatable({}, {__newindex=function(_,key,value)
        events[#events+1]='right-store'; t=b; k='new'
    end})
    t[k],proxy.slot=11,22
    return {a=a,b=b,t_is_b=t==b,k=k,events=events}
end
function probes.mixed_upvalue_local_index()
    local captured=0
    local function assign()
        local x,t=0,{}
        captured,x,t[1],captured=1,2,3,4
        return {x=x,t=t,captured=captured}
    end
    return assign()
end
function probes.result_packs_and_extra_effects()
    local a,b,c='a','b','c'
    local events={}
    local function values() events[#events+1]='values'; return 1,nil,3,4 end
    a,b,c=values()
    local first={a=a,b=b,c=c}
    local function extra() events[#events+1]='extra'; return 9 end
    a,b=values(),extra(),extra()
    return {first=first,a=a,b=b,c=c,events=events}
end
function probes.parenthesized_locals()
    local a,b={},{}
    local t,k,x=a,'old','initial'
    local function rhs() t=b; k='new'; return 11,22 end
    (t)[(k)],x=rhs()
    return snapshot(a,b,t,k,x,{})
end
function probes.constant_true_logical_key()
    local a,b={},{}
    local t,k,x=a,'old','initial'
    local function rhs() t=b; k='new'; return 11,22 end
    t[true and k],x=rhs()
    return snapshot(a,b,t,k,x,{})
end
function probes.constant_false_logical_key()
    local a,b={},{}
    local t,k,x=a,'old','initial'
    local function rhs() t=b; k='new'; return 11,22 end
    t[false or k],x=rhs()
    return snapshot(a,b,t,k,x,{})
end
function probes.constant_arithmetic_logical_key()
    local a,b={},{}
    local t,k,x=a,'old','initial'
    local function rhs() t=b; k='new'; return 11,22 end
    t[(1+2) and k],x=rhs()
    return snapshot(a,b,t,k,x,{})
end
function probes.dynamic_logical_key()
    local a,b={},{}
    local t,k,x=a,'old','initial'
    local function rhs() t=b; k='new'; return 11,22 end
    t[k or 'fallback'],x=rhs()
    return snapshot(a,b,t,k,x,{})
end
function probes.constant_true_logical_table()
    local a,b={},{}
    local t,k,x=a,'old','initial'
    local function rhs() t=b; k='new'; return 11,22 end
    (true and t)[k],x=rhs()
    return snapshot(a,b,t,k,x,{})
end
local function native_index_before_local(i,t,value,next_i)
    t[i],i=value,next_i
    return i
end
local function native_local_before_index(i,t,value,next_i)
    i,t[i]=next_i,value
    return i
end
local function native_duplicate(i,t)
    i,i,t[1],t[1]=1,2,3,4
    return i
end
local function native_effect(side,value)
    side.count=side.count+1
    return value,nil,'extra'
end
local function native_extra(t,side)
    local x
    x,t[1]=native_effect(side,1),native_effect(side,2),native_effect(side,3)
    return x,side.count
end
local function native_values() return 1,nil,3,4 end
local function native_packs(t)
    local x
    x,t[1],t[2],t[3]=native_values()
    return x
end
local function native_bad_rhs(side)
    side.count=side.count+1
    return nil+1
end
local function native_rhs_error(t,side)
    local x='initial'
    t[native_effect(side,'slot')],x=native_bad_rhs(side)
    return x
end
local function native_store_right_error(t,side,bad)
    t[native_effect(side,'left')],bad[native_effect(side,'right')]=11,22
end
local function native_store_left_error(t,side,bad)
    bad[native_effect(side,'left')],t[native_effect(side,'right')]=11,22
end
local function native_address_error(t,side,bad)
    bad.missing[native_effect(side,'left')],t[native_effect(side,'right')]=11,22
end
local function native_single(t,side)
    t[native_effect(side,'slot')]=native_effect(side,9)
    return t.slot
end
local function native_read(t,side)
    return t[native_effect(side,'slot')]
end
local function native_nested(t,side)
    return t[native_effect(side,'child')][native_effect(side,'slot')]
end
local function make_native_live()
    local captured=0
    local table_value,key_value={},'old'
    local function rhs(new_table,new_key)
        table_value=new_table
        key_value=new_key
        return 11,22
    end
    local function capture_addresses(new_table,new_key)
        local x
        table_value[key_value],x=rhs(new_table,new_key)
        return x,table_value,key_value
    end
    local function mixed(t)
        local x
        captured,x,t[1],captured=1,2,3,4
        return captured,x
    end
    local function read() return captured,table_value,key_value end
    return {capture_addresses=capture_addresses,mixed=mixed,read=read}
end
probes.native={index_before_local=native_index_before_local,local_before_index=native_local_before_index,
    duplicate=native_duplicate,extra=native_extra,packs=native_packs,rhs_error=native_rhs_error,
    store_right_error=native_store_right_error,store_left_error=native_store_left_error,
    address_error=native_address_error,single=native_single,read=native_read,nested=native_nested}
probes.native_live=make_native_live()
return probes
