# Reverse-primary census checkpoint

October 21, 2026. **Local checkpoint validation passed.** The
[implementation plan](../../implementation.md) remains the current resume point.
The [validation receipt](validation.json) records the successful final gates and
exact executed logs, separately from the retained initial failures.

The existing Import-only role artifact now requires `reverse_primary`. An
unavailable observation differs from an observed route containing zero, one or
multiple primary Gem candidates for an exact source/owned Skill. Multiple
candidates remain ambiguous. Runtime loading checks bounded local consistency;
offline publication reproduces the complete census from the authenticated
catalog. No native schema, calculation rule, Lua behavior or compatibility
reader is added. This contract change retires no selected build issue.

## Retained evidence

- [Cutover proof](cutover-proof.json): the completed Rust authorer derives 966
  role rows and 1,436 routes from the pinned catalog. Its candidate input is
  `a75ef471b57d8184b208ca5f2a26ce6a9a01d15340db8b15fc1f318be40c1905`,
  from predecessor
  `348f0c7588e915f7b7ea6c2de9b1d2ce5089ebdb4393f5822ed818537a4c7560`.
  Native recipe and 110 queries are preserved. The separate
  [all-five CLI gate](publication-validation.json) also passes.
- [Receipt rebind report](receipt-rebind-report.json): the second completed
  authoring test prepares two dependent receipt hashes, with exact whole-byte
  inverses and immutable-input checks. It changes no source-mechanic claim.
- [Baseline receipt](baseline-receipt.json): identifies the old binary and
  predecessor used to capture actual selected-build reports. The full reports
  and generated packages are not duplicated here; exact output hashes are
  retained in the maintained publication test.
- [Source review](source-review/README.md) and its [manifest](source-review/manifest.json):
  immutable pre-execution review of the source component target. Saved
  `LoadSkill` and generated construction use table-keyed reverse lookup;
  bare `ProcessSocketGroup` uses a string key and can take direct fallback.
  These observations supply source evidence, not Import or native authority.
- [Rebuild audit](rebuild-audit.md) and [next-adapter guidance](next-adapter-guidance.md):
  historical scope and next-step notes. Exact live provider/source/slot/level
  checks and independent field consumers remain required. Both Djinn effects
  must survive; Firebolt's singleton does not establish broader coverage.

[Archive metadata](archive-manifest.json) records source paths, sizes and
SHA-256 hashes. Copies were checked against the original bytes without
formatting. Per-folder attributes preserve those bytes in Git.
The earlier archive and source manifests retain their status at capture; the
final validation receipt above is the execution record.

## Validated result

All 615 checks pass: 558 Import tests, four optional PoB source-component tests,
52 ordinary CLI checks and the explicit all-five preservation gate. Both Rust
authoring executions also pass. Scoped strict Clippy passes for Import, PoB and
CLI. The ordinary batches report one ignored Import test and three ignored CLI
tests; the applicable all-five test is executed separately. These local results
do not assert hosted CI success.

All five unchanged original builds retain their complete draft values, sidecar
evidence apart from three authenticated dependency commitments, allocators,
selections and complete selected diagnostics. Counts remain 123/137/127/144/5.
Fresh replays agree. All eighteen package files rebuild byte-for-byte, with only
roles, normalization, tree-normalization and release changed from the prior
package. Exactly twenty dependent digest leaves change; scalar-input commitments,
native rules and all 110 query rows remain unchanged. No selected issue retires,
and no complete native build is claimed.

The two maintained seeds and two dependent receipt files were installed from the
Rust outputs with before/after hash guards; [installation metadata](seed-installation.json)
records the six exact files. Temporary authoring registrations are removed.

## Completed authoring and retained failure

The [first authoring log](logs/reverse-primary-authoring-20261021-01.log)
records one passing ignored library test. The
[receipt log](logs/reverse-primary-receipts-20261021-01.log) records one passing
temporary integration test. These executions create reviewed candidates; they
do not execute a native build calculation.

The [initial catalog log](logs/reverse-primary-import-catalog-20261021-01.log)
records 34 passes and one failure: Serde accepted unknown fields on the internally
tagged unit `Unavailable` variant. The correction uses `Unavailable {}` while
preserving the intended `{"kind":"unavailable"}` wire representation and the
same strict rejection test. The failed execution is retained as a failure;
successor validation belongs in the final checkpoint receipt.

The [first source run](logs/reverse-primary-source-20261021-01.log) records
three passes and one failure before the affected dispatch assertions: the
separately extracted `round` function lacked Common.lua's local `m_floor` alias.
The [host repair](source-host-repair/manifest.json) restores that exact original
binding only around the unchanged source span and removes an unused Rust import.
All four test bodies and source-span hashes remain unchanged. Its
[prior source](source-host-repair/before.rs) and
[copy metadata](source-host-repair/archive-manifest.json) are retained separately;
the initial source review manifest is unchanged. The repaired runtime run passes
all four tests; its exact log is recorded in the final validation receipt.

## Historical one-off invocation

The exact [cutover authorer](authoring/owned_reverse_primary_cutover.rs),
[seed rebuilder](authoring/owned_reverse_primary_seed_rebuild.rs),
[receipt helper](authoring/owned_reverse_primary_receipt_rebind.rs) and
[temporary receipt wrapper](authoring/reverse_primary_receipt_authoring_temp.rs)
are historical executed sources, not maintained runtime paths. The cutover
authorer was temporarily included as the `reverse_primary_cutover` Import
library test module so it could call the existing private checked rebinder.
Its original invocation selected
`reverse_primary_cutover::author_reverse_primary_cutover` with `--ignored --exact`
and `POE_OPTIMIZER_REVERSE_PRIMARY_OUTPUT` pointing to a fresh runs directory.
The receipt wrapper likewise ran only its explicit ignored authoring test.

Exact replay requires the original temporary module invocation, original relative
`runs/` locations and authenticated pre-migration seed/receipt bytes. The
predecessor package and proof inputs are pinned in the sources. Do not compile
these archived copies as a new runtime tool, rerun them against already updated
data, or relax their guards to make a different predecessor pass. No large
package or raw build report is copied into this archive.
