// One-off preparation/verification only. Never writes a maintained path.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const zlib = require('node:zlib');
const assert = require('node:assert/strict');
const root = path.resolve(__dirname, '../..');
const sha = b => crypto.createHash('sha256').update(b).digest('hex');
const read = p => fs.readFileSync(path.join(root, p));
const json = p => JSON.parse(read(p));
const clone = x => structuredClone(x);
const pin = p => ({ path: p, bytes: read(p).length, sha256: sha(read(p)) });
const authenticated = (p, expected) => { const b = read(p); assert.equal(sha(b), expected, p); return JSON.parse(b); };
const base = 'data/owned/poe2/3887ae68/mana-supplier-sources/';
const baselinePath = 'runs/modifier-order-pin-baseline-01/manifest.json';
const baseline = authenticated(baselinePath, 'd768bb178029c028762ee9081b44608b5f75a108f5d73b4b8fc968cf8bc6250a');
const authoringPath = 'docs/evidence/modifier-order-fixture-migration/authoring-report.json';
const authoring = authenticated(authoringPath, '529e9b08c40cd7503b5f7ea219ec1252ff5a9f1e5dde44e8ccf0163db8b0d63c');
const postcheckPath = 'runs/modifier-order-fixture-pins-01/postcheck.json';
const postcheck = authenticated(postcheckPath, 'd090ed0f9cec7fb9fbff4347421f530bd6ad41cd0be7c1313af3442eb0644ea3');
const minimalPath = 'runs/modifier-order-minimal-wire-successor-01/report.json';
const minimal = authenticated(minimalPath, '75020aa66270cea8378efa47a7bf3e15ffe0b8ec0bc972dccef9b343766aba53');
const oldRawHash = '42fe06d045eb06938b41991c4dde47aa435119dff3e3f95e0056f8fd0a3c515a';
const newRawHash = '6f86a5bcbde72f45b528cf309602536b96ee6045049380721623ad5bce0e1958';
const target = 'tests/support/owned_mana_configuration_sources.rs';
const originalPath = p => 'runs/modifier-order-pin-baseline-01/originals/' + p;
const one = (xs, predicate) => { const ys = xs.filter(predicate); assert.equal(ys.length, 1); return ys[0]; };
const files = ['configuration-reward-authority.json', 'configuration-reward-draft.gz', 'configuration-reward-sidecar.gz', 'configuration-reward-report.gz'];
for (const name of files) {
  const p = base + name;
  const before = one(baseline.originals, x => x.path === p);
  const after = one(postcheck.final_files, x => x.path === p);
  const emitted = one(authoring.files, x => x.path.replaceAll('\\', '/') === p);
  assert.equal(sha(read(originalPath(p))), before.sha256);
  assert.equal(read(originalPath(p)).length, before.bytes);
  assert.equal(sha(read(p)), after.sha256);
  assert.equal(read(p).length, after.bytes);
  assert.equal(emitted.old_sha256, before.sha256);
  assert.equal(emitted.new_sha256, after.sha256);
}
const oldAuthority = json(originalPath(base + files[0]));
const authority = json(base + files[0]);
const proof = authority.modifier_order_reauthoring;
assert.equal(proof.authoring_sha256, postcheck.authorer_sha256);
assert.equal(sha(read('docs/evidence/modifier-order-fixture-migration/authoring.rs')), proof.authoring_sha256);
assert.equal(proof.proof, 'whole-draft-sidecar-report-exact-inverse');
for (const field of ['source_release_artifacts_reauthored', 'source_selections_changed', 'native_coverage_authorized']) assert.equal(proof[field], false);
const authorityInverse = clone(authority);
delete authorityInverse.modifier_order_reauthoring;
authorityInverse.files = clone(oldAuthority.files);
assert.deepEqual(authorityInverse, oldAuthority);
const archives = (metadata, original) => ['draft', 'sidecar', 'report'].map(role => {
  const p = base + metadata.files[role].file;
  const compressed = read(original ? originalPath(p) : p);
  const raw = zlib.gunzipSync(compressed);
  const expected = metadata.files[role];
  assert.equal(sha(compressed), expected.compressed_sha256);
  assert.equal(compressed.length, expected.compressed_bytes);
  assert.equal(sha(raw), expected.sha256);
  assert.equal(raw.length, expected.bytes);
  return { raw, value: JSON.parse(raw) };
});
const old = archives(oldAuthority, true);
const current = archives(authority, false);
assert.equal(sha(old[2].raw), oldRawHash);
assert.equal(sha(current[2].raw), newRawHash);
const [oldDraft, oldSidecar, oldReport] = old.map(x => x.value);
const [draft, sidecar, report] = current.map(x => x.value);
assert.deepEqual(report.draft_digest, sidecar.draft);
assert.deepEqual(report.allocator_after, draft.draft.allocator);
assert.deepEqual(report.allocator_after, sidecar.allocator_after);
assert.equal(report.sidecar_sha256, sha(current[1].raw));
assert.equal(report.sidecar_bytes, current[1].raw.length);
assert.equal(report.issue_count, oldReport.issue_count + 20);
assert.equal(BigInt('0x' + report.allocator_after.last_issued), BigInt('0x' + oldReport.allocator_after.last_issued) + 20n);
const oldCode = 'item-modifier-order-not-converted';
const contributionCode = 'item-modifier-contribution-order-not-converted';
const numericCode = 'item-modifier-numeric-transform-order-not-converted';
const codes = clone(oldReport.issue_codes);
assert.equal(codes[oldCode], 20);
delete codes[oldCode]; codes[contributionCode] = 20; codes[numericCode] = 20;
assert.deepEqual(report.issue_codes, codes);
const draftInverse = clone(draft);
draftInverse.draft.allocator = clone(oldDraft.draft.allocator);
assert.equal(draft.draft.items.members.length, 28);
assert.equal(proof.independent_known_order_proofs.length, 8);
assert.equal(proof.new_pending_numeric_transform_orders.length, 20);
assert.equal(proof.exact_item_origin_indices.length, 20);
let pending = 0, known = 0;
const sidecarInverse = clone(sidecar);
draftInverse.draft.items.members.forEach((item, i) => {
  const priorItem = oldDraft.draft.items.members[i];
  assert.deepEqual(item.id, priorItem.id);
  const orders = item.modifier_orders;
  const contribution = clone(orders.contributions);
  if (contribution.kind === 'pending') {
    assert.equal(contribution.code, contributionCode);
    contribution.code = oldCode;
    const issue = proof.new_pending_numeric_transform_orders[pending];
    assert.deepEqual(orders.numeric_transforms, { kind: 'pending', id: issue, code: numericCode, candidates: [] });
    const index = proof.exact_item_origin_indices[pending];
    const origin = sidecarInverse.origins[index];
    assert.deepEqual(one(origin.links, x => x.kind === 'item').value, item.id);
    assert.deepEqual(origin.links.pop(), { kind: 'issue', value: issue });
    pending++;
  } else {
    assert.equal(contribution.kind, 'known');
    const witness = one(proof.independent_known_order_proofs, x => JSON.stringify(x.item) === JSON.stringify(item.id));
    assert.deepEqual(orders.contributions, { kind: 'known', value: witness.contributions });
    assert.deepEqual(orders.numeric_transforms, { kind: 'known', value: witness.numeric_transforms });
    known++;
  }
  assert.deepEqual(contribution, priorItem.modifier_order);
  delete item.modifier_orders; item.modifier_order = contribution;
});
assert.equal(pending, 20); assert.equal(known, 8);
assert.deepEqual(draftInverse, oldDraft);
for (const field of ['draft', 'allocator_after']) sidecarInverse[field] = clone(oldSidecar[field]);
assert.deepEqual(sidecarInverse, oldSidecar);
const reportFields = ['draft_digest', 'allocator_after', 'sidecar_sha256', 'sidecar_bytes', 'issue_count', 'issue_codes'];
const reportInverse = clone(report);
for (const field of reportFields) reportInverse[field] = clone(oldReport[field]);
assert.deepEqual(reportInverse, oldReport);
const unchangedAnchors = [['source', '4fa239bb7f72b936b522a84fb6adb0139cdddb2a4b593ce69a64bb722f7a2e9d'], ['release', 'ea5e38460e1dcb8df912a4dcddfd31d3e0be89dca2f5ede339b1a7173a1f34f2']];
for (const [role, hash] of unchangedAnchors) {
  assert.deepEqual(authority.files[role], oldAuthority.files[role]);
  const compressed = read(base + authority.files[role].file);
  assert.equal(sha(compressed), authority.files[role].compressed_sha256);
  assert.equal(sha(zlib.gunzipSync(compressed)), hash);
}
const before = read(target);
const needle = `const REWARD_REPORT: &str = "${oldRawHash}";`;
assert.equal(before.toString().split(needle).length, 2);
const after = Buffer.from(before.toString().replace(needle, `const REWARD_REPORT: &str = "${newRawHash}";`));
assert.deepEqual(Buffer.from(after.toString().replace(newRawHash, oldRawHash)), before);
assert(!minimal.files.some(x => x.path === target));
assert(!minimal.literal_checks.some(x => x.path === target));
assert(files.every(name => !minimal.files.some(x => x.path === base + name)));
const reportOut = {
  kind: 'supplemental-modifier-order-report-anchor-repair', status: 'prepared-not-rust-executed',
  script_sha256: sha(fs.readFileSync(__filename)), target,
  before: { sha256: sha(before), bytes: before.length }, after: { sha256: sha(after), bytes: after.length },
  old_raw_report_sha256: oldRawHash, new_raw_report_sha256: newRawHash,
  report_fields_changed: reportFields, old_issue_count: oldReport.issue_count, new_issue_count: report.issue_count,
  source_release_anchors_unchanged: true, full_draft_sidecar_report_inverse: true, exact_source_inverse: true,
  known_items: known, pending_items: pending, numerical_or_coverage_changes: false,
  authority: [pin(baselinePath), pin(authoringPath), pin(postcheckPath), pin(minimalPath), ...files.map(x => pin(base + x))],
  minimal_successor_overlap: { changed_files: [], authenticated_literal_files: [] },
  callers: { maintained_target: 'owned_mana_configuration_sources', ordinary_tests: 10, ignored_tests: ['retain_reviewed_configuration_bodies'], other_includers: [] },
  required_validation: ['cargo test -p poe-optimizer-cli --no-default-features --test owned_mana_configuration_sources', 'cargo clippy -p poe-optimizer-cli --no-default-features --test owned_mana_configuration_sources -- -D warnings']
};
const output = path.join(__dirname, 'files', target);
if (process.argv.includes('--verify')) {
  assert.deepEqual(fs.readFileSync(output), after);
  assert.deepEqual(JSON.parse(fs.readFileSync(path.join(__dirname, 'report.json'))), reportOut);
  console.log('PASS: exact single-anchor replacement, independent anchors, full fixture inverses, no minimal-successor overlap');
} else {
  assert(!fs.existsSync(output));
  fs.mkdirSync(path.dirname(output), { recursive: true });
  fs.writeFileSync(output, after, { flag: 'wx' });
  fs.writeFileSync(path.join(__dirname, 'before.rs'), before, { flag: 'wx' });
  fs.writeFileSync(path.join(__dirname, 'report.json'), JSON.stringify(reportOut, null, 2) + '\n', { flag: 'wx' });
  console.log(JSON.stringify({ before: reportOut.before, after: reportOut.after, report_sha256: sha(fs.readFileSync(path.join(__dirname, 'report.json'))) }));
}
