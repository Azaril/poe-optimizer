# Twelve physical primary-input families

This publication applies the existing `../statset-primary.json` policy to the
checked legacy corruption-flag release. It promotes twelve existing physical Gem
definitions from Unmapped to explicitly partial input schemas and allocates 24
required raw parameter slots, `def.00000000000030b2` through
`def.00000000000030c9`. Each family gains an independent Boolean corruption flag
and Count corruption-level delta. Missing or malformed values remain Pending;
literal `nil` follows the reviewed field-specific codecs.

The catalog's declared additional-stat-set aliases are absent as standalone Skill
identities and are absent from the constructed effect list. The unchanged
`SinglePrimary` compiler therefore emits only the exact mapped primary Skill.
This does not create extra Skills, generated providers, action stat-set choices,
effective Gem inputs, or numerical rules. The complete pinned-source observation
is in `owned_active_gem_inputs`: 48 families, 58 potential effects and 1,392 input
cases per JIT mode, including these twelve families.

All 604 previously Known complete quality-kind inventories remain unchanged.
The twelve newly compiled inventories retain the compiler's explicit Partial
quality closure, together with Partial parameter, choice, skill and generated
membership declarations. Any later quality-only closure correction should be an
explicit source-backed revision. Other existing rule bodies, schema facts,
registry entries, normalization recipes, source pins and query files are preserved.

`evidence.json` binds the authoring scope to predecessor input
`e1128e349928a1a6ee57fc07189143c1c30da65535e84b00cef4b5f3cd91efb1`
and the exact tracked policy commitment. The earlier compiler-only probe was
made against a different schema and registry. The regression rejects that stale
schema binding without creating a destination; it does not reuse the probe's
allocated slots.

Reproduce the default finite catalog/policy test:

```powershell
cargo test --locked -p poe-optimizer-cli --test owned_statset_gem_cli
```

The full real publication test is explicitly ignored unless selected. It requires
the exact checked predecessor and a fresh output directory:

```powershell
$env:POE_OPTIMIZER_TEST_STATSET_GEM_PRIOR = 'runs/owned-legacy-gem-flags-01/package'
$env:POE_OPTIMIZER_TEST_STATSET_GEM_OUTPUT = 'runs/owned-statset-gem-inputs-01'
cargo test --locked -p poe-optimizer-cli --test owned_statset_gem_cli real_statset_primary_inputs_preserve_full_release_and_originals -- --ignored --exact --test-threads=1
```

The test freshly compiles the policy using the public Rust compiler and CLI,
compares their exact migration/receipts, and executes the existing checked schema
migration and normalization publication commands. Those compact intermediates are
not the final baseline. The final full endpoint retains all six prior provenance
entries and appends one explicit authoring commitment, then is published and
rebuilt through `assemble-owned-release`. A full-input restoration comparison
allows only the twelve schema promotions, 24 parameter slots, twelve normalization
recipes, exact dependent rebinding, and appended provenance.

The shared V1 test fixture in `tests/support/owned_release_fixture.rs` loads and
verifies the exact prior artifact inventory. Its extraction was independently
checked by rerunning the physical quality revision's real publication test.

Validation on 2026-10-01 UTC: new default test 1 passed / 1 explicitly ignored;
explicit real publication 1 passed in 51.01 seconds; existing quality default and
real regression also passed. Five fresh original drafts compare equal after
canonicalizing their independent host lineages and removing only the two newly
supplied parameter members. The affected occurrences are `[0, 23, 3, 1, 7]`,
34 total. Five independent missing/malformed/nil flag and delta probes pass.
All 110 query rows remain byte-identical, and all originals remain Pending.

The final `runs/owned-statset-gem-inputs-01/package` and `rebuilt` contain 18
byte-identical files totaling 58,322,937 bytes:

- Release input: `f303c57eff31c6d0a42cb8f00634fb5b80544ec2fff3a3f74c98fcc5fd3f8710`.
- Schema: `cc4ebdffaead1b2aa58802b3a5ade812ceb58d39d8134aaf5939e4a651faa054`.
- Registry: `f98c0b22c2d6f1bcf43a790937ac8398c9df1e822e10ce1ee0dc302d937f4437`.

The full draft issue totals remain `[318, 869, 317, 383, 874]`: the new raw values
do not close Partial memberships. `remaining-original-issues.json` records each
original's exact issue codes. Logs are `runs/statset-gem-cli-tests.log` and
`runs/statset-gem-real-publication-tests.log`; the publication directory contains
the validation and execution receipts. Complete original numerical coverage
remains **0/5**. The next work should resolve blockers in the selected original
builds, rather than infer readiness from raw-input breadth.
