// Explicitly invoked only after the running runtime batch has completed.
const fs=require('fs'),path=require('path'),crypto=require('crypto'),assert=require('assert/strict');
const dir='runs/modifier-order-minimal-wire-successor-01',out='runs/modifier-order-minimal-wire-applied-01';
assert(!fs.existsSync(out));
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const reportBytes=fs.readFileSync(dir+'/report.json');
assert.equal(hash(reportBytes),'75020aa66270cea8378efa47a7bf3e15ffe0b8ec0bc972dccef9b343766aba53');
const report=JSON.parse(reportBytes);
assert.equal(hash(fs.readFileSync('runs/prepare_modifier_order_minimal_wire_successor.cjs')),report.script_sha256);
assert.equal(report.mode,'prepared-not-applied');assert.equal(report.files.length,7);assert.equal(report.seeds.length,2);assert.equal(report.pin_edits.length,5);assert.equal(report.literal_pin_edits,0);
const baselineBytes=fs.readFileSync('runs/modifier-order-pin-baseline-01/manifest.json');assert.equal(hash(baselineBytes),report.baseline_manifest_sha256);const baseline=JSON.parse(baselineBytes);
const priorPost=fs.readFileSync('runs/modifier-order-fixture-pins-01/postcheck.json');assert.equal(hash(priorPost),report.predecessor_postcheck_sha256);const post=JSON.parse(priorPost);
const staged=new Map();for(const row of report.files){assert(row.path.startsWith('data/owned/poe2/3887ae68/'));assert(!row.path.includes('incoming-critical'));const current=fs.readFileSync(row.path),next=fs.readFileSync(dir+'/files/'+row.path);assert.equal(hash(current),row.current_sha256);assert.equal(current.length,row.current_bytes);assert.equal(hash(next),row.successor_sha256);assert.equal(next.length,row.successor_bytes);assert(current.equals(fs.readFileSync(dir+'/current/'+row.path)));staged.set(row.path,{current,next});}
for(const row of post.final_files){const live=fs.readFileSync(row.path);assert.equal(hash(live),row.sha256,'unreviewed current artifact edit');assert.equal(live.length,row.bytes);}
for(const row of report.literal_checks)assert.equal(hash(fs.readFileSync(row.path)),row.sha256,'unreviewed literal pin edit');
for(const seed of report.seeds)assert.deepEqual(JSON.parse(staged.get(seed.path).next),JSON.parse(staged.get(seed.path).current),'seed semantics stay identical');
function at(v,keys){for(const k of keys)v=v[k];return v;}
for(const e of baseline.edges){const source=JSON.parse(staged.get(e.file)?.next||fs.readFileSync(e.file)),target=staged.get(e.target)?.next||fs.readFileSync(e.target);assert.equal(at(source,e.key),hash(target));const bytesKey=[...e.key.slice(0,-1),'bytes'];if(e.key.at(-1)==='sha256'&&at(source,bytesKey)!==undefined)assert.equal(at(source,bytesKey),target.length);}
fs.mkdirSync(out,{recursive:true});for(const [p,row]of staged){const backup=out+'/predecessor/'+p;fs.mkdirSync(path.dirname(backup),{recursive:true});fs.writeFileSync(backup,row.current);assert(fs.readFileSync(p).equals(row.current),'concurrent edit before installation');fs.writeFileSync(p,row.next);assert(fs.readFileSync(p).equals(row.next));}
const finalFiles=post.final_files.map(row=>{const b=fs.readFileSync(row.path),intended=staged.get(row.path)?.next;assert.equal(hash(b),intended?hash(intended):row.sha256,'unreviewed final artifact edit');assert.equal(b.length,intended?intended.length:row.bytes);return {path:row.path,bytes:b.length,sha256:hash(b)}});
for(const e of baseline.edges)assert.equal(at(JSON.parse(fs.readFileSync(e.file)),e.key),hash(fs.readFileSync(e.target)));
for(const row of report.literal_checks){const text=fs.readFileSync(row.path,'utf8');assert.equal(hash(Buffer.from(text)),row.sha256);for(const check of row.checks)assert.equal(text.split(check.sha256).length-1,check.count);}
const final={kind:'applied-minimal-byte-modifier-order-wire-successor',candidate_report_sha256:hash(reportBytes),predecessor_postcheck_sha256:hash(priorPost),apply_script_sha256:hash(fs.readFileSync(__filename)),seed_files:2,order_fields:32,dependent_pin_files:5,rust_files_changed:0,all_dependency_edges_checked:40,source_bodies_changed:false,numerical_programs_changed:false,native_coverage_changed:false,installed:report.files,final_files:finalFiles,literal_checks:report.literal_checks};
fs.writeFileSync(out+'/report.json',JSON.stringify(final,null,2)+'\n');console.log(JSON.stringify({output:out,installed:report.files.length,seed_files:2,pin_files:5,checked_edges:40,report_sha256:hash(fs.readFileSync(out+'/report.json'))},null,2));
