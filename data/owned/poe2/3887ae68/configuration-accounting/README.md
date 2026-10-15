# Checked configuration observation accounting

This packet admits one source-only disposition: the finite numeric
`enemyDamageRollRange` **Placeholder**, when the existing checked Encounter
adapter proves `enemyIsBoss`, `presetBossSkills` and `enemySizePreset` absent
in that exact ConfigSet. The common Import accounting owner checks the actual
Scenario and its still-live configuration obligation before changing the leaf.
The numeric magnitude is validated, then discarded. It is neither an owned
assumption nor an evaluator default.

The pinned `presetBossSkills` callback reads the roll only on its non-`None`
branch. The roll control has no independent `apply` callback. Fresh ConfigSet
construction selects the first boss-list entry, `None`. These are reviewed
source semantics, independent of widget visibility or the example build's
selected skill. The policy is intentionally narrower than the source witness:
authored roll Inputs, explicit `None`, string Placeholders, malformed values,
duplicate frames and unproved Encounter branches receive no new authority.

`rules.json` contains the injected name, numeric grammar, absence guards and
source-only reason. It uses the existing percentage-point unit and allocates no
new definition, slot, program or native input. The authoring helper constructs
the complete policy with `configuration_disposition_dependencies` from the
actual checked release, including the reward adapter, and commits the separate
catalogue and constructor evidence. Only that compiled policy enters the
normalization artifact; the evidence and archived producer files remain offline.

The predecessor is current compiler-v4 input
`690169b8e0303ca02ae9a49139a8446c6b470675c28914e7f06e69e09b4d8a54`.
The release inverse permits only this optional policy, its normalization label,
the tree normalizer's changed commitment and one provenance entry. It changes
no schema, rules, mappings, rewards, query or evaluation coverage.

## Retained evidence

Both original JIT reports are retained byte-for-byte from
`runs/owned-damage-roll-source-02/`. Each is 339,448 bytes with SHA-256
`c20f9c8992fa062f59b42bcc1da77abc17581bf9e1a7133c6fef20251e27d2b6`.
They contain twelve cases and thirteen complete load attempts per JIT mode:
the original, changed raw and placeholder values, missing placeholder, explicit
`None`, repeat and warm-to-original controls, and two positive boss contrasts.
The no-boss cases preserve complete scalar and availability commitments in MAIN
and CALCS. Those modes intentionally retain different selected minions. The
positive Shaper Ball controls change cold damage from 4,127 to 6,190 and exercise
the guarded roll read. Neither observation admits native boss-preset mechanics.

`evidence.json` authenticates both reports and the retained producer snapshots.
The acquisition report itself authenticates the exact observer bytes. Other
producer snapshots are pinned as they stood at adoption; their hashes are not
claimed to be historical acquisition attestations. The ordinary proof reconstructs
every case's exact XML hash from the unchanged fixture and authenticates the
observer, source manifest and file pins. Source excerpts preserve their original
line contents and reviewed line ranges in `catalogue-evidence.json`; their exact
full source files and upstream license are retained under `evidence/pob/`, so
ordinary verification does not require a PoB checkout.
The three source archives are exact bytes read from the pinned Git revision.
They match the source manifest hashes. The source authenticator explicitly
normalizes CRLF to LF before checking those hashes (`poe-optimizer-pob/src/source.rs`);
the adoption checkout's raw CRLF hashes are recorded separately. This does not
claim that an acquisition VM read the same raw line-ending bytes as the archive.

No PoB execution is required for these tests or publication. The archive is
evidence, not a runtime implementation or compatibility path.

## Publication and remaining work

The ignored publication test requires
`POE_OPTIMIZER_TEST_CONFIGURATION_ACCOUNTING_PRIOR` to identify the checked
predecessor package and `POE_OPTIMIZER_TEST_CONFIGURATION_ACCOUNTING_OUTPUT` to
name a new directory. Run the `owned_configuration_accounting` target with
`--include-ignored --test-threads=1`. It rebuilds all 18 release artifacts,
reimports all five unchanged originals, authenticates their derived commitments,
and checks the exact draft and sidecar inverse. It also checks eight changed-input
controls without running an oracle.

Validated Original05 change: configuration-linked origins 36 → 35, Config-local
origins 21 → 20, and all 15 outside-Config origins unchanged. All 110 queries,
draft values, local IDs, selections, four selected input obligations and every
inventory completion state remain unchanged. The four other originals each lose
only this same individually proved leaf link. Fresh imports deliberately allocate
new lineages; the preservation comparison normalizes only lineage and separately
authenticates the resulting draft/sidecar commitments.

This is one checked leaf, not complete configuration accounting. Retiring the
shared configuration issue still requires all remaining Config and outside-Config
origins, plus their real destinations, to be accounted for. Assumptions and usage
remain Pending.

## Validated checkpoint

Published package: `runs/owned-configuration-accounting-publication-02/package`,
input `3ee48944d26b5351ecc267cc2e87637b68a8b6153b7ab2bbc5c906f11e4bd702`.
All 18 artifacts rebuild byte-for-byte. The only changed files are
`normalization.json`, `tree-normalization.json` and `release.json`.

| Original | All linked origins | Config-local | Outside Config | Selected input issues |
| --- | --- | --- | --- | --- |
| 01 | 52 → 51 | 29 → 28 | 23 unchanged | 106 unchanged |
| 02 | 87 → 86 | 28 → 27 | 59 unchanged | 117 unchanged |
| 03 | 43 → 42 | 22 → 21 | 21 unchanged | 109 unchanged |
| 04 | 38 → 37 | 22 → 21 | 16 unchanged | 123 unchanged |
| 05 | 36 → 35 | 21 → 20 | 15 unchanged | 4 unchanged |

Three ordinary tests passed, with the publication test intentionally ignored by
default. The explicit five-original publication and eight changed-input controls
passed in 27.07 seconds. Strict target Clippy passed. These calls used the
workspace's optimized test profile with debug and overflow checks enabled;
no oracle runtime was executed. Final logs are
`runs/owned-configuration-accounting-data-tests-02.log`,
`runs/owned-configuration-accounting-data-publication-02.log` and
`runs/owned-configuration-accounting-data-clippy-01.log`.
The exact census is retained in the publication's `validation.json`.
