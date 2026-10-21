// One-off fixture provenance repair. No production rules, source bodies, release
// identities, closure labels, or compatibility code are authored by this file.
const fs=require('fs'),path=require('path'),crypto=require('crypto'),assert=require('assert/strict');
const base='runs/modifier-order-pin-baseline-01',manifest=JSON.parse(fs.readFileSync(base+'/manifest.json'));
const reportPath=process.argv[2],output=process.argv[3];
assert(reportPath&&output,'usage: node script <actual-Rust-authorer/report.json> <new-runs-output>');
assert(path.resolve(output).startsWith(path.resolve('runs')+path.sep));assert(!fs.existsSync(output));
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const report=JSON.parse(fs.readFileSync(reportPath));
const seedReportHashes=new Map(report.files.map(r=>[r.path.replaceAll('\\','/'),r.new_sha256]));
const originals=new Map(manifest.originals.map(r=>[r.path,r]));
const seed=new Set(manifest.seeds),edgesByFile=new Map();
assert.deepEqual(new Set(report.files.map(r=>r.path.replaceAll('\\','/'))),seed);
for(const r of report.files){const p=r.path.replaceAll('\\','/');assert.equal(r.old_sha256,originals.get(p).sha256);assert.equal(r.new_sha256,hash(fs.readFileSync(p)));}
for(const edge of manifest.edges){assert(!edge.file.includes('incoming-critical'));const list=edgesByFile.get(edge.file)||[];list.push(edge);edgesByFile.set(edge.file,list);}
// Preserve literal numerical spelling, formatting and every unaffected byte.
function leaves(s){const out=new Map();let i=0;const ws=()=>{while(/\s/.test(s[i]||'\0'))i++};function str(){const a=i++;while(i<s.length){if(s[i]==='\\'){i+=2;continue;}if(s[i++]==='"')return [a,i,JSON.parse(s.slice(a,i))];}throw Error('unterminated JSON string');}
 function value(ps){ws();const c=s[i];if(c==='{'){i++;ws();if(s[i]==='}'){i++;return;}for(;;){ws();const k=str()[2];ws();assert.equal(s[i++],':');value([...ps,k]);ws();const n=s[i++];if(n==='}')return;assert.equal(n,',');}}
 else if(c==='['){i++;ws();if(s[i]===']'){i++;return;}for(let n=0;;n++){value([...ps,n]);ws();const end=s[i++];if(end===']')return;assert.equal(end,',');}}
 else if(c==='"'){const [a,b,v]=str();out.set(JSON.stringify(ps),{a,b,v});}
 else{const a=i,m=/^(?:true|false|null|-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)/.exec(s.slice(i));assert(m);i+=m[0].length;out.set(JSON.stringify(ps),{a,b:i,v:JSON.parse(m[0])});}}
 value([]);ws();assert.equal(i,s.length);return out;
}
function patch(s,changes){const tokens=leaves(s),ranges=changes.map(c=>{const t=tokens.get(JSON.stringify(c.key));assert(t,JSON.stringify(c.key));assert.deepEqual(t.v,c.before);return {...t,text:JSON.stringify(c.after)};}).sort((a,b)=>b.a-a.a);for(const r of ranges)s=s.slice(0,r.a)+r.text+s.slice(r.b);JSON.parse(s);return s;}
const computed=new Map(),visiting=new Set(),changesByFile=new Map(),beforeByFile=new Map();
function compute(p){if(computed.has(p))return computed.get(p);assert(!visiting.has(p),'cyclic fixture pins');visiting.add(p);const row=originals.get(p);assert(row);const before=fs.readFileSync(p);assert.equal(hash(before),seed.has(p)?seedReportHashes.get(p):row.sha256,'unreviewed concurrent fixture edit: '+p);const changes=[];const tokens=leaves(p.endsWith('.json')?before.toString('utf8'):'null');
 for(const e of edgesByFile.get(p)||[]){const target=compute(e.target),prior=originals.get(e.target);const t=tokens.get(JSON.stringify(e.key)),after=hash(target);assert.notEqual(after,prior.sha256,'each declared dependency must actually change');const key=[...e.key.slice(0,-1),'bytes'],bytes=e.key.at(-1)==='sha256'?tokens.get(JSON.stringify(key)):undefined;if(seed.has(p)&&t.v===after){if(bytes)assert.equal(bytes.v,target.length);continue;}assert.equal(t.v,prior.sha256);changes.push({key:e.key,before:t.v,after,target:e.target});if(bytes){assert.equal(bytes.v,prior.bytes);changes.push({key,before:bytes.v,after:target.length,target:e.target});}}
 const next=changes.length?Buffer.from(patch(before.toString('utf8'),changes)):before;
 if(changes.length){const inverse=patch(next.toString('utf8'),changes.map(c=>({...c,before:c.after,after:c.before})));assert(Buffer.from(inverse).equals(before),'whole byte inverse');changesByFile.set(p,changes);beforeByFile.set(p,before);}
 computed.set(p,next);visiting.delete(p);return next;
}
for(const p of originals.keys())compute(p);
const codeChanges=[];
for(const row of manifest.code){assert(!row.path.includes('incoming_critical'));const before=fs.readFileSync(row.path);assert.equal(hash(before),row.sha256,'unreviewed concurrent code edit: '+row.path);let text=before.toString('utf8');const edits=[];for(const hit of row.hits){assert.equal(text.split(hit.old_sha256).length-1,hit.count);const after=hash(computed.get(hit.target));assert.notEqual(after,hit.old_sha256);text=text.replaceAll(hit.old_sha256,after);edits.push({...hit,new_sha256:after});}let inverse=text;for(const e of edits)inverse=inverse.replaceAll(e.new_sha256,e.old_sha256);assert(Buffer.from(inverse).equals(before));codeChanges.push({path:row.path,before,after:Buffer.from(text),edits});}
fs.mkdirSync(output,{recursive:true});const written=[];
function write(p,before,after,edits){const backup=output+'/originals/'+p;fs.mkdirSync(path.dirname(backup),{recursive:true});fs.writeFileSync(backup,before);assert(fs.readFileSync(p).equals(before));fs.writeFileSync(p,after);written.push({path:p,old_sha256:hash(before),new_sha256:hash(after),old_bytes:before.length,new_bytes:after.length,edits});}
for(const [p,edits]of changesByFile)write(p,beforeByFile.get(p),computed.get(p),edits);
for(const c of codeChanges)write(c.path,c.before,c.after,c.edits);
const summary={kind:'modifier-order-current-fixture-provenance-pins',baseline_manifest_sha256:hash(fs.readFileSync(base+'/manifest.json')),authorer_report_sha256:hash(fs.readFileSync(reportPath)),script_sha256:hash(fs.readFileSync(__filename)),whole_file_byte_inverses:true,source_bodies_changed:false,numerical_programs_changed:false,coverage_claims_changed:false,files:written};
fs.writeFileSync(output+'/report.json',JSON.stringify(summary,null,2)+'\n');console.log(JSON.stringify({output,artifact_pin_files:changesByFile.size,literal_pin_files:codeChanges.length,total_files:written.length},null,2));
