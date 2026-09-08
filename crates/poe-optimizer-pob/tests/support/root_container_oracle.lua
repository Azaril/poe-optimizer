-- Test-only host for unchanged source methods installed by root_container_parity.rs.
-- Controls retain the real SetText/SelByValue methods. No UI callback, network,
-- build calculation, aura parser or configuration effect is substituted here.
local function noop() end
local function edit()
 return setmetatable({buf='',ResetUndo=noop,changeFunc=function()error('unexpected GUI callback')end}, {__index=EditClass})
end
local function dropdown(list)
 return setmetatable({list=list,selIndex=1,changeFunc=function()error('unexpected GUI callback')end}, {__index=DropDownClass})
end
local function mod_list()
 return {NewMod=function(self,...)table.insert(self,{...})end,
         AddMod=function(self,mod)table.insert(self,mod)end}
end
new=function(kind)
 assert(kind=='ModDB' or kind=='ModList')
 return {ModDB=mod_list,ModList=mod_list}
end
NewImageHandle=function()return{Load=noop}end
local function tree()
 -- Tooltip/image construction has no saved state; the original constructor is
 -- executed with these inert GUI resources, not copied numerical defaults.
 local prior=new
 new=function(kind)if kind=='Tooltip'then return{Tooltip=function()return{}end}end;return prior(kind)end
 local t=setmetatable({}, {__index=PassiveTreeViewClass}):PassiveTreeView()
 new=prior
 return t
end
local function party(build)
 local t=setmetatable({build=build,controls={}}, {__index=PartyTabClass})
 original_party_defaults(t)
 for _,name in ipairs({'editPartyMemberStats','editAuras','editCurses','editWarcries','editLinks','enemyCond','enemyMods'})do t.controls[name]=edit()end
 for _,name in ipairs({'simpleAuras','simpleCurses','simpleWarcries','simpleLinks','simpleEnemyMods'})do t.controls[name]={}end
 t.controls.importCodeDestination=dropdown(original_party_destinations())
 t.controls.appendNotReplace={state=false};t.controls.ShowAdvanceTools={state=false}
 return t
end
local function import(build,context)
 return setmetatable({build=build,controls={
  accountRealm=dropdown({{id='initial'},{id='PC'},{id='PoE2'},{id='context'}}),
  charSelectLeague=dropdown({{id='initial'},{id='Standard'},{id='custom'}}),
  accountName=edit(),enablePartyExportBuffs={state=false},buildPlannerUseGeneratedItemText={state=true}
 }}, {__index=ImportTabClass})
end
local function calcs(build,sections)
 local t=setmetatable({build=build,ResetUndo=noop}, {__index=CalcsTabClass})
 original_calcs_defaults(t)
 t.sectionList=sections or {}
 return t
end
local function node(text,kind)
 local roots,err=originalXml.ParseXML(text);assert(roots,err)
 if roots[1].elem==kind then return roots[1]end
 for _,n in ipairs(roots[1])do if type(n)=='table'and n.elem==kind then return n end end
 return {elem=kind,attrib={}}
end
local function save(t,kind)
 local n={elem=kind,attrib={}};t:Save(n)
 local text,err=originalXml.ComposeXML(n);assert(text,err)
 return n,text
end
local function load(text,context)
 context=context or {}
 main={lastRealm=context.lastRealm,gameAccounts=context.accounts or {}}
 common.sha1=function(name)return 'hash:'..name end
 local b={importLink=context.importLink}
 b.partyTab=party(b);b.importTab=import(b,context);b.calcsTab=calcs(b,context.sections);b.treeView=tree()
 local errors={};launch={ShowErrMsg=function(_,message)table.insert(errors,message)end}
 diagnostics={};ConPrintf=function(message)table.insert(diagnostics,message)end
 local results={}
 for _,entry in ipairs({{'Import',b.importTab},{'Party',b.partyTab},{'Calcs',b.calcsTab},{'TreeView',b.treeView}})do
  results[entry[1]]=entry[2]:Load(node(text,entry[1]),'independent source oracle')
 end
 return b,errors,results
end
local function observe(text,context)
 local b,errors,results=load(text,context)
 local i,p,c,t=b.importTab,b.partyTab,b.calcsTab,b.treeView
 local import_saved,import_xml=save(i,'Import')
 local party_saved,party_xml=save(p,'Party')
 local calcs_saved,calcs_xml=save(c,'Calcs')
 local tree_saved,tree_xml=save(t,'TreeView')
 return {build=b,errors=errors,load_results=results,diagnostics=diagnostics,
  import_saved=import_saved,party_saved=party_saved,calcs_saved=calcs_saved,tree_saved=tree_saved,
  saved_xml={Import=import_xml,Party=party_xml,Calcs=calcs_xml,TreeView=tree_xml},
  selected_realm=i.controls.accountRealm.list[i.controls.accountRealm.selIndex].id,
  selected_league=i.controls.charSelectLeague.list[i.controls.charSelectLeague.selIndex].id}
end
return {observe=observe,load=load,node=node,save=save,mod_list=mod_list,
 config=function(inputs,calcs_inputs)
  local t=setmetatable({configSets={{input=inputs}},activeConfigSetId=1,
    build={calcsTab={input=calcs_inputs}},BuildModList=noop,UpdateControls=noop},{__index=ConfigTabClass})
  t.input=inputs;return t
 end}
