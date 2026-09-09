-- Only fields actually read by original generated-name/grant routines are projected.
-- All records originate from full original Data construction, including excluded gems.
local out = {gems={},skills={}}
for id,g in pairs(data.gems) do
    local e=g.grantedEffect
    local effect={id=e.id,name=e.name,baseTypeName=e.baseTypeName,hidden=e.hidden,support=e.support,fromItem=e.fromItem,skillTypes=e.skillTypes,statSets={}}
    for i,s in ipairs(e.statSets)do effect.statSets[i]={baseFlags={buff=s.baseFlags.buff}} end
    local row={name=g.name,grantedEffectId=g.grantedEffectId,grantedEffect=effect,tags=g.tags}
    if g.additionalGrantedEffects then
        row.additionalGrantedEffects={}
        for i,a in ipairs(g.additionalGrantedEffects)do row.additionalGrantedEffects[i]={support=a.support};row["additionalGrantedEffectId"..i]=g["additionalGrantedEffectId"..i] end
    end
    out.gems[id]=row
end
for id,s in pairs(data.skills)do out.skills[id]={name=s.name} end
return out
