-- Deliberately authored test database records; the pinned Item constructor and
-- every requirement operation run unchanged. No source data file is edited.
local nan=0/0
local minusZero=-1/math.huge
local rows={
 {id='nil-natural-falls-back',level=42},
 {id='zero-natural-precedes-level',natural=0,level=42},
 {id='negative-zero-natural',natural=minusZero,level=42},
 {id='natural-without-level',natural=42},
 {id='both-missing'},
 {id='nan-natural',natural=nan,level=42},
 {id='ignored-nan-level',natural=42,level=nan},
 {id='positive-infinity',natural=math.huge},
 {id='negative-infinity',natural=-math.huge},
 {id='own-level-maximum',natural=5,level=200,authored='6'},
 {id='negative-zero-authored',natural=minusZero,authored='-0'},
 {id='absent-natural-nan-level',level=nan},
 {id='header-positive-infinity',natural=5,authored=string.rep('9',400)},
 {id='header-negative-infinity',natural=5,authored='-'..string.rep('9',400)},
}
local out={cases={},max={}}
for _,row in ipairs(rows)do
 main.uniqueDB={list={['Oracle numeric, Iron Ring']={requirements={naturalLevel=row.natural,level=row.level}}}}
 local raw='Oracle numeric\nIron Ring'..(row.authored and '\nLevelReq: '..row.authored or '')
 local ok,item=pcall(function()return new('Item'):Item(raw,'UNIQUE',true)end)
 local result={id=row.id,ok=ok,authored=row.authored,db_natural=oracleNumberBits(row.natural),db_level=oracleNumberBits(row.level)}
 if ok then result.natural=oracleNumberBits(item.requirements.naturalLevel);result.level=oracleNumberBits(item.requirements.level)
 else result.error=item end
 out.cases[#out.cases+1]=result
end
local values={0,minusZero,42,nan,math.huge,-math.huge}
for _,a in ipairs(values)do for _,b in ipairs(values)do
 out.max[#out.max+1]={a=oracleNumberBits(a),b=oracleNumberBits(b),result=oracleNumberBits(math.max(a,b))}
end end
out.triples={
 {inputs={'nan','42','0'},result=oracleNumberBits(math.max(nan,42,0))},
 {inputs={'42','nan','0'},result=oracleNumberBits(math.max(42,nan,0))},
 {inputs={'42','0','nan'},result=oracleNumberBits(math.max(42,0,nan))},
 {inputs={'-0','0','-0'},result=oracleNumberBits(math.max(minusZero,0,minusZero))},
}
-- Directly exercise the unchanged complete requirement-finalization block with
-- explicit incoming rune state. This is not a claim that the original grammar
-- exposes a runeLevel header, and does not replace the full constructor cases.
out.rune_operands={}
for _,row in ipairs({{id='nan',value=nan},{id='negative-zero',value=minusZero},{id='nil'}})do
 local item={base={req={}},sockets={},requirements={level=minusZero,runeLevel=row.value}}
 function item:GetUniqueDBItem()return {requirements={naturalLevel=minusZero}}end
 local ok,err=pcall(originalRequirementFinalize,item)
 out.rune_operands[#out.rune_operands+1]={id=row.id,ok=ok,error=not ok and err or nil,
  natural=oracleNumberBits(item.requirements.naturalLevel),level=oracleNumberBits(item.requirements.level)}
end
return out
