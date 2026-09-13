-- Startup-only witness. The original C sorter and comparator are never replaced.
local globals,table_lib,debug_lib,jit_lib=_G,table,debug,jit
local sort=table.sort
local getinfo,getlocal,getupvalue,gethook,sethook,getmetatable=debug.getinfo,debug.getlocal,debug.getupvalue,debug.gethook,debug.sethook,debug.getmetatable
local rawget,rawset,next,type,rawequal,error,pcall,ipairs=rawget,rawset,next,type,rawequal,error,pcall,ipairs
local status,off,on,flush=jit.status,jit.off,jit.on,jit.flush
local find,sub=string.find,string.sub
local huge=math.huge
local MAX_HOOK_EVENTS,MAX_COMPARE,MAX_ROWS,MAX_DEPTH=20000000,1000000,4096,128
local fields={'name','slot','order','req','group'}
local function check(ok,message)if not ok then error('rune-sort startup observer: '..message,0)end end
local function source_path(s)
 local slash,backslash='/Classes/ItemsTab.lua','\\Classes\\ItemsTab.lua'
 return type(s)=='string' and #s<=65536 and (s=='@Classes/ItemsTab.lua' or sub(s,-#slash)==slash or sub(s,-#backslash)==backslash)
end
local function finite(v)
 check(type(v)=='nil' or type(v)=='boolean' or (type(v)=='string' and #v<=65536) or (type(v)=='number' and v==v and v>-huge and v<huge),'finite comparison field')
 return v
end
local function same(a,b)return rawequal(a,b) and (type(a)~='number' or a~=0 or 1/a==1/b)end
return function(observed)
 check(type(observed)=='boolean','observed flag');check(gethook()==nil,'pre-existing hook')
 check(type(sort)=='function' and getinfo(sort,'S').what=='C','original C table.sort')
 local capture,report={},{actual_sort_return_observed=false,actual_sort_completion_observed=false}
 local prior_jit=status();local hook,active,finished,cleaned,failure=nil,nil,false,false,nil
 local hook_events,comparisons,sort_calls,active_line_events,other_sort_calls=0,0,0,0,0
 local rows,records,ids,container,chunk,comparator,cmp_info,before_order,after_order={},{},{},nil,nil,nil,nil,{},{}
 local text_bytes=0
 local common,classes,items,consumer,consumer_slot=nil,nil,nil,nil,nil
 local module_slot
 local function identity()
  check(rawequal(rawget(globals,'table'),table_lib) and rawequal(rawget(table_lib,'sort'),sort),'retained original sorter changed')
  check(rawequal(rawget(globals,'debug'),debug_lib) and rawequal(rawget(debug_lib,'getinfo'),getinfo) and rawequal(rawget(debug_lib,'getlocal'),getlocal) and rawequal(rawget(debug_lib,'getupvalue'),getupvalue) and rawequal(rawget(debug_lib,'gethook'),gethook) and rawequal(rawget(debug_lib,'sethook'),sethook) and rawequal(rawget(debug_lib,'getmetatable'),getmetatable),'debug primitive changed')
  check(rawequal(rawget(globals,'jit'),jit_lib) and rawequal(rawget(jit_lib,'status'),status) and rawequal(rawget(jit_lib,'off'),off) and rawequal(rawget(jit_lib,'on'),on) and rawequal(rawget(jit_lib,'flush'),flush),'JIT primitive changed')
 end
 local function cleanup()
  if cleaned then return end
  cleaned=true
  if observed then
   if rawequal(gethook(),hook) then sethook() else failure=failure or 'rune-sort startup observer: foreign hook preserved' end
   if prior_jit then on() end
  end
 end
 local function unchanged(row,record)
  check(type(row)=='table' and getmetatable(row)==nil,'plain comparison row')
  for _,key in ipairs(fields) do check(same(rawget(row,key),rawget(record,key)),'comparison field mutation '..key) end
 end
 local function vector()
  check(type(container)=='table' and getmetatable(container)==nil,'retained actual module rune vector')
  local count=0;for key in next,container do count=count+1;check(count<=MAX_ROWS and type(key)=='number' and key>=1 and key%1==0 and key<=#rows,'dense source array')end
  check(count==#rows,'source row cardinality')
  local order,seen={},{}
  for i=1,#rows do
   local row=rawget(container,i);local token=ids[row];check(token~=nil and not seen[token],'source permutation identity');seen[token]=true
   unchanged(row,records[token]);order[i]=token
  end
  return order
 end
 local function collect(event,target)
  local frame
  for level=2,MAX_DEPTH do local info=getinfo(level,'f');if not info then break end;if rawequal(info.func,target) then frame=level;break end end
  check(frame~=nil,'actual hook frame')
  if active and rawequal(target,chunk) and (event=='line' or event=='return') then
   local info=getinfo(frame,'Sl')
   check(info.what=='main' and source_path(info.source),'retained original module continuation')
   if event=='line' then
    -- Pinned lj_parse.c proto_finish emits FNEW at the closing line (2286);
    -- parse_func assigns the named store its declaration line (2250). Both
    -- belong to the next original definition, not execution of its body.
    if info.currentline==2240 or info.currentline==2248 then return end
    check(info.currentline==2250 or info.currentline==2286,'unexpected next original module line: '..tostring(info.currentline))
   end
   local name,value=getlocal(frame,module_slot)
   check(name=='runeModLines' and rawequal(value,container),'retained module local at continuation')
   check(comparator~=nil and comparisons>0,'actual original comparator missing')
   after_order=vector()
   check(rawget(rows[1],'name')=='None' and rawget(rows[1],'slot')=='None' and after_order[1]==1,'actual original initial None row first')
   report.actual_sort_completion_observed=true;report.actual_initial_row_retained_first=true
   report.sort_completion_boundary={kind=event=='line' and 'next_original_module_line' or 'original_module_return',source=info.source,line=info.currentline,same_original_chunk=true,retained_module_local_identity=true}
   active=nil;identity();check(not status(),'interpreted original module continuation');cleanup();return
  end
  if rawequal(target,sort) then
   if event=='call' then
    local caller=getinfo(frame+1,'Sfl')
    if not (caller and source_path(caller.source) and caller.currentline==2240 and caller.what=='main') then if active then other_sort_calls=other_sort_calls+1 end;return end
    check(active==nil and sort_calls==0,'unique original module sorter call');identity();check(not status(),'interpreted startup sort');sort_calls=sort_calls+1
    chunk=caller.func;check(caller.what=='main' and caller.linedefined==0,'original module chunk')
    local locals_ended=false
    for index=1,256 do
     local name,value=getlocal(frame+1,index)
     if name==nil then locals_ended=true;break end
     if name=='runeModLines' then check(module_slot==nil,'unique original module runeModLines local');module_slot=index;container=value end
    end
    check(locals_ended and module_slot~=nil and type(container)=='table' and getmetatable(container)==nil,'actual module runeModLines local')
    local count=0;for key in next,container do count=count+1;check(count<=MAX_ROWS and type(key)=='number' and key>=1 and key%1==0,'source numeric array key')end
    check(count>1,'original comparator must be invoked')
    for i=1,count do
     local row=rawget(container,i);check(type(row)=='table' and getmetatable(row)==nil and ids[row]==nil,'distinct source row identities')
     rows[i]=row;ids[row]=i;local record={}
     for _,key in ipairs(fields) do local value=finite(rawget(row,key));if type(value)=='string' then text_bytes=text_bytes+#value;check(text_bytes<=2*1024*1024,'comparison field text bound')end;rawset(record,key,value) end
     records[i]=record;before_order[i]=i
    end
    active=true;report.original_chunk={source=caller.source,first_line=caller.linedefined,last_line=caller.lastlinedefined,call_line=caller.currentline}
    -- C return hooks are not required by LuaJIT. Enable line events only for
    -- this bounded interval and finish at the retained module's continuation.
    check(rawequal(gethook(),hook),'owned hook before enabling continuation events');sethook(hook,'crl')
   elseif event=='return' and active then
    -- Optional diagnostic only: completion still requires the original Lua
    -- module continuation, so unrelated/nested sorter returns cannot finish it.
    local caller=getinfo(frame+1,'fl')
    if caller and rawequal(caller.func,chunk) and caller.currentline==2240 then
     report.actual_sort_return_observed=true
    end
   end
  elseif event=='call' and active then
   local parent=getinfo(frame+1,'f')
   if not (parent and rawequal(parent.func,sort)) then return end
   local original=getinfo(frame+2,'fl')
   if not (original and rawequal(original.func,chunk) and original.currentline==2240) then return end
   local info=getinfo(target,'S');check(info.what=='Lua' and source_path(info.source) and info.linedefined==2240 and info.lastlinedefined==2248,'actual original comparator declaration: '..tostring(info.what)..' '..tostring(info.source)..':'..tostring(info.linedefined)..'-'..tostring(info.lastlinedefined))
   if comparator==nil then comparator=target;cmp_info={source=info.source,first_line=info.linedefined,last_line=info.lastlinedefined} else check(rawequal(target,comparator),'comparator identity changed')end
   local a_name,a=getlocal(frame,1);local b_name,b=getlocal(frame,2);check(a_name=='a' and b_name=='b','actual comparator parameters')
   local ai,bi=ids[a],ids[b];check(ai~=nil and bi~=nil,'comparator parameters belong to retained actual rows')
   unchanged(a,records[ai]);unchanged(b,records[bi]);comparisons=comparisons+1;check(comparisons<=MAX_COMPARE,'comparison call bound')
  end
 end
 hook=function(event)
  if failure then error(failure,0)end
  hook_events=hook_events+1
  if hook_events>MAX_HOOK_EVENTS then failure='rune-sort startup observer: startup hook event bound';cleanup();error(failure,0)end
  if active and event=='line' then active_line_events=active_line_events+1 end
  local info=getinfo(2,'f');local target=info and info.func
  if target and (rawequal(target,sort) or (active and (event=='call' or rawequal(target,chunk)))) then
   local ok,message=pcall(collect,event,target)
   if not ok then failure=message;cleanup();error(message,0)end
  end
 end
 if observed then off();flush();sethook(hook,'cr') end
 local function bind_consumer()
  local actual_common=rawget(globals,'common');check(type(actual_common)=='table','actual common')
  local actual_classes=rawget(actual_common,'classes');check(type(actual_classes)=='table','actual classes')
  local actual_items=rawget(actual_classes,'ItemsTab');check(type(actual_items)=='table','actual ItemsTab class')
  local actual_consumer=rawget(actual_items,'GetValidRunesForItem');check(type(actual_consumer)=='function','actual retained consumer')
  local info=getinfo(actual_consumer,'S');check(info.what=='Lua' and source_path(info.source) and info.linedefined==2250 and info.lastlinedefined==2286,'original consumer declaration')
  local slot,ended
  for index=1,256 do
   local name,value=getupvalue(actual_consumer,index)
   if name==nil then ended=true;break end
   if name=='runeModLines' then check(slot==nil and rawequal(value,container),'consumer upvalue is actual observed module local');slot=index end
  end
  check(ended and slot~=nil,'bounded consumer upvalue inventory')
  if consumer then check(rawequal(common,actual_common) and rawequal(classes,actual_classes) and rawequal(items,actual_items) and rawequal(consumer,actual_consumer) and slot==consumer_slot,'retained class/consumer/upvalue binding changed')
  else common,classes,items,consumer,consumer_slot=actual_common,actual_classes,actual_items,actual_consumer,slot end
  report.consumer={source=info.source,first_line=info.linedefined,last_line=info.lastlinedefined,upvalue_name='runeModLines',upvalue_slot=slot,module_local_slot=module_slot,actual_table_identity_join=true}
 end
 function capture.abort()cleanup()end
 function capture.finish()
  if finished then if failure then error(failure,0)end;return report end
  cleanup()
  local ok,result=pcall(function()
   identity();check(gethook()==nil,'before-build unexpected hook')
   if failure then error(failure,0)end
   if observed then check(sort_calls==1 and active==nil and report.actual_sort_completion_observed==true,'incomplete original startup sort');bind_consumer()end
   report.observed=observed;report.startup_jit_was_enabled=prior_jit;report.startup_jit_cache_flushed=observed;report.original_c_sort_retained_before_initialization=true;report.sort_identity_before_build=true
   report.comparator=cmp_info;report.hook_events=hook_events;report.comparison_calls=comparisons;report.active_line_events=active_line_events;report.other_sort_calls_during_witness=other_sort_calls;report.rows=#rows;report.comparison_fields=fields
   report.source_comparison_fields=records;report.before_order=before_order;report.after_order=after_order
   report.bounds={hook_events=MAX_HOOK_EVENTS,comparison_calls=MAX_COMPARE,rows=MAX_ROWS,depth=MAX_DEPTH,text_bytes=2*1024*1024}
   report.scope={source_functions_replaced=false,actual_c_frame_arguments_read=false,c_sort_return_hook_required=false,completion_requires_original_module_continuation=observed,line_events_only_after_target_sort_start=observed,comparator_actual_named_parameters_observed=observed,original_module_local_identity_and_row_permutation_verified=observed,only_comparison_fields_immutability_claim=true,hook_removed_before_xml_import=true,control_startup_unhooked=not observed,jit_cache_restoration_claim=false,whole_registry_restoration_claim=false,warm_claim=false}
   return report
  end)
  finished=true
  if not ok then failure=result;error(result,0)end
  return result
 end
 function capture.verify_after_import()
  identity();check(gethook()==nil,'post-import unexpected hook')
  report.sort_identity_after_import=true
  if observed then
   bind_consumer()
   local info=getinfo(comparator,'S');check(info.source==cmp_info.source and info.linedefined==cmp_info.first_line and info.lastlinedefined==cmp_info.last_line,'retained comparator declaration')
   local current=vector();check(#current==#after_order,'post-import source permutation count')
   for i=1,#current do check(current[i]==after_order[i],'source module-local rune order changed after startup')end
   report.comparison_fields_and_module_local_order_unchanged_after_import=true
  end
  return report
 end
 function capture.permutations()
  check(observed and finished and comparator~=nil,'actual comparator required');capture.verify_after_import()
  local was_on=status();off();flush()
  local ok,result=pcall(function()
   local function copied(row,name)
    local out={};for _,key in ipairs(fields) do out[key]=rawget(row,key)end;if name then out.name=name end;return out
   end
   local function find_row(name,slot)
    local result
    for _,row in ipairs(rows) do if rawget(row,'name')==name and rawget(row,'slot')==slot then check(result==nil,'unique actual permutation exemplar');result=row end end
    check(result~=nil,'actual permutation exemplar missing');return result
   end
   local a=find_row('Adept Rune','armour');local b=find_row('Greater Adept Rune','caster');local c=find_row("Aldur's Legacy",'armour');local minimum=find_row('None','None')
   check(comparator(a,b) and comparator(b,c) and comparator(c,a),'actual source comparator cycle')
   local function matrix(originals,label)
    local entries,permutation,used={},{},{}
    local function visit(depth)
     if depth<=#originals then for i=1,#originals do if not used[i] then used[i]=true;permutation[depth]=i;visit(depth+1);used[i]=nil end end;return end
     check(#entries<24,'permutation bound');local input,ids,before,after={}, {},{},{}
     for i,index in ipairs(permutation) do local row=copied(originals[index]);input[i]=row;ids[row]=index;before[i]=index end
     sort(input,comparator);local seen={}
     for i=1,#originals do local index=ids[input[i]];check(index and not seen[index],'derived permutation row identity');seen[index]=true;unchanged(input[i],originals[index]);after[i]=index end
     check(ids[input[1]]==1,'universal minimum first');entries[#entries+1]={before=before,after=after,minimum_first=true,identities_and_fields_preserved=true}
    end
    visit(1);return {label=label,rows=#originals,cases=entries}
   end
   local cycle=matrix({minimum,a,b,c},'actual cycle plus actual unique minimum, fresh copied comparison fields')
   local first=copied(a,'derived equal row A');local second=copied(a,'derived equal row B')
   check(not comparator(first,second) and not comparator(second,first),'derived noninitial ties')
   local ties=matrix({minimum,first,second},'derived distinct noninitial tied rows')
   local bad1={name='malformed missing req',order=9,group=1};local bad2={name='finite req',order=9,req=2,group=1}
   local valid,message=pcall(sort,{bad1,bad2},comparator)
   check(not valid and type(message)=='string' and find(message,'compare',1,true),'original malformed comparison error')
   local invalid=function()return true end
   local valid_guard,guard=pcall(sort,{{},{},{},{}},invalid)
   check(not valid_guard and type(guard)=='string' and find(guard,'invalid order',1,true),'intrinsic invalid comparison guard')
   return {cycle=cycle,ties=ties,malformed_original_comparison={failed=true,message=message},deliberately_substituted_symmetric_comparator={failed=true,message=guard,scope='intrinsic guard mechanics only; not the original comparator'},scope='fresh derived rows and retained actual original comparator/Csort; no observed traversal or result supplied to native production'}
  end)
  if was_on then on()end
  identity();if not ok then error(result,0)end;capture.verify_after_import();return result
 end
 return capture
end
