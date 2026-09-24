//! Optional authenticated source observations, not native admission or actor parity.
//! Ordinary rarity is INC; ordinary Chaos Resistance is BASE. Original ParseRaw
//! and assembly run unchanged behind a bounded phase observer.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
use mlua::{Function, Table};
use std::collections::BTreeMap;

// Keep this suite's observer small and share the authenticated source host. Lua
// assertions inspect original objects; they are not a second implementation of
// parsing, formatting, item assembly or native lifecycle behavior.
const OBSERVER: &str = r#"
families={
 {name='LootRarity',kind='INC',prefix='',suffix='% increased Rarity of Items found'},
 {name='ChaosResist',kind='BASE',prefix='+',suffix='% to Chaos Resistance'},
}
local lists={'classRequirementModLines','buffModLines','enchantModLines','runeModLines','implicitModLines','explicitModLines'}
local parse,format=modLib.parseMod,itemLib.applyRange
local build=common.classes.Item.BuildModList
local updateRunes=common.classes.Item.UpdateRunes
trace={calls={},formats={},assembly=false,rune_rebuild=false,rune_rebuilds={}}
function common.classes.Item:BuildModList(...)
 local previous=trace.assembly;trace.assembly=true
 local result=build(self,...);trace.assembly=previous;return result
end
function common.classes.Item:UpdateRunes(...)
 assert(#self.runeModLines<=512,'source rune member bound before reconstruction')
 local before={};for _,row in ipairs(self.runeModLines)do before[#before+1]=row end
 local previous=trace.rune_rebuild;trace.rune_rebuild=true
 local result=updateRunes(self,...);trace.rune_rebuild=previous
 assert(#self.runeModLines<=512,'source rune member bound after reconstruction')
 local after={};for _,row in ipairs(self.runeModLines)do after[#after+1]=row end
 assert(#trace.rune_rebuilds<512,'rune reconstruction trace limit')
 trace.rune_rebuilds[#trace.rune_rebuilds+1]={before=before,after=after}
 return result
end
function modLib.parseMod(text,combined,...)
 local mods,extra=parse(text,combined,...)
 if not trace.assembly then
  assert(#trace.calls<512 and #text<=16384,'rarity/chaos parse trace limit')
  trace.calls[#trace.calls+1]={text=text,combined=combined==true,mods=mods and copyTable(mods),extra=extra,rune_rebuild=trace.rune_rebuild}
 end
 return mods,extra
end
function itemLib.applyRange(text,range,scalar,corrupted,...)
 local output=format(text,range,scalar,corrupted,...)
 if not trace.assembly then
  assert(#trace.formats<512 and #text<=16384,'rarity/chaos format trace limit')
  trace.formats[#trace.formats+1]={text=text,scalar=scalar,output=output,rune_rebuild=trace.rune_rebuild}
 end
 return output
end
function probeRaw(raw)
 assert(#raw<=32768,'raw item bound')
 trace.calls={};trace.formats={};trace.assembly=false;trace.rune_rebuild=false;trace.rune_rebuilds={}
 local item=new('Item');item:ParseRaw(raw,nil,false);return item
end
function probe(base,headers,body)
 local item=probeRaw('Rarity: RARE\nRarity Chaos Probe\n'..base..'\nItem Level: 80\nQuality: 0\n'..headers..'\nImplicits: 0\n'..body)
 assert(item.baseName==base,'wrong source base: '..base);return item
end
function textFor(f,n)return f.prefix..tostring(n)..f.suffix end
function members(item,text)
 local out={}
 for _,list in ipairs(lists)do
  assert(#item[list]<=512,'source member bound')
  for _,row in ipairs(item[list])do if row.line==text then out[#out+1]={list=list,row=row}end end
 end
 return out
end
function assertRecords(mods,f,value)
 assert(type(mods)=='table'and #mods==1,'one parsed numeric record')
 local mod=mods[1]
 assert(mod.name==f.name and mod.type==f.kind,'exact source name/type')
 assert(mod.value==value,'source numeric value: '..tostring(mod.value)..' expected '..value)
 assert(mod.flags==0 and mod.keywordFlags==0 and mod[1]==nil,'unconditional source record')
end
function noCombined()
 for _,call in ipairs(trace.calls)do assert(not call.combined,'unexpected following-line retry: '..call.text)end
end
function hasCombined(text)
 for _,call in ipairs(trace.calls)do if call.combined and (not text or call.text:find(text,1,true))then return true end end
 return false
end
function initial(text,f,value,scalar,count)
 count=count or 1
 local observed=0
 for _,event in ipairs(trace.formats)do if event.text==text and not event.rune_rebuild then
  observed=observed+1;assert(event.scalar==scalar,'initial catalyst scalar')
  local calls=0
  for _,call in ipairs(trace.calls)do if call.text==event.output and not call.combined and not call.rune_rebuild then
   calls=calls+1;assert(not call.extra,'partial initial parse: '..event.output);assertRecords(call.mods,f,value)
  end end
  assert(calls==count,'initial parse occurrence count: '..text..' '..calls..' expected '..count)
 end end
 assert(observed==count,'initial formatter occurrence count: '..text..' '..observed..' expected '..count)
end
function independent(item,text,f,value)
 local matches=members(item,text)
 assert(#matches==1 and matches[1].list=='explicitModLines','one supplied source member: '..text)
 assert(not matches[1].row.extra and #matches[1].row.modTags==0,'ordinary member')
 assertRecords(matches[1].row.modList,f,value)
 assert(#item.explicitModLines==2,'member plus following sentinel')
 initial(text,f,value,1);noCombined()
end
function generated(base)
 local out={}
 for _,kind in ipairs({'flask','charm'})do
  local parent=base[kind]
  if parent and parent.buff then for _,text in ipairs(parent.buff)do out[text]=true end end
 end
 return out
end
function componentRecords(item,f)
 local values={}
 for _,mod in ipairs(item.baseModList)do if mod.name==f.name and mod.type==f.kind then values[#values+1]=mod.value end end
 return values
end
"#;

fn source() -> runtime::Oracle {
    let oracle = runtime::Oracle::new();
    oracle
        .lua
        .load(OBSERVER)
        .set_name("@rarity-chaos-observer")
        .exec()
        .unwrap();
    oracle
}
fn run(script: &str) {
    source()
        .lua
        .load(script)
        .set_name("@rarity-chaos-source-evidence")
        .exec()
        .unwrap();
}

#[test]
fn complete_catalog_proves_ordinary_members_except_exact_generated_duplicates() {
    run(r#"
 local names={};for name in pairs(data.itemBases)do names[#names+1]=name end;table.sort(names)
 assert(#names==1756,'constructed catalog size')
 local variants={families[1],{name='LootRarity',kind='INC',prefix='',suffix='% reduced Rarity of Items found',sign=-1},families[2]}
 local noPrefix,parses,independentCount=0,0,0;local skipped={}
 for index,base in ipairs(names)do
  local buffs=generated(data.itemBases[base]);if next(buffs)==nil then noPrefix=noPrefix+1 end
  local header=({'','Catalyst: Chayula\'s\nCatalystQuality: -200',
    'Catalyst: Chayula\'s\nCatalystQuality: 20','Catalyst: Chayula\'s\nCatalystQuality: 1000000000000000000000000000000'})[index%4+1]
  for _,f in ipairs(variants)do for _,amount in ipairs({0,15,18,1000000})do
   local numeric=amount*(f.sign or 1)
   local text=textFor(f,amount);local item=probe(base,header,text..'\nwhile stationary');parses=parses+1
   if buffs[text]then
    local matches=members(item,text)
    assert(#matches==1 and matches[1].list=='buffModLines','generated membership replaces supplied text')
    assertRecords(matches[1].row.modList,f,amount)
    assert(#item.explicitModLines==1 and item.explicitModLines[1].line=='while stationary')
    for _,event in ipairs(trace.formats)do assert(event.text~=text,'generated duplicate bypasses initial formatter')end
    skipped[#skipped+1]=base..'/'..text
   else
    independent(item,text,f,numeric);independentCount=independentCount+1
    local values=componentRecords(item,f)
    assert(#values==1 and values[1]==numeric,'no additional implicit numeric contribution')
   end
  end end
  if index%128==0 then collectgarbage('collect')end
 end
 assert(noPrefix==1743 and parses==21072 and independentCount==21070)
 assert(#skipped==2 and skipped[1]=='Amethyst Charm/+18% to Chaos Resistance'
  and skipped[2]=='Golden Charm/15% increased Rarity of Items found')
 "#);
}

#[test]
fn unsigned_integer_spellings_and_formatter_boundaries_are_distinct() {
    run(r#"
 for _,f in ipairs(families)do
  for _,n in ipairs({'0','000','0000010','999999','0001000000','1000001'})do
   local text=textFor(f,n);local item=probe('Iron Ring','',text..'\nwhile stationary')
   independent(item,text,f,tonumber(n))
  end
  for _,n in ipairs({'','+10','+-10','1e1','NaN','1000000000000000'})do
   local text=textFor(f,n);probe('Iron Ring','',text..'\nwhile stationary')
   assert(hasCombined(),'malformed/outside proved spelling should retry: '..text)
  end
  for _,case in ipairs({{'10.25',10},{'10.5',11},{'0.49',0},{'0.5',1}})do
   local text=textFor(f,case[1]);probe('Iron Ring','',text..'\nwhile stationary')
   initial(text,f,case[2],1);noCombined()
  end
 end
 -- The direct INC grammar accepts integer counts. Fresh item formatting can
 -- round a decimal before reaching it; do not reuse direct-parser conclusions.
 local mods,extra=modLib.parseMod('10.5% increased Rarity of Items found')
 assert(not mods or extra,'direct decimal INC is not complete')
 local mods,extra=modLib.parseMod('+10.5% to Chaos Resistance')
 assert(not extra);assertRecords(mods,families[2],10.5)
 for _,text in ipairs({'10% to Chaos Resistance','-0% to Chaos Resistance','-0.1% to Chaos Resistance'})do
  probe('Iron Ring','',text..'\nwhile stationary');assert(hasCombined(),'Chaos sign/zero contrast')
 end
 "#);
}

#[test]
fn reduced_rarity_applies_sign_after_integer_corruption_and_magnitude() {
    run(r#"
 for _,key in ipairs({'#% increased Rarity of Items found','#% to Chaos Resistance'})do
  local row=data.modScalability[key];assert(#row==1 and row[1].isScalable and row[1].formats==nil)
 end
 local reduced=data.modScalability['#% reduced Rarity of Items found']
 assert(#reduced==1 and reduced[1].isScalable and #reduced[1].formats==1 and reduced[1].formats[1]=='negate')
 for _,case in ipairs({{'10',30},{'10.49',30},{'10.5',33},{'0.49',0},{'0.5',3}})do
  for _,input in ipairs({case[1]..'% reduced Rarity of Items found','-'..case[1]..'% increased Rarity of Items found'})do
   local output=itemLib.applyRange(input,1,1.5,2)
   assert(output==case[2]..'% reduced Rarity of Items found','reduced format/sign order: '..input..' -> '..output)
   local mods,extra=modLib.parseMod(output);assert(not extra);assertRecords(mods,families[1],-case[2])
  end
 end
 local formatted=itemLib.formatValue(10,2,1.5,1)
 assert(tonumber(formatted)==30,'unsigned source component, sign still separate')
 assert(tonumber(itemLib.formatValue(-10,2,1.5,1))==-28,'signed Direct would not be equivalent')
 for _,case in ipairs({{'10',-10},{'0',0},{'000',0},{'0000010',-10},{'0001000000',-1000000},{'10.5',-11},{'0.49',0},{'0.5',-1}})do
  local text=case[1]..'% reduced Rarity of Items found'
  probe('Iron Ring','',text..'\nwhile stationary');initial(text,families[1],case[2],1);noCombined()
 end
 -- Reduced and increased are distinct physical grammars and share INC only after sign projection.
 "#);
}

#[test]
fn actual_tags_and_persistent_controls_not_the_english_name_select_catalysts() {
    run(r#"
 for _,f in ipairs(families)do
  local text=textFor(f,11)
  for _,case in ipairs({{'','chaos',11,1},{'Chayula\'s','',11,1},{'Chayula\'s','chaos',13,1.2},
      {'Chayula\'s','drop',11,1},{'Flesh','chaos',11,1}})do
   local headers=case[1]==''and ''or 'Catalyst: '..case[1]..'\nCatalystQuality: 20'
   local tag=case[2]==''and ''or '{tags:'..case[2]..'}'
   local item=probe('Iron Ring',headers,tag..text..'\nwhile stationary')
   assert(#members(item,text)==1);initial(text,f,case[3],case[4]);noCombined()
  end
  local nextText=textFor(f,8)
  local item=probe('Iron Ring','Catalyst: Chayula\'s\nCatalystQuality: 20',
   '[{ Modifier - Chaos }]\n'..text..'\n'..nextText)
  assert(#item.explicitModLines==2)
  for _,member in ipairs(item.explicitModLines)do assert(#member.modTags==1 and member.modTags[1]=='chaos')end
  initial(text,f,13,1.2);initial(nextText,f,9,1.2);noCombined()
  probe('Iron Ring','Catalyst: Chayula\'s\nCatalystQuality: -200',
   '[{ Modifier - Chaos }]\n'..text..'\n'..nextText)
  assert(trace.formats[1].scalar==-1)
  -- Magnitude is applied after the initial antonym scan, so a newly negative
  -- result is not normalized again before the first parser call.
  local expected=f.kind=='BASE'and '+-11% to Chaos Resistance'or '-11% increased Rarity of Items found'
  assert(trace.formats[1].output==expected,'negative scale preserves stage order')
  assert(hasCombined(),'negative scaled ordinary form retries')
 end
 "#);
}

#[test]
fn predecessor_retry_duplicates_and_structurally_different_forms_remain_visible() {
    run(r#"
 for index,f in ipairs(families)do
  local text=textFor(f,10)
  for _,previous in ipairs({'+69 to maximum Life','+40% to Cold Resistance','+11 to all Attributes'})do
   local item=probe('Iron Ring','',previous..'\n'..text..'\nwhile stationary')
   assert(#members(item,text)==1);initial(text,f,10,1);noCombined()
  end
  probe('Iron Ring','','unreviewed predecessor\n'..text);assert(hasCombined(text),'unknown predecessor may consume current text')
  local base=index==1 and 'Gold Ring'or 'Amethyst Ring'
  local item=probeRaw('Rarity: RARE\nDuplicate Probe\n'..base..'\nImplicits: 1\n'..text..'\n'..text)
  local copies=members(item,text);assert(#copies==2 and copies[1].list=='implicitModLines'and copies[2].list=='explicitModLines')
  initial(text,f,10,1,2);noCombined()
  local values=componentRecords(item,f);assert(#values==2 and values[1]==10 and values[2]==10)
  local empty=probe(base,'','+69 to maximum Life');assert(#componentRecords(empty,f)==0,'no implicit text auto-grant')
 end
 local mods,extra=modLib.parseMod('+17% to Cold and Chaos Resistances')
 assert(not extra and #mods==2 and mods[1].name=='ColdResist'and mods[2].name=='ChaosResist')
 local mods,extra=modLib.parseMod('+2% to maximum Chaos Resistance')
 assert(not extra and #mods==1 and mods[1].name=='ChaosResistMax')
 local mods,extra=modLib.parseMod('10% more Rarity of Items found')
 assert(not extra and #mods==1 and mods[1].name=='LootRarity'and mods[1].type=='MORE')
 local mods,extra=modLib.parseMod('10% increased Rarity of Items found while on Low Life')
 assert(not mods or extra or mods[1][1],'conditional form is not unconditional ordinary rarity')
 "#);
}

#[test]
fn all_twenty_rarity_and_seven_chaos_original_rows_keep_occurrence_identity() {
    let oracle = source();
    let parse: Function = oracle.lua.globals().get("probeRaw").unwrap();
    let check: Function = oracle.lua.load(r#"
      return function(item,family,text,amount,count,runeCount)
       local f=families[family];local matches=members(item,text)
       assert(#matches==count,'original physical occurrence count: '..text)
       local runeMembers=0
       for i,match in ipairs(matches)do
        for j=1,i-1 do assert(match.row~=matches[j].row,'duplicate occurrence lost row identity')end
        if match.list=='runeModLines'then
         runeMembers=runeMembers+1
         assert(match.row.rune and match.row.enchant and not match.row.extra,'rebuilt rune category')
         assertRecords(match.row.modList,f,amount)
        end
       end
       assert(runeMembers==runeCount,'exact final rune member count')
       initial(text,f,amount,1,count)
       local rebuildCalls=0
       for _,call in ipairs(trace.calls)do if call.rune_rebuild and call.text==text then
        rebuildCalls=rebuildCalls+1
        assert(not call.combined and not call.extra,'rune reconstruction is not a following-line retry')
        assertRecords(call.mods,f,amount)
       end end
       assert(rebuildCalls==runeCount,'exact rune reconstruction parse count')
       if runeCount>0 then
        assert(#trace.rune_rebuilds==1,'one source rune reconstruction')
        local before,after={},{}
        for _,row in ipairs(trace.rune_rebuilds[1].before)do if row.line==text then before[#before+1]=row end end
        for _,row in ipairs(trace.rune_rebuilds[1].after)do if row.line==text then after[#after+1]=row end end
        assert(#before==runeCount and #after==runeCount,'reconstruction replaces the supplied rune occurrence')
        for _,row in ipairs(after)do
         for _,prior in ipairs(before)do assert(row~=prior,'rune occurrence must be rebuilt')end
         local found=0;for _,match in ipairs(matches)do if match.row==row then found=found+1 end end
         assert(found==1,'one final member retains rebuilt identity')
        end
       end
      end
    "#).eval().unwrap();
    let mut counts = [0usize; 2];
    let mut plain = [0usize; 2];
    let mut originals = [[0usize; 2]; 5];
    let mut rune_rows = Vec::new();
    for original in 1..=5 {
        let xml = std::fs::read_to_string(runtime::repository().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{original:02}.xml"
        )))
        .unwrap();
        let document = roxmltree::Document::parse(&xml).unwrap();
        for node in document.descendants().filter(|node| {
            node.has_tag_name("Item")
                && node
                    .parent()
                    .is_some_and(|parent| parent.has_tag_name("Items"))
        }) {
            let raw = node.text().unwrap();
            let mut groups = BTreeMap::<(usize, String), (f64, usize, usize)>::new();
            for line in raw.lines().map(str::trim) {
                let mut semantic = line;
                let mut rune = false;
                while let Some(rest) = semantic.strip_prefix('{') {
                    let Some(end) = rest.find('}') else { break };
                    rune |= &rest[..end] == "rune";
                    semantic = &rest[end + 1..];
                }
                for (family, prefix, suffix) in [
                    (0, "", "% increased Rarity of Items found"),
                    (1, "+", "% to Chaos Resistance"),
                ] {
                    let Some(amount) = semantic
                        .strip_prefix(prefix)
                        .and_then(|s| s.strip_suffix(suffix))
                    else {
                        continue;
                    };
                    if amount.is_empty() || !amount.bytes().all(|b| b.is_ascii_digit()) {
                        continue;
                    }
                    counts[family] += 1;
                    originals[original - 1][family] += 1;
                    plain[family] += usize::from(line == semantic);
                    let entry = groups.entry((family, semantic.to_owned())).or_insert((
                        amount.parse().unwrap(),
                        0,
                        0,
                    ));
                    entry.1 += 1;
                    entry.2 += usize::from(rune);
                    if rune {
                        rune_rows.push((
                            original,
                            node.attribute("id").unwrap().to_owned(),
                            semantic.to_owned(),
                        ));
                    }
                }
            }
            if groups.is_empty() {
                continue;
            }
            let item: Table = parse.call(raw).unwrap();
            for ((family, text), (amount, count, rune_count)) in groups {
                check
                    .call::<()>((item.clone(), family + 1, text, amount, count, rune_count))
                    .unwrap();
            }
        }
    }
    assert_eq!(
        rune_rows,
        vec![(4, "21".to_owned(), "+11% to Chaos Resistance".to_owned())]
    );
    assert_eq!(counts, [20, 7]);
    assert_eq!(plain, [13, 6]);
    assert_eq!(originals, [[4, 0], [2, 4], [5, 0], [9, 2], [0, 1]]);
    println!(
        "rarity/chaos source rows: {}",
        serde_json::json!({"counts":counts,"plain":plain,
        "by_original":originals,"scope":"fresh raw source only; tags, overlays and native admission stay independent"})
    );
}
