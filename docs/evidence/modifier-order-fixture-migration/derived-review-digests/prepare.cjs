const fs=require('fs'),path=require('path'),zlib=require('zlib'),crypto=require('crypto'),assert=require('assert/strict');
const base='data/owned/poe2/3887ae68/',original='runs/modifier-order-pin-baseline-01/originals/';
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
class WireNumber{constructor(raw){this.rawJSON=raw;}}
function parse(b){const s=b.toString('utf8');let i=0;const ws=()=>{while(/\s/.test(s[i]||'\0'))i++};function str(){const a=i++;while(i<s.length){if(s[i]==='\\'){i+=2;continue;}if(s[i++]==='"')return JSON.parse(s.slice(a,i));}throw Error('string');}
 function val(){ws();const c=s[i];if(c==='{'){const o={};i++;ws();if(s[i]==='}'){i++;return o;}for(;;){ws();const k=str();ws();assert.equal(s[i++],':');o[k]=val();ws();const end=s[i++];if(end==='}')return o;assert.equal(end,',');}}
 if(c==='['){const a=[];i++;ws();if(s[i]===']'){i++;return a;}for(;;){a.push(val());ws();const end=s[i++];if(end===']')return a;assert.equal(end,',');}}
 if(c==='"')return str();const m=/^(?:true|false|null|-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)/.exec(s.slice(i));assert(m);i+=m[0].length;return /^[-0-9]/.test(m[0])?new WireNumber(m[0]):JSON.parse(m[0]);}const v=val();ws();assert.equal(i,s.length);return v;}
const canon=v=>(v instanceof WireNumber)?v.rawJSON:Array.isArray(v)?'['+v.map(canon).join(',')+']':v&&typeof v==='object'?'{'+Object.keys(v).sort().map(k=>JSON.stringify(k)+':'+canon(v[k])).join(',')+'}':JSON.stringify(v);
const digest=v=>sha(canon(v));
function bytes(p,old){return fs.readFileSync(old&&fs.existsSync(original+p)?original+p:p);}
function json(p,old){return parse(bytes(p,old));}
function gz(f,n,old){return parse(zlib.gunzipSync(bytes(base+f+'/'+n+'.gz',old)));}
function relineage(v,b,a){if(Array.isArray(v)){for(const x of v)relineage(x,b,a);}else if(v&&typeof v==='object'&&!(v instanceof WireNumber)){if('lineage'in v){assert.equal(v.lineage,b);v.lineage=a;}for(const x of Object.values(v))relineage(x,b,a);}}
function selected(old){
 const b=json(base+'self-flat-selected-bindings/bindings.json',old);
 const oldLineage=gz('self-flat-selected-bindings','draft.json',old).draft.allocator.lineage;
 const draft=gz('mana-supplier-sources','draft.json',old),side=gz('mana-supplier-sources','sidecar.json',old),selection=gz('mana-supplier-sources','selection.json',old),release=gz('mana-supplier-sources','release.json',old);
 relineage(b,oldLineage,draft.draft.allocator.lineage);
 const memberships=Object.fromEntries(['equipment','passives','authored_skills','support_assignments','generated_inputs','observed_player_effects','observed_supplied_actors','counts'].map(k=>[k,b[k]]));
 return {selection,publication:release.input,memberships,draft_digest:side.draft,source_origin_count:side.origins.length,configuration_change:{ordinal:455,scope:'one-placeholder-only'},descendant_delivery_certified:false};
}
function fixture(old){
 const authority=json(base+'self-flat-skill-construction/authority.json',old),sources=authority.files.map(p=>json(p.path,old));
 const gates=sources[2].raw_runs.filter(p=>p.path.includes('observed')).map(p=>json(base+'self-flat-runtime-gates/'+p.path,old).additional_observation);
 assert.equal(gates.length,2);
 return {authority,recipes:sources[0],selected:selected(old),off:gz('mana-supplier-sources','source-jit-off.json',old).state,on:gz('mana-supplier-sources','source-jit-on.json',old).state,gates};
}
const old=fixture(true),current=fixture(false);
const previousReview=json(base+'self-flat-skill-construction/review.json',true),previousChild=json(base+'self-flat-skill-construction/child-review.json',true);
const actual={old_selected:digest(old.selected),old_acquisition:digest(old),old_recipes:digest(old.recipes),new_selected:digest(current.selected),new_acquisition:digest(current),new_recipes:digest(current.recipes)};
console.log(JSON.stringify({actual,expected:{selected:previousReview.selected_source_membership_sha256,acquisition:previousReview.acquisition_and_correspondence_sha256,recipes:previousReview.recipes_sha256}},null,2));
assert.equal(actual.old_selected,previousReview.selected_source_membership_sha256,'canonical predecessor selected');
assert.equal(actual.old_acquisition,previousReview.acquisition_and_correspondence_sha256,'canonical predecessor acquisition');
assert.equal(actual.old_acquisition,previousChild.acquisition_sha256);
assert.equal(actual.old_recipes,previousReview.recipes_sha256);
assert.equal(actual.new_recipes,actual.old_recipes);
fs.writeFileSync(path.join(__dirname,'derived.json'),JSON.stringify(actual,null,2)+'\n');

