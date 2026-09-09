-- Observation only. Every wrapped calculation still calls the complete original.
defence_stages={};defence_calls={}
local class=common.classes.Item
local build=class.BuildModList
local parse=modLib.parseMod
local building=false
function class:BuildModList(...)
 local stage={before=defence_snapshot(self)}
 defence_stages[#defence_stages+1]=stage
 local old=building;building=true
 local result=build(self,...)
 building=old
 stage.after=defence_snapshot(self)
 return result
end
function modLib.parseMod(text,combined,...)
 local result,extra=parse(text,combined,...)
 if not building then defence_calls[#defence_calls+1]={text=text,combined=combined==true}end
 return result,extra
end
