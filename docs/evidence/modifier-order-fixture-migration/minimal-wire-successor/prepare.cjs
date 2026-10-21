// Prepare only. Never write a maintained artifact or execute Cargo/Git.
const fs=require('fs'),path=require('path'),crypto=require('crypto'),assert=require('assert/strict');
const OUT='runs/modifier-order-minimal-wire-successor-01';assert(!fs.existsSync(OUT));
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const baselinePath='runs/modifier-order-pin-baseline-01/manifest.json';
const currentPath='runs/modifier-order-fixture-pins-01/postcheck.json';
assert.equal(hash(fs.readFileSync(baselinePath)),'d768bb178029c028762ee9081b44608b5f75a108f5d73b4b8fc968cf8bc6250a');
assert.equal(hash(fs.readFileSync(currentPath)),'d090ed0f9cec7fb9fbff4347421f530bd6ad41cd0be7c1313af3442eb0644ea3');
const baseline=JSON.parse(fs.readFileSync(baselinePath)),postcheck=JSON.parse(fs.readFileSync(currentPath));
const prior=new Map(postcheck.final_files.map(r=>[r.path,r]));
const original=new Map(baseline.originals.map(r=>[r.path,r]));
const sourceRoot='runs/modifier-order-pin-baseline-01/originals/';
const seeds=[
 {file:'data/owned/poe2/3887ae68/item-evasion-property/dependencies.json',prefix:['items'],count:4},
 {file:'data/owned/poe2/3887ae68/player-evasion-final/dependencies.json',prefix:['selected_snapshot','items','members'],count:28},
];
const key=p=>JSON.stringify(p);
function at(v,p){for(const k of p)v=v[k];return v;}
function tokens(s){const fields=new Map(),scalars=new Map();let i=0;const ws=()=>{while(/\s/.test(s[i]||'\0'))i++};function str(){const a=i++;while(i<s.length){if(s[i]==='\\'){i+=2;continue;}if(s[i++]==='"')return {a,b:i,v:JSON.parse(s.slice(a,i))};}throw Error('unterminated string');}
 function value(ps){ws();const a=i,c=s[i];if(c==='{'){i++;ws();const names=new Set();if(s[i]==='}'){i++;return {a,b:i};}for(;;){ws();assert.equal(s[i],'"');const k=str();assert(!names.has(k.v),'duplicate JSON object key');names.add(k.v);ws();assert.equal(s[i++],':');ws();const child=[...ps,k.v],v=value(child);fields.set(key(child),{ka:k.a,kb:k.b,a:v.a,b:v.b});ws();const n=s[i++];if(n==='}')return {a,b:i};assert.equal(n,',');}}
 else if(c==='['){i++;ws();if(s[i]===']'){i++;return {a,b:i};}for(let n=0;;n++){value([...ps,n]);ws();const end=s[i++];if(end===']')return {a,b:i};assert.equal(end,',');}}
 else if(c==='"'){const t=str();scalars.set(key(ps),t);return {a,b:i};}
 else{const m=/^(?:true|false|null|-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)/.exec(s.slice(i));assert(m);i+=m[0].length;scalars.set(key(ps),{a,b:i,v:JSON.parse(m[0])});return {a,b:i};}}
 value([]);ws();assert.equal(i,s.length);return {fields,scalars};
}
function splice(s,patches){const sorted=[...patches].sort((a,b)=>b.a-a.a);let end=s.length;for(const p of sorted){assert(p.b<=end,'patches must not overlap');assert.equal(s.slice(p.a,p.b),p.before);s=s.slice(0,p.a)+p.after+s.slice(p.b);end=p.a;}return s;}
const current=new Map();for(const r of prior.values()){const b=fs.readFileSync(r.path);assert.equal(hash(b),r.sha256,'current reviewed artifact changed: '+r.path);assert.equal(b.length,r.bytes);current.set(r.path,b);}
const staged=new Map(),seedProofs=[];
for(const spec of seeds){const oldBytes=fs.readFileSync(sourceRoot+spec.file);assert.equal(hash(oldBytes),original.get(spec.file).sha256);assert.equal(oldBytes.length,original.get(spec.file).bytes);const old=oldBytes.toString('utf8'),now=current.get(spec.file),oldValue=JSON.parse(old),nowValue=JSON.parse(now);const ot=tokens(old),items=at(oldValue,spec.prefix);assert.equal(items.length,spec.count);const patches=[],proof=[];
 for(let n=0;n<items.length;n++){const oldPath=[...spec.prefix,n,'modifier_order'],newPath=[...spec.prefix,n,'modifier_orders'];const span=ot.fields.get(key(oldPath));assert(span);assert.equal(at(nowValue,[...spec.prefix,n]).id.local,items[n].id.local);const replacement=at(nowValue,newPath);assert(replacement);assert(!Object.hasOwn(at(nowValue,[...spec.prefix,n]),'modifier_order'));assert(!Object.hasOwn(items[n],'modifier_orders'));
  const lineStart=old.lastIndexOf('\n',span.ka-1)+1,indent=old.slice(lineStart,span.ka);assert(/^[ \t]*$/.test(indent));const newline=old.includes('\r\n')?'\r\n':'\n';if(newline==='\r\n')assert.equal((old.match(/\r\n/g)||[]).length,(old.match(/\n/g)||[]).length);
  const rendered=JSON.stringify(replacement,null,2).split('\n').join(newline+indent);const before=old.slice(span.ka,span.b),after=JSON.stringify('modifier_orders')+old.slice(span.kb,span.a)+rendered;
  patches.push({a:span.ka,b:span.b,before,after});proof.push({item:items[n].id,old_path:oldPath,new_path:newPath,original_field_sha256:hash(Buffer.from(before)),successor_field_sha256:hash(Buffer.from(after)),before_order:items[n].modifier_order,after_orders:replacement});
 }
 const next=splice(old,patches);assert.deepEqual(JSON.parse(next),nowValue,'whole semantic equality to current reviewed artifact');const nt=tokens(next),inverse=[];for(let n=0;n<proof.length;n++){const span=nt.fields.get(key(proof[n].new_path));assert(span);inverse.push({a:span.ka,b:span.b,before:next.slice(span.ka,span.b),after:patches[n].before});}assert.equal(splice(next,inverse),old,'whole original-byte inverse');
 // The raw complement is preserved by construction: each patch is exactly one
 // original order field, with no formatting pass over its enclosing document.
 staged.set(spec.file,Buffer.from(next));seedProofs.push({path:spec.file,original_sha256:hash(oldBytes),current_sha256:hash(now),successor_sha256:hash(Buffer.from(next)),original_bytes:oldBytes.length,current_bytes:now.length,successor_bytes:Buffer.byteLength(next),field_replacements:proof,whole_current_semantic_equality:true,whole_original_byte_inverse:true,unrelated_bytes_preserved:true});
}
const edgesByFile=new Map();for(const e of baseline.edges){const list=edgesByFile.get(e.file)||[];list.push(e);edgesByFile.set(e.file,list);}const visiting=new Set(),computed=new Set(),pinEdits=[];
function compute(p){if(computed.has(p))return staged.get(p)||current.get(p);assert(!visiting.has(p),'cyclic pin dependency');visiting.add(p);const before=staged.get(p)||current.get(p),s=before.toString('utf8'),ts=p.endsWith('.json')?tokens(s):null,patches=[],edits=[];
 for(const e of edgesByFile.get(p)||[]){const target=compute(e.target),old=prior.get(e.target);const node=ts.scalars.get(key(e.key));assert.equal(node.v,old.sha256,'authenticated current pin');const nextHash=hash(target);if(nextHash===old.sha256)continue;patches.push({a:node.a,b:node.b,before:s.slice(node.a,node.b),after:JSON.stringify(nextHash)});edits.push({key:e.key,before:node.v,after:nextHash,target:e.target});if(e.key.at(-1)==='sha256'){const byteKey=[...e.key.slice(0,-1),'bytes'],b=ts.scalars.get(key(byteKey));if(b){assert.equal(b.v,old.bytes);patches.push({a:b.a,b:b.b,before:s.slice(b.a,b.b),after:JSON.stringify(target.length)});edits.push({key:byteKey,before:b.v,after:target.length,target:e.target});}}}
 if(patches.length){const next=splice(s,patches),nt=tokens(next),inverse=edits.map(e=>{const t=nt.scalars.get(key(e.key));return {a:t.a,b:t.b,before:next.slice(t.a,t.b),after:JSON.stringify(e.before)}});assert.equal(splice(next,inverse),s,'whole pin-file byte inverse');staged.set(p,Buffer.from(next));pinEdits.push({path:p,edits,whole_current_byte_inverse:true});}visiting.delete(p);computed.add(p);return staged.get(p)||current.get(p);
}
for(const p of prior.keys())compute(p);
// Check all40 final dependency edges, including unchanged branches.
for(const e of baseline.edges)assert.equal(at(JSON.parse(staged.get(e.file)||current.get(e.file)),e.key),hash(staged.get(e.target)||current.get(e.target)));
const literalChecks=[];for(const row of baseline.code){const b=fs.readFileSync(row.path),s=b.toString('utf8');const checks=[];for(const hit of row.hits){const currentHash=prior.get(hit.target).sha256;assert.equal(s.split(currentHash).length-1,hit.count);const successorHash=hash(staged.get(hit.target)||current.get(hit.target));assert.equal(successorHash,currentHash,'unexpected literal pin dependency requires explicit staging');checks.push({target:hit.target,sha256:currentHash,count:hit.count});}literalChecks.push({path:row.path,sha256:hash(b),checks});}
fs.mkdirSync(OUT,{recursive:true});const files=[];for(const [p,b]of staged){assert(!p.includes('incoming-critical'));assert(fs.readFileSync(p).equals(current.get(p)),'maintained artifacts changed while staging');const dest=OUT+'/files/'+p,old=OUT+'/current/'+p;fs.mkdirSync(path.dirname(dest),{recursive:true});fs.mkdirSync(path.dirname(old),{recursive:true});fs.writeFileSync(dest,b);fs.writeFileSync(old,current.get(p));files.push({path:p,current_sha256:hash(current.get(p)),successor_sha256:hash(b),current_bytes:current.get(p).length,successor_bytes:b.length});}
const report={kind:'minimal-byte-modifier-order-wire-successor',mode:'prepared-not-applied',script_sha256:hash(fs.readFileSync(__filename)),predecessor_postcheck_sha256:hash(fs.readFileSync(currentPath)),baseline_manifest_sha256:hash(fs.readFileSync(baselinePath)),seeds:seedProofs,pin_edits:pinEdits,all_dependency_edges_checked:baseline.edges.length,literal_checks:literalChecks,literal_pin_edits:0,source_bodies_changed:false,numerical_programs_changed:false,native_coverage_changed:false,files};fs.writeFileSync(OUT+'/report.json',JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({output:OUT,seed_files:seedProofs.length,order_field_replacements:seedProofs.reduce((n,p)=>n+p.field_replacements.length,0),staged_files:files.length,pin_files:pinEdits.length,validated_edges:baseline.edges.length,literal_pin_edits:0,report_sha256:hash(fs.readFileSync(OUT+'/report.json')),files},null,2));
