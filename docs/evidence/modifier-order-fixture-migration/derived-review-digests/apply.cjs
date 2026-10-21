'use strict';
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto'), assert = require('node:assert/strict');
const candidate = 'runs/self-flat-derived-digest-repair-candidate-20261021';
const output = 'runs/owned-modifier-orders-self-flat-repair-applied-01.json';
assert(!fs.existsSync(output));
const sha = b => crypto.createHash('sha256').update(b).digest('hex');
const manifestBytes = fs.readFileSync(candidate + '/manifest.json');
assert.equal(sha(manifestBytes), '9efc4d936dfff1d7b1c4d825c2b1806cd36603d41fab3edeec2444384087d224');
const manifest = JSON.parse(manifestBytes);
const inventoryBytes = fs.readFileSync(candidate + '/expected-inventory.json');
assert.equal(sha(inventoryBytes), '6498f091b496e82a3bf087b86673839b3bb072d609337b3108574265097ec88a');
const inventory = JSON.parse(inventoryBytes);
assert.equal(inventory.candidate_manifest_sha256, sha(manifestBytes));
assert.equal(manifest.files.length, 30);
assert.equal(manifest.files.reduce((n, f) => n + f.changes.length, 0), 69);
const staged = new Map();
const at = (v, keys) => keys.reduce((x, k) => x[k], v);
for (const row of manifest.files) {
  assert(/^(data\/owned\/poe2\/3887ae68\/|tests\/support\/owned_)/.test(row.path));
  assert(!/incoming[-_]critical|(^|\/)\.\.($|\/)/.test(row.path));
  const before = fs.readFileSync(row.path), next = fs.readFileSync(path.join(candidate, 'files', row.path));
  assert.equal(sha(before), row.before_sha256, row.path);
  assert.equal(sha(next), row.after_sha256, row.path);
  assert.equal(before.length, row.bytes); assert.equal(next.length, row.bytes);
  assert(before.equals(fs.readFileSync(path.join(candidate, 'before', row.path))));
  const replacements = new Map();
  for (const change of row.changes) {
    assert(/^[a-f0-9]{64}$/.test(change.before) && /^[a-f0-9]{64}$/.test(change.after));
    const entry = replacements.get(change.before) || { after: change.after, count: 0 };
    assert.equal(entry.after, change.after); entry.count++; replacements.set(change.before, entry);
    if (change.key) { assert.equal(at(JSON.parse(before), change.key), change.before); assert.equal(at(JSON.parse(next), change.key), change.after); }
  }
  let rebuilt = before.toString(), inverse = next.toString();
  for (const [old, replacement] of replacements) {
    assert.equal(rebuilt.split(old).length - 1, replacement.count);
    rebuilt = rebuilt.replaceAll(old, replacement.after);
    inverse = inverse.replaceAll(replacement.after, old);
  }
  assert(Buffer.from(rebuilt).equals(next)); assert(Buffer.from(inverse).equals(before));
  staged.set(row.path, { before, next });
}
const intended = p => staged.get(p)?.next || fs.readFileSync(p);
assert.equal(inventory.final_files.length, 45);
for (const row of inventory.final_files) { const b = intended(row.path); assert.equal(sha(b), row.sha256); assert.equal(b.length, row.bytes); }
for (const row of inventory.literal_checks) {
  const b = intended(row.path); assert.equal(sha(b), row.sha256);
  for (const check of row.checks) { assert.equal(b.toString().split(check.sha256).length - 1, check.count); assert.equal(sha(intended(check.target)), check.sha256); }
}
for (const row of manifest.files) for (const c of row.changes) if (c.target) assert.equal(sha(intended(c.target)), c.after);
for (const [file, { before, next }] of staged) { assert(fs.readFileSync(file).equals(before)); fs.writeFileSync(file, next); assert(fs.readFileSync(file).equals(next)); }
for (const row of inventory.final_files) assert.equal(sha(fs.readFileSync(row.path)), row.sha256);
const postcheck = { ...inventory, kind: 'applied-derived-modifier-order-fixture-digests', status: 'installed-runtime-validation-pending', application_script_sha256: sha(fs.readFileSync(__filename)), changed_files: manifest.files, expected_inventory_sha256: sha(inventoryBytes), candidate_manifest_sha256: sha(manifestBytes), numerical_programs_changed: false, native_coverage_changed: false };
fs.writeFileSync(output, JSON.stringify(postcheck, null, 2) + '\n', { flag: 'wx' });
console.log(JSON.stringify({ output, sha256: sha(fs.readFileSync(output)), changed_files: staged.size, checked_artifacts: inventory.final_files.length, checked_literal_files: inventory.literal_checks.length }));
