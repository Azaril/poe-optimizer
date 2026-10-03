# Saved source settings and owned action selections

`resolve-owned-action` converts one caller-selected physical source occurrence
and one reference context into an existing `ImportQueryTarget`. It runs native
Rust in Import. It does not load Lua or execute PoB, materialize a build, select
which build to evaluate, activate an ability, or certify numerical coverage.

## Boundary

The injected `PobPhysicalPrimaryStatSetsV1` correspondence binds to the exact
owned definitions, role package, source pins and catalog. Construction checks the
physical Gem's primary Skill, supply, entering grant, player output and declared
part/mode/stat-set membership. Source strings and table ordinals exist only in
Import. The Core action retains owned IDs and an occurrence-specific provider.

The request contains an `ImportSkillUseLocator` (source hash, occurrence ordinal,
expected physical Gem) and `main` or `calcs`. Both contexts are independent PoB
reference settings, not native game state. The adapter accepts repeated or
archived physical occurrences without claiming they are active.

Matching `StatSetIndex` or `StatSetCalcsIndex` children are decoded with the shared
typed value recipe. A reviewed `absent_stat_set` is the only source absence
mapping. Missing indices on present rows, malformed values, duplicates, unknown
indices, unsupported child grammar, namespaces and stale source locators remain
unresolved. The pinned loader overwrites legacy scalar headers; the report
retains their attribute origins without using them as fallback selections.

Correspondence construction and resolution have bounded bytes, work, map rows,
stat sets, value decoding and output. The immutable adapter is reusable. Reports
retain the exact request, source snapshot, correspondence identity and selected
attribute, or a specific Pending reason. The CLI only supplies bounded I/O and
validated package loading.

## CLI and existing normalization

```powershell
poe-optimizer resolve-owned-action BUILD.xml --release RELEASE_DIRECTORY `
  --correspondence ADAPTER.json --request REQUEST.json
```

The command prints a report with `source_execution`, `calculation` and
`whole_build_parity` explicitly false. An application can put `report.target`
into an explicit query template and use `normalize-owned`. That normalizer must
still find the actual materialized SkillUse and physical Gem, preserve their
source links, and retain every unresolved inventory. This command does not
silently rewrite query files or the selected build.

Current physical Gem materialization permits child maps, while the shared
physical-input census requires flat Gem rows. A diagnostic query can therefore
resolve while other source-inventory proofs remain Pending. Resolving that
inventory boundary needs an explicit reviewed contract; action selection alone
must not broaden it.

## Ice Nova correspondence

The [reviewed packet](../data/owned/poe2/3887ae68/ice-nova-actions/README.md)
supplies one player action with two constructed stat sets, Ice Nova and
Cold-Infused. Metadata aliases are neither additional Skills nor extra stat
sets. All four physical occurrences in Original05 resolve independently; the
originals' 110 requested metrics remain unchanged. Ice Nova is exercised through
separate diagnostic queries.

The packet adds topology only. Final level/quality, damage tables, conditional
modifiers, complete physical/usage inventories and execution readiness remain
incomplete. Source assembly adds repeated stat names, so future damage data must
be validated after the source aggregation rather than copied by last-field
replacement. The existing native routing contract can distinguish these exact
owned stat sets; it needs no source ordinal or PoB reference context.

The native binding tests distinguish an invalid alternative on an offered output
from an unavailable output on a particular provider. Neither selects a fallback.
Finite topology tests exercise duplicate copies, disabled providers, query-free
binding, scratch reuse and independent Rayon workers. They do not assert complete
numerical evaluation of the real packet.

## Checkpoint reproduction

Default Rust targets are `poe-optimizer-import --test owned_source_actions`,
`poe-optimizer-engine --test owned_ice_nova_actions`, and
`poe-optimizer-cli --test owned_ice_nova_actions_cli`. The real CLI publication
case is ignored by default and requires the exact predecessor plus the two
source reports authenticated by `authoring.json`.

```powershell
$env:POE_OPTIMIZER_TEST_ICE_NOVA_PRIOR = 'C:\code\poe-optimizer\runs\owned-sniper-reservation-01\package'
$env:POE_OPTIMIZER_TEST_ICE_NOVA_OUTPUT = 'C:\code\poe-optimizer\runs\owned-ice-nova-actions-repeat'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_OPT_LEVEL = '2'
$env:CARGO_PROFILE_TEST_DEBUG_ASSERTIONS = 'true'
$env:CARGO_PROFILE_TEST_OVERFLOW_CHECKS = 'true'
$env:CARGO_INCREMENTAL = '0'
cargo test -p poe-optimizer-cli --test owned_ice_nova_actions_cli --locked -- --include-ignored
```

Choose an output directory that does not exist. The test writes the exact bound
adapter, package, rebuilt package, original/diagnostic normalized drafts and
`validation.json`. It verifies predecessor preservation, eighteen identical
rebuilt artifacts, all five original selections and eight mutation controls.
It does not change source builds or assert numerical parity.
