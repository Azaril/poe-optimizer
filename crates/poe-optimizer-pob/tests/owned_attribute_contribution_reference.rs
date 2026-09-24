//! Candidate item BASE attribute contributions, not equipped-build activation or
//! final actor attributes. All parsing, assembly and ModDB queries are upstream.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;

fn oracle(warm: bool) -> runtime::Oracle {
    let oracle = runtime::Oracle::new();
    oracle.lua.globals().set("attributeWarm", warm).unwrap();
    oracle.lua.load(r#"
attributeFamilies={
 {text="Strength",names={"Str"}},
 {text="Dexterity",names={"Dex"}},
 {text="Intelligence",names={"Int"}},
 {text="all Attributes",names={"Str","Dex","Int","All"}},
}
function attributeCandidate(id,headers,body)
 local item=new("Item")
 item.id=id
 item:ParseRaw("Rarity: RARE\nAttribute Contribution Probe\nIron Ring\nItem Level: 80\nQuality: 0\n"..headers.."\nImplicits: 0\n"..body,nil,false)
 assert(item.baseName=="Iron Ring")
 -- Repeat the complete original assembly explicitly; an observation must not
 -- inject already parsed modifier objects or emulate the assembly algorithm.
 item:BuildModList()
 assert(item.id==id and item.slotModList and item.slotModList[1])
 local db=new("ModDB"):ModDB()
 db:AddList(item.slotModList[1])
 return item,db
end
function assertAttributeContribution(item,db,names,value)
 local expected={}
 for _,name in ipairs(names) do expected[name]=true end
 for _,list in ipairs({item.baseModList,item.slotModList[1]}) do
  local count=0
  for _,mod in ipairs(list) do
   if mod.name=="Str" or mod.name=="Dex" or mod.name=="Int" or mod.name=="All" then
    count=count+1
    assert(expected[mod.name] and mod.type=="BASE" and mod.value==value)
    assert(mod.value%1==0 and mod.flags==0 and mod.keywordFlags==0 and #mod==0)
    assert(mod.source==item.modSource)
   end
  end
  assert(count==#names)
 end
 for _,name in ipairs({"Str","Dex","Int","All"}) do
  local amount=expected[name] and value or 0
  assert(db:Sum("BASE",nil,name)==amount)
  assert(db:Sum("BASE",{flags=ModFlag.Attack,keywordFlags=KeywordFlag.Attack},name)==amount)
  assert(db:Sum("BASE",{flags=ModFlag.Spell,keywordFlags=KeywordFlag.Spell},name)==amount)
  assert(db:Sum("INC",nil,name)==0 and db:More(nil,name)==1)
 end
end
if attributeWarm then jit.on() else jit.off();jit.flush() end
"#).set_name("@attribute-candidate-original-method-inputs").exec().unwrap();
    oracle
}

#[test]
fn flat_attributes_are_integral_global_base_contributions_after_original_formatting() {
    for warm in [false, true] {
        oracle(warm)
            .lua
            .load(
                r#"
for _,family in ipairs(attributeFamilies) do
 for _,case in ipairs({
  {"+0",0},{"+11",11},{"+10.25",10},{"+10.5",11},
  {"-11",-11},{"-10.25",-10},{"-10.5",-11},
 }) do
  -- Signed/fractional raw inputs are source-only contrasts. This test does not
  -- broaden the owned import's reviewed plus-integer grammar.
  local item,db=attributeCandidate(41,"",case[1].." to "..family.text)
  assert(#item.explicitModLines==1 and not item.explicitModLines[1].extra)
  assertAttributeContribution(item,db,family.names,case[2])
 end
end
local item,db=attributeCandidate(42,"","+11 to all Attributes")
assert(#item.explicitModLines==1 and #item.explicitModLines[1].modList==4)
-- The source retains its All marker, but Str/Dex/Int queries each receive one
-- BASE contribution. All is not automatically added to any of those channels.
assert(db:Sum("BASE",nil,"All")==11)
assert(db:Sum("BASE",nil,"Str","Dex","Int")==33)
assert(db:Sum("BASE",nil,"Str")==11 and db:Sum("BASE",nil,"Int")==11)
"#,
            )
            .set_name("@source-formatted-attribute-base-contributions")
            .exec()
            .unwrap();
    }
}

#[test]
fn authentic_corruption_and_attribute_catalyst_inputs_precede_base_contribution_queries() {
    for warm in [false, true] {
        oracle(warm).lua.load(r#"
for _,family in ipairs(attributeFamilies) do
 for _,case in ipairs({
  {headers="Catalyst: Adaptive\nCatalystQuality: 20",prefix="{tags:attribute}",number="+11",value=13},
  {headers="Catalyst: Adaptive\nCatalystQuality: 20",prefix="{tags:attribute}",number="+10.5",value=13},
  {headers="Catalyst: Adaptive\nCatalystQuality: -50",prefix="{tags:attribute}",number="+11",value=5},
  {headers="",prefix="{corruptedRange:0.5}",number="+11",value=6},
  {headers="",prefix="{corruptedRange:0.5}",number="-11",value=-5},
  {headers="Catalyst: Adaptive\nCatalystQuality: 20",prefix="{tags:attribute}{corruptedRange:0.5}",number="+10.5",value=7},
 }) do
  local item,db=attributeCandidate(51,case.headers,case.prefix..case.number.." to "..family.text)
  assert(#item.explicitModLines==1 and not item.explicitModLines[1].extra)
  assertAttributeContribution(item,db,family.names,case.value)
 end
end
-- An attribute name alone does not supply the tag that enables its catalyst.
local item,db=attributeCandidate(52,"Catalyst: Adaptive\nCatalystQuality: 20","+11 to Strength")
assertAttributeContribution(item,db,{"Str"},11)
"#).set_name("@source-scaled-attribute-base-contributions").exec().unwrap();
    }
}

#[test]
fn repeated_equal_item_and_line_occurrences_remain_distinct_in_original_moddb() {
    for warm in [false, true] {
        oracle(warm)
            .lua
            .load(
                r#"
local raw="+7 to all Attributes\n+7 to all Attributes\n+3 to Strength"
local first=attributeCandidate(61,"",raw)
local second=attributeCandidate(62,"",raw)
assert(first~=second and first.modSource~=second.modSource)
for _,item in ipairs({first,second}) do
 assert(#item.explicitModLines==3)
 assert(item.explicitModLines[1]~=item.explicitModLines[2])
 assert(item.explicitModLines[1].line==item.explicitModLines[2].line)
 assert(#item.explicitModLines[1].modList==4 and #item.explicitModLines[2].modList==4)
end
local db=new("ModDB"):ModDB()
db:AddList(first.slotModList[1]);db:AddList(second.slotModList[1])
assert(db:Sum("BASE",nil,"Str")==34)
assert(db:Sum("BASE",nil,"Dex")==28 and db:Sum("BASE",nil,"Int")==28)
assert(db:Sum("BASE",nil,"All")==28)
assert(#db.mods.Str==6 and #db.mods.Dex==4 and #db.mods.Int==4 and #db.mods.All==4)
for _,item in ipairs({first,second}) do
 assert(db:Sum("BASE",{source=item.modSource},"Str")==17)
 assert(db:Sum("BASE",{source=item.modSource},"Dex")==14)
 assert(db:Sum("BASE",{source=item.modSource},"Int")==14)
end
-- Distinct physical members survive; composing candidate lists here does not
-- certify these items are equipped, valid, or globally active on any build.
for index=1,#db.mods.All do
 for other=index+1,#db.mods.All do assert(db.mods.All[index]~=db.mods.All[other]) end
end
"#,
            )
            .set_name("@source-repeated-attribute-item-occurrences")
            .exec()
            .unwrap();
    }
}