function diffs(a,b,p=[],out=[]){if(canon(a)===canon(b))return out;if(a&&b&&typeof a==='object'&&typeof b==='object'&&!(a instanceof WireNumber)&&!(b instanceof WireNumber)){assert.equal(Array.isArray(a),Array.isArray(b));for(const k of [...new Set([...Object.keys(a),...Object.keys(b)])]){const key=[...p,Array.isArray(a)?Number(k):k];if(!(k in a)||!(k in b))out.push({key,before:a[k],after:b[k]});else diffs(a[k],b[k],key,out);}}else out.push({key:p,before:a,after:b});return out;}
const selectedChanges=diffs(old.selected,current.selected);
// Only exact order-field replacements and the canonical draft digest distinguish
// the selected input. Prove the full inverse instead of trusting matching counts.
const inverse=parse(canon(current.selected));
assert.equal(inverse.memberships.equipment.length,9);
for(let i=0;i<9;i++){
 const a=old.selected.memberships.equipment[i].owned_item,b=inverse.memberships.equipment[i].owned_item;
 assert.equal(canon(a.id),canon(b.id));
 assert.equal(canon(a.modifier_order),canon(b.modifier_orders.contributions));
 assert.equal(b.modifier_orders.numeric_transforms.kind,'known');
 delete b.modifier_orders;b.modifier_order=a.modifier_order;
}
inverse.draft_digest=old.selected.draft_digest;
assert.equal(canon(inverse),canon(old.selected),'whole selected correspondence inverse');
for(const k of ['recipes','off','on','gates'])assert.equal(canon(old[k]),canon(current[k]),'unchanged source input '+k);
assert.equal(diffs(old.authority,current.authority).length,2);
const baseline=JSON.parse(fs.readFileSync('docs/evidence/modifier-order-fixture-migration/minimal-wire-successor/postcheck.json'));
for(const p of baseline.final_files)assert.equal(sha(fs.readFileSync(p.path)),p.sha256||p.final_sha256||p.successor_sha256,'immediate predecessor '+p.path);
const childProcess=require('child_process');
const rels=childProcess.execFileSync('rg',['--files','data/owned/poe2/3887ae68','tests','-g','*.json','-g','*.rs'],{encoding:'utf8',maxBuffer:8*1024*1024}).trim().split(/\r?\n/).map(p=>p.replaceAll('\\','/')).filter(p=>!p.includes('incoming-critical')&&!p.includes('incoming_critical'));
const beforeByPath=new Map(),seedByPath=new Map(),changesByPath=new Map(),afterByPath=new Map();
function before(p){if(!beforeByPath.has(p))beforeByPath.set(p,fs.readFileSync(p));return beforeByPath.get(p);}
function leaves(v,at=[],out=[]){if(v&&typeof v==='object'){for(const k of Object.keys(v))leaves(v[k],[...at,Array.isArray(v)?Number(k):k],out);}else out.push({key:at,value:v});return out;}
const seeds=[
 ['self-flat-skill-construction/review.json',[['acquisition_and_correspondence_sha256'],['selected_source_membership_sha256']]],
 ['self-flat-skill-construction/child-review.json',[['acquisition_sha256'],['construction','selected_bindings_sha256']]],
 ['self-flat-minion-store/review.json',[['inputs_sha256']]],
 ['self-flat-later-helpers/review.json',[['inputs_sha256'],['construction','child_lifecycle','selected_bindings_sha256']]],
 ['self-flat-flask-keystone/review.json',[['selected_bindings_sha256']]],
 ['self-flat-action-factories/review.json',[['inputs_sha256']]]
];
for(const [rel,keys]of seeds){const p=base+rel,ls=leaves(JSON.parse(before(p)));const changes=[];for(const key of keys){const l=ls.find(x=>JSON.stringify(x.key)===JSON.stringify(key));assert(l,'seed leaf '+p+' '+key);const after=l.value===actual.old_acquisition?actual.new_acquisition:l.value===actual.old_selected?actual.new_selected:undefined;assert(after,'only authenticated derived digest');changes.push({key,before:l.value,after,kind:'recomputed-derived-digest'});}seedByPath.set(p,changes);}
function apply(p,changes){let out=before(p).toString('utf8');const byValue=new Map();for(const c of changes){const list=byValue.get(c.before)||[];list.push(c);byValue.set(c.before,list);}for(const [b,list]of byValue){assert(list.every(x=>x.after===list[0].after));const matches=out.split(b).length-1;assert.equal(matches,list.length,'exact replacement count '+p+' '+b);out=out.replaceAll(b,list[0].after);}let inverse=out;for(const [b,list]of byValue)inverse=inverse.replaceAll(list[0].after,b);assert(Buffer.from(inverse).equals(before(p)),'whole byte inverse '+p);assert.equal(Buffer.byteLength(out),before(p).length);if(p.endsWith('.json'))JSON.parse(out);return Buffer.from(out);}
for(const [p,c]of seedByPath){changesByPath.set(p,c);afterByPath.set(p,apply(p,c));}
for(let round=0;round<30;round++){
 const map=new Map([...afterByPath].map(([p,b])=>[sha(before(p)),{path:p,before:sha(before(p)),after:sha(b)}]));
 let progress=false;
 for(const p of rels){const raw=before(p).toString('utf8');const hits=[...map].filter(([h])=>raw.includes(h));if(!hits.length&&!seedByPath.has(p))continue;const changes=[...(seedByPath.get(p)||[])];
  if(p.endsWith('.json')){for(const l of leaves(JSON.parse(raw))){const t=map.get(l.value);if(t){assert.notEqual(t.path,p,'cycle');changes.push({key:l.key,before:t.before,after:t.after,target:t.path,kind:'file-sha256'});}}}
  else for(const [h,t]of hits){const count=raw.split(h).length-1;for(let i=0;i<count;i++)changes.push({before:h,after:t.after,target:t.path,kind:'literal-file-sha256',occurrence:i});}
  const after=apply(p,changes);if(!afterByPath.has(p)||!afterByPath.get(p).equals(after)){afterByPath.set(p,after);changesByPath.set(p,changes);progress=true;}
 }
 if(!progress)break;assert(round<29,'dependency cycle or nonconvergence');
}
// Every found dependency must still authenticate its exact current target.
for(const [p,cs]of changesByPath)for(const c of cs)if(c.target){assert.equal(c.before,sha(before(c.target)));assert.equal(c.after,sha(afterByPath.get(c.target)));}
const out=path.join(__dirname,'files'),back=path.join(__dirname,'before');
assert(!fs.existsSync(out)&&!fs.existsSync(back),'candidate already frozen');
const files=[];
for(const [p,b]of [...afterByPath].sort(([a],[b])=>a.localeCompare(b))){for(const [dir,bytes]of [[out,b],[back,before(p)]]){const dst=path.join(dir,p);fs.mkdirSync(path.dirname(dst),{recursive:true});fs.writeFileSync(dst,bytes);}files.push({path:p,before_sha256:sha(before(p)),after_sha256:sha(b),bytes:b.length,changes:changesByPath.get(p),whole_byte_inverse:true});}
const report={status:'prepared-unapplied',immediate_predecessor_postcheck_sha256:sha(fs.readFileSync('docs/evidence/modifier-order-fixture-migration/minimal-wire-successor/postcheck.json')),derived:actual,authenticated_predecessor_derived_digests:true,selected_order_fields:9,selected_physical_items:8,whole_selected_inverse:true,source_observations_unchanged:true,recipes_unchanged:true,gates_unchanged:true,files,tracked_files_changed:false,cargo_executed:false,runtime_validation:'pending'};
fs.writeFileSync(path.join(__dirname,'manifest.json'),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({files:files.map(x=>({path:x.path,changes:x.changes.length})),manifest_sha256:sha(fs.readFileSync(path.join(__dirname,'manifest.json')))},null,2));
