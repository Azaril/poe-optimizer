-- Observation only. The complete original functions run inside pcall so the
-- observer can restore its own phase state when the original raises an error.
buff_stages={};buff_calls={};buff_attempts={}
local class=common.classes.Item
local build=class.BuildModList
local parse=modLib.parseMod
local building=false
local current
local parseRaw=class.ParseRaw
function class:ParseRaw(...)
 local old=current;current=self
 local ok,result=pcall(parseRaw,self,...)
 current=old
 if not ok then error(result,0) end
 return result
end
function class:BuildModList(...)
 local stage={before=buff_snapshot(self)}
 buff_stages[#buff_stages+1]=stage
 local old=building;building=true
 local ok,result=pcall(build,self,...)
 building=old
 if not ok then stage.error=tostring(result);error(result,0) end
 stage.after=buff_snapshot(self)
 return result
end
function modLib.parseMod(text,combined,...)
 local attempt
 if not building then
  attempt={text=text,input_type=type(text),combined=combined==true,before=current and buff_snapshot(current)}
  buff_attempts[#buff_attempts+1]=attempt
 end
 local result,extra=parse(text,combined,...)
 if attempt then
  attempt.success=true
  buff_calls[#buff_calls+1]={text=text,combined=combined==true}
 end
 return result,extra
end
