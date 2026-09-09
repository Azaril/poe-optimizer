-- Full constructed DB for shipped lookups; additional records are explicit
-- test-owned injected data, with unchanged original GetUniqueDBItem behavior.
runUniqueConstruction('control')
local entries=main.uniqueDB.list
local rows={}
local function add(name,title,base)
 local item=new('Item')
 item.rarity='UNIQUE';item.name=name;item.title=title;item.baseName=base
 local found=item:GetUniqueDBItem()
 local key
 if found then for k,v in pairs(entries)do if v==found then assert(not key,'aliased fixture entry');key=k end end end
 rows[#rows+1]={name=name,title=title,base_name=base,found_key=key}
end
local first,second
for name,item in pairs(entries)do
 first=first or item; if item~=first and not second then second=item end
 add(name,nil,nil)
 add('unmatched source lookup',item.title,item.baseName)
 add('unmatched source lookup',item.title,'Runeforged '..item.baseName)
 add('unmatched source lookup',item.title,'Runemastered '..item.baseName)
end
add(first.name,second.title,'Runeforged '..second.baseName)
local originals={}
for key in pairs(entries)do originals[#originals+1]=key end
local injected={}
for index,key in ipairs({'Oracle key, ','',', Iron Ring','Oracle key, Iron Ring',
 'Oracle key, Runeforged Iron Ring','oracle key, Iron Ring','Oracle key,  Iron Ring'})do
 -- Empty canonical names are outside the native data contract. Keep the empty
 -- request case below, but use a nonempty injected key for this row.
 if key=='' then key='Oracle key, Runemastered Iron Ring'end
 local old=originals[index]
 local item=entries[old]
 entries[old]=nil
 item.requirements={naturalLevel=index==1 and -1/math.huge or index,level=index+20}
 entries[key]=item
 injected[#injected+1]={old_key=old,new_key=key,natural=index==1 and '-0' or tostring(index),level=index+20}
end
local extra={
 {'unmatched source lookup','Oracle key','Runeforged '},
 {'unmatched source lookup','Oracle key','Runemastered '},
 {'unmatched source lookup','','Runeforged Iron Ring'},
 {'unmatched source lookup',false,'Runeforged Iron Ring'},
 {'unmatched source lookup','Oracle key',false},
 {'unmatched source lookup','Oracle key',''},
 {'unmatched source lookup','Oracle key','Runeforged Runeforged Iron Ring'},
 {'unmatched source lookup','Oracle key','Runemastered Runeforged Iron Ring'},
 {'unmatched source lookup','Oracle key','before Runeforged Iron Ring'},
 {'unmatched source lookup','Oracle key','runeforged Iron Ring'},
 {'unmatched source lookup','oracle key','Runeforged Iron Ring'},
 {'unmatched source lookup','Oracle key','Runeforged  Iron Ring'},
 {'Oracle key, Iron Ring','Oracle key','Runeforged Runeforged Iron Ring'},
 {'', 'Oracle key', 'Runemastered Iron Ring'},
 {'Oracle key, ',false,false},
}
local original_count=#rows
for _,row in ipairs(extra)do add(row[1],row[2]~=false and row[2] or nil,row[3]~=false and row[3] or nil)end
return {rows=rows,original_count=original_count,injected=injected}
